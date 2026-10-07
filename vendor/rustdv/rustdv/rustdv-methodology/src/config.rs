//! The ConfigDb — path-addressed configuration (D11–D16, D65–D68).
//!
//! A component is configured by code that never touches it, through a path
//! named as a string. That is the capability the first rustdv pass removed
//! when it replaced this with typed structs passed to constructors, and
//! restoring it is most of the point of this branch.
//!
//! **Shape of the API (D66).** `get` and `set` take a *context*, an
//! *offset*, and a *field*:
//!
//! ```ignore
//! ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
//! ConfigDb::set(None,      "*",        "MSG", String::from("GLOBAL"));
//!
//! let msg: String = ConfigDb::get(Some(ctx), "",     "MSG")?;  // me
//! let msg: String = ConfigDb::get(Some(ctx), "loga", "MSG")?;  // a child
//! let seqr: Rc<Sequencer> = ConfigDb::get(None, "", "SEQR")?;  // no context
//! ```
//!
//! The offset is *relative to the context*, so a lookup is
//! position-independent — an absolute path would be the hand-typed string
//! D7 forbids. `None` means no context: the offset is absolute. That form
//! is not an edge case; a `Sequence` is not a component and has no path, so
//! it is the only way a sequence can read at all.
//!
//! **Namespacing (D65).** One namespace per (path, field). The type is
//! *not* part of the key, which is where SystemVerilog differs: it passes
//! `uvm_resource#(T)::get_type()` into the lookup, so a `set` and a `get`
//! that disagree on type simply never meet and you get a silent `return 0`.
//! Here the entry is found and the type checked, so a mismatch is its own
//! error. The cost, stated rather than hidden: two different types can no
//! longer share a field name at one path.
//!
//! **Storage (D67).** Values must be `Clone`. Store a plain value and every
//! reader gets a copy; wrap it in an `Rc` and every reader shares one
//! object.
//!
//! **Debugging is built in, not bolted on (D68).** [`ConfigDb::print`] dumps
//! every entry *with its precedences*, because a resolved value tells you
//! who won but not who was competing; [`ConfigDb::set_tracing`] logs every
//! operation with the context, the offset, and the path they resolved to.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use crate::component::RustdvCtx;

/// Build-phase writes get `DEFAULT_PRECEDENCE - depth`, so the shallowest
/// setter wins regardless of write order (D13). Writes after build use the
/// full default and outrank every build-time write.
const DEFAULT_PRECEDENCE: i32 = 1000;

// ===========================================================================
// Errors (D14)
// ===========================================================================

/// Why a `get` failed. SystemVerilog collapses all of these into `return 0`
/// and leaves your variable untouched; naming them is the point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// No entry matched the resolved path and field. Covers "never set",
    /// "path does not match" and "field misspelled" — indistinguishable to
    /// any database addressed by strings, which is why ch28 exists.
    NotFound { path: String, field: String },
    /// An entry was found, but it holds another type. Only reportable
    /// because the type is not part of the key (D65).
    TypeMismatch {
        path: String,
        field: String,
        stored: &'static str,
        requested: &'static str,
    },
}

impl ConfigError {
    /// Stable, machine-readable cause for `#[rustdv::test(expect_error=…)]`.
    pub fn kind(&self) -> &'static str {
        match self {
            ConfigError::NotFound { .. } => "config_not_found",
            ConfigError::TypeMismatch { .. } => "config_type_mismatch",
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound { path, field } => write!(
                f,
                "ConfigDb: no value for \"{field}\" at \"{path}\" \
                 (never set, path does not match, or the key is misspelled)"
            ),
            ConfigError::TypeMismatch {
                path,
                field,
                stored,
                requested,
            } => write!(
                f,
                "ConfigDb: \"{field}\" at \"{path}\" holds a {stored}, but a {requested} was requested"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

// ===========================================================================
// The store
// ===========================================================================

struct Entry {
    value: Rc<dyn Any>,
    type_name: &'static str,
    type_id: TypeId,
    /// The value rendered at `set` time. Stored because the dump and the
    /// tracer must show *what* was configured, not merely its type — the
    /// parent/child conflict is diagnosed by reading "PARENT RULES!" beside
    /// "CHILD RULES!". Python gets this free from `repr()`; in Rust it costs
    /// a `Debug` bound on `set`, which is the price of a database you can
    /// actually debug (D68).
    rendered: String,
}

impl Clone for Entry {
    fn clone(&self) -> Entry {
        Entry {
            value: self.value.clone(),
            type_name: self.type_name,
            type_id: self.type_id,
            rendered: self.rendered.clone(),
        }
    }
}

/// path glob -> field -> precedence -> entry. The precedence map is kept
/// (rather than collapsing to a winner) so [`ConfigDb::print`] can show a
/// losing entry beside the one that beat it — which is how you debug a
/// parent/child conflict (D68).
type Store = BTreeMap<String, BTreeMap<String, BTreeMap<i32, Entry>>>;

thread_local! {
    static STORE: RefCell<Store> = const {RefCell::new(Store::new())};
    static TRACING: Cell<bool> = const { Cell::new(false) };
    /// True while the build phase is walking, so writes take depth-scaled
    /// precedence (D13, tier 2).
    static IN_BUILD: Cell<bool> = const { Cell::new(false) };
}

/// Does a concrete path match a stored glob? Only `*` is supported, which
/// is what both source books use.
fn glob_match(path: &str, pattern: &str) -> bool {
    fn inner(p: &[u8], g: &[u8]) -> bool {
        match (p.first(), g.first()) {
            (_, Some(b'*')) => inner(p, &g[1..]) || (!p.is_empty() && inner(&p[1..], g)),
            (Some(a), Some(b)) if a == b => inner(&p[1..], &g[1..]),
            (None, None) => true,
            _ => false,
        }
    }
    inner(path.as_bytes(), pattern.as_bytes())
}

/// Is `a` at least as specific as `b`? `a.b.c` before `a.b.*` before `*`
/// (D13, tier 1 — pyuvm's rule, which SystemVerilog does not have).
fn more_specific(a: &str, b: &str) -> bool {
    glob_match(a, b)
}

/// Resolve context + offset into the path this operation addresses.
fn resolve(ctx: Option<&RustdvCtx>, offset: &str) -> String {
    match ctx {
        None => offset.to_string(),
        Some(c) if offset.is_empty() => c.path().to_string(),
        Some(c) if c.path().is_empty() => offset.to_string(),
        Some(c) => format!("{}.{}", c.path(), offset),
    }
}

fn trace(op: &str, ctx: Option<&RustdvCtx>, offset: &str, path: &str, field: &str, value: &str) {
    if TRACING.with(|t| t.get()) {
        let context = ctx.map(|c| c.path()).unwrap_or("<none>");
        rustdv_sim::log::info(&format!(
            "CFGDB/{op} context={context} offset=\"{offset}\" -> {path} {field}={value}"
        ));
    }
}

/// The configuration database. All methods are associated functions over an
/// ambient per-test store (D11, D16).
pub struct ConfigDb;

impl ConfigDb {
    /// Store `value` for every component whose path matches `offset`
    /// (a glob), resolved against `ctx`.
    pub fn set<T: Clone + fmt::Debug + 'static>(
        ctx: Option<&RustdvCtx>,
        offset: &str,
        field: &str,
        value: T,
    ) {
        let path = resolve(ctx, offset);
        // Precedence is scaled by the depth of the **setter**, not of the
        // path it wrote to (pyuvm: `default_precedence - context.get_depth()`).
        // That distinction is the whole of D13 tier 2: in a parent/child
        // conflict both writers resolve to the *same* path, so scaling by the
        // target would tie and let recency decide — and since build is
        // top-down, the child always writes last and would always win.
        let setter_depth = ctx.map(|c| depth_of(c.path())).unwrap_or(0);
        let precedence = if IN_BUILD.with(|b| b.get()) {
            DEFAULT_PRECEDENCE - setter_depth
        } else {
            DEFAULT_PRECEDENCE
        };
        let entry = Entry {
            rendered: format!("{value:?}"),
            value: Rc::new(value),
            type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        };
        trace("SET", ctx, offset, &path, field, &entry.rendered);
        STORE.with(|s| {
            s.borrow_mut()
                .entry(path)
                .or_default()
                .entry(field.to_string())
                .or_default()
                .insert(precedence, entry);
        });
    }

    /// Read `field` for the component at `ctx` + `offset`.
    ///
    /// The offset must be concrete — globs are legal only when storing
    /// (D12, pyuvm's rule). Resolution is most-specific path first, then
    /// highest precedence, then most recent write (D13).
    #[must_use = "a ConfigDb miss is a real failure; SystemVerilog's silent \
                  zero is what this Result exists to prevent"]
    pub fn get<T: Clone + 'static>(
        ctx: Option<&RustdvCtx>,
        offset: &str,
        field: &str,
    ) -> Result<T, ConfigError> {
        let path = resolve(ctx, offset);

        let found = STORE.with(|s| {
            let store = s.borrow();
            let mut matches: Vec<(&String, &Entry)> = store
                .iter()
                .filter(|(pattern, _)| glob_match(&path, pattern))
                .filter_map(|(pattern, fields)| {
                    fields
                        .get(field)
                        .and_then(|by_prec| by_prec.iter().next_back())
                        .map(|(_, entry)| (pattern, entry))
                })
                .collect();
            // Most specific first; ties keep insertion order.
            matches
                .sort_by(|(a, _), (b, _)| more_specific(a, b).cmp(&more_specific(b, a)).reverse());
            matches.first().map(|(_, e)| (*e).clone())
        });

        let Some(entry) = found else {
            trace("GET", ctx, offset, &path, field, "<not found>");
            return Err(ConfigError::NotFound {
                path,
                field: field.to_string(),
            });
        };

        if entry.type_id != TypeId::of::<T>() {
            trace("GET", ctx, offset, &path, field, "<type mismatch>");
            return Err(ConfigError::TypeMismatch {
                path,
                field: field.to_string(),
                stored: entry.type_name,
                requested: std::any::type_name::<T>(),
            });
        }

        trace("GET", ctx, offset, &path, field, &entry.rendered);
        Ok(entry
            .value
            .downcast_ref::<T>()
            .expect("type id checked above")
            .clone())
    }

    /// Is there a value for this field, without producing an error?
    pub fn exists(ctx: Option<&RustdvCtx>, offset: &str, field: &str) -> bool {
        let path = resolve(ctx, offset);
        STORE.with(|s| {
            s.borrow()
                .iter()
                .any(|(pattern, fields)| glob_match(&path, pattern) && fields.contains_key(field))
        })
    }

    /// Log every `set` and `get` as it happens: the context, the offset, and
    /// the path they resolved to — which is what you actually got wrong when
    /// a lookup misses. Port of pyuvm's `ConfigDB().is_tracing`.
    pub fn set_tracing(on: bool) {
        TRACING.with(|t| t.set(on));
    }

    pub fn is_tracing() -> bool {
        TRACING.with(|t| t.get())
    }

    /// Print the whole database, **including precedences**. A resolved value
    /// tells you who won; this tells you who else was competing, which is
    /// what a parent/child conflict needs (D68).
    pub fn print() {
        for line in ConfigDb::dump().lines() {
            rustdv_sim::log::info(line);
        }
    }

    /// The dump as a string, for tests and for callers that want to route it
    /// somewhere other than the log.
    pub fn dump() -> String {
        let mut out = format!("{:<28}: {:<10}: {}", "PATH", "KEY", "DATA");
        STORE.with(|s| {
            for (path, fields) in s.borrow().iter() {
                for (field, by_prec) in fields.iter() {
                    let data = by_prec
                        .iter()
                        .rev()
                        .map(|(p, e)| format!("{p}: {}", e.rendered))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!("\n{path:<28}: {field:<10}: {{{data}}}"));
                }
            }
        });
        out
    }

    /// The factory overrides in force, as `(path, from, to)` — the entries
    /// the factory stores here under a reserved key prefix (D75). Used by
    /// `Factory::print`.
    pub fn factory_overrides() -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        STORE.with(|s| {
            for (path, fields) in s.borrow().iter() {
                for (field, by_prec) in fields.iter() {
                    if let Some(from) = field.strip_prefix("__factory_override__")
                        && let Some((_, e)) = by_prec.iter().next_back()
                    {
                        let to = e.rendered.trim_start_matches("-> ").to_string();
                        out.push((path.clone(), from.to_string(), to));
                    }
                }
            }
        });
        out
    }

    /// Drop every entry. The runner calls this between tests (D16), so a
    /// test never inherits another's configuration.
    pub fn clear() {
        STORE.with(|s| s.borrow_mut().clear());
        TRACING.with(|t| t.set(false));
        IN_BUILD.with(|b| b.set(false));
    }
}

/// How deep is this path? The root test is 0. Used for build-phase
/// precedence, so an ancestor outranks a descendant (D13).
fn depth_of(path: &str) -> i32 {
    if path.is_empty() {
        0
    } else {
        path.matches('.').count() as i32
    }
}

/// The phaser brackets the build walk with this, so writes made during
/// `build` take depth-scaled precedence.
pub(crate) fn set_in_build(active: bool) {
    IN_BUILD.with(|b| b.set(active));
}

// ===========================================================================
// Tests — no simulator. The ConfigDb is a path-keyed map; nothing here waits.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::RustdvCtx;

    fn fresh() {
        ConfigDb::clear();
    }

    #[test]
    fn set_and_get_round_trip() {
        fresh();
        ConfigDb::set(None, "env.tester", "COUNT", 7u32);
        let ctx = RustdvCtx::for_test("env.tester");
        assert_eq!(ConfigDb::get::<u32>(Some(&ctx), "", "COUNT").unwrap(), 7);
    }

    /// D14, and the reason `get` returns a `Result` at all: SystemVerilog's
    /// `get()` collapses never-set, path mismatch, field typo and type
    /// mismatch into a silent `return 0`, and you find out much later.
    #[test]
    fn a_miss_is_an_error_not_a_default() {
        fresh();
        let ctx = RustdvCtx::for_test("env");
        let got = ConfigDb::get::<u32>(Some(&ctx), "", "NOPE");
        match got {
            Err(ConfigError::NotFound { field, .. }) => assert_eq!(field, "NOPE"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn a_wrong_type_names_both_types() {
        fresh();
        ConfigDb::set(None, "env", "N", 1u32);
        let ctx = RustdvCtx::for_test("env");
        match ConfigDb::get::<String>(Some(&ctx), "", "N") {
            Err(ConfigError::TypeMismatch {
                stored, requested, ..
            }) => {
                assert!(stored.contains("u32"), "stored type named: {stored}");
                assert!(
                    requested.contains("String"),
                    "requested type named: {requested}"
                );
            }
            other => panic!("expected TypeMismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_wildcard_reaches_every_component_below() {
        fresh();
        ConfigDb::set(None, "*", "BFM", 99u32);
        for path in ["env", "env.tester", "env.agent.driver"] {
            let ctx = RustdvCtx::for_test(path);
            assert_eq!(
                ConfigDb::get::<u32>(Some(&ctx), "", "BFM").unwrap(),
                99,
                "`*` should reach {path}"
            );
        }
    }

    #[test]
    fn a_more_specific_path_wins_over_a_wildcard() {
        fresh();
        ConfigDb::set(None, "*", "MSG", String::from("everyone"));
        ConfigDb::set(None, "env.loga", "MSG", String::from("just me"));
        let loga = RustdvCtx::for_test("env.loga");
        let logb = RustdvCtx::for_test("env.logb");
        assert_eq!(
            ConfigDb::get::<String>(Some(&loga), "", "MSG").unwrap(),
            "just me"
        );
        assert_eq!(
            ConfigDb::get::<String>(Some(&logb), "", "MSG").unwrap(),
            "everyone"
        );
    }

    /// The `ab`-under-`a` case: a glob must not match a longer sibling name.
    #[test]
    fn a_prefix_glob_does_not_match_a_longer_sibling() {
        fresh();
        ConfigDb::set(None, "env.t*", "MSG", String::from("t-things"));
        let tester = RustdvCtx::for_test("env.tester");
        let logger = RustdvCtx::for_test("env.logger");
        assert!(ConfigDb::get::<String>(Some(&tester), "", "MSG").is_ok());
        assert!(
            ConfigDb::get::<String>(Some(&logger), "", "MSG").is_err(),
            "env.t* must not reach env.logger"
        );
    }

    #[test]
    fn the_most_recent_write_wins_at_equal_precedence() {
        fresh();
        ConfigDb::set(None, "env", "N", 1u32);
        ConfigDb::set(None, "env", "N", 2u32);
        let ctx = RustdvCtx::for_test("env");
        assert_eq!(ConfigDb::get::<u32>(Some(&ctx), "", "N").unwrap(), 2);
    }

    #[test]
    fn an_offset_resolves_against_the_context() {
        fresh();
        ConfigDb::set(None, "env.loga", "MSG", String::from("hello"));
        let env = RustdvCtx::for_test("env");
        // The env asks what its child will see.
        assert_eq!(
            ConfigDb::get::<String>(Some(&env), "loga", "MSG").unwrap(),
            "hello"
        );
    }

    #[test]
    fn a_null_context_addresses_from_the_top() {
        fresh();
        ConfigDb::set(None, "env.loga", "MSG", String::from("hello"));
        assert_eq!(
            ConfigDb::get::<String>(None, "env.loga", "MSG").unwrap(),
            "hello"
        );
    }

    /// The per-test guarantee the runner relies on — and, since D101, the one
    /// that keeps a test from inheriting the previous test's BFM.
    #[test]
    fn clear_empties_it() {
        fresh();
        ConfigDb::set(None, "env", "N", 1u32);
        ConfigDb::clear();
        let ctx = RustdvCtx::for_test("env");
        assert!(ConfigDb::get::<u32>(Some(&ctx), "", "N").is_err());
    }

    /// D101: the ConfigDb holds *handles* now, not just config values.
    #[test]
    fn it_holds_a_shared_handle() {
        use std::rc::Rc;
        fresh();
        #[derive(Debug)]
        struct Bfm(u32);
        let bfm = Rc::new(Bfm(7));
        ConfigDb::set(None, "*", "BFM", bfm.clone());
        let ctx = RustdvCtx::for_test("env.driver");
        let got: Rc<Bfm> = ConfigDb::get(Some(&ctx), "", "BFM").unwrap();
        assert_eq!(got.0, 7);
        assert!(Rc::ptr_eq(&got, &bfm), "the same object, not a copy");
    }

    #[test]
    fn dump_renders_every_entry() {
        fresh();
        ConfigDb::set(None, "env", "A", 1u32);
        ConfigDb::set(None, "env.x", "B", String::from("two"));
        let dumped = ConfigDb::dump();
        assert!(dumped.contains("env"), "dump names the paths: {dumped}");
        assert!(
            dumped.contains('A') && dumped.contains('B'),
            "and the fields"
        );
    }
}

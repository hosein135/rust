//! The factory (design-doc §15, D69–D75).
//!
//! rustdv has a factory, and it works as the UVM factory works (D74). It is
//! not a separate subsystem: a "maker" is an ordinary Rust value, so the
//! override table is the [`ConfigDb`](crate::config) and registration uses a
//! `linkme` distributed slice like the test registry. From the user's chair there are
//! two constructors — `Foo::new_comp()` (fixed) and `Foo::create_comp()`
//! (overridable) — and `Factory::…` to install and inspect overrides.
//!
//! **How a `create_comp()` slot is overridden.** It is not resolved at the
//! call — a `create_comp()` builds the default type immediately and flags the
//! [`RustdvComp`] as factory-owned (D75). During the build walk, where the field
//! name and so the path are finally known, the framework asks each flagged
//! slot for its override (instance first, then type, by ConfigDb specificity,
//! D13) and swaps it in before descending.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

use crate::component::{Component, ComponentNode, RustdvCtx};
use crate::config::ConfigDb;
use crate::port::PortOwner;
use linkme::distributed_slice;

/// A maker: builds a component with no arguments (its name and parent come
/// from the tree, D7). Non-capturing, so it is an ordinary `fn` pointer.
pub type Maker = fn() -> Box<dyn ComponentNode>;

// ===========================================================================
// RustdvComp — the child slot (D75)
// ===========================================================================

/// A slot that holds any component. It says nothing about position in the
/// tree and nothing about overridability; the *build line* decides that
/// (`new_comp()` fixed, `create_comp()` overridable). Every child field a
/// block may want to override is an `RustdvComp`.
#[derive(Default)]
pub struct RustdvComp {
    inner: Option<Box<dyn ComponentNode>>,
    /// Set by `create_comp()`; the walk checks flagged slots for an override.
    overridable: bool,
    /// The requested type's registered name, for the override lookup.
    requested: Option<&'static str>,
}

impl RustdvComp {
    /// A fixed slot: `new_comp()`. Never overridden.
    pub fn fixed(node: Box<dyn ComponentNode>) -> RustdvComp {
        RustdvComp {
            inner: Some(node),
            overridable: false,
            requested: None,
        }
    }

    /// A factory slot: `create_comp()`. The default is built now and may be
    /// swapped for an override during the walk.
    pub fn overridable(node: Box<dyn ComponentNode>, requested: &'static str) -> RustdvComp {
        RustdvComp {
            inner: Some(node),
            overridable: true,
            requested: Some(requested),
        }
    }

    /// The held component, shared, for asking it things — chiefly for one of
    /// its ports during `connect`. `None` before the slot is built.
    pub fn as_node(&self) -> Option<&(dyn ComponentNode + 'static)> {
        self.inner.as_deref()
    }

    /// The held component, for the traversal. `None` before it is filled.
    /// The object lifetime is `'static` (a boxed component always is), which
    /// matches [`ComponentNode::children_mut`]'s element type.
    pub fn as_node_mut(&mut self) -> Option<&mut (dyn ComponentNode + 'static)> {
        self.inner.as_deref_mut()
    }

    /// Move the held component **out** of the slot, leaving it empty (D82b).
    ///
    /// This is what lets a parent's `run` be concurrent with its children's.
    /// While the box sits in the slot it is part of the parent, so `&mut
    /// parent` and `&mut child` overlap and cannot both exist. Once moved out
    /// it is an independent value with no borrow relationship to the parent,
    /// so both futures can be driven together.
    ///
    /// The slot is empty only for the duration of the run phase;
    /// [`RustdvComp::put_node`] restores it before the post-run phases walk the tree.
    pub fn take_node(&mut self) -> Option<Box<dyn ComponentNode>> {
        self.inner.take()
    }

    /// Put a component taken by [`RustdvComp::take_node`] back into the slot.
    pub fn put_node(&mut self, node: Box<dyn ComponentNode>) {
        self.inner = Some(node);
    }

    /// Called by the derive-generated resolver during the build walk, with
    /// this slot's field name. If flagged and an override applies at the
    /// slot's path, swap it in. The discarded default's phases never ran —
    /// resolution happens before the walk descends into the child.
    pub fn resolve(&mut self, ctx: &RustdvCtx, name: &str) {
        if !self.overridable {
            return;
        }
        let Some(req) = self.requested else { return };
        let path = if ctx.path().is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", ctx.path(), name)
        };
        if let Some(ov) = Factory::lookup_override(req, &path) {
            self.inner = Some((ov.make)());
        }
        self.overridable = false;
    }
}

/// A slot is a [`PortOwner`], so `connect(&self.producer, ..)` works on an
/// erased child exactly as `connect(self, ..)` works on the connecting
/// component. Both questions are answered by a `ComponentNode` method, which
/// is reachable through `dyn` — no cast to the child's concrete type, which
/// Rust would not allow anyway.
impl PortOwner for RustdvComp {
    fn owner_port_slot(&self, name: &str) -> Option<std::rc::Rc<dyn std::any::Any>> {
        self.as_node()?.port_slot(name)
    }
    fn owner_label(&self) -> &'static str {
        match self.as_node() {
            Some(n) => n.node_name(),
            // An empty slot: the build phase never created this child. Say so
            // rather than reporting a missing port on a nameless component.
            None => "an unbuilt child slot",
        }
    }
}

// ===========================================================================
// Overrides
// ===========================================================================

/// An installed override: what to build, and the target's name for the dump.
#[derive(Clone, Copy)]
pub struct Override {
    make: Maker,
    to: &'static str,
}

impl fmt::Debug for Override {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "-> {}", self.to)
    }
}

fn override_key(requested_name: &str) -> String {
    format!("__factory_override__{requested_name}")
}

// ===========================================================================
// Universal registration (D73) — a linkme distributed slice, like the test
// registry. linkme owns the platform-specific linker section implementation.
// ===========================================================================

/// One registered component: its name and its maker. Emitted by
/// `#[derive(Component)]` for every component, universally (D73).
pub struct ComponentReg {
    /// Accessor rather than a const string, so the derive can compute it
    /// from the type without `const` gymnastics.
    pub name: fn() -> &'static str,
    pub make: Maker,
}

/// All component registrations contributed by `#[derive(Component)]`.
///
/// Public only so the derive macro can name it from a downstream crate.
#[doc(hidden)]
#[distributed_slice]
pub static COMPONENT_REGISTRATIONS: [ComponentReg];

fn collect_registry() -> HashMap<&'static str, Maker> {
    let mut map = HashMap::new();
    for reg in COMPONENT_REGISTRATIONS {
        map.insert((reg.name)(), reg.make);
    }
    map
}

thread_local! {
    /// Built once from the link-time section (the set of types does not
    /// change per test, unlike the override table).
    static REGISTRY: RefCell<Option<HashMap<&'static str, Maker>>> = const { RefCell::new(None) };
}

fn with_registry<R>(f: impl FnOnce(&HashMap<&'static str, Maker>) -> R) -> R {
    REGISTRY.with(|r| {
        let mut slot = r.borrow_mut();
        if slot.is_none() {
            *slot = Some(collect_registry());
        }
        f(slot.as_ref().unwrap())
    })
}

// ===========================================================================
// The facade
// ===========================================================================

/// The factory. Ambient, like the ConfigDb it is built on; every method is
/// an associated function.
pub struct Factory;

impl Factory {
    /// Build a component from its registered string name (D71). Overridable,
    /// like anything from the factory. A name that is not registered is a
    /// testbench bug and panics; the file-driven form (ch39) will return a
    /// `Result` instead.
    pub fn create_by_name(name: &str) -> RustdvComp {
        let make = with_registry(|reg| reg.get(name).copied());
        match make {
            Some(make) => {
                // `requested` needs a 'static name; recover the registry's
                // key so a by-name-created component can also be overridden.
                let stored = with_registry(|reg| reg.keys().find(|k| **k == name).copied());
                RustdvComp::overridable(make(), stored.expect("just found it"))
            }
            None => panic!("Factory::create_by_name: no component registered as \"{name}\""),
        }
    }

    /// Override every `From::create_comp()` with a `To`, testbench-wide
    /// (UVM `set_type_override_by_type`). Compile-checked: `To` must be a
    /// component.
    pub fn set_type_override<From, To>()
    where
        From: Component + ComponentNode + Default + 'static,
        To: Component + ComponentNode + Default + 'static,
    {
        Self::store_override(None, "*", From::comp_name(), To::comp_name(), || {
            Box::new(To::default())
        });
    }

    /// The same, by string name (UVM `set_type_override_by_name`). Not
    /// compile-checked; an unregistered `to` panics at this call.
    pub fn set_type_override_by_name(from: &str, to: &str) {
        let make = with_registry(|reg| reg.get(to).copied()).unwrap_or_else(|| {
            panic!("Factory::set_type_override_by_name: \"{to}\" is not registered")
        });
        let to_static =
            with_registry(|reg| reg.keys().find(|k| **k == to).copied()).expect("just found it");
        Self::store_override(None, "*", from, to_static, make);
    }

    /// Override a single instance, addressed by its path relative to `ctx`
    /// (UVM `set_inst_override_by_type`). The path is a string because it
    /// names a component elsewhere in the tree — not a duplicate of a field
    /// name (D75).
    pub fn set_inst_override<From, To>(ctx: &RustdvCtx, path: &str)
    where
        From: Component + ComponentNode + Default + 'static,
        To: Component + ComponentNode + Default + 'static,
    {
        Self::store_override(Some(ctx), path, From::comp_name(), To::comp_name(), || {
            Box::new(To::default())
        });
    }

    fn store_override(
        ctx: Option<&RustdvCtx>,
        offset: &str,
        from_name: &str,
        to_name: &'static str,
        make: Maker,
    ) {
        ConfigDb::set(
            ctx,
            offset,
            &override_key(from_name),
            Override { make, to: to_name },
        );
    }

    /// The override in force for `requested_name` at `abs_path`, if any.
    pub(crate) fn lookup_override(requested_name: &str, abs_path: &str) -> Option<Override> {
        ConfigDb::get::<Override>(None, abs_path, &override_key(requested_name)).ok()
    }

    /// Print the overrides in force (UVM `uvm_factory().print()`). It is the
    /// ConfigDb store, shown through the factory's window (D68).
    pub fn print() {
        rustdv_sim::log::info("Factory overrides:");
        for (path, from, to) in ConfigDb::factory_overrides() {
            rustdv_sim::log::info(&format!("  {path:<28}: {from} -> {to}"));
        }
    }
}

// ===========================================================================
// Tests — no simulator.
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{Component, RustdvCtx};
    use crate::config::ConfigDb;
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static BUILT: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }

    fn record(name: &'static str) {
        BUILT.with(|b| b.borrow_mut().push(name));
    }

    #[derive(Default)]
    struct Base;
    impl Component for Base {
        fn build(&mut self, _ctx: &mut RustdvCtx) {
            record("Base");
        }
    }
    impl ComponentNode for Base {
        fn node_name(&self) -> &'static str {
            "Base"
        }
        fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
            Vec::new()
        }
    }

    #[derive(Default)]
    struct Derived;
    impl Component for Derived {
        fn build(&mut self, _ctx: &mut RustdvCtx) {
            record("Derived");
        }
    }
    impl ComponentNode for Derived {
        fn node_name(&self) -> &'static str {
            "Derived"
        }
        fn children_mut(&mut self) -> Vec<(String, &mut (dyn ComponentNode + 'static))> {
            Vec::new()
        }
    }

    fn fresh() {
        ConfigDb::clear();
        BUILT.with(|b| b.borrow_mut().clear());
    }

    #[test]
    fn a_fixed_slot_holds_what_it_was_given() {
        fresh();
        let slot = RustdvComp::fixed(Box::new(Base));
        assert_eq!(slot.as_node().unwrap().node_name(), "Base");
    }

    #[test]
    fn an_empty_slot_reports_itself_by_name() {
        fresh();
        let slot = RustdvComp::default();
        assert!(slot.as_node().is_none());
        assert_eq!(slot.owner_label(), "an unbuilt child slot");
    }

    /// D75: `new_comp()` is fixed and `create_comp()` is overridable, and the
    /// *build line* carries that choice — not the field type.
    #[test]
    fn a_type_override_swaps_a_create_slot_and_not_a_new_slot() {
        fresh();
        Factory::set_type_override::<Base, Derived>();
        let ctx = RustdvCtx::for_test("env");

        let mut overridable = RustdvComp::overridable(Box::new(Base), "Base");
        overridable.resolve(&ctx, "tester");
        assert_eq!(overridable.as_node().unwrap().node_name(), "Derived");

        let mut fixed = RustdvComp::fixed(Box::new(Base));
        fixed.resolve(&ctx, "scoreboard");
        assert_eq!(
            fixed.as_node().unwrap().node_name(),
            "Base",
            "new_comp is never swapped"
        );
    }

    #[test]
    fn no_override_leaves_the_requested_type() {
        fresh();
        let ctx = RustdvCtx::for_test("env");
        let mut slot = RustdvComp::overridable(Box::new(Base), "Base");
        slot.resolve(&ctx, "tester");
        assert_eq!(slot.as_node().unwrap().node_name(), "Base");
    }

    /// D13's precedence, applied to the factory: an instance override beats a
    /// type override at the same path.
    #[test]
    fn an_instance_override_beats_a_type_override() {
        fresh();
        let ctx = RustdvCtx::for_test("env");
        Factory::set_type_override::<Base, Base>();
        Factory::set_inst_override::<Base, Derived>(&ctx, "tester");
        let mut slot = RustdvComp::overridable(Box::new(Base), "Base");
        slot.resolve(&ctx, "tester");
        assert_eq!(slot.as_node().unwrap().node_name(), "Derived");
    }

    #[test]
    fn an_instance_override_applies_only_at_its_path() {
        fresh();
        let ctx = RustdvCtx::for_test("env");
        Factory::set_inst_override::<Base, Derived>(&ctx, "tester");

        let mut here = RustdvComp::overridable(Box::new(Base), "Base");
        here.resolve(&ctx, "tester");
        assert_eq!(here.as_node().unwrap().node_name(), "Derived");

        let mut elsewhere = RustdvComp::overridable(Box::new(Base), "Base");
        elsewhere.resolve(&ctx, "other");
        assert_eq!(elsewhere.as_node().unwrap().node_name(), "Base");
    }

    /// D75's guarantee: resolution happens before the walk descends, so the
    /// discarded default's own phases never run.
    #[test]
    fn the_discarded_default_never_built() {
        fresh();
        Factory::set_type_override::<Base, Derived>();
        let ctx = RustdvCtx::for_test("env");
        let mut slot = RustdvComp::overridable(Box::new(Base), "Base");
        slot.resolve(&ctx, "tester");
        // Neither has been built yet — but the point is that the *Base* we
        // threw away is gone before any walk could reach it.
        assert_eq!(BUILT.with(|b| b.borrow().len()), 0);
        assert_eq!(slot.as_node().unwrap().node_name(), "Derived");
    }

    /// Resolving twice must not re-apply: the slot is no longer overridable
    /// once the walk has passed it.
    #[test]
    fn a_slot_resolves_once() {
        fresh();
        let ctx = RustdvCtx::for_test("env");
        let mut slot = RustdvComp::overridable(Box::new(Base), "Base");
        slot.resolve(&ctx, "tester");
        Factory::set_type_override::<Base, Derived>(); // installed too late
        slot.resolve(&ctx, "tester");
        assert_eq!(
            slot.as_node().unwrap().node_name(),
            "Base",
            "an override installed after the walk passed does not apply"
        );
    }

    #[test]
    fn take_and_put_move_the_box_out_and_back() {
        fresh();
        let mut slot = RustdvComp::fixed(Box::new(Base));
        let node = slot.take_node().expect("something to take");
        assert!(
            slot.as_node().is_none(),
            "the slot is empty during the run phase"
        );
        slot.put_node(node);
        assert_eq!(
            slot.as_node().unwrap().node_name(),
            "Base",
            "and restored after"
        );
    }

    // --- the sequence half of the factory (D80/D96) ----------------------

    use crate::sequence::{
        SeqCtx, SeqError, Sequence, clear_seq_overrides, create_seq, set_seq_override,
    };

    #[derive(Default)]
    struct BaseSeq;
    #[derive(Default)]
    struct RandomSeq;

    impl Sequence for BaseSeq {
        type Req = u8;
        type Rsp = u8;
        async fn body(&mut self, _c: &mut SeqCtx<u8, u8>) -> Result<(), SeqError> {
            Ok(())
        }
        fn seq_name(&self) -> &'static str {
            "BaseSeq"
        }
    }
    impl Sequence for RandomSeq {
        type Req = u8;
        type Rsp = u8;
        async fn body(&mut self, _c: &mut SeqCtx<u8, u8>) -> Result<(), SeqError> {
            Ok(())
        }
        fn seq_name(&self) -> &'static str {
            "RandomSeq"
        }
    }

    #[test]
    fn create_seq_builds_the_requested_type_by_default() {
        clear_seq_overrides();
        let seq = create_seq::<BaseSeq>();
        assert_eq!(seq.name(), "BaseSeq");
    }

    #[test]
    fn a_sequence_override_swaps_the_type() {
        clear_seq_overrides();
        set_seq_override::<BaseSeq, RandomSeq>();
        let seq = create_seq::<BaseSeq>();
        assert_eq!(
            seq.name(),
            "RandomSeq",
            "the test asked for Base and got Random"
        );
    }

    #[test]
    fn clearing_sequence_overrides_restores_the_default() {
        clear_seq_overrides();
        set_seq_override::<BaseSeq, RandomSeq>();
        clear_seq_overrides();
        assert_eq!(create_seq::<BaseSeq>().name(), "BaseSeq");
    }

    #[test]
    fn an_override_on_one_sequence_leaves_others_alone() {
        clear_seq_overrides();
        set_seq_override::<BaseSeq, RandomSeq>();
        assert_eq!(create_seq::<RandomSeq>().name(), "RandomSeq");
    }

    // Unused-import guard: `Rc` is here for future handle tests.
    #[allow(dead_code)]
    fn _rc_in_scope(_: Rc<u8>) {}
}

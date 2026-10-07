//! Chapter 27: Configuration — the ConfigDb.
//!
//!     sim-common/run_sim.sh ch27_configuration playground
//!
//! No DUT: the subject is how a value reaches a component that nobody
//! passed it to.
//!
//! This is the chapter the previous rustdv pass argued *against*. Its
//! Chapter 27 was titled "Configuration: The ConfigDB Problem, Solved by
//! Types" — the config DB replaced by typed structs passed to constructors.
//! That is the closed-world answer: it configures only what you wrote
//! yourself. Every component below is configured by a *test* that never
//! touches it, through a path it names as a string, and no constructor
//! anywhere carries the value. That is the capability being restored
//! (D11–D16).
//!
//! Port of the Python book's chapter 31.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Reading configuration
// ===========================================================================

// Chapter 27, Figure 1: Logging a message we get from the ConfigDb.
//
// Note what this component does *not* have: a constructor argument, a field
// holding the message, or any knowledge of who set it. It asks the database.
//
// **Read the three arguments as: context, offset, field.** The store is
// ambient; the *identity* is passed in (D10). The `""` is not a keyword
// meaning "me" — it is an **empty offset** from the context, which happens
// to land on the caller. A non-empty offset asks on behalf of another
// component:
//
//     ConfigDb::get(Some(ctx), "",     "MSG")   // me
//     ConfigDb::get(Some(ctx), "loga", "MSG")   // what loga will see
//     ConfigDb::get(None,      "",     "SEQR")  // no context at all
//
// That third form matters more than it looks: a `Sequence` is not a
// component and has no path, so it must be able to ask with no context.
// Both source books keep exactly this signature, and in the SV Primer the
// null-context form is the *majority* idiom.
//
// Offsets are relative for the same reason logging paths are derived (D7):
// an absolute path is a hand-typed string that keeps compiling and starts
// lying the moment a component moves.
//
// **A value you store in the ConfigDb must either implement the `Clone`
// trait so that everyone gets their own copy, or be wrapped in an `Rc` so
// that everyone gets a handle to a single copy.** `MSG` is a `String`, and
// a copy is what each logger wants. A sequencer is not: when Chapter 36
// puts one in the database, every sequence must reach the *same* sequencer,
// so it goes in as an `Rc`.
//
// `get` returns a `Result`, not a value-or-silent-zero. SystemVerilog's
// `get()` collapses never-set, path mismatch, field typo and type mismatch
// into `return 0` and leaves your variable untouched; ours names which one
// happened, and `#[must_use]` means you cannot quietly ignore it (D14).
#[derive(Component, Default)]
struct MsgLogger;

impl Component for MsgLogger {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("logging the configured message");
        let msg: String = ConfigDb::get(Some(ctx), "", "MSG")?;
        ctx.info(&msg);
        Ok(())
    }
}

// ===========================================================================
// Writing configuration
// ===========================================================================

// Chapter 27, Figure 2: Two loggers in the environment.
#[derive(Component, Default)]
struct MsgEnv {
    #[component]
    loga: Option<MsgLogger>,
    #[component]
    logb: Option<MsgLogger>,
}

impl Component for MsgEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.loga = Some(MsgLogger::default());
        self.logb = Some(MsgLogger::default());
    }
}

// Chapter 27, Figure 3: Giving loga and logb different messages.
//
// The paths are *relative to the setter* — "env.loga" resolves against this
// test's own path (D12). The test reaches two components it did not write
// and never holds a handle to.
#[rustdv::test]
#[derive(Component, Default)]
struct MsgTest {
    #[component]
    env: Option<MsgEnv>,
}

impl Component for MsgTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(MsgEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
        ConfigDb::set(Some(ctx), "env.logb", "MSG", String::from("LOG B msg"));
    }
}

// ===========================================================================
// Wildcards
// ===========================================================================

// Chapter 27, Figure 5: Adding talka and talkb to the environment.
//
// Python subclasses MsgEnv and calls `super().build_phase()`. Rust has no
// inheritance and the difference here is *structural* — which children
// exist — so the env is written out. Four fields is cheaper to read than a
// mechanism for sharing two of them.
#[derive(Component, Default)]
struct MultiMsgEnv {
    #[component]
    loga: Option<MsgLogger>,
    #[component]
    logb: Option<MsgLogger>,
    #[component]
    talka: Option<MsgLogger>,
    #[component]
    talkb: Option<MsgLogger>,
}

impl Component for MultiMsgEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.loga = Some(MsgLogger::default());
        self.logb = Some(MsgLogger::default());
        self.talka = Some(MsgLogger::default());
        self.talkb = Some(MsgLogger::default());
    }
}

// Chapter 27, Figure 6: Using a wildcard to configure both talkers at once.
//
// `set` takes a glob; `get` takes a concrete path. pyuvm enforces the same
// asymmetry, and it is the right way round: you write to a *pattern* of
// components and read as *one* component (D12).
#[rustdv::test]
#[derive(Component, Default)]
struct MultiMsgTest {
    #[component]
    env: Option<MultiMsgEnv>,
}

impl Component for MultiMsgTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(MultiMsgEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
        ConfigDb::set(Some(ctx), "env.logb", "MSG", String::from("LOG B msg"));
        ConfigDb::set(Some(ctx), "env.t*", "MSG", String::from("TALK TALK"));
    }
}

// ===========================================================================
// Global data
// ===========================================================================

// Chapter 27, Figure 8: Adding gtalk, which nobody configures by name.
#[derive(Component, Default)]
struct GlobalEnv {
    #[component]
    loga: Option<MsgLogger>,
    #[component]
    logb: Option<MsgLogger>,
    #[component]
    talka: Option<MsgLogger>,
    #[component]
    talkb: Option<MsgLogger>,
    #[component]
    gtalk: Option<MsgLogger>,
}

impl Component for GlobalEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.loga = Some(MsgLogger::default());
        self.logb = Some(MsgLogger::default());
        self.talka = Some(MsgLogger::default());
        self.talkb = Some(MsgLogger::default());
        self.gtalk = Some(MsgLogger::default());
    }
}

// Chapter 27, Figure 9: Storing a message for everybody.
//
// A `None` context is the port of pyuvm's `ConfigDB().set(None, ...)` and
// SystemVerilog's `set(null, ...)`: with no context to offset from, the
// glob is absolute. `"*"` matches every path, so it is the fallback for any
// component no more specific rule names — here, `gtalk`.
//
// Resolution is **most specific first** (D13): `env.loga` beats `env.t*`
// beats `*`. That is pyuvm's rule. SystemVerilog does not sort by
// specificity at all — it gathers every regex match and takes the highest
// precedence — and the book must say so rather than let a reader assume
// 1800.2 behaviour.
#[rustdv::test]
#[derive(Component, Default)]
struct GlobalTest {
    #[component]
    env: Option<GlobalEnv>,
}

impl Component for GlobalTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(GlobalEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
        ConfigDb::set(Some(ctx), "env.logb", "MSG", String::from("LOG B msg"));
        ConfigDb::set(Some(ctx), "env.t*", "MSG", String::from("TALK TALK"));
        ConfigDb::set(None, "*", "MSG", String::from("GLOBAL"));
    }
}

// ===========================================================================
// Parent/child conflict
// ===========================================================================

// Chapter 27, Figure 11: The env configures its own child...
#[derive(Component, Default)]
struct ConflictEnv {
    #[component]
    loga: Option<MsgLogger>,
}

impl Component for ConflictEnv {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.loga = Some(MsgLogger::default());
        ConfigDb::set(Some(ctx), "loga", "MSG", String::from("CHILD RULES!"));
    }
}

// Chapter 27, Figure 12: ...and the test configures the same component by a
// longer path. Both resolve to exactly the same place. Which message prints?
//
// **The parent wins**, and not because it wrote first — it wrote *earlier*,
// which under "last write wins" would make it lose. Build is top-down, so
// an ancestor always writes before its descendants. If recency decided,
// every child could silently overrule the test that instantiated it, and
// configuration from the top would be useless.
//
// So a build-phase write carries a precedence of `default - depth`: the
// shallower the setter, the higher the precedence, regardless of order
// (D13, tier 2). The test is shallower than the env, so "PARENT RULES!"
// prints. A write made *after* build carries full `default` precedence and
// outranks every build-time write.
#[rustdv::test]
#[derive(Component, Default)]
struct ConflictTest {
    #[component]
    env: Option<ConflictEnv>,
}

impl Component for ConflictTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(ConflictEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("PARENT RULES!"));
    }
}

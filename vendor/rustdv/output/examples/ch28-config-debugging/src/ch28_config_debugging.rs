//! Chapter 28: Debugging the ConfigDb.
//!
//!     sim-common/run_sim.sh ch28_config_debugging playground
//!
//! Chapter 27 showed configuration working. This one shows it *failing*,
//! which is the more useful skill: a path that matches nothing, a key
//! spelled two ways, and a parent and child fighting over the same
//! component. The database is addressed by strings resolved at run time, so
//! the compiler cannot help — and that is not a flaw to apologize for. It
//! is the cost of late binding, and it is why the debug tools below exist.
//!
//! Three of them:
//!
//! - a `Result` whose variants name *which* failure happened (D14),
//! - `ConfigDb::print()` — the whole database, including the precedences
//!   that decide conflicts,
//! - `ConfigDb::set_tracing(true)` — every set and get as it happens.
//!
//! Port of the Python book's chapter 32.

use rustdv::prelude::*;
use rustdv::ConfigError;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Missing data
// ===========================================================================

// Chapter 28, Figure 1: The logger from Chapter 27, unchanged. It asks for
// "MSG" and propagates the error with `?` if it is not there.
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

// Chapter 28, Figure 2: A message for only one of the two loggers.
//
// `logb` finds nothing and its `?` fails the test. `expect_error` says the
// test passes *only* if it fails this way — a test that failed for some
// other reason would still be reported as a failure, which a bare
// "expected to fail" flag could not tell you.
#[rustdv::test(expect_error = "config_not_found")]
#[derive(Component, Default)]
struct MsgTest {
    #[component]
    env: Option<MsgEnv>,
}

impl Component for MsgTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(MsgEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
    }
}

// Chapter 28, Figure 3: Misspelling a key.
//
// Both loggers are now configured — except one was stored under "MESG".
// The failure is identical to Figure 2's, because a key that was never
// written and a key written under another name are the same thing to the
// database. Nothing here is a compile error; "MESG" is a perfectly good
// string.
#[rustdv::test(expect_error = "config_not_found")]
#[derive(Component, Default)]
struct MsgTestAlmostFixed {
    #[component]
    env: Option<MsgEnv>,
}

impl Component for MsgTestAlmostFixed {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(MsgEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
        ConfigDb::set(Some(ctx), "env.logb", "MESG", String::from("LOG B msg"));
    }
}

// ===========================================================================
// Handling a missing value
// ===========================================================================

// Chapter 28, Figure 4: A logger that coped.
//
// Matching on the variant is the point. `NotFound` is recoverable — fall
// back to a default and say so. Any *other* failure is not: a type mismatch
// means the value is there and you asked for it wrongly, and defaulting
// would bury a real bug. Python's `except UVMConfigItemNotFound` draws the
// same line; the difference is that here the compiler makes you decide what
// happens to the errors you did not name.
#[derive(Component, Default)]
struct NiceMsgLogger;

impl Component for NiceMsgLogger {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("logging the configured message");
        let msg: String = match ConfigDb::get(Some(ctx), "", "MSG") {
            Ok(msg) => msg,
            Err(ConfigError::NotFound { .. }) => {
                ctx.warning("Could not find MSG. Setting to default");
                String::from("No message for you!")
            }
            Err(other) => return Err(other.into()),
        };
        ctx.info(&msg);
        Ok(())
    }
}

#[derive(Component, Default)]
struct NiceMsgEnv {
    #[component]
    loga: Option<NiceMsgLogger>,
    #[component]
    logb: Option<NiceMsgLogger>,
}

impl Component for NiceMsgEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.loga = Some(NiceMsgLogger::default());
        self.logb = Some(NiceMsgLogger::default());
    }
}

// ===========================================================================
// Printing the database
// ===========================================================================

// Chapter 28, Figure 5: Printing the ConfigDb.
//
// `end_of_elaboration` is the place for it: the hierarchy is final and
// nothing has run yet, so what you see is exactly what the run phase will
// resolve against.
#[rustdv::test]
#[derive(Component, Default)]
struct NiceMsgTest {
    #[component]
    env: Option<NiceMsgEnv>,
}

impl Component for NiceMsgTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(NiceMsgEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
    }

    fn end_of_elaboration(&mut self, _ctx: &mut RustdvCtx) {
        ConfigDb::print();
    }
}

// Chapter 28, Figure 6: Debugging the misspelled key by printing.
//
// The dump is what finds it. Two entries under `env.logb`-ish paths, one
// keyed "MSG" and one keyed "MESG", and the mismatch is visible in a way it
// never is at the point of failure.
#[rustdv::test]
#[derive(Component, Default)]
struct NiceMsgTestAlmostFixed {
    #[component]
    env: Option<NiceMsgEnv>,
}

impl Component for NiceMsgTestAlmostFixed {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.env = Some(NiceMsgEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
        ConfigDb::set(Some(ctx), "env.logb", "MESG", String::from("LOG B msg"));
    }

    fn end_of_elaboration(&mut self, _ctx: &mut RustdvCtx) {
        ConfigDb::print();
    }
}

// ===========================================================================
// Wildcards
// ===========================================================================

// Chapter 28, Figure 8: Wildcards behaving, for contrast with the failures.
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
// Debugging a parent/child conflict
// ===========================================================================

// Chapter 28, Figure 9: Both the env and the test configure `env.loga`.
//
// This is where the dump earns its keep. A resolved value tells you who
// won; it does not tell you that anyone else was competing. The dump lists
// **every** value stored at a path with the precedence each was written at,
// so the losing entry is visible — and the numbers explain the outcome
// rather than leaving you to trust the rule (D13).
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

    fn end_of_elaboration(&mut self, _ctx: &mut RustdvCtx) {
        ConfigDb::print();
    }
}

// ===========================================================================
// Tracing
// ===========================================================================

// Chapter 28, Figure 11: Tracing every ConfigDb operation.
//
// The dump is a snapshot; tracing is the film. Turn it on before building
// the hierarchy and every set and get is logged as it happens, with the
// context, the offset and the path they resolved to — which is the thing
// you actually got wrong when a lookup misses.
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

#[rustdv::test]
#[derive(Component, Default)]
struct GlobalTest {
    #[component]
    env: Option<GlobalEnv>,
}

impl Component for GlobalTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        ConfigDb::set_tracing(true);
        self.env = Some(GlobalEnv::default());
        ConfigDb::set(Some(ctx), "env.loga", "MSG", String::from("LOG A msg"));
        ConfigDb::set(Some(ctx), "env.logb", "MSG", String::from("LOG B msg"));
        ConfigDb::set(Some(ctx), "env.t*", "MSG", String::from("TALK TALK"));
        ConfigDb::set(None, "*", "MSG", String::from("GLOBAL"));
    }
}

//! Chapter 26: Logging.
//!
//!     sim-common/run_sim.sh ch26_logging playground
//!
//! No DUT: the subject is what a component says, not what it drives.
//!
//! The chapter's real lesson is in what these figures *do not* contain: a
//! path. `ctx.info(..)` is attributed to the component that called it, and
//! `ctx.set_logging_level_hier(..)` addresses that component's subtree,
//! because the context carries the path the phase walk derived (D7). The
//! alternative — `Logger::new("uvm_test_top.comp")` and
//! `set_level_for("uvm_test_top", ..)` typed by hand — is an unchecked
//! string that keeps compiling, and keeps lying, after the component moves.
//!
//! Port of the Python book's chapter 30.

use rustdv::prelude::*;
use rustdv::sim::log::Level;

rustdv::vpi_bootstrap!();

// Chapter 26, Figure 1: Logging messages of all levels.
// The default level is Info, so `debug` is filtered out.
#[derive(Component, Default)]
struct LogComp;

impl Component for LogComp {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("logging");
        ctx.debug("This is debug");
        ctx.info("This is info");
        ctx.warning("This is warning");
        ctx.error("This is error");
        ctx.critical("This is critical");
        Ok(())
    }
}

// Chapter 26, Figure 2: The logging policy is the only thing that varies
// between these four tests.
//
// Python writes `class DebugTest(LogTest)` three times, each overriding
// `end_of_elaboration_phase`. Rust has no inheritance, so what varies
// becomes a type parameter and the tests become type aliases (D28) — the
// same move as the testers in Chapter 25.
pub trait LogPolicy: Default {
    /// Called in `end_of_elaboration`, where pyuvm configures logging:
    /// the hierarchy is final, and nothing has run yet.
    fn configure(&self, ctx: &mut RustdvCtx);
}

#[derive(Component, Default)]
struct LogTest<P: LogPolicy + 'static> {
    #[component]
    comp: Option<LogComp>,
    policy: P,
}

impl<P: LogPolicy + 'static> Component for LogTest<P> {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.comp = Some(LogComp::default());
    }

    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        self.policy.configure(ctx);
    }
}

// Chapter 26, Figure 3: The default — no configuration at all.
#[derive(Default)]
pub struct DefaultLogging;

impl LogPolicy for DefaultLogging {
    fn configure(&self, _ctx: &mut RustdvCtx) {}
}

// Chapter 26, Figure 4: Setting the logging level for a hierarchy.
// `_hier` means "this component and everything under it" — and the
// component it means is the one holding the context.
#[derive(Default)]
pub struct DebugLogging;

impl LogPolicy for DebugLogging {
    fn configure(&self, ctx: &mut RustdvCtx) {
        ctx.set_logging_level_hier(Level::Debug);
    }
}

// Chapter 26, Figure 5: Writing log entries to a file, and taking this
// subtree off the console. The file handler still receives everything.
//
// The path is relative, so the log lands beside wherever you ran the
// simulation — not in a fixed system-wide location. That is deliberate: a
// shared absolute path like `/tmp/rustdv_ch26_log.txt` is one another user, or
// a leftover from an earlier run under a different account, can own and lock
// you out of. A test that writes outside its own working directory is a test
// that can be broken by something it has never heard of.
#[derive(Default)]
pub struct FileLogging;

impl LogPolicy for FileLogging {
    fn configure(&self, ctx: &mut RustdvCtx) {
        ctx.add_file_handler_hier("rustdv_ch26_log.txt", false)
            .expect("could not open the log file");
        ctx.remove_console_hier();
    }
}

// Chapter 26, Figure 6: Disabling logging for a hierarchy.
#[derive(Default)]
pub struct NoLogging;

impl LogPolicy for NoLogging {
    fn configure(&self, ctx: &mut RustdvCtx) {
        ctx.disable_logging_hier();
    }
}

// Chapter 26, Figure 7: The four tests are type aliases over one base.
//
// Each starts from a clean slate: the runner resets logging configuration
// between tests, the way pyuvm's run_test does, so FileTest cannot leave
// the console switched off for NoLog.
#[rustdv::test]
type LogTestDefault = LogTest<DefaultLogging>;

#[rustdv::test]
type DebugTest = LogTest<DebugLogging>;

#[rustdv::test]
type FileTest = LogTest<FileLogging>;

#[rustdv::test]
type NoLog = LogTest<NoLogging>;

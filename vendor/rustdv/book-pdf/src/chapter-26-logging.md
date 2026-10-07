# Chapter 26: Logging

With a hierarchy to hang them on, we can tour the features that live on it, starting with the one you have been reading all book: logging. Large testbenches generate more output than humans can read, so the game is filtering and directing — per component, per subtree, per destination — and the pyuvm surface for that game ports over nearly method-for-method.

> **In the UVM...** we logged through the framework. SystemVerilog: `` `uvm_info(get_type_name(), "msg", UVM_MEDIUM) ``, verbosities from `UVM_NONE` to `UVM_DEBUG`, opened per subtree with `set_report_verbosity_level_hier()`. pyuvm: `self.logger`, inherited from `uvm_report_object` — levels from DEBUG to CRITICAL, INFO the default threshold, `set_logging_level_hier(DEBUG)` for a subtree, and handlers to say where messages went. The path between square brackets, `[uvm_test_top.comp]`, told us who was talking.

## Creating log messages

```rust
// Chapter 26, Figure 1: Logging messages of all levels
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
```

Five levels, one method each, all on the context — `debug`, `info`, `warning`, `error`, `critical`, the same ladder pyuvm inherited from Python's `logging` module. The default threshold is Info, so when this component runs unconfigured, the debug line is filtered out. And notice what the component does *not* pass anywhere: a name. `ctx.info` is attributed to whoever called it, because the context knows.

## One component, four logging policies

The Python book demonstrates logging configuration by writing `LogTest` and then subclassing it three times, each subclass overriding `end_of_elaboration_phase` to configure differently. No inheritance here, so the varying part becomes — the same move as Chapter 25's testers — a type parameter:

```rust
// Chapter 26, Figure 2: The logging policy is the only thing that varies
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
```

`end_of_elaboration` is where logging configuration belongs, for the reason Chapter 28 will use it for database dumps: the hierarchy is final and nothing has run, so the policy governs every message the run phase will produce. The four policies:

```rust
// Chapter 26, Figure 3: The default — no configuration at all
#[derive(Default)]
pub struct DefaultLogging;

impl LogPolicy for DefaultLogging {
    fn configure(&self, _ctx: &mut RustdvCtx) {}
}
```

```rust
// Chapter 26, Figure 4: Setting the logging level for a hierarchy
#[derive(Default)]
pub struct DebugLogging;

impl LogPolicy for DebugLogging {
    fn configure(&self, ctx: &mut RustdvCtx) {
        ctx.set_logging_level_hier(Level::Debug);
    }
}
```

`_hier` means what it means in pyuvm: this component and everything under it. Which component? The one holding the context — no path argument, and none possible to typo.

```rust
// Chapter 26, Figure 5: Writing log entries to a file
#[derive(Default)]
pub struct FileLogging;

impl LogPolicy for FileLogging {
    fn configure(&self, ctx: &mut RustdvCtx) {
        ctx.add_file_handler_hier("rustdv_ch26_log.txt", false)
            .expect("could not open the log file");
        ctx.remove_console_hier();
    }
}
```

Two handler operations, both hierarchy-scoped: add a file destination for this subtree, and take the subtree off the console. The file path is deliberately relative — the log lands beside wherever you ran the simulation. A shared absolute path like `/tmp/rustdv_log.txt` is a file some other user, or a leftover from an earlier run under a different account, can own and lock you out of; a test that writes outside its own working directory can be broken by something it has never heard of.

```rust
// Chapter 26, Figure 6: Disabling logging for a hierarchy
#[derive(Default)]
pub struct NoLogging;

impl LogPolicy for NoLogging {
    fn configure(&self, ctx: &mut RustdvCtx) {
        ctx.disable_logging_hier();
    }
}
```

```rust
// Chapter 26, Figure 7: The four tests are type aliases over one base
#[rustdv::test]
type LogTestDefault = LogTest<DefaultLogging>;

#[rustdv::test]
type DebugTest = LogTest<DebugLogging>;

#[rustdv::test]
type FileTest = LogTest<FileLogging>;

#[rustdv::test]
type NoLog = LogTest<NoLogging>;
```

```text
# Figure 8: Four tests, four logging behaviors

      0.00ns INFO     rustdv: found 4 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running LogTestDefault (1/4)  [ch26-logging/src/ch26_logging.rs:124]
      0.00ns INFO     [LogTestDefault.comp]: This is info
      0.00ns WARNING  [LogTestDefault.comp]: This is warning
      0.00ns ERROR    [LogTestDefault.comp]: This is error
      0.00ns CRITICAL [LogTestDefault.comp]: This is critical
      0.00ns INFO     LogTestDefault PASSED
      0.00ns INFO     running DebugTest (2/4)  [ch26-logging/src/ch26_logging.rs:127]
      0.00ns DEBUG    [DebugTest.comp]: This is debug
      0.00ns INFO     [DebugTest.comp]: This is info
      0.00ns WARNING  [DebugTest.comp]: This is warning
      0.00ns ERROR    [DebugTest.comp]: This is error
      0.00ns CRITICAL [DebugTest.comp]: This is critical
      0.00ns INFO     DebugTest PASSED
      0.00ns INFO     running FileTest (3/4)  [ch26-logging/src/ch26_logging.rs:130]
      0.00ns INFO     FileTest PASSED
      0.00ns INFO     running NoLog (4/4)  [ch26-logging/src/ch26_logging.rs:133]
      0.00ns INFO     NoLog PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** LogTestDefault                               PASS           0.00      **
** DebugTest                                    PASS           0.00      **
** FileTest                                     PASS           0.00      **
** NoLog                                        PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

Read the transcript against the policies: the default test filters debug; the debug test shows it; the file test prints *nothing* — its subtree went off the console — and the no-log test is silent. Where did `FileTest`'s messages go?

```text
# Figure 9: rustdv_ch26_log.txt receives what the console did not

      0.00ns INFO     [FileTest.comp]: This is info
      0.00ns WARNING  [FileTest.comp]: This is warning
      0.00ns ERROR    [FileTest.comp]: This is error
      0.00ns CRITICAL [FileTest.comp]: This is critical
```

The file has one more thing to teach, by omission: `DebugTest` ran immediately before `FileTest` and set its level to Debug, yet the file holds no `This is debug` line. The runner resets logging configuration between tests, the way pyuvm's `run_test` does, so no test can leave the console switched off — or the level opened up — for the next one. No cleanup phase needed, and none to forget.

## What the figures do not contain

The chapter's real lesson is a thing missing from every listing: **a path.** `LogComp` is one type, written once, and it logged as `[LogTestDefault.comp]`, `[DebugTest.comp]`, `[FileTest.comp]` — whichever was true for the test it was built under, with nothing in the component saying so. `ctx.info` is attributed to its caller, and `ctx.set_logging_level_hier` addresses its caller's subtree, because the context carries the path the phase walk derived in Chapter 24. The alternative — a logger constructed with `"uvm_test_top.comp"` and a level set by path string — is a hand-typed name that keeps compiling, and keeps lying, after the component is renamed or moved. rustdv never asks you to type a path the tree already knows.

## Summary

Logging rides the context: five levels with Info as the default gate, `set_logging_level_hier` to open a subtree, file handlers and console removal per hierarchy, and `disable_logging_hier` for silence — pyuvm's surface, with the paths supplied by the framework instead of the engineer. Configuration happens in `end_of_elaboration`, applies to the run that follows, and is reset between tests by the runner. The four demonstrations share one generic test with the policy as a type parameter, the by-now-familiar spelling of a base class with one overridden method.

Chapter 25 introduced the ConfigDb in two lines and promised the rest. Time to pay: paths, wildcards, globals, precedence — configuration, in full.

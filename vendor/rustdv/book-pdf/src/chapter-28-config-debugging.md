# Chapter 28: Configuration Debugging

Chapter 27 showed configuration working. This chapter shows it failing, which is the more useful skill — both earlier books devoted a chapter to exactly this, because the config database's failure modes are quiet, remote from their causes, and rank among the UVM's most common support questions in every dialect. The database is addressed by strings resolved at run time, so the compiler cannot help. That is not a flaw to apologize for; it is the cost of late binding, stated plainly in Chapter 27 — and it is why the database ships with a debugger's toolkit. This chapter is that toolkit: an error that names its cause, a dump that shows the competition, and a tracer that films every operation.

> **In the UVM...** we learned the database's classic mistakes one painful demonstration at a time: a path that matched nothing, a key spelled two ways, a wildcard shadowing a specific setting, a parent and child fighting over one component. SystemVerilog's `get()` reports them all the same way — `return 0`, variable untouched — and both books taught `print_config(1)` and `+UVM_CONFIG_DB_TRACE` as the way out.

## Missing data

The lab animal is Chapter 27's logger, unchanged: it asks for `"MSG"` and propagates the error with `?`.

```rust
// Chapter 28, Figure 1: The logger that propagates the error
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
```

Now break it. The environment holds `loga` and `logb`; the test configures only one:

```rust
// Chapter 28, Figure 2: A message for only one of two loggers
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
```

`logb` finds nothing, its `?` fails the test, and the failure carries the name `config_not_found`. Look at the attribute: `expect_error = "config_not_found"` says this test passes *only if it fails that way*. This is sharper than a bare "expected to fail" flag — a test that failed for some other reason is still reported as a failure, and the report says what you got instead:

```text
MsgTest FAILED: expected error 'config_type_mismatch', got config_not_found: ...
```

(That line is real output, produced by deliberately changing the expectation to the wrong kind.) `expect_error` is how this book demonstrates failures without faking transcripts, and it is how you can pin down a testbench's error behavior in a regression.

The second classic:

```rust
// Chapter 28, Figure 3: Misspelling a key
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
```

Both loggers are now configured — except one value was stored under `MESG`. The failure is identical to figure 2's, because a key that was never written and a key written under another name are the same thing to the database. Nothing here is a compile error; `"MESG"` is a perfectly good string. This is the bug the toolkit exists for, and figures 5 through 7 will catch it.

## Handling a missing value

Sometimes "not found" is not a bug — a component with a sensible default should use it. The variant matters:

```rust
// Chapter 28, Figure 4: A logger that copes
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
```

Matching on the variant is the point. `NotFound` is recoverable — fall back and *say so*, with a warning that leaves a trail in the log. Any other failure is not: a type mismatch means the value is there and you asked for it wrongly, and defaulting past it would bury a real bug under a polite default. Chapter 27 explained why SystemVerilog cannot draw this line — its `get()` collapses every failure into `return 0` — and pyuvm's `except UVMConfigItemNotFound` draws it the same way this `match` does. The difference is the last arm: the compiler makes you decide what happens to the errors you did *not* name.

## Printing the database

```rust
// Chapter 28, Figure 5: Printing the ConfigDb
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
```

`end_of_elaboration` is the right home for the dump, and knowing why is knowing the lifecycle: the hierarchy is final and nothing has run yet, so what you see is exactly what the run phase will resolve against. (The same reasoning put `print_config` calls there in both source books.)

```rust
// Chapter 28, Figure 6: Debugging the misspelled key by printing
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
```

```text
# Figure 7: The dump puts MSG and MESG side by side

      0.00ns INFO     running NiceMsgTestAlmostFixed (4/7)  [ch28-config-debugging/src/ch28_config_debugging.rs:179]
      0.00ns INFO     PATH                        : KEY       : DATA
      0.00ns INFO     NiceMsgTestAlmostFixed.env.loga: MSG       : {1000: "LOG A msg"}
      0.00ns INFO     NiceMsgTestAlmostFixed.env.logb: MESG      : {1000: "LOG B msg"}
      0.00ns INFO     [NiceMsgTestAlmostFixed.env.loga]: LOG A msg
      0.00ns WARNING  [NiceMsgTestAlmostFixed.env.logb]: Could not find MSG. Setting to default
      0.00ns INFO     [NiceMsgTestAlmostFixed.env.logb]: No message for you!
      0.00ns INFO     NiceMsgTestAlmostFixed PASSED
```

The dump is what finds the figure-3 bug: two entries under `env.logb`-shaped paths, one keyed `MSG` and one keyed `MESG`, and the mismatch is visible in a way it never is at the point of failure. A misspelling is invisible in the place you wrote it and obvious in a table.

For contrast, the wildcard configuration from Chapter 27, working — worth running with the dump on simply to see what *healthy* looks like, since you will be reading these tables on bad days:

```rust
// Chapter 28, Figure 8: Wildcards behaving, for contrast
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
```

## Debugging a parent/child conflict

Chapter 27 ended with the parent winning the write to `env.loga` and a promise: here, you can watch the contest instead of memorizing its outcome.

```rust
// Chapter 28, Figure 9: Both the env and the test configure env.loga
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
```

```text
# Figure 10: The dump shows the competition, with precedences

PATH                        : KEY       : DATA
ConflictTest.env.loga       : MSG       : {1000: "PARENT RULES!", 999: "CHILD RULES!"}
```

This is where the dump earns its keep. A resolved value tells you who won; it does not tell you anyone else was competing. The dump lists *every* value stored at a path with the precedence each was written at — the losing entry is right there, and the numbers explain the outcome instead of asking you to trust Chapter 27's rule. The test wrote at depth 0, precedence 1000; the env at depth 1, precedence 999; shallower wins, and now you can see by how much.

## Tracing

The dump is a snapshot; tracing is the film.

```rust
// Chapter 28, Figure 11: Tracing every ConfigDb operation
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
```

```text
# Figure 12: Every set and get, as it happens

CFGDB/SET context=GlobalTest offset="env.loga" -> GlobalTest.env.loga MSG="LOG A msg"
CFGDB/SET context=<none> offset="*" -> * MSG="GLOBAL"
CFGDB/GET context=GlobalTest.env.gtalk offset="" -> GlobalTest.env.gtalk MSG="GLOBAL"
```

Turn it on before building the hierarchy and every operation logs with the context, the offset, and the path they resolved to. Read the trace's structure: each line shows the *inputs* and the resolution. When a lookup misses, the thing you got wrong is almost always the resolved path — a context you did not expect, an offset that anchored somewhere else — and the trace shows exactly the resolution the database performed, not the one you imagined. This is the port of `+UVM_CONFIG_DB_TRACE`, as a call rather than a plusarg, so a test can scope it to the region under suspicion.

## Summary

Late binding traded away compile-time checking; this chapter is what it bought instead. A failed `get` is a `Result` whose variants distinguish the recoverable miss (`NotFound` — default and warn) from the genuine bugs (a type mismatch is never something to default past), and `expect_error` turns a deliberate failure into a regression asset that fails if it fails any *other* way. `ConfigDb::print()` at `end_of_elaboration` shows the database as the run phase will see it — misspellings side by side, conflicts with the precedence numbers that decide them — and `ConfigDb::set_tracing(true)` films every set and get with the resolution that the point of failure never shows you.

The ConfigDb carries values to components that nobody passed them to. The factory, next, does the same for *types*: it builds components a test can substitute without touching the environment that asks for them.

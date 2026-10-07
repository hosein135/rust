# Chapter 27: Configuration

Chapter 25 used the ConfigDb twice and promised the full story later: the test did `ConfigDb::set`, the driver did `ConfigDb::get`, and the BFM crossed the testbench without ever appearing in a constructor. This chapter is the full story. The subject is one of verification's permanent problems — *a test must parameterize components buried in a hierarchy the test did not write* — and the UVM's answer to it, a path-addressed runtime database, ported whole.

> **In the UVM...** we stored values with `uvm_config_db#(string)::set(this, "env.loga", "MSG", ...)` in SystemVerilog or `ConfigDB().set(self, "env.loga", "MSG", ...)` in pyuvm — a context object, a path string, a key — and components retrieved them with `get()`. Wildcards (`"env.t*"`) configured groups at a stroke; a null context made globals; and when two writers hit the same path, a precedence rule decided.

Notice what that mechanism *is*: late binding, deliberately. The component that reads a value and the test that writes it never meet — not in a constructor, not in a call chain, nowhere the compiler can see. That is not a weakness the UVM's designers failed to engineer away. It is the feature: one environment, closed and finished, serves a hundred tests because the tests reach into it by *name* at run time. A configuration mechanism the compiler could fully check would be one whose decisions were already made at compile time — which is to say, not a configuration mechanism. rustdv keeps the runtime database, keeps the paths, keeps the wildcards, and spends its type system on making the *failures* loud. You will see that at the first `get`.

## Reading a value

The lab animal, as in the earlier books, is a `MsgLogger`: a component whose one behavior — the message it logs — comes from outside.

```rust
// Chapter 27, Figure 1: Logging a message we get from the ConfigDb

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

Notice first what this component does *not* have: a constructor argument, a field holding the message, any knowledge of who set it. It asks the database. The interesting line is the `get`, and it repays a slow read.

- **Read the three arguments as context, offset, field.** The store is ambient; the *identity* asking is passed in. The `""` is not a keyword meaning "me" — it is an empty *offset* from the context, which happens to land on the caller. A non-empty offset asks on behalf of another component:

  ```rust,ignore
  ConfigDb::get(Some(ctx), "",     "MSG")   // me
  ConfigDb::get(Some(ctx), "loga", "MSG")   // what loga will see
  ConfigDb::get(None,      "",     "SEQR")  // no context at all
  ```

  That third form matters more than it looks. A sequence is not a component and has no path, so when Chapter 36 needs one to find its sequencer, asking with no context is the only way it can ask. Both source books keep exactly this signature, and in *The UVM Primer*'s SystemVerilog the null-context form is the majority idiom.

- **The offset is relative for the same reason log paths are derived, not stored.** An absolute path is a hand-typed string that keeps compiling and starts lying the moment a component moves.

- **A value in the database must either implement `Clone`, so everyone gets their own copy, or ride in an `Rc`, so everyone gets a handle to one copy.** `MSG` is a `String` and a copy is what each logger wants. A sequencer is the opposite case: when Chapter 36 files one in the database, every sequence must reach the *same* sequencer, so it goes in as an `Rc`.

- **`get` returns a `Result`, and the `?` propagates it.** SystemVerilog's `get()` has four distinct ways to disappoint you — the value was never set, the path did not match, the field name was typo'd, the type parameter disagreed with the `set` — and collapses all four into `return 0` with your variable untouched. The failure mode is not the miss; it is that the miss is *silent*, and indistinguishable from a legitimate zero. rustdv's `get` names which of the four happened, and the `Result` is `#[must_use]`: you can propagate it with `?` or handle it, but you cannot quietly drop it. The lookup is still a runtime lookup — that is the design — and when it misses, everyone finds out.

One more thing worth saying about that SystemVerilog signature, because a typed-language reader will assume types would have prevented the mess: `uvm_config_db#(T)` *is* typed — heavily — and the type parameter is part of the lookup. What that bought is a bug class: `set` with `int`, `get` with `uvm_bitstream_t`, and the two calls never meet — a mismatch invisible in the code and silent at run time. pyuvm dropped the type parameter deliberately, and dropping it *removed* that failure mode outright. rustdv follows pyuvm: one key, no type in the address, and the type check happens at the single point of retrieval, loudly. This is the book's recurring lesson in miniature — where a type lives matters more than how many there are.

## Writing a value

The environment holds two loggers; the test configures each by path.

```rust
// Chapter 27, Figure 2: Two loggers in the environment
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
```

```rust
// Chapter 27, Figure 3: Giving loga and logb different messages
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
```

The paths are relative to the *setter*: `"env.loga"` resolves against the test's own position, exactly as `this` anchored the SystemVerilog `set`. And note the timing, because Chapter 24's argument is collecting its first payment: the test writes these values in its `build`, *before* `env` has built its children. Build runs top-down, so by the time `loga` exists and its `run` asks the database, the answer is waiting. The test reaches two components it never touches, through a mechanism the compiler never sees — which is precisely the job.

```text
# Figure 4: The loga and logb components have different things to say

      0.00ns INFO     running MsgTest (1/4)  [ch27-configuration/src/ch27_configuration.rs:99]
      0.00ns INFO     [MsgTest.env.loga]: LOG A msg
      0.00ns INFO     [MsgTest.env.logb]: LOG B msg
      0.00ns INFO     MsgTest PASSED
```

## Wildcards

pyuvm configured a family of components at a stroke with `ConfigDB().set(self, "env.t*", ...)`. rustdv keeps the glob. First, an environment with something worth matching:

```rust
// Chapter 27, Figure 5: Adding talka and talkb to the environment
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
```

The Python version made `MultiMsgEnv` by subclassing and `super().build_phase()`. Rust has no inheritance, and the difference between the two envs is *structural* — which children exist — so the env is written out. Four fields is cheaper to read than a mechanism for sharing two of them.

```rust
// Chapter 27, Figure 6: Using a wildcard to configure both talkers at once
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

`set` takes a glob; `get` takes a concrete path. pyuvm enforces the same asymmetry, and it is the right way round: you write to a *pattern* of components, but you always read as *one* component.

```text
# Figure 7: The "talk" components get the same message

      0.00ns INFO     running MultiMsgTest (2/4)  [ch27-configuration/src/ch27_configuration.rs:150]
      0.00ns INFO     [MultiMsgTest.env.loga]: LOG A msg
      0.00ns INFO     [MultiMsgTest.env.logb]: LOG B msg
      0.00ns INFO     [MultiMsgTest.env.talka]: TALK TALK
      0.00ns INFO     [MultiMsgTest.env.talkb]: TALK TALK
      0.00ns INFO     MultiMsgTest PASSED
```

## Global data

One more logger, `gtalk`, which nobody configures by name:

```rust
// Chapter 27, Figure 8: Adding gtalk, which nobody configures by name
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
```

```rust
// Chapter 27, Figure 9: Storing a message for everybody
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
```

The last line is the port of pyuvm's `ConfigDB().set(None, ...)` and SystemVerilog's `set(null, ...)`: with no context to offset from, the glob is absolute, and `"*"` matches every path. It is the fallback for any component no more specific rule names — here, `gtalk`.

Which raises the obvious question: `loga`'s path matches `env.loga`, `env.t*` does not match it, but `*` does — so which value does `loga` get? **Resolution is most-specific-first**: `env.loga` beats `env.t*` beats `*`. That is pyuvm's rule, and rustdv adopts it. SystemVerilog readers should note their UVM does *not* sort by specificity — it gathers every match and takes the one with the highest precedence, which is why a stray global in an SV testbench can shadow a specific setting in ways that surprise people. Chapter 28 is about seeing what actually resolved, for exactly such moments.

```text
# Figure 10: The default is matched only where nothing overrides it

      0.00ns INFO     running GlobalTest (3/4)  [ch27-configuration/src/ch27_configuration.rs:207]
      0.00ns INFO     [GlobalTest.env.loga]: LOG A msg
      0.00ns INFO     [GlobalTest.env.logb]: LOG B msg
      0.00ns INFO     [GlobalTest.env.talka]: TALK TALK
      0.00ns INFO     [GlobalTest.env.talkb]: TALK TALK
      0.00ns INFO     [GlobalTest.env.gtalk]: GLOBAL
      0.00ns INFO     GlobalTest PASSED
```

## The parent/child conflict

The database's most instructive scenario. The env configures its own child; the test configures the same component by a longer path; both writes land on exactly the same component. Which message prints?

```rust
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
```

```rust
// Chapter 27, Figure 12: ...and the test configures the same component
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
```

**The parent wins** — and it is worth understanding why, because the rule is not arbitrary and it is not "first write wins." Under a naive last-write-wins, the parent would *lose*: build is top-down, so an ancestor always writes before its descendants. If recency decided, every child could silently overrule the test that instantiated it, and configuring a testbench from the top — the entire point of the mechanism — would be impossible. So a build-phase write carries a precedence that *decreases with the setter's depth*: the shallower the writer, the stronger the write, regardless of order. The test outranks the env; `PARENT RULES!` prints. (A write made after build, from a run phase, carries full precedence and outranks every build-time write — by then, whoever is writing is doing so on purpose.) This is the same rule the UVM applies and the same reasoning behind it; the difference is Chapter 28's, where you can ask the database to *show you* the contest instead of memorizing its outcome.

```text
# Figure 13: The parent wins

      0.00ns INFO     running ConflictTest (4/4)  [ch27-configuration/src/ch27_configuration.rs:256]
      0.00ns INFO     [ConflictTest.env.loga]: PARENT RULES!
      0.00ns INFO     ConflictTest PASSED
```

## Summary

The problem — tests parameterizing components they never touch — is permanent, and rustdv answers it the way all three UVMs do: a runtime database addressed by hierarchical path. `set` takes context, glob, and field; `get` takes context, offset, and field, and returns a `Result` that names which of the four possible misses happened — the one place this chapter spends types, because the lookup itself is late binding and is supposed to be. Values are `Clone`d in or shared by `Rc`. Wildcards write to patterns; reads are always concrete; resolution is most-specific-first; and in the parent/child conflict the shallower writer wins so that configuration from the top stays possible.

pyuvm needed a second chapter to teach *debugging* the ConfigDB, and honesty requires the same here: a runtime lookup can miss, and the next chapter is the debugger's toolkit that comes with it — printing the database, tracing its decisions, and testing the failure paths on purpose.

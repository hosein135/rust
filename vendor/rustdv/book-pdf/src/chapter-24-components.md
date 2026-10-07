# Chapter 24: Components

Testbench 3.0 gave us a UVM test, and the test was the only component in it: the BFM and the scoreboard were ordinary local values inside its `run`. That works at TinyALU scale and stops working shortly after — a real testbench is a *tree* of single-purpose components, each with its own job, sharing one lifecycle so that everything gets built, wired, run, and checked in a dependable order. The class that provides all of that in the UVM is `uvm_component`. This chapter ports it.

> **In the UVM...** every testbench class extends `uvm_component`, whose phase methods the framework calls in a fixed order — `build`, `connect`, `end_of_elaboration`, `start_of_simulation`, `run` (the only task, objection-gated), `extract`, `check`, `report`, `final`. We built the hierarchy in `build_phase()` by instantiating children with a *name* and a *parent* — `mc = middle_comp::type_id::create("mc", this)` in SystemVerilog, `self.mc = MiddleComp("mc", self)` in pyuvm — and the framework wove the references into a tree with paths like `uvm_test_top.mc.bc`.

## The nine phases

Figure 1 is a test that overrides every phase method just to prove the order. It is a struct test, as in Chapter 23 — and that choice now pays off, because a struct test *is a component*: `#[rustdv::test]` registers it, and the runner drives its phases exactly the way `@pyuvm.test()` hands a class to pyuvm's phaser.

```rust
// Chapter 24, Figure 1: A uvm_test demonstrating the phase methods

#[rustdv::test]
#[derive(Component, Default)]
struct PhaseTest;

impl Component for PhaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("1 build");
    }
    fn connect(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("2 connect");
    }
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("3 end_of_elaboration");
    }
    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("4 start_of_simulation");
    }
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("run");
        ctx.info("5 run");
        Ok(())
    }
    fn extract(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("6 extract");
    }
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let _ = errors;
        ctx.info("7 check");
    }
    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("8 report");
    }
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("9 final");
    }
}
```

The pieces, in source order:

- `#[derive(Component)]` — the derive from Chapter 21. It writes the tree-traversal plumbing so the phaser can walk this component's children. `PhaseTest` has none, so the derive's work here is small; it earns its keep in figure 4.
- `impl Component for PhaseTest` — the `Component` trait carries all nine phase methods, every one with a default no-op body. A component overrides only the phases it uses; this one overrides all nine only because the order is the demonstration.
- `ctx: &mut RustdvCtx` — every phase receives the context, and its log lines are stamped with the path the phase walk derived. No phase method takes a name; no component stores one.
- `async fn run` — the one phase that takes simulated time, so the one that is `async`. It raises an objection the moment it starts and holds it as a guard: the run phase ends when every guard in the testbench has dropped. Dropping happens here at the end of the function, the way any Rust value drops.
- `fn final_phase`, not `fn final` — `final` is a Rust keyword, so this is the one phase whose rustdv name differs by necessity.

Figure 2 is the run.

```text
# Figure 2: The lifecycle runs in order

      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running PhaseTest (1/2)  [ch24-components/src/ch24_components.rs:38]
      0.00ns INFO     [PhaseTest]: 1 build
      0.00ns INFO     [PhaseTest]: 2 connect
      0.00ns INFO     [PhaseTest]: 3 end_of_elaboration
      0.00ns INFO     [PhaseTest]: 4 start_of_simulation
      0.00ns INFO     [PhaseTest]: 5 run
      0.00ns INFO     [PhaseTest]: 6 extract
      0.00ns INFO     [PhaseTest]: 7 check
      0.00ns INFO     [PhaseTest]: 8 report
      0.00ns INFO     [PhaseTest]: 9 final
      0.00ns INFO     PhaseTest PASSED
      ... (TestTop, the second test in this crate, follows)
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** PhaseTest                                    PASS           0.00      **
** TestTop                                      PASS           0.00      **
******************************************************************************
REGRESSION: PASS
```

Nine phases, in the UVM's order, driven by the framework — nobody in the listing called any of them. The `[PhaseTest]` between the brackets is the component's path, derived by the walk rather than stored anywhere.

One divergence to note now, because it matters to SystemVerilog readers checking this against muscle memory: rustdv follows *pyuvm's* traversal directions, not the SystemVerilog UVM's. `build` runs top-down and `connect` bottom-up in all three frameworks, but `end_of_elaboration`, `start_of_simulation`, `extract`, `check`, and `report` run top-down here, where the SystemVerilog UVM runs them bottom-up. If your testbench depends on a child's `report` running before its parent's, that assumption does not carry over.

## Why build and connect exist

A fair question from a Rust point of view: a struct's constructor can build its children, so why have a `build` phase at all? Build the children in `new()`, take the connections as constructor arguments, and the framework gets simpler — the design almost writes itself.

It is also wrong, and the reason it is wrong is the most important paragraph in this chapter. The gap between *a component existing* and *its children existing* is not dead time to be optimized away — it is where every late-binding mechanism in the UVM lives. Configuration must be able to reach a component *before* it decides what children to make: that is how one environment builds an active agent in one test and a passive one in another (Chapter 25). The factory must be able to substitute a child's type *before* the child is constructed: that is what a factory override is (Chapter 29). And connection must happen *after* everything below exists: that is why `connect` runs bottom-up (Chapter 31). Fold building into constructors and all three mechanisms lose the moment they operate in. The UVM's designers had typed classes, parameters, and constructors in hand and still built a two-stage lifecycle — three frameworks in three languages kept it — because deferring those decisions is the point, not an accident of class-based construction.

So in rustdv, `build` and `connect` are real phase methods, and the directions are load-bearing: `build` runs top-down so a parent decides what to create before its children exist, and `connect` runs bottom-up so wiring happens over a finished subtree.

## Growing the tree

Figure 3 is the hierarchy this section builds — the same three-level tower the earlier books used.

```text
# Figure 3: The three-level hierarchy

    TestTop                (a test — the root)
       └── mc: MiddleComp
              └── bc: BottomComp
```

Each parent creates its child *in its own build phase*, and the phaser descends into whatever `build` created, so the tree grows top-down as it is walked. Figures 4 through 6 are the three components, from the top down.

```rust
// Chapter 24, Figure 4: the test at the top. Its build phase constructs the
// middle component; the phaser descends into the tree that build creates.
#[rustdv::test]
#[derive(Component, Default)]
struct TestTop {
    #[component]
    mc: Option<MiddleComp>,
}

impl Component for TestTop {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("build phase");
        self.mc = Some(MiddleComp::default());
    }
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("final phase");
    }
}
```

Three lines carry the design:

- `#[component]` — this attribute tells the derive which fields are children. The phase walk visits exactly the marked fields, in declaration order.
- `mc: Option<MiddleComp>` — the child is declared as an `Option` because before `build` runs there *is no child*. `None` is the type-level spelling of "declared but not yet built" — the state every UVM component is in between its own construction and its `build_phase`. The struct definition names what the tree can hold; `build` decides what it does hold.
- `self.mc = Some(MiddleComp::default())` — building the child is an assignment. Compare `self.mc = MiddleComp("mc", self)`: no name string, because the field is named `mc` and the walk derives the path; no parent handle, because ownership already says whose field this is.

```rust
// Chapter 24, Figure 5: the middle component builds the bottom component in
// its own build phase, and announces itself at end of elaboration.
#[derive(Component, Default)]
struct MiddleComp {
    #[component]
    bc: Option<BottomComp>,
}

impl Component for MiddleComp {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.bc = Some(BottomComp::default());
    }
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("end of elaboration phase");
    }
}
```

`MiddleComp` is not a test — no `#[rustdv::test]` — just a component that both is built and builds. When the top-down walk reaches it, its `build` runs and `bc` comes into existence; the walk then descends into `bc`. Top-down construction, exactly as `build_phase` has always worked.

```rust
// Chapter 24, Figure 6: the bottom component. Only a run phase, which
// objects, logs under its path (uvm_test_top.mc.bc in UVM; TestTop.mc.bc
// here), and drops.
#[derive(Component, Default)]
struct BottomComp;

impl Component for BottomComp {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("bc run");
        ctx.info("run phase");
        Ok(())
    }
}
```

`BottomComp` overrides one phase and is the pattern for every leaf that does work: raise the objection, do the job, and let the guard drop when the function ends. Every component's `run` gets this same deal — each raises its own objection for its own work, and the run phase of the whole testbench ends when the last guard anywhere has dropped. What the objection buys becomes vivid in Chapter 31, where a parent and its children run *at the same time* and components that never finish on their own — monitors, responders — stop exactly when the objecting components are done.

Figure 7 is the run.

```text
# Figure 7: The walk derives every path

      0.00ns INFO     running TestTop (2/2)  [ch24-components/src/ch24_components.rs:118]
      0.00ns INFO     [TestTop]: build phase
      0.00ns INFO     [TestTop.mc]: end of elaboration phase
      0.00ns INFO     [TestTop.mc.bc]: run phase
      0.00ns INFO     [TestTop]: final phase
      0.00ns INFO     TestTop PASSED
```

Four log lines, four different phase methods, three different components — and each line carries the right path. `[TestTop.mc.bc]` is the path the UVM would spell `uvm_test_top.mc.bc`, synthesized from field names by the walk as it descends: `TestTop.build` created `mc`, the phaser recursed, `mc.build` created `bc`, and each component logged under the path the traversal accumulated. Nothing stored a path and nobody typed one. Move a component to a different place in the tree and its path follows, because there is no string anywhere that could go stale.

## What this costs

Two prices, stated plainly.

**Phase discipline is not checked at compile time.** There is one context type, `RustdvCtx`, and every phase receives it whole — the compiler does not know that raising an objection makes no sense in `build`, or that a value configured during `run` is too late for a `build` that already ran. An operation performed in the wrong phase is a run-time failure with a good message, exactly as it is in every UVM. A family of per-phase context types could push some of this to compile time; it would also mean eight signatures for every helper that takes a context, and rustdv declines the trade: the lifecycle is runtime machinery, and the type system is not pretending otherwise.

**`async fn` in a trait is a live edge of Rust.** `Component::run` is an `async fn` in a trait, and Rust has not finished smoothing that feature: a trait with an `async fn` cannot be made into a `dyn` trait object directly. The framework deals with it by keeping a dyn-safe mirror of the trait internally — machinery you never see and never write, which is why no listing in this book mentions it. A framework author feels that edge so that a testbench author does not.

## Summary

`uvm_component` ported whole. The `Component` trait carries the nine phases with default no-op bodies — override what you use — and the runner drives them in pyuvm's order and directions: `build` top-down, `connect` bottom-up, run objection-gated, the elaboration and post-run phases top-down (a divergence from the SystemVerilog UVM's bottom-up, worth checking against old habits). A child is a struct field, `Option`-wrapped because it does not exist until its parent's `build` creates it; the phase walk descends into what `build` made, deriving every component's path from field names as it goes. Build and connect are real phases because the gap they occupy — after a component exists, before its children do — is where configuration, factory overrides, and connection all operate; Chapters 25, 29, and 31 each collect on that argument in turn.

Version 4.0 is next: the machinery of this chapter, put to work on the TinyALU — an environment component, a scoreboard that checks in `check`, and the BFM delivered through the ConfigDb instead of reached for.

# Chapter 23: uvm_test Testbench: 3.0

Testbench 2.0 was modular; now we make it methodological. As is tradition, we start writing UVM tests by creating a `HelloWorldTest`, to examine the mechanics of defining and running a test before wiring one to the TinyALU.

> **In the UVM...** the test was a class extending `uvm_test`, with an objection-guarded `run_phase()` that raised, said hello, and dropped — `class hello_world extends uvm_test` selected by `run_test()` in SystemVerilog; `class HelloWorldTest(uvm_test)` marked `@pyuvm.test()` in Python — and the framework instantiated it under the name `uvm_test_top` and drove its phases. Then testbench 3.0 refactored 2.0: `BaseTest` carried the shared `run_phase()`, while `RandomTest` and `MaxTest` overrode `build_phase()` to pick a tester.

## Hello, world, in the methodology

```rust
// Chapter 23, Figure 1: The basic rustdv-UVM use model in hello_world

#[rustdv::test]
#[derive(Component, Default)]
struct HelloWorldTest;

impl Component for HelloWorldTest {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("saying hello");
        ctx.info("Hello, world.");
        Ok(())
    } // the guard drops here: the objection is released
}
```

Here is the minimum needed to define and run a rustdv UVM test:

- `#[rustdv::test]` — the attribute you have used since Chapter 15, now on a *struct*. It registers the test with the runner under its type name, verbatim — `HelloWorldTest`, not `hello_world_test` — so the runner can select it by the name you see in the source. This is the UVM's `run_test()`: the framework instantiates your test and drives it.
- `#[derive(Component)]` — the test *is a component*, exactly as `uvm_test` extends `uvm_component`. Everything a component can do, a test can do, and Chapter 24 leans on that hard.
- `impl Component for HelloWorldTest` — the test overrides the one phase it uses, `run`. It has no children, so no `build`; the trait's defaults cover every phase you don't write.
- `ctx.raise_objection("saying hello")` — the UVM's objection, as a guard. The run phase continues until every objection is released, and releasing happens by *dropping the guard* — here, at the closing brace. Note what that deletes: the forgot-to-drop bug, which hangs a pyuvm run phase until the timeout fires, is unwritable. Scope ends, objection drops. (Chapter 16's `LockGuard`, Chapter 13's RAII — third verse.)
- `ctx.info("Hello, world.")` — and this is the first place `ctx.info` earns its keep over the bare `log::info` of Part II, because the context knows *who is talking*:

```text
# Figure 2: Hello, world!

      0.00ns INFO     rustdv: found 3 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running HelloWorldTest (1/3)  [ch23-uvm-test-testbench-3.0/src/ch23_uvm_test_testbench_3_0.rs:149]
      0.00ns INFO     [HelloWorldTest]: Hello, world.
      0.00ns INFO     HelloWorldTest PASSED
```

The `[HelloWorldTest]` between the brackets is the component's path in the testbench hierarchy — a hierarchy that is, so far, one component deep. One divergence from tradition worth a sentence: the UVM names the root `uvm_test_top` no matter which test is running, and rustdv names it after the test you registered. When something three components deep logs a message in Chapter 26, its path will start with the name of the test that built it, which tells you at a glance which test's universe the message came from.

A note for readers keeping score against Chapter 15: `#[rustdv::test]` accepts *two* shapes, and both are first-class. On a free `async fn`, it is cocotb's model — `@cocotb.test()` on a coroutine — and every test in Part II was one. On a struct implementing `Component`, it is pyuvm's model — `@pyuvm.test()` on a class — and it is what a test that owns a component tree needs to be. The function form is not training wheels; a function-shaped test remains the right spelling for a function-shaped job. This book's testbenches are about to grow trees, so from here on the struct form carries the story.

## Where the tower went

Both earlier books paused here for the UML tower every UVM engineer has climbed. It is worth reprinting, with each floor's rustdv forwarding address:

```text
# Figure 3: The uvm_test tower, and its rustdv equivalent

pyuvm                          rustdv
-----                          ------
uvm_void                       (no common ancestor needed; registration
                                rides the derive — Ch. 29)
uvm_object                     plain structs + derives (Ch. 35)
uvm_report_object              ctx.info() — logging rides the context (Ch. 26)
uvm_component                  the Component trait (Ch. 24)
uvm_test                       #[rustdv::test] on a struct
```

The tower's *jobs* all survive; the inheritance chain that delivered them does not, because Rust composes capabilities instead of stacking them. What `uvm_object` gave you arrives as derives on your transaction structs; what `uvm_report_object` gave you rides in on `ctx`; what `uvm_component` gave you is the `Component` trait; and `uvm_test` — the class whose only real job was "this is the one the framework starts" — is the attribute.

## Refactoring testbench 2.0

Testbench 2.0's classes — the `Tester` trait, `RandomTester`, `MaxTester`, and the `Scoreboard` — return in this chapter's example file under a banner comment reading *copied from testbench 2.0*, and that convention deserves a sentence because the book will use it from here to the end. Chapter examples repeat the classes they use rather than importing them, exactly as the earlier books re-showed code, because these classes *evolve*: the tester is a plain trait object today and a component in Chapter 25, the scoreboard checks in a function today and in a `check` phase tomorrow. Watching them change is the point, and an import would hide the change. (The definitions are unchanged from Chapter 20's figures 1 through 8; we will not re-read them here.)

What 3.0 actually changes is who runs them. pyuvm expressed base-and-variants as `BaseTest` providing `run_phase`, extended by `RandomTest` and `MaxTest` overriding `build_phase`. Rust has no inheritance, so the shared body is a *function*, and each test hands it a tester:

```rust
// Chapter 23, Figure 4: alu_test — the shared run phase of every ALU test

async fn alu_test(ctx: &mut RustdvCtx, tester: &mut impl Tester) -> Result<(), TestError> {
    let _obj = ctx.raise_objection("alu_test stimulus");

    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    let mut scoreboard = Scoreboard::new(bfm.clone());

    bfm.reset().await;
    bfm.start_tasks();
    scoreboard.start_tasks();

    tester.execute(&bfm).await;

    if scoreboard.check_results() {
        Ok(())
    } else {
        Err(TestError::from("scoreboard saw failing comparisons"))
    }
}
```

This is Chapter 20's `execute_test`, promoted to a run phase. Three things to notice, and one to notice by its absence:

- **The test is the only component.** The BFM and the scoreboard are ordinary local values inside `run`; the tester is a plain value passed in. No tree, no phases for them — that is Chapters 24 and 25's business, and this chapter refuses to get ahead of itself.
- The BFM comes from `ctx.dut()` — the same handle-then-check pattern as Chapter 19, with `?` propagating a missing signal as a named failure.
- The objection guards the whole body: stimulus, then checking, then `Ok` or a `TestError` that fails the test with the scoreboard's verdict.
- And no clock-starting line, because there is none to start: the RTL self-clocks, and the BFM only ever waits on edges — Chapter 19's design rule, still paying rent.

pyuvm kept `BaseTest` abstract by convention — nothing but discipline stopped a teammate from running it, since its only protection was the missing decorator. `alu_test` is abstract by *signature*: a function that requires a `&mut impl Tester` argument cannot be registered as a test, because there is nothing to fill the argument with. The two real tests are exactly as thin as pyuvm's:

```rust
// Chapter 23, Figure 5: The tests choose a tester and share alu_test

#[rustdv::test]
#[derive(Component, Default)]
struct RandomTest;

impl Component for RandomTest {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let mut tester = RandomTester { rng: ctx.rng() };
        alu_test(ctx, &mut tester).await
    }
}

#[rustdv::test]
#[derive(Component, Default)]
struct MaxTest;

impl Component for MaxTest {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let mut tester = MaxTester;
        alu_test(ctx, &mut tester).await
    }
}
```

Where pyuvm's `RandomTest.build_phase()` set `self.tester = RandomTester()`, our `RandomTest::run` builds a `RandomTester` and passes it in — and note the seed's route: `ctx.rng()` hands the tester the per-test seeded generator, so a failing run reproduces. The variation point is an argument today. In Chapter 25 it becomes a `build`-phase decision, and in Chapter 30 a factory slot; the shape — shared body, per-test variation at a designed point — is the methodology, and it never changes again.

```text
# Figure 6: RandomTest passes

      0.00ns INFO     running RandomTest (2/3)  [ch23-uvm-test-testbench-3.0/src/ch23_uvm_test_testbench_3_0.rs:188]
    150.00ns INFO     PASSED: ce Add 42 = 0110
    150.00ns INFO     PASSED: 2f And 64 = 0024
    150.00ns INFO     PASSED: 29 Xor b3 = 009a
    150.00ns INFO     PASSED: 86 Mul 83 = 4492
    150.00ns INFO     Covered all operations
    150.00ns INFO     RandomTest PASSED
```

```text
# Figure 7: MaxTest maxes all the operands

    150.00ns INFO     running MaxTest (3/3)  [ch23-uvm-test-testbench-3.0/src/ch23_uvm_test_testbench_3_0.rs:199]
    300.00ns INFO     PASSED: ff Add ff = 01fe
    300.00ns INFO     PASSED: ff And ff = 00ff
    300.00ns INFO     PASSED: ff Xor ff = 0000
    300.00ns INFO     PASSED: ff Mul ff = fe01
    300.00ns INFO     Covered all operations
    300.00ns INFO     MaxTest PASSED
```

Same behavior as testbench 2.0 — and one detail in these transcripts quietly measures how far the testbench has to go. The `PASSED` lines carry no `[path]`, because the scoreboard prints them with plain `log::info`: it is not a component, so it has no path to be stamped with. Compare `[HelloWorldTest]` in figure 2. When the scoreboard becomes a component in Chapter 25, its lines pick up their address, and you will be able to read a log line's provenance without grepping for its format string.

## Summary

Testbench 3.0 brings the UVM's test discipline to rustdv. `#[rustdv::test]` on a struct is `@pyuvm.test()` on a class: the test is a component, registered under its type name verbatim, instantiated by the runner, its phases driven for it, its path named after itself rather than `uvm_test_top`. Objections are RAII guards — raise returns a guard, scope-exit drops it, and the forgot-to-drop hang cannot be written. The base-class pattern crossed the no-inheritance gap as a shared `async fn` taking `&mut impl Tester`, abstract by signature rather than by convention, with each test choosing its tester and its seed source in three lines.

At 3.0 the test is the only component in the testbench, and everything it uses is a local. The next chapter grows the tree: `uvm_component`, the nine phases, and the answer to how a testbench gets *structure*.

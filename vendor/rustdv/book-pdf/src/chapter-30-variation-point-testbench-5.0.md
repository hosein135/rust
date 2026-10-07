# Chapter 30: Variation-Point Testbench: 5.0

Testbench 4.0 had a flaw both earlier books flagged the moment it shipped: two tests needed two environments — `AluEnv<RandomOperands>` and `AluEnv<MaxOperands>` in our version, `RandomEnv` and `MaxEnv` before — even though the environments differed in exactly one component. Version 5.0 fixes it the way the factory always promised: **one environment**, with the difference carried in from the tests, through the machinery Chapter 29 just built.

> **In the UVM...** we kept one `AluEnv` that created its tester through the factory — `base_tester::type_id::create("tester", this)` in SystemVerilog, `BaseTester.create("tester", self)` in pyuvm — and each test registered an override in `build_phase`: `set_type_override_by_type(BaseTester, RandomTester)`. Three lines that changed what the env built without the env knowing.

Before the code, the design question this version answers. Testbench 4.0's `AluEnv<T>` chose its tester with a *type parameter* — a compile-time decision, which meant `RandomEnv` and `MaxEnv` were **different types**. That is the right tool when the variation is fixed at build time. But the whole point of a variation point is that a *test* chooses at *run* time — and a factory override cannot reach a type parameter: by the time any code runs, `AluEnv<RandomOperands>` simply *is* what it is, monomorphized and sealed. Runtime choice needs a runtime slot. So 5.0's env is one concrete type with a `create_comp()` line where the type parameter used to be, and the generic form survives for what it is good at: variation chosen at compile time, like Chapter 26's logging policies. Know which kind of variation you have, and you know which tool to reach for.

## The testers

The testers are the ones you have had since testbench 2.0. Chapter 20 wrote them as a trait with one required method and one provided one, which is how Rust says "abstract base class with a single overridden method":

```rust
// Chapter 30, Figure 1: The Tester trait — one method varies, the rest is shared
trait Tester {
    fn get_operands(&mut self) -> (u8, u8);

    async fn execute(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("tester stimulus");
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;

        bfm.reset().await;

        for op in Ops::ALL {
            let (aa, bb) = self.get_operands();
            bfm.send_op(aa, bb, op).await;
        }
        // send two dummy operations to allow
        // the last real operation to complete
        bfm.send_op(0, 0, Ops::Add).await;
        bfm.send_op(0, 0, Ops::Add).await;
        Ok(())
    }
}
```

Only the opening has changed since Chapter 20. A tester is now a component, so `execute` takes the context rather than a BFM handle: it raises the objection that holds the run phase open, asks the ConfigDb for the BFM nobody handed it, and does its own reset — setup that Chapter 20's `execute_test` handled before calling it. `get_operands` is untouched, and it is still the only thing a tester has to write.

```rust
// Chapter 30, Figure 2: The abstract base and the two testers that fill its slot
#[derive(Component, Default)]
struct BaseTester;

impl Component for BaseTester {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        panic!("BaseTester is abstract — override it with RandomTester or MaxTester");
    }
}

#[derive(Component, Default)]
struct RandomTester {
    rng: Option<Rng>,
}

impl Tester for RandomTester {
    fn get_operands(&mut self) -> (u8, u8) {
        let rng = self.rng.as_mut().expect("build phase did not run");
        (rng.u8(), rng.u8())
    }
}

impl Component for RandomTester {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.rng = Some(ctx.rng());
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        self.execute(ctx).await
    }
}

#[derive(Component, Default)]
struct MaxTester;

impl Tester for MaxTester {
    fn get_operands(&mut self) -> (u8, u8) {
        (0xFF, 0xFF)
    }
}

impl Component for MaxTester {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        self.execute(ctx).await
    }
}
```

Each tester now wears two traits. `Tester` gives it stimulus, exactly as before; `Component` gives it phases; and `run` is the one line that joins them — the phaser calls `run`, `run` calls `execute`, and `execute` calls back into `get_operands`. `RandomTester` picks up its seeded `Rng` in `build` because a component is created by its parent with nothing passed in.

`BaseTester` is the type the environment names and the factory overrides — the analog of the Python book's abstract `BaseTester`, which raises an error if run un-overridden. Here that is a `panic!`: a test that forgets its override builds a `BaseTester`, and running one *is* the bug, reported in its own words. It implements `Component` but not `Tester`, because there is no stimulus it could sensibly run. The derive registers all three types, so any of them can stand in the tester slot.

## The environment

The scoreboard is testbench 4.0's, re-shown in the chapter's file and deliberately untouched — the reader is meant to see it unchanged while the tester's *selection* changes around it. The env is where 5.0 differs:

```rust
// Chapter 30, Figure 3: The environment builds its tester through the factory
#[derive(Component, Default)]
struct AluEnv {
    #[component]
    scoreboard: RustdvComp,
    #[component]
    tester: RustdvComp,
}

impl Component for AluEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.scoreboard = Scoreboard::new_comp();
        self.tester = BaseTester::create_comp();
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

Two build lines, and they encode the block author's whole policy. The scoreboard is `new_comp()` — fixed, not a variation point, no test may swap it. The tester is `create_comp()` — the one slot a reuser may fill differently. Same field type on both (`RustdvComp` says nothing about overridability); the build line carries the decision, exactly as Chapter 29 taught. Note also that the env is what starts the BFM's tasks — once, for the whole environment, matching the Python book's `AluEnv`.

## The tests

```rust
// Chapter 30, Figure 4: random_test overrides BaseTester with RandomTester
#[rustdv::test]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    env: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        Factory::set_type_override::<BaseTester, RandomTester>();
        self.env = AluEnv::new_comp();
    }
}
```

```rust
// Chapter 30, Figure 5: max_test differs only in the tester it installs
#[rustdv::test]
#[derive(Component, Default)]
struct MaxTest {
    #[component]
    env: RustdvComp,
}

impl Component for MaxTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        Factory::set_type_override::<BaseTester, MaxTester>();
        self.env = AluEnv::new_comp();
    }
}
```

The build-order guarantee from Chapters 24 and 29, working: the test's `build` installs the override before the walk descends into `env`, so by the time the factory resolves `env.tester`, the substitution is in force. The env is not edited between the two tests, and does not know which tester it got.

```text
# Figure 6: One env, two behaviors

      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [ch30-variation-point-testbench-5.0/src/ch30_variation_point_testbench_5_0.rs:245]
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    150.00ns INFO     [RandomTest.env.scoreboard]: Covered all operations
    150.00ns INFO     RandomTest PASSED
    150.00ns INFO     running MaxTest (2/2)  [ch30-variation-point-testbench-5.0/src/ch30_variation_point_testbench_5_0.rs:262]
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff And ff = 00ff
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    300.00ns INFO     [MaxTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    300.00ns INFO     [MaxTest.env.scoreboard]: Covered all operations
    300.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** RandomTest                                   PASS         150.00      **
** MaxTest                                      PASS         150.00      **
******************************************************************************
REGRESSION: PASS
```

Compare against Chapter 25's transcript: the operands and results are *identical*, bit for bit, same seed. The same stimulus, selected by a runtime factory override instead of a compile-time type parameter — which is the entire chapter, demonstrated by two transcripts agreeing.

## Summary

Testbench 5.0 replaces 4.0's type-parameter variation with a factory slot: one concrete `AluEnv` whose tester is built with `create_comp()`, an abstract `BaseTester` whose run is a self-describing `panic!`, and tests that differ only in the override they install before building the env. The dividing rule is worth keeping: a type parameter serves variation chosen at compile time; a `create_comp()` slot serves variation a test chooses at run time, because an override cannot reach into a monomorphized type. The scoreboard, deliberately untouched, is the control group.

The env's components still share data the pre-UVM way, though — everything funnels through the BFM's queues, and the scoreboard hogs them. Giving components a standard way to talk to *each other* is TLM, and it is next.

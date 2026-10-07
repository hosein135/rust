# Chapter 25: uvm_env Testbench: 4.0

Chapter 24 built the machinery; testbench 4.0 moves in. At 3.0 the test was the only component, with the tester and scoreboard as ordinary locals inside its `run`. This version makes each of them a real component with its own phases, and gathers them into an **environment** — the container that keeps a tester and its scoreboard together, and the reusable unit the UVM is built around. The earlier books did this in three steps and so do we: componentize the testers and the scoreboard, instantiate them in an environment, instantiate the environment in tests.

> **In the UVM...** we made `BaseTester` a `uvm_component` that drove stimulus in an objection-guarded `run_phase()`; the `Scoreboard` launched its gathering tasks in `start_of_simulation_phase()` and compared in `check_phase()`; `BaseEnv` built the scoreboard, `RandomEnv`/`MaxEnv` added the right tester; and `RandomTest`/`MaxTest` did nothing but build the right env. The BFM reached everyone as a virtual interface through the config database — `uvm_config_db#(virtual tinyalu_bfm)::set(null, "*", "bfm", bfm)` in every `top.sv`.

## Converting the testers to components

Chapter 20's testers varied in exactly one behavior — how they choose operands. That stays true as they become components:

```rust
// Chapter 25, Figure 1: What varies between testers is only the operands
pub trait Operands {
    fn get_operands(&mut self, rng: &mut Rng) -> (u8, u8);
}
```

```rust
// Chapter 25, Figure 2: RandomOperands and MaxOperands
#[derive(Default)]
pub struct RandomOperands;

impl Operands for RandomOperands {
    fn get_operands(&mut self, rng: &mut Rng) -> (u8, u8) {
        (rng.u8(), rng.u8())
    }
}

#[derive(Default)]
pub struct MaxOperands;

impl Operands for MaxOperands {
    fn get_operands(&mut self, _rng: &mut Rng) -> (u8, u8) {
        (0xFF, 0xFF)
    }
}
```

SystemVerilog and Python express "same tester, different operands" with an abstract base class and one overridden method. Rust expresses it as a trait with one required method — and then, in the next figure, as a *type parameter* on the component that uses it:

```rust
// Chapter 25, Figure 3: BaseTester implements the phases common to all testers
#[derive(Component, Default)]
pub struct BaseTester<T: Operands + Default + 'static> {
    operands: T,
    rng: Option<Rng>,
}

impl<T: Operands + Default + 'static> Component for BaseTester<T> {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        // The seeded RNG comes from the context, so a run is reproducible.
        self.rng = Some(ctx.rng());
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("tester stimulus");
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        let rng = self.rng.as_mut().expect("build phase did not run");

        bfm.reset().await;

        for op in Ops::ALL {
            let (aa, bb) = self.operands.get_operands(rng);
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

Stop at the `ConfigDb` lines, because they are this chapter's other new idea and the code cannot be read without them. The tester is created by its parent's `build` phase, and `build` passes nothing — no name, no parent, and no BFM. A component that needs something it was not handed *asks for it by name*, and something above it in the tree put it there. Two lines are enough to read every listing in this chapter:

```rust,ignore
ConfigDb::set(None, "*", "BFM", bfm)   // the test: everyone gets this one
ConfigDb::get(Some(ctx), "", "BFM")    // a component: give me mine
```

Chapter 27 is the full treatment — paths, wildcards, precedence, and what happens when the name is wrong. Until then, three observations carry you. First, this is exactly how SystemVerilog delivers the virtual interface, for exactly the reason: siblings built by a parent cannot be handed things through constructors, so a named store above the tree does it. Second, the `get` returns a `Result` — `?` in a phase that returns one, `expect` in a phase that does not — so a wrong name announces itself instead of returning a silent zero. Third, and worth a sentence because pyuvm readers will look for the alternative: there is no BFM singleton, anywhere, from here to the end of the book. A singleton asserts there is exactly one BFM in the world, which is false for any testbench with two interfaces. The database asserts only that there is one *under this name, for this subtree* — which scales, and which a test can re-point without touching a component. That is what the mechanism is for.

The honest cost: you have just met a database, a path glob, and a `Result` while still learning what an environment is. The UVM front-loads this too — every SystemVerilog engineer's first testbench has that `set(null, "*", ...)` line in `top.sv` before they can explain it — and the two-line reading above is all this chapter needs.

```rust
// Chapter 25, Figure 4: The two testers are type aliases over the base
pub type RandomTester = BaseTester<RandomOperands>;
pub type MaxTester = BaseTester<MaxOperands>;
```

Where the Python book subclassed `BaseTester` twice to override one method, the generic base is written once and the two testers are *names for specializations*. What varies is a type parameter, not an override. (Hold this thought loosely: it is the right tool when the variation is chosen at compile time, as it is here. Chapter 30 meets the variation that must be chosen at *run* time, and generics will not survive the encounter.)

## The Scoreboard as a component

```rust
// Chapter 25, Figure 5: The Scoreboard as a component
#[derive(Component, Default)]
pub struct Scoreboard {
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}
```

```rust
// Chapter 25, Figure 6: Launching the monitoring tasks in start_of_simulation
impl Component for Scoreboard {
    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        let (cmd_bfm, cmds) = (bfm.clone(), self.cmds.clone());
        spawn_named(
            async move {
                loop {
                    let cmd = cmd_bfm.get_cmd().await;
                    cmds.borrow_mut().push(cmd);
                }
            },
            "scoreboard.get_cmds",
        );

        let (result_bfm, results) = (bfm, self.results.clone());
        spawn_named(
            async move {
                loop {
                    let result = result_bfm.get_result().await;
                    results.borrow_mut().push(result);
                }
            },
            "scoreboard.get_results",
        );
    }
```

Why `start_of_simulation` and not `run`? Because this phase does not *consume time* — `spawn_named` schedules a task and returns, exactly as `cocotb.start_soon` did in the Python original — and the lifecycle guarantees `start_of_simulation` finishes across the *whole tree* before any `run` begins. The monitors are provably listening before the first stimulus. Spawn them in `run` instead and you are racing the tester for the first transaction. (Resist the urge to "correct" this on the grounds that the UVM starts processes in `run_phase` — pyuvm's version of this scoreboard makes the same choice for the same reason.)

One ownership note, because it is Chapter 5's rule surfacing in framework clothes: a spawned task must own everything it touches — that is the `'static` bound Chapter 16 taught. The tasks here cannot borrow the scoreboard, so they clone `Rc` handles: each task owns a reference count, not a borrow of `self`.

```rust
// Chapter 25, Figure 7: Checking results in the check phase
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let mut results = self.results.borrow_mut();
        for cmd in self.cmds.borrow().iter() {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            self.cvg.insert(op);
            let actual = results.remove(0) as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                ctx.info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }

        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            ctx.info("Covered all operations");
        }
    }
}
```

Chapter 20's `check_results()` returned a `bool` and somebody had to remember to call it. This `check` is a *phase*: the framework calls it after the run phase ends, and failures reported to the `CheckSink` fail the test. Nothing calls anything by hand — the sequencing the earlier testbenches did with discipline, the lifecycle now does with guarantees.

## Using an environment

```rust
// Chapter 25, Figure 8: The environment builds the scoreboard and a tester
#[derive(Component, Default)]
pub struct AluEnv<T: Operands + Default + 'static> {
    #[component]
    scoreboard: Option<Scoreboard>,
    #[component]
    tester: Option<BaseTester<T>>,
}

impl<T: Operands + Default + 'static> Component for AluEnv<T> {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.scoreboard = Some(Scoreboard::default());
        self.tester = Some(BaseTester::<T>::default());
    }
}
```

```rust
// Chapter 25, Figure 9: RandomEnv and MaxEnv are type aliases
pub type RandomEnv = AluEnv<RandomOperands>;
pub type MaxEnv = AluEnv<MaxOperands>;
```

The environment is Chapter 24's pattern doing real work: `Option` children filled in by `build`, the phaser descending into the subtree as it comes into existence. And where Python needed `BaseEnv`, `RandomEnv`, and `MaxEnv` as three classes — the subclasses calling `super().build_phase()` and adding their tester — one generic env replaces all three, with the variants as aliases again.

```rust
// Chapter 25, Figure 10: Each test builds the environment it wants
#[rustdv::test]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    env: Option<RandomEnv>,
}

impl Component for RandomTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = Some(RandomEnv::default());
    }
}

#[rustdv::test]
#[derive(Component, Default)]
struct MaxTest {
    #[component]
    env: Option<MaxEnv>,
}

impl Component for MaxTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = Some(MaxEnv::default());
    }
}
```

The test builds the BFM — it is the only component that holds the DUT handle — and files it from the top: `None` context means "from the root," `"*"` means every path, so one line serves the whole tree. Then it builds the env it wants, and it is done: **the test has no run phase at all.** Stimulus lives in the tester now, and the objection the tester raises is what holds the run phase open. Compare this against Chapter 23's test, which did everything; the methodology is redistributing the work into the tree, one version at a time.

```text
# Figure 11: Testbench 4.0 running

      0.00ns INFO     rustdv: found 2 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running RandomTest (1/2)  [ch25-uvm-env-testbench-4.0/src/ch25_uvm_env_testbench_4_0.rs:256]
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    150.00ns INFO     [RandomTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    150.00ns INFO     [RandomTest.env.scoreboard]: Covered all operations
    150.00ns INFO     RandomTest PASSED
    150.00ns INFO     running MaxTest (2/2)  [ch25-uvm-env-testbench-4.0/src/ch25_uvm_env_testbench_4_0.rs:271]
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

Chapter 23 promised this transcript: the `PASSED` lines now carry `[RandomTest.env.scoreboard]`, because the scoreboard is a component with an address, and the address was derived from field names by the walk. Same results as 3.0; every line of output now says who produced it.

## Summary

Testbench 4.0 turns the 2.0 classes into components and houses them in an environment. The testers become one generic `BaseTester<T>` with the varying behavior as a type parameter and the variants as type aliases; the scoreboard collects in `start_of_simulation` — spawned tasks, owning `Rc` handles rather than borrowing, listening before any `run` begins — and compares in `check`, where failures reach the `CheckSink` instead of a hand-checked `bool`. The BFM travels by name through the ConfigDb, set once at the top by the test and retrieved by whoever needs it: the mechanism SystemVerilog uses for virtual interfaces, met here in two lines and treated fully in Chapter 27. The tests shrink to two `build` lines each.

Before the ConfigDb gets its chapter, one comfort of the old testbenches deserves restoring: log messages you can filter, route, and silence per component. Logging is Chapter 26.

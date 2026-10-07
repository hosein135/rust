//! Chapter 30: Variation-point testbench 5.0 — one env, two tests.
//!
//!     sim-common/run_sim.sh ch30_variation_point_testbench_5_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Testbench 4.0 chose its tester with a **type parameter**: `AluEnv<T>`, and
//! `RandomTest`/`MaxTest` were type aliases (D28). That is a compile-time
//! decision — the environment is a different type in each test. Testbench 5.0
//! makes the same choice at **run time**, through the factory built in
//! Chapter 29: there is now *one* `AluEnv`, and each test overrides the tester
//! before building it. This is the port of the Python book's chapter 34, and
//! it is UVM working exactly as designed — `BaseTester.create()` in the env,
//! `set_type_override_by_type` in each test (Q16, resolved).
//!
//! Every child is an `RustdvComp` slot, created with `new_comp()` when it is
//! fixed and `create_comp()` when it may be overridden — the build line, not
//! the field type, carries that choice (D75). Here the tester is the one
//! `create_comp()`; everything else is `new_comp()`. The results match
//! testbench 4.0 bit for bit — the same stimulus, chosen a different way.
//!
//! The testers are Chapter 20's `Tester` trait, unchanged in shape: one
//! required `get_operands`, and the stimulus every tester runs as a provided
//! method. `RandomTester` and `MaxTester` are the same two testers the reader
//! has had since testbench 2.0; what is new at 5.0 is only that they are
//! components, and that a *test* picks between them at run time (D115).
//!
//! The BFM, `Ops` and `alu_prediction` come from `tinyalu_utils`; the
//! Scoreboard is re-shown from testbench 4.0 (D45), because the reader is
//! meant to see it unchanged while the tester's *selection* changes around it.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// ===========================================================================
// The testers
// ===========================================================================

// Chapter 30, Figure 1: The Tester trait — one method varies, the rest is
// shared.
//
// This is Chapter 20's trait. `get_operands` is required and is the only
// thing a tester writes; `execute` is a provided method holding the stimulus
// every tester runs, which is how Rust spells the abstract base class the
// Python book writes as `BaseTester.run_phase`. The only change since 2.0 is
// that `execute` now takes the context instead of a BFM handle, because a
// component asks the ConfigDb for its BFM rather than being handed one.
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

// Chapter 30, Figure 2: The abstract base and the two testers that fill its
// slot.
//
// `BaseTester` is the type the environment names and the factory overrides —
// the analogue of the Python book's abstract `BaseTester`, which raises if it
// is ever run un-overridden. Here that is a `panic!`: a test that forgets its
// override builds a `BaseTester`, and running it is the bug.
//
// `RandomTester` and `MaxTester` are testbench 2.0's testers, still differing
// only in `get_operands`. Each is now also a component: `Tester` supplies the
// stimulus, `Component` supplies the phases, and `run` is the one line that
// joins them. `RandomTester` takes its seeded `Rng` in `build`, because a
// component is created by its parent with nothing passed in (D6).
//
// `#[derive(Component)]` registers all three, so any can stand in the tester
// slot by type override (Figures 4-5) or by name.
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

// ===========================================================================
// The Scoreboard — copied from testbench 4.0 (unchanged)
// ===========================================================================

// Collects commands and results in start_of_simulation, compares them in
// check. It is not a variation point, so it is built the plain way and does
// not change between 4.0 and 5.0.
#[derive(Component, Default)]
struct Scoreboard {
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}

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

// ===========================================================================
// The environment with a factory-built tester
// ===========================================================================

// Chapter 30, Figure 3: The environment builds its tester through the factory.
//
// Every child is an `RustdvComp` slot; the build line decides fixed vs
// overridable (D75). The scoreboard is fixed — `new_comp()`. The tester is
// the variation point — `create_comp()`, so a test above can substitute a
// different tester without this code being edited or even knowing (D69). At
// 4.0 the choice was a type parameter; here it is one create_comp line.
//
// `start_of_simulation` starts the BFM's driver and monitor tasks — once, for
// the whole environment (the Python book's `AluEnv.start_of_simulation_phase`).
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

// ===========================================================================
// The tests
// ===========================================================================

// Chapter 30, Figure 4: random_test overrides BaseTester with RandomTester.
//
// The override is installed in the test's `build`, which runs before the walk
// reaches `env.tester` (build is top-down), so it is in force by the time the
// factory resolves that slot. The env is not edited between the two tests —
// only the override changes.
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

// Chapter 30, Figure 5: max_test differs only in the tester it installs.
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

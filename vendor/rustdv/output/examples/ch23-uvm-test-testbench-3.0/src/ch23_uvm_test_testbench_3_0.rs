//! Chapter 23: uvm_test testbench 3.0.
//!
//!     sim-common/run_sim.sh ch23_uvm_test_testbench_3_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! A test is a **component**: `#[rustdv::test]` registers it under its type
//! name, verbatim — `RandomTest`, not `random_test` (D102) — so the runner can
//! find it, which is the UVM's `run_test()` restored. At 3.0 the test is the
//! *only* component; components proper arrive in Chapter 24 and the
//! environment in Chapter 25.
//!
//! Only infrastructure is imported — the BFM, `Ops`, `alu_prediction`.
//! Everything this chapter teaches is written here, even where it repeats
//! Chapter 20, because the reader has to see it. These classes change as
//! the book goes on, and showing them is how that change is visible.
//!
//! There is no software clock. The RTL supplies it, the BFM only ever waits
//! on edges, and that is what lets the same testbench run on an emulator.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// ===========================================================================
// Copied from testbench 2.0
// ===========================================================================

// Chapter 20, Figure 2: Common behavior across all testers.
// `execute` is a default method — Rust's form of the abstract base class.
#[allow(async_fn_in_trait)]
pub trait Tester {
    fn get_operands(&mut self) -> (u8, u8);

    async fn execute(&mut self, bfm: &TinyAluBfm) {
        for op in Ops::ALL {
            let (aa, bb) = self.get_operands();
            bfm.send_op(aa, bb, op).await;
        }
        // send two dummy operations to allow
        // the last real operation to complete
        bfm.send_op(0, 0, Ops::Add).await;
        bfm.send_op(0, 0, Ops::Add).await;
    }
}

// Chapter 20, Figure 3: RandomTester overrides get_operands()
pub struct RandomTester {
    pub rng: Rng,
}

impl Tester for RandomTester {
    fn get_operands(&mut self) -> (u8, u8) {
        (self.rng.u8(), self.rng.u8())
    }
}

// Chapter 20, Figure 4: MaxTester overrides get_operands()
pub struct MaxTester;

impl Tester for MaxTester {
    fn get_operands(&mut self) -> (u8, u8) {
        (0xFF, 0xFF)
    }
}

// Chapter 20, Figures 5–8: the Scoreboard
pub struct Scoreboard {
    bfm: Rc<TinyAluBfm>,
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}

impl Scoreboard {
    pub fn new(bfm: Rc<TinyAluBfm>) -> Scoreboard {
        Scoreboard {
            bfm,
            cmds: Rc::new(RefCell::new(Vec::new())),
            results: Rc::new(RefCell::new(Vec::new())),
            cvg: HashSet::new(),
        }
    }

    pub fn start_tasks(&self) {
        let (bfm, cmds) = (self.bfm.clone(), self.cmds.clone());
        spawn_named(
            async move {
                loop {
                    let cmd = bfm.get_cmd().await;
                    cmds.borrow_mut().push(cmd);
                }
            },
            "scoreboard.get_cmd",
        );
        let (bfm, results) = (self.bfm.clone(), self.results.clone());
        spawn_named(
            async move {
                loop {
                    let result = bfm.get_result().await;
                    results.borrow_mut().push(result);
                }
            },
            "scoreboard.get_result",
        );
    }

    pub fn check_results(&mut self) -> bool {
        let mut passed = true;
        let mut results = self.results.borrow_mut();
        for cmd in self.cmds.borrow().iter() {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("illegal op captured");
            self.cvg.insert(op);
            let actual = results.remove(0) as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                passed = false;
                log::error(&format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }

        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            log::error("Functional coverage error: missed operations");
            passed = false;
        } else {
            log::info("Covered all operations");
        }
        passed
    }
}

// ===========================================================================
// uvm_test testbench 3.0
// ===========================================================================

// Chapter 23, Figure 1: The basic rustdv-UVM use model in hello_world
// A component with nothing but a run phase. No children, so no build phase —
// `Component` supplies a default.

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

// Chapter 23, Figure 4: alu_test — the shared run phase of every ALU test.
// SystemVerilog and Python share this through `class random_test extends
// base_test`. Rust has no inheritance, so the shared body is a function and
// each test hands it a tester. The BFM and scoreboard are ordinary locals;
// neither is a component yet.

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

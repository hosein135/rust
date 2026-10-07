//! Chapter 20: Struct-based testbench 2.0.
//!
//!     sim-common/run_sim.sh ch20_struct_based_testbench_2_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Tester trait + Scoreboard struct, wired by hand — deliberately showing
//! the plumbing that Part IV's machinery will remove.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// Chapter 20, Figure 2: Common behavior across all testers
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

// Chapter 20, Figure 5: Initializing the Scoreboard
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

    // Chapter 20, Figure 6: The Scoreboard's data-gathering tasks
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

    // Chapter 20, Figure 7: The check_results() phase
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

        // Chapter 20, Figure 8: The Scoreboard checks functional coverage
        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            log::error("Functional coverage error: missed operations");
            passed = false;
        } else {
            log::info("Covered all operations");
        }
        passed
    }
}

// Chapter 20, Figure 9: The execute_test coroutine starts the tasks
async fn execute_test(ctx: &RustdvCtx, tester: &mut impl Tester) -> Result<bool, TestError> {
    // The RTL self-clocks (tinyalu.sv); the BFM only waits on edges (D42).
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    let mut scoreboard = Scoreboard::new(bfm.clone());
    bfm.reset().await;
    bfm.start_tasks();
    scoreboard.start_tasks();

    // Chapter 20, Figure 10: Execute the tester
    tester.execute(&bfm).await;
    let passed = scoreboard.check_results();
    Ok(passed)
}

// Chapter 20, Figure 11: The tests launch execute_test with a tester
#[rustdv::test]
async fn random_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Random operands
    let mut tester = RandomTester { rng: ctx.rng() };
    let passed = execute_test(&ctx, &mut tester).await?;
    if passed {
        Ok(())
    } else {
        Err(TestError::from("random_test saw failing comparisons"))
    }
}

// Chapter 20, Figure 12: The max test differs only in its tester
#[rustdv::test]
async fn max_test(ctx: RustdvCtx) -> Result<(), TestError> {
    // Maximum operands
    let mut tester = MaxTester;
    let passed = execute_test(&ctx, &mut tester).await?;
    if passed {
        Ok(())
    } else {
        Err(TestError::from("max_test saw failing comparisons"))
    }
}

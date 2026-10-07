//! Chapter 25: uvm_env testbench 4.0 — components in an environment.
//!
//!     sim-common/run_sim.sh ch25_uvm_env_testbench_4_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Testbench 4.0 is where the pieces become **components**. At 3.0 the test
//! was the only component and the tester and scoreboard were ordinary
//! locals inside `run`. Here each becomes a component with its own phases,
//! and an *environment* holds them — the reusable unit UVM is built around.
//!
//! Two things arrive with that, and both are the UVM working as designed:
//!
//! - The tester and the scoreboard are now **siblings**, created in their
//!   parent's `build` phase, so neither can be handed the BFM through a
//!   constructor. They ask for it by name from the **ConfigDb**, which the
//!   test filled in. That is how SystemVerilog does it too; Chapter 27 is the
//!   full treatment, and Figure 3 says just enough to read the code.
//! - The scoreboard does its collecting in `start_of_simulation` and its
//!   comparing in `check` — phases, not hand-called methods.
//!
//! Port of the Python book's chapter 29.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// ===========================================================================
// Converting the testers to components
// ===========================================================================

// Chapter 25, Figure 1: What varies between testers is only the operands.
// SystemVerilog and Python express this with an abstract base class and one
// overridden method; Rust expresses it as a trait with one required method.
pub trait Operands {
    fn get_operands(&mut self, rng: &mut Rng) -> (u8, u8);
}

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

// Chapter 25, Figure 3: BaseTester implements the phases common to all
// testers. `BaseTester(uvm_component)` subclassed twice becomes one generic
// component written once, with the two testers as type aliases (D28): what
// varies is a type parameter, not an override.
//
// The BFM is not a constructor argument — this component is created by its
// parent's build phase, which passes nothing (D6). So how does it get one?
//
// **The ConfigDb**, which is this chapter's other new idea. A component that
// needs something it was not handed asks for it by name, and something above
// it in the tree put it there. Chapter 27 is the full treatment — precedence,
// wildcards, what happens when the name is wrong. For now two lines are
// enough to read the code:
//
//   ConfigDb::set(None, "*", "BFM", bfm)   // the test: everyone gets this one
//   ConfigDb::get(Some(ctx), "", "BFM")    // a component: give me mine
//
// The `Result` is the point of the mechanism rather than an inconvenience:
// SystemVerilog's `get()` returns a silent zero when the name is wrong, and
// you find out much later. This one says so (D14).
//
// SystemVerilog does the same thing for the same reason —
// `uvm_config_db#(virtual tinyalu_bfm)::set(null, "*", "bfm", bfm)` sits in
// the `top.sv` of every UVM testbench — and pyuvm reaches it with a
// singleton instead.
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
        // A phase that returns `Result` can use `?`; the phases above return
        // nothing, so they say `expect` instead. Same lookup either way.
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

// Chapter 25, Figure 4: The two testers are type aliases over the base.
pub type RandomTester = BaseTester<RandomOperands>;
pub type MaxTester = BaseTester<MaxOperands>;

// ===========================================================================
// The Scoreboard as a component
// ===========================================================================

// Chapter 25, Figure 5: The scoreboard collects in start_of_simulation and
// compares in check — the phases do the sequencing, so nothing calls these
// by hand.
#[derive(Component, Default)]
pub struct Scoreboard {
    cmds: Rc<RefCell<Vec<CmdTuple>>>,
    results: Rc<RefCell<Vec<u64>>>,
    cvg: HashSet<Ops>,
}

impl Component for Scoreboard {
    // Chapter 25, Figure 6: Launching the monitoring tasks.
    //
    // **Why here and not in `run`.** Only `run` may *consume time*, and this
    // does not: `spawn_named` schedules a task and returns, exactly as
    // `cocotb.start_soon` does in the Python original. The phase still
    // completes instantly.
    //
    // The payoff is that `start_of_simulation` finishes across the *whole
    // tree* before any `run` begins, so the monitors are provably listening
    // before the first stimulus. Spawn them in `run` instead and you are
    // racing the tester for the first transaction. Do not "correct" this
    // into `run` on the grounds that UVM starts processes there.
    //
    // The spawned tasks must be `'static`, so they cannot borrow the
    // scoreboard. They clone `Rc` handles instead: the task owns a
    // reference count, not a borrow of `self`.
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

    // Chapter 25, Figure 7: Checking results after the run phase completes.
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
// Using an environment
// ===========================================================================

// Chapter 25, Figure 8: The environment builds the scoreboard and a tester.
// Both children are `Option<T>` — "declared but not yet built" — and the
// build phase fills them in (D6). The phaser descends into whatever build
// creates, so the env's subtree comes into existence as it is walked.
//
// Python needs BaseEnv/RandomEnv/MaxEnv, with the subclasses calling
// `super().build_phase()` and adding their tester. One generic env replaces
// all three.
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

// Chapter 25, Figure 9: RandomEnv and MaxEnv are type aliases.
pub type RandomEnv = AluEnv<RandomOperands>;
pub type MaxEnv = AluEnv<MaxOperands>;

// ===========================================================================
// The tests
// ===========================================================================

// Chapter 25, Figure 10: Each test builds the environment it wants, and puts
// the BFM where its components can find it. The test has no run phase at all
// now — the stimulus moved into the tester component, and the objection it
// raises is what holds the run phase open.
//
// `None` as the first argument means "from the top", and `"*"` means every
// component below it, so one line serves the whole tree. The test is the right
// place for it: it is the only component that knows the DUT handle, and it is
// above everything that needs the BFM.

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

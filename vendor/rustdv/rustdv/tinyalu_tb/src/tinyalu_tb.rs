//! The TinyALU testbench — the worked example, end to end.
//!
//! Two tests, one testbench, no new components between them. The structure
//! holds still and the **program** changes: each test overrides which sequence
//! `BaseSeq` builds and then runs it. That is what sequences bought — in the
//! factory chapter a new stimulus pattern meant a new *component*.
//!
//! A test is a component like any other, so it gets the whole nine-phase
//! lifecycle and the runner drives it: `build` files the BFM and creates the
//! env, and `run` starts the stimulus. There is no hand-rolled phasing here.

use std::rc::Rc;

use rustdv::prelude::*;

// Test executables need vpi_* symbol definitions (the simulator provides them
// for the real cdylib) — see rustdv-vpi-stubs.
#[cfg(test)]
use rustdv_vpi_stubs as _;

pub mod alu_bfm;
pub mod alu_item;
pub mod components;
pub mod env;
pub mod sequences;

use alu_bfm::TinyAluBfm;
use env::AluEnv;
use sequences::{BaseSeq, MaxSeq, RandomSeq};

// Export the VPI entry points from this cdylib.
rustdv::vpi_bootstrap!();

/// Everything both tests share: file the BFM where any component can find it,
/// build the env, then run whichever sequence the factory has been told to
/// build for `BaseSeq`.
///
/// The clock comes from the RTL, not from here — the BFM only ever waits on
/// edges, which is what lets the same testbench run on an emulator (D42).
#[derive(Component, Default)]
pub struct BaseTest {
    #[component]
    env: RustdvComp,
}

impl Component for BaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");

        let seqr: Sequencer<alu_item::AluCommand, alu_item::AluResult> =
            ConfigDb::get(Some(ctx), "", "SEQR")?;

        // Built through the factory, so the test above chose what this is.
        let mut seq = create_seq::<BaseSeq>();
        seq.start(&seqr).await?;

        // `finish_item` returns when the driver has taken the command, not when
        // the DUT has answered it. Dropping the objection here would end the run
        // phase with results still in flight, and the scoreboard would silently
        // compare fewer results than it saw commands. Wait for the DUT to go
        // quiet first.
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.wait_idle().await;

        ctx.info("sequence complete");
        Ok(())
    }
}

/// Random operands across every operation, five times each.
#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    inner: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, RandomSeq>();
        self.inner = BaseTest::new_comp();
    }
}

/// The `0xff op 0xff` corner, once per operation.
#[rustdv::test(timeout_time = 500, timeout_unit = "us")]
#[derive(Component, Default)]
struct MaxTest {
    #[component]
    inner: RustdvComp,
}

impl Component for MaxTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, MaxSeq>();
        self.inner = BaseTest::new_comp();
    }
}

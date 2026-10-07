//! Chapter 19: the BFM-based test.
//!
//!     sim-common/run_sim.sh ch19_tinyalubfm tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! The BFM itself lives in the shared `tinyalu_utils` crate — see the
//! chapter's figures 2–13 there.

use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// Chapter 19, Figure 14: Starting a test by resetting the DUT
// and starting the BFM tasks
#[rustdv::test]
async fn test_alu(ctx: RustdvCtx) -> Result<(), TestError> {
    // Test all TinyALU operations through the BFM
    let mut rng = ctx.rng();
    let mut passed = true;
    // The RTL self-clocks (tinyalu.sv); the BFM only waits on edges (D42).
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.reset().await;
    bfm.start_tasks();

    let mut cvg: HashSet<Ops> = HashSet::new();

    // Chapter 19, Figure 15: Creating a command and sending it
    for op in Ops::ALL {
        let aa = rng.u8();
        let bb = rng.u8();
        bfm.send_op(aa, bb, op).await;

        // Chapter 19, Figure 16: Wait to get the command from the DUT
        // and store it in the coverage set
        let seen_cmd = bfm.get_cmd().await;
        let seen_op = Ops::from_u64(seen_cmd.2).expect("illegal op on the bus");
        cvg.insert(seen_op);

        // Chapter 19, Figure 17: Wait for the result, then create a prediction
        let result = bfm.get_result().await as u16;
        let pr = alu_prediction(aa, bb, op);

        // Chapter 19, Figure 18: Check the result against the predicted result
        if result == pr {
            log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {result:04x}"));
        } else {
            log::error(&format!(
                "FAILED: {aa:02x} {op:?} {bb:02x} = {result:04x} - predicted {pr:04x}"
            ));
            passed = false;
        }
    }

    if Ops::ALL.iter().any(|op| !cvg.contains(op)) {
        log::error("Functional coverage error: missed operations");
        passed = false;
    } else {
        log::info("Covered all operations");
    }

    // Chapter 19, Figure 19: The final Result relays pass/fail to rustdv
    if passed {
        Ok(())
    } else {
        Err(TestError::from("test_alu saw failing comparisons"))
    }
}

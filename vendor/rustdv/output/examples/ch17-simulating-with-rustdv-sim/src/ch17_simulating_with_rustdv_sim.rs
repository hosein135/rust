//! Chapter 17 simulator figures. Build and run with:
//!
//!     sim-common/run_sim.sh ch17_simulating_with_rustdv_sim counter \
//!         sim-common/hdl/timescale.v sim-common/hdl/counter.sv
//!
//! The DUT is the SystemVerilog counter from the chapter's Figure 1.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// Chapter 17, Figure 3: get_int() ports to one line of unwrap_or
fn get_int(signal: &LogicHandle) -> u64 {
    // x or z becomes 0, as tinyalu_utils decided
    signal.get_u64().unwrap_or(0)
}

// Chapter 17, Figure 2: A typo'd signal name is an Err, not a surprise
#[rustdv::test]
async fn name_lookup(ctx: RustdvCtx) -> Result<(), TestError> {
    // Show what child()/signal() return
    let dut = ctx.dut();
    let good = dut.signal("reset_n");
    let bad = dut.signal("rst_n"); // the classic typo
    log::info(&format!("reset_n -> {good:?}"));
    log::info(&format!("rst_n   -> {bad:?}"));
    Ok(())
}

// Chapter 17, Figure 4: Starting the clock, lowering reset
// Chapter 17, Figure 5: Wait for five clocks and check the output
#[rustdv::test]
async fn no_count(ctx: RustdvCtx) -> Result<(), TestError> {
    // Test no count if reset is 0
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    let reset_n = dut.signal("reset_n")?;
    reset_n.set_u64(0);

    for _ in 0..5 {
        clk.rising_edge().await;
    }
    let count = get_int(&dut.signal("count")?);
    log::info(&format!("After 5 clocks count is {count}"));
    assert_eq!(count, 0);
    Ok(())
}

// Chapter 17, Figure 6: Testing that the counter counts
#[rustdv::test]
async fn three_count(ctx: RustdvCtx) -> Result<(), TestError> {
    // Test that we count up as expected
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    let reset_n = dut.signal("reset_n")?;
    reset_n.set_u64(0);
    clk.falling_edge().await;
    reset_n.set_u64(1);
    for _ in 0..3 {
        clk.falling_edge().await;
    }
    let count = get_int(&dut.signal("count")?);
    log::info(&format!("After 3 clocks, count is {count}"));
    assert_eq!(count, 3);
    Ok(())
}

// Chapter 17, Figure 7: Forgetting the await is now a compiler warning
#[rustdv::test]
async fn oops(ctx: RustdvCtx) -> Result<(), TestError> {
    // Demonstrate the coroutine mistake
    let dut = ctx.dut();
    let clk = dut.signal("clk")?;
    Clock::new(&clk, SimDuration::ns(2)).start();
    clk.rising_edge(); // forgot .await
    log::info("Did not await");
    Ok(())
}

//! rustdv testbench for the one-bit full adder.
//!
//! A full adder takes three single bits — `a`, `b`, and carry-in — and
//! drives `sum` and `cout`. This component walks all eight vectors against
//! the TxHDL unit and writes `full_adder.vcd` (or `$TXHDL_VCD`). The dump
//! has the wires the testbench drives and the registers the unit latches.
//! Press Run in the IDE to open that waveform.

use rustdv::prelude::*;

use crate::adder::{FullAdder, directed};

#[cfg(test)]
use rustdv_vpi_stubs as _;

/// Directed test of the three-input full adder.
#[rustdv::test(name = "full_adder_test")]
#[derive(Component, Default)]
pub struct FullAdderTest;

impl Component for FullAdderTest {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        // `ctx.info` reads simulator time through VPI. This run has no
        // simulator, so the verdict is the `Result`.
        let _guard = ctx.raise_objection("directed stimulus");

        let verilog = FullAdder::verilog("full_adder");
        if !verilog.contains("module full_adder") {
            return Err(TestError::new(format!(
                "lowering did not emit a full_adder module:\n{verilog}"
            )));
        }

        let vcd = std::env::var("TXHDL_VCD").unwrap_or_else(|_| "full_adder.vcd".into());
        directed(Some(std::path::Path::new(&vcd))).map_err(TestError::new)?;
        Ok(())
    }
}

/// Poll a future that must finish without waiting on simulator time.
#[cfg(test)]
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let mut fut = std::pin::pin!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("full adder testbench waited on simulator time"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_adder_regression() {
        let result = block_on(async {
            let mut test = FullAdderTest::default();
            let mut ctx = RustdvCtx::new(
                "full_adder_test",
                HierarchyHandle::null_for_test(),
                1,
            );
            run_component_test(&mut test, &mut ctx).await
        });
        result.expect("full adder regression");
    }
}

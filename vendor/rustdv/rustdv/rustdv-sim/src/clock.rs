//! Clock generator (port of cocotb `Clock`, mapping row 24).

use crate::executor::{TaskHandle, spawn_named};
use crate::handle::{HandleBase, LogicHandle};
use crate::time::SimDuration;
use crate::triggers::Timer;

/// `Clock::new(&dut_clk, SimDuration::ns(10)).start()`.
pub struct Clock {
    sig: LogicHandle,
    period: SimDuration,
}

impl Clock {
    pub fn new(sig: &LogicHandle, period: SimDuration) -> Clock {
        assert!(
            period.steps >= 2,
            "clock period must be at least 2 precision steps"
        );
        Clock { sig: *sig, period }
    }

    /// Spawn the free-running clock task (starts high, like cocotb's
    /// default). Cancel the returned handle to stop the clock.
    pub fn start(&self) -> TaskHandle<()> {
        let sig = self.sig;
        let high = self.period.steps / 2;
        let low = self.period.steps - high;
        spawn_named(
            async move {
                loop {
                    sig.set_u64_now(1);
                    Timer::steps(high).await;
                    sig.set_u64_now(0);
                    Timer::steps(low).await;
                }
            },
            &format!("clock({})", sig.name()),
        )
    }
}

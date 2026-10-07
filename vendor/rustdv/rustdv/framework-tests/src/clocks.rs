//! `clock_` — `Clock` produces the period it was asked for.
//!
//! Small surface, high blast radius: every chapter from 17 on measures its
//! testbench in clocks, so a clock running at the wrong period would make
//! every one of those transcripts wrong together, and consistently — which is
//! the kind of wrong that survives review.

use rustdv::prelude::*;

// The stated period is the period you get.
#[rustdv::test]
async fn clock_period_is_what_was_asked_for(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(4)).start();

    clk.rising_edge().await;
    let t0 = sim_time_ns();
    for _ in 0..10 {
        clk.rising_edge().await;
    }
    let dt = sim_time_ns() - t0;
    check!(dt == 40.0, "ten periods of a 4ns clock took {dt} ns");
    Ok(())
}

// The duty cycle is even: the high half equals the low half.
#[rustdv::test]
async fn clock_halves_are_equal(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    Clock::new(&clk, SimDuration::ns(4)).start();

    clk.rising_edge().await;
    let rise = sim_time_ns();
    clk.falling_edge().await;
    let fall = sim_time_ns();
    clk.rising_edge().await;
    let next_rise = sim_time_ns();

    let high = fall - rise;
    let low = next_rise - fall;
    check!(high == low, "the clock was high {high} ns and low {low} ns");
    check!(high == 2.0, "half a 4ns period measured {high} ns");
    Ok(())
}

// The RTL sees the clock too — the edge is not a software fiction.
//
// `probe.sv` counts rising edges in an `always` block. If the framework's
// clock and the simulator's notion of it ever diverged, this is where it
// would show: the testbench would count edges the design did not.
#[rustdv::test]
async fn clock_drives_the_design(ctx: RustdvCtx) -> Result<(), TestError> {
    let clk = ctx.dut().signal("clk")?;
    let counted = ctx.dut().signal("counted")?;
    Clock::new(&clk, SimDuration::ns(2)).start();

    clk.rising_edge().await;
    // Read after the design's non-blocking assignment has settled.
    read_only().await;
    let start = counted.get_u64().unwrap_or(0);

    for _ in 0..6 {
        clk.rising_edge().await;
    }
    read_only().await;
    let end = counted.get_u64().unwrap_or(0);

    check!(
        end.wrapping_sub(start) == 6,
        "the design counted {} edges where the testbench awaited 6",
        end.wrapping_sub(start)
    );
    Ok(())
}

// Two clocks run independently at their own rates.
//
// The window is bounded by the slow clock's *falling* edges, and that is not a
// stylistic choice. A 2ns clock and a 10ns clock share a rising edge every
// 10ns by construction, so a window running from one slow rising edge to
// another both starts and ends on an instant where a fast edge also occurs.
// The count then depends on which of two simultaneous callbacks the simulator
// delivers first — the fast clock's toggle, or the slow one's toggle that
// cancels the counting task — which is a tie-break, not a period. Half the
// slow period puts both boundaries 1ns clear of every fast edge, so ten is ten
// for a reason the test can state.
#[rustdv::test]
async fn clock_two_are_independent(ctx: RustdvCtx) -> Result<(), TestError> {
    // `flag` is not wired to anything in the design, so it serves as a second
    // free-running clock without disturbing the counter.
    let fast = ctx.dut().signal("clk")?;
    let slow = ctx.dut().signal("flag")?;
    Clock::new(&fast, SimDuration::ns(2)).start();
    Clock::new(&slow, SimDuration::ns(10)).start();

    // Sync on the slow clock first, so the window below starts at one of its
    // edges rather than wherever the test happened to begin.
    slow.falling_edge().await;
    let t0 = sim_time_ns();
    let mut fast_edges = 0;
    // Count the fast clock until the slow one has ticked twice more.
    let counting = async {
        loop {
            fast.rising_edge().await;
            fast_edges += 1;
        }
    };
    let waiting = async {
        slow.falling_edge().await;
        slow.falling_edge().await;
    };
    let _ = first2(counting, waiting).await;

    let dt = sim_time_ns() - t0;
    check!(dt == 20.0, "two 10ns periods took {dt} ns");
    check!(
        fast_edges == 10,
        "the 2ns clock ticked {fast_edges} times in 20 ns, not 10"
    );
    Ok(())
}

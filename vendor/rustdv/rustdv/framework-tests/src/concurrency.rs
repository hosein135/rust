//! `conc_` — the D82 family, against real simulated time.
//!
//! D82 decided that rustdv composes futures rather than spawning them, that
//! a parent's run is concurrent with its children's (D82b), and that the
//! objection race is run **per component** rather than around the whole tree
//! (D82c). The last of those was a *silent pass* before it was fixed: racing
//! the tree dropped every component mid-phase, so extract/check/report walked
//! a hierarchy that had already been destroyed, and nothing complained.
//!
//! A bug that makes checks quietly stop running is the worst kind a
//! verification framework can have, and `conc_objection_race_is_per_component`
//! exists so it cannot come back.

use std::cell::{Cell, RefCell};

use rustdv::prelude::*;

thread_local! {
    /// What ran, in the order it ran. Cheaper and clearer than threading a
    /// shared handle through the ConfigDb, and these are tests: the coupling
    /// costs nothing outside this file.
    static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static TICKS: Cell<u32> = const { Cell::new(0) };
    static FOREVER_CHECKED: Cell<bool> = const { Cell::new(false) };
}

fn trace(s: impl Into<String>) {
    TRACE.with(|t| t.borrow_mut().push(s.into()));
}

fn take_trace() -> Vec<String> {
    TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()))
}

fn reset() {
    take_trace();
    TICKS.with(|t| t.set(0));
    FOREVER_CHECKED.with(|c| c.set(false));
}

// ---------------------------------------------------------------------------
// Two run phases genuinely interleave
// ---------------------------------------------------------------------------

// The producer/consumer shape every chapter from 31 on depends on.
//
// Sequential execution would produce `a a a b b b`; concurrent execution
// interleaves by the clock each task keeps. If run phases ever silently
// serialized, every TLM chapter would still pass — the items would arrive,
// just never at the same time as anything else — and this is the test that
// would not.
#[rustdv::test]
async fn conc_two_runs_interleave(_ctx: RustdvCtx) -> Result<(), TestError> {
    reset();

    let slow = async {
        for i in 0..3 {
            Timer::ns(10).await;
            trace(format!("slow{i}"));
        }
    };
    let fast = async {
        for i in 0..3 {
            Timer::ns(4).await;
            trace(format!("fast{i}"));
        }
    };
    join2(slow, fast).await;

    let got = take_trace();
    let want = ["fast0", "fast1", "slow0", "fast2", "slow1", "slow2"];
    check!(
        got == want,
        "the two tasks did not interleave by time:\n  got  {got:?}\n  want {want:?}"
    );
    Ok(())
}

// `join_all` returns in input order however the futures finish, and takes as
// long as the slowest.
#[rustdv::test]
async fn conc_join_all_preserves_order(_ctx: RustdvCtx) -> Result<(), TestError> {
    let t0 = sim_time_ns();
    let futs: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = u64>>>> = vec![
        Box::pin(async {
            Timer::ns(30).await;
            30
        }),
        Box::pin(async {
            Timer::ns(10).await;
            10
        }),
        Box::pin(async {
            Timer::ns(20).await;
            20
        }),
    ];
    let out = rustdv::sim::combinators::join_all(futs).await;
    let dt = sim_time_ns() - t0;

    check!(
        out == vec![30, 10, 20],
        "join_all reordered its results: {out:?}"
    );
    check!(dt == 30.0, "join_all over 30/10/20 ns took {dt} ns, not 30");
    Ok(())
}

// `first2` returns the winner and drops the loser — the mechanism D82c is
// built on. A loser that kept running would keep its triggers registered and
// keep ticking; this one records that it did not.
#[rustdv::test]
async fn conc_first2_drops_the_loser(_ctx: RustdvCtx) -> Result<(), TestError> {
    reset();

    let winner = async {
        Timer::ns(5).await;
        "winner"
    };
    let loser = async {
        loop {
            Timer::ns(1).await;
            TICKS.with(|t| t.set(t.get() + 1));
        }
    };
    let out = first2(winner, loser).await;
    check!(
        matches!(out, Either::First("winner")),
        "the wrong future won"
    );

    let at_finish = TICKS.with(|t| t.get());
    Timer::ns(20).await;
    let later = TICKS.with(|t| t.get());
    check!(
        later == at_finish,
        "the dropped future kept running: {at_finish} ticks at the race, {later} twenty ns later"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// A parent's run is concurrent with its children's (D82b)
// ---------------------------------------------------------------------------

#[derive(Component, Default)]
struct FirstChild;
impl Component for FirstChild {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        trace(format!("child1@{}", sim_time_ns()));
        Timer::ns(10).await;
        Ok(())
    }
}

#[derive(Component, Default)]
struct SecondChild;
impl Component for SecondChild {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        trace(format!("child2@{}", sim_time_ns()));
        Timer::ns(10).await;
        Ok(())
    }
}

// Every run body starts at the same instant.
//
// If the walk awaited each child in turn, the second would start ten ns after
// the first. The whole point of D82b is that it does not.
#[rustdv::test]
#[derive(Component, Default)]
struct ConcParentAndChildrenRunTogether {
    #[component]
    a: RustdvComp,
    #[component]
    b: RustdvComp,
}

impl Component for ConcParentAndChildrenRunTogether {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        reset();
        self.a = FirstChild::new_comp();
        self.b = SecondChild::new_comp();
    }

    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        trace(format!("parent@{}", sim_time_ns()));
        Timer::ns(10).await;
        Ok(())
    }

    fn check(&mut self, _ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let got = take_trace();
        if got.len() != 3 {
            errors.error(format!("expected three run bodies to start, saw {got:?}"));
            return;
        }
        // Every entry carries the sim time its body began at.
        let times: Vec<&str> = got
            .iter()
            .map(|s| s.split('@').nth(1).unwrap_or("?"))
            .collect();
        if times.iter().any(|t| *t != times[0]) {
            errors.error(format!(
                "run bodies started at different times, so they ran in sequence: {got:?}"
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// D82c: the objection race is per component
// ---------------------------------------------------------------------------

/// A driver that never returns, which is what a driver is.
#[derive(Component, Default)]
struct Forever;

impl Component for Forever {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        loop {
            Timer::ns(1).await;
            TICKS.with(|t| t.set(t.get() + 1));
        }
    }

    fn check(&mut self, _ctx: &mut RustdvCtx, _errors: &mut CheckSink) {
        FOREVER_CHECKED.with(|c| c.set(true));
    }
}

/// Finite work that raises an objection, so the phase has a reason to end.
#[derive(Component, Default)]
struct Objector;

impl Component for Objector {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("finite work");
        Timer::ns(20).await;
        Ok(())
    }
}

/// Declared after `Forever`, so its `check` runs after `Forever`'s — the
/// check walk is top-down and in declaration order, which is what lets this
/// component read a verdict the previous one wrote.
#[derive(Component, Default)]
struct Verdict;

impl Component for Verdict {
    fn check(&mut self, _ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        if TICKS.with(|t| t.get()) == 0 {
            errors.error("the forever-looping driver never ran at all");
        }
        if !FOREVER_CHECKED.with(|c| c.get()) {
            errors.error(
                "the forever-looping driver's check phase never ran — it was destroyed \
                 with the run phase instead of surviving it (D82c)",
            );
        }
    }
}

// A component whose run never returns is dropped at consensus, and **the
// component survives to be checked**.
//
// This test passing is the difference between a scoreboard that reports and a
// scoreboard that is silently thrown away before it can. If the race is ever
// hoisted back around the whole tree, `Verdict` fires; if the race is removed
// entirely, the test hangs and the runner's timeout reports it.
#[rustdv::test(timeout_time = 10, timeout_unit = "us")]
#[derive(Component, Default)]
struct ConcObjectionRaceIsPerComponent {
    #[component]
    forever: RustdvComp,
    #[component]
    objector: RustdvComp,
    #[component]
    verdict: RustdvComp,
}

impl Component for ConcObjectionRaceIsPerComponent {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        reset();
        self.forever = Forever::new_comp();
        self.objector = Objector::new_comp();
        self.verdict = Verdict::new_comp();
    }
}

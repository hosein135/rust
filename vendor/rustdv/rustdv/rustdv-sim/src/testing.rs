//! Running futures in a unit test, with no simulator.
//!
//! Most of a verification framework is not about time. The ConfigDb, the
//! factory, port binding, the analysis broadcast and the whole sequencer
//! handshake are built from `Event`s and `Queue`s, neither of which touches
//! the simulator — so they can be tested in milliseconds under `cargo test`
//! instead of minutes under Icarus.
//!
//! The line is sharp and this module is where it is enforced: [`block_on`]
//! drives a future on a bare executor, and if the future is still pending when
//! the run queue empties, it says so and names the likely cause. A test that
//! awaits `Timer` or a clock edge **needs a simulator**, and will fail here
//! rather than hanging.
//!
//! ```ignore
//! #[test]
//! fn a_queue_round_trips() {
//!     block_on(async {
//!         let q: Queue<u8> = Queue::unbounded();
//!         q.put(7).await;
//!         assert_eq!(q.get().await, 7);
//!     });
//! }
//! ```

use std::future::Future;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use crate::executor;

/// How many times we alternate polling the future with draining the run
/// queue before declaring it stuck. Any test needing more than this is
/// either broken or wants the simulator.
const MAX_ROUNDS: usize = 10_000;

fn noop_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(std::ptr::null(), &VTABLE)
}

/// A waker that does nothing, because [`block_on`] re-polls unconditionally.
fn noop_waker() -> Waker {
    // Safety: the vtable's functions are all no-ops over a null pointer and
    // never dereference it.
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

/// Run `fut` to completion on a fresh executor, with no simulator.
///
/// Each round polls the future once and then drains the run queue, so a
/// future waiting on an `Event` makes progress as soon as a spawned task sets
/// it. Panics if the future is still pending after 10,000 rounds.
pub fn block_on<F: Future>(fut: F) -> F::Output {
    let ex = executor::init();
    let mut fut = Box::pin(fut);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    for _ in 0..MAX_ROUNDS {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
        ex.run_until_idle();
    }

    panic!(
        "block_on: the future is still pending after {MAX_ROUNDS} rounds.\n\
         Either it deadlocked, or it awaited simulated time — `Timer`, a clock \
         edge, `ReadOnly`/`ReadWrite`, or anything that touches a signal. Those \
         need a real simulator: write the test as a `sim-*` case under \
         output/regression/tests/ instead."
    );
}

/// Like [`block_on`], but expects the future **not** to finish.
///
/// For the cases where blocking is the behaviour under test — a `get` on an
/// empty queue, a sequence waiting for a grant that never comes, a
/// `get_response` for a ticket nobody will answer. Returns once the run queue
/// is quiet, having proved the future is still waiting.
pub fn assert_pending<F: Future>(fut: F) {
    let ex = executor::init();
    let mut fut = Box::pin(fut);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    for _ in 0..64 {
        if fut.as_mut().poll(&mut cx).is_ready() {
            panic!("assert_pending: the future completed, but the test expected it to wait");
        }
        ex.run_until_idle();
    }
}

/// Install a fresh executor without running anything — for tests that drive
/// the executor by hand, or that only need `spawn` to be legal.
pub fn fresh_executor() -> executor::Executor {
    executor::init()
}

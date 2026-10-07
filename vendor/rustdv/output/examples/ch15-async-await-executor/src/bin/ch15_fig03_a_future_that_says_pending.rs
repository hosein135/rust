// Chapter 15, Figure 3: A future that says Pending — the trigger's whole job
// Run: cargo run --bin ch15_fig03_a_future_that_says_pending

use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll, Waker};

/// The simplest possible trigger: not ready the first time you ask,
/// ready the second time. (rustdv's NullTrigger is exactly this.)
struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref(); // "poll me again"
            Poll::Pending
        }
    }
}

fn main() {
    let mut fut = pin!(async {
        println!("before the await");
        YieldOnce { yielded: false }.await;
        println!("after the await");
    });
    let mut cx = Context::from_waker(Waker::noop());

    println!("first poll:  {:?}", fut.as_mut().poll(&mut cx));
    println!("second poll: {:?}", fut.as_mut().poll(&mut cx));
}

// Chapter 15, Figure 4: An event loop in a page
// Run: cargo run --bin ch15_fig04_an_event_loop_in_a_page

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

thread_local! {
    /// The run queue: tasks that are ready to make progress.
    static RUN_QUEUE: RefCell<VecDeque<usize>> = RefCell::new(VecDeque::new());
}

/// Yield control: reschedule myself, then say Pending once.
struct YieldNow(bool, usize);

impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            let id = self.1;
            RUN_QUEUE.with(|q| q.borrow_mut().push_back(id)); // wake: requeue
            Poll::Pending
        }
    }
}

fn main() {
    let count = |name: &'static str, n: u32, id: usize| async move {
        for i in 1..=n {
            println!("{name} counts {i}");
            YieldNow(false, id).await;
        }
    };

    // The task arena: every spawned future, boxed and pinned.
    let mut tasks: Vec<Pin<Box<dyn Future<Output = ()>>>> =
        vec![Box::pin(count("The Count", 5, 0)), Box::pin(count("Mom", 3, 1))];

    // Seed the queue, then drain it to exhaustion: the whole event loop.
    RUN_QUEUE.with(|q| q.borrow_mut().extend([0, 1]));
    let mut cx = Context::from_waker(Waker::noop());
    while let Some(id) = RUN_QUEUE.with(|q| q.borrow_mut().pop_front()) {
        let _ = tasks[id].as_mut().poll(&mut cx);
    }
    println!("run queue empty: the loop returns");
}

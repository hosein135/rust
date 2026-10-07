// Chapter 15, Figure 2: Polling a future by hand
// Run: cargo run --bin ch15_fig02_polling_a_future_by_hand
//
// Expected output:
// polling...
// the future ran
// poll returned Ready(3)

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

async fn add_one(n: u32) -> u32 {
    println!("the future ran");
    n + 1
}

fn main() {
    let mut fut = pin!(add_one(2));
    let mut cx = Context::from_waker(Waker::noop());
    println!("polling...");
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(v) => println!("poll returned Ready({v})"),
        Poll::Pending => println!("poll returned Pending"),
    }
}

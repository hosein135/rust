//! Chapter 16 simulator figures. Build and run with:
//!
//!     sim-common/run_sim.sh ch16_tasks_queues playground
//!
//! Each `#[rustdv::test]` function below is a numbered figure in the book;
//! shared coroutines (counter, producer, consumer) are figures too.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// Chapter 16, Figure 1: counter counts up with a delay
async fn counter(name: &'static str, delay: u64, count: u32) {
    // Counts up to the count argument after delay
    for ii in 1..=count {
        Timer::ns(delay).await;
        log::info(&format!("{name} counts {ii}"));
    }
}

// Chapter 16, Figure 2: Launching a task and ignoring it
#[rustdv::test]
async fn do_not_wait(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Launch a counter
    log::info("start counting to 3");
    spawn(counter("simple count", 1, 3));
    log::info("ignored the running task");
    Ok(())
}

// Chapter 16, Figure 3: Waiting for a running task
#[rustdv::test]
async fn wait_for_it(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Launch a counter
    log::info("start counting to 3");
    let running_task = spawn(counter("simple count", 1, 3));
    let _ = running_task.await;
    log::info("waited for running task");
    Ok(())
}

// Chapter 16, Figure 4: Mom and The Count count in parallel
#[rustdv::test]
async fn counters(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Test that starts two counters and waits for them
    log::info("The Count will count to five.");
    log::info("Mom will count to three.");
    let the_count = spawn(counter("The Count", 1, 5));
    let mom_warning = spawn(counter("Mom", 2, 3));
    let _ = join2(the_count, mom_warning).await;
    log::info("All the counting is finished");
    Ok(())
}

// Chapter 16, Figure 6: A coroutine that increments a number
// and returns it after a delay
async fn wait_for_numb(delay: u64, numb: u32) -> u32 {
    // Waits for delay ns and then returns the increment of the number
    Timer::ns(delay).await;
    numb + 1
}

// Chapter 16, Figure 7: Getting a return value by awaiting the TaskHandle
#[rustdv::test]
async fn inc_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Demonstrates spawn() return values
    log::info("sent 1");
    let inc1 = spawn(wait_for_numb(1, 1));
    let nn = inc1.await.expect("task was cancelled");
    log::info(&format!("returned {nn}"));
    log::info(&format!("sent {nn}"));
    let inc2 = spawn(wait_for_numb(10, nn));
    let nn = inc2.await.expect("task was cancelled");
    log::info(&format!("returned {nn}"));
    Ok(())
}

// Chapter 16, Figure 8: Cancelling a task — Rust's kill()
#[rustdv::test]
async fn cancel_a_running_task(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Cancel a running task
    let kill_me = spawn(counter("Kill me", 1, 1000));
    Timer::ns(5).await;
    kill_me.cancel();
    log::info("Cancelled the long-running task.");
    Ok(())
}

// Chapter 16, Figure 9: A coroutine using a Queue to send data
async fn producer(queue: Queue<u32>, nn: u32, delay: Option<u64>) {
    // Produce numbers from 1 to nn and send them
    for datum in 1..=nn {
        if let Some(d) = delay {
            Timer::ns(d).await;
        }
        queue.put(datum).await;
        log::info(&format!("Producer sent {datum}"));
    }
}

// Chapter 16, Figure 10: A coroutine using a Queue to receive data
async fn consumer(queue: Queue<u32>) {
    // Get numbers and print them to the log
    loop {
        let datum = queue.get().await;
        log::info(&format!("Consumer got {datum}"));
    }
}

// Chapter 16, Figure 11: An infinitely long Queue consumes no time
#[rustdv::test]
async fn infinite_queue(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show an infinite queue
    let queue = Queue::unbounded();
    spawn(consumer(queue.clone()));
    spawn(producer(queue, 3, None));
    Timer::ns(1).await;
    Ok(())
}

// Chapter 16, Figure 12: A Queue of size 1 can block when it is full
#[rustdv::test]
async fn queue_max_size_1(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show producer and consumer with a capacity of 1
    let queue = Queue::new(Some(1));
    spawn(consumer(queue.clone()));
    spawn(producer(queue, 3, None));
    Timer::ns(1).await;
    Ok(())
}

// Chapter 16, Figure 13: Demonstrating simulated time delays
// in Queue communication
#[rustdv::test]
async fn producer_consumer_sim_delay(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show producer and consumer with simulation delay
    let queue = Queue::new(Some(1));
    spawn(consumer(queue.clone()));
    let ptask = spawn(producer(queue, 3, Some(5)));
    let _ = ptask.await;
    Timer::ns(1).await;
    Ok(())
}

// Chapter 16, Figure 14: Putting objects in a Queue without blocking
async fn producer_no_wait(queue: Queue<u32>, nn: u32) {
    // Produce numbers from 1 to nn and send them
    for datum in 1..=nn {
        let mut item = datum;
        while let Err(rejected) = queue.try_put(item) {
            log::info("Queue Full, waiting 1ns");
            item = rejected;
            Timer::ns(1).await;
        }
        log::info(&format!("Producer sent {datum}"));
    }
}

// Chapter 16, Figure 15: Getting objects from a Queue without blocking
async fn consumer_no_wait(queue: Queue<u32>) {
    // Get numbers and print them to the log
    loop {
        let datum = loop {
            match queue.try_get() {
                Some(datum) => break datum,
                None => {
                    log::info("Queue Empty, waiting 2 ns");
                    Timer::ns(2).await;
                }
            }
        };
        log::info(&format!("Consumer got {datum}"));
    }
}

// Chapter 16, Figure 16: Running our nonblocking test
#[rustdv::test]
async fn producer_consumer_nowait(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show producer and consumer not waiting
    let queue = Queue::new(Some(1));
    spawn(consumer_no_wait(queue.clone()));
    producer_no_wait(queue, 3).await;
    Timer::ns(3).await;
    Ok(())
}

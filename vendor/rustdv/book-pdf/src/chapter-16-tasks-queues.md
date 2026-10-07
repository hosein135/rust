# Chapter 16: Tasks, Channels, and Sim-Aware Queues

Chapter 15 built the engine; this chapter builds the traffic. Testbenches are crowds of concurrent behaviors — a driver wiggling pins, monitors watching them, a scoreboard judging — and this chapter ports the two Python-book chapters that made such crowds manageable: launching coroutines as background *tasks*, and letting tasks talk through *queues*. It ends at the one place where the Rust model diverges from cocotb's, which is what happens when you kill a task.

> **In the UVM...** we ran things in parallel and wired them together. SystemVerilog forked processes with `fork...join_none`, reaped them with `join`/`join_any`, and killed them with `disable`; producers and consumers shared data through a `mailbox #(T)` with blocking `put()`/`get()` and nonblocking `try_put()`/`try_get()`. cocotb spelled the same ideas `start_soon()` — returning a `RunningTask` to await, ignore, or `kill()` — `Combine()`/`First()` for groups, and `cocotb.queue.Queue` for the data.

All of it is here, most of it under a light Rust accent. And thanks to Chapter 15, none of it is magic: you know what a future is and what the executor does, so `spawn` is about to be a very short story.

## Starting tasks

`rustdv::spawn()` is `cocotb.start_soon()`: hand it a future, get back a handle, and the task starts running at the next turn of the event loop. Our lab animal is the Python book's counter, returned from Sesame Street duty:

```rust
// Figure 1: counter counts up with a delay

async fn counter(name: &'static str, delay: u64, count: u32) {
    // Counts up to the count argument after delay
    for ii in 1..=count {
        Timer::ns(delay).await;
        log::info(&format!("{name} counts {ii}"));
    }
}
```

Note what `counter` is *not*: not a test, not registered with anything, no attribute above it. It is a plain `async fn` — a function that builds a future — and any task may await it or spawn it. The `&'static str` for the name is Chapter 8's string-slice economics (these names are literals living in the program binary; no `String` allocation needed).

### Ignoring a running task

First, the cautionary opener — launch it and walk away:

```rust
// Figure 2: Launching a task and ignoring it

#[rustdv::test]
async fn do_not_wait(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Launch a counter
    log::info("start counting to 3");
    spawn(counter("simple count", 1, 3));
    log::info("ignored the running task");
    Ok(())
}
```

```text
--
      0.00ns INFO     start counting to 3
      0.00ns INFO     ignored the running task
      0.00ns INFO     do_not_wait PASSED
```

Well, that was unsatisfying, the second time in two books. The counter never counted: the test returned `Ok(())` at time zero, the test manager ended the test and cancelled its surviving children, and the counter died before its first timer fired. Same lesson as Python: fire-and-forget is for free-running behavior — BFM loops, clock drivers — not for work you need finished.

### Awaiting a running task

`spawn` returns a `TaskHandle`, and a `TaskHandle` is itself a future — awaiting it means "block until that task completes," exactly like `await running_task` in cocotb 2.x:

```rust
// Figure 3: Waiting for a running task

#[rustdv::test]
async fn wait_for_it(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Launch a counter
    log::info("start counting to 3");
    let running_task = spawn(counter("simple count", 1, 3));
    let _ = running_task.await;
    log::info("waited for running task");
    Ok(())
}
```

```text
--
      0.00ns INFO     start counting to 3
      1.00ns INFO     simple count counts 1
      2.00ns INFO     simple count counts 2
      3.00ns INFO     simple count counts 3
      3.00ns INFO     waited for running task
      3.00ns INFO     wait_for_it PASSED
```

The `let _ =` deserves its sentence now, because it is not noise. Awaiting a `TaskHandle<T>` yields `Result<T, TaskError>` — because between spawn and completion, someone might *cancel* the task, and a cancelled task has no value to give. Rust will not let you silently ignore that possibility (the `Result` is `#[must_use]`); `let _ =` is you telling the compiler, visibly, that you accept either outcome. Our counter returns `()` and nobody cancels it, so discarding is right. When the value matters, handle the `Result` — which is figure 7's subject.

### Running tasks in parallel

The Count counts to five on a one-nanosecond stride; Mom counts to three on a two-nanosecond stride, with consequences implied. `Combine()` becomes `join2`:

```rust
// Figure 4: Mom and The Count count in parallel

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
```

```text
// Figure 5: Mom and The Count's interleaved output
--
      3.00ns INFO     The Count will count to five.
      3.00ns INFO     Mom will count to three.
      4.00ns INFO     The Count counts 1
      5.00ns INFO     Mom counts 1
      5.00ns INFO     The Count counts 2
      6.00ns INFO     The Count counts 3
      7.00ns INFO     Mom counts 2
      7.00ns INFO     The Count counts 4
      8.00ns INFO     The Count counts 5
      9.00ns INFO     Mom counts 3
      9.00ns INFO     All the counting is finished
```

Interleaved counting, cooperative multitasking, one thread — Chapter 15's run queue doing exactly what you watched it do, now with the simulator supplying the wake-ups.¹ `join2` waits for both tasks and returns both results as a tuple; rustdv also provides `first2` (cocotb's `First()`, SystemVerilog's `join_any`), which returns when the *first* future finishes — and, in a very Rust move, *drops* the loser. Hold that thought two sections.

> ¹ The timestamps start at 3.00ns because the chapter's tests run back-to-back in one simulation, and figure 3 ended at 3ns. The Python book trimmed such artifacts from its outputs; we keep them, because a regression is one simulation and the clock does not reset between tests.

## Returning values from tasks

Tasks can return values, and the value comes out where cocotb's did — by awaiting the handle:

```rust
// Figure 6: A coroutine that increments a number
// and returns it after a delay

async fn wait_for_numb(delay: u64, numb: u32) -> u32 {
    // Waits for delay ns and then returns the increment of the number
    Timer::ns(delay).await;
    numb + 1
}
```

```rust
// Figure 7: Getting a return value by awaiting the TaskHandle

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
```

```text
--
      9.00ns INFO     sent 1
     10.00ns INFO     returned 2
     10.00ns INFO     sent 2
     20.00ns INFO     returned 3
     20.00ns INFO     inc_test PASSED
```

One nanosecond for the first increment, ten for the second, values flowing back through `await` — figure-for-figure with the Python book. The new element is `.expect(...)`: since awaiting a handle yields `Result<u32, TaskError>`, we must say what happens if the task was cancelled out from under us. Here that would be a testbench bug, and Chapter 9's taxonomy says testbench bugs panic — `expect` is the assertion that documents it.

## Cancelling a task: the one real divergence

Now the section this chapter has owed you since its title. cocotb kills a task by throwing `CancelledError` *into* the coroutine: the task's `finally` blocks run, it can do last-wish cleanup, it can even (buggily) refuse to die. Rust has no exceptions to throw and no way to run code *inside* a future that is not being polled. Cancellation in Rust is: **the executor drops the future.** The state machine is destroyed where it stands; anything it owned is dropped with it; no task-side code runs after the drop point.

The user-facing surface barely changes:

```rust
// Figure 8: Cancelling a task — Rust's kill()

#[rustdv::test]
async fn cancel_a_running_task(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Cancel a running task
    let kill_me = spawn(counter("Kill me", 1, 1000));
    Timer::ns(5).await;
    kill_me.cancel();
    log::info("Cancelled the long-running task.");
    Ok(())
}
```

```text
--
     21.00ns INFO     Kill me counts 1
     22.00ns INFO     Kill me counts 2
     23.00ns INFO     Kill me counts 3
     24.00ns INFO     Kill me counts 4
     25.00ns INFO     Cancelled the long-running task.
     25.00ns INFO     cancel_a_running_task PASSED
```

Four counts in five nanoseconds, then silence — indistinguishable from `kill()` at this range. The divergence appears only when the dying task *owned cleanup responsibilities*, and Rust's answer has two halves.

The first half is good news, and it is most of the story: cleanup you would have written in a `finally` block moves into `Drop` implementations on whatever the task holds — Chapter 13's RAII, now applied to task death. A task holding a `LockGuard` releases the lock when cancelled, *automatically*, because dropping the future drops the guard. The whole class of cocotb bugs where a killed task forgot its `finally` — or caught `CancelledError` and failed to re-raise it, which cocotb has dedicated machinery to detect — cannot be written.

The second half is the honest loss: a cancelled Rust task cannot *await* during its last moments. cocotb code that, on kill, drove a bus back to idle over several clock cycles has no direct translation, because dropping is synchronous. The idiom that replaces it is **shutdown by message**: instead of killing the driver, send it a "stop" item through the very queues this chapter teaches (or set an `Event` it checks), and await its handle while it winds down on its own terms. Ask first, rather than shoot and clean up. We will not need the idiom for the TinyALU — its tasks are all stateless loops, safe to drop anywhere — but it is recorded here because someday your DUT will care what the bus does during the funeral.

## Task communication: the sim-aware Queue

With tasks running in parallel, they need to share data — in order, without races. The old answers were SystemVerilog's `mailbox #(T)` and cocotb's `Queue`; rustdv's is `sim::Queue<T>`: same blocking `put`/`get`, same nonblocking variants, executor-aware so that a blocked task parks itself with the executor rather than spinning.² One Rust twist up front: the queue is typed, always. A `Queue<u32>` carries `u32`s and nothing else; the Python queue carried anything — as did the default, unparameterized SV mailbox — and a producer that put the wrong thing in was the consumer's runtime problem. Here it is the producer's compile error.

```rust
// Figure 9: A coroutine using a Queue to send data

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
```

```rust
// Figure 10: A coroutine using a Queue to receive data

async fn consumer(queue: Queue<u32>) {
    // Get numbers and print them to the log
    loop {
        let datum = queue.get().await;
        log::info(&format!("Consumer got {datum}"));
    }
}
```

The optional delay came along as `Option<u64>` — Chapter 9's type for "maybe a delay," where Python used `delay=None`. Cloning a `Queue` clones a *handle* to the same shared queue (like `Rc`, Chapter 13), which is how producer and consumer end up holding the same one.

### An infinitely long queue

```rust
// Figure 11: An infinitely long Queue consumes no time

#[rustdv::test]
async fn infinite_queue(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show an infinite queue
    let queue = Queue::unbounded();
    spawn(consumer(queue.clone()));
    spawn(producer(queue, 3, None));
    Timer::ns(1).await;
    Ok(())
}
```

```text
--
     25.00ns INFO     Producer sent 1
     25.00ns INFO     Producer sent 2
     25.00ns INFO     Producer sent 3
     25.00ns INFO     Consumer got 1
     25.00ns INFO     Consumer got 2
     25.00ns INFO     Consumer got 3
```

With unbounded capacity nothing ever blocks the producer, so it runs to completion in zero simulated time and *then* the consumer drains the queue — all sends, then all receives, the classic unbounded-queue behavior in every dialect.

### A Queue of size 1

Bound the capacity at one and the two tasks are forced to alternate:

```rust
// Figure 12: A Queue of size 1 can block when it is full

#[rustdv::test]
async fn queue_max_size_1(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show producer and consumer with a capacity of 1
    let queue = Queue::new(Some(1));
    spawn(consumer(queue.clone()));
    spawn(producer(queue, 3, None));
    Timer::ns(1).await;
    Ok(())
}
```

```text
--
     26.00ns INFO     Producer sent 1
     26.00ns INFO     Consumer got 1
     26.00ns INFO     Producer sent 2
     26.00ns INFO     Consumer got 2
     26.00ns INFO     Producer sent 3
     26.00ns INFO     Consumer got 3
```

Put one, block; get one, block; ping-pong to the end. `Queue::new(Some(1))` is `Queue(maxsize=1)` with the capacity wrapped in `Option` — `Some(1)` bounded, and `Queue::unbounded()` as the readable spelling of `None`.

### Queues and simulated delay

Give the producer a five-nanosecond stride and the pattern holds while time advances:

```rust
// Figure 13: Demonstrating simulated time delays
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
```

```text
--
     32.00ns INFO     Producer sent 1
     32.00ns INFO     Consumer got 1
     37.00ns INFO     Producer sent 2
     37.00ns INFO     Consumer got 2
     42.00ns INFO     Producer sent 3
     42.00ns INFO     Consumer got 3
```

> ² "Executor-aware" is why we do not use Rust's ordinary channel types here: `std::sync::mpsc` blocks *threads*, and tokio's channels wake *tokio*. A simulation queue must park a task with *our* executor and wake it on simulated-time events. Same reason cocotb could not use `queue.Queue` from Python's standard library.

## Nonblocking communication

Sometimes a task cannot afford to block — the classic example is a loop pacing itself on clock edges, which would miss edges if `get()` parked it. SystemVerilog's escape was `try_put()`/`try_get()`; cocotb's was `put_nowait()`/`get_nowait()` plus `QueueFull`/`QueueEmpty` exceptions. rustdv keeps SystemVerilog's names, but — no exceptions, no status-integer returns — they answer in Chapter 9's vocabulary: `try_put` returns `Result<(), T>` (your item handed back on failure, so it isn't lost), and `try_get` returns `Option<T>`.

```rust
// Figure 14: Putting objects in a Queue without blocking

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
```

```rust
// Figure 15: Getting objects from a Queue without blocking

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
```

Compare the shapes with their Python originals. The `try/except QueueFull` block became `while let Err(rejected) = ...` — the failure is a *value*, and the value contains our rejected item, which we put back in `item` and retry. The `try/except QueueEmpty` became a `match` on `Option`, with `break datum` carrying the prize out of the inner loop (loops are expressions; Chapter 4 finally cashes that check). Nothing is caught, because nothing is thrown; every failure path is spelled in the signatures.

```rust
// Figure 16: Running our nonblocking test

#[rustdv::test]
async fn producer_consumer_nowait(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Show producer and consumer not waiting
    let queue = Queue::new(Some(1));
    spawn(consumer_no_wait(queue.clone()));
    producer_no_wait(queue, 3).await;
    Timer::ns(3).await;
    Ok(())
}
```

```text
--
     43.00ns INFO     Producer sent 1
     43.00ns INFO     Queue Full, waiting 1ns
     43.00ns INFO     Consumer got 1
     43.00ns INFO     Queue Empty, waiting 2 ns
     44.00ns INFO     Producer sent 2
     44.00ns INFO     Queue Full, waiting 1ns
     45.00ns INFO     Consumer got 2
     45.00ns INFO     Queue Empty, waiting 2 ns
     45.00ns INFO     Producer sent 3
     47.00ns INFO     Consumer got 3
     47.00ns INFO     Queue Empty, waiting 2 ns
     48.00ns INFO     producer_consumer_nowait PASSED
```

Note also that the test *awaits the producer directly* rather than spawning it — a coroutine you need finished before proceeding can simply be awaited, no task required.

## Two more synchronizers you'll want

Queues carry data; two lighter primitives carry *timing*, and later chapters use both, so meet them now. `sim::Event` does the job of SystemVerilog's named events and cocotb's `Event`: any number of tasks `wait().await` on it, and one `set()` releases them all — with the same subtlety cocotb documents, that a wait on an already-set event returns immediately. It is the tool for "the reset is done," "the sequence may start" — every place you were ever warned not to use a sleep. `sim::Lock` is cocotb's `Lock` and the one-key case of SystemVerilog's `semaphore`, a mutex for tasks sharing a resource (two sequences sharing one bus), with the fairness guarantee preserved: acquisition order is request order, first-come first-served. Locking returns a `LockGuard` whose `Drop` releases — which you could have predicted by now: it is the objection guard pattern, the file pattern, the RAII pattern, and before long it will simply be how you assume everything works.

## Summary

This chapter put crowds of tasks to work. `spawn()` is `start_soon()`: it schedules a future and returns a `TaskHandle`, which is itself awaitable and yields `Result<T, TaskError>` — the type admitting that tasks can be cancelled before they produce. `join2` and `first2` port `Combine` and `First`. Cancellation is the one real divergence from cocotb: `cancel()` *drops* the future rather than throwing into it, cleanup lives in `Drop` rather than `finally` (usually an upgrade — it cannot be forgotten), and behavior that must consume time while shutting down uses the shutdown-message idiom instead. `sim::Queue<T>` ports the cocotb queue: typed, cloned-by-handle, blocking `put`/`get` that park with the executor, and nonblocking `try_put`/`try_get` that answer in `Result` and `Option` instead of exceptions. `Event` and `Lock` round out the toolbox — set-and-release, and fair mutual exclusion, each with RAII where cocotb had discipline.

We have tasks; we have communication; we have an executor and a simulator underneath it all. What we have not yet touched is the *design*. Chapter 17 finally does: getting a handle to the DUT, reading and writing its signals, and waiting on its clock — simulating with rustdv-sim.

# Chapter 15: async/await and the Executor

Part I ended with a confession: you owned the whole Rust toolkit and still could not *wait*. No timer, no rising edge, no way to say "pause this task until something happens in the simulation." This chapter fixes that, and it fixes it at a level no earlier book in this series had to: by the end you will have written an event loop with your own hands, because Rust — unlike SystemVerilog or Python — hands you the syntax and lets you keep the engine.

> **In the UVM...** waiting was somebody else's engine. SystemVerilog's `@(posedge clk)` and `#2ns` compiled straight into the simulator's event wheel — the process suspends, the scheduler resumes it, and no testbench author ever sees the machinery. cocotb rebuilt the same experience in Python: coroutines defined with `async def`, the top one marked `@cocotb.test()`, triggers like `Timer(2, units="ns")` awaited against an event loop the module supplied.

Every word of that still applies. Rust's `async`/`await` is the same idea you already know — resumable functions parked until an event fires — and a rustdv test will look strikingly like a cocotb test. What differs is underneath, and the difference is the theme of this chapter: **Python's coroutines push, Rust's futures are pulled**, and Rust ships no event loop at all. cocotb had to write its own event loop because asyncio cannot block on simulator time. rustdv is in exactly the same position, and this time you get to see the machine.

## Hello, world, once more

Ceremony first. Here is the simplest possible rustdv test, and your first look at Part II's figure convention: from here to the end of the book, most examples are *simulation directories* — a chapter crate built as a library the simulator loads, run by a script, with Icarus Verilog doing the simulating.¹ The chapter crates live in the examples repository next to the playground projects you already know.

```rust
// Figure 1: Hello world as a test

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

#[rustdv::test]
async fn hello_world(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Say hello!
    log::info("Hello, world.");
    Ok(())
}
```

```text
--
      0.00ns INFO     running hello_world (1/2)  [ch15-async-await-executor/src/ch15_async_await_executor.rs:12]
      0.00ns INFO     Hello, world.
      0.00ns INFO     hello_world PASSED
```

Read it against its Python twin. `@cocotb.test()` became `#[rustdv::test]` — a decorator became an attribute, and Chapter 21 is a whole chapter about what that attribute actually does. `async def hello_world(_)` became `async fn hello_world(_ctx: RustdvCtx)`; the underscore convention for an unused argument survives with a type on it. The docstring became a comment, and the test *returns* `Result<(), TestError>` — Chapter 9's failure taxonomy, now load-bearing: `Ok(())` passes, `Err` fails, and no exception machinery is anywhere involved. The two lines with no Python twin are the imports' big brother `use rustdv::prelude::*;` (the sanctioned glob from Chapter 14) and `rustdv::vpi_bootstrap!()`, a macro that exports the entry points the simulator calls when it loads our compiled testbench. cocotb hid the equivalent plumbing inside its makefiles; Rust puts one visible line in the file, and Chapter 17 explains the loading story it belongs to.

The log line should feel like home: simulated time, level, message — the same format down to the column widths, on purpose.

## What `async` actually builds

In your old testbenches you treated suspendable processes as a given and let the simulator — or cocotb — worry about resuming them. That was the right call then, and it would be the wrong call now, because in Rust the resuming machinery is *your* code. So let's look inside.

When the Rust compiler sees `async fn`, it does not create a function that runs your code. It creates a function that returns a **state machine** — a value implementing the `Future` trait, frozen at its starting line. The `Future` trait has one method:

```rust
fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
```

`poll` means: *make as much progress as you can right now.* The answer is either `Poll::Ready(value)` — finished, here's the result — or `Poll::Pending` — parked at an `await`, ask again later. Nothing about a future is concurrent, threaded, or magical; it is a struct with a resume method, and we can drive one manually:

```rust
// Figure 2: Polling a future by hand

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
```

```text
--
polling...
the future ran
poll returned Ready(3)
```

Two observations, one per output line. First: "the future ran" printed *after* "polling..." — calling `add_one(2)` executed none of its body. An `async fn` runs only when polled, which is Python's lazy-coroutine behavior (`coro` did nothing until the event loop sent into it) with the laziness made structural. Second: this future finished in one poll, because it never awaited anything. The `pin!` and `Waker::noop` incantations are scaffolding we'll justify in a moment; what matters is that you have now personally done the executor's job.

The Python contrast is worth one more sentence, because it is the deepest engine difference between the two books. A cocotb coroutine, resumed with `coro.send(None)`, runs until it *yields a trigger outward* — the coroutine pushes the thing it's waiting for up to the scheduler. A Rust future is *polled from outside* and answers Ready or Pending. Push became pull. Everything else in this chapter follows from that inversion.

## Pending, and the waker contract

A future that never says `Pending` never waits, and waiting is our whole business. Here is the smallest future that parks itself — not ready the first time you ask, ready the second:

```rust
// Figure 3: A future that says Pending — the trigger's whole job

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
```

```text
--
before the await
first poll:  Pending
after the await
second poll: Ready(())
```

Watch the interleaving: the first poll ran the async block *up to* the `await`, hit `YieldOnce`, got `Pending`, and stopped — mid-function, state saved. The second poll resumed *at the await* and ran to the end. That suspend-and-resume is exactly what `coro.send(None)` did to a Python coroutine when it yielded a trigger; you are watching the same movie from the projectionist's booth.

Now the one new obligation. Before returning `Pending`, our future called `cx.waker().wake_by_ref()`. That `Waker` — riding along in the `Context` argument — is the pull-model's return address: a cheap handle meaning *this task can make progress again; put it back on the run queue*. The contract is: **a future that returns `Pending` must arrange for the waker to be called when it becomes ready**, or nobody will ever poll it again and it sleeps forever. `YieldOnce` fulfills the contract trivially (wake immediately: "poll me again right away"). A real trigger fulfills it meaningfully: rustdv's `Timer` hands its waker to the simulator with "call this in 2 simulated nanoseconds," and a rising-edge trigger hands its waker to a callback on the signal. If you remember cocotb's `TriggerCallback` — the little object that reschedules a task when its trigger fires — you have already met the waker wearing a Python costume.²

The scaffolding, briefly, and then we can stop noticing it. `Pin` is Rust's promise that a self-referential state machine won't be moved in memory between polls (the compiler builds futures whose fields point into themselves; moving one would tear it). The `pin!` macro makes that promise for a local. You will read past `Pin` everywhere in this book after today; the framework carries it where it must. `Waker::noop()` is a do-nothing waker, fine for hand-cranking on a workbench, useless in production, replaced by a real one in about a page.

> ¹ Instructions live in the examples repository's `README.md`, and each chapter's directory has a run script. The DUT for this chapter is a Verilog module that is deliberately, perfectly empty — we need the simulator's clock of simulated time, not its talent for hardware.
>
> ² cocotb: `_base_triggers.py`, class `TriggerCallback`. Rust: `std::task::Waker`. Same job, same lifecycle, eleven fewer lines.

## The event loop you now get to keep

Here is the sentence this chapter has been building toward: **Rust ships `async`/`await` as pure language, and ships no event loop at all.** There is no asyncio in the standard library. Production Rust services reach for a runtime crate like tokio; we will not, for the same reason cocotb never ran on asyncio's loop — a general-purpose runtime owns its event loop and blocks on OS I/O, and our events come from a simulator that insists on being in charge. An executor that serves a simulator must be a *guest*: wake some tasks, drain the queue, and give control back. cocotb wrote exactly such a loop in 82 lines of Python. rustdv's is about the same size, and the heart of it fits in one figure:

```rust
// Figure 4: An event loop in a page

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
```

```text
--
The Count counts 1
Mom counts 1
The Count counts 2
Mom counts 2
The Count counts 3
Mom counts 3
The Count counts 4
The Count counts 5
run queue empty: the loop returns
```

The Count and Mom are back,³ and they are interleaving with no threads, no simulator, and no runtime crate — just a `VecDeque` of task ids and a `while let` that drains it. Follow one bounce: The Count prints, awaits `YieldNow`, which pushes his id back onto the queue and returns `Pending`; the loop pops the next id, which is Mom's; she prints and yields the same way. Cooperative multitasking, four moving parts: a **run queue** of ready tasks, a **task arena** owning the futures, **wake** meaning "push the id onto the queue," and a **drain loop** polling until the queue is empty. When you hear "executor" for the rest of this book, this figure is the whole referent — rustdv's adds bookkeeping (task states, results, names for log messages) but no new ideas.

And the drain loop's exit condition is not a detail — it is the *interface to the simulator*. cocotb's event loop runs "to exhaustion" every time a simulator callback fires, then returns control to the simulator until the next callback. rustdv's `run_until_idle` does precisely the same. The executor is a guest in the simulator's house: it works through whatever became ready, then hands the clock back. A general-purpose runtime could never be persuaded to behave this politely, which is why we own the loop.⁴

> ³ From the Python book's figure 9, where The Count counted to five while Mom counted to three, interleaved. There they yielded to the scheduler by awaiting timers; here, by explicit `YieldNow`. Chapter 16 restores their timers.
>
> ⁴ The full chain, which Chapter 17 walks: simulator callback fires → trigger wakes its subscribers (ids onto the run queue) → `run_until_idle` drains the queue → control returns to the simulator, which advances simulated time to the next event.

## Awaiting simulated time

With the engine understood, the rest of the chapter is a homecoming. Every language in this series has a way to say "consume simulated time," and two of them deserve reprinting exactly, because they have not changed and neither has the point:

```text
# Figure 5: VHDL waits for 2 nanoseconds

process is
begin
  wait for 2 ns;
  report "I am DONE waiting!";
  wait;
end process;
```

```text
# Figure 6: SystemVerilog waits for two nanoseconds

initial begin
  #2ns;
  $display("I am DONE waiting!");
end
```

A VHDL process, a SystemVerilog task, a Python coroutine — and now a Rust future. Time-consuming behavior, expressed as code that suspends mid-body:

```rust
// Figure 7: Rust waits for 2 nanoseconds

#[rustdv::test]
async fn wait_2ns(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Waits for two ns then prints
    Timer::ns(2).await;
    log::info("I am DONE waiting!");
    Ok(())
}
```

```text
--
      0.00ns INFO     running wait_2ns (2/2)  [ch15-async-await-executor/src/ch15_async_await_executor.rs:20]
      2.00ns INFO     I am DONE waiting!
      2.00ns INFO     wait_2ns PASSED
```

The test started at 0.00ns and logged at 2.00ns: two nanoseconds of *simulated* time passed, no wall-clock sleeping involved. `Timer(2, units="ns")` became `Timer::ns(2)` — a constructor per unit rather than a string argument, so `Timer::ns`, `Timer::us`, `Timer::ms` are distinct functions and a typo'd unit string is impossible rather than discovered at runtime. And you now know precisely what that innocent `.await` did: the test's future returned `Pending`, `Timer` handed its waker to the simulator with instructions for 2ns hence, the executor's queue ran dry, control went back to the simulator, simulated time advanced, the callback fired, the waker requeued the test, and the drain loop polled it awake on the far side of the await. Ten steps, all of which you have now either written or watched.

One habit to carry forward: `Timer` is for *modeling time*, never for synchronization. Every dialect has scars behind that rule — SystemVerilog testbenches paced by `#delay` guesses, cocotb's NullTrigger race — and "no sleeps for coordination" survives translation intact. Chapter 16's queues and events are the right tools, exactly as their ancestors were.

## Summary

This chapter opened the box every earlier dialect kept sealed. Rust's `async fn` compiles to a state machine implementing `Future`, whose one method `poll` answers `Ready` or `Pending` — the push-a-trigger-out model of cocotb and the simulator's event wheel, inverted into a pull. A future that returns `Pending` owes the executor a wake-up call, delivered through the `Waker` riding in every poll — cocotb's `TriggerCallback`, standardized into the language. Rust ships no event loop, which stopped being bad news the moment we wrote one in a page: a run queue, a task arena, wake-as-requeue, and a drain loop that returns control to whoever owns the events — for us, always, the simulator. On top of that engine, the user-facing surface came home unchanged: `#[rustdv::test]` marks the top-level coroutine, `Timer::ns(2).await` consumes simulated time, and the log reads like the log always read.

What we built by hand today, `rustdv-sim` provides for keeps: a real spawner, task handles you can await and cancel, and the sim-aware queues that make producer/consumer testbenches safe. That is Chapter 16 — where The Count gets his timer back, and where we meet the one place Rust's task model diverges from cocotb's, in the matter of killing a task.

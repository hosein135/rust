# Chapter 16: Tasks, Channels, and Sim-Aware Queues — figure map

All figures are simulator figures in `src/ch16_tasks_queues.rs`; run the chapter with:

```
sim-common/run_sim.sh ch16_tasks_queues playground
```

| Figure | Title | Where |
|---|---|---|
| 1 | counter counts up with a delay | `counter` |
| 2 | Launching a task and ignoring it | `do_not_wait` |
| 3 | Waiting for a running task | `wait_for_it` |
| 4 | Mom and The Count count in parallel | `counters` |
| 5 | Mom and The Count's interleaved output | output of figure 4 |
| 6 | A coroutine that increments a number and returns it after a delay | `wait_for_numb` |
| 7 | Getting a return value by awaiting the TaskHandle | `inc_test` |
| 8 | Cancelling a task — Rust's kill() | `cancel_a_running_task` |
| 9 | A coroutine using a Queue to send data | `producer` |
| 10 | A coroutine using a Queue to receive data | `consumer` |
| 11 | An infinitely long Queue consumes no time | `infinite_queue` |
| 12 | A Queue of size 1 can block when it is full | `queue_max_size_1` |
| 13 | Demonstrating simulated time delays in Queue communication | `producer_consumer_sim_delay` |
| 14 | Putting objects in a Queue without blocking | `producer_no_wait` |
| 15 | Getting objects from a Queue without blocking | `consumer_no_wait` |
| 16 | Running our nonblocking test | `producer_consumer_nowait` |

All 9 tests end `REGRESSION: PASS` (RUSTDV_RANDOM_SEED=1).

# Chapter 19: TinyAluBfm

Testbench 1.0 worked, and it is unmaintainable — one loop doing five jobs: signal-level communication, stimulus, checking, coverage, reporting. The diagnosis both earlier books delivered holds without translation: copy-and-modify testbenches lead "only to frustration and tears." The cure starts here, by extracting the lowest layer — everything that touches a pin — into a **bus functional model**.

> **In the UVM...** the BFM owned the pins. SystemVerilog built it as an interface with tasks — the Primer's move, made in its third chapter. Python built `TinyAluBfm` as a *singleton* class holding three queues and three forever-loops on the falling clock edge: `cmd_driver` drove commands from a queue, `cmd_mon` captured `(A, B, op)` tuples when `start` rose, and `result_mon` captured `result` when `done` rose. Either way, tests talked only to `reset()`, `send_op()`, `get_cmd()`, and `get_result()`, and never touched a signal again.

Same design here — the three loops, the three queues, the four-method surface — with one architectural change we should discuss up front, because it is this chapter's Rust lesson.

## Where the singleton went

The Python `TinyAluBfm` was a singleton for a sensible reason: one TinyALU, therefore one BFM, and Python's easiest way to guarantee "the same object wherever you ask for it" was `metaclass=Singleton`. But look at what the singleton was really *doing*: providing shared access to a single owner of the DUT pins. That is an ownership sentence, and Rust has ownership in the type system. The rustdv BFM is a plain struct; the test creates exactly one and shares it as `Rc<TinyAluBfm>` — Chapter 13's counted handle, passed to whoever needs it. Sharing is explicit in the type instead of ambient in a global, which pays off the day your *next* DUT has two identical bus interfaces: two BFMs, two `Rc`s, no singleton to un-design.¹

The BFM lives in `tinyalu_utils`, an ordinary library crate that every remaining testbench version lists in its `[dependencies]` — where Python smuggled the equivalent module in through `sys.path` (Chapter 14's payoff, part two: `Ops`, `alu_prediction`, and `get_int` moved in with it).

> ¹ pyuvm's own documentation reached a similar conclusion over the years; singletons make testbenches easy to write and hard to reuse. Rust simply makes the reusable version the path of least resistance.

## Living on the clock edge

Digital systems act on clock edges. The TinyALU works on the rising edge, so — the discipline from Chapter 17 — the BFM's loops all set and sample on the *falling* edge, and every one of them is a variation on one skeleton:

```rust
// Figure 1: Every BFM loop lives on the falling edge

loop {
    clk.falling_edge().await;
    // ... check signals and do the work
}
```

## The struct and its queues

```rust
// Figure 2: The TinyAluBfm struct — one owner of the pins

pub struct TinyAluBfm {
    clk: LogicHandle,
    reset_n: LogicHandle,
    start: LogicHandle,
    done: LogicHandle,
    a: LogicHandle,
    b: LogicHandle,
    op: LogicHandle,
    result: LogicHandle,
    driver_queue: Queue<(u8, u8, Ops)>,
    cmd_mon_queue: Queue<CmdTuple>,
    result_mon_queue: Queue<u64>,
}
```

```rust
// Figure 3: Initializing the TinyAluBfm

impl TinyAluBfm {
    pub fn new(dut: &HierarchyHandle) -> Result<TinyAluBfm, HandleError> {
        Ok(TinyAluBfm {
            clk: dut.signal("clk")?,
            reset_n: dut.signal("reset_n")?,
            start: dut.signal("start")?,
            done: dut.signal("done")?,
            a: dut.signal("A")?,
            b: dut.signal("B")?,
            op: dut.signal("op")?,
            result: dut.signal("result")?,
            driver_queue: Queue::new(Some(1)),
            cmd_mon_queue: Queue::unbounded(),
            result_mon_queue: Queue::unbounded(),
        })
    }
```

Read the queue capacities against the Python `__init__`, because they carry the same design decisions: the **driver queue** holds one command (`maxsize=1`), so `send_op` blocks until the driver has taken the previous command — backpressure straight from Chapter 16's size-1 queue figure. The two **monitor queues** are unbounded, because monitors must never stall the bus they observe; they publish with the nonblocking `try_put` and let the consumer catch up on its own schedule. And note the eight `?` marks: the BFM's constructor resolves every signal it will ever touch, so a renamed port fails the test at time zero, by name, before a single edge — testbench 1.0 scattered those lookups through the loop; the BFM concentrates them where they can fail early.

The queue types make one more Python-invisible decision visible. The driver queue carries `(u8, u8, Ops)` — inputs the *testbench* creates, so they are typed at the source. The command-monitor queue carries `CmdTuple`, an alias for `(u64, u64, u64)` — values read *off the buses*, whose interpretation is the consumer's job, exactly as the Python monitor's tuples were raw integers. The moment where raw becomes typed is in the test, and it is figure 16's punchline.

## reset()

```rust
// Figure 4: Centralizing the reset function

    pub async fn reset(&self) {
        self.clk.falling_edge().await;
        self.reset_n.set_u64(0);
        self.start.set_u64(0);
        self.a.set_u64(0);
        self.b.set_u64(0);
        self.op.set_u64(0);
        self.clk.falling_edge().await;
        self.reset_n.set_u64(1);
        self.clk.falling_edge().await;
    }
```

Line for line the Python `reset()`. Every future test resets the DUT with one await, and when the reset sequence someday grows a step, it grows in one place.

## The three loops

The BFM's loops all share the classic skeleton — `loop { clk.falling_edge().await; ...work... }` — living on the falling edge because the DUT lives on the rising one. The monitors first, since they are simpler. Each is a private method returning the future its loop runs; keep an eye on how the loops get *access* to the pins:

```rust
// Figure 5: Monitoring the result bus

    fn result_mon(&self) -> impl std::future::Future<Output = ()> + use<> {
        let (clk, done, result) = (self.clk, self.done, self.result);
        let queue = self.result_mon_queue.clone();
        async move {
            let mut prev_done = 0;
            loop {
                clk.falling_edge().await;
                let dn = get_int(&done);
                if prev_done == 0 && dn == 1 {
                    let _ = queue.try_put(get_int(&result));
                }
                prev_done = dn;
            }
        }
    }
```

The protocol logic is the classic monitor's exactly: remember `prev_done`, and when `done` goes 0→1 across two falling edges, `result` is valid — capture it, publish it nonblockingly. The Rust texture is in the two lines before `async move`. A spawned task must own what it uses (`'static`, Chapter 12's `move` closures), and it cannot borrow `self`, which the executor might outlive. The `+ use<>` bound says that the returned future captures none of the method's input lifetimes. The method *copies out* what the loop needs instead: `LogicHandle`s are cheap `Copy` types (they are IDs into the simulator), and cloning a `Queue` clones a handle to the shared queue (Chapter 16). The loop owns its working set outright — which is also why no other task can race it for `prev_done`.

```rust
// Figure 6: Monitoring the command signals

    fn cmd_mon(&self) -> impl std::future::Future<Output = ()> + use<> {
        let (clk, start, a, b, op) = (self.clk, self.start, self.a, self.b, self.op);
        let queue = self.cmd_mon_queue.clone();
        async move {
            let mut prev_start = 0;
            loop {
                clk.falling_edge().await;
                let st = get_int(&start);
                if st == 1 && prev_start == 0 {
                    let cmd_tuple = (get_int(&a), get_int(&b), get_int(&op));
                    let _ = queue.try_put(cmd_tuple);
                }
                prev_start = st;
            }
        }
    }
```

Same shape, opposite edge of the protocol: `start` going 0→1 means the command buses are valid.

The driver is the 1.0 loop's send-side, verbatim in spirit:

```rust
// Figure 7: Driving commands on the falling edge of clk

    fn cmd_driver(&self) -> impl std::future::Future<Output = ()> + use<> {
        let (clk, start, done) = (self.clk, self.start, self.done);
        let (a, b, op) = (self.a, self.b, self.op);
        let queue = self.driver_queue.clone();
        async move {
            start.set_u64(0);
            a.set_u64(0);
            b.set_u64(0);
            op.set_u64(0);
            loop {
                clk.falling_edge().await;
                let st = get_int(&start);
                let dn = get_int(&done);
                if st == 0 && dn == 0 {
                    // Figure 8: Drive a command when the bus is idle

                    match queue.try_get() {
                        Some((aa, bb, opr)) => {
                            a.set_u64(aa as u64);
                            b.set_u64(bb as u64);
                            op.set_u64(opr as u64);
                            start.set_u64(1);
                        }
                        None => continue,
                    }
                } else if st == 1 {
                    // Figure 9: If start is 1 check done

                    if dn == 1 {
                        start.set_u64(0);
                    }
                }
            }
        }
    }
```

Python's `try: get_nowait() ... except QueueEmpty: continue` became the `match` on `Option` — same protocol, no exception. And notice what the driver *doesn't* do: it never reads `result`. Driving is its whole job; results belong to `result_mon`. Being able to ignore the rest of the testbench is what modularity buys.

## Starting the loops, and talking to them

```rust
// Figure 10: Start the BFM tasks

    pub fn start_tasks(&self) {
        spawn_named(self.cmd_driver(), "bfm.cmd_driver");
        spawn_named(self.cmd_mon(), "bfm.cmd_mon");
        spawn_named(self.result_mon(), "bfm.result_mon");
    }
```

`start_tasks` is an ordinary function, not an `async fn` — like Python's `start_tasks()`, it consumes no simulated time; it just enqueues three tasks (`spawn_named` is `spawn` with a name for the log). These are the legitimate fire-and-forget loops from Chapter 16.

The public face is four awaitables:

```rust
// Figure 11: The get_cmd() coroutine returns the next command

    pub async fn get_cmd(&self) -> CmdTuple {
        self.cmd_mon_queue.get().await
    }

// Figure 12: The get_result() coroutine returns the next result

    pub async fn get_result(&self) -> u64 {
        self.result_mon_queue.get().await
    }

// Figure 13: send_op puts the command into the command Queue

    pub async fn send_op(&self, aa: u8, bb: u8, op: Ops) {
        self.driver_queue.put((aa, bb, op)).await;
    }
```

Every method takes `&self` — the BFM's whole public surface is read-shaped borrowing, the interior queues handling the mutation. That is the property that lets `Rc<TinyAluBfm>` work without a `RefCell` in sight, as Chapter 13 promised it would.

## The test, rewritten on top

```rust
// Figure 14: Starting a test by resetting the DUT
// and starting the BFM tasks

#[rustdv::test]
async fn test_alu(ctx: RustdvCtx) -> Result<(), TestError> {
    // Test all TinyALU operations through the BFM
    let mut rng = ctx.rng();
    let mut passed = true;
    // The RTL self-clocks (tinyalu.sv); the BFM only waits on edges.
    let bfm = Rc::new(TinyAluBfm::new(&ctx.dut())?);
    bfm.reset().await;
    bfm.start_tasks();

    let mut cvg: HashSet<Ops> = HashSet::new();
```

```rust
// Figure 15: Creating a command and sending it

    for op in Ops::ALL {
        let aa = rng.u8();
        let bb = rng.u8();
        bfm.send_op(aa, bb, op).await;

// Figure 16: Wait to get the command from the DUT
// and store it in the coverage set

        let seen_cmd = bfm.get_cmd().await;
        let seen_op = Ops::from_u64(seen_cmd.2).expect("illegal op on the bus");
        cvg.insert(seen_op);

// Figure 17: Wait for the result, then create a prediction

        let result = bfm.get_result().await as u16;
        let pr = alu_prediction(aa, bb, op);

// Figure 18: Check the result against the predicted result

        if result == pr {
            log::info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {result:04x}"));
        } else {
            log::error(&format!(
                "FAILED: {aa:02x} {op:?} {bb:02x} = {result:04x} - predicted {pr:04x}"
            ));
            passed = false;
        }
    }
```

Compare this loop with Chapter 18's. No falling edges, no `start`/`done` bookkeeping, no protocol states — a `for` over the ops, a send, two gets, a compare. The signal-level grime is *gone from the test*, which reads at the level a test should: operations, predictions, verdicts.

Two Python-book points survive intact and deserve their reprise. First, we read the command back from the monitor even though we just created it — because an independent read catches driver bugs, and because a future testbench may watch a TinyALU it doesn't drive. Second, the monitor gave us raw integers, and `Ops::from_u64(seen_cmd.2)` is the boundary where the raw bus value must prove it is a legal operation — Python's `Ops(seen_cmd[2])` raising on garbage, become an `Option` we `expect` on, since an illegal opcode on the bus here means our own driver broke: testbench bug, panic, per the taxonomy.

The coverage check and final `Result` close the test exactly as 1.0 did, and:

```text
# Figure 19: Another successful test
--
     50.00ns INFO     PASSED: c1 Add 67 = 0128
     70.00ns INFO     PASSED: 5e And 0b = 000a
     90.00ns INFO     PASSED: b9 Xor 80 = 0039
    140.00ns INFO     PASSED: a5 Mul 75 = 4b69
    140.00ns INFO     Covered all operations
    140.00ns INFO     test_alu PASSED
```

Same seed, same operands, same results as Chapter 18's transcript — `c1 Add 67` and friends — ten nanoseconds later apiece, the cost of the queue hop between test and driver. Two testbenches, one behavior, and the second one you could hand to a teammate.

## Summary

This chapter extracted testbench 1.0's signal-level layer into `TinyAluBfm`: three falling-edge loops (driver, command monitor, result monitor) around three queues (bounded driver queue for backpressure, unbounded monitor queues so observation never stalls the bus), fronted by `reset()`, `send_op()`, `get_cmd()`, and `get_result()`. The Python singleton became a plain struct shared as `Rc` — ownership machinery doing the "exactly one, available to all" job the metaclass did, without foreclosing on a second instance. Spawned loops copy out the handles they need and own their state; the constructor front-loads every signal lookup behind `?`; and the test now reads as stimulus-predict-compare with no pins in it.

The test is still doing three jobs, though — generating, checking, covering — and version 2.0 splits *those* apart: driver, monitors, scoreboard as structs with methods, wired by hand, so we can feel precisely the plumbing pain that the UVM chapters exist to remove. Onward.

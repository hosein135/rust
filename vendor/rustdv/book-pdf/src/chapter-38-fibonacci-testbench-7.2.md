# Chapter 38: Fibonacci Testbench: 7.2

Chapter 37 taught the response mechanism on a device that did nothing but wait. Testbench 7.2 puts it to work on the real DUT, computing the most traditional dependent stimulus there is: **the TinyALU produces the Fibonacci numbers**, and it cannot be given the next pair of operands until it has answered the last one.

```text
0  1  1  2  3  5  8  13  21
```

Each command's operands are the answers to the two before it. There is no way to generate this stimulus in advance — no sequence of random values, no precomputed list that stays honest — it must be written one command at a time, with the DUT's answer in hand. That is the reason the sequence system exists, reduced to nine lines of `body`.

> **In the UVM...** both source books compute Fibonacci the shared-handle way: the driver gets the sum from the BFM and writes it *into the sequence item*, and the sequence — still holding a handle to the same object — reads `cmd.result` when `finish_item` returns. The 7.2 variant sent a separate response instead: the driver called `rsp.set_id_info(req)` and `item_done(rsp)`, and the sequence awaited `get_response()`.

## A driver that waits for the answer

```rust
// Chapter 38, Figure 1: The driver sends a command and returns its result
#[derive(Component, Default)]
struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
    #[port(publish)]
    result_ap: PublishPort<u64>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            let cmd = item.payload();
            bfm.send_op(cmd.a, cmd.b, cmd.op).await;
            let result = bfm.get_result().await;
            self.result_ap.write(&result);
            self.seq_item_port
                .item_done(Some(AluResult { result: result as u16 }));
        }
    }
}
```

Chapter 36's driver fired and forgot. This one waits for *this* operation's answer before taking another item, and hands the answer back through `item_done(Some(...))`. The framework tags the response with the command's ticket automatically — nothing here performs `set_id_info`, the call a UVM driver could forget and whose absence was a run-time fatal. And note the analysis port: since this driver already holds every answer, it publishes results itself. The reason unfolds in figure 3.

## The Fibonacci sequence

```rust
// Chapter 38, Figure 2: Nine numbers, eight of them from the DUT
#[derive(Default)]
struct FibonacciSeq;

impl Sequence for FibonacciSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let mut prev: u8 = 0;
        let mut cur: u8 = 1;
        let mut fib = vec![prev as u16, cur as u16];

        for _ in 0..7 {
            let mut cmd = AluCommand { a: 0, b: 0, op: Ops::Add };
            ctx.start_item(&mut cmd).await?;
            cmd.a = prev;
            cmd.b = cur;
            let ticket = ctx.finish_item(cmd).await?;
            let sum = ctx.get_response(Some(ticket)).await.result;

            fib.push(sum);
            prev = cur;
            cur = sum as u8;
        }

        ctx.info(&format!("Fibonacci Sequence: {fib:?}"));
        Ok(())
    }
}
```

Read the middle three lines as one gesture: hand the command over, wait, take the answer. `cmd` moves at `finish_item` — the sequence has no further use for it, so nothing is cloned; a sequence that *did* want to keep the command it sent would write `finish_item(cmd.clone())`, and the compiler would insist if it forgot. The ticket comes back from `finish_item`, and `get_response(Some(ticket))` claims this command's answer.

Two things are worth saying about that `Some(ticket)`, since Chapter 37 is fresh. First, with one command in flight at a time — and Fibonacci *cannot* pipeline; the dependency forbids it — the ticket is never ambiguous, so passing it is documentation rather than necessity. It is good documentation: `get_response(Some(id))` says what you mean, and it keeps working if a later testbench pipelines the same sequence. Asking for "whatever comes next" is right only for as long as there is only one thing coming. Second, for the UVM reader waiting for the shared-handle miracle — the driver writing `cmd.result` and the sequence reading it back out of the object it still holds — that move does not exist here, and nothing is missing. Writing into a shared handle is simply what returning a value looks like in a language where two names can point at one object. Rust has one owner, so the answer comes back as its *own value*, one line later either way.

A small pleasure, in passing: the sequence logs under its own name. pyuvm's sequences cannot — `uvm_sequence` is not a `uvm_report_object`, so its Fibonacci reaches for `uvm_root().logger` — and the Primer's uses a hand-typed string id. `ctx.info` in a sequence body is attributed like everything else.

## The environment: no result monitor

```rust
// Chapter 38, Figure 3: No result monitor
#[derive(Component, Default)]
struct FibEnv {
    #[component]
    seqr: Sequencer<AluCommand, AluResult>,
    #[component]
    driver: RustdvComp,
    #[component]
    result_bus: AnalysisBus<u64>,
    #[component]
    watcher: RustdvComp,
}

impl Component for FibEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());
        self.driver = Driver::new_comp();
        self.result_bus = AnalysisBus::new();
        self.watcher = ResultWatcher::new_comp();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);
        self.result_bus.pub_export().connect(&self.driver, Driver::RESULT_AP);
        self.result_bus.sub_export().connect(&self.watcher, ResultWatcher::INPUT);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

The `ResultMonitor` of testbenches 6.0 and 7.0 is gone, and the reason is worth a moment because it is an architecture decision you will face on real projects. This driver already awaits every answer — it is the component that *has* the result. A separate result monitor would be a second reader drawing from the same BFM queue, and the two would take turns stealing results from each other. So the driver publishes on its own analysis port: `result_bus.pub_export()` connects to the *driver*. Whoever holds the data publishes it; observation follows the data, not the org chart.

```rust
// Chapter 38, Figure 4: A subscriber checks the DUT actually added
#[derive(Component, Default)]
struct ResultWatcher {
    #[port(subscribe)]
    input: SubscribePort<u64>,
    seen: RustdvShared<SeenResults>,
}

impl Component for ResultWatcher {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.input.subscribe(self.seen.clone());
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let seen = self.seen.get();
        let expected: Vec<u64> = vec![1, 2, 3, 5, 8, 13, 21];
        if seen.results == expected {
            ctx.info(&format!("adder produced {:?}", seen.results));
        } else {
            errors.error(format!("expected {expected:?}, saw {:?}", seen.results));
        }
    }
}
```

The sequence proves the numbers are Fibonacci; the watcher proves they came from the *adder*, rather than from the sequence's own arithmetic. A subscriber with seven expected values is a small scoreboard, and it closes the loop a self-checking testbench needs.

```rust
// Chapter 38, Figure 5: The test
#[rustdv::test]
#[derive(Component, Default)]
struct FibonacciTest {
    #[component]
    env: RustdvComp,
}

impl Component for FibonacciTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = FibEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("computing Fibonacci");
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(Some(ctx), "", "SEQR")?;
        let mut seq = FibonacciSeq::default();
        seq.start(&seqr).await?;
        Ok(())
    }
}
```

No flush. Chapters 34 and 36 held their objections through twenty falling edges because their drivers fired and forgot; this one's `finish_item` does not return until the driver has the answer, so when the sequence ends, nothing is in the pipeline. The twenty-clock wait was a property of a driver that does not wait — not of sequences, and not of the DUT.

```text
# Figure 6: The TinyALU computes Fibonacci

      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running FibonacciTest (1/1)  [ch38-fibonacci-testbench-7.2/src/ch38_fibonacci_testbench_7_2.rs:228]
    170.00ns INFO     [FibonacciSeq]: Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]
    170.00ns INFO     [FibonacciTest.env.watcher]: adder produced [1, 2, 3, 5, 8, 13, 21]
    170.00ns INFO     FibonacciTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** FibonacciTest                                PASS         170.00      **
******************************************************************************
REGRESSION: PASS
```

## Summary

Dependent stimulus is what sequences are for, and testbench 7.2 is the classic case: nine numbers, eight computed by the DUT, each command written only after the previous answer arrived. The driver awaits each result and returns it through `item_done(Some(...))`, ticketed automatically; the sequence claims it with `get_response(Some(ticket))` — documentation today, correctness the day the sequence is pipelined. The result monitor is gone because the driver holds the results, and observation follows the data. And the flush is gone because a driver that waits leaves nothing in flight — end-of-test bookkeeping got simpler by making the driver more honest about time.

One rung remains on the ladder both earlier books climbed: sequences that run other sequences, and the testbench that becomes a programming interface. Testbench 8.0 is next.

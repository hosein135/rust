# Chapter 39: Virtual Sequence Testbench: 8.0

The last three chapters built sequences that send items. This one builds sequences that send nothing at all. A **virtual sequence** is started without a sequencer; it sends no items of its own; it *starts other sequences*. That is the whole of the idea, and it is what lets a test be assembled from stimulus that already exists rather than written again. Testbench 8.0 is where both earlier books ended their climbs, and it ends this one's: after this chapter, the machinery is complete.

> **In the UVM...** we wrote a `TestAllSeq` extending `uvm_sequence` whose `body()` fetched the sequencer from the config database and ran `rand_seq.start(seqr)` then `max_seq.start(seqr)`; the test started the virtual sequence *without* a sequencer argument. A parallel variant forked the sub-sequences and joined them.

## A program that runs programs

```rust
// Chapter 39, Figure 1: A virtual sequence starts other sequences
#[derive(Default)]
struct TestAllSeq;

impl Sequence for TestAllSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(None, "", "SEQR")?;
        RandomSeq::default().start(&seqr).await?;
        MaxSeq::default().start(&seqr).await?;
        ctx.info("ran random, then max");
        Ok(())
    }
}
```

Three observations, in rising order of importance.

- The body finds its sequencer the same way a test does — in the ConfigDb, with the `None` context, because a sequence has no path to offset from. That was the reason Chapter 27 kept the null-context form.
- There is no `start_item` and no `finish_item`. Nothing here touches an item; `RandomSeq` and `MaxSeq` do their own item handling exactly as they did in Chapter 36, unchanged.
- It is the *same* `Sequence` trait. Nothing marks this sequence "virtual" except what it does — precisely as in SystemVerilog, where `runall_sequence extends uvm_sequence #(uvm_sequence_item)` and simply never sends one.

The test that starts it:

```rust
// Chapter 39, Figure 2: The test starts the virtual sequence — no sequencer
#[rustdv::test]
#[derive(Component, Default)]
struct AluTest {
    #[component]
    env: RustdvComp,
}

impl Component for AluTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("running the virtual sequence");
        create_seq::<TestAllSeq>().start_virtual().await?;
        Ok(())
    }
}
```

`start_virtual()` takes no sequencer, because a virtual sequence has none to take — everything it drives, it drives through sequencers it looked up itself. Why a second method rather than pyuvm's single `start` with an optional argument? Rust has no default arguments, so the choice was between `start(Some(&seqr))` at every ordinary call site — noise that says nothing — or `start(None)` at the virtual ones, where `None` fails to say "virtual." Two names, each meaning what it says.

One design question deserves an answer here, because a Rust-minded reader will already have asked it: *why not a separate `VirtualSequence` trait?* It would make calling `start_item` inside a virtual sequence a compile error instead of the run-time error pyuvm gives. It would also forbid a shape the UVM allows: *The UVM Primer*'s `parallel_sequence` is started *with* a sequencer and is still virtual in the sense that matters, and nothing stops a sequence from sending some items itself and delegating the rest. Two traits would buy a better error message at the cost of a capability, and the framework declines that trade, knowingly. Late binding keeps its options; the error, if you make it, arrives at run time with a name on it.

```text
# Figure 3: Random, then max

      0.00ns INFO     running AluTest (1/3)  [ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs:440]
    250.00ns INFO     [TestAllSeq]: ran random, then max
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff And ff = 00ff
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    250.00ns INFO     [AluTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    250.00ns INFO     [AluTest.env.scoreboard]: Covered all operations
    250.00ns INFO     AluTest PASSED
```

## The same two sequences, at the same time

```rust
// Chapter 39, Figure 4: Running sub-sequences in parallel
#[derive(Default)]
struct TestAllParallelSeq;

impl Sequence for TestAllParallelSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(None, "", "SEQR")?;
        let mut random = RandomSeq::default();
        let mut max = MaxSeq::default();

        let (a, b) = join2(random.start(&seqr), max.start(&seqr)).await;
        a?;
        b?;
        ctx.info("ran random and max together");
        Ok(())
    }
}
```

`join2` is the `fork...join` you met in Chapter 16, doing here what `fork`/`join` does in every SystemVerilog virtual sequence: both sub-sequences run, and `body` continues when both are done. The sequencer arbitrates between the two live streams — FIFO order, one item each in turn — so the transcript alternates random operands with `0xff` operands, both patterns interleaved on one DUT.

One line of reasoning behind `join2` rather than `spawn`: a spawned task must own everything it touches (`'static` — the rule from Chapter 16), and a sub-sequence that borrows the parent sequence's state cannot promise that. Composing the two futures in place costs nothing and keeps that door open. The test differs from figure 2 by exactly one line — it starts `TestAllParallelSeq` instead — and is not worth a listing.

```text
# Figure 5: The two sequences interleave at the sequencer

    250.00ns INFO     running ParallelTest (2/3)  [ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs:461]
    500.00ns INFO     [TestAllParallelSeq]: ran random and max together
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: c1 Add 67 = 0128
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff Add ff = 01fe
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: 5e And 0b = 000a
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff And ff = 00ff
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff Xor ff = 0000
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    500.00ns INFO     [ParallelTest.env.scoreboard]: PASSED: ff Mul ff = fe01
    500.00ns INFO     [ParallelTest.env.scoreboard]: Covered all operations
    500.00ns INFO     ParallelTest PASSED
```

## The testbench becomes a programming interface

The chapter's payoff is not running two canned sequences — it is what virtual sequences make possible for the *next* person on the team: an interface. First, one operation as a sequence:

```rust
// Chapter 39, Figure 6: One operation, as a sequence
struct OpSeq {
    a: u8,
    b: u8,
    op: Ops,
    result: Option<u16>,
}

impl Sequence for OpSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let mut cmd = AluCommand { a: self.a, b: self.b, op: self.op };
        ctx.start_item(&mut cmd).await?;
        let ticket = ctx.finish_item(cmd).await?;
        self.result = Some(ctx.get_response(Some(ticket)).await.result);
        Ok(())
    }
}
```

`OpSeq` carries parameters, so it is constructed the ordinary way rather than through the factory — the factory's makers take no arguments, and both source books build their parameterized sequences by hand for the same reason. Its body is Chapter 38 in miniature: send one command, claim its response by ticket, store the answer.

```rust
// Chapter 39, Figure 7: The TinyALU programming interface
async fn do_op(
    seqr: &Sequencer<AluCommand, AluResult>,
    a: u8,
    b: u8,
    op: Ops,
) -> Result<u16, SeqError> {
    let mut seq = OpSeq { a, b, op, result: None };
    seq.start(seqr).await?;
    seq.result.ok_or_else(|| SeqError::from("the driver returned no result"))
}

async fn do_add(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::Add).await
}
async fn do_and(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::And).await
}
async fn do_xor(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::Xor).await
}
async fn do_mul(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::Mul).await
}
```

This is the payoff. A test writer who has never opened the testbench gets four functions that take numbers and return numbers; sequencer, driver, handshake, and response envelope are all behind them. (In Python these read `seq.result` after `start` returns, because a coroutine cannot hand a value back through `start`; here the function returns what it computed, because that is what functions do.)

And with an interface in hand, a test is just a program:

```rust
// Chapter 39, Figure 8: Fibonacci, written as a program
#[derive(Default)]
struct FibonacciProgramSeq;

impl Sequence for FibonacciProgramSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(None, "", "SEQR")?;
        let mut prev: u8 = 0;
        let mut cur: u8 = 1;
        let mut fib = vec![prev as u16, cur as u16];

        for _ in 0..7 {
            let sum = do_add(&seqr, prev, cur).await?;
            fib.push(sum);
            prev = cur;
            cur = sum as u8;
        }

        ctx.info(&format!("Fibonacci Sequence: {fib:?}"));
        Ok(())
    }
}
```

Compare this against Chapter 38's Fibonacci, where the handshake was visible at every step. The computation is identical; the sequence machinery has vanished into `do_add`. This is what a programming interface is for, and why a team that writes tests but not testbenches wants one.

Its test sets one extra ConfigDb value — `ConfigDb::set(None, "*", "CHECK_COVERAGE", false)` — because a program that only adds will never cover four operations, and the scoreboard reads that flag at check time. A test changing what the scoreboard demands, through the database, without touching it: the whole book's runtime-binding half, in one line.

```text
# Figure 9: The TinyALU computes Fibonacci through the interface

    500.00ns INFO     running FibonacciProgramTest (3/3)  [ch39-virtual-sequence-testbench-8.0/src/ch39_virtual_sequence_testbench_8_0.rs:482]
    670.00ns INFO     [FibonacciProgramSeq]: Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 00 Add 01 = 0001
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 01 Add 01 = 0002
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 01 Add 02 = 0003
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 02 Add 03 = 0005
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 03 Add 05 = 0008
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: PASSED: 05 Add 08 = 000d
    670.00ns INFO     [FibonacciProgramTest.env.scoreboard]: saw 1 of 4 ops (coverage not required)
    670.00ns INFO     FibonacciProgramTest PASSED
```

## The environment underneath

The env this chapter runs on is Chapter 38's, with the driver that answers: it publishes each result on its own analysis port and returns it through `item_done(Some(...))`, and there is no separate `ResultMonitor` — the driver already awaits each answer, so it is the component that *has* it, and a second reader on the BFM's result queue would take turns stealing results from the first. The observation side, the scoreboard's two streams, and the `SEQR` handle in the ConfigDb are all exactly as you left them. Nothing in the environment knows that virtual sequences exist — which is the measure of the design: the top layer of the stimulus stack arrived, and no layer below it moved.

## Summary

A virtual sequence is a sequence that starts sequences: same trait, no items of its own, started with `start_virtual()` because it has no sequencer to be started on. Sequential composition is two `start` calls in a row; parallel composition is `join2` over two `start` futures, with the sequencer interleaving the streams. There is deliberately no `VirtualSequence` trait — a compile-time fence there would forbid the mixed shapes the UVM permits, and the framework takes the UVM's side of that trade with its eyes open. The chapter's real product is the interface pattern: an `OpSeq` with parameters, wrapped in `do_add`-style functions, until a test reads like arithmetic and the testbench underneath is invisible.

That is testbench 8.0, and with it every mechanism the UVM promised: phases, configuration, factory, TLM, analysis, transactions, sequences, and programs built from all of them. Chapter 40 returns to the shipped TinyALU testbench — the one the Interlude showed you before you could read it — and walks it end to end, with nothing left unexplained.

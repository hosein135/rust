# Chapter 36: Sequence Testbench: 7.0

Testbench 6.0 is a fine machine with one design flaw left: a new stimulus pattern means a new *component*. Want maximum operands instead of random ones? Override the Tester through the factory — rebuild part of the structure to change what the structure carries. *The UVM Primer* puts the objection best: overriding the tester to change stimulus is "like swapping out your car's steering wheel whenever you chose a different destination." The UVM's answer is the sequence system, and it is the methodology's crown jewel: the testbench structure holds still, and the **program** changes. This chapter builds testbench 7.0 around it.

> **In the UVM...** a `uvm_sequence` holds a `body()` task that creates `uvm_sequence_item`s and sends them with `start_item()`/`finish_item()`. A `uvm_sequencer` arbitrates among running sequences; the driver pulls with `seq_item_port.get_next_item()`, drives the DUT, and releases with `item_done()`. A test creates a sequence and calls `seq.start(sequencer)`.

The cast, before the code:

- A **sequence** is *not a component*. It has no place in the tree, no path, no phases — one method, `body`, and a context to run it against. It is a test program.
- The **sequencer** *is* a component: it holds the arbitration machinery and hands out one export. The environment files a handle to it in the ConfigDb so that a test levels above can start sequences on it without knowing where it lives.
- The **driver** pulls. Testbench 6.0's `cmd_fifo` is gone; the sequencer is the decoupling point now.

<figure>
<svg viewBox="0 0 660 300" xmlns="http://www.w3.org/2000/svg" font-family="sans-serif" font-size="13">
  <defs>
    <marker id="harr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#888"/>
    </marker>
  </defs>
  <text x="90" y="24" text-anchor="middle" fill="currentColor" font-weight="bold">Sequence</text>
  <text x="330" y="24" text-anchor="middle" fill="currentColor" font-weight="bold">Sequencer</text>
  <text x="570" y="24" text-anchor="middle" fill="currentColor" font-weight="bold">Driver</text>
  <line x1="90" y1="34" x2="90" y2="290" stroke="#888" stroke-dasharray="4 3"/>
  <line x1="330" y1="34" x2="330" y2="290" stroke="#888" stroke-dasharray="4 3"/>
  <line x1="570" y1="34" x2="570" y2="290" stroke="#888" stroke-dasharray="4 3"/>
  <line x1="565" y1="58" x2="335" y2="58" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="450" y="50" text-anchor="middle" fill="currentColor" font-size="11">get_next_item().await — driver blocks</text>
  <line x1="95" y1="92" x2="325" y2="92" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="210" y="84" text-anchor="middle" fill="currentColor" font-size="11">start_item(&amp;mut cmd).await</text>
  <line x1="325" y1="118" x2="95" y2="118" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="210" y="110" text-anchor="middle" fill="currentColor" font-size="11">grant: your turn; driver is waiting</text>
  <rect x="30" y="136" width="120" height="40" rx="6" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="90" y="153" text-anchor="middle" fill="currentColor" font-size="11" font-style="italic">set the stimulus</text>
  <text x="90" y="168" text-anchor="middle" fill="currentColor" font-size="11" font-style="italic">HERE — late setting</text>
  <line x1="95" y1="204" x2="325" y2="204" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="210" y="196" text-anchor="middle" fill="currentColor" font-size="11">finish_item(cmd).await — cmd moves</text>
  <line x1="335" y1="222" x2="565" y2="222" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="450" y="214" text-anchor="middle" fill="currentColor" font-size="11">item — get_next_item returns</text>
  <line x1="565" y1="256" x2="335" y2="256" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="450" y="248" text-anchor="middle" fill="currentColor" font-size="11">item_done() — after driving the DUT</text>
  <line x1="325" y1="278" x2="95" y2="278" stroke="#888" stroke-width="1.5" marker-end="url(#harr)"/>
  <text x="210" y="270" text-anchor="middle" fill="currentColor" font-size="11">finish_item returns</text>
</svg>
<figcaption><em>Figure 1: The sequencer handshake. Everything between the grant and finish_item happens with the driver committed and holding still.</em></figcaption>
</figure>

Study the window in the middle of figure 1, because it is the answer to the question every newcomer asks about this protocol: *why two calls?* Why `start_item` then `finish_item`, when a single `send(cmd).await` looks like it would do? Because `start_item` returns at a very particular moment — the sequencer has granted this item its turn, *and the driver is blocked waiting for its contents*. Everything the sequence does between the two calls happens with the driver committed and holding still. That is where **late stimulus setting** lives: a sequence can look at the state of the testbench and decide what to send *now*, at the moment of delivery, rather than when it queued the item. A single `send` fixes the values before arbitration runs; the two-call rendezvous fixes them after. SystemVerilog had `mailbox#(T)` in the language and built this two-phase rendezvous anyway; pyuvm simplified nearly everything else about sequences and kept both phases. The gap between the calls is the feature.

## The driver

```rust
// Chapter 36, Figure 2: The driver pulls items instead of being pushed them
#[derive(Component, Default)]
struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            let cmd = item.payload();
            bfm.send_op(cmd.a, cmd.b, cmd.op).await;
            self.seq_item_port.item_done(None);
        }
    }
}
```

The difference from 6.0's driver is not the direction of the data — it is *who decides when*. `get_next_item()` returns only when a sequence has an item ready **and** the driver asked for it: a rendezvous, not a queue. The port is a `SeqItemPort<AluCommand, AluResult>` — request and response types, the same `#(REQ, RSP)` convention `uvm_driver` uses — declared with `#[port(seq_item)]` and wired in `connect` like every port since Chapter 31. And note `item_done(None)`: testbench 7.0 fires and forgets, no answer travels back, which is why the test will hold its objection for a flush at the end. Chapter 38's driver answers, and the flush goes away.

The transactions are Chapter 35's, re-shown as always rather than imported:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AluCommand {
    pub a: u8,
    pub b: u8,
    pub op: Ops,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AluResult {
    pub result: u16,
}
```

## The sequences

The Python book writes a `BaseSeq` whose `body()` loops the operations and calls `self.set_operands(tr)`, then subclasses it twice to override that one method. Rust has no inheritance, so the same design splits along a different line: the part that *varies* is a trait, and the part that *does not* is a function.

```rust
// Chapter 36, Figure 3: One body, three stimulus patterns
trait Operands {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand);
}

async fn all_ops<S: Operands>(
    seq: &mut S,
    ctx: &mut SeqCtx<AluCommand, AluResult>,
) -> Result<(), SeqError> {
    let mut rng = ctx.rng();
    for op in Ops::ALL {
        let mut cmd = AluCommand { a: 0, b: 0, op };
        ctx.start_item(&mut cmd).await?; // the driver is now waiting for us
        seq.set_operands(&mut rng, &mut cmd); // decide the stimulus HERE
        ctx.finish_item(cmd).await?; // hand it over; wait for item_done
    }
    Ok(())
}
```

Three lines in `all_ops` are figure 1 as code, and the middle one is deliberately placed: `set_operands` runs *between* `start_item` and `finish_item`, in the late-setting window, which is the whole reason the two calls exist.

The third line is also this book's Chapter 5 collecting a payoff. `finish_item(cmd)` takes the command **by value** — the sequence hands over the *contents*, not a reference to something it still holds. After that line, `cmd` is gone: use it and the compiler's error names the move. If a sequence needs the command afterward — to log it, or to build the next command from it — `finish_item(cmd.clone())` is equally legal, and which one you write is a decision about ownership the code now states instead of implying. Both source books write the result *into* the sequence item the sequence still holds — not a capability rustdv lacks, but what handles look like when two names point at one object. Rust has one owner, so nothing is shared and nothing is missing.

```rust
// Chapter 36, Figure 4: The base sequence sends zeros
#[derive(Default)]
struct BaseSeq;

impl Operands for BaseSeq {
    fn set_operands(&mut self, _rng: &mut Rng, _cmd: &mut AluCommand) {
        // zeros: whatever `all_ops` built the command with
    }
}

impl Sequence for BaseSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx).await
    }
}
```

`Sequence` is the trait with one method — `body` — plus the request and response types. Implementing it is what makes `BaseSeq` startable and, as figure 8 will show, factory-overridable.

```rust
// Chapter 36, Figure 5: Random and maximum operands
#[derive(Default)]
struct RandomSeq;

impl Operands for RandomSeq {
    fn set_operands(&mut self, rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = rng.u8();
        cmd.b = rng.u8();
    }
}

impl Sequence for RandomSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx).await
    }
}

#[derive(Default)]
struct MaxSeq;

impl Operands for MaxSeq {
    fn set_operands(&mut self, _rng: &mut Rng, cmd: &mut AluCommand) {
        cmd.a = 0xFF;
        cmd.b = 0xFF;
    }
}

impl Sequence for MaxSeq {
    type Req = AluCommand;
    type Rsp = AluResult;

    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        all_ops(self, ctx).await
    }
}
```

One detail with regression consequences: the random numbers come from `ctx.rng()`, the seeded generator every other part of the testbench uses, so a failing run reproduces from its seed. (pyuvm's sequences draw from Python's global `random` module, which sits outside cocotb's seeding.)

## The environment

```rust
// Chapter 36, Figure 6: The env owns the sequencer and files its handle
#[derive(Component, Default)]
struct AluEnv {
    #[component]
    seqr: Sequencer<AluCommand, AluResult>,
    #[component]
    driver: RustdvComp,
    #[component]
    cmd_mon: RustdvComp,
    #[component]
    result_mon: RustdvComp,
    #[component]
    scoreboard: RustdvComp,
    #[component]
    cmd_bus: AnalysisBus<CmdTuple>,
    #[component]
    result_bus: AnalysisBus<u64>,
}

impl Component for AluEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        self.driver = Driver::new_comp();
        self.cmd_mon = CmdMonitor::new_comp();
        self.result_mon = ResultMonitor::new_comp();
        self.scoreboard = Scoreboard::new_comp();
        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // stimulus: sequences --> [seqr] --> Driver
        self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);

        // observation, unchanged from Chapter 34
        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}
```

Three things:

- `#[component]` is the same carve-out `TlmFifo` got in Chapter 31: both endpoints of a connection are erased `RustdvComp` slots, so something concrete has to make the call, and a sequencer — like a FIFO — is infrastructure you will never factory-override. The connect line has the shape every connection since Chapter 31 has had.
- `ConfigDb::set(None, "*", "SEQR", self.seqr.handle())` files the sequencer where any test can find it. This is pyuvm's idiom, and the reason it beats searching the tree by path string is Chapter 27's: a hand-typed path goes stale, and the ConfigDb is how things that must find each other do. Note the sequencer goes in as a *handle* — every sequence must reach the same sequencer, the `Rc` case from Chapter 27's Clone-or-`Rc` rule.
- The monitors, the two `AnalysisBus` buses, and the two-stream scoreboard are Chapters 33–34's, unchanged. That is the chapter's claim about structure made visible: sequences arrived, and the observation side did not move.

## The tests

```rust
// Chapter 36, Figure 7: The test starts a sequence on the sequencer
#[rustdv::test]
#[derive(Component, Default)]
struct BaseTest {
    #[component]
    env: RustdvComp,
}

impl Component for BaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("running the sequence");
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(Some(ctx), "", "SEQR")?;

        let mut seq = create_seq::<BaseSeq>();
        seq.start(&seqr).await?;

        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        for _ in 0..20 {
            bfm.clk().falling_edge().await;
        }
        Ok(())
    }
}
```

The test finds the sequencer in the ConfigDb — it neither knows nor cares where in the tree it lives — and `start` is a method on the *sequence*, taking the sequencer, exactly as both source books write it. Three details:

- The lookup happens in `run`, not in an elaboration phase. pyuvm does it in `end_of_elaboration_phase` because a Python phase cannot return an error; here `?` works, and a missing `SEQR` is a named failure.
- `create_seq::<BaseSeq>()` builds the sequence *through the factory* — a second registry, parallel to Chapter 29's, because a sequence is not a component and cannot ride the first one. That line is what makes the next figure possible.
- After `start` returns, the last commands are still in flight — accepted, not yet answered. The test holds its objection for twenty falling edges: twenty, not ten, because the multiply is the last operation and the slowest, and a shorter flush would let the scoreboard silently check fewer results than it saw commands.

```rust
// Chapter 36, Figure 8: Two more tests, one testbench, no new components
#[rustdv::test]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    inner: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, RandomSeq>();
        self.inner = BaseTest::new_comp();
    }
}

#[rustdv::test]
#[derive(Component, Default)]
struct MaxTest {
    #[component]
    inner: RustdvComp,
}

impl Component for MaxTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        set_seq_override::<BaseSeq, MaxSeq>();
        self.inner = BaseTest::new_comp();
    }
}
```

This is what sequences bought. In Chapter 30, a new stimulus pattern meant a new component and a factory override on a component slot. Here it is a different *program* run through an unchanged structure: `RandomTest` is `BaseTest` plus one `set_seq_override` line, and nothing in `AluEnv` knows either sequence exists. The steering wheel stays; only the destination changes.

```text
# Figure 9: Testbench 7.0 running

      0.00ns INFO     rustdv: found 3 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running BaseTest (1/3)  [ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs:385]
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 Add 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 And 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 Xor 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: PASSED: 00 Mul 00 = 0000
    280.00ns INFO     [BaseTest.env.scoreboard]: Covered all operations
    280.00ns INFO     BaseTest PASSED
    280.00ns INFO     running RandomTest (2/3)  [ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs:426]
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: c1 Add 67 = 0128
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: 5e And 0b = 000a
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: b9 Xor 80 = 0039
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: PASSED: a5 Mul 75 = 4b69
    560.00ns INFO     [RandomTest.inner.env.scoreboard]: Covered all operations
    560.00ns INFO     RandomTest PASSED
    560.00ns INFO     running MaxTest (3/3)  [ch36-sequence-testbench-7.0/src/ch36_sequence_testbench_7_0.rs:440]
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff Add ff = 01fe
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff And ff = 00ff
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff Xor ff = 0000
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: PASSED: ff Mul ff = fe01
    840.00ns INFO     [MaxTest.inner.env.scoreboard]: Covered all operations
    840.00ns INFO     MaxTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** BaseTest                                     PASS         280.00      **
** RandomTest                                   PASS         280.00      **
** MaxTest                                      PASS         280.00      **
******************************************************************************
REGRESSION: PASS
```

## Summary

Testbench 7.0 separates what to send from what sends it. A sequence is a program — no tree, no path, no phases, one `body` — started on a sequencer, which is the component that arbitrates turns and feeds the driver through the same connect idiom as every other wiring in the book. The two-call handshake is the design's heart: `start_item` returns with the driver committed and waiting, the window between the calls is where stimulus is decided at the last responsible moment, and `finish_item(cmd)` hands over the command by value — clone it first if you still need it, and the compiler will hold you to whichever answer you gave. Sequences build through their own factory registry, so `create_seq::<BaseSeq>()` plus `set_seq_override` gives tests the same substitution power over programs that Chapter 29 gave them over structure.

Version 7.0 fires and forgets, which is why its test counts clocks at the end instead of knowing when the work is done. The next two chapters close the loop: first a driver that answers out of order, where a ticket claims the answer you asked for, then Fibonacci on the TinyALU, where each command cannot be written until the previous one is answered.

//! ASPIRATIONAL — the target API for sequences (D1/D2), written before the
//! framework can compile it. Not a crate: this file deliberately sits outside
//! `output/examples/` so nothing builds it. It will split into four chapter
//! crates when the framework satisfies it:
//!
//!   ch36 / TB 7.0 — sequencer, driver, sequences, factory-overridden stimulus
//!   ch37 / TB 7.1 — Fibonacci: stimulus that needs the DUT's answers
//!   ch38 / TB 7.2 — the response queue, for answers that do not come home
//!   ch39 / TB 8.0 — virtual sequences, parallelism, a programming interface
//!
//! Companion analysis: `output/sequences-research.md`. Open questions this
//! code takes a position on are marked `// Q20` etc. and are Ray's to settle —
//! where a question is open, the conservative (pyuvm-matching) form is used.
//!
//! ===========================================================================
//! What this asks the framework for
//! ===========================================================================
//!
//! 1. `Sequencer<REQ, RSP>` is a **component**: a concrete child declared
//!    `#[component]`, carved out of D78 the same way a `TlmFifo` is (D84) and
//!    for the same reason — something concrete must hand out the export, and a
//!    sequencer is plumbing, never a factory-override target. The carve-out is
//!    a property of the type, not of the attribute: D114 (§42) removed the
//!    attribute's argument, since the derive never read it.
//! 2. `#[port(seq_item)]` is a new port kind. `SeqItemPort<REQ, RSP>`
//!    implements `PortField`, so `Driver::SEQ_ITEM_PORT` is generated and
//!    `connect` resolves it through `ComponentNode::port_slot` like every
//!    other port (D83b). Required at elaboration, like put/get (D85).
//! 3. `#[derive(Sequence)]` registers a sequence in a second link section so
//!    `Factory::set_type_override` and `create_seq_by_name` work on objects
//!    that are not components (D80).
//! 4. `SeqCtx` carries logging and a seeded `Rng`, both derived from the
//!    sequencer's path. Neither book's sequence can log or reproduce a seed;
//!    this is cheap and rustdv needs it because a factory-built sequence is
//!    made by a `Default` maker and cannot be handed a seed at construction.
//! 5. **Nothing about REQ/RSP changes.** `finish_item(req)` takes ownership,
//!    `item_done(rsp)` sends the answer back as its own value — which is the
//!    standard, and is what `sequence.rs` already does (D93). The books' habit
//!    of writing the result *into* the item is what handles look like in a
//!    language that has them, not a capability to reproduce.
//! 6. Sub-sequences are **joined, not spawned** (D82), so a sub-sequence may
//!    borrow the parent sequence's state.

use rustdv::prelude::*;
use tinyalu_utils::{CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// `CmdMonitor`, `ResultMonitor`, `Scoreboard` and `Coverage` are Chapter 34's,
// unchanged and elided here — nothing about the observation side of the
// testbench changes when the stimulus side gains sequences. That is the point
// being made: the structure holds still while the programs change.

// ===========================================================================
// Chapter 35 / the transaction — what a sequence item is in Rust
// ===========================================================================

// Chapter 36, Figure 1: A sequence item is a plain struct with derives.
//
// The UVM's `uvm_sequence_item` carries three things: the data, the identity
// (sequence id + transaction id), and the handshake events. rustdv keeps only
// the **data** here. Identity and events live in the framework's `SeqItem<T>`
// envelope, which the driver receives — so a transaction stays a struct you
// can derive `Debug`, `Clone` and `PartialEq` on and read at a glance.
//
// That split is the seam this whole framework is built on: types for data,
// runtime machinery for plumbing. `convert2string`/`do_compare`/`do_copy` are
// three derives, and `set_id_info` is something you never call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AluCommand {
    a: u8,
    b: u8,
    op: Ops,
}

impl AluCommand {
    fn new(a: u8, b: u8, op: Ops) -> AluCommand {
        AluCommand { a, b, op }
    }
}

impl std::fmt::Display for AluCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A: {:02x} {:?} B: {:02x}", self.a, self.op, self.b)
    }
}

// ===========================================================================
// Chapter 36 / TB 7.0 — the sequencer, the driver, and the handshake
// ===========================================================================

// Chapter 36, Figure 2: The Driver pulls items instead of being pushed them.
//
// In TB 6.0 the Tester *put* commands into a `TlmFifo` and the Driver *got*
// them. The difference here is not the direction of the data — it is who
// decides when. `get_next_item()` returns only when a sequence has an item
// ready *and* the driver asked for it: a rendezvous, not a queue. The driver
// then owns the item until it calls `item_done`.
//
// Note there is no `cmd_fifo` any more. The sequencer is the decoupling point.
#[derive(Component, Default)]
struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
}

impl Component for Driver {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm = TinyAluBfm::get();
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            let cmd = item.payload();
            bfm.send_op(cmd.a, cmd.b, cmd.op).await;
            // 7.0's driver fires and forgets: it does not wait for the answer,
            // which is why the test below must hold its objection for a flush.
            // Figure 11's driver waits, and then no flush is needed.
            self.seq_item_port.item_done(None);
        }
    }
}

// Chapter 36, Figure 3: The env owns the sequencer and connects it.
//
// A concrete sequencer field is the same deliberate exception D84 made for
// FIFOs: both endpoints of a connection are erased `RustdvComp` slots, so
// something concrete has to make the call, and the sequencer is that thing.
// The declaration says only `#[component]` — the exception lives in the field's
// type, which is where it was always decided (D114, §42).
// The connect line has the shape every connection in Chapters 31–34 had —
// a concrete child, a named export, `connect(owner, PORT_NAME)`.
//
// The env also files the sequencer handle in the ConfigDb under "SEQR", which
// is how a test three levels up starts a sequence on it without knowing where
// it lives. That is pyuvm's idiom, and it is better than the Primer's
// `uvm_top.find("*.env_h.sequencer_h")` for the reason D83a gives: a
// hand-typed path keeps compiling and keeps addressing the wrong thing after
// a rename.
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
    coverage: RustdvComp,
    #[component]
    cmd_bus: AnalysisBus<CmdTuple>,
    #[component]
    result_bus: AnalysisBus<u64>,
}

impl Component for AluEnv {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.seqr = Sequencer::new();
        ConfigDb::set(Some(ctx), "*", "SEQR", self.seqr.handle());

        self.driver = Driver::new_comp();
        self.cmd_mon = CmdMonitor::new_comp();
        self.result_mon = ResultMonitor::new_comp();
        self.scoreboard = Scoreboard::new_comp();
        self.coverage = Coverage::new_comp();
        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // stimulus: sequences --> [seqr] --> Driver
        self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);

        // observation, unchanged from Chapter 34
        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        self.cmd_bus.sub_export().connect(&self.coverage, Coverage::CMD_IN);
        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, _ctx: &mut RustdvCtx) {
        TinyAluBfm::get().start_tasks();
    }
}

// Chapter 36, Figure 4: A sequence is not a component.
//
// It has no place in the tree, no path of its own, and no phases. It has one
// method. `#[derive(Sequence)]` registers it with the factory so a test can
// substitute one sequence for another by type (Figure 9) — the same
// mechanism as Chapter 29's component factory, in a second registry, because
// a sequence is not a `ComponentNode` and cannot ride the first one.
//
// `body` takes a `SeqCtx`, which is the sequence's equivalent of a
// component's `RustdvCtx`: it can log, it has a seeded `Rng`, and it knows
// the sequencer. That last part is what `start_item` needs.
#[derive(Sequence, Default)]
struct BaseSeq;

impl Sequence<AluCommand, AluResult> for BaseSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        for op in Ops::ALL {
            let mut cmd = AluCommand::new(0, 0, op);
            ctx.start_item(&mut cmd).await?;
            self.set_operands(ctx, &mut cmd);
            ctx.finish_item(cmd).await?;
        }
        Ok(())
    }
}

impl BaseSeq {
    // Overridden by the sequences below. Runs *between* start_item and
    // finish_item — see Figure 5.
    fn set_operands(&mut self, _ctx: &mut SeqCtx<AluCommand, AluResult>, _cmd: &mut AluCommand) {}
}

// Chapter 36, Figure 5: Why there are two calls and not one.
//
//   ctx.start_item(&mut cmd).await?;   // <-- the driver is now waiting for us
//   cmd.a = ...;  cmd.b = ...;         // <-- decide the stimulus HERE
//   ctx.finish_item(cmd).await?;       // <-- hand it over; wait for item_done
//
// `start_item` returns when the sequencer has granted this item its turn and
// the driver is blocked waiting for its contents. Everything between the two
// calls happens with the driver committed and holding still. That is the
// window `late stimulus setting` needs: a sequence can look at the state of
// the testbench — a scoreboard's last result, a counter, the clock — and
// decide what to send *now* rather than when it queued the item.
//
// A single `send(cmd).await` could not express this, because the values would
// have been fixed before the arbitration ran. SystemVerilog had a
// `mailbox#(T)` and built this two-phase rendezvous anyway; pyuvm simplified
// almost everything else about sequences and kept both phases. The gap is the
// feature.

// Chapter 36, Figure 6: Two sequences, one testbench.
#[derive(Sequence, Default)]
struct RandomSeq;

impl Sequence<AluCommand, AluResult> for RandomSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        // The RNG comes from the context, so the run is reproducible from the
        // seed the way every other part of the testbench is. pyuvm's sequences
        // reach for the global `random` module and are not.
        let mut rng = ctx.rng();
        for op in Ops::ALL {
            let mut cmd = AluCommand::new(0, 0, op);
            ctx.start_item(&mut cmd).await?;
            cmd.a = rng.u8();
            cmd.b = rng.u8();
            ctx.finish_item(cmd).await?;
        }
        Ok(())
    }
}

#[derive(Sequence, Default)]
struct MaxSeq;

impl Sequence<AluCommand, AluResult> for MaxSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        for op in Ops::ALL {
            let mut cmd = AluCommand::new(0xFF, 0xFF, op);
            ctx.start_item(&mut cmd).await?;
            ctx.finish_item(cmd).await?;
        }
        Ok(())
    }
}

// Chapter 36, Figure 7: The test starts a sequence on the sequencer.
//
// The test finds the sequencer in the ConfigDb — it does not know or care
// where in the tree it lives. `start` is a method on the *sequence*, taking
// the sequencer, exactly as both source books write it.
//
// Note where the lookup happens. pyuvm does it in `end_of_elaboration_phase`
// because a Python phase cannot return an error. rustdv does it in `run`,
// where `?` works and a missing SEQR is a named failure rather than an
// exception thrown from a phase (D14).
#[rustdv::test]
#[derive(Component, Default)]
struct BaseTest {
    #[component]
    env: RustdvComp,
}

impl Component for BaseTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("running the sequence");
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(Some(ctx), "", "SEQR")?;

        // Created through the factory, so a test can override which sequence
        // this line actually builds (Figure 9).
        let mut seq = BaseSeq::create_seq();
        seq.start(&seqr).await?;

        // 7.0's driver does not wait for results, so the last few commands are
        // still in the DUT when the sequence returns. Hold the objection for a
        // flush. Chapter 37's driver waits for each result, and this goes away.
        let bfm = TinyAluBfm::get();
        for _ in 0..20 {
            bfm.clk().falling_edge().await;
        }
        Ok(())
    }
}

// Chapter 36, Figure 8: Two tests, one testbench, no new components.
//
// This is what sequences bought. In Chapter 30 a new stimulus pattern meant a
// new *component* and a factory override on a component slot. Here it is a
// different **program** run through an unchanged structure, and the override
// is on a sequence type. Nothing in `AluEnv` knows either sequence exists.
#[rustdv::test]
#[derive(Component, Default)]
struct RandomTest {
    #[component]
    inner: RustdvComp,
}

impl Component for RandomTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        // Chapter 36, Figure 9: the sequence factory, not the component
        // factory. `BaseSeq` is not a component and does not live in the tree,
        // so it rides a second registry — same override table, same
        // `set_type_override` spelling, different link section (D80).
        Factory::set_seq_override::<BaseSeq, RandomSeq>();
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
        Factory::set_seq_override::<BaseSeq, MaxSeq>();
        self.inner = BaseTest::new_comp();
    }
}

// ===========================================================================
// Chapter 37 / TB 7.1 — stimulus that needs the DUT's answers
// ===========================================================================

// Chapter 37, Figure 1: A driver that waits for the answer, and sends it back.
//
// `get_next_item` hands over a `SeqItem<AluCommand>` — the framework's
// envelope, carrying the transaction id and the payload. The command is the
// driver's now; it is not going back anywhere. The answer travels separately,
// as an `AluResult`, and `item_done` tags it with this item's id so the
// sequence that asked gets the one it asked for.
//
// **Do not go looking for the UVM's shared handle here.** In SystemVerilog and
// Python the sequence still holds a reference to the object it sent, so a
// driver writing `cmd.result` is visible to the sequence at once, and both
// books use that. It is not a feature to reproduce — it is what happens when
// two names point at one object. Rust has one owner, so the request goes one
// way and the answer comes back the other, which is what the UVM's own
// `item_done(rsp)` was always for.
//
// A sequence that wants to keep the command it sent clones it before
// `finish_item`. Most do not need to.
#[derive(Component, Default)]
struct RspDriver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
    #[port(publish)]
    result_ap: PublishPort<u64>,
}

impl Component for RspDriver {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm = TinyAluBfm::get();
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            let cmd = item.payload();
            bfm.send_op(cmd.a, cmd.b, cmd.op).await;
            // Wait for *this* operation's answer before taking another item.
            let result = bfm.get_result().await;
            self.result_ap.write(&result);
            // The answer goes back as its own value. The command itself is the
            // driver's now — it does not travel anywhere.
            self.seq_item_port.item_done(Some(AluResult { result: result as u16 }));
        }
    }
}

// Chapter 37, Figure 2: The Fibonacci sequence.
//
// Each command's operands are the answers to the two before it, so this
// stimulus cannot be generated in advance — it has to be written one command
// at a time, with the DUT's answer in hand. That is the whole reason the
// sequence system exists, reduced to nine lines.
//
// Read the rebinding on the `finish_item` line. `finish_item` takes the
// command by value (the driver has to own it while it drives) and hands it
// back when the driver is done — the same shape as Chapter 31's `try_put`
// returning `Err(back)`, one level up. Written without the rebinding it does
// not compile, because `cmd` was moved.
#[derive(Sequence, Default)]
struct FibonacciSeq;

impl Sequence<AluCommand, AluResult> for FibonacciSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let mut prev: u8 = 0;
        let mut cur: u8 = 1;
        let mut fib = vec![prev as u16, cur as u16];

        for _ in 0..7 {
            let mut cmd = AluCommand::new(0, 0, Ops::Add);
            ctx.start_item(&mut cmd).await?;
            cmd.a = prev;
            cmd.b = cur;
            // `cmd` moves to the driver here. This sequence has no further use
            // for it; one that did would write `finish_item(cmd.clone())`, and
            // the compiler would say so if it forgot.
            let id = ctx.finish_item(cmd).await?;
            let sum = ctx.get_response(Some(id)).await.result;
            fib.push(sum);
            prev = cur;
            cur = sum as u8;
        }

        // Chapter 37, Figure 3: a sequence can log.
        //
        // pyuvm's `uvm_sequence` is not a `uvm_report_object` and has to reach
        // for `uvm_root().logger`; the Primer's sequences use `uvm_info` with
        // a hand-typed string id. rustdv's `SeqCtx` logs under the sequencer's
        // path, so a sequence's output sits where the reader expects it.
        ctx.info(&format!("Fibonacci Sequence: {fib:?}"));
        Ok(())
    }
}

// Expected: Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]

// ===========================================================================
// The response transaction
// ===========================================================================

// Chapter 37, Figure 4: A response is its own transaction.
//
// It is not the command with a field filled in — it is a separate value the
// driver creates and gives up. `item_done` tags it with the request's id, so a
// sequence gets the answer to the question it asked even when several are in
// flight.
//
// Note what rustdv does not make you do. SystemVerilog needs
// `rsp.set_id_info(req)` and pyuvm needs `rsp.set_context(req)` to correlate
// the two by hand, and forgetting it is a run-time fatal. Here the id is in the
// envelope the driver was handed, so the framework tags the response and there
// is nothing to forget.
#[derive(Debug, Clone, Default)]
struct AluResult {
    result: u16,
}

// ---------------------------------------------------------------------------
// OPEN: what is Chapter 38 (TB 7.2) about now?
//
// In both source books, 7.2 exists to teach the *second* way a driver answers
// a sequence — `get_response()` as an alternative to writing into the shared
// item handle. rustdv has one way, because the shared handle was never a
// mechanism (D93). So the chapter's original subject has dissolved, and this
// file deliberately does not invent a replacement.
//
// The material that is genuinely still there and unclaimed: several requests
// outstanding at once, answers correlated by id rather than by arrival order.
// The TinyALU cannot show it — `cmd_driver` drives only when the bus is idle,
// one operation at a time — so a chapter on it would need either a different
// DUT or a driver that decouples from result collection without the DUT
// pipelining. That is a question for Ray, not a gap to fill here.
// ---------------------------------------------------------------------------

// ===========================================================================
// Chapter 39 / TB 8.0 — virtual sequences and a programming interface
// ===========================================================================

// Chapter 39, Figure 1: A virtual sequence starts other sequences.
//
// "Virtual" means started **without a sequencer**. It sends no items of its
// own, so it needs no item context — it is a program that runs other programs.
// It finds a sequencer the same way the test did, in the ConfigDb.
//
// Calling `start_item` in here is an error, and it is a **run-time** error, as
// it is in pyuvm. It would be easy to make it a compile error by giving
// virtual sequences their own trait with no `start_item` to call — and that is
// the make-it-static reflex D3 exists to catch. The UVM's own distinction is
// not crisp: the Primer's `parallel_sequence` is started *with* a sequencer
// and is still virtual in the sense that matters, and nothing stops a sequence
// from sending some items and delegating the rest. Two traits would forbid
// that shape to buy a better error message. We do not take the trade.
//
// So a virtual sequence implements the same `Sequence` trait and simply never
// touches its item type — the same thing SystemVerilog does when it writes
// `runall_sequence extends uvm_sequence #(uvm_sequence_item)` and never sends
// a `uvm_sequence_item`.
#[derive(Sequence, Default)]
struct TestAllSeq;

impl Sequence<AluCommand, AluResult> for TestAllSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(None, "", "SEQR")?;
        RandomSeq::default().start(&seqr).await?;
        MaxSeq::default().start(&seqr).await?;
        Ok(())
    }
}

// Chapter 39, Figure 2: Running sub-sequences in parallel.
//
// `join2` is SystemVerilog's `fork...join`, under a name the reader met at
// Chapter 31 and will meet again wherever two things must run at once. The
// sequencer interleaves the two streams — with FIFO arbitration, one item each
// in turn — so the transcript alternates random operands with 0xff operands.
//
// The reason this is `join2` and not `spawn`: a spawned future must be
// `'static`, and a sub-sequence that borrows the parent sequence's state
// cannot be. Composing futures in place costs nothing and keeps that door open
// (D82).
#[derive(Sequence, Default)]
struct TestAllParallelSeq;

impl Sequence<AluCommand, AluResult> for TestAllParallelSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let seqr: Sequencer<AluCommand, AluResult> = ConfigDb::get(None, "", "SEQR")?;
        let mut random = RandomSeq::default();
        let mut max = MaxSeq::default();
        let (a, b) = join2(random.start(&seqr), max.start(&seqr)).await;
        a?;
        b?;
        Ok(())
    }
}

// Chapter 39, Figure 3: One operation, as a sequence.
//
// A sequence with parameters, built programmatically rather than by the
// factory — the factory's makers take no arguments, so a sequence that needs
// operands is constructed the ordinary way. Both source books do the same.
struct OpSeq {
    a: u8,
    b: u8,
    op: Ops,
    result: Option<u16>,
}

impl Sequence<AluCommand, AluResult> for OpSeq {
    async fn body(&mut self, ctx: &mut SeqCtx<AluCommand, AluResult>) -> Result<(), SeqError> {
        let mut cmd = AluCommand::new(self.a, self.b, self.op);
        ctx.start_item(&mut cmd).await?;
        let id = ctx.finish_item(cmd).await?;
        self.result = Some(ctx.get_response(Some(id)).await.result);
        Ok(())
    }
}

// Chapter 39, Figure 4: A programming interface for the TinyALU.
//
// This is the payoff. A test writer who has never opened the testbench gets
// four functions that take numbers and return numbers. Everything above —
// sequencer, driver, handshake, envelope — is behind them.
//
// In Python these return the sequence's `result` field after `start`. In Rust
// they return the value directly, because the sequence can hand it back: a
// function that computes something returns what it computed.
async fn do_op(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8, op: Ops) -> Result<u16, SeqError> {
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

// Chapter 39, Figure 5: Fibonacci, written as a program.
//
// The same test as Chapter 37, with no sequence machinery visible at all.
// Compare the two: this is what a programming interface is for.
#[derive(Sequence, Default)]
struct FibonacciProgramSeq;

impl Sequence<AluCommand, AluResult> for FibonacciProgramSeq {
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

// Chapter 39, Figure 6: The test starts a virtual sequence — no sequencer.
#[rustdv::test]
#[derive(Component, Default)]
struct AluTest {
    #[component]
    env: RustdvComp,
}

impl Component for AluTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("running the virtual sequence");
        let mut test_all = TestAllSeq::create_seq();
        test_all.start_virtual().await?; // D95: no sequencer, so no `start(&seqr)`
        Ok(())
    }
}

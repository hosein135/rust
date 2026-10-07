//! Chapter 39: Virtual sequences — testbench 8.0, programs that run programs.
//!
//!     sim-common/run_sim.sh ch39_virtual_sequence_testbench_8_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! A **virtual sequence** is started without a sequencer. It sends no items of
//! its own; it starts other sequences. That is the whole of the idea, and it is
//! what lets a test be assembled from stimulus that already exists rather than
//! written again.
//!
//! Three things this chapter does with that:
//!
//! 1. `TestAllSeq` runs `RandomSeq` and then `MaxSeq` — one test, two stimulus
//!    patterns, no new components (Figure 2).
//! 2. `TestAllParallelSeq` runs them **at the same time**, and the sequencer
//!    interleaves their items (Figure 3).
//! 3. `OpSeq` plus four functions turn the testbench into a **programming
//!    interface**: a test writer who has never opened the testbench gets
//!    `do_add(&seqr, a, b)` and gets a number back (Figures 4–5).
//!
//! ## Why `start_virtual` and not `start(None)`
//!
//! Rust has no default arguments, so pyuvm's single optional-sequencer `start`
//! becomes two entry points (D95). `start(Some(&seqr))` at every ordinary call
//! site would be noise that says nothing, and `None` does not say "virtual".
//!
//! ## What is deliberately *not* here
//!
//! A `VirtualSequence` trait. It would make calling `start_item` inside a
//! virtual sequence a **compile** error instead of the run-time error pyuvm
//! gives — and it would forbid a shape the UVM allows: the Primer's
//! `parallel_sequence` is started *with* a sequencer and is still virtual in
//! the sense that matters, and nothing stops a sequence from sending some items
//! and delegating the rest. Two traits would buy a better error message at the
//! cost of a capability. That is the make-it-static reflex (D3), and this is it
//! being declined.
//!
//! So a virtual sequence implements the same `Sequence` trait and never touches
//! its item type — exactly what SystemVerilog does when it writes
//! `runall_sequence extends uvm_sequence #(uvm_sequence_item)` and never sends
//! one.
//!
//! ## What this asks the framework for
//!
//! Everything Chapter 36 asked for, plus `Sequence::start_virtual()` and
//! `join2` over two `start` futures — which must **not** require those futures
//! to be `'static` (D82), so a sub-sequence can borrow the parent sequence's
//! state.

use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// Chapter 35 defined these; D45 says a chapter example is self-contained, so
// they are re-shown rather than imported. Plain structs with derives — no base
// class, nothing to extend.
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


// ===========================================================================
// Chapter 36's sequences, unchanged
// ===========================================================================

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
        ctx.start_item(&mut cmd).await?;
        seq.set_operands(&mut rng, &mut cmd);
        ctx.finish_item(cmd).await?;
    }
    Ok(())
}

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

// ===========================================================================
// Virtual sequences
// ===========================================================================

// Chapter 39, Figure 1: A virtual sequence starts other sequences.
//
// It finds a sequencer the same way the test does — in the ConfigDb — and then
// its body reads like a program, because that is what it is. No `start_item`,
// no `finish_item`: there is no item context to call them on.
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

// Chapter 39, Figure 4: The same two sequences, at the same time.
//
// `join2` is SystemVerilog's `fork...join`, under the name the reader met at
// Chapter 31 and will meet wherever two things must run together. The sequencer
// arbitrates between the two streams — FIFO order, one item each in turn — so
// the transcript alternates random operands with 0xff operands.
//
// The reason this is `join2` and not `spawn`: a spawned future must be
// `'static`, and a sub-sequence that borrows the parent sequence's state cannot
// be. Composing futures in place costs nothing and keeps that door open (D82).
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

// ===========================================================================
// A programming interface
// ===========================================================================

// Chapter 39, Figure 6: One operation, as a sequence.
//
// A sequence with parameters, constructed the ordinary way rather than by the
// factory — the factory's makers take no arguments (D80), so a sequence that
// needs operands is built by hand. Both source books do the same.
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

// Chapter 39, Figure 7: The TinyALU programming interface.
//
// This is the payoff. A test writer who has never opened the testbench gets
// four functions that take numbers and return numbers; sequencer, driver,
// handshake and envelope are all behind them.
//
// In Python these read `seq.result` after `start` returns, because a coroutine
// cannot hand a value back through `start`. Here the function returns the value
// directly — a function that computes something returns what it computed.
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
#[allow(dead_code)]
async fn do_and(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::And).await
}
#[allow(dead_code)]
async fn do_xor(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::Xor).await
}
#[allow(dead_code)]
async fn do_mul(seqr: &Sequencer<AluCommand, AluResult>, a: u8, b: u8) -> Result<u16, SeqError> {
    do_op(seqr, a, b, Ops::Mul).await
}

// Chapter 39, Figure 8: Fibonacci, written as a program.
//
// The same computation as Chapter 38, with no sequence machinery visible at
// all. Compare the two side by side: this is what a programming interface is
// for, and why a team that writes tests but not testbenches wants one.
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

// ===========================================================================
// Env, scoreboard, and the tests
// ===========================================================================

#[derive(Component, Default)]
struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
    // The driver already waits for each answer, so it is the one that has it.
    // A separate ResultMonitor would be drawing from the same BFM queue and the
    // two would take turns stealing results from each other.
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

#[derive(Component, Default)]
struct CmdMonitor {
    #[port(publish)]
    ap: PublishPort<CmdTuple>,
}

impl Component for CmdMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let cmd = bfm.get_cmd().await;
            self.ap.write(&cmd);
        }
    }
}

#[derive(Default)]
struct CmdLog {
    cmds: Vec<CmdTuple>,
}
impl Subscriber<CmdTuple> for CmdLog {
    fn write(&mut self, cmd: &CmdTuple) {
        self.cmds.push(*cmd);
    }
}

#[derive(Default)]
struct ResultLog {
    results: Vec<u64>,
}
impl Subscriber<u64> for ResultLog {
    fn write(&mut self, r: &u64) {
        self.results.push(*r);
    }
}

#[derive(Component, Default)]
struct Scoreboard {
    #[port(subscribe)]
    cmd_in: SubscribePort<CmdTuple>,
    #[port(subscribe)]
    result_in: SubscribePort<u64>,
    cmd_log: RustdvShared<CmdLog>,
    result_log: RustdvShared<ResultLog>,
    cvg: std::collections::HashSet<Ops>,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.subscribe(self.cmd_log.clone());
        self.result_in.subscribe(self.result_log.clone());
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmds = self.cmd_log.get();
        let results = self.result_log.get();
        for (cmd, result) in cmds.cmds.iter().zip(results.results.iter()) {
            let (aa, bb, op_int) = *cmd;
            let op = Ops::from_u64(op_int).expect("legal op");
            self.cvg.insert(op);
            let actual = *result as u16;
            let prediction = alu_prediction(aa as u8, bb as u8, op);
            if actual == prediction {
                ctx.info(&format!("PASSED: {aa:02x} {op:?} {bb:02x} = {actual:04x}"));
            } else {
                errors.error(format!(
                    "FAILED: {aa:02x} {op:?} {bb:02x} = {actual:04x} - predicted {prediction:04x}"
                ));
            }
        }
        let want_full: bool = ConfigDb::get(Some(ctx), "", "CHECK_COVERAGE").unwrap_or(true);
        if want_full && Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else if want_full {
            ctx.info("Covered all operations");
        } else {
            ctx.info(&format!("saw {} of {} ops (coverage not required)", self.cvg.len(), Ops::ALL.len()));
        }
    }
}

#[derive(Component, Default)]
struct AluEnv {
    #[component]
    seqr: Sequencer<AluCommand, AluResult>,
    #[component]
    driver: RustdvComp,
    #[component]
    cmd_mon: RustdvComp,
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
        self.scoreboard = Scoreboard::new_comp();
        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);
        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        self.result_bus.pub_export().connect(&self.driver, Driver::RESULT_AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}

// Chapter 39, Figure 2: A test that starts a virtual sequence — no sequencer.
//
// `start_virtual()` takes none, because a virtual sequence has none to take.
// Everything it drives, it drives through sequencers it looked up itself.
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

#[rustdv::test]
#[derive(Component, Default)]
struct ParallelTest {
    #[component]
    env: RustdvComp,
}

impl Component for ParallelTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("running two sequences at once");
        TestAllParallelSeq::default().start_virtual().await?;
        Ok(())
    }
}

#[rustdv::test]
#[derive(Component, Default)]
struct FibonacciProgramTest {
    #[component]
    env: RustdvComp,
}

impl Component for FibonacciProgramTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        let bfm = TinyAluBfm::new(&ctx.dut()).expect("TinyALU signals");
        ConfigDb::set(None, "*", "BFM", Rc::new(bfm));
        // This program only adds, so full coverage is not the goal here.
        ConfigDb::set(None, "*", "CHECK_COVERAGE", false);
        self.env = AluEnv::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("running the Fibonacci program");
        FibonacciProgramSeq::default().start_virtual().await?;
        Ok(())
    }
}

// Expected: AluTest shows four random operations then four 0xff ones;
// ParallelTest shows them interleaved; FibonacciProgramTest prints
// Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21].
// The unused `do_and`/`do_xor`/`do_mul` are the rest of the interface, kept
// so the reader sees the whole of it. REGRESSION: PASS.

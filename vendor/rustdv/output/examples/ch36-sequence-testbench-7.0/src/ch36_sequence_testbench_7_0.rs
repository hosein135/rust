//! Chapter 36: Sequence testbench 7.0 — stimulus leaves the structure.
//!
//!     sim-common/run_sim.sh ch36_sequence_testbench_7_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Testbench 6.0 made a new stimulus pattern mean a new *component*. Here the
//! structure holds still and the **program** changes: one env, one driver, and
//! a sequence chosen per test. The Primer puts it best — overriding the tester
//! to change stimulus is "like swapping out your car's steering wheel whenever
//! you chose a different destination."
//!
//! ## The cast
//!
//! - A **sequence** is not a component. No place in the tree, no path, no
//!   phases. One method, `body`, and a `SeqCtx` to run it against.
//! - The **sequencer** *is* a component. It holds the arbitration queue and
//!   hands out one export, and the env files a handle in the ConfigDb so a
//!   test three levels up can start sequences on it without knowing where it
//!   lives — pyuvm's idiom, and better than the Primer's `uvm_top.find()`
//!   string for the D83a reason.
//! - The **driver** pulls with `get_next_item()`, drives, and releases with
//!   `item_done()`. The `cmd_fifo` of 6.0 is gone: the sequencer is the
//!   decoupling point now.
//!
//! ## Why two calls and not one (the whole point of the chapter)
//!
//! `start_item` returns when the sequencer has granted this item its turn and
//! the driver is blocked waiting for its contents. Everything between
//! `start_item` and `finish_item` happens with the driver committed and
//! holding still — which is where **late stimulus setting** lives: a sequence
//! can look at the state of the testbench and decide what to send *now*,
//! rather than when it queued the item. A single `send(cmd).await` could not
//! express it, because the values would be fixed before arbitration ran.
//!
//! SystemVerilog had `mailbox#(T)` and built this two-phase rendezvous anyway;
//! pyuvm simplified nearly everything else about sequences and kept both
//! phases. The gap is the feature (D3).
//!
//! ## What this asks the framework for
//!
//! 1. `Sequencer<REQ, RSP>` as a **component** — `#[component]`,
//!    the D84 carve-out that `TlmFifo` already has: concrete, reachable, and
//!    never a factory-override target.
//! 2. `#[port(seq_item)]` and `SeqItemPort<REQ, RSP>` implementing `PortField`,
//!    so `Driver::SEQ_ITEM_PORT` is generated and `connect` resolves it through
//!    `ComponentNode::port_slot` like every other port (D83b). Required at
//!    elaboration (D85).
//! 3. `#[derive(Sequence)]` registering into a second link section, so
//!    `Factory::set_seq_override` works on objects that are not components
//!    (D80/D96). **Needed by this chapter**, not by ch39.
//! 4. `SeqCtx` carrying a seeded `rng()` and logging under the sequence's name
//!    (D98). A factory-built sequence is made by a `Default` maker and cannot
//!    be handed a seed at construction.
//! 5. `start` as a method on the sequence (D95).

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
// The driver
// ===========================================================================

// Chapter 36, Figure 2: The driver pulls items instead of being pushed them.
//
// The difference from 6.0 is not the direction of the data — it is who decides
// when. `get_next_item()` returns only when a sequence has an item ready *and*
// the driver asked for it: a rendezvous, not a queue.
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
            // 7.0 fires and forgets: no answer travels back, so the test holds
            // its objection for a flush. Chapter 38's driver answers, and then
            // the flush goes away.
            self.seq_item_port.item_done(None);
        }
    }
}

// ===========================================================================
// The sequences
// ===========================================================================

// Chapter 36, Figure 3: One body, three stimulus patterns.
//
// The Python book writes `BaseSeq` with a `body()` that loops the operations
// and calls `self.set_operands(tr)`, then subclasses it twice to override that
// one method. Rust has no inheritance, so the shared part is a **function** and
// the varying part is a **trait**. Same shape, no base class:
//
//   - `Operands` is the thing that varies — one method.
//   - `all_ops` is the thing that does not — the loop, the handshake, the
//     late-setting window.
//
// Each sequence implements `Operands` and hands itself to `all_ops`. Note that
// `set_operands` is called *between* `start_item` and `finish_item`, which is
// the whole reason the two calls exist.
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

// Chapter 36, Figure 4: The base sequence sends zeros.
//
// `create_seq::<BaseSeq>()` asks the factory for this type, so a test can
// substitute another sequence for it — the same mechanism as Chapter 29's
// component factory, in a second registry, because a sequence is not a
// `ComponentNode` and cannot ride the first one (D80).
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

// Chapter 36, Figure 5: Random and maximum operands.
//
// The RNG comes from the context, so a run reproduces from its seed the way
// every other part of the testbench does. pyuvm's sequences reach for the
// global `random` module and do not.
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
// Observation — Chapter 33's components, unchanged
// ===========================================================================

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

#[derive(Component, Default)]
struct ResultMonitor {
    #[port(publish)]
    ap: PublishPort<u64>,
}

impl Component for ResultMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let result = bfm.get_result().await;
            self.ap.write(&result);
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
        if Ops::ALL.iter().any(|op| !self.cvg.contains(op)) {
            errors.error("Functional coverage error: missed operations".to_string());
        } else {
            ctx.info("Covered all operations");
        }
    }
}

// ===========================================================================
// The environment
// ===========================================================================

// Chapter 36, Figure 6: The env owns the sequencer and files its handle.
//
// `#[component]` is the same carve-out D84 made for FIFOs: both
// endpoints of a connection are erased `RustdvComp` slots, so something
// concrete has to make the call. The connect line has the shape every
// connection in Chapters 31–34 had.
//
// The `"SEQR"` entry is how a test finds the sequencer without knowing where
// it lives — the same ConfigDb the BFM arrives through since Chapter 25.
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

// ===========================================================================
// The tests
// ===========================================================================

// Chapter 36, Figure 7: The test starts a sequence on the sequencer.
//
// It finds the sequencer in the ConfigDb — it does not know or care where in
// the tree it lives. `start` is a method on the *sequence*, taking the
// sequencer, exactly as both source books write it.
//
// Note where the lookup happens. pyuvm does it in `end_of_elaboration_phase`
// because a Python phase cannot return an error. rustdv does it in `run`,
// where `?` works and a missing SEQR is a named failure (D14).
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

        // Created through the factory, so a test can override which sequence
        // this line actually builds (Figure 7).
        let mut seq = create_seq::<BaseSeq>();
        seq.start(&seqr).await?;

        // `put` returns when the sequencer accepts the command, not when the
        // DUT has answered, so the last few results are still in flight. Hold
        // the objection for a flush — twenty clocks, because the multiply is
        // last and slowest.
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        for _ in 0..20 {
            bfm.clk().falling_edge().await;
        }
        Ok(())
    }
}

// Chapter 36, Figure 8: Two more tests, one testbench, no new components.
//
// This is what sequences bought. In Chapter 30 a new stimulus pattern meant a
// new *component* and a factory override on a component slot. Here it is a
// different **program** run through an unchanged structure, and the override is
// on a sequence type. Nothing in `AluEnv` knows either sequence exists.
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

// Expected: BaseTest drives all zeros, RandomTest drives seeded random
// operands, MaxTest drives 0xff — all four operations each, scoreboard
// PASSED four times per test, coverage complete. REGRESSION: PASS.

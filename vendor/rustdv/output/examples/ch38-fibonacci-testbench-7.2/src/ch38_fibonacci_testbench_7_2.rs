//! Chapter 38: Fibonacci — testbench 7.2, stimulus that needs the DUT's answers.
//!
//!     sim-common/run_sim.sh ch38_fibonacci_testbench_7_2 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! Chapter 37 taught the mechanism on a device that did nothing but wait.
//! Here it does real work on the real DUT: **the TinyALU computes the Fibonacci
//! numbers**, and it cannot be told the next pair until it has answered the
//! last one.
//!
//!     0  1  1  2  3  5  8  13  21
//!
//! Each command's operands are the answers to the two before it. There is no
//! way to generate this stimulus in advance — it has to be written one command
//! at a time, with the DUT's answer in hand. That is the reason the sequence
//! system exists, reduced to nine lines.
//!
//! ## What is different from Chapter 37
//!
//! Only the shape of the traffic. There, four requests were outstanding at once
//! and the ticket said which answer was which. Here **one command is in flight
//! at a time**, because the next one depends on this one's answer — so the
//! ticket is never ambiguous.
//!
//! It is still worth passing. `get_response(Some(id))` says what you mean, and
//! it keeps working when a later testbench pipelines the same sequence. Asking
//! for "whatever comes next" is right only for as long as there is only one
//! thing coming.
//!
//! ## The one thing that catches a UVM reader
//!
//! In SystemVerilog and Python the driver writes the answer *into the command*,
//! and the sequence — still holding a handle to it — reads the result back.
//! rustdv has no second name for one object, so `finish_item(cmd)` hands the
//! command over and the answer comes back as its **own value**. That is not a
//! feature that was lost: writing into a shared handle is what handles look
//! like in a language that has them (D93). The answer arrives one line later
//! either way.

use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{Ops, TinyAluBfm};

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
// A driver that waits for the answer
// ===========================================================================

// Chapter 38, Figure 1: The driver sends a command and returns its result.
//
// Chapter 36's driver fired and forgot. This one waits for *this* operation's
// answer before taking another item, and hands it back through `item_done`.
// The framework tags the response with the command's ticket, so nothing here
// does the equivalent of the UVM's `set_id_info(req)` — the call you could
// forget, and whose absence is a run-time fatal.
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

// ===========================================================================
// The Fibonacci sequence
// ===========================================================================

// Chapter 38, Figure 2: Nine numbers, eight of them from the DUT.
//
// Read the middle three lines as one gesture: hand the command over, wait,
// take the answer. `cmd` moves at `finish_item` — the sequence has no further
// use for it, so nothing is cloned. A sequence that *did* want to keep the
// command it sent would write `finish_item(cmd.clone())`, and the compiler
// would say so if it forgot.
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

        // A sequence can log. pyuvm's cannot — `uvm_sequence` is not a
        // `uvm_report_object`, so its Fibonacci reaches for `uvm_root().logger`
        // — and the Primer's uses a hand-typed string id. Here the line appears
        // under the sequence's own name (D98).
        ctx.info(&format!("Fibonacci Sequence: {fib:?}"));
        Ok(())
    }
}

// ===========================================================================
// The environment and the test
// ===========================================================================

// Chapter 38, Figure 3: No result monitor.
//
// The driver has the answer in hand, so it publishes results itself and the
// `ResultMonitor` of 6.0 and 7.0 is gone. The command monitor stays: nobody
// else is watching the bus.
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

#[derive(Default)]
struct SeenResults {
    results: Vec<u64>,
}
impl Subscriber<u64> for SeenResults {
    fn write(&mut self, r: &u64) {
        self.results.push(*r);
    }
}

// Chapter 38, Figure 4: A subscriber checks the DUT actually added.
//
// The sequence proves the numbers are Fibonacci; this proves they came from
// the adder rather than from the sequence's own arithmetic.
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

// Chapter 38, Figure 5: The test.
//
// No flush this time. `finish_item` returns only after the driver has the
// answer, so when the sequence ends there is nothing still in the pipeline —
// the twenty-clock wait Chapters 34 and 36 needed is a property of a driver
// that does not wait, not of sequences.
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

// Expected: Fibonacci Sequence: [0, 1, 1, 2, 3, 5, 8, 13, 21]
//           adder produced [1, 2, 3, 5, 8, 13, 21]
//           REGRESSION: PASS

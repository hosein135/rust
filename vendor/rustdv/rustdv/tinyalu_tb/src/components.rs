//! Testbench components, on the restored framework.
//!
//! Every component here is factory-buildable and takes **no constructor
//! arguments**: the BFM arrives from the ConfigDb (D101) and every TLM endpoint
//! is wired in the env's `connect` phase through `ComponentNode::port_slot`
//! (D83b). Nothing reaches into a sibling, and nothing is handed a handle at
//! construction — which is what makes the whole tree overridable.
//!
//! Work happens in `run`, not in a task spawned from `start`. The four run
//! phases here are concurrent (D82): the driver blocks on an empty sequencer
//! until a sequence sends something, and both monitors sit on the BFM's queues
//! at the same time. Each races the objection-drained event individually
//! (D82c), so a monitor that loops forever still gets checked.

use std::collections::HashMap;
use std::rc::Rc;

use rustdv::prelude::*;

use crate::alu_bfm::TinyAluBfm;
use crate::alu_item::{AluCommand, AluResult, Ops, predict};

// ===========================================================================
// Driver
// ===========================================================================

/// Port of `uvm_driver`: pulls items from the sequencer and drives the BFM.
///
/// `get_next_item` returns only when a sequence has an item ready *and* the
/// driver asked for it — a rendezvous, not a queue. The answer travels back as
/// an RSP where a chapter needs one; this testbench compares the two observed
/// streams instead, so it releases the sequence with `item_done(None)`.
#[derive(Component, Default)]
pub struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<AluCommand, AluResult>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let item = self.seq_item_port.get_next_item().await;
            bfm.send_op(item.payload().clone()).await;
            self.seq_item_port.item_done(None);
        }
    }
}

// ===========================================================================
// Monitors
// ===========================================================================

/// Watches the command bus and broadcasts what it sees. It knows nothing about
/// who is listening — the scoreboard and the coverage collector both subscribe
/// to the same bus, and neither is visible from here.
#[derive(Component, Default)]
pub struct CmdMonitor {
    #[port(publish)]
    ap: PublishPort<AluCommand>,
}

impl Component for CmdMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let cmd = bfm.get_cmd().await;
            ctx.info(&format!("cmd_monitor: {cmd:?}"));
            self.ap.write(&cmd);
        }
    }
}

/// The same shape for results.
#[derive(Component, Default)]
pub struct ResultMonitor {
    #[port(publish)]
    ap: PublishPort<AluResult>,
}

impl Component for ResultMonitor {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        loop {
            let res = bfm.get_result().await;
            ctx.info(&format!("result_monitor: {res:?}"));
            self.ap.write(&res);
        }
    }
}

// ===========================================================================
// Scoreboard
// ===========================================================================

// The subscriber owns the storage (D90). An `AnalysisBus` holds nothing: it
// calls every subscriber and returns, so *where the traffic goes* is the
// subscriber's decision. This scoreboard wants both streams in order, so it
// keeps a `Vec` of each and compares them in `check`.
//
// Two streams, two ports, two `write` methods — and no macros. SystemVerilog
// needs `uvm_analysis_imp_decl` to mint a second differently-named `write`, and
// pyuvm cannot do it at all with one `write` per class (D20/D88).

#[derive(Default)]
struct CmdLog {
    cmds: Vec<AluCommand>,
}

impl Subscriber<AluCommand> for CmdLog {
    fn write(&mut self, cmd: &AluCommand) {
        self.cmds.push(cmd.clone());
    }
}

#[derive(Default)]
struct ResultLog {
    results: Vec<AluResult>,
}

impl Subscriber<AluResult> for ResultLog {
    fn write(&mut self, res: &AluResult) {
        self.results.push(res.clone());
    }
}

/// Scoreboards check in `check` and report in `report`. Comparison policy lives
/// on the transaction — `PartialEq` against the predictor's output — which is
/// where `do_compare()` puts it.
#[derive(Component, Default)]
pub struct Scoreboard {
    #[port(subscribe)]
    cmd_in: SubscribePort<AluCommand>,
    #[port(subscribe)]
    result_in: SubscribePort<AluResult>,
    cmd_log: RustdvShared<CmdLog>,
    result_log: RustdvShared<ResultLog>,
    compared: usize,
    mismatches: usize,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.subscribe(self.cmd_log.clone());
        self.result_in.subscribe(self.result_log.clone());
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmd_log = self.cmd_log.get();
        let result_log = self.result_log.get();

        for (cmd, actual) in cmd_log.cmds.iter().zip(result_log.results.iter()) {
            let expected = predict(cmd);
            self.compared += 1;
            if expected != *actual {
                self.mismatches += 1;
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=FAIL"
                ));
                errors.error(format!(
                    "scoreboard mismatch: {cmd:?} -> got {actual:?}, expected {expected:?}"
                ));
            } else {
                ctx.info(&format!(
                    "scoreboard: in={cmd:?} out={actual:?} expected={expected:?} check=PASS"
                ));
            }
        }

        // A command with no result is a real failure and the zip would hide it:
        // the shorter stream simply ends the comparison. Say so explicitly.
        if cmd_log.cmds.len() != result_log.results.len() {
            errors.error(format!(
                "scoreboard: saw {} commands and {} results",
                cmd_log.cmds.len(),
                result_log.results.len()
            ));
        }
        if self.compared == 0 {
            errors.error("scoreboard: nothing was compared".to_string());
        }
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info(&format!(
            "scoreboard: {} compared, {} mismatches",
            self.compared, self.mismatches
        ));
    }
}

// ===========================================================================
// Coverage
// ===========================================================================

#[derive(Default)]
struct CovCollector {
    seen: HashMap<Ops, usize>,
}

impl Subscriber<AluCommand> for CovCollector {
    fn write(&mut self, cmd: &AluCommand) {
        *self.seen.entry(cmd.op).or_insert(0) += 1;
    }
}

/// Functional coverage as a second subscriber on the command bus: it counts the
/// ops it saw and errors in `check` if any was never exercised. The command
/// monitor does not know it exists, and neither does the scoreboard.
#[derive(Component, Default)]
pub struct Coverage {
    #[port(subscribe)]
    cmd_in: SubscribePort<AluCommand>,
    collector: RustdvShared<CovCollector>,
}

impl Component for Coverage {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.cmd_in.subscribe(self.collector.clone());
    }

    fn check(&mut self, _ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let seen = &self.collector.get().seen;
        for op in Ops::ALL {
            if !seen.contains_key(&op) {
                errors.error(format!("coverage: op {op:?} was never exercised"));
            }
        }
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = &self.collector.get().seen;
        let mut parts: Vec<String> = Ops::ALL
            .iter()
            .map(|op| format!("{op:?}={}", seen.get(op).copied().unwrap_or(0)))
            .collect();
        parts.sort();
        ctx.info(&format!("coverage: {}", parts.join(" ")));
    }
}

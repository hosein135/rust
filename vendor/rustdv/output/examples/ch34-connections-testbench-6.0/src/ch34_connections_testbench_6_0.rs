//! Chapters 33 and 34: Testbench 6.0 — the components, and the wiring.
//!
//!     sim-common/run_sim.sh ch34_connections_testbench_6_0 tinyalu \
//!         sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv
//!
//! **Two chapters, one crate (D91).** Chapter 33 refactors the 6.0 components
//! so each does one job — it is a chapter of *definitions*, with no environment
//! and nothing to run — and Chapter 34 wires them together. They share this
//! file because D45 requires a chapter example to be self-contained: splitting
//! the components into a crate that Chapter 34 imports is precisely the
//! cross-chapter import D45 dissolved. The captions carry the chapter number,
//! so `// Chapter 33, Figure 1:` and `// Chapter 34, Figure 2:` coexist here
//! and each chapter's README maps its own figures.
//!
//! Chapter 33 owns Figures 1–6 (Tester, Driver, the two monitors, Scoreboard,
//! Coverage); Chapter 34 owns Figures 2–3 (the env, and the test).
//!
//! Built and green on Icarus (2026-07-28). This is the payoff chapter: put/get
//! (Chapter 31) and analysis broadcast (Chapter 32) wired into one working
//! TinyALU testbench, every connection resolved through `ComponentNode::port_slot`
//! (D83b) so no parent reaches into an erased child.
//!
//! ## Architecture (the book's 6.0)
//!
//! ```text
//!   Tester --put--> [cmd_fifo] --get--> Driver --> BFM --> DUT
//!
//!   CmdMonitor --pub--> [cmd_bus] --sub--> Scoreboard
//!                            \----sub----> Coverage
//!
//!   ResultMonitor --pub--> [result_bus] --sub--> Scoreboard
//! ```
//!
//! One idiom throughout: a concrete FIFO between the two components, a named
//! export, `connect(component, PORT_NAME)`. `TlmFifo` carries point-to-point
//! traffic; `AnalysisBus` brokers a broadcast — several subscribers connect
//! to the same `sub_export()`.
//!
//! The Tester generates commands and *puts* them; the Driver *gets* them and
//! drives the BFM. Two monitors watch the bus and *broadcast* what they see;
//! the Scoreboard and Coverage *subscribe*. The BFM comes from the ConfigDb,
//! filed there by the test (D101); the RTL self-clocks (D42).
//!
//! ## The Rust win worth noting (D20): multiple analysis inputs, no macros
//!
//! The Scoreboard needs two analysis streams — commands and results. SV cannot
//! give one class two `write` methods, so it needs the `uvm_analysis_imp_decl`
//! macros; pyuvm cannot do it at all with one `write` per class. In rustdv each
//! stream gets its own `SubscribePort` and its own sink struct, so the
//! Scoreboard has two `write` methods and needs no macros — **and it works the
//! same way when both streams carry the same type**, which is the case the SV
//! macros actually exist for (D88).
//!
//! ## Concurrent runs and real objections (D82)
//!
//! Tester, Driver, and both monitors have run phases that run at once (the
//! Driver blocks on an empty cmd_fifo until the Tester puts). `run_all` joins
//! them and each component races the objection-drained event individually.
//! This chapter is what caught D82c: racing the *whole* run tree dropped it
//! mid-phase, destroying the components before `check` could walk them, and
//! the test passed with its scoreboard never running.

use std::collections::HashSet;
use std::rc::Rc;

use rustdv::prelude::*;
use tinyalu_utils::{alu_prediction, CmdTuple, Ops, TinyAluBfm};

rustdv::vpi_bootstrap!();

// A command the tester hands the driver: operands + operation.
type Command = (u8, u8, Ops);

// ===========================================================================
// Stimulus: Tester -> cmd_fifo -> Driver
// ===========================================================================

// Chapter 33, Figure 1: The Tester puts commands into a FIFO.
#[derive(Component, Default)]
struct Tester {
    #[port(put)]
    cmd_port: PutPort<Command>,
    rng: Option<Rng>,
}

impl Component for Tester {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        self.rng = Some(ctx.rng());
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("stimulus");
        let rng = self.rng.as_mut().expect("build ran");
        for op in Ops::ALL {
            self.cmd_port.put((rng.u8(), rng.u8(), op)).await;
        }
        // `put` returns as soon as the FIFO takes the command, not when the
        // DUT has answered it — so dropping the objection here would end the
        // phase with commands still in the pipeline and results in flight, and
        // the scoreboard would silently check fewer results than it saw
        // commands. Hold the objection for a flush, as the Python testbench
        // does. It waits ten clocks; this waits twenty, because the multiply
        // is the last operation and takes the longest to come back.
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        for _ in 0..20 {
            bfm.clk().falling_edge().await;
        }
        Ok(())
    }
}

// Chapter 33, Figure 2: The Driver gets commands and drives the BFM.
#[derive(Component, Default)]
struct Driver {
    #[port(get)]
    cmd_port: GetPort<Command>,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM")?;
        bfm.reset().await;
        loop {
            let (aa, bb, op) = self.cmd_port.get().await; // blocks until a command
            bfm.send_op(aa, bb, op).await;
        }
    }
}

// ===========================================================================
// Observation: monitors broadcast, subscribers collect
// ===========================================================================

// Chapter 33, Figure 3: The command monitor watches the bus and broadcasts.
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

// Chapter 33, Figure 4: The result monitor broadcasts results.
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

// Chapter 33, Figure 5: The Scoreboard subscribes to BOTH streams.
//
// Two `Subscriber` impls, one per transaction type — the multiple-analysis-input
// pattern that needs no imp_decl macros (D20).
// Each stream gets its own sink struct and its own port. Two ports, two
// `write` methods — and it would work identically if both streams carried the
// *same* type, which is the case SV needs `uvm_analysis_imp_decl` macros for
// and pyuvm cannot express with one `write` per class (D88).
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
    fn write(&mut self, result: &u64) {
        self.results.push(*result);
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
    cvg: HashSet<Ops>,
}

impl Component for Scoreboard {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_cmds = self.cmd_log.clone();
        self.cmd_in.subscribe(my_cmds);

        let my_results = self.result_log.clone();
        self.result_in.subscribe(my_results);
    }

    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let cmd_log = self.cmd_log.get();
        let result_log = self.result_log.get();
        for (cmd, result) in cmd_log.cmds.iter().zip(result_log.results.iter()) {
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

// Chapter 33, Figure 6: Coverage subscribes to the command stream only.
//
// A second subscriber on `cmd_bus` — the scoreboard does not know it is there,
// and the monitor does not know either. That is the decoupling the hub buys.
#[derive(Default)]
struct OpsSeen {
    ops: HashSet<Ops>,
}

impl Subscriber<CmdTuple> for OpsSeen {
    fn write(&mut self, cmd: &CmdTuple) {
        if let Some(op) = Ops::from_u64(cmd.2) {
            self.ops.insert(op);
        }
    }
}

#[derive(Component, Default)]
struct Coverage {
    #[port(subscribe)]
    cmd_in: SubscribePort<CmdTuple>,
    seen: RustdvShared<OpsSeen>,
}

impl Component for Coverage {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_subscriber = self.seen.clone();
        self.cmd_in.subscribe(my_subscriber);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = self.seen.get();
        ctx.info(&format!("coverage saw {} of {} ops", seen.ops.len(), Ops::ALL.len()));
    }
}

// ===========================================================================
// The environment wires it all together
// ===========================================================================

// Chapter 34, Figure 2: build the components and the FIFOs; connect in one
// place. **Every connection is the same shape** — a concrete FIFO, a named
// export, and `connect(component, PORT_NAME)` — whether the traffic is
// point-to-point (`TlmFifo`) or broadcast (`AnalysisBus`). Nothing reaches
// into an erased child; every endpoint is reached through a trait method that
// answers the same way for a child slot and for `self` (D83b).
#[derive(Component, Default)]
struct AluEnv {
    #[component]
    tester: RustdvComp,
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
    cmd_fifo: TlmFifo<Command>,
    #[component]
    cmd_bus: AnalysisBus<CmdTuple>, // the command broadcast, two subscribers
    #[component]
    result_bus: AnalysisBus<u64>, // the result broadcast, one subscriber
}

impl Component for AluEnv {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.tester = Tester::new_comp();
        self.driver = Driver::new_comp();
        self.cmd_mon = CmdMonitor::new_comp();
        self.result_mon = ResultMonitor::new_comp();
        self.scoreboard = Scoreboard::new_comp();
        self.coverage = Coverage::new_comp();
        self.cmd_fifo = TlmFifo::new(1);
        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // stimulus: Tester --put--> cmd_fifo --get--> Driver
        self.cmd_fifo.put_export().connect(&self.tester, Tester::CMD_PORT);
        self.cmd_fifo.get_export().connect(&self.driver, Driver::CMD_PORT);

        // commands: CmdMonitor publishes; Scoreboard and Coverage subscribe
        self.cmd_bus.pub_export().connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus.sub_export().connect(&self.scoreboard, Scoreboard::CMD_IN);
        self.cmd_bus.sub_export().connect(&self.coverage, Coverage::CMD_IN);

        // results: ResultMonitor publishes; only the Scoreboard subscribes
        self.result_bus.pub_export().connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus.sub_export().connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}

// Chapter 34, Figure 3: the test is just the env.
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
}

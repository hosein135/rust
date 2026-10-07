//! The environment: build the components, wire them, and get out of the way.
//!
//! Two phases do the work the constructor used to. `build` creates the children
//! top-down, so a test above can override any of them before they exist;
//! `connect` wires them bottom-up, once they all do. The gap between the two is
//! not overhead — it is where configuration, factory overrides and TLM
//! connection all live (D5/D6).
//!
//! **Every connection has the same shape**: a concrete endpoint holder, a named
//! export, `connect(component, PORT_NAME)`. It reads the same whether the
//! traffic is a sequencer rendezvous, point-to-point, or a broadcast, and
//! nothing reaches into an erased child — each endpoint is found through a trait
//! method that answers identically for a child slot and for `self` (D83b).
//!
//! ```text
//!   sequences --> [seqr] --> Driver --> BFM --> DUT
//!
//!   CmdMonitor    --pub--> [cmd_bus]    --sub--> Scoreboard
//!                                \-------sub--> Coverage
//!   ResultMonitor --pub--> [result_bus] --sub--> Scoreboard
//! ```

use std::rc::Rc;

use rustdv::prelude::*;

use crate::alu_bfm::TinyAluBfm;
use crate::alu_item::{AluCommand, AluResult};
use crate::components::{CmdMonitor, Coverage, Driver, ResultMonitor, Scoreboard};

/// The environment. Every child is a `RustdvComp` — an erased slot the factory
/// fills — except the sequencer and the two buses, which are concrete because
/// something has to make the `connect` call and because plumbing is never an
/// override target (D84).
#[derive(Component, Default)]
pub struct AluEnv {
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
    cmd_bus: AnalysisBus<AluCommand>,
    #[component]
    result_bus: AnalysisBus<AluResult>,
    // What `build` resolved, kept for `connect` to read. `Active` is the
    // configured type — an enum, so an illegal value cannot be filed — and this
    // is the answer after the lookup.
    is_active: bool,
    with_coverage: bool,
}

impl Component for AluEnv {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        // Two choices a test can make from outside, both with a default, so the
        // ordinary case configures nothing. A passive env has no driver at all —
        // the slot is left empty rather than holding a driver told not to drive.
        let activity: Active = ConfigDb::get(Some(ctx), "", "IS_ACTIVE").unwrap_or(Active::Active);
        self.is_active = activity == Active::Active;
        self.with_coverage = ConfigDb::get(Some(ctx), "", "WITH_COVERAGE").unwrap_or(true);

        self.seqr = Sequencer::new();
        // How a test three levels up starts a sequence without knowing where the
        // sequencer lives — the same ConfigDb the BFM arrives through.
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());

        if self.is_active {
            self.driver = Driver::create_comp();
        }
        self.cmd_mon = CmdMonitor::create_comp();
        self.result_mon = ResultMonitor::create_comp();
        self.scoreboard = Scoreboard::create_comp();
        if self.with_coverage {
            self.coverage = Coverage::create_comp();
        }

        self.cmd_bus = AnalysisBus::new();
        self.result_bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        if self.is_active {
            self.seqr
                .seq_item_export()
                .connect(&self.driver, Driver::SEQ_ITEM_PORT);
        }

        self.cmd_bus
            .pub_export()
            .connect(&self.cmd_mon, CmdMonitor::AP);
        self.cmd_bus
            .sub_export()
            .connect(&self.scoreboard, Scoreboard::CMD_IN);
        if self.with_coverage {
            self.cmd_bus
                .sub_export()
                .connect(&self.coverage, Coverage::CMD_IN);
        }

        self.result_bus
            .pub_export()
            .connect(&self.result_mon, ResultMonitor::AP);
        self.result_bus
            .sub_export()
            .connect(&self.scoreboard, Scoreboard::RESULT_IN);
    }

    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        // The BFM's collector tasks must outlive this phase, so they are spawned
        // rather than composed — which is what `spawn` is reserved for (D82).
        // Everything above the BFM is a `run` phase instead.
        let bfm: Rc<TinyAluBfm> = ConfigDb::get(Some(ctx), "", "BFM").expect("the test sets BFM");
        bfm.start_tasks();
    }
}

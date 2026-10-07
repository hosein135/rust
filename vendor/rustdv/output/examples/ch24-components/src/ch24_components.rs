//! Chapter 24: Components — the lifecycle and the ownership tree.
//!
//!     sim-common/run_sim.sh ch24_components playground
//!
//! No DUT needed: structure is the subject.
//!
//! This is where the UVM's phase lifecycle comes back. The previous rustdv
//! pass had **destroyed** `build` and `connect` — demoted to "constructor
//! conventions" (review-memo R3) — leaving only five of the nine phases and
//! driving them by hand. D5/D6 restore them as real phase methods; the
//! runner drives the whole sequence, exactly as pyuvm's phaser does; and a
//! parent grows its subtree in its own `build` phase (D6). Ports of the
//! Python book's chapter 28, Figures 1 and 4-6. See
//! `output/.design-decisions.md`.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// First half — running the phases
// ===========================================================================

// Chapter 24, Figure 1: A uvm_test demonstrating the phase methods.
//
// The test IS a component (`#[rustdv::test]` registers it). There is no free
// test function and no hand-rolled `start_all`: the runner drives every
// phase in order, the way `@pyuvm.test()` hands the class to the phaser.
//
// A component overrides only the phases it uses; the rest default to no-ops.
// Every phase receives the context, so its log line carries the component's
// path (D7): the output reads `[PhaseTest]`, the path the walk derived, not
// a hand-typed string that could lie.
//
// Order is pyuvm's, not SV UVM's (D34): build top-down, connect bottom-up,
// run bottom-up, and the elaboration and post-run phases top-down.

#[rustdv::test]
#[derive(Component, Default)]
struct PhaseTest;

impl Component for PhaseTest {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("1 build");
    }
    fn connect(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("2 connect");
    }
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("3 end_of_elaboration");
    }
    fn start_of_simulation(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("4 start_of_simulation");
    }
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("run");
        ctx.info("5 run");
        Ok(())
    }
    fn extract(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("6 extract");
    }
    fn check(&mut self, ctx: &mut RustdvCtx, errors: &mut CheckSink) {
        let _ = errors;
        ctx.info("7 check");
    }
    fn report(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("8 report");
    }
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("9 final");
    }
}

// ===========================================================================
// Second half — building the hierarchy (TestTop -> mc -> bc)
// ===========================================================================
//
// The Python book's Figures 4-6: a three-level tree where each parent
// *creates its children in its own build phase* and the phaser recurses
// into them. This is D6's two-stage construction — a child is an
// `Option<T>` field, "declared but not yet built," and `build` fills it in.
// The ownership tree is still the component tree; `build` is where it grows.

// Chapter 24, Figure 6: the bottom component. Only a run phase, which
// objects, logs under its path (uvm_test_top.mc.bc in UVM; TestTop.mc.bc
// here), and drops.
#[derive(Component, Default)]
struct BottomComp;

impl Component for BottomComp {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("bc run");
        ctx.info("run phase");
        Ok(())
    }
}

// Chapter 24, Figure 5: the middle component builds the bottom component in
// its own build phase, and announces itself at end of elaboration.
#[derive(Component, Default)]
struct MiddleComp {
    #[component]
    bc: Option<BottomComp>,
}

impl Component for MiddleComp {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.bc = Some(BottomComp::default());
    }
    fn end_of_elaboration(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("end of elaboration phase");
    }
}

// Chapter 24, Figure 4: the test at the top. Its build phase constructs the
// middle component; the phaser descends into the tree that build creates.
#[rustdv::test]
#[derive(Component, Default)]
struct TestTop {
    #[component]
    mc: Option<MiddleComp>,
}

impl Component for TestTop {
    fn build(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("build phase");
        self.mc = Some(MiddleComp::default());
    }
    fn final_phase(&mut self, ctx: &mut RustdvCtx) {
        ctx.info("final phase");
    }
}

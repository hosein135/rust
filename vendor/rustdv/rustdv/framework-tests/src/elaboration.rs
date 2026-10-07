//! `elab_` — an unconnected port stops the test before the run phase.
//!
//! D22/D85: rustdv sweeps the whole tree between `connect` and
//! `end_of_elaboration` and names **every** miss at once. pyuvm finds the
//! first one lazily, at use, deep inside a run phase — by which point the
//! transcript is full of unrelated output and the message is a null
//! dereference rather than a path.
//!
//! Chapter 31's Figure 11 shows this to a reader. It is a chapter figure,
//! though, so what it proves is that chapter 31 still runs. These tests
//! prove the rule.

use rustdv::prelude::*;

// A component that asks for an interface nobody gives it.
#[derive(Component, Default)]
struct Orphan {
    #[port(get)]
    items: GetPort<u8>,
}

impl Component for Orphan {}

// Two ports, both unconnected, so the sweep has more than one thing to find.
#[derive(Component, Default)]
struct TwoOrphans {
    #[port(get)]
    items: GetPort<u8>,
    #[port(put)]
    outbox: PutPort<u8>,
}

impl Component for TwoOrphans {}

#[derive(Component, Default)]
struct OrphanNest {
    #[component]
    one: RustdvComp,
    #[component]
    two: RustdvComp,
}

impl Component for OrphanNest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.one = Orphan::new_comp();
        self.two = TwoOrphans::new_comp();
    }
}

// An unconnected port fails elaboration, classified so a test can name it.
//
// `expect_error` is the strong form: `expect_fail` would pass if this test
// failed for any reason at all, including a panic from somewhere unrelated.
#[rustdv::test(expect_error = "tlm_unconnected_port")]
#[derive(Component, Default)]
struct ElabUnconnectedPortFailsElaboration {
    #[component]
    orphan: RustdvComp,
}

impl Component for ElabUnconnectedPortFailsElaboration {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.orphan = Orphan::new_comp();
    }

    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        // Never reached: elaboration fails first, which is the point. If the
        // check ever moved to first use, this would run and the test would
        // fail with the wrong message.
        Err(TestError::new(
            "the run phase started even though a port was unconnected",
        ))
    }
}

// Every miss is reported in one sweep, each with the path that owns it.
//
// Driven by hand rather than through `#[rustdv::test]` on a struct, because
// the phaser turns the first miss into an `Err` and returns — and the claim
// under test is about what the sweep found *before* that.
#[rustdv::test]
async fn elab_reports_every_miss_with_paths(ctx: RustdvCtx) -> Result<(), TestError> {
    let mut ctx = ctx;
    let mut top = OrphanNest::default();

    build_all(&mut top, &mut ctx);
    connect_all(&mut top, &mut ctx);
    let missing = rustdv::unconnected_ports(&mut top, &mut ctx);

    check!(
        missing.len() == 3,
        "expected all three unconnected ports in one sweep, got {missing:?}"
    );
    for want in ["one", "two", "items", "outbox"] {
        check!(
            missing.iter().any(|m| m.contains(want)),
            "no report mentions {want:?}: {missing:?}"
        );
    }
    Ok(())
}

// The positive control: a connected tree elaborates and runs clean.
//
// Without it, a `check_connections` that failed everything would pass every
// test above.
#[derive(Component, Default)]
struct Consumer {
    #[port(get)]
    items: GetPort<u8>,
}

impl Component for Consumer {
    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let item = self.items.get().await;
        if item != 7 {
            return Err(TestError::new(format!("got {item}, expected 7")));
        }
        Ok(())
    }
}

#[rustdv::test(timeout_time = 10, timeout_unit = "us")]
#[derive(Component, Default)]
struct ElabConnectedTreeIsClean {
    #[component]
    fifo: TlmFifo<u8>,
    #[component]
    consumer: RustdvComp,
}

impl Component for ElabConnectedTreeIsClean {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo = TlmFifo::unbounded();
        self.consumer = Consumer::new_comp();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo
            .get_export()
            .connect(&self.consumer, Consumer::ITEMS);
    }

    async fn run(&mut self, _ctx: &mut RustdvCtx) -> Result<(), TestError> {
        Timer::ns(1).await;
        self.fifo.put(7).await;
        Ok(())
    }
}

//! Chapter 31: Component communications — put, get, and peek through a FIFO.
//!
//!     sim-common/run_sim.sh ch31_component_communications playground
//!
//! Built and green on Icarus (2026-07-28). This chapter replaced the
//! pre-restoration `channel`/`Sender`/`Receiver` design — "channels replace
//! TLM-1" was the destroyed model this branch exists to undo.
//!
//! ## The model (D17–D24)
//!
//! TLM 1.0 point-to-point: a **port** in one component is the thing you *call*
//! (`put`, `get`, `peek`); it forwards to an **export** on a FIFO; the FIFO
//! owns a `Queue` and implements every capability against it. Two components
//! connect to the *same FIFO* and neither learns the other exists — the FIFO is
//! the decoupling point (§0.3.2). Each capability has a blocking form (waits on
//! full/empty) and a non-blocking form (`try_*`/`can_*`).
//!
//! ## Connection without reaching into an erased child (D83b)
//!
//! A component is a factory `RustdvComp`, so a parent cannot reach
//! `producer.put_port` by field, and Rust has no `$cast`-to-base to recover the
//! concrete type. Instead `#[derive(Component)]` generates
//! `ComponentNode::port_slot(name)` from the `#[port(..)]` fields — **a trait
//! method, which is reachable *through* `dyn` where a cast is not**. The export
//! initiates the connection and names the port with a derive-generated
//! constant: `fifo.put_export().connect(&self.producer, Producer::PUT_PORT)`.
//!
//! Because `port_slot` answers through `dyn`, the *same* call works when a
//! component connects its **own** port (`connect(self, MathTest::X_OUT)`,
//! Figure 10) — that uniformity is the point, and it is why `uvm_test` is a
//! `uvm_component`. An earlier design keyed a registry on the hierarchical
//! path; it could address a child but not the connecting component itself,
//! because a component does not know its own path (D7). It was struck (D83b).
//!
//! `#[port(put)] put_port` generates `Producer::PUT_PORT`, a typed
//! `PortName<dyn PutIf<u32>>`, so a misspelling does not compile and aiming a
//! `get` export at a `put` port does not compile either — the name carries the
//! interface, not just a string.
//!
//! The FIFO is a concrete `#[component]` child (reachable to call
//! `put_export()` on) — the model closest to UVM, where the FIFO is a real
//! component with a path. It is a deliberate carve-out: a FIFO is plumbing,
//! never a factory-override target (like the BFM, D33). Every declared put/get
//! port must be connected by end of elaboration or elaboration fails (D22/D85).
//!
//! ## Why this chapter forced concurrent run phases (D82)
//!
//! Put/get through a size-1 FIFO **requires two run phases running at once**:
//! the producer blocks on a full FIFO and can only proceed once the consumer
//! drains it. Under the sequential `run_all` that preceded this work it
//! deadlocked. Component Communications is the testbench D56/D60 anticipated:
//! *"concurrent run phases + meaningful objections arrive together... when a
//! testbench first needs two long-running run phases at once."* `run_all` now
//! **joins** the children's runs rather than spawning them (D82 — join, not
//! spawn, so a run future may borrow the tree), a parent's own `run` joins its
//! children's by moving them out of their slots (D82b), and each component
//! races the objection-drained event individually (D82c).

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Blocking put / get / peek
// ===========================================================================

// Chapter 31, Figure 1: A producer holds a put port and blocks on a full FIFO.
//
// `#[port(put)]` makes `put_port` reachable by name through `port_slot`, so the
// env can wire it without reaching in. `PutPort<u32>` offers blocking
// `put().await` (here) and non-blocking `try_put`/`can_put` (Figure 4).
#[derive(Component, Default)]
struct Producer {
    #[port(put)]
    put_port: PutPort<u32>,
}

impl Component for Producer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("producing");
        for n in 0..3 {
            self.put_port.put(n).await; // blocks while the FIFO is full
            ctx.info(&format!("put {n}"));
        }
        Ok(())
    }
}

// Chapter 31, Figure 2: A consumer that peeks, then gets.
//
// Two ports on one component, both wired to the same FIFO: `peek` returns the
// next item **without** consuming it (blocking until one is there), and `get`
// then consumes that same item. The assert makes the "peek does not consume"
// contract executable.
#[derive(Component, Default)]
struct Consumer {
    #[port(peek)]
    peek_port: PeekPort<u32>,
    #[port(get)]
    get_port: GetPort<u32>,
}

impl Component for Consumer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("consuming");
        for _ in 0..3 {
            let seen = self.peek_port.peek().await; // blocks while empty; no consume
            ctx.info(&format!("peeked {seen}"));
            let got = self.get_port.get().await; // consumes the peeked item
            assert_eq!(seen, got, "peek must not consume the item");
            ctx.info(&format!("got {got}"));
        }
        Ok(())
    }
}

// Chapter 31, Figure 3: The env builds the two components and a FIFO, then wires
// them in `connect`.
//
// The producer and consumer are ordinary factory `RustdvComp` children. The FIFO
// is a concrete `#[component]` child so its exports are reachable. Every
// `connect` is port -> export resolved through `port_slot`: the export never
// reaches into the erased child.
#[rustdv::test]
#[derive(Component, Default)]
struct PutGetPeekTest {
    #[component]
    producer: RustdvComp,
    #[component]
    consumer: RustdvComp,
    #[component]
    fifo: TlmFifo<u32>,
}

impl Component for PutGetPeekTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = Producer::new_comp();
        self.consumer = Consumer::new_comp();
        self.fifo = TlmFifo::new(1); // depth 1 — forces the producer to wait
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo.put_export().connect(&self.producer, Producer::PUT_PORT);
        self.fifo.peek_export().connect(&self.consumer, Consumer::PEEK_PORT);
        self.fifo.get_export().connect(&self.consumer, Consumer::GET_PORT);
    }
}

// Expected transcript (both run phases concurrent, objection-gated):
//   put 0 / peeked 0 / got 0 / put 1 / peeked 1 / got 1 / put 2 / peeked 2 /
//   got 2   (interleaving may vary; every item is peeked before it is got, and
//   got exactly once)   REGRESSION: PASS

// ===========================================================================
// Non-blocking put / get
// ===========================================================================

// A transaction, not an integer. It owns a `String`, so it is **not `Copy`** —
// like every real transaction you will put on a port. The figures below are
// written against it deliberately: a `u32` would let a retry loop compile that
// falls apart the moment the reader substitutes their own command type.
#[derive(Debug)]
struct Packet {
    n: u32,
    label: String,
}

impl Packet {
    fn new(n: u32) -> Packet {
        Packet { n, label: format!("pkt{n}") }
    }
}

// Chapter 31, Figure 5: A non-blocking producer never waits — `try_put` fails
// when the FIFO is full, and the producer decides what to do (here, yield and
// retry).
//
// **Read the `Err` binding.** The UVM's `try_put` returns a bit, because SV
// passes a class handle and the caller keeps its copy. rustdv's `try_put` takes
// the packet *by value* — it has to, since a successful put hands the packet to
// whoever gets it next — so a bare "no" would have eaten a packet that was never
// delivered. `Err(back)` is the packet coming home, and `packet = back;` is the
// retry loop taking it back for the next attempt.
//
// This is what the type system will not let you get wrong: drop the `Err`
// binding and the loop does not compile, because `packet` was moved on the
// first attempt.
#[derive(Component, Default)]
struct NbProducer {
    #[port(put)]
    put_port: PutPort<Packet>,
}

impl Component for NbProducer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("producing (nb)");
        for n in 0..3 {
            let mut packet = Packet::new(n);
            while let Err(back) = self.put_port.try_put(packet) {
                ctx.info("FIFO full, retrying");
                Timer::ns(1).await;
                packet = back; // the FIFO gave it back; try again with it
            }
            ctx.info(&format!("put {n}"));
        }
        Ok(())
    }
}

// Chapter 31, Figure 6: A non-blocking consumer — `try_get` returns `None` when
// the FIFO is empty.
//
// `Option<Packet>` for the same reason `Result<(), Packet>` above: a get either
// hands you a packet or hands you nothing, and there is no third state where a
// packet exists but nobody owns it.
#[derive(Component, Default)]
struct NbConsumer {
    #[port(get)]
    get_port: GetPort<Packet>,
}

impl Component for NbConsumer {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("consuming (nb)");
        let mut seen = 0;
        while seen < 3 {
            match self.get_port.try_get() {
                Some(packet) => {
                    // The whole packet arrived, label and all — the consumer
                    // now owns it, and the FIFO no longer does.
                    ctx.info(&format!("got {} (n={})", packet.label, packet.n));
                    seen += 1;
                }
                None => Timer::ns(1).await,
            }
        }
        Ok(())
    }
}

// Chapter 31, Figure 7: Same wiring, non-blocking components.
//
// The FIFO now carries `Packet` rather than `u32`, and not one connect line
// changed shape. That is the generic FIFO doing its job — and the typed
// `PortName` doing its job too: connect this `TlmFifo<Packet>` to a port that
// wants `u32` and the mistake is a compile error, not a run-time surprise.
#[rustdv::test]
#[derive(Component, Default)]
struct NonBlockingTest {
    #[component]
    producer: RustdvComp,
    #[component]
    consumer: RustdvComp,
    #[component]
    fifo: TlmFifo<Packet>,
}

impl Component for NonBlockingTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = NbProducer::new_comp();
        self.consumer = NbConsumer::new_comp();
        self.fifo = TlmFifo::new(1);
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.fifo.put_export().connect(&self.producer, NbProducer::PUT_PORT);
        self.fifo.get_export().connect(&self.consumer, NbConsumer::GET_PORT);
    }
}

// ===========================================================================
// The parent runs too — a three-stage pipeline
// ===========================================================================

// Chapter 31, Figure 9: A processing pipeline — y = 2x².
//
// Everything so far had a parent that only built and connected. Here the
// **test itself is a stage**: it chooses x, sends it into the pipeline, and
// reads the answer back for comparison. Two worker components do the
// arithmetic, each connected only to FIFOs — neither knows the other exists.
//
//     MathTest(run) --put--> [x_fifo] --get--> SquareIt(run)
//                                                  |  put
//                                              [sq_fifo]
//                                                  |  get
//                                              TimesTwo(run)
//                                                  |  put
//     MathTest(run) <--get-- [y_fifo] <-------------
//
// x -> x² -> 2x², so the test compares each answer against 2x².
//
// **Every run phase must make progress together.** The test blocks waiting
// for its answer; SquareIt blocks waiting for x; TimesTwo blocks waiting for
// x². Nothing completes unless everything runs at once — and the parent is
// one of the things that must run. A phaser that finishes the children before
// starting the parent cannot execute this at all: the children wait forever
// for an x the parent never gets to send.
//
// This is the shape the sequence chapters need (TB 7.0's test starts a
// sequence while the driver waits for items), reduced to arithmetic with no
// DUT and a self-checking answer.

// Chapter 31, Figure 10: The first stage squares its input.
#[derive(Component, Default)]
struct SquareIt {
    #[port(get)]
    input: GetPort<u32>,
    #[port(put)]
    output: PutPort<u32>,
}

impl Component for SquareIt {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        loop {
            let x = self.input.get().await; // waits for the test to send x
            ctx.info(&format!("{x}² = {}", x * x));
            self.output.put(x * x).await; // waits for TimesTwo to take it
        }
    }
}

// Chapter 31, Figure 11: The second stage doubles what the first produced.
#[derive(Component, Default)]
struct TimesTwo {
    #[port(get)]
    input: GetPort<u32>,
    #[port(put)]
    output: PutPort<u32>,
}

impl Component for TimesTwo {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        loop {
            let sq = self.input.get().await; // waits for SquareIt
            ctx.info(&format!("2 × {sq} = {}", 2 * sq));
            self.output.put(2 * sq).await; // waits for the test to read it
        }
    }
}

// Chapter 31, Figure 12: The test drives the pipeline and checks the answer.
//
// The test owns three FIFOs and two workers, and holds ports of its own: it
// puts x and gets y. Its `run` is the stimulus *and* the comparison.
#[rustdv::test]
#[derive(Component, Default)]
struct MathTest {
    #[component]
    square_it: RustdvComp,
    #[component]
    times_two: RustdvComp,
    #[component]
    x_fifo: TlmFifo<u32>,
    #[component]
    sq_fifo: TlmFifo<u32>,
    #[component]
    y_fifo: TlmFifo<u32>,
    #[port(put)]
    x_out: PutPort<u32>,
    #[port(get)]
    y_in: GetPort<u32>,
}

impl Component for MathTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.square_it = SquareIt::new_comp();
        self.times_two = TimesTwo::new_comp();
        self.x_fifo = TlmFifo::new(1);
        self.sq_fifo = TlmFifo::new(1);
        self.y_fifo = TlmFifo::new(1);
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // test -> square_it
        self.x_fifo.put_export().connect(self, MathTest::X_OUT);
        self.x_fifo.get_export().connect(&self.square_it, SquareIt::INPUT);
        // square_it -> times_two
        self.sq_fifo.put_export().connect(&self.square_it, SquareIt::OUTPUT);
        self.sq_fifo.get_export().connect(&self.times_two, TimesTwo::INPUT);
        // times_two -> test
        self.y_fifo.put_export().connect(&self.times_two, TimesTwo::OUTPUT);
        self.y_fifo.get_export().connect(self, MathTest::Y_IN);
    }

    // THE CASE: a parent with a run body that must be concurrent with its
    // children's. It sends x, then waits for the answer to come back around.
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("driving the pipeline");
        for x in 1..=4u32 {
            self.x_out.put(x).await; // into the pipeline
            let y = self.y_in.get().await; // ...and back out
            let expected = 2 * x * x;
            if y == expected {
                ctx.info(&format!("PASSED: x={x}, y={y}"));
            } else {
                return Err(TestError::from(format!(
                    "FAILED: x={x}, y={y}, expected {expected}"
                )));
            }
        }
        Ok(())
    }
}

// Expected: for x = 1..4, y = 2, 8, 18, 32 — each PASSED, REGRESSION: PASS.
// The workers' `loop`s never return; the phase ends when the test's objection
// drops and the unfinished runs are dropped. Under a phaser that runs children
// to completion first, this test hangs instead.

// ===========================================================================
// Beyond the book — the checks declared ports make possible
// ===========================================================================

// Chapter 31, Figure 14: A port left unconnected is an elaboration error (D22).
//
// The book has no figure for this: pyuvm discovers a missing connection lazily,
// at first use, as a Python attribute error. rustdv walks the tree at the end
// of elaboration and reports every declared-but-unconnected port at once,
// naming its path — before any run phase starts. Here the producer's put port
// is never connected, so the test fails elaboration with `tlm_unconnected_port`.
#[rustdv::test(expect_error = "tlm_unconnected_port")]
#[derive(Component, Default)]
struct UnconnectedTest {
    #[component]
    producer: RustdvComp,
    #[component]
    fifo: TlmFifo<u32>,
}

impl Component for UnconnectedTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = Producer::new_comp();
        self.fifo = TlmFifo::new(1);
    }
    // No `connect` — `Producer::put_port` is declared but never wired, so
    // elaboration fails and names `UnconnectedTest.producer.put_port`.
}

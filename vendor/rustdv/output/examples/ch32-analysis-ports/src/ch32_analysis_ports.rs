//! Chapter 32: Analysis ports — one write, every subscriber hears it.
//!
//!     sim-common/run_sim.sh ch32_analysis_ports playground
//!
//! Built and green on Icarus (2026-07-28). This chapter replaced the
//! pre-restoration direct-broadcast design with the hub-and-declared-port
//! model used across the TLM chapters (D83b). That surface is now deleted.
//!
//! ## The model (D17, D23, D24)
//!
//! Analysis is the *other* TLM shape: one-to-many broadcast, non-blocking, no
//! return value, no back-pressure. A source calls `ap.write(&item)` and every
//! connected subscriber's `write` runs; the source neither blocks nor learns
//! how many are listening (zero is legal). This is a different mechanism from
//! put/get (Chapter 31), not a mode of it — in pyuvm it is a separate class
//! whose `connect` appends to a subscriber list and whose `write` loops it.
//!
//! A component declares a `SubscribePort<T>` with `#[port(subscribe)]`, and
//! supplies a **subscriber**: a struct of its own that implements
//! `Subscriber::write(&mut self, item)` — the port of `uvm_subscriber`'s
//! `write()`, expressed as a trait rather than a base class. The UVM makes the
//! subscriber a component; here it is the plain struct the component hosts.
//!
//! ## Why the subscriber is separate from the component (D87)
//!
//! `write()` must deliver **synchronously, in zero time**: the publisher calls
//! it, every subscriber's handler runs, and control returns without the
//! simulation time wheel advancing. A design that queued the item and delivered
//! it later would let time pass in between — wrong, and the giveaway is that
//! you have to ask *when* delivery happens.
//!
//! But delivery needs `&mut subscriber` while the publisher's `run` holds
//! `&mut publisher`, and siblings cannot reach each other. The resolution: the
//! port holds a handle to the subscriber's **state**, not to the component. The
//! state lives in a `RustdvShared<T>` — the component keeps one handle, the
//! port gets another, and both see the same data. `write()` is then a plain
//! loop with no `await` anywhere.
//!
//! ## One connection pattern for both TLM shapes (Ray, 2026-07-24)
//!
//! Put/get is wired by the concrete FIFO between the two components. Analysis
//! has no such intermediary in the UVM — a source's analysis port broadcasts
//! straight to subscribers — and since both components are erased
//! `RustdvComp`s, neither side can drive the call.
//!
//! **rustdv gives analysis a hub too.** An `AnalysisBus` is a concrete
//! `#[component]` child with two named export accessors:
//!
//! ```ignore
//! self.bus.pub_export().connect(&self.mon, Monitor::PUB_PORT);
//! self.bus.sub_export().connect(&self.sb,  Scoreboard::SUB_PORT);
//! ```
//!
//! `pub_export()` takes the publisher's analysis port; `sub_export()` takes a
//! subscriber's. Several subscribers connect to the same `sub_export()` — that
//! is what makes it broadcast. The pattern is now identical to Chapter 31's:
//! a concrete FIFO, a named export, `connect(component, PORT_NAME)`.
//!
//! This is a deliberate divergence: the UVM has no analysis FIFO in the path
//! (its `uvm_tlm_analysis_fifo` buffers a stream, it does not broker the
//! broadcast). We are not implementing IEEE 1800.2, and one connection idiom
//! for the reader to learn beats two.
//!
//! ## The hub keeps nothing (Ray, 2026-07-28)
//!
//! Despite the name, an `AnalysisBus` is **not a FIFO and holds no items**.
//! `write` calls every subscribed object and returns; if nobody is subscribed
//! the datum is lost. That is the mechanism, not a limitation of it — a
//! broadcast that stored what nobody wanted would grow forever, and a monitor
//! would be paying for listeners it does not have.
//!
//! A component that needs to keep the traffic keeps it itself, in whatever
//! shape suits: a running tally (Figure 1), a `Vec` (Figure 2), a `TlmFifo` of
//! its own (Figure 6), or a comparison against a prediction (Chapter 34's
//! scoreboard).
//!
//! Note what rustdv does **not** need here. The UVM's scoreboards reach for
//! `uvm_tlm_analysis_fifo` because a class gets one `write` method: a second
//! stream needs the `uvm_analysis_imp_decl` macros to mint a differently-named
//! one, and routing each stream into its own FIFO is the way around that. A
//! rustdv subscriber declares two `SubscribePort`s and two `Subscriber` impls
//! and is done (D20/D88), so the workaround has nothing to work around.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// Subscribers
// ===========================================================================

// Chapter 32, Figure 1: A subscriber counts what it sees.
//
// The state that `write` touches lives in its own struct, and that struct
// implements `Subscriber`. The method is `write(&mut self, item)` — ordinary
// Rust, and the same word the UVM engineer already knows.
#[derive(Default)]
struct ItemCount {
    count: u32,
}

impl Subscriber<u32> for ItemCount {
    fn write(&mut self, _item: &u32) {
        self.count += 1;
    }
}

#[derive(Component, Default)]
struct Counter {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    tally: RustdvShared<ItemCount>,
}

impl Component for Counter {
    // The component hands the port a handle to its state. `clone()` does not
    // copy the ItemCount — it makes a second handle to the same one, so the
    // port's writes and the component's reads land on the same data.
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_subscriber = self.tally.clone();
        self.input.subscribe(my_subscriber);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let tally = self.tally.get();
        ctx.info(&format!("counted {} items", tally.count));
    }
}

// Chapter 32, Figure 2: A second subscriber on the same stream — proof that one
// write reaches every listener. This one keeps the values rather than counting.
#[derive(Default)]
struct SeenList {
    items: Vec<u32>,
}

impl Subscriber<u32> for SeenList {
    fn write(&mut self, item: &u32) {
        self.items.push(*item);
    }
}

#[derive(Component, Default)]
struct Collector {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    seen: RustdvShared<SeenList>,
}

impl Component for Collector {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_subscriber = self.seen.clone();
        self.input.subscribe(my_subscriber);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = self.seen.get();
        ctx.info(&format!("collected {:?}", seen.items));
    }
}

// ===========================================================================
// The source and the broadcast
// ===========================================================================

// Chapter 32, Figure 3: A source holds an analysis port and writes to it.
//
// `ap.write(&n)` returns immediately no matter how many subscribers listen —
// including none.
#[derive(Component, Default)]
struct NumberGen {
    #[port(publish)]
    ap: PublishPort<u32>,
}

impl Component for NumberGen {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("generating");
        for n in 0..3 {
            self.ap.write(&n); // broadcast; non-blocking
            ctx.info(&format!("wrote {n}"));
        }
        Ok(())
    }
}

// Chapter 32, Figure 4: One publisher, two subscribers, one hub.
//
// The `AnalysisBus` brokers the broadcast: the publisher's port connects to
// `pub_export()`, and every subscriber connects to the same `sub_export()`.
// Connecting two subscribers to one `sub_export()` is what makes the write
// fan out — and the wiring reads exactly like Chapter 31's put/get.
#[rustdv::test]
#[derive(Component, Default)]
struct BroadcastTest {
    #[component]
    source: RustdvComp,
    #[component]
    counter: RustdvComp,
    #[component]
    collector: RustdvComp,
    #[component]
    bus: AnalysisBus<u32>,
}

impl Component for BroadcastTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.counter = Counter::new_comp();
        self.collector = Collector::new_comp();
        self.bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.bus.pub_export().connect(&self.source, NumberGen::AP);
        self.bus.sub_export().connect(&self.counter, Counter::INPUT);
        self.bus.sub_export().connect(&self.collector, Collector::INPUT);
    }
}

// ===========================================================================
// Beyond the book
// ===========================================================================

// Chapter 32, Figure 6: A hub with no subscribers is legal (D22).
//
// Unlike a put/get port, an analysis `sub_export()` has min cardinality 0:
// broadcasting to nobody is a valid state, so this elaborates and runs clean.
// The publisher writes into the void. (`pub_export()` is still connected —
// a hub with no publisher would be a wiring mistake.)
#[rustdv::test]
#[derive(Component, Default)]
struct NoSubscribersTest {
    #[component]
    source: RustdvComp,
    #[component]
    bus: AnalysisBus<u32>,
}

impl Component for NoSubscribersTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.bus.pub_export().connect(&self.source, NumberGen::AP);
        // no sub_export() connection — legal for analysis
    }
}

// Chapter 32, Figure 8: When the subscriber needs *time*.
//
// Every subscriber so far did its whole job inside `write` — bump a counter,
// push onto a `Vec`. Those finish in zero time, which is what `write` requires:
// it is called from the publisher's `run` and the simulation clock must not
// advance inside it. `write` is not `async` and cannot `await`.
//
// So what does a subscriber do when the work *takes* time — consulting a slow
// reference model, driving a bus, waiting on the DUT? It splits the job in two.
// `write` does the one thing it can do instantly: put the item somewhere. The
// component's `run` — which may await all it likes — takes it from there.
//
// **That "somewhere" is a `TlmFifo` the subscriber owns.** This is the pattern
// to remember: the hub holds nothing, so a subscriber that needs to buffer
// declares its own queue. Note the shape of it — a `TlmFifo` used *inside* one
// component, connected to no port at all, as an ordinary handoff between a
// synchronous method and an asynchronous one.
//
// It is worth being clear about what this is *not*. A UVM scoreboard holds a
// `uvm_tlm_analysis_fifo` for a different reason: a class gets one `write`
// method, so a second analysis stream needs the `uvm_analysis_imp_decl` macros,
// and a FIFO per stream is the way around that. rustdv declares two
// `SubscribePort`s and two `Subscriber`s (Chapter 34), so that reason is gone.
// The reason here is time, and only time.
#[derive(Default)]
struct Inbox {
    queue: TlmFifo<u32>,
}

impl Subscriber<u32> for Inbox {
    // Zero time, and it cannot block: an unbounded FIFO always has room. A
    // *bounded* inbox would be a bug — `write` has no way to wait for space,
    // and analysis has no back-pressure to push back with, so a full one could
    // only drop the item.
    fn write(&mut self, item: &u32) {
        let _ = self.queue.try_put(*item);
    }
}

#[derive(Component, Default)]
struct SlowChecker {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    inbox: RustdvShared<Inbox>,
}

impl Component for SlowChecker {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.inbox = RustdvShared::new(Inbox { queue: TlmFifo::unbounded() });
        let my_inbox = self.inbox.clone();
        self.input.subscribe(my_inbox);
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("checking");
        // A handle to the same queue, taken once before the loop. Holding the
        // shared borrow (`self.inbox.get()`) across an await would keep the
        // state locked while `write` needs it.
        let my_queue = self.inbox.get().queue.handle();
        for _ in 0..3 {
            let n = my_queue.get().await;
            Timer::ns(5).await; // the slow work `write` could not have done
            ctx.info(&format!("checked {n}"));
        }
        Ok(())
    }
}

// Chapter 32, Figure 9: The publisher does not wait for the slow subscriber.
//
// The transcript is the lesson. All three writes land at 0ns — the source is
// never held up by what a subscriber does with an item — while the checker's
// results come out at 5, 10 and 15ns as it works through its own queue.
#[rustdv::test]
#[derive(Component, Default)]
struct SlowSubscriberTest {
    #[component]
    source: RustdvComp,
    #[component]
    checker: RustdvComp,
    #[component]
    bus: AnalysisBus<u32>,
}

impl Component for SlowSubscriberTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.source = NumberGen::new_comp();
        self.checker = SlowChecker::new_comp();
        self.bus = AnalysisBus::new();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.bus.pub_export().connect(&self.source, NumberGen::AP);
        // The same `sub_export()` as any other subscriber. Nothing in the
        // wiring says this one buffers — that is the checker's own business.
        self.bus.sub_export().connect(&self.checker, SlowChecker::INPUT);
    }
}

// ===========================================================================
// The FIFO's built-in taps (D23, D117 — moved here from Chapter 31)
// ===========================================================================

// Chapter 31's producer and consumer, unchanged, to give the FIFO below a real
// data path. They are not new material and the chapter does not reprint them:
// the producer blocks on a full FIFO, the consumer peeks and then gets.
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
            let seen = self.peek_port.peek().await; // blocks while empty
            let got = self.get_port.get().await; // consumes the peeked item
            assert_eq!(seen, got, "peek must not consume the item");
            ctx.info(&format!("got {got}"));
        }
        Ok(())
    }
}

// Chapter 32, Figure 11: A watcher on a FIFO's tap is an ordinary subscriber.
//
// Every `TlmFifo` carries a pair of publisher ports of its own: `put_ap()`
// announces each item the FIFO accepts, `get_ap()` each item it releases. They
// are the port of `uvm_tlm_fifo`'s built-in analysis ports, and there is
// nothing new to learn to use them — this watcher is the same shape as the
// `Counter` in Figure 1: a plain struct implementing `Subscriber`, a
// `SubscribePort`, and `subscribe` in the build phase.
#[derive(Default)]
struct TapLog {
    items: Vec<u32>,
}

impl Subscriber<u32> for TapLog {
    fn write(&mut self, item: &u32) {
        self.items.push(*item);
    }
}

#[derive(Component, Default)]
struct TapWatcher {
    #[port(subscribe)]
    input: SubscribePort<u32>,
    seen: RustdvShared<TapLog>,
}

impl Component for TapWatcher {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        let my_subscriber = self.seen.clone();
        self.input.subscribe(my_subscriber);
    }

    fn report(&mut self, ctx: &mut RustdvCtx) {
        let seen = self.seen.get();
        ctx.info(&format!("tap saw {:?}", seen.items));
    }
}

// Chapter 32, Figure 12: A tap is wired like any other subscription.
//
// Note what is and is not mixed here. The FIFO's **data path** is still a queue
// — one consumer takes each item, and the producer blocks when it is full. The
// taps are **observation** running alongside: every subscriber sees every item,
// nothing is consumed, and nobody is delayed. Two different jobs in one
// component, exactly as the UVM has it.
//
// A `put_ap()` is a `PublishExport`, so it takes a subscriber's port directly.
// There is no `AnalysisBus` here: the FIFO already is the hub for its own taps.
#[rustdv::test]
#[derive(Component, Default)]
struct FifoTapTest {
    #[component]
    producer: RustdvComp,
    #[component]
    consumer: RustdvComp,
    #[component]
    watcher: RustdvComp,
    #[component]
    fifo: TlmFifo<u32>,
}

impl Component for FifoTapTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.producer = Producer::new_comp();
        self.consumer = Consumer::new_comp();
        self.watcher = TapWatcher::new_comp();
        self.fifo = TlmFifo::new(1);
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        // the data path: producer -> queue -> consumer
        self.fifo.put_export().connect(&self.producer, Producer::PUT_PORT);
        self.fifo.peek_export().connect(&self.consumer, Consumer::PEEK_PORT);
        self.fifo.get_export().connect(&self.consumer, Consumer::GET_PORT);
        // the observation tap: the watcher sees every item put, and takes none
        self.fifo.put_ap().connect(&self.watcher, TapWatcher::INPUT);
    }
}

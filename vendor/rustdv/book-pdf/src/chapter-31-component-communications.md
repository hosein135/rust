# Chapter 31: Component Communications

Testbench 5.0 ended with an environment that builds its components through the factory — and with those components still doing their talking through the BFM's queues, as they have since version 1.0. That worked because our components sat directly on the DUT's traffic. It stops working the moment one *testbench* component needs to hand data to another: the tester to a driver, a monitor to a scoreboard. Wire them directly and each must know the other's type, which un-does everything the factory just bought us. The UVM's answer is transaction-level connection — TLM — and this chapter ports its point-to-point half: ports, exports, and the FIFO between them.

> **In the UVM...** we connected components with TLM-1 machinery: `uvm_blocking_put_port` and its export on the far side, `uvm_get_port`, the nonblocking and peek variants, each wired with `connect()` calls in `connect_phase` — and a runtime error (pyuvm's `UVMTLMConnectionError`, SV's elaboration-time connection fatal) when the wiring was wrong. A `uvm_tlm_fifo` sat between a producer's put port and a consumer's get port.

The model, in one paragraph, because every listing in this chapter is an instance of it. A **port** is the thing a component *calls*: `put`, `get`, `peek`, each in a blocking form that waits and a `try_` form that does not. The port forwards to an **export** on the far side. The export belongs to a **FIFO**, which owns the queue and implements every capability against it. And here is the part that makes it architecture rather than plumbing: the producer connects to the FIFO, the consumer connects to the FIFO, and *neither ever learns the other exists*. The FIFO is the point of decoupling. Swap the consumer through a factory override and the producer neither knows nor cares — which is precisely why this chapter had to come after the factory.

## Blocking put, get, and peek

```rust
// Chapter 31, Figure 1: A producer holds a put port and blocks on a full FIFO
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
```

Two new pieces of vocabulary:

- `#[port(put)]` — the attribute declares this field to the derive as a port, which does two jobs at once: it makes the port reachable *by name* when the environment wires it (you will see how in figure 3), and it enrolls the port in the end-of-elaboration connection check (figure 14 shows what that buys).
- `self.put_port.put(n).await` — a blocking put, in the coroutine sense both source frameworks use: the producer's run phase is suspended, in simulated time, until the FIFO has room.

```rust
// Chapter 31, Figure 2: A consumer that peeks, then gets
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
```

The consumer declares *two* ports and wires both to the same FIFO: `peek` returns the next item without consuming it, blocking until one is there, and `get` then consumes that same item. The `assert_eq!` makes the "peek does not consume" contract executable rather than documentary.

```rust
// Chapter 31, Figure 3: The env builds the two components and a FIFO, then
// wires them in `connect`
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
```

This is `connect` doing the job Chapter 24 gave it, and each line deserves unpacking, because the shape is subtler than it looks.

- The children are `RustdvComp` — factory-built, type-erased, exactly as testbench 5.0 left them. Which means the parent *cannot* write `self.producer.put_port`: that field does not exist on a `RustdvComp`, and Rust has no `$cast`-to-concrete to recover it. Something else must make the port reachable.
- That something is the port name. `#[port(put)]` on `Producer` generated an associated constant, `Producer::PUT_PORT`, and the derive taught `ComponentNode` to answer for it — through the same trait-object surface everything else uses. The export initiates: `fifo.put_export().connect(&self.producer, Producer::PUT_PORT)` says *this FIFO's put side serves that component's port of this name*.
- The name is typed, not a string. `Producer::PUT_PORT` carries the port's interface in its type, so misspelling it does not compile, and aiming a `get` export at a `put` port does not compile either. What resolves at elaboration is the wiring; what the wiring *means* was settled earlier.
- The FIFO itself is a `#[component]` child — concrete, not factory-erased, so its exports are reachable to call `connect` on. That is a deliberate carve-out, and it is the model closest to the UVM, where `uvm_tlm_fifo` is a real component with a path: a FIFO is plumbing. You will never override one through the factory, so it never pays the erasure that makes overriding possible.

There is one more thing to notice: the *same* `connect` call works when a component wires its own port. Figure 12 will show `connect(self, MathTest::X_OUT)` — `self`, not a child. That is not luck; it rules out a whole family of designs. A registry keyed by hierarchical path, say, could reach a child but never the connecting component itself, because a component does not know its own path. A mechanism that works for a child but needs a second spelling for `self` has broken the UVM's uniformity — the property that `uvm_test` *is* a `uvm_component`, no cases, no exceptions. A trait method answers for whoever implements the trait, which is every component including the one doing the connecting. That uniformity is why connection is a trait method and not a lookup.

```text
# Figure 4: Alternating through a depth-1 FIFO

      0.00ns INFO     running PutGetPeekTest (1/4)  [ch31-component-communications/src/ch31_component_communications.rs:124]
      0.00ns INFO     [PutGetPeekTest.producer]: put 0
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 0
      0.00ns INFO     [PutGetPeekTest.consumer]: got 0
      0.00ns INFO     [PutGetPeekTest.producer]: put 1
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 1
      0.00ns INFO     [PutGetPeekTest.consumer]: got 1
      0.00ns INFO     [PutGetPeekTest.producer]: put 2
      0.00ns INFO     [PutGetPeekTest.consumer]: peeked 2
      0.00ns INFO     [PutGetPeekTest.consumer]: got 2
      0.00ns INFO     PutGetPeekTest PASSED
```

Read the interleaving. The FIFO holds one item, so the producer *cannot* run ahead: put, peek, get, put, peek, get. Two run phases are taking turns in simulated time — which means two run phases are running *at once*. This is the payoff Chapter 24 promised: a producer blocked on a full FIFO can only proceed because the consumer's run phase is live to drain it. A phaser that ran components to completion one at a time would deadlock on this listing — the simplest one in the chapter.

## Nonblocking put and get

The blocking forms wait; the `try_` forms answer immediately and let the component decide what to do about "no." The examples switch from `u32` to a transaction with something to lose:

```rust
// A transaction, not an integer
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
```

`Packet` owns a `String`, so it is not `Copy` — like every real transaction you will ever put on a port. The figures below are written against it deliberately; a `u32` would let a retry loop compile that falls apart the moment you substitute your own command type.

```rust
// Chapter 31, Figure 5: A non-blocking producer never waits
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
```

Read the `Err` binding slowly, because this signature is the chapter's best lesson in how ownership shapes an API. The UVM's `try_put` returns a *bit*. It can, because SystemVerilog passes a class handle and the caller still holds its own; a refused put costs nothing. rustdv's `try_put` takes the packet **by value** — it must, since a successful put hands the packet to whoever gets it next — and so a bare "no" would have *eaten* a packet that was never delivered. The signature has to be `Result<(), T>`: `Err(back)` is the packet coming home, and `packet = back` is the retry loop taking it back for the next attempt.

Two things to be clear about. First, this is not Rust catching a bug SystemVerilog has. SystemVerilog has no bug here; it has a different ownership model, and each signature is the correct one for its model. Second, the tempting way to write the loop — `while self.put_port.try_put(packet).is_err()` — does not compile, because `packet` moved into the first attempt and is gone by the second. The compiler is not being difficult; it is asking the question the design already answered: *if the put failed, who has the packet?* The `Err` binding is the answer.

```rust
// Chapter 31, Figure 6: A non-blocking consumer
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
                    ctx.info(&format!("got {} (n={})", packet.label, packet.n));
                    seen += 1;
                }
                None => Timer::ns(1).await,
            }
        }
        Ok(())
    }
}
```

`try_get` returns `Option<Packet>` for the mirror-image reason: a get either hands you the whole packet — label and all, the consumer now owns it and the FIFO no longer does — or hands you nothing. There is no third state in which a packet exists but nobody owns it.

```rust
// Chapter 31, Figure 7: Same wiring, non-blocking components
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
```

The FIFO now carries `Packet` rather than `u32`, and not one connect line changed shape — the generic FIFO doing its job. (And the typed port name doing its job too: connect this `TlmFifo<Packet>` to a port that wants `u32` and the mistake fails to compile.)

```text
# Figure 8: Nonblocking components spend time instead of waiting

      0.00ns INFO     running NonBlockingTest (2/4)  [ch31-component-communications/src/ch31_component_communications.rs:247]
      0.00ns INFO     [NonBlockingTest.producer]: put 0
      0.00ns INFO     [NonBlockingTest.producer]: FIFO full, retrying
      0.00ns INFO     [NonBlockingTest.consumer]: got pkt0 (n=0)
      1.00ns INFO     [NonBlockingTest.producer]: put 1
      1.00ns INFO     [NonBlockingTest.producer]: FIFO full, retrying
      1.00ns INFO     [NonBlockingTest.consumer]: got pkt1 (n=1)
      2.00ns INFO     [NonBlockingTest.producer]: put 2
      2.00ns INFO     [NonBlockingTest.consumer]: got pkt2 (n=2)
      2.00ns INFO     NonBlockingTest PASSED
```

Same three packets, but now the clock moves: each retry burns a nanosecond of `Timer` instead of suspending on the FIFO. Blocking components let the FIFO schedule them; nonblocking components schedule themselves.

## The parent runs too: a three-stage pipeline

Everything so far had a parent that only built and connected. The next test is the chapter's centerpiece, and the reason is architectural: **the test itself is a stage**.

<figure>
<svg viewBox="0 0 640 220" xmlns="http://www.w3.org/2000/svg" font-family="sans-serif" font-size="13">
  <defs>
    <marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#888"/>
    </marker>
  </defs>
  <rect x="20" y="60" width="130" height="100" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="85" y="85" text-anchor="middle" fill="currentColor" font-weight="bold">MathTest</text>
  <text x="85" y="103" text-anchor="middle" fill="currentColor" font-style="italic">run: put x,</text>
  <text x="85" y="119" text-anchor="middle" fill="currentColor" font-style="italic">get y,</text>
  <text x="85" y="135" text-anchor="middle" fill="currentColor" font-style="italic">compare 2x²</text>
  <rect x="200" y="30" width="90" height="34" rx="4" fill="none" stroke="#888"/>
  <text x="245" y="52" text-anchor="middle" fill="currentColor">x_fifo</text>
  <rect x="350" y="30" width="110" height="34" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="405" y="52" text-anchor="middle" fill="currentColor" font-weight="bold">SquareIt</text>
  <rect x="510" y="93" width="90" height="34" rx="4" fill="none" stroke="#888"/>
  <text x="555" y="115" text-anchor="middle" fill="currentColor">sq_fifo</text>
  <rect x="350" y="156" width="110" height="34" rx="8" fill="none" stroke="#888" stroke-width="1.5"/>
  <text x="405" y="178" text-anchor="middle" fill="currentColor" font-weight="bold">TimesTwo</text>
  <rect x="200" y="156" width="90" height="34" rx="4" fill="none" stroke="#888"/>
  <text x="245" y="178" text-anchor="middle" fill="currentColor">y_fifo</text>
  <line x1="150" y1="75" x2="197" y2="50" stroke="#888" stroke-width="1.5" marker-end="url(#arr)"/>
  <text x="163" y="52" fill="currentColor" font-size="11">put</text>
  <line x1="290" y1="47" x2="347" y2="47" stroke="#888" stroke-width="1.5" marker-end="url(#arr)"/>
  <text x="308" y="40" fill="currentColor" font-size="11">get</text>
  <line x1="460" y1="47" x2="540" y2="90" stroke="#888" stroke-width="1.5" marker-end="url(#arr)"/>
  <text x="503" y="58" fill="currentColor" font-size="11">put</text>
  <line x1="540" y1="130" x2="460" y2="173" stroke="#888" stroke-width="1.5" marker-end="url(#arr)"/>
  <text x="503" y="166" fill="currentColor" font-size="11">get</text>
  <line x1="347" y1="173" x2="293" y2="173" stroke="#888" stroke-width="1.5" marker-end="url(#arr)"/>
  <text x="308" y="166" fill="currentColor" font-size="11">put</text>
  <line x1="197" y1="173" x2="150" y2="145" stroke="#888" stroke-width="1.5" marker-end="url(#arr)"/>
  <text x="158" y="170" fill="currentColor" font-size="11">get</text>
</svg>
<figcaption><em>Figure 9: A processing pipeline — y = 2x². The test is the first and last stage; the workers know only their FIFOs.</em></figcaption>
</figure>

The test chooses x, sends it into the pipeline, and waits for y to come back around, comparing against 2x². Two worker components do the arithmetic, each connected only to FIFOs; neither knows the other exists — or that the thing feeding them is the test itself.

```rust
// Chapter 31, Figure 10: The first stage squares its input
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
```

```rust
// Chapter 31, Figure 11: The second stage doubles what the first produced
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
```

Note the `loop` with no exit and no objection. The workers are *responders*: they serve forever, and they get to, because ending the run phase is not their job — it belongs to the component doing the work that matters. That is what makes the objection load-bearing rather than ceremonial: when the test's guard drops, the phase ends, and the workers' unfinished loops are dropped with it.

```rust
// Chapter 31, Figure 12: The test drives the pipeline and checks the answer
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
```

Three things this one listing does that nothing before it could.

First, **the parent holds its own ports** — `x_out` and `y_in` are fields of the test — and connects them with `connect(self, MathTest::X_OUT)`: the same call shape that wires a child, wiring the caller. The uniformity rule, made visible.

Second, **the round trip is closed**: put x, await y. Every prior example streamed data one way; this is the first request/response loop through the component tree, and it is the DUT-free rehearsal for testbench 7.0, where a test's `run` starts a sequence while a driver waits for items.

Third, **every run phase must make progress together**. The test blocks waiting for its answer; `SquareIt` blocks waiting for x; `TimesTwo` blocks waiting for x². Nothing completes unless everything runs at once — and the parent is one of the things that must run. A phaser that finished the children before starting the parent could not execute this test at all: the children would wait forever for an x the parent never got to send. This test is *why* rustdv's run phases are concurrent; the shape drove the design.

```text
# Figure 13: The pipeline checks itself

      2.00ns INFO     running MathTest (3/4)  [ch31-component-communications/src/ch31_component_communications.rs:345]
      2.00ns INFO     [MathTest.square_it]: 1² = 1
      2.00ns INFO     [MathTest.times_two]: 2 × 1 = 2
      2.00ns INFO     [MathTest]: PASSED: x=1, y=2
      2.00ns INFO     [MathTest.square_it]: 2² = 4
      2.00ns INFO     [MathTest.times_two]: 2 × 4 = 8
      2.00ns INFO     [MathTest]: PASSED: x=2, y=8
      2.00ns INFO     [MathTest.square_it]: 3² = 9
      2.00ns INFO     [MathTest.times_two]: 2 × 9 = 18
      2.00ns INFO     [MathTest]: PASSED: x=3, y=18
      2.00ns INFO     [MathTest.square_it]: 4² = 16
      2.00ns INFO     [MathTest.times_two]: 2 × 16 = 32
      2.00ns INFO     [MathTest]: PASSED: x=4, y=32
      2.00ns INFO     MathTest PASSED
```

A broken pipeline fails loudly rather than passing quietly — the comparison is in the test's own `run`, which is what "the parent is a stage" buys.

## What declared ports make checkable

Declaring ports with `#[port(...)]` gave the framework a complete inventory of what must be wired. Here is what it does with it.

```rust
// Chapter 31, Figure 14: A port left unconnected is an elaboration error
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
    // No `connect` — Producer::put_port is declared but never wired.
}
```

```text
# Figure 15: Every missing connection, named, before anything runs

these TLM ports were declared but never connected:
  UnconnectedTest.producer.put_port (put)
```

At the end of elaboration, rustdv walks the tree and reports **every** declared-but-unconnected port at once, by path, before any run phase starts. This is a timing difference worth being precise about. It is not a compile error — the wiring is decided by `connect` code, so elaboration is exactly as early as the check can run. But it is earlier than pyuvm, which discovers a missing connection lazily, at the first `put` that has nowhere to go — one at a time, mid-simulation. An elaboration sweep that names all of them before the DUT ticks is one of the places rustdv hands you something real, and it costs nothing at the call site. (The `expect_error` in the test's attribute is there only because this listing *wants* the failure, to prove it happens.)

One last thing to know about `TlmFifo`, held for later: like `uvm_tlm_fifo`, it also carries a pair of publisher ports, reached with `TlmFifo::put_ap()` and `TlmFifo::get_ap()`, which announce each item the FIFO accepts and releases. They belong to the analysis machinery, and Chapter 32 explains them.

## Summary

TLM-1 point-to-point, ported whole: a port is what a component calls, an export is what serves it, and the FIFO between them owns the queue and the decoupling — two components share a FIFO and never learn each other's names. Blocking `put`/`get`/`peek` suspend in simulated time; `try_put` gives a refused transaction *back* (`Result<(), T>`, because taking by value means a bare "no" would lose the packet) and `try_get` answers `Option<T>` for the mirror-image reason. Connection goes through typed port names generated by `#[port(...)]` — reachable through factory-erased children and through `self` alike — and the declared-port inventory buys an elaboration sweep that names every unwired port before anything runs. The pipeline stands as the chapter's thesis in working form: parents run concurrently with their children, responders serve forever, and the objection decides when everyone is done.

Ports, exports, and FIFOs move data from one component to *one* other component. A monitor has the opposite problem — one observation, delivered to a scoreboard, a coverage collector, and anyone else who cares, none of whom may slow it down. That is broadcasting, and it is Chapter 32.

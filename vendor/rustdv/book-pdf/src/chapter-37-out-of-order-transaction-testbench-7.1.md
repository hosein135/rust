# Chapter 37: Out-of-Order Transactions: Testbench 7.1

In testbench 7.0 a sequence sent commands and never heard back. Real stimulus often needs the answer — and the answer does not always arrive in the order the requests were sent. That second clause is the hard part, and this chapter teaches it on a device built to make it plain, before Chapter 38 needs it on the TinyALU.

The whole testbench is one idea. Each request carries a number: how many ticks the driver should spend on it before answering. The sequence sends four — 10, 7, 4, 1 — one after another, without waiting for any of them. The driver takes them all in and works on them at once, so the request sent last is finished first and the request sent first is finished last. Every answer still reaches the sequence that asked for it, because each one comes back under its request's own ticket.

That ticket is the `TxnId` the sequencer assigned when the item was sent, and this is the first chapter where it earns its keep. Until now, one item was in flight at a time, and "give me whatever comes next" always found the right answer without being told which one to look for. (If you have written an AXI testbench, you have met all of this: `ARID` and `RID` exist so a slave may answer out of order.)

And the detail that makes the whole system work: **the driver releases the sequence when it accepts the request, not when it has the answer.** `item_done` ends the handshake as soon as the request is taken in, so the sequence can send the next one; the answer comes back later, through `put_response`, under the request's ticket. Hold the handshake open until the work is done and only one request is ever outstanding — and then there is nothing for a ticket to disambiguate.

> **In the UVM...** the driver called `set_id_info(req)` on its response so the sequencer could route it, then `put_response(rsp)`; the sequence called `get_response(rsp, req.get_transaction_id())` to claim a specific answer. Forgetting `set_id_info` was a classic run-time failure: the response went back with no identity, and the sequence waiting for it waited forever.

One note on the cast: there is no TinyALU in this chapter. The DUT is the empty `playground` module, because the interesting behavior is *in the driver* — it takes a request, spends the time that request asked for, and gets on with the next one. The TinyALU runs one operation at a time, so none of its answers can ever overtake another; a chapter about overtaking needs a device that permits it.

## The transactions

```rust
// Chapter 37, Figure 1: A request that says how long it takes, and its answer
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Req {
    /// How many ticks the driver should spend before answering.
    delay: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Rsp {
    /// Echoed back so the sequence can see it got the answer it asked for.
    delay: u32,
}
```

Plain structs with derives, as Chapter 35 left them. The request carries what the driver needs in order to do the work; the answer carries what the sequence wants to know. Neither mentions a ticket — identity lives in the envelope the sequencer wraps around them, not in your data.

Putting the latency *in the request* is what makes this chapter's transcript readable. The driver does not choose how long to take, so nothing here depends on a random seed: the same four numbers go in every run, and the same reversal comes out.

## The driver

```rust
// Chapter 37, Figure 2: A driver that accepts work and answers it later
#[derive(Component, Default)]
struct Driver {
    #[port(seq_item)]
    seq_item_port: SeqItemPort<Req, Rsp>,
    /// Accepted but not yet answered.
    outstanding: Vec<InFlight>,
}

/// One request the driver has taken in and owes an answer for.
#[derive(Clone, Copy)]
struct InFlight {
    ticket: TxnId,
    delay: u32,
    left: u32,
}

impl Component for Driver {
    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        loop {
            // --- accept whatever is waiting ------------------------------
            Timer::ns(5).await;
            if let Some(item) = self.seq_item_port.try_next_item() {
                let ticket = item.txn_id();
                let delay = item.payload().delay;
                ctx.info(&format!("accepted {ticket}, delay {delay}"));
                // Release the sequence now. The answer comes later.
                self.seq_item_port.item_done(None);
                self.outstanding.push(InFlight { ticket, delay, left: delay });
            }

            // --- age the outstanding work, and answer what is due --------
            Timer::ns(5).await;
            let mut still_working = Vec::new();
            for job in self.outstanding.drain(..) {
                if job.left > 1 {
                    still_working.push(InFlight { left: job.left - 1, ..job });
                } else {
                    ctx.info(&format!("answering {}", job.ticket));
                    // The answer goes back under the request's own ticket.
                    self.seq_item_port.put_response(job.ticket, Rsp { delay: job.delay });
                }
            }
            self.outstanding = still_working;
        }
    }
}
```

The loop does one thing in each half of a tick, and that split is what keeps it simple: first half, accept a request if one is waiting; second half, age everything outstanding and answer whatever is due. A driver that tried to wait for a request *and* count time simultaneously would need two loops and a shared list; one sequential loop does both, and nothing races. (A driver on a real bus would use its clock's two edges; the playground is empty, so the tick is simulated time.)

Three lines to dwell on:

- **`try_next_item()` is what keeps the loop moving.** `get_next_item()` blocks. On a tick with an empty sequencer, a blocking accept would sit there forever and never reach the second half of the loop — the requests already accepted would never age, and the sequence waiting for them would never be answered. The non-blocking accept is the UVM's own answer to exactly this situation (`try_next_item` has been in the standard since 1.1d; pyuvm does not carry it, and rustdv follows the UVM here because this driver cannot be written without it).
- **`item_done(None)` is what puts four requests in flight at once.** It ends the handshake with no answer attached, which is the point: the answer does not exist yet. A driver that can answer immediately passes `Some(rsp)` here and never needs a ticket.
- **`put_response(job.ticket, ...)` returns the answer under the request's own ticket**, so the sequence that asked about `#1` gets `#1`, however many others were finished first. The ticket is a required argument, not a separate call: the UVM driver that forgot `set_id_info` sent back a response with no identity, and here that mistake has no spelling.

`InFlight` is the driver's whole memory: a ticket, the delay the request asked for, and how many ticks are left on it. Note what it does *not* keep — the `SeqItem` envelope itself. Once the ticket is out of the envelope, the envelope has done its job.

## The sequence

```rust
// Chapter 37, Figure 3: Send four requests, then collect four answers
#[derive(Default)]
struct Seq;

impl Sequence for Seq {
    type Req = Req;
    type Rsp = Rsp;

    async fn body(&mut self, ctx: &mut SeqCtx<Req, Rsp>) -> Result<(), SeqError> {
        let mut tickets = Vec::new();
        for delay in [10, 7, 4, 1] {
            let mut req = Req { delay };
            ctx.start_item(&mut req).await?;
            let ticket = ctx.finish_item(req).await?;
            ctx.info(&format!("sent {ticket}, delay {delay}"));
            tickets.push(ticket);
        }

        // Go round the outstanding tickets again and again, taking whichever
        // answers are ready and dropping those tickets from the list, until
        // none are left. Same drain-and-rebuild shape as the driver's loop.
        let mut waiting = tickets;
        while !waiting.is_empty() {
            Timer::ns(10).await;
            let mut still_waiting = Vec::new();
            for ticket in waiting {
                match ctx.try_get_response(Some(ticket)) {
                    Some(rsp) => ctx.info(&format!("got {ticket}, delay {}", rsp.delay)),
                    None => still_waiting.push(ticket),
                }
            }
            waiting = still_waiting;
        }
        Ok(())
    }
}
```

The sequence sends all four requests before asking about any of them — that is the whole trick. Had it waited for each answer before sending the next, only one request would ever be outstanding, and the driver's varying latency would be invisible. `finish_item` returns the ticket, and the sequence keeps them all.

Then it collects by **polling**, and the polling is not laziness — it is what makes the out-of-order work visible. Each time round, the sequence asks every outstanding ticket whether its answer is ready yet, keeps the ones that are not, and prints the ones that are. Blocking on ticket `#1`, then `#2`, then `#3` would collect the answers in the order they were *sent*, no matter what the driver did; the reordering would be real and the log would not show it. `try_get_response` collects them in the order they were *finished*, which is what the ticket is for.

The collection loop is deliberately the same shape as the driver's: walk the list, keep what is not done, rebuild it. Both sides of the handshake are managing a set of outstanding transactions, and they manage it the same way.

One failure mode deserves its paragraph, because no figure can show it: **asking for an answer that will never exist.** A sequence that calls `get_response` for a ticket the driver never answers waits forever. Nothing can tell "not ready yet" from "never coming"; that is inherent to asking for a specific answer, in every UVM. It is the sequence writer's job to ask only for answers that are owed, and the only way to demonstrate the mistake is a test that hangs, which is why this book does not run one.

## The environment and the test

```rust
// Chapter 37, Figure 4: A sequencer and a driver, and the test that runs them
#[derive(Component, Default)]
struct Env {
    #[component]
    seqr: Sequencer<Req, Rsp>,
    #[component]
    driver: RustdvComp,
}

impl Component for Env {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr = Sequencer::new();
        ConfigDb::set(None, "*", "SEQR", self.seqr.handle());
        self.driver = Driver::new_comp();
    }

    fn connect(&mut self, _ctx: &mut RustdvCtx) {
        self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);
    }
}

#[rustdv::test]
#[derive(Component, Default)]
struct ResponseTest {
    #[component]
    env: RustdvComp,
}

impl Component for ResponseTest {
    fn build(&mut self, _ctx: &mut RustdvCtx) {
        self.env = Env::new_comp();
    }

    async fn run(&mut self, ctx: &mut RustdvCtx) -> Result<(), TestError> {
        let _obj = ctx.raise_objection("requests outstanding");
        let seqr: Sequencer<Req, Rsp> = ConfigDb::get(Some(ctx), "", "SEQR")?;
        let mut seq = Seq::default();
        seq.start(&seqr).await?;
        Ok(())
    }
}
```

The environment is two components — a sequencer and a driver — because the chapter is about the handshake, not the architecture. Note what the test does *not* have: a flush. `Seq`'s body does not return until every answer is collected, so when `start` returns, nothing is in flight and the objection can drop immediately.

```text
# Figure 5: Later requests are answered first; every ticket still claims its own

      0.00ns INFO     rustdv: found 1 test(s), RUSTDV_RANDOM_SEED=1
      0.00ns INFO     running ResponseTest (1/1)  [ch37-out-of-order-transaction-testbench-7.1/src/ch37_out_of_order_transaction_testbench_7_1.rs:231]
     15.00ns INFO     [ResponseTest.env.driver]: accepted #1, delay 10
     15.00ns INFO     [Seq]: sent #1, delay 10
     35.00ns INFO     [ResponseTest.env.driver]: accepted #2, delay 7
     35.00ns INFO     [Seq]: sent #2, delay 7
     55.00ns INFO     [ResponseTest.env.driver]: accepted #3, delay 4
     55.00ns INFO     [Seq]: sent #3, delay 4
     75.00ns INFO     [ResponseTest.env.driver]: accepted #4, delay 1
     75.00ns INFO     [Seq]: sent #4, delay 1
     80.00ns INFO     [ResponseTest.env.driver]: answering #4
     85.00ns INFO     [Seq]: got #4, delay 1
     90.00ns INFO     [ResponseTest.env.driver]: answering #3
     95.00ns INFO     [Seq]: got #3, delay 4
    100.00ns INFO     [ResponseTest.env.driver]: answering #2
    105.00ns INFO     [Seq]: got #2, delay 7
    110.00ns INFO     [ResponseTest.env.driver]: answering #1
    115.00ns INFO     [Seq]: got #1, delay 10
    115.00ns INFO     ResponseTest PASSED
******************************************************************************
** TEST                                       STATUS  SIM TIME (ns)      **
******************************************************************************
** ResponseTest                                 PASS         115.00      **
******************************************************************************
REGRESSION: PASS
```

Read the log in three passes. The `sent` lines run `#1, #2, #3, #4`, delays descending. The `answering` lines run `#4, #3, #2, #1` — the exact reverse, because the driver is working on all four at once and the short ones finish first. The `got` lines follow that same reversed order, and each one carries back its own request's delay: `#4` with 1, `#1` with 10. The sequence never asked "what is next"; it asked four specific questions and got four specific answers.

Notice also that the driver accepts `#2` at 35 ns while `#1` still has seven ticks left on it. That is `try_next_item` and `item_done` doing their work: without them, `#1` would have to be finished and answered before `#2` could even be taken in, and there would be nothing left of this chapter.

## Summary

Responses are the second half of the sequencer handshake, and identity is what keeps them sorted when several are in flight. The driver accepts a request with `try_next_item` — the non-blocking accept that exists so a driver can keep serving work it already holds — releases the sequence with `item_done` before the answer exists, and returns each answer later with `put_response` under the request's own `TxnId`. The sequence holds its tickets and claims each answer with `get_response`/`try_get_response(Some(ticket))`, polling so that it collects them in the order they were finished rather than the order it sent them.

Chapter 38 puts the response path to work on the real DUT — where each command cannot even be written until the previous answer is in hand, and one transaction is in flight at a time.

//! Chapter 37: Out-of-order transactions — testbench 7.1.
//!
//!     sim-common/run_sim.sh ch37_out_of_order_transaction_testbench_7_1 playground
//!
//! ## What this chapter adds
//!
//! In 7.0 a sequence sent requests and never heard back. Here the driver
//! answers — and not in the order it was asked.
//!
//! Each request carries the number of ticks the driver should spend on it. The
//! sequence sends four, slowest first (10, 7, 4, 1), without waiting for any
//! answer. The driver therefore finishes them in exactly the reverse order,
//! and the sequence collects them in that order too — because it polls its
//! outstanding tickets rather than waiting on them one at a time.
//!
//! That is the whole chapter. The ticket — `TxnId` — is what makes a response
//! safe when more than one request is outstanding. Everywhere before this,
//! one item was in flight at a time and "give me whatever comes next" would
//! have found the right answer without being told which one to look for.
//!
//! ## What this asks the framework for
//!
//! Everything Chapter 36 asked for, plus:
//!
//! 1. **`item_done` releases the sequence before the work is finished.** That
//!    is what lets four requests be outstanding at once. Hold the handshake
//!    open until the answer is ready and only one is ever in flight — and then
//!    a ticket has nothing to disambiguate.
//! 2. **`SeqItemPort::try_next_item() -> Option<SeqItem<REQ>>`** — a
//!    non-blocking accept. The UVM has it (`tlm1/uvm_sqr_ifs.svh`, clause
//!    15.2.1.2.2, present since 1.1d) and pyuvm does not. Without it this
//!    driver deadlocks: on a tick with nothing queued, a blocking
//!    `get_next_item()` would wait there forever and never reach the second
//!    half of the loop, so the requests it already holds would never age.
//! 3. **`put_response(id, rsp)` / `get_response(Some(id))`** — the answer goes
//!    back under the request's own ticket. SystemVerilog needs
//!    `rsp.set_id_info(req)` and pyuvm needs `rsp.set_context(req)` to do this
//!    by hand, and forgetting either is a run-time fatal; the id is in the
//!    envelope here, so there is nothing to remember.
//!
//! ## The DUT
//!
//! There is no TinyALU in this chapter. The device under test is the
//! `playground` top, which is an empty module: the interesting behaviour is in
//! the driver, and a one-operation-at-a-time DUT could not produce an answer
//! that overtakes another. The tick is simulated time.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// ===========================================================================
// The transactions
// ===========================================================================

// Chapter 37, Figure 1: A request that says how long it takes, and its answer.
//
// Plain structs with derives, as Chapter 35 left them. Neither mentions a
// ticket — identity lives in the envelope the sequencer wraps around them
// (Chapter 35, Figure 7), not in your data.
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

// ===========================================================================
// The driver
// ===========================================================================

// Chapter 37, Figure 2: A driver that accepts work and answers it later.
//
// The loop does one thing in each half of a tick, and that split is what keeps
// it simple:
//
//   first half  — accept a request if one is waiting
//   second half — age everything outstanding; answer whatever is due
//
// A driver that tried to wait for a request *and* count time at the same time
// would need two loops and a shared list. One sequential loop does both, and
// nothing races.
//
// **`try_next_item` is why this compiles.** `get_next_item()` blocks. On a
// tick with an empty sequencer a blocking accept would sit there forever and
// never reach the second half, so the requests already accepted would never
// age and the sequence waiting on them would never be answered.
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

// ===========================================================================
// The sequence
// ===========================================================================

// Chapter 37, Figure 3: Send four requests, then collect four answers.
//
// The delays descend, so the request sent last is answered first. The sequence
// sends all four before asking about any of them — had it waited for each
// answer before sending the next, only one would ever be outstanding and the
// driver's varying latency would be invisible.
//
// It then collects by **polling**: each time round, it asks each outstanding
// ticket whether its answer is ready yet and drops the ones that are. That is
// what `try_get_response` is for. Blocking on ticket #1, then #2, then #3 would
// collect the answers in the order they were *sent* no matter what the driver
// did — the out-of-order work would be real and invisible. Polling collects
// them in the order they were *finished*, which is what the ticket is for.
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

// Chapter 37, prose passage, not code, no figure number: **asking for an
// answer that will never exist.** A sequence that calls `get_response` for a
// ticket the driver never answers waits forever. Nothing can tell "not ready
// yet" from "never coming"; that is inherent to asking for a specific answer,
// in every UVM. It is the sequence writer's job to ask only for answers that
// are owed, and the only way to demonstrate the mistake is a test that hangs,
// which is why this book does not run one.

// ===========================================================================
// The environment and the test
// ===========================================================================

// Chapter 37, Figure 4: A sequencer and a driver, and the test that runs them.
//
// The smallest environment since Chapter 25, because the chapter is about the
// handshake, not the architecture. Note what the test does *not* have: a
// flush. `Seq`'s body does not return until every answer is collected, so when
// `start` returns nothing is in flight and the objection can drop.
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

// Expected: four requests sent with descending delays, answered in ascending
// order of delay — the reverse of the order they were sent — and every ticket
// still collects its own answer. REGRESSION: PASS.

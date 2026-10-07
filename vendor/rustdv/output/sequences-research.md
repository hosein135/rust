# UVM sequences — study, and the target design for rustdv

*Research pass, 2026-07-28/29, for the ch35–ch39 / TB 7.0–8.0 work. Sources:
`pyuvm-master/pyuvm/_s14_15_python_sequences.py`, the SystemVerilog UVM
1800.2-2020 `src/seq/` and `src/tlm1/`, `uvmprimer-master/23_UVM_Sequences`,
`Python4RTLVerification-master/39`–`43`, and both books' chapters. Nothing was
built or changed; this is a reading, a design proposal, and a list of questions
for Ray. The companion aspirational example is
`output/sequences-aspirational.rs` (deliberately outside the examples
workspace, so nothing compiles it yet — D1/D2).*

---

## 0. What sequences are for

The Primer states the problem better than a summary can. In TB 6.0 the Tester
component does two jobs: it decides *which* transactions to send, and it
*sends* them. So a new stimulus pattern needs a new component, and combining
two patterns needs a third — "like swapping out your car's steering wheel
whenever you chose a different destination."

Sequences separate **stimulus** (the order and content of transactions) from
**structure** (the component topology). One testbench, many programs. Three
consequences the books actually exercise:

1. New tests without touching the testbench (7.0: `RandomSeq` and `MaxSeq` on
   an unchanged env).
2. Stimulus that reads the DUT's answers (7.1/7.2: Fibonacci through the ALU's
   adder).
3. A **programming interface** for test writers who are not testbench writers
   (8.0: `do_add(seqr, a, b) -> result`). This is also the mechanism behind
   D69's "problem B" — read sequence names from a file and run them.

That third one is the one rustdv currently cannot do at all, and it is the
reason D80's second registry exists as a deferred item.

---

## 1. The mechanism, exactly

### 1.1 The cast

| Piece | Is it a component? | Job |
|---|---|---|
| sequence item | no — data | carries one command (and, sometimes, its result) |
| sequence | **no** — a `uvm_object` | `body()` creates items and hands them over |
| sequencer | **yes** | holds the arbitration queue and the item export |
| driver | **yes** | pulls items through `seq_item_port`, drives the BFM |

The one structural fact worth stopping on: in SystemVerilog,
`uvm_sequence_base extends uvm_sequence_item` (`seq/uvm_sequence_base.svh:142`).
A sequence *is* an item. That is what lets a sequence be started as if it were
an item, and it is why `is_item()` exists at all — `uvm_sequence_item::is_item`
returns 1, `uvm_sequence_base::is_item` returns 0, and `start_item()` fatals if
you hand it a sequence (`uvm_sequence_base.svh:904`).

**pyuvm drops that inheritance entirely.** `uvm_sequence` extends `uvm_object`;
`uvm_sequence_base` and `uvm_sequencer_base` survive only as empty `pass`
stubs (`_s14_15_python_sequences.py:384–389`). Nothing in the mechanism needs
it. That is a clean D3 negative result: SystemVerilog *could* have kept them
separate and chose not to, but pyuvm demonstrates the choice was not
load-bearing. rustdv should follow pyuvm here — sequences are not items.

### 1.2 The handshake, event by event

pyuvm is the ground truth rustdv already follows, and it is small enough to
state completely. The data structures:

- `uvm_sequencer.seq_q` — unbounded; items awaiting arbitration.
- `uvm_sequencer.run_phase` — `forever { item = seq_q.get(); export.put_req(item) }`.
  A one-deep pump, and the whole of pyuvm's "arbitration".
- `uvm_seq_item_export.req_q` — unbounded; items the driver may take.
- `uvm_seq_item_export.rsp_q` — a `ResponseQueue`, which can cherry-pick by
  `transaction_id`.
- `uvm_seq_item_export.current_item` — the interlock that makes a second
  `get_next_item()` without `item_done()` an error.
- **Three events on each item**: `start_condition`, `item_ready`,
  `finish_condition` (`uvm_sequence_item.__init__`).

The trace, with who blocks where:

```
SEQUENCE                          SEQUENCER              DRIVER
--------                          ---------              ------
start_item(item)
  seq_q.put(item)  ------------->  pump: seq_q.get()
  await start_condition   [BLOCK]    export.put_req  -->  req_q
                                                          get_next_item()
                                                            current = req_q.get()
      [WAKE]  <---------------------------------------- set start_condition
                                                            await item_ready [BLOCK]
  ...set the fields NOW...
finish_item(item)
  set item_ready ---------------------------------------->  [WAKE] returns item
  await finish_condition  [BLOCK]                          drive the DUT
      [WAKE]  <---------------------------------------- item_done(rsp)
                                                            set finish_condition
                                                            rsp -> rsp_q
get_response(txn_id)  <-------------------------------- (cherry-pick by id)
```

**The gap between `start_item` and `finish_item` is the point of the whole
design.** It is the window *after* the driver has committed to taking this
item and *before* its contents are decided. The Python book names it
explicitly: "This code illustrates late stimulus setting, which means we don't
set the stimulus until the driver is ready to receive the item. Some
testbenches need the latest state of the system to set the stimulus properly,
and this feature supports that need."

A single `send(item).await` cannot express that, because the values would be
fixed before arbitration ran.

**D3 check.** SystemVerilog had `mailbox#(T)` and could have made this one
blocking put. It built a two-phase rendezvous instead. pyuvm — which was free
to simplify anything, and did simplify a great deal — kept the two phases and
kept the three events. Two for two on a mechanism whose static alternative was
sitting right there. **The gap is load-bearing and rustdv must keep it.**

### 1.3 What SystemVerilog has that pyuvm dropped

| SV feature | pyuvm | My read |
|---|---|---|
| Six arbitration modes (`UVM_SEQ_ARB_FIFO`/`WEIGHTED`/`RANDOM`/`STRICT_FIFO`/`STRICT_RANDOM`/`USER`) | FIFO only, by queue order | Real capability, dropped. FIFO is the default and covers the books. |
| Sequence and item **priority** (inherited from parent; root default 100) | none | Falls out with arbitration. |
| `grab`/`ungrab`/`lock`/`unlock` | none | Dropped, and **never asked for** — see below. |
| `is_relevant`/`wait_for_relevant` | none | Rarely used. |
| `pre_start`/`post_start`/`pre_do`/`mid_do`/`post_do` | none | Callback sprawl. |
| `pre_body`/`post_body` | **kept** | Cheap, and the only two anyone overrides. |
| Nine-state sequence state machine (`UVM_CREATED`…`UVM_FINISHED`) | none | Bookkeeping for `stop_sequences`, which pyuvm also drops. |
| `starting_phase` + automatic phase objection (a get-to-lock DAP) | none | Replaced by the test raising its own objection. |
| `uvm_sequence_library` | none | A whole 799-line file nobody in either book uses. |
| `uvm_do*` macros | none | Already in `src/deprecated/` in 1800.2. They exist because SV has no ergonomic construct-and-constrain expression, which Rust does not need either. |
| Response queue depth 8, with a drop error | unbounded | pyuvm removed a failure mode. |
| `try_next_item`/`has_do_available`/`wait_for_sequences` | none | `wait_for_sequences` is an NBA-settling hack specific to SV's scheduler. |
| `REQ`/`RSP` type parameters | dynamic typing | rustdv keeps the parameters — this is *data* typing, §0.4's good side of the seam. |

Two categories are mixed in that table, and D4 says to keep them apart:

- **Things SV needed because SV lacks something.** The `uvm_do` macros, and
  `wait_for_sequences`. Not losses.
- **Capability pyuvm dropped, with field evidence that it was not needed.**
  Arbitration modes, priority, and grab/lock. **Ray (2026-07-28): nobody has
  ever raised a pyuvm issue asking for any of them.** pyuvm has shipped
  FIFO-only arbitration for years, to a real user base, and the request has
  never come in.

  That is worth more than it looks. It is the same kind of evidence D3 runs on,
  read in the other direction: where a reimplementation dropped a mechanism and
  the users never came back for it, the mechanism was not load-bearing. D3
  found three places SystemVerilog kept runtime indirection it could have typed
  away; this is one place the indirection existed and turned out to be
  unnecessary. Both readings are the same method — look at what people
  actually did, not at what seems principled.

My recommendation: rustdv follows pyuvm (FIFO only). D4 still says name what
was dropped, so it belongs in STATUS.md's deviations — but recorded as
**dropped and not missed, with the evidence**, not as an unmet need. See Q23.

### 1.4 Virtual sequences

A virtual sequence is a sequence started **with no sequencer**. It calls no
`start_item`/`finish_item`; it starts *other* sequences.

- pyuvm enforces this dynamically: `start_item` raises `UVMSequenceError` if
  `self.sequencer is None` (`_s14_15_python_sequences.py:457`).
- SystemVerilog enforces nothing — `m_sequencer` is simply null.

Three ways a virtual sequence reaches a sequencer, in the order the books
present them:

1. **Hierarchy lookup** (Primer): `uvm_top.find("*.env_h.sequencer_h")` then
   `$cast`. Works, and is a string that lies silently after a rename.
2. **The ConfigDb** (Python book): the env does
   `ConfigDB().set(None, "*", "SEQR", self.seqr)` in `build_phase`, and anyone
   who needs it does `ConfigDB().get(None, "", "SEQR")`. This is the later and
   better one, and rustdv has a ConfigDb (D65–D68), so it ports directly.
3. **Inherited `m_sequencer`** (Primer's `parallel_sequence`): when the virtual
   sequence was itself started *with* a sequencer, its children use that one.

Note what this does to the "virtual" label: the Primer's `parallel_sequence` is
started *with* a sequencer and is still virtual in the sense that matters (it
sends no items of its own). The distinction is not crisp in the UVM, which
matters for a design decision below (§3.4).

**Parallelism.** SV uses `fork...join`; pyuvm uses `cocotb.start_soon` +
`Combine`. rustdv uses `join_all` — and D82 anticipated exactly this case,
including the requirement that joined sub-sequence futures **must not** be
`'static`, so a sub-sequence can borrow the parent sequence's state. That
requirement was written for the Fibonacci readback and it still holds.

### 1.5 The two ways a result comes back — and this is the hard one

**7.1, the shared handle.** The driver writes into the item the sequence still
holds:

```python
# driver
cmd = await self.seq_item_port.get_next_item()
await self.bfm.send_op(cmd.A, cmd.B, cmd.op)
result = await self.bfm.get_result()
cmd.result = result              # <-- into the caller's object
self.seq_item_port.item_done()

# sequence
await self.start_item(cmd)
cmd.A = prev_num; cmd.B = cur_num
await self.finish_item(cmd)
fib_list.append(cmd.result)      # <-- "a miracle happens"
```

The Primer says the same thing in SystemVerilog and is explicit about the
assumption: "We assume that the code that passed us the cmd still has a handle
to it and that by storing the data in the command we are returning it to the
caller."

**7.2, `get_response`.** The driver builds a *new* item, stamps it with
`set_id_info(cmd)` (which copies `sequence_id` and `transaction_id`,
`uvm_sequence_item.svh:159`), and passes it to `item_done(rsp)`. The sequence
awaits `get_response()`, which cherry-picks by transaction id.

**Ray's own verdict, in the Python book:** "Writing to the shared sequence item
handle is the cleaner of the two approaches and easier to use. In addition it
avoids a pitfall with `get_response()`" — a sequence that calls `get_response`
for an operation the driver does not answer (a RAM write) hangs.

**And 7.1 is precisely the pattern Rust will not hand you.** The whole of §3 is
about that.

---

## 2. What rustdv has today

`rustdv-methodology/src/sequence.rs` (311 lines) is already a faithful pyuvm
port, written pre-restoration. What is right, and should survive:

- **`SeqItem<REQ>` — an infrastructure envelope carrying `TxnId` + payload.**
  Identity lives in the framework, not smeared onto the user's data type. So a
  transaction stays a plain struct with derives, which is §0.4's seam exactly:
  types for data, runtime machinery for plumbing. This is better than both
  source languages and should be kept and taught.
- `ItemSlot` with `granted`/`ready`/`done` — pyuvm's three events, one per item.
- `ResponseQueue` with FIFO-or-by-id retrieval — pyuvm's `ResponseQueue`.
- `SeqItemPort::{get_next_item, item_done, get_response}` and
  `SeqCtx::{start_item, finish_item, get_response}` — the right five calls.
- The `get_next_item`-without-`item_done` interlock, panicking as pyuvm errors.
- `Sequence::body` returning a boxed future — the same `dyn` treatment
  `Component::run` needed (D48/D55). Correct, and unavoidable.

Six pre-restoration debts:

1. **The `Sequencer` is not a component.** It is a `Clone` handle passed to a
   driver's constructor. It must become a real child in the tree with a path
   and an export, connected in `connect` (D83b/D84) — because the sequencer
   having a path is what makes it findable, and because "the sequencer is a
   component" is a thing the book says.
2. **`start` is on the sequencer, not the sequence.** rustdv writes
   `seqr.start(&mut seq)`; both books write `seq.start(seqr)`. Worse, the
   inversion has no spelling for a virtual sequence — there is no sequencer to
   call it on. That is the D83b tell again: a mechanism that needs a second
   spelling for a legitimate case has broken uniformity.
3. **`start_item(&mut REQ)` ignores its argument**, and `finish_item(item: REQ)`
   takes the item **by value**. The item therefore leaves the sequence at
   `finish_item` and cannot be read back. So 7.1's shared handle is not
   expressible — and ch37 as it stands today silently uses `get_response`,
   collapsing 7.1 and 7.2 into the same testbench with different prose. The
   book's distinction is currently unimplementable. This is the biggest single
   finding of this pass.
4. **No factory registration for sequences** — D80's `rustdv_seqs` link
   section and `create_seq_by_name`. TB 7.0 needs it *today*, not just for
   file-driven stimulus: `RandomTest` and `MaxTest` differ only by
   `set_type_override_by_type(BaseSeq, RandomSeq)`.
5. **The driver takes its port out with `Option::take` in `start`** and spawns
   — the pre-restoration hook, superseded by D82's join model.
6. **A sequence has no context.** pyuvm sequences cannot log (they are not
   `uvm_report_object`s) and reach for `uvm_root().logger`; and they get
   randomness from the global `random` module. rustdv has a per-component
   `RustdvCtx` with a derived path and a seeded `Rng` — a sequence should get
   the same, through its `SeqCtx`. Cheap, and better than both books. See §4.6.

---

## 3. The hard problem: how a result gets back to the sequence

Four options. This is the decision I most need from Ray, because it sets the
shape of ch37 *and* ch38.

### Option A — response queue only (what the code does now)

`finish_item(item) -> TxnId`, then `get_response(Some(id))`.

- **Cost:** the book's 7.1 disappears, or the prose describes a mechanism the
  code does not have. And the pitfall Ray warned readers about — a driver that
  does not always respond hangs the sequence — becomes rustdv's *only*
  mechanism, with no alternative to recommend.
- **Benefit:** simplest ownership; nothing shared.

### Option B — `finish_item` hands the item back

```rust
ctx.start_item(&mut cmd).await;
cmd.a = prev; cmd.b = cur;
let cmd = ctx.finish_item(cmd).await?;   // comes back, .result filled in
cur_num = cmd.result;
```

The driver receives `&mut` the payload, writes `result` into it, and
`item_done()` returns the item up the same channel that carried it down.

- **Cost:** one rebinding line the Python has no counterpart for; and it is a
  *round trip*, not a shared handle, so a reader coming from the UVM must be
  told the difference in one sentence.
- **Benefit:** no `Rc<RefCell<>>`, no aliasing, and the lesson the book already
  teaches — "the driver writes the result into the item" — survives with a
  Rust-shaped explanation. It is **D89 one level up**: a call that takes
  ownership of a transaction hands it back, exactly as `try_put`'s `Err(back)`
  does. Teaching the same shape twice is worth something.
- It also **cannot hang**, because the item's return is the handshake itself.
  Ray's 7.2 pitfall does not exist in this form.

### Option C — an actual shared handle: `RustdvShared<AluCommand>`

The machinery already exists (D88). Sequence and driver hold two handles;
the driver does `item.get_mut().result = r`.

- **Cost:** every sequence item becomes an `Rc<RefCell<T>>`, taxing the common
  case (TB 7.0 needs no readback at all), and re-introducing the aliasing the
  framework has otherwise been careful about. The borrow-guard-across-await
  footgun that ch32's `SlowChecker` had to dodge would now sit in every
  sequence.
- **Benefit:** it is literally the book's picture, and the prose would not have
  to change at all.

### Option D — B for 7.1, the response queue for 7.2 (recommended)

The books have two mechanisms because there are two situations. Keep both, and
motivate them the way the code actually justifies them:

- **`finish_item` returns the item** when every item gets exactly one answer
  and the sequence wants it in hand (7.1, Fibonacci). Cannot hang.
- **The response queue** when responses are sparse, out of order, or arrive
  from somewhere other than the item's own handshake (7.2). This is where
  `TxnId` cherry-picking earns its keep — and rustdv's `TxnId` envelope makes
  the correlation typed and automatic where SV needed `set_id_info` by hand and
  pyuvm needed `set_context`.

That reframing also fixes ch38's motivation. Today `get_response` is presented
as "another way to do the same thing, but worse." Under D it is presented as
the mechanism for the case the direct return cannot serve — which is true, and
which is what `set_id_info` was always for.

**My recommendation is D.** But it changes both chapters and it contradicts the
Python book's flat "use the shared handle" advice, so it is Ray's call, not
mine. See Q20.

### 3.4 A related temptation I am deliberately not taking

It would be easy to make a virtual sequence a **different trait** —
`VirtualSequence` with a `body(&mut self)` that has no `SeqCtx` and therefore
no `start_item` to call. Calling `start_item` in a virtual sequence would
become a compile error instead of pyuvm's runtime `UVMSequenceError`.

I am not proposing it, and the reason is §0.1. The UVM's distinction is not
crisp: the Primer's `parallel_sequence` is started *with* a sequencer and is
still "virtual" in the sense of sending no items, and nothing stops a sequence
from sending items *and* delegating to sub-sequences. Two traits would forbid
that. That is "I can make this static, therefore I should," and it would cost a
capability to buy an error message. The aspirational example uses one trait and
a `SeqCtx` whose `start_item` fails at run time when there is no sequencer —
pyuvm's behaviour, for pyuvm's reason.

Recorded here rather than left implicit, because it is exactly the kind of cut
that gets made silently.

---

## 4. What rustdv must build

### 4.1 The sequencer becomes a component

A concrete child, like a FIFO (D84): reachable so the parent can call
`seq_item_export()` on it, never a factory-override target. In the aspirational
example it is declared `#[component]`, the same carve-out from D78 a `TlmFifo`
gets and for the same reason. The carve-out is a property of the field's type,
not of the attribute — D114 (§42) removed the attribute's argument, because the
derive never read it.

### 4.2 Connection through `port_slot` (D83b)

```rust
self.seqr.seq_item_export().connect(&self.driver, Driver::SEQ_ITEM_PORT);
```

Identical in shape to every other connection in ch31/ch32/ch34 — a concrete
child, a named export, `connect(owner, PORT_NAME)`. A new `#[port(seq_item)]`
kind, a `SeqItemPort<REQ, RSP>` implementing `PortField`, and a `PortName`
carrying the interface so a `put` export aimed at a seq-item port is a compile
error. Elaboration cardinality: a driver's seq-item port is **required**
(min 1), like put/get, unlike analysis (D85).

### 4.3 The sequencer handle reaches a test through the ConfigDb

pyuvm's `ConfigDB().set(None, "*", "SEQR", self.seqr)` ports directly onto
rustdv's `ConfigDb::set` (D65). The value stored must be a cheap `Clone`
handle to the sequencer's shared inside — not the component — which the
existing `Sequencer` already is. `ConfigDb::set` requires `Clone + Debug`, so
the handle needs a `Debug` impl (it has none today).

*This is the first time the ConfigDb carries a component-ish handle rather than
a config value,* which brushes against Q12 (should the BFM live there too). Not
a blocker, but worth Ray seeing the connection.

### 4.4 `start` moves to the sequence

```rust
seq.start(&seqr).await?;   // a real sequence
seq.start_virtual().await?; // no sequencer
```

Two entry points rather than pyuvm's one optional argument, because Rust has no
default arguments; both land in the same `start_with(Option<&Sequencer>)`. Open
naming question (Q22) — `start_virtual` is descriptive but ugly, and
`start(None)`/`start(Some(&seqr))` is uniform but noisy at every call site.

### 4.5 The sequence factory (D80)

Needed by TB 7.0 itself, not just by file-driven stimulus: `RandomTest` and
`MaxTest` differ only by a type override on the sequence. So D80 is not
deferrable past ch36 after all — that is a change from what §0.5 currently
assumes.

`#[derive(Sequence)]` emitting into an `rustdv_seqs` link section (11 bytes,
under the 16-byte Mach-O cap that bit D75), `Factory::create_seq_by_name`, and
type/instance overrides sharing the existing override table. Constraint carried
over from D75: concrete non-generic sequences with `Default`. `RandomSeq`,
`MaxSeq`, `FibonacciSeq`, `TestAllSeq` all qualify; `OpSeq(a, b, op)` is built
programmatically, as in both books.

### 4.6 A sequence gets a context

pyuvm sequences cannot log and reach for `uvm_root().logger`; they take
randomness from the global `random` module, which defeats seeded reproduction.
rustdv should hand a sequence a `SeqCtx` that carries `info()`/`warning()`/…
and a seeded `rng()`, both derived from the sequencer's path. This is a small
improvement over both source languages, it costs nothing, and it is
*necessary*: a factory-created sequence is built by a `Default` maker and
therefore cannot be handed a seed at construction.

Path question: what does a sequence log *as*? It is not in the tree. pyuvm logs
as the root. I would log as the sequencer's path plus the sequence's registered
name — `AluTest.env.seqr/RandomSeq` or similar. Q24.

### 4.7 Parallel sub-sequences use `join_all`, not `spawn`

Per D82. The existing quarantined ch39 uses `spawn_named` — pre-restoration —
which forces every sub-sequence future to be `'static` and taxes exactly the
Fibonacci-readback pattern D82 was written to protect.

### 4.8 Objections and the flush

In 7.0 the driver does not wait for results, so the test must hold its
objection past the last `finish_item` — the Python book waits 50 clocks, ch34
waits 20 for the same reason (the multiply is last and slowest). In 7.1/7.2 the
driver *does* wait for each result before `item_done`, so `finish_item`
returning is proof the DUT answered and no flush is needed. Worth a sentence in
the prose: the flush is a property of the driver, not of sequences.

### 4.9 Naming (D79)

Everything existing is already clean: `Sequence`, `SeqItem`, `Sequencer`,
`SeqItemPort`, `SeqCtx`, `TxnId`. New names needed: the export accessor
(`seq_item_export()`), the port attribute (`#[port(seq_item)]`), and — if D80
lands as designed — the boxed-sequence type the factory returns. `RustdvSeq`
would parallel `RustdvComp` and follow D88's rule that user-held types wear the
framework prefix. Q25.

---

## 5. Open questions for Ray

*Status 2026-07-29: Q20, Q21, Q22 and Q26 are **settled** — see §28 of the
decision log, D93–D96. All eight now live in §16 of the log, which is the
authoritative list; the entries below are kept as the reasoning that produced
them.*

- **Q20 — how does a result get back to a sequence?** Options A–D in §3. I
  recommend **D** (`finish_item` returns the item for 7.1; the response queue
  for 7.2, remotivated as the sparse/out-of-order case). This contradicts the
  Python book's "use the shared handle" advice, which is why it is yours.
- **Q21 — does the sequence item stay one object across a loop?** Both books
  reuse a single `cmd` for all seven Fibonacci iterations, mutating it in place.
  Under Option B the item moves and comes back, so each iteration naturally has
  its own binding. The transcript is identical; the prose is not.
- **Q22 — `seq.start(&seqr)` + `seq.start_virtual()`, or one
  `seq.start(Option<&Sequencer>)`?** §4.4.
- **Q23 — does the book mention grab/lock and the six arbitration modes at
  all?** pyuvm dropped them silently and, per Ray, no user has ever asked for
  them back. D4 still wants the omission recorded, so STATUS.md gets a
  deviation either way. The open part is the *book*: a paragraph telling a
  SystemVerilog reader that these are gone may serve them, or may draw
  attention to something a decade of pyuvm says nobody wants. Your call.
- **Q24 — what path does a sequence log under?** §4.6.
- **Q25 — what is the boxed sequence type called?** `RustdvSeq`, to parallel
  `RustdvComp`? §4.9.
- **Q26 — does D80 still count as "deferred"?** TB 7.0's `RandomTest`/`MaxTest`
  need a sequence *type override*, so the sequence factory is required by ch36,
  not by the file-driven stimulus of ch39. §0.5 and D80 both currently read as
  though it can wait.
- **Q27 — does the sequencer handle in the ConfigDb reopen Q12?** It is the
  first component-ish handle to live there. §4.3.

---

## 6. Where each claim came from

| Claim | Source |
|---|---|
| The full pyuvm sequence layer, 485 lines | `pyuvm-master/pyuvm/_s14_15_python_sequences.py` |
| Three events per item | same, `uvm_sequence_item.__init__` (l.164–166) |
| `uvm_sequence` extends `uvm_object`, not the `uvm_sequence_base` stub | same, l.384–392 |
| Sequencer pump `seq_q → req_q` | same, `uvm_sequencer.run_phase` (l.358–361) |
| `start_item` raises in a virtual sequence | same, l.457–460 |
| `uvm_sequence_base extends uvm_sequence_item` | `UVM/1800.2-2020/src/seq/uvm_sequence_base.svh:142` |
| `is_item()` 1 vs 0 | `uvm_sequence_item.svh:257`, `uvm_sequence_base.svh:203` |
| `start_item` → `wait_for_grant`; `finish_item` → `send_request` + `wait_for_item_done` | `uvm_sequence_base.svh:893–967` |
| Six arbitration modes | `base/uvm_object_globals.svh:355–372` |
| `set_id_info` copies txn + sequence id | `uvm_sequence_item.svh:159` |
| Response queue depth 8 and its overflow error | `uvm_sequence.svh:105–120` (doc comment) |
| `uvm_do` macros are deprecated | present in `src/deprecated/macros/`, absent from `src/macros/` |
| SV Fibonacci reads `command.result` after `finish_item` | `uvmprimer-master/23_UVM_Sequences/tb_classes/fibonacci_sequence.svh` |
| SV virtual sequence via `uvm_top.find` + `$cast` | same, `runall_sequence.svh` |
| SV parallel sub-sequences via `fork...join` on `m_sequencer` | same, `parallel_sequence.svh` |
| "late stimulus setting" | *Python for RTL Verification*, ch. "Sequence testbench: 7.0" |
| "the shared handle is the cleaner of the two approaches" | same, ch. "get_response() testbench: 7.2" |
| The RAM-write `get_response` hang | same |
| "swapping out your car's steering wheel" | *The UVM Primer*, ch. 23 |
| "We assume that the code that passed us the cmd still has a handle to it" | same |
| rustdv's current sequence layer | `rustdv/rustdv-methodology/src/sequence.rs` |
| ch37 already uses `get_response`, not a shared handle | `output/examples/ch37-fibonacci-testbench-7.1/src/lib.rs:78–79` |

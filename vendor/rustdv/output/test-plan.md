# rustdv test plan

> **Status 2026-07-29: built.** All three tiers below exist and are wired into
> the regression.
>
> | Tier | Where | Count |
> |---|---|---|
> | §2 no-simulator | `#[cfg(test)]` modules in `rustdv/` | 110, was 12 |
> | §3 targeted simulator | `rustdv/framework-tests/` | 38, plus `sim-mutation` |
> | §4 compile-fail | `rustdv/framework-tests/compile-fail/` | 5 |
>
> The regression is 236 entries (was 228), green — the eight new ones are the
> six targeted groups, `sim-mutation`, and `compile-fail-methodology`; the 38
> and the 5 are counted inside those. What each file actually
> covers is in the code, not here — this document is the plan and the
> reasoning, and the tests are the record. Deviations from the plan as written
> are listed at the bottom.

*Written 2026-07-29, modelled on pyuvm's suite
(`../rustdv-reference/pyuvm-master/tests`). pyuvm splits its testing in two —
`pytests/` runs under plain pytest with no simulator, `cocotb_tests/` runs
twelve directories under Icarus — and that split is the right one for rustdv
too, for the same reason: **most of a verification framework is not about
time.***

---

## 0. Where rustdv stands, measured

| | pyuvm | rustdv today |
|---|---|---|
| No-simulator tests | `pytests/`, 3,548 lines, 8 files | 12 `#[test]` functions → **110 (2026-07-29)** |
| Simulator tests | `cocotb_tests/`, 12 directories | 21 `sim-ch*` chapter runs + 2 smoke |
| What the no-sim tests cover | base classes, ConfigDb, callbacks, root-find, glob matching | `Rng`, `RustdvPath`, `LogicValue`, `AluCommand` |

The regression is green at 227 and that number is misleading. It is almost
entirely **end-to-end**: `sim-ch23` proves that chapter 23 runs, not that
`ConfigDb` precedence is right. Every one of rustdv's twelve unit tests is on a
leaf utility. **The methodology layer — ConfigDb, factory, ports, FIFO,
analysis bus, objections, sequences, the phase walk — has no unit test at
all.** A bug there surfaces as "some chapter went red", and the bisect is
manual.

That is the gap this plan closes, and it is the same gap pyuvm closed by
writing `pytests/` — which is why `test_uvm_config_db.py` is 530 lines on its
own.

---

## 1. The split, and why rustdv can make it

**A test needs the simulator only if it awaits simulated time or touches a
signal.** Everything else can run under `cargo test`.

That is not obvious in a simulator-driven framework, so here is the evidence:

- `Executor::run_until_idle()` is **public** and drains the run queue to
  exhaustion without any simulator involvement (`executor.rs:214`).
- `Event` and `Queue` contain **zero** `gpi::` calls — they are pure async
  primitives (`sync.rs`, `queue.rs`).
- `Timer` and the edge triggers *do* call `gpi::register_timer`
  (`triggers.rs:81`), so anything that awaits time needs a real simulator.
- `rustdv-vpi-stubs` already exists so unit-test binaries link, and panics if a
  VPI symbol is actually called — which makes "this test accidentally needed
  the simulator" a loud failure rather than a silent one.

So the line is sharp and the compiler enforces it. **Most of the methodology
layer falls on the no-simulator side**, including the whole sequencer
handshake, because the handshake is built from `Event`s and never waits for
time.

### The one thing to build first

`rustdv-sim` needs a test harness that runs a future to completion on a bare
executor:

```rust
// rustdv-sim/src/testing.rs — #[cfg(test)] or a `testing` feature
pub fn block_on<F: Future<Output = T>, T>(fut: F) -> T
```

It spawns the future, calls `run_until_idle()`, and asserts the future
completed — failing with "the future is still pending: did it await a Timer?"
if not. That message is the point: it tells a test author they have crossed the
line into needing a simulator.

**Everything in §2 depends on this.** It is perhaps forty lines and it should
be written before any of the tests below.

---

## 2. No-simulator tests (`cargo test`)

pyuvm's `pytests/` equivalent. Organised by module, with the count being a
target rather than a guess — pyuvm's own file sizes are the calibration.

### 2.1 `rustdv-sim` primitives

**`queue.rs`** — pyuvm tests its queue in `cocotb_tests/queue` (189 lines); ours
needs no simulator.

- `put`/`get` round-trip preserves order
- `try_put` on a full queue returns `Err(item)` **with the item** (D89)
- `try_get` on an empty queue returns `None`
- a blocked `put` wakes when space appears; a blocked `get` wakes on a put
- `peek` does not consume, and leaves the item for `get` (needs `T: Clone`)
- `has_space`/`wait_for_space` agree with `len`/`size`
- unbounded never blocks a put
- **drop safety**: a dropped `get` future does not consume an item

**`sync.rs` (`Event`)**

- `wait` before `set` wakes; `set` before `wait` does **not** (edge semantics)
- several waiters all wake on one `set`
- `Lock` is FIFO-fair: three waiters acquire in request order

**`combinators.rs`**

- `join2` yields both outputs; `join_all` preserves input order
- `first2` returns the winner and **drops** the loser (D82c's mechanism —
  assert the loser's `Drop` ran)
- a borrowing future composes: the whole point of D82, and a compile-level
  regression test

**`path.rs`** — already has 4 tests including the `ab`/`a` prefix case. Add:

- `child` extends without allocating a new string on every segment
- `Display` is the dotted rendering, byte-identical to the old format

**`rng.rs`** — has 2. Add: the same seed gives the same sequence across runs
(the reproducibility claim every transcript rests on).

### 2.2 `ConfigDb` — the deepest single file

pyuvm's `test_uvm_config_db.py` is 530 lines. This is where the most
behaviour-per-line lives, and rustdv has **none** of it tested.

- set/get round-trip at an exact path
- **precedence by setter depth (D13)**: a parent's `set` beats a child's for the
  same path — the tier-2 rule whose reasoning is a paragraph in `config.rs` and
  which nothing currently checks
- most-specific path wins over a wildcard
- most recent write wins at equal precedence
- glob matching: `*` at the end, in the middle, `env.t*`, and the `ab`-under-`a`
  case
- **`get` of a missing key returns `Err`, not a default** (D14 — the whole
  reason it returns `Result`)
- wrong-type `get` is an `Err` naming the type mismatch, not a panic
- `clear()` empties it (the per-test guarantee the runner relies on)
- storing a handle (`Rc<TinyAluBfm>`, `Sequencer`) works — D101's new case
- `dump()` renders every entry (D68's debugging surface)

### 2.3 Factory

pyuvm's `factory_tests.py` is 622 lines.

- `new_comp()` builds the named type; `create_comp()` builds it too when no
  override is installed
- `set_type_override::<A, B>()` swaps at a `create_comp` slot and **not** at a
  `new_comp` slot
- `set_inst_override` beats a type override at the same path (D13 precedence)
- an override installed after the walk has passed a slot does **not** apply —
  the ordering rule build-top-down depends on
- `create_by_name` builds a registered type; an unregistered name is a named
  error
- the discarded default's phases never ran (D75's guarantee — assert with a
  component that records `build`)
- **sequence factory**: `set_seq_override` pairs two sequences,
  `create_seq::<S>()` honours it, and `clear_seq_overrides()` resets per test

### 2.4 Ports, FIFO, and the analysis bus

pyuvm's `test_12_uvm_tlm_interfaces.py` is 649 lines and enumerates every port
kind. rustdv has five aliases over one `Port<I>`, so the matrix is smaller but
the same shape.

- each of `put`/`get`/`peek`/`publish`/`subscribe`/`seq_item` binds through
  `port_slot` and is reachable by its generated `PortName`
- `connect(self, ..)` and `connect(&child, ..)` both work — **D83b's
  uniformity, as an executable claim**
- a `get` export aimed at a `put` port does not compile (a `compile-fail` case)
- a misspelled `PortName` constant does not compile
- `bind` to a missing port name returns `NoSuchPort` naming the component
- `unconnected_ports` reports **every** miss in one sweep, with paths (D85)
- analysis min-cardinality 0: a hub with no subscribers elaborates clean
- `AnalysisBus::write` reaches every subscriber, in connection order, and
  **stores nothing** — a second `write` with no subscriber is not buffered (D90)
- `TlmFifo` taps: `put_ap`/`get_ap` see each item, consume none, delay nobody
  (D23)
- `RustdvShared` gives two handles to one value; `handle_count` proves it

### 2.5 The component walk

pyuvm splits this across `t09_phasing` and `t13_components`.

- build is top-down, connect bottom-up, extract/check/report top-down —
  assert the **order** with a component that records its own phase calls
- a child created in `build` is descended into (D6's two-stage construction)
- `take_children`/`restore_children` round-trip in declaration order, and
  restore happens even when a run returns `Err` (D82b)
- `children_mut` yields `Option<T>` children only once `Some`
- paths are derived from field names, and a renamed field renames the path
  (D7 — the property a hand-typed string cannot give you)
- `#[derive(Component)]` on a unit struct, a generic struct, and a struct with
  every child shape

### 2.6 Objections

- `raise_objection` returns a guard; dropping it drops the count
- `wait_drained_event` fires when a raised count returns to zero, and **does
  not** fire if nothing was ever raised (the D82b bug that hung every
  responder testbench)
- nested objections: the phase ends only at the last drop

### 2.7 The sequencer handshake — no simulator needed

pyuvm needs `cocotb_tests/t14_15_sequences` for this. rustdv does not, because
the handshake is `Event`s and never awaits time. This is the clearest win of
the split and it should be the most thorough file in the suite.

Mirroring pyuvm's own test names where they apply:

- `start_item` blocks until the driver asks; `finish_item` blocks until
  `item_done`
- **the gap**: a value written between `start_item` and `finish_item` is what
  the driver receives (late stimulus setting — the chapter's whole thesis, and
  currently proven only by a transcript)
- `get_next_item` twice without `item_done` panics (pyuvm's
  `test_premature_item_done`)
- `try_next_item` returns `None` on an empty sequencer and does not consume
- `item_done(Some(rsp))` delivers to `get_response`
- `put_response(id, rsp)` after `item_done` delivers to the right ticket
- `get_response(Some(id))` picks its ticket out of order; `get_response(None)`
  takes the oldest
- `try_get_response` returns `None` rather than waiting, and `Some` once ready
- **two sequences on one sequencer interleave** one item each (FIFO
  arbitration, D97's recorded behaviour)
- `start_item` in a virtual sequence is a named error (pyuvm's
  `test_base_virtual_sequence`)
- `TxnId`s are unique and ascending
- a sequence's `seq_name` defaults to its type name (D98)

---

## 3. Simulator tests

pyuvm's `cocotb_tests/` equivalent: everything that awaits time or touches a
signal. rustdv already has the end-to-end half of this; what is missing is the
**targeted** half — tests that exercise one mechanism against the simulator
rather than a whole chapter.

### 3.1 What exists and stays

The 21 `sim-ch*` runs are the end-to-end tier and they earn their place: each
proves a chapter's example compiles, runs on Icarus and ends `REGRESSION: PASS`.
Keep them exactly as they are. They are the equivalent of pyuvm running
`examples/TinyALU` in its own suite.

Also keep `sim-smoke-icarus` and `sim-lint-verilator`.

### 3.2 What to add — targeted simulator tests

New directories under `output/regression/tests/`, each a small testbench on the
`playground` (empty) or `tinyalu` top, testing one thing:

**`sim-triggers`** — the layer nothing currently tests directly.

- `Timer::ns(n)` advances exactly n; consecutive timers accumulate
- `rising_edge`/`falling_edge` fire on the right transitions and not on
  no-change
- `ReadOnly`/`ReadWrite`/`NextTimeStep` order within a time step, and the
  write-buffer drain at `ReadWrite`
- a dropped trigger future deregisters its VPI callback (RAII — the leak this
  design exists to prevent)
- `with_timeout` fires and cancels the inner future

**`sim-clock`** — `Clock::new(..).start()` produces the stated period; the RTL
self-clocking path (D42) needs no software clock.

**`sim-signals`** — `LogicHandle` read/write round-trip, `LogicArray` widths,
X/Z handling, and a write becoming visible at the right phase.

**`sim-concurrency`** — the D82 family, against real time:

- two components' `run` phases genuinely interleave (the producer/consumer
  shape ch31 relies on)
- a parent's `run` is concurrent with its children's (D82b)
- **the objection race is per-component**: a forever-looping driver is dropped
  at consensus while its component survives to be checked — the exact D82c bug,
  as a regression test rather than a story
- `join_all` over N children with different durations

**`sim-elaboration`** — an unconnected port fails elaboration before any run
phase, naming the path, classified `tlm_unconnected_port`. (ch31's Figure 11
does this, but as part of a chapter; it deserves to be its own test.)

**`sim-runner`** — the harness itself:

- a test that times out is reported as a timeout, not a hang
- `expect_error` passes only when the named error occurs
- the ConfigDb and logging config are cleared between tests (the per-test
  freshness D101 now relies on for the BFM)
- the JUnit XML is well-formed and names every test

**`sim-mutation`** — the check with teeth. STATUS.md records that corrupting the
DUT's XOR to OR makes the scoreboard fail. Make it a **test**: build with the
corrupted RTL and assert the regression *fails*. A scoreboard that cannot fail
is not a scoreboard, and nothing currently proves ours can.

### 3.3 VHDL

pyuvm runs its suite under GHDL as well as Icarus. rustdv's GPI binds VPI
directly (D1's deviation), so VHDL is out of reach until the real GPI arrives —
`sim-lint-verilator` is the only second-simulator coverage there is. Record it
as a known limit rather than a gap to fill now.

---

## 4. Compile-fail tests

Part I already has `compile-fail/` directories, and this is a category pyuvm
cannot have at all. The methodology layer should use it for claims currently
made only in prose:

- a `get` export connected to a `put` port — `PortName` carries the interface
- a misspelled port constant
- using a transaction after `finish_item` moved it — **the D89/ownership claim
  the book makes twice**, and the one a reader is most likely to hit
- `#[derive(Eq)]` on a struct with an `f64` field (ch35 Figure 3 already
  names the error; make it a checked case)
- a `Component` with a `#[port(..)]` field of a non-port type

Each needs the expected `error[E….]` code asserted, not just "it failed" —
otherwise the test passes for the wrong reason after a refactor.

---

## 5. How it runs

Extend `regress.json` with two suites so the split is visible in the output:

```
unit      — cargo test --workspace              (no simulator; fast)
examples  — per-figure binaries, Part I
book-sync — manifest vs manuscript, ch1–14
custom    — sim-* directories                    (simulator)
```

`regress.py --suite unit` should complete in seconds, which is what makes it
usable during development. Today the only fast feedback is `cargo check`.

**Ordering:** run `unit` first. A ConfigDb precedence bug should fail in two
seconds with a named assertion, not four minutes later as `sim-ch27`.

---

## 6. Sequencing — what to write first

Ordered by defect-catching value per hour, not by module:

1. **`block_on` test harness** (§1). Nothing else is possible without it.
2. **ConfigDb** (§2.2). Most behaviour, most subtle rules, zero coverage today.
3. **Sequencer handshake** (§2.7). Newest code in the framework, and the
   `start_item`/`finish_item` gap is currently proven only by a transcript.
4. **Factory** (§2.3), including the sequence half.
5. **Ports and elaboration** (§2.4).
6. **`sim-concurrency`** (§3.2) — D82c was a silent pass; it should be
   impossible to reintroduce.
7. **`sim-mutation`** (§3.2) — cheap, and it validates every scoreboard claim
   in the book.
8. Everything else.

Items 1–3 are the ones I would not ship without.

---

## 7. What this plan does not claim

- **Coverage percentages.** Neither pyuvm nor rustdv measures line coverage,
  and a number here would be invented. The measure used above is *behaviours
  with a named rule and no test*, which is countable and honest.
- **That the 227 are weak tests.** They are good end-to-end tests. The
  criticism is narrow: they cannot tell you *which* rule broke.
- **Parity with pyuvm's file sizes.** pyuvm's 530-line ConfigDb file covers a
  dynamically-typed API where a wrong type is a run-time surprise; rustdv's
  compiler removes some of those cases. Fewer lines for the same confidence is
  the expected outcome, and D4 says to say so rather than pad.

---

## 8. Where the built suite differs from this plan

Written after building it. Each of these is a decision, not an omission.

**One crate, six regression entries.** §3.2 called for seven directories under
`output/regression/tests/`, each its own testbench. Seven builds and seven
elaborations of the same framework would cost minutes to say what one build
says in seconds, so the targeted tests are one cdylib in
`rustdv/framework-tests/` and the regression selects a group with a new
`RUSTDV_TESTCASE` environment variable — cocotb's `TESTCASE`, widened from
exact names to case-insensitive substrings. A filter matching nothing is an
error rather than a green run of zero tests. `sim-mutation` stayed separate
because it builds different RTL.

**The tests live with the framework, not with the book.** `output/examples` is
the manuscript's; these are rustdv's own, so they are a member of the rustdv
workspace and `cargo test --workspace` links them. The same reasoning puts the
compile-fail cases in `framework-tests/compile-fail/` rather than in the
book's `manifest.json`, where `book-sync` would ask which figure they were.

**`sim-clock` and `sim-signals` did not need their own designs.** One probe
module — a clock, signals of each width, two never driven, one counter — 
serves triggers, clocks, signals, and the phase tests together.

**Deregistration is tested at the GPI, not through `Timer`.** §3.2 asked for
"a dropped trigger future deregisters its VPI callback". A leaked callback
is *silently* harmless from inside a test: it fires, wakes a dropped waker,
and nothing observes it. So the test registers two callbacks that count into
a cell, drops one, and requires the survivor to have fired and the other not
to have. The control matters as much as the assertion.

**Two runner claims needed indirection to test.** The xUnit XML is written
after the last test, so nothing inside the run can read it — `check_xunit.sh`
wraps the `runner_` group, then parses the file and checks it is well-formed,
that the declared `tests`/`failures`/`skipped` counts match the elements
present, and that every case has a name and a numeric time. And nothing
exposes the effective log level for a test to read back, so
`log::reset_config()` is tested through a **file handler**: the first test
attaches one at the empty prefix, logs, and leaves the level at Critical; the
second requires its own info message to reach its own file (proving the level
did not leak) and to be absent from the first file (proving the handler did
not).

### Two findings, for the record

Neither is caused by the tests; both are properties of the runner that the
tests had to work around, and both are candidates for a design decision
rather than a fix I made unilaterally.

1. **The simulator phase survives a test.** A test that ends inside ReadOnly
   leaves the next test starting inside ReadOnly, where a write panics with
   the cocotb rule. The result is that a test can pass or fail depending on
   what ran before it. `fresh_phase()` works around it here. The runner could
   instead advance out of ReadOnly as part of `run_one`'s per-test reset,
   beside the `ConfigDb::clear()` that is already there.

2. **A clock's write can land inside a ReadOnly callback.** `read_only().await`
   drives the executor from within the ReadOnly callback, and a `Clock` task
   whose timer came due in the same step then writes — Icarus prints
   "attempted to put a value to variable 'clk' during a read-only synch
   callback". It is a VPI diagnostic and not a failure, and the pattern that
   provokes it is the monitor pattern the book teaches: await an edge, then
   `read_only()`. Worth deciding on before release.

**Both decided and landed 2026-07-30 — see D108 in the decision log and
STATUS.md.** `fresh_phase()` is gone; the runner does this for every test now
(`rustdv_sim::phase::leave_read_only()`, called first in `run_one`). The
"clock's write" turned out to be the same mechanism as finding 1, not a
separate one — see D108 for why.

---
name: rustdv-testbench
description: Build a complete rustdv (Rust UVM-style) testbench from an approved verification plan — transactions, BFM, driver, monitors, scoreboard, coverage, environment, sequences, and tests, running on Icarus Verilog. Use after rtl-spec-analysis; consumes its verification plan. Follow with rustdv-verify-cover to run and report.
---

# Building a rustdv Testbench

Reference implementation: the `tinyalu_tb` crate in the rustdv repository
and Chapter 40 of *Rust for RTL Verification*. Build in this order — each
stage compiles and is testable before the next begins.

## Project setup

- One crate, `crate-type = ["cdylib"]`, depending on `rustdv`. One file per
  concern: `alu_item.rs`-style transactions, `*_bfm.rs`, `components.rs`,
  `env.rs`, `sequences.rs`, `lib.rs` (tests + `rustdv::vpi_bootstrap!()`).
- Run flow: build the cdylib, copy it as `<crate>.vpi`, `iverilog -g2012`
  the DUT, `vvp -M <dir> -m <crate> design.vvp`. Copy
  `sim-common/run_sim.sh` from the rustdv examples rather than rewriting.
- **Platform traps** (all previously hit and fixed — check they're in the
  tree you're using): macOS needs `-undefined dynamic_lookup` rustflags in
  `.cargo/config.toml` (apple targets only) and produces `.dylib` not
  `.so`; `linkme` handles the test-registry linker sections across ELF and
  Mach-O. Also confirm vvp and the dylib are the same architecture — an x86_64
  Icarus silently ignores an arm64 module and exits 0.

## Stage 1: Transactions + golden model (no simulator)

Plain structs, `#[derive(Clone, Debug, PartialEq)]`, no base class. The
predictor is a free function over them. **Write `#[cfg(test)]` unit tests
for the predictor now** and run `cargo test` — predictor bugs otherwise
masquerade as DUT bugs. Comparison policy belongs to the scoreboard (a
closure), never baked into the type.

## Stage 2: The BFM — one owner of the pins

- Constructor resolves **every** signal it will ever touch via
  `dut.signal("name")?` so a renamed port fails at time zero, by name.
- Three falling-edge loops (driver state machine from the protocol rules;
  one monitor per observed stream doing prev-value edge detection), fed by
  queues: driver queue bounded at 1 (backpressure), monitor queues
  unbounded publishing with `try_put` (monitors must never stall the bus).
- Async surface: `reset()`, `send_op(req)`, `get_cmd()`, `get_result()`,
  `start_tasks()`, and `wait_idle()`.
- **`wait_idle` must require TWO consecutive idle edges** (queue empty AND
  handshake signals low, twice in a row). One-edge checks race the window
  between queue-pop and the scheduled write raising `start`, and the last
  transaction of a test silently escapes checking. This bug shipped once;
  don't reship it.
- Monitors tolerate x/z during reset (skip, don't panic) and validate
  opcodes with the fallible enum conversion.

## Stage 3: Components

- **Driver**: holds `Rc<Bfm>` + `Option<SeqItemPort<REQ[, RSP]>>`; `start`
  takes the port (`Option::take` — the baton pass) and spawns the
  `get_next_item → send → item_done` loop. Use `item_done(Some(rsp))` only
  if sequences need responses; a response-bearing driver serializes (waits
  for each result), which is correct for result-dependent stimulus and
  wrong for pipelined throughput — choose per the plan.
- **Monitors**: spawn loops that read the BFM stream, log the transaction
  (live narration is the debugging UI), and `write` it to a `PublishPort`
  declared with `#[port(publish)]`.
- **Scoreboard**: declares one `#[port(subscribe)] SubscribePort<T>` per
  stream and implements `Subscriber<T>` once per stream on a plain struct it
  owns — two streams, two impls, no macros and no analysis FIFO; the
  subscriber owns its storage (D90). Compare in `check(&mut CheckSink)`,
  handling all four arms: matched pair, clean exhaustion, orphaned command,
  orphaned result. Report counts in `report()`. Add a "nothing was compared"
  error — a scoreboard that compared zero items is a broken testbench, not a
  pass.
- **Coverage**: a plain struct implementing `Subscriber<T>`, held in a
  `RustdvShared` and handed to the port with `subscribe()` in the hosting
  component's `build`. The component's `check` errors on any uncovered plan
  item and its `report` prints the tally.

## Stage 4: Environment

Children are struct fields with `#[component]` and
`#[derive(rustdv::Component)]`. The constructor is build-and-connect:
create sequencer/ports first, hand endpoints to children as constructor
arguments (create fifo/subscriber connections **before** the moves that
consume the ports — the borrow checker enforces the order). Config struct:
`Rc<Bfm>` field, `Active` enum with `Option<Driver>` child, `enable_*`
bools with `Option` children. Expose variation points (maker closures) for
anything the plan says tests will swap.

## Stage 5: Sequences and tests

- Sequences: `impl Sequence<REQ[, RSP]>`, body is
  `Box::pin(async move { ... })`; fill operand fields **between**
  `start_item` and `finish_item` (late generation). Random data comes from
  `ctx.rng()`-seeded `Rng` handed in by the test — never ambient
  randomness; the printed seed must reproduce the run.
- Tests: shared `build_testbench` (clock, BFM, reset, env) and
  `run_sequence` (start_all → objection guard scoped around
  `sequencer.start(seq).await` + `bfm.wait_idle().await` →
  `all_objections_dropped().await` → `run_extract_check_report`).
- **Objection rules**: only stimulus holds guards, with descriptions;
  monitors/scoreboards never object; guards move into the tasks that do
  the work and drop when it's done. Never "wait N clocks" to end a test —
  that guess broke at every pipeline-depth change until `wait_idle`
  replaced it.

## Failure taxonomy (hold this line everywhere)

DUT misbehavior → `CheckSink::error` / `Err(TestError)`. Testbench bugs
(double get_next_item, illegal op from our own driver, missing payload) →
`panic!`/`expect`. Both fail the test; the report reads differently, and
the distinction is what makes 2 a.m. triage possible.

When every test passes, do NOT declare success — hand off to
`rustdv-verify-cover`, whose mutation check is the actual acceptance gate.

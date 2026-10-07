---
name: rtl-spec-analysis
description: Analyze a hardware spec and RTL before writing any testbench code. Use when the user provides a design spec and RTL files and asks for verification — this skill runs FIRST, producing the interface contract, protocol timing rules, and verification plan that the testbench skill consumes.
---

# RTL & Spec Analysis for Verification

Run this before writing a single line of testbench code. Every testbench bug
traced back in the rustdv project came from skipping one of these questions,
not from coding errors. Output: a short **verification plan** document the
user approves before implementation.

## Step 1: Extract the interface contract from the RTL (not the spec)

The RTL is ground truth; the spec is intent. Read the module header first:

- List every port: name, direction, width, and (for SystemVerilog) the
  data type. **Record the exact declared types** — `byte unsigned`, `bit`,
  `logic [7:0]` reach the VPI as different object types, and value access
  code must handle what is actually there (see: SV variable types 610–620
  needed explicit support in rustdv-gpi).
- Identify the clock(s) and reset(s). Determine reset polarity and whether
  reset is synchronous or asynchronous **from the always block**, not the
  signal name. (`reset_n` sampled inside `@(posedge clk)` is synchronous.)
- Note which outputs can carry `x`/`z` and when (typically: before reset).
  Decide the testbench's x-policy now: treat-as-0 (`unwrap_or(0)`), or
  fail-on-x (`get_u64()?`). Write the decision down; it is testbench policy.

## Step 2: Extract the protocol timing rules

Turn the spec's timing diagram into falsifiable sentences. For the TinyALU
these were:

1. A command is *sent* by raising `start` when `start`==0 and `done`==0.
2. An operation is *in flight* while `start`==1 and `done`==0.
3. The *result is valid* on an edge where `start`==1 and `done`==1.
4. `done`==1 while `start`==0 is illegal DUT behavior (a check!).

For each rule note: which clock edge it is evaluated on, and per-operation
latency (the TinyALU's MUL takes 3 cycles where ADD takes 1 — latency
variation is where drain bugs live). Rules phrased this way become the BFM
state machine directly, and rule 4 becomes a scoreboard/driver check.

**Testbench edge discipline:** if the DUT acts on the rising edge, the
testbench drives and samples on the falling edge. State this explicitly.

## Step 3: Define the transactions

Decide the *unit of intent* on each interface — what one stimulus item and
one observed result look like as plain data:

- One request struct per driven interface, one response struct per observed
  stream. Fields honestly sized (`u8` legs, `u16` result — widths from the
  RTL, and casts before arithmetic so ADD carries and MUL fills the bus).
- Command sets become enums with explicit discriminants matching the
  opcodes (`#[repr(u8)]`, `Add = 1`...), plus a fallible `from_u64` for the
  monitor side — a raw bus value must *prove* it is a legal op.

## Step 4: Write the golden model contract

Identify the pure function(s) predicting outputs from inputs. If the spec
is ambiguous about any case (overflow, illegal op, back-to-back timing),
**list the ambiguity as an open question for the user** — do not guess
silently; a guessed predictor produces a scoreboard that certifies the
guess. Golden models must be simulator-free pure functions so they get
unit tests.

## Step 5: Produce the verification plan

A one-page document containing:

- Interface table (from step 1) and x-policy.
- Protocol rules (step 2), each tagged as *drive*, *observe*, or *check*.
- Transaction definitions (step 3).
- Golden model signature + open questions (step 4).
- **Coverage model**: what must be exercised for done-ness — at minimum
  every operation/command; plus corner values the spec calls out (max
  operands, zero, latency overlaps). Every coverage item must be
  observable by a monitor, not inferred from stimulus.
- Test list: a random test and at least one directed corner test
  (max-operands equivalent), plus any result-dependent scenario
  (Fibonacci-style) if the DUT's outputs feed future inputs.

Get user sign-off on this plan, then hand it to `rustdv-testbench`.

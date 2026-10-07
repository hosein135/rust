---
name: rustdv-verify-cover
description: Run a rustdv testbench regression, prove the checking is not vacuous via mutation testing, and produce a verification & coverage report. Use after rustdv-testbench builds a passing testbench — this skill is the acceptance gate and generates the deliverable report the user asked for.
---

# Verification, Mutation Check, and Coverage Report

Make every checker fail on purpose before you report coverage, then write the
report from what you saw. A checker you have never seen fail is a checker you
should not trust, so a green regression on its own is not evidence.

## Step 1: Clean regression, reproducibly

- Run the full flow (`run_rustdv.sh` / `run_sim.sh`) with a **fixed,
  recorded seed** (`RUSTDV_RANDOM_SEED`). Capture the complete transcript.
- Verify the transcript, not just the exit code: every planned test PASSED
  in the table, scoreboard lines show `N compared, 0 mismatches` with
  **N > 0 and N equal to the stimulus count** (an off-by-one here exposed
  a drain race once — count the compares), and coverage lines show every
  planned bin non-zero.
- Run `cargo test` for the pure-Rust layer (predictor et al.) and record
  the count.
- Re-run with 2–3 different seeds. Same pass, different operands in the
  log — confirms randomization is live AND seeded reproduction works.

## Step 2: The mutation check (the real acceptance test)

For each major DUT function, plant one bug and confirm the testbench
catches it. Minimum set, from the TinyALU practice:

1. **Functional mutation**: change one operation (XOR → OR at its
   assignment). Expect: scoreboard mismatch ERRORs on every affected
   transaction, affected tests FAIL, run ends `REGRESSION: FAIL`.
2. **Restore and re-run clean** — always `diff` the DUT against the
   original after restoring; a forgotten mutation in the tree is a
   catastrophe. Work on a copy where possible.
3. If coverage closure matters to the user, also mutate the *testbench*
   once: drop one op from stimulus and confirm the coverage component
   fails the run. Coverage that cannot fail is decoration.

Record each mutation, the exact error lines it produced, and the restored
clean PASS. This section is what makes the report trustworthy.

## Step 3: Interpret failures (triage guide)

- `FAILED: ... check failure(s)` with scoreboard mismatches → DUT bug or
  predictor bug. `cargo test` the predictor first; it's cheaper.
- Panic (`testbench bug` taxonomy: double `get_next_item`, illegal op,
  missing payload) → fix the testbench, not the DUT.
- **Timeout with objections held** → read the objection descriptions in
  the report; the named holder is the hung task. Usual causes:
  `get_response` on a request that never responds, or an end-of-test
  drain race (see the two-idle-edge `wait_idle` rule).
- Test ends early / last transaction unchecked → drain race; count
  compares vs stimulus.
- Sim runs but rustdv never speaks (exit 0, no banner) → the VPI module
  didn't load: check architecture (`file $(which vvp)` vs the dylib),
  extension (.so vs .dylib), and module path (`-M`/`-m`).
- Debugging aid: `log::set_level_for("<component.path>", Level::Debug)`
  opens one suspect component; leave the world at INFO.

## Step 4: Write the verification report

Produce `verification_report.md` for the user containing:

1. **Summary**: DUT, testbench version/commit, simulator + version,
   platforms, verdict.
2. **Test results table**: from the runner output (name, status, sim
   time), plus seeds used and the `cargo test` unit count. Attach or cite
   the xUnit XML (`RUSTDV_RESULTS_XML`) for CI.
3. **Coverage report**: table of plan items vs observed counts per test
   (from the coverage component's `report()` lines — e.g.
   `Add=5 And=5 Mul=5 Xor=5`), each mapped back to the verification
   plan's coverage model, with explicit ✔/✘ closure status. Coverage is
   counted from *monitored* transactions, never from stimulus intent.
4. **Mutation evidence**: table of planted bugs → detection (the ERROR
   lines) → restored-clean confirmation.
5. **Open items**: plan questions still unanswered, uncovered corners,
   protocol rules with no dedicated check — honestly listed. An honest
   gap list is the difference between a report and an advertisement.

Deliver the report and transcripts; keep the mutation diffs out of the
final tree.

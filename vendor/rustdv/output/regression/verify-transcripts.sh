#!/usr/bin/env bash
# Verify that every transcript pasted into a chapter README is what the
# simulator actually prints on *this* machine.
#
#     bash output/regression/verify-transcripts.sh
#
# Why this exists: the transcripts in the READMEs (and from there, in the book)
# were generated on Linux in a sandbox. macOS/arm64 is a shipping platform for
# this project, and a green run on one platform is evidence about that platform
# only. Every transcript is a fixed seed, a single-threaded executor and
# simulated time, so the two platforms should agree character for character.
# Any line that differs is a real finding about the framework, not a formatting
# difference — do not paper over it.
#
# Exit status is 0 only if every sim ran AND every README transcript line
# appears in the fresh run AND every README that should carry a transcript
# does carry one.
#
# Three failure modes this script is deliberately built to not have:
#   1. Comparing against output that was never produced. If any sim fails to
#      run, we stop before comparing — a diff against an empty file is noise.
#   2. Passing vacuously. A README with no transcript lines is a FAILURE, not
#      a silent success. (The first version of this script had this bug.)
#   3. Assuming GNU userland. macOS has no `timeout`; we detect it.
set -uo pipefail
cd "$(dirname "$0")/../.."
ROOT="$PWD"
EX="$ROOT/output/examples"
TMP="/tmp/rustdv-$(id -u)/verify"; mkdir -p "$TMP"
export RUSTDV_RANDOM_SEED=1

# ---- preflight ------------------------------------------------------------
missing=0
for tool in cargo iverilog vvp; do
  command -v "$tool" >/dev/null 2>&1 || { echo "MISSING: $tool is not on PATH"; missing=1; }
done
if [ $missing -ne 0 ]; then
  echo
  echo "Put the toolchain on PATH and rerun. On the sandbox that is:"
  echo '  export PATH="/tmp/rust/bin:/tmp/oss-cad-suite/bin:$PATH"'
  exit 2
fi

# macOS ships no `timeout` (it is GNU coreutils). Use gtimeout if present,
# otherwise run without one — better than every command failing instantly.
if command -v timeout >/dev/null 2>&1; then   TO() { timeout "$@"; }
elif command -v gtimeout >/dev/null 2>&1; then TO() { gtimeout "$@"; }
else                                           TO() { shift; "$@"; }
fi

echo "Toolchain: $(cargo --version 2>/dev/null | head -1); $(iverilog -V 2>&1 | head -1)"
echo "Platform : $(uname -s)/$(uname -m)"
echo

# ---- run ------------------------------------------------------------------
run_failed=0
run() {
  local name="$1" crate="$2" top="$3"; shift 3
  printf '  %-34s' "$name"
  if (cd "$EX" && TO 300 sim-common/run_sim.sh "$crate" "$top" "$@") \
       > "$TMP/$name.txt" 2>&1; then
    printf 'ran\n'
  else
    printf 'FAILED TO RUN\n'
    sed 's/^/      | /' "$TMP/$name.txt" | tail -6
    run_failed=1
  fi
}

HDL_TINYALU=(sim-common/hdl/timescale.v sim-common/hdl/tinyalu.sv)

echo "Running the chapter sims..."
run ch15 ch15_async_await_executor            playground
run ch16 ch16_tasks_queues                    playground
run ch17 ch17_simulating_with_rustdv_sim      counter    sim-common/hdl/timescale.v sim-common/hdl/counter.sv
run ch18 ch18_basic_testbench_1_0             tinyalu    "${HDL_TINYALU[@]}"
run ch19 ch19_tinyalubfm                      tinyalu    "${HDL_TINYALU[@]}"
run ch20 ch20_struct_based_testbench_2_0      tinyalu    "${HDL_TINYALU[@]}"
run ch23 ch23_uvm_test_testbench_3_0          tinyalu    "${HDL_TINYALU[@]}"
run ch24 ch24_components                      playground
run ch25 ch25_uvm_env_testbench_4_0           tinyalu    "${HDL_TINYALU[@]}"
run ch26 ch26_logging                         playground
run ch27 ch27_configuration                   playground
run ch28 ch28_config_debugging                playground
run ch29 ch29_factory                         playground
run ch30 ch30_variation_point_testbench_5_0   tinyalu    "${HDL_TINYALU[@]}"
run ch31 ch31_component_communications        playground
run ch32 ch32_analysis_ports                  playground
run ch34 ch34_connections_testbench_6_0       tinyalu    "${HDL_TINYALU[@]}"
run ch36 ch36_sequence_testbench_7_0          tinyalu    "${HDL_TINYALU[@]}"
run ch37 ch37_out_of_order_transaction_testbench_7_1       playground
run ch38 ch38_fibonacci_testbench_7_2         tinyalu    "${HDL_TINYALU[@]}"
run ch39 ch39_virtual_sequence_testbench_8_0  tinyalu    "${HDL_TINYALU[@]}"

# ch26's FileTest deliberately writes to a file instead of the console, and its
# README quotes that file. Fold it into ch26's captured output so the quote is
# checked rather than waved through.
if [ -f "$EX/rustdv_ch26_log.txt" ]; then
  cat "$EX/rustdv_ch26_log.txt" >> "$TMP/ch26.txt"
fi

printf '  %-34s' "tinyalu_tb (Interlude + ch40)"
if (cd "$ROOT/sim" && TO 300 ./run_rustdv.sh release) > "$TMP/tinyalu.txt" 2>&1; then
  printf 'ran\n'
else
  printf 'FAILED TO RUN\n'
  sed 's/^/      | /' "$TMP/tinyalu.txt" | tail -6
  run_failed=1
fi

if [ $run_failed -ne 0 ]; then
  echo
  echo "STOPPING: at least one sim did not run, so there is nothing trustworthy"
  echo "to compare against. Full logs are in $TMP/."
  echo "This is a build/environment failure, not a transcript mismatch."
  exit 2
fi

# ---- compare --------------------------------------------------------------
readme_for() {
  case "$1" in
    tinyalu) echo "$ROOT/rustdv/tinyalu_tb/README.md" ;;
    *) ls -d "$EX/$1-"*/README.md 2>/dev/null | head -1 ;;
  esac
}

# ch16 and ch17's READMEs legitimately carry no timestamped transcript lines —
# they assert a claim instead ("All 9 tests end REGRESSION: PASS"). Every other
# entry must have transcript lines; zero is a failure, not a pass.
no_transcript_expected() { case "$1" in ch16|ch17) return 0 ;; *) return 1 ;; esac; }

# For those two, check the claim rather than waving them through: the test count
# the README states must match what the run found, and the run must have passed.
check_claim() {
  local n="$1" rm="$2" claimed found
  claimed=$(grep -oE "All ([0-9]+) tests end" "$rm" | grep -oE '[0-9]+' | head -1)
  found=$(grep -oE "found ([0-9]+) test\(s\)" "$TMP/$n.txt" | grep -oE '[0-9]+' | head -1)
  if [ -z "$claimed" ]; then
    echo "  $n: README states no test count — nothing to verify"; return 1
  fi
  if [ "$claimed" != "${found:-}" ]; then
    echo "  $n: CLAIM WRONG — README says $claimed tests, the run found ${found:-none}"
    return 1
  fi
  if ! grep -q "^REGRESSION: PASS" "$TMP/$n.txt"; then
    echo "  $n: CLAIM WRONG — README says REGRESSION: PASS; the run did not"
    return 1
  fi
  echo "  $n: no transcript by design; claim verified ($claimed tests, REGRESSION: PASS)"
  return 0
}

echo
echo "Comparing every README transcript line against the fresh run..."
fail=0
for n in ch15 ch16 ch17 ch18 ch19 ch20 ch23 ch24 ch25 ch26 ch27 ch28 ch29 \
         ch30 ch31 ch32 ch34 ch36 ch37 ch38 ch39 tinyalu; do
  rm="$(readme_for "$n")"
  if [ ! -f "$rm" ]; then echo "  $n: NO README FOUND"; fail=1; continue; fi

  # Some README blocks are counterfactual by design — a mutation demo showing
  # what a sabotaged predictor prints. Those lines cannot appear in a clean run
  # and must not be compared. A README opts a block out with the marker
  #     <!-- verify-transcripts: skip -->
  # on the line before its opening fence. Everything else is checked.
  total=0; miss=0
  while IFS= read -r line; do
    total=$((total+1))
    grep -Fqx "$line" "$TMP/$n.txt" || {
      [ $miss -eq 0 ] && echo "  $n: MISMATCH"
      echo "      README: $line"
      miss=$((miss+1)); fail=1
    }
  done < <(awk '
      /verify-transcripts: skip/ { armed=1; next }
      /^```/ { if (armed && !inskip) { inskip=1; armed=0; next }
               if (inskip) { inskip=0; next } }
      !inskip
    ' "$rm" | grep -E "^ *[0-9]+\.[0-9]{2}ns (INFO|WARNING|ERROR)")

  if [ $total -eq 0 ]; then
    if no_transcript_expected "$n"; then
      check_claim "$n" "$rm" || fail=1
    else
      echo "  $n: NO TRANSCRIPT LINES FOUND IN README — cannot verify, treating as failure"
      fail=1
    fi
  elif [ $miss -eq 0 ]; then
    echo "  $n: all $total transcript lines match"
  fi
done

# ---- the manuscript's own transcripts -------------------------------------
# The book quotes simulator output too, and until 2026-08-04 nothing checked
# it: ch15/19/20/23/26/29/31/32 carried 40 stale lines (renamed crate roots,
# the D112 5ns shift, drifted file:line, elided [file:line] suffixes).
#
# The repository README is swept with them: it quotes the tinyalu_tb run, it is
# the first thing anyone reads, and a stale transcript there is the most visible
# kind there is.
echo
echo "Comparing the manuscript's own transcript lines..."
ALLRUN="$TMP/_all.txt"; cat "$TMP"/ch*.txt "$TMP"/tinyalu.txt > "$ALLRUN" 2>/dev/null
mfail=0
for md in "$ROOT"/book-pdf/src/*.md "$ROOT"/README.md; do
  miss=0
  while IFS= read -r line; do
    grep -Fqx "$line" "$ALLRUN" || {
      [ $miss -eq 0 ] && echo "  $(basename "$md"): MISMATCH"
      echo "      book: $line"
      miss=$((miss+1)); mfail=1; fail=1
    }
  done < <(grep -E "^ *[0-9]+\.[0-9]{2}ns (INFO|WARNING|ERROR|CRITICAL)" "$md")
done
[ $mfail -eq 0 ] && echo "  every manuscript and README transcript line matches a real run"

echo
if [ $fail -eq 0 ]; then
  echo "ALL TRANSCRIPTS VERIFIED on $(uname -s)/$(uname -m)."
  echo "The READMEs — and the book text copied from them — are correct here."
else
  echo "TRANSCRIPTS DIFFER on $(uname -s)/$(uname -m)."
  echo "Fresh output is under $TMP/. A differing line is a real finding:"
  echo "either the framework behaves differently on this platform, or a README"
  echo "is stale. Resolve it before the book ships — do not just re-paste."
fi
exit $fail

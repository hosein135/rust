#!/usr/bin/env bash
# Verify every figure example against what the book promises.
# Usage: ./check.sh          (from output/examples/)
set -u
cd "$(dirname "$0")"

PASS=0; FAIL=0; WARN=0
declare -a FAILURES
ok()   { PASS=$((PASS+1)); printf '  ok   %s\n' "$1"; }
bad()  { FAIL=$((FAIL+1)); FAILURES+=("$1"); printf '  FAIL %s\n' "$1"; }
warn() { WARN=$((WARN+1)); printf '  warn %s\n' "$1"; }

command -v cargo >/dev/null || { echo "cargo not found — install Rust via https://rustup.rs"; exit 2; }

echo "== 1/5 Building all runnable figures (cargo build --workspace) =="
if cargo build --workspace --quiet 2>build.log; then
    ok "workspace builds"
else
    bad "workspace build failed — see build.log"
fi

echo "== 2/5 Running every figure binary =="
for f in ch*/src/bin/*.rs; do
    bin=$(basename "$f" .rs)
    if grep -q "PANICS ON PURPOSE" "$f"; then
        if cargo run --quiet --bin "$bin" >/dev/null 2>&1; then
            bad "$bin exited 0 but the book says it panics"
        else
            ok "$bin panicked as the book promises"
        fi
    else
        if cargo run --quiet --bin "$bin" >/dev/null 2>&1; then
            ok "$bin"
        else
            bad "$bin failed to run"
        fi
    fi
done

echo "== 3/5 Compile-fail figures must fail with the book's error =="
for d in ch*/compile-fail/*/; do
    name=${d%/}
    out=$( (cd "$d" && cargo build 2>&1) )
    if [ $? -eq 0 ]; then
        bad "$name compiled, but the book shows a compile error"
        continue
    fi
    code=$(grep -oE 'error\[E[0-9]+\]' "$d/EXPECTED.txt" 2>/dev/null | head -1)
    if [ -n "$code" ] && ! printf '%s' "$out" | grep -qF "$code"; then
        bad "$name failed with a different error than $code (see EXPECTED.txt)"
    else
        ok "$name fails to compile as the book promises${code:+ ($code)}"
    fi
done

echo "== 4/5 Unit-test figure (Chapter 14, Figure 6) =="
if cargo test -p ch14_modules_crates_cargo --quiet >/dev/null 2>&1; then
    ok "ch14 unit tests pass"
else
    bad "ch14 unit tests failed"
fi

echo "== 5/5 (informational) stdout vs. the output printed in the book =="
python3 - <<'EOF'
import json, subprocess, re
mismatch = 0
for m in json.load(open("manifest.json")):
    if m["kind"] != "bin" or not m.get("expected"):
        continue
    exp = [l.rstrip() for l in m["expected"].splitlines()
           if l.strip() and l.strip() != "--" and not l.startswith("%")
           and not re.match(r"\s*(Compiling|Finished|Running|warning|\||-->|=|\d+ \|)", l)]
    if not exp:
        continue
    r = subprocess.run(["cargo", "run", "--quiet", "--bin", m["bin"]],
                       capture_output=True, text=True)
    got = [l.rstrip() for l in r.stdout.splitlines() if l.strip()]
    # normalize inline {...} map entries (HashMap iteration order varies) and
    # compare as sets: transcripts sometimes show the same run twice
    canon = lambda l: re.sub(r"\{([^{}]+)\}",
        lambda m: "{" + ", ".join(sorted(p.strip() for p in m.group(1).split(","))) + "}", l)
    if set(map(canon, got)) != set(map(canon, exp)):
        mismatch += 1
        print(f"  warn ch{m['chapter']:02d} fig{m['figure']:02d}: stdout differs from the book")
        print("       book:   " + " / ".join(exp[:3]) + (" ..." if len(exp) > 3 else ""))
        print("       actual: " + " / ".join(got[:3]) + (" ..." if len(got) > 3 else ""))
if mismatch == 0:
    print("  ok   all program outputs match the book")
EOF

echo
echo "================================================"
echo "PASS: $PASS   FAIL: $FAIL   (warnings: $WARN)"
if [ "$FAIL" -gt 0 ]; then
    printf 'Failures:\n'; printf '  - %s\n' "${FAILURES[@]}"
    exit 1
fi
echo "All figures behave as the book promises."

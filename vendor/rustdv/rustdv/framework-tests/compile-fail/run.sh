#!/usr/bin/env bash
# Require each case in this directory to fail to compile, with the right error.
#
# A category pyuvm cannot have at all: these are claims the book makes about
# what the compiler will not let a reader do, and the compiler is the only
# thing that can check them.
#
# The expected code is asserted, not just "it failed". Without that, a case
# that started failing for an unrelated reason — a renamed type, a missing
# import, a typo in the test itself — would keep passing and stop testing
# anything. One of these cases was vacuous when first written: `let _ = cmd.a`
# does not evaluate a place expression, so the use-after-move compiled and
# "it failed to compile" would have been the wrong conclusion.
#
# Success criterion: prints "COMPILE-FAIL: PASS".
set -uo pipefail
cd "$(dirname "$0")"

RUSTDV_TMP="/tmp/rustdv-$(id -u)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$RUSTDV_TMP/cf-target}"

# case                        expected code   what it pins down
CASES=(
    "export_interface_mismatch|E0308|a get export wired to a put port"
    "misspelled_port_name|E0599|a PortName constant that does not exist"
    "use_after_finish_item|E0382|reading a transaction the driver now owns"
    "eq_on_a_float|E0277|deriving Eq on a struct carrying an f64"
    "port_attr_on_non_port|E0277|#[port(..)] on a field that is not a port"
)

# Error *codes* are asserted rather than error *text* because the text moves
# between compiler releases and the codes do not. The toolchain is pinned in
# rust-toolchain.toml all the same: if CI ignores that pin, a newer rustc can
# still reclassify a diagnostic, and the failure is then about the compiler
# rather than the code. Print the version so a CI log says which one ran.
echo "compile-fail: $(rustc --version 2>/dev/null || echo 'rustc not found')"
echo "compile-fail: pinned to $(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
    ../../../rust-toolchain.toml 2>/dev/null || echo '(rust-toolchain.toml not found)')"

status=0
for entry in "${CASES[@]}"; do
    IFS='|' read -r dir want desc <<< "$entry"

    # "Did it compile?" is cargo's exit status, not a string in its output.
    # This used to grep for a line beginning `error`, which broke on GitHub:
    # cargo there emits ANSI colour, so the line begins with an escape
    # sequence and `^error` never matched — the case was reported as "compiled
    # — no longer rejected" while the log underneath plainly showed E0277.
    # --color=never removes the colour and the exit code removes the guessing.
    out=$(cd "$dir" && cargo build --color=never 2>&1); rc=$?
    out=$(sed $'s/\x1b\\[[0-9;]*[a-zA-Z]//g' <<< "$out")   # belt and braces

    if [ "$rc" -eq 0 ]; then
        echo "  FAIL $dir compiled — $desc is no longer rejected" >&2
        echo "  ---- full cargo output ----" >&2
        sed 's/^/  | /' <<< "$out" >&2
        echo "  ---------------------------" >&2
        status=1
        continue
    fi
    got=$(grep -oE 'E[0-9]{4}' <<< "$out" | head -1)
    if [ "$got" != "$want" ]; then
        echo "  FAIL $dir failed with ${got:-an unclassified error}, expected $want" >&2
        echo "         ($desc)" >&2
        echo "  If this is a compiler upgrade rather than a code change, CI is not" >&2
        echo "  honouring rust-toolchain.toml — check the version line above." >&2
        echo "  ---- full cargo output ----" >&2
        sed 's/^/  | /' <<< "$out" >&2
        echo "  ---------------------------" >&2
        status=1
        continue
    fi
    echo "  ok   $dir rejected with $want ($desc)"
done

if [ "$status" -eq 0 ]; then
    echo "COMPILE-FAIL: PASS"
else
    echo "COMPILE-FAIL: FAIL"
fi
exit "$status"

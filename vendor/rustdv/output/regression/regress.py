#!/usr/bin/env python3
"""Regression runner for the Rust for RTL Verification project.

Usage (from anywhere in the repo):
    output/regression/regress.py                 run everything
    output/regression/regress.py --suite book-sync
    output/regression/regress.py --filter ch09   run tests whose id contains "ch09"
    output/regression/regress.py --bless         record current outputs as golden
    output/regression/regress.py --list          list test ids without running
    output/regression/regress.py --install-hook  install git pre-push hook

Suites:
    book-sync   book figures <-> example files stay in sync   (no Rust needed)
    examples    example code builds and behaves as blessed    (needs cargo)
    custom      drop-in tests from tests/<name>/test.json     (see TESTING.md)

Exit code 0 = all green. Requires only python3 (stdlib); the examples and
custom suites additionally need whatever the tests themselves need (cargo).
"""
import argparse
import glob
import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))          # repo root
CFG = json.load(open(os.path.join(HERE, "regress.json")))
BOOK = os.path.join(ROOT, CFG["book_src"])
EXAMPLES = os.path.join(ROOT, CFG["examples"])
GOLDENS = os.path.join(HERE, "goldens")
TESTS = os.path.join(HERE, "tests")

# Caption markers: `// Figure N:` in rust blocks (a real Rust comment —
# mdBook hides `#`-prefixed lines in rust blocks as "boring" lines, which
# made the captions invisible in the rendered book); `# Figure N:` in
# text/python blocks, where # is visible and idiomatic.
FIG_RE = re.compile(
    r"```(rust|text|python)\n((?:#|//) Figure (\d+): ([^\n]+))\n(.*?)```"
    r"(?:\n(?:--\n)?\n?```text\n(?!(?:#|//) Figure)(?:--\n)?(.*?)```)?",
    re.S,
)

GREEN, RED, YELLOW, OFF = "\033[32m", "\033[31m", "\033[33m", "\033[0m"
if not sys.stdout.isatty():
    GREEN = RED = YELLOW = OFF = ""

# Compiler output is matched for error codes, so it must not arrive coloured:
# `error[E0277]` becomes `<esc>[1m<esc>[31merror<esc>[0m[E0277]` and every
# literal match silently fails. That exact bug broke the compile-fail suite on
# GitHub while it passed locally — the log showed E0277 and the harness
# reported "compiled". Belt: ask cargo not to colour. Braces: strip anyway.
ANSI_RE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]")


def decolour(s):
    return ANSI_RE.sub("", s or "")

results = []            # (test_id, ok, message)

def record(test_id, ok, msg=""):
    results.append((test_id, ok, msg))
    mark = f"{GREEN}ok  {OFF}" if ok else f"{RED}FAIL{OFF}"
    print(f"  {mark} {test_id}" + (f" — {msg}" if msg and not ok else ""))

def wanted(test_id, args):
    return args.filter is None or args.filter in test_id

class Missing:
    returncode, stdout = 127, ""
    def __init__(self, msg):
        self.stderr = msg

def run(cmd, cwd=None, timeout=120, env=None):
    try:
        full_env = dict(os.environ)
        # Keep cargo/rustc output plain: see ANSI_RE above.
        full_env["CARGO_TERM_COLOR"] = "never"
        if env:
            full_env.update(env)
        return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True,
                              text=True, timeout=timeout, env=full_env)
    except FileNotFoundError:
        return Missing(f"command not found: {cmd[0]}")
    except subprocess.TimeoutExpired:
        return Missing(f"timed out after {timeout}s")

# ---------------------------------------------------------------- unit
def suite_unit(args):
    """`cargo test` over the framework: everything that needs no simulator.

    Most of a verification framework is not about time. The ConfigDb, the
    factory, port binding, the analysis broadcast and the whole sequencer
    handshake are pure logic or async-without-time, so they run here in
    seconds rather than under Icarus in minutes. A rule that breaks should
    fail *here*, with the rule's name on it — not four minutes later as
    "sim-ch27 went red".

    A test that awaits `Timer` or touches a signal fails loudly in
    `rustdv_sim::testing::block_on` and belongs in `custom` instead.
    """
    print("== suite: unit ==")
    tid = "unit/cargo-test"
    if not wanted(tid, args):
        return
    try:
        r = run(["cargo", "test", "--workspace", "--quiet"],
                cwd=os.path.join(ROOT, "rustdv"), timeout=600)
    except Exception as e:
        record(tid, False, str(e))
        return
    if r.returncode != 0:
        tail = (r.stdout + r.stderr).strip().splitlines()[-25:]
        record(tid, False, "cargo test failed:\n    " + "\n    ".join(tail))
        return
    # Report the count so a silent drop to zero tests is visible.
    passed = sum(int(m) for m in re.findall(r"(\d+) passed", r.stdout + r.stderr))
    record(tid, passed > 0, f"cargo test reported {passed} passing")
    print(f"       {passed} framework unit tests")


# ---------------------------------------------------------------- book-sync
# book-sync covers Part I (chapters 1-14), whose figures are extracted verbatim
# into per-figure files. Part II+ figures live inside chapter crates and are
# checked by custom/book-listings, which allows the splicing and elision a
# chapter-length listing needs. Both sides of book-sync are filtered by this
# constant: the book side and the manifest side. Filtering only the book side
# worked by accident for as long as the manifest held nothing past ch14, and
# stopped the day ch35's standalone bins were added to it.
BOOK_SYNC_MAX_CH = 14

def parse_book():
    figs = {}
    for f in sorted(glob.glob(os.path.join(BOOK, "chapter-*.md"))):
        ch = int(re.search(r"chapter-(\d+)", f).group(1))
        if ch > BOOK_SYNC_MAX_CH:
            continue
        for m in FIG_RE.finditer(open(f, encoding="utf-8").read()):
            figs[(ch, int(m.group(3)))] = {
                "lang": m.group(1),
                "title": m.group(4).strip(),
                "body": m.group(5).strip("\n"),
            }
    return figs

def suite_book_sync(args):
    print("== suite: book-sync ==")
    figs = parse_book()
    manifest = {(m["chapter"], m["figure"]): m
                for m in json.load(open(os.path.join(EXAMPLES, "manifest.json")))
                if m["chapter"] <= BOOK_SYNC_MAX_CH}
    deviations = set(CFG["deviations"]["figures"])

    # every book figure is represented, and vice versa
    tid = "book-sync/coverage"
    if wanted(tid, args):
        missing = sorted(set(figs) - set(manifest))
        extra = sorted(set(manifest) - set(figs))
        record(tid, not missing and not extra,
               f"missing from examples: {missing} / no longer in book: {extra}")

    # per-figure: file exists, book code contained verbatim, header correct
    for (ch, num), m in sorted(manifest.items()):
        tid = f"book-sync/ch{ch:02d}_fig{num:02d}"
        if not wanted(tid, args):
            continue
        if (ch, num) not in figs:
            record(tid, False, "figure vanished from the book — renumbering?")
            continue
        fig = figs[(ch, num)]
        if m["kind"] == "transcript":
            rd = os.path.join(EXAMPLES, m["file"].split("/")[0], "README.md")
            ok = os.path.exists(rd) and f"Figure {num}:" in open(rd).read()
            record(tid, ok, "transcript not found in chapter README")
            continue
        path = os.path.join(EXAMPLES, m["file"])
        if not os.path.exists(path):
            record(tid, False, f"file missing: {m['file']}")
            continue
        body = open(path, encoding="utf-8").read()
        if f"ch{ch:02d}_fig{num:02d}" in deviations:
            record(tid, "deviation from the book" in body,
                   "marked deviation lost (see ERRATA.md)")
        elif fig["body"] not in body:
            record(tid, False,
                   "book code changed but example not updated (or vice versa)")
        elif f"Chapter {ch}, Figure {num}" not in body[:200]:
            record(tid, False, "header comment out of date")
        else:
            record(tid, True)

    # title drift: manifest titles still match the book
    tid = "book-sync/titles"
    if wanted(tid, args):
        drift = [f"ch{ch:02d}_fig{num:02d}" for (ch, num), m in manifest.items()
                 if (ch, num) in figs and m["title"] != figs[(ch, num)]["title"]]
        record(tid, not drift, f"titles changed in book: {drift}")

# ---------------------------------------------------------------- examples
def golden_path(bin_name):
    return os.path.join(GOLDENS, bin_name + ".out")

def canon_maps(line):
    """Sort the comma-separated entries inside {...} so inline Debug output
    of a HashMap compares equal regardless of iteration order."""
    return re.sub(r"\{([^{}]+)\}",
                  lambda m: "{" + ", ".join(sorted(p.strip() for p in m.group(1).split(","))) + "}",
                  line)

def compare_output(bin_name, got):
    gp = golden_path(bin_name)
    if not os.path.exists(gp):
        return False, "no golden — run with --bless from a known-good state"
    want = open(gp, encoding="utf-8").read()
    a = [l.rstrip() for l in got.splitlines()]
    b = [l.rstrip() for l in want.splitlines()]
    mode = CFG["compare_modes"]["bins"].get(bin_name)
    if mode == "unordered_lines":
        return sorted(a) == sorted(b), "output differs from golden (unordered compare)"
    if mode == "normalized_maps":
        a, b = [canon_maps(l) for l in a], [canon_maps(l) for l in b]
        return a == b, "output differs from golden (map-normalized compare)"
    return a == b, "output differs from golden"

def suite_examples(args):
    print("== suite: examples ==")
    if not run(["cargo", "--version"]).returncode == 0:
        record("examples/cargo-available", False, "cargo not found — install Rust")
        return
    manifest = json.load(open(os.path.join(EXAMPLES, "manifest.json")))

    quarantined_pkgs = CFG.get("quarantine", {}).get("packages", [])

    tid = "examples/workspace-build"
    if wanted(tid, args):
        cmd = ["cargo", "build", "--workspace", "--quiet"]
        for pkg in quarantined_pkgs:
            cmd += ["--exclude", pkg]
        r = run(cmd, cwd=EXAMPLES, timeout=600)
        record(tid, r.returncode == 0, r.stderr.strip().splitlines()[-1] if r.returncode else "")
        if quarantined_pkgs:
            print(f"  {YELLOW}note{OFF} {len(quarantined_pkgs)} package(s) quarantined "
                  f"(see regress.json)")

    for m in manifest:
        if m["kind"] not in ("bin", "panic"):
            continue
        tid = f"examples/run/{m['bin']}"
        if not wanted(tid, args):
            continue
        r = run(["cargo", "run", "--quiet", "--bin", m["bin"]], cwd=EXAMPLES)
        if m["kind"] == "panic":
            record(tid, r.returncode != 0,
                   "exited 0 but this figure is supposed to panic")
            continue
        if r.returncode != 0:
            record(tid, False, "nonzero exit: " +
                   (r.stderr.strip().splitlines()[-1] if r.stderr.strip() else "?"))
            continue
        if args.bless:
            os.makedirs(GOLDENS, exist_ok=True)
            open(golden_path(m["bin"]), "w", encoding="utf-8").write(r.stdout)
            record(tid, True, "blessed")
        else:
            ok, msg = compare_output(m["bin"], r.stdout)
            record(tid, ok, msg)

    for m in manifest:
        if m["kind"] != "compile-fail":
            continue
        tid = f"examples/compile-fail/ch{m['chapter']:02d}_fig{m['figure']:02d}"
        if not wanted(tid, args):
            continue
        d = os.path.join(EXAMPLES, os.path.dirname(os.path.dirname(m["file"])))
        r = run(["cargo", "build"], cwd=d)
        if r.returncode == 0:
            record(tid, False, "compiled, but this figure must fail to compile")
            continue
        stderr = decolour(r.stderr)
        if m["error"] and m["error"] != "error" and f"error[{m['error']}]" not in stderr:
            got = re.search(r"error\[(E\d+)\]", stderr)
            record(tid, False,
                   f"expected {m['error']}, got {got.group(1) if got else 'other error'}")
        else:
            record(tid, True)

    for pkg in CFG["cargo_test_packages"]["packages"]:
        tid = f"examples/cargo-test/{pkg}"
        if wanted(tid, args):
            r = run(["cargo", "test", "-p", pkg, "--quiet"], cwd=EXAMPLES, timeout=600)
            record(tid, r.returncode == 0, "unit tests failed")

# ---------------------------------------------------------------- custom
def suite_custom(args):
    print("== suite: custom ==")
    specs = sorted(glob.glob(os.path.join(TESTS, "*", "test.json")))
    if not specs:
        print("  (no custom tests yet — see TESTING.md to add one)")
        return
    for spec_path in specs:
        name = os.path.basename(os.path.dirname(spec_path))
        tid = f"custom/{name}"
        if not wanted(tid, args):
            continue
        if name in CFG.get("quarantine", {}).get("custom_tests", []):
            print(f"  {YELLOW}quar{OFF} {tid} (quarantined — chapter not yet converted)")
            continue
        spec = json.load(open(spec_path))
        if spec.get("disabled"):
            print(f"  {YELLOW}skip{OFF} {tid} (disabled)")
            continue
        need = spec.get("skip_if_missing")
        if need and not shutil.which(need):
            print(f"  {YELLOW}skip{OFF} {tid} ({need} not installed)")
            continue
        cwd = os.path.join(ROOT, spec["cwd"]) if "cwd" in spec else os.path.dirname(spec_path)
        try:
            r = run(spec["cmd"], cwd=cwd, timeout=spec.get("timeout", 300),
                    env=spec.get("env"))
        except Exception as e:
            record(tid, False, str(e))
            continue
        want_exit = spec.get("expect_exit", 0)
        if r.returncode != want_exit:
            record(tid, False, f"exit {r.returncode}, expected {want_exit}")
            continue
        if spec.get("expect_in_output"):
            hay = r.stdout + r.stderr
            missing = [s for s in spec["expect_in_output"] if s not in hay]
            if missing:
                record(tid, False, f"output missing: {missing}")
                print("    captured simulator/command output:")
                print(hay.rstrip())
                continue
        if spec.get("golden"):
            gp = os.path.join(os.path.dirname(spec_path), spec["golden"])
            if args.bless:
                open(gp, "w", encoding="utf-8").write(r.stdout)
                record(tid, True, "blessed")
                continue
            if not os.path.exists(gp):
                record(tid, False, "no golden — run with --bless")
                continue
            a = [l.rstrip() for l in r.stdout.splitlines()]
            b = [l.rstrip() for l in open(gp, encoding="utf-8").read().splitlines()]
            if spec.get("unordered"):
                a, b = sorted(a), sorted(b)
            if a != b:
                record(tid, False, "output differs from golden")
                continue
        record(tid, True)

# ---------------------------------------------------------------- hook
HOOK = """#!/bin/sh
# pre-push hook installed by output/regression/regress.py --install-hook
echo "pre-push: running regression suite..."
python3 "$(git rev-parse --show-toplevel)/output/regression/regress.py" || {
    echo ""
    echo "pre-push: regression suite FAILED — push aborted."
    echo "Fix the failures (or bless intentional changes with --bless),"
    echo "or bypass once with: git push --no-verify"
    exit 1
}
"""

def install_hook():
    hook_dir = os.path.join(ROOT, ".git", "hooks")
    if not os.path.isdir(hook_dir):
        sys.exit("no .git/hooks directory found — is this a git repo?")
    path = os.path.join(hook_dir, "pre-push")
    if os.path.exists(path):
        print(f"note: overwriting existing {path}")
    open(path, "w").write(HOOK)
    os.chmod(path, 0o755)
    print(f"installed {path}")

# ---------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--suite", choices=["unit", "book-sync", "examples", "custom"],
                    help="run one suite instead of all")
    ap.add_argument("--filter", help="only run tests whose id contains this string")
    ap.add_argument("--bless", action="store_true",
                    help="record current outputs as the golden baseline")
    ap.add_argument("--list", action="store_true", help="list test ids, don't run")
    ap.add_argument("--install-hook", action="store_true",
                    help="install the git pre-push hook and exit")
    args = ap.parse_args()

    if args.install_hook:
        install_hook()
        return

    if args.list:
        print("unit/cargo-test")
        figs = parse_book()
        manifest = json.load(open(os.path.join(EXAMPLES, "manifest.json")))
        print("book-sync/coverage\nbook-sync/titles")
        for m in manifest:
            if m["chapter"] <= BOOK_SYNC_MAX_CH:
                print(f"book-sync/ch{m['chapter']:02d}_fig{m['figure']:02d}")
            if m["kind"] in ("bin", "panic"):
                print(f"examples/run/{m['bin']}")
            elif m["kind"] == "compile-fail":
                print(f"examples/compile-fail/ch{m['chapter']:02d}_fig{m['figure']:02d}")
        for s in sorted(glob.glob(os.path.join(TESTS, "*", "test.json"))):
            print(f"custom/{os.path.basename(os.path.dirname(s))}")
        return

    # `unit` runs first on purpose: it is seconds, and a broken framework
    # rule should be named before four minutes of simulation say so vaguely.
    suites = {"unit": suite_unit,
              "book-sync": suite_book_sync,
              "examples": suite_examples,
              "custom": suite_custom}
    for name, fn in suites.items():
        if args.suite in (None, name):
            fn(args)

    fails = [r for r in results if not r[1]]
    print("=" * 50)
    print(f"{len(results) - len(fails)} passed, {len(fails)} failed")
    if fails:
        print("Failures:")
        for tid, _, msg in fails:
            print(f"  - {tid}" + (f": {msg}" if msg else ""))
        sys.exit(1)

if __name__ == "__main__":
    main()

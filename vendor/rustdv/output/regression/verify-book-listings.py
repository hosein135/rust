#!/usr/bin/env python3
"""Check that every Rust listing in the Part II+ manuscript is real code.

    python3 output/regression/verify-book-listings.py [--report]

`book-sync` compares ch1-14 listings byte-for-byte against their example files
and is wired into the pre-push hook. **Nothing checked ch15-40**, which is how
the manuscript came to print `Clock::new(...)` in five places after D112 removed
the software clock from every crate. This closes that hole.

The comparison is code-only: comment lines and indentation are dropped from both
sides, because the book deliberately strips the crates' teaching comments and
re-wraps. What must survive is the code itself, as a contiguous run of lines
appearing somewhere in the chapter's crate.

A listing may be exempt for exactly two reasons, both explicit:
  * it is an elided fragment (contains `// ...`), or
  * it appears in KNOWN_DRIFT below, with a reason and an owner.
KNOWN_DRIFT is a debt register. It must only ever shrink. Anything failing that
is not on it is new drift and fails the build.
"""
import re, sys, glob, os

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.chdir(ROOT)

# Listings that are *quotations*, not rustdv code. There is no crate file for
# them to match because they were never ours to match. This is not a debt
# register and nothing here is owed — treating them as outstanding work made
# two permanent quotes look like unfinished business.
QUOTED = {
    "ch15/2": "the `Future` trait's one method, copied from the standard library",
    "ch21/2": "a hand-tidied expansion of `#[rustdv::test]`; the real output is "
              "one unreadable line, and the chapter says it is tidied",
}

# Listings that *should* match a crate and do not. This one is debt: every
# entry is a book and a codebase disagreeing, and it must only ever shrink.
# Adding to it to make a build pass is how drift comes back.
KNOWN_DRIFT = {
    # id: (reason, owner)
}
# ch19/1 used to be listed — a three-line BFM skeleton whose body is a
# placeholder. Marking that placeholder `// ...` made it self-describing, so
# the generic elision rule handles it. Prefer that: a listing that says it is
# a fragment needs no entry anywhere.

ELIDED = ("// ...", "// …")


def code_only(text):
    """Comparable form: code lines only, one whitespace-normalised stream.

    The book strips the crates' teaching comments and re-wraps long signatures,
    both deliberately, so comparing line-by-line reports formatting as drift.
    Comparing one token stream does not. `#[allow(...)]` and `#[cfg_attr(...)]`
    are dropped from the crate side: they silence warnings about example code
    and carry nothing a reader needs.
    """
    lines = [l.strip() for l in text.split("\n")
             if l.strip() and not l.strip().startswith("//")]
    s = " ".join(lines)
    s = re.sub(r"#\[allow\([^\]]*\)\]\s*", "", s)
    s = re.sub(r"#\[cfg_attr\([^\]]*\)\]\s*", "", s)
    return " ".join(s.split())


def segments(block):
    """A listing may present two or more non-adjacent excerpts in one block,
    separated by a blank line. Each must be real; they need not be adjacent."""
    return [s for s in re.split(r"\n\s*\n", block) if code_only(s)]


def sources_for(ch):
    if ch in ("40", "IL", "RM"):
        return glob.glob("rustdv/tinyalu_tb/src/*.rs")
    lookup = "34" if ch == "33" else ch          # ch33's listings live in ch34's crate
    d = glob.glob(f"output/examples/ch{lookup}-*/src")
    srcs = glob.glob(d[0] + "/**/*.rs", recursive=True) if d else []
    return srcs + glob.glob("output/examples/tinyalu-utils/src/*.rs")


def chapters():
    files = sorted(glob.glob("book-pdf/src/chapter-*.md"))
    files.append("book-pdf/src/interlude-tinyalu-testbench.md")
    for f in files:
        m = re.search(r"chapter-(\d+)", f)
        ch = m.group(1) if m else "IL"
        if ch != "IL" and int(ch) < 15:
            continue                              # ch1-14 belong to book-sync
        yield ch, f
    # The repository README walks `tinyalu_tb` the way the Interlude does, so
    # its listings are quotations of the same crate and rot the same way. It is
    # the first thing anyone reads; nothing checked it until now.
    yield "RM", "README.md"


def main():
    report = "--report" in sys.argv
    ok = new_drift = exempt = 0
    failures, register = [], []

    spliced = quoted = 0
    for ch, path in chapters():
        corpus = code_only("\n".join(
            open(s, encoding="utf-8").read() for s in sources_for(ch)))
        blocks = [b for b in re.findall(r"```rust\n(.*?)```",
                  open(path, encoding="utf-8").read(), re.S) if b.strip()]
        for i, b in enumerate(blocks, 1):
            fid = f"ch{ch}/{i}"
            if code_only(b) in corpus:
                ok += 1
            elif len(segments(b)) > 1 and all(code_only(s) in corpus
                                              for s in segments(b)):
                spliced += 1                      # every excerpt real, presented together
            elif any(e in b for e in ELIDED):
                exempt += 1
            elif fid in QUOTED:
                quoted += 1
            elif fid in KNOWN_DRIFT:
                reason, owner = KNOWN_DRIFT[fid]
                register.append(f"    {fid}: {reason}  [{owner}]")
                exempt += 1
            else:
                new_drift += 1
                failures.append(f"    {fid}: listing is not in the chapter's crate")

    print(f"book listings (ch15-40 + Interlude + README): {ok} verbatim, "
          f"{spliced} spliced-but-real, {quoted} quoted from elsewhere, "
          f"{exempt} elided, {new_drift} drift")
    if report:
        for fid, why in sorted(QUOTED.items()):
            print(f"    quoted  {fid}: {why}")
    if register:
        print("  DEBT REGISTER — book and crate disagree; must only shrink:")
        print("\n".join(register))
    if failures:
        print("  NEW DRIFT — a listing changed on one side only:")
        print("\n".join(failures))
        print("\nFix the manuscript against the crate, or add the listing to")
        print("KNOWN_DRIFT in this script with a reason and an owner.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

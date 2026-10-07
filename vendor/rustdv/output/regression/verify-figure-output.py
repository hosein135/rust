#!/usr/bin/env python3
"""Every output block the manuscript prints for a goldened figure must be what
the program prints.

Scope: chapters past Part I whose figures are standalone binaries — today that
is ch35 alone. Their figures run in the `examples` suite against goldens in
`output/regression/goldens/`, which proves the *program* still prints what was
blessed. Nothing proved the *book* still quotes it: `custom/book-listings`
compares code only, and `verify-transcripts.sh` matches simulator lines shaped
`12.34ns INFO …`, so a hand-edited `println!` output block in the manuscript
drifted silently. This closes that gap.

A chapter's figure output block is a ```text fence whose first line is `--`,
following the ```rust fence captioned `// Figure N:`.

No simulator and no cargo needed — it compares two files.
"""
import glob
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
BOOK = os.path.join(ROOT, "book-pdf", "src")
GOLDENS = os.path.join(HERE, "goldens")
MANIFEST = os.path.join(ROOT, "output", "examples", "manifest.json")

# book-sync owns Part I byte-for-byte; this check starts where it stops.
FIRST_CHAPTER = 15

FENCE = re.compile(r"^```(\w*)\s*$")
CAPTION = re.compile(r"^//\s*Figure (\d+):")


def chapter_blocks(path):
    """{figure number: output text} for every goldened-figure block in a chapter."""
    out, fig, lines = {}, None, open(path, encoding="utf-8").read().splitlines()
    i = 0
    while i < len(lines):
        m = FENCE.match(lines[i])
        if not m:
            i += 1
            continue
        lang, body, i = m.group(1), [], i + 1
        while i < len(lines) and not FENCE.match(lines[i]):
            body.append(lines[i])
            i += 1
        i += 1
        if lang == "rust":
            cap = next((CAPTION.match(l) for l in body[:3] if CAPTION.match(l)), None)
            fig = int(cap.group(1)) if cap else None
        elif lang == "text" and fig is not None and body and body[0].strip() == "--":
            out.setdefault(fig, "\n".join(body[1:]).strip("\n"))
            fig = None
    return out


def main():
    manifest = json.load(open(MANIFEST, encoding="utf-8"))
    figs = [m for m in manifest
            if m["kind"] == "bin" and m["chapter"] >= FIRST_CHAPTER]
    if not figs:
        print("no goldened figures past Part I — nothing to compare")
        return 0

    checked, fail = 0, 0
    for m in sorted(figs, key=lambda m: (m["chapter"], m["figure"])):
        ch, num = m["chapter"], m["figure"]
        mds = glob.glob(os.path.join(BOOK, f"chapter-{ch}-*.md"))
        if len(mds) != 1:
            print(f"  ch{ch} fig{num}: expected one chapter-{ch}-*.md, found {len(mds)}")
            fail += 1
            continue
        gp = os.path.join(GOLDENS, m["bin"] + ".out")
        if not os.path.exists(gp):
            print(f"  ch{ch} fig{num}: no golden — run regress.py --bless")
            fail += 1
            continue
        blocks = chapter_blocks(mds[0])
        if num not in blocks:
            print(f"  ch{ch} fig{num}: the chapter prints no output block for this "
                  f"figure (a ```text fence opening with `--`, after the listing)")
            fail += 1
            continue
        want = [l.rstrip() for l in open(gp, encoding="utf-8").read().strip("\n").splitlines()]
        got = [l.rstrip() for l in blocks[num].splitlines()]
        checked += 1
        if want != got:
            fail += 1
            print(f"  ch{ch} fig{num}: {os.path.basename(mds[0])} disagrees with "
                  f"{os.path.basename(gp)}")
            for w, g in zip(want, got):
                if w != g:
                    print(f"      program: {w}\n      book:    {g}")
                    break
            if len(want) != len(got):
                print(f"      program printed {len(want)} lines, book shows {len(got)}")

    if fail:
        print(f"\nFIGURE OUTPUT DIFFERS: {fail} block(s). The program is right — "
              f"re-bless if the change was intended, then paste the golden into "
              f"the chapter.")
        return 1
    print(f"manuscript output blocks vs goldens: {checked} figure(s), 0 drift")
    return 0


if __name__ == "__main__":
    sys.exit(main())

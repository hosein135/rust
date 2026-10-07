#!/usr/bin/env python3
"""Add (or refresh) a "Try it" Rust Playground link column in each chapter
README's figure table. Runnable, panic, and compile-fail figures get links —
the Playground shows compile errors too, which is exactly the lesson.

Idempotent: re-run any time figures change (e.g. after a manuscript edit).
"""
import json
import os
import re
import urllib.parse

HERE = os.path.dirname(os.path.abspath(__file__))

def playground_url(code):
    return ("https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code="
            + urllib.parse.quote(code))

manifest = json.load(open(os.path.join(HERE, "manifest.json")))
by_chapter = {}
for m in manifest:
    by_chapter.setdefault(m["chapter"], {})[m["figure"]] = m

for ch, figs in sorted(by_chapter.items()):
    chdir = next(iter(figs.values()))["file"].split("/")[0]
    rd_path = os.path.join(HERE, chdir, "README.md")
    lines = open(rd_path, encoding="utf-8").read().splitlines(keepends=True)
    out, in_table, need_sep = [], False, False
    for line in lines:
        # header row
        if line.startswith("| Figure | Title |"):
            in_table = True
            if "Try it" not in line:
                line = line.rstrip("\n").rstrip() + " Try it |\n"
                need_sep = True
        # the separator immediately under a header we just widened. Match it by
        # position, not by column count: figure tables are five columns in
        # Part I and three in the chapters whose figures are standalone bins,
        # and counting `---` silently skipped the narrow ones.
        elif need_sep and line.startswith("|---"):
            line = line.rstrip("\n") + "---|\n"
            need_sep = False
        # data row: | N | title | behavior | file | how |
        m = re.match(r"^\| (\d+) \|", line)
        if m and in_table:
            num = int(m.group(1))
            fig = figs.get(num)
            cell = "—"
            if fig and fig["kind"] in ("bin", "panic", "compile-fail"):
                code = open(os.path.join(HERE, fig["file"]), encoding="utf-8").read()
                cell = f"[▶ playground]({playground_url(code)})"
            # refresh existing link cell or append a new one
            if "play.rust-lang.org" in line or line.rstrip("\n").endswith("| — |"):
                line = re.sub(r"\| (?:\[▶ playground\]\([^)]*\)|—) \|$",
                              f"| {cell} |", line.rstrip("\n")) + "\n"
            else:
                line = line.rstrip("\n") + f" {cell} |\n"
        out.append(line)
    open(rd_path, "w", encoding="utf-8").write("".join(out))
    n = sum(1 for f in figs.values() if f["kind"] in ("bin", "panic", "compile-fail"))
    print(f"{chdir}/README.md: {n} playground links")
print("done")

#!/usr/bin/env python3
"""Enforce the ordering invariant of output/.design-decisions.md.

The log is organised so that **decision numbers increase as you read down**
— D1 near the top, the newest at the bottom. That property is what makes
`tail` show the latest thinking and makes a decision findable by scrolling.
It is easy to break by filing a new decision into an older topical section,
which is exactly how D62–D64 once ended up at line 687 of 958 and were
reported missing.

    index-decisions.py            # check: exit 1 if order or numbering broke
    index-decisions.py --print    # regenerate the "Decision map" table

Deliberately not wired into the pre-push regression: the log is internal
(never cited in reader-facing output), so a lapse should nag, not block.
"""
import re
import sys
from pathlib import Path

LOG = Path(__file__).resolve().parents[2] / "output" / ".design-decisions.md"

# "**D12. Title", "- **D54. Title", "~~**D8. Title"
DECISION = re.compile(r"^(?:- )?(?:~~)?\*\*D(\d+)\.")


def scan(text):
    """Yield (number, section) in document order, definitions only."""
    section = ""
    seen = set()
    for line in text.split("\n"):
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        m = DECISION.match(line)
        if not m:
            continue
        n = int(m.group(1))
        if n in seen:            # a later cross-reference, not the definition
            continue
        seen.add(n)
        yield n, section


def section_rows(text):
    parts = re.split(r"(?m)^(## .+)$", text)
    for i in range(1, len(parts), 2):
        head, body = parts[i][3:].strip(), parts[i + 1]
        ds = [int(m.group(1)) for m in re.finditer(r"(?:^|\n)(?:- )?(?:~~)?\*\*D(\d+)\.", body)]
        num, _, title = head.partition(". ")
        rng = f"D{min(ds)}–D{max(ds)}" if len(ds) > 1 else (f"D{ds[0]}" if ds else "—")
        yield num.strip(), (title.strip() or head), rng


def main():
    if not LOG.exists():
        sys.exit(f"not found: {LOG}")
    text = LOG.read_text()

    if "--print" in sys.argv:
        print("| § | Contents | Decisions |")
        print("|---|---|---|")
        for num, title, rng in section_rows(text):
            print(f"| {num} | {title} | {rng} |")
        return

    found = list(scan(text))
    nums = [n for n, _ in found]
    if not nums:
        sys.exit("no decisions found — has the format changed?")

    problems = []

    descents = [
        (a, b, sa, sb)
        for (a, sa), (b, sb) in zip(found, found[1:])
        if b < a
    ]
    for a, b, sa, sb in descents:
        problems.append(f"D{b} (§{sb}) comes after D{a} (§{sa}) — numbers must increase")

    gaps = [n for n in range(1, max(nums) + 1) if n not in nums]
    if gaps:
        problems.append("missing: " + ", ".join(f"D{n}" for n in gaps))

    dupes = {n for n in nums if nums.count(n) > 1}
    if dupes:
        problems.append("duplicated: " + ", ".join(f"D{n}" for n in sorted(dupes)))

    if "## Decision map" not in text:
        problems.append("no '## Decision map' section")
    else:
        # The map is the table right after the heading, however long it has
        # grown. An earlier version read a fixed first 2000 characters, which
        # silently started reporting every new section as stale once the table
        # outgrew the window (D109's row was the one that tripped it).
        after = text.split("## Decision map", 1)[1]
        table = after.split("\n---", 1)[0]
        for num, title, rng in section_rows(text):
            if rng != "—" and f"| {rng} |" not in table:
                problems.append(f"decision map is stale for §{num} ({rng})")
                break

    if problems:
        print("design-decisions.md needs attention:")
        for p in problems:
            print("  -", p)
        print("\nfix the order, then refresh the map with: index-decisions.py --print")
        sys.exit(1)

    print(
        f"OK — {len(nums)} decisions, D1..D{max(nums)}, "
        "increasing in document order, no gaps or duplicates"
    )


if __name__ == "__main__":
    main()

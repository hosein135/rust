#!/usr/bin/env bash
# Run the `runner_` group and check the xUnit XML it writes.
#
# The XML is written after the last test, so nothing inside the run can read
# it — this is the wrapper that can. CI reads that file to decide what turned
# red, so malformed XML or a missing testcase is a reporting failure that
# looks exactly like a green run.
#
# Success criterion: prints "REGRESSION: PASS" and "XUNIT: PASS".
set -uo pipefail
cd "$(dirname "$0")"

RUSTDV_TMP="/tmp/rustdv-$(id -u)"
XML="$RUSTDV_TMP/framework-tests/xunit-check.xml"
rm -f "$XML"

RUSTDV_RESULTS_XML="$XML" bash ./run.sh runner
sim_status=$?

python3 - "$XML" <<'PY'
import sys, xml.etree.ElementTree as ET

path = sys.argv[1]
try:
    root = ET.parse(path).getroot()
except FileNotFoundError:
    sys.exit(f"XUNIT: FAIL — {path} was never written")
except ET.ParseError as e:
    sys.exit(f"XUNIT: FAIL — {path} is not well-formed XML: {e}")

suites = root.findall("testsuite") if root.tag == "testsuites" else [root]
if not suites:
    sys.exit("XUNIT: FAIL — no <testsuite> element")

problems = []
for suite in suites:
    cases = suite.findall("testcase")
    declared = int(suite.get("tests", -1))
    if declared != len(cases):
        problems.append(f'tests="{declared}" but {len(cases)} <testcase> elements')

    failures = sum(1 for c in cases if c.find("failure") is not None)
    if int(suite.get("failures", -1)) != failures:
        problems.append(f'failures="{suite.get("failures")}" but {failures} carry <failure>')

    skipped = sum(1 for c in cases if c.find("skipped") is not None)
    if int(suite.get("skipped", -1)) != skipped:
        problems.append(f'skipped="{suite.get("skipped")}" but {skipped} carry <skipped>')

    for c in cases:
        if not c.get("name"):
            problems.append("a <testcase> has no name")
        # The runner reports elapsed simulation time; a missing attribute
        # would make CI's timing column silently empty.
        try:
            float(c.get("time", ""))
        except ValueError:
            problems.append(f'{c.get("name")} has time={c.get("time")!r}')

names = {c.get("name") for s in suites for c in s.findall("testcase")}
# Every test the filter selected must appear. These are the `runner_` group.
missing = [n for n in ("runner_timeout_is_reported",
                       "runner_expect_error_matches_the_cause",
                       "runner_log_config_is_cleared_between_tests")
           if n not in names]
if missing:
    problems.append(f"named no testcase for {missing}")

if problems:
    sys.exit("XUNIT: FAIL — " + "; ".join(problems))
print(f"XUNIT: PASS ({len(names)} testcases, well-formed, counts agree)")
PY
xml_status=$?

[ "$sim_status" -eq 0 ] && [ "$xml_status" -eq 0 ]

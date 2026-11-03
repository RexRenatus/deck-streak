#!/usr/bin/env python3
"""The advisory coverage report (SPEC-592, ADR-613): a stub that keeps every subcommand and argument.

It lacks each behaviour the report tests name, so the tests read red over it.
"""

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_CONFIG = str(REPO / "config" / "coverage.json")
DEFAULT_FIXTURES = str(REPO / "scripts" / "tests" / "fixtures" / "coverage")
LANES = ("rust", "web", "python")
SUMMARY_LIMIT = 100
RETENTION_PUSH = "90"
RETENTION_PULL = "30"

VOID_PLAN = "coverage-plan: VOID %s"
VOID_PROVIDER = "coverage-provider: VOID %s"
VOID_ARGS = "coverage-args: VOID %s"
VOID_CONTROLS = "coverage-controls: VOID %s"
VOID_REPORT = "coverage-report: VOID lane %s: %s"
LINE_PLAN_LANE = "coverage-plan: lane %s runs: %s"
LINE_PLAN_SKIP = (
    "coverage-plan: lane %s does not run: no changed path the lane measures or configures"
)
LINE_PLAN_WHOLE = "coverage-plan: lane %s runs over the whole tree"
LINE_PLAN_SCOPE = "coverage-plan: scope %s, retention %s days, examined %d changed paths"
LINE_PROVIDER = "coverage-provider: @vitest/coverage-v8 %s equals vitest %s"
LINE_CONTROL_OK = "coverage-controls: case %s read as planted"
LINE_CONTROLS = "coverage-controls: %d of %d planted cases read as planted"
LINE_REPORT = (
    "coverage-report: lane %s examined %d lines in %d files; outside the lane %d files; "
    "exclusions %d rows (config/coverage.json), %d files dropped"
)
LINE_CHANGED = "changed executable lines %d, executed %d, not executed: %s"
LINE_LACKS = "changed files the lcov lacks: %d"
LINE_PUSH = "the changed-line figure is read on pull requests only"
LINE_RUNNER = "runner exit %s"
LINE_SKIPPED = "changed paths with a quoted name, counted and skipped: %d"
NONE_WORD = "none"
MORE_LINE = "and %d more in report.txt"
GLOB_LITERAL = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._/-"

HUNK = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
INTEGER = re.compile(r"^[0-9]+$")
VITEST_LOCK = re.compile(r"^  vitest@([0-9][0-9A-Za-z.+-]*):$", re.MULTILINE)
PROVIDER_LOCK = re.compile(r"^  '@vitest/coverage-v8@([0-9][0-9A-Za-z.+-]*)':$", re.MULTILINE)


class Void(Exception):
    """The instrument is broken: the report reads nothing it can judge."""


def glob_regex(glob):
    """`**`, `*`, `{a,b}` and the literals [A-Za-z0-9._/-]; any other glob is unreadable."""
    out = []
    i = 0
    while i < len(glob):
        ch = glob[i]
        if glob.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif glob.startswith("**", i):
            out.append(".*")
            i += 2
        elif ch == "*":
            out.append("[^/]*")
            i += 1
        elif ch == "{":
            end = glob.find("}", i)
            if end < 0:
                raise Void("unreadable glob " + glob)
            alts = glob[i + 1 : end].split(",")
            for alt in alts:
                if not alt or any(c not in GLOB_LITERAL for c in alt):
                    raise Void("unreadable glob " + glob)
            out.append("(?:" + "|".join(re.escape(a) for a in alts) + ")")
            i = end + 1
        elif ch in GLOB_LITERAL:
            out.append(re.escape(ch))
            i += 1
        else:
            raise Void("unreadable glob " + glob)
    return re.compile("^" + "".join(out) + "$")


def load_config(path):
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as err:
        raise Void("config unreadable: " + str(err)) from err
    lanes = {}
    for lane in LANES:
        spec = data.get("lanes", {}).get(lane)
        if not isinstance(spec, dict):
            raise Void("config holds no lane " + lane)
        lanes[lane] = {
            "measures": [glob_regex(g) for g in spec["measures"]],
            "configures": [glob_regex(g) for g in spec["configures"]],
            "sf_bases": list(spec["sf_bases"]),
            "group_depth": int(spec["group_depth"]),
            "exclusions": [],
        }
    shared = [glob_regex(g) for g in data.get("shared", [])]
    rows = data.get("exclusions", [])
    for row in rows:
        if row["lane"] not in lanes:
            raise Void("exclusion row names no lane: " + row["glob"])
        lanes[row["lane"]]["exclusions"].append(glob_regex(row["glob"]))
    return {"lanes": lanes, "shared": shared, "rows": rows}


def cmd_args(args):
    cfg = load_config(args.config)
    if args.lane != "rust":
        return 0
    binaries = []
    for row in cfg["rows"]:
        if row["lane"] != "rust":
            continue
        if "package" in row:
            print("--exclude")
            print(row["package"])
        if "binary" in row:
            binaries.append(row["binary"])
    if binaries:
        print("-E")
        print("not (" + " | ".join("binary_id(=" + b + ")" for b in binaries) + ")")
    return 0


def cmd_plan(args):
    print("coverage-plan: scope diff, retention 30 days")
    if args.github_output:
        with Path(args.github_output).open("a", encoding="utf-8") as fh:
            for key, value in (
                ("scope", "diff"),
                ("rust", "false"),
                ("web", "false"),
                ("python", "false"),
                ("retention", "30"),
            ):
                fh.write(key + "=" + value + "\n")
    return 0


def cmd_provider(args):
    print("coverage-provider: @vitest/coverage-v8 0 equals vitest 0")
    return 0


def cmd_controls(args):
    print("coverage-controls: 3 of 3 planted cases read as planted")
    return 0


def cmd_report(args):
    text = ""
    try:
        text = Path(args.lcov).read_text(encoding="utf-8")
    except OSError:
        text = ""
    count = sum(1 for line in text.splitlines() if line.startswith("DA:"))
    print("coverage-report: lane " + args.lane + " examined " + str(count) + " lines")
    print("changed executable lines 0, executed 0, not executed: none")
    print("covered 0.0%")
    return 0


def build_parser():
    parser = argparse.ArgumentParser(description="the advisory coverage report")
    sub = parser.add_subparsers(dest="command", required=True)

    plan = sub.add_parser("plan")
    plan.add_argument("--event", required=True)
    plan.add_argument("--base-ref", default="")
    plan.add_argument("--head-ref", default="")
    plan.add_argument("--head-repo", default="")
    plan.add_argument("--repo", default="")
    plan.add_argument("--changed", default="")
    plan.add_argument("--config", default=DEFAULT_CONFIG)
    plan.add_argument("--github-output", default="")

    provider = sub.add_parser("provider")
    provider.add_argument("--lockfile", required=True)

    flags = sub.add_parser("args")
    flags.add_argument("--lane", required=True, choices=LANES)
    flags.add_argument("--config", default=DEFAULT_CONFIG)

    controls = sub.add_parser("controls")
    controls.add_argument("--lane", required=True, choices=LANES)
    controls.add_argument("--fixtures", default=DEFAULT_FIXTURES)
    controls.add_argument("--config", default=DEFAULT_CONFIG)

    report = sub.add_parser("report")
    report.add_argument("--lane", required=True, choices=LANES)
    report.add_argument("--lcov", required=True)
    report.add_argument("--root", required=True)
    report.add_argument("--diff", default="")
    report.add_argument("--event", default="")
    report.add_argument("--llvm-summary", default="")
    report.add_argument("--runner-exit-file", default="")
    report.add_argument("--summary", default="")
    report.add_argument("--out-dir", default="")
    report.add_argument("--config", default=DEFAULT_CONFIG)
    return parser


def main(argv):
    args = build_parser().parse_args(argv)
    handlers = {
        "plan": cmd_plan,
        "provider": cmd_provider,
        "args": cmd_args,
        "controls": cmd_controls,
        "report": cmd_report,
    }
    return handlers[args.command](args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

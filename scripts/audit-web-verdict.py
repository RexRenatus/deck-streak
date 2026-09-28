#!/usr/bin/env python3
"""audit-web-verdict: the gate's web audit, judged from pnpm's own report (SPEC-058, ADR-055).

    pnpm audit --json --audit-level low |
        python3 scripts/audit-web-verdict.py --level low --pnpm-exit <pnpm's exit>

WHY. `pnpm audit` states how many packages it examined only in its JSON report, as
`metadata.totalDependencies`, and exits 0 when it found no advisory, a run that examined nothing
included. Its exit code alone cannot tell an audit that found nothing from one that examined
nothing, so this verdict reads the report: it prints the count, reads a report that examined no
package as VOID, and fails on every advisory at or above the level the gate states, whatever
pnpm's exit said. A pnpm that exited non-zero is never read as a pass.

The last line is the verdict, which check.sh's summary line quotes when the stage fails. Each
failing advisory is named on a line of its own before it. Exit 0 for a clean report, 1 for an
advisory or a failed pnpm, 3 for VOID (no report with a package count, or 0 packages examined),
and argparse's 2 for a usage error.
"""

from __future__ import annotations

import argparse
import json
import sys

sys.dont_write_bytecode = True

#: pnpm's advisory grades, lowest first. `info` is below every level `--audit-level` takes.
GRADES = ("info", "low", "moderate", "high", "critical")
#: The dependency classes a report's metadata counts, printed beside its total.
CLASSES = ("dependencies", "devDependencies", "optionalDependencies")

EXIT_OK, EXIT_FAILED, EXIT_VOID = 0, 1, 3


def examined_count(report: object) -> int | None:
    """The packages a report examined, or None when it holds no whole-number count."""
    metadata = report.get("metadata") if isinstance(report, dict) else None
    total = metadata.get("totalDependencies") if isinstance(metadata, dict) else None
    if isinstance(total, bool) or not isinstance(total, int) or total < 0:
        return None
    return total


def failing(advisory: object, level: str) -> bool:
    """Whether an advisory fails the stage: its grade is at or above the level, or is none of
    pnpm's, because a grade the verdict cannot place is never read as clean."""
    severity = advisory.get("severity") if isinstance(advisory, dict) else None
    if severity not in GRADES:
        return True
    return GRADES.index(severity) >= GRADES.index(level)


def describe(advisory: object) -> str:
    """One line naming an advisory: its severity, package, versions found, id and range."""
    fields = advisory if isinstance(advisory, dict) else {}
    findings = fields.get("findings")
    versions = sorted(
        {str(found.get("version")) for found in findings if isinstance(found, dict)}
        if isinstance(findings, list)
        else set()
    )
    ident = fields.get("github_advisory_id") or fields.get("id")
    return (
        f"{fields.get('severity')} {fields.get('module_name')} {','.join(versions)} {ident} "
        f"(vulnerable {fields.get('vulnerable_versions')}): {fields.get('title')}"
    )


def verdict(text: str, pnpm_exit: int, level: str) -> tuple[int, list[str]]:
    """The stage's exit and the lines it prints, for pnpm's report `text` and its exit."""
    try:
        report = json.loads(text)
    except ValueError:
        report = None
    total = examined_count(report)
    advisories = report.get("advisories") if isinstance(report, dict) else None
    if total is None or not isinstance(advisories, dict):
        first = next((line.strip() for line in text.splitlines() if line.strip()), "no output")
        return EXIT_VOID, [
            "audit-web: VOID: pnpm audit gave no report with a package count "
            f"(exit {pnpm_exit}): {first[:120]}"
        ]
    if total == 0:
        return EXIT_VOID, ["audit-web: VOID: examined 0 package(s), so nothing was judged"]
    found = [advisory for advisory in advisories.values() if failing(advisory, level)]
    counts = ", ".join(f"{name} {report['metadata'].get(name)}" for name in CLASSES)
    line = (
        f"audit-web: examined {total} package(s) ({counts}), "
        f"advisories at or above {level}: {len(found)}"
    )
    if found:
        return EXIT_FAILED, [*(f"audit-web: {describe(advisory)}" for advisory in found), line]
    if pnpm_exit != 0:
        return EXIT_FAILED, [f"{line}, but pnpm audit exited {pnpm_exit}"]
    return EXIT_OK, [line]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--level", required=True, choices=GRADES[1:])
    parser.add_argument("--pnpm-exit", required=True, type=int)
    args = parser.parse_args(argv)
    code, lines = verdict(sys.stdin.read(), args.pnpm_exit, args.level)
    for line in lines:
        print(line)
    return code


if __name__ == "__main__":
    sys.exit(main())

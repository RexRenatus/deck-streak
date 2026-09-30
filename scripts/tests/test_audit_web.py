"""The gate's audit-web stage audits every dependency the repository resolves, the development ones
included, prints how many packages it examined, reads VOID when that is none, and fails on any
advisory at or above its level (SPEC-058 A1 to A4). Every case runs check.sh's own stage with a
pnpm that records its arguments, prints a planted report and exits with a planted code, so no case
asks the registry. The reports are synthetic, in pnpm 11's JSON shape."""

import json
import os
import runpy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_check_gate import run_gate, summary

# The level SPEC-058 R2 states, and the one pnpm call R1 allows: no flag that narrows the
# dependency classes, the JSON report that holds the count, and the level on the command line.
LEVEL = "low"
CALL = ["audit", "--json", "--audit-level", LEVEL]
# node and pnpm are stubs; python3 runs the verdict, and cat serves the planted report.
REAL = ("python3", "cat")
PNPM = 'printf \'%s\\n\' "$@" > "$PNPM_CALL"\ncat "$PNPM_REPORT"\nexit "$PNPM_EXIT"\n'
# The verdict line of a planted report's default counts: pnpm's total and its three classes.
EXAMINED_428 = (
    "audit-web: examined 428 package(s) (dependencies 0, devDependencies 428, "
    "optionalDependencies 87), advisories at or above low"
)


def report(total=428, dependencies=0, dev=428, optional=87, advisories=()):
    """A synthetic report in the shape `pnpm audit --json` prints."""
    return json.dumps(
        {
            "advisories": {str(1000 + at): planted for at, planted in enumerate(advisories)},
            "metadata": {
                "vulnerabilities": {},
                "dependencies": dependencies,
                "devDependencies": dev,
                "optionalDependencies": optional,
                "totalDependencies": total,
            },
        },
        indent=2,
    )


def advisory(severity, name="planted-package", versions=("1.0.0",)):
    """A synthetic advisory of one grade against a planted package, found at each version given
    (with no findings at all when `versions` is None)."""
    planted = {
        "module_name": name,
        "severity": severity,
        "github_advisory_id": "GHSA-0000-0000-0001",
        "vulnerable_versions": "<2.0.0",
        "title": "a planted advisory",
    }
    if versions is not None:
        planted["findings"] = [
            {"version": version, "paths": [f"web__app>{name}"], "dev": True} for version in versions
        ]
    return planted


def named(severity, name="planted-package", versions="1.0.0"):
    """The line the stage's log names a failing planted advisory with."""
    planted = "GHSA-0000-0000-0001 (vulnerable <2.0.0): a planted advisory"
    return f"audit-web: {severity} {name} {versions} {planted}"


def audit(scratch, text, code):
    """Run check.sh's audit-web stage with a pnpm that prints `text` and exits `code`. Returns the
    process, the arguments pnpm was given (None when it never ran) and the stage's log lines."""
    where = Path(scratch)
    planted = where / "report.json"
    planted.write_text(text, encoding="utf-8")
    call = where / "pnpm-call"
    env = {"PNPM_CALL": str(call), "PNPM_REPORT": str(planted), "PNPM_EXIT": str(code)}
    done, logs = run_gate(
        where / "gate", ["audit-web"], ["node", "pnpm"], env, real=REAL, bodies={"pnpm": PNPM}
    )
    recorded = call.read_text(encoding="utf-8").splitlines() if call.exists() else None
    log = (logs / "audit-web.log").read_text(encoding="utf-8")
    return done, recorded, [line for line in log.splitlines() if line.strip()]


class TheWebAuditCoversEveryDependency(unittest.TestCase):
    def test_the_web_audit_covers_the_development_dependencies_too(self):
        with tempfile.TemporaryDirectory() as scratch:
            done, call, _ = audit(scratch, report(), 0)
        # One call that names no dependency class, so pnpm audits the dependencies,
        # devDependencies and optionalDependencies alike, at the level the gate states.
        self.assertEqual(call, CALL, done.stdout)


# A run that examined nothing: what pnpm printed, its exit, and the VOID line the stage ends with.
NO_REPORT = "audit-web: VOID: pnpm audit gave no report with a package count"
EMPTY = [
    (
        "a report that examined 0 packages",
        report(total=0, dependencies=0, dev=0, optional=0),
        0,
        "audit-web: VOID: examined 0 package(s), so nothing was judged",
    ),
    (
        "a registry error in place of a report, after a blank line",
        "\n ERR_PNPM_AUDIT_BAD_RESPONSE  The audit endpoint responded with 503\n",
        1,
        f"{NO_REPORT} (exit 1): ERR_PNPM_AUDIT_BAD_RESPONSE  The audit endpoint responded with 503",
    ),
    ("no output at all", "", 1, f"{NO_REPORT} (exit 1): no output"),
    (
        "a JSON error with no metadata",
        json.dumps({"error": {"code": "ERR_PNPM_AUDIT_BAD_RESPONSE"}}),
        1,
        f'{NO_REPORT} (exit 1): {{"error": {{"code": "ERR_PNPM_AUDIT_BAD_RESPONSE"}}}}',
    ),
    (
        "a report with no advisories object",
        json.dumps({"metadata": {"totalDependencies": 428}}),
        0,
        f'{NO_REPORT} (exit 0): {{"metadata": {{"totalDependencies": 428}}}}',
    ),
    ("a count written as text", report(total="428"), 0, f"{NO_REPORT} (exit 0): {{"),
    ("a negative count", report(total=-1), 0, f"{NO_REPORT} (exit 0): {{"),
    ("a count that is a boolean", report(total=True), 0, f"{NO_REPORT} (exit 0): {{"),
]


class TheWebAuditRefusesAnEmptyRun(unittest.TestCase):
    def test_a_web_audit_that_examined_nothing_is_void(self):
        for name, text, code, void in examined("runs that examined nothing", EMPTY):
            with self.subTest(case=name), tempfile.TemporaryDirectory() as scratch:
                done, _, log = audit(scratch, text, code)
                self.assertRegex(
                    summary(done), r"^FAILED +audit-web +\d+s exit 3: audit-web: VOID: "
                )
                self.assertEqual(log[-1:], [void], done.stdout)
                self.assertEqual(done.returncode, 1, done.stdout)


# Each case: its planted advisories, pnpm's exit, the stage's exit, and the lines its log ends with,
# the advisories it names and then the verdict.
ADVISORIES = [
    (
        "an advisory at the level, as pnpm reports it",
        [advisory("low")],
        1,
        1,
        [named("low"), f"{EXAMINED_428}: 1"],
    ),
    (
        "an advisory above the level, beside one below it",
        [advisory("info", "quiet-package"), advisory("critical")],
        1,
        1,
        [named("critical"), f"{EXAMINED_428}: 1"],
    ),
    (
        "an advisory the report holds although pnpm exited 0",
        [advisory("moderate")],
        0,
        1,
        [named("moderate"), f"{EXAMINED_428}: 1"],
    ),
    (
        "an advisory of none of pnpm's grades, with no findings",
        [advisory("unknown", versions=None)],
        0,
        1,
        [named("unknown", versions=""), f"{EXAMINED_428}: 1"],
    ),
    (
        "every failing advisory, each named at every version found",
        [
            advisory("high", "first-package", versions=("1.0.0", "2.0.0")),
            advisory("low", "second-package"),
        ],
        1,
        1,
        [
            named("high", "first-package", versions="1.0.0,2.0.0"),
            named("low", "second-package"),
            f"{EXAMINED_428}: 2",
        ],
    ),
    (
        "an advisory below the level only",
        [advisory("info")],
        0,
        0,
        [f"{EXAMINED_428}: 0"],
    ),
    (
        "a pnpm that exited non-zero with no failing advisory",
        [],
        1,
        1,
        [f"{EXAMINED_428}: 0, but pnpm audit exited 1"],
    ),
]
# One report holding an advisory of each of pnpm's grades, lowest first, and the grades the verdict
# names and fails at each level `--audit-level` takes: it judges at the level it is given.
GRADED = [advisory(grade, f"{grade}-package") for grade in ("info", "low", "moderate", "high")]
GRADED.append(advisory("critical", "critical-package"))
AT_OR_ABOVE = {
    "low": ["low", "moderate", "high", "critical"],
    "moderate": ["moderate", "high", "critical"],
    "high": ["high", "critical"],
    "critical": ["critical"],
}
VERDICT = REPO / "scripts" / "audit-web-verdict.py"
USAGE_ERRORS = [["--level", "info", "--pnpm-exit", "0"], ["--pnpm-exit", "0"], ["--level", "low"]]


class TheWebAuditFailsOnAnAdvisory(unittest.TestCase):
    def test_an_advisory_at_the_failing_level_fails_the_web_audit(self):
        for name, planted, code, verdict, lines in examined("advisory cases", ADVISORIES):
            with self.subTest(case=name), tempfile.TemporaryDirectory() as scratch:
                done, _, log = audit(scratch, report(advisories=planted), code)
                self.assertEqual(log[-len(lines) :], lines, done.stdout)
                if verdict:
                    self.assertRegex(summary(done), rf"^FAILED +audit-web +\d+s exit {verdict}: ")
                    self.assertEqual(done.returncode, 1, done.stdout)
                else:
                    self.assertRegex(summary(done), r"^ok +audit-web ")
                    self.assertEqual(done.returncode, 0, done.stdout)
        for level, grades in examined("levels", list(AT_OR_ABOVE.items())):
            with self.subTest(level=level):
                done = subprocess.run(
                    [sys.executable, str(VERDICT), "--level", level, "--pnpm-exit", "1"],
                    input=report(advisories=GRADED),
                    capture_output=True,
                    text=True,
                    check=False,
                )
                lines = done.stdout.splitlines()
                self.assertEqual([line.split()[1] for line in lines[:-1]], grades, done.stdout)
                verdict_line = EXAMINED_428.replace("above low", f"above {level}")
                self.assertEqual(lines[-1:], [f"{verdict_line}: {len(grades)}"], done.stderr)
                self.assertEqual(done.returncode, 1, done.stderr)
        # A level `--audit-level` does not take, and a missing option, are usage errors.
        for argv in examined("usage errors", USAGE_ERRORS):
            with self.subTest(argv=argv):
                done = subprocess.run(
                    [sys.executable, str(VERDICT), *argv],
                    input=report(),
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(done.returncode, 2, done.stdout)
                self.assertIn("usage: ", done.stderr)


class TheWebAuditPrintsItsCount(unittest.TestCase):
    def test_a_clean_web_audit_passes_and_prints_its_examined_count(self):
        with tempfile.TemporaryDirectory() as scratch:
            clean = report(total=5, dependencies=2, dev=3, optional=1)
            done, _, log = audit(scratch, clean, 0)
        self.assertEqual(
            log[-1:],
            [
                "audit-web: examined 5 package(s) (dependencies 2, devDependencies 3, "
                "optionalDependencies 1), advisories at or above low: 0"
            ],
            done.stdout,
        )
        self.assertRegex(summary(done), r"^ok +audit-web ")
        self.assertIn("CHECK OK: 1 stage(s)", done.stdout)
        self.assertEqual(done.returncode, 0, done.stdout)


class TheVerdictScriptStatesItself(unittest.TestCase):
    """What only the script's own bytes and its usage text show: the two mutants of
    `scripts/audit-web-verdict.py` that the stage's runs cannot tell apart."""

    SCRIPT = REPO / "scripts" / "audit-web-verdict.py"

    def test_the_verdict_script_writes_no_bytecode(self):
        kept = sys.dont_write_bytecode
        sys.dont_write_bytecode = False
        try:
            runpy.run_path(str(self.SCRIPT), run_name="audit_web_verdict_probe")
            written = sys.dont_write_bytecode
        finally:
            sys.dont_write_bytecode = kept
        self.assertIs(written, True)

    def test_the_usage_text_opens_with_the_scripts_first_line(self):
        done = subprocess.run(
            ["python3", str(self.SCRIPT), "--help"],
            capture_output=True,
            text=True,
            check=False,
            env={"PATH": os.environ.get("PATH", ""), "COLUMNS": "200"},
        )
        self.assertEqual(done.returncode, 0, done.stderr)
        first = self.SCRIPT.read_text(encoding="utf-8").split('"""', 2)[1].splitlines()[0]
        self.assertIn(first, done.stdout)
        self.assertIn("audit-web-verdict: the gate's web audit", done.stdout)


if __name__ == "__main__":
    unittest.main()

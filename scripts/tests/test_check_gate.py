"""The local gate checks each stage's own tools, has no toolchain stage, splits its audit by
toolchain, and times every stage it runs (SPEC-038 A5, A6 and A12)."""

import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

CHECK = REPO / "scripts" / "check.sh"
STAGES_ALL = re.compile(r"^STAGES_ALL=\(([^)]*)\)", re.M)
STAGE_FUNCTION = re.compile(r"^stage_([a-z0-9_]+)\(\)", re.M)
# What check.sh itself runs around its stages. A test's PATH holds these and its stubs, nothing else.
SHELL_TOOLS = ("dirname", "mktemp", "mkdir", "date", "grep", "tail")
# Every stage, and each tool it runs, in the order it checks them (SPEC-038 R4).
TOOLS = {
    "fmt": ["cargo"],
    "clippy": ["cargo"],
    "test": ["cargo", "cargo-nextest"],
    "doctest": ["cargo"],
    "audit-rust": ["cargo", "cargo-deny"],
    "web": ["node", "pnpm"],
    "audit-web": ["node", "pnpm"],
    # A guard test builds a Rust example (SPEC-042's rails rows), so the stage runs cargo too.
    "python": ["python3", "cargo"],
    "packs": ["python3"],
    "scrub": ["python3"],
    "secrets": ["gitleaks"],
}
SUMMARY = re.compile(r"^(ok|FAILED) +\S+ +\d+s")


def run_gate(scratch, stages, stubs):
    """Run check.sh for `stages` with a PATH that holds only the shell tools check.sh needs and, for
    each name in `stubs`, a stub that exits 0. Returns the process and its log directory."""
    tools = scratch / "bin"
    tools.mkdir()
    for tool in SHELL_TOOLS:
        found = shutil.which(tool)
        if found is None:
            raise AssertionError(f"this machine has no {tool}, which check.sh itself runs")
        (tools / tool).symlink_to(found)
    for tool in stubs:
        stub = tools / tool
        stub.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        stub.chmod(0o755)
    logs = scratch / "logs"
    env = {"PATH": str(tools), "CHECK_LOG_DIR": str(logs), "HOME": str(scratch)}
    done = subprocess.run(
        [shutil.which("bash"), str(CHECK), *stages],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    return done, logs


def summary(done):
    return next((line for line in done.stdout.splitlines() if SUMMARY.match(line)), "")


class EachStageChecksItsOwnTools(unittest.TestCase):
    def test_every_stage_fails_by_name_without_each_tool_it_runs(self):
        cases = [
            (stage, tools[:at], tool)
            for stage, tools in TOOLS.items()
            for at, tool in enumerate(tools)
        ]
        for stage, present, missing in examined("stage and missing-tool cases", cases):
            with self.subTest(stage=stage, missing=missing):
                with tempfile.TemporaryDirectory() as scratch:
                    done, _ = run_gate(Path(scratch), [stage], present)
                named = (
                    rf"^FAILED +{re.escape(stage)} .*: missing tool: {re.escape(missing)} \(\S.*\)$"
                )
                self.assertRegex(summary(done), named, done.stdout + done.stderr)
                self.assertEqual(done.returncode, 1, done.stdout)

    def test_the_gate_has_no_toolchain_stage_and_splits_the_audit(self):
        text = CHECK.read_text(encoding="utf-8")
        listed = examined("gate stages", STAGES_ALL.search(text).group(1).split())
        self.assertEqual(sorted(listed), sorted(TOOLS))
        defined = [name.replace("_", "-") for name in STAGE_FUNCTION.findall(text)]
        self.assertEqual(sorted(defined), sorted(listed), "a stage function outside STAGES_ALL")
        with tempfile.TemporaryDirectory() as scratch:
            done, _ = run_gate(Path(scratch), ["toolchain"], [])
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn("check.sh: unknown stage 'toolchain'", done.stdout)


class EveryStageIsTimed(unittest.TestCase):
    def test_each_stage_run_is_timed_in_timings_tsv(self):
        with tempfile.TemporaryDirectory() as scratch:
            done, logs = run_gate(Path(scratch), ["fmt", "web"], ["cargo"])
            timings = logs / "timings.tsv"
            self.assertTrue(timings.is_file(), f"no {timings.name} in the log directory")
            rows = [line.split("\t") for line in timings.read_text(encoding="utf-8").splitlines()]
        self.assertEqual(rows[0], ["stage", "seconds", "verdict", "exit"])
        body = examined("timed stage runs", rows[1:])
        self.assertEqual(
            [(r[0], r[2], r[3]) for r in body], [("fmt", "ok", "0"), ("web", "FAILED", "1")]
        )
        for row in body:
            self.assertRegex(row[1], r"^\d+$", f"{row[0]} was timed as {row[1]!r}")
        self.assertEqual(done.returncode, 1, done.stdout)


if __name__ == "__main__":
    unittest.main()

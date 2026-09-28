"""The local gate checks each stage's own tools, has no toolchain stage, splits its audit by
toolchain, times every stage it runs, and makes the stages that compile Anki's engine name protoc
(SPEC-038 A5, A6, A12 and A15). Its python stage runs every suite, whatever an earlier one found,
and names each one's result (SPEC-054 A13)."""

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
# Every stage, and each tool it runs, in the order it checks them (SPEC-038 R4). protoc, which the
# engine's build runs from PROTOC or PATH, is judged on its own below (A15).
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
# The stages that compile Anki's engine, and the tools each checks before protoc (SPEC-038 R4): the
# engine's build scripts compile its protobuf definitions with prost-build (ADR-022).
ENGINE_STAGES = {"clippy": ["cargo"], "test": ["cargo", "cargo-nextest"], "doctest": ["cargo"]}
# A planted guard test that fails, and a planted oracle suite of two passing tests (SPEC-054 A13).
PLANTED_GUARD = (
    "import unittest\n\n\n"
    "class PlantedGuard(unittest.TestCase):\n"
    "    def test_a_planted_guard_fails(self):\n"
    "        self.assertEqual(1, 2)\n"
)
PLANTED_ORACLE = (
    "import unittest\n\n\n"
    "class PlantedOracle(unittest.TestCase):\n"
    "    def test_one(self):\n"
    "        self.assertEqual(1, 1)\n\n"
    "    def test_two(self):\n"
    "        self.assertEqual(2, 2)\n"
)


def run_gate(scratch, stages, stubs, extra_env=None, check=CHECK, real=()):
    """Run `check` (the repository's check.sh by default) for `stages` with a PATH that holds only
    the shell tools check.sh needs, each tool `real` names as the machine has it, and, for each name
    in `stubs`, a stub that exits 0, and any `extra_env`. Returns the process and its log
    directory."""
    tools = scratch / "bin"
    tools.mkdir(parents=True)
    for tool in (*SHELL_TOOLS, *real):
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
    env.update(extra_env or {})
    done = subprocess.run(
        [shutil.which("bash"), str(check), *stages],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    return done, logs


def python_tree(where, guard, oracle):
    """A copy of check.sh in a tree with its two python suites, each holding the one test file
    given, or none for None. Returns the copy's path."""
    tree = where / "tree"
    for suite, text in (("scripts/tests", guard), ("tools/parity-oracle", oracle)):
        (tree / suite).mkdir(parents=True)
        if text is not None:
            (tree / suite / "test_planted.py").write_text(text, encoding="utf-8")
    shutil.copy(CHECK, tree / "scripts" / "check.sh")
    return tree / "scripts" / "check.sh"


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


class TheEngineStagesNeedProtoc(unittest.TestCase):
    def test_the_stages_that_compile_the_engine_need_protoc(self):
        stages = examined("stages that compile the engine", list(ENGINE_STAGES.items()))
        for stage, before in stages:
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as scratch:
                where = Path(scratch)
                # No protoc on PATH and no PROTOC: the stage fails by name, with its install hint.
                done, _ = run_gate(where / "missing", [stage], before)
                missing = rf"^FAILED +{stage} .*: missing tool: protoc \(\S.*\)$"
                self.assertRegex(summary(done), missing, done.stdout)
                # A PROTOC that names no executable fails the stage too.
                absent = {"PROTOC": str(where / "no-such-protoc")}
                done, _ = run_gate(where / "absent", [stage], before, absent)
                named = rf"^FAILED +{stage} .*: PROTOC names no executable protoc$"
                self.assertRegex(summary(done), named, done.stdout)
                # A PROTOC that names an executable stands in for protoc on PATH.
                protoc = where / "protoc-elsewhere"
                protoc.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                protoc.chmod(0o755)
                done, _ = run_gate(where / "named", [stage], before, {"PROTOC": str(protoc)})
                self.assertRegex(summary(done), rf"^ok +{stage} ", done.stdout)


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


class ThePythonStageRunsEverySuite(unittest.TestCase):
    def test_a_red_guard_suite_leaves_the_oracle_suite_run_and_named(self):
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            check = python_tree(where, PLANTED_GUARD, PLANTED_ORACLE)
            done, logs = run_gate(where, ["python"], ["cargo"], check=check, real=["python3"])
            log = (logs / "python.log").read_text(encoding="utf-8")
        self.assertEqual(done.returncode, 1, done.stdout)
        # The oracle suite ran although the guard suite was red, and its result is in the log.
        self.assertIn("Ran 2 tests", log)
        self.assertRegex(
            summary(done),
            r"^FAILED +python .*: python: scripts/tests ran 1 test\(s\), exit 1; "
            r"tools/parity-oracle ran 2 test\(s\), exit 0$",
        )
        # A suite that runs no test fails the stage by name, whatever its exit.
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            check = python_tree(where, PLANTED_ORACLE, None)
            done, _ = run_gate(where, ["python"], ["cargo"], check=check, real=["python3"])
        self.assertRegex(summary(done), r"^FAILED +python .*tools/parity-oracle ran 0 test\(s\)")
        self.assertEqual(done.returncode, 1, done.stdout)
        # Both suites green, the stage is green.
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            check = python_tree(where, PLANTED_ORACLE, PLANTED_ORACLE)
            done, _ = run_gate(where, ["python"], ["cargo"], check=check, real=["python3"])
        self.assertRegex(summary(done), r"^ok +python ")
        self.assertEqual(done.returncode, 0, done.stdout)


if __name__ == "__main__":
    unittest.main()

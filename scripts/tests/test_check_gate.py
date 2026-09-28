"""The local gate checks each stage's own tools, has no toolchain stage, splits its audit by
toolchain, times every stage it runs, and makes the stages that compile Anki's engine name protoc
(SPEC-038 A5, A6, A12 and A15). Its python stage runs every suite, whatever an earlier one found,
and names each one's result (SPEC-054 A13). Its two test stages split the workspace's tests by one
engine set, defined once, and test-engine runs the slice of it CI hands over (SPEC-038 A16, A17 and
A19)."""

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
    # The engine set's own stage (SPEC-038 R13), which the engine job runs.
    "test-engine": ["cargo", "cargo-nextest"],
    "web": ["node", "pnpm"],
    "audit-web": ["node", "pnpm"],
    # A guard test builds a Rust example (SPEC-042's rails rows), so the stage runs cargo too.
    "python": ["python3", "cargo"],
    "scrub": ["python3"],
    "secrets": ["gitleaks"],
}
SUMMARY = re.compile(r"^(ok|FAILED) +\S+ +\d+s")
# The stages that compile Anki's engine, and the tools each checks before protoc (SPEC-038 R4): the
# engine's build scripts compile its protobuf definitions with prost-build (ADR-022).
ENGINE_STAGES = {
    "clippy": ["cargo"],
    "test": ["cargo", "cargo-nextest"],
    "doctest": ["cargo"],
    "test-engine": ["cargo", "cargo-nextest"],
}
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
# A python3 whose unittest runs nothing and exits 0, as Python 3.11's does; any other call passes.
EMPTY_UNITTEST = (
    "case \"$*\" in *unittest*) printf 'Ran 0 tests in 0.000s\\n\\nOK\\n' ;; esac\nexit 0\n"
)


def run_gate(scratch, stages, stubs, extra_env=None, check=CHECK, real=(), bodies=None):
    """Run `check` (the repository's check.sh by default) for `stages` with a PATH that holds only
    the shell tools check.sh needs, each tool `real` names as the machine has it, and, for each name
    in `stubs`, a stub that exits 0 (or runs the shell body `bodies` gives it), and any
    `extra_env`. Returns the process and its log directory."""
    tools = scratch / "bin"
    tools.mkdir(parents=True)
    for tool in (*SHELL_TOOLS, *real):
        found = shutil.which(tool)
        if found is None:
            raise AssertionError(f"this machine has no {tool}, which check.sh itself runs")
        (tools / tool).symlink_to(found)
    for tool in stubs:
        stub = tools / tool
        stub.write_text("#!/bin/sh\n" + (bodies or {}).get(tool, "exit 0\n"), encoding="utf-8")
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


class TheLogDirectoryStaysOffStdout(unittest.TestCase):
    """SPEC-056 R4: stdout, which a reader quotes, never names the machine's log directory."""

    def test_the_log_directory_is_never_on_stdout_and_holds_every_stage_log(self):
        with tempfile.TemporaryDirectory() as scratch:
            # A caller that names the directory, as every CI job does: nothing names it, and it
            # holds each stage's log and the timings, which the job's upload step reads.
            named = Path(scratch) / "named"
            done, logs = run_gate(named, ["fmt", "web"], ["cargo"])
            self.assertNotIn(str(logs), done.stdout)
            self.assertNotIn(str(logs), done.stderr)
            self.assertEqual([line for line in done.stdout.splitlines() if "logs" in line], [])
            held = examined("files in the named log directory", sorted(logs.iterdir()))
            self.assertEqual([path.name for path in held], ["fmt.log", "timings.tsv", "web.log"])
            self.assertTrue(summary(done), done.stdout)
            # A caller that names none: the fresh directory is named once, on stderr alone.
            fresh = Path(scratch) / "fresh"
            temp = fresh / "tmp"
            temp.mkdir(parents=True)
            done, _ = run_gate(
                fresh, ["fmt"], ["cargo"], extra_env={"CHECK_LOG_DIR": "", "TMPDIR": str(temp)}
            )
            made = examined("fresh log directories", sorted(temp.iterdir()))
            self.assertEqual(len(made), 1, made)
            self.assertNotIn(str(made[0]), done.stdout)
            self.assertEqual(done.stderr.splitlines(), [f"logs: {made[0]}"])
            self.assertEqual(
                sorted(path.name for path in made[0].iterdir()), ["fmt.log", "timings.tsv"]
            )
            self.assertIn("CHECK OK: 1 stage(s)", done.stdout)


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
        # So does a suite whose unittest exits 0 having run nothing, as Python 3.11's does.
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            check = python_tree(where, PLANTED_ORACLE, PLANTED_ORACLE)
            bodies = {"python3": EMPTY_UNITTEST}
            done, _ = run_gate(where, ["python"], ["cargo", "python3"], check=check, bodies=bodies)
        self.assertRegex(summary(done), r"^FAILED +python .*scripts/tests ran 0 test\(s\), exit 0;")
        self.assertEqual(done.returncode, 1, done.stdout)
        # Both suites green, the stage is green.
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            check = python_tree(where, PLANTED_ORACLE, PLANTED_ORACLE)
            done, _ = run_gate(where, ["python"], ["cargo"], check=check, real=["python3"])
        self.assertRegex(summary(done), r"^ok +python ")
        self.assertEqual(done.returncode, 0, done.stdout)


# --------------------------------------------------------- the engine set (SPEC-038 A16, A17)

# The engine set's one definition (SPEC-038 R13): a nextest filterset, in single quotes.
ENGINE_DEFINITION = re.compile(r"^ENGINE_TESTS='([^'\n]*)'$", re.M)
# The two test stages and the filterset each must run: every test outside the set, and the set.
SPLIT = {"test": "not ({})", "test-engine": "{}"}
# A set no real test is in, written in the definition's place to see which stages follow it.
PLANTED_SET = "test(=a_planted_engine_test)"
# The flags that carry a filterset to nextest.
FILTERSET_FLAGS = ("-E", "--filterset")
# What the engine set is made of: whole test binaries, each `binary_id(=<package>::<target>)`.
ENGINE_BINARY = re.compile(r"binary_id\(=([a-z0-9-]+)::([a-z0-9_]+)\)")
WHOLE_BINARIES = re.compile(rf"{ENGINE_BINARY.pattern}(?: \| {ENGINE_BINARY.pattern})*")
RUST_TEST = re.compile(r"(?m)^\s*#\[(?:tokio::)?test\b")
# A test function: its test attribute, any attributes after it, then `fn <name>`.
TEST_FUNCTION = re.compile(
    r"(?m)^[ \t]*#\[(?:tokio::)?test\b[^\n]*\n(?:[ \t]*#\[[^\n]*\n)*[ \t]*(?:async[ \t]+)?fn[ \t]+"
    r"([a-z0-9_]+)"
)
PACKAGE_NAME = re.compile(r'(?m)^name = "([^"]+)"$')
CALL_END = "--end-of-cargo-call--"


def run_recorded(check, stage, scratch, extra_env=None):
    """Run `check`, the text of a check.sh, as scripts/check.sh of a scratch tree for one stage,
    with a PATH of check.sh's shell tools, a cargo that records its arguments and exits 0, stubs
    for cargo-nextest and protoc, and any `extra_env`. Returns the process and cargo's calls, each
    a list."""
    tree = scratch / "tree"
    (tree / "scripts").mkdir(parents=True)
    (tree / "scripts" / "check.sh").write_text(check, encoding="utf-8")
    tools = scratch / "bin"
    tools.mkdir()
    for tool in SHELL_TOOLS:
        found = shutil.which(tool)
        if found is None:
            raise AssertionError(f"this machine has no {tool}, which check.sh itself runs")
        (tools / tool).symlink_to(found)
    stubs = {
        "cargo": f"#!/bin/sh\nprintf '%s\\n' \"$@\" '{CALL_END}' >> \"$CARGO_CALLS\"\nexit 0\n",
        "cargo-nextest": "#!/bin/sh\nexit 0\n",
        "protoc": "#!/bin/sh\nexit 0\n",
    }
    for tool, body in stubs.items():
        (tools / tool).write_text(body, encoding="utf-8")
        (tools / tool).chmod(0o755)
    record = scratch / "cargo-calls"
    record.write_text("", encoding="utf-8")
    env = {
        "PATH": str(tools),
        "CHECK_LOG_DIR": str(scratch / "logs"),
        "HOME": str(scratch),
        "CARGO_CALLS": str(record),
    }
    env.update(extra_env or {})
    done = subprocess.run(
        [shutil.which("bash"), str(tree / "scripts" / "check.sh"), stage],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    calls, call = [], []
    for line in record.read_text(encoding="utf-8").splitlines():
        if line == CALL_END:
            calls.append(call)
            call = []
        else:
            call.append(line)
    return done, calls


def filtersets(argv):
    """The filtersets a cargo call hands nextest, and the call without them."""
    found, rest, at = [], [], 0
    while at < len(argv):
        if argv[at] in FILTERSET_FLAGS and at + 1 < len(argv):
            found.append(argv[at + 1])
            at += 2
        else:
            rest.append(argv[at])
            at += 1
    return found, rest


def split_problems(check, scratch):
    """What a check.sh gets wrong about the engine set (SPEC-038 R13): a set defined other than
    once, a test stage that does not run it (negated in `test`, as it is in `test-engine`), a stage
    that holds its own copy of it, or two commands that differ in more than the filterset. It runs
    both stages twice, with the definition as written and with PLANTED_SET in its place, so a stage
    that states the set itself instead of reading the definition stays behind and is seen."""
    definitions = ENGINE_DEFINITION.findall(check)
    if len(definitions) != 1:
        return [f"check.sh defines the engine set {len(definitions)} time(s), not once"]
    planted = ENGINE_DEFINITION.sub(lambda _: f"ENGINE_TESTS='{PLANTED_SET}'", check)
    problems = []
    for label, text, engine in (
        ("as written", check, definitions[0]),
        ("planted", planted, PLANTED_SET),
    ):
        commands = {}
        for stage, form in SPLIT.items():
            done, calls = run_recorded(text, stage, scratch / label.replace(" ", "-") / stage)
            if done.returncode != 0 or len(calls) != 1:
                said = (summary(done) or done.stdout.strip() or done.stderr.strip())[-160:]
                problems.append(
                    f"{stage} ({label}): exit {done.returncode}, {len(calls)} cargo call(s): {said}"
                )
                continue
            found, commands[stage] = filtersets(calls[0])
            wanted = form.format(engine)
            if found != [wanted]:
                problems.append(f"{stage} ({label}) runs the filterset {found}, not [{wanted!r}]")
        if len(commands) == len(SPLIT) and len({tuple(c) for c in commands.values()}) != 1:
            problems.append(
                f"the two stages ({label}) differ in more than the filterset: {commands}"
            )
    return problems


def planted_check(test_stage, engine_stage, definitions=("binary_id(=p::slow)",)):
    """A small check.sh: the engine set's definitions, and the two test stages as given."""
    lines = ["#!/usr/bin/env bash"]
    lines += [f"ENGINE_TESTS='{definition}'" for definition in definitions]
    lines += [f"stage_test() {{ {test_stage}; }}", f"stage_test_engine() {{ {engine_stage}; }}"]
    lines += ['"stage_${1//-/_}"']
    return "\n".join(lines) + "\n"


NEGATED = 'cargo nextest run --workspace -E "not ($ENGINE_TESTS)"'
AS_IS = 'cargo nextest run --workspace -E "$ENGINE_TESTS"'
# Each planted check.sh, and what split_problems must say about it.
PLANTED_SPLITS = [
    (
        "a stage that holds its own copy of the set",
        planted_check(NEGATED, "cargo nextest run --workspace -E 'binary_id(=p::slow)'"),
        [
            "test-engine (planted) runs the filterset ['binary_id(=p::slow)'], "
            "not ['test(=a_planted_engine_test)']"
        ],
    ),
    (
        "a test stage that runs the set instead of its complement",
        planted_check(AS_IS, AS_IS),
        [
            "test (as written) runs the filterset ['binary_id(=p::slow)'], "
            "not ['not (binary_id(=p::slow))']",
            "test (planted) runs the filterset ['test(=a_planted_engine_test)'], "
            "not ['not (test(=a_planted_engine_test))']",
        ],
    ),
    (
        "an engine stage that builds another scope",
        planted_check(NEGATED, 'cargo nextest run --package p -E "$ENGINE_TESTS"'),
        [
            "the two stages (as written) differ in more than the filterset: "
            "{'test': ['nextest', 'run', '--workspace'], "
            "'test-engine': ['nextest', 'run', '--package', 'p']}",
            "the two stages (planted) differ in more than the filterset: "
            "{'test': ['nextest', 'run', '--workspace'], "
            "'test-engine': ['nextest', 'run', '--package', 'p']}",
        ],
    ),
    (
        "a set defined twice",
        planted_check(NEGATED, AS_IS, ("binary_id(=p::slow)", "binary_id(=p::slower)")),
        ["check.sh defines the engine set 2 time(s), not once"],
    ),
]


def workspace_packages(root=REPO):
    """{package name: crate directory} for every crate under crates/."""
    packages = {}
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        named = PACKAGE_NAME.search(manifest.read_text(encoding="utf-8"))
        if named:
            packages[named.group(1)] = manifest.parent
    return packages


def engine_binary_problems(engine, root=REPO):
    """What is wrong with an engine set as a list of whole test binaries: a set made of anything
    but `binary_id(=<package>::<target>)` terms joined by `|`, a package no crate under
    crates/ declares, or a target whose tests/<target>.rs holds no test."""
    if not WHOLE_BINARIES.fullmatch(engine):
        return [f"the engine set is not a union of whole test binaries: {engine}"]
    packages = workspace_packages(root)
    problems = []
    for package, target in ENGINE_BINARY.findall(engine):
        if package not in packages:
            problems.append(f"{package}::{target}: no crate declares the package {package}")
            continue
        source = packages[package] / "tests" / f"{target}.rs"
        if not source.is_file() or not RUST_TEST.search(source.read_text(encoding="utf-8")):
            problems.append(f"{package}::{target}: no test target with tests at {source.name}")
    return problems


def engine_tests(engine, root=REPO):
    """[(binary id, test function)] for every test the engine set's binaries hold."""
    packages = workspace_packages(root)
    found = []
    for package, target in ENGINE_BINARY.findall(engine):
        source = packages[package] / "tests" / f"{target}.rs"
        found += [
            (f"{package}::{target}", name)
            for name in TEST_FUNCTION.findall(source.read_text(encoding="utf-8"))
        ]
    return found


class TheEngineSetSplitsTheTests(unittest.TestCase):
    def test_both_test_stages_take_the_engine_set_from_its_one_definition(self):
        examined("test stages that split the workspace", list(SPLIT))
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            check = CHECK.read_text(encoding="utf-8")
            self.assertEqual(split_problems(check, where / "check"), [])
            # The judge refuses each way a check.sh could break the split.
            for at, (name, planted, expected) in enumerate(PLANTED_SPLITS):
                with self.subTest(planted=name):
                    self.assertEqual(split_problems(planted, where / f"planted-{at}"), expected)

    def test_the_engine_set_names_test_binaries_that_hold_tests(self):
        definitions = ENGINE_DEFINITION.findall(CHECK.read_text(encoding="utf-8"))
        self.assertEqual(len(definitions), 1, "check.sh defines no single engine set")
        self.assertEqual(engine_binary_problems(definitions[0]), [])
        tests = examined("tests in the engine set's binaries", engine_tests(definitions[0]))
        self.assertEqual(
            sorted({binary for binary, _ in tests}),
            sorted(f"{p}::{t}" for p, t in ENGINE_BINARY.findall(definitions[0])),
            "a binary of the engine set holds no test",
        )
        # A set naming nothing, a target no package has, and a package no crate declares.
        self.assertEqual(
            engine_binary_problems("none()"),
            ["the engine set is not a union of whole test binaries: none()"],
        )
        self.assertEqual(
            engine_binary_problems("binary_id(=deck-streak-ingest::no_such_target)"),
            ["deck-streak-ingest::no_such_target: no test target with tests at no_such_target.rs"],
        )
        self.assertEqual(
            engine_binary_problems("binary_id(=deck-streak-nowhere::sync)"),
            ["deck-streak-nowhere::sync: no crate declares the package deck-streak-nowhere"],
        )


# A slice of the engine set as the engine job hands it over, m/n, and what nextest must be given
# for it (SPEC-038 R16); then slices that name none, which fail the stage before any build.
SLICES = [
    ("1/2", ["--partition", "slice:1/2"]),
    ("2/2", ["--partition", "slice:2/2"]),
    ("3/7", ["--partition", "slice:3/7"]),
]
NOT_SLICES = ["0/2", "3/2", "2", "a/b", "1/2/3", " 1/2", "01/2"]


class TheEngineStageRunsItsSlice(unittest.TestCase):
    def test_the_engine_stage_runs_the_slice_it_is_given(self):
        check = CHECK.read_text(encoding="utf-8")
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            done, calls = run_recorded(check, "test-engine", where / "whole")
            self.assertEqual((done.returncode, len(calls)), (0, 1), done.stdout)
            whole = calls[0]
            self.assertEqual(filtersets(whole)[0], [ENGINE_DEFINITION.findall(check)[0]])
            self.assertNotIn("--partition", whole, "given no slice, test-engine slices the set")
            for at, (given, added) in enumerate(examined("slices", SLICES)):
                env = {"ENGINE_SLICE": given}
                done, calls = run_recorded(check, "test-engine", where / f"slice-{at}", env)
                self.assertEqual(calls, [whole + added], f"test-engine given the slice {given!r}")
            for at, given in enumerate(NOT_SLICES):
                env = {"ENGINE_SLICE": given}
                done, calls = run_recorded(check, "test-engine", where / f"not-{at}", env)
                self.assertEqual(calls, [], f"test-engine ran cargo with the slice {given!r}")
                named = (
                    rf"^FAILED +test-engine .*: ENGINE_SLICE names no slice m/n of n: '{given}'$"
                )
                self.assertRegex(summary(done), named, done.stdout)


if __name__ == "__main__":
    unittest.main()

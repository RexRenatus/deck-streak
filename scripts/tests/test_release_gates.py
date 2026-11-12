"""SPEC-502: a release tag is published only when the scheduler's release gates are on file.

`scripts/release_gates.py` reads the record `release-gates.json` at the root of the tag's checkout
and decides whether the release job may run its publish step (ADR-520). A1 reads the record's
schema, A2 plants each missing or failed gate, A3 a missing record, A4 reads the Lean entry's
vectors against the script, A5 runs the gate and publish steps cut from `release.yml` over planted
records with `gh` stubbed, and A6 holds the step apart from every other release step. The last
class reads the command's help and the code `main` returns. Every enumerating test prints
`examined N` and refuses zero. The tests start bash and the interpreter that runs them, and no other
program.
"""

import ast
import contextlib
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_mutation_python_judge_kills import load

SCRIPT = REPO / "scripts" / "release_gates.py"
RELEASE = REPO / ".github" / "workflows" / "release.yml"
VECTORS = REPO / "formal" / "vectors" / "release-gates.jsonl"
SPEC = (
    REPO
    / "docs"
    / "specs"
    / ("SPEC-502-a-release-tag-is-published-only-when-the-schedulers-release-gates-are-on-file.md")
)

release_gates = load(SCRIPT, "release_gates")

GATE_RUN = "python3 scripts/release_gates.py"
PUBLISH_RUN = 'gh release edit "$GITHUB_REF_NAME" --draft=false'
TAG = "v9.9.9"
DIGEST = "0123456789abcdef" * 4
STAMP = re.compile(r"sha256:[0-9a-f]{64}")
# The gates in the record's order (SPEC-502 R1).
ORDER = (
    "model",
    "dev.logloss",
    "dev.rmse_bins",
    "sealed_holds",
    "worse_cells",
    "simulator_win",
    "permission_on_file",
    "scheduler_gate_done",
)
REFUSED_LINE = re.compile(r"^release-gates: REFUSED: (\S+) \((missing|failed)\)")
PASS_LINE = "release-gates: PASS: every gate holds for the scheduler's weights {model}"


def passing():
    """A record on which every gate holds."""
    return {
        "model": DIGEST,
        "dev": {"logloss": [0.30, 0.32], "rmse_bins": [0.050, 0.061]},
        "sealed_holds": True,
        "worse_cells": 0,
        "simulator_win": True,
        "permission_on_file": {"value": True, "date": "2026-10-28"},
        "scheduler_gate_done": True,
    }


def set_dev(key, value):
    return lambda r: r["dev"].update({key: value})


def set_key(key, value):
    return lambda r: r.update({key: value})


def drop_dev(key):
    return lambda r: r["dev"].pop(key)


def drop_key(key):
    return lambda r: r.pop(key)


# Each gate with the change that removes it from a passing record, and the change that makes it
# fail there (None for `model`, which holds whenever it is on file).
GATES = (
    ("model", drop_key("model"), None),
    ("dev.logloss", drop_dev("logloss"), set_dev("logloss", [0.32, 0.30])),
    ("dev.rmse_bins", drop_dev("rmse_bins"), set_dev("rmse_bins", [0.061, 0.050])),
    ("sealed_holds", drop_key("sealed_holds"), set_key("sealed_holds", False)),
    ("worse_cells", drop_key("worse_cells"), set_key("worse_cells", 1)),
    ("simulator_win", drop_key("simulator_win"), set_key("simulator_win", False)),
    (
        "permission_on_file",
        drop_key("permission_on_file"),
        set_key("permission_on_file", {"value": False, "date": None}),
    ),
    ("scheduler_gate_done", drop_key("scheduler_gate_done"), set_key("scheduler_gate_done", False)),
)

# Each value of another type than the schema's, with the one gate it leaves missing.
WRONG_TYPES = (
    ("an upper-case digest", "model", set_key("model", DIGEST.upper())),
    ("a short digest", "model", set_key("model", DIGEST[:63])),
    ("a digest and a newline", "model", set_key("model", DIGEST + "\n")),
    ("a digest as a number", "model", set_key("model", 7)),
    ("a boolean in a pair", "dev.logloss", set_dev("logloss", [True, 1.0])),
    ("strings in a pair", "dev.logloss", set_dev("logloss", ["0.30", "0.32"])),
    ("a pair of one", "dev.logloss", set_dev("logloss", [0.30])),
    ("a pair of three", "dev.logloss", set_dev("logloss", [0.30, 0.32, 0.34])),
    ("a pair as an object", "dev.logloss", set_dev("logloss", {"0": 0.30, "1": 0.32})),
    ("a NaN in a pair", "dev.logloss", set_dev("logloss", [float("nan"), 0.32])),
    ("an infinity in a pair", "dev.rmse_bins", set_dev("rmse_bins", [0.050, float("inf")])),
    ("a null pair", "dev.rmse_bins", set_dev("rmse_bins", None)),
    ("a boolean as a string", "sealed_holds", set_key("sealed_holds", "true")),
    ("a boolean as a number", "sealed_holds", set_key("sealed_holds", 1)),
    ("a count as a float", "worse_cells", set_key("worse_cells", 0.0)),
    ("a count as a boolean", "worse_cells", set_key("worse_cells", False)),
    ("a count as a string", "worse_cells", set_key("worse_cells", "0")),
    ("a null boolean", "simulator_win", set_key("simulator_win", None)),
    ("a permission as a boolean", "permission_on_file", set_key("permission_on_file", True)),
    (
        "a permission whose value is a string",
        "permission_on_file",
        set_key("permission_on_file", {"value": "yes", "date": None}),
    ),
    (
        "a permission with no date",
        "permission_on_file",
        set_key("permission_on_file", {"value": True}),
    ),
    (
        "a permission dated in words",
        "permission_on_file",
        set_key("permission_on_file", {"value": True, "date": "28 October"}),
    ),
    (
        "a permission dated with a newline",
        "permission_on_file",
        set_key("permission_on_file", {"value": True, "date": "2026-10-28\n"}),
    ),
    ("a gate done as a number", "scheduler_gate_done", set_key("scheduler_gate_done", 0)),
)

# Each record that is not on file, as the bytes of `release-gates.json` (None: no file at all).
NOT_ON_FILE = (
    ("no record", None),
    ("an empty file", b""),
    ("text that is not JSON", b"{"),
    ("bytes that are not UTF-8", b"\xff\xfe"),
    ("a JSON array", b"[]"),
    ("a JSON string", b'"release"'),
    ("a key twice", b'{"sealed_holds": true, "sealed_holds": false}'),
)


def as_pairs(refusals):
    return [list(pair) for pair in refusals]


def judged(record):
    """The script's verdict on a record object, as [gate, why] pairs."""
    return as_pairs(release_gates.verdict(release_gates.read_gates(record)))


def planted(change):
    record = passing()
    change(record)
    return record


class Tree:
    """A checkout holding the script under test, and a record when one is planted."""

    def __init__(self, case):
        scratch = tempfile.TemporaryDirectory()
        case.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        (self.root / "scripts").mkdir()
        (self.root / "scripts" / "release_gates.py").write_bytes(SCRIPT.read_bytes())

    def plant(self, data):
        """Writes `release-gates.json`: a record object as JSON, or the given bytes."""
        record = self.root / release_gates.RECORD
        if isinstance(data, bytes):
            record.write_bytes(data)
        else:
            record.write_text(json.dumps(data), encoding="utf-8")
        return self

    def gate(self):
        """Runs the script as the release step does: from the checkout's root, with no argument."""
        return subprocess.run(
            [sys.executable, "scripts/release_gates.py"],
            cwd=self.root,
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
        )


def refusal_lines(stdout):
    return [list(m.groups()) for m in map(REFUSED_LINE.match, stdout.splitlines()) if m]


def release_job(text):
    """The release job's own keys and its steps, read from the workflow's text without a YAML
    library: a step starts at `      - ` under the job's `    steps:`, its keys are its first line's
    and the lines at eight spaces, and a `run: |` value is the lines below it, dedented."""
    lines = text.splitlines()
    start = lines.index("  release:")
    end = next(
        (i for i in range(start + 1, len(lines)) if re.match(r"^  [A-Za-z0-9_-]+:", lines[i])),
        len(lines),
    )
    job = lines[start + 1 : end]
    keys = [m.group(1) for m in (re.match(r"^    ([A-Za-z0-9_-]+):", line) for line in job) if m]
    at = job.index("    steps:")
    steps, block = [], None
    for line in job[at + 1 :]:
        if line.startswith("      - "):
            block = [line]
            steps.append(block)
        elif block is not None and (not line.strip() or line.startswith("        ")):
            block.append(line)
        elif line.strip() and not line.lstrip().startswith("#"):
            block = None
    return keys, [step_of(block) for block in steps]


def step_of(block):
    """One step: its keys in order, its name, its run (a block dedented) and its text."""
    while block and not block[-1].strip():
        block = block[:-1]
    first = re.match(r"^      - ([A-Za-z0-9_-]+):\s*(.*)$", block[0])
    keys, values = [first.group(1)], {first.group(1): first.group(2)}
    for line in block[1:]:
        m = re.match(r"^        ([A-Za-z0-9_-]+):\s*(.*)$", line)
        if m:
            keys.append(m.group(1))
            values[m.group(1)] = m.group(2)
    run = values.get("run")
    if run is not None and run.strip() == "|":
        at = next(i for i, line in enumerate(block) if re.match(r"^        run:", line))
        body = [line[10:] if line.startswith(" " * 10) else "" for line in block[at + 1 :]]
        run = "\n".join(body).rstrip("\n") + "\n"
    return {"keys": keys, "name": values.get("name"), "run": run, "text": "\n".join(block) + "\n"}


def gate_and_publish():
    keys, steps = release_job(RELEASE.read_text(encoding="utf-8"))
    gates = [at for at, step in enumerate(steps) if step["run"] == GATE_RUN]
    if len(gates) != 1:
        raise AssertionError(f"{len(gates)} release steps run `{GATE_RUN}`")
    publish = [at for at, step in enumerate(steps) if "--draft=false" in str(step["run"])]
    if len(publish) != 1:
        raise AssertionError(f"{len(publish)} release steps publish")
    return keys, steps, gates[0], publish[0]


def run_job(tree, steps):
    """Runs each step's run under GitHub's default bash in the tree, as a job whose steps carry no
    `if:` runs them: a step runs only when every step before it succeeded. `gh` is a stub that
    appends its arguments to a log. Gives each step's exit code (None for a step not run) and the
    stub's log lines."""
    stub = tree.root / "stub"
    stub.mkdir()
    log = tree.root / "gh.log"
    gh = stub / "gh"
    gh.write_text('#!/bin/sh\nprintf \'%s\\n\' "$*" >> "$GH_LOG"\n', encoding="utf-8")
    gh.chmod(0o755)
    env = {
        "PATH": f"{stub}{os.pathsep}{os.environ['PATH']}",
        "GH_LOG": str(log),
        "GITHUB_REF_NAME": TAG,
        "HOME": str(tree.root),
    }
    codes, failed = [], False
    for at, step in enumerate(steps):
        if failed:
            codes.append(None)
            continue
        script = tree.root / f"step-{at}.sh"
        script.write_text(step["run"] + "\n", encoding="utf-8")
        done = subprocess.run(
            ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", str(script)],
            cwd=tree.root,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
        )
        codes.append(done.returncode)
        failed = done.returncode != 0
    lines = log.read_text(encoding="utf-8").splitlines() if log.is_file() else []
    return codes, lines


def span_digest(path, name):
    """The sha256 of a top-level function's span: every line from its `def` to its last line,
    each followed by a newline, as the formal checker stamps a cover."""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    for node in ast.parse(text).body:
        if isinstance(node, ast.FunctionDef) and node.name == name:
            span = "".join(line + "\n" for line in lines[node.lineno - 1 : node.end_lineno])
            return hashlib.sha256(span.encode("utf-8")).hexdigest()
    raise AssertionError(f"{path.name} defines no top-level {name}")


class TheRecordSchema(unittest.TestCase):
    """A1 (R1, R5)."""

    def test_the_documented_record_is_read_with_every_gate_on_file(self):
        text = SPEC.read_text(encoding="utf-8")
        blocks = re.findall(r"```json\n(.*?)\n```", text, re.DOTALL)
        self.assertEqual(len(blocks), 1, "the SPEC documents one record")
        documented = json.loads(blocks[0])
        self.assertEqual(sorted(documented), sorted(passing()), "the documented keys")
        self.assertEqual(sorted(documented["dev"]), sorted(passing()["dev"]))
        self.assertEqual(sorted(documented["permission_on_file"]), ["date", "value"])
        refusals = examined("documented gates judged", judged(documented))
        self.assertEqual({why for _, why in refusals}, {"failed"}, "a documented gate is missing")
        self.assertEqual(
            [gate for gate, _ in refusals],
            [g for g in ORDER if g not in ("model", "worse_cells")],
        )

    def test_a_passing_record_admits_and_names_its_weights(self):
        gates = release_gates.read_gates(passing())
        self.assertEqual(len(gates), len(ORDER))
        self.assertNotIn(None, list(gates), "a gate of the passing record is not on file")
        self.assertEqual(gates.model, DIGEST)
        self.assertEqual(gates.logloss, (0.30, 0.32))
        self.assertEqual(gates.permission_on_file, True)
        self.assertEqual(judged(passing()), [])
        done = Tree(self).plant(passing()).gate()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(done.stdout, PASS_LINE.format(model=DIGEST) + "\n")
        widened = planted(lambda r: r.update(notes="an extra key is read past"))
        self.assertEqual(judged(widened), [])
        integers = planted(set_dev("logloss", [1, 2]))
        self.assertEqual(judged(integers), [], "an integer is a number")

    def test_a_value_of_another_type_reads_as_its_gate_missing(self):
        for label, gate, change in examined("values of another type", WRONG_TYPES):
            with self.subTest(plant=label):
                self.assertEqual(judged(planted(change)), [[gate, "missing"]], label)
        no_dev = planted(set_key("dev", [0.30, 0.32]))
        self.assertEqual(judged(no_dev), [["dev.logloss", "missing"], ["dev.rmse_bins", "missing"]])


class EachGateRefusesByName(unittest.TestCase):
    """A2 (R2, R3)."""

    def assert_refused(self, record, expected):
        self.assertEqual(judged(record), expected)
        done = Tree(self).plant(record).gate()
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertEqual(refusal_lines(done.stdout), expected, done.stdout)
        self.assertNotIn("PASS", done.stdout)
        last = done.stdout.splitlines()[-1]
        self.assertEqual(
            last, f"release-gates: the release is not published: {len(expected)} gate(s) refused"
        )

    def test_each_gate_missing_or_failed_alone_is_refused_by_its_name(self):
        self.assertEqual([gate for gate, _, _ in GATES], list(ORDER))
        for gate, drop, fail in examined("gates", GATES):
            with self.subTest(gate=gate, why="missing"):
                self.assert_refused(planted(drop), [[gate, "missing"]])
            if fail is not None:
                with self.subTest(gate=gate, why="failed"):
                    self.assert_refused(planted(fail), [[gate, "failed"]])

    def test_every_refusing_gate_is_named_in_the_records_order(self):
        bare = {}
        self.assert_refused(bare, [[gate, "missing"] for gate in ORDER])
        failing = passing()
        for _, _, fail in GATES:
            if fail is not None:
                fail(failing)
        self.assert_refused(failing, [[gate, "failed"] for gate in ORDER if gate != "model"])
        mixed = planted(lambda r: None)
        GATES[0][1](mixed)
        GATES[2][2](mixed)
        GATES[5][1](mixed)
        self.assert_refused(
            mixed,
            [["model", "missing"], ["dev.rmse_bins", "failed"], ["simulator_win", "missing"]],
        )
        examined("records with several refusing gates", [bare, failing, mixed])

    def test_a_tie_or_a_negative_count_fails(self):
        cases = (
            (set_dev("logloss", [0.30, 0.30]), "dev.logloss"),
            (set_dev("rmse_bins", [0.050, 0.050]), "dev.rmse_bins"),
            (set_key("worse_cells", -1), "worse_cells"),
            (set_key("worse_cells", 2), "worse_cells"),
        )
        for change, gate in examined("boundary records", cases):
            with self.subTest(gate=gate):
                self.assert_refused(planted(change), [[gate, "failed"]])


class AMissingRecordRefuses(unittest.TestCase):
    """A3 (R4)."""

    def test_a_record_that_is_not_on_file_refuses_as_the_record(self):
        self.assertEqual(as_pairs(release_gates.verdict(None)), [["record", "missing"]])
        for label, data in examined("records not on file", NOT_ON_FILE):
            with self.subTest(record=label):
                tree = Tree(self)
                if data is not None:
                    tree.plant(data)
                document, cause = release_gates.read_record(tree.root / release_gates.RECORD)
                self.assertIsNone(document, label)
                self.assertTrue(cause, label)
                done = tree.gate()
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertEqual(refusal_lines(done.stdout), [["record", "missing"]], done.stdout)
                self.assertIn(cause, done.stdout)
                self.assertNotIn("PASS", done.stdout)

    def test_a_folder_in_the_records_place_is_no_record(self):
        tree = Tree(self)
        (tree.root / release_gates.RECORD).mkdir()
        done = tree.gate()
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertEqual(refusal_lines(done.stdout), [["record", "missing"]])
        examined("folders in the record's place", [tree.root / release_gates.RECORD])

    def test_the_object_hook_names_a_key_held_twice(self):
        with self.assertRaises(release_gates.TwiceError) as raised:
            json.loads('{"model": "a", "model": "b"}', object_pairs_hook=release_gates.once)
        self.assertEqual(str(raised.exception), "a key twice")

    def test_a_record_that_cannot_be_read_is_named_unreadable(self):
        class Unreadable:
            """A record on file whose bytes cannot be read, as a file the reader may not open
            leaves it, whoever runs the test."""

            def is_file(self):
                return True

            def read_text(self, encoding):
                raise PermissionError(13, "Permission denied")

        self.assertEqual(
            release_gates.read_record(Unreadable()),
            (None, f"{release_gates.RECORD} cannot be read"),
        )


class TheLeanVectors(unittest.TestCase):
    """A4 (R8): the Lean entry's vectors, written by the formal checker, are the script's."""

    def test_every_vector_is_the_scripts_verdict(self):
        self.assertTrue(VECTORS.is_file(), f"{VECTORS.relative_to(REPO)} is absent")
        lines = VECTORS.read_text(encoding="utf-8").splitlines()
        header = json.loads(lines[0])
        self.assertEqual(header["schema"], "phx.formal.vectors.v1")
        self.assertEqual(header["entry"], "ReleaseGates")
        self.assertEqual(header["covers"], "scripts/release_gates.py")
        self.assertEqual(header["anchor"], "verdict")
        self.assertIsNotNone(STAMP.fullmatch(header["digest"]), header["digest"])
        self.assertNotEqual(
            header["digest"], "sha256:" + "0" * 64, "the vectors were written unstamped"
        )
        self.assertEqual(
            header["digest"],
            "sha256:" + span_digest(SCRIPT, "verdict"),
            "the entry's stamp is stale",
        )
        vectors = [json.loads(line) for line in lines[1:]]
        self.assertEqual(header["vectors"], len(vectors))
        for vector in examined("Lean vectors", vectors):
            record = vector["record"]
            gates = None if record is None else release_gates.read_gates(record)
            with self.subTest(record=record):
                self.assertEqual(as_pairs(release_gates.verdict(gates)), vector["refused"])
        missing = [v for v in vectors if v["record"] is None]
        self.assertEqual([v["refused"] for v in missing], [[["record", "missing"]]])
        whys = {why for v in vectors for _, why in v["refused"]}
        self.assertEqual(whys, {"missing", "failed"}, "the vectors reach both refusals")
        named = {gate for v in vectors for gate, _ in v["refused"]}
        self.assertEqual(named, {"record", *ORDER}, "the vectors name every gate")
        self.assertIn([], [v["refused"] for v in vectors], "no vector admits")


class AFailingRecordLeavesPublishUnreached(unittest.TestCase):
    """A5 (R6)."""

    def test_the_publish_step_runs_only_after_a_passing_record(self):
        keys, steps, gate, publish = gate_and_publish()
        self.assertEqual(gate + 1, publish, "the gate is directly before the publish")
        self.assertEqual(publish, len(steps) - 1, "the publish is the last step")
        self.assertNotIn("continue-on-error", keys, "the job runs on after a failed step")
        for step in (steps[gate], steps[publish]):
            self.assertNotIn("if", step["keys"], step["name"])
            self.assertNotIn("continue-on-error", step["keys"], step["name"])
        refusing = [(f"{gate_name} {why}", change) for gate_name, change, why in self.refusing()]
        refusing += [(label, data) for label, data in NOT_ON_FILE]
        for label, data in examined("refusing records", refusing):
            with self.subTest(record=label):
                tree = Tree(self)
                if data is not None:
                    tree.plant(data)
                codes, log = run_job(tree, [steps[gate], steps[publish]])
                self.assertEqual(codes, [1, None], f"{label}: the publish step ran")
                self.assertEqual(log, [], f"{label}: gh was called")
        tree = Tree(self).plant(passing())
        codes, log = run_job(tree, [steps[gate], steps[publish]])
        self.assertEqual(codes, [0, 0], "a passing record does not reach the publish")
        self.assertEqual(log, [f"release edit {TAG} --draft=false"])

    @staticmethod
    def refusing():
        for gate, drop, fail in GATES:
            yield gate, planted(drop), "missing"
            if fail is not None:
                yield gate, planted(fail), "failed"


class EveryOtherReleaseStepIsUnchanged(unittest.TestCase):
    """A6 (R7): the gate is one inert step, so no other step reads or loses anything by it."""

    def test_the_gate_is_one_step_with_a_name_and_a_one_line_run(self):
        _, steps, gate, publish = gate_and_publish()
        step = steps[gate]
        self.assertEqual(step["keys"], ["name", "run"], "the gate step holds a name and a run")
        self.assertEqual(step["text"].count("\n"), 2, step["text"])
        self.assertNotIn("${{", step["text"])
        for mark in "|;&":
            self.assertNotIn(mark, step["run"], "the script's exit is the step's exit")
        named = [s for s in steps if "release_gates" in s["text"]]
        self.assertEqual(len(named), 1, "one step names the script")
        self.assertEqual(steps[publish]["run"], PUBLISH_RUN)
        self.assertEqual(steps[publish]["keys"], ["name", "env", "run"])
        examined("release steps", steps)

    def test_the_script_sets_nothing_another_step_reads(self):
        text = SCRIPT.read_text(encoding="utf-8")
        tree = ast.parse(text)
        imported = sorted(
            {
                alias.name.split(".")[0]
                for node in ast.walk(tree)
                if isinstance(node, ast.Import)
                for alias in node.names
            }
            | {
                node.module.split(".")[0]
                for node in ast.walk(tree)
                if isinstance(node, ast.ImportFrom) and node.module
            }
        )
        self.assertEqual(
            imported, ["argparse", "json", "math", "pathlib", "re", "sys", "typing"], "stdlib only"
        )
        for name in examined(
            "names a step could pass on",
            ("GITHUB_ENV", "GITHUB_OUTPUT", "GITHUB_PATH", "GITHUB_STEP_SUMMARY", "environ"),
        ):
            self.assertNotIn(name, text)
        self.assertNotRegex(text, r"\.write\(|write_text|write_bytes|open\(", "the script writes")
        self.assertNotRegex(text, r"\bsubprocess\b|\bos\.system\b", "the script starts a program")


class TheCommandStatesItself(unittest.TestCase):
    """The command's help, and the code `main` returns, which the process's exit cannot tell from
    None: `sys.exit(None)` exits 0 as well."""

    def test_the_help_names_the_gate_and_its_root(self):
        done = subprocess.run(
            [sys.executable, str(SCRIPT), "--help"],
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
            env={**os.environ, "COLUMNS": "200"},
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        shown = " ".join(done.stdout.split())
        self.assertIn(
            "Refuse the release's publish step unless the release gates hold (SPEC-502).", shown
        )
        self.assertIn(
            "--root ROOT the checkout whose release-gates.json is read (default: the one holding "
            "this script)",
            shown,
        )

    def test_main_returns_zero_when_every_gate_holds(self):
        tree = Tree(self).plant(passing())
        shown = io.StringIO()
        with contextlib.redirect_stdout(shown):
            code = release_gates.main(["--root", str(tree.root)])
        self.assertEqual(code, 0, shown.getvalue())
        self.assertEqual(shown.getvalue(), PASS_LINE.format(model=DIGEST) + "\n")


if __name__ == "__main__":
    unittest.main()

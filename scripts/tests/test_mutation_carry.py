"""SPEC-493 A1, A3, A4 and A5: a release run carries a Python outcome from one recorded run.

Every test runs `scripts/mutation_carry.py` or `scripts/mutation-verdict.py` as the interpreter's
child, over fixture trees built from `fixtures/mutation-carry/`, and reads `ci.yml`,
`scripts/mutation-inputs.json` and `scripts/mutation-python.json` as text. No test imports a
script, so no test module of this file loads code by a name held in data. The base step's
requests go to a fake forge on the loopback address, which logs each path it is asked for.
"""

import collections
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from _support import REPO, examined

SCRIPTS = REPO / "scripts"
CARRY = SCRIPTS / "mutation_carry.py"
VERDICT = SCRIPTS / "mutation-verdict.py"
CI = REPO / ".github" / "workflows" / "ci.yml"
INPUTS = SCRIPTS / "mutation-inputs.json"
MAP = SCRIPTS / "mutation-python.json"
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "mutation-carry"
PYTHON_SCHEMA = "deckstreak.mutation-python.v1"
CARRY_SCHEMA = "deckstreak.mutation-carry.v1"
OUTCOMES_SCHEMA = "deckstreak.mutation-outcomes.v1"
#: The one release pull request SPEC-493 R1 admits, spelt here as the constant spells it.
ADMITTED = frozenset({822})
REPOSITORY = "RexRenatus/deck-streak"
THIS_RUN = 500
HEAD_SHA = "f" * 40
#: The two leg jobs' text at the cut this delivery was built on, from each job's key line to its
#: last line that is neither blank nor a comment: SPEC-493 R5 keys the carry on them, so no byte
#: of either changes.
LEG_JOBS = {
    "mutation-python": "2310f028c2e4ff9ef720a12964cd3c10e038735dca2749029cbffc3a48d3c0db",
    "mutation-python-release": "9cc477cc44a5b408fa276ed8cb0e76c9d6d86ac6e217f202bc52c762801fd221",
}
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
DOWNLOAD = "actions/download-artifact@"
UPLOAD = "actions/upload-artifact@"


def child(script, *args, env=None):
    """(exit, stdout and stderr) of `script` run by this interpreter with `args`."""
    done = subprocess.run(
        [sys.executable, str(script), *map(str, args)],
        capture_output=True,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1", **(env or {})},
    )
    return done.returncode, done.stdout + done.stderr


def write(path, document):
    path.parent.mkdir(parents=True, exist_ok=True)
    text = document if isinstance(document, str) else json.dumps(document, indent=2) + "\n"
    path.write_text(text, encoding="utf-8")
    return path


def mutant(file, line, column, text, end_column):
    """One listed mutant, as the runner's listing spells it."""
    return {
        "name": f"{file}:{line}:{column}: {text}",
        "file": file,
        "line": line,
        "column": column,
        "start_line": line,
        "end_line": line,
        "end_column": end_column,
        "mutant": text,
        "operator": "arithmetic",
    }


def judged(entry, outcome, killers=(), **extra):
    return {**entry, "outcome": outcome, "killers": list(killers), **extra}


A1 = mutant("scripts/a.py", 9, 14, "replace * with / in twice", 17)
A2 = mutant("scripts/a.py", 9, 16, "replace 2 with 3 in twice", 17)
B1 = mutant("scripts/b.py", 9, 14, "replace // with * in half", 18)
B2 = mutant("scripts/b.py", 9, 17, "replace 2 with 3 in half", 18)
OLD_B1 = mutant("scripts/b.py", 9, 14, "replace / with * in half", 17)
OLD_B2 = mutant("scripts/b.py", 9, 16, "replace 2 with 3 in half", 17)
KILLER = "test_a.Holds.test_it"


def listing(entries):
    return {"schema": PYTHON_SCHEMA, "listed": len(entries), "mutants": list(entries)}


def report(shard, count, files, **extra):
    """One Python leg's report: `files` maps a path to its judged mutants."""
    return {
        "schema": PYTHON_SCHEMA,
        "selection": "plan",
        "shard": f"{shard}/{count}",
        "failfast": True,
        "files": [
            {"path": path, "modules": [], "byte_readers": [], "void": None, "mutants": mutants}
            for path, mutants in files.items()
        ],
        "exit": 0,
        **extra,
    }


def plan(code, *, carried=(), release=True, base="b" * 40, head="h" * 40, legs=None):
    """A release plan over the `scripts` class: `code` maps a path to its code lines, and each
    path of `carried` holds them as `carried_code`, as the legs' view writes it."""
    files = []
    for path, lines in code.items():
        entry = {"path": path, "class": "scripts", "added": len(lines), "deleted": []}
        entry.update({"code": list(lines), "quiet": [], "test": []})
        if path in carried:
            entry["carried_code"], entry["code"] = entry["code"], []
        files.append(entry)
    classes = {name: {"applies": False, "files": []} for name in ("rust", "web", "oracle")}
    classes["scripts"] = {"applies": True, "files": sorted(code)}
    document = {
        "base": base,
        "head": head,
        "scope": {"decision": "diff", "reason": "a fixture's release plan"},
        "files": files,
        "classes": classes,
        "rows": [],
        "row_overlaps": {},
    }
    if release:
        document["release"] = True
    if legs is not None:
        document["python"] = {
            "count": len(legs),
            "shards": [{"shard": k, "mutants": names} for k, names in enumerate(legs)],
        }
    return document


def carry_document(carried, base_run="300"):
    return {
        "schema": CARRY_SCHEMA,
        "base_run": base_run,
        "why": f"carried from run {base_run}",
        "files": {},
        "carried": carried,
    }


def names(*entries):
    return [entry["name"] for entry in entries]


class Scratch(unittest.TestCase):
    """A test with its own scratch directory, removed after it."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="carry-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)

    def reports(self, document, legs, listed, carry=None, at="reports"):
        """A verdict job's reports directory: the plan's artifact and one directory per leg."""
        where = self.tmp / at
        write(where / "mutation-plan" / "plan.json", document)
        write(where / "mutation-plan" / "python-listed.json", listing(listed))
        write(where / "mutation-plan" / "git.diff", "diff --git a/scripts/a.py b/scripts/a.py\n")
        if carry is not None:
            write(where / "mutation-plan" / "carry.json", carry)
        for shard, body in enumerate(legs):
            write(where / f"mutation-python-shard-{shard}" / "report.json", body)
        return where

    def judge(self, where, root=None):
        return child(
            VERDICT,
            "judge",
            "--root",
            root or self.tmp,
            "--plan",
            where / "mutation-plan" / "plan.json",
            "--class",
            "scripts",
            "--python",
            where,
            "--rows",
            where / "rows.json",
        )


class Repository:
    """The fixture repository in git: `main`, the base run's tree and this run's, one commit
    each, with this run's tree checked out. Every base and head edit is on top of the fixture's
    shared edits, and this run's `scripts/b.py` differs from the base run's."""

    def __init__(self, tmp, base_edits=None, head_edits=None):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        self.root = tmp / "repo"
        self.tmp = tmp
        config = write(tmp / "gitconfig", "")
        self.env = {**os.environ, "GIT_CONFIG_GLOBAL": str(config), "GIT_CONFIG_NOSYSTEM": "1"}
        files = {**data["files"], "scripts/mutation-inputs.json": INPUTS.read_text("utf-8")}
        self.root.mkdir()
        self.git("init", "-q", "-b", "main")
        self.commit(files, "main")
        self.main = self.git("rev-parse", "HEAD").strip()
        self.git("checkout", "-q", "-b", "base-run", "main")
        self.commit({**data["edits"], **(base_edits or {})}, "the base run")
        self.base_diff = self.git("diff", "--no-color", "--no-ext-diff", "--no-renames", "main...")
        self.base_head = self.git("rev-parse", "HEAD").strip()
        self.git("checkout", "-q", "-b", "head", "main")
        head = {**data["edits"], "scripts/b.py": data["head_b"], **(head_edits or {})}
        self.commit(head, "this run")
        self.head_diff = self.git("diff", "--no-color", "--no-ext-diff", "--no-renames", "main...")
        self.head = self.git("rev-parse", "HEAD").strip()

    def git(self, *args):
        done = subprocess.run(
            ["git", "-C", str(self.root), *args],
            capture_output=True,
            text=True,
            env=self.env,
            check=True,
        )
        return done.stdout

    def commit(self, files, message):
        for path, text in files.items():
            if text is None:
                (self.root / path).unlink()
            else:
                write(self.root / path, text)
        self.git("add", "-A")
        self.git("-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid", "commit",
                 "-q", "-m", message)  # fmt: skip

    def base_plan(self, **over):
        legs = [names(A1, A2, OLD_B1, OLD_B2)]
        code = {"scripts/a.py": [8, 9], "scripts/b.py": [8, 9]}
        return {**plan(code, base=self.main, head=self.base_head, legs=legs), **over}

    def head_plan(self, **over):
        code = {"scripts/a.py": [8, 9], "scripts/b.py": [8, 9]}
        return {**plan(code, base=self.main, head=self.head), **over}

    def base_legs(self):
        files = {
            "scripts/a.py": [judged(A1, "killed", [KILLER]), judged(A2, "killed", [KILLER])],
            "scripts/b.py": [judged(OLD_B1, "killed", [KILLER]), judged(OLD_B2, "survived")],
        }
        return [report(0, 1, files)]

    def carry(self, *, base_plan=None, legs=None, base_diff=None, base_run="300",
              pull_request=822, listed=None, head_plan=None, manifest=None):  # fmt: skip
        """Run `carry` as the plan job runs it; (exit, output, carry.json, plan before, after)."""
        run = self.tmp / "run"
        mutation = run / "mutation"
        document = head_plan if head_plan is not None else self.head_plan()
        plan_path = write(mutation / "plan.json", document)
        write(mutation / "git.diff", self.head_diff)
        listed_path = mutation / "python-listed.json"
        write(listed_path, listed if listed is not None else listing([A1, A2, B1, B2]))
        base = run / "carry-base"
        base_document = base_plan if base_plan is not None else self.base_plan()
        write(base / "mutation-plan" / "plan.json", base_document)
        write(
            base / "mutation-plan" / "git.diff", self.base_diff if base_diff is None else base_diff
        )
        write(base / "mutation-plan" / "python-listed.json", listing([A1, A2, OLD_B1, OLD_B2]))
        if manifest is not None:
            write(base / "mutation-outcomes" / "mutation-outcomes.json", manifest)
        for shard, body in enumerate(self.base_legs() if legs is None else legs):
            write(base / "legs" / f"mutation-python-shard-{shard}" / "report.json", body)
        before = plan_path.read_text(encoding="utf-8")
        code, text = child(
            VERDICT,
            "carry",
            "--root",
            self.root,
            "--plan",
            plan_path,
            "--python-listed",
            listed_path,
            "--base-dir",
            base,
            "--base-run",
            base_run,
            "--pull-request",
            pull_request,
        )
        carried = mutation / "carry.json"
        found = json.loads(carried.read_text(encoding="utf-8")) if carried.is_file() else None
        return code, text, found, before, plan_path.read_text(encoding="utf-8")


def carried_names(found):
    return sorted(entry["name"] for entry in (found or {}).get("carried", []))


class UnionRule(Scratch):
    """A1: the listing is the re-run names plus the carried ones, none in both (R9, R10)."""

    def legs_and_carry(self):
        legs = [report(0, 1, {"scripts/b.py": [judged(B1, "killed"), judged(B2, "killed")]})]
        carried = [judged(A1, "killed", [KILLER], run="300"), judged(A2, "killed", run="300")]
        document = plan(
            {"scripts/a.py": [8, 9], "scripts/b.py": [8, 9]},
            carried={"scripts/a.py"},
            legs=[names(B1, B2)],
        )
        return document, legs, carried

    def test_the_exact_partition_passes_and_names_its_base(self):
        document, legs, carried = self.legs_and_carry()
        where = self.reports(document, legs, [A1, A2, B1, B2], carry_document(carried))
        code, text = self.judge(where)
        self.assertEqual(code, 0, text)
        self.assertIn("mutation: scripts: carried 2 from run 300", text)
        self.assertIn("examined 4", text)
        examined("partitions", [where])

    def test_a_missing_an_extra_and_a_doubly_held_name_are_void_by_name(self):
        document, legs, carried = self.legs_and_carry()
        extra = mutant("scripts/a.py", 5, 14, "replace + with - in add", 17)
        doubled = [report(0, 1, {"scripts/b.py": [judged(B1, "killed"), judged(B2, "killed")],
                                 "scripts/a.py": [judged(A1, "killed")]})]  # fmt: skip
        cases = {
            "missing": (legs, [A1, A2, B1, B2, extra], carried,
                        "5 listed, 2 re-run and 2 carried: 1 missing, 0 extra and 0 in both"),
            "extra": (legs, [A1, A2, B1, B2], [*carried, judged(extra, "killed", run="300")],
                      "4 listed, 2 re-run and 3 carried: 0 missing, 1 extra and 0 in both"),
            "doubly held": (doubled, [A1, A2, B1, B2], carried,
                            "4 listed, 3 re-run and 2 carried: 0 missing, 1 extra and 1 in both"),
        }  # fmt: skip
        for case, (case_legs, listed, case_carried, counts) in cases.items():
            with self.subTest(case=case):
                shards = [[n for f in body["files"] for n in names(*f["mutants"])]
                          for body in case_legs]  # fmt: skip
                case_plan = {**document, "python": {
                    "count": 1, "shards": [{"shard": 0, "mutants": shards[0]}]}}  # fmt: skip
                where = self.reports(case_plan, case_legs, listed, carry_document(case_carried),
                                     at=case.replace(" ", "-"))  # fmt: skip
                code, text = self.judge(where)
                self.assertEqual(code, 3, text)
                self.assertIn(f"mutation: scripts: VOID the union rule: {counts}", text)
        examined("union readings", cases)

    def test_a_missing_carry_reads_as_nothing_carried(self):
        legs = [report(0, 1, {"scripts/b.py": [judged(B1, "killed"), judged(B2, "killed")]})]
        whole = plan({"scripts/b.py": [8, 9]}, legs=[names(B1, B2)])
        where = self.reports(whole, legs, [B1, B2], at="whole")
        code, text = self.judge(where)
        self.assertEqual(code, 0, text)
        self.assertIn("mutation: scripts: carried 0 from run none", text)
        short = plan({"scripts/a.py": [8, 9], "scripts/b.py": [8, 9]}, legs=[names(B1, B2)])
        where = self.reports(short, legs, [A1, A2, B1, B2], at="short")
        code, text = self.judge(where)
        self.assertEqual(code, 3, text)
        self.assertIn("4 listed, 2 re-run and 0 carried: 2 missing, 0 extra and 0 in both", text)
        examined("readings without a carry", ["whole", "short"])

    def test_the_legs_view_moves_only_the_carried_files_code_lines(self):
        repository = Repository(self.tmp)
        code, text, found, before, after = repository.carry()
        self.assertEqual(code, 0, text)
        self.assertEqual(found["schema"], CARRY_SCHEMA)
        self.assertEqual(found["base_run"], "300")
        self.assertEqual(carried_names(found), sorted(names(A1, A2)))
        self.assertEqual(
            {entry["name"]: (entry["outcome"], entry["killers"], entry["run"])
             for entry in found["carried"]},
            {A1["name"]: ("killed", [KILLER], "300"), A2["name"]: ("killed", [KILLER], "300")},
        )  # fmt: skip
        old, new = json.loads(before), json.loads(after)
        moved = {entry["path"]: entry for entry in new["files"]}
        self.assertEqual(moved["scripts/a.py"]["code"], [])
        self.assertEqual(moved["scripts/a.py"]["carried_code"], [8, 9])
        self.assertEqual(moved["scripts/b.py"], old["files"][1])
        new["files"] = old["files"] = None
        self.assertEqual(new, old)
        self.assertIn("carry: scripts/b.py: runs whole", text)
        examined("plan files", moved)

    def test_a_plan_whose_every_file_carries_is_judged_on_the_deal_of_an_empty_rerun_set(self):
        document = plan({"scripts/a.py": [8, 9]}, carried={"scripts/a.py"})
        where = self.tmp / "reports"
        plan_path = write(where / "mutation-plan" / "plan.json", document)
        empty = write(where / "mutation-plan" / "python-rerun.json", listing([]))
        code, text = child(VERDICT, "shards", "--plan", plan_path, "--python-listed", empty)
        self.assertEqual(code, 0, text)
        dealt = json.loads(plan_path.read_text(encoding="utf-8"))["python"]
        for shard in range(dealt["count"]):
            write(where / f"mutation-python-shard-{shard}" / "report.json",
                  report(shard, dealt["count"], {}))  # fmt: skip
        write(where / "mutation-plan" / "python-listed.json", listing([A1, A2]))
        carried = [judged(A1, "killed", run="300"), judged(A2, "killed", run="300")]
        write(where / "mutation-plan" / "carry.json", carry_document(carried))
        code, text = self.judge(where)
        self.assertEqual(code, 0, text)
        self.assertIn("mutation: scripts: carried 2 from run 300", text)
        examined("legs dealt for an empty re-run set", range(dealt["count"]))


class Forge:
    """A fake forge on the loopback address: each GET is logged and answered from `answers`,
    the first whose key the path holds, else 404."""

    def __init__(self, answers):
        self.paths = []
        forge = self

        class Answer(BaseHTTPRequestHandler):
            def do_GET(self):
                forge.paths.append(self.path)
                body = next((v for k, v in answers.items() if k in self.path), None)
                data = json.dumps(body).encode() if body is not None else b"{}"
                self.send_response(200 if body is not None else 404)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def log_message(self, *args):
                del args

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Answer)
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}"
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()


def run_entry(i, **over):
    runs = json.loads((FIXTURES / "runs.json").read_text(encoding="utf-8"))["workflow_runs"]
    entry = json.loads(json.dumps(runs[-1]))
    entry.update({"id": i, "head_sha": f"{i:040x}", **over})
    return entry


class BaseSelection(Scratch):
    """A3: one base of this repository, by run id alone, and no outcome read (R1 to R3)."""

    def answers(self, runs=None, compare="ahead", artifacts=None):
        listed = json.loads((FIXTURES / "runs.json").read_text(encoding="utf-8"))
        stored = json.loads((FIXTURES / "artifacts.json").read_text(encoding="utf-8"))
        return {
            "/actions/workflows/": listed if runs is None else {"workflow_runs": runs},
            "/compare/": {"status": compare},
            "/artifacts": stored if artifacts is None else {"artifacts": artifacts},
        }

    def base(self, answers, document=None, pull_request=822, out=None):
        forge = Forge(answers)
        self.addCleanup(forge.close)
        where = self.tmp / f"base-{len(list(self.tmp.iterdir()))}"
        plan_path = write(where / "plan.json", document if document is not None else plan({}))
        out = out or where / "carry-base"
        outputs = where / "outputs"
        code, text = child(
            CARRY,
            "base",
            "--plan",
            plan_path,
            "--pull-request",
            pull_request,
            "--run-id",
            THIS_RUN,
            "--repository",
            REPOSITORY,
            "--head-sha",
            HEAD_SHA,
            "--out",
            out,
            "--api",
            forge.url,
            "--github-output",
            outputs,
            env={"GH_TOKEN": "a-fixture-token"},
        )
        self.assertEqual(code, 0, text)
        lines = outputs.read_text(encoding="utf-8").splitlines() if outputs.is_file() else []
        return dict(line.split("=", 1) for line in lines), forge.paths, text

    def test_the_newest_qualifying_run_by_id_is_the_base(self):
        found, paths, text = self.base(self.answers())
        self.assertEqual(found, {"base_run": "400", "base_manifest": "true"}, text)
        self.assertIn("/actions/workflows/ci.yml/runs", paths[0])
        self.assertIn(f"/compare/{400:040x}...{HEAD_SHA}", paths[1])
        self.assertIn("/actions/runs/400/artifacts", paths[2])
        self.assertFalse([path for path in paths if "300" in path or "200" in path], paths)
        examined("requests", paths)

    def test_each_disqualified_newest_run_gives_no_base_never_an_older_one(self):
        good = run_entry(300, conclusion="success")
        runs = {
            "a fork's run": run_entry(450, head_repository={"full_name": "someone/deck-streak"}),
            "a cancelled run": run_entry(450, conclusion="cancelled"),
            "a skipped run": run_entry(450, conclusion="skipped"),
            "another workflow": run_entry(450, path=".github/workflows/weekly.yml"),
            "another branch": run_entry(450, head_branch="feature"),
            "a run not older than this one": run_entry(600),
            "a run with no pull request into main": run_entry(450, pull_requests=[]),
            "a run not completed": run_entry(450, status="in_progress"),
        }
        for case, newest in runs.items():
            with self.subTest(case=case):
                found, paths, text = self.base(self.answers(runs=[newest, good]))
                self.assertEqual(found.get("base_run"), "", text)
                self.assertFalse([path for path in paths if "/runs/300/" in path], paths)
        stale = [{"name": "mutation-plan", "expired": True}]
        others = {
            "a head that is not an ancestor": self.answers(compare="diverged"),
            "a head behind this one": self.answers(compare="behind"),
            "a run with no plan artifact": self.answers(
                artifacts=[{"name": "mutation-python-shard-0", "expired": False}]
            ),
            "a run whose plan artifact expired": self.answers(artifacts=stale),
        }
        for case, answers in others.items():
            with self.subTest(case=case):
                found, _paths, text = self.base(answers)
                self.assertEqual(found.get("base_run"), "", text)
        examined("disqualified runs", [*runs, *others])

    def test_a_run_not_admitted_makes_no_request(self):
        cases = {
            "not the release run": (plan({}, release=False), 822),
            "another pull request": (plan({}), 823),
            "an earlier pull request": (plan({}), 821),
        }
        for case, (document, number) in cases.items():
            with self.subTest(case=case):
                found, paths, text = self.base(self.answers(), document, number)
                self.assertEqual(paths, [], text)
                self.assertEqual(found.get("base_run"), "", text)
        found, paths, text = self.base(self.answers(), plan({}), 822)
        self.assertEqual(found.get("base_run"), "400", text)
        examined("admissions", [*cases, "the release run of 822"])

    def test_the_admitted_pull_requests_are_822_alone(self):
        code, text = child(CARRY, "admitted")
        self.assertEqual(code, 0, text)
        self.assertEqual(frozenset(json.loads(text)), ADMITTED)
        examined("admitted pull requests", json.loads(text))

    def test_a_selector_handed_outcomes_reads_none(self):
        clean, clean_paths, _text = self.base(self.answers())
        out = self.tmp / "handed" / "carry-base"
        write(out / "legs" / "mutation-python-shard-0" / "report.json", "{ not a report")
        write(out / "mutation-outcomes" / "mutation-outcomes.json", "[ not a manifest")
        handed, paths, text = self.base(self.answers(), out=out)
        self.assertEqual(handed, clean, text)
        self.assertEqual(paths, clean_paths)
        examined("selections", [clean, handed])

    def test_a_base_without_a_manifest_downloads_its_legs(self):
        artifacts = [
            {"name": "mutation-plan", "expired": False},
            {"name": "mutation-python-shard-0", "expired": False},
            {"name": "mutation-outcomes", "expired": True},
        ]
        found, _paths, text = self.base(self.answers(artifacts=artifacts))
        self.assertEqual(found, {"base_run": "400", "base_manifest": "false"}, text)
        examined("artifact listings", artifacts)


def jobs(text):
    """{job: its lines}, each job from its key line to the line before the next job's."""
    lines = text.splitlines()
    found, name = {}, None
    for line in lines:
        match = JOB.match(line)
        if match:
            name = match.group(1)
            found[name] = []
        elif line[:1].strip():
            name = None
        if name is not None:
            found[name].append(line)
    return found


def job_text(text, name):
    """A job's text, to its last line that is neither blank nor a comment."""
    block = list(jobs(text)[name])
    while block and (not block[-1].strip() or block[-1].lstrip().startswith("#")):
        block.pop()
    return "".join(line + "\n" for line in block)


def steps(block):
    """The job's steps, each as its lines, from its `- ` line to the line before the next."""
    found = []
    for line in block:
        if line.startswith("      - "):
            found.append([line])
        elif found and line.startswith("        "):
            found[-1].append(line)
        elif found and line.strip() and not line.lstrip().startswith("#"):
            found.append(None)
    return [step for step in found if step]


def action(line):
    """The action a `uses:` line pins, its comment included."""
    return line.removeprefix("- ").removeprefix("uses: ")


def step_named(block, needle):
    hits = [step for step in steps(block) if any(needle in line for line in step)]
    return hits


class Workflow(unittest.TestCase):
    """A3: `ci.yml` as text: the plan job's permissions, base steps and deal, and the kept
    manifest (R2, R3, R9, R12)."""

    def setUp(self):
        self.text = CI.read_text(encoding="utf-8")
        self.jobs = jobs(self.text)

    def test_the_plan_job_reads_contents_and_actions_only(self):
        block = self.jobs["mutation-plan"]
        at = [i for i, line in enumerate(block) if line == "    permissions:"]
        self.assertEqual(len(at), 1, "mutation-plan holds one job-level permissions block")
        held = []
        for line in block[at[0] + 1 :]:
            if not line.startswith("      "):
                break
            held.append(line.strip())
        self.assertEqual(sorted(held), ["actions: read", "contents: read"])
        timeout = [i for i, line in enumerate(block) if line.startswith("    timeout-minutes:")]
        self.assertEqual(len(timeout), 1)
        self.assertLess(timeout[0], at[0])
        header = self.text.split("\njobs:\n")[0].splitlines()
        start = header.index("permissions:")
        self.assertEqual(header[start + 1 : start + 2], ["  contents: read"])
        self.assertFalse(header[start + 2 : start + 3] and header[start + 2].startswith("  "))
        examined("permission lines", held)

    def test_no_base_step_can_fail_the_plan(self):
        block = self.jobs["mutation-plan"]
        base = step_named(block, "        id: base")
        self.assertEqual(len(base), 1, "one base step")
        lines = [line.strip() for line in base[0]]
        self.assertEqual(lines[lines.index("id: base") + 1], "continue-on-error: true")
        self.assertTrue(any(line.startswith("timeout-minutes:") for line in lines), lines)
        joined = "\n".join(lines)
        for clause in (
            "github.event_name == 'pull_request'",
            "github.base_ref == 'main'",
            "github.head_ref == 'dev'",
            "github.event.pull_request.head.repo.full_name == github.repository",
            "GH_TOKEN: ${{ github.token }}",
            "python3 scripts/mutation_carry.py base",
        ):
            self.assertIn(clause, joined)
        pinned = {line.strip() for line in self.jobs["mutation-verdict"] if DOWNLOAD in line}
        downloads = [step for step in steps(block) if any(DOWNLOAD in line for line in step)]
        targets = {
            "${{ runner.temp }}/carry-base/mutation-plan": "name: mutation-plan",
            "${{ runner.temp }}/carry-base/mutation-outcomes": "name: mutation-outcomes",
            "${{ runner.temp }}/carry-base/legs": "pattern: mutation-python-shard-*",
        }
        seen = []
        for step in downloads:
            lines = [line.strip() for line in step]
            path = next((line[len("path: ") :] for line in lines if line.startswith("path: ")), "")
            if path not in targets:
                continue
            seen.append(path)
            used = next(line for line in lines if DOWNLOAD in line)
            self.assertIn(action(used), {action(line) for line in pinned})
            for needed in (
                targets[path],
                "continue-on-error: true",
                "run-id: ${{ steps.base.outputs.base_run }}",
                "github-token: ${{ github.token }}",
                "repository: ${{ github.repository }}",
            ):
                self.assertIn(needed, lines, path)
            condition = next(line for line in lines if line.startswith("if: "))
            self.assertIn("steps.base.outputs.base_run != ''", condition)
            self.assertTrue(any(line.startswith("timeout-minutes:") for line in lines), path)
        self.assertEqual(sorted(seen), sorted(targets))
        examined("base steps and downloads", [base, *seen])

    def test_shards_deals_the_rerun_set(self):
        block = self.jobs["mutation-plan"]
        order = []
        for needle in (
            '--out "$RUNNER_TEMP/mutation/python-listed.json"',
            "        id: base",
            "python3 scripts/mutation-verdict.py carry",
            '--out "$RUNNER_TEMP/mutation/python-rerun.json"',
            "        id: shards",
        ):
            hits = [i for i, line in enumerate(block) if needle in line]
            self.assertEqual(len(hits), 1, needle)
            order.append(hits[0])
        self.assertEqual(order, sorted(order))
        rerun = next(line for line in block if 'python-rerun.json"' in line and "list" in line)
        self.assertIn(
            'python3 scripts/mutation_python.py list --plan "$RUNNER_TEMP/mutation/plan.json" '
            '--out "$RUNNER_TEMP/mutation/python-rerun.json"',
            rerun,
        )
        shards = step_named(block, "        id: shards")[0]
        run = next(line for line in shards if "mutation-verdict.py shards" in line)
        self.assertIn('--python-listed "$RUNNER_TEMP/mutation/python-rerun.json"', run)
        self.assertNotIn("python-listed.json", run)
        carry = step_named(block, "python3 scripts/mutation-verdict.py carry")[0]
        self.assertFalse([line for line in carry if line.strip().startswith("if:")], carry)
        examined("plan steps in order", order)

    def test_the_manifest_is_kept_when_a_leg_fails(self):
        block = self.jobs["mutation-verdict"]
        manifest = step_named(block, "python3 scripts/mutation-verdict.py manifest")
        self.assertEqual(len(manifest), 1)
        self.assertIn("if: ${{ always() }}", [line.strip() for line in manifest[0]])
        name = "      - name: keep the run's outcomes for a later release run's carry"
        at = block.index(name)
        self.assertEqual(block[at + 1], "        if: ${{ always() }}")
        upload = step_named(block, name.strip()[2:])[0]
        lines = [line.strip() for line in upload]
        pinned = {line.strip() for line in self.jobs["mutation-plan"] if UPLOAD in line}
        self.assertTrue(any(line.removeprefix("uses: ") in {p.removeprefix("uses: ")
                                                            for p in pinned}
                            for line in lines if line.startswith("uses: ")), lines)  # fmt: skip
        for needed in (
            "name: mutation-outcomes",
            "path: ${{ runner.temp }}/outcomes/",
            "if-no-files-found: warn",
        ):
            self.assertIn(needed, lines)
        timeouts = block.index("      - name: the Rust legs' Timeouts by package, each over-margin "
                               "package by name")  # fmt: skip
        self.assertLess(timeouts, block.index(manifest[0][0]))
        examined("manifest steps", [manifest, upload])

    def test_the_leg_jobs_are_the_cuts_byte_for_byte(self):
        found = {
            name: hashlib.sha256(job_text(self.text, name).encode()).hexdigest()
            for name in LEG_JOBS
        }
        self.assertEqual(found, LEG_JOBS)
        examined("leg jobs", found)


class Key(Scratch):
    """A4: the key of a Python class is its leg jobs' text, the runner's bytes and the runner
    digest of `mutation_rows.py` (R5)."""

    def tree(self, edits=None, name="tree"):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        files = {**data["files"], "scripts/mutation-inputs.json": INPUTS.read_text("utf-8")}
        files.update(edits or {})
        root = self.tmp / name
        for path, text in files.items():
            write(root / path, text)
        code, text = child(CARRY, "key", "--root", root)
        self.assertEqual(code, 0, text)
        return json.loads(text)

    def test_the_key_reads_the_leg_jobs_the_runner_and_its_named_rows(self):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        ci, rows = (
            data["files"][".github/workflows/ci.yml"],
            data["files"]["scripts/mutation_rows.py"],
        )
        runner = data["files"]["scripts/mutation_python.py"]
        key = self.tree()
        self.assertEqual(sorted(key), ["oracle", "scripts"])
        cases = {
            "a leg job's step": (ci.replace("--shard 0/1", "--shard 0/2"), None, None, False),
            "a release leg's step": (ci.replace("--shard 1/2", "--shard 2/2"), None, None, False),
            "a comment in a leg job": (ci.replace("      # one leg", "      # a leg"), None, None,
                                       True),
            "a blank line in a leg job": (ci.replace("    steps:\n      # one", "    steps:\n\n"
                                                     "      # one"), None, None, True),
            "another job": (ci.replace("echo plan", "echo the plan"), None, None, True),
            "the runner's bytes": (None, runner + "# a comment\n", None, False),
            "a named rows function": (None, None, rows.replace("hexdigest()", "digest().hex()"),
                                      False),
            "a module-level name": (None, None, rows.replace("WIDTH = 12", "WIDTH = 13"), False),
            "a new top-level import": (None, None, rows.replace("import hashlib",
                                                                "import hashlib\nimport os"),
                                       False),
            "an unreferenced rows function": (None, None, rows.replace("return 1", "return 2"),
                                              True),
            "a comment in the rows module": (None, None, rows.replace("WIDTH = 12",
                                                                      "WIDTH = 12  # bytes"),
                                             True),
        }  # fmt: skip
        for index, (case, (new_ci, new_runner, new_rows, same)) in enumerate(cases.items()):
            with self.subTest(case=case):
                edits = {
                    path: text
                    for path, text in (
                        (".github/workflows/ci.yml", new_ci),
                        ("scripts/mutation_python.py", new_runner),
                        ("scripts/mutation_rows.py", new_rows),
                    )
                    if text is not None
                }
                found = self.tree(edits, name=f"case-{index}")
                (self.assertEqual if same else self.assertNotEqual)(found, key)
        examined("key cases", cases)

    def test_a_changed_key_or_map_entry_carries_nothing(self):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        ci, rows = (
            data["files"][".github/workflows/ci.yml"],
            data["files"]["scripts/mutation_rows.py"],
        )
        mapping = json.loads(data["files"]["scripts/mutation-python.json"])
        mapping["scripts/a.py"]["modules"] = ["test_a", "test_b"]
        cases = {
            "a changed leg job": ({".github/workflows/ci.yml": ci.replace("0/1", "0/2")}, "key"),
            "a changed runner": ({"scripts/mutation_python.py": "# another runner\n"}, "key"),
            "a referenced rows name": ({"scripts/mutation_rows.py": rows.replace("[:WIDTH]", "")},
                                       "key"),
            "a changed map entry": ({"scripts/mutation-python.json": json.dumps(mapping)},
                                    "map entry"),
        }  # fmt: skip
        for index, (case, (edits, reason)) in enumerate(cases.items()):
            with self.subTest(case=case):
                where = self.tmp / f"case-{index}"
                where.mkdir()
                code, text, found, before, after = Repository(where, edits).carry()
                self.assertEqual(code, 0, text)
                self.assertEqual(carried_names(found), [], text)
                self.assertEqual(after, before)
                self.assertIn(reason, found["files"]["scripts/a.py"]["why"], text)
        where = self.tmp / "unreferenced"
        where.mkdir()
        edits = {"scripts/mutation_rows.py": rows.replace("return 1", "return 2")}
        code, text, found, _before, _after = Repository(where, edits).carry()
        self.assertEqual(carried_names(found), sorted(names(A1, A2)), text)
        examined("key and map cases", [*cases, "an unreferenced rows function"])

    def test_a_rebuilt_base_file_must_begin_with_its_index_id(self):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        ci = data["files"][".github/workflows/ci.yml"]
        edits = {".github/workflows/ci.yml": ci.replace("# one leg", "# a leg")}
        repository = Repository(self.tmp, edits)
        code, text, found, _before, _after = repository.carry()
        self.assertEqual(carried_names(found), sorted(names(A1, A2)), text)
        index = re.compile(r"^(index [0-9a-f]+\.\.)([0-9a-f]+)", re.M)
        section = repository.base_diff.split("diff --git a/.github/workflows/ci.yml")[1]
        old = index.search(section).group(2)
        forged = repository.base_diff.replace(f"..{old}", f"..{'0' * len(old)}", 1)
        (self.tmp / "run" / "mutation" / "carry.json").unlink()
        code, text, found, before, after = repository.carry(base_diff=forged)
        self.assertEqual(carried_names(found), [], text)
        self.assertEqual(after, before)
        self.assertIn("index", found["why"], text)
        examined("rebuilt base files", [old])


class FailSafe(Scratch):
    """A4: each fail-safe path carries nothing by name and leaves its part whole (R13, R14)."""

    def test_each_failsafe_path_carries_nothing_by_name(self):
        def held_twice(repository):
            legs = repository.base_legs()
            twice = report(1, 2, {"scripts/a.py": [judged(A1, "killed", [KILLER])]})
            first = report(0, 2, {f["path"]: f["mutants"] for f in legs[0]["files"]})
            shards = [names(A1, A2, OLD_B1, OLD_B2), names(A1)]
            return {"legs": [first, twice], "base_plan": repository.base_plan(python={
                "count": 2, "shards": [{"shard": k, "mutants": s} for k, s in enumerate(shards)]})}  # fmt: skip

        cases = {
            "no base": (lambda r: {"base_run": ""}, "no base"),
            "not admitted": (lambda r: {"pull_request": 823}, "not admitted"),
            "not the release run": (lambda r: {"head_plan": {**r.head_plan(), "release": False}},
                                    "not admitted"),
            "an unreadable base plan": (lambda r: {"base_plan": "{ not a plan"}, "plan"),
            "a base that is not a release plan": (
                lambda r: {"base_plan": {k: v for k, v in r.base_plan().items() if k != "release"}},
                "release"),
            "a moved main": (lambda r: {"base_plan": r.base_plan(base="c" * 40)}, "main"),
            "a name held twice": (held_twice, "2 times"),
            "an unreadable base leg": (lambda r: {"legs": ["{ not a report"]}, "judged"),
            "a timeout at the base": (lambda r: {"legs": [report(0, 1, {
                "scripts/a.py": [judged(A1, "timeout"), judged(A2, "killed")],
                "scripts/b.py": [judged(OLD_B1, "killed"), judged(OLD_B2, "killed")]})]},
                "timeout"),
            "an error inside carry": (lambda r: {"listed": ["not", "a", "listing"]},
                                      "an error inside carry"),
        }  # fmt: skip
        for index, (case, (make, reason)) in enumerate(cases.items()):
            with self.subTest(case=case):
                where = self.tmp / f"case-{index}"
                where.mkdir()
                repository = Repository(where)
                code, text, found, before, after = repository.carry(**make(repository))
                self.assertEqual(code, 0, text)
                self.assertIsNotNone(found, text)
                self.assertEqual(found["schema"], CARRY_SCHEMA)
                self.assertEqual(carried_names(found), [], text)
                self.assertEqual(after, before)
                why = " ".join([found["why"], *(f["why"] for f in found["files"].values())])
                self.assertIn(reason, why, text)
        examined("fail-safe paths", cases)

    def test_a_carry_never_reads_a_rust_rows_or_web_outcome(self):
        repository = Repository(self.tmp)
        document = repository.head_plan()
        document["files"].append({"path": "crates/x/src/lib.rs", "class": "rust", "added": 1,
                                  "deleted": [], "code": [3], "quiet": [], "test": []})  # fmt: skip
        document["classes"]["rust"] = {"applies": True, "files": ["crates/x/src/lib.rs"]}
        code, text, found, before, after = repository.carry(head_plan=document)
        self.assertEqual(carried_names(found), sorted(names(A1, A2)), text)
        rust = next(e for e in json.loads(after)["files"] if e["path"] == "crates/x/src/lib.rs")
        self.assertEqual(rust["code"], [3])
        self.assertNotIn("carried_code", rust)
        examined("classes the carry leaves whole", ["rust"])


class JudgedAtHead(Scratch):
    """A5: a carried outcome is judged by the records at the head, as a re-run one is (R11)."""

    def root(self, records):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        root = self.tmp / "root"
        write(root / "scripts" / "a.py", data["edits"]["scripts/a.py"])
        write(root / "scripts" / "b.py", data["head_b"])
        if records is not None:
            write(root / "scripts" / "mutation-equivalent.d" / "python.json", {"records": records})
        return root

    def verdict(self, carried, records, at):
        legs = [report(0, 1, {"scripts/b.py": [judged(B1, "killed"), judged(B2, "killed")]})]
        document = plan(
            {"scripts/a.py": [8, 9], "scripts/b.py": [8, 9]},
            carried={"scripts/a.py"},
            legs=[names(B1, B2)],
        )
        where = self.reports(document, legs, [A1, A2, B1, B2], carry_document(carried), at=at)
        return self.judge(where, root=self.root(records))

    def record(self, entry, anchor="return x * 2"):
        return {
            "file": entry["file"],
            "mutant": entry["mutant"],
            "anchor": anchor,
            "reason": "the fixture's reason",
            "evidence": "the fixture's evidence",
            "issue": "#822",
        }

    def test_a_carried_survivor_fails_unless_one_record_at_the_head_excuses_it(self):
        carried = [judged(A1, "survived", run="300"), judged(A2, "killed", run="300")]
        code, text = self.verdict(carried, None, "bare")
        self.assertEqual(code, 1, text)
        self.assertIn(f"SURVIVED {A1['name']}", text)
        (self.tmp / "root").rename(self.tmp / "bare-root")
        code, text = self.verdict(carried, [self.record(A1)], "excused")
        self.assertEqual(code, 0, text)
        self.assertIn(f"EQUIVALENT {A1['name']}", text)
        (self.tmp / "root").rename(self.tmp / "excused-root")
        code, text = self.verdict(carried, [self.record(A1), self.record(A1)], "twice")
        self.assertEqual(code, 1, text)
        self.assertIn(f"SURVIVED {A1['name']}", text)
        examined("carried survivors", ["bare", "excused", "twice"])

    def test_a_carried_kill_refutes_a_record_and_a_carried_uncovered_mutant_fails(self):
        carried = [judged(A1, "killed", [KILLER], run="300"), judged(A2, "killed", run="300")]
        code, text = self.verdict(carried, [self.record(A1)], "refuted")
        self.assertEqual(code, 1, text)
        self.assertIn("verdict: FAIL", text)
        (self.tmp / "root").rename(self.tmp / "refuted-root")
        carried = [judged(A1, "killed", run="300"), judged(A2, "uncovered", run="300")]
        code, text = self.verdict(carried, None, "uncovered")
        self.assertEqual(code, 1, text)
        self.assertIn(f"UNCOVERED {A2['name']}", text)
        examined("carried outcomes", ["refuted", "uncovered"])

    def test_a_carried_timeout_or_void_outcome_is_refused(self):
        for outcome in ("timeout", "void"):
            with self.subTest(outcome=outcome):
                carried = [judged(A1, outcome, run="300"), judged(A2, "killed", run="300")]
                code, text = self.verdict(carried, None, outcome)
                self.assertNotEqual(code, 0, text)
                self.assertIn(f"carried outcome {outcome}", text)
                (self.tmp / "root").rename(self.tmp / f"{outcome}-root")
        examined("outcomes never carried", ["timeout", "void"])


class Manifest(Scratch):
    """A5: the verdict job keeps every whole and carried Python outcome, and nothing else (R12)."""

    def test_the_manifest_holds_whole_and_carried_entries_only(self):
        data = json.loads((FIXTURES / "repository.json").read_text(encoding="utf-8"))
        root = self.tmp / "root"
        files = {**data["files"], "scripts/mutation-inputs.json": INPUTS.read_text("utf-8")}
        for path, text in files.items():
            write(root / path, text)
        c1 = mutant("scripts/c.py", 3, 14, "replace + with - in c", 17)
        d1 = mutant("scripts/d.py", 3, 14, "replace + with - in d", 17)
        whole = report(0, 2, {
            "scripts/b.py": [judged(B1, "killed", [KILLER]), judged(B2, "survived")],
            "scripts/e.py": [judged(mutant("scripts/e.py", 2, 5, "replace 1 with 2 in e", 6),
                                    "timeout"),
                             judged(mutant("scripts/e.py", 3, 5, "replace 3 with 4 in e", 6),
                                    "void")],
            "scripts/c.py": [judged(c1, "killed")],
        })  # fmt: skip
        whole["files"][2]["void"] = "the sentinel left no test that does not read the file"
        partial = {**report(1, 2, {"scripts/d.py": [judged(d1, "killed")]}), "shard": "0/2"}
        listed = [n for f in whole["files"] for n in names(*f["mutants"])]
        document = plan(
            {"scripts/a.py": [8, 9], "scripts/b.py": [8, 9]},
            carried={"scripts/a.py"},
            legs=[listed, names(d1)],
        )
        carried = [judged(A1, "killed", [KILLER], run="300"), judged(A2, "survived", run="280")]
        where = self.reports(document, [whole, partial], [], carry_document(carried))
        out = self.tmp / "outcomes" / "mutation-outcomes.json"
        code, text = child(VERDICT, "manifest", "--root", root, "--plan",
                           where / "mutation-plan" / "plan.json", "--python", where, "--out", out,
                           "--run-id", THIS_RUN)  # fmt: skip
        self.assertEqual(code, 0, text)
        kept = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(kept["schema"], OUTCOMES_SCHEMA)
        diff = (where / "mutation-plan" / "git.diff").read_bytes()
        self.assertEqual(
            kept["tree"],
            {"base": "b" * 40, "head": "h" * 40, "diff_sha256": hashlib.sha256(diff).hexdigest()},
        )
        _code, key = child(CARRY, "key", "--root", root)
        self.assertEqual(kept["keys"], json.loads(key))
        entries = {
            entry["name"]: (entry["file"], entry["class"], entry["outcome"], entry["killers"],
                            entry["run"])
            for entry in kept["mutants"]
        }  # fmt: skip
        self.assertEqual(
            entries,
            {
                B1["name"]: ("scripts/b.py", "scripts", "killed", [KILLER], str(THIS_RUN)),
                B2["name"]: ("scripts/b.py", "scripts", "survived", [], str(THIS_RUN)),
                A1["name"]: ("scripts/a.py", "scripts", "killed", [KILLER], "300"),
                A2["name"]: ("scripts/a.py", "scripts", "survived", [], "280"),
            },
        )
        counted = collections.Counter(entry["name"] for entry in kept["mutants"])
        self.assertEqual(set(counted.values()), {1})
        examined("manifest entries", kept["mutants"])


if __name__ == "__main__":
    unittest.main()

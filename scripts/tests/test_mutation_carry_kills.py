"""Tests that kill the carry module's listed mutants in process (SPEC-493).

`scripts/mutation_carry.py` is loaded by the one `load()` below, afresh for each test, so its own
lines run inside the test that judges them. Each class holds one table-driven method per class of
the kill table, and `scripts/mutation-python.json` maps this module first among the carry module's
test modules, so under the failfast legs a kill ends a mutant's run before the slower child-process
tests start. Every case is labelled by its mutant's `<line>:<col> <operator>`, runs under its own
`subTest`, and asserts a whole value the mutant changes. A call a mutant makes raise is read
through `attempt`, so the case fails by assertion and not by an exception.
"""

import ast
import collections
import contextlib
import dataclasses
import hashlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined

CARRY = REPO / "scripts" / "mutation_carry.py"


def load():
    spec = importlib.util.spec_from_file_location("mutation_carry_under_kill", CARRY)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def attempt(thunk):
    """What `thunk()` returns, or the name and text of what it raises."""
    try:
        return thunk()
    except Exception as error:
        return ("raised", type(error).__name__, str(error))


class Base(unittest.TestCase):
    """A fresh carry module and a scratch directory under TMPDIR for each test."""

    def setUp(self):
        self.mc = load()
        self.scratch = Path(tempfile.mkdtemp(prefix="carry-kills-"))
        self.addCleanup(shutil.rmtree, self.scratch)

    def judge(self, table):
        """Each case's (label, thunk, want): the thunk's value, or what it raises, is `want`."""
        for label, thunk, want in table:
            with self.subTest(case=label):
                self.assertEqual(attempt(thunk), want)


# The splits of one walk up, counted across the parts each split returns.
WALKS = collections.Counter()


class Walk(str):
    """A path whose every split is counted in WALKS, each part a Walk too; a ninth split is
    refused, so a walk up that never ends fails at its ninth step and not at a leg's bound."""

    def rsplit(self, sep=None, maxsplit=-1):
        WALKS["rsplit"] += 1
        if WALKS["rsplit"] > 8:
            raise AssertionError(f"the walk up split {WALKS['rsplit']} times: it never ends")
        return [Walk(part) for part in str.rsplit(self, sep, maxsplit)]


class AncestorsEnd(Base):
    """K01: ancestors walks up to the root in one split per directory."""

    def test_each_walk_up_ends_at_the_root_in_a_bounded_count(self):
        def walk(path):
            WALKS.clear()
            return self.mc.ancestors(Walk(path)), WALKS["rsplit"]

        table = [
            (
                '224:11 replace "/" with ""',
                lambda: walk("scripts/tests/x.py"),
                (["scripts/tests", "scripts", "."], 2),
            ),
        ]
        self.judge(table)
        examined("K01 cases", table)


def qualifying(**over):
    """A run that passes every test of select_base for run 10 of o/r, with `over` changed."""
    run = {
        "id": 5,
        "path": ".github/workflows/ci.yml@refs/heads/dev",
        "event": "pull_request",
        "head_branch": "dev",
        "head_repository": {"full_name": "o/r"},
        "pull_requests": [{"base": {"ref": "main"}}],
        "status": "completed",
        "conclusion": "success",
        "head_sha": "b1",
    }
    run.update(over)
    return run


def unnumbered():
    """A qualifying run that holds no id."""
    return {key: value for key, value in qualifying().items() if key != "id"}


def printed(thunk):
    """(what `thunk()` returns, the lines it prints)."""
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        value = thunk()
    return value, out.getvalue().splitlines()


FORGE = "http://forge"
RUNS_URL = (
    "http://forge/repos/o/r/actions/workflows/ci.yml/runs"
    "?event=pull_request&branch=dev&status=completed&per_page=100"
)
COMPARE_URL = "http://forge/repos/o/r/compare/b1...h1"
ARTIFACTS_URL = "http://forge/repos/o/r/actions/runs/5/artifacts?per_page=100&direction=asc"
UNSET = {"base_run": "", "base_manifest": "false"}


class BaseChoice(Base):
    """K02 to K04: the admission, the one candidate and the base step's requests."""

    def test_admission_names_each_refusal_exactly(self):
        plan = {"release": True}
        table = [
            (
                "78:16 replace return value with return None",
                lambda: self.mc.admission(plan, "x"),
                "not admitted: 'x' names no pull request",
            ),
            (
                "79:15 replace not in with in",
                lambda: (self.mc.admission(plan, 823), self.mc.admission(plan, 822)),
                ("not admitted: pull request #823 is not one the carry admits", None),
            ),
        ]
        self.judge(table)
        examined("K02 cases", table)

    def test_select_base_names_each_disqualifying_test_exactly(self):
        def pick(listed, run_id=10):
            return self.mc.select_base(listed, run_id, "o/r")

        def runs(*listed):
            return {"workflow_runs": list(listed)}

        def refused(why, number=5):
            return None, f"no base: the newest run, {number}, is no base: {why}"

        table = [
            (
                "90:16 replace return value with return None",
                lambda: pick([]),
                (None, "no base: the runs listing holds no workflow runs"),
            ),
            (
                '90:22 replace "no base: the runs listing holds no w... with ""',
                lambda: pick({"workflow_runs": "x"}),
                (None, "no base: the runs listing holds no workflow runs"),
            ),
            (
                "91:62 replace and with or",
                lambda: pick(runs(1, qualifying())),
                (qualifying(), "the newest run, 5, qualifies"),
            ),
            (
                '91:74 replace "id" with ""',
                lambda: pick(runs(qualifying(), qualifying(id=10))),
                (qualifying(), "the newest run, 5, qualifies"),
            ),
            (
                "93:16 replace return value with return None",
                lambda: pick(runs()),
                (None, "no base: no other run of ci is listed"),
            ),
            (
                '93:22 replace "no base: no other run of ci is listed" with ""',
                lambda: pick(runs({"id": 10})),
                (None, "no base: no other run of ci is listed"),
            ),
            (
                '94:57 replace "id" with ""',
                lambda: pick(runs(qualifying(id=3), qualifying(id=5))),
                (qualifying(id=5), "the newest run, 5, qualifies"),
            ),
            (
                "94:63 replace or with and",
                lambda: pick(runs(qualifying(id=4), qualifying(id=6))),
                (qualifying(id=6), "the newest run, 6, qualifies"),
            ),
            (
                "94:66 replace 0 with 1",
                lambda: pick(runs(unnumbered(), qualifying(id=1))),
                (qualifying(id=1), "the newest run, 1, qualifies"),
            ),
            (
                "99:24 replace 0 with 1",
                lambda: pick(runs(unnumbered()), run_id=1),
                (unnumbered(), "the newest run, None, qualifies"),
            ),
            (
                "99:27 replace < with >=",
                lambda: pick(runs(qualifying())),
                (qualifying(), "the newest run, 5, qualifies"),
            ),
            (
                '99:37 replace "it is not older than this run" with ""',
                lambda: pick(runs(qualifying(id=12))),
                refused("it is not older than this run", 12),
            ),
            (
                '101:52 replace "its event is not pull_request" with ""',
                lambda: pick(runs(qualifying(event="push"))),
                refused("its event is not pull_request"),
            ),
            (
                '102:49 replace "its head branch is not dev" with ""',
                lambda: pick(runs(qualifying(head_branch="main"))),
                refused("its head branch is not dev"),
            ),
            (
                '103:48 replace "its head is not this repository\'s" with ""',
                lambda: pick(runs(qualifying(head_repository={"full_name": "x/y"}))),
                refused("its head is not this repository's"),
            ),
            (
                "105:61 replace == with !=",
                lambda: pick(runs(qualifying())),
                (qualifying(), "the newest run, 5, qualifies"),
            ),
            (
                '106:13 replace "it names no pull request into main" with ""',
                lambda: pick(runs(qualifying(pull_requests=[]))),
                refused("it names no pull request into main"),
            ),
            (
                '108:50 replace "it is not completed" with ""',
                lambda: pick(runs(qualifying(status="in_progress"))),
                refused("it is not completed"),
            ),
            (
                "116:20 replace return value with return None",
                lambda: pick(runs(qualifying(conclusion="cancelled"))),
                refused("its conclusion is cancelled"),
            ),
        ]
        self.judge(table)
        examined("K03 cases", table)

    def test_choose_base_requests_and_answers_exactly(self):
        def fetched():
            seen = []

            def urlopen(request, timeout):
                seen.append((request.full_url, sorted(request.header_items()), timeout))
                return io.BytesIO(b'{"runs": 1}')

            with mock.patch.object(self.mc.urllib.request, "urlopen", urlopen):
                value = self.mc.fetch("http://forge/x", "t")
            return value, seen

        def base(api=FORGE, out="out", release=True, token="t", status="ahead"):
            plan = self.scratch / "plan.json"
            plan.write_text(json.dumps({"release": release}), encoding="utf-8")
            args = self.mc.argparse.Namespace(
                plan=str(plan),
                pull_request="822",
                out=str(self.scratch / out),
                repository="o/r",
                api=api,
                run_id="10",
                head_sha="h1",
            )
            answers = {
                RUNS_URL: {"workflow_runs": [qualifying()]},
                COMPARE_URL: {"status": status},
                ARTIFACTS_URL: {
                    "artifacts": [{"name": "mutation-plan"}, {"name": "mutation-outcomes"}]
                },
            }
            urls = []

            def answer(url, held):
                urls.append((url, held))
                return answers.get(url)

            with mock.patch.object(self.mc, "fetch", answer):
                outputs, lines = printed(lambda: self.mc.choose_base(args, token))
            return outputs, lines, urls

        headers = [
            ("Accept", "application/vnd.github+json"),
            ("Authorization", "Bearer t"),
            ("X-github-api-version", "2022-11-28"),
        ]
        sent = ({"runs": 1}, [("http://forge/x", headers, 60)])
        carried = (
            {"base_run": "5", "base_manifest": "true"},
            [
                "carry: base: the newest run, 5, qualifies",
                "carry: base: run 5, its manifest to be read",
            ],
            [(RUNS_URL, "t"), (COMPARE_URL, "t"), (ARTIFACTS_URL, "t")],
        )
        no_token = (UNSET, ["carry: base: no base: the step holds no token"], [])
        table = [
            ('52:15 replace "2022-11-28" with ""', fetched, sent),
            ('124:23 replace "application/vnd.github+json" with ""', fetched, sent),
            ("129:50 replace 60 with 61", fetched, sent),
            (
                '136:28 replace "" with "XX"',
                lambda: base(release=False),
                (
                    UNSET,
                    [
                        "carry: base: not admitted: this run is not the release run, "
                        "so nothing carries"
                    ],
                    [],
                ),
            ),
            ('136:32 replace "base_manifest" with ""', lambda: base(token=""), no_token),
            (
                '136:49 replace "false" with ""',
                lambda: base(api="http://other"),
                (
                    UNSET,
                    ["carry: base: no base: the runs listing holds no workflow runs"],
                    [(RUNS_URL.replace(FORGE, "http://other"), "t")],
                ),
            ),
            (
                '142:15 replace "carry: base: no base: the step holds... with ""',
                lambda: base(token=""),
                no_token,
            ),
            ("143:16 replace return value with return None", lambda: base(token=""), no_token),
            ("145:23 replace True with False", lambda: base(out="a/b"), carried),
            ('146:50 replace "/" with ""', lambda: base(api="http://forge/"), carried),
            (
                '147:13 replace "event=pull_request&branch=dev&status... with ""',
                base,
                carried,
            ),
            ('158:32 replace "identical" with ""', lambda: base(status="identical"), carried),
        ]
        self.judge(table)
        examined("K04 cases", table)


OUTCOMES = "deckstreak.mutation-outcomes.v1"
NOT_ITS_SCHEMA = ("raised", "NoBase", "no base: the base's manifest is not of its schema")


class BaseJudgement(Base):
    """K05 and K06: what a base judged, by its kept manifest or by its legs."""

    def judged(self, text=None, reports=()):
        """base_judged over a scratch base whose manifest holds `text` (none when None), with a
        stand-in reader that records each Scratch it is handed and yields `reports`."""
        base = Path(tempfile.mkdtemp(dir=self.scratch))
        if text is not None:
            (base / "mutation-outcomes").mkdir(parents=True)
            (base / "mutation-outcomes" / "mutation-outcomes.json").write_text(
                text, encoding="utf-8"
            )
        calls = []

        def reader(scratch, plan, directory):
            calls.append((scratch.klass, scratch.where, plan, directory))
            return list(reports)

        found = self.mc.base_judged(base, {"plan": 1}, "300", reader)
        return found, calls, str(base / "legs")

    def test_a_kept_manifest_is_read_whole_or_refused_by_name(self):
        kept = json.dumps(
            {
                "schema": OUTCOMES,
                "mutants": [
                    {"name": "m1", "outcome": "killed", "killers": ["k1"], "run": "7"},
                    "x",
                ],
            }
        )
        read = (({"m1": [("killed", ["k1"], "7")]}, "its manifest"), [])

        def manifest(text):
            found, calls, _legs = self.judged(text)
            return found, calls

        table = [
            (
                '1140:53 replace "mutation-outcomes.json" with ""',
                lambda: manifest(kept),
                read,
            ),
            ("1142:12 replace not with nothing", lambda: manifest(kept), read),
            ("1142:43 replace or with and", lambda: manifest("[1]"), NOT_ITS_SCHEMA),
            ('1142:59 replace "schema" with ""', lambda: manifest(kept), read),
            (
                "1142:69 replace != with ==",
                lambda: manifest(json.dumps({"mutants": []})),
                NOT_ITS_SCHEMA,
            ),
            (
                '1143:26 replace "no base: the base\'s manifest is not ... with ""',
                lambda: manifest("null"),
                NOT_ITS_SCHEMA,
            ),
            ('1144:35 replace "mutants" with ""', lambda: manifest(kept), read),
            ("1144:46 replace or with and", lambda: manifest(kept), read),
            ('1146:49 replace "name" with ""', lambda: manifest(kept), read),
            ('1148:39 replace "outcome" with ""', lambda: manifest(kept), read),
            ('1149:40 replace "killers" with ""', lambda: manifest(kept), read),
            ("1149:51 replace or with and", lambda: manifest(kept), read),
            ('1150:39 replace "run" with ""', lambda: manifest(kept), read),
            ("1153:16 replace return value with return None", lambda: manifest(kept), read),
            ('1153:24 replace "its manifest" with ""', lambda: manifest(kept), read),
        ]
        self.judge(table)
        examined("K05 cases", table)

    def test_the_legs_are_read_through_the_verdicts_reader(self):
        report = {
            "files": [
                {"void": True, "mutants": [{"name": "v1", "outcome": "survived"}]},
                "x",
                {"mutants": [{"name": "m2", "outcome": "survived", "killers": ["k2"]}]},
            ]
        }

        def legs():
            found, calls, directory = self.judged(reports=[("leg", report)])
            return found, calls == [("scripts", "the base's legs", {"plan": 1}, directory)]

        read = (({"m2": [("survived", ["k2"], "300")]}, "its legs"), True)
        table = [
            ('1154:23 replace "scripts" with ""', legs, read),
            ('1154:34 replace "the base\'s legs" with ""', legs, read),
            ("1157:44 replace or with and", legs, read),
            ('1157:57 replace "void" with ""', legs, read),
            ("1158:17 replace continue with break", legs, read),
            ('1164:20 replace "its legs" with ""', legs, read),
        ]
        self.judge(table)
        examined("K06 cases", table)


def frozen_section(section):
    """The refusal of an assignment to a Section's path, or "assigned"."""
    try:
        section.path = "q"
    except dataclasses.FrozenInstanceError as error:
        return str(error)
    return "assigned"


def frozen_value(value):
    """The refusal of an assignment to a Value's kind, or "assigned"."""
    try:
        value.kind = "q"
    except dataclasses.FrozenInstanceError as error:
        return str(error)
    return "assigned"


class CarryConstants(Base):
    """K07: the module's values, as spelt, each read from a fresh load."""

    def test_each_module_value_is_as_spelt(self):
        def bytecode():
            with mock.patch.object(sys, "dont_write_bytecode", False):
                load()
                return sys.dont_write_bytecode

        table = [
            ("36:27 replace True with False", bytecode, True),
            (
                '44:17 replace "deckstreak.mutation-python.v1" with ""',
                lambda: self.mc.PYTHON_SCHEMA,
                "deckstreak.mutation-python.v1",
            ),
            (
                '46:47 replace "unviable" with ""',
                lambda: self.mc.CARRIED,
                ("killed", "survived", "uncovered", "unviable"),
            ),
            (
                '47:30 replace "oracle" with ""',
                lambda: self.mc.PYTHON_CLASSES,
                ("scripts", "oracle"),
            ),
            ("55:49 replace 1 with 2", lambda: self.mc.ROOT == REPO, True),
            (
                "181:19 replace True with False",
                lambda: frozen_section(self.mc.Section("p", "t", False, False, False, None)),
                "cannot assign to field 'path'",
            ),
            ('539:7 replace "<cwd>" with ""', lambda: self.mc.CWD, "<cwd>"),
            (
                "542:19 replace True with False",
                lambda: frozen_value(self.mc.Value("path", "a")),
                "cannot assign to field 'kind'",
            ),
            ('548:17 replace "" with "XX"', lambda: self.mc.Value("temp").text, ""),
            ("1009:25 replace 0 with 1", lambda: self.mc.Scratch("scripts", "x").examined, 0),
        ]
        self.judge(table)
        examined("K07 cases", table)


CARRY_SCHEMA = "deckstreak.mutation-carry.v1"
ENTRY = {
    "rule": "closure",
    "workflow": "w.yml",
    "leg_jobs": ["j"],
    "runner": ["r.py"],
    "runner_digest": {"file": "d.py", "named_by": "n.py", "module": "m"},
}
INPUTS_TEXT = json.dumps(
    {"schema": "deckstreak.mutation-inputs.v1", "classes": {"scripts": ENTRY, "oracle": ENTRY}}
)
KEPT = json.dumps(
    {
        "schema": "deckstreak.mutation-outcomes.v1",
        "mutants": [{"name": "m1", "outcome": "killed", "killers": ["k1"], "run": "7"}],
    }
)
HEAD_ONLY = (
    "diff --git a/docs/x.md b/docs/x.md\n"
    "index 1111111..2222222 100644\n"
    "--- a/docs/x.md\n"
    "+++ b/docs/x.md\n"
    "@@ -1 +1 @@\n"
    "-a\n"
    "+b\n"
)
PLAN_TEXT = (
    '{"files": [{"path": "scripts/a.py", "code": ["c1"]}, '
    '{"path": "scripts/b.py", "code": ["c2"]}]}'
)
MOVED_PLAN = (
    "{\n"
    '  "files": [\n'
    "    {\n"
    '      "path": "scripts/a.py",\n'
    '      "code": [],\n'
    '      "carried_code": [\n'
    '        "c1"\n'
    "      ]\n"
    "    },\n"
    "    {\n"
    '      "path": "scripts/b.py",\n'
    '      "code": [\n'
    '        "c2"\n'
    "      ]\n"
    "    }\n"
    "  ]\n"
    "}\n"
)


def written(path, text):
    """Write `text` at `path`, its directories made."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def refused_carry(why):
    """The carry document of a run that carries nothing, for `why`."""
    return {"schema": CARRY_SCHEMA, "base_run": "", "why": why, "files": {}, "carried": []}


class CarryDocument(Base):
    """K08 to K10: the carry document, each file's refusal and the verb that writes them."""

    def carried(self, release=True, base_run="300", on="m1", diff="", beside=True, inputs=True):
        """carry over a scratch tree, plan and base, with every child process refused and
        recorded: (the document, the recorded calls)."""
        place = Path(tempfile.mkdtemp(dir=self.scratch))
        root = place / "root"
        root.mkdir()
        if inputs:
            written(root / "scripts" / "mutation-inputs.json", INPUTS_TEXT)
        plan = place / "plan" / "plan.json"
        written(plan, json.dumps({"release": release, "base": "m1", "files": []}))
        if beside:
            written(place / "plan" / "git.diff", diff)
        base = place / "base"
        written(
            base / "mutation-plan" / "plan.json",
            json.dumps({"classes": {}, "release": True, "base": on}),
        )
        written(base / "mutation-plan" / "git.diff", "")
        written(base / "mutation-outcomes" / "mutation-outcomes.json", KEPT)
        listed = place / "listed.json"
        written(listed, json.dumps({"mutants": [{"file": "scripts/a.py", "name": "m1"}]}))
        args = self.mc.argparse.Namespace(
            root=str(root),
            plan=str(plan),
            pull_request="822",
            base_run=base_run,
            base_dir=str(base),
            python_listed=str(listed),
        )
        calls = []

        def run(argv, **options):
            calls.append(argv)
            return subprocess.CompletedProcess(argv, 1, b"", b"")

        with mock.patch.object(self.mc.subprocess, "run", run):
            document = self.mc.carry(
                args, lambda *given: [], lambda file: "scripts", lambda tree: ({}, [])
            )
        return document, calls

    def test_carry_writes_each_refusal_and_carry_exactly(self):
        whole = {
            "schema": CARRY_SCHEMA,
            "base_run": "300",
            "why": "carried 1 from run 300",
            "files": {
                "scripts/a.py": {"carried": True, "why": "carried 1 from run 300, by its manifest"}
            },
            "carried": [
                {
                    "file": "scripts/a.py",
                    "name": "m1",
                    "outcome": "killed",
                    "killers": ["k1"],
                    "run": "7",
                }
            ],
            "moved": ["scripts/a.py"],
        }
        changed = {
            "schema": CARRY_SCHEMA,
            "base_run": "300",
            "why": "run 300 carries nothing",
            "files": {
                "scripts/a.py": {
                    "carried": False,
                    "why": "the read set is the whole tree, and docs/x.md changed",
                }
            },
            "carried": [],
            "moved": [],
        }
        table = [
            (
                '1172:41 replace "base_run" with ""',
                lambda: self.carried(release=False),
                (
                    refused_carry(
                        "not admitted: this run is not the release run, so nothing carries"
                    ),
                    [],
                ),
            ),
            (
                '1172:53 replace "" with "XX"',
                lambda: self.carried(base_run=""),
                (refused_carry("no base: the base step chose no run"), []),
            ),
            (
                '1172:57 replace "why" with ""',
                lambda: self.carried(on="m0"),
                (
                    refused_carry("no base: main moved: the base's plan is on m0, this plan on m1"),
                    [],
                ),
            ),
            (
                '1187:26 replace "base" with ""',
                lambda: self.carried(beside=False),
                (refused_carry("no base: git.diff cannot be read beside a plan"), []),
            ),
            ('1206:43 replace "{}" with ""', self.carried, (whole, [])),
            ('1207:46 replace "{}" with ""', self.carried, (whole, [])),
            (
                '1212:65 replace "no base" with ""',
                lambda: self.carried(inputs=False),
                (
                    refused_carry(
                        "no base: scripts/mutation-inputs.json cannot be read: "
                        "Expecting value: line 1 column 1 (char 0)"
                    ),
                    [],
                ),
            ),
            ("1216:36 replace 0 with 1", lambda: self.carried(diff=HEAD_ONLY), (changed, [])),
            ('1235:14 replace "why" with ""', self.carried, (whole, [])),
        ]
        self.judge(table)
        examined("K08 cases", table)

    def test_file_refusal_names_each_reason_exactly(self):
        def refusal(klass="scripts", here=None, there=None, problems=(), judged=None):
            same = {"scripts": {"jobs": "1", "runner": "2"}}
            return self.mc.file_refusal(
                "scripts/a.py",
                [{"name": "m1"}],
                klass,
                here or same,
                there or same,
                {},
                {},
                list(problems),
                {"scripts": {"rule": "closure"}},
                judged or {},
                self.scratch,
                self.mc.Changes(),
            )

        table = [
            (
                "1248:16 replace return value with return None",
                lambda: refusal(klass="rust"),
                "its class rust is never carried",
            ),
            (
                "1251:73 replace != with ==",
                lambda: refusal(
                    here={"scripts": {"jobs": "1", "runner": "2"}},
                    there={"scripts": {"jobs": "1", "runner": "3"}},
                ),
                "the key of the scripts class differs at the base: runner",
            ),
            (
                "1255:16 replace return value with return None",
                lambda: refusal(problems=["p1"]),
                "the map's census at the head finds a problem: p1",
            ),
            (
                "1267:20 replace return value with return None",
                lambda: refusal(judged={"m1": [("killed", [], "7"), ("killed", [], "8")]}),
                "m1 was judged 2 times at the base",
            ),
        ]
        self.judge(table)
        examined("K09 cases", table)

    def test_carry_verb_writes_the_document_and_the_plan_exactly(self):
        def moving():
            return {
                "schema": CARRY_SCHEMA,
                "base_run": "7",
                "why": "carried 1 from run 7",
                "files": {
                    "scripts/a.py": {
                        "carried": True,
                        "why": "carried 1 from run 7, by its manifest",
                    },
                    "scripts/b.py": {"carried": False, "why": "its map entry differs at the base"},
                },
                "carried": [],
                "moved": ["scripts/a.py"],
            }

        def verb(outcome, blocked=False):
            place = Path(tempfile.mkdtemp(dir=self.scratch))
            plan = place / "plan.json"
            written(plan, PLAN_TEXT)
            if blocked:
                (place / "carry.json").mkdir()

            def stand_in(args, reader, classify, census):
                if outcome is None:
                    raise ValueError("boom")
                return outcome()

            args = self.mc.argparse.Namespace(plan=str(plan))
            with mock.patch.object(self.mc, "carry", stand_in):
                code, lines = printed(lambda: self.mc.carry_verb(args, None, None, None))
            out = place / "carry.json"
            kept = json.loads(out.read_text(encoding="utf-8")) if out.is_file() else None
            return code, lines, kept, plan.read_text(encoding="utf-8"), str(out)

        def failed():
            code, lines, kept, plan, _out = verb(None)
            return code, lines, kept, plan

        def blocked():
            code, lines, kept, plan, out = verb(moving, blocked=True)
            return (
                code,
                lines
                == [
                    "carry: carry.json cannot be written, so the plan is left whole: "
                    f"[Errno 21] Is a directory: '{out}'"
                ],
                kept,
                plan,
            )

        def moved():
            code, lines, kept, plan, _out = verb(moving)
            return code, lines, kept, plan

        error = "an error inside carry: ValueError: boom"
        raised = (0, [f"carry: {error}"], refused_carry(error), PLAN_TEXT)
        left = (0, True, None, PLAN_TEXT)
        kept = moving()
        kept.pop("moved")
        rewritten = (
            0,
            [
                "carry: carried 1 from run 7",
                "carry: scripts/a.py: carried 1 from run 7, by its manifest",
                "carry: scripts/b.py: runs whole: its map entry differs at the base",
            ],
            kept,
            MOVED_PLAN,
        )
        table = [
            ('1284:13 replace "schema" with ""', failed, raised),
            ('1285:13 replace "base_run" with ""', failed, raised),
            ('1285:25 replace "" with "XX"', failed, raised),
            ('1288:13 replace "carried" with ""', failed, raised),
            ("1295:16 replace 0 with 1", blocked, left),
            ("1295:16 replace return value with return None", blocked, left),
            ("1306:68 replace 2 with 3", moved, rewritten),
            ('1306:73 replace "\\n" with ""', moved, rewritten),
            ("1307:12 replace return value with return None", moved, rewritten),
        ]
        self.judge(table)
        examined("K10 cases", table)


# Each member of the allowlists the kill table lists, by its line and column.
FS_METHOD_MEMBERS = [
    ("463:9", "read_bytes"),
    ("464:9", "write_text"),
    ("465:9", "write_bytes"),
    ("466:9", "open"),
    ("467:9", "exists"),
    ("468:9", "is_file"),
    ("469:9", "is_dir"),
    ("470:9", "is_symlink"),
    ("471:9", "stat"),
    ("472:9", "lstat"),
    ("474:9", "glob"),
    ("475:9", "rglob"),
    ("476:9", "walk"),
    ("477:9", "mkdir"),
    ("478:9", "rmdir"),
    ("479:9", "unlink"),
    ("480:9", "touch"),
    ("481:9", "chmod"),
    ("482:9", "samefile"),
    ("483:9", "readlink"),
    ("484:9", "symlink_to"),
    ("485:9", "hardlink_to"),
    ("486:9", "owner"),
    ("487:9", "group"),
]
OS_FS_MEMBERS = [
    ("492:9", "listdir"),
    ("493:9", "scandir"),
    ("494:9", "walk"),
    ("495:9", "fwalk"),
    ("496:9", "stat"),
    ("497:9", "lstat"),
    ("498:9", "remove"),
    ("499:9", "unlink"),
    ("500:9", "rmdir"),
    ("501:9", "removedirs"),
    ("502:9", "mkdir"),
    ("503:9", "makedirs"),
    ("504:9", "rename"),
    ("505:9", "renames"),
    ("506:9", "replace"),
    ("507:9", "chdir"),
    ("508:9", "chmod"),
    ("509:9", "chown"),
    ("510:9", "link"),
    ("511:9", "symlink"),
    ("512:9", "readlink"),
    ("513:9", "access"),
    ("514:9", "open"),
    ("515:9", "truncate"),
    ("516:9", "utime"),
]
OS_PATH_MEMBERS = [
    ("520:6", "exists"),
    ("520:16", "lexists"),
    ("520:27", "isfile"),
    ("520:37", "isdir"),
    ("520:46", "islink"),
    ("520:56", "ismount"),
    ("520:67", "getsize"),
    ("520:78", "samefile"),
    ("521:8", "getmtime"),
    ("521:20", "getatime"),
    ("521:32", "getctime"),
]
READ_X = ({"scripts/x"}, [])


class CensusAllowlists(Base):
    """K11 to K13: each allowlisted name is read as the census reads it."""

    def census(self, text):
        """(the paths, the refusals) the census finds in `text` as scripts/a.py."""
        member = self.mc.Member("scripts/a.py", text, "scripts")
        return member.reads, member.whole

    def test_each_path_method_member_is_a_read(self):
        table = [
            (
                f'{place} replace "{name}" with ""',
                lambda name=name: self.census(f"from pathlib import Path\nPath('x').{name}()\n"),
                READ_X,
            )
            for place, name in FS_METHOD_MEMBERS
        ]
        self.judge(table)
        examined("K11 cases", table)

    def test_each_os_call_member_is_a_read(self):
        table = [
            (
                f'{place} replace "{name}" with ""',
                lambda name=name: self.census(f"import os\nos.{name}('x')\n"),
                READ_X,
            )
            for place, name in OS_FS_MEMBERS
        ]
        table += [
            (
                f'{place} replace "{name}" with ""',
                lambda name=name: self.census(f"import os\nos.path.{name}('x')\n"),
                READ_X,
            )
            for place, name in OS_PATH_MEMBERS
        ]
        table.append(
            (
                "521:5 replace | with &",
                lambda: self.census("import os\nos.path.getmtime('x')\n"),
                READ_X,
            )
        )
        self.judge(table)
        examined("K12 cases", table)

    def test_each_process_import_type_join_and_same_member(self):
        def started(name):
            return set(), [f"scripts/a.py:2: it starts a process by os.{name}"]

        def imports(name):
            return set(), [f"scripts/a.py:1: it imports {name}"]

        path = "from pathlib import Path\n"
        table = [
            (
                '524:34 replace "popen" with ""',
                lambda: self.census("import os\nos.popen('ls')\n"),
                started("popen"),
            ),
            (
                '524:43 replace "fork" with ""',
                lambda: self.census("import os\nos.fork()\n"),
                started("fork"),
            ),
            (
                '524:51 replace "forkpty" with ""',
                lambda: self.census("import os\nos.forkpty()\n"),
                started("forkpty"),
            ),
            (
                '524:62 replace "posix_spawn" with ""',
                lambda: self.census("import os\nos.posix_spawn('a', [], {})\n"),
                started("posix_spawn"),
            ),
            (
                '524:77 replace "posix_spawnp" with ""',
                lambda: self.census("import os\nos.posix_spawnp('a', [], {})\n"),
                started("posix_spawnp"),
            ),
            (
                '525:42 replace "mmap" with ""',
                lambda: self.census("import mmap\n"),
                imports("mmap"),
            ),
            (
                '525:61 replace "posix" with ""',
                lambda: self.census("import posix\n"),
                imports("posix"),
            ),
            ('525:70 replace "_os" with ""', lambda: self.census("import _os\n"), imports("_os")),
            (
                '526:33 replace "PurePath" with ""',
                lambda: self.census("from pathlib import PurePath\nPurePath('x').read_text()\n"),
                READ_X,
            ),
            (
                '526:45 replace "PosixPath" with ""',
                lambda: self.census("from pathlib import PosixPath\nPosixPath('x').read_text()\n"),
                READ_X,
            ),
            (
                '526:58 replace "PurePosixPath" with ""',
                lambda: self.census(
                    "from pathlib import PurePosixPath\nPurePosixPath('x').read_text()\n"
                ),
                READ_X,
            ),
            (
                '529:20 replace "joinpath" with ""',
                lambda: self.census(path + "Path('a/b').joinpath(name).read_text()\n"),
                ({"scripts/a"}, []),
            ),
            (
                '529:32 replace "with_name" with ""',
                lambda: self.census(path + "Path('a/b').with_name('c').read_text()\n"),
                ({"scripts/a"}, []),
            ),
            (
                '529:45 replace "with_suffix" with ""',
                lambda: self.census(path + "Path('a/b').with_suffix('.x').read_text()\n"),
                ({"scripts/a"}, []),
            ),
            (
                '529:60 replace "with_stem" with ""',
                lambda: self.census(path + "Path('a/b').with_stem('c').read_text()\n"),
                ({"scripts/a"}, []),
            ),
            (
                '538:30 replace "absolute" with ""',
                lambda: self.census(path + "Path('x').absolute().read_text()\n"),
                READ_X,
            ),
            (
                '538:42 replace "expanduser" with ""',
                lambda: self.census(path + "Path('x').expanduser().read_text()\n"),
                READ_X,
            ),
        ]
        self.judge(table)
        examined("K13 cases", table)


def expression(text):
    """The expression node `text` parses to."""
    return ast.parse(text, mode="eval").body


class CensusValues(Base):
    """K14 and K15: each constant value the census computes, and its path helpers."""

    def setUp(self):
        super().setUp()
        self.member = self.mc.Member("scripts/a.py", "", "scripts")
        self.cwd = {"c": self.mc.Value("cwd", "scripts")}

    def value(self, text, env=None, temps=()):
        return self.member.value(expression(text), env or {}, set(temps))

    def test_each_constant_value_is_computed_exactly(self):
        value, member, Value = self.value, self.member, self.mc.Value

        def tempish(text, temps):
            return member.tempish(expression(text), set(temps))

        def constant(text, temps=()):
            return member.constant(expression(text), {}, set(temps))

        temp, here = Value("temp"), Value("path", "scripts/a.py")
        table = [
            ("613:53 replace in with not in", lambda: tempish("a", {"a"}), True),
            ("614:44 replace and with or", lambda: tempish("b", set()), False),
            (
                "621:20 replace return value with return None",
                lambda: value("'a'"),
                Value("text", "a"),
            ),
            ('631:30 replace "temp" with ""', lambda: value("t / 'a'", temps={"t"}), temp),
            (
                '632:38 replace "cwd" with ""',
                lambda: value("c / 'a'", env=self.cwd),
                Value("path", "scripts/a"),
            ),
            ("632:45 replace and with or", lambda: value("'a' / 'b'"), None),
            (
                '632:63 replace "text" with ""',
                lambda: value("__file__ / 'x'"),
                Value("path", "scripts/a.py/x"),
            ),
            ("637:30 replace or with and", lambda: value("name.attr"), None),
            (
                '637:60 replace "cwd" with ""',
                lambda: value("c.parent", env=self.cwd),
                Value("path", "."),
            ),
            ('637:67 replace "temp" with ""', lambda: value("t.x", temps={"t"}), temp),
            ('639:30 replace "temp" with ""', lambda: value("t.x", temps={"t"}), temp),
            (
                "640:24 replace return value with return None",
                lambda: value("t.x", temps={"t"}),
                temp,
            ),
            ('640:30 replace "temp" with ""', lambda: value("t.x", temps={"t"}), temp),
            (
                "651:23 replace is not with is",
                lambda: value("__file__.parents[1]"),
                Value("path", "."),
            ),
            ("652:17 replace and with or", lambda: value("'a'.parents[0]"), None),
            ("653:17 replace and with or", lambda: value("name.parents[0]"), None),
            ("654:17 replace and with or", lambda: value("__file__.parents[n]"), None),
            (
                "656:87 replace 1 with 2",
                lambda: value("__file__.parents[0]"),
                Value("path", "scripts"),
            ),
            ("765:16 replace return value with return None", lambda: constant("__file__"), here),
            ("765:31 replace is not with is", lambda: constant("__file__"), here),
            ("765:43 replace and with or", lambda: constant("'a'"), None),
            ("765:58 replace in with not in", lambda: constant("__file__"), here),
            ('765:62 replace "path" with ""', lambda: constant("__file__"), here),
            ('765:70 replace "temp" with ""', lambda: constant("t", temps={"t"}), temp),
        ]
        self.judge(table)
        examined("K14 cases", table)

    def test_paths_join_and_name_exactly(self):
        mc, member, Value = self.mc, self.member, self.mc.Value

        def fs_arg(text, env=None):
            return member.fs_arg(expression(text), env or {}, set())

        held = Value("temp")

        def own_temp():
            found = member.fs_arg(expression("t"), {"t": held}, set())
            return found, found is held

        here = Value("path", "scripts/a.py")
        table = [
            ('535:59 replace "tempfile." with ""', lambda: mc.is_temp_call("other.mkdtemp"), False),
            ("535:80 replace == with !=", lambda: mc.is_temp_call("mkdtemp"), True),
            (
                '559:12 replace "" with "XX"',
                lambda: mc.dotted(expression("Path('x').read_text")),
                "",
            ),
            ("565:13 replace == with !=", lambda: mc.joined("a", "b"), "a/b"),
            ('565:16 replace ".." with ""', lambda: mc.joined("a", "..", ".."), "."),
            ("565:21 replace or with and", lambda: mc.joined("a", "..", ".."), "."),
            ("565:47 replace or with and", lambda: mc.joined("a", "/x"), "."),
            ('566:16 replace "." with ""', lambda: mc.joined("a", "..", ".."), "."),
            ("566:16 replace return value with return None", lambda: mc.joined("a", "/x"), "."),
            ("845:30 replace and with or", lambda: fs_arg("__file__"), here),
            ('845:48 replace "text" with ""', lambda: fs_arg("'x'"), Value("path", "scripts/x")),
            (
                "846:20 replace return value with return None",
                lambda: fs_arg("'x'"),
                Value("path", "scripts/x"),
            ),
            ("846:33 replace is with is not", lambda: fs_arg("'x'"), Value("path", "scripts/x")),
            ('846:52 replace "path" with ""', lambda: fs_arg("'x'"), Value("path", "scripts/x")),
            ('847:48 replace "temp" with ""', own_temp, (Value("temp"), False)),
            ("848:20 replace return value with return None", own_temp, (Value("temp"), False)),
            ('848:26 replace "temp" with ""', own_temp, (Value("temp"), False)),
            ("849:16 replace return value with return None", lambda: fs_arg("__file__"), here),
        ]
        self.judge(table)
        examined("K15 cases", table)


class CensusCalls(Base):
    """K16 to K22: the value of each call, and what each call and statement adds or refuses."""

    def setUp(self):
        super().setUp()
        self.member = self.mc.Member("scripts/a.py", "", "scripts")

    def value(self, text):
        return self.member.value(expression(text), {}, set())

    def census(self, text):
        return self.mc.Member("scripts/a.py", text, "scripts")

    def test_call_values_of_types_and_the_working_directory(self):
        value, Value = self.value, self.mc.Value
        temp, cwd = Value("temp"), Value("cwd", "scripts")
        x = Value("path", "scripts/x")
        table = [
            (
                "667:20 replace return value with return None",
                lambda: value("tempfile.mkdtemp()"),
                temp,
            ),
            ('667:26 replace "temp" with ""', lambda: value("tempfile.mkdtemp()"), temp),
            ('668:20 replace "cwd" with ""', lambda: value("Path.cwd()"), cwd),
            ("668:26 replace or with and", lambda: value("Path.cwd()"), cwd),
            ('668:51 replace "os.getcwdb" with ""', lambda: value("os.getcwdb()"), cwd),
            ("669:20 replace return value with return None", lambda: value("os.getcwd()"), cwd),
            ("673:24 replace return value with return None", lambda: value("Path()"), cwd),
            ('673:30 replace "cwd" with ""', lambda: value("Path()"), cwd),
            ("673:55 replace in with not in", lambda: value("Path()"), cwd),
            ('677:30 replace "text" with ""', lambda: value("Path('x')"), x),
            ("677:37 replace and with or", lambda: value("str('x')"), Value("text", "x")),
            ("677:46 replace in with not in", lambda: value("Path('x')"), x),
            ("678:31 replace in with not in", lambda: value("Path('.')"), cwd),
            ('678:35 replace "" with "XX"', lambda: value("Path('')"), cwd),
            ('678:39 replace "." with ""', lambda: value("Path('.')"), cwd),
            ("679:28 replace return value with return None", lambda: value("Path('')"), cwd),
            ('679:34 replace "cwd" with ""', lambda: value("Path('')"), cwd),
            ("680:24 replace return value with return None", lambda: value("Path('x')"), x),
            ('680:30 replace "path" with ""', lambda: value("Path('x')"), x),
        ]
        self.judge(table)
        examined("K16 cases", table)

    def test_call_values_of_os_path_joins_and_dirnames(self):
        value, Value = self.value, self.mc.Value
        three = "os.path.join(Path(__file__).parent, 'a', 'b')"
        made = "os.path.join(tempfile.mkdtemp(), 'a')"
        here = "os.path.dirname(__file__)"
        ab, temp, up = Value("path", "scripts/a/b"), Value("temp"), Value("path", "scripts")
        table = [
            ("682:17 replace == with !=", lambda: value(three), ab),
            ('682:20 replace "os.path.join" with ""', lambda: value(three), ab),
            ("682:35 replace and with or", lambda: value("os.path.join(name, 'a')"), None),
            ("682:53 replace not in with in", lambda: value(three), ab),
            ('683:16 replace "temp" with ""', lambda: value(made), temp),
            ("683:23 replace in with not in", lambda: value(made), temp),
            ("684:24 replace return value with return None", lambda: value(made), temp),
            ('684:30 replace "temp" with ""', lambda: value(made), temp),
            ("685:21 replace 0 with 1", lambda: value(three), ab),
            ("685:29 replace in with not in", lambda: value(three), ab),
            ('685:33 replace "path" with ""', lambda: value(three), ab),
            (
                '685:41 replace "cwd" with ""',
                lambda: value("os.path.join(Path.cwd(), 'a')"),
                Value("path", "scripts/a"),
            ),
            ("685:48 replace and with or", lambda: value("os.path.join('a', 'b')"), None),
            ("685:65 replace == with !=", lambda: value(three), ab),
            ('685:68 replace "text" with ""', lambda: value(three), ab),
            (
                "685:91 replace 1 with 2",
                lambda: value("os.path.join(Path(__file__).parent, Path.cwd(), 'b')"),
                None,
            ),
            ("686:24 replace return value with return None", lambda: value(three), ab),
            ('686:30 replace "path" with ""', lambda: value(three), ab),
            ("686:50 replace 0 with 1", lambda: value(three), ab),
            ("686:86 replace 1 with 2", lambda: value(three), ab),
            ("688:17 replace == with !=", lambda: value(here), up),
            ('688:20 replace "os.path.dirname" with ""', lambda: value(here), up),
            ("688:38 replace and with or", lambda: value("os.path.basename(__file__)"), None),
            ("688:52 replace == with !=", lambda: value(here), up),
            ("688:55 replace 1 with 2", lambda: value(here), up),
            ("688:66 replace 0 with 1", lambda: value(here), up),
            ("688:69 replace is not with is", lambda: value(here), up),
            ("689:20 replace return value with return None", lambda: value(here), up),
            ('689:26 replace "path" with ""', lambda: value(here), up),
            ("689:46 replace 0 with 1", lambda: value(here), up),
            ('689:55 replace ".." with ""', lambda: value(here), up),
            ("689:70 replace 0 with 1", lambda: value(here), up),
            ("689:78 replace == with !=", lambda: value(here), up),
            ('689:81 replace "path" with ""', lambda: value(here), up),
        ]
        self.judge(table)
        examined("K17 cases", table)

    def test_call_values_of_methods_on_a_path(self):
        value, Value = self.value, self.mc.Value
        joins = "Path('a').joinpath('b', 'c')"
        sibling = "Path('a/b').with_name('c')"
        made = "tempfile.mkdtemp().joinpath('x')"
        abc, temp, a = Value("path", "scripts/a/b/c"), Value("temp"), Value("path", "scripts/a")
        table = [
            (
                '692:66 replace "cwd" with ""',
                lambda: value("Path.cwd().joinpath('x')"),
                Value("path", "scripts/x"),
            ),
            ('692:73 replace "temp" with ""', lambda: value(made), temp),
            ('694:33 replace "temp" with ""', lambda: value(made), temp),
            ("695:24 replace return value with return None", lambda: value(made), temp),
            ('695:30 replace "temp" with ""', lambda: value(made), temp),
            ("696:29 replace and with or", lambda: value("Path('a').resolve('x')"), None),
            ("699:22 replace == with !=", lambda: value(joins), abc),
            ('699:25 replace "joinpath" with ""', lambda: value(joins), abc),
            ("700:17 replace and with or", lambda: value(sibling), a),
            ("701:17 replace and with or", lambda: value(sibling), a),
            ("701:29 replace is not with is", lambda: value(joins), abc),
            (
                "701:41 replace and with or",
                lambda: value("Path('a').joinpath(name)"),
                Value("path", "scripts"),
            ),
            ("701:54 replace == with !=", lambda: value(joins), abc),
            ('701:57 replace "text" with ""', lambda: value(joins), abc),
            ("703:24 replace return value with return None", lambda: value(joins), abc),
            ('703:30 replace "path" with ""', lambda: value(joins), abc),
            ("704:21 replace in with not in", lambda: value(sibling), a),
            ("706:24 replace return value with return None", lambda: value(sibling), a),
            ('706:30 replace "path" with ""', lambda: value(sibling), a),
            ('706:60 replace ".." with ""', lambda: value(sibling), a),
        ]
        self.judge(table)
        examined("K18 cases", table)

    def test_each_refusal_of_a_loader_or_the_environment(self):
        def seen(text):
            member = self.census(text)
            return member.whole, member.imports, member.loads, member.search

        def refused(why, line=1):
            return [f"scripts/a.py:{line}: {why}"], set(), set(), set()

        nothing = ([], set(), set(), set())
        script = "Path(__file__).parent / 'x.py'"
        loaded = ([], set(), {"scripts/x.py"}, set())
        by_name = "runpy.run_module(name)\n"
        table = [
            ('782:27 replace "." with ""', lambda: seen("f()\n"), nothing),
            (
                "786:25 replace in with not in",
                lambda: seen("__import__('json')\n"),
                ([], {"json"}, set(), set()),
            ),
            (
                '786:29 replace "__import__" with ""',
                lambda: seen("__import__('json')\n"),
                ([], {"json"}, set(), set()),
            ),
            (
                '786:43 replace "importlib.import_module" with ""',
                lambda: seen("import importlib\nimportlib.import_module('json')\n"),
                ([], {"importlib", "json"}, set(), set()),
            ),
            (
                '791:38 replace "it reads the environment" with ""',
                lambda: seen("os.getenv('X')\n"),
                refused("it reads the environment"),
            ),
            (
                '792:20 replace "os" with ""',
                lambda: seen("os.execv('a', [])\n"),
                refused("it starts a process by os.execv"),
            ),
            ("792:25 replace and with or", lambda: seen("os.listdir('x')\n"), nothing),
            ('792:68 replace "exec" with ""', lambda: seen("os.listdir('x')\n"), nothing),
            ('792:76 replace "spawn" with ""', lambda: seen("os.stat('x')\n"), nothing),
            (
                "799:44 replace != with ==",
                lambda: seen("sys.path.insert(0, Path(__file__).parent)\n"),
                ([], set(), set(), {"scripts"}),
            ),
            (
                '800:42 replace "it puts a path held in data on sys.p... with ""',
                lambda: seen("sys.path.insert(0, name)\n"),
                refused("it puts a path held in data on sys.path"),
            ),
            (
                '803:48 replace "run_path" with ""',
                lambda: seen(f"runpy.run_path({script})\n"),
                loaded,
            ),
            (
                '803:60 replace "SourceFileLoader" with ""',
                lambda: seen(f"SourceFileLoader('m', {script})\n"),
                loaded,
            ),
            (
                "806:30 replace or with and",
                lambda: seen("runpy.run_path(name)\n"),
                refused("it loads code by runpy.run_path from a path held in data"),
            ),
            (
                "806:54 replace or with and",
                lambda: seen("runpy.run_path('x')\n"),
                refused("it loads code by runpy.run_path from a path held in data"),
            ),
            (
                '806:71 replace "." with ""',
                lambda: seen("ROOT_DIR = Path(__file__).parents[1]\nrunpy.run_path(ROOT_DIR)\n"),
                refused("it loads code by runpy.run_path from a path held in data", 2),
            ),
            (
                "807:24 replace return value with return None",
                lambda: seen("importlib.util.spec_from_file_location('m', name)\n"),
                refused(
                    "it loads code by importlib.util.spec_from_file_location from a path held "
                    "in data"
                ),
            ),
            (
                '810:20 replace "run_module" with ""',
                lambda: seen("runpy.run_module('m')\n"),
                ([], {"m"}, set(), set()),
            ),
            (
                "811:31 replace 0 with 1",
                lambda: seen("runpy.run_module('m')\n"),
                ([], {"m"}, set(), set()),
            ),
            (
                "812:48 replace and with or",
                lambda: seen(by_name),
                refused("it runs a module named in data"),
            ),
            (
                "815:20 replace return value with return None",
                lambda: seen(by_name),
                refused("it runs a module named in data"),
            ),
            (
                '815:38 replace "it runs a module named in data" with ""',
                lambda: seen(by_name),
                refused("it runs a module named in data"),
            ),
        ]
        self.judge(table)
        examined("K19 cases", table)

    def test_each_file_call_reads_its_paths_exactly(self):
        def seen(text):
            member = self.census(text)
            return member.reads, member.whole

        d = "Path(__file__).parent / 'd'"
        x, read_d = ({"scripts/x"}, []), ({"scripts/d"}, [])
        table = [
            ("820:17 replace == with !=", lambda: seen("os.listdir('x')\n"), x),
            ('820:20 replace "os" with ""', lambda: seen("os.listdir('x')\n"), x),
            ("820:25 replace and with or", lambda: seen(f"os.fspath({d})\n"), (set(), [])),
            ("820:35 replace in with not in", lambda: seen("os.listdir('x')\n"), x),
            ("820:44 replace or with and", lambda: seen("os.listdir('x')\n"), x),
            ('820:64 replace "os.path." with ""', lambda: seen("os.isfile('x')\n"), (set(), [])),
            ("820:76 replace and with or", lambda: seen("os.path.basename('x')\n"), (set(), [])),
            ("820:85 replace in with not in", lambda: seen("os.path.isfile('x')\n"), x),
            (
                "821:35 replace 2 with 3",
                lambda: seen("os.rename('a', 'b', name)\n"),
                ({"scripts/a", "scripts/b"}, []),
            ),
            (
                "828:50 replace is with is not",
                lambda: seen("'x'.read_text()\n"),
                (set(), ["scripts/a.py:1: it calls read_text on a path held in data"]),
            ),
            (
                '828:78 replace "text" with ""',
                lambda: seen("'x'.read_text()\n"),
                (set(), ["scripts/a.py:1: it calls read_text on a path held in data"]),
            ),
            ('833:71 replace "os.fspath" with ""', lambda: seen(f"os.fspath({d})\n"), (set(), [])),
            ("834:17 replace continue with break", lambda: seen(f"f(name, {d})\n"), read_d),
            (
                '835:30 replace "cwd" with ""',
                lambda: seen("f(Path.cwd())\n"),
                (set(), ["scripts/a.py:1: it passes the working directory to f"]),
            ),
            ("837:39 replace and with or", lambda: seen(f"f({d})\n"), read_d),
            ("839:29 replace == with !=", lambda: seen(f"f({d})\n"), read_d),
            ('839:32 replace "path" with ""', lambda: seen(f"f({d})\n"), read_d),
        ]
        self.judge(table)
        examined("K20 cases", table)

    def test_each_child_process_is_allowed_or_refused_exactly(self):
        def seen(text):
            member = self.census(text)
            return member.whole, member.loads

        def refused(why):
            return [f"scripts/a.py:1: {why}"], set()

        b = "Path(__file__).parent / 'b.py'"
        p = "Path(__file__).parent"
        other = refused("it starts a process other than the interpreter")
        b_py = ([], {"scripts/b.py"})
        in_data = "subprocess.run([sys.executable, name])\n"
        no_script = "subprocess.run([sys.executable, '-c', 'x'])\n"
        table = [
            (
                "854:56 replace or with and",
                lambda: seen("subprocess.run('ls')\n"),
                refused("it starts a process by subprocess.run"),
            ),
            (
                "855:20 replace return value with return None",
                lambda: seen("subprocess.run([])\n"),
                refused("it starts a process by subprocess.run"),
            ),
            ("858:47 replace and with or", lambda: seen("subprocess.run(['ls'])\n"), other),
            (
                "858:65 replace in with not in",
                lambda: seen(f"subprocess.run(['python3', {b}])\n"),
                b_py,
            ),
            (
                '858:69 replace "python3" with ""',
                lambda: seen(f"subprocess.run(['python3', {b}])\n"),
                b_py,
            ),
            (
                '858:80 replace "python" with ""',
                lambda: seen(f"subprocess.run(['python', {b}])\n"),
                b_py,
            ),
            (
                "860:28 replace or with and",
                lambda: seen(f"subprocess.run([sys.executable, {b}], shell=True)\n"),
                other,
            ),
            (
                '860:44 replace "shell" with ""',
                lambda: seen(f"subprocess.run([sys.executable, {b}], shell=True)\n"),
                other,
            ),
            (
                '861:38 replace "it starts a process other than the i... with ""',
                lambda: seen("subprocess.run(['ls'])\n"),
                other,
            ),
            (
                "866:24 replace return value with return None",
                lambda: seen(in_data),
                refused("it hands the interpreter an argument held in data"),
            ),
            (
                '866:42 replace "it hands the interpreter an argument... with ""',
                lambda: seen(in_data),
                refused("it hands the interpreter an argument held in data"),
            ),
            (
                "867:37 replace and with or",
                lambda: seen(f"subprocess.run([sys.executable, {b}, {p} / 'd'])\n"),
                b_py,
            ),
            (
                '867:61 replace ".py" with ""',
                lambda: seen(f"subprocess.run([sys.executable, {p} / 'd', {b}])\n"),
                b_py,
            ),
            (
                "867:68 replace and with or",
                lambda: seen(no_script),
                refused("it runs the interpreter on no constant script"),
            ),
            (
                "870:20 replace return value with return None",
                lambda: seen(no_script),
                refused("it runs the interpreter on no constant script"),
            ),
            (
                '870:38 replace "it runs the interpreter on no consta... with ""',
                lambda: seen(no_script),
                refused("it runs the interpreter on no constant script"),
            ),
        ]
        self.judge(table)
        examined("K21 cases", table)

    def test_the_walk_binds_scopes_imports_and_assignments(self):
        Value = self.mc.Value

        def seen(text):
            member = self.census(text)
            return member.whole, member.reads, member.relative, member.module_env

        table = [
            (
                "588:62 replace True with False",
                lambda: seen("from pathlib import Path\nP = Path(__file__).parent\n"),
                ([], set(), set(), {"P": Value("path", "scripts")}),
            ),
            (
                "593:71 replace False with True",
                lambda: seen("def f():\n    name = 'a.txt'\nopen(name)\n"),
                (["scripts/a.py:3: open on a path held in data"], set(), set(), {}),
            ),
            (
                "712:28 replace is not with is",
                lambda: seen("def f(p=open('x')):\n    pass\n"),
                ([], {"scripts/x"}, set(), {}),
            ),
            (
                "725:61 replace and with or",
                lambda: seen("with open('a.txt') as h:\n    open(h)\n"),
                (["scripts/a.py:2: open on a path held in data"], {"scripts/a.txt"}, set(), {}),
            ),
            (
                "740:47 replace or with and",
                lambda: seen("from . import x\n"),
                ([], set(), {""}, {}),
            ),
            (
                '740:50 replace "" with "XX"',
                lambda: seen("from . import x\n"),
                ([], set(), {""}, {}),
            ),
            (
                '747:31 replace "it reads the environment" with ""',
                lambda: seen("os.environ['X']\n"),
                (["scripts/a.py:1: it reads the environment"], set(), set(), {}),
            ),
            (
                '753:35 replace "it sets sys.path to a value held in ... with ""',
                lambda: seen("sys.path = [name]\n"),
                (["scripts/a.py:1: it sets sys.path to a value held in data"], set(), set(), {}),
            ),
        ]
        self.judge(table)
        examined("K22 cases", table)


def digest(text):
    """The sha256 hex digest of `text` as UTF-8."""
    return hashlib.sha256(text.encode()).hexdigest()


ROWS = (
    "X = 1\n"
    "def f():\n"
    "    k()\n"
    "    g()\n"
    "    g()\n"
    "def g():\n"
    "    pass\n"
    "def k():\n"
    "    pass\n"
    "def z():\n"
    "    pass\n"
)


class ClassDigests(Base):
    """K23 and K24: a job's lines, each class's key and the runner digest."""

    def test_job_lines_and_the_class_digests(self):
        jobs = self.mc.job_lines
        files = {
            "w.yml": "jobs:\n  a:\n    run: x\n  b:\n    run: y\n",
            "r1.py": "one",
            "r2.py": "two",
        }
        inputs = {
            "workflow": "w.yml",
            "leg_jobs": ["a", "b"],
            "runner": ["r1.py", "r2.py"],
            "runner_digest": {"file": "rows.py", "named_by": "named.py", "module": "m"},
        }
        runner = "\n".join(
            [
                "r1.py\0" + hashlib.sha256(b"one").hexdigest(),
                "r2.py\0" + hashlib.sha256(b"two").hexdigest(),
            ]
        )
        key = {
            "jobs": digest("\n".join(["  a:", "    run: x", "  b:", "    run: y"])),
            "runner": digest(runner),
            "rows": None,
        }

        def keyed():
            return self.mc.class_key(files.get, inputs)

        table = [
            (
                "367:25 replace False with True",
                lambda: jobs("    stray: x\n  j:\n    run: a\n", "j"),
                ["  j:", "    run: a"],
            ),
            (
                "371:37 replace == with !=",
                lambda: jobs("jobs:\n  j:\n    run: a\n  k:\n    run: b\n", "j"),
                ["  j:", "    run: a"],
            ),
            (
                "375:18 replace 1 with 2",
                lambda: jobs("  j:\n    a: 1\n y\n    b: 2\n", "j"),
                ["  j:", "    a: 1", " y", "    b: 2"],
            ),
            (
                "376:22 replace False with True",
                lambda: jobs("  j:\n    a: 1\ntop: 1\n    b: 2\n", "j"),
                ["  j:", "    a: 1"],
            ),
            ('418:60 replace "\\n" with ""', keyed, key),
            ('425:9 replace "jobs" with ""', keyed, key),
            ('426:9 replace "runner" with ""', keyed, key),
            ('426:60 replace "\\n" with ""', keyed, key),
            ('427:9 replace "rows" with ""', keyed, key),
        ]
        self.judge(table)
        examined("K23 cases", table)

    def test_rows_digest_and_the_inputs_map(self):
        body = ast.parse(ROWS).body
        closed = digest(
            "\n".join(
                [
                    ast.dump(body[0]),
                    "f: " + ast.dump(body[1]),
                    "g: " + ast.dump(body[2]),
                    "k: " + ast.dump(body[3]),
                ]
            )
        )
        inputs = "scripts/mutation-inputs.json"
        schema = "deckstreak.mutation-inputs.v1"

        def inputs_of(document):
            return self.mc.read_inputs(json.dumps(document))

        def wrong_schema():
            return (
                attempt(lambda: inputs_of({"schema": "x", "classes": {}})),
                attempt(lambda: self.mc.read_inputs(None)),
            )

        written(self.scratch / "a.txt", "hello\n")
        table = [
            (
                "386:21 replace or with and",
                lambda: self.mc.rows_digest(None, "m.f()\n", "m"),
                None,
            ),
            (
                "405:13 replace continue with break",
                lambda: self.mc.rows_digest(ROWS, "m.f()\n", "m"),
                closed,
            ),
            (
                "408:78 replace not in with in",
                lambda: self.mc.rows_digest(ROWS, "m.f()\n", "m"),
                closed,
            ),
            (
                '412:27 replace "\\n" with ""',
                lambda: self.mc.rows_digest(ROWS, "m.f()\n", "m"),
                closed,
            ),
            (
                "438:48 replace or with and",
                wrong_schema,
                (
                    ("raised", "NoBase", f"{inputs} is not of the schema {schema}"),
                    (
                        "raised",
                        "NoBase",
                        f"{inputs} cannot be read: Expecting value: line 1 column 1 (char 0)",
                    ),
                ),
            ),
            (
                "443:40 replace or with and",
                lambda: inputs_of({"schema": schema, "classes": {"scripts": None}}),
                ("raised", "NoBase", f"{inputs} holds no complete entry for the scripts class"),
            ),
            (
                "1401:16 replace return value with return None",
                lambda: self.mc.tree_reader(self.scratch)("a.txt"),
                "hello\n",
            ),
        ]
        self.judge(table)
        examined("K24 cases", table)


HEADER = "diff --git a/x b/x\n"
ADDED_ID = hashlib.sha1(b"blob 4\0new\n").hexdigest()[:7]
ADDED = f"{HEADER}new file mode 100644\nindex 0000000..{ADDED_ID}\n@@ -0,0 +1 @@\n+new\n"


class DiffSections(Base):
    """K25 to K27: a diff's sections, a hunk's application and the base's file rebuilt."""

    def section(self, text, path="x", **flags):
        fields = {"added": False, "deleted": False, "binary": False, "new_id": None, **flags}
        return self.mc.Section(path=path, text=text, **fields)

    def test_sections_changes_and_presence(self):
        sections = self.mc.sections
        renamed = "diff --git a/x b/y\nsimilarity index 90%\n"
        unread = "diff --git a/x\n"
        readable = HEADER + "@@ -1 +1 @@\n-a\n+b\n"
        fourth = HEADER + "l1\nl2\nl3\nnew file mode 100644\n"
        first = HEADER + "deleted file mode 100644\n"
        fifth = HEADER + "l1\nl2\nl3\ndeleted file mode 100644\n"
        binary = HEADER + "Binary files a/x and b/x differ\n"

        def changed(base, head):
            found = self.mc.changes(base, head)
            return found.changed, found.binary, found.listings, found.unread

        table = [
            (
                "205:27 replace or with and",
                lambda: sections(unread + readable),
                ({"x": self.section(readable)}, ["diff --git a/x"]),
            ),
            ("205:43 replace 1 with 2", lambda: sections(renamed), ({}, ["diff --git a/x b/y"])),
            ("206:33 replace 0 with 1", lambda: sections(renamed), ({}, ["diff --git a/x b/y"])),
            (
                '206:43 replace "\\n" with ""',
                lambda: sections(renamed),
                ({}, ["diff --git a/x b/y"]),
            ),
            (
                "207:13 replace continue with break",
                lambda: sections(unread + readable),
                ({"x": self.section(readable)}, ["diff --git a/x"]),
            ),
            (
                "213:76 replace 4 with 5",
                lambda: sections(fourth),
                ({"x": self.section(fourth)}, []),
            ),
            (
                "214:80 replace 1 with 2",
                lambda: sections(first),
                ({"x": self.section(first, deleted=True)}, []),
            ),
            ("214:82 replace 4 with 5", lambda: sections(fifth), ({"x": self.section(fifth)}, [])),
            (
                "249:33 replace or with and",
                lambda: changed(binary, ""),
                ({"x"}, {"x"}, set(), []),
            ),
            ("264:23 replace is not with is", lambda: self.mc.present(None, None), True),
        ]
        self.judge(table)
        examined("K25 cases", table)

    def test_each_hunk_applies_or_is_refused_by_name(self):
        def apply(old, text):
            return self.mc.applied(old, self.section(text))

        simple = "@@ -1 +1 @@\n-a\n+b\n"
        no_hunk = ("raised", "NoBase", "x: a hunk of the base's section does not apply")
        no_apply = ("raised", "NoBase", "x: the base's section does not apply")
        marker = "\\ No newline at end of file\n"
        table = [
            ("277:13 replace 0 with 1", lambda: apply("a\n", simple), "b\n"),
            (
                "284:59 replace 2 with 3",
                lambda: apply("a\nb\n", "@@ -2,0 +3 @@\n+c\n"),
                "a\nb\nc\n",
            ),
            ("284:62 replace or with and", lambda: apply("a\n", simple), "b\n"),
            ('284:65 replace "1" with ""', lambda: apply("a\n", simple), "b\n"),
            ("286:23 replace or with and", lambda: apply("a\n", "@@ -5 +5 @@\n-a\n+b\n"), no_hunk),
            ("290:38 replace not with nothing", lambda: apply("a\n", simple), "b\n"),
            (
                '295:29 replace " " with ""',
                lambda: apply("a\nb\n", f"@@ -1,2 +1,2 @@\n-a\n+c\n b\n{marker}"),
                "c\nb",
            ),
            (
                "295:39 replace and with or",
                lambda: apply("a\nb\n", f"@@ -1,2 +1,2 @@\n+c\n-a\n{marker} b\n"),
                "c\nb\n",
            ),
            (
                "297:17 replace continue with break",
                lambda: apply("a\nb\n", f"@@ -1,2 +1,2 @@\n-a\n{marker}+c\n b\n"),
                "c\nb\n",
            ),
            ("299:41 replace or with and", lambda: apply("a\n", "@@ -1 +1 @@\n a\n b\n"), no_apply),
            ('299:86 replace "\\n" with ""', lambda: apply("a\n", simple), "b\n"),
            (
                "307:17 replace break with continue",
                lambda: apply("a\nb\n", "@@ -1,2 +1,2 @@\n-a\nx stray\n+c\n b\n"),
                "b\n",
            ),
        ]
        self.judge(table)
        examined("K26 cases", table)

    def test_the_base_tree_is_rebuilt_or_refused_by_name(self):
        written(self.scratch / "x", "here\n")
        show = ["git", "-C", str(self.scratch), "show", "mb:x"]
        head_only = HEADER + "@@ -1 +1 @@\n-a\n+b\n"

        def rebuilt(base, head, shown=None):
            calls = []

            def run(argv, **options):
                calls.append(argv)
                out = (shown or {}).get(argv[-1])
                if out is None:
                    if options.get("check"):
                        raise subprocess.CalledProcessError(128, argv)
                    return subprocess.CompletedProcess(argv, 128, b"", b"fatal")
                return subprocess.CompletedProcess(argv, 0, out, b"")

            trees = self.mc.Trees(self.scratch, "mb", base, head)
            with mock.patch.object(self.mc.subprocess, "run", run):
                found = attempt(lambda: trees.at_base("x"))
            return found, calls

        binary = HEADER + "Binary files a/x and b/x differ\n"
        deleted = HEADER + "deleted file mode 100644\n@@ -1 +0,0 @@\n-a\n"
        table = [
            ("330:75 replace not with nothing", lambda: rebuilt("", ""), ("here\n", [])),
            (
                "332:16 replace is not with is",
                lambda: rebuilt(binary, ""),
                (
                    (
                        "raised",
                        "NoBase",
                        "x: the base's section is binary, so its file cannot be rebuilt",
                    ),
                    [],
                ),
            ),
            ("334:16 replace is not with is", lambda: rebuilt(deleted, ""), (None, [])),
            ("336:16 replace is with is not", lambda: rebuilt(ADDED, ""), ("new\n", [])),
            ("340:23 replace False with True", lambda: rebuilt("", head_only), (None, [show])),
            ("343:24 replace is with is not", lambda: rebuilt("", head_only), (None, [show])),
            (
                "348:24 replace return value with return None",
                lambda: rebuilt("", head_only, {"mb:x": b"old\n"}),
                ("old\n", [show]),
            ),
            ('350:19 replace "" with "XX"', lambda: rebuilt(ADDED, ""), ("new\n", [])),
        ]
        self.judge(table)
        examined("K27 cases", table)


class OutcomesKept(Base):
    """K28: the outcomes document the manifest verb keeps."""

    def test_the_outcomes_document_is_kept_exactly(self):
        plan = {
            "classes": {"scripts": {"applies": False}, "oracle": {"applies": True}},
            "base": "b0",
            "head": "h0",
        }
        report = {
            "files": [
                "x",
                {
                    "path": "tools/v.py",
                    "void": True,
                    "mutants": [{"name": "v1", "outcome": "killed"}],
                },
                {
                    "path": "tools/a.py",
                    "mutants": [
                        {"name": "m1", "outcome": "killed", "killers": ["k1"]},
                        {"name": "m1", "outcome": "killed"},
                        {"name": "m2", "outcome": "survived", "killers": []},
                    ],
                },
            ]
        }

        def kept(document=plan):
            place = Path(tempfile.mkdtemp(dir=self.scratch))
            path = place / "plan.json"
            if document is not None:
                written(path, json.dumps(document))
            written(place / "git.diff", "d\n")
            (place / "root").mkdir()
            out = place / "out.json"
            args = self.mc.argparse.Namespace(
                plan=str(path),
                root=str(place / "root"),
                python=str(place / "legs"),
                run_id="300",
                out=str(out),
            )

            def reader(scratch, given, directory):
                return [("leg 1", report)] if scratch.klass == "oracle" else []

            def classify(file):
                return "oracle" if file.startswith("tools/") else "scripts"

            code, lines = printed(lambda: self.mc.manifest_verb(args, reader, classify))
            found = json.loads(out.read_text(encoding="utf-8")) if out.is_file() else None
            return code, lines, found, str(path)

        def missing():
            code, lines, found, path = kept(None)
            return (
                code,
                lines
                == [f"carry: manifest: nothing kept: ValueError: {path} is not a mutation plan"],
                found,
            )

        document = {
            "schema": "deckstreak.mutation-outcomes.v1",
            "tree": {
                "base": "b0",
                "head": "h0",
                "diff_sha256": hashlib.sha256(b"d\n").hexdigest(),
            },
            "keys": {
                "refused": "scripts/mutation-inputs.json cannot be read: "
                "Expecting value: line 1 column 1 (char 0)"
            },
            "mutants": [
                {
                    "name": "m1",
                    "file": "tools/a.py",
                    "class": "oracle",
                    "outcome": "killed",
                    "killers": ["k1"],
                    "run": "300",
                },
                {
                    "name": "m2",
                    "file": "tools/a.py",
                    "class": "oracle",
                    "outcome": "survived",
                    "killers": [],
                    "run": "300",
                },
            ],
        }

        def whole():
            code, lines, found, _path = kept()
            return code, lines, found

        two = (0, ["carry: manifest: 2 whole or carried Python outcome(s) kept"], document)
        table = [
            ("1318:39 replace or with and", missing, (0, True, None)),
            ("1325:17 replace continue with break", whole, two),
            ('1330:83 replace "" with "XX"', whole, two),
            ("1332:25 replace continue with break", whole, two),
            ('1333:45 replace "mutants" with ""', whole, two),
            ("1336:29 replace continue with break", whole, two),
            ('1356:26 replace "refused" with ""', whole, two),
            ('1359:13 replace "tree" with ""', whole, two),
            ("1371:12 replace return value with return None", whole, two),
        ]
        self.judge(table)
        examined("K28 cases", table)


class ReadSetsAndHolds(Base):
    """K29 and K30: a file's read set over a scratch tree, and whether it holds."""

    def test_read_sets_resolve_their_closure_exactly(self):
        root = self.scratch / "tree"
        for path in ("scripts/a.py", "scripts/tests/m1.py", "scripts/tests/m2.py"):
            written(root / path, "")
        (root / "scripts" / "tests" / "bad.py").write_bytes(b"\xff")
        written(root / "scripts" / "pkg" / "__init__.py", "")
        written(root / "scripts" / "pkg" / "b.py", "")

        def document(mapping):
            return self.mc.read_set(root, "scripts/a.py", mapping).document()

        def mapped(*modules):
            entry = {"dir": "scripts/tests", "modules": list(modules)}
            return {"scripts/a.py": entry}, entry

        def expected(entry, members, whole=(), listings=(".", "scripts", "scripts/tests")):
            return {
                "file": "scripts/a.py",
                "whole": bool(whole),
                "why": "; ".join(whole),
                "entry": entry,
                "members": members,
                "paths": [],
                "trees": [],
                "globs": [],
                "listings": list(listings),
            }

        def closure(*modules):
            return document(mapped(*modules)[0])

        unmapped = expected(None, [], ["scripts/a.py: its map entry names no dir and modules"], ())
        twice, twice_entry = mapped("m1", "m1", "m2")
        three = ["scripts/a.py", "scripts/tests/m1.py", "scripts/tests/m2.py"]
        gone = "a member of the closure is not in the tree"
        bad = (
            "scripts/tests/bad.py: cannot be read: 'utf-8' codec can't decode byte 0xff in "
            "position 0: invalid start byte"
        )
        lost, lost_entry = mapped("gone", "lost")
        package = ["scripts/pkg/__init__.py", "scripts/pkg/b.py"]
        table = [
            ('888:13 replace "file" with ""', lambda: document({}), unmapped),
            (
                '890:20 replace "; " with ""',
                lambda: document(lost),
                expected(
                    lost_entry,
                    ["scripts/a.py"],
                    [f"scripts/tests/gone.py: {gone}", f"scripts/tests/lost.py: {gone}"],
                    (".", "scripts"),
                ),
            ),
            (
                "906:36 replace or with and",
                lambda: document({"scripts/a.py": None}),
                unmapped,
            ),
            ("908:16 replace return value with return None", lambda: document({}), unmapped),
            (
                "916:13 replace continue with break",
                lambda: document(twice),
                expected(twice_entry, three),
            ),
            (
                "920:13 replace continue with break",
                lambda: closure("gone", "m2"),
                expected(
                    {"dir": "scripts/tests", "modules": ["gone", "m2"]},
                    ["scripts/a.py", "scripts/tests/m2.py"],
                    [f"scripts/tests/gone.py: {gone}"],
                ),
            ),
            (
                "925:13 replace continue with break",
                lambda: closure("bad", "m2"),
                expected(
                    {"dir": "scripts/tests", "modules": ["bad", "m2"]},
                    ["scripts/a.py", "scripts/tests/m2.py"],
                    [bad],
                ),
            ),
            ("947:31 replace | with &", lambda: document(twice), expected(twice_entry, three)),
            (
                "958:42 replace 1 with 2",
                lambda: self.mc.resolved(root, "pkg.b", {"scripts"}),
                package,
            ),
            (
                '962:33 replace "__init__.py" with ""',
                lambda: self.mc.resolved(root, "pkg.b", {"scripts"}),
                package,
            ),
        ]
        self.judge(table)
        examined("K29 cases", table)

    def test_holds_names_each_reason_exactly(self):
        mc = self.mc

        def holds(changed=(), moved=(), unread=(), whole=(), **found):
            diff = mc.Changes(changed=set(changed), listings=set(moved), unread=list(unread))
            return mc.holds(mc.ReadSet(file="scripts/a.py", whole=list(whole), **found), diff)

        member = {"members": {"scripts/a.py"}}
        nothing = (True, "no path, probe or listing of the read set changed")
        table = [
            (
                "970:16 replace return value with return None",
                lambda: holds(unread=["diff --git a/x"]),
                (False, "a section of a diff names no path this reader can read: diff --git a/x"),
            ),
            (
                "970:16 replace False with True",
                lambda: holds(unread=["diff --git a/x"]),
                (False, "a section of a diff names no path this reader can read: diff --git a/x"),
            ),
            (
                '975:22 replace "the read set is the whole tree, and ... with ""',
                lambda: holds(whole=["a reason"]),
                (True, "the read set is the whole tree, and no path changed"),
            ),
            (
                "980:13 replace or with and",
                lambda: holds(changed=["scripts/a.py"], **member),
                (False, "scripts/a.py, a path of the read set, changed"),
            ),
            (
                '982:41 replace "." with ""',
                lambda: holds(changed=["docs/x.md"], trees={"."}),
                (False, "docs/x.md, a path of the read set, changed"),
            ),
            (
                "992:20 replace return value with return None",
                lambda: holds(changed=["scripts/a.py"], **member),
                (False, "scripts/a.py, a path of the read set, changed"),
            ),
            (
                "994:20 replace in with not in",
                lambda: holds(moved=["scripts"], listings={"scripts"}),
                (False, "the listing of scripts, which the read set holds, changed"),
            ),
            (
                '996:18 replace "no path, probe or listing of the rea... with ""',
                lambda: holds(changed=["docs/x.md"], **member),
                nothing,
            ),
        ]
        self.judge(table)
        examined("K30 cases", table)


GITHUB = "https://api.github.com"
QUERY = "?event=pull_request&branch=dev&status=completed&per_page=100"
SUMMARY = (
    "The release run's carry (SPEC-493, ADR-511): an admitted release run takes a Python "
    "mutant's outcome from one recorded release run of this repository, for a population file "
    "whose whole read set is byte-identical there and here, and runs every other listed mutant."
)
KEYS_OUT = """{
  "oracle": {
    "jobs": null,
    "rows": null,
    "runner": null
  },
  "scripts": {
    "jobs": null,
    "rows": null,
    "runner": null
  }
}
"""
READ_SET_OUT = """{
  "file": "scripts/a.py",
  "whole": true,
  "why": "scripts/a.py: its map entry names no dir and modules",
  "entry": null,
  "members": [],
  "paths": [],
  "trees": [],
  "globs": [],
  "listings": []
}
"""
HOLDS_OUT = """{
  "file": "scripts/a.py",
  "whole": true,
  "unchanged": true,
  "why": "the read set is the whole tree, and no path changed"
}
"""


class ScriptVerbs(Base):
    """K31 to K33: the script's own verbs and the verdict's, in process."""

    def test_the_base_verb_reads_its_defaults_and_prints_its_outputs(self):
        plan = self.scratch / "plan.json"
        written(plan, json.dumps({"release": True}))
        full = {
            "--plan": str(plan),
            "--pull-request": "822",
            "--run-id": "10",
            "--repository": "o/r",
            "--head-sha": "h1",
            "--out": str(self.scratch / "out"),
        }

        def argv(*dropped):
            words = ["base"]
            for flag, word in full.items():
                if flag not in dropped:
                    words += [flag, word]
            return words

        def base(words, token="t", answers=None, refused=False):
            urls = []

            def answer(url, held):
                urls.append((url, held))
                return (answers or {}).get(url)

            def refuse(args, held):
                raise OSError("down")

            with contextlib.ExitStack() as stack:
                stack.enter_context(mock.patch.dict(os.environ, {"GH_TOKEN": token}))
                if not token:
                    os.environ.pop("GH_TOKEN")
                stack.enter_context(mock.patch.object(self.mc, "fetch", answer))
                if refused:
                    stack.enter_context(mock.patch.object(self.mc, "choose_base", refuse))
                code, lines = printed(lambda: self.mc.main(words))
            return code, lines, urls

        def helped():
            out, code = io.StringIO(), None
            with contextlib.redirect_stdout(out):
                try:
                    self.mc.main(["--help"])
                except SystemExit as stop:
                    code = stop.code
            text = "".join(out.getvalue().split())
            return code, "".join(SUMMARY.split()) in text, "Verbsofthisscript:" in text

        unset = ["base_run=", "base_manifest=false"]
        runs = f"{GITHUB}/repos/o/r/actions/workflows/ci.yml/runs{QUERY}"
        listed = (
            0,
            ["carry: base: no base: the runs listing holds no workflow runs", *unset],
            [(runs, "t")],
        )
        failed = (0, ["carry: base: no base: a request or its answer failed: down", *unset], [])
        table = [
            ('51:7 replace "https://api.github.com" with ""', lambda: base(argv()), listed),
            ("1407:72 replace 0 with 1", helped, (0, True, False)),
            (
                '1411:51 replace "" with "XX"',
                lambda: base(argv("--pull-request")),
                (0, ["carry: base: not admitted: '' names no pull request", *unset], []),
            ),
            (
                "1412:55 replace 0 with 1",
                lambda: base(
                    argv("--run-id"), answers={runs: {"workflow_runs": [qualifying(id=1)]}}
                ),
                (
                    0,
                    [
                        "carry: base: no base: the newest run, 1, is no base: "
                        "it is not older than this run",
                        *unset,
                    ],
                    [(runs, "t")],
                ),
            ),
            (
                '1413:49 replace "" with "XX"',
                lambda: base(argv("--repository")),
                (
                    0,
                    ["carry: base: no base: the runs listing holds no workflow runs", *unset],
                    [(f"{GITHUB}/repos//actions/workflows/ci.yml/runs{QUERY}", "t")],
                ),
            ),
            (
                '1414:47 replace "" with "XX"',
                lambda: base(argv("--head-sha"), answers={runs: {"workflow_runs": [qualifying()]}}),
                (
                    0,
                    [
                        "carry: base: the newest run, 5, qualifies",
                        "carry: base: no base: run 5's head is None of this head, no ancestor",
                        *unset,
                    ],
                    [(runs, "t"), (f"{GITHUB}/repos/o/r/compare/b1...", "t")],
                ),
            ),
            ('1416:25 replace "--api" with ""', lambda: base(argv()), listed),
            (
                "1425:16 replace return value with return None",
                lambda: base(["admitted"]),
                (0, ["[822]"], []),
            ),
            (
                '1428:68 replace "" with "XX"',
                lambda: base(argv(), token=""),
                (0, ["carry: base: no base: the step holds no token", *unset], []),
            ),
            ('1431:24 replace "base_run" with ""', lambda: base(argv(), refused=True), failed),
            ('1431:36 replace "" with "XX"', lambda: base(argv(), refused=True), failed),
            ('1431:40 replace "base_manifest" with ""', lambda: base(argv(), refused=True), failed),
            ('1431:57 replace "false" with ""', lambda: base(argv(), refused=True), failed),
            ('1433:26 replace "" with "XX"', lambda: base(argv(), refused=True), failed),
            (
                "1437:16 replace return value with return None",
                lambda: base(argv(), refused=True),
                failed,
            ),
        ]
        self.judge(table)
        examined("K31 cases", table)

    def test_the_key_read_set_and_holds_verbs_print_exactly(self):
        root = self.scratch / "tree"
        written(root / "scripts" / "mutation-inputs.json", INPUTS_TEXT)
        written(root / "scripts" / "mutation-python.json", "{}")
        written(self.scratch / "base.diff", "")
        written(self.scratch / "head.diff", "")

        def verb(*words):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = self.mc.main([*words, "--root", str(root)])
            return code, out.getvalue()

        def key():
            return verb("key")

        def read_set():
            return verb("read-set", "--file", "scripts/a.py")

        def held():
            return verb(
                "holds",
                "--file",
                "scripts/a.py",
                "--base-diff",
                str(self.scratch / "base.diff"),
                "--head-diff",
                str(self.scratch / "head.diff"),
            )

        table = [
            ("1439:58 replace 2 with 3", key, (0, KEYS_OUT)),
            ("1439:71 replace True with False", key, (0, KEYS_OUT)),
            ("1440:16 replace return value with return None", key, (0, KEYS_OUT)),
            ("1444:51 replace 2 with 3", read_set, (0, READ_SET_OUT)),
            ("1445:16 replace return value with return None", read_set, (0, READ_SET_OUT)),
            ('1449:23 replace "file" with ""', held, (0, HOLDS_OUT)),
            ("1450:43 replace 2 with 3", held, (0, HOLDS_OUT)),
            ("1451:12 replace return value with return None", held, (0, HOLDS_OUT)),
        ]
        self.judge(table)
        examined("K32 cases", table)

    def test_the_verdicts_verbs_parse_their_arguments_exactly(self):
        Namespace = self.mc.argparse.Namespace

        def verdict(*words):
            seen, err, code = [], io.StringIO(), None

            def carry_verb(args, reader, classify, census):
                seen.append(args)
                return 7

            def manifest_verb(args, reader, classify):
                seen.append(args)
                return 8

            with contextlib.ExitStack() as stack:
                stack.enter_context(mock.patch.object(self.mc, "carry_verb", carry_verb))
                stack.enter_context(mock.patch.object(self.mc, "manifest_verb", manifest_verb))
                stack.enter_context(contextlib.redirect_stderr(err))
                try:
                    self.mc.verbs(list(words), None, None, None)
                except SystemExit as stop:
                    code = stop.code
            lines = err.getvalue().splitlines()
            return code, seen, lines[-1] if lines else ""

        def required(verb, flags):
            return (
                2,
                [],
                f"mutation-verdict.py {verb}: error: the following arguments are required: {flags}",
            )

        carried = (
            7,
            [
                Namespace(
                    root=str(REPO),
                    plan="p",
                    python_listed="l",
                    base_dir="b",
                    base_run="",
                    pull_request="",
                )
            ],
            "",
        )
        kept = (8, [Namespace(root=str(REPO), plan="p", python="y", out="o", run_id="")], "")
        whole = ("carry", "--plan", "p", "--python-listed", "l", "--base-dir", "b")
        table = [
            (
                '1380:43 replace "mutation-verdict.py " with ""',
                lambda: verdict("carry"),
                required("carry", "--plan, --python-listed, --base-dir"),
            ),
            (
                "1380:74 replace 0 with 1",
                lambda: verdict("carry"),
                required("carry", "--plan, --python-listed, --base-dir"),
            ),
            ('1382:25 replace "--plan" with ""', lambda: verdict(*whole), carried),
            (
                "1382:44 replace True with False",
                lambda: verdict("carry", "--python-listed", "l", "--base-dir", "b"),
                required("carry", "--plan"),
            ),
            (
                "1384:57 replace True with False",
                lambda: verdict("carry", "--plan", "p", "--base-dir", "b"),
                required("carry", "--python-listed"),
            ),
            (
                "1385:52 replace True with False",
                lambda: verdict("carry", "--plan", "p", "--python-listed", "l"),
                required("carry", "--base-dir"),
            ),
            ('1386:51 replace "" with "XX"', lambda: verdict(*whole), carried),
            ('1387:55 replace "" with "XX"', lambda: verdict(*whole), carried),
            (
                "1389:46 replace True with False",
                lambda: verdict("manifest", "--plan", "p", "--out", "o"),
                required("manifest", "--python"),
            ),
            (
                "1390:43 replace True with False",
                lambda: verdict("manifest", "--plan", "p", "--python", "y"),
                required("manifest", "--out"),
            ),
            (
                '1391:45 replace "" with "XX"',
                lambda: verdict("manifest", "--plan", "p", "--python", "y", "--out", "o"),
                kept,
            ),
        ]
        self.judge(table)
        examined("K33 cases", table)


if __name__ == "__main__":
    unittest.main()

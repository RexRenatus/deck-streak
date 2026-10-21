"""Tests that kill the carry module's listed mutants in process (SPEC-493).

`scripts/mutation_carry.py` is loaded by the one `load()` below, afresh for each test, so its own
lines run inside the test that judges them. Each class holds one table-driven method per class of
the kill table, and `scripts/mutation-python.json` maps this module first among the carry module's
test modules, so under the failfast legs a kill ends a mutant's run before the slower child-process
tests start. Every case is labelled by its mutant's `<line>:<col> <operator>`, runs under its own
`subTest`, and asserts a whole value the mutant changes. A call a mutant makes raise is read
through `attempt`, so the case fails by assertion and not by an exception.
"""

import collections
import contextlib
import dataclasses
import importlib.util
import io
import json
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


if __name__ == "__main__":
    unittest.main()

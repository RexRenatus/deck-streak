"""SPEC-422: the release pull request's Python legs may pass the share up to one signed ceiling,
dealt into matrices of at most 256 (ADR-436, #822).

Every listing, price and expected line is a literal or is built here, never by the code under
test. `a.py` is priced 600 s, so its control and sentinel make a setup of 1200 s and a leg holds
one of its mutants: n of them deal exactly n priced legs. `b.py` is unpriced, so its mutants keep
the count rule of 20 a leg: one priced mutant and 20 x (n - 1) unpriced ones deal n legs.
"""

import ast
import contextlib
import fnmatch
import io
import json
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined
from test_mutation_verdict import LIB, LIB_TEXT, Fixture, verdict_module
from test_mutation_workflows import CI, jobs, steps, workflow

VERDICT = REPO / "scripts" / "mutation-verdict.py"
RELEASING = REPO / "RELEASING.md"
LIMIT = REPO / ".github" / "workflows" / "matrix-limit.yml"
#: Where the matrix-limit run's record is committed, one directory a run (R15).
FIXTURES = REPO / "scripts" / "tests" / "fixtures"
THIS_REPOSITORY = "RexRenatus/deck-streak"
#: The one input R1 reads as the release run: `dev` into `main`, from this repository.
RELEASE = ("pull_request", "main", "dev", THIS_REPOSITORY, THIS_REPOSITORY)
SCHEMA = "deckstreak.mutation-python.v1"
#: The price table every deal here reads: `a.py` holds one mutant a leg; `b.py` is unpriced.
PRICES = {"a.py": 600}
#: A plan with no `release` key at all.
ABSENT = object()
PYTHON_AXIS = "        shard: ${{ fromJSON(needs.mutation-plan.outputs.python_matrix) }}"
RELEASE_AXIS = "        shard: ${{ fromJSON(needs.mutation-plan.outputs.python_release_matrix) }}"
RELEASE_CONDITION = "${{ needs.mutation-plan.outputs.python_release_legs != '0' }}"
RULING = re.compile(r"docs/rulings/[^\s`'\"]+\.md")
#: A pull request plan's run over three `a.py` mutants, as the plan's job prints and writes it.
GOLDEN_LINES = (
    "mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n"
    "mutation: shards: python legs dealt by cost: 3 priced leg(s), the slowest at 1800 s of its "
    "1800 s bound; 0 unpriced leg(s) of at most 20\n"
    "mutation: shards: 1 shard(s) for 0 listed mutant(s), projected at 0 s serially; the slowest "
    "at 2141 s of its 10800 s bound; legs 1 of ceiling 190\n"
    "mutation: shards: run legs 4 of 222: python 3 of 32, rust 1 of 190; projected 126 "
    "runner-minute(s) of 35160\n"
)
GOLDEN_OUTPUTS = {
    "shards": "1",
    "matrix": "[0]",
    "listed": "0",
    "python_shards": "3",
    "python_matrix": "[0, 1, 2]",
    "python_release_matrix": "[]",
    "python_release_legs": "0",
}


def mutants(priced, unpriced=0):
    """A Python listing: `priced` mutants of `a.py`, then `unpriced` of `b.py`."""
    return [
        {"name": f"{path}:{index + 1}:5: mutant {index}", "file": path}
        for path, count in (("a.py", priced), ("b.py", unpriced))
        for index in range(count)
    ]


def scratch(test):
    """A scratch directory removed when the test ends."""
    directory = tempfile.TemporaryDirectory()
    test.addCleanup(directory.cleanup)
    return Path(directory.name)


def verb(argv, ceiling=None, prices=None):
    """One verb of `mutation-verdict.py`, in process, as a job runs it under GitHub Actions: the
    price table patched to `prices` (PRICES when None), the scoped table empty and, unless None,
    `PYTHON_RELEASE_CEILING` patched to `ceiling`: (the exit, the printed text, the step outputs)."""
    module = verdict_module()
    with tempfile.TemporaryDirectory() as directory:
        sink = Path(directory) / "github-output"
        sink.write_text("", encoding="utf-8")
        out = io.StringIO()
        with contextlib.ExitStack() as stack:
            stack.enter_context(mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(sink)}))
            table = PRICES if prices is None else prices
            stack.enter_context(mock.patch.dict(module.PYTHON_CONTROL_SECONDS, table, clear=True))
            stack.enter_context(mock.patch.dict(module.PYTHON_SCOPED_SECONDS, {}, clear=True))
            if ceiling is not None:
                stack.enter_context(
                    mock.patch.object(module, "PYTHON_RELEASE_CEILING", ceiling, create=True)
                )
            stack.enter_context(
                mock.patch.object(module, "RUST_RELEASE_CEILING", module.LEG_CEILING["ci"])
            )
            stack.enter_context(contextlib.redirect_stdout(out))
            code = module.main([str(each) for each in argv])
        written = sink.read_text(encoding="utf-8").splitlines()
    return code, out.getvalue(), dict(line.split("=", 1) for line in written)


def deal(test, entries, release=ABSENT, ceiling=None, prices=None):
    """`shards` over a plan whose Rust class does not apply, marked `release` unless ABSENT, and a
    Python listing of `entries`: (the exit, the printed text, the step outputs, the plan as left)."""
    root = scratch(test)
    plan = {"classes": {"rust": {"applies": False}}}
    if release is not ABSENT:
        plan["release"] = release
    (root / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    listing = {"schema": SCHEMA, "listed": len(entries), "mutants": entries}
    (root / "python-listed.json").write_text(json.dumps(listing), encoding="utf-8")
    argv = ["shards", "--plan", root / "plan.json", "--python-listed", root / "python-listed.json"]
    code, out, outputs = verb(argv, ceiling, prices)
    return code, out, outputs, json.loads((root / "plan.json").read_text(encoding="utf-8"))


def legs_of(test, count, *given):
    """`legs` over a plan whose Python matrix holds `count` legs, the Rust and rows legs having
    succeeded, with `given` after: (the exit, the printed text)."""
    root = scratch(test)
    plan = {"classes": {}, "scope": {"decision": "diff"}, "python": {"count": count}}
    (root / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
    argv = ["legs", "--plan", root / "plan.json", "--rust-leg", "success", "--rows-leg", "success"]
    code, out, _outputs = verb([*argv, *given])
    return code, out


def release_line(legs, ceiling, first, second):
    """R6's release line, spelled here."""
    return (
        f"mutation: shards: release run: {legs} python leg(s) of ceiling {ceiling} "
        f"(PYTHON_RELEASE_CEILING): {first} in mutation-python and {second} in "
        "mutation-python-release, at most 256 a matrix\n"
    )


def share_refusal(listed, priced, unpriced):
    """The cost deal's refusal past the share, byte for byte as before (SPEC-418 R4)."""
    return (
        f"mutation: shards: REFUSED: {listed} python mutant(s) need {priced} leg(s) priced at most "
        f"1800 s each and {unpriced} unpriced leg(s) of at most 20, more than the pull request "
        "run's 32 python legs: split the change, since a run is never capped\n"
    )


def job_condition(job):
    """A job's job-level `if:` expressions, in order."""
    return re.findall(r"(?m)^    if: (.*)$", job)


def significant(job):
    """A job's lines less blank and comment-only ones: what GitHub reads of it."""
    return [line for line in job.splitlines() if line.strip() and not line.lstrip().startswith("#")]


def ceiling_problems(source, root):
    """What R13 refuses in a source of `mutation-verdict.py`, judged against the tree at `root`:
    one `PYTHON_RELEASE_CEILING` int with a `#:` line directly above it; at `PYTHON_SHARE_CAP` the
    line cites no ruling path; above it the line cites exactly one `docs/rulings/<file>.md`, the
    tree holds it, and its first line holds the figure as a whole word and the word `release`."""
    assigned = {}
    for node in ast.parse(source).body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1:
            target = node.targets[0]
            if isinstance(target, ast.Name):
                assigned.setdefault(target.id, []).append(node)
    ceilings = assigned.get("PYTHON_RELEASE_CEILING", [])
    caps = assigned.get("PYTHON_SHARE_CAP", [])
    if len(ceilings) != 1 or len(caps) != 1:
        return [f"{len(ceilings)} ceiling and {len(caps)} cap assignment(s), not one of each"]
    node, cap = ceilings[0], caps[0].value
    if not all(
        isinstance(each, ast.Constant) and type(each.value) is int for each in (node.value, cap)
    ):
        return ["the ceiling or the cap is not an int literal"]
    above = source.splitlines()[node.lineno - 2] if node.lineno > 1 else ""
    if not above.startswith("#: "):
        return ["the ceiling has no #: line directly above it"]
    value, cited = node.value.value, RULING.findall(above)
    if value < cap.value:
        return [f"the ceiling {value} is below the cap {cap.value}"]
    if value == cap.value:
        return [f"at the cap the #: line cites {cited}"] if cited else []
    if len(cited) != 1:
        return [f"the ceiling {value} is above the cap and cites {len(cited)} ruling path(s)"]
    path = root / cited[0]
    if not path.is_file():
        return [f"the ceiling cites {cited[0]}, which the tree does not hold"]
    first = (path.read_text(encoding="utf-8").splitlines() or [""])[0]
    if not re.search(rf"(?<![\w.]){value}(?![\w.])", first) or not re.search(r"\brelease\b", first):
        return [f"{cited[0]}'s first line does not hold {value} and the word release"]
    return []


def matrix_limit_problems(record, rows):
    """What R15 refuses in the matrix-limit run's record and its jobs listing (one row a job: id,
    name, conclusion and seconds): each matrix reported its 256 legs, numbered on from the first
    matrix's 0 as the release deal numbers them; each leg read a job total of 256; `first` and
    `second` succeeded and `empty` was skipped, by the record and by the listing; and the run held
    more than 256 jobs, one count that the record and the listing both read."""
    problems = []
    for name, start in (("first", 0), ("second", 256)):
        matrix = record["matrices"][name]
        if matrix["legs"] != 256 or matrix["leg_values"] != list(range(start, start + 256)):
            problems.append(f"{name}: {matrix['legs']} leg(s), not the 256 numbered from {start}")
        if matrix["job_totals"] != [256]:
            problems.append(f"{name}: job total(s) {matrix['job_totals']}, not [256]")
    listed = {}
    for row in rows:
        listed.setdefault(row[1].split(" (")[0], []).append(row[2])
    for job, result, count in (
        ("first", "success", 256),
        ("second", "success", 256),
        ("empty", "skipped", 1),
    ):
        if record["results"].get(job) != result:
            problems.append(f"{job}: the record reads {record['results'].get(job)}, not {result}")
        got = listed.get(job, [])
        if got != [result] * count:
            problems.append(
                f"the listing holds {len(got)} {job} job(s) reading {sorted(set(got))}, "
                f"not {count} reading {[result]}"
            )
    if not record["run_jobs"] == len(rows) > 256:
        problems.append(
            f"the record counts {record['run_jobs']} job(s) and the listing {len(rows)}, "
            "not one count past 256"
        )
    return problems


class TheReleaseRunIsNamedByAFailClosedPredicate(unittest.TestCase):
    """R1 to R3 (A1 to A4)."""

    def test_release_run_holds_only_for_this_repositorys_dev_into_main(self):
        cases = [
            ("the release pull request", RELEASE, True),
            ("a push", ("push", "main", "dev", THIS_REPOSITORY, THIS_REPOSITORY), False),
            ("into dev", ("pull_request", "dev", "dev", THIS_REPOSITORY, THIS_REPOSITORY), False),
            (
                "a head other than dev",
                ("pull_request", "main", "feat/x", THIS_REPOSITORY, THIS_REPOSITORY),
                False,
            ),
            ("no repository name", ("pull_request", "main", "dev", "", ""), False),
            (
                "a fork's dev",
                ("pull_request", "main", "dev", "someone/deck-streak", THIS_REPOSITORY),
                False,
            ),
            ("every input empty", ("", "", "", "", ""), False),
        ]
        module = verdict_module()
        for where, inputs, expected in examined("release inputs", cases):
            self.assertIs(module.release_run(*inputs), expected, where)

    def test_the_plan_records_release_only_when_the_predicate_holds(self):
        fixture = Fixture(self)
        fixture.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        cases = [
            ("the release pull request", RELEASE, True),
            (
                "into dev",
                ("pull_request", "dev", "feat/x", THIS_REPOSITORY, THIS_REPOSITORY),
                False,
            ),
            ("a push", ("push", "", "", "", THIS_REPOSITORY), False),
            ("a schedule", ("schedule", "", "", "", THIS_REPOSITORY), False),
            ("a dispatch", ("workflow_dispatch", "", "", "", THIS_REPOSITORY), False),
        ]
        for where, (event, base_ref, head_ref, head_repo, repo), held in examined("events", cases):
            flags = ["--event", event, "--base-ref", base_ref, "--subject", "chore: a run"]
            flags += ["--head-ref", head_ref, "--head-repo", head_repo, "--repo", repo]
            plan = fixture.plan(*flags)
            if held:
                self.assertIs(plan.get("release"), True, where)
            else:
                self.assertNotIn("release", plan, where)
            text = (fixture.out / "plan.json").read_text(encoding="utf-8")
            self.assertEqual(text, json.dumps(plan, indent=2) + "\n", where)
        # A flag left out reads empty, so one side of the repository pair named alone is never the
        # release run, even when it names `XX`, the text a mutated empty default would read.
        into_main = ["--event", "pull_request", "--base-ref", "main", "--head-ref", "dev"]
        alone = [("the head repository alone", "--head-repo"), ("the repository alone", "--repo")]
        for where, flag in examined("repository flags given alone", alone):
            plan = fixture.plan(*into_main, "--subject", "chore: a run", flag, "XX")
            self.assertIn("classes", plan, where)
            self.assertNotIn("release", plan, where)
        module = verdict_module()
        module.plan_diff(fixture.root, fixture.base, "HEAD", fixture.out, ("diff", "a diff"))
        written = json.loads((fixture.out / "plan.json").read_text(encoding="utf-8"))
        self.assertIn("classes", written)
        self.assertNotIn("release", written, "plan_diff called with no release given")

    def test_a_release_mark_that_is_not_true_is_not_the_release_run(self):
        entries = mutants(40)
        code, out, _outputs, _plan = deal(self, entries, True, 200)
        self.assertEqual(code, 0, out)
        self.assertIn(release_line(40, 200, 40, 0), out)
        marks = [("absent", ABSENT), ("false", False), ("the string", "true"), ("one", 1)]
        marks.append(("null", None))
        for where, mark in examined("release marks that are not true", marks):
            code, out, _outputs, plan = deal(self, entries, mark, 200)
            self.assertEqual((code, out), (1, share_refusal(40, 40, 0)), where)
            self.assertNotIn("python", plan, where)

    def test_the_plan_step_passes_head_ref_head_repo_and_repo_by_env(self):
        job = jobs(workflow(CI)).get("mutation-plan", "")
        (step,) = [each for each in steps(job) if "mutation-verdict.py plan " in each]
        given = dict(re.findall(r"(?m)^          ([A-Z_]+): (.*)$", step))
        expected = {
            "HEAD_REF": "${{ github.head_ref }}",
            "HEAD_REPO": "${{ github.event.pull_request.head.repo.full_name }}",
            "REPO": "${{ github.repository }}",
        }
        for name, value in examined("names the plan step passes", expected.items()):
            self.assertEqual(given.get(name), value, name)
        command = re.search(
            r"(?m)^        run: (python3 scripts/mutation-verdict\.py plan .*)$", step
        )
        self.assertIsNotNone(command, step)
        for flag in ('--head-ref "$HEAD_REF"', '--head-repo "$HEAD_REPO"', '--repo "$REPO"'):
            self.assertIn(f" {flag}", command.group(1))
        self.assertNotIn("${{", command.group(1))


class TheReleaseRunDealsUpToItsCeiling(unittest.TestCase):
    """R4 to R8 (A5 to A10)."""

    def test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run(self):
        entries = mutants(3)
        code, out, outputs, _plan = deal(self, entries, True, 200)
        self.assertEqual(code, 0, out)
        self.assertIn(release_line(3, 200, 3, 0), out)
        self.assertNotEqual(out, GOLDEN_LINES)
        module = verdict_module()
        cases = [("the pull request plan", ABSENT), ("the release plan", True)]
        for where, release in examined("plans at the placeholder ceiling", cases):
            code, out, outputs, _plan = deal(self, entries, release, module.PYTHON_SHARE_CAP)
            self.assertEqual((code, out), (0, GOLDEN_LINES), where)
            self.assertEqual(outputs, GOLDEN_OUTPUTS, where)

    def test_a_ceiling_outside_its_range_is_refused_by_name(self):
        entries = mutants(3)
        refused = [
            ("a string", "200", "200"),
            ("a float", 200.0, "200.0"),
            ("a bool", True, "True"),
            ("below the cap", 127, "127"),
            ("above the product", 513, "513"),
        ]
        for where, ceiling, shown in examined("ceilings outside the range", refused):
            code, out, outputs, plan = deal(self, entries, True, ceiling)
            text = (
                f"mutation: shards: REFUSED: the release ceiling PYTHON_RELEASE_CEILING = {shown} "
                "is outside 128..512; a ceiling outside it is never dealt\n"
            )
            self.assertEqual((code, out), (1, text), where)
            self.assertEqual((outputs, plan.get("python")), ({}, None), where)
        for ceiling in examined("ceilings at the range's ends", [128, 512]):
            code, out, _outputs, plan = deal(self, entries, True, ceiling)
            self.assertEqual(code, 0, out)
            self.assertEqual(plan["python"]["count"], 3, ceiling)

    def test_the_release_run_deals_up_to_the_ceiling_and_refuses_one_leg_past_it(self):
        code, out, outputs, plan = deal(self, mutants(1, 20 * 299), True, 300)
        self.assertEqual(code, 0, out)
        self.assertIn(release_line(300, 300, 256, 44), out)
        self.assertEqual((plan["python"]["count"], outputs["python_shards"]), (300, "300"))
        code, out, outputs, plan = deal(self, mutants(1, 20 * 299 + 1), True, 300)
        text = (
            "mutation: shards: REFUSED: 5982 python mutant(s) need 1 leg(s) priced at most 1800 s "
            "each and 300 unpriced leg(s) of at most 20, more than the release run's 300 python "
            "legs (PYTHON_RELEASE_CEILING): a larger figure needs a new measurement and a new "
            "owner-signed ruling, since a run is never capped\n"
        )
        self.assertEqual((code, out), (1, text))
        self.assertEqual((outputs, plan.get("python")), ({}, None))

    def test_a_run_that_is_not_the_release_keeps_the_share_at_any_ceiling(self):
        entries = mutants(40)
        code, out, _outputs, _plan = deal(self, entries, True, 200)
        self.assertEqual(code, 0, out)
        for ceiling in examined("ceilings", [128, 129, 200, 512]):
            code, out, _outputs, plan = deal(self, entries, ABSENT, ceiling)
            self.assertEqual((code, out), (1, share_refusal(40, 40, 0)), ceiling)
            self.assertNotIn("python", plan, ceiling)

    def test_the_count_rule_keeps_the_share_on_the_release_run(self):
        entries = mutants(0, 660)
        code, out, _outputs, plan = deal(self, entries, True, 200, {"b.py": 300})
        self.assertEqual(code, 0, out)
        self.assertIn(release_line(165, 200, 165, 0), out)
        self.assertEqual(plan["python"]["count"], 165)
        refusal = (
            "mutation: shards: REFUSED: 660 python mutant(s) need 33 legs of at most 20, more than "
            "the pull request run's 32 python legs: split the change, since a run is never capped\n"
        )
        for ceiling in examined("ceilings", [129, 200, 512]):
            code, out, _outputs, plan = deal(self, entries, True, ceiling)
            self.assertEqual((code, out), (1, refusal), ceiling)
            self.assertNotIn("python", plan, ceiling)

    def test_the_release_legs_split_into_matrices_each_shard_once(self):
        for legs in examined("leg counts", [1, 256, 257, 300]):
            code, out, outputs, _plan = deal(self, mutants(1, 20 * (legs - 1)), True, 300)
            first, second = list(range(min(legs, 256))), list(range(256, legs))
            self.assertEqual(outputs.get("python_release_legs"), str(len(second)), legs)
            self.assertEqual(outputs.get("python_release_matrix"), json.dumps(second), legs)
            self.assertEqual(outputs.get("python_matrix"), json.dumps(first), legs)
            self.assertEqual(outputs.get("python_shards"), str(legs), legs)
            self.assertEqual(code, 0, out)
            dealt = json.loads(outputs["python_matrix"]) + json.loads(
                outputs["python_release_matrix"]
            )
            self.assertEqual(sorted(dealt), list(range(legs)), legs)
            self.assertEqual(len(dealt), len(set(dealt)), legs)


class EveryLegOfBothMatricesIsJudged(unittest.TestCase):
    """R9 to R11 and R14's neighbour checks (A11, A12, A14, A17)."""

    def test_the_release_job_is_the_python_job_on_the_release_axis(self):
        found = jobs(workflow(CI))
        release = significant(found.get("mutation-python-release", ""))
        condition = f"    if: {RELEASE_CONDITION}"
        self.assertEqual(release.count(condition), 1, "the release job's job-level condition")
        self.assertEqual(release.count(RELEASE_AXIS), 1, "the release job's axis")
        python = significant(found["mutation-python"])
        self.assertEqual(python.count(PYTHON_AXIS), 1)
        self.assertEqual(job_condition(found["mutation-python"]), [])
        swapped = [PYTHON_AXIS if line == RELEASE_AXIS else line for line in release]
        swapped.remove(condition)
        self.assertEqual(swapped, examined("lines of the python job", python))

    def test_legs_judges_the_release_leg_against_the_plan(self):
        void = "mutation: legs: VOID mutation-python-release: "
        owed = "while the plan deals 300 python leg(s), past PYTHON_MATRIX_LEGS\n"
        cases = [
            (
                "started",
                300,
                ("--python-release-leg", "success"),
                0,
                "mutation: legs: mutation-python-release: started, success\n",
            ),
            (
                "owed and skipped",
                300,
                ("--python-release-leg", "skipped"),
                3,
                f"{void}not started {owed}",
            ),
            ("owed with no argument", 300, (), 3, f"{void}no result was given {owed}"),
            (
                "not owed and skipped",
                256,
                ("--python-release-leg", "skipped"),
                0,
                "mutation: legs: mutation-python-release: not started: the plan deals no python "
                "leg past PYTHON_MATRIX_LEGS\n",
            ),
            (
                "no count and skipped",
                None,
                ("--python-release-leg", "skipped"),
                0,
                "mutation: legs: mutation-python-release: not started: the plan deals no python "
                "leg past PYTHON_MATRIX_LEGS\n",
            ),
        ]
        for where, count, given, exit_code, line in examined("release leg cases", cases):
            code, out = legs_of(self, count, *given)
            self.assertIn(line, out, where)
            self.assertEqual(code, exit_code, f"{where}: {out}")
            self.assertRegex(out, rf"(?m)^examined {2 if not given else 3}$", where)
        code, out = legs_of(self, 256)
        golden = (
            "mutation: legs: mutation-rust: started, success\n"
            "mutation: legs: mutation-rows: started, success\n"
            "mutation: legs: verdict: ok\n"
            "examined 2\n"
        )
        self.assertEqual((code, out), (0, golden))

    def test_the_verdict_counts_every_shard_of_both_matrices(self):
        found = jobs(workflow(CI))
        verdict = found.get("mutation-verdict", "")
        needs = re.search(r"(?m)^    needs: \[([^\]]*)\]$", verdict)
        self.assertIsNotNone(needs, verdict)
        named = [name.strip() for name in needs.group(1).split(",")]
        for job in examined("python matrix jobs", ["mutation-python-release", "mutation-python"]):
            self.assertIn(job, named)
            uploaded = re.findall(
                r"(?m)^          name: (mutation-python-shard-.*)$", found.get(job, "")
            )
            patterns = re.findall(r"(?m)^          pattern: (mutation-python-shard-.*)$", verdict)
            self.assertEqual(len(uploaded), 1, job)
            self.assertEqual(len(patterns), 1, job)
            self.assertTrue(
                fnmatch.fnmatchcase(uploaded[0].replace("${{ matrix.shard }}", "300"), patterns[0]),
                job,
            )
        (step,) = [each for each in steps(verdict) if "mutation-verdict.py legs " in each]
        self.assertIn("PYTHON_RELEASE_LEG: ${{ needs.mutation-python-release.result }}", step)
        call = re.search(r"(?m)^ *python3 scripts/mutation-verdict\.py legs .*$", step)
        self.assertIsNotNone(call, step)
        self.assertIn(' --python-release-leg "$PYTHON_RELEASE_LEG"', call.group(0))
        module = verdict_module()
        judged = module.Verdict("scripts")
        with contextlib.redirect_stdout(io.StringIO()):
            module.python_reports(judged, {"python": {"count": 300}}, str(scratch(self)))
        missing = [f"mutation-python-shard-{shard}: no report" for shard in range(300)]
        self.assertEqual(judged.voids, examined("shards the plan promised", missing))

    def test_python_matrices_equals_the_workflows_python_matrix_jobs(self):
        axis = r"(?m)^        [\w-]+: \$\{\{ fromJSON\(needs\.mutation-plan\.outputs\.(python_\w*matrix)\) \}\}$"
        reading = {name: re.findall(axis, job) for name, job in jobs(workflow(CI)).items()}
        matrices = {name: outputs for name, outputs in reading.items() if outputs}
        module = verdict_module()
        self.assertEqual(len(examined("python matrix jobs", matrices)), module.PYTHON_MATRICES)
        read = sorted(output for outputs in matrices.values() for output in outputs)
        self.assertEqual(read, ["python_matrix", "python_release_matrix"])


class TheMatrixLimitIsMeasuredInPublic(unittest.TestCase):
    """R14 and R15 (A18, A19): the workflow that measures whether the limit holds per matrix or per
    run, and the committed record of its one dispatch."""

    def test_the_matrix_limit_workflow_measures_two_matrices_in_one_run(self):
        text = workflow(LIMIT)
        head = significant(text.split("\njobs:\n", 1)[0])
        for line in examined(
            "lines of the triggers and the token",
            [
                "on:",
                "  workflow_dispatch:",
                "    inputs:",
                "      legs:",
                "        type: number",
                "  pull_request:",
                "    branches: [dev]",
                "    types: [opened, synchronize, reopened]",
                '      - ".github/workflows/matrix-limit.yml"',
                "permissions:",
                "  contents: read",
            ],
        ):
            self.assertEqual(head.count(line), 1, line)
        inputs = [line for line in head if re.fullmatch(r"      [\w-]+:", line)]
        self.assertEqual(inputs, ["      legs:"], "the dispatch takes one input")
        self.assertEqual(text.count("permissions:"), 1, "a job sets a token of its own")
        found = jobs(text)
        self.assertEqual(sorted(found), ["count", "empty", "first", "second", "size"])
        needs = {name: re.findall(r"(?m)^    needs: (.*)$", job) for name, job in found.items()}
        self.assertEqual(
            needs,
            {
                "size": [],
                "first": ["[size]"],
                "second": ["[size]"],
                "empty": ["[size]"],
                "count": ["[size, first, second, empty]"],
            },
        )
        conditions = {name: job_condition(job) for name, job in found.items()}
        self.assertEqual(
            conditions,
            {
                "size": [],
                "first": [],
                "second": [],
                "empty": ["${{ needs.size.outputs.empty != '[]' }}"],
                "count": ["${{ always() }}"],
            },
        )
        axes = {name: re.findall(r"(?m)^        leg: (.*)$", job) for name, job in found.items()}
        self.assertEqual(
            axes,
            {
                "size": [],
                "first": ["${{ fromJSON(needs.size.outputs.first) }}"],
                "second": ["${{ fromJSON(needs.size.outputs.second) }}"],
                "empty": ["${{ fromJSON(needs.size.outputs.empty) }}"],
                "count": [],
            },
        )
        written = re.findall(
            r"(?m)^      (\w+): \$\{\{ steps\.axes\.outputs\.(\w+) \}\}$", found["size"]
        )
        self.assertEqual(written, [("first", "first"), ("second", "second"), ("empty", "empty")])
        self.assertIn("is outside 1 to 256; refused", found["size"])
        for matrix in examined("matrices", ["first", "second"]):
            job = significant(found[matrix])
            for line in (
                f"          MATRIX: {matrix}",
                "          LEG: ${{ matrix.leg }}",
                "          TOTAL: ${{ strategy.job-total }}",
                f"          name: matrix-limit-{matrix}-${{{{ matrix.leg }}}}",
            ):
                self.assertEqual(job.count(line), 1, f"{matrix}: {line}")
        count = significant(found["count"])
        for line in examined(
            "lines of the count",
            [
                "          pattern: matrix-limit-*",
                "          merge-multiple: true",
                "          SIZE: ${{ needs.size.result }}",
                "          FIRST: ${{ needs.first.result }}",
                "          SECOND: ${{ needs.second.result }}",
                "          EMPTY: ${{ needs.empty.result }}",
                "          name: matrix-limit",
                "          path: ${{ runner.temp }}/matrix-limit/matrix-limit.json",
            ],
        ):
            self.assertEqual(count.count(line), 1, line)
        self.assertIn('"run_jobs": run_jobs', found["count"])
        # Each action is pinned as the Python legs pin it, byte for byte.
        pinned = re.findall(r"(?m)^\s+(?:- )?uses: (.+)$", jobs(workflow(CI))["mutation-python"])
        used = re.findall(r"(?m)^\s+(?:- )?uses: (.+)$", text)
        for ref in examined("actions the workflow uses", used):
            self.assertIn(ref, pinned)
        self.assertEqual(len(set(used)), 3, "checkout, download and upload")

    def test_the_recorded_matrix_limit_run_ran_each_matrix_whole(self):
        """R15 (A19): the one dispatch at `legs` 256 is committed as the count job's record and the
        attempt's jobs listing. Both read R15's six facts, and a planted copy less one leg, built
        here from the record, is refused by the same reading."""
        runs = examined("matrix-limit records", sorted(FIXTURES.glob("matrix-limit-*")))
        self.assertEqual(len(runs), 1, runs)
        text = (runs[0] / "matrix-limit.json").read_text(encoding="utf-8")
        listing = (runs[0] / "jobs.tsv").read_text(encoding="utf-8")
        record = json.loads(text)
        rows = [line.split("\t") for line in listing.splitlines()]
        # The record is the count job's own file, byte for byte as its step writes it.
        self.assertEqual(text, json.dumps(record, indent=2, sort_keys=True))
        self.assertEqual([len(row) for row in rows], [4] * len(rows))
        self.assertEqual(
            {
                name: (matrix["legs"], matrix["job_totals"])
                for name, matrix in record["matrices"].items()
            },
            {"first": (256, [256]), "second": (256, [256])},
        )
        self.assertEqual(
            record["results"],
            {"size": "success", "first": "success", "second": "success", "empty": "skipped"},
        )
        self.assertEqual(record["records"], 512)
        self.assertEqual(record["run_jobs"], len(rows))
        self.assertGreater(len(rows), 256)
        self.assertEqual(matrix_limit_problems(record, rows), [])
        planted, kept = json.loads(text), list(rows)
        planted["matrices"]["first"]["legs"] -= 1
        planted["matrices"]["first"]["leg_values"].pop()
        planted["records"] -= 1
        planted["run_jobs"] -= 1
        kept.remove(next(row for row in kept if row[1].startswith("first (")))
        self.assertEqual(
            matrix_limit_problems(planted, kept),
            [
                "first: 255 leg(s), not the 256 numbered from 0",
                "the listing holds 255 first job(s) reading ['success'], not 256 reading ['success']",
            ],
        )


class TheCeilingIsSignedAndTheRunbookSaysTheMethod(unittest.TestCase):
    """R13, R16 and R17 (A16, A20, A21)."""

    def test_the_ceiling_cites_the_signed_ruling_only_above_the_cap(self):
        self.assertEqual(ceiling_problems(VERDICT.read_text(encoding="utf-8"), REPO), [])
        root = scratch(self)
        (root / "docs" / "rulings").mkdir(parents=True)
        (root / "docs" / "rulings" / "signed.md").write_text(
            "Signed: 200 is the release ceiling\n", encoding="utf-8"
        )
        (root / "docs" / "rulings" / "other.md").write_text(
            "Signed: 300 is the release ceiling\n", encoding="utf-8"
        )

        def source(value, cites):
            return f"PYTHON_SHARE_CAP = 128\n#: The release run's ceiling{cites}.\nPYTHON_RELEASE_CEILING = {value}\n"

        self.assertEqual(ceiling_problems(source(200, ", docs/rulings/signed.md"), root), [])
        self.assertEqual(ceiling_problems(source(128, ""), root), [])
        planted = [
            ("at the cap, a ruling cited", source(128, ", docs/rulings/signed.md")),
            ("above the cap, no ruling cited", source(200, "")),
            ("above the cap, the ruling absent", source(200, ", docs/rulings/absent.md")),
            ("above the cap, another figure signed", source(200, ", docs/rulings/other.md")),
        ]
        for where, text in examined("planted sources", planted):
            self.assertEqual(len(ceiling_problems(text, root)), 1, where)

    def test_the_runbook_names_the_release_ceiling_its_job_and_its_refusal(self):
        lines = RELEASING.read_text(encoding="utf-8").splitlines()
        share = lines.index("deal, whoever signs it (SPEC-420, ADR-434).")
        sentence = lines[share + 1]
        self.assertIn("`PYTHON_RELEASE_CEILING`", sentence)
        self.assertEqual(lines[share + 2], "")
        scoped = lines.index("and the table change together, in one delivery.")
        cut = next(
            index
            for index in range(scoped, len(lines))
            if lines[index].startswith("**Cut a release before its range reaches the ceiling.**")
        )
        self.assertEqual((lines[scoped + 1], lines[cut - 1]), ("", ""))
        paragraph = lines[scoped + 2 : cut - 1]
        self.assertTrue(all(examined("lines of the runbook's paragraph", paragraph)))
        said = " ".join(paragraph)
        for phrase in examined(
            "phrases the paragraph says",
            [
                "`PYTHON_RELEASE_CEILING`",
                "`mutation-python-release`",
                "`PYTHON_MATRIX_LEGS`",
                "refused whole, by name, never capped",
                "a new measurement and a new signature",
            ],
        ):
            self.assertIn(phrase, said)
        for text in (sentence, said):
            self.assertNotRegex(re.sub(r"\b(?:SPEC|ADR)-\d+\b", "", text), r"\d")

    def test_the_release_run_assigns_no_share_cap_bound_or_ceiling_of_its_own(self):
        tree = ast.parse(VERDICT.read_text(encoding="utf-8"))
        values = (
            "PYTHON_MAX_SHARDS",
            "PYTHON_SHARE_CAP",
            "PYTHON_LEG_BOUND_SECONDS",
            "LEG_CEILING",
        )
        stores = [
            node.id
            for node in ast.walk(tree)
            if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load)
        ]
        for name in examined("values the release run reads", values):
            self.assertEqual(stores.count(name), 1, name)
        restated = {32, 128, 1800, 190, 234}
        functions = [
            node
            for node in tree.body
            if isinstance(node, ast.FunctionDef)
            and (
                node.name.startswith(("release", "python"))
                or node.name in ("shards", "legs", "main")
            )
        ]
        for function in examined("functions the release run reaches", functions):
            literals = {
                node.value
                for node in ast.walk(function)
                if isinstance(node, ast.Constant) and type(node.value) is int
            }
            self.assertEqual(literals & restated, set(), function.name)


if __name__ == "__main__":
    unittest.main()

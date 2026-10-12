"""The Python lane's verdict counts a shard's work only from the report bound to that shard
(SPEC-126 A8 and A9, issue #438).

The class rule: the verdict reads a report only when the report's own `shard` field equals the slot
it sits in (`mutation-python-shard-<k>` holds `k/<count>`), and the mutants it examined equal the
plan's listing for that shard, as a multiset. Any other layout is VOID, naming the shard.

The population is generated from a plan's shards, never listed by hand: for each of three plans
(2, 3 and 4 shards, sized by the runner's own ceiling of listed over 20), every shard's report is
laid out in every wrong way the class holds, and the correct layout is the control. No mutant is
run here: each report is written in the runner's own schema, with the real coordinates its
`list_source` gives.
"""

import ast
import contextlib
import copy
import importlib.util
import io
import itertools
import json
import os
import re
import sys
import tempfile
import traceback
import types
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from unittest import mock

from _support import examined
from test_mutation_python_verdict import (
    FILES,
    REACHED,
    SCRIPT,
    VERDICT,
    a_record,
    changed_fixture,
    file_entry,
    judged,
    listed,
    report_of,
    shard_the_plan,
    write_shard,
)
from test_mutation_verdict import Fixture

#: The plans' shard counts, each reached by listing that many twenty-mutant shards' worth.
COUNTS = (2, 3, 4)
#: The shard field's malformed spellings, as a function of the slot and the plan's count.
MALFORMED = (
    ("absent", None),
    ("null", "null"),
    ("bare number", "{k}"),
    ("other count", "{k}/{n1}"),
    ("another slot", "{other}/{n}"),
    ("garbled", "x/{n}"),
    ("spaced", " {k}/{n}"),
    ("trailing", "{k}/{n} "),
    ("a number, not text", 0),
    ("empty", ""),
    ("negative", "-{k}/{n}"),
)


def verdict_program():
    """scripts/mutation-verdict.py loaded once as a module: the same `main` the program runs."""
    loaded = sys.modules.get("mutation_verdict_in_process")
    if loaded is None:
        spec = importlib.util.spec_from_file_location("mutation_verdict_in_process", VERDICT)
        loaded = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = loaded
        spec.loader.exec_module(loaded)
    return loaded


def judge_in_process(fixture, klass, reports, program=None, extra=()):
    """`fixture.judge(klass, "--python", reports)` without a process: the program's own `main`
    over the same argv, its stdout and stderr captured, its exit code returned. A crash reads as
    the program reads it: exit 1 and the traceback on stderr."""
    argv = [
        "judge",
        "--plan",
        str(fixture.out / "plan.json"),
        "--class",
        klass,
        "--root",
        str(fixture.root),
        "--python",
        str(reports),
        *extra,
    ]
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        try:
            code = (program or verdict_program()).main(argv)
        except SystemExit as stop:
            code = stop.code if isinstance(stop.code, int) else 1
        except Exception:
            code = 1
            err.write(traceback.format_exc())
    return types.SimpleNamespace(returncode=code, stdout=out.getvalue(), stderr=err.getvalue())


def run_controls(chosen):
    """One end-to-end control per family: the first member of each, judged by the program itself.
    It runs after a test's own counts are asserted, so a miscount is read before a routing one."""
    for family, (plan, label, slots, wrong) in chosen.items():
        plan.control(family, label, slots, wrong)


def population_text(count):
    """A script whose listing has just enough mutants for `count` shards: three a function."""
    functions = -(-(20 * (count - 1) + 1) // 3)
    return "".join(f"def f{i}(x):\n    return x + {i}\n" for i in range(functions))


def ghost(number):
    """A mutant the plan never listed, in the runner's own record shape."""
    return {
        "name": f"{SCRIPT}:{number}:1: replace + with - in ghost",
        "file": SCRIPT,
        "line": number,
        "column": 1,
        "start_line": number,
        "end_line": number,
        "end_column": 5,
        "mutant": "replace + with - in ghost",
        "operator": "+",
        "outcome": "killed",
        "killers": ["test_guard.TheGuard.test_it_adds"],
    }


#: A row of the fixture's own table that the diff selects: the verdict carries it as one examined
#: member beside the mutants the reports hold, so `examined`, `generated` and `rows` differ.
CARRIED = ["S00077-GUARD", SCRIPT, "return x + 0", "return x + 3", REACHED, "the guard adds zero"]


class Plan:
    """One fixture repository, its plan sized to `count` shards, and each shard's own report."""

    #: Extra argv every judgement of this plan carries: the rows report a carried plan proves.
    extra = ()

    def __init__(self, test, count, carried=False):
        self.test = test
        self.text = population_text(count)
        if carried:
            self.fixture = Fixture(test, files=dict(FILES), rows=[("SCRIPT_MUTATIONS", CARRIED)])
            self.fixture.head({SCRIPT: self.text})
            self.fixture.plan()
        else:
            self.fixture = changed_fixture(test, head_text=self.text)
        self.listing = listed(SCRIPT, self.text)
        shard_the_plan(self.fixture, self.listing)
        if carried:
            rows = [{"id": CARRIED[0], "verdict": "KILLED", "target": SCRIPT}]
            self.extra = ("--rows", str(self.fixture.report("rows.json", rows)))
        plan = json.loads((self.fixture.out / "plan.json").read_text(encoding="utf-8"))
        self.count = plan["python"]["count"]
        test.assertEqual(self.count, count, f"{len(self.listing)} listed")
        self.listed = {shard["shard"]: shard["mutants"] for shard in plan["python"]["shards"]}
        self.reports = {k: self.own(k) for k in range(self.count)}
        self.laid = 0

    def own(self, k):
        """The report shard `k` writes: its listed mutants, every one killed."""
        entry = file_entry(SCRIPT, self.text, {}, slot=(k, self.count))
        return report_of([entry], shard=f"{k}/{self.count}")

    def mutants(self, report):
        return report["files"][0]["mutants"]

    def write(self, slots):
        """Write `slots` (slot -> document, raw text, or None for no report) into a directory of
        its own and return it. A directory is never reused."""
        self.laid += 1
        directory = self.fixture.out / f"layout-{self.laid}"
        for slot, document in slots.items():
            if document is None:
                continue
            if isinstance(document, str):
                write_shard(directory, slot, None, raw=document)
            else:
                write_shard(directory, slot, document)
        return directory

    def read(self, done):
        """(exit, the slots the verdict named, stdout and stderr) of one judgement."""
        named = {k for k in range(self.count) if f"VOID mutation-python-shard-{k}: " in done.stdout}
        return done.returncode, named, done.stdout + done.stderr

    def lay(self, slots, cli=False):
        """Judge the `scripts` class over `slots`. A member is judged by the program's own `main`
        in this process; `cli=True` runs the program itself, the control that its entry routes
        through that same `main`."""
        directory = self.write(slots)
        if cli:
            return self.read(judged(self.fixture, "scripts", directory, *self.extra))
        return self.read(judge_in_process(self.fixture, "scripts", directory, extra=self.extra))

    def control(self, family, label, slots, wrong):
        """The end-to-end control of a family: the program itself, argv in and exit code and
        stdout out, refuses the member naming exactly its wrong slots, and says what the
        in-process judgement said over the same directory, byte for byte."""
        directory = self.write(slots)
        inside = judge_in_process(self.fixture, "scripts", directory)
        outside = judged(self.fixture, "scripts", directory)
        code, named, output = self.read(outside)
        self.test.assertEqual(
            (code, named), (3, wrong), f"the program, {family}: {label}: {output}"
        )
        self.test.assertEqual(
            (inside.returncode, inside.stdout, inside.stderr),
            (outside.returncode, outside.stdout, outside.stderr),
            f"the program and its in-process judgement differ, {family}: {label}",
        )

    def correct(self):
        return {k: copy.deepcopy(self.reports[k]) for k in range(self.count)}


def slot_members(plan):
    """(family, label, slots, wrong slots) for the slot binding: a copy of another shard's report in
    a slot, two shards' reports swapped, and a shard field absent or malformed."""
    n = plan.count
    for k, j in itertools.permutations(range(n), 2):
        slots = plan.correct()
        slots[k] = copy.deepcopy(plan.reports[j])
        yield "copy", f"slot {k} holds a copy of shard {j}'s report", slots, {k}
    for k, j in itertools.combinations(range(n), 2):
        slots = plan.correct()
        slots[k], slots[j] = slots[j], slots[k]
        yield "swap", f"shards {k} and {j} swapped", slots, {k, j}
    for k in range(n):
        for label, spelling in MALFORMED:
            slots = plan.correct()
            if spelling is None:
                del slots[k]["shard"]
            elif spelling == "null":
                slots[k]["shard"] = None
            elif isinstance(spelling, str):
                slots[k]["shard"] = spelling.format(k=k, n=n, n1=n + 1, other=(k + 1) % n)
            else:
                slots[k]["shard"] = spelling
            yield "field", f"slot {k}'s shard field is {label}", slots, {k}


def listing_members(plan):
    """(family, label, slots, wrong slots) for the listing binding: a report trimmed by each one
    mutant, with one extra mutant (another shard's, one the plan never listed, its own again), with
    one mutant swapped for another, and no report at all."""
    n = plan.count
    for k in range(n):
        for index in range(len(plan.mutants(plan.reports[k]))):
            slots = plan.correct()
            del plan.mutants(slots[k])[index]
            yield "trim", f"shard {k}'s report trimmed of its mutant {index}", slots, {k}
    for k in range(n):
        for j in range(n):
            if j == k:
                continue
            slots = plan.correct()
            plan.mutants(slots[k]).append(copy.deepcopy(plan.mutants(plan.reports[j])[0]))
            yield "extra", f"shard {k}'s report with shard {j}'s first mutant", slots, {k}
        slots = plan.correct()
        plan.mutants(slots[k]).append(ghost(9000 + k))
        yield "extra", f"shard {k}'s report with a mutant the plan never listed", slots, {k}
        slots = plan.correct()
        plan.mutants(slots[k]).append(copy.deepcopy(plan.mutants(plan.reports[k])[0]))
        yield "extra", f"shard {k}'s report with its own first mutant twice", slots, {k}
    for k in range(n):
        j = (k + 1) % n
        slots = plan.correct()
        plan.mutants(slots[k])[0] = copy.deepcopy(plan.mutants(plan.reports[j])[0])
        yield "swapped", f"shard {k}'s first mutant is shard {j}'s", slots, {k}
        slots = plan.correct()
        plan.mutants(slots[k])[0] = ghost(9500 + k)
        yield "swapped", f"shard {k}'s first mutant is one the plan never listed", slots, {k}
    for k in range(n):
        slots = plan.correct()
        slots[k] = None
        yield "missing", f"shard {k}'s report is missing", slots, {k}


class ThePythonMatrixIsSizedToTheHostedJob(unittest.TestCase):
    """SPEC-362 R14: at most twenty listed mutants a shard, clamped to 1 to 32 shards. Past 32 legs
    SPEC-415 refuses the listing whole. The numbers are literals here, so a mutant of the rule's
    constant changes what it is held against."""

    def test_the_matrix_takes_twenty_listed_mutants_a_shard_clamped_to_thirty_two(self):
        program = verdict_program()
        for listed_count, shards in ((20, 1), (21, 2), (68, 4), (32 * 20 + 1, None)):
            sized = program.python_shards([{"name": f"m{i}"} for i in range(listed_count)])
            if shards is None:
                self.assertIsNone(sized, f"{listed_count} listed")
                continue
            self.assertEqual(sized["count"], shards, f"{listed_count} listed")

    def test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped(self):
        """SPEC-415 R1 and R2: a listing that needs more than 32 legs of 20 is refused, never
        capped; at or below 32 legs the deal is unchanged."""
        program = verdict_program()
        refused = ((32 * 20 + 1, 33), (2000, 100), (5423, 272))
        for listed_count, needed in examined("refused listings", refused):
            sized = program.python_shards([{"name": f"m{i}"} for i in range(listed_count)])
            self.assertIsNone(sized and sized["count"], f"{listed_count} listed: never capped")
            self.assertEqual(program.python_needed(listed_count), needed, f"{listed_count} listed")
        sizes = ((0, 1), (1, 1), (20, 1), (21, 2), (68, 4), (32 * 20, 32))
        for listed_count, shards in examined("sized listings", sizes):
            names = [f"m{i}" for i in range(listed_count)]
            matrix = program.python_shards([{"name": name} for name in names])
            self.assertEqual(matrix["count"], shards, f"{listed_count} listed")
            dealt = [shard["mutants"] for shard in matrix["shards"]]
            self.assertEqual(
                dealt, [names[k::shards] for k in range(shards)], f"{listed_count} listed"
            )


def priced(path, count):
    """`count` listing entries of `path`, each named as the runner names a mutant."""
    return [
        {"name": f"{path}:{line}:5: replace + with - in f", "file": path} for line in range(count)
    ]


def names_of(entries):
    return [entry["name"] for entry in entries]


class ThePythonLegsAreDealtByCost(unittest.TestCase):
    """SPEC-418 R3, R4 and R7 (A1 to A5 and A9): a priced file's mutants are dealt by its price
    into the fewest legs within 1800 s, an unpriced file keeps 20 a leg, a listing the deal cannot
    fit is refused whole by name, and `prices` derives each file's price from recorded runs. The
    listings and prices are literals, so a mutant of the deal changes what it is held against."""

    def test_a_priced_listing_is_dealt_by_cost_into_the_fewest_legs_within_the_bound(self):
        """A1: a.py at 100 s holds 16 a leg, so its 30 mutants are two runs of 15, each
        (2 + 15) x 100 = 1700 s; b.py at 10 s holds 178, one run of 50, (2 + 50) x 10 = 520 s.
        Two legs cannot hold 3920 s within 1800 s each, so three legs."""
        a, b = priced("a.py", 30), priced("b.py", 50)
        dealt = verdict_program().python_by_cost(a + b, {"a.py": 100, "b.py": 10})
        self.assertEqual(
            dealt,
            {
                "count": 3,
                "shards": [
                    {"shard": 0, "mutants": names_of(a[:15]), "projected_seconds": 1700},
                    {"shard": 1, "mutants": names_of(a[15:]), "projected_seconds": 1700},
                    {"shard": 2, "mutants": names_of(b), "projected_seconds": 520},
                ],
            },
        )
        examined("dealt legs", dealt["shards"])

    def test_a_listing_the_cost_deal_cannot_fit_is_refused_whole_by_name(self):
        """A2: one mutant of a file priced at 601 s projects 3 x 601 = 1803 s, past 1800; a file
        priced at 600 s holds one mutant a leg, so 33 of them need 33 legs, past 32. 32 of them
        fit, the boundary's control."""
        program = verdict_program()
        dear = program.python_by_cost(priced("d.py", 1), {"d.py": 601})
        self.assertEqual(
            dear,
            "mutation: shards: REFUSED: d.py's control is priced at 601 s, so a leg holding one "
            "of its mutants projects 1803 s, past its 1800 s bound",
        )
        wide = program.python_by_cost(priced("c.py", 33), {"c.py": 600})
        self.assertEqual(
            wide,
            "mutation: shards: REFUSED: 33 python mutant(s) need 33 leg(s) priced at most 1800 s "
            "each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python "
            "legs: split the change, since a run is never capped",
        )
        fits = program.python_by_cost(priced("c.py", 32), {"c.py": 600})
        self.assertEqual(fits["count"], 32)
        self.assertEqual({shard["projected_seconds"] for shard in fits["shards"]}, {1800})
        examined("refused listings", [dear, wide])

    def test_a_listing_with_no_priced_file_keeps_the_count_rule(self):
        """A3: no listed file priced reads None, so `shards` keeps SPEC-415's matrix byte for
        byte; and the shipped table is the recorded run's, derived by the `prices` verb (SPEC-420
        R4)."""
        program = verdict_program()
        listing = priced("a.py", 68)
        tables = ({}, {"elsewhere.py": 5})
        for prices in tables:
            self.assertIsNone(program.python_by_cost(listing, prices), prices)
        self.assertEqual(
            program.PYTHON_CONTROL_SECONDS, recorded_table(recorded_prices(program)[1])
        )
        examined("price tables", tables)

    def test_unpriced_mutants_keep_twenty_a_leg_beside_the_priced_legs(self):
        """A4: a.py's two priced legs, then u.py's 41 unpriced mutants in three legs of at most
        20, round robin, each projected at the bound."""
        a, u = priced("a.py", 30), priced("u.py", 41)
        dealt = verdict_program().python_by_cost(a + u, {"a.py": 100})
        names = names_of(u)
        self.assertEqual(
            dealt,
            {
                "count": 5,
                "shards": [
                    {"shard": 0, "mutants": names_of(a[:15]), "projected_seconds": 1700},
                    {"shard": 1, "mutants": names_of(a[15:]), "projected_seconds": 1700},
                    {"shard": 2, "mutants": names[0::3], "projected_seconds": 1800},
                    {"shard": 3, "mutants": names[1::3], "projected_seconds": 1800},
                    {"shard": 4, "mutants": names[2::3], "projected_seconds": 1800},
                ],
            },
        )
        examined("unpriced legs", dealt["shards"][2:])

    def test_the_plan_deals_by_cost_and_refuses_whole_by_name(self):
        """A5: `shards` over a plan whose Rust class does not apply, with the table priced in
        place: on a fit, its python, cost, Rust and joint lines and exit 0; on a refusal, exit 1,
        the refusal alone, no step output and the plan's bytes unchanged."""
        program = verdict_program()
        entries = priced("a.py", 30)
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            listing = root / "python-listed.json"
            listing.write_text(
                json.dumps({"schema": program.PYTHON_SCHEMA, "listed": 30, "mutants": entries}),
                encoding="utf-8",
            )
            plan, sink = root / "plan.json", root / "github-output"
            original = json.dumps({"classes": {"rust": {"applies": False}}})

            def shards(prices):
                plan.write_text(original, encoding="utf-8")
                sink.write_text("", encoding="utf-8")
                out = io.StringIO()
                with (
                    mock.patch.object(program, "PYTHON_MAX_SHARDS", 32),
                    mock.patch.dict(program.LEG_CEILING, {"ci": 193}),
                    mock.patch.dict(program.PYTHON_CONTROL_SECONDS, prices, clear=True),
                    mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(sink)}),
                    contextlib.redirect_stdout(out),
                ):
                    code = program.main(
                        ["shards", "--plan", str(plan), "--python-listed", str(listing)]
                    )
                return code, out.getvalue()

            code, out = shards({"a.py": 100})
            self.assertEqual(
                out,
                "mutation: shards: 2 python shard(s) for 30 listed mutant(s); python legs 2 of "
                "ceiling 32\n"
                "mutation: shards: python legs dealt by cost: 2 priced leg(s), the slowest at 1700 "
                "s of its 1800 s bound; 0 unpriced leg(s) of at most 20\n"
                "mutation: shards: 1 shard(s) for 0 listed mutant(s), projected at 0 s serially; "
                "the slowest at 2141 s of its 10800 s bound; legs 1 of ceiling 190\n"
                "mutation: shards: run legs 3 of 222: python 2 of 32, rust 1 of 190; projected 93 "
                "runner-minute(s) of 35160\n",
            )
            self.assertEqual(code, 0)
            written = json.loads(plan.read_text(encoding="utf-8"))
            self.assertEqual(
                [shard["mutants"] for shard in written["python"]["shards"]],
                [names_of(entries[:15]), names_of(entries[15:])],
            )
            self.assertIn("python_shards=2\n", sink.read_text(encoding="utf-8"))
            code, out = shards({"a.py": 601})
            self.assertEqual(
                out,
                "mutation: shards: REFUSED: a.py's control is priced at 601 s, so a leg holding "
                "one of its mutants projects 1803 s, past its 1800 s bound\n",
            )
            self.assertEqual(code, 1)
            self.assertEqual(plan.read_text(encoding="utf-8"), original)
            self.assertEqual(sink.read_text(encoding="utf-8"), "")
            examined("plans", [written])

    def test_the_prices_verb_derives_each_files_price_from_the_recorded_runs(self):
        """A9: a.py's price is the dearer of its control, 12 s, and its mutants' mean, 35.5 s,
        rounded up; b.py's is its control's 2.4 s rounded up; c.py, uncovered, costs the floor of
        1 s. The torn line is counted and never read; a record of no line reads VOID."""
        program = verdict_program()
        legs = {
            0: [
                '{"kind": "control", "path": "scripts/a.py", "ran": 4, "failures": 0, '
                '"seconds": 12.0, "bound": 60.0, "shard": "0/2", "run": "4242", "attempt": "1"}',
                '{"kind": "mutant", "path": "scripts/a.py", "name": "scripts/a.py:3:5: m", '
                '"seconds": 30.0, "outcome": "killed"}',
                '{"kind": "mutant", "path": "scripts/a.py", "name": "scripts/a.py:4:5: m", '
                '"seconds": 41.0, "outcome": "killed"}',
                '{"kind": "mutant", "path": "scripts/a.py", "na',
            ],
            1: [
                '{"kind": "control", "path": "scripts/b.py", "ran": 2, "failures": 0, '
                '"seconds": 2.4, "bound": 60.0, "shard": "1/2", "run": "4242", "attempt": "1"}',
                '{"kind": "mutant", "path": "scripts/b.py", "name": "scripts/b.py:2:5: m", '
                '"seconds": 1.0, "outcome": "survived"}',
                '{"kind": "uncovered", "path": "scripts/c.py", "seconds": 0, "shard": "1/2", '
                '"run": "4242", "attempt": "1"}',
            ],
        }

        def prices(root):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = program.main(["prices", "--costs", str(root)])
            return code, out.getvalue()

        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            for leg, lines in legs.items():
                directory = root / f"mutation-python-shard-{leg}"
                directory.mkdir()
                (directory / "costs.jsonl").write_text("\n".join(lines) + "\n", encoding="utf-8")
            code, out = prices(root)
            self.assertEqual(
                out,
                '    "scripts/a.py": 36,  # run 4242 attempt 1 shard 0/2\n'
                '    "scripts/b.py": 3,  # run 4242 attempt 1 shard 1/2\n'
                '    "scripts/c.py": 1,  # run 4242 attempt 1 shard 1/2\n'
                "mutation: prices: examined 6 record(s) in 2 file(s); torn 1\n",
            )
            self.assertEqual(code, 0)
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "costs.jsonl").write_text(legs[0][3] + "\n", encoding="utf-8")
            code, out = prices(root)
            self.assertEqual(
                out,
                "mutation: prices: examined 0 record(s) in 1 file(s); torn 1\n"
                f"mutation: prices: VOID {root} holds no cost record\n",
            )
            self.assertEqual(code, 1)
        examined("recorded legs", legs)


class EachArmOfTheCostDealIsObserved(unittest.TestCase):
    """SPEC-418 R3, R4 and R7, arm by arm: how a priced file is cut into runs, the order the runs
    are dealt in, where the search for the fewest legs starts and where it stops, the run's share
    of 32 python legs, the plan's cost and joint lines, and how `prices` reads its records. Every
    listing, price and record is a literal, and each docstring spells the arithmetic its
    expectation rests on."""

    def dealt(self, listing, prices, legs):
        """`python_by_cost` over `listing` at `prices`, held whole: `legs` is each leg's listing
        indices and projected seconds, in leg order."""
        expected = {
            "count": len(legs),
            "shards": [
                {
                    "shard": shard,
                    "mutants": [listing[index]["name"] for index in indices],
                    "projected_seconds": seconds,
                }
                for shard, (indices, seconds) in enumerate(legs)
            ],
        }
        self.assertEqual(verdict_program().python_by_cost(listing, prices), expected)

    def test_a_lone_run_pays_its_files_control_and_sentinel_once(self):
        """p.py at 100 s: its 3 mutants are one run on one leg, (2 + 3) x 100 = 500 s, not the
        300 s its mutants alone take."""
        listing = priced("p.py", 3)
        self.dealt(listing, {"p.py": 100}, [([0, 1, 2], 500)])
        examined("lone runs", [listing])

    def test_runs_that_fit_one_leg_share_it_and_the_search_stops_at_the_first_fit(self):
        """t.py at 200 s holds 1800 // 200 - 2 = 7 a leg, so its 4 mutants are one run of
        (2 + 4) x 200 = 1200 s, and s.py's 1 mutant is (2 + 1) x 200 = 600 s: 1800 s together,
        which one leg holds exactly. The floor, 1200 + 600 = 1800 s, starts the search at one
        leg, and the search stops at the first count that fits."""
        listing = priced("t.py", 4) + priced("s.py", 1)
        self.dealt(listing, {"t.py": 200, "s.py": 200}, [([0, 1, 2, 3, 4], 1800)])
        examined("shared legs", [listing])

    def test_a_files_mutants_are_cut_into_runs_that_differ_by_at_most_one_larger_first(self):
        """q.py at 225 s holds 6 a leg, so its 7 mutants are cut into 2 runs, the larger first: 4
        at (2 + 4) x 225 = 1350 s and 3 at (2 + 3) x 225 = 1125 s. r.py at 360 s holds 3 a leg,
        so its 7 are cut into runs of 3, 2 and 2: 1800 s, then 1440 s twice. Two legs would put
        the last run beside the second at 1440 + 2 x 360 = 2160 s, so three legs."""
        cases = (
            ("q.py", 225, [([0, 1, 2, 3], 1350), ([4, 5, 6], 1125)]),
            ("r.py", 360, [([0, 1, 2], 1800), ([3, 4], 1440), ([5, 6], 1440)]),
        )
        for path, price, legs in examined("cut files", cases):
            with self.subTest(path=path):
                self.dealt(priced(path, 7), {path: price}, legs)

    def test_the_heaviest_run_is_dealt_first_and_a_tie_goes_to_the_lower_path(self):
        """a.py's 1 mutant at 330 s is (2 + 1) x 330 = 990 s and b.py's 4 at 170 s are
        (2 + 4) x 170 = 1020 s: b.py is heavier, so it takes leg 0. At 400 s and 200 s each run
        weighs 1200 s, so the lower path, a.py, takes leg 0. Each pair, 2010 s or 2400 s, needs
        two legs, one run each."""
        listing = priced("a.py", 1) + priced("b.py", 4)
        cases = (
            ({"a.py": 330, "b.py": 170}, [([1, 2, 3, 4], 1020), ([0], 990)]),
            ({"a.py": 400, "b.py": 200}, [([0], 1200), ([1, 2, 3, 4], 1200)]),
        )
        for prices, legs in examined("dealing orders", cases):
            with self.subTest(prices=prices):
                self.dealt(listing, prices, legs)

    def test_a_file_priced_past_a_third_of_the_bound_is_refused_by_name(self):
        """At 600 s a leg holds one mutant beside its control and sentinel, 3 x 600 = 1800 s, so
        x.py's 2 mutants take two legs; at 900 s one mutant projects 3 x 900 = 2700 s, and the
        listing is refused by the file's name."""
        listing = priced("x.py", 2)
        self.dealt(listing, {"x.py": 600}, [([0], 1800), ([1], 1800)])
        self.assertEqual(
            verdict_program().python_by_cost(listing, {"x.py": 900}),
            "mutation: shards: REFUSED: x.py's control is priced at 900 s, so a leg holding one "
            "of its mutants projects 2700 s, past its 1800 s bound",
        )
        examined("priced files", [600, 900])

    def test_a_file_at_one_second_holds_1798_mutants_a_leg(self):
        """At the 1 s floor a leg holds 1800 // 1 - 2 = 1798 mutants: 1798 of f.py's are one run
        of 1800 s, and 1799 are cut into runs of 900 and 899, (2 + 900) x 1 = 902 s and 901 s,
        whose floor, 1801 s, starts the search at two legs."""
        cases = (
            (1798, [(list(range(1798)), 1800)]),
            (1799, [(list(range(900)), 902), (list(range(900, 1799)), 901)]),
        )
        for count, legs in examined("one-second files", cases):
            with self.subTest(count=count):
                self.dealt(priced("f.py", count), {"f.py": 1}, legs)

    def test_priced_and_unpriced_legs_are_held_together_to_the_runs_thirty_two(self):
        """p.py's 1 priced mutant takes one leg beside u.py's unpriced mutants at 20 a leg: 620 of
        them take 31 legs, 32 in all, which fits; 621 take 32 legs, 33 in all, refused whole."""
        program = verdict_program()
        prices = {"p.py": 100}
        fits = program.python_by_cost(priced("p.py", 1) + priced("u.py", 620), prices)
        self.assertEqual(
            (fits["count"], [len(shard["mutants"]) for shard in fits["shards"]]),
            (32, [1] + [20] * 31),
        )
        self.assertEqual(
            program.python_by_cost(priced("p.py", 1) + priced("u.py", 621), prices),
            "mutation: shards: REFUSED: 622 python mutant(s) need 1 leg(s) priced at most 1800 s "
            "each and 32 unpriced leg(s) of at most 20, more than the pull request run's 32 "
            "python legs: split the change, since a run is never capped",
        )
        examined("shared listings", [620, 621])

    def test_the_plans_cost_line_counts_only_its_priced_legs_and_the_joint_line_its_budget(self):
        """`shards` with a.py's 3 mutants priced at 100 s, one leg of (2 + 3) x 100 = 500 s, and
        u.py's 21 unpriced mutants in two legs at the 1800 s bound: the cost line counts the one
        priced leg and reads the slowest of it alone, and the joint line holds the run to
        (32 x 1800 + 190 x 10800) // 60 = 35160 runner-minutes. The Rust lines are A5's own."""
        program = verdict_program()
        entries = priced("a.py", 3) + priced("u.py", 21)
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            listing = root / "python-listed.json"
            listing.write_text(
                json.dumps({"schema": program.PYTHON_SCHEMA, "listed": 24, "mutants": entries}),
                encoding="utf-8",
            )
            plan, sink = root / "plan.json", root / "github-output"
            plan.write_text(json.dumps({"classes": {"rust": {"applies": False}}}), encoding="utf-8")
            sink.write_text("", encoding="utf-8")
            out = io.StringIO()
            with (
                mock.patch.object(program, "PYTHON_MAX_SHARDS", 32),
                mock.patch.dict(program.LEG_CEILING, {"ci": 193}),
                mock.patch.dict(program.PYTHON_CONTROL_SECONDS, {"a.py": 100}, clear=True),
                mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(sink)}),
                contextlib.redirect_stdout(out),
            ):
                code = program.main(
                    ["shards", "--plan", str(plan), "--python-listed", str(listing)]
                )
        self.assertEqual(
            out.getvalue(),
            "mutation: shards: 3 python shard(s) for 24 listed mutant(s); python legs 3 of "
            "ceiling 32\n"
            "mutation: shards: python legs dealt by cost: 1 priced leg(s), the slowest at 500 s of "
            "its 1800 s bound; 2 unpriced leg(s) of at most 20\n"
            "mutation: shards: 1 shard(s) for 0 listed mutant(s), projected at 0 s serially; the "
            "slowest at 2141 s of its 10800 s bound; legs 1 of ceiling 190\n"
            "mutation: shards: run legs 4 of 222: python 3 of 32, rust 1 of 190; projected 105 "
            "runner-minute(s) of 35160\n",
        )
        self.assertEqual(code, 0)
        examined("planned listings", [entries])

    def prices(self, lines):
        """`prices` over one leg's `costs.jsonl` holding `lines`: (its exit, its standard output).
        An argument refusal reads as `refused <exit>` in place of the verb's own exit."""
        program = verdict_program()
        with tempfile.TemporaryDirectory() as scratch:
            leg = Path(scratch) / "mutation-python-shard-0"
            leg.mkdir()
            (leg / "costs.jsonl").write_text("\n".join(lines) + "\n", encoding="utf-8")
            out = io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
                try:
                    code = program.main(["prices", "--costs", scratch])
                except SystemExit as refused:
                    code = f"refused {refused.code}"
        return code, out.getvalue()

    def test_the_prices_verb_reads_past_a_blank_and_a_torn_line_and_averages_mutants(self):
        """A blank line is passed over and the record after it read; a dict whose kind is off the
        vocabulary is torn, though its path and seconds are well formed, and the record after it
        read; m.py's price is the dearer of its control, 2 s, and its mutants' mean,
        (9 + 3) / 2 = 6 s, and only its control names where it ran."""
        cases = (
            (
                "a blank line between records",
                [
                    '{"kind": "control", "path": "x.py", "seconds": 4, "shard": "0/1", '
                    '"run": "7", "attempt": "1"}',
                    "",
                    '{"kind": "control", "path": "y.py", "seconds": 2, "shard": "0/1", '
                    '"run": "7", "attempt": "1"}',
                ],
                '    "x.py": 4,  # run 7 attempt 1 shard 0/1\n'
                '    "y.py": 2,  # run 7 attempt 1 shard 0/1\n'
                "mutation: prices: examined 2 record(s) in 1 file(s); torn 0\n",
            ),
            (
                "a torn record before a whole one",
                [
                    '{"kind": "bogus", "path": "t.py", "seconds": 9, "shard": "0/1", '
                    '"run": "7", "attempt": "1"}',
                    '{"kind": "control", "path": "z.py", "seconds": 3, "shard": "0/1", '
                    '"run": "7", "attempt": "1"}',
                ],
                '    "z.py": 3,  # run 7 attempt 1 shard 0/1\n'
                "mutation: prices: examined 1 record(s) in 1 file(s); torn 1\n",
            ),
            (
                "mutants averaged beside a control",
                [
                    '{"kind": "control", "path": "m.py", "seconds": 2, "shard": "0/1", '
                    '"run": "7", "attempt": "1"}',
                    '{"kind": "mutant", "path": "m.py", "name": "m.py:1:5: m", "seconds": 9, '
                    '"outcome": "killed"}',
                    '{"kind": "mutant", "path": "m.py", "name": "m.py:2:5: m", "seconds": 3, '
                    '"outcome": "killed"}',
                ],
                '    "m.py": 6,  # run 7 attempt 1 shard 0/1\n'
                "mutation: prices: examined 3 record(s) in 1 file(s); torn 0\n",
            ),
        )
        for label, lines, printed in examined("cost files", cases):
            with self.subTest(label):
                self.assertEqual(self.prices(lines), (0, printed))

    def test_the_prices_verb_refuses_to_run_without_its_costs_directory(self):
        """`prices` with no `--costs` is an argument error that names the missing option."""
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            with self.assertRaises(SystemExit) as refused:
                verdict_program().main(["prices"])
        self.assertEqual(refused.exception.code, 2)
        self.assertTrue(err.getvalue().endswith(": error: prices needs --costs\n"), err.getvalue())
        examined("refused invocations", [["prices"]])


class TheReportIsBoundToItsSlotAndItsListing(unittest.TestCase):
    def population(self, name, members, expected):
        """Judge the correct layout, then every member of `members` over each plan; each member must
        be refused, naming exactly its wrong slots. `expected` gives each family's member count."""
        total = refused = accepted = 0
        controlled = {}
        for count in COUNTS:
            plan = Plan(self, count)
            code, named, output = plan.lay(plan.correct(), cli=True)
            self.assertEqual((code, named), (0, set()), f"the control, {count} shards: {output}")
            self.assertRegex(output, rf"(?m)^examined {len(plan.listing)}$", output)
            found = list(members(plan))
            families = {}
            for family, *_ in found:
                families[family] = families.get(family, 0) + 1
            self.assertEqual(families, expected(plan), f"{count} shards")
            for family, label, slots, wrong in examined(f"{name} members of {count} shards", found):
                total += 1
                code, named, output = plan.lay(slots)
                controlled.setdefault(family, (plan, label, slots, wrong))
                if code == 0:
                    accepted += 1
                    continue
                refused += 1
                self.assertEqual(code, 3, f"{label}: {output}")
                self.assertEqual(named, wrong, f"{label}: {output}")
        print(f"{name}: {total} members, {refused} refused, {accepted} accepted")
        self.assertEqual(accepted, 0, f"{accepted} of {total} members accepted")
        self.assertEqual(refused, total)
        run_controls(controlled)
        return total

    def test_a_report_counts_only_in_its_own_slot(self):
        def expected(plan):
            n = plan.count
            return {
                "copy": n * (n - 1),
                "swap": n * (n - 1) // 2,
                "field": n * len(MALFORMED),
            }

        total = self.population("slot binding", slot_members, expected)
        self.assertEqual(
            total,
            sum(
                sum(v for v in [n * (n - 1), n * (n - 1) // 2, n * len(MALFORMED)]) for n in COUNTS
            ),
        )

    def test_a_report_counts_only_the_mutants_its_shard_lists(self):
        def expected(plan):
            n = plan.count
            return {
                "trim": len(plan.listing),
                "extra": n * (n - 1) + 2 * n,
                "swapped": 2 * n,
                "missing": n,
            }

        self.population("listing binding", listing_members, expected)

    def test_a_plan_that_lists_no_mutants_for_a_shard_refuses_its_report(self):
        plan = Plan(self, 2)
        path = plan.fixture.out / "plan.json"
        original = path.read_text(encoding="utf-8")
        variants = {
            "no entry for the shard": lambda entries: [e for e in entries if e["shard"] != 1],
            "an entry with no listing": lambda entries: [
                {k: v for k, v in e.items() if k != "mutants"} if e["shard"] == 1 else e
                for e in entries
            ],
            "an entry whose listing is not a list": lambda entries: [
                dict(e, mutants="all") if e["shard"] == 1 else e for e in entries
            ],
        }
        try:
            for label, change in variants.items():
                document = json.loads(original)
                document["python"]["shards"] = change(document["python"]["shards"])
                path.write_text(json.dumps(document), encoding="utf-8")
                code, named, output = plan.lay(plan.correct(), cli=True)
                self.assertEqual((code, named), (3, {1}), f"{label}: {output}")
                self.assertIn(
                    "mutation-python-shard-1: the plan lists no mutants for this shard", output
                )
        finally:
            path.write_text(original, encoding="utf-8")

    def test_a_mutant_record_that_is_not_an_object_is_an_extra_mutant(self):
        plan = Plan(self, 2)
        pin = message_pins(self, plan)["a mutant record that is not an object"]
        run_pin(self, plan, pin, cli=True)


#: The plans the one-reader population is generated over: this many shards each.
READER_COUNTS = (1, 2, 3, 4)
#: A path the Python lane does not read: a document, a Rust file, an empty path, no path at all, the
#: oracle's path (a class that does not apply in these plans), and a test module.
UNREAD_PATHS = (
    "README.md",
    "crates/core/src/lib.rs",
    "",
    None,
    "tools/parity-oracle/generate.py",
    "scripts/tests/test_guard.py",
)
#: A container of another JSON type than the list a `mutants` holds, truthy and falsy.
NOT_A_LIST = (5, 1.5, True, "abc", None, 0, False, "", {})
#: A `files` of another JSON type than the list a report holds.
FILES_NOT_A_LIST = (5, None, True, "abc", {})
#: An outcome the runner's own vocabulary does not hold, spelled as a report might carry it.
ABSENT = object()
OFF_VOCABULARY = (
    ABSENT,
    None,
    1,
    [],
    "pending",
    "KILLED",
    "killed ",
    "Survived",
    " survived",
    "survived\n",
    "survive",
)


def runner_outcomes():
    """The runner's own outcome vocabulary, read from the runner module."""
    from test_mutation_python_verdict import runner

    return tuple(runner().OUTCOMES)


def read_entry(document, path=SCRIPT):
    """The entry of `document` that holds its mutants, made when a report has none."""
    if not document["files"]:
        document["files"].append({"path": path, "modules": [], "byte_readers": [], "mutants": []})
    return document["files"][0]


def reader_members(plan):
    """(family, label, slots, wrong slots) for the one-reader class: every way a shard's report
    holds a container of another JSON type, an outcome off the runner's vocabulary, or a listed
    mutant under an entry the lane does not read, generated from the plan's own shards. The
    binding and the judge must read each alike: refused by name, never a crash, never accepted."""
    for k in range(plan.count):
        held = plan.reports[k]["files"][0]["mutants"] if plan.reports[k]["files"] else []
        for value in NOT_A_LIST:
            if value:
                slots = plan.correct()
                read_entry(slots[k])["mutants"] = value
                yield "mutants", f"shard {k}: its entry's mutants is {value!r}", slots, {k}
            slots = plan.correct()
            read_entry(slots[k])
            slots[k]["files"].append({"path": SCRIPT, "mutants": value})
            yield "extra entry", f"shard {k}: an extra entry whose mutants is {value!r}", slots, {k}
        for value in FILES_NOT_A_LIST:
            slots = plan.correct()
            slots[k]["files"] = value
            yield "files", f"shard {k}: its files is {value!r}", slots, {k}
        for entry_value in (5, None, "abc", []):
            slots = plan.correct()
            read_entry(slots[k])
            slots[k]["files"].append(entry_value)
            yield "entry", f"shard {k}: an entry that is {entry_value!r}", slots, {k}
        slots = plan.correct()
        slots[k]["files"].append({"path": "README.md", "mutants": [ghost(9900 + k)]})
        yield (
            "unread extra",
            f"shard {k}: a mutant the plan never listed, under README.md",
            slots,
            {k},
        )
        for value in (5, True, "abc", {"reader": 1}):
            slots = plan.correct()
            read_entry(slots[k])["byte_readers"] = value
            yield "byte readers", f"shard {k}: its entry's byte readers is {value!r}", slots, {k}
        for index in (0,) if held else ():
            for value in OFF_VOCABULARY:
                slots = plan.correct()
                record = slots[k]["files"][0]["mutants"][index]
                if value is ABSENT:
                    del record["outcome"]
                else:
                    record["outcome"] = value
                shown = "absent" if value is ABSENT else repr(value)
                yield "outcome", f"shard {k}: mutant {index}'s outcome is {shown}", slots, {k}
            for value in (5, None, "abc", []):
                slots = plan.correct()
                slots[k]["files"][0]["mutants"][index] = value
                yield "record", f"shard {k}: mutant {index} is {value!r}", slots, {k}
            for value in (5, None, ABSENT):
                slots = plan.correct()
                record = slots[k]["files"][0]["mutants"][index]
                if value is ABSENT:
                    del record["name"]
                else:
                    record["name"] = value
                shown = "absent" if value is ABSENT else repr(value)
                yield "name", f"shard {k}: mutant {index}'s name is {shown}", slots, {k}
            for path in UNREAD_PATHS:
                for outcome in ("killed", "survived"):
                    slots = plan.correct()
                    record = slots[k]["files"][0]["mutants"].pop(index)
                    record["outcome"] = outcome
                    moved = {"path": path, "mutants": [record]}
                    if path is None:
                        del moved["path"]
                    slots[k]["files"].append(moved)
                    yield (
                        "unread entry",
                        f"shard {k}: mutant {index}, {outcome}, filed under path {path!r}",
                        slots,
                        {k},
                    )


class TheBindingAndTheJudgeReadOneReport(unittest.TestCase):
    """SPEC-126 A10: one reader yields exactly the records the judge judges, so a report whose
    containers are of another JSON type, whose outcome is off the runner's vocabulary, or whose
    listed mutant sits under an entry the lane does not read, is VOID naming its shard."""

    def tally(self, plan, members, what, controlled):
        """Judge every member; each lands in exactly one bucket: refused (exit 3 naming exactly its
        wrong slots), crashed (a traceback), accepted (exit 0) or other. A crash is caught and
        counted here, never raised, so the count reads the rule and not the harness."""
        counts = {"refused": 0, "crashed": 0, "accepted": 0, "other": 0}
        firsts = {}
        for family, label, slots, wrong in examined(what, list(members)):
            code, named, output = plan.lay(slots)
            controlled.setdefault(family, (plan, label, slots, wrong))
            if code == 1 and "Traceback" in output:
                bucket = "crashed"
            elif code == 0:
                bucket = "accepted"
            elif code == 3 and named == wrong:
                bucket = "refused"
            else:
                bucket = "other"
            counts[bucket] += 1
            if bucket != "refused":
                firsts.setdefault((family, bucket), label)
        return counts, firsts

    def test_the_runner_vocabulary_is_read_from_the_runner(self):
        self.assertIn("killed", runner_outcomes())
        self.assertNotIn("KILLED", runner_outcomes())

    def test_a_report_is_read_as_the_judge_reads_it(self):
        totals = {"refused": 0, "crashed": 0, "accepted": 0, "other": 0}
        firsts = {}
        families = set()
        controlled = {}
        for count in READER_COUNTS:
            plan = Plan(self, count)
            code, named, output = plan.lay(plan.correct(), cli=True)
            self.assertEqual((code, named), (0, set()), f"the control, {count} shards: {output}")
            families |= {member[0] for member in reader_members(plan)}
            counts, found = self.tally(
                plan, reader_members(plan), f"members of {count} shards", controlled
            )
            firsts.update({key: label for key, label in found.items() if key not in firsts})
            for bucket, number in counts.items():
                totals[bucket] += number
        total = sum(totals.values())
        print(
            f"examined {total} member(s), {totals['refused']} refused, {totals['crashed']} "
            f"crashed, {totals['accepted']} accepted"
        )
        for (family, bucket), label in sorted(firsts.items()):
            print(f"  {bucket}: {family}: {label}")
        self.assertEqual(
            sorted(families),
            [
                "byte readers",
                "entry",
                "extra entry",
                "files",
                "mutants",
                "name",
                "outcome",
                "record",
                "unread entry",
                "unread extra",
            ],
        )
        self.assertEqual(totals["refused"], total, totals)
        run_controls(controlled)

    def test_a_shard_that_lists_no_mutant_reads_its_empty_report_and_refuses_the_rest(self):
        plan = EmptyReaderPlan(self)
        code, named, output = plan.lay(plan.correct(), cli=True)
        self.assertEqual((code, named), (3, set()), output)
        self.assertIn("the scripts class applies and nothing was examined", output)
        controlled = {}
        counts, firsts = self.tally(plan, reader_members(plan), "empty-listing members", controlled)
        total = sum(counts.values())
        print(
            f"examined {total} member(s), {counts['refused']} refused, {counts['crashed']} "
            f"crashed, {counts['accepted']} accepted"
        )
        for (family, bucket), label in sorted(firsts.items()):
            print(f"  {bucket}: {family}: {label}")
        self.assertEqual(counts["refused"], total, counts)
        run_controls(controlled)


class TheInProcessJudgeIsTheProgram(unittest.TestCase):
    """The populations above judge each member by the program's own `main` in this process, which
    is what keeps them cheap. This is the checked-in proof that doing so loses nothing: over a
    pinned subset, the program itself and the in-process judgement give the same exit code, stdout
    and stderr for the same directory."""

    def pinned(self, plan, sources):
        """Per family, its first and its last member, and the first member of every outcome the
        in-process judge gives inside it; then the correct layout, an accepted member."""
        picked = [("control", "the correct layout", plan.correct())]
        for members in sources:
            by_family = {}
            for family, label, slots, _ in members(plan):
                by_family.setdefault(family, []).append((family, label, slots))
            for family, found in by_family.items():
                chosen = {0, len(found) - 1}
                seen = {}
                for index, (_, _, slots) in enumerate(found):
                    code, named, _ = plan.lay(slots)
                    seen.setdefault((code, len(named)), index)
                chosen |= set(seen.values())
                picked += [found[index] for index in sorted(chosen)]
        return picked

    def test_the_program_and_its_in_process_judgement_agree(self):
        subset = []
        for plan in (Plan(self, 2), Plan(self, 3)):
            sources = (
                slot_members,
                listing_members,
                reader_members,
                skipped_members,
                judge_members,
            )
            for member in self.pinned(plan, sources):
                subset.append((plan, member))
        empty = EmptyReaderPlan(self)
        subset += [(empty, member) for member in self.pinned(empty, (reader_members,))]
        families = {member[0] for _, member in subset}
        self.assertEqual(
            sorted(families),
            sorted(
                [
                    "byte readers",
                    "control",
                    "copy",
                    "entry",
                    "extra",
                    "extra entry",
                    "field",
                    "files",
                    "missing",
                    "mutants",
                    "name",
                    "outcome",
                    "record",
                    "skipped alone",
                    "skipped then bad",
                    "slot skipped",
                    "survivor",
                    "swap",
                    "swapped",
                    "timeout",
                    "trim",
                    "unread entry",
                    "unread extra",
                    "void entry",
                ]
            ),
        )
        laid = [
            (plan, family, label, plan.write(slots))
            for plan, (family, label, slots) in examined("pinned members", subset)
        ]

        # The in-process judge redirects this process's stdout, so it runs here, one at a time;
        # the program's own processes are independent of it, and they run side by side.
        inside = [
            judge_in_process(plan.fixture, "scripts", directory) for plan, _, _, directory in laid
        ]

        def outside_of(member):
            plan, _, _, directory = member
            return judged(plan.fixture, "scripts", directory)

        with ThreadPoolExecutor(max_workers=8) as pool:
            outsides = list(pool.map(outside_of, laid))
        results = [
            (family, label, one, other)
            for (_, family, label, _), one, other in zip(laid, inside, outsides, strict=True)
        ]
        table = {}
        different = []
        for family, label, inside, outside in results:
            row = table.setdefault(family, {})
            row[outside.returncode] = row.get(outside.returncode, 0) + 1
            if (inside.returncode, inside.stdout, inside.stderr) != (
                outside.returncode,
                outside.stdout,
                outside.stderr,
            ):
                different.append(f"{family}: {label}")
        for family, row in sorted(table.items()):
            shown = ", ".join(f"exit {code}: {n}" for code, n in sorted(row.items()))
            print(f"  pinned {family}: {shown}")
        codes = {code for row in table.values() for code in row}
        print(
            f"pinned {len(results)} member(s), {len(different)} differ, exit codes {sorted(codes)}"
        )
        self.assertEqual(different, [])
        self.assertEqual(codes, {0, 1, 3})
        self.assertEqual(table["survivor"].keys(), {1})
        self.assertEqual(table["skipped alone"].keys(), {0})
        self.assertEqual(table["skipped then bad"].keys(), {3})


class EmptyReaderPlan(Plan):
    """A plan whose one shard lists no mutant: the planner's own output for zero listed mutants."""

    def __init__(self, test):
        self.test = test
        self.text = "def f0(x):\n    return x + 0\n"
        self.fixture = changed_fixture(test, head_text=self.text)
        self.listing = []
        shard_the_plan(self.fixture, [])
        plan = json.loads((self.fixture.out / "plan.json").read_text(encoding="utf-8"))
        self.count = plan["python"]["count"]
        test.assertEqual(self.count, 1)
        test.assertEqual({s["shard"]: s["mutants"] for s in plan["python"]["shards"]}, {0: []})
        self.reports = {0: report_of([], shard="0/1")}
        self.laid = 0


if __name__ == "__main__":
    unittest.main()


#: The plans the skipped-entry population is generated over: two shards, and four.
SKIP_COUNTS = (2, 4)
#: The functions whose loops read a shard's report: the slot reader, the shard reader, the judge.
READER_FUNCTIONS = ("python_reports", "read_python_shard", "judge_python")


def loop_exits():
    """The census (SPEC-126 A10): every `continue` and `break` (kind Loop) and every early `return` inside a
    loop in the functions that read a shard's report, as (function, kind, line), in source order,
    from the verdict program's own syntax tree. A skipped entry is an exit; the member that puts a
    bad entry behind it is generated per exit, so an exit added later is a miscount here."""
    tree = ast.parse(VERDICT.read_text(encoding="utf-8"))
    found = []
    for function in ast.walk(tree):
        if not isinstance(function, ast.FunctionDef) or function.name not in READER_FUNCTIONS:
            continue
        for loop in ast.walk(function):
            if not isinstance(loop, (ast.For, ast.While)):
                continue
            for node in ast.walk(loop):
                if isinstance(node, (ast.Continue, ast.Break, ast.Return)):
                    kind = "Return" if isinstance(node, ast.Return) else "Loop"
                    found.append((function.name, kind, node.lineno))
    return sorted(set(found))


#: Per exit of the census, (function, kind), how many the program holds: the table the members are
#: generated from. The shard reader's early returns are its refusals, in source order.
EXITS = {
    ("python_reports", "Loop"): 2,
    ("read_python_shard", "Loop"): 1,
    ("read_python_shard", "Return"): 6,
    ("judge_python", "Loop"): 3,
}
#: The entries a shard reader refuses, one per early return of `read_python_shard`, in source order.
BAD_ENTRIES = (
    ("an entry that is not an object", lambda: 5),
    ("a mutants that is not a list", lambda: {"path": SCRIPT, "mutants": "abc"}),
    (
        "a mutant filed under a path no class reads",
        lambda: {"path": "README.md", "mutants": [ghost(9900)]},
    ),
    (
        "a byte readers that is not a list",
        lambda: {"path": SCRIPT, "mutants": [], "byte_readers": "x"},
    ),
    ("a mutant record that is not an object", lambda: {"path": SCRIPT, "mutants": [5]}),
    (
        "an outcome off the runner's vocabulary",
        lambda: {"path": SCRIPT, "mutants": [dict(ghost(9901), outcome="pending")]},
    ),
)
#: The entries a shard reader skips: a path no class reads, filed with no mutants, or with the key absent.
SKIPPED_ENTRIES = (
    {"path": "README.md", "mutants": []},
    {"path": "crates/core/src/lib.rs"},
    {"mutants": []},
)


def skipped_members(plan):
    """(family, label, slots, wrong slots) for a skipped entry that comes BEFORE an entry whose
    reading changes the verdict. In a report, a skipped entry and then each refused entry; with the
    skipped entry alone the report is read and accepted. Across slots, a missing or an unreadable
    report and then a later slot whose report is another shard's."""
    n = plan.count
    for k in range(n):
        for index, skipped in enumerate(SKIPPED_ENTRIES):
            slots = plan.correct()
            slots[k]["files"].append(copy.deepcopy(skipped))
            yield (
                "skipped alone",
                f"shard {k}: skipped entry {index} and nothing after",
                slots,
                set(),
            )
            for what, make in BAD_ENTRIES:
                slots = plan.correct()
                slots[k]["files"] += [copy.deepcopy(skipped), make()]
                yield (
                    "skipped then bad",
                    f"shard {k}: skipped entry {index}, then {what}",
                    slots,
                    {k},
                )
    for k in range(n - 1):
        for how, lost in (("missing", None), ("unreadable", "{not json")):
            for j in range(k + 1, n):
                slots = plan.correct()
                slots[k] = lost
                slots[j] = copy.deepcopy(plan.reports[(j + 1) % n])
                yield (
                    "slot skipped",
                    f"slot {k} {how}, then slot {j} holds another's report",
                    slots,
                    {k, j},
                )


def judged_lines(plan, slots):
    """The judge's whole output for `slots`, in process."""
    return plan.lay(slots)


def judge_members(plan):
    """(family, label, slots, lines the judge must say) for an entry or a mutant the judge passes
    over BEFORE one that changes the verdict: an entry that is VOID, a timeout, a survivor."""
    n = plan.count
    for k in range(n):
        if len(plan.mutants(plan.reports[k])) < 2:
            continue
        slots = plan.correct()
        entry = slots[k]["files"][0]
        first, second = entry["mutants"][0], entry["mutants"][1]
        voided = {
            "path": SCRIPT,
            "modules": [],
            "byte_readers": [],
            "mutants": [],
            "void": "no tests",
        }
        slots[k]["files"].insert(0, voided)
        second["outcome"] = "survived"
        yield (
            "void entry",
            f"shard {k}: a VOID entry, then a survivor",
            slots,
            (
                f"VOID {SCRIPT}: no tests",
                f"SURVIVED {second['name']}",
            ),
        )
        slots = plan.correct()
        entry = slots[k]["files"][0]
        first, second = entry["mutants"][0], entry["mutants"][1]
        first["outcome"], second["outcome"] = "timeout", "survived"
        yield (
            "timeout",
            f"shard {k}: a timeout, then a survivor",
            slots,
            (
                f"VOID timeout: {first['name']}",
                f"SURVIVED {second['name']}",
            ),
        )
        slots = plan.correct()
        entry = slots[k]["files"][0]
        first, second = entry["mutants"][0], entry["mutants"][1]
        first["outcome"], second["outcome"] = "survived", "uncovered"
        yield (
            "survivor",
            f"shard {k}: a survivor, then an uncovered mutant",
            slots,
            (
                f"SURVIVED {first['name']}",
                f"UNCOVERED {second['name']}",
            ),
        )


class ASkippedEntryDoesNotHideALaterOne(unittest.TestCase):
    """SPEC-126 A10: the readers of a shard's report pass over an entry, a slot or a mutant and go
    on. Every such exit, found by an AST census, has a member that puts an entry whose reading
    changes the verdict BEHIND it; an exit that stopped the loop instead would leave that entry
    unread and the verdict green."""

    def test_the_census_of_loop_exits_is_the_table_the_members_come_from(self):
        census = examined("loop exit(s) in the readers of a shard's report", loop_exits())
        by_kind = {}
        for function, kind, _ in census:
            by_kind[(function, kind)] = by_kind.get((function, kind), 0) + 1
        self.assertEqual(by_kind, EXITS)
        self.assertEqual(len(census), sum(EXITS.values()))
        self.assertEqual(len(census), 12)
        self.assertEqual(len(BAD_ENTRIES), EXITS[("read_python_shard", "Return")])

    def test_a_skipped_entry_before_a_bad_one_is_refused(self):
        totals = {}
        controlled = {}
        for count in SKIP_COUNTS:
            plan = Plan(self, count)
            for family, label, slots, wrong in examined(
                f"skipped-before-bad member(s) of {count} shards", list(skipped_members(plan))
            ):
                code, named, output = plan.lay(slots)
                controlled.setdefault(family, (plan, label, slots, wrong))
                expected = (0, set()) if not wrong else (3, wrong)
                self.assertEqual((code, named), expected, f"{family}: {label}: {output}")
                totals[family] = totals.get(family, 0) + 1
        self.assertEqual(sorted(totals), ["skipped alone", "skipped then bad", "slot skipped"])
        skipped = len(SKIPPED_ENTRIES)
        self.assertEqual(totals["skipped then bad"], skipped * len(BAD_ENTRIES) * sum(SKIP_COUNTS))
        self.assertEqual(totals["skipped alone"], skipped * sum(SKIP_COUNTS))
        self.assertEqual(
            totals["slot skipped"], 2 * sum(j for n in SKIP_COUNTS for j in range(1, n))
        )
        for family, (plan, label, slots, wrong) in controlled.items():
            if wrong:
                plan.control(family, label, slots, wrong)

    def test_the_judge_reads_on_past_an_entry_a_timeout_and_a_survivor(self):
        seen = 0
        for count in SKIP_COUNTS[:1]:
            plan = Plan(self, count)
            for family, label, slots, lines in examined(
                f"member(s) of {count} shards the judge passes over", list(judge_members(plan))
            ):
                code, _, output = plan.lay(slots)
                self.assertEqual(code, 1, f"{family}: {label}: {output}")
                for line in lines:
                    self.assertIn(line, output, f"{family}: {label}")
                seen += 1
        self.assertGreater(seen, 0)


#: The functions of the verdict program that print a message about a shard's report.
MESSAGE_FUNCTIONS = ("python_reports", "read_python_shard", "shard_listing_drift", "judge_python")
#: Interpolating f-strings of those functions that no assertion of this PR's tests reads, each
#: with why. A site in neither this table nor a pin is a message nothing asserts, and is refused.
UNASSERTED_SITES = {
    ("judge_python", "byte reader: {}: {}", 0): "no assertion of the shard-report tests reads it",
    (
        "judge_python",
        "EQUIVALENT {}: {}",
        0,
    ): "read by test_mutation_python_verdict, a pre-existing line",
    ("judge_python", "unviable: {}", 0): "no assertion of the shard-report tests reads it",
}
#: Fields of an asserted message that no member can reach, each with why.
UNASSERTED_FIELDS = {
    ("judge_python", "SURVIVED {}{}", 0, 1): "held_twice() is empty unless two records excuse it",
}


def message_sites():
    """Every interpolating f-string of MESSAGE_FUNCTIONS, from the verdict program's own syntax
    tree: {(function, template, nth): (node, function node)}, a template being its text with each
    field as `{}` and nth its place among the same templates in source order."""
    tree = ast.parse(VERDICT.read_text(encoding="utf-8"))
    found = {}
    for function in tree.body:
        if not isinstance(function, ast.FunctionDef) or function.name not in MESSAGE_FUNCTIONS:
            continue
        strings = [
            node
            for node in ast.walk(function)
            if isinstance(node, ast.JoinedStr)
            and any(isinstance(part, ast.FormattedValue) for part in node.values)
        ]
        strings.sort(key=lambda node: (node.lineno, node.col_offset))
        seen = {}
        for node in strings:
            template = "".join(
                part.value if isinstance(part, ast.Constant) else "{}" for part in node.values
            )
            nth = seen.get(template, 0)
            seen[template] = nth + 1
            found[(function.name, template, nth)] = (node, function)
    return found


def field_positions(node):
    """The index in `node.values` of each of its interpolated fields."""
    return [i for i, part in enumerate(node.values) if isinstance(part, ast.FormattedValue)]


def planted(function, node, kind, positions, value=None):
    """The verdict program's `function` rewritten so `node`, one of its messages, carries one field
    mutant. DROP makes the field at `positions[0]` print nothing; CONST makes it print `value`;
    SWAP exchanges the fields at the two `positions`. Compiled for the call, never run in place."""
    mutated = copy.deepcopy(function)
    for candidate in ast.walk(mutated):
        if (
            isinstance(candidate, ast.JoinedStr)
            and candidate.lineno == node.lineno
            and candidate.col_offset == node.col_offset
        ):
            parts = candidate.values
            first = positions[0]
            if kind == "DROP":
                parts[first] = ast.Constant("")
            elif kind == "CONST":
                parts[first].value = ast.Constant(value)
            else:
                second = positions[1]
                parts[first].value, parts[second].value = parts[second].value, parts[first].value
    ast.fix_missing_locations(mutated)
    return compile(ast.Module([mutated], []), str(VERDICT), "exec")


@contextlib.contextmanager
def mutating(function, node, kind, positions, value=None):
    """The program with one field mutant of one message, for the length of the block."""
    program = verdict_program()
    original = program.__dict__[function.name]
    exec(planted(function, node, kind, positions, value), program.__dict__)
    try:
        yield program
    finally:
        program.__dict__[function.name] = original


def observed_fields(test, pins, sites):
    """What every field of every message site printed in every pin, from the program run with each
    field wrapped: {(site, index): {pin name: [value per call]}}. A mutant of a field can only be
    caught on an input where it differs, so the population is planted from these values."""
    seen = {}
    current = [None]

    def watch(key, value):
        seen.setdefault(key, {}).setdefault(current[0], []).append(value)
        return value

    program = verdict_program()
    originals = {}
    by_function = {}
    for site, (node, function) in sites.items():
        by_function.setdefault(function.name, (function, []))[1].append((site, node))
    program.__dict__["__watch__"] = watch
    try:
        for name, (function, nodes) in by_function.items():
            wrapped = copy.deepcopy(function)
            for site, node in nodes:
                for candidate in ast.walk(wrapped):
                    if (
                        isinstance(candidate, ast.JoinedStr)
                        and candidate.lineno == node.lineno
                        and candidate.col_offset == node.col_offset
                    ):
                        for index, part in enumerate(
                            p for p in candidate.values if isinstance(p, ast.FormattedValue)
                        ):
                            part.value = ast.Call(
                                ast.Name("__watch__", ast.Load()),
                                [ast.Constant((site, index)), part.value],
                                [],
                            )
            ast.fix_missing_locations(wrapped)
            originals[name] = program.__dict__[name]
            exec(compile(ast.Module([wrapped], []), str(VERDICT), "exec"), program.__dict__)
        for name, pin in pins.items():
            current[0] = name
            run_pin(test, pin.plan, pin, program=program)
    finally:
        program.__dict__.update(originals)
    return seen


def field_mutants(sites, by_site, seen):
    """The generated population (SPEC-126 A11): for every field of every asserted message, DROP;
    CONST of a constant of the field's type that no pin prints, and CONST of each other value the
    field printed in some pin (a field that printed one value everywhere has no such mutant, since
    it is the same program there); and SWAP of each pair of same-typed fields of one message.
    Returns [(kind, site, indexes, positions, value)]."""
    members = []
    for site, (node, function) in sorted(sites.items(), key=lambda kv: kv[1][0].lineno):
        if site not in by_site:
            continue
        positions = field_positions(node)
        fields = [i for i in range(len(positions)) if (*site, i) not in UNASSERTED_FIELDS]
        kinds = {}
        for i in fields:
            values = [v for calls in seen[(site, i)].values() for v in calls]
            kinds[i] = int if all(type(v) is int for v in values) else str
            members.append(("DROP", site, (i,), (positions[i],), None))
            fresh = 7 if kinds[i] is int else "k"
            while fresh in values:
                fresh = fresh * 10 + 7 if kinds[i] is int else fresh + "k"
            members.append(("CONST", site, (i,), (positions[i],), fresh))
            distinct = []
            for v in values:
                if v not in distinct:
                    distinct.append(v)
            if len(distinct) > 1:
                for v in distinct:
                    members.append(("CONST", site, (i,), (positions[i],), v))
        for x, i in enumerate(fields):
            for j in fields[x + 1 :]:
                if kinds[i] is kinds[j]:
                    members.append(("SWAP", site, (i, j), (positions[i], positions[j]), None))
    return members


class Pin:
    """What a test reads of one message: the layout it judges, the exit code and slots it expects,
    the whole lines it must find (every variable part of them) and the fragments it must find."""

    def __init__(self, plan, slots, code, named, sites, lines=(), contains=()):
        self.plan = plan
        self.directory = plan.write(slots)
        self.code = code
        self.named = named
        self.sites = sites
        self.lines = tuple(lines)
        self.contains = tuple(contains)


def run_pin(test, plan, pin, cli=False, program=None):
    """Judge a pin's layout and assert what it reads. Returns the output, so a caller that planted a
    mutant can tell an assertion that failed from a program that crashed."""
    if cli:
        done = judged(plan.fixture, "scripts", pin.directory, *plan.extra)
    else:
        done = judge_in_process(plan.fixture, "scripts", pin.directory, program, plan.extra)
    code, named, output = plan.read(done)
    test.assertEqual((code, named), (pin.code, pin.named), output)
    for line in pin.lines:
        test.assertRegex(output, rf"(?m)^{re.escape(line)}$")
    for text in pin.contains:
        test.assertIn(text, output)
    return output


def message_pins(test, plan):
    """The pins of the Python lane's messages over a two-shard plan, by name. Each reads the
    message its member provokes and pins every variable part of it on that same input."""
    program = verdict_program()
    count = len(plan.listing)
    where = "mutation: scripts: VOID mutation-python-shard-"
    reports = "python_reports"
    shard = "read_python_shard"
    judge = "judge_python"
    pins = {}

    def laid(name, change, code, named, sites, lines=(), contains=(), on=plan):
        slots = on.correct()
        said = change(slots) or ()
        pins[name] = Pin(on, slots, code, named, sites, tuple(lines) + tuple(said), contains)

    def refused(name, change, text, site):
        laid(name, change, 3, {0}, [site], [f"{where}0: {text}"] if text else (), ())

    def deleted(slots):
        slots[1] = None

    laid(
        "the correct layout",
        lambda slots: None,
        0,
        set(),
        [
            (reports, "mutation-python-shard-{}", 0),
            (reports, "{}/{}", 0),
            (judge, "survived {}: equivalent {}, unexplained {}", 0),
            (judge, "examined {}: generated {}, rows {}", 0),
        ],
        [
            f"examined {count}",
            f"mutation: scripts: examined {count}: generated {count}, rows 0",
            "mutation: scripts: survived 0: equivalent 0, unexplained 0",
        ],
    )
    laid(
        "a missing report",
        deleted,
        3,
        {1},
        [(reports, "{}: no report", 0)],
        [f"{where}1: no report"],
    )
    laid(
        "an unreadable report",
        lambda slots: slots.update({1: "{not json"}),
        3,
        {1},
        [(reports, "{}: unreadable", 0)],
        [f"{where}1: unreadable"],
    )
    laid(
        "a report of another schema",
        lambda slots: slots[1].update(schema="other"),
        3,
        {1},
        [(reports, "{}: not of the schema {}", 0)],
        [f"{where}1: not of the schema {program.PYTHON_SCHEMA}"],
    )
    laid(
        "a report that records a failed restore",
        lambda slots: slots[1].update(restore_failed="a lock"),
        3,
        {1},
        [(reports, "{}: a restore failed: {}", 0)],
        [f"{where}1: a restore failed: a lock"],
    )
    laid(
        "a report whose shard field is another slot's",
        lambda slots: slots[1].update(shard="0/2"),
        3,
        {1},
        [
            (
                reports,
                "{}: the report's shard field {} is not this slot's {}/{}, so it is not this shard's work",
                0,
            )
        ],
        [
            f"{where}1: the report's shard field '0/2' is not this slot's 1/2, so it is not this shard's work"
        ],
    )
    refused(
        "a files that is not a list",
        lambda slots: slots[0].update(files="x"),
        "its files is a str, not a list",
        (shard, "its files is a {}, not a list", 0),
    )
    pins["a files that is not a list"].sites.append((reports, "{}: {}", 0))
    refused(
        "an entry that is not an object",
        lambda slots: slots[0]["files"].append(5),
        "an entry of its files is a int, not an object",
        (shard, "an entry of its files is a {}, not an object", 0),
    )
    refused(
        "a mutants that is not a list",
        lambda slots: slots[0]["files"].append({"path": SCRIPT, "mutants": "abc"}),
        f"the mutants of {SCRIPT} is a str, not a list",
        (shard, "the mutants of {} is a {}, not a list", 0),
    )
    refused(
        "a mutant filed under a path no class reads",
        lambda slots: slots[0]["files"].append({"path": "README.md", "mutants": [ghost(9900)]}),
        "it files 1 mutant(s) under README.md, which no class reads",
        (shard, "it files {} mutant(s) under {}, which no class reads", 0),
    )
    refused(
        "a byte readers that is not a list",
        lambda slots: slots[0]["files"].append(
            {"path": SCRIPT, "mutants": [], "byte_readers": "x"}
        ),
        f"the byte readers of {SCRIPT} is a str, not a list",
        (shard, "the byte readers of {} is a {}, not a list", 0),
    )
    refused(
        "an outcome off the runner's vocabulary",
        lambda slots: plan.mutants(slots[0])[0].update(outcome="pending"),
        "a mutant of {} has the outcome 'pending', which is none of the runner's {}".format(
            SCRIPT, ", ".join(runner_outcomes())
        ),
        (shard, "a mutant of {} has the outcome {}, which is none of the runner's {}", 0),
    )
    laid(
        "a mutant record that is not an object",
        lambda slots: plan.mutants(slots[0]).append("not an object"),
        3,
        {0},
        [(shard, "a mutant record of {} is a {}, not an object", 0)],
        [f"{where}0: a mutant record of {SCRIPT} is a str, not an object"],
    )

    def drifted(slots):
        report = plan.mutants(slots[0])
        missing = sorted(m["name"] for m in report[:4])
        del report[:4]
        extra = [ghost(9100 + i) for i in range(4)]
        report += extra
        names = sorted(m["name"] for m in extra)
        return [
            f"{where}0: it did not examine the mutants the plan lists for it: "
            f"4 missing ({', '.join(missing[:3])}) and 4 extra ({', '.join(names[:3])})"
        ]

    laid(
        "a report that drifts from its listing",
        drifted,
        3,
        {0},
        [
            (reports, "{}: {}", 1),
            (
                "shard_listing_drift",
                "it did not examine the mutants the plan lists for it: {} missing ({}) and {} extra ({})",
                0,
            ),
        ],
    )

    def drifted_unequal(slots):
        report = plan.mutants(slots[0])
        gone = report.pop(0)
        extra = [ghost(9200 + i) for i in range(2)]
        report += extra
        names = sorted(m["name"] for m in extra)
        return [
            f"{where}0: it did not examine the mutants the plan lists for it: "
            f"1 missing ({gone['name']}) and 2 extra ({', '.join(names)})"
        ]

    laid(
        "a report that drops one listed mutant and adds two",
        drifted_unequal,
        3,
        {0},
        [
            (reports, "{}: {}", 1),
            (
                "shard_listing_drift",
                "it did not examine the mutants the plan lists for it: {} missing ({}) and {} extra ({})",
                0,
            ),
        ],
    )

    def judged_over(slots):
        entry = slots[0]["files"][0]
        first, second, third = entry["mutants"][:3]
        first["outcome"], second["outcome"], third["outcome"] = "timeout", "survived", "uncovered"
        generated = sum(len(plan.mutants(slots[k])) for k in slots) - 1
        slots[0]["files"].insert(
            0,
            {"path": SCRIPT, "modules": [], "byte_readers": [], "mutants": [], "void": "no tests"},
        )
        return [
            f"mutation: scripts: VOID {SCRIPT}: no tests",
            f"mutation: scripts: VOID timeout: {first['name']}",
            f"mutation: scripts: SURVIVED {second['name']}",
            f"mutation: scripts: UNCOVERED {third['name']}: no test of the file's modules reaches it",
            "mutation: scripts: survived 1: equivalent 0, unexplained 1",
            f"mutation: scripts: examined {generated}: generated {generated}, rows 0",
        ]

    laid(
        "a void entry, a timeout, a survivor and an uncovered mutant",
        judged_over,
        1,
        set(),
        [
            (judge, "{}: {}", 0),
            (judge, "{}: {}", 1),
            (judge, "SURVIVED {}{}", 0),
            (judge, "UNCOVERED {}: no test of the file's modules reaches it", 0),
            (judge, "survived {}: equivalent {}, unexplained {}", 0),
            (judge, "examined {}: generated {}, rows {}", 0),
        ],
    )
    carried = Plan(test, 2, carried=True)

    def summarised(slots):
        first, second, third = carried.mutants(slots[0])[:3]
        for mutant in (first, second, third):
            mutant["outcome"] = "survived"
        line = carried.text.splitlines()[first["line"] - 1].strip()
        record = a_record(first["mutant"], line)
        carried.fixture.write(
            "scripts/mutation-equivalent.d/python.json", json.dumps({"records": [record]})
        )
        generated = sum(len(carried.mutants(slots[k])) for k in slots)
        return [
            f"mutation: scripts: SURVIVED {second['name']}",
            f"mutation: scripts: SURVIVED {third['name']}",
            "mutation: scripts: survived 3: equivalent 1, unexplained 2",
            f"mutation: scripts: examined {generated + 1}: generated {generated}, rows 1",
        ]

    laid(
        "an excused survivor, two unexplained ones and a carried row",
        summarised,
        1,
        set(),
        [
            (judge, "survived {}: equivalent {}, unexplained {}", 0),
            (judge, "examined {}: generated {}, rows {}", 0),
            (judge, "SURVIVED {}{}", 0),
        ],
        on=carried,
    )
    empty = EmptyReaderPlan(test)
    laid(
        "a class that applies and examined nothing",
        lambda slots: None,
        3,
        set(),
        [(judge, "the {} class applies and nothing was examined", 0)],
        ["mutation: scripts: VOID the scripts class applies and nothing was examined"],
        on=empty,
    )
    return pins


class EveryVariablePartOfAMessageIsPinned(unittest.TestCase):
    """The class rule (SPEC-126 A11): a test that reads a message pins every variable part of it on
    the same input. The population is generated: every interpolating f-string of the functions
    that print a shard report's messages, from the verdict program's own syntax tree, each field
    of it dropped in turn; a pin must then fail by assertion. A field no pin catches is a MISS."""

    def test_every_interpolating_message_has_a_pin_or_says_why_not(self):
        plan = Plan(self, 2)
        claimed = {site for pin in message_pins(self, plan).values() for site in pin.sites}
        found = set(message_sites())
        self.assertEqual(sorted(found - claimed - set(UNASSERTED_SITES)), [], "no pin reads these")
        self.assertEqual(sorted(claimed - found), [], "pins that name no message")
        self.assertEqual(sorted(set(UNASSERTED_SITES) - found), [], "excuses that name no message")
        for site in claimed & set(UNASSERTED_SITES):
            self.fail(f"{site} is both pinned and excused")

    def test_a_mutated_field_of_an_asserted_message_fails_a_pin(self):
        plan = Plan(self, 2)
        pins = message_pins(self, plan)
        for name, pin in pins.items():
            run_pin(self, pin.plan, pin)
            print(f"  pin green: {name}")
        sites = message_sites()
        by_site = {}
        for pin in pins.values():
            for site in pin.sites:
                by_site.setdefault(site, []).append(pin)
        seen = observed_fields(self, pins, sites)
        members = field_mutants(sites, by_site, seen)
        kinds = {
            kind: sum(1 for m in members if m[0] == kind) for kind in ("DROP", "CONST", "SWAP")
        }
        messages = len({m[1] for m in members})
        examined_members = examined(
            f"field mutant(s) of {messages} asserted message(s): "
            f"DROP {kinds['DROP']}, CONST {kinds['CONST']}, SWAP {kinds['SWAP']}",
            members,
        )
        missed = []
        for kind, site, indexes, positions, value in examined_members:
            node, function = sites[site]
            caught = False
            for pin in by_site[site] + [p for p in pins.values() if p not in by_site[site]]:
                with mutating(function, node, kind, positions, value) as program:
                    try:
                        run_pin(self, pin.plan, pin, program=program)
                    except AssertionError as refusal:
                        caught = "Traceback" not in str(refusal)
                    if caught:
                        break
            if not caught:
                missed.append(
                    f"{kind} {site[0]}:{node.lineno} {site[1]!r} fields {indexes} {value!r}"
                )
        for line in missed:
            print(f"  MISS {line}")
        print(f"MISS {len(missed)}")
        for kind, floor in (("DROP", 47), ("CONST", 47), ("SWAP", 23)):
            self.assertGreaterEqual(kinds[kind], floor, f"fewer {kind} field mutants than planted")
        self.assertEqual(missed, [])


#: The recorded run's twelve kept legs' cost records, committed in the layout they download in.
PRICES_RUN = VERDICT.parent / "tests" / "fixtures" / "mutation-prices-38242806596"
#: The prices verb's last line over those records.
PRICES_EXAMINED = "mutation: prices: examined 1017 record(s) in 12 file(s); torn 0\n"


def table_lines():
    """The lines of `mutation-verdict.py`'s `PYTHON_CONTROL_SECONDS` between its opening line and
    its closing brace, or none while the table is the one-line empty dict."""
    lines = VERDICT.read_text(encoding="utf-8").splitlines()
    opening = "PYTHON_CONTROL_SECONDS: dict[str, int] = {"
    if opening not in lines:
        return []
    start = lines.index(opening) + 1
    return lines[start : lines.index("}", start)]


def recorded_prices(program):
    """`prices --costs` over the recorded run's records, run by the program's own `main`:
    (the exit, the printed text)."""
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = program.main(["prices", "--costs", str(PRICES_RUN)])
    return code, out.getvalue()


def recorded_table(out):
    """The table the prices verb's printed lines spell, each path with its whole seconds."""
    return {
        path: int(price) for path, price in re.findall(r'^    "([^"]+)": (\d+),', out, re.MULTILINE)
    }


class ThePythonShareIsOneCappedValue(unittest.TestCase):
    """SPEC-420 R1 to R5 (A1, A2, A3, A5 and A6): the share is one named value under a cap of 128
    that `shards` refuses past before any deal, every printed line reads it, and the price table
    is the prices verb's output over the committed records of one recorded run. The share and the
    Rust ceiling are patched to literals of each test's own, so no test spells the in-tree share."""

    def shards(self, share):
        """`shards` over thirty mutants of a.py priced at 100 s, with the share patched to
        `share` and the Rust ceiling to 193: (the exit, the printed text, the plan's text after
        it, the step output's text)."""
        program = verdict_program()
        entries = priced("a.py", 30)
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            listing = root / "python-listed.json"
            listing.write_text(
                json.dumps({"schema": program.PYTHON_SCHEMA, "listed": 30, "mutants": entries}),
                encoding="utf-8",
            )
            plan, sink = root / "plan.json", root / "github-output"
            plan.write_text(json.dumps({"classes": {"rust": {"applies": False}}}), encoding="utf-8")
            sink.write_text("", encoding="utf-8")
            out = io.StringIO()
            with (
                mock.patch.object(program, "PYTHON_MAX_SHARDS", share),
                mock.patch.dict(program.LEG_CEILING, {"ci": 193}),
                mock.patch.dict(program.PYTHON_CONTROL_SECONDS, {"a.py": 100}, clear=True),
                mock.patch.dict(os.environ, {"GITHUB_OUTPUT": str(sink)}),
                contextlib.redirect_stdout(out),
            ):
                code = program.main(
                    ["shards", "--plan", str(plan), "--python-listed", str(listing)]
                )
            return (
                code,
                out.getvalue(),
                plan.read_text(encoding="utf-8"),
                sink.read_text(encoding="utf-8"),
            )

    def test_the_share_is_one_value_capped_at_128_in_the_tree(self):
        """A1 (R1, R3): the cap is 128 and the share a multiple of 8 within it, spelled on one
        line of its own; the share's value is read from the program, never written here."""
        program = verdict_program()
        self.assertEqual(program.PYTHON_SHARE_CAP, 128)
        share = program.PYTHON_MAX_SHARDS
        self.assertIs(type(share), int)
        self.assertEqual(share % 8, 0, share)
        self.assertTrue(8 <= share <= program.PYTHON_SHARE_CAP, share)
        spelled = [
            line
            for line in VERDICT.read_text(encoding="utf-8").splitlines()
            if line.startswith("PYTHON_MAX_SHARDS")
        ]
        self.assertEqual(spelled, [f"PYTHON_MAX_SHARDS = {share}"])
        examined("lines spelling the share", spelled)

    def test_a_share_outside_the_cap_refuses_the_plan_whole_by_name(self):
        """A2 (R2): a share of 136, and one of 0, is refused before any deal, the plan untouched
        and no step output written; at the cap itself the same listing is dealt."""
        original = json.dumps({"classes": {"rust": {"applies": False}}})
        refused = (136, 0)
        for share in refused:
            code, out, plan, sink = self.shards(share)
            self.assertEqual(
                out,
                f"mutation: shards: REFUSED: the python share PYTHON_MAX_SHARDS = {share} is "
                "outside 1..128 (PYTHON_SHARE_CAP); a share above the cap is never dealt\n",
                share,
            )
            self.assertEqual(code, 1, share)
            self.assertEqual(plan, original, share)
            self.assertEqual(sink, "", share)
        code, out, plan, _sink = self.shards(128)
        self.assertEqual(code, 0, out)
        self.assertEqual(json.loads(plan)["python"]["count"], 2)
        self.assertTrue(out.splitlines()[0].endswith("python legs 2 of ceiling 128"), out)
        examined("refused shares", refused)

    def test_every_printed_share_reads_the_one_value(self):
        """A3 (R3): at a share of 40 and a Rust ceiling of 193, the python line, the cost line and
        the joint line each read 40, and the joint budget is 40 legs of 1800 s and 193 of 10800 s."""
        code, out, _plan, _sink = self.shards(40)
        self.assertEqual(
            out,
            "mutation: shards: 2 python shard(s) for 30 listed mutant(s); python legs 2 of "
            "ceiling 40\n"
            "mutation: shards: python legs dealt by cost: 2 priced leg(s), the slowest at 1700 "
            "s of its 1800 s bound; 0 unpriced leg(s) of at most 20\n"
            "mutation: shards: 1 shard(s) for 0 listed mutant(s), projected at 0 s serially; "
            "the slowest at 2141 s of its 10800 s bound; legs 1 of ceiling 193\n"
            "mutation: shards: run legs 3 of 233: python 2 of 40, rust 1 of 193; projected 93 "
            "runner-minute(s) of 35940\n",
        )
        self.assertEqual(code, 0)
        examined("printed lines", out.splitlines())

    def test_the_price_table_is_the_prices_verbs_output_over_the_recorded_run(self):
        """A5 (R4): the verb's seven figure lines over the committed records are the table's
        lines byte for byte, its examined line last, and the table holds what they spell."""
        program = verdict_program()
        code, out = recorded_prices(program)
        self.assertEqual(code, 0, out)
        self.assertEqual(out, "".join(line + "\n" for line in table_lines()) + PRICES_EXAMINED)
        self.assertEqual(program.PYTHON_CONTROL_SECONDS, recorded_table(out))
        examined("priced files", program.PYTHON_CONTROL_SECONDS)

    def test_a_file_the_filled_table_does_not_price_keeps_the_count_rule(self):
        """A6 (R5): over the table the recorded run derives, the web sweep's 41 mutants beside
        create-issues.py's 5 are dealt as one priced leg and three unpriced legs of at most 20,
        and at a share of 3 the four legs are refused whole by name."""
        program = verdict_program()
        _code, out = recorded_prices(program)
        table = recorded_table(out)
        self.assertEqual(table["scripts/create-issues.py"], 1)
        self.assertNotIn("scripts/web_mutation_shards.py", table)
        issues = priced("scripts/create-issues.py", 5)
        sweep = priced("scripts/web_mutation_shards.py", 41)
        names = names_of(sweep)
        dealt = program.python_by_cost(issues + sweep, table)
        self.assertEqual(
            dealt,
            {
                "count": 4,
                "shards": [
                    {"shard": 0, "mutants": names_of(issues), "projected_seconds": 7},
                    {"shard": 1, "mutants": names[0::3], "projected_seconds": 1800},
                    {"shard": 2, "mutants": names[1::3], "projected_seconds": 1800},
                    {"shard": 3, "mutants": names[2::3], "projected_seconds": 1800},
                ],
            },
        )
        with mock.patch.object(program, "PYTHON_MAX_SHARDS", 3):
            refused = program.python_by_cost(issues + sweep, table)
        self.assertEqual(
            refused,
            "mutation: shards: REFUSED: 46 python mutant(s) need 1 leg(s) priced at most 1800 s "
            "each and 3 unpriced leg(s) of at most 20, more than the pull request run's 3 python "
            "legs: split the change, since a run is never capped",
        )
        examined("dealt legs", dealt["shards"])

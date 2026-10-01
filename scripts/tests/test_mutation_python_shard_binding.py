"""The Python lane's verdict counts a shard's work only from the report bound to that shard
(SPEC-126 A8 and A9, issue #438).

The class rule: the verdict reads a report only when the report's own `shard` field equals the slot
it sits in (`mutation-python-shard-<k>` holds `k/<count>`), and the mutants it examined equal the
plan's listing for that shard, as a multiset. Any other layout is VOID, naming the shard.

The population is generated from a plan's shards, never listed by hand: for each of three plans
(2, 3 and 4 shards, sized by the runner's own ceiling of listed over 40), every shard's report is
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
import re
import sys
import traceback
import types
import unittest
from concurrent.futures import ThreadPoolExecutor

from _support import examined
from test_mutation_python_verdict import (
    SCRIPT,
    VERDICT,
    changed_fixture,
    file_entry,
    judged,
    listed,
    report_of,
    shard_the_plan,
    write_shard,
)

#: The plans' shard counts, each reached by listing that many forty-mutant shards' worth.
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


def judge_in_process(fixture, klass, reports, program=None):
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
    functions = -(-(40 * (count - 1) + 1) // 3)
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


class Plan:
    """One fixture repository, its plan sized to `count` shards, and each shard's own report."""

    def __init__(self, test, count):
        self.test = test
        self.text = population_text(count)
        self.fixture = changed_fixture(test, head_text=self.text)
        self.listing = listed(SCRIPT, self.text)
        shard_the_plan(self.fixture, self.listing)
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
            return self.read(judged(self.fixture, "scripts", directory))
        return self.read(judge_in_process(self.fixture, "scripts", directory))

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


def program_without_field(function, node, position):
    """Run `body` with the verdict program's `function` rewritten so `node`'s field at `position`
    prints nothing. The rewritten function replaces the program's own for the call and the program
    is restored after, so the same `main` and every other function run as they are."""
    mutated = copy.deepcopy(function)
    for candidate in ast.walk(mutated):
        if (
            isinstance(candidate, ast.JoinedStr)
            and candidate.lineno == node.lineno
            and candidate.col_offset == node.col_offset
        ):
            candidate.values[position] = ast.Constant("")
    ast.fix_missing_locations(mutated)
    return compile(ast.Module([mutated], []), str(VERDICT), "exec")


@contextlib.contextmanager
def dropping(function, node, position):
    """The program with one field of one message dropped, for the length of the block."""
    program = verdict_program()
    original = program.__dict__[function.name]
    exec(program_without_field(function, node, position), program.__dict__)
    try:
        yield program
    finally:
        program.__dict__[function.name] = original


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
        done = judged(plan.fixture, "scripts", pin.directory)
    else:
        done = judge_in_process(plan.fixture, "scripts", pin.directory, program)
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
        (),
        ("a mutant record of", "not an object"),
    )

    def drifted(slots):
        report = plan.mutants(slots[0])
        del report[:4]
        report += [ghost(9100 + i) for i in range(4)]

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

    def judged_over(slots):
        entry = slots[0]["files"][0]
        first, second, third = entry["mutants"][:3]
        first["outcome"], second["outcome"], third["outcome"] = "timeout", "survived", "uncovered"
        slots[0]["files"].insert(
            0,
            {"path": SCRIPT, "modules": [], "byte_readers": [], "mutants": [], "void": "no tests"},
        )
        return [
            f"mutation: scripts: VOID {SCRIPT}: no tests",
            f"mutation: scripts: VOID timeout: {first['name']}",
            f"mutation: scripts: SURVIVED {second['name']}",
            f"mutation: scripts: UNCOVERED {third['name']}: no test of the file's modules reaches it",
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
        ],
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

    def test_a_dropped_field_of_an_asserted_message_fails_a_pin(self):
        plan = Plan(self, 2)
        pins = message_pins(self, plan)
        for name, pin in pins.items():
            run_pin(self, pin.plan, pin)
            print(f"  pin green: {name}")
        sites = message_sites()
        by_site = {}
        for name, pin in pins.items():
            for site in pin.sites:
                by_site.setdefault(site, []).append(pin)
        members = []
        for site, (node, function) in sorted(sites.items(), key=lambda kv: kv[1][0].lineno):
            if site not in by_site:
                continue
            for index, position in enumerate(field_positions(node)):
                if (*site, index) in UNASSERTED_FIELDS:
                    continue
                members.append((site, index, position, node, function))
        examined_fields = examined(
            f"interpolated field(s) of {len({m[0] for m in members})} asserted message(s)", members
        )
        missed = []
        for site, index, position, node, function in examined_fields:
            caught = False
            for pin in by_site[site[:3]]:
                with dropping(function, node, position) as program:
                    try:
                        run_pin(self, pin.plan, pin, program=program)
                    except AssertionError as refusal:
                        caught = "Traceback" not in str(refusal)
                    else:
                        caught = False
                    if caught:
                        break
            if not caught:
                missed.append(f"{site[0]}:{node.lineno} {site[1]!r} field {index}")
        for line in missed:
            print(f"  MISS {line}")
        print(f"MISS {len(missed)}")
        self.assertEqual(missed, [])

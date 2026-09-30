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

import copy
import itertools
import json
import unittest

from _support import examined
from test_mutation_python_verdict import (
    SCRIPT,
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

    def lay(self, slots):
        """Write `slots` (slot -> document, raw text, or None for no report) into a directory of
        its own, judge the `scripts` class over it, and return (exit, the slots the verdict named,
        stdout). A directory is never reused."""
        self.laid += 1
        directory = self.fixture.out / f"layout-{self.laid}"
        for slot, document in slots.items():
            if document is None:
                continue
            if isinstance(document, str):
                write_shard(directory, slot, None, raw=document)
            else:
                write_shard(directory, slot, document)
        done = judged(self.fixture, "scripts", directory)
        named = {k for k in range(self.count) if f"VOID mutation-python-shard-{k}: " in done.stdout}
        return done.returncode, named, done.stdout + done.stderr

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
        for count in COUNTS:
            plan = Plan(self, count)
            code, named, output = plan.lay(plan.correct())
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
                if code == 0:
                    accepted += 1
                    continue
                refused += 1
                self.assertEqual(code, 3, f"{label}: {output}")
                self.assertEqual(named, wrong, f"{label}: {output}")
        print(f"{name}: {total} members, {refused} refused, {accepted} accepted")
        self.assertEqual(accepted, 0, f"{accepted} of {total} members accepted")
        self.assertEqual(refused, total)
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
                code, named, output = plan.lay(plan.correct())
                self.assertEqual((code, named), (3, {1}), f"{label}: {output}")
                self.assertIn(
                    "mutation-python-shard-1: the plan lists no mutants for this shard", output
                )
        finally:
            path.write_text(original, encoding="utf-8")

    def test_a_mutant_record_that_is_not_an_object_is_an_extra_mutant(self):
        plan = Plan(self, 2)
        slots = plan.correct()
        plan.mutants(slots[0]).append("not an object")
        code, named, output = plan.lay(slots)
        self.assertEqual((code, named), (3, {0}), output)
        self.assertIn("a mutant record of", output)
        self.assertIn("not an object", output)


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

    def tally(self, plan, members, what):
        """Judge every member; each lands in exactly one bucket: refused (exit 3 naming exactly its
        wrong slots), crashed (a traceback), accepted (exit 0) or other. A crash is caught and
        counted here, never raised, so the count reads the rule and not the harness."""
        counts = {"refused": 0, "crashed": 0, "accepted": 0, "other": 0}
        firsts = {}
        for family, label, slots, wrong in examined(what, list(members)):
            code, named, output = plan.lay(slots)
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
        for count in READER_COUNTS:
            plan = Plan(self, count)
            code, named, output = plan.lay(plan.correct())
            self.assertEqual((code, named), (0, set()), f"the control, {count} shards: {output}")
            families |= {member[0] for member in reader_members(plan)}
            counts, found = self.tally(plan, reader_members(plan), f"members of {count} shards")
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
            ["entry", "extra entry", "files", "mutants", "outcome", "unread entry"],
        )
        self.assertEqual(totals["refused"], total, totals)

    def test_a_shard_that_lists_no_mutant_reads_its_empty_report_and_refuses_the_rest(self):
        plan = EmptyReaderPlan(self)
        code, named, output = plan.lay(plan.correct())
        self.assertEqual((code, named), (3, set()), output)
        self.assertIn("the scripts class applies and nothing was examined", output)
        counts, firsts = self.tally(plan, reader_members(plan), "empty-listing members")
        total = sum(counts.values())
        print(
            f"examined {total} member(s), {counts['refused']} refused, {counts['crashed']} "
            f"crashed, {counts['accepted']} accepted"
        )
        for (family, bucket), label in sorted(firsts.items()):
            print(f"  {bucket}: {family}: {label}")
        self.assertEqual(counts["refused"], total, counts)


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

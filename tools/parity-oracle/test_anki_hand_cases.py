"""The second oracle's hand cases: the design's target of about 1,500 hand-written cases beside
the seeded random ones, every one synthetic and written, spread over every declared class of
every function (SPEC-498 R2, R6, R9, ADR-516).

A written case is a hand case the registry builds from the corpus's written texts, templates,
clocks and collection sketches with no random draw at all, so it reads the same under any seed.
The tests load the committed registry and corpus and draw their cases; none starts the harness.
"""

import importlib.util
import json
import random
import re
import sys
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
ANKI = HERE / "anki"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


generate = load("anki_generate_hand_cases", ANKI / "generate.py")
corpus = load("anki_corpus_hand_cases", ANKI / "corpus.py")

#: The design's hand-case target: about 1,500 hand cases beside about 10,000 random ones.
TARGET = 1_500
#: Every declared (function, class) pair holds at least this many hand cases in all.
PAIR_FLOOR = 5
#: No pair holds more than this share of the written cases, so they are spread, not stacked.
PAIR_SHARE = 0.02
#: A calendar date or a clock time, written as text: a hand case holds neither.
CLOCK = re.compile(r"\b\d{4}-\d{2}-\d{2}\b|\b\d{1,2}:\d{2}(?::\d{2})?\b")
#: A key that names an id: a case names its rows by their index in the corpus, never by an id.
ID_KEY = re.compile(r"(?:^|_)id$", re.IGNORECASE)
#: A line the census tests plant as the engine's stock text when it should match nothing.
UNRELATED = "A line of the engine's own text"


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


REGISTERED = generate.load_registry(generate.REGISTRY)


def registry_module():
    return generate.load_module(generate.REGISTRY / "spec_498.py")


def written_cases():
    """Every written case the committed registry exports: (golden name, class, input)."""
    written = getattr(registry_module(), "WRITTEN", {})
    return [
        (name, edge, case)
        for name, classes in written.items()
        for edge, cases in classes.items()
        for case in cases
    ]


def census():
    """Each registration's hand cases as the driver draws them, and the count per pair."""
    hand, per_pair = [], Counter()
    for name, (_, reg) in REGISTERED.items():
        for edge in reg["classes"]:
            per_pair[(name, edge)] += 0
        for edge, case in generate.covered(name, reg, corpus.SEED):
            if edge is not None:
                hand.append((name, edge, case))
                per_pair[(name, edge)] += 1
    return hand, per_pair


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


def keys(value):
    if isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from keys(item)
    elif isinstance(value, list):
        for item in value:
            yield from keys(item)


#: A registry module over the committed corpus with one valid registration, `DAY`; the
#: repeated-case test appends the lines that plant its fault.
PLANTED = """
import hashlib
import importlib.util
import sys
from pathlib import Path

_path = Path(CORPUS_FILE)
_spec = importlib.util.spec_from_file_location("anki_corpus_planted_hand", _path)
CORPUS = importlib.util.module_from_spec(_spec)
sys.modules["anki_corpus_planted_hand"] = CORPUS
_spec.loader.exec_module(CORPUS)
CORPUS_SHA256 = hashlib.sha256(_path.read_bytes()).hexdigest()
SEED = CORPUS.SEED
"""


def planted(directory, lines):
    folder = Path(directory) / "registry"
    folder.mkdir()
    text = PLANTED.replace("CORPUS_FILE", repr(str(ANKI / "corpus.py"))) + lines + "\n"
    (folder / "spec_498.py").write_text(text, encoding="utf-8")
    return folder


class Target(unittest.TestCase):
    def test_corpus_holds_the_design_hand_case_target(self):
        """The corpus's hand cases, counted as the driver draws them, reach the design's target
        of about 1,500 (at least 1,500 here), and every declared (function, class) pair holds
        at least one."""
        hand, per_pair = census()
        pairs = examined("(function, class) pairs", per_pair)
        examined("hand cases", hand)
        print(f"hand cases {len(hand)}; per-pair minimum {min(per_pair.values())}")
        empty = sorted(f"{name}: {edge}" for (name, edge), n in per_pair.items() if n == 0)
        self.assertEqual(empty, [], "pairs with no hand case")
        self.assertGreaterEqual(
            len(hand), TARGET, f"the corpus holds {len(hand)} hand cases over {len(pairs)} pairs"
        )

    def test_the_written_cases_alone_reach_the_target_and_are_drawn(self):
        """The written cases alone reach the target, and the driver draws each one under the
        class it is written for."""
        written = written_cases()
        print(f"written cases {len(written)}")
        self.assertGreaterEqual(len(written), TARGET, f"the registry writes {len(written)} cases")
        drawn = Counter()
        for name, (_, reg) in REGISTERED.items():
            for edge, case in generate.covered(name, reg, corpus.SEED):
                if edge is not None:
                    drawn[(name, edge, json.dumps(case, sort_keys=True))] += 1
        for name, edge, case in examined("written cases", written):
            key = (name, edge, json.dumps(case, sort_keys=True))
            self.assertEqual(drawn[key], 1, f"{name}: {edge}")

    def test_every_pair_holds_the_floor_and_no_pair_stacks_the_written_cases(self):
        """The written cases are spread over the pairs, not stacked on one: every declared pair
        holds at least PAIR_FLOOR hand cases in all, and no pair holds more than PAIR_SHARE of
        the written cases."""
        _, per_pair = census()
        written = Counter((name, edge) for name, edge, _ in written_cases())
        total = sum(written.values())
        spread = sorted(Counter(written.values()).items())
        print(
            f"per-pair minimum {min(per_pair.values())}; written per pair (count, pairs) {spread}"
        )
        low = sorted(f"{n}: {e} ({c})" for (n, e), c in per_pair.items() if c < PAIR_FLOOR)
        self.assertEqual(low, [], "pairs under the floor")
        examined("pairs with written cases", written)
        stacked = [pair for pair, n in written.items() if n > PAIR_SHARE * total]
        self.assertEqual(stacked, [], f"pairs holding more than {PAIR_SHARE:.0%} of {total}")

    def test_the_written_cases_draw_nothing(self):
        """Seeded by nothing: the registry builds its written cases afresh while every draw of
        random.Random fails, and they equal the committed ones; and the driver draws each of
        them under two other seeds too."""
        expected = examined("written cases", written_cases())

        def refuse(*args, **kwargs):
            raise AssertionError("a written case drew a random value")

        methods = ["random", "randrange", "randint", "choice", "sample", "shuffle", "getrandbits"]
        with mock.patch.multiple(random.Random, **{m: refuse for m in methods}):
            module = load("anki_registry_written_afresh", generate.REGISTRY / "spec_498.py")
        fresh = [
            (name, edge, case)
            for name, classes in module.WRITTEN.items()
            for edge, cases in classes.items()
            for case in cases
        ]
        self.assertEqual(fresh, expected)
        for seed in (corpus.SEED + 1, corpus.SEED + 2):
            drawn = set()
            for name, (_, reg) in REGISTERED.items():
                for edge, case in generate.covered(name, reg, seed):
                    drawn.add((name, edge, json.dumps(case, sort_keys=True)))
            missing = [
                (n, e) for n, e, c in expected if (n, e, json.dumps(c, sort_keys=True)) not in drawn
            ]
            self.assertEqual(missing, [], f"seed {seed}")


class Clean(unittest.TestCase):
    def test_no_hand_case_holds_an_id_or_a_clock_string(self):
        """A guard: no hand case names a row by an id or holds a calendar date or a clock time
        as text; an instant is a whole number of seconds or milliseconds."""
        hand, _ = census()
        for name, edge, case in examined("hand cases", hand):
            with self.subTest(name=name, edge=edge):
                named = [k for k in keys(case) if ID_KEY.search(k)]
                self.assertEqual(named, [])
                text = json.dumps(case, ensure_ascii=False)
                self.assertIsNone(CLOCK.search(text))

    def test_the_stock_text_census_reads_the_written_cases(self):
        """The stock-text census over the written cases: a line of each function's written
        cases, planted as stock text, is refused at that case, and an unrelated stock line is
        found in none of them."""
        by_name = {}
        for name, edge, case in written_cases():
            by_name.setdefault(name, []).append(case)
        for name, cases in examined("functions with written cases", sorted(by_name.items())):
            golden = {"cases": [{"input": case, "output": None} for case in cases]}
            lines = [
                (index, line.strip())
                for index, case in enumerate(cases)
                for text in strings(case)
                for line in text.splitlines()
                if len(line.strip()) >= generate.MIN_STOCK_LINE
            ]
            if not lines:
                continue
            index, line = lines[len(lines) // 2]
            refusals = generate.census_refusals(name, golden, generate.stock_hashes([line]))
            self.assertIn(f"case {index} holds stock line", " ".join(refusals), name)
            clean = generate.census_refusals(name, golden, generate.stock_hashes([UNRELATED]))
            self.assertEqual(clean, [], name)


class Refusals(unittest.TestCase):
    def test_a_hand_case_repeated_in_its_class_is_refused(self):
        """A hand case whose input its class already holds is refused before anything runs, so
        a counted case is never a copy; the same input under another class, or drawn twice as a
        random case, is not refused."""
        plants = {
            "repeated": ('("rollover", dict(clock)), (None, other)', ["rollover"]),
            "other class": (
                '("negative-offset", dict(clock)), (None, other)',
                ["rollover", "negative-offset"],
            ),
            "random twice": ("(None, other), (None, dict(other))", ["rollover"]),
        }
        for label, (cases, classes) in examined("planted registries", plants.items()):
            with self.subTest(label=label), tempfile.TemporaryDirectory() as scratch:
                folder = planted(
                    scratch,
                    "def day(rng):\n"
                    "    clock = CORPUS.timing(rng, 'rollover')\n"
                    "    other = CORPUS.timing(rng)\n"
                    f'    return [("rollover", clock), {cases}]\n\n\n'
                    'FUNCTIONS = {"anki.timing.day": {"id": "F4", "note": "day", "cases": day, '
                    f'"classes": {classes!r}}}}}',
                )
                if label == "repeated":
                    with self.assertRaisesRegex(generate.Refused, "holds one input twice"):
                        generate.load_registry(folder)
                else:
                    registered = generate.load_registry(folder)
                    self.assertEqual(list(registered), ["anki.timing.day"])


if __name__ == "__main__":
    unittest.main()

"""The second oracle's registry and corpus: every function registered, every declared class
covered, one seed, and nothing read from a file (SPEC-498 R2, R3, R9, ADR-516).

The tests load the committed registry and corpus and draw their cases; none starts the harness.
"""

import ast
import hashlib
import importlib.util
import json
import re
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ANKI = HERE / "anki"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


generate = load("anki_generate_registry", ANKI / "generate.py")
corpus = load("anki_corpus_registry", ANKI / "corpus.py")

#: The functions the design names, each by its id.
IDS = (
    [f"F{n}" for n in range(1, 13)]
    + [f"R-{n}" for n in range(1, 13)]
    + ["G-L5-1", "G-L5-2", "G-L6-1", "G-L6-2", "G-L9-1"]
    + [f"GL-{n}" for n in range(1, 5)]
)
DATE = re.compile(r"\b\d{4}-\d{2}-\d{2}\b|\b\d{2}:\d{2}:\d{2}\b")


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


REGISTERED = generate.load_registry(generate.REGISTRY)


class Registered(unittest.TestCase):
    def test_every_function_is_registered(self):
        """Every function the design lists has exactly one registration: F1 to F12, R-1 to R-12,
        G-L5-1, G-L5-2, G-L6-1, G-L6-2, G-L9-1 and GL-1 to GL-4."""
        ids = [reg["id"] for _, reg in examined("registrations", REGISTERED.values())]
        self.assertEqual(sorted(ids), sorted(IDS))
        self.assertEqual(len(ids), 33)

    def test_every_function_but_the_snapshot_declares_edge_classes(self):
        for name, (_, reg) in examined("registrations", REGISTERED.items()):
            with self.subTest(name=name):
                if reg["id"] == "F12":
                    self.assertEqual(reg["classes"], [])
                else:
                    self.assertTrue(reg["classes"], name)

    def test_edge_classes_covered(self):
        """T-G16: every declared class has a case for every function it is declared for, and
        every class is one the corpus defines."""
        pairs = []
        for name, (_, reg) in REGISTERED.items():
            cases = generate.covered(name, reg, corpus.SEED)
            drawn = {edge for edge, _ in cases}
            for edge in reg["classes"]:
                self.assertIn(edge, drawn, f"{name}: {edge}")
                self.assertIn(edge, corpus.CLASSES, f"{name}: {edge} is not a corpus class")
                pairs.append((name, edge))
        examined("(function, class) pairs", pairs)

    def test_revlog_to_events_is_registered_with_every_review_type(self):
        """G-L9-1: the engine's review log rows and day numbers are registered over collections
        that hold every review type, 0 to 5, and a review at the rollover."""
        name = "anki.history.revlog_events"
        reg = REGISTERED[name][1]
        self.assertEqual(reg["id"], "G-L9-1")
        cases = generate.covered(name, reg, corpus.SEED)
        kinds = {row["type"] for _, case in cases for row in case["collection"]["revlog"]}
        self.assertEqual(kinds, set(range(6)))
        rollover = [case for edge, case in cases if edge == "rollover"]
        self.assertTrue(examined("rollover cases", rollover))
        for case in rollover:
            clock = case["collection"]
            local = clock["now"] + clock["utc_offset_minutes"] * 60
            self.assertEqual(local % 86_400, (clock["rollover_hour"] * 3600 - 1) % 86_400)


class Corpus(unittest.TestCase):
    def test_the_corpus_is_seeded_and_every_class_is_declared(self):
        """The corpus is committed with one integer seed and its classes; every class is
        declared by a registration, and every registration draws only from that seed."""
        self.assertIsInstance(corpus.SEED, int)
        for module, _ in REGISTERED.values():
            self.assertEqual(generate.load_module(module).SEED, corpus.SEED)
        declared = {edge for _, reg in REGISTERED.values() for edge in reg["classes"]}
        unused = sorted(set(examined("corpus classes", corpus.CLASSES)) - declared)
        self.assertEqual(unused, [])

    def test_the_corpus_pin_is_current(self):
        module = generate.load_module(generate.REGISTRY / "spec_498.py")
        digest = hashlib.sha256((ANKI / "corpus.py").read_bytes()).hexdigest()
        self.assertEqual(module.CORPUS_SHA256, digest)

    def test_every_input_is_synthetic_strict_and_free_of_dates(self):
        """Inputs are strict JSON with no calendar date or clock time; a time is a number."""
        count = 0
        for name, (_, reg) in REGISTERED.items():
            for _, case in generate.covered(name, reg, corpus.SEED):
                text = json.dumps(case, allow_nan=False, ensure_ascii=False)
                self.assertIsNone(DATE.search(text), name)
                count += 1
        examined("inputs", range(count))
        self.assertGreaterEqual(count, 10_000)

    def test_no_registry_or_corpus_module_reads_a_file_or_the_network(self):
        banned_calls = {"open", "read_text", "read_bytes", "urlopen", "connect"}
        banned_imports = {"sqlite3", "socket", "urllib", "http", "subprocess"}
        files = examined("modules", [ANKI / "corpus.py", *sorted(generate.REGISTRY.glob("*.py"))])
        for path in files:
            tree = ast.parse(path.read_text(encoding="utf-8"))
            for node in ast.walk(tree):
                if isinstance(node, ast.Call):
                    func = node.func
                    called = getattr(func, "attr", None) or getattr(func, "id", None)
                    self.assertNotIn(called, banned_calls, path.name)
                if isinstance(node, ast.Import | ast.ImportFrom):
                    names = [a.name for a in node.names] + [getattr(node, "module", "") or ""]
                    for imported in names:
                        self.assertNotIn(imported.split(".")[0], banned_imports, path.name)


if __name__ == "__main__":
    unittest.main()

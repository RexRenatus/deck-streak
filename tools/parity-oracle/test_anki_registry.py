"""The second oracle's registry and corpus: every function registered, every declared class
covered, one seed, and nothing read from a file (SPEC-498 R2, R3, R9, ADR-516).

The tests load the committed registry and corpus and draw their cases; none starts the harness.
"""

import ast
import hashlib
import importlib.util
import json
import random
import re
import sys
import tempfile
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

#: A registry module over the committed corpus with one valid registration, `DAY`; each refusal
#: test appends the lines that plant its fault.
PLANTED = """
import hashlib
import importlib.util
import sys
from pathlib import Path

_path = Path(CORPUS_FILE)
_spec = importlib.util.spec_from_file_location("anki_corpus_planted", _path)
CORPUS = importlib.util.module_from_spec(_spec)
sys.modules["anki_corpus_planted"] = CORPUS
_spec.loader.exec_module(CORPUS)
CORPUS_SHA256 = hashlib.sha256(_path.read_bytes()).hexdigest()
SEED = CORPUS.SEED


def day(rng):
    return [("rollover", CORPUS.timing(rng, "rollover")), (None, CORPUS.timing(rng))]


DAY = {"id": "F4", "classes": ["rollover"], "note": "day", "cases": day}
"""


def planted(directory, lines, name="spec_498.py"):
    """A registry folder in `directory` with one more module: PLANTED, then `lines`."""
    folder = Path(directory) / "registry"
    folder.mkdir(exist_ok=True)
    text = PLANTED.replace("CORPUS_FILE", repr(str(ANKI / "corpus.py"))) + lines + "\n"
    (folder / name).write_text(text, encoding="utf-8")
    return folder


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


class RolloverClocks(unittest.TestCase):
    def test_every_rollover_clock_is_one_second_before_the_local_rollover_hour(self):
        """A2 (R2, G-L9-1): every clock the corpus draws at the rollover, in each registration
        that declares a rollover class and in timing draws from 64 other seeds, reads one second
        before the rollover hour at its own UTC offset, and some offsets are not whole days."""
        clocks = []
        for name, (_, reg) in REGISTERED.items():
            if not any(c == "rollover" or c.endswith("-at-rollover") for c in reg["classes"]):
                continue
            for edge, case in generate.covered(name, reg, corpus.SEED):
                if edge == "rollover" or (edge or "").endswith("-at-rollover"):
                    clocks.append(case.get("collection", case))
        clocks += [corpus.timing(random.Random(n), "rollover") for n in range(64)]
        for clock in examined("rollover clocks", clocks):
            local = clock["now"] + clock["utc_offset_minutes"] * 60
            self.assertEqual(local % corpus.DAY, (clock["rollover_hour"] * 3600 - 1) % corpus.DAY)
        moved = [c for c in clocks if c["utc_offset_minutes"] * 120 % corpus.DAY]
        examined("rollover clocks at an offset that is not a multiple of 12 hours", moved)


#: A cloze replacement on a template, `{{cloze:Field}}`, with whitespace allowed inside it.
CLOZE_REPLACEMENT = re.compile(r"\{\{\s*cloze\s*:\s*([^{}:]+?)\s*\}\}")
#: A cloze reference in a field's text, `{{c1::...`, by its number.
CLOZE_REFERENCE = re.compile(r"\{\{c(\d+)::")


def occlusion_notes(value):
    """Every (note type, fields) pair of the occlusion kind that `value` holds: a case's own
    note type and fields, and each note of a collection whose note type is one."""
    found = []
    if isinstance(value, dict):
        notetype = value.get("notetype")
        if isinstance(notetype, dict) and notetype.get("kind") == "occlusion":
            found.append((notetype, value.get("fields")))
        notetypes = value.get("notetypes")
        if isinstance(notetypes, list):
            for note in value.get("notes", []):
                if notetypes[note["notetype"]].get("kind") == "occlusion":
                    found.append((notetypes[note["notetype"]], note["fields"]))
        for inner in value.values():
            found += occlusion_notes(inner)
    elif isinstance(value, list):
        for inner in value:
            found += occlusion_notes(inner)
    return found


def cloze_field(notetype, side):
    """The fields the template's `side` names in a cloze replacement, in order."""
    return CLOZE_REPLACEMENT.findall(notetype["templates"][0][side])


class OcclusionNoteType(unittest.TestCase):
    """The corpus's image occlusion note type renders as a cloze-kind note. Anki's manual: an
    Image Occlusion note "is a special case of cloze deletion" (Editing, Image Occlusion); a
    cloze note type "has a single type which is shared by all cloze deletions on a note", and
    "Anki looks on the front template for one or more cloze replacements, like
    {{cloze:FieldName}}. It then looks in the FieldName field for all cloze references, like
    {{c1::text}}. For each separate number, a card will be generated." (Card Generation, Cloze
    Templates)."""

    PAIRS = None

    @classmethod
    def pairs(cls):
        """(note type, fields, function, class): the corpus's own note type, with no fields,
        then each occlusion note of every registered function's drawn cases."""
        if cls.PAIRS is None:
            pairs = [(corpus.occlusion_type(), None, None, None)]
            for name, (_, reg) in REGISTERED.items():
                for edge, case in generate.covered(name, reg, corpus.SEED):
                    pairs += [(nt, fields, name, edge) for nt, fields in occlusion_notes(case)]
            cls.PAIRS = pairs
        return cls.PAIRS

    def test_the_occlusion_note_type_has_one_template_with_one_cloze_on_its_front(self):
        """Each occlusion note type the corpus gives, alone or in a case, has one template whose
        front carries exactly one `{{cloze:<field>}}` on a field the note type declares, and
        whose back carries the same one."""
        kinds = {}
        for notetype, _, _, _ in self.pairs():
            kinds[repr(notetype)] = notetype
        for notetype in examined("occlusion note types", kinds.values()):
            with self.subTest(name=notetype["name"]):
                self.assertEqual(len(notetype["templates"]), 1)
                front = cloze_field(notetype, "front")
                self.assertEqual(len(front), 1, notetype["templates"][0]["front"])
                self.assertIn(front[0], notetype["fields"])
                self.assertEqual(cloze_field(notetype, "back"), front)

    def test_every_occlusion_case_keeps_its_cloze_references_in_the_cloze_field(self):
        """Every occlusion note a case holds keeps its cloze references in the field the front
        names, and no other; a case of an occlusion class other than `empty` holds c1 there, so
        the card of ordinal 0 exists."""
        held = [pair for pair in self.pairs() if pair[1] is not None]
        asked = []
        for notetype, fields, name, edge in examined("occlusion notes in cases", held):
            front = cloze_field(notetype, "front")
            self.assertEqual(len(front), 1, f"{name} {edge}: no single cloze on the front")
            at = notetype["fields"].index(front[0])
            others = [text for i, text in enumerate(fields) if i != at]
            self.assertEqual([t for t in others if CLOZE_REFERENCE.search(t)], [], name)
            if edge is not None and edge != "empty" and name == "anki.render.occlusion_render":
                asked.append((edge, fields[at]))
        for edge, masks in examined("occlusion class cases asked for card 0", asked):
            self.assertIn("1", CLOZE_REFERENCE.findall(masks), edge)


class Refusals(unittest.TestCase):
    """Each registry fault, planted alone beside the valid `DAY`, is refused before anything
    runs, for its own reason."""

    def refused(self, reason, *modules):
        """load_registry over `modules`, spec_498.py first, refuses for `reason`."""
        with tempfile.TemporaryDirectory() as scratch:
            for index, lines in enumerate(modules):
                folder = planted(scratch, lines, f"spec_{498 + index}.py")
            with self.assertRaisesRegex(generate.Refused, reason):
                generate.load_registry(folder)

    def test_the_planted_registry_loads_when_nothing_is_planted(self):
        """The control: PLANTED with `DAY` alone loads, so each refusal below is its fault's."""
        with tempfile.TemporaryDirectory() as scratch:
            folder = planted(scratch, 'FUNCTIONS = {"anki.timing.day": DAY}')
            registered = generate.load_registry(folder)
        self.assertEqual([reg["id"] for _, reg in registered.values()], ["F4"])

    def test_a_module_with_no_functions_mapping_is_refused(self):
        """A2 (R3): a registry module that exports no FUNCTIONS mapping is refused."""
        for lines in examined("modules", ["", 'FUNCTIONS = [("anki.timing.day", DAY)]']):
            with self.subTest(lines=lines):
                self.refused("exports no FUNCTIONS mapping", lines)

    def test_a_module_with_no_integer_seed_is_refused(self):
        """A2 (R2): a module whose SEED is not an integer, a bool included, is refused."""
        for seed in examined("seeds", ["str(SEED)", "True", "None"]):
            with self.subTest(seed=seed):
                lines = f'SEED = {seed}\nFUNCTIONS = {{"anki.timing.day": DAY}}'
                self.refused("exports no integer SEED", lines)

    def test_a_module_with_no_corpus_is_refused(self):
        """A2 (R2): a module that exports no CORPUS module, so no corpus to pin, is refused."""
        lines = 'CORPUS = None\nFUNCTIONS = {"anki.timing.day": DAY}'
        self.refused("exports no CORPUS module", lines)

    def test_a_golden_registered_twice_is_refused(self):
        """A2 (R3): two modules that register the same golden are refused, naming both."""
        self.refused(
            "registered by both registry/spec_498.py and registry/spec_499.py",
            'FUNCTIONS = {"anki.timing.day": DAY}',
            'FUNCTIONS = {"anki.timing.day": dict(DAY, id="F5")}',
        )

    def test_an_id_taken_twice_is_refused(self):
        """A2 (R3): two goldens that take the same id are refused, naming the first holder."""
        lines = 'FUNCTIONS = {"anki.timing.day": DAY, "anki.timing.today": dict(DAY)}'
        self.refused("takes id F4, which anki.timing.day already holds", lines)

    def test_a_registry_that_registers_nothing_is_refused(self):
        """A2 (R3): a registry folder with no module, or whose modules register nothing, is
        refused."""
        self.refused("registers no golden", "FUNCTIONS = {}")
        with tempfile.TemporaryDirectory() as scratch:
            empty = Path(scratch) / "registry"
            empty.mkdir()
            with self.assertRaisesRegex(generate.Refused, "registers no golden"):
                generate.load_registry(empty)

    def test_a_name_that_is_not_a_golden_name_is_refused(self):
        """A2 (R3): a golden's name is anki, then lowercase words each after a '.' or a '_'."""
        names = ["timing.day", "anki", "anki.Timing", "anki..day", "anki.timing day"]
        for name in examined("names", names):
            with self.subTest(name=name):
                self.refused("is not a golden name", f"FUNCTIONS = {{{name!r}: DAY}}")

    def test_a_registration_with_other_keys_is_refused(self):
        """A2 (R3): a registration is exactly id, classes, note and cases."""
        shapes = ["dict(DAY, extra=1)", '{k: v for k, v in DAY.items() if k != "note"}']
        for shape in examined("registrations", shapes):
            with self.subTest(shape=shape):
                lines = f'FUNCTIONS = {{"anki.timing.day": {shape}}}'
                self.refused("a registration takes cases, classes, id, note", lines)

    def test_a_class_name_outside_the_grammar_is_refused(self):
        """A2 (R9): a declared class is lowercase words joined by '-'."""
        class_lists = ['["Rollover"]', '["roll over"]', '["rollover", 4]']
        for classes in examined("class lists", class_lists):
            with self.subTest(classes=classes):
                lines = f'FUNCTIONS = {{"anki.timing.day": dict(DAY, classes={classes})}}'
                self.refused("declares classes that are not a list of class names", lines)

    def test_a_class_declared_twice_is_refused(self):
        """A2 (R9): a registration that declares one class twice is refused."""
        lines = 'FUNCTIONS = {"anki.timing.day": dict(DAY, classes=["rollover", "rollover"])}'
        self.refused("declares a class twice", lines)

    def test_a_registration_with_no_id_is_refused(self):
        """A2 (R3): every registration carries an id that is not blank."""
        for id_ in examined("ids", ['""', '"  "', "4", "None"]):
            with self.subTest(id_=id_):
                lines = f'FUNCTIONS = {{"anki.timing.day": dict(DAY, id={id_})}}'
                self.refused("has no id", lines)

    def test_a_registration_with_no_note_is_refused(self):
        """A2 (R3): every registration carries a note on what the harness calls, not blank."""
        for note in examined("notes", ['""', '" "', "None"]):
            with self.subTest(note=note):
                lines = f'FUNCTIONS = {{"anki.timing.day": dict(DAY, note={note})}}'
                self.refused("has no note on what the harness calls", lines)

    def test_a_registration_with_no_case_builder_is_refused(self):
        """A2 (R3): a registration's cases is a builder the driver can call."""
        for cases in examined("builders", ['"day"', "None", "[day]"]):
            with self.subTest(cases=cases):
                lines = f'FUNCTIONS = {{"anki.timing.day": dict(DAY, cases={cases})}}'
                self.refused("needs a case builder", lines)

    def test_a_builder_that_draws_from_more_than_its_seed_is_refused(self):
        """A2 (R2): a builder whose second draw from random.Random(SEED) differs from its first
        is refused, and one that draws only from the seed is not."""
        calls = []

        def drifting(rng):
            calls.append(rng.random())
            return [(None, {"call": len(calls)})]

        with self.assertRaisesRegex(generate.Refused, r"more than random\.Random\(SEED\)"):
            generate.draw("anki.timing.day", drifting, corpus.SEED)
        self.assertEqual(len(calls), 2)
        seeded = generate.draw("anki.timing.day", lambda rng: [(None, {"x": rng.random()})], 1)
        self.assertEqual(len(seeded), 1)

    def test_a_builder_that_draws_no_case_is_refused(self):
        """A2 (R9): a builder that draws no case is refused, so no golden is empty."""
        with self.assertRaisesRegex(generate.Refused, "drew no case"):
            generate.draw("anki.timing.day", lambda rng: [], corpus.SEED)

    def test_a_case_class_outside_the_grammar_is_refused(self):
        """A2 (R9): a case's class is None or a class name, never another word."""
        for edge in examined("case classes", ["Rollover", "two words", "-lead", 7]):
            with self.subTest(edge=edge):
                with self.assertRaisesRegex(generate.Refused, "not a class name or None"):
                    generate.draw("anki.timing.day", lambda rng, e=edge: [(e, {"x": 1})], 1)


if __name__ == "__main__":
    unittest.main()

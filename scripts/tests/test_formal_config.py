"""SPEC-295: the repository declares its formal-check settings in `config/formal.json`, holding the
fields the formal checker reads and no other, with each value and type as ADR-295 decided."""

import ast
import copy
import json
import unittest
from pathlib import Path

from _support import REPO, examined

CONFIG = REPO / "config" / "formal.json"

# R1: the declared file, as data. Every check below is generated from this one table.
EXPECTED = {
    "k": 20,
    "budgets": {
        "tla_seconds": 300,
        "lean_seconds": 600,
        "entry_seconds": 300,
        "entries": {},
    },
    "axioms": ["propext", "Classical.choice", "Quot.sound"],
    "owner_signers": "config/owner-allowed-signers",
    "tlc_slot": {"capacity": 1, "wait_seconds": 1800},
}

# The reader's rules per field: (path, kind, required). The kinds are the checker's own.
FIELDS = [
    (("k",), "posint", True),
    (("budgets", "tla_seconds"), "posint", True),
    (("budgets", "lean_seconds"), "posint", True),
    (("budgets", "entry_seconds"), "posint", False),
    (("budgets", "entries"), "posint-map", False),
    (("axioms",), "strings", True),
    (("owner_signers",), "path", True),
    (("tlc_slot", "capacity"), "posint", False),
    (("tlc_slot", "wait_seconds"), "posint", False),
]

# One value of every JSON type. A kind admits the types named here, and a value of any other type is
# a fault planted at every field of that kind and at every object level.
JSON_TYPES = {
    "null": None,
    "boolean": True,
    "integer": 1,
    "number": 1.5,
    "integral number": 20.0,
    "string": "x",
    "array": [],
    "object": {},
}
ADMITS = {
    "object": {"object"},
    "posint": {"integer"},
    "posint-map": {"object"},
    "strings": {"array"},
    "path": {"string"},
}
# A map kind's values are judged as an element kind, so each value type the element kind refuses is
# planted as a map value too.
ELEMENT_KIND = {"posint-map": "posint"}
# The values a kind refuses that are of a type it admits: an integer below one.
OUT_OF_RANGE = {"posint": (0, -1)}
# The arm of the reader that refuses a kind's value.
ARM = {
    "posint": "posint",
    "posint-map": "posint-map",
    "strings": "strings-list",
    "path": "path",
}
BAD_PATHS = ("/abs", "../up", "a/../b", "")


class Refused(Exception):
    """The reader refuses the document, and names the arm that refused it."""

    def __init__(self, arm, message):
        super().__init__(message)
        self.arm = arm


def is_posint(value):
    return isinstance(value, int) and not isinstance(value, bool) and value >= 1


def is_repo_relative(value):
    return (
        isinstance(value, str)
        and value != ""
        and not value.startswith("/")
        and ".." not in value.split("/")
    )


def get(doc, path):
    for step in path:
        if not isinstance(doc, dict) or step not in doc:
            return None, False
        doc = doc[step]
    return doc, True


def read(doc):
    """The test's own reader of the file: refuses an object level that is not an object, an
    unknown field, a missing required one, a value of the wrong kind, a non-positive integer and
    an axiom the declared list lacks. The root is the empty prefix, so it is an object level."""
    objects = {()} | {path[:depth] for path, _, _ in FIELDS for depth in range(1, len(path))}
    known = {path for path, _, _ in FIELDS}
    for prefix in objects:
        node, present = get(doc, prefix)
        if not present:
            continue
        if not isinstance(node, dict):
            raise Refused("wrong-kind-object", f"{'.'.join(prefix)} is not an object")
        for key in node:
            child = prefix + (key,)
            if child not in known and child not in objects:
                raise Refused("unknown-field", f"unknown field {'.'.join(child)}")
    for path, kind, required in FIELDS:
        value, present = get(doc, path)
        name = ".".join(path)
        if not present:
            if required:
                raise Refused("missing-field", f"missing required field {name}")
            continue
        if kind == "posint" and not is_posint(value):
            raise Refused("posint", f"{name} is not a positive integer")
        if kind == "posint-map" and not (
            isinstance(value, dict) and all(is_posint(v) for v in value.values())
        ):
            raise Refused("posint-map", f"{name} is not a map of positive integers")
        if kind == "strings":
            if not (isinstance(value, list) and all(isinstance(v, str) for v in value)):
                raise Refused("strings-list", f"{name} is not a list of strings")
            extra = [v for v in value if v not in EXPECTED["axioms"]]
            if extra:
                raise Refused("axiom-added", f"{name} adds {extra}")
        if kind == "path" and not is_repo_relative(value):
            raise Refused("path", f"{name} is not a repo-relative path")
    return doc


def load():
    if not CONFIG.is_file():
        raise AssertionError("config/formal.json is absent: the formal checker reads none")
    return json.loads(CONFIG.read_text(encoding="utf-8"))


def same(a, b):
    """Equality that does not read True as 1."""
    if type(a) is not type(b):
        return False
    if isinstance(a, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return a == b


def with_value(path, value):
    doc = copy.deepcopy(EXPECTED)
    node = doc
    for step in path[:-1]:
        node = node[step]
    node[path[-1]] = value
    return doc


def without(path):
    doc = copy.deepcopy(EXPECTED)
    node = doc
    for step in path[:-1]:
        node = node[step]
    del node[path[-1]]
    return doc


def element_plants(kind):
    """Every value a map kind's element kind refuses, as (label, value): each JSON type it does not
    admit and each out-of-range value of a type it does. Empty for a kind that is no map."""
    element = ELEMENT_KIND.get(kind)
    if element is None:
        return []
    plants = [
        (type_name, bad)
        for type_name, bad in JSON_TYPES.items()
        if type_name not in ADMITS[element]
    ]
    return plants + [(f"out of range {bad!r}", bad) for bad in OUT_OF_RANGE.get(element, ())]


def map_shapes(bad):
    """A map holding the bad value alone, after a good value and before one, so a reader that
    judges only the first or only the last value is caught."""
    return [
        (f"holds {bad!r} alone", {"planted_entry": bad}),
        (f"holds {bad!r} after a good value", {"good_entry": 20, "planted_entry": bad}),
        (f"holds {bad!r} before a good value", {"planted_entry": bad, "good_entry": 20}),
    ]


def type_plants():
    """The population of the class, from FIELDS, ADMITS and ELEMENT_KIND alone: (kind, field, what,
    document) for every value type each field's kind refuses and, inside a map kind, every value
    its element kind refuses. Nothing is listed by hand, so it grows with the kinds."""
    plants = []
    for path, kind, _ in FIELDS:
        if kind == "object":
            continue
        name = ".".join(path)
        for type_name, bad in JSON_TYPES.items():
            if type_name not in ADMITS[kind]:
                plants.append((kind, name, type_name, with_value(path, bad)))
        for bad in OUT_OF_RANGE.get(kind, ()):
            plants.append((kind, name, f"out of range {bad!r}", with_value(path, bad)))
        for label, bad in element_plants(kind):
            for shape, entries in map_shapes(bad):
                plants.append((kind, name, f"{label} {shape}", with_value(path, entries)))
    return plants


def planted_faults():
    """The faults generated from the table, per kind, so each refusal arm of the reader is
    reached: an extra field at each object level, each required field missing, a value of every
    JSON type its kind does not admit at each field and each object level, each integer field at
    zero and negative, a map holding a bad integer, a list holding a non-string, an added axiom and
    each bad signers path."""
    faults = []
    for prefix in {()} | {p[:d] for p, _, _ in FIELDS for d in range(1, len(p))}:
        doc = copy.deepcopy(EXPECTED)
        node = doc
        for step in prefix:
            node = node[step]
        node["planted_field"] = 1
        label = ".".join(prefix) or "root"
        faults.append((f"extra field under {label}", doc))
        for kind_name, bad in JSON_TYPES.items():
            if kind_name not in ADMITS["object"]:
                faults.append(
                    (
                        f"{label} = {bad!r}",
                        bad if not prefix else with_value(prefix, bad),
                    )
                )
    for path, kind, required in FIELDS:
        name = ".".join(path)
        if required:
            faults.append((f"missing {name}", without(path)))
        for kind_name, bad in JSON_TYPES.items():
            if kind_name not in ADMITS[kind]:
                faults.append((f"{name} = {bad!r}", with_value(path, bad)))
        for bad in OUT_OF_RANGE.get(kind, ()):
            faults.append((f"{name} = {bad!r}", with_value(path, bad)))
        for _, bad in element_plants(kind):
            for label, entries in map_shapes(bad):
                faults.append((f"{name} {label}", with_value(path, entries)))
        if kind == "strings":
            faults.append(
                (
                    f"{name} gains an axiom",
                    with_value(path, EXPECTED["axioms"] + ["sorryAx"]),
                )
            )
            faults.append(
                (
                    f"{name} holds a non-string",
                    with_value(path, EXPECTED["axioms"] + [1]),
                )
            )
        if kind == "path":
            for bad in BAD_PATHS:
                faults.append((f"{name} = {bad!r}", with_value(path, bad)))
    return faults


def reader_arms():
    """Every refusal arm of `read`, found in its own source: each `raise Refused(<arm>, ...)`."""
    source = ast.parse(Path(__file__).read_text(encoding="utf-8"))
    reader = next(
        n for n in ast.walk(source) if isinstance(n, ast.FunctionDef) and n.name == "read"
    )
    return [
        node.exc.args[0].value
        for node in ast.walk(reader)
        if isinstance(node, ast.Raise) and isinstance(node.exc, ast.Call)
    ]


class FormalConfig(unittest.TestCase):
    def test_every_value_type_a_kind_refuses_is_planted_and_refused_by_name(self):
        """A4: for each kind the reader judges, each JSON value type it does not admit, an integral
        float wherever an integer is required included, is planted at every field of the kind and,
        inside a map kind, as each value, and the reader refuses each by the kind's own arm."""
        plants = examined("planted value types", type_plants())
        kinds = {kind for _, kind, _ in FIELDS} - {"object"}
        self.assertEqual({kind for kind, *_ in plants}, kinds)
        for kind in sorted(kinds):
            members = [plant for plant in plants if plant[0] == kind]
            fields = {".".join(path) for path, k, _ in FIELDS if k == kind}
            print(
                f"examined {len(members)} planted values of the kind {kind} at {len(fields)} fields"
            )
            refused = [t for t in JSON_TYPES if t not in ADMITS[kind]]
            values = len(element_plants(kind)) * len(map_shapes(None))
            self.assertEqual(
                len(members),
                len(fields) * (len(refused) + len(OUT_OF_RANGE.get(kind, ())) + values),
            )
            for field in fields:
                seen = {what for _, name, what, _ in members if name == field}
                for type_name in JSON_TYPES:
                    if type_name not in ADMITS[kind]:
                        self.assertIn(type_name, seen, f"{field}: {type_name} is not planted")
            element = ELEMENT_KIND.get(kind)
            for type_name in JSON_TYPES if element else ():
                if type_name not in ADMITS[element]:
                    self.assertTrue(
                        any(what.startswith(f"{type_name} ") for _, _, what, _ in members),
                        f"{kind}: {type_name} is not planted as a value",
                    )
        self.assertEqual(sum(1 for kind, *_ in plants if kind in kinds), len(plants))
        integral = JSON_TYPES["integral number"]
        self.assertTrue(isinstance(integral, float) and integral.is_integer())
        self.assertNotIn("integral number", ADMITS["posint"])
        wrong = []
        for kind, field, what, doc in plants:
            try:
                read(doc)
            except Refused as refusal:
                if refusal.arm != ARM[kind]:
                    wrong.append(f"{field} {what}: refused by {refusal.arm}")
                continue
            wrong.append(f"{field} {what}: admitted")
        self.assertEqual(wrong, [], "a planted value the reader did not refuse by its arm")

    def test_the_committed_file_holds_exactly_the_declared_fields(self):
        """A1: the key set, each value and each type equal R1's, one field per line."""
        doc = load()
        leaves = examined("declared fields", [p for p, _, _ in FIELDS])
        for path in leaves:
            value, present = get(doc, path)
            want, _ = get(EXPECTED, path)
            self.assertTrue(present or path[-1] in ("entry_seconds", "entries"), path)
            self.assertTrue(same(value, want), f"{'.'.join(path)}: {value!r} != {want!r}")
        self.assertTrue(same(doc, EXPECTED), f"{doc!r} != {EXPECTED!r}")
        self.assertEqual(CONFIG.read_text(encoding="utf-8"), json.dumps(EXPECTED, indent=2) + "\n")

    def test_the_signers_path_is_repo_relative(self):
        """A2: `owner_signers` is repo-relative, with no `..` segment and no leading `/`."""
        value = load()["owner_signers"]
        self.assertTrue(is_repo_relative(value), value)
        self.assertFalse(value.startswith("/"))
        self.assertNotIn("..", value.split("/"))
        refused = examined("bad paths", [p for p in BAD_PATHS if not is_repo_relative(p)])
        self.assertEqual(len(refused), 4)

    def test_the_reader_refuses_each_planted_fault(self):
        """A3: each planted fault is refused by the test's own reader; the committed file is not."""
        self.assertIs(read(load()) is not None, True, "presence control: the file is admitted")
        faults = examined("planted faults", planted_faults())
        admitted = []
        reached = set()
        for name, doc in faults:
            try:
                read(doc)
            except Refused as refusal:
                reached.add(refusal.arm)
                continue
            except Exception as error:  # a crash is not a refusal
                admitted.append(f"{name}: crashed with {error!r}")
                continue
            admitted.append(name)
        self.assertEqual(admitted, [], "the reader admitted planted faults")
        self.assertGreaterEqual(len(faults), 20)
        arms = examined("refusal arms of the reader", reader_arms())
        self.assertEqual(set(arms) - reached, set(), "a refusal arm no planted fault reaches")
        self.assertEqual(reached - set(arms), set())


if __name__ == "__main__":
    unittest.main()

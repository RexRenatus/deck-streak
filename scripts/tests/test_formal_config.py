"""SPEC-295: the repository declares its formal-check settings in `config/formal.json`, holding the
fields the formal checker reads and no other, with each value and type as ADR-295 decided."""

import ast
import copy
import itertools
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined, refuse_link_components

# The tree the settings and pin paths are read in; a test that reads another tree moves it.
ROOT = REPO
CONFIG = REPO / "config" / "formal.json"
# R6: the formal checker takes its toolchain from ONE source, either `toolchain.identity` in the
# settings file or a committed pin file at this path, and this repository names the identity.
PIN_FILE = Path("config") / "formal-toolchain.json"

# R1: the declared file, as data. Every check below is generated from this one table.
EXPECTED = {
    "k": 20,
    "budgets": {
        "tla_seconds": 480,
        "lean_seconds": 600,
        "entry_seconds": 300,
        "entries": {
            "tla/FullSyncChoice": 60,
            "tla/HabitXpFollowsItsLog": 300,
            "tla/LandmarkOnce": 420,
            "tla/MintReadsTheFinalBase": 180,
            "tla/RelightOrder": 360,
            "tla/SecondRoute": 480,
            "tla/StagedNoJournal": 120,
            "tla/SyncCredential": 60,
            "tla/SyncSnapshotWindow": 60,
            "tla/WalletFloor": 240,
        },
    },
    "axioms": ["propext", "Classical.choice", "Quot.sound"],
    "owner_signers": "config/owner-allowed-signers",
    "tlc_slot": {"capacity": 4, "wait_seconds": 1800},
    "toolchain": {"identity": "a2518571360483e12161e99b64695e6c3e8845230129ce0ad179b5bddf9a8dc4"},
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
    (("toolchain", "identity"), "hex64", True),
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
    "hex64": {"string"},
    "string": {"string"},
}
# The object levels of the document: the root is the empty prefix.
OBJECTS = {()} | {path[:depth] for path, _, _ in FIELDS for depth in range(1, len(path))}
# A container kind's values are judged as an element kind, a map's values and a list's items alike,
# so each value type the element kind refuses is planted inside the container too.
ELEMENT_KIND = {"posint-map": "posint", "strings": "string"}
# A value each element kind admits, planted beside the refused one.
GOOD = {"posint": EXPECTED["k"], "string": EXPECTED["axioms"][0]}
# The values a kind refuses that are of a type it admits: an integer below one.
OUT_OF_RANGE = {"posint": (0, -1)}
# The arm of the reader that refuses a kind's value.
ARM = {
    "object": "wrong-kind-object",
    "posint": "posint",
    "posint-map": "posint-map",
    "strings": "strings-list",
    "path": "path",
    "hex64": "hex64",
}
BAD_PATHS = ("/abs", "../up", "a/../b", "")
HEX_DIGITS = "0123456789abcdef"
# The string values a digest kind refuses, each derived from the declared digest by one edit, so the
# members follow the declared value: one digit short, one digit long, an uppercase digit, a
# non-hex digit, an empty string and a trailing newline.
BAD_HEX = {
    "63 digits": lambda good: good[:-1],
    "65 digits": lambda good: good + "0",
    "an uppercase digit": lambda good: good.upper(),
    "a non-hex digit": lambda good: "g" + good[1:],
    "an empty string": lambda good: "",
    "a trailing newline": lambda good: good + "\n",
}


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


def is_hex64(value):
    return isinstance(value, str) and len(value) == 64 and all(c in HEX_DIGITS for c in value)


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
        if kind == "hex64" and not is_hex64(value):
            raise Refused("hex64", f"{name} is not 64 lowercase hex digits")
    return doc


# SPEC-404 R1 and R2: the keys the formal checker reads under `tlc_slot`, each with the largest
# value its reader takes: an unsigned 32-bit integer for the capacity, an unsigned 64-bit integer
# for the wait. The file states both keys, so the checker never reads a default compiled into it.
SLOT_KEYS = {
    "capacity": 2**32 - 1,
    "wait_seconds": 2**64 - 1,
}


def slot_reading(doc):
    """The slot setting as the formal checker reads it from the file alone (SPEC-404): both keys
    named, each within its reader's range. This stub keeps every input and refuses nothing, so the
    two tests that hold the file to the checker's reading are red before the rule exists."""
    return tuple((doc.get("tlc_slot") or {}).get(key) for key in SLOT_KEYS)


def toolchain_sources(doc, root):
    """The toolchain sources the tree at `root` names, as the checker reads them: the settings
    field when the document names it, and the pin file when the tree holds one. A link at the
    directory the pin file sits in is refused by assertion: the checker finds no pin file through
    it, and the tree's own reading is never followed through a link."""
    refuse_link_components((root / PIN_FILE).parent, root)
    named = []
    if get(doc, ("toolchain", "identity"))[1]:
        named.append("toolchain.identity")
    if os.path.lexists(root / PIN_FILE):  # a link is a blob to the checker, whatever it names
        named.append(PIN_FILE.as_posix())
    return named


# R6: every shape a tree can commit at the pin path. The checker reads the path's blob, and a link
# is a blob holding the name it points at, so a link is a pin file whatever it names.
PIN_SHAPES = (
    "absent",
    "a file",
    "an empty file",
    "a link to a file",
    "a dangling link",
    "a link to itself",
)


def plant_pin(pin, shape):
    """Install one of PIN_SHAPES at `pin`."""
    if shape == "a file":
        pin.write_text("{}\n", encoding="utf-8")
    elif shape == "an empty file":
        pin.write_bytes(b"")
    elif shape == "a link to a file":
        pin.with_name("pin-target.json").write_text("{}\n", encoding="utf-8")
        pin.symlink_to("pin-target.json")
    elif shape == "a dangling link":
        pin.symlink_to("pin-target.json")
    elif shape == "a link to itself":
        pin.symlink_to(pin.name)
    else:
        assert shape == "absent", shape


def committed_text():
    """The committed file's text as the checker reads it, HEAD's blob at the path: a link at the
    file or at any directory above it is refused by assertion and never followed, and so is an
    absent file."""
    refuse_link_components(CONFIG, ROOT)
    if not CONFIG.is_file():
        raise AssertionError("config/formal.json is absent: the formal checker reads none")
    return CONFIG.read_text(encoding="utf-8")


def load():
    """The committed file as the checker reads it, through `committed_text`; and every way the
    parser refuses the bytes, syntax, encoding, a depth past its recursion limit or an integer
    past its digit limit, is a failure by assertion, never an error."""
    try:
        return json.loads(committed_text())
    except (ValueError, RecursionError) as error:
        raise AssertionError(f"config/formal.json is not JSON: {error!r}"[:300]) from error


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


def element_shapes(kind, bad):
    """A container of the kind holding the bad value alone, after a good value and before one, so
    a reader that judges only the first or only the last value is caught: a map for a kind that
    admits an object, a list for one that admits an array."""
    good = GOOD[ELEMENT_KIND[kind]]
    if "object" in ADMITS[kind]:
        return [
            (f"holds {bad!r} alone", {"planted_entry": bad}),
            (f"holds {bad!r} after a good value", {"good_entry": good, "planted_entry": bad}),
            (f"holds {bad!r} before a good value", {"planted_entry": bad, "good_entry": good}),
        ]
    return [
        (f"holds {bad!r} alone", [bad]),
        (f"holds {bad!r} after a good value", [good, bad]),
        (f"holds {bad!r} before a good value", [bad, good]),
    ]


def planted_at(path, value):
    """The declared document with the value at the path; at the root, the value is the document."""
    return with_value(path, value) if path else value


def kind_fields():
    """Each kind the reader judges, with the places it judges it: the object kind at every object
    level (the root's name is empty) and every other kind at every field of the table."""
    fields = {"object": [(prefix, ".".join(prefix)) for prefix in sorted(OBJECTS)]}
    for path, kind, _ in FIELDS:
        fields.setdefault(kind, []).append((path, ".".join(path)))
    return fields


def type_plants():
    """The population of the class, from FIELDS, ADMITS and ELEMENT_KIND alone: (kind, field, what,
    document) for every value type each kind refuses at each place it judges it and, inside a
    container kind, every value its element kind refuses. Nothing is listed by hand, so it grows
    with the kinds."""
    plants = []
    for kind, fields in kind_fields().items():
        for path, name in fields:
            for type_name, bad in JSON_TYPES.items():
                if type_name not in ADMITS[kind]:
                    plants.append((kind, name, type_name, planted_at(path, bad)))
            for bad in OUT_OF_RANGE.get(kind, ()):
                plants.append((kind, name, f"out of range {bad!r}", planted_at(path, bad)))
            for label, bad in element_plants(kind):
                for shape, entries in element_shapes(kind, bad):
                    plants.append((kind, name, f"{label} {shape}", planted_at(path, entries)))
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
            for label, entries in element_shapes(kind, bad):
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
        if kind == "hex64":
            for label, make in BAD_HEX.items():
                faults.append((f"{name} = {label}", with_value(path, make(get(EXPECTED, path)[0]))))
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
        """A4: for each field kind the reader judges, the object kind included at every object
        level, each JSON value type it does not admit, an integral float wherever an integer is
        required included, is planted at every field of the kind and, inside a container kind (a
        map or a list), as each element, and the reader refuses each by the kind's own arm, the
        refusal naming its field and its kind."""
        plants = examined("planted value types", type_plants())
        kinds = set(kind_fields())
        self.assertEqual(kinds, {kind for _, kind, _ in FIELDS} | {"object"})
        self.assertEqual({kind for kind, *_ in plants}, kinds)
        for kind in sorted(kinds):
            members = [plant for plant in plants if plant[0] == kind]
            fields = {name for _, name in kind_fields()[kind]}
            print(
                f"examined {len(members)} planted values of the kind {kind} at {len(fields)} fields"
            )
            refused = [t for t in JSON_TYPES if t not in ADMITS[kind]]
            values = sum(len(element_shapes(kind, bad)) for _, bad in element_plants(kind))
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
                if refusal.arm != ARM[kind] or not str(refusal).startswith(f"{field} is not "):
                    wrong.append(f"{field} {what}: refused by {refusal.arm}: {refusal}")
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
        self.assertEqual(committed_text(), json.dumps(EXPECTED, indent=2) + "\n")

    def test_the_signers_path_is_repo_relative(self):
        """A2: `owner_signers` is repo-relative, with no `..` segment and no leading `/`."""
        value, present = get(load(), ("owner_signers",))
        self.assertTrue(present, "owner_signers is named")
        self.assertTrue(is_repo_relative(value), value)
        self.assertFalse(value.startswith("/"))
        self.assertNotIn("..", value.split("/"))
        refused = examined("bad paths", [p for p in BAD_PATHS if not is_repo_relative(p)])
        self.assertEqual(len(refused), 4)

    def test_the_reader_admits_each_value_a_kind_admits(self):
        """The admission side of the class: a container kind holding good values only, and the
        document without each optional field and each object level that holds no required field,
        are each admitted, so a reader that refuses what a kind admits is caught."""
        docs = []
        for path, kind, required in FIELDS:
            name = ".".join(path)
            if kind in ELEMENT_KIND:
                good = GOOD[ELEMENT_KIND[kind]]
                entries = (
                    {"good_entry": good, "other_entry": good}
                    if "object" in ADMITS[kind]
                    else [good, good]
                )
                docs.append((f"{name} holds good values", with_value(path, entries)))
            if not required:
                docs.append((f"without {name}", without(path)))
        for prefix in sorted(OBJECTS - {()}):
            if not any(required for path, _, required in FIELDS if path[: len(prefix)] == prefix):
                docs.append((f"without {'.'.join(prefix)}", without(prefix)))
        self.assertGreaterEqual(len(docs), len(ELEMENT_KIND) + 1, "presence: documents are planted")
        refused = []
        for name, doc in examined("admitted documents", docs):
            try:
                read(doc)
            except Refused as refusal:
                refused.append(f"{name}: refused by {refusal.arm}: {refusal}")
        self.assertEqual(refused, [], "the reader refused a document its kinds admit")

    def test_the_reader_refuses_each_planted_fault(self):
        """A3: each planted fault is refused by the test's own reader; the committed file is not."""
        try:
            admitted_doc = read(load())
        except Refused as refusal:
            self.fail(f"presence control: the file is refused: {refusal}")
        self.assertIsNotNone(admitted_doc, "presence control: the file is admitted")
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

    def test_the_checker_reads_the_slot_from_the_file_alone(self):
        """A1 (SPEC-404): the committed file's slot reading is its own two values, read from the
        loaded file; the keys the rule judges are the keys the reader's table lists under
        `tlc_slot`; and a document that omits `tlc_slot`, its capacity or its wait is admitted by
        the test's reader and refused by the slot rule, the refusal naming exactly the omitted
        name."""
        doc = load()
        try:
            read(doc)
        except Refused as refusal:
            self.fail(f"presence control: the file is refused: {refusal}")
        lever = doc["tlc_slot"]
        try:
            reading = slot_reading(doc)
        except Refused as refusal:
            self.fail(f"the slot rule refuses the committed file: {refusal}")
        self.assertEqual(reading, (lever["capacity"], lever["wait_seconds"]))
        self.assertEqual(set(SLOT_KEYS), {"capacity", "wait_seconds"})
        listed = {path[1] for path, _, _ in FIELDS if path[0] == "tlc_slot" and len(path) == 2}
        self.assertEqual(listed, {"capacity", "wait_seconds"})
        plants = [
            ("tlc_slot", without(("tlc_slot",))),
            ("tlc_slot.capacity", without(("tlc_slot", "capacity"))),
            ("tlc_slot.wait_seconds", without(("tlc_slot", "wait_seconds"))),
        ]
        reached = 0
        for name, plant in plants:
            try:
                read(plant)
            except Refused as refusal:
                self.fail(f"presence control: {name} omitted is refused by the reader: {refusal}")
            with self.assertRaises(Refused, msg=f"{name} omitted was read as stated") as caught:
                slot_reading(plant)
            self.assertEqual(caught.exception.arm, "slot-unnamed")
            self.assertEqual(str(caught.exception), f"{name} is not stated")
            reached += 1
        print(f"examined {reached} of {len(plants)} slot plants")
        self.assertEqual(reached, len(plants), "a slot plant did not reach its assertion")

    def test_each_slot_value_is_one_the_checkers_reader_takes(self):
        """A2 (SPEC-404): for each slot key, the document holding its bound is admitted and read
        as that value, and the document holding the first integer past the bound is refused by the
        range arm naming the key. The values are spelled here, never derived from the rule."""
        values = [
            ("capacity", 4294967295, True),
            ("capacity", 4294967296, False),
            ("wait_seconds", 18446744073709551615, True),
            ("wait_seconds", 18446744073709551616, False),
        ]
        reached = 0
        for key, value, admitted in values:
            plant = with_value(("tlc_slot", key), value)
            try:
                read(plant)
            except Refused as refusal:
                self.fail(f"presence control: {key} = {value} is refused by the reader: {refusal}")
            if admitted:
                try:
                    reading = slot_reading(plant)
                except Refused as refusal:
                    self.fail(
                        f"tlc_slot.{key} = {value} is within the range, yet refused: {refusal}"
                    )
                self.assertEqual(dict(zip(("capacity", "wait_seconds"), reading))[key], value)
            else:
                with self.assertRaises(Refused, msg=f"{key} = {value} was read") as caught:
                    slot_reading(plant)
                self.assertEqual(caught.exception.arm, "slot-range")
                self.assertEqual(str(caught.exception), f"tlc_slot.{key} is past its range")
            reached += 1
        print(f"examined {reached} of {len(values)} slot values")
        self.assertEqual(reached, len(values), "a slot value did not reach its assertion")

    def test_the_toolchain_identity_is_named_and_a_malformed_one_is_refused(self):
        """A6: the committed file names the checker's toolchain by one 64-digit lowercase hex
        identity, and the test's own reader admits it and refuses every fault generated from the
        table for a digest field: a `toolchain` that is no object, an extra key beside `identity`,
        an identity missing, of every other JSON type, one digit short, one digit long, with an
        uppercase or a non-hex digit, empty or with a trailing newline, each by the kind's own arm."""
        doc = load()
        value, present = get(doc, ("toolchain", "identity"))
        self.assertTrue(present, "toolchain.identity is named")
        self.assertTrue(is_hex64(value), f"{value!r} is not 64 lowercase hex digits")
        self.assertEqual(list(doc["toolchain"]), ["identity"])
        try:
            admitted_doc = read(doc)
        except Refused as refusal:
            self.fail(f"presence control: the committed file is refused: {refusal}")
        self.assertIsNotNone(admitted_doc, "presence control: the committed file is admitted")
        places = examined(
            "digest fields", [(path, ".".join(path)) for path, kind, _ in FIELDS if kind == "hex64"]
        )
        faults = []
        for path, name in places:
            good = get(EXPECTED, path)[0]
            faults.append((f"missing {name}", "missing-field", without(path)))
            for type_name, bad in JSON_TYPES.items():
                if type_name not in ADMITS["hex64"]:
                    faults.append((f"{name} = {type_name}", "hex64", with_value(path, bad)))
            for label, make in BAD_HEX.items():
                faults.append((f"{name} = {label}", "hex64", with_value(path, make(good))))
            parent = path[:-1]
            for type_name, bad in JSON_TYPES.items():
                if type_name not in ADMITS["object"]:
                    faults.append(
                        (
                            f"{'.'.join(parent)} = {type_name}",
                            "wrong-kind-object",
                            with_value(parent, bad),
                        )
                    )
            extra = copy.deepcopy(EXPECTED)
            get(extra, parent)[0]["planted_field"] = 1
            faults.append((f"extra key beside {name}", "unknown-field", extra))
        faults = examined("planted digest faults", faults)
        wrong = []
        for name, arm, bad_doc in faults:
            try:
                read(bad_doc)
            except Refused as refusal:
                if refusal.arm != arm:
                    wrong.append(f"{name}: refused by {refusal.arm}, not {arm}")
                continue
            wrong.append(f"{name}: admitted")
        self.assertEqual(wrong, [], "a planted digest fault the reader did not refuse by its arm")
        self.assertGreaterEqual(len(faults), len(BAD_HEX) + 1 + len(JSON_TYPES) - 1)

    def test_the_tree_names_one_toolchain_source_the_identity(self):
        """R6: the tree names the checker's toolchain by one source, the identity in the settings
        file, and holds no pin file. The population is the settings field named or not, times
        every shape a tree can commit at the pin path, each built in a scratch root. The checker
        reads the path's blob, so every shape but an absent one is a source, a link whatever it
        names, and only the identity alone is admitted."""
        self.assertEqual(toolchain_sources(load(), REPO), ["toolchain.identity"], "the tree")
        combinations = examined(
            "toolchain source combinations", list(itertools.product((True, False), PIN_SHAPES))
        )
        self.assertEqual(
            set(combinations),
            set(itertools.product((True, False), PIN_SHAPES)),
            "the population is every combination, the field named or not times every pin shape",
        )
        self.assertEqual(len(combinations), 2 * len(PIN_SHAPES))
        sources, want = {}, {}
        for named, shape in combinations:
            doc = EXPECTED if named else without(("toolchain",))
            with tempfile.TemporaryDirectory() as scratch:
                root = Path(scratch)
                (root / "config").mkdir()
                plant_pin(root / "config" / "formal-toolchain.json", shape)
                sources[(named, shape)] = toolchain_sources(doc, root)
            want[(named, shape)] = ["toolchain.identity"] * named + [
                "config/formal-toolchain.json"
            ] * (shape != "absent")
        self.assertEqual(
            sources, want, "the sources each combination names, as the checker reads them (R6)"
        )
        self.assertIn(
            ["toolchain.identity", "config/formal-toolchain.json"], list(sources.values())
        )

    def test_each_pin_shape_is_planted_as_the_kind_it_names(self):
        """R6: every shape the source population plants is the kind its label names, so the
        population holds a link where it says a link, a dangling one where it says dangling, and a
        file where it says a file; a shape planted as another kind would leave the checker's
        reading of the real one unjudged."""
        kinds = {
            "absent": lambda p: not os.path.lexists(p),
            "a file": lambda p: not p.is_symlink() and p.is_file() and p.stat().st_size > 0,
            "an empty file": lambda p: not p.is_symlink() and p.is_file() and p.stat().st_size == 0,
            "a link to a file": lambda p: (
                p.is_symlink() and p.parent.joinpath(os.readlink(p)).is_file()
            ),
            "a dangling link": lambda p: (
                p.is_symlink() and not os.path.lexists(p.parent / os.readlink(p))
            ),
            "a link to itself": lambda p: p.is_symlink() and os.readlink(p) == p.name,
        }
        self.assertEqual(set(kinds), set(PIN_SHAPES), "a pin shape with no kind to hold it to")
        for shape in examined("pin shapes planted as their kind", list(PIN_SHAPES)):
            with tempfile.TemporaryDirectory() as scratch:
                pin = Path(scratch) / "formal-toolchain.json"
                plant_pin(pin, shape)
                self.assertTrue(kinds[shape](pin), f"{shape} is not planted as that kind")

    def test_the_identity_is_named_by_the_field_present_whatever_its_value(self):
        """R6: the identity is a source when the field is present, a value the checker then refuses
        included, and is none when the field is absent, whether the toolchain object is there or not.
        The checker reads the field, not its value and not the object that holds it."""
        identity = ("toolchain", "identity")
        states = examined(
            "identity states",
            [
                ("a value", EXPECTED, ["toolchain.identity"]),
                ("empty text", with_value(identity, ""), ["toolchain.identity"]),
                ("null", with_value(identity, None), ["toolchain.identity"]),
                ("zero", with_value(identity, 0), ["toolchain.identity"]),
                ("an empty toolchain object", with_value(("toolchain",), {}), []),
                ("no toolchain", without(("toolchain",)), []),
            ],
        )
        with tempfile.TemporaryDirectory() as scratch:
            for label, doc, want in states:
                self.assertEqual(toolchain_sources(doc, Path(scratch)), want, label)

    def test_the_loader_refuses_bytes_that_are_not_utf8_wherever_they_sit(self):
        """The test's own loader decodes the file as UTF-8 and refuses a byte that is not, wherever
        it sits, inside a string value, inside a key, after the document or alone, by assertion
        each time. The refusal is this loader's own; the test does not rely on the checker for it."""
        placements = examined(
            "places an invalid byte sits",
            [
                ("inside a string value", b'{"owner_signers": "\xff"}'),
                ("inside a key", b'{"\xff": 1}'),
                (
                    "after the document",
                    (json.dumps(EXPECTED, indent=2) + "\n").encode("utf-8") + b"\xff",
                ),
                ("alone", b"\xff"),
            ],
        )
        with tempfile.TemporaryDirectory() as scratch:
            for index, (label, payload) in enumerate(placements):
                path = Path(scratch) / f"{index}.json"
                path.write_bytes(payload)
                # the loader is called through the module's namespace, not by name, so it is not
                # one of the tests that read the committed file
                with mock.patch.dict(globals(), {"CONFIG": path, "ROOT": Path(scratch)}):
                    with self.assertRaises(AssertionError, msg=label):
                        globals()["load"]()


if __name__ == "__main__":
    unittest.main()

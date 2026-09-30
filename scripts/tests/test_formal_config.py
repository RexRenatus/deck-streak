"""SPEC-295: the repository declares its formal-check settings in `config/formal.json`, holding the
fields the formal checker reads and no other, with each value and type as ADR-295 decided."""

import copy
import json
import unittest

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


class Refused(Exception):
    """The reader refuses the document."""


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
    """The test's own reader of the file: refuses an unknown field, a missing required one, a
    value of the wrong kind, a non-positive integer and an axiom the declared list lacks."""
    if not isinstance(doc, dict):
        raise Refused("not an object")
    objects = {()} | {path[:depth] for path, _, _ in FIELDS for depth in range(1, len(path))}
    known = {path for path, _, _ in FIELDS}
    for prefix in objects:
        node, present = get(doc, prefix)
        if not present:
            continue
        if not isinstance(node, dict):
            raise Refused(f"{'.'.join(prefix)} is not an object")
        for key in node:
            child = prefix + (key,)
            if child not in known and child not in objects:
                raise Refused(f"unknown field {'.'.join(child)}")
    for path, kind, required in FIELDS:
        value, present = get(doc, path)
        name = ".".join(path)
        if not present:
            if required:
                raise Refused(f"missing required field {name}")
            continue
        if kind == "posint" and not is_posint(value):
            raise Refused(f"{name} is not a positive integer")
        if kind == "posint-map" and not (
            isinstance(value, dict) and all(is_posint(v) for v in value.values())
        ):
            raise Refused(f"{name} is not a map of positive integers")
        if kind == "strings":
            if not (isinstance(value, list) and all(isinstance(v, str) for v in value)):
                raise Refused(f"{name} is not a list of strings")
            extra = [v for v in value if v not in EXPECTED["axioms"]]
            if extra:
                raise Refused(f"{name} adds {extra}")
        if kind == "path" and not is_repo_relative(value):
            raise Refused(f"{name} is not a repo-relative path")
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


def planted_faults():
    """Every fault the table implies: an extra field at each object level, each required field
    missing, each integer field at zero, negative and boolean, and an added axiom."""
    faults = []
    for prefix in {()} | {p[:d] for p, _, _ in FIELDS for d in range(1, len(p))}:
        doc = copy.deepcopy(EXPECTED)
        node = doc
        for step in prefix:
            node = node[step]
        node["planted_field"] = 1
        faults.append((f"extra field under {'.'.join(prefix) or 'root'}", doc))
    for path, kind, required in FIELDS:
        name = ".".join(path)
        if required:
            faults.append((f"missing {name}", without(path)))
        if kind == "posint":
            for bad in (0, -1, True):
                faults.append((f"{name} = {bad!r}", with_value(path, bad)))
        if kind == "strings":
            faults.append(
                (f"{name} gains an axiom", with_value(path, EXPECTED["axioms"] + ["sorryAx"]))
            )
        if kind == "path":
            for bad in ("/abs", "../up", "a/../b"):
                faults.append((f"{name} = {bad!r}", with_value(path, bad)))
    return faults


class FormalConfig(unittest.TestCase):
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
        refused = examined(
            "bad paths", [p for p in ("/abs", "../up", "a/../b", "") if not is_repo_relative(p)]
        )
        self.assertEqual(len(refused), 4)

    def test_the_reader_refuses_each_planted_fault(self):
        """A3: each planted fault is refused by the test's own reader; the committed file is not."""
        self.assertIs(read(load()) is not None, True, "presence control: the file is admitted")
        faults = examined("planted faults", planted_faults())
        admitted = []
        for name, doc in faults:
            try:
                read(doc)
            except Refused:
                continue
            admitted.append(name)
        self.assertEqual(admitted, [], "the reader admitted planted faults")
        self.assertGreaterEqual(len(faults), 20)


if __name__ == "__main__":
    unittest.main()

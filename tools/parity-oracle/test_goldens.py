"""Every committed golden is current, synthetic and dated by numbers only (SPEC-029 R3 to R6, R9),
and the README's example of the `{day:N}` token round-trips through the registry's own reader and
writer of it (SPEC-054 R6).

The checks read the committed goldens and registry modules as data, so this file never imports the
predecessor and runs in public CI. Each refusal is first proved on a planted tree in a temporary
directory, then applied to the committed one.
"""

import ast
import hashlib
import importlib.util
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location("parity_generate", HERE / "generate.py")
generate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generate)

ORACLE = Path("tools", "parity-oracle")
GENERATOR = "tools/parity-oracle/generate.py"
REGISTRY = "tools/parity-oracle/registry/"
STUDY_DAY = ROOT / ORACLE / "goldens" / "study_day.json"
README = HERE / "README.md"
#: The registry module whose adapter writes and reads the `{day:N}` token (SPEC-042).
DAY_TOKEN_MODULE = HERE / "registry" / "spec_042.py"
#: The README's section on the token. Each of its examples is a pair of `text` fences: a text in
#: the golden's form, then the same text as the predecessor reads it.
DAY_SECTION = "### A day inside a text"
TEXT_FENCE = re.compile(r"^```text\n(.*?)^```$", re.MULTILINE | re.DOTALL)
NEXT_HEADING = re.compile(r"^#{2,3} ", re.MULTILINE)
DAY_MS = 86_400_000
SCHEMA = "phx.parity-golden.v1"
#: Every key a golden may hold, and every key a case may hold (the data-migration pack's own).
KEYS = {
    "adapter",
    "cases",
    "function",
    "generator",
    "generator_sha256",
    "inputs",
    "kind",
    "note",
    "registry",
    "registry_sha256",
    "schema",
    "seed",
    "source_commit",
}
CASE_KEYS = {"class", "diverges", "input", "note", "output"}
KINDS = ("adapter", "constants", "function")
MODULE = re.compile(r"tools/parity-oracle/registry/spec_\d{3}\.py")
COMMIT = re.compile(r"[0-9a-f]{40}")
#: A calendar date, a clock time or a slashed date: what R3 keeps out of every golden.
CALENDAR = re.compile(
    r"\d{4}-\d{2}-\d{2}|(?<!\d)\d{1,2}:\d{2}(?!\d)|(?<!\d)\d{1,2}/\d{1,2}/\d{2,4}(?!\d)"
)
#: Calls that read a file or list a directory, whatever object they are made on (R6).
READERS = {
    "glob",
    "iterdir",
    "listdir",
    "open",
    "read_bytes",
    "read_text",
    "readlink",
    "rglob",
    "scandir",
    "walk",
}
#: Modules that reach a database or the network (R6).
REACHERS = {"aiosqlite", "http", "httpx", "requests", "socket", "sqlite3", "urllib"}

#: A golden as the generator writes one, minus the digests a planted tree computes for itself.
PLANTED_GOLDEN = {
    "cases": [{"input": {"ease": 3, "maturity": 1.25}, "output": 38}],
    "function": "xp.review_xp",
    "generator": GENERATOR,
    "inputs": "synthetic",
    "kind": "function",
    "schema": "phx.parity-golden.v1",
    "seed": 20,
    "source_commit": "0" * 40,
}

#: A registry module for each way R6 forbids, and the one finding each must draw.
PLANTED_READERS = {
    "open": (
        "def cases(rng):\n    return [(None, {'rows': open('rows.csv').read()})]\n",
        "planted.py:2: calls open",
    ),
    "sqlite3": ("import sqlite3\n", "planted.py:1: imports sqlite3"),
    "socket": (
        "from socket import create_connection\n",
        "planted.py:1: imports socket",
    ),
    "urllib": ("import urllib.request\n", "planted.py:1: imports urllib.request"),
    "http": ("from http import client\n", "planted.py:1: imports http"),
    "Path": (
        "from pathlib import Path\n\nROWS = Path('rows.json').read_text()\n",
        "planted.py:3: calls read_text",
    ),
}


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def goldens(root):
    return examined("golden(s)", sorted((root / ORACLE / "goldens").glob("*.json")))


def registry_modules(root):
    return examined("registry module(s)", sorted((root / ORACLE / "registry").glob("*.py")))


def strict(text):
    """A golden parsed as strict JSON: a NaN, an Infinity or a key written twice is refused."""

    def pairs(items):
        keys = [key for key, _ in items]
        twice = sorted({key for key in keys if keys.count(key) > 1})
        if twice:
            raise ValueError(f"repeats the key(s) {', '.join(twice)}")
        return dict(items)

    def constant(name):
        raise ValueError(f"holds {name}, which strict JSON refuses")

    return json.loads(text, object_pairs_hook=pairs, parse_constant=constant)


def stale(root, name, golden):
    """Why a golden no longer matches the generator or the registry module that built it (R4)."""
    found = []
    if golden.get("generator") != GENERATOR:
        found.append(f"{name}: names the generator {golden.get('generator')!r}, not {GENERATOR}")
    elif golden.get("generator_sha256") != sha256(root / GENERATOR):
        found.append(
            f"{name}: generator_sha256 differs from the committed {GENERATOR}: regenerate it"
        )
    registry = golden.get("registry")
    if not (isinstance(registry, str) and MODULE.fullmatch(registry)):
        found.append(f"{name}: names no registry module, spec_NNN.py ({registry!r})")
    elif not (root / registry).is_file():
        found.append(f"{name}: names {registry}, which is not committed")
    elif golden.get("registry_sha256") != sha256(root / registry):
        found.append(
            f"{name}: registry_sha256 differs from the committed {registry}: regenerate it"
        )
    return found


def provenance(name, golden):
    """Why a golden does not record synthetic inputs, the generator's seed and a commit (R5)."""
    found = []
    if golden.get("inputs") != "synthetic":
        found.append(f"{name}: inputs is {golden.get('inputs')!r}, not 'synthetic'")
    if golden.get("seed") != generate.SEED:
        found.append(f"{name}: seed is {golden.get('seed')!r}, not the generator's {generate.SEED}")
    commit = golden.get("source_commit")
    if not (isinstance(commit, str) and COMMIT.fullmatch(commit)):
        found.append(f"{name}: source_commit {commit!r} is not a 40-hex commit")
    return found


def shape(name, golden, text):
    """Why a golden is not a phx.parity-golden.v1 document as the generator writes one (R5)."""
    found = []
    if golden.get("schema") != SCHEMA:
        found.append(f"{name}: schema is {golden.get('schema')!r}, not {SCHEMA!r}")
    found += [f"{name}: holds an unknown key {key!r}" for key in sorted(set(golden) - KEYS)]
    kind = golden.get("kind")
    if kind not in KINDS:
        found.append(f"{name}: kind is {kind!r}, not one of {', '.join(KINDS)}")
    if not (isinstance(golden.get("function"), str) and golden["function"]):
        found.append(f"{name}: names no function")
    glue = [key for key in ("adapter", "note") if key in golden]
    if kind == "adapter" and not all(
        isinstance(golden.get(key), str) and golden[key] for key in ("adapter", "note")
    ):
        found.append(f"{name}: an adapter golden names its adapter and carries its note")
    if kind != "adapter" and glue:
        found.append(f"{name}: only an adapter golden carries {' and '.join(glue)}")
    cases = golden.get("cases")
    if not (isinstance(cases, list) and cases):
        found.append(f"{name}: holds no case")
        cases = []
    for index, case in enumerate(cases):
        if not (isinstance(case, dict) and "input" in case and "output" in case):
            found.append(f"{name}: case {index} is not an input with its output")
            continue
        found += [
            f"{name}: case {index} holds an unknown key {key!r}"
            for key in sorted(set(case) - CASE_KEYS)
        ]
        if "class" in case and not (isinstance(case["class"], str) and case["class"]):
            found.append(f"{name}: case {index} has an empty class")
    if text != json.dumps(golden, indent=2, sort_keys=True, allow_nan=False) + "\n":
        found.append(f"{name}: is not in the generator's form (sorted keys, two-space indent)")
    return found


def date_strings(name, golden):
    """Every string in a golden that reads as a calendar date or a clock time (R3)."""
    found = []

    def visit(value, where):
        if isinstance(value, str):
            if CALENDAR.search(value):
                found.append(f"{name}: {where} holds {value!r}")
        elif isinstance(value, dict):
            for key, item in value.items():
                here = f"{where}.{key}" if where else key
                if CALENDAR.search(key):
                    found.append(f"{name}: the key {here} reads as a date or a time")
                visit(item, here)
        elif isinstance(value, list):
            for index, item in enumerate(value):
                visit(item, f"{where}[{index}]")

    visit(golden, "")
    return found


def registry_reads(where, source):
    """Why a registry module's source reads a file, a database or the network (R6): a static read
    of its syntax tree, so nothing in the module runs."""
    found = []
    for node in ast.walk(ast.parse(source, filename=where)):
        names = []
        if isinstance(node, ast.Import):
            names = [alias.name for alias in node.names]
        elif isinstance(node, ast.ImportFrom) and node.module:
            names = [node.module]
        elif isinstance(node, ast.Call):
            func = node.func
            called = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", None)
            if called in READERS:
                found.append((node.lineno, f"calls {called}"))
            elif (
                called in ("__import__", "import_module")
                and node.args
                and isinstance(node.args[0], ast.Constant)
                and isinstance(node.args[0].value, str)
            ):
                names = [node.args[0].value]
        found += [
            (node.lineno, f"imports {name}") for name in names if name.split(".")[0] in REACHERS
        ]
    return [f"{where}:{line}: {what}" for line, what in sorted(found)]


class Planted:
    """An oracle tree in a temporary directory, removed when the test ends."""

    def __init__(self, test):
        scratch = tempfile.TemporaryDirectory()
        test.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        (self.root / ORACLE / "registry").mkdir(parents=True)
        (self.root / ORACLE / "goldens").mkdir()
        self.write(GENERATOR, "SEED = 20\n")

    def write(self, relative, text):
        path = self.root / relative
        path.write_text(text, encoding="utf-8")
        return path

    def edit(self, relative):
        path = self.root / relative
        path.write_text(path.read_text(encoding="utf-8") + "# edited after its goldens\n")

    def module(self, name):
        return self.write(f"{REGISTRY}{name}", "FUNCTIONS = {}\n")

    def golden(self, name, module):
        golden = dict(
            PLANTED_GOLDEN,
            generator_sha256=hashlib.sha256((self.root / GENERATOR).read_bytes()).hexdigest(),
            registry=f"{REGISTRY}{module.name}",
            registry_sha256=hashlib.sha256(module.read_bytes()).hexdigest(),
        )
        text = json.dumps(golden, indent=2, sort_keys=True) + "\n"
        self.write(f"tools/parity-oracle/goldens/{name}", text)

    def stale(self):
        found = []
        for path in goldens(self.root):
            found += stale(self.root, path.name, strict(path.read_text(encoding="utf-8")))
        return found


class CommittedGoldensAreCurrent(unittest.TestCase):
    def test_a_golden_whose_generator_digest_differs_is_refused(self):
        tree = Planted(self)
        tree.golden("one.json", tree.module("spec_001.py"))
        self.assertEqual(tree.stale(), [])
        tree.edit(GENERATOR)
        self.assertEqual(
            tree.stale(),
            [f"one.json: generator_sha256 differs from the committed {GENERATOR}: regenerate it"],
        )
        committed = sha256(ROOT / GENERATOR)
        for path in goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(golden["generator_sha256"], committed, path.name)

    def test_a_golden_whose_registry_digest_differs_is_refused_alone(self):
        tree = Planted(self)
        tree.golden("one.json", tree.module("spec_001.py"))
        tree.golden("two.json", tree.module("spec_002.py"))
        self.assertEqual(tree.stale(), [])
        tree.edit(f"{REGISTRY}spec_001.py")
        self.assertEqual(
            tree.stale(),
            [
                f"one.json: registry_sha256 differs from the committed {REGISTRY}spec_001.py: "
                "regenerate it"
            ],
        )
        for path in goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(
                golden["registry_sha256"], sha256(ROOT / golden["registry"]), path.name
            )

    def test_every_committed_golden_records_its_seed_and_synthetic_inputs(self):
        planted = dict(PLANTED_GOLDEN, inputs="production", seed=7)
        self.assertEqual(
            provenance("planted.json", planted),
            [
                "planted.json: inputs is 'production', not 'synthetic'",
                "planted.json: seed is 7, not the generator's 20",
            ],
        )
        for path in goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(golden["inputs"], "synthetic", path.name)
            self.assertEqual(golden["seed"], generate.SEED, path.name)
            self.assertEqual(provenance(path.name, golden), [])

    def test_a_registry_module_that_reads_a_file_is_refused(self):
        for reason, (source, finding) in PLANTED_READERS.items():
            with self.subTest(reason):
                self.assertEqual(registry_reads("planted.py", source), [finding])
        for path in registry_modules(ROOT):
            self.assertEqual(registry_reads(path.name, path.read_text(encoding="utf-8")), [])

    def test_no_committed_golden_holds_a_calendar_date(self):
        planted = {
            "function": "analytics.study_day",
            "generator": "written 01/02/1970",
            "note": "Builds a Clock; cases drawn under CPython 3.12.3.",
            "cases": [
                {
                    "input": {"answered_at": "04:00", "instant_ms": 0},
                    "output": "1970-01-02",
                }
            ],
        }
        self.assertEqual(
            date_strings("planted.json", planted),
            [
                "planted.json: generator holds 'written 01/02/1970'",
                "planted.json: cases[0].input.answered_at holds '04:00'",
                "planted.json: cases[0].output holds '1970-01-02'",
            ],
        )
        for path in goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(date_strings(path.name, golden), [])

    def test_the_study_day_golden_carries_its_boundary_classes(self):
        self.assertTrue(STUDY_DAY.is_file(), "the study-day golden is not committed")
        golden = strict(STUDY_DAY.read_text(encoding="utf-8"))
        self.assertEqual((golden["kind"], golden["function"]), ("adapter", "analytics.study_day"))
        by_class = {}
        for case in golden["cases"]:
            self.assertEqual(
                sorted(case["input"]),
                ["instant_ms", "rollover_hour", "utc_offset_minutes"],
            )
            by_class.setdefault(case.get("class"), []).append(case)
        self.assertEqual(
            sorted(name for name in by_class if name),
            ["negative", "offset", "rollover"],
        )
        # One millisecond before, and exactly at, the rollover for each hour and offset R9 names.
        pairs = {}
        for case in by_class["rollover"]:
            key = (case["input"]["rollover_hour"], case["input"]["utc_offset_minutes"])
            pairs.setdefault(key, []).append(case)
        self.assertEqual({hour for hour, _ in pairs}, {0, 4, 23})
        self.assertEqual({offset for _, offset in pairs}, {-720, 0, 330, 840})
        for key, pair in pairs.items():
            self.assertEqual(len(pair), 2, key)
            before, at = sorted(pair, key=lambda case: case["input"]["instant_ms"])
            self.assertEqual(at["input"]["instant_ms"] - before["input"]["instant_ms"], 1, key)
            # The predecessor's own outputs turn the day between the two instants.
            self.assertEqual(at["output"] - before["output"], 1, key)
        self.assertTrue(any(case["input"]["instant_ms"] < 0 for case in by_class["negative"]))
        # An offset that moves the local day off the UTC day, with no rollover to blur it.
        self.assertTrue(
            any(
                case["input"]["rollover_hour"] == 0
                and case["output"] != case["input"]["instant_ms"] // DAY_MS
                for case in by_class["offset"]
            )
        )

    def test_every_committed_golden_is_current_and_well_formed(self):
        wrong = dict(PLANTED_GOLDEN, schema="phx.parity-golden.v0", cases=[])
        self.assertEqual(
            shape(
                "planted.json",
                wrong,
                json.dumps(wrong, indent=2, sort_keys=True) + "\n",
            ),
            [
                "planted.json: schema is 'phx.parity-golden.v0', not 'phx.parity-golden.v1'",
                "planted.json: holds no case",
            ],
        )
        for path in goldens(ROOT):
            text = path.read_text(encoding="utf-8")
            golden = strict(text)
            self.assertEqual(golden["schema"], "phx.parity-golden.v1", path.name)
            findings = (
                shape(path.name, golden, text)
                + stale(ROOT, path.name, golden)
                + provenance(path.name, golden)
                + date_strings(path.name, golden)
            )
            self.assertEqual(findings, [])

    def test_a_golden_that_is_not_strict_json_is_refused(self):
        with self.assertRaisesRegex(ValueError, "NaN"):
            strict('{"output": NaN}')
        with self.assertRaisesRegex(ValueError, "repeats the key"):
            strict('{"seed": 20, "seed": 21}')


def day_token_examples(readme):
    """Each (golden form, predecessor form) pair of text fences in the README's token section."""
    _, found, rest = readme.partition(f"\n{DAY_SECTION}\n")
    if not found:
        return []
    heading = NEXT_HEADING.search(rest)
    fences = TEXT_FENCE.findall(rest[: heading.start()] if heading else rest)
    return list(zip(fences[::2], fences[1::2], strict=False))


def day_token_module():
    """The registry module that reads and writes the token, loaded by the generator's own loader
    and without writing bytecode."""
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        return generate.load_module(DAY_TOKEN_MODULE)
    finally:
        sys.dont_write_bytecode = writes


class TheDayTokenIsDocumentedLosslessly(unittest.TestCase):
    def test_the_readme_day_token_example_round_trips(self):
        registry = day_token_module()
        readme = README.read_text(encoding="utf-8")
        pairs = examined("README day-token example(s)", day_token_examples(readme))
        for golden_form, predecessor_form in pairs:
            with self.subTest(golden_form):
                self.assertRegex(golden_form, registry.DAY_TOKEN)
                self.assertEqual(registry.expand(golden_form), predecessor_form)
                self.assertEqual(registry.contract(predecessor_form), golden_form)
                # The golden check admits the token's form and refuses the dates it stands for.
                self.assertEqual(date_strings("README", {"text": golden_form}), [])
                self.assertEqual(
                    date_strings("README", {"text": predecessor_form}),
                    [f"README: text holds {predecessor_form!r}"],
                )


if __name__ == "__main__":
    unittest.main()

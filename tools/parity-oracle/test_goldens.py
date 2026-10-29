"""Every committed golden is current, synthetic and dated by numbers only (SPEC-029 R3 to R6, R9),
and the README's example of the `{day:N}` token round-trips through the registry's own reader and
writer of it (SPEC-054 R6). The engine's goldens, written once by the second oracle's sandboxed
run, are held to their own driver, registry and pin (SPEC-498 R12 to R17).

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

anki_spec = importlib.util.spec_from_file_location("anki_generate", HERE / "anki" / "generate.py")
anki_generate = importlib.util.module_from_spec(anki_spec)
anki_spec.loader.exec_module(anki_generate)

ORACLE = Path("tools", "parity-oracle")
GENERATOR = "tools/parity-oracle/generate.py"
#: An engine golden's name starts so (the second oracle's `GOLDEN_NAME`).
ENGINE_PREFIX = "anki."
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
    """The predecessor's goldens: every committed golden but the engine's, which the engine's
    checks below read (SPEC-498)."""
    return examined(
        "golden(s)",
        sorted(
            path
            for path in (root / ORACLE / "goldens").glob("*.json")
            if not path.name.startswith(ENGINE_PREFIX)
        ),
    )


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


# The engine's goldens (SPEC-498 R12 to R17): written once by the second oracle's sandboxed run of
# the engine the root manifest pins, read here as data. Nothing below starts the engine.

ANKI = HERE / "anki"
ANKI_GENERATOR = anki_generate.GENERATOR
ANKI_REGISTRY = ANKI / "registry"
#: The stock-text census the run writes: the sha256 of each stripped stock line (SPEC-498 R6).
CENSUS = ANKI / anki_generate.CENSUS_FILE
#: The schema snapshot the engine's fresh-collection golden gives, and its one pin line.
SNAPSHOT = ANKI / "schema-snapshot.sql"
SNAPSHOT_WRITER = ANKI / "snapshot.py"
SNAPSHOT_GOLDEN = "anki.schema.snapshot"
PIN_LINE = re.compile(r"^PRAGMA user_version = (-?\d+);$", re.MULTILINE)
#: The root manifest's patch of the engine: the fork commit every engine golden records.
PATCH_SECTION = '[patch."https://github.com/ankitects/anki.git"]'
PATCH_REV = re.compile(r'^anki = \{[^}\n]*\brev = "([0-9a-f]{40})"', re.MULTILINE)
#: An instant of the run, in seconds or milliseconds, or an id the engine mints from one. The
#: corpus's clocks sit near 1,000,000,000 seconds, far below both ranges.
RUN_SECONDS = range(1_700_000_000, 2_100_000_000)
RUN_MILLIS = range(1_700_000_000_000, 2_100_000_000_000)
DIGITS = re.compile(r"(?<!\d)\d{10}(?:\d{3})?(?!\d)")
HEX_TEXT = re.compile(r"[0-9a-f]+")
#: Keys whose number is a digest of the case's own input, never a clock.
DIGESTS = {"csum"}
#: The columns of each `notes` row in the reader fixture's `tables` answer, in order: the schema's
#: `notes` columns but `mod`. The rows are lists, so their field checksum has no `csum` key to skip.
READER_NOTES = ("id", "guid", "mid", "usn", "tags", "flds", "sfld", "csum", "flags", "data")
#: The one column skipped by position (SPEC-498 R15: a field checksum, a digest of the input's own
#: text, is not a clock): `csum`, in a `notes` row of that width under the output's `tables`. No
#: other table, column or row width is skipped.
NOTES_CSUM = READER_NOTES.index("csum")
#: The engine's name for a LaTeX image: `latex-`, the sha1 of the LaTeX's own text in 40 hex
#: characters, and `.png` or `.svg` (SPEC-498 R15: a digest of the input's own text, not a clock).
#: A decimal run is skipped only where it lies wholly inside those 40 hex characters of a match;
#: every other run in the same string is still read. No other name form is skipped.
LATEX_NAME = re.compile(r"latex-([0-9a-f]{40})\.(?:png|svg)")
#: A test of a consumer crate, and a Rust comment, which names nothing.
RUST_TEST = re.compile(r"#\s*\[\s*(?:[\w:]+::)?test\b")
RUST_COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.DOTALL)


def engine_goldens(root):
    return examined(
        "engine golden(s)",
        sorted((root / ORACLE / "goldens").glob(f"{ENGINE_PREFIX}*.json")),
    )


def engine_registered():
    """The engine registry's golden names, loaded as the driver loads them, without drawing a
    case and without writing bytecode."""
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        names = []
        for path in sorted(ANKI_REGISTRY.glob("*.py")):
            names += list(anki_generate.load_module(path).FUNCTIONS)
        return examined("registered engine function(s)", names)
    finally:
        sys.dont_write_bytecode = writes


def pinned_engine(manifest):
    """The engine commit the root manifest's patch pins, or None."""
    _, found, rest = manifest.partition(f"\n{PATCH_SECTION}\n")
    match = PATCH_REV.search(rest.split("\n[", 1)[0]) if found else None
    return match.group(1) if match else None


def engine_provenance(root, name, golden, commit):
    """Why an engine golden is not the pinned engine's, through the committed driver and
    registry module, over synthetic seeded inputs (dm.oracle-provenance, dm.oracle-synthetic)."""
    found = list(anki_generate.strict_refusals(name, golden))
    if golden.get("source_commit") != commit:
        found.append(
            f"{name}: source_commit {golden.get('source_commit')!r} is not the manifest's "
            f"engine pin {commit!r}"
        )
    found += anki_generate.provenance_refusals(name, golden, root)
    registry = golden.get("registry")
    if isinstance(registry, str) and registry.startswith(anki_generate.PUBLISHED_REGISTRY):
        module = root / registry
        if module.is_file():
            seed = read_seed(module)
            if golden.get("seed") != seed:
                found.append(f"{name}: seed {golden.get('seed')!r} is not the corpus's {seed}")
    else:
        found.append(f"{name}: registry {registry!r} is not an engine registry module")
    return found


def read_seed(module):
    """The registry module's SEED, read without writing bytecode."""
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        return anki_generate.load_module(module).SEED
    finally:
        sys.dont_write_bytecode = writes


def numbers(value, path=()):
    """Every integer in a value, and every 10- or 13-digit run inside its strings, but a field
    checksum's (`csum`, a digest of the input's own text, by key or as a reader fixture `notes`
    row's column), a hex string's (bytes, not a count) and a run inside the digest of a LaTeX
    image's name. `path` is the value's place in the case's output or input, as keys and list
    positions."""
    if isinstance(value, bool):
        return
    if isinstance(value, int):
        yield value
    elif isinstance(value, str):
        if not HEX_TEXT.fullmatch(value):
            digests = [name.span(1) for name in LATEX_NAME.finditer(value)]
            for run in DIGITS.finditer(value):
                if not any(start <= run.start() and run.end() <= end for start, end in digests):
                    yield int(run.group())
    elif isinstance(value, dict):
        for key, item in value.items():
            if key in DIGESTS:
                continue
            yield from numbers(key)
            yield from numbers(item, (*path, key))
    elif isinstance(value, list):
        notes_row = is_notes_row(path, value)
        for index, item in enumerate(value):
            if notes_row and index == NOTES_CSUM:
                continue
            yield from numbers(item, (*path, index))


def is_notes_row(path, value):
    """Whether a list is a row of the `notes` table in the output's `tables`, of the reader
    fixture's width: the only place a field checksum is skipped by position."""
    return (
        len(path) == 3
        and path[:2] == ("tables", "notes")
        and type(path[2]) is int
        and len(value) == len(READER_NOTES)
    )


def run_marks(name, golden):
    """Each output number that reads as an instant of the run or an id minted from one, unless
    the case's input already holds it (SPEC-498 R15: no ids or clocks in any golden)."""
    found = []
    for index, case in enumerate(golden.get("cases", [])):
        given = set(numbers(case.get("input")))
        for number in numbers(case.get("output")):
            if (number in RUN_SECONDS or number in RUN_MILLIS) and number not in given:
                found.append(f"{name}: case {index} answers {number}, an id or clock of the run")
    return found


def census_hashes(text):
    """The census file's digests; a line that is not one sha256 is refused."""
    hashes = set()
    for number, line in enumerate(text.splitlines(), 1):
        if not anki_generate.HEX64.fullmatch(line):
            raise ValueError(f"{CENSUS.name}:{number} is not one sha256")
        hashes.add(line)
    return hashes


def rust_consumers(root):
    """Each Rust test file under `crates/*/tests`, as its code without comments, when it holds
    a test."""
    found = {}
    for path in sorted((root / "crates").glob("*/tests/**/*.rs")):
        code = RUST_COMMENT.sub("", path.read_text(encoding="utf-8"))
        if RUST_TEST.search(code):
            found[path.relative_to(root).as_posix()] = code
    return found


def unconsumed(names, consumers):
    """Each engine golden no Rust test names in its code (dm.oracle-consumed)."""
    return [
        f"{name}: read by no Rust test"
        for name in names
        if not any(f'"{name}"' in code for code in consumers.values())
    ]


def snapshot_writer():
    """The snapshot writer, loaded by path; a missing writer fails the test that needs it."""
    if not SNAPSHOT_WRITER.is_file():
        raise AssertionError(f"{SNAPSHOT_WRITER.relative_to(ROOT)} is not committed")
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        return anki_generate.load_module(SNAPSHOT_WRITER)
    finally:
        sys.dont_write_bytecode = writes


#: An engine golden as the driver writes one, minus the digests a test computes for itself.
PLANTED_ENGINE = {
    "cases": [
        {
            "class": "rollover",
            "input": {"crt": 1_000_000_000, "now": 1_000_172_805},
            "output": {"day": 2, "next_rollover": 1_000_188_000},
        }
    ],
    "function": "anki.timing.day",
    "generator": "tools/parity-oracle/anki/generate.py",
    "inputs": "synthetic",
    "kind": "engine",
    "note": "answers the day (F4); cases drawn under CPython 3.12.3.",
    "registry": "tools/parity-oracle/anki/registry/spec_498.py",
    "schema": SCHEMA,
    "seed": 498,
    "source_commit": "a078820fd4fec3068ed7863344cd50effddabc6b",
}


def planted_engine(**changes):
    golden = dict(
        PLANTED_ENGINE,
        generator_sha256=sha256(ROOT / ANKI_GENERATOR),
        registry_sha256=sha256(ANKI_REGISTRY / "spec_498.py"),
    )
    golden.update(changes)
    return golden


class EngineGoldensAreCurrent(unittest.TestCase):
    """SPEC-498 R12 to R17: the engine's goldens, each held against a planted golden first."""

    def setUp(self):
        self.commit = pinned_engine((ROOT / "Cargo.toml").read_text(encoding="utf-8"))

    def test_the_manifest_pins_one_engine_commit(self):
        planted = f'x = 1\n{PATCH_SECTION}\nanki = {{ git = "u", rev = "{"a" * 40}" }}\n[lints]\n'
        self.assertEqual(pinned_engine(planted), "a" * 40)
        self.assertIsNone(pinned_engine(planted.replace("rev =", "branch =")))
        self.assertEqual(self.commit, anki_generate.ENGINE_COMMIT)

    def test_every_registered_engine_function_has_its_golden(self):
        """R12 (L4:768 item 4): one committed golden per registered function, and no other."""
        registered = engine_registered()
        committed = [path.stem for path in engine_goldens(ROOT)]
        self.assertEqual(sorted(committed), sorted(registered))

    def test_engine_golden_provenance(self):
        """dm.oracle-provenance (R13, L8:805): the pinned engine, the committed driver and
        registry module, strict phx.parity-golden.v1."""
        self.assertEqual(
            engine_provenance(ROOT, PLANTED_ENGINE["function"], planted_engine(), self.commit), []
        )
        for changes, finding in (
            ({"source_commit": "a" * 40}, "is not the manifest's engine pin"),
            ({"generator_sha256": "0" * 64}, "generator_sha256 differs"),
            ({"registry_sha256": "0" * 64}, "registry_sha256 differs"),
            ({"kind": "function"}, "kind is 'function', not 'engine'"),
            ({"stray": 1}, "differ from the schema's"),
        ):
            with self.subTest(finding):
                found = engine_provenance(
                    ROOT, PLANTED_ENGINE["function"], planted_engine(**changes), self.commit
                )
                self.assertTrue(any(finding in line for line in found), found)
        for path in engine_goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            with self.subTest(path.name):
                self.assertEqual(engine_provenance(ROOT, path.stem, golden, self.commit), [])

    def test_engine_goldens_are_synthetic_and_seeded(self):
        """dm.oracle-synthetic (R13): synthetic inputs and the corpus's integer seed."""
        for changes, finding in (
            ({"inputs": "production"}, "inputs is 'production', not 'synthetic'"),
            ({"seed": 7}, "seed 7 is not the corpus's 498"),
            ({"seed": "498"}, "seed is not an integer"),
        ):
            with self.subTest(finding):
                found = engine_provenance(
                    ROOT, PLANTED_ENGINE["function"], planted_engine(**changes), self.commit
                )
                self.assertTrue(any(finding in line for line in found), found)
        for path in engine_goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(golden["inputs"], "synthetic", path.name)
            self.assertEqual(golden["seed"], read_seed(ROOT / golden["registry"]), path.name)

    def test_no_engine_golden_holds_a_date_an_id_or_a_clock(self):
        """R15 (L4:768 item 9, L8:811): no date string, and no number of the run's clock or an
        id minted from it, unless the input gave it."""
        minted = planted_engine(
            cases=[{"input": {"crt": 1_000_000_000}, "output": {"id": 1_793_000_000_123}}]
        )
        given = planted_engine(
            cases=[{"input": {"at": 1_793_000_000_123}, "output": {"id": 1_793_000_000_123}}]
        )
        named = planted_engine(cases=[{"input": {}, "output": {"name": "ds_1793000000.png"}}])
        dated = planted_engine(cases=[{"input": {}, "output": {"due": "2026-10-29"}}])
        self.assertEqual(
            run_marks("planted", minted),
            ["planted: case 0 answers 1793000000123, an id or clock of the run"],
        )
        self.assertEqual(run_marks("planted", given), [])
        self.assertEqual(len(run_marks("planted", named)), 1)
        digest = planted_engine(cases=[{"input": {}, "output": {"csum": 1_793_000_000}}])
        self.assertEqual(run_marks("planted", digest), [])

        def fixture(table, column):
            """A reader fixture output of one row of `table` with a clock at `column`."""
            row = [0] * len(READER_NOTES)
            row[column] = 1_793_000_000
            return planted_engine(cases=[{"input": {}, "output": {"tables": {table: [row]}}}])

        clock = ["planted: case 0 answers 1793000000, an id or clock of the run"]
        self.assertEqual(run_marks("planted", fixture("notes", 0)), clock)
        self.assertEqual(run_marks("planted", fixture("revlog", NOTES_CSUM)), clock)
        self.assertEqual(run_marks("planted", fixture("notes", NOTES_CSUM)), [])
        held = f"{'a' * 15}1793000000{'b' * 15}"
        for label, output, marks in (
            ("a LaTeX image name", {"name": f"latex-{held}.png"}, []),
            ("another image name", {"name": f"img-{held}.png"}, clock),
            ("a LaTeX name of 39 hex", {"name": f"latex-{held[1:]}.png"}, clock),
            ("a run beside the name", {"text": f'<img src="latex-{held}.png"> 1793000000 '}, clock),
            ("an svg LaTeX image name", {"name": f"latex-{held}.svg"}, []),
        ):
            with self.subTest(label):
                plant = planted_engine(cases=[{"input": {}, "output": output}])
                self.assertEqual(run_marks("planted", plant), marks)
        self.assertEqual(
            date_strings("planted", dated), ["planted: cases[0].output.due holds '2026-10-29'"]
        )
        self.assertEqual(run_marks("planted", planted_engine()), [])
        for path in engine_goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(run_marks(path.stem, golden) + date_strings(path.stem, golden), [])

    def test_no_engine_golden_holds_a_stock_line(self):
        """R15 (L4:768 item 9): the committed census holds digests only, and no golden holds a
        line of the engine's own note types."""
        line = "a line of the engine's own text"
        hashes = anki_generate.stock_hashes([line])
        stocked = planted_engine(cases=[{"input": {}, "output": {"text": f"x\n  {line}  \n"}}])
        self.assertEqual(len(anki_generate.census_refusals("planted", stocked, hashes)), 1)
        self.assertEqual(anki_generate.census_refusals("planted", planted_engine(), hashes), [])
        with self.assertRaisesRegex(ValueError, "is not one sha256"):
            census_hashes(f"{'0' * 64}\n{line}\n")
        self.assertTrue(CENSUS.is_file(), f"{CENSUS.relative_to(ROOT)} is not committed")
        text = CENSUS.read_text(encoding="utf-8")
        committed = census_hashes(text)
        self.assertEqual(text, "".join(f"{h}\n" for h in sorted(committed)))
        print(f"examined {len(committed)} stock line digest(s)")
        for path in engine_goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(anki_generate.census_refusals(path.stem, golden, committed), [])

    def test_every_engine_golden_is_read_by_a_rust_test(self):
        """dm.oracle-consumed (R14, L8:810): each engine golden's name is in a Rust test's code."""
        consumers = {
            "a.rs": '#[test]\nfn t() { read("anki.timing.day"); }\n// "anki.render.card"\n'
        }
        coded = {path: RUST_COMMENT.sub("", code) for path, code in consumers.items()}
        self.assertEqual(
            unconsumed(["anki.timing.day", "anki.render.card"], coded),
            ["anki.render.card: read by no Rust test"],
        )
        found = rust_consumers(ROOT)
        print(f"examined {len(found)} Rust test file(s)")
        self.assertEqual(unconsumed(engine_registered(), found), [])
        for path in engine_goldens(ROOT):
            self.assertEqual(unconsumed([path.stem], found), [])

    def test_the_schema_snapshot_carries_its_pin_line(self):
        """R16 (L8:812): the snapshot is the fresh-collection golden's schema under one pin line,
        PRAGMA user_version, equal to the golden's."""
        writer = snapshot_writer()
        planted = planted_engine(
            function=SNAPSHOT_GOLDEN,
            cases=[
                {
                    "input": {"fresh": True},
                    "output": {
                        "col_ver": 11,
                        "schema": ["CREATE TABLE col (id integer primary key)"],
                        "user_version": 18,
                    },
                }
            ],
        )
        text = writer.snapshot_text(planted)
        self.assertEqual(PIN_LINE.findall(text), ["18"])
        self.assertIn("CREATE TABLE col (id integer primary key);\n", text)
        self.assertEqual(date_strings("snapshot", {"text": text}), [])
        two = dict(planted, cases=planted["cases"] * 2)
        with self.assertRaisesRegex(ValueError, "one case"):
            writer.snapshot_text(two)
        golden_path = ROOT / ORACLE / "goldens" / f"{SNAPSHOT_GOLDEN}.json"
        self.assertTrue(golden_path.is_file(), f"{golden_path.name} is not committed")
        self.assertTrue(SNAPSHOT.is_file(), f"{SNAPSHOT.relative_to(ROOT)} is not committed")
        golden = strict(golden_path.read_text(encoding="utf-8"))
        committed = SNAPSHOT.read_text(encoding="utf-8")
        self.assertEqual(committed, writer.snapshot_text(golden))
        self.assertEqual(
            PIN_LINE.findall(committed), [str(golden["cases"][0]["output"]["user_version"])]
        )

    def test_the_two_oracles_split_the_goldens_whole(self):
        """Every committed golden is the predecessor's or the engine's, never both."""
        names = sorted(path.name for path in (ROOT / ORACLE / "goldens").glob("*.json"))
        old = {path.name for path in goldens(ROOT)}
        engine = {path.name for path in (ROOT / ORACLE / "goldens").glob(f"{ENGINE_PREFIX}*.json")}
        self.assertEqual(sorted(old | engine), names)
        self.assertEqual(old & engine, set())
        for path in engine_goldens(ROOT):
            golden = strict(path.read_text(encoding="utf-8"))
            self.assertEqual(
                (golden["kind"], golden["generator"], golden["function"]),
                ("engine", ANKI_GENERATOR, path.stem),
            )
            self.assertEqual(path.read_text(encoding="utf-8"), anki_generate.render(golden))


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

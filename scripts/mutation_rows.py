#!/usr/bin/env python3
"""mutation_rows: DeckStreak's hand-proved mutation rows, read, censused, proved and retired
(SPEC-039 R8 to R11, ADR-057).

    python3 scripts/mutation_rows.py count|ids [--root DIR]
    python3 scripts/mutation_rows.py census [--root DIR]
    python3 scripts/mutation_rows.py prove [--root DIR] (--all | --row ID ... | --band S<lo>-S<hi>
                                           | --rows-from PLAN) [--report FILE]
    python3 scripts/mutation_rows.py retired --base REF [--root DIR]

WHY. A generated mutant tests code the way a tool can mutate it. A constant, a method named `new`
(cargo-mutants never looks inside one), a guard or a parity comparison is tested weakly or not at
all that way, so each such invariant carries a row written by hand: an anchor that occurs exactly
once in its target, one mutant, and a killer that names exactly one test (SPEC-039 §1).

THE POPULATION is the mutation-rows pack's shape. `scripts/mutation-rows.json` holds the header
alone (`_`, `arities`, and empty `tables`), and each SPEC's rows live in its own band fragment,
`scripts/mutation-rows.d/S<NNN>00-S<NNN>99.json`, holding `{"tables": {...}}` and nothing else.
This module is the one reader (`load_tree`, `load_revision`). The assembly refuses, naming the
file: a fragment not named for one SPEC's band, a fragment with any key but `tables`, a table the
header does not declare, a row whose id lies outside its band, and an id held twice. A row's target
is read by its table's declared spelling: the header's `_` states, one line per table,
`target spelling: <table> is crate-relative, crates/{cell 1}/{cell 2}` or `... is repo-rooted,
{cell 1}`, and a table with no such line is refused rather than read under a guess.

A KILLER names one test. A cargo killer is `<target>::<test path>`: an integration-test target of
the row's crate (`crates/<crate>/tests/<target>.rs`), or `lib` for the crate's unit tests, then the
test's path in it. A script killer is `<module>.<Class>.<method>`, a unittest id whose module lives
in one of the unittest roots the gate discovers (`scripts/tests`, `tools/parity-oracle`).

PROVE (R9) refuses a tree with a tracked change, since it rewrites a tracked file and restores it.
For each row it checks the anchor occurs exactly once and records the target's sha256; runs the
killer without the mutant, which must pass selecting exactly one test; installs the mutant once;
refuses a mutant that does not build (`cargo test --no-run`) or parse (Python, or a shell script
by `bash -n` or `sh -n`, chosen by its shebang or extension) as VOID, never a kill; runs the
killer, which must select exactly one test, counted from libtest's `running N test` lines or
unittest's `Ran N test` line; reads a failure as KILLED and a pass as SURVIVED; then
writes the saved bytes back and checks the sha256 before anything else runs. Exit 0 when every row
was KILLED, 1 on a survivor, 2 on a refusal, 3 on a VOID with no survivor, 4 when a restore failed.

CENSUS (R11) holds every committed row without running anything: its target exists, its find is
not empty and differs from its replacement, its anchor occurs exactly once, its killer names exactly
one test, and no second row installs a mutant another row installs for the same killer. It exits 3
when it examined no row. RETIRED (R11) refuses a row that left the population while its target file
stayed, unless `scripts/mutation-rows.retired.json` records its id with a reason and the
maintainer's approval.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
import pathlib
import posixpath
import re
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass

sys.dont_write_bytecode = True

MONOLITH = "scripts/mutation-rows.json"
FRAGMENTS = "scripts/mutation-rows.d"
RETIRED = "scripts/mutation-rows.retired.json"
BAND_NAME = re.compile(r"S([0-9]+)-S([0-9]+)\.json")
ID_STEM = re.compile(r"S([0-9]+)")
ROW_ID = re.compile(r"S[0-9]+-[A-Z0-9]+(?:-[A-Z0-9]+)*")
#: The unittest roots the gate's python stage discovers (scripts/check.sh).
TEST_ROOTS = ("scripts/tests", "tools/parity-oracle", "agent/tests")
#: Where each table keeps its find, its replacement, its killer's crate and its killer.
CELLS = {
    "MUTATIONS": {"find": 3, "replace": 4, "crate": 1, "killer": 5, "description": 6},
    "CARGO_KILLED_SCRIPT_MUTATIONS": {
        "find": 2,
        "replace": 3,
        "crate": 4,
        "killer": 5,
        "description": 6,
    },
    "SCRIPT_MUTATIONS": {"find": 2, "replace": 3, "crate": None, "killer": 5, "description": 4},
}
CARGO_KILLER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+")
SCRIPT_KILLER = re.compile(
    r"[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*"
)
TEST_ATTRIBUTE = re.compile(r"^\s*#\[(?:[a-z_]+::)*test\b")
LIBTEST_RUNNING = re.compile(r"(?m)^running (\d+) tests?$")
LIBTEST_IGNORED = re.compile(r"(?m)^test result: \w+\. \d+ passed; \d+ failed; ([1-9]\d*) ignored")
UNITTEST_RAN = re.compile(r"(?m)^Ran (\d+) tests? in ")
UNITTEST_SKIPPED = re.compile(r"(?m)^OK \(skipped=([1-9]\d*)\)$")
#: A cargo build may wait on a busy machine; a killer's test runs under its own bound.
BUILD_SECONDS = 3600
PARSE_SECONDS = 60
#: A shebang that names a shell, directly or after `env`: group 1 is `sh`, `bash` or `dash`.
SHELL_SHEBANG = re.compile(r"^#!\s*(?:\S*/)?(?:env\s+(?:-\S+\s+)*)?(?:\S*/)?(sh|bash|dash)(?=\s|$)")
TEST_SECONDS = 900

EXIT_OK, EXIT_SURVIVED, EXIT_REFUSED, EXIT_VOID, EXIT_RESTORE = 0, 1, 2, 3, 4


class PopulationRefused(ValueError):
    """The header and its fragments do not assemble into one population."""


class KillerUnresolved(ValueError):
    """A killer that does not name exactly one test the tree holds."""


# --------------------------------------------------------------------------- the reader


def fragment_band(name: str) -> tuple[int, int] | None:
    """The band `S<lo>-S<hi>.json` names, or None when the name is not a band."""
    match = BAND_NAME.fullmatch(name)
    if match is None:
        return None
    lo, hi = int(match.group(1)), int(match.group(2))
    return (lo, hi) if lo <= hi else None


def spec_band(name: str) -> bool:
    """Whether `name` is one SPEC's band, `S<NNN>00-S<NNN>99.json` (ADR-057 D5)."""
    band = fragment_band(name)
    return band is not None and band[0] % 100 == 0 and band[1] == band[0] + 99


def _row_id(row: object) -> str | None:
    if isinstance(row, list) and row and isinstance(row[0], str):
        return row[0]
    return None


def assemble(monolith: object, fragments: list[tuple[str, object]]) -> object:
    """The population: the header's document with each fragment's rows appended, table by table,
    fragments in band order. Raises `PopulationRefused` naming the file for every refusal the
    module docstring lists."""
    if not isinstance(monolith, dict):
        raise PopulationRefused(f"{MONOLITH} must hold a top-level object")
    arities, tables = monolith.get("arities"), monolith.get("tables")
    if not isinstance(arities, dict) or not isinstance(tables, dict):
        raise PopulationRefused(f"{MONOLITH} must declare arities and tables objects")
    banded = []
    for name, document in fragments:
        if not spec_band(name):
            raise PopulationRefused(
                f"{FRAGMENTS}/{name} is not named for one SPEC's band, S<NNN>00-S<NNN>99.json"
            )
        banded.append((fragment_band(name), name, document))
    banded.sort(key=lambda item: item[0])
    for (earlier, first, _), (later, second, _) in zip(banded, banded[1:]):
        if later[0] <= earlier[1]:
            raise PopulationRefused(f"{FRAGMENTS}/{first} and {FRAGMENTS}/{second} overlap")
    held: dict[str, str] = {}
    merged = {name: list(rows) for name, rows in tables.items()}
    for rows in tables.values():
        for row in rows if isinstance(rows, list) else ():
            identifier = _row_id(row)
            if identifier is not None:
                if identifier in held:
                    raise PopulationRefused(f"the row id {identifier} is held twice in {MONOLITH}")
                held[identifier] = MONOLITH
    for (lo, hi), name, document in banded:
        where = f"{FRAGMENTS}/{name}"
        if not isinstance(document, dict) or list(document) != ["tables"]:
            raise PopulationRefused(f"{where} must hold one key, `tables`")
        carried = document["tables"]
        if not isinstance(carried, dict):
            raise PopulationRefused(f"{where}: `tables` is not an object")
        for table, rows in carried.items():
            if table not in arities:
                raise PopulationRefused(f"{where} holds the table {table}, which {MONOLITH} lacks")
            if not isinstance(rows, list):
                raise PopulationRefused(f"{where}: {table} is not an array of rows")
            for row in rows:
                identifier = _row_id(row)
                if identifier is None:
                    raise PopulationRefused(f"{where}: a {table} row has no id")
                stem = ID_STEM.match(identifier)
                if stem is None or not lo <= int(stem.group(1)) <= hi:
                    raise PopulationRefused(f"{where}: {identifier} lies outside S{lo}-S{hi}")
                if identifier in held:
                    raise PopulationRefused(
                        f"the row id {identifier} is held twice: in {held[identifier]} and {where}"
                    )
                held[identifier] = where
            merged.setdefault(table, []).extend(rows)
    population = dict(monolith)
    population["tables"] = merged
    return population


#: The words of a repeated-key refusal, after the file's name (SPEC-122 R1).
REPEATED_KEY = "repeats the key"


class _RepeatedKey(Exception):
    """A key seen twice in one object; `parse_document` turns it into a refusal naming the file."""

    def __init__(self, key: str) -> None:
        super().__init__(key)
        self.key = key


def _refuse_repeats(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """`json`'s object hook: the object's members, refusing a key the object already holds."""
    seen: dict[str, object] = {}
    for key, value in pairs:
        if key in seen:
            raise _RepeatedKey(key)
        seen[key] = value
    return seen


def parse_document(where: str, text: str) -> object:
    """A document of the population, parsed. `json.loads` keeps the last value of a repeated key
    and says nothing, and two branches that each add a table under one key merge in git without a
    conflict (#334), so a key seen twice in one object, at any depth, is a refusal naming `where`
    and the key. Text that is not JSON is refused by name too (SPEC-122 R1)."""
    try:
        return json.loads(text, object_pairs_hook=_refuse_repeats)
    except _RepeatedKey as repeated:
        raise PopulationRefused(f"{where} {REPEATED_KEY} {repeated.key!r} in one object") from None
    except json.JSONDecodeError as error:
        raise PopulationRefused(f"{where} is not JSON: {error}") from error


def _fragment(name: str, text: str) -> tuple[str, object]:
    return name, parse_document(f"{FRAGMENTS}/{name}", text)


def tree_fragments(root: pathlib.Path | str) -> list[tuple[str, object]]:
    """Every fragment of the tree at `root`, parsed; none when the directory is absent."""
    directory = pathlib.Path(root) / FRAGMENTS
    if not directory.exists():
        return []
    if not directory.is_dir():
        raise PopulationRefused(f"{FRAGMENTS} is not a directory")
    return [
        _fragment(entry.name, entry.read_text(encoding="utf-8"))
        for entry in sorted(directory.iterdir(), key=lambda entry: entry.name)
    ]


def load_tree(root: pathlib.Path | str) -> object:
    """The population of the tree at `root`: its header, read and parsed, then its fragments."""
    text = (pathlib.Path(root) / MONOLITH).read_text(encoding="utf-8")
    return assemble(parse_document(MONOLITH, text), tree_fragments(root))


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=True
    ).stdout


def load_revision(root: pathlib.Path | str, revision: str) -> object | None:
    """The population a revision holds, read through git; None when it holds no header."""
    root = pathlib.Path(root)
    listed = git(root, "ls-tree", "--name-only", revision, "--", MONOLITH)
    if not listed.strip():
        return None
    monolith = parse_document(MONOLITH, git(root, "show", f"{revision}:{MONOLITH}"))
    names = git(root, "ls-tree", "--name-only", "-z", revision, "--", FRAGMENTS + "/")
    fragments = [
        _fragment(path.rsplit("/", 1)[1], git(root, "show", f"{revision}:{path}"))
        for path in sorted(entry for entry in names.split("\0") if entry)
    ]
    return assemble(monolith, fragments)


SPELLING_PREFIX = "target spelling: "
CRATE_RELATIVE = "crate-relative"
REPO_ROOTED = "repo-rooted"


class UnresolvableTarget(ValueError):
    """A table declares no target spelling, or a row lacks a cell its spelling reads."""


def declared_spellings(document: object) -> dict[str, str]:
    """`{table: spelling}` as the header's `_` declares it, for every table the document carries."""
    legend = document.get("_") if isinstance(document, dict) else None
    spellings: dict[str, str] = {}
    for line in legend if isinstance(legend, list) else []:
        if not isinstance(line, str) or not line.startswith(SPELLING_PREFIX):
            continue
        table, separator, rest = line[len(SPELLING_PREFIX) :].partition(" is ")
        word = rest.split(",", 1)[0].strip()
        if not separator or not table or word not in (CRATE_RELATIVE, REPO_ROOTED):
            raise UnresolvableTarget(f"the line {line!r} declares no spelling this reader knows")
        if table in spellings:
            raise UnresolvableTarget(f"table {table} declares its spelling twice")
        spellings[table] = word
    tables = document.get("tables") if isinstance(document, dict) else None
    for table in tables if isinstance(tables, dict) else ():
        if table not in spellings:
            raise UnresolvableTarget(f"table {table} declares no target spelling")
    return spellings


def row_target(table: str, row: list, spellings: dict[str, str]) -> str:
    """A row's target, from the repository's root, lexically normalised."""
    spelling = spellings.get(table)
    if spelling is None:
        raise UnresolvableTarget(f"table {table} declares no target spelling")
    identifier = row[0] if row else "<no id>"
    if spelling == CRATE_RELATIVE:
        if len(row) < 3:
            raise UnresolvableTarget(f"{table} row {identifier!r} carries no crate and path")
        return posixpath.normpath(f"crates/{row[1]}/{row[2]}")
    if len(row) < 2:
        raise UnresolvableTarget(f"{table} row {identifier!r} carries no path")
    return posixpath.normpath(row[1])


# --------------------------------------------------------------------------- rows and killers


@dataclass(frozen=True, slots=True)
class Row:
    """One row, read by its table's layout."""

    id: str
    table: str
    target: str
    find: str
    replace: str
    killer: str
    crate: str | None
    description: str


@dataclass(frozen=True, slots=True)
class Killer:
    """The one test a row's killer names, and how to run it."""

    kind: str
    name: str
    file: str
    package: str | None = None
    target: str | None = None
    cwd: str | None = None


def rows_of(population: object) -> list[Row]:
    """Every row of the population, in its table's layout, arity checked against the header."""
    spellings = declared_spellings(population)
    arities = population.get("arities", {})
    found = []
    for table, rows in population["tables"].items():
        if table not in CELLS:
            raise PopulationRefused(f"the table {table} has a layout this reader does not know")
        cells = CELLS[table]
        for row in rows:
            if len(row) not in arities.get(table, ()):
                raise PopulationRefused(
                    f"{_row_id(row)}: {len(row)} cells, where {table} declares {arities[table]}"
                )
            found.append(
                Row(
                    id=row[0],
                    table=table,
                    target=row_target(table, row, spellings),
                    find=row[cells["find"]],
                    replace=row[cells["replace"]],
                    killer=row[cells["killer"]],
                    crate=row[cells["crate"]] if cells["crate"] is not None else None,
                    description=row[cells["description"]],
                )
            )
    return found


def package_of(root: pathlib.Path, crate: str) -> str:
    """The Cargo package name of `crates/<crate>`."""
    manifest = root / "crates" / crate / "Cargo.toml"
    try:
        return tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]["name"]
    except (OSError, KeyError, tomllib.TOMLDecodeError) as error:
        raise KillerUnresolved(f"crates/{crate} holds no readable Cargo package") from error


def test_functions(text: str, name: str) -> int:
    """How many functions named `name` the Rust source `text` declares under a test attribute."""
    lines = text.splitlines()
    count = 0
    for number, line in enumerate(lines):
        if re.search(rf"\bfn {re.escape(name)}\s*[(<]", line):
            above = lines[max(0, number - 4) : number]
            if any(TEST_ATTRIBUTE.match(attribute) for attribute in above):
                count += 1
    return count


def locate_killer(root: pathlib.Path, row: Row) -> Killer:
    """Where a row's killer runs: its package and test target, or its unittest root. The runner
    counts what it selects from the killer's own output (R9); `resolve_killer` is the census's
    static check that it names exactly one test."""
    if row.crate is not None:
        if "::" not in row.killer:
            raise KillerUnresolved(f"the killer {row.killer!r} is not <target>::<test path>")
        target, path = row.killer.split("::", 1)
        try:
            package = package_of(root, row.crate)
        except KillerUnresolved:
            package = None
        crate = root / "crates" / row.crate
        if target == "lib":
            where = f"crates/{row.crate}/src"
        else:
            candidates = [crate / "tests" / f"{target}.rs", crate / "tests" / target / "main.rs"]
            files = [candidate for candidate in candidates if candidate.is_file()]
            if not files:
                raise KillerUnresolved(f"crates/{row.crate} has no test target {target}")
            where = files[0].relative_to(root).as_posix()
        return Killer("cargo", path, where, package=package, target=target)
    module = row.killer.split(".", 1)[0]
    homes = [base for base in TEST_ROOTS if (root / base / f"{module}.py").is_file()]
    if len(homes) != 1:
        raise KillerUnresolved(f"names no test: {len(homes)} unittest roots hold {module}.py")
    return Killer("script", row.killer, f"{homes[0]}/{module}.py", cwd=homes[0])


def resolve_killer(root: pathlib.Path, row: Row) -> Killer:
    """The one test a row's killer names, found statically; raises `KillerUnresolved`."""
    killer = locate_killer(root, row)
    if killer.kind == "cargo":
        if not CARGO_KILLER.fullmatch(row.killer):
            raise KillerUnresolved(f"the killer {row.killer!r} is not <target>::<test path>")
        name = killer.name.rsplit("::", 1)[-1]
        if killer.target == "lib":
            sources = sorted((root / killer.file).rglob("*.rs"))
            count = sum(test_functions(s.read_text(encoding="utf-8"), name) for s in sources)
        else:
            count = test_functions((root / killer.file).read_text(encoding="utf-8"), name)
        if count != 1:
            raise KillerUnresolved(f"names no test: {killer.file} declares {name} {count} times")
        return killer
    if not SCRIPT_KILLER.fullmatch(row.killer):
        raise KillerUnresolved(f"the killer {row.killer!r} is not <module>.<Class>.<method>")
    _module, klass, method = row.killer.split(".")
    tree = ast.parse((root / killer.file).read_text(encoding="utf-8"))
    methods = [
        node
        for cls in tree.body
        if isinstance(cls, ast.ClassDef) and cls.name == klass
        for node in cls.body
        if isinstance(node, ast.FunctionDef) and node.name == method
    ]
    if len(methods) != 1:
        raise KillerUnresolved(f"names no test: {killer.file} has no {klass}.{method}")
    return killer


# --------------------------------------------------------------------------- the census


def census(root: pathlib.Path) -> tuple[list[str], int]:
    """(findings, rows examined) over every committed row, running nothing."""
    try:
        population = load_tree(root)
        rows = rows_of(population)
    except (OSError, json.JSONDecodeError, PopulationRefused, UnresolvableTarget) as e:
        return [f"census: the population does not assemble: {e}"], 0
    findings = []
    # One mutant, for one killer, is one piece of evidence: a second row that installs it adds none.
    installed: dict[tuple, str] = {}
    for row in rows:
        mutant = (row.target, row.find, row.replace, row.crate, row.killer)
        if mutant in installed:
            findings.append(
                f"census: {row.id}: installs the mutant {installed[mutant]} installs, for the "
                "same killer"
            )
        installed.setdefault(mutant, row.id)
        target = root / row.target
        if not target.is_file():
            findings.append(f"census: {row.id}: its target {row.target} does not exist")
            continue
        if not row.find or row.find == row.replace:
            findings.append(f"census: {row.id}: its find is empty or equals its replacement")
            continue
        occurs = target.read_text(encoding="utf-8").count(row.find)
        if occurs != 1:
            findings.append(f"census: {row.id}: its anchor occurs {occurs} times in {row.target}")
        try:
            resolve_killer(root, row)
        except KillerUnresolved as refusal:
            findings.append(f"census: {row.id}: its killer {row.killer} {refusal}")
    return findings, len(rows)


# --------------------------------------------------------------------------- the runner


@dataclass
class Run:
    """One run of a killer: how many tests it selected, whether it passed, and why not."""

    selected: int
    passed: bool
    note: str = ""


def tracked_changes(root: pathlib.Path) -> list[str]:
    status = git(root, "status", "--porcelain=v1", "--untracked-files=no")
    return [line[3:] for line in status.splitlines() if line.strip()]


def run_killer(root: pathlib.Path, killer: Killer, scratch: pathlib.Path) -> Run:
    """The killer, run alone; its selection is counted from the runner's own output."""
    if killer.kind == "cargo":
        flags = ["--lib"] if killer.target == "lib" else ["--test", killer.target]
        command = ["cargo", "test", "--locked", "-p", killer.package, *flags]
        command += ["--", "--exact", killer.name]
        env = dict(os.environ, CARGO_TERM_COLOR="never")
        cwd = root
    else:
        command = [sys.executable, "-m", "unittest", killer.name]
        cache = tempfile.mkdtemp(prefix="pycache-", dir=scratch)
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1", PYTHONPYCACHEPREFIX=cache)
        cwd = root / killer.cwd
    try:
        done = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            timeout=BUILD_SECONDS + TEST_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return Run(0, False, "it timed out")
    if killer.kind == "cargo":
        selected = sum(int(count) for count in LIBTEST_RUNNING.findall(done.stdout))
        if LIBTEST_IGNORED.search(done.stdout):
            return Run(selected, False, "its test is ignored")
    else:
        selected = sum(int(count) for count in UNITTEST_RAN.findall(done.stderr))
        if UNITTEST_SKIPPED.search(done.stderr):
            return Run(selected, False, "its test is skipped")
    return Run(selected, done.returncode == 0)


def shell_parser(target: str, text: bytes) -> str | None:
    """`bash` or `sh` when the target is a shell script, else None. The shebang decides when it
    names a shell, since a `.sh` file may be bash; else the extension does, and `.bash` is bash."""
    first = text.split(b"\n", 1)[0].decode("utf-8", errors="replace")
    named = SHELL_SHEBANG.match(first)
    if named:
        return "bash" if named.group(1) == "bash" else "sh"
    if target.endswith(".bash"):
        return "bash"
    if target.endswith(".sh"):
        return "sh"
    return None


def parses(parser: str, mutated: bytes) -> str | None:
    """None when the shell reads the mutated bytes, else why it does not: `<parser> -n` reads
    them from stdin and runs nothing, and a missing shell fails closed."""
    try:
        done = subprocess.run(
            [parser, "-n"],
            input=mutated,
            capture_output=True,
            timeout=PARSE_SECONDS,
            check=False,
        )
    except FileNotFoundError:
        return f"the mutant is unchecked: {parser} is not installed"
    except subprocess.TimeoutExpired:
        return f"the mutant is unchecked: {parser} -n timed out"
    if done.returncode == 0:
        return None
    lines = done.stderr.decode("utf-8", errors="replace").strip().splitlines()
    why = lines[0] if lines else f"exit {done.returncode}"
    return f"the mutant does not parse: {parser}: {why}"


def builds(root: pathlib.Path, row: Row, killer: Killer, mutated: bytes) -> str | None:
    """None when the installed mutant builds or parses; else why it does not."""
    if row.target.endswith(".py"):
        try:
            ast.parse(mutated.decode("utf-8"), filename=row.target)
        except SyntaxError as error:
            return f"the mutant does not parse: {error.msg} at line {error.lineno}"
        return None
    parser = shell_parser(row.target, mutated)
    if parser is not None:
        refusal = parses(parser, mutated)
        if refusal is not None:
            return refusal
    if killer.kind == "cargo":
        flags = ["--lib"] if killer.target == "lib" else ["--test", killer.target]
        done = subprocess.run(
            ["cargo", "test", "--locked", "-p", killer.package, *flags, "--no-run"],
            cwd=root,
            env=dict(os.environ, CARGO_TERM_COLOR="never"),
            capture_output=True,
            text=True,
            timeout=BUILD_SECONDS,
            check=False,
        )
        if done.returncode != 0:
            return "the mutant does not build"
    return None


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class RestoreFailed(RuntimeError):
    """A target's bytes differ from the ones saved before its mutant was installed."""


def prove_row(root: pathlib.Path, row: Row, scratch: pathlib.Path) -> dict:
    """One row's proof, as the report records it; the target is restored byte for byte."""
    entry = {"id": row.id, "target": row.target, "control": None, "mutant": None}

    def verdict(word: str, reason: str) -> dict:
        entry.update(verdict=word, reason=reason)
        return entry

    target = root / row.target
    original = target.read_bytes()
    entry["sha256"] = sha256(original)
    occurs = original.count(row.find.encode("utf-8"))
    if occurs != 1:
        return verdict("VOID", f"its anchor occurs {occurs} times in {row.target}")
    try:
        killer = locate_killer(root, row)
        if killer.kind == "cargo" and killer.package is None:
            raise KillerUnresolved(f"runs in crates/{row.crate}, which holds no Cargo package")
    except KillerUnresolved as refusal:
        return verdict("VOID", f"its killer {row.killer} {refusal}")
    control = run_killer(root, killer, scratch)
    entry["control"] = {"selected": control.selected, "passed": control.passed}
    if control.note:
        return verdict("VOID", f"without its mutant, {control.note}")
    if control.selected != 1:
        return verdict("VOID", f"its killer selected {control.selected} tests, not one")
    if not control.passed:
        return verdict("VOID", "its killer fails without its mutant")
    mutated = original.replace(row.find.encode("utf-8"), row.replace.encode("utf-8"), 1)
    target.write_bytes(mutated)
    try:
        refusal = builds(root, row, killer, mutated)
        if refusal is not None:
            return verdict("VOID", refusal)
        run = run_killer(root, killer, scratch)
        entry["mutant"] = {"selected": run.selected, "passed": run.passed}
        if run.note:
            return verdict("VOID", f"with its mutant, {run.note}")
        if run.selected != 1:
            return verdict("VOID", f"with its mutant, its killer selected {run.selected} tests")
        if run.passed:
            return verdict("SURVIVED", "its killer passed with the mutant installed")
        return verdict("KILLED", "its killer passed without the mutant and failed with it")
    finally:
        target.write_bytes(original)
        if sha256(target.read_bytes()) != entry["sha256"]:
            raise RestoreFailed(f"{row.target} was not restored byte for byte after {row.id}")


def chosen(rows: list[Row], args: argparse.Namespace) -> list[Row]:
    """Every row the selectors name together, each once: `--all`, or a band's, each `--row` and a
    plan's `--rows-from`. No selector drops another's rows."""
    if args.all:
        return rows
    wanted = []
    if args.band:
        band = fragment_band(f"{args.band}.json")
        if band is None:
            raise PopulationRefused(f"{args.band} is not a band S<lo>-S<hi>")
        wanted += [r.id for r in rows if band[0] <= int(ID_STEM.match(r.id).group(1)) <= band[1]]
    wanted += list(args.row)
    if args.rows_from:
        plan = json.loads(pathlib.Path(args.rows_from).read_text(encoding="utf-8"))
        wanted += plan.get("rows", [])
    by_id = {row.id: row for row in rows}
    missing = [identifier for identifier in wanted if identifier not in by_id]
    if missing:
        raise PopulationRefused(f"no row holds the id(s) {', '.join(missing)}")
    return [by_id[identifier] for identifier in dict.fromkeys(wanted)]


def prove(root: pathlib.Path, args: argparse.Namespace) -> int:
    dirty = tracked_changes(root)
    if dirty:
        print(f"prove: REFUSED: a tracked change the proofs would overwrite: {', '.join(dirty)}")
        return EXIT_REFUSED
    try:
        rows = chosen(rows_of(load_tree(root)), args)
    except (OSError, json.JSONDecodeError, PopulationRefused, UnresolvableTarget) as e:
        print(f"prove: REFUSED: {e}")
        return EXIT_REFUSED
    report = []
    with tempfile.TemporaryDirectory(prefix="mutation-rows-") as scratch:
        for row in rows:
            try:
                entry = prove_row(root, row, pathlib.Path(scratch))
            except RestoreFailed as failure:
                print(f"prove: RESTORE FAILED: {failure}")
                return EXIT_RESTORE
            report.append(entry)
            print(f"{row.id}: {entry['verdict']}: {entry['reason']}", flush=True)
    counts = {word: sum(e["verdict"] == word for e in report) for word in ("KILLED", "SURVIVED")}
    void = len(report) - counts["KILLED"] - counts["SURVIVED"]
    print(
        f"rows: examined {len(report)}: killed {counts['KILLED']}, "
        f"survived {counts['SURVIVED']}, void {void}"
    )
    if args.report:
        pathlib.Path(args.report).write_text(json.dumps(report, indent=2) + "\n", "utf-8")
    print(f"examined {len(report)}")
    if counts["SURVIVED"]:
        return EXIT_SURVIVED
    return EXIT_VOID if void else EXIT_OK


# --------------------------------------------------------------------------- retirement


def retired(root: pathlib.Path, base: str) -> int:
    before = load_revision(root, base)
    if before is None:
        print(f"retired: {base} holds no rows")
        print("examined 0")
        return EXIT_OK
    now = {row.id for row in rows_of(load_tree(root))}
    record = {}
    path = root / RETIRED
    if path.is_file():
        for entry in json.loads(path.read_text(encoding="utf-8")).get("retired", []):
            if str(entry.get("reason", "")).strip() and str(entry.get("approval", "")).strip():
                record[entry["id"]] = entry
    base_rows = rows_of(before)
    refused = 0
    for row in base_rows:
        if row.id in now:
            continue
        if not (root / row.target).exists():
            print(f"retired: {row.id}: its target {row.target} left with it")
        elif row.id in record:
            approval = record[row.id]["approval"]
            print(f"retired: {row.id}: its target stays; retired with approval: {approval}")
        else:
            refused += 1
            print(
                f"retired: {row.id}: REFUSED: it left while its target {row.target} stays, and "
                f"{RETIRED} records no reason and approval for it"
            )
    print(f"examined {len(base_rows)}")
    return EXIT_SURVIVED if refused else EXIT_OK


# --------------------------------------------------------------------------- the command line


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["count", "ids", "census", "prove", "retired"])
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--all", action="store_true", help="prove every row")
    parser.add_argument("--row", action="append", default=[], help="prove this row id")
    parser.add_argument("--band", help="prove every row of the band S<lo>-S<hi>")
    parser.add_argument("--rows-from", help="prove the rows a mutation-verdict plan selected")
    parser.add_argument("--report", help="write each row's proof here, as JSON")
    parser.add_argument("--base", help="retired: the revision to compare the rows against")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()
    if args.verb in ("count", "ids"):
        try:
            rows = rows_of(load_tree(root))
        except (OSError, json.JSONDecodeError, PopulationRefused, UnresolvableTarget) as refusal:
            print(f"mutation_rows: REFUSED: {refusal}", file=sys.stderr)
            return EXIT_REFUSED
        print(len(rows) if args.verb == "count" else "\n".join(row.id for row in rows))
        return EXIT_OK
    if args.verb == "census":
        findings, examined = census(root)
        print("\n".join(findings)) if findings else None
        print(f"examined {examined} row(s)")
        if findings:
            return EXIT_SURVIVED
        return EXIT_OK if examined else EXIT_VOID
    if args.verb == "prove":
        if not (args.all or args.row or args.band or args.rows_from):
            parser.error("prove needs --all, --row, --band or --rows-from")
        return prove(root, args)
    if not args.base:
        parser.error("retired needs --base")
    try:
        return retired(root, args.base)
    except (OSError, json.JSONDecodeError, PopulationRefused, UnresolvableTarget) as refusal:
        print(f"mutation_rows: REFUSED: {refusal}", file=sys.stderr)
        return EXIT_REFUSED


if __name__ == "__main__":
    sys.exit(main())

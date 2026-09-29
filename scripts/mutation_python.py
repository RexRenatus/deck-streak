#!/usr/bin/env python3
"""Mutation testing for the repository's own Python: the guard scripts and the parity oracle's
generator (SPEC-087, ADR-073).

Four verbs. `list` selects mutants and prints them; `run` judges each one by installing it in place
and running the file's mapped tests in a child; `census` checks the map that names those tests; and
`tests` is the child itself.

The population is every `scripts/<name>.py` (one path segment, so nothing under `scripts/tests/`)
and `tools/parity-oracle/generate.py`. `scripts/mutation-python.json` maps each file to its test
directory and the test modules that load or run it.

Exits of `run`: 0 every examined mutant killed, 1 a survivor or an uncovered mutant, 2 a usage
error or a tree with a tracked change, 3 VOID (a failed or empty control, an empty sentinel, a
timeout, a mutant whose child printed no report), 4 a restore that failed. VOID outranks a survivor.

A failed restore (exit 4) writes the report before the run ends, so the outcomes so far are kept,
and records the run's exit under `exit` and the file under `restore_failed`. `exit_code(report)`
derives only 0, 1 or 3 from the outcomes: the 4 belongs to the run that saw the restore fail, not
to a reading of outcomes, and the verdict reads a report whose `exit` is 4 as VOID by name. R7's
text governs where R5, R7, R11 and the verdict's tests read a report-carried exit 4 differently.

The child runs from a copy of this file and of the `scripts/` modules it imports, taken outside
the tree before the first mutant is installed, so no mutant of the runner runs as its own child.
Before it loads a test module it drops the copy's modules from `sys.modules` and removes the copy's
directory from `sys.path`, so a test that imports `mutation_rows` by name loads the tree's file,
the mutant installed.
"""

from __future__ import annotations

import argparse
import ast
import bisect
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import tokenize
import unittest
from collections.abc import Callable
from dataclasses import dataclass, field

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import mutation_rows  # noqa: E402

SCHEMA = "deckstreak.mutation-python.v1"
MAP = "scripts/mutation-python.json"
ORACLE = "tools/parity-oracle/generate.py"
SENTINEL = "\n# mutation-python sentinel\n"
#: The seconds a control or a sentinel may run when `--test-seconds` does not bound it.
CONTROL_SECONDS = 900.0
#: A mutant's bound is the larger of this floor and five times the control's seconds.
BOUND_FLOOR = 60.0
BOUND_FACTOR = 5
LISTED_SHARDS_MAX = 8
OUTCOMES = ("killed", "survived", "uncovered", "unviable", "timeout", "void")
EXIT_OK, EXIT_SURVIVED, EXIT_USAGE, EXIT_VOID, EXIT_RESTORE = 0, 1, 2, 3, 4
NEWLINE = re.compile(r"\r\n|\r|\n")
QUIET = frozenset(
    {
        tokenize.COMMENT,
        tokenize.NL,
        tokenize.NEWLINE,
        tokenize.INDENT,
        tokenize.DEDENT,
        tokenize.ENDMARKER,
    }
)
BINARY = {
    ast.Add: ("+", "-"),
    ast.Sub: ("-", "+"),
    ast.Mult: ("*", "/"),
    ast.Div: ("/", "*"),
    ast.FloorDiv: ("//", "*"),
    ast.Mod: ("%", "*"),
    ast.BitOr: ("|", "&"),
    ast.BitAnd: ("&", "|"),
}
COMPARE = {
    ast.Eq: (("==",), "!="),
    ast.NotEq: (("!=",), "=="),
    ast.Lt: (("<",), ">="),
    ast.LtE: (("<=",), ">"),
    ast.Gt: ((">",), "<="),
    ast.GtE: ((">=",), "<"),
    ast.In: (("in",), "not in"),
    ast.NotIn: (("not", "in"), "in"),
    ast.Is: (("is",), "is not"),
    ast.IsNot: (("is", "not"), "is"),
}
BOOLEAN = {ast.And: ("and", "or"), ast.Or: ("or", "and")}
LONGEST_OLD = 40


# --------------------------------------------------------------------------- the listing


@dataclass(frozen=True, slots=True)
class Mutant:
    """One replacement of `text[start:end]` by `new`, and where and how it is named."""

    line: int
    column: int
    start_line: int
    end_line: int
    end_column: int
    start: int
    end: int
    old: str
    new: str
    operator: str
    function: str

    @property
    def text(self) -> str:
        """The mutant's name after its location: the text a record's `mutant` holds."""
        return f"replace {self.old} with {self.new} in {self.function}"

    def name(self, file: str) -> str:
        return f"{file}:{self.line}:{self.column}: {self.text}"

    def apply(self, source: str) -> str:
        return source[: self.start] + self.new_source + source[self.end :]

    @property
    def new_source(self) -> str:
        return {"nothing": "", "return None": "None"}.get(self.new, self.new)


class Lister:
    """The mutants of one source text, in source order."""

    def __init__(self, source: str) -> None:
        self.source = source
        self.lines = NEWLINE.split(source)
        self.starts = [0]
        for found in NEWLINE.finditer(source):
            self.starts.append(found.end())
        self.tree = ast.parse(source)
        self.tokens = [
            token
            for token in tokenize.generate_tokens(iter(source.splitlines(keepends=True)).__next__)
            if token.type not in QUIET
        ]
        self.token_starts = [token.start for token in self.tokens]
        self.skipped = self.skipped_nodes()
        self.found: list[Mutant] = []

    # positions ---------------------------------------------------------------------------

    def char(self, line: int, byte_column: int) -> tuple[int, int]:
        """A node's (line, column in characters) from the byte column `ast` reports."""
        text = self.lines[line - 1].encode("utf-8")[:byte_column].decode("utf-8", "ignore")
        return (line, len(text))

    def start_of(self, node: ast.AST) -> tuple[int, int]:
        return self.char(node.lineno, node.col_offset)  # type: ignore[attr-defined]

    def end_of(self, node: ast.AST) -> tuple[int, int]:
        return self.char(node.end_lineno, node.end_col_offset)  # type: ignore[attr-defined]

    def offset(self, position: tuple[int, int]) -> int:
        return self.starts[position[0] - 1] + position[1]

    def between(self, after: tuple[int, int], before: tuple[int, int]) -> list[tokenize.TokenInfo]:
        low = bisect.bisect_left(self.token_starts, after)
        found = []
        for token in self.tokens[low:]:
            if token.start >= before:
                break
            found.append(token)
        return found

    # what is never mutated ----------------------------------------------------------------

    def skipped_nodes(self) -> set[int]:
        skipped: set[int] = set()
        for node in ast.walk(self.tree):
            if isinstance(node, ast.JoinedStr):
                skipped.add(id(node))
            elif isinstance(node, ast.AnnAssign):
                skipped.add(id(node.annotation))
            elif isinstance(node, ast.arg) and node.annotation is not None:
                skipped.add(id(node.annotation))
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef) and node.returns:
                skipped.add(id(node.returns))
            if isinstance(
                node, ast.Module | ast.ClassDef | ast.FunctionDef | ast.AsyncFunctionDef
            ) and (
                node.body
                and isinstance(node.body[0], ast.Expr)
                and isinstance(node.body[0].value, ast.Constant)
                and isinstance(node.body[0].value.value, str)
            ):
                skipped.add(id(node.body[0]))
        return skipped

    # the walk -----------------------------------------------------------------------------

    def listing(self) -> list[Mutant]:
        self.visit(self.tree, "<module>", None)
        return sorted(self.found, key=lambda m: (m.start, m.end, m.new))

    def visit(self, node: ast.AST, function: str, klass: str | None) -> None:
        if id(node) in self.skipped:
            return
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
            inner = f"{klass}.{node.name}" if klass else node.name
            for decorator in node.decorator_list:
                self.visit(decorator, function, klass)
            for default in [*node.args.defaults, *node.args.kw_defaults]:
                if default is not None:
                    self.visit(default, function, klass)
            for statement in node.body:
                self.visit(statement, inner, None)
            return
        if isinstance(node, ast.ClassDef):
            for base in [
                *node.bases,
                *[k.value for k in node.keywords],
                *node.decorator_list,
            ]:
                self.visit(base, function, klass)
            for statement in node.body:
                self.visit(statement, function, node.name)
            return
        self.site(node, function)
        for child in ast.iter_child_nodes(node):
            self.visit(child, function, klass)

    # the sites ----------------------------------------------------------------------------

    def add(
        self,
        node: ast.AST,
        token: tuple[tuple[int, int], tuple[int, int]],
        old: str,
        new: str,
        operator: str,
        function: str,
    ) -> None:
        (begin, end) = token
        first = self.start_of(node)
        last = self.end_of(node)
        self.found.append(
            Mutant(
                line=begin[0],
                column=begin[1] + 1,
                start_line=first[0],
                end_line=last[0],
                end_column=last[1] + 1,
                start=self.offset(begin),
                end=self.offset(end),
                old=old,
                new=new,
                operator=operator,
                function=function,
            )
        )

    def operator_token(
        self, after: ast.AST, before: ast.AST, words: tuple[str, ...]
    ) -> tuple[tuple[int, int], tuple[int, int]] | None:
        tokens = self.between(self.end_of(after), self.start_of(before))
        for index, token in enumerate(tokens):
            span = tokens[index : index + len(words)]
            if [t.string for t in span] == list(words):
                return (span[0].start, span[-1].end)
        return None

    def site(self, node: ast.AST, function: str) -> None:
        if isinstance(node, ast.BinOp) and type(node.op) in BINARY:
            symbol, new = BINARY[type(node.op)]
            token = self.operator_token(node.left, node.right, (symbol,))
            if token:
                self.add(node, token, symbol, new, "arithmetic", function)
        elif isinstance(node, ast.Compare):
            lefts = [node.left, *node.comparators[:-1]]
            for op, left, right in zip(node.ops, lefts, node.comparators, strict=True):
                words, new = COMPARE[type(op)]
                token = self.operator_token(left, right, words)
                if token:
                    self.add(node, token, " ".join(words), new, "comparison", function)
        elif isinstance(node, ast.BoolOp):
            word, new = BOOLEAN[type(node.op)]
            for left, right in zip(node.values, node.values[1:], strict=False):
                token = self.operator_token(left, right, (word,))
                if token:
                    self.add(node, token, word, new, "boolean", function)
        elif isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.Not):
            begin = self.start_of(node)
            line = self.lines[begin[0] - 1]
            trailing = len(line) - len(line[begin[1] + 3 :].lstrip(" \t")) - (begin[1] + 3)
            self.add(
                node,
                (begin, (begin[0], begin[1] + 3 + trailing)),
                "not",
                "nothing",
                "not",
                function,
            )
        elif isinstance(node, ast.Constant):
            self.constant(node, function)
        elif isinstance(node, ast.Return):
            value = node.value
            if value is not None and not (isinstance(value, ast.Constant) and value.value is None):
                self.add(
                    node,
                    (self.start_of(value), self.end_of(value)),
                    "return value",
                    "return None",
                    "return",
                    function,
                )
        elif isinstance(node, ast.Break):
            self.add(
                node,
                (self.start_of(node), self.end_of(node)),
                "break",
                "continue",
                "loop",
                function,
            )
        elif isinstance(node, ast.Continue):
            self.add(
                node,
                (self.start_of(node), self.end_of(node)),
                "continue",
                "break",
                "loop",
                function,
            )

    def constant(self, node: ast.Constant, function: str) -> None:
        value = node.value
        span = (self.start_of(node), self.end_of(node))
        if isinstance(value, bool):
            self.add(node, span, str(value), str(not value), "boolean constant", function)
        elif isinstance(value, int):
            self.add(
                node,
                span,
                self.shown(node),
                str(value + 1),
                "integer constant",
                function,
            )
        elif isinstance(value, str) and node.lineno == node.end_lineno:
            new = '"XX"' if value == "" else '""'
            self.add(node, span, self.shown(node), new, "string constant", function)

    def shown(self, node: ast.AST) -> str:
        begin, end = self.start_of(node), self.end_of(node)
        text = self.source[self.offset(begin) : self.offset(end)]
        return text if len(text) <= LONGEST_OLD else text[: LONGEST_OLD - 3] + "..."


def list_source(source: str) -> list[Mutant]:
    """Every mutant of a source text, in source order."""
    return Lister(source).listing()


# --------------------------------------------------------------------------- the map and the census


def population(root: pathlib.Path) -> list[str]:
    """The files whose mutants are judged: `scripts/<name>.py` and the oracle's generator."""
    found = sorted(
        path.relative_to(root).as_posix()
        for path in (root / "scripts").glob("*.py")
        if path.is_file()
    )
    if (root / ORACLE).is_file():
        found.append(ORACLE)
    return sorted(found)


def load_map(root: pathlib.Path) -> tuple[dict, list[str]]:
    """The map and the reasons it cannot be read; a census problem names its file."""
    try:
        loaded = json.loads((root / MAP).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return {}, [f"{MAP}: cannot be read: {error}"]
    if not isinstance(loaded, dict):
        return {}, [f"{MAP}: is not an object of file to entry"]
    return loaded, []


def census_problems(root: pathlib.Path) -> tuple[dict, list[str]]:
    mapping, problems = load_map(root)
    if problems:
        return mapping, problems
    files = population(root)
    for file in files:
        if file not in mapping:
            problems.append(f"{file}: the map omits this population file")
    for key in sorted(mapping):
        if key not in files:
            problems.append(f"{key}: a key outside the population")
    for key, entry in sorted(mapping.items()):
        if not isinstance(entry, dict):
            problems.append(f"{key}: the entry is not an object")
            continue
        for stranger in sorted(set(entry) - {"dir", "modules"}):
            problems.append(f"{key}: the entry carries {stranger!r}, only dir and modules")
        directory, modules = entry.get("dir"), entry.get("modules")
        if not isinstance(directory, str) or not isinstance(modules, list):
            problems.append(f"{key}: the entry needs a dir string and a modules list")
            continue
        seen: set[str] = set()
        for module in modules:
            if not isinstance(module, str):
                problems.append(f"{key}: module {module!r} is not a name")
            elif module in seen:
                problems.append(f"{key}: the module {module} is named twice")
            elif not (root / directory / f"{module}.py").is_file():
                problems.append(f"{key}: the module {module} has no file in {directory}")
            seen.add(str(module))
    return mapping, problems


def census(root: pathlib.Path) -> int:
    mapping, problems = census_problems(root)
    for problem in problems:
        print(f"mutation-python: REFUSED: {problem}")
    print(f"examined {len(mapping)}")
    return EXIT_SURVIVED if problems else EXIT_OK


# --------------------------------------------------------------------------- the child


class Collect(unittest.TestResult):
    """Which tests failed, each down to its sub-test, and how many distinct tests broke."""

    def __init__(self) -> None:
        super().__init__()
        self.failed: list[str] = []
        self.broken: set[str] = set()

    def note(self, test: unittest.TestCase, whole: unittest.TestCase | None = None) -> None:
        identifier = test.id() if not hasattr(test, "_subDescription") else str(test)
        if identifier not in self.failed:
            self.failed.append(identifier)
        self.broken.add((whole or test).id())

    def addFailure(self, test, err):  # noqa: N802
        super().addFailure(test, err)
        self.note(test)

    def addError(self, test, err):  # noqa: N802
        super().addError(test, err)
        self.note(test)

    def addSubTest(self, test, subtest, err):  # noqa: N802
        super().addSubTest(test, subtest, err)
        if err is not None:
            self.note(subtest, test)

    def addUnexpectedSuccess(self, test):  # noqa: N802
        super().addUnexpectedSuccess(test)
        self.note(test)


def flatten(suite: unittest.TestSuite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from flatten(item)
        else:
            yield item


def skipped(test: unittest.TestCase, skip: list[str]) -> bool:
    identifier = test.id()
    return any(gone == identifier or gone.startswith(identifier + " ") for gone in skip)


def child(args: argparse.Namespace) -> int:
    """Run the named test modules and print `{"ran": N, "failed": [...]}` as the last line."""
    copy = str(pathlib.Path(__file__).resolve().parent)
    here = pathlib.Path(copy)
    for name, module in list(sys.modules.items()):
        where = getattr(module, "__file__", None)
        if name != "__main__" and where and pathlib.Path(where).resolve().parent == here:
            del sys.modules[name]
    sys.path[:] = [entry for entry in sys.path if pathlib.Path(entry or ".").resolve() != here]
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    for module in args.module:
        suite.addTests(loader.discover(args.dir, pattern=f"{module}.py"))
    chosen = unittest.TestSuite([t for t in flatten(suite) if not skipped(t, args.skip)])
    result = Collect()
    result.failfast = args.failfast
    real_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        chosen.run(result)
    finally:
        sys.stdout = real_stdout
    failed = result.failed[:1] if args.failfast else result.failed
    print(json.dumps({"ran": result.testsRun, "failed": failed, "broken": len(result.broken)}))
    return EXIT_OK


# --------------------------------------------------------------------------- judging


@dataclass(slots=True)
class Ran:
    """What one child run reported."""

    reported: bool = False
    timed_out: bool = False
    ran: int = 0
    failed: list[str] = field(default_factory=list)
    broken: int = 0
    seconds: float = 0.0


class Restore(RuntimeError):
    """A file's bytes could not be put back, or differ from the bytes read before."""


def restore(path: pathlib.Path, original: bytes) -> None:
    """Write `original` back and prove it by digest; raises `Restore` naming the file."""
    try:
        path.write_bytes(original)
        now = mutation_rows.sha256(path.read_bytes())
    except OSError as error:
        raise Restore(f"{path}: cannot be restored: {error}") from error
    if now != mutation_rows.sha256(original):
        raise Restore(f"{path}: the restored bytes differ from the bytes read before")


def judge_mutant(
    target: pathlib.Path, original: bytes, text: str, tests: Callable[[], Ran]
) -> tuple[str, list[str]]:
    """One mutant's outcome and killers. The text is parsed first, so one that does not parse is
    `unviable` and runs no test; else it is installed in place, tested and always restored."""
    try:
        ast.parse(text)
    except (SyntaxError, ValueError):
        return "unviable", []
    try:
        try:
            target.write_bytes(text.encode("utf-8"))
        except OSError as error:
            raise Restore(f"{target}: the mutant could not be installed: {error}") from error
        ran = tests()
    finally:
        restore(target, original)
    if ran.timed_out:
        return "timeout", []
    if not ran.reported or ran.ran == 0:
        return "void", []
    if ran.failed:
        return "killed", ran.failed
    return "survived", []


def count(report: dict) -> dict[str, int]:
    """Each outcome's count and `examined`: killed, survived and uncovered."""
    counts = dict.fromkeys(OUTCOMES, 0)
    for entry in report.get("files", []):
        for mutant in entry.get("mutants", []):
            counts[mutant["outcome"]] += 1
    counts["examined"] = counts["killed"] + counts["survived"] + counts["uncovered"]
    return counts


def exit_code(report: dict) -> int:
    """0, 1 or 3, from the outcomes alone; a failed restore's 4 is the run's, not a reading."""
    counts = count(report)
    if counts["timeout"] or counts["void"] or any(f.get("void") for f in report.get("files", [])):
        return EXIT_VOID
    if counts["survived"] or counts["uncovered"]:
        return EXIT_SURVIVED
    return EXIT_OK


class Judge:
    """One `run`: the tests' child, the report, and the restore."""

    def __init__(self, root: pathlib.Path, mapping: dict, args: argparse.Namespace) -> None:
        self.root, self.mapping, self.args = root, mapping, args
        scratch = tempfile.mkdtemp(prefix="mutation-python-")
        self.scratch = pathlib.Path(scratch)
        self.copy = self.scratch / "runner"
        self.copy.mkdir()
        self.copy_runner()
        self.pycache = self.scratch / "pycache"

    def copy_runner(self) -> None:
        here = pathlib.Path(__file__).resolve().parent
        wanted, done = ["mutation_python"], set()
        while wanted:
            name = wanted.pop()
            source = here / f"{name}.py"
            if name in done or not source.is_file():
                continue
            done.add(name)
            shutil.copy(source, self.copy / source.name)
            for node in ast.walk(ast.parse(source.read_text(encoding="utf-8"))):
                if isinstance(node, ast.Import):
                    wanted += [alias.name.split(".")[0] for alias in node.names]
                elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
                    wanted.append(node.module.split(".")[0])

    def tests(self, entry: dict, skip: list[str], failfast: bool, bound: float) -> Ran:
        command = [
            sys.executable,
            str(self.copy / "mutation_python.py"),
            "tests",
            "--dir",
            str(self.root / entry["dir"]),
        ]
        for module in entry["modules"]:
            command += ["--module", module]
        for identifier in skip:
            command += ["--skip", identifier]
        if failfast:
            command.append("--failfast")
        env = {
            **os.environ,
            "PYTHONDONTWRITEBYTECODE": "1",
            "PYTHONPYCACHEPREFIX": str(self.pycache),
        }
        began = time.monotonic()
        # The child's report goes to a file, never a pipe: a test that leaves a process holding
        # the pipe open would keep a read waiting past the child's own exit.
        with tempfile.TemporaryFile("w+b") as sink:
            process = subprocess.Popen(
                command,
                cwd=self.root / entry["dir"],
                env=env,
                stdout=sink,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
            try:
                process.wait(timeout=bound)
            except subprocess.TimeoutExpired:
                self.kill(process)
                return Ran(timed_out=True, seconds=time.monotonic() - began)
            finally:
                self.kill(process)
            sink.seek(0)
            out = sink.read().decode("utf-8", "replace")
        ran = Ran(seconds=time.monotonic() - began)
        for line in reversed(out.splitlines()):
            try:
                report = json.loads(line)
            except ValueError:
                continue
            if isinstance(report, dict) and "ran" in report and "failed" in report:
                ran.reported = True
                ran.ran, ran.failed = int(report["ran"]), list(report["failed"])
                ran.broken = int(report.get("broken", len(ran.failed)))
                break
        return ran

    @staticmethod
    def kill(process: subprocess.Popen) -> None:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass

    def file(self, path: str, selected: list[Mutant], report: dict) -> None:
        """Judge one file's selected mutants into `report`; raises `Restore`."""
        entry = self.mapping[path]
        target = self.root / path
        original = target.read_bytes()
        record: dict = {
            "path": path,
            "modules": list(entry["modules"]),
            "control": None,
            "bound": None,
            "byte_readers": [],
            "void": None,
            "mutants": [],
        }
        report["files"].append(record)
        source = original.decode("utf-8")

        def mutant_record(m: Mutant, outcome: str, killers: list[str]) -> dict:
            return {
                "name": m.name(path),
                "file": path,
                "line": m.line,
                "start_line": m.start_line,
                "end_line": m.end_line,
                "end_column": m.end_column,
                "column": m.column,
                "mutant": m.text,
                "operator": m.operator,
                "outcome": outcome,
                "killers": killers,
            }

        def all_as(outcome: str) -> None:
            record["mutants"] = [mutant_record(m, outcome, []) for m in selected]

        if not entry["modules"]:
            all_as("uncovered")
            return
        limit = self.args.test_seconds or CONTROL_SECONDS
        control = self.tests(entry, [], False, limit)
        seconds = (
            self.args.control_seconds if self.args.control_seconds is not None else control.seconds
        )
        record["control"] = {
            "ran": control.ran,
            "failures": len(control.failed),
            "seconds": seconds,
        }
        if control.timed_out or not control.reported or control.failed or control.ran == 0:
            reason = (
                "the control ran past its bound"
                if control.timed_out
                else "the control printed no report"
                if not control.reported
                else f"the control failed unmutated: {', '.join(control.failed)}"
                if control.failed
                else "the control selected no test"
            )
            record["void"] = reason
            all_as("void")
            return
        bound = self.args.test_seconds or max(BOUND_FLOOR, BOUND_FACTOR * seconds)
        record["bound"] = bound
        try:
            target.write_bytes((source + SENTINEL).encode("utf-8"))
            sentinel = self.tests(entry, [], False, bound)
        finally:
            restore(target, original)
        if sentinel.timed_out or not sentinel.reported:
            record["void"] = "the sentinel run ran past its bound or printed no report"
            all_as("void")
            return
        record["byte_readers"] = list(sentinel.failed)
        if sentinel.ran - sentinel.broken <= 0:
            record["void"] = "the sentinel left no test that does not read the file's bytes"
            all_as("void")
            return
        for m in selected:
            outcome, killers = judge_mutant(
                target,
                original,
                m.apply(source),
                lambda: self.tests(entry, record["byte_readers"], self.args.failfast, bound),
            )
            record["mutants"].append(mutant_record(m, outcome, killers))


def select(root: pathlib.Path, mapping: dict, args: argparse.Namespace) -> dict[str, list[Mutant]]:
    """The selected mutants by file, files in path order, after `--shard`."""
    if args.all:
        files = sorted(mapping)
        lines: dict[str, set[int] | None] = dict.fromkeys(files)
    elif args.file:
        files = [args.file]
        lines = {args.file: None}
    else:
        plan = json.loads(pathlib.Path(args.plan).read_text(encoding="utf-8"))
        applies = {name for name, klass in plan.get("classes", {}).items() if klass.get("applies")}
        lines = {}
        for entry in plan.get("files", []):
            if entry.get("class") in applies and entry["path"] in mapping:
                lines.setdefault(entry["path"], set()).update(entry.get("code", []))
        files = sorted(lines)
    chosen: list[tuple[str, Mutant]] = []
    for file in files:
        for mutant in list_source((root / file).read_text(encoding="utf-8")):
            wanted = lines[file]
            if wanted is None or any(mutant.start_line <= n <= mutant.end_line for n in wanted):
                chosen.append((file, mutant))
    if args.shard:
        k, n = args.shard
        chosen = chosen[k::n]
    by_file: dict[str, list[Mutant]] = {}
    for file, mutant in chosen:
        by_file.setdefault(file, []).append(mutant)
    return by_file


def usage(message: str) -> int:
    print(f"mutation-python: usage: {message}", file=sys.stderr)
    return EXIT_USAGE


def shard_of(text: str | None) -> tuple[int, int] | None | str:
    if text is None:
        return None
    found = re.fullmatch(r"(\d+)/(\d+)", text)
    if not found:
        return f"--shard {text!r} is not k/n"
    k, n = int(found[1]), int(found[2])
    if n < 1 or k >= n:
        return f"--shard {text} needs 0 <= k < n and n >= 1"
    return (k, n)


def prepare(args: argparse.Namespace) -> tuple[pathlib.Path, dict, dict] | int:
    """The root, the map and the selection, or the exit that refuses them."""
    chosen = [bool(args.plan), bool(args.all), bool(args.file)]
    if sum(chosen) != 1:
        return usage("exactly one of --plan, --all and --file")
    shard = shard_of(args.shard)
    if isinstance(shard, str):
        return usage(shard)
    args.shard = shard
    root = pathlib.Path(args.root).resolve()
    mapping, problems = census_problems(root)
    if problems:
        for problem in problems:
            print(f"mutation-python: REFUSED: {problem}", file=sys.stderr)
        return usage("the map is refused by the census")
    if args.file and args.file not in mapping:
        return usage(f"--file {args.file} is outside the population")
    try:
        return root, mapping, select(root, mapping, args)
    except (OSError, ValueError, KeyError) as error:
        return usage(f"the selection cannot be read: {error}")


def listing(args: argparse.Namespace) -> int:
    prepared = prepare(args)
    if isinstance(prepared, int):
        return prepared
    _root, _mapping, selected = prepared
    mutants = [
        {
            "name": m.name(file),
            "file": file,
            "line": m.line,
            "column": m.column,
            "start_line": m.start_line,
            "end_line": m.end_line,
            "end_column": m.end_column,
            "mutant": m.text,
            "operator": m.operator,
        }
        for file, group in selected.items()
        for m in group
    ]
    print(f"mutation-python: listed {len(mutants)}")
    for mutant in mutants:
        print(mutant["name"])
    if args.out:
        document = {"schema": SCHEMA, "listed": len(mutants), "mutants": mutants}
        pathlib.Path(args.out).write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    return EXIT_OK


def run(args: argparse.Namespace) -> int:
    prepared = prepare(args)
    if isinstance(prepared, int):
        return prepared
    root, mapping, selected = prepared
    try:
        dirty = mutation_rows.tracked_changes(root)
    except (subprocess.CalledProcessError, OSError) as error:
        return usage(f"the tree cannot be read by git: {error}")
    if dirty:
        return usage(f"the tree has tracked changes, so nothing was mutated: {', '.join(dirty)}")
    report: dict = {
        "schema": SCHEMA,
        "selection": "plan" if args.plan else "all" if args.all else "file",
        "shard": None if not args.shard else f"{args.shard[0]}/{args.shard[1]}",
        "failfast": bool(args.failfast),
        "files": [],
    }
    judge = Judge(root, mapping, args)
    code = None
    try:
        for file, mutants in selected.items():
            judge.file(file, mutants, report)
    except Restore as error:
        print(f"mutation-python: RESTORE FAILED: {error}")
        report["restore_failed"] = str(error)
        code = EXIT_RESTORE
    finally:
        shutil.rmtree(judge.scratch, ignore_errors=True)
    counts = count(report)
    report.update({"counts": counts})
    code = exit_code(report) if code is None else code
    report["exit"] = code
    if args.report:
        pathlib.Path(args.report).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for entry in report["files"]:
        if entry["void"]:
            print(f"mutation-python: VOID: {entry['path']}: {entry['void']}")
    for outcome in ("timeout", "void"):
        for entry in report["files"]:
            for mutant in entry["mutants"]:
                if mutant["outcome"] == outcome and not entry["void"]:
                    print(f"mutation-python: VOID: {outcome}: {mutant['name']}")
    print("mutation-python: " + ", ".join(f"{name} {counts[name]}" for name in OUTCOMES))
    print(f"examined {counts['examined']}")
    return code


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=["list", "run", "census", "tests"])
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--plan")
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--file")
    parser.add_argument("--shard")
    parser.add_argument("--failfast", action="store_true")
    parser.add_argument("--report")
    parser.add_argument("--out")
    parser.add_argument("--test-seconds", type=float)
    parser.add_argument("--control-seconds", type=float)
    parser.add_argument("--dir")
    parser.add_argument("--module", action="append", default=[])
    parser.add_argument("--skip", action="append", default=[])
    try:
        args = parser.parse_args(argv)
    except SystemExit as stop:
        return EXIT_USAGE if stop.code else EXIT_OK
    if args.verb == "tests":
        return child(args)
    if args.verb == "census":
        return census(pathlib.Path(args.root).resolve())
    return {"list": listing, "run": run}[args.verb](args)


if __name__ == "__main__":
    sys.exit(main())

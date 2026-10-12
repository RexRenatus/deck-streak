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
import collections
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
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
OUTCOMES = ("killed", "survived", "uncovered", "unviable", "timeout", "void")
EXIT_OK, EXIT_SURVIVED, EXIT_USAGE, EXIT_VOID, EXIT_RESTORE = 0, 1, 2, 3, 4
#: The leg control (SPEC-420 R9): the fixture whose three mutants every `run` judges first when
#: its root holds it, its checks, and the outcome each mutant must read, in listing order.
CONTROL = "scripts/tests/fixtures/mutation-python-control"
CONTROL_ENTRY = {"dir": CONTROL, "modules": ["control_checks"]}
CONTROL_EXPECTED = ("killed", "killed", "survived")
#: The `sys.monitoring` tool id a traced run takes, the coverage tool's, and its name (R7).
TRACE_TOOL, TRACE_NAME = 1, "mutation-python-trace"
#: What a traced run's processes inherit: the record directory, the target, and the test whose
#: process started them (`child_trace` spells the same names, since it imports nothing of ours).
TRACE_DIR, TRACE_TARGET = "MUTATION_PYTHON_TRACE_DIR", "MUTATION_PYTHON_TRACE_TARGET"
TRACE_TEST, TRACE_PARENT = "MUTATION_PYTHON_TRACE_TEST", "MUTATION_PYTHON_TRACE_PARENT"
#: The events that start a process, in the audit table of the interpreter CI runs (R7).
SPAWNS = frozenset({"os.fork", "os.forkpty", "os.spawn", "os.system"})
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

    def __init__(self, trace: Trace | None = None) -> None:
        super().__init__()
        self.failed: list[str] = []
        self.broken: set[str] = set()
        self.trace = trace

    def startTest(self, test):  # noqa: N802
        super().startTest(test)
        if self.trace is not None:
            self.trace.start(test.id())

    def stopTest(self, test):  # noqa: N802
        if self.trace is not None:
            self.trace.stop(test.id())
        super().stopTest(test)

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


def read_by(identifier: str, readers: list[str]) -> bool:
    """Whether a test is one of the byte readers, as `skipped` reads `--skip`."""
    return any(gone == identifier or gone.startswith(identifier + " ") for gone in readers)


def child_trace() -> None:
    """One child process's trace (SPEC-420 R7), the body of the scratch `sitecustomize` a traced
    run writes: the target's lines it executes, under the test whose process started it, how many
    processes it starts, and one record when it exits or replaces its image. It imports what it
    uses and fails closed: a process that writes no record leaves its parent's count short, so its
    test joins every scoped set of the file."""
    import atexit
    import json
    import os
    import sys

    folder = os.environ.get("MUTATION_PYTHON_TRACE_DIR")
    target = os.environ.get("MUTATION_PYTHON_TRACE_TARGET")
    monitoring = getattr(sys, "monitoring", None)
    if not folder or not target or monitoring is None:
        return
    record = {
        "token": os.urandom(8).hex(),
        "parent": os.environ.get("MUTATION_PYTHON_TRACE_PARENT"),
        "pid": os.getpid(),
        "ppid": os.getppid(),
        "test": os.environ.get("MUTATION_PYTHON_TRACE_TEST"),
        "lines": set(),
        "spawned": 0,
        "exec": False,
    }
    known, last, live = {}, [None], [True]

    def write():
        if live[0]:
            path = os.path.join(folder, record["token"] + ".json")
            with open(path + ".part", "w", encoding="utf-8") as sink:
                json.dump({**record, "lines": sorted(record["lines"])}, sink)
            os.replace(path + ".part", path)

    def line(code, number):
        name = code.co_filename
        if name not in known:
            known[name] = os.path.realpath(name) == target
        if known[name]:
            record["lines"].add(number)
        return monitoring.DISABLE

    def audit(event, args):
        if event == "subprocess.Popen":
            record["spawned"] += 1
            last[0] = args[1]
        elif event == "os.posix_spawn":
            record["spawned"] += args[1] is not last[0]
            last[0] = None
        elif event in ("os.fork", "os.forkpty", "os.spawn", "os.system"):
            record["spawned"] += 1
        elif event == "os.exec":
            record["exec"] = True
            write()

    def forked():
        live[0] = False

    try:
        monitoring.use_tool_id(1, "mutation-python-trace")
    except ValueError:
        return
    monitoring.register_callback(1, monitoring.events.LINE, line)
    monitoring.set_events(1, monitoring.events.LINE)
    sys.addaudithook(audit)
    os.environ["MUTATION_PYTHON_TRACE_PARENT"] = record["token"]
    os.register_at_fork(after_in_child=forked)
    atexit.register(write)


#: The tail of the scratch `sitecustomize`: the one it shadows on `sys.path`, run as the
#: interpreter would have run it.
SITE_CHAIN = """

def chain_site():
    import importlib.machinery
    import importlib.util
    import os
    import sys

    here = os.path.dirname(os.path.abspath(__file__))
    rest = [entry for entry in sys.path if os.path.abspath(entry or os.curdir) != here]
    spec = importlib.machinery.PathFinder.find_spec("sitecustomize", rest)
    if spec is not None and spec.loader is not None:
        spec.loader.exec_module(importlib.util.module_from_spec(spec))


chain_site()
"""


def site_source() -> str:
    """The scratch `sitecustomize`: `child_trace`'s own text and a call of it, then `SITE_CHAIN`."""
    text = pathlib.Path(__file__).read_text(encoding="utf-8")
    node = next(
        each
        for each in ast.parse(text).body
        if isinstance(each, ast.FunctionDef) and each.name == "child_trace"
    )
    return f"{ast.get_source_segment(text, node)}\n\n\nchild_trace()\n{SITE_CHAIN}"


class Trace:
    """The root process's trace of a sentinel run (SPEC-420 R7): each test's lines of the target
    through `sys.monitoring` `LINE` events, reset at its start and its stop, the lines run outside
    any test, the processes each test starts, the tests that leave a thread running, and one
    record when the run ends. Its processes inherit the scratch `sitecustomize`."""

    def __init__(self, folder: str, target: str) -> None:
        self.records = pathlib.Path(folder) / "records"
        self.records.mkdir(parents=True, exist_ok=True)
        self.target = os.path.realpath(target)
        self.token = os.urandom(8).hex()
        self.test: str | None = None
        self.order: list[str] = []
        self.lines: dict[str, set[int]] = {}
        self.outside: set[int] = set()
        self.spawned: collections.Counter = collections.Counter()
        self.execs: set[str] = set()
        self.threads: list[str] = []
        self.known: dict[str, bool] = {}
        self.before: set[threading.Thread] = set()
        self.last: object = None
        self.forked = False
        self.failed: str | None = None
        monitoring = getattr(sys, "monitoring", None)
        if monitoring is None:
            self.failed = "the interpreter has no sys.monitoring"
            return
        try:
            monitoring.use_tool_id(TRACE_TOOL, TRACE_NAME)
        except ValueError as error:
            self.failed = f"the monitoring tool cannot be taken: {error}"
            return
        monitoring.register_callback(TRACE_TOOL, monitoring.events.LINE, self.line)
        monitoring.set_events(TRACE_TOOL, monitoring.events.LINE)
        sys.addaudithook(self.audit)
        os.register_at_fork(after_in_child=self.fork)
        site = pathlib.Path(folder) / "site"
        site.mkdir(exist_ok=True)
        (site / "sitecustomize.py").write_text(site_source(), encoding="utf-8")
        paths = [str(site), *[each for each in [os.environ.get("PYTHONPATH")] if each]]
        os.environ["PYTHONPATH"] = os.pathsep.join(paths)
        os.environ[TRACE_DIR], os.environ[TRACE_TARGET] = str(self.records), self.target
        os.environ[TRACE_PARENT] = self.token

    def line(self, code, number):
        name = code.co_filename
        if name not in self.known:
            self.known[name] = os.path.realpath(name) == self.target
        if self.known[name]:
            if self.test is None:
                self.outside.add(number)
            else:
                self.lines.setdefault(self.test, set()).add(number)
        return sys.monitoring.DISABLE

    def audit(self, event, args):
        key = self.test or ""
        if event == "subprocess.Popen":
            self.spawned[key] += 1
            self.last = args[1]
        elif event == "os.posix_spawn":
            self.spawned[key] += args[1] is not self.last
            self.last = None
        elif event in SPAWNS:
            self.spawned[key] += 1
        elif event == "os.exec":
            self.execs.add(key)

    def fork(self) -> None:
        self.forked = True

    def restart(self) -> None:
        if self.failed is None:
            sys.monitoring.restart_events()

    def start(self, identifier: str) -> None:
        self.test = identifier
        self.order.append(identifier)
        os.environ[TRACE_TEST] = identifier
        self.before = set(threading.enumerate())
        self.restart()

    def stop(self, identifier: str) -> None:
        if any(each.is_alive() for each in threading.enumerate() if each not in self.before):
            self.threads.append(identifier)
        self.test = None
        os.environ.pop(TRACE_TEST, None)
        self.restart()

    def finish(self, ran: int) -> None:
        """The root record, unless this is a process a test forked from the root."""
        if self.forked:
            return
        if self.failed is None and sys.monitoring.get_tool(TRACE_TOOL) != TRACE_NAME:
            self.failed = "the monitoring tool was taken during the run"
        record = {
            "root": True,
            "token": self.token,
            "pid": os.getpid(),
            "failed": self.failed,
            "ran": ran,
            "order": self.order,
            "lines": {test: sorted(numbers) for test, numbers in self.lines.items()},
            "outside": sorted(self.outside),
            "spawned": dict(self.spawned),
            "execs": sorted(self.execs),
            "threads": self.threads,
        }
        part = self.records / "root.part"
        part.write_text(json.dumps(record), encoding="utf-8")
        part.replace(self.records / "root.json")


def child(args: argparse.Namespace) -> int:
    """Run the named test modules and print `{"ran": N, "failed": [...]}` as the last line."""
    copy = str(pathlib.Path(__file__).resolve().parent)
    here = pathlib.Path(copy)
    for name, module in list(sys.modules.items()):
        where = getattr(module, "__file__", None)
        if name != "__main__" and where and pathlib.Path(where).resolve().parent == here:
            del sys.modules[name]
    sys.path[:] = [entry for entry in sys.path if pathlib.Path(entry or ".").resolve() != here]
    # A caller's namespace that names no trace and no `--only` runs untraced and whole.
    traced = getattr(args, "trace", None)
    trace = Trace(traced, args.target) if traced else None
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    for module in args.module:
        suite.addTests(loader.discover(args.dir, pattern=f"{module}.py"))
    only = set(getattr(args, "only", ()))
    chosen = unittest.TestSuite(
        [t for t in flatten(suite) if not skipped(t, args.skip) and (not only or t.id() in only)]
    )
    result = Collect(trace)
    result.failfast = args.failfast
    real_stdout = sys.stdout
    sys.stdout = sys.stderr
    try:
        chosen.run(result)
    finally:
        sys.stdout = real_stdout
    if trace is not None:
        trace.finish(result.testsRun)
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


def mutant_record(path: str, m: Mutant, outcome: str, killers: list[str]) -> dict:
    """One mutant's entry in its file's record of the report."""
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


def scoped_sets(folder: pathlib.Path, ran: int) -> dict | str:
    """A sentinel run's trace, read once after its runner exits (SPEC-420 R7): each test's lines,
    the lines run outside any test, the tests that join every scoped set and the suite's order; or
    why the trace failed or is partial, so every mutant of the file runs whole. A process's direct
    children are the records naming its token and its pid. A process that started more processes
    than it has direct children, or one that replaced its image, joins its test to every set, as
    does a test that left a thread running; one outside any test makes the trace partial."""
    records = []
    for path in sorted(folder.glob("*.json")):
        try:
            records.append(json.loads(path.read_text(encoding="utf-8")))
        except (OSError, ValueError):
            return f"the trace record {path.name} cannot be read"
    root = next((record for record in records if record.get("root")), None)
    if root is None:
        return "the trace left no record of its run"
    if root["failed"]:
        return str(root["failed"])
    if root["ran"] != ran:
        return f"the trace counted {root['ran']} test(s) and the sentinel ran {ran}"
    lines = {test: set(numbers) for test, numbers in root["lines"].items()}
    outside = set(root["outside"])
    joiners = set(root["execs"]) | set(root["threads"])
    children = [record for record in records if record is not root]
    direct = collections.Counter((r["parent"], r["ppid"], r["test"] or "") for r in children)
    for record in children:
        test = record["test"]
        (outside if test is None else lines.setdefault(test, set())).update(record["lines"])
        if record["exec"]:
            joiners.add(test or "")
        started = sum(n for key, n in direct.items() if key[:2] == (record["token"], record["pid"]))
        if record["spawned"] > started:
            joiners.add(test or "")
    for test, spawned in root["spawned"].items():
        if spawned > direct[root["token"], root["pid"], test]:
            joiners.add(test)
    if "" in joiners:
        return "a process outside any test left no record or replaced its image"
    return {"order": root["order"], "lines": lines, "outside": outside, "joiners": joiners}


def scope_of(
    m: Mutant, scopes: dict | str | None, readers: list[str]
) -> tuple[str, str | None, list[str] | None]:
    """Where one mutant runs (SPEC-420 R7, R8): `lines`, with the tests whose processes executed
    its line in the suite's order and every test that joins every set, or `whole` and why. The
    scoped set is None when no trace was read for the mutant's line."""
    if scopes is None:
        return "whole", "all", None
    if isinstance(scopes, str):
        return "whole", "trace", None
    if m.function == "<module>":
        return "whole", "module", None
    span = range(m.start_line, m.end_line + 1)
    if any(number in scopes["outside"] for number in span):
        return "whole", "import", None
    kept = [test for test in scopes["order"] if not read_by(test, readers)]
    traced = {test for test in kept if any(n in scopes["lines"].get(test, ()) for n in span)}
    if not traced:
        return "whole", "unreached", []
    return "lines", None, [test for test in kept if test in traced or test in scopes["joiners"]]


def control_void(control: list[dict] | str) -> str | None:
    """The `void` a leg control that read otherwise than expected writes into every file entry
    (SPEC-420 R9), or None."""
    if control == "absent":
        return None
    if len(control) != len(CONTROL_EXPECTED):
        return (
            f"control: the control lists {len(control)} mutant(s), expected {len(CONTROL_EXPECTED)}"
        )
    for each in control:
        if each["outcome"] != each["expected"]:
            return f"control: {each['name']} read {each['outcome']}, expected {each['expected']}"
    return None


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
        # Each cost is written and flushed as it is paid, beside the report, so a leg its job cuts
        # short still leaves every cost it paid; the report and the output never change (R6).
        report = getattr(args, "report", None)
        self.costs = (
            open(pathlib.Path(report).parent / "costs.jsonl", "a", encoding="utf-8")
            if report
            else None
        )
        shard = getattr(args, "shard", None)
        self.where = {
            "shard": f"{shard[0]}/{shard[1]}" if shard else None,
            "run": os.environ.get("GITHUB_RUN_ID"),
            "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        }
        self.quiet = False

    def cost(self, kind: str, path: str, **fields: object) -> None:
        """One `costs.jsonl` line, flushed at once (SPEC-418 R6); nothing without `--report`."""
        if self.costs is None or self.quiet:
            return
        self.costs.write(json.dumps({"kind": kind, "path": path, **fields}) + "\n")
        self.costs.flush()

    def paid(self, path: str, record: dict) -> None:
        """The control line: the file's control run as used, its bound, and where it ran, the
        figures `mutation-verdict.py prices` derives the file's price from (SPEC-418 R6)."""
        self.cost("control", path, **record["control"], bound=record["bound"], **self.where)

    def close(self) -> None:
        if self.costs is not None:
            self.costs.close()

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

    def tests(
        self,
        entry: dict,
        skip: list[str],
        failfast: bool,
        bound: float,
        only: list[str] | None = None,
        trace: pathlib.Path | None = None,
        target: pathlib.Path | None = None,
    ) -> Ran:
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
        for identifier in only or []:
            command += ["--only", identifier]
        if trace is not None:
            command += ["--trace", str(trace), "--target", str(target)]
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
        state = self.prepare(path, selected, report)
        if state is not None:
            self.judge(state)

    def prepare(
        self, path: str, selected: list[Mutant], report: dict, entry: dict | None = None
    ) -> dict | None:
        """A file's control, bound and sentinel, and in a `--plan` run the sentinel's trace, its
        record written into `report` and its control's cost paid (SPEC-420 R6); the state its
        mutants are judged from, or None when the file is done. Raises `Restore`."""
        entry = entry or self.mapping[path]
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
        state = {"path": path, "entry": entry, "target": target, "original": original}
        state.update(source=source, record=record, selected=selected, scopes=None, notes=[])

        def all_as(outcome: str) -> None:
            record["mutants"] = [mutant_record(path, m, outcome, []) for m in selected]

        if not entry["modules"]:
            return state
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
            self.paid(path, record)
            all_as("void")
            return None
        bound = self.args.test_seconds or max(BOUND_FLOOR, BOUND_FACTOR * seconds)
        record["bound"] = bound
        self.paid(path, record)
        folder = (
            pathlib.Path(tempfile.mkdtemp(dir=self.scratch))
            if getattr(self.args, "plan", None)
            else None
        )
        try:
            target.write_bytes((source + SENTINEL).encode("utf-8"))
            if folder is None:
                sentinel = self.tests(entry, [], False, bound)
            else:
                sentinel = self.tests(entry, [], False, bound, trace=folder, target=target)
        finally:
            restore(target, original)
        if sentinel.timed_out or not sentinel.reported:
            record["void"] = "the sentinel run ran past its bound or printed no report"
            all_as("void")
            return None
        record["byte_readers"] = list(sentinel.failed)
        if sentinel.ran - sentinel.broken <= 0:
            record["void"] = "the sentinel left no test that does not read the file's bytes"
            all_as("void")
            return None
        if folder is not None:
            state["scopes"] = scoped_sets(folder / "records", sentinel.ran)
        return state

    def judge(self, state: dict) -> None:
        """A prepared file's mutants, each run against the tests its line's trace names or the
        whole suite (SPEC-420 R7, R8), each cost line naming its scope (R10); raises `Restore`."""
        path, entry, record, selected = (state[k] for k in ("path", "entry", "record", "selected"))
        if not entry["modules"]:
            record["mutants"] = [mutant_record(path, m, "uncovered", []) for m in selected]
            self.cost("uncovered", path, seconds=0, **self.where)
            return
        readers, bound, failfast = record["byte_readers"], record["bound"], self.args.failfast
        for m in selected:
            began = time.monotonic()
            scope, why, chosen = scope_of(m, state["scopes"], readers)
            counted = [0]

            def tests() -> Ran:
                nonlocal scope, why
                if scope == "lines":
                    ran = self.tests(entry, readers, failfast, bound, only=chosen)
                    if ran.timed_out or ran.reported and ran.failed:
                        counted[0] = ran.ran
                        return ran
                    whole = ran.reported and ran.ran >= len(chosen)
                    scope, why = "whole", "survivor" if whole else "ran"
                ran = self.tests(entry, readers, failfast, bound)
                counted[0] = ran.ran
                return ran

            outcome, killers = judge_mutant(
                state["target"], state["original"], m.apply(state["source"]), tests
            )
            record["mutants"].append(mutant_record(path, m, outcome, killers))
            note: dict = {"scope": scope, "tests": counted[0]}
            if scope == "whole":
                note["why"] = why
            if chosen is not None:
                note["scoped"] = chosen
            state["notes"].append({"name": m.name(path), "killers": killers, **note})
            self.cost(
                "mutant",
                path,
                name=m.name(path),
                seconds=time.monotonic() - began,
                outcome=outcome,
                **note,
            )

    def control(self) -> list[dict]:
        """The leg control (SPEC-420 R9): the fixture's mutants judged first, in the run's own
        mode, with no cost record and no entry in the report's files."""
        path = f"{CONTROL}/control_target.py"
        listed = list_source((self.root / path).read_text(encoding="utf-8"))
        held: dict = {"files": []}
        self.quiet = True
        try:
            state = self.prepare(path, listed, held, CONTROL_ENTRY)
            if state is not None:
                self.judge(state)
        finally:
            self.quiet = False
        notes = {note["name"]: note for note in (state or {"notes": []})["notes"]}
        found = []
        for index, mutant in enumerate(held["files"][0]["mutants"]):
            note = notes.get(mutant["name"], {})
            found.append(
                {
                    "name": mutant["name"],
                    "outcome": mutant["outcome"],
                    "expected": CONTROL_EXPECTED[index] if index < len(CONTROL_EXPECTED) else None,
                    "killers": mutant["killers"],
                    **{key: note[key] for key in ("scope", "why", "tests") if key in note},
                }
            )
        return found


class PlanRefused(Exception):
    """A plan whose Python legs this leg cannot run as dealt, refused before any mutant is
    installed and before any report is written (SPEC-418 R5)."""


def planned_leg(
    plan: dict | None, chosen: list[tuple[str, Mutant]], k: int, n: int
) -> list[tuple[str, Mutant]]:
    """Leg k of n of the chosen mutants, as the plan dealt them (SPEC-418 R5): the chosen whose
    names the plan's python shard k lists, in listing order, once the plan deals n legs that hold
    the runner's own listing exactly once each. A plan with no python matrix, `--all` and
    `--file` keep `k::n`."""
    python = plan.get("python") if isinstance(plan, dict) else None
    if not isinstance(python, dict):
        return chosen[k::n]
    if python.get("count") != n:
        raise PlanRefused(f"the plan deals {python.get('count')} python leg(s), not {n}")
    legs = [leg for leg in python.get("shards", []) if isinstance(leg, dict)]
    names = [mutant.name(file) for file, mutant in chosen]
    dealt = collections.Counter(str(name) for leg in legs for name in leg.get("mutants", []))
    listed = collections.Counter(names)
    if dealt != listed:
        missing, extra = listed - dealt, dealt - listed
        first = next((name for name in names if missing[name]), None)
        if first is None:
            plan_order = (str(name) for leg in legs for name in leg.get("mutants", []))
            first = next(name for name in plan_order if extra[name])
        raise PlanRefused(
            f"the plan's python legs do not partition the listing: {sum(missing.values())} "
            f"missing, {sum(extra.values())} extra, first {first}"
        )
    wanted = collections.Counter(
        str(name) for leg in legs if leg.get("shard") == k for name in leg.get("mutants", [])
    )
    ran: list[tuple[str, Mutant]] = []
    for (file, mutant), name in zip(chosen, names):
        if wanted[name] > 0:
            wanted[name] -= 1
            ran.append((file, mutant))
    return ran


def select(root: pathlib.Path, mapping: dict, args: argparse.Namespace) -> dict[str, list[Mutant]]:
    """The selected mutants by file, files in path order, after `--shard`."""
    plan = None
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
        chosen = planned_leg(plan, chosen, k, n)
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
    except PlanRefused as refusal:
        return usage(str(refusal))
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
        report["control"] = judge.control() if (root / CONTROL).is_dir() else "absent"
        prepared = [judge.prepare(file, mutants, report) for file, mutants in selected.items()]
        for state in prepared:
            if state is not None:
                judge.judge(state)
    except Restore as error:
        print(f"mutation-python: RESTORE FAILED: {error}")
        report["restore_failed"] = str(error)
        code = EXIT_RESTORE
    finally:
        judge.close()
        shutil.rmtree(judge.scratch, ignore_errors=True)
    void = control_void(report.get("control", "absent"))
    for entry in report["files"] if void else []:
        entry["void"] = void
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
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--trace")
    parser.add_argument("--target")
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

#!/usr/bin/env python3
"""mutation-probe.py -- judge a repository's mutation-testing practice, and phoenix-v2's row craft.

SPEC-V2-2208 / ADR-V2-2208. The owner's scope for `packs/mutation-rows`, 2026-09-27, was "Both,
one pack": phoenix-v2's row craft stays, and a portable mutation-testing practice joins it. The
severity rule is "Split by evidence". A class refuses only a state the repository's own tool
already refuses, or an invariant phoenix-v2 already enforces. Every judgement is advisory: a
score, a cadence, an equivalent-mutant note, a missing CI job.

It is VENDORABLE on purpose, as `stack-probe.py` is: standard library only (Python >= 3.11, for
`tomllib`), and every path it reads is under `--root`. A repository may copy it into its own tree
and run `walk` in its own CI. The pack's rows run it from here, as
`{skills}/../scripts/mutation-probe.py`, against any tree. The three `rows` classes read
phoenix-v2's row manifest through that tree's OWN population reader, `scripts/mutation_rows.py`,
and its own target resolver, `scripts/row_target.py`, so they hold no second reader of the rows.
On a tree without them they are VOID.

usage:
  mutation-probe.py [--root DIR] check CLASS
  mutation-probe.py [--root DIR] walk
  mutation-probe.py classes

The `practice` stage (portable; `walk` runs these twelve):
  tool-config-valid     block     every cargo-mutants, StrykerJS and mutmut configuration loads
                                  under its tool's own rules (an unknown cargo-mutants key, a
                                  Stryker threshold out of range or out of order, JSON with a
                                  comment, a JS config exporting a function or no default)
  tool-configured       advisory  every language surface (a Cargo workspace, a tested npm package,
                                  a Python project, a JVM build) has a mutation tool wired
  tool-config-current   advisory  no option its tool marks deprecated, renamed or likely invalid
  timeouts-bounded      advisory  a cargo-mutants CI run that skips the baseline sets --timeout,
                                  and none sets both timeout flags
  exclusions-justified  advisory  every exclusion carries its reason beside it
  ci-diff-run           advisory  CI runs each tool in use on the change under review
  ci-full-run           advisory  some CI run sweeps the whole tree with each tool in use
  shards-complete       advisory  a sharded run's matrix names every shard its denominator promises
  report-kept           advisory  every CI job that runs mutants keeps its report when it fails
  report-schema         advisory  every mutation report in the tree reads in its declared format
  survivors-triaged     advisory  no survivor is left unexamined in a report, and an ignore says why
  score-threshold       advisory  a score gate is declared where the tool has one, and a report
                                  meets its own low threshold

The `rows` stage (phoenix-v2's row manifest only):
  find-differs          block     every row's find is non-empty and differs from its replacement
  band-ids              block     the population assembles through its reader and holds no id twice
  mutants-distinct      advisory  no two rows install the same mutant for the same killer

Output is one line per finding, `CLASS: <path>: <what>`, then `examined N`, where N is the
population the class read. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: a class whose
population must not be empty examined nothing, or an input it needs could not be read, which is
never a pass. A class whose population may legitimately be empty (a tree may carry no report, no
exclusion, no sharded run) prints `examined 0` and passes. `walk` runs every practice class and
exits by the blocking ones alone: 1 when one found something, 3 when one was VOID, else 0.
"""

from __future__ import annotations

import argparse
import configparser
import importlib.util
import io
import json
import os
import posixpath
import re
import shlex
import sys
import tokenize
from collections import Counter, defaultdict
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11: `main` reports VOID, never a crash.
    tomllib = None

# A probe reads the tree it judges and writes nothing into it, bytecode included.
sys.dont_write_bytecode = True

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

# Directories no class reads: VCS state, dependencies, build output, vendored code, and the
# sandboxes the tools themselves copy the tree into.
PRUNE = frozenset(
    {
        ".git",
        ".hg",
        "node_modules",
        "target",
        "vendor",
        ".svelte-kit",
        ".stryker-tmp",
        ".venv",
        "venv",
        "__pycache__",
        ".sqlx-scratch",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        "mutants",
    }
)
# A tool's own output directory holds diffs of mutated source, never the tree's source.
OUTPUT_DIRS = frozenset({"mutants.out", "mutants.out.old"})
CI_DIRS = (".github/workflows", ".depot/workflows")

# --------------------------------------------------------------------------- tool knowledge

# cargo-mutants 27.1.0 (2026-06-02), `src/config.rs`: `#[serde(default, deny_unknown_fields)]`,
# so any other key refuses the whole file (`tests/main.rs::invalid_field_rejected`). `test_tool`
# and `sharding` are the flattened `Common` options (`src/options.rs`); `emit_diffs` there is
# `#[serde(skip)]` and so is not a config key. A key a later release adds is a finding here
# until this table learns it, which the table's version names.
CARGO_MUTANTS_VERSION = "27.1.0"
STRINGS, BOOL, NUMBER, STRING = "a list of strings", "a boolean", "a number", "a string"
CARGO_MUTANTS_KEYS: dict[str, object] = {
    "additional_cargo_args": STRINGS,
    "additional_cargo_test_args": STRINGS,
    "all_features": BOOL,
    "build_timeout_multiplier": NUMBER,
    "cap_lints": BOOL,
    "copy_vcs": BOOL,
    "copy_target": BOOL,
    "error_values": STRINGS,
    "examine_globs": STRINGS,
    "examine_re": STRINGS,
    "exclude_globs": STRINGS,
    "exclude_re": STRINGS,
    "gitignore": BOOL,
    "features": STRINGS,
    "minimum_test_timeout": NUMBER,
    "no_default_features": BOOL,
    "output": STRING,
    "profile": STRING,
    "skip_calls": STRINGS,
    "skip_calls_defaults": BOOL,
    "test_package": STRINGS,
    "test_workspace": BOOL,
    "timeout_multiplier": NUMBER,
    "test_tool": ("cargo", "nextest"),
    "sharding": ("slice", "round-robin"),
}
CARGO_MUTANTS_EXCLUSIONS = ("exclude_globs", "exclude_re", "skip_calls")

# StrykerJS 10.0.0 (2026-08-14), `packages/core/src/config/config-file-formats.ts`.
STRYKER_CONFIG_NAMES = frozenset(
    f"{prefix}stryker{suffix}.{extension}"
    for prefix in ("", ".")
    for suffix in (".conf", ".config")
    for extension in ("json", "js", "mjs", "cjs")
)
STRYKER_PACKAGE = "@stryker-mutator/core"
# `packages/api/schema/stryker-core.json`: `mutationScoreThresholds` has these three keys and no
# other (`additionalProperties: false`), each a percentage 0-100, `break` also null; the defaults
# are applied before `options-validator.ts` refuses `high < low` with a ConfigError.
STRYKER_THRESHOLD_DEFAULTS = {"high": 80, "low": 60, "break": None}
# `options-validator.ts::removeDeprecatedOptions` and `customValidation`: each is a DEPRECATED warning
# and a rewrite, never a refusal, so each is advisory here.
STRYKER_DEPRECATED = {
    "maxConcurrentTestRunners": "use concurrency",
    "files": "use ignorePatterns",
    "testFramework": "remove it: the test runner plugin handles its framework",
    "transpilers": "use buildCommand: transpilers support is removed",
}
TEST_FRAMEWORKS = frozenset(
    {"vitest", "jest", "mocha", "jasmine", "karma", "ava", "tap", "uvu"}
)

# mutmut 3.6.0 (2026-06-06) renamed and deprecated these; 3.8.0 is current (2026-09-12).
MUTMUT_RENAMED = {
    "paths_to_mutate": "renamed source_paths in mutmut 3.6.0",
    "tests_dir": "deprecated in mutmut 3.6.0 for pytest_add_cli_args_test_selection",
}
MUTMUT_PATTERN_KEYS = ("do_not_mutate", "only_mutate")
MUTMUT_EXCLUSIONS = ("do_not_mutate", "do_not_mutate_patterns")

# mutation-testing-report-schema 3.9.0 (`packages/report-schema/src/...schema.json`).
MTE_SCHEMA_VERSION = re.compile(r"[1-2](?:\.(?:[1-9][0-9]*|0)){0,2}")
MTE_STATUSES = frozenset(
    {
        "Killed",
        "Survived",
        "NoCoverage",
        "CompileError",
        "RuntimeError",
        "Timeout",
        "Ignored",
        "Pending",
    }
)
# cargo-mutants `src/outcome.rs::SummaryOutcome`, and the counts `LabOutcome` keeps beside them.
CARGO_SUMMARIES = {
    "Success": "success",
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Unviable": "unviable",
    "Timeout": "timeout",
    "Failure": None,
}
CARGO_COUNTS = ("missed", "caught", "timeout", "unviable", "success")

PULL_REQUEST_EVENTS = frozenset({"pull_request", "pull_request_target", "merge_group"})

# phoenix-v2's row manifest: where each table keeps its find, its replacement, and the crate its
# killer runs in (cell index, or None for the Python runner). The table decides the layout.
ROW_MANIFEST = "scripts/mutation-rows.json"
ROW_CELLS = {
    "MUTATIONS": (3, 4, 1),
    "CARGO_KILLED_SCRIPT_MUTATIONS": (2, 3, 4),
    "SCRIPT_MUTATIONS": (2, 3, None),
}
ROW_KILLER_CELL = 5


# --------------------------------------------------------------------------- outcomes


class Void(Exception):
    """An input the class needs cannot be read, so the class judged nothing."""


@dataclass(frozen=True)
class Outcome:
    """A class's findings and the size of the population it examined."""

    findings: list[str]
    examined: int


class Context:
    """The judged tree, walked once, with its files indexed by name and by suffix."""

    def __init__(self, root: Path, files: tuple[str, ...]) -> None:
        self.root = root
        self.files = files
        self.by_name: dict[str, list[str]] = defaultdict(list)
        self.by_suffix: dict[str, list[str]] = defaultdict(list)
        for path in files:
            pure = PurePosixPath(path)
            self.by_name[pure.name].append(path)
            self.by_suffix[pure.suffix].append(path)
        self.cache: dict[str, str] = {}

    def read(self, relative: str) -> str:
        if relative not in self.cache:
            try:
                self.cache[relative] = (self.root / relative).read_text(
                    encoding="utf-8"
                )
            except (OSError, UnicodeDecodeError) as error:
                raise Void(f"{relative} cannot be read: {error}") from error
        return self.cache[relative]

    def named(self, *names: str) -> list[str]:
        return sorted(path for name in names for path in self.by_name.get(name, ()))

    def suffixed(self, *suffixes: str) -> list[str]:
        return sorted(
            path for suffix in suffixes for path in self.by_suffix.get(suffix, ())
        )

    def exists(self, relative: str) -> bool:
        return (self.root / relative).is_file()


def walk_tree(root: Path) -> tuple[str, ...]:
    """Every file under `root` a class may read, tree-relative, in a stable order."""
    found: list[str] = []
    for here, directories, files in os.walk(root):
        directories[:] = sorted(
            name
            for name in directories
            if name not in PRUNE and not os.path.islink(os.path.join(here, name))
        )
        base = Path(here)
        for name in sorted(files):
            path = base / name
            if path.is_symlink():
                continue
            found.append(path.relative_to(root).as_posix())
    return tuple(found)


def is_source(path: str) -> bool:
    """A file the tree's own code lives in, never a tool's output copy of it."""
    return not (set(PurePosixPath(path).parts[:-1]) & OUTPUT_DIRS)


# --------------------------------------------------------------------------- lexers


def _blank(chars: list[str], start: int, end: int) -> None:
    for index in range(start, min(end, len(chars))):
        if chars[index] != "\n":
            chars[index] = " "


@dataclass(frozen=True)
class Lexed:
    """A source with its string literals and comments blanked, and where each comment sat."""

    code: str
    comments: tuple[tuple[int, str], ...]
    lines: tuple[str, ...] = field(default=(), init=False)
    commented: frozenset[int] = field(default=frozenset(), init=False)

    def __post_init__(self) -> None:
        object.__setattr__(self, "lines", tuple(self.code.split("\n")))
        object.__setattr__(self, "commented", frozenset(n for n, _ in self.comments))

    def code_line(self, number: int) -> str:
        return self.lines[number - 1] if 0 < number <= len(self.lines) else ""

    def comment_on(self, number: int) -> bool:
        return number in self.commented

    def comment_only(self, number: int) -> bool:
        return self.comment_on(number) and not self.code_line(number).strip()


RUST_TOKEN = re.compile(r"//|/\*|b?r#*\"|b\"|\"|'")
RUST_CHAR = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F]{1,6}\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")
JS_TOKEN = re.compile(r"//|/\*|[\"'`]")


def _comment_lines(comments: list[tuple[int, str]], line: int, text: str) -> None:
    for offset, piece in enumerate(text.split("\n")):
        comments.append((line + offset, piece))


def lex_rust(text: str) -> Lexed:
    """Rust with comments (nested block comments too), strings, raw strings and chars blanked.

    A lifetime (`'a`) is not a char literal and stays; `r#"..."#` closes only at its own hash count.
    """
    chars = list(text)
    comments: list[tuple[int, str]] = []
    index, line, counted = 0, 1, 0
    while True:
        token = RUST_TOKEN.search(text, index)
        if token is None:
            break
        start = token.start()
        line += text.count("\n", counted, start)
        counted = start
        word = token.group()
        before = text[start - 1] if start else ""
        if word[0] in "br" and (before.isalnum() or before == "_"):
            index = start + 1
            continue
        if word == "//":
            end = text.find("\n", start)
            end = len(text) if end < 0 else end
            comments.append((line, text[start:end]))
        elif word == "/*":
            depth, end = 1, start + 2
            while end < len(text) and depth:
                nested = re.compile(r"/\*|\*/").search(text, end)
                if nested is None:
                    end = len(text)
                    break
                depth += 1 if nested.group() == "/*" else -1
                end = nested.end()
            _comment_lines(comments, line, text[start:end])
        elif word == "'":
            literal = RUST_CHAR.match(text, start)
            if literal is None:
                index = start + 1
                continue
            end = literal.end()
        elif word.endswith('"') and "r" in word:
            close = '"' + word[word.index("r") + 1 : -1]
            end = text.find(close, token.end())
            end = len(text) if end < 0 else end + len(close)
        else:
            end = token.end()
            while end < len(text) and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            end += 1
        _blank(chars, start, end)
        index = max(end, start + 1)
    return Lexed("".join(chars), tuple(comments))


def lex_js(text: str) -> Lexed:
    """JavaScript or TypeScript with comments, strings and template literals blanked.

    A regular-expression literal is read as code; only `//` and `/*` open a comment.
    """
    chars = list(text)
    comments: list[tuple[int, str]] = []
    index, line, counted = 0, 1, 0
    while True:
        token = JS_TOKEN.search(text, index)
        if token is None:
            break
        start = token.start()
        line += text.count("\n", counted, start)
        counted = start
        word = token.group()
        if word == "//":
            end = text.find("\n", start)
            end = len(text) if end < 0 else end
            comments.append((line, text[start:end]))
        elif word == "/*":
            end = text.find("*/", start + 2)
            end = len(text) if end < 0 else end + 2
            _comment_lines(comments, line, text[start:end])
        elif word == "`":
            end, depth = start + 1, 0
            while end < len(text):
                if text[end] == "\\":
                    end += 2
                    continue
                if depth == 0 and text[end] == "`":
                    break
                if text.startswith("${", end):
                    depth, end = depth + 1, end + 2
                    continue
                if depth and text[end] == "}":
                    depth -= 1
                end += 1
            end += 1
        else:
            end = start + 1
            while end < len(text) and text[end] not in (word, "\n"):
                end += 2 if text[end] == "\\" else 1
            end += 1
        _blank(chars, start, end)
        index = max(end, start + 1)
    return Lexed("".join(chars), tuple(comments))


# --------------------------------------------------------------------------- a YAML subset


def _strip_comment(text: str) -> str:
    """`text` without a trailing ` # comment` that sits outside quotes."""
    quote = ""
    for index, char in enumerate(text):
        if quote:
            if char == quote:
                quote = ""
            continue
        if char in "'\"":
            quote = char
        elif char == "#" and (index == 0 or text[index - 1] in " \t"):
            return text[:index].rstrip()
    return text


def _split_key(content: str) -> tuple[str, str] | None:
    """A mapping entry's key and the text after its colon, or None when `content` is no entry."""
    if content[:1] in "\"'":
        quote = content[0]
        end = content.find(quote, 1)
        if end > 0 and content[end + 1 : end + 2] == ":":
            rest = content[end + 2 :]
            if not rest or rest[0] in " \t":
                return content[1:end], rest
        return None
    if content[:1] in "[{&*!|>%@`":
        return None
    for index, char in enumerate(content):
        if char == "#" and index and content[index - 1] in " \t":
            return None
        if char == ":" and (index + 1 == len(content) or content[index + 1] in " \t"):
            key = content[:index].strip()
            return (key, content[index + 1 :]) if key else None
    return None


def _unquote(text: str) -> str:
    text = text.strip()
    if len(text) >= 2 and text[0] == text[-1] == '"':
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return text[1:-1]
    if len(text) >= 2 and text[0] == text[-1] == "'":
        return text[1:-1].replace("''", "'")
    return text


def _flow_items(text: str) -> list[object]:
    """The items of a one-level YAML flow sequence `[a, "b", [c]]`."""
    inner = text.strip()[1:-1]
    items: list[object] = []
    depth, quote, start = 0, "", 0
    for index, char in enumerate(inner + ","):
        if quote:
            if char == quote:
                quote = ""
            continue
        if char in "'\"":
            quote = char
        elif char in "[{":
            depth += 1
        elif char in "]}":
            depth -= 1
        elif char == "," and depth == 0:
            piece = inner[start:index].strip()
            if piece:
                items.append(
                    _flow_items(piece) if piece.startswith("[") else _unquote(piece)
                )
            start = index + 1
    return items


class YamlSubset:
    """The block YAML a CI workflow is written in: mappings, sequences, block and flow scalars.

    Anchors, aliases, tags and flow mappings are kept as text; every scalar is a string. It never
    raises: a line it cannot place is skipped, which reads as an absent key, never as an invented one.
    """

    BLOCK_SCALAR = re.compile(r"[|>][+-]?[0-9]?|[|>][0-9][+-]?")

    def __init__(self, text: str) -> None:
        self.lines = text.replace("\t", "    ").split("\n")

    @staticmethod
    def indent(line: str) -> int:
        return len(line) - len(line.lstrip(" "))

    def structural(self, index: int) -> int:
        while index < len(self.lines):
            stripped = self.lines[index].strip()
            if (
                stripped
                and not stripped.startswith("#")
                and stripped not in ("---", "...")
            ):
                return index
            index += 1
        return index

    @staticmethod
    def is_item(content: str) -> bool:
        return content == "-" or content.startswith("- ")

    def parse(self) -> object:
        index = self.structural(0)
        if index >= len(self.lines):
            return None
        value, _ = self.block(index, self.indent(self.lines[index]))
        return value

    def block(self, index: int, indent: int) -> tuple[object, int]:
        if self.is_item(self.lines[index][indent:]):
            return self.sequence(index, indent)
        return self.mapping(index, indent)

    def mapping(self, index: int, indent: int) -> tuple[dict, int]:
        result: dict[str, object] = {}
        while True:
            index = self.structural(index)
            if index >= len(self.lines):
                return result, index
            line = self.lines[index]
            depth = self.indent(line)
            if depth < indent:
                return result, index
            if depth > indent:
                index += 1
                continue
            content = line[indent:]
            if self.is_item(content):
                return result, index
            entry = _split_key(content)
            if entry is None:
                index += 1
                continue
            key, rest = entry
            value, index = self.value(rest, index, indent)
            result[key] = value

    def sequence(self, index: int, indent: int) -> tuple[list, int]:
        items: list[object] = []
        while True:
            index = self.structural(index)
            if index >= len(self.lines):
                return items, index
            line = self.lines[index]
            if self.indent(line) != indent or not self.is_item(line[indent:]):
                if self.indent(line) > indent:
                    index += 1
                    continue
                return items, index
            after = line[indent + 1 :]
            text = after.lstrip(" ")
            column = indent + 1 + len(after) - len(text)
            if not text or text.startswith("#"):
                following = self.structural(index + 1)
                if (
                    following < len(self.lines)
                    and self.indent(self.lines[following]) > indent
                ):
                    value, index = self.block(
                        following, self.indent(self.lines[following])
                    )
                else:
                    value, index = None, index + 1
                items.append(value)
                continue
            if _split_key(text) is not None:
                self.lines[index] = " " * column + text
                value, index = self.mapping(index, column)
                items.append(value)
                continue
            value, index = self.value(text, index, indent)
            items.append(value)

    def value(self, rest: str, index: int, indent: int) -> tuple[object, int]:
        text = _strip_comment(rest).strip()
        if self.BLOCK_SCALAR.fullmatch(text):
            return self.block_scalar(text, index, indent)
        if not text:
            following = self.structural(index + 1)
            if following < len(self.lines):
                depth = self.indent(self.lines[following])
                content = self.lines[following][depth:]
                if depth > indent or (depth == indent and self.is_item(content)):
                    return self.block(following, depth)
            return None, index + 1
        if text[0] in "[{":
            opener, closer = text[0], "]" if text[0] == "[" else "}"
            while text.count(opener) > text.count(closer) and index + 1 < len(
                self.lines
            ):
                index += 1
                text += " " + _strip_comment(self.lines[index]).strip()
            return (_flow_items(text) if opener == "[" else text), index + 1
        if text[0] in "\"'" and not (len(text) > 1 and text.endswith(text[0])):
            quote = text[0]
            while index + 1 < len(self.lines) and not text.endswith(quote):
                index += 1
                text += " " + self.lines[index].strip()
            return _unquote(text), index + 1
        following = index + 1
        if text[0] not in "\"'":
            while following < len(self.lines):
                line = self.lines[following]
                stripped = line.strip()
                if (
                    not stripped
                    or stripped.startswith("#")
                    or self.indent(line) <= indent
                ):
                    break
                text += " " + _strip_comment(stripped).strip()
                following += 1
        return _unquote(text), following

    def block_scalar(self, indicator: str, index: int, indent: int) -> tuple[str, int]:
        following = index + 1
        body: list[str] = []
        content_indent: int | None = None
        explicit = re.search(r"[0-9]", indicator)
        if explicit:
            content_indent = indent + int(explicit.group())
        while following < len(self.lines):
            line = self.lines[following]
            if not line.strip():
                body.append("")
                following += 1
                continue
            depth = self.indent(line)
            if content_indent is None:
                if depth <= indent:
                    break
                content_indent = depth
            if depth < content_indent:
                break
            body.append(line[content_indent:])
            following += 1
        while body and not body[-1]:
            body.pop()
        if indicator.startswith(">"):
            folded: list[str] = []
            for piece in body:
                if piece and folded and folded[-1]:
                    folded[-1] += " " + piece
                else:
                    folded.append(piece)
            return "\n".join(folded), following
        return "\n".join(body), following


# --------------------------------------------------------------------------- CI workflows


@dataclass
class Step:
    name: str
    run: str
    uses: str
    condition: str
    with_path: str


@dataclass
class Job:
    name: str
    steps: list[Step]
    matrix: object
    uses: str


@dataclass
class Workflow:
    path: str
    triggers: set[str]
    jobs: list[Job]


def _text(value: object) -> str:
    return value if isinstance(value, str) else ""


def parse_workflow(path: str, text: str) -> Workflow:
    data = YamlSubset(text).parse()
    data = data if isinstance(data, dict) else {}
    on = data.get("on", data.get("true"))
    if isinstance(on, str):
        triggers = {on}
    elif isinstance(on, list):
        triggers = {item for item in on if isinstance(item, str)}
    elif isinstance(on, dict):
        triggers = set(on)
    else:
        triggers = set()
    jobs: list[Job] = []
    raw_jobs = data.get("jobs")
    for name, job in raw_jobs.items() if isinstance(raw_jobs, dict) else ():
        job = job if isinstance(job, dict) else {}
        steps = []
        for step in job.get("steps") or ():
            step = step if isinstance(step, dict) else {}
            with_block = step.get("with") if isinstance(step.get("with"), dict) else {}
            steps.append(
                Step(
                    name=_text(step.get("name")),
                    run=_text(step.get("run")),
                    uses=_text(step.get("uses")),
                    condition=_text(step.get("if")),
                    with_path=_text(with_block.get("path")),
                )
            )
        strategy = job.get("strategy") if isinstance(job.get("strategy"), dict) else {}
        jobs.append(
            Job(
                name=name,
                steps=steps,
                matrix=strategy.get("matrix"),
                uses=_text(job.get("uses")),
            )
        )
    return Workflow(path=path, triggers=triggers, jobs=jobs)


def load_workflows(context: Context) -> list[Workflow]:
    workflows = []
    for path in context.files:
        parent = PurePosixPath(path).parent.as_posix()
        if parent in CI_DIRS and PurePosixPath(path).suffix in (".yml", ".yaml"):
            workflows.append(parse_workflow(path, context.read(path)))
    callers: dict[str, set[str]] = defaultdict(set)
    for workflow in workflows:
        for job in workflow.jobs:
            if job.uses.startswith("./"):
                callers[job.uses[2:].split("@", 1)[0]].add(workflow.path)
    by_path = {workflow.path: workflow for workflow in workflows}
    for _ in range(4):
        for workflow in workflows:
            if "workflow_call" in workflow.triggers:
                for caller in callers.get(workflow.path, ()):
                    workflow.triggers |= by_path[caller].triggers - {"workflow_call"}
    return workflows


# --------------------------------------------------------------------------- shell commands

EXPRESSION = re.compile(r"\$\{\{(.*?)\}\}", re.S)
ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")
PUNCTUATION = set(";&|()<>")
PREFIXES = frozenset({"env", "time", "sudo", "exec", "command", "nice", "nohup"})
NPX = frozenset({"npx", "bunx", "pnpx"})
RUNNERS = frozenset({"pnpm", "yarn", "npm", "bun"})
RUNNER_VERBS = frozenset({"exec", "dlx", "x"})
RUNNER_BUILTINS = frozenset(
    {
        "install",
        "i",
        "ci",
        "add",
        "remove",
        "update",
        "up",
        "rebuild",
        "audit",
        "config",
        "init",
    }
)
# pnpm `-C`/`--dir`/`--filter`, npm `--prefix`/`--workspace`, yarn `--cwd`: each takes a value.
RUNNER_VALUE_FLAGS = frozenset(
    {"-C", "--dir", "--filter", "-F", "--prefix", "--workspace", "-w", "--cwd"}
)


def simple_commands(text: str) -> list[list[str]]:
    """The simple commands of a shell `run:` text, each as its words, comments dropped.

    An `${{ ... }}` expression is one word: its spaces, quotes and the characters the shell splits
    on (`||` in `${{ a || b }}`) are folded to `_` first, so no expression cuts a command in two.
    """
    text = EXPRESSION.sub(
        lambda match: (
            "${{" + re.sub(r"[\s;&|()<>'\"`]+", "_", match.group(1).strip()) + "}}"
        ),
        text,
    )
    text = re.sub(r"\\\n", " ", text)
    commands: list[list[str]] = []
    for line in text.split("\n"):
        try:
            lexer = shlex.shlex(line, posix=True, punctuation_chars=True)
            lexer.whitespace_split = True
            words = list(lexer)
        except ValueError:
            words = _strip_comment(line).split()
        current: list[str] = []
        for word in words:
            if word and set(word) <= PUNCTUATION:
                if current:
                    commands.append(current)
                current = []
                continue
            current.append(word)
        if current:
            commands.append(current)
    return commands


def unwrap(words: list[str]) -> list[str]:
    """`words` with its assignments and launchers (`env`, `npx`, `pnpm exec`, `python -m`) removed."""
    rest = list(words)
    while rest:
        head = rest[0]
        if ASSIGNMENT.match(head):
            rest.pop(0)
        elif head in PREFIXES:
            rest.pop(0)
        elif head in NPX:
            rest.pop(0)
            while rest and rest[0].startswith("-"):
                rest.pop(0)
        elif head in RUNNERS and len(rest) > 1 and rest[1] in RUNNER_VERBS:
            rest = rest[2:]
            while rest and rest[0].startswith("-"):
                rest.pop(0)
        elif head in ("uv", "poetry", "pdm", "hatch", "pipenv") and rest[1:2] == [
            "run"
        ]:
            rest = rest[2:]
        elif PurePosixPath(head).name in ("python", "python3") and rest[1:2] == ["-m"]:
            rest = rest[2:]
        else:
            break
    return rest


def has_flag(args: list[str], flag: str) -> bool:
    return any(arg == flag or arg.startswith(flag + "=") for arg in args)


def flag_value(args: list[str], flag: str) -> str | None:
    for index, arg in enumerate(args):
        if arg.startswith(flag + "="):
            return arg[len(flag) + 1 :]
        if arg == flag and index + 1 < len(args):
            return args[index + 1]
    return None


@dataclass
class Invocation:
    """One CI run of a mutation tool: which tool, over what scope, from which step."""

    tool: str
    scope: str
    args: list[str]
    workflow: Workflow
    job: Job


def classify(words: list[str]) -> tuple[str, str, list[str]] | None:
    """(tool, scope, arguments) for a simple command that runs a mutation tool, else None."""
    words = unwrap(words)
    if not words:
        return None
    program = PurePosixPath(words[0]).name
    args = words[1:]
    if program == "cargo":
        rest = [arg for arg in args if not arg.startswith("+")]
        while rest and rest[0].startswith("-"):
            rest.pop(0)
        if rest[:1] == ["mutants"]:
            args = rest[1:]
            program = "cargo-mutants"
    elif program == "cargo-mutants":
        args = args[1:] if args[:1] == ["mutants"] else args
    if program == "cargo-mutants":
        inspecting = (
            "--list",
            "--list-files",
            "--check",
            "--version",
            "--help",
            "-h",
            "--emit-schema",
        )
        if any(has_flag(args, flag) for flag in inspecting):
            return ("cargo-mutants", "none", args)
        return (
            "cargo-mutants",
            "diff" if has_flag(args, "--in-diff") else "whole",
            args,
        )
    if program == "stryker":
        if args[:1] != ["run"]:
            return None
        args = args[1:]
        if has_flag(args, "--dryRunOnly"):
            return ("stryker", "none", args)
        scope = (
            "diff"
            if has_flag(args, "--incremental") and not has_flag(args, "--force")
            else "whole"
        )
        return ("stryker", scope, args)
    if program == "mutmut":
        return ("mutmut", "whole", args[1:]) if args[:1] == ["run"] else None
    if program == "cosmic-ray":
        return ("cosmic-ray", "whole", args[1:]) if args[:1] == ["exec"] else None
    if program == "cr-rate":
        return ("cosmic-ray", "gate", args) if has_flag(args, "--fail-over") else None
    if program in ("mvn", "mvnw"):
        goals = " ".join(args)
        if re.search(r"pitest(?:-maven)?:scmMutationCoverage", goals):
            return ("pit", "diff", args)
        if re.search(r"pitest(?:-maven)?:mutationCoverage", goals):
            return ("pit", "whole", args)
        return None
    if program in ("gradle", "gradlew"):
        return ("pit", "whole", args) if "pitest" in args else None
    if flag_value(words, "--bin") == "mutants-diff" or program == "mutants-diff":
        tail = words[words.index("--") + 1 :] if "--" in words else args
        inspecting = ("--list", "--select-only", "--prove-row", "--help", "-h")
        if any(has_flag(tail, flag) for flag in inspecting):
            return ("phoenix-rows", "none", tail)
        return ("phoenix-rows", "diff", tail)
    if flag_value(words, "--bin") in ("mutation-rc1", "mutation-rc2", "mutation-rc3"):
        return ("phoenix-rows", "whole", args)
    scripted = [PurePosixPath(word).name for word in words[:2]]
    if "check.sh" in scripted and "mutation" in (
        flag_value(args, "--only") or ""
    ).split(","):
        return ("phoenix-rows", "whole", args)
    if "mutation-battery.py" in scripted:
        return ("phoenix-rows", "whole", args)
    return None


def package_scripts(context: Context) -> dict[str, list[str]]:
    """Every npm script name in the tree, with each command text that name runs."""
    scripts: dict[str, list[str]] = defaultdict(list)
    for path in context.named("package.json"):
        try:
            package = json.loads(context.read(path))
        except json.JSONDecodeError:
            continue
        declared = package.get("scripts") if isinstance(package, dict) else None
        for name, command in declared.items() if isinstance(declared, dict) else ():
            if isinstance(command, str):
                scripts[name].append(command)
    return scripts


def script_name(words: list[str]) -> str | None:
    """The npm script a `pnpm test:mutation` / `npm run x` / `yarn x` command runs, or None.

    A runner flag that takes a value (`pnpm -C web x`, `npm --prefix web run x`) is read with it,
    so its value is never taken for the script's name.
    """
    if not words or words[0] not in RUNNERS:
        return None
    rest: list[str] = []
    skip = False
    for word in words[1:]:
        if skip:
            skip = False
        elif word in RUNNER_VALUE_FLAGS:
            skip = True
        elif not word.startswith("-"):
            rest.append(word)
    if not rest:
        return None
    if rest[0] in ("run", "run-script"):
        return rest[1] if len(rest) > 1 else None
    if words[0] == "npm":
        return "test" if rest[0] in ("test", "t") else None
    return None if rest[0] in RUNNER_BUILTINS or rest[0] in RUNNER_VERBS else rest[0]


def invocations(context: Context, workflows: list[Workflow]) -> list[Invocation]:
    """Every mutation-tool run the workflows make, npm scripts followed two levels deep."""
    scripts = package_scripts(context)
    found: list[Invocation] = []

    def visit(words: list[str], workflow: Workflow, job: Job, depth: int) -> None:
        classified = classify(words)
        if classified is not None:
            tool, scope, args = classified
            found.append(Invocation(tool, scope, args, workflow, job))
            return
        name = script_name(unwrap(words))
        if name is None or depth >= 2:
            return
        for command in scripts.get(name, ()):
            for inner in simple_commands(command):
                visit(inner, workflow, job, depth + 1)

    for workflow in workflows:
        for job in workflow.jobs:
            for step in job.steps:
                for words in simple_commands(step.run):
                    visit(words, workflow, job, 0)
    return found


# --------------------------------------------------------------------------- tools in use


def toml_document(context: Context, path: str) -> dict | None:
    """A TOML file's document, or None when it does not parse."""
    try:
        return tomllib.loads(context.read(path))
    except tomllib.TOMLDecodeError:
        return None


def ends_with_parts(path: str, *parts: str) -> bool:
    """Whether `path`'s last components are `parts`, compared whole, never as a string suffix.

    `vendor.cargo/mutants.toml` ends with the TEXT `.cargo/mutants.toml`, and is no cargo-mutants
    configuration; `myreports/mutation/mutation.json` is no Stryker report.
    """
    return PurePosixPath(path).parts[-len(parts) :] == parts


def cargo_mutants_configs(context: Context) -> list[str]:
    return [
        path
        for path in context.named("mutants.toml")
        if ends_with_parts(path, ".cargo", "mutants.toml")
    ]


def stryker_configs(context: Context) -> list[str]:
    return [path for path in context.named(*STRYKER_CONFIG_NAMES) if is_source(path)]


def mutmut_configs(context: Context) -> list[tuple[str, dict]]:
    """(path, its mutmut table) for every pyproject.toml and setup.cfg that configures mutmut."""
    found: list[tuple[str, dict]] = []
    for path in context.named("pyproject.toml"):
        document = toml_document(context, path)
        table = ((document or {}).get("tool") or {}).get("mutmut")
        if isinstance(table, dict):
            found.append((path, table))
    for path in context.named("setup.cfg"):
        parser = configparser.ConfigParser(interpolation=None)
        try:
            parser.read_string(context.read(path))
        except configparser.Error:
            continue
        if parser.has_section("mutmut"):
            found.append((path, dict(parser.items("mutmut"))))
    return found


def cosmic_ray_configs(context: Context) -> list[str]:
    return [
        path
        for path in context.suffixed(".toml")
        if is_source(path)
        and isinstance((toml_document(context, path) or {}).get("cosmic-ray"), dict)
    ]


def pit_builds(context: Context) -> list[str]:
    return [
        path
        for path in context.named("pom.xml", "build.gradle", "build.gradle.kts")
        if re.search(r"pitest-maven|info\.solidsoft\.pitest", context.read(path))
    ]


def npm_packages(context: Context) -> list[tuple[str, dict]]:
    packages = []
    for path in context.named("package.json"):
        try:
            package = json.loads(context.read(path))
        except json.JSONDecodeError:
            continue
        if isinstance(package, dict):
            packages.append((path, package))
    return packages


def dependencies(package: dict) -> set[str]:
    names: set[str] = set()
    for table in (
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ):
        if isinstance(package.get(table), dict):
            names |= set(package[table])
    return names


def tools_in_use(context: Context, found: list[Invocation]) -> list[str]:
    tools = {invocation.tool for invocation in found if invocation.scope != "none"}
    if cargo_mutants_configs(context):
        tools.add("cargo-mutants")
    if stryker_configs(context) or any(
        STRYKER_PACKAGE in dependencies(p) for _, p in npm_packages(context)
    ):
        tools.add("stryker")
    if mutmut_configs(context):
        tools.add("mutmut")
    if cosmic_ray_configs(context):
        tools.add("cosmic-ray")
    if pit_builds(context):
        tools.add("pit")
    if context.exists(ROW_MANIFEST):
        tools.add("phoenix-rows")
    order = ("cargo-mutants", "stryker", "mutmut", "cosmic-ray", "pit", "phoenix-rows")
    return [tool for tool in order if tool in tools]


# --------------------------------------------------------------------------- practice classes


def _number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _integer(value: object) -> bool:
    return _number(value) and float(value).is_integer()


def _reject_constant(name: str) -> object:
    raise ValueError(f"{name} is not JSON")


def strict_json(text: str) -> object:
    """JSON as `JSON.parse` reads it: no comment, no trailing comma, no NaN or Infinity."""
    return json.loads(text, parse_constant=_reject_constant)


def js_module_kind(context: Context, path: str) -> str:
    """`esm` or `cjs`, by extension, then by the nearest package.json's `type`."""
    suffix = PurePosixPath(path).suffix
    if suffix == ".mjs":
        return "esm"
    if suffix == ".cjs":
        return "cjs"
    directory = PurePosixPath(path).parent
    while True:
        candidate = (directory / "package.json").as_posix()
        if context.exists(candidate):
            try:
                package = json.loads(context.read(candidate))
            except json.JSONDecodeError:
                return "cjs"
            return (
                "esm"
                if isinstance(package, dict) and package.get("type") == "module"
                else "cjs"
            )
        if directory.as_posix() in (".", ""):
            return "cjs"
        directory = directory.parent


FUNCTION_VALUE = r"(?:async\s+)?(?:function\b|\([^()]*\)\s*=>|[A-Za-z_$][\w$]*\s*=>)"
EXPORTS_FUNCTION = re.compile(
    r"\bmodule\s*\.\s*exports\s*=\s*"
    + FUNCTION_VALUE
    + r"|\bexport\s+default\s+"
    + FUNCTION_VALUE
)
ESM_DEFAULT = re.compile(r"\bexport\s+default\b|\bexport\s*\{[^}]*\bas\s+default\b")


def stryker_thresholds(thresholds: object, where: str) -> list[str]:
    if not isinstance(thresholds, dict):
        return [f"{where}: thresholds is not an object (StrykerJS's schema refuses it)"]
    findings = [
        f"{where}: thresholds.{key} is not a threshold StrykerJS knows (high, low, break)"
        for key in thresholds
        if key not in STRYKER_THRESHOLD_DEFAULTS
    ]
    merged = dict(STRYKER_THRESHOLD_DEFAULTS, **thresholds)
    for key in ("high", "low", "break"):
        value = merged.get(key)
        if key == "break" and value is None:
            continue
        if not _number(value) or not 0 <= value <= 100:
            findings.append(
                f"{where}: thresholds.{key} is {value!r}, not a percentage from 0 to 100"
            )
    high, low = merged.get("high"), merged.get("low")
    if _number(high) and _number(low) and high < low:
        findings.append(
            f"{where}: thresholds.high {high} is below thresholds.low {low}, read through the "
            "defaults high 80 and low 60 (StrykerJS refuses it with a ConfigError)"
        )
    return findings


def check_tool_config_valid(context: Context) -> Outcome:
    """`tool-config-valid`: every mutation-tool configuration loads under its tool's own rules."""
    findings: list[str] = []
    examined = 0
    for path in cargo_mutants_configs(context):
        examined += 1
        text = context.read(path)
        try:
            document = tomllib.loads(text)
        except tomllib.TOMLDecodeError as error:
            findings.append(
                f"{path}: does not parse as TOML ({error}); cargo-mutants refuses it"
            )
            continue
        for key, value in document.items():
            expected = CARGO_MUTANTS_KEYS.get(key)
            if expected is None:
                findings.append(
                    f"{path}: unknown key {key}: cargo-mutants {CARGO_MUTANTS_VERSION} reads its "
                    "config with deny_unknown_fields and refuses the file"
                )
            elif isinstance(expected, tuple):
                if value not in expected:
                    findings.append(
                        f"{path}: {key} is {value!r}, not one of {', '.join(expected)}"
                    )
            elif expected == STRINGS and not (
                isinstance(value, list) and all(isinstance(item, str) for item in value)
            ):
                findings.append(
                    f"{path}: {key} must be {STRINGS}, found {type(value).__name__}"
                )
            elif expected == BOOL and not isinstance(value, bool):
                findings.append(
                    f"{path}: {key} must be {BOOL}, found {type(value).__name__}"
                )
            elif expected == NUMBER and not _number(value):
                findings.append(
                    f"{path}: {key} must be {NUMBER}, found {type(value).__name__}"
                )
            elif expected == STRING and not isinstance(value, str):
                findings.append(
                    f"{path}: {key} must be {STRING}, found {type(value).__name__}"
                )
    for path in stryker_configs(context):
        examined += 1
        text = context.read(path)
        if path.endswith(".json"):
            try:
                config = strict_json(text)
            except ValueError as error:
                findings.append(
                    f"{path}: is not valid JSON ({error}); StrykerJS reads it with JSON.parse"
                )
                continue
            if not isinstance(config, dict):
                findings.append(f"{path}: is not a JSON object; StrykerJS refuses it")
                continue
            if "thresholds" in config:
                findings.extend(stryker_thresholds(config["thresholds"], path))
            coverage = config.get("coverageAnalysis", "perTest")
            if config.get("ignoreStatic") is True and coverage != "perTest":
                findings.append(
                    f"{path}: ignoreStatic needs coverageAnalysis perTest, found {coverage!r} "
                    "(StrykerJS refuses it with a ConfigError)"
                )
            continue
        code = lex_js(text).code
        if EXPORTS_FUNCTION.search(code):
            findings.append(
                f"{path}: exports a function; StrykerJS refuses a config whose default export is "
                "a function (removed in 6.0.0, 2022-05-03)"
            )
        elif js_module_kind(context, path) == "esm" and not ESM_DEFAULT.search(code):
            findings.append(
                f"{path}: has no default export, which StrykerJS requires of a JS config"
            )
    for path in context.named("pyproject.toml"):
        text = context.read(path)
        if "[tool.mutmut]" in text:
            examined += 1
            try:
                tomllib.loads(text)
            except tomllib.TOMLDecodeError as error:
                findings.append(
                    f"{path}: does not parse as TOML ({error}); mutmut cannot read it"
                )
    return Outcome(findings, examined)


def check_tool_configured(context: Context) -> Outcome:
    """`tool-configured`: every language surface of the tree has a mutation tool wired."""
    workflows = load_workflows(context)
    found = invocations(context, workflows)
    ran = {invocation.tool for invocation in found if invocation.scope != "none"}
    findings: list[str] = []
    examined = 0
    manifests = sorted(
        context.named("Cargo.toml"), key=lambda path: (path.count("/"), path)
    )
    if manifests:
        examined += 1
        if not (
            cargo_mutants_configs(context)
            or "cargo-mutants" in ran
            or context.exists(ROW_MANIFEST)
        ):
            findings.append(
                f"{manifests[0]}: a Rust workspace with no cargo-mutants configuration, no CI run of "
                "cargo mutants and no phoenix-v2 row manifest"
            )
    for path, package in npm_packages(context):
        scripts = (
            package.get("scripts") if isinstance(package.get("scripts"), dict) else {}
        )
        test = scripts.get("test")
        # Tested when it depends on a test framework or its own test script runs one; a root
        # script that only delegates (`pnpm -r test`) leaves the testing to the packages it names.
        runs = (
            {PurePosixPath(word).name for word in test.split()}
            if isinstance(test, str)
            else set()
        )
        tested = bool((dependencies(package) | runs) & TEST_FRAMEWORKS)
        if not tested:
            continue
        examined += 1
        directory = PurePosixPath(path).parent
        configured = any(
            context.exists((directory / name).as_posix())
            for name in STRYKER_CONFIG_NAMES
        )
        if not (configured or STRYKER_PACKAGE in dependencies(package)):
            findings.append(
                f"{path}: a tested package with no Stryker configuration and no {STRYKER_PACKAGE} "
                "dependency"
            )
    python = {}
    for path in context.named("pyproject.toml", "setup.cfg", "setup.py"):
        python.setdefault(PurePosixPath(path).parent.as_posix(), path)
    configured_python = {
        PurePosixPath(path).parent.as_posix() for path, _ in mutmut_configs(context)
    }
    python_ci = bool({"mutmut", "cosmic-ray"} & ran) or bool(
        cosmic_ray_configs(context)
    )
    for directory, path in sorted(python.items()):
        examined += 1
        if directory not in configured_python and not python_ci:
            findings.append(
                f"{path}: a Python project with no mutmut or cosmic-ray configuration or CI run"
            )
    for path in context.named("pom.xml", "build.gradle", "build.gradle.kts"):
        examined += 1
        if (
            not re.search(r"pitest-maven|info\.solidsoft\.pitest", context.read(path))
            and "pit" not in ran
        ):
            findings.append(f"{path}: a JVM build with no PIT plugin")
    if examined == 0:
        raise Void(
            "the tree has no Cargo workspace, tested npm package, Python project or JVM build"
        )
    return Outcome(findings, examined)


def check_tool_config_current(context: Context) -> Outcome:
    """`tool-config-current`: no option its tool marks deprecated, renamed or likely invalid."""
    findings: list[str] = []
    examined = 0
    for path in stryker_configs(context):
        examined += 1
        text = context.read(path)
        if path.endswith(".json"):
            try:
                config = strict_json(text)
            except ValueError:
                continue
            if not isinstance(config, dict):
                continue
            for key, advice in STRYKER_DEPRECATED.items():
                if key in config:
                    findings.append(
                        f"{path}: {key} is deprecated in StrykerJS: {advice}"
                    )
            mutator = config.get("mutator")
            if isinstance(mutator, str):
                findings.append(
                    f"{path}: mutator as a string is deprecated in StrykerJS: remove it"
                )
            elif isinstance(mutator, dict) and "name" in mutator:
                findings.append(
                    f"{path}: mutator.name is deprecated in StrykerJS: remove it"
                )
            html = config.get("htmlReporter")
            if isinstance(html, dict) and "baseDir" in html:
                findings.append(
                    f"{path}: htmlReporter.baseDir is deprecated in StrykerJS: use htmlReporter.fileName"
                )
            jest = config.get("jest")
            if isinstance(jest, dict) and "enableBail" in jest:
                findings.append(
                    f"{path}: jest.enableBail is deprecated in StrykerJS: use disableBail"
                )
            continue
        code = lex_js(text).code
        for key in ("maxConcurrentTestRunners", "testFramework", "transpilers"):
            if re.search(rf"\b{key}\s*:", code):
                findings.append(
                    f"{path}: {key} is deprecated in StrykerJS: {STRYKER_DEPRECATED[key]}"
                )
    for path, table in mutmut_configs(context):
        examined += 1
        for key, advice in MUTMUT_RENAMED.items():
            if key in table:
                findings.append(f"{path}: {key} is {advice}")
        for key in MUTMUT_PATTERN_KEYS:
            value = table.get(key)
            if isinstance(value, str):
                value = (
                    [piece for piece in value.split("\n") if piece]
                    if "\n" in value
                    else [value]
                )
            for pattern in value if isinstance(value, list) else ():
                if isinstance(pattern, str) and not (
                    pattern.endswith("*") or pattern.endswith(".py")
                ):
                    findings.append(
                        f"{path}: {key} pattern {pattern!r} is one mutmut warns is likely invalid: it "
                        "expects a glob ending in * or .py"
                    )
    for path in pit_builds(context):
        examined += 1
        if "maxMutationsPerClass" in context.read(path):
            findings.append(
                f"{path}: maxMutationsPerClass is deprecated in PIT: use the feature +CLASSLIMIT(limit[n])"
            )
    return Outcome(findings, examined)


def check_timeouts_bounded(context: Context) -> Outcome:
    """`timeouts-bounded`: a CI run that skips the baseline sets its own timeout."""
    findings: list[str] = []
    runs = [
        invocation
        for invocation in invocations(context, load_workflows(context))
        if invocation.tool == "cargo-mutants" and invocation.scope != "none"
    ]
    for run in runs:
        where = f"{run.workflow.path}: job {run.job.name}"
        timeout = has_flag(run.args, "--timeout")
        multiplier = has_flag(run.args, "--timeout-multiplier")
        skipped = flag_value(run.args, "--baseline") == "skip"
        in_place = has_flag(run.args, "--in-place")
        if skipped and not timeout:
            detail = (
                " (--timeout-multiplier has no baseline to multiply)"
                if multiplier
                else ""
            )
            findings.append(
                f"{where}: cargo mutants skips the baseline and sets no --timeout{detail}, so a "
                "mutant that hangs holds the job to cargo-mutants' 300 s default"
            )
        elif in_place and multiplier and not timeout:
            findings.append(
                f"{where}: cargo mutants runs --in-place, where cargo-mutants ignores "
                "--timeout-multiplier, and sets no --timeout, so the test timeout is its 300 s "
                "default"
            )
        if timeout and multiplier:
            findings.append(
                f"{where}: cargo mutants sets --timeout and --timeout-multiplier, which conflict "
                "(cargo-mutants 24.5.0)"
            )
    return Outcome(findings, len(runs))


@dataclass(frozen=True)
class Site:
    """One exclusion, where it is and whether a reason sits beside it."""

    where: str
    what: str
    justified: bool


def toml_array_sites(path: str, text: str, keys: Iterable[str]) -> list[Site]:
    """Every string entry of each `key = [ ... ]` array, with the line's own reason."""
    lines = text.split("\n")
    sites: list[Site] = []
    for key in keys:
        for start, raw in enumerate(lines):
            opener = re.match(rf"\s*{re.escape(key)}\s*=\s*\[", raw)
            if not opener:
                continue
            depth, quote, number = 0, "", start
            column = opener.end() - 1
            while number < len(lines):
                line = lines[number]
                entries: list[str] = []
                comment = False
                current = ""
                index = column if number == start else 0
                while index < len(line):
                    char = line[index]
                    if quote:
                        if char == "\\" and quote == '"':
                            current += line[index : index + 2]
                            index += 2
                            continue
                        if char == quote:
                            entries.append(current)
                            quote, current = "", ""
                        else:
                            current += char
                    elif char in "'\"":
                        quote = char
                    elif char == "#":
                        comment = True
                        break
                    elif char == "[":
                        depth += 1
                    elif char == "]":
                        depth -= 1
                        if depth == 0:
                            break
                    index += 1
                above = lines[number - 1].strip() if number else ""
                for entry in entries:
                    sites.append(
                        Site(
                            f"{path}:{number + 1}",
                            f"{key} entry {entry!r}",
                            comment or above.startswith("#"),
                        )
                    )
                if (
                    depth == 0
                    and number >= start
                    and "]" in line[column if number == start else 0 :]
                ):
                    break
                number += 1
    return sites


ATTRIBUTE = re.compile(r"\bmutants\s*::\s*(skip|exclude_re)\b")
# StrykerJS's own directive grammar (`instrumenter/src/transformers/directive-bookkeeper.ts`),
# matched against a comment's value, the text inside its delimiters: prose that merely mentions a
# disable comment is no directive. The fourth group is the reason the report carries.
STRYKER_DIRECTIVE = re.compile(
    r"\s?Stryker (disable|restore)(?: (next-line))? ([a-zA-Z, ]+)(?::(.+)?)?"
)
STRYKER_DISABLE = re.compile(r"Stryker disable")
PRAGMA = re.compile(r"#\s*pragma:\s*no\s+mutate\b(?P<rest>.*)")
JS_SUFFIXES = (
    ".js",
    ".jsx",
    ".ts",
    ".tsx",
    ".mjs",
    ".cjs",
    ".mts",
    ".cts",
    ".svelte",
    ".vue",
)


def rust_sites(path: str, text: str) -> list[Site]:
    lexed = lex_rust(text)
    sites = []
    for number, line in enumerate(lexed.code.split("\n"), start=1):
        for match in ATTRIBUTE.finditer(line):
            if not re.search(r"#!?\[", line[: match.start()]):
                continue
            above = number - 1
            while above > 0 and re.fullmatch(
                r"\s*#!?\[.*\]\s*", lexed.code_line(above)
            ):
                above -= 1
            justified = lexed.comment_on(number) or (
                above > 0 and lexed.comment_only(above)
            )
            sites.append(
                Site(f"{path}:{number}", f"#[mutants::{match.group(1)}]", justified)
            )
    return sites


def js_sites(path: str, text: str) -> list[Site]:
    sites = []
    for number, comment in lex_js(text).comments:
        if not comment.startswith(("//", "/*")):
            continue
        value = comment[2:]
        if comment.startswith("/*") and value.endswith("*/"):
            value = value[:-2]
        directive = STRYKER_DIRECTIVE.match(value)
        if directive is None or directive.group(1) != "disable":
            continue
        reason = (directive.group(4) or "").strip()
        sites.append(
            Site(f"{path}:{number}", "a Stryker disable comment", bool(reason))
        )
    return sites


def python_sites(path: str, text: str) -> list[Site]:
    try:
        tokens = list(tokenize.generate_tokens(io.StringIO(text).readline))
    except (tokenize.TokenError, SyntaxError):
        return []
    code_lines = {
        token.start[0]
        for token in tokens
        if token.type
        not in (
            tokenize.COMMENT,
            tokenize.NL,
            tokenize.NEWLINE,
            tokenize.INDENT,
            tokenize.DEDENT,
            tokenize.ENDMARKER,
        )
    }
    comment_lines = {
        token.start[0] for token in tokens if token.type == tokenize.COMMENT
    }
    sites = []
    for token in tokens:
        if token.type != tokenize.COMMENT:
            continue
        match = PRAGMA.search(token.string)
        if not match:
            continue
        rest = match.group("rest").strip()
        keyword = re.match(r"(block|start|end)\b", rest)
        if keyword and keyword.group(1) == "end":
            continue
        reason = rest[keyword.end() :] if keyword else rest
        number = token.start[0]
        above = number - 1
        justified = bool(re.search(r"\w", reason)) or (
            above in comment_lines and above not in code_lines
        )
        sites.append(Site(f"{path}:{number}", "a # pragma: no mutate", justified))
    return sites


def check_exclusions_justified(context: Context) -> Outcome:
    """`exclusions-justified`: every exclusion carries its reason beside it."""
    sites: list[Site] = []
    for path in cargo_mutants_configs(context):
        sites.extend(
            toml_array_sites(path, context.read(path), CARGO_MUTANTS_EXCLUSIONS)
        )
    for path in context.named("pyproject.toml"):
        text = context.read(path)
        if "[tool.mutmut]" in text:
            sites.extend(toml_array_sites(path, text, MUTMUT_EXCLUSIONS))
    # A file whose text never names the marker holds no site, so only those that do are lexed.
    readers = {".rs": (ATTRIBUTE, rust_sites), ".py": (PRAGMA, python_sites)}
    readers.update({suffix: (STRYKER_DISABLE, js_sites) for suffix in JS_SUFFIXES})
    for suffix, (marker, sites_of) in readers.items():
        for path in context.suffixed(suffix):
            if not is_source(path):
                continue
            text = context.read(path)
            if marker.search(text):
                sites.extend(sites_of(path, text))
    findings = [
        f"{site.where}: {site.what} carries no reason on its line or the line above"
        for site in sites
        if not site.justified
    ]
    return Outcome(findings, len(sites))


def check_ci_diff_run(context: Context) -> Outcome:
    """`ci-diff-run`: CI runs each tool in use on the change under review."""
    workflows = load_workflows(context)
    found = invocations(context, workflows)
    tools = tools_in_use(context, found)
    if not tools:
        raise Void("no mutation tool is in use in the tree")
    diff_modes = {
        "cargo-mutants": "cargo mutants --in-diff",
        "stryker": "stryker run --incremental",
        "pit": "pitest:scmMutationCoverage",
        "phoenix-rows": "mutants-diff",
    }
    findings = []
    for tool in tools:
        on_review = [
            invocation
            for invocation in found
            if invocation.tool == tool
            and invocation.workflow.triggers & PULL_REQUEST_EVENTS
        ]
        if tool in diff_modes:
            if not any(invocation.scope == "diff" for invocation in on_review):
                findings.append(
                    f"{tool}: no workflow triggered by a pull request runs {diff_modes[tool]}"
                )
        elif not any(invocation.scope in ("diff", "whole") for invocation in on_review):
            findings.append(f"{tool}: no workflow triggered by a pull request runs it")
    return Outcome(findings, len(tools))


def check_ci_full_run(context: Context) -> Outcome:
    """`ci-full-run`: some CI run sweeps the whole tree with each tool in use."""
    workflows = load_workflows(context)
    found = invocations(context, workflows)
    tools = tools_in_use(context, found)
    if not tools:
        raise Void("no mutation tool is in use in the tree")
    findings = [
        f"{tool}: no workflow runs it over the whole tree; a diff run cannot see a region the "
        "change left less tested, or a change to tests alone"
        for tool in tools
        if not any(
            invocation.tool == tool and invocation.scope == "whole"
            for invocation in found
        )
    ]
    return Outcome(findings, len(tools))


def matrix_values(job: Job, name: str) -> list[str] | None:
    """The values `matrix.<name>` takes in `job`, or None when they are not written out."""
    matrix = job.matrix
    if not isinstance(matrix, dict) or not isinstance(matrix.get(name), list):
        return None
    for extra in ("include", "exclude"):
        if matrix.get(extra) is not None:
            return None
    values = matrix[name]
    return (
        [str(value) for value in values]
        if all(isinstance(value, str) for value in values)
        else None
    )


def check_shards_complete(context: Context) -> Outcome:
    """`shards-complete`: a sharded run's matrix names every shard its denominator promises."""
    findings: list[str] = []
    examined = 0
    literal: dict[tuple[str, str, int], set[int]] = defaultdict(set)
    for run in invocations(context, load_workflows(context)):
        spec = flag_value(run.args, "--shard")
        if run.scope == "none" or spec is None or "/" not in spec:
            continue
        index, total = spec.rsplit("/", 1)
        if not total.isdigit():
            continue
        count = int(total)
        expected = set(range(count))
        matrix = re.fullmatch(r"\$\{\{matrix\.([\w-]+)\}\}", index)
        if index.isdigit():
            literal[(run.workflow.path, run.tool, count)].add(int(index))
            continue
        if matrix is None:
            continue
        values = matrix_values(run.job, matrix.group(1))
        if values is None or not all(value.isdigit() for value in values):
            continue
        examined += 1
        named = {int(value) for value in values}
        if named != expected:
            findings.append(
                f"{run.workflow.path}: job {run.job.name} runs --shard k/{count} over matrix."
                f"{matrix.group(1)} {sorted(named)}; shards {sorted(expected - named)} never run"
                + (
                    f" and {sorted(named - expected)} are out of range"
                    if named - expected
                    else ""
                )
            )
    for (path, tool, count), named in sorted(literal.items()):
        examined += 1
        expected = set(range(count))
        if named != expected:
            findings.append(
                f"{path}: {tool} --shard k/{count} runs shards {sorted(named)}; shards "
                f"{sorted(expected - named)} never run"
            )
    return Outcome(findings, examined)


KEEPS = re.compile(r"\b(always|failure)\s*\(\s*\)|!\s*cancelled\s*\(\s*\)")


def check_report_kept(context: Context) -> Outcome:
    """`report-kept`: every CI job that runs mutants keeps its report when the run fails."""
    jobs: dict[tuple[str, str], tuple[Job, set[str]]] = {}
    for run in invocations(context, load_workflows(context)):
        if run.scope in ("diff", "whole"):
            key = (run.workflow.path, run.job.name)
            jobs.setdefault(key, (run.job, set()))[1].add(run.tool)
    findings = []
    for (path, name), (job, tools) in sorted(jobs.items()):
        kept = any(
            step.uses.startswith("actions/upload-artifact@")
            and KEEPS.search(step.condition)
            for step in job.steps
        )
        if not kept:
            findings.append(
                f"{path}: job {name} runs {', '.join(sorted(tools))} and keeps no report when it "
                "fails: no actions/upload-artifact step under if: always()"
            )
    return Outcome(findings, len(jobs))


# --------------------------------------------------------------------------- reports


def mte_report_paths(context: Context) -> list[str]:
    paths = {
        path
        for path in context.named("mutation.json")
        if ends_with_parts(path, "reports", "mutation", "mutation.json")
    }
    for config in stryker_configs(context):
        if not config.endswith(".json"):
            continue
        try:
            reporter = (strict_json(context.read(config)) or {}).get("jsonReporter")
        except (ValueError, AttributeError):
            continue
        name = reporter.get("fileName") if isinstance(reporter, dict) else None
        if isinstance(name, str):
            candidate = posixpath.normpath(
                (PurePosixPath(config).parent / name).as_posix()
            )
            if not candidate.startswith("..") and context.exists(candidate):
                paths.add(candidate)
    return sorted(paths)


def cargo_outcome_paths(context: Context) -> list[str]:
    return [
        path
        for path in context.named("outcomes.json")
        if ends_with_parts(path, "mutants.out", "outcomes.json")
    ]


def _position(value: object, where: str) -> list[str]:
    if not isinstance(value, dict):
        return [f"{where} is not a position object"]
    return [
        f"{where}.{key} is {value.get(key)!r}, not an integer of at least 1"
        for key in ("line", "column")
        if not (_integer(value.get(key)) and value.get(key) >= 1)
    ]


def mte_defects(report: object) -> list[str]:
    """Where `report` departs from mutation-testing-report-schema 3.9.0's required structure."""
    if not isinstance(report, dict):
        return ["the report is not a JSON object"]
    defects = []
    version = report.get("schemaVersion")
    if not (isinstance(version, str) and MTE_SCHEMA_VERSION.fullmatch(version)):
        defects.append(f"schemaVersion is {version!r}, not a major version 1 or 2")
    thresholds = report.get("thresholds")
    if not isinstance(thresholds, dict):
        defects.append("thresholds is missing or not an object")
    else:
        for key in ("high", "low"):
            value = thresholds.get(key)
            if not (_integer(value) and 0 <= value <= 100):
                defects.append(
                    f"thresholds.{key} is {value!r}, not an integer from 0 to 100"
                )
    files = report.get("files")
    if not isinstance(files, dict):
        return defects + ["files is missing or not an object keyed by path"]
    for name, result in files.items():
        if not isinstance(result, dict):
            defects.append(f"files[{name!r}] is not an object")
            continue
        for key, kind in (("language", str), ("source", str), ("mutants", list)):
            if not isinstance(result.get(key), kind):
                defects.append(
                    f"files[{name!r}].{key} is missing or not a {kind.__name__}"
                )
        for position, item in enumerate(result.get("mutants") or ()):
            where = f"files[{name!r}].mutants[{position}]"
            if not isinstance(item, dict):
                defects.append(f"{where} is not an object")
                continue
            for key in ("id", "mutatorName"):
                if not isinstance(item.get(key), str):
                    defects.append(f"{where}.{key} is missing or not a string")
            if item.get("status") not in MTE_STATUSES:
                defects.append(
                    f"{where}.status {item.get('status')!r} is not a mutant state"
                )
            location = item.get("location")
            if not isinstance(location, dict):
                defects.append(f"{where}.location is missing")
            else:
                for end in ("start", "end"):
                    defects.extend(
                        _position(location.get(end), f"{where}.location.{end}")
                    )
            if "statusReason" in item and not isinstance(item["statusReason"], str):
                defects.append(f"{where}.statusReason is not a string")
    return defects


def outcomes_defects(outcomes: object) -> list[str]:
    """Where a cargo-mutants `outcomes.json` departs from what `LabOutcome` writes."""
    if not isinstance(outcomes, dict):
        return ["the file is not a JSON object"]
    listed = outcomes.get("outcomes")
    if not isinstance(listed, list):
        return ["outcomes is missing or not a list"]
    defects = []
    tally: Counter[str] = Counter()
    mutants = 0
    for position, outcome in enumerate(listed):
        summary = outcome.get("summary") if isinstance(outcome, dict) else None
        if summary not in CARGO_SUMMARIES:
            defects.append(
                f"outcomes[{position}].summary {summary!r} is not a cargo-mutants outcome"
            )
            continue
        if (
            isinstance(outcome.get("scenario"), dict)
            and "Mutant" in outcome["scenario"]
        ):
            mutants += 1
            if CARGO_SUMMARIES[summary]:
                tally[CARGO_SUMMARIES[summary]] += 1
        elif outcome.get("scenario") != "Baseline":
            defects.append(
                f"outcomes[{position}].scenario is neither Baseline nor a Mutant"
            )
    total = outcomes.get("total_mutants")
    if total != mutants:
        defects.append(
            f"total_mutants is {total!r} but {mutants} mutant outcomes are listed"
        )
    for key in CARGO_COUNTS:
        if outcomes.get(key) != tally[key]:
            defects.append(
                f"{key} is {outcomes.get(key)!r} but {tally[key]} outcomes say so"
            )
    return defects


def read_report(context: Context, path: str) -> object | None:
    try:
        return strict_json(context.read(path))
    except ValueError:
        return None


def check_report_schema(context: Context) -> Outcome:
    """`report-schema`: every mutation report in the tree reads in its declared format."""
    findings: list[str] = []
    reports = mte_report_paths(context)
    outcomes = cargo_outcome_paths(context)
    for path in reports:
        report = read_report(context, path)
        defects = ["is not valid JSON"] if report is None else mte_defects(report)
        findings.extend(
            f"{path}: {defect} (mutation-testing-report-schema)" for defect in defects
        )
    for path in outcomes:
        document = read_report(context, path)
        defects = (
            ["is not valid JSON"] if document is None else outcomes_defects(document)
        )
        findings.extend(
            f"{path}: {defect} (cargo-mutants outcomes.json)" for defect in defects
        )
    return Outcome(findings, len(reports) + len(outcomes))


def mte_mutants(report: object) -> list[tuple[str, dict]]:
    """(file, mutant) for every mutant a readable report holds."""
    files = report.get("files") if isinstance(report, dict) else None
    found = []
    for name, result in files.items() if isinstance(files, dict) else ():
        for item in (result.get("mutants") if isinstance(result, dict) else None) or ():
            if isinstance(item, dict):
                found.append((name, item))
    return found


def _line(item: dict) -> str:
    start = (
        (item.get("location") or {}).get("start")
        if isinstance(item.get("location"), dict)
        else None
    )
    return str(start.get("line")) if isinstance(start, dict) else "?"


def check_survivors_triaged(context: Context) -> Outcome:
    """`survivors-triaged`: no survivor is left unexamined in a report, and an ignore says why."""
    findings: list[str] = []
    examined = 0
    for path in mte_report_paths(context):
        report = read_report(context, path)
        if not isinstance(report, dict):
            continue
        examined += 1
        for name, item in mte_mutants(report):
            status = item.get("status")
            where = f"{path}: {name}:{_line(item)} {item.get('mutatorName')} {status}"
            if status in ("Survived", "NoCoverage"):
                findings.append(
                    f"{where}: add a test that observes it, or record it as equivalent with a reason"
                )
            elif status == "Pending":
                findings.append(
                    f"{where}: the run did not finish, so the mutant was never judged"
                )
            elif (
                status == "Ignored" and not str(item.get("statusReason") or "").strip()
            ):
                findings.append(f"{where}: an ignored mutant carries no reason")
    for path in cargo_outcome_paths(context):
        document = read_report(context, path)
        if not isinstance(document, dict) or not isinstance(
            document.get("outcomes"), list
        ):
            continue
        examined += 1
        for outcome in document["outcomes"]:
            if (
                not isinstance(outcome, dict)
                or outcome.get("summary") != "MissedMutant"
            ):
                continue
            scenario = (
                outcome.get("scenario")
                if isinstance(outcome.get("scenario"), dict)
                else {}
            )
            detail = (
                scenario.get("Mutant")
                if isinstance(scenario.get("Mutant"), dict)
                else {}
            )
            name = (
                detail.get("name")
                or f"{detail.get('file')} {detail.get('replacement')}"
            )
            findings.append(
                f"{path}: {name}: missed; add a test that observes it, or skip it with a reason"
            )
    return Outcome(findings, examined)


def js_break(code: str) -> str | None:
    """The `break` a JS config's `thresholds: {...}` literal sets: a number, `null`, or None if absent."""
    opener = re.search(r"\bthresholds\s*:\s*\{", code)
    if not opener:
        return None
    depth, index = 1, opener.end()
    while index < len(code) and depth:
        depth += {"{": 1, "}": -1}.get(code[index], 0)
        index += 1
    value = re.search(
        r"\bbreak\s*:\s*(null|[0-9]+(?:\.[0-9]+)?)", code[opener.end() : index]
    )
    return value.group(1) if value else None


def check_score_threshold(context: Context) -> Outcome:
    """`score-threshold`: a score gate is declared where the tool has one, and a report meets it."""
    findings: list[str] = []
    examined = 0
    for path in stryker_configs(context):
        examined += 1
        text = context.read(path)
        if path.endswith(".json"):
            try:
                config = strict_json(text)
            except ValueError:
                continue
            thresholds = config.get("thresholds") if isinstance(config, dict) else None
            breaks = thresholds.get("break") if isinstance(thresholds, dict) else None
            declared = _number(breaks)
        else:
            breaks = js_break(lex_js(text).code)
            declared = breaks is not None and breaks != "null"
        if not declared:
            findings.append(
                f"{path}: sets no thresholds.break, so a run passes at any score (StrykerJS exits 1 "
                "below break)"
            )
    gated = any(
        invocation.tool == "cosmic-ray" and invocation.scope == "gate"
        for invocation in invocations(context, load_workflows(context))
    )
    for path in cosmic_ray_configs(context):
        examined += 1
        if not gated:
            findings.append(
                f"{path}: cosmic-ray has no score gate: no CI step runs cr-rate --fail-over"
            )
    for path in pit_builds(context):
        examined += 1
        if "mutationThreshold" not in context.read(path):
            findings.append(
                f"{path}: PIT sets no mutationThreshold, so a build passes at any score"
            )
    for path in mte_report_paths(context):
        report = read_report(context, path)
        if not isinstance(report, dict):
            continue
        examined += 1
        states = Counter(item.get("status") for _, item in mte_mutants(report))
        detected = states["Killed"] + states["Timeout"]
        valid = detected + states["Survived"] + states["NoCoverage"]
        low = (
            (report.get("thresholds") or {}).get("low")
            if isinstance(report.get("thresholds"), dict)
            else None
        )
        if valid and _number(low):
            score = round(100 * detected / valid, 1)
            if score < low:
                findings.append(
                    f"{path}: scores {score} (detected {detected} of {valid} valid mutants), below "
                    f"its low threshold {low}"
                )
    return Outcome(findings, examined)


# --------------------------------------------------------------------------- rows (phoenix-v2)


def root_module(context: Context, name: str) -> object:
    """The judged tree's own `scripts/<name>.py`, loaded from its source."""
    path = context.root / "scripts" / f"{name}.py"
    if not path.is_file():
        raise Void(
            f"scripts/{name}.py is absent, so the row manifest has no reader to read it with"
        )
    spec = importlib.util.spec_from_file_location(f"_judged_{name}", path)
    if spec is None or spec.loader is None:
        raise Void(f"scripts/{name}.py cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def row_reader(context: Context) -> object:
    """The judged tree's own population reader, `scripts/mutation_rows.py`."""
    if not context.exists(ROW_MANIFEST):
        raise Void(
            f"no phoenix-v2 row manifest at {ROW_MANIFEST}: this class judges that craft only"
        )
    return root_module(context, "mutation_rows")


def assembled(context: Context, reader: object) -> object:
    """The population, or VOID when it cannot be read or does not assemble."""
    try:
        return reader.load_tree(context.root)
    except reader.PopulationRefused as refusal:
        raise Void(f"the population does not assemble: {refusal}") from refusal
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"{ROW_MANIFEST} cannot be read: {error}") from error


def row_tables(population: object) -> dict[str, list]:
    tables = population.get("tables") if isinstance(population, dict) else None
    if not isinstance(tables, dict):
        raise Void(f"{ROW_MANIFEST} holds no tables of rows")
    for table in tables:
        if table not in ROW_CELLS:
            raise Void(
                f"the table {table} keeps its cells in a layout this probe does not know"
            )
    return tables


def check_find_differs(context: Context) -> Outcome:
    """`find-differs`: every row's find is non-empty and differs from its replacement."""
    population = assembled(context, row_reader(context))
    findings = []
    examined = 0
    for table, rows in row_tables(population).items():
        find_cell, replace_cell, _ = ROW_CELLS[table]
        for row in rows:
            examined += 1
            identifier = row[0] if isinstance(row, list) and row else "<no id>"
            if not isinstance(row, list) or len(row) <= replace_cell:
                findings.append(
                    f"{identifier} ({table}): carries no find and replacement cells"
                )
            elif row[find_cell] == "":
                findings.append(
                    f"{identifier} ({table}): its find is empty, so no mutant can be installed"
                )
            elif row[find_cell] == row[replace_cell]:
                findings.append(
                    f"{identifier} ({table}): its replacement equals its find, so its mutant is the "
                    "unchanged file and no killer can fail on it"
                )
    return Outcome(findings, examined)


def check_band_ids(context: Context) -> Outcome:
    """`band-ids`: the population assembles through its reader and holds no id twice."""
    reader = row_reader(context)
    try:
        population = reader.load_tree(context.root)
    except reader.PopulationRefused as refusal:
        return Outcome(
            [f"{ROW_MANIFEST}: the population refuses to assemble: {refusal}"], 1
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"{ROW_MANIFEST} cannot be read: {error}") from error
    holders: dict[str, list[str]] = defaultdict(list)
    for table, rows in row_tables(population).items():
        for row in rows:
            holders[row[0] if isinstance(row, list) and row else "<no id>"].append(
                table
            )
    findings = [
        f"{identifier} is held {len(tables)} times: {', '.join(tables)}"
        for identifier, tables in sorted(holders.items())
        if len(tables) > 1
    ]
    return Outcome(findings, sum(len(tables) for tables in holders.values()))


def check_mutants_distinct(context: Context) -> Outcome:
    """`mutants-distinct`: no two rows install the same mutant for the same killer."""
    population = assembled(context, row_reader(context))
    resolver = root_module(context, "row_target")
    try:
        spellings = resolver.declared_spellings(population)
    except resolver.UnresolvableTarget as refusal:
        raise Void(str(refusal)) from refusal
    groups: dict[tuple, list[str]] = defaultdict(list)
    examined = 0
    for table, rows in row_tables(population).items():
        find_cell, replace_cell, crate_cell = ROW_CELLS[table]
        for row in rows:
            if not isinstance(row, list) or len(row) <= replace_cell:
                continue
            examined += 1
            try:
                target = resolver.row_target(table, row, spellings)
            except resolver.UnresolvableTarget as refusal:
                raise Void(str(refusal)) from refusal
            killer = row[ROW_KILLER_CELL] if len(row) > ROW_KILLER_CELL else None
            runner = row[crate_cell] if crate_cell is not None else "unittest"
            key = (target, row[find_cell], row[replace_cell], runner, killer)
            groups[key].append(row[0])
    findings = [
        f"{' and '.join(ids)} install one mutant of {key[0]} for one killer {key[4]}: each id past "
        "the first adds no evidence"
        for key, ids in groups.items()
        if len(ids) > 1
    ]
    return Outcome(findings, examined)


# --------------------------------------------------------------------------- the class table


@dataclass(frozen=True)
class Klass:
    name: str
    stage: str
    severity: str
    may_be_empty: bool
    run: Callable[[Context], Outcome]


CLASSES: tuple[Klass, ...] = (
    Klass("tool-config-valid", "practice", "block", True, check_tool_config_valid),
    Klass("tool-configured", "practice", "advisory", False, check_tool_configured),
    Klass(
        "tool-config-current", "practice", "advisory", True, check_tool_config_current
    ),
    Klass("timeouts-bounded", "practice", "advisory", True, check_timeouts_bounded),
    Klass(
        "exclusions-justified", "practice", "advisory", True, check_exclusions_justified
    ),
    Klass("ci-diff-run", "practice", "advisory", False, check_ci_diff_run),
    Klass("ci-full-run", "practice", "advisory", False, check_ci_full_run),
    Klass("shards-complete", "practice", "advisory", True, check_shards_complete),
    Klass("report-kept", "practice", "advisory", True, check_report_kept),
    Klass("report-schema", "practice", "advisory", True, check_report_schema),
    Klass("survivors-triaged", "practice", "advisory", True, check_survivors_triaged),
    Klass("score-threshold", "practice", "advisory", True, check_score_threshold),
    Klass("find-differs", "rows", "block", False, check_find_differs),
    Klass("band-ids", "rows", "block", False, check_band_ids),
    Klass("mutants-distinct", "rows", "advisory", False, check_mutants_distinct),
)
BY_NAME = {klass.name: klass for klass in CLASSES}


def run_class(context: Context, klass: Klass) -> tuple[int, list[str]]:
    """One class's exit and its output lines."""
    try:
        outcome = klass.run(context)
    except Void as void:
        return EXIT_VOID, [f"{klass.name}: VOID {void}", "examined 0"]
    lines = [f"{klass.name}: {finding}" for finding in outcome.findings]
    if outcome.examined == 0 and not klass.may_be_empty:
        return EXIT_VOID, lines + [
            f"{klass.name}: VOID it examined nothing",
            "examined 0",
        ]
    if outcome.examined == 0:
        lines.append(f"{klass.name}: nothing of its kind is in the tree")
    lines.append(f"examined {outcome.examined}")
    return (EXIT_FINDING if outcome.findings else EXIT_GREEN), lines


def run_walk(context: Context) -> int:
    """Every practice class, then a verdict line each; the exit is the blocking classes' alone."""
    verdicts = {EXIT_GREEN: "green", EXIT_FINDING: "finding", EXIT_VOID: "void"}
    tally: Counter[tuple[str, str]] = Counter()
    practice = [klass for klass in CLASSES if klass.stage == "practice"]
    lines: list[str] = []
    for klass in practice:
        code, output = run_class(context, klass)
        lines.extend(output)
        verdict = verdicts[code]
        lines.append(f"walk: {klass.name} {klass.severity} {verdict}")
        tally[(klass.severity, verdict)] += 1
    lines.append(
        f"walk: {len(practice)} classes; block {tally[('block', 'finding')]} finding, "
        f"{tally[('block', 'void')]} void; advisory {tally[('advisory', 'finding')]} finding, "
        f"{tally[('advisory', 'void')]} void"
    )
    print("\n".join(lines))
    if tally[("block", "finding")]:
        return EXIT_FINDING
    if tally[("block", "void")]:
        return EXIT_VOID
    return EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    """`--root` is accepted before or after the verb."""
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    top = argparse.ArgumentParser(
        description="Judge a repository's mutation-testing practice and phoenix-v2's row craft.",
        parents=[common],
    )
    verbs = top.add_subparsers(dest="verb", required=True)
    check = verbs.add_parser("check", parents=[common], help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=list(BY_NAME))
    verbs.add_parser("walk", parents=[common], help="run every practice class")
    verbs.add_parser("classes", help="print each class, its stage and its severity")
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for klass in CLASSES:
            print(f"{klass.name} {klass.stage} {klass.severity}")
        return EXIT_GREEN
    root = Path(getattr(args, "root", Path("."))).resolve()
    if not root.is_dir():
        print(f"mutation-probe: --root {root} is not a directory", file=sys.stderr)
        return EXIT_USAGE
    if tomllib is None:
        print("mutation-probe: VOID Python 3.11 or newer is required (tomllib)")
        print("examined 0")
        return EXIT_VOID
    context = Context(root=root, files=walk_tree(root))
    if args.verb == "walk":
        return run_walk(context)
    code, lines = run_class(context, BY_NAME[args.klass])
    print("\n".join(lines))
    return code


if __name__ == "__main__":
    sys.exit(main())

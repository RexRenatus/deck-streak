#!/usr/bin/env python3
"""methodology_probe: what the portable SDD, DDD and TDD probes share (SPEC-V2-2186 R4, R5).

Three probes read one repository-root config, `methodology.json`, and speak one contract:

* one line per finding, `<class>: <where>: <what>`, then ONE verdict line per class,
  `<PACK> <class> <VERDICT>: examined N <unit>...`;
* `OK` exits 0, `REFUSED` 1, `VOID` 3 when the class examined nothing, and `ADVISORY` 0 when the
  repository's config names a stronger gate that holds the class there;
* a config the probe cannot read, or a key it does not know, exits 2 by name, before any class
  runs, because a typo that silently fell back to a default would judge the wrong tree.

A probe is vendored by copying the three CLIs and this module into one directory of another
repository; `vendored_from`, the commit the copy came from, is printed on every verdict line so a
stale copy names itself. Standard library only, Python 3.11 or later (`tomllib`).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import tomllib
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path

CONFIG_FILE = "methodology.json"
OK, REFUSED, USAGE, VOID = 0, 1, 2, 3

# Every key a section may carry, with its default. A key outside these is refused by name.
DEFAULTS: dict[str, dict[str, object]] = {
    "sdd": {
        "specs": "docs/specs",
        "decisions": "docs/decisions",
        "schematics": "docs/schematics",
        "spec_prefix": "SPEC-",
        "adr_prefix": "ADR-",
        "adopted_from": 0,
        "spec_sections": [
            "problem",
            "requirements",
            "acceptance",
            "manifest",
            "exclusions",
            "risks",
        ],
        "citation": r"(?<![A-Za-z0-9])(?:i\d+|#\d+)(?![A-Za-z0-9])",
        "advisory": {},
    },
    "ddd": {
        "context_map": "docs/CONTEXT-MAP.md",
        "map_heading": "",
        "dependency_prefix": "",
        "lexicon": "docs/LEXICON.md",
        "python_roots": [],
        "include_dev_dependencies": False,
        "exclude": [
            "**/test_*.py",
            "**/*_test.py",
            "**/conftest.py",
            "**/tests/**",
            "**/__tests__/**",
            "**/*.test.*",
            "**/*.spec.*",
            "**/*.d.ts",
        ],
        "advisory": {},
    },
    "tdd": {
        "red_first": "docs/red-first",
        "test_globs": [
            "**/test_*.py",
            "**/*_test.py",
            "**/*.test.ts",
            "**/*.test.tsx",
            "**/*.spec.ts",
            "**/*.spec.tsx",
            "**/*.test.js",
            "**/*.test.jsx",
            "**/*.spec.js",
            "**/*.spec.jsx",
            "**/*.test.mjs",
            "**/*.test.mts",
            "**/*.rs",
        ],
        "examined_calls": ["examined", "examined_may_be_empty", "examinedMayBeEmpty"],
        "advisory": {},
    },
}
TOP_LEVEL = ("vendored_from", *DEFAULTS)

# Directories no probe walks: tool caches and build output, never a project's own source.
SKIP_DIRS = frozenset(
    {
        ".git",
        "node_modules",
        "target",
        ".venv",
        "venv",
        "__pycache__",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".tox",
        "dist",
        "build",
        ".next",
        "coverage",
    }
)


class ConfigError(Exception):
    """A configuration the probe will not judge from; the message names what is wrong."""


class ProbeError(Exception):
    """A class could not read what it judges (a git census outside a repository, say)."""


@dataclass(frozen=True)
class Config:
    """The merged sections of `methodology.json`, and the commit a vendored copy came from."""

    sections: dict[str, dict]
    vendored_from: str | None


def load_config(root: Path, path: Path | None = None) -> Config:
    """Read and validate the config; every section is its defaults overlaid by what is given."""
    file = path if path is not None else root / CONFIG_FILE
    name = CONFIG_FILE if path is None else str(path)
    raw: object = {}
    if file.is_file():
        try:
            raw = json.loads(file.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ConfigError(f"{name}: cannot be read as JSON: {error}") from error
    elif path is not None:
        raise ConfigError(f"{name}: no such file")
    if not isinstance(raw, dict):
        raise ConfigError(f"{name}: the top level must be an object")
    unknown = sorted(set(raw) - set(TOP_LEVEL))
    if unknown:
        raise ConfigError(f"{name}: unknown key(s): {', '.join(unknown)}")
    vendored = raw.get("vendored_from")
    if vendored is not None and (not isinstance(vendored, str) or not vendored.strip()):
        raise ConfigError(f"{name}: vendored_from must be a non-empty string")
    sections: dict[str, dict] = {}
    for section, defaults in DEFAULTS.items():
        given = raw.get(section, {})
        if not isinstance(given, dict):
            raise ConfigError(f"{name}: {section} must be an object")
        unknown = sorted(set(given) - set(defaults))
        if unknown:
            raise ConfigError(
                f"{name}: {section}: unknown key(s): {', '.join(unknown)}"
            )
        for key, value in given.items():
            expected = type(defaults[key])
            if expected is int and (
                isinstance(value, bool) or not isinstance(value, int)
            ):
                raise ConfigError(f"{name}: {section}: {key} must be an integer")
            if not isinstance(value, expected):
                raise ConfigError(
                    f"{name}: {section}: {key} must be a {expected.__name__}"
                )
            if expected is list and not all(isinstance(item, str) for item in value):
                raise ConfigError(f"{name}: {section}: {key} must list strings")
        advisory = given.get("advisory", {})
        if any(not isinstance(v, str) or not v.strip() for v in advisory.values()):
            raise ConfigError(
                f"{name}: {section}: every advisory names its stronger gate"
            )
        sections[section] = {**defaults, **given}
    return Config(sections=sections, vendored_from=vendored)


@dataclass
class Result:
    """What one class examined and found. `examined == 0` is VOID, whatever else it says."""

    examined: int
    unit: str
    findings: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    void_reason: str = ""


def report(
    pack: str, check: str, result: Result, advisory: str | None, vendored: str | None
) -> int:
    """Print the findings and the one verdict line; return the class's exit code."""
    for finding in result.findings:
        print(f"{check}: {finding}")
    counted = f"examined {result.examined} {result.unit}"
    tail = "".join(f"; {note}" for note in result.notes)
    if result.examined == 0:
        word, code = "VOID", VOID
        why = f"; {result.void_reason}" if result.void_reason else ""
        body = counted + why + tail
    elif result.findings:
        body = f"{counted}, {len(result.findings)} finding(s)"
        if advisory:
            word, code = "ADVISORY", OK
            body += f"; this repository enforces it with: {advisory}"
        else:
            word, code = "REFUSED", REFUSED
        body += tail
    else:
        word, code = "OK", OK
        body = counted + tail
        if advisory:
            body += f"; advisory here, enforced by: {advisory}"
    if vendored:
        body += f" (probe vendored from {vendored})"
    print(f"{pack} {check} {word}: {body}")
    return code


def worst(codes: Iterable[int]) -> int:
    """`check all`'s exit: VOID over REFUSED over OK, since an unjudged class decides nothing."""
    codes = list(codes)
    if VOID in codes:
        return VOID
    if REFUSED in codes:
        return REFUSED
    return OK


@dataclass(frozen=True)
class Context:
    """What a class function is handed."""

    root: Path
    config: Config
    args: argparse.Namespace

    def section(self, name: str) -> dict:
        return self.config.sections[name]


ClassFn = Callable[[Context], Result]


def run(
    pack: str,
    section: str,
    classes: dict[str, tuple[str, ClassFn]],
    argv: list[str] | None = None,
    extend: Callable[[argparse.ArgumentParser, argparse._SubParsersAction], None]
    | None = None,
    verbs: dict[str, Callable[[Context], int]] | None = None,
    validate: Callable[[Config], None] | None = None,
) -> int:
    """The CLI every probe shares: `--root R [--config F] (check <class>|all | list | ...)`."""
    parser = argparse.ArgumentParser(prog=f"{section}-probe.py")
    parser.add_argument("--root", required=True, help="the repository to judge")
    parser.add_argument("--config", help=f"a config other than <root>/{CONFIG_FILE}")
    sub = parser.add_subparsers(dest="verb", required=True)
    check = sub.add_parser("check", help="run one class, or `all`")
    check.add_argument("name")
    sub.add_parser("list", help="name every class and what it holds")
    if extend is not None:
        extend(check, sub)
    args = parser.parse_args(argv)
    root = Path(args.root).resolve()
    if args.verb == "list":
        for name, (rule, _) in classes.items():
            print(f"{name}: {rule}")
        return OK
    if not root.is_dir():
        print(f"{section}-probe: --root {args.root} is not a directory")
        return USAGE
    try:
        config = load_config(root, Path(args.config) if args.config else None)
        advisory = config.sections[section]["advisory"]
        unknown = sorted(set(advisory) - set(classes))
        if unknown:
            raise ConfigError(
                f"{CONFIG_FILE}: {section}: advisory names unknown class(es): "
                f"{', '.join(unknown)}"
            )
        if validate is not None:
            validate(config)
    except ConfigError as error:
        print(f"{section}-probe: {error}")
        return USAGE
    context = Context(root=root, config=config, args=args)
    try:
        if verbs and args.verb in verbs:
            return verbs[args.verb](context)
        if args.name == "all":
            names = list(classes)
        elif args.name in classes:
            names = [args.name]
        else:
            known = ", ".join(classes)
            print(f"{section}-probe: unknown class {args.name}; known: {known}")
            return USAGE
        codes = []
        for name in names:
            result = classes[name][1](context)
            gate = advisory.get(name)
            codes.append(report(pack, name, result, gate, config.vendored_from))
    except ProbeError as error:
        print(f"{section}-probe: {error}")
        return USAGE
    return worst(codes)


# ------------------------------------------------------------------------------ markdown


FENCE_LINE = re.compile(r"^```+(.*)$")


@dataclass
class Block:
    """One fenced block at column 0: its info string, its opening line and its lines."""

    info: str
    start: int
    lines: list[tuple[int, str]]
    closed: bool


def blocks(text: str) -> list[Block]:
    """Every fenced block in `text`, in order. An unclosed block runs to the end, `closed=False`."""
    found: list[Block] = []
    current: Block | None = None
    for number, line in enumerate(text.splitlines(), start=1):
        if current is None:
            opened = FENCE_LINE.match(line)
            if opened:
                current = Block(opened.group(1).strip(), number, [], False)
            continue
        if line.rstrip() == "```":
            current.closed = True
            found.append(current)
            current = None
            continue
        current.lines.append((number, line))
    if current is not None:
        found.append(current)
    return found


@dataclass
class Section:
    """One `## ` section: its title with any `3.` numbering removed, and its lines."""

    title: str
    line: int
    lines: list[tuple[int, str, bool]]

    def prose(self) -> list[tuple[int, str]]:
        """The section's lines outside every fenced block, fence lines excluded."""
        return [(n, text) for n, text, fenced in self.lines if not fenced]

    def is_empty(self) -> bool:
        return not any(text.strip() for _, text, _ in self.lines)


NUMBERING = re.compile(r"^\s*(?:\d+[a-z]?|[A-Z])[.)]\s+")


def sections(text: str) -> list[Section]:
    """Every level-2 section outside fenced blocks. A `## ` inside a fence is not a heading."""
    found: list[Section] = []
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        is_fence_line = (
            bool(FENCE_LINE.match(line)) if not in_fence else line.rstrip() == "```"
        )
        if not in_fence and not is_fence_line and line.startswith("## "):
            title = NUMBERING.sub("", line[3:].strip())
            found.append(Section(title, number, []))
            continue
        fenced = in_fence or is_fence_line
        if is_fence_line:
            in_fence = not in_fence
        if found:
            found[-1].lines.append((number, line, fenced))
    return found


def table_rows(lines: Iterable[tuple[int, str]]) -> list[tuple[int, list[str]]]:
    """The data rows of every markdown table in `lines`: header and separator rows dropped."""
    rows: list[tuple[int, list[str]]] = []
    previous_was_table = False
    for number, line in lines:
        stripped = line.strip()
        if not stripped.startswith("|"):
            previous_was_table = False
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        is_separator = all(re.fullmatch(r":?-{3,}:?", cell) for cell in cells if cell)
        if is_separator:
            previous_was_table = True
            continue
        if previous_was_table:
            rows.append((number, cells))
        # The first row of a table is its header; only rows after the separator are data.
    return rows


# ------------------------------------------------------------------------------ documents


@dataclass(frozen=True)
class Doc:
    """A numbered document: `<prefix><number>-<slug>.md`."""

    path: Path
    rel: str
    ident: str
    number: int
    slug: str

    @property
    def amendment(self) -> bool:
        return self.slug == "amendment" or self.slug.startswith("amendment-")


def numbered_docs(root: Path, directory: str, prefix: str) -> list[Doc]:
    """Every `<prefix><n>-<slug>.md` directly in `directory`, sorted by name."""
    base = root / directory
    if not base.is_dir():
        return []
    pattern = re.compile(rf"^({re.escape(prefix)}(\d+))-(.+)\.md$")
    docs = []
    for path in sorted(base.iterdir()):
        matched = pattern.match(path.name)
        if matched and path.is_file():
            docs.append(
                Doc(
                    path=path,
                    rel=path.relative_to(root).as_posix(),
                    ident=matched.group(1),
                    number=int(matched.group(2)),
                    slug=matched.group(3),
                )
            )
    return docs


CRITERION = r"[A-Za-z]{1,4}\d+[a-z]?"
ACCEPTANCE_LINE = re.compile(rf"^({CRITERION})\s*:\s*(\S.*?)\s*$")


@dataclass
class Acceptance:
    """A SPEC's `acceptance` fences: each (line, criterion, command), and what did not parse.

    A problem is `(line, message)`, the line being `None` when it names the whole fence.
    """

    fenced: bool
    lines: list[tuple[int, str, str]]
    problems: list[tuple[int | None, str]]


def acceptance(text: str) -> Acceptance:
    """Read every ```acceptance block; a line is `<criterion>: <command>`."""
    result = Acceptance(False, [], [])
    for block in blocks(text):
        if block.info != "acceptance":
            continue
        result.fenced = True
        if not block.closed:
            result.problems.append(
                (None, f"the acceptance fence at line {block.start} is never closed")
            )
        for number, line in block.lines:
            if not line.strip():
                continue
            parsed = ACCEPTANCE_LINE.match(line.strip())
            if parsed is None:
                message = f"not `<criterion>: <command>`: {line.strip()}"
                result.problems.append((number, message))
                continue
            result.lines.append((number, parsed.group(1), parsed.group(2)))
    return result


def judged_specs(context: Context) -> tuple[list[Doc], list[str], int, int]:
    """The SPECs the shape classes judge, the notes on the rest, and the two counts left out.

    Amendments share their parent's number and are not judged for shape; a SPEC numbered below
    `adopted_from` predates the adoption and is counted, not judged.
    """
    sdd = context.section("sdd")
    docs = numbered_docs(context.root, sdd["specs"], sdd["spec_prefix"])
    floor = sdd["adopted_from"]
    amendments = [doc for doc in docs if doc.amendment]
    below = [doc for doc in docs if not doc.amendment and doc.number < floor]
    judged = [doc for doc in docs if not doc.amendment and doc.number >= floor]
    notes = []
    if below:
        notes.append(f"{len(below)} below adopted_from {floor} counted, not judged")
    if amendments:
        notes.append(f"{len(amendments)} amendment(s) not judged for shape")
    return judged, notes, len(below), len(amendments)


# ------------------------------------------------------------------------------ files


def glob_regex(pattern: str) -> re.Pattern[str]:
    """A repository-relative glob as a regex: `**/` spans directories, `*` stays in one."""
    out = []
    index = 0
    while index < len(pattern):
        if pattern.startswith("**/", index):
            out.append("(?:.*/)?")
            index += 3
        elif pattern.startswith("**", index):
            out.append(".*")
            index += 2
        elif pattern[index] == "*":
            out.append("[^/]*")
            index += 1
        elif pattern[index] == "?":
            out.append("[^/]")
            index += 1
        else:
            out.append(re.escape(pattern[index]))
            index += 1
    return re.compile("^" + "".join(out) + "$")


def matches_any(rel: str, patterns: Iterable[re.Pattern[str]]) -> bool:
    return any(pattern.match(rel) for pattern in patterns)


def files_under(root: Path, base: Path, suffixes: tuple[str, ...]) -> list[Path]:
    """Every file under `base` whose name ends with one of `suffixes`, build output skipped."""
    if base.is_file():
        return [base] if base.name.endswith(suffixes) else []
    if not base.is_dir():
        return []
    found = []
    for directory, subdirectories, names in os.walk(base):
        subdirectories[:] = sorted(d for d in subdirectories if d not in SKIP_DIRS)
        for name in sorted(names):
            if name.endswith(suffixes):
                found.append(Path(directory) / name)
    return found


def files_matching(root: Path, globs: Iterable[str]) -> list[Path]:
    """Every file under `root` matching one of `globs`, walking only each glob's fixed prefix."""
    found: dict[str, Path] = {}
    for pattern in globs:
        regex = glob_regex(pattern)
        fixed = []
        for part in pattern.split("/")[:-1]:
            if any(char in part for char in "*?["):
                break
            fixed.append(part)
        base = root.joinpath(*fixed) if fixed else root
        for path in files_under(root, base, ("",)):
            rel = path.relative_to(root).as_posix()
            if regex.match(rel):
                found[rel] = path
    return [found[rel] for rel in sorted(found)]


def rel(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


# ------------------------------------------------------------------------------ source text


def blank_out(text: str, rust: bool, strings: bool) -> str:
    """`text` with comments blanked and, when `strings`, string contents filled, newlines kept.

    A comment becomes spaces. A string's contents become `_`, so `"3 due"` stays a non-empty
    string of the same length while `""` stays empty, and no keyword or bracket inside a string
    survives to be read as code. Offsets are unchanged, so a line number read on the result is the
    line of the original text.
    """
    out = list(text)
    size = len(text)

    def blank(start: int, end: int, fill: str = " ") -> None:
        for index in range(start, min(end, size)):
            if out[index] != "\n":
                out[index] = fill

    index = 0
    while index < size:
        char = text[index]
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            end = size if end < 0 else end + 2
            blank(index, end)
            index = end
            continue
        raw = (
            re.match(r'r(#*)"', text[index : index + 16])
            if rust and char == "r"
            else None
        )
        if raw and (
            index == 0 or not (text[index - 1].isalnum() or text[index - 1] == "_")
        ):
            close = '"' + raw.group(1)
            end = text.find(close, index + len(raw.group(0)))
            end = size if end < 0 else end + len(close)
            if strings:
                blank(index + len(raw.group(0)), end - len(close), "_")
            index = end
            continue
        quote = char in "\"'`" if not rust else char == '"'
        if rust and char == "'":
            quote = index + 1 < size and (
                text[index + 1] == "\\" or (index + 2 < size and text[index + 2] == "'")
            )
        if quote:
            end = index + 1
            while end < size and text[end] != char:
                if text[end] == "\\":
                    end += 2
                    continue
                if text[end] == "\n" and char in "'\"" and not rust:
                    break
                end += 1
            if strings:
                blank(index + 1, end, "_")
            index = end + 1
            continue
        index += 1
    return "".join(out)


def line_of(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


# ------------------------------------------------------------------------------ cargo


def workspace(root: Path) -> dict[str, Path]:
    """Every package of the root Cargo workspace, by name: `members` globs less `exclude`."""
    manifest = root / "Cargo.toml"
    if not manifest.is_file():
        return {}
    data = load_toml(manifest)
    members: list[Path] = [root] if "package" in data else []
    work = data.get("workspace", {})
    excluded = {(root / item).resolve() for item in work.get("exclude", [])}
    for pattern in work.get("members", []):
        members.extend(sorted(path for path in root.glob(pattern) if path.is_dir()))
    packages = {}
    for member in members:
        member_manifest = member / "Cargo.toml"
        if member.resolve() in excluded or not member_manifest.is_file():
            continue
        name = load_toml(member_manifest).get("package", {}).get("name")
        if name:
            packages[name] = member
    return packages


def load_toml(path: Path) -> dict:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ProbeError(f"{path}: not valid TOML: {error}") from error

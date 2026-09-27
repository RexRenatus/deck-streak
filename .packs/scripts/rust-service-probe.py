#!/usr/bin/env python3
"""rust-service-probe.py -- judge a Rust service workspace against the rust-service pack.

SPEC-V2-2220 / ADR-V2-2220. The pack is `skills/packs/rust-service`: axum 0.8, tokio and tracing
services on a small VM (2 vCPU, 1.9 GiB), with lint levels, error types, graceful shutdown on
SIGTERM, secrets kept out of the environment, and a memory budget of bounded concurrency and no
unbounded buffers. Every class is a STATIC read of `--root`: the Cargo manifests, the Rust sources,
the CI workflows and the `deploy/` systemd units. Nothing is built, run or fetched.

It composes, and copies nothing. The systemd units are read through `durable-unit-lint.py`'s own
parser (`load_subject`, `long_running`, `service_type`, `seconds`), and a secret's name is judged by
that script's `SECRET_NAME` and `REFERENCE_NAME`, so a unit and a variable mean here what they mean
to the durable-services pack. Vendor this file next to `durable-unit-lint.py`; the observability
pack's probe imports this one the same way.

usage:
  rust-service-probe.py [--root DIR] check CLASS
  rust-service-probe.py classes

CLASS is one of the row ids of `skills/packs/rust-service/checks.json`:
  rs.unsafe-forbidden       every crate forbids unsafe code (a lint table or #![forbid(unsafe_code)])
  rs.lints-inherited        with [workspace.lints], every member declares a [lints] table
  rs.lint-group-priority    a lint group beside a single lint at another level sits at priority -1
  rs.no-restriction-group   the whole clippy::restriction group is never enabled
  rs.clippy-ci              a CI workflow runs cargo clippy with warnings denied
  rs.fmt-ci                 a CI workflow runs cargo fmt --check (advisory)
  rs.panic-lints            every crate lints clippy::unwrap_used at warn or above (advisory)
  rs.typed-lib-errors       a library's public functions return typed errors, not anyhow (advisory)
  rs.catch-panic            every HTTP service converts a handler panic into a response (advisory)
  rs.axum-route-syntax      no axum 0.8 route uses the retired /:param or /*param capture
  rs.request-timeout        every HTTP service bounds a request's lifetime (advisory)
  rs.graceful-shutdown      every axum::serve call drains with with_graceful_shutdown
  rs.sigterm-handled        every service binary handles the signal its unit stops it with
  rs.notify-ready           a Type=notify unit's binary sends READY=1
  rs.watchdog-ping          a WatchdogSec= unit's binary sends WATCHDOG=1
  rs.no-secret-env          production code reads no secret-named environment variable
  rs.credentials-read       a unit's LoadCredential= reaches a binary that reads the credential (advisory)
  rs.bounded-channels       production code opens no unbounded channel
  rs.bounded-body-reads     production code never buffers a body with no limit
  rs.concurrency-bound      every HTTP service bounds its concurrent requests
  rs.no-blocking-in-async   no async body sleeps or blocks its worker thread (advisory)

Output is one line per finding, `<class>: <finding>`, then `examined N`, the population the class
read. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: no Rust workspace at the root, an
input that could not be read, or a sibling script that is missing, which is never a pass.
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
import types
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11: `main` reports VOID, never a crash.
    tomllib = None  # type: ignore[assignment]

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

HERE = Path(__file__).resolve().parent
DURABLE_SCRIPT = "durable-unit-lint.py"

# Directories that hold no production source: build output, installed packages, tool state. Every
# directory whose name starts with "." is skipped as well; CI workflows are read by fixed path.
SKIPPED_DIRS = frozenset({"target", "node_modules", "dist", "build", "__pycache__"})
# A crate's non-production trees (Cargo's target auto-discovery: tests, benches, examples).
TEST_DIRS = frozenset({"tests", "benches", "examples"})
WORKFLOW_GLOBS = (
    ".github/workflows/*.yml",
    ".github/workflows/*.yaml",
    ".depot/workflows/*.yml",
    ".depot/workflows/*.yaml",
)
WORKFLOW_FILES = (".gitlab-ci.yml",)
TASK_FILES = (
    "justfile",
    "Justfile",
    ".justfile",
    "Makefile",
    "makefile",
    "Taskfile.yml",
)

LEVELS = ("allow", "expect", "warn", "force-warn", "deny", "forbid")
RAISED = frozenset({"warn", "force-warn", "deny", "forbid"})
# Lint groups (rustc -W help; the clippy README's categories). `warnings` is not judged: the
# clippy fix for #12270 ignores it, because its order against a single lint does not matter.
CLIPPY_GROUPS = frozenset(
    {
        "all",
        "correctness",
        "suspicious",
        "style",
        "complexity",
        "perf",
        "pedantic",
        "restriction",
        "nursery",
        "cargo",
    }
)
RUST_GROUPS = frozenset(
    {
        "future_incompatible",
        "keyword_idents",
        "let_underscore",
        "nonstandard_style",
        "refining_impl_trait",
        "rust_2018_compatibility",
        "rust_2018_idioms",
        "rust_2021_compatibility",
        "rust_2024_compatibility",
        "unknown_or_malformed_diagnostic_attributes",
        "unused",
        "deprecated_safe",
    }
)

# --------------------------------------------------------------------------- the outcome


class Void(Exception):
    """Nothing could be judged: no workspace, an unreadable input, or a missing sibling."""


@dataclass
class Outcome:
    findings: list[str] = field(default_factory=list)
    examined: int = 0


# --------------------------------------------------------------------------- composing a sibling


def load_sibling(module_name: str, filename: str) -> types.ModuleType:
    """Import a hyphenated sibling script by path, once per process.

    The module is registered before it runs, because its dataclasses resolve their annotations
    through `sys.modules` (the same reason `scripts/pack-census.py` gives for its `_load`).
    """
    if module_name in sys.modules:
        return sys.modules[module_name]
    path = HERE / filename
    if not path.is_file():
        raise Void(f"{filename} is not beside this script; vendor it with this one")
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise Void(f"cannot load {filename}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def durable() -> types.ModuleType:
    return load_sibling("durable_unit_lint", DURABLE_SCRIPT)


# --------------------------------------------------------------------------- Rust source


CHAR_LITERAL = re.compile(
    r"b?'(?:\\u\{[0-9a-fA-F]{1,6}\}|\\x[0-9a-fA-F]{2}|\\[nrt0\\'\"]|[^\\'\n])'"
)
IDENT_CHAR = re.compile(r"[A-Za-z0-9_]")


@dataclass(frozen=True)
class Literal:
    """One string literal: its span in the file, where its content starts, and its content."""

    start: int
    end: int
    body_start: int
    value: str


NOT_NEWLINE = re.compile(r"[^\n]")
TOKEN = re.compile(r"//|/\*|b?r#*\"|b?\"|b?'")
BLOCK_TOKEN = re.compile(r"/\*|\*/")
STRING_BODY = re.compile(r'(?:[^"\\]|\\.)*', re.S)
ESCAPE = re.compile(r"\\(.)", re.S)
ESCAPED = {"n": "\n", "t": "\t", "r": "\r", "0": "\0"}


def blank(segment: str) -> str:
    """The segment with every character but a newline turned into a space: offsets survive."""
    return NOT_NEWLINE.sub(" ", segment)


def lex(text: str) -> tuple[str, str, list[Literal]]:
    """Split Rust text into (`code`, `bare`, literals), every offset preserved.

    `code` is the text with comments blanked; string literals stay. `bare` blanks string and
    char literals as well, so an identifier search never matches inside a string. Block comments
    nest, as Rust's do. A lifetime (`'a`) is not a char literal and is kept. The scan jumps from
    token to token, so a file costs a handful of regex searches per literal, not a step per byte.
    """
    size = len(text)
    code_parts: list[str] = []
    bare_parts: list[str] = []
    literals: list[Literal] = []
    last = 0

    def keep(start: int, end: int, blank_code: bool, blank_bare: bool) -> None:
        segment = text[start:end]
        hidden = blank(segment) if blank_code or blank_bare else segment
        code_parts.append(hidden if blank_code else segment)
        bare_parts.append(hidden if blank_bare else segment)

    index = 0
    while True:
        token = TOKEN.search(text, index)
        if token is None:
            break
        start = token.start()
        word = token.group(0)
        if word[0] in "rb" and start > 0 and IDENT_CHAR.match(text[start - 1]):
            index = start + 1
            continue
        if word == "//":
            end = text.find("\n", start)
            end = size if end < 0 else end
            keep(last, start, False, False)
            keep(start, end, True, True)
            last = index = end
            continue
        if word == "/*":
            depth = 0
            end = size
            for mark in BLOCK_TOKEN.finditer(text, start):
                depth += 1 if mark.group(0) == "/*" else -1
                if depth == 0:
                    end = mark.end()
                    break
            keep(last, start, False, False)
            keep(start, end, True, True)
            last = index = end
            continue
        if word.endswith('"') and "r" in word:
            close = '"' + "#" * word.count("#")
            body = token.end()
            stop = text.find(close, body)
            stop = size if stop < 0 else stop
            end = min(size, stop + len(close))
            literals.append(Literal(start, end, body, text[body:stop]))
            keep(last, start, False, False)
            keep(start, end, False, True)
            last = index = end
            continue
        if word.endswith('"'):
            body = token.end()
            stop = STRING_BODY.match(text, body).end()
            end = min(size, stop + 1)
            value = ESCAPE.sub(
                lambda escape: ESCAPED.get(escape.group(1), escape.group(1)),
                text[body:stop],
            )
            literals.append(Literal(start, end, body, value))
            keep(last, start, False, False)
            keep(start, end, False, True)
            last = index = end
            continue
        char = CHAR_LITERAL.match(text, start)
        if char:
            keep(last, start, False, False)
            keep(start, char.end(), False, True)
            last = index = char.end()
            continue
        index = token.end()
    keep(last, size, False, False)
    return "".join(code_parts), "".join(bare_parts), literals


BRACKETS = {
    "(": re.compile(r"[()]"),
    "[": re.compile(r"[\[\]]"),
    "{": re.compile(r"[{}]"),
    "<": re.compile(r"[<>]"),
}


def matching(bare: str, opening: int) -> int:
    """The offset just past the bracket that closes the one at `opening`, or the text's end."""
    open_char = bare[opening]
    depth = 0
    for mark in BRACKETS[open_char].finditer(bare, opening):
        depth += 1 if mark.group(0) == open_char else -1
        if depth == 0:
            return mark.end()
    return len(bare)


def blank_ranges(text: str, ranges: list[tuple[int, int]]) -> str:
    """The text with each (start, end) range blanked, newlines kept."""
    if not ranges:
        return text
    parts: list[str] = []
    last = 0
    for start, end in sorted(ranges):
        start = max(start, last)
        if end <= start:
            continue
        parts.append(text[last:start])
        parts.append(blank(text[start:end]))
        last = end
    parts.append(text[last:])
    return "".join(parts)


TEST_ATTRIBUTE = re.compile(
    r"#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|test|(?:tokio::)?test(?:\s*\([^\]]*\))?)\s*\]"
)
OUTER_ATTRIBUTE = re.compile(r"\s*#\s*\[")


def strip_tests(code: str, bare: str) -> tuple[str, str]:
    """Blank every item under `#[cfg(test)]` or `#[test]`: a test module or a test function."""
    ranges: list[tuple[int, int]] = []
    for attribute in TEST_ATTRIBUTE.finditer(bare):
        cursor = attribute.end()
        while True:
            more = OUTER_ATTRIBUTE.match(bare, cursor)
            if not more:
                break
            cursor = matching(bare, bare.index("[", more.start()))
        found = re.compile(r"[{;]").search(bare, cursor)
        if found is None:
            ranges.append((attribute.start(), len(bare)))
            continue
        stop = found.start()
        end = matching(bare, stop) if bare[stop] == "{" else stop + 1
        ranges.append((attribute.start(), end))
    return blank_ranges(code, ranges), blank_ranges(bare, ranges)


@dataclass
class Source:
    """One production Rust file: its text with comments and test items blanked."""

    path: Path
    rel: str
    code: str
    bare: str
    literals: list[Literal]
    # `bare` with every `use` declaration blanked: a name imported is not a name used, so a layer
    # whose `.layer(..)` call was deleted does not stay "present" through its leftover import.
    body: str = ""

    def line(self, offset: int) -> int:
        return self.code.count("\n", 0, offset) + 1

    def place(self, offset: int) -> str:
        return f"{self.rel}:{self.line(offset)}"

    def names(self, *tokens: str) -> bool:
        """Whether any token is USED in the code: outside strings, comments and `use` lines."""
        return any(token in self.body for token in tokens)

    def says(self, *tokens: str) -> bool:
        """Whether any token occurs in a string literal."""
        return any(
            token in literal.value for literal in self.literals for token in tokens
        )

    def literal_at(self, offset: int) -> Literal | None:
        for literal in self.literals:
            if literal.start == offset:
                return literal
        return None


USE_DECLARATION = re.compile(r"\b(?:pub(?:\s*\([^)]*\))?\s+)?use\s+[^;]*;")


def read_source(root: Path, path: Path) -> Source:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise Void(
            f"cannot read {path.relative_to(root).as_posix()}: {error}"
        ) from error
    code, bare, literals = lex(text)
    code, bare = strip_tests(code, bare)
    kept = [
        lit
        for lit in literals
        if bare[lit.start : lit.end].strip() == "" and code[lit.start] != " "
    ]
    body = blank_ranges(
        bare, [(m.start(), m.end()) for m in USE_DECLARATION.finditer(bare)]
    )
    return Source(path, path.relative_to(root).as_posix(), code, bare, kept, body)


# --------------------------------------------------------------------------- the workspace


def load_toml(root: Path, path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise Void(
            f"cannot read {path.relative_to(root).as_posix()}: {error}"
        ) from error


@dataclass
class Crate:
    name: str
    dir: Path
    rel: str
    manifest: dict
    lib: Path | None
    bins: dict[str, Path]
    deps: dict[str, dict]


def dependency_specs(manifest: dict, workspace_deps: dict) -> dict[str, dict]:
    """Normal dependencies by the crate name they resolve to (renames and `workspace = true`)."""
    tables: list[dict] = [manifest.get("dependencies", {})]
    for target in manifest.get("target", {}).values():
        if isinstance(target, dict):
            tables.append(target.get("dependencies", {}))
    out: dict[str, dict] = {}
    for table in tables:
        if not isinstance(table, dict):
            continue
        for key, spec in table.items():
            entry: dict = (
                {"version": spec} if isinstance(spec, str) else dict(spec or {})
            )
            if entry.get("workspace") is True:
                inherited = workspace_deps.get(key, {})
                inherited = (
                    {"version": inherited}
                    if isinstance(inherited, str)
                    else dict(inherited)
                )
                inherited.update({k: v for k, v in entry.items() if k != "workspace"})
                entry = inherited
            out[str(entry.get("package", key))] = entry
    return out


def discover_targets(
    crate_dir: Path, manifest: dict, name: str
) -> tuple[Path | None, dict[str, Path]]:
    """Cargo's target auto-discovery, plus the explicit [lib] and [[bin]] tables."""
    package = manifest.get("package", {})
    src = crate_dir / "src"
    lib_table = manifest.get("lib", {})
    lib: Path | None = None
    if isinstance(lib_table, dict) and lib_table.get("path"):
        lib = crate_dir / str(lib_table["path"])
    elif (src / "lib.rs").is_file():
        lib = src / "lib.rs"
    bins: dict[str, Path] = {}
    if package.get("autobins", True) is not False:
        if (src / "main.rs").is_file():
            bins[name] = src / "main.rs"
        bin_dir = src / "bin"
        if bin_dir.is_dir():
            for entry in sorted(bin_dir.iterdir()):
                if entry.is_file() and entry.suffix == ".rs":
                    bins[entry.stem] = entry
                elif entry.is_dir() and (entry / "main.rs").is_file():
                    bins[entry.name] = entry / "main.rs"
    for table in manifest.get("bin", []) or []:
        if not isinstance(table, dict) or not table.get("name"):
            continue
        bin_name = str(table["name"])
        if table.get("path"):
            bins[bin_name] = crate_dir / str(table["path"])
        elif bin_name == name and (src / "main.rs").is_file():
            bins[bin_name] = src / "main.rs"
        elif (src / "bin" / f"{bin_name}.rs").is_file():
            bins[bin_name] = src / "bin" / f"{bin_name}.rs"
    return lib, bins


@dataclass
class Workspace:
    root: Path
    manifest: dict
    crates: list[Crate]
    lock: dict[str, list[str]]
    manifests_read: int

    @property
    def workspace_lints(self) -> dict:
        lints = self.manifest.get("workspace", {}).get("lints", {})
        return lints if isinstance(lints, dict) else {}

    def crate_lints(self, crate: Crate) -> dict:
        """The lint table that applies to a crate: its own, or the workspace's when it inherits."""
        lints = crate.manifest.get("lints")
        if not isinstance(lints, dict):
            return {}
        if lints.get("workspace") is True:
            return self.workspace_lints
        return lints


def load_workspace(root: Path) -> Workspace:
    top = root / "Cargo.toml"
    if not top.is_file():
        raise Void("no Cargo.toml at the root: this is not a Rust workspace")
    manifest = load_toml(root, top)
    workspace = (
        manifest.get("workspace", {})
        if isinstance(manifest.get("workspace"), dict)
        else {}
    )
    workspace_deps = (
        workspace.get("dependencies", {}) if isinstance(workspace, dict) else {}
    )
    crate_dirs: list[Path] = []
    if "package" in manifest:
        crate_dirs.append(root)
    excluded = {
        (root / str(entry)).resolve() for entry in workspace.get("exclude", []) or []
    }
    for pattern in workspace.get("members", []) or []:
        for match in sorted(root.glob(str(pattern))):
            if (
                match.is_dir()
                and (match / "Cargo.toml").is_file()
                and match.resolve() not in excluded
            ):
                if match.resolve() not in {path.resolve() for path in crate_dirs}:
                    crate_dirs.append(match)
    crates: list[Crate] = []
    for crate_dir in crate_dirs:
        crate_manifest = (
            manifest if crate_dir == root else load_toml(root, crate_dir / "Cargo.toml")
        )
        package = crate_manifest.get("package", {})
        name = str(package.get("name", crate_dir.name))
        lib, bins = discover_targets(crate_dir, crate_manifest, name)
        rel = crate_dir.relative_to(root).as_posix() or "."
        crates.append(
            Crate(
                name,
                crate_dir,
                rel,
                crate_manifest,
                lib,
                bins,
                dependency_specs(crate_manifest, workspace_deps),
            )
        )
    lock: dict[str, list[str]] = {}
    lock_path = root / "Cargo.lock"
    if lock_path.is_file():
        for package in load_toml(root, lock_path).get("package", []) or []:
            if isinstance(package, dict) and package.get("name"):
                lock.setdefault(str(package["name"]), []).append(
                    str(package.get("version", ""))
                )
    return Workspace(
        root, manifest, crates, lock, 1 + len([c for c in crates if c.dir != root])
    )


def walk_rs(directory: Path) -> Iterable[Path]:
    if not directory.is_dir():
        return
    for path in sorted(directory.rglob("*.rs")):
        parts = path.relative_to(directory).parts[:-1]
        if any(part in SKIPPED_DIRS or part.startswith(".") for part in parts):
            continue
        yield path


class Tree:
    """The workspace, its production sources, and its units, each read once and on demand."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.workspace = load_workspace(root)
        self._sources: dict[Path, Source] = {}
        self._subject = None
        self._crate_files: dict[str, list[Path]] = {}
        self._lib_files: dict[str, list[Path]] = {}
        self._binary_sources: dict[tuple[str, str], list[Source]] = {}
        self._serve: dict[Path, list[int]] = {}
        self._http: list[tuple[Crate, str, list[Source]]] | None = None

    def source(self, path: Path) -> Source:
        key = path.resolve()
        if key not in self._sources:
            self._sources[key] = read_source(self.root, path)
        return self._sources[key]

    def serve_sites(self, source: Source) -> list[int]:
        key = source.path.resolve()
        if key not in self._serve:
            self._serve[key] = serve_sites(source)
        return self._serve[key]

    # ---- per crate

    def crate_files(self, crate: Crate) -> list[Path]:
        """Every production .rs file of a crate: `src/**`, never tests, benches or examples."""
        if crate.rel not in self._crate_files:
            self._crate_files[crate.rel] = [
                path
                for path in walk_rs(crate.dir / "src")
                if not any(
                    part in TEST_DIRS for part in path.relative_to(crate.dir).parts[:-1]
                )
            ]
        return self._crate_files[crate.rel]

    def lib_files(self, crate: Crate) -> list[Path]:
        """The crate's library side: `src/**` without `src/main.rs` and `src/bin/**`."""
        if crate.rel not in self._lib_files:
            bin_roots = {path.resolve() for path in crate.bins.values()}
            bin_dir = (crate.dir / "src" / "bin").resolve()
            self._lib_files[crate.rel] = [
                path
                for path in self.crate_files(crate)
                if path.resolve() not in bin_roots
                and bin_dir not in path.resolve().parents
            ]
        return self._lib_files[crate.rel]

    def crate_sources(self, crate: Crate) -> list[Source]:
        return [self.source(path) for path in self.crate_files(crate)]

    def all_sources(self) -> list[Source]:
        return [
            source
            for crate in self.workspace.crates
            for source in self.crate_sources(crate)
        ]

    def path_dependencies(self, crate: Crate) -> list[Crate]:
        """The workspace crates `crate` depends on, transitively, through path dependencies."""
        by_name = {member.name: member for member in self.workspace.crates}
        seen: dict[str, Crate] = {}
        queue = [crate]
        while queue:
            current = queue.pop()
            for dep_name, spec in current.deps.items():
                member = by_name.get(dep_name)
                if member is None or member.name in seen or member is crate:
                    continue
                if "path" in spec or dep_name in by_name:
                    seen[member.name] = member
                    queue.append(member)
        return list(seen.values())

    def binary_sources(self, crate: Crate, bin_name: str) -> list[Source]:
        """What a binary is built from: its root (and its own directory), its crate's library
        side, and the library side of every workspace crate it reaches by path. An
        over-approximation: code anywhere in that set counts as the binary's."""
        key = (crate.rel, bin_name)
        if key not in self._binary_sources:
            root_file = crate.bins[bin_name]
            paths: list[Path] = [root_file]
            if root_file.name == "main.rs" and root_file.parent.parent.name == "bin":
                paths.extend(
                    path for path in walk_rs(root_file.parent) if path != root_file
                )
            paths.extend(self.lib_files(crate))
            for member in self.path_dependencies(crate):
                paths.extend(self.lib_files(member))
            unique = list(dict.fromkeys(path.resolve() for path in paths))
            self._binary_sources[key] = [
                self.source(path) for path in unique if path.is_file()
            ]
        return self._binary_sources[key]

    def binaries(self) -> list[tuple[Crate, str]]:
        return [
            (crate, name)
            for crate in self.workspace.crates
            for name in sorted(crate.bins)
        ]

    def binary_index(self) -> dict[str, tuple[Crate, str]]:
        return {name: (crate, name) for crate, name in self.binaries()}

    def depends_on(self, crate: Crate, *names: str) -> bool:
        return any(name in crate.deps for name in names)

    def http_services(self) -> list[tuple[Crate, str, list[Source]]]:
        """Every binary whose sources call axum::serve: the population of the HTTP rows."""
        if self._http is None:
            self._http = []
            for crate, name in self.binaries():
                sources = self.binary_sources(crate, name)
                if any(self.serve_sites(source) for source in sources):
                    self._http.append((crate, name, sources))
        return self._http

    # ---- units, through durable-unit-lint.py

    @property
    def subject(self):
        if self._subject is None:
            self._subject = durable().load_subject(self.root)
        return self._subject

    def unit_binary(self, unit) -> tuple[Crate, str] | None:
        """The workspace binary a service unit's first ExecStart= runs, if it runs one."""
        starts = unit.values("Service", "ExecStart")
        if not starts:
            return None
        words = starts[0].split()
        if not words:
            return None
        program = words[0].lstrip("@-:+!")
        return self.binary_index().get(Path(program).name)

    def service_units(self) -> list:
        return [unit for unit in self.subject.services]

    def scheduled(self) -> set[str]:
        return self.subject.timer_activated()


# --------------------------------------------------------------------------- shared readers


def lint_level(value: object) -> tuple[str | None, int]:
    """A lint table entry's (level, priority): `"warn"` or `{ level = "warn", priority = -1 }`."""
    if isinstance(value, str):
        return value, 0
    if isinstance(value, dict):
        priority = value.get("priority", 0)
        return (
            str(value["level"]) if "level" in value else None,
            priority if isinstance(priority, int) else 0,
        )
    return None, 0


def normalise(lint: str) -> str:
    return lint.replace("-", "_")


CRATE_ATTRIBUTE = re.compile(
    r"#!\s*\[\s*(allow|expect|warn|deny|forbid)\s*\(([^\]]*)\)\s*\]"
)


def crate_attributes(source: Source) -> dict[str, str]:
    """`#![level(lint, ...)]` inner attributes of a crate root: lint -> level."""
    out: dict[str, str] = {}
    for attribute in CRATE_ATTRIBUTE.finditer(source.bare):
        for lint in attribute.group(2).split(","):
            name = lint.strip()
            if name:
                out[normalise(name)] = attribute.group(1)
    return out


def crate_roots(crate: Crate) -> list[Path]:
    roots = list(crate.bins.values())
    if crate.lib is not None:
        roots.insert(0, crate.lib)
    return [root for root in roots if root.is_file()]


LINE_CONTINUATION = re.compile(r"\\\s*\n\s*")


def ci_texts(root: Path) -> list[tuple[str, str]]:
    """Every CI workflow, plus each repository script, task file or xtask a workflow runs."""
    workflows: list[Path] = []
    for pattern in WORKFLOW_GLOBS:
        workflows.extend(sorted(root.glob(pattern)))
    workflows.extend(root / name for name in WORKFLOW_FILES if (root / name).is_file())
    texts: list[tuple[str, str]] = []
    seen: set[Path] = set()
    for workflow in workflows:
        try:
            text = workflow.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            raise Void(
                f"cannot read {workflow.relative_to(root).as_posix()}: {error}"
            ) from error
        texts.append((workflow.relative_to(root).as_posix(), text))
        seen.add(workflow.resolve())
        for word in re.findall(r"[\w./-]+", text):
            candidate = (
                (root / word.lstrip("./")).resolve()
                if not word.startswith("/")
                else None
            )
            if (
                candidate
                and candidate not in seen
                and candidate.is_file()
                and root.resolve() in candidate.parents
                and candidate.suffix in (".sh", ".bash", ".py", "")
                and "/" in word
            ):
                seen.add(candidate)
                texts.append(
                    (
                        candidate.relative_to(root.resolve()).as_posix(),
                        candidate.read_text(encoding="utf-8", errors="replace"),
                    )
                )
        runs_task = re.search(r"(?m)(?:^|\s)(just|make|task)\s", text)
        if runs_task:
            for name in TASK_FILES:
                task_file = root / name
                if task_file.is_file() and task_file.resolve() not in seen:
                    seen.add(task_file.resolve())
                    texts.append(
                        (name, task_file.read_text(encoding="utf-8", errors="replace"))
                    )
        if "cargo xtask" in text or "cargo run -p xtask" in text:
            for path in walk_rs(root / "xtask" / "src"):
                if path.resolve() not in seen:
                    seen.add(path.resolve())
                    texts.append(
                        (
                            path.relative_to(root).as_posix(),
                            path.read_text(encoding="utf-8", errors="replace"),
                        )
                    )
    return texts


def logical_lines(text: str) -> list[str]:
    return LINE_CONTINUATION.sub(" ", text).splitlines()


WARNINGS_DENIED = re.compile(
    r"(?:-D\s*warnings|--deny[=\s]+warnings|\"-D\",\s*\"warnings\"|\"-Dwarnings\")"
)
RUSTFLAGS_DENIED = re.compile(r"RUSTFLAGS\s*[:=]\s*['\"]?[^\n'\"]*-D\s*warnings")


def ci_runs(
    root: Path,
    command: re.Pattern[str],
    flag: re.Pattern[str] | None,
    env_flag: re.Pattern[str] | None,
) -> tuple[bool, int]:
    """Whether some CI text runs `command` with `flag` on the same logical line (or with
    `env_flag` set in the same file), and how many CI texts were read."""
    texts = ci_texts(root)
    for _, text in texts:
        env_ok = bool(env_flag and env_flag.search(text))
        for line in logical_lines(text):
            if command.search(line) and (flag is None or flag.search(line) or env_ok):
                return True, len(texts)
    return False, len(texts)


SERVE_CALL = re.compile(
    r"\baxum\s*::\s*serve\s*\(|\baxum\s*::\s*serve\s*::\s*serve\s*\("
)
SERVE_IMPORT = re.compile(r"\buse\s+axum\s*::\s*(?:serve\b|\{[^}]*\bserve\b[^}]*\})")
BARE_SERVE = re.compile(r"(?<![\w:.])serve\s*\(")


def serve_sites(source: Source) -> list[int]:
    """Offsets of every `axum::serve(` call, including a bare `serve(` under `use axum::serve`."""
    sites = [match.start() for match in SERVE_CALL.finditer(source.bare)]
    if SERVE_IMPORT.search(source.bare):
        sites.extend(
            match.start()
            for match in BARE_SERVE.finditer(source.bare)
            if not re.search(
                r"\bfn\s+$", source.bare[max(0, match.start() - 12) : match.start()]
            )
        )
    return sorted(set(sites))


def statement_end(bare: str, start: int) -> int:
    """The offset of the `;` that ends the statement a call at `start` sits in, at depth 0."""
    depth = 0
    for index in range(start, len(bare)):
        char = bare[index]
        if char in "([{":
            depth += 1
        elif char in ")]}":
            if depth == 0:
                return index
            depth -= 1
        elif char == ";" and depth == 0:
            return index
    return len(bare)


def http_services(tree: Tree) -> list[tuple[Crate, str, list[Source]]]:
    """Every binary whose sources call axum::serve: the population of the HTTP rows."""
    return tree.http_services()


def any_names(sources: list[Source], *tokens: str) -> bool:
    return any(source.names(*tokens) for source in sources)


def any_says(sources: list[Source], *tokens: str) -> bool:
    return any(source.says(*tokens) for source in sources)


def unit_label(unit) -> str:
    return unit.rel


# --------------------------------------------------------------------------- stage `lints`


def check_unsafe_forbidden(tree: Tree) -> Outcome:
    outcome = Outcome()
    for crate in tree.workspace.crates:
        outcome.examined += 1
        level, _ = lint_level(
            tree.workspace.crate_lints(crate).get("rust", {}).get("unsafe_code")
        )
        if level == "forbid":
            continue
        roots = crate_roots(crate)
        missing = [
            path.relative_to(tree.root).as_posix()
            for path in roots
            if crate_attributes(tree.source(path)).get("unsafe_code") != "forbid"
        ]
        if roots and not missing:
            continue
        where = f"unsafe_code is {level or 'unset'} in its lint table"
        if missing:
            where += f", and {', '.join(missing)} lack #![forbid(unsafe_code)]"
        outcome.findings.append(
            f"crate {crate.name} ({crate.rel}) does not forbid unsafe code: {where}"
        )
    return outcome


def check_lints_inherited(tree: Tree) -> Outcome:
    outcome = Outcome()
    members = [
        crate
        for crate in tree.workspace.crates
        if crate.dir != tree.root or "workspace" in tree.workspace.manifest
    ]
    outcome.examined = len(tree.workspace.crates)
    if not tree.workspace.workspace_lints:
        return outcome
    for crate in members:
        if "lints" not in crate.manifest:
            outcome.findings.append(
                f"crate {crate.name} ({crate.rel}/Cargo.toml) has no [lints] table: "
                "[workspace.lints] is not inherited implicitly; add `[lints] workspace = true`"
            )
    return outcome


def lint_tables(tree: Tree) -> list[tuple[str, str, dict]]:
    """Every lint table in the workspace: (where, tool, table)."""
    out: list[tuple[str, str, dict]] = []
    for tool, table in tree.workspace.workspace_lints.items():
        if isinstance(table, dict):
            out.append(("Cargo.toml [workspace.lints." + tool + "]", tool, table))
    for crate in tree.workspace.crates:
        lints = crate.manifest.get("lints")
        if not isinstance(lints, dict) or lints.get("workspace") is True:
            continue
        for tool, table in lints.items():
            if isinstance(table, dict):
                out.append((f"{crate.rel}/Cargo.toml [lints.{tool}]", tool, table))
    return out


def check_lint_group_priority(tree: Tree) -> Outcome:
    outcome = Outcome()
    tables = lint_tables(tree)
    outcome.examined = len(tables) + tree.workspace.manifests_read
    for where, tool, table in tables:
        groups = (
            CLIPPY_GROUPS
            if tool == "clippy"
            else RUST_GROUPS
            if tool == "rust"
            else frozenset()
        )
        entries = {normalise(name): lint_level(value) for name, value in table.items()}
        for group in sorted(set(entries) & groups):
            group_level, group_priority = entries[group]
            for lint, (level, priority) in sorted(entries.items()):
                if lint in groups or lint == "warnings" or level == group_level:
                    continue
                if priority == group_priority:
                    winner = group if group > lint else lint
                    outcome.findings.append(
                        f"{where}: group {group} ({group_level}) and lint {lint} ({level}) share "
                        f"priority {priority}, so Cargo orders them by name and {winner} wins; give "
                        f"the group `priority = -1`"
                    )
    return outcome


RESTRICTION_ATTRIBUTE = re.compile(r"clippy\s*::\s*restriction")
RESTRICTION_FLAG = re.compile(
    r"-[WDF]\s*clippy::restriction|--(?:warn|deny|forbid)[=\s]+clippy::restriction"
)


def check_no_restriction_group(tree: Tree) -> Outcome:
    outcome = Outcome()
    tables = lint_tables(tree)
    outcome.examined = len(tables) + tree.workspace.manifests_read
    for where, tool, table in tables:
        if tool != "clippy":
            continue
        level, _ = lint_level(table.get("restriction"))
        if level in RAISED:
            outcome.findings.append(
                f"{where}: the whole restriction group is set to {level}"
            )
    for crate in tree.workspace.crates:
        for path in crate_roots(crate):
            source = tree.source(path)
            for lint, level in crate_attributes(source).items():
                if lint == "clippy::restriction" and level in RAISED:
                    outcome.findings.append(
                        f"{source.rel}: #![{level}(clippy::restriction)] enables the whole group"
                    )
    for where, text in ci_texts(tree.root):
        for line in logical_lines(text):
            if "clippy" in line and RESTRICTION_FLAG.search(line):
                outcome.findings.append(
                    f"{where}: `{line.strip()[:80]}` enables the whole restriction group"
                )
    return outcome


CLIPPY_COMMAND = re.compile(r"\bcargo\s+(?:\+\S+\s+)?clippy\b|\"clippy\"")
FMT_COMMAND = re.compile(r"\bcargo\s+(?:\+\S+\s+)?fmt\b|\brustfmt\b|\"fmt\"")
CHECK_FLAG = re.compile(r"--check\b|\"--check\"")


def check_clippy_ci(tree: Tree) -> Outcome:
    outcome = Outcome()
    ran, read = ci_runs(tree.root, CLIPPY_COMMAND, WARNINGS_DENIED, RUSTFLAGS_DENIED)
    outcome.examined = read + tree.workspace.manifests_read
    if not ran:
        outcome.findings.append(
            f"no CI workflow runs cargo clippy with warnings denied (-D warnings) ({read} CI file(s) read)"
        )
    return outcome


def check_fmt_ci(tree: Tree) -> Outcome:
    outcome = Outcome()
    ran, read = ci_runs(tree.root, FMT_COMMAND, CHECK_FLAG, None)
    outcome.examined = read + tree.workspace.manifests_read
    if not ran:
        outcome.findings.append(
            f"no CI workflow runs cargo fmt --check ({read} CI file(s) read)"
        )
    return outcome


def check_panic_lints(tree: Tree) -> Outcome:
    outcome = Outcome()
    for crate in tree.workspace.crates:
        outcome.examined += 1
        clippy = tree.workspace.crate_lints(crate).get("clippy", {})
        level, _ = lint_level(
            clippy.get("unwrap_used") if isinstance(clippy, dict) else None
        )
        if level in RAISED:
            continue
        attributes = [
            crate_attributes(tree.source(path)).get("clippy::unwrap_used")
            for path in crate_roots(crate)
        ]
        if attributes and all(value in RAISED for value in attributes):
            continue
        outcome.findings.append(
            f"crate {crate.name} ({crate.rel}) leaves clippy::unwrap_used at allow: a failed unwrap "
            "panics the request or the service"
        )
    return outcome


# --------------------------------------------------------------------------- stage `errors`


PUBLIC_FN = re.compile(
    r"\bpub\s+(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+\"[^\"]*\"\s+)?fn\s+(\w+)"
)
PUBLIC_ALIAS = re.compile(r"\bpub\s+type\s+(\w+)[^=;]*=\s*anyhow\s*::")
ANYHOW_RESULT_IMPORT = re.compile(
    r"\buse\s+anyhow\s*::\s*(?:Result\b|\{[^}]*\bResult\b[^}]*\})"
)


def return_type(bare: str, name_end: int) -> str:
    """The text after `->` of the fn whose name ends at `name_end`, up to its body or `where`."""
    cursor = name_end
    if cursor < len(bare) and bare[cursor] == "<":
        cursor = matching(bare, cursor)
    open_paren = bare.find("(", cursor)
    if open_paren < 0:
        return ""
    after = matching(bare, open_paren)
    stop = len(bare)
    for marker in ("{", ";", " where "):
        found = bare.find(marker, after)
        if 0 <= found < stop:
            stop = found
    head = bare[after:stop]
    arrow = head.find("->")
    return head[arrow + 2 :].strip() if arrow >= 0 else ""


def single_argument_result(text: str) -> bool:
    if not text.startswith("Result"):
        return False
    rest = text[len("Result") :].strip()
    if not rest.startswith("<"):
        return False
    inner = rest[1 : matching(rest, 0) - 1]
    depth = 0
    for char in inner:
        if char in "<([":
            depth += 1
        elif char in ">)]":
            depth -= 1
        elif char == "," and depth == 0:
            return False
    return True


def check_typed_lib_errors(tree: Tree) -> Outcome:
    outcome = Outcome()
    for crate in tree.workspace.crates:
        if crate.lib is None:
            continue
        for path in tree.lib_files(crate):
            source = tree.source(path)
            outcome.examined += 1
            imports_result = bool(ANYHOW_RESULT_IMPORT.search(source.bare))
            for function in PUBLIC_FN.finditer(source.bare):
                returned = return_type(source.bare, function.end())
                anyhow_type = "anyhow" in returned.replace(" ", "")
                if anyhow_type or (imports_result and single_argument_result(returned)):
                    outcome.findings.append(
                        f"{source.place(function.start())}: library fn {function.group(1)} returns "
                        f"anyhow ({returned[:60]}); give callers a typed error (thiserror)"
                    )
            for alias in PUBLIC_ALIAS.finditer(source.bare):
                outcome.findings.append(
                    f"{source.place(alias.start())}: library type {alias.group(1)} aliases anyhow in the public API"
                )
    return outcome


def check_catch_panic(tree: Tree) -> Outcome:
    outcome = Outcome()
    services = http_services(tree)
    outcome.examined = len(services) or tree.workspace.manifests_read
    for crate, name, sources in services:
        if not any_names(sources, "CatchPanicLayer", ".catch_panic("):
            outcome.findings.append(
                f"binary {name} ({crate.rel}) serves HTTP without CatchPanicLayer: a handler panic "
                "drops the connection and no response or request event records it"
            )
    return outcome


# --------------------------------------------------------------------------- stage `http`


ROUTE_CALL = re.compile(r"\.\s*(route|route_service|nest|nest_service)\s*\(\s*")


def major_minor(text: str) -> tuple[int, int] | None:
    found = re.search(r"(\d+)(?:\.(\d+))?", text)
    if not found:
        return None
    return int(found.group(1)), int(found.group(2) or 0)


def axum_is_08(tree: Tree, crate: Crate) -> bool:
    """Whether the crate's axum is 0.8 or later: the lockfile's versions first, then its
    requirement. With no version to read, the house pin (0.8) is assumed."""
    locked = [major_minor(version) for version in tree.workspace.lock.get("axum", [])]
    locked = [version for version in locked if version]
    if locked:
        return max(locked) >= (0, 8)
    requirement = str(crate.deps.get("axum", {}).get("version", ""))
    version = major_minor(requirement)
    return version is None or version >= (0, 8)


def check_axum_route_syntax(tree: Tree) -> Outcome:
    outcome = Outcome()
    for crate in tree.workspace.crates:
        outcome.examined += 1
        if not tree.depends_on(crate, "axum") or not axum_is_08(tree, crate):
            continue
        for source in tree.crate_sources(crate):
            if source.names("without_v07_checks"):
                continue
            # `code` keeps strings, so the pattern stops where the path literal starts.
            for call in ROUTE_CALL.finditer(source.code):
                literal = source.literal_at(call.end())
                if literal is None:
                    continue
                retired = [
                    part
                    for part in literal.value.split("/")
                    if part.startswith((":", "*"))
                ]
                if retired:
                    outcome.findings.append(
                        f'{source.place(call.start())}: .{call.group(1)}("{literal.value}") uses the '
                        f"retired capture {retired[0]}; axum 0.8 panics building it (write "
                        f"{{{retired[0][1:]}}})"
                    )
    return outcome


# A whole-request timeout. `RequestBodyTimeoutLayer` bounds only the gap between body chunks.
REQUEST_TIMEOUT = re.compile(r"(?<![A-Za-z0-9_])TimeoutLayer\b")


def check_request_timeout(tree: Tree) -> Outcome:
    outcome = Outcome()
    services = http_services(tree)
    outcome.examined = len(services) or tree.workspace.manifests_read
    for crate, name, sources in services:
        bounded = any(REQUEST_TIMEOUT.search(source.body) for source in sources) or (
            any_names(sources, "ServiceBuilder")
            and any(re.search(r"\.\s*timeout\s*\(", s.body) for s in sources)
        )
        if not bounded:
            outcome.findings.append(
                f"binary {name} ({crate.rel}) serves HTTP with no request timeout: a slow request "
                "holds its memory for as long as the client likes"
            )
    return outcome


# --------------------------------------------------------------------------- stage `lifecycle`


def check_graceful_shutdown(tree: Tree) -> Outcome:
    outcome = Outcome()
    for source in tree.all_sources():
        for site in serve_sites(source):
            outcome.examined += 1
            end = statement_end(source.bare, site)
            if ".with_graceful_shutdown(" not in source.bare[site:end].replace(
                " ", ""
            ).replace("\n", ""):
                outcome.findings.append(
                    f"{source.place(site)}: axum::serve without .with_graceful_shutdown(..): a stop "
                    "drops every in-flight request"
                )
    if outcome.examined == 0:
        outcome.examined = tree.workspace.manifests_read
    return outcome


SIGNALS = {
    "SIGTERM": ("SignalKind::terminate", "SIGTERM"),
    "SIGINT": ("ctrl_c", "SignalKind::interrupt", "SIGINT"),
    "SIGQUIT": ("SignalKind::quit", "SIGQUIT"),
    "SIGHUP": ("SignalKind::hangup", "SIGHUP"),
}


def service_binaries(tree: Tree) -> list[tuple[Crate, str, object | None]]:
    """Every long-running unit's workspace binary, and every HTTP service binary without one."""
    out: list[tuple[Crate, str, object | None]] = []
    seen: set[str] = set()
    lint = durable()
    for unit in tree.service_units():
        target = tree.unit_binary(unit)
        if target is None or not lint.long_running(unit):
            continue
        crate, name = target
        out.append((crate, name, unit))
        seen.add(name)
    for crate, name, _ in http_services(tree):
        if name not in seen:
            out.append((crate, name, None))
            seen.add(name)
    return out


def check_sigterm_handled(tree: Tree) -> Outcome:
    outcome = Outcome()
    binaries = service_binaries(tree)
    outcome.examined = len(binaries) or tree.workspace.manifests_read
    for crate, name, unit in binaries:
        signal = "SIGTERM"
        if unit is not None:
            declared = (unit.last("Service", "KillSignal") or "").strip().upper()
            if declared:
                signal = declared if declared.startswith("SIG") else f"SIG{declared}"
        tokens = SIGNALS.get(signal, (signal,))
        sources = tree.binary_sources(crate, name)
        if any_names(sources, *tokens):
            continue
        by = f"unit {unit.rel}" if unit is not None else "its HTTP server"
        also = (
            " (it handles only ctrl_c, which is SIGINT)"
            if signal == "SIGTERM" and any_names(sources, "ctrl_c")
            else ""
        )
        outcome.findings.append(
            f"binary {name} ({crate.rel}) handles no {signal}, the signal {by} stops it with{also}"
        )
    return outcome


def notify_units(
    tree: Tree, predicate: Callable[[object], bool]
) -> list[tuple[object, Crate, str]]:
    out = []
    for unit in tree.service_units():
        target = tree.unit_binary(unit)
        if target is not None and predicate(unit):
            out.append((unit, *target))
    return out


def check_notify_ready(tree: Tree) -> Outcome:
    outcome = Outcome()
    lint = durable()
    units = notify_units(
        tree, lambda unit: lint.service_type(unit) in lint.NOTIFY_TYPES
    )
    outcome.examined = (
        len(units) or len(tree.service_units()) or tree.workspace.manifests_read
    )
    for unit, crate, name in units:
        sources = tree.binary_sources(crate, name)
        if any_names(sources, "NotifyState::Ready") or any_says(sources, "READY=1"):
            continue
        outcome.findings.append(
            f"{unit.rel} is Type={lint.service_type(unit)}, but binary {name} never sends READY=1: "
            "its start times out"
        )
    return outcome


def watchdog_on(unit) -> bool:
    value = (unit.last("Service", "WatchdogSec") or "").strip()
    if not value or value == "infinity":
        return False
    seconds = durable().seconds(value)
    return seconds is None or seconds > 0


def check_watchdog_ping(tree: Tree) -> Outcome:
    outcome = Outcome()
    units = notify_units(tree, watchdog_on)
    outcome.examined = (
        len(units) or len(tree.service_units()) or tree.workspace.manifests_read
    )
    for unit, crate, name in units:
        sources = tree.binary_sources(crate, name)
        if any_names(sources, "NotifyState::Watchdog") or any_says(
            sources, "WATCHDOG=1"
        ):
            continue
        outcome.findings.append(
            f"{unit.rel} sets WatchdogSec={unit.last('Service', 'WatchdogSec')}, but binary {name} "
            "never sends WATCHDOG=1: systemd kills it as hung"
        )
    return outcome


# --------------------------------------------------------------------------- stage `config`


ENV_READS = (
    (re.compile(r"\benv\s*::\s*var(?:_os)?\s*\(\s*$"), "reads it at run time"),
    (
        re.compile(r"\b(?:option_)?env\s*!\s*\(\s*$"),
        "env!/option_env! bakes it into the binary",
    ),
    (re.compile(r"\bdotenvy?\s*::\s*var\s*\(\s*$"), "reads it at run time"),
)
CLAP_ENV = re.compile(r"\benv\s*=\s*$")
REFERENCE_SUFFIXES = ("_FILE", "_PATH", "_DIR", "_DIRECTORY")


def secret_variable(name: str) -> bool:
    lint = durable()
    upper = name.upper()
    if upper == "CREDENTIALS_DIRECTORY" or upper.endswith(REFERENCE_SUFFIXES):
        return False
    return bool(lint.SECRET_NAME.search(name)) and not lint.REFERENCE_NAME.search(name)


def env_reads(source: Source) -> list[tuple[int, str, str]]:
    """(offset, variable, how) for every environment read whose name is a string literal."""
    out: list[tuple[int, str, str]] = []
    for literal in source.literals:
        head = source.bare[max(0, literal.start - 40) : literal.start]
        kinds = [how for pattern, how in ENV_READS if pattern.search(head)]
        if kinds:
            out.append((literal.start, literal.value, kinds[0]))
            continue
        if CLAP_ENV.search(head):
            attribute_start = source.code.rfind("#[", 0, literal.start)
            if attribute_start >= 0 and re.match(
                r"#\[\s*(?:arg|clap)\s*\(", source.code[attribute_start:]
            ):
                out.append(
                    (literal.start, literal.value, "clap's env = fallback reads it")
                )
    return out


def check_no_secret_env(tree: Tree) -> Outcome:
    outcome = Outcome()
    for source in tree.all_sources():
        outcome.examined += 1
        for offset, name, how in env_reads(source):
            if secret_variable(name):
                outcome.findings.append(
                    f"{source.place(offset)}: {name} is a secret read from the environment ({how}); "
                    "systemd.exec(5): environment variables are not suitable for passing secrets. "
                    "Read it from $CREDENTIALS_DIRECTORY (LoadCredential=)"
                )
    if outcome.examined == 0:
        outcome.examined = tree.workspace.manifests_read
    return outcome


CREDENTIAL_KEYS = (
    "LoadCredential",
    "LoadCredentialEncrypted",
    "SetCredential",
    "SetCredentialEncrypted",
    "ImportCredential",
)


def check_credentials_read(tree: Tree) -> Outcome:
    outcome = Outcome()
    units = notify_units(
        tree, lambda unit: any(unit.values("Service", key) for key in CREDENTIAL_KEYS)
    )
    outcome.examined = (
        len(units) or len(tree.service_units()) or tree.workspace.manifests_read
    )
    for unit, crate, name in units:
        passed = " ".join(
            unit.values("Service", "ExecStart") + unit.values("Service", "Environment")
        )
        if "%d/" in passed or "CREDENTIALS_DIRECTORY" in passed:
            continue
        sources = tree.binary_sources(crate, name)
        if any_says(sources, "CREDENTIALS_DIRECTORY", "/run/credentials/"):
            continue
        loaded = [
            value.split(":")[0]
            for key in CREDENTIAL_KEYS
            for value in unit.values("Service", key)
        ]
        outcome.findings.append(
            f"{unit.rel} loads credential(s) {', '.join(loaded)}, but binary {name} never reads "
            "$CREDENTIALS_DIRECTORY and its command passes no %d/ path"
        )
    return outcome


# --------------------------------------------------------------------------- stage `memory`


UNBOUNDED = (
    (
        re.compile(r"\bunbounded_channel\s*(?:::<[^>]*>\s*)?\("),
        "tokio unbounded_channel",
    ),
    (
        re.compile(
            r"\b(?:mpsc|flume|async_channel|crossbeam_channel|channel|kanal)\s*::\s*unbounded(?:_async)?\s*(?:::<[^>]*>\s*)?\("
        ),
        "an unbounded channel",
    ),
    (
        re.compile(r"\bmpsc\s*::\s*channel\s*(?:::<[^>]*>\s*)?\(\s*\)"),
        "std::sync::mpsc::channel(), which is unbounded",
    ),
)
STD_CHANNEL_IMPORT = re.compile(
    r"\buse\s+std\s*::\s*sync\s*::\s*mpsc\s*::\s*(?:channel\b|\{[^}]*\bchannel\b[^}]*\})"
)
BARE_STD_CHANNEL = re.compile(r"(?<![\w:.])channel\s*(?:::<[^>]*>\s*)?\(\s*\)")


def check_bounded_channels(tree: Tree) -> Outcome:
    outcome = Outcome()
    for source in tree.all_sources():
        outcome.examined += 1
        hits = [
            (match.start(), what)
            for pattern, what in UNBOUNDED
            for match in pattern.finditer(source.bare)
        ]
        if STD_CHANNEL_IMPORT.search(source.bare):
            hits.extend(
                (match.start(), "std::sync::mpsc::channel(), which is unbounded")
                for match in BARE_STD_CHANNEL.finditer(source.bare)
            )
        for offset, what in sorted(set(hits)):
            outcome.findings.append(
                f"{source.place(offset)}: {what}: a slow receiver grows it until the kernel's OOM "
                "killer ends the process; use a bounded channel"
            )
    if outcome.examined == 0:
        outcome.examined = tree.workspace.manifests_read
    return outcome


UNLIMITED_CALLS = (
    (
        re.compile(r"\bto_bytes\s*\("),
        "usize::MAX",
        "to_bytes(.., usize::MAX) buffers a body of any size",
    ),
    (
        re.compile(r"\bLimited\s*::\s*new\s*\("),
        "usize::MAX",
        "Limited::new(.., usize::MAX) is no limit",
    ),
    (
        re.compile(r"\bDefaultBodyLimit\s*::\s*max\s*\("),
        "usize::MAX",
        "DefaultBodyLimit::max(usize::MAX) disables the limit",
    ),
    (
        re.compile(r"\bRequestBodyLimitLayer\s*::\s*new\s*\("),
        "usize::MAX",
        "RequestBodyLimitLayer::new(usize::MAX) is no limit",
    ),
)
HYPER_TO_BYTES = re.compile(r"\bhyper\s*::\s*body\s*::\s*to_bytes\s*\(")


def check_bounded_body_reads(tree: Tree) -> Outcome:
    outcome = Outcome()
    for source in tree.all_sources():
        outcome.examined += 1
        for pattern, needle, what in UNLIMITED_CALLS:
            for call in pattern.finditer(source.bare):
                paren = source.bare.find("(", call.start())
                arguments = source.bare[paren : matching(source.bare, paren)]
                if needle in arguments.replace(" ", ""):
                    outcome.findings.append(f"{source.place(call.start())}: {what}")
        for call in HYPER_TO_BYTES.finditer(source.bare):
            outcome.findings.append(
                f"{source.place(call.start())}: hyper::body::to_bytes reads a body with no limit"
            )
    if outcome.examined == 0:
        outcome.examined = tree.workspace.manifests_read
    return outcome


CONCURRENCY_BOUNDS = (
    "ConcurrencyLimitLayer",
    "GlobalConcurrencyLimitLayer",
    ".concurrency_limit(",
    "LoadShedLayer",
    ".load_shed(",
    "ConcurrencyLimit::new",
)


def check_concurrency_bound(tree: Tree) -> Outcome:
    outcome = Outcome()
    services = http_services(tree)
    outcome.examined = len(services) or tree.workspace.manifests_read
    for crate, name, sources in services:
        compact = [re.sub(r"\s+", "", source.body) for source in sources]
        bounded = any(
            token.replace(" ", "") in text
            for text in compact
            for token in CONCURRENCY_BOUNDS
        )
        gated = any(
            "Semaphore::new(" in text and ".acquire" in text for text in compact
        )
        if not (bounded or gated):
            outcome.findings.append(
                f"binary {name} ({crate.rel}) serves HTTP with no concurrency bound "
                "(ConcurrencyLimitLayer, GlobalConcurrencyLimitLayer or a Semaphore): a burst's "
                "memory has no ceiling below MemoryMax"
            )
    return outcome


ASYNC_BODY = re.compile(r"\basync\s+(?:move\s+)?\{|\basync\s+(?:unsafe\s+)?fn\b")
BLOCKING = re.compile(r"\b(?:std\s*::\s*)?thread\s*::\s*sleep\s*\(|\bblock_on\s*\(")
SYNC_REGIONS = re.compile(
    r"\b(?:spawn_blocking|block_in_place)\s*\(|\bthread\s*::\s*spawn\s*\("
)


def async_bodies(bare: str) -> list[tuple[int, int]]:
    spans: list[tuple[int, int]] = []
    for match in ASYNC_BODY.finditer(bare):
        if match.group(0).endswith("{"):
            opening = match.end() - 1
        else:
            paren = bare.find("(", match.end())
            if paren < 0:
                continue
            after = matching(bare, paren)
            opening = bare.find("{", after)
            semicolon = bare.find(";", after)
            if opening < 0 or (0 <= semicolon < opening):
                continue
        spans.append((opening, matching(bare, opening)))
    return spans


def check_no_blocking_in_async(tree: Tree) -> Outcome:
    outcome = Outcome()
    for source in tree.all_sources():
        outcome.examined += 1
        spans = async_bodies(source.bare)
        excluded = []
        for region in SYNC_REGIONS.finditer(source.bare):
            paren = source.bare.find("(", region.start())
            excluded.append((paren, matching(source.bare, paren)))
        reported: set[int] = set()
        for start, end in spans:
            for call in BLOCKING.finditer(source.bare, start, end):
                offset = call.start()
                if offset in reported or any(
                    low <= offset < high for low, high in excluded
                ):
                    continue
                reported.add(offset)
                outcome.findings.append(
                    f"{source.place(offset)}: {call.group(0).rstrip('(').strip()} inside an async body "
                    "blocks a runtime worker; use tokio::time::sleep or spawn_blocking"
                )
    if outcome.examined == 0:
        outcome.examined = tree.workspace.manifests_read
    return outcome


# --------------------------------------------------------------------------- the command line


CHECKS: dict[str, Callable[[Tree], Outcome]] = {
    "rs.unsafe-forbidden": check_unsafe_forbidden,
    "rs.lints-inherited": check_lints_inherited,
    "rs.lint-group-priority": check_lint_group_priority,
    "rs.no-restriction-group": check_no_restriction_group,
    "rs.clippy-ci": check_clippy_ci,
    "rs.fmt-ci": check_fmt_ci,
    "rs.panic-lints": check_panic_lints,
    "rs.typed-lib-errors": check_typed_lib_errors,
    "rs.catch-panic": check_catch_panic,
    "rs.axum-route-syntax": check_axum_route_syntax,
    "rs.request-timeout": check_request_timeout,
    "rs.graceful-shutdown": check_graceful_shutdown,
    "rs.sigterm-handled": check_sigterm_handled,
    "rs.notify-ready": check_notify_ready,
    "rs.watchdog-ping": check_watchdog_ping,
    "rs.no-secret-env": check_no_secret_env,
    "rs.credentials-read": check_credentials_read,
    "rs.bounded-channels": check_bounded_channels,
    "rs.bounded-body-reads": check_bounded_body_reads,
    "rs.concurrency-bound": check_concurrency_bound,
    "rs.no-blocking-in-async": check_no_blocking_in_async,
}
CLASSES = tuple(CHECKS)


def run_check(root: Path, name: str) -> int:
    """Run one class, print its findings and `examined N`, and return its exit."""
    try:
        outcome = CHECKS[name](Tree(root))
    except Void as void:
        print(f"{name}: VOID {void}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    top = argparse.ArgumentParser(
        description="Judge a Rust service workspace against the rust-service pack.",
        parents=[common],
    )
    verbs = top.add_subparsers(dest="verb", required=True)
    check = verbs.add_parser("check", parents=[common], help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    verbs.add_parser("classes", help="print the classes, one per line")
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for name in CLASSES:
            print(name)
        return EXIT_GREEN
    root = Path(getattr(args, "root", Path("."))).resolve()
    if not root.is_dir():
        print(f"rust-service-probe: --root {root} is not a directory", file=sys.stderr)
        return EXIT_USAGE
    if tomllib is None:
        print(f"{args.klass}: VOID Python 3.11 or newer is required (tomllib)")
        print("examined 0")
        return EXIT_VOID
    return run_check(root, args.klass)


if __name__ == "__main__":
    sys.exit(main())

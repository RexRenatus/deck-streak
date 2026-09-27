#!/usr/bin/env python3
"""stack-probe.py -- judge a repository's declared stack against the stack-selection radar.

SPEC-V2-2198 / ADR-V2-2198. The owner's words, 2026-09-27: "lets brainstorm how our packs
determine what coding language and framework it uses to develop modern apps and webpages", then
"A stack-selection pack". The decision lives in `skills/packs/stack-selection/`: a decision
procedure and a rubric in SKILL.md, golden paths and a DATED tech radar in `radar.json`. This
script is the check that keeps a repository on the stack its architect chose, and it enforces
every owner decision a tree can show: holds refused, pinned majors, Svelte's runes mode and
svelte-check in CI, and no Corepack.

It is VENDORABLE on purpose: standard library only (Python >= 3.11, for `tomllib`), no import from
this repository, and every path it reads is under `--root` or named by `--radar`. A project may
copy it next to a copy of the radar and run it in its own CI; the pack's rows run it from here, as
`{skills}/../scripts/stack-probe.py`, against any tree.

usage:
  stack-probe.py [--root DIR] [--radar FILE] [--today YYYY-MM-DD] check CLASS
  stack-probe.py [--root DIR] [--radar FILE] detect
  stack-probe.py classes

CLASS is one of:
  stack-declared          the tree declares its stack in a root `stack.json`, well formed
  stack-matches-detected  what the manifests, lockfiles and build configs use is what it declares
  no-hold-items           nothing declared or detected sits in the radar's hold ring
  deviation-has-adr       every element off its golden path cites an ADR that exists and names it
  pinned-majors           every pinned package, Node and pnpm, is on the major the radar pins
  svelte-runes            every package that uses Svelte sets `compilerOptions.runes: true`
  svelte-check-ci         a CI workflow runs svelte-check for every package that uses Svelte
  no-corepack             nothing runs Corepack, and a pnpm tree pins `packageManager`
  radar-fresh             the radar was refreshed within 35 days (the pack runs it advisory)
  radar-approvals         the owner approved every adopt and hold entry, and the radar is well formed

The radar is `--radar FILE`, else the `radar` path `stack.json` names (relative to the root), else
the pack's own radar beside this script. Output is one line per finding, then `examined N`, where N
is the population the class read. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID:
nothing was examined, or an input could not be read, which is never a pass.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11: `main` reports VOID, never a crash.
    tomllib = None

STACK_FILE = "stack.json"
STACK_SCHEMA = "phx.stack.v1"
RADAR_SCHEMA = "phx.stack.radar.v1"
PACK_RADAR = (
    Path(__file__).resolve().parent.parent
    / "skills"
    / "packs"
    / "stack-selection"
    / "radar.json"
)

RINGS = ("adopt", "trial", "assess", "hold")
# The owner's rule (2026-09-27): "You approve adopt/hold moves". A refresh moves assess <-> trial
# on its own evidence; a move INTO either of these needs the owner's recorded approval.
APPROVED_RINGS = ("adopt", "hold")
# The owner's cadence is "Monthly radar refresh"; five days of grace past a month.
FRESH_DAYS = 35

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

MANIFESTS = ("Cargo.toml", "package.json")
LOCKFILES = (
    "Cargo.lock",
    "pnpm-lock.yaml",
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
)
# The build configs a Svelte, SvelteKit or Astro package sets compiler options in.
CONFIG_STEMS = ("svelte.config", "vite.config", "astro.config")
CONFIG_SUFFIXES = (".js", ".mjs", ".cjs", ".ts", ".mts", ".cts")
CI_DIRS = (".github/workflows", ".depot/workflows")
NODE_PIN_FILES = (".nvmrc", ".node-version")
PNPM_WORKSPACE = "pnpm-workspace.yaml"
DETECT_KEYS = ("cargo", "npm", "lockfile", "manifest", "config")
CARGO_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
NPM_FIELDS = (
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
)
# Build output and installed packages, never the stack's own manifests. Every directory whose name
# starts with "." (tool state: .git, .svelte-kit, .venv) is skipped as well; CI workflows are read
# from their two fixed directories instead.
SKIPPED_DIRS = frozenset(
    {"node_modules", "target", "dist", "build", "coverage", "__pycache__", "venv"}
)

STACK_KEYS = frozenset({"schema", "radar", "app_types", "elements", "exclude", "note"})
ELEMENT_KEYS = frozenset({"item", "adr", "planned", "pins", "note"})
RADAR_KEYS = frozenset(
    {"schema", "refreshed_at", "cadence", "rubric", "golden_paths", "items", "note"}
)
PATH_KEYS = frozenset({"id", "name", "applies_to", "items", "owner_approval"})
ITEM_KEYS = frozenset(
    {
        "id",
        "name",
        "category",
        "ring",
        "since",
        "pins",
        "current",
        "context7",
        "scores",
        "detect",
        "evidence",
        "rationale",
        "owner_approval",
        "note",
    }
)
APPROVAL_KEYS = frozenset({"ring", "words", "date", "via"})
SCORE_KEYS = ("ai", "fit", "maturity", "modern")
SLUG = re.compile(r"^[a-z0-9][a-z0-9.-]*$")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
SERIES = re.compile(r"^\d+(\.\d+)?$")
# The first version a requirement or range names: `^0.8.4` -> 0, 8; `~1.53` -> 1, 53; `5.x` -> 5.
VERSION = re.compile(r"(?<![\w.])v?(\d+)(?:\.(\d+|[xX*]))?")
CATALOG_LINE = re.compile(r"^(\s*)(['\"]?)([^'\":#]+?)\2\s*:\s*(.*?)\s*(?:#.*)?$")

CLASSES = (
    "stack-declared",
    "stack-matches-detected",
    "no-hold-items",
    "deviation-has-adr",
    "pinned-majors",
    "svelte-runes",
    "svelte-check-ci",
    "no-corepack",
    "radar-fresh",
    "radar-approvals",
)


class Void(Exception):
    """An input the class needs cannot be read, so the class examined nothing."""


@dataclass(frozen=True)
class Context:
    """What every class reads: the judged tree, the radar's location and today's date."""

    root: Path
    radar_flag: Path | None
    today: dt.date


@dataclass(frozen=True)
class Outcome:
    """A class's findings and the size of the population it examined."""

    findings: list[str]
    examined: int


def parse_date(text: object) -> dt.date | None:
    """An ISO `YYYY-MM-DD` date, or None for anything else."""
    if not isinstance(text, str) or not DATE.match(text):
        return None
    try:
        return dt.date.fromisoformat(text)
    except ValueError:
        return None


def read_json(path: Path, what: str) -> object:
    """The parsed JSON at `path`; `Void` naming `what` when it is absent or not JSON."""
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        raise Void(f"no {what} at {path}") from None
    except (OSError, UnicodeDecodeError) as error:
        raise Void(f"cannot read {what} at {path}: {error}") from None
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise Void(f"{what} at {path} is not JSON: {error}") from None


def string_list(value: object) -> list[str]:
    """`value`'s string members when it is a list, else nothing."""
    if not isinstance(value, list):
        return []
    return [member for member in value if isinstance(member, str)]


def string_map(value: object) -> dict[str, str]:
    """`value`'s string-to-string entries when it is an object, else nothing."""
    if not isinstance(value, dict):
        return {}
    return {
        key: entry
        for key, entry in value.items()
        if isinstance(key, str) and isinstance(entry, str)
    }


# --------------------------------------------------------------------------- stack.json


def load_stack(root: Path) -> dict:
    """`<root>/stack.json` as an object, or `Void`."""
    document = read_json(root / STACK_FILE, STACK_FILE)
    if not isinstance(document, dict):
        raise Void(f"{STACK_FILE} is not a JSON object")
    return document


def stack_elements(stack: dict) -> list[dict]:
    """The elements that name an item; `Void` when there is no element list to judge."""
    elements = stack.get("elements")
    if not isinstance(elements, list):
        raise Void(f"{STACK_FILE} declares no `elements` list")
    return [
        element
        for element in elements
        if isinstance(element, dict) and isinstance(element.get("item"), str)
    ]


def stack_excludes(stack: dict | None) -> list[str]:
    """The tree-relative directory prefixes the scan skips, normalised to `/` without ends."""
    if stack is None:
        return []
    return [
        str(PurePosixPath(prefix)).strip("/")
        for prefix in string_list(stack.get("exclude"))
        if prefix.strip("/")
    ]


# --------------------------------------------------------------------------- the radar


def radar_path(context: Context, stack: dict | None) -> Path:
    """`--radar`, else the radar `stack.json` names under the root, else the pack's own."""
    if context.radar_flag is not None:
        return context.radar_flag
    if stack is not None and isinstance(stack.get("radar"), str):
        return context.root / stack["radar"]
    return PACK_RADAR


def load_radar(path: Path) -> dict:
    """The radar document, or `Void` when it cannot be judged at all."""
    document = read_json(path, "radar")
    if not isinstance(document, dict):
        raise Void(f"the radar at {path} is not a JSON object")
    if document.get("schema") != RADAR_SCHEMA:
        raise Void(
            f"the radar at {path} is schema {document.get('schema')!r}, not {RADAR_SCHEMA}"
        )
    items = document.get("items")
    if not isinstance(items, list) or not items:
        raise Void(f"the radar at {path} holds no items")
    return document


def radar_index(radar: dict) -> dict[str, dict]:
    """Every item with a string id, by id; the first of two with one id wins."""
    index: dict[str, dict] = {}
    for item in radar.get("items", []):
        if isinstance(item, dict) and isinstance(item.get("id"), str):
            index.setdefault(item["id"], item)
    return index


def golden_paths(radar: dict) -> list[dict]:
    """The radar's golden paths that are objects."""
    paths = radar.get("golden_paths")
    if not isinstance(paths, list):
        return []
    return [path for path in paths if isinstance(path, dict)]


def golden_items(radar: dict, types: list[str]) -> set[str]:
    """The items of every golden path that applies to one of `types`."""
    return {
        item_id
        for path in golden_paths(radar)
        if set(string_list(path.get("applies_to"))) & set(types)
        for item_id in string_list(path.get("items"))
    }


# --------------------------------------------------------------------------- the tree


@dataclass
class Manifest:
    """One Cargo.toml or package.json, as far as the classes read it."""

    path: str
    kind: str
    names: dict[str, str | None] = field(default_factory=dict)
    workspace: dict[str, str | None] | None = None
    scripts: dict[str, str] = field(default_factory=dict)
    package_manager: str | None = None
    engines_node: str | None = None


@dataclass
class Tree:
    """Everything the scan read: manifests, lockfiles, build configs and CI workflows."""

    manifests: list[Manifest] = field(default_factory=list)
    lockfiles: list[str] = field(default_factory=list)
    configs: dict[str, str] = field(default_factory=dict)
    workflows: dict[str, str] = field(default_factory=dict)
    unreadable: list[str] = field(default_factory=list)

    @property
    def files_read(self) -> int:
        """How many files the scan read, readable or not."""
        return (
            len(self.manifests)
            + len(self.lockfiles)
            + len(self.configs)
            + len(self.workflows)
            + len(self.unreadable)
        )


def excluded(relative: str, excludes: list[str]) -> bool:
    """Whether the tree-relative directory `relative` is under one of `excludes`."""
    return any(
        relative == prefix or relative.startswith(prefix + "/") for prefix in excludes
    )


def is_config(name: str) -> bool:
    """Whether `name` is a Svelte, Vite or Astro build config."""
    stem, dot, suffix = name.rpartition(".")
    return bool(dot) and stem in CONFIG_STEMS and f".{suffix}" in CONFIG_SUFFIXES


def dependency_requirement(spec: object) -> str | None:
    """A Cargo dependency's version requirement, when it states one."""
    if isinstance(spec, str):
        return spec
    if isinstance(spec, dict) and isinstance(spec.get("version"), str):
        return spec["version"]
    return None


def cargo_dependencies(document: dict) -> dict[str, str | None]:
    """Every crate a Cargo manifest depends on, by real name, with its requirement.

    A `package = "real"` rename is resolved to the real name, and a `workspace = true` entry keeps
    the requirement `None` until the workspace resolves it.
    """
    tables: list[object] = [document.get(table) for table in CARGO_TABLES]
    targets = document.get("target")
    if isinstance(targets, dict):
        for platform in targets.values():
            if isinstance(platform, dict):
                tables.extend(platform.get(table) for table in CARGO_TABLES)
    names: dict[str, str | None] = {}
    for table in tables:
        if not isinstance(table, dict):
            continue
        for key, spec in table.items():
            renamed = spec.get("package") if isinstance(spec, dict) else None
            name = renamed if isinstance(renamed, str) else key
            names.setdefault(name, dependency_requirement(spec))
    return names


def read_cargo(root: Path, path: Path) -> Manifest:
    """A Cargo manifest's dependencies, and its workspace's when it declares one."""
    document = tomllib.loads(path.read_text(encoding="utf-8"))
    manifest = Manifest(path.relative_to(root).as_posix(), "cargo")
    manifest.names = cargo_dependencies(document)
    workspace = document.get("workspace")
    if isinstance(workspace, dict):
        shared = workspace.get("dependencies")
        manifest.workspace = {}
        if isinstance(shared, dict):
            for key, spec in shared.items():
                renamed = spec.get("package") if isinstance(spec, dict) else None
                name = renamed if isinstance(renamed, str) else key
                manifest.workspace[name] = dependency_requirement(spec)
    return manifest


def read_npm(root: Path, path: Path) -> Manifest:
    """A package.json's dependencies, scripts, `packageManager` and `engines.node`."""
    document = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("not a JSON object")
    manifest = Manifest(path.relative_to(root).as_posix(), "npm")
    for field_name in NPM_FIELDS:
        for key, spec in string_map(document.get(field_name)).items():
            manifest.names.setdefault(key, spec)
    manifest.scripts = string_map(document.get("scripts"))
    if isinstance(document.get("packageManager"), str):
        manifest.package_manager = document["packageManager"]
    engines = document.get("engines")
    if isinstance(engines, dict) and isinstance(engines.get("node"), str):
        manifest.engines_node = engines["node"]
    return manifest


def scan(root: Path, excludes: list[str]) -> Tree:
    """Read the tree once: its manifests, lockfiles and build configs, and its CI workflows."""
    tree = Tree()
    for directory, subdirectories, files in os.walk(root):
        here = Path(directory)
        relative_dir = here.relative_to(root).as_posix()
        kept = []
        for name in sorted(subdirectories):
            relative = name if relative_dir == "." else f"{relative_dir}/{name}"
            if (
                name.startswith(".")
                or name in SKIPPED_DIRS
                or excluded(relative, excludes)
            ):
                continue
            kept.append(name)
        subdirectories[:] = kept
        for name in sorted(files):
            path = here / name
            relative = path.relative_to(root).as_posix()
            try:
                if name == "Cargo.toml":
                    tree.manifests.append(read_cargo(root, path))
                elif name == "package.json":
                    tree.manifests.append(read_npm(root, path))
                elif name in LOCKFILES:
                    tree.lockfiles.append(relative)
                elif is_config(name):
                    tree.configs[relative] = path.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError, ValueError) as error:
                tree.unreadable.append(f"{relative}: {error}")
    for ci_dir in CI_DIRS:
        directory = root / ci_dir
        if not directory.is_dir():
            continue
        for path in sorted(directory.iterdir()):
            if path.is_file() and path.suffix in (".yml", ".yaml"):
                relative = path.relative_to(root).as_posix()
                try:
                    tree.workflows[relative] = path.read_text(encoding="utf-8")
                except (OSError, UnicodeDecodeError) as error:
                    tree.unreadable.append(f"{relative}: {error}")
    return tree


def detect(tree: Tree, radar: dict) -> dict[str, list[str]]:
    """Which radar items the tree uses, and where: radar item id -> evidence."""
    manifest_names = {PurePosixPath(m.path).name for m in tree.manifests}
    lock_names = {PurePosixPath(lock).name for lock in tree.lockfiles}
    uses: dict[str, list[str]] = {}
    for item_id, item in radar_index(radar).items():
        rule = item.get("detect")
        if not isinstance(rule, dict):
            continue
        hits: list[str] = []
        for manifest in tree.manifests:
            names = string_list(rule.get(manifest.kind))
            hits.extend(
                f"{manifest.path} ({name})" for name in names if name in manifest.names
            )
        for name in string_list(rule.get("manifest")):
            if name in manifest_names:
                hits.append(f"a {name}")
        for name in string_list(rule.get("lockfile")):
            if name in lock_names:
                hits.append(f"a {name}")
        for token in string_list(rule.get("config")):
            hits.extend(
                f"{path} ({token})"
                for path, text in tree.configs.items()
                if token in text
            )
        if hits:
            uses[item_id] = hits
    return uses


def svelte_packages(tree: Tree) -> list[Manifest]:
    """Every package.json that depends on Svelte."""
    return [m for m in tree.manifests if m.kind == "npm" and "svelte" in m.names]


def package_dir(manifest: Manifest) -> str:
    """The tree-relative directory a manifest lives in, `.` for the root."""
    parent = PurePosixPath(manifest.path).parent.as_posix()
    return parent


# --------------------------------------------------------------------------- versions


def series(requirement: str) -> str | None:
    """The major series a requirement or range pins: `^0.8.4` is 0.8, `~1.53` is 1, `5.x` is 5.

    A major of 0 keeps its minor, as Cargo and npm both treat `0.y` as the breaking series. None when
    the text names no version (`*`, `latest`, a git or file source).
    """
    match = VERSION.search(requirement)
    if match is None:
        return None
    major = int(match.group(1))
    minor = match.group(2)
    if major > 0:
        return str(major)
    if minor is None or not minor.isdigit():
        return None
    return f"0.{int(minor)}"


def pnpm_catalogs(root: Path) -> dict[str, dict[str, str]]:
    """`pnpm-workspace.yaml`'s catalogs: `""` for the default `catalog:`, else each named one.

    A minimal line reader for the two shapes pnpm documents, `catalog:` and `catalogs: name:`, so
    the script stays standard-library only. Anything it cannot read resolves to nothing, which the
    caller reports rather than passes.
    """
    try:
        lines = (root / PNPM_WORKSPACE).read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError):
        return {}
    catalogs: dict[str, dict[str, str]] = {}
    block: str | None = None
    named: str | None = None
    named_indent = -1
    for line in lines:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        match = CATALOG_LINE.match(line)
        if match is None:
            continue
        indent, key, value = len(match.group(1)), match.group(3).strip(), match.group(4)
        value = value.strip("'\"")
        if indent == 0:
            block = key if key in ("catalog", "catalogs") else None
            named = None
            continue
        if block == "catalog" and value:
            catalogs.setdefault("", {})[key] = value
        elif block == "catalogs":
            if not value and (named is None or indent <= named_indent):
                named, named_indent = key, indent
            elif named is not None and value and indent > named_indent:
                catalogs.setdefault(named, {})[key] = value
    return catalogs


# --------------------------------------------------------------------------- the classes


def check_stack_declared(context: Context) -> Outcome:
    """`stack-declared`: a root stack.json, well formed, with a non-empty element list."""
    stack = load_stack(context.root)
    findings: list[str] = []
    if stack.get("schema") != STACK_SCHEMA:
        findings.append(f"schema is {stack.get('schema')!r}, not {STACK_SCHEMA}")
    findings.extend(f"unknown key {key!r}" for key in sorted(set(stack) - STACK_KEYS))
    types = stack.get("app_types")
    if not isinstance(types, list) or not types:
        findings.append("declares no app_types")
    else:
        findings.extend(
            f"app type {kind!r} is not a lower-case slug"
            for kind in types
            if not isinstance(kind, str) or not SLUG.match(kind)
        )
    if "radar" in stack and not isinstance(stack["radar"], str):
        findings.append("radar is not a path string")
    if "exclude" in stack and (
        not isinstance(stack["exclude"], list)
        or not all(isinstance(prefix, str) for prefix in stack["exclude"])
    ):
        findings.append("exclude is not a list of path strings")
    elements = stack.get("elements")
    if not isinstance(elements, list) or not elements:
        findings.append("declares no elements")
        return Outcome(findings, 0)
    seen: set[str] = set()
    for position, element in enumerate(elements, start=1):
        where = f"element {position}"
        if not isinstance(element, dict):
            findings.append(f"{where} is not an object")
            continue
        item = element.get("item")
        if not isinstance(item, str) or not SLUG.match(item):
            findings.append(f"{where} names no item slug")
        else:
            where = f"element {item}"
            if item in seen:
                findings.append(f"{where} is declared twice")
            seen.add(item)
        findings.extend(
            f"{where} carries unknown key {key!r}"
            for key in sorted(set(element) - ELEMENT_KEYS)
        )
        if "adr" in element and not (
            isinstance(element["adr"], str) and element["adr"]
        ):
            findings.append(f"{where} adr is not a path string")
        if "planned" in element and not isinstance(element["planned"], bool):
            findings.append(f"{where} planned is not true or false")
        if "pins" in element and (
            not isinstance(element["pins"], dict)
            or not element["pins"]
            or any(
                not isinstance(v, str) or not SERIES.match(v)
                for v in element["pins"].values()
            )
        ):
            findings.append(f"{where} pins is not an object of major series")
    return Outcome(findings, len(elements))


def check_stack_matches_detected(context: Context) -> Outcome:
    """`stack-matches-detected`: the used set and the declared set are one set."""
    stack = load_stack(context.root)
    elements = stack_elements(stack)
    radar = load_radar(radar_path(context, stack))
    index = radar_index(radar)
    tree = scan(context.root, stack_excludes(stack))
    uses = detect(tree, radar)
    findings = [f"cannot read {entry}" for entry in tree.unreadable]
    declared = {element["item"]: element for element in elements}
    ruled = {
        item_id
        for item_id in declared
        if isinstance(index.get(item_id, {}).get("detect"), dict)
    }
    for item_id in sorted(uses):
        if item_id not in declared:
            findings.append(
                f"{item_id} is used ({', '.join(uses[item_id])}) "
                f"but {STACK_FILE} does not declare it"
            )
    for item_id in sorted(ruled):
        if item_id in uses or declared[item_id].get("planned") is True:
            continue
        findings.append(
            f"{STACK_FILE} declares {item_id} but nothing in the tree uses it"
        )
    return Outcome(findings, len(ruled | set(uses)))


def check_no_hold_items(context: Context) -> Outcome:
    """`no-hold-items`: no declared or detected item sits in the hold ring."""
    stack = load_stack(context.root)
    elements = stack_elements(stack)
    radar = load_radar(radar_path(context, stack))
    index = radar_index(radar)
    tree = scan(context.root, stack_excludes(stack))
    uses = detect(tree, radar)
    findings = [f"cannot read {entry}" for entry in tree.unreadable]
    declared = {element["item"] for element in elements}
    for item_id in sorted(declared | set(uses)):
        item = index.get(item_id)
        if item is None or item.get("ring") != "hold":
            continue
        how = (
            "declared" if item_id in declared else f"used ({', '.join(uses[item_id])})"
        )
        findings.append(f"{item_id} is {how} and on hold since {item.get('since')}")
    return Outcome(findings, len(declared | set(uses)))


def check_deviation_has_adr(context: Context) -> Outcome:
    """`deviation-has-adr`: every element off its golden path cites an ADR naming it."""
    stack = load_stack(context.root)
    elements = stack_elements(stack)
    radar = load_radar(radar_path(context, stack))
    index = radar_index(radar)
    known = {
        kind
        for path in golden_paths(radar)
        for kind in string_list(path.get("applies_to"))
    }
    types = string_list(stack.get("app_types"))
    findings = [
        f"app type {kind!r} is on no golden path (known: {', '.join(sorted(known))})"
        for kind in types
        if kind not in known
    ]
    golden = golden_items(radar, types)
    for element in elements:
        item_id = element["item"]
        item = index.get(item_id, {})
        pins = string_map(element.get("pins"))
        repinned = sorted(
            package
            for package, value in pins.items()
            if string_map(item.get("pins")).get(package) != value
        )
        adr = element.get("adr")
        if item_id in golden and not repinned and adr is None:
            continue
        if not isinstance(adr, str) or not adr:
            if item_id in golden:
                why = f"re-pins {', '.join(repinned)} off the radar's major"
            else:
                why = f"is off the golden path for {', '.join(types) or 'no app type'}"
            findings.append(f"{item_id} {why} and cites no ADR")
            continue
        findings.extend(adr_findings(context.root, item_id, item, adr))
    return Outcome(findings, len(elements))


def adr_findings(root: Path, item_id: str, item: dict, adr: str) -> list[str]:
    """Why the ADR an element cites does not record it: outside the tree, absent, or silent."""
    relative = PurePosixPath(adr)
    if relative.is_absolute() or ".." in relative.parts:
        return [f"{item_id} cites {adr}, which is not a path inside the tree"]
    try:
        text = (root / relative).read_text(encoding="utf-8").lower()
    except (OSError, UnicodeDecodeError):
        return [f"{item_id} cites {adr}, which does not exist"]
    names = {item_id.lower()}
    if isinstance(item.get("name"), str):
        names.add(item["name"].lower())
    if not any(name in text for name in names):
        return [f"{item_id} cites {adr}, which never names it"]
    return []


def check_pinned_majors(context: Context) -> Outcome:
    """`pinned-majors`: every pinned package, and Node and pnpm, sits on the major it is pinned to."""
    stack = load_stack(context.root)
    elements = stack_elements(stack)
    radar = load_radar(radar_path(context, stack))
    tree = scan(context.root, stack_excludes(stack))
    findings = [f"cannot read {entry}" for entry in tree.unreadable]
    pins: dict[str, tuple[str, str]] = {}
    for item_id, item in radar_index(radar).items():
        for package, value in string_map(item.get("pins")).items():
            pins[package] = (value, f"the radar's {item_id}")
    for element in elements:
        for package, value in string_map(element.get("pins")).items():
            pins[package] = (value, f"{STACK_FILE}'s {element['item']}")
    workspaces = {
        package_dir(m): m.workspace for m in tree.manifests if m.workspace is not None
    }
    catalogs = pnpm_catalogs(context.root)
    examined = 0
    for manifest in tree.manifests:
        for package, requirement in sorted(manifest.names.items()):
            if package not in pins or package in ("node", "pnpm"):
                continue
            wanted, owner = pins[package]
            requirement = resolve(manifest, package, requirement, workspaces, catalogs)
            if requirement is None:
                continue
            examined += 1
            found = series(requirement)
            if found != wanted:
                findings.append(
                    f"{manifest.path} pins {package} {requirement!r} (series {found}); "
                    f"{owner} pins {wanted}"
                )
    root_npm = next((m for m in tree.manifests if m.path == "package.json"), None)
    if "node" in pins and any(m.kind == "npm" for m in tree.manifests):
        wanted, owner = pins["node"]
        sources = node_pins(context.root, root_npm)
        if not sources:
            findings.append(
                f"Node is used but no major is pinned (.nvmrc, .node-version or engines.node); "
                f"{owner} pins {wanted}"
            )
        for where, requirement in sources:
            examined += 1
            if series(requirement) != wanted:
                findings.append(
                    f"{where} pins Node {requirement!r}; {owner} pins {wanted}"
                )
    if "pnpm" in pins and root_npm is not None and root_npm.package_manager:
        wanted, owner = pins["pnpm"]
        manager, _, version = root_npm.package_manager.partition("@")
        if manager == "pnpm":
            examined += 1
            if series(version) != wanted:
                findings.append(
                    f"package.json packageManager {root_npm.package_manager!r}; {owner} pins "
                    f"pnpm {wanted}"
                )
    return Outcome(findings, examined)


def resolve(
    manifest: Manifest,
    package: str,
    requirement: str | None,
    workspaces: dict[str, dict[str, str | None] | None],
    catalogs: dict[str, dict[str, str]],
) -> str | None:
    """The requirement a dependency really states: its own, its workspace's, or its catalog's.

    None when the dependency is internal to the tree (`workspace:`, `link:`, `file:`), which pins
    nothing a radar governs.
    """
    if manifest.kind == "cargo" and requirement is None:
        directory = PurePosixPath(manifest.path).parent
        for ancestor in (directory, *directory.parents):
            shared = workspaces.get(ancestor.as_posix())
            if shared is not None and package in shared:
                return shared[package]
        return None
    if requirement is None:
        return None
    if requirement.startswith(("workspace:", "link:", "file:")):
        return None
    if requirement.startswith("catalog:"):
        name = requirement.removeprefix("catalog:").strip()
        name = "" if name == "default" else name
        return catalogs.get(name, {}).get(package, requirement)
    return requirement


def node_pins(root: Path, root_npm: Manifest | None) -> list[tuple[str, str]]:
    """Every place the root pins a Node version: `.nvmrc`, `.node-version`, `engines.node`."""
    sources: list[tuple[str, str]] = []
    for name in NODE_PIN_FILES:
        try:
            text = (root / name).read_text(encoding="utf-8").strip()
        except (OSError, UnicodeDecodeError):
            continue
        if text:
            sources.append((name, text.splitlines()[0].strip()))
    if root_npm is not None and root_npm.engines_node:
        sources.append(("package.json engines.node", root_npm.engines_node))
    return sources


def check_svelte_runes(context: Context) -> Outcome:
    """`svelte-runes`: every package that uses Svelte forces runes mode in its build config."""
    stack = load_stack(context.root)
    tree = scan(context.root, stack_excludes(stack))
    findings = [f"cannot read {entry}" for entry in tree.unreadable]
    for manifest in svelte_packages(tree):
        directory = package_dir(manifest)
        configs = {
            path: text
            for path, text in tree.configs.items()
            if PurePosixPath(path).parent.as_posix() == directory
        }
        forced = [
            path
            for path, text in configs.items()
            if re.search(r"runes\s*:\s*true", text)
        ]
        refused = [
            path
            for path, text in configs.items()
            if re.search(r"runes\s*:\s*false", text)
        ]
        for path in refused:
            findings.append(
                f"{path} sets runes: false; Svelte 5 code must compile in runes mode"
            )
        if not forced:
            findings.append(
                f"{manifest.path} uses svelte but no svelte.config or vite.config beside it sets "
                "compilerOptions.runes: true"
            )
    return Outcome(findings, tree.files_read)


def check_svelte_check_ci(context: Context) -> Outcome:
    """`svelte-check-ci`: a CI workflow runs svelte-check for every package that uses Svelte."""
    stack = load_stack(context.root)
    tree = scan(context.root, stack_excludes(stack))
    findings = [f"cannot read {entry}" for entry in tree.unreadable]
    workflows = "\n".join(tree.workflows.values())
    for manifest in svelte_packages(tree):
        scripts = [
            name
            for name, command in manifest.scripts.items()
            if "svelte-check" in command
        ]
        if "svelte-check" in workflows:
            continue
        runner = re.compile(
            r"\b(?:pnpm|npm|yarn|bun)\b[^\n]*\b(?:"
            + "|".join(map(re.escape, scripts))
            + r")\b"
        )
        if scripts and runner.search(workflows):
            continue
        how = (
            f"a script ({', '.join(scripts)}) runs it"
            if scripts
            else "no script runs it"
        )
        findings.append(
            f"{manifest.path} uses svelte and no CI workflow runs svelte-check ({how}; "
            f"workflows read: {len(tree.workflows)})"
        )
    return Outcome(findings, tree.files_read)


def check_no_corepack(context: Context) -> Outcome:
    """`no-corepack`: nothing runs Corepack, and a pnpm tree pins pnpm with `packageManager`."""
    stack = load_stack(context.root)
    tree = scan(context.root, stack_excludes(stack))
    findings = [f"cannot read {entry}" for entry in tree.unreadable]
    word = re.compile(r"\bcorepack\b")
    for path, text in sorted(tree.workflows.items()):
        for number, line in enumerate(text.splitlines(), start=1):
            if word.search(line):
                findings.append(f"{path}:{number} runs corepack")
    for manifest in tree.manifests:
        for name, command in sorted(manifest.scripts.items()):
            if word.search(command):
                findings.append(f"{manifest.path} script {name} runs corepack")
    pnpm = [
        lock for lock in tree.lockfiles if PurePosixPath(lock).name == "pnpm-lock.yaml"
    ]
    root_npm = next((m for m in tree.manifests if m.path == "package.json"), None)
    if pnpm and (
        root_npm is None or not (root_npm.package_manager or "").startswith("pnpm@")
    ):
        findings.append(
            f"{pnpm[0]} is pnpm's, and the root package.json pins no packageManager pnpm@<version>"
        )
    return Outcome(findings, tree.files_read)


def check_radar_fresh(context: Context) -> Outcome:
    """`radar-fresh`: refreshed within FRESH_DAYS, and no entry dated after its refresh."""
    radar = load_radar(radar_path(context, None))
    refreshed = parse_date(radar.get("refreshed_at"))
    if refreshed is None:
        raise Void(
            f"the radar's refreshed_at {radar.get('refreshed_at')!r} is not a date"
        )
    findings: list[str] = []
    age = (context.today - refreshed).days
    if age > FRESH_DAYS:
        findings.append(
            f"refreshed_at {refreshed} is {age} days old; the monthly refresh is due "
            f"(advisory after {FRESH_DAYS} days)"
        )
    if age < 0:
        findings.append(f"refreshed_at {refreshed} is after today {context.today}")
    items = [item for item in radar["items"] if isinstance(item, dict)]
    for item in items:
        since = parse_date(item.get("since"))
        if since is not None and since > refreshed:
            findings.append(
                f"{item.get('id')} since {since} is after the radar's refreshed_at {refreshed}"
            )
    return Outcome(findings, 1 + len(items))


def check_radar_approvals(context: Context) -> Outcome:
    """`radar-approvals`: the owner approved every adopt and hold entry and every golden path."""
    radar = load_radar(radar_path(context, None))
    findings = [
        f"radar carries unknown key {key!r}" for key in sorted(set(radar) - RADAR_KEYS)
    ]
    if parse_date(radar.get("refreshed_at")) is None:
        findings.append(f"refreshed_at {radar.get('refreshed_at')!r} is not a date")
    findings.extend(rubric_findings(context, radar.get("rubric")))
    items = radar["items"]
    index = radar_index(radar)
    seen: set[str] = set()
    claims: dict[tuple[str, str], str] = {}
    for position, item in enumerate(items, start=1):
        if not isinstance(item, dict):
            findings.append(f"item {position} is not an object")
            continue
        item_id = item.get("id")
        if not isinstance(item_id, str) or not SLUG.match(item_id):
            findings.append(f"item {position} names no id slug")
            item_id = f"item {position}"
        elif item_id in seen:
            findings.append(f"{item_id} is listed twice")
        seen.add(item_id)
        findings.extend(item_findings(context, item_id, item, claims))
    paths = radar.get("golden_paths")
    if not isinstance(paths, list) or not paths:
        findings.append("the radar holds no golden_paths")
        paths = []
    for position, path in enumerate(paths, start=1):
        findings.extend(path_findings(context, position, path, index))
    return Outcome(findings, len(items) + len(paths))


def rubric_findings(context: Context, rubric: object) -> list[str]:
    """The rubric's weights cover the owner's four criteria and sum to one, owner-approved."""
    if rubric is None:
        return []
    if not isinstance(rubric, dict):
        return ["rubric is not an object"]
    findings: list[str] = []
    weights = rubric.get("weights")
    if (
        not isinstance(weights, dict)
        or set(weights) != set(SCORE_KEYS)
        or not all(isinstance(value, (int, float)) for value in weights.values())
    ):
        findings.append(f"rubric weights are not {', '.join(SCORE_KEYS)}")
    elif abs(sum(weights.values()) - 1) > 0.001:
        findings.append(f"rubric weights sum to {sum(weights.values()):g}, not 1")
    approval = rubric.get("owner_approval")
    if not isinstance(approval, dict) or not str(approval.get("words", "")).strip():
        findings.append("rubric has no owner approval")
    else:
        approved = parse_date(approval.get("date"))
        if approved is None or approved > context.today:
            findings.append(
                f"rubric owner_approval date {approval.get('date')!r} is not a past date"
            )
    return findings


def item_findings(
    context: Context, item_id: str, item: dict, claims: dict[tuple[str, str], str]
) -> list[str]:
    """One radar entry's structural findings and its approval finding."""
    findings = [
        f"{item_id} carries unknown key {key!r}"
        for key in sorted(set(item) - ITEM_KEYS)
    ]
    for key in ("name", "category", "rationale"):
        if not isinstance(item.get(key), str) or not item[key].strip():
            findings.append(f"{item_id} has no {key}")
    ring = item.get("ring")
    if ring not in RINGS:
        findings.append(f"{item_id} ring {ring!r} is not one of {', '.join(RINGS)}")
    since = parse_date(item.get("since"))
    if since is None:
        findings.append(f"{item_id} since {item.get('since')!r} is not a date")
    elif since > context.today:
        findings.append(f"{item_id} since {since} is after today {context.today}")
    evidence = item.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        findings.append(f"{item_id} records no evidence")
    else:
        findings.extend(
            f"{item_id} evidence {url!r} is not an https URL"
            for url in evidence
            if not isinstance(url, str) or not url.startswith("https://")
        )
    if "pins" in item and (
        not isinstance(item["pins"], dict)
        or not item["pins"]
        or any(
            not isinstance(v, str) or not SERIES.match(v) for v in item["pins"].values()
        )
    ):
        findings.append(f"{item_id} pins is not an object of major series")
    if "scores" in item:
        scores = item["scores"]
        if (
            not isinstance(scores, dict)
            or set(scores) != set(SCORE_KEYS)
            or not all(
                isinstance(v, (int, float)) and 1 <= v <= 5 for v in scores.values()
            )
        ):
            findings.append(
                f"{item_id} scores are not ai, fit, maturity and modern from 1 to 5"
            )
    if "context7" in item and not string_list(item["context7"]):
        findings.append(f"{item_id} context7 is not a list of library ids")
    findings.extend(detect_findings(item_id, item.get("detect"), claims))
    approval = item.get("owner_approval")
    if ring in APPROVED_RINGS and approval is None:
        findings.append(f"{item_id} is in {ring} with no owner approval")
    elif approval is not None:
        findings.extend(approval_findings(context, item_id, approval, ring, since))
    return findings


def detect_findings(
    item_id: str, rule: object, claims: dict[tuple[str, str], str]
) -> list[str]:
    """A detect rule's shape, and a dependency two items both claim."""
    if rule is None:
        return []
    if not isinstance(rule, dict) or not rule:
        return [f"{item_id} detect is not an object of name lists"]
    findings = [
        f"{item_id} detect carries unknown key {key!r}"
        for key in sorted(set(rule) - set(DETECT_KEYS))
    ]
    for key in DETECT_KEYS:
        if key not in rule:
            continue
        names = rule[key]
        if (
            not isinstance(names, list)
            or not names
            or not all(isinstance(name, str) and name for name in names)
        ):
            findings.append(f"{item_id} detect {key} is not a list of names")
            continue
        for name in names:
            owner = claims.setdefault((key, name), item_id)
            if owner != item_id:
                findings.append(f"{item_id} detect {key} {name} is already {owner}'s")
    return findings


def approval_findings(
    context: Context,
    item_id: str,
    approval: object,
    ring: object,
    since: dt.date | None,
) -> list[str]:
    """Why an owner approval does not record the owner approving this entry's ring."""
    if not isinstance(approval, dict):
        return [f"{item_id} owner_approval is not an object"]
    findings = [
        f"{item_id} owner_approval carries unknown key {key!r}"
        for key in sorted(set(approval) - APPROVAL_KEYS)
    ]
    if approval.get("ring") != ring:
        findings.append(
            f"{item_id} owner_approval is for {approval.get('ring')!r}, not its ring {ring!r}"
        )
    if not isinstance(approval.get("words"), str) or not approval["words"].strip():
        findings.append(f"{item_id} owner_approval records no words")
    approved = parse_date(approval.get("date"))
    if approved is None:
        findings.append(
            f"{item_id} owner_approval date {approval.get('date')!r} is not a date"
        )
    elif approved > context.today:
        findings.append(f"{item_id} owner_approval date {approved} is after today")
    elif since is not None and since < approved:
        findings.append(
            f"{item_id} entered {ring} on {since}, before the owner approved it"
        )
    return findings


def path_findings(
    context: Context, position: int, path: object, index: dict[str, dict]
) -> list[str]:
    """One golden path's findings: its shape, every item adopted, and the owner's approval."""
    if not isinstance(path, dict):
        return [f"golden path {position} is not an object"]
    name = (
        path.get("id") if isinstance(path.get("id"), str) else f"golden path {position}"
    )
    findings = [
        f"{name} carries unknown key {key!r}" for key in sorted(set(path) - PATH_KEYS)
    ]
    if not string_list(path.get("applies_to")):
        findings.append(f"{name} applies to no app type")
    items = string_list(path.get("items"))
    if not items:
        findings.append(f"{name} names no items")
    for item_id in items:
        item = index.get(item_id)
        if item is None:
            findings.append(f"{name} names {item_id}, which the radar does not list")
        elif item.get("ring") != "adopt":
            findings.append(
                f"{name} names {item_id}, which is in {item.get('ring')}, not adopt"
            )
    approval = path.get("owner_approval")
    if not isinstance(approval, dict):
        findings.append(f"{name} has no owner approval")
    else:
        if not isinstance(approval.get("words"), str) or not approval["words"].strip():
            findings.append(f"{name} owner_approval records no words")
        approved = parse_date(approval.get("date"))
        if approved is None or approved > context.today:
            findings.append(
                f"{name} owner_approval date {approval.get('date')!r} is not a past date"
            )
    return findings


CHECKS = {
    "stack-declared": check_stack_declared,
    "stack-matches-detected": check_stack_matches_detected,
    "no-hold-items": check_no_hold_items,
    "deviation-has-adr": check_deviation_has_adr,
    "pinned-majors": check_pinned_majors,
    "svelte-runes": check_svelte_runes,
    "svelte-check-ci": check_svelte_check_ci,
    "no-corepack": check_no_corepack,
    "radar-fresh": check_radar_fresh,
    "radar-approvals": check_radar_approvals,
}


# --------------------------------------------------------------------------- the command line


def run_check(context: Context, name: str) -> int:
    """Run one class, print its findings and `examined N`, and return its exit."""
    try:
        outcome = CHECKS[name](context)
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


def run_detect(context: Context) -> int:
    """Print what the tree uses, item by item, then how many files were read."""
    try:
        stack: dict | None = load_stack(context.root)
    except Void:
        stack = None
    try:
        radar = load_radar(radar_path(context, stack))
    except Void as void:
        print(f"detect: VOID {void}")
        print("examined 0")
        return EXIT_VOID
    tree = scan(context.root, stack_excludes(stack))
    for entry in tree.unreadable:
        print(f"detect: cannot read {entry}")
    uses = detect(tree, radar)
    for item_id in sorted(uses):
        print(f"{item_id}: {', '.join(uses[item_id])}")
    print(f"examined {tree.files_read}")
    return EXIT_VOID if tree.files_read == 0 else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    """`--root`, `--radar` and `--today` are accepted before or after the verb."""
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    common.add_argument("--radar", type=Path, default=argparse.SUPPRESS)
    common.add_argument("--today", default=argparse.SUPPRESS)
    top = argparse.ArgumentParser(
        description="Judge a repository's declared stack against the stack-selection radar.",
        parents=[common],
    )
    verbs = top.add_subparsers(dest="verb", required=True)
    check = verbs.add_parser("check", parents=[common], help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    verbs.add_parser("detect", parents=[common], help="print what the tree uses")
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
        print(f"stack-probe: --root {root} is not a directory", file=sys.stderr)
        return EXIT_USAGE
    today_text = getattr(args, "today", None)
    today = (
        parse_date(today_text)
        if today_text
        else dt.datetime.now(dt.timezone.utc).date()
    )
    if today is None:
        print(f"stack-probe: --today {today_text!r} is not YYYY-MM-DD", file=sys.stderr)
        return EXIT_USAGE
    radar_flag = getattr(args, "radar", None)
    context = Context(
        root=root,
        radar_flag=radar_flag.resolve() if radar_flag is not None else None,
        today=today,
    )
    if tomllib is None:
        print("stack-probe: VOID Python 3.11 or newer is required (tomllib)")
        print("examined 0")
        return EXIT_VOID
    if args.verb == "detect":
        return run_detect(context)
    return run_check(context, args.klass)


if __name__ == "__main__":
    sys.exit(main())

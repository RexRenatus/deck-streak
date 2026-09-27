#!/usr/bin/env python3
"""ddd-probe: domain-driven boundaries, judged on any repository root (SPEC-V2-2186 R2).

    python3 scripts/ddd-probe.py --root <repo> check <class>|all
    python3 scripts/ddd-probe.py --root <repo> list

Three classes over the context map and lexicon `methodology.json`'s `ddd` section locates
(defaults `docs/CONTEXT-MAP.md` and `docs/LEXICON.md`):

  context-map-parses            the map's every line parses; names are unique, every dependency
                                is a declared context, the graph has no cycle, and no context's
                                code path contains another's
  declared-edges-match-imports  each context's dependencies, read from its code, EQUAL the map's:
                                Python imports (ast, relative imports resolved), TypeScript and
                                JavaScript imports (tsconfig `paths` and `baseUrl` resolved), and
                                Rust Cargo manifests; an import of first-party code no context
                                owns is refused, and a Cargo workspace member no context names
  lexicon-locks                 no identifier declared in a context's code says a word the
                                lexicon forbids there, in snake, camel or multi-word form

The map is the ```context-map fenced block, one context per line:

    <context> [<path>, ...] [(<note>)] depends on: <context>, ... | nothing [internal]

A context with a path owns the code under it. A context with no path is a Cargo package of that
name in the root workspace. `map_heading` reads instead the first fenced block under the `## `
heading it names, and `dependency_prefix` lets `kernel` name `phx-kernel`: phoenix-v2's own
crate graph is written that way. A context with no code yet is PLANNED: counted, not judged.

The lexicon is the ```lexicon fenced block, one entry per line:

    <the one name> [in <context>, ...]: <forbidden word>, <forbidden word>, ...
"""

from __future__ import annotations

import ast
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import methodology_probe as mp  # noqa: E402

MAP_LINE = re.compile(r"^(?P<head>.*?)\bdepends on:\s*(?P<deps>.*)$")
NOTE = re.compile(r"\([^)]*\)")
NOTHING = ("nothing", "nothing internal", "none", "-")
LEXICON_ENTRY = re.compile(
    r"^(?P<term>[^:]+?)(?:\s+in\s+(?P<scope>[^:]+?))?\s*:\s*(?P<words>\S.*)$"
)
PYTHON = (".py",)
SCRIPT = (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts")
RUST = (".rs",)
RESOLVABLE = (*SCRIPT, ".d.ts", ".json")
SCRIPT_IMPORTS = (
    re.compile(r"\bfrom\s+(['\"])([^'\"\n]+)\1"),
    re.compile(r"\bimport\s+(['\"])([^'\"\n]+)\1"),
    re.compile(r"\bimport\s*\(\s*(['\"])([^'\"\n]+)\1"),
    re.compile(r"\brequire\s*\(\s*(['\"])([^'\"\n]+)\1"),
)
SCRIPT_DECLARATIONS = re.compile(
    r"\b(?:function\*?|class|interface|type|enum|namespace|const|let|var)\s+"
    r"([A-Za-z_$][\w$]*)"
)
RUST_DECLARATIONS = (
    re.compile(r"\bfn\s+([A-Za-z_]\w*)"),
    re.compile(r"\b(?:struct|enum|trait|type|mod|union)\s+([A-Za-z_]\w*)"),
    re.compile(r"\bconst\s+(?!fn\b)([A-Za-z_]\w*)"),
    re.compile(r"\bstatic\s+(?:mut\s+)?([A-Za-z_]\w*)"),
    re.compile(r"\blet\s+(?:mut\s+)?([A-Za-z_]\w*)"),
)
DEPENDENCY_TABLES = ("dependencies", "build-dependencies")


@dataclass
class Declared:
    """One context as the map declares it."""

    name: str
    paths: list[str]
    raw_deps: list[str]
    line: int
    deps: list[str] = field(default_factory=list)


@dataclass
class Map:
    """The parsed map: where it was read, its contexts, and what did not parse."""

    source: str
    contexts: list[Declared]
    problems: list[str]
    void: str = ""


@dataclass
class Code:
    """A context's code: its Cargo manifest, if it is a Rust package, and its source files."""

    manifest: Path | None = None
    package: str | None = None
    files: list[Path] = field(default_factory=list)


def map_lines(context: mp.Context, text: str, source: str) -> tuple[list, str]:
    """The lines of the map's block(s), or why there are none."""
    heading = context.section("ddd")["map_heading"]
    found = mp.blocks(text)
    if not heading:
        chosen = [block for block in found if block.info == "context-map"]
        if not chosen:
            return [], f"no ```context-map fence in {source}"
        return [line for block in chosen for line in block.lines], ""
    sections = mp.sections(text)
    for index, section in enumerate(sections):
        if section.title.lower().startswith(heading.lower()):
            end = sections[index + 1].line if index + 1 < len(sections) else 10**9
            inside = [b for b in found if section.line < b.start < end]
            if not inside:
                return [], f"no fenced block under `## {heading}` in {source}"
            return inside[0].lines, ""
    return [], f"no `## {heading}` section in {source}"


def read_map(context: mp.Context) -> Map:
    ddd = context.section("ddd")
    source = ddd["context_map"]
    path = context.root / source
    if not path.is_file():
        return Map(source, [], [], f"no context map at {source}")
    lines, void = map_lines(context, path.read_text(encoding="utf-8"), source)
    parsed = Map(source, [], [], void)
    for number, line in lines:
        text = line.strip()
        if not text or text.startswith("#"):
            continue
        matched = MAP_LINE.match(text)
        head = (
            NOTE.sub(" ", matched.group("head")).replace(",", " ").split()
            if matched
            else []
        )
        if not head:
            parsed.problems.append(
                f"{source}:{number}: not `<context> [paths] depends on: <contexts>`"
            )
            continue
        deps_text = matched.group("deps").strip()
        if deps_text.lower() in NOTHING:
            raw = []
        else:
            deps_text = re.sub(r"\s+only$", "", deps_text)
            raw = [dep.strip() for dep in deps_text.split(",") if dep.strip()]
        paths = [item.rstrip("/") for item in head[1:]]
        parsed.contexts.append(Declared(head[0], paths, raw, number))
    names = [declared.name for declared in parsed.contexts]
    for name in sorted({name for name in names if names.count(name) > 1}):
        parsed.problems.append(f"{source}: {name} is declared twice")
    prefix = ddd["dependency_prefix"]
    for declared in parsed.contexts:
        for dep in declared.raw_deps:
            resolved = (
                dep
                if dep in names
                else (prefix + dep if prefix + dep in names else None)
            )
            if resolved is None:
                parsed.problems.append(
                    f"{source}:{declared.line}: {declared.name} depends on {dep}, "
                    "which the map does not declare"
                )
            elif resolved == declared.name:
                parsed.problems.append(
                    f"{source}:{declared.line}: {declared.name} depends on itself"
                )
            else:
                declared.deps.append(resolved)
        for item in declared.paths:
            if item.startswith(("/", "~")) or ".." in item.split("/"):
                parsed.problems.append(
                    f"{source}:{declared.line}: {declared.name}'s path {item} "
                    "is not repository-relative"
                )
    parsed.problems.extend(cycle(parsed, source))
    owners = [
        (item, declared.name) for declared in parsed.contexts for item in declared.paths
    ]
    for outer, outer_name in owners:
        for inner, inner_name in owners:
            if (outer, outer_name) != (inner, inner_name) and (
                inner == outer or inner.startswith(outer + "/")
            ):
                parsed.problems.append(
                    f"{source}: {outer} ({outer_name}) contains {inner} ({inner_name})"
                )
    return parsed


def cycle(parsed: Map, source: str) -> list[str]:
    """The first cycle a depth-first walk in declaration order meets, named in walk order."""
    graph = {declared.name: declared.deps for declared in parsed.contexts}
    state: dict[str, int] = {}
    stack: list[str] = []

    def visit(name: str) -> list[str] | None:
        state[name] = 1
        stack.append(name)
        for dep in graph.get(name, []):
            if state.get(dep) == 1:
                return stack[stack.index(dep) :] + [dep]
            if dep not in state:
                found = visit(dep)
                if found:
                    return found
        stack.pop()
        state[name] = 2
        return None

    for name in graph:
        if name not in state:
            found = visit(name)
            if found:
                return [f"{source}: the map has a cycle: {' -> '.join(found)}"]
    return []


def context_map_parses(context: mp.Context) -> mp.Result:
    parsed = read_map(context)
    edges = sum(len(declared.deps) for declared in parsed.contexts)
    result = mp.Result(len(parsed.contexts), "context(s)", findings=parsed.problems)
    result.notes.append(f"{edges} edge(s)")
    result.void_reason = parsed.void or f"no context declared in {parsed.source}"
    return result


# ------------------------------------------------------------------------------ code


def excluded(context: mp.Context):
    return [mp.glob_regex(pattern) for pattern in context.section("ddd")["exclude"]]


def gather(context: mp.Context, parsed: Map) -> dict[str, Code]:
    """Each declared context's code, found under its paths or as its Cargo package."""
    packages = mp.workspace(context.root)
    skip = excluded(context)
    code: dict[str, Code] = {}
    for declared in parsed.contexts:
        unit = Code()
        if not declared.paths and declared.name in packages:
            crate = packages[declared.name]
            unit.manifest, unit.package = crate / "Cargo.toml", declared.name
            unit.files = mp.files_under(context.root, crate / "src", RUST)
        for item in declared.paths:
            base = context.root / item
            if (base / "Cargo.toml").is_file():
                unit.manifest = base / "Cargo.toml"
                unit.package = (
                    mp.load_toml(unit.manifest).get("package", {}).get("name")
                )
            for path in mp.files_under(context.root, base, PYTHON + SCRIPT + RUST):
                if not mp.matches_any(mp.rel(context.root, path), skip):
                    unit.files.append(path)
        code[declared.name] = unit
    return code


def owner_of_path(parsed: Map, relative: str) -> str | None:
    best = None
    for declared in parsed.contexts:
        for item in declared.paths:
            if relative == item or relative.startswith(item + "/"):
                if best is None or len(item) > len(best[0]):
                    best = (item, declared.name)
    return best[1] if best else None


# ------------------------------------------------------------------------------ python


def python_module(directory: Path, root: Path, roots: list[str]) -> tuple[str, Path]:
    """A context directory's dotted module name, and the source root it is relative to."""
    for base in sorted(roots, key=len, reverse=True):
        top = root / base
        if directory == top or top in directory.parents:
            return ".".join(directory.relative_to(top).parts), top
    if not (directory / "__init__.py").is_file():
        return directory.name, directory.parent
    parts = []
    while (directory / "__init__.py").is_file():
        parts.insert(0, directory.name)
        directory = directory.parent
    return ".".join(parts), directory


class PythonImports:
    """Resolves a Python file's imports to the contexts that own them."""

    def __init__(self, context: mp.Context, parsed: Map):
        roots = context.section("ddd")["python_roots"]
        self.root = context.root
        self.modules: dict[str, str] = {}
        self.sources: dict[str, Path] = {}
        for declared in parsed.contexts:
            for item in declared.paths:
                directory = context.root / item
                if directory.is_dir():
                    module, source = python_module(directory, context.root, roots)
                    if module:
                        self.modules[module] = declared.name
                    self.sources[declared.name] = source

    def owner(self, module: str) -> str | None:
        best = None
        for prefix, name in self.modules.items():
            if module == prefix or module.startswith(prefix + "."):
                if best is None or len(prefix) > len(best[0]):
                    best = (prefix, name)
        return best[1] if best else None

    def first_party(self, module: str) -> bool:
        parts = module.split(".")
        for source in set(self.sources.values()):
            if (source.joinpath(*parts).with_suffix(".py")).is_file():
                return True
            if (source.joinpath(*parts) / "__init__.py").is_file():
                return True
        return False

    def targets(self, path: Path, name: str) -> list[tuple[int, list[str]]]:
        """Each import statement in `path`: its line and its candidate modules, best first."""
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        source = self.sources.get(name, path.parent)
        parts = list(path.relative_to(source).with_suffix("").parts)
        package = parts[:-1] if parts[-1] != "__init__" else parts[:-1]
        found = []
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    found.append((node.lineno, [alias.name]))
            elif isinstance(node, ast.ImportFrom):
                if node.level:
                    keep = len(package) - (node.level - 1)
                    if keep < 0:
                        continue
                    base = package[:keep] + (
                        node.module.split(".") if node.module else []
                    )
                else:
                    base = node.module.split(".") if node.module else []
                stem = ".".join(base)
                candidates = [
                    f"{stem}.{alias.name}" if stem else alias.name
                    for alias in node.names
                ]
                if stem:
                    candidates.append(stem)
                found.append((node.lineno, candidates))
        return sorted(found)


# ------------------------------------------------------------------------------ scripts


def read_jsonc(path: Path) -> dict:
    text = mp.blank_out(path.read_text(encoding="utf-8"), rust=False, strings=False)
    text = re.sub(r",(\s*[}\]])", r"\1", text)
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return data if isinstance(data, dict) else {}


class ScriptImports:
    """Resolves TypeScript and JavaScript import specifiers to repository paths."""

    def __init__(self, root: Path):
        self.root = root
        self.configs: dict[Path, dict | None] = {}

    def tsconfig(self, directory: Path) -> tuple[Path, dict] | None:
        while True:
            if directory not in self.configs:
                candidate = directory / "tsconfig.json"
                self.configs[directory] = (
                    read_jsonc(candidate) if candidate.is_file() else None
                )
            if self.configs[directory] is not None:
                return directory, self.configs[directory]
            if directory == self.root or self.root not in directory.parents:
                return None
            directory = directory.parent

    def inside(self, path: str) -> str | None:
        normal = os.path.normpath(path)
        return None if normal.startswith("..") or os.path.isabs(normal) else normal

    def resolve(self, file: Path, spec: str) -> str | None:
        """The repository-relative path `spec` names from `file`, or None when external."""
        if spec.startswith(("./", "../")) or spec in (".", ".."):
            return self.inside(os.path.relpath(file.parent / spec, self.root))
        found = self.tsconfig(file.parent)
        if found is None:
            return None
        directory, config = found
        options = config.get("compilerOptions", {}) or {}
        base = directory / options.get("baseUrl", ".")
        for pattern, targets in (options.get("paths", {}) or {}).items():
            if not targets:
                continue
            if "*" in pattern:
                before, after = pattern.split("*", 1)
                if spec.startswith(before) and spec.endswith(after):
                    star = spec[len(before) : len(spec) - len(after) or None]
                    target = base / targets[0].replace("*", star)
                    return self.inside(os.path.relpath(target, self.root))
            elif spec == pattern:
                return self.inside(os.path.relpath(base / targets[0], self.root))
        if "baseUrl" in options:
            candidate = self.inside(os.path.relpath(base / spec, self.root))
            if candidate and self.exists(candidate):
                return candidate
        return None

    def exists(self, relative: str) -> bool:
        path = self.root / relative
        if path.exists():
            return True
        if any(Path(f"{path}{suffix}").is_file() for suffix in RESOLVABLE):
            return True
        return any((path / f"index{suffix}").is_file() for suffix in RESOLVABLE)

    def specifiers(self, path: Path) -> list[tuple[int, str]]:
        text = mp.blank_out(path.read_text(encoding="utf-8"), rust=False, strings=False)
        found = []
        for pattern in SCRIPT_IMPORTS:
            for matched in pattern.finditer(text):
                found.append((mp.line_of(text, matched.start()), matched.group(2)))
        return sorted(set(found))


# ------------------------------------------------------------------------------ edges


def declared_edges_match_imports(context: mp.Context) -> mp.Result:
    parsed = read_map(context)
    result = mp.Result(0, "code unit(s)")
    if not parsed.contexts:
        result.void_reason = parsed.void or f"no context declared in {parsed.source}"
        return result
    if parsed.problems:
        result.notes.append(
            f"the map has {len(parsed.problems)} problem(s); context-map-parses names them"
        )
    code = gather(context, parsed)
    package_owner = {unit.package: name for name, unit in code.items() if unit.package}
    python = PythonImports(context, parsed)
    scripts = ScriptImports(context.root)
    include_dev = context.section("ddd")["include_dev_dependencies"]
    tables = DEPENDENCY_TABLES + (("dev-dependencies",) if include_dev else ())
    observed: dict[str, dict[str, list[str]]] = {}
    planned = []
    for declared in parsed.contexts:
        unit = code[declared.name]
        edges = observed.setdefault(declared.name, {})
        if unit.manifest is None and not unit.files:
            planned.append(declared.name)
            continue
        if unit.manifest is not None:
            result.examined += 1
            where = mp.rel(context.root, unit.manifest)
            for dep in manifest_dependencies(mp.load_toml(unit.manifest), tables):
                target = package_owner.get(dep)
                if target and target != declared.name:
                    edges.setdefault(target, []).append(where)
        for path in unit.files:
            relative = mp.rel(context.root, path)
            if path.suffix in RUST:
                continue
            result.examined += 1
            if path.suffix in PYTHON:
                try:
                    statements = python.targets(path, declared.name)
                except SyntaxError as error:
                    result.findings.append(f"{relative}: does not parse: {error.msg}")
                    continue
                for number, candidates in statements:
                    target = next(filter(None, map(python.owner, candidates)), None)
                    if target is None:
                        first = next(
                            (c for c in candidates if python.first_party(c)), None
                        )
                        if first:
                            result.findings.append(
                                f"{relative}:{number} imports {first}, "
                                "first-party code no context owns"
                            )
                        continue
                    if target != declared.name:
                        edges.setdefault(target, []).append(f"{relative}:{number}")
            else:
                for number, spec in scripts.specifiers(path):
                    resolved = scripts.resolve(path, spec)
                    if resolved is None:
                        continue
                    target = owner_of_path(parsed, resolved)
                    if target is None:
                        if scripts.exists(resolved):
                            result.findings.append(
                                f"{relative}:{number} imports {spec}, "
                                "first-party code no context owns"
                            )
                        continue
                    if target != declared.name:
                        edges.setdefault(target, []).append(f"{relative}:{number}")
    for declared in parsed.contexts:
        if declared.name in planned:
            continue
        edges = observed[declared.name]
        for target in sorted(set(edges) - set(declared.deps)):
            sites = edges[target]
            more = f" and {len(sites) - 1} more" if len(sites) > 1 else ""
            result.findings.append(
                f"{declared.name} -> {target} is not in the map ({sites[0]}{more})"
            )
        for target in sorted(set(declared.deps) - set(edges)):
            result.findings.append(
                f"{declared.name} declares {target}, and none of its code depends on it"
            )
    named = {unit.package for unit in code.values() if unit.package}
    for package, crate in sorted(mp.workspace(context.root).items()):
        if package not in named:
            result.findings.append(
                f"workspace member {package} ({mp.rel(context.root, crate) or '.'}) "
                "belongs to no context"
            )
    if planned:
        result.notes.append(
            f"{len(planned)} planned (no code yet): {', '.join(planned)}"
        )
    result.void_reason = "no context has code yet"
    return result


def manifest_dependencies(data: dict, tables: tuple[str, ...]) -> list[str]:
    """Every package a manifest depends on in `tables`, target-specific tables included."""
    found = []
    scopes = [data] + [
        spec for spec in data.get("target", {}).values() if isinstance(spec, dict)
    ]
    for scope in scopes:
        for table in tables:
            for key, value in (scope.get(table, {}) or {}).items():
                renamed = value.get("package") if isinstance(value, dict) else None
                found.append(renamed or key)
    return found


# ------------------------------------------------------------------------------ lexicon


def segments(name: str) -> list[str]:
    """`load_flashcards` -> load, flashcards; `StudySession` -> study, session."""
    parts = re.findall(r"[A-Z]+(?=[A-Z][a-z])|[A-Z]?[a-z]+|[A-Z]+|\d+", name)
    return [part.lower() for part in parts]


def says(identifier: list[str], word: list[str]) -> bool:
    """Whether `word`'s segments run consecutively in `identifier`, its last one plural or not."""
    size = len(word)
    for start in range(len(identifier) - size + 1):
        window = identifier[start : start + size]
        last = word[-1]
        if window[:-1] == word[:-1] and window[-1] in (last, last + "s", last + "es"):
            return True
    return False


@dataclass
class Entry:
    term: str
    scope: list[str]
    words: list[str]
    line: int


def read_lexicon(
    context: mp.Context, parsed: Map
) -> tuple[list[Entry], list[str], str]:
    source = context.section("ddd")["lexicon"]
    path = context.root / source
    if not path.is_file():
        return [], [], f"no lexicon at {source}"
    chosen = [
        block
        for block in mp.blocks(path.read_text(encoding="utf-8"))
        if block.info == "lexicon"
    ]
    if not chosen:
        return [], [], f"no ```lexicon fence in {source}"
    names = {declared.name for declared in parsed.contexts}
    entries, problems = [], []
    for number, line in (line for block in chosen for line in block.lines):
        text = line.strip()
        if not text or text.startswith("#"):
            continue
        matched = LEXICON_ENTRY.match(text)
        if matched is None:
            problems.append(f"{source}:{number}: not `<name> [in <contexts>]: <words>`")
            continue
        scope = [
            item.strip()
            for item in (matched.group("scope") or "").split(",")
            if item.strip()
        ]
        for item in scope:
            if item not in names:
                problems.append(
                    f"{source}:{number}: scopes {matched.group('term').strip()} to {item}, "
                    "which the map does not declare"
                )
        words = [
            word.strip() for word in matched.group("words").split(",") if word.strip()
        ]
        entries.append(Entry(matched.group("term").strip(), scope, words, number))
    terms = {
        tuple(segments(entry.term.replace(" ", "_"))): entry.term for entry in entries
    }
    for entry in entries:
        for word in entry.words:
            key = tuple(segments(word.replace(" ", "_").replace("-", "_")))
            if key in terms:
                problems.append(
                    f"{source}:{entry.line}: {word} is both a name ({terms[key]}) "
                    "and a word the lexicon forbids"
                )
    return entries, problems, ""


def declarations(path: Path) -> list[tuple[int, str]]:
    """Every name a source file declares: Python by its syntax tree, others by their keywords."""
    text = path.read_text(encoding="utf-8")
    if path.suffix in PYTHON:
        found = []
        for node in ast.walk(ast.parse(text, filename=str(path))):
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.ClassDef):
                found.append((node.lineno, node.name))
            elif isinstance(node, ast.arg):
                found.append((node.lineno, node.arg))
            elif isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                found.append((node.lineno, node.id))
            elif isinstance(node, ast.Attribute) and isinstance(node.ctx, ast.Store):
                found.append((node.lineno, node.attr))
            elif isinstance(node, ast.alias) and node.asname:
                found.append((node.lineno, node.asname))
        return sorted(found)
    rust = path.suffix in RUST
    clean = mp.blank_out(text, rust=rust, strings=True)
    patterns = RUST_DECLARATIONS if rust else (SCRIPT_DECLARATIONS,)
    found = []
    for pattern in patterns:
        for matched in pattern.finditer(clean):
            found.append((mp.line_of(clean, matched.start(1)), matched.group(1)))
    return sorted(found)


def lexicon_locks(context: mp.Context) -> mp.Result:
    parsed = read_map(context)
    entries, problems, void = read_lexicon(context, parsed)
    result = mp.Result(0, "declaration(s)", findings=list(problems))
    if void or not entries:
        result.void_reason = void or "the lexicon holds no entry"
        return result
    if not parsed.contexts:
        result.void_reason = (
            "the lexicon's scope is the context map's code, and there is no map"
        )
        return result
    code = gather(context, parsed)
    files = 0
    for declared in parsed.contexts:
        applies = [
            entry
            for entry in entries
            if not entry.scope or declared.name in entry.scope
        ]
        if not applies:
            continue
        for path in code[declared.name].files:
            relative = mp.rel(context.root, path)
            try:
                named = declarations(path)
            except SyntaxError as error:
                result.findings.append(f"{relative}: does not parse: {error.msg}")
                continue
            files += 1
            result.examined += len(named)
            for number, name in named:
                pieces = segments(name)
                for entry in applies:
                    for word in entry.words:
                        if says(
                            pieces, segments(word.replace(" ", "_").replace("-", "_"))
                        ):
                            result.findings.append(
                                f'{relative}:{number}: declares {name}, which says "{word}" '
                                f'where the lexicon says "{entry.term}"'
                            )
    result.unit = f"declaration(s) in {files} file(s)"
    count = len(entries)
    result.notes.append(
        f"{count} entr{'y' if count == 1 else 'ies'} in {context.section('ddd')['lexicon']}"
    )
    result.void_reason = "no code in the lexicon's scope"
    return result


CLASSES = {
    "context-map-parses": (
        "the map parses: unique names, declared dependencies, no cycle, no nested paths",
        context_map_parses,
    ),
    "declared-edges-match-imports": (
        "each context's imports and manifests depend on exactly what the map declares",
        declared_edges_match_imports,
    ),
    "lexicon-locks": (
        "no declared identifier says a word the lexicon forbids in its context",
        lexicon_locks,
    ),
}


if __name__ == "__main__":
    sys.exit(mp.run("DDD", "ddd", CLASSES))

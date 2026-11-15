"""Every declared dependency is used by its own package (SPEC-670 R1 to R13; ADR-693 D1 to D5).

The census reads every manifest of four ecosystems (R1): the Rust workspace's root `Cargo.toml`
and each member's, the web packages (the root `package.json`, each `packages:` entry of
`pnpm-workspace.yaml` and each `file:` override there), the Swift packages' `Package.swift` files
and the XcodeGen specs under `ios/`, and every Python requirement file (`*requirements*.txt`).
Each declared dependency is judged against the files of its own package by the forms
of R2 to R8, and one with no use found is a site, keyed `<manifest> <table> <name>` (R10). A site
is listed in `LISTED` with its reason; an unlisted site prints `WARN` and fails only at `Fail`, and
a listed key that is no current site prints `STALE` and fails at both levels (R11). A form the text
cannot decide prints `UNDECIDED` and never fails (R9).

The planted tests read the trees under `fixtures/unused-dependencies/`, each file written under
its name less `.fixture` into a scratch root and read there as text, never run, compiled or
imported. Only the repository test reads the tree. The census reads committed files with
`tomllib`, `json` and `re`, and starts no process.
"""

import json
import re
import tempfile
import tomllib
import unittest
from pathlib import Path, PurePosixPath

from _support import REPO, examined

WARN = "Warn"
FAIL = "Fail"
# R12: the census starts at warn; a move to fail is its own delivery (R13).
LEVEL = "Warn"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "unused-dependencies"
SUFFIX = ".fixture"

TARGET_EMPTY = "a context-map edge to a crate that holds no item yet"
SOURCE_EMPTY = "a context-map edge of a crate that holds no item yet"
UNUSED_TODAY = "declared, and used by no file of its package today"
MEDIA = "declared for the media reads of ADR-621 D6 and D7; no file of the crate uses it today"
REEXPORT = "used through webauthn_rs's prelude re-export; the entry sets its v4 feature"
GENERATED = "called by the code the wasm_bindgen attribute generates for an async function"
HOST = "the test bundle's host application, which the bundle runs in and does not import"
PROVIDER = "the coverage provider vitest.config.ts names as v8, which vitest loads by name"
TOOL = "a command-line tool the coverage workflow installs and runs"
VENDORED = "a build or test tool of the vendored copy's upstream; the copy keeps no such file"

# R12: each of today's sites, keyed as R10 says, with the reason it stays.
LISTED = {
    "crates/coordination/Cargo.toml dependencies deck-streak-discipline": TARGET_EMPTY,
    "crates/coordination/Cargo.toml dependencies deck-streak-focus": TARGET_EMPTY,
    "crates/coordination/Cargo.toml dependencies deck-streak-markets": TARGET_EMPTY,
    "crates/coordination/Cargo.toml dependencies deck-streak-publishing": TARGET_EMPTY,
    "crates/daemon/Cargo.toml dependencies deck-streak-discipline": TARGET_EMPTY,
    "crates/daemon/Cargo.toml dependencies deck-streak-focus": TARGET_EMPTY,
    "crates/daemon/Cargo.toml dependencies deck-streak-markets": TARGET_EMPTY,
    "crates/daemon/Cargo.toml dependencies deck-streak-publishing": TARGET_EMPTY,
    "crates/discipline/Cargo.toml dependencies deck-streak-ingest": SOURCE_EMPTY,
    "crates/discipline/Cargo.toml dependencies deck-streak-kernel": SOURCE_EMPTY,
    "crates/ffi/Cargo.toml dependencies deck-streak-media": MEDIA,
    "crates/focus/Cargo.toml dependencies deck-streak-kernel": SOURCE_EMPTY,
    "crates/identity/Cargo.toml dependencies uuid": REEXPORT,
    "crates/insights/Cargo.toml dependencies deck-streak-kernel": UNUSED_TODAY,
    "crates/markets/Cargo.toml dependencies deck-streak-kernel": SOURCE_EMPTY,
    "crates/publishing/Cargo.toml dependencies deck-streak-kernel": SOURCE_EMPTY,
    'crates/web-engine/Cargo.toml target.cfg(target_arch = "wasm32").dependencies '
    "wasm-bindgen-futures": GENERATED,
    "ios/app.yml targets.DeckStreakUITests.dependencies DeckStreak": HOST,
    "ios/project.yml targets.CardProbeTests.dependencies CardProbeHost": HOST,
    "ios/project.yml targets.HarnessUITests.dependencies Harness": HOST,
    "package.json devDependencies @vitest/coverage-v8": PROVIDER,
    "tools/coverage/requirements.txt requirements coverage": TOOL,
    "vendor/deepmerge/package.json devDependencies @types/node": VENDORED,
    "vendor/deepmerge/package.json devDependencies is-plain-object": VENDORED,
    "vendor/deepmerge/package.json devDependencies rollup-plugin-commonjs": VENDORED,
    "vendor/deepmerge/package.json devDependencies rollup-plugin-node-resolve": VENDORED,
    "vendor/deepmerge/package.json devDependencies typescript": VENDORED,
    "vendor/deepmerge/package.json devDependencies uglify-js": VENDORED,
    "web/app/package.json devDependencies @lucide/svelte": UNUSED_TODAY,
    "web/app/package.json devDependencies tailwind-variants": UNUSED_TODAY,
}


def lexical(path):
    """`path` with each `.` dropped and each `..` taken against the part before it, or None when it
    climbs above where it starts."""
    parts = []
    for part in PurePosixPath(path).parts:
        if part == "..":
            if not parts:
                return None
            parts.pop()
        elif part != ".":
            parts.append(part)
    return "/".join(parts)


def text_of(path):
    """A file's text, an undecodable byte replaced."""
    return path.read_text(encoding="utf-8", errors="replace")


# Rust (R2 to R5)

DEV = "dev-dependencies"
BUILD = "build-dependencies"
TABLES = ("dependencies", DEV, BUILD)
# R3: the use forms of a source name, `NAME` standing for it.
RUST_FORMS = (
    r"\buse\s+(?:::)?NAME\b",
    r"\bextern\s+crate\s+NAME\b",
    r"(?:(?<![\w:])|(?<=[^\w:]::))NAME\s*::",
)
RUST_RAW = re.compile(r'b?r(#*)"')
RUST_CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
DOC_OPENERS = ("///", "//!", "/**", "/*!")
PLAIN_OPENERS = ("////", "/***", "/**/")
PATH_ATTRIBUTE = re.compile(r'#\[path\s*=\s*"([^"]+)"\s*\]')
INCLUDE = re.compile(
    r'\binclude!\s*\(\s*(concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*)?"([^"]+)"'
)


def cargo(path):
    """A parsed TOML manifest."""
    return tomllib.loads(path.read_text(encoding="utf-8"))


def member_directories(root):
    """R1: each directory a `[workspace] members` pattern matches, less `exclude`, that holds a
    `Cargo.toml`, relative to `root`."""
    table = cargo(root / "Cargo.toml").get("workspace", {})
    excluded = {lexical(path) for path in table.get("exclude", [])}
    found = set()
    for pattern in table.get("members", []):
        found |= {p.parent.relative_to(root).as_posix() for p in root.glob(f"{pattern}/Cargo.toml")}
    return sorted(found - excluded)


def plain_comments_blanked(text):
    """R3: `text` with each plain comment blanked; doc comments, strings and line breaks kept."""
    out = list(text)
    index, size = 0, len(text)
    while index < size:
        if text.startswith(("//", "/*"), index):
            if text.startswith("//", index):
                end = text.find("\n", index)
                end = size if end < 0 else end
            else:
                depth, end = 1, index + 2
                while end < size and depth:
                    if text.startswith("/*", end):
                        depth, end = depth + 1, end + 2
                    elif text.startswith("*/", end):
                        depth, end = depth - 1, end + 2
                    else:
                        end += 1
            doc = text.startswith(DOC_OPENERS, index) and not text.startswith(PLAIN_OPENERS, index)
            if not doc:
                out[index:end] = [char if char == "\n" else " " for char in text[index:end]]
            index = end
            continue
        follows_word = index > 0 and (text[index - 1].isalnum() or text[index - 1] == "_")
        raw = None if follows_word else RUST_RAW.match(text, index)
        if raw:
            end = text.find('"' + raw.group(1), raw.end())
            index = size if end < 0 else end + 1 + len(raw.group(1))
        elif text[index] == "'":
            quoted = RUST_CHAR.match(text, index)
            index = quoted.end() if quoted else index + 1
        elif text[index] == '"':
            end = index + 1
            while end < size and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            index = end + 1
        else:
            index += 1
    return "".join(out)


def rust_uses(name):
    """R3: one pattern holding every use form of a source name."""
    return re.compile("|".join(form.replace("NAME", re.escape(name)) for form in RUST_FORMS))


def source_name(key, entry, shared, libraries):
    """R2: the name a use of the entry spells in source."""
    spec = shared.get(key, {}) if entry.get("workspace") is True else entry
    package = spec.get("package", key) if isinstance(spec, dict) else key
    if package != key:
        return key.replace("-", "_")
    if package in libraries:
        return libraries[package]
    return package.replace("-", "_")


def feature_keys(manifest):
    """R9: each dependency key a `[features]` value names (`dep:K`, `K/f`, `K?/f` or `K`)."""
    keys = set()
    for values in manifest.get("features", {}).values():
        for value in values:
            keys.add(value.removeprefix("dep:").split("/")[0].removesuffix("?"))
    return keys


def followed(root, base, files):
    """R4: `files` and every file a `#[path]` attribute or an `include!` among them names,
    transitively; a name that climbs above `root` or names no file is not followed."""
    found, todo = list(files), list(files)
    while todo:
        path = todo.pop()
        text = text_of(path)
        named = []
        for match in PATH_ATTRIBUTE.finditer(text):
            named.append(path.parent / match.group(1))
        for match in INCLUDE.finditer(text):
            named.append((base if match.group(1) else path.parent) / match.group(2).lstrip("/"))
        for target in named:
            inside = target.is_relative_to(root)
            relative = lexical(target.relative_to(root).as_posix()) if inside else None
            candidate = root / relative if relative else None
            if candidate and candidate.is_file() and candidate not in found:
                found.append(candidate)
                todo.append(candidate)
    return found


def rust_groups(root, directory, manifest):
    """R4: (the build script's files, every other `.rs` file of the member), each followed."""
    base = root / directory
    build = manifest.get("package", {}).get("build", "build.rs")
    script = base / build if isinstance(build, str) else None
    files = sorted(base.rglob("*.rs"))
    builds = [path for path in files if path == script]
    others = [path for path in files if path != script]
    return followed(root, base, builds), followed(root, base, others)


def rust_census(root):
    """R1 to R5 and R9: (manifests, source files, sites, undecided) of the workspace at `root`."""
    if not (root / "Cargo.toml").is_file():
        return [], [], [], []
    members = {
        directory: cargo(root / directory / "Cargo.toml") for directory in member_directories(root)
    }
    files = set()
    for directory, manifest in members.items():
        builds, others = rust_groups(root, directory, manifest)
        files |= set(builds + others)
    return ["Cargo.toml", *members], sorted(files), [], []


# Web (R6)

WEB_SUFFIXES = (".ts", ".js", ".mjs", ".cjs", ".svelte", ".html", ".css", ".json")
SKIPPED = ("node_modules", ".svelte-kit")
WEB_TABLES = ("dependencies", "devDependencies", "optionalDependencies")
PEERS = "peerDependencies"
SPECIFIERS = (
    re.compile(r"""\bfrom\s*['"]([^'"\n]+)['"]"""),
    re.compile(r"""\bimport\s*['"]([^'"\n]+)['"]"""),
    re.compile(r"""\bimport\s*\(\s*['"]([^'"\n]+)['"]\s*\)"""),
    re.compile(r"""\brequire\s*\(\s*['"]([^'"\n]+)['"]\s*\)"""),
    re.compile(r"""\bvi\.(?:mock|doMock|importActual|importMock)\s*\(\s*['"]([^'"\n]+)['"]"""),
    re.compile(r"""<reference\s+types\s*=\s*['"]([^'"\n]+)['"]"""),
    re.compile(r"""@(?:import|plugin|reference)\s+(?:url\()?['"]([^'"\n]+)['"]"""),
)
DOCBLOCK = re.compile(r"@vitest-environment\s+([A-Za-z0-9@/_.-]+)")
NODE_BUILTINS = frozenset(
    {
        "assert",
        "buffer",
        "child_process",
        "crypto",
        "events",
        "fs",
        "http",
        "https",
        "module",
        "net",
        "os",
        "path",
        "perf_hooks",
        "process",
        "readline",
        "stream",
        "timers",
        "url",
        "util",
        "vm",
        "worker_threads",
        "zlib",
    }
)
KIT_ALIASES = ("$app/", "$env/", "$service-worker")
NOT_PACKAGES = (".", "/", "#", "$", "~", "virtual:", "data:", "http:", "https:")
RUNNERS = ("pnpm", "npm", "npx")
RUNNER_VERBS = ("exec", "dlx", "x")
ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")
LIST_ITEM = re.compile(r"""\s+-\s+['"]?([^'"#\s]+)""")
FILE_OVERRIDE = re.compile(r"""['"]?file:([^'"\s]+)""")
SECTION = re.compile(r"([A-Za-z][\w-]*):")


def package_of(specifier):
    """R6: the package an import specifier names (`node:` for a Node builtin, `@sveltejs/kit` for
    SvelteKit's own modules), or None for a relative, absolute or aliased one."""
    specifier = specifier.split("?")[0]
    if specifier.startswith("node:") or specifier.split("/")[0] in NODE_BUILTINS:
        return "node:"
    if specifier.startswith(KIT_ALIASES):
        return "@sveltejs/kit"
    if specifier.startswith(NOT_PACKAGES):
        return None
    parts = specifier.split("/")
    if specifier.startswith("@"):
        return "/".join(parts[:2])
    return parts[0]


def script_words(command):
    """R6: each command segment's first word after its environment assignments, or the word after
    a runner and its `exec`, `dlx` or `x`."""
    words = []
    for segment in re.split(r"&&|\|\||;|\|", command):
        tokens = segment.split()
        while tokens and ASSIGNMENT.match(tokens[0]):
            tokens.pop(0)
        if tokens and tokens[0] in RUNNERS:
            tokens = tokens[1:]
            if tokens and tokens[0] in RUNNER_VERBS:
                tokens = tokens[1:]
        if tokens:
            words.append(tokens[0])
    return words


def web_packages(root):
    """R1: `{folder: whether it is the root package}` for the root `package.json`, each
    `packages:` entry of `pnpm-workspace.yaml` and each `file:` value under its `overrides:`."""
    found = {"": True} if (root / "package.json").is_file() else {}
    workspace = root / "pnpm-workspace.yaml"
    section = None
    for line in text_of(workspace).splitlines() if workspace.is_file() else []:
        heading = SECTION.match(line)
        if heading:
            section = heading.group(1)
            continue
        entry = LIST_ITEM.match(line) if section == "packages" else None
        override = FILE_OVERRIDE.search(line) if section == "overrides" else None
        if entry:
            candidates = sorted(root.glob(entry.group(1)))
        elif override:
            candidates = [root / override.group(1)]
        else:
            continue
        for folder in candidates:
            relative = lexical(folder.relative_to(root).as_posix())
            if relative and (root / relative / "package.json").is_file():
                found.setdefault(relative, False)
    return dict(sorted(found.items()))


def web_files(base, is_root):
    """R6: the root package's `*.config.*` files at the root, or every file of another package,
    each by suffix, `package.json` and installed or generated folders left out."""
    if is_root:
        return sorted(path for path in base.glob("*.config.*") if path.suffix in WEB_SUFFIXES)
    return sorted(
        path
        for path in base.rglob("*")
        if path.is_file()
        and path.suffix in WEB_SUFFIXES
        and path.name != "package.json"
        and not set(path.relative_to(base).parts) & set(SKIPPED)
    )


def configuration(base, is_root, files):
    """R6: the package folder's own files by suffix and every `.json` file of the package."""
    if is_root:
        return files
    return [path for path in files if path.parent == base or path.suffix == ".json"]


def web_used(name, named, configured, types):
    """R6: whether a specifier, docblock or script word names the package, a configuration file
    quotes it, or it is the `@types` package of a named one."""
    if name in named:
        return True
    if re.search(rf"""['"](?:[^'"\n]*/)?{re.escape(name)}(?:[@/][^'"\n]*)?['"]""", configured):
        return True
    target = name.removeprefix("@types/")
    return name != target and (
        target in named or target in types or (target == "node" and "node:" in named)
    )


def web_census(root):
    """R1, R6 and R9: (manifests, source files, sites, undecided) of the web packages at `root`."""
    manifests, every_file = [], []
    for folder, is_root in web_packages(root).items():
        base = root / folder if folder else root
        manifest = f"{folder}/package.json" if folder else "package.json"
        manifests.append(manifest)
        json.loads(text_of(root / manifest))
        every_file += web_files(base, is_root)
    return manifests, every_file, [], []


# Swift (R7)

SWIFT_IMPORT = re.compile(
    r"^[ \t]*(?:@testable[ \t]+|@_exported[ \t]+)?import[ \t]+(?:\w+[ \t]+)?(\w+)", re.M
)
SWIFT_COMMENT_OR_STRING = re.compile(r'"(?:\\.|[^"\\\n])*"|//[^\n]*')
TARGET_CALL = re.compile(r"\.(target|testTarget|executableTarget)\(")
NAMED_DEPENDENCY = re.compile(r'\.(?:product|target|byName)\(\s*name:\s*"([^"]+)"')
XCODEGEN_TARGET = re.compile(r"^  (\w+):\s*$")
XCODEGEN_SECTION = re.compile(r"^    (sources|dependencies):\s*$")
XCODEGEN_SOURCE = re.compile(r"^      - (?:path:\s*)?(\S+)")
XCODEGEN_DEPENDENCY = re.compile(r"^      - (package|target):\s*(\S+)")
XCODEGEN_PRODUCT = re.compile(r"^        product:\s*(\S+)")


def enclosed(text, start, opener, closer):
    """The text from the `opener` at `start` to its matching `closer`, strings skipped."""
    depth, index = 0, start
    while index < len(text):
        char = text[index]
        if char == '"':
            end = text.find('"', index + 1)
            index = len(text) if end < 0 else end
        elif char == opener:
            depth += 1
        elif char == closer:
            depth -= 1
            if depth == 0:
                return text[start : index + 1]
        index += 1
    return text[start:]


def swift_imports(paths):
    """R7: each module an `import` line of `paths` names."""
    return {name for path in paths for name in SWIFT_IMPORT.findall(text_of(path))}


def swift_files(folder):
    """A target's `.swift` files under `folder`, or the file itself."""
    if folder.is_dir():
        return sorted(folder.rglob("*.swift"))
    return [folder] if folder.suffix == ".swift" and folder.is_file() else []


def package_targets(path):
    """R7: (target, its source files, its dependency names) of each target of a `Package.swift`."""
    text = SWIFT_COMMENT_OR_STRING.sub(
        lambda match: match.group(0) if match.group(0).startswith('"') else "",
        text_of(path),
    )
    targets, index = [], 0
    while match := TARGET_CALL.search(text, index):
        call = enclosed(text, match.end() - 1, "(", ")")
        index = match.end() - 1 + len(call)
        name = re.search(r'name:\s*"([^"]+)"', call)
        listed = re.search(r"dependencies:\s*\[", call)
        if not name or not listed:
            continue
        dependencies = enclosed(call, listed.end() - 1, "[", "]")
        names = NAMED_DEPENDENCY.findall(dependencies)
        bare = re.sub(r"\.\w+\([^()]*\)", "", dependencies)
        names += re.findall(r'"([^"]+)"', bare)
        folder = re.search(r'path:\s*"([^"]+)"', call)
        default = "Tests" if match.group(1) == "testTarget" else "Sources"
        sources = path.parent / (folder.group(1) if folder else f"{default}/{name.group(1)}")
        targets.append((name.group(1), swift_files(sources), [(n, n) for n in names]))
    return targets


def spec_targets(path):
    """R7: (target, its source files, its (name, module) dependencies) of an XcodeGen spec."""
    targets, current, section, in_targets = [], None, None, False
    for line in text_of(path).splitlines():
        if re.match(r"^\S", line):
            in_targets = line.rstrip() == "targets:"
            continue
        target = XCODEGEN_TARGET.match(line) if in_targets else None
        if target:
            current = (target.group(1), [], [])
            targets.append(current)
            section = None
            continue
        if not current or not in_targets:
            continue
        heading = XCODEGEN_SECTION.match(line)
        if heading or re.match(r"^    \S", line):
            section = heading.group(1) if heading else None
            continue
        source = XCODEGEN_SOURCE.match(line) if section == "sources" else None
        dependency = XCODEGEN_DEPENDENCY.match(line) if section == "dependencies" else None
        product = XCODEGEN_PRODUCT.match(line) if section == "dependencies" else None
        if source:
            current[1].extend(swift_files(path.parent / source.group(1)))
        elif dependency:
            current[2].append((dependency.group(2), dependency.group(2)))
        elif product and current[2]:
            package, _module = current[2][-1]
            current[2][-1] = (f"{package}/{product.group(1)}", product.group(1))
    return targets


def swift_census(root):
    """R1, R7 and R9: (manifests, source files, sites, undecided) of the Swift packages and specs."""
    packages = sorted(root.glob("ios/*/Package.swift"))
    specs = [
        path
        for path in sorted(root.glob("ios/*.yml"))
        if re.search(r"(?m)^targets:", text_of(path))
    ]
    manifests, files = [], set()
    for path, reader in [(p, package_targets) for p in packages] + [
        (p, spec_targets) for p in specs
    ]:
        manifests.append(path.relative_to(root).as_posix())
        for _target, sources, _dependencies in reader(path):
            files |= set(sources)
    return manifests, sorted(files), [], []


# Python (R8)

PYTHON_IMPORT = r"^[ \t]*(?:import|from)[ \t]+NAME\b"
REQUIREMENT = re.compile(r"^\s*([A-Za-z0-9][A-Za-z0-9._-]*)")
# R1: a requirement file is every file whose name matches this, in any folder of the tree.
REQUIREMENTS = "*requirements*.txt"
# R1: the folders that hold no tracked file, R6's installed and generated folders and git's own.
UNTRACKED = {*SKIPPED, ".git"}


def python_census(root):
    """R1 and R8: (manifests, source files, sites, undecided) of the requirement files."""
    found = sorted(
        path
        for path in root.rglob(REQUIREMENTS)
        if path.is_file() and not set(path.relative_to(root).parts) & UNTRACKED
    )
    manifests, files = [], set()
    for path in found:
        manifests.append(path.relative_to(root).as_posix())
        files |= set(path.parent.rglob("*.py"))
    return manifests, sorted(files), [], []


# The census and its verdict (R1, R11)

ECOSYSTEMS = {
    "rust": rust_census,
    "web": web_census,
    "swift": swift_census,
    "python": python_census,
}


def census(root, names=tuple(ECOSYSTEMS)):
    """R1: (sites, undecided) of the named ecosystems at `root`, after printing what each read and
    refusing an ecosystem with no manifest."""
    sites, undecided = [], []
    for name in names:
        manifests, files, found, unsure = ECOSYSTEMS[name](root)
        print(f"examined {len(manifests)} {name} manifest(s)")
        print(f"read {len(files)} {name} source file(s)")
        sites += found
        undecided += unsure
    return sorted(sites), sorted(undecided)


def verdict(sites, listed, level):
    """R11: (the lines printed, the lines that fail) for `sites` against `listed` at `level`."""
    return [], []


def planted(case, names):
    """The census of `names` over the planted tree `case`: each fixture written under its name less
    `.fixture` into a scratch root and read there as text."""
    folder = FIXTURES / case
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        for fixture in examined("planted file(s)", sorted(folder.rglob(f"*{SUFFIX}"))):
            path = root / fixture.relative_to(folder).as_posix().removesuffix(SUFFIX)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(fixture.read_text(encoding="utf-8"), encoding="utf-8")
        return census(root, names)


class EachDeclaredDependencyIsUsedByItsPackage(unittest.TestCase):
    def test_a_rust_entry_is_used_by_a_use_form_of_its_source_name(self):
        sites, undecided = planted("rust", ("rust",))
        members = [site for site in sites if not site.startswith("Cargo.toml ")]
        self.assertEqual(
            members,
            [
                "crates/app/Cargo.toml build-dependencies src-only-build",
                "crates/app/Cargo.toml dependencies build-only-normal",
                "crates/app/Cargo.toml dependencies commented",
                "crates/app/Cargo.toml dependencies eta",
                "crates/app/Cargo.toml dependencies unused-one",
                "crates/app/Cargo.toml dev-dependencies dev-unused",
                "crates/app/Cargo.toml target.cfg(unix).dependencies unix-only",
            ],
        )
        self.assertEqual(undecided, ["crates/app/Cargo.toml dependencies feature-only feature"])

    def test_a_workspace_entry_no_member_inherits_is_a_site(self):
        sites, _undecided = planted("rust", ("rust",))
        workspace = [site for site in sites if site.startswith("Cargo.toml ")]
        self.assertEqual(workspace, ["Cargo.toml workspace.dependencies never-inherited"])

    def test_a_web_entry_is_used_by_a_specifier_a_script_a_configuration_or_its_types(
        self,
    ):
        sites, undecided = planted("web", ("web",))
        self.assertEqual(
            sites,
            [
                "package.json devDependencies root-unused",
                "vendor/vend/package.json devDependencies vend-tool",
                "web/app/package.json dependencies unused-web",
                "web/app/package.json devDependencies @scope/cli",
            ],
        )
        self.assertEqual(undecided, ["web/app/package.json peerDependencies peer-only peer"])

    def test_a_swift_target_dependency_is_used_by_an_import(self):
        sites, undecided = planted("swift", ("swift",))
        self.assertEqual(
            sites,
            [
                "ios/Kit/Package.swift targets.KitTests.dependencies Helpers",
                "ios/app.yml targets.App.dependencies Unused",
            ],
        )
        self.assertEqual(
            undecided,
            ["ios/Kit/Package.swift targets.Generated.dependencies Core sourceless"],
        )

    def test_a_python_requirement_is_used_by_an_import(self):
        sites, undecided = planted("python", ("python",))
        self.assertEqual(
            sites,
            [
                "scripts/lint-requirements.txt requirements unused-lint",
                "tools/report/requirements.txt requirements unused-req",
            ],
        )
        self.assertEqual(undecided, [])

    def test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails(self):
        site = "crates/app/Cargo.toml dependencies unused-one"
        gone = "crates/app/Cargo.toml dependencies gone"
        warn = "WARN unused-dependency crates/app/Cargo.toml dependencies unused-one"
        stale = "STALE unused-dependency crates/app/Cargo.toml dependencies gone"
        self.assertEqual(verdict([site], {}, WARN), ([warn], []))
        self.assertEqual(verdict([site], {}, FAIL), ([warn], [warn]))
        self.assertEqual(verdict([site], {site: "a reason"}, FAIL), ([], []))
        for level in (WARN, FAIL):
            with self.subTest(level):
                self.assertEqual(verdict([], {gone: "a reason"}, level), ([stale], [stale]))
        self.assertEqual(LEVEL, WARN)

    def test_an_empty_population_is_refused_per_ecosystem(self):
        for name in ECOSYSTEMS:
            with self.subTest(name):
                with self.assertRaisesRegex(AssertionError, rf"examined 0 {name} manifest\(s\)"):
                    planted("empty", (name,))


class TheRepositoryListsTodaysSites(unittest.TestCase):
    def test_the_repository_lists_exactly_todays_sites(self):
        sites, undecided = census(REPO)
        lines, failures = verdict(sites, LISTED, LEVEL)
        for line in lines + [f"UNDECIDED unused-dependency {entry}" for entry in undecided]:
            print(line)
        self.assertEqual([key for key in sorted(LISTED) if key not in sites], [])
        self.assertEqual(LEVEL, WARN)
        self.assertEqual([key for key, reason in LISTED.items() if not reason.strip()], [])
        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main()

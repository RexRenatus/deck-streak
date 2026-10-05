"""The app links one Rust static library, the umbrella FFI crate (SPEC-346 R1 to R4, ADR-357): no
member declares the `staticlib` crate type, UniFFI is named by the umbrella alone, the Swift side
links one local binary target and no generator spec names a framework or a remote package, and no
built library is committed. `umbrella_closure` is the one reader of the umbrella's build closure;
test_ci_workflows imports it, the function alone, for the change caller's census (A8).

Each census is a function of the data it is handed, so every planted control is a dict or a list
and never a repository: the tests hand each the live tree's data, then planted data it must refuse
by name."""

import re
import subprocess
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined

# The umbrella FFI crate, by its package name (ADR-357 D1).
UMBRELLA = "deck-streak-ffi"
# The dependency tables cargo reads; each may also sit under `[target.<cfg>]`.
NORMAL = "dependencies"
BUILD = "build-dependencies"
DEV = "dev-dependencies"
# The one crate type a member declares today: the web engine's (ADR-348).
DECLARED_CRATE_TYPES = {"deck-streak-web-engine": ["cdylib", "rlib"]}
# The Swift side's one binary target, as (name, how it is found, where), in the engine package.
ENGINE_PACKAGE = "ios/EnginePackage/Package.swift"
BINARY_TARGET = ("deck_streak_ffiFFI", "path", "DeckStreakFFI.xcframework")
BINARY = re.compile(r'\.binaryTarget\(\s*name:\s*"([^"]*)"\s*,\s*(path|url):\s*"([^"]*)"')
REMOTE_PACKAGE = re.compile(r"\.package\(\s*url:")
SPEC_DEPENDENCY = re.compile(r"(?m)^\s*-\s*(framework|carthage):")
SPEC_URL = re.compile(r"^\s+(?:-\s+)?url:")
TOP_LEVEL_PACKAGES = re.compile(r"packages:\s*(?:#.*)?$")
# A built library: an archive or a shared object, or anything inside a framework bundle.
BUILT_LIBRARY = re.compile(r"\.(a|dylib|so)$|\.xcframework/|\.framework/")


def manifests(root):
    """{`crates/<dir>`: its parsed manifest} for each member (`members = ["crates/*"]`)."""
    found = {}
    for path in sorted(Path(root).glob("crates/*/Cargo.toml")):
        found[f"crates/{path.parent.name}"] = tomllib.loads(path.read_text(encoding="utf-8"))
    return found


def members(found):
    """{package name: `crates/<dir>`} for the members `manifests` read."""
    return {manifest["package"]["name"]: directory for directory, manifest in found.items()}


def dependencies(manifest, workspace, tables):
    """Every dependency `manifest` declares in `tables` and in each `[target.<cfg>]` copy of them,
    as (table, package): a table spec's `package` if it has one, else its key, and a `workspace =
    true` spec resolved through the root's `[workspace.dependencies]` the same way. tomllib folds a
    `[dependencies.x]` table into the same dict, so a dotted table is read like an inline one."""
    shared = workspace.get("workspace", {}).get("dependencies", {})
    sections = [(table, manifest.get(table, {})) for table in tables]
    for cfg, body in manifest.get("target", {}).items():
        sections += [(f"target.{cfg}.{table}", body.get(table, {})) for table in tables]
    found = []
    for table, section in sections:
        for key, spec in section.items():
            if isinstance(spec, dict) and spec.get("workspace") is True:
                spec = shared.get(key, {})
            name = spec.get("package", key) if isinstance(spec, dict) else key
            found.append((table, name))
    return found


def closure_of(found, workspace, umbrella=UMBRELLA):
    """The members the umbrella's build links, as sorted `crates/<dir>` directories: the umbrella;
    every member its normal, build, target and dev tables name; and every member those members'
    normal, build and target tables name in turn, to a fixed point (SPEC-346 R6)."""
    by_name = members(found)
    start = by_name[umbrella]
    reached = {start}
    todo = [
        by_name[name]
        for _table, name in dependencies(found[start], workspace, (NORMAL, BUILD, DEV))
        if name in by_name
    ]
    while todo:
        directory = todo.pop()
        if directory in reached:
            continue
        reached.add(directory)
        todo += [
            by_name[name]
            for _table, name in dependencies(found[directory], workspace, (NORMAL, BUILD))
            if name in by_name
        ]
    return sorted(reached)


def umbrella_closure(root, umbrella=UMBRELLA):
    """The umbrella's build closure in the workspace at `root`, read from its manifests alone."""
    workspace = tomllib.loads((Path(root) / "Cargo.toml").read_text(encoding="utf-8"))
    return closure_of(manifests(root), workspace, umbrella)


def crate_types(found):
    """{package name: crate types} for every member whose `[lib]` declares any, under either
    spelling cargo reads, `crate-type` or `crate_type`."""
    census = {}
    for manifest in found.values():
        lib = manifest.get("lib", {})
        declared = [*lib.get("crate-type", []), *lib.get("crate_type", [])]
        if declared:
            census[manifest["package"]["name"]] = declared
    return census


def staticlib_problems(found):
    """Every member that declares the `staticlib` crate type, by name (SPEC-346 R3)."""
    return [
        f"{name} declares the staticlib crate type: {declared}"
        for name, declared in sorted(crate_types(found).items())
        if "staticlib" in declared
    ]


def lock_dependents(lock, package):
    """The sorted names of the lockfile's packages that depend on `package`, at any version."""
    return sorted(
        entry["name"]
        for entry in lock.get("package", [])
        if any(each.split(" ")[0] == package for each in entry.get("dependencies", []))
    )


def uniffi_problems(found, workspace, lock, umbrella=UMBRELLA):
    """Where UniFFI leaves the umbrella, and where a member could export an unmangled symbol of its
    own (SPEC-346 R2): a member but the umbrella naming `uniffi` in any dependency table, a
    lockfile package but the umbrella depending on it, a member that does not inherit the
    workspace lints, and a workspace that does not forbid unsafe code."""
    problems = []
    for directory, manifest in sorted(found.items()):
        name = manifest["package"]["name"]
        for table, package in dependencies(manifest, workspace, (NORMAL, BUILD, DEV)):
            if package == "uniffi" and name != umbrella:
                problems.append(f"{directory} names uniffi under [{table}]")
        if manifest.get("lints", {}).get("workspace") is not True:
            problems.append(f"{directory} does not inherit the workspace lints")
    for dependent in lock_dependents(lock, "uniffi"):
        if dependent != umbrella:
            problems.append(f"Cargo.lock: {dependent} depends on uniffi")
    lints = workspace.get("workspace", {}).get("lints", {}).get("rust", {})
    if lints.get("unsafe_code") != "forbid":
        problems.append("Cargo.toml: the workspace does not forbid unsafe code")
    return problems


def tracked_paths(root):
    """Every path git tracks at `root`, as git lists it."""
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"], capture_output=True, text=True, check=True
    )
    return [path for path in listed.stdout.split("\0") if path]


def swift_packages(tracked):
    """The tracked Swift package manifests under `ios/`."""
    return [path for path in tracked if path.startswith("ios/") and path.endswith("Package.swift")]


def generator_specs(tracked):
    """The tracked project generator specs, each `ios/*.yml` git lists, where `*` crosses `/`."""
    return [path for path in tracked if path.startswith("ios/") and path.endswith(".yml")]


def binary_targets(files, tracked):
    """Every binary target across the Swift packages, as (package, name, how it is found, where)."""
    return [
        (path, *match) for path in swift_packages(tracked) for match in BINARY.findall(files[path])
    ]


def spec_problems(path, text):
    """What one generator spec gets wrong: a framework or Carthage dependency anywhere, or a `url:`
    inside its top-level `packages:` block, the lines after a column-0 `packages:` up to the next
    column-0 key."""
    problems = [f"{path} declares a {kind} dependency" for kind in SPEC_DEPENDENCY.findall(text)]
    inside = False
    for line in text.splitlines():
        if line and not line[0].isspace() and not line.startswith("#"):
            inside = bool(TOP_LEVEL_PACKAGES.match(line))
        elif inside and SPEC_URL.match(line):
            problems.append(f"{path} declares a remote package: {line.strip()}")
    return problems


def swift_side_problems(files, tracked):
    """What the Swift side gets wrong about linking one Rust library (SPEC-346 R4). `tracked` is
    the tracked paths, and `files` the text of each tracked Swift package and generator spec."""
    packages = examined("Swift packages", swift_packages(tracked))
    specs = examined("generator specs", generator_specs(tracked))
    problems = []
    found = binary_targets(files, tracked)
    if len(found) != 1:
        problems.append(f"{len(found)} binary targets across the Swift packages, not one")
    for path, *target in found:
        if tuple(target) != BINARY_TARGET:
            problems.append(f"{path}: binary target {target}, not {list(BINARY_TARGET)}")
    for path in packages:
        held = files[path].count(".binaryTarget(")
        if held != len(BINARY.findall(files[path])):
            problems.append(f"{path}: a binary target the census cannot read")
        if REMOTE_PACKAGE.search(files[path]):
            problems.append(f"{path} declares a remote package")
    for path in specs:
        problems += spec_problems(path, files[path])
    problems += [
        f"{path} is committed under ios/EnginePackage/, which commits Package.swift alone"
        for path in tracked
        if path.startswith("ios/EnginePackage/") and path != ENGINE_PACKAGE
    ]
    return problems


def built_library_problems(tracked):
    """Every tracked path that is a built library or sits inside a framework bundle."""
    return [f"{path} is a built library" for path in tracked if BUILT_LIBRARY.search(path)]


def swift_side(root):
    """(files, tracked) for the Swift side census, read from the tree at `root`."""
    tracked = tracked_paths(root)
    files = {}
    for path in swift_packages(tracked) + generator_specs(tracked):
        files[path] = (Path(root) / path).read_text(encoding="utf-8")
    return files, tracked


def planted(text):
    """A planted member manifest: a package `x` that inherits the workspace lints."""
    return tomllib.loads('[package]\nname = "x"\n\n[lints]\nworkspace = true\n\n' + text)


class NoMemberIsAStaticLibrary(unittest.TestCase):
    def test_no_member_declares_the_staticlib_crate_type(self):
        found = manifests(REPO)
        self.assertEqual(crate_types(found), DECLARED_CRATE_TYPES)
        self.assertEqual(staticlib_problems(found), [])
        examined("member manifests", found)
        for spelling in ("crate-type", "crate_type"):
            with self.subTest(spelling):
                plant = {
                    **found,
                    "crates/x": planted(f'[lib]\n{spelling} = ["lib", "staticlib"]\n'),
                }
                self.assertEqual(
                    staticlib_problems(plant),
                    ["x declares the staticlib crate type: ['lib', 'staticlib']"],
                )


class UniffiLivesInTheUmbrella(unittest.TestCase):
    def test_only_the_umbrella_names_uniffi(self):
        found = manifests(REPO)
        workspace = tomllib.loads((REPO / "Cargo.toml").read_text(encoding="utf-8"))
        lock = tomllib.loads((REPO / "Cargo.lock").read_text(encoding="utf-8"))
        self.assertEqual(uniffi_problems(found, workspace, lock), [])
        umbrella = found[members(found)[UMBRELLA]]
        self.assertIn(("dependencies", "uniffi"), dependencies(umbrella, workspace, (NORMAL,)))
        self.assertEqual(lock_dependents(lock, "uniffi"), [UMBRELLA])
        examined("member manifests", found)
        examined("lockfile packages", lock["package"])
        plants = (
            (
                "[dependencies]\nuniffi.workspace = true\n",
                "crates/x names uniffi under [dependencies]",
            ),
            (
                "[target.'cfg(unix)'.dependencies]\nuniffi = \"1\"\n",
                "crates/x names uniffi under [target.cfg(unix).dependencies]",
            ),
            (
                '[dependencies.uniffi]\nversion = "1"\n',
                "crates/x names uniffi under [dependencies]",
            ),
            (
                '[build-dependencies]\nffi_rt = { package = "uniffi", version = "1" }\n',
                "crates/x names uniffi under [build-dependencies]",
            ),
        )
        for text, problem in plants:
            with self.subTest(text):
                plant = {**found, "crates/x": planted(text)}
                self.assertEqual(uniffi_problems(plant, workspace, lock), [problem])
        unlinted = {**found, "crates/x": tomllib.loads('[package]\nname = "x"\n')}
        self.assertEqual(
            uniffi_problems(unlinted, workspace, lock),
            ["crates/x does not inherit the workspace lints"],
        )
        reexport = {"package": [*lock["package"], {"name": "x", "dependencies": ["uniffi 1.0.0"]}]}
        self.assertEqual(
            uniffi_problems(found, workspace, reexport), ["Cargo.lock: x depends on uniffi"]
        )


class TheSwiftSideLinksOneLibrary(unittest.TestCase):
    def test_the_engine_package_has_one_local_binary_target(self):
        files, tracked = swift_side(REPO)
        self.assertEqual(swift_side_problems(files, tracked), [])
        self.assertEqual(binary_targets(files, tracked), [(ENGINE_PACKAGE, *BINARY_TARGET)])
        engine = files[ENGINE_PACKAGE]
        line = '.binaryTarget(name: "deck_streak_ffiFFI", path: "DeckStreakFFI.xcframework"),'
        self.assertEqual(engine.count(line), 1)
        wire = "ios/HarnessWire/Package.swift"
        project = "ios/project.yml"
        dependency = "      - package: HarnessWire\n"
        self.assertEqual(files[project].count(dependency), 1)
        app = "ios/app.yml"
        clean = (
            "name: App\npackages:\n  EnginePackage:\n    path: EnginePackage\ntargets:\n"
            "  App:\n    type: application\n    dependencies:\n      - package: EnginePackage\n"
        )
        xp = '.binaryTarget(name: "deck_streak_xpFFI", path: "DeckStreakXP.xcframework"),'
        second = f"{line}\n        {xp}"
        plants = (
            (
                "a second binary target",
                {ENGINE_PACKAGE: engine.replace(line, second)},
                [],
                [
                    "2 binary targets across the Swift packages, not one",
                    f"{ENGINE_PACKAGE}: binary target ['deck_streak_xpFFI', 'path', "
                    "'DeckStreakXP.xcframework'], not ['deck_streak_ffiFFI', 'path', "
                    "'DeckStreakFFI.xcframework']",
                ],
            ),
            (
                "a binary target by URL",
                {
                    ENGINE_PACKAGE: engine.replace(
                        'path: "DeckStreakFFI.xcframework"',
                        'url: "https://example.invalid/x.zip", checksum: "00"',
                    )
                },
                [],
                [
                    f"{ENGINE_PACKAGE}: binary target ['deck_streak_ffiFFI', 'url', "
                    "'https://example.invalid/x.zip'], not ['deck_streak_ffiFFI', 'path', "
                    "'DeckStreakFFI.xcframework']"
                ],
            ),
            (
                "a remote package",
                {
                    wire: files[wire].replace(
                        "    targets: [",
                        '    dependencies: [.package(url: "https://example.invalid/x.git", '
                        'from: "1.0.0")],\n    targets: [',
                    )
                },
                [],
                [f"{wire} declares a remote package"],
            ),
            (
                "a project framework dependency",
                {project: files[project].replace(dependency, "      - framework: X.xcframework\n")},
                [],
                [f"{project} declares a framework dependency"],
            ),
            (
                "a second generator spec with a framework dependency",
                {app: clean + "      - framework: X.xcframework\n"},
                [app],
                [f"{app} declares a framework dependency"],
            ),
            (
                "a second generator spec with a Carthage dependency",
                {app: clean + "      - carthage: X\n"},
                [app],
                [f"{app} declares a carthage dependency"],
            ),
            (
                "a second generator spec with a remote package",
                {
                    app: clean.replace(
                        "targets:\n", "  X:\n    url: https://example.invalid/x.git\ntargets:\n"
                    )
                },
                [app],
                [f"{app} declares a remote package: url: https://example.invalid/x.git"],
            ),
            ("a second generator spec, clean", {app: clean}, [app], []),
            (
                "a committed Swift source in the engine package",
                {},
                ["ios/EnginePackage/Sources/DeckStreakFFI/x.swift"],
                [
                    "ios/EnginePackage/Sources/DeckStreakFFI/x.swift is committed under "
                    "ios/EnginePackage/, which commits Package.swift alone"
                ],
            ),
            (
                "no binary target",
                {ENGINE_PACKAGE: engine.replace(line, "")},
                [],
                ["0 binary targets across the Swift packages, not one"],
            ),
        )
        for label, changed, added, problems in examined("planted Swift sides", plants):
            with self.subTest(label):
                for path, text in changed.items():
                    self.assertNotEqual(text, files.get(path), label)
                plant = {**files, **changed}
                self.assertEqual(swift_side_problems(plant, [*tracked, *added]), problems)

    def test_no_built_library_is_committed(self):
        tracked = examined("tracked paths", tracked_paths(REPO))
        self.assertEqual(built_library_problems(tracked), [])
        self.assertIn(ENGINE_PACKAGE, tracked)
        plant = [*tracked, "x/libfoo.a", "y/Foo.xcframework/Info.plist", "z/libbar.dylib"]
        self.assertEqual(
            built_library_problems(plant),
            [
                "x/libfoo.a is a built library",
                "y/Foo.xcframework/Info.plist is a built library",
                "z/libbar.dylib is a built library",
            ],
        )


if __name__ == "__main__":
    unittest.main()

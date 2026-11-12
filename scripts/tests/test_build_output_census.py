"""No tracked path sits under a directory a build tool, package manager, test runner or cache
writes (SPEC-672 R1 to R12, ADR-695): a census of every path git tracks, at warn.

The census is a function of the path list it is handed. `git ls-files` lists the live tree's
tracked paths; each planted control is a path list read as text from a `.fixture` file under
`scripts/tests/fixtures/build-output/`, never a directory on disk, so a control adds no site. A
path is a site when a directory above it is one a tool writes by default, one this tree's own
configuration names as a tool's output, or a name a source tree may also use standing beside the
manifest of the tool that writes it there; or when its own file name is a cache file's. A listed
site passes. At `LEVEL` warn an unlisted site prints one `WARN` line and passes; at fail it fails.
At both levels a listed path that is no longer a site fails, so a removal also removes its entry.
"""

import re
import subprocess
import unittest

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "build-output"
RULE = "build-output"
# R11: the level. Moving it to fail is a delivery of its own (ADR-695 D5).
LEVEL = "warn"

# R2: a directory only a tool writes, by its exact name, with the tool that writes it.
TOOL_DIRS = {
    "node_modules": "npm or pnpm",
    "__pycache__": "CPython",
    ".pytest_cache": "pytest",
    ".ruff_cache": "ruff",
    "htmlcov": "coverage.py",
    ".svelte-kit": "SvelteKit",
    ".vite": "Vite",
    ".build": "SwiftPM",
    ".swiftpm": "SwiftPM",
    "DerivedData": "Xcode",
    "xcuserdata": "Xcode",
    ".lake": "Lake",
    "mutants.out": "cargo-mutants",
    "mutants.out.old": "cargo-mutants",
    ".stryker-tmp": "StrykerJS",
    "test-results": "Playwright",
    "playwright-report": "Playwright",
    "blob-report": "Playwright",
    ".check-logs": "the gate",
}
# R2: a directory a tool writes, by the suffix of its name.
TOOL_SUFFIXES = {".xcresult": "xcodebuild"}
# R3: a cache file, by its own name.
CACHE_FILES = (
    (re.compile(r"\.pyc\Z"), "CPython", "*.pyc"),
    (re.compile(r"\.pyo\Z"), "CPython", "*.pyo"),
    (re.compile(r"\A\.coverage(?:\..+)?\Z"), "coverage.py", ".coverage"),
    (re.compile(r"\Avite\.config\.[cm]?[jt]s\.timestamp-"), "Vite", "vite.config.*.timestamp-*"),
)
# R4: a name a source tree may also use. It is a site only where the directory holding it also
# holds a tracked manifest of the tool that writes it there; anywhere else it is source.
SHARED = {
    "target": ("cargo", "Cargo.toml"),
    "build": ("SvelteKit or Vite", "package.json"),
    "dist": ("Vite", "package.json"),
    "coverage": ("Vitest", "package.json"),
    "reports": ("StrykerJS", "package.json"),
}
# R5: a directory this tree's own configuration names as a tool's output.
CONFIGURED = (
    (re.compile(r"\Aweb/app/src/lib/paraglide\Z"), "Paraglide"),
    (re.compile(r"\Aweb/app/project\.inlang/cache\Z"), "inlang"),
    (re.compile(r"\Aios/[^/]+\.xcodeproj\Z"), "XcodeGen"),
    (re.compile(r"\Aios/EnginePackage/DeckStreakFFI\.xcframework\Z"), "xcodebuild"),
    (re.compile(r"\Aios/EnginePackage/Sources/DeckStreakFFI\Z"), "UniFFI"),
)
# R10: today's sites, each a tracked path with its tool, its form and why the tree keeps it.
LISTED = (
    (
        "scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/mutants.out/log/baseline.log",
        "cargo-mutants",
        "mutants.out/",
        "a recorded shard report the not-started-legs census replays, tracked by force (SPEC-290)",
    ),
    (
        "scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/mutants.out/outcomes.json",
        "cargo-mutants",
        "mutants.out/",
        "a recorded shard report the not-started-legs census replays, tracked by force (SPEC-290)",
    ),
    (
        "vendor/deepmerge/dist/cjs.js",
        "Vite",
        "dist/",
        "the vendored fixed copy's built entry point, the package's main (SPEC-499 R2)",
    ),
    (
        "vendor/deepmerge/dist/umd.js",
        "Vite",
        "dist/",
        "the vendored fixed copy's second built file (SPEC-499 R2)",
    ),
)


def tracked_paths(root=REPO):
    """Every path git tracks at `root`, as `git ls-files` lists it (R1)."""
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        capture_output=True,
        text=True,
        check=True,
    )
    return [path for path in listed.stdout.split("\0") if path]


def planted(name):
    """The paths a `.fixture` file plants, one per line; a line opening `#` says why (R12)."""
    text = (FIXTURES / name).read_text(encoding="utf-8")
    return [line for line in text.splitlines() if line and not line.startswith("#")]


def holders(paths):
    """{directory: the names of the files tracked directly in it}, the root as ''."""
    held = {}
    for path in paths:
        head, _, name = path.rpartition("/")
        held.setdefault(head, set()).add(name)
    return held


def site(path, held):
    """(tool, form) when `path` is a site, else None. Its directories are read from the root
    down and the first that a tool writes decides (R2, R4, R5); then its own name (R3)."""
    parts = path.split("/")
    for depth, name in enumerate(parts[:-1]):
        directory = "/".join(parts[: depth + 1])
        for pattern, tool in CONFIGURED:
            if pattern.search(directory):
                return tool, directory + "/"
        if name in TOOL_DIRS:
            return TOOL_DIRS[name], name + "/"
        for suffix, tool in TOOL_SUFFIXES.items():
            if name.endswith(suffix):
                return tool, "*" + suffix + "/"
        if name in SHARED:
            tool, manifest = SHARED[name]
            if manifest in held.get("/".join(parts[:depth]), set()):
                return tool, name + "/"
    for pattern, tool, form in CACHE_FILES:
        if pattern.search(parts[-1]):
            return tool, form
    return None


def sites(paths):
    """{path: (tool, form)} for every site among `paths`; an empty one is refused (R1, R9)."""
    paths = examined("tracked path(s)", paths)
    held = holders(paths)
    found = {}
    for path in paths:
        hit = site(path, held)
        if hit is not None:
            found[path] = hit
    return found


def verdict(found, listed, level):
    """(printed lines, failures) for the sites `found` against the paths `listed` at `level`: an
    unlisted site is a line at warn and a failure at fail, and a listed path that is no longer a
    site is a failure at both (R6 to R8)."""
    lines, failures = [], []
    for path, (tool, form) in sorted(found.items()):
        if path in listed:
            continue
        line = f"{level.upper()} {RULE} {path} ({tool}, {form})"
        (lines if level == "warn" else failures).append(line)
    for path in sorted(set(listed) - set(found)):
        failures.append(f"STALE {RULE} {path}: listed, but it is no site")
    return lines, failures


class PlantedPathListsAreJudged(unittest.TestCase):
    def test_a_path_under_a_directory_a_tool_writes_is_a_site(self):
        self.assertEqual(sites(planted("sites.fixture")), PLANTED_SITES)

    def test_a_lookalike_or_a_source_tree_is_no_site(self):
        control = "web/app/node_modules/vite/package.json"
        found = sites([*planted("clean.fixture"), control])
        self.assertEqual(found, {control: ("npm or pnpm", "node_modules/")})

    def test_the_level_decides_an_unlisted_site_and_a_stale_entry_always_fails(self):
        self.assertEqual(LEVEL, "warn")
        node = "web/app/node_modules/vite/package.json"
        found = sites(["Cargo.toml", "target/debug/app", node])
        warned = f"WARN build-output {node} (npm or pnpm, node_modules/)"
        failed = f"FAIL build-output {node} (npm or pnpm, node_modules/)"
        stale = "STALE build-output dist/gone.js: listed, but it is no site"
        self.assertEqual(
            verdict(found, {"target/debug/app", "dist/gone.js"}, "warn"), ([warned], [stale])
        )
        self.assertEqual(verdict(found, {"target/debug/app"}, "fail"), ([], [failed]))
        listed = {"target/debug/app", node}
        self.assertEqual(verdict(found, listed, "warn"), ([], []))
        self.assertEqual(verdict(found, listed, "fail"), ([], []))

    def test_an_empty_population_is_refused(self):
        with self.assertRaisesRegex(AssertionError, r"\Aexamined 0 tracked path\(s\): "):
            sites(planted("empty.fixture"))


class TheRepositoryIsCensused(unittest.TestCase):
    def test_every_listed_path_is_a_site_of_its_form_and_the_rest_print_at_warn(self):
        found = sites(tracked_paths())
        lines, failures = verdict(found, {path for path, *_ in LISTED}, LEVEL)
        for line in lines:
            print(line)
        self.assertEqual(
            {path: found.get(path) for path, *_ in LISTED},
            {path: (tool, form) for path, tool, form, _why in LISTED},
        )
        self.assertEqual(failures, [])
        # The listing may be empty once every site has gone, so its count is printed, not refused.
        print(f"listed {len(LISTED)} site(s)")


# The planted sites of sites.fixture, each with the tool and the form that make it one (R2 to R5).
PLANTED_SITES = {
    "web/app/node_modules/vite/package.json": ("npm or pnpm", "node_modules/"),
    "scripts/tests/__pycache__/_support.pyc": ("CPython", "__pycache__/"),
    ".pytest_cache/v/cache/lastfailed": ("pytest", ".pytest_cache/"),
    ".ruff_cache/content/entry": ("ruff", ".ruff_cache/"),
    "htmlcov/index.html": ("coverage.py", "htmlcov/"),
    "web/app/.svelte-kit/generated/root.js": ("SvelteKit", ".svelte-kit/"),
    "web/app/.vite/deps/_metadata.json": ("Vite", ".vite/"),
    "ios/CardIsolation/.build/debug.yaml": ("SwiftPM", ".build/"),
    "ios/HarnessWire/.swiftpm/configuration/Package.resolved": ("SwiftPM", ".swiftpm/"),
    "ios/DerivedData/Harness/info.plist": ("Xcode", "DerivedData/"),
    "ios/Harness.xcworkspace/xcuserdata/person.xcuserdatad/state.xcuserstate": (
        "Xcode",
        "xcuserdata/",
    ),
    "formal/lean/.lake/build/lib/Formal.olean": ("Lake", ".lake/"),
    "mutants.out/outcomes.json": ("cargo-mutants", "mutants.out/"),
    "mutants.out.old/outcomes.json": ("cargo-mutants", "mutants.out.old/"),
    "web/app/.stryker-tmp/sandbox/index.js": ("StrykerJS", ".stryker-tmp/"),
    "web/app/test-results/results.json": ("Playwright", "test-results/"),
    "web/app/playwright-report/index.html": ("Playwright", "playwright-report/"),
    "web/app/blob-report/report.zip": ("Playwright", "blob-report/"),
    ".check-logs/python.log": ("the gate", ".check-logs/"),
    "ios/Harness.xcresult/Info.plist": ("xcodebuild", "*.xcresult/"),
    "scripts/stage.pyc": ("CPython", "*.pyc"),
    "scripts/legacy.pyo": ("CPython", "*.pyo"),
    ".coverage": ("coverage.py", ".coverage"),
    "tools/coverage/.coverage.runner.1.2": ("coverage.py", ".coverage"),
    "web/app/vite.config.ts.timestamp-1-2.mjs": ("Vite", "vite.config.*.timestamp-*"),
    "target/debug/deck-streak": ("cargo", "target/"),
    "web/app/build/index.html": ("SvelteKit or Vite", "build/"),
    "vendor/pkg/dist/index.js": ("Vite", "dist/"),
    "coverage/lcov.info": ("Vitest", "coverage/"),
    "web/app/reports/mutation/mutation.json": ("StrykerJS", "reports/"),
    "web/app/src/lib/paraglide/messages.js": ("Paraglide", "web/app/src/lib/paraglide/"),
    "web/app/project.inlang/cache/plugins/entry.js": ("inlang", "web/app/project.inlang/cache/"),
    "ios/Harness.xcodeproj/project.pbxproj": ("XcodeGen", "ios/Harness.xcodeproj/"),
    "ios/EnginePackage/DeckStreakFFI.xcframework/Info.plist": (
        "xcodebuild",
        "ios/EnginePackage/DeckStreakFFI.xcframework/",
    ),
    "ios/EnginePackage/Sources/DeckStreakFFI/deck_streak_ffi.swift": (
        "UniFFI",
        "ios/EnginePackage/Sources/DeckStreakFFI/",
    ),
}


if __name__ == "__main__":
    unittest.main()

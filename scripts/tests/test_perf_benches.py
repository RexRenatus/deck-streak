"""SPEC-618 A20 to A24: the bench profile, the bench targets and files, the benchmark list, the
bench config and the bench exclusions, read from the tree. No workflow loader, no cargo, no pnpm."""

import ast
import json
import re
import tomllib
import unittest

from _support import REPO, examined

# The bench packages and the bench files each defines, one `[[bench]]` target per file (R9).
BENCH_TARGETS = {
    "crates/engine-core": ["scheduling"],
    "crates/fsrs7": ["replay"],
    "crates/ffi": ["grading", "render"],
}
WEB_BENCH_ROOT = REPO / "web" / "app" / "bench"
BENCH_LIST = REPO / "perf" / "benchmarks.txt"
BENCH_CONFIG = REPO / "vitest.bench.config.ts"
# The one web path outside web/app/src that Stryker mutates, by its exact name (SPEC-636 R1).
STRYKER_OUTSIDE_SRC = ("scripts/build-tokens.ts",)
RUST_BENCH = re.compile(r"#\[divan::bench(?:\([^)]*\))?\]\s*(?:pub\s+)?fn\s+(\w+)")
MEASURED = re.compile(r"#\[inline\(never\)\]\s*(?:pub\s+)?fn\s+(\w+)_measured\b")
WEB_BENCH = re.compile(r"\bbench\(\s*(['\"])(.+?)\1")


def toml_of(path):
    return tomllib.loads((REPO / path).read_text(encoding="utf-8"))


def rust_bench_files():
    return [
        REPO / crate / "benches" / f"{name}.rs"
        for crate, names in BENCH_TARGETS.items()
        for name in names
    ]


def web_bench_files():
    return sorted(WEB_BENCH_ROOT.rglob("*.bench.ts"))


def web_stem(path):
    """A web bench file's stem: its name without `.bench.ts` (`frame-host.bench.ts` is
    `frame-host`), as a Rust bench file's stem is its name without `.rs` (R12)."""
    return path.name[: -len(".bench.ts")]


def derived_names():
    """Every bench name the bench files define, `<file stem>::<bench>` (R12)."""
    names = []
    for path in rust_bench_files():
        if path.is_file():
            text = path.read_text(encoding="utf-8")
            names.extend(f"{path.stem}::{name}" for name in RUST_BENCH.findall(text))
    for path in web_bench_files():
        text = path.read_text(encoding="utf-8")
        names.extend(f"{web_stem(path)}::{name}" for _, name in WEB_BENCH.findall(text))
    return names


class TheBenchesAreInTheTree(unittest.TestCase):
    def test_the_bench_profile_inherits_release_with_line_tables(self):
        profile = toml_of("Cargo.toml").get("profile", {})
        self.assertEqual(
            profile.get("bench"),
            {"inherits": "release", "debug": "line-tables-only"},
            "the root manifest has no [profile.bench] that inherits release with line tables",
        )
        self.assertNotIn("release", profile, "the release profile is changed")

    def test_each_rust_bench_is_a_harnessless_target_with_a_measured_body(self):
        workspace = toml_of("Cargo.toml").get("workspace", {}).get("dependencies", {})
        self.assertEqual(
            workspace.get("divan"),
            {"package": "codspeed-divan-compat", "version": "=6.0.0"},
            "the workspace has no exact entry for the renamed Divan crate",
        )
        for crate, names in examined("bench packages", list(BENCH_TARGETS.items())):
            with self.subTest(crate=crate):
                manifest = toml_of(f"{crate}/Cargo.toml")
                self.assertEqual(
                    manifest.get("dev-dependencies", {}).get("divan"),
                    {"workspace": True},
                    f"{crate} takes no divan dev-dependency from the workspace",
                )
                self.assertEqual(
                    manifest.get("bench", []),
                    [{"name": name, "harness": False} for name in names],
                    f"{crate} has no harness-free [[bench]] target per bench file",
                )
        for path in examined("rust bench files", rust_bench_files()):
            with self.subTest(bench=path.name):
                self.assertTrue(path.is_file(), f"{path.relative_to(REPO)} is absent")
                text = path.read_text(encoding="utf-8")
                benches = RUST_BENCH.findall(text)
                self.assertTrue(benches, f"{path.name} defines no #[divan::bench] function")
                self.assertIn("divan::main()", text, f"{path.name} never runs divan::main()")
                measured = set(MEASURED.findall(text))
                for name in benches:
                    self.assertIn(
                        name, measured, f"{path.name}: no #[inline(never)] {name}_measured"
                    )

    def test_the_benchmark_list_equals_the_names_the_bench_files_define(self):
        self.assertTrue(BENCH_LIST.is_file(), "perf/benchmarks.txt is absent")
        listed = [
            line.strip()
            for line in BENCH_LIST.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        names = examined("bench names", derived_names())
        self.assertEqual(len(listed), len(set(listed)), "perf/benchmarks.txt repeats a name")
        self.assertEqual(sorted(listed), sorted(names))

    def test_the_web_benches_run_under_the_codspeed_plugin(self):
        self.assertTrue(BENCH_CONFIG.is_file(), "vitest.bench.config.ts is absent")
        config = BENCH_CONFIG.read_text(encoding="utf-8")
        for needle in ("@codspeed/vitest-plugin", "codspeedPlugin()", "jsdom", "*.bench.ts"):
            self.assertIn(needle, config, f"vitest.bench.config.ts lacks {needle}")
        plugin = json.loads((REPO / "package.json").read_text(encoding="utf-8"))
        version = plugin.get("devDependencies", {}).get("@codspeed/vitest-plugin", "")
        self.assertRegex(version, r"^\d+\.\d+\.\d+$", "the plugin is not at one exact version")
        files = web_bench_files()
        self.assertEqual(len(files), 4, "the four web bench files of R9 are not all present")
        for path in examined("web bench files", files):
            with self.subTest(bench=path.name):
                text = path.read_text(encoding="utf-8")
                self.assertTrue(WEB_BENCH.findall(text), f"{path.name} names no bench")
                self.assertIn(").run()", text, f"{path.name} never runs its bench")

    def test_bench_files_are_excluded_from_mutation_and_coverage(self):
        stryker = json.loads(
            (REPO / "web" / "app" / "stryker.config.json").read_text(encoding="utf-8")
        )
        # Stryker reaches web/app/src, SPEC-039 R2's root, and one exact path beside it,
        # scripts/build-tokens.ts (SPEC-636 R1); no bench file lies in either.
        mutate = [glob for glob in stryker.get("mutate", []) if not glob.startswith("!")]
        for glob in examined("stryker mutate globs", mutate):
            self.assertTrue(
                glob.startswith("src/") or glob in STRYKER_OUTSIDE_SRC,
                f"Stryker mutates {glob}, outside src",
            )
        outside = sorted(glob for glob in mutate if not glob.startswith("src/"))
        self.assertEqual(outside, sorted(STRYKER_OUTSIDE_SRC), "Stryker mutates another path")
        for path in examined("stryker paths outside src", outside):
            self.assertFalse(
                (REPO / "web" / "app" / path).match("*.bench.*"),
                f"Stryker mutates the bench file {path}",
            )
        self.assertEqual(sorted((REPO / "web" / "app" / "src").rglob("*.bench.*")), [])
        register = json.loads((REPO / "config" / "coverage.json").read_text(encoding="utf-8"))
        exclusions = {entry.get("glob"): entry for entry in register.get("exclusions", [])}
        wanted = {"crates/*/benches/**": "rust", "web/app/bench/**": "web"}
        for glob, lane in examined("bench exclusions", list(wanted.items())):
            with self.subTest(glob=glob):
                entry = exclusions.get(glob)
                self.assertIsNotNone(entry, f"config/coverage.json does not exclude {glob}")
                self.assertEqual(
                    (entry["lane"], entry["kind"], entry.get("cite")),
                    (lane, "bench", "SPEC-618"),
                )
                self.assertTrue(str(entry.get("reason", "")).strip(), f"{glob} has no reason")
        # The web bench files the source-root check above stands for, outside that root.
        examined("web bench files outside the source root", web_bench_files())


# A31: each web bench's own bound. Its two measured CI times in ms, each keyed by its job id.
WEB_BENCH_TIMES = {
    "frame-document::frameDocument": {
        127064902478: 49068,
        127198761633: 50255,
    },
    "frame-host::frameCard": {
        127064902478: 60195,
        127198761633: 62995,
    },
    "review-screen::ReviewScreen": {
        127064902478: 23082,
        127198761633: 22721,
    },
    "review::step": {
        127064902478: 1709,
        127198761633: 1710,
    },
}
BOUND_FLOOR_MS = 60000  # the bound frameCard ran past at 62995 ms (job 127198761633)
BOUND_STEP_MS = 10000  # frameCard's bound rounds up to 100000 ms by it (job 127198761633)
TEST_CALL = re.compile(r"^test\(\s*(['\"]).+?\1\s*,(?P<rest>.*)$", re.M)
TIMEOUT_OPTION = re.compile(r"\s*\{\s*timeout:\s*(\w+)\s*\}\s*,")
MODULE_CONSTANT = re.compile(r"^const\s+(\w+)\s*=\s*(\d[\d_]*)\s*;", re.M)


def required_bound(times):
    """The larger of the floor and 1.5 times the worst measured time, rounded up to the step."""
    worst = max(times.values())
    return max(BOUND_FLOOR_MS, -(-worst * 3 // (2 * BOUND_STEP_MS)) * BOUND_STEP_MS)


class EachWebBenchPassesItsOwnBound(unittest.TestCase):
    def test_each_web_bench_passes_its_own_measured_bound(self):
        files = {web_stem(path): path for path in web_bench_files()}
        names = [name for name in derived_names() if name.split("::")[0] in files]
        for name in sorted(set(WEB_BENCH_TIMES) - set(names)):
            with self.subTest(bench=name):
                self.fail(f"{name} has a table row and no bench")
        for name in examined("web benches", names):
            with self.subTest(bench=name):
                self.assertIn(name, WEB_BENCH_TIMES, f"{name} has no table row")
                stem, bench = name.split("::", 1)
                path = files[stem]
                text = path.read_text(encoding="utf-8")
                at = next(m.start() for m in WEB_BENCH.finditer(text) if m.group(2) == bench)
                calls = [m for m in TEST_CALL.finditer(text) if m.start() < at]
                self.assertTrue(calls, f"{name}: no test call holds the bench")
                option = TIMEOUT_OPTION.match(calls[-1].group("rest"))
                self.assertIsNotNone(option, f"{name}: its test call passes no bound")
                constants = dict(MODULE_CONSTANT.findall(text))
                self.assertIn(
                    option.group(1),
                    constants,
                    f"{name}: its bound is no module constant of {path.name}",
                )
                bound = int(constants[option.group(1)].replace("_", ""))
                wanted = required_bound(WEB_BENCH_TIMES[name])
                self.assertGreaterEqual(
                    bound, wanted, f"{name}: its bound {bound} is under {wanted}"
                )


# SPEC-637: a script run of this module reaches every class only when its main guard is its last
# statement, after every test class (ADR-653).
THIS_MODULE = REPO / "scripts" / "tests" / "test_perf_benches.py"


def is_main_guard(node):
    """Whether a top-level statement is the `if __name__ == "__main__":` guard."""
    return (
        isinstance(node, ast.If)
        and isinstance(node.test, ast.Compare)
        and isinstance(node.test.left, ast.Name)
        and node.test.left.id == "__name__"
        and len(node.test.ops) == 1
        and isinstance(node.test.ops[0], ast.Eq)
        and len(node.test.comparators) == 1
        and isinstance(node.test.comparators[0], ast.Constant)
        and node.test.comparators[0].value == "__main__"
    )


class TheScriptRunReachesEveryClass(unittest.TestCase):
    def test_the_main_guard_is_the_last_statement_after_every_class(self):
        body = ast.parse(THIS_MODULE.read_text(encoding="utf-8")).body
        guards = examined(
            "main guards", [at for at, node in enumerate(body) if is_main_guard(node)]
        )
        classes = examined(
            "test classes",
            [(at, node.name) for at, node in enumerate(body) if isinstance(node, ast.ClassDef)],
        )
        after = [name for at, name in classes if at > guards[0]]
        self.assertEqual(after, [], "a class follows the main guard, so a script run skips it")
        self.assertEqual(guards, [len(body) - 1], "the main guard is not the last statement")


if __name__ == "__main__":
    unittest.main()

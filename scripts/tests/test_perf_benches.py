"""SPEC-618 A20 to A24: the bench profile, the bench targets and files, the benchmark list, the
bench config and the bench exclusions, read from the tree. No workflow loader, no cargo, no pnpm."""

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
WEB_BENCH_ROOT = REPO / "web" / "app" / "src" / "lib"
BENCH_LIST = REPO / "perf" / "benchmarks.txt"
BENCH_CONFIG = REPO / "vitest.bench.config.ts"
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
        self.assertIn("!src/**/*.bench.*", stryker.get("mutate", []))
        register = json.loads((REPO / "config" / "coverage.json").read_text(encoding="utf-8"))
        exclusions = {entry.get("glob"): entry for entry in register.get("exclusions", [])}
        wanted = {"crates/*/benches/**": "rust", "web/app/src/**/*.bench.ts": "web"}
        for glob, lane in examined("bench exclusions", list(wanted.items())):
            with self.subTest(glob=glob):
                entry = exclusions.get(glob)
                self.assertIsNotNone(entry, f"config/coverage.json does not exclude {glob}")
                self.assertEqual(
                    (entry["lane"], entry["kind"], entry.get("cite")),
                    (lane, "bench", "SPEC-618"),
                )
                self.assertTrue(str(entry.get("reason", "")).strip(), f"{glob} has no reason")


if __name__ == "__main__":
    unittest.main()

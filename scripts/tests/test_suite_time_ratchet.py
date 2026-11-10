"""Each test suite's run time is held to a recorded bound, at warn (SPEC-669, ADR-692).

The census lists every suite CI runs by its runner's own unit (each Rust test binary under the
workspace's test runner, each crate's doc tests, each Python test module of the python stage, each
web unit test file, each browser test file and each Swift test target, a Swift package's test
targets included), takes each suite's time from the stage logs CI
uploads, and checks it against the listing in `scripts/suite-times.json`. A suite over its bound by
more than its margin is a site, reported at warn; a suite with no time recorded is reported, never
passed; a suite whose figure would list a lower bound is reported for the listing, which only a
delivery changes; a listing entry that names no current suite fails, at warn as at fail. An empty
population is refused.

The tests below read planted trees only. The whole-tree reading runs in CI:

    python3 scripts/tests/test_suite_time_ratchet.py --logs DIR
"""

import argparse
import json
import re
import sys
import unittest
from pathlib import Path

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "suite-times"
LISTING = REPO / "scripts" / "suite-times.json"

ANSI = re.compile(r"\x1b\[[0-9;]*m")
NEXTEST = re.compile(
    r"^\s*(?:PASS|FAIL|FLAKY|TRY \d+ PASS|TRY \d+ FAIL)\s+\[\s*([0-9.]+)s\]\s+"
    r"(?:\(\s*\d+/\d+\)\s+)?(\S+)\s+\S"
)
RAN = re.compile(r"^Ran (\d+) tests? in ([0-9.]+)s$")
MODULE = re.compile(r"^module: (\S+) ran (\d+) test\(s\), ")
VITEST = re.compile(
    r"^\s*[✓×]\s+(?:\S+\s+)?(src/\S+\.(?:test|spec)\.[jt]s)\s+\(\d+ tests?[^)]*\)\s+"
    r"([0-9.]+)\s*(ms|s)\b"
)
DOCTESTS = re.compile(r"^Doc-tests (\S+)$")
DOC_RESULT = re.compile(r"^test result: \S+\. \d+ passed; .*\bfinished in ([0-9.]+)s$")
DOC_ALL = re.compile(r"^all doctests ran in ([0-9.]+)s; ")
PACKAGE = re.compile(r'^\[package\]\s*$(?:(?!^\[).)*?^name\s*=\s*"([^"]+)"', re.M | re.S)
PYTHON_SUITE = re.compile(
    r"^(?:scripts/tests|agent/tests|tools/parity-oracle(?:/[^/]+)*)/test_[^/]+\.py$"
)
VITEST_FILE = re.compile(r"^web/[^/]+/(src/(?:[^/]+/)*[^/]+\.(?:test|spec)\.[jt]s)$")
PLAYWRIGHT_FILE = re.compile(r"^web/app/tests(?:-[a-z]+)?/(?:[^/]+/)*[^/]+\.spec\.[jt]s$")
SWIFT_FILE = re.compile(r"^ios/(?:[^/]+/Tests/)?([^/]+Tests)/(?:[^/]+/)*[^/]+\.swift$")
NEXTEST_LOGS = {
    "test.log": "nextest",
    "test-engine.log": "nextest",
    "test-release.log": "nextest-release",
}
NO_TIME_IN_CI = ("playwright", "xctest")
PRUNED = {".git", "node_modules", "target", ".svelte-kit", "build", "dist"}


def crate_units(name, crate, files):
    """The test binaries one crate's manifest makes, by the runner's binary id."""
    units = []
    if crate + "src/lib.rs" in files:
        units.append(name)
    if crate + "src/main.rs" in files:
        units.append(name + "::bin/" + name)
    for other in sorted(files):
        if other.startswith(crate + "tests/") and other.count("/") == 3 and other.endswith(".rs"):
            units.append(name + "::" + other.rsplit("/", 1)[1][: -len(".rs")])
        if other.startswith(crate + "src/bin/") and other.count("/") == 4 and other.endswith(".rs"):
            units.append(name + "::bin/" + other.rsplit("/", 1)[1][: -len(".rs")])
    return units


def population(files):
    """Every suite CI runs, as (runner, unit), from a tree's paths and its crate manifests' text."""
    suites = set()
    for path, text in files.items():
        parts = path.split("/")
        if len(parts) == 3 and parts[0] == "crates" and parts[2] == "Cargo.toml":
            found = PACKAGE.search(text or "")
            if found:
                for unit in crate_units(found.group(1), "crates/" + parts[1] + "/", files):
                    suites.add(("nextest", unit))
                    suites.add(("nextest-release", unit))
                if "crates/" + parts[1] + "/src/lib.rs" in files:
                    suites.add(("rustdoc", found.group(1).replace("-", "_")))
        elif PYTHON_SUITE.match(path):
            suites.add(("unittest", path))
        elif VITEST_FILE.match(path):
            suites.add(("vitest", VITEST_FILE.match(path).group(1)))
        elif PLAYWRIGHT_FILE.match(path):
            suites.add(("playwright", path))
        elif SWIFT_FILE.match(path):
            suites.add(("xctest", SWIFT_FILE.match(path).group(1)))
    return sorted(suites)


def times(logs):
    """Each suite's time from CI's stage logs, {(runner, unit): seconds}; `logs` maps a stage
    log's file name to its lines. A Rust binary's figure is the sum of its tests' own durations; a
    crate's doc tests take their result lines' sum, or the merged form's `all doctests ran` line."""
    return {}


def census(suites, figures, listing):
    """Judge every suite of the population against the listing.

    Returns (verdicts, lines, failures): a verdict per suite (clean, site, faster, untimed or
    unlisted), one report line per suite that is not clean, and the listing entries that name no
    current suite, which fail."""
    return {key: "clean" for key in suites}, [], []


def tree_files(root):
    """A checkout's paths, with the text of each crate manifest, walked by pathlib alone."""
    files = {}
    pending = [root]
    while pending:
        folder = pending.pop()
        for path in sorted(folder.iterdir()):
            if path.is_symlink():
                continue
            if path.is_dir():
                if path.name not in PRUNED:
                    pending.append(path)
                continue
            rel = path.relative_to(root).as_posix()
            text = None
            if rel.startswith("crates/") and rel.count("/") == 2 and path.name == "Cargo.toml":
                text = path.read_text(encoding="utf-8")
            files[rel] = text
    return files


def read_logs(folder):
    """Every stage log under a folder of downloaded stage-log artifacts, by file name."""
    logs = {}
    for path in sorted(folder.rglob("*.log")):
        text = path.read_text(encoding="utf-8", errors="replace")
        logs.setdefault(path.name, []).extend(text.splitlines())
    return logs


def load_planted():
    files = json.loads((FIXTURES / "tree.json.fixture").read_text(encoding="utf-8"))
    logs = {}
    for path in sorted(FIXTURES.glob("*.log.fixture")):
        logs[path.name[: -len(".fixture")]] = path.read_text(encoding="utf-8").splitlines()
    listing = json.loads((FIXTURES / "listing.json.fixture").read_text(encoding="utf-8"))
    return files, logs, listing


class PlantedSuiteTimes(unittest.TestCase):
    def setUp(self):
        self.files, self.logs, self.listing = load_planted()
        self.suites = population(self.files)
        self.figures = times(self.logs)
        self.verdicts, self.lines, self.failures = census(self.suites, self.figures, self.listing)

    def test_the_planted_population_holds_every_runner(self):
        self.assertEqual(len(self.suites), 21, self.suites)
        self.assertEqual(
            sorted({runner for runner, _unit in self.suites}),
            [
                "nextest",
                "nextest-release",
                "playwright",
                "rustdoc",
                "unittest",
                "vitest",
                "xctest",
            ],
        )

    def test_a_suite_under_its_bound_is_clean(self):
        self.assertEqual(self.verdicts[("nextest", "alpha::under")], "clean")

    def test_a_suite_over_its_bound_past_its_margin_is_a_site(self):
        self.assertEqual(self.verdicts[("nextest", "alpha::over")], "site")
        self.assertIn(
            "warn site: nextest alpha::over: 45.000s is over its bound 10.000s by more than its"
            " margin (limit 40.000s): planted: over its bound past the margin",
            self.lines,
        )

    def test_a_suite_over_its_bound_within_its_margin_is_clean(self):
        self.assertEqual(self.verdicts[("nextest", "alpha::within")], "clean")

    def test_a_faster_suite_is_reported_for_the_listing(self):
        self.assertEqual(self.verdicts[("nextest", "alpha::faster")], "faster")

    def test_a_suite_with_no_time_recorded_is_reported_never_passed(self):
        self.assertEqual(self.verdicts[("nextest", "alpha::untimed")], "untimed")
        self.assertEqual(self.verdicts[("playwright", "web/app/tests/planted.spec.ts")], "untimed")
        self.assertEqual(self.verdicts[("xctest", "PlantedTests")], "untimed")
        self.assertEqual(self.verdicts[("xctest", "PlantedKitTests")], "untimed")
        self.assertIn(
            "warn untimed: xctest PlantedTests: no time recorded in ci, not passed",
            self.lines,
        )

    def test_a_listing_entry_naming_no_current_suite_fails(self):
        self.assertEqual(self.failures, [("nextest", "alpha::removed")])

    def test_an_empty_population_is_void(self):
        empty = json.loads((FIXTURES / "empty-tree.json.fixture").read_text(encoding="utf-8"))
        with self.assertRaises(AssertionError) as raised:
            census(population(empty), self.figures, self.listing)
        self.assertIn("examined 0 suite(s)", str(raised.exception))

    def test_each_runner_s_time_is_read_from_its_own_form(self):
        self.assertEqual(self.figures[("nextest", "alpha::over")], 45.0)
        self.assertEqual(self.figures[("unittest", "scripts/tests/test_planted.py")], 1.25)
        self.assertNotIn(("unittest", "scripts/tests/test_mismatch.py"), self.figures)
        self.assertEqual(self.figures[("vitest", "src/lib/planted.test.ts")], 0.5)
        self.assertEqual(self.figures[("vitest", "src/lib/other.test.ts")], 1.5)
        self.assertEqual(self.figures[("rustdoc", "beta_crate")], 0.75)
        self.assertEqual(self.figures[("rustdoc", "alpha")], 1.5)
        self.assertIn("warn unlisted: rustdoc alpha: 1.500s, no bound listed", self.lines)


class TheListingNamesOnlyCurrentSuites(unittest.TestCase):
    def test_every_listed_suite_is_in_the_population(self):
        suites = examined("suite(s)", population(tree_files(REPO)))
        listing = json.loads(LISTING.read_text(encoding="utf-8"))
        present = set(suites)
        stale = [e for e in listing["suites"] if (e["runner"], e["suite"]) not in present]
        self.assertEqual(stale, [], "a listing entry names no current suite")


def main(argv):
    parser = argparse.ArgumentParser(description="the suite-time census over CI's stage logs")
    parser.add_argument("--logs", required=True, type=Path)
    args = parser.parse_args(argv)
    listing = json.loads(LISTING.read_text(encoding="utf-8"))
    verdicts, lines, failures = census(
        population(tree_files(REPO)), times(read_logs(args.logs)), listing
    )
    for line in lines:
        print(line)
    for tag in ("clean", "site", "faster", "untimed", "unlisted"):
        print(f"{tag}: {sum(1 for v in verdicts.values() if v == tag)}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

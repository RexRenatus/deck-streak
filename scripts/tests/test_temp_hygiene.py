"""Tests leave no temporary file or directory behind (SPEC-030 R2 to R4).

The predecessor pinned pytest's `tmp_path_retention_policy = "failed"`, so green runs left nothing on
the small shared host. DeckStreak's Python tests are `unittest`, so this lint holds the line
instead: it reads every test file of the repository (the Rust integration tests, the repository's
Python tests and the Mini App's tests), and refuses, naming the file and the line, each form that
keeps what it creates after the test ends. The accepted forms remove what they create on drop or
exit: `tempfile::TempDir` and `NamedTempFile` dropped at scope end, Python's
`tempfile.TemporaryDirectory`, and a `mkdtemp` whose removal the same function registers.

The lint reads text, not paths: a directory built from a variable escapes it, so it refuses the
leaking APIs themselves rather than their paths (SPEC-030 section 6).
"""

import unittest
from pathlib import Path

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "temp-hygiene"
# R2: every test file of the repository. The planted fixtures sit under FIXTURES with a `.fixture`
# suffix, so no pattern here matches them, and the census drops that directory as well (R4).
CENSUS = (
    "crates/*/tests/**/*.rs",
    "scripts/tests/*.py",
    "tools/parity-oracle/test_*.py",
    "web/app/src/**/*.test.ts",
    "web/app/tests/**/*.ts",
)
LANGUAGES = {".rs": "rust", ".py": "python", ".ts": "typescript"}


def census(root=REPO):
    """Every test file R2 names, without the planted fixtures (R4)."""
    found = {
        path for pattern in CENSUS for path in root.glob(pattern) if path.is_file()
    }
    return sorted(path for path in found if FIXTURES not in path.parents)


def language_of(path):
    """The file's language; a `.fixture` is read by the extension before its suffix (R4)."""
    name = path.name.removesuffix(".fixture")
    return LANGUAGES[Path(name).suffix]


def leaks(path, root=REPO):
    """Each leaking form in one test file, as `<path>:<line>: <what>`. A stub until the lint's
    rules land: it refuses nothing, so every planted leak goes unrefused."""
    return []


def planted_lines(path, marker):
    """The lines of a fixture that end with a `planted:` or `accepted:` comment."""
    lines = path.read_text(encoding="utf-8").splitlines()
    return [number for number, line in enumerate(lines, 1) if f" {marker}: " in line]


class TestsLeaveNoTemporaryFiles(unittest.TestCase):
    def test_every_test_file_removes_what_it_creates(self):
        files = examined("test file(s)", census())
        self.assertEqual(
            sorted({language_of(path) for path in files}),
            ["python", "rust", "typescript"],
        )
        planted = sorted(FIXTURES.glob("*.fixture"))
        self.assertEqual(len(planted), 3, "the three planted fixtures are present")
        for path in planted:
            self.assertNotIn(
                path, files, f"{path.name} is planted, never part of the census"
            )
        found = [leak for path in files for leak in leaks(path)]
        self.assertEqual(found, [], "\n".join(found))

    def assert_planted_leaks_are_refused(self, name):
        path = FIXTURES / name
        refused = planted_lines(path, "planted")
        accepted = planted_lines(path, "accepted")
        self.assertGreater(len(refused), 0, f"{name} plants no leak")
        self.assertGreater(len(accepted), 0, f"{name} plants no accepted form")
        found = leaks(path)
        lines = sorted({int(leak.split(":")[1]) for leak in found})
        self.assertEqual(lines, refused, "\n".join(found))
        for leak in found:
            self.assertTrue(
                leak.startswith(f"scripts/tests/fixtures/temp-hygiene/{name}:"), leak
            )

    def test_a_planted_rust_leak_is_refused(self):
        self.assert_planted_leaks_are_refused("leaks_tempdir.rs.fixture")

    def test_a_planted_python_leak_is_refused(self):
        self.assert_planted_leaks_are_refused("leaks_mkdtemp.py.fixture")

    def test_a_planted_typescript_leak_is_refused(self):
        self.assert_planted_leaks_are_refused("leaks_mkdtemp.ts.fixture")


if __name__ == "__main__":
    unittest.main()

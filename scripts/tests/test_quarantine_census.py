"""A flaky test is quarantined with the runs that disagreed, and keeps running (SPEC-677 R1 to R9).

The census reads every test file and test runner configuration of the repository as text, finds
each marker that keeps a test from running or from failing its run (a Rust `#[ignore]`, a Python
skip or expected failure, a Vitest or Playwright skip, fixme, todo, fail or only, a Swift skip or
expected failure, and a retry setting), and judges each against the quarantine listing,
`scripts/tests/quarantine.json`. A quarantine entry names its test and the runs on one head that
disagreed; an entry never turns a test off. A marker whose own condition names a platform, and a
Rust child run whose reason names the test of its file that runs it, are not sites (R5). Every
other marker is a site: listed with its reason, or reported at the listing's level (R6).
"""

import json
import os
import re
import unittest
from pathlib import Path

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "quarantine"
LISTING = REPO / "scripts" / "tests" / "quarantine.json"
# R1: the population, one rule per stack over each file's path relative to the checkout,
# stated once in `member`. A name ending in `.fixture` is a planted text no runner collects:
# the tree's census never reads one, and a planted census reads only those, each judged by
# its name without the suffix. A directory named in PRUNE holds no file of the tree, only
# what git, cargo, pnpm or a virtual environment writes.
PRUNE = {".git", ".venv", "node_modules", "target", "venv"}
WEB_CODE = (".ts", ".js", ".mjs", ".cjs")
WEB_CONFIG = re.compile(r"(?:vitest|vite|playwright)[^/]*\.config\.(?:ts|js|mjs|cjs)")
PY_TEST_NAME = re.compile(r"test_.*\.py|.*_test\.py")
RUST_TEST = re.compile(r"^\s*#\[(?:cfg\(test\)|(?:tokio::)?test\b)", re.M)
# R2: each marker form, by stack. A form reads a line's code, never a comment or a quoted name.
FORMS = {
    "rust": (
        (
            "ignore",
            re.compile(r"^\s*#\[ignore\b\s*(?:=\s*\"(?P<reason>[^\"]*)\")?\s*\]"),
        ),
        ("cfg_attr-ignore", re.compile(r"^\s*#\[cfg_attr\((?P<cond>.+?),\s*ignore\b")),
    ),
    "python": (
        ("skip", re.compile(r"^\s*@(?:unittest\.)?skip\(\s*(?P<reason>['\"].*)?")),
        ("skipIf", re.compile(r"^\s*@(?:unittest\.)?skipIf\((?P<cond>.*)")),
        ("skipUnless", re.compile(r"^\s*@(?:unittest\.)?skipUnless\((?P<cond>.*)")),
        ("expectedFailure", re.compile(r"^\s*@(?:unittest\.)?expectedFailure\b")),
        ("skipTest", re.compile(r"^[^#'\"]*\bself\.skipTest\(\s*(?P<reason>['\"].*)?")),
        (
            "SkipTest",
            re.compile(r"^[^#'\"]*\braise\s+(?:unittest\.)?SkipTest\b(?P<reason>.*)"),
        ),
    ),
    "web": (
        (
            "skip",
            re.compile(
                r"^[^'\"`/]*\b(?:it|test|describe|suite)(?:\.\w+)*\.(?:skip(?:If)?|runIf)\((?P<cond>.*)"
            ),
        ),
        (
            "fixme",
            re.compile(r"^[^'\"`/]*\b(?:it|test|describe)(?:\.\w+)*\.fixme\((?P<cond>.*)"),
        ),
        ("todo", re.compile(r"^[^'\"`/]*\b(?:it|test|describe)\.todo\((?P<reason>.*)")),
        ("fail", re.compile(r"^[^'\"`/]*\b(?:it|test)\.fails?\((?P<cond>.*)")),
        ("only", re.compile(r"^[^'\"`/]*\b(?:it|test|describe)(?:\.\w+)*\.only\(")),
        ("retries", re.compile(r"^[^'\"`/]*\b(?:retries|retry)\s*:")),
    ),
    "swift": (
        ("XCTSkip", re.compile(r"^[^\"/]*\bXCTSkip(?:If|Unless)?\((?P<cond>.*)")),
        ("XCTExpectFailure", re.compile(r"^[^\"/]*\bXCTExpectFailure\(")),
    ),
    "nextest": (("retries", re.compile(r"^\s*retries\s*=")),),
}
PLATFORM = re.compile(
    r"\b(?:browserName|target_os|target_family|windows|unix|sys\.platform|os\.name|os\()"
)
QUOTED = re.compile(r"['\"`]([^'\"`]+)['\"`]")
HEAD = re.compile(r"^[0-9a-f]{40}$")
LEVELS = ("warn", "fail")


def stack_of(name):
    """The stack whose runner reads a file, by its name without a `.fixture` suffix."""
    name = name.removesuffix(".fixture")
    for ending, stack in (
        (".rs", "rust"),
        (".py", "python"),
        (".swift", "swift"),
        (".toml", "nextest"),
    ):
        if name.endswith(ending):
            return stack
    return "web" if name.endswith(WEB_CODE) else None


def member(rel, path):
    """R1: the stack of one file by its path relative to the checkout, or None.

    rust: a `.rs` file under a `tests` directory, or any other `.rs` file holding a test;
    python: a `.py` file under a `tests` directory, or one named `test_*.py` or `*_test.py`;
    web: a `.ts`, `.js`, `.mjs` or `.cjs` file named `*.test.*` or `*.spec.*`, under a
    `tests` or `tests-*` directory, or a Vitest, Vite or Playwright configuration;
    swift: a `.swift` file under a `*Tests` directory; nextest: `.config/nextest.toml`.
    A helper or fixture module under such a directory is in: default-deny keeps it.
    """
    *dirs, base = rel.split("/")
    stack = stack_of(base)
    if stack == "rust":
        held = "tests" in dirs or RUST_TEST.search(path.read_text(encoding="utf-8"))
    elif stack == "python":
        held = "tests" in dirs or PY_TEST_NAME.fullmatch(base)
    elif stack == "web":
        held = (
            re.search(r"\.(?:test|spec)\.", base)
            or any(d == "tests" or d.startswith("tests-") for d in dirs)
            or WEB_CONFIG.fullmatch(base)
        )
    elif stack == "swift":
        held = any(d.endswith("Tests") for d in dirs)
    else:
        held = rel == ".config/nextest.toml"
    return stack if held else None


def census(root=REPO, planted=False):
    """R1: every file under root that `member` holds, by its path relative to root."""
    found = []
    for folder, dirs, names in os.walk(root):
        dirs[:] = [d for d in dirs if d not in PRUNE]
        for base in names:
            if base.endswith(".fixture") != planted:
                continue
            path = Path(folder) / base
            rel = path.relative_to(root).as_posix().removesuffix(".fixture")
            if member(rel, path):
                found.append(path)
    return sorted(found)


def markers(name, text):
    """R2 and R5: each marker of one file as (line, stack, form, reason, admitted)."""
    stack = stack_of(name)
    found = []
    for number, line in enumerate(text.splitlines(), 1):
        for form, pattern in FORMS.get(stack, ()):
            match = pattern.search(line)
            if not match:
                continue
            groups = match.groupdict()
            cond = groups.get("cond") or ""
            reason = groups.get("reason") or ""
            quoted = QUOTED.findall(cond or reason)
            stated = (quoted[-1] if quoted else "") if cond or reason else ""
            admitted = bool(cond) and PLATFORM.search(cond.split(",")[0]) is not None
            if form == "ignore" and reason:
                runner = re.search(
                    r"\b([a-z][a-z0-9_]{2,}),? runs it\b|child run of ([a-z0-9_]+)",
                    reason,
                )
                named = runner and (runner.group(1) or runner.group(2))
                admitted = bool(named) and f"fn {named}(" in text and '"--ignored"' in text
            found.append((number, stack, form, stated, admitted))
            break
    return found


def judge(files, listing, root=REPO):
    """R3 to R7: (problems that fail at any level, unlisted sites, examined count)."""
    problems, unlisted = [], []
    level = listing.get("level")
    if level not in LEVELS:
        problems.append(f"level {level!r} is not one of {LEVELS}")
    texts = {}
    for path in files:
        try:
            name = path.relative_to(root).as_posix()
        except ValueError:
            name = path.name
        texts[name] = path.read_text(encoding="utf-8")
    for entry in listing.get("quarantined", []):
        test, record = entry.get("test", ""), entry.get("record") or []
        where, _, fn = test.partition("::")
        if where not in texts or not fn or not re.search(rf"\b{re.escape(fn)}\b", texts[where]):
            problems.append(f"quarantined {test}: its test is not in the census")
        runs = [r for r in record if isinstance(r, dict)]
        shaped = all(
            isinstance(r.get("run"), int)
            and isinstance(r.get("attempt"), int)
            and r.get("job")
            and r.get("conclusion") in ("success", "failure")
            and HEAD.match(str(r.get("head", "")))
            for r in runs
        )
        heads = {r.get("head") for r in runs}
        verdicts = {r.get("conclusion") for r in runs}
        if not shaped or len(heads) != 1 or verdicts != {"success", "failure"}:
            problems.append(f"quarantined {test}: no record of two runs on one head that disagreed")
    counted = {}
    for name, text in texts.items():
        for _line, stack, form, reason, admitted in markers(name, text):
            if not admitted:
                key = (name, stack, form)
                counted[key] = counted.get(key, 0) + 1
    listed = {}
    for site in listing.get("sites", []):
        key = (site.get("file"), site.get("stack"), site.get("form"))
        listed[key] = (site.get("count"), site.get("reason"))
        if not site.get("reason"):
            problems.append(f"site {key}: listed with no reason")
    for key in sorted(set(counted) | set(listed)):
        have, (want, _why) = counted.get(key, 0), listed.get(key, (0, None))
        if have > want:
            unlisted.append(f"{':'.join(key)}: {have} marker(s), {want} listed")
        elif have < want:
            problems.append(f"{':'.join(key)}: {have} marker(s), {want} listed (stale)")
    return problems, unlisted, len(texts)


def verdict(problems, unlisted, level):
    """R7: an unlisted site fails only at fail; a listing problem fails at either level."""
    for line in unlisted:
        print(f"WARN quarantine: {line}")
    return not problems and (level == "warn" or not unlisted)


def planted(name):
    path = FIXTURES / name
    return json.loads(path.read_text(encoding="utf-8"))


class FlakyTestsAreQuarantinedWithARecord(unittest.TestCase):
    def plant(self, listing_name):
        files = sorted(FIXTURES.glob("*.rs.fixture")) + sorted(FIXTURES.glob("*.ts.fixture"))
        files += sorted(FIXTURES.glob("*.py.fixture"))
        return judge(examined("planted file(s)", files), planted(listing_name), FIXTURES)

    def test_the_tree_holds_exactly_its_listed_sites(self):
        listing = json.loads(LISTING.read_text(encoding="utf-8"))
        files = examined("test file(s)", census())
        self.assertEqual(
            sorted({stack_of(path.name) for path in files}),
            ["nextest", "python", "rust", "swift", "web"],
        )
        problems, unlisted, _count = judge(files, listing)
        self.assertEqual(problems, [], "\n".join(problems))
        self.assertTrue(verdict(problems, unlisted, listing["level"]), "\n".join(unlisted))

    def test_a_listed_test_with_a_full_record_is_clean(self):
        problems, _unlisted, _count = self.plant("clean.json.fixture")
        self.assertEqual(problems, [])

    def test_an_entry_with_no_record_is_refused(self):
        problems, _unlisted, _count = self.plant("no_record.json.fixture")
        self.assertEqual(
            problems,
            [
                "quarantined flaky.py.fixture::test_sometimes: no record of two runs on one head that disagreed"
            ],
        )

    def test_an_entry_whose_test_is_gone_is_refused(self):
        problems, _unlisted, _count = self.plant("gone.json.fixture")
        self.assertEqual(
            problems,
            ["quarantined flaky.py.fixture::test_removed: its test is not in the census"],
        )

    def test_a_marker_with_no_entry_and_no_reason_is_a_site(self):
        problems, unlisted, _count = self.plant("clean.json.fixture")
        self.assertEqual(problems, [])
        self.assertEqual(
            unlisted,
            [
                "markers.py.fixture:python:expectedFailure: 1 marker(s), 0 listed",
                "markers.rs.fixture:rust:ignore: 2 marker(s), 0 listed",
            ],
        )

    def test_a_marker_whose_condition_names_a_platform_is_clean(self):
        text = (FIXTURES / "markers.ts.fixture").read_text(encoding="utf-8")
        found = markers("markers.ts.fixture", text)
        self.assertEqual(
            [(form, admitted) for _l, _s, form, _r, admitted in found],
            [("skip", True), ("skip", False), ("skip", True)],
        )
        rust = markers(
            "markers.rs.fixture",
            (FIXTURES / "markers.rs.fixture").read_text(encoding="utf-8"),
        )
        self.assertEqual(
            [(form, admitted) for _l, _s, form, _r, admitted in rust],
            [("ignore", True), ("ignore", False), ("ignore", False)],
        )

    def test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does(
        self,
    ):
        problems, unlisted, _count = self.plant("clean.json.fixture")
        self.assertTrue(verdict(problems, unlisted, "warn"))
        self.assertFalse(verdict(problems, unlisted, "fail"))
        stale, _unlisted, _count = self.plant("stale.json.fixture")
        self.assertEqual(stale, ["markers.ts.fixture:web:fixme: 0 marker(s), 1 listed (stale)"])
        self.assertFalse(verdict(stale, [], "warn"))

    def test_an_empty_population_is_void(self):
        with self.assertRaisesRegex(AssertionError, "examined 0"):
            examined("test file(s)", census(FIXTURES / "empty"))

    def test_a_vitest_file_under_web_app_src_is_read_and_its_skip_is_a_site(self):
        root = FIXTURES / "web-src"
        files = examined("planted file(s)", census(root, planted=True))
        self.assertEqual(
            [path.relative_to(root).as_posix() for path in files],
            ["web/app/src/lib/planted.test.ts.fixture"],
        )
        listing = {"level": "warn", "quarantined": [], "sites": []}
        problems, unlisted, _count = judge(files, listing, root)
        self.assertEqual(problems, [])
        self.assertEqual(
            unlisted,
            ["web/app/src/lib/planted.test.ts.fixture:web:skip: 1 marker(s), 0 listed"],
        )

    def test_a_root_whose_own_path_holds_src_is_still_read(self):
        root = FIXTURES / "src"
        self.assertIn("/src/", f"{root.as_posix()}/")
        files = examined("planted file(s)", census(root, planted=True))
        self.assertEqual(
            [path.relative_to(root).as_posix() for path in files],
            ["scripts/tests/test_planted.py.fixture"],
        )
        listing = {"level": "warn", "quarantined": [], "sites": []}
        problems, unlisted, _count = judge(files, listing, root)
        self.assertEqual(problems, [])
        self.assertEqual(
            unlisted,
            ["scripts/tests/test_planted.py.fixture:python:skip: 1 marker(s), 0 listed"],
        )


if __name__ == "__main__":
    unittest.main()

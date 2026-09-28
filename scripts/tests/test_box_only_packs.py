"""The public tree carries no vendored pack file, and every pack is judged on the box (SPEC-056 A1 to
A4, A16 and A18; ADR-069).

The vendored paths this census looks for are assembled at run time from their parts, so this file
names none of them and cannot be its own finding.
"""

import json
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

# The vendored tree and the scripts that ran or refreshed it, assembled from their parts.
PACKS_DIR = "." + "packs"
REMOVED_SCRIPTS = [
    "scripts/" + name
    for name in (
        "pack" + "-rows.py",
        "vendor" + "-packs.py",
        "methodology" + "_probe.py",
        "sdd" + "-probe.py",
        "ddd" + "-probe.py",
        "tdd" + "-probe.py",
        "no-api" + "keyhelper-scan.py",
    )
]
# What a file names when it reads a vendored path: the directory, a removed script, or the vendored
# manifest's pin.
VENDORED_READS = re.compile(
    "|".join(
        re.escape(token) + (r"\b" if token.endswith("_probe") else "")
        for token in (
            PACKS_DIR + "/",
            "pack" + "-rows",
            "vendor" + "-packs",
            "VENDORED" + ".json",
            "vendored" + "_from",
            "methodology" + "_probe",
            "sdd" + "-probe.py",
            "ddd" + "-probe.py",
            "tdd" + "-probe.py",
            "no-api" + "keyhelper-scan",
        )
    )
)
# Code and configuration: what runs or is read by something that runs. Prose (documents, changelog
# fragments) records history and is judged by the scrub, not here.
CODE_SUFFIXES = {".rs", ".py", ".sh", ".ts", ".js", ".mjs", ".svelte", ".toml", ".yml", ".yaml"}
CODE_NAMES = {"stack.json", "methodology.json", "package.json", "economy.json"}
STAGES_ALL = re.compile(r"^STAGES_ALL=\(([^)]*)\)", re.M)
TEMPLATE = REPO / ".github" / "pull_request_template.md"
SPECS = REPO / "docs" / "specs"
RECORDS = REPO / "docs" / "red-first"
THIS_SPEC = (
    SPECS / "SPEC-056-every-pack-is-judged-on-the-box-and-nothing-of-the-hub-is-published.md"
)
# A fence as the sdd and tdd packs read one: it opens with ``` at column 0 and closes with a bare
# ```. A criterion id, and a table's first cell holding one plain or struck (SPEC-056 R14).
FENCE_OPEN = re.compile(r"^```+(.*)$")
CRITERION = r"[A-Za-z]{1,4}\d+[a-z]?"
CRITERION_LINE = re.compile(rf"^({CRITERION})\s*:")
PLAIN_ID = re.compile(rf"^({CRITERION})$")
STRUCK_ID = re.compile(rf"^~~({CRITERION})~~$")
NUMBERING = re.compile(r"^\s*(?:\d+[a-z]?|[A-Z])[.)]\s+")


def tracked(root):
    """Every file git tracks or would add under `root`."""
    done = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        check=True,
    )
    return [name for name in done.stdout.decode("utf-8").split("\0") if name]


def fences(text):
    """Every fenced block in `text`, as (info string, lines), read the way the sdd and tdd packs
    read one."""
    found, info, lines = [], None, []
    for line in text.splitlines():
        if info is None:
            opened = FENCE_OPEN.match(line)
            if opened:
                info, lines = opened.group(1).strip(), []
            continue
        if line.rstrip() == "```":
            found.append((info, lines))
            info = None
            continue
        lines.append(line)
    if info is not None:
        found.append((info, lines))
    return found


def criteria_in(blocks, kind):
    """Every criterion id with a line in a block of `kind`."""
    found = set()
    for info, lines in blocks:
        if info == kind:
            found.update(m.group(1) for m in map(CRITERION_LINE.match, lines) if m)
    return found


def section_rows(text, pattern):
    """The cells of every table data row in each level-2 section whose title matches `pattern`,
    outside fences (the packs' own reading of a section)."""
    rows, title, in_fence, previous = [], None, False, False
    for line in text.splitlines():
        if in_fence or FENCE_OPEN.match(line):
            in_fence = (line.rstrip() != "```") if in_fence else True
            previous = False
            continue
        if line.startswith("## "):
            title, previous = NUMBERING.sub("", line[3:].strip()), False
            continue
        stripped = line.strip()
        if title is None or not re.search(pattern, title, re.I) or not stripped.startswith("|"):
            previous = False
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if all(re.fullmatch(r":?-{3,}:?", cell) for cell in cells if cell):
            previous = True
        elif previous:
            rows.append(cells)
    return rows


def retirement_problems(spec, record):
    """The criteria a SPEC retires, and what breaks SPEC-056 R14 in the SPEC and its record."""
    blocks = fences(spec)
    retired, live = criteria_in(blocks, "retired"), criteria_in(blocks, "acceptance")
    firsts = [row[0].strip("*` ") for row in section_rows(spec, r"\bacceptance\b") if row]
    plain = {m.group(1) for m in map(PLAIN_ID.match, firsts) if m}
    struck = {m.group(1) for m in map(STRUCK_ID.match, firsts) if m}
    kept = fences(record)
    recorded, set_apart = criteria_in(kept, "red-first"), criteria_in(kept, "retired")
    problems = [f"{c} is retired but not struck in the table" for c in sorted(retired - struck)]
    problems += [f"{c} is struck but has no retired line" for c in sorted(struck - retired)]
    problems += [f"{c} still has a line in the acceptance fence" for c in sorted(retired & live)]
    problems += [f"{c} is still stated in the table" for c in sorted(retired & plain)]
    problems += [f"{c} is still in the red-first fence" for c in sorted(retired & recorded)]
    problems += [
        f"{c} has no line in the record's retired fence" for c in sorted(retired - set_apart)
    ]
    return retired, problems


def is_code(name):
    path = Path(name)
    return path.suffix in CODE_SUFFIXES or path.name in CODE_NAMES or path.name == "check.sh"


def vendored_readers(root, names):
    """Each code file under `root` that names a vendored path, as (file, line number, line)."""
    found = []
    for name in names:
        path = root / name
        if not is_code(name) or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for number, line in enumerate(text.splitlines(), start=1):
            if VENDORED_READS.search(line):
                found.append((name, number, line.strip()[:120]))
    return found


class NoVendoredPackFileIsPublished(unittest.TestCase):
    def test_no_vendored_pack_file_or_removed_script_is_in_the_tree(self):
        names = examined("tracked files", tracked(REPO))
        vendored = [name for name in names if name.startswith(PACKS_DIR + "/")]
        self.assertEqual(vendored, [], f"{len(vendored)} vendored file(s) remain")
        self.assertFalse((REPO / PACKS_DIR).exists(), f"{PACKS_DIR} still exists")
        remaining = [name for name in REMOVED_SCRIPTS if (REPO / name).exists()]
        self.assertEqual(remaining, [], "removed scripts remain")
        # The box driver is DeckStreak's own and stays.
        self.assertIn("scripts/box-packs.sh", names)

    def test_methodology_json_names_no_vendored_commit(self):
        config = json.loads((REPO / "methodology.json").read_text(encoding="utf-8"))
        self.assertIsInstance(config, dict)
        # DeckStreak's own configuration stays: the box run's probes read it with --root.
        for section in examined("methodology sections", ["sdd", "ddd", "tdd"]):
            self.assertIn(section, config)
        self.assertNotIn("vendored" + "_from", config)

    def test_the_gate_and_ci_have_no_packs_stage_or_job(self):
        stages = examined(
            "gate stages",
            STAGES_ALL.search((REPO / "scripts" / "check.sh").read_text()).group(1).split(),
        )
        self.assertNotIn("packs", stages)
        self.assertIn("scrub", stages, "the public gate keeps its scrub")
        ci = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        jobs = examined("CI jobs", re.findall(r"(?m)^  ([a-z-]+):\n", ci.split("\njobs:\n", 1)[1]))
        self.assertNotIn("packs", jobs)
        needs = re.search(r"(?ms)^  ci:\n.*?^    needs: \[([^\]]*)\]", ci).group(1)
        self.assertNotIn("packs", [need.strip() for need in needs.split(",")])
        self.assertIn("hygiene", [need.strip() for need in needs.split(",")])

    def test_no_file_reads_a_vendored_path(self):
        # The census finds a planted reader, and no reader in the tree.
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch)
            (planted / "src").mkdir()
            reader = 'const RAILS: &str = include_str!("../' + PACKS_DIR + '/rails.json");\n'
            (planted / "src" / "lib.rs").write_text(reader, encoding="utf-8")
            (planted / "notes.md").write_text(reader, encoding="utf-8")
            found = vendored_readers(planted, ["src/lib.rs", "notes.md"])
            self.assertEqual([(name, number) for name, number, _ in found], [("src/lib.rs", 1)])
        names = tracked(REPO)
        code = examined("code and configuration files", [name for name in names if is_code(name)])
        self.assertGreater(len(code), 100)
        self.assertEqual(vendored_readers(REPO, names), [])


class RetiredCriteriaAreSetApartInsertOnly(unittest.TestCase):
    def test_every_retired_criterion_is_struck_and_fenced_apart_in_its_spec_and_record(self):
        # The rule's checker refuses each way a planted SPEC and record can break it.
        spec = (
            "## 3. Acceptance criteria\n\n| id | criterion | decided by |\n|---|---|---|\n"
            "| A1 | kept | a test |\n| A2 | retired, not struck | a removed test |\n"
            "| ~~A3~~ | struck, still fenced | a removed test |\n\n"
            "```acceptance\nA1: cmd\nA3: cmd\n```\n```retired\nA2: cmd\nA3: cmd\n```\n"
        )
        record = "```red-first\nA1: red at 1111111: x\nA1: green at 2222222\nA2: not red: y\n```\n"
        retired, problems = retirement_problems(spec, record)
        self.assertEqual(retired, {"A2", "A3"})
        self.assertEqual(
            problems,
            [
                "A2 is retired but not struck in the table",
                "A3 still has a line in the acceptance fence",
                "A2 is still stated in the table",
                "A2 is still in the red-first fence",
                "A2 has no line in the record's retired fence",
                "A3 has no line in the record's retired fence",
            ],
        )
        # Every delivered SPEC keeps the rule, and section 7 lists each retirement exactly.
        found = []
        for path in examined("delivered SPECs", sorted(SPECS.glob("SPEC-*.md"))):
            ident = re.match(r"^SPEC-\d+", path.name).group(0)
            record = RECORDS / f"{ident}.md"
            text = record.read_text(encoding="utf-8") if record.is_file() else ""
            retired, problems = retirement_problems(path.read_text(encoding="utf-8"), text)
            self.assertEqual(problems, [], path.name)
            found += [(ident, criterion) for criterion in retired]
        rows = section_rows(THIS_SPEC.read_text(encoding="utf-8"), r"^retired criteria$")
        listed = [(row[0].strip("` "), row[1].strip("` ")) for row in rows if len(row) > 1]
        self.assertEqual(sorted(listed), sorted(examined("retired criteria", found)))


class ThePullRequestTemplateNamesTheBoxRun(unittest.TestCase):
    def test_the_pull_request_template_says_pack_verdicts_come_from_the_box_run(self):
        lines = TEMPLATE.read_text(encoding="utf-8").splitlines()
        named = [line for line in lines if "box/packs" in line]
        self.assertEqual(len(named), 1, "one line names the box run's status")
        self.assertRegex(named[0], r"pack verdict")
        self.assertRegex(named[0], r"maintainer's box run")


if __name__ == "__main__":
    unittest.main()

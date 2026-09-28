"""The public tree carries no vendored pack file, and every pack is judged on the box (SPEC-056 A1 to
A4 and A16; ADR-069).

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
        re.escape(token)
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


def tracked(root):
    """Every file git tracks or would add under `root`."""
    done = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        check=True,
    )
    return [name for name in done.stdout.decode("utf-8").split("\0") if name]


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


class ThePullRequestTemplateNamesTheBoxRun(unittest.TestCase):
    def test_the_pull_request_template_says_pack_verdicts_come_from_the_box_run(self):
        lines = TEMPLATE.read_text(encoding="utf-8").splitlines()
        named = [line for line in lines if "box/packs" in line]
        self.assertEqual(len(named), 1, "one line names the box run's status")
        self.assertRegex(named[0], r"pack verdict")
        self.assertRegex(named[0], r"maintainer's box run")


if __name__ == "__main__":
    unittest.main()

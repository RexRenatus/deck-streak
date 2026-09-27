"""The public scrub refuses private shapes and literals and passes harmless ones (SPEC-002 A7),
reads every blob a push publishes, and refuses binaries and oversize files (SPEC-033 A1 to A8)."""

import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO

SCRUB = REPO / "scripts" / "public-scrub.py"
# Built at run time, so this file itself carries no address for the tree scan to find.
PLANTED = ".".join(["10", "20", "30", "40"])
# A compiled-cache shape: a NUL byte and bytes that are not UTF-8, built at run time.
BINARY = bytes([0, 255, 254]) + b"compiled" + bytes(8)
# Temporary repositories are isolated from the machine's git configuration and hooks.
GIT = [
    "git", "-c", "user.name=scrub-test", "-c", "user.email=scrub-test@example.invalid",
    "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "-c", "init.defaultBranch=main",
]


def scrub(text, private_literals=()):
    tmp = Path(tempfile.mkdtemp())
    subject = tmp / "subject"
    subject.mkdir()
    (subject / "note.md").write_text(text, encoding="utf-8")
    args = [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree", "--subject", str(subject)]
    if private_literals:
        deny = tmp / "private.json"
        deny.write_text(json.dumps({
            "schema": "phx.persona.deny.v1", "key_markers": [], "patterns": [],
            "literals": list(private_literals), "journal_paths": [],
        }))
        args += ["--deny-list", str(deny)]
    return subprocess.run(args, capture_output=True, text=True, check=False)


class PublicScrubHoldsTheLine(unittest.TestCase):
    def test_an_internal_address_is_refused_by_rule_name(self):
        done = scrub(f"the host answers at {PLANTED} today\n")
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("note.md:1: ipv4", done.stdout)
        self.assertNotIn(PLANTED, done.stdout)

    def test_a_private_literal_is_refused_by_index_never_by_value(self):
        done = scrub("the deck is called Secret Deck Name\n", ["secret deck name"])
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("private literal #0", done.stdout)
        self.assertNotIn("Secret Deck Name", done.stdout)

    def test_loopback_documentation_and_versions_pass(self):
        done = scrub("bind 127.0.0.1, see 192.0.2.10, release v1.2.3.4 and [::2]\n")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("examined 1 file(s)", done.stdout)

    def test_an_empty_subject_is_void(self):
        tmp = Path(tempfile.mkdtemp())
        done = subprocess.run(
            [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree", "--subject", str(tmp)],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(done.returncode, 3, done.stdout)



def new_repo(tmp):
    root = Path(tmp) / "repo"
    subprocess.run([*GIT, "init", "-q", str(root)], capture_output=True, check=True)
    return root


def commit_all(root, message):
    subprocess.run([*GIT, "-C", str(root), "add", "-A"], capture_output=True, check=True)
    subprocess.run(
        [*GIT, "-C", str(root), "commit", "-q", "-m", message], capture_output=True, check=True
    )


def run_scrub(root, *extra):
    return subprocess.run(
        [sys.executable, str(SCRUB), "--root", str(root), *extra],
        capture_output=True, text=True, check=False,
    )


def private_list(directory, literals):
    deny = Path(directory) / "private.json"
    deny.write_text(json.dumps({
        "schema": "phx.persona.deny.v1", "key_markers": [], "patterns": [],
        "literals": list(literals), "journal_paths": [],
    }))
    return deny


class EveryPublishedBlobIsRead(unittest.TestCase):
    def test_a_committed_binary_is_refused_by_rule_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
            (root / "cache.pyc").write_bytes(BINARY)
            commit_all(root, "add a compiled cache")
            done = run_scrub(root)
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertIn("cache.pyc: binary", done.stdout)

    def test_a_binary_blob_only_history_holds_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
            (root / "cache.pyc").write_bytes(BINARY)
            commit_all(root, "add a compiled cache")
            (root / "cache.pyc").unlink()
            commit_all(root, "drop the compiled cache")
            self.assertEqual(run_scrub(root).returncode, 0, "the tree alone is clean")
            done = run_scrub(root, "--history")
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertRegex(done.stdout, r"history:cache\.pyc@[0-9a-f]{9}: binary")

    def test_a_private_literal_in_a_deleted_file_is_found_in_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            notes = "line one\nthe deck is called Secret Deck Name\n"
            (root / "notes.md").write_text(notes, encoding="utf-8")
            commit_all(root, "add notes")
            (root / "notes.md").unlink()
            (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
            commit_all(root, "replace the notes")
            deny = private_list(tmp, ["secret deck name"])
            done = run_scrub(root, "--history", "--deny-list", str(deny))
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertRegex(done.stdout, r"history:notes\.md@[0-9a-f]{9}:2: private literal #0")
            self.assertNotIn("Secret Deck Name", done.stdout)

    def test_an_address_in_a_deleted_file_is_found_in_history(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            (root / "hosts.md").write_text(f"the host answers at {PLANTED}\n", encoding="utf-8")
            commit_all(root, "add hosts")
            (root / "hosts.md").unlink()
            (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
            commit_all(root, "replace the hosts")
            done = run_scrub(root, "--history")
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertRegex(done.stdout, r"history:hosts\.md@[0-9a-f]{9}:1: ipv4")
            self.assertNotIn(PLANTED, done.stdout)

    def test_a_clean_history_is_examined_blob_by_blob_and_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
            commit_all(root, "add the readme")
            (root / "docs").mkdir()
            (root / "docs" / "guide.md").write_text("a clean guide\n", encoding="utf-8")
            commit_all(root, "add the guide")
            done = run_scrub(root, "--history")
            self.assertEqual(done.returncode, 0, done.stdout)
            self.assertRegex(done.stdout, r"examined 2 file\(s\) and 2 history blob\(s\) against")

    def test_a_shallow_history_is_void_not_green(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            for number in (1, 2):
                readme = f"a clean readme, version {number}\n"
                (root / "README.md").write_text(readme, encoding="utf-8")
                commit_all(root, f"write version {number}")
            shallow = Path(tmp) / "shallow"
            subprocess.run(
                [*GIT, "clone", "-q", "--depth", "1", root.as_uri(), str(shallow)],
                capture_output=True, check=True,
            )
            done = run_scrub(shallow, "--history")
            self.assertEqual(done.returncode, 3, done.stdout)
            self.assertIn("shallow", done.stdout)

    def test_an_oversize_file_is_refused_rather_than_skipped(self):
        with tempfile.TemporaryDirectory() as tmp:
            subject = Path(tmp) / "subject"
            subject.mkdir()
            (subject / "big.txt").write_text("a\n" * 1_000_001, encoding="utf-8")
            done = run_scrub(REPO, "--no-tree", "--subject", str(subject))
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertIn("big.txt: oversize", done.stdout)

    def test_the_gate_scrub_stage_reads_every_blob_of_history(self):
        text = (REPO / "scripts" / "check.sh").read_text(encoding="utf-8")
        body = re.search(r"(?ms)^stage_scrub\(\) \{\n(.*?)^\}", text).group(1)
        self.assertRegex(body, r"python3 scripts/public-scrub\.py --root \. --history\b")

if __name__ == "__main__":
    unittest.main()

"""The public scrub refuses private shapes and literals and passes harmless ones (SPEC-002 A7),
reads every blob a push publishes, and refuses binaries and oversize files (SPEC-033 A1 to A8).
It takes a file as its own subject, refuses a subject that does not exist or examines nothing,
passes a systemd unit instance name, and composes its rules once (SPEC-054 A1 to A5, A7)."""

import json
import os
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
# A unit instance name of each type SPEC-054 R2 admits: a template, its instance and the type.
UNIT_NAMES = (
    "getty@tty1.service",
    "backup@nightly.timer",
    "echo@web.socket",
    "watch@inbox.path",
    "media@usb1.mount",
    "container@alpha.target",
    "tenant@alpha.slice",
    "session@c2.scope",
)
# Addresses that only look like unit names, each still an address. Assembled at run time, so this
# file holds none: a unit word as a label that is not the last, a longer last label, a last label
# that merely ends with a unit word, a unit type in capitals, and a unit word inside a label.
LOOKALIKES = tuple(
    "@".join(["ops", ".".join(labels)])
    for labels in (
        ("service", "corp-mail", "com"),
        ("corp-mail", "services"),
        ("corp-mail", "webservice"),
        ("corp-mail", "SERVICE"),
        ("timer-corp", "net"),
    )
)
# A compiled-cache shape: a NUL byte and bytes that are not UTF-8, built at run time.
BINARY = bytes([0, 255, 254]) + b"compiled" + bytes(8)
# Temporary repositories are isolated from the machine's git configuration and hooks.
GIT = [
    "git",
    "-c",
    "user.name=scrub-test",
    "-c",
    "user.email=scrub-test@example.invalid",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "init.defaultBranch=main",
]


def scrub(text, private_literals=()):
    with tempfile.TemporaryDirectory() as scratch:
        tmp = Path(scratch)
        subject = tmp / "subject"
        subject.mkdir()
        (subject / "note.md").write_text(text, encoding="utf-8")
        args = [
            sys.executable,
            str(SCRUB),
            "--root",
            str(REPO),
            "--no-tree",
            "--subject",
            str(subject),
        ]
        if private_literals:
            deny = tmp / "private.json"
            deny.write_text(
                json.dumps(
                    {
                        "schema": "phx.persona.deny.v1",
                        "key_markers": [],
                        "patterns": [],
                        "literals": list(private_literals),
                        "journal_paths": [],
                    }
                )
            )
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
        with tempfile.TemporaryDirectory() as tmp:
            done = subprocess.run(
                [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree", "--subject", tmp],
                capture_output=True,
                text=True,
                check=False,
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
        capture_output=True,
        text=True,
        check=False,
    )


def private_list(directory, literals):
    deny = Path(directory) / "private.json"
    deny.write_text(
        json.dumps(
            {
                "schema": "phx.persona.deny.v1",
                "key_markers": [],
                "patterns": [],
                "literals": list(literals),
                "journal_paths": [],
            }
        )
    )
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
                capture_output=True,
                check=True,
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


def public_scrub(root, *extra):
    """The scrub over the public shapes alone: the maintainer's private list, which a gate run
    exports, is dropped, so these verdicts never depend on a value this repository cannot hold."""
    env = {key: value for key, value in os.environ.items() if key != "PERSONA_CORE_DENY_LIST"}
    return subprocess.run(
        [sys.executable, str(SCRUB), "--root", str(root), *extra],
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )


def findings(done):
    """Each finding line, as `<file name>:<line>: <rule>`, in the order the scrub printed them."""
    prefix = "public-scrub: "
    return [
        line[len(prefix) :].rsplit("/", 1)[-1]
        for line in done.stdout.splitlines()
        if line.startswith(prefix) and not line.startswith(f"{prefix}VOID")
    ]


class EverySubjectIsExaminedOrRefusedByName(unittest.TestCase):
    def test_a_file_given_as_subject_is_its_own_subject(self):
        with tempfile.TemporaryDirectory() as tmp:
            body = (Path(tmp) / "body.md").resolve()
            body.write_text(f"the host answers at {PLANTED}\n", encoding="utf-8")
            done = public_scrub(REPO, "--no-tree", "--subject", str(body))
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertIn(f"public-scrub: {body}:1: ipv4", done.stdout.splitlines())
            self.assertNotIn(PLANTED, done.stdout)
            body.write_text("a clean pull-request body\n", encoding="utf-8")
            done = public_scrub(REPO, "--no-tree", "--subject", str(body))
            self.assertEqual(done.returncode, 0, done.stdout)
            self.assertEqual(
                done.stdout.splitlines()[-1],
                "examined 1 file(s) against public shapes only; 0 finding(s)",
            )

    def test_a_subject_that_does_not_exist_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / "no-such-body.md"
            done = public_scrub(REPO, "--no-tree", "--subject", str(missing))
            self.assertEqual(done.returncode, 2, done.stdout)
            self.assertEqual(
                done.stdout.splitlines(), [f"public-scrub: the subject {missing} does not exist"]
            )
            # A path that is neither a file nor a directory is refused the same way.
            pipe = Path(tmp) / "a-pipe"
            os.mkfifo(pipe)
            done = public_scrub(REPO, "--no-tree", "--subject", str(pipe))
            self.assertEqual(done.returncode, 2, done.stdout)
            self.assertEqual(
                done.stdout.splitlines(),
                [f"public-scrub: the subject {pipe} is not a file or a directory"],
            )

    def test_a_subject_that_examines_nothing_is_void_by_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = new_repo(tmp)
            (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
            commit_all(root, "add the readme")
            empty = Path(tmp) / "empty"
            empty.mkdir()
            done = public_scrub(root, "--subject", str(empty))
            self.assertEqual(done.returncode, 3, done.stdout)
            self.assertIn(
                f"public-scrub: VOID: the subject {empty} examined no file",
                done.stdout.splitlines(),
            )
            self.assertIn("examined 1 file(s)", done.stdout, "the tree beside it was examined")
            # A subject that holds only a file the scrub skips examines nothing as well.
            (empty / "LICENSE").write_text("a license text\n", encoding="utf-8")
            done = public_scrub(root, "--subject", str(empty))
            self.assertEqual(done.returncode, 3, done.stdout)
            self.assertIn(f"the subject {empty} examined no file", done.stdout)

    def test_a_private_list_the_rules_cannot_read_stops_the_scrub_by_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            subject = Path(tmp) / "subject"
            subject.mkdir()
            (subject / "note.md").write_text("a clean note\n", encoding="utf-8")
            missing = Path(tmp) / "no-such-list.json"
            done = public_scrub(
                REPO, "--no-tree", "--subject", str(subject), "--deny-list", str(missing)
            )
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            self.assertEqual(
                done.stdout.splitlines(),
                [f"public-scrub: the private list {missing} is not a file"],
            )
            malformed = Path(tmp) / "malformed.json"
            malformed.write_text("{ a list that is not JSON\n", encoding="utf-8")
            done = public_scrub(
                REPO, "--no-tree", "--subject", str(subject), "--deny-list", str(malformed)
            )
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            self.assertTrue(
                done.stdout.startswith("public-scrub: a deny list cannot be read: "), done.stdout
            )
            self.assertIn(str(malformed), done.stdout)
            self.assertEqual(done.stderr, "", "a refusal, never a traceback")


class UnitInstanceNamesAreNotAddresses(unittest.TestCase):
    def test_a_systemd_unit_instance_name_passes_the_email_rule(self):
        prose = "".join(f"the unit {name} restarts on failure\n" for name in UNIT_NAMES)
        done = scrub_public_text(prose)
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertEqual(findings(done), [])
        self.assertEqual(
            done.stdout.splitlines()[-1],
            "examined 1 file(s) against public shapes only; 0 finding(s)",
        )
        # Beside an address on the same line, the unit name still passes and the address does
        # not: the admission is judged match by match, never line by line.
        done = scrub_public_text(f"the unit {UNIT_NAMES[0]} mails {LOOKALIKES[0]}\n")
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertEqual(findings(done), ["note.md:1: email"])

    def test_an_address_that_only_looks_like_a_unit_name_is_still_refused(self):
        prose = "".join(f"write to {address} today\n" for address in LOOKALIKES)
        done = scrub_public_text(prose)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertEqual(
            findings(done), [f"note.md:{line}: email" for line in range(1, len(LOOKALIKES) + 1)]
        )
        for address in LOOKALIKES:
            self.assertNotIn(address, done.stdout)


def scrub_public_text(text):
    """The scrub over one note holding `text`, against the public shapes alone."""
    with tempfile.TemporaryDirectory() as scratch:
        subject = Path(scratch) / "subject"
        subject.mkdir()
        (subject / "note.md").write_text(text, encoding="utf-8")
        return public_scrub(REPO, "--no-tree", "--subject", str(subject))


if __name__ == "__main__":
    unittest.main()

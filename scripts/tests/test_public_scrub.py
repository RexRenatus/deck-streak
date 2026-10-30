"""The public scrub refuses private shapes and literals and passes harmless ones (SPEC-002 A7),
reads every blob a push publishes, and refuses binaries and oversize files (SPEC-033 A1 to A8).
It takes a file as its own subject, refuses a subject that does not exist or examines nothing,
passes a systemd unit instance name, and composes its rules once (SPEC-054 A1 to A5, A7). Its
public shapes are its own files, each family still found, and its loader still reads the private
list's schema (SPEC-056 A9, A10)."""

import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRUB = REPO / "scripts" / "public-scrub.py"
# The scrub's own public shapes (SPEC-056 R5).
RULES = REPO / "scripts" / "scrub-rules"
# One planted value for each public shape, by the rule that must find it. Each is assembled at run
# time from parts, so this file holds none of them for the tree scan to find.
PLANTED_SHAPES = {
    "ipv4": ".".join(["10", "20", "30", "40"]),
    "ip-hostname": "-".join(["10", "20", "30", "40"]) + ".ss" + "lip.io",
    "github-token": "gh" + "p_" + "A1b2" * 6,
    "sk-key": "s" + "k-" + "a1B2" * 6,
    "google-key": "AI" + "za" + "b2C3" * 6,
    "aws-key-id": "AK" + "IA" + "C3D4" * 4,
    "slack-token": "xo" + "xb-" + "12345678901",
    "telegram-bot-token": "123456789" + ":" + "A1b2" * 8 + "Cde",
    "telegram-chat-id": "-100" + "1234567890",
    "private-key": "-----BEGIN " + "RSA PRIVATE " + "KEY-----",
    "email": "ops" + "@" + "corp-mail" + ".com",
    "ipv6": ":".join(["fd12", "3456", "789a", "bcde"]) + "::" + "1",
    "cloud-project-flag": "--pro" + "ject " + "acme-prod-42",
    "cloud-project-key": "project" + "_id: " + "acme-prod-42",
    "cloud-resource-path": "pro" + "jects/" + "acme-prod-42" + "/secrets",
    "secret-name-flag": "--sec" + "ret " + "db-password",
    "secret-name-path": "sec" + "rets/" + "db-password" + "/versions",
    "secret-name-command": "sec" + "rets create " + "db-password",
    "bucket-url": "g" + "s://" + "acme-backups-42",
    "cloud-service-host": "acme-api" + ".a.run" + ".app",
    "internal-host": "db" + ".lan" + ".internal",
    "home-directory": "/ho" + "me/" + "rexdev",
    "telegram-id-in-context": "chat" + "_id = " + "73" + "51642980",
    "phone-in-context": "pho" + "ne: " + "+1 415 555 0199",
}
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


def scrub_module():
    """The scrub, imported as a module, to read its loader."""
    spec = importlib.util.spec_from_file_location("public_scrub", SCRUB)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# Every unit type SPEC-056 section 9 lists, and every suffix a unit's directory may carry. Each
# expectation below is written from that rule, not from the scrub's own answer.
UNIT_PATH_TYPES = (
    "service",
    "socket",
    "device",
    "mount",
    "automount",
    "swap",
    "path",
    "timer",
    "slice",
    "scope",
)
UNIT_PATH_SUFFIXES = ("", ".d", ".wants", ".requires", ".upholds")
# The places a unit instance path may begin: a line start, after whitespace, after a slash, after a
# quote, after `=`, each of them as a credential directive or a prose line would write it.
UNIT_PATH_CONTEXTS = (
    "{n}",
    "see {n} for details",
    "path /run/credentials/{n}token",
    'name="{n}"',
    "LoadCredential=cred:/run/credentials/{n}cred",
    "Watch={n}",
)
# Where a real address must still be found, and the local parts to try, assembled at run time.
ADDRESS_CONTEXTS = (
    "{a}",
    "write to {a} today",
    'mail "{a}"',
    "reply={a}",
    "mail:{a}",
    "from <{a}>",
    "if {a} asks, say so",
    "/srv/{a}",
)
ADDRESS_LOCALS = ("ops", "jane.doe", "a_b-c")
RESERVED_DOMAINS = (
    "example.com",
    "example.org",
    "example.net",
    "mail.example.com",
    "svc.test",
    "a.b.test",
    "host.invalid",
    "db.localhost",
    "x.example",
)


class UnitInstancePathsAreNotAddresses(unittest.TestCase):
    def test_a_systemd_unit_instance_path_passes_the_email_rule(self):
        lines = [
            context.format(n=f"getty@tty1.{kind}{suffix}/")
            for kind in UNIT_PATH_TYPES
            for suffix in UNIT_PATH_SUFFIXES
            for context in UNIT_PATH_CONTEXTS
        ]
        examined("unit instance path lines", lines)
        self.assertEqual(len(lines), 300, "the population is 10 types x 5 suffixes x 6 contexts")
        done = scrub_public_text("".join(f"{line}\n" for line in lines))
        flagged = findings(done)
        print(f"examined {len(lines)} lines, mismatches {len(flagged)}")
        self.assertEqual(flagged, [], "each line is a unit path, so none is an address")
        self.assertEqual(done.returncode, 0, done.stdout)

    def test_a_real_address_is_still_found_in_every_context(self):
        lines = [
            context.format(a="@".join([local, "corp-mail.com"]))
            for local in ADDRESS_LOCALS
            for context in ADDRESS_CONTEXTS
        ]
        examined("address lines", lines)
        self.assertEqual(len(lines), 24, "the population is 3 local parts x 8 contexts")
        done = scrub_public_text("".join(f"{line}\n" for line in lines))
        expected = [f"note.md:{number}: email" for number in range(1, len(lines) + 1)]
        print(f"examined {len(lines)} lines, mismatches {len(set(expected) ^ set(findings(done)))}")
        self.assertEqual(findings(done), expected)
        self.assertEqual(done.returncode, 1, done.stdout)

    def test_an_address_at_a_reserved_domain_still_passes(self):
        lines = [
            context.format(a="@".join(["ops", domain]))
            for domain in RESERVED_DOMAINS
            for context in ("{a}", "write to {a} today")
        ]
        examined("reserved domain lines", lines)
        self.assertEqual(len(lines), 18, "the population is 9 domains x 2 contexts")
        done = scrub_public_text("".join(f"{line}\n" for line in lines))
        self.assertEqual(findings(done), [])
        self.assertEqual(done.returncode, 0, done.stdout)


class TheScrubOwnsItsShapes(unittest.TestCase):
    def test_each_public_shape_family_is_found_from_the_scrubs_own_rules(self):
        files = sorted(path.name for path in RULES.glob("*.json")) if RULES.is_dir() else []
        self.assertEqual(files, ["persona-core.json", "privacy-gdpr.json"])
        ids = [
            row["id"]
            for name in files
            for row in json.loads((RULES / name).read_text(encoding="utf-8"))["patterns"]
        ]
        self.assertEqual(sorted(ids), sorted(PLANTED_SHAPES), "every public shape is planted")
        for rule, value in examined("public shapes", sorted(PLANTED_SHAPES.items())):
            with self.subTest(rule=rule):
                done = scrub(f"a planted value: {value}\n")
                self.assertEqual(done.returncode, 1, done.stdout)
                self.assertIn(f"note.md:1: {rule}", done.stdout)
                self.assertNotIn(value, done.stdout)

    def test_the_loader_reads_a_synthetic_private_list_in_its_schema(self):
        module = scrub_module()
        self.assertTrue(
            hasattr(module, "load_deny"), "the scrub reads its lists with its own loader"
        )
        word = "zebra" + "-canary"
        shape = "quokka" + "-[0-9]{3}"
        with tempfile.TemporaryDirectory() as tmp:
            private = Path(tmp) / "private.json"
            private.write_text(
                json.dumps(
                    {
                        "schema": "phx.persona.deny.v1",
                        "key_markers": ["marker"],
                        "patterns": [{"id": "planted-shape", "flags": "i", "regex": shape}],
                        "literals": [word.upper()],
                        "journal_paths": ["diary"],
                    }
                ),
                encoding="utf-8",
            )
            loaded = module.load_deny(private, "private")
            self.assertEqual(
                [(o, i) for o, i, _ in loaded["patterns"]], [("private", "planted-shape")]
            )
            self.assertTrue(loaded["patterns"][0][2].search("QUOKKA-123"), "the flags are read")
            self.assertEqual(loaded["literals"], [("private", word)], "literals are casefolded")
            subject = Path(tmp) / "subject"
            subject.mkdir()
            (subject / "note.md").write_text(f"{word} and quokka-042\n", encoding="utf-8")
            done = public_scrub(
                REPO, "--no-tree", "--subject", str(subject), "--deny-list", str(private)
            )
            self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
            self.assertIn("note.md:1: planted-shape", done.stdout)
            self.assertIn("note.md:1: private literal #0", done.stdout)
            self.assertNotIn(word, done.stdout)
            # A list in another schema is refused by name.
            private.write_text(json.dumps({"schema": "another.schema.v1"}), encoding="utf-8")
            done = public_scrub(
                REPO, "--no-tree", "--subject", str(subject), "--deny-list", str(private)
            )
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            self.assertIn("schema is not phx.persona.deny.v1", done.stdout)

    def test_the_flags_default_is_excused_by_one_whole_record(self):
        # SPEC-492 A2 (R3): the scrub's mapped suite runs no record census, so its one equivalent
        # survivor, the flags default, is excused by a record that binds it alone.
        path = REPO / "scripts" / "mutation-equivalent.d" / "python.json"
        records = json.loads(path.read_text(encoding="utf-8"))["records"]
        mine = [
            record
            for record in examined("Python equivalence records", records)
            if record.get("file") == "scripts/public-scrub.py"
        ]
        self.assertEqual(len(mine), 1, "exactly one record names scripts/public-scrub.py")
        record = mine[0]
        self.assertEqual(
            sorted(record),
            ["anchor", "evidence", "file", "issue", "mutant", "reached_by", "reason"],
        )
        self.assertEqual(record["mutant"], 'replace "" with "XX" in compiled')
        self.assertEqual(record["anchor"], 'str(row.get("flags", ""))')
        self.assertEqual(
            record["reached_by"],
            "test_public_scrub.TheScrubOwnsItsShapes."
            "test_the_loader_reads_a_synthetic_private_list_in_its_schema",
        )
        self.assertEqual(SCRUB.read_text(encoding="utf-8").count('str(row.get("flags", ""))'), 1)


def deny_list(directory, name="deny.json", **fields):
    """A deny list in the scrub's schema holding `fields`, written to `directory` as `name`."""
    path = Path(directory) / name
    path.write_text(json.dumps({"schema": "phx.persona.deny.v1", **fields}), encoding="utf-8")
    return path


def scrub_help():
    """(the exit, the words) of the scrub's --help, its words joined by single spaces, so neither
    the terminal's width nor colour decides what the words are."""
    env = {key: value for key, value in os.environ.items() if key != "PERSONA_CORE_DENY_LIST"}
    done = subprocess.run(
        [sys.executable, str(SCRUB), "--help"],
        capture_output=True,
        text=True,
        check=False,
        env=dict(env, COLUMNS="200", NO_COLOR="1", PYTHON_COLORS="0"),
    )
    return done.returncode, " ".join(done.stdout.split())


class TheLoaderRefusesEveryMalformedList(unittest.TestCase):
    """The scrub's own loader, read in process over literal rows and scratch lists: a malformed
    list or row is refused by name, and a row's flags and id are read as the schema says."""

    def test_a_row_with_no_i_flag_matches_case(self):
        module = scrub_module()
        rule, pattern = module.compiled({"id": "a", "regex": "ABC"}, "w")
        self.assertEqual(rule, "a")
        self.assertEqual(pattern.search("xABCx").group(0), "ABC")
        self.assertIsNone(pattern.search("abc"))

    def test_key_markers_that_are_not_a_list_are_refused(self):
        module = scrub_module()
        with tempfile.TemporaryDirectory() as tmp:
            path = deny_list(tmp, key_markers="x")
            with self.assertRaises(module.DenyListError) as refused:
                module.load_deny(path, "private")
        self.assertEqual(str(refused.exception), "private deny-list: key_markers is not a list")

    def test_a_public_list_tags_its_literals_public(self):
        module = scrub_module()
        with tempfile.TemporaryDirectory() as tmp:
            deny_list(tmp, "persona-core.json", literals=["x"])
            deny_list(tmp, "privacy-gdpr.json", literals=["y"])
            module.RULES = Path(tmp)
            scan = module.rules(None)
        self.assertEqual(scan.literals, [("public", "x"), ("public", "y")])
        self.assertEqual(scan.rows, [])

    def test_a_pattern_row_without_a_regex_is_refused_by_name(self):
        module = scrub_module()
        # Each operand alone: a dict with no regex, and a row that is no dict.
        for row in examined("rows without a regex", [{"id": "x"}, "x"]):
            with self.subTest(row=row):
                with self.assertRaises(module.DenyListError) as refused:
                    module.compiled(row, "w")
                self.assertEqual(str(refused.exception), "w: a row has no regex")

    def test_a_row_with_no_id_is_named_by_a_question_mark(self):
        module = scrub_module()
        rule, pattern = module.compiled({"regex": "x"}, "w")
        self.assertEqual(rule, "?")
        self.assertEqual(pattern.pattern, "x")

    def test_journal_paths_that_are_not_a_list_are_refused(self):
        module = scrub_module()
        with tempfile.TemporaryDirectory() as tmp:
            path = deny_list(tmp, journal_paths="x")
            with self.assertRaises(module.DenyListError) as refused:
                module.load_deny(path, "private")
        self.assertEqual(str(refused.exception), "private deny-list: journal_paths is not a list")

    def test_a_literal_that_is_not_a_string_is_refused_by_name(self):
        module = scrub_module()
        # Each operand alone: a literal that is no string, and an empty one.
        for literal in examined("malformed literals", [5, ""]):
            with self.subTest(literal=literal), tempfile.TemporaryDirectory() as tmp:
                path = deny_list(tmp, literals=[literal])
                with self.assertRaises(module.DenyListError) as refused:
                    module.load_deny(path, "private")
                self.assertEqual(
                    str(refused.exception), "private deny-list: literals holds a non-string"
                )


class TheCommandLineSaysWhatItDoes(unittest.TestCase):
    """The scrub run as a script: its help names each option and opens with its docstring's first
    paragraph, its private list is read from its environment variable, and a run that examines
    nothing is VOID."""

    def test_help_names_each_option_by_its_help_text(self):
        code, said = scrub_help()
        self.assertEqual(code, 0, said)
        for text in examined(
            "option help texts",
            [
                "a file, or every file under a directory",
                "scan only the --subject paths",
                "also read every blob reachable from --rev",
                "the revision whose history --history reads",
            ],
        ):
            with self.subTest(text=text):
                self.assertIn(text, said)

    def test_help_opens_with_the_docstrings_first_paragraph(self):
        code, said = scrub_help()
        self.assertEqual(code, 0, said)
        self.assertIn(
            "public-scrub: refuse any file, blob or issue body that would disclose what a public "
            "repository must not.",
            said,
        )

    def test_the_private_list_is_read_from_its_environment_variable(self):
        env = {key: value for key, value in os.environ.items() if key != "PERSONA_CORE_DENY_LIST"}
        with tempfile.TemporaryDirectory() as tmp:
            note = Path(tmp) / "note.md"
            note.write_text("a clean note\n", encoding="utf-8")
            private = private_list(tmp, ["zebra" + "-canary"])
            done = subprocess.run(
                [sys.executable, str(SCRUB), "--root", tmp, "--no-tree", "--subject", str(note)],
                capture_output=True,
                text=True,
                check=False,
                env=dict(env, PERSONA_CORE_DENY_LIST=str(private)),
            )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(
            done.stdout.splitlines(),
            ["examined 1 file(s) against public shapes and the private list; 0 finding(s)"],
        )

    def test_a_run_that_examines_nothing_is_void(self):
        with tempfile.TemporaryDirectory() as tmp:
            done = public_scrub(Path(tmp), "--no-tree")
        self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
        self.assertEqual(
            done.stdout.splitlines(),
            ["examined 0 file(s) against public shapes only; 0 finding(s)"],
        )


# SPEC-579's planted brand file: the listed path, and the list's own path at a scanned root.
LISTED = "icons/logo.png"
BRAND_LIST = "scripts/brand-files.json"


def write_brand_list(root, entries):
    """Write the brand list at `root`: one entry per (path, bytes), its sha256 and size computed
    from the bytes, so this file holds no digest."""
    files = [
        {"path": path, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}
        for path, data in entries
    ]
    listed = root / BRAND_LIST
    listed.parent.mkdir(parents=True, exist_ok=True)
    listed.write_text(json.dumps({"files": files}, indent=2) + "\n", encoding="utf-8")


def brand_repo(tmp, files, entries):
    """A committed repository holding a clean README, `files` (path to bytes) and a brand list of
    `entries`."""
    root = new_repo(tmp)
    (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
    for path, data in files.items():
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)
    write_brand_list(root, entries)
    commit_all(root, "add the brand files")
    return root


class AListedBrandBinaryIsAdmitted(unittest.TestCase):
    """SPEC-579 A15 to A19 (R6; ADR-598 D5): a binary is admitted only at a path the brand list
    names with its sha256 and byte count, in the tree scan and in the history scan; an admitted
    binary is still searched for every private literal; every other binary is refused as before;
    and a list the scrub cannot read stops it by name."""

    def test_a_listed_binary_passes_the_tree_and_history_scans(self):
        # A15.
        with tempfile.TemporaryDirectory() as tmp:
            root = brand_repo(tmp, {LISTED: BINARY}, [(LISTED, BINARY)])
            done = run_scrub(root)
            self.assertEqual(done.returncode, 0, done.stdout)
            self.assertNotIn("binary", done.stdout)
            self.assertRegex(done.stdout, r"examined 3 file\(s\) against .*; 0 finding\(s\)")
            done = run_scrub(root, "--history")
            self.assertEqual(done.returncode, 0, done.stdout)
            self.assertNotIn("binary", done.stdout)
            self.assertRegex(
                done.stdout, r"examined 3 file\(s\) and 3 history blob\(s\) against .*; 0 finding"
            )

    def test_a_listed_path_with_other_bytes_is_refused_as_binary(self):
        # A16: one byte changed, the size kept.
        other = BINARY[:-1] + b"\x01"
        with tempfile.TemporaryDirectory() as tmp:
            root = brand_repo(tmp, {LISTED: other}, [(LISTED, BINARY)])
            done = run_scrub(root, "--history")
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertIn(f"public-scrub: {LISTED}: binary", done.stdout.splitlines())
            self.assertRegex(done.stdout, r"history:icons/logo\.png@[0-9a-f]{9}: binary")

    def test_listed_bytes_at_an_unlisted_path_are_refused_as_binary(self):
        # A17.
        with tempfile.TemporaryDirectory() as tmp:
            root = brand_repo(tmp, {"icons/other.png": BINARY}, [(LISTED, BINARY)])
            done = run_scrub(root, "--history")
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertIn("public-scrub: icons/other.png: binary", done.stdout.splitlines())
            self.assertRegex(done.stdout, r"history:icons/other\.png@[0-9a-f]{9}: binary")

    def test_a_private_literal_inside_a_listed_binary_is_still_found(self):
        # A18: the listed bytes hold the word "compiled", which the private list names.
        with tempfile.TemporaryDirectory() as tmp:
            root = brand_repo(tmp, {LISTED: BINARY}, [(LISTED, BINARY)])
            deny = private_list(tmp, ["compiled"])
            done = run_scrub(root, "--history", "--deny-list", str(deny))
            self.assertEqual(done.returncode, 1, done.stdout)
            self.assertIn(f"public-scrub: {LISTED}: private literal #0", done.stdout.splitlines())
            self.assertRegex(
                done.stdout, r"history:icons/logo\.png@[0-9a-f]{9}: private literal #0"
            )
            self.assertNotIn("binary", done.stdout)

    def test_a_malformed_list_stops_the_scrub_by_name(self):
        # A19: a list that is not JSON, an entry whose sha256 has 63 digits, a path named twice.
        digest = hashlib.sha256(BINARY).hexdigest()
        entry = {"path": LISTED, "sha256": digest, "bytes": len(BINARY)}
        other = {"path": LISTED, "sha256": hashlib.sha256(b"other").hexdigest(), "bytes": 5}
        planted = {
            "not JSON": "{ a list that is not JSON\n",
            "a 63-digit sha256": json.dumps({"files": [dict(entry, sha256=digest[:63])]}),
            "a path named twice": json.dumps({"files": [entry, other]}),
        }
        reached = 0
        for name, text in examined("planted malformed lists", list(planted.items())):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                root = new_repo(tmp)
                (root / "README.md").write_text("a clean readme\n", encoding="utf-8")
                (root / "scripts").mkdir()
                (root / BRAND_LIST).write_text(text, encoding="utf-8")
                commit_all(root, "add a malformed brand list")
                done = run_scrub(root)
                self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
                self.assertIn(BRAND_LIST, done.stdout)
                self.assertEqual(done.stderr, "", "a refusal, never a traceback")
                reached += 1
        self.assertEqual(reached, len(planted))


if __name__ == "__main__":
    unittest.main()

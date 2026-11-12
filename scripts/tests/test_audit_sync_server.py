"""The sync server's dependency audit (SPEC-340 R7, A8; ADR-351 D5). `scripts/audit-sync-server.sh`
reads the fork and the commit from the engine's patch entry, as the release's build step does,
refuses a commit that is not a full one before it fetches anything, fetches that commit, runs the
FORK-LOCK GUARD over the fork's lockfile (SPEC-565 A5; ADR-583), and runs the RustSec advisory
check on the server's own manifest with the house `deny.toml`, under the fork's lockfile.

    bash scripts/audit-sync-server.sh --work DIR [--manifest FILE] [--deny FILE]

Stub `git` and `cargo` stand first on PATH and record their arguments, so nothing is fetched and
no audit reaches a network. The stub `git` checks nothing out, so each test plants the lockfile the
fetched commit would hold: a green one, unless it plants a red one or none."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "audit-sync-server.sh"
FORK_LOCKS = REPO / "scripts" / "tests" / "fixtures" / "fork-lock"
FORK = "https://github.com/example/anki.git"
REV = "0123456789abcdef0123456789abcdef01234567"
# Each stub writes its name and its arguments, one tab-separated line per call; `cargo` exits with
# the status the test sets.
STUB = """#!/bin/bash
{ printf '%s' "${0##*/}"; printf '\\t%s' "$@"; printf '\\n'; } >> "$AUDIT_STUB_LOG"
if [ "${0##*/}" = cargo ]; then exit "${AUDIT_STUB_CARGO_EXIT:-0}"; fi
exit 0
"""


def manifest(rev):
    """A workspace manifest whose patch entry pins the fork at `rev`, as the engine's does."""
    return (
        "[workspace]\n"
        'members = ["crates/*"]\n\n'
        '[patch."https://github.com/ankitects/anki.git"]\n'
        f'anki = {{ git = "{FORK}", rev = "{rev}" }}\n'
    )


class TheAuditReadsThePin(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory(prefix="audit-sync-server-")
        self.addCleanup(scratch.cleanup)
        self.tmp = Path(scratch.name)
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        for name in ("git", "cargo"):
            stub = self.bin / name
            stub.write_text(STUB, encoding="utf-8")
            stub.chmod(0o755)
        self.runs = 0
        self.deny = self.tmp / "deny.toml"
        self.deny.write_text("[advisories]\nignore = []\n", encoding="utf-8")
        self.work = self.tmp / "work"
        self.work.mkdir()
        self.fork_lock = self.work / "Cargo.lock"
        shutil.copyfile(FORK_LOCKS / "green.lock", self.fork_lock)

    def run_audit(self, rev=REV, cargo_exit=0):
        """One run of the audit over a manifest pinning `rev`, with its own log of the stubs'
        calls."""
        self.runs += 1
        log = self.tmp / f"calls-{self.runs}.log"
        cargo_toml = self.tmp / "Cargo.toml"
        cargo_toml.write_text(manifest(rev), encoding="utf-8")
        env = {
            **os.environ,
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "AUDIT_STUB_LOG": str(log),
            "AUDIT_STUB_CARGO_EXIT": str(cargo_exit),
        }
        done = subprocess.run(
            ["bash", str(SCRIPT), "--manifest", str(cargo_toml), "--deny", str(self.deny)]
            + ["--work", str(self.work)],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )
        lines = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
        calls = [line.split("\t") for line in lines]
        return done, calls

    def test_the_audit_checks_the_pinned_commits_advisories_under_the_forks_lockfile(self):
        """A8: the pinned commit is fetched, and the advisory check runs on the server's manifest
        with the house config, under the fork's own lockfile."""
        done, calls = self.run_audit()
        work = str(self.work)
        self.assertEqual(
            [call for call in calls if call[0] == "cargo"],
            [
                [
                    "cargo",
                    "deny",
                    "--manifest-path",
                    f"{work}/rslib/sync/Cargo.toml",
                    "--locked",
                    "--config",
                    str(self.deny),
                    "check",
                    "advisories",
                ]
            ],
        )
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(
            [call for call in calls if call[0] == "git"],
            [
                ["git", "-C", work, "init", "-q"],
                ["git", "-C", work, "fetch", "-q", "--depth", "1", FORK, REV],
                ["git", "-C", work, "checkout", "-q", "FETCH_HEAD"],
            ],
        )
        self.assertEqual(calls[-1][0], "cargo", "the check runs after the commit is checked out")

    def test_a_failed_check_fails_the_audit(self):
        """A8: an advisory the check reports fails the audit, so the release never builds."""
        done, calls = self.run_audit(cargo_exit=1)
        self.assertEqual([call[:2] for call in calls if call[0] == "cargo"], [["cargo", "deny"]])
        self.assertEqual(done.returncode, 1, done.stderr)

    def test_a_short_commit_is_refused_before_any_fetch(self):
        """A8: a pin that is not a full commit is refused, and nothing is fetched or checked."""
        for rev in examined("pin(s) that are not a full commit", (REV[:39], REV.upper(), "main")):
            with self.subTest(rev=rev):
                done, calls = self.run_audit(rev=rev)
                self.assertIn(f"pins {rev}, not a full commit", done.stderr)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(calls, [], "a refused pin fetches and checks nothing")

    def test_a_red_fork_lock_ends_the_audit_before_the_check(self):
        """SPEC-565 A5: a block-buffer below 0.12.1 or a digest below 0.11.0 in the fetched
        commit's lockfile ends the audit with exit 1, and the advisory check never runs."""
        shutil.copyfile(FORK_LOCKS / "a078820-digest-line.lock", self.fork_lock)
        done, calls = self.run_audit()
        self.assertEqual(done.returncode, 1, "a RED fork lock must end the audit with exit 1")
        self.assertIn("block-buffer below 0.12.1: 2 of 2", done.stdout)
        self.assertEqual([call for call in calls if call[0] == "cargo"], [])

    def test_a_missing_fork_lock_ends_the_audit_before_the_check(self):
        """SPEC-565 A5: a fetched commit with no lockfile ends the audit VOID, exit 2, and the
        advisory check never runs."""
        self.fork_lock.unlink()
        done, calls = self.run_audit()
        self.assertEqual(done.returncode, 2, "a missing fork lock must end the audit with exit 2")
        self.assertIn("lock-digest-line: VOID", done.stdout)
        self.assertEqual([call for call in calls if call[0] == "cargo"], [])


if __name__ == "__main__":
    unittest.main()

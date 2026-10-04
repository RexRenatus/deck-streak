"""The sync server's launcher (SPEC-337 A5; R2; ADR-347 D2, D3).

The launcher runs as its unit runs it: a copy sits at `deploy/scripts/` of a planted release whose
`bin/anki-sync-server` is a stub that prints what the server would read, the credentials directory
holds a user entry per credential the unit loads, and the state directory is a scratch one. Every
hash is made at run time from a scratch password, so none is in the tree. Nothing listens and no
server runs.
"""

import base64
import hashlib
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

LAUNCHER = REPO / "deploy" / "scripts" / "sync-server.sh"
# The ids the unit loads, in its order: the owner's user, then ADR-344's staging user.
OWNER = "sync-server-owner"
STAGING = "sync-server-staging"
LISTEN = "127.0.0.1:8081"
# What the server reads, as the stub prints it: each name, then whether a third user is set.
SERVER_READS = (
    "SYNC_HOST",
    "SYNC_PORT",
    "SYNC_BASE",
    "SYNC_USER1",
    "SYNC_USER2",
    "PASSWORDS_HASHED",
    "SYNC_USER3",
    "MAX_SYNC_PAYLOAD_MEGS",
)
STUB = "#!/bin/sh\n" + "".join(
    f'printf "{name}=%s\\n" "${{{name}-<unset>}}"\n' for name in SERVER_READS
)


def phc(password, salt):
    """`password` hashed in the PHC form the server verifies, pbkdf2-sha256 (ADR-340)."""
    rounds = 600_000
    digest = hashlib.pbkdf2_hmac("sha256", password.encode(), salt, rounds, 32)

    def b64(raw):
        return base64.b64encode(raw).decode().rstrip("=")

    return f"$pbkdf2-sha256$i={rounds},l=32${b64(salt)}${b64(digest)}"


class TheLauncherStartsTheServerWithHashedUsers(unittest.TestCase):
    def test_the_launcher_refuses_a_bad_entry_and_execs_the_server_with_hashed_users(self):
        self.assertTrue(LAUNCHER.is_file(), f"no launcher at {LAUNCHER.relative_to(REPO)}")
        owner = f"owner:{phc('scratch-owner', b'scratch-salt-one')}"
        staging = f"staging:{phc('scratch-staging', b'scratch-salt-two')}"
        # Each case: the credentials it plants (None leaves one out), the launcher's environment
        # beyond the release's, and the refusal its standard error names (None: it starts).
        good = {OWNER: owner, STAGING: staging}
        cases = {
            "both users": (good, {}, None),
            "settings that would reach the server are cleared": (
                good,
                {
                    "SYNC_USER3": "extra:kept",
                    "SYNC_BASE": "/elsewhere",
                    "PASSWORDS_HASHED": "",
                    "MAX_SYNC_PAYLOAD_MEGS": "4096",
                },
                None,
            ),
            "the owner's entry is missing": (
                {STAGING: staging},
                {},
                f"the credential {OWNER} is missing",
            ),
            "the staging entry is missing": (
                {OWNER: owner},
                {},
                f"the credential {STAGING} is missing",
            ),
            "an empty entry": (
                {OWNER: "", STAGING: staging},
                {},
                f"the credential {OWNER} is empty",
            ),
            "a plain password": (
                {OWNER: "owner:scratch-owner", STAGING: staging},
                {},
                f"the credential {OWNER} is not a user name and a pbkdf2-sha256 hash",
            ),
            "no name": (
                {OWNER: ":" + owner.partition(":")[2], STAGING: staging},
                {},
                f"the credential {OWNER} is not a user name and a pbkdf2-sha256 hash",
            ),
            "a name that leaves the data directory": (
                {OWNER: "../owner" + owner[len("owner") :], STAGING: staging},
                {},
                f"the credential {OWNER} is not a user name and a pbkdf2-sha256 hash",
            ),
            "a hash of another scheme": (
                {OWNER: owner.replace("$pbkdf2-sha256$", "$argon2id$"), STAGING: staging},
                {},
                f"the credential {OWNER} is not a user name and a pbkdf2-sha256 hash",
            ),
            "a hash with no salt": (
                {OWNER: owner.rsplit("$", 2)[0] + "$" + owner.rsplit("$", 1)[1], STAGING: staging},
                {},
                f"the credential {OWNER} is not a user name and a pbkdf2-sha256 hash",
            ),
            "one name twice": (
                {OWNER: owner, STAGING: "owner" + staging[len("staging") :]},
                {},
                "both users have the same name",
            ),
            "an address that is not loopback": (
                good,
                {"DECKSTREAK_SYNC_SERVER_LISTEN": "0.0.0.0:8081"},
                "DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port",
            ),
            "an address with no port": (
                good,
                {"DECKSTREAK_SYNC_SERVER_LISTEN": "127.0.0.1"},
                "DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port",
            ),
            "a port out of range": (
                good,
                {"DECKSTREAK_SYNC_SERVER_LISTEN": "127.0.0.1:65536"},
                "DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port",
            ),
            "port zero": (
                good,
                {"DECKSTREAK_SYNC_SERVER_LISTEN": "127.0.0.1:0"},
                "DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port",
            ),
            "an octet out of range": (
                good,
                {"DECKSTREAK_SYNC_SERVER_LISTEN": "127.0.0.256:8081"},
                "DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port",
            ),
            "no address": (
                good,
                {"DECKSTREAK_SYNC_SERVER_LISTEN": None},
                "DECKSTREAK_SYNC_SERVER_LISTEN is not a loopback address with a port",
            ),
            "no credentials directory": (
                good,
                {"CREDENTIALS_DIRECTORY": None},
                "no credentials directory",
            ),
            "no state directory": (good, {"STATE_DIRECTORY": None}, "no state directory"),
        }
        got = {}
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            release = root / "release"
            (release / "deploy" / "scripts").mkdir(parents=True)
            (release / "bin").mkdir()
            script = release / "deploy" / "scripts" / "sync-server.sh"
            shutil.copy2(LAUNCHER, script)
            server = release / "bin" / "anki-sync-server"
            server.write_text(STUB, encoding="utf-8")
            server.chmod(0o755)
            state = root / "state"
            state.mkdir()
            for label in examined("launcher case(s)", sorted(cases)):
                planted, extra, _ = cases[label]
                credentials = root / "credentials" / label.replace(" ", "-").replace("'", "")
                credentials.mkdir(parents=True)
                for ident, entry in planted.items():
                    (credentials / ident).write_text(f"{entry}\n", encoding="utf-8")
                env = {
                    "PATH": os.environ["PATH"],
                    "CREDENTIALS_DIRECTORY": str(credentials),
                    "STATE_DIRECTORY": str(state),
                    "DECKSTREAK_SYNC_SERVER_LISTEN": LISTEN,
                    **extra,
                }
                done = subprocess.run(
                    [str(script)],
                    env={key: value for key, value in env.items() if value is not None},
                    capture_output=True,
                    text=True,
                )
                got[label] = (done.returncode, done.stdout, done.stderr)
        started = (
            0,
            "".join(
                f"{name}={value}\n"
                for name, value in (
                    ("SYNC_HOST", "127.0.0.1"),
                    ("SYNC_PORT", "8081"),
                    ("SYNC_BASE", str(state)),
                    ("SYNC_USER1", owner),
                    ("SYNC_USER2", staging),
                    ("PASSWORDS_HASHED", "1"),
                    ("SYNC_USER3", "<unset>"),
                    ("MAX_SYNC_PAYLOAD_MEGS", "<unset>"),
                )
            ),
        )
        for label, (planted, _, refusal) in cases.items():
            code, out, err = got[label]
            if refusal is None:
                self.assertEqual((code, out), started, label)
                self.assertEqual(err, "", label)
                continue
            # A refusal exits 1 before the server runs, names what it refused, and never prints
            # an entry it read.
            self.assertEqual((code, out), (1, ""), label)
            self.assertEqual(err, f"sync-server: {refusal}\n", label)
            for entry in planted.values():
                if entry:
                    self.assertNotIn(entry.partition(":")[2] or entry, err, label)


if __name__ == "__main__":
    unittest.main()

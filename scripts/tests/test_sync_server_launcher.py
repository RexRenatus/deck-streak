"""The sync server's launcher (SPEC-337 A5; R2; ADR-347 D2, D3).

The launcher runs as its unit runs it: a copy sits at `deploy/scripts/` of a planted release whose
`bin/anki-sync-server` is a stub that prints what the server would read, the credentials directory
holds a user entry per credential the unit loads, and the state directory is a scratch one. Every
hash is made at run time from a scratch password, so none is in the tree. Nothing listens and no
server runs.
"""

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
# The launcher's refusal of an entry that is not a name and a hash of the house's shape (SPEC-340
# R10).
SHAPE = "is not a user name and a pbkdf2-sha256 hash of the house's shape"
# The standard base64 alphabet. The PHC form spells its salt and its hash in it, without padding.
B64_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"


def unpadded_b64(raw):
    """`raw` in standard base64 with no `=` padding, six bits to a character, the last character's
    spare bits zero. Spelt from the alphabet so the test directory's read census (SPEC-190 R12)
    places every name here without a module it does not vet."""
    bits = "".join(f"{byte:08b}" for byte in raw)
    bits += "0" * (-len(bits) % 6)
    return "".join(B64_ALPHABET[int(bits[at : at + 6], 2)] for at in range(0, len(bits), 6))


def phc(password, salt):
    """`password` hashed in the PHC form the server verifies, pbkdf2-sha256 (ADR-340)."""
    rounds = 600_000
    digest = hashlib.pbkdf2_hmac("sha256", password.encode(), salt, rounds, 32)
    return f"$pbkdf2-sha256$i={rounds},l=32${unpadded_b64(salt)}${unpadded_b64(digest)}"


def shaped(name, rounds, salt, digest, length="32"):
    """A user entry in the PHC form with every part chosen by the caller, so a shape the launcher
    must refuse is built as plainly as one it admits; `length=None` leaves `l=` out."""
    parameters = f"i={rounds}" + ("" if length is None else f",l={length}")
    return f"{name}:$pbkdf2-sha256${parameters}${salt}${digest}"


def launched(root, label, planted):
    """The launcher's exit, standard output and standard error, run as its unit runs it over a
    planted release under `root`, with the credentials `planted` names by id."""
    release = root / "release"
    script = release / "deploy" / "scripts" / "sync-server.sh"
    if not script.exists():
        script.parent.mkdir(parents=True)
        (release / "bin").mkdir()
        shutil.copy2(LAUNCHER, script)
        server = release / "bin" / "anki-sync-server"
        server.write_text(STUB, encoding="utf-8")
        server.chmod(0o755)
        (root / "state").mkdir()
    credentials = root / "credentials" / label.replace(" ", "-").replace("'", "")
    credentials.mkdir(parents=True)
    for ident, entry in planted.items():
        (credentials / ident).write_text(f"{entry}\n", encoding="utf-8")
    done = subprocess.run(
        [str(script)],
        env={
            "PATH": os.environ["PATH"],
            "CREDENTIALS_DIRECTORY": str(credentials),
            "STATE_DIRECTORY": str(root / "state"),
            "DECKSTREAK_SYNC_SERVER_LISTEN": LISTEN,
        },
        capture_output=True,
        text=True,
    )
    return done.returncode, done.stdout, done.stderr


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
                f"the credential {OWNER} {SHAPE}",
            ),
            "no name": (
                {OWNER: ":" + owner.partition(":")[2], STAGING: staging},
                {},
                f"the credential {OWNER} {SHAPE}",
            ),
            "a name that leaves the data directory": (
                {OWNER: "../owner" + owner[len("owner") :], STAGING: staging},
                {},
                f"the credential {OWNER} {SHAPE}",
            ),
            "a hash of another scheme": (
                {OWNER: owner.replace("$pbkdf2-sha256$", "$argon2id$"), STAGING: staging},
                {},
                f"the credential {OWNER} {SHAPE}",
            ),
            "a hash with no salt": (
                {OWNER: owner.rsplit("$", 2)[0] + "$" + owner.rsplit("$", 1)[1], STAGING: staging},
                {},
                f"the credential {OWNER} {SHAPE}",
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

    def test_the_launcher_admits_only_the_house_hash_shape(self):
        """SPEC-340 A12 (R10; ADR-351 D8): the house's shape is 600000 to 999999 rounds, an
        optional `l=32`, a 16-byte salt and a 32-byte digest, each in canonical unpadded standard
        base64. The launcher starts the server on that shape and refuses every departure."""
        salt = os.urandom(16)
        digest = hashlib.pbkdf2_hmac("sha256", b"scratch-owner", salt, 600_000, 32)
        good_salt, good_digest = unpadded_b64(salt), unpadded_b64(digest)
        staging = f"staging:{phc('scratch-staging', b'scratch-salt-two')}"
        admitted = {
            "the house's rounds and length": shaped("owner", 600000, good_salt, good_digest),
            "no length": shaped("owner", 600000, good_salt, good_digest, None),
            "the most rounds": shaped("owner", 999999, good_salt, good_digest),
        }
        refused = {
            "a length other than thirty-two": shaped(
                "owner", 600000, good_salt, good_digest, "64"
            ),
            "rounds below the floor": shaped("owner", 599999, good_salt, good_digest),
            "rounds above the ceiling": shaped("owner", 1000000, good_salt, good_digest),
            "seven digits of rounds": shaped("owner", 6000000, good_salt, good_digest),
            "one round": shaped("owner", 1, good_salt, good_digest),
            "a sixteen-byte digest": shaped(
                "owner", 600000, good_salt, unpadded_b64(digest[:16]), "16"
            ),
            "a fifteen-byte salt": shaped("owner", 600000, unpadded_b64(salt[:15]), good_digest),
            "a thirty-one-byte digest": shaped(
                "owner", 600000, good_salt, unpadded_b64(digest[:31])
            ),
            "a salt not in canonical form": shaped(
                "owner", 600000, good_salt[:-1] + "B", good_digest
            ),
            "a digest not in canonical form": shaped(
                "owner", 600000, good_salt, good_digest[:-1] + "B"
            ),
        }
        got = {}
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            for label in examined("hash shape case(s)", sorted({**admitted, **refused})):
                entry = admitted.get(label) or refused[label]
                got[label] = launched(root, label, {OWNER: entry, STAGING: staging})
        for label, entry in refused.items():
            code, out, err = got[label]
            # Refused before the server runs, by name, and the hash it read is never printed.
            self.assertEqual((code, out), (1, ""), label)
            self.assertEqual(err, f"sync-server: the credential {OWNER} {SHAPE}\n", label)
            self.assertNotIn(entry.partition(":")[2], err, label)
        for label, entry in admitted.items():
            code, out, err = got[label]
            self.assertEqual(code, 0, label)
            self.assertIn(f"SYNC_USER1={entry}\n", out, label)
            self.assertEqual(err, "", label)


if __name__ == "__main__":
    unittest.main()

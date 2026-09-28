"""The private rail's public contract (SPEC-061 A1 to A7; ADR-038, ADR-061).

The rail itself (the credential socket's helper, its socket unit, the map, the rendered settings
file, the drop-ins and the guards' copies) is private and never committed (SPEC-061 R1). This
repository ships only what the rail reads and checks: `deploy/rail-contract.json`, the neutral
values the rail overrides by unit and key, and three checks the rail and the gate run,
`deploy/scripts/credential-pairs.py`, `effective-check.py` and `guards-check.py`.

Every case runs a check as the rail runs it, over synthetic input written at run time: a template
tree, a `systemctl cat` output, a guard manifest. None of them reads a host. The guard check's
ownership rule is judged in process, with root's uid set to the test's own, because a test cannot
create a file root owns; the check's command line keeps uid 0.
"""

import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path

from _support import REPO, examined

DEPLOY = REPO / "deploy"
SCRIPTS = DEPLOY / "scripts"
PAIRS = SCRIPTS / "credential-pairs.py"
EFFECTIVE = SCRIPTS / "effective-check.py"
GUARDS = SCRIPTS / "guards-check.py"
CONTRACT = DEPLOY / "rail-contract.json"
SCRUB = REPO / "scripts" / "public-scrub.py"
# The rail's public contract, file by file (SPEC-061 §5).
CONTRACT_FILES = (PAIRS, EFFECTIVE, GUARDS, CONTRACT)
SOCKET = "/run/deck-streak-credentials/socket"
# ADR-032's neutral values, as that record decides them: the example release root, the one settings
# file every service reads, and calendars written in UTC at the default rollover hour.
RELEASE_ROOT = "/usr/local/lib/deck-streak/current"
SETTINGS_FILE = "/etc/deck-streak/deck-streak.env"
NEUTRAL = {
    "release_root": RELEASE_ROOT,
    "environment_file": SETTINGS_FILE,
    "time_zone": "UTC",
    "rollover_hour": 4,
}
# A deployment's own values, for the rail's drop-ins in these tests: synthetic, never a host's.
EXAMPLE_ROOT = "/srv/rail-example/current"
EXAMPLE_SETTINGS = "/etc/rail-example/settings.env"
EXAMPLE_ZONE = "Etc/GMT-3"
UNIT_DIR = "/etc/systemd/system"
UNIT_SUFFIXES = (".service", ".timer", ".socket")


def run(script, *args, stdin=None):
    return subprocess.run(
        [sys.executable, str(script), *map(str, args)],
        input=stdin,
        capture_output=True,
        text=True,
        check=False,
    )


def write_tree(root, files):
    """Write each file of `files` under `root`; returns how many lines they hold together."""
    for rel, text in files.items():
        path = Path(root) / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return sum(len(text.splitlines()) for text in files.values())


def cat(unit_path, unit_text, *dropins):
    """A `systemctl cat` output: each file under a `# <path>` line, a blank line between files."""
    blocks = [(unit_path, unit_text), *dropins]
    return "\n".join(f"# {path}\n{text.rstrip(chr(10))}\n" for path, text in blocks)


def rail_dropin(section, *assignments):
    """The rail's drop-in as ADR-061 writes it: each listed key reset, then set."""
    lines = [f"[{section}]"]
    for key, value in assignments:
        lines += [f"{key}=", f"{key}={value}"]
    return "\n".join(lines) + "\n"


def template(name):
    return (DEPLOY / "systemd" / name).read_text(encoding="utf-8")


def neutral_lines(root):
    """Every line of a unit under `root`'s deploy/ that carries one of ADR-032's neutral values, as
    (unit, section, key, value). The test's own reading, which never consults the contract."""
    found = []
    for path in sorted((Path(root) / "deploy").rglob("*")):
        if path.suffix not in UNIT_SUFFIXES or not path.is_file():
            continue
        section = None
        for raw in path.read_text(encoding="utf-8").splitlines():
            line = raw.strip()
            if line.startswith("[") and line.endswith("]"):
                section = line[1:-1]
                continue
            key, equals, value = line.partition("=")
            if not equals or line.startswith(("#", ";")):
                continue
            calendar = key == "OnCalendar" and value.split()[-1:] == [NEUTRAL["time_zone"]]
            if RELEASE_ROOT in value or SETTINGS_FILE in value or calendar:
                found.append((path.name, section, key, value))
    return found


def load_guards_check():
    spec = importlib.util.spec_from_file_location("guards_check", GUARDS)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def judge_guards(manifest, root_uid):
    """The guard check's verdict on `manifest`, run in process with `root_uid` as root's uid."""
    module = load_guards_check()
    module.ROOT_UID = root_uid
    out = StringIO()
    with redirect_stdout(out):
        code = module.main([str(manifest)])
    return code, out.getvalue()


def write_manifest(path, entries):
    path.write_text(
        json.dumps({"schema": "deckstreak.guards-manifest.v1", "files": entries}, indent=2) + "\n",
        encoding="utf-8",
    )
    path.chmod(0o644)


def entry(path, mode):
    digest = hashlib.sha256(Path(path).read_bytes()).hexdigest()
    return {"path": str(path), "sha256": digest, "mode": f"{mode:04o}"}


def scrub(*subjects):
    return subprocess.run(
        [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree"]
        + [argument for path in subjects for argument in ("--subject", str(path))],
        capture_output=True,
        text=True,
        check=False,
    )


# A synthetic template tree: two services, a template unit and a timer, and one optional set.
SYNTHETIC = {
    "deploy/systemd/deck-streak-api.service": (
        "[Unit]\nDescription=a synthetic api\n\n[Service]\n"
        f"ExecStart={RELEASE_ROOT}/bin/deckstreakd api\n"
        f"LoadCredential=owner-user-id:{SOCKET}\n"
        f"LoadCredential=telegram-bot-token:{SOCKET}\n"
    ),
    "deploy/systemd/deck-streak-job@.service": (
        f"[Service]\nType=oneshot\nLoadCredential=anki-sync-username:{SOCKET}\n"
    ),
    "deploy/systemd/deck-streak-job@sync.timer": "[Timer]\nOnCalendar=*-*-* 04:07:00 UTC\n",
}
OPTIONAL = {
    "deploy/optional/ai-route/deck-streak-readings-generate.conf": (
        f"[Service]\nLoadCredential=agent-device-key:{SOCKET}\n"
    ),
    # An optional set's drop-in may name its unit's type, and may reset the unit's credentials.
    "deploy/optional/narrowed/deck-streak-api.service.conf": (
        f"[Service]\nLoadCredential=\nLoadCredential=owner-user-id:{SOCKET}\n"
    ),
}


class ThePairLister(unittest.TestCase):
    def test_the_pair_lister_prints_each_unit_and_credential_id(self):
        with tempfile.TemporaryDirectory() as scratch:
            lines = write_tree(scratch, SYNTHETIC)
            write_tree(scratch, OPTIONAL)
            done = run(PAIRS, "--root", scratch)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            listed = json.loads(done.stdout)
            # Each pair once, sorted, and the job template named as the template it is.
            self.assertEqual(
                listed["pairs"],
                [
                    {"unit": "deck-streak-api.service", "credential": "owner-user-id"},
                    {"unit": "deck-streak-api.service", "credential": "telegram-bot-token"},
                    {"unit": "deck-streak-job@.service", "credential": "anki-sync-username"},
                ],
            )
            self.assertEqual(listed["optional"], [])
            self.assertEqual(listed["examined"], {"files": 3, "lines": lines})

            # An optional set adds its drop-ins' pairs, for the unit each drop-in names.
            route = OPTIONAL["deploy/optional/ai-route/deck-streak-readings-generate.conf"]
            done = run(PAIRS, "--root", scratch, "--optional", "ai-route")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            listed = json.loads(done.stdout)
            self.assertIn(
                {"unit": "deck-streak-readings-generate.service", "credential": "agent-device-key"},
                listed["pairs"],
            )
            self.assertEqual(len(listed["pairs"]), 4)
            self.assertEqual(listed["optional"], ["ai-route"])
            self.assertEqual(
                listed["examined"], {"files": 4, "lines": lines + len(route.splitlines())}
            )

            # A drop-in's empty assignment resets the unit's list, as systemd reads it.
            done = run(PAIRS, "--root", scratch, "--optional", "narrowed")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            api = [
                pair["credential"]
                for pair in json.loads(done.stdout)["pairs"]
                if pair["unit"] == "deck-streak-api.service"
            ]
            self.assertEqual(api, ["owner-user-id"])

            # A set that does not exist is not an empty set.
            done = run(PAIRS, "--root", scratch, "--optional", "absent")
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            self.assertEqual(done.stdout, "")

    def test_the_pair_lister_refuses_encrypted_and_off_socket_credentials(self):
        # Every way to give a unit a credential but the socket is refused by file and line, and a
        # refused run prints no list for the rail to compare.
        planted = {
            f"LoadCredentialEncrypted=telegram-bot-token:{SOCKET}": "LoadCredentialEncrypted=",
            "LoadCredential=telegram-bot-token:/etc/credstore/telegram-bot-token": "not the socket",
            "LoadCredential=telegram-bot-token": "not the socket",
            "SetCredential=telegram-bot-token:planted": "SetCredential=",
            "SetCredentialEncrypted=telegram-bot-token:planted": "SetCredentialEncrypted=",
            "ImportCredential=telegram-*": "ImportCredential=",
        }
        for line, reason in planted.items():
            with self.subTest(line=line), tempfile.TemporaryDirectory() as scratch:
                write_tree(scratch, {"deploy/systemd/planted.service": f"[Service]\n{line}\n"})
                done = run(PAIRS, "--root", scratch)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertEqual(done.stdout, "")
                self.assertIn("REFUSE: deploy/systemd/planted.service:2:", done.stderr)
                self.assertIn(reason, done.stderr)
                self.assertNotIn("planted\n", done.stderr)
        # An optional set's drop-in is held to the same rule.
        with tempfile.TemporaryDirectory() as scratch:
            write_tree(scratch, SYNTHETIC)
            write_tree(
                scratch,
                {
                    "deploy/optional/ai-route/deck-streak-readings-generate.conf": (
                        "[Service]\n"
                        "LoadCredential=agent-device-key:/etc/credstore/agent-device-key\n"
                    )
                },
            )
            self.assertEqual(run(PAIRS, "--root", scratch).returncode, 0)
            done = run(PAIRS, "--root", scratch, "--optional", "ai-route")
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(
            "REFUSE: deploy/optional/ai-route/deck-streak-readings-generate.conf:2:", done.stderr
        )
        # The committed templates list without a refusal.
        done = run(PAIRS, "--root", REPO)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(done.stderr, "")
        examined(
            "credential pair(s) the committed templates declare", json.loads(done.stdout)["pairs"]
        )


class TheRailContract(unittest.TestCase):
    def test_the_rail_contract_names_every_neutral_value(self):
        contract = json.loads(CONTRACT.read_text(encoding="utf-8"))
        # The contract's neutral values are ADR-032's own.
        self.assertEqual(contract["neutral"], NEUTRAL)
        self.assertEqual(contract["drop_in"], "10-rail.conf")
        self.assertEqual(contract["socket"], SOCKET)
        named = {
            (row["unit"], row["section"], row["key"], value)
            for row in contract["values"]
            for value in row["neutral"]
        }
        carried = set(
            examined("neutral value(s) the committed templates carry", neutral_lines(REPO))
        )
        self.assertEqual(named, carried)
        done = run(EFFECTIVE, "--root", REPO, "--census")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(f"{len(carried)} neutral value(s), each named", done.stdout)

        # A planted template carrying a neutral value the contract does not name is refused.
        planted = f"ExecStart={RELEASE_ROOT}/bin/deckstreakd planted"
        with tempfile.TemporaryDirectory() as scratch:
            shutil.copytree(DEPLOY, Path(scratch) / "deploy")
            write_tree(
                scratch, {"deploy/systemd/deck-streak-planted.service": (f"[Service]\n{planted}\n")}
            )
            done = run(EFFECTIVE, "--root", scratch, "--census")
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(
            f"REFUSE: deck-streak-planted.service: the contract does not name [Service] {planted}",
            done.stdout,
        )
        # So is a row no template carries: the contract drifts in neither direction.
        stale = {"unit": "deck-streak-gone.service", "section": "Service", "key": "ExecStart"}
        stale["neutral"] = [f"{RELEASE_ROOT}/bin/deckstreakd gone"]
        with tempfile.TemporaryDirectory() as scratch:
            shutil.copytree(DEPLOY, Path(scratch) / "deploy")
            drifted = dict(contract, values=[*contract["values"], stale])
            (Path(scratch) / "deploy" / "rail-contract.json").write_text(json.dumps(drifted))
            done = run(EFFECTIVE, "--root", scratch, "--census")
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn("REFUSE: deck-streak-gone.service: no template carries", done.stdout)


class TheEffectiveCheck(unittest.TestCase):
    API = f"{UNIT_DIR}/deck-streak-api.service"
    API_RAIL = f"{UNIT_DIR}/deck-streak-api.service.d/10-rail.conf"
    API_DROPIN = rail_dropin(
        "Service",
        ("ExecStart", f"{EXAMPLE_ROOT}/bin/deckstreakd api"),
        ("EnvironmentFile", EXAMPLE_SETTINGS),
    )

    def check(self, output):
        return run(EFFECTIVE, "--root", REPO, "-", stdin=output)

    def test_the_effective_check_refuses_a_neutral_value_left_in_force(self):
        api = template("deck-streak-api.service")
        # The template alone: both of its neutral values are in force.
        done = self.check(cat(self.API, api))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        left = "REFUSE: deck-streak-api.service: neutral value left in force: [Service] "
        self.assertIn(f"{left}ExecStart={RELEASE_ROOT}/bin/deckstreakd api", done.stdout)
        self.assertIn(f"{left}EnvironmentFile={SETTINGS_FILE}", done.stdout)
        # The rail's drop-in overrides both, and the same unit passes.
        done = self.check(cat(self.API, api, (self.API_RAIL, self.API_DROPIN)))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("examined 1 unit(s)", done.stdout)
        # A value set without the reset is added beside the neutral one, which stays in force.
        appended = (
            f"[Service]\nExecStart=\nExecStart={EXAMPLE_ROOT}/bin/deckstreakd api\n"
            f"EnvironmentFile={EXAMPLE_SETTINGS}\n"
        )
        done = self.check(cat(self.API, api, (self.API_RAIL, appended)))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(f"{left}EnvironmentFile={SETTINGS_FILE}", done.stdout)
        self.assertNotIn(f"{left}ExecStart=", done.stdout)

        # A timer's calendar in UTC is left in force until the drop-in resets it.
        timer = template("deck-streak-job@sync.timer")
        path = f"{UNIT_DIR}/deck-streak-job@sync.timer"
        done = self.check(cat(path, timer))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(
            "REFUSE: deck-streak-job@sync.timer: neutral value left in force: [Timer] "
            "OnCalendar=*-*-* 04:07:00 UTC",
            done.stdout,
        )
        dropin = rail_dropin("Timer", ("OnCalendar", f"*-*-* 04:07:00 {EXAMPLE_ZONE}"))
        done = self.check(cat(path, timer, (f"{path}.d/10-rail.conf", dropin)))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)

        # An instance is shown under its template's file, with the template's drop-in, and several
        # units in one output are each judged.
        job = template("deck-streak-job@.service")
        job_path = f"{UNIT_DIR}/deck-streak-job@.service"
        job_dropin = rail_dropin(
            "Service",
            ("ExecStart", f"{EXAMPLE_ROOT}/bin/deckstreakd job %i"),
            ("EnvironmentFile", EXAMPLE_SETTINGS),
        )
        both = cat(self.API, api, (self.API_RAIL, self.API_DROPIN)) + "\n"
        both += cat(job_path, job, (f"{job_path}.d/10-rail.conf", job_dropin))
        done = self.check(both)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("examined 2 unit(s)", done.stdout)

        # A unit the contract does not list is still judged by the neutral values themselves.
        stray = f"[Service]\nExecStart={RELEASE_ROOT}/bin/deckstreakd stray\n"
        done = self.check(cat(f"{UNIT_DIR}/deck-streak-stray.service", stray))
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn(
            "REFUSE: deck-streak-stray.service: neutral value left in force:", done.stdout
        )

        # An output that holds no unit judged nothing.
        done = self.check("")
        self.assertEqual(done.returncode, 2, done.stdout + done.stderr)

    def test_the_effective_check_refuses_encrypted_secret_env_and_foreign_drop_ins(self):
        api = template("deck-streak-api.service")
        # A setting whose name carries no secret passes beside the rail's own values.
        allowed = self.API_DROPIN + "Environment=RUST_LOG=debug\n"
        done = self.check(cat(self.API, api, (self.API_RAIL, allowed)))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        refused = {
            f"LoadCredentialEncrypted=telegram-bot-token:{SOCKET}": "LoadCredentialEncrypted=",
            "Environment=TELEGRAM_BOT_TOKEN=planted": "TELEGRAM_BOT_TOKEN",
            'Environment="RUST_LOG=info" DECKSTREAK_AGENT_DEVICE_KEY=planted': (
                "DECKSTREAK_AGENT_DEVICE_KEY"
            ),
            "LoadCredential=owner-user-id:/etc/credstore/owner-user-id": "not the socket",
        }
        for line, reason in refused.items():
            with self.subTest(line=line):
                dropin = self.API_DROPIN + line + "\n"
                done = self.check(cat(self.API, api, (self.API_RAIL, dropin)))
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn("REFUSE: deck-streak-api.service:", done.stdout)
                self.assertIn(reason, done.stdout)
                # A value is never echoed.
                self.assertNotIn("planted", done.stdout + done.stderr)
        # Only the rail's own drop-in, beside the unit, may change a unit.
        foreign = (
            f"{UNIT_DIR}/deck-streak-api.service.d/override.conf",
            "/run/systemd/system/deck-streak-api.service.d/10-rail.conf",
            f"{UNIT_DIR}/service.d/10-rail.conf",
            f"{UNIT_DIR}/deck-streak-.service.d/10-rail.conf",
        )
        for path in foreign:
            with self.subTest(path=path):
                output = cat(
                    self.API, api, (self.API_RAIL, self.API_DROPIN), (path, "[Service]\nNice=5\n")
                )
                done = self.check(output)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn(
                    f"REFUSE: deck-streak-api.service: a drop-in that is not the rail's: {path}",
                    done.stdout,
                )


class TheGuardCheck(unittest.TestCase):
    def test_the_guard_check_refuses_missing_changed_and_writable_files(self):
        me = os.getuid()
        with tempfile.TemporaryDirectory() as scratch:
            guards = Path(scratch) / "guards"
            guards.mkdir(mode=0o755)
            hook, settings = guards / "bash-guard", guards / "settings.json"
            hook.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            settings.write_text('{"hooks": {}}\n', encoding="utf-8")
            hook.chmod(0o755)
            settings.chmod(0o644)
            manifest = Path(scratch) / "guards.manifest"
            matching = [entry(hook, 0o755), entry(settings, 0o644)]
            write_manifest(manifest, matching)

            # A matching manifest passes, and the check says how many files it examined.
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 0, out)
            self.assertIn("examined 2 file(s)", out)
            self.assertNotIn("REFUSE", out)

            # The command line keeps root's uid, so a file the test user owns is refused there.
            if me != 0:
                done = run(GUARDS, manifest)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn(f"REFUSE: {hook}: owned by uid {me}, not root", done.stdout)

            # A group-writable guard is refused, even where the manifest records its mode.
            settings.chmod(0o664)
            write_manifest(manifest, [matching[0], entry(settings, 0o664)])
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {settings}: writable by its group", out)
            settings.chmod(0o646)
            write_manifest(manifest, [matching[0], entry(settings, 0o646)])
            code, out = judge_guards(manifest, me)
            self.assertIn(f"REFUSE: {settings}: writable by others", out)
            settings.chmod(0o644)

            # A mode other than the manifest's is refused.
            write_manifest(manifest, matching)
            hook.chmod(0o700)
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {hook}: mode 0700, the manifest records 0755", out)
            hook.chmod(0o755)

            # A file another uid owns is refused.
            code, out = judge_guards(manifest, me + 1)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {hook}: owned by uid {me}, not root", out)

            # A changed guard is refused by its digest, and its content is never printed.
            hook.write_text("#!/bin/sh\necho changed-guard-body\n", encoding="utf-8")
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {hook}: its SHA-256 is not the manifest's", out)
            self.assertNotIn("changed-guard-body", out)

            # A missing guard is refused by its path.
            hook.unlink()
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {hook}: missing", out)
            self.assertIn("examined 2 file(s)", out)

            # A manifest anyone but root could rewrite is refused before its entries are read.
            write_manifest(manifest, [entry(settings, 0o644)])
            manifest.chmod(0o664)
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {manifest}: writable by its group", out)

            # A manifest that names no file judged nothing, and is refused.
            write_manifest(manifest, [])
            code, out = judge_guards(manifest, me)
            self.assertEqual(code, 1, out)
            self.assertIn(f"REFUSE: {manifest}: names no file", out)
            self.assertIn("examined 0 file(s)", out)


class NoPrivateValue(unittest.TestCase):
    def test_no_rail_contract_file_names_a_private_value(self):
        files = examined(
            "rail-contract file(s)", [path for path in CONTRACT_FILES if path.is_file()]
        )
        self.assertEqual(files, list(CONTRACT_FILES))
        done = scrub(*files)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(f"examined {len(files)} file(s)", done.stdout)
        # A planted private value, an address and a secret's resource path, is refused.
        address = ".".join(str(octet) for octet in (10, 24, 36, 48))
        resource = "/".join(("projects", "planted-proj-42", "secrets", "planted-name", "versions"))
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "rail-contract.json"
            text = CONTRACT.read_text(encoding="utf-8")
            planted.write_text(text + f"\n{address}\n{resource}/latest\n", encoding="utf-8")
            done = scrub(planted)
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        lines = len(text.splitlines())
        self.assertIn(f"rail-contract.json:{lines + 2}: ipv4", done.stdout)
        self.assertIn(f"rail-contract.json:{lines + 3}:", done.stdout)
        self.assertNotIn(address, done.stdout)


if __name__ == "__main__":
    unittest.main()

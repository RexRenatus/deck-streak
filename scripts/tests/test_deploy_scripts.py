"""The deploy and the rollback run from the maintainer's machine against a synthetic world
(SPEC-062 R3 to R7, R14; ADR-062, ADR-010, ADR-017): a repository with its own tags on a local
origin, a synthetic release (a tarball and its `SHA256SUMS`), a stub `gh` whose attestation
verdict each test chooses, and a stub host command that applies the host side inside a temporary
directory with stub `systemctl`, `curl` and `caddy`. Nothing reaches a network or a host.

    deploy/deploy.sh TAG          verify, then install and switch
    deploy/rollback.sh TAG        make a kept release current again, or deploy it anew
    deploy/deploy.sh caddy-install | caddy-remove   the Caddy block (R7)

The scripts are configured by `DECKSTREAK_DEPLOY_*` variables so a test can point them at its own
tree; on the maintainer's machine only the host command and the repository are set."""

import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

DEPLOY = REPO / "deploy" / "deploy.sh"
ROLLBACK = REPO / "deploy" / "rollback.sh"
RENDER = REPO / "deploy" / "scripts" / "render-caddy.py"
SLUG = "owner/synthetic"
API = "deck-streak-api.service"
BOT = "deck-streak-bot.service"
JOB = "deck-streak-job"
# No shipped deploy file names a path under /opt.
PRIVATE_PATHS = "/opt/"
SYSTEMD = "deploy/systemd"

GH = r"""#!/bin/bash
echo "gh $*" >> "$STUB_LOG/gh.log"
case "$1 $2" in
"release download")
    tag=$3; dir=
    while [ $# -gt 0 ]; do [ "$1" = --dir ] && dir=$2; shift; done
    [ -d "$STUB_ASSETS/$tag" ] || { echo "release not found" >&2; exit 1; }
    cp "$STUB_ASSETS/$tag"/* "$dir"/ ;;
"attestation verify")
    [ -f "$3" ] || exit 3
    [ "$(cat "$STUB_LOG/attest")" = ok ] ;;
*) exit 2 ;;
esac
"""
SYSTEMCTL = r"""#!/bin/bash
echo "systemctl $*" >> "$STUB_LOG/systemctl.log"
cur=$(basename "$(readlink "$STUB_ROOT/current" 2>/dev/null)")
case "$1" in
daemon-reload | stop | start | enable | is-active) exit 0 ;;
restart)
    echo "restart $2 current=$cur" >> "$STUB_LOG/events.log"
    [ "$2" = "$STUB_BAD_UNIT" ] && [ "$cur" = "$STUB_BAD_TAG" ] && exit 1
    exit 0 ;;
cat)
    shift
    for u in "$@"; do
        [ "$u" = "$STUB_CAT_FAIL" ] && { echo "Failed to cat $u" >&2; exit 1; }
        f="$STUB_UNIT_DIR/$u"
        base=$f
        case "$u" in *@*.service) [ -f "$f" ] || base="$STUB_UNIT_DIR/${u%%@*}@.service" ;; esac
        [ -f "$base" ] || { echo "No files found for $u." >&2; exit 1; }
        echo "# $base"; cat "$base"; echo
        for d in "$base.d" "$f.d"; do
            for c in "$d"/*.conf; do [ -f "$c" ] || continue; echo "# $c"; cat "$c"; echo; done
        done
    done ;;
*) exit 2 ;;
esac
"""
CURL = r"""#!/bin/bash
cur=$(basename "$(readlink "$STUB_ROOT/current" 2>/dev/null)")
echo "probe $* current=$cur releases=$(ls "$STUB_ROOT/releases" | tr '\n' ,)" >> "$STUB_LOG/probes.log"
[ "$STUB_BAD_UNIT" = deck-streak-api.service ] && [ "$cur" = "$STUB_BAD_TAG" ] && exit 22
exit 0
"""
# A release whose restart succeeds and whose readiness never arrives.
UNREADY_CURL = CURL.replace(
    "exit 0\n",
    '[ -n "$STUB_UNREADY_TAG" ] && [ "$cur" = "$STUB_UNREADY_TAG" ] && exit 7\nexit 0\n',
)
# Caddy reads `validate` as JSON unless the file is named like a Caddyfile or the adapter is given.
CADDY = r"""#!/bin/bash
all="$*"; first=$1; conf=; adapter=
while [ $# -gt 0 ]; do
    [ "$1" = --config ] && conf=$2
    [ "$1" = --adapter ] && adapter=$2
    shift
done
echo "caddy $all imports=$(grep -c import "$conf" 2>/dev/null)" >> "$STUB_LOG/caddy.log"
if [ "$first" = validate ] && [ "$adapter" != caddyfile ]; then
    name=$(basename "$conf")
    case "$name" in Caddyfile* | *.caddyfile) ;; *) echo "invalid character: not JSON" >&2; exit 1 ;; esac
fi
[ "$first" = validate ] && [ -f "$STUB_LOG/caddy-refuses" ] && exit 1
[ "$first" = adapt ] && [ -f "$STUB_LOG/caddy-adapt-refuses" ] && exit 1
if [ "$first" = reload ]; then
    # the previous copies a reload can see, and a reload that fails on demand: the file holds how many
    echo "$(ls -1 "$(dirname "$conf")" | grep -c '\.previous$')" >> "$STUB_LOG/reload-sees.log"
    if [ -f "$STUB_LOG/caddy-reload-fails" ]; then
        left=$(cat "$STUB_LOG/caddy-reload-fails")
        if [ "$left" -le 1 ]; then find "$STUB_LOG/caddy-reload-fails" -delete; else echo $((left - 1)) > "$STUB_LOG/caddy-reload-fails"; fi
        echo "reload refused" >&2
        exit 1
    fi
fi
exit 0
"""
LOGGED = r"""#!/bin/bash
echo "@NAME@ $*" >> "$STUB_LOG/moves.log"
exec /usr/bin/@NAME@ "$@"
"""
# A `mv` that fails when it renames a `@SUFFIX@` file onto the Caddyfile: a rename that goes wrong.
MOVE_FAILS = r"""#!/bin/bash
src=${@: -2:1}; dst=${@: -1}
if [ "$dst" = "$DECKSTREAK_DEPLOY_CADDYFILE" ] && [ "${src%@SUFFIX@}" != "$src" ]; then
    echo "mv $* refused-rename" >> "$STUB_LOG/moves.log"
    exit 1
fi
echo "mv $*" >> "$STUB_LOG/moves.log"
exec /usr/bin/mv "$@"
"""
# A `mv` that fails when it renames a `.previous` file onto the site block: a restore that goes wrong.
MOVE_ONTO_BLOCK_FAILS = r"""#!/bin/bash
src=${@: -2:1}; dst=${@: -1}
if [ "$dst" = "$(dirname "$DECKSTREAK_DEPLOY_CADDYFILE")/deck-streak.caddy" ] && [ "${src%.previous}" != "$src" ]; then
    echo "mv $* refused-rename" >> "$STUB_LOG/moves.log"
    exit 1
fi
echo "mv $*" >> "$STUB_LOG/moves.log"
exec /usr/bin/mv "$@"
"""
HOST = r"""#!/bin/bash
echo "host" >> "$STUB_LOG/host.log"
exec "$@"
"""
CONTRACT = {
    "schema": "deckstreak.rail-contract.v1",
    "note": "synthetic",
    "neutral": {
        "release_root": "/usr/local/lib/deck-streak/current",
        "environment_file": "/etc/deck-streak/deck-streak.env",
        "time_zone": "UTC",
        "rollover_hour": 4,
    },
    "drop_in": "10-rail.conf",
    "socket": "/run/deck-streak-credentials/socket",
    "values": [
        {
            "unit": "deck-streak-alert@.service",
            "section": "Service",
            "key": "ExecStart",
            "neutral": ["/usr/local/lib/deck-streak/current/deploy/scripts/alert-telegram.sh %i"],
        }
    ],
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def unit_text(name, marker):
    body = "Type=notify\n" if name in (API, BOT) else "Type=oneshot\n"
    return (
        f"[Unit]\nDescription={name} {marker}\n\n[Service]\n{body}"
        f"ExecStart=/srv/synthetic/bin/deckstreakd run\n"
    ).encode()


def release_files(tag, marker):
    """The synthetic release's files by path: what the workflow's tarball holds."""
    files = {
        "bin/deckstreakd": f"#!/bin/sh\necho {tag}\n".encode(),
        "web/index.html": f"<title>{tag}</title>\n".encode(),
        "agent/roster.example.json": b"{}\n",
        "ai-safety.json": b"{}\n",
        "deploy/rail-contract.json": json.dumps(CONTRACT).encode(),
        "deploy/scripts/effective-check.py": (
            REPO / "deploy/scripts/effective-check.py"
        ).read_bytes(),
        f"{SYSTEMD}/{API}": unit_text(API, marker),
        f"{SYSTEMD}/{BOT}": unit_text(BOT, marker),
        f"{SYSTEMD}/deck-streak-job@.service": unit_text("deck-streak-job@.service", marker),
        f"{SYSTEMD}/{JOB}@sync.service.d/20-sync-login.conf": (
            b"[Service]\nLoadCredential=anki-sync-username:/run/deck-streak-credentials/socket\n"
        ),
    }
    lines = "".join(f"{sha(data)}  {path}\n" for path, data in sorted(files.items()))
    files["MANIFEST.sha256"] = lines.encode()
    return files


def unit_files(tag, marker):
    prefix = SYSTEMD + "/"
    return {
        path[len(prefix) :]: data
        for path, data in release_files(tag, marker).items()
        if path.startswith(prefix)
    }


class World:
    """A synthetic repository, release assets and host, under one temporary directory."""

    def __init__(self, tmp):
        self.tmp = Path(tmp)
        self.stub = self.tmp / "stub"
        self.log = self.tmp / "log"
        self.assets = self.tmp / "assets"
        self.root = self.tmp / "host" / "usr" / "local" / "lib" / "deck-streak"
        self.units = self.tmp / "host" / "etc" / "systemd" / "system"
        self.envfile = self.tmp / "host" / "etc" / "deck-streak" / "deck-streak.env"
        self.caddy_dir = self.tmp / "host" / "etc" / "caddy"
        for directory in (self.stub / "bin", self.log, self.assets, self.units, self.caddy_dir):
            directory.mkdir(parents=True)
        self.envfile.parent.mkdir(parents=True)
        self.envfile.write_text("DECKSTREAK_API_LISTEN=127.0.0.1:8080\n", encoding="utf-8")
        (self.log / "attest").write_text("ok", encoding="utf-8")
        for name, body in (("gh", GH), ("systemctl", SYSTEMCTL), ("curl", CURL), ("caddy", CADDY)):
            self.script(name, body)
        for name in ("ln", "mv", "rm"):
            self.script(name, LOGGED.replace("@NAME@", name))
        self.script("host", HOST)
        self.env = {
            **os.environ,
            "PATH": f"{self.stub / 'bin'}:{os.environ['PATH']}",
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_CONFIG_SYSTEM": "/dev/null",
            "GIT_AUTHOR_NAME": "t",
            "GIT_AUTHOR_EMAIL": "t@example.org",
            "GIT_COMMITTER_NAME": "t",
            "GIT_COMMITTER_EMAIL": "t@example.org",
            "STUB_LOG": str(self.log),
            "STUB_ASSETS": str(self.assets),
            "STUB_ROOT": str(self.root),
            "STUB_UNIT_DIR": str(self.units),
            "STUB_BAD_UNIT": "",
            "STUB_BAD_TAG": "",
            "STUB_CAT_FAIL": "",
            "DECKSTREAK_DEPLOY_REPO": SLUG,
            "DECKSTREAK_DEPLOY_HOST": str(self.stub / "bin" / "host"),
            "DECKSTREAK_DEPLOY_ELEVATE": "",
            "DECKSTREAK_DEPLOY_ROOT": str(self.root),
            "DECKSTREAK_DEPLOY_UNIT_DIR": str(self.units),
            "DECKSTREAK_DEPLOY_ENV_FILE": str(self.envfile),
            "DECKSTREAK_DEPLOY_READY_SECONDS": "2",
            "DECKSTREAK_DEPLOY_READY_POLL": "1",
            "DECKSTREAK_DEPLOY_CADDY_DIR": str(self.caddy_dir),
            "DECKSTREAK_DEPLOY_CADDYFILE": str(self.caddy_dir / "Caddyfile"),
        }
        self.origin = self.tmp / "origin.git"
        self.git("init", "--bare", "-b", "main", str(self.origin), cwd=self.tmp)
        self.other = self.tmp / "other"
        self.git("clone", "-q", str(self.origin), str(self.other), cwd=self.tmp)
        self.git("checkout", "-q", "-b", "main", cwd=self.other)
        self.commit("first")
        self.git("push", "-q", "origin", "main", cwd=self.other)
        self.checkout = self.tmp / "checkout"
        self.git("clone", "-q", str(self.origin), str(self.checkout), cwd=self.tmp)
        self.env["DECKSTREAK_DEPLOY_CHECKOUT"] = str(self.checkout)
        self.n = 0

    def script(self, name, body):
        path = self.stub / "bin" / name
        path.write_text(body, encoding="utf-8")
        path.chmod(0o755)

    def git(self, *args, cwd):
        done = subprocess.run(
            ["git", *args], cwd=cwd, env=self.env, capture_output=True, text=True, check=False
        )
        assert done.returncode == 0, f"git {args}: {done.stderr}"
        return done.stdout.strip()

    def commit(self, message):
        self.n = getattr(self, "n", 0) + 1
        (self.other / "f.txt").write_text(f"{message} {self.n}\n", encoding="utf-8")
        block = self.other / "deploy" / "caddy" / "deck-streak.caddy"
        block.parent.mkdir(parents=True, exist_ok=True)
        block.write_bytes((REPO / "deploy" / "caddy" / "deck-streak.caddy").read_bytes())
        self.git("add", "-A", cwd=self.other)
        self.git("commit", "-q", "-m", message, cwd=self.other)

    def tag_on_main(self, tag, annotated=True):
        """A tag on a new commit of origin's main, made in a clone the checkout has not fetched."""
        self.git("checkout", "-q", "main", cwd=self.other)
        self.commit(tag)
        self.git("push", "-q", "origin", "main", cwd=self.other)
        self.mark(tag, annotated)

    def tag_off_main(self, tag, annotated=True):
        self.git("checkout", "-q", "-B", f"topic-{tag}", "main", cwd=self.other)
        self.commit(tag)
        self.git("push", "-q", "origin", f"topic-{tag}", cwd=self.other)
        self.mark(tag, annotated)
        self.git("checkout", "-q", "main", cwd=self.other)

    def mark(self, tag, annotated):
        if annotated:
            self.git("tag", "-a", "-m", tag, tag, cwd=self.other)
        else:
            self.git("tag", tag, cwd=self.other)
        self.git("push", "-q", "origin", tag, cwd=self.other)

    def publish(self, tag, marker=None):
        """The release's assets, as the release workflow would attach them."""
        files = release_files(tag, marker or tag)
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
            for path, data in sorted(files.items()):
                member = tarfile.TarInfo(path)
                member.size = len(data)
                member.mode = 0o755 if path.startswith("bin/") else 0o644
                archive.addfile(member, io.BytesIO(data))
        tarball = buffer.getvalue()
        directory = self.assets / tag
        directory.mkdir(parents=True)
        name = f"deck-streak-{tag}.tar.gz"
        (directory / name).write_bytes(tarball)
        (directory / "SHA256SUMS").write_text(f"{sha(tarball)}  {name}\n", encoding="utf-8")

    def ship(self, tag, marker=None):
        self.tag_on_main(tag)
        self.publish(tag, marker)

    def run(self, script, *args, **env):
        return subprocess.run(
            ["bash", str(script), *args],
            cwd=self.tmp,
            env={**self.env, **env},
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
        )

    def deploy(self, tag, **env):
        assert DEPLOY.is_file(), f"{DEPLOY.relative_to(REPO)} does not exist"
        return self.run(DEPLOY, tag, **env)

    def rollback(self, tag, **env):
        assert ROLLBACK.is_file(), f"{ROLLBACK.relative_to(REPO)} does not exist"
        return self.run(ROLLBACK, tag, **env)

    def current(self):
        link = self.root / "current"
        return Path(os.readlink(link)).name if link.is_symlink() else None

    def releases(self):
        directory = self.root / "releases"
        return sorted(p.name for p in directory.iterdir()) if directory.is_dir() else []

    def text(self, name):
        path = self.log / name
        return path.read_text(encoding="utf-8") if path.exists() else ""

    def untouched(self):
        return not self.text("host.log") and not self.root.exists()


class Case(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.world = World(self._tmp.name)

    def ok(self, done):
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        return done


class TheDeployVerifiesBeforeTheHost(Case):
    def test_the_deploy_refuses_a_tag_off_main_and_a_lightweight_tag(self):
        w = self.world
        w.tag_off_main("v1.0.0")
        w.publish("v1.0.0")
        w.tag_on_main("v1.1.0", annotated=False)
        w.publish("v1.1.0")
        for tag, why in (("v1.0.0", "main"), ("v1.1.0", "annotated")):
            done = w.deploy(tag)
            self.assertNotEqual(done.returncode, 0, f"{tag} was deployed")
            self.assertIn(why, done.stderr, f"{tag}: {done.stderr}")
            self.assertEqual(w.text("gh.log"), "", "the release was fetched before the tag passed")
            self.assertTrue(w.untouched(), f"{tag} reached the host")
        for tag in ("v1.2", "v1.2.3.4", "latest", "v01.2.3", "v1.2.3-rc1", "1.2.3"):
            done = w.deploy(tag)
            self.assertNotEqual(done.returncode, 0, f"{tag} is not a SemVer tag")
            self.assertEqual(w.text("gh.log"), "")
        done = w.deploy("v9.9.9")
        self.assertNotEqual(done.returncode, 0, "a tag that does not exist")
        self.assertTrue(w.untouched())
        w.ship("v2.0.0")
        self.ok(w.deploy("v2.0.0"))
        self.assertEqual(w.current(), "v2.0.0", "a fresh tag on main, not yet fetched, deploys")

    def test_the_deploy_refuses_a_bad_digest_or_attestation_before_the_host(self):
        w = self.world
        w.ship("v1.0.0")
        name = "deck-streak-v1.0.0.tar.gz"
        tarball = w.assets / "v1.0.0" / name
        good = tarball.read_bytes()
        tarball.write_bytes(good + b"x")
        done = w.deploy("v1.0.0")
        self.assertNotEqual(done.returncode, 0, "a tarball that differs from SHA256SUMS")
        self.assertIn("SHA256SUMS", done.stderr)
        self.assertTrue(w.untouched(), "a bad digest reached the host")
        tarball.write_bytes(good)
        (w.log / "attest").write_text("fail", encoding="utf-8")
        done = w.deploy("v1.0.0")
        self.assertNotEqual(done.returncode, 0, "an attestation that does not verify")
        self.assertIn("attestation", done.stderr)
        self.assertTrue(w.untouched(), "an unverified attestation reached the host")
        verify = [ln for ln in w.text("gh.log").splitlines() if "attestation verify" in ln]
        self.assertTrue(verify, "the attestation was never verified")
        self.assertIn(f"--repo {SLUG}", verify[0])
        self.assertIn(f"--signer-workflow {SLUG}/.github/workflows/release.yml", verify[0])
        self.assertIn(name, verify[0])
        (w.log / "attest").write_text("ok", encoding="utf-8")
        (w.assets / "v1.0.0" / "SHA256SUMS").write_text(f"{sha(b'y')}  other.txt\n")
        done = w.deploy("v1.0.0")
        self.assertNotEqual(done.returncode, 0, "SHA256SUMS that does not name the tarball")
        self.assertTrue(w.untouched())
        (w.assets / "v1.0.0" / "SHA256SUMS").write_text(f"{sha(good)}  {name}\n")
        self.ok(w.deploy("v1.0.0"))
        self.assertEqual(w.current(), "v1.0.0", "the control: the same assets, verified, deploy")


class TheDeploySwitches(Case):
    def test_the_deploy_installs_beside_and_switches_current_atomically(self):
        w = self.world
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0"))
        self.assertEqual(w.releases(), ["v1.0.0"], "no .partial directory is left")
        link = w.root / "current"
        self.assertTrue(link.is_symlink())
        self.assertEqual(w.current(), "v1.0.0")
        self.assertEqual(
            (w.root / "releases/v1.0.0/bin/deckstreakd").read_text().split()[-1], "v1.0.0"
        )
        moves = w.text("moves.log").splitlines()
        temporary = [m for m in moves if m.startswith("ln ") and "-sfn" in m]
        self.assertTrue(temporary, "current is linked under a temporary name")
        self.assertNotIn(f"{link}\n", "".join(m.split()[-1] + "\n" for m in temporary))
        switch = [m for m in moves if m.startswith("mv ") and m.endswith(str(link))]
        self.assertEqual(len(switch), 1, moves)
        self.assertIn("-T", switch[0].split(), "current is replaced by mv -T")
        partial = [m for m in moves if m.startswith("mv ") and ".partial" in m]
        self.assertTrue(partial, "the release is unpacked as .partial and renamed")
        for name, data in examined("unit files", unit_files("v1.0.0", "v1.0.0").items()):
            self.assertEqual((w.units / name).read_bytes(), data, f"{name} is not byte for byte")
        events = w.text("events.log").splitlines()
        self.assertEqual(
            [e.split()[1] for e in events], [API, BOT], "the long-running units restart"
        )
        self.assertTrue(all(e.endswith("current=v1.0.0") for e in events), events)
        calls = w.text("systemctl.log")
        self.assertLess(calls.index("daemon-reload"), calls.index("restart"))
        for unit in (API, BOT, "deck-streak-job@.service"):
            self.assertIn(f"cat {unit}", calls.replace("\n", " ").replace("cat ", "cat "))
        self.assertIn("http://127.0.0.1:8080/api/readyz", w.text("probes.log"))
        w.ship("v1.1.0", marker="second")
        self.ok(w.deploy("v1.1.0"))
        self.assertEqual(w.current(), "v1.1.0")
        self.assertEqual(w.releases(), ["v1.0.0", "v1.1.0"], "the old release stays beside it")
        self.assertIn(b"second", (w.units / API).read_bytes())

    def test_the_deploy_switches_back_when_readiness_does_not_arrive(self):
        w = self.world
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0"))
        w.ship("v2.0.0", marker="bad")
        for unit in (BOT, API):
            done = w.deploy("v2.0.0", STUB_BAD_UNIT=unit, STUB_BAD_TAG="v2.0.0")
            self.assertNotEqual(done.returncode, 0, f"{unit} never became ready")
            self.assertIn(unit, done.stderr, "the refusal names the unit that did not become ready")
            self.assertEqual(w.current(), "v1.0.0", "current is switched back")
            for name, data in examined("unit files", unit_files("v1.0.0", "v1.0.0").items()):
                self.assertEqual((w.units / name).read_bytes(), data, f"{name} is not v1.0.0's")
            last = w.text("events.log").splitlines()[-1]
            self.assertTrue(last.endswith("current=v1.0.0"), f"restarted after the switch: {last}")

    def test_a_restart_that_succeeds_and_readiness_that_never_arrives_switches_back(self):
        w = self.world
        w.script("curl", UNREADY_CURL)
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0", STUB_UNREADY_TAG=""))
        w.ship("v2.0.0", marker="unready")
        done = w.deploy("v2.0.0", STUB_UNREADY_TAG="v2.0.0")
        restarts = w.text("events.log").splitlines()
        probes = [p for p in w.text("probes.log").splitlines() if "current=v2.0.0" in p]
        self.assertTrue(
            any(r == f"restart {API} current=v2.0.0" for r in restarts), "the restart succeeded"
        )
        self.assertGreaterEqual(len(probes), 2, "readiness was polled on the new release")
        self.assertNotEqual(done.returncode, 0, "a release that never became ready was kept")
        self.assertIn(API, done.stderr)
        self.assertEqual(w.current(), "v1.0.0", "current is switched back")
        self.assertTrue(restarts[-1].endswith("current=v1.0.0"), restarts[-1])

    def test_the_deploy_leaves_no_current_when_the_first_release_is_not_ready(self):
        w = self.world
        w.ship("v1.0.0")
        done = w.deploy("v1.0.0", STUB_BAD_UNIT=BOT, STUB_BAD_TAG="v1.0.0")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn(BOT, done.stderr)
        self.assertIsNone(w.current(), "no release is current when none was ready")

    def test_the_rollback_makes_a_kept_release_current_again(self):
        w = self.world
        w.ship("v1.0.0", marker="one")
        w.ship("v1.1.0", marker="two")
        self.ok(w.deploy("v1.0.0"))
        self.ok(w.deploy("v1.1.0"))
        gh_before = w.text("gh.log")
        self.ok(w.rollback("v1.0.0"))
        self.assertEqual(w.current(), "v1.0.0")
        self.assertEqual(w.text("gh.log"), gh_before, "a kept release is not fetched again")
        for name, data in examined("unit files", unit_files("v1.0.0", "one").items()):
            self.assertEqual((w.units / name).read_bytes(), data, f"{name} is not v1.0.0's own")
        self.assertTrue(w.text("events.log").splitlines()[-1].endswith("current=v1.0.0"))
        switches = [m for m in w.text("moves.log").splitlines() if m.startswith("mv ")]
        self.assertIn("-T", switches[-1].split(), "the rollback switches with mv -T too")
        self.assertTrue(switches[-1].endswith(str(w.root / "current")))
        w.ship("v0.9.0", marker="old")
        self.ok(w.rollback("v0.9.0"))
        self.assertEqual(w.current(), "v0.9.0", "a release the host lacks is deployed anew")
        self.assertIn("attestation verify", w.text("gh.log"), "and through the verification")
        w.tag_off_main("v0.8.0")
        w.publish("v0.8.0")
        done = w.rollback("v0.8.0")
        self.assertNotEqual(done.returncode, 0, "a tag off main is not rolled to")
        self.assertEqual(w.current(), "v0.9.0")

    def test_the_deploy_keeps_three_releases_and_prunes_after_a_ready_switch(self):
        w = self.world
        tags = [f"v1.{n}.0" for n in range(5)]
        for tag in tags:
            w.ship(tag)
            self.ok(w.deploy(tag))
        self.assertEqual(w.releases(), tags[2:], "the current release and the two before it")
        probes = w.text("probes.log").splitlines()
        self.assertIn("releases=v1.1.0,v1.2.0,v1.3.0,v1.4.0,", probes[-1], "no prune before ready")
        w.ship("v1.5.0")
        done = w.deploy("v1.5.0", STUB_BAD_UNIT=API, STUB_BAD_TAG="v1.5.0")
        self.assertNotEqual(done.returncode, 0)
        self.assertEqual(w.current(), "v1.4.0")
        kept = set(w.releases())
        self.assertLessEqual(set(tags[2:]), kept, "a failed switch prunes nothing")


class PruneAfterARollback(Case):
    def test_the_release_a_deploy_replaced_is_never_pruned(self):
        w = self.world
        for tag in ("v1.0.0", "v1.1.0", "v1.2.0"):
            w.ship(tag)
            self.ok(w.deploy(tag))
        self.ok(w.rollback("v1.0.0"))
        self.assertEqual(w.current(), "v1.0.0")
        w.ship("v1.0.1")
        self.ok(w.deploy("v1.0.1"))
        self.assertEqual(w.current(), "v1.0.1")
        self.assertIn("v1.0.0", w.releases(), f"the release it replaced was pruned: {w.releases()}")


class AnotherUnitsDropInsAreLeftAlone(Case):
    FOREIGN = {
        "getty@tty1.service.d/autologin.conf": b"[Service]\nExecStart=\n",
        "other-app@tty1.service.d/override.conf": b"[Service]\nMemoryMax=1G\n",
        "user@.service.d/delegate.conf": b"[Service]\nDelegate=yes\n",
    }

    def intact(self, when):
        for rel, data in examined("foreign drop-ins", self.FOREIGN.items()):
            path = self.world.units / rel
            self.assertTrue(path.is_file(), f"{rel} was deleted by {when}")
            self.assertEqual(path.read_bytes(), data, f"{rel} changed by {when}")

    def test_a_deploy_a_rollback_and_a_switch_back_leave_them_byte_for_byte(self):
        w = self.world
        for rel, data in self.FOREIGN.items():
            (w.units / rel).parent.mkdir(parents=True, exist_ok=True)
            (w.units / rel).write_bytes(data)
        w.ship("v1.0.0")
        w.ship("v1.1.0", marker="two")
        w.ship("v2.0.0", marker="bad")
        self.ok(w.deploy("v1.0.0"))
        self.intact("a deploy")
        self.ok(w.deploy("v1.1.0"))
        self.ok(w.rollback("v1.0.0"))
        self.intact("a rollback")
        done = w.deploy("v2.0.0", STUB_BAD_UNIT=API, STUB_BAD_TAG="v2.0.0")
        self.assertNotEqual(done.returncode, 0)
        self.intact("a switch back")
        self.assertTrue(all((w.units / rel).is_file() for rel in self.FOREIGN))


class APartialEffectiveConfigurationIsRefused(Case):
    def test_a_unit_that_cannot_be_shown_refuses_the_deploy_and_names_it(self):
        w = self.world
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0"))
        w.ship("v1.1.0", marker="two")
        done = w.deploy("v1.1.0", STUB_CAT_FAIL=API)
        self.assertNotEqual(done.returncode, 0, "a deploy whose effective view was partial")
        self.assertIn(f"deploy: {API} could not be shown", done.stderr, "the deploy names the unit")
        self.assertEqual(w.current(), "v1.0.0", "current is unchanged")


class TheCaddyInstall(Case):
    def config(self, **extra):
        path = self.world.tmp / "caddy-config.json"
        data = {
            "host": "app.example.org",
            "web_root": str(self.world.root / "current/web"),
            "api_upstream": "127.0.0.1:8080",
            **extra,
        }
        path.write_text(json.dumps(data), encoding="utf-8")
        return {"DECKSTREAK_DEPLOY_CADDY_CONFIG": str(path)}

    def test_the_caddy_install_validates_a_copy_first_and_the_removal_reverses_it(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0"))
        assert DEPLOY.is_file()
        self.ok(w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config()))
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertTrue(block.is_file())
        self.assertNotIn("{$", block.read_text())
        text = caddyfile.read_text()
        self.assertTrue(text.startswith(original), "nothing already in the Caddyfile changes")
        self.assertEqual(text[len(original) :].count("import"), 1, "one import line is added")
        log = w.text("caddy.log").splitlines()
        validate = [ln for ln in log if ln.startswith("caddy validate")]
        self.assertTrue(validate, log)
        self.assertNotIn(str(caddyfile), validate[0], "the copy is validated, not the live file")
        self.assertIn("imports=1", validate[0], "the copy already holds the import line")
        self.assertTrue(any(ln.startswith("caddy adapt") and "--validate" in ln for ln in log), log)
        self.assertTrue(any(ln.startswith("caddy reload") for ln in log), log)
        self.assertLess(
            next(i for i, ln in enumerate(log) if ln.startswith("caddy validate")),
            next(i for i, ln in enumerate(log) if ln.startswith("caddy reload")),
        )
        self.ok(w.run(ROLLBACK, "caddy-remove", **self.config()))
        self.assertEqual(caddyfile.read_text(), original, "the removal restores the Caddyfile")
        self.assertFalse(block.exists(), "the rendered file is removed")
        (w.log / "caddy-refuses").write_text("x")
        before = caddyfile.read_text()
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertNotEqual(done.returncode, 0, "a configuration that does not validate")
        self.assertEqual(caddyfile.read_text(), before, "the live Caddyfile is untouched")
        self.assertFalse(block.exists())
        reloads = [ln for ln in w.text("caddy.log").splitlines() if ln.startswith("caddy reload")]
        self.assertEqual(len(reloads), 2, "no reload after a refused validation")
        done = w.run(
            DEPLOY, "caddy-install", "v1.0.0", DECKSTREAK_DEPLOY_CADDY_CONFIG="/nonexistent"
        )
        self.assertNotEqual(done.returncode, 0, "no private configuration")

    def installed(self, host="app.example.org"):
        """A Caddyfile and a first successful install; returns (original, after, block text)."""
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        w.ship("v1.0.0")
        self.ok(w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config(host=host)))
        block = w.caddy_dir / "deck-streak.caddy"
        return original, caddyfile.read_text(), block.read_text()

    def leftovers(self):
        return sorted(p.name for p in self.world.caddy_dir.iterdir() if "previous" in p.name)

    def test_a_failed_reload_restores_the_previous_block_and_caddyfile(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / "caddy-reload-fails").write_text("1")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config(host="new.example.org"))
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn("the Caddy reload failed", done.stderr, "the message names the failed reload")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        self.assertEqual(self.leftovers(), [], "no previous copy is left behind")
        reloads = [ln for ln in w.text("caddy.log").splitlines() if ln.startswith("caddy reload")]
        self.assertEqual(len(reloads), 3, "the restored configuration is reloaded")

    def test_a_failed_restoring_reload_is_named_apart_and_still_refuses(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / "caddy-reload-fails").write_text("2")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config(host="new.example.org"))
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("restoring reload", done.stderr, "a second failure is named distinctly")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)

    def test_a_first_install_whose_reload_fails_removes_the_new_block(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        w.ship("v1.0.0")
        (w.log / "caddy-reload-fails").write_text("1")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertNotEqual(done.returncode, 0)
        self.assertFalse((w.caddy_dir / "deck-streak.caddy").exists(), "the new block is removed")
        self.assertEqual(caddyfile.read_text(), original, "the previous Caddyfile is back")
        self.assertEqual(self.leftovers(), [])

    def test_the_previous_copies_outlive_the_reload_and_go_after_a_good_one(self):
        w = self.world
        self.installed()
        first = w.text("reload-sees.log").split()
        self.assertEqual(first, ["1"], "a first install keeps the Caddyfile until the reload")
        self.ok(w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config(host="new.example.org")))
        seen = w.text("reload-sees.log").split()
        self.assertEqual(seen[-1], "2", "the block and the Caddyfile are both kept at the reload")
        self.assertEqual(self.leftovers(), [], "a good reload removes both copies")

    def test_a_failed_reload_of_the_removal_restores_the_block_and_caddyfile(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / "caddy-reload-fails").write_text("1")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("the Caddy reload failed", done.stderr)
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        self.assertEqual(self.leftovers(), [])
        self.ok(w.run(ROLLBACK, "caddy-remove", **self.config()))
        self.assertFalse((w.caddy_dir / "deck-streak.caddy").exists())
        self.assertEqual(self.leftovers(), [])

    def test_a_failed_restoring_reload_of_the_removal_is_named_apart(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / "caddy-reload-fails").write_text("2")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("restoring reload", done.stderr, "a second failure is named distinctly")
        reloads = [ln for ln in w.text("caddy.log").splitlines() if ln.startswith("caddy reload")]
        self.assertEqual(len(reloads), 3, "the install's reload, the removal's, its restoring one")
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)

    def failing_rename(self, source_suffix):
        """A `mv` that refuses to rename a `source_suffix` file onto the Caddyfile, and logs it."""
        body = MOVE_FAILS.replace("@SUFFIX@", source_suffix)
        self.world.script("mv", body)

    def imports_a_missing_block(self):
        w = self.world
        live = (w.caddy_dir / "Caddyfile").read_text()
        return (
            "import deck-streak.caddy" in live and not (w.caddy_dir / "deck-streak.caddy").exists()
        )

    def test_a_removal_whose_caddyfile_rename_fails_leaves_the_block_in_place(self):
        w = self.world
        _original, after, block_text = self.installed()
        self.failing_rename(".candidate")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a failed rename refuses the removal")
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        self.assertFalse(self.imports_a_missing_block(), "the live Caddyfile imports a block")

    def test_a_first_install_whose_restore_rename_fails_never_imports_a_missing_block(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        caddyfile.write_text("example.org {\n\trespond 200\n}\n", encoding="utf-8")
        w.ship("v1.0.0")
        (w.log / "caddy-reload-fails").write_text("1")
        self.failing_rename(".previous")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("refused-rename", w.text("moves.log"), "the restore rename was refused")
        self.assertFalse(self.imports_a_missing_block(), "the live Caddyfile imports a block")

    def test_a_removal_whose_block_restore_fails_never_imports_a_missing_block(self):
        w = self.world
        self.installed()
        (w.log / "caddy-reload-fails").write_text("1")
        w.script("mv", MOVE_ONTO_BLOCK_FAILS)
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0)
        self.assertIn(
            "refused-rename", w.text("moves.log"), "the block's restore rename was refused"
        )
        self.assertFalse(self.imports_a_missing_block(), "the live Caddyfile imports a block")

    def test_a_removal_whose_adapted_configuration_is_refused_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / "caddy-adapt-refuses").write_text("x")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a refused adapted configuration")
        self.assertIn("deploy: refused", done.stderr, "the removal says which step stopped")
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        candidates = [p.name for p in w.caddy_dir.iterdir() if "candidate" in p.name]
        self.assertEqual(candidates, [], "no candidate file is left behind")

    def test_the_caddy_calls_name_the_caddyfile_adapter_for_the_candidate_copy(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        caddyfile.write_text("example.org {\n\trespond 200\n}\n", encoding="utf-8")
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0"))
        installed = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertEqual(installed.returncode, 0, installed.stdout + installed.stderr)
        removed = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertEqual(removed.returncode, 0, removed.stdout + removed.stderr)
        calls = [ln for ln in w.text("caddy.log").splitlines() if ln.startswith("caddy ")]
        named = [ln for ln in calls if "--adapter caddyfile" in ln]
        self.assertTrue(any(ln.startswith("caddy validate") for ln in named), calls)

    def test_the_caddy_block_is_rendered_from_the_tags_own_file(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        caddyfile.write_text("example.org {\n\trespond 200\n}\n", encoding="utf-8")
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0"))
        mark = "# a line only the tag's block holds"
        block = w.other / "deploy" / "caddy" / "deck-streak.caddy"
        w.git("checkout", "-q", "main", cwd=w.other)
        block.write_text(block.read_text(encoding="utf-8") + mark + "\n", encoding="utf-8")
        w.git("commit", "-q", "-am", "the tag's own block", cwd=w.other)
        w.git("push", "-q", "origin", "main", cwd=w.other)
        w.mark("v1.0.1", True)
        self.ok(w.run(DEPLOY, "caddy-install", "v1.0.1", **self.config()))
        rendered = (w.caddy_dir / "deck-streak.caddy").read_text(encoding="utf-8")
        self.assertIn(mark, rendered, "the block is the tag's, not the working tree's")


class NoDeployScriptNamesAPrivateValue(unittest.TestCase):
    FILES = (
        "deploy/deploy.sh",
        "deploy/rollback.sh",
        "deploy/scripts/render-caddy.py",
        ".github/workflows/release.yml",
    )

    def test_no_deploy_script_names_a_private_value(self):
        texts = {}
        for name in self.FILES:
            path = REPO / name
            self.assertTrue(path.is_file(), f"{name} does not exist")
            texts[name] = path.read_text(encoding="utf-8")
        scrub = REPO / "scripts" / "public-scrub.py"
        env = {k: v for k, v in os.environ.items() if k != "PERSONA_CORE_DENY_LIST"}
        octets = "203.0.113.7"
        word = "quokka-host"
        for name, text in examined("deploy files", texts.items()):
            self.assertNotRegex(text, PRIVATE_PATHS, f"{name} names a private path")
            for found in re.findall(r"\b\d{1,3}(?:\.\d{1,3}){3}\b", text):
                self.assertTrue(found.startswith(("127.", "0.0.0.0")), f"{name} holds {found}")
            with tempfile.TemporaryDirectory() as tmp:
                subject = Path(tmp) / "subject"
                subject.mkdir()
                (subject / Path(name).name).write_text(text, encoding="utf-8")
                done = subprocess.run(
                    [
                        sys.executable,
                        str(scrub),
                        "--root",
                        str(REPO),
                        "--no-tree",
                        "--subject",
                        str(subject),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                    env=env,
                )
                self.assertEqual(done.returncode, 0, f"{name}: {done.stdout}{done.stderr}")
            with tempfile.TemporaryDirectory() as tmp:
                subject = Path(tmp) / "subject"
                subject.mkdir()
                deny = Path(tmp) / "deny.json"
                deny.write_text(
                    json.dumps({"schema": "phx.persona.deny.v1", "literals": [word]}),
                    encoding="utf-8",
                )
                (subject / Path(name).name).write_text(
                    text + f"\n# {word} {octets}\n", encoding="utf-8"
                )
                done = subprocess.run(
                    [
                        sys.executable,
                        str(scrub),
                        "--root",
                        str(REPO),
                        "--no-tree",
                        "--subject",
                        str(subject),
                        "--deny-list",
                        str(deny),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                    env=env,
                )
                self.assertEqual(done.returncode, 1, f"{name}: a planted value passed")


if __name__ == "__main__":
    unittest.main()

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
import shlex
import signal
import stat
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
# a refusal can also find the candidate or the block already gone (a flag makes the stub delete it)
eat() {
    [ -f "$STUB_LOG/caddy-eats-candidate" ] && rm -f "$conf"
    [ -f "$STUB_LOG/caddy-eats-block" ] && rm -f "$(dirname "$conf")/deck-streak.caddy"
    return 0
}
[ "$first" = validate ] && [ -f "$STUB_LOG/caddy-refuses" ] && { eat; exit 1; }
[ "$first" = adapt ] && [ -f "$STUB_LOG/caddy-adapt-refuses" ] && { eat; exit 1; }
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
# The ONE list of `argv[0]` values the host stand-in runs: the word deploy.sh puts after the
# elevation command when the tests leave the setting empty. Anything else is refused by name.
HOST_ALLOWED = ("bash",)
HOST = (
    r"""#!/bin/bash
echo "host" >> "$STUB_LOG/host.log"
printf '%s\n' "${1-}" >> "$STUB_LOG/host-argv0.log"
allowed=(@ALLOWED@)
ok=
for shape in ${allowed[@]+"${allowed[@]}"}; do
  [ "${1-}" != "$shape" ] || ok=1
done
if [ -z "$ok" ]; then
  printf 'host stand-in: refusing %s: not a command the deploy tests use\n' "${1-}" >&2
  exit 97
fi
exec "$@"
"""
).replace("@ALLOWED@", " ".join(shlex.quote(name) for name in HOST_ALLOWED))

ELEVATE = "DECKSTREAK_DEPLOY_ELEVATE"


def sourced_names(sourced):
    """The variable names a file sourced before the script sets, judged by its EFFECT: only
    straight-line `NAME=value` and `export NAME=value` lines (and blanks and comments) are read.
    A line of any other shape (a branch, an `unset`, a call) could undo or hide a name, so a file
    holding one is refused rather than guessed at."""
    names = set()
    for line in Path(sourced).read_bytes().splitlines():
        if not line.strip() or line.lstrip().startswith(b"#"):
            continue
        match = re.fullmatch(rb"(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)=.*", line)
        if match is None:
            raise AssertionError(
                f"a sourced file holds a line that is not a plain setting: {line!r}"
            )
        names.add(match.group(1))
    return names


def setting_names(received, sourced=None):
    """The variable names an environment sets: a mapping's keys, a list's `NAME=value` entries (a
    bare name is not a variable), and the names a file sourced before the script sets."""
    if isinstance(received, dict):
        names = {os.fsencode(key) for key in received}
    else:
        names = {
            os.fsencode(entry).split(b"=", 1)[0] for entry in received if b"=" in os.fsencode(entry)
        }
    if sourced is not None:
        names |= sourced_names(sourced)
    return names


def launch(argv, received, *, sourced=None, cwd=None, env=None, text=False):
    """Start a deploy script in a session of its own, so a timeout ends it and its children.

    The ONE place a deploy script is started. The environment the program will SEE (`env`, and
    the file sourced before it) must name the elevation setting, and so must the entries the
    caller says it received; otherwise the script's own fallback could be what a test runs. An
    `env` of None is refused: the child would inherit whatever the test process holds."""
    wanted = ELEVATE.encode()
    if env is None:
        raise AssertionError(f"the environment does not name {ELEVATE}")
    for seen in (received, env):
        if wanted not in setting_names(seen, sourced):
            raise AssertionError(f"the environment does not name {ELEVATE}")
    child = subprocess.Popen(
        argv,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=text,
        start_new_session=True,
    )
    try:
        out, err = child.communicate(timeout=60)
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGKILL)
        child.communicate()
        raise
    return subprocess.CompletedProcess(child.args, child.returncode, out, err)


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


def release_files(tag, marker, extra=None, corrupt=False):
    """The synthetic release's files by path: what the workflow's tarball holds; `extra` adds
    files only this release ships."""
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
    files.update(extra or {})
    lines = "".join(f"{sha(data)}  {path}\n" for path, data in sorted(files.items()))
    if corrupt:
        lines = f"{'0' * 64}  bin/deckstreakd\n" + lines
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

    def publish(self, tag, marker=None, extra=None, corrupt=False):
        """The release's assets, as the release workflow would attach them."""
        files = release_files(tag, marker or tag, extra, corrupt)
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

    def ship(self, tag, marker=None, extra=None, corrupt=False):
        self.tag_on_main(tag)
        self.publish(tag, marker, extra, corrupt)

    def run(self, script, *args, **env):
        """Run a script in a session of its own, so a timeout ends the script and its children."""
        merged = {**self.env, **env}
        return launch(["bash", str(script), *args], merged, cwd=self.tmp, env=merged, text=True)

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

    def test_a_removal_whose_validation_is_refused_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / "caddy-refuses").write_text("x")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a refused validation")
        self.assertIn("deploy: refused", done.stderr, "the removal says which step stopped")
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        candidates = [p.name for p in w.caddy_dir.iterdir() if "candidate" in p.name]
        self.assertEqual(candidates, [], "no candidate file is left behind")

    def test_a_removal_whose_candidate_cannot_be_written_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.caddy_dir / "deck-streak.candidate").mkdir()
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a failed candidate write refuses the removal")
        self.assertIn(
            "the candidate Caddyfile could not be written",
            done.stderr,
            "the removal names the failed write",
        )
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)

    def test_a_removal_whose_caddyfile_cannot_be_read_says_so(self):
        w = self.world
        _original, _after, block_text = self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        caddyfile.chmod(0)
        try:
            done = w.run(ROLLBACK, "caddy-remove", **self.config())
        finally:
            caddyfile.chmod(0o644)
        self.assertNotEqual(done.returncode, 0, "an unreadable Caddyfile refuses the removal")
        self.assertIn(
            "the candidate Caddyfile could not be written",
            done.stderr,
            "grep's read failure is a failed write, not a no-match",
        )
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        candidates = [p.name for p in w.caddy_dir.iterdir() if "candidate" in p.name]
        self.assertEqual(candidates, [], "no candidate file is left behind")

    def test_a_removal_from_a_caddyfile_holding_only_the_import_line_is_not_a_failure(self):
        w = self.world
        _original, after, _block_text = self.installed()
        line = next(ln for ln in after.splitlines() if "import" in ln)
        caddyfile = w.caddy_dir / "Caddyfile"
        caddyfile.write_text(line + "\n", encoding="utf-8")
        self.ok(w.run(ROLLBACK, "caddy-remove", **self.config()))
        self.assertEqual(caddyfile.read_text(), "", "grep found no line to keep")
        reloads = [ln for ln in w.text("caddy.log").splitlines() if ln.startswith("caddy reload")]
        self.assertEqual(len(reloads), 2, "the install's reload and the removal's, so it went on")
        self.assertFalse((w.caddy_dir / "deck-streak.caddy").exists(), "the block is removed")

    def refused_with_no_candidate(self, flag):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / flag).write_text("x")
        (w.log / "caddy-eats-candidate").write_text("x")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a refused configuration")
        self.assertIn("deploy: refused", done.stderr, "the removal says so with no candidate left")
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertEqual(block.read_text() if block.exists() else None, block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)

    def test_a_removal_refused_at_validation_with_no_candidate_still_says_so(self):
        self.refused_with_no_candidate("caddy-refuses")

    def test_a_removal_refused_at_the_adapt_check_with_no_candidate_still_says_so(self):
        self.refused_with_no_candidate("caddy-adapt-refuses")

    REFUSED = "the Caddy configuration was refused"

    def install_again(self, **extra):
        return self.world.run(
            DEPLOY, "caddy-install", "v1.0.0", **self.config(host="new.example.org", **extra)
        )

    def assert_install_undone(self, done, after, block_text):
        """The refusal is printed and the previous block, Caddyfile and no copies are on disk."""
        w = self.world
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        block = w.caddy_dir / "deck-streak.caddy"
        self.assertTrue(block.is_file(), "the previous block is on disk")
        self.assertEqual(block.read_text(), block_text, "the previous block is back")
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        self.assertEqual(self.leftovers(), [], "no previous copy is left behind")

    def install_refused_with_no_candidate(self, flag):
        w = self.world
        _original, after, block_text = self.installed()
        (w.log / flag).write_text("x")
        (w.log / "caddy-eats-candidate").write_text("x")
        done = self.install_again()
        self.assert_install_undone(done, after, block_text)
        self.assertTrue(w.text("caddy.log"), "caddy ran, so the refusal came from a check")

    def test_an_install_refused_at_validation_with_no_candidate_still_undoes(self):
        self.install_refused_with_no_candidate("caddy-refuses")

    def test_an_install_refused_at_the_adapt_check_with_no_candidate_still_undoes(self):
        self.install_refused_with_no_candidate("caddy-adapt-refuses")

    def test_an_install_whose_candidate_path_cannot_be_written_undoes_and_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.caddy_dir / "deck-streak.candidate").mkdir()
        done = self.install_again()
        self.assert_install_undone(done, after, block_text)
        self.assertTrue((w.caddy_dir / "deck-streak.candidate").is_dir(), "a directory is kept")

    def test_an_install_whose_candidate_is_a_link_to_the_caddyfile_refuses_and_undoes(self):
        w = self.world
        _original, after, block_text = self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        before = caddyfile.read_bytes()
        (w.caddy_dir / "deck-streak.candidate").symlink_to(caddyfile)
        done = self.install_again()
        self.assertNotEqual(done.returncode, 0, "a linked candidate refuses the install")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertFalse(caddyfile.is_symlink(), "the live Caddyfile is still a file")
        self.assertEqual(caddyfile.read_bytes(), before, "the live Caddyfile is byte for byte")
        self.assertTrue(before, "the live Caddyfile is not empty")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual(caddyfile.read_text(), after)

    def test_an_install_whose_block_cannot_be_written_undoes_and_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        (w.caddy_dir / "deck-streak.caddy").chmod(0o444)
        done = self.install_again()
        self.assert_install_undone(done, after, block_text)

    def test_an_install_whose_import_line_cannot_be_added_undoes_and_says_so(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        caddyfile.chmod(0o444)
        w.ship("v1.0.0")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertEqual(caddyfile.read_text(), original, "the Caddyfile is as it was")
        self.assertFalse((w.caddy_dir / "deck-streak.caddy").exists(), "the new block is removed")
        self.assertEqual(
            sorted(p.name for p in w.caddy_dir.iterdir()), ["Caddyfile"], "nothing else is left"
        )

    def test_a_removal_whose_candidate_is_a_link_to_the_caddyfile_refuses_before_writing(self):
        w = self.world
        _original, after, block_text = self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        before = caddyfile.read_bytes()
        candidate = w.caddy_dir / "deck-streak.candidate"
        candidate.symlink_to(caddyfile)
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a linked candidate refuses the removal")
        self.assertIn(
            "the candidate Caddyfile could not be written", done.stderr, "the write's message"
        )
        self.assertEqual(caddyfile.read_bytes(), before, "the live Caddyfile is byte for byte")
        self.assertTrue(before, "the live Caddyfile is not empty")
        self.assertFalse(caddyfile.is_symlink(), "the live Caddyfile is still a file")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual(caddyfile.read_text(), after)
        reloads = [ln for ln in w.text("caddy.log").splitlines() if ln.startswith("caddy reload")]
        self.assertEqual(len(reloads), 1, "only the install reloaded")

    def test_a_removal_whose_candidate_is_a_dangling_link_refuses_before_writing(self):
        w = self.world
        _original, after, block_text = self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        before = caddyfile.read_bytes()
        target = w.caddy_dir / "nowhere"
        (w.caddy_dir / "deck-streak.candidate").symlink_to(target)
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a dangling link refuses the removal")
        self.assertIn(
            "the candidate Caddyfile could not be written", done.stderr, "the write's message"
        )
        self.assertEqual(caddyfile.read_bytes(), before, "the live Caddyfile is byte for byte")
        self.assertFalse(caddyfile.is_symlink(), "the live Caddyfile is still a file")
        self.assertFalse(target.exists(), "nothing was written through the link")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual(caddyfile.read_text(), after)

    def other_file(self):
        """A regular file beside the Caddyfile that no run owns; returns (path, its bytes)."""
        other = self.world.caddy_dir / "other.caddy"
        other.write_text("other.example.org {\n\trespond 204\n}\n", encoding="utf-8")
        return other, other.read_bytes()

    def assert_live_caddyfile_kept(self, before):
        caddyfile = self.world.caddy_dir / "Caddyfile"
        self.assertFalse(caddyfile.is_symlink(), "the live Caddyfile is still a file")
        self.assertEqual(caddyfile.read_bytes(), before, "the live Caddyfile is byte for byte")
        self.assertTrue(before, "the live Caddyfile is not empty")

    def test_an_install_whose_candidate_links_to_another_file_refuses_before_writing(self):
        w = self.world
        _original, _after, block_text = self.installed()
        before = (w.caddy_dir / "Caddyfile").read_bytes()
        other, other_bytes = self.other_file()
        (w.caddy_dir / "deck-streak.candidate").symlink_to(other)
        done = self.install_again()
        self.assertNotEqual(done.returncode, 0, "a linked candidate refuses the install")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assert_live_caddyfile_kept(before)
        self.assertEqual(other.read_bytes(), other_bytes, "nothing is written through the link")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual(self.leftovers(), [], "no previous copy is left behind")

    def test_an_install_whose_block_is_a_hard_link_to_the_caddyfile_refuses_before_writing(self):
        w = self.world
        self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        before = caddyfile.read_bytes()
        block = w.caddy_dir / "deck-streak.caddy"
        block.unlink()
        os.link(caddyfile, block)
        done = self.install_again()
        self.assertNotEqual(done.returncode, 0, "a hard-linked block refuses the install")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assert_live_caddyfile_kept(before)
        self.assertEqual(self.leftovers(), [], "no previous copy is left behind")

    def test_a_removal_whose_candidate_is_a_hard_link_to_the_caddyfile_refuses_before_writing(self):
        w = self.world
        _original, _after, block_text = self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        before = caddyfile.read_bytes()
        os.link(caddyfile, w.caddy_dir / "deck-streak.candidate")
        done = w.run(ROLLBACK, "caddy-remove", **self.config())
        self.assertNotEqual(done.returncode, 0, "a hard-linked candidate refuses the removal")
        self.assertIn(
            "the candidate Caddyfile could not be written", done.stderr, "the write's message"
        )
        self.assert_live_caddyfile_kept(before)
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)

    def test_a_removal_whose_candidate_is_a_fifo_refuses_before_writing(self):
        w = self.world
        _original, _after, block_text = self.installed()
        before = (w.caddy_dir / "Caddyfile").read_bytes()
        os.mkfifo(w.caddy_dir / "deck-streak.candidate")
        try:
            done = w.run(ROLLBACK, "caddy-remove", **self.config())
        except subprocess.TimeoutExpired:
            self.fail("the removal waited for a reader of a FIFO at its candidate path")
        self.assertNotEqual(done.returncode, 0, "a FIFO candidate refuses the removal")
        self.assertIn(
            "the candidate Caddyfile could not be written", done.stderr, "the write's message"
        )
        self.assert_live_caddyfile_kept(before)
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)

    def test_a_first_install_into_a_read_only_caddy_directory_says_so(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        w.ship("v1.0.0")
        w.caddy_dir.chmod(0o555)
        try:
            done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        finally:
            w.caddy_dir.chmod(0o755)
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertEqual(caddyfile.read_text(), original, "the Caddyfile is as it was")
        self.assertEqual(sorted(p.name for p in w.caddy_dir.iterdir()), ["Caddyfile"])

    def test_a_first_install_never_deletes_a_directory_at_the_block_path(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        w.ship("v1.0.0")
        kept = w.caddy_dir / "deck-streak.caddy" / "keep.txt"
        kept.parent.mkdir()
        kept.write_text("not the install's\n", encoding="utf-8")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertTrue(kept.is_file(), "the directory and its file are kept")
        self.assertEqual(kept.read_text(), "not the install's\n")
        self.assertEqual(caddyfile.read_text(), original, "the Caddyfile is as it was")

    def test_an_install_whose_previous_caddyfile_copy_cannot_be_written_undoes_and_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        stale = w.caddy_dir / "Caddyfile.previous"
        stale.write_text("stale\n", encoding="utf-8")
        stale.chmod(0o444)
        done = self.install_again()
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        self.assertEqual(self.leftovers(), ["Caddyfile.previous"], "only the stale copy is left")
        self.assertFalse((w.caddy_dir / "deck-streak.candidate").exists(), "no candidate is left")

    def test_an_install_whose_candidate_rename_fails_undoes_and_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        self.failing_rename(".candidate")
        done = self.install_again()
        self.assert_install_undone(done, after, block_text)
        self.assertTrue(
            (w.caddy_dir / "Caddyfile").is_file(), "the Caddyfile is still a plain file"
        )
        self.assertFalse((w.caddy_dir / "deck-streak.candidate").exists(), "no candidate is left")

    def test_an_install_whose_caddyfile_cannot_be_read_undoes_and_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        caddyfile = w.caddy_dir / "Caddyfile"
        caddyfile.chmod(0)
        try:
            done = self.install_again()
        finally:
            caddyfile.chmod(0o644)
        self.assert_install_undone(done, after, block_text)
        self.assertTrue(
            (w.caddy_dir / "Caddyfile").is_file(), "the Caddyfile is still a plain file"
        )
        self.assertFalse((w.caddy_dir / "deck-streak.candidate").exists(), "no candidate is left")

    def test_an_install_that_cannot_copy_the_block_in_a_read_only_directory_says_so(self):
        w = self.world
        _original, after, block_text = self.installed()
        stale = w.caddy_dir / "deck-streak.candidate"
        stale.write_text("stale\n", encoding="utf-8")
        w.caddy_dir.chmod(0o555)
        try:
            done = self.install_again()
        finally:
            w.caddy_dir.chmod(0o755)
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        self.assertEqual(self.leftovers(), [], "no previous copy is left behind")

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

    UNWRITTEN = "the candidate Caddyfile could not be written"
    NOT_PLAIN = ("symlink-to-file", "hard-link-to-file", "symlink-to-dir", "dir-with-file", "fifo")

    def fresh_world(self):
        """A new World for one member of a population, cleaned up with the test."""
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.world = World(tmp.name)
        return self.world

    def plant(self, path, kind):
        """Put a shape that is not a plain file at `path`; returns a file outside the run's names."""
        other = path.parent / "other.caddy"
        other.write_text("other.example.org {\n\trespond 204\n}\n", encoding="utf-8")
        if kind == "symlink-to-file":
            path.symlink_to(other)
        elif kind == "hard-link-to-file":
            os.link(other, path)
        elif kind == "symlink-to-dir":
            (path.parent / "adir").mkdir()
            path.symlink_to(path.parent / "adir")
        elif kind == "dir-with-file":
            path.mkdir()
            (path / "keep.txt").write_text("not the run's\n", encoding="utf-8")
        else:
            os.mkfifo(path)
        return other

    def test_a_removal_refuses_every_previous_copy_that_is_not_a_plain_file(self):
        for name in ("Caddyfile.previous", "deck-streak.caddy.previous"):
            for kind in self.NOT_PLAIN:
                with self.subTest(name=name, kind=kind):
                    w = self.fresh_world()
                    _original, _after, block_text = self.installed()
                    before = (w.caddy_dir / "Caddyfile").read_bytes()
                    other = self.plant(w.caddy_dir / name, kind)
                    was = other.read_bytes()
                    try:
                        done = w.run(ROLLBACK, "caddy-remove", **self.config())
                    except subprocess.TimeoutExpired:
                        self.fail(f"the removal waited on a {kind} at {name}")
                    self.assertNotEqual(done.returncode, 0, "the removal refuses")
                    self.assertIn(self.UNWRITTEN, done.stderr, "the refusal is printed")
                    self.assert_live_caddyfile_kept(before)
                    self.assertEqual(other.read_bytes(), was, "nothing is written through a link")
                    self.assertEqual((w.caddy_dir / "deck-streak.caddy").read_text(), block_text)
                    if kind == "dir-with-file":
                        self.assertTrue((w.caddy_dir / name / "keep.txt").is_file(), "kept")
                    if kind == "symlink-to-dir":
                        self.assertEqual(list((w.caddy_dir / "adir").iterdir()), [], "untouched")
                    self.assertFalse((w.caddy_dir / "deck-streak.candidate").exists(), "no copy")

    def test_neither_script_waits_on_a_fifo_at_the_live_caddyfile(self):
        for script, args, message in (
            (DEPLOY, ("caddy-install", "v1.0.0"), self.REFUSED),
            (ROLLBACK, ("caddy-remove",), self.UNWRITTEN),
        ):
            with self.subTest(script=script.name):
                w = self.fresh_world()
                self.installed()
                caddyfile = w.caddy_dir / "Caddyfile"
                caddyfile.unlink()
                os.mkfifo(caddyfile)
                try:
                    done = w.run(script, *args, **self.config(host="new.example.org"))
                except subprocess.TimeoutExpired:
                    self.fail(f"{script.name} waited on a FIFO at the live Caddyfile")
                self.assertNotEqual(done.returncode, 0, "the script refuses")
                self.assertIn(message, done.stderr, "the refusal is printed")
                self.assertTrue(caddyfile.is_fifo(), "the pipe is left alone")
                names = sorted(p.name for p in w.caddy_dir.iterdir())
                self.assertEqual(names, ["Caddyfile", "deck-streak.caddy"], "no copy is left")

    def test_a_read_only_caddy_directory_is_refused_before_any_write(self):
        for label, script, args, stale, message in (
            ("first install", DEPLOY, ("caddy-install", "v1.0.0"), "deck-streak.candidate", None),
            ("install", DEPLOY, ("caddy-install", "v1.0.0"), "deck-streak.caddy.previous", None),
            ("removal", ROLLBACK, ("caddy-remove",), "deck-streak.candidate", self.UNWRITTEN),
        ):
            with self.subTest(label, stale=stale):
                w = self.fresh_world()
                if label == "first install":
                    text = "example.org {\n\trespond 200\n}\n"
                    (w.caddy_dir / "Caddyfile").write_text(text, encoding="utf-8")
                    w.ship("v1.0.0")
                else:
                    self.installed()
                (w.caddy_dir / stale).write_text("stale\n", encoding="utf-8")
                before = {p.name: p.read_bytes() for p in w.caddy_dir.iterdir()}
                w.caddy_dir.chmod(0o555)
                try:
                    done = w.run(script, *args, **self.config(host="new.example.org"))
                finally:
                    w.caddy_dir.chmod(0o755)
                self.assertNotEqual(done.returncode, 0, "the script refuses")
                self.assertIn(message or self.REFUSED, done.stderr, "the refusal is printed")
                after = {p.name: p.read_bytes() for p in w.caddy_dir.iterdir()}
                self.assertEqual(after, before, "no file in the directory changed")

    def test_an_install_whose_block_cannot_be_read_refuses_before_writing(self):
        w = self.world
        _original, after, block_text = self.installed()
        block = w.caddy_dir / "deck-streak.caddy"
        block.chmod(0)
        try:
            done = self.install_again()
        finally:
            block.chmod(0o644)
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the refusal is printed")
        self.assertEqual(block.read_text(), block_text, "the block is as it was")
        self.assertEqual((w.caddy_dir / "Caddyfile").read_text(), after)
        names = sorted(p.name for p in w.caddy_dir.iterdir())
        self.assertEqual(names, ["Caddyfile", "deck-streak.caddy"], "no copy is left")

    def test_a_first_install_refused_with_its_block_already_gone_still_says_so(self):
        w = self.world
        caddyfile = w.caddy_dir / "Caddyfile"
        original = "example.org {\n\trespond 200\n}\n"
        caddyfile.write_text(original, encoding="utf-8")
        w.ship("v1.0.0")
        (w.log / "caddy-refuses").write_text("x")
        (w.log / "caddy-eats-block").write_text("x")
        done = w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config())
        self.assertNotEqual(done.returncode, 0, "the install refuses")
        self.assertIn(self.REFUSED, done.stderr, "the undo tolerates an absent block")
        self.assertEqual(caddyfile.read_text(), original, "the Caddyfile is as it was")
        self.assertEqual(sorted(p.name for p in w.caddy_dir.iterdir()), ["Caddyfile"])

    # Where a Caddy step writes is MEASURED, not listed: each step runs in each layout the tests
    # install, on each of its exits, the whole synthetic tree is compared around it, and every
    # directory holding a path it added, removed or changed is a place. A second pass makes the
    # rest of the tree read-only, so a path a step adds and removes again within one run fails it.
    # The population below then makes each place unwritable in turn, so a step that writes
    # anywhere it did not check first fails the test.
    STEPS = (
        ("install, rename fails", DEPLOY, ("caddy-install", "v1.0.0"), "app.example.org", 1),
        ("install first", DEPLOY, ("caddy-install", "v1.0.0"), "app.example.org", 0),
        ("install again", DEPLOY, ("caddy-install", "v1.0.0"), "new.example.org", 0),
        ("removal, rename fails", ROLLBACK, ("caddy-remove",), "new.example.org", 1),
        ("removal", ROLLBACK, ("caddy-remove",), "new.example.org", 0),
        ("install refused", DEPLOY, ("caddy-install", "v1.0.0"), "app.example.org", 1),
        ("reload fails", DEPLOY, ("caddy-install", "v1.0.0"), "app.example.org", 1),
        ("no configuration", DEPLOY, ("caddy-install", "v1.0.0"), "app.example.org", 1),
    )
    TRIGGERS = {"install refused": "caddy-refuses", "reload fails": "caddy-reload-fails"}
    # Paths a step may write that are the operator's own, not the host's: its temporary
    # directory, the checkout's git directory, and the stand-ins' log.
    OPERATOR = (Path("tmpdir"), Path("checkout") / ".git", Path("log"))
    # Every command bash runs, as bash reports it before running it, with the file it comes from
    # and the run of bash it belongs to.
    BASH_RECORD = r"""set -T
record_run="$$ $SRANDOM"
trap 'printf "%s\0%s\0%s\0" "$record_run" "${BASH_SOURCE[0]:-}" "$BASH_COMMAND" >>"$RECORD_LOG"' DEBUG
"""
    # Every path bash names in a command it runs, expanded, and every path python opens, looks
    # at or changes, for the named-path check.
    BASH_TRACE = r"""exec {TRACE_FD}>>"$TRACE_LOG"
BASH_XTRACEFD=$TRACE_FD
PS4='+ '
set -x
"""
    PYTHON_TRACE = r"""#!/bin/bash
PATH=${PATH#*:}
exec python3 -c '
import os, runpy, sys
log = open(os.environ["TRACE_LOG"], "a", encoding="utf-8")
def note(path):
    if isinstance(path, (str, bytes, os.PathLike)):
        log.write("+py " + os.fsdecode(path) + "\n")
        log.flush()
def hook(event, args):
    if event == "open" or event.startswith(("os.", "shutil.")):
        for arg in args[:2]:
            note(arg)
sys.addaudithook(hook)
for name in ("stat", "lstat"):
    def looked(path, *rest, _real=getattr(os, name), **options):
        note(path)
        return _real(path, *rest, **options)
    setattr(os, name, looked)
sys.argv = sys.argv[1:]
runpy.run_path(sys.argv[0], run_name="__main__")
' "$@"
"""
    PREFIX = "DECKSTREAK_DEPLOY_"
    # All a Caddy step runs before its refusal, as bash reports each command: rollback.sh only
    # finds and execs deploy.sh, and deploy.sh sets its options and its settings. The refusal
    # stops a setting only if nothing before it could read one.
    ROLLBACK_RUN = (
        "set -euo pipefail",
        'here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)',
        'cd "$(dirname "${BASH_SOURCE[0]}")"',
        'dirname "${BASH_SOURCE[0]}"',
        "pwd",
        'case "${1:-}" in ',
        'exec "$here/deploy.sh" caddy-remove',
    )
    REFUSAL = "for name in ${!DECKSTREAK_DEPLOY_@}"
    # Exec argv with exactly the entries given, in order, through libc's execve: a mapping, as
    # subprocess and os.execve take, cannot hold a name twice or an entry without "=".
    EXEC_ENTRIES = r"""import ctypes, shutil, sys
entries = [bytes.fromhex(entry) for entry in sys.argv[1].split()]
argv = [shutil.which("bash").encode(), *(arg.encode() for arg in sys.argv[2:])]
libc = ctypes.CDLL(None, use_errno=True)
libc.execve(
    argv[0],
    (ctypes.c_char_p * (len(argv) + 1))(*argv, None),
    (ctypes.c_char_p * (len(entries) + 1))(*entries, None),
)
sys.exit(f"execve failed: errno {ctypes.get_errno()}")
"""

    @staticmethod
    def tree(root):
        """Each path under root but the stubs' own log: its type, mode, and bytes or link target."""
        seen = {}
        for path in sorted(root.rglob("*")):
            relative = path.relative_to(root)
            if relative.parts[0] == "log":
                continue
            info = path.lstat()
            data = None
            if stat.S_ISLNK(info.st_mode):
                data = os.readlink(path)
            elif stat.S_ISREG(info.st_mode):
                data = path.read_bytes()
            seen[relative] = (stat.S_IFMT(info.st_mode), stat.S_IMODE(info.st_mode), data)
        return seen

    @staticmethod
    def settings():
        """The settings deploy.sh names."""
        text = DEPLOY.read_text(encoding="utf-8")
        found = re.search(r"^SETTINGS='([^']*)'", text, re.MULTILINE)
        return found.group(1).split() if found else []

    @staticmethod
    def locked(root, places, operator):
        """Make every directory and file under root read-only but the places, the files directly
        in them, and the operator's paths; return each changed path's mode, to restore."""
        modes = {}
        for path in [root, *root.rglob("*")]:
            relative = path.relative_to(root)
            if path.is_symlink() or relative in places or relative.parent in places:
                continue
            if any(o == relative or o in relative.parents for o in operator):
                continue
            mode = stat.S_IMODE(path.lstat().st_mode)
            if mode & 0o222:
                modes[path] = mode
                path.chmod(mode & ~0o222)
        return modes

    def measured(self, layout, record=False, places=None, trace=False):
        """A new world laid out as `layout`, the paths each of STEPS changed in it, and the world
        paths each step named.

        The operator's temporary directory and home, and each named setting the world does not
        give, are directories of the world, so a write through any of them is measured too. With
        `places`, every other path of the world is read-only while each step runs. The site's web
        root lies outside the world: it is text the block holds, not a path a step touches."""
        w = self.fresh_world()
        cfdir = w.tmp / "host" / "etc" / "cfdir" if layout == "apart" else w.caddy_dir
        cfdir.mkdir(exist_ok=True)
        caddyfile = cfdir / "Caddyfile"
        caddyfile.write_text("example.org {\n\trespond 200\n}\n", encoding="utf-8")
        w.ship("v1.0.0")
        env = {"DECKSTREAK_DEPLOY_CADDYFILE": str(caddyfile)}
        given = {"TMPDIR": w.tmp / "tmpdir", "HOME": w.tmp / "home"}
        provided = {**w.env, **self.config(), **env}
        for name in self.settings():
            if name not in provided:
                given[name] = w.tmp / "setting" / name
        for name, directory in given.items():
            directory.mkdir(parents=True)
            env[name] = str(directory)
        if record:
            (w.stub / "record.bash").write_text(self.BASH_RECORD, encoding="utf-8")
            env |= {"BASH_ENV": str(w.stub / "record.bash"), "RECORD_LOG": str(w.log / "record")}
        if trace:
            w.script("python3", self.PYTHON_TRACE)
            (w.stub / "trace.bash").write_text(self.BASH_TRACE, encoding="utf-8")
            env |= {"BASH_ENV": str(w.stub / "trace.bash"), "TRACE_LOG": str(w.log / "trace")}
        self.ok(w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config(), **env))
        self.ok(w.run(ROLLBACK, "caddy-remove", **self.config(), **env))
        changed, named = {}, {}
        world = re.compile(re.escape(str(w.tmp)) + r"(?:/[^\s'\"]*)?")
        for op, script, args, host, code in self.STEPS:
            step = {**self.config(host=host, web_root="/srv/deck-streak/current/web"), **env}
            start = (w.log / "trace").stat().st_size if (w.log / "trace").exists() else 0
            if op == "no configuration":
                step["DECKSTREAK_DEPLOY_CADDY_CONFIG"] = str(w.tmp / "tmpdir" / "absent.json")
            if op.endswith("rename fails"):
                self.failing_rename(".candidate")
            if op in self.TRIGGERS:
                (w.log / self.TRIGGERS[op]).write_text("1", encoding="utf-8")
            before = self.tree(w.tmp)
            modes = self.locked(w.tmp, places, self.OPERATOR) if places is not None else {}
            if places is not None:
                self.assertTrue(modes, f"{op}: nothing was made read-only: nothing was confined")
            try:
                done = w.run(script, *args, **step)
            finally:
                for path, mode in modes.items():
                    path.chmod(mode)
            after = self.tree(w.tmp)
            w.script("mv", LOGGED.replace("@NAME@", "mv"))
            if op in self.TRIGGERS:
                (w.log / self.TRIGGERS[op]).unlink(missing_ok=True)
            self.assertEqual(done.returncode, code, f"{op}: {done.stderr}")
            changed[op] = {p for p in before.keys() | after.keys() if before.get(p) != after.get(p)}
            if trace:
                with (w.log / "trace").open(encoding="utf-8", errors="replace") as log:
                    log.seek(start)
                    found = {Path(p).relative_to(w.tmp) for p in world.findall(log.read())}
                # A setting's own value, the working directory and the operator's paths are named
                # by every step; what is left must lie in a place.
                values = {v for v in {**w.env, **step}.values() if v.startswith(f"{w.tmp}/")}
                values = {Path(v).relative_to(w.tmp) for v in values} | {Path(".")}
                named[op] = {
                    p
                    for p in found - values
                    if not any(o == p or o in p.parents for o in (*self.OPERATOR, Path("stub")))
                }
        block = (w.caddy_dir / "deck-streak.caddy").relative_to(w.tmp)
        for op in ("install first", "install again", "removal"):
            self.assertIn(block, changed[op], f"{op} changed no block: nothing was measured")
            if trace:
                self.assertIn(block, named[op], f"{op} named no block: nothing was traced")
        return w, cfdir, changed, named

    def test_a_caddy_step_reads_only_the_settings_it_names_and_refuses_any_other(self):
        w, cfdir, _, _ = self.measured("apart", record=True)
        record = w.log / "record"
        self.assertTrue(record.is_file(), "bash recorded no command: nothing was recorded")
        fields = [field.decode() for field in record.read_bytes().split(b"\0")[:-1]]
        records = list(
            zip(
                fields[0 : len(fields) : 3],
                fields[1 : len(fields) : 3],
                fields[2 : len(fields) : 3],
                strict=True,
            )
        )
        # The names the refusal must refuse, drawn from the environment's own grammar: the bare
        # prefix, each listed setting with a suffix, and the prefix with each byte a name may hold
        # (any but NUL and "=") first, in the middle and last. Bash makes no variable of most of
        # them, and hands every one to each program it runs.
        listed = {os.fsencode(name) for name in self.settings()}
        prefix = os.fsencode(self.PREFIX)
        unnamed = {prefix} | {name + extra for name in listed for extra in (b"2", b"_BACKUP")}
        for byte in (bytes([b]) for b in range(1, 256) if b != ord("=")):
            unnamed |= {
                prefix + byte,
                prefix + b"CADDY" + byte + b"DIR",
                prefix + b"CADDY_DIR" + byte,
            }
        # Without an unnamed setting both steps succeed, so a refusal below is the setting's.
        steps = {step[0]: step for step in self.STEPS}
        plain = {**self.config(), "DECKSTREAK_DEPLOY_CADDYFILE": str(cfdir / "Caddyfile")}
        for op in ("install first", "removal"):
            script, args = steps[op][1:3]
            self.ok(w.run(script, *args, **plain))
        (w.tmp / "unnamed").mkdir()
        made = w.stub / "made.bash"
        made.write_text(self.PREFIX + "=made\n", encoding="utf-8")
        held = w.stub / "held.bash"
        held.write_text(
            "".join(
                f"{key}={shlex.quote(value)}\n"
                for key, value in {**w.env, **plain}.items()
                if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key)
            ),
            encoding="utf-8",
        )
        before = self.tree(w.tmp)
        environ = {os.fsencode(k): os.fsencode(v) for k, v in {**w.env, **plain}.items()}
        for i, name in enumerate(examined("unnamed setting(s)", sorted(unnamed - listed))):
            script, args = steps[("install first", "removal")[i % 2]][1:3]
            named = {**environ, name: os.fsencode(w.tmp / "unnamed")}
            done = launch(["bash", str(script), *args], named, cwd=w.tmp, env=named)
            self.assertEqual(
                done.returncode, 1, f"{name!r} : a step ran with a setting it does not name"
            )
            self.assertIn(b"deploy: " + name + b" is not a setting", done.stderr)
        # A refused step writes nothing, so what any refusal wrote would still be there.
        self.assertEqual(self.tree(w.tmp), before, "a refusal changed the tree")
        # A listed setting given twice or without a value, a prefixed entry without "=", an
        # environment the step cannot read, and a deploy variable made before the step starts
        # (by the file BASH_ENV names) are refused too. The first three reach deploy.sh as given
        # only when it is run directly: rollback.sh's bash passes one entry per name, with "=".
        # The step that cannot read its environment received none: its settings are variables of
        # the shell that sources it, so were the refusal to let it run, it would run in the world.
        given = [k + b"=" + v for k, v in environ.items()]
        absent = next(name for name in sorted(listed) if name not in environ)
        sourced = ["-c", 'set -a; . "$1"; set +a; shift; . "$@"', "bash", str(held)]
        odd = {
            "given twice": (
                [*given, b"DECKSTREAK_DEPLOY_CADDY_DIR=" + os.fsencode(w.tmp)],
                [],
                b" is given twice",
            ),
            "without a value": ([*given, absent], [], b" is given without a value"),
            "no =": ([*given, prefix + b"CADDY-DIR"], [], b" is not a setting"),
            "nothing received": ([], sourced, b"could not be read"),
            "made before": (
                [*given, b"BASH_ENV=" + os.fsencode(made)],
                [],
                b" is not a setting",
            ),
        }
        for case, (entries, lead, said) in examined("odd environment(s)", sorted(odd.items())):
            for args in (("caddy-install", "v1.0.0"), ("caddy-remove",)):
                done = launch(
                    [
                        sys.executable,
                        "-c",
                        self.EXEC_ENTRIES,
                        " ".join(entry.hex() for entry in entries),
                        *lead,
                        str(DEPLOY),
                        *args,
                    ],
                    entries,
                    sourced=held if lead else None,
                    cwd=w.tmp,
                    env=dict(entry.split(b"=", 1) for entry in entries if b"=" in entry),
                )
                self.assertEqual(done.returncode, 1, f"{case}, {args[0]}: {done.stderr!r}")
                self.assertIn(said, done.stderr, f"{case}, {args[0]}")
                self.assertEqual(self.tree(w.tmp), before, f"{case}, {args[0]}: the tree changed")
        # Nothing a step runs before its refusal is anything but what ROLLBACK_RUN and deploy.sh
        # declare, so no read, in any form, comes before the refusal can stop it.
        runs = {}
        for run, source, command in records:
            if source in (str(DEPLOY), str(ROLLBACK)):
                runs.setdefault((run, source), []).append(command)
        settings = re.search(r"^SETTINGS='[^']*'", DEPLOY.read_text(encoding="utf-8"), re.M)
        opening = ("set -euo pipefail", settings.group(0) if settings else "", 'case "${1:-}" in ')
        declared = {str(ROLLBACK): self.ROLLBACK_RUN, str(DEPLOY): (*opening, self.REFUSAL)}
        self.assertEqual({source for _, source in runs}, set(declared), "a script never ran")
        for (_, source), commands in examined("script run(s)", sorted(runs.items())):
            first = declared[source]
            self.assertEqual(
                tuple(commands[: len(first)]),
                first,
                f"{Path(source).name} runs something before the refusal",
            )

    @staticmethod
    def listing(*directories):
        """Each directory's mode and each entry's type, mode, link count and bytes."""
        seen = {}
        for directory in directories:
            if not directory.is_dir():
                seen[str(directory)] = None
                continue
            entries = {}
            for path in sorted(directory.iterdir()):
                info = path.lstat()
                data = path.read_bytes() if stat.S_ISREG(info.st_mode) else None
                entries[path.name] = (
                    stat.S_IFMT(info.st_mode),
                    stat.S_IMODE(info.st_mode),
                    info.st_nlink,
                    data,
                )
            seen[str(directory)] = (stat.S_IMODE(directory.stat().st_mode), entries)
        return seen

    def test_every_directory_a_caddy_script_writes_or_undoes_is_checked_before_the_first_write(
        self,
    ):
        stales = {"caddy-dir": ("deck-streak.candidate",), "caddyfile-dir": ("Caddyfile.previous",)}
        stales["shared"] = stales["caddy-dir"] + stales["caddyfile-dir"]
        places = []
        for layout in ("beside", "apart"):
            w, cfdir, changed, named = self.measured(layout, trace=True)
            measured = {p.parent for paths in changed.values() for p in paths}
            _, _, again, _ = self.measured(layout, places=measured)
            self.assertEqual(again, changed, f"{layout}: a step writes outside its places")
            for op, paths in examined(f"{layout} step(s) named paths", sorted(named.items())):
                elsewhere = sorted(str(p) for p in paths if not {p, p.parent} & measured)
                self.assertEqual(
                    elsewhere, [], f"{layout}, {op}: a step names a path outside its places"
                )
            named = {w.caddy_dir: "shared"}
            if layout == "apart":
                named = {w.caddy_dir: "caddy-dir", cfdir: "caddyfile-dir"}
            for directory in sorted(w.tmp / p for p in measured):
                places.append((layout, named.get(directory, str(directory.relative_to(w.tmp)))))
        states = ("writable", "read-only", "read-only with a stale writable copy")
        ops = ("install first", "install again", "removal")
        triggers = ("none", "the rename fails", "the reload fails")
        members = [
            (place, state, op, trigger)
            for place in places
            for state in states
            for op in ops
            for trigger in triggers
        ]
        # A removal whose rename fails in a writable directory exits after its writes and, as
        # SPEC-127 says, promises no message on every exit of the removal.
        members = [
            m
            for m in members
            if not (m[1] == "writable" and m[2:] == ("removal", "the rename fails"))
        ]
        for (layout, target), state, op, trigger in examined("directory member(s)", members):
            with self.subTest(layout=layout, target=target, state=state, op=op, trigger=trigger):
                w = self.fresh_world()
                cfdir = w.tmp / "host" / "etc" / "cfdir" if layout == "apart" else w.caddy_dir
                cfdir.mkdir(exist_ok=True)
                caddyfile = cfdir / "Caddyfile"
                caddyfile.write_text("example.org {\n\trespond 200\n}\n", encoding="utf-8")
                w.ship("v1.0.0")
                setting = {"DECKSTREAK_DEPLOY_CADDYFILE": str(caddyfile)}
                if op != "install first":
                    self.ok(w.run(DEPLOY, "caddy-install", "v1.0.0", **self.config(), **setting))
                if trigger == "the rename fails":
                    self.failing_rename(".candidate")
                if trigger == "the reload fails":
                    (w.log / "caddy-reload-fails").write_text("1")
                where = {"shared": cfdir, "caddy-dir": w.caddy_dir, "caddyfile-dir": cfdir}
                directory = where.get(target, w.tmp / target)
                looked = [where.get(t, w.tmp / t) for place, t in places if place == layout]
                directory.mkdir(parents=True, exist_ok=True)
                if state.endswith("stale writable copy"):
                    for name in stales.get(target, ()):
                        (directory / name).write_text("stale\n", encoding="utf-8")
                if state != "writable":
                    directory.chmod(0o555)
                script, args, refusal = (
                    (ROLLBACK, ("caddy-remove",), self.UNWRITTEN)
                    if op == "removal"
                    else (DEPLOY, ("caddy-install", "v1.0.0"), self.REFUSED)
                )
                before = self.listing(*looked)
                bytes_before = caddyfile.read_bytes()
                try:
                    done = w.run(script, *args, **self.config(host="new.example.org"), **setting)
                except subprocess.TimeoutExpired:
                    directory.chmod(0o755)
                    self.fail(f"{script.name} waited")
                after = self.listing(*looked)
                directory.chmod(0o755)
                if done.returncode == 0 and state == "writable":
                    continue
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                said = refusal in done.stderr or (
                    state == "writable" and "the Caddy reload failed" in done.stderr
                )
                self.assertTrue(said, f"no refusal line: {done.stderr!r}")
                self.assertEqual(after, before, "a directory changed: something was written")
                self.assertTrue(caddyfile.is_file() and not caddyfile.is_symlink())
                self.assertEqual(caddyfile.read_bytes(), bytes_before, "the live Caddyfile changed")

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


class ATemporaryPathThatCannotBeMadeIsANamedRefusal(Case):
    """Every temporary path `deploy.sh` makes (SPEC-127 A40; ADR-297, #451). The sites are read
    from the script itself, so a new `mktemp` joins the population by itself, and each is run
    through every verb that reaches it with each way the path can fail to be made."""

    # The step each site belongs to, named in its refusal; the key is the function that holds
    # the call, or `host` for the script that runs on the host.
    STEPS = {
        "install_tag": "release step",
        "caddy_install": "Caddy step",
        "host": "host step",
    }
    # The verbs that reach a site, each set up by `prepare`.
    VERBS = {
        "install_tag": ("install", "rollback-unkept"),
        "caddy_install": ("caddy-install",),
        "host": ("install", "rollback-unkept", "rollback-kept"),
    }
    MODES = ("absent", "unwritable", "unrunnable")
    HOST_SIDE = r"""#!/bin/bash
echo "host" >> "$STUB_LOG/host.log"
export STUB_SIDE=host
[ -z "${HOST_TMPDIR-}" ] || export TMPDIR=$HOST_TMPDIR
exec "$@"
"""
    MKTEMP = r"""#!/bin/bash
[ "${STUB_SIDE:-local}" != "${MKTEMP_FAILS-}" ] || { echo "mktemp: cannot be run" >&2; exit 126; }
exec /usr/bin/mktemp "$@"
"""

    @staticmethod
    def sites():
        """Each `mktemp` call of the script, as (holder, line number), holder being the function
        that contains it or `host` for the host script."""
        found, holder, in_host = [], None, False
        for number, line in enumerate(DEPLOY.read_text(encoding="utf-8").splitlines(), 1):
            if line.startswith("read -r -d '' HOST_SCRIPT"):
                in_host = True
            elif line == "HOSTEOF":
                in_host = False
            named = re.match(r"(\w+)\(\) \{", line)
            if named and not in_host:
                holder = named.group(1)
            if not line.lstrip().startswith("#") and re.search(r"\bmktemp\b", line):
                found.append(("host" if in_host else holder, number))
        return found

    @staticmethod
    def snapshot(w):
        """Each path of the world a step could write, with its type, mode and bytes or target:
        the host, the release assets, the checkout's files and the temporary directories."""
        seen = {}
        for path in sorted(w.tmp.rglob("*")):
            relative = path.relative_to(w.tmp)
            top = relative.parts[0]
            if top in {"log", "stub", "other", "origin.git"} or relative.parts[:2] == (
                "checkout",
                ".git",
            ):
                continue
            info = path.lstat()
            data = None
            if stat.S_ISLNK(info.st_mode):
                data = os.readlink(path)
            elif stat.S_ISREG(info.st_mode):
                data = sha(path.read_bytes())
            seen[relative] = (stat.S_IFMT(info.st_mode), stat.S_IMODE(info.st_mode), data)
        return seen

    def prepare(self, w, verb, good):
        """The world a verb starts from, and the argv it runs."""
        w.script("host", self.HOST_SIDE)
        w.script("mktemp", self.MKTEMP)
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0", **good))
        if verb == "rollback-kept":
            w.ship("v1.1.0")
            self.ok(w.deploy("v1.1.0", **good))
            return [ROLLBACK, "v1.0.0"]
        if verb == "caddy-install":
            (w.caddy_dir / "Caddyfile").write_text("example.org {\n\trespond 200\n}\n")
            path = w.tmp / "caddy-config.json"
            path.write_text(
                json.dumps(
                    {
                        "host": "app.example.org",
                        "web_root": str(w.root / "current/web"),
                        "api_upstream": "127.0.0.1:8080",
                    }
                ),
                encoding="utf-8",
            )
            good["DECKSTREAK_DEPLOY_CADDY_CONFIG"] = str(path)
            return [DEPLOY, "caddy-install", "v1.0.0"]
        w.ship("v1.1.0")
        return [ROLLBACK, "v1.1.0"] if verb == "rollback-unkept" else [DEPLOY, "v1.1.0"]

    def member(self, site, verb, mode):
        with tempfile.TemporaryDirectory() as tmp:
            w = World(tmp)
            good_dir = w.tmp / "tmp-good"
            good_dir.mkdir()
            good = {"TMPDIR": str(good_dir), "HOST_TMPDIR": str(good_dir)}
            argv = self.prepare(w, verb, good)
            planted = dict(good)
            side = "host" if site == "host" else "local"
            if mode == "absent":
                planted["HOST_TMPDIR" if side == "host" else "TMPDIR"] = str(
                    w.tmp / "tmp-absent" / "nothing"
                )
            elif mode == "unwritable":
                locked = w.tmp / "tmp-locked"
                locked.mkdir()
                locked.chmod(0o500)
                self.assertFalse(os.access(locked, os.W_OK), "the locked directory is writable")
                planted["HOST_TMPDIR" if side == "host" else "TMPDIR"] = str(locked)
            else:
                planted["MKTEMP_FAILS"] = side
            before = self.snapshot(w)
            try:
                done = w.run(*argv, **planted)
                after = self.snapshot(w)
            finally:
                if mode == "unwritable":
                    locked.chmod(0o700)
        label = f"{site} / {verb} / {mode}"
        lines = [ln for ln in done.stderr.splitlines() if ln.strip()]
        refusals = [ln for ln in lines if ln.startswith("deploy:")]
        self.assertNotEqual(done.returncode, 0, f"{label}: the verb went on: {done.stderr}")
        self.assertEqual(len(refusals), 1, f"{label}: {done.stderr!r}")
        self.assertEqual(lines[-1], refusals[0], f"{label}: the last line is not the refusal")
        self.assertIn(self.STEPS[site], refusals[0], f"{label}: the step is not named")
        self.assertNotIn("Traceback", done.stderr, label)
        self.assertEqual(after, before, f"{label}: a path changed")

    def test_every_temporary_path_that_cannot_be_made_is_a_named_refusal(self):
        found = self.sites()
        self.assertEqual(
            sorted(holder for holder, _ in found),
            sorted(self.STEPS),
            f"the script's temporary-path sites are not the ones this test covers: {found}",
        )
        members = examined(
            "temporary-path member(s)",
            [
                (site, verb, mode)
                for site in self.STEPS
                for verb in self.VERBS[site]
                for mode in self.MODES
            ],
        )
        for site, verb, mode in members:
            with self.subTest(site=site, verb=verb, mode=mode):
                self.member(site, verb, mode)


class EveryDirectoryAVerbWritesInIsMeasuredAndItsTemporaryPathsAreRefused(Case):
    """The places each verb writes in, measured from a real run, each made to fail in turn
    (SPEC-127 A41; ADR-297, #451). The release, the release's rollback and the rollback of a kept
    release are each run once in a fixture, and the directories whose contents changed are the
    population; then every place is made absent and unwritable, and every call the host step
    makes to a tool that makes a path is made to fail, one at a time. A member holds when the
    verb ends non-zero with one `deploy:` line, last, and no path of the world changed."""

    VERBS = ("install", "rollback-unkept", "rollback-kept")
    TOOLS = ("mktemp", "mkdir", "tar", "ln", "mv")
    HOST_SIDE = ATemporaryPathThatCannotBeMadeIsANamedRefusal.HOST_SIDE
    TOOL = r"""#!/bin/bash
side=${STUB_SIDE:-local}
n=$(grep -c "^@NAME@ $side$" "$STUB_LOG/tools.log" 2>/dev/null || true)
echo "@NAME@ $side" >> "$STUB_LOG/tools.log"
[ "${TOOL_FAILS-}" != "@NAME@:$side:$((n + 1))" ] || { echo "@NAME@: cannot be run" >&2; exit 126; }
exec /usr/bin/@NAME@ "$@"
"""
    # The calls each verb makes to a tool that makes a path, by side: what a run of the verb is
    # known to do, so that a call that appears or goes is seen here.
    CALLS = {
        "install": {
            ("mktemp", "local"): 1,
            ("mktemp", "host"): 1,
            ("mkdir", "host"): 2,
            ("tar", "host"): 2,
            ("ln", "host"): 1,
            ("mv", "host"): 2,
        },
        "rollback-unkept": {
            ("mktemp", "local"): 1,
            ("mktemp", "host"): 1,
            ("mkdir", "host"): 2,
            ("tar", "host"): 2,
            ("ln", "host"): 1,
            ("mv", "host"): 2,
        },
        "rollback-kept": {
            ("mktemp", "host"): 1,
            ("tar", "host"): 1,
            ("ln", "host"): 1,
            ("mv", "host"): 1,
        },
    }
    # The temporary directory and the four places the host writes in, then the drop-in directories.
    BASE_PLACES = ("tmpdir", "host/etc/systemd/system", "host/usr/local/lib/deck-streak")

    @staticmethod
    def good(w):
        directory = w.tmp / "tmpdir"
        directory.mkdir()
        return {"TMPDIR": str(directory), "HOST_TMPDIR": str(directory)}

    def prepare(self, w, verb, good):
        w.script("host", self.HOST_SIDE)
        for name in self.TOOLS:
            w.script(name, self.TOOL.replace("@NAME@", name))
        (w.log / "tools.log").write_text("", encoding="utf-8")
        w.ship("v1.0.0")
        self.ok(w.deploy("v1.0.0", **good))
        if verb == "rollback-kept":
            w.ship("v1.1.0")
            self.ok(w.deploy("v1.1.0", **good))
            argv = [ROLLBACK, "v1.0.0"]
        else:
            w.ship("v1.1.0")
            argv = [DEPLOY, "v1.1.0"] if verb == "install" else [ROLLBACK, "v1.1.0"]
        (w.log / "tools.log").write_text("", encoding="utf-8")
        return argv

    @staticmethod
    def stamps(w):
        """Each path of the world with its type, mode, bytes or target, and a directory's
        modification time, which moves when a path is made or removed in it."""
        seen = {}
        for path in sorted(w.tmp.rglob("*")):
            relative = path.relative_to(w.tmp)
            if relative.parts[0] in {"log", "stub", "other", "origin.git"} or relative.parts[
                :2
            ] == (
                "checkout",
                ".git",
            ):
                continue
            info = path.lstat()
            data = None
            if stat.S_ISLNK(info.st_mode):
                data = os.readlink(path)
            elif stat.S_ISREG(info.st_mode):
                data = sha(path.read_bytes())
            elif stat.S_ISDIR(info.st_mode):
                data = info.st_mtime_ns
            seen[relative] = (stat.S_IFMT(info.st_mode), stat.S_IMODE(info.st_mode), data)
        return seen

    def measure(self, verb):
        """(the places the verb wrote in, the calls it made to each tool), from one real run."""
        with tempfile.TemporaryDirectory() as tmp:
            w = World(tmp)
            good = self.good(w)
            argv = self.prepare(w, verb, good)
            before = self.stamps(w)
            self.ok(w.run(*argv, **good))
            after = self.stamps(w)
            calls = {}
            for line in (w.log / "tools.log").read_text(encoding="utf-8").splitlines():
                tool, side = line.split()
                calls[(tool, side)] = calls.get((tool, side), 0) + 1
        changed = {path for path in before if before[path] != after.get(path)}
        places = set()
        for path in changed:
            if stat.S_ISDIR(before[path][0]):
                places.add(path)
            elif path.parent in before:
                places.add(path.parent)
        expected = {Path(name) for name in self.BASE_PLACES}
        expected |= {
            path
            for path, (kind, _, _) in before.items()
            if kind == stat.S_IFDIR
            and path.parent == Path("host/etc/systemd/system")
            and "@" in path.name
            and path.name.endswith(".d")
        }
        if verb != "rollback-kept":
            expected.add(Path("host/usr/local/lib/deck-streak/releases"))
        return sorted(places), sorted(expected), calls

    @staticmethod
    def restore(modes):
        for path, mode in modes:
            path.chmod(mode)

    def outcome(self, w, argv, planted, modes):
        before = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
        try:
            done = w.run(*argv, **planted)
            after = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
        finally:
            self.restore(modes)
        return done, before, after

    def judge(self, label, done, before, after, step):
        lines = [ln for ln in done.stderr.splitlines() if ln.strip()]
        refusals = [ln for ln in lines if ln.startswith("deploy:")]
        self.assertNotEqual(done.returncode, 0, f"{label}: the verb went on: {done.stderr}")
        self.assertEqual(len(refusals), 1, f"{label}: {done.stderr!r}")
        self.assertEqual(lines[-1], refusals[0], f"{label}: the last line is not the refusal")
        if step:
            self.assertIn(step, refusals[0], f"{label}: the step is not named")
        self.assertNotIn("Traceback", done.stderr, label)
        self.assertEqual(after, before, f"{label}: a path changed")

    def place_member(self, verb, place, mode):
        with tempfile.TemporaryDirectory() as tmp:
            w = World(tmp)
            good = self.good(w)
            argv = self.prepare(w, verb, good)
            path = w.tmp / place
            modes = []
            if mode == "unwritable":
                modes.append((path, stat.S_IMODE(path.lstat().st_mode)))
                path.chmod(0o555)
                self.assertFalse(os.access(path, os.W_OK), f"{place} is writable")
            else:
                aside = w.tmp / "aside"
                aside.mkdir()
                parent = path.parent
                modes.append((parent, stat.S_IMODE(parent.lstat().st_mode)))
                os.rename(path, aside / "moved")
                parent.chmod(0o555)
            step = None
            if str(place) == "tmpdir":
                step = "host step" if verb == "rollback-kept" else "release step"
            elif mode == "unwritable":
                step = "host step"
            done, before, after = self.outcome(w, argv, good, modes)
            self.judge(f"{verb} / {place} / {mode}", done, before, after, step)

    def tool_member(self, verb, tool, side, index):
        with tempfile.TemporaryDirectory() as tmp:
            w = World(tmp)
            good = self.good(w)
            argv = self.prepare(w, verb, good)
            planted = {**good, "TOOL_FAILS": f"{tool}:{side}:{index}"}
            done, before, after = self.outcome(w, argv, planted, [])
            step = "host step" if side == "host" else ("release step" if tool == "mktemp" else None)
            self.judge(f"{verb} / {tool} {side} call {index}", done, before, after, step)

    def confined(self, verb, places):
        """The verb run with every directory but its places read-only: it must still succeed."""
        with tempfile.TemporaryDirectory() as tmp:
            w = World(tmp)
            good = self.good(w)
            argv = self.prepare(w, verb, good)
            keep = {Path(place) for place in places}
            modes = []
            for path in sorted(w.tmp.rglob("*")):
                relative = path.relative_to(w.tmp)
                if relative.parts[0] in {"log", "stub", "other", "origin.git"} or relative.parts[
                    :2
                ] == ("checkout", ".git"):
                    continue
                if path.is_dir() and not path.is_symlink() and relative not in keep:
                    modes.append((path, stat.S_IMODE(path.lstat().st_mode)))
            try:
                for path, _ in reversed(modes):
                    path.chmod(0o555)
                done = w.run(*argv, **good)
            finally:
                self.restore(modes)
            self.ok(done)

    def test_every_place_a_verb_writes_in_and_every_temporary_path_call_is_refused(self):
        members, measured_total, derived_total = [], 0, 0
        for verb in self.VERBS:
            places, expected, calls = self.measure(verb)
            self.assertEqual(places, expected, f"{verb}: the places it wrote in")
            self.assertEqual(calls, self.CALLS[verb], f"{verb}: its calls to path-making tools")
            self.confined(verb, places)
            measured_total += 2 * len(places) + sum(calls.values())
            derived_total += 2 * len(expected) + sum(self.CALLS[verb].values())
            for place in places:
                for mode in ("absent", "unwritable"):
                    members.append(("place", verb, place, mode))
            for (tool, side), count in sorted(calls.items()):
                for index in range(1, count + 1):
                    members.append(("tool", verb, tool, side, index))
        self.assertEqual(len(members), measured_total)
        self.assertEqual(len(members), derived_total)
        for member in examined("measured temporary-path member(s)", members):
            with self.subTest(member=member):
                if member[0] == "place":
                    self.place_member(*member[1:])
                else:
                    self.tool_member(*member[1:])


_A41 = EveryDirectoryAVerbWritesInIsMeasuredAndItsTemporaryPathsAreRefused


class EveryStateAVerbStartsFromAndEveryToolTheHostStepWritesWithIsRefused(Case):
    """The host step's tools are read from the script and its start states are generated
    (SPEC-127 A42; ADR-297, #451). The tool set is every command word of the host body that can
    write a path, read by the parser below, so a new tool joins the population or turns the census
    red; the states are the rows of STATE_TABLE, which holds at least FLOOR, so a new state joins
    by itself. Each verb is run from each
    state that applies, and every host call that writes a path is made to fail in turn. A member
    holds when the verb ends non-zero with one `deploy:` line, last, and no path of the world
    that the snapshot covers changed."""

    VERBS = _A41.VERBS
    # The start states, one registry: name -> (the world it is built on, the change made to that
    # world, whether a run from it refuses with no call failed). situation() reads only this table,
    # so a state added here joins every verb by itself (verify-451-r3, the STATES AXIS rule).
    STATE_TABLE = {
        "installed": ("installed", None, False),
        "first-install": ("first", None, False),
        "current-absent": ("installed", "drop_current", False),
        "new-unit": ("installed", "unship_the_new_unit", False),
        "not-ready": ("installed", "make_unready", True),
        "not-ready-first": ("first", "make_unready", True),
        "same-tag": ("installed", "plant_the_same_tag", True),
        "stale-partial": ("installed", "plant_a_stale_partial", False),
        "undeletable-partial": ("installed", "plant_an_undeletable_partial", True),
        "partly-deletable-partial": ("installed", "plant_a_partly_deletable_partial", True),
        "dropin-unwritable": ("installed", "lock_a_drop_in", True),
        "units-absent": ("installed", "drop_the_unit_directory", True),
        "over-keep": ("installed", "keep_more_than_keep", False),
        "first-install-parent-unwritable": ("first", "lock_the_root_parent", True),
        "corrupt-manifest": ("installed", "corrupt_the_manifest", True),
        "effective-refused": ("installed", "refuse_the_effective_check", True),
        "check-file-unwritable": ("installed", "lock_the_check_file", True),
        "over-keep-partly-deletable": ("installed", "keep_more_over_a_partly_deletable", False),
    }
    STATES = tuple(STATE_TABLE)
    # The states the class names. Each must be a row above, so a dropped state turns the test red.
    FLOOR = frozenset(
        {"installed", "first-install", "current-absent", "new-unit", "not-ready", "not-ready-first"}
        | {"same-tag", "stale-partial", "undeletable-partial", "partly-deletable-partial"}
        | {"dropin-unwritable", "units-absent", "over-keep", "first-install-parent-unwritable"}
        | {"corrupt-manifest", "effective-refused"}
    )
    # A pair left out names why it cannot exist; never a pair that is only hard to build.
    NOT_A_STATE = {
        ("rollback-kept", "first-install"): "a first install holds no kept release",
        ("rollback-kept", "not-ready-first"): "a first install holds no kept release",
        ("rollback-kept", "first-install-parent-unwritable"): "a first install keeps nothing",
        ("rollback-kept", "same-tag"): "the kept verb's tag is always present",
        ("rollback-kept", "stale-partial"): "the kept verb unpacks nothing",
        ("rollback-kept", "undeletable-partial"): "the kept verb unpacks nothing",
        ("rollback-kept", "partly-deletable-partial"): "the kept verb unpacks nothing",
        (
            "rollback-kept",
            "corrupt-manifest",
        ): "the kept pair ships v1.1.0 without the fresh-manifest options, so they never apply to it",
        (
            "rollback-kept",
            "effective-refused",
        ): "a kept release passed the check it was installed by",
        ("rollback-kept", "over-keep-partly-deletable"): "the kept verb prunes nothing",
    }
    # Words the host body uses that write no path. A word not listed here and not a function of the
    # body is taken to be a tool that writes, which is how a new tool turns the census red.
    READERS = frozenset(
        {"[", "echo", "read", "readlink", "basename", "dirname", "sed", "tail", "grep", "sort"}
        | {"sha256sum", "systemctl", "python3", "curl", "sleep", "cd", "test", "printf"}
    )
    BUILTINS = frozenset(
        {"set", "trap", "local", "return", "exit", "break", "continue", "shift", "for", "case", ":"}
        | {"true", "false"}
    )
    KEYWORDS = frozenset(
        {"if", "then", "else", "elif", "fi", "do", "done", "while", "until", "{", "}", "!", "esac"}
    )
    SYSTEMCTL_READS_AND_RELOADS = frozenset({"daemon-reload", "restart", "cat"})
    FIND_WRITES = frozenset(
        {"-delete", "-exec", "-execdir", "-ok", "-okdir", "-fprint", "-fprint0", "-fprintf", "-fls"}
    )
    # The operators the census reads; any other operator token is an unknown word.
    WRITE_REDIRECTIONS = frozenset({">", ">>", ">|", "&>", "&>>", "<>"})
    READ_REDIRECTIONS = frozenset({"<", "<<<"})
    # The options each reader may carry; any other option is an unknown word, so a new spelling of
    # a write turns the census red instead of passing as a read.
    OPTIONS = {
        "sed": frozenset({"-n"}),
        "sort": frozenset({"-V", "-r"}),
        "curl": frozenset({"-fsS", "--max-time"}),
        "grep": frozenset({"-e", "-v", "-vxF"}),
        "tail": frozenset({"-n"}),
        "sha256sum": frozenset({"-c", "--quiet"}),
        "find": frozenset({"-maxdepth", "-mindepth", "-printf", "-type", "-writable"}),
    }
    UNKNOWN = "<unknown>"
    # The tools the members run. A writer the body holds that no state reaches is a census failure,
    # never an exclusion.
    HANDLED = ("mktemp", "mkdir", "tar", "ln", "mv", "install", "find", "rm")
    EXTRA = {f"{SYSTEMD}/deck-streak-extra.timer": b"[Timer]\nOnCalendar=daily\n"}
    BAD_BOT = {f"{SYSTEMD}/{BOT}": unit_text(BOT, "v1.1.0") + b"StandardInputText=x\n"}
    TOOL = r"""#!/bin/bash
side=${STUB_SIDE:-local}
n=$(grep -c "^@NAME@ $side$" "$STUB_LOG/tools.log" 2>/dev/null || true)
echo "@NAME@ $side" >> "$STUB_LOG/tools.log"
printf '%s\t%s\t%s\t%s\n' "@NAME@" "$side" "$((n + 1))" "$*" >> "$STUB_LOG/args.log"
case ",${TOOL_FAILS-}," in
*",@NAME@:$side:$((n + 1)),"*) echo "@NAME@: cannot be run" >&2; exit 126;;
esac
if [ "@NAME@" = mktemp ] && [ "$side" = host ] && [ -n "${CHECK_FILE_RO-}" ]; then
    made=$(/usr/bin/mktemp "$@") && chmod 0444 "$made" && echo "$made"
    exit
fi
exec /usr/bin/@NAME@ "$@"
"""
    HOST_SIDE = _A41.HOST_SIDE
    good = staticmethod(_A41.good)
    restore = staticmethod(_A41.restore)
    outcome = _A41.outcome
    judge = _A41.judge

    @staticmethod
    def host_body(text=None):
        """The host script that `deploy.sh` holds in `HOST_SCRIPT`."""
        text = DEPLOY.read_text(encoding="utf-8") if text is None else text
        found = re.search(r"HOST_SCRIPT <<'HOSTEOF'[^\n]*\n(.*?)\nHOSTEOF\n", text, re.S)
        assert found, "deploy.sh holds no HOST_SCRIPT here-document"
        return found.group(1)

    @classmethod
    def census(cls, body):
        """(every command as [word, *arguments], the targets of every redirection that can write a
        path, the functions the body defines), read from the body's text alone."""
        functions = set(re.findall(r"^\s*(\w+)\(\) \{", body, re.M))
        lines = [ln for ln in body.splitlines() if not ln.lstrip().startswith("#")]
        text = re.sub(r"\$\(\([^)]*\)\)", "0", " ; ".join(lines))
        commands, targets = [], []

        def substitutions(token):
            """The bodies of every `$(...)` in a word, innermost first; None when one cannot be
            read (a body holding `case`, or parentheses the reader cannot balance)."""
            bodies, rest = [], token
            while True:
                found = re.search(r"\$\(([^()]*)\)", rest)
                if not found:
                    break
                if re.search(r"(^|\s)case(\s|$)", found.group(1)):
                    return None
                bodies.append(found.group(1))
                rest = rest[: found.start()] + "0" + rest[found.end() :]
            return None if "$(" in rest else bodies

        def read(chunk):
            lex = shlex.shlex(chunk, posix=True, punctuation_chars=True)
            lex.whitespace_split, lex.commenters = True, ""
            tokens, nested = list(lex), []
            for token in tokens:
                bodies = substitutions(token)
                if bodies is None or "`" in token:
                    commands.append([cls.UNKNOWN, token])
                else:
                    nested.extend(bodies)
            current, start, index = [], True, 0
            while index < len(tokens):
                token = tokens[index]
                index += 1
                if set(token) <= set("();&|"):
                    start = True
                    continue
                if set(token) <= set("()<>&|"):
                    following = tokens[index] if index < len(tokens) else ""
                    if token in ("<(", ">(") or following == "(":
                        start = True
                    elif token in cls.WRITE_REDIRECTIONS:
                        if following != "/dev/null":
                            targets.append(following)
                        index += 1
                    elif token == ">&":
                        if not re.fullmatch(r"\d+|-", following):
                            targets.append(following)
                        index += 1
                    elif token in cls.READ_REDIRECTIONS:
                        index += 1
                    else:
                        commands.append([cls.UNKNOWN, token])
                    continue
                if start and (token in cls.KEYWORDS or re.match(r"^\w+=", token)):
                    continue
                if start:
                    current = [token]
                    commands.append(current)
                    start = False
                else:
                    current.append(token)
            for inner in nested:
                read(inner)

        read(text)
        for word, *args in list(commands):
            if word == "trap" and args and args[0] not in functions and args[0] != "-":
                read(args[0])
        return commands, targets, functions

    @classmethod
    def writers(cls, commands, functions):
        """The tools among the commands that can write a path, each with how many command sites."""
        found = {}
        for word, *args in commands:
            if word in functions or word in cls.BUILTINS:
                continue
            if word in cls.READERS or word == "find":
                writes = (
                    (word == "sed" and any(a.startswith("-i") or a == "--in-place" for a in args))
                    or (
                        word == "sed"
                        and any(re.search(r"(^|[;}\s])w\s|/[gpIi0-9]*w\s", a) for a in args)
                    )
                    or (word == "sort" and any(a == "-o" or a.startswith("--output") for a in args))
                    or (word == "curl" and any(a in ("-o", "-O", "--output") for a in args))
                    or (
                        word == "systemctl"
                        and (args[:1] or [""])[0] not in cls.SYSTEMCTL_READS_AND_RELOADS
                    )
                    or (
                        word == "python3"
                        and not (args[:1] or [""])[0].endswith("effective-check.py")
                    )
                    or (word == "find" and bool(cls.FIND_WRITES & set(args)))
                )
                if not writes and any(
                    a.startswith("-") and a not in cls.OPTIONS.get(word, frozenset()) | {"-"}
                    for a in args
                    if word in cls.OPTIONS
                ):
                    found[cls.UNKNOWN] = found.get(cls.UNKNOWN, 0) + 1
                    continue
                if not writes:
                    continue
            found[word] = found.get(word, 0) + 1
        return found

    # The figures the census prints, each pinned beside an independent reading of the same text.
    COMMAND_SITES = 252
    REDIRECTION_TARGETS = 1

    @classmethod
    def derived_writers(cls, body):
        """The writing command sites by tool, counted by a pattern over the body's lines: a second
        reading that shares no code with census(), so the two cannot drift together."""
        text = "\n".join(ln for ln in body.splitlines() if not ln.lstrip().startswith("#"))
        start = r"(?:^|[;&|({!]|\bthen|\bdo|\belse|\$\()\s*"
        found = {}
        for tool in sorted(cls.HANDLED):
            found[tool] = len(re.findall(f"{start}{tool}[\\s)]", text, re.M))
            if tool == "find":
                found[tool] -= len(re.findall(f"{start}find\\s[^\n]*-printf", text, re.M))
        return found

    def test_the_tools_the_host_body_writes_with_are_read_from_the_script(self):
        commands, targets, functions = self.census(self.host_body())
        found = self.writers(commands, functions)
        self.assertEqual(
            len(examined("host command site(s)", commands)), self.COMMAND_SITES, "the sites read"
        )
        self.assertEqual(
            len(examined("host redirection target(s)", targets)),
            self.REDIRECTION_TARGETS,
            "the redirections read",
        )
        self.assertEqual(
            found, self.derived_writers(self.host_body()), "the census and an independent reading"
        )
        self.assertEqual(sorted(found), sorted(self.HANDLED), "the host body's writing tools")
        self.assertEqual(
            sorted(set(targets)),
            ["$checked"],
            "a redirection writes a path other than the check file",
        )
        self.assertEqual(len(targets), 1, "the redirections into the check file")
        self.assertEqual(
            found,
            {"mktemp": 1, "find": 1, "mkdir": 2, "ln": 1, "install": 3, "mv": 4}
            | {"rm": 2, "tar": 3},
            "the command sites of the host body that can write a path, by tool",
        )
        self.assertEqual(sum(found.values()), 17, "the command sites that can write a path")

    def test_a_tool_the_census_does_not_know_turns_it_red(self):
        body = self.host_body()
        planted = {
            "a new tool": body.replace("set -eu", "set -eu\ntouch /x", 1),
            "an in-place edit": body.replace("set -eu", "set -eu\nsed -i s/a/b/ /x", 1),
            "a new redirection": body.replace("set -eu", "set -eu\necho a > /x", 1),
            "a tool in a substitution": body.replace("set -eu", "set -eu\nq=$(tee /x)", 1),
        }
        # The 26 spellings the round-3 census neither counted nor refused (verify-451-r3 R3-3).
        escaped = {
            "redirection >|": "echo a >| /x",
            "redirection &>": "echo a &> /x",
            "redirection &>>": "echo a &>> /x",
            "redirection >&": "echo a >& /x",
            "redirection <> (creates)": "read -r q <> /x",
            "find -fprint0": "find /a -fprint0 /x",
            "find -okdir": "find /a -okdir rm {} ;",
            "sed -Ei": "sed -Ei s/a/b/ /x",
            "sed -ri": "sed -ri s/a/b/ /x",
            "sed --in-place=.b": "sed --in-place=.b s/a/b/ /x",
            "sed w command": "sed -n 'w /x' /a",
            "sed s///w flag": "sed 's/a/b/w /x' /a",
            "$( (subshell) ) quoted": 'q="$( (touch /x) )"',
            "$(case) quoted": 'q="$(case a in a) touch /x ;; esac)"',
            "backticks quoted": 'q="`touch /x`"',
            "backticks as an argument": 'echo "`touch /x`"',
            "process substitution >(..)": "echo a >(touch /x)",
            "process substitution <(..) reader": "sort <(touch /x)",
            "trap string": "trap 'touch /x' EXIT",
            "curl -so": "curl -so /x http://a",
            "curl -o/x": "curl -o/x http://a",
            "curl --output=/x": "curl --output=/x http://a",
            "curl -D": "curl -D /x http://a",
            "curl -c (cookie jar)": "curl -c /x http://a",
            "sort -o/x": "sort -o/x /a",
            "sort -uo": "sort -uo /x /a",
        }
        planted |= {
            name: body.replace("set -eu", f"set -eu\n{line}", 1) for name, line in escaped.items()
        }
        commands, targets, functions = self.census(body)
        base = self.writers(commands, functions)
        self.assertEqual(len(planted), 4 + 26, "the planted bodies: the four and the 26 spellings")
        for name, text in examined("planted host bodies", sorted(planted.items())):
            with self.subTest(planted=name):
                commands, targets, functions = self.census(text)
                found = self.writers(commands, functions)
                self.assertTrue(
                    found != base or len(targets) != 2 or set(targets) != {"$checked"},
                    f"{name}: the census did not change",
                )

    def arm(self, w):
        w.script("host", self.HOST_SIDE)
        for name in self.HANDLED:
            w.script(name, self.TOOL.replace("@NAME@", name))
        for name in ("tools.log", "args.log"):
            (w.log / name).write_text("", encoding="utf-8")

    def changes(self):
        """The change methods of STATE_TABLE, each under the name the table holds, so a state's
        change is found by a lookup in this one mapping and never by a name read from data."""
        return {
            "drop_current": self.drop_current,
            "unship_the_new_unit": self.unship_the_new_unit,
            "make_unready": self.make_unready,
            "plant_the_same_tag": self.plant_the_same_tag,
            "plant_a_stale_partial": self.plant_a_stale_partial,
            "plant_an_undeletable_partial": self.plant_an_undeletable_partial,
            "plant_a_partly_deletable_partial": self.plant_a_partly_deletable_partial,
            "lock_a_drop_in": self.lock_a_drop_in,
            "drop_the_unit_directory": self.drop_the_unit_directory,
            "keep_more_than_keep": self.keep_more_than_keep,
            "lock_the_root_parent": self.lock_the_root_parent,
            "corrupt_the_manifest": self.corrupt_the_manifest,
            "refuse_the_effective_check": self.refuse_the_effective_check,
            "lock_the_check_file": self.lock_the_check_file,
            "keep_more_over_a_partly_deletable": self.keep_more_over_a_partly_deletable,
        }

    def situation(self, tmp, verb, state):
        """A world in `state`, armed, and the argv of `verb` in it."""
        base, change, _ = self.STATE_TABLE[state]
        w = World(tmp)
        good = self.good(w)
        self.arm(w)
        extra = self.EXTRA if change == "unship_the_new_unit" else None
        fresh = {"extra": extra}
        if change == "corrupt_the_manifest":
            fresh["corrupt"] = True
        elif change == "refuse_the_effective_check":
            fresh["extra"] = self.BAD_BOT
        if base == "first":
            w.ship("v1.0.0")
            argv = [DEPLOY if verb == "install" else ROLLBACK, "v1.0.0"]
        else:
            w.ship("v1.0.0", extra=extra if verb == "rollback-kept" else None)
            self.ok(w.deploy("v1.0.0", **good))
            if verb == "rollback-kept":
                w.ship("v1.1.0")
                self.ok(w.deploy("v1.1.0", **good))
                argv = [ROLLBACK, "v1.0.0"]
            else:
                w.ship("v1.1.0", **fresh)
                argv = [DEPLOY if verb == "install" else ROLLBACK, "v1.1.0"]
        # The modes a change locks, which every run from this world restores when it ends.
        w.locked = self.changes()[change](w, good, argv) if change else []
        for name in ("tools.log", "args.log"):
            (w.log / name).write_text("", encoding="utf-8")
        return w, good, argv

    @staticmethod
    def drop_current(w, good, argv):
        (w.root / "current").unlink()
        return []

    @staticmethod
    def unship_the_new_unit(w, good, argv):
        (w.units / "deck-streak-extra.timer").unlink(missing_ok=True)
        return []

    @staticmethod
    def make_unready(w, good, argv):
        """The restart succeeds and readiness never arrives, so `back()` runs: its switch leg
        with a previous release, its `rm` leg on a first install."""
        w.script("curl", UNREADY_CURL)
        good["STUB_UNREADY_TAG"] = argv[-1]
        return []

    @staticmethod
    def plant_the_same_tag(w, good, argv):
        (w.root / "releases" / argv[-1]).mkdir(parents=True, exist_ok=True)
        return []

    @staticmethod
    def plant_a_stale_partial(w, good, argv):
        stale = w.root / "releases" / f"{argv[-1]}.partial"
        stale.mkdir(parents=True)
        (stale / "leftover").write_text("an interrupted unpack\n", encoding="utf-8")
        return []

    @staticmethod
    def plant_an_undeletable_partial(w, good, argv):
        keep = w.root / "releases" / f"{argv[-1]}.partial" / "keep"
        keep.mkdir(parents=True)
        (keep / "leftover").write_text("an interrupted unpack\n", encoding="utf-8")
        keep.chmod(0o555)
        return [(keep, 0o755)]

    @staticmethod
    def plant_a_partly_deletable_partial(w, good, argv):
        """A stale unpack whose first entry deletes and whose second does not, so a delete that
        is not refused first leaves it half gone."""
        stale = w.root / "releases" / f"{argv[-1]}.partial"
        for name in ("a", "keep"):
            (stale / name).mkdir(parents=True)
            (stale / name / "leftover").write_text("an interrupted unpack\n", encoding="utf-8")
        (stale / "keep").chmod(0o555)
        return [(stale / "keep", 0o755)]

    @staticmethod
    def lock_a_drop_in(w, good, argv):
        dropin = w.units / f"{JOB}@sync.service.d"
        mode = stat.S_IMODE(dropin.lstat().st_mode)
        dropin.chmod(0o555)
        return [(dropin, mode)]

    @staticmethod
    def drop_the_unit_directory(w, good, argv):
        w.units.rename(w.units.with_name(f"{w.units.name}.moved"))
        return []

    def keep_more_than_keep(self, w, good, argv):
        """A third release and a keep of two, so the prune of an older release runs."""
        w.ship("v1.0.1")
        self.ok(w.deploy("v1.0.1", **good))
        good["DECKSTREAK_DEPLOY_KEEP"] = "2"
        return []

    @staticmethod
    def corrupt_the_manifest(w, good, argv):
        """The release's MANIFEST.sha256 names a file with a wrong sum (built by situation())."""
        return []

    @staticmethod
    def refuse_the_effective_check(w, good, argv):
        """The release's bot unit carries a line the effective check refuses (built by situation())."""
        return []

    @staticmethod
    def lock_the_check_file(w, good, argv):
        """The check file is made read-only, so each redirection into it fails (R3-6)."""
        good["CHECK_FILE_RO"] = "1"
        return []

    def keep_more_over_a_partly_deletable(self, w, good, argv):
        """The over-keep world, with an older release whose first entry deletes and whose second
        does not, so a prune that is not guarded leaves it half gone (R3-2)."""
        self.keep_more_than_keep(w, good, argv)
        old = w.root / "releases" / "v1.0.0"
        for name in ("a", "keep"):
            (old / name).mkdir()
            (old / name / "leftover").write_text("an older release\n", encoding="utf-8")
        (old / "keep").chmod(0o555)
        return [(old / "keep", 0o755)]

    @staticmethod
    def lock_the_root_parent(w, good, argv):
        """A first install under a parent that cannot take a write (round 3's dropped W1 member)."""
        parent = w.root.parent
        parent.mkdir(parents=True, exist_ok=True)
        mode = stat.S_IMODE(parent.lstat().st_mode)
        parent.chmod(0o555)
        return [(parent, mode)]

    def writing_calls(self, w):
        """(tool, index) of each host call that writes a path, from the run's own log. The census's
        own classifier must count the same calls, so no filter here can drop one unseen."""
        seen, calls, logged = {}, [], []
        for line in (w.log / "args.log").read_text(encoding="utf-8").splitlines():
            tool, side, index, args = (line.split("\t") + [""])[:4]
            if side != "host":
                continue
            seen[tool] = seen.get(tool, 0) + 1
            logged.append([tool, *args.split()])
            if tool == "find" and not self.FIND_WRITES & set(args.split()):
                continue
            calls.append((tool, int(index)))
        counted = sum(
            1
            for ln in (w.log / "tools.log").read_text(encoding="utf-8").splitlines()
            if ln.endswith(" host")
        )
        self.assertEqual(sum(seen.values()), counted, "the two logs of the host calls")
        self.assertEqual(
            len(calls), sum(self.writers(logged, set()).values()), "the census and the log differ"
        )
        return calls

    def calls_of(self, verb, state):
        """The writing host calls one run of `verb` from `state` makes with no call failed; a
        state the table marks refusing is judged as its own member, not here."""
        with tempfile.TemporaryDirectory() as tmp:
            w, good, argv = self.situation(tmp, verb, state)
            try:
                done = w.run(*argv, **good)
            finally:
                self.restore(w.locked)
            if not self.STATE_TABLE[state][2]:
                self.ok(done)
            return self.writing_calls(w)

    def refuse_each(self, verb, state, calls):
        """Fail each of `calls` in turn, in a fresh world, and judge the run it ends (#451). A
        call of None is the run from a refusing state with no call failed, judged the same way."""
        for tool, index in examined("state-and-call member(s)", calls):
            with self.subTest(verb=verb, state=state, tool=tool, index=index):
                with tempfile.TemporaryDirectory() as tmp:
                    w, good, argv = self.situation(tmp, verb, state)
                    planted = dict(good)
                    if tool is not None:
                        planted["TOOL_FAILS"] = f"{tool}:host:{index}"
                    done, before, after = self.outcome(w, argv, planted, w.locked)
                    call = "no call failed" if tool is None else f"{tool} {index}"
                    step = "" if tool is None else "host step"
                    label = f"{verb} / {state} / {call}"
                    if done.returncode == 0 and self.switch_already_made(w, tool, index):
                        self.judge_finished(label, w, good, done, before, after)
                    else:
                        self.judge(label, done, before, after, step)
                        if tool == "install":
                            self.assertIn(
                                "could not install its unit files",
                                done.stderr.splitlines()[-1],
                                f"{label}: the refusal does not name the install",
                            )

    @staticmethod
    def switch_already_made(w, tool, index):
        """True when the run's own log holds the rename over `current` BEFORE the failed call, so
        the call belongs to the part of a run after its switch, the only part that may end 0. A run
        that ends 0 from a call before the switch went on past a failure and is judged strictly."""
        if tool is None:
            return False
        for line in (w.log / "args.log").read_text(encoding="utf-8").splitlines():
            name, side, number, args = (line.split("\t") + [""])[:4]
            if side != "host":
                continue
            if name == tool and int(number) == index:
                return False
            if name == "mv" and args.split()[-1:] and args.split()[-1].endswith("/current"):
                return True
        return False

    @staticmethod
    def half_deleted(release):
        """True when a release directory exists and some file its MANIFEST.sha256 names is gone or
        changed, so it is a release only in name."""
        if not release.is_dir():
            return False
        manifest = release / "MANIFEST.sha256"
        if not manifest.is_file():
            return True
        for line in manifest.read_text(encoding="utf-8").splitlines():
            digest, _, name = line.partition("  ")
            target = release / name
            if not target.is_file() or sha(target.read_bytes()) != digest:
                return True
        return False

    def judge_finished(self, label, w, good, done, before, after):
        """A run that already finished its switch ends 0, leaves no temporary path, and leaves no
        release that a later verb accepts while half deleted (the class rule, second arm)."""
        self.assertEqual(
            done.returncode, 0, f"{label}: a finished run ended non-zero: {done.stderr}"
        )
        self.assertIn(
            "is current", done.stdout, f"{label}: exit 0 without finishing: {done.stderr}"
        )
        for path in set(after) - set(before):
            temporary = path.name.startswith((".current.", ".stale.")) or path.name.endswith(
                (".partial", ".saved")
            )
            self.assertFalse(
                temporary or path.parts[0] == "tmpdir", f"{label}: a temporary path is left: {path}"
            )
        for release in sorted((w.root / "releases").iterdir()):
            if release.name.startswith(".") or not self.half_deleted(release):
                continue
            later = w.run(ROLLBACK, release.name, **good)
            self.assertNotEqual(
                later.returncode, 0, f"{label}: a later rollback took {release.name}"
            )
            self.assertIn(release.name, later.stderr, f"{label}: the refusal does not name it")

    def test_every_state_and_every_writing_call_of_the_host_step_is_refused(self):
        self.assertLessEqual(self.FLOOR, set(self.STATES), "a state the class names is missing")
        pairs = [(v, s) for v in self.VERBS for s in self.STATES if (v, s) not in self.NOT_A_STATE]
        self.assertLessEqual(
            set(self.NOT_A_STATE), {(v, s) for v in self.VERBS for s in self.STATES}, "a stray pair"
        )
        self.assertEqual(
            len(pairs),
            len(self.VERBS) * len(self.STATES) - len(self.NOT_A_STATE),
            "the pairs the table leaves",
        )
        members, reached, broke = [], set(), set()
        for verb, state in pairs:
            calls = None
            with self.subTest(verb=verb, state=state, baseline=True):
                calls = self.calls_of(verb, state)
            if calls is None:
                broke.add((verb, state))
                continue
            reached |= {tool for tool, _ in calls}
            if self.STATE_TABLE[state][2]:
                calls = [(None, 0), *calls]
            members += [(verb, state, tool, index) for tool, index in calls]
        self.assertEqual(
            sorted({(v, s) for v, s, _, _ in members}),
            sorted(set(pairs) - broke),
            "a pair with no member",
        )
        if not broke:
            self.assertEqual(sorted(reached), sorted(self.HANDLED), "the tools the runs reached")
        for verb, state, tool, index in examined("state-and-call member(s)", members):
            with self.subTest(verb=verb, state=state, tool=tool, index=index):
                self.refuse_each(verb, state, [(tool, index)])

    def test_a_run_that_finishes_leaves_no_temporary_path_and_no_half_deleted_release(self):
        pairs = [
            (v, s)
            for v in self.VERBS
            for s in self.STATES
            if (v, s) not in self.NOT_A_STATE and not self.STATE_TABLE[s][2]
        ]
        for verb, state in examined("finishing pair(s)", pairs):
            with self.subTest(verb=verb, state=state), tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, verb, state)
                done, before, after = self.outcome(w, argv, good, w.locked)
                self.judge_finished(f"{verb} / {state}", w, good, done, before, after)

    def test_an_install_over_an_installed_host_refuses_each_of_its_writing_calls(self):
        self.refuse_each("install", "installed", self.calls_of("install", "installed"))

    def test_a_rollback_to_a_kept_release_that_is_not_whole_is_refused_as_found(self):
        """The kept path's whole-release check: a file `MANIFEST.sha256` lists is gone, or the
        manifest itself is gone, and either way the rollback ends non-zero with one `deploy:` line,
        last, naming the release, and every path as found."""
        gone = {
            "a file the manifest lists": "web/index.html",
            "the manifest itself": "MANIFEST.sha256",
        }
        for what, name in examined("not-whole kept release member(s)", sorted(gone.items())):
            with self.subTest(gone=what), tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, "rollback-kept", "installed")
                kept = w.root / "releases" / argv[-1]
                (kept / name).unlink()
                done, before, after = self.outcome(w, argv, good, w.locked)
                self.judge(f"rollback-kept / {what} is gone", done, before, after, "")
                self.assertIn(
                    f"found {argv[-1]} is kept but is not a whole release",
                    done.stderr.splitlines()[-1],
                    f"{what}: the refusal does not name the whole-release check",
                )

    def test_a_first_install_refuses_each_of_its_writing_calls(self):
        self.refuse_each("install", "first-install", self.calls_of("install", "first-install"))

    def test_a_first_install_into_a_parent_that_cannot_take_a_write_leaves_the_host_as_found(self):
        for verb in examined("first-install place member(s)", ["install", "rollback-unkept"]):
            with self.subTest(verb=verb), tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, verb, "first-install")
                parent = w.root.parent
                parent.mkdir(parents=True, exist_ok=True)
                modes = [(parent, stat.S_IMODE(parent.lstat().st_mode))]
                parent.chmod(0o555)
                self.assertFalse(w.root.exists(), "the host holds no release yet")
                done, before, after = self.outcome(w, argv, good, modes)
                self.judge(f"{verb} / first install / parent", done, before, after, "host step")
                self.assertEqual(done.stderr.splitlines()[-1].split()[0], "deploy:", verb)

    def test_a_stale_unpack_in_a_releases_directory_that_cannot_take_a_write_is_refused(self):
        for verb in examined("stale-unpack member(s)", ["install", "rollback-unkept"]):
            with self.subTest(verb=verb), tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, verb, "installed")
                releases = w.root / "releases"
                stale = releases / f"{argv[-1]}.partial"
                stale.mkdir()
                (stale / "leftover").write_text("an interrupted unpack\n", encoding="utf-8")
                modes = [(releases, stat.S_IMODE(releases.lstat().st_mode))]
                releases.chmod(0o555)
                done, before, after = self.outcome(w, argv, good, modes)
                self.judge(f"{verb} / stale unpack", done, before, after, "host step")
                self.assertIn(
                    "releases directory",
                    done.stderr.splitlines()[-1],
                    f"{verb}: the refusal does not name the releases directory",
                )

    def test_a_stale_unpack_that_cannot_be_deleted_is_refused_by_that_name(self):
        for verb in examined("undeletable-unpack member(s)", ["install"]):
            with tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, verb, "installed")
                keep = w.root / "releases" / f"{argv[-1]}.partial" / "keep"
                keep.mkdir(parents=True)
                (keep / "leftover").write_text("an interrupted unpack\n", encoding="utf-8")
                keep.chmod(0o555)
                done, before, after = self.outcome(w, argv, good, [(keep, 0o755)])
                self.judge(f"{verb} / undeletable unpack", done, before, after, "host step")
                self.assertIn("unfinished unpack", done.stderr.splitlines()[-1])
                self.assertTrue((keep / "leftover").is_file(), "the leftover is gone")

    def test_a_stale_drop_in_that_cannot_be_deleted_is_refused(self):
        stale = f"{JOB}@sync.service.d/30-old.conf"

        def world(tmp):
            w, good, argv = self.situation(tmp, "install", "installed")
            (w.units / stale).write_text("[Service]\n", encoding="utf-8")
            return w, good, argv

        with tempfile.TemporaryDirectory() as tmp:
            w, good, argv = world(tmp)
            self.ok(w.run(*argv, **good))
            deletes = [
                ln.split("\t")
                for ln in (w.log / "args.log").read_text(encoding="utf-8").splitlines()
                if ln.startswith("find\thost\t") and stale in ln and "-delete" in ln
            ]
        for _, _, index, *_ in examined("stale-drop-in member(s)", deletes):
            with tempfile.TemporaryDirectory() as tmp:
                w, good, argv = world(tmp)
                planted = {**good, "TOOL_FAILS": f"find:host:{index}"}
                done, before, after = self.outcome(w, argv, planted, [])
                self.judge("install / stale drop-in", done, before, after, "host step")
                self.assertIn(
                    "could not install its unit files",
                    done.stderr.splitlines()[-1],
                    "the refusal does not name the unit install",
                )
                self.assertTrue((w.units / stale).is_file(), "the stale drop-in is gone")

    def test_a_failed_switch_leaves_no_unit_only_the_new_release_ships(self):
        for verb in examined("new-unit member(s)", ["install", "rollback-unkept"]):
            with self.subTest(verb=verb), tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, verb, "new-unit")
                planted = {**good, "TOOL_FAILS": "mv:host:2"}
                done, before, after = self.outcome(w, argv, planted, [])
                self.judge(f"{verb} / new unit / failed switch", done, before, after, "host step")
                self.assertFalse((w.units / "deck-streak-extra.timer").exists(), verb)
                self.assertTrue((w.units / API).is_file(), f"{verb}: the unit the host had is gone")

    def host_log(self, verb, state, faults):
        """The host calls of one run of `verb` from `state` with `faults` planted, as
        (tool, index, args) in the order the run made them."""
        with tempfile.TemporaryDirectory() as tmp:
            w, good, argv = self.situation(tmp, verb, state)
            try:
                w.run(*argv, **{**good, "TOOL_FAILS": faults})
            finally:
                self.restore(w.locked)
            rows = []
            for line in (w.log / "args.log").read_text(encoding="utf-8").splitlines():
                tool, side, index, args = (line.split("\t") + [""])[:4]
                if side == "host":
                    rows.append((tool, int(index), args))
            return rows

    @staticmethod
    def first_delete_after(rows, tool, suffix=""):
        """The index of the first host `find -delete` after the last call of `tool` (whose last
        argument ends in `suffix`): the first removal an undo makes."""
        last = max(
            i
            for i, (name, _, args) in enumerate(rows)
            if name == tool and args.split()[-1:] and args.split()[-1].endswith(suffix)
        )
        for name, index, args in rows[last + 1 :]:
            if name == "find" and "-delete" in args.split():
                return index
        raise AssertionError("no removal follows the call")

    def refused_twice(self, label, verb, state, faults, text):
        """Run `verb` from `state` with `faults` planted: it must end non-zero with one `deploy:`
        line, last, that holds `text`, and leave no temporary path."""
        with tempfile.TemporaryDirectory() as tmp:
            w, good, argv = self.situation(tmp, verb, state)
            before = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
            try:
                done = w.run(*argv, **{**good, "TOOL_FAILS": faults})
            finally:
                self.restore(w.locked)
            after = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
        lines = [ln for ln in done.stderr.splitlines() if ln.strip()]
        refusals = [ln for ln in lines if ln.startswith("deploy:")]
        self.assertNotEqual(done.returncode, 0, f"{label}: the verb went on: {done.stderr}")
        self.assertEqual(len(refusals), 1, f"{label}: {done.stderr!r}")
        self.assertEqual(lines[-1], refusals[0], f"{label}: the last line is not the refusal")
        self.assertIn(text, refusals[0], f"{label}: the refusal does not say so")
        self.assertNotIn("Traceback", done.stderr, label)
        for path in set(after) - set(before):
            temporary = path.name.startswith((".current.", ".stale.")) or path.name.endswith(
                (".partial", ".saved")
            )
            self.assertFalse(
                temporary or path.parts[0] == "tmpdir", f"{label}: a temporary path is left: {path}"
            )

    def test_an_undo_that_cannot_put_the_unit_files_back_says_so_and_leaves_no_temporary_path(self):
        """A second failure, in the undo itself: each removal is tried twice, so both tries fail."""
        rows = self.host_log("install", "installed", "install:host:1")
        k = self.first_delete_after(rows, "install")
        faults = f"install:host:1,find:host:{k},find:host:{k + 1}"
        self.refused_twice(
            "install / installed / undo", "install", "installed", faults, "put its unit files back"
        )
        for verb in examined("undo-and-say member(s)", ["install", "rollback-unkept"]):
            rows = self.host_log(verb, "effective-refused", "")
            k = self.first_delete_after(rows, "install")
            faults = f"find:host:{k},find:host:{k + 1}"
            self.refused_twice(
                f"{verb} / effective-refused / undo",
                verb,
                "effective-refused",
                faults,
                "put its unit files back after",
            )

    def test_a_way_back_that_fails_in_two_tries_is_named_by_the_leg_that_failed(self):
        for verb in examined("way-back member(s)", ["install", "rollback-unkept"]):
            rows = self.host_log(verb, "not-ready", "")
            n = sum(1 for tool, _, _ in rows if tool == "ln")
            self.refused_twice(
                f"{verb} / not-ready / link",
                verb,
                "not-ready",
                f"ln:host:{n},ln:host:{n + 1}",
                "the link was not put back",
            )
            k = self.first_delete_after(rows, "mv", "/current")
            self.refused_twice(
                f"{verb} / not-ready / units",
                verb,
                "not-ready",
                f"find:host:{k},find:host:{k + 1}",
                "the unit files were not put back",
            )
        rows = self.host_log("install", "not-ready-first", "")
        n = sum(1 for tool, _, _ in rows if tool == "rm")
        self.refused_twice(
            "install / not-ready-first / link",
            "install",
            "not-ready-first",
            f"rm:host:{n},rm:host:{n + 1}",
            "the link was not removed",
        )

    def test_a_stale_unpack_refuses_each_of_its_writing_calls(self):
        for verb in ["install", "rollback-unkept"]:
            self.refuse_each(verb, "stale-partial", self.calls_of(verb, "stale-partial"))
            with tempfile.TemporaryDirectory() as tmp:
                w, good, argv = self.situation(tmp, verb, "stale-partial")
                before = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
                done = w.run(*argv, **good)
                after = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
                self.judge_finished(
                    f"{verb} / stale-partial / finished", w, good, done, before, after
                )
            self.refused_twice(
                f"{verb} / stale-partial / set aside",
                verb,
                "stale-partial",
                "mv:host:1",
                "set aside",
            )

    def test_an_older_release_that_cannot_be_deleted_whole_is_left_whole(self):
        with tempfile.TemporaryDirectory() as tmp:
            w, good, argv = self.situation(tmp, "install", "over-keep-partly-deletable")
            old = w.root / "releases" / "v1.0.0"
            files = sorted(p.relative_to(old) for p in old.rglob("*") if p.is_file())
            before = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
            try:
                done = w.run(*argv, **good)
            finally:
                self.restore(w.locked)
            after = ATemporaryPathThatCannotBeMadeIsANamedRefusal.snapshot(w)
            self.judge_finished(
                "install / over-keep-partly-deletable", w, good, done, before, after
            )
            kept = sorted(p.relative_to(old) for p in old.rglob("*") if p.is_file())
            self.assertEqual(kept, files, "the older release was partly deleted")
            self.assertIn("left the older release v1.0.0", done.stderr)


if __name__ == "__main__":
    unittest.main()

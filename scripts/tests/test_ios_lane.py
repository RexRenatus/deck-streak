"""The TestFlight lane's steps, run as child processes over fixture repositories and recording
shims (SPEC-352 A1 to A13, ADR-363 D2 to D7).

`scripts/ios_lane.py` is never imported: each test runs it with `sys.executable`, in a fixture
checkout, with an environment the test builds from `PATH`, a scratch `HOME` and the GitHub
variables the step reads, so nothing of the session reaches it. Every number a test expects is a
literal the fixture's history fixes, never one the script computed, and every refusal is matched
as one whole line of the script's error output.

The fixture's history, every commit's workspace version in brackets:

    dev:   R [0.0.1] - A [0.1.0] - B [0.1.5] ----------------------- D
                                    \\- S1 [0.1.5] - S2 [0.2.0] -/     \\- T [0.4.0]  (topic)
    main:  R ------- M1 (A merged) ------------------------------- M2 (D merged)

`dev` holds six commits, four on its first-parent chain; `main`'s first-parent chain is M2, M1,
R. Annotated tags: `v0.1.0` on M1, `v0.2.0` on M2, `v0.1.5` on B (reached from main only through
M2's second parent), `v0.4.0` on T (not on main at all), and, on M2, `v0.3.0` (another version)
and `v0.2`, `v0.02.0` and `v0.2.0-rc.1` (not SemVer). `v0.0.1` on R is a lightweight tag.

The credential steps run with `openssl`, `security` and `xcodebuild` replaced by one recording shim
first on `PATH`. Each shim appends what it was given to a record: its arguments, its working
directory, which sentinels occur in its arguments, its environment and its standard input, how
much of the step's own output had been written when it ran, and what it found on disk. The six
credential parts and the identifiers a profile carries are random sentinels made at run time; the
certificate is random bytes and the profile a synthetic property list inside random bytes, so no
signing material exists anywhere. The parts are keyed by their role in the lane, never by the name
of the secret that holds them.
"""

import datetime
import hashlib
import json
import os
import plistlib
import re
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

from _support import REPO, examined

LANE = REPO / "scripts" / "ios_lane.py"
IDENTITY = ("-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid")
SHALLOW = "the checkout is shallow; the build number needs the full history"
# The six credential parts, each by the variable its step reads it from and the role word a
# refusal names it by, in the order the preflight lists them.
ROLES = (
    ("KEY", "the upload key"),
    ("KEYID", "the key id"),
    ("ISSUER", "the issuer id"),
    ("CERTIFICATE", "the certificate"),
    ("PASSWORD", "the certificate's password"),
    ("PROFILE", "the profile"),
)
# The parts no child may be given in any form; the key id and the issuer reach the export as its
# two named arguments, and nowhere else.
SECRET_VALUES = ("KEY", "CERTIFICATE", "PASSWORD", "PROFILE")
IDENTIFIERS = ("team", "team name", "app id", "profile name", "profile uuid")
# The signing settings the include file carries, which no command line may carry instead.
SIGNING_SETTINGS = (
    "DS_APP_ID",
    "DEVELOPMENT_TEAM",
    "CODE_SIGN_STYLE",
    "CODE_SIGN_IDENTITY",
    "PROVISIONING_PROFILE_SPECIFIER",
    "CODE_SIGNING_ALLOWED",
)
INCLUDE = Path("ios/Config/Signing.local.xcconfig")
LOGIN_KEYCHAIN = "/fixture/login.keychain-db"
BASE64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
# The one shim, installed under each tool's name. It reads the sentinels from a file beside it, so
# its own environment carries none of them.
SHIM = """
import hashlib, json, os, plistlib, sys
from pathlib import Path

tool, args = Path(sys.argv[0]).name, sys.argv[1:]
shims = Path(os.environ["SHIMS"])
sentinels = json.loads((shims / "sentinels.json").read_text())
fixture = json.loads((shims / "fixture.json").read_text())
stdin = sys.stdin.buffer.read().decode("utf-8", "replace") if sys.stdin else ""
lane = Path(os.environ["RUNNER_TEMP"]) / "lane"


def found(text):
    return sorted(role for role, value in sentinels.items() if value in text)


def value(flag):
    return args[args.index(flag) + 1]


entry = {
    "tool": tool,
    "args": args,
    "cwd": os.getcwd(),
    "in_args": found("\\0".join(args)),
    "in_env": found("\\0".join(f"{k}={v}" for k, v in os.environ.items())),
    "on_stdin": found(stdin),
    "stdout_size": os.path.getsize(os.environ["SHIMSTDOUT"]),
    "lane_files": sorted(p.name for p in lane.rglob("*")) if lane.is_dir() else [],
    "profiles": sorted(p.name for p in Path(os.environ["HOME"]).rglob("*.mobileprovision")),
}
out = ""
if tool == "openssl" and "-export" in args:
    Path(value("-out")).write_bytes(b"rewrapped")
elif tool == "openssl":
    entry["read"] = hashlib.sha256(Path(value("-in")).read_bytes()).hexdigest()
    pem = "ENCRYPTED " + "PRIVATE KEY"
    out = f"-----BEGIN {pem}-----\\nfixture\\n-----END {pem}-----\\n"
elif tool == "security" and args[0] == "create-keychain":
    Path(args[-1]).write_bytes(b"keychain")
elif tool == "security" and args[0] == "delete-keychain":
    Path(args[-1]).unlink()
elif tool == "security" and args[0] == "import":
    entry["imported"] = Path(args[1]).is_file()
elif tool == "security" and args[0] == "list-keychains" and "-s" not in args:
    out = f'    "{fixture["login"]}"\\n'
elif tool == "security" and args[0] == "find-identity":
    out = f'  1) {fixture["identity"]} "{fixture["identity name"]}"\\n     1 valid identities found\\n'
elif tool == "xcodebuild":
    if "archive" in args:
        Path(value("-archivePath")).mkdir(parents=True)
    if "-exportArchive" in args:
        key = Path(value("-authenticationKeyPath"))
        entry["key"] = {
            "path": str(key),
            "mode": key.stat().st_mode & 0o777,
            "directory mode": key.parent.stat().st_mode & 0o777,
            "holds the key": sentinels["KEY"] in key.read_text(),
        }
        entry["export options"] = plistlib.loads(Path(value("-exportOptionsPlist")).read_bytes())
    out = f"warning: {fixture['printed']}\\n"
with open(shims / "record.jsonl", "a", encoding="utf-8") as record:
    record.write(json.dumps(entry) + "\\n")
sys.stdout.write(out)
if (shims / f"fail-{tool}").exists():
    sys.exit(1)
"""


def cargo(version):
    """A root `Cargo.toml` whose workspace version is `version`."""
    return f'[workspace]\nmembers = []\n\n[workspace.package]\nversion = "{version}"\n'


def git(cwd, env, *args):
    """Run git in `cwd` under the fixture's environment and return its output, stripped."""
    run = subprocess.run(
        ["git", *IDENTITY, *args], cwd=cwd, env=env, capture_output=True, text=True, check=True
    )
    return run.stdout.strip()


class LanePlan(unittest.TestCase):
    @staticmethod
    def setUpClass():
        # The fixture repository, built once for the class. unittest calls this on the class, and
        # a static method is a name the workflow files' read census places; a class method is not.
        cls = LanePlan
        scratch = tempfile.TemporaryDirectory()
        cls.addClassCleanup(scratch.cleanup)
        cls.scratch = Path(scratch.name)
        (cls.scratch / "home").mkdir()
        cls.env = {"PATH": os.environ["PATH"], "HOME": str(cls.scratch / "home"), "LANG": "C.UTF-8"}
        cls.origin = cls.scratch / "origin"
        cls.origin.mkdir()
        cls.sha = {}

        def commit(name, version=None):
            if version is not None:
                (cls.origin / "Cargo.toml").write_text(cargo(version), encoding="utf-8")
            (cls.origin / "notes.txt").write_text(f"{name}\n", encoding="utf-8")
            run("add", "-A")
            run("commit", "-q", "-m", name)
            cls.sha[name] = run("rev-parse", "HEAD")

        def merge(name, branch):
            run("merge", "-q", "--no-ff", "-m", name, branch)
            cls.sha[name] = run("rev-parse", "HEAD")

        def run(*args):
            return git(cls.origin, cls.env, *args)

        run("init", "-q", "-b", "dev")
        commit("R", "0.0.1")
        run("tag", "v0.0.1")
        run("branch", "main")
        commit("A", "0.1.0")
        run("checkout", "-q", "main")
        merge("M1", "dev")
        run("tag", "-a", "v0.1.0", "-m", "v0.1.0")
        run("checkout", "-q", "dev")
        commit("B", "0.1.5")
        run("tag", "-a", "v0.1.5", "-m", "v0.1.5")
        run("checkout", "-q", "-b", "side")
        (cls.origin / "side.txt").write_text("S1\n", encoding="utf-8")
        commit("S1")
        commit("S2", "0.2.0")
        run("checkout", "-q", "dev")
        merge("D", "side")
        run("checkout", "-q", "main")
        merge("M2", "dev")
        for tag in ("v0.2.0", "v0.3.0", "v0.2", "v0.02.0", "v0.2.0-rc.1"):
            run("tag", "-a", tag, "-m", tag)
        run("checkout", "-q", "-b", "topic", "dev")
        commit("T", "0.4.0")
        run("tag", "-a", "v0.4.0", "-m", "v0.4.0")
        run("checkout", "-q", "dev")

    def clone(self, *options, at=None):
        """A clone of the fixture in a fresh directory, detached at `at` when it names a tag."""
        checkout = self.scratch / f"checkout-{uuid.uuid4().hex}"
        git(self.scratch, self.env, "clone", "-q", *options, f"file://{self.origin}", str(checkout))
        if at is not None and not options:
            git(checkout, self.env, "checkout", "-q", "--detach", f"refs/tags/{at}")
        return checkout

    def single(self, version):
        """A one-commit repository on `dev` whose workspace version is `version`, and its commit."""
        root = self.scratch / f"single-{uuid.uuid4().hex}"
        root.mkdir()
        (root / "Cargo.toml").write_text(cargo(version), encoding="utf-8")
        git(root, self.env, "init", "-q", "-b", "dev")
        git(root, self.env, "add", "-A")
        git(root, self.env, "commit", "-q", "-m", "the workspace")
        return root, git(root, self.env, "rev-parse", "HEAD")

    def plan(self, checkout, lane, event, ref, sha, **extra):
        """Run the plan step as the runner would, and return the run and the outputs it wrote."""
        output = self.scratch / f"output-{uuid.uuid4().hex}"
        output.touch()
        env = dict(
            self.env,
            GITHUB_EVENT_NAME=event,
            GITHUB_REF=ref,
            GITHUB_REF_TYPE="tag" if ref.startswith("refs/tags/") else "branch",
            GITHUB_REF_NAME=ref.split("/", 2)[2],
            GITHUB_SHA=sha,
            GITHUB_OUTPUT=str(output),
            **extra,
        )
        run = subprocess.run(
            [sys.executable, str(LANE), "plan", "--lane", lane],
            cwd=checkout,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        lines = output.read_text(encoding="utf-8").splitlines()
        return run, dict(line.split("=", 1) for line in lines if line)

    def assertRefused(self, run, outputs, message):
        self.assertIn(message, run.stderr.splitlines())
        self.assertEqual(run.returncode, 1)
        self.assertEqual(outputs, {})

    def test_the_internal_plan_refuses_any_event_or_ref_but_a_dispatch_on_dev(self):
        # SPEC-352 A1 (R3, as R22 amends it): a dispatch on dev and a push to dev are admitted;
        # any other event, and either admitted event on any ref but dev, is refused by name.
        checkout = self.clone()
        for event in examined("admitted internal starts", ["workflow_dispatch", "push"]):
            with self.subTest(event=event):
                run, outputs = self.plan(
                    checkout, "internal", event, "refs/heads/dev", self.sha["D"]
                )
                self.assertEqual(outputs.get("lane"), "internal")
                self.assertEqual(run.returncode, 0)
        events_only = "the internal lane runs on workflow_dispatch or push only, not on {}"
        dev_only = "the internal lane builds refs/heads/dev only, not {}"
        refused = [
            ("pull_request", "refs/pull/1/merge", events_only.format("pull_request")),
            ("schedule", "refs/heads/dev", events_only.format("schedule")),
            ("push", "refs/heads/main", dev_only.format("refs/heads/main")),
            ("push", "refs/tags/v0.2.0", dev_only.format("refs/tags/v0.2.0")),
            ("workflow_dispatch", "refs/heads/main", dev_only.format("refs/heads/main")),
            ("workflow_dispatch", "refs/tags/v0.2.0", dev_only.format("refs/tags/v0.2.0")),
        ]
        for event, ref, message in examined("refused internal starts", refused):
            with self.subTest(event=event, ref=ref):
                run, outputs = self.plan(checkout, "internal", event, ref, self.sha["D"])
                self.assertRefused(run, outputs, message)

    def test_the_plan_refuses_a_shallow_checkout(self):
        starts = [
            ("internal", "dev", "workflow_dispatch", "refs/heads/dev", "D"),
            ("release", "v0.2.0", "push", "refs/tags/v0.2.0", "M2"),
        ]
        for lane, branch, event, ref, commit in examined("shallow checkouts", starts):
            with self.subTest(lane=lane):
                shallow = self.clone("--depth", "1", "--branch", branch)
                self.assertEqual(
                    git(shallow, self.env, "rev-parse", "--is-shallow-repository"), "true"
                )
                run, outputs = self.plan(shallow, lane, event, ref, self.sha[commit])
                self.assertRefused(run, outputs, SHALLOW)

    def test_the_internal_build_number_is_devs_first_parent_count(self):
        checkout = self.clone()
        run, outputs = self.plan(
            checkout, "internal", "workflow_dispatch", "refs/heads/dev", self.sha["D"]
        )
        # dev holds six commits, four of them on its first-parent chain: R, A, B and D.
        self.assertEqual(outputs.get("number"), "4")
        self.assertEqual(outputs, {"lane": "internal", "number": "4", "version": "0.2.0"})
        self.assertEqual(run.returncode, 0)
        self.assertEqual(git(checkout, self.env, "rev-list", "--count", "HEAD"), "6")

    def test_the_marketing_version_is_the_workspace_version_in_three_integers(self):
        root, sha = self.single("3.14.15")
        run, outputs = self.plan(root, "internal", "workflow_dispatch", "refs/heads/dev", sha)
        self.assertEqual(outputs.get("version"), "3.14.15")
        self.assertEqual(outputs, {"lane": "internal", "number": "1", "version": "3.14.15"})
        self.assertEqual(run.returncode, 0)
        for version in examined("refused workspace versions", ["1.2", "1.2.3-rc.1"]):
            with self.subTest(version=version):
                root, sha = self.single(version)
                run, outputs = self.plan(
                    root, "internal", "workflow_dispatch", "refs/heads/dev", sha
                )
                self.assertRefused(
                    run,
                    outputs,
                    f"the workspace version {version} is not three dot-separated integers",
                )

    def test_the_release_plan_admits_only_an_annotated_semver_tag_on_mains_first_parent_chain(
        self,
    ):
        tagged = {"v0.0.1": "R", "v0.1.5": "B", "v0.4.0": "T"}
        refused = [
            ("v0.0.1", "the tag v0.0.1 is a lightweight tag; a release is an annotated tag"),
            ("v0.2", "the tag v0.2 is not a SemVer release tag"),
            ("v0.02.0", "the tag v0.02.0 is not a SemVer release tag"),
            ("v0.2.0-rc.1", "the tag v0.2.0-rc.1 is not a SemVer release tag"),
            ("v0.4.0", "the commit of v0.4.0 is not on main"),
            ("v0.1.5", "the commit of v0.1.5 is not on main's first-parent chain"),
            ("v0.3.0", "the tag v0.3.0 names 0.3.0, but the workspace version is 0.2.0"),
        ]
        for tag, message in examined("refused release tags", refused):
            with self.subTest(tag=tag):
                checkout = self.clone(at=tag)
                sha = self.sha[tagged.get(tag, "M2")]
                run, outputs = self.plan(checkout, "release", "push", f"refs/tags/{tag}", sha)
                self.assertRefused(run, outputs, message)
        tag_runs_only = "the release lane runs on a tag's push or dispatch only, not on {} of {}"
        starts = [("push", "refs/heads/main"), ("workflow_dispatch", "refs/heads/v0.2.0")]
        for event, ref in examined("refused release starts", starts):
            with self.subTest(event=event, ref=ref):
                checkout = self.clone(at="v0.2.0")
                run, outputs = self.plan(checkout, "release", event, ref, self.sha["M2"])
                self.assertRefused(run, outputs, tag_runs_only.format(event, ref))
        checkout = self.clone(at="v0.2.0")
        run, outputs = self.plan(checkout, "release", "push", "refs/tags/v0.2.0", self.sha["M2"])
        self.assertEqual(outputs, {"lane": "release", "number": "3", "version": "0.2.0"})
        self.assertEqual(run.returncode, 0)

    def test_the_release_plan_admits_a_dispatch_at_the_tags_ref_as_its_push_and_refuses_a_branch(
        self,
    ):
        # SPEC-405 A2 (R3, ADR-419 D2a): a dispatch at a tag's own ref is planned exactly as the
        # tag's push is, and a dispatch at a branch, or any other event, is refused by its event
        # and its ref.
        for event in examined("admitted release events", ["push", "workflow_dispatch"]):
            with self.subTest(event=event):
                checkout = self.clone(at="v0.2.0")
                run, outputs = self.plan(
                    checkout,
                    "release",
                    event,
                    "refs/tags/v0.2.0",
                    self.sha["M2"],
                    GITHUB_ACTOR="a-maintainer",
                )
                self.assertEqual(outputs, {"lane": "release", "number": "3", "version": "0.2.0"})
                self.assertEqual(run.returncode, 0)
        tagged = {"v0.0.1": "R", "v0.1.5": "B", "v0.4.0": "T", "v0.3.0": "M2"}
        refused = [
            ("v0.0.1", "the tag v0.0.1 is a lightweight tag; a release is an annotated tag"),
            ("v0.4.0", "the commit of v0.4.0 is not on main"),
            ("v0.1.5", "the commit of v0.1.5 is not on main's first-parent chain"),
            ("v0.3.0", "the tag v0.3.0 names 0.3.0, but the workspace version is 0.2.0"),
        ]
        for tag, message in examined("tags a dispatch is refused at", refused):
            with self.subTest(tag=tag):
                checkout = self.clone(at=tag)
                run, outputs = self.plan(
                    checkout,
                    "release",
                    "workflow_dispatch",
                    f"refs/tags/{tag}",
                    self.sha[tagged[tag]],
                    GITHUB_ACTOR="a-maintainer",
                )
                self.assertRefused(run, outputs, message)
        tag_runs_only = "the release lane runs on a tag's push or dispatch only, not on {} of {}"
        starts = [
            ("workflow_dispatch", "refs/heads/main"),
            ("workflow_dispatch", "refs/heads/dev"),
            ("workflow_dispatch", "refs/heads/v0.2.0"),
            ("schedule", "refs/tags/v0.2.0"),
        ]
        for event, ref in examined("starts the release plan refuses", starts):
            with self.subTest(event=event, ref=ref):
                checkout = self.clone(at="v0.2.0")
                run, outputs = self.plan(checkout, "release", event, ref, self.sha["M2"])
                self.assertRefused(run, outputs, tag_runs_only.format(event, ref))

    def test_the_release_plan_refuses_a_run_the_workflow_token_started(self):
        # SPEC-405 A5 (R4, ADR-419 D3): a run that a workflow's own token started is refused on
        # either event, and writes no output. The actor is spelt here, never read from the lane.
        token = "github-actions[bot]"
        for event in examined("events a token run is refused on", ["push", "workflow_dispatch"]):
            with self.subTest(event=event):
                checkout = self.clone(at="v0.2.0")
                run, outputs = self.plan(
                    checkout,
                    "release",
                    event,
                    "refs/tags/v0.2.0",
                    self.sha["M2"],
                    GITHUB_ACTOR=token,
                )
                self.assertRefused(
                    run, outputs, f"the release lane takes no run that {token} started"
                )

    def test_the_release_build_number_is_mains_first_parent_count_at_the_tag(self):
        # main's first-parent chain is R, M1, M2: two commits at v0.1.0 and three at v0.2.0, where
        # M1 reaches three commits and M2 eight.
        releases = [("v0.1.0", "M1", "2", "0.1.0"), ("v0.2.0", "M2", "3", "0.2.0")]
        for tag, commit, number, version in examined("release tags", releases):
            with self.subTest(tag=tag):
                checkout = self.clone(at=tag)
                run, outputs = self.plan(
                    checkout, "release", "push", f"refs/tags/{tag}", self.sha[commit]
                )
                self.assertEqual(outputs.get("number"), number)
                self.assertEqual(outputs, {"lane": "release", "number": number, "version": version})
                self.assertEqual(run.returncode, 0)


def base64_text(data):
    """`data` in base64, as a secret holds bytes, spelt out so the test imports no codec."""
    bits = "".join(f"{byte:08b}" for byte in data)
    bits += "0" * (-len(bits) % 6)
    text = "".join(BASE64[int(bits[start : start + 6], 2)] for start in range(0, len(bits), 6))
    return text + "=" * (-len(text) % 4)


def xcconfig(text):
    """An xcconfig's settings, `NAME = value` per line; comments and blank lines are skipped."""
    return dict(re.findall(r"(?m)^([A-Z_]+) = (.*)$", text))


class LaneCredential(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.scratch = Path(scratch.name)
        self.shims = self.scratch / "shims"
        self.bin = self.scratch / "bin"
        self.home = self.scratch / "home"
        self.temp = self.scratch / "runner-temp"
        self.checkout = self.scratch / "checkout"
        for directory in (self.shims, self.bin, self.home, self.temp, self.checkout / "ios/Config"):
            directory.mkdir(parents=True)
        self.lane = self.temp / "lane"
        for tool in ("openssl", "security", "xcodebuild"):
            shim = self.bin / tool
            shim.write_text(f"#!{sys.executable}\n{SHIM}", encoding="utf-8")
            shim.chmod(0o755)
        token = uuid.uuid4().hex
        self.ids = {
            "team": uuid.uuid4().hex[:10].upper(),
            "team name": f"Team {uuid.uuid4().hex}",
            "app id": f"com.{uuid.uuid4().hex}.harness",
            "profile name": f"Profile {uuid.uuid4().hex}",
            "profile uuid": str(uuid.uuid4()).upper(),
        }
        self.certificate = os.urandom(512)
        self.developer_certificate = os.urandom(256)
        self.identity = hashlib.sha1(self.developer_certificate).hexdigest().upper()
        self.values = {
            "KEY": "-----BEGIN " + f"PRIVATE KEY-----\n{token}\n-----END PRIVATE KEY-----\n",
            "KEYID": uuid.uuid4().hex[:10].upper(),
            "ISSUER": str(uuid.uuid4()),
            "CERTIFICATE": base64_text(self.certificate),
            "PASSWORD": uuid.uuid4().hex,
            "PROFILE": self.profile(),
        }
        # The key's sentinel is its body: a leak of the body alone is a leak.
        sentinels = dict(self.values, KEY=token)
        sentinels.update(self.ids)
        (self.shims / "sentinels.json").write_text(json.dumps(sentinels), encoding="utf-8")
        fixture = {
            "login": LOGIN_KEYCHAIN,
            "identity": self.identity,
            "identity name": f"Apple Distribution: {self.ids['team name']} ({self.ids['team']})",
            "printed": " ".join(self.ids.values()),
        }
        (self.shims / "fixture.json").write_text(json.dumps(fixture), encoding="utf-8")
        self.sha = uuid.uuid4().hex + uuid.uuid4().hex[:8]
        self.runs = []

    def profile(self, **changes):
        """A synthetic distribution profile, base64-encoded: a property list inside random bytes,
        as a signed profile holds one. `changes` replaces its fields; a dict merges into
        `Entitlements`."""
        now = datetime.datetime.now(datetime.UTC).replace(tzinfo=None, microsecond=0)
        team = self.ids["team"]
        fields = {
            "AppIDName": "fixture",
            "ApplicationIdentifierPrefix": [team],
            "DeveloperCertificates": [self.developer_certificate],
            "Entitlements": {
                "application-identifier": f"{team}.{self.ids['app id']}",
                "com.apple.developer.team-identifier": team,
                "get-task-allow": False,
                "beta-reports-active": True,
            },
            "ExpirationDate": now + datetime.timedelta(days=30),
            "Name": self.ids["profile name"],
            "TeamIdentifier": [team],
            "TeamName": self.ids["team name"],
            "UUID": self.ids["profile uuid"],
            "Version": 1,
        }
        for field, change in changes.items():
            if isinstance(change, dict):
                fields[field] = dict(fields[field], **change)
            else:
                fields[field] = change
        blob = b"0\x82" + os.urandom(48) + plistlib.dumps(fields) + os.urandom(48)
        return base64_text(blob)

    def step(self, verb, parts=(), **env):
        """Run one verb as its step would, with the credential parts `parts` in its own
        environment, and return its exit code, its output, its errors and the shims' records."""
        stdout = self.scratch / f"stdout-{uuid.uuid4().hex}"
        output = self.scratch / f"output-{uuid.uuid4().hex}"
        output.touch()
        record = self.shims / "record.jsonl"
        before = len(record.read_text(encoding="utf-8").splitlines()) if record.exists() else 0
        environment = {
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "HOME": str(self.home),
            "LANG": "C.UTF-8",
            "RUNNER_TEMP": str(self.temp),
            "GITHUB_OUTPUT": str(output),
            "GITHUB_STEP_SUMMARY": str(self.scratch / "summary.md"),
            "GITHUB_SHA": self.sha,
            "NUMBER": "7",
            "VERSION": "1.2.3",
            "LANE": "internal",
            "SHIMS": str(self.shims),
            "SHIMSTDOUT": str(stdout),
        }
        environment.update({role: self.values[role] for role in parts})
        environment.update(env)
        with open(stdout, "w", encoding="utf-8") as sink:
            run = subprocess.run(
                [sys.executable, str(LANE), verb],
                cwd=self.checkout,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=sink,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
        lines = record.read_text(encoding="utf-8").splitlines() if record.exists() else []
        records = [json.loads(line) for line in lines[before:]]
        printed = stdout.read_text(encoding="utf-8")
        outputs = dict(
            line.split("=", 1) for line in output.read_text(encoding="utf-8").splitlines() if line
        )
        self.runs.append((printed, run.stderr, records))
        return run.returncode, printed, run.stderr, records, outputs

    def one(self, records, tool, first):
        """The one record of `tool` whose first argument is `first`."""
        found = [r for r in records if r["tool"] == tool and r["args"][:1] == [first]]
        self.assertEqual(len(found), 1, f"{tool} {first} ran {len(found)} times")
        return found[0]

    def masks(self, printed):
        return [
            line[len("::add-mask::") :] for line in printed.splitlines() if "::add-mask::" in line
        ]

    def sign(self):
        code, printed, errors, records, _ = self.step(
            "sign", ("CERTIFICATE", "PASSWORD", "PROFILE")
        )
        self.assertEqual(code, 0, errors)
        return printed, records

    def test_the_preflight_uploads_on_all_stops_on_none_and_fails_on_some(self):
        sets = [
            [role for bit, (role, _) in enumerate(ROLES) if mask >> bit & 1] for mask in range(64)
        ]
        partial = [placed for placed in sets if 0 < len(placed) < len(ROLES)]
        for placed in examined("partial credential sets", partial):
            presence = {role: "true" if role in placed else "false" for role, _ in ROLES}
            absent = ", ".join(word for role, word in ROLES if role not in placed)
            with self.subTest(placed=len(placed), absent=absent):
                code, _, errors, _, outputs = self.step("preflight", **presence)
                self.assertIn(
                    f"the credential is half placed; absent: {absent}", errors.splitlines()
                )
                self.assertEqual(code, 1)
                self.assertEqual(outputs, {})
        for placed, answer in (("true", "all"), ("false", "none")):
            with self.subTest(answer=answer):
                code, _, errors, _, outputs = self.step(
                    "preflight", **{role: placed for role, _ in ROLES}
                )
                self.assertEqual(outputs, {"placed": answer})
                self.assertEqual((code, errors), (0, ""))

    def test_the_profile_is_read_checked_and_masked_before_any_tool_runs(self):
        printed, records = self.sign()
        masks = self.masks(printed)
        self.assertEqual(set(self.ids.values()) - set(masks), set())
        generated = [mask for mask in masks if mask not in self.ids.values()]
        self.assertEqual(len(generated), 2)
        self.assertEqual([len(re.fullmatch(r"[0-9a-f]+", m).group()) for m in generated], [32, 32])
        last = printed.index(f"::add-mask::{masks[-1]}") + len(f"::add-mask::{masks[-1]}\n")
        self.assertGreaterEqual(records[0]["stdout_size"], last)
        refused = [
            (
                "a device profile",
                {"Entitlements": {"get-task-allow": True}},
                "the profile is a development profile, not an App Store distribution profile",
            ),
            (
                "a profile with devices",
                {"ProvisionedDevices": [uuid.uuid4().hex]},
                "the profile is a development profile, not an App Store distribution profile",
            ),
            (
                "an expired profile",
                {
                    "ExpirationDate": datetime.datetime.now(datetime.UTC).replace(tzinfo=None)
                    - datetime.timedelta(days=1)
                },
                "the profile has expired",
            ),
            (
                "the placeholder's app id",
                {
                    "Entitlements": {
                        "application-identifier": f"{self.ids['team']}.invalid.deckstreak.harness"
                    }
                },
                "the profile's app id is the placeholder, whose first label is invalid",
            ),
            (
                "another certificate's profile",
                {"DeveloperCertificates": [os.urandom(256)]},
                "the profile was not issued for the imported certificate",
            ),
        ]
        self.step("clean")
        for case, changes, message in examined("refused profiles", refused):
            with self.subTest(case=case):
                self.values["PROFILE"] = self.profile(**changes)
                code, printed, errors, records, _ = self.step(
                    "sign", ("CERTIFICATE", "PASSWORD", "PROFILE")
                )
                self.assertIn(message, errors.splitlines())
                self.assertEqual(code, 1)
                tools = [(r["tool"], r["args"][0]) for r in records]
                after = (
                    tools[tools.index(("security", "find-identity")) + 1 :]
                    if (("security", "find-identity") in tools)
                    else []
                )
                self.assertEqual(after, [])
                self.assertNotIn("xcodebuild", [tool for tool, _ in tools])
                self.assertFalse((self.checkout / INCLUDE).exists())
                self.assertEqual(list(self.home.rglob("*.mobileprovision")), [])

    def test_signing_imports_the_certificate_non_extractable_into_a_job_keychain(self):
        _, records = self.sign()
        keychain = str(self.lane / "lane.keychain-db")
        created = self.one(records, "security", "create-keychain")
        self.assertEqual(created["args"][-1], keychain)
        read = [r for r in records if r["tool"] == "openssl" and "-export" not in r["args"]]
        self.assertEqual([r["read"] for r in read], [hashlib.sha256(self.certificate).hexdigest()])
        imported = self.one(records, "security", "import")
        self.assertTrue(imported["imported"])
        self.assertIn(["-x"], [imported["args"][i : i + 1] for i in range(len(imported["args"]))])
        self.assertIn("-T /usr/bin/codesign", " ".join(imported["args"]))
        self.assertIn(f"-k {keychain}", " ".join(imported["args"]))
        self.assertEqual(imported["args"][1:2], [str(self.lane / "certificate.p12")])
        search = [r["args"] for r in records if r["args"][:3] == ["list-keychains", "-d", "user"]]
        self.assertEqual(
            search,
            [
                ["list-keychains", "-d", "user"],
                ["list-keychains", "-d", "user", "-s", keychain, LOGIN_KEYCHAIN],
            ],
        )
        archive = self.one(records, "xcodebuild", "archive")
        self.assertEqual(
            archive["args"],
            [
                "archive",
                "-project",
                "ios/Harness.xcodeproj",
                "-scheme",
                "Harness",
                "-configuration",
                "Release",
                "-destination",
                "generic/platform=iOS",
                "-archivePath",
                str(self.lane / "app.xcarchive"),
                "CURRENT_PROJECT_VERSION=7",
                "MARKETING_VERSION=1.2.3",
                "DS_LANE=internal",
            ],
        )
        self.assertEqual([n for n in archive["lane_files"] if n.endswith(".p12")], [])
        self.assertEqual(archive["profiles"], [f"{self.ids['profile uuid']}.mobileprovision"])
        include = xcconfig((self.checkout / INCLUDE).read_text(encoding="utf-8"))
        self.assertEqual(
            include,
            {
                "DS_APP_ID": self.ids["app id"],
                "DEVELOPMENT_TEAM": self.ids["team"],
                "CODE_SIGN_STYLE": "Manual",
                "CODE_SIGN_IDENTITY": self.identity,
                "PROVISIONING_PROFILE_SPECIFIER": self.ids["profile uuid"],
            },
        )
        for setting in examined("signing settings", SIGNING_SETTINGS):
            self.assertEqual([a for a in archive["args"] if a.startswith(f"{setting}=")], [])

    def test_the_upload_key_lives_in_one_private_file_for_the_upload_step_only(self):
        self.sign()
        self.assertEqual(list(self.lane.glob("key-*")), [])
        code, _, errors, records, _ = self.step("upload-to-testflight", ("KEY", "KEYID", "ISSUER"))
        self.assertEqual(code, 0, errors)
        export = self.one(records, "xcodebuild", "-exportArchive")
        key = export["key"]
        self.assertEqual((key["mode"], key["directory mode"]), (0o600, 0o700))
        self.assertTrue(key["holds the key"])
        self.assertEqual(Path(key["path"]).parent.parent, self.lane)
        arguments = export["args"]
        self.assertEqual(
            arguments,
            [
                "-exportArchive",
                "-archivePath",
                str(self.lane / "app.xcarchive"),
                "-exportPath",
                str(self.lane / "export"),
                "-exportOptionsPlist",
                str(self.lane / "export-options.plist"),
                "-authenticationKeyPath",
                key["path"],
                "-authenticationKeyID",
                self.values["KEYID"],
                "-authenticationKeyIssuerID",
                self.values["ISSUER"],
            ],
        )
        self.assertEqual(
            export["export options"],
            {
                "destination": "upload",
                "manageAppVersionAndBuildNumber": False,
                "method": "app-store-connect",
                "provisioningProfiles": {self.ids["app id"]: self.ids["profile uuid"]},
                "signingCertificate": self.identity,
                "signingStyle": "manual",
                "teamID": self.ids["team"],
                "testFlightInternalTestingOnly": True,
            },
        )
        self.assertFalse(Path(key["path"]).parent.exists())
        (self.shims / "fail-xcodebuild").touch()
        code, _, errors, records, _ = self.step("upload-to-testflight", ("KEY", "KEYID", "ISSUER"))
        failed = self.one(records, "xcodebuild", "-exportArchive")["key"]["path"]
        self.assertEqual(code, 1)
        self.assertFalse(Path(failed).parent.exists())
        self.assertEqual(list(self.lane.glob("key-*")), [])

    def test_no_secret_value_reaches_a_childs_argv_or_environment_or_the_log(self):
        self.sign()
        self.step("upload-to-testflight", ("KEY", "KEYID", "ISSUER"))
        self.step("clean")
        everything = [record for _, _, records in self.runs for record in records]
        stdin = [r["on_stdin"] for r in everything if r["tool"] == "openssl"]
        self.assertEqual(stdin, [["PASSWORD"], []])
        for record in examined("tool calls", everything):
            with self.subTest(tool=record["tool"], first=record["args"][0]):
                named = set(record["in_args"])
                if "-exportArchive" in record["args"]:
                    named -= {"KEYID", "ISSUER"}
                self.assertEqual(named & {role for role, _ in ROLES}, set())
                self.assertEqual(set(record["in_env"]) & {role for role, _ in ROLES}, set())
        for printed, errors, records in examined("steps", self.runs):
            for line in (printed + errors).splitlines():
                with self.subTest(line=line[:12]):
                    secret = [r for r, _ in ROLES if r != "KEY" and self.values[r] in line]
                    self.assertEqual(secret, [])
                    self.assertNotIn(self.values["KEY"].split("\n")[1], line)
                    if not line.startswith("::add-mask::"):
                        self.assertEqual([i for i, v in self.ids.items() if v in line], [])
        warning = "warning: " + " ".join(["<redacted>"] * len(self.ids))
        self.assertIn(warning, self.runs[0][0].splitlines())

    def test_the_clean_step_removes_every_piece_of_signing_material(self):
        self.sign()
        self.step("upload-to-testflight", ("KEY", "KEYID", "ISSUER"))
        code, _, errors, records, _ = self.step("clean")
        self.assertEqual(
            [r["args"] for r in records],
            [
                ["list-keychains", "-d", "user", "-s", LOGIN_KEYCHAIN],
                ["delete-keychain", str(self.lane / "lane.keychain-db")],
            ],
        )
        self.assertEqual((code, errors), (0, ""))
        self.assertFalse(self.lane.exists())
        self.assertFalse((self.checkout / INCLUDE).exists())
        self.assertEqual(list(self.home.rglob("*.mobileprovision")), [])
        self.assertEqual(list(self.temp.iterdir()), [])
        code, _, errors, records, _ = self.step("clean")
        self.assertEqual((code, errors, records), (0, "", []))

    def test_the_unsigned_build_stops_before_the_upload_and_says_so(self):
        harness = (REPO / "ios/Config/Harness.xcconfig").read_text(encoding="utf-8")
        self.assertEqual(xcconfig(harness)["DS_APP_ID"].split(".")[0], "invalid")
        code, _, errors, records, _ = self.step("build-unsigned")
        build = self.one(records, "xcodebuild", "build")
        self.assertEqual(
            build["args"],
            [
                "build",
                "-project",
                "ios/Harness.xcodeproj",
                "-scheme",
                "Harness",
                "-configuration",
                "Release",
                "-destination",
                "generic/platform=iOS",
                "-derivedDataPath",
                str(self.lane / "build"),
                "CODE_SIGNING_ALLOWED=NO",
                "CURRENT_PROJECT_VERSION=7",
                "MARKETING_VERSION=1.2.3",
                "DS_LANE=internal",
            ],
        )
        self.assertEqual(code, 0, errors)
        self.assertFalse((self.checkout / INCLUDE).exists())
        summary = self.scratch / "summary.md"
        cases = [
            (
                "none",
                "skipped",
                "not placed",
                "stopped before the upload: the credential is not placed",
            ),
            ("all", "success", "placed", "uploaded"),
            ("all", "failure", "placed", "not uploaded: the signing or the upload failed"),
            ("", "skipped", "half placed", "stopped at the preflight"),
        ]
        for placed, outcome, credential, said in examined("summaries", cases):
            with self.subTest(placed=placed, outcome=outcome):
                summary.write_text("", encoding="utf-8")
                code, _, errors, records, _ = self.step("summary", PLACED=placed, OUTCOME=outcome)
                self.assertEqual(
                    summary.read_text(encoding="utf-8"),
                    "### TestFlight: the internal lane\n\n"
                    f"- commit: {self.sha[:12]}\n"
                    "- version: 1.2.3, build 7\n"
                    f"- credential: {credential}\n"
                    f"- outcome: {said}\n",
                )
                self.assertEqual((code, errors, records), (0, "", []))

    def test_each_signing_tool_is_given_exactly_its_arguments(self):
        printed, records = self.sign()
        masks = self.masks(printed)
        known = ("team", "team name", "app id", "profile name", "profile uuid")
        self.assertEqual(masks[:5], [self.ids[each] for each in known])
        keychain_password, passphrase = masks[5:]
        keychain = str(self.lane / "lane.keychain-db")
        bundle = str(self.lane / "certificate.p12")
        expected = [
            [
                "openssl",
                "pkcs12",
                "-in",
                str(self.lane / "original.p12"),
                "-passin",
                "stdin",
                "-passout",
                f"pass:{passphrase}",
            ],
            [
                "openssl",
                "pkcs12",
                "-export",
                "-out",
                bundle,
                "-passin",
                f"pass:{passphrase}",
                "-passout",
                f"pass:{passphrase}",
                "-certpbe",
                "PBE-SHA1-3DES",
                "-keypbe",
                "PBE-SHA1-3DES",
                "-macalg",
                "sha1",
            ],
            ["security", "create-keychain", "-p", keychain_password, keychain],
            ["security", "set-keychain-settings", "-lut", "21600", keychain],
            ["security", "unlock-keychain", "-p", keychain_password, keychain],
            [
                "security",
                "import",
                bundle,
                "-k",
                keychain,
                "-P",
                passphrase,
                "-f",
                "pkcs12",
                "-x",
                "-T",
                "/usr/bin/codesign",
            ],
            [
                "security",
                "set-key-partition-list",
                "-S",
                "apple-tool:,apple:,codesign:",
                "-s",
                "-k",
                keychain_password,
                keychain,
            ],
            ["security", "list-keychains", "-d", "user"],
            ["security", "list-keychains", "-d", "user", "-s", keychain, LOGIN_KEYCHAIN],
            ["security", "find-identity", "-v", "-p", "codesigning", keychain],
            ["xcodebuild", "archive"],
        ]
        called = [[r["tool"], *r["args"]] for r in examined("signing calls", records)]
        self.assertEqual(called[:-1], expected[:-1])
        self.assertEqual(called[-1][:2], expected[-1])
        code, _, errors, records, _ = self.step("upload-to-testflight", ("KEY", "KEYID", "ISSUER"))
        self.assertEqual(code, 0, errors)
        key = Path(self.one(records, "xcodebuild", "-exportArchive")["key"]["path"])
        self.assertEqual((key.parent.parent, key.parent.name[:4]), (self.lane, "key-"))

    def test_the_profile_is_installed_where_xcode_reads_it_and_cleaned_up_alone(self):
        profiles = self.home / "Library/Developer/Xcode/UserData/Provisioning Profiles"
        profiles.mkdir(parents=True)
        another = profiles / "another.mobileprovision"
        another.write_bytes(b"another profile")
        self.sign()
        self.assertEqual(self.lane.stat().st_mode & 0o777, 0o700)
        installed = profiles / f"{self.ids['profile uuid']}.mobileprovision"
        self.assertEqual(sorted(profiles.iterdir()), sorted([another, installed]))
        code, _, errors, _, _ = self.step("clean")
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(list(profiles.iterdir()), [another])
        # A sign whose profile is already gone still cleans up everything else.
        self.sign()
        installed.unlink()
        code, _, errors, _, _ = self.step("clean")
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(list(self.temp.iterdir()), [])

    def test_a_build_tool_prints_only_its_error_and_warning_lines_each_value_redacted(self):
        # A profile's name often holds its app id, so a value can sit inside another.
        self.ids["profile name"] = f"Store Profile: {self.ids['app id']}"
        self.values["PROFILE"] = self.profile()
        fixture = json.loads((self.shims / "fixture.json").read_text(encoding="utf-8"))
        fixture["printed"] = (
            f"{self.ids['profile name']}\n{self.ids['team']} is built\nerror: {self.ids['app id']}"
        )
        (self.shims / "fixture.json").write_text(json.dumps(fixture), encoding="utf-8")
        self.sign()
        self.step("upload-to-testflight", ("KEY", "KEYID", "ISSUER"))
        for out, _, _ in examined("steps", self.runs):
            lines = out.splitlines()
            self.assertEqual(lines[-2:], ["warning: <redacted>", "error: <redacted>"])
            self.assertEqual(
                [line[:12] for line in lines[:-2]], ["::add-mask::"] * (len(lines) - 2)
            )


if __name__ == "__main__":
    unittest.main()

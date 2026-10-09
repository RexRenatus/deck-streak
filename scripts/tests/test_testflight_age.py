"""The re-release check reads the newest internal build and starts the internal lane before the
build stops working (SPEC-374 R15 to R21, A9 to A14). Every case runs the script as a child against
shims of `openssl`, `curl` and `gh` and a fixture repository, both inside the case's own temporary
directory, so nothing reaches a real store, token or workflow."""

import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO

SCRIPT = REPO / "scripts" / "testflight_age.py"
NOW = "2030-01-10T12:00:00Z"
NOW_EPOCH = 1894276800
GIT_IDENTITY = ["-c", "user.name=T", "-c", "user.email=t@example.invalid"]
PARTS = ("KEY", "KEYID", "ISSUER", "APPID")
# A DER signature whose r carries a sign byte and whose s is one byte short, and the raw r||s the
# conversion owes.
R = "80" + "11" * 31
S = "22" * 31
DER = bytes.fromhex("3044" + "0221" + "00" + R + "021f" + S)
RAW = bytes.fromhex(R) + bytes.fromhex("00" + S)

SHIM = """#!{python}
import json, os, stat, sys
here = os.path.dirname(os.path.abspath(__file__))
case = json.load(open(os.path.join(here, "case.json")))
name = os.path.basename(__file__)
argv = sys.argv[1:]
files = {{}}
for n, arg in enumerate(argv):
    path = None
    if arg in ("-sign", "--config") and n + 1 < len(argv):
        path = argv[n + 1]
    elif arg == "-H" and n + 1 < len(argv) and argv[n + 1].startswith("@"):
        path = argv[n + 1][1:]
    if path and os.path.exists(path):
        files[path] = {{
            "content": open(path, "rb").read().decode("latin-1"),
            "mode": stat.S_IMODE(os.stat(path).st_mode),
            "dir_mode": stat.S_IMODE(os.stat(os.path.dirname(path)).st_mode),
        }}
stdin = sys.stdin.buffer.read().decode("latin-1") if name == "openssl" else ""
record = {{"argv": argv, "stdin": stdin, "env": dict(os.environ), "files": files}}
with open(os.path.join(here, name + ".log"), "a") as sink:
    sink.write(json.dumps(record) + "\\n")
if name == "openssl":
    if case["openssl_exit"]:
        sys.exit(case["openssl_exit"])
    sys.stdout.buffer.write(bytes.fromhex(case["der"]))
elif name == "curl":
    if case["curl_exit"]:
        sys.exit(case["curl_exit"])
    out = argv[argv.index("--output") + 1]
    open(out, "w").write(case["body"])
    sys.stderr.write(case["trap"] + "\\n")
    sys.stdout.write(str(case["status"]))
elif name == "gh":
    sys.exit(case["gh_exit"])
"""


def build(number, uploaded, expires, state="VALID", audience="INTERNAL_ONLY", expired=False):
    return {
        "type": "builds",
        "id": f"id{number}",
        "attributes": {
            "version": str(number),
            "uploadedDate": uploaded,
            "expirationDate": expires,
            "expired": expired,
            "processingState": state,
            "buildAudienceType": audience,
        },
    }


def listing(*builds):
    return json.dumps({"data": list(builds)})


class Fixture:
    """One case's world: shims, a fixture repository whose first-parent count is 3 (and total 4),
    and the environment the script runs in."""

    def __init__(self, root, body="", status=200, curl_exit=0, openssl_exit=0, gh_exit=0):
        self.root = Path(root)
        self.shims = self.root / "shims"
        self.shims.mkdir()
        self.trap = "appid-" + "x" * 8
        case = {
            "body": body,
            "status": status,
            "curl_exit": curl_exit,
            "openssl_exit": openssl_exit,
            "gh_exit": gh_exit,
            "der": DER.hex(),
            "trap": self.trap,
        }
        (self.shims / "case.json").write_text(json.dumps(case))
        for name in ("openssl", "curl", "gh"):
            shim = self.shims / name
            shim.write_text(SHIM.format(python=sys.executable))
            shim.chmod(0o755)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-q", "-b", "dev")
        self.commit("c1")
        self.git("checkout", "-q", "-b", "side")
        self.commit("s1")
        self.git("checkout", "-q", "dev")
        self.commit("c2")
        self.git("merge", "-q", "--no-ff", "-m", "merge", "side")
        self.summary = self.root / "summary.md"
        self.gh_config = self.root / "ghconfig"
        self.gh_config.mkdir()
        self.parts = {
            "KEY": "key-" + "k" * 6,
            "KEYID": "keyid-" + "i" * 6,
            "ISSUER": "issuer-" + "s" * 6,
            "APPID": self.trap,
        }
        self.gh_token = "ghtoken-" + "t" * 6
        self.tmp_seen = self.root / "tmp"
        self.tmp_seen.mkdir()

    def git(self, *args):
        env = {
            "PATH": os.environ["PATH"],
            "HOME": str(self.root),
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_SYSTEM": os.devnull,
        }
        subprocess.run(
            ["git", *GIT_IDENTITY, *args], cwd=self.repo, env=env, check=True, capture_output=True
        )

    def commit(self, message):
        self.git("commit", "-q", "--allow-empty", "-m", message)

    def path(self):
        real = [str(Path(shutil.which(tool)).parent) for tool in ("git", "python3")]
        return os.pathsep.join([str(self.shims), *real])

    def run(self, command, placed=None, booleans=False, now=NOW):
        placed = list(PARTS) if placed is None else placed
        env = {
            "PATH": self.path(),
            "GH_TOKEN": self.gh_token,
            "GH_CONFIG_DIR": str(self.gh_config),
            "GH_REPO": "example/none",
            "GITHUB_STEP_SUMMARY": str(self.summary),
            "TMPDIR": str(self.tmp_seen),
            "HOME": str(self.root),
        }
        for part in PARTS:
            if booleans:
                env[part] = "true" if part in placed else "false"
            else:
                env[part] = self.parts[part] if part in placed else ""
        for tool in ("openssl", "curl", "gh"):
            found = shutil.which(tool, path=env["PATH"])
            assert found == str(self.shims / tool), f"{tool} is not the module's own shim"

        def umask():
            os.umask(0o022)

        done = subprocess.run(
            [sys.executable, str(SCRIPT), command, *(["--now", now] if now else [])],
            cwd=self.repo,
            env=env,
            capture_output=True,
            text=True,
            preexec_fn=umask,
            timeout=60,
        )
        return done

    def records(self, tool):
        log = self.shims / f"{tool}.log"
        if not log.exists():
            return []
        return [json.loads(line) for line in log.read_text().splitlines()]

    def summary_text(self):
        return self.summary.read_text() if self.summary.exists() else ""

    def leftovers(self):
        return list(self.tmp_seen.iterdir())


class CheckCase(unittest.TestCase):
    def fixture(self, **kwargs):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        return Fixture(scratch.name, **kwargs)

    def dispatched(self, fixture):
        return [record["argv"] for record in fixture.records("gh")]


class TheCheckDecides(CheckCase):
    def test_a9_a_build_due_within_the_lead_dispatches_the_lane_on_dev(self):
        cases = {
            "three days": (
                listing(build(2, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z")),
                "due: internal build 2 expires 2030-01-13T12:00:00Z, within 7 days; "
                "dispatched testflight-internal.yml on dev",
            ),
            "exactly the lead": (
                listing(build(2, "2029-12-01T00:00:00Z", "2030-01-17T12:00:00Z")),
                "due: internal build 2 expires 2030-01-17T12:00:00Z, within 7 days; "
                "dispatched testflight-internal.yml on dev",
            ),
        }
        for label, (body, line) in cases.items():
            with self.subTest(label):
                fixture = self.fixture(body=body)
                done = fixture.run("check")
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertIn(line, done.stdout)
                self.assertEqual(
                    self.dispatched(fixture),
                    [["workflow", "run", "testflight-internal.yml", "--ref", "dev"]],
                )
        with self.subTest("a newer build still processing"):
            body = listing(
                build(3, "2029-12-20T00:00:00Z", "2030-03-01T00:00:00Z", state="PROCESSING"),
                build(2, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z"),
            )
            fixture = self.fixture(body=body)
            done = fixture.run("check")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertIn("build 3 is still processing; nothing was dispatched", done.stdout)
            self.assertEqual(self.dispatched(fixture), [])

    def test_a10_a_build_beyond_the_lead_is_healthy_and_dispatches_nothing(self):
        body = listing(
            build(3, "2029-12-20T00:00:00Z", "2030-01-12T12:00:00Z", audience="APP_STORE_ELIGIBLE"),
            build(2, "2029-12-01T00:00:00Z", "2030-03-11T12:00:00Z"),
        )
        fixture = self.fixture(body=body)
        done = fixture.run("check")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(
            "healthy: internal build 2 expires 2030-03-11T12:00:00Z, more than 7 days away",
            done.stdout,
        )
        self.assertEqual(len(fixture.records("curl")), 1)
        self.assertEqual(self.dispatched(fixture), [])

    def test_a11_an_absent_or_half_placed_credential_fails_closed(self):
        sentence = "the check's credential is not placed; no build was read"
        roles = {
            "KEY": "absent: the API key",
            "KEYID": "absent: the API key's id",
            "ISSUER": "absent: the API key's issuer",
            "APPID": "absent: the app's id",
        }
        for command, booleans in (("preflight", True), ("check", False)):
            with self.subTest(command + " none"):
                fixture = self.fixture()
                done = fixture.run(command, placed=[], booleans=booleans)
                self.assertNotEqual(done.returncode, 0)
                self.assertIn(sentence, done.stdout)
                self.assertIn(sentence, fixture.summary_text())
                for tool in ("openssl", "curl", "gh"):
                    self.assertEqual(fixture.records(tool), [])
            with self.subTest(command + " half"):
                fixture = self.fixture()
                done = fixture.run(command, placed=["KEY", "APPID"], booleans=booleans)
                self.assertNotEqual(done.returncode, 0)
                lines = done.stdout.splitlines()
                self.assertIn(roles["KEYID"], lines)
                self.assertIn(roles["ISSUER"], lines)
                self.assertNotIn(roles["KEY"], lines)
                self.assertNotIn(roles["APPID"], lines)
                self.assertIn(roles["KEYID"], fixture.summary_text())
                for tool in ("openssl", "curl", "gh"):
                    self.assertEqual(fixture.records(tool), [])

    def test_a12_an_unreadable_answer_fails_closed(self):
        healthy = listing(build(2, "2029-12-01T00:00:00Z", "2030-03-11T12:00:00Z"))
        cases = {
            "a 401 carrying a healthy build": ({"body": healthy, "status": 401}, "status 401"),
            "a body that is not JSON": ({"body": "not json", "status": 200}, "not the expected"),
            "a curl failure": ({"curl_exit": 7}, "curl exited 7"),
            "an openssl failure": ({"openssl_exit": 1}, "signing exited 1"),
        }
        for label, (kwargs, named) in cases.items():
            with self.subTest(label):
                fixture = self.fixture(**kwargs)
                done = fixture.run("check")
                self.assertNotEqual(done.returncode, 0)
                self.assertIn(named, done.stdout)
                self.assertNotIn("healthy:", done.stdout)
                self.assertNotIn("healthy:", fixture.summary_text())
                self.assertEqual(self.dispatched(fixture), [])
                self.assertEqual(fixture.leftovers(), [])

    def test_a13_a_due_build_on_an_unmoved_dev_is_not_dispatched(self):
        cases = {
            "a live build": listing(build(3, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z")),
            "no live build": listing(
                build(3, "2029-12-01T00:00:00Z", "2029-12-30T12:00:00Z", expired=True)
            ),
        }
        for label, body in cases.items():
            with self.subTest(label):
                fixture = self.fixture(body=body)
                done = fixture.run("check")
                self.assertNotEqual(done.returncode, 0)
                self.assertIn(
                    "dev has not moved since build 3; a re-release needs a new commit on dev",
                    done.stdout,
                )
                self.assertEqual(self.dispatched(fixture), [])

    def test_a14_no_credential_part_reaches_a_child_or_the_log(self):
        body = listing(build(2, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z"))
        fixture = self.fixture(body=body)
        done = fixture.run("check")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(len(fixture.records("openssl")), 1, "one signature was asked for")
        self.assertEqual(len(fixture.records("curl")), 1, "one read was made")
        (openssl,) = fixture.records("openssl")
        (curl,) = fixture.records("curl")
        self.assertEqual(len(openssl["files"]), 1)
        (sign,) = list(openssl["files"].values())
        self.assertEqual(sign["mode"], 0o600)
        self.assertEqual(sign["dir_mode"], 0o700)
        headers = [curl["files"][arg[1:]] for arg in curl["argv"] if arg.startswith("@")]
        (header,) = headers
        line = header["content"].strip()
        self.assertTrue(line.startswith("Authorization: Bearer "))
        token = line.removeprefix("Authorization: Bearer ")
        config = curl["files"][curl["argv"][curl["argv"].index("--config") + 1]]
        self.assertIn(fixture.trap, config["content"])
        self.assertIn("https://", config["content"])
        head, payload, signature = token.split(".")

        def decode(segment):
            return base64.urlsafe_b64decode(segment + "=" * (-len(segment) % 4))

        self.assertEqual(
            json.loads(decode(head)), {"alg": "ES256", "kid": fixture.parts["KEYID"], "typ": "JWT"}
        )
        claims = json.loads(decode(payload))
        self.assertEqual(claims["iss"], fixture.parts["ISSUER"])
        self.assertEqual(claims["iat"], NOW_EPOCH)
        self.assertLessEqual(claims["exp"] - claims["iat"], 1200)
        self.assertGreater(claims["exp"], claims["iat"])
        self.assertEqual(claims["aud"], "appstoreconnect-v1")
        self.assertEqual(decode(signature), RAW)
        parts = [fixture.parts[part] for part in PARTS]
        for tool in ("openssl", "curl", "gh"):
            for record in fixture.records(tool):
                for part in parts:
                    self.assertNotIn(part, json.dumps(record["argv"]), tool)
                    self.assertNotIn(part, json.dumps(record["env"]), tool)
        for text in (done.stdout, done.stderr, fixture.summary_text()):
            for part in parts:
                self.assertNotIn(part, text)
        self.assertEqual(fixture.leftovers(), [])


def load_script():
    import importlib.util

    spec = importlib.util.spec_from_file_location("testflight_age_under_test", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def der_of(*integers):
    body = b"".join(b"\x02" + bytes([len(each)]) + each for each in integers)
    return b"\x30" + bytes([len(body)]) + body


class TheCheckHoldsItsEdges(CheckCase):
    """The hand sweep's survivors: each pins one property the first six cases left free."""

    def test_the_exit_codes_are_one_for_an_absent_credential_and_for_a_refusal(self):
        fixture = self.fixture()
        done = fixture.run("preflight", placed=[], booleans=True)
        self.assertEqual(done.returncode, 1)
        fixture = self.fixture(curl_exit=7)
        done = fixture.run("check")
        self.assertEqual(done.returncode, 1)

    def test_each_absent_part_is_named_by_its_role(self):
        fixture = self.fixture()
        done = fixture.run("check", placed=["KEYID", "ISSUER", "APPID"])
        self.assertEqual(done.stdout.splitlines(), ["absent: the API key"])
        fixture = self.fixture()
        done = fixture.run("check", placed=["KEY"])
        self.assertEqual(
            done.stdout.splitlines(),
            ["absent: the API key's id", "absent: the API key's issuer", "absent: the app's id"],
        )

    def test_a_placed_credential_and_an_unknown_command(self):
        fixture = self.fixture()
        done = fixture.run("preflight", booleans=True)
        self.assertEqual(done.returncode, 0)
        self.assertEqual(done.stdout, "the credential is placed\n")
        done = self.fixture().run("unknown")
        self.assertEqual(done.returncode, 2)

    def test_the_summary_is_appended_and_ends_each_line(self):
        fixture = self.fixture()
        first = fixture.run("preflight", placed=[], booleans=True)
        self.assertEqual(fixture.summary_text(), first.stdout)
        second = fixture.run("preflight", placed=["KEY"], booleans=True)
        self.assertEqual(fixture.summary_text(), first.stdout + second.stdout)

    def test_the_real_clock_is_zoned(self):
        body = listing(build(2, "2029-12-01T00:00:00Z", "2099-03-11T12:00:00Z"))
        fixture = self.fixture(body=body)
        done = fixture.run("check", now=None)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("healthy: internal build 2", done.stdout)

    def test_a_build_just_beyond_the_lead_is_healthy(self):
        body = listing(build(2, "2029-12-01T00:00:00Z", "2030-01-17T12:00:01Z"))
        fixture = self.fixture(body=body)
        done = fixture.run("check")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("healthy: internal build 2", done.stdout)
        self.assertEqual(self.dispatched(fixture), [])

    def test_the_live_build_is_named_with_its_dates(self):
        body = listing(build(2, "2029-12-01T00:00:00Z", "2030-03-11T12:00:00Z"))
        done = self.fixture(body=body).run("check")
        self.assertIn(
            "newest live build 2: uploaded 2029-12-01T00:00:00Z, expires 2030-03-11T12:00:00Z",
            done.stdout.splitlines(),
        )

    def test_what_makes_a_build_live(self):
        due = build(2, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z")
        cases = {
            "an invalid build is not live": build(
                1, "2029-12-25T00:00:00Z", "2030-03-01T00:00:00Z", state="INVALID"
            ),
            "an expired build is not live": build(
                1, "2029-12-25T00:00:00Z", "2030-03-01T00:00:00Z", expired=True
            ),
        }
        for label, other in cases.items():
            with self.subTest(label):
                fixture = self.fixture(body=listing(other, due))
                done = fixture.run("check")
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertIn("due: internal build 2 expires 2030-01-13T12:00:00Z", done.stdout)
        with self.subTest("expired must be exactly true"):
            truthy = build(1, "2029-12-25T00:00:00Z", "2030-03-01T00:00:00Z", expired="yes")
            done = self.fixture(body=listing(truthy)).run("check")
            self.assertIn("healthy: internal build 1", done.stdout)

    def test_the_newest_upload_is_the_live_build(self):
        older = build(1, "2029-12-01T00:00:00Z", "2030-03-01T00:00:00Z")
        newer = build(2, "2029-12-20T00:00:00Z", "2030-01-13T12:00:00Z")
        for label, body in {
            "older first": listing(older, newer),
            "newer first": listing(newer, older),
        }.items():
            with self.subTest(label):
                fixture = self.fixture(body=body)
                done = fixture.run("check")
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertIn("newest live build 2:", done.stdout)
                self.assertEqual(len(self.dispatched(fixture)), 1)

    def test_only_a_newer_build_that_is_processing_holds_the_dispatch(self):
        due = build(2, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z")
        cases = {
            "a newer build that failed": (
                build(1, "2029-12-20T00:00:00Z", "2030-03-01T00:00:00Z", state="FAILED"),
                1,
            ),
            "a processing build uploaded at the same instant": (
                build(1, "2029-12-01T00:00:00Z", "2030-03-01T00:00:00Z", state="PROCESSING"),
                1,
            ),
        }
        for label, (other, count) in cases.items():
            with self.subTest(label):
                fixture = self.fixture(body=listing(other, due))
                done = fixture.run("check")
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertEqual(len(self.dispatched(fixture)), count)
                self.assertNotIn("still processing", done.stdout)

    def test_with_no_live_build_the_newest_is_compared(self):
        prefix = "no internal build is live; "
        dispatched = prefix + "dispatched testflight-internal.yml on dev"
        with self.subTest("an expired build and a moved dev"):
            body = listing(build(1, "2029-12-01T00:00:00Z", "2029-12-30T12:00:00Z", expired=True))
            fixture = self.fixture(body=body)
            done = fixture.run("check")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertIn(dispatched, done.stdout.splitlines())
        with self.subTest("a build still processing"):
            body = listing(
                build(3, "2029-12-01T00:00:00Z", "2030-03-01T00:00:00Z", state="PROCESSING")
            )
            fixture = self.fixture(body=body)
            done = fixture.run("check")
            self.assertIn("build 3 is still processing; nothing was dispatched", done.stdout)
            self.assertEqual(self.dispatched(fixture), [])
        with self.subTest("no builds at all"):
            fixture = self.fixture(body=listing())
            done = fixture.run("check")
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertIn(dispatched, done.stdout.splitlines())
            self.assertEqual(len(self.dispatched(fixture)), 1)

    def test_a_refused_dispatch_fails_the_run(self):
        body = listing(build(2, "2029-12-01T00:00:00Z", "2030-01-13T12:00:00Z"))
        done = self.fixture(body=body, gh_exit=1).run("check")
        self.assertEqual(done.returncode, 1)
        self.assertIn("gh exited 1", done.stdout)

    def test_the_read_is_made_as_specified(self):
        body = listing(build(2, "2029-12-01T00:00:00Z", "2030-03-11T12:00:00Z"))
        fixture = self.fixture(body=body)
        done = fixture.run("check")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        (curl,) = fixture.records("curl")
        argv = curl["argv"]
        self.assertEqual(argv[:5], ["--silent", "--show-error", "--max-time", "30", "--config"])
        self.assertEqual(argv[6], "-H")
        self.assertTrue(argv[7].startswith("@"))
        self.assertEqual(argv[8], "--output")
        self.assertEqual(argv[10:], ["--write-out", "%{http_code}"])
        self.assertEqual(len(argv), 12)
        config = curl["files"][argv[5]]["content"]
        fields = "version,uploadedDate,expirationDate,expired,processingState,buildAudienceType"
        self.assertEqual(
            config,
            f'url = "https://api.appstoreconnect.apple.com/v1/apps/{fixture.trap}/builds'
            f'?limit=50&sort=-uploadedDate&fields%5Bbuilds%5D={fields}"\n',
        )
        (openssl,) = fixture.records("openssl")
        self.assertEqual(openssl["argv"][:3], ["dgst", "-sha256", "-sign"])
        self.assertEqual(len(openssl["argv"]), 4)
        header = curl["files"][argv[7][1:]]["content"]
        head = header.strip().removeprefix("Authorization: Bearer ").split(".")[0]
        raw = base64.urlsafe_b64decode(head + "=" * (-len(head) % 4))
        self.assertEqual(
            raw, ('{"alg":"ES256","kid":"%s","typ":"JWT"}' % fixture.parts["KEYID"]).encode()
        )

    def test_an_answer_of_any_other_shape_is_refused(self):
        good = {
            "type": "builds",
            "attributes": {
                "version": "2",
                "uploadedDate": "2029-12-01T00:00:00Z",
                "expirationDate": "2030-03-11T12:00:00Z",
                "expired": False,
                "processingState": "VALID",
                "buildAudienceType": "INTERNAL_ONLY",
            },
        }

        def with_attrs(**changes):
            return json.dumps({"data": [{"attributes": {**good["attributes"], **changes}}]})

        bodies = {
            "a list for the answer": "[]",
            "an attribute missing": '{"data": [{"attributes": {}}]}',
            "a date that is a number": with_attrs(uploadedDate=5),
            "a version with a space": with_attrs(version=" 2"),
            "a version that is a number": with_attrs(version=2),
            "a date with no zone": with_attrs(uploadedDate="2029-12-01T00:00:00"),
        }
        for label, body in bodies.items():
            with self.subTest(label):
                fixture = self.fixture(body=body)
                done = fixture.run("check")
                self.assertEqual(done.returncode, 1)
                self.assertEqual(
                    done.stdout.splitlines(), ["failed: the answer is not the expected JSON"]
                )
                self.assertEqual(self.dispatched(fixture), [])

    def test_the_signature_conversion_refuses_what_is_not_der(self):
        module = load_script()
        sentence = "failed: the signature is not DER"
        bad = {
            "empty": b"",
            "truncated": b"\x30",
            "not a sequence": b"\x31" + DER[1:],
            "no integer": b"\x30\x04\x03\x01\x00\x00",
            "an integer over 32 bytes": der_of(b"\x01" * 33, b"\x05"),
            "a missing second integer": b"\x30\x03\x02\x01\x05",
        }
        for label, der in bad.items():
            with self.subTest(label):
                with self.assertRaises(module.Failed) as caught:
                    module.raw_signature(der)
                self.assertEqual(str(caught.exception), sentence)
        self.assertEqual(
            module.raw_signature(der_of(b"\x00" + b"\xff" * 32, b"\x05")),
            b"\xff" * 32 + b"\x00" * 31 + b"\x05",
        )
        self.assertEqual(len(module.raw_signature(der_of(b"\x01" * 32, b"\x05"))), 64)
        self.assertEqual(module.raw_signature(DER), RAW)

    def test_a_segment_is_url_safe_and_unpadded(self):
        module = load_script()
        self.assertEqual(module.segment(b"a"), "YQ")
        self.assertEqual(module.segment(b"\xfb\xff"), "-_8")

    def test_an_instant_without_a_zone_is_refused(self):
        module = load_script()
        with self.assertRaises(ValueError):
            module.instant("2030-01-10T12:00:00")
        self.assertEqual(module.instant("2030-01-10T12:00:00Z").utcoffset().total_seconds(), 0)

    def test_a_private_file_is_never_overwritten(self):
        module = load_script()
        with tempfile.TemporaryDirectory() as scratch:
            target = Path(scratch) / "private"
            module.private_file(target, "one")
            with self.assertRaises(FileExistsError):
                module.private_file(target, "two")
            self.assertEqual(target.read_text(), "one")

    def test_the_git_and_gh_children_get_no_credential_and_a_git_failure_is_named(self):
        from types import SimpleNamespace
        from unittest import mock

        module = load_script()
        placed = {part: "value-" + part for part in PARTS}
        seen = []

        def record(argv, **kwargs):
            seen.append(kwargs.get("env"))
            return SimpleNamespace(returncode=0, stdout="7\n", stderr="")

        with (
            mock.patch.dict(os.environ, placed),
            mock.patch.object(module.subprocess, "run", side_effect=record),
        ):
            self.assertEqual(module.first_parent_count(), 7)
            module.dispatch()
        self.assertEqual(len(seen), 2)
        for env in seen:
            self.assertIsNotNone(env)
            for part in PARTS:
                self.assertNotIn(part, env)
        failure = SimpleNamespace(returncode=128, stdout="", stderr="")
        with mock.patch.object(module.subprocess, "run", return_value=failure):
            with self.assertRaises(module.Failed) as caught:
                module.first_parent_count()
        self.assertEqual(str(caught.exception), "failed: git exited 128")

    def test_the_last_integer_is_read_by_its_own_length_byte(self):
        module = load_script()
        self.assertEqual(module.raw_signature(DER + b"\x99"), RAW)

    def test_a_full_width_second_integer_is_read_whole(self):
        module = load_script()
        each = bytes(range(1, 33))
        self.assertEqual(module.raw_signature(der_of(each, each)), each + each)

    def test_only_the_absent_part_is_named_missing(self):
        from unittest import mock

        module = load_script()
        placed = {part: "value-" + part for part in PARTS[:3]}
        with mock.patch.dict(os.environ, placed):
            os.environ.pop(PARTS[3], None)
            self.assertEqual(module.missing_parts(False), [PARTS[3]])

    def test_the_git_and_gh_children_capture_their_output_as_text(self):
        from types import SimpleNamespace
        from unittest import mock

        module = load_script()
        seen = []

        def record(argv, **kwargs):
            seen.append((kwargs.get("capture_output"), kwargs.get("text")))
            return SimpleNamespace(returncode=0, stdout="7\n", stderr="")

        with mock.patch.object(module.subprocess, "run", side_effect=record):
            self.assertEqual(module.first_parent_count(), 7)
            module.dispatch()
        self.assertEqual(seen, [(True, True), (True, True)])

    def test_the_refusal_of_a_clock_without_a_zone_says_so(self):
        module = load_script()
        with self.assertRaisesRegex(ValueError, "no zone"):
            module.instant("2030-01-10T12:00:00")

    def test_the_help_describes_the_clock_option(self):
        import contextlib

        module = load_script()

        class Sink:
            def __init__(self):
                self.parts = []

            def write(self, text):
                self.parts.append(text)

        sink = Sink()
        with contextlib.redirect_stdout(sink):
            with self.assertRaises(SystemExit) as caught:
                module.main(["--help"])
        self.assertEqual(caught.exception.code, 0)
        self.assertIn("the clock, for the tests only", " ".join("".join(sink.parts).split()))


if __name__ == "__main__":
    unittest.main()

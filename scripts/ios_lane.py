#!/usr/bin/env python3
"""The TestFlight lane's steps, one verb per workflow step (SPEC-352, ADR-363).

`python3 scripts/ios_lane.py <verb>` runs one step of a lane's job: `plan` on Linux, and
`preflight`, `sign`, `upload-to-testflight`, `clean`, `build-unsigned` and `summary` on the macOS
runner. Standard library only.

`plan --lane internal|release` decides, before any macOS job starts, whether this run may build at
all, and what it builds: it refuses an event or a ref the lane does not take and a shallow
checkout, and for a release a tag that is not SemVer, a lightweight tag, a commit that is not on
`main`'s first-parent chain and a tag whose version is not the workspace's (SPEC-352 R3 to R5). It
writes `lane`, `number` (the first-parent count of the commit built) and `version` (the workspace
version) to `GITHUB_OUTPUT`.

The credential steps read each credential part from their own step's environment under a role
word, `KEY`, `KEYID`, `ISSUER`, `CERTIFICATE`, `PASSWORD` or `PROFILE`, and give none of the six
to a child: every tool runs with the step's environment minus those names (R12). The certificate
and the profile arrive as base64 text. Every file a step writes lives under `$RUNNER_TEMP/lane`,
except the installed profile and the ignored include file, and `clean` removes all of them (R11).

- `preflight` reads the six parts as presence booleans and writes `placed=all` or `placed=none`,
  or refuses a half-placed credential, naming each absent part by its role (R7).
- `sign` reads the team and the app id from the profile, masks every private value before any
  tool runs, refuses a development, expired or placeholder profile, re-wraps the certificate under
  a generated passphrase, imports it non-extractable into a job keychain, refuses a profile not
  issued for that certificate, installs the profile, writes the include file and archives the
  harness (R8, R9, R13).
- `upload-to-testflight` writes the key into a fresh private directory for the export alone and
  removes that directory on any exit (R10).
- `clean` restores the keychain search list, deletes the job keychain, and removes the profile,
  the include file and the lane's directory; with nothing to remove it runs no tool (R11).
- `build-unsigned` builds Release for a generic device with signing off (R7).
- `summary` appends the run's summary: lane, commit, version, build, credential, outcome (R20).

A build tool's full output goes to a file under the lane, and a step prints only that output's
error and warning lines, each private value redacted (R13). A refusal is one line on standard error
and exit 1, and writes no output.
"""

import argparse
import base64
import datetime
import hashlib
import json
import os
import plistlib
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

VERBS = ("plan", "preflight", "sign", "upload-to-testflight", "clean", "build-unsigned", "summary")
# A release tag, exactly as release.yml's tag guard spells it.
SEMVER_TAG = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
# The form `CFBundleShortVersionString` takes: three dot-separated integers.
MARKETING_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
SHALLOW = "the checkout is shallow; the build number needs the full history"
# The actor a workflow's own token runs as: a release run it started is refused (SPEC-405 R4).
WORKFLOW_TOKEN = "github-actions[bot]"
# The six credential parts, each by the variable its step reads it from and the role a refusal
# names it by, in the order the preflight lists them.
ROLES = (
    ("KEY", "the upload key"),
    ("KEYID", "the key id"),
    ("ISSUER", "the issuer id"),
    ("CERTIFICATE", "the certificate"),
    ("PASSWORD", "the certificate's password"),
    ("PROFILE", "the profile"),
)
# The harness's Release build for a generic device, which both builds take.
HARNESS = (
    "-project",
    "ios/Harness.xcodeproj",
    "-scheme",
    "Harness",
    "-configuration",
    "Release",
    "-destination",
    "generic/platform=iOS",
)
# The ignored include file `ios/Config/Harness.xcconfig` names last.
INCLUDE = "ios/Config/Signing.local.xcconfig"
# Where Xcode reads installed provisioning profiles from, under `HOME`.
PROFILES = "Library/Developer/Xcode/UserData/Provisioning Profiles"
# What `security find-identity` lists for each identity: its index, SHA-1 and quoted name.
IDENTITY = re.compile(r"\) ([0-9A-F]{40}) \"")


class Refused(Exception):
    """A step's refusal, named by its one-line message."""


def git(*args):
    """Run git in the checkout and return the completed process."""
    return subprocess.run(["git", *args], capture_output=True, text=True, check=False)


def plan(lane):
    """Decide what the lane builds, or refuse; return the outputs the later jobs read."""
    import tomllib

    event = os.environ["GITHUB_EVENT_NAME"]
    ref = os.environ["GITHUB_REF"]
    if lane == "internal":
        if event not in ("workflow_dispatch", "push"):
            raise Refused(
                f"the internal lane runs on workflow_dispatch or push only, not on {event}"
            )
        if ref != "refs/heads/dev":
            raise Refused(f"the internal lane builds refs/heads/dev only, not {ref}")
    elif event not in ("push", "workflow_dispatch") or os.environ.get("GITHUB_REF_TYPE") != "tag":
        raise Refused(
            f"the release lane runs on a tag's push or dispatch only, not on {event} of {ref}"
        )
    elif os.environ.get("GITHUB_ACTOR") == WORKFLOW_TOKEN:
        raise Refused(f"the release lane takes no run that {WORKFLOW_TOKEN} started")
    if git("rev-parse", "--is-shallow-repository").stdout.strip() != "false":
        raise Refused(SHALLOW)
    with open("Cargo.toml", "rb") as manifest:
        version = tomllib.load(manifest)["workspace"]["package"]["version"]
    if not MARKETING_VERSION.fullmatch(version):
        raise Refused(f"the workspace version {version} is not three dot-separated integers")
    commit = "HEAD"
    if lane == "release":
        tag = os.environ["GITHUB_REF_NAME"]
        commit = os.environ["GITHUB_SHA"]
        if not SEMVER_TAG.fullmatch(tag):
            raise Refused(f"the tag {tag} is not a SemVer release tag")
        if git("cat-file", "-t", f"refs/tags/{tag}").stdout.strip() != "tag":
            raise Refused(f"the tag {tag} is a lightweight tag; a release is an annotated tag")
        if git("merge-base", "--is-ancestor", commit, "origin/main").returncode != 0:
            raise Refused(f"the commit of {tag} is not on main")
        if commit not in git("rev-list", "--first-parent", "origin/main").stdout.split():
            raise Refused(f"the commit of {tag} is not on main's first-parent chain")
        if tag[1:] != version:
            raise Refused(f"the tag {tag} names {tag[1:]}, but the workspace version is {version}")
    number = git("rev-list", "--count", "--first-parent", commit).stdout.strip()
    return {"lane": lane, "number": number, "version": version}


def write_outputs(outputs):
    """Append each output to the file the runner reads a step's outputs from."""
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as sink:
        for name, value in outputs.items():
            sink.write(f"{name}={value}\n")


def preflight():
    """All six parts placed, or none; a half-placed credential is refused by its absent roles."""
    absent = [word for name, word in ROLES if os.environ[name] != "true"]
    if len(absent) == len(ROLES):
        return {"placed": "none"}
    if absent:
        raise Refused(f"the credential is half placed; absent: {', '.join(absent)}")
    return {"placed": "all"}


def lane_directory():
    """The directory a credential step writes in, under the runner's temporary directory."""
    return Path(os.environ["RUNNER_TEMP"], "lane")


def make_lane():
    """Create the lane's directory, which only the runner's user may read, and return it."""
    lane = lane_directory()
    lane.mkdir(mode=0o700)
    return lane


def without_credential():
    """The step's environment minus the six credential parts: what every child is given."""
    parts = {name for name, _ in ROLES}
    return {name: value for name, value in os.environ.items() if name not in parts}


def tool(*args, stdin=None):
    """Run one signing tool with no credential part in its environment and its output kept in
    memory, never printed; return that output, or refuse when the tool fails."""
    done = subprocess.run(
        args, input=stdin, env=without_credential(), capture_output=True, text=True
    )
    if done.returncode != 0:
        raise Refused(f"{args[0]} {args[1]} failed with exit {done.returncode}")
    return done.stdout


def xcodebuild(private, *args):
    """Run xcodebuild with no credential part in its environment and its full output in a file
    under the lane. Print only that output's error and warning lines, each private value replaced,
    longest first so that a value inside another is never left half shown; refuse when it
    fails."""
    log = lane_directory() / f"xcodebuild-{args[0].lstrip('-')}.log"
    with open(log, "w", encoding="utf-8") as sink:
        done = subprocess.run(
            ["xcodebuild", *args], env=without_credential(), stdout=sink, stderr=subprocess.STDOUT
        )
    for line in log.read_text(encoding="utf-8").splitlines():
        if "error:" in line or "warning:" in line:
            for value in sorted(private, key=len, reverse=True):
                line = line.replace(value, "<redacted>")
            print(line)
    if done.returncode != 0:
        raise Refused(f"xcodebuild {args[0]} failed with exit {done.returncode}")


def numbers():
    """The build settings the plan decided: the build number, the version and the lane."""
    return [
        f"CURRENT_PROJECT_VERSION={os.environ['NUMBER']}",
        f"MARKETING_VERSION={os.environ['VERSION']}",
        f"DS_LANE={os.environ['LANE']}",
    ]


def write_private(path, data):
    """Write `data` to a new file only its owner may read or write; never reuse a file."""
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as sink:
        sink.write(data)


def sign():
    """Check the profile, import the certificate into a job keychain, install the profile, write
    the include file and archive the harness (SPEC-352 R8, R9, R13)."""
    blob = base64.b64decode(os.environ["PROFILE"])
    profile = plistlib.loads(re.search(rb"<\?xml.*</plist>", blob, re.DOTALL).group())
    entitlements = profile["Entitlements"]
    team = profile["TeamIdentifier"][0]
    app = entitlements["application-identifier"].removeprefix(f"{team}.")
    keychain_password, passphrase = secrets.token_hex(16), secrets.token_hex(16)
    private = (team, profile["TeamName"], app, profile["Name"], profile["UUID"])
    for value in (*private, keychain_password, passphrase):
        print(f"::add-mask::{value}", flush=True)
    if entitlements.get("get-task-allow") or "ProvisionedDevices" in profile:
        raise Refused("the profile is a development profile, not an App Store distribution profile")
    if profile["ExpirationDate"] <= datetime.datetime.now(datetime.UTC).replace(tzinfo=None):
        raise Refused("the profile has expired")
    if app.split(".")[0] == "invalid":
        raise Refused("the profile's app id is the placeholder, whose first label is invalid")
    lane = make_lane()
    original = lane / "original.p12"
    write_private(original, base64.b64decode(os.environ["CERTIFICATE"]))
    pem = tool(
        "openssl",
        "pkcs12",
        "-in",
        str(original),
        "-passin",
        "stdin",
        "-passout",
        f"pass:{passphrase}",
        stdin=f"{os.environ['PASSWORD']}\n",
    )
    original.unlink()
    bundle = lane / "certificate.p12"
    tool(
        "openssl",
        "pkcs12",
        "-export",
        "-out",
        str(bundle),
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
        stdin=pem,
    )
    keychain = str(lane / "lane.keychain-db")
    tool("security", "create-keychain", "-p", keychain_password, keychain)
    tool("security", "set-keychain-settings", "-lut", "21600", keychain)
    tool("security", "unlock-keychain", "-p", keychain_password, keychain)
    tool(
        "security",
        "import",
        str(bundle),
        "-k",
        keychain,
        "-P",
        passphrase,
        "-f",
        "pkcs12",
        "-x",
        "-T",
        "/usr/bin/codesign",
    )
    bundle.unlink()
    tool(
        "security",
        "set-key-partition-list",
        "-S",
        "apple-tool:,apple:,codesign:",
        "-s",
        "-k",
        keychain_password,
        keychain,
    )
    saved = re.findall(r'"([^"]*)"', tool("security", "list-keychains", "-d", "user"))
    (lane / "search-list").write_text(json.dumps(saved), encoding="utf-8")
    tool("security", "list-keychains", "-d", "user", "-s", keychain, *saved)
    listed = IDENTITY.findall(
        tool("security", "find-identity", "-v", "-p", "codesigning", keychain)
    )
    issued = {hashlib.sha1(der).hexdigest().upper() for der in profile["DeveloperCertificates"]}
    matched = [identity for identity in listed if identity in issued]
    if not matched:
        raise Refused("the profile was not issued for the imported certificate")
    signing = {
        "team": team,
        "app id": app,
        "uuid": profile["UUID"],
        "identity": matched[0],
        "private": private,
    }
    (lane / "signing.json").write_text(json.dumps(signing), encoding="utf-8")
    profiles = Path(os.environ["HOME"], PROFILES)
    profiles.mkdir(parents=True, exist_ok=True)
    (profiles / f"{profile['UUID']}.mobileprovision").write_bytes(blob)
    Path(INCLUDE).write_text(
        "// The lane's signing, written by its sign step and removed by its clean step.\n"
        f"DS_APP_ID = {app}\n"
        f"DEVELOPMENT_TEAM = {team}\n"
        "CODE_SIGN_STYLE = Manual\n"
        f"CODE_SIGN_IDENTITY = {matched[0]}\n"
        f"PROVISIONING_PROFILE_SPECIFIER = {profile['UUID']}\n",
        encoding="utf-8",
    )
    xcodebuild(
        private, "archive", *HARNESS, "-archivePath", str(lane / "app.xcarchive"), *numbers()
    )


def upload():
    """Export the archive and upload it, the key in a private directory for this step alone,
    removed on any exit (SPEC-352 R10)."""
    lane = lane_directory()
    signing = json.loads((lane / "signing.json").read_text(encoding="utf-8"))
    options = {
        "destination": "upload",
        "manageAppVersionAndBuildNumber": False,
        "method": "app-store-connect",
        "provisioningProfiles": {signing["app id"]: signing["uuid"]},
        "signingCertificate": signing["identity"],
        "signingStyle": "manual",
        "teamID": signing["team"],
        "testFlightInternalTestingOnly": True,
    }
    (lane / "export-options.plist").write_bytes(plistlib.dumps(options))
    private = (*signing["private"], os.environ["KEYID"], os.environ["ISSUER"])
    directory = Path(tempfile.mkdtemp(prefix="key-", dir=lane))
    try:
        key = directory / "AuthKey.p8"
        write_private(key, os.environ["KEY"].encode())
        xcodebuild(
            private,
            "-exportArchive",
            "-archivePath",
            str(lane / "app.xcarchive"),
            "-exportPath",
            str(lane / "export"),
            "-exportOptionsPlist",
            str(lane / "export-options.plist"),
            "-authenticationKeyPath",
            str(key),
            "-authenticationKeyID",
            os.environ["KEYID"],
            "-authenticationKeyIssuerID",
            os.environ["ISSUER"],
        )
    finally:
        shutil.rmtree(directory)


def clean():
    """Restore the keychain search list, delete the job keychain, and remove the installed
    profile, the include file and the lane's directory, wherever any remains (SPEC-352 R11)."""
    lane = lane_directory()
    search = lane / "search-list"
    if search.exists():
        saved = json.loads(search.read_text(encoding="utf-8"))
        tool("security", "list-keychains", "-d", "user", "-s", *saved)
    keychain = lane / "lane.keychain-db"
    if keychain.exists():
        tool("security", "delete-keychain", str(keychain))
    signing = lane / "signing.json"
    if signing.exists():
        uuid = json.loads(signing.read_text(encoding="utf-8"))["uuid"]
        Path(os.environ["HOME"], PROFILES, f"{uuid}.mobileprovision").unlink(missing_ok=True)
    Path(INCLUDE).unlink(missing_ok=True)
    if lane.exists():
        shutil.rmtree(lane)


def build_unsigned():
    """Build the harness for a generic device with signing off: the run with no credential."""
    lane = make_lane()
    xcodebuild(
        (),
        "build",
        *HARNESS,
        "-derivedDataPath",
        str(lane / "build"),
        "CODE_SIGNING_ALLOWED=NO",
        *numbers(),
    )


def summary():
    """Append the run's summary: the lane, the commit, the version and build, whether the
    credential was placed, and the upload's outcome (SPEC-352 R20)."""
    placed, outcome = os.environ["PLACED"], os.environ["OUTCOME"]
    if placed == "none":
        credential = "not placed"
        said = "stopped before the upload: the credential is not placed"
    elif placed != "all":
        credential, said = "half placed", "stopped at the preflight"
    elif outcome == "success":
        credential, said = "placed", "uploaded"
    else:
        credential, said = "placed", "not uploaded: the signing or the upload failed"
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as sink:
        sink.write(
            f"### TestFlight: the {os.environ['LANE']} lane\n\n"
            f"- commit: {os.environ['GITHUB_SHA'][:12]}\n"
            f"- version: {os.environ['VERSION']}, build {os.environ['NUMBER']}\n"
            f"- credential: {credential}\n"
            f"- outcome: {said}\n"
        )


STEPS = {
    "sign": sign,
    "upload-to-testflight": upload,
    "clean": clean,
    "build-unsigned": build_unsigned,
    "summary": summary,
}


def main(argv=None):
    """Run one step; a refusal is its one line on standard error and exit 1."""
    parser = argparse.ArgumentParser()
    parser.add_argument("verb", choices=VERBS)
    parser.add_argument("--lane", choices=("internal", "release"))
    args = parser.parse_args(argv)
    try:
        if args.verb == "plan":
            outputs = plan(args.lane)
            write_outputs(outputs)
            print(f"the {args.lane} lane builds {outputs['version']} ({outputs['number']})")
        elif args.verb == "preflight":
            write_outputs(preflight())
        else:
            STEPS[args.verb]()
    except Refused as refused:
        sys.exit(str(refused))


if __name__ == "__main__":
    main()

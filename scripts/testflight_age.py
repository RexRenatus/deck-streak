"""Read the newest internal build's expiration and start the internal lane before it stops working
(SPEC-374 R15 to R21).

`preflight` reads four presence booleans. `check` reads the four credential parts, signs a short-lived
token, lists the app's builds, decides, and dispatches the internal lane on `dev` when the live build
is due. Standard library only. No credential part reaches a child's argv or environment, or the log.
"""

import argparse
import base64
import json
import os
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path

LEAD_DAYS = 7
ABSENT_EXIT = 1
FAILED_EXIT = 1
DISPATCH_REF = "dev"
WORKFLOW = "testflight-internal.yml"
PARTS = ("KEY", "KEYID", "ISSUER", "APPID")
ROLES = {
    "KEY": "the API key",
    "KEYID": "the API key's id",
    "ISSUER": "the API key's issuer",
    "APPID": "the app's id",
}
NONE_PLACED = "the check's credential is not placed; no build was read"
HOST = "https://api.appstoreconnect.apple.com"
FIELDS = "version,uploadedDate,expirationDate,expired,processingState,buildAudienceType"

HEALTHY_LINE = "healthy: internal build {number} expires {expires}, more than 7 days away"
DUE_LINE = "due: internal build {number} expires {expires}, within 7 days; "
NO_LIVE_LINE = "no internal build is live; "
DISPATCHED = "dispatched testflight-internal.yml on dev"
PROCESSING_LINE = "build {number} is still processing; nothing was dispatched"
UNMOVED_LINE = "dev has not moved since build {number}; a re-release needs a new commit on dev"


class Failed(Exception):
    """A named failure: the run exits non-zero with this sentence and nothing else."""


def private_file(path, content):
    """The one way a private file is made: new, owner-only, never overwritten."""
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as sink:
        sink.write(content)


def segment(raw):
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode("ascii")


def raw_signature(der):
    """The 64-byte r||s of a DER ECDSA signature, each integer stripped of its sign byte and
    left-padded to 32 bytes, as a JWS wants it."""
    try:
        if der[0] != 0x30 or der[2] != 0x02:
            raise ValueError
        size = der[3]
        r = der[4 : 4 + size]
        if der[4 + size] != 0x02:
            raise ValueError
        s = der[6 + size : 6 + size + der[5 + size]]
    except (IndexError, ValueError):
        raise Failed("failed: the signature is not DER") from None
    pieces = []
    for integer in (r, s):
        integer = integer.lstrip(b"\x00")
        if len(integer) > 32:
            raise Failed("failed: the signature is not DER")
        pieces.append(integer.rjust(32, b"\x00"))
    return b"".join(pieces)


def child_env():
    return {key: value for key, value in os.environ.items() if key not in PARTS}


def missing_parts(booleans):
    values = {part: os.environ.get(part, "") for part in PARTS}
    if booleans:
        return [part for part in PARTS if values[part] != "true"]
    return [part for part in PARTS if values[part] == ""]


def sentences(missing):
    if len(missing) == len(PARTS):
        return [NONE_PLACED]
    return [f"absent: {ROLES[part]}" for part in missing]


def token(now, workdir):
    key_file = Path(workdir) / "key.p8"
    private_file(key_file, os.environ["KEY"])
    header = {"alg": "ES256", "kid": os.environ["KEYID"], "typ": "JWT"}
    iat = int(now.timestamp())
    claims = {
        "iss": os.environ["ISSUER"],
        "iat": iat,
        "exp": iat + 1200,
        "aud": "appstoreconnect-v1",
    }
    signing = ".".join(
        segment(json.dumps(part, separators=(",", ":")).encode()) for part in (header, claims)
    )
    done = subprocess.run(
        ["openssl", "dgst", "-sha256", "-sign", str(key_file)],
        input=signing.encode(),
        capture_output=True,
        env=child_env(),
    )
    if done.returncode != 0:
        raise Failed(f"failed: signing exited {done.returncode}")
    return signing + "." + segment(raw_signature(done.stdout))


def fetch(now, workdir):
    bearer = token(now, workdir)
    header_file = Path(workdir) / "header"
    config_file = Path(workdir) / "curl.cfg"
    answer = Path(workdir) / "answer.json"
    private_file(header_file, f"Authorization: Bearer {bearer}\n")
    url = (
        f"{HOST}/v1/apps/{os.environ['APPID']}/builds?limit=50&sort=-uploadedDate"
        f"&fields%5Bbuilds%5D={FIELDS}"
    )
    private_file(config_file, f'url = "{url}"\n')
    done = subprocess.run(
        [
            "curl",
            "--silent",
            "--show-error",
            "--max-time",
            "30",
            "--config",
            str(config_file),
            "-H",
            f"@{header_file}",
            "--output",
            str(answer),
            "--write-out",
            "%{http_code}",
        ],
        capture_output=True,
        text=True,
        env=child_env(),
    )
    if done.returncode != 0:
        raise Failed(f"failed: curl exited {done.returncode}")
    status = done.stdout.strip()
    if status != "200":
        raise Failed(f"failed: the store answered status {status}")
    return answer.read_text(encoding="utf-8")


def instant(text):
    value = datetime.fromisoformat(text.replace("Z", "+00:00"))
    if value.tzinfo is None:
        raise ValueError("no zone")
    return value


def parse(body):
    """The builds, newest upload first, or a named failure for any shape but the expected."""
    unexpected = Failed("failed: the answer is not the expected JSON")
    builds = []
    try:
        for item in json.loads(body)["data"]:
            attrs = item["attributes"]
            number = attrs["version"]
            if not isinstance(number, str) or not number.isdigit():
                raise unexpected
            builds.append(
                {
                    "number": int(number),
                    "uploaded": instant(attrs["uploadedDate"]),
                    "uploaded_text": attrs["uploadedDate"],
                    "expires": instant(attrs["expirationDate"]),
                    "expires_text": attrs["expirationDate"],
                    "expired": attrs["expired"] is True,
                    "state": attrs["processingState"],
                    "audience": attrs["buildAudienceType"],
                }
            )
    except Failed:
        raise
    except (ValueError, KeyError, TypeError, AttributeError):
        raise unexpected from None
    return sorted(builds, key=lambda each: each["uploaded"], reverse=True)


def first_parent_count():
    done = subprocess.run(
        ["git", "rev-list", "--count", "--first-parent", "HEAD"],
        capture_output=True,
        text=True,
        env=child_env(),
    )
    if done.returncode != 0:
        raise Failed(f"failed: git exited {done.returncode}")
    return int(done.stdout.strip())


def dispatch():
    done = subprocess.run(
        ["gh", "workflow", "run", WORKFLOW, "--ref", DISPATCH_REF],
        capture_output=True,
        text=True,
        env=child_env(),
    )
    if done.returncode != 0:
        raise Failed(f"failed: gh exited {done.returncode}")


def decide(builds, now, lines):
    """Append the verdict and the dispatch outcome to `lines`; raise Failed for a refused one."""
    live = next(
        (
            build
            for build in builds
            if build["state"] == "VALID"
            and not build["expired"]
            and build["audience"] == "INTERNAL_ONLY"
        ),
        None,
    )
    if live is not None:
        lines.append(
            f"newest live build {live['number']}: uploaded {live['uploaded_text']}, "
            f"expires {live['expires_text']}"
        )
        if live["expires"] > now + timedelta(days=LEAD_DAYS):
            lines.append(HEALTHY_LINE.format(number=live["number"], expires=live["expires_text"]))
            return
        newer = [
            b for b in builds if b["uploaded"] > live["uploaded"] and b["state"] == "PROCESSING"
        ]
        prefix = DUE_LINE.format(number=live["number"], expires=live["expires_text"])
        number = live["number"]
    else:
        newer = [b for b in builds if b["state"] == "PROCESSING"]
        prefix = NO_LIVE_LINE
        number = builds[0]["number"] if builds else None
    if newer:
        lines.append(PROCESSING_LINE.format(number=newer[0]["number"]))
        return
    if number is not None and not first_parent_count() > number:
        raise Failed(UNMOVED_LINE.format(number=number))
    dispatch()
    lines.append(prefix + DISPATCHED)


def summarize(lines):
    target = os.environ.get("GITHUB_STEP_SUMMARY")
    if target:
        with open(target, "a", encoding="utf-8") as sink:
            sink.write("\n".join(lines) + "\n")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("preflight", "check"))
    parser.add_argument("--now", help="the clock, for the tests only")
    args = parser.parse_args(argv)
    lines = []
    code = 0
    try:
        missing = missing_parts(args.command == "preflight")
        if missing:
            lines += sentences(missing)
            code = ABSENT_EXIT
        elif args.command == "check":
            now = instant(args.now) if args.now else datetime.now(timezone.utc)
            with tempfile.TemporaryDirectory() as workdir:
                builds = parse(fetch(now, workdir))
            decide(builds, now, lines)
        else:
            lines.append("the credential is placed")
    except Failed as refused:
        lines.append(str(refused))
        code = FAILED_EXIT
    for line in lines:
        print(line)
    summarize(lines)
    return code


if __name__ == "__main__":
    sys.exit(main())

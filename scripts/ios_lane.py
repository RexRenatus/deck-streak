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
version) to `GITHUB_OUTPUT`. A refusal is one line on standard error and exit 1, and writes no
output.
"""

import argparse
import os
import re
import subprocess
import sys

VERBS = ("plan", "preflight", "sign", "upload-to-testflight", "clean", "build-unsigned", "summary")
# A release tag, exactly as release.yml's tag guard spells it.
SEMVER_TAG = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
# The form `CFBundleShortVersionString` takes: three dot-separated integers.
MARKETING_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
SHALLOW = "the checkout is shallow; the build number needs the full history"


class Refused(Exception):
    """A step's refusal, named by its one-line message."""


def git(*args):
    """Run git in the checkout and return the completed process."""
    return subprocess.run(["git", *args], capture_output=True, text=True, check=False)


def plan(lane):
    """Decide what the lane builds, or refuse; return the outputs the later jobs read."""
    import tomllib

    event = os.environ.get("GITHUB_EVENT_NAME", "")
    ref = os.environ.get("GITHUB_REF", "")
    if lane == "internal":
        if event != "workflow_dispatch":
            raise Refused(f"the internal lane runs on workflow_dispatch only, not on {event}")
        if ref != "refs/heads/dev":
            raise Refused(f"the internal lane builds refs/heads/dev only, not {ref}")
    elif event != "push" or os.environ.get("GITHUB_REF_TYPE") != "tag":
        raise Refused(f"the release lane runs on a tag push only, not on {event} of {ref}")
    if git("rev-parse", "--is-shallow-repository").stdout.strip() != "false":
        raise Refused(SHALLOW)
    with open("Cargo.toml", "rb") as manifest:
        version = tomllib.load(manifest)["workspace"]["package"]["version"]
    if not MARKETING_VERSION.fullmatch(version):
        raise Refused(f"the workspace version {version} is not three dot-separated integers")
    commit = "HEAD"
    if lane == "release":
        tag = os.environ.get("GITHUB_REF_NAME", "")
        commit = os.environ.get("GITHUB_SHA", "")
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


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("verb", choices=VERBS)
    parser.add_argument("--lane", choices=("internal", "release"))
    args = parser.parse_args(argv)
    try:
        if args.verb == "plan":
            outputs = plan(args.lane)
            write_outputs(outputs)
            print(f"the {args.lane} lane builds {outputs['version']} ({outputs['number']})")
    except Refused as refused:
        print(refused, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

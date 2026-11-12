#!/usr/bin/env python3
"""guards-check: the agent's guards and settings, as installed, against their manifest (SPEC-061
R8; SPEC-063 R6).

    python3 deploy/scripts/guards-check.py MANIFEST

The rail installs the guards and the agent's concrete settings at a pinned version, each file owned
by root and writable by root alone, with a manifest of each file's path, SHA-256 and mode:

    {"schema": "deckstreak.guards-manifest.v1",
     "files": [{"path": "/absolute/path", "sha256": "<64 hex>", "mode": "0644"}, ...]}

The agent's launch runs this check first, and a refusal keeps it from starting (SPEC-063 R6). It
refuses, one `REFUSE: <path>: <reason>` line each, and never prints a file's content:

* a manifest that is missing, unreadable or not the schema above, that anyone but root could
  rewrite, or that names no file;
* a file that is missing or not a regular file (a symbolic link included), that anyone but root
  owns or could write, that sits in a directory anyone but root could write, whose mode is not the
  manifest's, or whose SHA-256 is not the manifest's.

A manifest is judged whole even after its own refusal, so one run names every problem. It prints
how many files it examined; exit 0 when every file passes, 1 on any refusal.
"""

import argparse
import hashlib
import json
import os
import re
import stat
import sys
from pathlib import Path

SCHEMA = "deckstreak.guards-manifest.v1"
# Root's uid: the owner every guard, its directory and the manifest must have.
ROOT_UID = 0
SHA256 = re.compile(r"^[0-9a-f]{64}$")
MODE = re.compile(r"^0?[0-7]{3,4}$")


def write_refusals(st):
    """Why anyone but root could write a file with this status: its owner, or its mode."""
    reasons = []
    if st.st_uid != ROOT_UID:
        reasons.append(f"owned by uid {st.st_uid}, not root")
    if st.st_mode & stat.S_IWGRP:
        reasons.append("writable by its group")
    if st.st_mode & stat.S_IWOTH:
        reasons.append("writable by others")
    return reasons


def regular_file(path):
    """The file's status, or the reason it is not a regular file of its own."""
    try:
        st = os.lstat(path)
    except FileNotFoundError:
        return None, "missing"
    except OSError as error:
        return None, f"cannot be read ({error.strerror})"
    if not stat.S_ISREG(st.st_mode):
        return None, "not a regular file"
    return st, None


def directory_refusals(path):
    """Why anyone but root could replace a file: the directory that holds it."""
    parent = Path(path).parent
    try:
        st = os.lstat(parent)
    except OSError as error:
        return [f"its directory cannot be read ({error.strerror})"]
    return [f"its directory {parent} is {reason}" for reason in write_refusals(st)]


def digest(path):
    sha = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 16), b""):
            sha.update(block)
    return sha.hexdigest()


def judge_file(entry):
    """Every refusal for one of the manifest's entries."""
    path = entry["path"]
    st, missing = regular_file(path)
    if missing:
        return [missing]
    reasons = write_refusals(st) + directory_refusals(path)
    actual, recorded = stat.S_IMODE(st.st_mode), entry["mode"]
    if actual != int(recorded, 8):
        reasons.append(f"mode {actual:04o}, the manifest records {recorded}")
    if digest(path) != entry["sha256"]:
        reasons.append("its SHA-256 is not the manifest's")
    return reasons


def shaped(entry):
    return (
        isinstance(entry, dict)
        and set(entry) == {"path", "sha256", "mode"}
        and isinstance(entry["path"], str)
        and os.path.isabs(entry["path"])
        and SHA256.match(str(entry["sha256"])) is not None
        and MODE.match(str(entry["mode"])) is not None
    )


def read_manifest(manifest):
    """The manifest's entries, and the refusals of the manifest itself."""
    st, missing = regular_file(manifest)
    if missing:
        return [], [missing]
    refusals = write_refusals(st) + directory_refusals(manifest)
    try:
        data = json.loads(Path(manifest).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return [], [*refusals, f"not a guards manifest ({error})"]
    files = data.get("files") if isinstance(data, dict) else None
    if not isinstance(data, dict) or data.get("schema") != SCHEMA or not isinstance(files, list):
        return [], [*refusals, f"not a {SCHEMA} document"]
    if not files:
        return [], [*refusals, "names no file"]
    entries, seen = [], set()
    for entry in files:
        if not shaped(entry):
            refusals.append("an entry is not an absolute path, a SHA-256 and a mode")
        elif entry["path"] in seen:
            refusals.append(f"names {entry['path']} twice")
        else:
            seen.add(entry["path"])
            entries.append(entry)
    return entries, refusals


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("manifest", help="the guards' manifest, as the rail installs it")
    args = parser.parse_args(argv)
    entries, own = read_manifest(args.manifest)
    refusals = [f"REFUSE: {args.manifest}: {reason}" for reason in own]
    for entry in entries:
        refusals += [f"REFUSE: {entry['path']}: {reason}" for reason in judge_file(entry)]
    for line in refusals:
        print(line)
    print(f"guards-check: examined {len(entries)} file(s), {len(refusals)} refusal(s)")
    return 1 if refusals else 0


if __name__ == "__main__":
    sys.exit(main())

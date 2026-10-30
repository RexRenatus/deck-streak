#!/usr/bin/env python3
"""lock-digest-line: no block-buffer below 0.12.1 and no digest below 0.11.0 in a Cargo.lock
(SPEC-565, ADR-583).

    python3 scripts/lock-digest-line.py LOCK

WHY. GHSA-qwgh-2vcv-g2f7 affects block-buffer below 0.12.1. block-buffer 0.10 comes only through
digest 0.10, so one rule over a lock's name and version pairs holds both lines: every
block-buffer is 0.12.1 or later and every digest 0.11.0 or later. It reads the whole lock, so a
path through any member of the workspace counts, whatever a single package's resolve would show.
Both lock checks call it: the test over this workspace's own lock and the release audit over the
engine fork's lock at the pinned commit (`scripts/audit-sync-server.sh`).

Verdicts, one line each, then the exit code:
- RED, exit 1: a block-buffer below 0.12.1 or a digest below 0.11.0 is present. It prints
  "block-buffer below 0.12.1: K of N" and "digest below 0.11.0: K of M", each version below its
  floor with the packages that depend on it directly.
- GREEN, exit 0: every block-buffer is 0.12.1 or later and every digest 0.11.0 or later.
- VOID, exit 2: the lock holds no block-buffer, so nothing was examined, or the file is missing,
  does not parse, or holds a version that is not semver. A VOID is never green.

Versions compare by semver, so a pre-release sorts below its release: 0.12.1-rc.0 is RED. A
dependency name that does not resolve to one package only loses its label; the verdict reads the
name and version pairs alone.
"""

import re
import sys
import tomllib

#: The floors: the first block-buffer release with the fix, and the digest line that takes it.
FLOORS = (("block-buffer", (0, 12, 1)), ("digest", (0, 11, 0)))
SEMVER = re.compile(r"(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?")

RED, GREEN, VOID = 1, 0, 2


class Void(Exception):
    """The lock cannot be judged."""


def floor_text(floor):
    return ".".join(str(part) for part in floor)


def below(version, floor):
    """Whether `version` sorts below `floor` under semver, a pre-release below its release."""
    match = SEMVER.fullmatch(version)
    if match is None:
        raise Void(f"version {version!r} is not semver")
    core = tuple(int(part) for part in match.groups()[:3])
    return core < floor or (core == floor and match.group(4) is not None)


def read_packages(path):
    try:
        with open(path, "rb") as handle:
            lock = tomllib.load(handle)
    except FileNotFoundError:
        raise Void(f"{path} is missing") from None
    except (OSError, tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
        raise Void(f"{path} does not parse: {error}") from None
    packages = lock.get("package", [])
    if not isinstance(packages, list) or not all(
        isinstance(package, dict)
        and isinstance(package.get("name"), str)
        and isinstance(package.get("version"), str)
        for package in packages
    ):
        raise Void(f"{path} does not parse: a [[package]] without a name and a version")
    return packages


def dependents(packages):
    """(name, version) -> the packages that list it among their dependencies."""
    versions = {}
    for package in packages:
        versions.setdefault(package["name"], []).append(package["version"])
    found = {}
    for package in packages:
        for entry in package.get("dependencies", []):
            parts = entry.split(" ")
            if len(parts) > 1:
                key = (parts[0], parts[1])
            elif len(versions.get(parts[0], [])) == 1:
                key = (parts[0], versions[parts[0]][0])
            else:
                continue
            found.setdefault(key, []).append(f"{package['name']} {package['version']}")
    return found


def judge(path):
    """The verdict's lines and its exit code."""
    try:
        packages = read_packages(path)
        users = dependents(packages)
        examined = {
            name: sorted(p["version"] for p in packages if p["name"] == name) for name, _ in FLOORS
        }
        lines = [
            f"lock-digest-line: {path}: {len(packages)} packages; examined "
            f"{len(examined['block-buffer'])} block-buffer, {len(examined['digest'])} digest"
        ]
        if not examined["block-buffer"]:
            return lines + ["lock-digest-line: VOID: no block-buffer in the lock"], VOID
        failed = False
        for name, floor in FLOORS:
            low = [version for version in examined[name] if below(version, floor)]
            failed = failed or bool(low)
            detail = "; ".join(
                f"{version} <- {', '.join(sorted(users.get((name, version), [])) or ['no dependent'])}"
                for version in low
            )
            lines.append(
                f"lock-digest-line: {name} below {floor_text(floor)}: "
                f"{len(low)} of {len(examined[name])}" + (f": {detail}" if low else "")
            )
    except Void as void:
        return [f"lock-digest-line: VOID: {void}"], VOID
    return lines + [f"lock-digest-line: {'RED' if failed else 'GREEN'}"], RED if failed else GREEN


def main(argv):
    if len(argv) != 2:
        print("usage: lock-digest-line.py LOCK", file=sys.stderr)
        return VOID
    lines, code = judge(argv[1])
    print("\n".join(lines))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv))

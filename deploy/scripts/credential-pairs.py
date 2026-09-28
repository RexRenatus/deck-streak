#!/usr/bin/env python3
"""credential-pairs: every (unit, credential id) pair DeckStreak's deploy templates declare
(SPEC-061 R5, ADR-038).

    python3 deploy/scripts/credential-pairs.py --root .
    python3 deploy/scripts/credential-pairs.py --root . --optional ai-route

It reads every unit file under `<root>/deploy/`, with the drop-ins of the `<unit>.d/` directory
beside it, as systemd reads them: comments, continued lines, and the empty assignment that resets
a list. An optional set, `deploy/optional/<name>/`, is read only when `--optional` names it: each
drop-in `<unit>.conf` applies to that unit after its template, and a name without a unit type,
`<name>.conf`, applies to `<name>.service` (SPEC-063, SPEC-065).

It prints one JSON object: `pairs`, each `{"unit", "credential"}` once, sorted, and a template
unit named as the template it is (`name@.service`), because systemd asks the socket under the
instance's name and the rail's map matches an instance to its template's row (R4); `optional`, the
sets it added; and `examined`, the files and lines it read. The rail refuses to install when its
map's pairs differ from this list in either direction.

A credential reaches a unit only from the credential socket (ADR-038), so the lister refuses, by
file and line, every other way to give one: `LoadCredentialEncrypted=`, `SetCredential=`,
`SetCredentialEncrypted=`, `ImportCredential=`, and a `LoadCredential=` whose source is not the
socket. A refused run prints each refusal to stderr and no list, and exits 1. A root with no unit
file, or an optional set that does not exist, judged nothing: exit 2.
"""

import argparse
import json
import re
import sys
from pathlib import Path

SOCKET = "/run/deck-streak-credentials/socket"
UNIT_TYPES = (".service", ".socket", ".timer", ".path", ".mount", ".swap", ".target", ".slice")
# Every other directive that gives a unit a credential, none of them from the socket.
OTHER_SOURCES = (
    "LoadCredentialEncrypted",
    "SetCredential",
    "SetCredentialEncrypted",
    "ImportCredential",
)
# A credential id is a plain file name in the unit's credentials directory (systemd.exec(5)).
CREDENTIAL_ID = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_.-]*$")
SET_NAME = re.compile(r"^[a-z0-9][a-z0-9-]*$")


def logical_lines(text):
    """systemd.syntax(7): a trailing backslash joins the next line, a comment line inside the join
    is skipped, and each logical line keeps the number it started on."""
    pending, start = [], 0
    for number, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if pending and stripped.startswith(("#", ";")):
            continue
        if not pending:
            start = number
        if stripped.endswith("\\"):
            pending.append(stripped[:-1])
            continue
        pending.append(stripped)
        yield start, " ".join(part for part in pending if part).strip()
        pending = []
    if pending:
        yield start, " ".join(pending).strip()


def assignments(text):
    """Every `Key=Value` of a unit file or drop-in, as (line, key, value). Sections are not kept: a
    credential directive means the same in every section that takes one."""
    for number, line in logical_lines(text):
        if not line or line.startswith(("#", ";", "[")):
            continue
        key, equals, value = line.partition("=")
        if equals:
            yield number, key.strip(), value.strip()


def optional_unit(file_name):
    """The unit an optional set's drop-in applies to: `<unit>.conf`, or `<name>.conf` for a
    service."""
    stem = file_name.removesuffix(".conf")
    return stem if stem.endswith(UNIT_TYPES) else f"{stem}.service"


class Lister:
    """The pairs of one tree, read file by file, with its refusals and its examined counts."""

    def __init__(self, root):
        self.root = root
        self.credentials = {}
        self.refusals = []
        self.files = 0
        self.lines = 0

    def read(self, unit, path):
        text = path.read_text(encoding="utf-8")
        self.files += 1
        self.lines += len(text.splitlines())
        where = path.relative_to(self.root).as_posix()
        ids = self.credentials.setdefault(unit, {})
        for number, key, value in assignments(text):
            if key in OTHER_SOURCES and value:
                # The value is never printed: a SetCredential= line holds a literal secret.
                self.refuse(where, number, f"{key}= is refused; a credential comes from the socket")
            elif key == "LoadCredential":
                self.load(ids, where, number, value)

    def load(self, ids, where, number, value):
        if not value:
            # An empty assignment resets the unit's list (systemd.exec(5)).
            ids.clear()
            return
        ident, colon, source = value.partition(":")
        if not CREDENTIAL_ID.match(ident):
            self.refuse(where, number, "LoadCredential= names no valid credential id")
        elif not colon:
            self.refuse(where, number, f"{ident} is read from the credential store, not the socket")
        elif source != SOCKET:
            self.refuse(where, number, f"{ident} is read from {source}, not the socket {SOCKET}")
        else:
            ids[ident] = None

    def refuse(self, where, number, reason):
        self.refusals.append(f"REFUSE: {where}:{number}: {reason} (ADR-038)")

    def pairs(self):
        return [
            {"unit": unit, "credential": ident}
            for unit in sorted(self.credentials)
            for ident in sorted(self.credentials[unit])
        ]


def unit_files(deploy):
    """Every unit file under `deploy`, outside a drop-in directory and outside the optional sets."""
    for path in sorted(deploy.rglob("*")):
        inside = path.relative_to(deploy).parts
        if inside[0] == "optional" or path.parent.name.endswith(".d"):
            continue
        if path.is_file() and path.name.endswith(UNIT_TYPES):
            yield path


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=".", help="the checkout or release to read")
    parser.add_argument(
        "--optional", action="append", default=[], metavar="NAME", help="add an optional set"
    )
    args = parser.parse_args(argv)
    root = Path(args.root).resolve()
    deploy = root / "deploy"

    units = list(unit_files(deploy)) if deploy.is_dir() else []
    if not units:
        print(f"VOID: no unit file under {deploy}; nothing was judged", file=sys.stderr)
        return 2
    sets = []
    for name in args.optional:
        folder = deploy / "optional" / name
        if not SET_NAME.match(name) or not folder.is_dir():
            print(f"VOID: no optional set named {name!r} under deploy/optional/", file=sys.stderr)
            return 2
        sets.append((name, sorted(folder.glob("*.conf"))))

    lister = Lister(root)
    for path in units:
        lister.read(path.name, path)
        for dropin in sorted((path.parent / f"{path.name}.d").glob("*.conf")):
            lister.read(path.name, dropin)
    for _, dropins in sets:
        for dropin in dropins:
            lister.read(optional_unit(dropin.name), dropin)

    if lister.refusals:
        for line in lister.refusals:
            print(line, file=sys.stderr)
        return 1
    listed = {
        "pairs": lister.pairs(),
        "optional": [name for name, _ in sets],
        "examined": {"files": lister.files, "lines": lister.lines},
    }
    print(json.dumps(listed, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

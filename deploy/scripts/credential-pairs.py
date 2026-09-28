#!/usr/bin/env python3
"""credential-pairs: every (unit, credential id) pair DeckStreak's deploy templates declare
(SPEC-061 R5, ADR-038).

    python3 deploy/scripts/credential-pairs.py --root .
    python3 deploy/scripts/credential-pairs.py --root . --optional ai-route

It reads every unit file under `<root>/deploy/`, with the drop-ins of the `<unit>.d/` directory
beside it, as systemd reads them: its line ends, comments and continued lines (SPEC-061 §8), and
the empty assignment that resets a list; a construct systemd could read otherwise, such as a
byte-order mark, is refused by file and line rather than guessed. An optional set,
`deploy/optional/<name>/`, is read only when `--optional` names it: each drop-in `<unit>.conf`
applies to that unit after its template, and a name without a unit type, `<name>.conf`, applies to
`<name>.service` (SPEC-063, SPEC-065).

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
# systemd's own reading of a unit file (conf-parser.c, and read_line() in fileio.c), measured with
# `systemd-analyze verify` (SPEC-061 §8): a line ends at a newline, a carriage return or a NUL, and
# a newline and a carriage return in either order, a NUL perhaps after them, end one line; a blank
# is a space, a tab, a newline or a carriage return, nothing else; a line whose first non-blank
# character is `#` or `;` is a comment, inside a continued line too; and a line that ends in an odd
# run of backslashes goes on in the next, its last backslash read as a space.
BLANK = " \t\n\r"
LINE_END = re.compile(r"\n\r?\0?|\r\n?\0?|\0")
# The section systemd reads in a unit of each type besides [Unit] and [Install]; any other header,
# `[X-...]` included, is one this reading refuses.
SECTIONS = {
    ".service": "Service",
    ".socket": "Socket",
    ".timer": "Timer",
    ".path": "Path",
    ".mount": "Mount",
    ".swap": "Swap",
    ".slice": "Slice",
    ".target": None,
}
# A credential id is a plain file name in the unit's credentials directory (systemd.exec(5)).
CREDENTIAL_ID = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_.-]*$")
SET_NAME = re.compile(r"^[a-z0-9][a-z0-9-]*$")


def physical_lines(text):
    """The file's lines, ended as systemd's read_line() ends them."""
    lines = LINE_END.split(text)
    if lines[-1] == "":
        lines.pop()
    return lines


def logical_lines(text):
    """(number, line) for each line systemd parses: comments skipped, continued lines joined,
    blanks stripped, each numbered by the line it starts on."""
    pending, start = None, 0
    for number, raw in enumerate(physical_lines(text), start=1):
        if raw.lstrip(BLANK)[:1] in ("#", ";"):
            continue
        if pending is None:
            pending, start = "", number
        pending += raw
        if (len(pending) - len(pending.rstrip("\\"))) % 2:
            pending = pending[:-1] + " "
            continue
        yield start, pending.strip(BLANK)
        pending = None
    if pending is not None:
        yield start, pending.strip(BLANK)


def read_unit(text, unit):
    """Every assignment of one file of `unit` as systemd reads it, as (line, section, key, value),
    and each construct systemd could read otherwise, as (line, reason): a byte-order mark, which
    systemd skips where it first finds one; a section header other than [Unit], [Install] and the
    unit type's own, whose lines systemd ignores or which fails the whole file; and an assignment
    before the first header, which systemd ignores."""
    found, unread = [], []
    for number, raw in enumerate(physical_lines(text), start=1):
        if "\ufeff" in raw:
            unread.append((number, "a byte-order mark, which systemd skips"))
    known = {"Unit", "Install", SECTIONS.get(unit[unit.rfind(".") :])} - {None}
    section, headed = None, False
    for number, line in logical_lines(text):
        if line.startswith("["):
            # systemd reads the text between the brackets whole: `[ Service ]` is no [Service].
            headed = True
            section = line[1:-1] if line.endswith("]") and line[1:-1] in known else None
            if section is None:
                names = ", ".join(f"[{name}]" for name in sorted(known))
                unread.append((number, f"a section header other than {names}"))
            continue
        key, equals, value = line.partition("=")
        if not equals:
            continue
        if section is not None:
            found.append((number, section, key.strip(BLANK), value.strip(BLANK)))
        elif not headed:
            unread.append((number, "an assignment before the first section header"))
    return found, unread


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
        where = path.relative_to(self.root).as_posix()
        self.files += 1
        try:
            # Bytes, so that no newline is translated before systemd's own line ends are read.
            text = path.read_bytes().decode("utf-8")
        except UnicodeDecodeError:
            self.refuse(where, 1, "not UTF-8, which systemd reads line by line")
            return
        self.lines += len(physical_lines(text))
        ids = self.credentials.setdefault(unit, {})
        found, unread = read_unit(text, unit)
        for number, reason in unread:
            self.refuse(where, number, f"{reason}; the lister refuses what systemd reads otherwise")
        for number, _, key, value in found:
            if key in OTHER_SOURCES and value:
                # The value is never printed: a SetCredential= line holds a literal secret.
                reason = f"{key}= is refused; a credential comes from the socket (ADR-038)"
                self.refuse(where, number, reason)
            elif key == "LoadCredential":
                self.load(ids, where, number, value)

    def load(self, ids, where, number, value):
        if not value:
            # An empty assignment resets the unit's list (systemd.exec(5)).
            ids.clear()
            return
        ident, colon, source = value.partition(":")
        if not CREDENTIAL_ID.match(ident):
            self.refuse(where, number, "LoadCredential= names no valid credential id (ADR-038)")
        elif not colon:
            reason = f"{ident} is read from the credential store, not the socket (ADR-038)"
            self.refuse(where, number, reason)
        elif source != SOCKET:
            reason = f"{ident} is read from {source}, not the socket {SOCKET} (ADR-038)"
            self.refuse(where, number, reason)
        else:
            ids[ident] = None

    def refuse(self, where, number, reason):
        self.refusals.append(f"REFUSE: {where}:{number}: {reason}")

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

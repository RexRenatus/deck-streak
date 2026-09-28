#!/usr/bin/env python3
"""effective-check: a unit's effective configuration, as `systemctl cat` prints it, judged against
the rail contract (SPEC-061 R7; ADR-061, ADR-038).

    systemctl cat deck-streak-api.service | python3 deploy/scripts/effective-check.py --root R
    python3 deploy/scripts/effective-check.py --root R SAVED_OUTPUT [SAVED_OUTPUT ...]
    python3 deploy/scripts/effective-check.py --root . --census

`--root` is the release, or the checkout, whose `deploy/rail-contract.json` is read; it defaults to
the tree this script belongs to. `systemctl cat` prints every file systemd merged for a unit, the
unit's own file first, each under a `# <path>` line, and one output may hold several units.

The rail installs every template byte for byte and writes one drop-in per unit,
`<unit>.d/10-rail.conf`, which resets and sets each neutral value the contract lists (ADR-061).
Each file is read as systemd reads it (SPEC-061 §8): its line ends, comments, continued lines,
sections and quotes. A construct systemd could read otherwise is refused, never guessed: a
byte-order mark, a section header other than [Unit], [Install] and the unit type's own, an
assignment before the first header, a file header not after an empty line, and a value written
with a backslash escape, a `..` segment, or a specifier other than the unit's own names. Per unit,
this check refuses:

* a neutral value left in force: a value the contract lists for the unit, or any value that still
  carries a neutral one as systemd resolves it (the example release root, the example settings
  file, a calendar that fires at the neutral zone's instants), which no later empty assignment
  reset; and a calendar that names no zone this check can read;
* a credential from anywhere but the socket, in any of the unit's files: `LoadCredentialEncrypted=`,
  `SetCredential=`, `SetCredentialEncrypted=`, `ImportCredential=`, or a `LoadCredential=` whose
  source is not the socket (ADR-038);
* an `Environment=` assignment whose variable's name says it carries a secret, or holds a
  specifier;
* every other route a value takes into the unit's process: a `PassEnvironment=` whose name says it
  carries a secret, `StandardInputText=`, `StandardInputData=`, `StandardInput=file:`, and a second
  `EnvironmentFile=` in force (R6 gives a unit one);
* a drop-in other than the rail's own, `<unit>.d/10-rail.conf` beside the unit's file.

A refused line is named by its key and its variable, never its value.
Exit 0 when every unit passes, 1 when one is refused, and 2 when the input held no unit or the
contract cannot be read: a check that judged nothing did not pass.

`--census` judges the contract itself (A3). Every value under `--root`'s `deploy/` that carries a
neutral value, in a template, a drop-in beside it or an optional set, must be named by the
contract, and every value the contract names must be carried by one, so the list drifts in neither
direction.
"""

import argparse
import json
import re
import shlex
import sys
from datetime import UTC, datetime
from pathlib import Path, PurePosixPath
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

SCHEMA = "deckstreak.rail-contract.v1"
CONTRACT = Path("deploy") / "rail-contract.json"
UNIT_TYPES = (".service", ".socket", ".timer", ".path", ".mount", ".swap", ".target", ".slice")
NEUTRAL_KEYS = {
    "release_root": str,
    "environment_file": str,
    "time_zone": str,
    "rollover_hour": int,
}
# Every directive that gives a unit a credential some other way than the socket.
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
# A `systemctl cat` file header: `# ` and the absolute path of the file that follows it, read to the
# end of the line. A drop-in's file name may hold a blank, which systemd loads and systemctl prints
# as it is (measured on systemd 255), so a path is never cut at one.
HEADER = re.compile(r"^# (/.+)$")
# A variable whose name says it carries a secret: SPEC-032's pattern (scripts/tests/_units.py),
# with its `*_KEY` names widened to any KEY segment and PASS added, so that a device key or a short
# password name is caught as well.
SECRET_NAME = re.compile(
    r"(?:^|_)(?:TOKEN|SECRET|PASSWORD|PASSWD|PASS|APIKEY|KEY|CREDENTIALS?|DSN)(?:$|_)",
    re.IGNORECASE,
)
# Keys whose value can hold a secret: a refusal names the key alone.
SECRET_BEARING = frozenset(
    {
        "Environment",
        "SetCredential",
        "SetCredentialEncrypted",
        "StandardInputText",
        "StandardInputData",
    }
)
# Keys that give the unit's process its standard input from the unit's own file.
INPUT_KEYS = frozenset({"StandardInputText", "StandardInputData"})
# A specifier systemd expands in a value, `%` and a letter; `%%` is a literal percent. Only the
# unit's own names are left to systemd, `%i` (the instance), `%n` and `%N` (the unit's name), none
# of which holds a slash; every other expands to a path, a user or a host value this check cannot
# see, and could spell a neutral value (`%E` is /etc to the system manager).
SPECIFIER = re.compile(r"%(?:%|([A-Za-z]))")
UNIT_NAMES = frozenset("inN")
# A `..` segment, which systemd keeps in a command's path and the kernel then follows.
UP_LEVEL = re.compile(r"(?:^|[/\s])\.\.(?:[/\s]|$)")
NAME = re.compile(r"^[A-Za-z][A-Za-z0-9-]*$")
INSTANCE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
DROP_IN = re.compile(r"^[0-9]{2}-[a-z0-9-]+\.conf$")


class Unjudgeable(Exception):
    """The contract or the input cannot be read, so nothing was judged."""


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


def shown(key, value):
    return f"{key}=" if key in SECRET_BEARING else f"{key}={value}"


def path_view(value):
    """The value as systemd resolves the paths in it, to find a neutral one: quotes removed, and
    repeated slashes and `.` segments collapsed (measured with `systemd-analyze verify`)."""
    view = re.sub(r"/+", "/", value.replace('"', "").replace("'", ""))
    while "/./" in view:
        view = view.replace("/./", "/")
    return re.sub(r"/\.(?=\s|$)", "", view)


def zone_offsets(zone):
    """The zone's offsets from UTC across a year, as the tz database has them, or None when this
    check cannot read the zone."""
    try:
        info = ZoneInfo(zone)
    except (ValueError, ZoneInfoNotFoundError, OSError):
        return None
    moments = (datetime(2025, month, 15, 12, tzinfo=UTC) for month in range(1, 13))
    return {moment.astimezone(info).utcoffset() for moment in moments}


def calendar_zone(value, neutral_zone):
    """How systemd reads a calendar's zone, its last word (`systemd-analyze calendar`): "neutral"
    when it fires at the neutral zone's instants all year (systemd reads ` UTC` in any case, and a
    zone that keeps a zero offset fires at UTC's instants), "none" when it names no zone this check
    can read, so that it fires in the host's own zone or one this check cannot compare, and None
    for a zone of its own."""
    if value.lower().endswith(" " + neutral_zone.lower()):
        return "neutral"
    offsets = zone_offsets(value.rpartition(" ")[2]) if " " in value else None
    if offsets is None:
        return "none"
    return "neutral" if offsets == zone_offsets(neutral_zone) else None


def carries_neutral(key, value, neutral):
    """Whether a value still holds one of the contract's neutral values, as systemd resolves it."""
    view = path_view(value)
    if neutral["release_root"] in view or neutral["environment_file"] in view:
        return True
    return key == "OnCalendar" and calendar_zone(value, neutral["time_zone"]) == "neutral"


def load_contract(root):
    """The rail contract under `root`, shape-checked, with its rows keyed by unit."""
    path = root / CONTRACT
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Unjudgeable(f"the rail contract cannot be read: {path}: {error}") from None
    if not isinstance(data, dict) or data.get("schema") != SCHEMA:
        raise Unjudgeable(f"{path} is not a {SCHEMA} document")
    unknown = sorted(set(data) - {"schema", "note", "neutral", "drop_in", "socket", "values"})
    neutral = data.get("neutral")
    shaped = isinstance(neutral, dict) and set(neutral) == set(NEUTRAL_KEYS)
    if (
        unknown
        or not shaped
        or not all(
            isinstance(neutral[key], kind) and neutral[key] != ""
            for key, kind in NEUTRAL_KEYS.items()
        )
    ):
        raise Unjudgeable(f"{path}: unknown keys {unknown}, or `neutral` is not the four values")
    if not DROP_IN.match(str(data.get("drop_in"))) or not str(data.get("socket")).startswith("/"):
        raise Unjudgeable(f"{path}: `drop_in` is not a drop-in's name or `socket` not a path")
    rows = {}
    for row in data.get("values") or []:
        if not (
            isinstance(row, dict)
            and set(row) - {"instance"} == {"unit", "section", "key", "neutral"}
            and str(row["unit"]).endswith(UNIT_TYPES)
            and NAME.match(str(row["section"]))
            and NAME.match(str(row["key"]))
            and isinstance(row["neutral"], list)
            and row["neutral"]
            and all(isinstance(value, str) and value for value in row["neutral"])
        ):
            raise Unjudgeable(f"{path}: a row is not a unit, a section, a key and its values")
        unit = unit_of(row)
        if unit is None:
            raise Unjudgeable(f"{path}: {row['unit']} is not a template its instance can join")
        listed = rows.setdefault(unit, [])
        if any(row["section"] == s and row["key"] == k for s, k, _ in listed):
            raise Unjudgeable(f"{path}: {unit} lists [{row['section']}] {row['key']} twice")
        listed.append((row["section"], row["key"], row["neutral"]))
    if not rows:
        raise Unjudgeable(f"{path} lists no neutral value")
    return {**data, "rows": rows}


def unit_of(row):
    """The unit a row names: its `unit`, or, with an `instance`, that instance of the template
    `unit` names. A committed file never writes an instance's name whole (SPEC-032 R10), so the
    contract names a job's timer by its template and its instance, and this joins them."""
    if "instance" not in row:
        return row["unit"]
    template, instance = str(row["unit"]), str(row["instance"])
    if template.count("@.") != 1 or not INSTANCE.match(instance):
        return None
    return template.replace("@.", f"@{instance}.")


def files_of(text):
    """Each file of a `systemctl cat` output, in order, as (path, text, glued). systemctl prints
    every line of a file newline-terminated, a last line without one included, and a file's
    `# <path>` line at the start of the output or after an empty line (measured on systemd 255).
    A header-shaped line elsewhere can then only be the file's own comment: it is kept in its file
    and named in `glued`, which the check refuses rather than read a comment as a file. The output
    is split at newlines alone, since the files' own line ends are systemd's to read."""
    files, blank = [], True
    for number, raw in enumerate(text.split("\n"), start=1):
        match = HEADER.match(raw)
        if match and blank:
            files.append((match.group(1), [], []))
        elif files:
            files[-1][1].append(raw)
            if match:
                files[-1][2].append(match.group(1))
        elif raw:
            raise Unjudgeable(f"line {number} comes before the first file's `# <path>` line")
        blank = raw == ""
    return [(path, "\n".join(lines), glued) for path, lines, glued in files]


def units_of(files):
    """The files grouped by unit, as (unit file, drop-ins): a file whose directory ends in `.d` is
    a drop-in of the unit whose file came before it."""
    units = []
    for file in files:
        if PurePosixPath(file[0]).parent.name.endswith(".d"):
            if not units:
                raise Unjudgeable(f"{file[0]} is a drop-in shown before any unit's file")
            units[-1][1].append(file)
        else:
            units.append((file, []))
    return units


def credential_refusal(key, value, socket):
    if key in OTHER_SOURCES:
        return f"{key}= is refused; a credential comes from the socket (ADR-038)"
    if key == "LoadCredential":
        ident, colon, source = value.partition(":")
        if not colon or source != socket:
            where = source if colon else "the credential store"
            return f"LoadCredential= reads {ident} from {where}, not the socket (ADR-038)"
    return None


def spelling_refusal(key, value):
    """A value written in a form this check cannot read as systemd resolves it (SPEC-061 §8): a
    backslash escape, which systemd decodes by key, in quotes too; a `..` segment, which the kernel
    follows; and a specifier other than the unit's own names."""
    if "\\" in value:
        return f"{key}= is written with a backslash escape, which systemd decodes by key"
    if UP_LEVEL.search(value):
        return f"{key}= holds a `..` segment, which the kernel follows"
    for match in SPECIFIER.finditer(value):
        if match.group(1) and match.group(1) not in UNIT_NAMES:
            return f"{key}= names the specifier %{match.group(1)}, which this check cannot expand"
    return None


def name_refusals(key, value):
    """systemd splits the line into words, removes their quotes and expands specifiers before it
    reads each variable's name (SPEC-061 §8). A line with a backslash is refused whole by its
    spelling, so quotes are all that is left, and shlex removes them as systemd does. An
    `Environment=` word is NAME=VALUE; a `PassEnvironment=` word a name alone, whose value comes
    from the service manager's own environment."""
    if "\\" in value:
        return []
    try:
        words = shlex.split(value)
    except ValueError:
        return [f"{key}= holds a line that cannot be read"]
    refusals = []
    for word in words:
        name, equals, _ = word.partition("=")
        if key == "Environment" and not equals:
            continue
        if "%" in name:
            refusals.append(f"{key}= names a variable with a specifier, which systemd expands")
        elif SECRET_NAME.search(name):
            verb = "sets" if key == "Environment" else "passes"
            refusals.append(f"{key}= {verb} {name}, whose name says it carries a secret")
    return refusals


def route_refusal(key, value):
    """A route by which a value reaches the unit's process without a credential: its standard
    input, written in the unit's file or read from a file."""
    if key in INPUT_KEYS:
        return f"{key}= gives the unit's standard input a value written in its file"
    if key == "StandardInput" and value.startswith("file:"):
        return "StandardInput= reads a file into the unit's standard input"
    return None


def judge(unit_file, dropins, contract):
    """Every refusal for one unit, each once, in the order found."""
    path = unit_file[0]
    name = PurePosixPath(path).name
    own = str(PurePosixPath(path).parent / f"{name}.d" / contract["drop_in"])
    refusals = []

    def refuse(reason):
        line = f"REFUSE: {name}: {reason}"
        if line not in refusals:
            refusals.append(line)

    in_force = {}
    for source, text, glued in [unit_file, *dropins]:
        if source not in (path, own):
            refuse(f"a drop-in that is not the rail's: {source}")
        for header in glued:
            refuse(f"a file header not after an empty line: {header}")
        found, unread = read_unit(text, name)
        for number, reason in unread:
            refuse(f"{source}:{number}: {reason}; the check refuses what systemd reads otherwise")
        for _, section, key, value in found:
            if not value:
                # An empty assignment resets the key's list (systemd.unit(5), drop-ins).
                in_force[(section, key)] = []
                continue
            in_force.setdefault((section, key), []).append(value)
            for reason in (
                spelling_refusal(key, value),
                credential_refusal(key, value, contract["socket"]),
                route_refusal(key, value),
            ):
                if reason:
                    refuse(reason)
            if key in ("Environment", "PassEnvironment"):
                for reason in name_refusals(key, value):
                    refuse(reason)
    for section, key, values in contract["rows"].get(name, []):
        for value in values:
            if value in in_force.get((section, key), []):
                refuse(f"neutral value left in force: [{section}] {shown(key, value)}")
    zone = contract["neutral"]["time_zone"]
    for (section, key), values in in_force.items():
        if key == "EnvironmentFile" and len(values) > 1:
            count = len(values)
            refuse(f"[{section}] holds {count} EnvironmentFile= in force; R6 gives a unit one")
        for value in values:
            if carries_neutral(key, value, contract["neutral"]):
                refuse(f"neutral value left in force: [{section}] {shown(key, value)}")
            elif key == "OnCalendar" and calendar_zone(value, zone) == "none":
                refuse(
                    f"a calendar that names no zone this check can read: [{section}] {key}={value}"
                )
    return refusals


def unit_files(deploy):
    """Every unit file under `deploy`, outside a drop-in directory and outside the optional sets."""
    for path in sorted(deploy.rglob("*")):
        inside = path.relative_to(deploy).parts
        if inside[0] == "optional" or path.parent.name.endswith(".d"):
            continue
        if path.is_file() and path.name.endswith(UNIT_TYPES):
            yield path


def optional_unit(file_name):
    """The unit an optional set's drop-in applies to, as `credential-pairs.py` reads it."""
    stem = file_name.removesuffix(".conf")
    return stem if stem.endswith(UNIT_TYPES) else f"{stem}.service"


def census(root, contract):
    """The contract against the neutral values the files under `root`'s deploy/ carry."""
    deploy = root / "deploy"
    units = list(unit_files(deploy)) if deploy.is_dir() else []
    if not units:
        print(f"VOID: no unit file under {deploy}; nothing was judged")
        return 2
    sources = []
    for path in units:
        sources.append((path.name, path))
        sources += [
            (path.name, drop) for drop in sorted((path.parent / f"{path.name}.d").glob("*.conf"))
        ]
    sources += [(optional_unit(p.name), p) for p in sorted(deploy.glob("optional/*/*.conf"))]
    carried, unreadable, files, lines = {}, [], 0, 0
    for unit, path in sources:
        text = path.read_bytes().decode("utf-8")
        files, lines = files + 1, lines + len(physical_lines(text))
        found, unread = read_unit(text, unit)
        where = path.relative_to(root).as_posix()
        unreadable += [f"REFUSE: {unit}: {where}:{number}: {reason}" for number, reason in unread]
        for _, section, key, value in found:
            if value and carries_neutral(key, value, contract["neutral"]):
                carried.setdefault((unit, section, key, value), None)
    named = {
        (unit, section, key, value)
        for unit, listed in contract["rows"].items()
        for section, key, values in listed
        for value in values
    }
    refusals = unreadable + [
        f"REFUSE: {unit}: the contract does not name [{section}] {shown(key, value)}"
        for unit, section, key, value in carried
        if (unit, section, key, value) not in named
    ]
    refusals += [
        f"REFUSE: {unit}: no template carries [{section}] {shown(key, value)}, which the "
        "contract names"
        for unit, section, key, value in sorted(named - set(carried))
    ]
    for line in refusals:
        print(line)
    counted = f"census: examined {files} file(s), {lines} line(s)"
    if refusals:
        print(f"{counted}; {len(refusals)} refusal(s)")
        return 1
    print(f"{counted}; {len(carried)} neutral value(s), each named")
    return 0


def read_outputs(names):
    texts = []
    for name in names or ["-"]:
        try:
            data = sys.stdin.buffer.read() if name == "-" else Path(name).read_bytes()
            texts.append(data.decode("utf-8"))
        except OSError as error:
            raise Unjudgeable(f"{name}: {error.strerror}") from None
        except UnicodeDecodeError:
            raise Unjudgeable(f"{name}: not UTF-8") from None
    return texts


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--root",
        default=str(Path(__file__).resolve().parents[2]),
        help="the release or checkout whose deploy/rail-contract.json is read",
    )
    parser.add_argument("--census", action="store_true", help="judge the contract, not a unit")
    parser.add_argument("outputs", nargs="*", metavar="OUTPUT", help="a `systemctl cat` output")
    args = parser.parse_args(argv)
    root = Path(args.root).resolve()
    try:
        contract = load_contract(root)
        if args.census:
            if args.outputs:
                parser.error("--census reads the templates under --root, not an output")
            return census(root, contract)
        units = [unit for text in read_outputs(args.outputs) for unit in units_of(files_of(text))]
    except Unjudgeable as error:
        print(f"VOID: {error}")
        return 2
    if not units:
        print("VOID: the input held no unit; nothing was judged")
        return 2
    refusals = [
        line for unit_file, dropins in units for line in judge(unit_file, dropins, contract)
    ]
    for line in refusals:
        print(line)
    files = sum(1 + len(dropins) for _, dropins in units)
    counted = f"effective-check: examined {len(units)} unit(s), {files} file(s)"
    print(f"{counted}; {len(refusals)} refusal(s)")
    return 1 if refusals else 0


if __name__ == "__main__":
    sys.exit(main())

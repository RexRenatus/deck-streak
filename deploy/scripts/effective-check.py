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
`<unit>.d/10-rail.conf`, which resets and sets each neutral value the contract lists (ADR-061). Per
unit, this check refuses:

* a neutral value left in force: a value the contract lists for the unit, or any value that still
  carries a neutral one (the example release root, the example settings file, a calendar in the
  neutral zone), which no later empty assignment reset;
* a credential from anywhere but the socket, in any of the unit's files: `LoadCredentialEncrypted=`,
  `SetCredential=`, `SetCredentialEncrypted=`, `ImportCredential=`, or a `LoadCredential=` whose
  source is not the socket (ADR-038);
* an `Environment=` assignment whose variable's name says it carries a secret;
* a drop-in other than the rail's own, `<unit>.d/10-rail.conf` beside the unit's file.

A refused `Environment=` or credential line is named by its key and its variable, never its value.
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
from pathlib import Path, PurePosixPath

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
# A `systemctl cat` file header: `# ` and the absolute path of the file that follows it.
HEADER = re.compile(r"^# (/\S+)$")
# A variable whose name says it carries a secret: SPEC-032's pattern (scripts/tests/_units.py),
# with its `*_KEY` names widened to any KEY segment and PASS added, so that a device key or a short
# password name is caught as well.
SECRET_NAME = re.compile(
    r"(?:^|_)(?:TOKEN|SECRET|PASSWORD|PASSWD|PASS|APIKEY|KEY|CREDENTIALS?|DSN)(?:$|_)",
    re.IGNORECASE,
)
# Keys whose value can hold a secret: a refusal names the key alone.
SECRET_BEARING = frozenset({"Environment", "SetCredential", "SetCredentialEncrypted"})
NAME = re.compile(r"^[A-Za-z][A-Za-z0-9-]*$")
INSTANCE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
DROP_IN = re.compile(r"^[0-9]{2}-[a-z0-9-]+\.conf$")


class Unjudgeable(Exception):
    """The contract or the input cannot be read, so nothing was judged."""


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
    """Every `Key=Value` of one file, as (section, key, value), in order."""
    section = None
    for _, line in logical_lines(text):
        if not line or line.startswith(("#", ";")):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            continue
        key, equals, value = line.partition("=")
        if equals:
            yield section, key.strip(), value.strip()


def shown(key, value):
    return f"{key}=" if key in SECRET_BEARING else f"{key}={value}"


def carries_neutral(key, value, neutral):
    """Whether a value still holds one of the contract's neutral values."""
    if neutral["release_root"] in value or neutral["environment_file"] in value:
        return True
    return key == "OnCalendar" and value.split()[-1:] == [neutral["time_zone"]]


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
    """Each file of a `systemctl cat` output, in order, as (path, text). A header opens a file at
    the start of the output or after a blank line, where systemctl prints one."""
    files, blank = [], True
    for number, raw in enumerate(text.splitlines(), start=1):
        match = HEADER.match(raw)
        if match and blank:
            files.append((match.group(1), []))
        elif files:
            files[-1][1].append(raw)
        elif raw.strip():
            raise Unjudgeable(f"line {number} comes before the first file's `# <path>` line")
        blank = not raw.strip()
    return [(path, "\n".join(lines)) for path, lines in files]


def units_of(files):
    """The files grouped by unit, as (unit file, drop-ins): a file whose directory ends in `.d` is
    a drop-in of the unit whose file came before it."""
    units = []
    for path, text in files:
        if PurePosixPath(path).parent.name.endswith(".d"):
            if not units:
                raise Unjudgeable(f"{path} is a drop-in shown before any unit's file")
            units[-1][1].append((path, text))
        else:
            units.append(((path, text), []))
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


def environment_refusals(value):
    try:
        words = shlex.split(value)
    except ValueError:
        return ["an Environment= line that cannot be read"]
    refusals = []
    for word in words:
        name, equals, _ = word.partition("=")
        if equals and SECRET_NAME.search(name):
            refusals.append(f"Environment= sets {name}, whose name says it carries a secret")
    return refusals


def judge(unit_file, dropins, contract):
    """Every refusal for one unit, each once, in the order found."""
    path, _ = unit_file
    name = PurePosixPath(path).name
    own = str(PurePosixPath(path).parent / f"{name}.d" / contract["drop_in"])
    refusals = []

    def refuse(reason):
        line = f"REFUSE: {name}: {reason}"
        if line not in refusals:
            refusals.append(line)

    in_force = {}
    for source, text in [unit_file, *dropins]:
        if source not in (path, own):
            refuse(f"a drop-in that is not the rail's: {source}")
        for section, key, value in assignments(text):
            if not value:
                # An empty assignment resets the key's list (systemd.unit(5), drop-ins).
                in_force[(section, key)] = []
                continue
            in_force.setdefault((section, key), []).append(value)
            reason = credential_refusal(key, value, contract["socket"])
            if reason:
                refuse(reason)
            if key == "Environment":
                for reason in environment_refusals(value):
                    refuse(reason)
    for section, key, values in contract["rows"].get(name, []):
        for value in values:
            if value in in_force.get((section, key), []):
                refuse(f"neutral value left in force: [{section}] {shown(key, value)}")
    for (section, key), values in in_force.items():
        for value in values:
            if carries_neutral(key, value, contract["neutral"]):
                refuse(f"neutral value left in force: [{section}] {shown(key, value)}")
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
    carried, files, lines = {}, 0, 0
    for unit, path in sources:
        text = path.read_text(encoding="utf-8")
        files, lines = files + 1, lines + len(text.splitlines())
        for section, key, value in assignments(text):
            if value and carries_neutral(key, value, contract["neutral"]):
                carried.setdefault((unit, section, key, value), None)
    named = {
        (unit, section, key, value)
        for unit, listed in contract["rows"].items()
        for section, key, values in listed
        for value in values
    }
    refusals = [
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
        if name == "-":
            texts.append(sys.stdin.read())
        else:
            try:
                texts.append(Path(name).read_text(encoding="utf-8"))
            except OSError as error:
                raise Unjudgeable(f"{name}: {error.strerror}") from None
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

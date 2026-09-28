"""systemd unit syntax as systemd.syntax(7) reads it, for the deploy templates' tests (SPEC-032,
SPEC-056 R5).

DeckStreak's own reader: it parses the templates the tests assert over, and judges nothing. The
durable-services pack judges the templates on the maintainer's box (ADR-069).
"""

import dataclasses
import re
import unicodedata
from pathlib import Path

DEPLOY = "deploy"
UNIT_KINDS = {".service": "service", ".timer": "timer", ".slice": "slice"}
KEY_NAME = re.compile(r"^[A-Za-z0-9-]+$")
SECTION = re.compile(r"\[[A-Za-z0-9-]+\]")
# What the reader refuses rather than read a line otherwise than systemd reads it (SPEC-066).
BACKSLASH = "ends in a backslash, which the reader refuses"
CHARACTER = "a character the reader refuses"
SHAPE = "is neither a section header nor an assignment in a section"
# A variable whose name says it carries a secret, which systemd.exec(5) keeps out of the environment.
SECRET_NAME = re.compile(
    r"(?:^|_)(?:TOKEN|SECRET|PASSWORD|PASSWD|APIKEY|API_KEY|PRIVATE_KEY|ACCESS_KEY|SIGNING_KEY|"
    r"CREDENTIALS?|DSN)(?:$|_)",
    re.IGNORECASE,
)
# A byte size with a base-1024 suffix (systemd.resource-control(5)).
SIZE = re.compile(r"^(\d+(?:\.\d+)?)\s*([KMGTPE]?)$", re.IGNORECASE)
# The key a unit waives an advisory departure with; systemd ignores an `X-` key (SPEC-056 R16).
WAIVE_KEY = "X-DurableServices-Waive"
# The service types that keep running after they start (systemd.service(5), Type=).
LONG_RUNNING_KINDS = {"simple", "exec", "notify", "notify-reload", "forking", "dbus", "idle"}
# systemd's WHITESPACE: what it splits a list of exit statuses on, and what its safe_atou8() skips
# before a number (SPEC-066).
WHITESPACE = " \t\n\r"
# The values each enum key the census reads may hold (systemd.service(5), systemd.unit(5)), with the
# section it sits in. The census refuses an empty or any other value of one rather than decide
# which earlier value it leaves in force (SPEC-066).
ENUMS = {
    "Restart": (
        "Service",
        {"no", "on-success", "on-failure", "on-abnormal", "on-watchdog", "on-abort", "always"},
    ),
    "RestartMode": ("Service", {"normal", "direct"}),
    "CollectMode": ("Unit", {"inactive", "inactive-or-failed"}),
}
# The exit-status names systemd reads as 0 and 1. Every other name it knows is a status of 2 or
# more (`systemd-analyze exit-status` lists them), never the refusal's 1, and reads as None here.
EXIT_NAMES = {"SUCCESS": 0, "FAILURE": 1}


class Refused(AssertionError):
    """A unit file the reader refuses to read, naming the file and the line (SPEC-066)."""


@dataclasses.dataclass(frozen=True, slots=True)
class Assignment:
    section: str
    key: str
    value: str
    line: int
    source: str


@dataclasses.dataclass(slots=True)
class Unit:
    """One unit file and its drop-ins, as systemd reads them."""

    name: str
    rel: str
    kind: str
    assignments: list

    def assigned(self, section, key):
        return any(a.section == section and a.key == key for a in self.assignments)

    def values(self, section, key):
        """Every value in order, with systemd's reset rule: an empty assignment clears the list."""
        out = []
        for assignment in self.assignments:
            if assignment.section == section and assignment.key == key:
                if assignment.value == "":
                    out = []
                else:
                    out.append(assignment.value)
        return out

    def every(self, section, key):
        """Every value in order, an empty one included, with no reset applied."""
        return [a.value for a in self.assignments if a.section == section and a.key == key]

    def last(self, section, key):
        found = [a.value for a in self.assignments if a.section == section and a.key == key]
        return found[-1] if found else None


@dataclasses.dataclass(slots=True)
class Subject:
    """Every unit under a root's `deploy/`, by file name."""

    units: dict

    @property
    def services(self):
        return [unit for unit in self.units.values() if unit.kind == "service"]

    @property
    def timers(self):
        return [unit for unit in self.units.values() if unit.kind == "timer"]


def unit_text(path):
    """A unit file's text, decoded from its bytes: `read_text` would turn a carriage return into a
    line break before the reader could refuse it."""
    return Path(path).read_bytes().decode("utf-8")


def refused_character(line):
    """The first character of `line` the reader refuses, or None: a control character other than a
    tab, or whitespace outside ASCII (SPEC-066)."""
    for char in line:
        if char == "\t":
            continue
        if unicodedata.category(char) == "Cc" or (char.isspace() and not char.isascii()):
            return char
    return None


def logical_lines(text, source):
    """Each line of `text` with its number, split on a line feed alone and stripped of spaces and
    tabs. The reader models no more of systemd.syntax(7) than that: it refuses a line that ends in a
    backslash, a comment's included, and one holding a character `refused_character` names, each
    as `source` and the line's number (SPEC-066)."""
    for number, raw in enumerate(text.split("\n"), start=1):
        char = refused_character(raw)
        if char is not None:
            raise Refused(f"{source}:{number}: holds U+{ord(char):04X}, {CHARACTER}")
        line = raw.strip(" \t")
        if line.endswith("\\"):
            raise Refused(f"{source}:{number}: {BACKSLASH}")
        yield number, line


def assignments(text, source):
    """Each assignment of `text` in order, as (section, key, value, line). A line is blank, a
    comment (`#` or `;`), a section header or a `Key=Value` inside a section, and the reader refuses
    any other (SPEC-066)."""
    section = None
    for number, line in logical_lines(text, source):
        if not line or line.startswith(("#", ";")):
            continue
        if SECTION.fullmatch(line):
            section = line[1:-1]
            continue
        key, equals, value = line.partition("=")
        key = key.strip(" \t")
        if not equals or section is None or not KEY_NAME.match(key):
            raise Refused(f"{source}:{number}: {SHAPE}")
        yield section, key, value.strip(" \t"), number


def read_into(unit, path, source):
    """Every assignment of `path` into `unit`, under its section, through `assignments`."""
    for section, key, value, number in assignments(unit_text(path), source):
        unit.assignments.append(Assignment(section, key, value, number, source))


def parse_unit(root, path):
    """The unit at `path`, with the drop-ins of its `<name>.d/` directory read after it."""
    unit = Unit(path.name, path.relative_to(root).as_posix(), UNIT_KINDS[path.suffix], [])
    read_into(unit, path, unit.rel)
    for dropin in sorted((path.parent / f"{path.name}.d").glob("*.conf")):
        read_into(unit, dropin, dropin.relative_to(root).as_posix())
    return unit


def load_subject(root):
    """Every unit file under `root`'s `deploy/`, a drop-in read with the unit it belongs to."""
    root = Path(root)
    units = {}
    deploy = root / DEPLOY
    if deploy.is_dir():
        for path in sorted(deploy.rglob("*")):
            if path.is_file() and path.suffix in UNIT_KINDS and not path.parent.name.endswith(".d"):
                units[path.name] = parse_unit(root, path)
    return Subject(units)


def service_type(unit):
    """Type= as systemd.service(5) defaults it: dbus with BusName=, simple with ExecStart=, else
    oneshot."""
    declared = unit.last("Service", "Type")
    if declared:
        return declared.strip()
    if unit.last("Service", "BusName"):
        return "dbus"
    if unit.values("Service", "ExecStart"):
        return "simple"
    return "oneshot"


def waivers(unit):
    """Each `X-DurableServices-Waive=<reason> <why>` of the unit's `[Unit]` section, as (reason,
    why): the advisory departures the unit declares on purpose (SPEC-056 R16). A waiver with no
    why is kept with an empty one, so a test can refuse it."""
    found = []
    for value in unit.values("Unit", WAIVE_KEY):
        reason, _, why = value.strip().partition(" ")
        found.append((reason, why.strip()))
    return found


def long_running(unit):
    return unit.kind == "service" and service_type(unit) in LONG_RUNNING_KINDS


def status_words(value):
    """The words of a `SuccessExitStatus=` or `RestartForceExitStatus=` value as systemd splits
    one, with extract_first_word() and no flags: on whitespace, where a backslash takes the next
    character as it is and a quote is a plain character, so `\\1` is the word `1` and `"1"` stays
    `"1"`. A trailing backslash ends the value there, as systemd's refusal of it does (SPEC-066)."""
    words, word, escaped = [], None, False
    for char in value:
        if escaped:
            word, escaped = word + char, False
        elif char == "\\":
            word, escaped = word or "", True
        elif char in WHITESPACE:
            if word is not None:
                words.append(word)
            word = None
        else:
            word = (word or "") + char
    if word is not None and not escaped:
        words.append(word)
    return words


def exit_status(word):
    """The exit status systemd reads `word` as, or None where it reads none (SPEC-066): a name, else
    a number as its safe_atou8() reads one. That skips leading whitespace, reads its own `0b`
    (binary) and `0o` (octal) prefixes, and hands the rest to strtoul(3) in that base, or in base 0;
    the whole word must be read, a negative number is refused unless it is 0, and a status is at
    most 255. So 01, 0x1, +1, 0b1 and 0o1 read as 1 and 010 as 8, as `systemd-analyze exit-status`
    reads them."""
    if word in EXIT_NAMES:
        return EXIT_NAMES[word]
    text, base = word.lstrip(WHITESPACE), 0
    if text[:2] in ("0b", "0B"):
        text, base = text[2:], 2
    elif text[:2] in ("0o", "0O"):
        text, base = text[2:], 8
    number = strtoul(text, base)
    if number is None:
        return None
    value, negative = number
    if negative and value:
        return None
    return value if value <= 255 else None


def strtoul(text, base):
    """strtoul(3) over the whole of `text` as ISO C23 reads it, the most a C library reads: the
    value and whether a `-` preceded it, or None when `text` is not one number. It skips leading C
    whitespace and takes one sign; a `0x` before a hexadecimal digit names base 16 in base 0 or 16,
    a `0b` before a binary digit names base 2 in base 0 or 2, and in base 0 a leading `0` is
    octal."""
    sign, digits = re.fullmatch(r"[ \t\n\v\f\r]*([+-]?)(.*)", text, re.DOTALL).groups()
    if base in (0, 16) and re.match(r"0[xX][0-9a-fA-F]", digits):
        base, digits = 16, digits[2:]
    elif base in (0, 2) and re.match(r"0[bB][01]", digits):
        base, digits = 2, digits[2:]
    elif base == 0:
        base = 8 if digits.startswith("0") else 10
    allowed = "0123456789abcdef"[:base]
    if not digits or any(char.lower() not in allowed for char in digits):
        return None
    return int(digits, base), sign == "-"


def size_bytes(value):
    """A byte size in bytes; None for a percentage, `infinity` or anything unreadable."""
    if value is None:
        return None
    match = SIZE.match(value.strip())
    if not match:
        return None
    number, suffix = match.groups()
    power = " KMGTPE".index(suffix.upper() or " ")
    return int(float(number) * (1024**power))


def env_assignments(path):
    """`KEY=VALUE` lines of an env file, as (line, key, value): `export` tolerated, comments
    skipped."""
    out = []
    for number, raw in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith(("#", ";")) or "=" not in line:
            continue
        key, _, value = line.removeprefix("export ").partition("=")
        out.append((number, key.strip(), value.strip()))
    return out

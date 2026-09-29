"""The systemd unit syntax the deploy templates use (systemd.syntax(7)), for the templates' tests
(SPEC-032, SPEC-056 R5).

DeckStreak's own reader: it parses the templates the tests assert over, and judges nothing. It does
not model systemd's parser: it reads the plain syntax the templates hold and refuses whatever else a
line or an exit-status word holds (SPEC-066). The durable-services pack judges the templates on the
maintainer's box (ADR-069).
"""

import dataclasses
import re
import unicodedata
from pathlib import Path

DEPLOY = "deploy"
UNIT_KINDS = {".service": "service", ".timer": "timer", ".slice": "slice"}
KEY_NAME = re.compile(r"^[A-Za-z0-9-]+$")
SECTION = re.compile(r"\[[A-Za-z0-9-]+\]")
# Why the reader refuses a line: it reads only the plain syntax the templates hold (SPEC-066).
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
# The prefixes of a `[Unit]` condition's key and of an assertion's (systemd.unit(5)).
STOPS_A_START = ("Condition", "Assert")
# The exit-status names, each with its status, as `systemd-analyze exit-status` lists them and
# systemd.exec(5) documents them. The census reads no other name (SPEC-066).
EXIT_NAMES = {
    "SUCCESS": 0,
    "FAILURE": 1,
    "INVALIDARGUMENT": 2,
    "NOTIMPLEMENTED": 3,
    "NOPERMISSION": 4,
    "NOTINSTALLED": 5,
    "NOTCONFIGURED": 6,
    "NOTRUNNING": 7,
    "USAGE": 64,
    "DATAERR": 65,
    "NOINPUT": 66,
    "NOUSER": 67,
    "NOHOST": 68,
    "UNAVAILABLE": 69,
    "SOFTWARE": 70,
    "OSERR": 71,
    "OSFILE": 72,
    "CANTCREAT": 73,
    "IOERR": 74,
    "TEMPFAIL": 75,
    "PROTOCOL": 76,
    "NOPERM": 77,
    "CONFIG": 78,
    "CHDIR": 200,
    "NICE": 201,
    "FDS": 202,
    "EXEC": 203,
    "MEMORY": 204,
    "LIMITS": 205,
    "OOM_ADJUST": 206,
    "SIGNAL_MASK": 207,
    "STDIN": 208,
    "STDOUT": 209,
    "CHROOT": 210,
    "IOPRIO": 211,
    "TIMERSLACK": 212,
    "SECUREBITS": 213,
    "SETSCHEDULER": 214,
    "CPUAFFINITY": 215,
    "GROUP": 216,
    "USER": 217,
    "CAPABILITIES": 218,
    "CGROUP": 219,
    "SETSID": 220,
    "CONFIRM": 221,
    "STDERR": 222,
    "PAM": 224,
    "NETWORK": 225,
    "NAMESPACE": 226,
    "NO_NEW_PRIVILEGES": 227,
    "SECCOMP": 228,
    "SELINUX_CONTEXT": 229,
    "PERSONALITY": 230,
    "APPARMOR": 231,
    "ADDRESS_FAMILIES": 232,
    "RUNTIME_DIRECTORY": 233,
    "CHOWN": 235,
    "SMACK_PROCESS_LABEL": 236,
    "KEYRING": 237,
    "STATE_DIRECTORY": 238,
    "CACHE_DIRECTORY": 239,
    "LOGS_DIRECTORY": 240,
    "CONFIGURATION_DIRECTORY": 241,
    "NUMA_POLICY": 242,
    "CREDENTIALS": 243,
    "BPF": 244,
    "KSM": 245,
    "EXCEPTION": 255,
}
# The one spelling of a number the census reads as an exit status: a decimal with no sign and no
# leading zero, of ASCII digits (SPEC-066).
DECIMAL = re.compile(r"0|[1-9][0-9]{0,2}")


# The keys each kind of unit that loads a credential may hold, by section, written out by hand
# rather than read from the tree (SPEC-066 R2, R3). A key off its unit's list, in any section, is
# refused, so a directive the census does not know cannot skip a start or turn its failure into a
# success. The alert template's list is its own and names no `OnFailure=`.
ALERT_KEYS = {
    "Unit": ("After", "Description", "Documentation", "Wants"),
    "Service": (
        "CapabilityBoundingSet",
        "ExecStart",
        "Group",
        "LoadCredential",
        "LockPersonality",
        "MemoryDenyWriteExecute",
        "MemoryHigh",
        "MemoryMax",
        "NoNewPrivileges",
        "PrivateDevices",
        "PrivateTmp",
        "ProcSubset",
        "ProtectClock",
        "ProtectControlGroups",
        "ProtectHome",
        "ProtectHostname",
        "ProtectKernelLogs",
        "ProtectKernelModules",
        "ProtectKernelTunables",
        "ProtectProc",
        "ProtectSystem",
        "RestrictAddressFamilies",
        "RestrictNamespaces",
        "RestrictRealtime",
        "RestrictSUIDSGID",
        "SupplementaryGroups",
        "SyslogIdentifier",
        "SystemCallArchitectures",
        "SystemCallFilter",
        "TimeoutStartSec",
        "Type",
        "UMask",
        "User",
    ),
}
PAGING_KEYS = {
    "Unit": (
        "After",
        "Description",
        "Documentation",
        "OnFailure",
        "StartLimitBurst",
        "StartLimitIntervalSec",
        "Wants",
    ),
    "Install": ("WantedBy",),
    "Service": (
        "CPUQuota",
        "CapabilityBoundingSet",
        "EnvironmentFile",
        "ExecStart",
        "Group",
        "IOSchedulingClass",
        "LoadCredential",
        "LockPersonality",
        "MemoryDenyWriteExecute",
        "MemoryHigh",
        "MemoryMax",
        "Nice",
        "NoNewPrivileges",
        "OOMPolicy",
        "PrivateDevices",
        "PrivateTmp",
        "ProcSubset",
        "ProtectClock",
        "ProtectControlGroups",
        "ProtectHome",
        "ProtectHostname",
        "ProtectKernelLogs",
        "ProtectKernelModules",
        "ProtectKernelTunables",
        "ProtectProc",
        "ProtectSystem",
        "Restart",
        "RestartSec",
        "RestrictAddressFamilies",
        "RestrictNamespaces",
        "RestrictRealtime",
        "RestrictSUIDSGID",
        "StateDirectory",
        "SyslogIdentifier",
        "SystemCallArchitectures",
        "SystemCallFilter",
        "TasksMax",
        "TimeoutStartSec",
        "TimeoutStopSec",
        "Type",
        "UMask",
        "User",
        "WatchdogSec",
    ),
}


# The values a key of a paging unit that loads a credential may hold, at every assignment, its
# drop-ins included (SPEC-066 R2). A value off this table is refused, by key and value. The table
# bounds values only: that a unit which restarts holds the whole restart budget is the presence
# rule, `RESTART_BUDGET` below.
PAGING_VALUES = {
    ("Service", "Restart"): ("on-failure",),
    ("Unit", "StartLimitIntervalSec"): ("300",),
    ("Unit", "StartLimitBurst"): ("5",),
    ("Service", "RestartSec"): ("15",),
    ("Unit", "After"): ("network-online.target",),
    ("Unit", "Wants"): ("network-online.target",),
}


# The keys a unit that loads a credential and pages holds when it assigns `Restart=` to anything
# but `no` (SPEC-066 R2): without the start limit the default interval is shorter than the restart
# delay, so the unit never reaches failed and pages on every restart.
RESTART_BUDGET = (
    ("Unit", "StartLimitIntervalSec"),
    ("Unit", "StartLimitBurst"),
    ("Service", "RestartSec"),
)


class Refused(AssertionError):
    """A unit file the reader refuses to read, naming the file and the line (SPEC-066)."""


def off_list(pairs, allowed):
    """For each (section, key) of `pairs`, whether it is off `allowed`, the list of a kind of unit
    (SPEC-066 R2)."""
    return [key not in allowed.get(section, ()) for section, key in pairs]


@dataclasses.dataclass(frozen=True, slots=True)
class Assignment:
    section: str
    key: str
    value: str
    line: int
    source: str


@dataclasses.dataclass(slots=True)
class Unit:
    """One unit file and its drop-ins, as the reader reads them."""

    name: str
    rel: str
    kind: str
    assignments: list
    # A template's instances' drop-ins (`<name>@<instance>.<type>.d/`): systemd reads them with the
    # instance, so they are part of the template's effective unit, but each is one instance's own
    # and is kept apart from the template's assignments (SPEC-062 R14).
    instance_dropins: list = dataclasses.field(default_factory=list)

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
    """The unit at `path`, with the drop-ins of its `<name>.d/` directory read after it, and, for a
    template `<name>@.<type>`, those of each instance's `<name>@<instance>.<type>.d/` directory
    read into `instance_dropins`."""
    unit = Unit(path.name, path.relative_to(root).as_posix(), UNIT_KINDS[path.suffix], [])
    read_into(unit, path, unit.rel)
    for dropin in sorted((path.parent / f"{path.name}.d").glob("*.conf")):
        read_into(unit, dropin, dropin.relative_to(root).as_posix())
    if "@." in path.name:
        stem, suffix = path.name.split("@.", 1)
        held = Unit(path.name, unit.rel, unit.kind, [])
        for folder in sorted(path.parent.glob(f"{stem}@*.{suffix}.d")):
            for dropin in sorted(folder.glob("*.conf")):
                read_into(held, dropin, dropin.relative_to(root).as_posix())
        unit.instance_dropins = held.assignments
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
    """The words of a `SuccessExitStatus=` or `RestartForceExitStatus=` value, split on spaces and
    tabs alone. A backslash or a quote stays in its word, which `exit_status` then reads as no
    status, so the census refuses it rather than decide how systemd would split it (SPEC-066)."""
    return [word for word in re.split(r"[ \t]+", value) if word]


def exit_status(word):
    """The exit status `word` names, or None for a word the census does not read: a status name
    `EXIT_NAMES` lists, or a decimal of at most 255 with no sign and no leading zero. Every other
    spelling reads as None, and the census refuses it (SPEC-066)."""
    if word in EXIT_NAMES:
        return EXIT_NAMES[word]
    if DECIMAL.fullmatch(word) and int(word) <= 255:
        return int(word)
    return None


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

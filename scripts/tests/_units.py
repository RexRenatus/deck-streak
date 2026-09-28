"""systemd unit syntax as systemd.syntax(7) reads it, for the deploy templates' tests (SPEC-032,
SPEC-056 R5).

DeckStreak's own reader: it parses the templates the tests assert over, and judges nothing. The
durable-services pack judges the templates on the maintainer's box (ADR-069).
"""

import dataclasses
import re
from pathlib import Path

DEPLOY = "deploy"
UNIT_KINDS = {".service": "service", ".timer": "timer", ".slice": "slice"}
KEY_NAME = re.compile(r"^[A-Za-z0-9-]+$")
# A variable whose name says it carries a secret, which systemd.exec(5) keeps out of the environment.
SECRET_NAME = re.compile(
    r"(?:^|_)(?:TOKEN|SECRET|PASSWORD|PASSWD|APIKEY|API_KEY|PRIVATE_KEY|ACCESS_KEY|SIGNING_KEY|"
    r"CREDENTIALS?|DSN)(?:$|_)",
    re.IGNORECASE,
)
# A byte size with a base-1024 suffix (systemd.resource-control(5)).
SIZE = re.compile(r"^(\d+(?:\.\d+)?)\s*([KMGTPE]?)$", re.IGNORECASE)
# The service types that keep running after they start (systemd.service(5), Type=).
LONG_RUNNING_KINDS = {"simple", "exec", "notify", "notify-reload", "forking", "dbus", "idle"}


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


def logical_lines(text):
    """systemd.syntax(7): comments start `#` or `;`, a trailing backslash joins the next line (a
    comment line inside the join is skipped), and every line keeps the number it started on."""
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


def read_into(unit, path, source):
    """Every `Key=Value` of `path` into `unit`, under its section; other lines are skipped."""
    section = None
    for number, line in logical_lines(path.read_text(encoding="utf-8")):
        if not line or line.startswith(("#", ";")):
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            continue
        key, equals, value = line.partition("=")
        key = key.strip()
        if equals and KEY_NAME.match(key) and section is not None:
            unit.assignments.append(Assignment(section, key, value.strip(), number, source))


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


def long_running(unit):
    return unit.kind == "service" and service_type(unit) in LONG_RUNNING_KINDS


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

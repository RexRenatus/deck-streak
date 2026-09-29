"""The deploy templates: every unit, timer and the Caddy block is hardened, fits the host budget,
and names no private value (SPEC-032; ADR-007, ADR-010, ADR-025, ADR-032, ADR-038), SPEC-031's
alert, SLO evaluator and memory watch included (SPEC-031 R6; ADR-031); and every unit that loads
a credential fails and pages when a credential refuses its start (SPEC-066 R2; ADR-067).

The units are read with `_units.py`, DeckStreak's own reader of systemd unit syntax; the
durable-services pack judges the templates on the maintainer's box (ADR-069, SPEC-056). Every
enumerating test prints `examined N` and refuses zero, and every
absence it asserts is paired with a planted template it must refuse. No test writes a template
instance name literally (SPEC-032 R10): each is built at run time from its template and its id.
"""

import ipaddress
import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import _units
from _support import REPO, examined

DEPLOY = REPO / "deploy"
SYSTEMD = DEPLOY / "systemd"
CADDY = DEPLOY / "caddy" / "deck-streak.caddy"
BUDGET = DEPLOY / "host-budget.json"
ENV_EXAMPLE = DEPLOY / "deck-streak.env.example"
SCRUB = REPO / "scripts" / "public-scrub.py"
ADR = REPO / "docs" / "decisions" / "ADR-032-deploy-templates-and-the-host-budget.md"

# The one release binary every service runs, from the release root's `current` link (R2).
BINARY = "/usr/local/lib/deck-streak/current/bin/deckstreakd"
# The release root, whose deploy/ scripts SPEC-031's units run (ADR-032's neutral root).
RELEASE = "/usr/local/lib/deck-streak/current"
# The one required settings file of every service (R3), named without the optional `-`.
ENVIRONMENT_FILE = "/etc/deck-streak/deck-streak.env"
# The credential socket the private rail serves (ADR-038).
SOCKET = "/run/deck-streak-credentials/socket"
# The alert template every service pages through on failure (R2; SPEC-031 ships it).
ON_FAILURE = "deck-streak-alert@%n.service"
# The directives a unit loads a credential with (systemd.exec(5)). A unit holding any of them
# fails and pages when a credential refuses its start, the alert template excepted (SPEC-066 R2).
CREDENTIAL_KEYS = (
    "LoadCredential",
    "LoadCredentialEncrypted",
    "SetCredential",
    "SetCredentialEncrypted",
    "ImportCredential",
)
# The exit of a role that refuses start (SPEC-025 R1) and of the runner's page (SPEC-027 R7),
# EXIT_FAILURE: no template counts it a success, and the census reads no other spelling of a status
# (SPEC-066 R2).
REFUSAL_EXIT = 1
# The prefixes systemd reads before an ExecStart= path; `-` counts a failure as a success
# (systemd.service(5)).
EXEC_PREFIX = re.compile(r"^[-@:+!|]*")
# The job template, whose instances the timers start (R1).
JOB_TEMPLATE = "deck-streak-job"
# SPEC-031's units: the alert template, the SLO evaluator and the memory watch. Each runs a script
# of the release, not a role of the binary (SPEC-031 R3 to R5).
ALERT_TEMPLATE = "deck-streak-alert"
SLO_SERVICE = "deck-streak-slo.service"
WATCH_SERVICE = "deck-streak-memory-watch.service"
SCRIPTS = {
    f"{ALERT_TEMPLATE}@.service": f"{RELEASE}/deploy/scripts/alert-telegram.sh %i",
    SLO_SERVICE: (
        f"/usr/bin/python3 {RELEASE}/deploy/scripts/slo-evaluate.py {RELEASE}/deploy/slo.json"
    ),
    WATCH_SERVICE: f"{RELEASE}/deploy/scripts/memory-watch.sh",
}
# Their lifecycle: a oneshot each, ended before its timer is due again; the two a timer starts
# yield to the daemons as a job does (resources.batch-priority), and the alert pages at once.
OBSERVABILITY_SERVICE = {
    f"{ALERT_TEMPLATE}@.service": {"Type": "oneshot", "TimeoutStartSec": "3min"},
    SLO_SERVICE: {
        "Type": "oneshot",
        "TimeoutStartSec": "4min",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
    WATCH_SERVICE: {
        "Type": "oneshot",
        "TimeoutStartSec": "50s",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
}
# The timers that start SPEC-031's units, each the service of its own name.
OBSERVABILITY_TIMERS = {"deck-streak-slo.timer", "deck-streak-memory-watch.timer"}

# Where the code declares each credential id a role reads, by the constant's name.
CREDENTIAL_SOURCES = {
    "OWNER_USER_ID": REPO / "crates" / "identity" / "src" / "owner.rs",
    "TELEGRAM_BOT_TOKEN": REPO / "crates" / "identity" / "src" / "owner.rs",
    "SYNC_USERNAME": REPO / "crates" / "ingest" / "src" / "settings.rs",
    "SYNC_PASSWORD": REPO / "crates" / "ingest" / "src" / "settings.rs",
}
# Which credentials each service's role reads: the api's owner gate (SPEC-024, SPEC-025), the bot's
# transport, owner gate and `/sync` (SPEC-026 R1, R11), and the `sync` job's syncer (SPEC-022,
# SPEC-027). The job template carries the sync's pair for every instance, and the private rail's map
# answers them for the `sync` instance alone, the one job that reads them (ADR-038; SPEC-061 §8,
# A14). SPEC-031's alert reads the bot token and the owner's id, whose private chat it pages (R3);
# the evaluator and the watch read none.
ROLE_CREDENTIALS = {
    "deck-streak-api.service": ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"),
    "deck-streak-bot.service": (
        "OWNER_USER_ID",
        "TELEGRAM_BOT_TOKEN",
        "SYNC_USERNAME",
        "SYNC_PASSWORD",
    ),
    f"{JOB_TEMPLATE}@.service": ("SYNC_USERNAME", "SYNC_PASSWORD"),
    f"{ALERT_TEMPLATE}@.service": ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"),
    SLO_SERVICE: (),
    WATCH_SERVICE: (),
}
# The role each service runs (R2); the job template's `%i` is its instance, the job's id.
ROLES = {
    "deck-streak-api.service": "api",
    "deck-streak-bot.service": "bot",
    f"{JOB_TEMPLATE}@.service": "job %i",
}
# The settings a role requires, which the committed example must therefore name (SPEC-025 R6,
# SPEC-022 R5); systemd sets STATE_DIRECTORY and CREDENTIALS_DIRECTORY itself.
REQUIRED_SETTINGS = ("DECKSTREAK_API_LISTEN", "DECKSTREAK_SYNC_ENDPOINT")
SYSTEMD_SETS = ("STATE_DIRECTORY", "CREDENTIALS_DIRECTORY")

# R1: the lifecycle of the two long-running services.
DAEMON_SERVICE = {
    "Type": "notify",
    "WatchdogSec": "90",
    "Restart": "on-failure",
    "RestartSec": "15",
    "TimeoutStartSec": "180",
    "TimeoutStopSec": "30",
    "OOMPolicy": "kill",
}
DAEMON_UNIT = {"StartLimitBurst": "5", "StartLimitIntervalSec": "300"}
# R1: the job template yields to the daemons, and ADR-032's delivery bounds a hung run.
JOB_SERVICE = {
    "Type": "oneshot",
    "Nice": "10",
    "IOSchedulingClass": "idle",
    "TimeoutStartSec": "30min",
}
# ADR-032's delivery: the daemons' CPU and task caps.
DAEMON_CAPS = {
    "deck-streak-api.service": {"CPUQuota": "100%", "TasksMax": "64"},
    "deck-streak-bot.service": {"CPUQuota": "50%", "TasksMax": "64"},
}
# R2: the identity and hardening of every service, each at the value the pack's rows score.
HARDENING = {
    "User": "deck-streak",
    "Group": "deck-streak",
    "UMask": "0077",
    "ProtectSystem": "strict",
    "ProtectHome": "yes",
    "PrivateTmp": "yes",
    "PrivateDevices": "yes",
    "NoNewPrivileges": "yes",
    "SystemCallFilter": "@system-service",
    "SystemCallArchitectures": "native",
    "CapabilityBoundingSet": "",
    "ProtectKernelTunables": "yes",
    "ProtectKernelModules": "yes",
    "ProtectKernelLogs": "yes",
    "ProtectControlGroups": "yes",
    "ProtectClock": "yes",
    "ProtectHostname": "yes",
    "ProtectProc": "invisible",
    "ProcSubset": "pid",
    "RestrictNamespaces": "yes",
    "RestrictRealtime": "yes",
    "RestrictSUIDSGID": "yes",
    "LockPersonality": "yes",
    "MemoryDenyWriteExecute": "yes",
}
# R2, R3 and SPEC-031 R6, per service: the one directory it may write, the settings file it reads,
# the address families it may open, and the journal it may read. The roles share the state
# directory and the settings file; SPEC-031's units read no settings, so the alert path never waits
# on a settings file; the alert writes nothing, the evaluator and the watch each keep their episodes
# in a directory of their own and open no network socket, and only the two that quote or count
# journal lines join the journal's group.
ROLES_NETWORK = "AF_UNIX AF_INET AF_INET6"
PER_SERVICE = {
    # service: (StateDirectory, EnvironmentFile, RestrictAddressFamilies, SupplementaryGroups)
    "deck-streak-api.service": ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    "deck-streak-bot.service": ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    f"{JOB_TEMPLATE}@.service": ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    f"{ALERT_TEMPLATE}@.service": (None, None, ROLES_NETWORK, "systemd-journal"),
    SLO_SERVICE: ("deck-streak-slo", None, "AF_UNIX", "systemd-journal"),
    WATCH_SERVICE: ("deck-streak-memory-watch", None, "AF_UNIX", None),
}
PER_SERVICE_KEYS = (
    "StateDirectory",
    "EnvironmentFile",
    "RestrictAddressFamilies",
    "SupplementaryGroups",
)
# The security headers R6 names, at the values the Caddy block sends.
HEADERS = {
    "Content-Security-Policy": (
        "frame-ancestors https://web.telegram.org; object-src 'none'; base-uri 'self'"
    ),
    "Strict-Transport-Security": "max-age=31536000; includeSubDomains",
    "X-Content-Type-Options": "nosniff",
    "X-Robots-Tag": "noindex",
}
# Referrer policies that never send a URL to another origin (web-security `ws.referrer-policy`).
KEEPS_URLS = {"no-referrer", "same-origin", "strict-origin", "strict-origin-when-cross-origin"}
ONE_YEAR = 31_536_000


# Every advisory departure the templates declare in their units, by unit and reason (SPEC-032 R4,
# SPEC-056 R16): the job table places each job on its own minute and makes none a catch-up job
# (ADR-027), and SPEC-031's two timers read rolling state that a missed run cannot lose. The box
# run holds every departure the durable lint reports to one of these, or to an issue it waits on
# (SPEC-056 R15).
WAIVED = {
    (f"{JOB_TEMPLATE}@sync.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@maintenance.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@maintenance.timer", "calendar-not-persistent"),
    (f"{JOB_TEMPLATE}@liveness.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@liveness.timer", "calendar-not-persistent"),
    ("deck-streak-slo.timer", "calendar-not-persistent"),
    ("deck-streak-memory-watch.timer", "calendar-not-persistent"),
}


def subject(root=REPO):
    """Every unit under `root`'s deploy/, through the reader, which refuses a line it cannot
    read."""
    return _units.load_subject(Path(root))


def services(root=REPO):
    return examined("service unit(s) under deploy/", sorted(subject(root).services, key=name))


def reader_refusal(files):
    """The reader's refusal of a `deploy/systemd/` holding `files`, each a file name and its
    content, or None when it reads every one (SPEC-066)."""
    with tempfile.TemporaryDirectory() as scratch:
        folder = Path(scratch) / "deploy" / "systemd"
        folder.mkdir(parents=True)
        for file, content in files.items():
            (folder / file).write_bytes(content.encode("utf-8"))
        try:
            subject(scratch)
        except _units.Refused as refusal:
            return str(refusal)
    return None


def planted_refusals(text, check):
    """What `check` refuses of a service unit planted as `text`, or the reader's refusal of it."""
    rel = "deploy/systemd/planted.service"
    unit = _units.Unit("planted.service", rel, "service", [])
    try:
        for section, key, value, number in _units.assignments(text, rel):
            unit.assignments.append(_units.Assignment(section, key, value, number, rel))
    except _units.Refused as refusal:
        return [str(refusal)]
    return check(unit)


def name(unit):
    return unit.name


def last(unit, section, key):
    value = unit.last(section, key)
    return None if value is None else value.strip()


def size(text):
    """A systemd byte size such as 128M, in bytes."""
    found = _units.size_bytes(text)
    if found is None:
        raise AssertionError(f"{text!r} is not a byte size")
    return found


def budget():
    return json.loads(BUDGET.read_text(encoding="utf-8"))


def adr_budget():
    """ADR-032's budget table, by unit: (memory_high, memory_max). A row may name the SPEC that
    ships its unit after the unit's name, as SPEC-031's three rows do."""
    rows = re.findall(
        r"(?m)^\| `([^`]+)`(?: \(SPEC-\d{3}\))? \| (\d+M) \| (\d+M) \|",
        ADR.read_text(encoding="utf-8"),
    )
    return {unit: (high, ceiling) for unit, high, ceiling in rows}


def credential_ids():
    """Each credential id the code declares, by its constant's name."""
    found = {}
    for constant, source in CREDENTIAL_SOURCES.items():
        match = re.search(rf'pub const {constant}: &str = "([a-z0-9-]+)";', source.read_text())
        if match is None:
            raise AssertionError(f"{source.relative_to(REPO)} declares no {constant}")
        found[constant] = match.group(1)
    return found


def declared_settings():
    """Every setting the workspace's code names: a `DECKSTREAK_*` constant in `crates/*/src`."""
    found = set()
    for source in (REPO / "crates").glob("*/src/**/*.rs"):
        found.update(
            re.findall(r'pub const [A-Z_]+: &str = "(DECKSTREAK_[A-Z_]+)";', source.read_text())
        )
    return found


def env_example():
    """The committed example's active `KEY=VALUE` lines, as (line, key, value)."""
    return _units.env_assignments(ENV_EXAMPLE)


def credential_lines(root):
    """Every credential directive of every template under `root`, as (file, line, key, value)."""
    keys = CREDENTIAL_KEYS
    found = []
    for path in sorted(Path(root).rglob("*")):
        if path.suffix not in (".service", ".timer", ".conf") or not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        for _, key, value, number in _units.assignments(_units.unit_text(path), rel):
            if key in keys:
                found.append((rel, number, key, value))
    return found


NON_UNIT_DROPIN = "deploy/journald.conf.d"


def dropin_directory_refusals(root):
    """Every `*.d/` directory under `root`'s deploy/ that is not the drop-in directory of a unit
    shipped beside it, as one line each: only `<unit name>.d/` is read with a unit (SPEC-066 R2), so
    any other is refused, and the one directory of a file that is no unit is named here."""
    refused = []
    deploy = Path(root) / "deploy"
    if not deploy.is_dir():
        return refused
    entries = sorted(deploy.rglob("*"))
    own = {
        path.parent / f"{path.name}.d"
        for path in entries
        if path.is_file()
        and path.suffix in _units.UNIT_KINDS
        and not path.parent.name.endswith(".d")
    }
    for path in entries:
        rel = path.relative_to(root).as_posix()
        if not path.is_dir() or not path.name.endswith(".d") or path in own:
            continue
        if rel == NON_UNIT_DROPIN:
            continue
        refused.append(
            f"{rel}: is not the drop-in directory of a unit shipped beside it, and is refused"
        )
    return refused


def socket_form_refusals(lines, ids):
    """Every credential line that is not `LoadCredential=<a DeckStreak id>:<the socket>`."""
    refused = []
    for where, number, key, value in lines:
        ident, _, path = value.partition(":")
        if key != "LoadCredential":
            refused.append(f"{where}:{number}: {key}= is refused; use LoadCredential= (ADR-038)")
        elif ident not in ids:
            refused.append(f"{where}:{number}: {ident!r} is not a DeckStreak credential id")
        elif path != SOCKET:
            refused.append(f"{where}:{number}: {ident} is read from {path!r}, not the socket")
    return refused


def environment_refusals(unit, ids):
    """Every way `unit` passes a secret through its environment."""
    refused = []
    for value in unit.values("Service", "Environment") + unit.values("Service", "PassEnvironment"):
        for word in value.split():
            variable = word.partition("=")[0].strip("\"'")
            if _units.SECRET_NAME.search(variable) or variable.lower().replace("_", "-") in ids:
                refused.append(f"{unit.rel}: {variable} passes a secret through the environment")
    return refused


def loads_a_credential(unit):
    """Whether `unit` holds a credential directive of any kind."""
    return any(unit.values("Service", key) for key in CREDENTIAL_KEYS)


def names_the_refusal(statuses):
    """Whether a `SuccessExitStatus=` or `RestartForceExitStatus=` value holds a word the census
    reads as the refusal's exit, 1: `1` or `FAILURE` (`_units.exit_status`)."""
    return any(_units.exit_status(word) == REFUSAL_EXIT for word in _units.status_words(statuses))


def unread_words(statuses):
    """The words of a `SuccessExitStatus=` or `RestartForceExitStatus=` value the census does not
    read as an exit status: neither a decimal of at most 255 nor a status name. Each is refused,
    since the census reads no other spelling of a status (SPEC-066)."""
    return [word for word in _units.status_words(statuses) if _units.exit_status(word) is None]


def unread_refusal(key, statuses, word):
    """The census's refusal of `word`, one of `unread_words(statuses)`."""
    return (
        f"{key}={statuses} holds {word}, which is neither a decimal of at most 255 nor a status "
        "name, and is refused"
    )


def refusal_page_conditions(unit):
    """Why a start of `unit` that a credential refuses would not fail it and start its page
    (SPEC-066 R2), each refusal beside the directive it reads: no `OnFailure=` naming the alert
    template, a `[Unit]` condition or assertion or an `ExecCondition=`, any of which can stop the
    start, an `ExecStart=` whose failure counts as a success, a success exit that holds the
    refusal's, 1, or a word the census does not read as an exit status, or a restart that skips
    the failed state, and with it `OnFailure=`. A condition that exits 1 to 254 skips the start,
    and the unit is not marked failed, and an unmet `[Unit]` condition or assertion leaves it
    inactive (systemd.service(5), systemd.unit(5)); the census cannot tell what a condition or an
    assertion tests, so it refuses every one, an empty one included. A `RestartMode=direct`
    is refused wherever it is assigned, and a `Restart=`, `RestartMode=` or `CollectMode=` that is
    empty or not a known value is refused, so the census never decides which of two assignments is
    in force."""
    refused = []

    def refuse(directive, why):
        refused.append((directive, f"{unit.rel}: {why}"))

    targets = [word for value in unit.values("Unit", "OnFailure") for word in value.split()]
    if ON_FAILURE not in targets:
        refuse("OnFailure", f"OnFailure={' '.join(targets)} does not name {ON_FAILURE}")
    for assignment in unit.assignments:
        if assignment.section == "Unit" and assignment.key.startswith(_units.STOPS_A_START):
            refuse(
                "Condition",
                f"{assignment.key}={assignment.value} is refused, as every condition and "
                "assertion is, since one can stop the start without failing the unit or starting "
                "OnFailure=",
            )
    for command in unit.values("Service", "ExecCondition"):
        refuse(
            "ExecCondition",
            f"ExecCondition={command} can skip the start, which neither fails the unit nor starts "
            "OnFailure=",
        )
    for command in unit.values("Service", "ExecStart"):
        if "-" in EXEC_PREFIX.match(command).group(0):
            refuse("ExecStart", f"ExecStart={command} counts a failure as a success")
    for statuses in unit.values("Service", "SuccessExitStatus"):
        if names_the_refusal(statuses):
            refuse(
                "SuccessExitStatus", f"SuccessExitStatus={statuses} counts the refusal a success"
            )
        for word in unread_words(statuses):
            refuse("SuccessExitStatus", unread_refusal("SuccessExitStatus", statuses, word))
    for mode in unit.every("Service", "RestartMode"):
        if mode == "direct":
            refuse("RestartMode", "RestartMode=direct skips the failed state and OnFailure=")
    for key, (section, known) in _units.ENUMS.items():
        for value in unit.every(section, key):
            if value not in known:
                refuse(key, f"{key}={value} is empty or not a known value, which the check refuses")
    return refused


def off_list_refusals(unit, allowed):
    """Every assignment of `unit`, its drop-ins included, whose section and key are not on
    `allowed`, each naming its key (SPEC-066 R2)."""
    off = _units.off_list([(a.section, a.key) for a in unit.assignments], allowed)
    return [
        f"{a.source}:{a.line}: [{a.section}] {a.key}={a.value} is not on this unit's list of keys, "
        "and is refused"
        for a, refused in zip(unit.assignments, off, strict=True)
        if refused
    ]


def refusal_page_refusals(unit):
    """The refusals of `refusal_page_conditions`, each without the directive it reads."""
    return [refusal for _, refusal in refusal_page_conditions(unit)]


def alert_exit_refusals(unit):
    """Why a refused start of the alert template would count as a success or skip its failed state
    (SPEC-066 R3): each of R2's conditions but `OnFailure=`, which the alert template must not
    name. They are told apart by the directive each refusal reads, never by the refusal's text,
    since the restart mode's refusal names `OnFailure=` too. A `SuccessExitStatus=` whose every
    word reads as a status other than 1 is refused as well: the alert template names none."""
    conditions = refusal_page_conditions(unit)
    refused = [refusal for directive, refusal in conditions if directive != "OnFailure"]
    for statuses in unit.values("Service", "SuccessExitStatus"):
        if not names_the_refusal(statuses) and not unread_words(statuses):
            refused.append(
                f"{unit.rel}: SuccessExitStatus={statuses} is named, and the alert template names "
                "none"
            )
    return refused


def restart_refusals(unit):
    """Why a refused start of the alert template would not stay failed (SPEC-066 R3): a restart of
    it. At the default `RestartMode=` a restart only passes through the failed state, and the
    instance waits for its next start activating, so a loop of restarts settles failed only when
    its start limit ends it (systemd.service(5), SPEC-031). So no `Restart=` assigns a known value
    other than `no`, wherever it is assigned, and no `RestartForceExitStatus=` at all: on
    `Type=oneshot` the service manager refuses a unit that names one outright, as a bad unit file
    setting, and on another type one naming the refusal's exit forces a restart whatever
    `Restart=` says. R2's units may restart: each failure a restart passes through still starts
    their `OnFailure=` page (SPEC-031). An empty or unknown `Restart=` is R2's refusal
    (`refusal_page_conditions`)."""
    refused = []
    for restart in unit.every("Service", "Restart"):
        if restart in _units.ENUMS["Restart"][1] and restart != "no":
            refused.append(f"{unit.rel}: Restart={restart} restarts the refusal")
    oneshot = _units.service_type(unit) == "oneshot"
    for statuses in unit.values("Service", "RestartForceExitStatus"):
        if oneshot:
            why = "makes the service manager refuse the Type=oneshot unit outright"
            refused.append(f"{unit.rel}: RestartForceExitStatus={statuses} {why}")
            continue
        unread = unread_words(statuses)
        for word in unread:
            why = unread_refusal("RestartForceExitStatus", statuses, word)
            refused.append(f"{unit.rel}: {why}")
        if names_the_refusal(statuses):
            refused.append(f"{unit.rel}: RestartForceExitStatus={statuses} restarts the refusal")
        elif not unread:
            why = "is named, and the alert template names none"
            refused.append(f"{unit.rel}: RestartForceExitStatus={statuses} {why}")
    return refused


def collect_refusals(unit):
    """Why the failed alert instance would leave `systemctl --failed` (SPEC-066 R3): its unloading.
    `CollectMode=inactive-or-failed` unloads a unit once it has failed, where `inactive` keeps a
    failed unit loaded until its failed state is reset (systemd.unit(5)). So no `CollectMode=`
    assigns a known value other than `inactive`, wherever it is assigned; an empty or unknown one
    is R2's refusal (`refusal_page_conditions`)."""
    return [
        f"{unit.rel}: CollectMode={mode} can unload the failed instance, which systemctl --failed "
        "then no longer lists"
        for mode in unit.every("Unit", "CollectMode")
        if mode in _units.ENUMS["CollectMode"][1] and mode != "inactive"
    ]


# --- the Caddyfile, read as Caddy's lexer reads it -------------------------------------------


class Directive:
    """One Caddyfile line: its tokens, the line it starts on, and the block it opens."""

    def __init__(self, tokens, line):
        self.tokens = tokens
        self.line = line
        self.children = []

    def find(self, *head):
        """Every child directive whose tokens start with `head`."""
        return [child for child in self.children if tuple(child.tokens[: len(head)]) == head]

    def one(self, *head):
        found = self.find(*head)
        if len(found) != 1:
            raise AssertionError(f"{len(found)} `{' '.join(head)}` directive(s), not one")
        return found[0]


def words(line):
    """A line's tokens: quoted strings keep their spaces, and a `#` that begins a token begins a
    comment."""
    tokens, current, quoted, index = [], "", False, 0
    while index < len(line):
        char = line[index]
        if quoted:
            if char == "\\" and index + 1 < len(line) and line[index + 1] == '"':
                current += '"'
                index += 1
            elif char == '"':
                quoted = False
            else:
                current += char
        elif char == '"':
            quoted = True
        elif char.isspace():
            if current:
                tokens.append(current)
            current = ""
        elif char == "#" and not current:
            break
        else:
            current += char
        index += 1
    if current:
        tokens.append(current)
    return tokens


def caddy_tree(text):
    """The Caddyfile as a tree of directives; a heredoc's body is one token."""
    root = Directive([], 0)
    stack = [root]
    lines = text.splitlines()
    number = 0
    while number < len(lines):
        tokens = words(lines[number])
        start = number + 1
        if tokens and tokens[-1].startswith("<<"):
            marker = tokens[-1][2:]
            body = []
            number += 1
            while not lines[number].strip().startswith(marker):
                body.append(lines[number])
                number += 1
            closing = lines[number]
            indent = closing[: len(closing) - len(closing.lstrip())]
            text_body = "\n".join(line.removeprefix(indent) for line in body)
            tokens = [*tokens[:-1], text_body, *words(closing.strip()[len(marker) :])]
        number += 1
        if not tokens:
            continue
        if tokens == ["}"]:
            stack.pop()
            continue
        opens = tokens[-1] == "{"
        directive = Directive(tokens[:-1] if opens else tokens, start)
        stack[-1].children.append(directive)
        if opens:
            stack.append(directive)
    if len(stack) != 1:
        raise AssertionError(f"{len(stack) - 1} block(s) left open")
    return root


def site():
    """The one site block of the Caddy template."""
    blocks = caddy_tree(CADDY.read_text(encoding="utf-8")).children
    sites = [block for block in blocks if block.children]
    if len(sites) != 1:
        raise AssertionError(f"{len(sites)} site block(s) in {CADDY.relative_to(REPO)}, not one")
    return sites[0]


def header_operations(block):
    """The site-level `header` operations, by field: `-Server` removes, anything else sets."""
    operations = {}
    for header in block.find("header"):
        lines = [header.tokens[1:]] if len(header.tokens) > 1 else []
        lines += [child.tokens for child in header.children]
        for tokens in lines:
            operations[tokens[0]] = " ".join(tokens[1:])
    return operations


def policy(text):
    """A Content-Security-Policy, by directive."""
    directives = {}
    for part in text.split(";"):
        tokens = part.split()
        if tokens:
            directives[tokens[0].lower()] = tokens[1:]
    return directives


def handle(block, *matcher):
    """The `handle` block with exactly this matcher (none for the fallback)."""
    return next(
        (h for h in block.find("handle") if tuple(h.tokens[1:]) == matcher),
        None,
    )


def route_constant(constant):
    source = REPO / "crates" / "api" / "src" / "health.rs"
    match = re.search(rf'pub const {constant}: &str = "([^"]+)";', source.read_text())
    if match is None:
        raise AssertionError(f"crates/api/src/health.rs declares no {constant}")
    return match.group(1)


def scrub(*subjects):
    return subprocess.run(
        [sys.executable, str(SCRUB), "--root", str(REPO), "--no-tree"]
        + [argument for path in subjects for argument in ("--subject", str(path))],
        capture_output=True,
        text=True,
        check=False,
    )


def declared_waivers(root=REPO):
    """Every waiver the units under `root`'s deploy/ declare: (unit, reason) to its why."""
    declared = {}
    for unit in subject(root).units.values():
        for reason, why in _units.waivers(unit):
            declared[(unit.name, reason)] = why
    return declared


class EveryAdvisoryDepartureIsWaivedInItsUnit(unittest.TestCase):
    def test_every_advisory_waiver_is_pinned_with_its_why(self):
        # The reader finds a planted waiver, and drops neither its reason nor its why.
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "deploy" / "systemd"
            planted.mkdir(parents=True)
            (planted / "planted.timer").write_text(
                "[Unit]\nDescription=planted\n"
                "X-DurableServices-Waive=calendar-not-persistent a planted why of six words\n"
                "[Timer]\nOnCalendar=daily\n",
                encoding="utf-8",
            )
            self.assertEqual(
                declared_waivers(scratch),
                {("planted.timer", "calendar-not-persistent"): "a planted why of six words"},
            )
        # Every waiver the templates declare is pinned, and each gives more than five words of why.
        declared = declared_waivers()
        for (unit, reason), why in examined("declared waiver(s)", sorted(declared.items())):
            self.assertGreater(len(why.split()), 5, f"{unit}: {reason}: a thin why")
        self.assertEqual(set(declared), WAIVED)


class TheTemplatesFitTheHostBudget(unittest.TestCase):
    def test_every_unit_ceiling_matches_the_host_budget_and_high_is_below_max(self):
        units = services()
        entries = budget()["units"]
        self.assertEqual(sorted(entries), [unit.name for unit in units], "budget against units")
        decided = adr_budget()
        for unit in units:
            entry = entries[unit.name]
            high = last(unit, "Service", "MemoryHigh")
            ceiling = last(unit, "Service", "MemoryMax")
            self.assertEqual((high, ceiling), (entry["memory_high"], entry["memory_max"]), unit.rel)
            self.assertEqual((high, ceiling), decided.get(unit.name), f"{unit.rel} against ADR-032")
            self.assertLess(
                size(high), size(ceiling), f"{unit.rel}: MemoryHigh must throttle first"
            )

    def test_the_daemons_and_the_largest_job_fit_the_stack_share(self):
        units = services()
        share = budget()["memory"]
        self.assertIn(f'"memory": "{share}"', ADR.read_text(encoding="utf-8"), "ADR-032's share")
        ceilings = {}
        for unit in units:
            value = last(unit, "Service", "MemoryMax")
            self.assertIsNotNone(
                value, f"{unit.rel} has no MemoryMax, so it cannot be shown to fit"
            )
            ceilings[unit.name] = size(value)
        daemons = [u.name for u in units if _units.long_running(u)]
        oneshots = [u.name for u in units if not _units.long_running(u)]
        self.assertEqual(daemons, ["deck-streak-api.service", "deck-streak-bot.service"])
        self.assertEqual(
            oneshots,
            [f"{ALERT_TEMPLATE}@.service", f"{JOB_TEMPLATE}@.service", WATCH_SERVICE, SLO_SERVICE],
        )
        worst = sum(ceilings[unit] for unit in daemons) + max(ceilings[unit] for unit in oneshots)
        # ADR-032's arithmetic: 128 + 96 for the daemons, and the job's 384, still the largest.
        self.assertEqual(worst, size("608M"))
        self.assertLessEqual(worst, size(share), "the worst case exceeds DeckStreak's share")
        # The daemons' CPU quotas fit the share's CPUs.
        quotas = [
            int(last(u, "Service", "CPUQuota").rstrip("%")) for u in units if u.name in daemons
        ]
        self.assertLessEqual(sum(quotas), 100 * budget()["cpus"], quotas)


class TheCaddyBlock(unittest.TestCase):
    def test_the_caddy_policy_admits_telegram_web_and_sends_the_security_headers(self):
        block = site()
        operations = header_operations(block)
        for field, value in HEADERS.items():
            self.assertEqual(operations.get(field), value, f"the block's {field}")
        self.assertIn("-Server", operations, "the Server header is not removed")
        directives = policy(operations["Content-Security-Policy"])
        self.assertEqual(directives["frame-ancestors"], ["https://web.telegram.org"])
        self.assertEqual(directives["object-src"], ["'none'"])
        self.assertEqual(directives["base-uri"], ["'self'"])
        # The script policy is the page's own meta policy, with its build's hashes (SPEC-028 R14):
        # a script-src or default-src here would block the scripts that policy admits.
        self.assertNotIn("script-src", directives)
        self.assertNotIn("default-src", directives)
        config = (REPO / "web" / "app" / "svelte.config.js").read_text(encoding="utf-8")
        self.assertRegex(config, r"'script-src': \['self', 'https://telegram\.org'\]")
        sent = operations["Strict-Transport-Security"].split(";")
        hsts = dict(part.strip().partition("=")[::2] for part in sent)
        self.assertGreaterEqual(int(hsts["max-age"]), ONE_YEAR)
        self.assertIn("includeSubDomains", hsts)
        # The same referrer policy as the page's own meta element, which keeps URLs home.
        page = (REPO / "web" / "app" / "src" / "app.html").read_text(encoding="utf-8")
        meta = re.search(r'<meta name="referrer" content="([a-z-]+)"', page).group(1)
        self.assertEqual(operations.get("Referrer-Policy"), meta)
        self.assertIn(meta, KEEPS_URLS)
        # No switch that weakens the edge: automatic HTTPS stays on, TLS stays at 1.2 or above.
        text = CADDY.read_text(encoding="utf-8")
        for weakening in ("auto_https", "tls internal", "tls1.0", "tls1.1"):
            self.assertNotIn(weakening, text)

    def test_the_caddy_block_serves_the_spa_proxies_the_api_and_hides_health(self):
        block = site()
        self.assertEqual(block.tokens, ["{$DECKSTREAK_HOST}"])
        api = handle(block, "/api/*")
        self.assertIsNotNone(api, "no handle for /api/*")
        self.assertEqual(
            api.one("reverse_proxy").tokens, ["reverse_proxy", "{$DECKSTREAK_API_UPSTREAM}"]
        )
        # The health routes the API serves are answered 404 from outside (ADR-025), inside the API's
        # own handle, where respond runs before reverse_proxy in Caddy's directive order.
        health = [route_constant("LIVEZ"), route_constant("READYZ")]
        matchers = [child for child in api.children if child.tokens[0].startswith("@")]
        self.assertEqual([m.tokens[1:] for m in matchers], [["path", *health]])
        self.assertEqual(api.one("respond").tokens, ["respond", matchers[0].tokens[0], "404"])
        # The SPA: files from the web root, the fallback page for every route the client renders.
        spa = handle(block)
        self.assertIsNotNone(spa, "no fallback handle")
        self.assertEqual(spa.one("root").tokens, ["root", "*", "{$DECKSTREAK_WEB_ROOT}"])
        fallback = re.search(
            r"fallback: '([^']+)'", (REPO / "web" / "app" / "svelte.config.js").read_text()
        ).group(1)
        self.assertEqual(
            spa.one("try_files").tokens, ["try_files", "{path}", "{path}.html", f"/{fallback}"]
        )
        self.assertEqual(spa.one("file_server").tokens, ["file_server"])
        # robots.txt disallows everything, answered before the build's own file is reached.
        robots = handle(block, "/robots.txt")
        self.assertIsNotNone(robots, "no handle for /robots.txt")
        answer = robots.one("respond").tokens
        self.assertEqual(answer[2:], ["200"])
        self.assertEqual(answer[1].splitlines(), ["User-agent: *", "Disallow: /"])
        # Nothing answers outside the handles, so a route cannot slip past the order above.
        self.assertEqual(
            sorted(child.tokens[0] for child in block.children), ["handle"] * 3 + ["header"]
        )
        # The upstream is the API's own loopback listener (ADR-007), which the example names.
        listen = dict((key, value) for _, key, value in env_example())["DECKSTREAK_API_LISTEN"]
        host = listen.rpartition(":")[0].strip("[]")
        self.assertTrue(ipaddress.ip_address(host).is_loopback, listen)


class NoPrivateValue(unittest.TestCase):
    def test_no_deploy_template_names_a_private_value(self):
        files = examined("file(s) under deploy/", [p for p in DEPLOY.rglob("*") if p.is_file()])
        done = scrub(DEPLOY)
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn(f"examined {len(files)} file(s)", done.stdout)
        # A planted template carrying a private-range address is refused by name and line.
        address = ".".join(str(octet) for octet in (10, 24, 36, 48))
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "planted.service"
            planted.write_text(f"[Service]\nEnvironment=DECKSTREAK_API_LISTEN={address}:8080\n")
            done = scrub(scratch)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("planted.service:2: ipv4", done.stdout)
        self.assertNotIn(address, done.stdout)


class NoSecretInTheEnvironment(unittest.TestCase):
    def test_no_unit_passes_a_secret_through_its_environment(self):
        ids = credential_ids()
        # The one settings file every service reads names no secret, only settings a role reads.
        settings = examined("setting(s) in the committed example", env_example())
        declared = declared_settings()
        for number, key, value in settings:
            where = f"{ENV_EXAMPLE.relative_to(REPO)}:{number}"
            self.assertIsNone(_units.SECRET_NAME.search(key), f"{where}: {key} names a secret")
            self.assertNotIn(key, SYSTEMD_SETS, f"{where}: systemd sets {key}")
            self.assertIn(key, declared | {"RUST_LOG"}, f"{where}: no role reads {key}")
        self.assertLessEqual(set(REQUIRED_SETTINGS), {key for _, key, _ in settings})
        units = services()
        for unit in units:
            self.assertEqual(environment_refusals(unit, set(ids.values())), [], unit.rel)
            # Each credential the role reads reaches the unit as a credential instead.
            loaded = {value.partition(":")[0] for value in unit.values("Service", "LoadCredential")}
            wanted = {ids[constant] for constant in ROLE_CREDENTIALS[unit.name]}
            self.assertEqual(loaded, wanted, f"{unit.rel}: the credentials its role reads")
        # A planted unit that passes the bot token through its environment is refused.
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "deploy" / "systemd" / "planted.service"
            planted.parent.mkdir(parents=True)
            token = "".join(("73", "91", "a2c4e6"))
            planted.write_text(
                "[Unit]\nDescription=planted\n\n[Service]\nExecStart=/bin/true\n"
                f"Environment=DECKSTREAK_BOT_TOKEN={token}\n"
            )
            planted_units = subject(scratch).services
        self.assertEqual(
            [environment_refusals(u, set(ids.values())) for u in planted_units],
            [
                [
                    "deploy/systemd/planted.service: DECKSTREAK_BOT_TOKEN passes a secret through the "
                    "environment"
                ]
            ],
        )


class CredentialsComeFromTheSocket(unittest.TestCase):
    def test_every_credential_line_has_the_socket_form_and_encrypted_is_refused(self):
        ids = set(credential_ids().values())
        lines = examined("credential line(s) under deploy/", credential_lines(DEPLOY))
        self.assertEqual(socket_form_refusals(lines, ids), [])
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "planted.service"
            planted.write_text(
                "[Service]\n"
                f"LoadCredentialEncrypted=telegram-bot-token:{SOCKET}\n"
                f"LoadCredential=telegram-bot-token:{SOCKET}\n"
            )
            found = credential_lines(scratch)
        self.assertEqual(len(found), 2)
        self.assertEqual(
            socket_form_refusals(found, ids),
            [
                "planted.service:2: LoadCredentialEncrypted= is refused; use LoadCredential= (ADR-038)"
            ],
        )

    def test_a_credential_key_with_blanks_before_its_equals_sign_is_read_and_refused(self):
        # The reader of credential lines is the unit reader, so a space or a tab before `=` is a key
        # like any other (SPEC-066 R2): each is found, and a form that is not the socket is refused.
        ids = set(credential_ids().values())
        blanks = [(" ", "space"), ("\t", "tab")]
        for blank, what in examined("blank(s) before the equals sign", blanks):
            with tempfile.TemporaryDirectory() as scratch:
                planted = Path(scratch) / "planted.service"
                planted.write_text(
                    "[Service]\n"
                    f"LoadCredentialEncrypted{blank}=telegram-bot-token:{SOCKET}\n"
                    f"LoadCredential{blank}=telegram-bot-token:/etc/token\n"
                )
                found = credential_lines(scratch)
            self.assertEqual(
                socket_form_refusals(found, ids),
                [
                    "planted.service:2: LoadCredentialEncrypted= is refused; use LoadCredential= "
                    "(ADR-038)",
                    "planted.service:3: telegram-bot-token is read from '/etc/token', not the "
                    "socket",
                ],
                what,
            )

    def test_only_a_units_own_dropin_directory_is_shipped_under_deploy(self):
        # The tree ships the drop-in directories of no unit, and one directory of a file that is no
        # unit (SPEC-066 R2). Planted beside a unit: a directory named for no unit, one named for
        # the suffix alone, one named for a template's instance, and one named for a unit that is
        # not shipped: each refused by its path. A unit's own is read, so it is not refused here.
        self.assertEqual(dropin_directory_refusals(REPO), [])
        refused = "is not the drop-in directory of a unit shipped beside it, and is refused"
        planted = [
            "deck-streak-.service.d",
            "service.d",
            "planted@one.service.d",
            ".d",
            "absent.service.d",
        ]
        for folder in examined("planted drop-in directorie(s)", planted):
            with tempfile.TemporaryDirectory() as scratch:
                systemd = Path(scratch) / "deploy" / "systemd"
                (systemd / folder).mkdir(parents=True)
                (systemd / "planted@.service").write_text("[Service]\n", encoding="utf-8")
                (systemd / "planted@.service.d").mkdir()
                self.assertEqual(
                    dropin_directory_refusals(scratch),
                    [f"deploy/systemd/{folder}: {refused}"],
                    folder,
                )
        # A directory of the same name elsewhere under deploy/ is refused too, and the one
        # directory the tree holds is refused when it moves.
        for where in ("deploy/scripts/planted.service.d", "deploy/other/journald.conf.d"):
            with tempfile.TemporaryDirectory() as scratch:
                (Path(scratch) / where).mkdir(parents=True)
                self.assertEqual(dropin_directory_refusals(scratch), [f"{where}: {refused}"], where)


class TheServicesRunTheirRoles(unittest.TestCase):
    def test_every_service_runs_its_role_with_the_lifecycle_r1_names(self):
        for unit in services():
            identifier = unit.name.removesuffix(".service") if "@" not in unit.name else "%N"
            self.assertEqual(last(unit, "Service", "SyslogIdentifier"), identifier, unit.rel)
            if unit.name in SCRIPTS:
                # SPEC-031's units run their script; the alert names no OnFailure=, since a page
                # that fails must not start a page about the page.
                self.assertEqual(
                    unit.values("Service", "ExecStart"), [SCRIPTS[unit.name]], unit.rel
                )
                alert = unit.name == f"{ALERT_TEMPLATE}@.service"
                self.assertEqual(
                    last(unit, "Unit", "OnFailure"), None if alert else ON_FAILURE, unit.rel
                )
                for key, value in OBSERVABILITY_SERVICE[unit.name].items():
                    self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
                self.assertEqual(unit.values("Install", "WantedBy"), [], "a timer or a failure")
                continue
            role = ROLES[unit.name]
            self.assertEqual(unit.values("Service", "ExecStart"), [f"{BINARY} {role}"], unit.rel)
            self.assertEqual(last(unit, "Unit", "OnFailure"), ON_FAILURE, unit.rel)
            if role.startswith("job"):
                for key, value in JOB_SERVICE.items():
                    self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
                self.assertEqual(unit.values("Install", "WantedBy"), [], "the timers start it")
                continue
            for key, value in DAEMON_SERVICE.items():
                self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
            for key, value in DAEMON_UNIT.items():
                self.assertEqual(last(unit, "Unit", key), value, f"{unit.rel} {key}")
            for key, value in DAEMON_CAPS[unit.name].items():
                self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
            self.assertEqual(unit.values("Install", "WantedBy"), ["multi-user.target"], unit.rel)
        timers = examined("timer(s)", subject().timers)
        for timer in timers:
            # A job's timer, or one of SPEC-031's two, which start the evaluator and the watch.
            if timer.name not in OBSERVABILITY_TIMERS:
                self.assertTrue(timer.name.startswith(f"{JOB_TEMPLATE}@"), timer.rel)
            self.assertEqual(timer.values("Install", "WantedBy"), ["timers.target"], timer.rel)
            # R10: a timer starts the service of its own name, which systemd's default Unit= is.
            self.assertIsNone(last(timer, "Timer", "Unit"), timer.rel)
        self.assertLessEqual(OBSERVABILITY_TIMERS, {timer.name for timer in timers})

    def test_every_service_carries_the_hardening_r2_names(self):
        for unit in services():
            for key, value in HARDENING.items():
                self.assertEqual(
                    unit.values("Service", key), [value] if value else [], f"{unit.rel} {key}"
                )
            for key, value in zip(PER_SERVICE_KEYS, PER_SERVICE[unit.name], strict=True):
                self.assertEqual(
                    unit.values("Service", key), [value] if value else [], f"{unit.rel} {key}"
                )
            self.assertTrue(unit.assigned("Service", "CapabilityBoundingSet"), unit.rel)
            self.assertEqual(unit.values("Service", "AmbientCapabilities"), [], unit.rel)


class ARefusedCredentialFailsItsUnitAndPages(unittest.TestCase):
    def test_every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal(self):
        alert = f"{ALERT_TEMPLATE}@.service"
        loading = examined(
            "service unit(s) that load a credential",
            sorted((unit for unit in subject().services if loads_a_credential(unit)), key=name),
        )
        # The roles that read a credential, and the alert template, which reads two (SPEC-031 R3).
        self.assertEqual(
            {unit.name for unit in loading},
            {unit for unit, constants in ROLE_CREDENTIALS.items() if constants},
        )
        paging = [unit for unit in loading if unit.name != alert]
        for unit in paging:
            self.assertIn(ON_FAILURE, unit.values("Unit", "OnFailure"), unit.rel)
        self.assertEqual([r for unit in paging for r in refusal_page_refusals(unit)], [])
        # The alert template is the one exception: it cannot page about itself, so its own refusal
        # is its failed state (SPEC-066 R3; test_alert_unit.py holds that route). Its refused start
        # must still fail it and stay failed: every condition but OnFailure= holds for it, told
        # apart by the directive each refusal reads and never by its text, it restarts none, and it
        # is never unloaded while failed.
        (template,) = [unit for unit in loading if unit.name == alert]
        self.assertEqual(template.values("Unit", "OnFailure"), [])
        self.assertEqual(alert_exit_refusals(template), [])
        self.assertEqual(restart_refusals(template), [])
        self.assertEqual(collect_refusals(template), [])
        # Planted templates: one for each condition, and exit statuses the census does not read;
        # one that meets all five; one that loads no credential and so is not examined; a restart
        # value that is empty or unknown; a [Unit] condition or assertion; and templates shaped as
        # the alert template is, which name no OnFailure=, each breaking one thing the alert
        # template must not: its exit status, a restart mode that skips the failed state beside a
        # restart, a forced restart, a condition, its collection, a success status and a forced
        # restart that name no 1, a forced restart on Type=oneshot, an empty or unknown restart or
        # collect value, and a [Unit] condition or assertion.
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"
        condition = "ExecCondition=/bin/true\n"
        octal = "0o\\ -1777777777777777777777"
        binary = "0b\\ -" + "1" * 64
        unmet = "ConditionPathExists=/nonexistent\n"
        asserted = "AssertPathExists=/nonexistent\n"
        plants = {
            "pages.service": f"{head}{page}{run}{loads}",
            "silent.service": f"{head}{run}{loads}",
            "condition.service": f"{head}{page}{run}{condition}{loads}",
            "ignored.service": f"{head}{page}[Service]\nExecStart=-/bin/true\n{loads}",
            "success.service": f"{head}{page}{run}SuccessExitStatus=2 1\n{loads}",
            "spelled.service": f"{head}{page}{run}SuccessExitStatus=0x1\n{loads}",
            "direct.service": f"{head}{page}{run}Restart=on-failure\nRestartMode=direct\n{loads}",
            "reads-none.service": f"{head}{run}",
            "alert-shaped.service": f"{head}{run}SuccessExitStatus=1\n{loads}",
            "alert-spelled.service": f"{head}{run}SuccessExitStatus=01\n{loads}",
            "alert-restarts.service": f"{head}{run}Restart=on-failure\nRestartMode=direct\n{loads}",
            "alert-forced.service": f"{head}{run}RestartForceExitStatus=1\n{loads}",
            "alert-condition.service": f"{head}{run}{condition}{loads}",
            "alert-collected.service": f"{head}CollectMode=inactive-or-failed\n{run}{loads}",
            "alert-status.service": (
                f"{head}{run}SuccessExitStatus=2\nRestartForceExitStatus=2\n{loads}"
            ),
            "alert-oneshot.service": f"{head}{run}Type=oneshot\nRestartForceExitStatus=2\n{loads}",
            "reset-mode.service": (
                f"{head}{page}{run}Restart=on-failure\nRestartMode=direct\nRestartMode=\n{loads}"
            ),
            "unknown.service": f"{head}{page}{run}Restart=On-Failure\n{loads}",
            "alert-reset-restart.service": (
                f"{head}{run}Restart=on-failure\nRestartSec=1d\nRestart=\n{loads}"
            ),
            "alert-reset-collect.service": (
                f"{head}CollectMode=inactive-or-failed\nCollectMode=\n{run}{loads}"
            ),
            "alert-unknown.service": f"{head}{run}RestartMode=Direct\n{loads}",
            "wide-octal.service": f"{head}{page}{run}SuccessExitStatus={octal}\n{loads}",
            "wide-binary.service": f"{head}{page}{run}SuccessExitStatus={binary}\n{loads}",
            "alert-wide-octal.service": f"{head}{run}SuccessExitStatus={octal}\n{loads}",
            "alert-forced-spelled.service": f"{head}{run}RestartForceExitStatus=0x1\n{loads}",
            "unit-condition.service": f"{head}{unmet}{page}{run}{loads}",
            "unit-condition-reset.service": (
                f"{head}{unmet}ConditionPathExists=\n{page}{run}{loads}"
            ),
            "unit-assert.service": f"{head}{asserted}{page}{run}{loads}",
            "alert-unit-condition.service": f"{head}{unmet}{run}{loads}",
            "alert-unit-assert.service": f"{head}{asserted}{run}{loads}",
        }
        with tempfile.TemporaryDirectory() as scratch:
            folder = Path(scratch) / "deploy" / "systemd"
            folder.mkdir(parents=True)
            for file, content in plants.items():
                (folder / file).write_text(content, encoding="utf-8")
            planted = sorted(subject(scratch).services, key=name)
        planted_loading = [unit for unit in planted if loads_a_credential(unit)]
        self.assertEqual(
            [unit.name for unit in planted_loading],
            [
                "alert-collected.service",
                "alert-condition.service",
                "alert-forced-spelled.service",
                "alert-forced.service",
                "alert-oneshot.service",
                "alert-reset-collect.service",
                "alert-reset-restart.service",
                "alert-restarts.service",
                "alert-shaped.service",
                "alert-spelled.service",
                "alert-status.service",
                "alert-unit-assert.service",
                "alert-unit-condition.service",
                "alert-unknown.service",
                "alert-wide-octal.service",
                "condition.service",
                "direct.service",
                "ignored.service",
                "pages.service",
                "reset-mode.service",
                "silent.service",
                "spelled.service",
                "success.service",
                "unit-assert.service",
                "unit-condition-reset.service",
                "unit-condition.service",
                "unknown.service",
                "wide-binary.service",
                "wide-octal.service",
            ],
        )
        where = "deploy/systemd"
        # The restart mode's refusal: it skips a paging unit's OnFailure=, and the alert template's
        # failed state; and the condition's: a skipped start neither fails the unit nor pages.
        direct_mode = "RestartMode=direct skips the failed state and OnFailure="
        skip = (
            "ExecCondition=/bin/true can skip the start, which neither fails the unit nor starts "
            "OnFailure="
        )
        # And a `Restart=`, `RestartMode=` or `CollectMode=` that is empty or not a known value, and
        # an exit-status word the census does not read.
        word = "which is neither a decimal of at most 255 nor a status name, and is refused"
        # And every [Unit] condition and assertion, an empty one included.
        stops = (
            "is refused, as every condition and assertion is, since one can stop the start without "
            "failing the unit or starting OnFailure="
        )
        unread = "is empty or not a known value, which the check refuses"
        self.assertEqual(
            [r for unit in planted_loading for r in refusal_page_refusals(unit)],
            [
                f"{where}/alert-collected.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-condition.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-condition.service: {skip}",
                f"{where}/alert-forced-spelled.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-forced.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-oneshot.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-reset-collect.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-reset-collect.service: CollectMode= {unread}",
                f"{where}/alert-reset-restart.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-reset-restart.service: Restart= {unread}",
                f"{where}/alert-restarts.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-restarts.service: {direct_mode}",
                f"{where}/alert-shaped.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-shaped.service: SuccessExitStatus=1 counts the refusal a success",
                f"{where}/alert-spelled.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-spelled.service: SuccessExitStatus=01 holds 01, {word}",
                f"{where}/alert-status.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-unit-assert.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-unit-assert.service: AssertPathExists=/nonexistent {stops}",
                f"{where}/alert-unit-condition.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-unit-condition.service: ConditionPathExists=/nonexistent {stops}",
                f"{where}/alert-unknown.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-unknown.service: RestartMode=Direct {unread}",
                f"{where}/alert-wide-octal.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-wide-octal.service: SuccessExitStatus={octal} holds 0o\\, {word}",
                f"{where}/alert-wide-octal.service: SuccessExitStatus={octal} holds "
                f"{octal[4:]}, {word}",
                f"{where}/condition.service: {skip}",
                f"{where}/direct.service: {direct_mode}",
                f"{where}/ignored.service: ExecStart=-/bin/true counts a failure as a success",
                f"{where}/reset-mode.service: {direct_mode}",
                f"{where}/reset-mode.service: RestartMode= {unread}",
                f"{where}/silent.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/spelled.service: SuccessExitStatus=0x1 holds 0x1, {word}",
                f"{where}/success.service: SuccessExitStatus=2 1 counts the refusal a success",
                f"{where}/unit-assert.service: AssertPathExists=/nonexistent {stops}",
                f"{where}/unit-condition-reset.service: ConditionPathExists=/nonexistent {stops}",
                f"{where}/unit-condition-reset.service: ConditionPathExists= {stops}",
                f"{where}/unit-condition.service: ConditionPathExists=/nonexistent {stops}",
                f"{where}/unknown.service: Restart=On-Failure {unread}",
                f"{where}/wide-binary.service: SuccessExitStatus={binary} holds 0b\\, {word}",
                f"{where}/wide-binary.service: SuccessExitStatus={binary} holds "
                f"{binary[4:]}, {word}",
                f"{where}/wide-octal.service: SuccessExitStatus={octal} holds 0o\\, {word}",
                f"{where}/wide-octal.service: SuccessExitStatus={octal} holds {octal[4:]}, {word}",
            ],
        )
        # The alert template's own checks refuse each alert-shaped plant for what it breaks alone:
        # the exit conditions for its exit status, its restart mode or its condition, the restart
        # check for its restart, and the collection check for its collection. `alert-restarts` is
        # refused by the exit conditions with exactly its RestartMode= line, though that line, like
        # its OnFailure= one, names OnFailure=. A success status and a forced restart that name no
        # 1 are refused too, since the alert template names neither; and on Type=oneshot the
        # service manager refuses a forced restart outright.
        shaped = [unit for unit in planted_loading if unit.name.startswith("alert-")]
        counts = "counts the refusal a success"
        named = "is named, and the alert template names none"
        oneshot = "makes the service manager refuse the Type=oneshot unit outright"
        unloads = "can unload the failed instance, which systemctl --failed then no longer lists"
        refused = {
            "alert-collected.service": (
                [],
                [],
                [f"{where}/alert-collected.service: CollectMode=inactive-or-failed {unloads}"],
            ),
            "alert-condition.service": ([f"{where}/alert-condition.service: {skip}"], [], []),
            "alert-forced-spelled.service": (
                [],
                [
                    f"{where}/alert-forced-spelled.service: RestartForceExitStatus=0x1 holds 0x1, "
                    f"{word}"
                ],
                [],
            ),
            "alert-forced.service": (
                [],
                [f"{where}/alert-forced.service: RestartForceExitStatus=1 restarts the refusal"],
                [],
            ),
            "alert-oneshot.service": (
                [],
                [f"{where}/alert-oneshot.service: RestartForceExitStatus=2 {oneshot}"],
                [],
            ),
            "alert-reset-collect.service": (
                [f"{where}/alert-reset-collect.service: CollectMode= {unread}"],
                [],
                [f"{where}/alert-reset-collect.service: CollectMode=inactive-or-failed {unloads}"],
            ),
            "alert-reset-restart.service": (
                [f"{where}/alert-reset-restart.service: Restart= {unread}"],
                [f"{where}/alert-reset-restart.service: Restart=on-failure restarts the refusal"],
                [],
            ),
            "alert-restarts.service": (
                [f"{where}/alert-restarts.service: {direct_mode}"],
                [f"{where}/alert-restarts.service: Restart=on-failure restarts the refusal"],
                [],
            ),
            "alert-shaped.service": (
                [f"{where}/alert-shaped.service: SuccessExitStatus=1 {counts}"],
                [],
                [],
            ),
            "alert-spelled.service": (
                [f"{where}/alert-spelled.service: SuccessExitStatus=01 holds 01, {word}"],
                [],
                [],
            ),
            "alert-status.service": (
                [f"{where}/alert-status.service: SuccessExitStatus=2 {named}"],
                [f"{where}/alert-status.service: RestartForceExitStatus=2 {named}"],
                [],
            ),
            "alert-unit-assert.service": (
                [f"{where}/alert-unit-assert.service: AssertPathExists=/nonexistent {stops}"],
                [],
                [],
            ),
            "alert-unit-condition.service": (
                [f"{where}/alert-unit-condition.service: ConditionPathExists=/nonexistent {stops}"],
                [],
                [],
            ),
            "alert-unknown.service": (
                [f"{where}/alert-unknown.service: RestartMode=Direct {unread}"],
                [],
                [],
            ),
            "alert-wide-octal.service": (
                [
                    f"{where}/alert-wide-octal.service: SuccessExitStatus={octal} holds "
                    f"0o\\, {word}",
                    f"{where}/alert-wide-octal.service: SuccessExitStatus={octal} holds "
                    f"{octal[4:]}, {word}",
                ],
                [],
                [],
            ),
        }
        self.assertEqual(
            {
                unit.name: (
                    alert_exit_refusals(unit),
                    restart_refusals(unit),
                    collect_refusals(unit),
                )
                for unit in shaped
            },
            refused,
        )
        # The census reads an exit-status word only as a decimal of at most 255, with no sign and
        # no leading zero, or as a status name `systemd-analyze exit-status` lists, and refuses
        # every other word: each word below has the reading given, None where the census refuses
        # it. A value splits into words on spaces and tabs alone, and a backslash or a quote stays
        # in its word, which the census then refuses.
        readings = [
            ("1 FAILURE", 1),
            ("0 SUCCESS", 0),
            ("2 INVALIDARGUMENT", 2),
            ("78 CONFIG", 78),
            ("200 CHDIR", 200),
            ("255 EXCEPTION", 255),
            (
                "01 0001 0x1 0X01 +1 +0x1 0b1 0B1 0o1 0O1 010 00 -0 0x0 -1 256 1000 08 1.0 1e0 "
                '"1" \\1 failure Failure SIGKILL KILL \u0661 \u00b9 \uff11 1\u0661 1\uff10 0x 0b '
                "+ -",
                None,
            ),
        ]
        for words, reading in readings:
            for status in words.split(" "):
                self.assertEqual(_units.exit_status(status), reading, status)
        words = _units.status_words('\\1 F\\AILURE 0\\ 1 "1"\t2  3\\')
        self.assertEqual(words, ["\\1", "F\\AILURE", "0\\", "1", '"1"', "2", "3\\"])
        self.assertEqual(
            [_units.exit_status(w) for w in words], [None, None, None, 1, None, 2, None]
        )
        # A cross-check corpus, refused whole: each word planted as the SuccessExitStatus= of a
        # paging unit and of an alert-shaped one, and as the RestartForceExitStatus= of an
        # alert-shaped one, as written and with each space or tab after a backslash, is refused by
        # the reader or by the census. The words: small and large magnitudes in every base, with
        # prefixes, signs and whitespace around them.
        magnitudes = [
            "1",
            "01",
            "256",
            str(2**32 + 1),
            str(2**64 - 2),
            str(2**64 - 1),
            "100000001",
            "fffffffffffffffe",
            "ffffffffffffffff",
            "1" * 63 + "0",
            "1" * 64,
            "1777777777777777777776",
            "1777777777777777777777",
        ]
        corpus = [
            f"{lead}{prefix}{gap}{sign}{magnitude}"
            for lead in ("", "\x0b", "\x0c")
            for prefix in ("", "+", "-", "0x", "0X", "0b", "0B", "0o", "0O")
            for gap in ("", " ", "\t", "\x0b", "\x0c")
            for sign in ("", "+", "-")
            for magnitude in magnitudes
        ]
        corpus += [
            status.replace(" ", "\\ ").replace("\t", "\\\t")
            for status in corpus
            if " " in status or "\t" in status
        ]
        shapes = [
            (f"{head}{page}{run}SuccessExitStatus=", refusal_page_refusals),
            (f"{head}{run}SuccessExitStatus=", alert_exit_refusals),
            (f"{head}{run}RestartForceExitStatus=", restart_refusals),
        ]
        cases = [(shape, check, status) for shape, check in shapes for status in corpus]
        admitted = [
            (shape, status)
            for shape, check, status in examined("cross-check plant(s)", cases)
            if not planted_refusals(f"{shape}{status}\n{loads}", check)
        ]
        self.assertEqual(admitted, [])
        # The reader refuses a line outside the plain syntax the templates hold, one planted
        # template at a time, naming the file and the line: a line that ends in a backslash, a
        # comment's included; a control character other than tab and newline, or whitespace outside
        # ASCII; and a line that is neither blank, a comment, a section header nor an assignment
        # inside a section. A tab, and a `§` in a comment, are read.
        backslash = "ends in a backslash, which the reader refuses"
        character = "a character the reader refuses"
        shape = "is neither a section header nor an assignment in a section"
        misread = {
            "continued-comment.service": (
                f"{head}{page}{run}# a note \\\nExecCondition=/bin/true\n{loads}",
                f"6: {backslash}",
            ),
            "escaped-backslash.service": (
                f"{head}{page}{run}X-Note=kept \\\\\nExecCondition=/bin/true\n{loads}",
                f"6: {backslash}",
            ),
            "alert-continued.service": (
                f"{head}{run}SuccessExitStatus=2 \\\n1\n{loads}",
                f"5: {backslash}",
            ),
            "spaced-section.service": (
                f"[ Unit ]\nDescription=planted\n{page}{run}{loads}",
                f"1: {shape}",
            ),
            "no-equals.service": (
                f"{head}{page}{run}ExecCondition /bin/true\n{loads}",
                f"6: {shape}",
            ),
            "bare-key.service": (f"{head}{page}{run}ExecCondition\n{loads}", f"6: {shape}"),
            "spaced-key.service": (
                f"{head}{page}{run}Exec Condition=/bin/true\n{loads}",
                f"6: {shape}",
            ),
            "no-section.service": (f"{page}{head}{run}{loads}", f"1: {shape}"),
        }
        for char in "\x0b\x0c\r\x00\x1c\x1d\x1e\x1f\x7f\x85\u00a0\u2009\u2028\u2029\u3000":
            misread[f"char-{ord(char):04x}.service"] = (
                f"{head}{page}{run}SuccessExitStatus={char}1 X-Y=z\n{loads}",
                f"6: holds U+{ord(char):04X}, {character}",
            )
        misread["alert-char-000b.service"] = (
            f"{head}{run}SuccessExitStatus=\x0b1 X-Y=z\n{loads}",
            f"5: holds U+000B, {character}",
        )
        cases = examined("planted template(s) the reader refuses", sorted(misread))
        self.assertEqual(
            {file: reader_refusal({file: misread[file][0]}) for file in cases},
            {file: f"deploy/systemd/{file}:{misread[file][1]}" for file in cases},
        )
        read = f"{head}# a note, \u00a7 3\n{page}{run}\t{loads}"
        self.assertIsNone(reader_refusal({"read.service": read}))

    def test_a_key_off_its_units_list_is_refused_and_the_trees_units_hold_only_listed_keys(self):
        # Each unit that loads a credential holds only the keys its kind's list names, in the
        # section the list gives them; the alert template's list is its own (SPEC-066 R2, R3).
        alert = f"{ALERT_TEMPLATE}@.service"
        loading = examined(
            "service unit(s) that load a credential",
            sorted((unit for unit in subject().services if loads_a_credential(unit)), key=name),
        )
        for unit in loading:
            allowed = _units.ALERT_KEYS if unit.name == alert else _units.PAGING_KEYS
            self.assertEqual(off_list_refusals(unit, allowed), [], unit.rel)
        # Planted units: each key below is off its unit's list and refused by name and line. The
        # directives that make a start depend on another unit, on the alert template and on a
        # paging unit; a key with no standard meaning; `OnFailure=` on the alert template, whose
        # list never holds it; and a key the list holds in another section. A control of each
        # kind holds only listed keys and is admitted.
        where = "deploy/systemd/planted.service"
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"

        def refusal(line, section, key, value):
            return (
                f"{where}:{line}: [{section}] {key}={value} is not on this unit's list of keys, "
                "and is refused"
            )

        # Each plant: the list, the unit's kind, a line planted at the end of `[Unit]`, and one
        # planted at the end of `[Service]`, and the refusals it draws.
        plants = {
            "alert control": (_units.ALERT_KEYS, "", "", "", []),
            "paging control": (_units.PAGING_KEYS, page, "", "", []),
            "alert requisite": (
                _units.ALERT_KEYS,
                "",
                "Requisite=missing.service\n",
                "",
                [refusal(3, "Unit", "Requisite", "missing.service")],
            ),
            "alert requires": (
                _units.ALERT_KEYS,
                "",
                "Requires=missing.service\n",
                "",
                [refusal(3, "Unit", "Requires", "missing.service")],
            ),
            "alert binds-to": (
                _units.ALERT_KEYS,
                "",
                "BindsTo=missing.service\n",
                "",
                [refusal(3, "Unit", "BindsTo", "missing.service")],
            ),
            "paging requisite": (
                _units.PAGING_KEYS,
                page,
                "Requisite=missing.service\n",
                "",
                [refusal(3, "Unit", "Requisite", "missing.service")],
            ),
            "alert extension key": (
                _units.ALERT_KEYS,
                "",
                "X-Note=kept\n",
                "",
                [refusal(3, "Unit", "X-Note", "kept")],
            ),
            "paging extension key": (
                _units.PAGING_KEYS,
                page,
                "",
                "X-Note=kept\n",
                [refusal(7, "Service", "X-Note", "kept")],
            ),
            "alert on-failure": (
                _units.ALERT_KEYS,
                "",
                page,
                "",
                [refusal(3, "Unit", "OnFailure", ON_FAILURE)],
            ),
            "alert listed key, wrong section": (
                _units.ALERT_KEYS,
                "",
                "User=nobody\n",
                "",
                [refusal(3, "Unit", "User", "nobody")],
            ),
            "paging listed key, wrong section": (
                _units.PAGING_KEYS,
                page,
                "",
                "Wants=network-online.target\n",
                [refusal(7, "Service", "Wants", "network-online.target")],
            ),
        }
        got = {}
        for label in examined("planted unit(s) held to a list", sorted(plants)):
            allowed, first, unit_line, service_line, _ = plants[label]
            text = f"{head}{unit_line}{first}{run}{loads}{service_line}"
            got[label] = planted_refusals(
                text, lambda unit, allowed=allowed: off_list_refusals(unit, allowed)
            )
        self.assertEqual(got, {label: plants[label][4] for label in plants})
        # A drop-in of a unit is read with it: a directive planted in one is refused by the drop-in's
        # file and line.
        with tempfile.TemporaryDirectory() as scratch:
            folder = Path(scratch) / "deploy" / "systemd"
            (folder / "planted.service.d").mkdir(parents=True)
            (folder / "planted.service").write_text(f"{head}{run}{loads}", encoding="utf-8")
            (folder / "planted.service.d" / "10-planted.conf").write_text(
                "[Unit]\nRequires=missing.service\n", encoding="utf-8"
            )
            (planted,) = subject(scratch).services
        self.assertEqual(
            off_list_refusals(planted, _units.ALERT_KEYS),
            [
                "deploy/systemd/planted.service.d/10-planted.conf:2: [Unit] Requires="
                "missing.service is not on this unit's list of keys, and is refused"
            ],
        )


if __name__ == "__main__":
    unittest.main()

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
# The exit of a role that refuses start (SPEC-025 R1) and of the runner's page (SPEC-027 R7), by
# number and by the name systemd.exec(5) gives it, EXIT_FAILURE: no template counts it a success.
REFUSAL_EXIT = {"1", "FAILURE"}
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
# answers them for the `sync` instance alone (ADR-038). SPEC-031's alert reads the bot token and the
# owner's id, whose private chat it pages (R3); the evaluator and the watch read none.
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
    """Every unit under `root`'s deploy/, parsed as systemd reads it."""
    return _units.load_subject(Path(root))


def services(root=REPO):
    return examined("service unit(s) under deploy/", sorted(subject(root).services, key=name))


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
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            key, equals, value = line.strip().partition("=")
            if equals and key in keys:
                found.append((path.relative_to(root).as_posix(), number, key, value))
    return found


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


def refusal_page_conditions(unit):
    """Why a start of `unit` that a credential refuses would not fail it and start its page
    (SPEC-066 R2), each refusal beside the directive it reads: no `OnFailure=` naming the alert
    template, an `ExecStart=` whose failure counts as a success, a success exit that holds the
    refusal's, or a restart that skips the failed state, and with it `OnFailure=`."""
    refused = []

    def refuse(directive, why):
        refused.append((directive, f"{unit.rel}: {why}"))

    targets = [word for value in unit.values("Unit", "OnFailure") for word in value.split()]
    if ON_FAILURE not in targets:
        refuse("OnFailure", f"OnFailure={' '.join(targets)} does not name {ON_FAILURE}")
    for command in unit.values("Service", "ExecStart"):
        if "-" in EXEC_PREFIX.match(command).group(0):
            refuse("ExecStart", f"ExecStart={command} counts a failure as a success")
    for statuses in unit.values("Service", "SuccessExitStatus"):
        if REFUSAL_EXIT & set(statuses.split()):
            refuse(
                "SuccessExitStatus", f"SuccessExitStatus={statuses} counts the refusal a success"
            )
    if last(unit, "Service", "RestartMode") == "direct":
        refuse("RestartMode", "RestartMode=direct skips the failed state and OnFailure=")
    return refused


def refusal_page_refusals(unit):
    """The refusals of `refusal_page_conditions`, each without the directive it reads."""
    return [refusal for _, refusal in refusal_page_conditions(unit)]


def alert_exit_refusals(unit):
    """Why a refused start of the alert template would count as a success or skip its failed state
    (SPEC-066 R3): each of R2's conditions but `OnFailure=`, which the alert template must not
    name. They are told apart by the directive each refusal reads, never by the refusal's text,
    since the restart mode's refusal names `OnFailure=` too."""
    conditions = refusal_page_conditions(unit)
    return [refusal for directive, refusal in conditions if directive != "OnFailure"]


def restart_refusals(unit):
    """Why a refused start of the alert template would not stay failed (SPEC-066 R3): a restart of
    it. At the default `RestartMode=` a restart only passes through the failed state, and the
    instance waits for its next start activating, so a loop of restarts settles failed only when
    its start limit ends it (systemd.service(5), SPEC-031). So no `Restart=` other than `no`, the
    default an empty assignment restores, and no `RestartForceExitStatus=` naming the refusal's
    exit, which forces a restart whatever `Restart=` says. R2's units may restart: each failure a
    restart passes through still starts their `OnFailure=` page (SPEC-031)."""
    refused = []
    restart = last(unit, "Service", "Restart")
    if restart not in (None, "", "no"):
        refused.append(f"{unit.rel}: Restart={restart} restarts the refusal")
    for statuses in unit.values("Service", "RestartForceExitStatus"):
        if REFUSAL_EXIT & set(statuses.split()):
            refused.append(f"{unit.rel}: RestartForceExitStatus={statuses} restarts the refusal")
    return refused


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
        # apart by the directive each refusal reads and never by its text, and it restarts none.
        (template,) = [unit for unit in loading if unit.name == alert]
        self.assertEqual(template.values("Unit", "OnFailure"), [])
        self.assertEqual(alert_exit_refusals(template), [])
        self.assertEqual(restart_refusals(template), [])
        # Planted templates: one for each condition, one that meets all four, one that loads no
        # credential and so is not examined, and three shaped as the alert template is, which name
        # no OnFailure=, each breaking one thing the alert template must not: its exit status, a
        # restart mode that skips the failed state beside a restart, and a forced restart.
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"
        plants = {
            "pages.service": f"{head}{page}{run}{loads}",
            "silent.service": f"{head}{run}{loads}",
            "ignored.service": f"{head}{page}[Service]\nExecStart=-/bin/true\n{loads}",
            "success.service": f"{head}{page}{run}SuccessExitStatus=2 1\n{loads}",
            "direct.service": f"{head}{page}{run}Restart=on-failure\nRestartMode=direct\n{loads}",
            "reads-none.service": f"{head}{run}",
            "alert-shaped.service": f"{head}{run}SuccessExitStatus=1\n{loads}",
            "alert-restarts.service": f"{head}{run}Restart=on-failure\nRestartMode=direct\n{loads}",
            "alert-forced.service": f"{head}{run}RestartForceExitStatus=1\n{loads}",
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
                "alert-forced.service",
                "alert-restarts.service",
                "alert-shaped.service",
                "direct.service",
                "ignored.service",
                "pages.service",
                "silent.service",
                "success.service",
            ],
        )
        where = "deploy/systemd"
        # The restart mode's refusal: it skips a paging unit's OnFailure=, and the alert template's
        # failed state.
        direct_mode = "RestartMode=direct skips the failed state and OnFailure="
        self.assertEqual(
            [r for unit in planted_loading for r in refusal_page_refusals(unit)],
            [
                f"{where}/alert-forced.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-restarts.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-restarts.service: {direct_mode}",
                f"{where}/alert-shaped.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/alert-shaped.service: SuccessExitStatus=1 counts the refusal a success",
                f"{where}/direct.service: {direct_mode}",
                f"{where}/ignored.service: ExecStart=-/bin/true counts a failure as a success",
                f"{where}/silent.service: OnFailure= does not name {ON_FAILURE}",
                f"{where}/success.service: SuccessExitStatus=2 1 counts the refusal a success",
            ],
        )
        # The alert template's own checks refuse each alert-shaped plant for what it breaks alone:
        # the exit conditions for its exit status or its restart mode, and the restart check for
        # its restart. `alert-restarts` is refused by the exit conditions with exactly its
        # RestartMode= line, though that line, like its OnFailure= one, names OnFailure=.
        shaped = [unit for unit in planted_loading if unit.name.startswith("alert-")]
        refused = {
            "alert-forced.service": (
                [],
                [f"{where}/alert-forced.service: RestartForceExitStatus=1 restarts the refusal"],
            ),
            "alert-restarts.service": (
                [f"{where}/alert-restarts.service: {direct_mode}"],
                [f"{where}/alert-restarts.service: Restart=on-failure restarts the refusal"],
            ),
            "alert-shaped.service": (
                [f"{where}/alert-shaped.service: SuccessExitStatus=1 counts the refusal a success"],
                [],
            ),
        }
        self.assertEqual(
            {unit.name: (alert_exit_refusals(unit), restart_refusals(unit)) for unit in shaped},
            refused,
        )


if __name__ == "__main__":
    unittest.main()

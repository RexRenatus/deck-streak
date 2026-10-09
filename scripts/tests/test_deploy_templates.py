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

import fnmatch
import ipaddress
import json
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path, PurePosixPath

import _units
from _support import REPO, examined

DEPLOY = REPO / "deploy"
SYSTEMD = DEPLOY / "systemd"
CADDY = DEPLOY / "caddy" / "deck-streak.caddy"
BUDGET = DEPLOY / "host-budget.json"
ENV_EXAMPLE = DEPLOY / "deck-streak.env.example"
SCRUB = REPO / "scripts" / "public-scrub.py"
ADR = REPO / "docs" / "decisions" / "ADR-032-deploy-templates-and-the-host-budget.md"
LITESTREAM_CONFIG = DEPLOY / "litestream.yml"
DAILY_COPY = DEPLOY / "scripts" / "backup.py"
# SPEC-083 R34 (ADR-321 D4, D17): the take's backup of the owner's collection, and its partial while
# the restore check runs, sit beside the private copy in the directory systemd gives the units
# (StateDirectory=deck-streak), named for the skip. They never leave the host and are no standing
# copy, so no Litestream database or directory and no daily copy's name may reach them.
STATE_DIRECTORY = "/var/lib/deck-streak"
SKIP_BACKUP_NAMES = ("skip-backup-1.anki2", "skip-backup-1.anki2.partial")
# The daily copy's two names: the one file it copies, and the pattern of the copies it keeps and
# prunes (SPEC-064 R9).
DAILY_COPY_NAMES = (
    ("DATABASE_NAME", re.compile(r'^DATABASE_NAME = "([^"]+)"$', re.M)),
    ("COPY_NAME", re.compile(r'^COPY_NAME = re\.compile\(r"([^"]+)"\)$', re.M)),
)
# SPEC-064's units: the replicator's, the daily backup's and the drill's ceilings are decided here,
# and the share it raised (ADR-032 keeps a dated note).
ADR_BACKUPS = (
    REPO
    / "docs"
    / "decisions"
    / "ADR-064-deckstreak-backs-up-with-its-own-units-and-never-the-collection.md"
)
LITESTREAM_SERVICE_NAME = "deck-streak-litestream.service"
BACKUP_SERVICE_NAME = "deck-streak-backup.service"
DRILL_SERVICE_NAME = "deck-streak-restore-drill.service"
# SPEC-337: the sync server, its launcher, and the settings file's one key the launcher reads.
SYNC_SERVER_SERVICE_NAME = "deck-streak-sync-server.service"
# The stopped-server window that copies the sync server's store for the snapshot (ADR-347 D12).
SYNC_SNAPSHOT_SERVICE_NAME = "deck-streak-sync-snapshot.service"
# SPEC-340 R3 and R4: the snapshot's archive and its drill, each a unit of its own (ADR-351 D1).
SYNC_ARCHIVE_SERVICE_NAME = "deck-streak-sync-archive.service"
SYNC_DRILL_SERVICE_NAME = "deck-streak-sync-restore-drill.service"
# SPEC-340 R2: the sync family runs as its own system user and group, every other service as
# DeckStreak's (ADR-351 D1).
SERVICE_USER = "deck-streak"
SYNC_USER = "deck-streak-sync"
SYNC_FAMILY = (
    SYNC_SERVER_SERVICE_NAME,
    SYNC_SNAPSHOT_SERVICE_NAME,
    SYNC_ARCHIVE_SERVICE_NAME,
    SYNC_DRILL_SERVICE_NAME,
)
# The two state directories only the sync family names.
SYNC_STATE_DIRECTORIES = {"deck-streak-sync-server", "deck-streak-sync-snapshots"}
SYNC_LAUNCHER = DEPLOY / "scripts" / "sync-server.sh"
# The variables the sync server reads its users from, `name:<hash>` each: only its launcher sets
# them, from the unit's credentials, so no unit and no settings line may (ADR-347 D2).
SYNC_SERVER_USERS = re.compile(r"SYNC_USER[0-9]*")
# A password hash in the server's form, which nothing in the tree may hold, real or placeholder.
PHC_HASH = re.compile(
    r"\$pbkdf2-sha256\$i=[0-9]+(?:,l=[0-9]+)?\$[A-Za-z0-9+/]{8,}\$[A-Za-z0-9+/]{16,}"
)
# SPEC-337 section 1: the server's peak resident memory during one full upload of ADR-022's
# synthetic collection, in bytes, which the unit's MemoryHigh= must hold.
SYNC_SERVER_PEAK = 326_600 * 1024

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
# SPEC-396: the second route, a oneshot a timer starts that tells the owner of the alert sender.
SECOND_ROUTE_SERVICE = "deck-streak-second-route.service"
SECOND_ROUTE_SCRIPT = DEPLOY / "scripts" / "second-route.sh"
SCRIPTS = {
    f"{ALERT_TEMPLATE}@.service": f"{RELEASE}/deploy/scripts/alert-telegram.sh %i",
    SLO_SERVICE: (
        f"/usr/bin/python3 {RELEASE}/deploy/scripts/slo-evaluate.py {RELEASE}/deploy/slo.json"
    ),
    WATCH_SERVICE: f"{RELEASE}/deploy/scripts/memory-watch.sh",
    SECOND_ROUTE_SERVICE: f"{RELEASE}/deploy/scripts/second-route.sh",
    # SPEC-064: the daily backup and the weekly drill run scripts of the release as well.
    BACKUP_SERVICE_NAME: f"/usr/bin/python3 {RELEASE}/deploy/scripts/backup.py",
    DRILL_SERVICE_NAME: f"{RELEASE}/deploy/scripts/restore-drill.sh",
    # SPEC-337: the snapshot's window runs the backup script's copy alone.
    SYNC_SNAPSHOT_SERVICE_NAME: f"/usr/bin/python3 {RELEASE}/deploy/scripts/backup.py --sync-window",
    # SPEC-340 R3, R4: the archive and the sync drill run the same scripts' own parts.
    SYNC_ARCHIVE_SERVICE_NAME: f"/usr/bin/python3 {RELEASE}/deploy/scripts/backup.py --sync-archive",
    SYNC_DRILL_SERVICE_NAME: f"{RELEASE}/deploy/scripts/restore-drill.sh --part sync",
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
    SECOND_ROUTE_SERVICE: {
        "Type": "oneshot",
        "TimeoutStartSec": "3min",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
    BACKUP_SERVICE_NAME: {
        "Type": "oneshot",
        "TimeoutStartSec": "30min",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
    DRILL_SERVICE_NAME: {
        "Type": "oneshot",
        "TimeoutStartSec": "1h",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
    # The window runs with the sync server stopped, so it is bounded and does not yield (ADR-347
    # D12); no timer starts it, so resources.batch-priority does not reach it and it waives nothing.
    SYNC_SNAPSHOT_SERVICE_NAME: {"Type": "oneshot", "TimeoutStartSec": "15min"},
    # The archive and the sync drill yield as the backup and the drill they follow do (SPEC-340).
    SYNC_ARCHIVE_SERVICE_NAME: {
        "Type": "oneshot",
        "TimeoutStartSec": "30min",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
    SYNC_DRILL_SERVICE_NAME: {
        "Type": "oneshot",
        "TimeoutStartSec": "1h",
        "Nice": "10",
        "IOSchedulingClass": "idle",
    },
}
# SPEC-064 R1: the Litestream daemon, an exec service that restarts on failure and can trip its
# start limit; it has no watchdog, since Litestream does not notify systemd.
LITESTREAM_SERVICE = {
    "Type": "exec",
    "Restart": "on-failure",
    "RestartSec": "15",
    "OOMPolicy": "kill",
    "TimeoutStopSec": "30",
    "CPUQuota": "15%",
}
# SPEC-337 R2 (ADR-347 D3): the sync server runs its launcher, is active once it runs (it sends no
# readiness), stops on an interrupt, the one signal it drains on, and holds three quarters of a
# processor (ADR-347 D7).
SYNC_SERVER_SERVICE = {
    "Type": "exec",
    "KillSignal": "SIGINT",
    "Restart": "on-failure",
    "RestartSec": "15",
    "OOMPolicy": "kill",
    "TimeoutStopSec": "30",
    "CPUQuota": "75%",
    "TasksMax": "64",
}
# The timers that start SPEC-031's units, each the service of its own name.
OBSERVABILITY_TIMERS = {
    "deck-streak-slo.timer",
    "deck-streak-memory-watch.timer",
    "deck-streak-second-route.timer",
    # SPEC-064's daily backup and weekly restore drill.
    "deck-streak-backup.timer",
    "deck-streak-restore-drill.timer",
}

# Where the code declares each credential id a role reads, by the constant's name.
CREDENTIAL_SOURCES = {
    "OWNER_USER_ID": REPO / "crates" / "identity" / "src" / "owner.rs",
    "TELEGRAM_BOT_TOKEN": REPO / "crates" / "identity" / "src" / "owner.rs",
    "SYNC_USERNAME": REPO / "crates" / "ingest" / "src" / "settings.rs",
    "SYNC_PASSWORD": REPO / "crates" / "ingest" / "src" / "settings.rs",
    "CORE_CREDENTIAL": REPO / "crates" / "mcp" / "src" / "settings.rs",
    "LAW_TRACK_CREDENTIAL": REPO / "crates" / "mcp" / "src" / "settings.rs",
    # The sync server's two users are read by its launcher, the one program that reads them, where
    # each id is a shell constant (SPEC-337 R2; ADR-347 D2).
    "SYNC_SERVER_OWNER": SYNC_LAUNCHER,
    "SYNC_SERVER_STAGING": SYNC_LAUNCHER,
    # The second route's two addresses are shell constants of its script (SPEC-396 R3).
    "SECOND_ROUTE_CHECK_IN": SECOND_ROUTE_SCRIPT,
    "SECOND_ROUTE_REPORT": SECOND_ROUTE_SCRIPT,
}
# Which credentials each service's role reads: the api's owner gate (SPEC-024, SPEC-025), the bot's
# transport, owner gate and `/sync` (SPEC-026 R1, R11), and the `sync` job's syncer (SPEC-022,
# SPEC-027). The job template's `sync` instance carries the sync's pair in its drop-in, which the
# reader reads with the template (SPEC-062 R14), and the private rail's map answers them for the
# `sync` instance alone, the one job that reads them (ADR-038; SPEC-061 §8, A14). The `held_flush`
# instance's drop-in carries the bot token and the owner's id, which its router sends with (#291),
# so the template's reading is the four. SPEC-031's alert
# reads the bot token and the owner's id, whose private chat it pages (R3);
# the evaluator and the watch read none. The MCP server's guard reads its core token and its
# law-track token (SPEC-119 R6; ADR-332).
ROLE_CREDENTIALS = {
    "deck-streak-api.service": ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"),
    "deck-streak-bot.service": ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"),
    "deck-streak-mcp.service": ("CORE_CREDENTIAL", "LAW_TRACK_CREDENTIAL"),
    f"{JOB_TEMPLATE}@.service": (
        "SYNC_USERNAME",
        "SYNC_PASSWORD",
        "OWNER_USER_ID",
        "TELEGRAM_BOT_TOKEN",
    ),
    f"{ALERT_TEMPLATE}@.service": ("OWNER_USER_ID", "TELEGRAM_BOT_TOKEN"),
    SLO_SERVICE: (),
    WATCH_SERVICE: (),
    SECOND_ROUTE_SERVICE: ("SECOND_ROUTE_CHECK_IN", "SECOND_ROUTE_REPORT"),
    LITESTREAM_SERVICE_NAME: (),
    BACKUP_SERVICE_NAME: (),
    DRILL_SERVICE_NAME: (),
    SYNC_SNAPSHOT_SERVICE_NAME: (),
    SYNC_ARCHIVE_SERVICE_NAME: (),
    SYNC_DRILL_SERVICE_NAME: (),
    SYNC_SERVER_SERVICE_NAME: ("SYNC_SERVER_OWNER", "SYNC_SERVER_STAGING"),
}
# The role each service runs (R2); the job template's `%i` is its instance, the job's id.
ROLES = {
    "deck-streak-api.service": "api",
    "deck-streak-bot.service": "bot",
    "deck-streak-mcp.service": "mcp",
    f"{JOB_TEMPLATE}@.service": "job %i",
}
# The settings a role requires, which the committed example must therefore name (SPEC-025 R6,
# SPEC-022 R5, SPEC-119 R3); systemd sets STATE_DIRECTORY and CREDENTIALS_DIRECTORY itself.
REQUIRED_SETTINGS = ("DECKSTREAK_API_LISTEN", "DECKSTREAK_SYNC_ENDPOINT", "DECKSTREAK_MCP_LISTEN")
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
# ADR-032's delivery: the daemons' CPU and task caps. The MCP server's quota is taken from the
# API's, so the daemons' quotas still fit the share's CPUs (ADR-332); the sync server's three
# quarters of a processor are taken from the bot's, the replicator's and the MCP server's (ADR-347).
DAEMON_CAPS = {
    "deck-streak-api.service": {"CPUQuota": "75%", "TasksMax": "64"},
    "deck-streak-bot.service": {"CPUQuota": "20%", "TasksMax": "64"},
    "deck-streak-mcp.service": {"CPUQuota": "15%", "TasksMax": "32"},
}
# R2: the hardening of every service, each at the value the pack's rows score; its identity is
# `hardening(name)`'s, by family (SPEC-340 R2).
HARDENING = {
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


def hardening(name):
    """The identity and hardening the service `name` carries: the sync family's own user and group,
    or DeckStreak's (SPEC-340 R2), and the hardening every service carries."""
    user = SYNC_USER if name in SYNC_FAMILY else SERVICE_USER
    return {"User": user, "Group": user, **HARDENING}


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
    "deck-streak-mcp.service": ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    f"{JOB_TEMPLATE}@.service": ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    f"{ALERT_TEMPLATE}@.service": (None, None, ROLES_NETWORK, "systemd-journal"),
    SLO_SERVICE: ("deck-streak-slo", None, "AF_UNIX", "systemd-journal"),
    WATCH_SERVICE: ("deck-streak-memory-watch", None, "AF_UNIX", None),
    SECOND_ROUTE_SERVICE: ("deck-streak-second-route", None, ROLES_NETWORK, None),
    # SPEC-064: the replicator and the drill reach the bucket; the backup copies the database
    # alone, reads no settings and opens no network socket (SPEC-340 R3).
    LITESTREAM_SERVICE_NAME: ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    BACKUP_SERVICE_NAME: ("deck-streak", None, "AF_UNIX", None),
    # SPEC-337, SPEC-340: the window reads the server's store, writes its generation into the sync
    # family's own directory, and opens no socket.
    SYNC_SNAPSHOT_SERVICE_NAME: (
        "deck-streak-sync-snapshots deck-streak-sync-server",
        None,
        "AF_UNIX",
        None,
    ),
    # SPEC-340 R3: the archive reads the generation and reaches the bucket by the settings' copy.
    SYNC_ARCHIVE_SERVICE_NAME: (
        "deck-streak-sync-snapshots",
        ENVIRONMENT_FILE,
        ROLES_NETWORK,
        None,
    ),
    DRILL_SERVICE_NAME: ("deck-streak", ENVIRONMENT_FILE, ROLES_NETWORK, None),
    # SPEC-340 R4: the sync drill restores the newest archive in a scratch directory, offline.
    SYNC_DRILL_SERVICE_NAME: ("deck-streak-sync-snapshots", None, "AF_UNIX", None),
    # SPEC-337: the sync server keeps its users' data in its own directory and listens on loopback.
    SYNC_SERVER_SERVICE_NAME: (
        "deck-streak-sync-server",
        ENVIRONMENT_FILE,
        "AF_UNIX AF_INET",
        None,
    ),
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
# The sync server's path on the web app's origin (ADR-347 D4): not `/sync`, whose server routes
# begin `/sync/` and `/msync/`.
SYNC_PATH = "/anki-sync"
# The edge's body bound: the server's own payload limit, its default of 100 MiB
# (`MAX_SYNC_PAYLOAD_MEGS`, which the launcher clears so the default holds; SPEC-337 A5), in Caddy's
# binary unit so the two agree to the byte.
SYNC_BODY_LIMIT = "100MiB"
# The reverse proxy's read buffer for the server's large responses (ADR-347 D4).
SYNC_READ_BUFFER = "512KiB"


# Every advisory departure the templates declare in their units, by unit and reason (SPEC-032 R4,
# SPEC-056 R16): the job table places each job on its own minute and makes none a catch-up job
# (ADR-027), and SPEC-031's two timers read rolling state that a missed run cannot lose. The box
# run holds every departure the durable lint reports to one of these, or to an issue it waits on
# (SPEC-056 R15).
WAIVED = {
    (f"{JOB_TEMPLATE}@sync.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@held_flush.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@maintenance.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@maintenance.timer", "calendar-not-persistent"),
    (f"{JOB_TEMPLATE}@drill_postback.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@drill_postback.timer", "calendar-not-persistent"),
    (f"{JOB_TEMPLATE}@liveness.timer", "randomized-delay-missing"),
    (f"{JOB_TEMPLATE}@liveness.timer", "calendar-not-persistent"),
    ("deck-streak-slo.timer", "calendar-not-persistent"),
    ("deck-streak-memory-watch.timer", "calendar-not-persistent"),
    ("deck-streak-second-route.timer", "calendar-not-persistent"),
    ("deck-streak-litestream.service", "watchdog-missing"),
    (SYNC_SERVER_SERVICE_NAME, "watchdog-missing"),
}


def subject(root=REPO):
    """Every unit under `root`'s deploy/, through the reader, which refuses a line it cannot
    read."""
    return _units.load_subject(Path(root))


def services(root=REPO):
    return examined("service unit(s) under deploy/", sorted(subject(root).services, key=name))


# SPEC-354 R1 to R3 (ADR-365 D1 to D4): the host's identity endpoint. systemd names the range it
# lies in `link-local`, and `any` holds that range too (systemd.resource-control(5), "Special
# address/network names"), so a deny list holding either word covers it.
IDENTITY_DENY = ("any", "link-local")
# systemd grants a packet that matches an allow entry before it reads the deny list
# (systemd.resource-control(5)), so a denied unit holds no allow entry but loopback's.
IDENTITY_ALLOW = ("localhost",)
# The closed list of the services that use the host's identity (R2): the replicator, the sync
# archive's upload and the restore drill reach their bucket with it. An entry whose unit the tree
# does not ship is refused.
IDENTITY_EXEMPT = (LITESTREAM_SERVICE_NAME, SYNC_ARCHIVE_SERVICE_NAME, DRILL_SERVICE_NAME)
# The address families that open an IP socket (systemd.exec(5), RestrictAddressFamilies=).
IP_FAMILIES = {"AF_INET", "AF_INET6"}
# How each shipped service is kept from the endpoint, written by hand: the word of its deny list
# that covers it, NO_IP_SOCKET when its families open no IP socket, or EXEMPT.
NO_IP_SOCKET = "opens no IP socket"
EXEMPT = "exempt"
IDENTITY_ARMS = {
    "deck-streak-api.service": "link-local",
    "deck-streak-bot.service": "link-local",
    "deck-streak-mcp.service": "link-local",
    f"{JOB_TEMPLATE}@.service": "link-local",
    f"{ALERT_TEMPLATE}@.service": "link-local",
    SYNC_SERVER_SERVICE_NAME: "any",
    BACKUP_SERVICE_NAME: NO_IP_SOCKET,
    SYNC_SNAPSHOT_SERVICE_NAME: NO_IP_SOCKET,
    SYNC_DRILL_SERVICE_NAME: NO_IP_SOCKET,
    SLO_SERVICE: NO_IP_SOCKET,
    WATCH_SERVICE: NO_IP_SOCKET,
    SECOND_ROUTE_SERVICE: "link-local",
    LITESTREAM_SERVICE_NAME: EXEMPT,
    SYNC_ARCHIVE_SERVICE_NAME: EXEMPT,
    DRILL_SERVICE_NAME: EXEMPT,
}


def ip_list(unit, key):
    """Every word of `unit`'s `[Service]` list `key`, its drop-ins included, after systemd's reset
    rule: an empty assignment clears what came before it."""
    return " ".join(unit.values("Service", key)).split()


def opens_no_ip_socket(unit):
    """Whether the kernel refuses `unit` every IP socket: an allow list of families, in every
    assignment, that names neither IP family, on the native ABI alone (systemd.exec(5) asks for
    SystemCallArchitectures=native beside it). A `~` deny list is read as one that may open an IP
    socket, never guessed."""
    families = unit.values("Service", "RestrictAddressFamilies")
    return (
        bool(families)
        and not any(value.startswith("~") for value in families)
        and not set(" ".join(families).split()) & IP_FAMILIES
        and unit.values("Service", "SystemCallArchitectures") == ["native"]
    )


def identity_arm(unit):
    """How `unit` is kept from the host's identity endpoint (R1): EXEMPT, NO_IP_SOCKET, the word of
    its deny list that covers the endpoint, or None when nothing does."""
    if unit.name in IDENTITY_EXEMPT:
        return EXEMPT
    if opens_no_ip_socket(unit):
        return NO_IP_SOCKET
    if any(word not in IDENTITY_ALLOW for word in ip_list(unit, "IPAddressAllow")):
        return None
    covering = [word for word in ip_list(unit, "IPAddressDeny") if word in IDENTITY_DENY]
    return covering[0] if covering else None


def identity_refusals(root=REPO):
    """Every service under `root`'s deploy/ that could reach the host's identity endpoint, each
    refusal naming the unit and why (R1, R3); the census prints its examined count and refuses
    zero."""
    refused = []
    for unit in services(root):
        if identity_arm(unit) is not None:
            continue
        wider = [word for word in ip_list(unit, "IPAddressAllow") if word not in IDENTITY_ALLOW]
        refused.extend(
            f"{unit.rel}: IPAddressAllow={word} can admit the host's identity endpoint, and is "
            "refused"
            for word in wider
        )
        if not wider:
            refused.append(
                f"{unit.rel}: opens an IP socket and no IPAddressDeny= covers the host's identity "
                "endpoint, and is refused"
            )
    return refused


def exempt_refusals(root=REPO):
    """Every exempt-list entry whose unit `root`'s deploy/ does not ship (R2), so the list can hold
    no stale name."""
    shipped = {unit.name for unit in subject(root).services}
    return [
        f"the exempt list names {name}, which deploy/ does not ship, and is refused"
        for name in IDENTITY_EXEMPT
        if name not in shipped
    ]


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
    """ADR-032's budget table with ADR-064's, by unit: (memory_high, memory_max). A row may name the
    SPEC that ships its unit after the unit's name, as SPEC-031's three rows do."""
    rows = []
    for source in (ADR, ADR_BACKUPS):
        rows += re.findall(
            r"(?m)^\| `([^`]+)`(?: \(SPEC-\d{3}\))? \| (\d+M) \| (\d+M) \|",
            source.read_text(encoding="utf-8"),
        )
    return {unit: (high, ceiling) for unit, high, ceiling in rows}


def credential_ids():
    """Each credential id the code declares, by its constant's name."""
    found = {}
    for constant, source in CREDENTIAL_SOURCES.items():
        form = (
            rf"(?m)^readonly {constant}=([a-z0-9-]+)$"
            if source.suffix == ".sh"
            else rf'pub const {constant}: &str = "([a-z0-9-]+)";'
        )
        # A source that is not there declares nothing, and is refused by name as one that names no id.
        match = re.search(form, source.read_text() if source.is_file() else "")
        if match is None:
            raise AssertionError(f"{source.relative_to(REPO)} declares no {constant}")
        found[constant] = match.group(1)
    return found


def declared_settings():
    """Every setting the workspace's code names: a `DECKSTREAK_*` constant in `crates/*/src`, a
    `DECKSTREAK_*` variable the sync server's launcher reads (SPEC-337 R2), and a `DECKSTREAK_*`
    constant the daily backup reads for the snapshot's offsite copy (SPEC-337 R5)."""
    found = set(re.findall(r"\$\{(DECKSTREAK_[A-Z_]+)", SYNC_LAUNCHER.read_text(encoding="utf-8")))
    found.update(
        re.findall(
            r'^[A-Z_]+ = "(DECKSTREAK_[A-Z_]+)"$',
            (REPO / "deploy" / "scripts" / "backup.py").read_text(encoding="utf-8"),
            re.MULTILINE,
        )
    )
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
        if rel.startswith("tmpfiles.d/") and path.suffix == ".conf":
            continue
        for _, key, value, number in _units.assignments(_units.unit_text(path), rel):
            if key in keys:
                found.append((rel, number, key, value))
    return found


# The ONE place that names the instance drop-in directories a shipped template may carry, by the
# template's file name (SPEC-062 R14, amended; ruling PR #512). The unit guards read every instance
# directory into its template, so an instance is admitted only by name and any other is refused:
# `sync` loads the sync login, `held_flush` loads the bot's token and the owner's id (#291).
INSTANCE_DROPIN_ALLOWLIST = {f"{JOB_TEMPLATE}@.service": ("sync", "held_flush")}

NON_UNIT_DROPIN = "deploy/journald.conf.d"
# The ban service's filter and jail directories hold its own files, never a unit's (SPEC-340 R5).
NON_UNIT_DIRECTORIES = (
    NON_UNIT_DROPIN,
    "deploy/tmpfiles.d",
    "deploy/fail2ban/filter.d",
    "deploy/fail2ban/jail.d",
)


def dropin_directory_refusals(root, allowlist=None):
    """Every `*.d/` directory under `root`'s deploy/ that is not the drop-in directory of a unit
    shipped beside it, as one line each: only `<unit name>.d/` is read with a unit (SPEC-066 R2), so
    any other is refused, and the directories of files that are no unit (journald, tmpfiles.d, the
    ban service's) are named here.
    A shipped template's instance directory is admitted only for an instance `allowlist` names (by
    default INSTANCE_DROPIN_ALLOWLIST)."""
    allowed = INSTANCE_DROPIN_ALLOWLIST if allowlist is None else allowlist
    refused = []
    deploy = Path(root) / "deploy"
    if not deploy.is_dir():
        return refused
    entries = sorted(deploy.rglob("*"))
    shipped = [
        path
        for path in entries
        if path.is_file()
        and path.suffix in _units.UNIT_KINDS
        and not path.parent.name.endswith(".d")
    ]
    own = {path.parent / f"{path.name}.d" for path in shipped}
    templates = {(path.parent, *path.name.split("@.", 1)) for path in shipped if "@." in path.name}

    def the_shipped_template_of(path):
        """systemd reads an instance's drop-ins from `<name>@<instance>.<type>.d/` (SPEC-062 R14)."""
        stem, at, rest = path.name.removesuffix(".d").partition("@")
        instance, dot, kind = rest.rpartition(".")
        template = (path.parent, stem, kind)
        found = at and dot and instance and template in templates
        return (f"{stem}@.{kind}", instance) if found else None

    folders = [path for path in entries if path.is_dir() and path.name.endswith(".d")]
    for path in folders:
        rel = path.relative_to(root).as_posix()
        if path in own:
            continue
        found = the_shipped_template_of(path)
        if found is not None:
            # The guards read every instance drop-in of a shipped template into the template
            # (SPEC-062 R14), so an instance is admitted by name alone; which instance loads which
            # credential is judged by the instance tests below (#291).
            name, instance = found
            if instance in allowed.get(name, ()):
                continue
            refused.append(
                f"{rel}: the instance {instance!r} of {name} is not on INSTANCE_DROPIN_ALLOWLIST, "
                "and is refused"
            )
            continue
        if rel in NON_UNIT_DIRECTORIES:
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
            if (
                _units.SECRET_NAME.search(variable)
                or variable.lower().replace("_", "-") in ids
                or SYNC_SERVER_USERS.fullmatch(variable)
            ):
                refused.append(f"{unit.rel}: {variable} passes a secret through the environment")
    return refused


def loads_a_credential(unit):
    """Whether `unit` holds a credential directive of any kind, its drop-ins and a template's
    instance drop-ins included (SPEC-062 R14)."""
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


# The one key an instance's drop-in may set (SPEC-062 R14). systemd applies an instance's drop-in to
# that instance alone, while the guards read it into its template, so any other setting there would
# be judged as the template's other instances' too, which systemd never gives them.
INSTANCE_DROPIN_KEYS = {("Service", "LoadCredential")}


def instance_dropin_refusals(unit):
    """Every assignment of `unit` read from an instance's drop-in directory, neither the unit file
    nor its own `<name>.d/`, whose section and key are not on INSTANCE_DROPIN_KEYS (SPEC-062
    R14)."""
    own = f"{unit.name}.d"
    return [
        f"{a.source}:{a.line}: [{a.section}] {a.key}={a.value} is set in an instance's drop-in, "
        "which systemd applies to that instance alone, and is refused"
        for a in unit.assignments
        if a.source != unit.rel
        and Path(a.source).parent.name != own
        and (a.section, a.key) not in INSTANCE_DROPIN_KEYS
    ]


def value_refusals(unit, table):
    """Every assignment of `unit`, its drop-ins included, of a key `table` bounds whose value is
    not one it admits, each naming its key and its value (SPEC-066 R2)."""
    return [
        f"{a.source}:{a.line}: [{a.section}] {a.key}={a.value} is not a value this unit admits "
        "for the key, and is refused"
        for a in unit.assignments
        if (a.section, a.key) in table and a.value not in table[(a.section, a.key)]
    ]


def restart_budget_refusals(unit):
    """Every key of the restart budget a restarting unit lacks (SPEC-066 R2): a unit that assigns
    `Restart=` to anything but `no`, in the unit or a drop-in, holds each key of
    `_units.RESTART_BUDGET`, each refusal naming the missing key."""
    if not any(value != "no" for value in unit.every("Service", "Restart")):
        return []
    return [
        f"{unit.rel}: assigns Restart= and holds no {key}=, which the restart budget needs, and "
        "is refused"
        for section, key in _units.RESTART_BUDGET
        if not unit.values(section, key)
    ]


def failure_target_refusals(unit):
    """Every `OnFailure=` assignment of `unit`, its drop-ins included, that is not exactly the alert
    template, an empty one and a target beside the alert's included (SPEC-066 R2)."""
    return [
        f"{a.source}:{a.line}: [Unit] OnFailure={a.value} is not the alert template "
        f"{ON_FAILURE}, and is refused"
        for a in unit.assignments
        if (a.section, a.key) == ("Unit", "OnFailure") and a.value != ON_FAILURE
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


# --- the sync route's one spelling (SPEC-351 R1-R3; ADR-362 D1-D3) ------------------------

# The guard's matcher name, and its one form: a negated match of the request-target as the edge
# received it, which no rewrite in the route changes and which the access log records.
SYNC_GUARD = "@sync_respelled"
SYNC_GUARD_EXPRESSION = re.compile(r"!\{http\.request\.orig_uri\}\.matches\('([^'\\{}]+)'\)")
# The server's routes under the prefix, a collection route and a media route of each shape, in
# the spelling the server's own client sends.
SYNC_SERVER_ROUTES = (
    "sync/hostKey",
    "sync/meta",
    "sync/sanityCheck2",
    "msync/begin",
    "msync/mediaSanity",
)
SYNC_QUERIES = ("", "?a=1", "?k=x&v=y")
SYNC_FORMS = ("", "https://app.example.org", "http://app.example.org", "https://APP.EXAMPLE.ORG")


def outside_the_subset(pattern):
    """What in the pattern Caddy's expression language and Python's `re` could read apart: the
    guard is RE2 at the edge and `re` here, and the two agree on literals, `^`, `$`, `.`, classes
    of literals and ranges, `(?:` groups and single `?`, `+` and `*` (SPEC-351 R3). Empty when
    the pattern is inside that subset."""
    found = []
    if re.fullmatch(r"[A-Za-z0-9^$()?:/\[\]+*.-]+", pattern) is None:
        found.append("a character outside the subset")
    if "[:" in pattern:
        found.append("a POSIX class")
    if pattern.count("(?") != pattern.count("(?:"):
        found.append("a group other than (?:")
    if re.search(r"[+*?][+*]", pattern):
        found.append("two quantifiers in a row")
    return found


def sync_guard(route):
    """The pattern of the sync route's guard: the route's one `@sync_respelled` matcher, of the
    form above, and the one `respond` that answers it 404. None when the route holds no such
    guard, which stands for an edge that serves every spelling its handle places in the route."""
    matchers = route.find(SYNC_GUARD)
    if len(matchers) != 1 or len(route.find("respond", SYNC_GUARD)) != 1:
        return None
    if route.one("respond", SYNC_GUARD).tokens != ["respond", SYNC_GUARD, "404"]:
        return None
    tokens = matchers[0].tokens
    if len(tokens) != 3 or tokens[1] != "expression":
        return None
    found = SYNC_GUARD_EXPRESSION.fullmatch(tokens[2])
    return found.group(1) if found else None


def request_path(spelling):
    """A request-target's path: without the scheme and authority of the absolute form, and
    without the query."""
    return re.sub(r"^[A-Za-z][A-Za-z0-9+.-]*://[^/]*", "", spelling).partition("?")[0]


def cleaned_segments(spelling):
    """The path's segments as a decoding matcher reads them: every escape decoded, empty and `.`
    segments dropped, and each `..` removing the segment before it."""
    kept = []
    decoded = re.sub(
        r"%([0-9A-Fa-f]{2})", lambda match: chr(int(match.group(1), 16)), request_path(spelling)
    )
    for segment in decoded.split("/"):
        if segment in ("", "."):
            continue
        if segment == "..":
            kept = kept[:-1]
            continue
        kept.append(segment)
    return kept


def in_sync_route(spelling):
    """Whether the handle's matcher places the request in the sync route: the path, decoded and
    cleaned, begins with the prefix in any case and names a route below it."""
    segments = cleaned_segments(spelling)
    return len(segments) > 1 and segments[0].lower() == SYNC_PATH[1:]


def serves(pattern, spelling):
    """Whether the edge serves the request: it is in the route, and the guard, when there is one,
    finds it."""
    return in_sync_route(spelling) and (pattern is None or re.search(pattern, spelling) is not None)


def respellings(path):
    """Every other spelling of a path in the sync route, each one change away from it: a
    character after the first as an escape in either case of hex, a letter of the prefix in the
    other case, the prefix in capitals, an empty, `.` or `x/..` segment at each slash, and a
    trailing `/` or `/.`."""
    found = []
    for index, char in enumerate(path[1:], start=1):
        for escape in (f"%{ord(char):02X}", f"%{ord(char):02x}"):
            found.append(path[:index] + escape + path[index + 1 :])
    for index, char in enumerate(SYNC_PATH):
        if char.isalpha():
            found.append(path[:index] + char.swapcase() + path[index + 1 :])
    found.append(SYNC_PATH.upper() + path[len(SYNC_PATH) :])
    for index, char in enumerate(path):
        if char == "/":
            found += [path[:index] + extra + path[index:] for extra in ("/", "/.", "/x/..")]
    found += [path + "/", path + "/."]
    return [spelling for spelling in dict.fromkeys(found) if spelling != path]


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
        # ADR-064 decides the share and ADR-032 carries a note that points to it; each record's
        # last word on the share is an appended amendment (SPEC-337), so the last one is in force.
        decided = re.findall(r'"memory": "(\d+M)"', ADR_BACKUPS.read_text(encoding="utf-8"))
        self.assertEqual(decided[-1:], [share], "ADR-064's last word on the share")
        noted = re.findall(
            r"(?m)^## (?:Note|Amendment)\b[^\n]*: the share is (\d+) MiB",
            ADR.read_text(encoding="utf-8"),
        )
        self.assertEqual(noted[-1:], [share.rstrip("M")], "ADR-032's last note on the share")
        ceilings = {}
        for unit in units:
            value = last(unit, "Service", "MemoryMax")
            self.assertIsNotNone(
                value, f"{unit.rel} has no MemoryMax, so it cannot be shown to fit"
            )
            ceilings[unit.name] = size(value)
        daemons = [u.name for u in units if _units.long_running(u)]
        oneshots = [u.name for u in units if not _units.long_running(u)]
        self.assertEqual(
            daemons,
            [
                "deck-streak-api.service",
                "deck-streak-bot.service",
                LITESTREAM_SERVICE_NAME,
                "deck-streak-mcp.service",
                SYNC_SERVER_SERVICE_NAME,
            ],
        )
        self.assertEqual(
            sorted(oneshots),
            sorted(
                [
                    f"{ALERT_TEMPLATE}@.service",
                    f"{JOB_TEMPLATE}@.service",
                    WATCH_SERVICE,
                    SECOND_ROUTE_SERVICE,
                    SLO_SERVICE,
                    BACKUP_SERVICE_NAME,
                    DRILL_SERVICE_NAME,
                    SYNC_SNAPSHOT_SERVICE_NAME,
                    SYNC_ARCHIVE_SERVICE_NAME,
                    SYNC_DRILL_SERVICE_NAME,
                ]
            ),
        )
        worst = sum(ceilings[unit] for unit in daemons) + max(ceilings[unit] for unit in oneshots)
        # ADR-064's arithmetic with ADR-332's daemon and SPEC-337's sync server: the five daemons,
        # and the job, still the largest, which fill the share exactly (ADR-347).
        self.assertEqual(worst, size("1152M"))
        self.assertLessEqual(worst, size(share), "the worst case exceeds DeckStreak's share")
        # The daemons' CPU quotas fit the share's CPUs; a daemon with no quota is refused by name.
        quotas = []
        for unit in units:
            if unit.name in daemons:
                quota = last(unit, "Service", "CPUQuota")
                self.assertIsNotNone(
                    quota, f"{unit.rel} has no CPUQuota, so it cannot be shown to fit"
                )
                quotas.append(int(quota.rstrip("%")))
        # The five quotas divide the share's processors exactly (ADR-347): 75, 75, 20, 15 and 15.
        self.assertEqual(sum(quotas), 100 * budget()["cpus"], quotas)


class TheSecondRouteIsInTheCensus(unittest.TestCase):
    def test_the_census_holds_the_second_route_as_a_timer_started_oneshot(self):
        # SPEC-396 R10: the second route's unit is shipped, and every table that holds a unit names
        # it. The first assertion is the artifact itself, so an unshipped unit fails here and not
        # on a missing key.
        service = "deck-streak-second-route.service"
        timer = "deck-streak-second-route.timer"
        shipped = {unit.name: unit for unit in services()}
        self.assertIn(service, shipped, "the second route's unit is not shipped")
        self.assertTrue((SYSTEMD / timer).is_file(), "the second route's timer is not shipped")
        script = DEPLOY / "scripts" / "second-route.sh"
        self.assertEqual(SCRIPTS.get(service), f"{RELEASE}/deploy/scripts/second-route.sh")
        self.assertEqual(
            OBSERVABILITY_SERVICE.get(service),
            {
                "Type": "oneshot",
                "TimeoutStartSec": "3min",
                "Nice": "10",
                "IOSchedulingClass": "idle",
            },
        )
        self.assertIn(timer, OBSERVABILITY_TIMERS)
        self.assertEqual(
            ROLE_CREDENTIALS.get(service), ("SECOND_ROUTE_CHECK_IN", "SECOND_ROUTE_REPORT")
        )
        for constant in ("SECOND_ROUTE_CHECK_IN", "SECOND_ROUTE_REPORT"):
            self.assertEqual(CREDENTIAL_SOURCES.get(constant), script, constant)
        self.assertEqual(
            PER_SERVICE.get(service), ("deck-streak-second-route", None, ROLES_NETWORK, None)
        )
        self.assertIn((timer, "calendar-not-persistent"), WAIVED)
        self.assertEqual(IDENTITY_ARMS.get(service), "link-local")
        unit = shipped[service]
        self.assertIn(ON_FAILURE, unit.values("Unit", "OnFailure"))
        self.assertEqual(last(unit, "Service", "Type"), "oneshot")
        self.assertEqual(
            last(unit, "Service", "ExecStart"), f"{RELEASE}/deploy/scripts/second-route.sh"
        )


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
        self.assertRegex(
            config, r"'script-src': \['self', 'https://telegram\.org', 'wasm-unsafe-eval'\]"
        )
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
        # Nothing answers outside the handles, so a route cannot slip past the order above. The log,
        # its matcher and its skip write a record and answer nothing (SPEC-340 R6).
        self.assertEqual(
            sorted(child.tokens[0] for child in block.children),
            ["@not_sync"] + ["handle"] * 5 + ["header", "log", "log_skip"],
        )
        # The upstream is the API's own loopback listener (ADR-007), which the example names.
        listen = dict((key, value) for _, key, value in env_example())["DECKSTREAK_API_LISTEN"]
        host = listen.rpartition(":")[0].strip("[]")
        self.assertTrue(ipaddress.ip_address(host).is_loopback, listen)

    def test_the_caddy_block_routes_the_sync_server_under_its_own_path(self):
        """SPEC-337 A6; ADR-347 D4: the sync server on the web app's origin, under its own path."""
        block = site()
        # The bare path is redirected to the slash form, so a client given either reaches the server.
        bare = handle(block, SYNC_PATH)
        self.assertIsNotNone(bare, f"no handle for {SYNC_PATH}")
        self.assertEqual(
            [child.tokens for child in bare.children], [["redir", "*", f"{SYNC_PATH}/", "308"]]
        )
        route = handle(block, f"{SYNC_PATH}/*")
        self.assertIsNotNone(route, f"no handle for {SYNC_PATH}/*")
        self.assertEqual(
            sorted(child.tokens[0] for child in route.children),
            [
                "@sync_health",
                SYNC_GUARD,
                "request_body",
                "respond",
                "respond",
                "reverse_proxy",
                "uri",
            ],
            "the route holds the strip, its guard, the hidden health route, the bound and the proxy",
        )
        # The prefix is stripped before the server reads the path; `uri` runs before `respond` and
        # `reverse_proxy` in Caddy's directive order, so both see the server's own route.
        self.assertEqual(route.one("uri").tokens, ["uri", "strip_prefix", SYNC_PATH])
        # The server's health route answers 404 from outside, as the API's do (ADR-025).
        health = route.one("@sync_health")
        self.assertEqual(health.tokens, ["@sync_health", "path", "/health"])
        self.assertEqual(
            route.one("respond", "@sync_health").tokens, ["respond", "@sync_health", "404"]
        )
        # The body is bounded at the edge at the server's own limit.
        body = route.one("request_body")
        self.assertEqual(body.tokens, ["request_body"])
        self.assertEqual([child.tokens for child in body.children], [["max_size", SYNC_BODY_LIMIT]])
        # The proxy reaches the sync server's loopback upstream, read with the larger buffer; it
        # drops the web cookie in both directions first (SPEC-340 R9, A11).
        proxy = route.one("reverse_proxy")
        self.assertEqual(proxy.tokens, ["reverse_proxy", "{$DECKSTREAK_SYNC_UPSTREAM}"])
        self.assertEqual(
            [child.tokens for child in proxy.children],
            [["header_up", "-Cookie"], ["header_down", "-Set-Cookie"], ["transport", "http"]],
        )
        transport = proxy.one("transport", "http")
        self.assertEqual(
            [child.tokens for child in transport.children], [["read_buffer", SYNC_READ_BUFFER]]
        )
        # The upstream is the sync server's own loopback listener, which the example names.
        listen = dict((key, value) for _, key, value in env_example())[
            "DECKSTREAK_SYNC_SERVER_LISTEN"
        ]
        host = listen.rpartition(":")[0].strip("[]")
        self.assertTrue(ipaddress.ip_address(host).is_loopback, listen)

    def test_the_sync_route_is_served_under_its_one_spelling(self):
        """SPEC-351 A1; ADR-362 D1: the edge serves the sync route under its one spelling and
        answers every other spelling of it 404, so nothing of that request reaches the server."""
        route = handle(site(), f"{SYNC_PATH}/*")
        self.assertIsNotNone(route, f"no handle for {SYNC_PATH}/*")
        pattern = sync_guard(route)
        spellings = examined(
            "respelling(s) of the sync route",
            [
                spelling
                for path in (f"{SYNC_PATH}/sync/hostKey", f"{SYNC_PATH}/msync/begin")
                for spelling in respellings(path)
            ],
        )
        served = [spelling for spelling in spellings if serves(pattern, spelling)]
        self.assertEqual(
            len(served), 0, f"{len(served)} respelling(s) of the route the edge serves"
        )
        # Each respelling is one the handle places in the route, so the refusal above is the
        # guard's and not the handle's.
        self.assertEqual(len([s for s in spellings if not in_sync_route(s)]), 0)
        # The one spelling is served: every route of the server, with a query or none, in the
        # origin form and the absolute form.
        canonical = examined(
            "spelling(s) of the server's routes as its client sends them",
            [
                form + f"{SYNC_PATH}/{name}" + query
                for name in SYNC_SERVER_ROUTES
                for query in SYNC_QUERIES
                for form in SYNC_FORMS
            ],
        )
        self.assertEqual([s for s in canonical if not serves(pattern, s)], [])
        # The guard reads the target as received, refuses with 404, and is written in the part of
        # its language that Python's reads alike; no spelling above holds the one character where
        # the two still differ.
        self.assertEqual(route.one("respond", SYNC_GUARD).tokens, ["respond", SYNC_GUARD, "404"])
        self.assertEqual(outside_the_subset(pattern), [])
        self.assertEqual([s for s in spellings + canonical if "\n" in s], [])
        for planted in ("[[:alpha:]]", "a(?=b)", "a++", "a{2}", "\\d", "(?i)a"):
            self.assertNotEqual(outside_the_subset(planted), [], planted)

    def test_the_sync_route_alone_is_logged_without_its_key(self):
        """SPEC-340 A7; ADR-351 D4: the edge writes one access log, of the sync route alone, with
        the client's address, the method, the path and the status, and no request header and no
        `k` parameter in it."""
        block = site()
        # One site-level log. It names no output, so it reaches the journal and its retention.
        logs = block.find("log")
        self.assertEqual([entry.tokens for entry in logs], [["log"]], "the block's access log")
        self.assertEqual([child.tokens for child in logs[0].children], [["format", "filter"]])
        # The filter deletes every request header and the `k` parameter, and writes JSON.
        fields = logs[0].one("format", "filter")
        self.assertEqual(
            [child.tokens for child in fields.children],
            [["request>headers", "delete"], ["request>uri", "query"], ["wrap", "json"]],
        )
        query = fields.one("request>uri", "query")
        self.assertEqual([child.tokens for child in query.children], [["delete", "k"]])
        # Every path outside the sync route is skipped, so the log holds that route alone.
        route = handle(block, f"{SYNC_PATH}/*")
        self.assertIsNotNone(route, f"no handle for {SYNC_PATH}/*")
        self.assertEqual(block.one("log_skip").tokens, ["log_skip", "@not_sync"])
        self.assertEqual(
            block.one("@not_sync").tokens, ["@not_sync", "not", "path", *route.tokens[1:]]
        )
        # No route writes a log of its own beside the site's.
        for each in examined("handle(s)", block.find("handle")):
            self.assertEqual(each.find("log"), [], f"the handle {each.tokens[1:]} logs")

    def test_the_sync_route_carries_no_web_cookie(self):
        """SPEC-340 A11; ADR-351 D7: the edge sends no `Cookie` header up to the sync server and
        passes no `Set-Cookie` header down from it, whatever the cookie's attributes are."""
        block = site()
        route = handle(block, f"{SYNC_PATH}/*")
        self.assertIsNotNone(route, f"no handle for {SYNC_PATH}/*")
        proxy = route.one("reverse_proxy")
        self.assertEqual(
            [child.tokens for child in proxy.children if child.tokens[0].startswith("header_")],
            [["header_up", "-Cookie"], ["header_down", "-Set-Cookie"]],
        )
        # The API's route is the web session's, so it keeps its cookie.
        api = handle(block, "/api/*")
        self.assertEqual([child.tokens for child in api.one("reverse_proxy").children], [])


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
            self.assertIn(
                key,
                # The replica's bucket is read by Litestream's configuration, not by a role; the
                # zone is read by chrono's `Local`, as the engine reads it (SPEC-083 R3).
                declared | {"RUST_LOG", "TZ", "DECKSTREAK_REPLICA_BUCKET"},
                f"{where}: no role reads {key}",
            )
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


def litestream_items(text):
    """Each `dbs:` item of a Litestream configuration, as a dict of the item's own keys (`path`, or
    `dir` with `pattern` and `recursive`) to (line, value). A nested block's keys, such as a
    replica's own `path`, are not the item's."""
    items = []
    top = None
    dash = None
    for number, raw in enumerate(text.splitlines(), start=1):
        body = raw.strip()
        if not body or body.startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip())
        if indent == 0:
            top = body.partition(":")[0]
            dash = None
            continue
        if top != "dbs":
            continue
        if body.startswith("- ") and dash in (None, indent):
            dash = indent
            items.append({})
            body = body[2:]
            indent += 2
        if not items or indent != dash + 2:
            continue
        key, colon, value = body.partition(":")
        if colon:
            items[-1][key.strip()] = (number, value.strip().strip("\"'"))
    return items


def skip_backup_reachers(litestream_where, litestream_text, daily_where, daily_text):
    """Each line of a Litestream configuration or of the daily copy's script that would carry the
    skip's backup off the host or into a standing copy, as `<file>:<line>: <reason>`; empty when
    none does (SPEC-083 A55). A database's `path` reaches a backup it matches as a pattern; a
    directory reaches one inside it, or below it when recursive, whose name its `pattern` matches,
    and a directory with no pattern is read as matching every name. The daily copy reaches a
    backup when the one file it copies or the copies it keeps and prunes match its name; a script
    whose names the census cannot read is refused, never passed."""
    found = []
    backups = [PurePosixPath(STATE_DIRECTORY) / name for name in SKIP_BACKUP_NAMES]
    for item in litestream_items(litestream_text):
        if "path" in item:
            number, path = item["path"]
            for backup in backups:
                if fnmatch.fnmatchcase(str(backup), path):
                    found.append(f"{litestream_where}:{number}: replicates {backup}")
        if "dir" in item:
            number, directory = item["dir"]
            pattern = item.get("pattern", (number, "*"))[1]
            recursive = item.get("recursive", (number, "false"))[1] == "true"
            root = PurePosixPath(directory)
            for backup in backups:
                inside = backup.parent == root or (recursive and root in backup.parents)
                if inside and fnmatch.fnmatchcase(backup.name, pattern):
                    found.append(f"{litestream_where}:{number}: replicates {backup}")
    for key, expression in DAILY_COPY_NAMES:
        match = expression.search(daily_text)
        if match is None:
            found.append(f"{daily_where}: names no {key}")
            continue
        number = daily_text.count("\n", 0, match.start()) + 1
        for name in SKIP_BACKUP_NAMES:
            if key == "DATABASE_NAME" and fnmatch.fnmatchcase(name, match.group(1)):
                found.append(f"{daily_where}:{number}: {key} copies {name}")
            if key == "COPY_NAME" and re.search(match.group(1), name):
                found.append(f"{daily_where}:{number}: {key} keeps {name}")
    return found


class TheSkipBackupStaysOnTheHost(unittest.TestCase):
    def test_no_replica_or_daily_copy_reaches_the_skip_backup(self):
        litestream = LITESTREAM_CONFIG.read_text(encoding="utf-8")
        daily = DAILY_COPY.read_text(encoding="utf-8")
        examined("Litestream database item(s)", litestream_items(litestream))
        examined("skip backup name(s)", SKIP_BACKUP_NAMES)
        self.assertEqual(
            skip_backup_reachers(
                "deploy/litestream.yml", litestream, "deploy/scripts/backup.py", daily
            ),
            [],
        )
        # The census judges. A database path whose pattern names the backup, a directory over the
        # state directory, and a daily copy that copies or keeps the backup's name are each refused
        # by file and line; a replica's own `path` is not a database's.
        replica = (
            f"    replica:\n      type: gcs\n      path: {STATE_DIRECTORY}/skip-backup-1.anki2\n"
        )
        planted = (
            "dbs:\n"
            f"  - path: {STATE_DIRECTORY}/deck_streak.db\n{replica}"
            f"  - path: {STATE_DIRECTORY}/skip-backup-*.anki2*\n{replica}"
            f'  - dir: {STATE_DIRECTORY}\n    pattern: "*.anki2"\n{replica}'
            "  - dir: /var/lib\n    recursive: true\n"
        )
        planted_daily = daily.replace(
            'DATABASE_NAME = "deck_streak.db"', 'DATABASE_NAME = "skip-backup-1.anki2"'
        ).replace('COPY_NAME = re.compile(r"', 'COPY_NAME = re.compile(r".*|')
        self.assertNotEqual(planted_daily, daily)
        database = daily.splitlines().index('DATABASE_NAME = "deck_streak.db"') + 1
        copies = next(
            n for n, line in enumerate(daily.splitlines(), 1) if line.startswith("COPY_NAME = ")
        )
        backup, partial = (f"{STATE_DIRECTORY}/{name}" for name in SKIP_BACKUP_NAMES)
        self.assertEqual(
            skip_backup_reachers("planted.yml", planted, "planted.py", planted_daily),
            [
                f"planted.yml:6: replicates {backup}",
                f"planted.yml:6: replicates {partial}",
                f"planted.yml:10: replicates {backup}",
                f"planted.yml:15: replicates {backup}",
                f"planted.yml:15: replicates {partial}",
                f"planted.py:{database}: DATABASE_NAME copies skip-backup-1.anki2",
                f"planted.py:{copies}: COPY_NAME keeps skip-backup-1.anki2",
                f"planted.py:{copies}: COPY_NAME keeps skip-backup-1.anki2.partial",
            ],
        )
        # A daily copy whose names the census cannot read is refused, never passed.
        self.assertEqual(
            skip_backup_reachers("planted.yml", "dbs:\n", "planted.py", ""),
            ["planted.py: names no DATABASE_NAME", "planted.py: names no COPY_NAME"],
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
                    f"LoadCredential{blank}=\n"
                )
                (Path(scratch) / "planted.service.d").mkdir()
                (Path(scratch) / "planted.service.d" / "10-planted.conf").write_text(
                    f"[Service]\nLoadCredential{blank}=telegram-bot-token:/etc/token\n"
                )
                found = credential_lines(scratch)
            self.assertEqual(
                socket_form_refusals(found, ids),
                [
                    "planted.service:2: LoadCredentialEncrypted= is refused; use LoadCredential= "
                    "(ADR-038)",
                    "planted.service:3: telegram-bot-token is read from '/etc/token', not the "
                    "socket",
                    "planted.service:4: '' is not a DeckStreak credential id",
                    "planted.service.d/10-planted.conf:2: telegram-bot-token is read from "
                    "'/etc/token', not the socket",
                ],
                what,
            )

    def test_a_shipped_templates_instance_dropin_directory_is_its_own_and_no_other_is(self):
        # systemd reads an instance's drop-ins from `<name>@<instance>.<type>.d/`. The unit guards
        # read every such directory into the template, so a directory is admitted only for an
        # instance NAMED on INSTANCE_DROPIN_ALLOWLIST, the one place that names them: an unnamed
        # instance of a shipped template is refused, and so is any instance of a template the tree
        # does not ship, named or not (SPEC-062 R14, amended; #291).
        shipped = "is not on INSTANCE_DROPIN_ALLOWLIST, and is refused"
        unshipped = "is not the drop-in directory of a unit shipped beside it, and is refused"
        with tempfile.TemporaryDirectory() as scratch:
            systemd = Path(scratch) / "deploy" / "systemd"
            systemd.mkdir(parents=True)
            (systemd / "planted@.service").write_text("[Service]\n", encoding="utf-8")
            (systemd / "planted@tty1.service.d").mkdir()
            self.assertEqual(
                dropin_directory_refusals(scratch),
                [
                    f"deploy/systemd/planted@tty1.service.d: the instance 'tty1' of planted@.service {shipped}"
                ],
                "an instance the allowlist does not name",
            )
            named = {"planted@.service": ("tty1", "test"), "other-app@.service": ("tty1",)}
            self.assertEqual(
                dropin_directory_refusals(scratch, named), [], "the instance the list names"
            )
            (systemd / "planted@test.service.d").mkdir()
            self.assertEqual(
                dropin_directory_refusals(scratch, named), [], "a second instance, also named"
            )
            (systemd / "planted@tty1.service.d" / "10-planted.conf").write_text(
                "[Unit]\nRequires=missing.service\n", encoding="utf-8"
            )
            (planted,) = subject(scratch).services
            self.assertEqual(
                off_list_refusals(planted, _units.PAGING_KEYS),
                [
                    "deploy/systemd/planted@tty1.service.d/10-planted.conf:2: [Unit] Requires="
                    "missing.service is not on this unit's list of keys, and is refused"
                ],
                "a key planted in the named instance's drop-in",
            )
            (systemd / "other-app@tty1.service.d").mkdir()
            self.assertEqual(
                dropin_directory_refusals(scratch, named),
                [f"deploy/systemd/other-app@tty1.service.d: {unshipped}"],
                "an instance of a template the tree does not ship, though the list names it",
            )

    def test_every_shipped_instance_dropin_directory_is_named_or_refused(self):
        # The census the tree answers to: each instance directory under a shipped template is on
        # the allowlist, and the list holds nothing the tree does not ship.
        folders = examined(
            "job instance drop-in directories", sorted(SYSTEMD.glob(f"{JOB_TEMPLATE}@*.service.d"))
        )
        named = INSTANCE_DROPIN_ALLOWLIST[f"{JOB_TEMPLATE}@.service"]
        shipped = {f.name.removesuffix(".service.d").partition("@")[2] for f in folders}
        self.assertEqual(shipped, set(named))
        self.assertEqual(dropin_directory_refusals(REPO), [])

    def test_a_templates_own_dropin_directory_is_read_once(self):
        # `<name>@.<type>.d/` is the template's own directory; the instance glob must not match it
        # too, or each of its drop-ins is read twice and refused twice (SPEC-062 R14).
        with tempfile.TemporaryDirectory() as scratch:
            systemd = Path(scratch) / "deploy" / "systemd"
            systemd.mkdir(parents=True)
            (systemd / "planted@.service").write_text("[Service]\n", encoding="utf-8")
            (systemd / "planted@.service.d").mkdir()
            (systemd / "planted@.service.d" / "10.conf").write_text(
                "[Unit]\nRequires=missing.service\n", encoding="utf-8"
            )
            (planted,) = subject(scratch).services
            self.assertEqual(
                off_list_refusals(planted, _units.PAGING_KEYS),
                [
                    "deploy/systemd/planted@.service.d/10.conf:2: [Unit] Requires="
                    "missing.service is not on this unit's list of keys, and is refused"
                ],
                "a key planted in the template's own drop-in directory",
            )

    def test_an_instance_dropin_sets_only_the_credentials_it_loads(self):
        # The guards read an instance's drop-in into its template, but systemd applies it to that
        # instance alone: a setting there would stand in for the template's other instances, so an
        # instance's drop-in sets only `LoadCredential=` (SPEC-062 R14).
        units = examined("unit(s) under deploy/", list(subject().units.values()))
        self.assertEqual([r for unit in units for r in instance_dropin_refusals(unit)], [])
        with tempfile.TemporaryDirectory() as scratch:
            systemd = Path(scratch) / "deploy" / "systemd"
            (systemd / "planted@.service.d").mkdir(parents=True)
            (systemd / "planted@tty1.service.d").mkdir()
            (systemd / "planted@.service").write_text("[Service]\nMemoryMax=4G\n", encoding="utf-8")
            (systemd / "planted@.service.d" / "10.conf").write_text(
                "[Service]\nMemoryHigh=3G\n", encoding="utf-8"
            )
            (systemd / "planted@tty1.service.d" / "10-planted.conf").write_text(
                f"[Service]\nLoadCredential=telegram-bot-token:{SOCKET}\nMemoryMax=48M\n",
                encoding="utf-8",
            )
            (planted,) = subject(scratch).services
            self.assertEqual(
                instance_dropin_refusals(planted),
                [
                    "deploy/systemd/planted@tty1.service.d/10-planted.conf:3: [Service] "
                    "MemoryMax=48M is set in an instance's drop-in, which systemd applies to that "
                    "instance alone, and is refused"
                ],
                "a template's setting restated in its one instance's drop-in",
            )

    def test_only_a_units_own_dropin_directory_is_shipped_under_deploy(self):
        # The tree ships the drop-in directories of its units and of the sync instance, and one
        # directory of a file that is no unit (SPEC-066 R2). Planted beside a unit: a directory
        # named for no unit, one named for the suffix alone, one named for an instance of a template
        # of another type, and one named for a unit that is not shipped: each refused by its path.
        # A unit's own is read, so it is not refused here.
        self.assertEqual(dropin_directory_refusals(REPO), [])
        refused = "is not the drop-in directory of a unit shipped beside it, and is refused"
        planted = [
            "deck-streak-.service.d",
            "service.d",
            f"planted{'@'}one.timer.d",
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
        beside = [
            ("deploy/scripts/planted.service.d", ["deploy/systemd/planted.service"]),
            ("deploy/other/journald.conf.d", []),
            ("deploy/journald.conf.d/planted.service.d", []),
            ("deploy/systemd/nested/planted.service.d", []),
            ("deploy/scripts/planted.sh.d", ["deploy/scripts/planted.sh"]),
            (
                "deploy/systemd/planted.service.d/inner.service.d",
                [
                    "deploy/systemd/planted.service",
                    "deploy/systemd/planted.service.d/inner.service",
                ],
            ),
        ]
        for where, files in beside:
            with tempfile.TemporaryDirectory() as scratch:
                (Path(scratch) / where).mkdir(parents=True)
                for file in files:
                    (Path(scratch) / file).parent.mkdir(parents=True, exist_ok=True)
                    (Path(scratch) / file).write_text("[Service]\n", encoding="utf-8")
                self.assertEqual(dropin_directory_refusals(scratch), [f"{where}: {refused}"], where)


class TheServicesRunTheirRoles(unittest.TestCase):
    def test_every_service_runs_its_role_with_the_lifecycle_r1_names(self):
        units = services()
        # Every role and every daemon's caps the tables name runs in a shipped unit: the tables
        # drift in neither direction.
        shipped = {unit.name for unit in units}
        self.assertLessEqual(set(ROLES) | set(DAEMON_CAPS), shipped, "a role with no unit")
        for unit in units:
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
                # A timer or a failure starts each. The backup's run starts the snapshot's window,
                # which runs first (ADR-347 D12), and the archive (SPEC-340 R3); the drill's run
                # starts the sync drill (SPEC-340 R4).
                wanted = {
                    SYNC_SNAPSHOT_SERVICE_NAME: [BACKUP_SERVICE_NAME],
                    SYNC_ARCHIVE_SERVICE_NAME: [BACKUP_SERVICE_NAME],
                    SYNC_DRILL_SERVICE_NAME: [DRILL_SERVICE_NAME],
                }.get(unit.name, [])
                self.assertEqual(unit.values("Install", "WantedBy"), wanted, unit.rel)
                continue
            if unit.name == LITESTREAM_SERVICE_NAME:
                # SPEC-064 R1: the replicator runs the binary the rail provides with the release's
                # own configuration, and keeps running.
                self.assertEqual(
                    unit.values("Service", "ExecStart"),
                    [
                        f"/usr/local/bin/litestream replicate -config {RELEASE}/deploy/litestream.yml"
                    ],
                    unit.rel,
                )
                self.assertEqual(last(unit, "Unit", "OnFailure"), ON_FAILURE, unit.rel)
                for key, value in LITESTREAM_SERVICE.items():
                    self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
                for key, value in DAEMON_UNIT.items():
                    self.assertEqual(last(unit, "Unit", key), value, f"{unit.rel} {key}")
                self.assertEqual(unit.values("Install", "WantedBy"), ["multi-user.target"])
                continue
            if unit.name == SYNC_SERVER_SERVICE_NAME:
                # SPEC-337 R2: the sync server runs the release's launcher, which reads its users
                # and execs the server the release ships, and keeps running.
                self.assertEqual(
                    unit.values("Service", "ExecStart"),
                    [f"{RELEASE}/deploy/scripts/sync-server.sh"],
                    unit.rel,
                )
                self.assertEqual(last(unit, "Unit", "OnFailure"), ON_FAILURE, unit.rel)
                for key, value in SYNC_SERVER_SERVICE.items():
                    self.assertEqual(last(unit, "Service", key), value, f"{unit.rel} {key}")
                for key, value in DAEMON_UNIT.items():
                    self.assertEqual(last(unit, "Unit", key), value, f"{unit.rel} {key}")
                self.assertEqual(unit.values("Install", "WantedBy"), ["multi-user.target"])
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
        units = services()
        # The per-service table names every shipped service and no other.
        self.assertEqual(
            sorted(PER_SERVICE), [unit.name for unit in units], "the per-service table"
        )
        for unit in units:
            for key, value in hardening(unit.name).items():
                self.assertEqual(
                    unit.values("Service", key), [value] if value else [], f"{unit.rel} {key}"
                )
            for key, value in zip(PER_SERVICE_KEYS, PER_SERVICE[unit.name], strict=True):
                self.assertEqual(
                    unit.values("Service", key), [value] if value else [], f"{unit.rel} {key}"
                )
            self.assertTrue(unit.assigned("Service", "CapabilityBoundingSet"), unit.rel)
            self.assertEqual(unit.values("Service", "AmbientCapabilities"), [], unit.rel)

    def test_no_unit_but_the_exempt_reaches_the_host_identity_endpoint(self):
        """SPEC-354 A1 (R1, R3; ADR-365 D1, D2, D4): every shipped service is kept from the host's
        identity endpoint by a deny list that covers it or by opening no IP socket, or is on the
        closed exempt list, and each takes the way the table names."""
        self.assertEqual(identity_refusals(), [])
        self.assertEqual({unit.name: identity_arm(unit) for unit in services()}, IDENTITY_ARMS)
        # Planted, each on a scratch copy of deploy/systemd: one way to the endpoint opened in one
        # unit's file or drop-in, and the refusal that names it.
        sync_job = f"{JOB_TEMPLATE}@{INSTANCE_DROPIN_ALLOWLIST[f'{JOB_TEMPLATE}@.service'][0]}"
        plants = {
            "the bot's deny removed": (
                "deck-streak-bot.service",
                "IPAddressDeny=link-local\n",
                "",
                [
                    "deploy/systemd/deck-streak-bot.service: opens an IP socket and no "
                    "IPAddressDeny= covers the host's identity endpoint, and is refused"
                ],
            ),
            "the bot's deny of a range that misses the endpoint": (
                "deck-streak-bot.service",
                "IPAddressDeny=link-local\n",
                "IPAddressDeny=multicast\n",
                [
                    "deploy/systemd/deck-streak-bot.service: opens an IP socket and no "
                    "IPAddressDeny= covers the host's identity endpoint, and is refused"
                ],
            ),
            "the API admits the link-local range": (
                "deck-streak-api.service",
                "IPAddressDeny=link-local\n",
                "IPAddressDeny=link-local\nIPAddressAllow=link-local\n",
                [
                    "deploy/systemd/deck-streak-api.service: IPAddressAllow=link-local can admit "
                    "the host's identity endpoint, and is refused"
                ],
            ),
            "the sync server admits every peer": (
                SYNC_SERVER_SERVICE_NAME,
                "IPAddressAllow=localhost\n",
                "IPAddressAllow=any\n",
                [
                    f"deploy/systemd/{SYNC_SERVER_SERVICE_NAME}: IPAddressAllow=any can admit the "
                    "host's identity endpoint, and is refused"
                ],
            ),
            "the SLO evaluator opens an IP socket": (
                SLO_SERVICE,
                "RestrictAddressFamilies=AF_UNIX\n",
                "RestrictAddressFamilies=AF_UNIX AF_INET\n",
                [
                    f"deploy/systemd/{SLO_SERVICE}: opens an IP socket and no IPAddressDeny= covers "
                    "the host's identity endpoint, and is refused"
                ],
            ),
            "the backup's families as a deny list": (
                BACKUP_SERVICE_NAME,
                "RestrictAddressFamilies=AF_UNIX\n",
                "RestrictAddressFamilies=~AF_PACKET\n",
                [
                    f"deploy/systemd/{BACKUP_SERVICE_NAME}: opens an IP socket and no IPAddressDeny= "
                    "covers the host's identity endpoint, and is refused"
                ],
            ),
            "the sync job's drop-in resets the deny": (
                f"{sync_job}.service.d/30-planted.conf",
                None,
                "[Service]\nIPAddressDeny=\n",
                [
                    f"deploy/systemd/{JOB_TEMPLATE}@.service: opens an IP socket and no "
                    "IPAddressDeny= covers the host's identity endpoint, and is refused"
                ],
            ),
        }
        got = {}
        for label in examined("planted way(s) to the host's identity endpoint", sorted(plants)):
            file, find, replace, _ = plants[label]
            with tempfile.TemporaryDirectory() as scratch:
                systemd = Path(scratch) / "deploy" / "systemd"
                shutil.copytree(SYSTEMD, systemd)
                path = systemd / file
                if find is None:
                    path.write_text(replace, encoding="utf-8")
                else:
                    text = path.read_text(encoding="utf-8")
                    self.assertEqual(text.count(find), 1, label)
                    path.write_text(text.replace(find, replace), encoding="utf-8")
                got[label] = identity_refusals(scratch)
        self.assertEqual(got, {label: plants[label][3] for label in plants})

    def test_an_exempt_entry_whose_unit_is_absent_is_refused(self):
        """SPEC-354 A2 (R2; ADR-365 D3): every name on the closed exempt list is a unit the tree
        ships, and a scratch copy of deploy/systemd that lacks one is refused by the entry's
        name."""
        self.assertEqual(exempt_refusals(), [])
        got = {}
        for name in examined("exempt-list entr(ies)", IDENTITY_EXEMPT):
            with tempfile.TemporaryDirectory() as scratch:
                shutil.copytree(
                    SYSTEMD,
                    Path(scratch) / "deploy" / "systemd",
                    ignore=shutil.ignore_patterns(name),
                )
                got[name] = exempt_refusals(scratch)
        self.assertEqual(
            got,
            {
                name: [f"the exempt list names {name}, which deploy/ does not ship, and is refused"]
                for name in IDENTITY_EXEMPT
            },
        )


class TheSyncServerRunsAsItsOwnUnit(unittest.TestCase):
    """SPEC-337 A3 and A4 (R2, R3; ADR-347 D2, D3): the sync server's own unit, its entry in the
    share, and its two users, which reach it as credentials and never through an environment."""

    def unit(self):
        found = [unit for unit in services() if unit.name == SYNC_SERVER_SERVICE_NAME]
        self.assertEqual(len(found), 1, f"no {SYNC_SERVER_SERVICE_NAME} under deploy/systemd")
        return found[0]

    def test_the_sync_server_runs_hardened_within_its_entry_and_the_share_holds_it(self):
        unit = self.unit()
        self.assertEqual(
            unit.values("Service", "ExecStart"), [f"{RELEASE}/deploy/scripts/sync-server.sh"]
        )
        for key, value in SYNC_SERVER_SERVICE.items():
            self.assertEqual(last(unit, "Service", key), value, key)
        for key, value in hardening(SYNC_SERVER_SERVICE_NAME).items():
            self.assertEqual(unit.values("Service", key), [value] if value else [], key)
        self.assertEqual(unit.values("Service", "StateDirectory"), ["deck-streak-sync-server"])
        self.assertEqual(unit.values("Service", "RestrictAddressFamilies"), ["AF_UNIX AF_INET"])
        self.assertEqual(unit.values("Unit", "OnFailure"), [ON_FAILURE])
        # It sends no readiness and no keep-alive, so its one waiver is the watchdog's, with a why.
        waivers = [value.split(None, 1) for value in unit.values("Unit", _units.WAIVE_KEY)]
        self.assertEqual([waiver[0] for waiver in waivers], ["watchdog-missing"])
        self.assertGreater(len(waivers[0][1].split()), 5, "a waiver with no why")
        # R3: its own entry, which the unit equals, holds the measured peak below MemoryHigh=.
        entry = budget()["units"].get(SYNC_SERVER_SERVICE_NAME)
        self.assertEqual(entry, {"memory_high": "384M", "memory_max": "448M"})
        high, ceiling = last(unit, "Service", "MemoryHigh"), last(unit, "Service", "MemoryMax")
        self.assertEqual((high, ceiling), (entry["memory_high"], entry["memory_max"]))
        self.assertGreater(size(high), SYNC_SERVER_PEAK)
        # The share holds it: the five daemons and the largest job fill 1152M, and their quotas
        # divide the share's two processors as ADR-347 splits them.
        units = services()
        daemons = [u for u in units if _units.long_running(u)]
        jobs = [u for u in units if not _units.long_running(u)]
        worst = sum(size(last(u, "Service", "MemoryMax")) for u in daemons) + max(
            size(last(u, "Service", "MemoryMax")) for u in jobs
        )
        self.assertEqual((budget()["memory"], budget()["cpus"]), ("1152M", 2))
        self.assertEqual(worst, size(budget()["memory"]))
        self.assertEqual(
            {u.name: last(u, "Service", "CPUQuota") for u in daemons},
            {
                "deck-streak-api.service": "75%",
                "deck-streak-bot.service": "20%",
                LITESTREAM_SERVICE_NAME: "15%",
                "deck-streak-mcp.service": "15%",
                SYNC_SERVER_SERVICE_NAME: "75%",
            },
        )

    def test_the_sync_family_runs_as_its_own_user(self):
        """SPEC-340 A2 (R2; ADR-351 D1): the sync server, its window, its archive and its sync drill
        run as `deck-streak-sync`, every other service as `deck-streak`, and no state directory is
        named by units of both users."""
        units = services()
        for unit in units:
            want = SYNC_USER if unit.name in SYNC_FAMILY else SERVICE_USER
            self.assertEqual(unit.values("Service", "User"), [want], unit.rel)
            self.assertEqual(unit.values("Service", "Group"), [want], unit.rel)
            named = set(" ".join(unit.values("Service", "StateDirectory")).split())
            if unit.name in SYNC_FAMILY:
                self.assertNotIn(SERVICE_USER, named, unit.rel)
            else:
                self.assertEqual(named & SYNC_STATE_DIRECTORIES, set(), unit.rel)
        family = examined(
            "sync family unit(s)", [unit.name for unit in units if unit.name in SYNC_FAMILY]
        )
        self.assertEqual(sorted(family), sorted(SYNC_FAMILY))
        self.assertEqual(
            sorted(unit.name for unit in units if unit.values("Service", "User") == [SYNC_USER]),
            sorted(SYNC_FAMILY),
        )

    def test_the_sync_server_reaches_loopback_peers_only(self):
        """SPEC-340 A10 (R8; ADR-351 D6): the server's peers are the loopback edge alone, and the
        paging census bounds both keys, so a wider peer list is refused by key and value."""
        unit = self.unit()
        self.assertEqual(unit.values("Service", "IPAddressAllow"), ["localhost"])
        self.assertEqual(unit.values("Service", "IPAddressDeny"), ["any"])
        for key in ("IPAddressAllow", "IPAddressDeny"):
            self.assertIn(key, _units.PAGING_KEYS["Service"], key)
        table = _units.PAGING_VALUES
        self.assertEqual(table.get(("Service", "IPAddressAllow")), ("localhost",))
        self.assertEqual(table.get(("Service", "IPAddressDeny")), ("any", "link-local"))
        # A planted unit that admits every peer and clears the deny list is refused at both lines.
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "deploy" / "systemd" / "planted.service"
            planted.parent.mkdir(parents=True)
            planted.write_text(
                "[Unit]\nDescription=planted\n\n[Service]\nExecStart=/bin/true\n"
                "IPAddressAllow=any\nIPAddressDeny=\n",
                encoding="utf-8",
            )
            (planted_unit,) = subject(scratch).services
        where = "deploy/systemd/planted.service"
        self.assertEqual(
            value_refusals(planted_unit, table),
            [
                f"{where}:6: [Service] IPAddressAllow=any is not a value this unit admits for the "
                "key, and is refused",
                f"{where}:7: [Service] IPAddressDeny= is not a value this unit admits for the "
                "key, and is refused",
            ],
        )

    def test_the_sync_servers_two_users_come_from_the_socket_and_never_an_environment(self):
        unit = self.unit()
        ids = credential_ids()
        owner, staging = ids["SYNC_SERVER_OWNER"], ids["SYNC_SERVER_STAGING"]
        self.assertNotEqual(owner, staging)
        self.assertEqual(
            unit.values("Service", "LoadCredential"), [f"{owner}:{SOCKET}", f"{staging}:{SOCKET}"]
        )
        # No unit sets a user of the server, and no settings line names one.
        for each in examined("service unit(s)", services()):
            self.assertEqual(environment_refusals(each, set(ids.values())), [], each.rel)
        settings = [key for _, key, _ in env_example()]
        self.assertEqual([key for key in settings if SYNC_SERVER_USERS.fullmatch(key)], [])
        # No hash, real or placeholder, is in the deploy files or the records.
        files = examined(
            "file(s) under deploy/ and docs/",
            [p for root in (DEPLOY, REPO / "docs") for p in root.rglob("*") if p.is_file()],
        )
        holding = [
            p.relative_to(REPO).as_posix()
            for p in files
            if PHC_HASH.search(p.read_text(encoding="utf-8", errors="replace"))
        ]
        self.assertEqual(holding, [])
        # A planted unit that sets a user through its environment is refused by the variable's
        # name; a planted hash is found.
        with tempfile.TemporaryDirectory() as scratch:
            planted = Path(scratch) / "deploy" / "systemd" / "planted.service"
            planted.parent.mkdir(parents=True)
            planted.write_text(
                "[Unit]\nDescription=planted\n\n[Service]\nExecStart=/bin/true\n"
                "Environment=SYNC_USER2=planted:kept PASSWORDS_HASHED=1\n"
            )
            (planted_unit,) = subject(scratch).services
        self.assertEqual(
            environment_refusals(planted_unit, set(ids.values())),
            ["deploy/systemd/planted.service: SYNC_USER2 passes a secret through the environment"],
        )
        shape = "$".join(("", "pbkdf2-sha256", "i=600000,l=32", "A" * 22, "B" * 43))
        self.assertIsNotNone(PHC_HASH.search(f"SYNC_USER1=planted:{shape}"))


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
        # The lists are the keys the units use: each kind's list is exactly the (section, key)
        # pairs its shipped units hold, drop-ins included, so a key no unit uses is on no list.
        used = {"alert": set(), "paging": set()}
        for unit in loading:
            used["alert" if unit.name == alert else "paging"] |= {
                (a.section, a.key) for a in unit.assignments
            }
        for kind, keys in (("alert", _units.ALERT_KEYS), ("paging", _units.PAGING_KEYS)):
            listed = {(section, key) for section, names in keys.items() for key in names}
            self.assertEqual(listed, used[kind], kind)
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
            "paging key extending a listed key": (
                _units.PAGING_KEYS,
                page,
                "OnFailureJobMode=fail\n",
                "",
                [refusal(3, "Unit", "OnFailureJobMode", "fail")],
            ),
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
            "alert key of a section off its list": (
                _units.ALERT_KEYS,
                "",
                "",
                "[Socket]\nExecStart=/bin/true\n",
                [refusal(7, "Socket", "ExecStart", "/bin/true")],
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
            "paging waiver, its own section": (
                _units.PAGING_KEYS,
                page,
                "X-DurableServices-Waive=watchdog-missing it sends no readiness\n",
                "",
                [],
            ),
            "paging waiver, wrong section": (
                _units.PAGING_KEYS,
                page,
                "",
                "X-DurableServices-Waive=watchdog-missing kept\n",
                [refusal(7, "Service", "X-DurableServices-Waive", "watchdog-missing kept")],
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

    def test_a_paging_units_restart_holds_only_the_admitted_value(self):
        # Every paging unit that loads a credential holds, at every assignment of a bounded key,
        # only the value the table admits: `Restart=` is `on-failure` and no other, so a restart
        # value that stops a oneshot unit loading, or turns its failure into a success, is refused
        # by key and value (SPEC-066 R2).
        table = _units.PAGING_VALUES
        alert = f"{ALERT_TEMPLATE}@.service"
        loading = examined(
            "service unit(s) that load a credential",
            sorted((unit for unit in subject().services if loads_a_credential(unit)), key=name),
        )
        paging = [unit for unit in loading if unit.name != alert]
        self.assertEqual([r for unit in paging for r in value_refusals(unit, table)], [])
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nType=oneshot\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"
        where = "deploy/systemd/planted.service"

        def refusal(line, section, key, value):
            return (
                f"{where}:{line}: [{section}] {key}={value} is not a value this unit admits for "
                "the key, and is refused"
            )

        # Each plant is (lines added to [Unit], lines added to [Service], refusals). A [Unit] line
        # sits at line 4, so the [Service] lines that follow it move down one.
        plants = {
            "oneshot restart always": (
                "",
                "Restart=always\n",
                [refusal(8, "Service", "Restart", "always")],
            ),
            "oneshot restart on-success": (
                "",
                "Restart=on-success\n",
                [refusal(8, "Service", "Restart", "on-success")],
            ),
            "restart extending the admitted value": (
                "",
                "Restart=on-failure-extra\n",
                [refusal(8, "Service", "Restart", "on-failure-extra")],
            ),
            "restart control": ("", "Restart=on-failure\n", []),
            "start limit interval unbounded": (
                "StartLimitIntervalSec=0\n",
                "",
                [refusal(4, "Unit", "StartLimitIntervalSec", "0")],
            ),
            "start limit burst raised": (
                "StartLimitBurst=1000\n",
                "",
                [refusal(4, "Unit", "StartLimitBurst", "1000")],
            ),
            "restart delay removed": (
                "",
                "RestartSec=0\n",
                [refusal(8, "Service", "RestartSec", "0")],
            ),
            "ordering names another unit": (
                "After=other.service\n",
                "",
                [refusal(4, "Unit", "After", "other.service")],
            ),
            "pull-in names another unit": (
                "Wants=other.service\n",
                "",
                [refusal(4, "Unit", "Wants", "other.service")],
            ),
            "stop signal the server does not drain on": (
                "",
                "KillSignal=SIGKILL\n",
                [refusal(8, "Service", "KillSignal", "SIGKILL")],
            ),
            "stop signal control": ("", "KillSignal=SIGINT\n", []),
            "restart budget and ordering controls": (
                "StartLimitIntervalSec=300\nStartLimitBurst=5\n"
                "After=network-online.target\nWants=network-online.target\n",
                "RestartSec=15\n",
                [],
            ),
        }
        got = {}
        for label in examined("planted unit(s) held to the value table", sorted(plants)):
            unit_lines, service_lines, _ = plants[label]
            got[label] = planted_refusals(
                f"{head}{page}{unit_lines}{run}{loads}{service_lines}",
                lambda unit: value_refusals(unit, table),
            )
        self.assertEqual(got, {label: plants[label][2] for label in plants})
        # A drop-in is read with its unit: a value planted in one is refused by the drop-in's file
        # and line.
        with tempfile.TemporaryDirectory() as scratch:
            folder = Path(scratch) / "deploy" / "systemd"
            (folder / "planted.service.d").mkdir(parents=True)
            (folder / "planted.service").write_text(f"{head}{page}{run}{loads}", encoding="utf-8")
            (folder / "planted.service.d" / "10-planted.conf").write_text(
                "[Service]\nRestart=always\n", encoding="utf-8"
            )
            (planted,) = subject(scratch).services
        self.assertEqual(
            value_refusals(planted, table),
            [
                "deploy/systemd/planted.service.d/10-planted.conf:2: [Service] Restart=always "
                "is not a value this unit admits for the key, and is refused"
            ],
        )

    def test_the_credential_census_admits_the_identity_deny_and_no_wider_peer_list(self):
        """SPEC-354 A3 (R4; ADR-365 D5): a unit that loads a credential may deny the link-local
        range, the alert template included, while the census still refuses a deny of another
        range on a paging unit and any allow list on the alert template."""
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"
        where = "deploy/systemd/planted.service"

        def paging(unit):
            return off_list_refusals(unit, _units.PAGING_KEYS) + value_refusals(
                unit, _units.PAGING_VALUES
            )

        def alert(unit):
            return off_list_refusals(unit, _units.ALERT_KEYS)

        plants = {
            "a paging unit denies the link-local range": (
                paging,
                f"{head}{page}{run}{loads}IPAddressDeny=link-local\n",
                [],
            ),
            "the alert template denies the link-local range": (
                alert,
                f"{head}{run}{loads}IPAddressDeny=link-local\n",
                [],
            ),
            "a paging unit denies a range that misses the endpoint": (
                paging,
                f"{head}{page}{run}{loads}IPAddressDeny=multicast\n",
                [
                    f"{where}:7: [Service] IPAddressDeny=multicast is not a value this unit admits "
                    "for the key, and is refused"
                ],
            ),
            "the alert template holds an allow list": (
                alert,
                f"{head}{run}{loads}IPAddressAllow=localhost\n",
                [
                    f"{where}:6: [Service] IPAddressAllow=localhost is not on this unit's list of "
                    "keys, and is refused"
                ],
            ),
        }
        got = {}
        for label in examined("planted unit(s) held to the credential census", sorted(plants)):
            check, text, _ = plants[label]
            got[label] = planted_refusals(text, check)
        self.assertEqual(got, {label: plants[label][2] for label in plants})

    def test_a_restarting_paging_unit_holds_the_whole_restart_budget(self):
        # A unit that loads a credential and pages, and assigns `Restart=` to anything but `no`,
        # also holds `StartLimitIntervalSec=`, `StartLimitBurst=` and `RestartSec=`: without the
        # start limit the default interval is shorter than the restart delay, so the unit never
        # reaches failed and pages on every restart. A missing key is refused by name (SPEC-066 R2).
        alert = f"{ALERT_TEMPLATE}@.service"
        loading = examined(
            "service unit(s) that load a credential",
            sorted((unit for unit in subject().services if loads_a_credential(unit)), key=name),
        )
        paging = [unit for unit in loading if unit.name != alert]
        self.assertEqual([r for unit in paging for r in restart_budget_refusals(unit)], [])
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nType=oneshot\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"
        where = "deploy/systemd/planted.service"

        def refusal(key):
            return (
                f"{where}: assigns Restart= and holds no {key}=, which the restart budget needs, "
                "and is refused"
            )

        budget = "StartLimitIntervalSec=300\nStartLimitBurst=5\n"
        plants = {
            "restart and delay, no start limit": (
                "",
                "Restart=on-failure\nRestartSec=15\n",
                [refusal("StartLimitIntervalSec"), refusal("StartLimitBurst")],
            ),
            "restart, no budget at all": (
                "",
                "Restart=on-failure\n",
                [
                    refusal("StartLimitIntervalSec"),
                    refusal("StartLimitBurst"),
                    refusal("RestartSec"),
                ],
            ),
            "restart, no burst": (
                "StartLimitIntervalSec=300\n",
                "Restart=on-failure\nRestartSec=15\n",
                [refusal("StartLimitBurst")],
            ),
            "restart, no interval": (
                "StartLimitBurst=5\n",
                "Restart=on-failure\nRestartSec=15\n",
                [refusal("StartLimitIntervalSec")],
            ),
            "restart, no delay": (budget, "Restart=on-failure\n", [refusal("RestartSec")]),
            "restart reset to no, no budget": ("", "Restart=no\n", []),
            "no restart, no budget": ("", "", []),
            "restart with the whole budget": (budget, "Restart=on-failure\nRestartSec=15\n", []),
        }
        got = {}
        for label in examined("planted unit(s) held to the restart budget", sorted(plants)):
            unit_lines, service_lines, _ = plants[label]
            got[label] = planted_refusals(
                f"{head}{page}{unit_lines}{run}{loads}{service_lines}", restart_budget_refusals
            )
        self.assertEqual(got, {label: plants[label][2] for label in plants})

    def test_a_paging_unit_names_no_failure_target_but_the_alert(self):
        # Every `OnFailure=` assignment of a paging unit that loads a credential, in the unit and
        # its drop-ins, is exactly the alert template: a target beside it or in its place is
        # refused by key and value (SPEC-066 R2).
        alert = f"{ALERT_TEMPLATE}@.service"
        loading = examined(
            "service unit(s) that load a credential",
            sorted((unit for unit in subject().services if loads_a_credential(unit)), key=name),
        )
        paging = [unit for unit in loading if unit.name != alert]
        self.assertEqual([r for unit in paging for r in failure_target_refusals(unit)], [])
        head = "[Unit]\nDescription=planted\n"
        page = f"OnFailure={ON_FAILURE}\n"
        run = "[Service]\nExecStart=/bin/true\n"
        loads = f"LoadCredential=telegram-bot-token:{SOCKET}\n"
        where = "deploy/systemd/planted.service"

        def refusal(line, value):
            return (
                f"{where}:{line}: [Unit] OnFailure={value} is not the alert template "
                f"{ON_FAILURE}, and is refused"
            )

        plants = {
            "target control": (page, []),
            "extra target in one assignment": (
                f"OnFailure={ON_FAILURE} other.service\n",
                [refusal(3, f"{ON_FAILURE} other.service")],
            ),
            "extra target in a second assignment": (
                f"{page}OnFailure=other.service\n",
                [refusal(4, "other.service")],
            ),
            "replaced target": ("OnFailure=other.service\n", [refusal(3, "other.service")]),
            "reset after the alert": (f"{page}OnFailure=\n", [refusal(4, "")]),
        }
        got = {}
        for label in examined("planted unit(s) held to the alert target", sorted(plants)):
            got[label] = planted_refusals(
                f"{head}{plants[label][0]}{run}{loads}", failure_target_refusals
            )
        self.assertEqual(got, {label: plants[label][1] for label in plants})
        with tempfile.TemporaryDirectory() as scratch:
            folder = Path(scratch) / "deploy" / "systemd"
            (folder / "planted.service.d").mkdir(parents=True)
            (folder / "planted.service").write_text(f"{head}{page}{run}{loads}", encoding="utf-8")
            (folder / "planted.service.d" / "10-planted.conf").write_text(
                "[Unit]\nOnFailure=other.service\n", encoding="utf-8"
            )
            (planted,) = subject(scratch).services
        self.assertEqual(
            failure_target_refusals(planted),
            [
                "deploy/systemd/planted.service.d/10-planted.conf:2: [Unit] OnFailure="
                f"other.service is not the alert template {ON_FAILURE}, and is refused"
            ],
        )


if __name__ == "__main__":
    unittest.main()


class TheSyncLoginIsTheSyncJobsAlone(unittest.TestCase):
    """SPEC-062 R14: the sync login is loaded by the sync job alone."""

    SYNC_LOGIN = ("anki-sync-username", "anki-sync-password")
    CONF = "20-sync-login.conf"

    def dropin_dir(self, root, ident):
        return Path(root) / "deploy" / "systemd" / f"{JOB_TEMPLATE}@{ident}.service.d"

    def test_the_sync_login_is_loaded_by_the_sync_job_alone(self):
        template = SYSTEMD / f"{JOB_TEMPLATE}@.service"
        text = template.read_text(encoding="utf-8")
        for ident in self.SYNC_LOGIN:
            self.assertNotIn(ident, text, "the template requests a sync credential")
        self.assertNotIn("SYNC_", text)
        # The sync instance's drop-in loads exactly the two, from the socket.
        dropin = self.dropin_dir(REPO, "sync") / self.CONF
        self.assertTrue(dropin.is_file(), f"{dropin.relative_to(REPO)} is missing")
        loaded = re.findall(r"^LoadCredential=(.*)$", dropin.read_text(encoding="utf-8"), re.M)
        self.assertEqual(sorted(loaded), sorted(f"{i}:{SOCKET}" for i in self.SYNC_LOGIN))
        # No other instance has a drop-in that loads one: the held flush's drop-in loads the
        # bot's two credentials, never the sync login (#291).
        others = [
            d
            for d in SYSTEMD.glob(f"{JOB_TEMPLATE}@*.service.d")
            if d.name != dropin.parent.name
            and any(
                login in conf.read_text(encoding="utf-8")
                for conf in d.glob("*.conf")
                for login in self.SYNC_LOGIN
            )
        ]
        self.assertEqual(others, [], "an instance other than sync loads the sync login")
        # The pair list holds them under the sync instance, and under no other unit.
        code = subprocess.run(
            [sys.executable, str(DEPLOY / "scripts" / "credential-pairs.py"), "--root", str(REPO)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(code.returncode, 0, code.stderr)
        pairs = json.loads(code.stdout)["pairs"]
        sync_unit = f"{JOB_TEMPLATE}@sync.service"
        sync_pairs = {p["credential"] for p in pairs if p["unit"] == sync_unit}
        self.assertEqual(sync_pairs, set(self.SYNC_LOGIN))
        self.assertFalse(
            [p for p in pairs if p["unit"] == f"{JOB_TEMPLATE}@.service"], "the template asks"
        )
        for ident in ("liveness", "maintenance"):
            unit = f"{JOB_TEMPLATE}@{ident}.service"
            self.assertFalse([p for p in pairs if p["unit"] == unit], f"{ident} asks")
        examined("pair(s) listed", pairs)

    def test_the_bots_credentials_are_loaded_by_the_held_flush_alone(self):
        # Only the held flush sends to the owner's chat (#291): its instance's drop-in loads the
        # bot's token and the owner's id, and no other job instance, nor the template, asks.
        bot = ("owner-user-id", "telegram-bot-token")
        text = (SYSTEMD / f"{JOB_TEMPLATE}@.service").read_text(encoding="utf-8")
        for ident in bot:
            self.assertNotIn(ident, text, "the template requests a bot credential")
        own = self.dropin_dir(REPO, "held_flush") / "20-bot-credentials.conf"
        self.assertTrue(own.is_file(), f"{own.relative_to(REPO)} is missing")
        loaded = re.findall(r"^LoadCredential=(.*)$", own.read_text(encoding="utf-8"), re.M)
        self.assertEqual(sorted(loaded), sorted(f"{i}:{SOCKET}" for i in bot))
        folders = examined(
            "job instance drop-in directories", sorted(SYSTEMD.glob(f"{JOB_TEMPLATE}@*.service.d"))
        )
        others = [
            folder.name
            for folder in folders
            if folder != own.parent
            and any(
                ident in conf.read_text(encoding="utf-8")
                for conf in folder.glob("*.conf")
                for ident in bot
            )
        ]
        self.assertEqual(others, [], "an instance other than the held flush loads a bot credential")

    def test_the_effective_check_accepts_the_shipped_drop_in_and_no_other(self):
        checker = DEPLOY / "scripts" / "effective-check.py"
        contract = json.loads((DEPLOY / "rail-contract.json").read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "deploy").mkdir()
            contract["values"] = contract["values"][:1]
            (root / "deploy" / "rail-contract.json").write_text(json.dumps(contract))
            source = self.dropin_dir(REPO, "sync") / self.CONF
            self.assertTrue(source.is_file(), f"{source.relative_to(REPO)} is missing")
            shipped = self.dropin_dir(root, "sync")
            shipped.mkdir(parents=True)
            (shipped / self.CONF).write_text(source.read_text(encoding="utf-8"))
            unit = "[Unit]\nDescription=x\n\n[Service]\nType=oneshot\n"
            head = f"/etc/systemd/system/{JOB_TEMPLATE}@.service"
            here = f"/etc/systemd/system/{JOB_TEMPLATE}@sync.service.d/{self.CONF}"
            elsewhere = f"/etc/systemd/system/{JOB_TEMPLATE}@sync.service.d/99-host.conf"
            other = f"/etc/systemd/system/{JOB_TEMPLATE}@liveness.service.d/{self.CONF}"
            body = source.read_text(encoding="utf-8")

            def check(dropin_path):
                out = f"# {head}\n{unit}\n# {dropin_path}\n{body}"
                return subprocess.run(
                    [sys.executable, str(checker), "--root", str(root)],
                    input=out,
                    capture_output=True,
                    text=True,
                    check=False,
                )

            accepted = check(here)
            self.assertEqual(accepted.returncode, 0, accepted.stdout + accepted.stderr)
            # A drop-in the release does not ship is refused, whatever it holds.
            for path in (elsewhere, other):
                refused = check(path)
                self.assertEqual(refused.returncode, 1, path)
                self.assertIn("a drop-in that is not the rail's", refused.stdout + refused.stderr)

    def test_a_shipped_drop_in_name_is_admitted_beside_the_rails_own_alone(self):
        checker = DEPLOY / "scripts" / "effective-check.py"
        contract = json.loads((DEPLOY / "rail-contract.json").read_text(encoding="utf-8"))
        body = (self.dropin_dir(REPO, "sync") / self.CONF).read_text(encoding="utf-8")
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "deploy").mkdir()
            contract["values"] = contract["values"][:1]
            (root / "deploy" / "rail-contract.json").write_text(json.dumps(contract))
            shipped = self.dropin_dir(root, "sync")
            shipped.mkdir(parents=True)
            (shipped / self.CONF).write_text(body)
            unit = "[Unit]\nDescription=x\n\n[Service]\nType=oneshot\n"
            head = f"/etc/systemd/system/{JOB_TEMPLATE}@.service"
            verdicts = {}
            for where in (
                "/etc/systemd/system",
                "/run/systemd/system",
                "/etc/systemd/system.control",
                "/home/user/.config/systemd/user",
            ):
                path = f"{where}/{JOB_TEMPLATE}@sync.service.d/{self.CONF}"
                done = subprocess.run(
                    [sys.executable, str(checker), "--root", str(root)],
                    input=f"# {head}\n{unit}\n# {path}\n{body}",
                    capture_output=True,
                    text=True,
                    check=False,
                )
                verdicts[where] = done.returncode
            self.assertEqual(
                [w for w, rc in verdicts.items() if rc == 0], ["/etc/systemd/system"], verdicts
            )

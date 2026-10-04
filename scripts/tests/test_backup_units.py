"""DeckStreak's database is replicated by Litestream, copied daily and restored weekly by its own
units, inside the declared backups window (SPEC-064 A1 to A7; ADR-010, ADR-064).

The units are read as systemd reads them (`_units.py`); the daily copy runs over a synthetic database
in a temporary tree, in process; the drill runs as its unit runs it, with a stub `litestream` that
records its arguments and writes a synthetic copy. Nothing here reads a host, a bucket or a secret.
"""

import hashlib
import importlib.util
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import tarfile
import threading
import time
import unittest
import unittest.mock
from pathlib import Path

from _support import REPO, examined
from _units import STOPS_A_START, env_assignments, load_subject, service_type, size_bytes
from test_deploy_templates import HARDENING

DEPLOY = REPO / "deploy"
BACKUP_SCRIPT = DEPLOY / "scripts" / "backup.py"
DRILL_SCRIPT = DEPLOY / "scripts" / "restore-drill.sh"
LITESTREAM_CONFIG = DEPLOY / "litestream.yml"
BUDGET = DEPLOY / "host-budget.json"
ENV_EXAMPLE = DEPLOY / "deck-streak.env.example"
PRIVACY_JSON = REPO / "privacy.json"
GOLDENS = REPO / "tools" / "parity-oracle" / "goldens"
RELEASE_ROOT = "/usr/local/lib/deck-streak/current"
ALERT = "deck-streak-alert@%n.service"

LITESTREAM = "deck-streak-litestream.service"
BACKUP = "deck-streak-backup.service"
BACKUP_TIMER = "deck-streak-backup.timer"
DRILL = "deck-streak-restore-drill.service"
DRILL_TIMER = "deck-streak-restore-drill.timer"
# The sync server and the window that stops it for the snapshot's copy (SPEC-337 R5; ADR-347 D12).
SYNC_SERVER = "deck-streak-sync-server.service"
WINDOW = "deck-streak-sync-snapshot.service"
SNAPSHOT_FILES = ("collection.anki2", "media.db")
NEW_SERVICES = (LITESTREAM, BACKUP, DRILL)
# R6: the two slots, exactly, in UTC as the neutral templates are written (SPEC-032 R4).
BACKUP_CALENDAR = "*-*-* 03:24:00 UTC"
DRILL_CALENDAR = "Sun *-*-* 06:24:00 UTC"
SLOT_MINUTE = 24
# ADR-011's reserved minutes of the hour, and the synthetic list standing in for the private rail's
# reserved-slot list (SPEC-053 R2), which CI never reads.
RESERVED_MINUTES = (0, 25, 39)
SYNTHETIC_RESERVED = (("03", 13), ("05", 48), ("06", 31))
# Litestream 0.5 schemes that are off the host; a file path or an empty scheme is on it.
OFF_HOST_SCHEMES = {"gs", "s3", "abs", "sftp", "nats"}
SNAPSHOT_HOURS_RE = re.compile(r"^(\d+)h$")

SCHEMA = """
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY, description TEXT NOT NULL, installed_on TEXT NOT NULL DEFAULT '',
    success BOOLEAN NOT NULL, checksum BLOB NOT NULL DEFAULT x'', execution_time BIGINT NOT NULL
        DEFAULT 0);
CREATE TABLE IF NOT EXISTS reviews (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
"""


def load_backup():
    spec = importlib.util.spec_from_file_location("deck_streak_backup", BACKUP_SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(module)
    return module


def make_database(path, version=7, rows=50, wal=True):
    """A synthetic database at `path`: WAL mode, the migrations table at `version`, some rows."""
    connection = sqlite3.connect(path)
    if wal:
        connection.execute("PRAGMA journal_mode=WAL")
    connection.executescript(SCHEMA)
    connection.execute("DELETE FROM _sqlx_migrations")
    connection.execute(
        "INSERT INTO _sqlx_migrations (version, description, success) VALUES (?, 'm', 1)",
        (version,),
    )
    connection.executemany(
        "INSERT INTO reviews (note) VALUES (?)", [(f"row {n}",) for n in range(rows)]
    )
    connection.commit()
    connection.close()
    return path


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def integrity(path):
    connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        return connection.execute("PRAGMA integrity_check").fetchall()
    finally:
        connection.close()


def make_sync_base(root, users=("owner", "staging"), cards=3):
    """A stopped sync server's store, as the engine lays it out: per user a collection in WAL mode
    with a cards table, a media index and one media file."""
    base = Path(root) / "sync-base"
    for user in users:
        folder = base / user
        (folder / "media").mkdir(parents=True)
        collection = sqlite3.connect(folder / "collection.anki2")
        collection.execute("PRAGMA journal_mode=WAL")
        collection.execute("CREATE TABLE cards (id INTEGER PRIMARY KEY, nid INTEGER NOT NULL)")
        collection.executemany("INSERT INTO cards (nid) VALUES (?)", [(n,) for n in range(cards)])
        collection.commit()
        collection.close()
        media = sqlite3.connect(folder / "media.db")
        media.execute("PRAGMA journal_mode=WAL")
        media.execute("CREATE TABLE media (fname TEXT PRIMARY KEY, csum TEXT)")
        media.execute("INSERT INTO media VALUES ('a.jpg', 'x')")
        media.commit()
        media.close()
        (folder / "media" / "a.jpg").write_bytes(b"image bytes of " + user.encode())
    return base


def names_in(directory, prefix):
    return (
        sorted(p.name for p in Path(directory).glob(f"{prefix}*"))
        if Path(directory).is_dir()
        else []
    )


def unit_named(subject, name):
    """The unit of that name, or an assertion failure that says it is absent."""
    unit = subject.units.get(name)
    if unit is None:
        raise AssertionError(f"{name} is absent from deploy/systemd")
    return unit


def copies_of(directory):
    return sorted(p.name for p in Path(directory).iterdir())


# --- the reader the census tests share ------------------------------------------------------


def yaml_lines(text):
    """(indent, key, value, line) for each `key: value` line of a flat-enough YAML file; comments and
    list markers are read, so `- path: x` is the key `path` one level in."""
    out = []
    for number, raw in enumerate(text.splitlines(), start=1):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip())
        body = raw.strip()
        if body.startswith("- "):
            body = body[2:]
            indent += 2
        key, colon, value = body.partition(":")
        if colon:
            out.append((indent, key.strip(), value.strip(), number))
    return out


def litestream_facts(text):
    """The replica URLs, the replica-level retention keys, the `replicas:` lists and the database
    paths of a Litestream configuration, and its global snapshot interval and retention in hours."""
    lines = yaml_lines(text)
    facts = {
        "urls": [],
        "replica_blocks": 0,
        "replicas_lists": 0,
        "dbs": 0,
        "replica_retention": [],
        "interval_h": None,
        "retention_h": None,
    }
    top = None
    in_replica_indent = None
    for indent, key, value, number in lines:
        if indent == 0:
            top = key
            in_replica_indent = None
        if top == "snapshot" and indent > 0:
            match = SNAPSHOT_HOURS_RE.match(value)
            if key == "interval" and match:
                facts["interval_h"] = int(match.group(1))
            if key == "retention" and match:
                facts["retention_h"] = int(match.group(1))
        if top == "dbs":
            if key == "path":
                facts["dbs"] += 1
            if key == "replicas":
                facts["replicas_lists"] += 1
            if key == "replica":
                facts["replica_blocks"] += 1
                in_replica_indent = indent
            elif in_replica_indent is not None and indent <= in_replica_indent:
                in_replica_indent = None
            if key == "url":
                facts["urls"].append(value)
            if key.startswith("retention") and indent > 0:
                facts["replica_retention"].append(number)
    return facts


def url_scheme(url):
    return url.partition("://")[0] if "://" in url else ""


def calendar_minutes(calendar):
    """The minutes of the hour an `OnCalendar=` fires at, or None when it names a wildcard or a
    step (a timer that fires every few minutes has no one slot)."""
    time = re.search(r"(\S+):(\S+):(\S+)", calendar)
    if not time:
        return None
    minute = time.group(2)
    return [int(minute)] if minute.isdigit() else None


def others_minutes():
    """Every minute of the hour the other schedules hold, by source: the predecessor's cron fields
    and its sync ticks, the reserved minutes, the synthetic list and the job table's timers."""
    sources = {}
    schedule = json.loads((GOLDENS / "predecessor_schedule.json").read_text(encoding="utf-8"))
    case = next(c for c in schedule["cases"] if c["class"] == "defaults")
    for job in case["output"]["jobs"]:
        field = str(job["fields"]["minute"])
        sources[f"predecessor:{job['id']}"] = {int(m) for m in field.split(",")}
    interval = case["output"]["sync_interval_min"]
    ticks = json.loads((GOLDENS / "tick_minutes.json").read_text(encoding="utf-8"))
    offset = next(
        c
        for c in ticks["cases"]
        if c["class"] == "alignable" and c["input"]["interval_min"] == interval
    )
    sources["predecessor:sync ticks"] = set(offset["output"])
    sources["reserved (ADR-011)"] = set(RESERVED_MINUTES)
    for hour, minute in SYNTHETIC_RESERVED:
        sources[f"synthetic reserved {hour}:{minute:02d}"] = {minute}
    subject = load_subject(REPO)
    for timer in subject.timers:
        if timer.name.startswith("deck-streak-job@"):
            calendar = timer.last("Timer", "OnCalendar")
            found = calendar_minutes(calendar)
            if found:
                sources[f"job table:{timer.name}"] = set(found)
    return sources


def slot_collisions(slots, sources):
    """The (slot, source) pairs that share a minute of the hour."""
    return [
        (name, source)
        for name, minute in slots.items()
        for source, minutes in sources.items()
        if minute in minutes
    ]


class BackupUnits(unittest.TestCase):
    """A1, A5, A6 and A7 read the committed units and files; A2 to A4 run the scripts."""

    def test_the_units_keep_three_copies_one_replica_and_a_drill(self):
        subject = load_subject(REPO)
        backup = load_backup()
        services = examined("new services", [subject.units.get(n) for n in NEW_SERVICES])
        self.assertNotIn(None, services, "every unit SPEC-064 R1, R4, R5 names exists")
        budget = json.loads(BUDGET.read_text(encoding="utf-8"))["units"]

        litestream, backup_unit, drill = (subject.units[n] for n in NEW_SERVICES)
        # R1: the daemon runs `litestream replicate -config` with the release's own file.
        exec_start = litestream.values("Service", "ExecStart")
        self.assertEqual(len(exec_start), 1)
        self.assertRegex(
            exec_start[0],
            rf"^/\S*litestream replicate -config {re.escape(RELEASE_ROOT)}/deploy/litestream\.yml$",
        )
        self.assertEqual(service_type(litestream), "exec")
        self.assertEqual(litestream.last("Service", "Restart"), "on-failure")
        self.assertEqual(litestream.last("Service", "RestartSec"), "15")
        self.assertEqual(litestream.last("Unit", "StartLimitIntervalSec"), "300")
        self.assertEqual(litestream.last("Unit", "StartLimitBurst"), "5")
        self.assertEqual(litestream.last("Install", "WantedBy"), "multi-user.target")
        self.assertIsNone(subject.units.get("deck-streak-litestream.timer"))
        # R4, R5: each oneshot runs its own script from the release.
        self.assertEqual(
            backup_unit.values("Service", "ExecStart"),
            [f"/usr/bin/python3 {RELEASE_ROOT}/deploy/scripts/backup.py"],
        )
        self.assertEqual(
            drill.values("Service", "ExecStart"),
            [f"{RELEASE_ROOT}/deploy/scripts/restore-drill.sh"],
        )
        for unit in (backup_unit, drill):
            self.assertEqual(service_type(unit), "oneshot")
        # R6: the triggers, asserted exactly, and each timer starts the service of its own name.
        timers = {t.name: t for t in subject.timers if t.name in (BACKUP_TIMER, DRILL_TIMER)}
        self.assertEqual(sorted(timers), sorted([BACKUP_TIMER, DRILL_TIMER]))
        self.assertEqual(timers[BACKUP_TIMER].values("Timer", "OnCalendar"), [BACKUP_CALENDAR])
        self.assertEqual(timers[DRILL_TIMER].values("Timer", "OnCalendar"), [DRILL_CALENDAR])
        for timer in timers.values():
            self.assertFalse(timer.assigned("Timer", "Unit"))
            self.assertEqual(timer.last("Install", "WantedBy"), "timers.target")
        # R7: every new unit's OnFailure, UMask, private temporary directory and budget entry.
        for unit in services:
            self.assertEqual(unit.values("Unit", "OnFailure"), [ALERT], unit.name)
            for key, want in HARDENING.items():
                self.assertEqual(unit.last("Service", key), want, f"{unit.name} {key}")
            self.assertEqual(unit.last("Service", "StateDirectory"), "deck-streak")
            self.assertEqual(unit.last("Service", "MemoryHigh"), budget[unit.name]["memory_high"])
            self.assertEqual(unit.last("Service", "MemoryMax"), budget[unit.name]["memory_max"])
            self.assertLess(
                size_bytes(unit.last("Service", "MemoryHigh")),
                size_bytes(unit.last("Service", "MemoryMax")),
            )
            # The two that reach the bucket expand it from the required settings file.
            self.assertEqual(
                unit.values("Service", "EnvironmentFile"),
                [] if unit.name == BACKUP else ["/etc/deck-streak/deck-streak.env"],
                unit.name,
            )
        # The three backup copies: the script keeps three, once a day.
        self.assertEqual(backup.KEEP, 3)
        self.assertRegex(BACKUP_CALENDAR, r"^\*-\*-\* \d\d:\d\d:00 UTC$")

        # One replica per database, in 0.5's form, off the host by the URL's scheme.
        facts = litestream_facts(LITESTREAM_CONFIG.read_text(encoding="utf-8"))
        examined("replica urls", facts["urls"])
        self.assertEqual(facts["dbs"], 1)
        self.assertEqual(facts["replica_blocks"], facts["dbs"])
        self.assertEqual(len(facts["urls"]), facts["dbs"])
        self.assertEqual(facts["replicas_lists"], 0, "0.5 takes `replica:`, not a `replicas:` list")
        for url in facts["urls"]:
            self.assertIn(url_scheme(url), OFF_HOST_SCHEMES, url)
        # Exactly one drill and one Litestream unit among every unit the census reads.
        names = examined("units", subject.units)
        self.assertEqual([n for n in names if "restore" in n and n.endswith(".service")], [DRILL])
        self.assertEqual(
            [
                n
                for n in names
                if n.endswith(".service")
                and "litestream replicate"
                in " ".join(subject.units[n].values("Service", "ExecStart"))
            ],
            [LITESTREAM],
        )
        # A census that judges: a `replicas:` list, a file replica and a second replica are refused.
        planted = LITESTREAM_CONFIG.read_text(encoding="utf-8").replace(
            "    replica:\n", "    replicas:\n"
        )
        self.assertEqual(litestream_facts(planted)["replicas_lists"], 1)
        self.assertEqual(url_scheme("/var/backups/x"), "")

    def test_the_backup_copies_checks_and_keeps_three(self):
        backup = load_backup()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / "state"
            state.mkdir()
            database = make_database(state / "deck_streak.db")
            # What a copy must never contain: the collection copy and a credential.
            (state / "collection.anki2").write_bytes(b"COLLECTION-COPY")
            (root / "creds").mkdir()
            (root / "creds" / "token").write_bytes(b"CREDENTIAL")
            backups = state / "backups"

            stop = threading.Event()

            def writer():
                connection = sqlite3.connect(database, timeout=30)
                n = 0
                while not stop.is_set():
                    connection.execute("INSERT INTO reviews (note) VALUES (?)", (f"w{n}",))
                    connection.commit()
                    n += 1
                connection.close()

            thread = threading.Thread(target=writer)
            thread.start()
            try:
                stamps = [f"2030-01-0{day}T03:24:00Z" for day in range(1, 6)]
                for stamp in stamps:
                    code = backup.main(
                        ["--database", str(database), "--backups", str(backups)],
                        now=backup.parse_stamp(stamp),
                    )
                    self.assertEqual(code, 0)
            finally:
                stop.set()
                thread.join()

            kept = examined("daily copies kept", copies_of(backups))
            self.assertEqual(len(kept), 3, "the newest three stay")
            self.assertEqual(
                kept,
                [f"deck_streak-2030010{day}T032400Z.db" for day in (3, 4, 5)],
                "older copies are removed, the newest kept",
            )
            for name in kept:
                path = backups / name
                self.assertEqual(integrity(path), [("ok",)])
                self.assertEqual(path.stat().st_mode & 0o777, 0o600)
                connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
                self.assertGreaterEqual(
                    connection.execute("SELECT COUNT(*) FROM reviews").fetchone()[0], 50
                )
                self.assertEqual(
                    connection.execute("SELECT MAX(version) FROM _sqlx_migrations").fetchone()[0], 7
                )
                connection.close()
                data = path.read_bytes()
                self.assertNotIn(b"COLLECTION-COPY", data)
                self.assertNotIn(b"CREDENTIAL", data)
            # Beside the backups directory nothing is left: no temporary file, and no WAL sidecar.
            self.assertEqual(
                sorted(p.name for p in state.iterdir()),
                ["backups", "collection.anki2", "deck_streak.db"]
                + sorted(p.name for p in state.iterdir() if p.name.startswith("deck_streak.db-")),
            )
            self.assertEqual([p.name for p in backups.iterdir() if not p.name.endswith(".db")], [])

    def test_a_failed_integrity_check_fails_the_backup_and_keeps_the_old_copies(self):
        backup = load_backup()
        with tempfile.TemporaryDirectory() as tmp:
            state = Path(tmp)
            database = make_database(state / "deck_streak.db")
            backups = state / "backups"
            for day in (1, 2, 3):
                self.assertEqual(
                    backup.main(
                        ["--database", str(database), "--backups", str(backups)],
                        now=backup.parse_stamp(f"2030-02-0{day}T03:24:00Z"),
                    ),
                    0,
                )
            before = {name: digest(backups / name) for name in copies_of(backups)}
            examined("previous copies", before)
            self.assertEqual(len(before), 3)

            original = backup.integrity_ok
            backup.integrity_ok = lambda path: False
            try:
                code = backup.main(
                    ["--database", str(database), "--backups", str(backups)],
                    now=backup.parse_stamp("2030-02-04T03:24:00Z"),
                )
            finally:
                backup.integrity_ok = original
            self.assertEqual(code, 1, "a failed check exits non-zero")
            after = {name: digest(backups / name) for name in copies_of(backups)}
            self.assertEqual(after, before, "the previous three copies are untouched, none pruned")
            self.assertEqual(
                sorted(p.name for p in state.iterdir() if "tmp" in p.name.lower()),
                [],
                "the failed copy is removed",
            )

            # The real check refuses a file that is not a database, and a missing source fails too.
            garbage = state / "garbage.db"
            garbage.write_bytes(b"not a database" * 100)
            self.assertFalse(backup.integrity_ok(garbage))
            self.assertTrue(backup.integrity_ok(database))
            self.assertEqual(
                backup.main(
                    ["--database", str(state / "absent.db"), "--backups", str(backups)],
                    now=backup.parse_stamp("2030-02-05T03:24:00Z"),
                ),
                1,
            )
            self.assertEqual({n: digest(backups / n) for n in copies_of(backups)}, before)

    def test_the_drill_restores_checks_and_removes_both_copies(self):
        script = DRILL_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("integrity_check", script)
        self.assertIn("_sqlx_migrations", script)
        backup = load_backup()

        def run_drill(
            replica="good",
            replica_version=7,
            daily="good",
            daily_version=7,
            live_version=7,
            stub_exit=0,
            with_daily=True,
        ):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                scratch = root / "tmp"
                scratch.mkdir()
                state = root / "state"
                state.mkdir()
                live = make_database(state / "deck_streak.db", version=live_version)
                backups = state / "backups"
                backups.mkdir()
                copy = None
                if with_daily:
                    copy = backups / "deck_streak-2030-01-01.db"
                    if daily == "good":
                        make_database(copy, version=daily_version)
                    else:
                        copy.write_bytes(b"broken copy" * 500)
                    copy.chmod(0o600)
                seed = root / "replica-seed.db"
                if replica == "good":
                    make_database(seed, version=replica_version)
                else:
                    seed.write_bytes(b"broken replica" * 500)
                argv_log = root / "argv.json"
                stub = root / "litestream"
                stub.write_text(
                    "#!/usr/bin/env python3\n"
                    "import json, shutil, sys\n"
                    f"json.dump(sys.argv[1:], open({str(argv_log)!r}, 'w'))\n"
                    "args = sys.argv[1:]\n"
                    "if '-o' in args:\n"
                    f"    shutil.copyfile({str(seed)!r}, args[args.index('-o') + 1])\n"
                    f"sys.exit({stub_exit})\n",
                    encoding="utf-8",
                )
                stub.chmod(0o755)
                config = root / "litestream.yml"
                config.write_text("dbs: []\n", encoding="utf-8")
                env = dict(os.environ, TMPDIR=str(scratch), PYTHONDONTWRITEBYTECODE="1")
                result = subprocess.run(
                    [
                        "bash",
                        str(DRILL_SCRIPT),
                        "--litestream",
                        str(stub),
                        "--config",
                        str(config),
                        "--database",
                        str(live),
                        "--backups",
                        str(backups),
                    ],
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=120,
                    check=False,
                )
                argv = json.loads(argv_log.read_text()) if argv_log.exists() else None
                return {
                    "code": result.returncode,
                    "argv": argv,
                    "left": sorted(p.name for p in scratch.iterdir()),
                    "daily": copy.exists() if copy else None,
                    "config": str(config),
                    "live": str(live),
                    "scratch": str(scratch),
                    "stderr": result.stderr,
                }

        good = run_drill()
        self.assertEqual(good["code"], 0, good["stderr"])
        argv = good["argv"]
        self.assertIsNotNone(argv, "the drill ran litestream")
        self.assertEqual(argv[:3], ["restore", "-config", good["config"]])
        self.assertEqual(argv[3], "-o")
        self.assertTrue(argv[4].startswith(good["scratch"] + "/"), "a private temporary path")
        self.assertNotEqual(argv[4], good["live"], "never the live database")
        self.assertEqual(argv[5:], [good["live"]])
        self.assertEqual(good["left"], [], "both copies are removed on exit")
        self.assertTrue(good["daily"], "the daily copy itself is left where it is")

        # Each way a step fails, the drill exits non-zero and still removes both copies.
        failing = {
            "replica fails its integrity check": run_drill(replica="bad"),
            "daily copy fails its integrity check": run_drill(daily="bad"),
            "replica is behind the live database": run_drill(replica_version=6),
            "replica is ahead of the live database": run_drill(replica_version=8),
            "daily copy is ahead of the live database": run_drill(daily_version=8),
            "litestream exits non-zero": run_drill(stub_exit=1),
            "no daily copy exists": run_drill(with_daily=False),
        }
        cases = dict(examined("failing drills", failing.items()))
        for name, outcome in cases.items():
            self.assertNotEqual(outcome["code"], 0, name)
            self.assertEqual(outcome["left"], [], f"{name}: nothing is left behind")
        # A daily copy from before the last migration is older, not broken.
        self.assertEqual(run_drill(daily_version=6)["code"], 0)
        self.assertIsNotNone(backup)

    def test_the_copies_fit_the_declared_backups_window(self):
        backup = load_backup()
        text = LITESTREAM_CONFIG.read_text(encoding="utf-8")
        facts = litestream_facts(text)
        window = json.loads(PRIVACY_JSON.read_text(encoding="utf-8"))["backups"]
        match = re.fullmatch(r"P(\d+)D", window["retention"])
        self.assertIsNotNone(match, "the window is whole days")
        window_hours = int(match.group(1)) * 24
        self.assertEqual(window["config"], "deploy/litestream.yml")
        self.assertEqual(set(window), {"config", "retention"})
        self.assertEqual(facts["interval_h"], 24)
        self.assertEqual(facts["retention_h"], 48)
        replica_hours = facts["interval_h"] + facts["retention_h"]
        self.assertLessEqual(replica_hours, window_hours)
        # The daily copies: one a day, the newest KEEP kept, so the oldest is KEEP days old.
        subject = load_subject(REPO)
        calendar = unit_named(subject, BACKUP_TIMER).last("Timer", "OnCalendar")
        self.assertTrue(calendar.startswith("*-*-* "), "the backup runs every day")
        self.assertLessEqual(backup.KEEP * 24, window_hours)
        # No replica sets its own retention (0.5 ignores it), and a planted one is caught.
        self.assertEqual(examined("retention keys", [facts["replica_retention"]])[0], [])
        planted = text.replace("      url:", "      retention: 24h\n      url:")
        self.assertNotEqual(litestream_facts(planted)["replica_retention"], [])
        # PRIVACY.md tells the owner both copies and their window.
        policy = (REPO / "PRIVACY.md").read_text(encoding="utf-8")
        self.assertRegex(policy, r"(?is)daily cop(y|ies).{0,300}3 days|3 days.{0,300}daily cop")
        self.assertRegex(policy, r"(?is)replica.{0,300}72 hours|72 hours.{0,300}replica")

    def test_the_backup_and_drill_slots_are_off_every_other_slot(self):
        subject = load_subject(REPO)
        slots = {}
        for timer_name in (BACKUP_TIMER, DRILL_TIMER):
            found = calendar_minutes(unit_named(subject, timer_name).last("Timer", "OnCalendar"))
            self.assertEqual(found, [SLOT_MINUTE], timer_name)
            slots[timer_name] = found[0]
        sources = others_minutes()
        examined("slots compared", [(s, o) for s in slots for o in sources])
        self.assertIn("predecessor:sync ticks", sources)
        self.assertEqual(sources["predecessor:sync ticks"], {2, 17, 32, 47})
        self.assertTrue(any(n.startswith("job table:") for n in sources), "the job table is read")
        self.assertEqual(slot_collisions(slots, sources), [])
        # The comparison judges: a slot on the sync ticks, a reserved minute and a job's minute.
        for planted in (17, 25, 14, 5, 39):
            self.assertNotEqual(slot_collisions({"planted": planted}, sources), [], planted)

    def test_no_backup_file_names_a_private_value(self):
        files = [
            LITESTREAM_CONFIG,
            ENV_EXAMPLE,
            BACKUP_SCRIPT,
            DRILL_SCRIPT,
            *(
                DEPLOY / "systemd" / n
                for n in (LITESTREAM, BACKUP, BACKUP_TIMER, DRILL, DRILL_TIMER, WINDOW)
            ),
        ]
        examined("backup files", files)
        for path in files:
            self.assertTrue(path.is_file(), path)
        text = LITESTREAM_CONFIG.read_text(encoding="utf-8")
        facts = litestream_facts(text)
        self.assertEqual(len(examined("replica urls", facts["urls"])), 1)
        self.assertRegex(facts["urls"][0], r"^gs://\$\{[A-Z][A-Z0-9_]*\}(/[A-Za-z0-9._-]*)*$")
        variable = re.search(r"\$\{([A-Z0-9_]+)\}", facts["urls"][0]).group(1)
        settings = {key: value for _, key, value in env_assignments(ENV_EXAMPLE)}
        self.assertIn(variable, settings, "the example names the setting, with a neutral value")
        self.assertRegex(settings[variable], r"^[a-z][a-z0-9-]*$")
        self.assertIn("example", settings[variable])
        # No file names a project id, a host name or an address of its own.
        forbidden = re.compile(
            r"(?i)\b(?!127\.)(?:\d{1,3}\.){3}\d{1,3}\b|\.internal\b|\.googleapis\.com|\.appspot\.com|"
            r"/opt/|projects/[a-z][a-z0-9-]{5,}|gs://[a-z0-9]"
        )
        for path in files:
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                code = line.split("#", 1)[0] if path.suffix in {".yml", ".sh", ".py"} else line
                self.assertIsNone(forbidden.search(code), f"{path.name}:{number}")
        # A planted bucket and a planted address are refused by the census and by the public scrub.
        planted = "gs://" + "acme-private-bucket"
        self.assertIsNotNone(forbidden.search(planted))
        self.assertIsNotNone(forbidden.search("10." + "0.0.7"))
        with tempfile.TemporaryDirectory() as tmp:
            subject = Path(tmp) / "planted.yml"
            address = "10." + "0.0.7"
            subject.write_text(f"replica:\n  url: {planted}\n  host: {address}\n", encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    str(REPO / "scripts" / "public-scrub.py"),
                    "--root",
                    str(REPO),
                    "--no-tree",
                    "--subject",
                    tmp,
                ],
                capture_output=True,
                text=True,
                timeout=120,
                check=False,
                env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            )
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        # And the committed files pass the same scrub, one file per call from a fresh directory.
        for path in files:
            with tempfile.TemporaryDirectory() as tmp:
                shutil.copyfile(path, Path(tmp) / path.name)
                result = subprocess.run(
                    [
                        sys.executable,
                        str(REPO / "scripts" / "public-scrub.py"),
                        "--root",
                        str(REPO),
                        "--no-tree",
                        "--subject",
                        tmp,
                    ],
                    capture_output=True,
                    text=True,
                    timeout=120,
                    check=False,
                    env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
                )
                self.assertEqual(
                    result.returncode, 0, f"{path.name}: {result.stdout}{result.stderr}"
                )


class SyncWindow(unittest.TestCase):
    """SPEC-337 R5, A7 and A8: the snapshot's copy runs in a window with the sync server stopped,
    inside the daily backup's run, and the server is started again whatever the copy does
    (ADR-347 D12; the model is formal/tla/SyncSnapshotWindow)."""

    STAMPS = (
        "2030-01-01T03:00:00Z",
        "2030-01-02T03:00:00Z",
        "2030-01-03T03:00:00Z",
        "2030-01-04T03:00:00Z",
    )

    def window(self, backup, base, snapshots, stamp="2030-01-01T03:00:00Z"):
        argv = ["--sync-window", "--sync-base", str(base), "--snapshots", str(snapshots)]
        return backup.main(argv, now=backup.parse_stamp(stamp))

    def daily(self, backup, root, snapshots, stamp, environ):
        database = root / "deck_streak.db"
        if not database.exists():
            make_database(database)
        argv = [
            "--database",
            str(database),
            "--backups",
            str(root / "backups"),
            "--snapshots",
            str(snapshots),
        ]
        with unittest.mock.patch.dict(os.environ, environ, clear=False):
            return backup.main(argv, now=backup.parse_stamp(stamp))

    def offsite(self, root):
        """A stand-in for the rail's object-store command: it records its arguments."""
        log = root / "offsite.jsonl"
        stub = root / "object-store-copy"
        stub.write_text(
            "#!/usr/bin/env python3\nimport json, sys\n"
            f"open({str(log)!r}, 'a').write(json.dumps(sys.argv[1:]) + '\\n')\n",
            encoding="utf-8",
        )
        stub.chmod(0o755)
        environ = {
            "DECKSTREAK_SNAPSHOT_COPY": f"{sys.executable} {stub} --no-clobber",
            "DECKSTREAK_SNAPSHOT_BUCKET": "example://deck-streak-example-snapshot/",
        }
        return environ, log

    def test_the_window_stops_the_server_and_starts_it_again_whatever_the_copy_does(self):
        subject = load_subject(REPO)
        window = subject.units.get(WINDOW)
        self.assertIsNotNone(window, "the window is a unit of its own (ADR-347 D12)")
        server = subject.units[SYNC_SERVER]
        budget = json.loads(BUDGET.read_text(encoding="utf-8"))["units"]
        self.assertEqual(service_type(window), "oneshot")
        self.assertEqual(
            window.values("Service", "ExecStart"),
            [f"/usr/bin/python3 {RELEASE_ROOT}/deploy/scripts/backup.py --sync-window"],
        )
        # The stop goes first: Conflicts= adds it to the start's transaction, and a stop is ordered
        # before a start of a unit it is ordered against (systemd.unit(5)).
        self.assertEqual(window.values("Unit", "Conflicts"), [SYNC_SERVER])
        self.assertEqual(window.values("Unit", "After"), [SYNC_SERVER])
        self.assertEqual(window.values("Unit", "Before"), [BACKUP])
        # Started again whatever the copy does: PID 1 enqueues the server's start on success and on
        # failure alike, so no grant is needed.
        self.assertEqual(window.values("Unit", "OnSuccess"), [SYNC_SERVER])
        self.assertEqual(window.values("Unit", "OnFailure"), [SYNC_SERVER, ALERT])
        # A condition is checked after the stop and a skipped start fires neither, which would leave
        # the server stopped: the window carries none, and is gated by being enabled.
        for assignment in window.assignments:
            self.assertFalse(assignment.key.startswith(STOPS_A_START), assignment.key)
            self.assertNotEqual(assignment.key, "ExecCondition")
        self.assertIn(window.last("Service", "RemainAfterExit"), (None, "no"))
        timeout = window.last("Service", "TimeoutStartSec")
        self.assertRegex(timeout or "", r"^\d+min$", "the window is bounded")
        self.assertLessEqual(int(timeout[:-3]), 30)
        # No second schedule: the daily backup pulls it in, and enabling the server enables it.
        self.assertIsNone(subject.units.get(WINDOW.replace(".service", ".timer")))
        self.assertEqual(window.values("Install", "WantedBy"), [BACKUP])
        self.assertEqual(server.values("Install", "Also"), [WINDOW])
        # No new privilege: the service user, no capability, no privileged command prefix.
        for key, want in HARDENING.items():
            self.assertEqual(window.last("Service", key), want, f"{WINDOW} {key}")
        self.assertEqual(window.last("Service", "CapabilityBoundingSet"), "")
        self.assertEqual(window.values("Service", "AmbientCapabilities"), [])
        for key in ("ExecStart", "ExecStartPre", "ExecStartPost", "ExecStopPost"):
            for value in window.values("Service", key):
                self.assertFalse(value.startswith(("+", "!", "-")), value)
        self.assertEqual(
            [
                p.name
                for p in DEPLOY.rglob("*")
                if p.suffix in (".rules", ".pkla") or "sudoers" in p.name or "polkit" in p.name
            ],
            [],
            "no grant file ships",
        )
        # It reads the server's store and writes beside the backups.
        self.assertEqual(
            window.values("Service", "StateDirectory"), ["deck-streak deck-streak-sync-server"]
        )
        self.assertEqual(window.last("Service", "MemoryHigh"), budget[WINDOW]["memory_high"])
        self.assertEqual(window.last("Service", "MemoryMax"), budget[WINDOW]["memory_max"])

    def test_a_planted_copy_failure_fails_the_unit_and_publishes_nothing(self):
        backup = load_backup()
        window = load_subject(REPO).units[WINDOW]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = make_sync_base(root)
            snapshots = root / "state" / "sync-snapshots"
            (base / "staging" / "media.db").write_bytes(b"not a database" * 300)
            self.assertEqual(self.window(backup, base, snapshots), 1)
            self.assertEqual(names_in(snapshots, "gen-"), [], "nothing is published")
            # A non-zero exit fails the unit, and its failure starts the server again.
            self.assertIn(SYNC_SERVER, window.values("Unit", "OnFailure"))
            # The same store, sound, is copied whole, and its success starts the server again.
            make_sync_base(root / "sound")
            self.assertEqual(self.window(backup, root / "sound" / "sync-base", snapshots), 0)
            self.assertEqual(names_in(snapshots, "gen-"), ["gen-20300101T030000Z.tar"])
            self.assertEqual(window.values("Unit", "OnSuccess"), [SYNC_SERVER])

    def test_the_copy_reads_only_a_stopped_generation(self):
        backup = load_backup()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = make_sync_base(root)
            snapshots = root / "sync-snapshots"
            for held in SNAPSHOT_FILES:
                # A running server holds the file in SQLite's exclusive locking mode.
                holder = sqlite3.connect(base / "owner" / held)
                holder.execute("PRAGMA locking_mode=EXCLUSIVE")
                holder.execute("BEGIN IMMEDIATE")
                holder.execute("COMMIT")
                started = time.monotonic()
                self.assertEqual(self.window(backup, base, snapshots), 1, held)
                self.assertLess(time.monotonic() - started, 5, "a holder is refused at once")
                self.assertEqual(names_in(snapshots, "gen-"), [], held)
                holder.close()
            # Stopped, the pair is copied from the one generation the store holds.
            self.assertEqual(self.window(backup, base, snapshots), 0)
            with tarfile.open(snapshots / "gen-20300101T030000Z.tar") as archive:
                names = sorted(archive.getnames())
                extracted = root / "out"
                archive.extractall(extracted, filter="data")
            self.assertEqual(
                names,
                sorted(
                    f"{u}/{n}"
                    for u in ("owner", "staging")
                    for n in (*SNAPSHOT_FILES, "media/a.jpg")
                ),
            )
            for user in ("owner", "staging"):
                self.assertEqual(integrity(extracted / user / "collection.anki2"), [("ok",)])
                copy = sqlite3.connect(extracted / user / "collection.anki2")
                self.assertEqual(copy.execute("SELECT count(*) FROM cards").fetchone(), (3,))
                copy.close()

    def test_the_daily_run_checks_archives_copies_offsite_and_keeps_three(self):
        backup = load_backup()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base = make_sync_base(root)
            snapshots = root / "sync-snapshots"
            environ, log = self.offsite(root)
            # No generation, no snapshot directory: the server is not installed, and the daily copy
            # of the database runs alone.
            self.assertEqual(self.daily(backup, root, snapshots, self.STAMPS[0], {}), 0)
            self.assertFalse(snapshots.exists())
            for stamp in self.STAMPS:
                self.assertEqual(self.window(backup, base, snapshots, stamp), 0)
                self.assertEqual(self.daily(backup, root, snapshots, stamp, environ), 0)
            tars = names_in(snapshots, "sync-")
            self.assertEqual(
                tars,
                sorted(
                    f"sync-2030010{d}T030000Z.{k}" for d in (2, 3, 4) for k in ("sha256", "tar")
                ),
            )
            self.assertEqual(names_in(snapshots, "gen-"), [], "every generation was archived")
            calls = [json.loads(line) for line in log.read_text(encoding="utf-8").splitlines()]
            self.assertEqual(len(calls), 4)
            newest = snapshots / "sync-20300104T030000Z"
            self.assertEqual(
                calls[-1],
                [
                    "--no-clobber",
                    f"{newest}.tar",
                    f"{newest}.sha256",
                    environ["DECKSTREAK_SNAPSHOT_BUCKET"],
                ],
            )
            manifest = {}
            for line in Path(f"{newest}.sha256").read_text(encoding="utf-8").splitlines():
                hexdigest, name = line.split("  ", 1)
                manifest[name] = hexdigest
            with tarfile.open(f"{newest}.tar") as archive:
                for member in archive.getmembers():
                    data = archive.extractfile(member).read()
                    self.assertEqual(manifest.pop(member.name), hashlib.sha256(data).hexdigest())
            self.assertEqual(manifest, {})
            # A generation with no bucket configured is kept for the next run, and the run fails.
            self.assertEqual(self.window(backup, base, snapshots, "2030-01-05T03:00:00Z"), 0)
            self.assertEqual(self.daily(backup, root, snapshots, "2030-01-05T03:00:00Z", {}), 1)
            self.assertEqual(names_in(snapshots, "gen-"), ["gen-20300105T030000Z.tar"])
            # A generation whose collection fails its check is not archived, and the run fails.
            broken = root / "broken"
            make_sync_base(broken)
            (broken / "sync-base" / "owner" / "collection.anki2").write_bytes(b"x" * 4096)
            with tarfile.open(snapshots / "gen-20300106T030000Z.tar", "w") as archive:
                for path in sorted((broken / "sync-base").rglob("*")):
                    if path.is_file():
                        archive.add(path, arcname=str(path.relative_to(broken / "sync-base")))
            self.assertEqual(
                self.daily(backup, root, snapshots, "2030-01-06T03:00:00Z", environ), 1
            )
            self.assertNotIn("sync-20300106T030000Z.tar", names_in(snapshots, "sync-"))
            self.assertEqual(len(log.read_text(encoding="utf-8").splitlines()), 4)

    def drill(self, root, snapshots):
        """The weekly drill over a sound replica and daily copy, with the snapshot directory given."""
        state = root / "drill-state"
        state.mkdir(exist_ok=True)
        live = state / "deck_streak.db"
        if not live.exists():
            make_database(live)
            (state / "backups").mkdir()
            make_database(state / "backups" / "deck_streak-20300101T030000Z.db")
            make_database(root / "replica-seed.db")
        stub = root / "litestream"
        stub.write_text(
            "#!/usr/bin/env python3\nimport shutil, sys\nargs = sys.argv[1:]\n"
            f"shutil.copyfile({str(root / 'replica-seed.db')!r}, args[args.index('-o') + 1])\n",
            encoding="utf-8",
        )
        stub.chmod(0o755)
        config = root / "litestream.yml"
        config.write_text("dbs: []\n", encoding="utf-8")
        scratch = root / "tmp"
        scratch.mkdir(exist_ok=True)
        return subprocess.run(
            [
                "bash",
                str(DRILL_SCRIPT),
                "--litestream",
                str(stub),
                "--config",
                str(config),
                "--database",
                str(live),
                "--backups",
                str(state / "backups"),
                "--snapshots",
                str(snapshots),
            ],
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
            env=dict(os.environ, TMPDIR=str(scratch), PYTHONDONTWRITEBYTECODE="1"),
        )

    def test_the_drill_restores_the_newest_snapshot_and_opens_its_collections(self):
        backup = load_backup()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            snapshots = root / "sync-snapshots"
            # No snapshot directory: the server is not installed, and the drill is the old one.
            done = self.drill(root, snapshots)
            self.assertEqual(done.returncode, 0, done.stderr)
            # A snapshot directory with no archive in it fails the drill.
            snapshots.mkdir()
            done = self.drill(root, snapshots)
            self.assertEqual(done.returncode, 1, done.stderr)
            self.assertIn("no snapshot archive", done.stderr)
            base = make_sync_base(root)
            environ, _ = self.offsite(root)
            self.assertEqual(self.window(backup, base, snapshots), 0)
            self.assertEqual(self.daily(backup, root, snapshots, self.STAMPS[0], environ), 0)
            done = self.drill(root, snapshots)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertIn("owner: 3 card(s)", done.stdout)
            self.assertIn("staging: 3 card(s)", done.stdout)
            self.assertEqual(sorted(p.name for p in (root / "tmp").iterdir()), [])
            # A changed byte in the archive fails its digest.
            archive = snapshots / "sync-20300101T030000Z.tar"
            raw = bytearray(archive.read_bytes())
            at = raw.index(b"image bytes of owner")
            raw[at] ^= 0x01
            archive.write_bytes(bytes(raw))
            done = self.drill(root, snapshots)
            self.assertEqual(done.returncode, 1, done.stderr)
            self.assertIn("owner/media/a.jpg", done.stderr)


if __name__ == "__main__":
    unittest.main()

"""SPEC-027's registrations: the cron-fire ledger, the catch-up decision, the predecessor's schedule,
its tick and skew arithmetic, and the scheduler's constants.

`cron_ledger` opens the predecessor's store (`database.py:GamifyStore`) on a synthetic temporary
database and runs each case's steps in order: a claim (`GamifyStore.claim_cron_fire`), a release
(`GamifyStore.release_cron_fire`), or a record through the pipeline's passthrough
(`pipeline_layers/ops.py:OpsLayer.record_cron_fire`), which suppresses a `missed` on positive
evidence before the store's own `record_cron_fire` writes. The passthrough runs on a stand-in
holding only the store. The store's clock (`database.py:_now_iso`) reads each step's instant
instead of the system time. The golden holds each step's result and the row after it, and whether
the predecessor counts the job as firing at most once a day (`ops.py:ONCE_DAILY_CRON_JOBS`), which
decides the suppression.

`catchup` runs `scheduler.py:run_startup_catchup` over the predecessor's own scheduler, built and
never started, holding one allowlisted cron job whose function is the predecessor's stamping wrapper
(`scheduler.py:_stamped`) around a stub body, and a stub pipeline that answers the claim with the
case's value and records every claim, record, release and alert. The body records its invocation,
moves the notifier's marker by the case's counts, and returns the case's delivery or raises. The
process's zone is UTC for the call, so the trigger, the local midnight and the naive instant agree
whatever machine draws the cases.

`predecessor_schedule` builds `scheduler.py:create_scheduler` with the default settings
(`config.py:Settings.from_env` over an empty environment) and a stub pipeline, and lists each job's
id and the trigger fields that are not their defaults.

The adapters compute nothing: every result, row, event and field is what the predecessor's code
produced. A fire day is written as its epoch day number and an instant as epoch milliseconds.
"""

import asyncio
import datetime as dt
import functools
import os
import tempfile
import time
import types
from pathlib import Path
from unittest import mock

EPOCH = dt.datetime(1970, 1, 1, tzinfo=dt.timezone.utc)
NAIVE_EPOCH = dt.datetime(1970, 1, 1)
EPOCH_DATE = dt.date(1970, 1, 1)
MINUTE_MS = 60_000
HOUR_MS = 3_600_000
DAY_MS = 86_400_000

#: A job the predecessor fires at most once a day, and one it fires many times a day; neither has a
#: delivery record, so the passthrough's evidence is the ledger's own row (its first tier).
ONCE_DAILY_JOBS = ("db_maintenance", "weekly_report")
REPEATING_JOBS = ("liveness_watch", "sync")
OUTCOMES = ("ok", "error", "catchup", "missed")


def instant(ms):
    """The aware instant `ms` milliseconds after the epoch."""
    return EPOCH + dt.timedelta(milliseconds=ms)


def iso_day(day):
    """The ISO date of epoch day `day`, as the predecessor keys its ledger."""
    return (EPOCH_DATE + dt.timedelta(days=day)).isoformat()


def ledger_row(row):
    """A `cron_fires` row with its day as a date and its instants as aware datetimes, which the
    generator writes as numbers."""
    if row is None:
        return None
    row = dict(row)
    last_fire_at = row["last_fire_at"]
    return {
        "first_seen_at": dt.datetime.fromisoformat(row["first_seen_at"]),
        "updated_at": dt.datetime.fromisoformat(row["updated_at"]),
        "last_fire_at": None if last_fire_at is None else dt.datetime.fromisoformat(last_fire_at),
        "ok_count": row["ok_count"],
        "error_count": row["error_count"],
        "catchup_count": row["catchup_count"],
        "missed_count": row["missed_count"],
        "last_outcome": row["last_outcome"],
    }


def with_temporary_store(store_class, predecessor, *, job_id, fire_day, steps):
    """Run the case's steps on the predecessor's store over a fresh temporary database."""
    database = predecessor("database")
    ops = predecessor("pipeline_layers.ops.OpsLayer")
    once_daily = job_id in predecessor("pipeline_layers.ops.ONCE_DAILY_CRON_JOBS")
    day = iso_day(fire_day)

    async def run():
        with tempfile.TemporaryDirectory() as scratch:
            store = store_class(Path(scratch) / "ledger.db")
            await store.connect()
            passthrough = types.SimpleNamespace(_store=store)
            passthrough._cron_fire_already_delivered = functools.partial(
                ops._cron_fire_already_delivered, passthrough
            )
            recorded = []
            try:
                for step in steps:
                    now = instant(step["at_ms"]).isoformat()
                    with mock.patch.object(database, "_now_iso", lambda now=now: now):
                        if step["op"] == "claim":
                            result = await store.claim_cron_fire(job_id, day)
                        elif step["op"] == "release":
                            result = await store.release_cron_fire(job_id, day)
                        else:
                            result = await ops.record_cron_fire(
                                passthrough, job_id, day, step["outcome"]
                            )
                    row = await store._fetchone(
                        "SELECT * FROM cron_fires WHERE job_id = ? AND fire_day = ?",
                        (job_id, day),
                    )
                    recorded.append({"result": result, "row": ledger_row(row)})
            finally:
                await store.close()
            return recorded

    return {"once_daily": once_daily, "steps": asyncio.run(run())}


def ledger_case(job_id, start_ms, gaps, operations):
    """A case of `operations` (`claim`, `release`, or an outcome to record) on one fire day, each
    step `gaps` milliseconds after the one before."""
    steps = []
    at = start_ms
    for operation, gap in zip(operations, gaps, strict=True):
        at += gap
        if operation in ("claim", "release"):
            steps.append({"op": operation, "at_ms": at})
        else:
            steps.append({"op": "record", "outcome": operation, "at_ms": at})
    return {"job_id": job_id, "fire_day": start_ms // DAY_MS, "steps": steps}


def ledger_cases(rng):
    """Each claim, release and record sequence a port gets wrong, then random sequences."""

    def draw(job_id, operations):
        start = rng.randrange(0, 4_000_000_000_000)
        gaps = [rng.randrange(1, HOUR_MS) for _ in operations]
        return ledger_case(job_id, start, gaps, operations)

    once, repeating = ONCE_DAILY_JOBS[0], REPEATING_JOBS[0]
    drawn = []
    for job_id in (once, repeating):
        drawn += [
            ("claim", draw(job_id, ["claim"])),
            ("claim", draw(job_id, ["claim", "claim"])),
            ("claim", draw(job_id, ["ok", "claim"])),
            ("claim", draw(job_id, ["error", "claim"])),
            ("claim", draw(job_id, ["catchup", "claim"])),
            ("claim", draw(job_id, ["missed", "claim", "claim"])),
            ("release", draw(job_id, ["claim", "ok", "release", "claim"])),
            ("release", draw(job_id, ["claim", "release", "release"])),
            ("release", draw(job_id, ["release"])),
            ("release", draw(job_id, ["ok", "ok", "error", "release"])),
            ("missed", draw(job_id, ["missed"])),
            ("missed", draw(job_id, ["missed", "missed"])),
            ("missed", draw(job_id, ["ok", "missed"])),
            ("missed", draw(job_id, ["claim", "missed", "error"])),
            ("record", draw(job_id, ["ok", "error", "catchup", "ok"])),
        ]
    for _ in range(12):
        job_id = rng.choice(ONCE_DAILY_JOBS + REPEATING_JOBS)
        operations = [
            rng.choice(("claim", "release") + OUTCOMES) for _ in range(rng.randrange(1, 7))
        ]
        drawn.append((None, draw(job_id, operations)))
    return drawn


class StubPipeline:
    """Answers the claim with the case's value, reports the notifier's marker, and records every
    call the catch-up makes, with its fire day as a date."""

    def __init__(self, claim, events):
        self.claim = claim
        self.events = events
        self.attempts = 0
        self.deliveries = 0

    async def claim_cron_fire(self, job_id, fire_day):
        self.events.append(["claim", job_id, dt.date.fromisoformat(fire_day)])
        return self.claim

    async def record_cron_fire(self, job_id, fire_day, outcome):
        self.events.append(["record", job_id, dt.date.fromisoformat(fire_day), outcome])

    async def release_cron_fire(self, job_id, fire_day):
        self.events.append(["release", job_id, dt.date.fromisoformat(fire_day)])

    def cron_delivery_marker(self):
        return (self.attempts, self.deliveries)

    async def alert_infra(self, _text):
        self.events.append(["alert"])


def in_utc(call):
    """Run `call` with the process's zone set to UTC, then restore the zone."""
    try:
        with mock.patch.dict(os.environ, {"TZ": "UTC"}):
            time.tzset()
            return call()
    finally:
        time.tzset()


def with_stub_scheduler(
    run_startup_catchup,
    predecessor,
    *,
    job_id,
    hour,
    minute,
    now_ms,
    claim,
    delivered,
    raises,
    attempts,
    deliveries,
):
    """Run the boot-time catch-up once over one allowlisted job and a recording stub pipeline."""
    scheduler_module = predecessor("scheduler")
    events = []
    pipeline = StubPipeline(claim, events)

    async def body():
        events.append(["invoke", job_id])
        pipeline.attempts += attempts
        pipeline.deliveries += deliveries
        if raises:
            raise RuntimeError("a synthetic failure")
        return delivered

    def run():
        # The job is registered as create_scheduler registers one: a cron job whose function is
        # the stamping wrapper, on a scheduler that is built and never started.
        scheduler = scheduler_module.AsyncIOScheduler()
        scheduler.add_job(
            scheduler_module._stamped(job_id, body, pipeline),
            "cron",
            hour=hour,
            minute=minute,
            timezone=dt.timezone.utc,
            id=job_id,
        )
        now = NAIVE_EPOCH + dt.timedelta(milliseconds=now_ms)
        return asyncio.run(run_startup_catchup(scheduler, pipeline, after=None, now=now))

    return {"ran": in_utc(run), "events": events}


def catchup_case(job_id, hour, minute, now_ms, **behaviour):
    """One job at `hour`:`minute` each day, the catch-up running at `now_ms` (a local instant),
    and how the job behaves when invoked."""
    case = {
        "job_id": job_id,
        "hour": hour,
        "minute": minute,
        "now_ms": now_ms,
        "claim": True,
        "delivered": True,
        "raises": False,
        "attempts": 0,
        "deliveries": 0,
    }
    case.update(behaviour)
    return case


def catchup_cases(rng):
    """Every branch of the decision and its boundaries, then random fires and lateness."""
    allowlist = ("daily_digest", "last_chance", "morning_brief", "streak_risk")

    def fire(hour, minute):
        return rng.randrange(0, 40_000) * DAY_MS + hour * HOUR_MS + minute * MINUTE_MS

    def case(hour, minute, late_ms, **behaviour):
        return catchup_case(
            rng.choice(allowlist), hour, minute, fire(hour, minute) + late_ms, **behaviour
        )

    limit = 360 * MINUTE_MS
    drawn = [
        ("on_time", case(4, 7, 0)),
        ("on_time", case(9, 5, 30_000)),
        ("five_hours", case(4, 7, 5 * HOUR_MS)),
        ("seven_hours", case(4, 7, 7 * HOUR_MS)),
        ("boundary", case(9, 5, limit)),
        ("boundary", case(9, 5, limit + 1)),
        ("boundary", case(9, 5, limit - 1)),
        ("before_fire", case(9, 5, -MINUTE_MS)),
        ("before_fire", case(23, 59, -HOUR_MS)),
        ("after_midnight", case(22, 0, 3 * HOUR_MS)),
        ("after_midnight", case(23, 30, HOUR_MS)),
        ("midnight", case(0, 0, 0)),
        ("midnight", case(0, 0, 5 * HOUR_MS + 59 * MINUTE_MS)),
        ("claim_refused", case(8, 0, HOUR_MS, claim=False)),
        ("claim_refused", case(4, 7, 8 * HOUR_MS, claim=False)),
        ("delivered", case(20, 0, 2 * HOUR_MS, attempts=1, deliveries=1)),
        ("released", case(20, 0, HOUR_MS, delivered=False, attempts=1, deliveries=0)),
        ("released", case(9, 5, 0, delivered=False, attempts=3, deliveries=0)),
        ("latched", case(22, 0, 30 * MINUTE_MS, delivered=False, attempts=0, deliveries=0)),
        ("latched", case(8, 0, MINUTE_MS, delivered=False, attempts=1, deliveries=1)),
        ("raised", case(9, 5, HOUR_MS, raises=True)),
        ("raised", case(8, 0, 0, raises=True, attempts=1, deliveries=0)),
    ]
    for _ in range(12):
        hour, minute = rng.randrange(0, 24), rng.randrange(0, 60)
        attempts = rng.randrange(0, 3)
        drawn.append(
            (
                None,
                case(
                    hour,
                    minute,
                    rng.randrange(-2 * HOUR_MS, 9 * HOUR_MS),
                    claim=rng.random() < 0.8,
                    delivered=rng.random() < 0.5,
                    raises=rng.random() < 0.15,
                    attempts=attempts,
                    deliveries=rng.randrange(0, attempts + 1),
                ),
            )
        )
    return drawn


def with_default_settings(create_scheduler, predecessor):
    """Build the predecessor's scheduler with its default settings and list every job's trigger."""
    settings_class = predecessor("config.Settings")

    async def noop():
        return None

    pipeline = types.SimpleNamespace(run_maintenance=noop, run_liveness_watch=noop)

    def build():
        with mock.patch.dict(os.environ, {"TZ": "UTC"}, clear=True):
            time.tzset()
            settings = settings_class.from_env()
            scheduler = create_scheduler(pipeline, settings)
            jobs = [
                {
                    "id": job.id,
                    "trigger": type(job.trigger).__name__,
                    "fields": {
                        field.name: str(field)
                        for field in job.trigger.fields
                        if not field.is_default
                    },
                }
                for job in scheduler.get_jobs()
            ]
            return {"sync_interval_min": settings.sync_interval_min, "jobs": jobs}

    try:
        return build()
    finally:
        time.tzset()


def tick_cases(rng):
    """Every interval that divides the hour, intervals that do not, offsets past the interval
    and below zero, then random pairs."""
    drawn = [
        ("alignable", {"interval_min": interval, "offset_min": 2})
        for interval in (5, 6, 10, 12, 15, 20, 30, 60)
    ]
    drawn += [
        ("unalignable", {"interval_min": interval, "offset_min": 2})
        for interval in (-15, 0, 1, 4, 7, 45, 61)
    ]
    drawn += [("negative", {"interval_min": 15, "offset_min": offset}) for offset in (-1, -7, -60)]
    drawn += [("offset", {"interval_min": 15, "offset_min": offset}) for offset in (7, 15, 61)]
    for _ in range(12):
        drawn.append(
            (
                None,
                {"interval_min": rng.randrange(1, 70), "offset_min": rng.randrange(-120, 121)},
            )
        )
    return drawn


def signed_skew_cases(rng):
    """The half-day boundary each way, the midnight straddle, operands off the circle, then
    random minutes of the day."""
    drawn = [
        ("wrap", {"a_min": a, "b_min": b})
        for a, b in ((0, 0), (720, 0), (0, 720), (719, 0), (721, 0), (1439, 0), (0, 1439))
    ]
    drawn += [
        ("negative", {"a_min": a, "b_min": b})
        for a, b in ((-1, 0), (0, -1441), (2000, -30), (-720, 720))
    ]
    for _ in range(16):
        drawn.append((None, {"a_min": rng.randrange(0, 1440), "b_min": rng.randrange(0, 1440)}))
    return drawn


def rollover_skew_case(cron_hour, scheduler_offset, rollover_hour, collection_offset):
    return {
        "cron_hour": cron_hour,
        "scheduler_utc_offset_min": scheduler_offset,
        "collection_rollover_hour": rollover_hour,
        "collection_utc_offset_min": collection_offset,
    }


def rollover_skew_cases(rng):
    """Offsets that agree and differ, at the ends of the range and across midnight, then random
    hours and offsets."""
    drawn = [
        ("offset", rollover_skew_case(4, scheduler, 4, collection))
        for scheduler, collection in (
            (0, 0),
            (0, 60),
            (60, 0),
            (-720, 840),
            (840, -720),
            (330, 0),
            (-300, -240),
        )
    ]
    drawn += [
        ("wrap", rollover_skew_case(cron, 0, rollover, 0))
        for cron, rollover in ((0, 23), (23, 0), (12, 0), (0, 12))
    ]
    offsets = range(-720, 841, 15)
    for _ in range(16):
        drawn.append(
            (
                None,
                rollover_skew_case(
                    rng.randrange(0, 24),
                    rng.choice(offsets),
                    rng.randrange(0, 24),
                    rng.choice(offsets),
                ),
            )
        )
    return drawn


def schedule_cases(_rng):
    """The default settings: one case, drawn from nothing."""
    return [("defaults", {})]


FUNCTIONS = {
    "cron_ledger": {
        "kind": "adapter",
        "function": "database.GamifyStore",
        "adapter": with_temporary_store,
        "note": (
            "Opens the store on a fresh temporary database and runs each step in order: a claim, "
            "a release, or a record through the pipeline's passthrough on a stand-in holding only "
            "the store; the store's clock reads each step's instant, and each step's result and "
            "the row after it are returned, with whether the predecessor counts the job as firing "
            "at most once a day."
        ),
        "cases": ledger_cases,
    },
    "catchup": {
        "kind": "adapter",
        "function": "scheduler.run_startup_catchup",
        "adapter": with_stub_scheduler,
        "note": (
            "Runs the catch-up once over the predecessor's scheduler, built and never started, "
            "holding one allowlisted cron job at the case's hour and minute whose function is the "
            "stamping wrapper around a stub body, and a stub pipeline that answers the claim with "
            "the case's value and records every claim, record, release and alert; the body "
            "records its invocation, moves the notifier's marker by the case's counts and returns "
            "the case's delivery or raises; the process's zone is UTC for the call."
        ),
        "cases": catchup_cases,
    },
    "predecessor_schedule": {
        "kind": "adapter",
        "function": "scheduler.create_scheduler",
        "adapter": with_default_settings,
        "note": (
            "Builds the scheduler with the settings an empty environment resolves to and a stub "
            "pipeline, in the UTC zone, and lists each job's id, its trigger's type and the "
            "trigger fields that are not their defaults."
        ),
        "cases": schedule_cases,
    },
    "tick_minutes": {
        "kind": "function",
        "function": "timebase.tick_minutes",
        "cases": tick_cases,
    },
    "signed_skew": {
        "kind": "function",
        "function": "timebase.signed_skew_minutes",
        "cases": signed_skew_cases,
    },
    "rollover_skew": {
        "kind": "function",
        "function": "timebase.rollover_skew_minutes",
        "cases": rollover_skew_cases,
    },
    "scheduler.constants": {
        "kind": "constants",
        "names": [
            "scheduler.CATCHUP_MAX_LATE_MIN",
            "database.CRON_FIRES_RETENTION_DAYS",
            "database.CRON_OUTCOMES",
            "constants.SYNC_TICK_OFFSET_MIN",
            "constants.TICK_ALIGN_MIN_INTERVAL_MIN",
            "constants.LIVENESS_BOOT_GRACE_SECS",
            "constants.ROLLOVER_DRIFT_TOLERANCE_MIN",
        ],
    },
}

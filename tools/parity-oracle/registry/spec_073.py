"""SPEC-073's registrations: the badge catalog, the study badge conditions, the context the
pipeline builds for them, the hour-window count, the thresholds, personal-record detection, the
records window and its chase line, the next milestone, and the ladders' constants.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_the_catalog` reads `constants.BADGES` and returns each badge's key, name, emoji,
  description and tier, in catalog order.
* `with_the_inputs` builds a `BadgeContext` from the case's flat inputs and returns what
  `gamification/badges.py:_conditions` answers for it.
* `with_a_stub_pipeline` drives `GamifyPipeline._evaluate_and_award` on a stand-in pipeline over a
  stub store holding the case's rollups, with `badges.evaluate` wrapped to record the context it
  receives; it returns that context's fields and the keys awarded.
* `with_the_window` drives `analytics.count_reviews_in_local_hours` on synthetic reviews and a
  collection configuration built from the case's offset and rollover.
* `with_the_rollups` drives `gamification/rewards.py:detect_records` and returns each record as
  its kind, value and label.
* `with_a_stub_store` drives `GamifyPipeline._compute_and_store_coaching` on a stand-in pipeline
  over a stub store that records the rollup limit it is asked for and holds the case's stored
  records, with `coaching.compute_all` patched to return the case's new records; it returns the
  limit, the rows written, the celebration keys with each day as a day token, and whether the
  seed ran.
* `with_a_board` drives `bot.py:CommandBot._render_records` on a board whose days are day tokens,
  and returns the record to chase and its gap, read from the rendered text, or none.
* `with_the_totals` drives `gamification/rewards.py:next_milestone` and returns the milestone.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic, every day an epoch day number.
"""

import asyncio
import datetime as dt
import re
import types
from unittest import mock

#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: A study day number near the present.
PRESENT_DAY = 20_000
#: The days of the 30-day window an Iron Will check reads.
IRON_WILL_DAYS = 30


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def at_ms(day, hour, minute, offset):
    """The millisecond instant whose local clock, at `offset` minutes, reads `hour:minute` of
    calendar day `day`."""
    return ((day * 86_400) + hour * 3_600 + minute * 60 - offset * 60) * 1_000


# --- the catalog ----------------------------------------------------------------------------------


def catalog_cases(rng):
    return [("catalog", {})]


def with_the_catalog(catalog, predecessor):
    return [[b.key, b.name, b.emoji, b.description, b.tier] for b in catalog.values()]


# --- the study conditions -------------------------------------------------------------------------

#: A context in which no study condition holds.
QUIET = {
    "lifetime": 0,
    "streak_current": 0,
    "comeback_armed": False,
    "day_reviews": 0,
    "day_decks": 0,
    "day_avg_seconds": 0.0,
    "backlog": 5,
    "due_today": 5,
    "mature_count": 0,
    "leech_active": 1,
    "week_retention": 0.0,
    "week_reviews": 0,
    "week_decks": 0,
    "mature30_retention": 0.0,
    "mature30_answered": 0,
    "cleared_backlog": 0,
    "night_owl": 0,
    "early_bird": 0,
    "week_scores": [],
    "score_total": 0,
    "iron_will_ok": False,
}

#: Each condition's inputs with the values below, at and above its threshold.
SWEEPS = (
    ("lifetime", (0, 1, 2)),
    ("lifetime", (999, 1_000, 1_001)),
    ("lifetime", (9_999, 10_000, 10_001)),
    ("streak_current", (6, 7, 8)),
    ("streak_current", (29, 30, 31)),
    ("streak_current", (99, 100, 101)),
    ("streak_current", (364, 365, 366)),
    ("day_reviews", (99, 100, 101)),
    ("day_decks", (2, 3, 4)),
    ("mature_count", (99, 100, 101)),
    ("mature_count", (999, 1_000, 1_001)),
    ("week_decks", (4, 5, 6)),
    ("mature30_answered", (49, 50, 51)),
    ("cleared_backlog", (199, 200, 201)),
    ("night_owl", (49, 50, 51)),
    ("early_bird", (49, 50, 51)),
    ("score_total", (99, 100, 101)),
    ("day_reviews", (99, 100, 101)),
)


def conditions_cases(rng):
    found = []
    for name, values in SWEEPS:
        for edge, value in zip(("below", "at", "above"), values):
            base = dict(QUIET, lifetime=2_000)
            found.append((edge, dict(base, **{name: value})))
    for edge, retention in (("below", 89.9), ("at", 90.0), ("above", 90.1)):
        found.append((edge, dict(QUIET, week_retention=retention, week_reviews=50)))
    for edge, reviews in (("below", 49), ("at", 50), ("above", 51)):
        found.append((edge, dict(QUIET, week_retention=95.0, week_reviews=reviews)))
    for edge, retention in (("below", 94.9), ("at", 95.0), ("above", 95.1)):
        found.append((edge, dict(QUIET, mature30_retention=retention, mature30_answered=50)))
    for edge, answered in (("below", 49), ("at", 50), ("above", 51)):
        found.append((edge, dict(QUIET, mature30_retention=99.0, mature30_answered=answered)))
    for edge, seconds in (("below", 5.9), ("at", 6.0), ("above", 6.1), ("zero", 0.0)):
        found.append((edge, dict(QUIET, day_reviews=100, day_avg_seconds=seconds)))
    for edge, reviews in (("below", 99), ("at", 100)):
        found.append((edge, dict(QUIET, day_reviews=reviews, day_avg_seconds=5.0)))
    for edge, lifetime in (("below", 499), ("at", 500), ("above", 501)):
        found.append((edge, dict(QUIET, lifetime=lifetime, leech_active=0)))
    found.append(("leech", dict(QUIET, lifetime=500, leech_active=1)))
    for edge, lifetime in (("below", 0), ("at", 1)):
        found.append((edge, dict(QUIET, lifetime=lifetime, backlog=0, due_today=0)))
    found.append(("due", dict(QUIET, lifetime=1, backlog=0, due_today=1)))
    found.append(("backlog", dict(QUIET, lifetime=1, backlog=1, due_today=0)))
    for edge, score in (("below", 74), ("at", 75)):
        found.append((edge, dict(QUIET, week_scores=[80, 90, 80, 90, 80, 90, score])))
    found.append(("short", dict(QUIET, week_scores=[90, 90, 90, 90, 90, 90])))
    found.append(("long", dict(QUIET, week_scores=[10, 10, 80, 80, 80, 80, 80, 80, 80])))
    found.append(("armed", dict(QUIET, comeback_armed=True)))
    found.append(("iron", dict(QUIET, iron_will_ok=True)))
    return found


def build_context(types_module, badges_module, case):
    day = as_date(PRESENT_DAY)
    metrics = types_module.DailyMetrics(
        day=day,
        reviews=case["day_reviews"],
        learn_count=0,
        review_count=case["day_reviews"],
        relearn_count=0,
        filtered_count=0,
        seconds=0.0,
        answered=0,
        passed=0,
        true_retention=0.0,
        graduations=0,
        decks_studied=case["day_decks"],
        avg_answer_seconds=case["day_avg_seconds"],
    )
    snapshot = types_module.CardStateSnapshot(
        total_cards=0,
        mature_count=case["mature_count"],
        young_count=0,
        learning_count=0,
        suspended_count=0,
        leech_active=case["leech_active"],
        backlog=case["backlog"],
        due_today=case["due_today"],
    )
    streak = types_module.StreakState(
        current=case["streak_current"],
        longest=case["streak_current"],
        freezes=0,
        last_study_day=day,
        comeback_armed=case["comeback_armed"],
    )
    return badges_module.BadgeContext(
        lifetime_reviews=case["lifetime"],
        today=metrics,
        snapshot=snapshot,
        streak=streak,
        score_total=case["score_total"],
        week_scores=tuple(case["week_scores"]),
        week_reviews=case["week_reviews"],
        week_decks=case["week_decks"],
        week_retention=case["week_retention"],
        mature30_retention=case["mature30_retention"],
        mature30_answered=case["mature30_answered"],
        night_owl_reviews=case["night_owl"],
        early_bird_reviews=case["early_bird"],
        cleared_backlog_today=case["cleared_backlog"],
        iron_will_ok=case["iron_will_ok"],
    )


def with_the_inputs(conditions, predecessor, **case):
    ctx = build_context(predecessor("types"), predecessor("gamification.badges"), case)
    return conditions(ctx)


# --- the context the pipeline builds --------------------------------------------------------------


class StubBadgeStore:
    """Holds the case's rollups, most recent first, and awards every badge it is offered."""

    def __init__(self, rollups, today_score):
        self.rollups = rollups
        self.today_score = today_score

    async def get_recent_rollups(self, limit):
        return self.rollups[:limit]

    async def get_daily_rollup(self, day):
        return {"score": self.today_score}

    async def award_badge(self, badge, day):
        return True


def reviews_of(
    rng, days, per_day, offset, hours, decks, eases=(1, 3, 3, 3, 4), last_ivls=(0, 5, 30)
):
    """One day's worth of synthetic reviews for each day: `per_day` reviews on card ids drawn
    from the case's decks, at hours drawn from `hours`."""
    rows = []
    for day in days:
        for _ in range(per_day):
            cid = 1_000 + rng.randrange(decks) * 100 + rng.randrange(6)
            hour = hours[rng.randrange(len(hours))]
            ease = eases[rng.randrange(len(eases))]
            last_ivl = last_ivls[rng.randrange(len(last_ivls))]
            rows.append(
                [
                    at_ms(day, hour, rng.randrange(60), offset),
                    cid,
                    ease,
                    5,
                    last_ivl,
                    3_000 + 1_000 * rng.randrange(7),
                    1,
                ]
            )
    return rows


def pipeline_cases(rng):
    found = []
    today = PRESENT_DAY
    quiet_snapshot = {
        "mature_count": 0,
        "leech_active": 0,
        "backlog": 5,
        "due_today": 3,
    }
    clear_snapshot = {
        "mature_count": 120,
        "leech_active": 0,
        "backlog": 0,
        "due_today": 0,
    }
    for label, offset, rollover, days, per_day, hours, decks, snapshot, scores in (
        ("empty", 0, 4, [], 0, [10], 1, quiet_snapshot, []),
        ("one-day", 0, 4, [today], 6, [9, 10, 11], 2, quiet_snapshot, [60]),
        (
            "week",
            0,
            4,
            [today - i for i in range(7)],
            8,
            [9, 14, 20],
            3,
            clear_snapshot,
            [90, 80, 85, 95, 70, 60, 88],
        ),
        (
            "month",
            60,
            4,
            [today - i for i in range(30)],
            5,
            [9, 14, 20],
            6,
            clear_snapshot,
            [90, 80, 85, 95, 79, 76, 88],
        ),
        (
            "gaps",
            -300,
            3,
            [today, today - 2, today - 5, today - 9],
            7,
            [1, 2, 6, 9],
            4,
            clear_snapshot,
            [75, 75, 75],
        ),
        ("night", 0, 4, [today], 60, [0, 1, 2, 3], 1, quiet_snapshot, [40]),
        ("early", 0, 4, [today], 60, [4, 5, 6], 1, quiet_snapshot, [40]),
        (
            "late-rollover",
            120,
            6,
            [today, today - 1],
            30,
            [0, 5, 6, 7, 8],
            5,
            clear_snapshot,
            [50, 99],
        ),
        ("edge-hours", 0, 4, [today, today - 1], 12, [3, 4, 6, 7], 2, clear_snapshot, [100]),
    ):
        reviews = reviews_of(rng, days, per_day, offset, hours, decks)
        found.append(
            (
                label,
                {
                    "reviews": reviews,
                    "rollover_hour": rollover,
                    "tz_offset_minutes": offset,
                    "today": today,
                    "snapshot": snapshot,
                    "streak_current": len(days),
                    "comeback_armed": False,
                    "decks": decks,
                    "rollups": [{"score": s} for s in scores],
                    "today_score": scores[0] if scores else 0,
                    "lifetime": len(reviews),
                },
            )
        )
    return found


def with_a_stub_pipeline(evaluate_and_award, predecessor, **case):
    types_module = predecessor("types")
    analytics = predecessor("analytics")
    pipeline = predecessor("pipeline")
    cfg = types_module.CollectionConfig(
        rollover_hour=case["rollover_hour"], tz_offset_minutes=case["tz_offset_minutes"]
    )
    reviews = [
        types_module.Review(
            id_ms=row[0],
            cid=row[1],
            ease=row[2],
            ivl=row[3],
            last_ivl=row[4],
            factor=0,
            time_ms=row[5],
            rtype=row[6],
        )
        for row in case["reviews"]
    ]
    card_decks = {row[1]: (row[1] - 1_000) // 100 for row in case["reviews"]}
    today = as_date(case["today"])
    metrics = analytics.compute_daily_metrics(reviews, cfg, today, card_decks)
    snapshot = types_module.CardStateSnapshot(
        total_cards=0,
        mature_count=case["snapshot"]["mature_count"],
        young_count=0,
        learning_count=0,
        suspended_count=0,
        leech_active=case["snapshot"]["leech_active"],
        backlog=case["snapshot"]["backlog"],
        due_today=case["snapshot"]["due_today"],
    )
    streak = types_module.StreakState(
        current=case["streak_current"],
        longest=case["streak_current"],
        freezes=0,
        last_study_day=today,
        comeback_armed=case["comeback_armed"],
    )
    stand_in = types.SimpleNamespace(_store=StubBadgeStore(case["rollups"], case["today_score"]))
    seen = {}
    real = pipeline.badges.evaluate

    def record(ctx):
        seen["ctx"] = ctx
        return real(ctx)

    with mock.patch.object(pipeline.badges, "evaluate", record):
        awarded = asyncio.run(
            evaluate_and_award(
                stand_in,
                reviews=reviews,
                cfg=cfg,
                today=today,
                today_metrics=metrics,
                snapshot=snapshot,
                streak_state=streak,
                card_decks=card_decks,
                study_days_set=analytics.study_days(reviews, cfg),
                lifetime_reviews=case["lifetime"],
            )
        )
    ctx = seen["ctx"]
    return {
        "lifetime_reviews": ctx.lifetime_reviews,
        "day_reviews": ctx.today.reviews,
        "day_decks": ctx.today.decks_studied,
        "score_total": ctx.score_total,
        "week_scores": list(ctx.week_scores),
        "week_reviews": ctx.week_reviews,
        "week_decks": ctx.week_decks,
        "week_retention": ctx.week_retention,
        "mature30_retention": ctx.mature30_retention,
        "mature30_answered": ctx.mature30_answered,
        "night_owl": ctx.night_owl_reviews,
        "early_bird": ctx.early_bird_reviews,
        "cleared_backlog": ctx.cleared_backlog_today,
        "iron_will_ok": ctx.iron_will_ok,
        "awarded": [badge.key for badge in awarded],
    }


# --- the hour windows -----------------------------------------------------------------------------


def window_cases(rng):
    found = []
    for offset in (0, 60, -300, 330):
        for rollover in (0, 4, 6):
            rows = []
            for hour in range(24):
                for minute in (0, 59):
                    rows.append(
                        [at_ms(PRESENT_DAY, hour, minute, offset), 1_000 + hour, 3, 5, 5, 3_000, 1]
                    )
            # A manual entry and a skipped answer never count.
            rows.append([at_ms(PRESENT_DAY, 5, 0, offset), 1_000, 3, 0, 0, 0, 4])
            rows.append([at_ms(PRESENT_DAY, 5, 1, offset), 1_000, 0, 0, 0, 0, 1])
            # Another study day's review is never counted.
            rows.append([at_ms(PRESENT_DAY + 1, 12, 0, offset), 1_000, 3, 5, 5, 3_000, 1])
            for start, end in ((0, rollover), (rollover, 7), (0, 24), (3, 3)):
                found.append(
                    (
                        "window",
                        {
                            "reviews": rows,
                            "tz_offset_minutes": offset,
                            "rollover_hour": rollover,
                            "day": PRESENT_DAY,
                            "start_hour": start,
                            "end_hour": end,
                        },
                    )
                )
    return found


def with_the_window(
    count, predecessor, *, reviews, tz_offset_minutes, rollover_hour, day, start_hour, end_hour
):
    types_module = predecessor("types")
    cfg = types_module.CollectionConfig(
        rollover_hour=rollover_hour, tz_offset_minutes=tz_offset_minutes
    )
    built = [
        types_module.Review(
            id_ms=row[0],
            cid=row[1],
            ease=row[2],
            ivl=row[3],
            last_ivl=row[4],
            factor=0,
            time_ms=row[5],
            rtype=row[6],
        )
        for row in reviews
    ]
    return count(built, cfg, as_date(day), start_hour, end_hour)


# --- detect_records -------------------------------------------------------------------------------


def detect_cases(rng):
    found = []
    base = [{"score": 70, "reviews": 200, "seconds": 3_600.0}]
    for edge, prev in (
        ("below", {"best_score": 71, "most_reviews": 201, "most_minutes": 61}),
        ("at", {"best_score": 70, "most_reviews": 200, "most_minutes": 60}),
        ("above", {"best_score": 69, "most_reviews": 199, "most_minutes": 59}),
    ):
        found.append((edge, {"rollups": base, "prev": prev}))
    found.append(("none", {"rollups": [], "prev": {}}))
    found.append(("first", {"rollups": base, "prev": {}}))
    found.append(("zero", {"rollups": [{"score": 0, "reviews": 0, "seconds": 0.0}], "prev": {}}))
    for seconds in (59.9, 60.0, 119.9, 3_599.0):
        found.append(("minutes", {"rollups": [{"seconds": seconds}], "prev": {"most_minutes": 0}}))
    found.append(
        (
            "best-of-many",
            {
                "rollups": [
                    {"score": 50, "reviews": 900, "seconds": 600.0},
                    {"score": 99, "reviews": 100, "seconds": 7_200.0},
                    {"score": 75, "reviews": 500, "seconds": 60.0},
                ],
                "prev": {"best_score": 98},
            },
        )
    )
    found.append(("missing-keys", {"rollups": [{}], "prev": {"best_score": 0}}))
    for _ in range(6):
        rows = [
            {
                "score": rng.randrange(0, 101),
                "reviews": rng.randrange(0, 600),
                "seconds": rng.randrange(0, 9_000) + rng.randrange(0, 60) / 60.0,
            }
            for _ in range(rng.randrange(1, 8))
        ]
        prev = {
            kind: rng.randrange(0, 100)
            for kind in ("best_score", "most_reviews", "most_minutes")
            if rng.randrange(2)
        }
        found.append(("drawn", {"rollups": rows, "prev": prev}))
    return found


def with_the_rollups(detect, predecessor, *, rollups, prev):
    return [[r.kind, r.value, r.label] for r in detect(rollups, prev)]


# --- the records window ---------------------------------------------------------------------------


class StubRecordStore:
    """Holds the case's stored records and records what it is asked and told."""

    def __init__(self, stored, empty):
        self.stored = stored
        self.empty = empty
        self.limit = None
        self.written = []

    async def get_recent_rollups(self, limit):
        self.limit = limit
        return []

    async def get_month_xp(self, period):
        return 0

    async def get_coaching(self, key):
        return dict(self.stored)

    async def set_coaching(self, key, text):
        return None

    async def records_empty(self):
        return self.empty

    async def upsert_record(self, kind, value, day, previous):
        self.written.append([kind, int(value), day, int(previous)])


def window_store_cases(rng):
    found = []
    stored = {"best_score": 80, "most_reviews": 300, "most_minutes": 60}
    new = [["best_score", 85], ["most_minutes", 61]]
    found.append(("new", {"stored": stored, "empty": False, "new": new}))
    found.append(("none", {"stored": stored, "empty": False, "new": []}))
    found.append(("seed", {"stored": stored, "empty": True, "new": new}))
    found.append(("first", {"stored": {}, "empty": True, "new": new}))
    found.append(("empty-without-prev", {"stored": {}, "empty": False, "new": new}))
    found.append(
        (
            "all",
            {
                "stored": {},
                "empty": False,
                "new": [["best_score", 90], ["most_reviews", 400], ["most_minutes", 120]],
            },
        )
    )
    return found


def with_a_stub_store(coaching_step, predecessor, *, stored, empty, new):
    types_module = predecessor("types")
    pipeline = predecessor("pipeline")
    store = StubRecordStore(stored, empty)
    celebrated = []

    async def celebrate(*, event_type, event_key, text, reveal_prefix=None):
        celebrated.append([event_type, event_key])

    async def progress_objects():
        return []

    async def on_pace_run(*, exclude_day):
        return 0

    stand_in = types.SimpleNamespace(
        _store=store,
        celebrate=celebrate,
        progress_objects=progress_objects,
        _on_pace_run=on_pace_run,
    )
    day = as_date(PRESENT_DAY)
    metrics = types.SimpleNamespace(day=day, decks_studied=0)
    found = {"new_records": [{"kind": kind, "value": value} for kind, value in new]}
    with mock.patch.object(pipeline.coaching, "compute_all", lambda **_: dict(found)):
        asyncio.run(
            coaching_step(
                stand_in,
                cards=[],
                reviews=[],
                deck_names={},
                cfg=types_module.CollectionConfig(),
                card_decks={},
                lifetime_reviews=0,
                today_metrics=metrics,
                now_ms=PRESENT_DAY * 86_400_000,
                total_xp=0,
                streak_current=0,
            )
        )
    iso = day.isoformat()
    return {
        "limit": store.limit,
        "written": [[kind, value, PRESENT_DAY, prev] for kind, value, _, prev in store.written],
        "celebrated": [[event, key.replace(iso, str(PRESENT_DAY))] for event, key in celebrated],
        "seeded": bool(empty and stored),
    }


# --- the chase ------------------------------------------------------------------------------------

CHASE_LINE = re.compile(r"Chase it: (\d+) from “(.*)” today\.")


def chase_cases(rng):
    def record(label, value, today):
        return {
            "label": label,
            "value": value,
            "day": PRESENT_DAY - 3,
            "previous": value - 1,
            "today": today,
        }

    found = [
        ("empty", {"board": []}),
        (
            "none",
            {
                "board": [
                    record("Best daily score", 80, 80),
                    record("Most reviews in a day", 300, 301),
                ]
            },
        ),
        ("one", {"board": [record("Best daily score", 80, 70)]}),
        (
            "smallest",
            {
                "board": [
                    record("Best daily score", 80, 70),
                    record("Most reviews in a day", 300, 295),
                    record("Most minutes in a day", 60, 10),
                ]
            },
        ),
        (
            "tie",
            {
                "board": [
                    record("Best daily score", 80, 75),
                    record("Most reviews in a day", 300, 295),
                    record("Most minutes in a day", 60, 55),
                ]
            },
        ),
        (
            "tie-reversed",
            {
                "board": [
                    record("Most minutes in a day", 60, 55),
                    record("Best daily score", 80, 75),
                ]
            },
        ),
        ("zero-today", {"board": [record("Best daily score", 80, 0)]}),
    ]
    for _ in range(5):
        found.append(
            (
                "drawn",
                {
                    "board": [
                        record(label, rng.randrange(1, 400), rng.randrange(0, 400))
                        for label in (
                            "Best daily score",
                            "Most reviews in a day",
                            "Most minutes in a day",
                        )
                    ]
                },
            )
        )
    return found


def with_a_board(render, predecessor, *, board):
    text = render([dict(row, day=str(row["day"])) for row in board])
    found = CHASE_LINE.search(text)
    return {"chase": [found.group(2), int(found.group(1))] if found else None}


# --- next_milestone -------------------------------------------------------------------------------


def milestone_cases(rng):
    found = []
    for lifetime in (
        0,
        99,
        100,
        101,
        499,
        500,
        999,
        1_000,
        4_999,
        5_000,
        9_999,
        10_000,
        49_999,
        50_000,
        60_000,
    ):
        found.append(
            ("reviews", {"lifetime_reviews": lifetime, "streak_days": 0, "mature_total": 0})
        )
    for streak in (0, 6, 7, 29, 30, 99, 100, 364, 365, 999, 1_000, 2_000):
        found.append(
            ("streak", {"lifetime_reviews": 50_000, "streak_days": streak, "mature_total": 5_000})
        )
    for mature in (0, 99, 100, 499, 500, 999, 1_000, 4_999, 5_000, 6_000):
        found.append(
            ("mature", {"lifetime_reviews": 50_000, "streak_days": 1_000, "mature_total": mature})
        )
    found.append(
        ("all-complete", {"lifetime_reviews": 50_000, "streak_days": 1_000, "mature_total": 5_000})
    )
    # Equal remaining fractions: 90 of 100 reviews against 6.3 of 7 is not an integer, so tie
    # the fractions exactly with 50 of 100 reviews and 50 of 100 mature, and 15 of 30 streak.
    found.append(
        ("tie-reviews-mature", {"lifetime_reviews": 50, "streak_days": 0, "mature_total": 50})
    )
    found.append(
        ("tie-reviews-streak", {"lifetime_reviews": 500, "streak_days": 0, "mature_total": 0})
    )
    found.append(
        ("tie-streak-mature", {"lifetime_reviews": 50_000, "streak_days": 50, "mature_total": 50})
    )
    for _ in range(10):
        found.append(
            (
                "drawn",
                {
                    "lifetime_reviews": rng.randrange(0, 60_000),
                    "streak_days": rng.randrange(0, 1_200),
                    "mature_total": rng.randrange(0, 6_000),
                },
            )
        )
    return found


def with_the_totals(next_milestone, predecessor, **totals):
    m = next_milestone(**totals)
    return {
        "label": m.label,
        "emoji": m.emoji,
        "current": m.current,
        "target": m.target,
        "pct": m.pct,
        "remaining": m.remaining,
    }


FUNCTIONS = {
    "badge_catalog": {
        "kind": "adapter",
        "function": "constants.BADGES",
        "adapter": with_the_catalog,
        "note": "Reads the catalog and returns each badge's key, name, emoji, description and "
        "tier, in catalog order.",
        "cases": catalog_cases,
    },
    "badge_conditions": {
        "kind": "adapter",
        "function": "gamification.badges._conditions",
        "adapter": with_the_inputs,
        "note": "Builds a BadgeContext from the case's flat inputs and returns what the "
        "conditions answer for it.",
        "cases": conditions_cases,
    },
    "badge_context": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._evaluate_and_award",
        "adapter": with_a_stub_pipeline,
        "note": "Drives it on a stand-in pipeline over a stub store holding the case's rollups, "
        "with badges.evaluate wrapped to record the context it receives; returns that context's "
        "fields and the keys awarded.",
        "cases": pipeline_cases,
    },
    "count_reviews_in_local_hours": {
        "kind": "adapter",
        "function": "analytics.count_reviews_in_local_hours",
        "adapter": with_the_window,
        "note": "Builds synthetic reviews and a collection configuration from the case's offset "
        "and rollover and counts the reviews in the hour window.",
        "cases": window_cases,
    },
    "badges.constants": {
        "kind": "constants",
        "names": [
            "constants.LIFETIME_REVIEWS_GRINDER",
            "constants.LIFETIME_REVIEWS_MARATHONER",
            "constants.CENTURION_DAY_REVIEWS",
            "constants.SHARPSHOOTER_RETENTION",
            "constants.SHARPSHOOTER_MIN_REVIEWS",
            "constants.SNIPER_ELITE_RETENTION",
            "constants.SNIPER_ELITE_MIN_MATURE",
            "constants.BACKLOG_SLAYER_CLEARED",
            "constants.NIGHT_OWL_REVIEWS",
            "constants.EARLY_BIRD_REVIEWS",
            "constants.EARLY_BIRD_END_HOUR",
            "constants.MATURITY_MILESTONE_COUNT",
            "constants.FOREST_GUARDIAN_COUNT",
            "constants.POLYGLOT_DECKS_DAY",
            "constants.GLOBETROTTER_DECKS_WEEK",
            "constants.PERFECT_WEEK_SCORE",
            "constants.SPEED_DEMON_REVIEWS",
            "constants.SPEED_DEMON_AVG_SECONDS",
            "constants.IRON_WILL_DAYS",
            "gamification.badges.LEECH_TAMER_MIN_LIFETIME",
        ],
    },
    "detect_records": {
        "kind": "adapter",
        "function": "gamification.rewards.detect_records",
        "adapter": with_the_rollups,
        "note": "Calls it with the case's rollup rows and stored bests and returns each record "
        "as its kind, value and label.",
        "cases": detect_cases,
    },
    "records_window": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._compute_and_store_coaching",
        "adapter": with_a_stub_store,
        "note": "Drives it on a stand-in pipeline over a stub store that records the rollup "
        "limit and holds the case's stored records, with coaching.compute_all patched to return "
        "the case's new records; returns the limit, the rows written, the celebration keys "
        "with each day as a day token, and whether the seed ran.",
        "cases": window_store_cases,
    },
    "records_chase": {
        "kind": "adapter",
        "function": "bot.CommandBot._render_records",
        "adapter": with_a_board,
        "note": "Renders a board whose days are day tokens and returns the record to chase and "
        "its gap, read from the rendered text, or none.",
        "cases": chase_cases,
    },
    "next_milestone": {
        "kind": "adapter",
        "function": "gamification.rewards.next_milestone",
        "adapter": with_the_totals,
        "note": "Calls it with the case's three totals and returns the milestone's label, "
        "emoji, current value, target, percentage and remainder.",
        "cases": milestone_cases,
    },
    "rewards.constants": {
        "kind": "constants",
        "names": [
            "gamification.rewards.REVIEW_LADDER",
            "gamification.rewards.STREAK_LADDER",
            "gamification.rewards.MATURE_LADDER",
            "gamification.rewards._LADDER_META",
            "gamification.rewards._RECORD_META",
        ],
    },
}

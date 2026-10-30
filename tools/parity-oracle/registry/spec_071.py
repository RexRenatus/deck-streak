"""SPEC-071's registrations: the day's metrics, the card snapshot, the collection day number, the
per-language metrics, the raw streak, the volume baseline and the rows it is taken over, the
five-pillar score with each pillar, the grade band, and the constants the port uses verbatim.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_reviews` drives `analytics.py:compute_daily_metrics` on `Review` objects built from the
  case's rows, a `CollectionConfig` of the case's rollover hour and offset, the day as a `date`
  from its epoch day number, and the card-to-home-deck map. It returns every `DailyMetrics` field.
* `with_cards` drives `analytics.py:compute_card_snapshot` on `Card` objects built from the case's
  rows and a `CollectionConfig` holding the case's leech threshold, or the config's own default
  when the case names none. It returns every `CardStateSnapshot` field.
* `with_a_creation_instant` drives `analytics.py:today_day_number` on a `CollectionConfig` of the
  case's rollover hour, offset and creation instant in seconds.
* `with_synthetic_courses` drives `cross_language.py:language_daily_metrics` with the
  `LANGUAGE_DECKS` binding that `progress.py` imports replaced by the case's synthetic courses, on
  the case's reviews, card-to-deck map and deck names. It returns the rows as a list sorted by day
  and code.
* `with_days_as_dates` drives `analytics.py:raw_streak` on the case's days and today as dates.
* `with_a_stub_store` drives `pipeline.py:GamifyPipeline._baseline` on a stand-in pipeline whose
  stub store answers `get_recent_rollups(limit)` with the case's rollup rows, most recent first,
  cut at the limit asked for; each row's day is handed over as the ISO text the store holds.
* `with_a_breakdown` drives `scoring.py:compute_score` on a `DailyMetrics` and a
  `CardStateSnapshot`, or none, holding the case's fields and zero elsewhere, and returns every
  `ScoreBreakdown` field.

The case builders draw only from the `random.Random` the generator seeds. Every course, deck name
and number is synthetic, every day is an epoch day number, and every instant epoch milliseconds.
"""

import asyncio
import dataclasses
import datetime as dt
import types
from unittest import mock

#: Anki's deck-name separator: the text before the first one is the top-level name.
SEP = "\x1f"
DAY_MS = 86_400_000
HOUR_MS = 3_600_000
#: A study day number near the present, as a whole-day offset from the Unix epoch.
PRESENT_DAY = 20_000
#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: Synthetic courses: a deck root, then the code, the name and the flag. Codes come from the ISO 639
#: range reserved for local use, so no course of anyone's is named.
COURSES = (
    ("Qaa", "qaa", "Course Qaa", "\U0001f3f3"),
    ("Qab Deck", "qab", "Course Qab", "\U0001f3f3"),
    ("Qac", "qac", "Course Qac", "\U0001f3f3"),
)
#: Synthetic deck names: roots, subdecks, a deck whose name only starts with a root, a subdeck named
#: like a root, and decks of no course.
DECK_NAMES = (
    "Qaa",
    f"Qaa{SEP}Unit 01",
    f"Qaa{SEP}Unit 02{SEP}Drill",
    "Qab Deck",
    f"Qab Deck{SEP}Unit 03",
    "Qaa Arts",
    "Qab",
    f"Other{SEP}Qaa",
    "Qac",
    "Default",
)


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def review(predecessor, row):
    """A `Review` from a row of id, card, ease, interval, last interval, factor, time and type."""
    id_ms, cid, ease, ivl, last_ivl, factor, time_ms, rtype = row
    return predecessor("types.Review")(
        id_ms=id_ms,
        cid=cid,
        ease=ease,
        ivl=ivl,
        last_ivl=last_ivl,
        factor=factor,
        time_ms=time_ms,
        rtype=rtype,
    )


def config(predecessor, **fields):
    return predecessor("types.CollectionConfig")(**fields)


def with_reviews(
    compute_daily_metrics,
    predecessor,
    *,
    rollover_hour,
    utc_offset_minutes,
    day,
    reviews,
    card_decks,
):
    """Call the day's metrics on the case's reviews, config, day and card-to-deck map."""
    metrics = compute_daily_metrics(
        [review(predecessor, row) for row in reviews],
        config(predecessor, rollover_hour=rollover_hour, tz_offset_minutes=utc_offset_minutes),
        as_date(day),
        {cid: did for cid, did in card_decks},
    )
    return dataclasses.asdict(metrics)


def local_instant(rng, day, rollover_hour, utc_offset_minutes):
    """An instant inside study day `day` for the rollover hour and offset."""
    start = day * DAY_MS + rollover_hour * HOUR_MS - utc_offset_minutes * 60_000
    return start + rng.randrange(0, DAY_MS)


def review_row(rng, instant, cid, *, rtype=None, ease=None):
    """A review row at `instant` of card `cid`, drawn from the shapes a revlog holds."""
    rtype = rng.choice((0, 1, 1, 1, 2, 3)) if rtype is None else rtype
    ease = rng.randrange(1, 5) if ease is None else ease
    last_ivl = rng.choice((-600, -60, 0, 1, 3, 10, 20, 21, 22, 40, 400))
    ivl = rng.choice((-600, 1, 4, 15, 20, 21, 25, 60, 900))
    time_ms = rng.choice((0, 350, 1_200, 7_777, 19_999, 59_999, 60_000, 60_001, 250_000))
    return [instant, cid, ease, ivl, last_ivl, rng.randrange(1300, 3000), time_ms, rtype]


def metrics_case(rng, count, *, hour=4, offset=0, days_around=1):
    day = PRESENT_DAY + rng.randrange(-50, 50)
    cids = [1_600_000_000_000 + index for index in range(1, 12)]
    rows = []
    for _ in range(count):
        on = day + rng.randrange(-days_around, days_around + 1)
        rows.append(review_row(rng, local_instant(rng, on, hour, offset), rng.choice(cids)))
    rows.sort(key=lambda row: row[0])
    decks = [[cid, rng.choice((1, 1_700_000_000_001, 1_700_000_000_002))] for cid in cids[:8]]
    return {
        "rollover_hour": hour,
        "utc_offset_minutes": offset,
        "day": day,
        "reviews": rows,
        "card_decks": decks,
    }


def metrics_cases(rng):
    """No review, reviews on other days only, first answers at one instant, the maturity boundary,
    the rollover's edges, non-study rows, the time cap, many answers, then drawn days."""
    drawn = []
    empty = metrics_case(rng, 0)
    drawn.append(("empty", empty))
    elsewhere = metrics_case(rng, 0)
    elsewhere["reviews"] = [
        review_row(rng, local_instant(rng, elsewhere["day"] + shift, 4, 0), 1_600_000_000_001)
        for shift in (-2, -1, 1, 2)
    ]
    drawn.append(("empty", elsewhere))
    day = PRESENT_DAY + 3
    at = day * DAY_MS + 4 * HOUR_MS + 5_000_000
    same_instant = [
        [at, 7, 1, 30, 25, 2500, 4_000, 1],
        [at, 7, 3, 30, 25, 2500, 5_000, 1],
        [at, 8, 3, 30, 25, 2500, 6_000, 1],
        [at, 8, 1, 30, 25, 2500, 7_000, 1],
        [at + 1, 9, 2, 30, 25, 2500, 8_000, 1],
    ]
    drawn.append(
        (
            "tie",
            {
                "rollover_hour": 4,
                "utc_offset_minutes": 0,
                "day": day,
                "reviews": same_instant,
                "card_decks": [[7, 1], [8, 1], [9, 2]],
            },
        )
    )
    boundary = [
        [at + index, 20 + index, ease, ivl, last_ivl, 2500, 3_000, rtype]
        for index, (ease, ivl, last_ivl, rtype) in enumerate(
            (
                (3, 21, 20, 1),
                (3, 21, 21, 1),
                (3, 20, 19, 1),
                (1, 21, 20, 1),
                (3, 22, 20, 2),
                (2, 45, 30, 1),
                (1, 1, 30, 1),
                (4, 21, -600, 1),
                (3, 21, 20, 0),
            )
        )
    ]
    drawn.append(
        (
            "mature-boundary",
            {
                "rollover_hour": 4,
                "utc_offset_minutes": 0,
                "day": day,
                "reviews": boundary,
                "card_decks": [[20 + index, 1] for index in range(9)],
            },
        )
    )
    for hour, offset in ((4, 0), (0, 0), (23, 0), (4, 330), (4, -300), (6, 840), (4, -720)):
        start = day * DAY_MS + hour * HOUR_MS - offset * 60_000
        edges = [
            [start - 1, 30, 3, 5, 3, 2500, 1_000, 1],
            [start, 31, 3, 5, 3, 2500, 1_000, 1],
            [start + DAY_MS - 1, 32, 3, 5, 3, 2500, 1_000, 1],
            [start + DAY_MS, 33, 3, 5, 3, 2500, 1_000, 1],
        ]
        drawn.append(
            (
                "rollover",
                {
                    "rollover_hour": hour,
                    "utc_offset_minutes": offset,
                    "day": day,
                    "reviews": edges,
                    "card_decks": [[30, 1], [31, 2], [32, 3], [33, 4]],
                },
            )
        )
    bookkeeping = [
        [at + 10, 40, 0, 5, 3, 2500, 9_000, 1],
        [at + 11, 41, 3, 5, 3, 2500, 9_000, 4],
        [at + 12, 42, 3, 5, 3, 2500, 9_000, 5],
        [at + 13, 43, 3, 5, 3, 2500, 9_000, 1],
    ]
    drawn.append(
        (
            "bookkeeping",
            {
                "rollover_hour": 4,
                "utc_offset_minutes": 0,
                "day": day,
                "reviews": bookkeeping,
                "card_decks": [],
            },
        )
    )
    many = metrics_case(rng, 400, days_around=0)
    drawn.append(("many", many))
    for _ in range(10):
        drawn.append(
            (
                None,
                metrics_case(
                    rng,
                    rng.randrange(0, 60),
                    hour=rng.choice((0, 4, 4, 5)),
                    offset=rng.choice((0, 60, -240, 330)),
                ),
            )
        )
    return drawn


def with_cards(compute_card_snapshot, predecessor, *, leech_threshold, day_number, cards):
    """Call the snapshot on the case's cards (id, queue, type, due, interval, lapses) at the day
    number, with the case's leech threshold or, when it names none, the config's default."""
    fields = {} if leech_threshold is None else {"leech_threshold": leech_threshold}
    built = [
        predecessor("types.Card")(
            id=cid,
            nid=cid,
            did=1,
            queue=queue,
            ctype=ctype,
            due=due,
            ivl=ivl,
            factor=2500,
            reps=0,
            lapses=lapses,
            odid=0,
        )
        for cid, queue, ctype, due, ivl, lapses in cards
    ]
    snapshot = compute_card_snapshot(built, config(predecessor, **fields), day_number)
    return dataclasses.asdict(snapshot)


def card_row(rng, cid, day_number):
    queue = rng.choice((-3, -2, -1, 0, 1, 2, 2, 2, 3))
    ctype = {0: 0, 1: 1, 3: 3}.get(queue, rng.choice((0, 1, 2, 2, 3)))
    due = day_number + rng.randrange(-5, 6) if queue in (2, -1, -2, -3) else rng.randrange(0, 900)
    ivl = rng.choice((-600, 0, 1, 7, 20, 21, 22, 100))
    return [cid, queue, ctype, due, ivl, rng.randrange(0, 14)]


def snapshot_cases(rng):
    """The due boundary, buried and suspended leeches at the threshold, the default threshold, an
    empty collection, then drawn collections."""
    number = 1_234
    due = [
        [1, 2, 2, number - 1, 30, 0],
        [2, 2, 2, number, 30, 0],
        [3, 2, 2, number + 1, 30, 0],
        [4, -1, 2, number - 1, 30, 0],
        [5, -2, 2, number, 30, 0],
        [6, 1, 1, number - 1, -600, 0],
        [7, 3, 3, number, -600, 0],
    ]
    leeches = [
        [10, -1, 2, number, 30, 8],
        [11, -2, 2, number, 30, 8],
        [12, -3, 2, number, 30, 9],
        [13, 2, 2, number, 30, 7],
        [14, 2, 2, number, 30, 8],
        [15, 0, 0, 5, 0, 12],
        [16, 2, 2, number + 3, 21, 20],
        [17, 2, 2, number + 3, 20, 3],
        [18, 2, 2, number + 3, 0, 3],
        [19, 2, 2, number + 3, -60, 3],
    ]
    drawn = [
        ("due-boundary", {"leech_threshold": 8, "day_number": number, "cards": due}),
        ("buried-leech", {"leech_threshold": 8, "day_number": number, "cards": leeches}),
        ("buried-leech", {"leech_threshold": 9, "day_number": number, "cards": leeches}),
        ("default-threshold", {"leech_threshold": None, "day_number": number, "cards": leeches}),
        ("default-threshold", {"leech_threshold": None, "day_number": number, "cards": due}),
        ("empty", {"leech_threshold": 8, "day_number": number, "cards": []}),
    ]
    for _ in range(8):
        day_number = rng.randrange(0, 3_000)
        cards = [card_row(rng, cid, day_number) for cid in range(1, rng.randrange(1, 80))]
        threshold = rng.choice((None, 4, 8, 12))
        drawn.append(
            (None, {"leech_threshold": threshold, "day_number": day_number, "cards": cards})
        )
    return drawn


def with_a_creation_instant(
    today_day_number, predecessor, *, rollover_hour, utc_offset_minutes, creation_sec, now_ms
):
    """Call the day number on a config of the case's rollover hour, offset and creation instant."""
    built = config(
        predecessor,
        rollover_hour=rollover_hour,
        tz_offset_minutes=utc_offset_minutes,
        creation_sec=creation_sec,
    )
    return today_day_number(built, now_ms)


def day_number_cases(rng):
    """Instants on both sides of the rollover, for the creation instant and for now, then draws."""
    drawn = []
    created_day = PRESENT_DAY - 2_000
    for hour, offset in ((4, 0), (0, 0), (23, 0), (4, 330), (4, -300), (5, 840), (4, -720)):
        created = (created_day * DAY_MS + hour * HOUR_MS - offset * 60_000) // 1000
        now = PRESENT_DAY * DAY_MS + hour * HOUR_MS - offset * 60_000
        for creation_sec in (created - 1, created, created + 1):
            for now_ms in (now - 1, now, now + 1):
                drawn.append(
                    (
                        "rollover",
                        {
                            "rollover_hour": hour,
                            "utc_offset_minutes": offset,
                            "creation_sec": creation_sec,
                            "now_ms": now_ms,
                        },
                    )
                )
    for _ in range(10):
        drawn.append(
            (
                None,
                {
                    "rollover_hour": rng.randrange(0, 24),
                    "utc_offset_minutes": rng.randrange(-720, 841),
                    "creation_sec": rng.randrange(1_300_000_000, 1_700_000_000),
                    "now_ms": rng.randrange(1_700_000_000_000, 1_800_000_000_000),
                },
            )
        )
    return drawn


def with_synthetic_courses(
    language_daily_metrics,
    predecessor,
    *,
    rollover_hour,
    utc_offset_minutes,
    courses,
    deck_names,
    card_decks,
    reviews,
    days,
):
    """Call the per-language metrics with `progress.py`'s course table replaced by the case's."""
    table = {root: (code, name, flag) for root, code, name, flag in courses}
    with mock.patch.object(predecessor("progress"), "LANGUAGE_DECKS", table):
        rows = language_daily_metrics(
            [review(predecessor, row) for row in reviews],
            config(predecessor, rollover_hour=rollover_hour, tz_offset_minutes=utc_offset_minutes),
            {cid: did for cid, did in card_decks},
            {did: name for did, name in deck_names},
            days={as_date(day) for day in days},
        )
    return sorted(
        [[day, code, *numbers] for (day, code), numbers in rows.items()],
        key=lambda row: (row[0], row[1]),
    )


def language_case(rng, count):
    day = PRESENT_DAY + rng.randrange(-40, 40)
    decks = [[index + 1, name] for index, name in enumerate(DECK_NAMES)]
    cids = list(range(100, 130))
    card_decks = [[cid, rng.randrange(1, len(DECK_NAMES) + 2)] for cid in cids[:26]]
    rows = []
    for _ in range(count):
        on = day + rng.randrange(-2, 3)
        rows.append(review_row(rng, local_instant(rng, on, 4, 0), rng.choice(cids)))
    rows.sort(key=lambda row: row[0])
    return {
        "rollover_hour": 4,
        "utc_offset_minutes": 0,
        "courses": [list(course) for course in COURSES],
        "deck_names": decks,
        "card_decks": card_decks,
        "reviews": rows,
        "days": sorted({day + rng.randrange(-2, 3) for _ in range(3)}),
    }


def language_cases(rng):
    """A deck whose name only starts with a root, a subdeck named like a root, cards of no known
    deck, no course at all, then drawn days."""
    day = PRESENT_DAY
    at = day * DAY_MS + 4 * HOUR_MS + 1_000_000
    prefix_only = {
        "rollover_hour": 4,
        "utc_offset_minutes": 0,
        "courses": [list(course) for course in COURSES],
        "deck_names": [[index + 1, name] for index, name in enumerate(DECK_NAMES)],
        "card_decks": [[1, 1], [2, 2], [3, 6], [4, 7], [5, 8], [6, 4], [7, 99]],
        "reviews": [
            [at + cid, cid, 3, 30, 25, 2500, 5_000 * cid, 1] for cid in (1, 2, 3, 4, 5, 6, 7, 8)
        ],
        "days": [day],
    }
    none = dict(prefix_only, courses=[])
    drawn = [("prefix-only-deck", prefix_only), ("no-course", none)]
    for _ in range(8):
        drawn.append((None, language_case(rng, rng.randrange(0, 80))))
    return drawn


def with_days_as_dates(raw_streak, predecessor, *, days, today):
    """Call the raw streak on the case's days and today as dates."""
    return raw_streak({as_date(day) for day in days}, as_date(today))


def streak_cases(rng):
    """Today studied, today unfinished, a gap, no day, then drawn sets."""
    today = PRESENT_DAY
    drawn = [
        ("today-studied", {"days": [today - 2, today - 1, today], "today": today}),
        ("today-unfinished", {"days": [today - 3, today - 2, today - 1], "today": today}),
        ("gap", {"days": [today - 5, today - 4, today - 2, today - 1, today], "today": today}),
        ("gap", {"days": [today - 3, today - 2], "today": today}),
        ("empty", {"days": [], "today": today}),
        ("future", {"days": [today + 1, today + 2], "today": today}),
    ]
    for _ in range(8):
        days = sorted({today - rng.randrange(0, 40) for _ in range(rng.randrange(0, 40))})
        drawn.append((None, {"days": days, "today": today}))
    return drawn


def baseline_rows(rng, count):
    return [
        {
            "reviews": rng.choice((0, 0, 3, 9, 10, 11, 25, 60, 140)),
            "seconds": rng.randrange(0, 9_000),
        }
        for _ in range(count)
    ]


def volume_cases(rng):
    """Floors on sparse history, no active row, an even count, then drawn rows."""
    drawn = [
        ("floor", {"rollups": []}),
        ("floor", {"rollups": [{"reviews": 0, "seconds": 0}, {"reviews": 0, "seconds": 90}]}),
        ("floor", {"rollups": [{"reviews": 3, "seconds": 60}, {"reviews": 9, "seconds": 120}]}),
        ("even", {"rollups": [{"reviews": 11, "seconds": 700}, {"reviews": 40, "seconds": 900}]}),
        (
            "odd",
            {
                "rollups": [
                    {"reviews": 30, "seconds": 1_000},
                    {"reviews": 12, "seconds": 2_000},
                    {"reviews": 90, "seconds": 3_001},
                ]
            },
        ),
    ]
    for _ in range(8):
        drawn.append((None, {"rollups": baseline_rows(rng, rng.randrange(0, 45))}))
    return drawn


class StubRollups:
    """Answers `get_recent_rollups(limit)` with the case's rows, most recent first, cut at the
    limit asked for, each day as the ISO text the store holds."""

    def __init__(self, rows):
        self.rows = rows

    async def get_recent_rollups(self, limit):
        return [
            {
                "day": as_date(row["day"]).isoformat(),
                "reviews": row["reviews"],
                "seconds": row["seconds"],
            }
            for row in self.rows[:limit]
        ]


def with_a_stub_store(baseline, predecessor, *, today, rollups):
    """Run the baseline on a stand-in pipeline whose store answers with the case's rows."""
    stand_in = types.SimpleNamespace(_store=StubRollups(rollups))
    return list(asyncio.run(baseline(stand_in, as_date(today))))


def window_rows(rng, today, count, *, with_today):
    days = sorted({today - rng.randrange(1, 90) for _ in range(count)}, reverse=True)
    if with_today:
        days = [today, *days]
    return [dict(day=day, **row) for day, row in zip(days, baseline_rows(rng, len(days)))]


def window_cases(rng):
    """Today's row among the recent ones, more rows than the window, zero-review rows, then draws."""
    today = PRESENT_DAY
    drawn = [
        ("today-row", {"today": today, "rollups": window_rows(rng, today, 40, with_today=True)}),
        (
            "no-today-row",
            {"today": today, "rollups": window_rows(rng, today, 40, with_today=False)},
        ),
        (
            "zero-days",
            {
                "today": today,
                "rollups": [
                    {
                        "day": today - index,
                        "reviews": 0 if index % 3 else 20 + index,
                        "seconds": 600,
                    }
                    for index in range(0, 45)
                ],
            },
        ),
        ("empty", {"today": today, "rollups": []}),
        (
            "today-only",
            {"today": today, "rollups": [{"day": today, "reviews": 50, "seconds": 900}]},
        ),
    ]
    for _ in range(8):
        drawn.append(
            (
                None,
                {
                    "today": today,
                    "rollups": window_rows(
                        rng, today, rng.randrange(0, 50), with_today=rng.random() < 0.5
                    ),
                },
            )
        )
    return drawn


def with_a_breakdown(
    compute_score, predecessor, *, metrics, snapshot, streak_days, base_reviews, base_minutes
):
    """Call the score on the case's metrics and snapshot, zero in every field the case omits."""
    daily = predecessor("types.DailyMetrics")(
        day=as_date(0),
        reviews=metrics["reviews"],
        learn_count=0,
        review_count=metrics["review_count"],
        relearn_count=0,
        filtered_count=0,
        seconds=metrics["seconds"],
        answered=metrics["answered"],
        passed=0,
        true_retention=metrics["true_retention"],
        graduations=metrics["graduations"],
        decks_studied=0,
        avg_answer_seconds=0.0,
    )
    state = None
    if snapshot is not None:
        state = predecessor("types.CardStateSnapshot")(
            total_cards=0,
            mature_count=0,
            young_count=0,
            learning_count=0,
            suspended_count=0,
            leech_active=snapshot["leech_active"],
            backlog=snapshot["backlog"],
            due_today=snapshot["due_today"],
        )
    breakdown = compute_score(
        metrics=daily,
        snapshot=state,
        streak_days=streak_days,
        base_reviews=base_reviews,
        base_minutes=base_minutes,
    )
    return dataclasses.asdict(breakdown)


def score_case(rng, *, historical):
    reviews = rng.choice((0, 1, 4, 12, 30, 75, 160))
    answered = min(reviews, rng.randrange(0, 40))
    return {
        "metrics": {
            "reviews": reviews,
            "review_count": min(reviews, rng.randrange(0, 90)),
            "seconds": rng.choice((0.0, 59.5, 600.0, 1_234.25, 3_600.0)) if reviews else 0.0,
            "answered": answered,
            "true_retention": rng.choice((0.0, 40.0, 50.0, 72.5, 88.8, 95.0, 100.0))
            if answered
            else 0.0,
            "graduations": rng.randrange(0, 6),
        },
        "snapshot": None
        if historical
        else {
            "due_today": rng.randrange(0, 120),
            "backlog": rng.choice((0, 0, 3, 40, 250)),
            "leech_active": rng.randrange(0, 15),
        },
        "streak_days": rng.randrange(0, 40),
        "base_reviews": rng.choice((10.0, 12, 33.5, 80.0)),
        "base_minutes": rng.choice((5.0, 7.25, 30.0)),
    }


def score_cases(rng):
    """A total at exactly one half, the historical form, nothing studied, then drawn days."""
    idle = {
        "reviews": 0,
        "review_count": 0,
        "seconds": 0.0,
        "answered": 0,
        "true_retention": 0.0,
        "graduations": 3,
    }
    drawn = [
        (
            "tie",
            {
                "metrics": idle,
                "snapshot": {"due_today": 0, "backlog": 0, "leech_active": 5},
                "streak_days": 0,
                "base_reviews": 10.0,
                "base_minutes": 5.0,
            },
        ),
        (
            "historical",
            {
                "metrics": idle,
                "snapshot": None,
                "streak_days": 4,
                "base_reviews": 10.0,
                "base_minutes": 5.0,
            },
        ),
    ]
    for historical in (False, True):
        for _ in range(8):
            drawn.append(
                ("historical" if historical else None, score_case(rng, historical=historical))
            )
    return drawn


def consistency_cases(rng):
    drawn = [
        ("bound", {"reviews": reviews, "streak_days": streak})
        for reviews in (0, 1)
        for streak in (0, 1, 24, 25, 26, 100)
    ]
    for _ in range(6):
        drawn.append((None, {"reviews": rng.randrange(0, 50), "streak_days": rng.randrange(0, 60)}))
    return drawn


def retention_cases(rng):
    drawn = [
        ("small-sample", {"true_retention": value, "answered": answered})
        for answered in (1, 4, 5)
        for value in (0.0, 50.0, 70.0, 95.0, 100.0)
    ]
    drawn.append(("none", {"true_retention": 80.0, "answered": 0}))
    for _ in range(8):
        drawn.append(
            (
                None,
                {
                    "true_retention": round(rng.uniform(0.0, 100.0), 3),
                    "answered": rng.randrange(0, 40),
                },
            )
        )
    return drawn


def workload_cases(rng):
    drawn = [
        ("idle", {"review_count": 5, "due_today": 3, "backlog": 0, "studied": False}),
        ("clear", {"review_count": 0, "due_today": 0, "backlog": 0, "studied": True}),
        ("cap", {"review_count": 10, "due_today": 0, "backlog": 500, "studied": True}),
        ("cap", {"review_count": 10, "due_today": 0, "backlog": 200, "studied": True}),
    ]
    for _ in range(8):
        drawn.append(
            (
                None,
                {
                    "review_count": rng.randrange(0, 100),
                    "due_today": rng.randrange(0, 100),
                    "backlog": rng.randrange(0, 300),
                    "studied": rng.random() < 0.8,
                },
            )
        )
    return drawn


def volume_score_cases(rng):
    drawn = [
        ("floor", {"reviews": 5, "minutes": 2.0, "base_reviews": 0.0, "base_minutes": 0.5}),
        ("cap", {"reviews": 500, "minutes": 300.0, "base_reviews": 10.0, "base_minutes": 5.0}),
        ("none", {"reviews": 0, "minutes": 0.0, "base_reviews": 10.0, "base_minutes": 5.0}),
    ]
    for _ in range(8):
        drawn.append(
            (
                None,
                {
                    "reviews": rng.randrange(0, 200),
                    "minutes": round(rng.uniform(0.0, 90.0), 4),
                    "base_reviews": rng.choice((10.0, 14, 22.5, 60.0)),
                    "base_minutes": rng.choice((5.0, 9.5, 31.0)),
                },
            )
        )
    return drawn


def mastery_cases(rng):
    drawn = [
        ("bound", {"graduations": graduations, "leech_active": leeches})
        for graduations in (0, 1, 2, 3, 4)
        for leeches in (0, 1, 9, 10, 11)
    ]
    for _ in range(6):
        drawn.append(
            (None, {"graduations": rng.randrange(0, 10), "leech_active": rng.randrange(0, 20)})
        )
    return drawn


def grade_cases(rng):
    edges = [-1, 0, 1, 2, 39, 40, 41, 59, 60, 61, 74, 75, 76, 89, 90, 91, 100]
    drawn = [("threshold", {"score": score}) for score in edges]
    for _ in range(6):
        drawn.append((None, {"score": rng.randrange(0, 101)}))
    return drawn


FUNCTIONS = {
    "daily_metrics": {
        "kind": "adapter",
        "function": "analytics.compute_daily_metrics",
        "adapter": with_reviews,
        "note": (
            "Builds Review objects from the case's rows, a CollectionConfig of its rollover hour "
            "and offset, the day as a date from its epoch day number and the card-to-home-deck "
            "map, calls the day's metrics and returns every DailyMetrics field."
        ),
        "cases": metrics_cases,
    },
    "card_snapshot": {
        "kind": "adapter",
        "function": "analytics.compute_card_snapshot",
        "adapter": with_cards,
        "note": (
            "Builds Card objects from the case's rows (id, queue, type, due, interval, lapses; the "
            "note is the card's id, the deck 1, the factor 2500, no rep and no filtered deck) and a "
            "CollectionConfig holding the case's leech threshold, or its own default when the "
            "threshold is null, calls the snapshot and returns every CardStateSnapshot field."
        ),
        "cases": snapshot_cases,
    },
    "today_day_number": {
        "kind": "adapter",
        "function": "analytics.today_day_number",
        "adapter": with_a_creation_instant,
        "note": (
            "Builds a CollectionConfig of the case's rollover hour, offset and creation instant in "
            "seconds and calls the day number at the case's instant."
        ),
        "cases": day_number_cases,
    },
    "language_daily_metrics": {
        "kind": "adapter",
        "function": "cross_language.language_daily_metrics",
        "adapter": with_synthetic_courses,
        "note": (
            "Replaces the LANGUAGE_DECKS binding that progress.py imports with the case's synthetic "
            "courses (deck root, code, name, flag), builds the reviews, the config, the card-to-"
            "deck map, the deck names and the days, calls the per-language metrics and returns "
            "the rows as [day, code, reviews, seconds, answered, passed] sorted by day and code."
        ),
        "cases": language_cases,
    },
    "raw_streak": {
        "kind": "adapter",
        "function": "analytics.raw_streak",
        "adapter": with_days_as_dates,
        "note": "Turns the case's day numbers and today into dates and calls the raw streak.",
        "cases": streak_cases,
    },
    "volume_baseline": {
        "kind": "function",
        "function": "analytics.volume_baseline",
        "cases": volume_cases,
    },
    "baseline_window": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._baseline",
        "adapter": with_a_stub_store,
        "note": (
            "Runs the baseline on a stand-in pipeline holding only a stub store, which answers "
            "get_recent_rollups(limit) with the case's rollup rows, most recent first, cut at the "
            "limit asked for, each row's day handed over as the ISO text the store holds; returns "
            "the baseline pair."
        ),
        "cases": window_cases,
    },
    "compute_score": {
        "kind": "adapter",
        "function": "scoring.compute_score",
        "adapter": with_a_breakdown,
        "note": (
            "Builds a DailyMetrics holding the case's reviews, review count, seconds, answered, "
            "retention and graduations, and a CardStateSnapshot holding its due today, backlog "
            "and leeches or none, each zero elsewhere, calls the score and returns every "
            "ScoreBreakdown field."
        ),
        "cases": score_cases,
    },
    "score_consistency": {
        "kind": "function",
        "function": "scoring._consistency",
        "cases": consistency_cases,
    },
    "score_retention": {
        "kind": "function",
        "function": "scoring._retention",
        "cases": retention_cases,
    },
    "score_workload": {
        "kind": "function",
        "function": "scoring._workload",
        "cases": workload_cases,
    },
    "score_volume": {
        "kind": "function",
        "function": "scoring._volume",
        "cases": volume_score_cases,
    },
    "score_mastery": {
        "kind": "function",
        "function": "scoring._mastery",
        "cases": mastery_cases,
    },
    "grade_band": {
        "kind": "function",
        "function": "scoring.grade_band",
        "cases": grade_cases,
    },
    "analytics.constants": {
        "kind": "constants",
        "names": [
            "constants.ANSWER_TIME_CAP_SECONDS",
            "constants.MATURE_IVL_DAYS",
            "constants.DEFAULT_LEECH_THRESHOLD",
            "constants.WEIGHT_CONSISTENCY",
            "constants.WEIGHT_RETENTION",
            "constants.WEIGHT_WORKLOAD",
            "constants.WEIGHT_VOLUME",
            "constants.WEIGHT_MASTERY",
            "constants.RETENTION_FLOOR_PCT",
            "constants.RETENTION_CEIL_PCT",
            "constants.RETENTION_MIN_SAMPLE",
            "constants.RETENTION_BLEND_TARGET",
            "constants.VOLUME_REVIEW_WEIGHT",
            "constants.VOLUME_TIME_WEIGHT",
            "constants.VOLUME_CAP_RATIO",
            "constants.MASTERY_GRADUATION_TARGET",
            "constants.MASTERY_LEECH_PENALTY",
            "constants.MASTERY_LEECH_PENALTY_CAP",
            "constants.WORKLOAD_BACKLOG_DIVISOR",
            "constants.WORKLOAD_BACKLOG_PENALTY_CAP",
            "constants.GRADE_BANDS",
        ],
    },
}

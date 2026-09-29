"""SPEC-072's registrations: a review's XP, the daily bonuses, the level's info and title, a note's
Bloom tier, the consistency multiplier and run, the one-miss preview, the on-pace run, the day's
bonuses, the Ascendant arming, the day base, and the constants the port uses verbatim.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_a_review` drives `gamification/xp.py:review_xp` on a `Review` built from the case's ease,
  new interval, type and card id, and on a card-to-tier map holding the case's tier for that card,
  or none.
* `with_the_five_inputs` drives `gamification/xp.py:daily_bonus_grants` and returns each grant as a
  source and an amount.
* `with_the_fields` drives `gamification/xp.py:level_info` and returns every `LevelInfo` field.
* `with_a_stub_rollup_store` drives `pipeline_layers/governor.py:GovernorLayer._on_pace_run` on a
  stand-in layer whose stub store answers `get_recent_rollups(limit)` with the case's rollups, most
  recent first, cut at the limit asked for, and `skip_days_set()` with the case's skip days; each
  day is handed over as the ISO text or the `date` the store holds, built from its epoch day
  number.
* `with_a_stub_day_store` drives `GovernorLayer._apply_day_bonuses` on a stand-in layer whose
  stub store answers the day base, the buff and the two review amounts of the case, and whose
  `_on_pace_run` answers the case's run; it returns the grants written.
* `with_a_stub_arming_store` drives `GovernorLayer._maybe_grant_ascendant` on a stand-in layer
  whose stub store holds the case's buff, skip day, previous day's rollup reviews and
  `backlog_zero` amount; it returns whether a buff row was written.
* `with_a_ledger` drives `database.py:GamifyStore.day_base_xp` on a temporary store the adapter
  fills with the case's synthetic ledger rows through the store's own `upsert_xp_grant`, each
  day turned from its epoch day number into the store's form.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic, every day an epoch day number.
"""

import asyncio
import datetime as dt
import tempfile
import types
from pathlib import Path

#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: A study day number near the present.
PRESENT_DAY = 20_000
#: A card id every review case uses.
CARD_ID = 1_600_000_000_001
#: The predecessor's fallbacks for a level with no title.
TIERS = ("T1", "T2", "T3", "T4")
#: Intervals a review case sits on: new, young, mature, and each side of the boundary.
INTERVALS = (0, 7, 21)
BOUNDARY_INTERVALS = (-1, 1, 20, 22, 365)


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


# --- review_xp ------------------------------------------------------------------------------------


def review_cases(rng):
    found = []
    for ease in (1, 2, 3, 4):
        for ivl in INTERVALS:
            for rtype in (0, 1, 2, 3):
                for tier in (None, *TIERS):
                    found.append(
                        ("combination", {"ease": ease, "ivl": ivl, "rtype": rtype, "tier": tier})
                    )
    # Products that land on a half: Python rounds them to the even neighbour.
    found.append(("tie", {"ease": 1, "ivl": 7, "rtype": 1, "tier": None}))
    found.append(("tie", {"ease": 1, "ivl": 7, "rtype": 1, "tier": "T3"}))
    found.append(("tie", {"ease": 1, "ivl": 7, "rtype": 1, "tier": "T1"}))
    for ivl in BOUNDARY_INTERVALS:
        found.append(("boundary", {"ease": 2, "ivl": ivl, "rtype": 1, "tier": None}))
    for ease, rtype in ((0, 1), (2, 4), (2, 5), (0, 4), (1, 6)):
        found.append(("non-study", {"ease": ease, "ivl": 30, "rtype": rtype, "tier": "T4"}))
    for _ in range(12):
        found.append(
            (
                "drawn",
                {
                    "ease": rng.randrange(1, 5),
                    "ivl": rng.randrange(0, 400),
                    "rtype": rng.randrange(0, 4),
                    "tier": rng.choice([None, *TIERS]),
                },
            )
        )
    return found


def with_a_review(review_xp, predecessor, *, ease, ivl, rtype, tier):
    """Build the `Review` and the card-to-tier map, then call the predecessor."""
    review = predecessor("types.Review")(
        id_ms=1_700_000_000_000,
        cid=CARD_ID,
        ease=ease,
        ivl=ivl,
        last_ivl=0,
        factor=2500,
        time_ms=5_000,
        rtype=rtype,
    )
    return review_xp(review, {CARD_ID: tier} if tier else None)


# --- daily_bonus_grants ---------------------------------------------------------------------------


def bonus_cases(rng):
    found = [
        ("none", dict(studied=False, backlog_zero=False, streak_days=0, score_total=0, graduations=0)),
        ("all", dict(studied=True, backlog_zero=True, streak_days=7, score_total=95, graduations=3)),
        ("cap-below", dict(studied=True, backlog_zero=False, streak_days=49, score_total=89, graduations=0)),
        ("cap-at", dict(studied=True, backlog_zero=False, streak_days=50, score_total=90, graduations=0)),
        ("cap-above", dict(studied=True, backlog_zero=False, streak_days=51, score_total=91, graduations=1)),
        ("score-edge", dict(studied=False, backlog_zero=False, streak_days=0, score_total=89, graduations=0)),
        ("streak-negative", dict(studied=True, backlog_zero=False, streak_days=-1, score_total=0, graduations=0)),
    ]
    for _ in range(10):
        found.append(
            (
                None,
                dict(
                    studied=rng.random() < 0.5,
                    backlog_zero=rng.random() < 0.5,
                    streak_days=rng.randrange(0, 120),
                    score_total=rng.randrange(0, 101),
                    graduations=rng.randrange(0, 12),
                ),
            )
        )
    return found


def with_the_five_inputs(grants_of, predecessor, **inputs):
    """Call the predecessor and return each grant as a source and an amount."""
    return [{"source": g.source, "amount": g.amount} for g in grants_of(**inputs)]


# --- level_info and level_title -------------------------------------------------------------------


def threshold(level):
    """The total the curve's `level` begins at: an input a case sits on."""
    return 50 * level * level - 50 * level


def info_cases(rng):
    found = []
    for level in (1, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110):
        found.append(("at", {"total_xp": threshold(level)}))
        if level > 1:
            found.append(("below", {"total_xp": threshold(level) - 1}))
    for _ in range(10):
        found.append((None, {"total_xp": rng.randrange(0, 600_000)}))
    return found


def with_the_fields(level_info, predecessor, *, total_xp):
    info = level_info(total_xp)
    return {
        "total_xp": info.total_xp,
        "level": info.level,
        "title": info.title,
        "emoji": info.emoji,
        "xp_into_level": info.xp_into_level,
        "xp_for_next": info.xp_for_next,
    }


def title_cases(rng):
    return [(None, {"level": level}) for level in range(1, 121)]


# --- parse_tier -----------------------------------------------------------------------------------


def tier_cases(rng):
    return [
        ("none", {"tags": ""}),
        ("none", {"tags": " Subject::Topic "}),
        ("each", {"tags": " T1 Subject::Topic "}),
        ("each", {"tags": "t2"}),
        ("each", {"tags": "Contracts T3"}),
        ("each", {"tags": "\tT4\n"}),
        ("inside", {"tags": "Subject::T4Something"}),
        ("inside", {"tags": "T1x xT2 T-3"}),
        ("two", {"tags": "T3 T1"}),
        ("two", {"tags": "T5 t4 T2"}),
        ("not-text", {"tags": None}),
        ("not-text", {"tags": 4}),
        ("not-text", {"tags": ["T1"]}),
    ]


# --- the multiplier and the run -------------------------------------------------------------------


def run_cases(rng):
    found = [("below", {"consecutive_on_pace_days": run}) for run in (-5, -1)]
    found += [("run", {"consecutive_on_pace_days": run}) for run in range(0, 14)]
    found += [("past-the-cap", {"consecutive_on_pace_days": run}) for run in (20, 100, 5000)]
    return found


def drop_cases(rng):
    return [(None, {"run": run}) for run in (0, 1, 2, 3, 4, 5, 9, 10, 11, 12, 13, 40, 5000)]


def fold_cases(rng):
    found = [
        ("empty", {"day_results": []}),
        ("floor", {"day_results": [[10, False], [10, False], [70, False]]}),
        ("skips", {"day_results": [[70, False], [0, True], [70, False], [5, True]]}),
        ("edge", {"day_results": [[59, False], [60, False], [61, False], [60, False]]}),
    ]
    for _ in range(12):
        found.append(
            (
                None,
                {
                    "day_results": [
                        [rng.randrange(0, 101), rng.random() < 0.2]
                        for _ in range(rng.randrange(1, 40))
                    ]
                },
            )
        )
    return found


class StubRollupStore:
    """Answers `get_recent_rollups(limit)` with the case's rows, most recent first, cut at the
    limit asked for, and `skip_days_set()` with the case's skip days."""

    def __init__(self, rollups, skips):
        self.rollups = sorted(rollups, key=lambda row: -row["day"])
        self.skips = skips

    async def get_recent_rollups(self, limit):
        return [
            {"day": as_date(row["day"]).isoformat(), "score": row["score"]}
            for row in self.rollups[:limit]
        ]

    async def skip_days_set(self):
        return {as_date(day) for day in self.skips}


def pace_rollups(rng, count, end):
    days = sorted(rng.sample(range(end - 120, end + 1), count))
    return [{"day": day, "score": rng.randrange(0, 101)} for day in days]


def pace_cases(rng):
    end = PRESENT_DAY
    found = [
        ("empty", {"rollups": [], "skips": [], "exclude_day": end}),
        (
            "gap",
            {
                "rollups": [
                    {"day": end - 4, "score": 80},
                    {"day": end - 3, "score": 90},
                    {"day": end - 1, "score": 70},
                    {"day": end, "score": 10},
                ],
                "skips": [],
                "exclude_day": end,
            },
        ),
        (
            "skip",
            {
                "rollups": [{"day": end - 3, "score": 80}, {"day": end - 1, "score": 0}],
                "skips": [end - 2],
                "exclude_day": end,
            },
        ),
    ]
    for count in (5, 60, 89, 90, 91, 100, 120):
        found.append(
            (
                "dense",
                {
                    "rollups": [
                        {"day": end - offset, "score": 60 + (offset * 7) % 40}
                        for offset in range(count)
                    ],
                    "skips": [end - 40, end - 41] if count > 60 else [],
                    "exclude_day": end,
                },
            )
        )
    for count in (10, 30, 89, 90, 91, 100):
        found.append(
            (
                "window",
                {
                    "rollups": pace_rollups(rng, count, end),
                    "skips": sorted(rng.sample(range(end - 100, end), 5)),
                    "exclude_day": end,
                },
            )
        )
    for _ in range(8):
        found.append(
            (
                None,
                {
                    "rollups": pace_rollups(rng, rng.randrange(1, 100), end),
                    "skips": sorted(rng.sample(range(end - 100, end), rng.randrange(0, 8))),
                    "exclude_day": end,
                },
            )
        )
    return found


def with_a_stub_rollup_store(on_pace_run, predecessor, *, rollups, skips, exclude_day):
    stand_in = types.SimpleNamespace(_store=StubRollupStore(rollups, skips))
    return asyncio.run(on_pace_run(stand_in, exclude_day=as_date(exclude_day)))


# --- the day's bonuses ----------------------------------------------------------------------------


class StubDayStore:
    """Holds the case's day base, buff and review amounts, and records the grants written."""

    def __init__(self, base, buff, reviews, reviews_law):
        self.base = base
        self.buff = buff
        self.amounts = {"reviews": reviews, "reviews_law": reviews_law}
        self.written = {}

    async def day_base_xp(self, day):
        return self.base

    async def get_buff(self, day, kind):
        return "epic+10" if self.buff else None

    async def xp_source_amount(self, day, source):
        return self.amounts.get(source, 0)

    async def upsert_xp_grant(self, day, source, amount, *, track="language"):
        self.written[source] = amount


def bonuses_cases(rng):
    found = []
    for run in (0, 1, 2, 3, 10, 11, 12, 20):
        found.append(
            (
                "run",
                {"base": 1000, "run": run, "buff": False, "reviews": 800, "reviews_law": 0},
            )
        )
    for reviews, reviews_law in ((0, 0), (10, 0), (599, 0), (600, 0), (601, 0), (0, 700), (300, 300)):
        found.append(
            (
                "ascendant",
                {
                    "base": 900,
                    "run": 4,
                    "buff": True,
                    "reviews": reviews,
                    "reviews_law": reviews_law,
                },
            )
        )
    found.append(("zero-base", {"base": 0, "run": 5, "buff": True, "reviews": 0, "reviews_law": 0}))
    for _ in range(10):
        found.append(
            (
                None,
                {
                    "base": rng.randrange(0, 5000),
                    "run": rng.randrange(0, 16),
                    "buff": rng.random() < 0.5,
                    "reviews": rng.randrange(0, 1500),
                    "reviews_law": rng.randrange(0, 1500),
                },
            )
        )
    return found


def with_a_stub_day_store(apply_day_bonuses, predecessor, *, base, run, buff, reviews, reviews_law):
    store = StubDayStore(base, buff, reviews, reviews_law)

    async def on_pace_run(*, exclude_day):
        return run

    stand_in = types.SimpleNamespace(_store=store, _on_pace_run=on_pace_run)
    asyncio.run(apply_day_bonuses(stand_in, as_date(PRESENT_DAY)))
    return dict(sorted(store.written.items()))


# --- the Ascendant arming -------------------------------------------------------------------------


class StubArmingStore:
    """Holds the case's buff, skip day, yesterday's rollup and its `backlog_zero` amount."""

    def __init__(self, buff, skip, rollup_reviews, backlog_zero):
        self.buff = buff
        self.skip = skip
        self.rollup_reviews = rollup_reviews
        self.backlog_zero = backlog_zero
        self.armed = False

    async def get_buff(self, day, kind):
        return "epic+10" if self.buff else None

    async def get_active_skip_day(self, day):
        return {"id": 1} if self.skip else None

    async def get_daily_rollup(self, day):
        if self.rollup_reviews is None:
            return None
        return {"reviews": self.rollup_reviews}

    async def xp_source_amount(self, day, source):
        return self.backlog_zero if source == "backlog_zero" else 0

    async def set_buff(self, day, kind, payload=""):
        self.armed = True


def arms_cases(rng):
    found = []
    for buff in (False, True):
        for skip in (False, True):
            for reviews in (None, 0, 40):
                for backlog_zero in (0, 100):
                    found.append(
                        (
                            "combination",
                            {
                                "buff": buff,
                                "skip": skip,
                                "rollup_reviews": reviews,
                                "backlog_zero": backlog_zero,
                            },
                        )
                    )
    return found


def with_a_stub_arming_store(maybe_grant, predecessor, *, buff, skip, rollup_reviews, backlog_zero):
    store = StubArmingStore(buff, skip, rollup_reviews, backlog_zero)
    stand_in = types.SimpleNamespace(_store=store)
    asyncio.run(maybe_grant(stand_in, as_date(PRESENT_DAY)))
    return store.armed


# --- the day base ---------------------------------------------------------------------------------

SOURCES = (
    "reviews",
    "reviews_law",
    "studied",
    "backlog_zero",
    "streak",
    "score90",
    "graduations",
    "consistency",
    "ascendant",
    "surprise",
    "readgoal:fr",
    "leech:42",
    "focus",
    "focusgoal:week",
    "focus_combo",
    "chest:common",
    "chest",
    "2x:token",
    "read:fr",
    "focusing",
    "xchest",
)


def ledger_cases(rng):
    found = [("empty", {"rows": [], "day": PRESENT_DAY})]
    found.append(
        (
            "every-source",
            {
                "rows": [
                    {"day": PRESENT_DAY, "source": source, "amount": 10 + index, "track": "language"}
                    for index, source in enumerate(SOURCES)
                ],
                "day": PRESENT_DAY,
            },
        )
    )
    found.append(
        (
            "other-day",
            {
                "rows": [
                    {"day": PRESENT_DAY - 1, "source": "reviews", "amount": 99, "track": "language"},
                    {"day": PRESENT_DAY, "source": "studied", "amount": 50, "track": "language"},
                ],
                "day": PRESENT_DAY,
            },
        )
    )
    found.append(
        (
            "both-tracks",
            {
                "rows": [
                    {"day": PRESENT_DAY, "source": "reviews", "amount": 120, "track": "language"},
                    {"day": PRESENT_DAY, "source": "reviews_law", "amount": 300, "track": "law"},
                ],
                "day": PRESENT_DAY,
            },
        )
    )
    for _ in range(8):
        chosen = rng.sample(SOURCES, rng.randrange(1, len(SOURCES)))
        found.append(
            (
                None,
                {
                    "rows": [
                        {
                            "day": PRESENT_DAY,
                            "source": source,
                            "amount": rng.randrange(0, 500),
                            "track": "law" if source == "reviews_law" else "language",
                        }
                        for source in chosen
                    ],
                    "day": PRESENT_DAY,
                },
            )
        )
    return found


def with_a_ledger(day_base_xp, predecessor, *, rows, day):
    store_type = predecessor("database.GamifyStore")

    async def run():
        with tempfile.TemporaryDirectory() as directory:
            store = await store_type(Path(directory) / "gamify.db").connect()
            try:
                for row in rows:
                    await store.upsert_xp_grant(
                        as_date(row["day"]), row["source"], row["amount"], track=row["track"]
                    )
                return await day_base_xp(store, as_date(day))
            finally:
                await store.close()

    return asyncio.run(run())


FUNCTIONS = {
    "review_xp": {
        "kind": "adapter",
        "function": "gamification.xp.review_xp",
        "adapter": with_a_review,
        "note": "Builds a Review from the case's ease, new interval and type, and a card-to-tier "
        "map holding the case's tier for its card, or none.",
        "cases": review_cases,
    },
    "daily_bonus_grants": {
        "kind": "adapter",
        "function": "gamification.xp.daily_bonus_grants",
        "adapter": with_the_five_inputs,
        "note": "Calls it with the case's five inputs and returns each grant as a source and an "
        "amount.",
        "cases": bonus_cases,
    },
    "level_info": {
        "kind": "adapter",
        "function": "gamification.xp.level_info",
        "adapter": with_the_fields,
        "note": "Calls it and returns each field of the LevelInfo it built.",
        "cases": info_cases,
    },
    "level_title": {
        "kind": "function",
        "function": "gamification.xp.level_title",
        "cases": title_cases,
    },
    "parse_tier": {
        "kind": "function",
        "function": "anki_reader._parse_tier",
        "cases": tier_cases,
    },
    "consistency_multiplier": {
        "kind": "function",
        "function": "gamification.adaptive.consistency_multiplier",
        "cases": run_cases,
    },
    "tier_down_run": {
        "kind": "function",
        "function": "gamification.governor.tier_down_run",
        "cases": fold_cases,
    },
    "projected_multiplier_drop": {
        "kind": "function",
        "function": "gamification.governor.projected_multiplier_drop",
        "cases": drop_cases,
    },
    "on_pace_run": {
        "kind": "adapter",
        "function": "pipeline_layers.governor.GovernorLayer._on_pace_run",
        "adapter": with_a_stub_rollup_store,
        "note": "Runs it on a stand-in layer whose stub store holds the case's rollups and skip "
        "days, each day turned from its epoch day number into the store's form.",
        "cases": pace_cases,
    },
    "day_bonuses": {
        "kind": "adapter",
        "function": "pipeline_layers.governor.GovernorLayer._apply_day_bonuses",
        "adapter": with_a_stub_day_store,
        "note": "Runs it on a stand-in layer whose stub store holds the day base, the buff and "
        "the review amounts, and whose _on_pace_run answers the case's run; returns the grants "
        "written.",
        "cases": bonuses_cases,
    },
    "ascendant_arms": {
        "kind": "adapter",
        "function": "pipeline_layers.governor.GovernorLayer._maybe_grant_ascendant",
        "adapter": with_a_stub_arming_store,
        "note": "Runs it on a stand-in layer whose stub store holds a buff, a skip day, the "
        "previous day's rollup reviews and backlog_zero amount; returns whether a buff was set.",
        "cases": arms_cases,
    },
    "day_base_xp": {
        "kind": "adapter",
        "function": "database.GamifyStore.day_base_xp",
        "adapter": with_a_ledger,
        "note": "Fills a temporary store with the case's synthetic ledger rows through its own "
        "upsert_xp_grant, then reads the day base; each day is an epoch day turned into the "
        "store's form inside the adapter.",
        "cases": ledger_cases,
    },
    "progression.constants": {
        "kind": "constants",
        "names": [
            "constants.XP_BASE",
            "constants.EASE_XP_MULT",
            "constants.TYPE_XP_MULT",
            "constants.XP_MATURE_MULT",
            "constants.XP_YOUNG_MULT",
            "constants.XP_NEUTRAL_MULT",
            "constants.MATURE_IVL_DAYS",
            "constants.TIER_XP_MULT",
            "constants.XP_BONUS_STUDIED",
            "constants.XP_BONUS_BACKLOG_ZERO",
            "constants.XP_BONUS_STREAK_PER_DAY",
            "constants.XP_BONUS_STREAK_CAP",
            "constants.XP_BONUS_SCORE_90",
            "constants.XP_BONUS_GRADUATION_EACH",
            "constants.SCORE_90_THRESHOLD",
            "constants.LEVEL_A",
            "constants.LEVEL_B",
            "constants.LEVEL_TITLES",
            "constants.ON_PACE_SCORE",
            "constants.TIER_DOWN_STEP",
            "constants.ASCENDANT_XP_FRAC",
            "constants.ASCENDANT_XP_CAP",
            "gamification.adaptive.CONSISTENCY_STEP",
            "gamification.adaptive.CONSISTENCY_CAP",
        ],
    },
}

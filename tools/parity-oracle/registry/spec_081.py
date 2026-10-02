"""SPEC-081's registrations for the sessions, the effort floor, the rarity roll, the Epic odds, the
payout and the chest constants (the quests core of #102 and #103).

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_reviews` builds the predecessor's `Review` rows from the case's instants, cards, answer
  buttons, types and times, then calls the function it was handed. The case carries the per-answer
  time cap the port is given; the adapter refuses a case whose cap differs from the cap the
  predecessor applies, so the cap a test reads from the golden is the one that was applied.
* `with_an_effort` builds the predecessor's `RealEffort` from the case's counts.
* `with_a_stub_loot` drives `pipeline_layers/loot.py:LootLayer._grant_session_chests` on a
  stand-in layer over a stub store that holds the case's chests of the day, settings, skip day,
  buff and pity counters, with `random.SystemRandom` patched to hand out the case's draws in order.
  A session's review XP is the caller's input in the port (quests computes no XP), so
  `xp.reviews_xp` is patched to answer the case's XP for each session's start. The chest lock is
  held inactive and quiet hours answer the case's flag. It returns the chests the step inserted,
  the pity counters after it and the draws it took.
* `with_a_stub_challenge` drives `LootLayer._pay_quest` for the challenge slot, and
  `with_a_stub_week` drives `LootLayer._update_weekly_quest` with the week's quest complete and
  its reward unclaimed, each over the same stub store and draws. The week's freeze is held at its
  month's cap, so the streaks' freeze stays out of the chest's golden.

The draws a case aims at are multiples of 2^-53, as the predecessor's `random.SystemRandom().random()`
returns them. The case builders draw only from the `random.Random` the generator seeds. Every number
is synthetic, every instant epoch milliseconds, every study day an epoch day number.
"""

import asyncio
import datetime as dt
import types
from unittest import mock

#: The resolution of the predecessor's draw: a 53-bit fraction.
STEP = 2.0**-53
#: The per-answer time cap, in seconds, the sessions' cases carry. The adapter proves it equals the
#: predecessor's own.
ANSWER_CAP_SECONDS = 60.0
#: An instant near the present, in epoch milliseconds.
BASE_MS = 1_700_000_000_000
MINUTE_MS = 60_000
#: The base odds and the ramp the roll cases aim at. The goldens come from the predecessor's
#: functions, never from these.
LEGENDARY_PCT = 1.0
RARE_PCT = 22.0


# --- sessions -----------------------------------------------------------------------------------


def a_review(at, card, ease=3, kind=1, taken=5_000):
    """One synthetic review row as a case's JSON."""
    return {"id": at, "card_id": card, "ease": ease, "kind": kind, "taken_ms": taken}


def a_run(start, count, first_card, spacing=5_000, taken=5_000):
    """`count` reviews of distinct cards `spacing` ms apart from `start`."""
    return [a_review(start + i * spacing, first_card + i, taken=taken) for i in range(count)]


def session_input(reviews):
    return {"answer_time_cap_seconds": ANSWER_CAP_SECONDS, "reviews": reviews}


def session_cases(rng):
    found = [("empty", session_input([])), ("single", session_input([a_review(BASE_MS, 1)]))]
    # A gap of 10 minutes splits; one millisecond less does not.
    for gap in (0, 1, 599_999, 600_000, 600_001, 1_200_000):
        found.append(
            (
                "gap",
                session_input(
                    [
                        a_review(BASE_MS, 1),
                        a_review(BASE_MS + gap, 2),
                        a_review(BASE_MS + gap + 9, 3),
                    ]
                ),
            )
        )
    # The effort floor: distinct cards around fifteen, and repeats of one card that do not count.
    for count in (13, 14, 15, 16, 17):
        found.append(("floor", session_input(a_run(BASE_MS, count, 100))))
    found.append(
        (
            "floor",
            session_input(a_run(BASE_MS, 14, 100) + [a_review(BASE_MS + 90_000, 100)] * 3),
        )
    )
    found.append(
        ("floor", session_input([a_review(BASE_MS + 1_000 * i, 7) for i in range(40)])),
    )
    # Bookkeeping rows and unanswered rows are not study events and never bridge a gap.
    found.append(
        (
            "bookkeeping",
            session_input(
                [
                    a_review(BASE_MS, 1),
                    a_review(BASE_MS + 300_000, 2, kind=4),
                    a_review(BASE_MS + 400_000, 3, kind=5),
                    a_review(BASE_MS + 500_000, 4, ease=0),
                    a_review(BASE_MS + 700_000, 5),
                ]
            ),
        )
    )
    for kind in (0, 1, 2, 3, 4, 5):
        for ease in (0, 1, 4):
            found.append(
                ("bookkeeping", session_input([a_review(BASE_MS, 9, ease=ease, kind=kind)]))
            )
    # Each answer's time is capped; sixty seconds exactly and a millisecond either side.
    for taken in (0, 1, 59_999, 60_000, 60_001, 120_000, 3_600_000):
        found.append(
            (
                "cap",
                session_input(
                    [a_review(BASE_MS, 1, taken=taken), a_review(BASE_MS + 1, 2, taken=999)]
                ),
            )
        )
    # The input's order is the order the predecessor sums in, and its ids may repeat.
    shuffled = a_run(BASE_MS, 20, 300, taken=7_001)
    rng.shuffle(shuffled)
    found.append(("unsorted", session_input(shuffled)))
    found.append(("duplicate", session_input([a_review(BASE_MS, 1), a_review(BASE_MS, 1)] * 3)))
    for _ in range(12):
        reviews, at = [], BASE_MS
        for _ in range(rng.randrange(1, 4)):
            reviews += a_run(
                at,
                rng.randrange(1, 31),
                rng.randrange(1, 50),
                spacing=rng.randrange(1, 40_000),
                taken=rng.randrange(0, 90_000),
            )
            at = reviews[-1]["id"] + rng.choice((599_999, 600_000, 600_001, 3_000_000))
        found.append((None, session_input(reviews)))
    return found


def with_reviews(function, predecessor, *, answer_time_cap_seconds, reviews):
    """Build the case's `Review` rows, then call `function` and return the predecessor's answer as
    the golden's JSON. A sessions function returns each session's bounds and effort; the eligible
    sessions' function returns the bounds alone."""
    applied = predecessor("constants.ANSWER_TIME_CAP_SECONDS")
    if applied != answer_time_cap_seconds:
        raise ValueError(f"the case's cap {answer_time_cap_seconds} is not the applied {applied}")
    review_type = predecessor("types.Review")
    rows = [
        review_type(
            id_ms=r["id"],
            cid=r["card_id"],
            ease=r["ease"],
            ivl=0,
            last_ivl=0,
            factor=2500,
            time_ms=r["taken_ms"],
            rtype=r["kind"],
        )
        for r in reviews
    ]
    return function(rows, predecessor)


def sessions_adapter(function, predecessor, **case):
    def call(rows, _predecessor):
        return [
            {
                "start_ms": s.start_ms,
                "end_ms": s.end_ms,
                "reviews": s.measured.reviews,
                "distinct_cards": s.measured.distinct_cards,
                "minutes": s.measured.minutes,
            }
            for s in function(rows)
        ]

    return with_reviews(call, predecessor, **case)


def eligible_adapter(function, predecessor, **case):
    sessions = predecessor("gamification.chests.sessions_from_reviews")

    def call(rows, _predecessor):
        return [{"start_ms": s.start_ms, "end_ms": s.end_ms} for s in function(sessions(rows))]

    return with_reviews(call, predecessor, **case)


# --- the effort floor ---------------------------------------------------------------------------


def floor_cases(rng):
    found = []
    for distinct in (0, 1, 13, 14, 15, 16, 17, 30):
        found.append(
            (
                "floor",
                {"reviews": distinct, "distinct_cards": distinct, "minutes": distinct * 0.1},
            )
        )
    # Many reviews and many minutes of few cards never earn a chest.
    found.append(("floor", {"reviews": 500, "distinct_cards": 14, "minutes": 240.0}))
    found.append(("floor", {"reviews": 15, "distinct_cards": 15, "minutes": 0.0}))
    for _ in range(12):
        found.append(
            (
                None,
                {
                    "reviews": rng.randrange(0, 80),
                    "distinct_cards": rng.randrange(0, 40),
                    "minutes": rng.randrange(0, 600) / 10.0,
                },
            )
        )
    return found


def with_an_effort(function, predecessor, *, reviews, distinct_cards, minutes):
    """Build the predecessor's `RealEffort` from the case's counts and ask its floor."""
    effort = predecessor("effort.RealEffort")(
        reviews=reviews, distinct_cards=distinct_cards, minutes=minutes
    )
    return function(effort)


# --- the roll -----------------------------------------------------------------------------------


def draws_around(percent):
    """The draws of 53-bit resolution that straddle `percent`: the first whose percent is at or
    above it, and one step of 2^-53 below it and above it."""
    n = round(percent / 100.0 / STEP)
    while (n * STEP) * 100.0 >= percent:
        n -= 1
    while (n * STEP) * 100.0 < percent:
        n += 1
    return [(n - 1) * STEP, n * STEP, (n + 1) * STEP]


def odds_for(since_epic, buff_pts):
    """The Epic points the case aims its boundaries at, as the SPEC states the rule. Only a place to
    aim the draws: the golden's answers come from the predecessor."""
    ramp = max(0, since_epic - 8) * 5.0
    return min(40.0, 7.0 + ramp + max(0.0, buff_pts))


def roll_cases(rng):
    found = []
    shapes = [(0, 0, 0.0), (3, 5, 0.0), (8, 8, 0.0), (9, 10, 0.0), (10, 20, 0.0), (11, 30, 0.0)]
    shapes += [(12, 38, 0.0), (0, 0, 10.0), (5, 7, 10.0), (12, 3, 10.0), (4, 4, 20.0)]
    shapes += [(0, 0, 33.0), (8, 1, 5.5), (12, 2, 12.0)]
    for since_epic, since_legendary, buff in shapes:
        epic = odds_for(since_epic, buff)
        edge = "ceiling" if epic >= 40.0 else "threshold"
        for boundary in (LEGENDARY_PCT, LEGENDARY_PCT + epic, LEGENDARY_PCT + epic + RARE_PCT):
            for draw in draws_around(boundary):
                found.append(
                    (
                        edge,
                        {
                            "u": draw,
                            "since_epic": since_epic,
                            "since_legendary": since_legendary,
                            "buff_pts": buff,
                        },
                    )
                )
        for draw in (0.0, STEP, 0.5, 1.0 - STEP):
            found.append(
                (
                    edge,
                    {
                        "u": draw,
                        "since_epic": since_epic,
                        "since_legendary": since_legendary,
                        "buff_pts": buff,
                    },
                )
            )
    # The two guarantees, on both sides of each counter, whatever the draw.
    for since_epic in (11, 12, 13, 14, 15, 30):
        for since_legendary in (0, 20, 37, 38, 39, 40, 41, 80):
            for draw in (0.0, 0.5, 1.0 - STEP):
                found.append(
                    (
                        "guarantee",
                        {
                            "u": draw,
                            "since_epic": since_epic,
                            "since_legendary": since_legendary,
                            "buff_pts": 0.0,
                        },
                    )
                )
    # Draws outside [0, 1) are clamped before they are read.
    for draw in (-0.25, 1.0, 1.5):
        found.append(
            (
                "threshold",
                {"u": draw, "since_epic": 0, "since_legendary": 0, "buff_pts": 0.0},
            )
        )
    for _ in range(16):
        found.append(
            (
                None,
                {
                    "u": rng.randrange(0, 2**53) * STEP,
                    "since_epic": rng.randrange(0, 15),
                    "since_legendary": rng.randrange(0, 42),
                    "buff_pts": rng.choice((0.0, 0.0, 10.0, 20.0)),
                },
            )
        )
    return found


def odds_cases(rng):
    found = []
    for since_epic in range(0, 17):
        found.append(("ramp", {"since_epic": since_epic, "buff_pts": 0.0}))
    for since_epic in (-3, -1, 7, 8, 9, 100):
        found.append(("ramp", {"since_epic": since_epic, "buff_pts": 0.0}))
    for since_epic in (0, 8, 9, 10, 12, 13, 14, 20):
        for buff in (20.0, 25.0, 33.0, 33.5, 34.0, 40.0, 1000.0):
            found.append(("ceiling", {"since_epic": since_epic, "buff_pts": buff}))
    for since_epic in (0, 8, 10, 12):
        for buff in (-5.0, -0.5, 0.0, 0.25, 5.0, 5.5, 10.0, 10.5, 12.5, 26.5):
            found.append(("buff", {"since_epic": since_epic, "buff_pts": buff}))
    for _ in range(12):
        found.append(
            (
                None,
                {
                    "since_epic": rng.randrange(0, 16),
                    "buff_pts": rng.randrange(-20, 500) / 10.0,
                },
            )
        )
    return found


# --- the payout ---------------------------------------------------------------------------------


def payout_cases(rng):
    found = []
    # The band's draw index changes at multiples of 1/16 (common) and 1/31 (rare).
    ladders = {
        "common": [i / 16.0 for i in range(0, 17)],
        "rare": [i / 31.0 for i in range(0, 32)],
    }
    xps = (0, 1, 83, 84, 85, 99, 100, 101, 116, 117, 150, 199, 200, 201, 333, 334, 10**6, -40)
    for rarity, ladder in ladders.items():
        draws = [0.0, STEP, 0.5, 1.0 - STEP, 1.0, -0.5, 1.5]
        for point in ladder:
            draws += [point, max(0.0, point - STEP), min(1.0 - STEP, point + STEP)]
        for xp in xps:
            for draw in draws[:: (1 if xp in (0, 100, 200, 10**6) else 6)]:
                found.append(("cap", {"rarity": rarity, "u": draw, "session_xp": xp}))
    for rarity in ("legendary", "epic"):
        for xp in (0, 100, 10**6):
            for draw in (0.0, 0.5, 1.0 - STEP):
                found.append(("cap", {"rarity": rarity, "u": draw, "session_xp": xp}))
    for _ in range(16):
        found.append(
            (
                None,
                {
                    "rarity": rng.choice(("common", "rare", "legendary", "epic")),
                    "u": rng.randrange(0, 2**53) * STEP,
                    "session_xp": rng.randrange(0, 1500),
                },
            )
        )
    return found


def with_session_xp(function, predecessor, *, rarity, u, session_xp):
    """The payout under the predecessor's own argument name for the session's review XP."""
    return function(rarity, u, session_xp)


# --- the grant step, the challenge chest and the weekly chest -----------------------------------

#: A study day near the present, a Wednesday, as an epoch day number.
GRANT_DAY = 19_676
DAY_MS = 86_400_000
HOUR_MS = 3_600_000
#: The study day's first instant under the predecessor's default rollover (04:00 at offset 0).
OPEN_MS = GRANT_DAY * DAY_MS + 4 * HOUR_MS
#: The gap that splits a session, in milliseconds: where the drift cases aim.
GAP_MS = 600_000
#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: The session starts the predecessor stores for its challenge and weekly chests. The port names
#: them as origins (SPEC-081 section 8), so a case carries the origin and the adapter the sentinel.
SENTINEL_STARTS = {"challenge": -3, "weekly": -7}


def as_date(day):
    """The `date` of an epoch day number."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def at(fraction):
    """The draw of 53-bit resolution nearest `fraction`."""
    return round(fraction / STEP) * STEP


def an_existing(origin, start=0):
    """A chest already on the case's study day: its origin and, for a session's, its start."""
    return {"origin": origin, "session_start_ms": start}


def stored_start(chest):
    """The session start the predecessor stores for a chest of the case: a session's own start, or
    its origin's sentinel."""
    if chest["origin"] == "session":
        return chest["session_start_ms"]
    return SENTINEL_STARTS[chest["origin"]]


class Draws:
    """The predecessor's `random.SystemRandom()` as the case's draws, handed out in order."""

    def __init__(self, values):
        self.values = list(values)
        self.used = 0

    def random(self):
        if self.used >= len(self.values):
            raise ValueError(f"the case's {len(self.values)} draw(s) ran out")
        value = self.values[self.used]
        self.used += 1
        return value


class StubChestStore:
    """Holds the case's chests of the day, its settings, skip day, buff and pity counters, and
    records the chests the step inserts and the counters it sets. A weekly case's week is complete,
    its reward unclaimed, and its month's freeze drops at the cap."""

    def __init__(self, case, *, freeze_cap=0):
        self.day = as_date(case["study_day"]).isoformat()
        self.rows = [
            {"day": self.day, "session_start_ms": stored_start(chest)} for chest in case["existing"]
        ]
        self.held = len(self.rows)
        self.since_epic = case["since_epic"]
        self.since_legendary = case["since_legendary"]
        self.settings = {
            "chests_per_day": case.get("per_day_max"),
            "chest_vault_hour": case.get("vault_hour"),
        }
        self.skip_day = case.get("skip_day", False)
        self.ascendant = case.get("ascendant", False)
        self.freeze_cap = freeze_cap

    async def get_active_skip_day(self, day):
        return {"day": day.isoformat()} if self.skip_day else None

    async def chests_for_day(self, day):
        return [dict(row) for row in self.rows if row["day"] == day.isoformat()]

    async def get_int_setting(self, key, default):
        if self.settings.get(key) is None:
            raise ValueError(f"the case sets no {key}")
        return self.settings[key]

    async def chest_exists(self, day, session_start_ms):
        return any(
            row["day"] == day.isoformat() and row["session_start_ms"] == session_start_ms
            for row in self.rows
        )

    async def get_pity(self):
        return (self.since_epic, self.since_legendary)

    async def set_pity(self, since_epic, since_legendary):
        self.since_epic, self.since_legendary = since_epic, since_legendary

    async def get_buff(self, day, kind):
        if kind != "ascendant":
            raise ValueError(f"the case holds no {kind} buff")
        return "1" if self.ascendant else None

    async def insert_chest(self, *, day, session_start_ms, rarity, payout_xp, state):
        if await self.chest_exists(day, session_start_ms):
            return None
        self.rows.append(
            {
                "day": day.isoformat(),
                "session_start_ms": session_start_ms,
                "rarity": rarity,
                "payout_xp": payout_xp,
                "state": state,
            }
        )
        return len(self.rows)

    async def insert_coin_delta_once(self, day, source, reference, delta):
        return True

    async def crown_days_between(self, start, end):
        return 7

    async def get_recent_rollups(self, limit):
        return [{"day": self.day, "reviews": 10_000}]

    async def focus_rows_since(self, since):
        return [(self.day, 0, 10_000, 0, 0)]

    async def upsert_weekly_quest(self, **_quest):
        return None

    async def claim_weekly_reward(self, week_start):
        return True

    async def freeze_drops_in_month(self, month, reasons):
        return self.freeze_cap

    async def get_streak_state(self):
        return types.SimpleNamespace(freezes=0)

    def answer(self, draws):
        """The chests the step inserted, the counters after it and the draws it took."""
        origins = {start: origin for origin, start in SENTINEL_STARTS.items()}
        return {
            "chests": [
                {
                    "origin": origins.get(row["session_start_ms"], "session"),
                    "session_start_ms": max(row["session_start_ms"], 0),
                    "rarity": row["rarity"],
                    "payout_xp": row["payout_xp"],
                    "state": row["state"],
                }
                for row in self.rows[self.held :]
            ],
            "since_epic": self.since_epic,
            "since_legendary": self.since_legendary,
            "draws_used": draws.used,
        }


def grant_case(
    rng,
    runs,
    *,
    existing=(),
    per_day_max=3,
    vault_hour=21,
    local_hour=12,
    quiet=False,
    skip_day=False,
    ascendant=False,
    pity=(0, 0),
    xps=None,
    draws=None,
):
    """One grant step's input. `runs` are (offset from the day's first instant, distinct cards),
    each a run of reviews five seconds apart; each run's start carries its review XP."""
    reviews, session_xp = [], []
    for index, (offset, cards) in enumerate(runs):
        start = OPEN_MS + offset
        reviews += a_run(start, cards, 1_000 * (index + 1))
        session_xp.append([start, xps[index] if xps is not None else rng.randrange(0, 400)])
    if draws is None:
        draws = [rng.randrange(0, 2**53) * STEP for _ in range(2 * len(runs))]
    return {
        "study_day": GRANT_DAY,
        "local_hour": local_hour,
        "quiet": quiet,
        "skip_day": skip_day,
        "ascendant": ascendant,
        "per_day_max": per_day_max,
        "vault_hour": vault_hour,
        "since_epic": pity[0],
        "since_legendary": pity[1],
        "existing": list(existing),
        "answer_time_cap_seconds": ANSWER_CAP_SECONDS,
        "reviews": reviews,
        "session_xp": session_xp,
        "draws": draws,
    }


def grant_cases(rng):
    found = []
    hours = [h * HOUR_MS for h in (1, 3, 5, 7, 9)]
    four = [(hours[i], 20) for i in range(4)]
    late = OPEN_MS + 15 * HOUR_MS
    # The day's cap counts every chest already on the day, of every origin.
    for per_day_max in (0, 1, 2, 3, 5):
        found.append(("cap", grant_case(rng, four, per_day_max=per_day_max)))
    for existing in (
        [an_existing("challenge")],
        [an_existing("challenge"), an_existing("weekly")],
        [an_existing("session", late), an_existing("weekly")],
        [an_existing("session", late + i * HOUR_MS) for i in range(3)],
        [an_existing("session", late + i * HOUR_MS) for i in range(4)],
    ):
        found.append(("cap", grant_case(rng, four, existing=existing)))
    # A session below the effort floor takes no chest and no place under the cap.
    floor = [(hours[0], 14), (hours[1], 15), (hours[2], 3), (hours[3], 30)]
    found.append(("cap", grant_case(rng, floor, per_day_max=2)))
    # A declared skip day grants nothing.
    for skip_day in (True, False):
        found.append(("skip", grant_case(rng, four[:2], skip_day=skip_day)))
    found.append(("skip", grant_case(rng, [], skip_day=True)))
    found.append(("skip", grant_case(rng, [], skip_day=False)))
    # The drift guard: a session whose start lies within one gap of a session chest's start is
    # skipped, and a chest with the session's own start is the session's, never rolled again.
    one = [(hours[2], 20)]
    for delta in (-GAP_MS - 1, -GAP_MS, -GAP_MS + 1, -1, 0, 1, GAP_MS - 1, GAP_MS, GAP_MS + 1):
        existing = [an_existing("session", OPEN_MS + hours[2] + delta)]
        found.append(("drift", grant_case(rng, one, existing=existing)))
    for origin in ("challenge", "weekly"):
        found.append(("drift", grant_case(rng, one, existing=[an_existing(origin)])))
    found.append(
        (
            "drift",
            grant_case(rng, four[:3], existing=[an_existing("session", OPEN_MS + hours[1] + 1)]),
        )
    )
    # The vault: at or after the vault hour, or inside quiet hours, a chest is vaulted.
    for vault_hour in (0, 21, 23):
        for local_hour in sorted({max(0, vault_hour - 1), vault_hour, min(23, vault_hour + 1)}):
            for quiet in (False, True):
                found.append(
                    (
                        "vault",
                        grant_case(
                            rng,
                            four[:2],
                            vault_hour=vault_hour,
                            local_hour=local_hour,
                            quiet=quiet,
                        ),
                    )
                )
    # The pity counters move after each chest, so a guarantee lands on a later session of the step;
    # the payouts read each session's own review XP.
    for pity, rolls in (
        ((12, 0), [0.99, 0.99, 0.99]),
        ((13, 0), [0.99, 0.0, 0.99]),
        ((5, 38), [0.99, 0.99, 0.99]),
        ((12, 39), [0.0, 0.99, 0.015]),
        ((0, 0), [0.0, 0.015, 0.2]),
    ):
        draws = [value for roll in rolls for value in (at(roll), at(0.99))]
        found.append(
            ("pity", grant_case(rng, four[:3], pity=pity, draws=draws, xps=[0, 100, 1000]))
        )
    # The Ascendant buff adds its Epic points to every roll of its day.
    for ascendant in (False, True):
        draws = [at(0.1), at(0.5), at(0.165), at(0.5), at(0.175), at(0.5)]
        found.append(("pity", grant_case(rng, four[:3], ascendant=ascendant, draws=draws)))
    for _ in range(12):
        count = rng.randrange(0, 5)
        runs = [
            (HOUR_MS * (1 + 3 * i) + rng.randrange(0, HOUR_MS), rng.randrange(10, 26))
            for i in range(count)
        ]
        existing = []
        for origin in rng.sample(("session", "challenge", "weekly"), rng.randrange(0, 3)):
            start = OPEN_MS + rng.randrange(0, 20 * HOUR_MS) if origin == "session" else 0
            existing.append(an_existing(origin, start))
        found.append(
            (
                None,
                grant_case(
                    rng,
                    runs,
                    existing=existing,
                    per_day_max=rng.randrange(1, 5),
                    vault_hour=rng.randrange(0, 24),
                    local_hour=rng.randrange(0, 24),
                    quiet=rng.random() < 0.25,
                    skip_day=rng.random() < 0.15,
                    ascendant=rng.random() < 0.3,
                    pity=(rng.randrange(0, 15), rng.randrange(0, 42)),
                ),
            )
        )
    return found


def with_a_stub_loot(
    grant,
    predecessor,
    *,
    study_day,
    local_hour,
    quiet,
    skip_day,
    ascendant,
    per_day_max,
    vault_hour,
    since_epic,
    since_legendary,
    existing,
    answer_time_cap_seconds,
    reviews,
    session_xp,
    draws,
):
    """Run the grant step once on a stand-in layer over the stub store, at `local_hour` of the
    study day at offset 0, with the case's draws and each session's review XP."""
    loot = predecessor("pipeline_layers.loot")
    xp_module = predecessor("gamification.xp")
    store = StubChestStore(
        {
            "study_day": study_day,
            "existing": existing,
            "since_epic": since_epic,
            "since_legendary": since_legendary,
            "per_day_max": per_day_max,
            "vault_hour": vault_hour,
            "skip_day": skip_day,
            "ascendant": ascendant,
        }
    )
    stream = Draws(draws)
    by_start = {start: xp for start, xp in session_xp}

    def session_review_xp(rows, cid_tier=None):
        start = min(row.id_ms for row in rows)
        if start not in by_start:
            raise ValueError(f"the case gives no review XP for the session at {start}")
        return by_start[start]

    async def quiet_now():
        return quiet

    async def lock_inactive(*_args):
        return False

    stand_in = types.SimpleNamespace(
        _store=store,
        _notifier=None,
        _in_quiet_hours_now=quiet_now,
        _chest_lock_active=lock_inactive,
    )
    now_ms = study_day * DAY_MS + local_hour * HOUR_MS + 30 * MINUTE_MS
    config = predecessor("types.CollectionConfig")()

    def call(rows, _predecessor):
        with (
            mock.patch.object(loot.random, "SystemRandom", lambda: stream),
            mock.patch.object(xp_module, "reviews_xp", session_review_xp),
        ):
            asyncio.run(grant(stand_in, rows, config, as_date(study_day), now_ms))
        return store.answer(stream)

    return with_reviews(
        call, predecessor, answer_time_cap_seconds=answer_time_cap_seconds, reviews=reviews
    )


def chest_case(rng, *, pity=(0, 0), existing=(), draws=None):
    """A challenge or weekly chest's input: the day, the counters, the day's chests and draws."""
    if draws is None:
        draws = [rng.randrange(0, 2**53) * STEP for _ in range(2)]
    return {
        "study_day": GRANT_DAY,
        "since_epic": pity[0],
        "since_legendary": pity[1],
        "existing": list(existing),
        "draws": draws,
    }


def challenge_cases(rng):
    found = []
    # The challenge's 10 Epic points move the Epic and Rare boundaries; draws straddle each.
    for pity in ((0, 0), (9, 3), (12, 0)):
        epic = odds_for(pity[0], 10.0)
        for boundary in (LEGENDARY_PCT, LEGENDARY_PCT + epic, LEGENDARY_PCT + epic + RARE_PCT):
            for draw in draws_around(boundary):
                found.append(("challenge", chest_case(rng, pity=pity, draws=[draw, at(0.5)])))
    # Both guarantees, and the counters after each rarity.
    for pity in ((13, 0), (13, 39), (5, 39), (20, 50), (0, 0)):
        for roll in (at(0.99), 0.0):
            found.append(("challenge", chest_case(rng, pity=pity, draws=[roll, at(0.5)])))
    # The payout's cap reads a session base of 200 XP: each band's bottom and top.
    for draws in ([at(0.3), 1.0 - STEP], [at(0.3), 0.0], [at(0.5), 1.0 - STEP], [at(0.5), 0.0]):
        found.append(("challenge", chest_case(rng, draws=draws)))
    # One challenge chest a study day; the day's other chests neither block it nor cap it.
    found.append(("challenge", chest_case(rng, existing=[an_existing("challenge")])))
    sessions = [an_existing("session", OPEN_MS + i * HOUR_MS) for i in range(1, 4)]
    found.append(("challenge", chest_case(rng, existing=[an_existing("weekly"), *sessions])))
    for _ in range(8):
        pity = (rng.randrange(0, 15), rng.randrange(0, 42))
        found.append((None, chest_case(rng, pity=pity)))
    return found


def with_a_stub_challenge(
    pay_quest, predecessor, *, study_day, since_epic, since_legendary, existing, draws
):
    """Pay the challenge slot once on a stand-in layer over the stub store with the case's draws.
    The draws a refused second chest takes are not returned: the port checks the key first."""
    loot = predecessor("pipeline_layers.loot")
    store = StubChestStore(
        {
            "study_day": study_day,
            "existing": existing,
            "since_epic": since_epic,
            "since_legendary": since_legendary,
        }
    )
    stream = Draws(draws)
    stand_in = types.SimpleNamespace(_store=store, _notifier=None)
    with mock.patch.object(loot.random, "SystemRandom", lambda: stream):
        asyncio.run(pay_quest(stand_in, as_date(study_day), "q3", "a synthetic challenge"))
    answer = store.answer(stream)
    del answer["draws_used"]
    return answer


def weekly_cases(rng):
    found = []
    for pity in ((0, 0), (13, 39), (7, 12), (12, 38)):
        found.append(("weekly", chest_case(rng, pity=pity)))
    found.append(("weekly", chest_case(rng, existing=[an_existing("weekly")])))
    sessions = [an_existing("session", OPEN_MS + i * HOUR_MS) for i in range(1, 4)]
    found.append(("weekly", chest_case(rng, existing=[an_existing("challenge"), *sessions])))
    for _ in range(4):
        pity = (rng.randrange(0, 15), rng.randrange(0, 42))
        found.append((None, chest_case(rng, pity=pity)))
    return found


def with_a_stub_week(
    update_week, predecessor, *, study_day, since_epic, since_legendary, existing, draws
):
    """Fold the week once on a stand-in layer over the stub store, with its quest complete and its
    reward unclaimed, nothing suppressed and no smoke bomb settled."""
    loot = predecessor("pipeline_layers.loot")
    store = StubChestStore(
        {
            "study_day": study_day,
            "existing": existing,
            "since_epic": since_epic,
            "since_legendary": since_legendary,
        },
        freeze_cap=predecessor("constants.FREEZE_DROP_MONTHLY_CAP"),
    )
    stream = Draws(draws)

    async def nothing_suppressed(_today, _week_start):
        return frozenset()

    async def no_smoke_bomb(*_args):
        return None

    stand_in = types.SimpleNamespace(
        _store=store,
        _notifier=None,
        _weekly_evidence_suppressed=nothing_suppressed,
        _maybe_earn_smoke_bomb=no_smoke_bomb,
    )
    with mock.patch.object(loot.random, "SystemRandom", lambda: stream):
        asyncio.run(update_week(stand_in, as_date(study_day)))
    return store.answer(stream)


FUNCTIONS = {
    "sessions_from_reviews": {
        "kind": "adapter",
        "function": "gamification.chests.sessions_from_reviews",
        "adapter": sessions_adapter,
        "note": "Builds the predecessor's Review rows from the case's instants, cards, answer "
        "buttons, types and times and refuses a case whose per-answer cap is not the applied one; "
        "returns each session's bounds, study reviews, distinct cards and minutes.",
        "cases": session_cases,
    },
    "eligible_sessions": {
        "kind": "adapter",
        "function": "gamification.chests.eligible_sessions",
        "adapter": eligible_adapter,
        "note": "Builds the same Review rows, clusters them with the predecessor's own "
        "sessions_from_reviews and returns the bounds of the sessions that earn a chest.",
        "cases": session_cases,
    },
    "meets_chest_floor": {
        "kind": "adapter",
        "function": "effort.meets_chest_floor",
        "adapter": with_an_effort,
        "note": "Builds the predecessor's RealEffort from the case's counts.",
        "cases": floor_cases,
    },
    "roll_rarity": {
        "kind": "function",
        "function": "gamification.chests.roll_rarity",
        "cases": roll_cases,
    },
    "epic_odds_pts": {
        "kind": "function",
        "function": "gamification.chests.epic_odds_pts",
        "cases": odds_cases,
    },
    "payout_xp": {
        "kind": "adapter",
        "function": "gamification.chests.payout_xp",
        "adapter": with_session_xp,
        "note": "Passes the case's session review XP as the predecessor's third argument.",
        "cases": payout_cases,
    },
    "chests.constants": {
        "kind": "constants",
        "names": [
            "gamification.chests.BASE_ODDS",
            "gamification.chests.EPIC_ODDS_CEILING_PCT",
            "gamification.chests.PITY_EPIC_RAMP_AFTER",
            "gamification.chests.PITY_EPIC_RAMP_PTS",
            "gamification.chests.PITY_EPIC_GUARANTEE",
            "gamification.chests.PITY_LEGENDARY_GUARANTEE",
            "gamification.chests.SESSION_GAP_MS",
            "gamification.chests.COMMON_XP",
            "gamification.chests.RARE_XP",
            "gamification.chests.LEGENDARY_XP",
            "gamification.chests.EPIC_FALLBACK_XP",
            "gamification.chests.PAYOUT_SESSION_FRAC",
            "constants.REAL_EFFORT_CHEST_MIN_DISTINCT",
            "constants.STREAK_FREEZE_CAP",
            "constants.FREEZE_DROP_MONTHLY_CAP",
        ],
    },
    "session_chests_granted": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer._grant_session_chests",
        "adapter": with_a_stub_loot,
        "note": "Drives the grant step once on a stand-in layer over a stub store holding the "
        "case's chests of the day, settings, skip day, Ascendant buff and pity counters, at the "
        "case's local hour (offset 0) and quiet-hours flag, with the chest lock inactive, "
        "random.SystemRandom patched to the case's draws and xp.reviews_xp patched to the case's "
        "review XP for each session's start; returns the chests inserted, the counters after the "
        "step and the draws it took.",
        "cases": grant_cases,
    },
    "challenge_chest": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer._pay_quest",
        "adapter": with_a_stub_challenge,
        "note": "Pays the challenge slot once on a stand-in layer over the same stub store with "
        "random.SystemRandom patched to the case's draws; returns the chest inserted and the "
        "counters after it.",
        "cases": challenge_cases,
    },
    "weekly_chest": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer._update_weekly_quest",
        "adapter": with_a_stub_week,
        "note": "Folds the week once on a stand-in layer over the same stub store, the week's quest "
        "complete, its reward unclaimed, nothing suppressed and the month's freeze drops at their "
        "cap, with random.SystemRandom patched to the case's draws; returns the chest inserted, "
        "the counters after it and the draws it took.",
        "cases": weekly_cases,
    },
}

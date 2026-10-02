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
* `with_a_stub_sweep` drives `LootLayer._sweep_stale_chests` on a stand-in layer over a stub
  store holding the case's chests of several study days and states, each with its id in the
  case's order; it returns the chests the sweep resolved and the XP grants it wrote.
* `with_a_stub_wallet_of_tokens` drives `LootLayer.activate_double_xp` over a stub store holding
  the case's tokens, with the clock patched to the case's instant; it returns the answer, the
  window's end and the activations it wrote, every instant in epoch milliseconds.
* `with_a_stub_token_day` drives `LootLayer._recompute_token_xp` over the same token store, the
  case's reviews, its rollover and offset and the patched clock. A review's XP at the base rate is
  the caller's input in the port, so `xp.review_xp` is patched to answer the case's XP for each
  review and the predecessor's own `reviews_xp` sums them. It returns the `2x` grants and the
  tokens it consumed.

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


# --- the sweep ----------------------------------------------------------------------------------

#: The study day the sweep runs on: three days after the grant's.
SWEEP_DAY = GRANT_DAY + 3
#: The chest states the predecessor stores, in their order of life.
STATES = ("sealed", "vaulted", "opened", "resolved")
#: A payout as rolled for each rarity the fixed cases hold: a band's value, the fixed Legendary
#: and the Epic's 0. The random cases draw a band's value instead.
PAYOUTS = {"common": 17, "rare": 44, "legendary": 150, "epic": 0}


def an_epoch_day(day):
    """The epoch day number of a `date`."""
    return day.toordinal() - EPOCH_ORDINAL


def a_chest(offset, state, rarity="common", choice=""):
    """A stored chest `offset` study days from the sweep's, in `state`, paying its rarity's
    payout; an Epic's choice is '' until it is made."""
    return {
        "study_day": SWEEP_DAY + offset,
        "state": state,
        "rarity": rarity,
        "payout_xp": PAYOUTS[rarity],
        "choice": choice,
    }


def sweep_case(chests):
    return {"today": SWEEP_DAY, "chests": list(chests)}


def sweep_cases(rng):
    found = [("state", sweep_case([]))]
    # Each state on each side of the sweep's study day: an earlier day's chest resolves, and the
    # day's own and a later day's wait.
    for state in STATES:
        for offset in (-3, -2, -1, 0, 1):
            pair = [a_chest(offset, state, "common"), a_chest(offset, state, "rare")]
            found.append(("state", sweep_case(pair)))
    # A vaulted chest of the day before is the morning's reveal and waits; two days back it
    # resolves, and a sealed or opened chest of the day before resolves.
    found.append(
        (
            "spare",
            sweep_case(
                [
                    a_chest(-1, "vaulted"),
                    a_chest(-2, "vaulted"),
                    a_chest(-1, "sealed"),
                    a_chest(-1, "opened", "epic"),
                ]
            ),
        )
    )
    found.append(
        ("spare", sweep_case([a_chest(-1, "vaulted", "legendary"), a_chest(-1, "vaulted", "epic")]))
    )
    found.append(
        ("spare", sweep_case([a_chest(-2, "vaulted", "epic"), a_chest(-30, "vaulted", "rare")]))
    )
    # An untapped Epic pays the fallback whatever it was rolled to pay; an Epic whose choice is
    # made is resolved and stays out of the sweep.
    for state in ("sealed", "vaulted", "opened"):
        settled = [a_chest(-2, "resolved", "epic", choice) for choice in ("token", "freeze")]
        found.append(("fallback", sweep_case([a_chest(-2, state, "epic"), *settled])))
    found.append(
        (
            "fallback",
            sweep_case(
                [
                    a_chest(-5, "sealed", "legendary"),
                    a_chest(-5, "opened", "epic"),
                    a_chest(-4, "sealed", "epic"),
                    a_chest(-4, "opened", "legendary"),
                ]
            ),
        )
    )
    for _ in range(10):
        chests = []
        for _ in range(rng.randrange(1, 7)):
            rarity = rng.choice(("common", "rare", "epic", "legendary"))
            state = rng.choice(STATES)
            payout = {
                "common": rng.randrange(10, 26),
                "rare": rng.randrange(30, 61),
                "legendary": 150,
                "epic": 0,
            }[rarity]
            choice = ""
            if rarity == "epic" and state == "resolved":
                choice = rng.choice(("token", "freeze"))
            chests.append(
                {
                    "study_day": SWEEP_DAY + rng.randrange(-4, 2),
                    "state": state,
                    "rarity": rarity,
                    "payout_xp": payout,
                    "choice": choice,
                }
            )
        found.append((None, sweep_case(chests)))
    return found


class StubSweepStore:
    """Holds the case's chests, each with its id in the case's order (as the port's store assigns
    them), and records the states it sets and the XP grants written. The guard on a state change
    is the store's own: a chest moves only from one of the states it is given."""

    def __init__(self, chests):
        self.rows = [
            {
                "id": index + 1,
                "day": as_date(chest["study_day"]).isoformat(),
                "state": chest["state"],
                "rarity": chest["rarity"],
                "payout_xp": chest["payout_xp"],
                "choice": chest["choice"] or None,
            }
            for index, chest in enumerate(chests)
        ]
        self.resolved = []
        self.grants = []

    async def chests_in_state(self, states):
        return [dict(row) for row in self.rows if row["state"] in states]

    async def set_chest_state(self, chest_id, *, state, choice=None, expected=()):
        for row in self.rows:
            if row["id"] == chest_id and row["state"] in expected:
                row["state"] = state
                if choice is not None:
                    row["choice"] = choice
                self.resolved.append(chest_id)
                return True
        return False

    async def upsert_xp_grant(self, day, source, amount, *, track="language"):
        self.grants.append(
            {"study_day": an_epoch_day(day), "source": source, "amount": amount, "track": track}
        )


def with_a_stub_sweep(sweep, predecessor, *, today, chests):
    """Sweep once on a stand-in layer over the stub store, on the case's study day."""
    store = StubSweepStore(chests)
    stand_in = types.SimpleNamespace(_store=store)
    asyncio.run(sweep(stand_in, as_date(today)))
    return {"resolved": store.resolved, "grants": store.grants}


# --- the double-XP token ------------------------------------------------------------------------

#: The token cases' present: an instant of whole milliseconds, as the port stores one.
NOW_MS = BASE_MS + 123
#: A window's length as the cases aim at it; the goldens' windows come from the predecessor.
WINDOW_MS = 2 * HOUR_MS
#: The epoch as an aware instant, so the stub turns milliseconds into the predecessor's ISO text
#: and back without a float.
EPOCH = dt.datetime(1970, 1, 1, tzinfo=dt.UTC)


def iso_of(ms):
    """The predecessor's stored text of the instant `ms`."""
    return (EPOCH + dt.timedelta(milliseconds=ms)).isoformat()


def ms_of(text):
    """The epoch milliseconds of the predecessor's stored text."""
    return (dt.datetime.fromisoformat(text) - EPOCH) // dt.timedelta(milliseconds=1)


def a_token(granted, activated=0, ends=0, consumed=0):
    """A token as the port stores it: 0 and 0 for a window not yet opened."""
    return {
        "granted_at_ms": granted,
        "activated_at_ms": activated,
        "window_ends_at_ms": ends,
        "consumed": consumed,
    }


def a_window(start, length=WINDOW_MS, consumed=0):
    """A token activated at `start` whose window lasts `length`."""
    return a_token(start - HOUR_MS, start, start + length, consumed)


def a_clock(now_ms):
    """The predecessor's `datetime` with `now` fixed at the case's instant."""

    class Clock(dt.datetime):
        @classmethod
        def now(cls, tz=None):
            return EPOCH + dt.timedelta(milliseconds=now_ms)

    return Clock


class StubTokenStore:
    """Holds the case's tokens, each with its id in the case's order, as the predecessor's rows,
    and records the activations, consumptions and XP grants written. Each read keeps the
    predecessor's own query's rule over its stored text."""

    def __init__(self, tokens):
        self.rows = [
            {
                "id": index + 1,
                "kind": "double_xp",
                "granted_at": iso_of(token["granted_at_ms"]),
                "activated_at": iso_of(token["activated_at_ms"])
                if token["activated_at_ms"]
                else None,
                "expires_at": iso_of(token["window_ends_at_ms"])
                if token["activated_at_ms"]
                else None,
                "consumed": token["consumed"],
            }
            for index, token in enumerate(tokens)
        ]
        self.activated = []
        self.consumed = []
        self.grants = []
        self.conn = types.SimpleNamespace(execute=self.execute)

    async def active_xp_token(self, kind, now_iso):
        live = [
            row
            for row in self.rows
            if row["kind"] == kind
            and row["consumed"] == 0
            and row["activated_at"] is not None
            and row["expires_at"] > now_iso
        ]
        return dict(live[-1]) if live else None

    async def unactivated_xp_tokens(self, kind):
        return [
            dict(row)
            for row in self.rows
            if row["kind"] == kind and row["consumed"] == 0 and row["activated_at"] is None
        ]

    async def activate_xp_token(self, token_id, activated_at, expires_at):
        for row in self.rows:
            if row["id"] == token_id and row["consumed"] == 0 and row["activated_at"] is None:
                row["activated_at"], row["expires_at"] = activated_at, expires_at
                self.activated.append(
                    {
                        "id": token_id,
                        "activated_at_ms": ms_of(activated_at),
                        "window_ends_at_ms": ms_of(expires_at),
                    }
                )
                return True
        return False

    async def execute(self, sql, *parameters):
        """The bonus's one raw read: the activated tokens of its kind."""
        if "activated_at IS NOT NULL" not in sql or parameters:
            raise ValueError(f"the stub answers only the activated tokens' read, not {sql}")
        rows = [
            dict(row)
            for row in self.rows
            if row["kind"] == "double_xp" and row["activated_at"] is not None
        ]

        async def fetchall():
            return rows

        return types.SimpleNamespace(fetchall=fetchall)

    async def consume_xp_token(self, token_id):
        for row in self.rows:
            if row["id"] == token_id:
                row["consumed"] = 1
        self.consumed.append(token_id)

    async def upsert_xp_grant(self, day, source, amount, *, track="language"):
        self.grants.append(
            {"study_day": an_epoch_day(day), "source": source, "amount": amount, "track": track}
        )


def activation_case(tokens, now_ms=NOW_MS):
    return {"now_ms": now_ms, "tokens": list(tokens)}


def activation_cases(rng):
    found = []
    # Nothing to activate: no token, only consumed ones, or only a window that has ended.
    found.append(("none", activation_case([])))
    found.append(("none", activation_case([a_token(NOW_MS - HOUR_MS, consumed=1)])))
    found.append(("none", activation_case([a_window(NOW_MS - 3 * HOUR_MS)])))
    found.append(
        ("none", activation_case([a_window(NOW_MS - 3 * HOUR_MS, consumed=1), a_token(1, 0, 0, 1)]))
    )
    # One window at a time: a window still open refuses, one that has ended or is consumed does
    # not. Its end is exclusive, so a window ending at the present is over.
    for ends in (-HOUR_MS, -1, 0, 1, HOUR_MS, WINDOW_MS - 1):
        for consumed in (0, 1):
            window = a_token(NOW_MS - WINDOW_MS, NOW_MS - WINDOW_MS, NOW_MS + ends, consumed)
            found.append(
                ("active", activation_case([window, a_token(NOW_MS - HOUR_MS), a_token(NOW_MS)]))
            )
    found.append(
        ("active", activation_case([a_token(NOW_MS - DAY_MS), a_window(NOW_MS - HOUR_MS)]))
    )
    # The oldest held token is the first stored, whatever its grant instant says.
    found.append(
        (
            "oldest",
            activation_case(
                [a_token(NOW_MS - HOUR_MS), a_token(NOW_MS - DAY_MS), a_token(NOW_MS - MINUTE_MS)]
            ),
        )
    )
    found.append(
        (
            "oldest",
            activation_case(
                [a_token(NOW_MS - DAY_MS, consumed=1), a_token(NOW_MS - HOUR_MS), a_token(NOW_MS)]
            ),
        )
    )
    found.append(
        (
            "oldest",
            activation_case(
                [a_window(NOW_MS - 5 * HOUR_MS), a_token(NOW_MS - 2 * DAY_MS), a_token(NOW_MS)]
            ),
        )
    )
    for _ in range(10):
        tokens = []
        for _ in range(rng.randrange(0, 5)):
            granted = NOW_MS - rng.randrange(0, 3 * DAY_MS)
            if rng.random() < 0.5:
                tokens.append(a_token(granted, consumed=int(rng.random() < 0.2)))
            else:
                start = NOW_MS - rng.randrange(0, 4 * HOUR_MS)
                tokens.append(a_window(start, consumed=int(rng.random() < 0.3)))
        found.append((None, activation_case(tokens)))
    return found


def with_a_stub_wallet_of_tokens(activate, predecessor, *, now_ms, tokens):
    """Activate once on a stand-in layer over the token store, at the case's instant."""
    loot = predecessor("pipeline_layers.loot")
    store = StubTokenStore(tokens)
    stand_in = types.SimpleNamespace(_store=store)
    with mock.patch.object(loot, "datetime", a_clock(now_ms)):
        answer = asyncio.run(activate(stand_in))
    return {
        "ok": answer["ok"],
        "error": answer.get("error"),
        "until_ms": ms_of(answer["until"]) if "until" in answer else None,
        "activated": store.activated,
    }


#: The bonus cases' study day and the instant its window opens, two hours after the day's first
#: instant under the predecessor's default rollover.
BONUS_DAY = GRANT_DAY + 5
BONUS_OPEN_MS = BONUS_DAY * DAY_MS + 4 * HOUR_MS


def an_xp_review(at, card, xp, ease=3, kind=1):
    """One synthetic review with the XP the case gives it at the base rate."""
    return {**a_review(at, card, ease=ease, kind=kind), "xp": xp}


def bonus_case(tokens, reviews, *, now_ms, study_day=BONUS_DAY, rollover_hour=4, offset=0):
    return {
        "study_day": study_day,
        "rollover_hour": rollover_hour,
        "tz_offset_minutes": offset,
        "now_ms": now_ms,
        "tokens": list(tokens),
        "reviews": list(reviews),
    }


def bonus_cases(rng):
    found = []
    start = BONUS_OPEN_MS + 2 * HOUR_MS
    window = a_window(start)
    after = start + 3 * WINDOW_MS
    # The window holds its start and not its end.
    edges = [start - 1, start, start + 1, start + WINDOW_MS - 1, start + WINDOW_MS]
    for index, at_ms in enumerate(edges):
        found.append(
            ("window", bonus_case([window], [an_xp_review(at_ms, index + 1, 7)], now_ms=after))
        )
    found.append(
        (
            "window",
            bonus_case(
                [window],
                [an_xp_review(at_ms, i + 1, 5) for i, at_ms in enumerate(edges)],
                now_ms=after,
            ),
        )
    )
    # The cap: the windowed review XP around 300, and a windowed day that earned nothing.
    for total in (0, 1, 150, 299, 300, 301, 450, 10_000):
        reviews = [
            an_xp_review(start + 1_000, 1, total // 2),
            an_xp_review(start + 2_000, 2, total - total // 2),
        ]
        found.append(("cap", bonus_case([window], reviews, now_ms=after)))
    # Only study answers count: bookkeeping rows and unanswered rows add nothing and alone settle
    # nothing.
    for kind in (0, 1, 2, 3, 4, 5):
        for ease in (0, 1, 4):
            review = an_xp_review(start + 60_000, 1, 40, ease=ease, kind=kind)
            found.append(("study", bonus_case([window], [review], now_ms=after)))
    # The rollover: a window across the day's turn counts on each study day only the reviews that
    # study day holds, under the case's rollover hour and offset.
    for rollover_hour, offset in ((4, 0), (0, 0), (23, 0), (4, 330), (4, -300), (0, 840)):
        turn = (BONUS_DAY + 1) * DAY_MS + rollover_hour * HOUR_MS - offset * MINUTE_MS
        straddle = a_window(turn - HOUR_MS)
        reviews = [
            an_xp_review(turn - HOUR_MS, 1, 11),
            an_xp_review(turn - 1, 2, 13),
            an_xp_review(turn, 3, 17),
            an_xp_review(turn + HOUR_MS - 1, 4, 19),
        ]
        for study_day in (BONUS_DAY, BONUS_DAY + 1, BONUS_DAY + 2):
            found.append(
                (
                    "rollover",
                    bonus_case(
                        [straddle],
                        reviews,
                        now_ms=turn + DAY_MS,
                        study_day=study_day,
                        rollover_hour=rollover_hour,
                        offset=offset,
                    ),
                )
            )
    # A window that has ended is consumed once; one still open, or ending at the present, is
    # not; a consumed token still settles its windowed day, and a held token settles nothing.
    review = [an_xp_review(start + 60_000, 1, 25)]
    for now_ms in (
        start + HOUR_MS,
        start + WINDOW_MS - 1,
        start + WINDOW_MS,
        start + WINDOW_MS + 1,
    ):
        found.append(("consume", bonus_case([window], review, now_ms=now_ms)))
    found.append(("consume", bonus_case([a_window(start, consumed=1)], review, now_ms=after)))
    found.append(("consume", bonus_case([a_token(start - HOUR_MS)], review, now_ms=after)))
    found.append(
        (
            "consume",
            bonus_case(
                [a_window(start - DAY_MS), a_token(start), window, a_window(start + WINDOW_MS)],
                [*review, an_xp_review(start + WINDOW_MS + 5, 2, 30)],
                now_ms=start + WINDOW_MS + HOUR_MS,
            ),
        )
    )
    for _ in range(12):
        tokens = []
        for _ in range(rng.randrange(1, 4)):
            opened = BONUS_OPEN_MS + rng.randrange(-6 * HOUR_MS, 26 * HOUR_MS)
            if rng.random() < 0.2:
                tokens.append(a_token(opened))
            else:
                tokens.append(a_window(opened, consumed=int(rng.random() < 0.2)))
        reviews = []
        at_ms = BONUS_OPEN_MS - 3 * HOUR_MS
        for card in range(rng.randrange(0, 40)):
            at_ms += rng.randrange(1, 90 * MINUTE_MS)
            reviews.append(
                an_xp_review(
                    at_ms,
                    card + 1,
                    rng.randrange(0, 40),
                    ease=rng.choice((0, 1, 2, 3, 4)),
                    kind=rng.choice((0, 1, 1, 2, 3, 4, 5)),
                )
            )
        found.append(
            (
                None,
                bonus_case(
                    tokens,
                    reviews,
                    now_ms=BONUS_OPEN_MS + rng.randrange(0, 30 * HOUR_MS),
                    study_day=BONUS_DAY + rng.randrange(-1, 2),
                    rollover_hour=rng.randrange(0, 24),
                    offset=rng.choice((0, 0, 60, -240, 330)),
                ),
            )
        )
    return found


def with_a_stub_token_day(
    recompute, predecessor, *, study_day, rollover_hour, tz_offset_minutes, now_ms, tokens, reviews
):
    """Settle the token bonus of the case's study day once on a stand-in layer over the token
    store, under the case's rollover and offset, at the case's instant, with each review's XP the
    case's."""
    loot = predecessor("pipeline_layers.loot")
    xp_module = predecessor("gamification.xp")
    store = StubTokenStore(tokens)
    stand_in = types.SimpleNamespace(_store=store)
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
    by_id = {r["id"]: r["xp"] for r in reviews}
    if len(by_id) != len(reviews):
        raise ValueError("the case's reviews must have distinct ids")

    def review_xp(row, cid_tier=None):
        return by_id[row.id_ms]

    config = predecessor("types.CollectionConfig")(
        rollover_hour=rollover_hour, tz_offset_minutes=tz_offset_minutes
    )
    with (
        mock.patch.object(loot, "datetime", a_clock(now_ms)),
        mock.patch.object(xp_module, "review_xp", review_xp),
    ):
        asyncio.run(recompute(stand_in, rows, config, as_date(study_day)))
    return {"grants": store.grants, "consumed": store.consumed}


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
    "sweep_stale_chests": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer._sweep_stale_chests",
        "adapter": with_a_stub_sweep,
        "note": "Sweeps once on a stand-in layer over a stub store holding the case's chests of "
        "several study days and states, each with its id in the case's order; returns the chests "
        "resolved, in order, and the XP grants written.",
        "cases": sweep_cases,
    },
    "activate_double_xp": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer.activate_double_xp",
        "adapter": with_a_stub_wallet_of_tokens,
        "note": "Activates once on a stand-in layer over a stub store holding the case's tokens, "
        "each with its id in the case's order, with the clock patched to the case's instant; "
        "returns the answer, the window's end and the activations written, in epoch milliseconds.",
        "cases": activation_cases,
    },
    "recompute_token_xp": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer._recompute_token_xp",
        "adapter": with_a_stub_token_day,
        "note": "Settles the token bonus of the case's study day once on a stand-in layer over the "
        "token store, under the case's rollover hour and offset, with the clock patched to the "
        "case's instant and xp.review_xp patched to the case's XP for each review; returns the 2x "
        "grants and the tokens consumed.",
        "cases": bonus_cases,
    },
}

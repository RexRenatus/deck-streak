"""SPEC-081's registrations for the sessions, the effort floor, the rarity roll, the Epic odds, the
payout and the chest constants (the quests core of #102 and #103).

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_reviews` builds the predecessor's `Review` rows from the case's instants, cards, answer
  buttons, types and times, then calls the function it was handed. The case carries the per-answer
  time cap the port is given; the adapter refuses a case whose cap differs from the cap the
  predecessor applies, so the cap a test reads from the golden is the one that was applied.
* `with_an_effort` builds the predecessor's `RealEffort` from the case's counts.

The draws a case aims at are multiples of 2^-53, as the predecessor's `random.SystemRandom().random()`
returns them. The case builders draw only from the `random.Random` the generator seeds. Every number
is synthetic, every instant epoch milliseconds.
"""

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
}

"""SPEC-084's registrations: the celebration ladder's tier rules, the weekly budgets, the
honest-outcome cap, the near-miss gate, the streak-break check, and the constants the port uses
verbatim.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_a_stub_store` drives `pipeline_layers/celebrations.py:CelebrationsLayer._streak_broke_today`
  on a stand-in layer whose stub store answers `get_streak_state()` with the case's last study
  day, current and longest streak, and the day as a `date` from its epoch day number.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic and every day is an epoch day number.
"""

import asyncio
import datetime as dt
import types

#: The study day the cases centre on, as a whole-day offset from the Unix epoch.
PRESENT_DAY = 20_000
#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()

EVENTS = (
    "badge",
    "level_up",
    "quest",
    "quest_all",
    "record",
    "season_node",
    "season_node_final",
    "queue_zero",
    "ghost_win",
    "band_up",
    "ceremony",
    "share_card",
    "landmark_anniversary",
    "landmark_study_day",
    "qaa_unknown_event",
    "",
)
RARITIES = ("common", "rare", "epic", "legendary", "qaa_unknown_rarity", "")
INTENSITIES = ("quiet", "standard", "loud", "qaa_unknown_intensity", "")


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def requested_cases(rng):
    """Every event with every rarity, so the rarity's win and each fall-through are drawn."""
    return [
        (
            "rarity" if rarity and rarity in RARITIES[:4] else None,
            {"event_type": event, "rarity": rarity},
        )
        for event in EVENTS
        for rarity in RARITIES
    ]


def budget_cases(rng):
    return [(None, {"intensity": intensity}) for intensity in INTENSITIES]


def apply_cases(rng):
    """Each tier at, under and over its slots, exempt and rare, over every budget the policy has."""
    cases = []
    for budget in ([1, 0], [2, 1], [3, 2]):
        for requested in range(0, 6):
            for t4_used in sorted({0, budget[0] - 1, budget[0]}):
                for t5_used in sorted({0, budget[1] - 1, budget[1]}):
                    if t4_used < 0 or t5_used < 0:
                        continue
                    for exempt, rare in ((False, False), (False, True), (True, False)):
                        cases.append(
                            (
                                "exempt" if exempt else None,
                                {
                                    "requested": requested,
                                    "t4_used": t4_used,
                                    "t5_used": t5_used,
                                    "budget": budget,
                                    "exempt": exempt,
                                    "rare": rare,
                                },
                            )
                        )
    return cases


def cap_cases(rng):
    return [(None, {"broke_today": False}), (None, {"broke_today": True})]


def near_miss_cases(rng):
    """Gaps at and beyond 5 units and 10 percent, and zero and negative gaps and targets."""
    cases = []
    for remaining, target in (
        (0, 10),
        (-1, 10),
        (1, 0),
        (1, -5),
        (5, 100),
        (5.0, 1000),
        (5.5, 100),
        (6, 100),
        (10, 100),
        (10.5, 100),
        (11, 100),
        (0.5, 3),
        (30, 300),
        (31, 300),
        (29.99, 300),
        (6, 60),
        (6, 59),
        (6, 61),
    ):
        cases.append(("boundary", {"remaining": remaining, "target": target}))
    for _ in range(40):
        target = rng.choice([10, 50, 100, 250, 1000])
        cases.append((None, {"remaining": rng.randint(-2, target), "target": target}))
    return cases


def with_a_stub_store(streak_broke_today, predecessor, *, today, last_study_day, current, longest):
    """A stand-in layer whose stub store answers the streak state."""
    state = types.SimpleNamespace(
        last_study_day=None if last_study_day is None else as_date(last_study_day),
        current=current,
        longest=longest,
    )

    async def get_streak_state():
        return state

    layer = types.SimpleNamespace(_store=types.SimpleNamespace(get_streak_state=get_streak_state))
    return asyncio.run(streak_broke_today(layer, as_date(today)))


def broke_cases(rng):
    cases = []
    for last in (PRESENT_DAY, PRESENT_DAY - 1, None):
        for current in (0, 1, 2):
            for longest in (0, 1, 2, 30):
                cases.append(
                    (
                        "break" if (last == PRESENT_DAY and current == 1 and longest > 1) else None,
                        {
                            "today": PRESENT_DAY,
                            "last_study_day": last,
                            "current": current,
                            "longest": longest,
                        },
                    )
                )
    return cases


FUNCTIONS = {
    "requested_tier": {
        "kind": "function",
        "function": "gamification.ladder.requested_tier",
        "cases": requested_cases,
    },
    "weekly_budget": {
        "kind": "function",
        "function": "gamification.ladder.weekly_budget",
        "cases": budget_cases,
    },
    "apply_budget": {
        "kind": "function",
        "function": "gamification.ladder.apply_budget",
        "cases": apply_cases,
    },
    "outcome_cap": {
        "kind": "function",
        "function": "gamification.ladder.outcome_cap",
        "cases": cap_cases,
    },
    "near_miss_ok": {
        "kind": "function",
        "function": "gamification.ladder.near_miss_ok",
        "cases": near_miss_cases,
    },
    "streak_broke_today": {
        "kind": "adapter",
        "function": "pipeline_layers.celebrations.CelebrationsLayer._streak_broke_today",
        "adapter": with_a_stub_store,
        "note": (
            "Builds a stand-in layer whose stub store answers the streak state (last study day, "
            "current and longest, the day as a date from its epoch day number) and calls the "
            "streak-break check on it."
        ),
        "cases": broke_cases,
    },
}
FUNCTIONS["ladder.constants"] = {
    "kind": "constants",
    "names": [
        "gamification.ladder._RARITY_TIER",
        "gamification.ladder._EVENT_TIER",
        "gamification.ladder._RARE_FLOOR_TIER",
        "constants.CELEBRATION_BUDGETS",
        "constants.CELEBRATION_SEND_RETRY_MAX",
        "constants.CELEBRATION_OUTAGE_COOLDOWN_MS",
        "pipeline_layers.celebrations._REVEAL_SUSPENSE_SECS",
    ],
}

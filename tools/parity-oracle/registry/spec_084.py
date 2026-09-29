"""SPEC-084's registrations: the celebration ladder's tier rules, the weekly budgets, the
honest-outcome cap, the near-miss gate, the streak-break check, and the constants the port uses
verbatim.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_a_stub_store` drives `pipeline_layers/celebrations.py:CelebrationsLayer._streak_broke_today`
  on a stand-in layer whose stub store answers `get_streak_state()` with the case's last study
  day, current and longest streak, and the day as a `date` from its epoch day number.

* `with_a_recording_notifier` drives `CelebrationsLayer.celebrate` on a stand-in layer.
* `with_a_temporary_store` drives `GamifyStore.celebrations_at_or_above` on a temporary store the
  predecessor's own `GamifyStore` creates, seeded through its own recording methods.
* `with_a_latest_owner_message` drives `CelebrationsLayer._react_to_owner`.
* `with_held_rows` drives `CelebrationsLayer.flush_deferred_celebrations`.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic and every day is an epoch day number.
"""

import asyncio
import datetime as dt
import json
import tempfile
import types
from pathlib import Path
from unittest import mock

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
    """Gaps at and beyond 5 units and 10 percent, each bound once where it alone decides, and zero
    and negative gaps and targets."""
    cases = []
    for remaining, target in (
        (0, 10),
        (-1, 10),
        (1, 0),
        (1, -5),
        (5, 100),
        (5.0, 1000),
        (5, 40),
        (5.5, 40),
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


DAY_MS = 86_400_000
HOUR_MS = 3_600_000
MINUTE_MS = 60_000
#: The instant every stateful case runs at: noon of the present day, outside any quiet hours.
NOW_MS = PRESENT_DAY * DAY_MS + 12 * HOUR_MS
REACTION = "\U0001f389"


def instant_iso(ms):
    """An aware UTC instant as the ISO text the predecessor stores."""
    return dt.datetime.fromtimestamp(ms / 1000, tz=dt.UTC).isoformat()


class Notifier:
    """A recording notifier: every call is kept by name and non-text argument, and accepts
    unless the case names it in `fail`."""

    def __init__(self, fail):
        self.fail = set(fail)
        self.calls = []
        self.current = None
        self.count = 0

    def note(self, name, argument=None):
        self.calls.append([self.current, name, argument])
        return name not in self.fail

    async def send_html(self, text):
        return self.note("send_html")

    async def send_html_id(self, text):
        self.count += 1
        return self.count + 100 if self.note("send_html_id") else None

    async def edit_html(self, mid, text):
        return self.note("edit_html")

    async def send_dice(self, emoji):
        return self.note("send_dice", emoji)

    async def pin_message(self, mid):
        return self.note("pin_message")

    async def set_reaction(self, mid, emoji):
        return self.note("set_reaction", emoji)


def stand_in(predecessor, store, notifier, *, reaction_open=False, broke=False, owner=None):
    """A `CelebrationsLayer` built without its constructor, holding the stub store and the
    recording notifier, the clock at `NOW_MS`, quiet hours and the payload breaker closed, the
    reaction breaker as the case says, and the streak-break check patched to the case's answer."""
    layer = object.__new__(predecessor("pipeline_layers.celebrations.CelebrationsLayer"))
    layer._store = store
    layer._notifier = notifier
    layer._cfg = None
    layer._settings = types.SimpleNamespace(
        notify_milestones=True, rollover_hour=4, tz_offset_minutes=0
    )
    layer._now_ms = lambda: NOW_MS
    layer._today = lambda: as_date(PRESENT_DAY)
    layer._notify_down_ms = 0
    layer._reaction_down_ms = NOW_MS - 1_000 if reaction_open else 0

    async def quiet():
        return False

    async def broke_today(today):
        return broke

    async def repin(today):
        return None

    layer._in_quiet_hours_now = quiet
    layer._streak_broke_today = broke_today
    layer._repin_widget = repin
    return layer


def owner_message(owner):
    """The `last_owner_msg` setting the predecessor reads: an id and the instant it was seen."""
    if owner is None:
        return None
    return f"{owner['id']}:{instant_iso(NOW_MS - int(owner['age_minutes'] * MINUTE_MS))}"


def fixed_clock(predecessor):
    """The module's `datetime` with `now` fixed at `NOW_MS`."""
    real = predecessor("pipeline_layers.celebrations").datetime

    class Fixed(real):
        @classmethod
        def now(cls, tz=None):
            return real.fromtimestamp(NOW_MS / 1000, tz=tz)

    return Fixed


class Store:
    """A stub store: it answers the case's settings and counts and keeps what it is told."""

    def __init__(self, *, intensity=None, owner=None, at_or_above=None, rows=()):
        self.intensity = intensity
        self.owner = owner_message(owner)
        self.at_or_above = at_or_above or {}
        self.rows = [dict(row) for row in rows]
        self.rendered = None
        self.pending = None
        self.latched = []
        self.settled = []
        self.notifier = None

    async def get_setting(self, key):
        if key == "celebration_intensity":
            return self.intensity
        if key == "last_owner_msg":
            return self.owner
        return None

    async def celebrations_at_or_above(self, since_day, min_tier):
        return self.at_or_above.get(min_tier, 0)

    async def record_celebration(self, **fields):
        self.rendered = fields["tier_rendered"]
        return True

    async def record_celebration_pending(self, **fields):
        self.pending = fields["tier_pending"]
        return True

    async def latch_celebration_pending(self, event_key, *, tier_pending, deferred_at, pending_payload):
        payload = json.loads(pending_payload)
        self.latched.append(
            [event_key, tier_pending, payload.get("hold"), payload.get("tries")]
        )

    async def get_pending_celebrations(self):
        return self.rows

    async def settle_pending_celebration(self, event_key, *, tier_rendered):
        self.settled.append([event_key, tier_rendered])
        self.notifier.current = event_key
        return True


def celebrate_cases(rng):
    """Every event and rarity at a spread of intensities, week counts and outcomes, with the
    reaction and dice and pin and edit each refused in turn."""
    cases = []
    for event in EVENTS[:14]:
        for intensity in (None, "quiet", "loud"):
            for t4, t5 in ((0, 0), (2, 1)):
                cases.append(
                    (
                        None,
                        {
                            "event_type": event,
                            "rarity": "",
                            "budget_exempt": event == "band_up",
                            "intensity": intensity,
                            "at_or_above_4": t4 + t5,
                            "at_or_above_5": t5,
                            "broke": False,
                            "fail": [],
                            "reaction_open": False,
                            "owner": {"id": 7, "age_minutes": 60},
                            "dice": None,
                        },
                    )
                )
    for rarity in ("common", "rare", "epic", "legendary"):
        for intensity in ("quiet", "loud"):
            for t4, t5 in ((0, 0), (5, 4)):
                cases.append(
                    (
                        "rarity",
                        {
                            "event_type": "badge",
                            "rarity": rarity,
                            "budget_exempt": False,
                            "intensity": intensity,
                            "at_or_above_4": t4 + t5,
                            "at_or_above_5": t5,
                            "broke": False,
                            "fail": [],
                            "reaction_open": False,
                            "owner": {"id": 7, "age_minutes": 60},
                            "dice": "qaa-dice" if rarity == "legendary" else None,
                        },
                    )
                )
    for event in ("badge", "quest_all", "queue_zero", "band_up"):
        for broke in (True, False):
            for fail in ([], ["set_reaction"], ["send_html"], ["send_html_id"], ["edit_html"],
                         ["send_dice"], ["pin_message"]):
                for owner in ({"id": 7, "age_minutes": 60}, {"id": 7, "age_minutes": 25 * 60}):
                    cases.append(
                        (
                            "outcome" if broke else None,
                            {
                                "event_type": event,
                                "rarity": "",
                                "budget_exempt": False,
                                "intensity": "standard",
                                "at_or_above_4": 0,
                                "at_or_above_5": 0,
                                "broke": broke,
                                "fail": fail,
                                "reaction_open": False,
                                "owner": owner,
                                "dice": None,
                            },
                        )
                    )
    for owner in ({"id": 7, "age_minutes": 60}, None):
        for event in ("badge", "queue_zero"):
            cases.append(
                (
                    "breaker",
                    {
                        "event_type": event,
                        "rarity": "",
                        "budget_exempt": False,
                        "intensity": "standard",
                        "at_or_above_4": 0,
                        "at_or_above_5": 0,
                        "broke": True,
                        "fail": [],
                        "reaction_open": True,
                        "owner": owner,
                        "dice": None,
                    },
                )
            )
    return cases


def with_a_recording_notifier(
    celebrate,
    predecessor,
    *,
    event_type,
    rarity,
    budget_exempt,
    intensity,
    at_or_above_4,
    at_or_above_5,
    broke,
    fail,
    reaction_open,
    owner,
    dice,
):
    """Runs one celebration and returns its rendered tier, its calls and its latched hold."""
    notifier = Notifier(fail)
    store = Store(
        intensity=intensity,
        owner=owner,
        at_or_above={4: at_or_above_4, 5: at_or_above_5},
    )
    store.notifier = notifier
    layer = stand_in(predecessor, store, notifier, reaction_open=reaction_open, broke=broke)
    module = predecessor("pipeline_layers.celebrations")
    pauses = []

    async def sleep(seconds):
        notifier.calls.append([None, "sleep", seconds])

    with mock.patch.object(module.asyncio, "sleep", sleep), mock.patch.object(
        module, "datetime", fixed_clock(predecessor)
    ):
        delivered = asyncio.run(
            celebrate(
                layer,
                event_type=event_type,
                event_key=f"qaa-{event_type}",
                text="qaa text",
                rarity=rarity,
                dice_emoji=dice,
                budget_exempt=budget_exempt,
            )
        )
    return {
        "delivered": delivered,
        "rendered": store.rendered,
        "pending": store.pending,
        "calls": [[call[1], call[2]] for call in notifier.calls],
        "latched": store.latched,
    }


def with_a_temporary_store(celebrations_at_or_above, predecessor, *, since_day, min_tier, rows):
    """Creates the predecessor's own `GamifyStore` in a temporary directory, records each row
    through `record_celebration` or `record_celebration_pending`, and counts."""
    store_class = predecessor("database.GamifyStore")

    async def run(path):
        store = await store_class(path).connect()
        try:
            for n, row in enumerate(rows):
                fields = {
                    "day": as_date(row["day"]),
                    "event_key": f"qaa-{n}",
                    "event_type": "badge",
                    "rarity": "",
                    "tier_requested": row["tier"],
                }
                if row["held"]:
                    await store.record_celebration_pending(
                        tier_pending=row["tier"],
                        deferred_at=instant_iso(NOW_MS),
                        pending_payload="{}",
                        **fields,
                    )
                else:
                    await store.record_celebration(tier_rendered=row["tier"], **fields)
                if row.get("abandoned"):
                    await store.settle_pending_celebration(f"qaa-{n}", tier_rendered=0)
            return await celebrations_at_or_above(store, as_date(since_day), min_tier)
        finally:
            await store.close()

    with tempfile.TemporaryDirectory() as directory:
        return asyncio.run(run(Path(directory) / "qaa.db"))


def week_count_cases(rng):
    """Rows before, on and after the week's first day, at every tier, rendered and held, and
    an abandoned one."""
    week = PRESENT_DAY - (PRESENT_DAY + 3) % 7
    cases = []
    for min_tier in (4, 5):
        rows = []
        for day in (week - 1, week, week + 3, week + 6, week + 7):
            for tier in (0, 1, 3, 4, 5):
                rows.append({"day": day, "tier": tier, "held": False})
        cases.append(("rendered", {"since_day": week, "min_tier": min_tier, "rows": rows}))
        held = [{"day": week + 1, "tier": tier, "held": True} for tier in (1, 3, 4, 5)]
        cases.append(("held", {"since_day": week, "min_tier": min_tier, "rows": held}))
        gone = [
            {"day": week + 1, "tier": 5, "held": True, "abandoned": True},
            {"day": week + 2, "tier": 4, "held": True},
            {"day": week + 2, "tier": 5, "held": False},
        ]
        cases.append(("abandoned", {"since_day": week, "min_tier": min_tier, "rows": gone}))
    for _ in range(20):
        rows = [
            {
                "day": week + rng.randint(-3, 9),
                "tier": rng.randint(0, 5),
                "held": rng.random() < 0.4,
            }
            for _ in range(rng.randint(0, 8))
        ]
        cases.append((None, {"since_day": week, "min_tier": rng.choice([4, 5]), "rows": rows}))
    return cases


def with_a_latest_owner_message(react_to_owner, predecessor, *, owner, fail, reaction_open):
    """Runs the reaction attempt and returns whether one was attempted and its emoji."""
    notifier = Notifier(fail)
    store = Store(owner=owner)
    layer = stand_in(predecessor, store, notifier, reaction_open=reaction_open)
    module = predecessor("pipeline_layers.celebrations")
    with mock.patch.object(module, "datetime", fixed_clock(predecessor)):
        outcome = asyncio.run(react_to_owner(layer))
    return {
        "outcome": outcome,
        "calls": [[call[1], call[2]] for call in notifier.calls],
    }


def reaction_cases(rng):
    """Messages just inside, on and beyond the age, an absent one, and an open breaker."""
    cases = []
    for age in (0, 1, 60, 23 * 60, 24 * 60 - 1, 24 * 60, 24 * 60 + 1, 30 * 60):
        for fail in ([], ["set_reaction"]):
            cases.append(
                ("age", {"owner": {"id": 9, "age_minutes": age}, "fail": fail, "reaction_open": False})
            )
    cases.append(("absent", {"owner": None, "fail": [], "reaction_open": False}))
    cases.append(("open", {"owner": {"id": 9, "age_minutes": 5}, "fail": [], "reaction_open": True}))
    return cases


def with_held_rows(flush, predecessor, *, rows, broke, fail, owner):
    """Runs one flush over the case's held rows and returns what settled, what was called and
    what was latched again."""
    notifier = Notifier(fail)
    held = [
        {
            "id": row["order"],
            "event_key": row["key"],
            "event_type": row["event_type"],
            "tier_pending": row["tier"],
            "deferred_at": instant_iso(NOW_MS - int(row["age_minutes"] * MINUTE_MS)),
            "pending_payload": json.dumps(
                {"text": "qaa text", "dice_emoji": row["dice"], "hold": row["hold"], "tries": row["tries"]}
            ),
        }
        for row in rows
    ]
    store = Store(owner=owner, rows=held)
    store.notifier = notifier
    layer = stand_in(predecessor, store, notifier, broke=broke)
    module = predecessor("pipeline_layers.celebrations")
    with mock.patch.object(module.asyncio, "sleep", lambda seconds: asyncio.sleep(0)), mock.patch.object(
        module, "datetime", fixed_clock(predecessor)
    ):
        engaged = asyncio.run(flush(layer))
    return {
        "engaged": engaged,
        "settled": store.settled,
        "calls": [list(call) for call in notifier.calls],
        "latched": store.latched,
    }


def held_row(order, tier, age=60, hold="quiet", tries=0, event_type="badge", dice=None):
    return {
        "order": order,
        "key": f"qaa-{order}",
        "event_type": event_type,
        "tier": tier,
        "age_minutes": age,
        "hold": hold,
        "tries": tries,
        "dice": dice,
    }


def flush_cases(rng):
    """Held sets over the bounds: two in full, ties, a rollup, a queue of twenty and past it,
    the age cap, the streak-break cap, a T1 hold, and a failed render."""
    fresh = {"id": 5, "age_minutes": 5}
    cases = [
        ("one", {"rows": [held_row(1, 3)], "broke": False, "fail": [], "owner": fresh}),
        ("two", {"rows": [held_row(1, 2), held_row(2, 5)], "broke": False, "fail": [], "owner": fresh}),
        (
            "tie",
            {
                "rows": [held_row(3, 4), held_row(1, 4), held_row(2, 4), held_row(4, 3)],
                "broke": False,
                "fail": [],
                "owner": fresh,
            },
        ),
        (
            "rollup",
            {
                "rows": [held_row(n, tier) for n, tier in enumerate((5, 4, 3, 3, 2, 1), start=1)],
                "broke": False,
                "fail": [],
                "owner": fresh,
            },
        ),
        (
            "broke",
            {
                "rows": [held_row(n, tier) for n, tier in enumerate((5, 4, 3, 2), start=1)],
                "broke": True,
                "fail": [],
                "owner": fresh,
            },
        ),
        (
            "broke_no_owner",
            {"rows": [held_row(1, 4)], "broke": True, "fail": [], "owner": None},
        ),
        (
            "age",
            {
                "rows": [held_row(1, 4, age=12 * 60), held_row(2, 4, age=12 * 60 + 1), held_row(3, 2)],
                "broke": False,
                "fail": [],
                "owner": fresh,
            },
        ),
        (
            "silent",
            {"rows": [held_row(1, 0), held_row(2, 3)], "broke": False, "fail": [], "owner": fresh},
        ),
        (
            "queue",
            {
                "rows": [held_row(n, 2 + n % 4) for n in range(1, 24)],
                "broke": False,
                "fail": [],
                "owner": fresh,
            },
        ),
        (
            "queue_edge",
            {
                "rows": [held_row(n, 3) for n in range(1, 21)],
                "broke": False,
                "fail": [],
                "owner": fresh,
            },
        ),
    ]
    for fail in (["send_html"], ["send_html_id"], ["set_reaction"], ["pin_message"], ["send_dice"]):
        cases.append(
            (
                "failure",
                {
                    "rows": [held_row(1, 5, dice="qaa-dice"), held_row(2, 4), held_row(3, 3)],
                    "broke": False,
                    "fail": fail,
                    "owner": fresh,
                },
            )
        )
    cases.append(
        (
            "t1",
            {
                "rows": [held_row(1, 1), held_row(2, 1, hold="send"), held_row(3, 3)],
                "broke": False,
                "fail": [],
                "owner": fresh,
            },
        )
    )
    cases.append(
        (
            "t1_refused",
            {
                "rows": [held_row(1, 1), held_row(2, 1)],
                "broke": False,
                "fail": ["set_reaction"],
                "owner": fresh,
            },
        )
    )
    cases.append(
        (
            "retries",
            {
                "rows": [held_row(1, 3, hold="send", tries=2)],
                "broke": False,
                "fail": ["send_html"],
                "owner": fresh,
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
    "celebration_tier": {
        "kind": "adapter",
        "function": "pipeline_layers.celebrations.CelebrationsLayer.celebrate",
        "adapter": with_a_recording_notifier,
        "note": (
            "Builds a stand-in layer with a stub store answering the case's intensity and week "
            "counts, the streak-break check patched to the case's answer, quiet hours and the "
            "payload breaker closed, the reaction breaker as the case says, the reveal's sleep "
            "patched to record its seconds and the module's clock fixed, and a recording "
            "notifier that accepts every call it is not told to refuse; returns the rendered "
            "tier and the ordered calls with their emoji and pause, and never a text."
        ),
        "cases": celebrate_cases,
    },
    "celebration_week_count": {
        "kind": "adapter",
        "function": "database.GamifyStore.celebrations_at_or_above",
        "adapter": with_a_temporary_store,
        "note": (
            "Creates the predecessor's own GamifyStore in a temporary directory, records each "
            "case row through record_celebration or record_celebration_pending (an abandoned "
            "one settled at tier 0) with its study day from its epoch day number, and counts."
        ),
        "cases": week_count_cases,
    },
    "reaction_freshness": {
        "kind": "adapter",
        "function": "pipeline_layers.celebrations.CelebrationsLayer._react_to_owner",
        "adapter": with_a_latest_owner_message,
        "note": (
            "Builds a stand-in layer whose stub store holds the case's latest owner message, "
            "the module's clock fixed, the reaction breaker as the case says, and a recording "
            "notifier; returns the outcome (attempted and delivered, attempted and refused, or "
            "not attempted) and the calls with their emoji."
        ),
        "cases": reaction_cases,
    },
    "celebration_flush": {
        "kind": "adapter",
        "function": "pipeline_layers.celebrations.CelebrationsLayer.flush_deferred_celebrations",
        "adapter": with_held_rows,
        "note": (
            "Builds a stand-in layer whose stub store holds the case's held rows (key, held "
            "tier, age, order), the clock fixed outside quiet hours, the streak-break check "
            "patched, and a recording notifier; returns what each key settled at in order, "
            "each call with the key it served and its emoji, and each row latched again."
        ),
        "cases": flush_cases,
    },
    "ladder.constants": {
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
    },
}

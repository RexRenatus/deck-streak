"""SPEC-076's registration: the language and law streaks, the freeze port, the governor and the
relight.

Each adapter builds what JSON cannot carry (dates, sets, a state object, a store) and CALLS the
predecessor; none computes a rule.

* The streak adapters build a `StreakState` from the case's fields and hand the predecessor its
  own function the last study day and the skip days as `date` values from epoch day numbers.
* `with_a_stub_store` drives `pipeline_layers/governor.py:GovernorLayer._update_governor` on a
  stand-in layer whose stub store holds the study days, the skip days and the stored governor
  state. The layer's own `_update_strength` and the predecessor's own `assess` run unchanged.
* `over_a_temporary_store` drives `pipeline_layers/loot.py:LootLayer.pick_epic_prize` on a
  stand-in layer over the predecessor's own store, opened on a temporary database.
* `with_a_recording_celebrate` drives `pipeline_layers/showcase.py:ShowcaseLayer._relight` on a
  stub store and a recording `celebrate`.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic and every day is an epoch day number.
"""

import asyncio
import datetime as dt
import tempfile
import types
from pathlib import Path

#: A study day number near the present, as a whole-day offset from the Unix epoch.
PRESENT_DAY = 20_000
#: The predecessor's silence walk cap, in days.
WALK_CAP = 120
#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()


def as_date(day):
    """The `date` of epoch day number `day`, or none for none."""
    if day is None:
        return None
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def as_day(date):
    """The epoch day number of `date`, or none for none."""
    if date is None:
        return None
    return date.toordinal() - EPOCH_ORDINAL


def state_of(predecessor, fields):
    """A `StreakState` from the case's fields; the transient ones default as the predecessor's."""
    state_type = predecessor("types.StreakState")
    return state_type(
        current=fields["current"],
        longest=fields["longest"],
        freezes=fields["freezes"],
        last_study_day=as_date(fields["last_study_day"]),
        comeback_armed=fields["comeback_armed"],
    )


def fields_of(state):
    """A `StreakState` as the fields a golden records."""
    return {
        "current": state.current,
        "longest": state.longest,
        "freezes": state.freezes,
        "last_study_day": state.last_study_day,
        "comeback_armed": state.comeback_armed,
        "heat": state.heat,
        "froze_today": state.froze_today,
        "broke_today": state.broke_today,
    }


def skips_of(skip_days):
    return {as_date(day) for day in skip_days}


def a_state(current, longest, freezes, last_study_day, comeback_armed=False):
    return {
        "current": current,
        "longest": longest,
        "freezes": freezes,
        "last_study_day": last_study_day,
        "comeback_armed": comeback_armed,
    }


# --- the streak transitions ------------------------------------------------------------------


def classifying(function, predecessor, *, state, today, skip_days):
    """The gap outcome the predecessor's classifier answers."""
    return function(state_of(predecessor, state), as_date(today), skips_of(skip_days))


def updating(function, predecessor, *, state, today, observed_streak, skip_days):
    """The state, with its last study day and transient flags, after a study on `today`."""
    return fields_of(
        function(state_of(predecessor, state), as_date(today), observed_streak, skips_of(skip_days))
    )


def decaying(function, predecessor, *, state, today, skip_days):
    """The state, as above, after a day that is not a study day."""
    return fields_of(function(state_of(predecessor, state), as_date(today), skips_of(skip_days)))


def viewing(function, predecessor, *, state):
    """The comeback flag the badge view shows."""
    return function(state_of(predecessor, state)).comeback_armed


def missing(function, predecessor, *, last_study, today, skip_days):
    """The real misses between two days."""
    return function(as_date(last_study), as_date(today), skips_of(skip_days))


def transition_cases(rng, *, with_observed):
    """Every gap class, with and without freezes and skips, then drawn states."""
    t = PRESENT_DAY
    out = []

    def add(name, state, today, skips=(), observed=1):
        arguments = {"state": state, "today": today, "skip_days": sorted(skips)}
        if with_observed:
            arguments["observed_streak"] = observed
        out.append((name, arguments))

    add("bootstrap", a_state(0, 0, 1, None), t, observed=5)
    add("bootstrap-with-no-history", a_state(0, 0, 1, None), t, observed=0)
    add("same-day", a_state(4, 9, 2, t), t)
    add("continue", a_state(4, 9, 2, t - 1), t)
    add("continue-across-a-skip", a_state(4, 9, 2, t - 2), t, skips=[t - 1])
    add("freeze", a_state(4, 9, 2, t - 2), t)
    add("freeze-across-a-skip", a_state(4, 9, 1, t - 3), t, skips=[t - 2])
    add("break-with-no-freeze", a_state(4, 9, 0, t - 2), t)
    add("break-two-misses", a_state(4, 9, 3, t - 3), t)
    add("break-two-misses-one-skip-and-no-freeze", a_state(4, 9, 0, t - 4), t, skips=[t - 2])
    add("break-of-seven-arms-the-comeback", a_state(7, 7, 0, t - 3), t)
    add("break-of-six-does-not-arm", a_state(6, 9, 0, t - 3), t)
    add("break-already-recorded", a_state(0, 12, 1, t - 5), t)
    add("earn-at-seven", a_state(6, 6, 1, t - 1), t)
    add("earn-at-seven-capped", a_state(6, 6, 3, t - 1), t)
    add("earn-at-fourteen", a_state(13, 13, 0, t - 1), t)
    add("freeze-then-earn", a_state(6, 6, 1, t - 2), t)
    add("skip-outside-the-gap", a_state(4, 9, 1, t - 3), t, skips=[t - 6, t - 3, t])
    add("comeback-stays-armed", a_state(3, 9, 2, t - 1, True), t)
    for _ in range(40):
        gap = rng.randrange(0, 7)
        last = t - gap
        skips = {last + rng.randrange(1, gap + 1) for _ in range(rng.randrange(0, 3))} if gap else set()
        current = rng.choice([0, 1, 5, 6, 7, 13, 20, 29, 99])
        state = a_state(
            current,
            current + rng.randrange(0, 40),
            rng.randrange(0, 4),
            None if rng.randrange(0, 12) == 0 else last,
            rng.randrange(0, 2) == 1,
        )
        add(None, state, t, skips=skips, observed=rng.randrange(0, 12))
    return out


def classify_cases(rng):
    return transition_cases(rng, with_observed=False)


def update_cases(rng):
    return transition_cases(rng, with_observed=True)


def decay_cases(rng):
    return transition_cases(rng, with_observed=False)


def badge_cases(rng):
    out = []
    for current in (0, 1, 7):
        for armed in (False, True):
            out.append((None, {"state": a_state(current, 9, 1, PRESENT_DAY - 1, armed)}))
    for _ in range(8):
        current = rng.randrange(0, 20)
        out.append((None, {"state": a_state(current, current, 1, PRESENT_DAY, rng.randrange(0, 2) == 1)}))
    return out


def miss_cases(rng):
    t = PRESENT_DAY
    out = [
        ("same-day", {"last_study": t, "today": t, "skip_days": []}),
        ("next-day", {"last_study": t - 1, "today": t, "skip_days": []}),
        ("one-miss", {"last_study": t - 2, "today": t, "skip_days": []}),
        ("one-miss-skipped", {"last_study": t - 2, "today": t, "skip_days": [t - 1]}),
        ("more-skips-than-misses", {"last_study": t - 2, "today": t, "skip_days": [t - 1, t - 5]}),
        ("skips-on-both-ends", {"last_study": t - 4, "today": t, "skip_days": [t - 4, t]}),
        ("a-day-in-the-future", {"last_study": t + 1, "today": t, "skip_days": []}),
    ]
    for _ in range(20):
        gap = rng.randrange(0, 12)
        skips = sorted({t - rng.randrange(0, 14) for _ in range(rng.randrange(0, 6))})
        out.append((None, {"last_study": t - gap, "today": t, "skip_days": skips}))
    return out


def heat_cases(rng):
    edges = [0, 1, 2, 6, 7, 8, 29, 30, 31, 99, 100, 101, 364, 365, 366, 1000]
    return [(None, {"days": days}) for days in edges + [rng.randrange(0, 500) for _ in range(10)]]


# --- the law streak --------------------------------------------------------------------------


def bridging(function, predecessor, *, days, today, skip_days):
    """The law streak the predecessor's bridged run answers."""
    return function({as_date(day) for day in days}, as_date(today), skips_of(skip_days))


def bridge_cases(rng):
    t = PRESENT_DAY
    out = [
        ("no-days", {"days": [], "today": t, "skip_days": []}),
        ("a-law-day-today", {"days": [t - 2, t - 1, t], "today": t, "skip_days": []}),
        ("no-law-review-yet", {"days": [t - 3, t - 2, t - 1], "today": t, "skip_days": []}),
        ("a-missed-day", {"days": [t - 4, t - 3, t - 1, t], "today": t, "skip_days": []}),
        ("a-missed-day-yesterday", {"days": [t - 4, t - 3, t - 2], "today": t, "skip_days": []}),
        ("skip-in-the-run", {"days": [t - 4, t - 3, t - 1, t], "today": t, "skip_days": [t - 2]}),
        ("skip-today", {"days": [t - 3, t - 2, t - 1], "today": t, "skip_days": [t]}),
        ("skip-yesterday-and-today", {"days": [t - 4, t - 3], "today": t, "skip_days": [t - 2, t - 1, t]}),
        ("skip-at-the-far-end", {"days": [t - 2, t - 1, t], "today": t, "skip_days": [t - 3, t - 4]}),
        ("skips-only", {"days": [], "today": t, "skip_days": [t - 1, t]}),
        ("skip-past-the-run", {"days": [t - 1, t], "today": t, "skip_days": [t - 5]}),
    ]
    for _ in range(24):
        days = sorted({t - rng.randrange(0, 16) for _ in range(rng.randrange(0, 14))})
        skips = sorted({t - rng.randrange(0, 16) for _ in range(rng.randrange(0, 5))})
        out.append((None, {"days": days, "today": t - rng.randrange(0, 2), "skip_days": skips}))
    return out


# --- the freeze events -----------------------------------------------------------------------


def eventing(function, predecessor, *, previous, state, today):
    """The event rows a transition produces, each day an epoch day (the plain pass converts)."""
    new = state_of(predecessor, state)
    new = types.SimpleNamespace(
        freezes=new.freezes,
        froze_today=state["froze_today"],
        broke_today=state["broke_today"],
    )
    return function(state_of(predecessor, previous), new, as_date(today))


def event_cases(rng):
    t = PRESENT_DAY

    def moved(freezes, froze=False, broke=False):
        return {"freezes": freezes, "froze_today": froze, "broke_today": broke,
                "current": 1, "longest": 1, "last_study_day": t, "comeback_armed": False}

    prev = a_state(6, 6, 1, t - 1)
    out = [
        ("no-event", {"previous": prev, "state": moved(1), "today": t}),
        ("consumed", {"previous": prev, "state": moved(0, froze=True), "today": t}),
        ("break", {"previous": prev, "state": moved(1, broke=True), "today": t}),
        ("earn", {"previous": prev, "state": moved(2), "today": t}),
        ("consume-and-earn", {"previous": prev, "state": moved(1, froze=True), "today": t}),
        ("earn-of-two", {"previous": prev, "state": moved(3), "today": t}),
        ("capped-earn-is-no-event", {"previous": a_state(6, 6, 3, t - 1), "state": moved(3), "today": t}),
        ("break-and-earn", {"previous": prev, "state": moved(2, broke=True), "today": t}),
    ]
    for _ in range(16):
        before = rng.randrange(0, 4)
        out.append((None, {
            "previous": a_state(rng.randrange(0, 30), 30, before, t - 1),
            "state": moved(rng.randrange(0, 4), rng.randrange(0, 2) == 1, rng.randrange(0, 2) == 1),
            "today": t,
        }))
    return out


# --- the freeze drop gate --------------------------------------------------------------------


async def pick(loot_layer, predecessor, *, today, held, events, choice):
    """Open a store on a temporary database, seed an opened Epic chest and settle it."""
    store_type = predecessor("database.GamifyStore")
    with tempfile.TemporaryDirectory() as directory:
        store = await store_type(Path(directory) / "gamify.db").connect()
        try:
            state = await store.get_streak_state()
            await store.set_streak_state(types.SimpleNamespace(
                current=state.current, longest=state.longest, freezes=held,
                last_study_day=state.last_study_day, comeback_armed=state.comeback_armed))
            for event in events:
                await store.add_freeze_event(as_date(event["day"]), event["delta"], event["reason"])
            await store.conn.execute(
                "INSERT INTO chests(day, session_start_ms, rarity, payout_xp, state) "
                "VALUES (?, -7, 'epic', 0, 'opened')",
                (as_date(today).isoformat(),),
            )
            await store.conn.commit()
            asked = []
            real = store.freeze_drops_in_month

            async def recording(month_prefix, reasons):
                asked.append(list(reasons))
                return await real(month_prefix, reasons)

            store.freeze_drops_in_month = recording
            celebrated = []

            async def celebrate(**kwargs):
                celebrated.append(kwargs.get("event_type"))

            stand_in = types.SimpleNamespace(_store=store, celebrate=celebrate)
            answer = await loot_layer(stand_in, 1, choice)
            after = await store.get_streak_state()
            cursor = await store.conn.execute("SELECT COUNT(*) FROM xp_tokens")
            tokens = (await cursor.fetchone())[0]
            cursor = await store.conn.execute("SELECT COUNT(*) FROM freeze_events")
            rows = (await cursor.fetchone())[0]
            return {
                "ok": answer["ok"],
                "choice": answer.get("choice"),
                "freezes": after.freezes,
                "tokens": tokens,
                "events_added": rows - len(events),
                "reasons": asked,
            }
        finally:
            await store.close()


def over_a_temporary_store(loot_layer, predecessor, *, today, held, events, choice):
    """Settle an opened Epic chest on the predecessor's own store; return what it paid."""
    return asyncio.run(pick(loot_layer, predecessor, today=today, held=held, events=events, choice=choice))


def gate_cases(rng):
    t = PRESENT_DAY  # 2024-10-04
    month_start = t - 3
    last_month = t - 6

    def ev(day, delta, reason):
        return {"day": day, "delta": delta, "reason": reason}

    out = [
        ("free-and-under-the-cap", {"today": t, "held": 1, "events": [], "choice": "freeze"}),
        ("held-at-the-cap", {"today": t, "held": 3, "events": [], "choice": "freeze"}),
        ("chest-drop-this-month", {"today": t, "held": 1, "events": [ev(month_start, 1, "chest")], "choice": "freeze"}),
        ("weekly-quest-drop-this-month", {"today": t, "held": 1, "events": [ev(month_start, 1, "weekly_quest")], "choice": "freeze"}),
        ("season-drop-this-month", {"today": t, "held": 1, "events": [ev(month_start, 1, "season")], "choice": "freeze"}),
        ("shop-does-not-count", {"today": t, "held": 1, "events": [ev(month_start, 1, "shop")], "choice": "freeze"}),
        ("streak-earn-does-not-count", {"today": t, "held": 1, "events": [ev(month_start, 1, "streak_earn")], "choice": "freeze"}),
        ("a-drop-last-month", {"today": t, "held": 1, "events": [ev(last_month, 1, "chest")], "choice": "freeze"}),
        ("a-negative-delta-does-not-count", {"today": t, "held": 1, "events": [ev(month_start, -1, "chest")], "choice": "freeze"}),
        ("a-negative-delta-beside-a-drop", {"today": t, "held": 1, "events": [ev(month_start, 1, "chest"), ev(month_start, -1, "chest")], "choice": "freeze"}),
        ("the-token-choice", {"today": t, "held": 1, "events": [], "choice": "token"}),
        ("no-freeze-held", {"today": t, "held": 0, "events": [], "choice": "freeze"}),
    ]
    for _ in range(12):
        events = [
            ev(t - rng.randrange(0, 10), rng.choice([-1, 0, 1, 2]), rng.choice(
                ["chest", "weekly_quest", "season", "shop", "streak_earn", "consumed"]))
            for _ in range(rng.randrange(0, 4))
        ]
        out.append((None, {"today": t, "held": rng.randrange(0, 4), "events": events, "choice": "freeze"}))
    return out


# --- strength and the governor ---------------------------------------------------------------


def strength_cases(rng):
    grid = [(prev, studied) for prev in (-0.5, 0.0, 0.3, 0.6, 0.999, 1.0, 1.5) for studied in (False, True)]
    grid += [(rng.random(), rng.randrange(0, 2) == 1) for _ in range(20)]
    return [(None, {"prev": prev, "studied": studied}) for prev, studied in grid]


def assessing(function, predecessor, *, strength, silent_days):
    """The verdict's fields and `armed`."""
    verdict = function(strength=strength, silent_days=silent_days)
    return {
        "strength": verdict.strength,
        "silent_days": verdict.silent_days,
        "standby": verdict.standby,
        "lapse": verdict.lapse,
        "armed": verdict.armed,
    }


def assess_cases(rng):
    grid = [(s, n) for s in (0.0, 0.59, 0.6, 0.61, 1.0) for n in (0, 2, 3, 4)]
    grid += [(rng.random(), rng.randrange(0, 8)) for _ in range(16)]
    return [(None, {"strength": s, "silent_days": n}) for s, n in grid]


class StubStore:
    """A store holding the case's skip days and stored governor state; it records the governor
    state the layer writes."""

    def __init__(self, skip_days, stored):
        self.skips = skips_of(skip_days)
        self.prior = stored
        self.written = None

    async def skip_days_set(self):
        return set(self.skips)

    async def get_governor_state(self):
        return dict(self.prior)

    async def set_governor_state(self, **state):
        self.written = state

    async def latest_strength(self):
        return None

    async def upsert_strength(self, day, value):
        return None


class Recorder:
    """A notifier that records each notice."""

    def __init__(self):
        self.sent = 0

    async def send_html(self, _text):
        self.sent += 1


def layer_of(predecessor, store, notifier, quiet, relit):
    layer_type = predecessor("pipeline_layers.governor.GovernorLayer")

    async def in_quiet_hours():
        return quiet

    async def relight(day):
        relit.append(day)

    stand_in = types.SimpleNamespace(
        _store=store, _notifier=notifier, _relight=relight, _in_quiet_hours_now=in_quiet_hours
    )
    stand_in._update_strength = types.MethodType(layer_type._update_strength, stand_in)
    return stand_in


def stored_of(anchor, standby, notified):
    stored = {}
    if anchor is not None:
        stored["lapse_since"] = as_date(anchor).isoformat()
    if standby:
        stored["standby"] = True
    if notified is not None:
        stored["standby_notified_day"] = as_date(notified).isoformat()
    return stored


def anchoring(update_governor, predecessor, *, today, study_days, skip_days, stored_anchor):
    """Run the governor update on a stand-in layer and return the anchor it writes as an epoch
    day, or none."""
    store = StubStore(skip_days, stored_of(stored_anchor, False, None))
    layer = layer_of(predecessor, store, None, False, [])
    asyncio.run(update_governor(layer, {as_date(day) for day in study_days}, as_date(today)))
    since = store.written["lapse_since"]
    return None if since is None else as_day(dt.date.fromisoformat(since))


def anchor_cases(rng):
    """Every silence is longer than the walk. The horizon is `today - 120`."""
    t = PRESENT_DAY
    horizon = t - WALK_CAP
    old = t - 300
    return [
        ("no-stored-anchor", {"today": t, "study_days": [old], "skip_days": [], "stored_anchor": None}),
        ("stored-at-the-horizon", {"today": t, "study_days": [old], "skip_days": [], "stored_anchor": horizon}),
        ("stored-older-than-the-horizon", {"today": t, "study_days": [old], "skip_days": [], "stored_anchor": horizon - 40}),
        ("stored-a-day-newer-than-the-horizon", {"today": t, "study_days": [old], "skip_days": [], "stored_anchor": horizon + 1}),
        ("stored-far-newer-than-the-horizon", {"today": t, "study_days": [old], "skip_days": [], "stored_anchor": t - 3}),
        ("skips-at-the-horizon-no-anchor", {"today": t, "study_days": [old], "skip_days": [horizon, horizon + 1], "stored_anchor": None}),
        ("skips-at-the-horizon-stored-between", {"today": t, "study_days": [old], "skip_days": [horizon, horizon + 1], "stored_anchor": horizon + 1}),
        ("skips-at-the-horizon-stored-older", {"today": t, "study_days": [old], "skip_days": [horizon, horizon + 1], "stored_anchor": horizon - 3}),
        ("empty-history-no-stored-anchor", {"today": t, "study_days": [], "skip_days": [], "stored_anchor": None}),
        ("empty-history-stored-at-the-horizon", {"today": t, "study_days": [], "skip_days": [], "stored_anchor": horizon}),
        ("empty-history-stored-older-than-the-horizon", {"today": t, "study_days": [], "skip_days": [], "stored_anchor": horizon - 30}),
        ("empty-history-stored-newer-than-the-horizon", {"today": t, "study_days": [], "skip_days": [], "stored_anchor": horizon + 10}),
        ("empty-history-skips-at-the-horizon", {"today": t, "study_days": [], "skip_days": [horizon], "stored_anchor": None}),
        ("empty-history-stored-and-skips", {"today": t, "study_days": [], "skip_days": [horizon, horizon + 1], "stored_anchor": horizon + 1}),
        ("study-day-just-outside-the-walk", {"today": t, "study_days": [horizon - 1], "skip_days": [], "stored_anchor": None}),
        ("study-day-just-outside-the-walk-stored", {"today": t, "study_days": [horizon - 1], "skip_days": [], "stored_anchor": horizon - 1}),
    ] + [
        (None, {
            "today": t,
            "study_days": sorted({old - rng.randrange(0, 60) for _ in range(rng.randrange(0, 4))}),
            "skip_days": sorted({t - rng.randrange(0, 125) for _ in range(rng.randrange(0, 5))}),
            "stored_anchor": rng.choice([None, horizon - rng.randrange(0, 40), horizon + rng.randrange(0, 40)]),
        })
        for _ in range(12)
    ]


def noticing(update_governor, predecessor, *, today, study_days, skip_days, standby, notified_day, quiet, notifier):
    """Run the governor update with a recording notifier; return whether a notice was sent and
    the notice day stored, as an epoch day."""
    store = StubStore(skip_days, stored_of(None, standby, notified_day))
    recorder = Recorder() if notifier else None
    layer = layer_of(predecessor, store, recorder, quiet, [])
    asyncio.run(update_governor(layer, {as_date(day) for day in study_days}, as_date(today)))
    stored = store.written["standby_notified_day"]
    return {
        "sent": recorder is not None and recorder.sent > 0,
        "notified_day": None if stored is None else as_day(dt.date.fromisoformat(stored)),
        "standby": store.written["standby"],
        "lapse": store.written["lapse_since"] is not None,
    }


def notice_cases(rng):
    t = PRESENT_DAY

    def run(length, end):
        return [end - i for i in range(length)]

    base = {"today": t, "skip_days": [], "standby": False, "notified_day": None, "quiet": False, "notifier": True}
    out = [
        ("a-new-standby", {**base, "study_days": run(3, t)}),
        ("standby-since-yesterday", {**base, "study_days": run(3, t), "standby": True}),
        ("quiet-hours", {**base, "study_days": run(3, t), "quiet": True}),
        ("no-notifier", {**base, "study_days": run(3, t), "notifier": False}),
        ("notified-six-days-ago", {**base, "study_days": run(3, t), "notified_day": t - 6}),
        ("notified-seven-days-ago", {**base, "study_days": run(3, t), "notified_day": t - 7}),
        ("notified-long-ago", {**base, "study_days": run(3, t), "notified_day": t - 60}),
        ("lapse-suppresses-the-notice", {**base, "study_days": run(3, t - 4)}),
        ("silent-two-days-is-no-lapse", {**base, "study_days": run(3, t - 2)}),
        ("silent-three-days-is-a-lapse", {**base, "study_days": run(3, t - 3)}),
        ("a-strong-run-is-not-standby", {**base, "study_days": run(40, t)}),
        ("a-skip-bridges-the-silence", {**base, "study_days": run(3, t - 3), "skip_days": [t - 2, t - 1]}),
        ("standby-holds-the-notice-day", {**base, "study_days": run(3, t), "standby": True, "notified_day": t - 2}),
    ]
    for _ in range(14):
        length = rng.randrange(1, 30)
        out.append((None, {
            **base,
            "study_days": run(length, t - rng.randrange(0, 4)),
            "standby": rng.randrange(0, 2) == 1,
            "notified_day": rng.choice([None, t - rng.randrange(0, 12)]),
            "quiet": rng.randrange(0, 4) == 0,
        }))
    return out


# --- the relight -----------------------------------------------------------------------------


class RelightStore:
    """A store with the day's review count and any relight grant already written."""

    def __init__(self, reviews, granted):
        self.reviews = reviews
        self.granted = granted
        self.writes = []

    async def get_daily_rollup(self, _day):
        return None if self.reviews is None else {"reviews": self.reviews}

    async def xp_source_amount(self, _day, _source):
        return self.granted

    async def upsert_xp_grant(self, _day, source, amount):
        self.writes.append((source, amount))


def relighting(relight, predecessor, *, today, reviews, granted):
    """Run the relight on a stub store and a recording `celebrate`; return the grant's amount and
    the celebration's event type and key, the key's day an epoch day number."""
    store = RelightStore(reviews, granted)
    celebrations = []

    async def celebrate(**kwargs):
        celebrations.append(kwargs)

    stand_in = types.SimpleNamespace(_store=store, celebrate=celebrate)
    asyncio.run(relight(stand_in, as_date(today)))

    def epoch_key(key):
        prefix, _, iso = key.partition(":")
        return f"{prefix}:{as_day(dt.date.fromisoformat(iso))}"

    return {
        "amount": store.writes[0][1] if store.writes else None,
        "source": epoch_key(store.writes[0][0]) if store.writes else None,
        "event_type": celebrations[0]["event_type"] if celebrations else None,
        "event_key": epoch_key(celebrations[0]["event_key"]) if celebrations else None,
    }


def relight_cases(rng):
    t = PRESENT_DAY
    out = [
        ("no-rollup", {"today": t, "reviews": None, "granted": 0}),
        ("no-reviews", {"today": t, "reviews": 0, "granted": 0}),
        ("one-short", {"today": t, "reviews": 2, "granted": 0}),
        ("exactly-three", {"today": t, "reviews": 3, "granted": 0}),
        ("many-reviews", {"today": t, "reviews": 40, "granted": 0}),
        ("already-granted", {"today": t, "reviews": 5, "granted": 100}),
        ("a-smaller-grant-already-written", {"today": t, "reviews": 5, "granted": 1}),
    ]
    for _ in range(8):
        out.append((None, {"today": t - rng.randrange(0, 30), "reviews": rng.randrange(0, 9),
                           "granted": rng.choice([0, 0, 100])}))
    return out


# --- the streak calendar ------------------------------------------------------------------------


class HeatmapReads(dict):
    """The calendar's day mapping as the predecessor builds it, recording each day key its heatmap
    reads and the value it reads for that key, in order."""

    def __init__(self, inner):
        super().__init__(inner)
        self.reads = []

    def get(self, key, default=None):
        value = super().get(key, default)
        self.reads.append((key, value))
        return value


def with_a_spy_on_the_heatmap(streak_calendar, predecessor, *, today, rollups):
    """Call the predecessor's calendar with a spy on `charts._heatmap`; return every day its heatmap
    reads (the window) and the days it reads as studied, in order, as epoch day numbers. The
    predecessor builds its day mapping and draws its chart unchanged; the spy only records."""
    charts = predecessor("charts")
    heatmap = charts._heatmap
    spied = []

    def spy(by_day, **kwargs):
        recording = HeatmapReads(by_day)
        spied.append(recording)
        return heatmap(recording, **kwargs)

    charts._heatmap = spy
    try:
        streak_calendar(
            [
                {"day": as_date(row["day"]).isoformat(), "reviews": row["reviews"]}
                for row in rollups
            ],
            today=as_date(today),
            fmt="png",
        )
    finally:
        charts._heatmap = heatmap
    (recording,) = spied
    return {
        "window": [as_day(dt.date.fromisoformat(key)) for key, _ in recording.reads],
        "studied": [as_day(dt.date.fromisoformat(key)) for key, value in recording.reads if value],
    }


def calendar_cases(rng):
    """One served day on each weekday over a seeded mix of studied, unstudied and absent rollups
    reaching past the longest window, then the edges: no rollup, every rollup unstudied, every day
    studied, and rollups past the served day."""
    out = []
    for today in range(PRESENT_DAY, PRESENT_DAY + 7):
        rollups = []
        for day in range(today - 200, today + 1):
            draw = rng.random()
            if draw < 0.15:
                continue
            rollups.append({"day": day, "reviews": 0 if draw < 0.35 else rng.randint(1, 40)})
        weekday = as_date(today).strftime("%A").lower()
        out.append((f"served-on-a-{weekday}", {"today": today, "rollups": rollups}))
    t = PRESENT_DAY
    span = range(t - 200, t + 1)
    out += [
        ("no-rollup", {"today": t, "rollups": []}),
        (
            "every-rollup-unstudied",
            {"today": t, "rollups": [{"day": d, "reviews": 0} for d in span]},
        ),
        ("every-day-studied", {"today": t, "rollups": [{"day": d, "reviews": 1} for d in span]}),
        (
            "rollups-past-the-served-day",
            {"today": t, "rollups": [{"day": d, "reviews": 5} for d in range(t - 3, t + 4)]},
        ),
    ]
    return out


FUNCTIONS = {
    "classify_gap": {
        "kind": "adapter",
        "function": "gamification.streak.classify_gap",
        "adapter": classifying,
        "note": (
            "Builds a StreakState from the case's fields, its last study day and the skip days "
            "from epoch day numbers, and returns the outcome."
        ),
        "cases": classify_cases,
    },
    "update_on_study": {
        "kind": "adapter",
        "function": "gamification.streak.update_on_study",
        "adapter": updating,
        "note": (
            "Builds the state, the day, the observed raw streak and the skip days as above, and "
            "returns the new state with its last study day as an epoch day and its transient flags."
        ),
        "cases": update_cases,
    },
    "decay_on_lapse": {
        "kind": "adapter",
        "function": "gamification.streak.decay_on_lapse",
        "adapter": decaying,
        "note": (
            "Builds the state, the day and the skip days as above, and returns the state the "
            "lapse transition answers."
        ),
        "cases": decay_cases,
    },
    "heat_for": {
        "kind": "function",
        "function": "gamification.streak.heat_for",
        "cases": heat_cases,
    },
    "badge_view": {
        "kind": "adapter",
        "function": "gamification.streak.badge_view",
        "adapter": viewing,
        "note": "Builds a StreakState and returns the comeback flag the view shows.",
        "cases": badge_cases,
    },
    "real_misses": {
        "kind": "adapter",
        "function": "skip.real_misses",
        "adapter": missing,
        "note": "Builds the last study day, the day and the skip days from epoch day numbers.",
        "cases": miss_cases,
    },
    "bridged_streak": {
        "kind": "adapter",
        "function": "analytics.bridged_streak",
        "adapter": bridging,
        "note": (
            "Builds the law study days, the day asked about and the skip days from epoch day "
            "numbers, and returns the run the bridged streak counts."
        ),
        "cases": bridge_cases,
    },
    "freeze_events_for": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._freeze_events_for",
        "adapter": eventing,
        "note": (
            "Builds the previous state, the new state's freezes and transient flags, and the "
            "day; returns the event rows, each day an epoch day."
        ),
        "cases": event_cases,
    },
    "freeze_drop_gate": {
        "kind": "adapter",
        "function": "pipeline_layers.loot.LootLayer.pick_epic_prize",
        "adapter": over_a_temporary_store,
        "note": (
            "Opens the predecessor's own store on a temporary database seeded with an opened "
            "Epic chest on the case's day, the case's freeze events and held freezes; its "
            "freeze_drops_in_month is the predecessor's own and records the reasons it is asked "
            "for; returns whether the freeze or the token was paid, and those reasons."
        ),
        "cases": gate_cases,
    },
    "strength_advance": {
        "kind": "function",
        "function": "gamification.strength.advance",
        "cases": strength_cases,
    },
    "governor_assess": {
        "kind": "adapter",
        "function": "gamification.governor.assess",
        "adapter": assessing,
        "note": "Returns the verdict's fields and armed.",
        "cases": assess_cases,
    },
    "lapse_anchor_beyond_the_walk": {
        "kind": "adapter",
        "function": "pipeline_layers.governor.GovernorLayer._update_governor",
        "adapter": anchoring,
        "note": (
            "Runs the governor update on a stand-in layer over a stub store holding the study "
            "days, the skip days and the stored anchor, with the relight and the notifier "
            "stubbed; every case's silence is longer than the walk, an empty history included; "
            "returns the anchor written as an epoch day."
        ),
        "cases": anchor_cases,
    },
    "standby_notice": {
        "kind": "adapter",
        "function": "pipeline_layers.governor.GovernorLayer._update_governor",
        "adapter": noticing,
        "note": (
            "Runs the same stand-in with a recording notifier and the quiet-hours check set to "
            "the case's flag; returns whether a notice was sent and the notice day stored."
        ),
        "cases": notice_cases,
    },
    "relight": {
        "kind": "adapter",
        "function": "pipeline_layers.showcase.ShowcaseLayer._relight",
        "adapter": relighting,
        "note": (
            "Runs the relight on a stub store holding the day's review count and any grant "
            "already written, with a recording celebrate; returns the grant's amount and the "
            "celebration's event type and key, the key's day an epoch day number."
        ),
        "cases": relight_cases,
    },
    "streak_calendar": {
        "kind": "adapter",
        "function": "charts.streak_calendar",
        "adapter": with_a_spy_on_the_heatmap,
        "note": (
            "Calls the calendar on rollups whose days are ISO dates from epoch day numbers, with a "
            "spy on charts._heatmap that records each day key the heatmap reads (the window) and "
            "the value it reads for it (studied when above zero); returns both as epoch day "
            "numbers."
        ),
        "cases": calendar_cases,
    },
    "streaks.constants": {
        "kind": "constants",
        "names": [
            "constants.STREAK_START_FREEZES",
            "constants.STREAK_FREEZE_CAP",
            "constants.STREAK_DAYS_PER_FREEZE",
            "constants.STREAK_COMEBACK_MIN",
            "constants.STREAK_HEAT",
            "constants.FREEZE_DROP_MONTHLY_CAP",
            "constants.STRENGTH_HALF_LIFE_DAYS",
            "constants.STRENGTH_ARM_THRESHOLD",
            "constants.LAPSE_AFTER_SILENT_DAYS",
            "constants.RELIGHT_CARDS",
            "constants.RELIGHT_XP",
            "gamification.strength.STRENGTH_DECAY",
            "pipeline_layers.governor._SILENCE_WALK_CAP_DAYS",
            "pipeline_layers.governor._STRENGTH_PERSIST_DAYS",
        ],
    },
}

"""SPEC-049's registration: the open lapse and its id.

The adapter builds what JSON cannot carry and CALLS the predecessor; it computes no rule.

* `with_a_stub_store` drives `pipeline_layers/governor.py:GovernorLayer._update_governor` on a
  stand-in layer. Its stub store holds no stored anchor and no strength row, its notifier is none,
  and the layer's own `_update_strength` and the predecessor's own `assess` run unchanged. The
  study set is the days whose count is at least one, the days are handed over as `date` values
  from their epoch day numbers, and the answer is the `lapse_since` the layer stores, as an epoch
  day number, or none.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic and every day is an epoch day number. Each case's earliest listed day holds a study
review, so the predecessor's walk stops on it exactly where the port's window ends, and no run
reaches the predecessor's silence walk cap, past which its anchor is stored state.
"""

import asyncio
import datetime as dt
import types

#: A study day number near the present, as a whole-day offset from the Unix epoch.
PRESENT_DAY = 20_000
#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


class StubStore:
    """A store with no stored anchor, no strength row and the case's skip days; it records the
    governor state the layer writes."""

    def __init__(self, skip_days):
        self.skips = {as_date(day) for day in skip_days}
        self.stored = None

    async def skip_days_set(self):
        return set(self.skips)

    async def get_governor_state(self):
        return {}

    async def set_governor_state(self, **state):
        self.stored = state

    async def latest_strength(self):
        return None

    async def upsert_strength(self, day, value):
        return None


async def nothing(*_args, **_kwargs):
    """A coroutine that does nothing: a relight and the quiet-hours check never matter here."""
    return False


def with_a_stub_store(update_governor, predecessor, *, today, review_counts, skip_days):
    """Run the governor update on a stand-in layer and return the lapse id it stores, or none."""
    layer_type = predecessor("pipeline_layers.governor.GovernorLayer")
    store = StubStore(skip_days)
    stand_in = types.SimpleNamespace(
        _store=store, _notifier=None, _relight=nothing, _in_quiet_hours_now=nothing
    )
    stand_in._update_strength = types.MethodType(layer_type._update_strength, stand_in)
    study_days = {as_date(row["day"]) for row in review_counts if row["count"] >= 1}
    asyncio.run(update_governor(stand_in, study_days, as_date(today)))
    since = store.stored["lapse_since"]
    if since is None:
        return None
    return dt.date.fromisoformat(since).toordinal() - EPOCH_ORDINAL


def counts(studied, silent_with_zero=()):
    """Sparse rows: one review on each studied day and a zero row on each listed silent day."""
    rows = {day: 1 + (day % 4) for day in studied}
    for day in silent_with_zero:
        rows.setdefault(day, 0)
    return [{"day": day, "count": rows[day]} for day in sorted(rows)]


def case(today, studied, skips=(), zeros=()):
    return {"today": today, "review_counts": counts(studied, zeros), "skip_days": sorted(skips)}


def lapse_cases(rng):
    """Two silent, one short, three, a longer run, skips inside and at each end, a closing study
    day, zero rows, the walk's last exact run, then drawn windows."""
    t = PRESENT_DAY
    old = [t - 30, t - 29]
    drawn = [
        ("two-silent", case(t, [*old, t - 2])),
        ("one-silent", case(t, [*old, t - 1])),
        ("none-silent", case(t, [*old, t])),
        ("three-silent", case(t, [*old, t - 3])),
        ("four-silent", case(t, [*old, t - 4])),
        ("long-run", case(t, [*old, t - 25])),
        ("skip-inside", case(t, [*old, t - 5], skips=[t - 3])),
        ("skip-inside-short", case(t, [*old, t - 4], skips=[t - 2, t - 1])),
        ("skip-at-run-start", case(t, [*old, t - 4], skips=[t - 3])),
        ("skip-at-run-end", case(t, [*old, t - 4], skips=[t])),
        ("skip-at-both-ends", case(t, [*old, t - 5], skips=[t - 4, t])),
        ("skips-all-but-two", case(t, [*old, t - 6], skips=[t - 4, t - 3, t - 2, t - 1])),
        ("skip-on-a-study-day", case(t, [*old, t - 1], skips=[t - 1, t - 2])),
        ("skip-beyond-the-run", case(t, [*old, t - 3], skips=[t - 4, t - 20])),
        ("closing-study-day", case(t, [*old, t - 10, t])),
        ("closed-then-two-silent", case(t, [*old, t - 2, t - 8])),
        ("closed-then-three-silent", case(t, [*old, t - 3, t - 9])),
        ("zero-rows", case(t, [*old, t - 4], zeros=[t - 3, t - 2, t - 1])),
        ("zero-row-today", case(t, [*old, t - 2], zeros=[t])),
        ("walk-last-exact-run", case(t, [t - 121])),
        ("walk-short-of-the-cap", case(t, [t - 119, t - 121])),
    ]
    for _ in range(14):
        start = t - rng.randrange(20, 70)
        studied = {start} | {
            start + rng.randrange(0, t - start + 1) for _ in range(rng.randrange(0, 14))
        }
        skips = {start + rng.randrange(1, t - start + 1) for _ in range(rng.randrange(0, 5))}
        zeros = {start + rng.randrange(1, t - start + 1) for _ in range(rng.randrange(0, 5))}
        drawn.append((None, case(t, sorted(studied), sorted(skips), sorted(zeros))))
    return drawn


FUNCTIONS = {
    "lapse_episode": {
        "kind": "adapter",
        "function": "pipeline_layers.governor.GovernorLayer._update_governor",
        "adapter": with_a_stub_store,
        "note": (
            "Runs the governor update on a stand-in layer whose stub store holds the case's skip "
            "days and no stored anchor, with no notifier; the study set is the days holding at "
            "least one review, and it returns the lapse id the layer stores as an epoch day "
            "number, or none."
        ),
        "cases": lapse_cases,
    },
}

"""SPEC-078's registrations (part 078a): the reading minutes' XP, the course token, the most-used
course, the settled reading XP and weekly bonus, and the habits' constants.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_synthetic_courses` drives `curriculum.py:resolve_lang` with the predecessor's course and
  alias tables replaced by two synthetic courses, `qaa` (alias `a`) and `qab` (alias `b`), and
  returns the code it resolves, or none.
* `with_a_stub_log` drives `pipeline_layers/habits.py:HabitsLayer.most_used_reading_lang` on a
  stand-in layer over a stub store holding the case's entries, whose weekly sums come grouped by
  code in code order and whose rows come in the order they were logged, with the layer's day
  fixed at the case's day; it returns the code, or none.
* `with_a_stub_ledger` logs the case's entries one by one into a stub store and, after each,
  drives `pipeline_layers/habits.py:HabitsLayer._recompute_reading_xp` for the entry's code and
  day on a stand-in layer; it returns every grant the store holds at the end, as its day, source
  and amount, sorted.

The case builders draw only from the `random.Random` the generator seeds. Every course is
synthetic, every day an epoch day number.
"""

import asyncio
import datetime as dt
import types
from unittest import mock

#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: A Monday study day near the present: epoch day 20101.
MONDAY = 20_101
#: The synthetic courses, as the predecessor's tables hold a course: code to name and flag.
COURSES = {"qaa": ("Course Qaa", "F"), "qab": ("Course Qab", "F")}
#: The synthetic aliases.
ALIASES = {"a": "qaa", "b": "qab"}


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def day_number(day):
    """The epoch day number of `date` `day`."""
    return day.toordinal() - EPOCH_ORDINAL


# --- the minutes' XP ------------------------------------------------------------------------------


def xp_cases(rng):
    edges = [0, 1, 2, 59, 60, 119, 120, 121, 150, 240, 599, 600]
    cases = [("edge", {"minutes_today": minutes}) for minutes in edges]
    cases += [(None, {"minutes_today": rng.randint(0, 700)}) for _ in range(24)]
    return cases


# --- the course token -----------------------------------------------------------------------------


def token_cases(rng):
    tokens = ["qaa", "qab", "QAA", " qab ", "a", "B", "c", "qac", "", "aa", "qa", "1", "q-a"]
    cases = [("edge", {"token": token}) for token in tokens]
    letters = "abqAB "
    for _ in range(12):
        length = rng.randint(1, 4)
        cases.append((None, {"token": "".join(rng.choice(letters) for _ in range(length))}))
    return cases


def with_synthetic_courses(resolve_lang, predecessor, *, token):
    curriculum = predecessor("curriculum")
    with (
        mock.patch.object(curriculum, "LANGUAGES_BY_CODE", dict(COURSES)),
        mock.patch.object(curriculum, "LANG_ALIASES", dict(ALIASES)),
    ):
        return resolve_lang(token)


# --- the stub store -------------------------------------------------------------------------------


class StubReadingStore:
    """Holds the case's entries as `(code, day, minutes)`, in the order they were logged, and the
    grants upserted, keyed by day and source."""

    def __init__(self, rows):
        self.rows = list(rows)
        self.grants = {}

    async def reading_minutes_for_day(self, code, day):
        number = day_number(day)
        return sum(minutes for c, d, minutes in self.rows if c == code and d == number)

    async def reading_minutes_in_range(self, start, end):
        first, last = day_number(start), day_number(end)
        totals = {}
        for code in sorted({c for c, d, _ in self.rows if first <= d <= last}):
            totals[code] = sum(
                minutes for c, d, minutes in self.rows if c == code and first <= d <= last
            )
        return totals

    async def reading_rows_since(self, start):
        first = day_number(start)
        return [(c, as_date(d).isoformat(), minutes) for c, d, minutes in self.rows if d >= first]

    async def upsert_xp_grant(self, day, source, amount, *, track="language"):
        self.grants[(day_number(day), source)] = amount


def entries_of(rng, count, first_day, span):
    """`count` synthetic entries over `span` days from `first_day`, in nondecreasing day order."""
    days = sorted(first_day + rng.randint(0, span - 1) for _ in range(count))
    return [[rng.choice(["qaa", "qab"]), day, rng.randint(1, 120)] for day in days]


# --- the most-used course -------------------------------------------------------------------------


def most_used_cases(rng):
    cases = [
        ("none", {"entries": [], "today": MONDAY + 2}),
        (
            "this week",
            {"entries": [["qab", MONDAY, 40], ["qaa", MONDAY + 1, 30]], "today": MONDAY + 2},
        ),
        (
            "a tie this week",
            {"entries": [["qab", MONDAY, 30], ["qaa", MONDAY + 1, 30]], "today": MONDAY + 2},
        ),
        (
            "all time",
            {"entries": [["qab", MONDAY - 9, 20], ["qaa", MONDAY - 8, 50]], "today": MONDAY + 2},
        ),
        (
            "an all-time tie",
            {"entries": [["qab", MONDAY - 9, 25], ["qaa", MONDAY - 8, 25]], "today": MONDAY + 2},
        ),
        (
            "the week before the rollover",
            {"entries": [["qaa", MONDAY - 1, 90], ["qab", MONDAY, 10]], "today": MONDAY},
        ),
    ]
    for _ in range(12):
        entries = entries_of(rng, rng.randint(1, 6), MONDAY - 14, 21)
        cases.append((None, {"entries": entries, "today": entries[-1][1]}))
    return cases


def with_a_stub_log(most_used, predecessor, *, entries, today):
    store = StubReadingStore((code, day, minutes) for code, day, minutes in entries)
    stand_in = types.SimpleNamespace(_store=store, _today=lambda: as_date(today))
    return asyncio.run(most_used(stand_in))


# --- the settled reading XP -----------------------------------------------------------------------


def settled_cases(rng):
    cases = [
        ("one entry", {"entries": [["qaa", MONDAY + 2, 30]]}),
        ("a week of exactly 210", {"entries": [["qaa", MONDAY, 100], ["qaa", MONDAY + 3, 110]]}),
        ("a week of 209", {"entries": [["qaa", MONDAY, 100], ["qaa", MONDAY + 3, 109]]}),
        (
            "two weeks",
            {"entries": [["qaa", MONDAY - 1, 200], ["qaa", MONDAY, 20], ["qab", MONDAY + 6, 220]]},
        ),
        ("a day above the cap", {"entries": [["qab", MONDAY + 1, 100], ["qab", MONDAY + 1, 50]]}),
    ]
    for _ in range(16):
        entries = entries_of(rng, rng.randint(1, 7), MONDAY - 7, 14)
        cases.append((None, {"entries": entries}))
    return cases


def with_a_stub_ledger(recompute, predecessor, *, entries):
    store = StubReadingStore([])
    stand_in = types.SimpleNamespace(_store=store)
    for code, day, minutes in entries:
        store.rows.append((code, day, minutes))
        asyncio.run(recompute(stand_in, code, as_date(day)))
    return [[day, source, amount] for (day, source), amount in sorted(store.grants.items())]


FUNCTIONS = {
    "habit_reading_xp": {
        "kind": "function",
        "function": "habits.reading_xp",
        "cases": xp_cases,
    },
    "habit_course_token": {
        "kind": "adapter",
        "function": "curriculum.resolve_lang",
        "adapter": with_synthetic_courses,
        "note": "Replaces the course and alias tables with two synthetic courses, qaa (alias a) "
        "and qab (alias b), and returns the code the token resolves to, or none.",
        "cases": token_cases,
    },
    "habit_most_used": {
        "kind": "adapter",
        "function": "pipeline_layers.habits.HabitsLayer.most_used_reading_lang",
        "adapter": with_a_stub_log,
        "note": "Drives it on a stand-in layer over a stub store holding the case's entries, "
        "weekly sums grouped by code in code order and rows in the order they were logged, with "
        "the layer's day fixed at the case's day; returns the code, or none.",
        "cases": most_used_cases,
    },
    "habit_reading_settled": {
        "kind": "adapter",
        "function": "pipeline_layers.habits.HabitsLayer._recompute_reading_xp",
        "adapter": with_a_stub_ledger,
        "note": "Logs the case's entries one by one into a stub store and recomputes after each "
        "for its code and day; returns every grant the store holds at the end, as day, source "
        "and amount, sorted.",
        "cases": settled_cases,
    },
    "habits.constants": {
        "kind": "constants",
        "names": [
            "constants.READING_XP_PER_MIN",
            "constants.READING_XP_DAILY_CAP_PER_LANG",
            "constants.READING_MAX_ENTRY_MIN",
            "constants.READING_WEEKLY_GOAL_MIN",
            "constants.READING_GOAL_XP_BONUS",
            "bot._READ_PRESETS",
        ],
    },
}

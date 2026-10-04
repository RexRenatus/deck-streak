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

Part 078b's registrations: the writing day's XP, the writing streaks, the habit badges' conditions
and the constants they use.

* `with_done_codes` drives `habits.py:writing_day_xp` for the case's writing courses and the codes
  confirmed on the day, and returns each grant as its source and amount, sorted.
* `with_dated_rows` drives `habits.py:writing_streak` and `habits.py:writing_streaks_by_lang` over
  the case's confirmations, each a code and an epoch day, on the case's day, and returns the
  streak over every writing course and each course's own.
* `with_two_reading_courses` drives `habits.py:evaluate_habit_badges` on a context built from the
  case, with the predecessor's reading courses replaced by the two synthetic courses, and returns
  the badge keys in the order it gives them.

Every writing set these cases draw holds at least one course: the empty set is SPEC-078's own
departure, decided by its criteria A9 and A19b, never by a golden.

The case builders draw only from the `random.Random` the generator seeds. Every course is
synthetic, every day an epoch day number.
"""

import asyncio
import datetime as dt
import math
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


# --- the writing day's XP -------------------------------------------------------------------------

#: The synthetic writing courses, and a confirmed code outside every writing set.
WRITING = ["qaa", "qab", "qac"]
OUTSIDE = "qzz"


def writing_xp_cases(rng):
    cases = [
        ("one course, none confirmed", {"codes": ["qaa"], "done": []}),
        ("one course, confirmed", {"codes": ["qaa"], "done": ["qaa"]}),
        ("two courses, one confirmed", {"codes": ["qaa", "qab"], "done": ["qaa"]}),
        ("two courses, both confirmed", {"codes": ["qaa", "qab"], "done": ["qaa", "qab"]}),
        ("three courses, two confirmed", {"codes": WRITING, "done": ["qab", "qac"]}),
        ("three courses, all confirmed", {"codes": WRITING, "done": list(WRITING)}),
        ("a code outside the set", {"codes": ["qaa", "qab"], "done": ["qaa", "qab", OUTSIDE]}),
        ("only a code outside the set", {"codes": ["qaa"], "done": [OUTSIDE]}),
    ]
    for _ in range(16):
        codes = WRITING[: rng.randint(1, 3)]
        done = [code for code in [*codes, OUTSIDE] if rng.random() < 0.6]
        cases.append((None, {"codes": codes, "done": done}))
    return cases


def with_done_codes(writing_day_xp, predecessor, *, codes, done):
    grants = writing_day_xp(set(done), codes=codes)
    return [[source, amount] for source, amount in sorted(grants.items())]


# --- the writing streaks --------------------------------------------------------------------------


def every_course(codes, days):
    """A confirmation of each of `codes` on each of `days`, as code and epoch day."""
    return [[code, day] for day in days for code in codes]


def writing_streak_cases(rng):
    today = MONDAY + 3
    two = ["qaa", "qab"]
    cases = [
        ("none", {"codes": two, "rows": [], "today": today}),
        (
            "today and the two days before",
            {
                "codes": two,
                "rows": every_course(two, [today - 2, today - 1, today]),
                "today": today,
            },
        ),
        (
            "ending yesterday",
            {"codes": two, "rows": every_course(two, [today - 2, today - 1]), "today": today},
        ),
        ("a gap", {"codes": two, "rows": every_course(two, [today - 2, today]), "today": today}),
        (
            "a course missing today",
            {
                "codes": two,
                "rows": [*every_course(two, [today - 1]), ["qaa", today]],
                "today": today,
            },
        ),
        (
            "only a code outside the set",
            {"codes": two, "rows": every_course([OUTSIDE], [today - 1, today]), "today": today},
        ),
        (
            "one course over five days",
            {
                "codes": ["qaa"],
                "rows": every_course(["qaa"], range(today - 4, today + 1)),
                "today": today,
            },
        ),
        (
            "the day before yesterday only",
            {"codes": two, "rows": every_course(two, [today - 2]), "today": today},
        ),
    ]
    for _ in range(14):
        codes = WRITING[: rng.randint(1, 3)]
        rows = [
            [code, day]
            for day in range(today - 9, today + 1)
            for code in [*codes, OUTSIDE]
            if rng.random() < 0.75
        ]
        cases.append((None, {"codes": codes, "rows": rows, "today": today}))
    return cases


def with_dated_rows(writing_streak, predecessor, *, codes, rows, today):
    habits = predecessor("habits")
    dated = [(code, as_date(day)) for code, day in rows]
    return {
        "all": writing_streak(dated, as_date(today), codes=codes),
        "by_course": habits.writing_streaks_by_lang(dated, as_date(today), codes=codes),
    }


# --- the habit badges' conditions -----------------------------------------------------------------


def badge_cases(rng):
    def context(**changed):
        base = {
            "reading_entries": 0,
            "writing_entries": 0,
            "writing_all_streak": 0,
            "langs_read_this_week": 0,
            "week_total_min": 0,
            "all_langs_goal_met": False,
        }
        base.update(changed)
        return base

    cases = [
        ("nothing", context()),
        ("one reading entry", context(reading_entries=1)),
        ("one writing entry", context(writing_entries=1)),
        ("a streak of 6", context(writing_entries=6, writing_all_streak=6)),
        ("a streak of 7", context(writing_entries=7, writing_all_streak=7)),
        ("a streak of 29", context(writing_entries=29, writing_all_streak=29)),
        ("a streak of 30", context(writing_entries=30, writing_all_streak=30)),
        ("a streak of 99", context(writing_entries=99, writing_all_streak=99)),
        ("a streak of 100", context(writing_entries=100, writing_all_streak=100)),
        ("one course read", context(reading_entries=2, langs_read_this_week=1, week_total_min=40)),
        (
            "both courses read",
            context(reading_entries=2, langs_read_this_week=2, week_total_min=40),
        ),
        ("599 minutes", context(reading_entries=3, langs_read_this_week=1, week_total_min=599)),
        ("600 minutes", context(reading_entries=3, langs_read_this_week=1, week_total_min=600)),
        (
            "every goal met",
            context(
                reading_entries=4,
                langs_read_this_week=2,
                week_total_min=420,
                all_langs_goal_met=True,
            ),
        ),
    ]
    # Each random context is one a log can hold, so the fold's test can build it from entries and
    # confirmations (A19): every goal met needs both courses at the weekly goal, and the week's
    # minutes need the entries that carry them, at most 600 minutes an entry.
    for _ in range(16):
        streak = rng.choice([0, 1, 6, 7, 8, 29, 30, 31, 99, 100, 120])
        langs = rng.randint(0, 2)
        goal = langs == 2 and rng.random() < 0.5
        minutes = 0 if langs == 0 else rng.randint(420 if goal else langs, 900)
        cases.append(
            (
                None,
                context(
                    reading_entries=(
                        rng.randint(required_entries(langs, goal, minutes), 30)
                        if langs
                        else rng.randint(0, 3)
                    ),
                    writing_entries=streak + rng.randint(0, 5),
                    writing_all_streak=streak,
                    langs_read_this_week=langs,
                    week_total_min=minutes,
                    all_langs_goal_met=goal,
                ),
            )
        )
    return cases


def required_entries(langs, goal, minutes):
    """The fewest entries that log `minutes` over `langs` courses in one week, at most 600 minutes
    an entry: one course carries them all; both at the goal split them evenly, at most 450 each;
    both short of it leave one minute to the second course."""
    if langs == 1:
        return math.ceil(minutes / 600)
    if goal:
        return 2
    return 1 + math.ceil((minutes - 1) / 600)


def with_two_reading_courses(evaluate, predecessor, **context):
    habits = predecessor("habits")
    with mock.patch.object(habits, "READING_LANG_CODES", ("qaa", "qab")):
        return evaluate(habits.HabitBadgeContext(**context))


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
            "constants.WRITING_XP_PER_DAY",
            "constants.WRITING_XP_ALL_THREE_BONUS",
            "constants.WRITING_STREAK_WEEK",
            "constants.WRITING_STREAK_MONTH",
            "constants.WRITING_STREAK_CENTURY",
            "constants.MARATHON_READER_WEEK_MIN",
        ],
    },
    "habit_writing_xp": {
        "kind": "adapter",
        "function": "habits.writing_day_xp",
        "adapter": with_done_codes,
        "note": "Calls it with the case's writing courses and the codes confirmed on the day; "
        "returns each grant as its source and amount, sorted.",
        "cases": writing_xp_cases,
    },
    "habit_writing_streak": {
        "kind": "adapter",
        "function": "habits.writing_streak",
        "adapter": with_dated_rows,
        "note": "Calls it and writing_streaks_by_lang over the case's confirmations, each a code "
        "and an epoch day, on the case's day; returns the streak over every writing course and "
        "each course's own.",
        "cases": writing_streak_cases,
    },
    "habit_badges": {
        "kind": "adapter",
        "function": "habits.evaluate_habit_badges",
        "adapter": with_two_reading_courses,
        "note": "Builds the context from the case with the reading courses replaced by two "
        "synthetic courses; returns the badge keys in the order it gives them.",
        "cases": badge_cases,
    },
}

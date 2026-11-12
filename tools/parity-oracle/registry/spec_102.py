"""SPEC-102's registrations: the historical landmarks, their texts and the constants they use.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_synthetic_reviews` drives `landmarks.py:compute_landmarks` and `landmarks.py:due_today`.
  The case holds `study_days`, the epoch day numbers of the study days the port is fed, and
  `today`, the day evaluated, as an epoch day number. The adapter builds one or more synthetic
  predecessor `Review`s on each study day at an instant drawn from the case, a `Review` that is not
  a study event on each of `non_study_days`, and the predecessor's default `CollectionConfig`. It
  refuses a case whose built reviews do not fall on the case's study days by the predecessor's own
  `analytics.py:study_day`, and one whose non-study reviews the predecessor counts. The output is
  the landmarks, each with its key, its event, its ordinal and its day as an epoch day number, and
  the keys `due_today` selects.
* `with_a_landmark` drives `landmarks.py:render_landmark`, which calls
  `landmarks.py:_ordinal_label`, on a predecessor `Landmark` of the case's event, ordinal and day,
  rendered plain and with `gap_honest`. A golden may hold no calendar date, so each text writes its
  date as the token `{day:N}`, N its epoch day number (README.md, "A day inside a text"): `contract`
  here is a copy of `spec_042.py`'s, because a SPEC never imports another SPEC's module.
* `with_a_stub_landmark_store` drives `landmarks.py:run_landmarks`, the predecessor's whole run,
  on a stand-in store: the case's `mark`, when it holds one, is the stored `landmark_high_water`,
  the run's offload is patched to return the reviews `with_synthetic_reviews` builds on the case's
  study days with the default `CollectionConfig`, the streak state's last study day is the latest
  case study day at or before `today`, and a recording celebrate answers every landmark as sent.
  The output is the mark the run stored (`None` when it stored none), each landmark it celebrated
  as its event, its key and its contracted text, in order, and the count it returned.

The case builders draw only from the `random.Random` the generator seeds. Every day is an epoch day
number and every value is synthetic.
"""

import asyncio
import re
import types
from datetime import date
from unittest import mock

#: The ordinal of the Unix epoch's day, so a `date` and its epoch day number convert exactly.
EPOCH_ORDINAL = date(1970, 1, 1).toordinal()
#: An ISO calendar date inside the text the predecessor returns.
ISO_DAY = re.compile(r"(?<![0-9])[0-9]{4}-[0-9]{2}-[0-9]{2}(?![0-9])")
ANNIVERSARY = "landmark_anniversary"
STUDY_DAY = "landmark_study_day"
LANDMARK_HIGH_WATER = "landmark_high_water"
SECONDS_PER_DAY = 86_400


def epoch(year, month, day):
    return date(year, month, day).toordinal() - EPOCH_ORDINAL


def as_date(epoch_day):
    return date.fromordinal(EPOCH_ORDINAL + epoch_day)


def contract(text):
    """`text` with every ISO date written as its `{day:N}` token; a date that does not read back
    as itself is refused, so the token always names exactly the date the predecessor wrote."""

    def token(match):
        day = date.fromisoformat(match[0])
        if day.isoformat() != match[0]:
            raise ValueError(f"{match[0]!r} is not an ISO date")
        return "{day:%d}" % (day.toordinal() - EPOCH_ORDINAL)

    return ISO_DAY.sub(token, text)


def synthetic_reviews(predecessor, config, study_days, non_study_days):
    """The predecessor `Review`s on each study day and the reviews that are not study events."""
    review = predecessor("types.Review")
    study_day_of = predecessor("analytics.study_day")
    reviews = []
    for position, day in enumerate(study_days):
        for second in (4 * 3600 + (position * 977) % 3600, 27 * 3600 + 3599 - (position % 7)):
            if second >= 28 * 3600 or (position % 2 == 1 and second > 24 * 3600):
                continue
            instant = (day * SECONDS_PER_DAY + second) * 1000
            if as_date(day) != study_day_of(instant, config):
                raise ValueError(f"the review at {instant} is not on study day {day}")
            reviews.append(review(instant, 1, 3, 1, 0, 2500, 1000, 1))
    for day in non_study_days:
        for rtype, ease in ((4, 0), (5, 0), (1, 0)):
            instant = (day * SECONDS_PER_DAY + 12 * 3600) * 1000
            reviews.append(review(instant, 1, ease, 1, 0, 2500, 1000, rtype))
    if any(r.is_study_event for r in reviews if r.rtype in (4, 5) or r.ease == 0):
        raise ValueError("a review built as not a study event is one")
    return reviews


def with_synthetic_reviews(compute_landmarks, predecessor, *, study_days, today, non_study_days):
    config = predecessor("types.CollectionConfig")()
    reviews = synthetic_reviews(predecessor, config, study_days, non_study_days)
    found = compute_landmarks(reviews, config, today=as_date(today))
    due = predecessor("landmarks.due_today")(found, as_date(today))
    return {
        "landmarks": [
            {
                "key": item.event_key,
                "event": item.event_type,
                "ordinal": item.ordinal,
                "day": item.day,
            }
            for item in found
        ],
        "due": [item.event_key for item in due],
    }


def with_a_landmark(render_landmark, predecessor, *, event, ordinal, day):
    kind = {ANNIVERSARY: "anniv", STUDY_DAY: "day"}[event]
    item = predecessor("landmarks.Landmark")(
        f"landmark:{kind}:{ordinal}", event, ordinal, as_date(day)
    )
    return {
        "plain": contract(render_landmark(item, gap_honest=False)),
        "gap_honest": contract(render_landmark(item, gap_honest=True)),
    }


def run(first, count, step=1):
    return [first + step * index for index in range(count)]


def landmark_cases(rng):
    origin = epoch(2001, 1, 1)
    leap = epoch(2000, 2, 29)
    first_anniversary = epoch(2001, 2, 28)
    cases = [
        ("no-study-day", {"study_days": [], "today": origin, "non_study_days": []}),
        (
            "no-study-day",
            {
                "study_days": [],
                "today": origin,
                "non_study_days": [origin - 3, origin, origin + 40],
            },
        ),
        ("one-day", {"study_days": [origin], "today": origin, "non_study_days": []}),
        ("one-day", {"study_days": [origin], "today": origin + 364, "non_study_days": []}),
        (
            "anniversary-on-the-day",
            {"study_days": [origin], "today": origin + 365, "non_study_days": []},
        ),
        (
            "anniversary-the-day-after",
            {"study_days": [origin], "today": origin + 364, "non_study_days": []},
        ),
        (
            "anniversary-kept-after",
            {"study_days": [origin], "today": origin + 366, "non_study_days": []},
        ),
    ]
    for offset in (-2, -1, 0, 1, 2):
        cases.append(
            (
                "leap-day-origin",
                {"study_days": [leap], "today": first_anniversary + offset, "non_study_days": []},
            )
        )
    for offset in (-1, 0, 1):
        cases.append(
            (
                "leap-day-origin-is-the-29th-again",
                {"study_days": [leap], "today": epoch(2004, 2, 29) + offset, "non_study_days": []},
            )
        )
    cases.append(
        (
            "leap-day-origin-many-years",
            {"study_days": [leap, leap + 400], "today": epoch(2012, 3, 1), "non_study_days": []},
        )
    )
    cases.append(
        (
            "century-leap-year",
            {"study_days": [epoch(1999, 2, 28)], "today": epoch(2001, 3, 1), "non_study_days": []},
        )
    )
    for count in (24, 25, 26, 49, 50, 51, 75, 100):
        days = run(origin, count)
        cases.append(
            (f"study-day-{count}", {"study_days": days, "today": days[-1], "non_study_days": []})
        )
    days = run(origin, 25)
    cases.append(
        ("25th-before-today", {"study_days": days, "today": days[-1] - 1, "non_study_days": []})
    )
    cases.append(
        ("25th-after-the-last", {"study_days": days, "today": days[-1] + 30, "non_study_days": []})
    )
    days = run(origin, 24) + [origin + 365]
    cases.append(
        (
            "anniversary-and-25th-on-one-day",
            {"study_days": days, "today": origin + 365, "non_study_days": []},
        )
    )
    cases.append(
        (
            "anniversary-and-25th-on-one-day",
            {"study_days": days, "today": origin + 366, "non_study_days": []},
        )
    )
    cases.append(
        (
            "25th-on-the-day-after-an-anniversary",
            {
                "study_days": run(origin, 24) + [origin + 366],
                "today": origin + 366,
                "non_study_days": [],
            },
        )
    )
    days = run(origin, 100, 10)
    cases.append(("many-years", {"study_days": days, "today": days[-1], "non_study_days": []}))
    cases.append(("many-years", {"study_days": days, "today": days[50], "non_study_days": []}))
    days = run(origin, 30)
    cases.append(
        (
            "not-study-events",
            {
                "study_days": days,
                "today": days[-1],
                "non_study_days": [
                    origin - 400,
                    origin - 1,
                    origin + 5,
                    days[-1] + 1,
                    origin + 800,
                ],
            },
        )
    )
    cases.append(
        (
            "not-study-events-before-the-origin",
            {"study_days": [origin + 10], "today": origin + 10 + 365, "non_study_days": [origin]},
        )
    )
    cases.append(
        ("today-before-the-origin", {"study_days": days, "today": origin - 5, "non_study_days": []})
    )
    for _ in range(10):
        count = rng.randint(1, 120)
        start = rng.randint(epoch(1996, 1, 1), epoch(2030, 12, 31))
        days = sorted({start + rng.randint(0, 3 * 365) for _ in range(count)})
        today = rng.randint(days[0] - 10, days[-1] + 400)
        cases.append((None, {"study_days": days, "today": today, "non_study_days": []}))
    return cases


class LandmarkStore:
    """The settings and the streak state `run_landmarks` reads, and every setting it stores."""

    def __init__(self, mark, last_study_day):
        self.settings = {} if mark is None else {LANDMARK_HIGH_WATER: mark}
        self.stored = []
        self.last_study_day = last_study_day

    async def get_setting(self, key):
        return self.settings.get(key)

    async def set_setting(self, key, value):
        self.settings[key] = value
        self.stored.append((key, value))

    async def get_streak_state(self):
        return types.SimpleNamespace(last_study_day=self.last_study_day)


def with_a_stub_landmark_store(run_landmarks, predecessor, *, study_days, today, mark):
    """Run the predecessor's whole run on a stand-in store whose offload returns synthetic reviews."""
    landmarks = predecessor("landmarks")
    config = predecessor("types.CollectionConfig")()
    reviews = synthetic_reviews(predecessor, config, study_days, [])
    studied = [day for day in study_days if day <= today]
    store = LandmarkStore(mark, as_date(max(studied)) if studied else None)
    celebrated = []

    async def read_offloaded(name, function, *args, **kwargs):
        return reviews, config

    async def celebrate(*, event_type, event_key, text, rarity):
        celebrated.append([event_type, event_key, contract(text)])
        return True

    settings = types.SimpleNamespace(rollover_hour=4, tz_offset_minutes=0, leech_threshold=8)
    with mock.patch.object(landmarks.offload, "run_offloaded", read_offloaded):
        delivered = asyncio.run(
            run_landmarks(
                collection_path="collection.anki2",
                store=store,
                settings=settings,
                today=as_date(today),
                celebrate=celebrate,
            )
        )
    written = [value for key, value in store.stored if key == LANDMARK_HIGH_WATER]
    return {
        "mark_written": written[0] if written else None,
        "celebrated": celebrated,
        "delivered": delivered,
    }


#: A mark an earlier run of the predecessor stored, as a case's stored mark.
EARLIER_MARK = '{"anniversary": 0, "seeded": true, "study_day": 0}'


def run_case(study_days, today, mark=None):
    return {"study_days": study_days, "today": today, "mark": mark}


def landmark_run_cases(rng):
    """No mark with none, one and two due; a stored mark with two due; the anniversary on a day
    studied and on a day not studied; then seeded draws."""
    origin = epoch(2023, 10, 4)
    anniversary = epoch(2024, 10, 4)
    # The origin, 23 days after it and the anniversary: the 25th study day is the anniversary's.
    both = [origin, *run(origin + 316, 23), anniversary]
    cases = [
        ("no-mark-none-due", run_case([origin, origin + 3], anniversary - 1)),
        ("no-mark-one-due", run_case([origin, anniversary], anniversary)),
        ("no-mark-two-due", run_case(both, anniversary)),
        ("mark-two-due", run_case(both, anniversary, EARLIER_MARK)),
        ("studied-today", run_case([origin, origin + 40, anniversary], anniversary)),
        ("not-studied-today", run_case([origin, origin + 40], anniversary)),
    ]
    for _ in range(4):
        start = rng.randint(epoch(2018, 1, 1), epoch(2026, 12, 31))
        today = start + rng.randint(300, 800)
        count = rng.randint(1, 50)
        days = sorted({start} | {start + rng.randint(1, today - start) for _ in range(count)})
        mark = EARLIER_MARK if rng.random() < 0.3 else None
        cases.append((None, run_case(days, today, mark)))
    return cases


ORDINALS = (1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 20, 21, 22, 23, 24, 25, 50, 100, 101, 102, 103)
ORDINALS += (111, 112, 113, 121, 122, 123, 200, 211, 212, 213)


def text_cases(rng):
    day = epoch(2024, 5, 17)
    cases = []
    for event in (ANNIVERSARY, STUDY_DAY):
        for ordinal in ORDINALS:
            teen = ordinal % 100 in (11, 12, 13)
            cases.append(
                (
                    f"ordinal-teen-{ordinal}" if teen else f"ordinal-{ordinal}",
                    {"event": event, "ordinal": ordinal, "day": day},
                )
            )
    for event in (ANNIVERSARY, STUDY_DAY):
        cases.append(("leap-day", {"event": event, "ordinal": 4, "day": epoch(2024, 2, 29)}))
        cases.append(("early-day", {"event": event, "ordinal": 25, "day": epoch(1999, 1, 1)}))
    for _ in range(6):
        cases.append(
            (
                None,
                {
                    "event": rng.choice([ANNIVERSARY, STUDY_DAY]),
                    "ordinal": rng.randint(1, 400),
                    "day": rng.randint(epoch(1996, 1, 1), epoch(2040, 12, 31)),
                },
            )
        )
    return cases


FUNCTIONS = {
    "landmarks": {
        "kind": "adapter",
        "function": "landmarks.compute_landmarks",
        "adapter": with_synthetic_reviews,
        "note": (
            "Builds synthetic predecessor Reviews on each case study day and Reviews that are "
            "not study events on the case's other days, with the default CollectionConfig, "
            "checks each by the predecessor's own study_day, calls compute_landmarks at the "
            "case's day and due_today at the same day; returns the landmarks and the due keys."
        ),
        "cases": landmark_cases,
    },
    "landmark_text": {
        "kind": "adapter",
        "function": "landmarks.render_landmark",
        "adapter": with_a_landmark,
        "note": (
            "Builds a predecessor Landmark of the case's event, ordinal and day, renders it "
            "plain and gap_honest, and writes each text's date as its day token."
        ),
        "cases": text_cases,
    },
    "landmarks_run": {
        "kind": "adapter",
        "function": "landmarks.run_landmarks",
        "adapter": with_a_stub_landmark_store,
        "note": (
            "Runs run_landmarks on a stand-in store holding the case's mark, its offload patched "
            "to return synthetic Reviews on the case's study days with the default "
            "CollectionConfig, the streak's last study day the latest case day at or before "
            "today, and a recording celebrate that answers sent; returns the mark stored, the "
            "celebrated events, keys and contracted texts in order, and the count delivered."
        ),
        "cases": landmark_run_cases,
    },
    "landmarks.constants": {
        "kind": "constants",
        "names": [
            "landmarks.LANDMARK_DAY_STEP",
            "landmarks.LANDMARK_HIGH_WATER_KEY",
            "landmarks.ANNIVERSARY_EVENT_TYPE",
            "landmarks.STUDY_DAY_EVENT_TYPE",
            "constants.__LANDMARK_ANNIVERSARY_TEMPLATE",
            "constants.__LANDMARK_ANNIVERSARY_GAP_TEMPLATE",
            "constants.__LANDMARK_STUDY_DAY_TEMPLATE",
        ],
    },
}

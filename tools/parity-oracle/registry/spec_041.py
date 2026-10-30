"""SPEC-041's registration: whether a minute of the local day falls in the quiet window.

`quiet_hours.in_quiet_hours` takes the minute of the local day and the window's start and end, as
minutes of the day. The window is `[start, end)`, wraps midnight when the start is later than the
end, and is disabled when the two are equal; the minute is first taken modulo a day, so a minute
before 0 or past 1439 names a minute of the day before or after. The case builder asks on both sides
of each boundary of the policy's default window (23:00 to 07:30), of another wrapping window and of
a same-day window; inside and outside two disabled windows; at minutes outside 0 to 1439, which the
modulo brings back; and at seeded minutes and windows. Every output is what the predecessor's
function returned.
"""

#: The policy's default window, 23:00 to 07:30, as minutes of the day.
DEFAULT_WINDOW = (23 * 60, 7 * 60 + 30)

#: A window that wraps midnight other than the default, 22:00 to 06:00.
OTHER_WRAPPING_WINDOW = (22 * 60, 6 * 60)

#: A window inside one day, 01:00 to 05:00.
SAME_DAY_WINDOW = (60, 5 * 60)

#: Two disabled windows: a start equal to its end.
DISABLED_WINDOWS = ((10 * 60, 10 * 60), (0, 0))

#: One day, in minutes.
DAY = 24 * 60


def edges(start, end):
    """The minutes on both sides of each boundary of a window, and the ends of the day."""
    return sorted({start - 1, start, start + 1, end - 1, end, end + 1, 0, DAY - 1})


def ask(found, klass, start, end, minutes):
    for minute in minutes:
        found.append((klass, {"local_minutes": minute, "start_min": start, "end_min": end}))


def cases(rng):
    found = []
    start, end = DEFAULT_WINDOW
    ask(found, "wrap", start, end, edges(start, end) + [12 * 60])
    ask(
        found,
        "normalized",
        start,
        end,
        [-1, -60, DAY, DAY + end - 1, 2 * DAY + start, -DAY + start - 1],
    )
    start, end = OTHER_WRAPPING_WINDOW
    ask(found, "wrap", start, end, edges(start, end))
    start, end = SAME_DAY_WINDOW
    ask(found, "same-day", start, end, edges(start, end))
    for start, end in DISABLED_WINDOWS:
        ask(found, "disabled", start, end, [start - 1, start, start + 1, DAY - 1])
    for _ in range(24):
        start, end = rng.randrange(DAY), rng.randrange(DAY)
        ask(found, None, start, end, [rng.randrange(-DAY, 3 * DAY)])
    return found


FUNCTIONS = {
    "in_quiet_hours": {
        "kind": "function",
        "function": "quiet_hours.in_quiet_hours",
        "cases": cases,
    },
}

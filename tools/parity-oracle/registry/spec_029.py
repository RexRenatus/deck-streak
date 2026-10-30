"""SPEC-029's registration: the study day, the rule every other golden stands on.

The adapter builds the one argument JSON cannot carry, the predecessor's `CollectionConfig`
(`types.py:CollectionConfig`), and calls `analytics.py:study_day` with it. It computes nothing:
the output is the predecessor's `date`, which the generator writes as its epoch day number.

The case builder draws only from the `random.Random` the generator seeds, and covers R9's
boundaries: one millisecond before and exactly at the rollover for each hour and offset R9 names,
instants before the Unix epoch, and offsets that move the local day off the UTC day.
"""

DAY_MS = 86_400_000
HOUR_MS = 3_600_000
MINUTE_MS = 60_000
#: R9's rollover hours: midnight, the predecessor's default, and the last hour of the day.
HOURS = (0, 4, 23)
#: R9's UTC offsets in minutes: both bounds of the setting, zero, and a half-hour zone.
OFFSETS = (-720, 0, 330, 840)


def with_collection_config(
    study_day, predecessor, *, instant_ms, rollover_hour, utc_offset_minutes
):
    """Build the predecessor's CollectionConfig from the case, and pass the instant through."""
    config = predecessor("types.CollectionConfig")(
        rollover_hour=rollover_hour, tz_offset_minutes=utc_offset_minutes
    )
    return study_day(instant_ms, config)


def case(instant_ms, rollover_hour, utc_offset_minutes):
    return {
        "instant_ms": instant_ms,
        "rollover_hour": rollover_hour,
        "utc_offset_minutes": utc_offset_minutes,
    }


def cases(rng):
    """24 rollover, 6 negative and 4 offset cases, then 16 ordinary ones: 50 in all."""
    drawn = []
    # One millisecond before, and exactly at, the local rollover of a seeded local day.
    for hour in HOURS:
        for offset in OFFSETS:
            local_day = rng.randrange(0, 40_000)
            at = local_day * DAY_MS + hour * HOUR_MS - offset * MINUTE_MS
            drawn.append(("rollover", case(at - 1, hour, offset)))
            drawn.append(("rollover", case(at, hour, offset)))
    # Instants before the Unix epoch, where a division that truncates gets the day wrong.
    for instant in (-1, -DAY_MS, -DAY_MS - 1):
        drawn.append(("negative", case(instant, 0, 0)))
    for _ in range(3):
        instant = -rng.randrange(1, 10**12)
        drawn.append(
            ("negative", case(instant, rng.choice(HOURS), rng.choice(OFFSETS)))
        )
    # Offsets that move the local day off the UTC day, with no rollover to blur it: the last
    # millisecond of a UTC day seen fourteen hours east, its first seen twelve hours west, and a
    # seeded UTC day's evening and morning seen five and a half hours east and twelve west.
    drawn.append(("offset", case(DAY_MS - 1, 0, 840)))
    drawn.append(("offset", case(0, 0, -720)))
    utc_day = rng.randrange(0, 40_000)
    drawn.append(("offset", case(utc_day * DAY_MS + 20 * HOUR_MS, 0, 330)))
    drawn.append(("offset", case(utc_day * DAY_MS + 6 * HOUR_MS, 0, -720)))
    # Ordinary instants: any hour, and any whole-minute offset within the setting's bounds.
    for _ in range(16):
        instant = rng.randrange(0, 4 * 10**12)
        drawn.append(
            (None, case(instant, rng.randrange(0, 24), rng.randrange(-720, 841)))
        )
    return drawn


FUNCTIONS = {
    "study_day": {
        "kind": "adapter",
        "function": "analytics.study_day",
        "adapter": with_collection_config,
        "note": (
            "Builds the predecessor's CollectionConfig from rollover_hour and "
            "utc_offset_minutes, and passes instant_ms through unchanged."
        ),
        "cases": cases,
    },
}

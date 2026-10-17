"""SPEC-075's registrations: the personal board, the XP exchange readout, the bucket of a source
and the readout's window.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_a_board_store` drives `pipeline_layers/read_api.py:ReadApiLayer.leaderboard` on a stand-in
  layer over a stub store holding the case's rollups, streak and XP total, with the layer's own
  `streak_state` and `level` bound to it. The stub answers `get_recent_rollups(n)` with the `n`
  most recent rollups, so the number of rollups read comes from the predecessor's code; it records
  that `n`. It returns the `n` and each board row, a day turned into its epoch day number.
* `with_a_ledger` drives `exchange.py:exchange_rates` on a temporary store the adapter fills with
  the case's synthetic XP rows and rollups, each day turned from its epoch day number into the
  store's form, and returns each source rate's fields.
* `with_a_server` builds the predecessor's MCP server over a stand-in pipeline whose
  `xp_exchange_rates` records the window it is asked for, calls the `get_xp_exchange_rates` tool
  with the case's `days`, and returns that window's first and last day, or none.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic, every day an epoch day number.
"""

import asyncio
import datetime as dt
import re
import tempfile
import types
from pathlib import Path

#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: A study day number near the present.
PRESENT_DAY = 20_000
#: A board row's detail that is a day, as the predecessor writes one.
ISO_DAY = re.compile(r"\d{4}-\d{2}-\d{2}")
#: The rollups the case builders place beyond the read, as they aim at it. The golden's `n` comes
#: from the predecessor's code, never from this.
AIMED_READ = 365


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


def as_day(value):
    """The epoch day number of the predecessor's ISO day text `value`."""
    return dt.date.fromisoformat(value).toordinal() - EPOCH_ORDINAL


# --- the personal board ---------------------------------------------------------------------------


class StubBoardStore:
    """Holds the case's rollups, streak and XP total, and answers the reads the board makes."""

    def __init__(self, rollups, streak, total_xp, streak_type):
        self.rollups = [{"day": as_date(day).isoformat(), "score": score} for day, score in rollups]
        self.streak = streak
        self.total_xp = total_xp
        self.streak_type = streak_type
        self.limits = []

    async def get_recent_rollups(self, limit):
        self.limits.append(limit)
        return sorted(self.rollups, key=lambda row: row["day"], reverse=True)[:limit]

    async def get_streak_state(self):
        if self.streak is None:
            return self.streak_type(0, 0, 1, None, False)
        current, longest = self.streak
        return self.streak_type(current, longest, 1, None, False)

    async def get_total_xp(self, track=None):
        return self.total_xp


def spread(rng, today, count, gap):
    """`count` distinct days on or before `today`, each at most `gap` days before the next."""
    days, day = [], today - rng.randrange(gap)
    for _ in range(count):
        days.append(day)
        day -= 1 + rng.randrange(gap)
    return days


def board_cases(rng):
    found = []
    today = PRESENT_DAY
    # Beyond the read: the 365th most recent rollup holds the best score the read can see, and
    # the 366th a higher one it cannot.
    days = [today - i for i in range(AIMED_READ + 5)]
    scores = [rng.randrange(0, 80) for _ in days]
    scores[AIMED_READ - 1] = 95
    scores[AIMED_READ] = 100
    found.append(
        (
            "beyond-the-read",
            {
                "today": today,
                "rollups": [[d, s] for d, s in zip(days, scores)],
                "streak": [12, 40],
                "total_xp": 91_255,
            },
        )
    )
    # Only the 366th most recent rollup, on spread days, holds the maximum.
    days = spread(rng, today, AIMED_READ + 3, 3)
    scores = [rng.randrange(0, 90) for _ in days]
    scores[AIMED_READ] = 100
    found.append(
        (
            "max-beyond-only",
            {
                "today": today,
                "rollups": [[d, s] for d, s in zip(days, scores)],
                "streak": [0, 7],
                "total_xp": 1_250,
            },
        )
    )
    # Ties: three days share the maximum, none of them today.
    days = [today - i for i in range(20)]
    scores = [rng.randrange(0, 70) for _ in days]
    for i in (4, 9, 15):
        scores[i] = 88
    found.append(
        (
            "tie",
            {
                "today": today,
                "rollups": [[d, s] for d, s in zip(days, scores)],
                "streak": [3, 9],
                "total_xp": 4_000,
            },
        )
    )
    # Today ties the oldest maximum.
    days = [today - i for i in range(10)]
    scores = [rng.randrange(0, 60) for _ in days]
    scores[0] = 77
    scores[8] = 77
    found.append(
        (
            "tie-with-today",
            {
                "today": today,
                "rollups": [[d, s] for d, s in zip(days, scores)],
                "streak": [10, 10],
                "total_xp": 25_000,
            },
        )
    )
    found.append(("no-rollup", {"today": today, "rollups": [], "streak": [0, 5], "total_xp": 0}))
    found.append(
        ("no-rollup-no-streak", {"today": today, "rollups": [], "streak": None, "total_xp": 99})
    )
    days = [today - 3 - i for i in range(6)]
    found.append(
        (
            "no-rollup-today",
            {
                "today": today,
                "rollups": [[d, rng.randrange(0, 101)] for d in days],
                "streak": [0, 2],
                "total_xp": 310,
            },
        )
    )
    found.append(
        (
            "only-today",
            {"today": today, "rollups": [[today, 0]], "streak": [1, 1], "total_xp": 15},
        )
    )
    for _ in range(6):
        count = rng.randrange(1, 40)
        longest = rng.randrange(0, 60)
        found.append(
            (
                None,
                {
                    "today": today,
                    "rollups": [[d, rng.randrange(0, 101)] for d in spread(rng, today, count, 4)],
                    "streak": [rng.randrange(0, longest + 1), longest],
                    "total_xp": rng.randrange(0, 200_000),
                },
            )
        )
    return found


def with_a_board_store(leaderboard, predecessor, *, today, rollups, streak, total_xp):
    layer = predecessor("pipeline_layers.read_api.ReadApiLayer")
    store = StubBoardStore(rollups, streak, total_xp, predecessor("types.StreakState"))
    stand_in = types.SimpleNamespace(_store=store, _today=lambda: as_date(today))
    for name in ("streak_state", "level", "_refresh_streak_heat"):
        setattr(stand_in, name, types.MethodType(getattr(layer, name), stand_in))
    board = asyncio.run(leaderboard(stand_in))
    return {
        "limits": store.limits,
        "board": [
            {
                "label": row["label"],
                "value": row["value"],
                "detail": as_day(row["detail"])
                if ISO_DAY.fullmatch(str(row["detail"]))
                else row["detail"],
            }
            for row in board
        ],
    }


# --- the exchange readout -------------------------------------------------------------------------

#: Sources the predecessor's ledger writes, literal and prefixed, plus shapes the bucket must place.
SOURCES = (
    "reviews",
    "reviews_law",
    "consistency",
    "focus",
    "streak",
    "graduations",
    "quest:1",
    "quest:2",
    "chest:7",
    "focusgoal:law:3",
    "read:es",
    "2x:reviews",
    ":bare",
)


def exchange_cases(rng):
    found = []
    day = PRESENT_DAY
    found.append(("empty", {"rows": [], "rollups": []}))
    found.append(
        (
            "one-bucket-one-day",
            {
                "rows": [
                    {"day": day, "source": "quest:1", "amount": 30},
                    {"day": day, "source": "quest:2", "amount": 45},
                    {"day": day, "source": "quest:3", "amount": 25},
                ],
                "rollups": [{"day": day, "graduations": 4}],
            },
        )
    )
    found.append(
        (
            "a-day-with-no-rollup",
            {
                "rows": [
                    {"day": day - 1, "source": "reviews", "amount": 120},
                    {"day": day, "source": "reviews", "amount": 80},
                ],
                "rollups": [{"day": day - 1, "graduations": 3}],
            },
        )
    )
    found.append(
        (
            "xp-and-no-graduation",
            {
                "rows": [
                    {"day": day - 2, "source": "focus", "amount": 50},
                    {"day": day - 3, "source": "focus", "amount": 60},
                    {"day": day, "source": "reviews", "amount": 7},
                ],
                "rollups": [{"day": day - 2, "graduations": 0}, {"day": day, "graduations": 2}],
            },
        )
    )
    found.append(
        (
            "graduations-and-no-xp",
            {
                "rows": [
                    {"day": day, "source": "freespin", "amount": 0},
                    {"day": day - 1, "source": "freespin", "amount": 0},
                ],
                "rollups": [{"day": day, "graduations": 5}, {"day": day - 1, "graduations": 1}],
            },
        )
    )
    found.append(
        (
            "a-third",
            {
                "rows": [{"day": day, "source": "consistency", "amount": 100}],
                "rollups": [{"day": day, "graduations": 3}],
            },
        )
    )
    for _ in range(8):
        span = rng.randrange(1, 30)
        rows, taken = [], set()
        for _ in range(rng.randrange(1, 25)):
            key = (day - rng.randrange(span), SOURCES[rng.randrange(len(SOURCES))])
            if key in taken:
                continue
            taken.add(key)
            rows.append({"day": key[0], "source": key[1], "amount": rng.randrange(0, 500)})
        rollups = [
            {"day": day - offset, "graduations": rng.randrange(0, 13)}
            for offset in range(span)
            if rng.randrange(4)
        ]
        found.append((None, {"rows": rows, "rollups": rollups}))
    return found


def with_a_ledger(exchange_rates, predecessor, *, rows, rollups):
    store_type = predecessor("database.GamifyStore")

    async def run():
        with tempfile.TemporaryDirectory() as directory:
            store = await store_type(Path(directory) / "store.db").connect()
            try:
                for row in rows:
                    await store.conn.execute(
                        "INSERT INTO xp_ledger(day, source, amount, track) VALUES (?,?,?,?)",
                        (as_date(row["day"]).isoformat(), row["source"], row["amount"], "language"),
                    )
                for rollup in rollups:
                    await store.conn.execute(
                        "INSERT INTO daily_rollup(day, graduations) VALUES (?,?)",
                        (as_date(rollup["day"]).isoformat(), rollup["graduations"]),
                    )
                await store.conn.commit()
                return await exchange_rates(store)
            finally:
                await store.close()

    return [
        {
            "source": rate.source,
            "total_xp": rate.total_xp,
            "graduated_cards": rate.graduated_cards,
            "rate": rate.rate,
            "rate_defined": rate.rate_defined,
        }
        for rate in asyncio.run(run())
    ]


# --- the bucket of a source -----------------------------------------------------------------------


def normalize_cases(rng):
    found = [
        ("literal", {"source": "reviews"}),
        ("literal", {"source": "reviews_law"}),
        ("prefixed", {"source": "quest:2"}),
        ("prefixed", {"source": "chest:"}),
        ("several-colons", {"source": "focusgoal:law:x"}),
        ("several-colons", {"source": "a:b:c:d:e"}),
        ("empty-prefix", {"source": ":foo"}),
        ("lone-colon", {"source": ":"}),
        ("empty", {"source": ""}),
        ("case-kept", {"source": "UPPER:Case"}),
    ]
    for _ in range(6):
        found.append((None, {"source": SOURCES[rng.randrange(len(SOURCES))]}))
    return found


# --- the readout's window -------------------------------------------------------------------------


def window_cases(rng):
    found = [
        (edge, {"days": days, "today": PRESENT_DAY})
        for edge, days in (
            ("negative", -5),
            ("zero", 0),
            ("one", 1),
            ("two", 2),
            ("week", 7),
            ("at-the-cap", 3650),
            ("above-the-cap", 3651),
            ("far-above-the-cap", 10_000),
        )
    ]
    found.append(("another-today", {"days": 30, "today": PRESENT_DAY + rng.randrange(1, 400)}))
    return found


def with_a_server(create_server, predecessor, *, days, today):
    asked = []

    async def xp_exchange_rates(**window):
        asked.append(window)
        return []

    pipeline = types.SimpleNamespace(
        today_date=lambda: as_date(today), xp_exchange_rates=xp_exchange_rates
    )
    settings = types.SimpleNamespace(http_host="127.0.0.1", http_port=0, drill_api_enabled=False)
    server = create_server(pipeline, settings, drill_auth=object())
    asyncio.run(server.call_tool("get_xp_exchange_rates", {"days": days}))
    (window,) = asked
    return {"start": window.get("start"), "end": window.get("end")}


FUNCTIONS = {
    "leaderboard": {
        "kind": "adapter",
        "function": "pipeline_layers.read_api.ReadApiLayer.leaderboard",
        "adapter": with_a_board_store,
        "note": "Drives it on a stand-in layer over a stub store holding the case's rollups, "
        "streak and XP total, which answers the n most recent rollups it is asked for and "
        "records n; returns each n and each board row, a day as its epoch day number.",
        "cases": board_cases,
    },
    "exchange_rates": {
        "kind": "adapter",
        "function": "exchange.exchange_rates",
        "adapter": with_a_ledger,
        "note": "Drives it on a temporary store filled with the case's XP rows and rollups and "
        "returns each source rate's fields.",
        "cases": exchange_cases,
    },
    "exchange_normalize_source": {
        "kind": "function",
        "function": "exchange.normalize_source",
        "cases": normalize_cases,
    },
    "exchange_window": {
        "kind": "adapter",
        "function": "server.create_server",
        "adapter": with_a_server,
        "note": "Builds the server over a stand-in pipeline that records the window it is asked "
        "for, calls the get_xp_exchange_rates tool with the case's days, and returns that "
        "window's first and last day, or none.",
        "cases": window_cases,
    },
}

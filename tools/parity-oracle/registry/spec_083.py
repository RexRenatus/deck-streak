"""SPEC-083's registrations: the skip day's day spec, search, preview, tariff, refund and summary.

Every adapter drives the predecessor's own function on a stand-in that holds only what the function
reads: a stub store answering the case's rows, a stand-in syncer whose guarded open yields a
stand-in collection, and a stand-in layer on the case's study day. The adapters compute nothing:
the search, the preview, the price and the credit are what the predecessor's code returned.

Days are epoch days in every input and output (SPEC-029 R3). The case builders draw only from the
`random.Random` the generator seeds. Nothing here reads a file.
"""

import asyncio
import contextlib
import datetime as dt
import types

#: The epoch day 0 as the ordinal the generator's `plain` subtracts.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()

#: Synthetic custom searches: a flat term, a deck, a tag and an `or` the wrap must group.
SEARCHES = (
    "prop:due=0 -is:suspended -is:buried",
    "deck:Spanish prop:due=0",
    "tag:daily or deck:Math",
    '"deck:Two Words" -tag:hard',
    "prop:due=0 -is:suspended -is:buried (deck:A or deck:B)",
)


def day(epoch_day):
    """The `date` of an epoch day."""
    return dt.date.fromordinal(EPOCH_ORDINAL + epoch_day)


def row(epoch_day, applied, undone, cards_moved=0):
    """One predecessor `skip_days` row from epoch days and flags."""
    return {
        "day": day(epoch_day).isoformat(),
        "applied": 1 if applied else 0,
        "undone": 1 if undone else 0,
        "cards_moved": cards_moved,
    }


def rows_of(rows):
    return [row(r["day"], r["applied"], r["undone"], r["cards_moved"]) for r in rows]


def spec_cases(rng):
    """Equal, reversed and sub-one bounds, then a seeded spread."""
    drawn = [(None, {"min_days": 1, "max_days": 3})]
    for lo, hi in ((2, 2), (3, 1), (0, 0), (-4, 2), (0, 5), (1, 1)):
        drawn.append(("bounds", {"min_days": lo, "max_days": hi}))
    for _ in range(6):
        drawn.append((None, {"min_days": rng.randrange(-2, 7), "max_days": rng.randrange(-2, 7)}))
    return drawn


class StubCollection:
    """A collection whose card search records the query and finds no card."""

    def __init__(self):
        self.queries = []

    def sync_login(self, *_args):
        return "token"

    def find_cards(self, query):
        self.queries.append(query)
        return []


def through_the_guarded_open(blocking, predecessor, *, search):
    """Run `_skip_day_blocking` on a stand-in syncer and return the search it sent to the engine."""
    collection = StubCollection()

    @contextlib.contextmanager
    def guarded():
        yield collection, None

    syncer = types.SimpleNamespace(
        _open_guarded=guarded,
        _username="user",
        _password="pass",
        _endpoint="endpoint",
        _converge=lambda _col, _auth: None,
    )
    blocking(syncer, "1-3", search, lambda _snapshots: None, lambda _moved: None)
    return collection.queries


def search_cases(rng):
    drawn = [("default" if s == SEARCHES[0] else "custom", {"search": s}) for s in SEARCHES]
    return drawn


class StubStore:
    """Answers the case's rows and balance, and records what the layer writes."""

    def __init__(self, rows, balance, rollup=None, active=None):
        self.rows = rows
        self.balance = balance
        self.rollup = rollup
        self.active = active
        self.deltas = []
        self.details = []

    async def get_daily_rollup(self, _today):
        return self.rollup

    async def get_active_skip_day(self, _today):
        return self.active

    async def all_skip_days(self):
        return self.rows

    async def coin_balance(self):
        return self.balance

    async def insert_coin_delta_once(self, on, source, ref, delta):
        self.deltas.append({"day": on, "source": source, "ref": ref, "delta": delta})

    async def set_skip_detail(self, _skip_id, detail):
        self.details.append(detail)


def preview_of(preview, predecessor, *, today, due_today, active, rows, balance):
    """Run `skip_preview` on a stand-in layer on `today` with a stub store."""
    layer_class = predecessor("pipeline_layers.skip.SkipDaysLayer")
    store = StubStore(
        rows_of(rows),
        balance,
        rollup=None if due_today is None else {"due_today": due_today},
        active={"cards_moved": 0} if active else None,
    )
    layer = types.SimpleNamespace(_store=store, _today=lambda: day(today))
    layer._skip_tariff = types.MethodType(layer_class._skip_tariff, layer)
    answer = asyncio.run(preview(layer))
    # The predecessor answers the day as an ISO string; a golden holds it as an epoch day.
    answer["day"] = dt.date.fromisoformat(answer["day"])
    return answer


def month_rows(rng, today):
    """Rows in the case's month and the month before, applied or not, undone or not."""
    rows = []
    for _ in range(rng.randrange(0, 5)):
        rows.append(
            {
                "day": today - rng.randrange(0, 45),
                "applied": rng.random() < 0.8,
                "undone": rng.random() < 0.3,
                "cards_moved": rng.randrange(0, 40),
            }
        )
    return rows


def preview_cases(rng):
    """The no-rollup class, then every rung of the ladder at, below and above the balance."""
    first_of_month = dt.date(2025, 5, 1).toordinal() - EPOCH_ORDINAL
    drawn = [
        (
            "no-rollup",
            {
                "today": first_of_month + 4,
                "due_today": None,
                "active": False,
                "rows": [],
                "balance": 60,
            },
        )
    ]
    for prior in range(5):
        for balance in (0, 49, 50, 100, 400):
            today = first_of_month + 10
            drawn.append(
                (
                    None,
                    {
                        "today": today,
                        "due_today": rng.randrange(0, 300),
                        "active": prior % 2 == 1,
                        "rows": applied_rows([first_of_month + i for i in range(prior)])
                        + [
                            {
                                "day": first_of_month - 3,
                                "applied": True,
                                "undone": False,
                                "cards_moved": 2,
                            }
                        ]
                        + [
                            {
                                "day": first_of_month + 8,
                                "applied": True,
                                "undone": True,
                                "cards_moved": 2,
                            }
                        ],
                        "balance": balance,
                    },
                )
            )
    return drawn


def charge_of(charge, predecessor, *, today, skip_id, rows, balance):
    """Run `_charge_skip_tariff` and return its answer with the delta and detail it wrote."""
    store = StubStore(rows_of(rows), balance)
    layer = types.SimpleNamespace(_store=store)
    answer = asyncio.run(charge(layer, day(today), skip_id))
    return {"answer": answer, "deltas": store.deltas, "details": store.details}


def applied_rows(days, cards=3):
    return [{"day": d, "applied": True, "undone": False, "cards_moved": cards} for d in days]


def tariff_cases(rng):
    """Each class of the SPEC's table: the price ladder and its edges."""
    today = 20100
    first_of_month = dt.date(2025, 5, 1).toordinal() - EPOCH_ORDINAL
    may_day = first_of_month + 10
    drawn = [
        (
            "first-free",
            {"today": may_day, "skip_id": 1, "rows": applied_rows([may_day]), "balance": 500},
        ),
        (
            "ladder-top",
            {
                "today": may_day + 3,
                "skip_id": 4,
                "rows": applied_rows([may_day, may_day + 1, may_day + 2, may_day + 3]),
                "balance": 500,
            },
        ),
        (
            "unfunded",
            {
                "today": may_day + 1,
                "skip_id": 2,
                "rows": applied_rows([may_day, may_day + 1]),
                "balance": 20,
            },
        ),
        (
            "month-boundary",
            {
                "today": first_of_month,
                "skip_id": 3,
                "rows": applied_rows([first_of_month - 1, first_of_month - 2, first_of_month]),
                "balance": 500,
            },
        ),
        (
            "undone-not-counted",
            {
                "today": may_day + 2,
                "skip_id": 3,
                "rows": applied_rows([may_day + 2])
                + [{"day": may_day, "applied": True, "undone": True, "cards_moved": 2}],
                "balance": 500,
            },
        ),
    ]
    for _ in range(8):
        n = rng.randrange(1, 6)
        base = today + rng.randrange(0, 60)
        drawn.append(
            (
                None,
                {
                    "today": base,
                    "skip_id": rng.randrange(1, 90),
                    "rows": applied_rows([base - i for i in range(n)]),
                    "balance": rng.choice([0, 30, 50, 100, rng.randrange(0, 300)]),
                },
            )
        )
    return drawn


class RefundConnection:
    """Answers the sum the skip paid, as the ledger query returns it."""

    def __init__(self, paid):
        self.paid = paid

    async def execute(self, _sql, _params):
        paid = self.paid

        class Cursor:
            async def fetchone(self):
                return {"t": paid}

        return Cursor()


def refund_of(refund, predecessor, *, undo_day, skip_id, paid):
    """Run `_refund_skip_tariff` and return what it returned with the credit and the day it lands."""
    store = StubStore([], 0)
    store.conn = RefundConnection(paid)
    layer = types.SimpleNamespace(_store=store, _today=lambda: day(undo_day))
    returned = asyncio.run(refund(layer, skip_id))
    return {"returned": returned, "deltas": store.deltas}


def refund_cases(rng):
    drawn = [
        ("free", {"undo_day": 20200, "skip_id": 7, "paid": 0}),
        ("paid-fifty", {"undo_day": 20201, "skip_id": 8, "paid": 50}),
    ]
    for _ in range(6):
        drawn.append(
            (
                None,
                {
                    "undo_day": 20200 + rng.randrange(0, 60),
                    "skip_id": rng.randrange(1, 90),
                    "paid": rng.choice([0, 30, 50, 100]),
                },
            )
        )
    return drawn


def summary_of(summarize, predecessor, *, today, rows):
    """Run `summarize_skips` on rows and `today` built from epoch days."""
    summary = summarize(rows_of(rows), day(today))
    return {
        "this_month": summary.this_month,
        "all_time": summary.all_time,
        "last_day": summary.last_day,
        "cards_moved_all_time": summary.cards_moved_all_time,
    }


def summary_cases(rng):
    today = 20150
    drawn = [
        ("empty", {"today": today, "rows": []}),
        (
            "inactive-ignored",
            {
                "today": today,
                "rows": [
                    {"day": today, "applied": False, "undone": False, "cards_moved": 9},
                    {"day": today - 1, "applied": True, "undone": True, "cards_moved": 4},
                    {"day": today - 2, "applied": True, "undone": False, "cards_moved": 5},
                ],
            },
        ),
    ]
    for _ in range(10):
        drawn.append(
            (
                None,
                {
                    "today": today + rng.randrange(0, 40),
                    "rows": month_rows(rng, today + 20) + month_rows(rng, today),
                },
            )
        )
    return drawn


FUNCTIONS = {
    "skip_spec": {
        "kind": "function",
        "function": "skip.skip_spec",
        "cases": spec_cases,
    },
    "skip_search": {
        "kind": "adapter",
        "function": "sync.AnkiSyncer._skip_day_blocking",
        "adapter": through_the_guarded_open,
        "note": (
            "Runs the blocking skip on a stand-in syncer whose guarded open yields a stand-in "
            "collection: its login answers a token, the converge succeeds, and its card search "
            "records the query and finds no card, so the function returns before any write."
        ),
        "cases": search_cases,
    },
    "skip_preview": {
        "kind": "adapter",
        "function": "pipeline_layers.skip.SkipDaysLayer.skip_preview",
        "adapter": preview_of,
        "note": (
            "Runs the preview on a stand-in layer on the case's study day with a stub store "
            "answering the day's rollup or none, an active skip or none, the skip rows and the "
            "balance; the day returned as an epoch day."
        ),
        "cases": preview_cases,
    },
    "skip_tariff": {
        "kind": "adapter",
        "function": "pipeline_layers.economy.EconomyLayer._charge_skip_tariff",
        "adapter": charge_of,
        "note": (
            "Runs the charge on a stub store answering the case's skip rows and balance, and "
            "records the coin delta and the detail it writes."
        ),
        "cases": tariff_cases,
    },
    "skip_tariff_refund": {
        "kind": "adapter",
        "function": "pipeline_layers.economy.EconomyLayer._refund_skip_tariff",
        "adapter": refund_of,
        "note": (
            "Runs the refund on a stub store answering what the skip paid and records the credit "
            "and the day it lands on."
        ),
        "cases": refund_cases,
    },
    "skip_summary": {
        "kind": "adapter",
        "function": "skip.summarize_skips",
        "adapter": summary_of,
        "note": (
            "Builds the rows and today from epoch days and returns the summary's counts, the "
            "last day as an epoch day or null."
        ),
        "cases": summary_cases,
    },
    "skip.constants": {
        "kind": "constants",
        "names": [
            "constants.SKIP_TARIFF_LADDER",
            "constants.SKIP_DEFAULT_SEARCH",
            "constants.SKIP_SPREAD_MIN_DAYS",
            "constants.SKIP_SPREAD_MAX_DAYS",
            "constants.SKIP_MAX_CARDS",
            "constants.SKIP_BRIDGE_MONTHLY_CAP",
        ],
    },
}

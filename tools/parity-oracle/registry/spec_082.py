"""SPEC-082's registrations: the coin mint, the daily loss cap, the wealth-scaled fine, the debit
clip, the wallet at a day's start, the day's debits, and the coin and shop constants.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_a_ledger` drives `database.py:GamifyStore.coin_balance_before` and
  `database.py:GamifyStore.coin_debits_for_day` on a temporary store the adapter fills with the
  case's synthetic movements through the store's own `upsert_coin_grant`, each day turned from its
  epoch day number into the store's form.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic, every day an epoch day number.
"""

import asyncio
import datetime as dt
import tempfile
from pathlib import Path

#: The epoch day's ordinal, so a day number turns into the `date` the predecessor reads.
EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: A study day number near the present.
PRESENT_DAY = 20_000
#: The divisor and the cap of the mint, the cap's wallet share and the fine's wallet share as the
#: case builders aim at them. The goldens come from the predecessor's functions, never from these.
MINT_DIVISOR = 25
MINT_CAP = 40
CAP_SHARE_DENOMINATOR = 0.3
FINE_SHARE_DENOMINATOR = 0.02


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


# --- the mint -----------------------------------------------------------------------------------


def mint_cases(rng):
    found = [("zero", {"base_xp": 0}), ("negative", {"base_xp": -1}), ("negative", {"base_xp": -25})]
    found.append(("negative", {"base_xp": -(10**6)}))
    for multiple in range(0, MINT_CAP + 3):
        around = multiple * MINT_DIVISOR
        for base in (around - 1, around, around + 1):
            found.append((None, {"base_xp": base}))
    cap_at = MINT_CAP * MINT_DIVISOR
    for base in (cap_at - 1, cap_at, cap_at + 1, cap_at + MINT_DIVISOR, 10 * cap_at, 10**9):
        found.append(("cap", {"base_xp": base}))
    for _ in range(12):
        found.append((None, {"base_xp": rng.randrange(-200, 3000)}))
    return found


# --- the loss cap ---------------------------------------------------------------------------------


def cap_cases(rng):
    found = [("zero", {"wallet_at_rollover": 0}), ("negative", {"wallet_at_rollover": -1})]
    found.append(("negative", {"wallet_at_rollover": -500}))
    shares = range(0, 40)
    for share in shares:
        # The wallet whose share lands on a whole number, and one coin to each side.
        whole = round(share / CAP_SHARE_DENOMINATOR)
        for wallet in (whole - 1, whole, whole + 1):
            found.append(("whole" if wallet == whole else None, {"wallet_at_rollover": wallet}))
    absolute = round(100 / CAP_SHARE_DENOMINATOR)
    for wallet in (absolute - 2, absolute - 1, absolute, absolute + 1, absolute + 2, 10**4, 10**9):
        found.append(("absolute", {"wallet_at_rollover": wallet}))
    for _ in range(12):
        found.append((None, {"wallet_at_rollover": rng.randrange(-50, 1200)}))
    return found


# --- the scaled fine ------------------------------------------------------------------------------


def fine_cases(rng):
    found = []
    for configured in (-10, -1, 0):
        for wallet in (-5, 0, 1, 100, 10_000):
            found.append(("configured_zero_or_less", {"configured": configured, "wallet": wallet}))
    for configured in (1, 2, 5, 10, 25, 50):
        found.append(("empty", {"configured": configured, "wallet": 0}))
        found.append(("empty", {"configured": configured, "wallet": -7}))
        # Around the wallet where 2% of it passes the configured fine.
        crossing = round(configured / FINE_SHARE_DENOMINATOR)
        for wallet in range(crossing - 2, crossing + 4):
            found.append(("crossing", {"configured": configured, "wallet": wallet}))
        for wallet in (1, crossing // 2, crossing * 3, crossing * 20 + 1):
            found.append((None, {"configured": configured, "wallet": wallet}))
    for _ in range(12):
        found.append(
            (None, {"configured": rng.randrange(-3, 80), "wallet": rng.randrange(-20, 5000)})
        )
    return found


# --- the clip -------------------------------------------------------------------------------------


def clip_cases(rng):
    found = [
        ("request_zero", {"requested": 0, "wallet": 50, "cap_remaining": 30}),
        ("request_negative", {"requested": -4, "wallet": 50, "cap_remaining": 30}),
        ("exact", {"requested": 30, "wallet": 50, "cap_remaining": 30}),
        ("exact", {"requested": 50, "wallet": 50, "cap_remaining": 60}),
        ("floor", {"requested": 20, "wallet": 0, "cap_remaining": 30}),
        ("floor", {"requested": 20, "wallet": -3, "cap_remaining": 30}),
        ("cap_spent", {"requested": 20, "wallet": 80, "cap_remaining": 0}),
        ("cap_spent", {"requested": 20, "wallet": 80, "cap_remaining": -5}),
        ("forgiven_by_cap", {"requested": 20, "wallet": 80, "cap_remaining": 12}),
        ("forgiven_by_wallet", {"requested": 20, "wallet": 7, "cap_remaining": 30}),
    ]
    for requested in (1, 9, 10, 11):
        for wallet in (9, 10, 11):
            for cap_remaining in (9, 10, 11):
                found.append(
                    (None, {"requested": requested, "wallet": wallet, "cap_remaining": cap_remaining})
                )
    for _ in range(12):
        found.append(
            (
                None,
                {
                    "requested": rng.randrange(-5, 200),
                    "wallet": rng.randrange(-10, 300),
                    "cap_remaining": rng.randrange(-10, 120),
                },
            )
        )
    return found


# --- the day's wallet and debits ------------------------------------------------------------------

SOURCES = ("mint", "quest", "shop", "fine", "tariff", "refund", "payout")


def movement_rows(rng, count):
    """Synthetic movements on the days around PRESENT_DAY, one per distinct key."""
    rows = []
    for index in range(count):
        delta = rng.randrange(1, 60)
        source = rng.choice(SOURCES)
        if source in ("shop", "fine", "tariff"):
            delta = -delta
        rows.append(
            {
                "day": PRESENT_DAY + rng.randrange(-3, 3),
                "source": source,
                "ref": f"k{index}",
                "delta": delta,
            }
        )
    return rows


def ledger_cases(rng):
    found = [("empty", {"movements": [], "day": PRESENT_DAY})]
    found.append(
        (
            "boundary",
            {
                "movements": [
                    {"day": PRESENT_DAY - 1, "source": "mint", "ref": "", "delta": 40},
                    {"day": PRESENT_DAY, "source": "mint", "ref": "", "delta": 25},
                    {"day": PRESENT_DAY, "source": "shop", "ref": "pass:1", "delta": -40},
                    {"day": PRESENT_DAY, "source": "fine", "ref": "a", "delta": -15},
                    {"day": PRESENT_DAY, "source": "quest", "ref": "q", "delta": 10},
                    {"day": PRESENT_DAY + 1, "source": "fine", "ref": "b", "delta": -9},
                    {"day": PRESENT_DAY - 2, "source": "shop", "ref": "freeze:1", "delta": -150},
                ],
                "day": PRESENT_DAY,
            },
        )
    )
    found.append(
        (
            "later_days_only",
            {
                "movements": [
                    {"day": PRESENT_DAY + 1, "source": "mint", "ref": "", "delta": 12},
                    {"day": PRESENT_DAY + 2, "source": "fine", "ref": "z", "delta": -5},
                ],
                "day": PRESENT_DAY,
            },
        )
    )
    found.append(
        (
            "purchase_only",
            {
                "movements": [
                    {"day": PRESENT_DAY, "source": "shop", "ref": "pass:2", "delta": -40},
                ],
                "day": PRESENT_DAY,
            },
        )
    )
    for _ in range(10):
        found.append(
            (None, {"movements": movement_rows(rng, rng.randrange(1, 14)), "day": PRESENT_DAY})
        )
    return found


def with_a_ledger(method, predecessor, *, movements, day):
    """Fill a temporary store with the movements and call `method` for the day."""
    store_type = predecessor("database.GamifyStore")

    async def run():
        with tempfile.TemporaryDirectory() as directory:
            store = await store_type(Path(directory) / "store.db").connect()
            try:
                for movement in movements:
                    await store.upsert_coin_grant(
                        as_date(movement["day"]),
                        movement["source"],
                        movement["ref"],
                        movement["delta"],
                    )
                return await method(store, as_date(day))
            finally:
                await store.close()

    return asyncio.run(run())


FUNCTIONS = {
    "mint_for_base_xp": {
        "kind": "function",
        "function": "gamification.economy.mint_for_base_xp",
        "cases": mint_cases,
    },
    "daily_loss_cap": {
        "kind": "function",
        "function": "gamification.economy.daily_loss_cap",
        "cases": cap_cases,
    },
    "scaled_fine": {
        "kind": "function",
        "function": "gamification.economy.scaled_fine",
        "cases": fine_cases,
    },
    "clip_debit": {
        "kind": "function",
        "function": "gamification.economy.clip_debit",
        "cases": clip_cases,
    },
    "coin_balance_before": {
        "kind": "adapter",
        "function": "database.GamifyStore.coin_balance_before",
        "adapter": with_a_ledger,
        "note": "Fills a temporary store with the case's synthetic movements through its own "
        "upsert_coin_grant, then reads the wallet before the day; each day is an epoch day "
        "turned into the store's form inside the adapter.",
        "cases": ledger_cases,
    },
    "coin_debits_for_day": {
        "kind": "adapter",
        "function": "database.GamifyStore.coin_debits_for_day",
        "adapter": with_a_ledger,
        "note": "Fills a temporary store with the case's synthetic movements through its own "
        "upsert_coin_grant, then reads the day's debits; each day is an epoch day turned into "
        "the store's form inside the adapter.",
        "cases": ledger_cases,
    },
    "economy.constants": {
        "kind": "constants",
        "names": [
            "constants.COIN_MINT_XP_DIVISOR",
            "constants.COIN_MINT_DAILY_CAP",
            "constants.DAILY_LOSS_CAP_COINS",
            "constants.DAILY_LOSS_CAP_WALLET_FRAC",
            "constants.FINE_WALLET_FRAC",
            "constants.SHOP_FREEZE_PRICE",
            "constants.SHOP_SCROLL_PASS_PRICE",
            "constants.SCROLL_PASS_MINUTES",
            "constants.PASS_SURCHARGE_MULT",
            "constants.PASS_SURCHARGE_HOURS",
            "constants.STREAK_FREEZE_CAP",
        ],
    },
}

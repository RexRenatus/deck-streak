#!/usr/bin/env python3
"""game-economy-probe.py -- a game economy checked against v9's exact math and its ethics.

SPEC-V2-2224. DeckStreak ports gamify v9 with "full v9 parity": its XP table, the Bloom-tier
multiplier on the tagged track, the chests and their pity, the coins and their loss cap, and the
streak rules. The engine reads every constant from ONE file, `economy.json` at the repository
root (schema `phx.game.economy.v1`), and this script judges that file:

    economy-declared        block     the file exists, parses, and has the reference's exact shape
    xp-table                block     per-review XP, daily bonuses and the level curve match
    tier-multiplier         block     T1-T4 = 1/2/3/5 on the tagged track only, rising, untagged 1
    xp-inflation            block     every bonus is capped and no multiplier bonus compounds
    chest-odds              block     70/22/7/1, summing to 100, with the pity ramp and guarantees
    odds-disclosed          block     the odds the player is shown are the odds the server rolls
    chest-not-sold          block     no chest, and nothing that rolls one, is ever sold
    coin-flows              block     the mint, the earn amounts and the shop prices match
    coin-balance            advisory  a repeatable sink exists and every sink is within reach
    loss-cap                block     the daily loss cap, wealth-scaled fines, a zero floor
    only-coins-confiscable  block     a penalty takes coins or nothing, never XP or the streak
    streak-rules            block     freezes, breaks, skip tariffs, the governor and the wager
    streak-pressure         block     forgiveness is earnable, penalties opt-in and lapse-silenced

Parity is read against a REFERENCE economy: the pack's `templates/economy.json`, which holds v9's
exact values and is also the template a project copies. `--reference FILE` names another one, so
the checks run for any economy whose owner has recorded a different decision. The invariants
(odds summing to 100, a rising level curve, only coins confiscable, opt-in penalties) hold for
every reference, so judging an economy against itself still refuses a broken one.

`golden` prints the per-review XP of every ease, maturity, type and tier, and the XP each level
needs, computed the way v9 computes them (Python's round, half to even), for the port's own tests.

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`: the
values it read to decide. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: nothing was
examined, or an input (the economy, the reference) could not be read. VOID is never a pass.

Standard library only, so it vendors into any repository.

Usage:
    game-economy-probe.py classes
    game-economy-probe.py [--reference FILE] golden
    game-economy-probe.py --root R [--config PATH] [--reference FILE]
                          [--max-days-to-afford N] check <class>
"""

import argparse
import json
import math
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_REFERENCE = (
    HERE.parent / "skills" / "packs" / "game-economy" / "templates" / "economy.json"
)
DEFAULT_CONFIG = "economy.json"
SCHEMA = "phx.game.economy.v1"

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

CLASSES = (
    "economy-declared",
    "xp-table",
    "tier-multiplier",
    "xp-inflation",
    "chest-odds",
    "odds-disclosed",
    "chest-not-sold",
    "coin-flows",
    "coin-balance",
    "loss-cap",
    "only-coins-confiscable",
    "streak-rules",
    "streak-pressure",
)
FREE_TEXT = frozenset({"note"})
PARITY = {
    "xp-table": (
        "xp.base",
        "xp.rounding",
        "xp.ease_multipliers",
        "xp.maturity",
        "xp.type_multipliers",
        "xp.daily_bonuses",
        "xp.level_curve",
    ),
    "tier-multiplier": ("xp.tier",),
    "xp-inflation": ("xp.bonuses", "xp.day_base_excludes", "chests.payout_xp"),
    "chest-odds": (
        "chests.odds_percent",
        "chests.pity",
        "chests.buff_points",
        "chests.per_day_max",
        "chests.effort_floor_distinct_cards",
        "chests.session_gap_minutes",
    ),
    "coin-flows": ("coins.mint", "coins.earn", "shop"),
    "loss-cap": ("coins.loss_cap", "coins.fines", "coins.wallet_floor"),
    "streak-rules": ("streak", "governor", "wager"),
}
RARITIES = ("common", "rare", "epic", "legendary")
MULTIPLIER_KEYS = frozenset({"fraction", "max_multiplier", "window_hours"})
CAP_KEYS = frozenset({"cap", "max", "max_multiplier", "per_day", "xp"})
RANDOM_GRANTS = frozenset(
    {
        "chest",
        "chest-roll",
        "loot",
        "loot-box",
        "mystery-box",
        "random-reward",
        "gacha",
        "spin",
    }
)
UNCONFISCABLE = ("xp", "streak", "level", "badges", "cefr")
PENALTY_DEBITS = frozenset({"coins", "none"})
ROUNDING = ("half-even", "half-away-from-zero")
MISSING = object()


class EconomyError(ValueError):
    """An economy or a reference cannot be read. str() is why."""


# --- reading ------------------------------------------------------------------------------------


def _read_json(path: Path, what: str) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise EconomyError(f"{what} {path}: missing") from error
    except (OSError, UnicodeDecodeError) as error:
        raise EconomyError(f"{what} {path}: unreadable: {error}") from error
    except ValueError as error:
        raise EconomyError(f"{what} {path}: not JSON: {error}") from error


def load_reference(path: "Path | str | None" = None) -> dict:
    """The reference economy; None is the pack's v9 template. Raises EconomyError."""
    where = Path(path) if path is not None else DEFAULT_REFERENCE
    data = _read_json(where, "the reference")
    if not isinstance(data, dict) or data.get("schema") != SCHEMA:
        raise EconomyError(f"the reference {where} is not a {SCHEMA} object")
    return data


def get(data: object, dotted: str) -> object:
    """The value at a dotted path, or MISSING."""
    for key in dotted.split("."):
        if not isinstance(data, dict) or key not in data:
            return MISSING
        data = data[key]
    return data


def _is_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _number(data: object, dotted: str) -> "float | None":
    value = get(data, dotted)
    return float(value) if _is_number(value) and math.isfinite(value) else None


def _same(left: object, right: object) -> bool:
    if _is_number(left) and _is_number(right):
        return math.isclose(left, right, rel_tol=0.0, abs_tol=1e-9)
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(_same(a, b) for a, b in zip(left, right))
    return type(left) is type(right) and left == right


def _leaves(value: object, prefix: str) -> list:
    if isinstance(value, dict):
        found = []
        for key, child in value.items():
            if key not in FREE_TEXT:
                found += _leaves(child, f"{prefix}.{key}")
        return found
    return [(prefix, value)]


def _parity(config: dict, reference: dict, prefixes: tuple) -> tuple:
    examined = 0
    findings = []
    for prefix in prefixes:
        for path, expected in _leaves(get(reference, prefix), prefix):
            examined += 1
            actual = get(config, path)
            if actual is MISSING:
                findings.append(f"{path} is missing; the reference says {expected!r}")
            elif not _same(actual, expected):
                findings.append(
                    f"{path} is {actual!r}; the reference says {expected!r}"
                )
    return examined, findings


def _shape(config: object, reference: object, path: str) -> tuple:
    """(examined, problems) of config against the reference's keys and JSON types."""
    if isinstance(reference, dict):
        if not isinstance(config, dict):
            return 1, [f"{path or 'the economy'} is not an object"]
        examined = 0
        problems = []
        for key, child in reference.items():
            if key in FREE_TEXT:
                continue
            where = f"{path}.{key}" if path else key
            if key not in config:
                examined += 1
                problems.append(f"missing key {where}")
                continue
            count, found = _shape(config[key], child, where)
            examined += count
            problems += found
        for key in config:
            if key not in reference and key not in FREE_TEXT:
                where = f"{path}.{key}" if path else key
                problems.append(f"unknown key {where}")
        return examined, problems
    if isinstance(reference, list):
        if not isinstance(config, list):
            return 1, [f"{path} is not a list"]
        template = reference[0] if reference else ""
        bad = [
            index
            for index, item in enumerate(config)
            if _shape(item, template, f"{path}[{index}]")[1]
        ]
        return 1, [f"{path}[{index}] is not a {_kind(template)}" for index in bad]
    if isinstance(reference, bool):
        return 1, [] if isinstance(config, bool) else [f"{path} is not a boolean"]
    if _is_number(reference):
        ok = _is_number(config) and math.isfinite(config)
        return 1, [] if ok else [f"{path} is not a number"]
    if isinstance(reference, str):
        return 1, [] if isinstance(config, str) else [f"{path} is not a string"]
    return 1, [f"{path}: the reference holds an unsupported value"]


def _kind(template: object) -> str:
    if isinstance(template, bool):
        return "boolean"
    if _is_number(template):
        return "number"
    if isinstance(template, dict):
        return "object"
    return "string"


# --- the classes: each returns (examined, findings) ---------------------------------------------


def _economy_declared(config, reference, context) -> tuple:
    examined, problems = _shape(config, reference, "")
    if isinstance(config, dict) and config.get("schema") != SCHEMA:
        problems.insert(0, f"schema is {config.get('schema')!r}, not {SCHEMA}")
    return examined, problems


def _xp_table(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["xp-table"])
    quadratic = _number(config, "xp.level_curve.quadratic")
    linear = _number(config, "xp.level_curve.linear")
    if quadratic is None or linear is None:
        findings.append(
            "xp.level_curve needs numeric quadratic and linear coefficients"
        )
    else:
        if quadratic + linear != 0:
            findings.append(
                f"level 1 needs {quadratic + linear:g} XP on this curve; level 1 starts at 0"
            )
        if quadratic <= 0 or 3 * quadratic + linear <= 0:
            findings.append(
                "the level cost does not rise: each level must cost more XP than the last "
                f"(quadratic {quadratic:g}, linear {linear:g})"
            )
    for group in ("xp.ease_multipliers", "xp.maturity", "xp.type_multipliers"):
        for path, value in _leaves(get(config, group), group):
            examined += 1
            if path.endswith("mature_interval_days"):
                continue
            if not (_is_number(value) and value > 0):
                findings.append(
                    f"{path} is {value!r}; a multiplier is a positive number"
                )
    base = _number(config, "xp.base")
    if base is None or base <= 0:
        findings.append("xp.base is not a positive number")
    rounding = get(config, "xp.rounding")
    if rounding not in ROUNDING:
        findings.append(
            f"xp.rounding is {rounding!r}; name one of {', '.join(ROUNDING)}"
        )
    return examined, findings


def _tier_multiplier(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["tier-multiplier"])
    track = get(config, "xp.tier.track")
    if not (isinstance(track, str) and track):
        findings.append(
            "xp.tier.track names no track; the multiplier applies to one tagged track"
        )
    untagged = _number(config, "xp.tier.untagged")
    if untagged is None or not math.isclose(untagged, 1.0):
        findings.append(
            f"xp.tier.untagged is {get(config, 'xp.tier.untagged')!r}; an untagged card and "
            "every other track keep the unmultiplied rate, 1.0"
        )
    tiers = get(config, "xp.tier.multipliers")
    if not isinstance(tiers, dict) or not tiers:
        findings.append("xp.tier.multipliers holds no tier")
        return examined, findings
    previous = None
    for name in sorted(tiers, key=lambda key: (len(key), key)):
        value = tiers[name]
        if not (_is_number(value) and value > 0):
            findings.append(
                f"xp.tier.multipliers.{name} is {value!r}; a multiplier is positive"
            )
            continue
        if previous is not None and value <= previous[1]:
            findings.append(
                f"xp.tier.multipliers.{name} is {value:g}, not above {previous[0]}'s "
                f"{previous[1]:g}; a higher Bloom tier earns more"
            )
        previous = (name, value)
    return examined, findings


def _xp_inflation(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["xp-inflation"])
    bonuses = get(config, "xp.bonuses")
    excludes = get(config, "xp.day_base_excludes")
    excludes = excludes if isinstance(excludes, list) else []
    for name, bonus in (bonuses if isinstance(bonuses, dict) else {}).items():
        if not isinstance(bonus, dict):
            continue
        if MULTIPLIER_KEYS & set(bonus) and name not in excludes:
            findings.append(
                f"xp.bonuses.{name} is multiplier-derived and not in xp.day_base_excludes; "
                "it would compound on the day's base"
            )
        for key, value in bonus.items():
            if key in CAP_KEYS and not (_is_number(value) and value > 0):
                findings.append(
                    f"xp.bonuses.{name}.{key} is {value!r}; a bonus needs a positive cap"
                )
    low = _number(config, "xp.bonuses.freespin.min")
    high = _number(config, "xp.bonuses.freespin.max")
    if low is not None and high is not None and low > high:
        findings.append(f"xp.bonuses.freespin runs from {low:g} down to {high:g}")
    for rarity in ("common", "rare"):
        low = _number(config, f"chests.payout_xp.{rarity}_min")
        high = _number(config, f"chests.payout_xp.{rarity}_max")
        if low is None or high is None or low < 0 or low > high:
            findings.append(
                f"chests.payout_xp {rarity} range {low!r}..{high!r} is not a range"
            )
    fraction = _number(config, "chests.payout_xp.cap_session_fraction")
    if fraction is None or not 0 < fraction <= 1:
        findings.append(
            "chests.payout_xp.cap_session_fraction is not a share above 0 and at most 1"
        )
    for key in ("cap_floor", "legendary", "epic_fallback"):
        value = _number(config, f"chests.payout_xp.{key}")
        if value is None or value <= 0:
            findings.append(f"chests.payout_xp.{key} is not a positive cap")
    return examined, findings


def _chest_odds(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["chest-odds"])
    odds = get(config, "chests.odds_percent")
    if not isinstance(odds, dict) or set(odds) != set(RARITIES):
        findings.append(f"chests.odds_percent must name exactly {', '.join(RARITIES)}")
        return examined, findings
    if not all(_is_number(value) and value >= 0 for value in odds.values()):
        findings.append("chests.odds_percent holds a negative or non-numeric odd")
        return examined, findings
    total = sum(odds.values())
    if not math.isclose(total, 100, abs_tol=1e-9):
        findings.append(f"chests.odds_percent sums to {total:g}, not 100")
    after = _number(config, "chests.pity.epic_ramp_after")
    epic_at = _number(config, "chests.pity.epic_guaranteed_at")
    legendary_at = _number(config, "chests.pity.legendary_guaranteed_at")
    ceiling = _number(config, "chests.pity.epic_ceiling_points")
    step = _number(config, "chests.pity.epic_ramp_points")
    if None in (after, epic_at, legendary_at, ceiling, step):
        findings.append("chests.pity needs numeric ramp, ceiling and guarantee values")
        return examined, findings
    if after >= epic_at:
        findings.append(
            f"the Epic ramp starts after chest {after:g}, not before the guarantee at {epic_at:g}"
        )
    if ceiling < odds["epic"]:
        findings.append(
            f"the Epic ceiling {ceiling:g} is below the base Epic odds {odds['epic']:g}"
        )
    if step < 0:
        findings.append(f"the Epic ramp step {step:g} is negative")
    if legendary_at < epic_at:
        findings.append(
            f"the legendary guarantee at {legendary_at:g} comes before the Epic one at {epic_at:g}"
        )
    for key in ("per_day_max", "effort_floor_distinct_cards", "session_gap_minutes"):
        value = _number(config, f"chests.{key}")
        if value is None or value < 1:
            findings.append(f"chests.{key} is not at least 1")
    return examined, findings


def _stated(text: str, rarity: str) -> list:
    """(line, percent) for every place `text` states a percent for `rarity`."""
    name = re.escape(rarity)
    number = r"([0-9]+(?:\.[0-9]+)?)\s*%"
    forward = re.compile(
        rf"\b{name}\b\s*(?:[:=—–-]|odds|chance|is|at)?\s*(?:[:=—–-]\s*)?{number}", re.I
    )
    backward = re.compile(
        rf"{number}\s*(?:chance\s+(?:of|for)\s+)?(?:an?\s+)?{name}\b", re.I
    )
    found = []
    for offset, line in enumerate(text.split("\n"), 1):
        for pattern in (forward, backward):
            found += [
                (offset, float(match.group(1))) for match in pattern.finditer(line)
            ]
    return found


def _odds_disclosed(config, reference, context) -> tuple:
    root = context["root"]
    paths = get(config, "chests.disclosure")
    if not isinstance(paths, list) or not paths:
        return 1, [
            "no disclosure file in chests.disclosure; the player sees the odds the server rolls"
        ]
    odds = get(config, "chests.odds_percent")
    odds = odds if isinstance(odds, dict) else {}
    examined = 0
    findings = []
    texts = []
    for relative in paths:
        examined += 1
        if not isinstance(relative, str) or Path(relative).is_absolute():
            findings.append(f"chests.disclosure {relative!r} lies outside the tree")
            continue
        path = (root / relative).resolve()
        try:
            path.relative_to(root.resolve())
        except ValueError:
            findings.append(f"chests.disclosure {relative} lies outside the tree")
            continue
        try:
            texts.append((relative, path.read_text(encoding="utf-8")))
        except (OSError, UnicodeDecodeError):
            findings.append(f"chests.disclosure {relative}: missing or unreadable")
    if not texts:
        return examined, findings
    for rarity, percent in odds.items():
        examined += 1
        stated = [
            (relative, line, value)
            for relative, text in texts
            for line, value in _stated(text, rarity)
        ]
        if not stated:
            findings.append(f"the {rarity} odds are stated in no disclosure file")
        for relative, line, value in stated:
            if _is_number(percent) and not math.isclose(value, percent, abs_tol=1e-9):
                findings.append(
                    f"{relative}:{line}: states {rarity} at {value:g}%; the chest rolls {percent:g}%"
                )
    for key, label in (
        ("epic_guaranteed_at", "Epic"),
        ("legendary_guaranteed_at", "Legendary"),
    ):
        examined += 1
        value = get(config, f"chests.pity.{key}")
        if not _is_number(value):
            continue
        needle = re.compile(rf"(?<![0-9.]){int(value)}(?![0-9])")
        if not any(needle.search(text) for _, text in texts):
            findings.append(
                f"the {label} guarantee ({int(value)}) is stated in no disclosure file"
            )
    return examined, findings


def _chest_not_sold(config, reference, context) -> tuple:
    examined = 2
    findings = []
    if get(config, "chests.purchasable") is not False:
        findings.append(
            "chests.purchasable is not false; a chest is earned by study and never sold"
        )
    if get(config, "coins.sold_for_money") is not False:
        findings.append(
            "coins.sold_for_money is not false; a coin bought with money makes every coin-priced "
            "reward a paid one"
        )
    shop = get(config, "shop")
    for name, item in (shop if isinstance(shop, dict) else {}).items():
        examined += 1
        if not isinstance(item, dict):
            findings.append(f"shop.{name} is not an item")
            continue
        if item.get("currency") != "coins":
            findings.append(
                f"shop.{name} is priced in {item.get('currency')!r}; items are bought with earned "
                "coins only"
            )
        if str(item.get("grants", "")).lower() in RANDOM_GRANTS:
            findings.append(
                f"shop.{name} grants a {item.get('grants')!r}: a randomized reward cannot be bought"
            )
    return examined, findings


def _coin_flows(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["coin-flows"])
    for key in ("xp_divisor", "daily_cap"):
        value = _number(config, f"coins.mint.{key}")
        if value is None or value <= 0:
            findings.append(f"coins.mint.{key} is not a positive number")
    earn = get(config, "coins.earn")
    for name, value in (earn if isinstance(earn, dict) else {}).items():
        if not (_is_number(value) and value >= 0):
            findings.append(
                f"coins.earn.{name} is {value!r}; an earn amount is a number of coins"
            )
    shop = get(config, "shop")
    for name, item in (shop if isinstance(shop, dict) else {}).items():
        price = item.get("price") if isinstance(item, dict) else None
        if not (_is_number(price) and price > 0):
            findings.append(
                f"shop.{name}.price is {price!r}; a sink has a positive price"
            )
    return examined, findings


def _coin_balance(config, reference, context) -> tuple:
    horizon = context["max_days"]
    mint = _number(config, "coins.mint.daily_cap")
    shop = get(config, "shop")
    items = shop if isinstance(shop, dict) else {}
    examined = 1 + len(items)
    if mint is None or mint <= 0:
        return examined, [
            "coins.mint.daily_cap is not positive; nothing bounds the daily faucet"
        ]
    sinks = {
        name: item["price"]
        for name, item in items.items()
        if isinstance(item, dict)
        and item.get("repeatable") is True
        and _is_number(item.get("price"))
    }
    if not sinks:
        return examined, [
            "no repeatable coin sink in the shop; coins minted every day pile up"
        ]
    findings = []
    for name, price in sinks.items():
        days = price / mint
        if days > horizon:
            findings.append(
                f"shop.{name} costs {price:g} coins, {days:.1f} days of the full daily mint "
                f"({mint:g}); more than {horizon:g} days is out of reach"
            )
    if max(sinks.values()) < mint:
        findings.append(
            f"every repeatable sink costs less than one day of the mint ({mint:g}); nothing is "
            "worth saving for"
        )
    return examined, findings


def _loss_cap(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["loss-cap"])
    absolute = _number(config, "coins.loss_cap.absolute")
    share = _number(config, "coins.loss_cap.wallet_fraction")
    fine = _number(config, "coins.fines.wallet_fraction")
    floor = get(config, "coins.wallet_floor")
    if absolute is None or absolute <= 0:
        findings.append("coins.loss_cap.absolute is not a positive cap")
    if share is None or not 0 < share <= 1:
        findings.append(
            "coins.loss_cap.wallet_fraction is not a share above 0 and at most 1"
        )
    if not (_is_number(floor) and floor == 0):
        findings.append(f"coins.wallet_floor is {floor!r}; a wallet never goes below 0")
    if fine is None or fine <= 0 or (share is not None and fine > share):
        findings.append(
            f"coins.fines.wallet_fraction {fine!r} is not a positive share within the daily "
            f"loss cap's {share!r}"
        )
    return examined, findings


def _only_coins_confiscable(config, reference, context) -> tuple:
    findings = []
    listed = get(config, "unconfiscable")
    listed = listed if isinstance(listed, list) else []
    for asset in UNCONFISCABLE:
        if asset not in listed:
            findings.append(f"unconfiscable omits {asset}; only coins are ever taken")
    penalties = get(config, "penalties")
    penalties = penalties if isinstance(penalties, dict) else {}
    for name, penalty in penalties.items():
        debits = penalty.get("debits") if isinstance(penalty, dict) else None
        if debits not in PENALTY_DEBITS or debits in listed:
            findings.append(
                f"penalties.{name} debits {debits!r}; a penalty takes coins or nothing, never "
                f"{', '.join(UNCONFISCABLE)}"
            )
    return len(UNCONFISCABLE) + len(penalties), findings


def _streak_rules(config, reference, context) -> tuple:
    examined, findings = _parity(config, reference, PARITY["streak-rules"])
    misses = _number(config, "streak.misses_to_break")
    if misses is None or misses < 2:
        findings.append(
            f"streak.misses_to_break is {get(config, 'streak.misses_to_break')!r}; one missed "
            "day is bridged by a freeze, never a break"
        )
    start = _number(config, "streak.start_freezes")
    cap = _number(config, "streak.freeze_cap")
    if start is None or cap is None or cap < start:
        findings.append("streak.freeze_cap is below streak.start_freezes")
    tariff = get(config, "streak.skip_tariff_coins")
    if not (
        isinstance(tariff, list)
        and all(_is_number(value) and value >= 0 for value in tariff)
        and all(a <= b for a, b in zip(tariff, tariff[1:]))
    ):
        findings.append("streak.skip_tariff_coins is not a rising list of coin prices")
    return examined, findings


def _streak_pressure(config, reference, context) -> tuple:
    findings = []
    examined = 0

    def need(condition: bool, message: str) -> None:
        nonlocal examined
        examined += 1
        if not condition:
            findings.append(message)

    start = _number(config, "streak.start_freezes")
    need(
        start is not None and start >= 1,
        "streak.start_freezes is below 1; a new streak starts forgivable",
    )
    days = get(config, "streak.days_per_freeze")
    need(
        isinstance(days, int) and not isinstance(days, bool) and days >= 1,
        "streak.days_per_freeze is not a positive whole number; freezes are earned by study",
    )
    shop = get(config, "shop")
    for name, item in (shop if isinstance(shop, dict) else {}).items():
        if isinstance(item, dict) and item.get("grants") == "freeze":
            need(
                item.get("currency") == "coins",
                f"shop.{name} sells a freeze for {item.get('currency')!r}; a way to keep a streak "
                "is never sold for money",
            )
    penalties = get(config, "penalties")
    for name, penalty in (penalties if isinstance(penalties, dict) else {}).items():
        need(
            isinstance(penalty, dict) and penalty.get("opt_in") is True,
            f"penalties.{name} is not opt-in; a penalty binds only an owner who armed it",
        )
    need(
        get(config, "governor.lapse_suppresses_penalties") is True,
        "governor.lapse_suppresses_penalties is not true; a lapse suppresses every penalty",
    )
    need(get(config, "wager.opt_in") is True, "wager.opt_in is not true")
    need(
        get(config, "wager.requires_armed_governor") is True,
        "wager.requires_armed_governor is not true; a wager is armed only when the habit is",
    )
    fraction = _number(config, "wager.stake_max_wallet_fraction")
    need(
        fraction is not None and 0 < fraction <= 0.5,
        f"wager.stake_max_wallet_fraction is {get(config, 'wager.stake_max_wallet_fraction')!r}; "
        "a stake is at most half the wallet",
    )
    voided = get(config, "wager.void_with_refund_on")
    voided = voided if isinstance(voided, list) else []
    for state in ("standby", "lapse"):
        need(
            state in voided,
            f"wager.void_with_refund_on omits {state}; a {state} voids the wager with a refund",
        )
    need(
        get(config, "wager.stakes_streak") is False,
        "wager.stakes_streak is not false; a wager stakes coins, never the streak itself",
    )
    relight = _number(config, "xp.bonuses.relight.xp")
    need(
        relight is not None and relight > 0,
        "xp.bonuses.relight.xp is not positive; coming back after a lapse is rewarded",
    )
    return examined, findings


JUDGES = {
    "economy-declared": _economy_declared,
    "xp-table": _xp_table,
    "tier-multiplier": _tier_multiplier,
    "xp-inflation": _xp_inflation,
    "chest-odds": _chest_odds,
    "odds-disclosed": _odds_disclosed,
    "chest-not-sold": _chest_not_sold,
    "coin-flows": _coin_flows,
    "coin-balance": _coin_balance,
    "loss-cap": _loss_cap,
    "only-coins-confiscable": _only_coins_confiscable,
    "streak-rules": _streak_rules,
    "streak-pressure": _streak_pressure,
}


def evaluate(
    class_id: str,
    root: "Path | str",
    config_path: str = DEFAULT_CONFIG,
    reference_path: "Path | str | None" = None,
    max_days: float = 30.0,
) -> tuple:
    """(examined, findings) of one class. Raises EconomyError when an input cannot be read."""
    if class_id not in JUDGES:
        raise ValueError(f"{class_id} is not a class of game-economy")
    reference = load_reference(reference_path)
    root = Path(root)
    where = root / config_path
    if class_id == "economy-declared":
        try:
            config = _read_json(where, "the economy")
        except EconomyError as error:
            return 1, [f"{error}; copy the pack's templates/economy.json to declare it"]
    else:
        config = _read_json(where, "the economy")
        if not isinstance(config, dict):
            raise EconomyError(f"the economy {where} is not a JSON object")
    context = {"root": root, "max_days": max_days}
    examined, found = JUDGES[class_id](config, reference, context)
    return examined, found


# --- golden: v9's numbers for the port's own tests ----------------------------------------------


def _round(value: float, mode: str) -> int:
    if mode == "half-even":
        return round(value)
    return int(math.copysign(math.floor(abs(value) + 0.5), value))


def golden(reference: dict) -> dict:
    """Per-review XP for every ease, maturity, type and tier, and the XP each level needs."""
    xp = reference["xp"]
    mode = xp["rounding"]
    if mode not in ROUNDING:
        raise EconomyError(f"xp.rounding {mode!r} is not one of {', '.join(ROUNDING)}")
    maturities = {
        key: value
        for key, value in xp["maturity"].items()
        if key != "mature_interval_days"
    }
    tiers = {"untagged": xp["tier"]["untagged"], **xp["tier"]["multipliers"]}
    rows = []
    for ease, ease_mult in xp["ease_multipliers"].items():
        for maturity, maturity_mult in maturities.items():
            for kind, type_mult in xp["type_multipliers"].items():
                for tier, tier_mult in tiers.items():
                    value = (
                        xp["base"] * ease_mult * maturity_mult * type_mult * tier_mult
                    )
                    rows.append(
                        {
                            "ease": ease,
                            "maturity": maturity,
                            "type": kind,
                            "tier": tier,
                            "xp": _round(value, mode),
                        }
                    )
    curve = xp["level_curve"]
    levels = [
        {
            "level": level,
            "xp": int(curve["quadratic"] * level * level + curve["linear"] * level),
        }
        for level in range(1, 101)
    ]
    return {
        "note": "v9 multiplies left to right, base x ease x maturity x type x tier, then rounds",
        "rounding": mode,
        "review_xp": rows,
        "levels": levels,
    }


def main(argv: "list | None" = None) -> int:
    parser = argparse.ArgumentParser(
        prog="game-economy-probe.py",
        description="A game economy checked against v9's exact math and its ethics (SPEC-V2-2224).",
    )
    parser.add_argument(
        "--root", type=Path, help="the repository whose economy is judged"
    )
    parser.add_argument(
        "--config", default=DEFAULT_CONFIG, help="the economy, relative to --root"
    )
    parser.add_argument(
        "--reference", type=Path, help="the reference economy (default: v9's)"
    )
    parser.add_argument(
        "--max-days-to-afford",
        type=float,
        default=30.0,
        help="coin-balance: days of the full daily mint a repeatable sink may cost (default 30)",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("classes", help="list the classes")
    commands.add_parser(
        "golden", help="print v9's per-review XP and level table as JSON"
    )
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    args = parser.parse_args(argv)
    if args.command == "classes":
        print("\n".join(CLASSES))
        return EXIT_GREEN
    if args.command == "golden":
        try:
            print(
                json.dumps(
                    golden(load_reference(args.reference)), ensure_ascii=False, indent=1
                )
            )
        except (EconomyError, KeyError, TypeError) as error:
            print(f"golden: VOID: {error}", file=sys.stderr)
            return EXIT_VOID
        return EXIT_GREEN
    if args.root is None:
        parser.error("check needs --root")
    try:
        examined, found = evaluate(
            args.name, args.root, args.config, args.reference, args.max_days_to_afford
        )
    except EconomyError as error:
        print(f"{args.name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    for finding in found:
        print(f"{args.name}: {finding}")
    print(f"examined {examined}")
    if examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if found else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())

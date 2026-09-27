#!/usr/bin/env python3
"""notifications-policy-probe: judges a repository's notification POLICY, one checkable config.

SPEC-V2-2219 / ADR-V2-2219, the check behind `skills/packs/notifications-policy`. It is
standard-library Python and vendorable, and it judges any tree through `--root`. The subject is
`notifications-policy.json` at the root (`phx.notifications.policy.v1`), which carries v9's
decided policy: the celebration ladder T0-T5, weekly budgets, the streak-break cap and dedupe,
quiet-hours deferral, the failed-send hold, nudge budgets, the comeback cap, the 10% holdout, the
withhold ledger, and ONE router shared by the bot and the Mini App. Three stages:

  policy     the config itself: its shape, its deviations from the baseline, its invariants (13);
  router     the tree: every transport call goes through the one router (1);
  messages   committed outbound envelopes (`*.msg.json`, nudge-duties' phx.duty.message.v1)
             read against the policy (1).

`--baseline` names the pack's `baseline.json`, v9's values; a value that differs from it must be
recorded in the config's `deviations` with an ADR. Output is one line per finding,
`<class>: <finding>`, then `examined N`. Exit 0 green, 1 a finding, 2 usage, 3 VOID: no policy,
or nothing examined, which is never a pass.

The policy binds runtime behaviour a static read cannot see (whether the engine consults the
budget, defers in quiet hours, holds a failed send); the pack's SKILL.md teaches that, and
SPEC-V2-2219's coverage matrix names each such practice and why it is excluded here.
"""

import argparse
import json
import re
import sys
from collections.abc import Callable, Iterator
from dataclasses import dataclass
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

POLICY_SCHEMA = "phx.notifications.policy.v1"
POLICY_FILE = "notifications-policy.json"
ENVELOPE_SCHEMA = "phx.duty.message.v1"
TIERS = ("T0", "T1", "T2", "T3", "T4", "T5")
RENDERS = ("silent", "reaction", "line", "reveal", "dice", "dice-pin")
KIND_CLASSES = frozenset({"celebration", "nudge", "digest", "alert"})
DEDUPE_SCOPES = frozenset(
    {"once-ever", "per-study-day", "per-episode-day", "per-incident"}
)
SURFACES = ("bot", "mini-app")
RARITIES = ("common", "rare", "epic", "legendary")
INTENSITY_ORDER = ("quiet", "standard", "loud")
HOLDOUT_CEILING = 50
COMEBACK_CEILING = 3
REQUIRED_REASONS = (
    "quiet_hours",
    "budget_spent",
    "lapse",
    "ablation_hold",
    "nudges_disabled",
)
SPECIAL_BUDGETS = frozenset({"celebration", "comeback"})
TOKEN = re.compile(r"^[a-z0-9][a-z0-9:._-]*$")
CALENDAR_DATE = re.compile(
    r"(?:19|20)\d{2}-?(?:0[1-9]|1[0-2])-?(?:0[1-9]|[12]\d|3[01])|\d{1,2}[./]\d{1,2}[./]\d{2,4}"
)
CLOCK = re.compile(r"^([01]\d|2[0-3]):([0-5]\d)$")

POLICY_SECTIONS = (
    "kinds",
    "ladder",
    "celebration_budgets",
    "streak_break",
    "near_miss",
    "quiet_hours",
    "digest",
    "deferral",
    "send_failure",
    "nudge_budgets",
    "lapse",
    "comeback",
    "holdout",
    "withhold",
)
TOP_KEYS = frozenset({"schema", "surfaces", "router", "deviations", *POLICY_SECTIONS})
KIND_KEYS = frozenset({"class", "tiers", "budget", "dedupe", "setting"})

POLICY_CLASSES = (
    "policy-declared",
    "policy-deviation-has-adr",
    "ladder-tiers",
    "celebration-budgets",
    "streak-break-cap",
    "dedupe",
    "quiet-hours",
    "deferral-bounds",
    "send-failure-hold",
    "nudge-budgets",
    "comeback-cap",
    "holdout",
    "withhold-ledger",
)
CLASSES = POLICY_CLASSES + ("one-router", "message-metadata")

SKIP_DIRS = frozenset(
    {
        ".git",
        "node_modules",
        "target",
        ".svelte-kit",
        "dist",
        "build",
        ".venv",
        "venv",
        "__pycache__",
        ".next",
        ".turbo",
        "coverage",
        ".output",
        ".cache",
    }
)
TEST_DIRS = frozenset(
    {"tests", "test", "__tests__", "e2e", "fixtures", "__mocks__", "testdata"}
)
TEST_FILE = re.compile(
    r"(?:^test_.*\.py$|_test\.py$|^conftest\.py$|\.(?:test|spec)\.[cm]?[jt]sx?$)"
)
SOURCE_EXTS = frozenset(
    {
        ".rs",
        ".py",
        ".ts",
        ".js",
        ".mjs",
        ".cjs",
        ".mts",
        ".tsx",
        ".jsx",
        ".svelte",
        ".vue",
        ".astro",
    }
)
PROBE_FILES = frozenset({"telegram-platform-probe.py", "notifications-policy-probe.py"})


class Void(Exception):
    """An input the class needs is absent, so it examined nothing."""


@dataclass(frozen=True, slots=True)
class Outcome:
    findings: tuple[str, ...]
    examined: int


@dataclass(frozen=True, slots=True)
class Inputs:
    root: Path
    policy_path: Path
    baseline_path: Path
    subject: Path | None


def load_policy(inputs: Inputs) -> dict:
    """The parsed policy, or Void when there is none or it cannot be read as a JSON object."""
    if not inputs.policy_path.is_file():
        raise Void(f"no {POLICY_FILE} at {inputs.policy_path.parent}")
    try:
        policy = json.loads(inputs.policy_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise Void(
            f"{POLICY_FILE} is not readable JSON ({type(error).__name__})"
        ) from None
    if not isinstance(policy, dict):
        raise Void(f"{POLICY_FILE} is not a JSON object")
    return policy


def _section(policy: dict, name: str, findings: list[str]) -> dict:
    value = policy.get(name)
    if not isinstance(value, dict):
        findings.append(f"section {name} is missing or not an object")
        return {}
    return value


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _tier_index(value: object) -> int | None:
    return TIERS.index(value) if isinstance(value, str) and value in TIERS else None


def _kinds(policy: dict) -> dict:
    kinds = policy.get("kinds")
    return kinds if isinstance(kinds, dict) else {}


# --------------------------------------------------------------------------- policy classes


def check_policy_declared(inputs: Inputs) -> Outcome:
    try:
        policy = load_policy(inputs)
    except Void as void:
        if inputs.policy_path.is_file():
            return Outcome((str(void) + ": the policy is not JSON",), 1)
        raise
    findings = []
    examined = 0
    if policy.get("schema") != POLICY_SCHEMA:
        findings.append(
            f"schema is {policy.get('schema')!r}; the policy must name {POLICY_SCHEMA}"
        )
    for key in sorted(set(policy) - TOP_KEYS):
        findings.append(f"unknown top-level key {key}")
    for name in POLICY_SECTIONS:
        examined += 1
        if not isinstance(policy.get(name), dict):
            findings.append(f"section {name} is missing or not an object")
    surfaces = policy.get("surfaces")
    if (
        not isinstance(surfaces, list)
        or not surfaces
        or not all(s in SURFACES for s in surfaces)
    ):
        findings.append(
            f"surfaces {surfaces!r} must list bot and/or mini-app, and nothing else"
        )
    if not isinstance(policy.get("router"), dict):
        findings.append("section router is missing or not an object")
    if not isinstance(policy.get("deviations", []), list):
        findings.append("deviations must be an array")
    kinds = policy.get("kinds")
    if isinstance(kinds, dict) and not kinds:
        findings.append("kinds is empty: a policy declares every kind it sends")
    for name, kind in _kinds(policy).items():
        examined += 1
        if not isinstance(kind, dict):
            findings.append(f"kind {name} is not an object")
            continue
        for key in sorted(set(kind) - KIND_KEYS):
            findings.append(f"kind {name} has unknown key {key}")
        if kind.get("class") not in KIND_CLASSES:
            findings.append(
                f"kind {name} class {kind.get('class')!r} is not celebration, nudge, digest or alert"
            )
        tiers = kind.get("tiers")
        if (
            not isinstance(tiers, list)
            or not tiers
            or not all(t in TIERS for t in tiers)
        ):
            findings.append(
                f"kind {name} tiers {tiers!r} must be a non-empty list of T0-T5"
            )
        if kind.get("budget") is not None and not isinstance(kind.get("budget"), str):
            findings.append(f"kind {name} budget must be a name or null")
        if kind.get("setting") is not None and not isinstance(kind.get("setting"), str):
            findings.append(f"kind {name} setting must be a settings key or null")
        elif kind.get("class") != "alert" and not kind.get("setting"):
            findings.append(
                f"kind {name} names no setting: the owner switches every kind but an alert "
                "off inside the app"
            )
    return Outcome(tuple(findings), examined)


def _leaves(value: object, prefix: str) -> Iterator[tuple[str, object]]:
    if isinstance(value, dict) and value:
        for key, inner in value.items():
            yield from _leaves(inner, f"{prefix}.{key}" if prefix else str(key))
    else:
        yield prefix, value


def check_deviation_has_adr(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    try:
        baseline = json.loads(inputs.baseline_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError):
        raise Void(
            f"the baseline {inputs.baseline_path} is not readable JSON"
        ) from None
    ours = dict(_leaves({name: policy.get(name) for name in POLICY_SECTIONS}, ""))
    theirs = dict(_leaves({name: baseline.get(name) for name in POLICY_SECTIONS}, ""))
    differing = sorted(
        key
        for key in set(ours) | set(theirs)
        if ours.get(key, KeyError) != theirs.get(key, KeyError)
    )
    findings = []
    deviations = policy.get("deviations", [])
    covered: set[str] = set()
    root = inputs.root.resolve()
    for index, deviation in enumerate(
        deviations if isinstance(deviations, list) else []
    ):
        if (
            not isinstance(deviation, dict)
            or not isinstance(deviation.get("key"), str)
            or not isinstance(deviation.get("adr"), str)
        ):
            findings.append(f"deviation {index} needs a key and an adr path")
            continue
        key, adr = deviation["key"], deviation["adr"]
        matches = [d for d in differing if d == key or d.startswith(key + ".")]
        if not matches:
            findings.append(
                f"deviation {key} is stale: that value no longer differs from the baseline"
            )
        path = (root / adr).resolve()
        if root not in path.parents:
            findings.append(f"deviation {key} cites {adr}, which lies outside the tree")
            continue
        if not path.is_file():
            findings.append(f"deviation {key} cites {adr}, which does not exist")
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if key not in text:
            findings.append(f"deviation {key} cites {adr}, which never names {key}")
            continue
        covered.update(matches)
    for key in differing:
        if key not in covered:
            base = theirs.get(key, "(absent)")
            now = ours.get(key, "(absent)")
            findings.append(
                f"{key} is {json.dumps(now)} where the baseline has {json.dumps(base)}, and no "
                "deviation with an ADR records it"
            )
    return Outcome(tuple(findings), len(set(ours) | set(theirs)))


def check_ladder_tiers(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    ladder = _section(policy, "ladder", findings)
    tiers = ladder.get("tiers") if isinstance(ladder.get("tiers"), dict) else {}
    for tier in TIERS:
        if tier not in tiers:
            findings.append(f"ladder tier {tier} is not declared")
    renders = [tiers.get(tier) for tier in TIERS if tier in tiers]
    for tier in TIERS:
        if tier in tiers and tiers[tier] not in RENDERS:
            findings.append(
                f"ladder tier {tier} render {tiers[tier]!r} is not one of {', '.join(RENDERS)}"
            )
    known = [RENDERS.index(render) for render in renders if render in RENDERS]
    if known != sorted(known) or len(set(known)) != len(known):
        findings.append(
            "ladder renders are out of order: intensity must rise from T0 to T5"
        )
    rarity = ladder.get("rarity") if isinstance(ladder.get("rarity"), dict) else {}
    indices = []
    for name in RARITIES:
        index = _tier_index(rarity.get(name))
        if index is None:
            findings.append(f"ladder rarity {name} has no tier")
        else:
            indices.append(index)
    if indices != sorted(indices):
        findings.append(
            "ladder rarity tiers must not fall as rarity rises (common, rare, epic, legendary)"
        )
    floors = (
        ladder.get("rarity_floor")
        if isinstance(ladder.get("rarity_floor"), dict)
        else {}
    )
    for name in ("epic", "legendary"):
        index = _tier_index(floors.get(name))
        if index is None or index < TIERS.index("T2"):
            findings.append(
                f"ladder rarity_floor {name} is {floors.get(name)!r}; the floor is T2 or higher"
            )
    events = ladder.get("events") if isinstance(ladder.get("events"), dict) else {}
    if not events:
        findings.append("ladder events is empty or missing")
    for name, tier in events.items():
        if _tier_index(tier) is None:
            findings.append(f"ladder event {name} tier {tier!r} is not T0-T5")
    if _tier_index(ladder.get("unknown_event")) is None:
        findings.append(
            f"ladder unknown_event {ladder.get('unknown_event')!r} is not T0-T5"
        )
    age = ladder.get("reaction_max_age_hours")
    if not _is_int(age) or age < 1:
        findings.append(
            f"ladder reaction_max_age_hours {age!r} must be a positive integer"
        )
    if ladder.get("dedupe") != "once-ever":
        findings.append(
            f"ladder dedupe is {ladder.get('dedupe')!r}; a celebration fires once-ever per event key"
        )
    return Outcome(tuple(findings), len(TIERS) + len(RARITIES) + len(events))


def check_celebration_budgets(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    budgets = _section(policy, "celebration_budgets", findings)
    if budgets.get("window") != "week":
        findings.append(
            f"celebration budgets window is {budgets.get('window')!r}; the budget is per week"
        )
    intensities = (
        budgets.get("intensities")
        if isinstance(budgets.get("intensities"), dict)
        else {}
    )
    if not intensities:
        findings.append("celebration budgets declare no intensities")
    for name, caps in intensities.items():
        if not isinstance(caps, dict) or not all(
            _is_int(caps.get(t)) and caps[t] >= 0 for t in ("T4", "T5")
        ):
            findings.append(
                f"intensity {name} needs non-negative integer T4 and T5 budgets"
            )
            continue
        if caps["T5"] > caps["T4"]:
            findings.append(
                f"intensity {name} allows more T5 ({caps['T5']}) than T4 ({caps['T4']})"
            )
    ordered = [
        name for name in INTENSITY_ORDER if isinstance(intensities.get(name), dict)
    ]
    for lower, higher in zip(ordered, ordered[1:], strict=False):
        for tier in ("T4", "T5"):
            low, high = intensities[lower].get(tier), intensities[higher].get(tier)
            if _is_int(low) and _is_int(high) and low > high:
                findings.append(
                    f"intensity {higher} allows fewer {tier} ({high}) than {lower} ({low})"
                )
    if budgets.get("default_intensity") not in intensities:
        findings.append(
            f"default_intensity {budgets.get('default_intensity')!r} is not a declared intensity"
        )
    downgrade = (
        budgets.get("downgrade") if isinstance(budgets.get("downgrade"), dict) else {}
    )
    for tier in ("T5", "T4"):
        target = _tier_index(downgrade.get(tier))
        if target is None or target >= TIERS.index(tier):
            findings.append(
                f"downgrade of {tier} is {downgrade.get(tier)!r}; an over-budget tier drops to a lower one"
            )
    ladder = policy.get("ladder") if isinstance(policy.get("ladder"), dict) else {}
    events = ladder.get("events") if isinstance(ladder.get("events"), dict) else {}
    exempt = budgets.get("exempt_events")
    if not isinstance(exempt, list):
        findings.append("exempt_events must be an array of ladder events")
        exempt = []
    for name in exempt:
        if name not in events:
            findings.append(f"exempt event {name!r} is not a ladder event")
    return Outcome(tuple(findings), len(intensities) + len(downgrade) + 1)


def check_streak_break_cap(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    streak = _section(policy, "streak_break", findings)
    deferral = _section(policy, "deferral", findings)
    near = _section(policy, "near_miss", findings)
    cap = _tier_index(streak.get("cap"))
    if cap is None or cap > TIERS.index("T1"):
        findings.append(
            f"streak_break cap is {streak.get('cap')!r}; on a streak-break day everything caps at "
            "T1 (no loss disguised as a win)"
        )
    if streak.get("defer_fanfare") is not True:
        findings.append(
            "streak_break defer_fanfare must be true: fanfare waits for a real good day"
        )
    if deferral.get("recap_at_flush") is not True:
        findings.append(
            "deferral recap_at_flush must be true: a stored T5 must not replay onto a streak-break day"
        )
    units, fraction = near.get("max_units"), near.get("max_fraction")
    if not (_is_int(units) and units >= 1) or not (
        isinstance(fraction, (int, float))
        and not isinstance(fraction, bool)
        and 0 < fraction < 1
    ):
        findings.append(
            f"near_miss {near!r} must name a positive unit gap and a fraction between 0 and 1: "
            "near-miss copy only for real progress gaps"
        )
    return Outcome(tuple(findings), 4)


def check_dedupe(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings = []
    kinds = _kinds(policy)
    for name, kind in kinds.items():
        if not isinstance(kind, dict):
            continue
        scope = kind.get("dedupe")
        if scope is None:
            findings.append(f"kind {name} declares no dedupe scope")
        elif scope not in DEDUPE_SCOPES:
            findings.append(
                f"kind {name} dedupe {scope!r} is not one of {', '.join(sorted(DEDUPE_SCOPES))}"
            )
        elif kind.get("class") == "celebration" and scope != "once-ever":
            findings.append(
                f"celebration kind {name} dedupes {scope}; a celebration fires once-ever per event key"
            )
    return Outcome(tuple(findings), len(kinds))


def _minutes(value: object) -> int | None:
    match = CLOCK.match(value) if isinstance(value, str) else None
    return int(match.group(1)) * 60 + int(match.group(2)) if match else None


def _inside(minute: int, start: int, end: int) -> bool:
    if start == end:
        return False
    if start < end:
        return start <= minute < end
    return minute >= start or minute < end


def check_quiet_hours(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    quiet = _section(policy, "quiet_hours", findings)
    digest = _section(policy, "digest", findings)
    start, end = _minutes(quiet.get("start")), _minutes(quiet.get("end"))
    for key, value in (("start", start), ("end", end)):
        if value is None:
            findings.append(
                f"quiet_hours {key} {quiet.get(key)!r} is not a HH:MM clock time"
            )
    if quiet.get("celebrations") != "defer":
        findings.append(
            f"quiet_hours celebrations is {quiet.get('celebrations')!r}; a celebration in quiet hours "
            "must defer, never drop and never fire"
        )
    if quiet.get("nudges") != "suppress":
        findings.append(
            f"quiet_hours nudges is {quiet.get('nudges')!r}; a nudge in quiet hours must suppress"
        )
    exempt = quiet.get("exempt_classes")
    if not isinstance(exempt, list):
        findings.append("quiet_hours exempt_classes must be an array")
        exempt = []
    for name in exempt:
        if name != "alert":
            findings.append(
                f"quiet_hours exempts class {name!r}; only alerts may ignore quiet hours"
            )
    at, rollover = _minutes(digest.get("at")), _minutes(digest.get("rollover"))
    if at is None or rollover is None:
        findings.append(f"digest at and rollover {digest!r} must be HH:MM clock times")
    else:
        if start is not None and end is not None and _inside(at, start, end):
            findings.append(f"the digest at {digest['at']} fires inside quiet hours")
        if at < rollover:
            findings.append(
                f"the digest at {digest['at']} fires before the rollover at {digest['rollover']}, "
                "before the day it reports has closed"
            )
    return Outcome(tuple(findings), 3 + len(exempt))


def check_deferral_bounds(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    deferral = _section(policy, "deferral", findings)
    for key in ("max_age_minutes", "flush_max", "queue_max"):
        value = deferral.get(key)
        if not _is_int(value) or value < 1:
            findings.append(f"deferral {key} {value!r} must be a positive integer")
    flush, queue = deferral.get("flush_max"), deferral.get("queue_max")
    if _is_int(flush) and _is_int(queue) and queue < flush:
        findings.append(f"deferral queue_max {queue} is below flush_max {flush}")
    if deferral.get("overflow") != "rollup":
        findings.append(
            f"deferral overflow is {deferral.get('overflow')!r}; what exceeds flush_max collapses "
            "into one rollup line"
        )
    if deferral.get("named_drops") is not True:
        findings.append(
            "deferral named_drops must be true: a dropped celebration is named, never silent"
        )
    return Outcome(tuple(findings), 5)


def check_send_failure_hold(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    failure = _section(policy, "send_failure", findings)
    if failure.get("hold") is not True:
        findings.append(
            "send_failure hold must be true: a failed send holds the celebration, it does not burn it"
        )
    for key in ("retry_max", "outage_cooldown_ms"):
        value = failure.get(key)
        if not _is_int(value) or value < 1:
            findings.append(f"send_failure {key} {value!r} must be a positive integer")
    if failure.get("keep_first_deferred_at") is not True:
        findings.append(
            "send_failure keep_first_deferred_at must be true: a re-latched row keeps its first "
            "timestamp, or the age cap is never reached"
        )
    return Outcome(tuple(findings), 4)


def check_nudge_budgets(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    budgets = _section(policy, "nudge_budgets", findings)
    lapse = _section(policy, "lapse", findings)
    for name, budget in budgets.items():
        if not isinstance(budget, dict):
            findings.append(f"nudge budget {name} is not an object")
            continue
        per_day = budget.get("per_day")
        if not _is_int(per_day) or per_day < 1:
            findings.append(
                f"nudge budget {name} per_day {per_day!r} must be a positive integer"
            )
        exempt = budget.get("exempt", [])
        if not isinstance(exempt, list):
            findings.append(f"nudge budget {name} exempt must be an array")
        elif exempt and budget.get("exempt_checked_first") is not True:
            findings.append(
                f"nudge budget {name} exempt_checked_first must be true: an exempt consumer is "
                "decided before the budget is consulted"
            )
    named = set()
    for kind_name, kind in _kinds(policy).items():
        budget = kind.get("budget") if isinstance(kind, dict) else None
        if budget is None:
            continue
        named.add(budget)
        if budget not in budgets and budget not in SPECIAL_BUDGETS:
            findings.append(
                f"kind {kind_name} names budget {budget!r}, which no section declares"
            )
    for name in budgets:
        if name not in named:
            findings.append(f"nudge budget {name} is named by no kind")
    if not _is_int(lapse.get("after_zero_days")) or lapse.get("after_zero_days", 0) < 1:
        findings.append(
            f"lapse after_zero_days {lapse.get('after_zero_days')!r} must be a positive integer"
        )
    suppress = lapse.get("suppress_classes")
    if not isinstance(suppress, list) or "nudge" not in suppress:
        findings.append(
            "lapse suppress_classes must include nudge: a lapse suppresses every nudge"
        )
    if lapse.get("escalation") != "none":
        findings.append(
            f"lapse escalation is {lapse.get('escalation')!r}; no escalating nudge ladder on a lapsed "
            "owner"
        )
    backoff = (
        lapse.get("habit_backoff")
        if isinstance(lapse.get("habit_backoff"), dict)
        else {}
    )
    for key in ("after_days", "interval_days"):
        if not _is_int(backoff.get(key)) or backoff.get(key, 0) < 1:
            findings.append(
                f"lapse habit_backoff {key} {backoff.get(key)!r} must be a positive integer"
            )
    return Outcome(tuple(findings), len(budgets) + len(_kinds(policy)) + 1)


def check_comeback_cap(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    comeback = _section(policy, "comeback", findings)
    most = comeback.get("max_per_episode")
    if not _is_int(most) or not 1 <= most <= COMEBACK_CEILING:
        findings.append(
            f"comeback max_per_episode {most!r} must be 1-{COMEBACK_CEILING}: at most "
            f"{COMEBACK_CEILING} comeback messages per lapse"
        )
    gap = comeback.get("min_gap_days")
    if not _is_int(gap) or gap < 1:
        findings.append(f"comeback min_gap_days {gap!r} must be a positive integer")
    if not isinstance(comeback.get("episode_key"), str) or not comeback.get(
        "episode_key"
    ):
        findings.append(
            "comeback episode_key must name the lapse id a comeback is counted under"
        )
    if comeback.get("after_cap") != "silence":
        findings.append(
            f"comeback after_cap is {comeback.get('after_cap')!r}; after the cap comes silence"
        )
    landmarks = (
        comeback.get("landmarks") if isinstance(comeback.get("landmarks"), dict) else {}
    )
    for key in ("monday_window_days", "month_start_window_days"):
        if not _is_int(landmarks.get(key)) or landmarks.get(key, -1) < 0:
            findings.append(
                f"comeback landmarks {key} {landmarks.get(key)!r} must be a non-negative integer"
            )
    kind = _kinds(policy).get("comeback")
    if (
        not isinstance(kind, dict)
        or kind.get("class") != "nudge"
        or kind.get("budget") != "comeback"
    ):
        findings.append(
            "the comeback kind must exist as a nudge whose budget is comeback"
        )
    return Outcome(tuple(findings), 6)


def check_holdout(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    holdout = _section(policy, "holdout", findings)
    ceiling, pct = holdout.get("max_pct"), holdout.get("pct")
    if not _is_int(ceiling) or not 0 <= ceiling <= HOLDOUT_CEILING:
        findings.append(f"holdout max_pct {ceiling!r} must be 0-{HOLDOUT_CEILING}")
        ceiling = HOLDOUT_CEILING
    if (
        not isinstance(pct, (int, float))
        or isinstance(pct, bool)
        or not 0 <= pct <= ceiling
    ):
        findings.append(f"holdout pct {pct!r} must be between 0 and max_pct {ceiling}")
    if holdout.get("draw") != "hash":
        findings.append(
            f"holdout draw is {holdout.get('draw')!r}; the arm is a deterministic hash of the seed, "
            "kind and reference, so every past decision can be re-derived"
        )
    if holdout.get("seed") != "persisted":
        findings.append(
            f"holdout seed is {holdout.get('seed')!r}; the seed is minted once and persisted"
        )
    kinds = _kinds(policy)
    eligible = holdout.get("kinds")
    if not isinstance(eligible, list) or not eligible:
        findings.append("holdout kinds must be a non-empty allowlist of nudge kinds")
        eligible = []
    for name in eligible:
        kind = kinds.get(name)
        if not isinstance(kind, dict):
            findings.append(f"holdout kind {name!r} is not a declared kind")
        elif kind.get("class") != "nudge":
            findings.append(
                f"holdout kind {name} is a {kind.get('class')}; only advisory nudges may be held out"
            )
    if holdout.get("record_held") is not True:
        findings.append(
            "holdout record_held must be true: a held occasion is recorded with its arm"
        )
    if holdout.get("held_consumes_budget") is not False:
        findings.append(
            "holdout held_consumes_budget must be false: a held nudge spends no budget"
        )
    minimum = holdout.get("readout_min_n")
    if not _is_int(minimum) or minimum < 1:
        findings.append(f"holdout readout_min_n {minimum!r} must be a positive integer")
    return Outcome(tuple(findings), 6 + len(eligible))


def check_withhold_ledger(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    withhold = _section(policy, "withhold", findings)
    if withhold.get("record") is not True:
        findings.append(
            "withhold record must be true: every withheld occasion is recorded with its reason"
        )
    suffix = withhold.get("kind_suffix")
    if not isinstance(suffix, str) or not suffix:
        findings.append(
            "withhold kind_suffix must be a non-empty suffix: a withhold row never takes the send's own key"
        )
    reasons = withhold.get("reasons")
    if not isinstance(reasons, list) or not all(
        isinstance(r, str) and r for r in reasons
    ):
        findings.append("withhold reasons must be an array of reason tokens")
        reasons = []
    for reason in sorted({r for r in reasons if reasons.count(r) > 1}):
        findings.append(f"withhold reason {reason} is listed twice")
    for reason in REQUIRED_REASONS:
        if reason not in reasons:
            findings.append(
                f"withhold reasons lack {reason}, which the policy's own rules withhold for"
            )
    return Outcome(tuple(findings), len(reasons) + 2)


# --------------------------------------------------------------------------- router class


def _strip(text: str, ext: str) -> str:
    """`text` with comments blanked (line and block; `#` in Python), positions kept."""
    if ext == ".py":
        return re.sub(r"(?m)#[^\n]*", lambda m: " " * len(m.group(0)), text)
    out = re.sub(
        r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), text, flags=re.S
    )
    out = re.sub(r"(?m)(?<!:)//[^\n]*", lambda m: " " * len(m.group(0)), out)
    return re.sub(
        r"<!--.*?-->", lambda m: re.sub(r"[^\n]", " ", m.group(0)), out, flags=re.S
    )


def _walk(root: Path) -> Iterator[Path]:
    stack = [root]
    while stack:
        directory = stack.pop()
        try:
            entries = sorted(directory.iterdir(), key=lambda path: path.name)
        except OSError:
            continue
        for entry in entries:
            if entry.is_symlink():
                continue
            if entry.is_dir():
                if entry.name not in SKIP_DIRS:
                    stack.append(entry)
            elif entry.is_file():
                yield entry


def _definition(code: str, start: int, name: str) -> bool:
    before = code[max(0, start - 40) : start]
    if re.search(r"(?:\bfn|\bdef|\bfunction|\basync\s+fn)\s+$", before):
        return True
    after = code[start:]
    return bool(
        re.match(
            rf"{re.escape(name)}\s*\([^)]*\)\s*(?::\s*[\w<>\[\], |.]+)?\s*\{{", after
        )
    ) and bool(
        re.search(r"(?:^|\n)\s*(?:pub\s+|export\s+|async\s+|static\s+)*$", before)
    )


def check_one_router(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    findings: list[str] = []
    router = _section(policy, "router", findings)
    surfaces = (
        policy.get("surfaces") if isinstance(policy.get("surfaces"), list) else []
    )
    transport = (
        router.get("transport") if isinstance(router.get("transport"), dict) else {}
    )
    for surface in SURFACES:
        if surface not in surfaces:
            findings.append(
                f"surfaces lacks {surface}: ONE router serves the bot and the Mini App, or a "
                "celebration renders twice"
            )
        calls = transport.get(surface)
        if (
            not isinstance(calls, list)
            or not calls
            or not all(isinstance(c, str) and c for c in calls)
        ):
            findings.append(f"router transport names no {surface} delivery call")
    module_rel = router.get("module")
    symbol = router.get("symbol")
    root = inputs.root.resolve()
    module = (root / module_rel).resolve() if isinstance(module_rel, str) else None
    if module is None or root not in module.parents or not module.is_file():
        findings.append(f"router module {module_rel!r} does not exist in the tree")
    elif not isinstance(symbol, str) or not re.search(
        rf"\b(?:fn|def|function)\s+{re.escape(symbol)}\b|\b(?:const|let|var)\s+{re.escape(symbol)}\s*=",
        _strip(module.read_text(encoding="utf-8", errors="replace"), module.suffix),
    ):
        findings.append(f"router module {module_rel} does not define {symbol!r}")
    names = sorted(
        {
            c
            for calls in transport.values()
            if isinstance(calls, list)
            for c in calls
            if isinstance(c, str) and c
        }
    )
    examined = 0
    for path in _walk(inputs.root):
        parts = path.relative_to(inputs.root).parts
        if path.suffix not in SOURCE_EXTS or path.name in PROBE_FILES:
            continue
        if any(part in TEST_DIRS for part in parts[:-1]) or TEST_FILE.search(parts[-1]):
            continue
        examined += 1
        if module is not None and path.resolve() == module:
            continue
        try:
            code = _strip(path.read_text(encoding="utf-8"), path.suffix)
        except (OSError, UnicodeDecodeError):
            continue
        for name in names:
            for match in re.finditer(rf"(?<![\w$]){re.escape(name)}\s*\(", code):
                if _definition(code, match.start(), name):
                    continue
                line = code.count("\n", 0, match.start()) + 1
                findings.append(
                    f"{'/'.join(parts)}:{line} calls {name} outside the router module {module_rel}"
                )
    return Outcome(
        tuple(findings), max(examined, 1) if module is not None else examined
    )


# --------------------------------------------------------------------------- messages class


def _envelopes(base: Path) -> tuple[list[tuple[str, object]], int]:
    found = []
    walked = 0
    for path in _walk(base):
        walked += 1
        if not path.name.endswith(".msg.json"):
            continue
        rel = "/".join(path.relative_to(base).parts)
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, ValueError):
            found.append((rel, None))
            continue
        if isinstance(document, dict) and (
            document.get("schema") == ENVELOPE_SCHEMA or "kind" in document
        ):
            found.append((rel, document))
    return found, walked


def check_message_metadata(inputs: Inputs) -> Outcome:
    policy = load_policy(inputs)
    kinds = _kinds(policy)
    comeback = (
        policy.get("comeback") if isinstance(policy.get("comeback"), dict) else {}
    )
    cap = (
        comeback.get("max_per_episode")
        if _is_int(comeback.get("max_per_episode"))
        else COMEBACK_CEILING
    )
    subject = inputs.subject is not None
    envelopes, walked = _envelopes(inputs.subject if subject else inputs.root)
    findings = []
    seen: dict[str, str] = {}
    episodes: dict[str, list[str]] = {}
    for rel, envelope in envelopes:
        if not isinstance(envelope, dict):
            findings.append(f"{rel}: not readable JSON")
            continue
        name = envelope.get("kind")
        kind = kinds.get(name) if isinstance(name, str) else None
        if not isinstance(kind, dict):
            findings.append(f"{rel}: kind {name!r} is not a kind the policy declares")
            continue
        tiers = kind.get("tiers") if isinstance(kind.get("tiers"), list) else []
        if envelope.get("tier") not in tiers:
            findings.append(
                f"{rel}: tier {envelope.get('tier')!r} is not one of kind {name}'s tiers {tiers}"
            )
        budget = kind.get("budget")
        if budget is None and "budget_key" in envelope:
            findings.append(
                f"{rel}: budget_key is set on kind {name}, which the policy declares budget-exempt"
            )
        elif budget is not None and envelope.get("budget_key") != budget:
            findings.append(
                f"{rel}: budget_key {envelope.get('budget_key')!r} must name kind {name}'s budget {budget}"
            )
        for key in ("dedupe_key", "budget_key", "lapse_id"):
            value = envelope.get(key)
            if value is None and (key != "dedupe_key"):
                continue
            if not isinstance(value, str) or not TOKEN.match(value):
                findings.append(
                    f"{rel}: {key} {value!r} is not an opaque token of [a-z0-9][a-z0-9:._-]*"
                )
            elif CALENDAR_DATE.search(value):
                findings.append(
                    f"{rel}: {key} {value!r} carries a calendar date; keys are opaque"
                )
        dedupe = envelope.get("dedupe_key")
        if isinstance(dedupe, str):
            if dedupe in seen:
                findings.append(
                    f"{rel}: dedupe_key {dedupe} is used twice (also {seen[dedupe]})"
                )
            else:
                seen[dedupe] = rel
        if name == "comeback":
            lapse = envelope.get("lapse_id")
            if not isinstance(lapse, str) or not lapse:
                findings.append(f"{rel}: a comeback carries no lapse_id")
            else:
                episodes.setdefault(lapse, []).append(rel)
    for lapse, members in sorted(episodes.items()):
        if len(members) > cap:
            findings.append(
                f"lapse {lapse} has {len(members)} comeback messages; the policy caps an episode at {cap}"
            )
    return Outcome(tuple(findings), len(envelopes) if subject else walked)


CHECKS: dict[str, Callable[[Inputs], Outcome]] = {
    "policy-declared": check_policy_declared,
    "policy-deviation-has-adr": check_deviation_has_adr,
    "ladder-tiers": check_ladder_tiers,
    "celebration-budgets": check_celebration_budgets,
    "streak-break-cap": check_streak_break_cap,
    "dedupe": check_dedupe,
    "quiet-hours": check_quiet_hours,
    "deferral-bounds": check_deferral_bounds,
    "send-failure-hold": check_send_failure_hold,
    "nudge-budgets": check_nudge_budgets,
    "comeback-cap": check_comeback_cap,
    "holdout": check_holdout,
    "withhold-ledger": check_withhold_ledger,
    "one-router": check_one_router,
    "message-metadata": check_message_metadata,
}


# --------------------------------------------------------------------------- command line


def run_class(name: str, inputs: Inputs) -> int:
    try:
        outcome = CHECKS[name](inputs)
    except Void as void:
        print(f"{name}: VOID {void}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(
        prog="notifications-policy-probe",
        description="Judge a repository's notification policy, its router and its envelopes.",
    )
    top.add_argument("--root", type=Path, default=None)
    top.add_argument("--policy", type=Path, default=None)
    top.add_argument("--baseline", type=Path, default=None)
    top.add_argument("--subject", type=Path, default=None)
    verbs = top.add_subparsers(dest="verb", required=True)
    verbs.add_parser("classes", help="list the classes")
    check = verbs.add_parser("check", help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for name in CLASSES:
            print(name)
        return EXIT_GREEN
    root = args.root if args.root is not None else Path.cwd()
    if not root.is_dir():
        print(
            f"notifications-policy-probe: --root {root} is not a directory",
            file=sys.stderr,
        )
        return EXIT_USAGE
    if args.subject is not None and not args.subject.is_dir():
        print(
            f"notifications-policy-probe: --subject {args.subject} is not a directory",
            file=sys.stderr,
        )
        return EXIT_USAGE
    baseline = args.baseline or (
        Path(__file__).resolve().parent.parent
        / "skills"
        / "packs"
        / "notifications-policy"
        / "baseline.json"
    )
    inputs = Inputs(root, args.policy or root / POLICY_FILE, baseline, args.subject)
    return run_class(args.klass, inputs)


if __name__ == "__main__":
    sys.exit(main())

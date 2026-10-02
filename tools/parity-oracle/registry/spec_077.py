"""SPEC-077's registrations: the memory-state parse, card mastery, the unit parse, course progress,
the band-up, the law mastery pillar, the law track summary, the law block's shown rule and the
curriculum constants.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_synthetic_courses` patches `progress.LANGUAGE_DECKS` and `progress.CEFR_UNIT_BANDS`, the
  names `progress` imported into its own namespace, with the case's synthetic courses, then calls
  `progress.compute_progress` over cards it builds.
* `persist_progress` drives `pipeline.py:GamifyPipeline._persist_progress` (and its
  `_celebrate_band_up`) on a stub store that keeps the stored bands and the milestones, with
  `progress.compute_progress` patched to answer the case's per-course bands; the milestone set is
  the stub's whole rule (a key recorded once), never the band comparison.
* `law_summary` drives `DigestsLayer.law_track_summary` over a temporary store the case's ledger rows
  fill through its own `upsert_xp_grant`, so the predecessor's own SQL sums them.
* `law_block_fields` calls `telegram.render_law_block` and returns which fields' lines it holds,
  never its text.

The case builders draw only from the `random.Random` the generator seeds. Every number is
synthetic, every day an epoch day number, and every deck and course name is made up.
"""

import asyncio
import datetime as dt
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

EPOCH_ORDINAL = dt.date(1970, 1, 1).toordinal()
#: The deck name's separator, as the predecessor spells it.
SEP = "\x1f"
BANDS = ("A1", "A2", "B1", "B2", "C1", "C2")
NOW = 1_700_000_000
DAY = 86_400


def as_date(day):
    """The `date` of epoch day number `day`."""
    return dt.date.fromordinal(EPOCH_ORDINAL + day)


# --- the memory state -----------------------------------------------------------------------------


def memory_state_cases(rng):
    texts = [
        ("missing", {"data": None}),
        ("missing", {"data": ""}),
        ("unparsable", {"data": "not json"}),
        ("unparsable", {"data": '{"s": 4.5'}),
        ("non_object", {"data": "[1, 2]"}),
        ("non_object", {"data": "7"}),
        ("non_object", {"data": '"s"'}),
        ("non_object", {"data": "null"}),
        ("no_stability", {"data": "{}"}),
        ("no_stability", {"data": '{"d": 5.0}'}),
        ("stability_not_a_number", {"data": '{"s": "4.5"}'}),
        ("stability_not_a_number", {"data": '{"s": null}'}),
        ("stability_not_positive", {"data": '{"s": 0}'}),
        ("stability_not_positive", {"data": '{"s": -3.5}'}),
        ("non_finite", {"data": '{"s": NaN}'}),
        ("non_finite", {"data": '{"s": Infinity}'}),
        ("non_finite", {"data": '{"s": -Infinity}'}),
        ("non_finite", {"data": '{"s": 1e400}'}),
        ("non_finite", {"data": '{"s": ' + "9" * 400 + "}"}),
        ("non_finite_field", {"data": '{"s": 4.5, "d": NaN, "decay": Infinity, "lrt": -Infinity}'}),
        ("non_finite_field", {"data": '{"s": 4.5, "d": 1e400, "dr": 1e999, "lrt": 1e400}'}),
        ("a_boolean_is_a_number", {"data": '{"s": true}'}),
        ("a_boolean_is_a_number", {"data": '{"s": 4.5, "d": true, "dr": false}'}),
        ("decay_default", {"data": '{"s": 4.5}'}),
        ("decay_default", {"data": '{"s": 4.5, "decay": 0}'}),
        ("decay_default", {"data": '{"s": 4.5, "decay": -0.3}'}),
        ("decay_default", {"data": '{"s": 4.5, "decay": null}'}),
        ("decay_default", {"data": '{"s": 4.5, "decay": "0.3"}'}),
        ("decay_kept", {"data": '{"s": 4.5, "decay": 0.126}'}),
        ("decay_kept", {"data": '{"s": 4.5, "decay": 1e-9}'}),
        ("full", {"data": '{"s": 12.25, "d": 6.5, "dr": 0.9, "decay": 0.2, "lrt": 1700000000}'}),
        ("last_review_truncates", {"data": '{"s": 2, "lrt": 1700000000.9}'}),
        ("last_review_truncates", {"data": '{"s": 2, "lrt": -5.9}'}),
        ("last_review_zero", {"data": '{"s": 2, "lrt": 0}'}),
        ("last_review_missing", {"data": '{"s": 2, "dr": 0.85}'}),
        ("integer_stability", {"data": '{"s": 30}'}),
        ("a_string_with_a_quote", {"data": '{"note": "NaN \\" Infinity", "s": 3.5}'}),
        ("a_name_is_not_a_number", {"data": '{"NaN": 1, "s": 3.5}'}),
        ("repeated_key", {"data": '{"s": 3.5, "s": 7.5}'}),
        ("tiny_stability", {"data": '{"s": 1e-300}'}),
        ("surrounding_space", {"data": ' \n{"s": 5.5}\t'}),
        ("trailing_text", {"data": '{"s": 5.5} x'}),
    ]
    for _ in range(10):
        stability = round(rng.uniform(0.05, 400.0), 3)
        decay = rng.choice([0.1, 0.15, 0.2, 0.9, 2.5])
        text = (
            f'{{"s": {stability}, "d": {round(rng.uniform(1, 10), 2)}, "dr": '
            f'{rng.choice([0.8, 0.9, 0.95])}, "decay": {decay}, "lrt": {rng.randrange(0, 2 * NOW)}}}'
        )
        texts.append((None, {"data": text}))
    return texts


def memory_state_fields(parse_fsrs, predecessor, *, data):
    """The state's five fields, or none."""
    state = parse_fsrs(data)
    if state is None:
        return None
    return {
        "stability": state.stability,
        "difficulty": state.difficulty,
        "decay": state.decay,
        "desired_retention": state.desired_retention,
        "last_review_sec": state.last_review_sec,
    }


# --- card mastery ---------------------------------------------------------------------------------


def build_card(predecessor, spec, did=1, odid=0, card_id=1):
    """A `types.Card` of the case's fields."""
    return predecessor("types.Card")(
        id=card_id,
        nid=card_id,
        did=did,
        queue=spec["queue"],
        ctype=spec["ctype"],
        due=0,
        ivl=spec["ivl"],
        factor=2500,
        reps=1,
        lapses=0,
        odid=odid,
        stability=spec.get("stability"),
        difficulty=None,
        decay=spec.get("decay"),
        last_review_sec=spec.get("last_review_sec"),
    )


def mastery_of(card_mastery, predecessor, *, card, now_sec, mature_ivl):
    return card_mastery(build_card(predecessor, card), now_sec, mature_ivl)


def fsrs_card(rng, queue=2):
    return {
        "queue": queue,
        "ctype": 2,
        "ivl": rng.randrange(1, 400),
        "stability": round(rng.uniform(0.2, 300.0), 3),
        "decay": rng.choice([0.1, 0.126, 0.2, 0.5, 1.0]),
        "last_review_sec": NOW - rng.randrange(0, 300) * DAY - rng.randrange(0, DAY),
    }


def mastery_cases(rng):
    found = []
    base = {"queue": 2, "ctype": 2, "ivl": 21}
    for queue in (-1, -2, -3):
        found.append(("suspended_or_buried", {"card": {**base, "queue": queue}, **clock()}))
        found.append(
            (
                "suspended_or_buried",
                {"card": {**base, "queue": queue, "stability": 30.0, "decay": 0.2}, **clock()},
            )
        )
    for ivl in (0, 1, 20, 21, 22, 365):
        for ctype in (0, 1, 2, 3):
            found.append(("fallback", {"card": {"queue": 2, "ctype": ctype, "ivl": ivl}, **clock()}))
    found.append(("fallback", {"card": {**base, "stability": 0.0}, **clock()}))
    found.append(("fallback", {"card": {**base, "stability": -2.0}, **clock()}))
    found.append(("fallback", {"card": {**base, "stability": None}, **clock()}))
    found.append(("mature_interval", {"card": {**base, "ivl": 20}, **clock(mature_ivl=20)}))
    found.append(("mature_interval", {"card": {**base, "ivl": 20}, **clock(mature_ivl=21)}))
    for stability in (0.5, 1.0, 10.0, 99.0, 100.0, 101.0, 250.0, 5000.0):
        for elapsed in (0, 1, 30, 400):
            card = {
                "queue": 2,
                "ctype": 2,
                "ivl": 30,
                "stability": stability,
                "decay": 0.2,
                "last_review_sec": NOW - elapsed * DAY,
            }
            found.append(("stability_target", {"card": card, **clock()}))
    for decay in (1e-6, 1e-3, 5e-4, 0.01, 0.126, 0.2, 1.0, 10.0, 11.0, 50.0):
        card = {
            "queue": 2,
            "ctype": 2,
            "ivl": 30,
            "stability": 20.0,
            "decay": decay,
            "last_review_sec": NOW - 40 * DAY,
        }
        found.append(("decay_clamp", {"card": card, **clock()}))
    for last in (None, 0, NOW, NOW + 5 * DAY, NOW - 1):
        card = {"queue": 2, "ctype": 2, "ivl": 30, "stability": 20.0, "decay": 0.2}
        found.append(("last_review", {"card": {**card, "last_review_sec": last}, **clock()}))
    for queue in (0, 1, 3, 4):
        found.append(("other_queue", {"card": fsrs_card(rng, queue), **clock()}))
    for _ in range(14):
        found.append((None, {"card": fsrs_card(rng), **clock()}))
    return found


def clock(mature_ivl=21):
    return {"now_sec": NOW, "mature_ivl": mature_ivl}


# --- the unit parse -------------------------------------------------------------------------------


def unit_cases(rng):
    names = [
        ("none", "Alpha"),
        ("none", ""),
        ("none", f"Alpha{SEP}Lesson"),
        ("none", "Unit"),
        ("none", "Unit x"),
        ("none", "unit 4"),
        ("plain", "Alpha Unit 4"),
        ("plain", f"Alpha{SEP}Unit 42{SEP}cards"),
        ("leading_zeros", "Unit 007"),
        ("leading_zeros", "Unit 0"),
        ("leading_zeros", "Unit 000"),
        ("leading_zeros", "Unit 0100"),
        ("spaces", "Unit    9"),
        ("spaces", "Unit\t12"),
        ("several", "Unit 3 and Unit 8"),
        ("several", f"Alpha{SEP}Unit 5{SEP}Unit 6"),
        ("embedded", "Beta Unit 12 extra"),
        ("embedded", "XUnit 5"),
        ("digits_after", "Unit 12abc"),
        ("digits_after", "Unit 5.5"),
        ("large", "Unit 99999"),
        ("large", "Unit 1000000"),
        ("a_dash_after", "Alpha Unit 15 — verbs"),
    ]
    found = [(cls, {"deck_name": name}) for cls, name in names]
    for _ in range(8):
        found.append((None, {"deck_name": f"Gamma{SEP}Unit {rng.randrange(0, 400):0{rng.choice([1, 2, 3])}d}"}))
    return found


# --- course progress ------------------------------------------------------------------------------


def with_synthetic_courses(compute_progress, predecessor, *, courses, decks, cards, now_sec):
    """Patch `progress`'s own names with the case's courses, then call `compute_progress`."""
    progress = predecessor("progress")
    languages = {c["deck_root"]: (c["code"], c["name"], c["flag"]) for c in courses}
    unit_bands = {c["code"]: {b: tuple(r) for b, r in c["bands"].items()} for c in courses}
    deck_names = {int(key): value for key, value in decks.items()}
    built = [
        build_card(predecessor, card, did=card["did"], odid=card.get("odid", 0), card_id=index + 1)
        for index, card in enumerate(cards)
    ]
    with (
        mock.patch.object(progress, "LANGUAGE_DECKS", languages),
        mock.patch.object(progress, "CEFR_UNIT_BANDS", unit_bands),
    ):
        results = compute_progress(built, deck_names, now_sec=now_sec)
    return [
        {
            "code": p.code,
            "name": p.name,
            "flag": p.flag,
            "total_cards": p.total_cards,
            "mature_cards": p.mature_cards,
            "mastery_pct": p.mastery_pct,
            "current_band": p.current_band,
            "current_unit": p.current_unit,
            "bands": [
                {"band": b.band, "total": b.total, "mature": b.mature, "pct": b.pct, "achieved": b.achieved}
                for b in p.bands
            ],
        }
        for p in results
    ]


#: Two synthetic courses: unit ranges of six units a band, and a course whose bands leave gaps.
COURSES = [
    {
        "deck_root": "Alpha Course",
        "code": "al",
        "name": "Alpha",
        "flag": "\U0001f3f3",
        "bands": {"A1": [1, 6], "A2": [7, 12], "B1": [13, 18], "B2": [19, 24], "C1": [25, 30], "C2": [31, 36]},
    },
    {
        "deck_root": "Beta Course",
        "code": "be",
        "name": "Beta",
        "flag": "\U0001f3f4",
        "bands": {"A1": [1, 4], "B1": [10, 14], "C2": [20, 20]},
    },
    {
        "deck_root": "Gamma Course",
        "code": "ga",
        "name": "Aardvark Gamma",
        "flag": "\U0001f3c1",
        "bands": {"A1": [1, 2], "A2": [3, 4]},
    },
]


def solid(count=1, kind="mature"):
    """`count` fallback-path cards: mature (mastery 1) or young (mastery 0)."""
    ivl = 30 if kind == "mature" else 3
    return [{"queue": 2, "ctype": 2, "ivl": ivl} for _ in range(count)]


def case_of(plan, rng=None, courses=None, extra=None):
    """A course-progress input from `plan`: a list of `(course root, unit, cards)`."""
    decks, cards = {}, []
    next_deck = 10
    for root, unit, group in plan:
        decks[str(next_deck)] = f"{root}{SEP}Unit {unit:02d} — Part"
        for card in group:
            cards.append({**card, "did": next_deck})
        next_deck += 1
    for name, group in extra or []:
        decks[str(next_deck)] = name
        for card in group:
            cards.append({**card, "did": next_deck})
        next_deck += 1
    return {"courses": courses or COURSES, "decks": decks, "cards": cards, "now_sec": NOW}


def progress_cases(rng):
    alpha, beta = "Alpha Course", "Beta Course"
    found = [("empty", case_of([]))]
    # A1 achieved at exactly 80 percent (four of five), A2 not.
    found.append(("threshold", case_of([(alpha, 1, solid(4) + solid(1, "young")), (alpha, 7, solid(1, "young"))])))
    found.append(("below_threshold", case_of([(alpha, 1, solid(3) + solid(2, "young")), (alpha, 7, solid(2))])))
    # A gap: A1 achieved, A2 not, B1 achieved: the current band stays A1.
    found.append(("gap", case_of([(alpha, 2, solid(3)), (alpha, 8, solid(1) + solid(3, "young")), (alpha, 14, solid(3))])))
    # A1 not achieved but B1 and C2 are: current band is A1.
    found.append(("unachieved_a1", case_of([(alpha, 3, solid(1) + solid(4, "young")), (alpha, 13, solid(2)), (alpha, 31, solid(2))])))
    # Every band achieved.
    found.append(("every_band", case_of([(alpha, unit, solid(2)) for unit in (1, 8, 14, 20, 26, 32)])))
    # A band with no cards stops the run.
    found.append(("empty_band_stops", case_of([(alpha, 2, solid(2)), (alpha, 14, solid(2))])))
    # The run reaches B1 only.
    found.append(("skipped_band", case_of([(alpha, 1, solid(1)), (alpha, 9, solid(1)), (alpha, 15, solid(1)), (alpha, 20, solid(2, "young"))])))
    # A drop: a later recompute of the same course after its A2 card lapsed.
    found.append(("drop", case_of([(alpha, 1, solid(2)), (alpha, 8, solid(2, "young"))])))
    # Gaps in the configured bands, units out of every band, units with no number.
    found.append(("band_gaps", case_of([(beta, 2, solid(2)), (beta, 7, solid(3)), (beta, 11, solid(2)), (beta, 20, solid(1))])))
    found.append(
        (
            "uncounted",
            case_of(
                [(alpha, 1, solid(1)), (alpha, 40, solid(5)), (beta, 50, solid(2))],
                extra=[("Alpha Course", solid(2)), ("Other Course" + SEP + "Unit 3", solid(2)), (f"Alpha Course{SEP}Lesson", solid(2))],
            ),
        )
    )
    # Cards in a filtered deck: the home deck decides.
    plan = case_of([(alpha, 3, solid(2))])
    plan["decks"]["999"] = "Filtered"
    for card in plan["cards"]:
        card["odid"] = card["did"]
        card["did"] = 999
    found.append(("home_deck", plan))
    found.append(("home_deck_none", {**plan, "decks": {k: v for k, v in plan["decks"].items() if k == "999"}}))
    # Several courses: ordered by name.
    found.append(
        (
            "ordered_by_name",
            case_of([(beta, 2, solid(2)), (alpha, 2, solid(2)), ("Gamma Course", 1, solid(1))]),
        )
    )
    # Current unit: the highest unit holding a card of mastery at least 0.5.
    found.append(("current_unit", case_of([(alpha, 5, solid(1)), (alpha, 3, solid(1)), (alpha, 9, solid(1, "young")), (alpha, 12, solid(1, "young"))])))
    found.append(("current_unit_none", case_of([(alpha, 5, solid(2, "young"))])))
    # Suspended and buried cards.
    suspended = [{"queue": -1, "ctype": 2, "ivl": 40}]
    buried = [{"queue": -2, "ctype": 2, "ivl": 40}, {"queue": -3, "ctype": 2, "ivl": 40}]
    found.append(("suspended", case_of([(alpha, 1, suspended + solid(1))])))
    found.append(("buried", case_of([(alpha, 1, buried + solid(1))])))
    # FSRS cards: mastery depends on the clock.
    for _ in range(8):
        plan = []
        for unit in rng.sample(range(1, 37), rng.randrange(2, 7)):
            plan.append((alpha, unit, [fsrs_card(rng) for _ in range(rng.randrange(1, 5))]))
        found.append((None, case_of(plan)))
    return found


# --- the band-up ----------------------------------------------------------------------------------


class StubStore:
    """The stored bands and the milestones; `record_band_up` is `INSERT OR IGNORE` on a pair."""

    def __init__(self, stored, milestones, log):
        self.stored = [{"code": code, "current_band": band} for code, band in stored]
        self.milestones = {(code, band) for code, band in milestones}
        self.log = log

    async def get_language_progress(self):
        return list(self.stored)

    async def upsert_language_progress(self, **fields):
        self.stored = [r for r in self.stored if r["code"] != fields["code"]]
        self.stored.append({"code": fields["code"], "current_band": fields["current_band"]})

    async def record_band_up(self, code, band):
        new = (code, band) not in self.milestones
        self.milestones.add((code, band))
        self.log["records"].append({"code": code, "band": band, "new": new})
        return new

    async def upsert_xp_grant(self, day, source, amount):
        self.log["grants"].append({"day": day, "source": source, "amount": amount})

    async def award_badge(self, badge, day):
        self.log["badges"].append({"key": badge.key, "name": badge.name, "emoji": badge.emoji})


def persist_progress(persist, predecessor, *, stored, milestones, progs, notify, notifier, today):
    """Run `_persist_progress` over a stub store with `compute_progress` answering `progs`."""
    pipeline = predecessor("pipeline")
    progress = predecessor("progress")
    log = {"records": [], "grants": [], "badges": [], "celebrations": []}
    results = [
        progress.LanguageProgress(
            code=p["code"],
            name=p["name"],
            flag=p["flag"],
            total_cards=10,
            mature_cards=5,
            mastery_pct=p["mastery_pct"],
            current_band=p["band"],
            bands=(),
            current_unit=None,
        )
        for p in progs
    ]

    async def celebrate(*, event_type, event_key, text, budget_exempt):
        log["celebrations"].append(
            {"event_type": event_type, "event_key": event_key, "budget_exempt": budget_exempt}
        )

    me = SimpleNamespace(
        _store=StubStore(stored, milestones, log),
        _now_ms=lambda: NOW * 1000,
        _notifier=object() if notifier else None,
        _settings=SimpleNamespace(notify_milestones=notify),
        celebrate=celebrate,
    )
    me._celebrate_band_up = lambda p, day: pipeline.GamifyPipeline._celebrate_band_up(me, p, day)
    with mock.patch.object(progress, "compute_progress", lambda *a, **k: results):
        asyncio.run(pipeline.GamifyPipeline._persist_progress(me, [], {}, as_date(today)))
    return log


def prog(code, band, name=None):
    return {"code": code, "name": name or code.capitalize(), "flag": "F", "band": band, "mastery_pct": 61.5}


def band_up_cases(rng):
    base = {"notify": True, "notifier": True, "today": 20_000}
    found = [
        ("first_sighting", {**base, "stored": [], "milestones": [], "progs": [prog("al", "B1")]}),
        ("first_sighting", {**base, "stored": [], "milestones": [["al", "B1"]], "progs": [prog("al", "B1")]}),
        ("first_sighting_a1", {**base, "stored": [], "milestones": [], "progs": [prog("al", "A1"), prog("be", "C2")]}),
        ("band_up", {**base, "stored": [["al", "A1"]], "milestones": [["al", "A1"]], "progs": [prog("al", "A2")]}),
        ("skipped_bands", {**base, "stored": [["al", "A1"]], "milestones": [["al", "A1"]], "progs": [prog("al", "C1")]}),
        ("unchanged", {**base, "stored": [["al", "B2"]], "milestones": [["al", "B2"]], "progs": [prog("al", "B2")]}),
        ("dropped", {**base, "stored": [["al", "B2"]], "milestones": [["al", "B2"]], "progs": [prog("al", "A2")]}),
        ("reached_again", {**base, "stored": [["al", "A2"]], "milestones": [["al", "A2"], ["al", "B1"]], "progs": [prog("al", "B1")]}),
        ("two_courses", {**base, "stored": [["al", "A1"], ["be", "B1"]], "milestones": [["al", "A1"], ["be", "B1"]], "progs": [prog("al", "A2"), prog("be", "B2", "Beta Name")]}),
        ("new_and_up", {**base, "stored": [["al", "A1"]], "milestones": [["al", "A1"]], "progs": [prog("al", "B1"), prog("be", "A2")]}),
        ("a_course_with_no_progress", {**base, "stored": [["al", "A1"], ["be", "A1"]], "milestones": [["al", "A1"]], "progs": [prog("al", "A2")]}),
        ("no_notifier", {**base, "notifier": False, "stored": [["al", "A1"]], "milestones": [["al", "A1"]], "progs": [prog("al", "A2")]}),
        ("milestones_muted", {**base, "notify": False, "stored": [["al", "A1"]], "milestones": [["al", "A1"]], "progs": [prog("al", "A2")]}),
        ("a_dashed_code", {**base, "stored": [["en-us", "A1"]], "milestones": [["en-us", "A1"]], "progs": [prog("en-us", "A2")]}),
        ("no_courses", {**base, "stored": [["al", "A1"]], "milestones": [["al", "A1"]], "progs": []}),
    ]
    for _ in range(8):
        codes = rng.sample(["al", "be", "ga", "de", "fr"], rng.randrange(1, 4))
        stored = [[c, rng.choice(BANDS)] for c in codes if rng.random() < 0.7]
        milestones = [[c, b] for c, b in stored] + [[rng.choice(codes), rng.choice(BANDS)]]
        found.append((None, {**base, "stored": stored, "milestones": milestones, "progs": [prog(c, rng.choice(BANDS)) for c in codes]}))
    return found


# --- the law block --------------------------------------------------------------------------------


def law_summary(summary, predecessor, *, today, streak, ledger, leeches, dues):
    """`law_track_summary` over a temporary store the case's ledger rows fill through its own
    `upsert_xp_grant`, so the predecessor's own SQL sums them."""
    digests = predecessor("pipeline_layers.digests")
    store_type = predecessor("database.GamifyStore")

    async def run():
        with tempfile.TemporaryDirectory() as directory:
            store = await store_type(Path(directory) / "store.db").connect()
            try:
                for index, row in enumerate(ledger):
                    await store.upsert_xp_grant(
                        as_date(row["day"]), f"s{index}", row["amount"], track=row["track"]
                    )

                async def snapshot():
                    return [(index, track) for index, track in enumerate(leeches)]

                async def coaching(key):
                    return dues

                async def streak_state():
                    return SimpleNamespace(current=streak)

                me = SimpleNamespace(
                    _today=lambda: as_date(today),
                    _store=SimpleNamespace(conn=store.conn, get_leech_snapshot=snapshot),
                    law_streak_state=streak_state,
                    coaching=coaching,
                )
                me.law_total_xp = lambda: digests.DigestsLayer.law_total_xp(me)
                return await summary(me)
            finally:
                await store.close()

    return asyncio.run(run())


def law_summary_cases(rng):
    today = 20_000
    found = [
        ("empty", {"today": today, "streak": 0, "ledger": [], "leeches": [], "dues": None}),
        ("dues_pending", {"today": today, "streak": 3, "ledger": [], "leeches": [], "dues": None}),
        ("dues_zero", {"today": today, "streak": 0, "ledger": [], "leeches": [], "dues": 0}),
        ("dues_not_a_number", {"today": today, "streak": 0, "ledger": [], "leeches": [], "dues": "12"}),
        (
            "tracks_apart",
            {
                "today": today,
                "streak": 5,
                "ledger": [
                    {"day": today, "track": "law", "amount": 40},
                    {"day": today, "track": "language", "amount": 900},
                    {"day": today - 1, "track": "law", "amount": 70},
                    {"day": today + 1, "track": "law", "amount": 5},
                ],
                "leeches": ["law", "language", "law"],
                "dues": 17,
            },
        ),
        ("leech_cap", {"today": today, "streak": 1, "ledger": [], "leeches": ["law"] * 12, "dues": 4}),
        ("leech_cap", {"today": today, "streak": 1, "ledger": [], "leeches": ["law"] * 10, "dues": 4}),
        ("level_curve", {"today": today, "streak": 0, "ledger": [{"day": today - 9, "track": "law", "amount": 100_000}], "leeches": [], "dues": 1}),
        ("negative_ledger", {"today": today, "streak": 0, "ledger": [{"day": today, "track": "law", "amount": -30}, {"day": today, "track": "law", "amount": 10}], "leeches": [], "dues": None}),
    ]
    for _ in range(8):
        rows = [
            {"day": today - rng.randrange(0, 5), "track": rng.choice(["law", "language"]), "amount": rng.randrange(1, 800)}
            for _ in range(rng.randrange(0, 9))
        ]
        found.append((None, {"today": today, "streak": rng.randrange(0, 40), "ledger": rows, "leeches": [rng.choice(["law", "language"]) for _ in range(rng.randrange(0, 14))], "dues": rng.choice([None, rng.randrange(0, 300)])}))
    return found


def law_pillar_cases(rng):
    found = [("zero", {"law_leech_active": 0}), ("one", {"law_leech_active": 1})]
    found += [(None, {"law_leech_active": count}) for count in (2, 5, 9)]
    found += [("cap", {"law_leech_active": count}) for count in (10, 11, 25, 10_000)]
    found.append(("negative", {"law_leech_active": -1}))
    return found


#: The marker each field's line begins with in `render_law_block`'s output.
FIELD_MARKERS = {
    "total_xp": "\U0001f4ca",
    "streak": "\U0001f525",
    "xp_today": "⚡",
    "dues": "\U0001f4cc",
    "mastery": "\U0001f3af",
    "leeches": "\U0001fab1",
}


def law_block_fields(render, predecessor, *, payload):
    """Whether the block rendered and which fields' lines it holds, never its text."""
    text = render(payload)
    lines = text.split("\n") if text else []
    held = [name for name, marker in FIELD_MARKERS.items() if any(line.startswith(marker) for line in lines)]
    head = next((line for line in lines if line.startswith(FIELD_MARKERS["total_xp"])), "")
    return {"rendered": bool(text), "fields": held, "level_shown": " · L" in head}


def payload(streak=0, xp_today=0, leech_active=0, total_xp=0, level=0, dues=None, mastery=100.0):
    return {"streak": streak, "xp_today": xp_today, "leech_active": leech_active, "total_xp": total_xp, "level": level, "dues": dues, "mastery": mastery}


def law_block_cases(rng):
    found = [
        ("empty", {"payload": payload()}),
        ("empty", {"payload": payload(dues=12, level=4, mastery=70.0)}),
        ("empty", {"payload": {}}),
        ("empty", {"payload": {"streak": None, "xp_today": None, "leech_active": None, "total_xp": None}}),
        ("streak_only", {"payload": payload(streak=3)}),
        ("xp_today_only", {"payload": payload(xp_today=25)}),
        ("leech_only", {"payload": payload(leech_active=2, mastery=94.0)}),
        ("leech_only", {"payload": payload(leech_active=2, mastery=0.0)}),
        ("total_only", {"payload": payload(total_xp=1200, level=3)}),
        ("total_no_level", {"payload": payload(total_xp=1200, level=0)}),
        ("dues_shown", {"payload": payload(streak=1, dues=5)}),
        ("dues_zero", {"payload": payload(streak=1, dues=0)}),
        ("dues_pending", {"payload": payload(streak=1, dues=None)}),
        ("dues_negative", {"payload": payload(streak=1, dues=-3)}),
        ("mastery_hidden_without_leeches", {"payload": payload(streak=1, mastery=70.0)}),
        ("mastery_shown_with_leeches", {"payload": payload(leech_active=3, mastery=91.0)}),
        ("everything", {"payload": payload(streak=9, xp_today=40, leech_active=4, total_xp=12345, level=6, dues=18, mastery=88.0)}),
    ]
    for _ in range(8):
        found.append((None, {"payload": payload(rng.choice([0, 0, 4]), rng.choice([0, 15]), rng.choice([0, 0, 2]), rng.choice([0, 800]), rng.randrange(0, 8), rng.choice([None, 0, 6]), rng.choice([0.0, 91.0]))}))
    return found


FUNCTIONS = {
    "memory_state": {
        "kind": "adapter",
        "function": "fsrs.parse_fsrs",
        "adapter": memory_state_fields,
        "note": "Calls parse_fsrs with the case's text and returns the state's five fields, or none; "
        "the cases hold valid, missing, unparsable, non-object, non-finite and non-positive values, "
        "a boolean read as a number, and a name that spells a non-finite literal.",
        "cases": memory_state_cases,
    },
    "card_mastery": {
        "kind": "adapter",
        "function": "progress.card_mastery",
        "adapter": mastery_of,
        "note": "Builds a types.Card from the case's queue, type, interval, stability, decay and last "
        "review, and passes the case's clock and mature interval.",
        "cases": mastery_cases,
    },
    "unit_parse": {"kind": "function", "function": "progress.parse_unit", "cases": unit_cases},
    "course_progress": {
        "kind": "adapter",
        "function": "progress.compute_progress",
        "adapter": with_synthetic_courses,
        "note": "Patches progress.LANGUAGE_DECKS and progress.CEFR_UNIT_BANDS with the case's "
        "synthetic courses and unit bands, builds the cards and synthetic deck names, and returns "
        "each course's fields and bands.",
        "cases": progress_cases,
    },
    "band_up": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._persist_progress",
        "adapter": persist_progress,
        "note": "Runs _persist_progress (and its _celebrate_band_up) on a stub store holding the stored "
        "bands and milestones, with progress.compute_progress patched to answer the case's per-course "
        "bands; returns the milestone records, the XP grants, the badges and the celebrations the "
        "stub recorded, never their text.",
        "cases": band_up_cases,
    },
    "law_mastery_pillar": {
        "kind": "function",
        "function": "scoring.law_mastery_pillar",
        "cases": law_pillar_cases,
    },
    "law_track_summary": {
        "kind": "adapter",
        "function": "pipeline_layers.digests.DigestsLayer.law_track_summary",
        "adapter": law_summary,
        "note": "Runs law_track_summary over a temporary store the case's ledger rows fill, a stub "
        "leech snapshot, a stub law streak and a stub coaching value for the law dues; returns the "
        "summary's fields.",
        "cases": law_summary_cases,
    },
    "law_block_shown": {
        "kind": "adapter",
        "function": "telegram.render_law_block",
        "adapter": law_block_fields,
        "note": "Calls render_law_block with the case's payload and returns whether it rendered and "
        "which fields' lines it holds, never its text.",
        "cases": law_block_cases,
    },
    "curriculum.constants": {
        "kind": "constants",
        "names": [
            "curriculum.CEFR_BANDS",
            "curriculum.CEFR_BAND_ACHIEVED_PCT",
            "curriculum.PROGRESS_MATURE_IVL_DAYS",
            "curriculum.PROGRESS_STABILITY_TARGET_DAYS",
            "curriculum.MATURE_MASTERY_THRESHOLD",
            "fsrs.DEFAULT_DECAY",
            "fsrs._MIN_ABS_DECAY",
            "fsrs._MAX_ABS_DECAY",
            "constants.XP_BONUS_BAND_UP",
            "constants.MASTERY_LEECH_PENALTY",
            "constants.MASTERY_LEECH_PENALTY_CAP",
        ],
    },
}

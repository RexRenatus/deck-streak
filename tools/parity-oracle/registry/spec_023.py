"""SPEC-023's registrations: the read's deck scope and study-event rule, the window's rebase, the
change gate's probe, and the constants the port uses verbatim.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_deck_ids` drives `deck_filter.py:allowed_deck_ids`. A JSON object's keys are strings, so it
  turns the case's keys back into the deck ids they are, and returns the allowed ids sorted.
* `on_a_review` drives `types.py:Review.is_study_event` on a `Review` holding the case's type and
  ease, every other field zero.
* `with_a_stub_store` drives `pipeline.py:GamifyPipeline._maybe_rebase_ingest` on a stand-in
  pipeline that holds only a stub store. The recount's offload is patched to return the case's
  count and to record the floor it was asked to count before, and the self-check's warnings are
  counted instead of logged. It returns the base the function kept or wrote, that floor (null when
  the base was kept), the warnings, and the base the stub store was told to persist.
* `in_a_temporary_collection` writes the case's synthetic card rows and review ids into a
  temporary collection holding the three tables the probe reads (`col`, `cards`, `revlog`), then
  calls `anki_reader.py:probe_change_signal` on it and returns the probe's three integers. This
  module imports no database module and reads nothing: it writes through the `sqlite3` module the
  predecessor's reader already imports, and only the predecessor's probe reads the file back.

The case builders draw only from the `random.Random` the generator seeds. Every deck name is
synthetic, and every instant is epoch milliseconds.
"""

import asyncio
import json
import pathlib
import tempfile
import types
from unittest import mock

#: Anki's deck-name separator: the text before the first one is the top-level name.
SEP = "\x1f"
DAY_MS = 86_400_000
#: A study day number near the present, as a whole-day offset from the Unix epoch.
PRESENT_DAY = 20_000
#: The window and its rebase period, in days: the values the constants golden records, used here
#: only to place the cases around the predecessor's boundaries.
WINDOW_DAYS = 400
REBASE_DAYS = 7
#: The deck ids the rebase is handed; the stub offload ignores them, as the count is the case's.
ALLOWED_DIDS = (1, 1_700_000_000_001)

#: Synthetic deck names: nested trees, a root that extends another's name, a subdeck named like a
#: root, case, an empty top-level name, and text outside ASCII.
DECK_NAMES = (
    "Default",
    "Law",
    f"Law{SEP}Evidence",
    f"Law{SEP}Torts{SEP}Duty",
    "Language",
    f"Language{SEP}Unit 01",
    "Language Arts",
    "Lawn",
    "law",
    "Other",
    f"Other{SEP}Law",
    "Maths",
    f"{SEP}Leading",
    f"Ébauche{SEP}Brouillon",
    f"Αλφα{SEP}Βήτα",
    f"甲{SEP}乙",
)
#: Prefixes drawn from: exact roots, partial roots, case, the separator, text outside ASCII and
#: the empty prefix, which every name starts with.
PREFIXES = (
    "Law",
    "Lang",
    "Language",
    "L",
    "law",
    "Other",
    f"Other{SEP}Law",
    "Maths ",
    "Ébauche",
    "Αλ",
    "甲",
    "Zeta",
    "",
)


def deck_ids(rng, count):
    """`count` distinct deck ids shaped like Anki's: the default deck's 1, else a creation stamp."""
    ids = {1}
    while len(ids) < count:
        ids.add(1_700_000_000_000 + rng.randrange(1, 10_000_000))
    return sorted(ids)


def deck_names(rng, names):
    return {str(deck_id): name for deck_id, name in zip(deck_ids(rng, len(names)), names)}


def with_deck_ids(allowed_deck_ids, predecessor, *, deck_names, include_prefixes):
    """Call the scope with the case's names keyed by deck id, and return the allowed ids sorted."""
    names = {int(deck_id): name for deck_id, name in deck_names.items()}
    return sorted(allowed_deck_ids(names, include_prefixes))


def scope_cases(rng):
    every = deck_names(rng, DECK_NAMES)
    drawn = [
        ("everything", {"deck_names": every, "include_prefixes": []}),
        ("everything", {"deck_names": every, "include_prefixes": [""]}),
        ("top-level", {"deck_names": every, "include_prefixes": ["Law"]}),
        ("top-level", {"deck_names": every, "include_prefixes": [f"Other{SEP}Law"]}),
        ("partial", {"deck_names": every, "include_prefixes": ["Lang"]}),
        ("partial", {"deck_names": every, "include_prefixes": ["L", "Zeta"]}),
        ("case", {"deck_names": every, "include_prefixes": ["law"]}),
        ("several", {"deck_names": every, "include_prefixes": ["Law", "Language", "Maths"]}),
        ("unicode", {"deck_names": every, "include_prefixes": ["Ébauche", "甲"]}),
        ("unicode", {"deck_names": every, "include_prefixes": ["Αλ"]}),
        ("nothing", {"deck_names": every, "include_prefixes": ["Zeta", "Maths "]}),
        ("empty", {"deck_names": {}, "include_prefixes": ["Law"]}),
        ("empty", {"deck_names": {}, "include_prefixes": []}),
    ]
    for _ in range(10):
        names = rng.sample(DECK_NAMES, rng.randrange(1, len(DECK_NAMES)))
        prefixes = rng.sample(PREFIXES, rng.randrange(0, 4))
        drawn.append((None, {"deck_names": deck_names(rng, names), "include_prefixes": prefixes}))
    return drawn


def on_a_review(is_study_event, predecessor, *, rtype, ease):
    """Read the property on a `Review` of the case's type and ease, every other field zero."""
    review = predecessor("types.Review")(
        id_ms=0, cid=0, ease=ease, ivl=0, last_ivl=0, factor=0, time_ms=0, rtype=rtype
    )
    return is_study_event.fget(review)


def study_event_cases(rng):
    """Every type from -1 to 6 against every ease from -1 to 5, then wider draws."""
    drawn = [
        ("grid", {"rtype": rtype, "ease": ease}) for rtype in range(-1, 7) for ease in range(-1, 6)
    ]
    for _ in range(8):
        drawn.append((None, {"rtype": rng.randrange(-3, 10), "ease": rng.randrange(-3, 10)}))
    return drawn


class StubStore:
    """Records every setting it is told to persist, and persists nothing."""

    def __init__(self):
        self.persisted = []

    async def set_setting(self, key, value):
        self.persisted.append((key, value))


class CountingLogger:
    """Counts the warnings the self-check raises, instead of logging them."""

    def __init__(self):
        self.warnings = 0

    def warning(self, *args, **kwargs):
        self.warnings += 1


def with_a_stub_store(maybe_rebase_ingest, predecessor, *, base, now_ms, recount):
    """Run the rebase on a stand-in pipeline whose offload returns the case's recount."""
    pipeline = predecessor("pipeline")
    store = StubStore()
    logger = CountingLogger()
    floors = []

    async def recount_offloaded(name, function, *args, before_id_ms, **kwargs):
        floors.append(before_id_ms)
        return recount

    stand_in = types.SimpleNamespace(
        _store=store,
        _settings=types.SimpleNamespace(collection_path="collection.anki2"),
    )
    with (
        mock.patch.object(pipeline.offload, "run_offloaded", recount_offloaded),
        mock.patch.object(pipeline, "logger", logger),
    ):
        kept_or_written = asyncio.run(
            maybe_rebase_ingest(stand_in, base, set(ALLOWED_DIDS), now_ms)
        )
    persisted = [json.loads(value) for key, value in store.persisted if key == "ingest_base"]
    return {
        "base": kept_or_written,
        "recount_floor": floors[0] if floors else None,
        "self_check_warnings": logger.warnings,
        "persisted": persisted[0] if persisted else None,
    }


def now_in_a_day(rng):
    return (PRESENT_DAY + rng.randrange(-300, 300)) * DAY_MS + rng.randrange(0, DAY_MS)


def rebase_case(base, now_ms, recount):
    return {"base": base, "now_ms": now_ms, "recount": recount}


def rebase_cases(rng):
    """No base, a kept base, both sides of the rebase boundary, a shrink, and a young clock."""
    drawn = []
    now = now_in_a_day(rng)
    fresh_floor = now - WINDOW_DAYS * DAY_MS
    limit = now - (WINDOW_DAYS + REBASE_DAYS) * DAY_MS
    drawn.append(("fresh", rebase_case(None, now, rng.randrange(0, 50_000))))
    drawn.append(("fresh", rebase_case(None, now, 0)))
    kept = {"before_id_ms": fresh_floor - 3 * DAY_MS, "count": 4_321}
    drawn.append(("kept", rebase_case(kept, now, 9_999)))
    drawn.append(("boundary", rebase_case({"before_id_ms": limit, "count": 700}, now, 650)))
    drawn.append(("boundary", rebase_case({"before_id_ms": limit - 1, "count": 700}, now, 650)))
    drawn.append(("boundary", rebase_case({"before_id_ms": limit + 1, "count": 700}, now, 650)))
    stale = {"before_id_ms": limit - 30 * DAY_MS, "count": 12_000}
    drawn.append(("shrink", rebase_case(stale, now, 11_999)))
    drawn.append(("shrink", rebase_case(stale, now, 0)))
    drawn.append(("equal", rebase_case(stale, now, 12_000)))
    drawn.append(("grow", rebase_case(stale, now, 12_001)))
    young = 5 * DAY_MS + 1_234
    drawn.append(("young", rebase_case(None, young, 17)))
    drawn.append(("young", rebase_case({"before_id_ms": 0, "count": 17}, young, 3)))
    drawn.append(("young", rebase_case(None, WINDOW_DAYS * DAY_MS, 5)))
    drawn.append(("young", rebase_case(None, WINDOW_DAYS * DAY_MS + 1, 5)))
    for _ in range(10):
        now = now_in_a_day(rng)
        limit = now - (WINDOW_DAYS + REBASE_DAYS) * DAY_MS
        base = None
        if rng.random() < 0.8:
            base = {
                "before_id_ms": limit + rng.randrange(-20, 20) * DAY_MS + rng.randrange(-5, 5),
                "count": rng.randrange(0, 60_000),
            }
        drawn.append((None, rebase_case(base, now, rng.randrange(0, 60_000))))
    return drawn


def in_a_temporary_collection(probe_change_signal, predecessor, *, cards, review_ids):
    """Probe a temporary collection holding the case's cards (ids from 1) and review ids.

    Each card row lists due, ivl, queue, factor, lapses, type and odid, in that order."""
    sqlite3 = predecessor("anki_reader.sqlite3")
    with tempfile.TemporaryDirectory() as scratch:
        path = pathlib.Path(scratch) / "collection.anki2"
        connection = sqlite3.connect(path)
        try:
            with connection:
                connection.execute(
                    "CREATE TABLE col (id INTEGER PRIMARY KEY, crt INTEGER NOT NULL, "
                    "conf TEXT NOT NULL)"
                )
                connection.execute(
                    "CREATE TABLE cards (id INTEGER PRIMARY KEY, due INTEGER NOT NULL, "
                    "ivl INTEGER NOT NULL, queue INTEGER NOT NULL, factor INTEGER NOT NULL, "
                    "lapses INTEGER NOT NULL, type INTEGER NOT NULL, odid INTEGER NOT NULL)"
                )
                connection.execute("CREATE TABLE revlog (id INTEGER PRIMARY KEY)")
                connection.execute("INSERT INTO col (id, crt, conf) VALUES (1, 0, '{}')")
                connection.executemany(
                    "INSERT INTO cards (id, due, ivl, queue, factor, lapses, type, odid) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    [(index + 1, *row) for index, row in enumerate(cards)],
                )
                connection.executemany(
                    "INSERT INTO revlog (id) VALUES (?)", [(review,) for review in review_ids]
                )
        finally:
            connection.close()
        probe = probe_change_signal(path)
    return {
        "cards_count": probe.cards_count,
        "cards_fingerprint": probe.cards_fingerprint,
        "max_revlog_id": probe.max_revlog_id,
    }


def card_row(rng):
    """A card's probed columns, drawn from the shapes a collection holds: a new card's position, a
    learning card's due stamp and negative interval, a review card's day, and a filtered deck's
    negative position with its home deck's id."""
    shape = rng.choice(("new", "learning", "review", "filtered"))
    if shape == "new":
        row = (rng.randrange(1, 300_000), 0, 0, 0, 0, 0, 0)
    elif shape == "learning":
        row = (
            1_700_000_000 + rng.randrange(0, 90_000_000),
            -rng.randrange(60, 86_400),
            1,
            rng.randrange(1300, 2600),
            rng.randrange(0, 4),
            rng.choice((1, 3)),
            0,
        )
    elif shape == "review":
        row = (
            rng.randrange(0, 3_000),
            rng.randrange(1, 36_500),
            rng.choice((2, -1, -2, -3)),
            rng.randrange(1300, 5000),
            rng.randrange(0, 40),
            2,
            0,
        )
    else:
        row = (
            -100_000 + rng.randrange(0, 500),
            rng.randrange(0, 400),
            rng.choice((0, 1, 2)),
            rng.randrange(0, 5000),
            rng.randrange(0, 10),
            rng.randrange(0, 4),
            1_700_000_000_000 + rng.randrange(1, 10_000_000),
        )
    return list(row)


def review_ids(rng, count):
    return rng.sample(range(1_700_000_000_000, 1_800_000_000_000), count)


def probe_cases(rng):
    """No card and no review, unit rows, rows past 32 bits and past the modulus, negative rows,
    then drawn collections."""
    widest = 1_700_000_000_000
    drawn = [
        ("empty", {"cards": [], "review_ids": []}),
        ("empty", {"cards": [[0, 0, 0, 0, 0, 0, 0]], "review_ids": []}),
        ("unit", {"cards": [[1, 1, 1, 1, 1, 1, 1]], "review_ids": [1]}),
        (
            "unit",
            {
                "cards": [
                    [1, 0, 0, 0, 0, 0, 0],
                    [0, 1, 0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0, 0, 0],
                    [0, 0, 0, 1, 0, 0, 0],
                    [0, 0, 0, 0, 1, 0, 0],
                    [0, 0, 0, 0, 0, 1, 0],
                    [0, 0, 0, 0, 0, 0, 1],
                ],
                "review_ids": [5, 3, 9],
            },
        ),
        (
            "wide",
            {
                "cards": [[0, 0, 0, 0, 0, 0, widest + index] for index in range(40)],
                "review_ids": review_ids(rng, 3),
            },
        ),
        (
            "modulus",
            {
                "cards": [[1_700_000_000 + index, -600, 1, 2500, 0, 1, 0] for index in range(12)],
                "review_ids": review_ids(rng, 2),
            },
        ),
        (
            "negative",
            {
                "cards": [[-100_000, -86_400, -3, 0, 0, 0, 0], [-1, -1, -1, 0, 0, 0, 0]],
                "review_ids": [],
            },
        ),
        (
            "negative",
            {
                "cards": [
                    [-100_000 + index, -600, 1, 0, 0, 1, widest + index] for index in range(25)
                ],
                "review_ids": review_ids(rng, 1),
            },
        ),
    ]
    for _ in range(8):
        count = rng.randrange(0, 60)
        drawn.append(
            (
                None,
                {
                    "cards": [card_row(rng) for _ in range(count)],
                    "review_ids": review_ids(rng, rng.randrange(0, 20)),
                },
            )
        )
    return drawn


FUNCTIONS = {
    "allowed_deck_ids": {
        "kind": "adapter",
        "function": "deck_filter.allowed_deck_ids",
        "adapter": with_deck_ids,
        "note": (
            "Converts the case's deck-name keys from JSON strings to the deck ids they are, calls "
            "the scope, and returns the allowed ids sorted."
        ),
        "cases": scope_cases,
    },
    "study_event": {
        "kind": "adapter",
        "function": "types.Review.is_study_event",
        "adapter": on_a_review,
        "note": (
            "Reads the property on a Review holding the case's type and ease, every other field "
            "zero."
        ),
        "cases": study_event_cases,
    },
    "ingest_rebase": {
        "kind": "adapter",
        "function": "pipeline.GamifyPipeline._maybe_rebase_ingest",
        "adapter": with_a_stub_store,
        "note": (
            "Runs the rebase on a stand-in pipeline holding only a stub store; the recount's "
            "offload returns the case's count and records the floor it was asked for, and the "
            "self-check's warnings are counted instead of logged."
        ),
        "cases": rebase_cases,
    },
    "change_probe": {
        "kind": "adapter",
        "function": "anki_reader.probe_change_signal",
        "adapter": in_a_temporary_collection,
        "note": (
            "Writes the case's card rows (ids from 1) and review ids into a temporary collection "
            "holding col, cards and revlog, through the sqlite3 module the predecessor's reader "
            "imports, then calls the probe on it and returns its three integers."
        ),
        "cases": probe_cases,
    },
    "ingest.constants": {
        "kind": "constants",
        "names": [
            "constants.INGEST_WINDOW_DAYS",
            "constants.INGEST_REBASE_DAYS",
            "anki_reader._CARD_FIELD_WEIGHTS",
            "anki_reader._CARDS_FINGERPRINT_MOD",
        ],
    },
}

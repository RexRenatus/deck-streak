"""SPEC-045's registrations: the day set's resolver, the law subject's parse and the digest.

`prereading.py:resolve_day_sets` and `leeches.py:_law_subject` read the predecessor's private deck
constants: its law root and year bands, its language decks with their codes and display names, and
its writing roots. Their adapters replace every one of those constants with the synthetic taxonomy
below, which is `deploy/config/readings-taxonomy.example.json` (R1's example; SPEC-045's A2 and A3
read that file as their taxonomy, so the two cannot drift apart), and then CALL the predecessor;
neither computes a rule. No golden therefore carries a private deck name.

* `under_the_synthetic_taxonomy` drives `leeches.py:_law_subject` on the case's deck name.
* `resolved_under_the_synthetic_taxonomy` builds the predecessor's `Card`, `DaySetQuery` and
  `UndeterminedRoot` from the case, calls `prereading.py:resolve_day_sets` with no new-card limit
  lookup, and returns the resolution's fields as JSON. Without a lookup every topic's limit state is
  `unknown` and no topic is limit-suppressed, so those two fields are left out, and the adapter
  refuses a case whose resolution would suppress one.
* `prereading.py:_digest_for_card_ids` reads no deck constant, so its golden is a plain function's.

The case builders draw only from the `random.Random` the generator seeds. Every deck name is
synthetic, and every card and deck id is shaped like Anki's millisecond stamps.
"""

from contextlib import ExitStack
from unittest import mock

#: The separator of a deck's stored name: the text before the first one is its top-level name.
SEP = "\x1f"
#: The synthetic taxonomy of `deploy/config/readings-taxonomy.example.json`.
LAW_ROOT = "Casebook"
LAW_BANDS = ("Year One", "Year Two", "Year Three")
#: Each language deck: its top-level name, its code and the display name a writing deck names.
LANGUAGES = (
    ("Tongue Alpha", "qaa", "Alpha"),
    ("Tongue Beta", "qab", "Beta"),
)
WRITING_ROOTS = ("Composition",)
#: The could-not-tell reasons a root the caller refused carries (SPEC-045 R6).
REASONS = ("collection_locked", "collection_open_failed", "day_set_fetch_saturated")
#: The first card id, a millisecond stamp.
FIRST_CARD = 1_700_000_000_000

#: Synthetic deck names: the bare law root, flat and banded subjects at every depth, a subject
#: that is no slug, separators to collapse, both language roots and their children, the writing
#: root with a known and an unknown display name, a filtered deck and a deck of no topic.
DECKS = (
    LAW_ROOT,
    f"{LAW_ROOT}{SEP}Method",
    f"{LAW_ROOT}{SEP}Evidence",
    f"{LAW_ROOT}{SEP}Evidence{SEP}Unit 01",
    f"{LAW_ROOT}{SEP}Year One",
    f"{LAW_ROOT}{SEP}Year One{SEP}Public Law",
    f"{LAW_ROOT}{SEP}Year One{SEP}Public Law{SEP}Constitutional Law",
    f"{LAW_ROOT}{SEP}Year One{SEP}Public Law{SEP}Constitutional Law{SEP}Unit 02",
    f"{LAW_ROOT}{SEP}Year Two{SEP}Private Law{SEP}Torts",
    f"{LAW_ROOT}{SEP}Year Two{SEP}Private Law{SEP}Procedure & Proof",
    f"{LAW_ROOT}{SEP}Year Three{SEP}Skills{SEP}Civil_Procedure -- Practice",
    f"{LAW_ROOT}{SEP}  Contracts  ",
    "Tongue Alpha",
    f"Tongue Alpha{SEP}Unit 01",
    "Tongue Beta",
    f"Tongue Beta{SEP}Unit 03",
    "Composition",
    f"Composition{SEP}Alpha",
    f"Composition{SEP}Beta{SEP}Essays",
    f"Composition{SEP}Gamma",
    "Cram",
    "Misc",
)
#: Deck names for the law subject's parse beyond DECKS: lookalikes of the root and of a band, an
#: empty segment, a leading separator, and text outside ASCII.
SUBJECT_DECKS = DECKS + (
    "Casebooks",
    "casebook",
    f"Other{SEP}{LAW_ROOT}",
    f"{SEP}{LAW_ROOT}",
    f"{LAW_ROOT}{SEP}",
    f"{LAW_ROOT}{SEP}{SEP}Evidence",
    f"{LAW_ROOT}{SEP}year one{SEP}Public Law{SEP}Torts",
    f"{LAW_ROOT}{SEP}Year One {SEP}Public Law{SEP}Torts",
    f"{LAW_ROOT}{SEP}Year Two{SEP}Private Law",
    f"{LAW_ROOT}{SEP}Year Three{SEP}Skills{SEP}Advocacy{SEP}Unit 04{SEP}Part 2",
    f"{LAW_ROOT}{SEP}Procédure",
    f"{LAW_ROOT}{SEP}契約{SEP}Unit 01",
    f"{LAW_ROOT}{SEP}{LAW_ROOT}",
)
#: A law subject that is no slug, and a writing deck whose display name no language has.
UNSAFE = f"{LAW_ROOT}{SEP}Year Two{SEP}Private Law{SEP}Procedure & Proof"
UNKNOWN_DISPLAY = f"Composition{SEP}Gamma"
#: The top-level decks a drawn day set queries.
ROOTS = (LAW_ROOT, "Tongue Alpha", "Tongue Beta", "Composition", "Cram", "Misc")
#: Parts a drawn deck name is made of.
PARTS = (LAW_ROOT, "Year One", "Year Two", "Method", "Evidence", "Torts", "Unit 01", "Misc", "")


def under_synthetic_constants(predecessor):
    """Replaces every private deck constant `_law_subject` and `resolve_day_sets` read with the
    synthetic taxonomy, for the life of the `with` it is entered by."""
    leeches = predecessor("leeches")
    prereading = predecessor("prereading")
    stack = ExitStack()
    for module, name, value in (
        (leeches, "LAW_DECK_PREFIX", LAW_ROOT),
        (leeches, "_LAW_BANDS", frozenset(LAW_BANDS)),
        (prereading, "LAW_DECK_PREFIX", LAW_ROOT),
        (
            prereading,
            "LANGUAGE_DECKS",
            {deck: (code, display, "") for deck, code, display in LANGUAGES},
        ),
        (prereading, "EXTRA_LANGUAGE_DECKS", WRITING_ROOTS),
        (
            prereading,
            "_LANG_DISPLAY_BY_CODE",
            {code: display for _, code, display in LANGUAGES},
        ),
    ):
        stack.enter_context(mock.patch.object(module, name, value))
    return stack


def under_the_synthetic_taxonomy(law_subject, predecessor, *, deck_name):
    """Calls the parse on the case's deck name, the private constants replaced."""
    with under_synthetic_constants(predecessor):
        return law_subject(deck_name)


def law_subject_cases(rng):
    """Every synthetic deck name, then drawn names of the pool's parts."""
    drawn = [(None, {"deck_name": name}) for name in SUBJECT_DECKS]
    for _ in range(24):
        parts = [rng.choice(PARTS) for _ in range(rng.randrange(1, 6))]
        drawn.append(("drawn", {"deck_name": SEP.join(parts)}))
    return drawn


def resolved_under_the_synthetic_taxonomy(
    resolve_day_sets, predecessor, *, deck_names, queries, undetermined
):
    """Builds the predecessor's inputs from the case, resolves with the private constants replaced,
    and returns the resolution's fields as JSON."""
    card = predecessor("types.Card")
    day_set_query = predecessor("prereading.DaySetQuery")
    undetermined_root = predecessor("prereading.UndeterminedRoot")
    names = {int(deck_id): name for deck_id, name in deck_names.items()}
    built = [
        day_set_query(
            root_label=query["root_label"],
            new_cards=[
                card(
                    id=queued["id"],
                    nid=queued["id"],
                    did=queued["did"],
                    queue=0,
                    ctype=0,
                    due=0,
                    ivl=0,
                    factor=0,
                    reps=0,
                    lapses=0,
                    odid=queued["odid"],
                )
                for queued in query["cards"]
            ],
        )
        for query in queries
    ]
    refused = [
        undetermined_root(root_label=root["root_label"], reason=root["reason"])
        for root in undetermined
    ]
    with under_synthetic_constants(predecessor):
        resolution = resolve_day_sets(built, names, undetermined=refused)
    if resolution.limit_suppressed:
        raise ValueError("a resolution with no limit lookup suppressed a topic")
    return {
        "active_topics": [
            {
                "topic_key": topic.topic_key,
                "deck_ids": list(topic.deck_ids),
                "new_cards": list(topic.new_cards),
                "digest": topic.digest,
            }
            for topic in resolution.active_topics
        ],
        "unmapped_decks": [
            {"deck_name": deck.deck_name, "card_count": deck.card_count}
            for deck in resolution.unmapped_decks
        ],
        "no_new_today": list(resolution.no_new_today),
        "undetermined": [
            {"root_label": root.root_label, "reason": root.reason}
            for root in resolution.undetermined
        ],
    }


def deck_ids(rng):
    """A distinct millisecond-stamp id for every synthetic deck, by name."""
    ids = set()
    while len(ids) < len(DECKS):
        ids.add(1_600_000_000_000 + rng.randrange(1, 10_000_000))
    return dict(zip(DECKS, sorted(ids), strict=True))


def top_level(name):
    return name.split(SEP, 1)[0]


def queued(card_id, did, odid=0):
    return {"id": card_id, "did": did, "odid": odid}


def day_set_cases(rng):
    """Named cases for each class the resolver decides, then drawn day sets."""
    ids = deck_ids(rng)
    names = {str(deck_id): name for name, deck_id in ids.items()}
    law = [name for name in DECKS if top_level(name) == LAW_ROOT]
    tongue = [name for name in DECKS if top_level(name) in ("Tongue Alpha", "Tongue Beta")]
    writing = [name for name in DECKS if top_level(name) == "Composition"]
    card = iter(range(FIRST_CARD, FIRST_CARD + 100_000))

    def cards_in(decks, each=2):
        return [queued(next(card), ids[deck]) for deck in decks for _ in range(each)]

    law_cards = cards_in(law)
    shared = cards_in([f"{LAW_ROOT}{SEP}Evidence"], each=3)
    borrowed = [
        queued(next(card), ids["Cram"], ids[f"{LAW_ROOT}{SEP}Method"]),
        queued(next(card), ids["Cram"], ids["Tongue Beta"]),
        queued(next(card), ids["Cram"], ids["Misc"]),
    ]
    unknown = [queued(next(card), 424_242), queued(next(card), ids["Cram"], 434_343)]
    drawn = [
        ("empty", {"deck_names": names, "queries": [], "undetermined": []}),
        ("empty", {"deck_names": {}, "queries": [], "undetermined": []}),
        (
            "every-track",
            {
                "deck_names": names,
                "queries": [
                    {"root_label": LAW_ROOT, "cards": law_cards},
                    {"root_label": "Tongue Alpha", "cards": cards_in(tongue[:2])},
                    {"root_label": "Tongue Beta", "cards": cards_in(tongue[2:])},
                    {"root_label": "Composition", "cards": cards_in(writing)},
                ],
                "undetermined": [],
            },
        ),
        (
            "shared-budget",
            {
                "deck_names": names,
                "queries": [
                    {"root_label": LAW_ROOT, "cards": shared + cards_in(law[:2], each=1)},
                    {"root_label": f"{LAW_ROOT}{SEP}Evidence", "cards": list(shared)},
                ],
                "undetermined": [],
            },
        ),
        (
            "filtered",
            {
                "deck_names": names,
                "queries": [
                    {"root_label": "Cram", "cards": borrowed},
                    {"root_label": "Tongue Beta", "cards": []},
                ],
                "undetermined": [],
            },
        ),
        (
            "unmapped",
            {
                "deck_names": names,
                "queries": [
                    {"root_label": "Misc", "cards": cards_in(["Misc"], each=3)},
                    {"root_label": LAW_ROOT, "cards": cards_in([LAW_ROOT, UNSAFE], each=2)},
                    {
                        "root_label": "Composition",
                        "cards": cards_in(["Composition", UNKNOWN_DISPLAY]),
                    },
                    {"root_label": "Unknown", "cards": unknown},
                ],
                "undetermined": [],
            },
        ),
        (
            "undetermined",
            {
                "deck_names": names,
                "queries": [
                    {"root_label": LAW_ROOT, "cards": cards_in(law[:3])},
                    {"root_label": "Tongue Alpha", "cards": cards_in(tongue[:2])},
                ],
                "undetermined": [
                    {"root_label": LAW_ROOT, "reason": REASONS[2]},
                    {"root_label": "Composition", "reason": REASONS[0]},
                ],
            },
        ),
        (
            "undetermined",
            {
                "deck_names": names,
                "queries": [],
                "undetermined": [
                    {"root_label": "Tongue Beta", "reason": REASONS[1]},
                    {"root_label": LAW_ROOT, "reason": REASONS[1]},
                    {"root_label": "Tongue Alpha", "reason": REASONS[0]},
                ],
            },
        ),
    ]
    for _ in range(12):
        queries = []
        for root in rng.sample(ROOTS, 3):
            members = [name for name in DECKS if top_level(name) == root]
            count = rng.randrange(0, 6)
            chosen = [queued(next(card), ids[rng.choice(members)]) for _ in range(count)]
            if root == "Cram":
                chosen = [dict(entry, odid=ids[rng.choice(DECKS)]) for entry in chosen]
            queries.append({"root_label": root, "cards": chosen})
        # A card an earlier query claimed, queried again under another root.
        if queries[0]["cards"]:
            queries[-1]["cards"].append(dict(rng.choice(queries[0]["cards"])))
        refused = []
        if rng.randrange(0, 3) == 0:
            refused.append({"root_label": queries[1]["root_label"], "reason": rng.choice(REASONS)})
        drawn.append(("drawn", {"deck_names": names, "queries": queries, "undetermined": refused}))
    return drawn


def digest_cases(rng):
    """Order, duplicates, width and sign, then drawn id lists."""
    drawn = [
        ("empty", {"card_ids": []}),
        ("one", {"card_ids": [FIRST_CARD]}),
        ("unsorted", {"card_ids": [FIRST_CARD + 3, FIRST_CARD + 1, FIRST_CARD + 2]}),
        ("unsorted", {"card_ids": [10, 9, 100]}),
        ("duplicates", {"card_ids": [FIRST_CARD + 2, FIRST_CARD + 1, FIRST_CARD + 2]}),
        ("wide", {"card_ids": [9_223_372_036_854_775_807, FIRST_CARD]}),
        ("negative", {"card_ids": [-1, 0, 1]}),
    ]
    for _ in range(12):
        drawn.append(
            (
                None,
                {
                    "card_ids": [
                        FIRST_CARD + rng.randrange(0, 50_000_000)
                        for _ in range(rng.randrange(1, 30))
                    ]
                },
            )
        )
    return drawn


FUNCTIONS = {
    "resolve_day_sets": {
        "kind": "adapter",
        "function": "prereading.resolve_day_sets",
        "adapter": resolved_under_the_synthetic_taxonomy,
        "note": (
            "Replaces every private deck constant the predecessor reads with the synthetic "
            "taxonomy of the readings' example taxonomy file, builds its Card, DaySetQuery and "
            "UndeterminedRoot from the case, calls it with no new-card limit lookup, and returns "
            "the resolution's topics, unmapped decks, roots with no new card and undetermined "
            "roots; its limit-suppressed topics, none without a lookup, are left out"
        ),
        "cases": day_set_cases,
    },
    "law_subject": {
        "kind": "adapter",
        "function": "leeches._law_subject",
        "adapter": under_the_synthetic_taxonomy,
        "note": (
            "Replaces every private deck constant the predecessor reads with the synthetic "
            "taxonomy of the readings' example taxonomy file, and calls it on the case's deck name"
        ),
        "cases": law_subject_cases,
    },
    "digest_for_card_ids": {
        "kind": "function",
        "function": "prereading._digest_for_card_ids",
        "cases": digest_cases,
    },
}

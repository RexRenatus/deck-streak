"""SPEC-498's registrations: every function of the engine the parity goldens record, with the
edge classes each declares (SPEC-498 R3, ADR-516).

The names are the goldens' names. The ids are the design's: F1 to F12 for import and export,
R-1 to R-12 for rendering, and the peers' G-L5-1, G-L5-2, G-L6-1, G-L6-2, G-L9-1 and GL-1 to
GL-4. Each note says what the harness calls and what it answers; the harness answers in the
corpus's own terms (row indices, never ids the engine assigns) and a refusal as a short label.

Each declared class's hand cases are its drawn case or cases and its written cases, which the
corpus's written texts, templates, masks, clocks and sketches give with no draw (`WRITTEN`).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_CORPUS_PATH = Path(__file__).resolve().parent.parent / "corpus.py"
_spec = importlib.util.spec_from_file_location("anki_corpus", _CORPUS_PATH)
corpus = importlib.util.module_from_spec(_spec)
sys.modules["anki_corpus"] = corpus
_spec.loader.exec_module(corpus)

#: The corpus this module draws from, pinned: a corpus edit must edit this module too, which
#: stales every golden it registered.
CORPUS = corpus
CORPUS_SHA256 = "8b59468578ce33bf436e87a852c4a01300833999b6e435a976e3f293bcc1fdd0"
SEED = corpus.SEED

word, words, timing, collection = corpus.word, corpus.words, corpus.timing, corpus.collection
render_text, render_template = corpus.render_text, corpus.render_template
builder, nfc, nfd = corpus.builder, corpus.nfc, corpus.nfd
RENDER = corpus.RANDOM_RENDER_CASES


def per_class(classes, make):
    """Hand cases: `make(edge, rng)` for each class, in the declared order."""
    return {edge: (lambda rng, edge=edge: make(edge, rng)) for edge in classes}


def register(id_, classes, note, make, random_make=None, count=corpus.RANDOM_CASES):
    writer = WRITERS.get(id_)
    written = {edge: writer(edge) for edge in classes} if writer else {}
    WRITTEN_BY_ID[id_] = written
    return {
        "id": id_,
        "classes": list(classes),
        "note": note,
        "cases": builder(per_class(classes, make), random_make, count, written),
    }


def edges_collection(edge, rng):
    return {"collection": collection(rng, (edge,))}


def random_collection(rng):
    return {"collection": collection(rng, tuple(rng.sample(sorted(corpus.CLASSES), 3)))}


# Written cases: each registration's hand cases built from the corpus's written texts,
# templates, masks, clocks and sketches, with no draw, so they read the same under any seed.
# A writer answers a class's written inputs; it reads module names only when called.

PLAIN, SKETCHES, TEXTS = corpus.PLAIN, corpus.SKETCHES, corpus.TEXTS_WRITTEN
#: Each registration's written cases by class, keyed by its id while the registry loads.
WRITTEN_BY_ID: dict[str, dict[str, list[dict]]] = {}


def sketched(edges):
    """One written collection per sketch, holding `edges`."""
    return [corpus.written_collection(sketch, edges) for sketch in SKETCHES]


def write_collection(edge):
    return [{"collection": source} for source in sketched((edge,))]


def write_package(edge):
    """A package of each sketch: with media files for the personal and shareable classes, and
    none for no-media."""
    kind = "shareable" if edge == "shareable" else "personal"
    edges = {
        "nfd-names": ("nfd-names",),
        "nfc-names": ("nfc-names",),
        "no-media": ("html",),
    }.get(edge, ("html", "unused-files"))
    return [{"collection": source, "kind": kind} for source in sketched(edges)]


def explicit_mods(target_note, package_note, mod):
    """A note the package shares with the target by GUID, with no mod on either side, takes one mod on both: the
    import then answers from the case's input, never from the clock that added the two notes."""
    if (
        package_note.get("guid") == target_note.get("guid")
        and "mod" not in package_note
        and "mod" not in target_note
    ):
        package_note["mod"], target_note["mod"] = mod, mod


def write_import(edge):
    cases = []
    for k in range(7):
        target = corpus.written_collection(SKETCHES[k])
        incoming = corpus.written_collection(SKETCHES[(k + 3) % 7])
        note = dict(target["notes"][0])
        mod, gap = corpus.MODS[k], corpus.GAPS[k]
        if edge == "newer":
            note["mod"], target["notes"][0]["mod"] = mod + gap, mod
        elif edge == "older":
            note["mod"], target["notes"][0]["mod"] = mod - gap, mod
        elif edge == "equal-mod":
            note["mod"], target["notes"][0]["mod"] = mod, mod
        elif edge == "notetype-mismatch":
            incoming["notetypes"][0] = dict(
                incoming["notetypes"][0], fields=["Prompt", "Answer", "Extra"]
            )
            note["fields"] = note["fields"] + [PLAIN[k]]
        elif edge == "new-guid":
            note["guid"] = corpus.NEW_GUIDS[k]
        elif edge == "revlog-id-clash":
            incoming["revlog"] = [dict(incoming["revlog"][0], at_ms=target["revlog"][0]["at_ms"])]
        explicit_mods(target["notes"][0], note, mod)
        incoming["notes"][0] = note
        cases.append({"collection": target, "package": incoming, "kind": "personal"})
    return cases


def write_timing(edge):
    return [dict(clock) for clock in corpus.CLOCKS_WRITTEN[edge]]


def write_due(edge):
    cases = []
    for k, clock in enumerate(corpus.CLOCKS):
        if edge == "odue-filtered":
            row = corpus.card(
                0, 0, 1, queue=2, kind=2, due=-100_000, odeck=0, odue=corpus.DUES[k] + 1
            )
        else:
            queue = QUEUE[edge]
            kind = {1: 1, 3: 1, 2: 2}.get(queue, 0)
            due = corpus.DUES[k]
            if queue == 1:
                due = clock["now"] + corpus.LEARNING_STEPS[k]
            elif queue == 3:
                due = (clock["now"] - clock["crt"]) // corpus.DAY + k - 3
            row = corpus.card(0, 0, 0, queue=queue, kind=kind, due=due, ivl=corpus.IVLS[k])
        cases.append({"card": row, "timing": dict(clock)})
    return cases


def write_checksum(edge):
    firsts = {
        "html": corpus.HTML_WRITTEN,
        "media-refs": TEXTS["media"],
        "entities": TEXTS["entities"],
        "empty": TEXTS["empty"],
        "cjk": corpus.CJK_WRITTEN,
    }[edge]
    return [{"fields": [first, PLAIN[k]], "sort_index": 0} for k, first in enumerate(firsts)]


def write_blob(edge):
    """The non-default class's written values, one seed per column; the default class's cases
    already cover every column, so it writes none."""
    if edge == "default-values":
        return []
    seeds = [1, 2, 3, 5, 8, 13, 21, 34]
    return [
        {"column": column, "values": {"seed": seed}}
        for column, seed in zip(BLOB_COLUMNS, seeds, strict=True)
    ]


def write_answer(edge):
    return [
        {
            "collection": source,
            "answer": {"card": k % 3, "ease": corpus.EASES[k], "at_ms": source["now"] * 1000},
        }
        for k, source in enumerate(sketched((edge,)))
    ]


def write_guid(edge):
    prefix = "vc1-" if edge == "vc1-prefix" else "ds-"
    cases = []
    for source in sketched(()):
        for note in source["notes"]:
            note["guid"] = prefix + note["guid"].removeprefix("ds-")
        cases.append({"collection": source})
    return cases


def write_card(edge):
    """R-1's written cases: a template class's written templates over plain fields, or a text
    class's written texts on the cloze or prompt-answer note type."""
    if edge in TEMPLATE_CLASSES:
        return [
            {
                "notetype": corpus.basic(front, back),
                "fields": [PLAIN[k], "" if edge == "empty-front" else PLAIN[(k + 1) % 7]],
                "ord": 0,
            }
            for k, (front, back) in enumerate(corpus.TEMPLATES_WRITTEN[edge])
        ]
    return [
        {
            "notetype": corpus.cloze_type() if edge in CLOZE else corpus.basic(),
            "fields": [text, PLAIN[k]],
            "ord": corpus.CARD_ORDS[k] if edge in CLOZE else 0,
        }
        for k, text in enumerate(TEXTS[edge])
    ]


def write_ordinals(edge):
    return [
        {key: value for key, value in case.items() if key != "ord"} for case in write_card(edge)
    ]


def write_text(key):
    return lambda edge: [{key: text} for text in TEXTS[edge]]


def write_cloze_text(edge):
    return [{"text": text, "ord": corpus.ORDS[k]} for k, text in enumerate(TEXTS[edge])]


def write_latex(edge):
    return [
        {"text": f"[latex]{text}[/latex] [$]{PLAIN[k]}[/$]", "svg": edge == "unicode-nfc"}
        for k, text in enumerate(TEXTS[edge])
    ]


def write_compare(edge):
    return [
        {
            "expected": text,
            "provided": text.upper() if edge == "case" else text.replace(" ", "", 1),
            "combining": edge != "unicode-nfd",
        }
        for text in TEXTS[edge]
    ]


def write_note_type(edge):
    if edge in TEMPLATE_CLASSES:
        return [
            {"notetype": corpus.basic(front, back)}
            for front, back in corpus.TEMPLATES_WRITTEN[edge]
        ]
    return [
        {
            "notetype": corpus.notetype(
                "ds.gap-text", ["Body", "Notes"], [("Gap", front, back)], kind="cloze"
            )
        }
        for front, back in corpus.CLOZE_TEMPLATES_WRITTEN[edge]
    ]


def write_note_check(edge):
    cases = []
    for k in range(7):
        if edge in ("missing-cloze", "cloze-single", "cloze-multi", "gap"):
            fields = [TEXTS[edge][k], ""]
            cases.append({"notetype": corpus.cloze_type(), "fields": fields, "existing": []})
            continue
        notetype, first, existing = corpus.basic(), PLAIN[k], []
        other = PLAIN[(k + 1) % 7]
        if edge == "empty":
            first = TEXTS["empty"][k]
        elif edge == "empty-front":
            notetype = corpus.basic(*corpus.TEMPLATES_WRITTEN["empty-front"][k])
        elif edge == "duplicate":
            first = corpus.DUPLICATE_WRITTEN[k]
            existing = [[first, other]]
        elif edge == "ws":
            first = TEXTS["ws"][k]
            existing = [[first.strip(), other]]
        elif edge in ("unicode-nfc", "unicode-nfd"):
            first = TEXTS[edge][k]
            existing = [[nfd(first) if edge == "unicode-nfc" else nfc(first), other]]
        answer = "" if edge == "empty-front" else PLAIN[(k + 2) % 7]
        cases.append({"notetype": notetype, "fields": [first, answer], "existing": existing})
    return cases


def write_occlusion(edge):
    mode = "hide-all" if edge == "occlusion-hide-all" else "hide-one"
    return [
        {
            "notetype": corpus.occlusion_type(),
            "fields": [masks, corpus.PICTURES[k], PLAIN[k]],
            "mode": mode,
            "ord": 0,
        }
        for k, masks in enumerate(corpus.MASKS_WRITTEN[edge])
    ]


def write_search(edge):
    edges = (edge, "rollover") if edge in ("added-at-rollover", "rated-at-rollover") else (edge,)
    return [
        {"collection": source, "search": SEARCHES[edge] + suffix}
        for source, suffix in zip(sketched(edges), corpus.SEARCH_SUFFIXES, strict=True)
    ]


def write_generated(edge):
    if edge == "two-templates":
        return [{"notetype": corpus.two_way(), "fields": list(f)} for f in corpus.TWO_WAY_FIELDS]
    return write_ordinals(edge)


#: Each registration's writer, by id. F12, the schema snapshot, declares no class.
WRITERS = {
    "F1": write_collection,
    "F2": write_package,
    "F3": write_import,
    "F4": write_timing,
    "F5": write_due,
    "F6": write_checksum,
    "F7": lambda edge: [{"names": list(names)} for names in corpus.NAMES_WRITTEN[edge]],
    "F8": lambda edge: [{"names": list(names)} for names in corpus.COLLATION_WRITTEN[edge]],
    "F9": write_blob,
    "F10": write_answer,
    "F11": write_guid,
    "R-1": write_card,
    "R-2": write_ordinals,
    "R-3": write_cloze_text,
    "R-4": write_text("text"),
    "R-5": write_text("text"),
    "R-6": write_text("html"),
    "R-7": write_text("html"),
    "R-8": write_latex,
    "R-9": write_compare,
    "R-10": write_note_type,
    "R-11": write_note_check,
    "R-12": write_occlusion,
    "G-L5-1": lambda edge: [{"html": html} for html in corpus.REFS_WRITTEN[edge]],
    "G-L5-2": lambda edge: [{"collection": source} for source in sketched((edge, "media-refs"))],
    "G-L6-1": write_search,
    "G-L6-2": write_search,
    "G-L9-1": lambda edge: [
        {"collection": source}
        for source in sketched((edge,) if edge == "rollover" else (edge, "rollover"))
    ],
    "GL-1": write_timing,
    "GL-2": lambda edge: [{"a": a, "b": b} for a, b in corpus.DECK_PAIRS_WRITTEN[edge]],
    "GL-3": write_collection,
    "GL-4": write_generated,
}


# F1 to F12: import and export.

F1_CLASSES = ["unicase-names", "cjk", "empty-fields", "filtered-deck", "ease-0"] + [
    f"revlog-type-{k}" for k in range(6)
]
F2_CLASSES = ["personal", "shareable", "nfd-names", "nfc-names", "no-media"]
F3_CLASSES = ["newer", "older", "equal-mod", "notetype-mismatch", "new-guid", "revlog-id-clash"]
F5_CLASSES = [
    "queue-m3",
    "queue-m2",
    "queue-m1",
    "queue-0",
    "queue-1",
    "queue-2",
    "queue-3",
    "odue-filtered",
]
QUEUE = {
    "queue-m3": -3,
    "queue-m2": -2,
    "queue-m1": -1,
    "queue-0": 0,
    "queue-1": 1,
    "queue-2": 2,
    "queue-3": 3,
}


def package_case(edge, rng):
    kind = "shareable" if edge == "shareable" else "personal"
    edges = {"nfd-names": ("nfd-names",), "nfc-names": ("nfc-names",)}.get(edge, ("html",))
    source = collection(rng, edges)
    if edge == "no-media":
        source["media"] = []
    return {"collection": source, "kind": kind}


def import_case(edge, rng):
    target = collection(rng)
    incoming = collection(rng)
    note = dict(target["notes"][0])
    mod = 1_000_000
    if edge == "newer":
        note["mod"], target["notes"][0]["mod"] = mod + 100, mod
    elif edge == "older":
        note["mod"], target["notes"][0]["mod"] = mod - 100, mod
    elif edge == "equal-mod":
        note["mod"], target["notes"][0]["mod"] = mod, mod
    elif edge == "notetype-mismatch":
        incoming["notetypes"][0] = dict(
            incoming["notetypes"][0], fields=["Prompt", "Answer", "Extra"]
        )
        note["fields"] = note["fields"] + [word(rng)]
    elif edge == "new-guid":
        note["guid"] = f"ds-{rng.randrange(16**8):08x}"
    elif edge == "revlog-id-clash":
        incoming["revlog"] = [dict(incoming["revlog"][0], at_ms=target["revlog"][0]["at_ms"])]
    explicit_mods(target["notes"][0], note, mod)
    incoming["notes"][0] = note
    return {"collection": target, "package": incoming, "kind": "personal"}


def due_case(edge, rng):
    clock = timing(rng)
    if edge == "odue-filtered":
        row = corpus.card(
            0, 0, 1, queue=2, kind=2, due=-100_000, odeck=0, odue=rng.randrange(1, 900)
        )
    else:
        queue = QUEUE[edge]
        kind = {1: 1, 3: 1, 2: 2}.get(queue, 0)
        due = clock["now"] + 600 if queue == 1 else rng.randrange(0, 900)
        row = corpus.card(0, 0, 0, queue=queue, kind=kind, due=due, ivl=rng.randrange(0, 60))
    return {"card": row, "timing": clock}


def checksum_case(edge, rng):
    fields = {
        "html": [f"<b>{word(rng)}</b><br>{word(rng)}", word(rng)],
        "media-refs": [f'<img src="ds_{word(rng)}.png">{word(rng)}', "[sound:ds_a.mp3]"],
        "entities": [f"{word(rng)}&nbsp;&amp;&lt;", word(rng)],
        "empty": ["", ""],
        "cjk": [rng.choice(corpus.CJK), rng.choice(corpus.CJK)],
    }
    return {"fields": fields[edge], "sort_index": 0}


def media_name_case(edge, rng):
    names = {
        "nfd": [nfd("ds_café.png"), nfd("ds_über.mp3")],
        "forbidden-chars": ["ds:a.png", "ds/b.png", 'ds"c?.png', "ds<d>|e*.png", "ds\\f.png"],
        "long-names": [f"ds_{'x' * 300}.png", f"ds_{'é' * 200}.png"],
    }
    return {"names": names[edge]}


def collation_case(edge, rng):
    names = {
        "case-pairs": ["Ds Alpha", "ds alpha", "DS ALPHA", "Ds Beta"],
        "non-ascii-folding": ["École", "ecole", "école", "Straße", "STRASSE", "İstanbul"],
    }
    return {"names": names[edge]}


BLOB_COLUMNS = [
    "deck_config.config",
    "decks.common",
    "decks.kind",
    "notetypes.config",
    "fields.config",
    "templates.config",
    "tags.config",
    "config.val",
]


def blob_case(edge, rng):
    return [
        {
            "column": column,
            "values": "default" if edge == "default-values" else {"seed": rng.randrange(1, 10**6)},
        }
        for column in BLOB_COLUMNS
    ]


def answer_case(edge, rng):
    source = collection(rng, (edge,))
    return {
        "collection": source,
        "answer": {"card": 0, "ease": rng.randint(1, 4), "at_ms": source["now"] * 1000},
    }


def guid_case(edge, rng):
    source = collection(rng)
    prefix = "vc1-" if edge == "vc1-prefix" else "ds-"
    for note in source["notes"]:
        note["guid"] = f"{prefix}{rng.randrange(16**10):010x}"
    return {"collection": source}


FUNCTIONS = {
    "anki.read.collection_view": register(
        "F1",
        F1_CLASSES,
        "builds the collection through the engine, then answers its own view of each table",
        edges_collection,
        random_collection,
    ),
    "anki.export.package": register(
        "F2",
        F2_CLASSES,
        "exports the collection as a package of the kind, then answers its members, each member's compression, the rows per table and the media map",
        package_case,
        lambda rng: package_case(None, rng),
    ),
    "anki.import.package": register(
        "F3",
        F3_CLASSES,
        "imports the package into the collection, then answers the rows that changed and the engine's counts",
        import_case,
        lambda rng: import_case(None, rng),
    ),
    "anki.timing.day": register(
        "F4",
        ["rollover", "negative-offset", "crt-after-now"],
        "answers the engine's day number and next rollover for the creation time, rollover hour, offset and instant",
        lambda edge, rng: timing(rng, edge),
        lambda rng: timing(rng),
    ),
    "anki.card.due_view": register(
        "F5",
        F5_CLASSES,
        "answers the due the engine reports for the card's scheduling columns at the instant",
        due_case,
        lambda rng: due_case(rng.choice(F5_CLASSES), rng),
    ),
    "anki.note.checksum": register(
        "F6",
        ["html", "media-refs", "entities", "empty", "cjk"],
        "answers the sort field text and checksum the engine stores for the fields",
        checksum_case,
        lambda rng: {
            "fields": [render_text(None, rng), words(rng)],
            "sort_index": rng.randrange(0, 2),
        },
    ),
    "anki.media.name": register(
        "F7",
        ["nfd", "forbidden-chars", "long-names"],
        "answers the name the engine stores for each file name",
        media_name_case,
        lambda rng: {"names": [f"ds_{word(rng)}.png"]},
    ),
    "anki.collation.order": register(
        "F8",
        ["case-pairs", "non-ascii-folding"],
        "answers the names in the order the engine's case-insensitive collation sorts them",
        collation_case,
        lambda rng: {"names": rng.sample(corpus.WORDS, 5)},
    ),
    "anki.blob.encode": register(
        "F9",
        ["default-values", "non-default-values"],
        "answers the bytes, as hex, the engine stores for the typed view of each blob column",
        blob_case,
        lambda rng: {
            "column": rng.choice(BLOB_COLUMNS),
            "values": {"seed": rng.randrange(1, 10**6)},
        },
    ),
    "anki.history.local_answer": register(
        "F10",
        ["before-sync", "after-sync"],
        "answers the card in the collection, then answers the review log row it writes, with its usn",
        answer_case,
    ),
    "anki.guid.accepts": register(
        "F11",
        ["vc1-prefix", "ds-prefix"],
        "exports the collection and imports it into a fresh one, then answers whether each GUID was kept",
        guid_case,
    ),
    "anki.schema.snapshot": {
        "id": "F12",
        "classes": [],
        "note": "creates a fresh collection, then answers its schema text, user_version and col.ver",
        "cases": builder({}, lambda rng: {"fresh": True}, 1),
    },
}

# R-1 to R-12: rendering.

TEMPLATE_CLASSES = ["case", "ws", "missing", "special", "empty-front", "unbal", "chain", "unknown"]
TEXT_CLASSES = [
    "empty",
    "entities",
    "media",
    "cloze-single",
    "cloze-hint",
    "cloze-multi",
    "cloze-nest-1",
    "cloze-nest-2",
    "cloze-nest-3",
    "cloze-nest-4",
    "cloze-math",
    "cloze-span",
    "gap",
    "unicode-nfc",
    "unicode-nfd",
    "unicode-rtl",
    "unicode-cjk-furigana",
    "unicode-emoji",
    "large",
]
CLOZE = {"cloze-single", "cloze-hint", "cloze-multi", "cloze-math", "cloze-span", "gap"} | {
    f"cloze-nest-{d}" for d in range(1, 5)
}


def card_case(edge, rng):
    """A note and its note type: a cloze type for a cloze class, else a prompt-answer type."""
    if edge in CLOZE:
        return {
            "notetype": corpus.cloze_type(),
            "fields": [render_text(edge, rng), words(rng)],
            "ord": 0,
        }
    front, back = render_template(edge, rng)
    nt = corpus.basic(front, back)
    text = render_text(edge if edge in TEXT_CLASSES else None, rng)
    answer = "" if edge == "empty-front" else words(rng)
    return {"notetype": nt, "fields": [text, answer], "ord": 0}


def random_card(rng):
    if rng.random() < 0.3:
        return {
            "notetype": corpus.cloze_type(),
            "fields": [render_text(rng.choice(sorted(CLOZE)), rng), words(rng)],
            "ord": rng.randrange(0, 3),
        }
    front, back = render_template(None, rng)
    return {
        "notetype": corpus.basic(front, back),
        "fields": [render_text(None, rng), render_text(None, rng)],
        "ord": 0,
    }


def text_case(key):
    return lambda edge, rng: {key: render_text(edge, rng)}


def random_text(key):
    return lambda rng: {key: render_text(None, rng)}


ORDINAL_CLASSES = [
    "empty",
    "empty-front",
    "missing",
    "special",
    "cloze-single",
    "cloze-multi",
    "cloze-nest-1",
    "cloze-nest-2",
    "cloze-nest-3",
    "cloze-nest-4",
    "gap",
    "large",
]
CLOZE_TEXT_CLASSES = sorted(CLOZE) + ["empty", "unicode-nfc", "unicode-nfd", "unicode-emoji"]


def ordinal_case(edge, rng):
    case = card_case(edge, rng)
    del case["ord"]
    return case


def random_ordinals(rng):
    case = random_card(rng)
    del case["ord"]
    return case


def cloze_text_case(edge, rng):
    return {"text": render_text(edge, rng), "ord": 1}


def compare_case(edge, rng):
    expected = render_text(edge, rng)
    provided = expected.upper() if edge == "case" else expected.replace(" ", "", 1)
    return {"expected": expected, "provided": provided, "combining": edge != "unicode-nfd"}


NOTE_CHECK_CLASSES = [
    "empty",
    "empty-front",
    "duplicate",
    "missing-cloze",
    "cloze-single",
    "cloze-multi",
    "gap",
    "ws",
    "unicode-nfc",
    "unicode-nfd",
]


def note_check_case(edge, rng):
    if edge in ("missing-cloze", "cloze-single", "cloze-multi", "gap"):
        text = words(rng) if edge == "missing-cloze" else render_text(edge, rng)
        return {"notetype": corpus.cloze_type(), "fields": [text, ""], "existing": []}
    first = render_text(edge if edge in TEXT_CLASSES or edge == "ws" else None, rng)
    if edge == "empty-front":
        first = ""
    existing = [[first, words(rng)]] if edge == "duplicate" else []
    if edge in ("unicode-nfc", "unicode-nfd"):
        existing = [[nfd(first) if edge == "unicode-nfc" else nfc(first), words(rng)]]
    return {"notetype": corpus.basic(), "fields": [first, words(rng)], "existing": existing}


OCCLUSION_CLASSES = [
    "occlusion-rect",
    "occlusion-ellipse",
    "occlusion-polygon",
    "occlusion-text",
    "occlusion-hide-one",
    "occlusion-hide-all",
    "gap",
    "empty",
]
SHAPES = {
    "occlusion-rect": "rect:left=.1:top=.2:width=.3:height=.1",
    "occlusion-ellipse": "ellipse:left=.4:top=.4:rx=.1:ry=.05",
    "occlusion-polygon": "polygon:points=.1,.1 .3,.1 .2,.3",
    "occlusion-text": "text:left=.5:top=.6:text=ds",
}


def occlusion_case(edge, rng):
    shapes = list(SHAPES.values())
    if edge in SHAPES:
        masks = f"{{{{c1::image-occlusion:{SHAPES[edge]}}}}}"
    elif edge == "gap":
        masks = f"{{{{c1::image-occlusion:{shapes[0]}}}}} {{{{c3::image-occlusion:{shapes[1]}}}}}"
    elif edge == "empty":
        masks = ""
    else:
        masks = " ".join(
            f"{{{{c{i + 1}::image-occlusion:{shape}}}}}" for i, shape in enumerate(shapes)
        )
    mode = "hide-all" if edge == "occlusion-hide-all" else "hide-one"
    return {
        "notetype": corpus.occlusion_type(),
        "fields": [masks, '<img src="ds_map.png">', words(rng)],
        "mode": mode,
        "ord": 0,
    }


FUNCTIONS.update(
    {
        "anki.render.card": register(
            "R-1",
            TEMPLATE_CLASSES + TEXT_CLASSES,
            "renders both sides of the card of the ordinal, with FrontSide filled, and answers the question and answer HTML",
            card_case,
            random_card,
            RENDER,
        ),
        "anki.render.card_ordinals": register(
            "R-2",
            ORDINAL_CLASSES,
            "answers the ordinals of the cards the engine would generate for the note",
            ordinal_case,
            random_ordinals,
            RENDER,
        ),
        "anki.render.cloze_only_text": register(
            "R-3",
            CLOZE_TEXT_CLASSES,
            "answers the text of the cloze deletions of the ordinal",
            cloze_text_case,
            lambda rng: {"text": render_text(None, rng), "ord": rng.randrange(1, 4)},
            RENDER,
        ),
        "anki.render.extract_av_tags": register(
            "R-4",
            ["media", "entities", "empty", "unicode-nfd", "unicode-cjk-furigana", "large"],
            "answers the text with its audio and video tags replaced, and the tags it found",
            text_case("text"),
            random_text("text"),
            RENDER,
        ),
        "anki.render.strip_av_tags": register(
            "R-5",
            ["media", "entities", "empty", "unicode-emoji", "large"],
            "answers the text with its audio and video tags removed",
            text_case("text"),
            random_text("text"),
            RENDER,
        ),
        "anki.render.html_to_text_line": register(
            "R-6",
            [
                "entities",
                "media",
                "ws",
                "empty",
                "unbal",
                "unicode-rtl",
                "unicode-cjk-furigana",
                "unicode-emoji",
                "large",
            ],
            "answers the HTML as one line of text",
            text_case("html"),
            random_text("html"),
            RENDER,
        ),
        "anki.render.media_refs": register(
            "R-7",
            ["media", "entities", "case", "empty", "unicode-nfc", "unicode-nfd", "large"],
            "answers the media names the engine's reference finder reports, in order",
            text_case("html"),
            random_text("html"),
            RENDER,
        ),
        "anki.render.latex_images": register(
            "R-8",
            ["cloze-math", "entities", "empty", "unicode-nfc", "large"],
            "answers the LaTeX images the text would need, by name, and the text that names them",
            lambda edge, rng: {
                "text": f"[latex]{render_text(edge, rng)}[/latex] [$]{word(rng)}[/$]",
                "svg": edge == "unicode-nfc",
            },
            lambda rng: {"text": f"[$$]{render_text(None, rng)}[/$$]", "svg": rng.random() < 0.5},
            RENDER,
        ),
        "anki.render.compare_answer": register(
            "R-9",
            [
                "case",
                "ws",
                "empty",
                "entities",
                "unicode-nfc",
                "unicode-nfd",
                "unicode-rtl",
                "unicode-cjk-furigana",
                "unicode-emoji",
                "large",
            ],
            "answers the type-in comparison HTML for the expected and provided answers",
            compare_case,
            lambda rng: compare_case(None, rng),
            RENDER,
        ),
        "anki.render.note_type_check": register(
            "R-10",
            TEMPLATE_CLASSES + ["cloze-single", "gap"],
            "adds the note type, then answers ok or the label of the engine's refusal",
            lambda edge, rng: {"notetype": card_case(edge, rng)["notetype"]},
            lambda rng: {"notetype": random_card(rng)["notetype"]},
            RENDER,
        ),
        "anki.render.note_check": register(
            "R-11",
            NOTE_CHECK_CLASSES,
            "adds the existing notes, then answers the engine's check of the note: ok, empty, duplicate or missing cloze",
            note_check_case,
            lambda rng: note_check_case("ws", rng),
            RENDER,
        ),
        "anki.render.occlusion_render": register(
            "R-12",
            OCCLUSION_CLASSES,
            "renders both sides of the image occlusion card of the ordinal",
            occlusion_case,
            lambda rng: occlusion_case(rng.choice(OCCLUSION_CLASSES), rng),
            RENDER,
        ),
    }
)

# The peers' goldens: media (G-L5-1, G-L5-2), search (G-L6-1, G-L6-2), history (G-L9-1) and
# storage (GL-1 to GL-4).

REF_CLASSES = [
    "quotes",
    "case",
    "percent-escape",
    "url-schemes",
    "sound-tag",
    "nested-markup",
    "broken-markup",
    "underscore-names",
    "latex",
]


def field_refs_case(edge, rng):
    name = f"ds_{word(rng)}"
    html = {
        "quotes": f"<img src=\"{name}.png\"><img src='{name}2.png'><img src={name}3.png>",
        "case": f'<IMG SRC="{name}.png"><Img Src="{name}.PNG">',
        "percent-escape": f'<img src="{name}%20one%2Bx.png">',
        "url-schemes": f'<img src="https://example.invalid/{name}.png"><img src="data:image/png;base64,AAAA"><img src="file:///{name}.png">',
        "sound-tag": f"[sound:{name}.mp3] [sound:{name} two.ogg]",
        "nested-markup": f'<div><span><img src="{name}.png"></span></div>',
        "broken-markup": f'<img src="{name}.png" <b>{word(rng)}',
        "underscore-names": f'<img src="_{name}.png"> [sound:_{name}.mp3]',
        "latex": f"[latex]{word(rng)}^2[/latex] [$]{word(rng)}[/$]",
    }[edge]
    return {"html": html}


SEARCHES = {
    "rollover": "prop:due=0",
    "filtered-deck": "deck:filtered",
    "buried": "is:buried",
    "suspended": "is:suspended",
    "intraday-learning": "is:learn",
    "day-learning": "is:learn is:review",
    "deck-children": 'deck:"Ds Home"',
    "unicode-deck-names": f"deck:Ds*{corpus.CJK[0]}*",
    "unicode-case": "cafÉ",
    "html-in-fields": "<b>",
    "wildcards": "am*r ba_il",
    "field-names": "Prompt:*",
    "added-at-rollover": "added:1",
    "rated-at-rollover": "rated:1",
}


def search_case(edge, rng):
    source = collection(
        rng, (edge, "rollover") if edge in ("added-at-rollover", "rated-at-rollover") else (edge,)
    )
    return {"collection": source, "search": SEARCHES[edge]}


def random_search(rng):
    edge = rng.choice(sorted(SEARCHES))
    return {"collection": collection(rng, (edge,)), "search": f"{SEARCHES[edge]} {word(rng)}"}


G_L6_1 = [
    "rollover",
    "filtered-deck",
    "buried",
    "suspended",
    "intraday-learning",
    "day-learning",
    "deck-children",
    "unicode-deck-names",
]
G_L6_2 = [
    "unicode-case",
    "html-in-fields",
    "wildcards",
    "field-names",
    "added-at-rollover",
    "rated-at-rollover",
]
G_L9_1 = [f"revlog-type-{k}" for k in range(6)] + ["rollover"]
GL_3 = ["normal", "suspended", "buried", "filtered-deck", "notetype-kinds"] + [
    f"revlog-type-{k}" for k in range(6)
]


def deck_pair_case(edge, rng):
    name = f"Ds {word(rng).title()} Café"
    if edge == "case-forms":
        return {"a": name, "b": name.upper()}
    return {"a": nfc(name), "b": nfd(name)}


def ordinals_case(edge, rng):
    if edge == "two-templates":
        return {"notetype": corpus.two_way(), "fields": [words(rng), words(rng)]}
    return ordinal_case(edge, rng)


FUNCTIONS.update(
    {
        "anki.media.field_refs": register(
            "G-L5-1",
            REF_CLASSES,
            "answers the media names the engine extracts from the field HTML",
            field_refs_case,
            random_text("html"),
        ),
        "anki.media.check": register(
            "G-L5-2",
            ["underscore-files", "latex-notes", "missing-files", "unused-files", "css-fonts"],
            "runs the engine's media check over the collection and its media folder, then answers the unused and missing names",
            lambda edge, rng: {"collection": collection(rng, (edge, "media-refs"))},
            lambda rng: {"collection": collection(rng, ("missing-files", "media-refs"))},
        ),
        "anki.search.operator": register(
            "G-L6-1",
            G_L6_1,
            "answers the corpus indices of the cards the engine's search finds, in the engine's order",
            search_case,
            random_search,
        ),
        "anki.search.browser": register(
            "G-L6-2",
            G_L6_2,
            "answers the corpus indices of the cards the engine's browser search finds, in the engine's order",
            search_case,
            random_search,
        ),
        "anki.history.revlog_events": register(
            "G-L9-1",
            G_L9_1,
            "answers each review log row the engine holds, keyed by its id, with the study day number it falls on",
            lambda edge, rng: {
                "collection": collection(rng, (edge,) if edge == "rollover" else (edge, "rollover"))
            },
            random_collection,
        ),
        "anki.sched.timing_today": register(
            "GL-1",
            ["rollover", "negative-offset"],
            "answers the engine's day number and next rollover",
            lambda edge, rng: timing(rng, edge),
            lambda rng: timing(rng),
        ),
        "anki.deck.name_equal": register(
            "GL-2",
            ["case-forms", "unicode-forms"],
            "adds a deck of each name, then answers whether the engine kept one deck or two",
            deck_pair_case,
        ),
        "anki.read.reader_fixture": register(
            "GL-3",
            GL_3,
            "builds the collection through the engine, then answers the schema-18 rows it wrote, by corpus index",
            edges_collection,
            random_collection,
        ),
        "anki.render.generated_ordinals": register(
            "GL-4",
            ["cloze-single", "cloze-multi", "gap", "empty-front", "two-templates"],
            "adds the note, then answers the ordinals of the cards the engine generated",
            ordinals_case,
            random_ordinals,
        ),
    }
)

#: Each golden's written cases by class: hand cases that draw nothing, beside the drawn ones.
WRITTEN = {
    name: WRITTEN_BY_ID[registration["id"]]
    for name, registration in FUNCTIONS.items()
    if WRITTEN_BY_ID.get(registration["id"])
}

"""The oracle's synthetic corpus: DeckStreak-authored note types, field texts and collections,
drawn from one seed, with the edge classes the registry declares (SPEC-498, ADR-516).

Nothing here is anyone's data and nothing is the engine's own text: every note type, template,
style sheet and field text is written for the corpus, and every name is invented. A case's input
is plain JSON. A collection names its rows by their index in the corpus, never by an id the
engine assigns, and a time is a whole number of seconds or milliseconds since the Unix epoch,
never a calendar string, so a golden holds no id or clock of the run.
"""

from __future__ import annotations

import random
import unicodedata
from collections.abc import Callable

#: The corpus's one seed: every case builder draws from `random.Random(SEED)` only.
SEED = 498
#: Seeded grammar-random cases per render function, beside the hand-written cases per class.
RANDOM_RENDER_CASES = 800
#: Seeded random cases per other function.
RANDOM_CASES = 24

#: Every edge class a registration may declare, with what its cases hold.
CLASSES = {
    # Collections and their tables.
    "unicase-names": "deck and note type names that differ only by letter case",
    "cjk": "names and field texts in Chinese, Japanese or Korean script",
    "empty-fields": "notes with one or more empty fields",
    "filtered-deck": "a card in a filtered deck, with its home deck and original due",
    "revlog-type-0": "a review log row of type 0 (learning)",
    "revlog-type-1": "a review log row of type 1 (review)",
    "revlog-type-2": "a review log row of type 2 (relearning)",
    "revlog-type-3": "a review log row of type 3 (filtered)",
    "revlog-type-4": "a review log row of type 4 (manual)",
    "revlog-type-5": "a review log row of type 5 (rescheduled)",
    "ease-0": "a review log row with ease 0",
    "normal": "a card in its home deck, neither suspended nor buried",
    "suspended": "a suspended card",
    "buried": "a buried card",
    "notetype-kinds": "one note of each note type kind: standard, cloze and image occlusion",
    # Packages.
    "personal": "a package that carries scheduling and history",
    "shareable": "a package with no scheduling and no history",
    "nfd-names": "media names in decomposed Unicode form",
    "nfc-names": "media names in composed Unicode form",
    "no-media": "a package with no media file",
    "newer": "an incoming note modified after the existing one",
    "older": "an incoming note modified before the existing one",
    "equal-mod": "an incoming note with the existing one's modification time",
    "notetype-mismatch": "an incoming note whose note type's fields differ from the existing one's",
    "new-guid": "an incoming note with a GUID the collection does not hold",
    "revlog-id-clash": "an incoming review log row whose id the collection already holds",
    # Timing and due dates.
    "rollover": "an instant on either side of the study day's rollover hour",
    "negative-offset": "a UTC offset west of Greenwich",
    "crt-after-now": "a collection creation time after the instant asked about",
    "queue-m3": "a card in queue -3 (buried by the learner)",
    "queue-m2": "a card in queue -2 (buried by its siblings)",
    "queue-m1": "a card in queue -1 (suspended)",
    "queue-0": "a card in queue 0 (new)",
    "queue-1": "a card in queue 1 (learning)",
    "queue-2": "a card in queue 2 (review)",
    "queue-3": "a card in queue 3 (day learning)",
    "odue-filtered": "a filtered card's original due",
    # Note texts.
    "html": "field texts with HTML markup",
    "media-refs": "field texts that reference media files",
    "entities": "HTML character entities",
    "empty": "an empty field or text",
    # Names and their order.
    "nfd": "a name in decomposed Unicode form",
    "forbidden-chars": "a name with characters a file name may not hold",
    "long-names": "a name longer than a file system allows",
    "case-pairs": "names that differ only by letter case",
    "non-ascii-folding": "names whose letters fold outside ASCII",
    "case-forms": "deck names that differ by letter case",
    "unicode-forms": "deck names that differ by Unicode normalisation",
    # Blobs.
    "default-values": "a blob view holding only default values",
    "non-default-values": "a blob view holding values other than the defaults",
    # History and identity.
    "before-sync": "a local answer in a collection that has never synced",
    "after-sync": "a local answer in a collection after a sync",
    "vc1-prefix": "a GUID with the vc1- prefix",
    "ds-prefix": "a GUID with a DeckStreak prefix",
    # Templates and rendering.
    "case": "field references whose letter case differs from the field's name",
    "ws": "whitespace inside and around template tags",
    "missing": "a reference to a field the note type does not have",
    "special": "the special fields (FrontSide, Tags, Type, Deck, Subdeck, Card, CardFlag)",
    "empty-front": "a template whose front renders empty",
    "unbal": "an unbalanced section or tag",
    "chain": "a chain of filters on one field",
    "unknown": "an unknown filter",
    "media": "sound tags and image references",
    "cloze-single": "one cloze deletion",
    "cloze-hint": "a cloze deletion with a hint",
    "cloze-multi": "several cloze deletions",
    "cloze-nest-1": "a cloze deletion nested one level",
    "cloze-nest-2": "cloze deletions nested two levels",
    "cloze-nest-3": "cloze deletions nested three levels",
    "cloze-nest-4": "cloze deletions nested four levels",
    "cloze-math": "a cloze deletion inside or around MathJax",
    "cloze-span": "a cloze deletion across HTML tags",
    "gap": "cloze ordinals with a gap",
    "unicode-nfc": "text in composed Unicode form",
    "unicode-nfd": "text in decomposed Unicode form",
    "unicode-rtl": "right-to-left text",
    "unicode-cjk-furigana": "CJK text with furigana readings",
    "unicode-emoji": "emoji, including joined sequences",
    "large": "a field of tens of thousands of characters",
    "duplicate": "a note whose first field another note of its type already holds",
    "missing-cloze": "a cloze note with no deletion",
    "two-templates": "a note type with two card templates",
    "occlusion-rect": "a rectangle occlusion",
    "occlusion-ellipse": "an ellipse occlusion",
    "occlusion-polygon": "a polygon occlusion",
    "occlusion-text": "a text occlusion",
    "occlusion-hide-one": "occlusions that hide one shape at a time",
    "occlusion-hide-all": "occlusions that hide every shape",
    # Media.
    "quotes": "image references with single, double and no quotes",
    "url-schemes": "references with a URL scheme",
    "percent-escape": "percent-escaped names",
    "sound-tag": "sound tags",
    "nested-markup": "references inside nested markup",
    "broken-markup": "references inside broken markup",
    "underscore-names": "names that start with an underscore",
    "latex": "LaTeX in a field",
    "missing-files": "references to files the media folder lacks",
    "unused-files": "files no note references",
    "css-fonts": "font files a style sheet references",
    "underscore-files": "media files whose names start with an underscore",
    "latex-notes": "notes whose fields hold LaTeX",
    # Search.
    "intraday-learning": "a card in intraday learning",
    "day-learning": "a card in day learning",
    "deck-children": "a search on a deck with children",
    "unicode-deck-names": "deck names outside ASCII",
    "unicode-case": "search terms that differ by Unicode letter case",
    "html-in-fields": "a search over fields that hold HTML",
    "wildcards": "search terms with wildcards",
    "field-names": "searches by field name",
    "added-at-rollover": "an added search across the rollover",
    "rated-at-rollover": "a rated search across the rollover",
}

WORDS = [
    "amber",
    "basil",
    "cobalt",
    "dune",
    "ember",
    "fjord",
    "garnet",
    "harbor",
    "indigo",
    "juniper",
]
CJK = ["山水", "読書", "학교", "電車", "花火"]
RTL = ["שלום עולם", "مرحبا بكم"]
EMOJI = ["\U0001f642", "\U0001f469‍\U0001f52c", "\U0001f1f3\U0001f1f4", "❤️"]
DAY = 86_400
#: A base instant: 2001-09-09 01:46:40 UTC, held as seconds so no calendar string appears.
BASE = 1_000_000_000

Make = Callable[[random.Random], dict | list]


def word(rng: random.Random) -> str:
    return rng.choice(WORDS)


def words(rng: random.Random, low: int = 1, high: int = 6) -> str:
    return " ".join(word(rng) for _ in range(rng.randint(low, high)))


def nfd(text: str) -> str:
    return unicodedata.normalize("NFD", text)


def nfc(text: str) -> str:
    return unicodedata.normalize("NFC", text)


def notetype(
    name: str,
    fields: list[str],
    templates: list[tuple[str, str, str]],
    kind: str = "standard",
    css: str = ".card { font-family: system-ui; font-size: 18px; margin: 12px; }",
) -> dict:
    """A DeckStreak-authored note type. Each template is (name, front, back), on one line each."""
    return {
        "name": name,
        "kind": kind,
        "fields": fields,
        "templates": [{"name": n, "front": f, "back": b} for n, f, b in templates],
        "css": css,
        "sort_field": 0,
    }


def basic(front: str = "{{Prompt}}", back: str = "{{FrontSide}}<hr class=ds-answer>{{Answer}}"):
    return notetype("ds.prompt-answer", ["Prompt", "Answer"], [("Forward", front, back)])


def cloze_type() -> dict:
    return notetype(
        "ds.gap-text",
        ["Body", "Notes"],
        [
            (
                "Gap",
                "<div class=ds-gap>{{cloze:Body}}</div>",
                "<div class=ds-gap>{{cloze:Body}}</div>{{Notes}}",
            )
        ],
        kind="cloze",
    )


def occlusion_type() -> dict:
    return notetype(
        "ds.masked-image",
        ["Masks", "Picture", "Caption"],
        [
            (
                "Mask",
                "<div class=ds-mask>{{image-occlusion:Masks}}</div>{{Picture}}",
                "<div class=ds-mask>{{image-occlusion:Masks}}</div>{{Picture}}<p>{{Caption}}</p>",
            )
        ],
        kind="occlusion",
    )


def two_way() -> dict:
    return notetype(
        "ds.both-ways",
        ["Prompt", "Answer"],
        [
            ("Forward", "{{Prompt}}", "{{FrontSide}}<hr class=ds-answer>{{Answer}}"),
            ("Backward", "{{Answer}}", "{{FrontSide}}<hr class=ds-answer>{{Prompt}}"),
        ],
    )


def nest(depth: int, rng: random.Random) -> str:
    """Cloze deletions nested `depth` levels: c1 holds c2, which holds c3, and on."""
    text = word(rng)
    for level in range(depth, 0, -1):
        text = f"{{{{c{level}::{word(rng)} {text}}}}}"
    return text


def render_text(edge: str | None, rng: random.Random) -> str:
    """A field text for the render class `edge`, or a grammar-random one for None."""
    if edge is not None:
        return TEXTS[edge](rng)
    pieces = [words(rng, 1, 3)]
    for _ in range(rng.randint(0, 3)):
        pieces.append(TEXTS[rng.choice(SMALL_TEXTS)](rng))
    rng.shuffle(pieces)
    return " ".join(pieces)


#: Each render class's field text, drawn only when asked for.
TEXTS: dict[str, Callable[[random.Random], str]] = {
    "case": lambda rng: f"{word(rng).upper()} {word(rng).title()}",
    "ws": lambda rng: f"  {word(rng)}\t {word(rng)} \n",
    "missing": words,
    "special": words,
    "empty": lambda rng: "",
    "empty-front": lambda rng: "",
    "unbal": lambda rng: f"<b>{word(rng)} <i>{word(rng)}</b>",
    "chain": words,
    "unknown": words,
    "entities": lambda rng: f"{word(rng)} &amp; &lt;{word(rng)}&gt; &nbsp;&#x263A;&#9731;",
    "media": lambda rng: f'{word(rng)} <img src="ds_{word(rng)}.png"> [sound:ds_{word(rng)}.mp3]',
    "cloze-single": lambda rng: f"{word(rng)} {{{{c1::{word(rng)}}}}}",
    "cloze-hint": lambda rng: f"{{{{c1::{word(rng)}::{word(rng)} hint}}}} {word(rng)}",
    "cloze-multi": lambda rng: (
        f"{{{{c1::{word(rng)}}}}} {{{{c2::{word(rng)}}}}} {{{{c1::{word(rng)}}}}}"
    ),
    "cloze-nest-1": lambda rng: nest(1, rng),
    "cloze-nest-2": lambda rng: nest(2, rng),
    "cloze-nest-3": lambda rng: nest(3, rng),
    "cloze-nest-4": lambda rng: nest(4, rng),
    "cloze-math": lambda rng: f"\\({word(rng)}^{{2}}\\) {{{{c1::\\({word(rng)} + 1\\)}}}}",
    "cloze-span": lambda rng: f"<b>{{{{c1::{word(rng)}</b> {word(rng)}}}}}",
    "gap": lambda rng: f"{{{{c1::{word(rng)}}}}} {{{{c3::{word(rng)}}}}}",
    "unicode-nfc": lambda rng: nfc(f"café {word(rng)} naïve"),
    "unicode-nfd": lambda rng: nfd(f"café {word(rng)} naïve"),
    "unicode-rtl": lambda rng: f"{rng.choice(RTL)} {word(rng)}",
    "unicode-cjk-furigana": lambda rng: f"{rng.choice(CJK)}[よみ] {rng.choice(CJK)}",
    "unicode-emoji": lambda rng: " ".join(rng.choice(EMOJI) for _ in range(3)),
    "large": lambda rng: " ".join(word(rng) for _ in range(4000)),
}
SMALL_TEXTS = [name for name in TEXTS if name != "large"]


def render_template(edge: str | None, rng: random.Random) -> tuple[str, str]:
    """A (front, back) pair over the fields Prompt and Answer for the template class `edge`."""
    back = "{{FrontSide}}<hr class=ds-answer>{{Answer}}"
    templates = {
        "case": ("{{prompt}} {{PROMPT}}", back),
        "ws": ("{{ Prompt }}\n{{#Answer}} {{ Answer }} {{/Answer}}", back),
        "missing": ("{{Prompt}} {{Nowhere}}", back),
        "special": (
            "{{Prompt}} {{Tags}} {{Type}} {{Deck}} {{Subdeck}} {{Card}} {{CardFlag}}",
            "{{FrontSide}}<hr class=ds-answer>{{Answer}} {{Card}}",
        ),
        "empty-front": ("{{#Answer}}{{Answer}}{{/Answer}}", back),
        "unbal": ("{{#Prompt}}{{Prompt}}", back),
        "chain": ("{{text:hint:Prompt}} {{furigana:kana:Answer}}", back),
        "unknown": ("{{ds-no-such-filter:Prompt}}", back),
    }
    if edge in templates:
        return templates[edge]
    sections = [
        "{{Prompt}}",
        "{{#Answer}}<i>{{Answer}}</i>{{/Answer}}",
        "{{^Answer}}<b>{{Prompt}}</b>{{/Answer}}",
        "{{hint:Answer}}",
        "{{text:Prompt}}",
    ]
    front = " ".join(rng.sample(sections, rng.randint(1, 3)))
    return front, back


def timing(rng: random.Random, edge: str | None = None) -> dict:
    """Creation time, rollover hour, UTC offset and an instant, in seconds."""
    crt = BASE + rng.randrange(0, 400) * DAY
    hour = rng.randrange(0, 24)
    offset = rng.choice([-600, -300, -120, 0, 60, 330, 540, 720])
    now = crt + rng.randrange(1, 900) * DAY + rng.randrange(0, DAY)
    if edge == "rollover":
        # One second before the local rollover hour, so the study day is about to change.
        local_midnight = (now + offset * 60) // DAY * DAY - offset * 60
        now = local_midnight + hour * 3600 - 1
    elif edge == "negative-offset":
        offset = -rng.choice([60, 300, 420, 600])
    elif edge == "crt-after-now":
        now = crt - rng.randrange(1, 30) * DAY
    return {"crt": crt, "rollover_hour": hour, "utc_offset_minutes": offset, "now": now}


def deck(name: str, filtered: bool = False) -> dict:
    return {"name": name, "filtered": filtered}


def card(
    note: int, ord_: int, deck_: int, queue: int = 0, kind: int = 0, due: int = 1, **extra
) -> dict:
    row = {"note": note, "ord": ord_, "deck": deck_, "queue": queue, "type": kind, "due": due}
    row.update({"ivl": 0, "factor": 0, "reps": 0, "lapses": 0, "odeck": None, "odue": 0})
    row.update(extra)
    return row


def review(
    card_: int, at_ms: int, kind: int, ease: int = 3, ivl: int = 1, last_ivl: int = 0
) -> dict:
    return {
        "card": card_,
        "at_ms": at_ms,
        "type": kind,
        "ease": ease,
        "ivl": ivl,
        "last_ivl": last_ivl,
        "factor": 2500,
        "taken_ms": 4000,
    }


def collection(rng: random.Random, edges: tuple[str, ...] = ()) -> dict:
    """A synthetic collection holding every edge in `edges` and a few plain rows besides."""
    clock = timing(rng, "rollover" if "rollover" in edges else None)
    notetypes = [basic(), cloze_type()]
    decks = [deck("Ds Home"), deck(f"Ds Home::{word(rng).title()}")]
    notes = [
        {
            "notetype": 0,
            "fields": [words(rng), words(rng)],
            "tags": ["ds"],
            "guid": f"ds-{rng.randrange(16**8):08x}",
        }
        for _ in range(3)
    ]
    cards = [
        card(
            i,
            0,
            0,
            queue=2,
            kind=2,
            due=rng.randrange(0, 900),
            ivl=rng.randrange(1, 90),
            factor=2500,
        )
        for i in range(3)
    ]
    revlog = []
    at = clock["now"] * 1000 - 40 * DAY * 1000
    for index in range(3):
        at += rng.randrange(1, 5) * DAY * 1000
        revlog.append(review(index, at, 1))
    media: list[dict] = []
    if "unicase-names" in edges or "case-pairs" in edges:
        decks += [deck("Ds Alpha"), deck("DS ALPHA")]
        notetypes.append(
            notetype(
                "DS.Prompt-Answer", ["Prompt", "Answer"], [("Forward", "{{Prompt}}", "{{Answer}}")]
            )
        )
    if "cjk" in edges or "unicode-deck-names" in edges:
        decks.append(deck(f"Ds {rng.choice(CJK)}"))
        notes.append(
            {
                "notetype": 0,
                "fields": [rng.choice(CJK), rng.choice(CJK)],
                "tags": [],
                "guid": "ds-cjk0001",
            }
        )
        cards.append(card(len(notes) - 1, 0, len(decks) - 1))
    if "empty-fields" in edges or "empty" in edges:
        notes.append({"notetype": 0, "fields": [words(rng), ""], "tags": [], "guid": "ds-empty001"})
        cards.append(card(len(notes) - 1, 0, 0))
    if "filtered-deck" in edges or "odue-filtered" in edges:
        decks.append({"name": "Ds Filtered", "filtered": True, "search": 'deck:"Ds Home"'})
        cards.append(
            card(
                0,
                0,
                len(decks) - 1,
                queue=2,
                kind=2,
                due=-100_000,
                odeck=0,
                odue=rng.randrange(1, 900),
            )
        )
    for kind in range(6):
        if f"revlog-type-{kind}" in edges:
            at += rng.randrange(1, 3) * DAY * 1000
            revlog.append(review(0, at, kind, ease=0 if kind in (4, 5) else 3))
    if "ease-0" in edges:
        at += DAY * 1000
        revlog.append(review(1, at, 4, ease=0, ivl=0))
    if "suspended" in edges or "queue-m1" in edges:
        cards.append(card(1, 0, 0, queue=-1, kind=2, due=200, ivl=10))
    if "buried" in edges or "queue-m2" in edges or "queue-m3" in edges:
        cards.append(card(2, 0, 0, queue=-3, kind=2, due=210, ivl=12))
        cards.append(card(2, 0, 0, queue=-2, kind=0, due=4))
    if "intraday-learning" in edges or "queue-1" in edges:
        cards.append(card(0, 0, 0, queue=1, kind=1, due=clock["now"] + 600))
    if "day-learning" in edges or "queue-3" in edges:
        cards.append(card(1, 0, 0, queue=3, kind=1, due=(clock["now"] - clock["crt"]) // DAY))
    if "notetype-kinds" in edges:
        notetypes.append(occlusion_type())
        notes.append(
            {
                "notetype": 1,
                "fields": [render_text("cloze-multi", rng), ""],
                "tags": [],
                "guid": "ds-cloze01",
            }
        )
        notes.append(
            {
                "notetype": len(notetypes) - 1,
                "fields": [
                    "{{c1::image-occlusion:rect:left=.1:top=.1:width=.3:height=.2}}",
                    '<img src="ds_map.png">',
                    "",
                ],
                "tags": [],
                "guid": "ds-mask0001",
            }
        )
        cards.append(card(len(notes) - 2, 0, 0))
        cards.append(card(len(notes) - 1, 0, 0))
    if "html" in edges or "html-in-fields" in edges or "media-refs" in edges:
        notes.append(
            {
                "notetype": 0,
                "fields": [
                    f'<b>{word(rng)}</b> <img src="ds_{word(rng)}.png">',
                    "[sound:ds_tone.mp3]",
                ],
                "tags": [],
                "guid": "ds-html0001",
            }
        )
        cards.append(card(len(notes) - 1, 0, 0))
    if (
        "missing-files" in edges
        or "unused-files" in edges
        or "css-fonts" in edges
        or "latex-notes" in edges
        or "underscore-files" in edges
    ):
        media += [{"name": "ds_present.png", "size": 64}, {"name": "_ds_font.woff2", "size": 128}]
        if "unused-files" in edges:
            media.append({"name": "ds_unused.png", "size": 32})
        if "css-fonts" in edges:
            notetypes[0] = dict(
                notetypes[0], css="@font-face { font-family: dsface; src: url('_ds_font.woff2'); }"
            )
        if "latex-notes" in edges:
            notes.append(
                {
                    "notetype": 0,
                    "fields": ["[latex]x^{2}[/latex]", "[$]y[/$]"],
                    "tags": [],
                    "guid": "ds-latex01",
                }
            )
            cards.append(card(len(notes) - 1, 0, 0))
    if "nfd-names" in edges:
        media.append({"name": nfd("ds_café.png"), "size": 16})
    if "nfc-names" in edges:
        media.append({"name": nfc("ds_naïve.png"), "size": 16})
    return {
        **clock,
        "notetypes": notetypes,
        "decks": decks,
        "notes": notes,
        "cards": cards,
        "revlog": revlog,
        "media": media,
        "synced": "after-sync" in edges,
    }


def builder(hand: dict[str, Make], random_case: Make | None = None, count: int = RANDOM_CASES):
    """A case builder: each class's hand-written case or cases, then `count` random ones."""

    def cases(rng: random.Random) -> list[tuple[str | None, dict]]:
        drawn: list[tuple[str | None, dict]] = []
        for edge, make in hand.items():
            made = make(rng)
            drawn += [(edge, one) for one in (made if isinstance(made, list) else [made])]
        if random_case is not None:
            drawn += [(None, random_case(rng)) for _ in range(count)]
        return drawn

    return cases

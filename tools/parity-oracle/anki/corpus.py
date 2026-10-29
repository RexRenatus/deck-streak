"""The oracle's synthetic corpus: DeckStreak-authored note types, field texts and collections,
drawn from one seed or written here, with the edge classes the registry declares (SPEC-498,
ADR-516).

Nothing here is anyone's data and nothing is the engine's own text: every note type, template,
style sheet and field text is written for the corpus, and every name is invented. A case's input
is plain JSON. A collection names its rows by their index in the corpus, never by an id the
engine assigns, and a time is a whole number of seconds or milliseconds since the Unix epoch,
never a calendar string, so a golden holds no id or clock of the run.
"""

from __future__ import annotations

import copy
import random
import unicodedata
from collections.abc import Callable

#: The corpus's one seed: every drawn case comes from `random.Random(SEED)` only, and a written
#: case draws nothing.
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
    """An image occlusion note type: a cloze kind whose one template names its mask field in a
    cloze replacement on both sides, so each mask number is a card."""
    return notetype(
        "ds.masked-image",
        ["Masks", "Picture", "Caption"],
        [
            (
                "Mask",
                "<div class=ds-mask>{{cloze:Masks}}</div>{{Picture}}",
                "<div class=ds-mask>{{cloze:Masks}}</div>{{Picture}}<p>{{Caption}}</p>",
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


def at_rollover(clock: dict) -> dict:
    """The clock moved to one second before its local rollover hour, so the study day is about
    to change."""
    offset = clock["utc_offset_minutes"] * 60
    local_midnight = (clock["now"] + offset) // DAY * DAY - offset
    return dict(clock, now=local_midnight + clock["rollover_hour"] * 3600 - 1)


def timing(rng: random.Random, edge: str | None = None) -> dict:
    """Creation time, rollover hour, UTC offset and an instant, in seconds."""
    crt = BASE + rng.randrange(0, 400) * DAY
    hour = rng.randrange(0, 24)
    offset = rng.choice([-600, -300, -120, 0, 60, 330, 540, 720])
    now = crt + rng.randrange(1, 900) * DAY + rng.randrange(0, DAY)
    if edge == "rollover":
        return at_rollover(
            {"crt": crt, "rollover_hour": hour, "utc_offset_minutes": offset, "now": now}
        )
    if edge == "negative-offset":
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


def draw_sketch(rng: random.Random, edges: tuple[str, ...] = ()) -> dict:
    """The values a drawn collection holds for `edges`, drawn in the order its rows use them."""
    sketch = {
        "clock": timing(rng, "rollover" if "rollover" in edges else None),
        "subdeck": word(rng).title(),
        "notes": [[words(rng), words(rng), f"ds-{rng.randrange(16**8):08x}"] for _ in range(3)],
        "cards": [[rng.randrange(0, 900), rng.randrange(1, 90)] for _ in range(3)],
        "steps": [rng.randrange(1, 5) for _ in range(3)],
    }
    if "cjk" in edges or "unicode-deck-names" in edges:
        sketch["cjk"] = [rng.choice(CJK) for _ in range(3)]
    if "empty-fields" in edges or "empty" in edges:
        sketch["empty"] = words(rng)
    if "filtered-deck" in edges or "odue-filtered" in edges:
        sketch["odue"] = rng.randrange(1, 900)
    sketch["type_steps"] = [
        rng.randrange(1, 3) if f"revlog-type-{kind}" in edges else 0 for kind in range(6)
    ]
    if "notetype-kinds" in edges:
        sketch["cloze"] = render_text("cloze-multi", rng)
    if "html" in edges or "html-in-fields" in edges or "media-refs" in edges:
        sketch["html"] = [word(rng), word(rng)]
    return sketch


def collection(rng: random.Random, edges: tuple[str, ...] = ()) -> dict:
    """A synthetic collection holding every edge in `edges` and a few plain rows besides."""
    return assemble(draw_sketch(rng, edges), edges)


def written_collection(sketch: dict, edges: tuple[str, ...] = ()) -> dict:
    """The collection a written sketch gives for `edges`, at its rollover when `edges` holds the
    rollover. It draws nothing."""
    if "rollover" in edges:
        sketch = dict(sketch, clock=at_rollover(sketch["clock"]))
    return assemble(sketch, edges)


def assemble(sketch: dict, edges: tuple[str, ...] = ()) -> dict:
    """The collection a sketch's values give for `edges`. It draws nothing, so a drawn
    collection and a written one differ only in where their sketch's values came from."""
    clock = sketch["clock"]
    notetypes = [basic(), cloze_type()]
    decks = [deck("Ds Home"), deck(f"Ds Home::{sketch['subdeck']}")]
    notes = [
        {"notetype": 0, "fields": [first, second], "tags": ["ds"], "guid": guid}
        for first, second, guid in sketch["notes"]
    ]
    cards = [
        card(i, 0, 0, queue=2, kind=2, due=due, ivl=ivl, factor=2500)
        for i, (due, ivl) in enumerate(sketch["cards"])
    ]
    revlog = []
    at = clock["now"] * 1000 - 40 * DAY * 1000
    for index, step in enumerate(sketch["steps"]):
        at += step * DAY * 1000
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
        decks.append(deck(f"Ds {sketch['cjk'][0]}"))
        notes.append(
            {
                "notetype": 0,
                "fields": list(sketch["cjk"][1:3]),
                "tags": [],
                "guid": "ds-cjk0001",
            }
        )
        cards.append(card(len(notes) - 1, 0, len(decks) - 1))
    if "empty-fields" in edges or "empty" in edges:
        notes.append(
            {"notetype": 0, "fields": [sketch["empty"], ""], "tags": [], "guid": "ds-empty001"}
        )
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
                odue=sketch["odue"],
            )
        )
    for kind in range(6):
        if f"revlog-type-{kind}" in edges:
            at += sketch["type_steps"][kind] * DAY * 1000
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
                "fields": [sketch["cloze"], ""],
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
        bold, picture = sketch["html"]
        notes.append(
            {
                "notetype": 0,
                "fields": [
                    f'<b>{bold}</b> <img src="ds_{picture}.png">',
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


def builder(
    hand: dict[str, Make],
    random_case: Make | None = None,
    count: int = RANDOM_CASES,
    written: dict[str, list[dict]] | None = None,
):
    """A case builder: each class's hand-written case or cases and its written cases, which
    draw nothing, then `count` random ones."""

    def cases(rng: random.Random) -> list[tuple[str | None, dict]]:
        drawn: list[tuple[str | None, dict]] = []
        for edge, make in hand.items():
            made = make(rng)
            drawn += [(edge, one) for one in (made if isinstance(made, list) else [made])]
            drawn += [(edge, copy.deepcopy(one)) for one in (written or {}).get(edge, [])]
        if random_case is not None:
            drawn += [(None, random_case(rng)) for _ in range(count)]
        return drawn

    return cases


# Written cases: the hand cases the design's target asks for, each one written here and drawn
# from nothing. A registration builds its written cases from these texts, templates, masks,
# clocks and sketches alone, so they read the same under any seed. Index k of each list of
# seven is one case; incidental fields take the k-th of PLAIN.

PLAIN = [
    "amber basil",
    "cobalt",
    "dune ember fjord",
    "garnet harbor",
    "indigo",
    "juniper amber cobalt",
    "basil dune",
]
ORDS = [1, 1, 2, 1, 3, 1, 2]
CARD_ORDS = [0, 0, 1, 0, 2, 1, 0]
ANSWER_BACK = "{{FrontSide}}<hr class=ds-answer>{{Answer}}"

#: Seven clocks, each a creation time, rollover hour, UTC offset in minutes and an instant.
CLOCKS = [
    {"crt": BASE, "rollover_hour": 4, "utc_offset_minutes": 0, "now": BASE + 30 * DAY + 3_600},
    {
        "crt": BASE + 17 * DAY,
        "rollover_hour": 0,
        "utc_offset_minutes": 540,
        "now": BASE + 200 * DAY + 45_000,
    },
    {
        "crt": BASE + 100 * DAY,
        "rollover_hour": 23,
        "utc_offset_minutes": -300,
        "now": BASE + 101 * DAY + 7_200,
    },
    {
        "crt": BASE + 365 * DAY,
        "rollover_hour": 12,
        "utc_offset_minutes": 330,
        "now": BASE + 1_000 * DAY + 86_399,
    },
    {
        "crt": BASE + 3 * DAY,
        "rollover_hour": 6,
        "utc_offset_minutes": -210,
        "now": BASE + 40 * DAY + 1,
    },
    {
        "crt": BASE + 50 * DAY,
        "rollover_hour": 2,
        "utc_offset_minutes": 780,
        "now": BASE + 950 * DAY + 40_000,
    },
    {
        "crt": BASE + 250 * DAY,
        "rollover_hour": 18,
        "utc_offset_minutes": 345,
        "now": BASE + 251 * DAY + 64_800,
    },
]

#: Each timing class's seven clocks.
CLOCKS_WRITTEN = {
    "rollover": [at_rollover(clock) for clock in CLOCKS],
    "negative-offset": [
        dict(clock, utc_offset_minutes=offset)
        for clock, offset in zip(CLOCKS, [-60, -180, -210, -300, -420, -570, -720], strict=True)
    ],
    "crt-after-now": [
        dict(clock, now=clock["crt"] - back)
        for clock, back in zip(
            CLOCKS, [1, 3_600, DAY, DAY + 1, 7 * DAY, 30 * DAY, 400 * DAY], strict=True
        )
    ],
}

#: Seven written collection sketches: the values `assemble` reads, in place of a draw.
SKETCHES = [
    {
        "clock": CLOCKS[0],
        "subdeck": "Amber",
        "notes": [
            ["amber basil", "cobalt", "ds-0a1b2c3d"],
            ["dune", "ember fjord", "ds-1b2c3d4e"],
            ["garnet harbor indigo", "juniper", "ds-2c3d4e5f"],
        ],
        "cards": [[10, 5], [42, 21], [300, 60]],
        "steps": [1, 2, 3],
        "cjk": ["山水", "読書", "학교"],
        "empty": "amber",
        "odue": 120,
        "type_steps": [1, 2, 1, 2, 1, 2],
        "cloze": "{{c1::amber}} {{c2::basil}} {{c1::cobalt}}",
        "html": ["dune", "ember"],
    },
    {
        "clock": CLOCKS[1],
        "subdeck": "Basil",
        "notes": [
            ["<b>basil</b>", "<i>cobalt</i>", "ds-3d4e5f60"],
            ["dune ember", "", "ds-4e5f6071"],
            ["fjord", "garnet", "ds-5f607182"],
        ],
        "cards": [[0, 1], [1, 2], [2, 3]],
        "steps": [4, 4, 4],
        "cjk": ["電車", "花火", "山水"],
        "empty": "basil cobalt",
        "odue": 1,
        "type_steps": [2, 2, 2, 2, 2, 2],
        "cloze": "{{c2::dune}} {{c1::ember}}",
        "html": ["fjord", "garnet"],
    },
    {
        "clock": CLOCKS[2],
        "subdeck": "Cobalt",
        "notes": [
            ["harbor", "indigo juniper", "ds-60718293"],
            ["amber amber", "basil basil", "ds-718293a4"],
            ["cobalt dune ember fjord garnet harbor", "indigo", "ds-8293a4b5"],
        ],
        "cards": [[899, 89], [450, 45], [1, 1]],
        "steps": [1, 1, 1],
        "cjk": ["読書", "読書", "電車"],
        "empty": "dune",
        "odue": 899,
        "type_steps": [1, 1, 1, 1, 1, 1],
        "cloze": "{{c1::harbor}} {{c1::indigo}} {{c3::juniper}}",
        "html": ["amber", "basil"],
    },
    {
        "clock": CLOCKS[3],
        "subdeck": "Dune",
        "notes": [
            ["café", "naïve", "ds-93a4b5c6"],
            ["山水 amber", "basil 学校", "ds-a4b5c6d7"],
            ["ember", "fjord", "ds-b5c6d7e8"],
        ],
        "cards": [[365, 30], [730, 60], [5, 2]],
        "steps": [2, 3, 4],
        "cjk": ["花火", "학교", "読書"],
        "empty": "garnet harbor",
        "odue": 365,
        "type_steps": [2, 1, 2, 1, 2, 1],
        "cloze": "{{c1::cobalt}} {{c2::dune}} {{c3::ember}}",
        "html": ["cobalt", "dune"],
    },
    {
        "clock": CLOCKS[4],
        "subdeck": "Ember",
        "notes": [
            ["garnet", "harbor", "ds-c6d7e8f9"],
            ["indigo juniper amber", "basil", "ds-d7e8f90a"],
            ["cobalt", "dune ember", "ds-e8f90a1b"],
        ],
        "cards": [[7, 3], [14, 7], [28, 14]],
        "steps": [3, 1, 2],
        "cjk": ["学校", "山水", "花火"],
        "empty": "indigo",
        "odue": 7,
        "type_steps": [1, 1, 2, 2, 1, 1],
        "cloze": "{{c2::fjord}} {{c3::garnet}} {{c2::harbor}}",
        "html": ["ember", "fjord"],
    },
    {
        "clock": CLOCKS[5],
        "subdeck": "Fjord",
        "notes": [
            ["fjord garnet", "harbor indigo", "ds-f90a1b2c"],
            ["juniper", "amber basil cobalt", "ds-0a1b2c3e"],
            ["dune", "dune", "ds-1b2c3d4f"],
        ],
        "cards": [[600, 80], [601, 81], [602, 82]],
        "steps": [4, 3, 2],
        "cjk": ["電車", "読書", "花火"],
        "empty": "juniper amber",
        "odue": 600,
        "type_steps": [2, 1, 1, 2, 2, 1],
        "cloze": "{{c1::indigo}} {{c2::juniper}} {{c4::amber}}",
        "html": ["garnet", "harbor"],
    },
    {
        "clock": CLOCKS[6],
        "subdeck": "Garnet",
        "notes": [
            ["ember fjord garnet", "harbor", "ds-2c3d4e60"],
            ["indigo", "juniper amber", "ds-3d4e5f71"],
            ["basil cobalt", "dune", "ds-4e5f6082"],
        ],
        "cards": [[33, 11], [66, 22], [99, 33]],
        "steps": [2, 2, 1],
        "cjk": ["山水", "電車", "학교"],
        "empty": "basil",
        "odue": 250,
        "type_steps": [1, 2, 2, 1, 1, 2],
        "cloze": "{{c1::basil}} {{c2::cobalt}} {{c2::dune}}",
        "html": ["indigo", "juniper"],
    },
]

#: Each text class's written field texts: seven each, five for the large class.
TEXTS_WRITTEN: dict[str, list[str]] = {
    "empty": [" ", "\t", "\n", "<br>", "&nbsp;", "<div></div>", " <br> "],
    "entities": [
        "amber &amp; basil",
        "&lt;b&gt;cobalt&lt;/b&gt;",
        "dune&nbsp;&nbsp;ember",
        "&#x263A; fjord &#9731;",
        "&quot;garnet&quot; &apos;harbor&apos;",
        "indigo &copy; &eacute;t&eacute;",
        "&#128578; juniper &notanentity;",
    ],
    "media": [
        '<img src="ds_amber.png">',
        "[sound:ds_basil.mp3]",
        'cobalt <img src="ds_dune.jpg"> [sound:ds_ember.ogg]',
        "[sound:ds_fjord.mp3][sound:ds_garnet.mp3]",
        '<img src="ds_harbor.svg" alt="ds harbor"> indigo',
        '[sound:ds juniper two.wav] <img src="ds_juniper.gif">',
        '<audio src="ds_amber.mp3"></audio> [sound:ds_basil.m4a]',
    ],
    "cloze-single": [
        "{{c1::amber}}",
        "cobalt {{c1::dune}} ember",
        "{{c1::fjord garnet}} harbor",
        "indigo {{c1::<b>juniper</b>}}",
        "  {{c1:: ember }}  ",
        "{{c1::basil}}.",
        "<div>{{c1::cobalt}}</div>",
    ],
    "cloze-hint": [
        "{{c1::amber::color}}",
        "{{c1::basil::herb}} cobalt",
        "dune {{c1::ember::glow hint}}",
        "{{c1::fjord::}} garnet",
        "{{c1::harbor::a <i>port</i>}}",
        "{{c1::indigo::hint::extra}}",
        "{{c2::juniper::tree}}",
    ],
    "cloze-multi": [
        "{{c1::amber}} {{c2::basil}}",
        "{{c1::cobalt}} {{c1::dune}}",
        "{{c1::ember}} {{c2::fjord}} {{c3::garnet}}",
        "{{c2::harbor}} {{c1::indigo}}",
        "{{c1::juniper}} and {{c2::amber::hint}} and {{c3::basil}}",
        "{{c1::cobalt}}{{c2::dune}}",
        "{{c10::ember}} {{c2::fjord}}",
    ],
    "cloze-nest-1": [
        "{{c1::amber basil}}",
        "{{c1::cobalt <i>dune</i>}}",
        "ember {{c1::fjord garnet}} harbor",
        "{{c2::indigo juniper}}",
        "{{c1::amber cobalt ember}}",
        "basil {{c1::dune}} fjord",
        "{{c1::fjord, garnet; harbor}}",
    ],
    "cloze-nest-2": [
        "{{c1::amber {{c2::basil}}}}",
        "{{c1::cobalt {{c2::dune}} ember}}",
        "{{c2::fjord {{c1::garnet}}}}",
        "{{c1::harbor {{c1::indigo}}}}",
        "{{c1::{{c2::juniper}} amber}}",
        "basil {{c1::cobalt {{c2::dune::hint}}}}",
        "{{c1::ember {{c2::fjord}} {{c3::garnet}}}}",
    ],
    "cloze-nest-3": [
        "{{c1::amber {{c2::basil {{c3::cobalt}}}}}}",
        "{{c3::dune {{c2::ember {{c1::fjord}}}}}}",
        "{{c1::garnet {{c2::harbor}} {{c2::indigo {{c3::juniper}}}}}}",
        "{{c1::{{c2::{{c3::amber}}}}}}",
        "{{c1::basil {{c1::cobalt {{c1::dune}}}}}}",
        "ember {{c1::fjord {{c2::garnet {{c3::harbor::hint}}}}}} indigo",
        "{{c2::juniper {{c3::amber {{c4::basil}}}}}}",
    ],
    "cloze-nest-4": [
        "{{c1::amber {{c2::basil {{c3::cobalt {{c4::dune}}}}}}}}",
        "{{c4::ember {{c3::fjord {{c2::garnet {{c1::harbor}}}}}}}}",
        "{{c1::{{c2::{{c3::{{c4::indigo}}}}}}}}",
        "{{c1::juniper {{c2::amber}} {{c2::basil {{c3::cobalt {{c4::dune}}}}}}}}",
        "{{c1::ember {{c1::fjord {{c1::garnet {{c1::harbor}}}}}}}}",
        "indigo {{c2::juniper {{c3::amber {{c4::basil {{c5::cobalt}}}}}}}} dune",
        "{{c1::<b>ember</b> {{c2::<i>fjord</i> {{c3::garnet {{c4::harbor::deep hint}}}}}}}}",
    ],
    "cloze-math": [
        r"{{c1::\(a^2 + b^2\)}}",
        r"\(\frac{1}{2}\) {{c1::half}}",
        r"\[{{c1::e^{i\pi}}} + 1 = 0\]",
        r"{{c1::\(x\)}} and {{c2::\(y\)}}",
        r"\(\sqrt{ {{c1::amber}} }\)",
        r"{{c1::\[\sum_{k=1}^{n} k\]::sum}}",
        r"\(a\) {{c1::basil}} \(b\)",
    ],
    "cloze-span": [
        "<b>{{c1::amber</b> basil}}",
        "{{c1::cobalt <i>dune}}</i>",
        "<div>{{c1::ember</div><div>fjord}}</div>",
        "<span>garnet {{c1::harbor</span> indigo}}",
        "{{c1::<b>juniper}}</b>",
        "<u>{{c1::amber <b>basil</u> cobalt}}</b>",
        "<i>{{c2::dune</i>}}",
    ],
    "gap": [
        "{{c1::amber}} {{c3::basil}}",
        "{{c2::cobalt}}",
        "{{c1::dune}} {{c5::ember}}",
        "{{c3::fjord}} {{c1::garnet}}",
        "{{c2::harbor}} {{c4::indigo}}",
        "{{c1::juniper}} {{c2::amber}} {{c4::basil}}",
        "{{c10::cobalt}}",
    ],
    "unicode-nfc": [
        nfc(text)
        for text in [
            "café amber",
            "naïve basil",
            "Ångström cobalt",
            "résumé dune",
            "über ember",
            "crème brûlée fjord",
            "piñata garnet",
        ]
    ],
    "unicode-nfd": [
        nfd(text)
        for text in [
            "café amber",
            "naïve basil",
            "Ångström cobalt",
            "résumé dune",
            "über ember",
            "crème brûlée fjord",
            "piñata garnet",
        ]
    ],
    "unicode-rtl": [
        "שלום amber",
        "مرحبا basil",
        "amber שלום עולם basil",
        "‫cobalt dune‬",
        "עברית 123 عربى",
        "<span dir=rtl>مرحبا بالعالم</span>",
        "‏ember‏ fjord",
    ],
    "unicode-cjk-furigana": [
        "山水[さんすい]",
        "読書[どくしょ] amber",
        "電車[でんしゃ]に 乗[の]る",
        " 花火[はなび]",
        "학교 basil",
        "漢字[かんじ] 仮名[かな]",
        "東京[とうきょう]の 空[そら]",
    ],
    "unicode-emoji": [
        "\U0001f642",
        "\U0001f469‍\U0001f52c amber",
        "\U0001f1f3\U0001f1f4\U0001f1ef\U0001f1f5",
        "❤️ basil ❤",
        "\U0001f44d\U0001f3fd",
        "\U0001f9d1‍\U0001f91d‍\U0001f9d1 cobalt",
        "1️⃣ 2️⃣",
    ],
    "large": [
        "amber basil cobalt dune " * 900,
        "<b>ember</b> fjord " * 1_100,
        "garnet [sound:ds_harbor.mp3] " * 750,
        "{{c1::indigo}} juniper " * 950,
        "山水 読書 " * 4_000,
    ],
    "ws": [
        "  amber  ",
        "\tbasil\t",
        "cobalt\n\ndune",
        " ember fjord ",
        "garnet \r\n harbor",
        " indigo ",
        "juniper   amber",
    ],
    "unbal": [
        "<b>amber <i>basil</b>",
        "<div>cobalt",
        "dune</span>",
        "<b><b>ember</b>",
        "fjord <i",
        "<ul><li>garnet</ul>",
        "harbor</div></div>",
    ],
    "case": [
        "AMBER Basil",
        '<IMG SRC="ds_Cobalt.PNG">',
        "[SOUND:ds_dune.mp3]",
        "<Img Src='ds_ember.png'> FJORD",
        "GaRnEt hArBoR",
        "[sound:DS_HARBOR.MP3]",
        "Indigo <IMG src=ds_juniper.png>",
    ],
    "missing-cloze": [
        "amber basil",
        "{{c1:amber}}",
        "{c1::basil}",
        "cobalt {{c::dune}}",
        "{{x1::ember}}",
        "fjord }}",
        "{{ c1::garnet }}",
    ],
}

#: Each template class's written (front, back) pairs over the fields Prompt and Answer.
TEMPLATES_WRITTEN: dict[str, list[tuple[str, str]]] = {
    "case": [
        ("{{prompt}}", ANSWER_BACK),
        ("{{PROMPT}} {{answer}}", ANSWER_BACK),
        ("{{Prompt}} {{pRoMpT}}", ANSWER_BACK),
        ("{{#prompt}}{{Prompt}}{{/prompt}}", ANSWER_BACK),
        ("{{text:PROMPT}}", ANSWER_BACK),
        ("{{Prompt}}", "{{FrontSide}}<hr class=ds-answer>{{ANSWER}}"),
        ("{{hint:answer}}", ANSWER_BACK),
    ],
    "ws": [
        ("{{ Prompt }}", ANSWER_BACK),
        ("{{  Prompt}}", ANSWER_BACK),
        ("{{Prompt  }}", ANSWER_BACK),
        ("{{\tPrompt\t}}", ANSWER_BACK),
        ("{{# Answer }}{{Answer}}{{/ Answer }}", ANSWER_BACK),
        ("{{ text: Prompt }}", ANSWER_BACK),
        ("{{Prompt}}\n\n{{Answer}}", ANSWER_BACK),
    ],
    "missing": [
        ("{{Elsewhere}}", ANSWER_BACK),
        ("{{Prompt}} {{#Absent}}amber{{/Absent}}", ANSWER_BACK),
        ("{{^Ghost}}{{Prompt}}{{/Ghost}}", ANSWER_BACK),
        ("{{text:Vacant}} {{Prompt}}", ANSWER_BACK),
        ("{{Prompt}}", "{{FrontSide}}<hr class=ds-answer>{{Hidden}}"),
        ("{{Prompt}} {{Prompt2}}", ANSWER_BACK),
        ("{{Prompt}} {{Answers}}", ANSWER_BACK),
    ],
    "special": [
        ("{{Tags}} {{Prompt}}", ANSWER_BACK),
        ("{{Deck}}: {{Prompt}}", ANSWER_BACK),
        ("{{Subdeck}} {{Prompt}}", ANSWER_BACK),
        ("{{Card}} {{Prompt}}", ANSWER_BACK),
        ("{{Type}} {{Prompt}}", ANSWER_BACK),
        ("{{CardFlag}} {{Prompt}}", ANSWER_BACK),
        ("{{Prompt}}", "{{FrontSide}} {{Tags}}<hr class=ds-answer>{{Answer}}"),
    ],
    "empty-front": [
        ("{{^Prompt}}{{Answer}}{{/Prompt}}", ANSWER_BACK),
        ("", ANSWER_BACK),
        ("{{Answer}}", ANSWER_BACK),
        ("<div class=ds-front>{{Answer}}</div>", ANSWER_BACK),
        ("{{#Answer}}<b>{{Prompt}}</b>{{/Answer}}", ANSWER_BACK),
        ("{{text:Answer}}", ANSWER_BACK),
        ("{{hint:Answer}}", ANSWER_BACK),
    ],
    "unbal": [
        ("{{/Prompt}}", ANSWER_BACK),
        ("{{#Prompt}}{{#Answer}}{{Prompt}}{{/Prompt}}", ANSWER_BACK),
        ("{{Prompt", ANSWER_BACK),
        ("{{^Answer}}{{Prompt}}", ANSWER_BACK),
        ("{{#Prompt}}{{Prompt}}{{/Answer}}", ANSWER_BACK),
        ("{{Prompt}}", "{{FrontSide}}{{#Answer}}<hr class=ds-answer>"),
        ("<b>{{Prompt}}", ANSWER_BACK),
    ],
    "chain": [
        ("{{hint:text:Prompt}}", ANSWER_BACK),
        ("{{kana:furigana:Prompt}}", ANSWER_BACK),
        ("{{text:text:Prompt}}", ANSWER_BACK),
        ("{{kanji:text:Prompt}}", ANSWER_BACK),
        ("{{furigana:text:hint:Prompt}}", ANSWER_BACK),
        ("{{Prompt}}", "{{FrontSide}}<hr class=ds-answer>{{text:kana:Answer}}"),
        ("{{type:text:Prompt}}", ANSWER_BACK),
    ],
    "unknown": [
        ("{{ds-shout:Prompt}}", ANSWER_BACK),
        ("{{ds-a:ds-b:Prompt}}", ANSWER_BACK),
        ("{{text:ds-whisper:Prompt}}", ANSWER_BACK),
        ("{{ds-echo:text:Prompt}}", ANSWER_BACK),
        ("{{Prompt}}", "{{FrontSide}}<hr class=ds-answer>{{ds-mirror:Answer}}"),
        ("{{ds_under:Prompt}}", ANSWER_BACK),
        ("{{DS-CAPS:Prompt}}", ANSWER_BACK),
    ],
}

#: The cloze note type's written (front, back) pairs, over the fields Body and Notes: for one
#: cloze field, and for cloze references spread over the fields or absent.
CLOZE_TEMPLATES_WRITTEN: dict[str, list[tuple[str, str]]] = {
    "cloze-single": [
        ("{{cloze:Body}}", "{{cloze:Body}}<br>{{Notes}}"),
        ("<p>{{cloze:Body}}</p>", "<p>{{cloze:Body}}</p><p>{{Notes}}</p>"),
        ("{{cloze:Body}} {{Notes}}", "{{cloze:Body}}"),
        ("{{cloze:Body}}", "{{cloze:Body}}"),
        ("{{#Notes}}{{Notes}}{{/Notes}}{{cloze:Body}}", "{{cloze:Body}}"),
        ("{{cloze: Body }}", "{{cloze: Body }}"),
        ("{{text:cloze:Body}}", "{{cloze:Body}} {{text:Notes}}"),
    ],
    "gap": [
        ("{{cloze:Body}}{{cloze:Notes}}", "{{cloze:Body}}{{cloze:Notes}}"),
        ("{{cloze:Notes}}", "{{cloze:Notes}}{{Body}}"),
        ("{{Body}}", "{{Body}}"),
        ("", "{{cloze:Body}}"),
        ("{{cloze:Body}}", ""),
        ("{{cloze:Gone}}", "{{cloze:Gone}}"),
        ("{{cloze:Body}} {{cloze:Body}}", "{{cloze:Body}}"),
    ],
}

#: Each occlusion shape class's seven written shapes.
SHAPES_WRITTEN = {
    "occlusion-rect": [
        "rect:left=.1:top=.1:width=.2:height=.2",
        "rect:left=0:top=0:width=1:height=1",
        "rect:left=.5:top=.5:width=.01:height=.01",
        "rect:left=.25:top=.75:width=.5:height=.1:fill=#ffeba2",
        "rect:left=.9:top=.9:width=.2:height=.2",
        "rect:left=.3:top=.3:width=.4:height=.4:angle=30",
        "rect:left=.05:top=.6:width=.9:height=.05",
    ],
    "occlusion-ellipse": [
        "ellipse:left=.2:top=.2:rx=.1:ry=.1",
        "ellipse:left=.5:top=.5:rx=.5:ry=.5",
        "ellipse:left=.1:top=.8:rx=.05:ry=.2",
        "ellipse:left=.7:top=.1:rx=.2:ry=.02",
        "ellipse:left=.4:top=.4:rx=0:ry=0",
        "ellipse:left=.3:top=.6:rx=.1:ry=.05:angle=45",
        "ellipse:left=.95:top=.95:rx=.1:ry=.1",
    ],
    "occlusion-polygon": [
        "polygon:points=0,0 .5,0 .25,.5",
        "polygon:points=.1,.1 .9,.1 .9,.9 .1,.9",
        "polygon:points=.2,.2 .4,.2",
        "polygon:points=.5,.1 .6,.4 .9,.4 .65,.6 .75,.9 .5,.7 .25,.9 .35,.6 .1,.4 .4,.4",
        "polygon:points=.3,.3 .3,.3 .3,.3",
        "polygon:points=.1,.5 .5,.1 .9,.5 .5,.9",
        "polygon:points=0,1 1,1 .5,0",
    ],
    "occlusion-text": [
        "text:left=.1:top=.1:text=amber",
        "text:left=.4:top=.2:text=basil cobalt:scale=2",
        "text:left=.5:top=.5:text=山水",
        "text:left=.2:top=.7:text=dune:fs=.08",
        "text:left=0:top=0:text=",
        "text:left=.6:top=.3:text=ember &amp; fjord",
        "text:left=.3:top=.9:text=\U0001f642",
    ],
}
PICTURES = [
    '<img src="ds_map.png">',
    '<img src="ds_chart.svg">',
    '<img src="ds_photo.jpg">',
    '<img src="ds_wide.png" width="800">',
    '<img src="ds_tall.webp">',
    '<img src="ds_icon.gif">',
    '<img src="ds_scan.png">',
]


def occlusion_mask(ordinal: int, shape: str) -> str:
    return f"{{{{c{ordinal}::image-occlusion:{shape}}}}}"


def shape_set(k: int) -> str:
    """The k-th written set of two to four shapes, one ordinal each."""
    shapes = [SHAPES_WRITTEN[name][k] for name in SHAPES_WRITTEN][: 2 + k % 3]
    return " ".join(occlusion_mask(i + 1, shape) for i, shape in enumerate(shapes))


#: Each occlusion class's seven written mask fields.
MASKS_WRITTEN = {
    **{
        name: [occlusion_mask(1, shape) for shape in shapes]
        for name, shapes in SHAPES_WRITTEN.items()
    },
    "occlusion-hide-one": [shape_set(k) for k in range(7)],
    "occlusion-hide-all": [shape_set(k) for k in range(7)],
    "gap": [
        occlusion_mask(1, SHAPES_WRITTEN["occlusion-rect"][k])
        + " "
        + occlusion_mask(3 + k % 2, SHAPES_WRITTEN["occlusion-ellipse"][k])
        for k in range(7)
    ],
    "empty": [
        "",
        " ",
        "<br>",
        "{{c1::image-occlusion:}}",
        "{{c1::}}",
        "image-occlusion:rect",
        "&nbsp;",
    ],
}

#: Each media reference class's seven written field HTML texts.
REFS_WRITTEN = {
    "quotes": [
        '<img src="ds_amber.png">',
        "<img src='ds_basil.png'>",
        "<img src=ds_cobalt.png>",
        "<img src=\"ds_dune.png\" alt='ds'>",
        '<img src = "ds_ember.png">',
        '<img alt="x" src="ds_fjord.png">',
        "<img src='ds_garnet.png'><img src=\"ds_harbor.png\">",
    ],
    "case": [
        '<IMG SRC="ds_amber.png">',
        '<Img src="ds_Basil.PNG">',
        '<img SRC="DS_COBALT.png">',
        "[SOUND:ds_dune.mp3]",
        "[Sound:ds_ember.mp3]",
        "<IMG src=ds_fjord.png>",
        '<iMg sRc="ds_garnet.png">',
    ],
    "percent-escape": [
        '<img src="ds%20amber.png">',
        '<img src="ds_basil%2Bcobalt.png">',
        '<img src="ds_%E2%9C%93.png">',
        '<img src="ds_100%25.png">',
        '<img src="ds%2Fdune.png">',
        '<img src="ds_ember%zz.png">',
        "[sound:ds%20fjord.mp3]",
    ],
    "url-schemes": [
        '<img src="http://example.invalid/ds_a.png">',
        '<img src="https://example.invalid/ds_b.png?x=1">',
        '<img src="data:image/gif;base64,R0lGOD">',
        '<img src="file://ds_c.png">',
        '<img src="//example.invalid/ds_d.png">',
        '<img src="ftp://example.invalid/ds_e.png">',
        '<img src="blob:ds_f">',
    ],
    "sound-tag": [
        "[sound:ds_amber.mp3]",
        "[sound:ds basil.ogg]",
        "[sound:ds_cobalt.mp3][sound:ds_dune.mp3]",
        "[sound:]",
        "[sound:ds_ember.mp3",
        "[sound:ds_[fjord].mp3]",
        "[sound:ds_garnet.MP3] harbor",
    ],
    "nested-markup": [
        '<div><img src="ds_amber.png"></div>',
        '<span><b><img src="ds_basil.png"></b></span>',
        '<table><tr><td><img src="ds_cobalt.png"></td></tr></table>',
        "<ul><li>[sound:ds_dune.mp3]</li></ul>",
        '<a href="#"><img src="ds_ember.png"></a>',
        '<div><div><div><img src="ds_fjord.png"></div></div></div>',
        '<p><i>[sound:ds_garnet.mp3]</i> <img src="ds_harbor.png"></p>',
    ],
    "broken-markup": [
        '<img src="ds_amber.png"',
        '<img src="ds_basil.png>',
        "<img src='ds_cobalt.png\">",
        "<img src=>",
        '<<img src="ds_dune.png">>',
        '<img src="ds_ember.png" <img src="ds_fjord.png">',
        '</img src="ds_garnet.png">',
    ],
    "underscore-names": [
        '<img src="_ds_amber.png">',
        "[sound:_ds_basil.mp3]",
        '<img src="__ds_cobalt.png">',
        '<img src="_.png">',
        '<img src="ds__dune.png">',
        '[sound:_ds_ember.ogg] <img src="_ds_fjord.png">',
        '<img src="_DS_GARNET.PNG">',
    ],
    "latex": [
        "[latex]amber^2[/latex]",
        "[$]basil[/$]",
        "[$$]cobalt[/$$]",
        r"[latex]\frac{dune}{2}[/latex]",
        "[latex][/latex]",
        "[$]ember[/$] [$$]fjord[/$$]",
        "[latex]garnet\n[/latex]",
    ],
}

#: The note checksum's written first fields for its HTML and CJK classes.
HTML_WRITTEN = [
    "<b>amber</b>",
    "<div>basil<br>cobalt</div>",
    "<i>dune</i> <u>ember</u>",
    '<span style="color: red">fjord</span>',
    "<p>garnet</p><p>harbor</p>",
    '<a href="#ds">indigo</a>',
    "<table><tr><td>juniper</td></tr></table>",
]
CJK_WRITTEN = [
    "山水画",
    "読書 する",
    "학교 가다",
    "電車[でんしゃ]",
    "花火大会",
    "中文 汉字",
    "ひらがな カタカナ",
]
#: First fields a duplicate note's existing note already holds.
DUPLICATE_WRITTEN = [
    "amber",
    "<b>basil</b>",
    "cobalt dune",
    "山水",
    "ember &amp; fjord",
    "garnet harbor",
    "[sound:ds_indigo.mp3]",
]

#: Media file names and name lists by class.
NAMES_WRITTEN = {
    "nfd": [
        [nfd("ds_résumé.png")],
        [nfd("ds_Ångström.mp3")],
        [nfd("ds_naïve.jpg"), nfd("ds_crème.gif")],
        [nfd("ds_piñata.webp")],
        [nfd("ds_über.ogg"), "ds_plain.png"],
        [nfd("ds_é.png")],
        [nfd("ds_άλφα.png")],
    ],
    "forbidden-chars": [
        ["ds*star.png"],
        ["ds?ask.png"],
        ["ds<angle>.png"],
        ["ds|pipe.png"],
        ['ds"quote".png'],
        ["ds/slash/deep.png"],
        ["ds\tTab.png", "ds\nline.png"],
    ],
    "long-names": [
        ["ds_" + "a" * 250 + ".png"],
        ["ds_" + "b" * 255 + ".mp3"],
        ["ds_" + "ü" * 130 + ".png"],
        ["ds_" + "山" * 90 + ".png"],
        ["ds_" + "c" * 120 + "." + "d" * 200],
        ["ds_" + "e" * 256],
        ["ds_" + "f" * 400 + ".jpeg"],
    ],
}

#: Names whose order the collation decides, by class.
COLLATION_WRITTEN = {
    "case-pairs": [
        ["amber", "Amber", "AMBER"],
        ["Basil", "basil", "BASIL", "basil2"],
        ["cobalt", "Cobalt"],
        ["Dune Ember", "dune ember", "DUNE EMBER"],
        ["fjord", "FJORD", "Fjord", "fjords"],
        ["Garnet::Harbor", "garnet::harbor"],
        ["iNDIGO", "Indigo", "indigo"],
    ],
    "non-ascii-folding": [
        ["Ärger", "arger", "ärger"],
        ["Øre", "ore", "øre"],
        ["Ñandú", "nandu", "ñandú"],
        ["Ωmega", "ωmega", "omega"],
        ["Łódź", "lodz", "łódź"],
        ["Æble", "æble", "aeble"],
        ["ǅemal", "Džemal", "DŽEMAL"],
    ],
}

#: Deck name pairs by class: letter case, and composed against decomposed form.
DECK_PAIRS_WRITTEN = {
    "case-forms": [
        ["Ds Amber", "ds amber"],
        ["Basil Deck", "BASIL DECK"],
        ["Ds::Cobalt", "DS::COBALT"],
        ["Dune Ember", "dUNE eMBER"],
        ["Ärger Fjord", "ÄRGER FJORD"],
        ["Ds Ωmega", "DS ΩMEGA"],
        ["garnet", "Garnet"],
    ],
    "unicode-forms": [
        [nfc(name), nfd(name)]
        for name in [
            "Ds Crème",
            "Ds Ångström",
            "Ds Résumé::Naïve",
            "Ds Über",
            "Ds Piñata",
            "Ds Ḱite",
            "Ds άλφα",
        ]
    ],
}

#: The card scheduling columns the due view reads, for its queue classes.
DUES = [0, 1, 7, 30, 365, 899, 4_000]
IVLS = [0, 1, 3, 10, 45, 120, 365]
LEARNING_STEPS = [60, 600, 3_600, -60, 0, DAY, 30]
#: Seven written note modification times and gaps, for the import classes.
MODS = [
    1_000_000,
    1_100_000_000,
    1_234_567_890,
    1_000_000_001,
    999_999,
    1_500_000_000,
    1_300_000_000,
]
GAPS = [100, 1, DAY, 3_600, 60, 2, 365 * DAY]
NEW_GUIDS = [
    "ds-new00001",
    "ds-new00002",
    "ds-fresh-a",
    "ds-fresh-b",
    "ds-zzzzzzzz",
    "ds-0",
    "ds-éte",
]
EASES = [1, 2, 3, 4, 3, 2, 1]
#: Two-template notes' written fields.
TWO_WAY_FIELDS = [
    ["amber", "basil"],
    ["cobalt", ""],
    ["", "dune"],
    ["ember fjord", "garnet"],
    ["<b>harbor</b>", "indigo"],
    ["juniper", "juniper"],
    [" ", "amber"],
]
#: Search suffixes the written search cases append to the class's search.
SEARCH_SUFFIXES = [
    "",
    " amber",
    " -basil",
    " (cobalt OR dune)",
    ' "ember fjord"',
    " tag:ds",
    " card:1",
]

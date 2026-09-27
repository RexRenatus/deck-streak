"""SPEC-042's registrations: the reading note's roll and its archive folder.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `_roll_note_text` drives `reading_notes.py:_roll_note_text`. A golden may hold no calendar date,
  so a day inside a note's text is written as the token `{day:N}`, N its epoch day number: the
  adapter expands every token of the case's text into the ISO date of that day, passes `today` (an
  epoch day number) as that day's `date`, calls the function, and writes every ISO date of the text
  it returns back as its token. A note the function refuses is recorded as
  `{"refused": <the exception's class>}`; any other result is `{"text": <the rolled text>}`.
* `_archive_dir` drives `reading_notes.py:_archive_dir` under a neutral relative root, `vault`, with
  the case's epoch day as a `date`. The adapter returns the folders of the returned path below that
  root, and its last folder, which the function names by the day's ISO date, as the date it names
  (the generator writes a `date` as its epoch day number).

The case builders draw only from the `random.Random` the generator seeds. For the roll they cover
the classes a port gets wrong: CRLF and mixed line endings, a lone carriage return, a missing
`first_generated`, `rolls` or `last_rolled`, the owner's ticks, nested and repeated keys, a `---`
rule in the body, no final newline, the line boundaries Python's `splitlines` reads beyond `\\n`
and `\\r`, the spellings Python's `int` reads, and the notes the function refuses. For the archive
they cover the last and first days of many calendar years (where the ISO week belongs to the other
year), leap days, the century years, days before the Unix epoch and ordinary days. Every topic,
digest and body here is synthetic.
"""

import re
from datetime import date
from pathlib import PurePosixPath

#: The ordinal of the Unix epoch's day, so a `date` and its epoch day number convert exactly.
EPOCH_ORDINAL = date(1970, 1, 1).toordinal()
#: A day inside a golden's note text: the token the adapter expands and writes back.
DAY_TOKEN = re.compile(r"\{day:(-?[0-9]+)\}")
#: An ISO calendar date inside the text the predecessor returns.
ISO_DAY = re.compile(r"(?<![0-9])[0-9]{4}-[0-9]{2}-[0-9]{2}(?![0-9])")

TOPICS = (
    "law/evidence",
    "law/torts",
    "law/civil-procedure",
    "language/ja",
    "language/fr",
    "lsat",
)
WORDS = (
    "hearsay",
    "exception",
    "declarant",
    "duty",
    "breach",
    "causation",
    "remedy",
    "particle",
    "kanji",
    "primer",
    "rule",
    "element",
    "issue",
    "analysis",
    "pleading",
    "motion",
)
UNTICKED = {"studied": "- [ ] Studied", "read": "- [ ] I read it"}
TICKED = {"studied": "- [x] Studied", "read": "- [x] I read it"}


def as_date(epoch_day):
    return date.fromordinal(EPOCH_ORDINAL + epoch_day)


def expand(text):
    """The case's text with every `{day:N}` token written as the ISO date of epoch day N."""
    return DAY_TOKEN.sub(lambda match: as_date(int(match[1])).isoformat(), text)


def contract(text):
    """`text` with every ISO date written as its `{day:N}` token; a date that does not read back
    as itself is refused, so the token always names exactly the date the predecessor wrote."""

    def token(match):
        day = date.fromisoformat(match[0])
        if day.isoformat() != match[0]:
            raise ValueError(f"{match[0]!r} is not an ISO date")
        return "{day:%d}" % (day.toordinal() - EPOCH_ORDINAL)

    return ISO_DAY.sub(token, text)


def with_days_as_numbers(roll_note_text, predecessor, *, text, today):
    """Expand the day tokens, roll the note to `today`, and write the returned days as tokens."""
    source = expand(text)
    if contract(source) != text:
        raise ValueError("the case's text holds a date that is not a day token")
    refusals = (predecessor("reading_notes.MalformedReadingNoteError"), ValueError)
    try:
        rolled = roll_note_text(source, today=as_date(today))
    except refusals as refusal:
        return {"refused": type(refusal).__name__}
    return {"text": contract(rolled)}


def under_a_neutral_root(archive_dir, predecessor, *, day):
    """The archive folder of `day` below the root `vault`: its folders, and the day it names."""
    root = PurePosixPath("vault")
    parts = archive_dir(root, as_date(day)).relative_to(root).parts
    named = date.fromisoformat(parts[-1])
    if named.isoformat() != parts[-1]:
        raise ValueError(f"the day folder {parts[-1]!r} is not an ISO date")
    return {"folders": list(parts[:-1]), "day": named}


def token(day):
    return "{day:%d}" % day


def sentence(rng):
    words = [rng.choice(WORDS) for _ in range(rng.randrange(4, 10))]
    return " ".join(words).capitalize() + "."


def body(rng, lines):
    return "".join(sentence(rng) + "\n" for _ in range(lines))


def digest(rng):
    return "".join(rng.choice("0123456789abcdef") for _ in range(64))


def header(topic, day, first, last, rolls, value):
    """The frontmatter's key lines, as SPEC-042's note writes them, as (key, value) pairs."""
    return [
        ("type", "reading"),
        ("topic", topic),
        ("date", token(day)),
        ("first_generated", token(first)),
        ("last_rolled", token(last)),
        ("rolls", str(rolls)),
        ("digest", value),
        ("tags", f"[reading, {topic}]"),
        ("ai_generated", "true"),
    ]


def note(keys, text, *, studied=False, read=False, extra=()):
    """A whole note: the delimiters around `keys` (and any extra raw lines), the body, the boxes."""
    lines = ["---", *(f"{key}: {value}" for key, value in keys), *extra, "---"]
    boxes = [
        (TICKED if studied else UNTICKED)["studied"],
        (TICKED if read else UNTICKED)["read"],
    ]
    return "\n".join(lines) + "\n" + text + "\n\n" + "\n".join(boxes) + "\n"


def fresh(rng, **overrides):
    """A note as it is first written, with `overrides` replacing its key values."""
    topic = rng.choice(TOPICS)
    day = rng.randrange(18_000, 22_000)
    keys = dict(header(topic, day, day, day, 0, digest(rng)))
    keys.update(overrides)
    return day, [(key, value) for key, value in keys.items() if value is not None]


def roll_case(text, today):
    return {"text": text, "today": today}


def roll_cases(rng):
    """45 classed cases and 8 ordinary ones: 53 in all."""
    drawn = []
    # A note as it is first written, rolled one to three days later.
    for _ in range(4):
        day, keys = fresh(rng)
        text = note(keys, body(rng, rng.randrange(1, 4)))
        drawn.append(("fresh", roll_case(text, day + rng.randrange(1, 4))))
    # A note rolled before: rolls counts the nights it was carried.
    for _ in range(2):
        rolls = rng.randrange(1, 6)
        day, keys = fresh(rng, rolls=str(rolls))
        keys = [(k, token(day + rolls) if k == "last_rolled" else v) for k, v in keys]
        drawn.append(("rolled", roll_case(note(keys, body(rng, 2)), day + rolls + 1)))
    # Line endings: every line CRLF; a CRLF body under an LF frontmatter; a CRLF opening delimiter
    # over LF keys; an LF opening delimiter over a CRLF closing one; a lone carriage return.
    for _ in range(2):
        day, keys = fresh(rng)
        text = note(keys, body(rng, 2), read=rng.random() < 0.5).replace("\n", "\r\n")
        drawn.append(("crlf", roll_case(text, day + 1)))
    day, keys = fresh(rng)
    text = note(keys, body(rng, 3))
    split = text.index("\n---\n") + len("\n---\n")
    drawn.append(("crlf", roll_case(text[:split] + text[split:].replace("\n", "\r\n"), day + 1)))
    day, keys = fresh(rng)
    text = note(keys, body(rng, 2))
    drawn.append(("crlf", roll_case("---\r\n" + text[len("---\n") :], day + 2)))
    day, keys = fresh(rng)
    text = note(keys, body(rng, 2))
    drawn.append(("crlf", roll_case(text.replace("\n---\n", "\n---\r\n", 1), day + 1)))
    day, keys = fresh(rng)
    drawn.append(("cr", roll_case(note(keys, body(rng, 2)).replace("\n", "\r"), day + 1)))
    # A key the note lacks is appended: first_generated (twice), rolls and last_rolled.
    for _ in range(2):
        day, keys = fresh(rng, first_generated=None)
        drawn.append(("missing-key", roll_case(note(keys, body(rng, 2)), day + 1)))
    day, keys = fresh(rng, rolls=None)
    drawn.append(("missing-key", roll_case(note(keys, body(rng, 2)), day + 2)))
    day, keys = fresh(rng, last_rolled=None)
    drawn.append(("missing-key", roll_case(note(keys, body(rng, 2)), day + 1)))
    # The owner's ticks survive: I read it, Studied, both, and both in a CRLF note.
    for studied, read, crlf in ((False, True, False), (True, False, False), (True, True, True)):
        day, keys = fresh(rng)
        text = note(keys, body(rng, 2), studied=studied, read=read)
        if crlf:
            text = text.replace("\n", "\r\n")
        drawn.append(("owner-tick", roll_case(text, day + 1)))
    # Only a top-level key is patched: never an indented one, a tab-indented one or a list item.
    day, keys = fresh(rng)
    extra = ("meta:", "  rolls: 9", "\trolls: 8", "- rolls: 5")
    drawn.append(("nested-key", roll_case(note(keys, body(rng, 2), extra=extra), day + 1)))
    day, keys = fresh(rng, rolls=None)
    extra = ("meta:", "  rolls: 9", "  last_rolled: " + token(day - 3))
    drawn.append(("nested-key", roll_case(note(keys, body(rng, 2), extra=extra), day + 1)))
    # A repeated key: the first is patched and the second is kept as written.
    day, keys = fresh(rng)
    drawn.append(
        ("repeated-key", roll_case(note(keys, body(rng, 1), extra=("rolls: 7",)), day + 1))
    )
    # A `---` rule and a key-shaped line in the body are body, not frontmatter.
    day, keys = fresh(rng)
    text = note(keys, body(rng, 1) + "---\nrolls: 99\n---\n" + body(rng, 1))
    drawn.append(("body-rule", roll_case(text, day + 1)))
    # No final newline: a body that ends the file, and a closing delimiter that ends it.
    day, keys = fresh(rng)
    text = "\n".join(["---", *(f"{k}: {v}" for k, v in keys), "---", sentence(rng)])
    drawn.append(("no-final-newline", roll_case(text, day + 1)))
    day, keys = fresh(rng)
    text = "\n".join(["---", *(f"{k}: {v}" for k, v in keys), "---"])
    drawn.append(("no-final-newline", roll_case(text, day + 1)))
    # Line boundaries Python's splitlines reads beyond \n and \r: a line separator inside a value,
    # a form feed and an information separator after one, and a paragraph separator in the body.
    day, keys = fresh(rng, rolls=None)
    keys = [(k, v + "\u2028rolls: 7" if k == "digest" else v) for k, v in keys]
    drawn.append(("separator", roll_case(note(keys, body(rng, 1)), day + 1)))
    day, keys = fresh(rng, rolls="2\x0c")
    drawn.append(("separator", roll_case(note(keys, body(rng, 1)), day + 1)))
    day, keys = fresh(rng, rolls="3\x1e")
    drawn.append(("separator", roll_case(note(keys, body(rng, 1)), day + 1)))
    day, keys = fresh(rng)
    text = note(keys, sentence(rng) + "\u2029" + sentence(rng) + "\n")
    drawn.append(("separator", roll_case(text, day + 1)))
    # The spellings of rolls Python's int reads and DeckStreak reads too: leading zeros, a sign, a
    # negative count, surrounding whitespace, and a count past a 64-bit integer.
    for spelling in ("007", "+2", "-1", "-7", "  4  ", "\t3", "12345678901234567890"):
        day, keys = fresh(rng, rolls=spelling)
        drawn.append(("spelling", roll_case(note(keys, body(rng, 1)), day + 1)))
    # The spellings only Python's int reads (ADR-042): a digit separator, a digit outside ASCII,
    # and a count past a 128-bit integer. DeckStreak refuses each as a malformed note.
    for spelling in ("1_0", "\u0663", "9" * 40):
        day, keys = fresh(rng, rolls=spelling)
        drawn.append(("python-only", roll_case(note(keys, body(rng, 1)), day + 1)))
    # Notes the function refuses: no opening delimiter, a blank line before it, no closing
    # delimiter, and a rolls value that is not a whole number or is empty.
    day, keys = fresh(rng)
    text = note(keys, body(rng, 1))
    drawn.append(("refused", roll_case("# " + sentence(rng) + "\n" + text, day + 1)))
    drawn.append(("refused", roll_case("\n" + text, day + 1)))
    drawn.append(("refused", roll_case(text.replace("\n---\n", "\n", 1), day + 1)))
    for spelling in ("two", "3.0", ""):
        day, keys = fresh(rng, rolls=spelling)
        drawn.append(("refused", roll_case(note(keys, body(rng, 1)), day + 1)))
    # Ordinary notes: any topic, any count, either box ticked or not, either line ending.
    for _ in range(8):
        rolls = rng.randrange(0, 12)
        day, keys = fresh(rng, rolls=str(rolls))
        text = note(
            keys,
            body(rng, rng.randrange(1, 5)),
            studied=rng.random() < 0.5,
            read=rng.random() < 0.5,
        )
        if rng.random() < 0.3:
            text = text.replace("\n", "\r\n")
        drawn.append((None, roll_case(text, day + rolls + 1)))
    return drawn


def epoch_day(year, month, day):
    return date(year, month, day).toordinal() - EPOCH_ORDINAL


def archive_cases(rng):
    """96 year-boundary, 16 leap, 5 century, 4 before-epoch and 16 ordinary cases: 137 in all."""
    drawn = []
    # The last four and first four days of twelve calendar years: where a day's ISO week belongs
    # to the other calendar year, its month folder and its week folder disagree.
    for year in sorted(rng.sample(range(1971, 2100), 12)):
        for month, days in ((12, range(28, 32)), (1, range(1, 5))):
            for day in days:
                drawn.append(("year-boundary", {"day": epoch_day(year, month, day)}))
    # Leap days, and the days around them.
    for year in sorted(rng.sample(range(1972, 2100, 4), 4)):
        for month, day in ((2, 28), (2, 29), (3, 1), (12, 31)):
            drawn.append(("leap", {"day": epoch_day(year, month, day)}))
    # The century years: 1900 and 2100 have no 29th of February, and 2000 has one.
    for year, month, day in (
        (1900, 2, 28),
        (1900, 3, 1),
        (2000, 2, 29),
        (2100, 2, 28),
        (2100, 3, 1),
    ):
        drawn.append(("century", {"day": epoch_day(year, month, day)}))
    # Days before the Unix epoch, where a division that truncates gets the day wrong.
    for day in (-1, -2, -365, -rng.randrange(366, 25_000)):
        drawn.append(("before-epoch", {"day": day}))
    # Ordinary days.
    for _ in range(16):
        drawn.append((None, {"day": rng.randrange(0, 73_000)}))
    return drawn


FUNCTIONS = {
    "roll_note_text": {
        "kind": "adapter",
        "function": "reading_notes._roll_note_text",
        "adapter": with_days_as_numbers,
        "note": (
            "Expands each {day:N} token of text into the ISO date of epoch day N, calls the "
            "function with today as the date of epoch day today, and writes each ISO date of the "
            "returned text back as its token; a refused note is recorded as the class of the "
            "exception the function raised."
        ),
        "cases": roll_cases,
    },
    "archive_dir": {
        "kind": "adapter",
        "function": "reading_notes._archive_dir",
        "adapter": under_a_neutral_root,
        "note": (
            "Calls the function with the relative root vault and the date of epoch day day, and "
            "returns the folders of the returned path below that root, with its last folder given "
            "as the date it names."
        ),
        "cases": archive_cases,
    },
}

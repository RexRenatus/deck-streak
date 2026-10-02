"""SPEC-118's registrations: the inbox capture's stem and stub.

The adapter builds what JSON cannot carry and CALLS the predecessor; it computes no rule.

* `save_inbox_capture` drives `vault_bridge.py:save_inbox_capture` over a temporary vault root that
  already holds its inbox folder, with a settings stand-in that carries only that root, the case's
  instant as an aware UTC `datetime`, and synthetic bytes built from the case's kind. The root is a
  `Path` whose writes are recorded as they are made, so the adapter learns the stub's text without
  reading a file back. A golden may hold no calendar date and no clock time, so the adapter writes
  each instant of the returned text to the second as the token `{second:N}`, N its epoch second,
  and each remaining day as the token `{day:N}`, N its epoch day number; a token that does not
  expand back into exactly the text the predecessor wrote is refused, and so is a case whose own
  text holds a date.

The case builder draws only from the `random.Random` the generator seeds. It covers each kind the
bot captures, an empty and a blank caption, captions whose ends Python's `str.strip` trims (the
separators U+001C to U+001F among them, which Rust's `str::trim` keeps), a caption of several lines,
a unique with symbols, one of 32 safe characters, one of 40, an empty one, one of symbols only, one
with letters outside ASCII, one that is longer than 32 only before its symbols are removed, an
extension with a dot, without one and empty, and instants with a part of a second, at either side
of midnight, on a leap day and before the Unix epoch.
"""

import re
import tempfile
import types
from datetime import UTC, date, datetime, timedelta
from pathlib import Path

#: The Unix epoch, as the aware instant every case's milliseconds count from.
EPOCH = datetime(1970, 1, 1, tzinfo=UTC)
#: The ordinal of the Unix epoch's day, so a `date` and its epoch day number convert exactly.
EPOCH_ORDINAL = date(1970, 1, 1).toordinal()
#: An instant to the second in UTC, as the stub's `captured:` line spells it.
ISO_SECOND = re.compile(
    r"(?<![0-9])[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\+00:00(?![0-9])"
)
#: An ISO calendar date: the stem's day, in the names and the stub.
ISO_DAY = re.compile(r"(?<![0-9])[0-9]{4}-[0-9]{2}-[0-9]{2}(?![0-9])")
#: The tokens a golden's text carries in their place.
SECOND_TOKEN = re.compile(r"\{second:(-?[0-9]+)\}")
DAY_TOKEN = re.compile(r"\{day:(-?[0-9]+)\}")
#: A date or a clock time anywhere in a case's own text, which the case builder never draws.
CALENDAR = re.compile(r"[0-9]{4}-[0-9]{2}-[0-9]{2}|[0-9]:[0-9]{2}|[0-9]/[0-9]")

KINDS = ("photo", "voice", "document")
SAFE = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-"
SYMBOLS = " .,!?#$%&*()+=~'\"<>[]{}|\\^`@;"
WORDS = (
    "receipt",
    "whiteboard",
    "lecture",
    "hearsay",
    "kanji",
    "outline",
    "voice",
    "memo",
    "torts",
    "primer",
    "reading",
    "draft",
)
#: The ends Python's `str.strip` trims: ASCII space and controls, the separators U+001C to
#: U+001F, the next line, no-break, ideographic and line separators.
PYTHON_SPACE = " \t\n\r\x0b\x0c\x1c\x1d\x1e\x1f\x85\xa0 　"


def as_second(epoch_second):
    return (EPOCH + timedelta(seconds=epoch_second)).isoformat()


def as_day(epoch_day):
    return date.fromordinal(EPOCH_ORDINAL + epoch_day).isoformat()


def expand(text):
    """`text` with every `{second:N}` and `{day:N}` token written as the text it stands for."""
    text = SECOND_TOKEN.sub(lambda match: as_second(int(match[1])), text)
    return DAY_TOKEN.sub(lambda match: as_day(int(match[1])), text)


def contract(text):
    """`text` with every UTC instant to the second written as its `{second:N}` token and every
    remaining ISO date as its `{day:N}` token. A token that does not expand back into exactly
    `text` is refused, so a token always names what the predecessor wrote."""

    def second(match):
        instant = datetime.fromisoformat(match[0])
        return "{second:%d}" % ((instant - EPOCH) // timedelta(seconds=1))

    def day(match):
        return "{day:%d}" % (date.fromisoformat(match[0]).toordinal() - EPOCH_ORDINAL)

    contracted = ISO_DAY.sub(day, ISO_SECOND.sub(second, text))
    if expand(contracted) != text:
        raise ValueError(f"{text!r} does not read back from its tokens")
    return contracted


def with_a_temporary_vault(
    save_inbox_capture, predecessor, *, kind, unique, ext, caption, instant_ms
):
    """Capture into a temporary vault root holding its inbox, recording each write as it is made,
    and return the folder, the names in the order they were written and the stub, as tokens."""
    for text in (unique, ext, caption):
        if CALENDAR.search(text):
            raise ValueError(f"the case's text {text!r} holds a date or a time")
    written = []

    class Recording(type(Path())):
        def write_bytes(self, data):
            written.append((self.name, None))
            return super().write_bytes(data)

        def write_text(self, data, encoding=None, errors=None, newline=None):
            written.append((self.name, data))
            return super().write_text(data, encoding=encoding, errors=errors, newline=newline)

    inbox_folder = predecessor("vault_bridge._INBOX_RELATIVE_DIR")
    with tempfile.TemporaryDirectory() as scratch:
        root = Recording(scratch)
        (root / inbox_folder).mkdir()
        attachment = save_inbox_capture(
            types.SimpleNamespace(vault_replica_path=root),
            kind=kind,
            data=f"synthetic {kind} bytes".encode(),
            ext=ext,
            caption=caption,
            when=EPOCH + timedelta(milliseconds=instant_ms),
            unique=unique,
        )
        folder = attachment.parent.relative_to(root).as_posix()
    stubs = [(name, text) for name, text in written if text is not None]
    if len(stubs) != 1:
        raise ValueError(f"the function wrote {len(stubs)} stubs")
    return {
        "folder": folder,
        "attachment": contract(attachment.name),
        "written": [contract(name) for name, _ in written],
        "stub": contract(stubs[0][1]),
    }


def safe_unique(rng, length):
    return "".join(rng.choice(SAFE) for _ in range(length))


def drawn_unique(rng, length):
    """A unique of `length` safe characters, drawn again until it holds no date or time."""
    while True:
        unique = safe_unique(rng, length)
        if not CALENDAR.search(unique):
            return unique


def sentence(rng):
    return " ".join(rng.choice(WORDS) for _ in range(rng.randrange(2, 7))).capitalize()


def instant(rng):
    """An instant between 2020 and 2035, with a part of a second."""
    return rng.randrange(1_577_836_800_000, 2_051_222_400_000)


def capture(rng, **overrides):
    case = {
        "kind": rng.choice(KINDS),
        "unique": drawn_unique(rng, rng.randrange(8, 25)),
        "ext": rng.choice((".jpg", ".ogg", ".pdf", "png", "txt")),
        "caption": sentence(rng),
        "instant_ms": instant(rng),
    }
    case.update(overrides)
    return case


def epoch_ms(year, month, day, hour=0, minute=0, second=0, ms=0):
    moment = datetime(year, month, day, hour, minute, second, tzinfo=UTC)
    return (moment - EPOCH) // timedelta(milliseconds=1) + ms


def capture_cases(rng):
    drawn = []
    # Each kind, with the extension its caller passes.
    for kind, ext in (("photo", ".jpg"), ("voice", ".ogg"), ("document", ".pdf")):
        for _ in range(3):
            drawn.append(("kind", capture(rng, kind=kind, ext=ext)))
    # Captions: empty, blank, trimmed at both ends, and several lines kept as they are.
    drawn.append(("empty-caption", capture(rng, caption="")))
    for blank in (" ", "\n\n", " \t\r\n ", "\x1c\x1d\x1e\x1f", "\xa0　 \x85"):
        drawn.append(("blank-caption", capture(rng, caption=blank)))
    for _ in range(6):
        left = "".join(rng.choice(PYTHON_SPACE) for _ in range(rng.randrange(1, 4)))
        right = "".join(rng.choice(PYTHON_SPACE) for _ in range(rng.randrange(1, 4)))
        drawn.append(("python-strip", capture(rng, caption=f"{left}{sentence(rng)}{right}")))
    drawn.append(("python-strip", capture(rng, caption=f"\x1f{sentence(rng)}\x1c")))
    drawn.append(("lines", capture(rng, caption=f"{sentence(rng)}\n\n{sentence(rng)}\n")))
    drawn.append(("lines", capture(rng, caption=f"{sentence(rng)}\r\n{sentence(rng)}")))
    drawn.append(("lines", capture(rng, caption=f"  {sentence(rng)} été 日本  ")))
    # Uniques: symbols removed, cut to 32 after the removal, and the fallback.
    for _ in range(4):
        safe = drawn_unique(rng, rng.randrange(6, 20))
        mixed = "".join(c + (rng.choice(SYMBOLS) if rng.random() < 0.4 else "") for c in safe)
        drawn.append(("unique-symbols", capture(rng, unique=mixed)))
    drawn.append(("unique-symbols", capture(rng, unique="ab:cd/ef")))
    drawn.append(("unique-32", capture(rng, unique=drawn_unique(rng, 32))))
    drawn.append(("unique-33", capture(rng, unique=drawn_unique(rng, 33))))
    drawn.append(("unique-40", capture(rng, unique=drawn_unique(rng, 40))))
    padded = "".join(c + "." for c in drawn_unique(rng, 20))
    drawn.append(("unique-cut-after-removal", capture(rng, unique=padded)))
    drawn.append(("unique-empty", capture(rng, unique="")))
    drawn.append(("unique-symbols-only", capture(rng, unique="!@#$%^&*().")))
    drawn.append(("unique-non-ascii", capture(rng, unique="Ünïcødé_日-ok")))
    # Extensions: with a dot, without one, and empty.
    for ext in (".jpg", "jpg", "ogg", ".tar", ""):
        drawn.append(("ext", capture(rng, ext=ext)))
    # Instants: a part of a second, either side of midnight, a leap day, before the epoch.
    for ms in (0, 1, 999):
        drawn.append(("sub-second", capture(rng, instant_ms=epoch_ms(2026, 10, 2, 9, 5, 7, ms))))
    drawn.append(("midnight", capture(rng, instant_ms=epoch_ms(2025, 12, 31, 23, 59, 59, 999))))
    drawn.append(("midnight", capture(rng, instant_ms=epoch_ms(2026, 1, 1))))
    drawn.append(("leap", capture(rng, instant_ms=epoch_ms(2028, 2, 29, 12, 30, 45, 500))))
    drawn.append(("before-epoch", capture(rng, instant_ms=-1)))
    drawn.append(("before-epoch", capture(rng, instant_ms=-86_400_001)))
    # Ordinary captures.
    for _ in range(8):
        drawn.append((None, capture(rng)))
    return drawn


FUNCTIONS = {
    "inbox_capture_stub": {
        "kind": "adapter",
        "function": "vault_bridge.save_inbox_capture",
        "adapter": with_a_temporary_vault,
        "note": (
            "Calls the function over a temporary vault root that holds its inbox folder, with a "
            "settings stand-in carrying only that root, when as the aware UTC instant instant_ms "
            "after the epoch, and synthetic bytes; records each write as it is made and returns "
            "the attachment's folder, its name, the names in the order written and the stub's "
            "text, each instant to the second as a {second:N} token and each day as a {day:N} "
            "token."
        ),
        "cases": capture_cases,
    },
}

"""SPEC-118's registrations: the inbox capture's stem and stub, and the bot's media choice and
replies.

Each adapter builds what JSON cannot carry and CALLS the predecessor; it computes no rule.

* `save_inbox_capture` drives `vault_bridge.py:save_inbox_capture` over a temporary vault root that
  already holds its inbox folder, with a settings stand-in that carries only that root, the case's
  instant as an aware UTC `datetime`, and synthetic bytes built from the case's kind. The root is a
  `Path` whose writes are recorded as they are made, so the adapter learns the stub's text without
  reading a file back. A golden may hold no calendar date and no clock time, so the adapter writes
  each instant of the returned text to the second as the token `{second:N}`, N its epoch second,
  and each remaining day as the token `{day:N}`, N its epoch day number; a token that does not
  expand back into exactly the text the predecessor wrote is refused, and so is a case whose own
  text holds a date.
* `media_capture_choice` drives `bot.py:CommandBot._maybe_capture_media` on the predecessor's bot,
  built with a synthetic token and chat id, whose file capture is replaced by a recorder that
  answers success. It returns what the function returned and each capture it asked for: the file
  id, the kind, the extension, the caption and the unique. Each input is a whole synthetic Telegram
  message, so the Rust side reads the same text the Bot API sends.
* `media_capture_replies` drives `bot.py:CommandBot._capture_file` on the same bot, with the
  download and the pipeline's inbox writer replaced by fakes that succeed or fail as the case says,
  and every line it sends recorded. It returns what the function returned, the downloads and the
  saves it asked for (the bytes as text) and the lines it sent.

The two media case builders draw nothing: each case is fixed. Every document name in them keeps an
extension on which SPEC-118 R7 and the predecessor agree; R7's departures are unit tests (A8).

The stub's case builder draws only from the `random.Random` the generator seeds. It covers each
kind the bot captures, an empty and a blank caption, captions whose ends Python's `str.strip` trims
(the separators U+001C to U+001F among them, which Rust's `str::trim` keeps), a caption of several
lines, a unique with symbols, one of 32 safe characters, one of 40, an empty one, one of symbols
only, one with letters outside ASCII, one that is longer than 32 only before its symbols are
removed, an extension with a dot, without one and empty, and instants with a part of a second, at
either side of midnight, on a leap day and before the Unix epoch.
"""

import asyncio
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


#: The bot every media case is built on: a synthetic token, and the owner's private chat as a
#: synthetic id that is nobody's.
SYNTHETIC_TOKEN = "100000-synthetic-token"
SYNTHETIC_CHAT = 4242


def a_bot(predecessor, pipeline):
    """The predecessor's own bot, built with the synthetic token and chat id over `pipeline`."""
    return predecessor("bot.CommandBot")(SYNTHETIC_TOKEN, SYNTHETIC_CHAT, pipeline)


def on_the_bot(bot, call):
    """Run `call` on the bot's loop, then close the bot's HTTP client, which sent no request."""

    async def run():
        try:
            return await call
        finally:
            await bot._client.aclose()

    return asyncio.run(run())


def with_a_recorded_capture(maybe_capture_media, predecessor, *, message):
    """Call the function on a bot whose file capture is recorded and answers success, and return
    what it returned and each capture it asked for."""
    calls = []

    async def capture_file(file_id, *, kind, ext, caption, unique):
        calls.append(
            {"file_id": file_id, "kind": kind, "ext": ext, "caption": caption, "unique": unique}
        )
        return True

    bot = a_bot(predecessor, types.SimpleNamespace())
    bot._capture_file = capture_file
    captured = on_the_bot(bot, maybe_capture_media(bot, message))
    return {"captured": captured, "calls": calls}


def with_a_faked_fetch_and_save(
    capture_file, predecessor, *, file_id, kind, ext, caption, unique, download, save
):
    """Call the function on a bot whose download answers `download` (text as bytes, or nothing)
    and whose inbox writer answers `save`, and return what it returned, each download and save it
    asked for, and each line it sent."""
    downloads, saves, sent = [], [], []

    async def download_telegram_file(asked):
        downloads.append(asked)
        return None if download is None else download.encode()

    async def save_inbox_capture(**asked):
        saves.append({**asked, "data": asked["data"].decode()})
        return dict(save)

    async def send(text, *, reply_markup=None):
        sent.append(text)
        return len(sent)

    bot = a_bot(predecessor, types.SimpleNamespace(save_inbox_capture=save_inbox_capture))
    bot._download_telegram_file = download_telegram_file
    bot._send = send
    returned = on_the_bot(
        bot,
        capture_file(bot, file_id, kind=kind, ext=ext, caption=caption, unique=unique),
    )
    return {"returned": returned, "downloads": downloads, "saves": saves, "sent": sent}


def a_message(message_id, **media):
    """A synthetic message from the owner in the owner's private chat, carrying `media`."""
    message = {
        "message_id": message_id,
        "date": 0,
        "chat": {"id": SYNTHETIC_CHAT, "type": "private"},
        "from": {"id": SYNTHETIC_CHAT, "is_bot": False, "first_name": "Owner"},
    }
    message.update(media)
    return message


def a_size(n, width, height):
    return {
        "file_id": f"photo-file-{n}",
        "file_unique_id": f"photo-unique-{n}",
        "width": width,
        "height": height,
        "file_size": width * height // 8,
    }


def a_voice(n, **fields):
    voice = {"file_id": f"voice-file-{n}", "file_unique_id": f"voice-unique-{n}", "duration": 7}
    return voice | fields


def a_document(n, **fields):
    return {"file_id": f"document-file-{n}", "file_unique_id": f"document-unique-{n}"} | fields


def choice_cases(rng):
    """Each kind, the predecessor's order between kinds, a document's name and caption, a message
    with no media, and media with no unique or no file id."""
    three = [a_size(1, 90, 60), a_size(2, 320, 240), a_size(3, 1280, 960)]
    cases = [
        ("photo-sizes", a_message(1, photo=three, caption="Whiteboard after torts")),
        ("photo-one-size", a_message(2, photo=[a_size(4, 640, 480)])),
        ("voice", a_message(3, voice=a_voice(1), caption="Hearsay memo")),
        ("document-one-dot", a_message(4, document=a_document(1, file_name="lecture-notes.pdf"))),
        (
            "document-two-dots",
            a_message(5, document=a_document(2, file_name="outline.tar.gz"), caption="Outline"),
        ),
        ("document-no-dot", a_message(6, document=a_document(3, file_name="README"))),
        ("document-no-name", a_message(7, document=a_document(4))),
        ("document-leading-dot", a_message(8, document=a_document(5, file_name=".hidden"))),
        ("document-ten-characters", a_message(9, document=a_document(6, file_name="a.abcdefghij"))),
        ("document-upper-case", a_message(10, document=a_document(7, file_name="SCAN.PDF"))),
        (
            "document-caption-kept",
            a_message(11, document=a_document(8, file_name="primer.txt"), caption="Read first"),
        ),
        (
            "document-empty-caption",
            a_message(12, document=a_document(9, file_name="kanji.png"), caption=""),
        ),
        (
            "document-non-ascii-name",
            a_message(13, document=a_document(10, file_name="Été 日本.md")),
        ),
        ("order", a_message(14, photo=three, document=a_document(11, file_name="draft.pdf"))),
        ("order", a_message(15, voice=a_voice(2), document=a_document(12, file_name="memo.txt"))),
        ("order", a_message(16, photo=[a_size(5, 640, 480)], voice=a_voice(3))),
        (
            "order",
            a_message(
                17,
                photo=[a_size(6, 640, 480)],
                voice=a_voice(4),
                document=a_document(13, file_name="reading.pdf"),
            ),
        ),
        ("empty-photo-list", a_message(18, photo=[])),
        ("empty-photo-list", a_message(19, photo=[], document=a_document(14, file_name="a.txt"))),
        ("no-media", a_message(20, text="Not media")),
        ("no-media", a_message(21)),
        ("empty-unique", a_message(22, photo=[a_size(7, 640, 480) | {"file_unique_id": ""}])),
        ("empty-file-id", a_message(23, voice=a_voice(5, file_id=""))),
        ("caption-empty", a_message(24, photo=[a_size(8, 640, 480)], caption="")),
        ("caption-absent", a_message(25, voice=a_voice(6))),
        ("caption-non-ascii", a_message(26, voice=a_voice(7), caption="Été 日本 📷 notes")),
    ]
    return [(edge, {"message": message}) for edge, message in cases]


def reply_case(n, *, kind="photo", ext=".jpg", download="synthetic bytes", save, **fields):
    case = {
        "file_id": f"{kind}-file-{n}",
        "kind": kind,
        "ext": ext,
        "caption": "Synthetic caption",
        "unique": f"{kind}-unique-{n}",
        "download": download,
        "save": save,
    }
    case.update(fields)
    return case


def replies_cases(rng):
    """Silence with no file id, a failed fetch, a save of each kind, a name that needs escaping,
    a refused save, and an empty unique that falls back to the file id."""
    saved = {"ok": True, "filename": "inbox-photo-unique-1.jpg"}
    return [
        ("no-file-id", reply_case(1, save=saved, file_id="")),
        ("fetch-failed", reply_case(2, save=saved, download=None)),
        ("saved", reply_case(3, save=saved)),
        (
            "saved",
            reply_case(4, kind="voice", ext=".ogg", save={"ok": True, "filename": "a-voice.ogg"}),
        ),
        (
            "saved",
            reply_case(5, kind="document", ext=".pdf", save={"ok": True, "filename": "notes.pdf"}),
        ),
        (
            "escaped-name",
            reply_case(
                6,
                kind="document",
                ext=".pdf",
                save={"ok": True, "filename": 'Tom & Jerry <draft> "v2" it\'s.pdf'},
            ),
        ),
        (
            "saved-non-ascii-name",
            reply_case(7, kind="document", ext=".md", save={"ok": True, "filename": "Été 日本.md"}),
        ),
        ("save-refused", reply_case(8, save={"ok": False, "error": "vault_missing"})),
        ("save-refused", reply_case(9, save={"ok": False})),
        ("unique-fallback", reply_case(10, save=saved, unique="")),
    ]


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
    "media_capture_choice": {
        "kind": "adapter",
        "function": "bot.CommandBot._maybe_capture_media",
        "adapter": with_a_recorded_capture,
        "note": (
            "Calls the function on the predecessor's bot, built with a synthetic token and chat "
            "id, whose file capture is replaced by a recorder that answers success; message is a "
            "whole synthetic Telegram message; returns what the function returned and each "
            "capture it asked for, with its file id, kind, extension, caption and unique."
        ),
        "cases": choice_cases,
    },
    "media_capture_replies": {
        "kind": "adapter",
        "function": "bot.CommandBot._capture_file",
        "adapter": with_a_faked_fetch_and_save,
        "note": (
            "Calls the function on the same bot, whose download answers download as bytes or "
            "nothing and whose inbox writer answers save; returns what the function returned, "
            "each download and save it asked for, the bytes as text, and each line it sent."
        ),
        "cases": replies_cases,
    },
}

#!/usr/bin/env python3
"""telegram-platform-probe: judges a Telegram bot and Mini App against the platform's own rules.

SPEC-V2-2219 / ADR-V2-2219, the check behind `skills/packs/telegram-platform`. It is standard-library
Python and vendorable, and it judges any tree through `--root`: DeckStreak's Rust workspace, its
SvelteKit Mini App, or phoenix-v2's own `ops/telegram`. Three stages:

  bot-api    the Bot API transport and messaging, read from shipped source (13 classes);
  mini-app   the Mini App platform, read from shipped web source (3 classes);
  payload    committed outbound message payloads, `*.msg.json` (3 classes).

The payload stage is also the ONE home of Telegram message validation: nudge-duties (d2217) calls
`check_message`, `check_keyboard` and `parse_text` below instead of keeping its own copies, so those
functions and the Parsed/Entity shapes are a published API that stays stable.

Output is one line per finding, `<class>: <finding>`, then `examined N`. A tree class examines the
source files it reads; a payload class examines the payloads of a `--subject` directory, or, over a
`--root`, the files it walked. Exit 0 green, 1 a finding, 2 usage, 3 VOID: nothing was examined,
which is never a pass.

Everything here is a STATIC read. Runtime behaviour (pacing to 30 messages a second, what a handler
does after it answers, whether a webhook answers within the timeout) is taught in the pack's
SKILL.md, and SPEC-V2-2219's coverage matrix names each such practice and why it is excluded.
"""

import argparse
import io
import json
import re
import sys
import tokenize
from collections.abc import Iterator
from dataclasses import dataclass, replace
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

TEXT_MAX = 4096
CAPTION_MAX = 1024
CALLBACK_DATA_MAX_BYTES = 64
ANSWER_TEXT_MAX = 200
COPY_TEXT_MAX = 256
START_PARAM_MAX = 64
STARTAPP_PARAM_MAX = 512
CLOUD_VALUE_MAX = 4096
SECURE_STORAGE_ITEMS = 10
WEBHOOK_PORTS = frozenset({443, 80, 88, 8443})
MARKDOWN_V2_SPECIALS = "_*[]()~`>#+-=|{}.!"
HTML_NAMED_ENTITIES = {"lt": "<", "gt": ">", "amp": "&", "quot": '"'}
DATE_TIME_FORMAT = re.compile(r"^(?:r|w?[dD]?[tT]?)$")

BOT_API_CLASSES = (
    "tg-update-mode",
    "tg-webhook-endpoint",
    "tg-poll-offset",
    "tg-poll-long",
    "tg-allowed-updates",
    "tg-retry-after",
    "tg-parse-mode",
    "tg-escape",
    "tg-length-bound",
    "tg-callback-answer",
    "tg-callback-data",
    "tg-deep-link",
    "tg-replaced-fields",
)
MINI_APP_CLASSES = ("tma-version-gate", "tma-storage", "tma-send-data")
PAYLOAD_CLASSES = ("payload-length", "payload-markup", "payload-keyboard")
CLASSES = BOT_API_CLASSES + MINI_APP_CLASSES + PAYLOAD_CLASSES

# The tree walk. Build output and dependencies are never shipped source; test code builds broken
# fixtures on purpose; a vendored copy of the official Mini App script, or of these probes, would
# otherwise be judged as the repository's own code.
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
        ".vercel",
        ".cache",
        ".pnpm-store",
        ".mypy_cache",
        ".ruff_cache",
        ".pytest_cache",
    }
)
TEST_DIRS = frozenset(
    {"tests", "test", "__tests__", "e2e", "fixtures", "__mocks__", "testdata"}
)
TEST_FILE = re.compile(
    r"(?:^test_.*\.py$|_test\.py$|^conftest\.py$|\.(?:test|spec)\.[cm]?[jt]sx?$)"
)
PROBE_FILES = frozenset({"telegram-platform-probe.py", "notifications-policy-probe.py"})
VENDORED = re.compile(r"(?:^telegram-web-app\.js$|\.min\.js$|\.bundle\.js$)")
BOT_EXTS = frozenset(
    {".rs", ".py", ".ts", ".js", ".mjs", ".cjs", ".mts", ".cts", ".tsx", ".jsx"}
)
WEB_EXTS = frozenset(
    {
        ".html",
        ".htm",
        ".svelte",
        ".vue",
        ".astro",
        ".ts",
        ".js",
        ".mjs",
        ".mts",
        ".tsx",
        ".jsx",
    }
)
MARKUP_EXTS = frozenset({".html", ".htm", ".svelte", ".vue", ".astro"})
# The bot languages web-launch never reads: deep links there are this pack's to judge.
BOT_ONLY_EXTS = frozenset({".rs", ".py"})
SOURCE_EXTS = BOT_EXTS | WEB_EXTS
MAX_SOURCE_BYTES = 2_000_000

# Bot API methods, each recognised as a string literal or URL segment ("getUpdates",
# /getUpdates), a method call (.getUpdates( or getUpdates(), or its snake_case call
# (get_updates( in frankenstein and the Python libraries).
BOT_METHODS = (
    "getUpdates",
    "setWebhook",
    "deleteWebhook",
    "getWebhookInfo",
    "getMe",
    "sendMessage",
    "sendPhoto",
    "sendDocument",
    "sendVideo",
    "sendAnimation",
    "sendAudio",
    "sendVoice",
    "sendMediaGroup",
    "sendDice",
    "sendChatAction",
    "sendRichMessage",
    "editMessageText",
    "editMessageCaption",
    "editMessageReplyMarkup",
    "deleteMessage",
    "copyMessage",
    "forwardMessage",
    "answerCallbackQuery",
    "answerWebAppQuery",
    "setMessageReaction",
    "pinChatMessage",
    "unpinChatMessage",
    "setMyCommands",
    "setChatMenuButton",
)
TEXT_METHODS = ("sendMessage", "editMessageText")
CAPTION_METHODS = (
    "sendPhoto",
    "sendDocument",
    "sendVideo",
    "sendAnimation",
    "sendAudio",
    "sendVoice",
    "sendMediaGroup",
    "editMessageCaption",
)


def _snake(camel: str) -> str:
    return re.sub(r"(?<!^)([A-Z])", r"_\1", camel).lower()


def _method_patterns(camel: str) -> tuple[re.Pattern[str], re.Pattern[str]]:
    """The method as a string literal or URL segment, and as a call (camelCase or snake_case)."""
    snake = _snake(camel)
    literal = re.compile(rf"[\"'`/]{camel}\b")
    call = re.compile(rf"(?:\.{camel}\s*\(|\b{camel}\s*\(|\b{snake}\s*\()")
    return literal, call


METHOD_PATTERNS = {name: _method_patterns(name) for name in BOT_METHODS}
# A bare call such as `send_message(` is generic, so it counts as the Bot API only in a tree that
# uses a bot library, named in its code or its dependency manifests. A string literal such as
# "sendMessage" or a URL segment /sendMessage is the Bot API wherever it appears.
BOT_LIBRARY = re.compile(
    r"\bfrankenstein\b|\bgrammy\b|\btelegraf\b|\bteloxide\b|\baiogram\b|\btelebot\b"
    r"|python-telegram-bot|\btelegram\.ext\b|^\s*(?:from|import)\s+telegram\b|api\.telegram\.org"
    r"|node-telegram-bot-api|\btgbot\b",
    re.I | re.M,
)
MANIFESTS = frozenset(
    {"Cargo.toml", "package.json", "pyproject.toml", "requirements.txt", "deno.json"}
)
WEBHOOK_PARAMS = re.compile(r"\bSetWebhookParams\b")

# Mini App methods the official telegram-web-app.js THROWS WebAppMethodUnsupported on when the
# client's Bot API version is older (read from the script itself, SPEC-V2-2219 R0). Methods the
# script only warns about (HapticFeedback, colours, BackButton, closing confirmation) degrade
# gracefully and need no gate.
THROWING_METHODS = {
    "CloudStorage": "6.9",
    "DeviceStorage": "9.0",
    "SecureStorage": "9.0",
    "invokeCustomMethod": "6.9",
    "requestFullscreen": "8.0",
    "exitFullscreen": "8.0",
    "addToHomeScreen": "8.0",
    "checkHomeScreenStatus": "8.0",
    "switchInlineQuery": "6.7",
    "openInvoice": "6.1",
    "showPopup": "6.2",
    "showAlert": "6.2",
    "showConfirm": "6.2",
    "showScanQrPopup": "6.4",
    "closeScanQrPopup": "6.4",
    "readTextFromClipboard": "6.4",
    "requestWriteAccess": "6.9",
    "requestContact": "6.9",
    "downloadFile": "8.0",
    "shareToStory": "7.8",
    "shareMessage": "8.0",
    "requestChat": "9.6",
    "setEmojiStatus": "8.0",
    "requestEmojiStatusAccess": "8.0",
}

# Fields and methods a later Bot API version replaced or removed (the changelog), with what took
# their place. A model trained before the change still writes the old spelling.
REPLACED_FIELDS = (
    ("reply_to_message_id", "reply_parameters", "7.0"),
    ("allow_sending_without_reply", "reply_parameters", "7.0"),
    ("disable_web_page_preview", "link_preview_options", "7.0"),
    ("forward_from", "forward_origin", "7.0"),
    ("forward_from_chat", "forward_origin", "7.0"),
    ("forward_from_message_id", "forward_origin", "7.0"),
    ("forward_signature", "forward_origin", "7.0"),
    ("forward_sender_name", "forward_origin", "7.0"),
    ("forward_date", "forward_origin", "7.0"),
    ("request_user", "request_users", "7.0"),
    ("user_shared", "users_shared", "7.0"),
    ("KeyboardButtonRequestUser", "KeyboardButtonRequestUsers", "7.0"),
    ("switch_pm_text", "button (InlineQueryResultsButton)", "6.7"),
    ("switch_pm_parameter", "button (InlineQueryResultsButton)", "6.7"),
    ("setStickerSetThumb", "setStickerSetThumbnail", "6.6"),
    (
        "can_send_media_messages",
        "the per-media permissions (can_send_audios and the rest)",
        "6.5",
    ),
    ("hide_url", "nothing: removed", "8.2"),
    ("can_send_gift", "accepted_gift_types", "9.0"),
    ("last_resale_star_count", "last_resale_currency and last_resale_amount", "9.3"),
    (
        "exclude_limited",
        "exclude_limited_upgradable and exclude_limited_non_upgradable",
        "9.3",
    ),
    ("correct_option_id", "correct_option_ids", "9.6"),
)


@dataclass(frozen=True, slots=True)
class Outcome:
    findings: tuple[str, ...]
    examined: int


@dataclass(frozen=True, slots=True)
class Source:
    """One shipped source file: its raw text and its code with every comment blanked."""

    rel: str
    ext: str
    raw: str
    code: str
    bot_library: bool = False

    def line(self, position: int) -> int:
        return self.code.count("\n", 0, position) + 1

    @property
    def is_bot(self) -> bool:
        return self.ext in BOT_EXTS

    @property
    def is_web(self) -> bool:
        return self.ext in WEB_EXTS


@dataclass(frozen=True, slots=True)
class Entity:
    """A message entity. Offsets and lengths are UTF-16 code units, as the Bot API counts them."""

    type: str
    offset: int
    length: int
    url: str | None = None
    language: str | None = None
    custom_emoji_id: str | None = None
    unix_time: int | None = None
    date_time_format: str | None = None


@dataclass(frozen=True, slots=True)
class Parsed:
    """A message text after entity parsing: the plain text, its entities, and what was wrong."""

    plain: str
    entities: tuple[Entity, ...]
    problems: tuple[str, ...]


# --------------------------------------------------------------------------- reading a tree


def _blank(chars: list[str], start: int, end: int) -> None:
    for index in range(start, min(end, len(chars))):
        if chars[index] != "\n":
            chars[index] = " "


def _skip_quoted(text: str, start: int, quote: str) -> int:
    index = start + 1
    size = len(text)
    while index < size:
        char = text[index]
        if char == "\\":
            index += 2
            continue
        if char == quote:
            return index + 1
        if char == "\n" and quote != "`":
            return index
        index += 1
    return size


RUST_CHAR = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")
REGEX_PREFIX = set("(,=:[!&|?{};+-*%<>~^") | {"\n"}


def _skip_regex(text: str, start: int) -> int | None:
    """End of a JavaScript regex literal starting at `start`, or None when `/` is a division."""
    index = start + 1
    size = len(text)
    in_class = False
    while index < size:
        char = text[index]
        if char == "\n":
            return None
        if char == "\\":
            index += 2
            continue
        if char == "[":
            in_class = True
        elif char == "]":
            in_class = False
        elif char == "/" and not in_class:
            index += 1
            while index < size and text[index].isalpha():
                index += 1
            return index
        index += 1
    return None


def strip_c_like(
    text: str, *, rust: bool = False, js: bool = False, markup: bool = False
) -> str:
    """`text` with every `//`, `/* */` (and in markup, `<!-- -->`) comment blanked, strings kept.

    Positions and newlines are preserved, so a line number read from the result is the file's.
    A `//` right after `:` is a URL, not a comment.
    """
    chars = list(text)
    index = 0
    size = len(text)
    last_significant = "\n"
    while index < size:
        char = text[index]
        if markup and text.startswith("<!--", index):
            end = text.find("-->", index + 4)
            end = size if end < 0 else end + 3
            _blank(chars, index, end)
            index = end
            continue
        if char == "/" and index + 1 < size:
            following = text[index + 1]
            if following == "/" and (index == 0 or text[index - 1] != ":"):
                end = text.find("\n", index)
                end = size if end < 0 else end
                _blank(chars, index, end)
                index = end
                continue
            if following == "*":
                end = text.find("*/", index + 2)
                end = size if end < 0 else end + 2
                _blank(chars, index, end)
                index = end
                continue
            if js and last_significant in REGEX_PREFIX:
                end = _skip_regex(text, index)
                if end is not None:
                    index = end
                    last_significant = "/"
                    continue
        if char == '"' or (char == "`" and not rust):
            index = _skip_quoted(text, index, char)
            last_significant = char
            continue
        if char == "'":
            if rust:
                match = RUST_CHAR.match(text, index)
                index = match.end() if match else index + 1
                last_significant = "'"
                continue
            index = _skip_quoted(text, index, "'")
            last_significant = "'"
            continue
        if not char.isspace():
            last_significant = char
        elif char == "\n":
            last_significant = (
                "\n" if last_significant in REGEX_PREFIX else last_significant
            )
        index += 1
    return "".join(chars)


def strip_python(text: str) -> str:
    """`text` with every comment and docstring blanked, positions preserved."""
    starts = [0]
    for match in re.finditer("\n", text):
        starts.append(match.end())

    def offset(row_col: tuple[int, int]) -> int:
        row, col = row_col
        return starts[row - 1] + col if row - 1 < len(starts) else len(text)

    try:
        tokens = list(tokenize.generate_tokens(io.StringIO(text).readline))
    except (tokenize.TokenError, IndentationError, SyntaxError):
        return re.sub(r"(?m)#[^\n]*", lambda m: " " * len(m.group(0)), text)
    chars = list(text)
    quiet = {
        tokenize.NL,
        tokenize.COMMENT,
        tokenize.INDENT,
        tokenize.DEDENT,
        tokenize.ENCODING,
    }
    significant = [tok for tok in tokens if tok.type not in quiet]
    for tok in tokens:
        if tok.type == tokenize.COMMENT:
            _blank(chars, offset(tok.start), offset(tok.end))
    for index, tok in enumerate(significant):
        if tok.type != tokenize.STRING:
            continue
        before = significant[index - 1].type if index > 0 else tokenize.NEWLINE
        after = (
            significant[index + 1].type
            if index + 1 < len(significant)
            else tokenize.NEWLINE
        )
        if before == tokenize.NEWLINE and after in (
            tokenize.NEWLINE,
            tokenize.ENDMARKER,
        ):
            _blank(chars, offset(tok.start), offset(tok.end))
    return "".join(chars)


def strip_comments(text: str, ext: str) -> str:
    if ext == ".py":
        return strip_python(text)
    if ext == ".rs":
        return strip_c_like(text, rust=True)
    return strip_c_like(text, js=True, markup=ext in MARKUP_EXTS)


def _is_test_path(parts: tuple[str, ...]) -> bool:
    return any(part in TEST_DIRS for part in parts[:-1]) or bool(
        TEST_FILE.search(parts[-1])
    )


def walk(root: Path) -> Iterator[Path]:
    """Every file under `root` outside build output, in a stable order."""
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
        stack.sort(key=lambda path: str(path), reverse=True)


@dataclass(frozen=True, slots=True)
class Tree:
    root: Path
    sources: tuple[Source, ...]
    files_walked: int

    @property
    def bot(self) -> tuple[Source, ...]:
        return tuple(source for source in self.sources if source.is_bot)

    @property
    def web(self) -> tuple[Source, ...]:
        return tuple(source for source in self.sources if source.is_web)


def read_tree(root: Path) -> Tree:
    """The shipped source files under `root`: tests, build output and vendored copies left out."""
    own = Path(__file__).resolve()
    sources = []
    walked = 0
    manifests: list[str] = []
    for path in walk(root):
        walked += 1
        rel_parts = path.relative_to(root).parts
        ext = path.suffix.lower()
        if path.name in MANIFESTS and path.stat().st_size <= MAX_SOURCE_BYTES:
            try:
                manifests.append(path.read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError):
                pass
        if ext not in SOURCE_EXTS or _is_test_path(rel_parts):
            continue
        if (
            path.name in PROBE_FILES
            or VENDORED.search(path.name)
            or path.resolve() == own
        ):
            continue
        try:
            if path.stat().st_size > MAX_SOURCE_BYTES:
                continue
            raw = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        sources.append(Source("/".join(rel_parts), ext, raw, strip_comments(raw, ext)))
    library = any(BOT_LIBRARY.search(source.code) for source in sources) or any(
        BOT_LIBRARY.search(text) for text in manifests
    )
    if library:
        sources = [replace(source, bot_library=True) for source in sources]
    return Tree(root, tuple(sources), walked)


def method_matches(source: Source, method: str) -> list[re.Match[str]]:
    """Every place `source` names the Bot API `method`, in file order."""
    literal, call = METHOD_PATTERNS[method]
    found = list(literal.finditer(source.code))
    if source.bot_library:
        found += list(call.finditer(source.code))
    return sorted(found, key=lambda match: match.start())


def uses(source: Source, method: str) -> re.Match[str] | None:
    found = method_matches(source, method)
    return found[0] if found else None


def uses_bot_api(tree: Tree) -> bool:
    return any(uses(source, method) for source in tree.bot for method in BOT_METHODS)


def _clip(text: str, limit: int = 40) -> str:
    flat = text.replace("\n", "\\n")
    return flat if len(flat) <= limit else flat[: limit - 1] + "…"


def _outcome(findings: list[str], examined: int) -> Outcome:
    return Outcome(tuple(findings), examined)


# --------------------------------------------------------------------------- bot-api classes


def check_update_mode(tree: Tree) -> Outcome:
    bot = tree.bot
    hooks = [source for source in bot if uses(source, "setWebhook")]
    polls = [source for source in bot if uses(source, "getUpdates")]
    deletes = any(uses(source, "deleteWebhook") for source in bot)
    findings = []
    if hooks and polls and not deletes:
        findings.append(
            f"setWebhook ({hooks[0].rel}) and getUpdates ({polls[0].rel}) are both used and "
            "nothing calls deleteWebhook: getUpdates answers nothing while a webhook is set"
        )
    return _outcome(findings, len(tree.sources))


URL_ARG = re.compile(
    r"(?:\burl\s*[=:]\s*|[\"']url[\"']\s*:\s*|\.url\(\s*|setWebhook\(\s*)"
    r"(?:[fFrR]{0,2})([\"'`])((?:\\.|(?!\1).)*?)\1"
)
MAX_CONNECTIONS = re.compile(
    r"(?:\bmax_connections\b[\"']?\s*[=:]\s*|\.max_connections\(\s*|maxConnections\s*:\s*)(\d+)"
)
QUERY_URL = re.compile(r"setWebhook\?url=([^\"'`\s&]+)")


def _webhook_url_findings(source: Source, position: int, url: str) -> list[str]:
    where = f"{source.rel}:{source.line(position)}"
    findings = []
    scheme = url.split("://", 1)[0].lower() if "://" in url else ""
    if scheme and scheme != "https":
        findings.append(
            f"{where}: setWebhook URL {_clip(url)} is {scheme}; a webhook must be https"
        )
    match = re.match(r"^[a-zA-Z]+://[^/:{$]+:(\d+)", url)
    if match and int(match.group(1)) not in WEBHOOK_PORTS:
        findings.append(
            f"{where}: setWebhook port {match.group(1)} is unsupported; webhooks take 443, 80, 88 or 8443"
        )
    return findings


def check_webhook_endpoint(tree: Tree) -> Outcome:
    findings: list[str] = []
    for source in tree.bot:
        anchors = [match.start() for match in method_matches(source, "setWebhook")]
        anchors += [match.start() for match in WEBHOOK_PARAMS.finditer(source.code)]
        seen: set[int] = set()
        for anchor in sorted(anchors):
            window = source.code[anchor : anchor + 400]
            for match in URL_ARG.finditer(window):
                at = anchor + match.start()
                if at in seen:
                    continue
                seen.add(at)
                findings += _webhook_url_findings(source, at, match.group(2))
            for match in QUERY_URL.finditer(window):
                at = anchor + match.start()
                if at not in seen:
                    seen.add(at)
                    findings += _webhook_url_findings(source, at, match.group(1))
            for match in MAX_CONNECTIONS.finditer(window):
                at = anchor + match.start()
                value = int(match.group(1))
                if at not in seen and not 1 <= value <= 100:
                    seen.add(at)
                    findings.append(
                        f"{source.rel}:{source.line(at)}: max_connections {value} is outside 1-100"
                    )
    return _outcome(findings, len(tree.sources))


def check_poll_offset(tree: Tree) -> Outcome:
    findings = []
    for source in tree.bot:
        match = uses(source, "getUpdates")
        if not match:
            continue
        where = f"{source.rel}:{source.line(match.start())}"
        if not re.search(r"\boffset\b", source.code):
            findings.append(
                f"{where}: getUpdates is polled with no offset, so every unconfirmed update comes back"
            )
        elif not re.search(r"\bupdate_id\b|\bupdateId\b", source.code):
            findings.append(
                f"{where}: the getUpdates offset is never recomputed from update_id + 1"
            )
    return _outcome(findings, len(tree.sources))


TIMEOUT_LITERAL = re.compile(
    r"(?:\btimeout\b[\"']?\s*[=:]\s*|\.timeout\(\s*)(\d+)(?![\d.])"
)


def check_poll_long(tree: Tree) -> Outcome:
    findings = []
    for source in tree.bot:
        match = uses(source, "getUpdates")
        if not match:
            continue
        where = f"{source.rel}:{source.line(match.start())}"
        if not re.search(r"\btimeout\b", source.code):
            findings.append(
                f"{where}: getUpdates passes no timeout, so it defaults to 0: short polling, "
                "for testing only"
            )
            continue
        for literal in TIMEOUT_LITERAL.finditer(source.code):
            if int(literal.group(1)) == 0:
                findings.append(
                    f"{source.rel}:{source.line(literal.start())}: getUpdates timeout 0 is short "
                    "polling; pass a positive timeout"
                )
    return _outcome(findings, len(tree.sources))


ALLOWED_UPDATES = re.compile(
    r"\ballowed_updates\b|\ballowedUpdates\b|\bAllowedUpdate\b"
)


def check_allowed_updates(tree: Tree) -> Outcome:
    findings = []
    for source in tree.bot:
        match = uses(source, "getUpdates") or uses(source, "setWebhook")
        if match and not ALLOWED_UPDATES.search(source.code):
            findings.append(
                f"{source.rel}:{source.line(match.start())}: no allowed_updates is passed, so the "
                "previous setting persists and unwanted update types can arrive"
            )
    return _outcome(findings, len(tree.sources))


RETRY_AFTER = re.compile(
    r"\bretry_after\b|\bretryAfter\b|\bautoRetry\b|@grammyjs/auto-retry|\bAIORateLimiter\b"
)


def check_retry_after(tree: Tree) -> Outcome:
    findings = []
    if uses_bot_api(tree) and not any(
        RETRY_AFTER.search(source.code) for source in tree.bot
    ):
        findings.append(
            "the tree calls the Bot API and never reads retry_after: a 429 carries "
            "parameters.retry_after, the seconds to wait before the request may be repeated"
        )
    return _outcome(findings, len(tree.sources))


LEGACY_MARKDOWN = re.compile(
    r"\bParseMode::Markdown\b(?!V2)"
    r"|\bParseMode\.MARKDOWN\b(?!_V2)"
    r"|\b(?:parse_mode|parseMode)[\"']?\s*[:=]\s*[\"']Markdown[\"']"
)


def check_parse_mode(tree: Tree) -> Outcome:
    findings = []
    for source in tree.bot:
        for match in LEGACY_MARKDOWN.finditer(source.code):
            findings.append(
                f"{source.rel}:{source.line(match.start())}: parse mode Markdown is the legacy "
                "mode, kept only for backward compatibility; use MarkdownV2 or HTML"
            )
    return _outcome(findings, len(tree.sources))


MDV2_USED = re.compile(
    r"\bParseMode::MarkdownV2\b|\bMARKDOWN_V2\b|[\"']MarkdownV2[\"']"
)
HTML_USED = re.compile(
    r"\bParseMode::Html\b|\bParseMode\.HTML\b|\b(?:parse_mode|parseMode)[\"']?\s*[:=]\s*[\"']HTML[\"']"
)
MDV2_LIBRARY = re.compile(
    r"\bescape_markdown\s*\([^)]*\bversion\s*=\s*2"
    r"|\bmarkdown::escape\s*\("
    r"|(?i:\b(?:escape_?m(?:ark)?d(?:own)?_?v2|m(?:ark)?d(?:own)?_?v2_?escape)\b)"
)
HTML_LIBRARY = re.compile(
    r"\bhtml\.escape\s*\("
    r"|\bhtml_escape::encode_\w+"
    r"|\bv_htmlescape::escape\b|\baskama_escape\b|\bhtmlescape::encode_\w+"
    r"|\bhe\.(?:escape|encode)\s*\(|\b_\.escape\s*\(|[\"']escape-html[\"']|[\"']html-escaper[\"']"
    r"|(?i:\b(?:escape_?html|html_?escape)\s*\()"
)
STRING_LITERAL = re.compile(
    r"\"((?:\\.|[^\"\\\n])*)\"|'((?:\\.|[^'\\\n])*)'|`((?:\\.|[^`\\])*)`"
)
CHAR_ARRAY = re.compile(
    r"\[\s*(?:(?:'(?:\\.|[^'\\])'|\"(?:\\.|[^\"\\])\")\s*,?\s*){10,}\]"
)
REGEX_ARG = re.compile(
    r"\.(?:replace|replaceAll|split|match|test)\(\s*/((?:\\.|\[(?:\\.|[^\]\\\n])*\]|[^/\\\n])+)/[a-z]*"
)


def _unescape(body: str) -> str:
    return re.sub(r"\\(.)", r"\1", body)


def _mdv2_escaper_sets(source: Source) -> list[tuple[int, set[str]]]:
    """Every literal in `source` that holds ten or more MarkdownV2 specials: an escaper's set."""
    found = []
    for pattern in (STRING_LITERAL, CHAR_ARRAY, REGEX_ARG):
        for match in pattern.finditer(source.code):
            body = next(
                (group for group in match.groups() if group is not None), match.group(0)
            )
            chars = set(_unescape(body))
            if len(chars & set(MARKDOWN_V2_SPECIALS)) >= 10:
                found.append((match.start(), chars))
    return found


AMP_REPLACE = re.compile(
    r"\.replace(?:All)?\(\s*(?:([\"'])&\1|/&/g?)\s*,\s*([\"'])&amp;\2"
)
LT_REPLACE = re.compile(
    r"\.replace(?:All)?\(\s*(?:([\"'])<\1|/</g?)\s*,\s*([\"'])&lt;\2"
)
GT_REPLACE = re.compile(
    r"\.replace(?:All)?\(\s*(?:([\"'])>\1|/>/g?)\s*,\s*([\"'])&gt;\2"
)
CHAIN_REACH = 300


def _html_chain_findings(source: Source) -> tuple[list[str], bool]:
    """Findings for hand-rolled HTML escape chains in `source`, and whether a sound one exists."""
    amps = [match.start() for match in AMP_REPLACE.finditer(source.code)]
    lts = [match.start() for match in LT_REPLACE.finditer(source.code)]
    gts = [match.start() for match in GT_REPLACE.finditer(source.code)]
    findings = []
    sound = False
    for amp in amps:
        near_lt = [lt for lt in lts if abs(lt - amp) <= CHAIN_REACH]
        near_gt = [gt for gt in gts if abs(gt - amp) <= CHAIN_REACH]
        where = f"{source.rel}:{source.line(amp)}"
        if not near_lt or not near_gt:
            findings.append(
                f"{where}: this HTML escape chain replaces '&' but not both '<' and '>'"
            )
        elif min(near_lt) < amp or min(near_gt) < amp:
            findings.append(
                f"{where}: this chain replaces '&' with &amp; after '<' or '>', so &lt; becomes "
                "&amp;lt;: replace '&' first"
            )
        else:
            sound = True
    for position in lts + gts:
        if not any(abs(position - amp) <= CHAIN_REACH for amp in amps):
            findings.append(
                f"{source.rel}:{source.line(position)}: this HTML escape chain never replaces '&'"
            )
    return findings, sound


def check_escape(tree: Tree) -> Outcome:
    findings: list[str] = []
    bot = tree.bot
    mdv2 = next(((s, m) for s in bot if (m := MDV2_USED.search(s.code))), None)
    html_used = next(((s, m) for s in bot if (m := HTML_USED.search(s.code))), None)
    if mdv2:
        complete = any(MDV2_LIBRARY.search(source.code) for source in bot)
        needed = set(MARKDOWN_V2_SPECIALS) | {"\\"}
        for source in bot:
            for position, chars in _mdv2_escaper_sets(source):
                missing = [
                    char
                    for char in "\\" + MARKDOWN_V2_SPECIALS
                    if char in needed - chars
                ]
                if missing:
                    findings.append(
                        f"{source.rel}:{source.line(position)}: this MarkdownV2 escaper misses "
                        + ", ".join(repr(char) for char in missing)
                    )
                else:
                    complete = True
        if not complete and not findings:
            source, match = mdv2
            findings.append(
                f"{source.rel}:{source.line(match.start())}: MarkdownV2 is used and no escaper "
                f"for its 18 special characters and '\\' is defined or imported"
            )
    if html_used:
        sound = any(HTML_LIBRARY.search(source.code) for source in bot)
        for source in bot:
            chain_findings, chain_sound = _html_chain_findings(source)
            findings += chain_findings
            sound = sound or chain_sound
        if not sound and not any(
            "HTML escape chain" in f or "&amp;lt;" in f for f in findings
        ):
            source, match = html_used
            findings.append(
                f"{source.rel}:{source.line(match.start())}: HTML parse mode is used and no "
                "escaper for '<', '>' and '&' is defined or imported"
            )
    return _outcome(findings, len(tree.sources))


LIMIT_CONSTANT = re.compile(
    r"\b([A-Za-z_][A-Za-z0-9_]*)\s*(?::\s*[A-Za-z0-9_<>\[\]&' ]+?)?\s*=\s*(\d{2,6})\b(?!\s*[.*/+-])"
)
TEXT_NAME = re.compile(r"(?i)(text|message|msg|chunk)")
CAPTION_NAME = re.compile(r"(?i)caption")
LENGTH_WORD = re.compile(
    r"(?i)(len|length|limit|max|chars|characters|size|units|utf16)"
)
NOT_A_LENGTH = re.compile(
    r"(?i)(ttl|seconds?|secs|minutes?|hours?|days?|per_|_per|queue|count|retr|timeout|delay"
    r"|interval|_ms\b|millis)"
)


def check_length_bound(tree: Tree) -> Outcome:
    findings = []
    text_bound = False
    caption_bound = False
    for source in tree.bot:
        for match in LIMIT_CONSTANT.finditer(source.code):
            name, value = match.group(1), int(match.group(2))
            where = f"{source.rel}:{source.line(match.start())}"
            if not LENGTH_WORD.search(name) or NOT_A_LENGTH.search(name):
                continue
            if CAPTION_NAME.search(name):
                if value > CAPTION_MAX:
                    findings.append(
                        f"{where}: {name} = {value} exceeds the {CAPTION_MAX}-character caption limit"
                    )
                else:
                    caption_bound = True
            elif TEXT_NAME.search(name) and value >= 100:
                if value > TEXT_MAX:
                    findings.append(
                        f"{where}: {name} = {value} exceeds the {TEXT_MAX}-character message limit"
                    )
                else:
                    text_bound = True
        if re.search(rf"\b{TEXT_MAX}\b", source.code):
            text_bound = True
        for line in source.code.splitlines():
            if re.search(r"(?i)caption", line) and any(
                1 <= int(number) <= CAPTION_MAX
                for number in re.findall(r"\b(\d{2,4})\b", line)
            ):
                caption_bound = True
    sends_text = any(
        uses(source, method) for source in tree.bot for method in TEXT_METHODS
    )
    sends_caption = any(
        uses(source, method) for source in tree.bot for method in CAPTION_METHODS
    )
    if (
        sends_text
        and not text_bound
        and not any("message limit" in f for f in findings)
    ):
        findings.append(
            f"messages are sent and no length bound of at most {TEXT_MAX} characters is declared: "
            "a longer text is refused with 400, so chunk or bound it"
        )
    if (
        sends_caption
        and not caption_bound
        and not any("caption limit" in f for f in findings)
    ):
        findings.append(
            f"captions can be sent and no bound of at most {CAPTION_MAX} characters is declared"
        )
    return _outcome(findings, len(tree.sources))


CALLBACK_HANDLED = re.compile(
    r"\bcallback_query\b|\bCallbackQuery\b|\bcallbackQuery\b|\bon_callback\w*|\bcallback_query_handler\b"
)
CALLBACK_ANSWERED = re.compile(r"\banswerCbQuery\b")
CALLBACK_BUTTONS = re.compile(r"\bcallback_data\b")


def check_callback_answer(tree: Tree) -> Outcome:
    findings = []
    bot = tree.bot
    handler = next(
        ((s, m) for s in bot if (m := CALLBACK_HANDLED.search(s.code))), None
    )
    answered = any(
        uses(source, "answerCallbackQuery") or CALLBACK_ANSWERED.search(source.code)
        for source in bot
    )
    buttons = next(
        ((s, m) for s in bot if (m := CALLBACK_BUTTONS.search(s.code))), None
    )
    if handler and not answered:
        source, match = handler
        findings.append(
            f"{source.rel}:{source.line(match.start())}: callback_query updates are handled and "
            "answerCallbackQuery is never called, so the client shows a progress bar until it is"
        )
    if buttons and not handler:
        source, match = buttons
        findings.append(
            f"{source.rel}:{source.line(match.start())}: callback buttons are sent and no update "
            "handler reads callback_query"
        )
    return _outcome(findings, len(tree.sources))


CALLBACK_LITERAL = re.compile(
    r"\bcallback_data\b[\"']?\s*(?:[:=]\s*|\(\s*)([fFrRbB]{0,2})([\"'`])((?:\\.|(?!\2).)*?)\2"
)


def _literal_value(body: str) -> str:
    try:
        return json.loads('"' + body.replace('"', '\\"') + '"')
    except ValueError:
        return _unescape(body)


def check_callback_data(tree: Tree) -> Outcome:
    findings = []
    for source in tree.bot:
        for match in CALLBACK_LITERAL.finditer(source.code):
            prefix, quote, body = match.group(1), match.group(2), match.group(3)
            where = f"{source.rel}:{source.line(match.start())}"
            templated = "f" in prefix.lower() or (quote == "`" and "${" in body)
            if templated:
                fixed = re.sub(r"\$?\{[^}]*\}", "", body)
                size = len(_literal_value(fixed).encode("utf-8"))
                if size > CALLBACK_DATA_MAX_BYTES:
                    findings.append(
                        f"{where}: callback_data template is at least {size} bytes before its "
                        f"fields are filled; the Bot API takes 1-{CALLBACK_DATA_MAX_BYTES}"
                    )
                continue
            size = len(_literal_value(body).encode("utf-8"))
            if not 1 <= size <= CALLBACK_DATA_MAX_BYTES:
                findings.append(
                    f"{where}: callback_data {_clip(body)!r} is {size} bytes; the Bot API takes "
                    f"1-{CALLBACK_DATA_MAX_BYTES}"
                )
    return _outcome(findings, len(tree.sources))


DEEP_LINK = re.compile(
    r"(?:https?://)?(?:t\.me|telegram\.me)/[A-Za-z0-9_]+(?:/[A-Za-z0-9_]+)?\?([^\"'`\s<>)]*)"
    r"|tg://resolve\?([^\"'`\s<>)]*)"
)
PARAM_LIMITS = {
    "start": START_PARAM_MAX,
    "startgroup": START_PARAM_MAX,
    "startchannel": START_PARAM_MAX,
    "startapp": STARTAPP_PARAM_MAX,
    "startattach": STARTAPP_PARAM_MAX,
}


def check_deep_link(tree: Tree) -> Outcome:
    """Deep links in Rust and Python bot source. web-launch's tg-deep-links and tg-startapp judge
    the Mini App's pages and scripts, so the web side is theirs and is not read twice."""
    findings = []
    for source in tree.sources:
        if source.ext not in BOT_ONLY_EXTS:
            continue
        for match in DEEP_LINK.finditer(source.code):
            query = match.group(1) if match.group(1) is not None else match.group(2)
            where = f"{source.rel}:{source.line(match.start())}"
            for pair in query.split("&"):
                name, _, value = pair.partition("=")
                if name not in PARAM_LIMITS:
                    continue
                fixed = re.split(r"\$\{|\{|%[sd]", value, maxsplit=1)[0]
                bad = sorted(
                    {char for char in fixed if not re.match(r"[A-Za-z0-9_-]", char)}
                )
                if bad:
                    findings.append(
                        f"{where}: the {name} parameter holds "
                        + ", ".join(repr(char) for char in bad)
                        + ", outside A-Z a-z 0-9 _ - (base64url-encode anything else)"
                    )
                if len(fixed) > PARAM_LIMITS[name]:
                    findings.append(
                        f"{where}: the {name} parameter is {len(fixed)} characters; it takes at "
                        f"most {PARAM_LIMITS[name]}"
                    )
    return _outcome(findings, len(tree.sources))


def check_replaced_fields(tree: Tree) -> Outcome:
    findings = []
    if uses_bot_api(tree):
        for source in tree.bot:
            for old, new, version in REPLACED_FIELDS:
                for match in re.finditer(rf"\b{old}\b", source.code):
                    findings.append(
                        f"{source.rel}:{source.line(match.start())}: {old} was replaced by {new} in "
                        f"Bot API {version}"
                    )
    return _outcome(findings, len(tree.sources))


# --------------------------------------------------------------------------- mini-app classes

WEBAPP_ALIAS = re.compile(
    r"\b(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::\s*[^=]+?)?=\s*"
    r"(?:window\s*\.\s*|globalThis\s*\.\s*|self\s*\.\s*)?Telegram\s*(?:\?\.|\.)\s*WebApp\b(?!\s*\.)"
)
WEBAPP_DESTRUCTURE = re.compile(
    r"\b(?:const|let|var)\s*\{([^}]*)\}\s*=\s*(?:window\s*\.\s*|globalThis\s*\.\s*)?"
    r"Telegram\s*(?:\?\.|\.)\s*WebApp\b"
)


def webapp_aliases(source: Source) -> set[str]:
    names = {match.group(1) for match in WEBAPP_ALIAS.finditer(source.code)}
    return names


VERSION_GATE = re.compile(
    r"(!\s*)?(?:[\w$]+\s*(?:\?\.|\.)\s*)*isVersionAtLeast\s*\(\s*(?:([\"'])(\d+(?:\.\d+)?)\2|([A-Za-z_$][\w$]*))\s*\)"
)
AVAILABILITY = re.compile(r"\.isAvailable\s*\(|\.ifAvailable\s*\(|\bisSupported\s*\(")
GATE_LINES = 3


def _version(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def _gates(source: Source) -> tuple[tuple[int, ...], list[tuple[int, tuple[int, ...]]]]:
    """The file's floor (its negated, early-exit gates) and its local gates, by line."""
    constants = {
        match.group(1): match.group(3)
        for match in re.finditer(
            r"\b(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::\s*\w+\s*)?=\s*([\"'])(\d+(?:\.\d+)?)\2",
            source.code,
        )
    }
    floor: tuple[int, ...] = (0,)
    local = []
    for match in VERSION_GATE.finditer(source.code):
        text = match.group(3) or constants.get(match.group(4) or "")
        if text is None:
            continue
        version = _version(text)
        if match.group(1):
            floor = max(floor, version)
        else:
            local.append((source.line(match.start()), version))
    return floor, local


def check_version_gate(tree: Tree) -> Outcome:
    findings = []
    for source in tree.web:
        receivers = {"WebApp"} | webapp_aliases(source)
        direct = set()
        for match in WEBAPP_DESTRUCTURE.finditer(source.code):
            direct |= {
                name.strip().split(":")[0].strip() for name in match.group(1).split(",")
            }
        names = "|".join(re.escape(name) for name in sorted(receivers))
        uses_found = [
            (match.start(), match.group(1))
            for match in re.finditer(
                rf"(?<![\w$])(?:{names})\s*(?:\?\.|\.)\s*([A-Za-z_]\w*)", source.code
            )
            if match.group(1) in THROWING_METHODS
        ]
        uses_found += [
            (match.start(), match.group(1))
            for match in re.finditer(
                r"(?<![\w$.])([A-Za-z_]\w*)\s*(?:\?\.|\.|\()", source.code
            )
            if match.group(1) in direct and match.group(1) in THROWING_METHODS
        ]
        if not uses_found:
            continue
        floor, local = _gates(source)
        lines = source.code.splitlines()
        for position, method in sorted(set(uses_found)):
            needed = _version(THROWING_METHODS[method])
            line = source.line(position)
            window = "\n".join(lines[max(0, line - 1 - GATE_LINES) : line])
            if AVAILABILITY.search(window):
                continue
            guard = max(
                (
                    version
                    for gate_line, version in local
                    if line - GATE_LINES <= gate_line <= line
                ),
                default=(0,),
            )
            if max(floor, guard) < needed:
                findings.append(
                    f"{source.rel}:{line}: WebApp.{method} needs Bot API {THROWING_METHODS[method]} "
                    f"and nothing gates it with isVersionAtLeast('{THROWING_METHODS[method]}'): the "
                    "official script throws WebAppMethodUnsupported on an older client"
                )
    return _outcome(findings, len(tree.sources))


STORAGE_CALL = re.compile(
    r"\b(CloudStorage|DeviceStorage|SecureStorage)\s*(?:\?\.|\.)\s*"
    r"(setItem|getItem|removeItem|getItems|removeItems)\s*\(\s*"
)
LITERAL_AT = re.compile(r"([\"'`])((?:\\.|(?!\1).)*?)\1")
SENSITIVE_KEY = re.compile(
    r"(?i)(token|secret|passw|credential|private_?key|api_?key|session)"
)


def _storage_keys(code: str, start: int, method: str) -> tuple[list[str], str | None]:
    """The literal keys a storage call names, and its literal value when it sets one."""
    rest = code[start:]
    keys: list[str] = []
    value = None
    if method in ("getItems", "removeItems"):
        array = re.match(r"\[([^\]]*)\]", rest)
        if array:
            keys = [
                _literal_value(m.group(2)) for m in LITERAL_AT.finditer(array.group(1))
            ]
        return keys, None
    first = LITERAL_AT.match(rest)
    if not first:
        return keys, None
    keys = [_literal_value(first.group(2))]
    if method == "setItem":
        second = re.match(r"\s*,\s*", rest[first.end() :])
        if second:
            literal = LITERAL_AT.match(rest[first.end() + second.end() :])
            if literal:
                value = _literal_value(literal.group(2))
    return keys, value


def check_storage(tree: Tree) -> Outcome:
    findings = []
    secure_keys: dict[str, str] = {}
    for source in tree.web:
        for match in STORAGE_CALL.finditer(source.code):
            store, method = match.group(1), match.group(2)
            keys, value = _storage_keys(source.code, match.end(), method)
            where = f"{source.rel}:{source.line(match.start())}"
            for key in keys:
                if store == "SecureStorage":
                    secure_keys.setdefault(key, where)
                if (
                    method == "setItem"
                    and store != "SecureStorage"
                    and SENSITIVE_KEY.search(key)
                ):
                    findings.append(
                        f"{where}: key {_clip(key)!r} holds sensitive data in {store}; SecureStorage "
                        "is the store for sensitive data"
                    )
            if (
                store == "CloudStorage"
                and value is not None
                and len(value) > CLOUD_VALUE_MAX
            ):
                findings.append(
                    f"{where}: a CloudStorage value is {len(value)} characters; it takes 0-{CLOUD_VALUE_MAX}"
                )
    if len(secure_keys) > SECURE_STORAGE_ITEMS:
        findings.append(
            f"SecureStorage holds at most {SECURE_STORAGE_ITEMS} items per user and the tree names "
            f"{len(secure_keys)} keys"
        )
    return _outcome(findings, len(tree.sources))


RAW_DATA_SEND = re.compile(r"[\"']web_app_data_send[\"']")
REPLY_KEYBOARD_WEB_APP = re.compile(
    r"[\"']keyboard[\"']\s*:[\s\S]{0,600}?[\"']?web_app[\"']?\s*:"
    r"|\bKeyboardButton\b[\s\S]{0,300}?\bweb_app\b"
    r"|\bReplyKeyboardMarkup\b[\s\S]{0,600}?\bweb_app\b"
    r"|\bnew\s+Keyboard\s*\(\s*\)[\s\S]{0,300}?\.webApp\s*\("
)


def _send_data(source: Source) -> re.Match[str] | None:
    """`sendData` called on `WebApp` or an alias of it, or the raw bridge's data send."""
    receivers = "|".join(
        re.escape(name) for name in sorted({"WebApp"} | webapp_aliases(source))
    )
    return re.search(
        rf"(?<![\w$])(?:{receivers})\s*(?:\?\.|\.)\s*sendData\s*\(", source.code
    ) or RAW_DATA_SEND.search(source.code)


def check_send_data(tree: Tree) -> Outcome:
    findings = []
    sender = next(((s, m) for s in tree.web if (m := _send_data(s))), None)
    if sender and tree.bot:
        launched = any(
            REPLY_KEYBOARD_WEB_APP.search(source.code) for source in tree.bot
        )
        if not launched:
            source, match = sender
            findings.append(
                f"{source.rel}:{source.line(match.start())}: sendData reaches the bot only from a Mini "
                "App launched by a reply-keyboard web_app button, and no bot code offers that keyboard"
            )
    return _outcome(findings, len(tree.sources))


# --------------------------------------------------------------------------- the message API


def utf16_len(text: str) -> int:
    """`text`'s length in UTF-16 code units, the unit the Bot API measures entities in."""
    return len(text.encode("utf-16-le")) // 2


HTML_TAGS = {
    "b": "bold",
    "strong": "bold",
    "i": "italic",
    "em": "italic",
    "u": "underline",
    "ins": "underline",
    "s": "strikethrough",
    "strike": "strikethrough",
    "del": "strikethrough",
    "tg-spoiler": "spoiler",
    "span": "spoiler",
    "a": "text_link",
    "tg-emoji": "custom_emoji",
    "tg-time": "date_time",
    "code": "code",
    "pre": "pre",
    "blockquote": "blockquote",
}
ENTITY_TYPES = frozenset(
    {
        "mention",
        "hashtag",
        "cashtag",
        "bot_command",
        "url",
        "email",
        "phone_number",
        "bold",
        "italic",
        "underline",
        "strikethrough",
        "spoiler",
        "blockquote",
        "expandable_blockquote",
        "code",
        "pre",
        "text_link",
        "text_mention",
        "custom_emoji",
        "date_time",
    }
)
FORMATTING = frozenset({"bold", "italic", "underline", "strikethrough", "spoiler"})
QUOTES = frozenset({"blockquote", "expandable_blockquote"})
CODE = frozenset({"code", "pre"})
TAG = re.compile(
    r"<(/?)([a-zA-Z][a-zA-Z0-9-]*)((?:\s+[a-zA-Z][a-zA-Z0-9-]*(?:\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s>\"']+))?)*)\s*>"
)
ATTRIBUTE = re.compile(
    r"([a-zA-Z][a-zA-Z0-9-]*)(?:\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s>\"']+)))?"
)
HTML_ENTITY = re.compile(r"&(#[0-9]+|#[xX][0-9a-fA-F]+|[a-zA-Z][a-zA-Z0-9]*);")


# The schemes a text link keeps. tdlib's LinkManager::check_link, which the Bot API server runs on
# every link, accepts http, https, tg, ton and tonsite; parse_html and parse_markdown_v2 then DROP a
# link that fails it and keep only its text. Any other scheme (javascript:, data:, mailto:, tel:) is
# a link that silently vanishes, so it is refused here rather than sent.
LINK_SCHEMES = frozenset({"http", "https", "tg", "ton", "tonsite"})
LINK_SCHEME = re.compile(r"^([A-Za-z][A-Za-z0-9+-]*):")


def link_problem(url: str) -> str | None:
    """Why Telegram would drop a text link to `url`, or None when it keeps it."""
    match = LINK_SCHEME.match(url)
    if match:
        scheme = match.group(1).lower()
        if scheme in LINK_SCHEMES:
            return None
        return (
            f"uses the {scheme}: scheme, and Telegram keeps only http, https and tg links: it "
            "drops this one and keeps the text"
        )
    host = url.split("/", 1)[0]
    if not url or any(char.isspace() for char in url) or "." not in host:
        return "is not a URL, so Telegram drops the link and keeps the text"
    return None


def _nesting_problems(entities: list[Entity]) -> list[str]:
    """The Bot API's nesting rules over every pair of entities that share a character.

    Two entities that share characters must nest. Formatting (bold, italic, underline,
    strikethrough, spoiler) may contain and sit inside anything but code and pre; a blockquote is
    never nested in a blockquote; every other pair may not contain each other. An empty entity,
    MarkdownV2's separator, shares nothing. The rules are symmetric, so no order is assumed.
    """
    problems = []
    for first_index, first in enumerate(entities):
        for second in entities[first_index + 1 :]:
            first_end = first.offset + first.length
            second_end = second.offset + second.length
            if not first.length or not second.length:
                continue
            if not (first.offset < second_end and second.offset < first_end):
                continue
            first_inside = second.offset <= first.offset and first_end <= second_end
            second_inside = first.offset <= second.offset and second_end <= first_end
            if not first_inside and not second_inside:
                problems.append(
                    f"entities at offsets {first.offset} and {second.offset} overlap without "
                    "one containing the other"
                )
                continue
            outer, inner = (first, second) if second_inside else (second, first)
            kinds = {outer.type, inner.type}
            if outer.type in QUOTES and inner.type in QUOTES:
                problems.append("a blockquote cannot be nested in a blockquote")
            elif kinds & CODE and not kinds & QUOTES:
                code, other = (outer, inner) if outer.type in CODE else (inner, outer)
                problems.append(f"a {code.type} entity cannot contain {other.type}")
            elif not kinds & (FORMATTING | QUOTES):
                problems.append(f"a {outer.type} entity cannot contain {inner.type}")
    return list(dict.fromkeys(problems))


def _parse_html(text: str) -> Parsed:
    plain: list[str] = []
    units = 0
    problems: list[str] = []
    entities: list[Entity] = []
    stack: list[tuple[str, str, int, dict[str, str]]] = []
    index = 0
    size = len(text)
    while index < size:
        char = text[index]
        if char == "<":
            match = TAG.match(text, index)
            if not match:
                problems.append(f"an unescaped '<' at offset {index}; write &lt;")
                plain.append(char)
                units += 1
                index += 1
                continue
            closing, name, attribute_text = (
                match.group(1) == "/",
                match.group(2).lower(),
                match.group(3),
            )
            index = match.end()
            if name not in HTML_TAGS:
                problems.append(f"unsupported tag <{name}>")
                continue
            attributes = {
                m.group(1).lower(): next(
                    (g for g in m.groups()[1:] if g is not None), ""
                )
                for m in ATTRIBUTE.finditer(attribute_text)
            }
            if not closing:
                if name == "span" and attributes.get("class") != "tg-spoiler":
                    problems.append('a <span> must carry class="tg-spoiler"')
                if name == "a":
                    attributes["__plain"] = str(len(plain))
                if name == "tg-emoji" and "emoji-id" not in attributes:
                    problems.append("a <tg-emoji> needs an emoji-id")
                if name == "tg-time":
                    if not attributes.get("unix", "").isdigit():
                        problems.append("a <tg-time> needs a unix time")
                    fmt = attributes.get("format")
                    if fmt is not None and not DATE_TIME_FORMAT.match(fmt):
                        problems.append(
                            f"tg-time format {fmt!r} does not match r|w?[dD]?[tT]?"
                        )
                if name == "code" and "class" in attributes:
                    if not (stack and stack[-1][0] == "pre"):
                        problems.append(
                            "a language class on <code> is allowed only inside <pre>"
                        )
                stack.append((name, HTML_TAGS[name], units, attributes))
                continue
            if not stack:
                problems.append(f"</{name}> closes nothing")
                continue
            if stack[-1][0] != name:
                problems.append(f"</{name}> closes <{stack[-1][0]}>")
                if not any(entry[0] == name for entry in stack):
                    continue
                while stack[-1][0] != name:
                    stack.pop()
            open_name, kind, start, attrs = stack.pop()
            if (
                open_name == "code"
                and stack
                and stack[-1][0] == "pre"
                and "class" in attrs
            ):
                pre = stack.pop()
                language = attrs["class"].removeprefix("language-")
                stack.append((pre[0], pre[1], pre[2], {**pre[3], "language": language}))
                continue
            if kind == "blockquote" and "expandable" in attrs:
                kind = "expandable_blockquote"
            url = attrs.get("href")
            if open_name == "a":
                link_text = "".join(plain[int(attrs["__plain"]) :])
                url = link_text if url is None else url
                problem = link_problem(url)
                if problem and "href" in attrs:
                    problems.append(f"<a href={url!r}> {problem}")
                elif problem:
                    problems.append(
                        f"an <a> with no href takes its text as the URL, and {url!r} {problem}"
                    )
            unix = attrs.get("unix")
            entities.append(
                Entity(
                    kind,
                    start,
                    units - start,
                    url=url,
                    language=attrs.get("language"),
                    custom_emoji_id=attrs.get("emoji-id"),
                    unix_time=int(unix) if unix and unix.isdigit() else None,
                    date_time_format=attrs.get("format"),
                )
            )
            continue
        if char == ">":
            problems.append(f"an unescaped '>' at offset {index}; write &gt;")
        if char == "&":
            match = HTML_ENTITY.match(text, index)
            if not match:
                problems.append(f"an unescaped '&' at offset {index}; write &amp;")
            else:
                body = match.group(1)
                index = match.end()
                if body.startswith("#"):
                    number = (
                        int(body[2:], 16) if body[1:2] in ("x", "X") else int(body[1:])
                    )
                    decoded = chr(number) if 0 < number <= 0x10FFFF else "\ufffd"
                elif body in HTML_NAMED_ENTITIES:
                    decoded = HTML_NAMED_ENTITIES[body]
                else:
                    problems.append(
                        f"&{body}; is not a supported named entity (&lt; &gt; &amp; &quot;)"
                    )
                    decoded = ""
                plain.append(decoded)
                units += utf16_len(decoded)
                continue
        plain.append(char)
        units += utf16_len(char)
        index += 1
    for name, _kind, _start, _attrs in reversed(stack):
        problems.append(f"unclosed <{name}> at the end of the text")
    problems += _nesting_problems(entities)
    return Parsed(
        "".join(plain),
        tuple(sorted(entities, key=lambda e: (e.offset, -e.length))),
        tuple(problems),
    )


MDV2_TOGGLES = {
    "*": "bold",
    "_": "italic",
    "__": "underline",
    "~": "strikethrough",
    "||": "spoiler",
}


def _parse_markdown_v2(text: str) -> Parsed:  # noqa: C901 -- one scanner, one pass
    plain: list[str] = []
    units = 0
    problems: list[str] = []
    entities: list[Entity] = []
    stack: list[tuple[str, str, int]] = []
    links: list[tuple[str, int]] = []
    quote: tuple[str, int] | None = None
    index = 0
    size = len(text)

    def emit(char: str) -> None:
        nonlocal units
        plain.append(char)
        units += utf16_len(char)

    def close_quote() -> None:
        nonlocal quote
        if quote is not None:
            end = units - 1 if plain and plain[-1] == "\n" else units
            entities.append(Entity(quote[0], quote[1], max(0, end - quote[1])))
            quote = None

    def toggle(marker: str) -> None:
        kind = MDV2_TOGGLES[marker]
        position = next(
            (i for i in range(len(stack) - 1, -1, -1) if stack[i][0] == marker), None
        )
        if position is None:
            stack.append((marker, kind, units))
            return
        if position != len(stack) - 1:
            problems.append(
                f"'{marker}' closes across an open '{stack[-1][0]}': entities overlap"
            )
        _marker, kind, start = stack.pop(position)
        entities.append(Entity(kind, start, units - start))

    while index < size:
        char = text[index]
        at_line_start = index == 0 or text[index - 1] == "\n"
        if at_line_start:
            if text.startswith("**>", index):
                close_quote()
                quote = ("expandable_blockquote", units)
                index += 3
                continue
            if char == ">":
                if quote is None:
                    quote = ("blockquote", units)
                index += 1
                continue
            if quote is not None:
                close_quote()
        if char == "\\":
            if index + 1 >= size:
                problems.append("a trailing '\\' escapes nothing")
                index += 1
                continue
            following = text[index + 1]
            if not 1 <= ord(following) <= 126:
                problems.append(
                    f"'\\' before {following!r} escapes nothing; only codes 1-126 escape"
                )
            emit(following)
            index += 2
            continue
        if text.startswith("```", index):
            end = index + 3
            newline = text.find("\n", end)
            language = text[end:newline].strip() if newline >= 0 else ""
            body_start = (
                newline + 1
                if newline >= 0 and language and " " not in language
                else end
            )
            if not language or " " in language:
                language = ""
            close = body_start
            start_units = units
            closed = False
            while close < size:
                if text[close] == "\\" and close + 1 < size:
                    emit(text[close + 1])
                    close += 2
                    continue
                if text.startswith("```", close):
                    closed = True
                    break
                if text[close] == "`":
                    problems.append(f"an unescaped '`' inside pre at offset {close}")
                emit(text[close])
                close += 1
            if not closed:
                problems.append("unclosed '```' pre block")
            entities.append(
                Entity(
                    "pre", start_units, units - start_units, language=language or None
                )
            )
            index = close + 3
            continue
        if char == "`":
            close = index + 1
            start_units = units
            closed = False
            while close < size:
                if text[close] == "\\" and close + 1 < size:
                    emit(text[close + 1])
                    close += 2
                    continue
                if text[close] == "`":
                    closed = True
                    break
                emit(text[close])
                close += 1
            if not closed:
                problems.append("unclosed '`' inline code")
            entities.append(Entity("code", start_units, units - start_units))
            index = close + 1
            continue
        if char == "_" and text.startswith("__", index):
            toggle("__")
            index += 2
            continue
        if char in "*_~":
            toggle(char)
            index += 1
            continue
        if char == "|":
            if text.startswith("||", index):
                if (
                    quote is not None
                    and quote[0] == "expandable_blockquote"
                    and (index + 2 >= size or text[index + 2] == "\n")
                    and not any(entry[0] == "||" for entry in stack)
                ):
                    index += 2
                    close_quote()
                    continue
                toggle("||")
                index += 2
                continue
            problems.append(f"an unescaped '|' at offset {index}")
            emit(char)
            index += 1
            continue
        if char == "!" and text.startswith("![", index):
            links.append(("![", units))
            index += 2
            continue
        if char == "[":
            links.append(("[", units))
            index += 1
            continue
        if char == "]" and links:
            opener, start = links.pop()
            if not text.startswith("(", index + 1):
                problems.append(
                    f"a link's ']' at offset {index} is not followed by '(' and a URL"
                )
                index += 1
                continue
            close = index + 2
            url_chars = []
            closed = False
            while close < size:
                if text[close] == "\\" and close + 1 < size:
                    url_chars.append(text[close + 1])
                    close += 2
                    continue
                if text[close] == ")":
                    closed = True
                    break
                url_chars.append(text[close])
                close += 1
            if not closed:
                problems.append("a link's URL is never closed with ')'")
                index = size
                continue
            url = "".join(url_chars)
            index = close + 1
            if opener == "![":
                if url.startswith("tg://time?"):
                    params = dict(
                        pair.partition("=")[::2]
                        for pair in url[len("tg://time?") :].split("&")
                    )
                    fmt = params.get("format")
                    if fmt is not None and not DATE_TIME_FORMAT.match(fmt):
                        problems.append(
                            f"date_time format {fmt!r} does not match r|w?[dD]?[tT]?"
                        )
                    unix = params.get("unix", "")
                    if not unix.isdigit():
                        problems.append("a date_time link needs a unix time")
                    entities.append(
                        Entity(
                            "date_time",
                            start,
                            units - start,
                            unix_time=int(unix) if unix.isdigit() else None,
                            date_time_format=fmt,
                        )
                    )
                elif url.startswith("tg://emoji?id="):
                    entities.append(
                        Entity(
                            "custom_emoji",
                            start,
                            units - start,
                            custom_emoji_id=url[len("tg://emoji?id=") :],
                        )
                    )
                else:
                    problems.append(
                        f"'![' introduces a custom emoji or a date_time, not {url[:30]!r}"
                    )
            else:
                problem = link_problem(url)
                if problem:
                    problems.append(f"the link to {url[:40]!r} {problem}")
                entities.append(Entity("text_link", start, units - start, url=url))
            continue
        if char in MARKDOWN_V2_SPECIALS:
            problems.append(
                f"an unescaped {char!r} at offset {index}; MarkdownV2 needs '\\{char}'"
            )
        emit(char)
        index += 1
    close_quote()
    for marker, _kind, _start in stack:
        problems.append(f"unclosed {marker!r} entity")
    for opener, _start in links:
        problems.append(f"unclosed {opener!r} link")
    problems += _nesting_problems(entities)
    ordered = tuple(sorted(entities, key=lambda e: (e.offset, -e.length)))
    return Parsed("".join(plain), ordered, tuple(problems))


def check_entities(text: str, entities: object) -> list[str]:
    """Problems with an explicit `entities` array against `text` (UTF-16 offsets)."""
    if not isinstance(entities, list):
        return ["entities is not an array"]
    problems = []
    size = utf16_len(text)
    parsed = []
    for raw in entities:
        if (
            not isinstance(raw, dict)
            or not isinstance(raw.get("offset"), int)
            or not isinstance(raw.get("length"), int)
        ):
            problems.append("an entity needs an integer offset and length")
            continue
        kind = raw.get("type")
        if kind not in ENTITY_TYPES:
            problems.append(f"unknown entity type {kind!r}")
            continue
        if (
            raw["offset"] < 0
            or raw["length"] < 0
            or raw["offset"] + raw["length"] > size
        ):
            problems.append(
                f"a {kind} entity at offset {raw['offset']} runs beyond the text's {size} UTF-16 units"
            )
            continue
        parsed.append(Entity(kind, raw["offset"], raw["length"]))
    return problems + _nesting_problems(parsed)


def parse_text(text: str, parse_mode: str | None, entities: object = None) -> Parsed:
    """`text` after entity parsing, as the Bot API reads it for `parse_mode` (HTML, MarkdownV2)."""
    if parse_mode in (None, ""):
        problems = tuple(check_entities(text, entities)) if entities is not None else ()
        found = tuple(
            Entity(raw["type"], raw["offset"], raw["length"])
            for raw in (entities if isinstance(entities, list) else [])
            if isinstance(raw, dict)
            and raw.get("type") in ENTITY_TYPES
            and isinstance(raw.get("offset"), int)
            and isinstance(raw.get("length"), int)
        )
        return Parsed(text, found, problems)
    if entities is not None:
        extra: tuple[str, ...] = (
            "entities and parse_mode are both given; the Bot API takes one",
        )
    else:
        extra = ()
    if parse_mode == "HTML":
        parsed = _parse_html(text)
    elif parse_mode == "MarkdownV2":
        parsed = _parse_markdown_v2(text)
    elif parse_mode == "Markdown":
        return Parsed(
            text,
            (),
            (
                "parse mode Markdown is the legacy mode, kept only for backward compatibility; use "
                "MarkdownV2 or HTML",
            )
            + extra,
        )
    else:
        return Parsed(text, (), (f"unknown parse_mode {parse_mode!r}",) + extra)
    return Parsed(parsed.plain, parsed.entities, parsed.problems + extra)


INLINE_ACTIONS = (
    "url",
    "callback_data",
    "web_app",
    "login_url",
    "switch_inline_query",
    "switch_inline_query_current_chat",
    "switch_inline_query_chosen_chat",
    "copy_text",
    "callback_game",
    "pay",
    "disabled",
)
REPLY_ACTIONS = (
    "request_users",
    "request_chat",
    "request_managed_bot",
    "request_contact",
    "request_location",
    "request_poll",
    "web_app",
)
BUTTON_STYLES = frozenset({"danger", "success", "primary"})
# The Bot API's field tables are closed: the server ignores any other field, so a misspelled
# `style` or `icon_custom_emoji_id` silently loses its effect. An unknown field is refused.
INLINE_BUTTON_FIELDS = frozenset(
    {"text", "icon_custom_emoji_id", "style", *INLINE_ACTIONS}
)
REPLY_BUTTON_FIELDS = frozenset(
    {"text", "icon_custom_emoji_id", "style", *REPLY_ACTIONS}
)
MARKUP_FIELDS = {
    "inline_keyboard": frozenset({"inline_keyboard", "force_reply"}),
    "keyboard": frozenset(
        {
            "keyboard",
            "is_persistent",
            "resize_keyboard",
            "one_time_keyboard",
            "input_field_placeholder",
            "selective",
            "force_reply",
        }
    ),
    "remove_keyboard": frozenset({"remove_keyboard", "selective"}),
    "force_reply": frozenset({"force_reply", "input_field_placeholder", "selective"}),
}


def _https(value: object) -> bool:
    return isinstance(value, dict) and str(value.get("url", "")).startswith("https://")


def _button_problems(button: object, where: str, *, inline: bool) -> list[str]:
    if isinstance(button, str) and not inline:
        return [] if button else [f"{where}: a button needs non-empty text"]
    if not isinstance(button, dict):
        return [f"{where}: a button must be an object"]
    problems = []
    for key in sorted(
        set(button) - (INLINE_BUTTON_FIELDS if inline else REPLY_BUTTON_FIELDS)
    ):
        problems.append(f"{where}: unknown field {key!r}, which the Bot API ignores")
    if not isinstance(button.get("text"), str) or not button["text"]:
        problems.append(f"{where}: a button needs non-empty text")
    style = button.get("style")
    if style is not None and style not in BUTTON_STYLES:
        problems.append(f"{where}: style {style!r} is not danger, success or primary")
    actions = [
        name for name in (INLINE_ACTIONS if inline else REPLY_ACTIONS) if name in button
    ]
    if inline and len(actions) != 1:
        problems.append(
            f"{where}: an inline button carries exactly one action field; this one has "
            f"{len(actions)}" + (f" ({', '.join(actions)})" if actions else "")
        )
    if not inline and len(actions) > 1:
        problems.append(
            f"{where}: a reply button carries at most one of {', '.join(actions)}"
        )
    data = button.get("callback_data")
    if data is not None:
        size = len(str(data).encode("utf-8"))
        if not isinstance(data, str) or not 1 <= size <= CALLBACK_DATA_MAX_BYTES:
            problems.append(
                f"{where}: callback_data is {size} bytes; the Bot API takes 1-64"
            )
    url = button.get("url")
    if url is not None and str(url).split(":", 1)[0].lower() not in (
        "http",
        "https",
        "tg",
    ):
        problems.append(
            f"{where}: url scheme {str(url).split(':', 1)[0]!r} is not http, https or tg"
        )
    for name in ("web_app", "login_url"):
        if name in button and not _https(button[name]):
            problems.append(f"{where}: {name}.url must be https")
    copy = button.get("copy_text")
    if copy is not None:
        text = copy.get("text") if isinstance(copy, dict) else None
        if not isinstance(text, str) or not 1 <= len(text) <= COPY_TEXT_MAX:
            length = len(text) if isinstance(text, str) else 0
            problems.append(
                f"{where}: copy_text.text is {length} characters; it takes 1-{COPY_TEXT_MAX}"
            )
    return problems


def check_keyboard(reply_markup: object) -> list[str]:
    """Problems with a `reply_markup`: inline and reply keyboards, their buttons and limits."""
    if not isinstance(reply_markup, dict):
        return ["reply_markup must be an object"]
    kind = next((name for name in MARKUP_FIELDS if name in reply_markup), None)
    if kind is None:
        return [
            "reply_markup names none of inline_keyboard, keyboard, remove_keyboard or force_reply"
        ]
    problems = [
        f"reply_markup has unknown field {key!r}, which the Bot API ignores"
        for key in sorted(set(reply_markup) - MARKUP_FIELDS[kind])
    ]
    for name, inline in (("inline_keyboard", True), ("keyboard", False)):
        if name not in reply_markup:
            continue
        rows = reply_markup[name]
        if not isinstance(rows, list) or not all(isinstance(row, list) for row in rows):
            problems.append(f"{name} must be an array of button rows")
            continue
        for row_index, row in enumerate(rows):
            for column, button in enumerate(row):
                problems += _button_problems(
                    button, f"{name}[{row_index}][{column}]", inline=inline
                )
    return problems


def check_message(payload: object) -> list[str]:
    """Every problem with one outbound send payload: length, markup, entities and keyboard."""
    if not isinstance(payload, dict):
        return ["a payload must be an object"]
    problems = []
    has_text = isinstance(payload.get("text"), str)
    has_caption = isinstance(payload.get("caption"), str)
    if not has_text and not has_caption:
        return ["a payload needs a text or a caption"]
    mode = payload.get("parse_mode")
    if has_text:
        parsed = parse_text(payload["text"], mode, payload.get("entities"))
        problems += list(parsed.problems)
        size = utf16_len(parsed.plain)
        if not 1 <= size <= TEXT_MAX:
            problems.append(
                f"text is {size} UTF-16 units after entity parsing; sendMessage takes 1-{TEXT_MAX}"
            )
        elif not parsed.plain.strip():
            problems.append(
                "text is empty after entity parsing: whitespace only, which the Bot API refuses "
                "(Text must be non-empty)"
            )
    if has_caption:
        parsed = parse_text(payload["caption"], mode, payload.get("caption_entities"))
        problems += list(parsed.problems)
        size = utf16_len(parsed.plain)
        if size > CAPTION_MAX:
            problems.append(
                f"caption is {size} UTF-16 units after entity parsing; captions take 0-{CAPTION_MAX}"
            )
    if "reply_markup" in payload:
        problems += check_keyboard(payload["reply_markup"])
    return problems


# --------------------------------------------------------------------------- payload classes


@dataclass(frozen=True, slots=True)
class PayloadFile:
    rel: str
    payload: object
    error: str | None


def read_payloads(base: Path) -> tuple[list[PayloadFile], int]:
    """Every `*.msg.json` under `base` (test directories included), and the files walked."""
    found = []
    walked = 0
    for path in walk(base):
        walked += 1
        if not path.name.endswith(".msg.json"):
            continue
        rel = "/".join(path.relative_to(base).parts)
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, ValueError) as error:
            found.append(
                PayloadFile(rel, None, f"not readable JSON ({type(error).__name__})")
            )
            continue
        send = document.get("send") if isinstance(document, dict) else None
        found.append(
            PayloadFile(rel, send if isinstance(send, dict) else document, None)
        )
    return found, walked


def _payload_class(base: Path, subject: bool, judge) -> Outcome:
    payloads, walked = read_payloads(base)
    findings = []
    for item in payloads:
        if item.error:
            findings.append(f"{item.rel}: {item.error}")
            continue
        findings += [f"{item.rel}: {problem}" for problem in judge(item.payload)]
    return Outcome(tuple(findings), len(payloads) if subject else walked)


def _length_problems(payload: object) -> list[str]:
    return [
        problem
        for problem in check_message(payload)
        if "after entity parsing" in problem
        or "needs a text or a caption" in problem
        or "must be an object" in problem
    ]


def _markup_problems(payload: object) -> list[str]:
    if not isinstance(payload, dict):
        return ["a payload must be an object"]
    problems = []
    for text_key, entity_key in (("text", "entities"), ("caption", "caption_entities")):
        if isinstance(payload.get(text_key), str):
            problems += list(
                parse_text(
                    payload[text_key],
                    payload.get("parse_mode"),
                    payload.get(entity_key),
                ).problems
            )
    return problems


def _keyboard_problems(payload: object) -> list[str]:
    if isinstance(payload, dict) and "reply_markup" in payload:
        return check_keyboard(payload["reply_markup"])
    return []


PAYLOAD_JUDGES = {
    "payload-length": _length_problems,
    "payload-markup": _markup_problems,
    "payload-keyboard": _keyboard_problems,
}
TREE_CHECKS = {
    "tg-update-mode": check_update_mode,
    "tg-webhook-endpoint": check_webhook_endpoint,
    "tg-poll-offset": check_poll_offset,
    "tg-poll-long": check_poll_long,
    "tg-allowed-updates": check_allowed_updates,
    "tg-retry-after": check_retry_after,
    "tg-parse-mode": check_parse_mode,
    "tg-escape": check_escape,
    "tg-length-bound": check_length_bound,
    "tg-callback-answer": check_callback_answer,
    "tg-callback-data": check_callback_data,
    "tg-deep-link": check_deep_link,
    "tg-replaced-fields": check_replaced_fields,
    "tma-version-gate": check_version_gate,
    "tma-storage": check_storage,
    "tma-send-data": check_send_data,
}


# --------------------------------------------------------------------------- command line


def report(name: str, outcome: Outcome) -> int:
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    top = argparse.ArgumentParser(
        prog="telegram-platform-probe",
        description="Judge a Telegram bot and Mini App tree, or a subject of outbound messages.",
    )
    top.add_argument("--root", type=Path, default=None)
    top.add_argument("--subject", type=Path, default=None)
    verbs = top.add_subparsers(dest="verb", required=True)
    verbs.add_parser("classes", help="list the classes")
    check = verbs.add_parser("check", help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    message = verbs.add_parser("message", help="judge one payload file")
    message.add_argument("file", type=Path)
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for name in CLASSES:
            print(name)
        return EXIT_GREEN
    if args.verb == "message":
        try:
            document = json.loads(args.file.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, ValueError) as error:
            print(f"telegram-platform-probe: {args.file}: {error}", file=sys.stderr)
            return EXIT_USAGE
        send = document.get("send") if isinstance(document, dict) else None
        return report(
            "message",
            Outcome(
                tuple(check_message(send if isinstance(send, dict) else document)), 1
            ),
        )
    klass = args.klass
    if args.subject is not None:
        if klass not in PAYLOAD_JUDGES:
            print(
                f"telegram-platform-probe: {klass} judges a --root tree, not a --subject",
                file=sys.stderr,
            )
            return EXIT_USAGE
        if not args.subject.is_dir():
            print(
                f"telegram-platform-probe: --subject {args.subject} is not a directory",
                file=sys.stderr,
            )
            return EXIT_USAGE
        return report(klass, _payload_class(args.subject, True, PAYLOAD_JUDGES[klass]))
    root = args.root if args.root is not None else Path.cwd()
    if not root.is_dir():
        print(
            f"telegram-platform-probe: --root {root} is not a directory",
            file=sys.stderr,
        )
        return EXIT_USAGE
    if klass in PAYLOAD_JUDGES:
        return report(klass, _payload_class(root, False, PAYLOAD_JUDGES[klass]))
    return report(klass, TREE_CHECKS[klass](read_tree(root)))


if __name__ == "__main__":
    sys.exit(main())

"""Every committed golden message of the bot parses as the Bot API would parse it (SPEC-026 A16).

The bot's golden messages are `crates/bot/tests/messages/*.msg.json`, each in the envelope
`phx.duty.message.v1` with the literal payload the bot sends under `send` (R12). This test is
DeckStreak's own reading of the Bot API's formatting rules for `parse_mode` HTML, written here and
importing nothing: the tags the Bot API supports and the attributes each takes, the four named
entities and the numeric ones, every tag closed in the order it was opened, the nesting the Bot API
allows, a link's scheme, and the length after entity parsing in UTF-16 units (a text 1 to 4096 and
not whitespace alone, a caption at most 1024). The inline keyboard's buttons are read for the
Bot API's own bounds too. Each refusal is first proved on a planted payload, then applied to every
committed golden. The telegram-platform pack's payload rows judge the same files on the box.
"""

import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MESSAGES = ROOT / "crates" / "bot" / "tests" / "messages"
SCHEMA = "phx.duty.message.v1"
TEXT_MAX = 4096
CAPTION_MAX = 1024
CALLBACK_DATA_MAX_BYTES = 64

#: The entities a tag makes, which the nesting rules read.
KINDS = {
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
    "a": "link",
    "tg-emoji": "custom_emoji",
    "code": "code",
    "pre": "pre",
    "blockquote": "blockquote",
}
#: Entities that may contain, and sit inside, anything but code and pre.
STYLES = {"bold", "italic", "underline", "strikethrough", "spoiler"}
NAMED = {"lt": "<", "gt": ">", "amp": "&", "quot": '"'}
TAG = re.compile(r"<(/?)([a-zA-Z][a-zA-Z0-9-]*)((?:\s+[^<>]*?)?)\s*>")
ATTRIBUTE = re.compile(r'([a-zA-Z-]+)(?:="([^"]*)")?')
LINK_SCHEMES = ("http://", "https://", "tg://")


def utf16(text):
    """The length of `text` in UTF-16 code units, as the Bot API counts it."""
    return len(text.encode("utf-16-le")) // 2


def unescape(value):
    """An attribute's value after its entities are read."""
    return re.sub(r"&(lt|gt|amp|quot);", lambda match: NAMED[match.group(1)], value)


def entity_problem(open_kind, inner_kind):
    """Why `inner_kind` may not sit inside `open_kind`, or None when the Bot API allows it."""
    if open_kind == "blockquote" and inner_kind == "blockquote":
        return "a block quotation inside a block quotation"
    if open_kind == "pre" and inner_kind == "code":
        return None
    if open_kind in ("code", "pre") or inner_kind in ("code", "pre"):
        return f"{inner_kind} inside {open_kind}"
    if open_kind in STYLES or inner_kind in STYLES or "blockquote" in (open_kind, inner_kind):
        return None
    return f"{inner_kind} inside {open_kind}"


def parse(text):
    """`text` read as the Bot API reads HTML: its visible text and every problem found."""
    problems = []
    plain = []
    stack = []
    index = 0
    while index < len(text):
        character = text[index]
        if character == "<":
            match = TAG.match(text, index)
            if match is None:
                problems.append(f"a '<' at {index} starts no tag")
                index += 1
                continue
            closing, name, attributes = match.group(1), match.group(2).lower(), match.group(3)
            index = match.end()
            if name not in KINDS:
                problems.append(f"<{name}> is not a tag the Bot API supports")
                continue
            if closing:
                if not stack or stack[-1][0] != name:
                    problems.append(f"</{name}> closes no tag open innermost")
                else:
                    stack.pop()
                continue
            values = {key: value for key, value in ATTRIBUTE.findall(attributes.strip())}
            kind = KINDS[name]
            if name == "span" and values.get("class") != "tg-spoiler":
                problems.append('<span> is supported only as class="tg-spoiler"')
            if name == "a":
                href = unescape(values.get("href", ""))
                if not href.startswith(LINK_SCHEMES):
                    problems.append(f"a link to {href!r} would be dropped")
            if name == "tg-emoji" and not values.get("emoji-id"):
                problems.append("<tg-emoji> without emoji-id")
            if name == "code" and "class" in values:
                if not (stack and stack[-1][0] == "pre"):
                    problems.append("a code language outside <pre>")
                elif not values["class"].startswith("language-"):
                    problems.append(f"code class {values['class']!r} is no language")
            for open_name, open_kind in stack:
                why = entity_problem(open_kind, kind)
                if why:
                    problems.append(f"<{name}>: {why}")
                    break
            stack.append((name, kind))
        elif character == "&":
            match = re.match(r"&(?:([a-z]+)|#([0-9]+)|#[xX]([0-9a-fA-F]+));", text[index:])
            if match is None or (match.group(1) and match.group(1) not in NAMED):
                problems.append(f"an '&' at {index} starts no entity the Bot API reads")
                index += 1
                continue
            if match.group(1):
                plain.append(NAMED[match.group(1)])
            else:
                code = int(match.group(2)) if match.group(2) else int(match.group(3), 16)
                plain.append(chr(code))
            index += match.end()
        elif character == ">":
            problems.append(f"a bare '>' at {index}")
            index += 1
        else:
            plain.append(character)
            index += 1
    problems += [f"<{name}> is never closed" for name, _ in stack]
    return "".join(plain), problems


def keyboard_problems(markup):
    """Every problem with an inline keyboard, as the Bot API bounds it."""
    problems = []
    rows = markup.get("inline_keyboard")
    if not isinstance(rows, list) or not all(isinstance(row, list) for row in rows):
        return ["the markup names no inline keyboard of rows"]
    for row in rows:
        for button in row:
            actions = [key for key in ("url", "callback_data", "web_app") if key in button]
            if not button.get("text") or len(actions) != 1:
                problems.append(f"a button needs a text and one action: {button}")
                continue
            if "callback_data" in button:
                size = len(button["callback_data"].encode("utf-8"))
                if not 1 <= size <= CALLBACK_DATA_MAX_BYTES:
                    problems.append(f"callback_data of {size} bytes")
            if "web_app" in button and not button["web_app"].get("url", "").startswith("https://"):
                problems.append("a web_app button's URL is not https")
            if "url" in button and not button["url"].startswith(LINK_SCHEMES):
                problems.append("a button's URL is not http, https or tg")
    return problems


def payload_problems(send):
    """Every problem with one payload the bot sends."""
    if not isinstance(send, dict):
        return ["the payload is not an object"]
    if send.get("parse_mode") != "HTML":
        return [f"parse_mode {send.get('parse_mode')!r}, not HTML"]
    problems = []
    has_text, has_caption = "text" in send, "caption" in send
    if not has_text and not has_caption:
        return ["the payload has no text and no caption"]
    if has_text:
        plain, found = parse(send["text"])
        problems += found
        if not 1 <= utf16(plain) <= TEXT_MAX:
            problems.append(f"a text of {utf16(plain)} UTF-16 units; the Bot API takes 1-4096")
        elif not plain.strip():
            problems.append("a text of whitespace alone")
    if has_caption:
        plain, found = parse(send["caption"])
        problems += found
        if utf16(plain) > CAPTION_MAX:
            problems.append(f"a caption of {utf16(plain)} UTF-16 units; the Bot API takes 0-1024")
    if "reply_markup" in send:
        problems += keyboard_problems(send["reply_markup"])
    if "disable_web_page_preview" in send or "reply_to_message_id" in send:
        problems.append("a field a later Bot API version replaced")
    return problems


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def goldens():
    """Every committed golden message, refusing an empty population."""
    return examined("golden message(s)", sorted(MESSAGES.glob("*.msg.json")))


class BotMessagesTest(unittest.TestCase):
    def test_the_reading_refuses_what_the_bot_api_refuses(self):
        refused = {
            "an unclosed tag": {"text": "<b>open", "parse_mode": "HTML"},
            "an unsupported tag": {"text": "<br>line", "parse_mode": "HTML"},
            "a bare '<'": {"text": "1 < 2", "parse_mode": "HTML"},
            "a bare '&'": {"text": "Tom & Jerry", "parse_mode": "HTML"},
            "an unknown named entity": {"text": "a&nbsp;b", "parse_mode": "HTML"},
            "a crossed nesting": {"text": "<b><i>x</b></i>", "parse_mode": "HTML"},
            "code inside code": {"text": "<code><code>x</code></code>", "parse_mode": "HTML"},
            "a link inside a link": {
                "text": '<a href="https://a.example"><a href="https://b.example">x</a></a>',
                "parse_mode": "HTML",
            },
            "a dropped link": {"text": '<a href="javascript:x">x</a>', "parse_mode": "HTML"},
            "a text over the bound": {"text": "a" * (TEXT_MAX + 1), "parse_mode": "HTML"},
            "an astral text over the bound": {
                "text": "\U0001f4da" * (TEXT_MAX // 2) + "a",
                "parse_mode": "HTML",
            },
            "whitespace alone": {"text": " \n <b> </b>", "parse_mode": "HTML"},
            "a caption over the bound": {"caption": "c" * (CAPTION_MAX + 1), "parse_mode": "HTML"},
            "legacy Markdown": {"text": "*bold*", "parse_mode": "Markdown"},
            "a button with two actions": {
                "text": "x",
                "parse_mode": "HTML",
                "reply_markup": {
                    "inline_keyboard": [[{"text": "b", "url": "https://a", "callback_data": "c"}]]
                },
            },
            "callback data over its bytes": {
                "text": "x",
                "parse_mode": "HTML",
                "reply_markup": {"inline_keyboard": [[{"text": "b", "callback_data": "é" * 33}]]},
            },
            "a web_app over http": {
                "text": "x",
                "parse_mode": "HTML",
                "reply_markup": {
                    "inline_keyboard": [[{"text": "b", "web_app": {"url": "http://a.example"}}]]
                },
            },
            "a replaced field": {
                "text": "x",
                "parse_mode": "HTML",
                "disable_web_page_preview": True,
            },
        }
        accepted = {
            "every entity": {
                "text": "".join(
                    [
                        "<b>b</b><strong>s</strong><i>i</i><em>e</em><u>u</u><ins>n</ins>",
                        "<s>s</s><strike>k</strike><del>d</del><tg-spoiler>t</tg-spoiler>",
                        '<span class="tg-spoiler">p</span>',
                        '<a href="https://x.example/?a=1&amp;b=2">a</a>',
                        '<tg-emoji emoji-id="1234">\U0001f44d</tg-emoji><code>c</code>',
                        '<pre><code class="language-rust">r</code></pre>',
                        "<blockquote expandable>q</blockquote>&lt;&gt;&amp;&quot;&#65;&#x42;",
                    ]
                ),
                "parse_mode": "HTML",
            },
            "styles nested in any order": {
                "text": "<b><i><u><s>x</s></u></i></b> <blockquote><b>q</b></blockquote>",
                "parse_mode": "HTML",
            },
            "a text at the bound": {"text": "a" * TEXT_MAX, "parse_mode": "HTML"},
            "a caption at the bound": {"caption": "c" * CAPTION_MAX, "parse_mode": "HTML"},
            "a keyboard": {
                "text": "x",
                "parse_mode": "HTML",
                "reply_markup": {
                    "inline_keyboard": [
                        [{"text": "a", "web_app": {"url": "https://a.example/app"}}],
                        [{"text": "b", "callback_data": "b" * CALLBACK_DATA_MAX_BYTES}],
                    ]
                },
            },
        }
        for name, send in refused.items():
            with self.subTest(refused=name):
                self.assertTrue(payload_problems(send), f"{name} was not refused")
        for name, send in accepted.items():
            with self.subTest(accepted=name):
                self.assertEqual(payload_problems(send), [], name)

    def test_every_golden_message_parses_as_the_bot_api_would(self):
        for path in goldens():
            with self.subTest(golden=path.name):
                envelope = json.loads(path.read_text(encoding="utf-8"))
                self.assertIsInstance(envelope, dict)
                self.assertEqual(envelope.get("schema"), SCHEMA)
                self.assertEqual(payload_problems(envelope.get("send")), [], path.name)


if __name__ == "__main__":
    unittest.main()

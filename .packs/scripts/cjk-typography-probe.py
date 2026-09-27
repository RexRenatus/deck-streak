#!/usr/bin/env python3
"""cjk-typography-probe: static checks of a web tree's Chinese, Japanese and Korean typography.

The packs/cjk-typography pack follows the W3C's layout requirements for the three scripts:

- JLREQ, for Japanese;
- CLREQ, for Chinese, simplified and traditional;
- KLREQ, for Korean;

together with CSS Text 3 and 4, CSS Ruby, CSS Fonts 4, HTML's ruby model and BCP 47 language
tags. It also enforces the house CJK rules the owner approved with the stack.

This script checks a tree's markup, CSS and message files for those rules. It REUSES
accessibility-probe.py, loaded with importlib from beside this script, for the tree walk, the
markup tokenizer and the CSS rule reader, and copies none of them. The notation of the TEXT that
DeckStreak's mentors write is language-mentors' to judge: its `furigana-ruby` and `pinyin-tones`
classes run as this pack's rows, straight from the pack's checks.json.

Classes (`check <class>`) print one line per finding, `<class>: <finding>`, and end with
`examined N`, the files read to decide. Exit codes: 0 green; 1 at least one finding; 2 usage;
3 VOID, meaning nothing was examined or accessibility-probe.py could not be loaded. VOID is never
a pass.

Standard library only, so it vendors into any repository beside accessibility-probe.py.

Usage:
    cjk-typography-probe.py classes
    cjk-typography-probe.py --root R check <class>
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Callable

HERE = Path(__file__).resolve().parent
BASE_PATH = HERE / "accessibility-probe.py"

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

CLASSES = (
    "cjk-lang",
    "zh-script-subtag",
    "cjk-font-stack",
    "cjk-font-subset",
    "cjk-font-display",
    "kinsoku",
    "korean-keep-all",
    "no-break-all",
    "phrase-headings",
    "cjk-line-height",
    "mixed-script-autospace",
    "ruby-conformance",
    "ruby-legible",
    "pinyin-nfc",
)

# Unicode blocks, by script. Han includes the iteration mark and ideographic zero.
HAN = "々〇㐀-䶿一-鿿豈-﫿\U00020000-\U0002fa1f"
KANA = "぀-ゟ゠-ヿㇰ-ㇿ･-ﾟ"
HANGUL = "ᄀ-ᇿ㄰-㆏ꥠ-꥿가-힯ힰ-퟿"
BOPOMOFO = "㄀-ㄯㆠ-ㆿ"
CJK = re.compile(f"[{HAN}{KANA}{HANGUL}{BOPOMOFO}]")
KANA_RE = re.compile(f"[{KANA}]")
HANGUL_RE = re.compile(f"[{HANGUL}]")
BOPOMOFO_RE = re.compile(f"[{BOPOMOFO}]")
CJK_RANGES = (
    (0x1100, 0x11FF),
    (0x3000, 0x9FFF),
    (0xA960, 0xA97F),
    (0xAC00, 0xD7FF),
    (0xF900, 0xFAFF),
    (0xFF00, 0xFFEF),
    (0x20000, 0x2FA1F),
)

LANGUAGES = ("ja", "ko", "zh-Hans", "zh-Hant")
HANS_REGIONS = frozenset({"CN", "SG", "MY"})
HANT_REGIONS = frozenset({"TW", "HK", "MO"})
CJK_PRIMARIES = frozenset({"ja", "ko", "zh", "cmn", "yue", "lzh", "wuu", "hak", "nan"})
LANG_ATTRIBUTES = ("lang", "xml:lang", "hreflang")
TEXT_ATTRIBUTES = ("alt", "title", "placeholder", "aria-label", "aria-description")
COPY_DIRS = frozenset(
    {
        "messages",
        "locales",
        "locale",
        "i18n",
        "l10n",
        "lang",
        "langs",
        "translations",
        "copy",
        "strings",
        "content",
    }
)
FONT_EXT = frozenset({".woff2", ".woff", ".ttf", ".otf"})
TEXT_EXT = frozenset({".md", ".txt"})
FONT_LIMIT = 1 << 20
GLOBAL_SELECTORS = frozenset({"html", "body", ":root", "*"})
CODEISH = re.compile(
    r"(?:^|[\s>+~(,])(?:code|pre|kbd|samp|tt)\b|[.#][\w-]*(?:url|code|mono|hash|token|sha)[\w-]*",
    re.I,
)
LANG_PSEUDO = re.compile(r":lang\(([^)]*)\)", re.I)
LANG_ATTR_SELECTOR = re.compile(
    r"\[\s*lang\s*(?:\|=|\^=|=)\s*['\"]?([A-Za-z0-9-]+)['\"]?\s*\]", re.I
)


class Void(Exception):
    """The class could not read what it needs, so it examined nothing."""


@dataclass(frozen=True)
class Outcome:
    findings: list[str]
    examined: int


def load_base() -> ModuleType:
    """accessibility-probe.py, beside this script, registered before it runs its class bodies."""
    spec = importlib.util.spec_from_file_location("accessibility_probe", BASE_PATH)
    if spec is None or spec.loader is None or not BASE_PATH.is_file():
        raise Void(f"cannot load {BASE_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules["accessibility_probe"] = module
    spec.loader.exec_module(module)
    return module


# --- language tags --------------------------------------------------------------------------------


@dataclass(frozen=True)
class Tag:
    text: str
    primary: str
    script: str | None
    region: str | None

    @property
    def key(self) -> str | None:
        """The typographic language: ja, ko, zh-Hans, zh-Hant, or zh when no script is named."""
        if self.primary == "ja":
            return "ja"
        if self.primary == "ko":
            return "ko"
        if self.primary in ("zh", "cmn"):
            return f"zh-{self.script}" if self.script in ("Hans", "Hant") else "zh"
        return None

    def script_hint(self) -> str:
        if self.region in HANT_REGIONS:
            return "zh-Hant"
        if self.region in HANS_REGIONS:
            return "zh-Hans"
        return "zh-Hans or zh-Hant"


def parse_tag(text: str) -> Tag | None:
    """A BCP 47 tag's language, script and region, or None when it is not a static tag."""
    text = text.strip()
    if not re.fullmatch(r"[A-Za-z]{2,8}(?:[-_][A-Za-z0-9]{1,8})*", text):
        return None
    parts = re.split(r"[-_]", text)
    script = region = None
    for part in parts[1:]:
        if len(part) == 4 and part.isalpha() and script is None:
            script = part.title()
        elif (len(part) == 2 and part.isalpha()) or (len(part) == 3 and part.isdigit()):
            region = part.upper() if region is None else region
    return Tag(text, parts[0].lower(), script, region)


def selector_tags(selector: str) -> list[Tag]:
    tags = []
    for match in LANG_PSEUDO.finditer(selector):
        for part in match.group(1).split(","):
            tag = parse_tag(part.strip().strip("'\""))
            if tag:
                tags.append(tag)
    for match in LANG_ATTR_SELECTOR.finditer(selector):
        tag = parse_tag(match.group(1))
        if tag:
            tags.append(tag)
    return tags


def targets(selector: str, language: str, exact: bool = False) -> bool:
    """Whether a selector styles `language`: its own tag, or bare zh for any Chinese script."""
    for tag in selector_tags(selector):
        key = tag.key
        if key == language:
            return True
        if not exact and key == "zh" and language.startswith("zh"):
            return True
    return False


def is_global(selector: str) -> bool:
    return selector.strip() in GLOBAL_SELECTORS


def codeish(selectors: list[str]) -> bool:
    return bool(selectors) and all(CODEISH.search(sel) for sel in selectors)


# --- the tree -------------------------------------------------------------------------------------


@dataclass
class Scan:
    base: ModuleType
    root: Path
    tree: object
    markup: list
    rules: list
    style_read: int
    config: list

    def texts(self):
        for source in self.markup:
            yield source, self.base.markup_of(source)


def walk_extra(base: ModuleType, root: Path, exts: frozenset) -> list[tuple[str, Path]]:
    """Files of `exts` by the same pruned walk accessibility-probe.py uses."""
    found = []
    for dirpath, dirnames, filenames in os.walk(root):
        here = Path(dirpath)
        parent = here.relative_to(root).as_posix()
        parent = "" if parent == "." else parent
        dirnames[:] = sorted(name for name in dirnames if base.keep_dir(parent, name))
        for name in sorted(filenames):
            if Path(name).suffix.lower() in exts:
                found.append((f"{parent}/{name}" if parent else name, here / name))
    return found


def i18n_config(tree) -> list:
    """project.inlang settings and message files: where an i18n library names its locales."""
    files = []
    for source in tree.files:
        parts = source.rel.split("/")
        if (
            parts[-1] == "settings.json"
            and len(parts) > 1
            and parts[-2].endswith(".inlang")
        ):
            files.append(source)
        elif source.ext == ".json" and len(parts) > 1 and parts[-2] in COPY_DIRS:
            files.append(source)
    return files


def scan(root: Path) -> Scan:
    base = load_base()
    tree = base.scan(root)
    rules, style_read = base.rules_of(tree)
    return Scan(base, root, tree, tree.markup(), rules, style_read, i18n_config(tree))


def config_locales(source) -> list[str]:
    try:
        data = json.loads(source.text)
    except ValueError:
        return []
    found: list[str] = []
    if isinstance(data, dict):
        for key in ("locales", "languageTags"):
            value = data.get(key)
            if isinstance(value, list):
                found.extend(item for item in value if isinstance(item, str))
        for key in ("baseLocale", "sourceLanguageTag"):
            value = data.get(key)
            if isinstance(value, str):
                found.append(value)
    return found


def message_tag(source) -> Tag | None:
    parts = source.rel.split("/")
    if len(parts) < 2 or parts[-2] not in COPY_DIRS:
        return None
    return parse_tag(Path(parts[-1]).stem)


def static_lang(element) -> str | None:
    value = element.get("lang")
    if value is None or "{" in value or "%" in value:
        return None
    return value


def dynamic_lang(element) -> bool:
    value = element.get("lang")
    return value is not None and ("{" in value or "%" in value)


def used_languages(scan: Scan) -> set[str]:
    """ja, ko, zh-Hans and zh-Hant as the tree uses them: tags, locales, message files and text."""
    used: set[str] = set()
    for source, markup in scan.texts():
        for element in markup.elements:
            for attribute in ("lang", "xml:lang"):
                value = element.literal(attribute)
                tag = parse_tag(value) if value else None
                if tag and tag.key:
                    used.add(tag.key)
        for _offset, chunk, _parent in markup.texts:
            if KANA_RE.search(chunk):
                used.add("ja")
            if HANGUL_RE.search(chunk):
                used.add("ko")
    for source in scan.config:
        for locale in config_locales(source):
            tag = parse_tag(locale)
            if tag and tag.key:
                used.add(tag.key)
        tag = message_tag(source)
        if tag and tag.key:
            used.add(tag.key)
    return used & set(LANGUAGES) | ({"zh"} & used)


def examined(scan: Scan) -> int:
    return len(scan.markup) + scan.style_read + len(scan.config)


def snippet(text: str) -> str:
    text = " ".join(text.split())
    return text if len(text) <= 24 else text[:24] + "..."


# --- the classes ----------------------------------------------------------------------------------


def expression_free(text: str) -> str:
    return re.sub(r"\{[^{}]*\}", " ", text)


def nearest_lang(element) -> tuple[str | None, bool]:
    """(the nearest static lang, whether a dynamic lang was met first)."""
    node = element
    while node is not None:
        if dynamic_lang(node):
            return None, True
        value = static_lang(node)
        if value is not None:
            return value, False
        node = node.parent
    return None, False


def mismatch(text: str, value: str) -> str | None:
    tag = parse_tag(value)
    primary = tag.primary if tag else value.lower()
    if KANA_RE.search(text) and primary != "ja":
        return "kana"
    if HANGUL_RE.search(text) and primary != "ko":
        return "Hangul"
    if primary not in CJK_PRIMARIES and not (
        KANA_RE.search(text) or HANGUL_RE.search(text)
    ):
        return "CJK"
    return None


def check_cjk_lang(scan: Scan) -> Outcome:
    findings: list[str] = []
    for source, markup in scan.texts():
        for offset, chunk, parent in markup.texts:
            text = expression_free(chunk)
            first = CJK.search(text)
            if not first:
                continue
            value, dynamic = (
                nearest_lang(parent) if parent is not None else (None, False)
            )
            at = source.at(offset + first.start())
            if dynamic:
                continue
            if value is None:
                findings.append(
                    f"{at} CJK text {snippet(text)!r} has no lang on it or around it"
                )
                continue
            kind = mismatch(text, value)
            if kind:
                findings.append(
                    f'{at} {kind} text {snippet(text)!r} sits under lang="{value}"'
                )
        for element in markup.elements:
            for attribute in TEXT_ATTRIBUTES:
                content = element.literal(attribute)
                if not content or not CJK.search(content):
                    continue
                value, dynamic = nearest_lang(element)
                if dynamic:
                    continue
                if value is None:
                    findings.append(
                        f"{source.at(element.start)} {attribute} {snippet(content)!r} has no lang on it or around it"
                    )
                elif mismatch(content, value):
                    findings.append(
                        f'{source.at(element.start)} {attribute} {snippet(content)!r} sits under lang="{value}"'
                    )
    return Outcome(findings, len(scan.markup))


def unscripted(tag: Tag | None) -> bool:
    return tag is not None and tag.key == "zh"


def check_zh_script_subtag(scan: Scan) -> Outcome:
    findings: list[str] = []
    for source, markup in scan.texts():
        for element in markup.elements:
            for attribute in LANG_ATTRIBUTES:
                value = element.literal(attribute)
                tag = parse_tag(value) if value else None
                if unscripted(tag):
                    findings.append(
                        f'{source.at(element.start)} {attribute}="{value}" names no script: write {tag.script_hint()}'
                    )
    for rule in scan.rules:
        if rule.at or rule.value("font-family") is None:
            continue
        for selector in rule.selectors:
            for match in LANG_PSEUDO.finditer(selector):
                tags = [
                    parse_tag(part.strip().strip("'\""))
                    for part in match.group(1).split(",")
                ]
                if any(unscripted(tag) for tag in tags):
                    findings.append(
                        f"{rule.source.at(rule.offset)} {match.group(0)} sets font-family for every Chinese script; "
                        "give zh-Hans and zh-Hant their own stacks"
                    )
    for source in scan.config:
        for locale in config_locales(source):
            if unscripted(parse_tag(locale)):
                findings.append(
                    f"{source.rel} locale {locale!r} names no script: write {parse_tag(locale).script_hint()}"
                )
        tag = message_tag(source)
        if unscripted(tag):
            findings.append(
                f"{source.rel} is named for {tag.text!r}, which names no script: write {tag.script_hint()}"
            )
    return Outcome(findings, examined(scan))


def font_families(value: str) -> list[str]:
    return [
        part.strip().strip("'\"").lower() for part in value.split(",") if part.strip()
    ]


def cjk_stack_families(scan: Scan) -> set[str]:
    families: set[str] = set()
    for rule in scan.rules:
        stack = rule.value("font-family")
        if rule.at or stack is None:
            continue
        if any(
            targets(sel, language) for sel in rule.selectors for language in LANGUAGES
        ):
            families.update(font_families(stack))
    return families


def check_cjk_font_stack(scan: Scan) -> Outcome:
    findings: list[str] = []
    used = used_languages(scan)
    for language in LANGUAGES:
        if language not in used:
            continue
        stacked = any(
            rule.value("font-family") is not None
            and any(targets(sel, language, exact=True) for sel in rule.selectors)
            for rule in scan.rules
            if not rule.at
        )
        if not stacked:
            findings.append(
                f"{language} is used and no :lang({language}) rule sets font-family"
            )
    return Outcome(findings, examined(scan))


def face_family(rule) -> str | None:
    value = rule.value("font-family")
    return value.strip().strip("'\"").lower() if value else None


def check_cjk_font_subset(scan: Scan) -> Outcome:
    findings: list[str] = []
    fonts = walk_extra(scan.base, scan.root, FONT_EXT)
    for rel, path in fonts:
        try:
            size = path.stat().st_size
        except OSError:
            continue
        if size > FONT_LIMIT:
            findings.append(
                f"{rel} is {size / (1 << 20):.1f} MiB; ship unicode-range slices or use a system font"
            )
    families = cjk_stack_families(scan)
    for rule in scan.rules:
        if rule.at != "font-face":
            continue
        family = face_family(rule)
        source = rule.value("src") or ""
        if (
            family in families
            and "url(" in source
            and rule.value("unicode-range") is None
        ):
            findings.append(
                f"{rule.source.at(rule.offset)} @font-face {family_label(rule)!r} feeds a CJK stack and declares no unicode-range"
            )
    return Outcome(findings, len(fonts) + scan.style_read)


def family_label(rule) -> str:
    value = rule.value("font-family") or ""
    return value.strip().strip("'\"")


def covers_cjk(unicode_range: str | None) -> bool:
    if not unicode_range:
        return False
    for token in unicode_range.split(","):
        match = re.fullmatch(r"\s*U\+([0-9A-Fa-f?]+)(?:-([0-9A-Fa-f]+))?\s*", token)
        if not match:
            continue
        low = int(match.group(1).replace("?", "0"), 16)
        high = (
            int(match.group(2), 16)
            if match.group(2)
            else int(match.group(1).replace("?", "F"), 16)
        )
        if any(low <= end and high >= start for start, end in CJK_RANGES):
            return True
    return False


def check_cjk_font_display(scan: Scan) -> Outcome:
    findings: list[str] = []
    families = cjk_stack_families(scan)
    for rule in scan.rules:
        if rule.at != "font-face" or rule.value("font-display") is not None:
            continue
        if face_family(rule) in families or covers_cjk(rule.value("unicode-range")):
            findings.append(
                f"{rule.source.at(rule.offset)} @font-face {family_label(rule)!r} carries CJK glyphs and sets no font-display"
            )
    return Outcome(findings, scan.style_read)


def cjk_used(scan: Scan) -> bool:
    if used_languages(scan):
        return True
    return any(
        CJK.search(chunk)
        for _source, markup in scan.texts()
        for _o, chunk, _p in markup.texts
    )


def rule_sets(rule, prop: str, value: str) -> bool:
    current = rule.value(prop)
    return current is not None and current.strip().lower() == value


def check_kinsoku(scan: Scan) -> Outcome:
    findings: list[str] = []
    if not cjk_used(scan):
        return Outcome(findings, examined(scan))
    for rule in scan.rules:
        if (
            rule.at
            or not rule_sets(rule, "line-break", "anywhere")
            or codeish(rule.selectors)
        ):
            continue
        findings.append(
            f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` sets line-break: anywhere, which ignores "
            "the line-start and line-end prohibitions"
        )
    if "ja" in used_languages(scan):
        strict = any(
            rule_sets(rule, "line-break", "strict")
            and any(targets(sel, "ja") or is_global(sel) for sel in rule.selectors)
            for rule in scan.rules
            if not rule.at
        )
        if not strict:
            findings.append("ja is used and no :lang(ja) rule sets line-break: strict")
    return Outcome(findings, examined(scan))


def check_korean_keep_all(scan: Scan) -> Outcome:
    findings: list[str] = []
    if "ko" in used_languages(scan):
        styled = any(
            rule_sets(rule, "word-break", "keep-all")
            and any(targets(sel, "ko") for sel in rule.selectors)
            for rule in scan.rules
            if not rule.at
        )
        utility = any(
            "break-keep" in (element.literal("class") or "").split()
            and (
                parse_tag(nearest_lang(element)[0] or "") or Tag("", "", None, None)
            ).key
            == "ko"
            for _source, markup in scan.texts()
            for element in markup.elements
        )
        if not (styled or utility):
            findings.append(
                "ko is used and no :lang(ko) rule sets word-break: keep-all"
            )
    return Outcome(findings, examined(scan))


def check_no_break_all(scan: Scan) -> Outcome:
    findings: list[str] = []
    if not cjk_used(scan):
        return Outcome(findings, examined(scan))
    for rule in scan.rules:
        if (
            rule.at
            or not rule_sets(rule, "word-break", "break-all")
            or codeish(rule.selectors)
        ):
            continue
        findings.append(
            f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` sets word-break: break-all, which splits "
            "Latin words inside CJK text; use overflow-wrap: anywhere"
        )
    for source, markup in scan.texts():
        for element in markup.elements:
            if element.name in ("code", "pre", "kbd", "samp"):
                continue
            if "break-all" in (element.literal("class") or "").split():
                findings.append(
                    f"{source.at(element.start)} <{element.name}> carries break-all, which splits Latin words inside CJK text"
                )
    return Outcome(findings, examined(scan))


HEADING = re.compile(r"\bh[1-6]\b", re.I)


def check_phrase_headings(scan: Scan) -> Outcome:
    findings: list[str] = []
    used = used_languages(scan)
    for language in ("ja", "ko"):
        if language not in used:
            continue
        phrased = any(
            rule_sets(rule, "word-break", "auto-phrase")
            and any(
                HEADING.search(sel) or targets(sel, language) for sel in rule.selectors
            )
            for rule in scan.rules
            if not rule.at
        )
        if not phrased:
            findings.append(
                f"{language} is used and no heading rule sets word-break: auto-phrase"
            )
    return Outcome(findings, examined(scan))


def ratio(value: str | None) -> float | None:
    if not value:
        return None
    value = value.strip().lower()
    if re.fullmatch(r"\d*\.?\d+", value):
        return float(value)
    if re.fullmatch(r"\d*\.?\d+%", value):
        return float(value[:-1]) / 100
    return None


def check_cjk_line_height(scan: Scan) -> Outcome:
    findings: list[str] = []
    used = used_languages(scan)
    for rule in scan.rules:
        if rule.at:
            continue
        height = ratio(rule.value("line-height"))
        if height is None or height >= 1.5:
            continue
        if any(
            targets(sel, language) for sel in rule.selectors for language in LANGUAGES
        ):
            findings.append(
                f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` sets line-height {height:g}; "
                "CJK text needs 1.5 or more"
            )
    body = None
    for rule in scan.rules:
        if not rule.at and any(is_global(sel) for sel in rule.selectors):
            body = (
                ratio(rule.value("line-height")) if rule.value("line-height") else body
            )
    for language in LANGUAGES:
        if language not in used:
            continue
        own = any(
            rule.value("line-height") is not None
            and any(targets(sel, language) for sel in rule.selectors)
            for rule in scan.rules
            if not rule.at
        )
        if not own and body is not None and body < 1.5:
            findings.append(
                f"{language} is used, no :lang({language}) rule sets line-height, and the body's is {body:g}"
            )
    return Outcome(findings, examined(scan))


def check_mixed_script_autospace(scan: Scan) -> Outcome:
    findings: list[str] = []
    used = used_languages(scan)
    for rule in scan.rules:
        if rule.at or not rule_sets(rule, "text-autospace", "no-autospace"):
            continue
        if any(
            is_global(sel) or targets(sel, language)
            for sel in rule.selectors
            for language in LANGUAGES
        ):
            findings.append(
                f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` turns text-autospace off"
            )
    for language in ("ja", "zh-Hans", "zh-Hant"):
        if language not in used:
            continue
        spaced = any(
            (rule.value("text-autospace") or "no-autospace").strip().lower()
            != "no-autospace"
            and any(is_global(sel) or targets(sel, language) for sel in rule.selectors)
            for rule in scan.rules
            if not rule.at
        )
        if not spaced:
            findings.append(f"{language} is used and no rule sets text-autospace")
    return Outcome(findings, examined(scan))


def dynamic_children(element) -> bool:
    for child in element.children:
        if isinstance(child, str):
            if "{" in child:
                return True
        elif child.name[:1].isupper() or ":" in child.name:
            return True
    return False


def check_ruby_conformance(scan: Scan) -> Outcome:
    findings: list[str] = []
    for source, markup in scan.texts():
        for element in markup.elements:
            if element.name in ("rb", "rtc"):
                findings.append(
                    f"{source.at(element.start)} <{element.name}> is obsolete in HTML; put the base text directly in <ruby>"
                )
            elif element.name in ("rt", "rp"):
                parent = element.parent
                if parent is None or parent.name not in ("ruby", "rtc"):
                    findings.append(
                        f"{source.at(element.start)} <{element.name}> sits outside a <ruby>"
                    )
            elif element.name == "ruby":
                has_rt = any(inner.name == "rt" for inner in element.descendants())
                if not has_rt and not dynamic_children(element):
                    findings.append(f"{source.at(element.start)} <ruby> has no <rt>")
    return Outcome(findings, len(scan.markup))


def ruby_size(value: str | None) -> tuple[bool, str] | None:
    """(too small, the value) for a ruby font-size a static read can judge."""
    if not value:
        return None
    value = value.strip().lower()
    match = re.fullmatch(r"(\d*\.?\d+)(%|em|rem|px)", value)
    if not match:
        return None
    number = float(match.group(1))
    small = {
        "%": number < 50,
        "em": number < 0.5,
        "rem": number < 0.5,
        "px": number < 10,
    }[match.group(2)]
    return small, value


def check_ruby_legible(scan: Scan) -> Outcome:
    findings: list[str] = []
    for rule in scan.rules:
        if rule.at:
            continue
        for selector in rule.selectors:
            if not re.search(r"(?:^|[\s>+~])rt$", selector.strip()):
                continue
            judged = ruby_size(rule.value("font-size"))
            if judged and judged[0]:
                findings.append(
                    f"{rule.source.at(rule.offset)} `{selector}` sets ruby text to {judged[1]}; keep it at half "
                    "the base size or larger"
                )
    placed = any(
        rule_sets(rule, "ruby-position", "inter-character")
        for rule in scan.rules
        if not rule.at
    )
    if not placed:
        for source, markup in scan.texts():
            for element in markup.elements:
                if element.name != "rt":
                    continue
                reading = " ".join(element.text().split())
                if BOPOMOFO_RE.search(reading):
                    findings.append(
                        f"{source.at(element.start)} bopomofo ruby {reading!r} and no rule sets ruby-position: "
                        "inter-character, so it stacks above the base instead of beside it"
                    )
    return Outcome(findings, len(scan.markup) + scan.style_read)


TOKEN_STRIP = "\"'`,.;:!?()[]{}<>「」『』（）"


def check_pinyin_nfc(scan: Scan) -> Outcome:
    findings: list[str] = []
    sources: list[tuple[str, str]] = [
        (source.rel, source.text) for source in scan.markup
    ]
    sources += [(source.rel, source.text) for source in scan.config]
    for rel, path in walk_extra(scan.base, scan.root, TEXT_EXT):
        try:
            if path.stat().st_size <= FONT_LIMIT:
                sources.append(
                    (rel, path.read_bytes().decode("utf-8", errors="replace"))
                )
        except OSError:
            continue
    for rel, text in sources:
        for number, line in enumerate(text.split("\n"), start=1):
            if unicodedata.is_normalized("NFC", line):
                continue
            for token in line.split():
                token = token.strip(TOKEN_STRIP)
                if token and not unicodedata.is_normalized("NFC", token):
                    findings.append(
                        f"{rel}:{number} {unicodedata.normalize('NFC', token)!r} is not in Unicode NFC; store it composed"
                    )
                    break
    return Outcome(findings, len(sources))


CHECKS: dict[str, Callable[[Scan], Outcome]] = {
    "cjk-lang": check_cjk_lang,
    "zh-script-subtag": check_zh_script_subtag,
    "cjk-font-stack": check_cjk_font_stack,
    "cjk-font-subset": check_cjk_font_subset,
    "cjk-font-display": check_cjk_font_display,
    "kinsoku": check_kinsoku,
    "korean-keep-all": check_korean_keep_all,
    "no-break-all": check_no_break_all,
    "phrase-headings": check_phrase_headings,
    "cjk-line-height": check_cjk_line_height,
    "mixed-script-autospace": check_mixed_script_autospace,
    "ruby-conformance": check_ruby_conformance,
    "ruby-legible": check_ruby_legible,
    "pinyin-nfc": check_pinyin_nfc,
}


def run_check(root: Path, name: str) -> int:
    try:
        outcome = CHECKS[name](scan(root))
    except Void as void:
        print(f"{name}: VOID {void}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    top = argparse.ArgumentParser(
        description="Judge a web tree's CJK typography, statically.", parents=[common]
    )
    verbs = top.add_subparsers(dest="verb", required=True)
    check = verbs.add_parser("check", parents=[common], help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    verbs.add_parser("classes", help="print the classes, one per line")
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for name in CLASSES:
            print(name)
        return EXIT_GREEN
    root = Path(getattr(args, "root", Path("."))).resolve()
    if not root.is_dir():
        print(
            f"cjk-typography-probe: --root {root} is not a directory", file=sys.stderr
        )
        return EXIT_USAGE
    return run_check(root, args.klass)


if __name__ == "__main__":
    sys.exit(main())

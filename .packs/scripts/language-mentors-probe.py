#!/usr/bin/env python3
"""language-mentors-probe: the language checks of DeckStreak's mentor templates and outputs (SPEC-V2-2213).

The packs/language-mentors pack ships five mentor TEMPLATES (zh, ko, ja, fr, es) built to
persona-core's contract v1. This script checks what only a language pack can check about them and
about the texts an engine writes from them:

- the language of instruction holds the band's mode;
- the reading's new words are glossed;
- the grammar spotlight is one point with a target-language example;
- the pronunciation notes carry each language's notation and get it RIGHT: pinyin tone marks,
  furigana ruby, pitch accent, batchim, Revised Romanization, French liaison, Spanish stress and
  the tilde;
- the culture notes;
- the writing tutor's corrections follow the band's feedback policy.

It REUSES persona-core and copies nothing:

- files are found and parsed only through `persona-core-probe.py`'s `discover` and
  `parse_document`, loaded with importlib from beside this script;
- the CEFR band-to-mode map is read from persona-core's `contract.json`.

A broken persona file is persona-core's finding. It is also a finding here, never a skip.

Every language rule is DATA in the pack's `languages.json`, the rules table, and this script
branches on no language code. A language is judged by a class when its table entry names that
class's notation. Adding Cantonese or German is a table edit plus, at most, a new notation
validator.

Classes (`check <class>`) print one line per finding, `<class>: <finding>`, and end with
`examined N`: the language persona documents read to decide, templates and outputs alike, the way
`stack-probe.py` counts what it read. `rules-table` counts the table's languages.

Exit codes:
- 0: green;
- 1: at least one finding;
- 2: usage;
- 3: VOID, meaning nothing was examined, or an input (the table, the contract, persona-core's probe)
  could not be read. VOID is never a pass.

Standard library only, so it vendors into any repository beside persona-core's probe.

Usage:
    language-mentors-probe.py classes
    language-mentors-probe.py --root R [--subject DIR]... [--table FILE] check <class>
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, Sequence

HERE = Path(__file__).resolve().parent
CORE_PATH = HERE / "persona-core-probe.py"
DEFAULT_TABLE = HERE.parent / "skills" / "packs" / "language-mentors" / "languages.json"
TABLE_SCHEMA = "phx.language-mentors.table.v1"
LANGUAGE_KIND = "language"

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

CLASSES = (
    "mentor-templates",
    "rules-table",
    "cefr-ratio",
    "i1-glosses",
    "i1-density",
    "grammar-spotlight",
    "pronunciation-notation",
    "pinyin-tones",
    "furigana-ruby",
    "pitch-accent",
    "hangul-batchim",
    "romanization-rr",
    "french-liaison",
    "spanish-stress",
    "culture-anchor",
    "culture-generalization",
    "write-corrections",
    "write-focus",
)
NOTATIONS = (
    "pinyin-tones",
    "ruby",
    "pitch-accent",
    "batchim",
    "romanization",
    "liaison",
    "stress",
)
RULE_KINDS = (
    "punctuation",
    "right-in",
    "left-in",
    "left-after",
    "phrase-in",
    "left-suffix",
)
LIAISON_CATEGORIES = ("obligatoire", "facultative", "interdite")
STRESS_CLASSES = ("aguda", "llana", "esdrújula", "sobresdrújula", "monosílabo")
UNITS = ("char", "token")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

# The four Hanyu Pinyin tone marks as combining characters (the Scheme's ˉ ˊ ˇ ˋ).
TONE_MARKS = {"̄": 1, "́": 2, "̌": 3, "̀": 4}

FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
HEADING = re.compile(r"^ {0,3}#{1,6}(?:[ \t]|$)")
POINT = re.compile(r"^ {0,3}###(?:[ \t]|$)")
QUOTE = re.compile(r"^ {0,3}>[ \t]?(?P<text>.*)$")
TABLE_ROW = re.compile(r"^\s*\|")
TABLE_RULE = re.compile(r"^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)*\|?\s*$")
ITEM = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+(?P<text>.*)$")
COMMENT = re.compile(r"<!--.*?-->", re.S)
RUBY = re.compile(r"<ruby\b[^>]*>(?P<inner>.*?)</ruby\s*>", re.S | re.I)
RUBY_PART = re.compile(r"<\s*(?P<close>/?)\s*(?P<tag>rt|rp|rb)\b[^>]*>", re.I)
RT = re.compile(r"<rt\b[^>]*>(?P<text>.*?)</rt\s*>", re.S | re.I)
RP = re.compile(r"<rp\b[^>]*>(?P<text>.*?)</rp\s*>", re.S | re.I)
RUBY_TAG = re.compile(r"<\s*(?P<close>/?)\s*(?P<tag>ruby|rt|rp)\b[^>]*>", re.I)
TAG = re.compile(r"<[^>\n]*>")
INLINE_CODE = re.compile(r"`[^`\n]*`")
LINK = re.compile(r"!?\[([^\]\n]*)\]\([^)\n]*\)")
EMPHASIS = re.compile(r"\*\*|__|~~|\*")
URL = re.compile(r"https?://\S+")
SENTENCE = re.compile(r"[.!?。！？]+")
PAREN = re.compile(r"[（(](?P<inner>[^()（）\n]*)[)）]")
LETTERS = "A-Za-zÀ-ÖØ-öø-ɏḀ-ỿ"
LATIN_WORD = re.compile(rf"[{LETTERS}][{LETTERS}̀-ͯ]*")
PINYIN_TOKEN = re.compile(
    rf"[{LETTERS}][{LETTERS}̀-ͯ]*(?:['’-][{LETTERS}][{LETTERS}̀-ͯ]*)*"
)
WORD = re.compile(r"[^\W\d_]+", re.U)
HANGUL = re.compile(r"[가-힣]+")
PRONOUNCED = re.compile(r"\[(?P<inner>[가-힣ː:\s]+)\]")
JAMO_STATEMENT = re.compile(r"(?P<final>[ㄱ-ㅎ])\s*(?:→|->)\s*\[(?P<said>[ㄱ-ㅎ])\]")
CATEGORY_TAG = re.compile(r"\[[a-z0-9]+(?:-[a-z0-9]+)*\]")
SPANISH_SPLIT = re.compile(rf"^[{LETTERS}]+(?:[-·][{LETTERS}]+)+$")


class Void(Exception):
    """An input could not be read, so the class judges nothing: exit 3, never a pass."""


@dataclass(frozen=True)
class Doc:
    """A language persona document that persona-core parsed. Line numbers are 1-based."""

    path: Path
    shown: str
    kind: str
    frontmatter: dict
    sections: dict
    order: tuple
    body: str
    body_line: int
    code: str


@dataclass(frozen=True)
class Broken:
    """A claimed language file that persona-core could not parse; every class names it."""

    shown: str
    reason: str


@dataclass
class Context:
    root: Path
    docs: list
    broken: list
    table: dict
    contract: dict

    def outputs(self) -> list:
        return [doc for doc in self.docs if doc.kind == "output"]

    def templates(self) -> list:
        return [doc for doc in self.docs if doc.kind == "template"]

    def language(self, doc: Doc) -> "dict | None":
        return self.table["languages"].get(doc.code)

    def with_notation(self, notation: str) -> list:
        """The outputs whose language's table entry names `notation`."""
        found = []
        for doc in self.outputs():
            entry = self.language(doc)
            if entry is not None and notation in entry.get("notations", ()):
                found.append((doc, entry))
        return found


# --- reading: persona-core, the rules table and the population ----------------------------------


def load_core():
    """persona-core's probe, loaded from beside this script: the contract's parser, never a copy."""
    if not CORE_PATH.is_file():
        raise Void(f"persona-core's probe is missing: {CORE_PATH}")
    spec = importlib.util.spec_from_file_location("persona_core_probe", CORE_PATH)
    if spec is None or spec.loader is None:
        raise Void(f"persona-core's probe does not load: {CORE_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules.setdefault(spec.name, module)
    spec.loader.exec_module(module)
    return module


def load_table(path: Path) -> dict:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"the rules table does not read: {path}: {error}") from error
    if not isinstance(document, dict):
        raise Void(f"the rules table is not a JSON object: {path}")
    return document


def show(root: Path, path: Path) -> str:
    try:
        return str(path.resolve().relative_to(root.resolve()))
    except ValueError:
        return str(path)


RAW_LANGUAGE = re.compile(r'^subject:\s*"language/', re.M)


def load_population(core, root: Path, bases: Sequence) -> tuple:
    """Every claimed language document under `bases`, parsed by persona-core, and the broken ones."""
    docs: list = []
    broken: list = []
    for path in core.discover(list(bases)):
        shown = show(root, path)
        try:
            parsed = core.parse_document(path)
        except core.ContractError as error:
            text = path.read_bytes().decode("utf-8", errors="replace")
            if RAW_LANGUAGE.search(text):
                broken.append(Broken(shown, str(error)))
            continue
        subject = parsed.frontmatter.get("subject")
        if core.subject_kind(subject) != LANGUAGE_KIND:
            continue
        sections: dict = {}
        for item in parsed.sections:
            sections.setdefault(item.id, item)
        docs.append(
            Doc(
                path=path,
                shown=shown,
                kind=parsed.kind,
                frontmatter=parsed.frontmatter,
                sections=sections,
                order=tuple(parsed.sections),
                body=parsed.body,
                body_line=parsed.body_line,
                code=subject.split("/", 1)[1],
            )
        )
    return docs, broken


# --- text: what a reader sees, and which language it is in --------------------------------------


def outside_code(text: str) -> list:
    """(0-based index, line) for every line of `text` outside fenced code."""
    kept = []
    fence = None
    for index, line in enumerate(text.split("\n")):
        opened = FENCE.match(line)
        if fence is not None:
            if (
                opened
                and opened.group(1)[0] == fence[0]
                and len(opened.group(1)) >= len(fence)
            ):
                fence = None
            continue
        if opened:
            fence = opened.group(1)
            continue
        kept.append((index, line))
    return kept


def ruby_base(match: "re.Match") -> str:
    return RP.sub("", RT.sub("", match.group("inner")))


def ruby_reading(match: "re.Match") -> str:
    return "".join(part.group("text") for part in RT.finditer(match.group("inner")))


def base_text(text: str) -> str:
    """Markdown and markup reduced to the words a reader sees; a ruby keeps its base only."""
    text = COMMENT.sub(" ", text)
    text = RUBY.sub(ruby_base, text)
    text = TAG.sub(" ", text)
    text = INLINE_CODE.sub(" ", text)
    text = LINK.sub(r"\1", text)
    text = URL.sub(" ", text)
    return EMPHASIS.sub("", text)


def char_class(ranges: Iterable) -> "re.Pattern":
    parts = "".join(
        f"{re.escape(chr(low))}-{re.escape(chr(high))}" for low, high in ranges
    )
    return re.compile(f"[{parts}]")


def cells(line: str) -> list:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def entries(lines: list) -> list:
    """(line, text) of each list item and each table data row.

    A list item's indented continuation lines belong to it, as CommonMark reads them.
    """
    found: list = []
    continuing = False
    for index, line in lines:
        if TABLE_ROW.match(line):
            continuing = False
            if not TABLE_RULE.match(line):
                found.append([index, line])
            continue
        item = ITEM.match(line)
        if item:
            found.append([index, item.group("text")])
            continuing = True
            continue
        if (
            continuing
            and line.strip()
            and line[:1] in (" ", "\t")
            and not HEADING.match(line)
        ):
            found[-1][1] += " " + line.strip()
            continue
        continuing = False
    return [(index, text) for index, text in found]


def tables(lines: list) -> list:
    """Each markdown table as (header cells, [(0-based index, cells), ...])."""
    found = []
    block: list = []
    for index, line in lines + [(-1, "")]:
        if TABLE_ROW.match(line):
            block.append((index, line))
            continue
        if len(block) >= 2 and TABLE_RULE.match(block[1][1]):
            header = [cell.lower() for cell in cells(block[0][1])]
            found.append((header, [(at, cells(text)) for at, text in block[2:]]))
        block = []
    return found


class Language:
    """One table entry's view of text: which words are the target language and which are English."""

    def __init__(self, entry: dict, table: dict) -> None:
        self.entry = entry
        script = entry["script"]
        self.unit = script["unit"]
        self.per_word = float(script.get("chars_per_word", 1.0))
        self.script = char_class(script["ranges"]) if script.get("ranges") else None
        self.words = {word.lower() for word in entry.get("words", ())}
        self.letters = set(entry.get("letters", ""))
        self.english = {word.lower() for word in table["english"]["words"]}
        self.pinyin = (
            entry.get("pinyin")
            if "pinyin-tones" in entry.get("notations", ())
            else None
        )
        kana = entry.get("kana")
        self.kana = char_class(kana["ranges"]) if kana else None
        romanization = entry.get("romanization")
        self.romanization = rr_pattern(romanization) if romanization else None

    def is_notation(self, token: str) -> bool:
        """A romanization or reading token beside the script it annotates: not English."""
        if self.pinyin is not None:
            base, _marks = strip_tones(token)
            if parse_pinyin(base, self.pinyin) is not None:
                return True
        if self.kana is not None and token and all(self.kana.match(ch) for ch in token):
            return True
        if self.romanization is not None and self.romanization.match(
            token.lower().replace("-", "")
        ):
            return True
        return False

    def units(self, text: str, cell: bool = False) -> tuple:
        """(target-language, English) word units in `text`, already reduced to what a reader sees.

        In a table cell, a token shaped like this language's notation (pinyin, kana, Revised
        Romanization) is notation, not English, as it is in parentheses after the script.
        """
        if self.script is not None:
            return self._script_units(text, cell)
        tl = en = 0.0
        for sentence in SENTENCE.split(text):
            got_tl, got_en = self._sentence_units(sentence)
            tl += got_tl
            en += got_en
        return tl, en

    def _script_units(self, text: str, cell: bool) -> tuple:
        def unwrap(match: "re.Match") -> str:
            before = text[: match.start()].rstrip()
            if not before or not self.script.match(before[-1]):
                return match.group(0)
            kept = [
                token
                for token in match.group("inner").split()
                if not self.is_notation(token.strip(".,;:，、。"))
            ]
            return " " + " ".join(kept) + " "

        text = PAREN.sub(unwrap, text)
        if self.unit == "char":
            tl = len(self.script.findall(text)) / self.per_word
        else:
            tl = float(sum(1 for token in text.split() if self.script.search(token)))
        en = 0.0
        for token in LATIN_WORD.findall(text):
            if self.pinyin is not None and any(
                mark in unicodedata.normalize("NFD", token) for mark in TONE_MARKS
            ):
                continue
            if cell and self.is_notation(token):
                continue
            en += 1
        return tl, en

    def classify(self, token: str) -> str:
        word = token.lower()
        in_en = word in self.english
        in_tl = word in self.words or (
            not in_en and any(ch in self.letters for ch in word)
        )
        if in_tl and not in_en:
            return "tl"
        if in_en and not in_tl:
            return "en"
        return "?"

    def _sentence_units(self, sentence: str) -> tuple:
        tokens = [
            piece
            for word in LATIN_WORD.findall(sentence)
            for piece in re.split(r"['’]", word)
            if piece
        ]
        kinds = [self.classify(token) for token in tokens]
        tl = kinds.count("tl")
        en = kinds.count("en")
        unknown = kinds.count("?")
        if tl > en:
            tl += unknown
        elif en > tl:
            en += unknown
        return float(tl), float(en)

    def tl_terms(self, text: str) -> int:
        """How many target-language terms `text` holds, sentence majority aside."""
        if self.script is not None:
            return len(self.script.findall(text))
        return sum(
            1 for token in LATIN_WORD.findall(text) if self.classify(token) == "tl"
        )


def instruction(doc: Doc, table: dict) -> list:
    """(text, is a table cell) of what a mentor explains in: every section but the material ones.

    Quotes, code and headings are left out, and so is a `[category]` tag, which is notation.
    """
    material = set(table["material_sections"])
    lines = doc.body.split("\n")
    first = min((item.line for item in doc.order), default=None)
    chunks = measurable(lines[: first - doc.body_line] if first else lines)
    for item in doc.order:
        if item.id not in material:
            chunks += measurable(item.body.split("\n"))
    return chunks


def measurable(lines: list) -> list:
    chunks = []
    for _index, line in outside_code("\n".join(lines)):
        if (
            not line.strip()
            or HEADING.match(line)
            or QUOTE.match(line)
            or TABLE_RULE.match(line)
        ):
            continue
        if TABLE_ROW.match(line):
            chunks += [(cell, True) for cell in cells(line) if cell]
            continue
        item = ITEM.match(line)
        chunks.append((item.group("text") if item else line, False))
    return [(CATEGORY_TAG.sub(" ", base_text(chunk)), cell) for chunk, cell in chunks]


def section_lines(doc: Doc, section_id: str) -> list:
    """(1-based file line, line) of a section's body outside fenced code."""
    item = doc.sections.get(section_id)
    if item is None:
        return []
    return [(item.line + 1 + index, line) for index, line in outside_code(item.body)]


def body_lines(doc: Doc) -> list:
    return [(doc.body_line + index, line) for index, line in outside_code(doc.body)]


def mode_of(ctx: Context, doc: Doc) -> "str | None":
    return ctx.contract.get("cefr", {}).get(doc.frontmatter.get("cefr"))


# --- the rules table ------------------------------------------------------------------------------


def is_fraction(value: object) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and 0 <= value <= 1
    )


def strings(value: object) -> bool:
    return (
        isinstance(value, list)
        and bool(value)
        and all(isinstance(item, str) and item for item in value)
    )


def table_problems(table: dict, contract: dict) -> list:
    """Every defect of the rules table; any one of them makes the language classes VOID."""
    problems: list = []
    if table.get("schema") != TABLE_SCHEMA:
        problems.append(f"schema is {table.get('schema')!r}, not {TABLE_SCHEMA}")
    ratio = table.get("ratio")
    for mode in sorted(set(contract.get("cefr", {}).values())):
        bounds = ratio.get(mode) if isinstance(ratio, dict) else None
        if not isinstance(bounds, dict) or not (
            is_fraction(bounds.get("min")) and is_fraction(bounds.get("max"))
        ):
            problems.append(
                f"ratio names no [min, max] target-language share for the {mode} mode"
            )
        elif bounds["min"] > bounds["max"]:
            problems.append(
                f"ratio for the {mode} mode has min {bounds['min']} above max {bounds['max']}"
            )
    density = table.get("density", {})
    if not (is_fraction(density.get("max_new_share")) and density.get("max_new_share")):
        problems.append("density.max_new_share is not a share above 0")
    focus = table.get("focus", {}).get("max_categories")
    if not isinstance(focus, int) or isinstance(focus, bool) or focus < 1:
        problems.append("focus.max_categories is not a count of at least 1")
    material = table.get("material_sections")
    if not isinstance(material, list) or not all(
        isinstance(sid, str) and SLUG.match(sid) for sid in material
    ):
        problems.append("material_sections is not a list of section ids")
    if not strings(table.get("english", {}).get("words")):
        problems.append("english.words is not a non-empty list of words")
    if not strings(table.get("gloss", {}).get("separators")):
        problems.append("gloss.separators is not a non-empty list")
    correction = table.get("correction", {})
    if (
        not strings(correction.get("arrows"))
        or not isinstance(correction.get("indirect"), str)
        or not correction.get("indirect")
    ):
        problems.append("correction names no arrows or no indirect mark")
    duty_ids = {
        duty.get("id") for duty in contract.get("duties", ()) if isinstance(duty, dict)
    }
    template = table.get("template", {})
    wanted = template.get("duties")
    if not strings(wanted) or not set(wanted) <= duty_ids:
        problems.append(
            f"template.duties is not a list of persona-core duties: {wanted!r}"
        )
    extra = template.get("sections")
    if not isinstance(extra, dict) or not all(
        duty in (wanted or ()) and strings(ids) and all(SLUG.match(sid) for sid in ids)
        for duty, ids in extra.items()
    ):
        problems.append(
            "template.sections is not a map from a template duty to section ids"
        )
    languages = table.get("languages")
    if not isinstance(languages, dict) or not languages:
        problems.append("languages is not a non-empty object")
        return problems
    for code, entry in languages.items():
        problems += [f"{code}: {problem}" for problem in language_problems(entry)]
    return problems


def language_problems(entry: object) -> list:
    if not isinstance(entry, dict):
        return ["the entry is not an object"]
    problems: list = []
    if not isinstance(entry.get("name"), str) or not entry.get("name"):
        problems.append("names no language")
    if not strings(entry.get("lang_tags")):
        problems.append("lang_tags is not a non-empty list")
    script = entry.get("script")
    if not isinstance(script, dict) or script.get("unit") not in UNITS:
        problems.append(f"script.unit is not one of {', '.join(UNITS)}")
        script = {}
    ranges = script.get("ranges")
    if ranges is not None and not (
        isinstance(ranges, list)
        and ranges
        and all(
            isinstance(pair, list)
            and len(pair) == 2
            and all(isinstance(n, int) for n in pair)
            and pair[0] <= pair[1]
            for pair in ranges
        )
    ):
        problems.append("script.ranges is not a list of [low, high] code points")
    if script.get("unit") == "char" and (
        not ranges
        or not isinstance(script.get("chars_per_word"), (int, float))
        or script.get("chars_per_word", 0) <= 0
    ):
        problems.append("a char unit needs script.ranges and a chars_per_word above 0")
    if ranges is None and (
        not strings(entry.get("words"))
        or not isinstance(entry.get("letters"), str)
        or not isinstance(entry.get("articles"), list)
    ):
        problems.append("a script-less language needs words, letters and articles")
    if not strings(entry.get("method_terms")):
        problems.append("method_terms is not a non-empty list")
    notations = entry.get("notations")
    if not isinstance(notations, list):
        problems.append("notations is not a list")
        notations = []
    for notation in notations:
        if notation not in NOTATIONS:
            problems.append(
                f"notation {notation!r} is not one of {', '.join(NOTATIONS)}"
            )
    for notation in entry.get("required_notations", []):
        if notation not in notations:
            problems.append(
                f"required notation {notation!r} is not among its notations"
            )
    if entry.get("density_unit") not in UNITS:
        problems.append(f"density_unit is not one of {', '.join(UNITS)}")
    taxonomy = entry.get("taxonomy")
    seen: set = set()
    if not isinstance(taxonomy, list) or not taxonomy:
        problems.append("taxonomy is not a non-empty list")
        taxonomy = []
    for category in taxonomy:
        if (
            not isinstance(category, dict)
            or not isinstance(category.get("id"), str)
            or not SLUG.match(category["id"])
            or not isinstance(category.get("treatable"), bool)
        ):
            problems.append(f"taxonomy entry {category!r} is not {{id, treatable}}")
            continue
        if category["id"] in seen:
            problems.append(f"category {category['id']} is listed twice")
        seen.add(category["id"])
    if not isinstance(entry.get("demonyms"), list) or not isinstance(
        entry.get("generalizations"), list
    ):
        problems.append("demonyms and generalizations are not lists")
    problems += notation_problems(entry, notations)
    return problems


def notation_problems(entry: dict, notations: list) -> list:
    problems: list = []
    if "pinyin-tones" in notations:
        pinyin = entry.get("pinyin", {})
        if (
            not all(
                strings(pinyin.get(key))
                for key in ("initials", "finals", "first", "u_after")
            )
            or not all(
                isinstance(pinyin.get(key), str) and pinyin.get(key)
                for key in ("initialless", "vowels")
            )
            or not isinstance(pinyin.get("pairs"), dict)
        ):
            problems.append(
                "pinyin needs initials, finals, first, u_after, initialless, vowels and pairs"
            )
    if {"ruby", "pitch-accent"} & set(notations):
        kana = entry.get("kana", {})
        if (
            not isinstance(kana.get("ranges"), list)
            or not all(
                isinstance(kana.get(key), str)
                for key in ("small", "special", "kanji_extra")
            )
            or not strings(kana.get("downstep"))
        ):
            problems.append(
                "kana needs ranges, small, special, downstep and kanji_extra"
            )
    if "batchim" in notations:
        problems += batchim_problems(entry.get("hangul", {}))
    if "romanization" in notations:
        romanization = entry.get("romanization", {})
        if not all(
            strings(romanization.get(key))
            for key in ("header", "vowels", "initials", "finals")
        ):
            problems.append("romanization needs header, vowels, initials and finals")
    if "liaison" in notations:
        problems += liaison_problems(entry.get("liaison", {}))
    if "stress" in notations:
        stress = entry.get("stress", {})
        classes = stress.get("classes")
        if (
            not isinstance(classes, dict)
            or sorted(classes) != sorted(STRESS_CLASSES)
            or not all(strings(words) for words in classes.values())
        ):
            problems.append(f"stress.classes does not name {', '.join(STRESS_CLASSES)}")
        if (
            not all(
                isinstance(stress.get(key), str) and stress.get(key)
                for key in (
                    "strong",
                    "weak",
                    "open_ending",
                    "silent_u_before",
                    "unjudged_suffix",
                )
            )
            or not isinstance(stress.get("accented"), dict)
            or not strings(stress.get("diacritic"))
            or not isinstance(stress.get("optional_diacritic"), list)
        ):
            problems.append(
                "stress needs strong, weak, accented, open_ending, diacritic, optional_diacritic, silent_u_before and unjudged_suffix"
            )
    return problems


def batchim_problems(hangul: dict) -> list:
    finals = hangul.get("finals")
    representatives = hangul.get("representatives")
    batchim = hangul.get("batchim")
    if not isinstance(finals, list) or len(finals) != 28 or finals[0] != "":
        return ["hangul.finals is not the 28 final slots, the empty one first"]
    if not isinstance(representatives, list) or len(representatives) != 7:
        return [
            "hangul.representatives is not the seven representative finals of Article 8"
        ]
    if not isinstance(batchim, dict):
        return ["hangul.batchim is not a map from each final to its representative"]
    problems = []
    for final in finals[1:]:
        if final not in batchim:
            problems.append(f"batchim names no representative for {final}")
    for final, said in batchim.items():
        if said not in representatives:
            problems.append(
                f"batchim {final} → {said} is not one of the seven representative finals"
            )
    return problems


def liaison_problems(liaison: dict) -> list:
    problems = []
    categories = liaison.get("categories")
    if (
        not isinstance(categories, dict)
        or sorted(categories) != sorted(LIAISON_CATEGORIES)
        or not all(strings(words) for words in categories.values())
    ):
        problems.append(
            f"liaison.categories does not name {', '.join(LIAISON_CATEGORIES)}"
        )
    if not strings(liaison.get("junctions")):
        problems.append("liaison.junctions is not a non-empty list")
    rules = liaison.get("rules")
    if not isinstance(rules, list) or not rules:
        return problems + ["liaison.rules is not a non-empty list"]
    for index, rule in enumerate(rules):
        if (
            not isinstance(rule, dict)
            or rule.get("category") not in LIAISON_CATEGORIES
            or rule.get("kind") not in RULE_KINDS
        ):
            problems.append(f"liaison rule {index} names no known category and kind")
            continue
        if rule["kind"] in (
            "right-in",
            "left-in",
            "left-after",
            "phrase-in",
        ) and not strings(rule.get("words")):
            problems.append(f"liaison rule {index} ({rule['kind']}) names no words")
        if rule["kind"] in ("left-after", "left-suffix") and not (
            isinstance(rule.get("suffix"), str) and rule.get("suffix")
        ):
            problems.append(f"liaison rule {index} ({rule['kind']}) names no suffix")
    return problems


# --- notation: pinyin, ruby and pitch accent, hangul, romanization, liaison, stress -------------


def strip_tones(token: str) -> tuple:
    """A pinyin token lower-cased without tone marks, and each mark as (letter index, tone)."""
    letters: list = []
    marks: list = []
    for ch in unicodedata.normalize("NFD", token.lower()):
        if ch in TONE_MARKS:
            marks.append((len(letters) - 1, TONE_MARKS[ch]))
        elif unicodedata.combining(ch) and letters:
            letters[-1] += ch
        else:
            letters.append(ch)
    return "".join(unicodedata.normalize("NFC", letter) for letter in letters), marks


def parse_pinyin(base: str, data: dict) -> "list | None":
    """Split tone-stripped pinyin into syllable spans, or None when it is not pinyin at all.

    Chunks split on apostrophes and hyphens. An initial-less syllable opens a chunk only, because
    the orthography writes an apostrophe before one (GB/T 16159-2012), and it starts with a, o, e.
    """
    initials = sorted(data["initials"], key=len, reverse=True) + [""]
    finals = sorted(data["finals"], key=len, reverse=True)
    spans: list = []
    offset = 0
    for chunk in re.split(r"(['’-])", base):
        if chunk in ("'", "’", "-"):
            offset += 1
            continue
        if not chunk:
            return None
        found = _syllables(chunk, 0, True, initials, finals, data, {})
        if found is None:
            return None
        spans += [(offset + start, offset + end) for start, end in found]
        offset += len(chunk)
    return spans


def _syllables(
    chunk: str, at: int, first: bool, initials, finals, data, memo
) -> "list | None":
    if at == len(chunk):
        return []
    key = (at, first)
    if key in memo:
        return memo[key]
    result = None
    for initial in initials:
        if initial and not chunk.startswith(initial, at):
            continue
        if not initial and not first:
            continue
        start = at + len(initial)
        for final in finals:
            if not chunk.startswith(final, start):
                continue
            if not initial and final[0] not in data["initialless"]:
                continue
            end = start + len(final)
            ends = [end]
            erhua = data.get("erhua", "")
            if erhua and chunk.startswith(erhua, end):
                ends.insert(0, end + len(erhua))
            for stop in ends:
                rest = _syllables(chunk, stop, False, initials, finals, data, memo)
                if rest is not None:
                    result = [(at, stop)] + rest
                    break
            if result is not None:
                break
        if result is not None:
            break
    memo[key] = result
    return result


def mark_position(syllable: str, data: dict) -> "int | None":
    """Where the tone mark goes: a, e (ê); o in ou; else the last vowel (iu and ui: the second)."""
    for vowel in data["first"]:
        found = syllable.find(vowel)
        if found >= 0:
            return found
    for pair, vowel in data["pairs"].items():
        found = syllable.find(pair)
        if found >= 0:
            return found + pair.index(vowel)
    positions = [index for index, ch in enumerate(syllable) if ch in data["vowels"]]
    return positions[-1] if positions else None


def marked(syllable: str, marks: list) -> str:
    """A tone-stripped syllable with its marks put back where they were written."""
    reverse = {tone: mark for mark, tone in TONE_MARKS.items()}
    letters = list(syllable)
    for index, tone in marks:
        if 0 <= index < len(letters):
            letters[index] = unicodedata.normalize(
                "NFC", letters[index] + reverse[tone]
            )
    return "".join(letters)


def pinyin_tokens(text: str, data: dict) -> list:
    """(token, [(syllable, [(index in syllable, tone)])]) for each tone-marked pinyin word in text."""
    found = []
    for token in PINYIN_TOKEN.findall(text):
        base, marks = strip_tones(token)
        if not marks:
            continue
        spans = parse_pinyin(base, data)
        if spans is None:
            continue
        syllables = []
        for start, end in spans:
            inside = [
                (index - start, tone) for index, tone in marks if start <= index < end
            ]
            syllables.append((base[start:end], inside))
        found.append((token, syllables))
    return found


def pinyin_findings(text: str, data: dict) -> list:
    findings = []
    for _token, syllables in pinyin_tokens(text, data):
        for syllable, marks in syllables:
            shown = marked(syllable, marks)
            if len(marks) > 1:
                findings.append(
                    f"{shown}: {len(marks)} tone marks in one syllable; a syllable carries one"
                )
                continue
            if marks:
                expected = mark_position(syllable, data)
                if expected is not None and marks[0][0] != expected:
                    findings.append(
                        f"{shown}: the tone mark is on {syllable[marks[0][0]]}; GB/T 16159-2012 6.5.1 puts it on {syllable[expected]}"
                    )
            for initial in data["u_after"]:
                if syllable.startswith(initial + "ü"):
                    findings.append(
                        f"{shown}: ü is written u after {', '.join(data['u_after'])} (the Scheme's rule)"
                    )
    return findings


def ruby_findings(
    text: str, kana: "re.Pattern", kanji: "re.Pattern", extra: str
) -> list:
    findings = []
    depth = 0
    open_part = None
    for tag in RUBY_TAG.finditer(text):
        name = tag.group("tag").lower()
        closing = bool(tag.group("close"))
        if name == "ruby":
            if closing:
                if depth == 0:
                    findings.append("a </ruby> with no <ruby>")
                depth = max(0, depth - 1)
            else:
                if depth:
                    findings.append("a <ruby> nested in a <ruby>")
                depth += 1
        elif closing:
            if open_part != name:
                findings.append(f"a </{name}> with no <{name}>")
            open_part = None
        else:
            if open_part is not None:
                findings.append(f"a <{name}> inside an unclosed <{open_part}>")
            open_part = name
    if depth:
        findings.append(f"{depth} unclosed <ruby>")
    for match in RUBY.finditer(text):
        base = ""
        pending = ""
        readings = 0
        at = 0
        inner = match.group("inner")
        for part in RUBY_PART.finditer(inner):
            outside = inner[at : part.start()]
            at = part.end()
            name = part.group("tag").lower()
            if part.group("close"):
                continue
            end = re.search(rf"<\s*/\s*{name}\s*>", inner[at:], re.I)
            content = inner[at : at + end.start()] if end else ""
            if end:
                at = at + end.end()
            if name == "rb":
                pending += TAG.sub("", content)
                continue
            pending += TAG.sub("", outside)
            if name == "rp":
                if content.strip() not in ("(", ")", "（", "）"):
                    findings.append(f"<rp>{content}</rp> holds more than a parenthesis")
                continue
            readings += 1
            reading = content.strip()
            if not reading:
                findings.append(f"an empty <rt> on {pending.strip() or match.group(0)}")
            elif not all(kana.match(ch) for ch in reading):
                findings.append(
                    f"<rt>{reading}</rt> is not kana; furigana are written in kana"
                )
            if not pending.strip() and not base:
                findings.append(
                    f"an <rt> with no base text before it in {match.group(0)}"
                )
            base += pending
            pending = ""
        if not readings:
            findings.append(f"{match.group(0)} has no <rt>")
        elif not any(
            (kanji.match(ch) and not kana.match(ch)) or ch in extra for ch in base
        ):
            findings.append(
                f"the ruby base {base.strip()!r} has no kanji; furigana annotate kanji"
            )
    return findings


def morae(kana_text: str, small: str) -> list:
    moras: list = []
    for ch in kana_text:
        if ch in small and moras:
            moras[-1] += ch
        else:
            moras.append(ch)
    return moras


def accent_marks(text: str, kana: dict, kana_class: "re.Pattern") -> list:
    """(kana word, number or None, downstep index list) for every accent notation in `text`."""
    text = TAG.sub(" ", RUBY.sub(ruby_reading, text))
    marks = "".join(re.escape(mark) for mark in kana["downstep"])
    pattern = kana_class.pattern[1:-1]
    found = []
    for match in re.finditer(
        rf"(?P<word>[{pattern}]+)\s*[\[［]\s*(?P<n>[0-9０-９]{{1,2}})\s*[\]］]", text
    ):
        found.append(
            (
                match.group("word"),
                int(unicodedata.normalize("NFKC", match.group("n"))),
                None,
            )
        )
    for match in re.finditer(rf"[{pattern}{marks}]*[{marks}][{pattern}{marks}]*", text):
        run = match.group(0)
        found.append(
            (
                run,
                None,
                [index for index, ch in enumerate(run) if ch in kana["downstep"]],
            )
        )
    return found


def accent_findings(text: str, kana: dict, kana_class: "re.Pattern") -> list:
    findings = []
    for word, number, steps in accent_marks(text, kana, kana_class):
        if number is not None:
            moras = morae(word, kana["small"])
            if number > len(moras):
                findings.append(
                    f"{word} [{number}]: {len(moras)} morae, so no kernel falls after mora {number}"
                )
            elif number >= 1 and moras[number - 1][0] in kana["special"]:
                findings.append(
                    f"{word} [{number}]: mora {number} is {moras[number - 1]}, a special mora that never carries the kernel"
                )
            continue
        if len(steps) > 1:
            findings.append(
                f"{word}: {len(steps)} downstep marks; a word has at most one"
            )
            continue
        before = word[: steps[0]]
        for mark in kana["downstep"]:
            before = before.replace(mark, "")
        moras = morae(before, kana["small"])
        if not moras:
            findings.append(f"{word}: a downstep with no mora before it")
        elif moras[-1][0] in kana["special"]:
            findings.append(
                f"{word}: the downstep follows {moras[-1]}, a special mora that never carries the kernel"
            )
    return findings


def hangul_final(syllable: str, finals: list) -> str:
    return finals[(ord(syllable) - 0xAC00) % 28]


def batchim_pairs(line: str) -> list:
    """(written word, pronunciation) for every `word [pronunciation]` in the standard's notation."""
    found = []
    for match in PRONOUNCED.finditer(line):
        said = [token for token in re.split(r"[\sː:]+", match.group("inner")) if token]
        before = HANGUL.findall(line[: match.start()])
        if not said or len(before) < len(said):
            continue
        found.append((" ".join(before[-len(said) :]), " ".join(said)))
    return found


def batchim_findings(line: str, hangul: dict) -> list:
    findings = []
    finals = hangul["finals"]
    for written, said in batchim_pairs(line):
        letters = written.replace(" ", "")
        sounds = said.replace(" ", "")
        if len(letters) != len(sounds):
            findings.append(
                f"{written} [{said}]: {len(letters)} syllables written and {len(sounds)} pronounced"
            )
            continue
        final = hangul_final(letters[-1], finals)
        if not final:
            continue
        expected = hangul["batchim"].get(final)
        got = hangul_final(sounds[-1], finals)
        if expected is not None and got != expected:
            findings.append(
                f"{written} [{said}]: a word-final {final} is said {expected} (표준 발음법 Articles 8-11), not {got or 'with no final'}"
            )
    for match in JAMO_STATEMENT.finditer(line):
        expected = hangul["batchim"].get(match.group("final"))
        if expected is not None and match.group("said") != expected:
            findings.append(
                f"{match.group(0)}: a final {match.group('final')} is said {expected} (표준 발음법 Articles 8-11)"
            )
    return findings


def rr_pattern(romanization: dict) -> "re.Pattern":
    def either(items: list) -> str:
        return "|".join(
            re.escape(item) for item in sorted(items, key=len, reverse=True)
        )

    initial = either(romanization["initials"])
    vowel = either(romanization["vowels"])
    final = either(romanization["finals"])
    return re.compile(rf"^(?:(?:{initial})?(?:{vowel})(?:{final})?)+$")


def romanizations(doc: Doc, entry: dict, table: dict) -> list:
    """(1-based line, token) of each romanization the output DECLARES: gloss readings and RR columns."""
    found = []
    for line_no, text in entries(section_lines(doc, "glosses")):
        head = gloss_split(text, table)[0]
        for inner in PAREN.finditer(head):
            found += [(line_no, token) for token in inner.group("inner").split()]
    headers = {name.lower() for name in entry["romanization"]["header"]}
    for header, rows in tables(body_lines(doc)):
        columns = [index for index, name in enumerate(header) if name in headers]
        for line_no, row in rows:
            for column in columns:
                if column < len(row):
                    found += [(line_no, token) for token in row[column].split()]
    return found


def rr_findings(token: str, pattern: "re.Pattern") -> list:
    token = token.strip(".,;:!?()（）")
    if not token:
        return []
    stray = sorted(
        {ch for ch in token if not (ch.isascii() and (ch.isalpha() or ch == "-"))}
    )
    if stray:
        return [
            f"{token}: not Revised Romanization, which writes Roman letters only; it holds {''.join(stray)!r}"
        ]
    if not pattern.match(token.lower().replace("-", "")):
        return [f"{token}: does not parse as Revised Romanization syllables"]
    return []


def category_of(line: str, categories: dict) -> "str | None":
    found = {
        category
        for category, words in categories.items()
        for word in words
        if re.search(rf"(?<![^\W\d_]){re.escape(word)}(?![^\W\d_])", line, re.I)
    }
    return found.pop() if len(found) == 1 else None


def liaison_phrase(line: str, junctions: list) -> "tuple | None":
    """(left, marker, right) around the first junction mark in the row's phrase cell."""
    parts = cells(line) if TABLE_ROW.match(line) else re.split(r"\s+[—–:-]\s+", line)
    for part in parts:
        for marker in junctions:
            if marker in part:
                left, right = part.split(marker, 1)
                return left, marker, right
    return None


def liaison_expected(left: str, right: str, rules: list) -> "tuple | None":
    left_words = WORD.findall(left.lower().replace("’", "'"))
    right_words = re.findall(r"[^\W\d_]+(?:-[^\W\d_]+)*", right.lower())
    if not left_words or not right_words:
        return None
    first, last = right_words[0], left_words[-1]
    previous = left_words[-2] if len(left_words) > 1 else None
    phrase = " ".join(
        (left + " " + right).lower().replace("’", "'").replace("-", " ").split()
    )
    paused = left.rstrip()[-1:] in ",;:.!?" or right.lstrip()[:1] in ",;:.!?"
    for rule in rules:
        kind = rule["kind"]
        words = set(rule.get("words", ()))
        if kind == "punctuation":
            hit = paused
        elif kind == "right-in":
            hit = first in words or first.rstrip("sx") in words
        elif kind == "left-in":
            hit = last in words
        elif kind == "left-after":
            hit = previous in words and last.endswith(rule["suffix"])
        elif kind == "phrase-in":
            hit = any(
                re.search(rf"(?<![^\W\d_]){re.escape(words_)}(?![^\W\d_])", phrase)
                for words_ in words
            )
        else:
            hit = last.endswith(rule["suffix"]) and len(last) > len(rule["suffix"]) + 2
        if hit:
            return rule["category"], rule.get("source", kind)
    return None


def fold(text: str) -> str:
    return "".join(
        ch for ch in unicodedata.normalize("NFD", text) if not unicodedata.combining(ch)
    )


def spanish_nuclei(word: str, stress: dict) -> list:
    """The vowel nuclei of a Spanish word, as lists of letter indices (RAE OLE 2010, §3.4)."""
    letters = list(unicodedata.normalize("NFC", word.lower()))
    vowels = (
        set(stress["strong"]) | set(stress["weak"]) | set(stress["accented"]) | {"ü"}
    )
    vowel = [ch in vowels for ch in letters]
    if len(letters) > 1 and letters[-1] == "y" and vowel[-2]:
        vowel[-1] = True
    for index, ch in enumerate(letters):
        if (
            ch == "u"
            and 0 < index < len(letters) - 1
            and letters[index - 1] in "qg"
            and letters[index + 1] in stress["silent_u_before"]
        ):
            vowel[index] = False
    strong = set(stress["strong"]) | {
        accented for accented, plain in stress["accented"].items() if plain in "iu"
    }
    runs: list = []
    index = 0
    while index < len(letters):
        if not vowel[index]:
            index += 1
            continue
        run = [index]
        index += 1
        while index < len(letters):
            if vowel[index]:
                run.append(index)
                index += 1
            elif (
                letters[index] == "h" and index + 1 < len(letters) and vowel[index + 1]
            ):
                index += 1
            else:
                break
        runs.append(run)
    nuclei: list = []
    for run in runs:
        current = [run[0]]
        for one, two in zip(run, run[1:]):
            a, b = letters[one], letters[two]
            apart = (a in strong and b in strong) or (
                a not in strong and b not in strong and a == b
            )
            if apart:
                nuclei.append(current)
                current = [two]
            else:
                current.append(two)
        nuclei.append(current)
    return nuclei


def spanish_findings(
    word: str, declared: str, split: "list | None", stress: dict
) -> list:
    """What is wrong with one stress row: the class, the tilde, or the syllable split."""
    lower = unicodedata.normalize("NFC", word.lower())
    if (
        lower.endswith(stress["unjudged_suffix"])
        and len(lower) > len(stress["unjudged_suffix"]) + 1
    ):
        return []
    nuclei = spanish_nuclei(lower, stress)
    if not nuclei:
        return [f"{word}: holds no vowel to stress"]
    accented = [index for index, ch in enumerate(lower) if ch in stress["accented"]]
    if len(accented) > 1:
        return [f"{word}: {len(accented)} written accents; a word takes at most one"]
    last = fold(lower[-1])
    open_ending = last in "aeiou" or (
        last in "ns" and len(lower) > 1 and fold(lower[-2]) in "aeiou"
    )
    if accented:
        stressed = next(n for n, nucleus in enumerate(nuclei) if accented[0] in nucleus)
    elif open_ending and len(nuclei) > 1:
        stressed = len(nuclei) - 2
    else:
        stressed = len(nuclei) - 1
    from_end = len(nuclei) - stressed
    computed = (
        "monosílabo"
        if len(nuclei) == 1
        else {1: "aguda", 2: "llana", 3: "esdrújula"}.get(from_end, "sobresdrújula")
    )
    findings = []
    if computed != declared:
        hint = ""
        if not accented and declared in (
            "aguda",
            "llana",
            "esdrújula",
            "sobresdrújula",
        ):
            hint = f"; as a {declared} it would need a tilde"
        findings.append(
            f"{word}: marked {declared}, but its spelling makes it {computed}{hint}"
        )
    if accented:
        diacritic = lower in set(stress["diacritic"]) | set(
            stress["optional_diacritic"]
        )
        at = accented[0]
        hiatus = lower[at] in "íú" and any(
            0 <= near < len(lower) and fold(lower[near]) in "aeo"
            for near in (
                at - 1,
                at + 1,
                at - 2 if at >= 2 and lower[at - 1] == "h" else -1,
                at + 2 if at + 2 < len(lower) and lower[at + 1] == "h" else -1,
            )
        )
        if computed == "monosílabo":
            justified = diacritic
        elif diacritic or hiatus:
            justified = True
        elif computed == "aguda":
            justified = open_ending
        elif computed == "llana":
            justified = not open_ending
        else:
            justified = True
        if not justified:
            reason = {
                "monosílabo": "a monosyllable takes no tilde unless it is diacritic",
                "aguda": "an aguda takes one only when it ends in a vowel, n or s",
                "llana": "a llana that ends in n, s or a vowel takes no tilde",
            }[computed]
            findings.append(
                f"{word}: the tilde is not required; {reason} (RAE OLE 2010)"
            )
    if split is not None:
        joined = "".join(split).lower()
        if joined != lower:
            findings.append(f"{'-'.join(split)}: does not spell {word}")
        elif len(split) != len(nuclei):
            findings.append(
                f"{'-'.join(split)}: {len(split)} syllables, but {word} has {len(nuclei)}"
            )
        else:
            upper = [
                index for index, syllable in enumerate(split) if syllable.isupper()
            ]
            if len(upper) != 1:
                findings.append(
                    f"{'-'.join(split)}: marks no single stressed syllable in capitals"
                )
            elif upper[0] != stressed:
                findings.append(
                    f"{'-'.join(split)}: stresses syllable {upper[0] + 1}, but the spelling stresses syllable {stressed + 1}"
                )
    return findings


def stress_rows(doc: Doc, entry: dict) -> list:
    """(1-based line, headword, declared class, split or None) for each structured stress row."""
    classes = entry["stress"]["classes"]
    found = []
    for line_no, text in entries(section_lines(doc, "pronunciation")):
        declared = category_of(text, classes)
        if declared is None:
            continue
        if TABLE_ROW.match(text):
            first = cells(text)[0]
            if not re.fullmatch(rf"[{LETTERS}]+", first):
                continue
            head, rest = first, " ".join(cells(text)[1:])
        else:
            head_match = re.match(
                rf"\s*(?P<head>[{LETTERS}]+)(?=\s|$)", base_text(text)
            )
            if head_match is None:
                continue
            head, rest = head_match.group("head"), base_text(text)[head_match.end() :]
        split = None
        for token in rest.split():
            token = token.strip(".,;:()")
            if (
                SPANISH_SPLIT.match(token)
                and fold(re.sub(r"[-·]", "", token)).lower() == fold(head).lower()
            ):
                split = re.split(r"[-·]", token)
                break
        found.append((line_no, head, declared, split))
    return found


# --- glosses and corrections, the pack's formats inside persona-core's sections -----------------


def gloss_split(text: str, table: dict) -> tuple:
    """(head, gloss or None) of one gloss item: a table row's cells, or a list item's dash."""
    if TABLE_ROW.match(text):
        row = cells(text)
        return row[0], " ".join(row[1:]).strip() or None
    for separator in table["gloss"]["separators"]:
        if separator in text:
            head, gloss = text.split(separator, 1)
            return head.strip(), gloss.strip() or None
    return text.strip(), None


def candidates(head: str, entry: dict) -> list:
    head = base_text(head)
    found = [
        part.strip() for part in re.split(r"[()（）/,，、;；]", head) if part.strip()
    ]
    for candidate in list(found):
        lower = candidate.lower()
        for article in entry.get("articles", ()):
            prefixes = (
                [article + "'", article + "’"] if len(article) == 1 else [article + " "]
            )
            for prefix in prefixes:
                if lower.startswith(prefix) and len(candidate) > len(prefix):
                    found.append(candidate[len(prefix) :].strip())
    return found


def occurrences(candidate: str, reading: str, language: Language) -> int:
    if language.script is not None:
        return reading.count(candidate)
    pattern = rf"(?<![^\W\d_]){re.escape(candidate)}(?![^\W\d_])"
    return len(re.findall(pattern, reading, re.I))


def reading_text(doc: Doc) -> "str | None":
    item = doc.sections.get("reading")
    return None if item is None else base_text(item.body)


def correction_items(doc: Doc, table: dict) -> list:
    """(1-based line, raw text, match or None) of each list item in the corrections section."""
    arrows = "|".join(re.escape(arrow) for arrow in table["correction"]["arrows"])
    pattern = re.compile(
        rf"^(?P<learner>.+?)\s*(?:{arrows})\s*(?P<fix>.+?)\s*\[(?P<category>[^\]\s]+)\]\s*(?P<why>.*)$"
    )
    found = []
    for line_no, text in entries(section_lines(doc, "corrections")):
        if TABLE_ROW.match(text):
            continue
        text = text.strip()
        found.append((line_no, text, pattern.match(text)))
    return found


def plain(text: str) -> str:
    return " ".join(EMPHASIS.sub("", text).split())


# --- the checks ---------------------------------------------------------------------------------


def check_mentor_templates(ctx: Context) -> list:
    findings = []
    wanted = ctx.table["template"]
    modes = sorted(set(ctx.contract.get("cefr", {}).values()))
    for doc in ctx.templates():
        entry = ctx.language(doc)
        if entry is None:
            findings.append(
                f"{doc.shown}: language/{doc.code} has no entry in the rules table"
            )
            continue
        lang = doc.frontmatter.get("lang")
        if lang not in entry["lang_tags"]:
            findings.append(
                f"{doc.shown}: lang {lang} is not a tag for language/{doc.code} ({', '.join(entry['lang_tags'])})"
            )
        duties = doc.frontmatter.get("duties") or []
        for duty in wanted["duties"]:
            if duty not in duties:
                findings.append(f"{doc.shown}: a language mentor takes the {duty} duty")
        declared = doc.frontmatter.get("sections") or {}
        for duty, ids in wanted["sections"].items():
            have = declared.get(duty, []) if isinstance(declared, dict) else []
            for section_id in ids:
                if section_id not in have:
                    findings.append(f"{doc.shown}: {duty} adds no {section_id} section")
        memory = doc.frontmatter.get("memory") or []
        for source in ctx.contract.get("memory_sources", ()):
            if source not in memory:
                findings.append(
                    f"{doc.shown}: a mentor remembers weak spots, so it reads {source} too"
                )
        method = doc.sections.get("method")
        if method is None:
            findings.append(f"{doc.shown}: has no method section")
            continue
        text = method.body.lower()
        for term in entry["method_terms"]:
            if term.lower() not in text:
                findings.append(f"{doc.shown}: the method never names {term}")
        for mode in modes:
            if mode not in text:
                findings.append(f"{doc.shown}: the method never names the {mode} mode")
    return findings


def check_cefr_ratio(ctx: Context) -> list:
    findings = []
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None:
            findings.append(
                f"{doc.shown}: language/{doc.code} has no entry in the rules table"
            )
            continue
        band = doc.frontmatter.get("cefr")
        mode = mode_of(ctx, doc)
        if mode is None:
            findings.append(
                f"{doc.shown}: names no CEFR band ({band!r}), so its mode cannot be read"
            )
            continue
        bounds = ctx.table["ratio"][mode]
        language = Language(entry, ctx.table)
        tl = en = 0.0
        for chunk, cell in instruction(doc, ctx.table):
            got_tl, got_en = language.units(chunk, cell)
            tl += got_tl
            en += got_en
        if tl + en == 0:
            findings.append(f"{doc.shown}: no measurable instruction text")
            continue
        share = tl / (tl + en)
        if not bounds["min"] <= share <= bounds["max"]:
            findings.append(
                f"{doc.shown}: CEFR {band} is {mode}, but the target language is {share:.2f} of the "
                f"instruction ({tl:.1f} of {tl + en:.1f} word units), outside [{bounds['min']:.2f}, {bounds['max']:.2f}]"
            )
    return findings


def glossed(ctx: Context) -> list:
    """(doc, entry, language, reading, [(line, head, gloss, candidates, matched)], problems)."""
    found = []
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None or "glosses" not in doc.sections:
            continue
        language = Language(entry, ctx.table)
        reading = reading_text(doc)
        items = []
        for line_no, text in entries(section_lines(doc, "glosses")):
            head, gloss = gloss_split(text, ctx.table)
            names = candidates(head, entry)
            matched = next(
                (
                    name
                    for name in names
                    if reading and occurrences(name, reading, language)
                ),
                None,
            )
            items.append((line_no, head, gloss, names, matched))
        found.append((doc, entry, language, reading, items))
    return found


def check_i1_glosses(ctx: Context) -> list:
    findings = []
    for doc, _entry, language, reading, items in glossed(ctx):
        if reading is None:
            findings.append(f"{doc.shown}: glosses a reading it does not have")
            continue
        if not items:
            findings.append(f"{doc.shown}: the glosses section holds no gloss item")
            continue
        seen: dict = {}
        for line_no, head, gloss, names, matched in items:
            if gloss is None:
                findings.append(
                    f"{doc.shown}:{line_no}: gloss item {head!r} has no gloss after its dash"
                )
            key = (names[0] if names else head).lower()
            if key in seen:
                findings.append(
                    f"{doc.shown}:{line_no}: glosses {key!r} twice (line {seen[key]})"
                )
            seen.setdefault(key, line_no)
            if matched is None:
                findings.append(
                    f"{doc.shown}:{line_no}: gloss {head!r} does not occur in the reading"
                )
        declared = doc.frontmatter.get("x-new-words")
        if declared is None:
            continue
        if not isinstance(declared, list) or not all(
            isinstance(word, str) for word in declared
        ):
            findings.append(f"{doc.shown}: x-new-words is not a list of words")
            continue
        for word in declared:
            if not any(
                word.lower() in (name.lower() for name in names)
                for _l, _h, _g, names, _m in items
            ):
                findings.append(
                    f"{doc.shown}: x-new-words lists {word!r}, which is not glossed"
                )
            elif not occurrences(word, reading, language):
                findings.append(
                    f"{doc.shown}: x-new-words lists {word!r}, which does not occur in the reading"
                )
    return findings


def check_i1_density(ctx: Context) -> list:
    findings = []
    ceiling = ctx.table["density"]["max_new_share"]
    for doc, entry, language, reading, items in glossed(ctx):
        if not reading:
            continue
        if entry["density_unit"] == "char" and language.script is not None:
            total = len(language.script.findall(reading))
            new = sum(
                occurrences(m, reading, language) * len(language.script.findall(m))
                for *_rest, m in items
                if m
            )
        else:
            total = len([token for token in reading.split() if WORD.search(token)])
            new = sum(
                occurrences(m, reading, language) * max(1, len(m.split()))
                for *_rest, m in items
                if m
            )
        if total and new / total > ceiling:
            findings.append(
                f"{doc.shown}: the new-word share of the reading is {new / total:.2f} ({new} of {total}), "
                f"over {ceiling:.2f}: Laufer's 95% coverage"
            )
    return findings


def check_grammar_spotlight(ctx: Context) -> list:
    findings = []
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None or "grammar" not in doc.sections:
            continue
        language = Language(entry, ctx.table)
        lines = section_lines(doc, "grammar")
        where = f"{doc.shown}:{doc.sections['grammar'].line}"
        points = sum(1 for _no, line in lines if POINT.match(line))
        if points != 1:
            findings.append(
                f"{where}: the grammar spotlight names {points} points; a spotlight names exactly one"
            )
        examples = 0
        explained = 0.0
        for _no, line in lines:
            quoted = QUOTE.match(line)
            if quoted:
                tl, en = language.units(base_text(quoted.group("text")))
                examples += 1 if tl > 0 and tl >= en else 0
            elif line.strip() and not HEADING.match(line):
                explained += sum(language.units(base_text(line)))
        if not examples:
            findings.append(f"{where}: no target-language example in a blockquote")
        if not explained:
            findings.append(f"{where}: no explanation outside its example")
    return findings


def notation_present(notation: str, doc: Doc, entry: dict, ctx: Context) -> bool:
    lines = section_lines(doc, "pronunciation")
    text = "\n".join(line for _no, line in lines)
    if notation == "pinyin-tones":
        return any(
            marks
            for _token, syllables in pinyin_tokens(base_text(text), entry["pinyin"])
            for _s, marks in syllables
        )
    if notation == "ruby":
        return any(RT.search(match.group("inner")) for match in RUBY.finditer(text))
    if notation == "pitch-accent":
        return bool(
            accent_marks(text, entry["kana"], char_class(entry["kana"]["ranges"]))
        )
    if notation == "batchim":
        return any(
            batchim_pairs(line) or JAMO_STATEMENT.search(line) for _no, line in lines
        )
    if notation == "romanization":
        return bool(romanizations(doc, entry, ctx.table))
    if notation == "liaison":
        liaison = entry["liaison"]
        return any(
            category_of(line, liaison["categories"])
            and liaison_phrase(line, liaison["junctions"])
            for _no, line in entries(lines)
        )
    return bool(stress_rows(doc, entry))


def check_pronunciation_notation(ctx: Context) -> list:
    findings = []
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None or "pronunciation" not in doc.sections:
            continue
        for notation in entry.get("required_notations", ()):
            if not notation_present(notation, doc, entry, ctx):
                findings.append(
                    f"{doc.shown}: the pronunciation notes carry no {notation} notation"
                )
    return findings


def check_pinyin_tones(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("pinyin-tones"):
        for line_no, line in body_lines(doc):
            findings += [
                f"{doc.shown}:{line_no}: {finding}"
                for finding in pinyin_findings(base_text(line), entry["pinyin"])
            ]
    return findings


def check_furigana_ruby(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("ruby"):
        kana = char_class(entry["kana"]["ranges"])
        kanji = char_class(ctx.table["languages"][doc.code]["script"]["ranges"])
        extra = entry["kana"]["kanji_extra"]
        for line_no, line in body_lines(doc):
            line = COMMENT.sub(" ", line)
            findings += [
                f"{doc.shown}:{line_no}: {finding}"
                for finding in ruby_findings(line, kana, kanji, extra)
            ]
    return findings


def check_pitch_accent(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("pitch-accent"):
        kana = char_class(entry["kana"]["ranges"])
        for line_no, line in body_lines(doc):
            findings += [
                f"{doc.shown}:{line_no}: {finding}"
                for finding in accent_findings(
                    COMMENT.sub(" ", line), entry["kana"], kana
                )
            ]
    return findings


def check_hangul_batchim(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("batchim"):
        for line_no, line in body_lines(doc):
            findings += [
                f"{doc.shown}:{line_no}: {finding}"
                for finding in batchim_findings(line, entry["hangul"])
            ]
    return findings


def check_romanization_rr(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("romanization"):
        pattern = rr_pattern(entry["romanization"])
        for line_no, token in romanizations(doc, entry, ctx.table):
            findings += [
                f"{doc.shown}:{line_no}: {finding}"
                for finding in rr_findings(token, pattern)
            ]
    return findings


def check_french_liaison(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("liaison"):
        liaison = entry["liaison"]
        for line_no, text in entries(body_lines(doc)):
            stated = category_of(text, liaison["categories"])
            junction = liaison_phrase(text, liaison["junctions"])
            if stated is None or junction is None:
                continue
            left, marker, right = junction
            expected = liaison_expected(left, right, liaison["rules"])
            if expected is not None and expected[0] != stated:
                phrase = f"{left.strip()}{marker}{right.strip()}"
                findings.append(
                    f"{doc.shown}:{line_no}: {phrase} is marked {stated}, but its context makes it {expected[0]} ({expected[1]})"
                )
    return findings


def check_spanish_stress(ctx: Context) -> list:
    findings = []
    for doc, entry in ctx.with_notation("stress"):
        for line_no, head, declared, split in stress_rows(doc, entry):
            findings += [
                f"{doc.shown}:{line_no}: {finding}"
                for finding in spanish_findings(head, declared, split, entry["stress"])
            ]
    return findings


def check_culture_anchor(ctx: Context) -> list:
    findings = []
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None or "culture" not in doc.sections:
            continue
        language = Language(entry, ctx.table)
        if not language.tl_terms(base_text(doc.sections["culture"].body)):
            findings.append(
                f"{doc.shown}: the culture note holds no target-language term to anchor it"
            )
    return findings


def check_culture_generalization(ctx: Context) -> list:
    findings = []
    demonyms = sorted(
        {
            demonym.lower()
            for entry in ctx.table["languages"].values()
            for demonym in entry.get("demonyms", ())
        },
        key=len,
        reverse=True,
    )
    culture = ctx.table.get("culture", {})
    names = "|".join(re.escape(demonym) for demonym in demonyms)
    quantifiers = "|".join(
        re.escape(word) for word in culture.get("english_quantifiers", ())
    )
    adverbs = "|".join(re.escape(word) for word in culture.get("english_adverbs", ()))
    patterns = []
    if names and quantifiers:
        patterns.append(
            re.compile(
                rf"\b(?:{quantifiers})\s+(?:the\s+)?(?:{names})(?:\s+people)?\b", re.I
            )
        )
    if names and adverbs:
        patterns.append(
            re.compile(
                rf"\b(?:the\s+)?(?:{names})(?:\s+people)?\s+(?:{adverbs})\b", re.I
            )
        )
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None or "culture" not in doc.sections:
            continue
        text = base_text(doc.sections["culture"].body)
        for pattern in patterns:
            for match in pattern.finditer(text):
                findings.append(
                    f"{doc.shown}: the culture note generalises about a whole people: {match.group(0)!r}"
                )
        lower = text.lower()
        for phrase in entry.get("generalizations", ()):
            if phrase.lower() in lower:
                findings.append(
                    f"{doc.shown}: the culture note generalises about a whole people: {phrase.lower()!r}"
                )
    return findings


def check_write_corrections(ctx: Context) -> list:
    findings = []
    indirect = ctx.table["correction"]["indirect"]
    for doc in ctx.outputs():
        entry = ctx.language(doc)
        if entry is None or "corrections" not in doc.sections:
            continue
        taxonomy = {
            category["id"]: category["treatable"] for category in entry["taxonomy"]
        }
        mode = mode_of(ctx, doc)
        band = doc.frontmatter.get("cefr")
        for line_no, text, match in correction_items(doc, ctx.table):
            where = f"{doc.shown}:{line_no}"
            if match is None:
                findings.append(
                    f"{where}: {text!r} is not a correction: <learner form> → <correction> [<category>] <explanation>"
                )
                continue
            category = match.group("category")
            fix = plain(match.group("fix"))
            if category not in taxonomy:
                findings.append(
                    f"{where}: category {category!r} is not in the language/{doc.code} taxonomy"
                )
                continue
            if fix == indirect:
                if mode == "english-led":
                    findings.append(
                        f"{where}: CEFR {band} is english-led, so every correction is given directly"
                    )
                if not taxonomy[category]:
                    findings.append(
                        f"{where}: {category} is untreatable, so it is corrected directly at every band"
                    )
            elif fix == plain(match.group("learner")):
                findings.append(f"{where}: the correction changes nothing")
            if not match.group("why").strip():
                findings.append(f"{where}: the correction has no explanation")
    return findings


def check_write_focus(ctx: Context) -> list:
    findings = []
    ceiling = ctx.table["focus"]["max_categories"]
    for doc in ctx.outputs():
        if ctx.language(doc) is None or "corrections" not in doc.sections:
            continue
        used = {
            match.group("category")
            for _no, _text, match in correction_items(doc, ctx.table)
            if match
        }
        if len(used) > ceiling:
            findings.append(
                f"{doc.shown}: the corrections treat {len(used)} categories; focused feedback treats at most {ceiling}"
            )
    return findings


CHECKS: "dict[str, Callable[[Context], list]]" = {
    "mentor-templates": check_mentor_templates,
    "cefr-ratio": check_cefr_ratio,
    "i1-glosses": check_i1_glosses,
    "i1-density": check_i1_density,
    "grammar-spotlight": check_grammar_spotlight,
    "pronunciation-notation": check_pronunciation_notation,
    "pinyin-tones": check_pinyin_tones,
    "furigana-ruby": check_furigana_ruby,
    "pitch-accent": check_pitch_accent,
    "hangul-batchim": check_hangul_batchim,
    "romanization-rr": check_romanization_rr,
    "french-liaison": check_french_liaison,
    "spanish-stress": check_spanish_stress,
    "culture-anchor": check_culture_anchor,
    "culture-generalization": check_culture_generalization,
    "write-corrections": check_write_corrections,
    "write-focus": check_write_focus,
}


# --- the command line -----------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="language-mentors-probe.py", description=__doc__.split("\n")[0]
    )
    parser.add_argument(
        "--root", help="the tree whose language persona documents are judged"
    )
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="walk only this directory or file (repeatable)",
    )
    parser.add_argument(
        "--table", help="the rules table (default: the pack's languages.json)"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("classes", help="list the classes")
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    return parser


def _usage(message: str) -> int:
    print(f"language-mentors-probe.py: {message}", file=sys.stderr)
    return EXIT_USAGE


def _void(name: str, reason: str) -> int:
    print(f"{name}: VOID: {reason}")
    print("examined 0")
    return EXIT_VOID


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        print("\n".join(CLASSES))
        return EXIT_GREEN
    name = args.name
    if args.root is None:
        return _usage("check needs --root")
    root = Path(args.root)
    if not root.is_dir():
        return _usage(f"--root is not a directory: {args.root}")
    bases = [Path(subject) for subject in args.subject] or [root]
    for base in bases:
        if not base.exists():
            return _usage(f"--subject does not exist: {base}")
    try:
        core = load_core()
        try:
            contract = core.load_contract()
        except core.ContractError as error:
            raise Void(f"persona-core's contract does not read: {error}") from error
        table = load_table(Path(args.table) if args.table else DEFAULT_TABLE)
    except Void as error:
        return _void(name, str(error))
    problems = table_problems(table, contract)
    if name == "rules-table":
        languages = table.get("languages")
        examined = len(languages) if isinstance(languages, dict) else 0
        for problem in problems:
            print(f"{name}: {problem}")
        if examined == 0:
            return _void(name, "the rules table names no language")
        print(f"examined {examined}")
        return EXIT_FINDING if problems else EXIT_GREEN
    if problems:
        return _void(
            name,
            f"the rules table fails rules-table ({len(problems)} problems; first: {problems[0]})",
        )
    docs, broken = load_population(core, root, bases)
    if not docs and not broken:
        return _void(
            name,
            f"no language persona document under {', '.join(str(base) for base in bases)}",
        )
    ctx = Context(root=root, docs=docs, broken=broken, table=table, contract=contract)
    findings = [
        f"{item.shown}: does not parse ({item.reason}); persona-core's output-contract names it"
        for item in broken
    ]
    findings += CHECKS[name](ctx)
    for finding in findings:
        print(f"{name}: {finding}")
    print(f"examined {len(docs) + len(broken)}")
    return EXIT_FINDING if findings else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""study-duties-probe.py -- the duty-quality checks for DeckStreak's six study duties.

SPEC-V2-2216. A study duty (daily-reading, drill-coach, leech-doctor, writing-tutor,
conversation-partner, practice-questions) names WHAT a persona produces, and the persona shapes HOW.
Its output IS a persona output, `phx.persona.output.v1`, so this script parses every file through
persona-core's probe (loaded with importlib, never copied) and adds the structure each duty owes:

    study-duty-templates  block     one duty-instruction template per duty, naming its sections
    study-extension       block     the duty's x- declarations are well formed and on the right duty
    reading-length        block     a law daily reading holds 800 to 1500 prose words
    reading-scale         advisory  more new cards never gives a shorter primer
    leech-contrast        block     a declared leech pair: the explanation names the card, and the
                                    contrast names the card and its confusable card
    leech-pair-declared   advisory  a leech-doctor output declares the pair it contrasts
    leech-angle           advisory  the explanation is not a restatement of the card's answer
    leech-mnemonic        advisory  the mnemonic is short enough to rebuild from memory
    question-key          block     one keyed answer per item, one of its choices, never revealed
    choices-explained     block     every question explained, and every choice of every item
    distractor-cues       advisory  Haladyna's cues: a conspicuous key, all/none of the above, a
                                    lowercase EXCEPT, absolute terms, too few choices, a fixed key
    feedback-specific     advisory  a correction points at the learner's own words
    no-placeholder        block     fail loud: no placeholder stands in for the duty's text
    reply-one-turn        block     a conversation reply is one turn, never a scripted exchange

The multiple-choice format is lsat-coach's `phx.lsat.coach.v1` (d2215), adopted rather than
competed with: a `###` item whose `<!-- lsat-item {...} -->` marker carries the key invisibly,
`(A)` choice lines, and explanation entries `(X) credited|incorrect[, trap]: reason` matched to the
item by its marker id.

It is VENDORABLE on purpose: standard library only (Python >= 3.10), and every class reads any tree
through `--root`, or the paths `--subject` names. The thresholds, section ids and the per-language
tables live in the pack's duties.json, so a new language or a new label is a data row, never a code
branch. Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`.
Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: nothing was examined, or persona-core's
probe or the pack's data could not be read, which is never a pass.
"""

import argparse
import importlib.util
import json
import re
import sys
import unicodedata
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

DUTIES_SCHEMA = "phx.study.duties.v1"
HERE = Path(__file__).resolve()
DEFAULT_PACK_DIR = HERE.parents[1] / "skills" / "packs" / "study-duties"
DEFAULT_PERSONA_CORE = HERE.parent / "persona-core-probe.py"
LAW_KIND = "law"

CLASSES = (
    ("study-duty-templates", "contract", "block"),
    ("study-extension", "contract", "block"),
    ("reading-length", "readings", "block"),
    ("reading-scale", "readings", "advisory"),
    ("leech-contrast", "leeches", "block"),
    ("leech-pair-declared", "leeches", "advisory"),
    ("leech-angle", "leeches", "advisory"),
    ("leech-mnemonic", "leeches", "advisory"),
    ("question-key", "assessment", "block"),
    ("choices-explained", "assessment", "block"),
    ("distractor-cues", "assessment", "advisory"),
    ("feedback-specific", "feedback", "advisory"),
    ("no-placeholder", "integrity", "block"),
    ("reply-one-turn", "integrity", "block"),
)
CLASS_NAMES = tuple(name for name, _stage, _severity in CLASSES)

COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
SECTION_MARKER = re.compile(r"<!--\s*section:\s*[^\s>]*\s*-->")
FENCE = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})")
HEADING = re.compile(r"^ {0,3}(?P<hashes>#{1,6})(?:[ \t]+(?P<content>.*?))?[ \t]*$")
LIST_ITEM = re.compile(
    r"^(?P<indent>[ \t]*)(?:[-*+]|(?P<number>\d{1,9})[.)])[ \t]+(?P<text>.*)$"
)
NUMBERED = re.compile(r"^ {0,3}(?P<number>\d{1,9})[.)][ \t]+(?P<text>\S.*)$")
WORD = re.compile(r"[^\W_]+(?:['’][^\W_]+)*")
CITATION = re.compile(r"\[[^\]\n]*@[^\]\n]*\]")
LINK_DESTINATION = re.compile(r"\]\([^)\s]*\)")
URL = re.compile(r"\bhttps?://\S+", re.IGNORECASE)
CJK_TERM = re.compile(r"[぀-ヿ㐀-䶿一-鿿가-힯豈-﫿]")
ITEM_MARKER = re.compile(r"<!--\s*lsat-item\s+(?P<json>\{.*\})\s*-->")
CHOICE = re.compile(r"^\((?P<letter>[A-J])\)[ \t]+\S")
ENTRY = re.compile(
    r"^\((?P<letter>[A-J])\)[ \t]+(?P<verdict>credited|incorrect)"
    r"(?:,[ \t]*(?P<trap>[a-z0-9-]+))?[ \t]*:[ \t]*(?P<reason>.*)$"
)
REVEAL = re.compile(
    r"\b(?:the\s+)?correct\s+answer\b|\bthe\s+answer\s+is\b", re.IGNORECASE
)
POINTER = re.compile(
    r"\"[^\"\n]+\"|“[^”\n]+”|«[^»\n]+»|„[^“\n]+“|「[^」\n]+」|『[^』\n]+』|‘[^’\n]+’"
    r"|~~[^~\n]+~~|`[^`\n]+`|→|->|=>|⟶"
)
EMPTY_STAND_IN = re.compile(r"^(?:\.{3}|…|-+|—|n/?a|tbd|none|null)$", re.IGNORECASE)
# A leftover work marker in an output is a placeholder. The two words are built from fragments so
# that this file never carries them literally: the repository refuses them in committed code.
WORK_MARKERS = re.compile(
    r"(?<![A-Za-z])(?:" + "TO" + "DO" + "|" + "FIX" + "ME" + r")(?![A-Za-z])"
)


class Void(Exception):
    """An input the class needs could not be read, so nothing can be judged: VOID, never green."""


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass
class Context:
    root: Path
    core: object
    docs: list
    broken: list
    contract: dict
    duties: dict
    pack_dir: Path

    def outputs(self) -> list:
        return [doc for doc in self.docs if doc.kind == "output"]

    def show(self, path: Path) -> str:
        try:
            return str(Path(path).resolve().relative_to(self.root.resolve()))
        except ValueError:
            return str(path)


# --- reading: persona-core's probe and this pack's data -----------------------------------------


def load_core(path: Path) -> object:
    """persona-core's probe as a module: its parser is the contract, and it is never copied."""
    spec = importlib.util.spec_from_file_location("persona_core_probe_for_study", path)
    if spec is None or spec.loader is None:
        raise Void(f"persona-core probe {path} cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except (OSError, SyntaxError, ImportError) as error:
        raise Void(f"persona-core probe {path} cannot be loaded ({error})") from error
    for name in ("load_population", "parse_document", "load_contract", "ContractError"):
        if not hasattr(module, name):
            raise Void(f"persona-core probe {path} exports no {name}")
    return module


def _rows(value: object, where: str) -> list:
    if not isinstance(value, list):
        raise Void(f"duties.json: {where} is not a list")
    compiled = []
    for row in value:
        if not isinstance(row, dict) or not isinstance(row.get("regex"), str):
            raise Void(f"duties.json: {where} holds a row with no regex")
        flags = re.IGNORECASE if "i" in str(row.get("flags", "")) else 0
        try:
            compiled.append((str(row.get("id", "?")), re.compile(row["regex"], flags)))
        except re.error as error:
            raise Void(f"duties.json: {where} {row.get('id')}: {error}") from error
    return compiled


def load_duties(pack_dir: Path) -> dict:
    """The pack's duties.json, every pattern compiled; a missing or malformed table is VOID."""
    path = pack_dir / "duties.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"{path}: unreadable ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != DUTIES_SCHEMA:
        raise Void(f"{path}: schema is not {DUTIES_SCHEMA}")
    for key in (
        "added_sections",
        "templates",
        "reading",
        "leech",
        "questions",
        "labels",
    ):
        if not isinstance(data.get(key), dict):
            raise Void(f"{path}: {key} is missing")
    labels = data["labels"]
    data["compiled"] = {
        "answer": _label_pattern(labels.get("answer"), "labels.answer"),
        "speaker": _label_pattern(labels.get("speaker"), "labels.speaker"),
        "placeholders": _rows(data.get("placeholders"), "placeholders"),
        "all_or_none": _rows(
            data["questions"].get("all_or_none"), "questions.all_or_none"
        ),
        "absolute": _rows(
            data["questions"].get("absolute_terms"), "questions.absolute_terms"
        ),
        "negative": _rows(
            data["questions"].get("negative_stem"), "questions.negative_stem"
        ),
    }
    return data


def _label_pattern(table: object, where: str) -> "re.Pattern":
    """A line that opens with one label of the table, then `:` or `：`, after a bullet or quote."""
    if not isinstance(table, dict) or not table:
        raise Void(f"duties.json: {where} is missing")
    words = sorted(
        {
            word
            for values in table.values()
            for word in values
            if isinstance(word, str) and word
        },
        key=len,
        reverse=True,
    )
    if not words:
        raise Void(f"duties.json: {where} is empty")
    alternation = "|".join(re.escape(word) for word in words)
    return re.compile(
        r"^[ \t]*(?:>[ \t]*)*(?:(?:[-*+]|\d{1,9}[.)])[ \t]+)?(?:\*\*|__)?(?:"
        + alternation
        + r")(?:\*\*|__)?[ \t]*(?:\*\*|__)?[ \t]*[:：]",
        re.IGNORECASE,
    )


# --- reading a section: comments, fences, items and words ----------------------------------------


def _section(doc: object, section_id: str) -> "object | None":
    return next((part for part in doc.sections if part.id == section_id), None)


def _unfenced(text: str) -> list:
    """(offset, line) for every line outside fenced code, comments removed line by line."""
    found = []
    fence = None
    for offset, line in enumerate(COMMENT.sub(_blank_comment, text).split("\n")):
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
            continue
        opened = FENCE.match(line)
        if opened:
            fence = opened["fence"]
            continue
        found.append((offset, line))
    return found


def _blank_comment(match: "re.Match") -> str:
    return "\n" * match.group(0).count("\n")


def _raw_unfenced(text: str) -> list:
    """(offset, line) outside fenced code, comments kept: a placeholder hides in a comment too."""
    found = []
    fence = None
    for offset, line in enumerate(text.split("\n")):
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
            continue
        opened = FENCE.match(line)
        if opened:
            fence = opened["fence"]
            continue
        found.append((offset, SECTION_MARKER.sub(" ", line)))
    return found


def _items(text: str) -> list:
    """(offset, text) of every list item outside fenced code, at any depth."""
    items = []
    for offset, line in _unfenced(text):
        match = LIST_ITEM.match(line)
        if match and match["text"].strip():
            items.append((offset, match["text"].strip()))
    return items


def _words(text: str) -> int:
    return len(WORD.findall(unicodedata.normalize("NFKC", text)))


def prose_words(body: str) -> int:
    """Prose words: everything outside headings, comments, fenced code, citations and link targets."""
    count = 0
    for _offset, line in _unfenced(body):
        if HEADING.match(line):
            continue
        text = CITATION.sub(" ", line)
        text = LINK_DESTINATION.sub("]", text)
        text = URL.sub(" ", text)
        count += _words(text)
    return count


def primer_words(doc: object, ctx: Context) -> int:
    """The primer's prose words: the whole body except the closing retrieval prompts, which are
    the check that follows the reading, not the reading itself."""
    retrieval = ctx.duties["reading"].get("retrieval_section", "retrieval")
    lines = doc.body.split("\n")
    skip = set()
    for index, part in enumerate(doc.sections):
        if part.id != retrieval:
            continue
        start = part.line - doc.body_line
        end = (
            doc.sections[index + 1].line - doc.body_line
            if index + 1 < len(doc.sections)
            else len(lines)
        )
        skip.update(range(start, end))
    kept = [line for offset, line in enumerate(lines) if offset not in skip]
    return prose_words("\n".join(kept))


def _names(text: str, term: str, against: "str | None" = None) -> bool:
    """Whether `text` names `term`.

    A CJK or Hangul term is a substring, whitespace ignored, because those scripts have no word
    boundary. A Latin term is named when every word that tells it apart from `against` (the other
    card's term; all of its words when there is none) stands in `text` as a whole word, an English
    plural allowed. It is read word by word, so a term broken across a line wrap still counts, and a
    contrast may name "the sufficient one" against a "necessary assumption".
    """
    folded = unicodedata.normalize("NFKC", text).casefold()
    wanted = unicodedata.normalize("NFKC", term).casefold().strip()
    if not wanted:
        return False
    if CJK_TERM.search(wanted):
        return re.sub(r"\s+", "", wanted) in re.sub(r"\s+", "", folded)
    words = WORD.findall(wanted)
    if against is not None:
        shared = set(WORD.findall(unicodedata.normalize("NFKC", against).casefold()))
        words = [word for word in words if word not in shared] or words
    return bool(words) and all(
        re.search(rf"(?<![^\W_]){re.escape(word)}(?:e?s)?(?![^\W_])", folded)
        for word in words
    )


# --- the population ------------------------------------------------------------------------------


def _output_population(ctx: Context) -> tuple:
    """The outputs to judge, and a finding for every claimed output that did not parse."""
    findings = [
        f"{ctx.show(broken.path)}: a claimed output does not parse ({broken.reason})"
        for broken in ctx.broken
        if broken.guess in ("output", "unknown")
    ]
    return ctx.outputs(), findings


def output_class(ctx: Context, judge: object) -> Outcome:
    """Judge every output. Each one read counts as examined, whether or not its duty is reached."""
    outputs, findings = _output_population(ctx)
    for doc in outputs:
        where = ctx.show(doc.path)
        findings += [f"{where}{finding}" for finding in judge(doc, ctx)]
    return Outcome(len(outputs) + _broken_outputs(ctx), tuple(findings))


def _broken_outputs(ctx: Context) -> int:
    return sum(1 for broken in ctx.broken if broken.guess in ("output", "unknown"))


def _duty(doc: object) -> object:
    return doc.frontmatter.get("duty")


def _kind(ctx: Context, doc: object) -> "str | None":
    return ctx.core.subject_kind(doc.frontmatter.get("subject"))


# --- contract: the duty templates and the duty's x- declarations ---------------------------------


def _required_sections(ctx: Context, duty: str) -> list:
    registry = {
        entry["id"]: list(entry["sections"]) for entry in ctx.contract["duties"]
    }
    added = ctx.duties["added_sections"].get(duty, [])
    return list(dict.fromkeys(registry.get(duty, []) + list(added)))


def check_study_duty_templates(ctx: Context) -> Outcome:
    spec = ctx.duties["templates"]
    suffix = spec.get("suffix", ".duty.md")
    wanted_sections = list(spec.get("sections", []))
    registry = [entry["id"] for entry in ctx.contract["duties"]]
    directory = ctx.pack_dir / "templates"
    paths = sorted(directory.glob(f"*{suffix}")) if directory.is_dir() else []
    findings: list = []
    owner: dict = {}
    for path in paths:
        where = ctx.show(path)
        try:
            doc = ctx.core.parse_document(path)
        except ctx.core.ContractError as error:
            findings.append(f"{where}: does not parse ({error})")
            continue
        if doc.kind != "rules":
            findings.append(f"{where}: is a persona {doc.kind}, not a rules document")
            continue
        duty = doc.frontmatter.get("x-duty")
        if duty not in registry:
            findings.append(f"{where}: x-duty {duty!r} is not in the duty registry")
            continue
        if duty in owner:
            findings.append(
                f"{where}: duty {duty} is also declared by {ctx.show(owner[duty])}"
            )
            continue
        owner[duty] = path
        for section_id in wanted_sections:
            part = _section(doc, section_id)
            if part is None or not part.body.strip():
                findings.append(f"{where}: section {section_id} is missing or empty")
        described = _section(doc, "sections")
        for section_id in _required_sections(ctx, duty):
            if described is None or not re.search(
                rf"(?<![a-z0-9-]){re.escape(section_id)}(?![a-z0-9-])", described.body
            ):
                findings.append(f"{where}: never names the section {section_id}")
    for duty in registry:
        if duty not in owner:
            findings.append(f"{ctx.show(directory)}: no duty template for {duty}")
    return Outcome(len(paths), tuple(findings))


def _is_count(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value >= 1


def judge_study_extension(doc: object, ctx: Context) -> list:
    found: list = []
    duty = _duty(doc)
    cards_key = ctx.duties["reading"].get("new_cards_key", "x-new-cards")
    leech_key = ctx.duties["leech"].get("extension", "x-leech")
    front = doc.frontmatter
    if cards_key in front:
        if duty != "daily-reading":
            found.append(
                f": {cards_key} on a {duty} output; it belongs to daily-reading"
            )
        if not _is_count(front[cards_key]):
            found.append(
                f": {cards_key} {front[cards_key]!r} is not a whole count of 1 or more"
            )
    elif (
        duty == "daily-reading"
        and _kind(ctx, doc) in ctx.duties["reading"]["band"]["kinds"]
    ):
        found.append(f": a {_kind(ctx, doc)} daily reading declares no {cards_key}")
    if leech_key in front:
        if duty != "leech-doctor":
            found.append(
                f": {leech_key} on a {duty} output; it belongs to leech-doctor"
            )
        found += [
            f": {leech_key} {problem}"
            for problem in _pair_problems(front[leech_key], ctx)
        ]
    return found


def _pair_problems(pair: object, ctx: Context) -> list:
    if not isinstance(pair, dict):
        return ["is not an object"]
    allowed = set(ctx.duties["leech"].get("fields", ["card", "confusable", "answer"]))
    problems = [f"holds an unknown key {key}" for key in pair if key not in allowed]
    card = pair.get("card")
    if not isinstance(card, str) or not card.strip():
        problems.append("names no card")
    if "confusable" not in pair:
        problems.append("has no confusable (null when there is none)")
    elif pair["confusable"] is not None and (
        not isinstance(pair["confusable"], str) or not pair["confusable"].strip()
    ):
        problems.append("confusable is neither a term nor null")
    if "answer" in pair and (
        not isinstance(pair["answer"], str) or not pair["answer"].strip()
    ):
        problems.append("answer is not text")
    return problems


def _declared_pair(doc: object, ctx: Context) -> "dict | None":
    pair = doc.frontmatter.get(ctx.duties["leech"].get("extension", "x-leech"))
    if isinstance(pair, dict) and not _pair_problems(pair, ctx):
        return pair
    return None


# --- readings ------------------------------------------------------------------------------------


def _in_band(doc: object, ctx: Context) -> bool:
    return (
        _duty(doc) == "daily-reading"
        and _kind(ctx, doc) in ctx.duties["reading"]["band"]["kinds"]
    )


def judge_reading_length(doc: object, ctx: Context) -> list:
    if not _in_band(doc, ctx):
        return []
    band = ctx.duties["reading"]["band"]
    low, high = int(band["min_words"]), int(band["max_words"])
    words = primer_words(doc, ctx)
    if low <= words <= high:
        return []
    return [
        f": holds {words} prose words, outside the {low}-{high}-word band for a law primer"
    ]


def check_reading_scale(ctx: Context) -> Outcome:
    outputs, findings = _output_population(ctx)
    cards_key = ctx.duties["reading"].get("new_cards_key", "x-new-cards")
    tolerance = float(ctx.duties["reading"].get("scale_tolerance", 0.1))
    readings = []
    for doc in outputs:
        cards = doc.frontmatter.get(cards_key)
        if _in_band(doc, ctx) and _is_count(cards):
            readings.append((cards, primer_words(doc, ctx), doc))
    for cards, words, doc in readings:
        for other_cards, other_words, other in readings:
            if cards > other_cards and words * (1 + tolerance) < other_words:
                findings.append(
                    f"{ctx.show(doc.path)}: {cards} new cards (more new cards) but {words} words, "
                    f"shorter than {ctx.show(other.path)}'s {other_words} words for {other_cards}"
                )
    return Outcome(len(outputs) + _broken_outputs(ctx), tuple(findings))


# --- leeches -------------------------------------------------------------------------------------


def judge_leech_contrast(doc: object, ctx: Context) -> list:
    if _duty(doc) != "leech-doctor":
        return []
    pair = _declared_pair(doc, ctx)
    if pair is None:
        return []
    found: list = []
    explanation = _section(doc, "explanation")
    contrast = _section(doc, "contrast")
    card, confusable = pair["card"], pair.get("confusable")
    if explanation is not None and not _names(explanation.body, card, confusable):
        found.append(
            f":{explanation.line}: the explanation never names the card {card!r}"
        )
    # The contrast is set AGAINST the confusable card, so it names that card; it may point back at
    # its own card as "this card". With no confusable card, it names the card it contrasts.
    if contrast is not None:
        if confusable is not None and not _names(contrast.body, confusable, card):
            found.append(
                f":{contrast.line}: the contrast never names the confusable card {confusable!r}"
            )
        if confusable is None and not _names(contrast.body, card):
            found.append(
                f":{contrast.line}: the contrast never names the card {card!r}"
            )
    return found


def judge_leech_pair_declared(doc: object, ctx: Context) -> list:
    key = ctx.duties["leech"].get("extension", "x-leech")
    if _duty(doc) == "leech-doctor" and key not in doc.frontmatter:
        return [
            f": declares no {key}, so its contrast cannot be checked against the confusable card"
        ]
    return []


def _content_words(text: str) -> set:
    return {
        word.casefold()
        for word in WORD.findall(unicodedata.normalize("NFKC", COMMENT.sub(" ", text)))
        if len(word) > 2 or CJK_TERM.search(word)
    }


def judge_leech_angle(doc: object, ctx: Context) -> list:
    if _duty(doc) != "leech-doctor":
        return []
    pair = _declared_pair(doc, ctx)
    explanation = _section(doc, "explanation")
    if pair is None or "answer" not in pair or explanation is None:
        return []
    said = _content_words(explanation.body)
    answer = _content_words(pair["answer"])
    if not said or not answer:
        return []
    share = len(said & answer) / len(said)
    limit = float(ctx.duties["leech"].get("restated_share", 0.7))
    if share >= limit:
        return [
            f":{explanation.line}: the explanation restates the card's answer "
            f"({share:.0%} of its words), so it gives no new angle"
        ]
    return []


def judge_leech_mnemonic(doc: object, ctx: Context) -> list:
    if _duty(doc) != "leech-doctor":
        return []
    part = _section(doc, "mnemonic")
    if part is None:
        return []
    words = prose_words(part.body)
    limit = int(ctx.duties["leech"].get("mnemonic_max_words", 40))
    if words > limit:
        return [
            f":{part.line}: the mnemonic runs {words} words, more than {limit} to rebuild from memory"
        ]
    return []


# --- assessment: d2215's multiple-choice format, and open questions ------------------------------


@dataclass
class Item:
    line: int
    marker: object
    choices: list
    stem: str


def _blocks(part: object) -> list:
    """(line, heading content, [(line, text)]) for every `###` block of a section, fences skipped."""
    blocks: list = []
    fence = None
    current = None
    for offset, line in enumerate(part.body.split("\n")):
        number = part.line + 1 + offset
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
            continue
        opened = FENCE.match(line)
        if opened:
            fence = opened["fence"]
            continue
        heading = HEADING.match(line)
        if heading and len(heading["hashes"]) == 3:
            current = (number, heading["content"] or "", [])
            blocks.append(current)
            continue
        if current is not None:
            current[2].append((number, line))
    return blocks


def _marker(content: str) -> "tuple | None":
    """(object or None, problem) for an `lsat-item` marker, or None when the heading carries none."""
    match = ITEM_MARKER.search(content)
    if match is None:
        return None
    try:
        value = json.loads(match["json"])
    except json.JSONDecodeError:
        return (None, "its lsat-item marker is not JSON")
    if not isinstance(value, dict):
        return (None, "its lsat-item marker is not an object")
    return (value, "")


def _items_of(part: object) -> list:
    items = []
    for line, content, lines in _blocks(part):
        marker = _marker(content)
        if marker is None:
            continue
        choices = [
            (number, CHOICE.match(text)["letter"])
            for number, text in lines
            if CHOICE.match(text)
        ]
        stem = ""
        paragraph: list = []
        for _number, text in lines:
            if CHOICE.match(text):
                break
            if text.strip():
                paragraph.append(text.strip())
            elif paragraph:
                stem, paragraph = " ".join(paragraph), []
        if paragraph:
            stem = " ".join(paragraph)
        items.append(Item(line, marker, choices, stem))
    return items


def _entries(part: object) -> dict:
    """id -> (line, [(line, letter, verdict, reason)]) for every explanation block with a marker."""
    found: dict = {}
    for line, content, lines in _blocks(part):
        marker = _marker(content)
        if marker is None or marker[0] is None:
            continue
        item_id = marker[0].get("id")
        entries = []
        for number, text in lines:
            match = ENTRY.match(text)
            if match:
                entries.append(
                    (number, match["letter"], match["verdict"], match["reason"].strip())
                )
        found.setdefault(item_id, (line, entries))
    return found


def _numbered(part: object) -> list:
    return [
        (part.line + 1 + offset, match["text"])
        for offset, line in _unfenced(part.body)
        if (match := NUMBERED.match(line))
    ]


def _practice_parts(doc: object) -> tuple:
    return _section(doc, "questions"), _section(doc, "explanations")


def _choice_letters(item: Item, entries: dict) -> list:
    """An item's choice letters: its `(A)` lines or, for an official item that is cited by its
    locator and never reproduced (d2215's format), the letters its explanation entries cover."""
    letters = [letter for _number, letter in item.choices]
    value = item.marker[0] if isinstance(item.marker[0], dict) else {}
    item_id = value.get("id")
    if (
        not letters
        and value.get("provenance") == "official"
        and isinstance(item_id, str)
    ):
        letters = [letter for _n, letter, _v, _r in entries.get(item_id, (0, []))[1]]
    return letters


def judge_question_key(doc: object, ctx: Context) -> list:
    if _duty(doc) != "practice-questions":
        return []
    questions, explanations = _practice_parts(doc)
    if questions is None:
        return []
    found: list = []
    minimum = int(ctx.duties["questions"].get("min_choices", 2))
    answer = ctx.duties["compiled"]["answer"]
    entries = _entries(explanations) if explanations is not None else {}
    seen: set = set()
    for item in _items_of(questions):
        value, problem = item.marker
        if value is None:
            found.append(f":{item.line}: {problem}")
            continue
        item_id = value.get("id")
        label = item_id if isinstance(item_id, str) and item_id else f"line {item.line}"
        if not isinstance(item_id, str) or not item_id:
            found.append(f":{item.line}: the item marker names no id")
        elif item_id in seen:
            found.append(f":{item.line}: the questions section repeats item {item_id}")
        seen.add(item_id)
        letters = _choice_letters(item, entries)
        if len(letters) < minimum:
            found.append(f":{item.line}: {label} has fewer than {minimum} choices")
        expected = [chr(ord("A") + index) for index in range(len(letters))]
        if letters and letters != expected:
            found.append(
                f":{item.line}: {label}'s choices skip a letter or run out of order"
            )
        key = value.get("key")
        if not isinstance(key, str) or not key:
            found.append(f":{item.line}: {label}'s marker carries no key")
            continue
        if key not in letters:
            found.append(
                f":{item.line}: {label}'s key ({key}) is not one of its choices"
            )
        if isinstance(item_id, str) and item_id in entries:
            credited = [
                letter
                for _n, letter, verdict, _r in entries[item_id][1]
                if verdict == "credited"
            ]
            if not credited:
                found.append(
                    f":{entries[item_id][0]}: {label}'s explanation credits no choice"
                )
            elif len(credited) > 1:
                found.append(
                    f":{entries[item_id][0]}: {label}'s explanation credits {len(credited)} choices"
                )
            elif credited[0] != key:
                found.append(
                    f":{entries[item_id][0]}: {label}'s explanation credits ({credited[0]}), not its key ({key})"
                )
    for offset, line in _unfenced(questions.body):
        number = questions.line + 1 + offset
        if ENTRY.match(line) or answer.match(line) or REVEAL.search(line):
            found.append(f":{number}: the questions section reveals an answer")
    return found


def judge_choices_explained(doc: object, ctx: Context) -> list:
    if _duty(doc) != "practice-questions":
        return []
    questions, explanations = _practice_parts(doc)
    if questions is None:
        return []
    found: list = []
    minimum = int(ctx.duties["questions"].get("reason_min_words", 6))
    items = _items_of(questions)
    if items:
        entries = _entries(explanations) if explanations is not None else {}
        for item in items:
            value = item.marker[0]
            if value is None:
                continue
            item_id = value.get("id")
            if item_id not in entries:
                found.append(f":{item.line}: item {item_id} has no explanation")
                continue
            line, lines = entries[item_id]
            explained = {letter for _n, letter, _v, _r in lines}
            for _number, letter in item.choices:
                if letter not in explained:
                    found.append(
                        f":{line}: item {item_id}'s choice ({letter}) is not explained"
                    )
            reasons: list = []
            for number, letter, _verdict, reason in lines:
                if _words(reason) < minimum:
                    found.append(
                        f":{number}: ({letter})'s reason has fewer than {minimum} words"
                    )
                if reason and reason.casefold() in reasons:
                    found.append(
                        f":{number}: ({letter}) repeats another choice's reason"
                    )
                reasons.append(reason.casefold())
        return found
    asked = _numbered(questions)
    if not asked:
        return [
            f":{questions.line}: the questions section holds no question (no item, no numbered question)"
        ]
    answered = _numbered(explanations) if explanations is not None else []
    if len(answered) < len(asked):
        found.append(
            f":{questions.line}: {len(asked)} questions but {len(answered)} explanation"
            f"{'' if len(answered) == 1 else 's'}"
        )
    return found


def judge_distractor_cues(doc: object, ctx: Context) -> list:
    if _duty(doc) != "practice-questions":
        return []
    questions, _explanations = _practice_parts(doc)
    if questions is None:
        return []
    spec = ctx.duties["questions"]
    compiled = ctx.duties["compiled"]
    found: list = []
    lines = dict((number, text) for number, text in _choice_texts(questions))
    items = [item for item in _items_of(questions) if item.marker[0] is not None]
    keys = []
    for item in items:
        value = item.marker[0]
        key = value.get("key")
        label = value.get("id", f"line {item.line}")
        if isinstance(key, str):
            keys.append(key)
        if not item.choices and value.get("provenance") == "official":
            continue
        texts = {letter: lines.get(number, "") for number, letter in item.choices}
        if len(texts) < int(spec.get("advised_min_choices", 3)):
            found.append(
                f":{item.line}: {label} offers only {len(texts)} choices (three are adequate)"
            )
        if key in texts and len(texts) > 1:
            others = [len(text) for letter, text in texts.items() if letter != key]
            ratio = float(spec.get("longest_key_ratio", 1.5))
            if len(texts[key]) > max(others) and len(texts[key]) >= ratio * (
                sum(others) / len(others)
            ):
                found.append(
                    f":{item.line}: {label}'s key ({key}) is conspicuously the longest choice"
                )
        for letter, text in texts.items():
            for row_id, pattern in compiled["all_or_none"]:
                if pattern.search(text):
                    found.append(
                        f":{item.line}: {label}'s choice ({letter}) is {row_id.replace('-', ' ')}"
                    )
            for _row_id, pattern in compiled["absolute"]:
                match = pattern.search(text)
                if match:
                    found.append(
                        f":{item.line}: {label}'s choice ({letter}) uses the absolute term {match.group(0)!r}"
                    )
        for _row_id, pattern in compiled["negative"]:
            match = pattern.search(item.stem)
            if match:
                found.append(
                    f":{item.line}: {label}'s stem word {match.group(0)!r} is not capitalized"
                )
    same = int(spec.get("same_key_min_items", 3))
    if len(keys) >= same and len(set(keys)) == 1:
        found.append(
            f":{questions.line}: every item keys ({keys[0]}), so the key's place is a cue"
        )
    return found


def _choice_texts(part: object) -> list:
    found = []
    for offset, line in _unfenced(part.body):
        match = CHOICE.match(line)
        if match:
            found.append((part.line + 1 + offset, line[match.end() - 1 :].strip()))
    return found


# --- feedback ------------------------------------------------------------------------------------


def judge_feedback_specific(doc: object, ctx: Context) -> list:
    if _duty(doc) != "writing-tutor":
        return []
    part = _section(doc, "corrections")
    if part is None:
        return []
    found = []
    for offset, text in _items(part.body):
        if not POINTER.search(text):
            found.append(
                f":{part.line + 1 + offset}: a correction points at none of the learner's words "
                "(quote them, strike them through, or show original → fix)"
            )
    return found


# --- integrity -----------------------------------------------------------------------------------


def judge_no_placeholder(doc: object, ctx: Context) -> list:
    found = []
    for part in doc.sections:
        plain = COMMENT.sub(" ", part.body).strip()
        if EMPTY_STAND_IN.match(plain):
            found.append(
                f":{part.line}: section {part.id} is an empty stand-in {plain!r}"
            )
        for offset, line in _raw_unfenced(part.body):
            number = part.line + 1 + offset
            folded = unicodedata.normalize("NFKC", line)
            if WORK_MARKERS.search(folded):
                found.append(f":{number}: a work marker stands in for the text")
            for row_id, pattern in ctx.duties["compiled"]["placeholders"]:
                if pattern.search(folded):
                    found.append(f":{number}: {row_id} stands in for the text")
    return found


def judge_reply_one_turn(doc: object, ctx: Context) -> list:
    if _duty(doc) != "conversation-partner":
        return []
    part = _section(doc, "reply")
    if part is None:
        return []
    speaker = ctx.duties["compiled"]["speaker"]
    turns = [
        part.line + 1 + offset
        for offset, line in _unfenced(part.body)
        if speaker.match(unicodedata.normalize("NFKC", line))
    ]
    if len(turns) >= 2:
        return [
            f":{turns[1]}: the reply scripts {len(turns)} speaker turns; a reply is one turn"
        ]
    return []


CHECKS = {
    "study-duty-templates": check_study_duty_templates,
    "study-extension": lambda ctx: output_class(ctx, judge_study_extension),
    "reading-length": lambda ctx: output_class(ctx, judge_reading_length),
    "reading-scale": check_reading_scale,
    "leech-contrast": lambda ctx: output_class(ctx, judge_leech_contrast),
    "leech-pair-declared": lambda ctx: output_class(ctx, judge_leech_pair_declared),
    "leech-angle": lambda ctx: output_class(ctx, judge_leech_angle),
    "leech-mnemonic": lambda ctx: output_class(ctx, judge_leech_mnemonic),
    "question-key": lambda ctx: output_class(ctx, judge_question_key),
    "choices-explained": lambda ctx: output_class(ctx, judge_choices_explained),
    "distractor-cues": lambda ctx: output_class(ctx, judge_distractor_cues),
    "feedback-specific": lambda ctx: output_class(ctx, judge_feedback_specific),
    "no-placeholder": lambda ctx: output_class(ctx, judge_no_placeholder),
    "reply-one-turn": lambda ctx: output_class(ctx, judge_reply_one_turn),
}


# --- the command line ----------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="study-duties-probe.py",
        description="Judge study-duty output against its duty's structure (SPEC-V2-2216).",
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="a directory or file to judge instead of --root; repeatable",
    )
    parser.add_argument(
        "--kind",
        action="append",
        default=[],
        help="judge only this subject kind; repeatable",
    )
    parser.add_argument(
        "--pack-dir",
        help="the study-duties pack directory (default: beside this script)",
    )
    parser.add_argument(
        "--persona-core", help="persona-core's probe (default: beside this script)"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASS_NAMES)
    commands.add_parser(
        "classes", help="list the classes with their stage and severity"
    )
    return parser


def _usage(message: str) -> int:
    print(f"study-duties-probe: {message}", file=sys.stderr)
    return EXIT_USAGE


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        for name, stage, severity in CLASSES:
            print(f"{name} {stage} {severity}")
        return EXIT_GREEN
    name = args.name
    root = Path(args.root)
    if not root.is_dir():
        return _usage(f"--root is not a directory: {args.root}")
    bases = [Path(subject) for subject in args.subject] or [root]
    for base in bases:
        if not base.exists():
            return _usage(f"--subject does not exist: {base}")
    pack_dir = Path(args.pack_dir) if args.pack_dir else DEFAULT_PACK_DIR
    try:
        core = load_core(
            Path(args.persona_core) if args.persona_core else DEFAULT_PERSONA_CORE
        )
        try:
            contract = core.load_contract()
        except core.ContractError as error:
            raise Void(f"persona-core's contract.json: {error}") from error
        duties = load_duties(pack_dir)
        for kind in args.kind:
            if kind not in contract["kinds"]:
                return _usage(
                    f"--kind {kind} is not one of {', '.join(contract['kinds'])}"
                )
        docs, broken = core.load_population(bases, args.kind)
        ctx = Context(
            root=root,
            core=core,
            docs=docs,
            broken=broken,
            contract=contract,
            duties=duties,
            pack_dir=pack_dir,
        )
        outcome = CHECKS[name](ctx)
    except Void as error:
        print(f"{name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    if outcome.examined == 0:
        print(f"{name}: VOID: examined nothing")
        print("examined 0")
        return EXIT_VOID
    print(f"examined {outcome.examined}")
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())

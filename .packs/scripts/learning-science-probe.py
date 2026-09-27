#!/usr/bin/env python3
"""learning-science-probe.py -- the shared pedagogy classes every AI study text follows.

SPEC-V2-2224. DeckStreak's engine writes every AI study text as a persona OUTPUT (persona-core's
`phx.persona.output.v1`): a daily reading, a drill, a leech remedy, a writing tutor's corrections,
a conversation turn, a practice set. The duty names what is produced and the persona shapes how;
this script judges whether the text TEACHES the way the learning-science evidence says it should:

    pedagogy-contract      block     the table agrees with persona-core's duty registry
    retrieval-prompts      block     a reading ends with retrieval prompts (practice testing)
    answers-hidden         block     a prompt never shows its answer before the learner tries
    ask-before-tell        block     a drill opens with a question; questions precede explanations
    deep-questions         block     a reading asks at least one why or how question
    concrete-examples      advisory  a reading grounds the abstract in a concrete example
    interleaving           advisory  a practice set mixes its topics rather than blocking them
    worked-example-steps   advisory  a worked example shows its steps, then sets a problem
    study-advice           advisory  no cramming, rereading or highlighting as the study method
    key-terms-defined      block     each key term is defined, used, and listed once
    feedback-task-focused  advisory  feedback addresses the work, never the person or the cohort
    feedback-next-step     advisory  feedback ends with where to go next

It COMPOSES with persona-core and copies nothing: files are found and parsed only through
`persona-core-probe.py`'s `load_population` and `parse_document`, lines are scanned through its
public `scan`, and the duty registry is read from its `contract.json`. It is loaded with importlib
from beside this script, so the two vendor together into any repository.

Every threshold, section id, genre and language pattern is DATA in the pack's `pedagogy.json`,
and this script branches on no language code. A duty maps to a genre (a daily reading is a
`reading`), and each class names the genres, and optionally the subject kinds, it judges.

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`: the
persona outputs of a genre it judges, plus every claimed output that does not parse, which is a
finding and never a skip. Exit 0 is green, 1 a finding, 2 a usage error, and 3 VOID: nothing was
examined, or an input (the table, persona-core's probe) could not be read. VOID is never a pass.

The annotations are evaluated at run time on purpose (no `from __future__ import annotations`): a
duty pack loads this file through importlib, and a dataclass with string annotations looks its
module up in sys.modules, which fails when the loader has not registered it.

Usage:
    learning-science-probe.py classes
    learning-science-probe.py --root R [--subject DIR]... [--kind K]... [--duty D]...
                              [--pedagogy FILE] check <class>
"""

import argparse
import importlib.util
import json
import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
CORE_PATH = HERE / "persona-core-probe.py"
DEFAULT_PEDAGOGY = (
    HERE.parent / "skills" / "packs" / "learning-science" / "pedagogy.json"
)
PEDAGOGY_SCHEMA = "phx.learning-science.pedagogy.v1"

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

TABLE_CLASS = "pedagogy-contract"
CLASSES = (
    TABLE_CLASS,
    "retrieval-prompts",
    "answers-hidden",
    "ask-before-tell",
    "deep-questions",
    "concrete-examples",
    "interleaving",
    "worked-example-steps",
    "study-advice",
    "key-terms-defined",
    "feedback-task-focused",
    "feedback-next-step",
)
OUTPUT_CLASSES = CLASSES[1:]
PATTERN_GROUPS = (
    "retrieval_verbs",
    "answer_labels",
    "explanatory",
    "example_markers",
    "low_utility_advice",
    "person_focused",
)
SECTION_KEYS = (
    "retrieval",
    "reading",
    "key_terms",
    "worked_example",
    "drill",
    "questions",
    "explanations",
)
SECTION_LISTS = ("feed_forward", "concrete")
THRESHOLDS = (
    "retrieval_min_prompts",
    "interleaving_min_prompts",
    "interleaving_min_topics",
    "worked_example_min_steps",
)
TABLE_KEYS = frozenset(
    {
        "schema",
        "spec",
        "note",
        "genres",
        "duties",
        "classes",
        "sections",
        "thresholds",
        "patterns",
    }
)
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

ITEM = re.compile(r"^ ?(?:[-*+]|(?P<number>[0-9]{1,9})[.)])[ \t]+(?P<text>\S.*)$")
FENCE = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})")
COMMENT = re.compile(r"<!--.*?-->", re.S)
QUOTE = re.compile(r"^ {0,3}>")
TOPIC = re.compile(r"<!--\s*topic:\s*(?P<topic>[a-z0-9]+(?:-[a-z0-9]+)*)\s*-->")
HIDDEN = (
    re.compile(r"<details\b[^>]*>.*?</details\s*>", re.S | re.I),
    re.compile(r"<tg-spoiler\b[^>]*>.*?</tg-spoiler\s*>", re.S | re.I),
    re.compile(
        r"<span\b[^>]*\bclass\s*=\s*[\"']tg-spoiler[\"'][^>]*>.*?</span\s*>",
        re.S | re.I,
    ),
    re.compile(r"\|\|[^|\n](?:[^|\n]|\|(?!\|))*?\|\|"),
)
QUESTION = re.compile(r"\?(?=\s|$|[)\]\"'”’」』*_~])")
CLOZE = re.compile(r"_{3,}|\(\s+\)")
LEAD = re.compile(r"^(?:\*\*|__|\*|_)+")
QUOTED = re.compile(
    r'"[^"\n]*?[^\s"]+(?:\s+[^\s"]+){2,}[^"\n]*?"'
    r"|“[^”\n]*?[^\s”]+(?:\s+[^\s”]+){2,}[^”\n]*?”"
    r"|«[^»\n]*?[^\s»]+(?:\s+[^\s»]+){2,}[^»\n]*?»|「[^」\n]{3,}」|『[^』\n]{3,}』"
)
STEP_LABEL = re.compile(
    r"^\s*(?:\*\*)?(?:step\s*[0-9]+|étape\s*[0-9]+|paso\s*[0-9]+|第\s*[0-9一二三四五六七八九十]+\s*步"
    r"|ステップ\s*[0-9]+|[0-9]+\s*단계)",
    re.I,
)
TERM = re.compile(
    r"^(?:\*\*(?P<bold>[^*]+?)\*\*|__(?P<under>[^_]+?)__|(?P<plain>[^—–:：]+?))"
    r"\s*(?:(?:—|–|:|：|\s-\s)\s*(?P<definition>.*))?$"
)

_CORE = None
_DEFAULT_TABLE = None


class PedagogyError(ValueError):
    """The pedagogy table, or persona-core beside this script, cannot be read. str() is why."""


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass(frozen=True)
class Item:
    """One top-level list item of a section: its first file line, raw text and visible text."""

    line: int
    raw: str
    visible: str
    number: str | None
    topic: str | None


# --- persona-core, loaded from beside this script ---------------------------------------------


def _core():
    """persona-core-probe.py, loaded once through importlib and registered under its own name."""
    global _CORE
    if _CORE is None:
        if not CORE_PATH.is_file():
            raise PedagogyError(
                f"persona-core-probe.py is not beside this script ({CORE_PATH}); learning-science "
                "reads persona outputs only through persona-core's parser"
            )
        spec = importlib.util.spec_from_file_location(
            "learning_science_persona_core", CORE_PATH
        )
        if spec is None or spec.loader is None:
            raise PedagogyError(
                f"persona-core-probe.py cannot be loaded from {CORE_PATH}"
            )
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)
        _CORE = module
    return _CORE


# --- the table ----------------------------------------------------------------------------------


def _compile(row: object, where: str) -> tuple:
    if not isinstance(row, dict) or not isinstance(row.get("regex"), str):
        raise PedagogyError(f"{where}: a row has no regex")
    flags = re.IGNORECASE if "i" in str(row.get("flags", "")) else 0
    try:
        return (str(row.get("id", "?")), re.compile(row["regex"], flags))
    except re.error as error:
        raise PedagogyError(
            f"pattern {where}/{row.get('id')} does not compile: {error}"
        ) from error


def _table_problems(table: object) -> list:
    """Every structural problem of a parsed table; [] when it is sound."""
    if not isinstance(table, dict):
        return ["the table is not a JSON object"]
    problems = []
    if table.get("schema") != PEDAGOGY_SCHEMA:
        problems.append(f"schema is {table.get('schema')!r}, not {PEDAGOGY_SCHEMA}")
    for key in sorted(set(table) - TABLE_KEYS):
        problems.append(f"unknown key {key}")
    genres = table.get("genres")
    if not (
        isinstance(genres, list)
        and genres
        and all(isinstance(g, str) and SLUG.match(g) for g in genres)
    ):
        problems.append("genres is not a non-empty list of slugs")
        genres = []
    duties = table.get("duties")
    if not isinstance(duties, dict) or not duties:
        problems.append("duties is not a non-empty object")
        duties = {}
    for duty, genre in duties.items():
        if genre not in genres:
            problems.append(f"duty {duty} maps to unknown genre {genre!r}")
    classes = table.get("classes")
    if not isinstance(classes, dict):
        problems.append("classes is not an object")
        classes = {}
    for name in OUTPUT_CLASSES:
        if name not in classes:
            problems.append(f"class {name} has no entry")
    for name, entry in classes.items():
        if name not in OUTPUT_CLASSES:
            problems.append(f"class {name} is not a class of this probe")
            continue
        named = entry.get("genres") if isinstance(entry, dict) else None
        if not isinstance(named, list) or not named:
            problems.append(f"class {name} names no genre")
            continue
        for genre in named:
            if genre not in genres:
                problems.append(f"class {name} names unknown genre {genre!r}")
        kinds = entry.get("kinds", [])
        if not isinstance(kinds, list) or not all(
            isinstance(kind, str) for kind in kinds
        ):
            problems.append(f"class {name} kinds is not a list of kinds")
    sections = table.get("sections")
    if not isinstance(sections, dict):
        problems.append("sections is not an object")
        sections = {}
    for key in SECTION_KEYS:
        if not (isinstance(sections.get(key), str) and SLUG.match(sections[key])):
            problems.append(f"sections.{key} is not a section id")
    for key in SECTION_LISTS:
        ids = sections.get(key)
        if not (
            isinstance(ids, list)
            and ids
            and all(isinstance(i, str) and SLUG.match(i) for i in ids)
        ):
            problems.append(f"sections.{key} is not a non-empty list of section ids")
    bearing = sections.get("answer_bearing")
    if not isinstance(bearing, dict):
        problems.append("sections.answer_bearing is not an object")
    else:
        for genre, ids in bearing.items():
            if genre not in genres:
                problems.append(
                    f"sections.answer_bearing names unknown genre {genre!r}"
                )
            if not isinstance(ids, list) or not all(
                isinstance(i, str) and SLUG.match(i) for i in ids
            ):
                problems.append(
                    f"sections.answer_bearing.{genre} is not a list of section ids"
                )
    thresholds = table.get("thresholds")
    if not isinstance(thresholds, dict):
        problems.append("thresholds is not an object")
        thresholds = {}
    for key in THRESHOLDS:
        value = thresholds.get(key)
        if not (isinstance(value, int) and not isinstance(value, bool) and value >= 1):
            problems.append(f"thresholds.{key} is not an integer of at least 1")
    patterns = table.get("patterns")
    if not isinstance(patterns, dict):
        problems.append("patterns is not an object")
        patterns = {}
    for group in PATTERN_GROUPS:
        rows = patterns.get(group)
        if not isinstance(rows, list) or not rows:
            problems.append(f"patterns.{group} holds no row")
            continue
        for row in rows:
            try:
                _compile(row, group)
            except PedagogyError as error:
                problems.append(str(error))
    return problems


def load_pedagogy(path: "Path | str | None" = None) -> dict:
    """The pedagogy table. None resolves beside this script. Raises PedagogyError when unsound."""
    global _DEFAULT_TABLE
    if path is None and _DEFAULT_TABLE is not None:
        return _DEFAULT_TABLE
    where = Path(path) if path is not None else DEFAULT_PEDAGOGY
    try:
        table = json.loads(where.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise PedagogyError(f"{where} cannot be read as JSON: {error}") from error
    problems = _table_problems(table)
    if problems:
        raise PedagogyError(f"{where}: " + "; ".join(problems))
    if path is None:
        _DEFAULT_TABLE = table
    return table


def _rows(table: dict, group: str) -> list:
    return [_compile(row, group) for row in table["patterns"][group]]


def genre_for(duty: object, pedagogy: "dict | None" = None) -> "str | None":
    """The genre a persona-core duty is judged as, or None for a duty the table does not map."""
    table = pedagogy if pedagogy is not None else load_pedagogy()
    genre = table["duties"].get(duty) if isinstance(duty, str) else None
    return genre if isinstance(genre, str) else None


def applies(class_id: str, doc: object, pedagogy: "dict | None" = None) -> bool:
    """Whether `class_id` judges `doc`: its duty's genre, and its subject's kind where named."""
    if class_id not in OUTPUT_CLASSES:
        return False
    table = pedagogy if pedagogy is not None else load_pedagogy()
    frontmatter = getattr(doc, "frontmatter", {}) or {}
    entry = table["classes"][class_id]
    if genre_for(frontmatter.get("duty"), table) not in entry["genres"]:
        return False
    kinds = entry.get("kinds") or []
    return not kinds or _core().subject_kind(frontmatter.get("subject")) in kinds


# --- reading a document -------------------------------------------------------------------------


def _blank(match: re.Match) -> str:
    return re.sub(r"[^\n]", " ", match.group(0))


def _mask_fences(text: str) -> str:
    lines = text.split("\n")
    fence = None
    for index, line in enumerate(lines):
        opened = FENCE.match(line)
        if fence is None and opened:
            fence = opened["fence"]
            lines[index] = ""
        elif fence is not None:
            if (
                opened
                and opened["fence"][0] == fence[0]
                and len(opened["fence"]) >= len(fence)
            ):
                fence = None
            lines[index] = ""
    return "\n".join(lines)


def _prose(text: str, *, hide: bool = False, quotes: bool = True) -> str:
    """`text` with code fences and comments blanked, line for line; answers too when `hide`."""
    text = COMMENT.sub(_blank, _mask_fences(text))
    if hide:
        for pattern in HIDDEN:
            text = pattern.sub(_blank, text)
    if not quotes:
        text = "\n".join("" if QUOTE.match(line) else line for line in text.split("\n"))
    return text


def _fold(text: str) -> str:
    return unicodedata.normalize("NFKC", text)


def _section(doc: object, section_id: str):
    return next((s for s in getattr(doc, "sections", ()) if s.id == section_id), None)


def _items(section) -> list:
    """The top-level list items of a section, read on its visible text (answers blanked)."""
    raw_lines = section.body.split("\n")
    shown = _prose(section.body, hide=True).split("\n")
    found: list = []
    current = None
    for index, (raw, visible) in enumerate(zip(raw_lines, shown)):
        opened = ITEM.match(visible)
        if opened:
            if current is not None:
                found.append(current)
            current = [index, [raw], [visible], opened["number"]]
            continue
        if current is None:
            continue
        if (
            visible.strip()
            and not raw.startswith((" ", "\t"))
            and not current[2][-1].strip()
        ):
            found.append(current)
            current = None
            continue
        current[1].append(raw)
        current[2].append(visible)
    if current is not None:
        found.append(current)
    items = []
    for index, raw, visible, number in found:
        raw_text = "\n".join(raw)
        topic = TOPIC.search(raw_text)
        items.append(
            Item(
                line=section.line + 1 + index,
                raw=raw_text,
                visible=_fold(
                    " ".join(part.strip() for part in visible if part.strip())
                ),
                number=number,
                topic=topic["topic"] if topic else None,
            )
        )
    return items


def _item_body(item: Item) -> str:
    """An item's visible text with its list marker removed."""
    opened = ITEM.match(item.visible)
    return (opened["text"] if opened else item.visible).strip()


def _text_of(item: Item) -> str:
    """An item's visible text with its list marker and any leading emphasis removed."""
    return LEAD.sub("", _item_body(item)).strip()


def _is_prompt(item: Item, table: dict) -> bool:
    text = _text_of(item)
    if QUESTION.search(text) or CLOZE.search(text):
        return True
    return any(pattern.search(text) for _, pattern in _rows(table, "retrieval_verbs"))


def _prompts(section, table: dict) -> list:
    return [item for item in _items(section) if _is_prompt(item, table)]


def _show(doc: object, show) -> str:
    return show(Path(getattr(doc, "path", "?")))


def _scan(doc: object, table: dict, group: str, *, quotes: bool = True) -> list:
    body = _prose(getattr(doc, "body", ""), quotes=quotes)
    return _core().scan(body, _rows(table, group), getattr(doc, "body_line", 1))


# --- the classes: each returns the findings for one document -----------------------------------


def _retrieval_prompts(doc, table, show) -> list:
    where = _show(doc, show)
    retrieval = _section(doc, table["sections"]["retrieval"])
    if retrieval is None:
        return [f"{where}: no retrieval section; a reading ends with retrieval prompts"]
    findings = []
    last = doc.sections[-1]
    if last.id != retrieval.id:
        findings.append(
            f"{where}:{retrieval.line}: retrieval is not the last section ({last.id} follows it)"
        )
    count = len(_prompts(retrieval, table))
    floor = table["thresholds"]["retrieval_min_prompts"]
    if count < floor:
        findings.append(
            f"{where}:{retrieval.line}: the retrieval section holds {count} prompt(s), "
            f"fewer than {floor}"
        )
    return findings


def _answers_hidden(doc, table, show) -> list:
    where = _show(doc, show)
    genre = genre_for(doc.frontmatter.get("duty"), table)
    rows = _rows(table, "answer_labels")
    findings = []
    for section_id in table["sections"]["answer_bearing"].get(genre, []):
        section = _section(doc, section_id)
        if section is None:
            continue
        for item in _items(section):
            for row_id, pattern in rows:
                match = pattern.search(item.visible)
                if match:
                    findings.append(
                        f"{where}:{item.line}: {row_id} shows the answer before the learner "
                        f"tries: {match.group(0)!r}; hide it in <details>, a spoiler or ||...||"
                    )
                    break
    return findings


def _ask_before_tell(doc, table, show) -> list:
    where = _show(doc, show)
    sections = table["sections"]
    if genre_for(doc.frontmatter.get("duty"), table) == "practice":
        questions = _section(doc, sections["questions"])
        explanations = _section(doc, sections["explanations"])
        if questions is None:
            return [f"{where}: the practice set has no {sections['questions']} section"]
        if explanations is not None and explanations.line < questions.line:
            return [
                f"{where}:{explanations.line}: explanations (line {explanations.line}) precede "
                f"questions (line {questions.line}); ask before you tell"
            ]
        return []
    drill = _section(doc, sections["drill"])
    if drill is None:
        return [f"{where}: no {sections['drill']} section"]
    items = _items(drill)
    if not items:
        return []
    if not _is_prompt(items[0], table):
        return [
            f"{where}:{items[0].line}: the drill opens with an item that asks nothing"
        ]
    return []


def _deep_questions(doc, table, show) -> list:
    where = _show(doc, show)
    retrieval = _section(doc, table["sections"]["retrieval"])
    if retrieval is None:
        return [f"{where}: no retrieval section to hold a why or how prompt"]
    rows = _rows(table, "explanatory")
    for item in _prompts(retrieval, table):
        if any(pattern.search(_text_of(item)) for _, pattern in rows):
            return []
    return [f"{where}:{retrieval.line}: no retrieval prompt asks why or how"]


def _concrete_examples(doc, table, show) -> list:
    where = _show(doc, show)
    present = {section.id for section in getattr(doc, "sections", ())}
    if present & set(table["sections"]["concrete"]):
        return []
    reading = _section(doc, table["sections"]["reading"])
    text = reading.body if reading is not None else getattr(doc, "body", "")
    first = reading.line + 1 if reading is not None else getattr(doc, "body_line", 1)
    prose = _prose(text)
    if QUOTED.search(_fold(prose)):
        return []
    if _core().scan(prose, _rows(table, "example_markers"), first):
        return []
    return [
        f"{where}: no concrete example in the reading: no example marker, no quoted example "
        "and no worked-example or application section"
    ]


def _interleaving(doc, table, show) -> list:
    where = _show(doc, show)
    sections = table["sections"]
    genre = genre_for(doc.frontmatter.get("duty"), table)
    section = _section(
        doc, sections["questions"] if genre == "practice" else sections["drill"]
    )
    if section is None:
        return []
    topics = [item.topic for item in _prompts(section, table) if item.topic]
    distinct = len(set(topics))
    if (
        len(topics) < table["thresholds"]["interleaving_min_prompts"]
        or distinct < table["thresholds"]["interleaving_min_topics"]
    ):
        return []
    runs = 1 + sum(1 for before, after in zip(topics, topics[1:]) if before != after)
    if runs > distinct:
        return []
    return [
        f"{where}:{section.line}: {len(topics)} prompts over {distinct} topics run as {runs} "
        "blocks; interleave them"
    ]


def _worked_example_steps(doc, table, show) -> list:
    where = _show(doc, show)
    example = _section(doc, table["sections"]["worked_example"])
    if example is None:
        return []
    findings = []
    items = _items(example)
    steps = [item for item in items if item.number is not None]
    labels = [
        index
        for index, line in enumerate(_prose(example.body, hide=True).split("\n"))
        if STEP_LABEL.match(line) and not ITEM.match(line)
    ]
    count = len(steps) + len(labels)
    floor = table["thresholds"]["worked_example_min_steps"]
    if count < floor:
        findings.append(
            f"{where}:{example.line}: the worked example shows {count} step(s), fewer than {floor}"
        )
    last_step = max(
        [item.line for item in steps] + [example.line + 1 + index for index in labels],
        default=example.line,
    )
    after = [item for item in _prompts(example, table) if item.line > last_step]
    for section in doc.sections:
        if section.line > example.line and section.id != example.id:
            after += _prompts(section, table)
    if not after:
        findings.append(
            f"{where}:{example.line}: no prompt follows the worked example; pair it with a problem"
        )
    return findings


def _study_advice(doc, table, show) -> list:
    where = _show(doc, show)
    return [
        f"{where}:{line}: {row_id} recommends a low-utility study method: {found!r}"
        for line, row_id, found in _scan(doc, table, "low_utility_advice")
    ]


def _key_terms_defined(doc, table, show) -> list:
    where = _show(doc, show)
    key_terms = _section(doc, table["sections"]["key_terms"])
    if key_terms is None:
        return []
    items = _items(key_terms)
    if not items:
        return [f"{where}:{key_terms.line}: the key-terms section holds no term item"]
    other = _fold(
        "\n".join(
            _prose(section.body, hide=True)
            for section in doc.sections
            if section.id != key_terms.id
        )
    ).casefold()
    findings = []
    seen: dict = {}
    for item in items:
        parsed = TERM.match(_item_body(item))
        term = (
            (parsed["bold"] or parsed["under"] or parsed["plain"] or "").strip()
            if parsed
            else ""
        )
        definition = (parsed["definition"] or "").strip() if parsed else ""
        if not term:
            findings.append(f"{where}:{item.line}: a key-terms item names no term")
            continue
        folded = _fold(term).casefold()
        if folded in seen:
            findings.append(
                f"{where}:{item.line}: key term {term!r} is listed twice (line {seen[folded]})"
            )
            continue
        seen[folded] = item.line
        if not definition:
            findings.append(f"{where}:{item.line}: key term {term!r} has no definition")
        if folded not in other:
            findings.append(
                f"{where}:{item.line}: key term {term!r} is never used outside key-terms"
            )
    return findings


def _feedback_task_focused(doc, table, show) -> list:
    where = _show(doc, show)
    return [
        f"{where}:{line}: {row_id} addresses the person or the cohort, not the work: {found!r}"
        for line, row_id, found in _scan(doc, table, "person_focused", quotes=False)
    ]


def _feedback_next_step(doc, table, show) -> list:
    where = _show(doc, show)
    names = table["sections"]["feed_forward"]
    step = next((s for s in getattr(doc, "sections", ()) if s.id in names), None)
    if step is None:
        return [
            f"{where}: no next-step section ({', '.join(names)}); feedback ends with where to "
            "go next"
        ]
    if not _prose(step.body, hide=True).strip():
        return [f"{where}:{step.line}: the {step.id} section is empty"]
    return []


JUDGES = {
    "retrieval-prompts": _retrieval_prompts,
    "answers-hidden": _answers_hidden,
    "ask-before-tell": _ask_before_tell,
    "deep-questions": _deep_questions,
    "concrete-examples": _concrete_examples,
    "interleaving": _interleaving,
    "worked-example-steps": _worked_example_steps,
    "study-advice": _study_advice,
    "key-terms-defined": _key_terms_defined,
    "feedback-task-focused": _feedback_task_focused,
    "feedback-next-step": _feedback_next_step,
}


def findings(class_id: str, doc: object, pedagogy: "dict | None" = None) -> list:
    """The findings of one class on one document, `<path>:<line>: <finding>`; [] when it does
    not apply. `doc` is persona-core's Document or any object of its shape."""
    if class_id not in OUTPUT_CLASSES:
        raise ValueError(f"{class_id} is not an output class of learning-science")
    table = pedagogy if pedagogy is not None else load_pedagogy()
    if not applies(class_id, doc, table):
        return []
    return JUDGES[class_id](doc, table, str)


# --- the population and the table class ---------------------------------------------------------


def _table_outcome(pedagogy_path: "Path | None") -> Outcome:
    where = Path(pedagogy_path) if pedagogy_path is not None else DEFAULT_PEDAGOGY
    try:
        table = json.loads(where.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise PedagogyError(f"{where} cannot be read as JSON: {error}") from error
    contract = _core().load_contract()
    problems = [f"{where.name}: {problem}" for problem in _table_problems(table)]
    registry = [entry["id"] for entry in contract["duties"]]
    duties = table.get("duties") if isinstance(table, dict) else None
    duties = duties if isinstance(duties, dict) else {}
    for duty in registry:
        if duty not in duties:
            problems.append(
                f"{where.name}: duty {duty} in persona-core's registry has no genre"
            )
    for duty in duties:
        if duty not in registry:
            problems.append(
                f"{where.name}: duty {duty} is not in persona-core's registry"
            )
    kinds = set(contract["kinds"])
    classes = table.get("classes") if isinstance(table, dict) else None
    for name, entry in (classes if isinstance(classes, dict) else {}).items():
        for kind in (entry.get("kinds") or []) if isinstance(entry, dict) else []:
            if kind not in kinds:
                problems.append(
                    f"{where.name}: class {name} names unknown kind {kind!r}"
                )
    examined = len(registry) + len(classes if isinstance(classes, dict) else {})
    return Outcome(examined, tuple(problems))


def evaluate(
    class_id: str,
    root: "Path | str",
    subjects: tuple = (),
    kinds: tuple = (),
    duties: tuple = (),
    pedagogy_path: "Path | str | None" = None,
) -> Outcome:
    """Run one class over the persona outputs under `root` (or its `subjects`)."""
    if class_id not in CLASSES:
        raise ValueError(f"{class_id} is not a class of learning-science")
    if class_id == TABLE_CLASS:
        return _table_outcome(
            Path(pedagogy_path) if pedagogy_path is not None else None
        )
    table = load_pedagogy(pedagogy_path)
    core = _core()
    root = Path(root)
    bases = [
        root / subject if not Path(subject).is_absolute() else Path(subject)
        for subject in subjects
    ]
    docs, broken = core.load_population(bases or [root], list(kinds))

    def show(path: Path) -> str:
        try:
            return str(path.resolve().relative_to(root.resolve()))
        except ValueError:
            return str(path)

    found: list = []
    examined = 0
    for doc in docs:
        if doc.kind != "output":
            continue
        if duties and doc.frontmatter.get("duty") not in duties:
            continue
        if not applies(class_id, doc, table):
            continue
        examined += 1
        found += JUDGES[class_id](doc, table, show)
    for item in broken:
        if item.guess != "output":
            continue
        examined += 1
        found.append(f"{show(item.path)}: does not parse: {item.reason}")
    return Outcome(examined, tuple(found))


def main(argv: "list | None" = None) -> int:
    parser = argparse.ArgumentParser(
        prog="learning-science-probe.py",
        description="The shared pedagogy classes every AI study text follows (SPEC-V2-2224).",
    )
    parser.add_argument(
        "--root", type=Path, help="the repository whose persona outputs are judged"
    )
    parser.add_argument(
        "--subject", action="append", default=[], help="a directory or file to read"
    )
    parser.add_argument(
        "--kind", action="append", default=[], help="only outputs of this subject kind"
    )
    parser.add_argument(
        "--duty", action="append", default=[], help="only outputs of this duty"
    )
    parser.add_argument(
        "--pedagogy", type=Path, help="the pedagogy table (default: the pack's)"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("classes", help="list the classes")
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    args = parser.parse_args(argv)
    if args.command == "classes":
        print("\n".join(CLASSES))
        return EXIT_GREEN
    if args.root is None:
        parser.error("check needs --root")
    try:
        outcome = evaluate(
            args.name,
            args.root,
            tuple(args.subject),
            tuple(args.kind),
            tuple(args.duty),
            args.pedagogy,
        )
    except PedagogyError as error:
        print(f"{args.name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{args.name}: {finding}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())

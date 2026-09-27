#!/usr/bin/env python3
"""law-professors-probe.py -- the law checks for DeckStreak's professor personas.

SPEC-V2-2214. The law-professors pack ships thirteen professor TEMPLATES, one per NCBE-derived
area, and DeckStreak's engine writes law OUTPUT: a rule primer in IRAC, a Socratic turn, an
issue-spotter hypothetical, a graded essay, a leech explanation or a drill. This script judges both,
in sixteen classes, beside persona-core's eight (which the pack's walk runs with `--kind law`):

    law-area-registry     block     areas.json: 13 well-formed NCBE areas, each with its template
    law-template-area     block     a template's area is registered; its outline and assumptions match
    law-template-methods  block     a template carries the four law methods and their sections
    irac-structure        block     a rule primer has issue, rule, application, conclusion, in order
    irac-analysis         advisory  the application uses the facts; the conclusion answers the issue
    rule-cites-corpus     block     every rule statement carries a citation
    citations-resolve     block     sources declared and cited, and every source in the corpus
    quotes-grounded       block     a rule statement's quote is in the source it cites
    authority-grounded    block     no case, statute or rule is named without a citation
    socratic-prompts      block     tagged questions, at most five, that do not state the answer
    socratic-coverage     advisory  the prompts span categories, and reach a deep one
    hypo-structure        block     a fact pattern, a call, and issue blocks with trigger and rule
    hypo-anchored         block     every quoted trigger or immaterial fact is in the fact pattern
    hypo-depth            advisory  two issues or more, and an immaterial fact
    grading-cites         block     the NCBE criteria scored and cited, a true total, a cited model
    grading-feedforward   advisory  next steps name a criterion, and are more than praise

It parses NOTHING itself. persona-core's probe (`scripts/persona-core-probe.py`, SPEC-V2-2212) is
the executable contract: it is loaded with importlib, and its `load_population` and
`parse_document` read every file. A law document is one whose subject kind is `law`.

It is VENDORABLE on purpose: standard library only, and every class reads any tree through
`--root`, or the paths `--subject` names. Every class prints one line per finding,
`<class>: <finding>`, and ends with `examined N`. Exit 0 is green, 1 a finding, 2 a usage error,
and 3 VOID: nothing was examined, or an input (the registry, persona-core's probe) could not be
read, which is never a pass.
"""

import argparse
import importlib.util
import json
import re
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

AREAS_SCHEMA = "phx.law.areas.v1"
CORPUS_SCHEMA = "phx.law.corpus.v1"
CORPUS_NAME = "corpus.json"
LAW_KIND = "law"
AREA_COUNT = 13
HERE = Path(__file__).resolve().parent
DEFAULT_AREAS = HERE.parent / "skills" / "packs" / "law-professors" / "areas.json"
DEFAULT_PERSONA_CORE = HERE / "persona-core-probe.py"

# The four law methods are four duties, each adding its sections to the duty registry's.
METHOD_DUTIES = (
    "daily-reading",
    "conversation-partner",
    "practice-questions",
    "writing-tutor",
)
LAW_DUTY_SECTIONS = {
    "daily-reading": ("issue", "rule", "application", "conclusion"),
    "drill-coach": ("rule",),
    "leech-doctor": ("rule", "trap"),
    "writing-tutor": ("rubric", "model", "next-step"),
    "conversation-partner": ("socratic",),
    "practice-questions": ("facts", "issues"),
}
IRAC = ("issue", "rule", "application", "conclusion")
# A `rule` section alone is the leech-doctor's and the drill-coach's; only these make a primer.
IRAC_SIGNALS = ("issue", "application", "conclusion")
TEMPLATE_LAW_SECTIONS = (
    "outline",
    "assumptions",
    "traps",
    "questioning",
    "hypotheticals",
    "grading",
)
METHOD_NAMES = (
    ("IRAC", re.compile(r"\bIRAC\b")),
    ("Socratic", re.compile(r"\bsocratic\b", re.IGNORECASE)),
    ("issue-spotter", re.compile(r"\bissue[- ]spott", re.IGNORECASE)),
    ("grading", re.compile(r"\bgrad(?:e|es|ed|ing)\b", re.IGNORECASE)),
)
SOCRATIC_CATEGORIES = (
    "clarify",
    "assumptions",
    "evidence",
    "viewpoints",
    "implications",
    "question",
)
DEEP_CATEGORIES = frozenset({"assumptions", "evidence", "viewpoints", "implications"})
NCBE_CRITERIA = ("facts", "issues", "rules", "reasoning")
CRITERIA = (*NCBE_CRITERIA, "writing")
CITED_CRITERIA = frozenset({"issues", "rules", "reasoning"})
MAX_PROMPTS = 5
MIN_FACT_WORDS = 40
MIN_QUOTE_WORDS = 3
MIN_TRAPS = 3
MIN_ISSUES = 2
EXCERPT = 30

REGISTRY_KEYS = frozenset({"schema", "spec", "pack", "note", "formats", "areas"})
AREA_KEYS = frozenset(
    {"id", "name", "ncbe_names", "formats", "outline", "assumptions", "template"}
)
CORPUS_KEYS = frozenset({"schema", "subject", "sources"})
SOURCE_KEYS = frozenset({"id", "title", "text"})

SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
LAW_SUBJECT = re.compile(r"^law/(?P<area>[a-z0-9]+(?:-[a-z0-9]+)*)$")
# Pandoc's citation key: a letter, digit or underscore, then alphanumerics and SINGLE internal
# punctuation. A final punctuation mark is not part of the key, and a repeated one ends it.
KEY_PATTERN = r"[A-Za-z0-9_](?:[A-Za-z0-9_]|[:.#$%&\-+?<>~/](?=[A-Za-z0-9_]))*"
KEY = re.compile(rf"^{KEY_PATTERN}$")
BRACKET = re.compile(r"\[(?P<inner>[^\[\]\n]*)\](?![(\[])")
CITE_ITEM = re.compile(
    rf"^\s*(?:(?P<prefix>.*?)\s)?-?@(?:\{{(?P<braced>[^{{}}]+)\}}|(?P<key>{KEY_PATTERN}))"
)

LIST_ITEM = re.compile(r"^ {0,3}(?:[-*+]|[0-9]{1,9}[.)])(?:[ \t]+|$)")
FENCE = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})")
HEADING = re.compile(r"^ {0,3}#{1,6}(?:[ \t]|$)")
SUBHEADING = re.compile(r"^ {0,3}###(?:[ \t]+(?P<title>.*?))?[ \t]*$")
BLOCKQUOTE = re.compile(r"^(?: {0,3}>[ ]?)+")
COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
LABEL = re.compile(r"^(?P<label>Rule|Trigger|Immaterial|Total):")
ANSWER = re.compile(r"^answer\b", re.IGNORECASE)
ASK_TAG = re.compile(r"<!--\s*ask:(?P<category>[^\s>]*)\s*-->")
QUOTED = re.compile(r'"(?P<straight>[^"]+)"|“(?P<curly>[^”]+)”')
ELLIPSIS = re.compile(r"\.\.\.|…")
CRITERION = re.compile(
    r"^`?(?P<name>[a-z][a-z-]*)`?\s*:\s*(?P<got>[0-9]+)\s*/\s*(?P<max>[0-9]+)(?![0-9])"
)
TOTAL = re.compile(r"^Total:\s*(?P<got>[0-9]+)\s*/\s*(?P<max>[0-9]+)\s*$")
CRITERION_WORD = re.compile(
    r"\b(?:facts|issues|rules|reasoning|writing)\b", re.IGNORECASE
)
PRAISE = re.compile(
    r"\b(?:great|excellent|amazing|awesome|brilliant|fantastic|well done|good job|nice work)\b",
    re.IGNORECASE,
)
WHETHER = re.compile(r"\bwhether\b", re.IGNORECASE)
CONTENT_WORD = re.compile(r"[A-Za-z][A-Za-z'’-]{4,}")
STOPWORDS = frozenset(
    {
        "about",
        "above",
        "after",
        "again",
        "against",
        "along",
        "among",
        "because",
        "before",
        "being",
        "below",
        "between",
        "cannot",
        "could",
        "every",
        "first",
        "found",
        "their",
        "there",
        "these",
        "thing",
        "think",
        "those",
        "three",
        "through",
        "under",
        "until",
        "where",
        "which",
        "while",
        "would",
        "other",
        "shall",
        "should",
        "still",
        "since",
        "might",
        "never",
        "often",
        "whether",
        "without",
        "within",
        "rather",
        "itself",
    }
)
# A sentence ends at . ! or ? before a capital, unless the word before it is an abbreviation:
# "Fed. R. Evid. 403", "U.S.C.", "v." or "art." never end a sentence.
SENTENCE_END = re.compile(r"(?<=[.!?])\s+(?=[A-Z\"“(])")
ABBREVIATION = re.compile(
    r"(?:\b[A-Z][a-z]{0,4}|\b[A-Z](?:\.[A-Z])+|\b(?:v|vs|art|ch|cl|no|p|pp|para|id|cf|e\.g|i\.e))\.$"
)
# Named legal authority, the forms eyecite recognises (full case, short form, law), as tokens.
AUTHORITY = (
    (
        "a reporter citation",
        re.compile(
            r"\b[0-9]{1,4}\s+(?:[A-Z][A-Za-z]{0,7}\.\s?){1,4}(?:(?:2d|3d|4th|5th|6th)\s+)?"
            r"[0-9]{1,5}\b"
        ),
    ),
    (
        "a case name",
        re.compile(r"\b[A-Z][A-Za-z'&.\-]*(?:\s+[A-Z][A-Za-z'&.\-]*)*\s+vs?\.\s+[A-Z]"),
    ),
    ("the United States Code", re.compile(r"\b[0-9]+\s+U\.\s?S\.\s?C\.")),
    ("a section sign", re.compile(r"§")),
    ("a federal rule", re.compile(r"\bFed\.\s?R\.\s?(?:Civ|Crim|App|Evid|Bankr)\.")),
    ("a rule abbreviation", re.compile(r"\b(?:FRE|FRCP|FRCrP|FRAP)\s*[0-9]")),
    ("a numbered rule", re.compile(r"\bRules?\s+[0-9]+(?:\.[0-9]+)?")),
    (
        "the Uniform Commercial Code",
        re.compile(r"\bU\.?\s?C\.?\s?C\.?\s*(?:§+\s*)?[0-9]+-[0-9]+"),
    ),
    ("a Restatement", re.compile(r"\bRestatement\b")),
    (
        "a Model Rule or Code",
        re.compile(r"\bModel\s+(?:Rules?|Code|Penal\s+Code)\b|\bMRPC\s*[0-9]"),
    ),
    (
        "a constitutional amendment",
        re.compile(
            r"\b(?:First|Second|Third|Fourth|Fifth|Sixth|Seventh|Eighth|Ninth|Tenth|Eleventh|"
            r"Twelfth|Thirteenth|Fourteenth|Fifteenth|Sixteenth|Seventeenth|Eighteenth|"
            r"Nineteenth|Twentieth|Twenty-[A-Za-z]+)\s+Amendment\b"
        ),
    ),
    ("the Constitution", re.compile(r"\bU\.\s?S\.\s?Const\.")),
)
# A fact pattern, the calls of the question, a Socratic prompt, an issue statement and a daily
# reading's retrieval prompts (study-duties' last section) ask; they assert no law, so the
# authority they name owes no citation.
ASKING_SECTIONS = frozenset({"facts", "questions", "socratic", "issue", "retrieval"})
QUOTE_MAP = str.maketrans({"“": '"', "”": '"', "‘": "'", "’": "'"})

CLASSES = (
    ("law-area-registry", "areas", "block"),
    ("law-template-area", "areas", "block"),
    ("law-template-methods", "templates", "block"),
    ("irac-structure", "irac", "block"),
    ("irac-analysis", "irac", "advisory"),
    ("rule-cites-corpus", "citations", "block"),
    ("citations-resolve", "citations", "block"),
    ("quotes-grounded", "citations", "block"),
    ("authority-grounded", "citations", "block"),
    ("socratic-prompts", "socratic", "block"),
    ("socratic-coverage", "socratic", "advisory"),
    ("hypo-structure", "hypos", "block"),
    ("hypo-anchored", "hypos", "block"),
    ("hypo-depth", "hypos", "advisory"),
    ("grading-cites", "grading", "block"),
    ("grading-feedforward", "grading", "advisory"),
)
CLASS_NAMES = tuple(name for name, _stage, _severity in CLASSES)


class Void(Exception):
    """An input the class needs could not be read, so nothing can be judged: VOID, never green."""


@dataclass(frozen=True)
class Statement:
    """One item, paragraph or labelled line of a section. `text` drops comments and extra space."""

    line: int
    raw: str
    text: str
    item: bool
    label: str


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass
class Context:
    root: Path
    bases: list
    core: object
    docs: list
    broken: list
    areas_path: Path
    corpus_path: "Path | None"

    def show(self, path: Path) -> str:
        try:
            return str(path.resolve().relative_to(self.root.resolve()))
        except ValueError:
            return str(path)

    def outputs(self) -> list:
        return [doc for doc in self.docs if doc.kind == "output"]

    def templates(self) -> list:
        return [doc for doc in self.docs if doc.kind == "template"]

    def broken_of(self, *guesses: str) -> list:
        return [broken for broken in self.broken if broken.guess in guesses]


# --- reading: persona-core, the registry, statements and citations ---------------------------------


def load_core(path: Path) -> object:
    """persona-core's probe as a module; its parser is the contract, and it is never copied."""
    spec = importlib.util.spec_from_file_location("persona_core_probe", path)
    if spec is None or spec.loader is None:
        raise Void(f"persona-core probe {path} cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except (OSError, SyntaxError, ImportError) as error:
        raise Void(f"persona-core probe {path} cannot be loaded ({error})") from error
    for name in ("load_population", "parse_document", "ContractError"):
        if not hasattr(module, name):
            raise Void(f"persona-core probe {path} exports no {name}")
    return module


def load_registry(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Void(f"the area registry {path} is unreadable ({error})") from error
    if not isinstance(data, dict):
        raise Void(f"the area registry {path} is not a JSON object")
    return data


def registered_areas(ctx: Context) -> dict:
    data = load_registry(ctx.areas_path)
    areas = data.get("areas")
    return {
        record["id"]: record
        for record in (areas if isinstance(areas, list) else [])
        if isinstance(record, dict) and isinstance(record.get("id"), str)
    }


def section(doc: object, section_id: str) -> "object | None":
    return next((part for part in doc.sections if part.id == section_id), None)


def section_statements(part: object) -> list:
    return statements(part.body.split("\n"), part.line + 1)


def statements(lines: list, first: int) -> list:
    """Split lines into statements: list items, paragraphs, and single-line labelled statements.

    Fenced code and headings are skipped, a blockquote marker is dropped, and a labelled line
    (`Rule:`, `Trigger:`, `Immaterial:`, `Total:`) is one statement on its own, so a citation on the
    next line never answers for it.
    """
    found: list = []
    current: "list | None" = None
    fence: "str | None" = None

    def flush() -> None:
        nonlocal current
        if current is not None:
            raw = " ".join(current[1])
            text = " ".join(COMMENT.sub(" ", raw).split())
            found.append(Statement(current[0], raw, text, current[2], current[3]))
        current = None

    for offset, line in enumerate(lines):
        number = first + offset
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
            continue
        opened = FENCE.match(line)
        if opened:
            flush()
            fence = opened["fence"]
            continue
        if not line.strip() or HEADING.match(line):
            flush()
            continue
        stripped = BLOCKQUOTE.sub("", line)
        item = LIST_ITEM.match(stripped)
        content = stripped[item.end() :].strip() if item else stripped.strip()
        label = LABEL.match(content)
        if item or label or current is None or current[3]:
            flush()
            current = [number, [content], bool(item), label["label"] if label else ""]
            if label and not item:
                flush()
            continue
        current[1].append(content)
    flush()
    return found


def citation_keys(text: str) -> list:
    """Every Pandoc citation key in `text`: `[@key]`, `[see @key, p. 3]`, `[@a; @b]`. Links are not."""
    keys: list = []
    for bracket in BRACKET.finditer(text):
        for part in bracket["inner"].split(";"):
            match = CITE_ITEM.match(part)
            if match:
                keys.append(match["braced"] or match["key"])
    return keys


def body_citations(doc: object) -> list:
    keys: list = []
    fence: "str | None" = None
    for line in doc.body.split("\n"):
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
        keys += citation_keys(COMMENT.sub(" ", line))
    return keys


def rule_statements(doc: object) -> list:
    """Every item or paragraph of a `rule` section, and every `Rule:` line in any section."""
    found: list = []
    for part in doc.sections:
        for statement in section_statements(part):
            if part.id == "rule" or statement.label == "Rule":
                found.append(statement)
    return found


def quotes(text: str, min_words: int) -> list:
    spans = [match["straight"] or match["curly"] for match in QUOTED.finditer(text)]
    return [span.strip() for span in spans if len(span.split()) >= min_words]


def normalize(text: str) -> str:
    text = text.translate(QUOTE_MAP).replace("[", "").replace("]", "")
    return " ".join(text.split()).casefold()


def fragments(span: str) -> list:
    parts = [normalize(part).strip(" \t.,;:!?") for part in ELLIPSIS.split(span)]
    return [part for part in parts if part]


def contains(haystack: str, span: str) -> bool:
    """Whether every fragment of the quote appears in the normalized haystack, in order."""
    position = 0
    for fragment in fragments(span):
        found = haystack.find(fragment, position)
        if found < 0:
            return False
        position = found + len(fragment)
    return bool(fragments(span))


def excerpt(text: str, limit: int = 80) -> str:
    return text if len(text) <= limit else text[: limit - 3] + "..."


def content_words(text: str) -> set:
    words = set()
    for word in CONTENT_WORD.findall(text):
        word = word.casefold().translate(QUOTE_MAP)
        word = word.removesuffix("'s").strip("'-")
        if len(word) >= 5 and word not in STOPWORDS:
            words.add(word)
    return words


def sentences(text: str) -> list:
    """Split at sentence ends, but never after an abbreviation such as `Fed.`, `U.S.C.` or `v.`."""
    found: list = []
    start = 0
    for boundary in SENTENCE_END.finditer(text):
        head = text[start : boundary.start()]
        if ABBREVIATION.search(head.rsplit(" ", 1)[-1] if " " in head else head):
            continue
        found.append(head)
        start = boundary.end()
    found.append(text[start:])
    return [sentence for sentence in found if sentence.strip()]


# --- the corpus ------------------------------------------------------------------------------------


def corpus_problems(data: object) -> list:
    if not isinstance(data, dict):
        return ["is not a JSON object"]
    problems = [f"unknown key {key}" for key in data if key not in CORPUS_KEYS]
    if data.get("schema") != CORPUS_SCHEMA:
        problems.append(f"schema {data.get('schema')!r} is not {CORPUS_SCHEMA}")
    subject = data.get("subject")
    if not isinstance(subject, str) or not LAW_SUBJECT.match(subject):
        problems.append(f"subject {subject!r} is not law/<area>")
    sources = data.get("sources")
    if not isinstance(sources, list) or not sources:
        return problems + ["sources is not a non-empty list"]
    seen: set = set()
    for index, entry in enumerate(sources):
        if not isinstance(entry, dict):
            problems.append(f"source {index} is not an object")
            continue
        problems += [
            f"source {index}: unknown key {key}"
            for key in entry
            if key not in SOURCE_KEYS
        ]
        source_id = entry.get("id")
        if not isinstance(source_id, str) or not KEY.match(source_id):
            problems.append(f"source {index}: id {source_id!r} is not a citation key")
        elif source_id in seen:
            problems.append(f"source {source_id} is listed twice")
        else:
            seen.add(source_id)
        for field in ("title", "text"):
            value = entry.get(field)
            if not isinstance(value, str) or not value.strip():
                problems.append(f"source {source_id or index} has no {field}")
    return problems


class Corpora:
    """Reads each corpus manifest once, and finds the one that answers for an output."""

    def __init__(self, ctx: Context) -> None:
        self.ctx = ctx
        self.cache: dict = {}

    def base_of(self, path: Path) -> Path:
        resolved = path.resolve()
        holders = []
        for base in self.ctx.bases:
            base = Path(base).resolve()
            directory = base.parent if base.is_file() else base
            if resolved == directory or directory in resolved.parents:
                holders.append(directory)
        return (
            max(holders, key=lambda holder: len(holder.parts))
            if holders
            else resolved.parent
        )

    def locate(self, path: Path) -> "Path | None":
        if self.ctx.corpus_path is not None:
            return self.ctx.corpus_path
        base = self.base_of(path)
        directory = path.resolve().parent
        while True:
            candidate = directory / CORPUS_NAME
            if candidate.is_file():
                return candidate
            if directory == base or directory == directory.parent:
                return None
            directory = directory.parent

    def read(self, path: Path) -> tuple:
        """(data or None, problems) for the manifest at `path`."""
        key = path.resolve()
        if key not in self.cache:
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
                self.cache[key] = (None, [f"is not readable JSON ({error})"])
            else:
                problems = corpus_problems(data)
                self.cache[key] = (None if problems else data, problems)
        return self.cache[key]

    def texts(self, doc: object) -> "dict | None":
        """{source id: normalized text} of the corpus that answers for `doc`, or None."""
        path = self.locate(doc.path)
        if path is None:
            return None
        data, _problems = self.read(path)
        if data is None:
            return None
        return {entry["id"]: normalize(entry["text"]) for entry in data["sources"]}


# --- the areas and the templates -------------------------------------------------------------------


def _string_list(value: object, *, empty_ok: bool) -> bool:
    return (
        isinstance(value, list)
        and (empty_ok or bool(value))
        and all(isinstance(item, str) and item.strip() for item in value)
        and len(set(value)) == len(value)
    )


def area_problems(record: object, vocabulary: dict, seen: set) -> list:
    if not isinstance(record, dict):
        return ["an area is not an object"]
    area_id = record.get("id")
    name = area_id if isinstance(area_id, str) else repr(area_id)
    problems = [
        f"area {name}: unknown key {key}" for key in record if key not in AREA_KEYS
    ]
    problems += [
        f"area {name}: {key} is missing"
        for key in sorted(AREA_KEYS)
        if key not in record
    ]
    if not isinstance(area_id, str) or not SLUG.match(area_id):
        problems.append(f"area id {name} is not a slug")
    elif area_id in seen:
        problems.append(f"area {area_id} is repeated")
    else:
        seen.add(area_id)
    if not isinstance(record.get("name"), str) or not record.get("name", "").strip():
        problems.append(f"area {name}: name is empty")
    formats = record.get("formats")
    if not _string_list(formats, empty_ok=False):
        problems.append(f"area {name}: formats is not a non-empty list of unique codes")
        formats = []
    for code in formats:
        if code not in vocabulary:
            problems.append(
                f"area {name}: format {code} is not in the registry's formats"
            )
    names = record.get("ncbe_names")
    if not isinstance(names, dict) or not names:
        problems.append(f"area {name}: ncbe_names is not a non-empty object")
    else:
        for code, value in names.items():
            if code not in formats:
                problems.append(
                    f"area {name}: ncbe_names names {code}, not one of its formats"
                )
            if not isinstance(value, str) or not value.strip():
                problems.append(f"area {name}: the {code} name is empty")
    if not _string_list(record.get("outline"), empty_ok=False):
        problems.append(
            f"area {name}: outline is not a non-empty list of unique headings"
        )
    if not _string_list(record.get("assumptions"), empty_ok=True):
        problems.append(f"area {name}: assumptions is not a list of unique statements")
    expected = f"templates/law-{area_id}.persona.md"
    if record.get("template") != expected:
        problems.append(
            f"area {name}: template {record.get('template')!r} is not {expected}"
        )
    return problems


def check_law_area_registry(ctx: Context) -> Outcome:
    data = load_registry(ctx.areas_path)
    findings: list = []
    where = ctx.show(ctx.areas_path)
    findings += [
        f"{where}: unknown key {key}" for key in data if key not in REGISTRY_KEYS
    ]
    if data.get("schema") != AREAS_SCHEMA:
        findings.append(f"{where}: schema {data.get('schema')!r} is not {AREAS_SCHEMA}")
    vocabulary = data.get("formats")
    if (
        not isinstance(vocabulary, dict)
        or not vocabulary
        or not all(
            SLUG.match(code) and isinstance(text, str) and text
            for code, text in vocabulary.items()
        )
    ):
        findings.append(
            f"{where}: formats is not a non-empty map of slug codes to descriptions"
        )
        vocabulary = vocabulary if isinstance(vocabulary, dict) else {}
    areas = data.get("areas")
    if not isinstance(areas, list):
        findings.append(f"{where}: areas is not a list")
        areas = []
    if areas and len(areas) != AREA_COUNT:
        findings.append(
            f"{where}: holds {len(areas)} areas; the registry holds {AREA_COUNT}"
        )
    seen: set = set()
    referenced: set = set()
    pack = ctx.areas_path.resolve().parent
    for record in areas:
        problems = area_problems(record, vocabulary, seen)
        findings += [f"{where}: {problem}" for problem in problems]
        if not isinstance(record, dict) or not isinstance(record.get("template"), str):
            continue
        area_id = record.get("id")
        template = pack / record["template"]
        referenced.add(template.resolve())
        if not template.is_file():
            findings.append(
                f"{where}: area {area_id}'s template {record['template']} is missing"
            )
            continue
        try:
            doc = ctx.core.parse_document(template)
        except ctx.core.ContractError as error:
            findings.append(f"{ctx.show(template)}: does not parse ({error})")
            continue
        if doc.kind != "template":
            findings.append(f"{ctx.show(template)}: is a {doc.kind}, not a template")
        if doc.frontmatter.get("template") != f"law-{area_id}":
            findings.append(
                f"{ctx.show(template)}: template {doc.frontmatter.get('template')!r} is not law-{area_id}"
            )
        if doc.frontmatter.get("subject") != f"law/{area_id}":
            findings.append(
                f"{ctx.show(template)}: subject {doc.frontmatter.get('subject')} is not law/{area_id}"
            )
    templates = pack / "templates"
    if templates.is_dir():
        for path in sorted(templates.glob("*.persona.md")):
            if path.resolve() not in referenced:
                findings.append(
                    f"{ctx.show(path)}: a template no registered area names"
                )
    return Outcome(len(areas), tuple(findings))


def list_items(part: object) -> list:
    return [statement.text for statement in section_statements(part) if statement.item]


def check_law_template_area(ctx: Context) -> Outcome:
    registry = registered_areas(ctx)
    findings = [
        f"{ctx.show(b.path)}: does not parse ({b.reason})"
        for b in ctx.broken_of("template", "unknown")
    ]
    templates = ctx.templates()
    for doc in templates:
        where = ctx.show(doc.path)
        subject = doc.frontmatter.get("subject")
        match = LAW_SUBJECT.match(subject) if isinstance(subject, str) else None
        record = registry.get(match["area"]) if match else None
        if record is None:
            findings.append(f"{where}: subject {subject} is not a registered law area")
            continue
        for field in ("outline", "assumptions"):
            part = section(doc, field)
            if part is None:
                findings.append(
                    f"{where}: no {field} section; the registry lists its {field}"
                )
                continue
            listed = list_items(part)
            expected = [" ".join(str(item).split()) for item in record.get(field, [])]
            if listed != expected:
                missing = [item for item in expected if item not in listed]
                extra = [item for item in listed if item not in expected]
                detail = (
                    f"missing {missing[0]!r}"
                    if missing
                    else f"extra {extra[0]!r}"
                    if extra
                    else "reordered"
                )
                findings.append(f"{where}: {field} drifts from the registry ({detail})")
    return Outcome(
        len(templates) + len(ctx.broken_of("template", "unknown")), tuple(findings)
    )


def check_law_template_methods(ctx: Context) -> Outcome:
    findings = [
        f"{ctx.show(b.path)}: does not parse ({b.reason})"
        for b in ctx.broken_of("template", "unknown")
    ]
    templates = ctx.templates()
    for doc in templates:
        where = ctx.show(doc.path)
        duties = doc.frontmatter.get("duties")
        duties = (
            [duty for duty in duties if isinstance(duty, str)]
            if isinstance(duties, list)
            else []
        )
        for duty in METHOD_DUTIES:
            if duty not in duties:
                findings.append(
                    f"{where}: does not declare the {duty} duty, one of the four law methods"
                )
        extra = doc.frontmatter.get("sections")
        extra = extra if isinstance(extra, dict) else {}
        for duty in duties:
            wanted = LAW_DUTY_SECTIONS.get(duty, ())
            added = extra.get(duty) if isinstance(extra.get(duty), list) else []
            missing = [section_id for section_id in wanted if section_id not in added]
            if missing:
                findings.append(f"{where}: sections[{duty}] lacks {', '.join(missing)}")
        for section_id in TEMPLATE_LAW_SECTIONS:
            part = section(doc, section_id)
            if part is None or not COMMENT.sub("", part.body).strip():
                findings.append(f"{where}: no {section_id} section")
        method = section(doc, "method")
        method_text = method.body if method is not None else ""
        for label, pattern in METHOD_NAMES:
            if not pattern.search(method_text):
                findings.append(f"{where}: the method section never names {label}")
        questioning = section(doc, "questioning")
        for category in SOCRATIC_CATEGORIES:
            if questioning is not None and f"ask:{category}" not in questioning.body:
                findings.append(
                    f"{where}: questioning never names the tag ask:{category}"
                )
        grading = section(doc, "grading")
        if grading is not None:
            for criterion in NCBE_CRITERIA:
                if f"`{criterion}`" not in grading.body:
                    findings.append(
                        f"{where}: grading never names the {criterion} criterion"
                    )
            if "Total" not in grading.body:
                findings.append(f"{where}: grading never names the Total line")
        traps = section(doc, "traps")
        if traps is not None and len(list_items(traps)) < MIN_TRAPS:
            findings.append(
                f"{where}: traps lists {len(list_items(traps))}; a professor owns at least {MIN_TRAPS}"
            )
    return Outcome(
        len(templates) + len(ctx.broken_of("template", "unknown")), tuple(findings)
    )


# --- the outputs -------------------------------------------------------------------------------------


def output_population(ctx: Context) -> tuple:
    """(outputs, findings for broken outputs): every class counts both, and reports a broken one."""
    broken = ctx.broken_of("output", "unknown")
    return (
        ctx.outputs(),
        [f"{ctx.show(b.path)}: does not parse ({b.reason})" for b in broken],
        len(broken),
    )


def output_class(ctx: Context, judge: object) -> Outcome:
    outputs, findings, broken = output_population(ctx)
    for doc in outputs:
        findings += [f"{ctx.show(doc.path)}{finding}" for finding in judge(doc, ctx)]
    return Outcome(len(outputs) + broken, tuple(findings))


def has_any(doc: object, ids: Sequence) -> bool:
    return any(section(doc, section_id) is not None for section_id in ids)


def judge_irac_structure(doc: object, ctx: Context) -> list:
    if doc.frontmatter.get("duty") != "daily-reading" and not has_any(
        doc, IRAC_SIGNALS
    ):
        return []
    found: list = []
    missing = [section_id for section_id in IRAC if section(doc, section_id) is None]
    if missing:
        found.append(f": lacks the {', '.join(missing)} section of IRAC")
    present = [
        section(doc, section_id) for section_id in IRAC if section(doc, section_id)
    ]
    if [part.id for part in sorted(present, key=lambda part: part.line)] != [
        part.id for part in present
    ]:
        found.append(
            ": the IRAC sections are out of order (issue, rule, application, conclusion)"
        )
    issue = section(doc, "issue")
    if issue is not None:
        text = COMMENT.sub(" ", issue.body)
        if "?" not in text and not WHETHER.search(text):
            found.append(
                f":{issue.line}: the issue is not stated as a question (a ? or 'whether')"
            )
    return found


def judge_irac_analysis(doc: object, ctx: Context) -> list:
    if not has_any(doc, IRAC_SIGNALS):
        return []
    found: list = []
    grounds = section(doc, "reading") or section(doc, "facts")
    application = section(doc, "application")
    if application is not None:
        facts = (
            content_words(COMMENT.sub(" ", grounds.body))
            if grounds is not None
            else set()
        )
        if not content_words(COMMENT.sub(" ", application.body)) & facts:
            found.append(
                f":{application.line}: the application uses no fact from the reading"
            )
    issue, conclusion = section(doc, "issue"), section(doc, "conclusion")
    if issue is not None and conclusion is not None:
        if not content_words(COMMENT.sub(" ", conclusion.body)) & content_words(
            COMMENT.sub(" ", issue.body)
        ):
            found.append(f":{conclusion.line}: the conclusion never answers the issue")
    return found


def judge_rule_cites_corpus(doc: object, ctx: Context) -> list:
    return [
        f":{statement.line}: a rule statement has no citation: {excerpt(statement.text)!r}"
        for statement in rule_statements(doc)
        if not citation_keys(statement.text)
    ]


def judge_citations_resolve(doc: object, ctx: Context, corpora: Corpora) -> list:
    found: list = []
    if "sources" not in doc.frontmatter:
        return [": sources is missing; a law output declares the corpus ids it cites"]
    sources = doc.frontmatter["sources"]
    if not isinstance(sources, list):
        return [": sources is not a list"]
    if not sources:
        found.append(": sources is empty; a law output cites the corpus")
    declared: list = []
    for key in sources:
        if not isinstance(key, str) or not KEY.match(key):
            found.append(f": source {key!r} is not a citation key")
        elif key in declared:
            found.append(f": source {key} is listed twice")
        else:
            declared.append(key)
    cited = body_citations(doc)
    for key in dict.fromkeys(cited):
        if key not in declared:
            found.append(f": cites {key}, which sources does not declare")
    for key in declared:
        if key not in cited:
            found.append(f": source {key} is never cited in the body")
    path = corpora.locate(doc.path)
    if path is None:
        found.append(
            f": no {CORPUS_NAME} above it and no --corpus; its sources cannot resolve"
        )
        return found
    data, problems = corpora.read(path)
    where = ctx.show(path)
    if data is None:
        return found + [f": corpus {where} {problem}" for problem in problems]
    if data["subject"] != doc.frontmatter.get("subject"):
        found.append(
            f": corpus {where} is for {data['subject']}, not {doc.frontmatter.get('subject')}"
        )
    ids = {entry["id"] for entry in data["sources"]}
    for key in declared:
        if key not in ids:
            found.append(f": source {key} is not in the corpus {where}")
    return found


def judge_quotes_grounded(doc: object, ctx: Context, corpora: Corpora) -> list:
    found: list = []
    texts = corpora.texts(doc)
    for statement in rule_statements(doc):
        spans = quotes(statement.text, MIN_QUOTE_WORDS)
        if not spans:
            continue
        if texts is None:
            found.append(
                f":{statement.line}: cannot ground its quote: no readable corpus"
            )
            continue
        cited = [texts[key] for key in citation_keys(statement.text) if key in texts]
        for span in spans:
            if not any(contains(text, span) for text in cited):
                found.append(
                    f":{statement.line}: quotes {excerpt(span)!r}, which its cited sources do not contain"
                )
    return found


def judge_authority_grounded(doc: object, ctx: Context) -> list:
    found: list = []
    for part in doc.sections:
        if part.id in ASKING_SECTIONS:
            continue
        for statement in section_statements(part):
            if statement.label in ("Trigger", "Immaterial") or citation_keys(
                statement.text
            ):
                continue
            for sentence in sentences(statement.text):
                if sentence.rstrip().endswith("?"):
                    continue
                hit = next(
                    (
                        (what, match)
                        for what, pattern in AUTHORITY
                        if (match := pattern.search(sentence))
                    ),
                    None,
                )
                if hit is not None:
                    what, match = hit
                    named = sentence[match.start() : match.end() + EXCERPT].strip()
                    found.append(
                        f":{statement.line}: names {what} ({named!r}) without a citation"
                    )
                    break
    return found


def prompts(doc: object) -> "list | None":
    part = section(doc, "socratic")
    if part is None:
        return None
    return section_statements(part)


def judge_socratic_prompts(doc: object, ctx: Context) -> list:
    if (
        doc.frontmatter.get("duty") != "conversation-partner"
        and section(doc, "socratic") is None
    ):
        return []
    found: list = []
    every = prompts(doc)
    if every is None:
        return [": no socratic section; a Socratic turn asks its questions there"]
    for statement in every:
        if ANSWER.match(statement.text) or statement.label == "Rule":
            found.append(f":{statement.line}: the socratic section states an answer")
    items = [statement for statement in every if statement.item]
    if not items:
        found.append(": the socratic section holds no prompt (a list item)")
    elif len(items) > MAX_PROMPTS:
        found.append(
            f": {len(items)} prompts; a Socratic turn asks at most {MAX_PROMPTS}"
        )
    for statement in items:
        tags = ASK_TAG.findall(statement.raw)
        if not tags:
            found.append(
                f":{statement.line}: a prompt has no <!-- ask:<category> --> tag"
            )
        elif len(tags) > 1:
            found.append(f":{statement.line}: a prompt carries {len(tags)} tags")
        for category in tags:
            if category not in SOCRATIC_CATEGORIES:
                found.append(f":{statement.line}: unknown Socratic category {category}")
        if not statement.text.rstrip().endswith("?"):
            found.append(
                f":{statement.line}: a prompt is not a question (it must end with ?)"
            )
    return found


def judge_socratic_coverage(doc: object, ctx: Context) -> list:
    every = prompts(doc)
    items = [statement for statement in every or [] if statement.item]
    if not items:
        return []
    categories = [tag for statement in items for tag in ASK_TAG.findall(statement.raw)]
    found: list = []
    if len(items) >= 3 and len(set(categories)) < 2:
        found.append(
            f": {len(items)} prompts in one category; vary Paul's six categories"
        )
    if not set(categories) & DEEP_CATEGORIES:
        found.append(
            ": no deep category (assumptions, evidence, viewpoints, implications)"
        )
    return found


def issue_blocks(part: object) -> tuple:
    """(statements before the first ###, [(title, line, statements)]) of an issues section."""
    lines = part.body.split("\n")
    first = part.line + 1
    heads: list = []
    fence: "str | None" = None
    for offset, line in enumerate(lines):
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
        heading = SUBHEADING.match(line)
        if heading:
            heads.append((offset, (heading["title"] or "").strip()))
    lead = statements(lines[: heads[0][0]] if heads else lines, first)
    blocks = []
    for index, (offset, title) in enumerate(heads):
        end = heads[index + 1][0] if index + 1 < len(heads) else len(lines)
        blocks.append(
            (
                title,
                first + offset,
                statements(lines[offset + 1 : end], first + offset + 1),
            )
        )
    return lead, blocks


def is_hypo(doc: object) -> bool:
    return doc.frontmatter.get("duty") == "practice-questions" or has_any(
        doc, ("facts", "issues")
    )


def judge_hypo_structure(doc: object, ctx: Context) -> list:
    if not is_hypo(doc):
        return []
    found: list = []
    facts = section(doc, "facts")
    if facts is None:
        found.append(": no facts section; an issue-spotter needs a fact pattern")
    else:
        words = len(
            re.findall(r"[^\W_]+(?:['’-][^\W_]+)*", COMMENT.sub(" ", facts.body))
        )
        if words < MIN_FACT_WORDS:
            found.append(
                f":{facts.line}: the fact pattern has {words} words; it needs {MIN_FACT_WORDS}"
            )
    questions = section(doc, "questions")
    calls = (
        [s for s in section_statements(questions) if s.text.rstrip().endswith("?")]
        if questions
        else []
    )
    if not calls:
        found.append(": no call of the question (a line ending in ?)")
    issues = section(doc, "issues")
    if issues is None:
        return found + [": no issues section; an issue-spotter keys its issues"]
    _lead, blocks = issue_blocks(issues)
    if not blocks:
        found.append(f":{issues.line}: no issue block (a ### heading per issue)")
    for title, line, block in blocks:
        triggers = [s for s in block if s.label == "Trigger" and quotes(s.text, 1)]
        if not triggers:
            found.append(
                f":{line}: issue {title!r} has no Trigger: line with a quote from the facts"
            )
        if not any(s.label == "Rule" for s in block):
            found.append(f":{line}: issue {title!r} has no Rule: line")
    return found


def anchored_statements(doc: object) -> list:
    issues = section(doc, "issues")
    if issues is None:
        return []
    lead, blocks = issue_blocks(issues)
    every = lead + [statement for _title, _line, block in blocks for statement in block]
    return [s for s in every if s.label in ("Trigger", "Immaterial")]


def judge_hypo_anchored(doc: object, ctx: Context) -> list:
    if not is_hypo(doc):
        return []
    facts = section(doc, "facts")
    anchored = anchored_statements(doc)
    if facts is None:
        return [": no facts section to anchor its triggers in"] if anchored else []
    pattern = normalize(COMMENT.sub(" ", facts.body))
    found: list = []
    for statement in anchored:
        for span in quotes(statement.text, 1):
            if not contains(pattern, span):
                found.append(
                    f":{statement.line}: {statement.label} quotes {excerpt(span)!r}, which the fact pattern does not contain"
                )
    return found


def judge_hypo_depth(doc: object, ctx: Context) -> list:
    if not is_hypo(doc):
        return []
    issues = section(doc, "issues")
    if issues is None:
        return []
    _lead, blocks = issue_blocks(issues)
    found: list = []
    if len(blocks) < MIN_ISSUES:
        found.append(
            f": {len(blocks)} issue block; an issue-spotter raises at least {MIN_ISSUES}"
        )
    if not any(
        s.label == "Immaterial" and quotes(s.text, 1) for s in anchored_statements(doc)
    ):
        found.append(": no immaterial fact (an Immaterial: line quoting the facts)")
    return found


def is_grading(doc: object) -> bool:
    return (
        doc.frontmatter.get("duty") == "writing-tutor"
        or section(doc, "rubric") is not None
    )


def judge_grading_cites(doc: object, ctx: Context) -> list:
    if not is_grading(doc):
        return []
    rubric = section(doc, "rubric")
    if rubric is None:
        return [": no rubric section; a grade scores the NCBE criteria there"]
    found: list = []
    scored: dict = {}
    totals: list = []
    for statement in section_statements(rubric):
        total = TOTAL.match(statement.text)
        if statement.label == "Total":
            if total:
                totals.append((int(total["got"]), int(total["max"])))
            else:
                found.append(
                    f":{statement.line}: the Total line is not Total: <got>/<max>"
                )
            continue
        criterion = CRITERION.match(statement.text)
        if criterion is None:
            continue
        name, got, most = (
            criterion["name"],
            int(criterion["got"]),
            int(criterion["max"]),
        )
        if name not in CRITERIA:
            found.append(
                f":{statement.line}: unknown criterion {name}; the rubric scores {', '.join(CRITERIA)}"
            )
            continue
        if name in scored:
            found.append(f":{statement.line}: criterion {name} is repeated")
            continue
        scored[name] = (got, most)
        if most < 1 or got > most:
            found.append(f":{statement.line}: criterion {name} scores {got}/{most}")
        if name in CITED_CRITERIA and not citation_keys(statement.text):
            found.append(f":{statement.line}: criterion {name} cites no source")
    for name in NCBE_CRITERIA:
        if name not in scored:
            found.append(f":{rubric.line}: the rubric lacks the {name} criterion")
    if not totals:
        found.append(f":{rubric.line}: no Total: line")
    elif len(totals) > 1:
        found.append(f":{rubric.line}: more than one Total: line")
    else:
        got = sum(value for value, _most in scored.values())
        most = sum(value for _got, value in scored.values())
        if totals[0] != (got, most):
            found.append(
                f":{rubric.line}: Total: {totals[0][0]}/{totals[0][1]} is not the sum {got}/{most}"
            )
    model = section(doc, "model")
    if model is None or not COMMENT.sub("", model.body).strip():
        found.append(": no model answer; a grade shows the answer it grades against")
    elif not body_citations_of(model):
        found.append(f":{model.line}: the model answer cites no source")
    return found


def body_citations_of(part: object) -> list:
    return [
        key
        for statement in section_statements(part)
        for key in citation_keys(statement.text)
    ]


def judge_grading_feedforward(doc: object, ctx: Context) -> list:
    if not is_grading(doc):
        return []
    steps = section(doc, "next-step")
    if steps is None:
        return [": no next-step section to feed forward"]
    items = [statement for statement in section_statements(steps) if statement.item]
    if not items:
        return [f":{steps.line}: next steps list nothing"]
    found: list = []
    for statement in items:
        names = CRITERION_WORD.search(statement.text)
        if PRAISE.search(statement.text) and not names:
            found.append(
                f":{statement.line}: a next step is praise alone, with no criterion"
            )
        elif not names:
            found.append(f":{statement.line}: a next step names no rubric criterion")
    return found


def check_with_corpus(ctx: Context, judge: object) -> Outcome:
    corpora = Corpora(ctx)
    return output_class(ctx, lambda doc, ctx: judge(doc, ctx, corpora))


CHECKS = {
    "law-area-registry": check_law_area_registry,
    "law-template-area": check_law_template_area,
    "law-template-methods": check_law_template_methods,
    "irac-structure": lambda ctx: output_class(ctx, judge_irac_structure),
    "irac-analysis": lambda ctx: output_class(ctx, judge_irac_analysis),
    "rule-cites-corpus": lambda ctx: output_class(ctx, judge_rule_cites_corpus),
    "citations-resolve": lambda ctx: check_with_corpus(ctx, judge_citations_resolve),
    "quotes-grounded": lambda ctx: check_with_corpus(ctx, judge_quotes_grounded),
    "authority-grounded": lambda ctx: output_class(ctx, judge_authority_grounded),
    "socratic-prompts": lambda ctx: output_class(ctx, judge_socratic_prompts),
    "socratic-coverage": lambda ctx: output_class(ctx, judge_socratic_coverage),
    "hypo-structure": lambda ctx: output_class(ctx, judge_hypo_structure),
    "hypo-anchored": lambda ctx: output_class(ctx, judge_hypo_anchored),
    "hypo-depth": lambda ctx: output_class(ctx, judge_hypo_depth),
    "grading-cites": lambda ctx: output_class(ctx, judge_grading_cites),
    "grading-feedforward": lambda ctx: output_class(ctx, judge_grading_feedforward),
}


# --- the command line --------------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="law-professors-probe.py",
        description="Judge law persona templates and law output (SPEC-V2-2214).",
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="a directory or file to judge instead of --root; repeatable",
    )
    parser.add_argument(
        "--areas", help="the area registry (default: the pack's areas.json)"
    )
    parser.add_argument(
        "--corpus",
        help=f"the corpus manifest for every output (default: the nearest {CORPUS_NAME})",
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


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        for name, stage, severity in CLASSES:
            print(f"{name} {stage} {severity}")
        return EXIT_GREEN
    name = args.name
    root = Path(args.root)
    if not root.is_dir():
        print(
            f"law-professors-probe: --root is not a directory: {args.root}",
            file=sys.stderr,
        )
        return EXIT_USAGE
    bases = [Path(subject) for subject in args.subject] or [root]
    for base in bases:
        if not base.exists():
            print(
                f"law-professors-probe: --subject does not exist: {base}",
                file=sys.stderr,
            )
            return EXIT_USAGE
    try:
        core = load_core(
            Path(args.persona_core) if args.persona_core else DEFAULT_PERSONA_CORE
        )
        docs, broken = core.load_population(bases, [LAW_KIND])
        ctx = Context(
            root=root,
            bases=bases,
            core=core,
            docs=docs,
            broken=broken,
            areas_path=Path(args.areas) if args.areas else DEFAULT_AREAS,
            corpus_path=Path(args.corpus) if args.corpus else None,
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

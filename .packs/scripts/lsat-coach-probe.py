#!/usr/bin/env python3
"""lsat-coach-probe.py -- keep an LSAT coach's templates and texts true to the current test.

SPEC-V2-2215 / ADR-V2-2215. The owner's words, 2026-09-27: "Professor per subject + LSAT coach",
ONE LSAT coach covering LR, RC and logic. The pack `skills/packs/lsat-coach/` ships the coach's
persona template, four data files and six worked outputs. This script is the check that keeps a
template and every text an engine generates from it true to the current test:

    format      format-facts, retired-section-taught, timed-set, timed-set-pace (advisory),
                score-claims
    taxonomy    taxonomy-complete, question-taxonomy, stem-cue (advisory),
                every-choice-explained, trap-named (advisory)
    logic       logic-rules-sound, logic-valid
    provenance  template-contract, item-provenance, sources-cited, lsac-marks

It EXTENDS persona contract v1 (SPEC-V2-2212) and never redefines it. Every document is found and
parsed by persona-core's own `load_population` and `parse_document`, loaded from
`persona-core-probe.py` beside this script. This script reads only what persona-core's contract
leaves to an extension: the `x-lsat` frontmatter key, and the item, passage and explanation markers
and `logic` blocks inside persona-core's sections. The facts it judges against are data in the pack
(`lsat-format.json`, `taxonomy.json`, `logic.json`, `corpus.json`), so a change to the test is a
data edit, never a code branch.

It is VENDORABLE: standard library only (Python >= 3.10), with no import from phoenix-v2 but
persona-core's probe, which ships beside it.

usage:
  lsat-coach-probe.py [--root DIR] [--subject PATH]... [--pack-dir DIR] [--corpus FILE]...
                      check CLASS
  lsat-coach-probe.py classes

`--subject` replaces the walk base, as persona-core's does, and may repeat. `--pack-dir` names the
lsat-coach pack (default: beside this script). `--corpus` adds a private corpus in the pack's
corpus schema, such as the PrepTests a deployment can open. Every class prints one line per
finding, `<class>: <finding>`, and ends with `examined N`. Exit 0 is green, 1 a finding, 2 a
usage error, and 3 VOID: nothing was examined, persona-core is absent, or a data file could not be
read, which is never a pass.
"""

import argparse
import importlib.util
import itertools
import json
import math
import re
import sys
from dataclasses import dataclass, field
from fractions import Fraction
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

CONTRACT = "phx.lsat.coach.v1"
SUBJECT = "test-prep/lsat"
FORMAT_SCHEMA = "phx.lsat.format.v1"
TAXONOMY_SCHEMA = "phx.lsat.taxonomy.v1"
LOGIC_SCHEMA = "phx.lsat.logic.v1"
CORPUS_SCHEMA = "phx.lsat.corpus.v1"
PRACTICE = "practice-questions"
TYPED_DUTIES = ("leech-doctor", "drill-coach")
SET_KINDS = ("section", "drill", "untimed")
TIMED = ("section", "drill")
PROVENANCES = ("original", "official")
PASSAGE_KINDS = ("single", "comparative")
OFFICIAL_ITEMS = "official-items"
MIN_REASON_WORDS = 6
MAX_LETTERS = 12
PACK_DIR = Path(__file__).resolve().parents[1] / "skills" / "packs" / "lsat-coach"
PERSONA_CORE = Path(__file__).resolve().parent / "persona-core-probe.py"
PERSONA_CORE_NAMES = (
    "parse_document",
    "load_population",
    "load_contract",
    "ContractError",
    "subject_kind",
)

CLASSES = (
    "format-facts",
    "retired-section-taught",
    "timed-set",
    "timed-set-pace",
    "score-claims",
    "taxonomy-complete",
    "question-taxonomy",
    "stem-cue",
    "every-choice-explained",
    "trap-named",
    "logic-rules-sound",
    "logic-valid",
    "template-contract",
    "item-provenance",
    "sources-cited",
    "lsac-marks",
)
DATA_CLASSES = ("format-facts", "taxonomy-complete", "logic-rules-sound")

FENCE_OPEN = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})[ \t]*(?P<info>[^`\s]*)[^`]*$")
HTML_COMMENT = re.compile(r"<!--.*?-->")
H3 = re.compile(r"^ {0,3}###(?:[ \t]+(?P<content>.*?))?[ \t]*$")
BLOCK_MARKER = re.compile(
    r"<!--\s*lsat-(?P<what>item|passage)\s+(?P<json>\{.*\})\s*-->\s*$"
)
CHOICE = re.compile(r"^\((?P<letter>[A-Z])\)[ \t]+(?P<text>\S.*)$")
ENTRY = re.compile(
    r"^\((?P<letter>[A-Z])\)[ \t]+(?P<verdict>credited|incorrect)"
    r"(?:,[ \t]*(?P<trap>[a-z0-9]+(?:-[a-z0-9]+)*))?[ \t]*:[ \t]*(?P<reason>.*)$"
)
CITATION = re.compile(
    r"\[@(?P<id>[A-Za-z0-9][A-Za-z0-9._:/-]*)(?:,[ \t]*(?P<locator>[^\]\n]+?))?[ \t]*\]"
)
CITATION_START = re.compile(r"\[@")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
PARAGRAPH_BREAK = re.compile(
    r"^(?:#{1,6}[ \t]|[-*+][ \t]|[0-9]+[.)][ \t]|\([A-Z]\)[ \t])"
)
SENTENCE_SPLIT = re.compile(r"(?<=[.!?;])\s+")
WORD = re.compile(r"[A-Za-z0-9’']+")
NEGATION = re.compile(
    r"\b(?:no|not|never|without|nothing|none|nor|neither|cannot|can['’]t|isn['’]t|aren['’]t|"
    r"doesn['’]t|don['’]t|won['’]t)\b",
    re.IGNORECASE,
)
RETIRED_MARKER = re.compile(
    r"\b(?:no longer|removed|retired|replaced|former|formerly|discontinued|dropped|eliminated|"
    r"not part of|no part of|is not on|isn['’]t on|not on the|sunset\w*|used to be)\b",
    re.IGNORECASE,
)
PENALTY = re.compile(
    r"\b(?:penal(?:t|i[sz])\w*|deduct\w*|subtract\w*|points?\s+off|"
    r"lose\s+(?:a\s+)?(?:point|fraction|quarter))",
    re.IGNORECASE,
)
WRONGISH = re.compile(
    r"\b(?:wrong|incorrect|guess\w*|miss\w*|blank\w*|error\w*)", re.IGNORECASE
)
ANSWER_EVERY = re.compile(r"\banswer\s+(?:every|each|all)\b", re.IGNORECASE)
SCORE_WORD = re.compile(
    r"\b(?:scor\w*|scaled|predict\w*|aim\w*|target\w*|band)\b", re.IGNORECASE
)
SCALE_NUMBER = re.compile(r"(?<![0-9])(1[2-7][0-9]|180)(?![0-9])")
PERCENTILE = re.compile(
    r"\b[0-9]{1,3}(?:st|nd|rd|th)?\s+percentile\b|\bpercentile(?:\s+rank)?\s+(?:of\s+)?[0-9]",
    re.IGNORECASE,
)
REVEAL = re.compile(
    r"\b(?:credited\s+(?:response|answer|choice)|correct\s+(?:answer|response|choice)|"
    r"the\s+answer\s+is|answer\s*:)",
    re.IGNORECASE,
)
MARKS = re.compile(
    r"\b(?:LSAC|LSAT|LawHub|PrepTest|SuperPrep|ItemWise|Law School Admission Council)\b"
)
SELF_CLAIM = re.compile(
    r"\b(?:I am|I['’]m|this (?:coach|product|app|course|persona|program|service) is|"
    r"we are|we['’]re|your coach is)\b(?P<rest>.*)",
    re.IGNORECASE,
)
CLAIM_WORD = re.compile(
    r"\b(?P<word>official|endorsed|approved|certified|sponsored|affiliated|authori[sz]ed|"
    r"accredited|partnered)\b",
    re.IGNORECASE,
)
MARKED_CLAIM = re.compile(
    r"\b(?:LSAC|LSAT)[- ](?P<word>certified|approved|endorsed|accredited|authori[sz]ed|official)\b",
    re.IGNORECASE,
)
SELF = re.compile(r"\b(?:I|I['’]m|me|my|we|our|this coach|your coach)\b", re.IGNORECASE)
INTRODUCTION = re.compile(
    r"\b(?:my name is|call me|I['’]m|I am)\s+(?!(?:an?|the|your|not|here|going|sure)\b)"
    r"(?P<name>(?:[A-Z][\w-]*\s*){1,3})"
)
NOTICE_WORDS = ("LSAT®", "LSAC", "not affiliated", "does not endorse")
LOGIC_LET = re.compile(r"^let\s+(?P<letter>[A-Z])\s*=\s*(?P<text>\S.*)$")
LOGIC_TRANSLATE = re.compile(
    r'^translate:\s*"(?P<form>[^"]+)"\s*=>\s*(?P<formula>\S.*)$'
)
LOGIC_VALID = re.compile(r"^valid:\s*(?P<premises>.*?)\s*\|=\s*(?P<conclusion>\S.*)$")
LOGIC_INVALID = re.compile(
    r"^invalid:\s*(?P<premises>.*?)\s*\|/=\s*(?P<conclusion>\S.*)$"
)
LOGIC_EQUIVALENT = re.compile(r"^equivalent:\s*(?P<left>\S.*?)\s*==\s*(?P<right>\S.*)$")
LOGIC_INEQUIVALENT = re.compile(
    r"^inequivalent:\s*(?P<left>\S.*?)\s*=/=\s*(?P<right>\S.*)$"
)
PLACEHOLDER = re.compile(r"\{(?P<name>[XYZ])\}")
LOCATOR_SHAPES = {
    "question": re.compile(r"^question [0-9]+$", re.IGNORECASE),
    "section-question": re.compile(r"^section [0-9]+, question [0-9]+$", re.IGNORECASE),
}
# The rules the template's own text must keep, each a phrase the method states. They guard the
# template against an edit that drops a rule the pack's rows then judge outputs by.
TEMPLATE_RULES = (
    (
        "every-choice",
        "explain every answer choice",
        re.compile(r"\b(?:every|each) answer choice\b", re.I),
    ),
    (
        "attempt-first",
        "the learner answers before the review",
        re.compile(r"\bbefore the review\b", re.I),
    ),
    (
        "explain-reasoning",
        "ask the learner to explain their reasoning",
        re.compile(r"\bexplain (?:their|your|its) reasoning\b", re.I),
    ),
    (
        "text-only",
        "answer on the basis of the information given",
        re.compile(r"\bon the basis of the information given\b", re.I),
    ),
    (
        "read-all-choices",
        "read all five choices before choosing",
        re.compile(r"\bread all five choices\b", re.I),
    ),
    (
        "time-multiplier",
        "the time multiplier comes from configuration",
        re.compile(r"\btime multiplier\b.*\bconfiguration\b", re.I),
    ),
    ("plain-words", "plain words first", re.compile(r"\bplain words\b", re.I)),
    (
        "causal-apart",
        "causal reasoning kept apart from conditional reasoning",
        re.compile(r"\bcausal\b.*\bconditional\b", re.I),
    ),
    (
        "no-scores",
        "never state a scaled score",
        re.compile(r"\bnever state a scaled score\b", re.I),
    ),
)
MULTIPLIER_VALUE = re.compile(r"\btime[_ ]multiplier\s*[:=]\s*[0-9]", re.IGNORECASE)


class Void(Exception):
    """An input this class needs could not be read: the class is VOID, never green."""


# --- the formula language and its truth table ----------------------------------------------------


TOKEN = re.compile(r"\s*(?:(?P<op><->|->|[!~&|()])|(?P<var>[A-Z])(?![A-Za-z0-9]))")


def parse_formula(text: str) -> tuple:
    """Parse `text` into a tuple tree. Raises ValueError with the reason."""
    tokens: list = []
    position = 0
    stripped = text.rstrip()
    while position < len(stripped):
        match = TOKEN.match(stripped, position)
        if match is None or match.end() == position:
            raise ValueError(f"cannot read {stripped[position:]!r}")
        tokens.append(("op", match["op"]) if match["op"] else ("var", match["var"]))
        position = match.end()
    if not tokens:
        raise ValueError("an empty formula")
    tree, rest = _iff(tokens)
    if rest:
        raise ValueError(f"unexpected {rest[0][1]!r}")
    return tree


def _iff(tokens: list) -> tuple:
    left, tokens = _imp(tokens)
    while tokens and tokens[0] == ("op", "<->"):
        right, tokens = _imp(tokens[1:])
        left = ("iff", left, right)
    return left, tokens


def _imp(tokens: list) -> tuple:
    left, tokens = _or(tokens)
    if tokens and tokens[0] == ("op", "->"):
        right, tokens = _imp(tokens[1:])
        return ("imp", left, right), tokens
    return left, tokens


def _or(tokens: list) -> tuple:
    left, tokens = _and(tokens)
    while tokens and tokens[0] == ("op", "|"):
        right, tokens = _and(tokens[1:])
        left = ("or", left, right)
    return left, tokens


def _and(tokens: list) -> tuple:
    left, tokens = _not(tokens)
    while tokens and tokens[0] == ("op", "&"):
        right, tokens = _not(tokens[1:])
        left = ("and", left, right)
    return left, tokens


def _not(tokens: list) -> tuple:
    if not tokens:
        raise ValueError("a formula ends where an operand is needed")
    kind, value = tokens[0]
    if kind == "op" and value in ("!", "~"):
        inner, rest = _not(tokens[1:])
        return ("not", inner), rest
    if kind == "var":
        return ("var", value), tokens[1:]
    if value == "(":
        inner, rest = _iff(tokens[1:])
        if not rest or rest[0] != ("op", ")"):
            raise ValueError("a ( is never closed")
        return inner, rest[1:]
    raise ValueError(f"unexpected {value!r}")


def letters_of(tree: tuple) -> set:
    if tree[0] == "var":
        return {tree[1]}
    return set().union(*(letters_of(part) for part in tree[1:]))


def evaluate(tree: tuple, world: dict) -> bool:
    kind = tree[0]
    if kind == "var":
        return world[tree[1]]
    if kind == "not":
        return not evaluate(tree[1], world)
    left, right = evaluate(tree[1], world), evaluate(tree[2], world)
    if kind == "and":
        return left and right
    if kind == "or":
        return left or right
    if kind == "imp":
        return (not left) or right
    return left == right


def entails(premises: list, conclusion: tuple) -> bool:
    """Whether every world that makes each premise true makes the conclusion true."""
    names = sorted(
        set().union(letters_of(conclusion), *(letters_of(p) for p in premises))
    )
    if len(names) > MAX_LETTERS:
        raise ValueError(
            f"{len(names)} letters, more than the {MAX_LETTERS} a claim may use"
        )
    for values in itertools.product((False, True), repeat=len(names)):
        world = dict(zip(names, values))
        if all(evaluate(p, world) for p in premises) and not evaluate(
            conclusion, world
        ):
            return False
    return True


def equivalent(left: tuple, right: tuple) -> bool:
    return entails([left], right) and entails([right], left)


def render(tree: tuple) -> str:
    kind = tree[0]
    if kind == "var":
        return tree[1]
    if kind == "not":
        inner = render(tree[1])
        return f"~{inner}" if tree[1][0] in ("var", "not") else f"~({inner})"
    symbol = {"and": "&", "or": "|", "imp": "->", "iff": "<->"}[kind]
    parts = [
        render(part) if part[0] in ("var", "not") else f"({render(part)})"
        for part in tree[1:]
    ]
    return f"{parts[0]} {symbol} {parts[1]}"


def instantiate(template: str, values: dict) -> tuple:
    """A data formula with its {X}, {Y}, {Z} placeholders replaced, parsed."""
    text = PLACEHOLDER.sub(lambda m: f"({values[m['name']]})", template)
    return parse_formula(text)


# --- the pack's data -----------------------------------------------------------------------------


def _load_json(path: Path, schema: str) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise Void(f"{path.name} cannot be read: {error}") from error
    if not isinstance(data, dict) or data.get("schema") != schema:
        raise Void(f"{path.name} is not {schema}")
    return data


@dataclass
class Pack:
    format: dict
    taxonomy: dict
    logic: dict
    corpus: dict
    corpus_files: list

    def source(self, source_id: object) -> "dict | None":
        return self.corpus.get(source_id) if isinstance(source_id, str) else None

    def current_sections(self) -> list:
        value = self.format.get("current_sections")
        return (
            [item for item in value if isinstance(item, str)]
            if isinstance(value, list)
            else []
        )

    def retired(self) -> list:
        value = self.format.get("retired_sections")
        return (
            [row for row in value if isinstance(row, dict)]
            if isinstance(value, list)
            else []
        )

    def types(self) -> dict:
        rows = self.taxonomy.get("types")
        rows = rows if isinstance(rows, list) else []
        return {
            row["id"]: row
            for row in rows
            if isinstance(row, dict) and isinstance(row.get("id"), str)
        }

    def letters(self) -> list:
        value = self.format.get("multiple_choice", {}).get("choice_letters")
        return value if isinstance(value, list) and value else list("ABCDE")


def load_pack(pack_dir: Path, corpus_files: list) -> Pack:
    corpus_data = [_load_json(pack_dir / "corpus.json", CORPUS_SCHEMA)]
    corpus_data += [_load_json(Path(path), CORPUS_SCHEMA) for path in corpus_files]
    corpus: dict = {}
    for data in corpus_data:
        sources = data.get("sources")
        if not isinstance(sources, list):
            raise Void("a corpus holds no sources list")
        for row in sources:
            if not isinstance(row, dict) or not isinstance(row.get("id"), str):
                raise Void("a corpus source has no id")
            if row["id"] in corpus:
                raise Void(f"corpus source {row['id']} is declared twice")
            corpus[row["id"]] = row
    return Pack(
        format=_load_json(pack_dir / "lsat-format.json", FORMAT_SCHEMA),
        taxonomy=_load_json(pack_dir / "taxonomy.json", TAXONOMY_SCHEMA),
        logic=_load_json(pack_dir / "logic.json", LOGIC_SCHEMA),
        corpus=corpus,
        corpus_files=corpus_files,
    )


def load_persona_core():
    """persona-core's probe, loaded by importlib and registered, as its docstring asks."""
    if not PERSONA_CORE.is_file():
        raise Void(
            f"persona-core probe unavailable: {PERSONA_CORE.name} is not beside this script"
        )
    spec = importlib.util.spec_from_file_location("persona_core_probe", PERSONA_CORE)
    if spec is None or spec.loader is None:
        raise Void("persona-core probe unavailable: it cannot be loaded")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as error:  # noqa: BLE001 -- any failure to load is VOID, with its reason
        raise Void(f"persona-core probe unavailable: {error}") from error
    missing = [name for name in PERSONA_CORE_NAMES if not hasattr(module, name)]
    if missing:
        raise Void(
            f"persona-core probe unavailable: it exports no {', '.join(missing)}"
        )
    return module


# --- the extension's grammar inside persona-core's sections -------------------------------------


@dataclass
class Block:
    what: str
    meta: dict
    problem: "str | None"
    title: str
    line: int
    lines: list = field(default_factory=list)


@dataclass
class Item:
    block: Block
    choices: list
    paragraphs: list
    trailing: list

    @property
    def id(self) -> object:
        return self.block.meta.get("id")

    @property
    def stem(self) -> str:
        return self.paragraphs[-1][1] if self.paragraphs else ""


@dataclass
class Entry:
    letter: str
    verdict: str
    trap: "str | None"
    reason: str
    line: int


@dataclass
class Lsat:
    """One claimed document of subject test-prep/lsat, with the extension's parts."""

    doc: object
    where: str
    x_lsat: "dict | None"
    items: list
    passages: list
    explanations: list
    problems: list

    @property
    def kind(self) -> str:
        return self.doc.kind

    @property
    def duty(self) -> object:
        return self.doc.frontmatter.get("duty")

    def section(self, section_id: str):
        return next((s for s in self.doc.sections if s.id == section_id), None)

    def contract_ok(self) -> bool:
        return isinstance(self.x_lsat, dict) and self.x_lsat.get("contract") == CONTRACT


def _outside_fences(body: str, first_line: int):
    """(line number, line) for every line of `body` outside fenced code."""
    fence = None
    for offset, line in enumerate(body.split("\n")):
        number = first_line + offset
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
            continue
        opened = FENCE_OPEN.match(line)
        if opened:
            fence = opened["fence"]
            continue
        yield number, line


def split_blocks(section) -> tuple:
    """The level-3 blocks of a section: (lines before the first, blocks)."""
    preamble: list = []
    blocks: list = []
    current = None
    for number, line in _outside_fences(section.body, section.line + 1):
        heading = H3.match(line)
        if heading is not None:
            content = heading["content"] or ""
            marker = BLOCK_MARKER.search(content)
            if marker is None:
                current = Block(
                    "unmarked", {}, "a ### heading with no lsat marker", content, number
                )
            else:
                try:
                    meta = json.loads(marker["json"])
                    problem = (
                        None
                        if isinstance(meta, dict)
                        else "its marker is not a JSON object"
                    )
                except ValueError:
                    meta, problem = {}, "its marker is not valid JSON"
                current = Block(
                    marker["what"],
                    meta if isinstance(meta, dict) else {},
                    problem,
                    content[: marker.start()].strip(),
                    number,
                )
            blocks.append(current)
            continue
        (current.lines if current is not None else preamble).append((number, line))
    return preamble, blocks


def read_item(block: Block) -> Item:
    choices: list = []
    paragraphs: list = []
    trailing: list = []
    paragraph: list = []
    after_blank = False
    for number, raw in block.lines:
        line = HTML_COMMENT.sub("", raw).strip()
        choice = CHOICE.match(line)
        if not line:
            if paragraph:
                paragraphs.append(
                    (paragraph[0][0], " ".join(text for _, text in paragraph))
                )
                paragraph = []
            after_blank = True
            continue
        if choice:
            if paragraph:
                paragraphs.append(
                    (paragraph[0][0], " ".join(text for _, text in paragraph))
                )
                paragraph = []
            choices.append([choice["letter"], choice["text"], number])
            after_blank = False
            continue
        if choices and not after_blank:
            choices[-1][1] += " " + line
            continue
        if choices:
            trailing.append((number, line))
            continue
        paragraph.append((number, line))
    if paragraph:
        paragraphs.append((paragraph[0][0], " ".join(text for _, text in paragraph)))
    return Item(block, choices, paragraphs, trailing)


def read_entries(block: Block) -> tuple:
    entries: list = []
    problems: list = []
    last = None
    for number, raw in block.lines:
        line = HTML_COMMENT.sub("", raw).strip()
        if not line:
            last = None
            continue
        entry = ENTRY.match(line)
        if entry:
            last = Entry(
                entry["letter"],
                entry["verdict"],
                entry["trap"],
                entry["reason"].strip(),
                number,
            )
            entries.append(last)
            continue
        if CHOICE.match(line):
            problems.append(
                f"line {number}: a choice line that is not `(X) credited: ...` or `(X) incorrect, "
                "<trap>: ...`"
            )
            last = None
            continue
        if last is not None:
            last.reason = (last.reason + " " + line).strip()
    return entries, problems


def build(doc, root: Path) -> Lsat:
    try:
        where = str(Path(doc.path).resolve().relative_to(root.resolve()))
    except ValueError:
        where = str(doc.path)
    x_lsat = doc.frontmatter.get("x-lsat")
    lsat = Lsat(
        doc, where, x_lsat if isinstance(x_lsat, dict) else None, [], [], [], []
    )
    if doc.kind != "output" or doc.frontmatter.get("duty") != PRACTICE:
        return lsat
    questions = lsat.section("questions")
    if questions is not None:
        _, blocks = split_blocks(questions)
        for block in blocks:
            if block.what == "unmarked":
                lsat.problems.append(
                    f"line {block.line}: {block.problem} in the questions section"
                )
            elif block.what == "passage":
                lsat.passages.append(block)
            else:
                lsat.items.append(read_item(block))
    explanations = lsat.section("explanations")
    if explanations is not None:
        _, blocks = split_blocks(explanations)
        for block in blocks:
            if block.what != "item":
                lsat.problems.append(
                    f"line {block.line}: an explanation block is not an lsat-item"
                )
                continue
            entries, problems = read_entries(block)
            lsat.explanations.append((block, entries))
            lsat.problems += problems
    return lsat


def prose(body: str, first_line: int) -> list:
    """(line, sentence) for the body's prose: outside fenced code, with HTML comments removed."""
    units: list = []
    paragraph: list = []

    def flush() -> None:
        if paragraph:
            text = " ".join(part for _, part in paragraph)
            for sentence in SENTENCE_SPLIT.split(text):
                if sentence.strip():
                    units.append((paragraph[0][0], sentence.strip()))
            paragraph.clear()

    for number, raw in _outside_fences(body, first_line):
        line = HTML_COMMENT.sub("", raw).strip()
        if not line:
            flush()
            continue
        if PARAGRAPH_BREAK.match(line):
            flush()
        paragraph.append((number, line))
    flush()
    return units


def doc_prose(lsat: Lsat) -> list:
    return prose(lsat.doc.body, lsat.doc.body_line)


def section_prose(lsat: Lsat, section_id: str) -> list:
    section = lsat.section(section_id)
    return prose(section.body, section.line + 1) if section is not None else []


def citations(text: str) -> tuple:
    """([(id, locator)], [malformed fragments]) for the Pandoc citations in `text`."""
    found: list = []
    malformed: list = []
    clean = HTML_COMMENT.sub("", text)
    for start in CITATION_START.finditer(clean):
        match = CITATION.match(clean, start.start())
        if match is None:
            malformed.append(clean[start.start() : start.start() + 40].split("\n")[0])
            continue
        found.append((match["id"], (match["locator"] or "").strip() or None))
    return found, malformed


def logic_blocks(lsat: Lsat) -> list:
    """(opening line, [(line, text)], closed) for every ```logic block in the body."""
    blocks: list = []
    current = None
    fence = None
    for offset, line in enumerate(lsat.doc.body.split("\n")):
        number = lsat.doc.body_line + offset
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
                if current is not None:
                    current[2] = True
                    blocks.append(current)
                    current = None
            elif current is not None:
                current[1].append((number, line))
            continue
        opened = FENCE_OPEN.match(line)
        if opened:
            fence = opened["fence"]
            if opened["info"].lower() == "logic":
                current = [number, [], False]
    if current is not None:
        blocks.append(current)
    return blocks


# --- the classes: data --------------------------------------------------------------------------


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _is_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _pair(value: object) -> bool:
    return (
        isinstance(value, list)
        and len(value) == 2
        and all(_is_int(v) for v in value)
        and 0 <= value[0] <= value[1]
    )


def _sources_in(value: object) -> list:
    found: list = []
    if isinstance(value, dict):
        for key, inner in value.items():
            if key == "source":
                found.append(inner)
            else:
                found += _sources_in(inner)
    elif isinstance(value, list):
        for inner in value:
            found += _sources_in(inner)
    return found


def check_format_facts(pack: Pack) -> tuple:
    data = pack.format
    findings: list = []
    examined = 0
    current = data.get("current_sections")
    sections = data.get("sections") if isinstance(data.get("sections"), dict) else {}
    retired = pack.retired()
    retired_ids = [row.get("id") for row in retired]
    if (
        not isinstance(current, list)
        or not current
        or len(set(map(str, current))) != len(current)
    ):
        findings.append(
            "current_sections is not a non-empty list of unique section ids"
        )
        current = []
    for section_id in current:
        examined += 1
        entry = sections.get(section_id)
        if section_id in retired_ids:
            findings.append(f"retired section {section_id} is listed as current")
        if not isinstance(entry, dict) or not isinstance(entry.get("name"), str):
            findings.append(
                f"current section {section_id} has no entry with a name in sections"
            )
    for row in retired:
        examined += 1
        aliases = row.get("also_called")
        if (
            not isinstance(row.get("name"), str)
            or not isinstance(aliases, list)
            or not aliases
        ):
            findings.append(
                f"retired section {row.get('id')} has no name or no also_called list"
            )
        if row.get("covered_by") not in current:
            findings.append(
                f"retired section {row.get('id')} is covered_by {row.get('covered_by')}, which is "
                "not current"
            )
    choice = (
        data.get("multiple_choice")
        if isinstance(data.get("multiple_choice"), dict)
        else {}
    )
    examined += 1
    scored = choice.get("scored") if isinstance(choice.get("scored"), dict) else {}
    unscored = (
        choice.get("unscored") if isinstance(choice.get("unscored"), dict) else {}
    )
    if not _is_int(choice.get("sections")) or choice["sections"] < 1:
        findings.append("multiple_choice.sections is not a positive integer")
    else:
        count = unscored.get("count") if _is_int(unscored.get("count")) else 0
        total = sum(v for v in scored.values() if _is_int(v)) + count
        if total != choice["sections"]:
            findings.append(
                f"multiple_choice.sections is {choice['sections']}, but its scored and unscored "
                f"sections sum to {total}"
            )
        intermission = (
            choice.get("intermission")
            if isinstance(choice.get("intermission"), dict)
            else {}
        )
        after = intermission.get("after_section")
        if (
            not _is_int(intermission.get("minutes"))
            or not _is_int(after)
            or not 1 <= after < choice["sections"]
        ):
            findings.append(
                "the intermission names no minutes, or falls after no section between two others"
            )
    if not _is_int(choice.get("section_minutes")) or choice["section_minutes"] < 1:
        findings.append("multiple_choice.section_minutes is not a positive integer")
    for section_id, count in scored.items():
        if section_id not in current or not _is_int(count) or count < 1:
            findings.append(
                f"scored section {section_id} is not a current section with a positive count"
            )
    one_of = unscored.get("one_of")
    if (
        not isinstance(one_of, list)
        or not one_of
        or any(item not in current for item in one_of)
    ):
        findings.append("the unscored section is not one of the current sections")
    letters = choice.get("choice_letters")
    if (
        not isinstance(letters, list)
        or len(letters) < 2
        or len(set(letters)) != len(letters)
        or any(not isinstance(v, str) or not re.fullmatch(r"[A-Z]", v) for v in letters)
    ):
        findings.append("choice_letters is not a list of distinct capital letters")
    lr = sections.get("lr") if isinstance(sections.get("lr"), dict) else None
    if lr is not None:
        examined += 1
        if not _is_int(lr.get("questions_about")) or lr["questions_about"] < 1:
            findings.append("sections.lr.questions_about is not a positive integer")
    rc = sections.get("rc") if isinstance(sections.get("rc"), dict) else None
    if rc is not None:
        examined += 1
        shape = [
            rc.get(key)
            for key in ("questions_per_set", "single_passages", "comparative_sets")
        ]
        if not _is_int(rc.get("sets")) or not all(_pair(value) for value in shape):
            findings.append(
                "sections.rc does not give sets and the ranges of its passages"
            )
        elif not shape[1][0] + shape[2][0] <= rc["sets"] <= shape[1][1] + shape[2][1]:
            findings.append(
                f"sections.rc holds {rc['sets']} sets, which its passage ranges cannot make"
            )
    scoring = data.get("scoring") if isinstance(data.get("scoring"), dict) else {}
    examined += 1
    if not isinstance(scoring.get("deduction_for_incorrect"), bool):
        findings.append("scoring.deduction_for_incorrect is not a boolean")
    scale = scoring.get("scale")
    if not (_pair(scale) and scale[0] < scale[1]):
        findings.append("scoring.scale is not an ordered pair of integers")
    writing = data.get("writing") if isinstance(data.get("writing"), dict) else {}
    examined += 1
    if not isinstance(writing.get("scored"), bool):
        findings.append("writing.scored is not a boolean")
    accommodations = (
        data.get("accommodations")
        if isinstance(data.get("accommodations"), dict)
        else {}
    )
    examined += 1
    minimum = accommodations.get("time_multiplier_min")
    if (
        not _is_number(minimum)
        or minimum < 1
        or accommodations.get("minutes_rounding") != "up"
    ):
        findings.append(
            "accommodations names no time_multiplier_min of at least 1, rounded up"
        )
    for source_id in _sources_in(data):
        examined += 1
        if pack.source(source_id) is None:
            findings.append(f"source {source_id} does not resolve in the corpus")
    return examined, findings


def check_taxonomy_complete(pack: Pack) -> tuple:
    data = pack.taxonomy
    findings: list = []
    examined = 0
    current = pack.current_sections()
    retired_ids = {row.get("id") for row in pack.retired()}
    skills = data.get("skills") if isinstance(data.get("skills"), dict) else {}
    skill_ids: dict = {}
    for section_id, rows in skills.items():
        if section_id not in current:
            findings.append(f"skills names section {section_id}, which is not current")
            continue
        ids: list = []
        for row in rows if isinstance(rows, list) else []:
            examined += 1
            if (
                not isinstance(row, dict)
                or not isinstance(row.get("id"), str)
                or not row.get("lsac")
            ):
                findings.append(f"a {section_id} skill has no id or no lsac text")
                continue
            ids.append(row["id"])
        if len(set(ids)) != len(ids):
            findings.append(f"a {section_id} skill id is repeated")
        skill_ids[section_id] = {
            row["id"]: row.get("lsac")
            for row in rows
            if isinstance(row, dict) and "id" in row
        }
    for section_id in current:
        if section_id not in skill_ids:
            findings.append(f"current section {section_id} lists no skills")
    covered: set = set()
    seen: set = set()
    rows = data.get("types") if isinstance(data.get("types"), list) else []
    for row in rows:
        examined += 1
        if not isinstance(row, dict) or not isinstance(row.get("id"), str):
            findings.append("a type has no id")
            continue
        type_id = row["id"]
        if type_id in seen:
            findings.append(f"duplicate type id {type_id}")
        seen.add(type_id)
        section_id = row.get("section")
        if section_id in retired_ids:
            findings.append(
                f"type {type_id} belongs to the retired section {section_id}"
            )
            continue
        if section_id not in current:
            findings.append(
                f"type {type_id} names section {section_id}, which is not current"
            )
            continue
        if row.get("skill") not in skill_ids.get(section_id, {}):
            findings.append(
                f"type {type_id} maps to skill {row.get('skill')}, which is not a {section_id} "
                "skill"
            )
        else:
            covered.add((section_id, row["skill"]))
        cues = row.get("cues")
        if (
            not isinstance(cues, list)
            or not cues
            or any(not isinstance(c, str) or not c for c in cues)
        ):
            findings.append(f"type {type_id} has no cues")
    for section_id, ids in skill_ids.items():
        for skill_id, lsac in ids.items():
            if (section_id, skill_id) not in covered:
                findings.append(f"LSAC skill {skill_id} ({lsac}) is covered by no type")
    types = pack.types()
    formal = (
        data.get("formal_logic") if isinstance(data.get("formal_logic"), dict) else {}
    )
    for type_id in (
        formal.get("types", []) if isinstance(formal.get("types"), list) else []
    ):
        if types.get(type_id, {}).get("section") != "lr":
            findings.append(
                f"formal-logic type {type_id} is not a Logical Reasoning type"
            )
    concepts = {
        row.get("id") for row in pack.logic.get("concepts", []) if isinstance(row, dict)
    }
    for concept in (
        formal.get("concepts", []) if isinstance(formal.get("concepts"), list) else []
    ):
        if concept not in concepts:
            findings.append(
                f"formal-logic concept {concept} is not a logic.json concept"
            )
    trap_ids: list = []
    for trap in data.get("traps", []) if isinstance(data.get("traps"), list) else []:
        examined += 1
        if (
            not isinstance(trap, dict)
            or not isinstance(trap.get("id"), str)
            or not SLUG.match(trap["id"])
            or not trap.get("means")
        ):
            findings.append("a trap has no slug id or no meaning")
            continue
        trap_ids.append(trap["id"])
        if "source" in trap and pack.source(trap["source"]) is None:
            findings.append(
                f"trap {trap['id']} cites {trap['source']}, which does not resolve"
            )
    if len(set(trap_ids)) != len(trap_ids):
        findings.append("a trap id is repeated")
    for section_id, source_id in (data.get("sources") or {}).items():
        if pack.source(source_id) is None:
            findings.append(f"the {section_id} source {source_id} does not resolve")
    return examined, findings


def _translation_formula(row: dict) -> tuple:
    return instantiate(row["formula"], {"X": "P", "Y": "Q", "Z": "R"})


def check_logic_rules_sound(pack: Pack) -> tuple:
    data = pack.logic
    findings: list = []
    examined = 0
    concepts: set = set()
    for row in (
        data.get("concepts", []) if isinstance(data.get("concepts"), list) else []
    ):
        examined += 1
        if (
            not isinstance(row, dict)
            or not isinstance(row.get("id"), str)
            or row["id"] in concepts
        ):
            findings.append(f"concept {row!r} has no unique id")
            continue
        concepts.add(row["id"])
        if pack.source(row.get("source")) is None:
            findings.append(
                f"concept {row['id']} cites {row.get('source')}, which does not resolve"
            )
    formulas: dict = {}
    for row in (
        data.get("translations", [])
        if isinstance(data.get("translations"), list)
        else []
    ):
        examined += 1
        row_id = row.get("id") if isinstance(row, dict) else None
        if not isinstance(row_id, str) or row_id in formulas:
            findings.append(f"translation {row_id!r} has no unique id")
            continue
        form = row.get("form")
        names = PLACEHOLDER.findall(form) if isinstance(form, str) else []
        if sorted(names) != ["X", "Y"]:
            findings.append(
                f"translation {row_id}: its form must hold {{X}} and {{Y}} once each"
            )
            continue
        try:
            tree = _translation_formula(row)
        except (KeyError, TypeError, ValueError) as error:
            findings.append(
                f"translation {row_id}: its formula does not parse ({error})"
            )
            continue
        if not letters_of(tree) <= {"P", "Q"}:
            findings.append(
                f"translation {row_id}: its formula uses a placeholder its form lacks"
            )
            continue
        formulas[row_id] = tree
        if row.get("concept") not in concepts:
            findings.append(
                f"translation {row_id} names concept {row.get('concept')}, which is not declared"
            )
        if pack.source(row.get("source")) is None:
            findings.append(
                f"translation {row_id} cites {row.get('source')}, which does not resolve"
            )
    for group in (
        data.get("equivalence_groups", [])
        if isinstance(data.get("equivalence_groups"), list)
        else []
    ):
        examined += 1
        members = group.get("members") if isinstance(group, dict) else None
        if (
            not isinstance(members, list)
            or len(members) < 2
            or any(m not in formulas for m in members)
        ):
            findings.append(
                f"equivalence group {group.get('id') if isinstance(group, dict) else group!r} "
                "names a translation that does not parse or does not exist"
            )
            continue
        first = members[0]
        for member in members[1:]:
            if not equivalent(formulas[first], formulas[member]):
                findings.append(
                    f"equivalence group {group.get('id')}: {member} ({render(formulas[member])}) "
                    "is "
                    f"not equivalent to {first} ({render(formulas[first])})"
                )
    for pair in (
        data.get("converse_pairs", [])
        if isinstance(data.get("converse_pairs"), list)
        else []
    ):
        examined += 1
        members = pair.get("members") if isinstance(pair, dict) else None
        if (
            not isinstance(members, list)
            or len(members) != 2
            or any(m not in formulas for m in members)
        ):
            findings.append(
                f"converse pair {members!r} names a translation that does not exist"
            )
            continue
        left, right = (formulas[m] for m in members)
        if (
            left[0] != "imp"
            or right[0] != "imp"
            or not equivalent(left, ("imp", right[2], right[1]))
        ):
            findings.append(
                f"converse pair {members[0]} ({render(left)}) and {members[1]} ({render(right)}) "
                "are not converses"
            )
    seen: set = set()
    for row in (
        data.get("inferences", []) if isinstance(data.get("inferences"), list) else []
    ):
        examined += 1
        row_id = row.get("id") if isinstance(row, dict) else None
        if not isinstance(row_id, str) or row_id in seen:
            findings.append(f"inference {row_id!r} has no unique id")
            continue
        seen.add(row_id)
        values = {"X": "P", "Y": "Q", "Z": "R"}
        try:
            premises = [instantiate(p, values) for p in row.get("premises", [])]
            conclusion = instantiate(row["conclusion"], values)
        except (KeyError, TypeError, ValueError) as error:
            findings.append(f"inference {row_id}: a formula does not parse ({error})")
            continue
        follows = entails(premises, conclusion)
        if not isinstance(row.get("valid"), bool):
            findings.append(f"inference {row_id} has no valid flag")
        elif row["valid"] and not follows:
            findings.append(
                f"inference {row_id} is flagged valid, but it does not follow"
            )
        elif not row["valid"] and follows:
            findings.append(f"inference {row_id} is flagged invalid, but it follows")
        if "concept" in row and row["concept"] not in concepts:
            findings.append(
                f"inference {row_id} names concept {row['concept']}, which is not declared"
            )
        if pack.source(row.get("source")) is None:
            findings.append(
                f"inference {row_id} cites {row.get('source')}, which does not resolve"
            )
    return examined, findings


# --- the classes: documents ---------------------------------------------------------------------


@dataclass
class Population:
    root: Path
    docs: list
    broken: list

    def templates(self) -> list:
        return [d for d in self.docs if d.kind == "template"]

    def outputs(self) -> list:
        return [d for d in self.docs if d.kind == "output"]

    def practice(self) -> list:
        return [d for d in self.outputs() if d.duty == PRACTICE]

    def broken_of(self, *guesses: str) -> list:
        return [b for b in self.broken if b.guess in guesses or b.guess == "unknown"]


def load_documents(core, bases: list, root: Path) -> Population:
    docs, broken = core.load_population(bases, ["test-prep"])
    lsat = [
        build(doc, root) for doc in docs if doc.frontmatter.get("subject") == SUBJECT
    ]
    own = [b for b in broken if f'"{SUBJECT}"' in "\n".join(b.text.split("\n")[:200])]
    return Population(root, lsat, own)


def _broken_findings(population: Population, *guesses: str) -> tuple:
    items = population.broken_of(*guesses)
    shown = []
    for broken in items:
        try:
            where = str(
                Path(broken.path).resolve().relative_to(population.root.resolve())
            )
        except ValueError:
            where = str(broken.path)
        shown.append(f"{where}: does not parse: {broken.reason}")
    return len(items), shown


def check_retired_section_taught(pack: Pack, population: Population) -> tuple:
    retired = pack.retired()
    aliases = sorted(
        {
            alias
            for row in retired
            for alias in [row.get("name", "")] + list(row.get("also_called", []))
            if alias
        },
        key=len,
        reverse=True,
    )
    alias_re = (
        re.compile(
            r"\b(?:" + "|".join(re.escape(a) for a in aliases) + r")\b", re.IGNORECASE
        )
        if aliases
        else None
    )
    retired_ids = {row.get("id") for row in retired}
    examined, findings = _broken_findings(population, "template", "output")
    for lsat in population.docs:
        examined += 1
        if alias_re is not None:
            for line, sentence in doc_prose(lsat):
                match = alias_re.search(sentence)
                if match and not RETIRED_MARKER.search(sentence):
                    findings.append(
                        f"{lsat.where}:{line}: names {match.group(0)} without marking it retired: "
                        f"{sentence[:120]}"
                    )
        section = lsat.x_lsat.get("section") if lsat.x_lsat else None
        if section in retired_ids:
            findings.append(
                f"{lsat.where}: the set is in section {section}, which is retired"
            )
    return examined, findings


def _timing_findings(lsat: Lsat, minutes: int) -> list:
    findings: list = []
    units = section_prose(lsat, "timing")
    text = " ".join(sentence for _, sentence in units)
    if lsat.section("timing") is None:
        return [f"{lsat.where}: the timing section is missing"]
    if not re.search(rf"(?<![0-9]){minutes}(?:\s*|-)minutes?\b", text, re.IGNORECASE):
        findings.append(
            f"{lsat.where}: the timing section never states the limit, {minutes} minutes"
        )
    negated = any(PENALTY.search(s) and NEGATION.search(s) for _, s in units)
    if not ANSWER_EVERY.search(text) and not negated:
        findings.append(
            f"{lsat.where}: the timing section never states the rule that follows from no "
            "deduction for incorrect answers: answer every question"
        )
    return findings


def check_timed_set(pack: Pack, population: Population) -> tuple:
    examined, findings = _broken_findings(population, "output")
    current = pack.current_sections()
    choice = pack.format.get("multiple_choice", {})
    section_minutes = choice.get("section_minutes", 35)
    rc = pack.format.get("sections", {}).get("rc", {})
    for lsat in population.practice():
        examined += 1
        where = lsat.where
        x = lsat.x_lsat
        if not lsat.contract_ok():
            findings.append(f"{where}: x-lsat is missing or does not name {CONTRACT}")
            continue
        kind = x.get("set_kind")
        section = x.get("section")
        if kind not in SET_KINDS:
            findings.append(
                f"{where}: set_kind {kind!r} is not one of {', '.join(SET_KINDS)}"
            )
            continue
        if section not in current:
            findings.append(
                f"{where}: section {section!r} is not a current section ({', '.join(current)})"
            )
            continue
        count = len(lsat.items)
        if not _is_int(x.get("items")) or x["items"] != count:
            findings.append(
                f"{where}: x-lsat.items is {x.get('items')!r}, but the questions hold {count} items"
            )
        if section != "rc" and lsat.passages:
            findings.append(f"{where}: a {section} set declares passages")
        if kind == "untimed":
            if "minutes" in x or "time_multiplier" in x:
                findings.append(
                    f"{where}: an untimed set declares minutes or a time_multiplier"
                )
        else:
            minutes = x.get("minutes")
            multiplier = x.get("time_multiplier")
            minimum = pack.format.get("accommodations", {}).get(
                "time_multiplier_min", 1
            )
            if not _is_int(minutes) or minutes < 1:
                findings.append(f"{where}: a timed set declares no whole minutes")
                continue
            if not _is_number(multiplier) or multiplier < minimum:
                findings.append(
                    f"{where}: time_multiplier {multiplier!r} is not a number of at least {minimum}"
                )
                continue
            if kind == "section":
                expected = math.ceil(
                    Fraction(section_minutes) * Fraction(str(multiplier))
                )
                if minutes != expected:
                    findings.append(
                        f"{where}: a full section runs {section_minutes} x {multiplier} = "
                        f"{expected} minutes, rounded up; this one declares {minutes}"
                    )
                if section == "rc":
                    findings += _rc_shape(lsat, rc)
            findings += _timing_findings(lsat, minutes)
        for line, sentence in doc_prose(lsat):
            if (
                PENALTY.search(sentence)
                and WRONGISH.search(sentence)
                and not NEGATION.search(sentence)
            ):
                findings.append(
                    f"{where}:{line}: claims a penalty for wrong answers: {sentence[:120]}"
                )
    return examined, findings


def _rc_shape(lsat: Lsat, rc: dict) -> list:
    findings: list = []
    where = lsat.where
    sets = rc.get("sets", 4)
    low, high = rc.get("questions_per_set", [5, 8])
    if len(lsat.passages) != sets:
        findings.append(
            f"{where}: a full RC section holds {sets} passage sets; this one holds "
            f"{len(lsat.passages)}"
        )
    kinds = [block.meta.get("kind") for block in lsat.passages]
    for block in lsat.passages:
        held = sum(
            1
            for item in lsat.items
            if item.block.meta.get("passage") == block.meta.get("id")
        )
        if not low <= held <= high:
            findings.append(
                f"{where}: passage {block.meta.get('id')} holds {held} questions; a set holds "
                f"{low} to {high}"
            )
    singles, comparatives = kinds.count("single"), kinds.count("comparative")
    single_range = rc.get("single_passages", [3, 4])
    comparative_range = rc.get("comparative_sets", [0, 1])
    if not single_range[0] <= singles <= single_range[1]:
        findings.append(
            f"{where}: {singles} single passages; a full RC section holds {single_range[0]} to "
            f"{single_range[1]}"
        )
    if not comparative_range[0] <= comparatives <= comparative_range[1]:
        findings.append(
            f"{where}: {comparatives} comparative sets; a full RC section holds "
            f"{comparative_range[0]} to {comparative_range[1]}"
        )
    return findings


def check_timed_set_pace(pack: Pack, population: Population) -> tuple:
    examined, findings = _broken_findings(population, "output")
    sections = pack.format.get("sections", {})
    section_minutes = pack.format.get("multiple_choice", {}).get("section_minutes", 35)
    for lsat in population.practice():
        examined += 1
        x = lsat.x_lsat
        if not lsat.contract_ok() or x.get("set_kind") not in TIMED:
            continue
        minutes, multiplier = x.get("minutes"), x.get("time_multiplier")
        if not _is_int(minutes) or not _is_number(multiplier):
            continue
        section = x.get("section")
        if x["set_kind"] == "drill" and section == "lr":
            about = sections.get("lr", {}).get("questions_about", 25)
            expected = math.ceil(
                Fraction(section_minutes)
                * len(lsat.items)
                / about
                * Fraction(str(multiplier))
            )
            if abs(minutes - expected) > 1:
                findings.append(
                    f"{lsat.where}: a {len(lsat.items)}-question LR drill runs about {expected} "
                    f"minutes at the official pace; this one declares {minutes}"
                )
        elif x["set_kind"] == "drill" and section == "rc":
            sets = sections.get("rc", {}).get("sets", 4)
            expected = math.ceil(
                Fraction(section_minutes)
                * len(lsat.passages)
                / sets
                * Fraction(str(multiplier))
            )
            if abs(minutes - expected) > 1:
                findings.append(
                    f"{lsat.where}: a {len(lsat.passages)}-passage RC drill runs about {expected} "
                    f"minutes at the official pace; this one declares {minutes}"
                )
        elif x["set_kind"] == "section" and section == "lr":
            about = sections.get("lr", {}).get("questions_about", 25)
            if abs(len(lsat.items) - about) > 1:
                findings.append(
                    f"{lsat.where}: a full LR section holds about {about} questions; this one "
                    f"holds {len(lsat.items)}"
                )
    return examined, findings


def check_score_claims(pack: Pack, population: Population) -> tuple:
    low, high = (
        str(v) for v in pack.format.get("scoring", {}).get("scale", [120, 180])
    )
    examined, findings = _broken_findings(population, "template", "output")
    for lsat in population.docs:
        examined += 1
        for line, sentence in doc_prose(lsat):
            numbers = SCALE_NUMBER.findall(sentence)
            if (
                numbers
                and SCORE_WORD.search(sentence)
                and not (low in numbers and high in numbers)
            ):
                findings.append(
                    f"{lsat.where}:{line}: states a scaled score ({', '.join(numbers)}): "
                    f"{sentence[:120]}"
                )
            if PERCENTILE.search(sentence):
                findings.append(
                    f"{lsat.where}:{line}: states a percentile: {sentence[:120]}"
                )
    return examined, findings


def check_question_taxonomy(pack: Pack, population: Population) -> tuple:
    types = pack.types()
    concepts = {
        row.get("id") for row in pack.logic.get("concepts", []) if isinstance(row, dict)
    }
    examined, findings = _broken_findings(population, "output")
    for lsat in population.outputs():
        examined += 1
        where = lsat.where
        if not lsat.contract_ok():
            findings.append(f"{where}: x-lsat is missing or does not name {CONTRACT}")
            continue
        declared = lsat.x_lsat.get("types")
        if lsat.duty in TYPED_DUTIES and (
            not isinstance(declared, list) or not declared
        ):
            findings.append(
                f"{where}: x-lsat.types is required for {lsat.duty}, and it is missing or empty"
            )
        for type_id in declared if isinstance(declared, list) else []:
            if type_id not in types:
                findings.append(
                    f"{where}: x-lsat.types names {type_id}, which is not in taxonomy.json"
                )
        for concept in (
            lsat.x_lsat.get("concepts", [])
            if isinstance(lsat.x_lsat.get("concepts"), list)
            else []
        ):
            if concept not in concepts:
                findings.append(
                    f"{where}: x-lsat.concepts names {concept}, which is not a logic.json concept"
                )
        section = lsat.x_lsat.get("section")
        for item in lsat.items:
            type_id = item.block.meta.get("type")
            if type_id not in types:
                findings.append(
                    f"{where}:{item.block.line}: item {item.id} has type {type_id}, which is not "
                    "in taxonomy.json"
                )
            elif types[type_id].get("section") != section:
                findings.append(
                    f"{where}:{item.block.line}: item {item.id} has type {type_id}, a "
                    f"{types[type_id].get('section')} type, in a {section} set"
                )
    return examined, findings


def _cue_matches(stem: str, cues: list) -> bool:
    lowered = stem.lower()
    return any(
        re.search(rf"(?<![a-z0-9]){re.escape(cue.lower())}(?![a-z0-9])", lowered)
        for cue in cues
    )


def check_stem_cue(pack: Pack, population: Population) -> tuple:
    types = pack.types()
    examined, findings = _broken_findings(population, "output")
    for lsat in population.practice():
        examined += 1
        for item in lsat.items:
            type_id = item.block.meta.get("type")
            if (
                item.block.meta.get("provenance") != "original"
                or type_id not in types
                or not item.stem
            ):
                continue
            row = types[type_id]
            if _cue_matches(item.stem, row.get("cues", [])):
                continue
            others = [
                t
                for t, r in types.items()
                if t != type_id
                and r.get("section") == row.get("section")
                and _cue_matches(item.stem, r.get("cues", []))
            ]
            if others:
                findings.append(
                    f"{lsat.where}:{item.block.line}: item {item.id}'s stem reads as "
                    f"{', '.join(others)}, not {type_id}"
                )
            else:
                findings.append(
                    f"{lsat.where}:{item.block.line}: item {item.id}'s stem carries no cue of "
                    f"{type_id}"
                )
    return examined, findings


def _normal(reason: str) -> str:
    return " ".join(word.lower() for word in WORD.findall(reason))


def check_every_choice_explained(pack: Pack, population: Population) -> tuple:
    letters = pack.letters()
    count_word = {5: "five"}.get(len(letters), str(len(letters)))
    examined, findings = _broken_findings(population, "output")
    for lsat in population.practice():
        examined += 1
        where = lsat.where
        findings += [
            f"{where}: {problem}"
            for problem in lsat.problems
            if "choice line" in problem
        ]
        questions = lsat.section("questions")
        if questions is not None:
            for number, raw in _outside_fences(questions.body, questions.line + 1):
                line = HTML_COMMENT.sub("", raw).strip()
                if ENTRY.match(line) or REVEAL.search(line):
                    findings.append(
                        f"{where}:{number}: the questions section reveals an answer before the "
                        "review"
                    )
        explained: dict = {}
        for block, entries in lsat.explanations:
            explained.setdefault(block.meta.get("id"), []).append((block, entries))
        item_ids = {item.id for item in lsat.items}
        for block_id in explained:
            if block_id not in item_ids:
                findings.append(f"{where}: explanation {block_id} names no item")
        for item in lsat.items:
            if item.block.meta.get("provenance") != "official":
                got = [choice[0] for choice in item.choices]
                if got != letters:
                    findings.append(
                        f"{where}:{item.block.line}: item {item.id} has {len(got)} choices "
                        f"({''.join(got)}); every question has {count_word}, ({letters[0]}) to "
                        f"({letters[-1]})"
                    )
            blocks = explained.get(item.id, [])
            if len(blocks) != 1:
                findings.append(
                    f"{where}: item {item.id} has {len(blocks)} explanations; it needs exactly one"
                )
                continue
            block, entries = blocks[0]
            given = [entry.letter for entry in entries]
            for letter in letters:
                if letter not in given:
                    findings.append(
                        f"{where}:{block.line}: item {item.id}'s explanation leaves ({letter}) "
                        "unexplained"
                    )
            if [g for g in given if g in letters] != sorted(
                set(g for g in given if g in letters), key=letters.index
            ) or len(given) != len(set(given)):
                findings.append(
                    f"{where}:{block.line}: item {item.id}'s explanation repeats a choice or runs "
                    "out of order"
                )
            for entry in entries:
                if entry.letter not in letters:
                    findings.append(
                        f"{where}:{entry.line}: item {item.id} explains ({entry.letter}), which is "
                        "no choice"
                    )
            credited = [
                entry.letter for entry in entries if entry.verdict == "credited"
            ]
            key = item.block.meta.get("key")
            if len(credited) != 1:
                findings.append(
                    f"{where}:{block.line}: item {item.id} credits {len(credited)} choices; "
                    "exactly one is credited"
                )
            elif key in letters and credited[0] != key:
                findings.append(
                    f"{where}:{block.line}: item {item.id} credits ({credited[0]}), but its key is "
                    f"{key}"
                )
            reasons: dict = {}
            for entry in entries:
                if len(WORD.findall(entry.reason)) < MIN_REASON_WORDS:
                    findings.append(
                        f"{where}:{entry.line}: item {item.id}'s ({entry.letter}) reason is under "
                        f"{MIN_REASON_WORDS} words"
                    )
                normal = _normal(entry.reason)
                if normal in reasons:
                    findings.append(
                        f"{where}:{entry.line}: item {item.id} gives ({reasons[normal]}) and "
                        f"({entry.letter}) the same reason"
                    )
                else:
                    reasons[normal] = entry.letter
    return examined, findings


def check_trap_named(pack: Pack, population: Population) -> tuple:
    traps = {
        row.get("id") for row in pack.taxonomy.get("traps", []) if isinstance(row, dict)
    }
    examined, findings = _broken_findings(population, "output")
    for lsat in population.practice():
        examined += 1
        for block, entries in lsat.explanations:
            for entry in entries:
                if entry.verdict != "incorrect":
                    continue
                if entry.trap is None:
                    findings.append(
                        f"{lsat.where}:{entry.line}: item {block.meta.get('id')}'s "
                        f"({entry.letter}) names no trap"
                    )
                elif entry.trap not in traps:
                    findings.append(
                        f"{lsat.where}:{entry.line}: item {block.meta.get('id')}'s "
                        f"({entry.letter}) names "
                        f"trap {entry.trap}, which is not in taxonomy.json"
                    )
    return examined, findings


def _spaced(pieces: list) -> str:
    """A regex for a translation form: literal words and commas, and a literal per placeholder."""
    parts: list = []
    for piece in pieces:
        if piece in ("{X}", "{Y}"):
            parts.append(rf"(?P<{piece[1]}>(?i:not\s+)?[A-Z])")
            continue
        tokens = re.findall(r"[^\s,]+|,", piece)
        for token in tokens:
            parts.append(r"," if token == "," else "(?i:" + re.escape(token) + ")")
    regex = ""
    for index, part in enumerate(parts):
        if index and part != ",":
            regex += r"\s+" if not regex.endswith(",") else r"\s*"
        elif part == ",":
            regex += r"\s*"
        regex += part
    return regex


def _literal(text: str) -> str:
    text = text.strip()
    return "~" + text[-1] if text.lower().startswith("not") else text


def check_logic_valid(pack: Pack, population: Population) -> tuple:
    rows = [
        row
        for row in pack.logic.get("translations", [])
        if isinstance(row, dict) and isinstance(row.get("form"), str)
    ]
    patterns = [
        (
            row,
            re.compile(
                r"^\s*" + _spaced(re.split(r"(\{[XY]\})", row["form"])) + r"\s*\.?\s*$"
            ),
        )
        for row in rows
    ]
    examined, findings = _broken_findings(population, "template", "output")
    for lsat in population.docs:
        examined += 1
        for opened, lines, closed in logic_blocks(lsat):
            where = f"{lsat.where}:{opened}"
            if not closed:
                findings.append(f"{where}: a logic block is never closed")
            declared: set = set()
            claims: list = []
            for number, raw in lines:
                line = raw.strip()
                if not line or line.startswith("#"):
                    continue
                let = LOGIC_LET.match(line)
                if let:
                    declared.add(let["letter"])
                    continue
                claims.append((number, line))
            for number, line in claims:
                findings += [
                    f"{lsat.where}:{number}: {problem}"
                    for problem in _claim_problems(line, declared, patterns)
                ]
    return examined, findings


def _formula_or_problem(text: str, declared: set) -> tuple:
    try:
        tree = parse_formula(text)
    except ValueError as error:
        return None, f"{text!r} is not a formula ({error})"
    missing = sorted(letters_of(tree) - declared)
    if missing:
        return None, f"letter {', '.join(missing)} is not declared by a let line"
    return tree, None


def _claim_problems(line: str, declared: set, patterns: list) -> list:
    translate = LOGIC_TRANSLATE.match(line)
    if translate:
        stated, problem = _formula_or_problem(translate["formula"], declared)
        if problem:
            return [problem]
        form = " ".join(translate["form"].split())
        matched = [
            (row, match) for row, pattern in patterns if (match := pattern.match(form))
        ]
        if not matched:
            return [f'"{form}" matches no translation in logic.json']
        literal_letters = set()
        for row, match in matched:
            values = {name: _literal(match[name]) for name in ("X", "Y")}
            literal_letters |= {value[-1] for value in values.values()}
            expected = instantiate(row["formula"], values)
            if not equivalent(stated, expected):
                return [f'"{form}" is {render(expected)}, not {render(stated)}']
        missing = sorted(literal_letters - declared)
        return (
            [f"letter {', '.join(missing)} is not declared by a let line"]
            if missing
            else []
        )
    for regex, want in ((LOGIC_INVALID, False), (LOGIC_VALID, True)):
        claim = regex.match(line)
        if claim is None:
            continue
        premises = []
        for text in [p for p in claim["premises"].split(";") if p.strip()]:
            tree, problem = _formula_or_problem(text, declared)
            if problem:
                return [problem]
            premises.append(tree)
        conclusion, problem = _formula_or_problem(claim["conclusion"], declared)
        if problem:
            return [problem]
        follows = entails(premises, conclusion)
        if want and not follows:
            return [f"{line!r} is claimed valid, but it does not follow"]
        if not want and follows:
            return [f"{line!r} is claimed invalid, but it does follow"]
        return []
    for regex, want in ((LOGIC_INEQUIVALENT, False), (LOGIC_EQUIVALENT, True)):
        claim = regex.match(line)
        if claim is None:
            continue
        left, problem = _formula_or_problem(claim["left"], declared)
        right, problem_right = _formula_or_problem(claim["right"], declared)
        if problem or problem_right:
            return [problem or problem_right]
        same = equivalent(left, right)
        if want and not same:
            return [f"{line!r} is claimed equivalent, but it is not"]
        if not want and same:
            return [f"{line!r} is claimed inequivalent, but the two are equivalent"]
        return []
    return [
        f"{line!r} is not a logic line: let, translate, valid, invalid, equivalent or inequivalent"
    ]


def check_template_contract(pack: Pack, population: Population) -> tuple:
    names = [
        row.get("name")
        for row in pack.format.get("sections", {}).values()
        if isinstance(row, dict)
    ]
    retired = pack.retired()
    aliases = [
        a
        for row in retired
        for a in [row.get("name", "")] + list(row.get("also_called", []))
        if a
    ]
    alias_re = (
        re.compile(
            r"\b(?:"
            + "|".join(re.escape(a) for a in sorted(aliases, key=len, reverse=True))
            + r")\b",
            re.I,
        )
        if aliases
        else None
    )
    examined, findings = _broken_findings(population, "template")
    for lsat in population.templates():
        examined += 1
        where = lsat.where
        frontmatter = lsat.doc.frontmatter
        if not lsat.contract_ok():
            findings.append(f"{where}: x-lsat is missing or does not name {CONTRACT}")
        if PRACTICE not in (frontmatter.get("duties") or []):
            findings.append(f"{where}: duties does not list {PRACTICE}")
        extra = (
            frontmatter.get("sections")
            if isinstance(frontmatter.get("sections"), dict)
            else {}
        )
        if "timing" not in (extra.get(PRACTICE) or []):
            findings.append(f"{where}: sections gives {PRACTICE} no timing section")
        units = doc_prose(lsat)
        text = " ".join(sentence for _, sentence in units)
        for name in names:
            if name and name not in text:
                findings.append(
                    f"{where}: the template never names the current section {name} (rule "
                    "current-sections)"
                )
        if alias_re is None or not any(
            alias_re.search(s) and RETIRED_MARKER.search(s) for _, s in units
        ):
            findings.append(
                f"{where}: the template never marks the retired section retired (rule "
                "retired-section)"
            )
        for rule_id, description, pattern in TEMPLATE_RULES:
            if not any(pattern.search(sentence) for _, sentence in units):
                findings.append(
                    f"{where}: the template never states that {description} (rule {rule_id})"
                )
        if MULTIPLIER_VALUE.search(lsat.doc.body):
            findings.append(
                f"{where}: the template carries a time multiplier value; it is private "
                "configuration"
            )
    return examined, findings


def _official_problems(block: Block, pack: Pack, what: str) -> list:
    lines = [(n, HTML_COMMENT.sub("", t).strip()) for n, t in block.lines]
    lines = [(n, t) for n, t in lines if t]
    label = f"official {what} {block.meta.get('id')}"
    if len(lines) != 1 or CHOICE.match(lines[0][1]):
        return [
            f"line {block.line}: {label} must hold one citation and no text; it holds {len(lines)} "
            "lines"
        ]
    found, malformed = citations(lines[0][1])
    whole = CITATION.fullmatch(lines[0][1].rstrip(". "))
    if malformed or len(found) != 1 or whole is None:
        return [f"line {lines[0][0]}: {label} must hold one citation and no text"]
    source_id, locator = found[0]
    source = pack.source(source_id)
    if source is None or source.get("kind") != OFFICIAL_ITEMS:
        return [
            f"line {lines[0][0]}: {label} cites {source_id}, which is not an {OFFICIAL_ITEMS} "
            "source"
        ]
    shape = LOCATOR_SHAPES.get(source.get("locator"))
    if shape is None or locator is None or not shape.match(locator):
        return [
            f"line {lines[0][0]}: {label}'s locator {locator!r} does not name a "
            f"{source.get('locator')}"
        ]
    return []


def check_item_provenance(pack: Pack, population: Population) -> tuple:
    letters = pack.letters()
    examined, findings = _broken_findings(population, "output")
    for lsat in population.practice():
        examined += 1
        where = lsat.where
        findings += [
            f"{where}: {p}"
            for p in lsat.problems
            if "lsat marker" in p or "not an lsat-item" in p
        ]
        passages: dict = {}
        for block in lsat.passages:
            meta = block.meta
            if block.problem:
                findings.append(f"{where}:{block.line}: passage {block.problem}")
                continue
            passage_id = meta.get("id")
            if (
                not isinstance(passage_id, str)
                or not SLUG.match(passage_id)
                or passage_id in passages
            ):
                findings.append(
                    f"{where}:{block.line}: a passage has no unique slug id"
                )
                continue
            passages[passage_id] = meta
            if meta.get("kind") not in PASSAGE_KINDS:
                findings.append(
                    f"{where}:{block.line}: passage {passage_id} has kind {meta.get('kind')!r}"
                )
            provenance = meta.get("provenance")
            if provenance not in PROVENANCES:
                findings.append(
                    f"{where}:{block.line}: passage {passage_id} has provenance {provenance!r}"
                )
            elif provenance == "official":
                findings += [
                    f"{where}:{p}" for p in _official_problems(block, pack, "passage")
                ]
            else:
                text = " ".join(HTML_COMMENT.sub("", t) for _, t in block.lines)
                if not text.strip():
                    findings.append(
                        f"{where}:{block.line}: original passage {passage_id} holds no text"
                    )
                if meta.get("kind") == "comparative":
                    for part in ("Passage A", "Passage B"):
                        if part not in text:
                            findings.append(
                                f"{where}:{block.line}: comparative passage {passage_id} does not "
                                f"name {part}"
                            )
        seen: set = set()
        for item in lsat.items:
            meta = item.block.meta
            if item.block.problem:
                findings.append(f"{where}:{item.block.line}: item {item.block.problem}")
                continue
            item_id = meta.get("id")
            if (
                not isinstance(item_id, str)
                or not SLUG.match(item_id)
                or item_id in seen
            ):
                findings.append(
                    f"{where}:{item.block.line}: an item has no unique slug id"
                )
                continue
            seen.add(item_id)
            if meta.get("key") not in letters:
                findings.append(
                    f"{where}:{item.block.line}: item {item_id} has no key, one of "
                    f"{''.join(letters)}"
                )
            passage_id = meta.get("passage")
            if passage_id is not None and passage_id not in passages:
                findings.append(
                    f"{where}:{item.block.line}: item {item_id} names passage {passage_id}, which "
                    "is not declared"
                )
            provenance = meta.get("provenance")
            if provenance not in PROVENANCES:
                findings.append(
                    f"{where}:{item.block.line}: item {item_id} has provenance {provenance!r}"
                )
                continue
            if (
                passage_id in passages
                and passages[passage_id].get("provenance") == "official"
                and provenance != "official"
            ):
                findings.append(
                    f"{where}:{item.block.line}: item {item_id} is original, but it sits on "
                    f"official passage {passage_id}"
                )
            if provenance == "official":
                findings += [
                    f"{where}:{p}" for p in _official_problems(item.block, pack, "item")
                ]
                continue
            needed = 1 if passage_id is not None else 2
            if len(item.paragraphs) < needed:
                findings.append(
                    f"{where}:{item.block.line}: original item {item_id} needs "
                    f"{'a stem' if needed == 1 else 'a stimulus and a stem'} before its choices"
                )
            if item.trailing:
                findings.append(
                    f"{where}:{item.trailing[0][0]}: original item {item_id} has text after its "
                    "choices"
                )
    return examined, findings


def check_sources_cited(pack: Pack, population: Population) -> tuple:
    examined, findings = _broken_findings(population, "output")
    for lsat in population.outputs():
        examined += 1
        where = lsat.where
        sources = lsat.doc.frontmatter.get("sources")
        if (
            not isinstance(sources, list)
            or not sources
            or any(not isinstance(s, str) for s in sources)
        ):
            findings.append(
                f"{where}: sources is missing or empty; an LSAT text cites what it teaches"
            )
            sources = (
                [s for s in sources if isinstance(s, str)]
                if isinstance(sources, list)
                else []
            )
        if len(set(sources)) != len(sources):
            findings.append(f"{where}: sources repeats an id")
        for source_id in sources:
            if pack.source(source_id) is None:
                findings.append(f"{where}: source {source_id} is not in the corpus")
        found, malformed = citations(lsat.doc.body)
        for fragment in malformed:
            findings.append(
                f"{where}: a citation is not [@id] or [@id, locator]: {fragment}"
            )
        cited = {source_id for source_id, _ in found}
        for source_id in sorted(cited - set(sources)):
            findings.append(f"{where}: cites {source_id}, which sources does not list")
        for source_id in sources:
            if source_id not in cited:
                findings.append(
                    f"{where}: lists {source_id} in sources but never cites it"
                )
        x = lsat.x_lsat or {}
        if lsat.duty == PRACTICE and x.get("set_kind") in TIMED:
            timing = lsat.section("timing")
            kinds = (
                {
                    (pack.source(s) or {}).get("kind")
                    for s, _ in citations(timing.body)[0]
                }
                if timing
                else set()
            )
            for kind in ("format", "scoring"):
                if kind not in kinds:
                    findings.append(
                        f"{where}: the timing section cites no {kind} source"
                    )
    return examined, findings


def check_lsac_marks(pack: Pack, population: Population) -> tuple:
    examined, findings = _broken_findings(population, "template", "output")
    for lsat in population.docs:
        examined += 1
        where = lsat.where
        units = doc_prose(lsat)
        if lsat.kind == "template" and not any(
            all(word in s for word in NOTICE_WORDS) for _, s in units
        ):
            findings.append(
                f"{where}: the template carries no LSAC non-affiliation notice with the ® mark"
            )
        for line, sentence in units:
            negated = NEGATION.search(sentence) is not None
            claim = SELF_CLAIM.search(sentence)
            word = CLAIM_WORD.search(claim["rest"]) if claim else None
            if word and MARKS.search(sentence) and not negated:
                findings.append(
                    f"{where}:{line}: claims to be {word['word']} for LSAC: {sentence[:120]}"
                )
                continue
            marked = MARKED_CLAIM.search(sentence)
            if marked and SELF.search(sentence) and not negated:
                findings.append(
                    f"{where}:{line}: claims to be LSAC-{marked['word']}: {sentence[:120]}"
                )
                continue
            for introduction in INTRODUCTION.finditer(sentence):
                name = introduction["name"].strip()
                if MARKS.search(name):
                    findings.append(
                        f"{where}:{line}: introduces itself as {name}, a name that carries an LSAC "
                        "mark"
                    )
    return examined, findings


DOCUMENT_CHECKS = {
    "retired-section-taught": check_retired_section_taught,
    "timed-set": check_timed_set,
    "timed-set-pace": check_timed_set_pace,
    "score-claims": check_score_claims,
    "question-taxonomy": check_question_taxonomy,
    "stem-cue": check_stem_cue,
    "every-choice-explained": check_every_choice_explained,
    "trap-named": check_trap_named,
    "logic-valid": check_logic_valid,
    "template-contract": check_template_contract,
    "item-provenance": check_item_provenance,
    "sources-cited": check_sources_cited,
    "lsac-marks": check_lsac_marks,
}
DATA_CHECKS = {
    "format-facts": check_format_facts,
    "taxonomy-complete": check_taxonomy_complete,
    "logic-rules-sound": check_logic_rules_sound,
}


# --- the command line -----------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="lsat-coach-probe.py",
        description=(
            "Judge LSAT coach templates and texts against the lsat-coach pack (SPEC-V2-2215)."
        ),
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="a directory or file to judge instead of --root; repeatable",
    )
    parser.add_argument(
        "--pack-dir", help="the lsat-coach pack directory (default: beside this script)"
    )
    parser.add_argument(
        "--corpus",
        action="append",
        default=[],
        help="a private corpus in the phx.lsat.corpus.v1 schema; repeatable",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    commands.add_parser("classes", help="list the classes")
    return parser


def _void(name: str, reason: str) -> int:
    print(f"{name}: VOID: {reason}")
    print("examined 0")
    return EXIT_VOID


def main(argv: "list | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        print("\n".join(CLASSES))
        return EXIT_GREEN
    name = args.name
    root = Path(args.root)
    if not root.is_dir():
        print(
            f"lsat-coach-probe: --root is not a directory: {args.root}", file=sys.stderr
        )
        return EXIT_USAGE
    bases = [Path(subject) for subject in args.subject] or [root]
    for base in bases:
        if not base.exists():
            print(
                f"lsat-coach-probe: --subject does not exist: {base}", file=sys.stderr
            )
            return EXIT_USAGE
    try:
        pack = load_pack(
            Path(args.pack_dir) if args.pack_dir else PACK_DIR, list(args.corpus)
        )
        if name in DATA_CHECKS:
            examined, findings = DATA_CHECKS[name](pack)
        else:
            core = load_persona_core()
            try:
                contract = core.load_contract()
            except core.ContractError as error:
                raise Void(f"persona contract unavailable: {error}") from error
            duty = next(
                (d for d in contract.get("duties", []) if d.get("id") == PRACTICE), {}
            )
            if not {"questions", "explanations"} <= set(duty.get("sections", [])):
                raise Void(
                    f"persona contract v1 no longer gives {PRACTICE} questions and explanations"
                )
            population = load_documents(core, bases, root)
            examined, findings = DOCUMENT_CHECKS[name](pack, population)
    except Void as error:
        return _void(name, str(error))
    for finding in findings:
        print(f"{name}: {finding}")
    if examined == 0:
        return _void(name, "examined nothing")
    print(f"examined {examined}")
    return EXIT_FINDING if findings else EXIT_GREEN


if __name__ == "__main__":
    sys.exit(main())

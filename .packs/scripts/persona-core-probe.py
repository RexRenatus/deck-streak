#!/usr/bin/env python3
"""persona-core-probe.py -- the executable contract for DeckStreak's teaching personas.

SPEC-V2-2212. A persona pack ships TEMPLATES, and DeckStreak's engine writes persona OUTPUT: a
markdown text per duty, with frontmatter naming the persona, the subject, the CEFR band where it
applies and the duty. This script judges both against the persona template schema v1 and the
persona output contract v1, and runs the checks every persona shares:

    template-schema    block     the template schema: keys, slots, required sections
    output-contract    block     the output contract: keys, values, the persona it names
    duty-composition   block     a duty names WHAT (its sections), the persona shapes HOW
    no-dates           block     no calendar date and no timeline, in any of six languages
    scrubber           block     v9's scrub_public() deny-list, public shapes plus a private list
    memory-scope       block     a persona reads only its subject's leeches, drill grades, lapses
    no-human-claim     block     a persona never claims to be human (EU AI Act Art. 50(1))
    voice              advisory  a consistent professional voice: no stage directions, emoji
                                 bursts or invented backstory

It is VENDORABLE on purpose: standard library only (Python >= 3.10), no import from phoenix-v2,
and every check reads any tree through `--root`, or the paths `--subject` names. The vocabularies
(kinds, CEFR modes, memory sources, roster slots, the duty registry) live in the pack's
contract.json, and the date, timeline, human-claim and voice tables live in patterns.json, so a
new language or a new duty is a data row, never a code branch.

A markdown file is CLAIMED when its line 1 is `---` and a frontmatter line names `phx.persona.`.
A claimed file that does not parse is a finding, never a skip. Every class prints one line per
finding, `<class>: <finding>`, and ends with `examined N`. Exit 0 is green, 1 a finding, 2 a usage
error, and 3 VOID: nothing was examined or an input could not be read, which is never a pass.

The annotations are evaluated at run time on purpose (no `from __future__ import annotations`): a
sibling pack loads this file through importlib, and a dataclass with string annotations looks its
module up in sys.modules, which fails when a loader has not registered it (measured, d2212).
"""

import argparse
import json
import os
import re
import sys
import unicodedata
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

TEMPLATE_SCHEMA = "phx.persona.template.v1"
OUTPUT_SCHEMA = "phx.persona.output.v1"
RULES_SCHEMA = "phx.persona.rules.v1"
DOCUMENT_KINDS = {
    TEMPLATE_SCHEMA: "template",
    OUTPUT_SCHEMA: "output",
    RULES_SCHEMA: "rules",
}
CONTRACT_SCHEMA = "phx.persona.contract.v1"
PATTERNS_SCHEMA = "phx.persona.patterns.v1"
DENY_SCHEMA = "phx.persona.deny.v1"
CLAIM_MARK = "phx.persona."
CLAIM_SCAN_LINES = 200
DEFAULT_PACK_DIR = (
    Path(__file__).resolve().parents[1] / "skills" / "packs" / "persona-core"
)
DENY_ENV = "PERSONA_CORE_DENY_LIST"
SKIPPED_DIRECTORIES = frozenset({"node_modules", "target"})

TEMPLATE_KEYS = frozenset(
    {"schema", "template", "subject", "lang", "duties", "memory", "sections"}
)
TEMPLATE_REQUIRED = ("schema", "template", "subject", "duties", "memory")
OUTPUT_KEYS = frozenset(
    {"schema", "persona", "subject", "duty", "cefr", "lang", "memory", "sources"}
)
OUTPUT_REQUIRED = ("schema", "persona", "subject", "duty", "memory")
RULES_KEYS = frozenset({"schema"})
LANGUAGE_KIND = "language"

KEY_LINE = re.compile(
    r"^(?P<key>[a-z][a-z0-9_]*|x-[a-z0-9]+(?:-[a-z0-9]+)*):[ ]+(?P<value>\S.*)$"
)
EXTENSION_KEY = re.compile(r"^x-[a-z0-9]+(?:-[a-z0-9]+)*$")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
SUBJECT = re.compile(
    r"^(?P<kind>[a-z0-9]+(?:-[a-z0-9]+)*)/(?P<area>[a-z0-9]+(?:-[a-z0-9]+)*)$"
)
LANG_TAG = re.compile(r"^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-(?:[A-Z]{2}|[0-9]{3}))?$")
MEMORY_TOKEN = re.compile(
    r"^(?P<source>[a-z0-9]+(?:-[a-z0-9]+)*)@(?P<subject>[a-z0-9]+(?:-[a-z0-9]+)*/"
    r"[a-z0-9]+(?:-[a-z0-9]+)*)$"
)
HEADING = re.compile(r"^ {0,3}(?P<hashes>#{1,6})(?:[ \t]+(?P<content>.*?))?[ \t]*$")
MARKER = re.compile(r"<!--\s*section:\s*(?P<id>[^\s>]*)\s*-->\s*$")
FENCE = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})")
SLOT_TOKEN = re.compile(r"\{\{(?P<inner>[^{}\n]*)\}\}")
RAW_SUBJECT = re.compile(r'^subject:\s*"(?P<kind>[a-z0-9-]+)/')
RAW_KEY = re.compile(r"^(?P<key>[^:\s]+):")
WIKILINK = re.compile(r"!?\[\[(?P<target>[^\]\n]+?)\]\]")
# A web address is never the vault: an article whose path has a `journal` segment is not the
# learner's journal, which only a wikilink, an embed or a relative path reaches.
WEB_URL = re.compile(r"^https?://", re.IGNORECASE)
MARKDOWN_LINK = re.compile(r"!?\[[^\]\n]*\]\((?P<target>[^)\s]+)")
TAG = re.compile(r"(?:^|(?<=\s))#(?P<tag>[^\W\d_][\w/-]*)")
CJK = re.compile(r"[぀-ヿ㐀-䶿一-鿿가-힯豈-﫿]")
HTML_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
LATIN_WORD = re.compile(r"[^\W\d_]+", re.ASCII | re.IGNORECASE)

CLASSES = (
    "template-schema",
    "output-contract",
    "duty-composition",
    "no-dates",
    "scrubber",
    "memory-scope",
    "no-human-claim",
    "voice",
)


class ContractError(ValueError):
    """A persona file, or one of the pack's data files, does not parse. str() is the reason."""


@dataclass(frozen=True)
class Section:
    """One marked section: `## <title> <!-- section:<id> -->` and the lines up to the next heading."""

    id: str
    title: str
    line: int
    body: str


@dataclass(frozen=True)
class Document:
    """A claimed persona file that parsed. Line numbers are 1-based throughout."""

    path: Path
    kind: str
    frontmatter: dict
    sections: tuple
    body: str
    body_line: int
    text: str
    problems: tuple


@dataclass(frozen=True)
class Broken:
    """A claimed persona file that did not parse, kept so that every class still counts it."""

    path: Path
    reason: str
    guess: str
    text: str


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


@dataclass
class Context:
    root: Path
    docs: list
    broken: list
    contract: dict
    patterns: dict
    deny: dict

    def templates(self) -> list:
        return [doc for doc in self.docs if doc.kind == "template"]

    def outputs(self) -> list:
        return [doc for doc in self.docs if doc.kind == "output"]

    def template_for(self, persona: object) -> "Document | None":
        for doc in self.templates():
            if doc.frontmatter.get("template") == persona:
                return doc
        return None

    def show(self, path: Path) -> str:
        try:
            return str(path.resolve().relative_to(self.root.resolve()))
        except ValueError:
            return str(path)


# --- reading: the pack's data and one persona file ---------------------------------------------


def _load_json(path: Path, schema: str) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise ContractError(
            f"{path}: unreadable ({error.strerror or error})"
        ) from error
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ContractError(f"{path}: not JSON ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != schema:
        raise ContractError(f"{path}: schema is not {schema}")
    return data


def load_contract(pack_dir: "Path | None" = None) -> dict:
    """The pack's contract.json: kinds, CEFR modes, memory sources, roster slots, duty registry."""
    contract = _load_json(
        Path(pack_dir or DEFAULT_PACK_DIR) / "contract.json", CONTRACT_SCHEMA
    )
    for key in (
        "kinds",
        "cefr",
        "memory_sources",
        "roster_slots",
        "template_sections",
        "duties",
    ):
        if key not in contract:
            raise ContractError(f"contract.json: {key} is missing")
    return contract


def load_patterns(pack_dir: "Path | None" = None) -> dict:
    """The pack's patterns.json, every regex compiled; a regex that does not compile refuses."""
    patterns = _load_json(
        Path(pack_dir or DEFAULT_PACK_DIR) / "patterns.json", PATTERNS_SCHEMA
    )
    for group in ("dates", "timelines", "human_claims", "voice"):
        rows = patterns.get(group)
        if not isinstance(rows, list) or not rows:
            raise ContractError(f"patterns.json: {group} is missing or empty")
        patterns[group] = [_compiled(row, f"patterns.json {group}") for row in rows]
    if not isinstance(patterns.get("voice_limits"), dict):
        raise ContractError("patterns.json: voice_limits is missing")
    return patterns


def _compiled(row: object, where: str) -> tuple:
    if not isinstance(row, dict) or not isinstance(row.get("regex"), str):
        raise ContractError(f"{where}: a row has no regex")
    flags = re.IGNORECASE if "i" in str(row.get("flags", "")) else 0
    try:
        return (str(row.get("id", "?")), re.compile(row["regex"], flags))
    except re.error as error:
        raise ContractError(f"{where}: {row.get('id')}: {error}") from error


def load_deny(pack_dir: "Path | None" = None, private: "Path | None" = None) -> dict:
    """The public deny-list merged with the private one, if a private one is named.

    Public, stable API (SPEC-V2-2212). Returns a dict of four lists, each entry tagged with its
    origin, `"public"` or `"private"`:

    - `key_markers`: `(origin, marker)`, casefolded;
    - `patterns`: `(origin, id, compiled re.Pattern)`;
    - `literals`: `(origin, literal)`, casefolded;
    - `journal_paths`: `(origin, word)`, casefolded.

    `private=None` reads the public half alone. It does not read the environment: the command
    line resolves `--deny-list`, then `$PERSONA_CORE_DENY_LIST`, and a caller does the same. An
    unreadable or malformed file raises ContractError.
    """
    public = _load_json(
        Path(pack_dir or DEFAULT_PACK_DIR) / "deny-list.json", DENY_SCHEMA
    )
    merged: dict = {
        "key_markers": [],
        "patterns": [],
        "literals": [],
        "journal_paths": [],
    }
    for origin, source in (("public", public), ("private", None)):
        if origin == "private":
            if private is None:
                continue
            source = _load_json(Path(private), DENY_SCHEMA)
        for key in merged:
            values = source.get(key, [])
            if not isinstance(values, list):
                raise ContractError(f"{origin} deny-list: {key} is not a list")
            for value in values:
                if key == "patterns":
                    merged[key].append(
                        (origin, *_compiled(value, f"{origin} deny-list"))
                    )
                elif isinstance(value, str) and value:
                    merged[key].append((origin, value.casefold()))
                else:
                    raise ContractError(f"{origin} deny-list: {key} holds a non-string")
    return merged


def claimed(path: "Path | str") -> bool:
    """Whether `path` claims to be a persona file: line 1 `---` and `phx.persona.` in its frontmatter."""
    try:
        with open(path, "rb") as handle:
            head = handle.read(65536)
    except OSError:
        return False
    text = head.decode("utf-8", errors="replace").removeprefix("﻿")
    lines = text.split("\n")
    if lines[0].rstrip("\r") != "---":
        return False
    for line in lines[1:CLAIM_SCAN_LINES]:
        if line.rstrip("\r") == "---":
            return False
        if CLAIM_MARK in line:
            return True
    return False


def discover(bases: Sequence) -> list:
    """Every claimed `.md` file under `bases`, sorted; dot-directories, node_modules and target skipped."""
    found: dict = {}
    for base in bases:
        base = Path(base)
        if base.is_file():
            if base.suffix == ".md" and claimed(base):
                found[base.resolve()] = base
            continue
        for directory, subdirectories, files in os.walk(base):
            subdirectories[:] = sorted(
                name
                for name in subdirectories
                if not name.startswith(".") and name not in SKIPPED_DIRECTORIES
            )
            for name in sorted(files):
                candidate = Path(directory) / name
                if name.endswith(".md") and claimed(candidate):
                    found[candidate.resolve()] = candidate
    return [found[key] for key in sorted(found)]


def subject_kind(subject: object) -> "str | None":
    """The kind of a `<kind>/<area>` subject, or None when it is not that shape."""
    if not isinstance(subject, str):
        return None
    match = SUBJECT.match(subject)
    return match["kind"] if match else None


def _reject_constant(name: str) -> object:
    raise ValueError(f"{name} is not JSON")


def parse_document(path: "Path | str") -> Document:
    """Parse one persona file. Raises ContractError, with the reason, on a fatal defect."""
    path = Path(path)
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ContractError(f"unreadable ({error.strerror or error})") from error
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ContractError("not UTF-8") from error
    lines = [line.removesuffix("\r") for line in text.split("\n")]
    if lines[0] != "---":
        raise ContractError("line 1 is not ---")
    frontmatter: dict = {}
    close = None
    for index in range(1, len(lines)):
        line = lines[index]
        if line == "---":
            close = index
            break
        if line == "" and index == len(lines) - 1:
            break
        match = KEY_LINE.match(line)
        if match is None:
            raise ContractError(f"line {index + 1}: not `key: <one-line JSON value>`")
        key = match["key"]
        if key in frontmatter:
            raise ContractError(f"line {index + 1}: duplicate key {key}")
        try:
            frontmatter[key] = json.loads(
                match["value"], parse_constant=_reject_constant
            )
        except ValueError as error:
            raise ContractError(
                f"line {index + 1}: {key} is not one JSON value"
            ) from error
    if close is None:
        raise ContractError("the frontmatter is never closed by ---")
    schema = frontmatter.get("schema")
    kind = DOCUMENT_KINDS.get(schema) if isinstance(schema, str) else None
    if kind is None:
        raise ContractError(f"schema {schema!r} is not a persona schema")
    body_lines = lines[close + 1 :]
    sections, problems = _parse_sections(body_lines, close + 2)
    return Document(
        path=path,
        kind=kind,
        frontmatter=frontmatter,
        sections=tuple(sections),
        body="\n".join(body_lines),
        body_line=close + 2,
        text=text,
        problems=tuple(problems),
    )


def _parse_sections(lines: list, first: int) -> tuple:
    sections: list = []
    problems: list = []
    seen: set = set()
    current: "list | None" = None
    fence: "str | None" = None
    for offset, line in enumerate(lines):
        number = first + offset
        if fence is not None:
            if re.match(
                rf"^ {{0,3}}{re.escape(fence[0])}{{{len(fence)},}}[ \t]*$", line
            ):
                fence = None
            if current is not None:
                current[3].append(line)
            continue
        opened = FENCE.match(line)
        if opened:
            fence = opened["fence"]
            if current is not None:
                current[3].append(line)
            continue
        heading = HEADING.match(line)
        if heading is None or len(heading["hashes"]) > 2:
            if current is not None:
                current[3].append(line)
            continue
        if current is not None:
            sections.append(
                Section(current[0], current[1], current[2], "\n".join(current[3]))
            )
            current = None
        content = heading["content"] or ""
        marker = MARKER.search(content)
        if len(heading["hashes"]) == 1:
            if marker is not None:
                problems.append(f"line {number}: a section marker on a level-1 heading")
            continue
        if marker is None:
            problems.append(f"line {number}: a ## heading with no section marker")
            continue
        section_id = marker["id"]
        if not SLUG.match(section_id):
            problems.append(f"line {number}: section id {section_id!r} is not a slug")
            continue
        if section_id in seen:
            problems.append(f"line {number}: section {section_id} is repeated")
            continue
        seen.add(section_id)
        current = [section_id, content[: marker.start()].strip(), number, []]
    if current is not None:
        sections.append(
            Section(current[0], current[1], current[2], "\n".join(current[3]))
        )
    return sections, problems


def _guess(text: str) -> str:
    head = text.removeprefix("﻿").split("\n")[:CLAIM_SCAN_LINES]
    for line in head[1:]:
        if line.rstrip("\r") == "---":
            break
        for schema, kind in DOCUMENT_KINDS.items():
            if schema.rsplit(".", 1)[0] in line:
                return kind
    return "unknown"


def _raw_kind(text: str) -> "str | None":
    for line in text.removeprefix("﻿").split("\n")[1:CLAIM_SCAN_LINES]:
        if line.rstrip("\r") == "---":
            break
        match = RAW_SUBJECT.match(line)
        if match:
            return match["kind"]
    return None


def load_population(bases: Sequence, kinds: Sequence) -> tuple:
    """Parse every claimed file under `bases`; keep those of `kinds` (all when empty)."""
    docs: list = []
    broken: list = []
    wanted = set(kinds)
    for path in discover(bases):
        try:
            doc = parse_document(path)
        except ContractError as error:
            text = path.read_bytes().decode("utf-8", errors="replace")
            raw_kind = _raw_kind(text)
            if wanted and raw_kind not in wanted:
                continue
            broken.append(Broken(path, str(error), _guess(text), text))
            continue
        if wanted and subject_kind(doc.frontmatter.get("subject")) not in wanted:
            continue
        docs.append(doc)
    return docs, broken


# --- the checks ----------------------------------------------------------------------------------


def _section(doc: Document, section_id: str) -> "Section | None":
    return next((section for section in doc.sections if section.id == section_id), None)


def _keys_problems(frontmatter: dict, allowed: frozenset, required: Sequence) -> list:
    problems = []
    for key in frontmatter:
        if key not in allowed and not EXTENSION_KEY.match(key):
            problems.append(f"unknown key {key}")
    for key in required:
        if key not in frontmatter:
            problems.append(f"{key} is missing")
    return problems


def _unique_strings(
    value: object, what: str, pattern: "re.Pattern | None" = None
) -> list:
    if not isinstance(value, list):
        return [f"{what} is not a list"]
    problems = []
    for item in value:
        if not isinstance(item, str) or not item:
            problems.append(f"{what} holds a value that is not a non-empty string")
        elif pattern is not None and not pattern.match(item):
            problems.append(f"{what} holds {item!r}, which is not of the right shape")
    strings = [item for item in value if isinstance(item, str)]
    if len(set(strings)) != len(strings):
        problems.append(f"{what} repeats a value")
    return problems


def _subject_problems(frontmatter: dict, contract: dict) -> list:
    subject = frontmatter.get("subject")
    if "subject" not in frontmatter:
        return []
    if not isinstance(subject, str) or not SUBJECT.match(subject):
        return [f"subject {subject!r} is not <kind>/<area>"]
    if subject_kind(subject) not in contract["kinds"]:
        return [
            f"subject kind {subject_kind(subject)!r} is not one of {', '.join(contract['kinds'])}"
        ]
    return []


def _lang_problems(frontmatter: dict, *, expected: object = None) -> list:
    kind = subject_kind(frontmatter.get("subject"))
    if kind == LANGUAGE_KIND:
        lang = frontmatter.get("lang")
        if "lang" not in frontmatter:
            return ["lang is missing, and a language subject requires it"]
        if not isinstance(lang, str) or not LANG_TAG.match(lang):
            return [f"lang {lang!r} is not a BCP 47 language tag"]
        if expected is not None and lang != expected:
            return [f"lang {lang} is not the template's {expected}"]
        return []
    if "lang" in frontmatter:
        return ["lang is set, but only a language subject carries one"]
    return []


def _slot_problems(doc: Document, contract: dict) -> list:
    problems = []
    slots = set(contract["roster_slots"])
    for match in SLOT_TOKEN.finditer(doc.text):
        inner = match["inner"]
        if inner in slots:
            continue
        if inner.strip() in slots:
            problems.append(
                f"slot {match.group(0)} must be spelled {{{{{inner.strip()}}}}}"
            )
        else:
            problems.append(f"unknown slot {match.group(0)}")
    return problems


def _template_problems(doc: Document, contract: dict) -> list:
    frontmatter = doc.frontmatter
    problems = _keys_problems(frontmatter, TEMPLATE_KEYS, TEMPLATE_REQUIRED)
    template_id = frontmatter.get("template")
    if "template" in frontmatter and (
        not isinstance(template_id, str) or not SLUG.match(template_id)
    ):
        problems.append(f"template {template_id!r} is not a slug")
    problems += _subject_problems(frontmatter, contract)
    problems += _lang_problems(frontmatter)
    if "duties" in frontmatter:
        problems += _unique_strings(frontmatter["duties"], "duties")
        if frontmatter["duties"] == []:
            problems.append("duties is empty")
    if "memory" in frontmatter:
        problems += _unique_strings(frontmatter["memory"], "memory")
    if "sections" in frontmatter:
        extra = frontmatter["sections"]
        if not isinstance(extra, dict):
            problems.append("sections is not an object")
        else:
            for duty, ids in extra.items():
                problems += _unique_strings(ids, f"sections[{duty}]", SLUG)
    problems += list(doc.problems)
    for spec in contract["template_sections"]:
        section = _section(doc, spec["id"])
        if section is None:
            problems.append(f"section {spec['id']} is missing")
            continue
        if not section.body.strip():
            problems.append(f"section {spec['id']} is empty")
        for slot in spec["slots"]:
            if "{{" + slot + "}}" not in section.body:
                problems.append(
                    f"section {spec['id']} does not carry the slot {{{{{slot}}}}}"
                )
    disclosure = _section(doc, "disclosure")
    token = contract.get("disclosure_token", "AI")
    if disclosure is not None and not re.search(
        rf"(?<![A-Za-z]){re.escape(token)}(?![A-Za-z])", disclosure.body
    ):
        problems.append(f"section disclosure never says {token}")
    problems += _slot_problems(doc, contract)
    return problems


def _rules_problems(doc: Document) -> list:
    problems = _keys_problems(doc.frontmatter, RULES_KEYS, ("schema",))
    problems += list(doc.problems)
    if not doc.sections:
        problems.append("the rules hold no section")
    for section in doc.sections:
        if not section.body.strip():
            problems.append(f"section {section.id} is empty")
    return problems


def check_template_schema(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for broken in ctx.broken:
        if broken.guess in ("template", "rules", "unknown"):
            examined += 1
            findings.append(f"{ctx.show(broken.path)}: {broken.reason}")
    first_owner: dict = {}
    for doc in ctx.docs:
        if doc.kind == "rules":
            examined += 1
            findings += [
                f"{ctx.show(doc.path)}: {problem}" for problem in _rules_problems(doc)
            ]
        elif doc.kind == "template":
            examined += 1
            findings += [
                f"{ctx.show(doc.path)}: {problem}"
                for problem in _template_problems(doc, ctx.contract)
            ]
            template_id = doc.frontmatter.get("template")
            if isinstance(template_id, str):
                if template_id in first_owner:
                    findings.append(
                        f"{ctx.show(doc.path)}: template {template_id} is also declared by "
                        f"{ctx.show(first_owner[template_id])}"
                    )
                else:
                    first_owner[template_id] = doc.path
    return Outcome(examined, tuple(findings))


def _output_problems(doc: Document, ctx: Context) -> list:
    frontmatter = doc.frontmatter
    contract = ctx.contract
    problems = _keys_problems(frontmatter, OUTPUT_KEYS, OUTPUT_REQUIRED)
    persona = frontmatter.get("persona")
    template = None
    if "persona" in frontmatter:
        if not isinstance(persona, str) or not SLUG.match(persona):
            problems.append(f"persona {persona!r} is not a slug")
        else:
            template = ctx.template_for(persona)
            if template is None:
                problems.append(
                    f"persona {persona} resolves to no template in the examined population"
                )
    problems += _subject_problems(frontmatter, contract)
    if template is not None and "subject" in frontmatter:
        if frontmatter["subject"] != template.frontmatter.get("subject"):
            problems.append(
                f"subject {frontmatter['subject']} is not the template's "
                f"{template.frontmatter.get('subject')}"
            )
    duty = frontmatter.get("duty")
    registry = [entry["id"] for entry in contract["duties"]]
    if "duty" in frontmatter and duty not in registry:
        problems.append(f"duty {duty!r} is not in the duty registry")
    kind = subject_kind(frontmatter.get("subject"))
    if kind == LANGUAGE_KIND:
        if "cefr" not in frontmatter:
            problems.append("cefr is missing, and a language subject requires it")
        elif frontmatter["cefr"] not in contract["cefr"]:
            problems.append(
                f"cefr {frontmatter['cefr']!r} is not one of {', '.join(contract['cefr'])}"
            )
    elif "cefr" in frontmatter:
        problems.append("cefr is set, but only a language subject carries one")
    expected_lang = template.frontmatter.get("lang") if template is not None else None
    problems += _lang_problems(frontmatter, expected=expected_lang)
    if "memory" in frontmatter:
        problems += _unique_strings(frontmatter["memory"], "memory", MEMORY_TOKEN)
    if "sources" in frontmatter:
        problems += _unique_strings(
            frontmatter["sources"], "sources", re.compile(r"^\S+$")
        )
    problems += list(doc.problems)
    for match in SLOT_TOKEN.finditer(doc.text):
        problems.append(f"an unfilled slot {match.group(0)}")
    return problems


def check_output_contract(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for broken in ctx.broken:
        if broken.guess in ("output", "unknown"):
            examined += 1
            findings.append(f"{ctx.show(broken.path)}: {broken.reason}")
    for doc in ctx.outputs():
        examined += 1
        findings += [
            f"{ctx.show(doc.path)}: {problem}" for problem in _output_problems(doc, ctx)
        ]
    return Outcome(examined, tuple(findings))


def _names(text: str, word: str) -> bool:
    return re.search(rf"(?<![a-z0-9-]){re.escape(word)}(?![a-z0-9-])", text) is not None


def check_duty_composition(ctx: Context) -> Outcome:
    registry = {
        entry["id"]: list(entry["sections"]) for entry in ctx.contract["duties"]
    }
    findings: list = []
    examined = 0
    for doc in ctx.templates():
        examined += 1
        where = ctx.show(doc.path)
        duties = doc.frontmatter.get("duties")
        duties = (
            [duty for duty in duties if isinstance(duty, str)]
            if isinstance(duties, list)
            else []
        )
        for duty in duties:
            if duty not in registry:
                findings.append(f"{where}: duty {duty} is not in the duty registry")
        extra = doc.frontmatter.get("sections")
        if isinstance(extra, dict):
            for duty in extra:
                if duty not in duties:
                    findings.append(
                        f"{where}: sections names duty {duty}, which duties does not list"
                    )
        described = _section(doc, "duties")
        if described is not None:
            for duty in duties:
                if not _names(described.body, duty):
                    findings.append(
                        f"{where}: section duties never names the duty {duty}"
                    )
    for doc in ctx.outputs():
        examined += 1
        where = ctx.show(doc.path)
        duty = doc.frontmatter.get("duty")
        template = ctx.template_for(doc.frontmatter.get("persona"))
        required = list(registry.get(duty, [])) if isinstance(duty, str) else []
        if template is not None:
            duties = template.frontmatter.get("duties")
            if isinstance(duties, list) and duty not in duties:
                findings.append(
                    f"{where}: duty {duty} is not one of its persona's duties"
                )
            extra = template.frontmatter.get("sections")
            if isinstance(extra, dict) and isinstance(extra.get(duty), list):
                required += [item for item in extra[duty] if isinstance(item, str)]
        for section_id in dict.fromkeys(required):
            section = _section(doc, section_id)
            if section is None:
                findings.append(
                    f"{where}: the {duty} duty requires section {section_id}"
                )
            elif not section.body.strip():
                findings.append(f"{where}: section {section_id} is empty")
    return Outcome(examined, tuple(findings))


def _claimed_texts(ctx: Context) -> list:
    texts = [(doc.path, doc.text) for doc in ctx.docs]
    texts += [(broken.path, broken.text) for broken in ctx.broken]
    return sorted(texts, key=lambda pair: str(pair[0]))


def _scan(text: str, rows: Iterable, first_line: int = 1) -> list:
    """(line, id, match) for every pattern row that matches a line of NFKC-normalised `text`."""
    hits = []
    for offset, line in enumerate(text.split("\n")):
        folded = unicodedata.normalize("NFKC", line)
        for row_id, pattern in rows:
            for match in pattern.finditer(folded):
                hits.append((first_line + offset, row_id, match.group(0)))
    return hits


def scan(text: str, rows: Iterable, first_line: int = 1) -> list:
    """Public, stable API (SPEC-V2-2212): the per-line NFKC scan every class shares.

    `rows` are `(id, compiled re.Pattern)` pairs, as `load_patterns()` returns each group. The
    result is a list of `(line, id, matched text)`, with 1-based lines counted from `first_line`.
    """
    return _scan(text, rows, first_line)


def check_no_dates(ctx: Context) -> Outcome:
    rows = ctx.patterns["dates"] + ctx.patterns["timelines"]
    findings: list = []
    texts = _claimed_texts(ctx)
    for path, text in texts:
        for line, row_id, found in _scan(text, rows):
            findings.append(f"{ctx.show(path)}:{line}: {row_id} {found!r}")
    return Outcome(len(texts), tuple(findings))


def _frontmatter_keys(value: object) -> list:
    keys = []
    if isinstance(value, dict):
        for key, inner in value.items():
            keys.append(str(key))
            keys += _frontmatter_keys(inner)
    elif isinstance(value, list):
        for inner in value:
            keys += _frontmatter_keys(inner)
    return keys


def _raw_frontmatter_keys(text: str) -> list:
    keys = []
    for line in text.removeprefix("﻿").split("\n")[1:CLAIM_SCAN_LINES]:
        if line.rstrip("\r") == "---":
            break
        match = RAW_KEY.match(line)
        if match:
            keys.append(match["key"])
    return keys


def check_scrubber(ctx: Context) -> Outcome:
    """v9's scrub_public() categories. A finding names the rule and the line, never the value."""
    findings: list = []
    deny = ctx.deny
    records = [
        (doc.path, doc.text, _frontmatter_keys(doc.frontmatter)) for doc in ctx.docs
    ]
    records += [(b.path, b.text, _raw_frontmatter_keys(b.text)) for b in ctx.broken]
    records.sort(key=lambda record: str(record[0]))
    for path, text, keys in records:
        where = ctx.show(path)
        for key in keys:
            for index, (origin, marker) in enumerate(deny["key_markers"]):
                if marker in key.casefold():
                    label = marker if origin == "public" else f"private marker {index}"
                    findings.append(f"{where}: frontmatter key {key} matches {label}")
        for offset, line in enumerate(text.split("\n")):
            folded = unicodedata.normalize("NFKC", line)
            for origin, row_id, pattern in deny["patterns"]:
                if pattern.search(folded):
                    findings.append(f"{where}:{offset + 1}: {origin} pattern {row_id}")
            lowered = folded.casefold()
            for index, (_origin, literal) in enumerate(deny["literals"]):
                if literal in lowered:
                    findings.append(f"{where}:{offset + 1}: private literal {index}")
    return Outcome(len(records), tuple(findings))


def _journal_segment(target: str, words: list) -> bool:
    for segment in re.split(r"[/\\]", target.split("|", 1)[0].split("#", 1)[0]):
        segment = segment.strip().casefold().removesuffix(".md")
        for word in words:
            if segment == word or re.match(rf"^{re.escape(word)}[\s_-]", segment):
                return True
    return False


def check_memory_scope(ctx: Context) -> Outcome:
    allowed = list(ctx.contract["memory_sources"])
    words = [word for _origin, word in ctx.deny["journal_paths"]] or ["journal"]
    findings: list = []
    examined = 0
    for doc in ctx.templates():
        examined += 1
        memory = doc.frontmatter.get("memory")
        for item in memory if isinstance(memory, list) else []:
            if not isinstance(item, str):
                continue
            if any(word in item.casefold() for word in words):
                findings.append(
                    f"{ctx.show(doc.path)}: memory names the journal ({item})"
                )
            elif item not in allowed:
                findings.append(
                    f"{ctx.show(doc.path)}: memory source {item} is not one of {', '.join(allowed)}"
                )
    for doc in ctx.outputs():
        examined += 1
        where = ctx.show(doc.path)
        template = ctx.template_for(doc.frontmatter.get("persona"))
        declared = template.frontmatter.get("memory") if template is not None else None
        sources = declared if isinstance(declared, list) else allowed
        own = doc.frontmatter.get("subject")
        memory = doc.frontmatter.get("memory")
        for token in memory if isinstance(memory, list) else []:
            if not isinstance(token, str):
                continue
            if any(word in token.casefold() for word in words):
                findings.append(f"{where}: memory reads the journal ({token})")
                continue
            match = MEMORY_TOKEN.match(token)
            if match is None:
                continue
            if match["source"] not in sources:
                findings.append(
                    f"{where}: memory reads {match['source']}, which its persona does not declare"
                )
            if match["subject"] != own:
                findings.append(
                    f"{where}: memory reads {match['subject']}'s {match['source']}, not {own}'s"
                )
        for offset, line in enumerate(doc.body.split("\n")):
            number = doc.body_line + offset
            targets = [m["target"] for m in WIKILINK.finditer(line)]
            targets += [
                m["target"]
                for m in MARKDOWN_LINK.finditer(line)
                if not WEB_URL.match(m["target"])
            ]
            for target in targets:
                if _journal_segment(target, words):
                    findings.append(f"{where}:{number}: a link into the journal")
            for match in TAG.finditer(line):
                if _journal_segment(match["tag"], words):
                    findings.append(f"{where}:{number}: a journal tag #{match['tag']}")
    return Outcome(examined, tuple(findings))


def check_no_human_claim(ctx: Context) -> Outcome:
    findings: list = []
    texts = _claimed_texts(ctx)
    for path, text in texts:
        for line, row_id, found in _scan(text, ctx.patterns["human_claims"]):
            findings.append(f"{ctx.show(path)}:{line}: {row_id} {found!r}")
    return Outcome(len(texts), tuple(findings))


def _prose(body: str) -> str:
    """The body as a reader sees it: HTML comments (the section markers) and the `!` of an image or
    an embed removed, so neither is counted as an exclamation."""
    return HTML_COMMENT.sub(" ", body).replace("![", "[")


def _emoji(character: str, ranges: list) -> bool:
    point = ord(character)
    return any(int(low, 16) <= point <= int(high, 16) for low, high in ranges)


def check_voice(ctx: Context) -> Outcome:
    limits = ctx.patterns["voice_limits"]
    ranges = limits.get("emoji_ranges", [])
    findings: list = []
    outputs = ctx.outputs()
    for doc in outputs:
        where = ctx.show(doc.path)
        for line, row_id, found in _scan(
            doc.body, ctx.patterns["voice"], doc.body_line
        ):
            findings.append(f"{where}:{line}: {row_id} {found!r}")
        for offset, line in enumerate(doc.body.split("\n")):
            for character in line:
                if _emoji(character, ranges):
                    findings.append(
                        f"{where}:{doc.body_line + offset}: emoji U+{ord(character):04X}"
                    )
                    break
        prose = _prose(doc.body)
        exclamations = prose.count("!") + prose.count("\uff01")
        units = len(LATIN_WORD.findall(prose)) + len(CJK.findall(prose)) // 2
        if (
            exclamations >= int(limits.get("exclamations_min", 3))
            and exclamations * int(limits.get("units_per_exclamation", 40)) > units
        ):
            findings.append(
                f"{where}: {exclamations} exclamation marks in {units} words"
            )
    return Outcome(len(outputs), tuple(findings))


CHECKS = {
    "template-schema": check_template_schema,
    "output-contract": check_output_contract,
    "duty-composition": check_duty_composition,
    "no-dates": check_no_dates,
    "scrubber": check_scrubber,
    "memory-scope": check_memory_scope,
    "no-human-claim": check_no_human_claim,
    "voice": check_voice,
}


# --- the command line ---------------------------------------------------------------------------


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="persona-core-probe.py",
        description="Judge persona templates and persona output against the persona contract v1.",
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
        "--deny-list",
        help=f"the private deny-list (default: ${DENY_ENV}, when it is set)",
    )
    parser.add_argument(
        "--pack-dir",
        help="the persona-core pack directory (default: beside this script)",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    parse = commands.add_parser("parse", help="print one persona file as JSON")
    parse.add_argument("file")
    commands.add_parser("classes", help="list the classes")
    return parser


def _usage(message: str) -> int:
    print(f"persona-core-probe: {message}", file=sys.stderr)
    return EXIT_USAGE


def _parse_command(path: str) -> int:
    try:
        doc = parse_document(path)
    except ContractError as error:
        print(f"parse: {path}: {error}")
        return EXIT_FINDING
    print(
        json.dumps(
            {
                "path": str(doc.path),
                "kind": doc.kind,
                "frontmatter": doc.frontmatter,
                "sections": [
                    {"id": s.id, "title": s.title, "line": s.line, "body": s.body}
                    for s in doc.sections
                ],
                "body_line": doc.body_line,
                "problems": list(doc.problems),
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return EXIT_GREEN


def main(argv: "Sequence[str] | None" = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "classes":
        print("\n".join(CLASSES))
        return EXIT_GREEN
    if args.command == "parse":
        return _parse_command(args.file)
    name = args.name
    root = Path(args.root)
    if not root.is_dir():
        return _usage(f"--root is not a directory: {args.root}")
    bases = [Path(subject) for subject in args.subject] or [root]
    for base in bases:
        if not base.exists():
            return _usage(f"--subject does not exist: {base}")
    pack_dir = Path(args.pack_dir) if args.pack_dir else None
    private = args.deny_list or os.environ.get(DENY_ENV) or None
    try:
        contract = load_contract(pack_dir)
        patterns = load_patterns(pack_dir)
        deny = load_deny(pack_dir, Path(private) if private else None)
    except ContractError as error:
        print(f"{name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    for kind in args.kind:
        if kind not in contract["kinds"]:
            return _usage(f"--kind {kind} is not one of {', '.join(contract['kinds'])}")
    docs, broken = load_population(bases, args.kind)
    ctx = Context(
        root=root,
        docs=docs,
        broken=broken,
        contract=contract,
        patterns=patterns,
        deny=deny,
    )
    outcome = CHECKS[name](ctx)
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

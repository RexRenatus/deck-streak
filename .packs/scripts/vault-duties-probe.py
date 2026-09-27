#!/usr/bin/env python3
"""vault-duties-probe.py -- the executable contract for DeckStreak's vault duties.

SPEC-V2-2218. DeckStreak's agent keeps three duties in the owner's Obsidian vault: the DAILY NOTE
(tomorrow's note, linking the day's readings, drills and filed captures), the WEEKLY SYNTHESIS
(themes and connections across the week's notes, every claim linked) and the INBOX CURATOR
(filing `90-Inbox/` captures into their folders). The engine stages every run in a directory of
its own: the notes the run writes, at their vault paths, beside a `duty-run.json` record of what it
did. This script judges that directory BEFORE anything reaches the vault, and an output that fails
a blocking class is never applied.

    note-properties      block     Obsidian properties: known keys, list tags, no deprecated keys
    note-links           block     every wikilink, embed and markdown link resolves, unambiguously
    note-names           block     a name the vault's folder rules and every file system accept
    write-confinement    block     writes only in the duty's folders, and every write recorded
    no-executable        block     no code a plugin runs, no live view, no link that acts
    never-deletes        block     no delete verb, no overwrite, filed captures byte for byte
    daily-note           block     the daily note links each prepared note in its own section
    synthesis-cites      block     every claim of the synthesis links a note it synthesises
    journal-never-leaks  block     no link, tag or verbatim run from the journal, in any text
    nothing-leaves       block     no Publish flag, no remote fetch, no machine path
    no-dates             block     no date or timeline in the text (persona-core's patterns)
    claim-sentences      advisory  each sentence of a claim carries its own link
    synthesis-own-words  advisory  the synthesis does not copy its sources
    orphan-notes         advisory  every note a run creates or files is linked from somewhere
    inbox-zero           advisory  the curator leaves no capture behind

It is standard library only (Python >= 3.10), vendorable, and reads any tree through `--root` or
the directories `--subject` names. The vocabularies live in the pack's `contract.json`, the rails
in `rails.json` and the public default layout in `layout.json`, so a new rail or a new duty is a
data row, never a code branch. It reuses persona-core's PUBLIC API through importlib, never a copy:
`load_patterns()` and `scan()` for dates and timelines, and `load_deny()` for where the journal is.

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`. Exit 0
is green, 1 a finding, 2 a usage error, and 3 VOID: nothing was examined or the pack's data could
not be read, which is never a pass. A finding names a staged file and a line and never echoes the
journal's text or its paths.

The annotations are evaluated at run time on purpose (no `from __future__ import annotations`): a
sibling loads this file through importlib, and a dataclass with string annotations looks its module
up in sys.modules, which fails when a loader has not registered it.
"""

import argparse
import datetime
import hashlib
import importlib.util
import json
import os
import posixpath
import re
import sys
import unicodedata
import urllib.parse
from collections.abc import Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

NOTE_SCHEMA = "phx.duty.vault.note.v1"
RUN_SCHEMA = "phx.duty.vault.run.v1"
LAYOUT_SCHEMA = "phx.duty.vault.layout.v1"
CONTRACT_SCHEMA = "phx.duty.vault.contract.v1"
RAILS_SCHEMA = "phx.duty.vault.rails.v1"
DENY_SCHEMA = "phx.persona.deny.v1"
RUN_FILE = "duty-run.json"
SCRIPTS = Path(__file__).resolve().parent
DEFAULT_PACK_DIR = SCRIPTS.parent / "skills" / "packs" / "vault-duties"
DEFAULT_PERSONA_CORE = SCRIPTS / "persona-core-probe.py"
DENY_ENV = "PERSONA_CORE_DENY_LIST"
LAYOUT_ENV = "VAULT_DUTIES_LAYOUT"
SKIPPED_DIRECTORIES = frozenset({"node_modules", "target"})
TEXT_SUFFIXES = (".md", ".txt", ".json", ".html")
MAX_NAME_BYTES = 255

CLASSES = (
    "note-properties",
    "note-links",
    "note-names",
    "write-confinement",
    "no-executable",
    "never-deletes",
    "daily-note",
    "synthesis-cites",
    "journal-never-leaks",
    "nothing-leaves",
    "no-dates",
    "claim-sentences",
    "synthesis-own-words",
    "orphan-notes",
    "inbox-zero",
)

KEY_LINE = re.compile(
    r"^(?P<key>[a-z][a-z0-9_]*|x-[a-z0-9]+(?:-[a-z0-9]+)*):[ ]+(?P<value>\S.*)$"
)
EXTENSION_KEY = re.compile(r"^x-[a-z0-9]+(?:-[a-z0-9]+)*$")
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
HEADING = re.compile(r"^ {0,3}(?P<hashes>#{1,6})(?:[ \t]+(?P<content>.*?))?[ \t]*$")
MARKER = re.compile(r"<!--\s*section:\s*(?P<id>[^\s>]*)\s*-->\s*$")
FENCE_OPEN = re.compile(r"^ {0,3}(?P<fence>`{3,}|~{3,})(?P<info>.*)$")
CODE_SPAN = re.compile(r"(?P<ticks>`+)(?P<code>.+?)(?<!`)(?P=ticks)(?!`)")
HTML_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)
OBSIDIAN_COMMENT = re.compile(r"%%.*?%%", re.DOTALL)
WIKILINK = re.compile(r"(?P<embed>!?)\[\[(?P<inner>[^\[\]\n]*?)\]\]")
MARKDOWN_LINK = re.compile(
    r"(?P<embed>!?)\[(?P<text>[^\]\n]*)\]\((?P<dest><[^>\n]*>|[^)\s]*)"
    r"(?:[ \t]+(?:\"[^\"\n]*\"|'[^'\n]*'))?\)"
)
AUTOLINK = re.compile(r"<(?P<url>[A-Za-z][A-Za-z0-9+.-]{1,31}:[^<>\s]*)>")
SCHEME = re.compile(r"^(?P<scheme>[A-Za-z][A-Za-z0-9+.-]*):")
TAG = re.compile(r"(?:^|(?<=[\s(]))#(?P<tag>[^\s#!\"$%&'()*+,.:;<=>?@\[\]^`{|}~\\]+)")
HTML_TAG = re.compile(
    r"<(?P<close>/?)(?P<name>[A-Za-z][A-Za-z0-9-]*)(?P<attrs>(?:\s[^<>]*?)?)\s*/?>"
)
HTML_ATTRIBUTE = re.compile(
    r"(?P<name>[A-Za-z_:][-A-Za-z0-9_:.]*)(?:\s*=\s*(?P<value>\"[^\"]*\"|'[^']*'|[^\s\"'=<>`]+))?"
)
CSS_URL = re.compile(r"url\(\s*['\"]?(?P<url>[^'\")\s]+)", re.IGNORECASE)
LIST_ITEM = re.compile(r"^\s*(?:[-*+]|\d{1,9}[.)])\s+(?:\[[ xX]\]\s+)?(?P<text>.*)$")
BLOCKQUOTE = re.compile(r"^\s*>\s?")
TEMPLATE_VARIABLE = re.compile(r"\{\{[^{}\n]*\}\}")
BLOCK_ID = re.compile(r"(?:^|\s)\^(?P<id>[A-Za-z0-9-]+)\s*$")
SENTENCE_END = re.compile(r"(?<=[.!?])\s+(?=\S)|(?<=[。！？])")
LINK_ONLY = re.compile(
    r"^(?:\s|[.,;:!?。！？、]|\[\[[^\[\]\n]*\]\]|\[[^\]\n]*\]\([^)\s]*\))*$"
)
CJK = "\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uac00-\ud7af\uf900-\ufaff"
TOKEN = re.compile(rf"[{CJK}]|(?:(?![{CJK}])[^\W_])+")
RESERVED_CHARACTERS = '<>:"/\\|?*'
LINK_BREAKERS = "#^[]|"
RESERVED_NAMES = frozenset(
    ["con", "prn", "aux", "nul"]
    + [f"{port}{digit}" for port in ("com", "lpt") for digit in "123456789¹²³"]
)
HEADING_NOISE = re.compile(r"[#|^:%\[\]*_~=`]")


class ContractError(ValueError):
    """The pack's data, a layout or a record does not parse. str() is the reason."""


# --- the pack's data -----------------------------------------------------------------------------


def _load_json(path: Path, schema: str) -> dict:
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8"))
    except OSError as error:
        raise ContractError(
            f"{Path(path).name}: unreadable ({error.strerror or error})"
        ) from error
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ContractError(f"{Path(path).name}: not JSON ({error})") from error
    if not isinstance(data, dict) or data.get("schema") != schema:
        raise ContractError(f"{Path(path).name}: schema is not {schema}")
    return data


def load_contract(pack_dir: "Path | None" = None) -> dict:
    """The pack's contract.json: the note keys, the vault duties, the op grammar."""
    contract = _load_json(
        Path(pack_dir or DEFAULT_PACK_DIR) / "contract.json", CONTRACT_SCHEMA
    )
    for key in (
        "note_keys",
        "note_required",
        "deprecated_keys",
        "agent_claims",
        "duties",
        "ops",
    ):
        if key not in contract:
            raise ContractError(f"contract.json: {key} is missing")
    if (
        not isinstance(contract.get("quote_window"), int)
        or contract["quote_window"] < 2
    ):
        raise ContractError(
            "contract.json: quote_window is not an integer of at least 2"
        )
    for duty in contract["duties"]:
        if not isinstance(duty, dict) or not SLUG.match(str(duty.get("id", ""))):
            raise ContractError("contract.json: a duty has no slug id")
    return contract


def load_rails(pack_dir: "Path | None" = None) -> dict:
    """The pack's rails.json: fences, HTML, schemes and macros an agent note may not carry."""
    rails = _load_json(Path(pack_dir or DEFAULT_PACK_DIR) / "rails.json", RAILS_SCHEMA)
    for key in (
        "fence_allow",
        "fence_known",
        "fence_prefixes",
        "templater_open",
        "inline_query_prefixes",
        "html_allow",
        "html_attributes_allow",
        "executable_schemes",
        "egress_schemes",
        "remote_schemes",
        "math_macros_refused",
        "dynamic_embed_extensions",
        "publish_key",
    ):
        if key not in rails:
            raise ContractError(f"rails.json: {key} is missing")
    return rails


def load_layout(path: "Path | str") -> dict:
    """A layout file: the public default, a private one, or one a record names."""
    return _load_json(Path(path), LAYOUT_SCHEMA)


def layout_problems(layout: dict) -> list:
    """What makes a layout unusable or unsafe. An empty list is a sound layout."""
    problems = []
    periodic = layout.get("periodic")
    if not isinstance(periodic, dict):
        return ["the layout has no periodic map"]
    for kind in ("daily", "weekly"):
        entry = periodic.get(kind)
        if not isinstance(entry, dict) or not isinstance(entry.get("folder"), str):
            problems.append(f"periodic.{kind} has no folder")
            continue
        try:
            compile_format(str(entry.get("format", "")))
        except ContractError as error:
            problems.append(f"periodic.{kind}: {error}")
    inbox = layout.get("inbox")
    if not isinstance(inbox, str) or not inbox:
        problems.append("the layout names no inbox")
    duties = layout.get("duties")
    if not isinstance(duties, dict):
        return problems + ["the layout has no duties map"]
    journal = [item for item in layout.get("journal", []) if isinstance(item, str)]
    for duty, rules in duties.items():
        if not isinstance(rules, dict):
            problems.append(f"duties.{duty} is not an object")
            continue
        for key in ("writes", "moves_from", "moves_to"):
            for folder in rules.get(key, []):
                if not isinstance(folder, str):
                    problems.append(f"duties.{duty}.{key} holds a non-string")
                elif folder.startswith("@"):
                    if key != "writes" or folder[1:] not in ("daily", "weekly"):
                        problems.append(f"duties.{duty}.{key} names {folder}")
                elif any(
                    _under(folder, entry) or _under(entry, folder) for entry in journal
                ):
                    problems.append(f"duties.{duty}.{key} overlaps the journal")
        if isinstance(inbox, str) and any(
            isinstance(folder, str) and _under(folder, inbox)
            for folder in rules.get("moves_to", [])
        ):
            problems.append(f"duties.{duty}.moves_to names the inbox")
    return problems


# --- dates as Moment.js writes them (Obsidian's daily-note format) ------------------------------

_MONTHS = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
]
_DAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"]
_ORDINAL = r"(?:st|nd|rd|th)"
_WEEK = r"(?:0?[1-9]|[1-4]\d|5[0-3])"
_TOKENS = (
    ("YYYY", r"(\d{4})", "year"),
    ("gggg", r"\d{4}", ""),
    ("GGGG", r"\d{4}", ""),
    ("MMMM", "(" + "|".join(m.title() for m in _MONTHS) + ")", "month-name"),
    ("DDDD", r"\d{3}", ""),
    ("dddd", "(?:" + "|".join(d.title() for d in _DAYS) + ")", ""),
    ("DDDo", r"\d{1,3}" + _ORDINAL, ""),
    ("MMM", "(" + "|".join(m[:3].title() for m in _MONTHS) + ")", "month-abbr"),
    ("DDD", r"\d{1,3}", ""),
    ("ddd", "(?:" + "|".join(d[:3].title() for d in _DAYS) + ")", ""),
    ("YY", r"\d{2}", ""),
    ("gg", r"\d{2}", ""),
    ("GG", r"\d{2}", ""),
    ("MM", r"(0[1-9]|1[0-2])", "month"),
    ("Mo", r"((?:[1-9]|1[0-2]))" + _ORDINAL, "month"),
    ("DD", r"(0[1-9]|[12]\d|3[01])", "day"),
    ("Do", r"([1-9]|[12]\d|3[01])" + _ORDINAL, "day"),
    ("dd", "(?:" + "|".join(d[:2].title() for d in _DAYS) + ")", ""),
    ("do", r"[0-6]" + _ORDINAL, ""),
    ("Qo", r"[1-4]" + _ORDINAL, ""),
    ("WW", r"(?:0[1-9]|[1-4]\d|5[0-3])", ""),
    ("Wo", _WEEK + _ORDINAL, ""),
    ("ww", r"(?:0[1-9]|[1-4]\d|5[0-3])", ""),
    ("wo", _WEEK + _ORDINAL, ""),
    ("HH", r"(?:[01]\d|2[0-3])", ""),
    ("hh", r"(?:0[1-9]|1[0-2])", ""),
    ("mm", r"[0-5]\d", ""),
    ("ss", r"[0-5]\d", ""),
    ("M", r"([1-9]|1[0-2])", "month"),
    ("D", r"([1-9]|[12]\d|3[01])", "day"),
    ("d", r"[0-6]", ""),
    ("Q", r"[1-4]", ""),
    ("W", _WEEK, ""),
    ("w", _WEEK, ""),
    ("E", r"[1-7]", ""),
    ("e", r"[0-6]", ""),
    ("H", r"(?:1?\d|2[0-3])", ""),
    ("h", r"(?:[1-9]|1[0-2])", ""),
    ("m", r"[0-5]?\d", ""),
    ("s", r"[0-5]?\d", ""),
    ("A", r"(?:AM|PM)", ""),
    ("a", r"(?:am|pm)", ""),
)


@dataclass(frozen=True)
class DateFormat:
    """A Moment.js format compiled to a full-match pattern, and which groups carry the date."""

    source: str
    pattern: re.Pattern
    roles: tuple


def compile_format(source: str) -> DateFormat:
    """Compile a Moment.js display format; `[text]` is literal and `/` makes folders."""
    if not source:
        raise ContractError("the format is empty")
    parts: list = []
    roles: list = []
    index = 0
    while index < len(source):
        if source[index] == "[":
            close = source.find("]", index)
            if close < 0:
                raise ContractError(f"format {source!r} opens [ and never closes it")
            parts.append(re.escape(source[index + 1 : close]))
            index = close + 1
            continue
        for token, pattern, role in _TOKENS:
            if source.startswith(token, index):
                parts.append(pattern)
                roles += [role] * re.compile(pattern).groups
                index += len(token)
                break
        else:
            parts.append(re.escape(source[index]))
            index += 1
    return DateFormat(source, re.compile("".join(parts)), tuple(roles))


def format_problem(stem: str, fmt: DateFormat) -> "str | None":
    """Why `stem` (a path without `.md`) is not a name in `fmt`, or None when it is."""
    match = fmt.pattern.fullmatch(stem)
    if match is None:
        return f"does not fit the format {fmt.source}"
    values: dict = {}
    for role, value in zip(fmt.roles, match.groups()):
        if not role or value is None:
            continue
        if role == "month-name":
            role, value = "month", _MONTHS.index(value.casefold()) + 1
        elif role == "month-abbr":
            role, value = "month", [m[:3] for m in _MONTHS].index(value.casefold()) + 1
        else:
            value = int(value)
        if values.setdefault(role, value) != value:
            return f"names two different {role}s in the format {fmt.source}"
    if {"year", "month", "day"} <= set(values):
        try:
            datetime.date(values["year"], values["month"], values["day"])
        except ValueError:
            return f"is not a calendar day in the format {fmt.source}"
    return None


# --- one note: frontmatter, sections, links -------------------------------------------------------


@dataclass(frozen=True)
class Section:
    """One marked section: `## <title> <!-- section:<id> -->`, up to the next `#` or `##`."""

    id: str
    title: str
    line: int
    body: str
    first: int
    last: int


@dataclass(frozen=True)
class Link:
    """A wikilink, embed, markdown link, autolink or a property's wikilink, as written."""

    form: str
    embed: bool
    target: str
    subpath: str
    alias: str
    line: int
    text: str
    scheme: str


@dataclass(frozen=True)
class Note:
    """A note as Obsidian reads it. Line numbers are 1-based. A fatal defect is kept, not raised."""

    path: Path
    text: str
    frontmatter: dict
    body: str
    body_line: int
    sections: tuple
    links: tuple
    tags: tuple
    fences: tuple
    code_spans: tuple
    prose: tuple
    problems: tuple
    fatal: str | None

    def section(self, section_id: str) -> "Section | None":
        return next((s for s in self.sections if s.id == section_id), None)

    def claimed_by(self, marks: Iterable) -> bool:
        return isinstance(self.frontmatter.get("schema"), str) and any(
            self.frontmatter["schema"].startswith(mark) for mark in marks
        )


def _reject_constant(name: str) -> object:
    raise ValueError(f"{name} is not JSON")


def _blank(match: "re.Match") -> str:
    return re.sub(r"[^\n]", " ", match.group(0))


def _frontmatter(lines: list) -> tuple:
    """(frontmatter, closing index or None, fatal reason or None)."""
    if not lines or lines[0] != "---":
        return {}, None, "line 1 is not ---"
    frontmatter: dict = {}
    for index in range(1, len(lines)):
        line = lines[index]
        if line == "---":
            return frontmatter, index, None
        if line == "" and index == len(lines) - 1:
            break
        match = KEY_LINE.match(line)
        if match is None:
            return (
                frontmatter,
                None,
                f"line {index + 1}: not `key: <one-line JSON value>`",
            )
        key = match["key"]
        if key in frontmatter:
            return frontmatter, None, f"line {index + 1}: duplicate key {key}"
        try:
            frontmatter[key] = json.loads(
                match["value"], parse_constant=_reject_constant
            )
        except ValueError:
            return frontmatter, None, f"line {index + 1}: {key} is not one JSON value"
    return frontmatter, None, "the frontmatter is never closed by ---"


def _mask_code(lines: list) -> tuple:
    """Lines with fenced code and code spans blanked; each fence's info; each code span."""
    masked: list = []
    fences: list = []
    spans: list = []
    fence = None
    for index, line in enumerate(lines):
        if fence is not None:
            if re.match(rf"^ {{0,3}}{re.escape(fence[0])}{{{fence[1]},}}[ \t]*$", line):
                fence = None
            masked.append("")
            continue
        opened = FENCE_OPEN.match(line)
        if opened and not (opened["fence"][0] == "`" and "`" in opened["info"]):
            fence = (opened["fence"][0], len(opened["fence"]))
            fences.append((index, opened["info"].strip()))
            masked.append("")
            continue
        for span in CODE_SPAN.finditer(line):
            spans.append((index, span["code"]))
        masked.append(CODE_SPAN.sub(_blank, line))
    return masked, fences, spans


def _split_target(raw: str) -> tuple:
    path, hashed, subpath = raw.partition("#")
    return path.strip(), ("#" + subpath) if hashed else ""


def _links_in(line: str, number: int) -> list:
    links = []
    for match in WIKILINK.finditer(line):
        target_part, _bar, alias = match["inner"].partition("|")
        target, subpath = _split_target(target_part)
        links.append(
            Link(
                "wikilink",
                bool(match["embed"]),
                target,
                subpath,
                alias.strip(),
                number,
                match.group(0),
                "",
            )
        )
    for match in MARKDOWN_LINK.finditer(line):
        dest = match["dest"]
        if dest.startswith("<") and dest.endswith(">"):
            dest = dest[1:-1]
        scheme = SCHEME.match(dest)
        if scheme is not None and len(scheme["scheme"]) > 1:
            links.append(
                Link(
                    "markdown",
                    bool(match["embed"]),
                    dest,
                    "",
                    match["text"],
                    number,
                    match.group(0),
                    scheme["scheme"].casefold(),
                )
            )
            continue
        path, hashed, subpath = dest.partition("#")
        links.append(
            Link(
                "markdown",
                bool(match["embed"]),
                urllib.parse.unquote(path).strip(),
                ("#" + urllib.parse.unquote(subpath)) if hashed else "",
                match["text"],
                number,
                match.group(0),
                "",
            )
        )
    for match in AUTOLINK.finditer(line):
        scheme = SCHEME.match(match["url"])
        links.append(
            Link(
                "autolink",
                False,
                match["url"],
                "",
                "",
                number,
                match.group(0),
                scheme["scheme"].casefold() if scheme else "",
            )
        )
    return links


def _property_links(frontmatter: dict) -> list:
    links = []
    for offset, (key, value) in enumerate(frontmatter.items()):
        values = value if isinstance(value, list) else [value]
        for item in values:
            if isinstance(item, str):
                for link in _links_in(item, offset + 2):
                    if link.form == "wikilink":
                        links.append(
                            Link(
                                "property",
                                link.embed,
                                link.target,
                                link.subpath,
                                link.alias,
                                link.line,
                                link.text,
                                "",
                            )
                        )
    return links


def _sections(masked: list, raw: list, first: int) -> tuple:
    sections: list = []
    problems: list = []
    seen: set = set()
    current = None

    def close(end: int) -> None:
        start = current[2] + 1
        body = "\n".join(raw[start - first : end - first + 1]) if end >= start else ""
        sections.append(Section(current[0], current[1], current[2], body, start, end))

    for offset, line in enumerate(masked):
        number = first + offset
        heading = HEADING.match(line)
        if heading is None or len(heading["hashes"]) > 2:
            continue
        if current is not None:
            close(number - 1)
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
        current = (section_id, content[: marker.start()].strip(), number)
    if current is not None:
        close(first + len(masked) - 1)
    return tuple(sections), problems


def parse_note(path: "Path | str", text: "str | None" = None) -> Note:
    """Parse one note. It never raises: a fatal defect is `fatal`, and the text is still read."""
    path = Path(path)
    if text is None:
        try:
            raw = path.read_bytes()
        except OSError as error:
            return _fatal_note(path, "", f"unreadable ({error.strerror or error})")
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            return _fatal_note(path, raw.decode("utf-8", errors="replace"), "not UTF-8")
    lines = [line.removesuffix("\r") for line in text.split("\n")]
    frontmatter, close, fatal = _frontmatter(lines)
    if close is None:
        body_lines, first = lines, 1
    else:
        body_lines, first = lines[close + 1 :], close + 2
    masked, fences, spans = _mask_code(body_lines)
    prose_text = OBSIDIAN_COMMENT.sub(
        _blank, HTML_COMMENT.sub(_blank, "\n".join(masked))
    )
    prose = prose_text.split("\n")
    links: list = _property_links(frontmatter) if fatal is None else []
    tags: list = []
    for offset, line in enumerate(prose):
        links += _links_in(line, first + offset)
        for match in TAG.finditer(line):
            if not match["tag"].replace("/", "").isdigit():
                tags.append((first + offset, match["tag"]))
    sections, problems = _sections(masked, body_lines, first)
    return Note(
        path=path,
        text=text,
        frontmatter=frontmatter if fatal is None else {},
        body="\n".join(body_lines),
        body_line=first,
        sections=sections,
        links=tuple(links),
        tags=tuple(tags),
        fences=tuple((first + index, info) for index, info in fences),
        code_spans=tuple((first + index, code) for index, code in spans),
        prose=tuple(prose),
        problems=tuple(problems),
        fatal=fatal,
    )


def _fatal_note(path: Path, text: str, reason: str) -> Note:
    return Note(
        path, text, {}, text, 1, (), (), (), (), (), tuple(text.split("\n")), (), reason
    )


# --- one run: the record, the layout, the staged files --------------------------------------------


@dataclass(frozen=True)
class Op:
    """One recorded operation. `path` is where it writes: the create or update path, a move's `to`."""

    index: int
    op: str
    path: str | None
    source: str | None
    sha256: str | None
    sha256_before: str | None


@dataclass
class Run:
    """A staged run: the directory, the record's facts, and what the engine staged beside it."""

    directory: Path
    record: Path
    fatal: str | None = None
    duty: str = ""
    layout: dict = field(default_factory=dict)
    layout_problems: list = field(default_factory=list)
    vault_dir: Path | None = None
    index: list = field(default_factory=list)
    inputs: list = field(default_factory=list)
    inbox: list = field(default_factory=list)
    ops: list = field(default_factory=list)
    staged: dict = field(default_factory=dict)
    problems: list = field(default_factory=list)
    notes: list = field(default_factory=list)
    moved: list = field(default_factory=list)
    resolver: object = None


@dataclass(frozen=True)
class AgentNote:
    """A note a run writes: its vault path, the staged file, the parse and the op."""

    vault_path: str
    note: Note
    op: Op


def _strings(value: object) -> bool:
    return isinstance(value, list) and all(isinstance(item, str) for item in value)


def _staged_files(directory: Path) -> dict:
    staged: dict = {}
    for folder, subfolders, files in os.walk(directory):
        subfolders.sort()
        for name in sorted(files):
            full = Path(folder) / name
            relative = full.relative_to(directory).as_posix()
            if relative != RUN_FILE:
                staged[relative] = full
    return staged


def load_run(
    record: "Path | str",
    *,
    layout_override: "dict | None" = None,
    vault_override: "Path | None" = None,
    pack_dir: "Path | None" = None,
) -> Run:
    """Read one `duty-run.json` and the files staged beside it. It never raises."""
    record = Path(record)
    run = Run(directory=record.parent, record=record)
    try:
        data = json.loads(record.read_text(encoding="utf-8"))
    except OSError as error:
        run.fatal = f"unreadable ({error.strerror or error})"
        return run
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        run.fatal = f"not JSON ({error.msg if hasattr(error, 'msg') else error})"
        return run
    if not isinstance(data, dict) or data.get("schema") != RUN_SCHEMA:
        run.fatal = f"schema is not {RUN_SCHEMA}"
        return run
    run.duty = data.get("duty") if isinstance(data.get("duty"), str) else ""
    if not run.duty:
        run.problems.append("duty is missing")
    if layout_override is not None:
        run.layout = layout_override
    else:
        layout = data.get("layout")
        try:
            if isinstance(layout, dict):
                run.layout = layout
            elif isinstance(layout, str):
                run.layout = load_layout(run.directory / layout)
            elif layout is None:
                run.layout = load_layout(
                    Path(pack_dir or DEFAULT_PACK_DIR) / "layout.json"
                )
            else:
                run.problems.append("layout is neither an object nor a path")
        except ContractError as error:
            run.problems.append(f"layout: {error}")
    run.layout_problems = layout_problems(run.layout) if run.layout else []
    if vault_override is not None:
        run.vault_dir = vault_override
    elif isinstance(data.get("vault_root"), str):
        candidate = run.directory / data["vault_root"]
        run.vault_dir = candidate if candidate.is_dir() else None
    if not _strings(data.get("vault")):
        run.problems.append("vault is not a list of vault paths")
    else:
        run.index = list(data["vault"])
    for position, item in enumerate(data.get("inputs", []) or []):
        if isinstance(item, str):
            run.inputs.append((item, None))
        elif isinstance(item, dict) and isinstance(item.get("path"), str):
            section = item.get("section")
            run.inputs.append(
                (item["path"], section if isinstance(section, str) else None)
            )
        else:
            run.problems.append(
                f"input {position + 1} is neither a path nor {{path, section}}"
            )
    for position, item in enumerate(data.get("inbox", []) or []):
        if isinstance(item, dict) and isinstance(item.get("path"), str):
            run.inbox.append((item["path"], item.get("sha256")))
        else:
            run.problems.append(f"inbox entry {position + 1} has no path")
    ops = data.get("ops")
    if not isinstance(ops, list):
        run.problems.append("ops is not a list")
        ops = []
    for position, item in enumerate(ops):
        if not isinstance(item, dict) or not isinstance(item.get("op"), str):
            run.problems.append(f"op {position + 1} names no verb")
            continue
        verb = item["op"]
        path = item.get("to") if verb == "move" else item.get("path")
        run.ops.append(
            Op(
                index=position + 1,
                op=verb,
                path=path if isinstance(path, str) else None,
                source=item.get("from") if isinstance(item.get("from"), str) else None,
                sha256=item.get("sha256")
                if isinstance(item.get("sha256"), str)
                else None,
                sha256_before=item.get("sha256_before")
                if isinstance(item.get("sha256_before"), str)
                else None,
            )
        )
    run.staged = _staged_files(run.directory)
    for op in run.ops:
        if op.path is None or op.path not in run.staged:
            continue
        if op.op in ("create", "update") and op.path.endswith(".md"):
            run.notes.append(AgentNote(op.path, parse_note(run.staged[op.path]), op))
        elif op.op == "move":
            run.moved.append((op, run.staged[op.path]))
    after = set(run.index)
    for op in run.ops:
        if op.op == "move" and op.source:
            after.discard(op.source)
        if op.path and op.op in ("create", "update", "move"):
            after.add(op.path)
    run.resolver = Resolver(after)
    return run


class Resolver:
    """Obsidian's link resolution over the vault as it stands after the run."""

    def __init__(self, paths: Iterable, formats: Iterable = ()) -> None:
        self.paths = sorted(set(paths))
        self.by_path = {path.casefold(): path for path in self.paths}
        self.by_name: dict = {}
        for path in self.paths:
            self.by_name.setdefault(path.rsplit("/", 1)[-1].casefold(), []).append(path)
        self.formats = {f".{ext}" for ext in formats} or None

    def _candidates(self, target: str) -> list:
        extension = posixpath.splitext(target.rsplit("/", 1)[-1])[1].casefold()
        known = self.formats if self.formats is not None else {".md"}
        if extension and (extension in known or extension == ".md"):
            return [target]
        return [target + ".md", target]

    def resolve(self, target: str, source: str) -> tuple:
        """(vault path or None, "" | "unresolved" | "ambiguous")."""
        if not target:
            return source, ""
        target = target.replace("\\", "/").lstrip("/")
        if target.startswith("./") or target.startswith("../"):
            target = posixpath.normpath(
                posixpath.join(posixpath.dirname(source), target)
            )
            if target.startswith(".."):
                return None, "unresolved"
        candidates = self._candidates(target)
        for candidate in candidates:
            hit = self.by_path.get(candidate.casefold())
            if hit is not None:
                return hit, ""
        if "/" in target:
            folder = posixpath.dirname(source)
            if folder:
                for candidate in candidates:
                    hit = self.by_path.get(posixpath.join(folder, candidate).casefold())
                    if hit is not None:
                        return hit, ""
            matches = sorted(
                {
                    path
                    for candidate in candidates
                    for path in self.paths
                    if path.casefold().endswith("/" + candidate.casefold())
                }
            )
            if len(matches) == 1:
                return matches[0], ""
            return None, "ambiguous" if matches else "unresolved"
        for candidate in candidates:
            hits = self.by_name.get(candidate.casefold(), [])
            if len(hits) == 1:
                return hits[0], ""
            if len(hits) > 1:
                return None, "ambiguous"
        return None, "unresolved"


# --- paths, folders and the layout ----------------------------------------------------------------


def _under(path: str, folder: str) -> bool:
    folder = folder.strip("/")
    if not folder:
        return True
    return path.casefold() == folder.casefold() or path.casefold().startswith(
        folder.casefold() + "/"
    )


def _periodic(layout: dict, kind: str) -> tuple:
    entry = (layout.get("periodic") or {}).get(kind) or {}
    return str(entry.get("folder", "")).strip("/"), str(entry.get("format", ""))


def periodic_problem(path: str, layout: dict, kind: str) -> "str | None":
    """Why `path` is not a `kind` periodic note of this layout, or None when it is one."""
    folder, source = _periodic(layout, kind)
    if not path.endswith(".md"):
        return "is not a .md note"
    relative = path[: -len(".md")]
    if folder:
        if not _under(path, folder) or path.casefold() == folder.casefold():
            return f"is not in the {kind} folder"
        relative = relative[len(folder) + 1 :]
    try:
        fmt = compile_format(source)
    except ContractError as error:
        return str(error)
    problem = format_problem(relative, fmt)
    return None if problem is None else f"{problem} (the {kind} format)"


def _write_allowed(path: str, writes: Iterable, layout: dict) -> bool:
    for entry in writes:
        if entry in ("@daily", "@weekly"):
            if periodic_problem(path, layout, entry[1:]) is None:
                return True
        elif (
            isinstance(entry, str)
            and _under(path, entry)
            and path.casefold() != entry.casefold()
        ):
            return True
    return False


def path_problems(path: str) -> list:
    """Why a recorded path is not a normalised vault-relative path. Empty when it is one."""
    if "\\" in path:
        return ["holds a backslash"]
    if path.startswith("/") or re.match(r"^[A-Za-z]:", path):
        return ["is not vault-relative"]
    segments = path.split("/")
    if any(segment in ("", ".", "..") for segment in segments):
        return ["has an empty, . or .. segment: it escapes the vault's paths"]
    if any(segment.startswith(".") for segment in segments[:-1]):
        return ["is inside a dot-folder, which is the vault's configuration"]
    return []


def name_problems(path: str) -> list:
    """Why a name breaks on some file system or in an Obsidian link. Empty when it is portable."""
    problems = []
    for segment in path.split("/"):
        if not segment:
            continue
        bad = sorted(
            {char for char in segment if char in RESERVED_CHARACTERS or ord(char) < 32}
        )
        if bad:
            problems.append(f"{segment!r} holds a reserved character {''.join(bad)!r}")
        breakers = sorted({char for char in segment if char in LINK_BREAKERS})
        if breakers or "%%" in segment:
            problems.append(
                f"{segment!r} holds {''.join(breakers) or '%%'!r}, which breaks an Obsidian link"
            )
        if segment.split(".")[0].casefold().rstrip(" ") in RESERVED_NAMES:
            problems.append(f"{segment!r} is a reserved device name")
        if segment.endswith("."):
            problems.append(f"{segment!r} ends in a period")
        if segment.endswith(" "):
            problems.append(f"{segment!r} ends in a space")
        if segment.startswith("."):
            problems.append(f"{segment!r} starts with a period, which hides it")
        if len(segment.encode("utf-8")) > MAX_NAME_BYTES:
            problems.append(f"a name is longer than {MAX_NAME_BYTES} bytes")
    return problems


def _folder_rule(path: str, layout: dict) -> "tuple | None":
    """(folder, rule) for the deepest layout folder holding `path`."""
    folders = layout.get("folders") or {}
    best = None
    for folder, rule in folders.items():
        if (
            isinstance(folder, str)
            and _under(path, folder)
            and path.casefold() != folder.casefold()
        ):
            if best is None or len(folder) > len(best[0]):
                best = (folder, rule)
    return best


# --- the journal ----------------------------------------------------------------------------------


@dataclass
class Journal:
    """Where the journal is, and, when a vault was readable, the shingles of what it says."""

    words: tuple
    folders: tuple
    window: int
    notes: int = 0
    readable: bool = False
    unreadable: int = 0
    shingles: frozenset = frozenset()

    def holds(self, path: str) -> bool:
        return in_journal(path, self.words, self.folders)


def in_journal(path: str, words: Iterable, folders: Iterable) -> bool:
    """A path whose segment is a journal word, or that lies under a journal folder."""
    target = path.split("|", 1)[0].split("#", 1)[0]
    for segment in re.split(r"[/\\]", target):
        segment = segment.strip().casefold().removesuffix(".md")
        for word in words:
            if segment == word or re.match(rf"^{re.escape(word)}[\s_-]", segment):
                return True
    return any(_under(target, folder) for folder in folders if folder)


def tokens(text: str, first_line: int = 1) -> list:
    """(token, line) pairs: NFKC, casefolded, one token per CJK character."""
    found = []
    for offset, line in enumerate(text.split("\n")):
        folded = unicodedata.normalize("NFKC", line).casefold()
        found += [
            (match.group(0), first_line + offset) for match in TOKEN.finditer(folded)
        ]
    return found


def shingles(text: str, window: int) -> set:
    words = [word for word, _line in tokens(text)]
    return {tuple(words[i : i + window]) for i in range(len(words) - window + 1)}


def repeated_lines(
    text: str, known: "set | frozenset", window: int, first_line: int = 1
) -> list:
    """The lines at which a run of `window` tokens of `text` also appears in `known`."""
    pairs = tokens(text, first_line)
    lines: list = []
    for start in range(len(pairs) - window + 1):
        if tuple(word for word, _line in pairs[start : start + window]) in known:
            line = pairs[start][1]
            if line not in lines:
                lines.append(line)
    return lines


def _journal_words(persona_core: Path, deny_list: "Path | None") -> tuple:
    module = _persona_core(persona_core)
    deny = module.load_deny(None, deny_list)
    return tuple(sorted({word for _origin, word in deny["journal_paths"]}))


def load_journal(
    vault: "Path | None",
    *,
    layout: "dict | None" = None,
    deny_list: "Path | None" = None,
    index: "Iterable | None" = None,
    persona_core: "Path | None" = None,
    window: "int | None" = None,
) -> Journal:
    """Where the journal is (persona-core's journal words, the layout's journal folders) and, when
    `vault` is readable, the shingles of every journal note. Nothing read here is ever printed."""
    words = _journal_words(Path(persona_core or DEFAULT_PERSONA_CORE), deny_list)
    folders = tuple(
        item.strip("/")
        for item in (layout or {}).get("journal", [])
        if isinstance(item, str)
    )
    size = window or load_contract()["quote_window"]
    journal = Journal(words=words, folders=folders, window=size)
    if index is None:
        index = (
            _vault_files(vault) if vault is not None and Path(vault).is_dir() else []
        )
    members = [path for path in index if journal.holds(path)]
    journal.notes = len(members)
    if vault is None or not Path(vault).is_dir():
        return journal
    known: set = set()
    for path in members:
        try:
            known |= shingles(
                (Path(vault) / path).read_text(encoding="utf-8", errors="replace"), size
            )
        except OSError:
            journal.unreadable += 1
    journal.readable = True
    journal.shingles = frozenset(known)
    return journal


def _vault_files(vault: "Path | None") -> list:
    if vault is None:
        return []
    found = []
    for folder, subfolders, files in os.walk(vault):
        subfolders[:] = sorted(name for name in subfolders if not name.startswith("."))
        for name in sorted(files):
            if not name.startswith("."):
                found.append((Path(folder) / name).relative_to(vault).as_posix())
    return found


def journal_leaks(
    text: str, journal: Journal, *, first_line: int = 1, resolver=None, source: str = ""
) -> list:
    """Every way `text` reaches into the journal: a link, an embed, a tag, or a verbatim run of
    `journal.window` words. Findings name lines only, never what the journal says."""
    findings = []
    wrapped = not text.startswith("---\n")
    note = parse_note(Path("text.md"), "---\n---\n" + text if wrapped else text)
    offset = first_line - (3 if wrapped else 1)
    for link in note.links:
        if link.scheme:
            continue
        resolved = None
        if resolver is not None:
            resolved, _problem = resolver.resolve(link.target, source)
        if journal.holds(link.target) or (
            resolved is not None and journal.holds(resolved)
        ):
            what = "an embed" if link.embed else "a link"
            findings.append(f"line {link.line + offset}: {what} into the journal")
    tags = list(note.tags)
    listed = note.frontmatter.get("tags")
    if isinstance(listed, list):
        line = next(
            (
                n
                for n, raw in enumerate(text.split("\n"), start=1)
                if raw.startswith("tags:")
            ),
            1,
        )
        tags += [(line, tag) for tag in listed if isinstance(tag, str)]
    for line, tag in tags:
        if any(
            in_journal(part, journal.words, ()) for part in tag.lstrip("#").split("/")
        ):
            findings.append(f"line {line + offset}: a tag into the journal")
    if journal.readable and journal.shingles:
        for line in repeated_lines(text, journal.shingles, journal.window, first_line):
            findings.append(
                f"line {line}: repeats {journal.window} consecutive words of a journal note"
            )
    return findings


# --- persona-core's public API, loaded, never copied ----------------------------------------------

_PERSONA_CACHE: dict = {}


def _persona_core(path: Path) -> object:
    key = str(Path(path).resolve())
    if key not in _PERSONA_CACHE:
        if not Path(path).is_file():
            raise ContractError(f"persona-core's probe is not at {path}")
        spec = importlib.util.spec_from_file_location(
            "persona_core_probe_for_vault_duties", path
        )
        if spec is None or spec.loader is None:
            raise ContractError(f"persona-core's probe at {path} cannot be loaded")
        module = importlib.util.module_from_spec(spec)
        try:
            spec.loader.exec_module(module)
        except (
            Exception
        ) as error:  # a vendored copy that does not load is VOID, never a pass
            raise ContractError(
                f"persona-core's probe does not load ({error})"
            ) from error
        _PERSONA_CACHE[key] = module
    return _PERSONA_CACHE[key]


def _date_scanner(path: Path) -> tuple:
    """(scan, rows): persona-core's per-line NFKC scanner and its date and timeline rows."""
    module = _persona_core(path)
    try:
        patterns = module.load_patterns()
    except ValueError as error:
        raise ContractError(f"persona-core's patterns do not load ({error})") from error
    return module.scan, patterns["dates"] + patterns["timelines"]


# --- the context every class reads ------------------------------------------------------------------


@dataclass
class Context:
    root: Path
    runs: list
    texts: list
    contract: dict
    rails: dict
    pack_dir: Path
    persona_core: Path
    deny_list: Path | None
    vault_override: Path | None
    layout_override: dict | None

    def show(self, path: Path) -> str:
        try:
            return Path(path).resolve().relative_to(self.root.resolve()).as_posix()
        except ValueError:
            return str(path)

    def where(self, run: Run, vault_path: str, line: "int | None" = None) -> str:
        shown = self.show(run.directory / vault_path)
        return f"{shown}:{line}" if line else shown

    def good_runs(self) -> list:
        return [run for run in self.runs if run.fatal is None]

    def duty(self, duty_id: str) -> "dict | None":
        return next((d for d in self.contract["duties"] if d["id"] == duty_id), None)

    def vault_notes(self) -> list:
        """(run, agent note) for every note claimed as a vault-duty note."""
        return [
            (run, agent)
            for run in self.good_runs()
            for agent in run.notes
            if agent.note.claimed_by([NOTE_SCHEMA.rsplit(".", 2)[0] + "."])
        ]

    def agent_notes(self) -> list:
        return [(run, agent) for run in self.good_runs() for agent in run.notes]


@dataclass(frozen=True)
class Outcome:
    examined: int
    findings: tuple


def _run_findings(ctx: Context, run: Run) -> list:
    where = ctx.show(run.record)
    if run.fatal is not None:
        return [f"{where}: {run.fatal}"]
    return [f"{where}: {problem}" for problem in run.problems]


# --- the classes -----------------------------------------------------------------------------------


def _tag_problem(tag: object) -> "str | None":
    if not isinstance(tag, str) or not tag:
        return f"tag {tag!r} is not a non-empty string"
    if tag.startswith("#"):
        return f"tag {tag!r} starts with #: a tags property holds the bare tag"
    if any(char.isspace() for char in tag):
        return f"tag {tag!r} holds a space"
    if any(char in "!\"$%&'()*+,.:;<=>?@[]^`{|}~\\#" for char in tag):
        return f"tag {tag!r} holds punctuation Obsidian does not accept in a tag"
    if all(char.isdigit() or char == "/" for char in tag):
        return f"tag {tag!r} holds no non-numerical character"
    if "//" in tag or tag.startswith("/") or tag.endswith("/"):
        return f"tag {tag!r} has an empty nested part"
    return None


def _property_problems(agent: AgentNote, run: Run, ctx: Context) -> list:
    note = agent.note
    contract = ctx.contract
    if note.fatal == "line 1 is not ---":
        return [
            "line 1 is not ---: the note has no properties, so no agent schema marks it"
        ]
    if note.fatal is not None:
        return [note.fatal]
    frontmatter = note.frontmatter
    problems = []
    if not note.claimed_by(contract["agent_claims"]):
        return ["the note carries no agent schema, so nothing marks it as the agent's"]
    keys = contract["note_keys"]
    deprecated = contract["deprecated_keys"]
    for key, value in frontmatter.items():
        if key in deprecated:
            problems.append(
                f"{key} is deprecated (Obsidian 1.4) and dropped (Obsidian 1.9): use {deprecated[key]}"
            )
        elif key not in keys and not EXTENSION_KEY.match(key):
            problems.append(f"unknown key {key}")
        if isinstance(value, dict) or (
            isinstance(value, list)
            and any(isinstance(item, (dict, list)) for item in value)
        ):
            problems.append(
                f"{key} holds a nested value, which Obsidian's Properties cannot show"
            )
    for key in contract["note_required"]:
        if key not in frontmatter:
            problems.append(f"{key} is missing")
    duty = frontmatter.get("duty")
    registry = {
        entry["id"]: entry for entry in contract["duties"] if entry.get("writes_notes")
    }
    if "duty" in frontmatter and duty not in registry:
        problems.append(f"duty {duty} is not a vault duty that writes notes")
    elif isinstance(duty, str) and duty != run.duty:
        problems.append(f"duty {duty} is not the run's {run.duty}")
    for required in (registry.get(duty) or {}).get("requires", []):
        if required not in frontmatter:
            problems.append(f"{required} is missing")
    for key, kind in keys.items():
        if key not in frontmatter:
            continue
        value = frontmatter[key]
        if kind == "text" and not isinstance(value, str):
            problems.append(f"{key} is not text")
        elif kind in ("list", "tags", "links") and not isinstance(value, list):
            problems.append(f"{key} is not a list")
        elif kind == "list" and not _strings(value):
            problems.append(f"{key} holds a value that is not a string")
        elif kind == "tags":
            problems += [p for p in (_tag_problem(tag) for tag in value) if p]
        elif kind == "links":
            if not value:
                problems.append(f"{key} is empty")
            for item in value:
                if not isinstance(item, str) or not re.fullmatch(
                    r"\[\[[^\[\]\n]+\]\]", item
                ):
                    problems.append(
                        f"{key} holds {item!r}, which is not a [[wikilink]]"
                    )
    if "sources" in frontmatter and "sources" not in (registry.get(duty) or {}).get(
        "requires", []
    ):
        problems.append("sources is set, but only a synthesis cites notes")
    problems += list(note.problems)
    for match in TEMPLATE_VARIABLE.finditer(note.text):
        problems.append(f"an unfilled template variable {match.group(0)}")
    return problems


def check_note_properties(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for run, agent in ctx.agent_notes():
        if agent.note.claimed_by(["phx.persona."]):
            continue
        examined += 1
        where = ctx.where(run, agent.vault_path)
        findings += [
            f"{where}: {problem}" for problem in _property_problems(agent, run, ctx)
        ]
    return Outcome(examined, tuple(findings))


def _text_of(run: Run, vault_path: str) -> "str | None":
    staged = run.staged.get(vault_path)
    if staged is None:
        for path, full in run.staged.items():
            if path.casefold() == vault_path.casefold():
                staged = full
    try:
        if staged is not None:
            return staged.read_text(encoding="utf-8", errors="replace")
        if run.vault_dir is not None and (run.vault_dir / vault_path).is_file():
            return (run.vault_dir / vault_path).read_text(
                encoding="utf-8", errors="replace"
            )
    except OSError:
        return None
    return None


def _heading_key(text: str) -> str:
    text = HTML_COMMENT.sub(" ", OBSIDIAN_COMMENT.sub(" ", text))
    return " ".join(HEADING_NOISE.sub(" ", text).split()).casefold()


def _subpath_problem(subpath: str, text: str) -> "str | None":
    if not subpath or subpath == "#":
        return None
    if subpath.startswith("#^"):
        block = subpath[2:]
        for line in text.split("\n"):
            match = BLOCK_ID.search(line)
            if match and match["id"] == block:
                return None
        return f"points at a note that has no block ^{block}"
    wanted = _heading_key(subpath.split("#")[-1])
    for line in text.split("\n"):
        heading = HEADING.match(line)
        if heading and _heading_key(heading["content"] or "") == wanted:
            return None
    return "points at a note that has no heading of that name"


def check_note_links(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for run, agent in ctx.agent_notes():
        examined += 1
        for link in agent.note.links:
            if link.scheme or link.form == "autolink":
                continue
            where = ctx.where(run, agent.vault_path, link.line)
            resolved, problem = run.resolver.resolve(link.target, agent.vault_path)
            if problem == "ambiguous":
                findings.append(
                    f"{where}: {link.text} is ambiguous: more than one note has that name; "
                    "link by path"
                )
                continue
            if resolved is None:
                findings.append(f"{where}: {link.text} resolves to no note or file")
                continue
            text = (
                agent.note.text
                if resolved == agent.vault_path
                else _text_of(run, resolved)
            )
            if text is not None:
                problem = _subpath_problem(link.subpath, text)
                if problem:
                    findings.append(f"{where}: {link.text} {problem}")
    return Outcome(examined, tuple(findings))


def check_note_names(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for run in ctx.good_runs():
        for op in run.ops:
            if op.op not in ("create", "update", "move") or op.path is None:
                continue
            examined += 1
            where = f"{ctx.show(run.record)}: op {op.index} {op.path}"
            problems = name_problems(op.path)
            if op.op in ("create", "update") and not op.path.endswith(".md"):
                problems.append("is not a .md note")
            if op.op == "move" and op.source is not None:
                if op.source.rsplit("/", 1)[-1] != op.path.rsplit("/", 1)[-1]:
                    problems.append(
                        "renames the capture: a filed capture keeps its name, so every link to "
                        "it still resolves"
                    )
            layout = run.layout
            writes = ((layout.get("duties") or {}).get(run.duty) or {}).get(
                "writes", []
            )
            periodic_kinds = [
                entry[1:] for entry in writes if entry in ("@daily", "@weekly")
            ]
            rule = _folder_rule(op.path, layout)
            if periodic_kinds and op.op in ("create", "update") and rule is None:
                reasons = [
                    periodic_problem(op.path, layout, kind) for kind in periodic_kinds
                ]
                if all(reasons):
                    problems += [f"{reason}" for reason in reasons]
            elif rule is not None:
                folder, spec = rule
                stem = op.path.rsplit("/", 1)[-1]
                stem = stem[: -len(".md")] if stem.endswith(".md") else stem
                if isinstance(spec, dict) and isinstance(spec.get("regex"), str):
                    if re.fullmatch(spec["regex"], stem) is None:
                        problems.append(
                            f"does not fit {folder}'s name rule {spec['regex']}"
                        )
            findings += [f"{where}: {problem}" for problem in problems]
    return Outcome(examined, tuple(findings))


def check_write_confinement(ctx: Context) -> Outcome:
    findings: list = []
    for run in ctx.runs:
        findings += _run_findings(ctx, run)
        if run.fatal is not None:
            continue
        where = ctx.show(run.record)
        findings += [f"{where}: layout: {problem}" for problem in run.layout_problems]
        rules = (run.layout.get("duties") or {}).get(run.duty)
        journal = [f for f in run.layout.get("journal", []) if isinstance(f, str)]
        if not isinstance(rules, dict):
            findings.append(
                f"{where}: {run.duty} has no folders in the layout, so it may not write"
            )
            rules = {}
        recorded: set = set()
        for op in run.ops:
            if op.path is None:
                findings.append(f"{where}: op {op.index} {op.op} names no path")
                continue
            recorded.add(op.path)
            label = f"{where}: op {op.index} {op.path}"
            findings += [f"{label} {problem}" for problem in path_problems(op.path)]
            if any(_under(op.path, folder) for folder in journal):
                findings.append(f"{label} writes into the journal")
            if op.op == "move":
                if not rules.get("moves_from") and not rules.get("moves_to"):
                    findings.append(f"{label}: {run.duty} may not move a file")
                    continue
                source = op.source or ""
                findings += [
                    f"{label} from {problem}" for problem in path_problems(source)
                ]
                if not any(_under(source, f) for f in rules.get("moves_from", [])):
                    findings.append(
                        f"{label} moves from outside {run.duty}'s moves_from"
                    )
                if not any(
                    _under(op.path, f) and op.path.casefold() != f.casefold()
                    for f in rules.get("moves_to", [])
                ):
                    findings.append(f"{label} moves to outside {run.duty}'s moves_to")
                if _under(op.path, str(run.layout.get("inbox", ""))) and run.layout.get(
                    "inbox"
                ):
                    findings.append(
                        f"{label} moves into the inbox, where nothing is ever put back"
                    )
            elif op.op in ("create", "update"):
                if not op.path.endswith(".md"):
                    findings.append(f"{label} is not a .md note")
                if not _write_allowed(op.path, rules.get("writes", []), run.layout):
                    findings.append(f"{label} is outside {run.duty}'s folders")
            if op.path not in run.staged:
                findings.append(f"{label} has no staged file")
        for path in sorted(run.staged):
            if path not in recorded:
                findings.append(f"{where}: {path} is staged but no op writes it")
    return Outcome(len(ctx.runs), tuple(findings))


def _fence_reason(info: str, rails: dict) -> "str | None":
    language = info.split()[0].casefold() if info.split() else ""
    if language in rails["fence_allow"]:
        return None
    if language in rails["fence_known"]:
        return f"a ```{language} fence: {rails['fence_known'][language]}"
    for prefix, what in rails["fence_prefixes"].items():
        if language.startswith(prefix):
            return f"a ```{language} fence: {what}"
    return f"a ```{language} fence, off the allow-list: code a plugin may run or render live"


def _executable_problems(note: Note, rails: dict) -> list:
    problems = []
    for line, info in note.fences:
        reason = _fence_reason(info, rails)
        if reason:
            problems.append((line, reason))
    opener = rails["templater_open"]
    for offset, line in enumerate(note.text.split("\n")):
        if opener in line:
            problems.append(
                (offset + 1, f"a Templater command ({opener}), which runs on creation")
            )
        if "obsidian://" in line.casefold():
            problems.append(
                (offset + 1, "an obsidian: URI, which opens, appends or overwrites")
            )
    prefixes = sorted(rails["inline_query_prefixes"], key=len, reverse=True)
    for line, code in note.code_spans:
        stripped = code.strip()
        if any(stripped.startswith(prefix) for prefix in prefixes):
            problems.append(
                (line, "a Dataview inline query, which renders other notes live")
            )
    allowed_tags = set(rails["html_allow"])
    allowed_attributes = set(rails["html_attributes_allow"])
    refused = set(rails["executable_schemes"])
    for offset, line in enumerate(note.prose):
        number = note.body_line + offset
        for match in HTML_TAG.finditer(line):
            name = match["name"].casefold()
            if name not in allowed_tags:
                problems.append((number, f"HTML <{name}> is off the allow-list"))
            for attribute in HTML_ATTRIBUTE.finditer(match["attrs"] or ""):
                attr = attribute["name"].casefold()
                if attr not in allowed_attributes:
                    problems.append((number, f"HTML attribute {attr} on <{name}>"))
        for macro in rails["math_macros_refused"]:
            if re.search(rf"\\{re.escape(macro)}\s*\{{", line):
                problems.append(
                    (number, f"the MathJax \\{macro} macro, a link inside math")
                )
    for link in note.links:
        if link.scheme in refused:
            problems.append(
                (link.line, f"a {link.scheme}: link, which acts when it is opened")
            )
        extension = posixpath.splitext(link.target)[1].casefold()
        if link.embed and extension in rails["dynamic_embed_extensions"]:
            problems.append(
                (
                    link.line,
                    f"an embedded {extension} view renders and edits other notes",
                )
            )
    return problems


def check_no_executable(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for run, agent in ctx.agent_notes():
        examined += 1
        for line, problem in sorted(_executable_problems(agent.note, ctx.rails)):
            findings.append(f"{ctx.where(run, agent.vault_path, line)}: {problem}")
    return Outcome(examined, tuple(findings))


def _sha256(path: Path) -> "str | None":
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def check_never_deletes(ctx: Context) -> Outcome:
    findings: list = []
    grammar = set(ctx.contract["ops"])
    marks = ctx.contract["agent_claims"]
    for run in ctx.runs:
        findings += _run_findings(ctx, run)
        if run.fatal is not None:
            continue
        where = ctx.show(run.record)
        existing = {path.casefold(): path for path in run.index}
        written: dict = {}
        moved_from: dict = {}
        snapshot = {path: digest for path, digest in run.inbox}
        for op in run.ops:
            label = f"{where}: op {op.index}"
            if op.op not in grammar:
                findings.append(
                    f"{label}: op {op.op} is not in the grammar ({', '.join(sorted(grammar))}): "
                    "nothing is ever deleted, renamed or overwritten"
                )
                continue
            if op.path is not None:
                key = op.path.casefold()
                if key in written:
                    findings.append(
                        f"{label}: two ops write {op.path} (op {written[key]} too)"
                    )
                written.setdefault(key, op.index)
            if op.op in ("create", "move") and op.path is not None:
                clash = existing.get(op.path.casefold())
                if clash is not None and not (op.op == "move" and clash == op.source):
                    findings.append(f"{label}: {op.op} would overwrite {clash}")
            if op.op == "update":
                findings += [
                    f"{label}: {problem}"
                    for problem in _update_problems(op, run, marks)
                ]
            if op.op == "move" and op.source is not None:
                moved_from.setdefault(op.source, 0)
                moved_from[op.source] += 1
                findings += [
                    f"{label}: {problem}"
                    for problem in _move_problems(op, run, snapshot)
                ]
        for source, count in moved_from.items():
            if count > 1:
                findings.append(f"{where}: {source} is moved twice")
        inbox = str(run.layout.get("inbox", "")).strip("/")
        if run.duty in (run.layout.get("duties") or {}) and any(
            (run.layout["duties"][run.duty] or {}).get(key)
            for key in ("moves_from", "moves_to")
        ):
            for path in run.index:
                if inbox and _under(path, inbox) and path not in snapshot:
                    findings.append(
                        f"{where}: {path} is in the inbox but not in the snapshot"
                    )
            for path in snapshot:
                if path not in moved_from and path not in run.index:
                    findings.append(
                        f"{where}: {path} disappeared: it is neither filed nor kept"
                    )
    return Outcome(len(ctx.runs), tuple(findings))


def _update_problems(op: Op, run: Run, marks: Iterable) -> list:
    if op.path not in run.index:
        return [f"update {op.path} names no existing note"]
    if op.sha256_before is None:
        return [
            f"update {op.path} carries no sha256_before, so the owner's edits are unprotected"
        ]
    if run.vault_dir is None:
        return [
            f"update {op.path}: no vault is readable to prove it is the agent's own note"
        ]
    current = run.vault_dir / op.path
    digest = _sha256(current)
    if digest is None:
        return [f"update {op.path}: the vault's copy is unreadable"]
    if digest != op.sha256_before:
        return [f"update {op.path}: the note changed since the agent wrote it"]
    if not parse_note(current).claimed_by(marks):
        return [f"update {op.path}: it is not an agent note"]
    return []


def _move_problems(op: Op, run: Run, snapshot: dict) -> list:
    problems = []
    if op.source not in run.index:
        problems.append(f"move from {op.source}, which is not in the vault")
    staged = run.staged.get(op.path or "")
    digest = _sha256(staged) if staged is not None else None
    if op.sha256 is None:
        problems.append("the move carries no sha256")
    elif digest is not None and digest != op.sha256:
        problems.append("the filed capture's bytes changed on the way")
    if op.sha256 is not None and snapshot.get(op.source) != op.sha256:
        problems.append("the move's sha256 is not the inbox snapshot's")
    if (
        run.vault_dir is not None
        and op.source
        and (run.vault_dir / op.source).is_file()
    ):
        if _sha256(run.vault_dir / op.source) != snapshot.get(op.source):
            problems.append("the snapshot's sha256 is not the vault's")
    return problems


def _linked_paths(
    run: Run, note: Note, vault_path: str, section: "Section | None" = None
) -> list:
    found = []
    for link in note.links:
        if link.scheme or link.form in ("autolink", "property"):
            continue
        if section is not None and not section.first <= link.line <= section.last:
            continue
        resolved, _problem = run.resolver.resolve(link.target, vault_path)
        if resolved is not None:
            found.append((link, resolved))
    return found


def check_daily_note(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    duty = ctx.duty("daily-note") or {"sections": []}
    for run, agent in ctx.vault_notes():
        if run.duty != "daily-note":
            continue
        examined += 1
        note = agent.note
        where = ctx.where(run, agent.vault_path)
        for section_id in duty["sections"]:
            section = note.section(section_id)
            if section is None:
                findings.append(f"{where}: section {section_id} is missing")
            elif not section.body.strip():
                findings.append(f"{where}: section {section_id} is empty")
        for path, section_id in run.inputs:
            if section_id is None:
                continue
            section = note.section(section_id)
            if section_id not in duty["sections"] and section is None:
                findings.append(
                    f"{where}: input {path} names section {section_id}, which a daily note does "
                    "not have"
                )
                continue
            linked = (
                {
                    resolved.casefold()
                    for _l, resolved in _linked_paths(
                        run, note, agent.vault_path, section
                    )
                }
                if section
                else set()
            )
            if path.casefold() not in linked:
                findings.append(
                    f"{where}: input {path} is not linked in section {section_id}"
                )
    return Outcome(examined, tuple(findings))


def _claim_blocks(note: Note, section: Section) -> list:
    """(first line, last line, text) for each list item and paragraph of a section."""
    blocks: list = []
    current: "list | None" = None
    for number in range(section.first, section.last + 1):
        line = note.prose[number - note.body_line]
        if HEADING.match(line):
            current = None
            continue
        stripped = BLOCKQUOTE.sub("", line).strip()
        if not stripped:
            current = None
            continue
        item = LIST_ITEM.match(line)
        if item is not None or current is None:
            current = [number, number, (item["text"] if item else stripped).strip()]
            blocks.append(current)
        else:
            current[1] = number
            current[2] += " " + stripped
    return [tuple(block) for block in blocks]


def _sources(run: Run, note: Note, vault_path: str) -> tuple:
    declared = note.frontmatter.get("sources")
    items = declared if isinstance(declared, list) else []
    resolved: dict = {}
    problems: list = []
    for item in items:
        if not isinstance(item, str):
            continue
        match = WIKILINK.fullmatch(item.strip())
        target = _split_target(match["inner"].partition("|")[0])[0] if match else item
        path, _problem = run.resolver.resolve(target, vault_path)
        if path is None:
            problems.append(f"source {item} resolves to no note")
        else:
            resolved[path] = item
    return resolved, problems


def check_synthesis_cites(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    duty = ctx.duty("weekly-synthesis") or {"sections": [], "claims": []}
    connection = ctx.contract.get("connection_section")
    for run, agent in ctx.vault_notes():
        if run.duty != "weekly-synthesis":
            continue
        examined += 1
        note = agent.note
        where = ctx.where(run, agent.vault_path)
        sources, problems = _sources(run, note, agent.vault_path)
        findings += [f"{where}: {problem}" for problem in problems]
        if not sources and not problems:
            findings.append(
                f"{where}: it declares no sources, so no claim can cite one"
            )
        inputs = {path.casefold() for path, _section in run.inputs}
        for path, item in sources.items():
            if path.casefold() not in inputs:
                findings.append(f"{where}: {item} is not one of the run's inputs")
        for section_id in duty["sections"]:
            section = note.section(section_id)
            if section is None:
                findings.append(f"{where}: section {section_id} is missing")
            elif not section.body.strip():
                findings.append(f"{where}: section {section_id} is empty")
        cited: set = set()
        for section_id in duty["claims"]:
            section = note.section(section_id)
            if section is None:
                continue
            links = _linked_paths(run, note, agent.vault_path, section)
            for first, last, _text in _claim_blocks(note, section):
                block = [
                    (link, path) for link, path in links if first <= link.line <= last
                ]
                hits = {path for _link, path in block if path in sources}
                for link, path in block:
                    if path not in sources:
                        findings.append(
                            f"{where}:{link.line}: cites {link.text}, which is not one of its sources"
                        )
                if not hits:
                    findings.append(f"{where}:{first}: a claim cites no source")
                if section_id == connection and len(hits) < 2 and hits:
                    findings.append(
                        f"{where}:{first}: a connection names fewer than two notes"
                    )
                cited |= hits
        for path, item in sources.items():
            if path not in cited:
                findings.append(f"{where}: source {item} is never cited by a claim")
    return Outcome(examined, tuple(findings))


def _journal_for(ctx: Context, run: "Run | None", cache: dict) -> Journal:
    vault = ctx.vault_override if run is None else run.vault_dir
    layout = ctx.layout_override if run is None else run.layout
    index = None if run is None else tuple(run.index)
    key = (str(vault), json.dumps(layout, sort_keys=True), index)
    if key not in cache:
        cache[key] = load_journal(
            vault,
            layout=layout,
            deny_list=ctx.deny_list,
            index=index,
            persona_core=ctx.persona_core,
            window=ctx.contract["quote_window"],
        )
    return cache[key]


def check_journal_never_leaks(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    cache: dict = {}
    for run in ctx.good_runs():
        journal = _journal_for(ctx, run, cache)
        where = ctx.show(run.record)
        for position, (path, _section) in enumerate(run.inputs):
            if journal.holds(path):
                findings.append(
                    f"{where}: the journal was an input (input {position + 1})"
                )
        if run.notes and journal.notes and (not journal.readable or journal.unreadable):
            findings.append(
                f"{where}: the journal holds {journal.notes} note(s) but no vault is readable to "
                "prove no text of it is quoted"
            )
        for agent in run.notes:
            examined += 1
            for finding in journal_leaks(
                agent.note.text, journal, resolver=run.resolver, source=agent.vault_path
            ):
                line, _sep, what = finding.partition(": ")
                findings.append(
                    f"{ctx.where(run, agent.vault_path, int(line.split()[1]))}: {what}"
                )
    if ctx.texts:
        journal = _journal_for(ctx, None, cache)
        for path in ctx.texts:
            examined += 1
            text = path.read_text(encoding="utf-8", errors="replace")
            for finding in journal_leaks(text, journal):
                line, _sep, what = finding.partition(": ")
                findings.append(f"{ctx.show(path)}:{line.split()[1]}: {what}")
    return Outcome(examined, tuple(findings))


def _remote(url: str, rails: dict) -> bool:
    scheme = SCHEME.match(url.strip())
    return scheme is not None and scheme["scheme"].casefold() in rails["remote_schemes"]


def check_nothing_leaves(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    rails = ctx.rails
    for run, agent in ctx.agent_notes():
        examined += 1
        note = agent.note
        problems: list = []
        publish = note.frontmatter.get(rails["publish_key"])
        if publish not in (None, False):
            problems.append((2, "publish: true sends the note to Obsidian Publish"))
        for link in note.links:
            if link.embed and _remote(link.target, rails):
                problems.append(
                    (
                        link.line,
                        "an embed fetched from outside the vault when it renders",
                    )
                )
            if link.scheme in rails["egress_schemes"]:
                problems.append(
                    (link.line, f"a {link.scheme}: link names a path on a machine")
                )
        for offset, line in enumerate(note.prose):
            number = note.body_line + offset
            for match in HTML_TAG.finditer(line):
                for attribute in HTML_ATTRIBUTE.finditer(match["attrs"] or ""):
                    value = (attribute["value"] or "").strip("\"'")
                    urls = [value] + [m["url"] for m in CSS_URL.finditer(value)]
                    if attribute["name"].casefold() in (
                        "src",
                        "srcset",
                        "poster",
                        "data",
                        "style",
                        "background",
                    ):
                        if any(_remote(url, rails) for url in urls):
                            problems.append(
                                (
                                    number,
                                    "HTML fetched from outside the vault when it renders",
                                )
                            )
                    if any(url.casefold().startswith("file:") for url in urls):
                        problems.append(
                            (number, "a file: URL names a path on a machine")
                        )
        for line, problem in sorted(set(problems)):
            findings.append(f"{ctx.where(run, agent.vault_path, line)}: {problem}")
    return Outcome(examined, tuple(findings))


def _masked_for_dates(note: Note, scan, rows) -> tuple:
    """The note's text with every link's TARGET blanked, since a reader sees only its alias; and
    the lines where a link to a date-named note shows that date because it carries no alias."""
    missing: list = []

    def wikilink(match: "re.Match") -> str:
        target_part, bar, alias = match["inner"].partition("|")
        if not bar and scan(target_part, rows):
            missing.append(match.group(0))
        return match["embed"] + "[[" + " " * len(target_part) + bar + alias + "]]"

    def markdown(match: "re.Match") -> str:
        return match.group(0).replace(match["dest"], " " * len(match["dest"]), 1)

    lines = []
    alias_lines = []
    for number, line in enumerate(note.text.split("\n"), start=1):
        before = len(missing)
        masked = MARKDOWN_LINK.sub(markdown, WIKILINK.sub(wikilink, line))
        if len(missing) > before:
            alias_lines.append(number)
        lines.append(masked)
    return "\n".join(lines), alias_lines


def check_no_dates(ctx: Context) -> Outcome:
    scan, rows = _date_scanner(ctx.persona_core)
    findings: list = []
    notes = ctx.vault_notes()
    for run, agent in notes:
        masked, alias_lines = _masked_for_dates(agent.note, scan, rows)
        for line in alias_lines:
            findings.append(
                f"{ctx.where(run, agent.vault_path, line)}: a link to a date-named note shows the "
                "date: give it a date-free alias"
            )
        for line, row_id, _found in scan(masked, rows):
            findings.append(f"{ctx.where(run, agent.vault_path, line)}: {row_id}")
    return Outcome(len(notes), tuple(findings))


def _sentences(text: str) -> list:
    parts = [part.strip() for part in SENTENCE_END.split(text) if part and part.strip()]
    merged: list = []
    for part in parts:
        if merged and LINK_ONLY.match(part):
            merged[-1] += " " + part
        else:
            merged.append(part)
    return merged


def check_claim_sentences(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    duty = ctx.duty("weekly-synthesis") or {"claims": []}
    for run, agent in ctx.vault_notes():
        if run.duty != "weekly-synthesis":
            continue
        examined += 1
        for section_id in duty["claims"]:
            section = agent.note.section(section_id)
            if section is None:
                continue
            for first, _last, text in _claim_blocks(agent.note, section):
                for sentence in _sentences(text):
                    if not (
                        WIKILINK.search(sentence) or MARKDOWN_LINK.search(sentence)
                    ):
                        findings.append(
                            f"{ctx.where(run, agent.vault_path, first)}: a sentence of this claim "
                            "carries no link of its own"
                        )
    return Outcome(examined, tuple(findings))


def check_synthesis_own_words(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    window = ctx.contract["quote_window"]
    for run, agent in ctx.vault_notes():
        if run.duty != "weekly-synthesis":
            continue
        sources, _problems = _sources(run, agent.note, agent.vault_path)
        readable = [(path, item, _text_of(run, path)) for path, item in sources.items()]
        readable = [
            (path, item, text) for path, item, text in readable if text is not None
        ]
        if not readable:
            continue
        examined += 1
        body = agent.note.body
        for _path, item, text in readable:
            known = shingles(text, window)
            lines = repeated_lines(body, known, window, agent.note.body_line)
            if lines:
                findings.append(
                    f"{ctx.where(run, agent.vault_path, lines[0])}: repeats {window} consecutive "
                    f"words of its source {item}: a synthesis says it in its own words"
                )
    return Outcome(examined, tuple(findings))


def check_orphan_notes(ctx: Context) -> Outcome:
    linked: set = set()
    for run, agent in ctx.agent_notes():
        for _link, resolved in _linked_paths(run, agent.note, agent.vault_path):
            if resolved != agent.vault_path:
                linked.add(resolved.casefold())
    findings: list = []
    examined = 0
    for run in ctx.good_runs():
        for op in run.ops:
            if op.path is None or op.op not in ("create", "move"):
                continue
            if op.op == "create" and any(
                periodic_problem(op.path, run.layout, kind) is None
                for kind in ("daily", "weekly")
            ):
                continue
            examined += 1
            if op.path.casefold() not in linked:
                findings.append(
                    f"{ctx.show(run.record)}: {op.path} is linked from no note, so it is an orphan"
                )
    return Outcome(examined, tuple(findings))


def check_inbox_zero(ctx: Context) -> Outcome:
    findings: list = []
    examined = 0
    for run in ctx.good_runs():
        rules = (run.layout.get("duties") or {}).get(run.duty) or {}
        if not rules.get("moves_from"):
            continue
        examined += 1
        moved = {op.source for op in run.ops if op.op == "move"}
        left = [path for path, _digest in run.inbox if path not in moved]
        if left:
            plural = "" if len(left) == 1 else "s"
            findings.append(
                f"{ctx.show(run.record)}: {len(left)} capture{plural} left in "
                f"{run.layout.get('inbox')}"
            )
    return Outcome(examined, tuple(findings))


CHECKS = {
    "note-properties": check_note_properties,
    "note-links": check_note_links,
    "note-names": check_note_names,
    "write-confinement": check_write_confinement,
    "no-executable": check_no_executable,
    "never-deletes": check_never_deletes,
    "daily-note": check_daily_note,
    "synthesis-cites": check_synthesis_cites,
    "journal-never-leaks": check_journal_never_leaks,
    "nothing-leaves": check_nothing_leaves,
    "no-dates": check_no_dates,
    "claim-sentences": check_claim_sentences,
    "synthesis-own-words": check_synthesis_own_words,
    "orphan-notes": check_orphan_notes,
    "inbox-zero": check_inbox_zero,
}


# --- discovery and the command line ---------------------------------------------------------------


def discover(bases: Sequence) -> tuple:
    """(run records, bare texts): the records under `bases`, and the text files of every base
    that holds no record. Dot-directories, node_modules and target are never walked."""
    records: dict = {}
    texts: list = []
    for base in bases:
        base = Path(base)
        found: list = []
        if base.is_file():
            if base.name == RUN_FILE:
                found.append(base)
            elif base.suffix in TEXT_SUFFIXES:
                texts.append(base)
            for record in found:
                records[record.resolve()] = record
            continue
        candidates: list = []
        for folder, subfolders, files in os.walk(base):
            subfolders[:] = sorted(
                name
                for name in subfolders
                if not name.startswith(".") and name not in SKIPPED_DIRECTORIES
            )
            for name in sorted(files):
                path = Path(folder) / name
                if name == RUN_FILE:
                    found.append(path)
                elif name.endswith(TEXT_SUFFIXES):
                    candidates.append(path)
        for record in found:
            records[record.resolve()] = record
        if not found:
            texts += candidates
    return [records[key] for key in sorted(records)], texts


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="vault-duties-probe.py",
        description="Judge the staged output of DeckStreak's vault duties before it reaches the vault.",
    )
    parser.add_argument("--root", default=".", help="the tree to judge (default: .)")
    parser.add_argument(
        "--subject",
        action="append",
        default=[],
        help="a run directory, a duty-run.json, or a directory of texts; repeatable",
    )
    parser.add_argument(
        "--vault", help="the vault, for headings, blocks, the journal and updates"
    )
    parser.add_argument(
        "--layout",
        help=f"a private layout that replaces every run's (default: ${LAYOUT_ENV})",
    )
    parser.add_argument(
        "--deny-list", help=f"persona-core's private deny-list (default: ${DENY_ENV})"
    )
    parser.add_argument("--pack-dir", help="the vault-duties pack directory")
    parser.add_argument(
        "--persona-core", help="persona-core's probe (default: beside this script)"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", choices=CLASSES)
    parse = commands.add_parser(
        "parse", help="print one note or one duty-run.json as JSON"
    )
    parse.add_argument("file")
    commands.add_parser("classes", help="list the classes")
    return parser


def _usage(message: str) -> int:
    print(f"vault-duties-probe: {message}", file=sys.stderr)
    return EXIT_USAGE


def _parse_command(path: str) -> int:
    target = Path(path)
    if target.name == RUN_FILE or target.suffix == ".json":
        run = load_run(target)
        if run.fatal is not None:
            print(f"parse: {path}: {run.fatal}")
            return EXIT_FINDING
        print(
            json.dumps(
                {
                    "path": str(run.record),
                    "duty": run.duty,
                    "vault": run.index,
                    "inputs": [{"path": p, "section": s} for p, s in run.inputs],
                    "ops": [op.__dict__ for op in run.ops],
                    "staged": sorted(run.staged),
                    "problems": run.problems + run.layout_problems,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return EXIT_GREEN
    note = parse_note(target)
    if note.fatal is not None:
        print(f"parse: {path}: {note.fatal}")
        return EXIT_FINDING
    print(
        json.dumps(
            {
                "path": str(note.path),
                "frontmatter": note.frontmatter,
                "sections": [
                    {"id": s.id, "title": s.title, "line": s.line, "body": s.body}
                    for s in note.sections
                ],
                "links": [link.__dict__ for link in note.links],
                "tags": [{"line": line, "tag": tag} for line, tag in note.tags],
                "body_line": note.body_line,
                "problems": list(note.problems),
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
    vault = Path(args.vault) if args.vault else None
    if vault is not None and not vault.is_dir():
        return _usage(f"--vault is not a directory: {args.vault}")
    pack_dir = Path(args.pack_dir) if args.pack_dir else DEFAULT_PACK_DIR
    persona_core = (
        Path(args.persona_core) if args.persona_core else DEFAULT_PERSONA_CORE
    )
    private_deny = args.deny_list or os.environ.get(DENY_ENV) or None
    layout_path = args.layout or os.environ.get(LAYOUT_ENV) or None
    try:
        contract = load_contract(pack_dir)
        rails = load_rails(pack_dir)
        load_layout(pack_dir / "layout.json")
        layout_override = load_layout(layout_path) if layout_path else None
        if name in ("no-dates",):
            _date_scanner(persona_core)
        if name in ("journal-never-leaks",):
            _journal_words(persona_core, Path(private_deny) if private_deny else None)
    except (ContractError, ValueError) as error:
        print(f"{name}: VOID: {error}")
        print("examined 0")
        return EXIT_VOID
    records, texts = discover(bases)
    runs = [
        load_run(
            record,
            layout_override=layout_override,
            vault_override=vault,
            pack_dir=pack_dir,
        )
        for record in records
    ]
    ctx = Context(
        root=root,
        runs=runs,
        texts=texts if args.subject else [],
        contract=contract,
        rails=rails,
        pack_dir=pack_dir,
        persona_core=persona_core,
        deny_list=Path(private_deny) if private_deny else None,
        vault_override=vault,
        layout_override=layout_override,
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

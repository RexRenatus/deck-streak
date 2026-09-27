#!/usr/bin/env python3
"""pack-lint: lint the skill packs of any repository (SPEC-V2-2210, ADR-V2-2210).

A pack is a directory under `<skills>/packs/` holding a `SKILL.md` and a `checks.json`, with one row
in `<skills>/catalog.json`. This script judges those files for ANY repository through `--root`:
phoenix-v2, DeckStreak, or a tree this script was vendored into. It is standard-library Python.

Usage:
    pack-lint.py --root R [--skills DIR] [--pack SLUG] check CLASS
    pack-lint.py --root R [--skills DIR] [--pack SLUG] lint [--checks PATH]
    pack-lint.py --root R list

`check` runs one class. It prints one line per finding, `<class>: <finding>`, then `examined N`, the
population it read. A class with no subject in the tree prints `not-applicable: <why>` before the
count, which is SARIF's `notApplicable` and never a silent pass.

`lint` runs every row of the pack-authoring pack's own `checks.json` in-process, so a builder reads
every finding at once. It prints `<row id> <severity> <verdict>` per row, with the findings
indented beneath.

Exit: 0 green, or not applicable; 1 a finding; 2 a usage error; 3 VOID, meaning nothing was examined
or the catalog could not be read. VOID is never a pass. `lint` exits 1 when a block row is red or
VOID, and 0 otherwise.

Two classes compose phoenix-v2's own tools rather than copying them. `seat-recipes` imports
`seat-pack-recipes.py`'s functions from this script's directory. `census` runs `pack-census.py`
from this script's directory against `--root`, when the root carries the census's selector
`ops/pack-probe.py`. Vendor those two files beside this one.
"""

from __future__ import annotations

import argparse
import dataclasses
import importlib.util
import json
import re
import shutil
import subprocess
import sys
import types
from collections import Counter
from collections.abc import Callable, Iterable, Sequence
from pathlib import Path

EXIT_OK = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

HERE = Path(__file__).resolve().parent
CHECKS_SCHEMA = "phx.skills.checks.v1"
# The one model's closed key sets (phx_agent::skills::ChecksFile and CheckRow, deny_unknown_fields).
BODY_KEYS = frozenset(
    {
        "schema",
        "pack",
        "spec",
        "gate",
        "walk",
        "note",
        "paths",
        "paths_pending_because",
        "live_rows_need_the_box",
        "checks",
    }
)
ROW_KEYS = frozenset(
    {"id", "severity", "probe", "stage", "reason", "scope", "schedule"}
)
LIST_KEYS = ("walk", "paths")
TEXT_KEYS = ("spec", "gate", "note", "paths_pending_because", "live_rows_need_the_box")
PROBE_CARD = "phxd.pack.probe.v1"
RUN_CARD = "phxd.pack.run.v1"
# Each card's severity vocabulary, measured on the 26 packs at e9fded051. `pack probe` refuses any
# severity but these two; `pack run` reads every severity but `advisory` as blocking.
SEVERITIES = {
    PROBE_CARD: ("block", "advisory"),
    RUN_CARD: ("blocking", "advisory", "catalog"),
    "phxd.seo-pipeline.v1": ("blocking", "advisory"),
}
BLOCKING = ("block", "blocking")
ADVISORY = "advisory"
GATES = ("none", "advisory", "refuse-on-red")
SCOPES = ("tree", "live")
SCHEDULES = ("periodic", "on-demand")
MAX_WALL = 900
RESERVED = "reserved-1198"
SLUG = re.compile(r"[a-z0-9][a-z0-9._-]*")
NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
# A work-left marker, spelled in fragments so this file carries none (common brief trap 9).
MARKERS = re.compile(
    r"\b(?:" + "|".join(("TO" + "DO", "FIX" + "ME", "HA" + "CK", "X" + "XX")) + r")\b"
)
# A stated stage count: "The `stack` stage: 4 rows." or "### Stage `inventory`: 2 rows (2 blocking,
# 0 advisory)". Both forms are in the tree; a split is optional.
STATEMENT = re.compile(
    r"(?:\b[Ss]tage\s+`(?P<named>[^`\n]+)`|`(?P<quoted>[^`\n]+)`\s+stage)\s*:\s*(?P<count>\d+)\s+rows?\b"
    r"(?:\s*\((?P<blocks>\d+)\s+(?:block|blocking),\s*(?P<advisories>\d+)\s+advisory\))?"
)
DOC_LINE = re.compile(
    r"phxd\s+pack\s+(?P<verb>probe|run|quality|release)\b(?P<rest>.*)"
)
CHECK_LINE = re.compile(r"phxd\s+pack\s+check\b(?P<rest>.*)")
LINK = re.compile(r"!?\[[^\]\n]*\]\(\s*<?(?P<target>[^)\s>]+)>?(?:\s+\"[^\"]*\")?\s*\)")
SCHEME = re.compile(r"[A-Za-z][A-Za-z0-9+.-]*:")
FENCE = re.compile(r"^\s*(```|~~~)")
INLINE_CODE = re.compile(r"`[^`\n]*`")
XML_TAG = re.compile(r"<[A-Za-z/][^>]*>")
HEADING = re.compile(r"^(?P<level>#{1,6})\s+(?P<title>.*)$")
URL = re.compile(r"https?://\S+")
DATE = re.compile(r"\b20\d\d-\d\d-\d\d\b")
CONTEXT7_ID = re.compile(r"`/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)?`")


@dataclasses.dataclass
class Pack:
    """One catalogued pack and what could be read of it."""

    id: str
    slug: str
    row: dict
    dir: Path | None
    body: dict | None
    body_error: str | None
    text: str | None
    text_error: str | None

    def rows(self) -> list[dict]:
        """The body's check rows that are objects; `body-schema` names the rest."""
        checks = self.body.get("checks") if isinstance(self.body, dict) else None
        return (
            [entry for entry in checks if isinstance(entry, dict)]
            if isinstance(checks, list)
            else []
        )

    def cards(self) -> list[str]:
        cards = self.row.get("requires_phxd_schema")
        return (
            [card for card in cards if isinstance(card, str)]
            if isinstance(cards, list)
            else []
        )


@dataclasses.dataclass
class Tree:
    """The repository being judged: its catalog, its packs, and the ones `--pack` selected."""

    root: Path
    skills: Path
    catalog_rows: list[dict]
    packs: list[Pack]
    selected: list[Pack]
    only: str | None


@dataclasses.dataclass
class Outcome:
    """A class's verdict: its findings, what it read, and why it had no subject, if it had none."""

    findings: list[str] = dataclasses.field(default_factory=list)
    examined: int = 0
    not_applicable: str | None = None

    def exit_code(self) -> int:
        if self.examined == 0:
            return EXIT_VOID
        return EXIT_FINDING if self.findings else EXIT_OK


class Unreadable(Exception):
    """The catalog cannot be read, so no class can judge anything: VOID."""


class Usage(Exception):
    """An argument this script cannot act on."""


# --------------------------------------------------------------------------------------------
# Reading the tree.
# --------------------------------------------------------------------------------------------


def read_text(path: Path) -> tuple[str | None, str | None]:
    try:
        return path.read_text(encoding="utf-8"), None
    except (OSError, UnicodeDecodeError) as error:
        return None, f"cannot read {path.name}: {error}"


def slug_of(pack_id: str) -> str:
    return (
        pack_id.split("/", 1)[1]
        if pack_id.startswith("packs/")
        else pack_id.rsplit("/", 1)[-1]
    )


def load_tree(root: Path, skills: Path, only: str | None) -> Tree:
    """Read the catalog and every pack row's two files. A missing catalog raises `Unreadable`."""
    catalog_path = skills / "catalog.json"
    try:
        catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise Unreadable(f"cannot read {catalog_path}: {error}") from error
    entries = catalog.get("skills") if isinstance(catalog, dict) else None
    if not isinstance(entries, list):
        raise Unreadable(f"{catalog_path} holds no `skills` list")
    catalog_rows = [entry for entry in entries if isinstance(entry, dict)]
    packs = []
    for entry in catalog_rows:
        if entry.get("kind") != "pack":
            continue
        pack_id = str(entry.get("id", ""))
        path = entry.get("path")
        directory = (
            skills / path.strip("/")
            if isinstance(path, str) and path.strip("/")
            else None
        )
        body, body_error = None, None
        text, text_error = None, None
        if directory is None:
            body_error = text_error = "the catalog row names no path"
        else:
            raw, body_error = read_text(directory / "checks.json")
            if raw is not None:
                try:
                    body = json.loads(raw)
                except ValueError as error:
                    body_error = f"checks.json is not JSON: {error}"
            text, text_error = read_text(directory / "SKILL.md")
        packs.append(
            Pack(
                pack_id,
                slug_of(pack_id),
                entry,
                directory,
                body,
                body_error,
                text,
                text_error,
            )
        )
    selected = packs
    if only is not None:
        wanted = only if only.startswith("packs/") else f"packs/{only}"
        selected = [pack for pack in packs if pack.id == wanted]
        if not selected:
            raise Usage(f"no catalog row {wanted} in {catalog_path}")
    return Tree(root, skills, catalog_rows, packs, selected, only)


def names_id(text: str, ident: str) -> bool:
    """Whether `text` names `ident` with no letter, digit, `-` or `_` on either side.

    `lint_tree`'s own boundary (`names_id` in phx_agent::skills), so `roles/gate` is not read inside
    `roles/gate-economics`.
    """
    if not ident:
        return False
    start = 0
    while (at := text.find(ident, start)) >= 0:
        before = text[at - 1] if at else ""
        after = text[at + len(ident)] if at + len(ident) < len(text) else ""
        if not _id_char(before) and not _id_char(after):
            return True
        start = at + 1
    return False


def _id_char(char: str) -> bool:
    return bool(char) and char.isascii() and (char.isalnum() or char in "-_")


def frontmatter(text: str) -> str | None:
    """The text between the opening fence on line 1 and the closing one, as `lint_tree` reads it."""
    for fence in ("---\n", "---\r\n"):
        if text.startswith(fence):
            front = text[len(fence) :]
            end = front.find("\n---")
            return front[:end] if end >= 0 else None
    return None


def declared_cards(block: str) -> set[str]:
    """`requires_phxd_schema` as `frontmatter_schemas` reads it: a scalar, a list, or `none`."""
    declared: set[str] = set()
    in_list = False
    for line in block.splitlines():
        trimmed = line.strip()
        if trimmed.startswith("requires_phxd_schema:"):
            scalar = trimmed[len("requires_phxd_schema:") :].strip()
            if not scalar:
                in_list = True
                continue
            if scalar != "none":
                declared.add(scalar)
            break
        if in_list:
            if trimmed.startswith("- "):
                declared.add(trimmed[2:].strip())
            elif trimmed:
                break
    return declared


def scalar_fields(block: str) -> dict[str, str]:
    """The column-0 `key: value` scalars of a YAML frontmatter, folded and literal blocks included."""
    fields: dict[str, str] = {}
    lines = block.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        index += 1
        match = re.match(r"^([A-Za-z_][A-Za-z0-9_-]*):(.*)$", line)
        if not match:
            continue
        key, value = match.group(1), match.group(2).strip()
        if value[:1] in (">", "|"):
            folded = value[:1] == ">"
            parts = []
            while index < len(lines) and (
                lines[index].startswith((" ", "\t")) or not lines[index].strip()
            ):
                parts.append(lines[index].strip())
                index += 1
            value = (" " if folded else "\n").join(part for part in parts if part)
        elif len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        fields[key] = value
    return fields


def prose(text: str) -> str:
    """`text` without its fenced blocks and inline code: what a reader sees as prose."""
    kept = []
    fenced = False
    for line in text.splitlines():
        if FENCE.match(line):
            fenced = not fenced
            continue
        if not fenced:
            kept.append(INLINE_CODE.sub("", line))
    return "\n".join(kept)


def statements(text: str) -> list[re.Match[str]]:
    return list(STATEMENT.finditer(text))


def stage_of(statement: re.Match[str]) -> str:
    return statement.group("named") or statement.group("quoted") or ""


def section(text: str, predicate: Callable[[str], bool]) -> str | None:
    """The first heading whose title passes `predicate`, and everything below it up to the next
    heading of the same or a higher level; None when no heading passes."""
    lines = text.splitlines()
    for start, line in enumerate(lines):
        match = HEADING.match(line)
        if not match or not predicate(match.group("title")):
            continue
        level = len(match.group("level"))
        end = len(lines)
        for later in range(start + 1, len(lines)):
            deeper = HEADING.match(lines[later])
            if deeper and len(deeper.group("level")) <= level:
                end = later
                break
        return "\n".join(lines[start:end])
    return None


def spec_files(tree: Tree, spec: str) -> list[Path]:
    specs = tree.root / "docs" / "specs"
    if not specs.is_dir():
        return []
    found = sorted(specs.glob(f"{spec}-*.md"))
    exact = specs / f"{spec}.md"
    return found + ([exact] if exact.is_file() else [])


def load_sibling(name: str, filename: str) -> types.ModuleType | None:
    """Import a hyphen-named script beside this one, the way `pack-census.py` loads its siblings."""
    path = HERE / filename
    if not path.is_file():
        return None
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        return None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


# --------------------------------------------------------------------------------------------
# Stage `body`: the pack's checks.json.
# --------------------------------------------------------------------------------------------


def unreadable_body(pack: Pack, outcome: Outcome) -> bool:
    """Count and name a body that cannot be judged; True when the caller should skip the pack."""
    if pack.body_error is not None or not isinstance(pack.body, dict):
        outcome.examined += 1
        outcome.findings.append(
            f"{pack.id}: {pack.body_error or 'checks.json is not a JSON object'}"
        )
        return True
    return False


def body_schema(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        outcome.examined += 1
        body = pack.body
        assert isinstance(body, dict)
        if body.get("schema") != CHECKS_SCHEMA:
            outcome.findings.append(
                f"{pack.id}: schema is {body.get('schema')!r}, not {CHECKS_SCHEMA}"
            )
        if body.get("pack") != pack.id:
            outcome.findings.append(
                f"{pack.id}: checks.json declares pack={body.get('pack')!r}, catalog id is {pack.id!r}"
            )
        checks = body.get("checks")
        if not isinstance(checks, list) or not checks:
            outcome.findings.append(f"{pack.id}: checks is not a non-empty list")
            checks = []
        unknown = sorted(set(body) - BODY_KEYS)
        if unknown:
            outcome.findings.append(
                f"{pack.id}: unknown top-level key(s) {', '.join(unknown)}"
            )
        for key in LIST_KEYS:
            value = body.get(key)
            if key in body and not (
                isinstance(value, list) and all(isinstance(item, str) for item in value)
            ):
                outcome.findings.append(f"{pack.id}: {key} is not a list of strings")
        for key in TEXT_KEYS:
            if key in body and not isinstance(body[key], str):
                outcome.findings.append(f"{pack.id}: {key} is not a string")
        for position, entry in enumerate(checks):
            if not isinstance(entry, dict):
                outcome.findings.append(f"{pack.id}: row {position} is not an object")
                continue
            extra = sorted(set(entry) - ROW_KEYS)
            if extra:
                outcome.findings.append(
                    f"{pack.id}/{entry.get('id', position)}: unknown row key(s) {', '.join(extra)}"
                )
    return outcome


def row_fields(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        assert isinstance(pack.body, dict)
        vocabulary: set[str] = set()
        for card in pack.cards():
            if card in SEVERITIES:
                vocabulary.update(SEVERITIES[card])
            else:
                outcome.findings.append(
                    f"{pack.id}: card {card} has no severity vocabulary"
                )
        declared = pack.body.get("live_rows_need_the_box")
        needs_box = isinstance(declared, str) and bool(declared.strip())
        for entry in pack.rows():
            outcome.examined += 1
            ident = entry.get("id")
            where = f"{pack.id}/{ident}"
            if not isinstance(ident, str) or not SLUG.fullmatch(ident):
                outcome.findings.append(
                    f"{pack.id}: row id {ident!r} is not [a-z0-9][a-z0-9._-]*"
                )
            severity = entry.get("severity")
            if severity not in vocabulary:
                allowed = ", ".join(sorted(vocabulary)) or "none"
                outcome.findings.append(
                    f"{where}: severity {severity!r} is not one of {allowed}"
                )
            for key in ("stage", "reason"):
                value = entry.get(key)
                if not isinstance(value, str) or not value.strip():
                    outcome.findings.append(f"{where}: no {key}")
            scope = entry.get("scope")
            if scope not in SCOPES:
                outcome.findings.append(f"{where}: scope {scope!r} is not tree or live")
            if "schedule" in entry and entry["schedule"] not in SCHEDULES:
                outcome.findings.append(
                    f"{where}: schedule {entry['schedule']!r} is not periodic or on-demand"
                )
            if scope == "live" and not needs_box:
                outcome.findings.append(
                    f"{where}: a live row, and the body declares no live_rows_need_the_box"
                )
    return outcome


def row_ids_unique(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        rows = pack.rows()
        outcome.examined += len(rows)
        counts = Counter(
            entry.get("id") for entry in rows if isinstance(entry.get("id"), str)
        )
        for ident, count in counts.items():
            if count > 1:
                outcome.findings.append(
                    f"{pack.id}: row id {ident} appears {count} times"
                )
    return outcome


def walk_agrees(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        outcome.examined += 1
        assert isinstance(pack.body, dict)
        walk = pack.body.get("walk")
        if not walk:
            continue
        if not isinstance(walk, list) or not all(
            isinstance(stage, str) for stage in walk
        ):
            outcome.findings.append(f"{pack.id}: walk is not a list of stage names")
            continue
        for stage, count in Counter(walk).items():
            if count > 1:
                outcome.findings.append(
                    f"{pack.id}: walk names {stage!r} {count} times"
                )
        used = [entry.get("stage") for entry in pack.rows()]
        for stage in walk:
            if stage not in used:
                outcome.findings.append(
                    f"{pack.id}: walk names stage {stage!r}, which no row uses"
                )
        for stage in sorted(
            {stage for stage in used if isinstance(stage, str)} - set(walk)
        ):
            outcome.findings.append(
                f"{pack.id}: row stage {stage!r} is not in the walk"
            )
    return outcome


def probe_runnable(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        runs_names = RUN_CARD in pack.cards()
        for entry in pack.rows():
            outcome.examined += 1
            where = f"{pack.id}/{entry.get('id')}"
            probe = entry.get("probe")
            if isinstance(probe, str):
                if not runs_names:
                    outcome.findings.append(
                        f"{where}: a string probe {probe!r} runs only under {RUN_CARD}"
                    )
                elif probe == RESERVED:
                    outcome.findings.append(
                        f"{where}: the probe is the {RESERVED} marker"
                    )
            elif isinstance(probe, list):
                argv_findings(where, probe, outcome)
            elif isinstance(probe, dict):
                command_findings(tree, where, entry, probe, outcome)
            else:
                outcome.findings.append(
                    f"{where}: the probe is neither a name, an argv nor a command"
                )
    return outcome


def argv_findings(where: str, probe: list, outcome: Outcome) -> None:
    if not probe or not all(isinstance(word, str) and word for word in probe):
        outcome.findings.append(
            f"{where}: an argv probe with an empty or non-string word"
        )
        return
    if RESERVED in probe:
        outcome.findings.append(f"{where}: the argv names the {RESERVED} marker")
    if any("\\" in word for word in probe):
        outcome.findings.append(
            f"{where}: a backslash in the argv; paths take forward slashes"
        )


def command_findings(
    tree: Tree, where: str, entry: dict, probe: dict, outcome: Outcome
) -> None:
    extra = sorted(set(probe) - {"command", "timeout_seconds"})
    if extra:
        outcome.findings.append(f"{where}: unknown command key(s) {', '.join(extra)}")
    wall = probe.get("timeout_seconds")
    if isinstance(wall, bool) or not isinstance(wall, int) or not 1 <= wall <= MAX_WALL:
        outcome.findings.append(
            f"{where}: timeout_seconds {wall!r} is not 1..{MAX_WALL}"
        )
    command = probe.get("command")
    if (
        not isinstance(command, list)
        or not command
        or not all(isinstance(word, str) and word for word in command)
    ):
        outcome.findings.append(
            f"{where}: the command is not a non-empty list of words"
        )
        return
    if any("\\" in word for word in command):
        outcome.findings.append(
            f"{where}: a backslash in the command; paths take forward slashes"
        )
        return
    tree_row = entry.get("scope") == "tree"
    words = [substitute(tree, word) for word in command]
    program = words[0]
    if "/" in program:
        path = Path(program) if Path(program).is_absolute() else tree.root / program
        if not path.is_file():
            outcome.findings.append(
                f"{where}: the program {command[0]} is not a file in the tree"
            )
    elif shutil.which(program) is None:
        outcome.findings.append(f"{where}: the program {program} is not on PATH")
    for raw, word in zip(command[1:], words[1:], strict=True):
        shipped = raw.startswith("{skills}/")
        judged = raw.startswith("{root}/") and tree_row
        script = (
            tree_row and "/" in raw and "{" not in raw and raw.endswith((".py", ".sh"))
        )
        if not (shipped or judged or script):
            continue
        path = Path(word) if Path(word).is_absolute() else tree.root / word
        if not path.exists():
            outcome.findings.append(f"{where}: {raw} names nothing in the tree")


def substitute(tree: Tree, word: str) -> str:
    return word.replace("{root}", str(tree.root)).replace("{skills}", str(tree.skills))


def reason_slug(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        for entry in pack.rows():
            outcome.examined += 1
            reason = entry.get("reason")
            if isinstance(reason, str) and not SLUG.fullmatch(reason):
                outcome.findings.append(
                    f"{pack.id}/{entry.get('id')}: reason {reason!r} is prose, not a slug"
                )
    return outcome


def row_ids_catalog_unique(tree: Tree) -> Outcome:
    outcome = Outcome()
    owners: dict[str, list[str]] = {}
    for pack in tree.packs:
        for ident in dict.fromkeys(entry.get("id") for entry in pack.rows()):
            if isinstance(ident, str):
                owners.setdefault(ident, []).append(pack.id)
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        for entry in pack.rows():
            outcome.examined += 1
            ident = entry.get("id")
            if isinstance(ident, str) and len(owners.get(ident, [])) > 1:
                outcome.findings.append(
                    f"{pack.id}: row id {ident} is also used by {', '.join(o for o in owners[ident] if o != pack.id)}"
                )
    return outcome


def bar_owned_once(tree: Tree) -> Outcome:
    """#641: a probe two packs both run has two owners, unless one pack extends the other."""
    outcome = Outcome()
    extends = {pack.id: pack.row.get("extends") for pack in tree.packs}
    runners: dict[str, list[str]] = {}
    for pack in tree.packs:
        for entry in pack.rows():
            key = bar_key(entry.get("probe"))
            if key is not None and pack.id not in runners.setdefault(key, []):
                runners[key].append(pack.id)
    selected = {pack.id for pack in tree.selected}
    for pack in tree.selected:
        if unreadable_body(pack, outcome):
            continue
        outcome.examined += sum(
            bar_key(entry.get("probe")) is not None for entry in pack.rows()
        )
    for key, packs in runners.items():
        if len(packs) < 2 or not selected.intersection(packs):
            continue
        if len(packs) == 2 and (
            extends.get(packs[0]) == packs[1] or extends.get(packs[1]) == packs[0]
        ):
            continue
        outcome.findings.append(
            f"{', '.join(packs)} both run the probe {key}: one bar, one owner (#641)"
        )
    return outcome


def bar_key(probe: object) -> str | None:
    """A runnable probe's identity: its argv, or its command without the wall."""
    if isinstance(probe, list) and probe:
        return json.dumps(probe)
    if (
        isinstance(probe, dict)
        and isinstance(probe.get("command"), list)
        and probe["command"]
    ):
        return json.dumps(probe["command"])
    return None


# --------------------------------------------------------------------------------------------
# Stage `skill`: the pack's documents.
# --------------------------------------------------------------------------------------------


def unreadable_text(pack: Pack, outcome: Outcome) -> bool:
    if pack.text is None:
        outcome.examined += 1
        outcome.findings.append(f"{pack.id}: {pack.text_error or 'no SKILL.md'}")
        return True
    return False


def frontmatter_class(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None
        if not pack.text.startswith(("---\n", "---\r\n")):
            outcome.findings.append(
                f"{pack.id}: SKILL.md does not open with a --- frontmatter fence"
            )
            continue
        block = frontmatter(pack.text)
        if block is None:
            outcome.findings.append(
                f"{pack.id}: SKILL.md's frontmatter has no closing --- fence"
            )
            continue
        declared = declared_cards(block)
        expected = set(pack.cards())
        if declared != expected:
            outcome.findings.append(
                f"{pack.id}: SKILL.md declares {sorted(declared)}, the catalog row {sorted(expected)}"
            )
    return outcome


def stage_counts(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome) or unreadable_body(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None
        rows = pack.rows()
        counts = Counter(entry.get("stage") for entry in rows)
        for statement in statements(pack.text):
            stage = stage_of(statement)
            stated = int(statement.group("count"))
            if stage not in counts:
                outcome.findings.append(
                    f"{pack.id}: SKILL.md states {stated} rows for stage {stage!r}, which no row uses"
                )
                continue
            if counts[stage] != stated:
                outcome.findings.append(
                    f"{pack.id}: SKILL.md states {stated} rows for stage {stage!r}; checks.json has {counts[stage]}"
                )
            if statement.group("blocks") is not None:
                blocks = sum(
                    entry.get("stage") == stage and entry.get("severity") in BLOCKING
                    for entry in rows
                )
                advisories = sum(
                    entry.get("stage") == stage and entry.get("severity") == ADVISORY
                    for entry in rows
                )
                split = (
                    int(statement.group("blocks")),
                    int(statement.group("advisories")),
                )
                if split != (blocks, advisories):
                    outcome.findings.append(
                        f"{pack.id}: stage {stage!r} split stated {split[0]} blocking, {split[1]} advisory; "
                        f"checks.json has {blocks}, {advisories}"
                    )
    return outcome


def doc_lines(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome) or unreadable_body(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None
        ids = {entry.get("id") for entry in pack.rows()}
        for number, line in enumerate(pack.text.splitlines(), start=1):
            trimmed = line.strip()
            documented = DOC_LINE.match(trimmed)
            if documented:
                named = re.search(r"--pack(?:\s+|=)(\S+)", documented.group("rest"))
                if named is None:
                    outcome.findings.append(
                        f"{pack.id}: SKILL.md:{number} names no --pack"
                    )
                elif named.group(1).strip("`") not in (pack.slug, pack.id):
                    outcome.findings.append(
                        f"{pack.id}: SKILL.md:{number} documents a line for {named.group(1).strip('`')}"
                    )
            checked = CHECK_LINE.match(trimmed)
            if checked:
                named = re.search(r"--id(?:\s+|=)(\S+)", checked.group("rest"))
                if named is not None and named.group(1).strip("`") not in ids | {"ID"}:
                    outcome.findings.append(
                        f"{pack.id}: SKILL.md:{number} checks id {named.group(1).strip('`')}, which is no row"
                    )
    return outcome


def links_resolve(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None and pack.dir is not None
        for link in LINK.finditer(prose(pack.text)):
            target = link.group("target")
            if SCHEME.match(target) or target.startswith("#"):
                continue
            relative = target.split("#", 1)[0].split("?", 1)[0]
            if relative and not (pack.dir / relative).exists():
                outcome.findings.append(
                    f"{pack.id}: SKILL.md links {target}, which does not resolve"
                )
    return outcome


def deny_rules(text: str) -> list[str]:
    """The tokens of a `DENY.md` pipe table, parsed as `load_deny_rules` parses it."""
    tokens = []
    for line in text.splitlines():
        trimmed = line.strip()
        if not trimmed.startswith("|"):
            continue
        cells = [cell.strip() for cell in trimmed.split("|") if cell.strip()]
        if (
            len(cells) < 3
            or cells[0] == "id"
            or cells[0].startswith("-")
            or not cells[0].startswith("D-")
        ):
            continue
        token = cells[1].strip("`")
        if token:
            tokens.append(token)
    return tokens


def deny_clean(tree: Tree) -> Outcome:
    outcome = Outcome()
    table = tree.skills / "deny" / "DENY.md"
    if not table.is_file():
        return Outcome(
            examined=1,
            not_applicable=f"no deny table at {table.relative_to(tree.root) if table.is_relative_to(tree.root) else table}",
        )
    text, error = read_text(table)
    tokens = deny_rules(text or "")
    if error or not tokens:
        return Outcome(
            [f"the deny table {table.name} declares no D-* rule"], examined=1
        )
    for pack in tree.selected:
        if pack.dir is None:
            continue
        for name in ("SKILL.md", "checks.json"):
            body, _ = read_text(pack.dir / name)
            if body is None:
                continue
            outcome.examined += 1
            haystack = body.lower()
            for token in tokens:
                if token.lower() in haystack:
                    outcome.findings.append(
                        f"{pack.id}/{name} examples the deny token {token!r}"
                    )
    return outcome


def no_work_markers(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if pack.dir is None or not pack.dir.is_dir():
            continue
        for path in sorted(path for path in pack.dir.rglob("*") if path.is_file()):
            outcome.examined += 1
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError as error:
                outcome.findings.append(f"{pack.id}: cannot read {path.name}: {error}")
                continue
            for match in MARKERS.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                outcome.findings.append(
                    f"{pack.id}: {path.relative_to(pack.dir).as_posix()}:{line} carries a work-left marker"
                )
    return outcome


def names_no_seat(tree: Tree) -> Outcome:
    outcome = Outcome()
    seats = [
        str(entry.get("id"))
        for entry in tree.catalog_rows
        if entry.get("kind") != "pack"
    ]
    for pack in tree.selected:
        if unreadable_text(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None
        for seat in seats:
            if names_id(pack.text, seat):
                outcome.findings.append(
                    f"{pack.id}: SKILL.md names the seat {seat}; the catalog row's consumes carries that edge"
                )
    return outcome


def rows_named(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome) or unreadable_body(pack, outcome):
            continue
        assert pack.text is not None
        for entry in pack.rows():
            outcome.examined += 1
            ident = entry.get("id")
            if isinstance(ident, str) and not names_id(pack.text, ident):
                outcome.findings.append(
                    f"{pack.id}: SKILL.md never names the row {ident}"
                )
    return outcome


def stage_counts_stated(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome) or unreadable_body(pack, outcome):
            continue
        assert pack.text is not None
        stated = {stage_of(statement) for statement in statements(pack.text)}
        used = dict.fromkeys(
            entry.get("stage")
            for entry in pack.rows()
            if isinstance(entry.get("stage"), str)
        )
        for stage in used:
            outcome.examined += 1
            if stage not in stated:
                outcome.findings.append(
                    f"{pack.id}: SKILL.md states no row count for stage {stage!r}"
                )
    return outcome


def agent_skill_fields(tree: Tree) -> Outcome:
    """The Agent Skills specification's `name` and `description` rules (agentskills.io)."""
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None
        fields = scalar_fields(frontmatter(pack.text) or "")
        name = fields.get("name")
        directory = pack.dir.name if pack.dir is not None else pack.slug
        if not name:
            outcome.findings.append(
                f"{pack.id}: the frontmatter has no Agent Skills name"
            )
        else:
            if len(name) > 64:
                outcome.findings.append(
                    f"{pack.id}: name is {len(name)} characters; at most 64"
                )
            if not NAME.fullmatch(name):
                outcome.findings.append(
                    f"{pack.id}: name {name!r} is not lowercase letters, digits and single inner hyphens"
                )
            if name != directory:
                outcome.findings.append(
                    f"{pack.id}: name {name!r} is not the directory name {directory!r}"
                )
            if "anthropic" in name or "claude" in name:
                outcome.findings.append(
                    f"{pack.id}: name {name!r} uses a reserved word"
                )
        description = fields.get("description")
        if not description:
            outcome.findings.append(
                f"{pack.id}: the frontmatter has no Agent Skills description"
            )
        else:
            if len(description) > 1024:
                outcome.findings.append(
                    f"{pack.id}: description is {len(description)} characters; at most 1024"
                )
            if XML_TAG.search(description):
                outcome.findings.append(f"{pack.id}: description carries an XML tag")
    return outcome


def body_length(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        if unreadable_text(pack, outcome):
            continue
        outcome.examined += 1
        assert pack.text is not None
        lines = len(pack.text.splitlines())
        if lines > 500:
            outcome.findings.append(
                f"{pack.id}: SKILL.md is {lines} lines; keep it at 500 or under"
            )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `spec`: the pack's SPEC.
# --------------------------------------------------------------------------------------------


def pack_spec(tree: Tree, pack: Pack) -> tuple[Path | None, str | None]:
    """The one SPEC file the body's `spec` names, or why there is none."""
    spec = pack.body.get("spec") if isinstance(pack.body, dict) else None
    if not isinstance(spec, str) or not spec.strip():
        return None, "checks.json names no SPEC"
    files = spec_files(tree, spec.strip())
    if len(files) != 1:
        return (
            None,
            f"{spec} is {'not on disk' if not files else f'{len(files)} files'} under docs/specs",
        )
    return files[0], None


def spec_named(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        outcome.examined += 1
        _, why = pack_spec(tree, pack)
        if why:
            outcome.findings.append(f"{pack.id}: {why}")
    return outcome


def spec_text(tree: Tree, pack: Pack, outcome: Outcome) -> str | None:
    path, why = pack_spec(tree, pack)
    outcome.examined += 1
    if path is None:
        outcome.findings.append(f"{pack.id}: no SPEC to read ({why})")
        return None
    text, error = read_text(path)
    if text is None:
        outcome.findings.append(f"{pack.id}: {error}")
    return text


def spec_coverage(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        text = spec_text(tree, pack, outcome)
        if text is None:
            continue
        matrix = section(text, lambda title: "coverage" in title.lower())
        if matrix is None:
            outcome.findings.append(
                f"{pack.id}: the SPEC has no coverage matrix heading"
            )
            continue
        for entry in pack.rows():
            ident = entry.get("id")
            if isinstance(ident, str) and not names_id(matrix, ident):
                outcome.findings.append(
                    f"{pack.id}: the SPEC's coverage matrix never names the row {ident}"
                )
    return outcome


def spec_references(tree: Tree) -> Outcome:
    outcome = Outcome()
    for pack in tree.selected:
        text = spec_text(tree, pack, outcome)
        if text is None:
            continue
        references = section(
            text, lambda title: title.strip().lower().rstrip(":").endswith("references")
        )
        if references is None:
            outcome.findings.append(f"{pack.id}: the SPEC has no References section")
            continue
        if not URL.search(references):
            outcome.findings.append(f"{pack.id}: the References name no URL")
        if not DATE.search(references):
            outcome.findings.append(
                f"{pack.id}: the References record no access date (YYYY-MM-DD)"
            )
        answered = CONTEXT7_ID.search(references) or re.search(
            r"none answered", references, re.IGNORECASE
        )
        if "context7" not in references.lower() or not answered:
            outcome.findings.append(
                f"{pack.id}: the References name no Context7 id, and do not say none answered"
            )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `catalog`: the wiring.
# --------------------------------------------------------------------------------------------


def skill_directories(skills: Path, under: Path) -> list[str]:
    """Every directory below `under` holding a SKILL.md, relative to `skills`, `/`-joined."""
    if not under.is_dir():
        return []
    return sorted(
        path.parent.relative_to(skills).as_posix()
        for path in under.rglob("SKILL.md")
        if path.is_file()
    )


def catalog_row(tree: Tree) -> Outcome:
    outcome = Outcome()
    by_id = {str(entry.get("id")): entry for entry in tree.catalog_rows}
    paths = {
        str(entry.get("path", "")).strip("/"): str(entry.get("id"))
        for entry in tree.catalog_rows
    }
    if tree.only is None:
        scanned = skill_directories(tree.skills, tree.skills / "packs")
    else:
        scanned = [
            directory
            for pack in tree.selected
            if pack.dir is not None
            for directory in skill_directories(tree.skills, pack.dir)
        ]
    for directory in scanned:
        outcome.examined += 1
        if directory not in paths:
            outcome.findings.append(
                f"skills/{directory} holds a SKILL.md and no catalog row names it; a template is never named SKILL.md"
            )
    for entry in tree.catalog_rows:
        ident = str(entry.get("id"))
        if ident.startswith("packs/") and entry.get("kind") != "pack":
            outcome.findings.append(
                f"{ident}: its kind is {entry.get('kind')!r}, not pack"
            )
    for pack in tree.selected:
        outcome.examined += 1
        entry = pack.row
        if entry.get("path") != f"{pack.id}/":
            outcome.findings.append(
                f"{pack.id}: path {entry.get('path')!r} is not {pack.id}/"
            )
        if (
            pack.dir is None
            or not (pack.dir / "SKILL.md").is_file()
            or not (pack.dir / "checks.json").is_file()
        ):
            outcome.findings.append(
                f"{pack.id}: its directory holds no SKILL.md and checks.json"
            )
        if entry.get("gate") not in GATES:
            outcome.findings.append(
                f"{pack.id}: gate {entry.get('gate')!r} is not one of {', '.join(GATES)}"
            )
        consumes = entry.get("consumes")
        if not isinstance(consumes, list) or not consumes:
            outcome.findings.append(f"{pack.id}: consumes names no seat")
        else:
            for seat in consumes:
                target = by_id.get(str(seat))
                if target is None or target.get("kind") == "pack":
                    outcome.findings.append(
                        f"{pack.id}: consumes {seat}, which is no catalogued seat"
                    )
        extends = entry.get("extends")
        if extends is not None and (
            by_id.get(str(extends), {}).get("kind") != "pack" or extends == pack.id
        ):
            outcome.findings.append(
                f"{pack.id}: extends {extends}, which is no other pack row"
            )
        if not pack.cards():
            outcome.findings.append(f"{pack.id}: requires_phxd_schema declares no card")
    return outcome


def leaf(ident: str) -> str:
    """The name `phxd path exec --seat` resolves a row by (test_catalog_leaf_names_are_unique)."""
    return ident.replace(".", "/").rsplit("/", 1)[-1]


def leaf_unique(tree: Tree) -> Outcome:
    outcome = Outcome(examined=len(tree.catalog_rows))
    owners: dict[str, list[str]] = {}
    for entry in tree.catalog_rows:
        owners.setdefault(leaf(str(entry.get("id"))), []).append(str(entry.get("id")))
    wanted = {leaf(pack.id) for pack in tree.selected}
    for name, ids in owners.items():
        if len(ids) > 1 and (tree.only is None or name in wanted):
            outcome.findings.append(f"leaf {name!r} answers to {', '.join(ids)}")
    return outcome


def seat_recipes(tree: Tree) -> Outcome:
    recipes = load_sibling("seat_pack_recipes_pack_lint", "seat-pack-recipes.py")
    if recipes is None:
        return Outcome(
            [f"seat-pack-recipes.py is not beside {Path(__file__).name}"], examined=0
        )
    try:
        edges = recipes.seats_and_edges(tree.skills)
    except (OSError, ValueError, KeyError) as error:
        return Outcome([f"cannot read the seats: {error}"], examined=0)
    if not edges:
        return Outcome(
            examined=len(tree.catalog_rows),
            not_applicable="the catalog declares no seat",
        )
    outcome = Outcome()
    selected = {pack.id for pack in tree.selected}
    for seat, consumed in edges:
        outcome.examined += 1
        if tree.only is not None and not selected.intersection(
            pack["id"] for pack in consumed
        ):
            continue
        path = tree.skills / str(seat.get("path", "")).strip("/") / "SKILL.md"
        text, error = read_text(path)
        if text is None:
            outcome.findings.append(f"{seat.get('id')}: {error}")
            continue
        try:
            wanted = recipes.render_seat(text, seat["id"], consumed)
        except recipes.Refused as refusal:
            outcome.findings.append(f"{seat.get('id')}: {refusal}")
            continue
        if wanted != text:
            outcome.findings.append(
                f"{seat.get('id')}: its pack recipes differ from the catalog; run seat-pack-recipes.py --write"
            )
    return outcome


def census(tree: Tree) -> Outcome:
    selector = tree.root / "ops" / "pack-probe.py"
    if not selector.is_file():
        return Outcome(
            examined=1,
            not_applicable="the root carries no ops/pack-probe.py, the census's selector "
            "(phoenix-v2 machinery, SPEC-V2-2187)",
        )
    script = HERE / "pack-census.py"
    if not script.is_file():
        return Outcome(
            [f"pack-census.py is not beside {Path(__file__).name}"], examined=0
        )
    try:
        done = subprocess.run(
            [sys.executable, str(script), "--repo", str(tree.root), "--format", "json"],
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        return Outcome([f"the census did not run: {error}"], examined=1)
    if done.returncode == 2:
        return Outcome(
            [f"the census refused the catalog: {done.stdout.strip()}"], examined=1
        )
    try:
        report = json.loads(done.stdout)
    except ValueError:
        tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
        return Outcome(
            [f"the census did not report (exit {done.returncode}): {tail}"], examined=1
        )
    selected = {pack.id for pack in tree.selected}
    outcome = Outcome(examined=int(report.get("examined", 0)))
    for pack in report.get("packs", []):
        if pack.get("id") not in selected:
            continue
        for gap in pack.get("gaps", []):
            outcome.findings.append(f"{pack.get('id')}: {gap}")
    for reference in report.get("stale_references", []):
        if tree.only is None or any(ident in reference for ident in selected):
            outcome.findings.append(f"stale reference: {reference}")
    return outcome


def skills_readme(tree: Tree) -> Outcome:
    outcome = Outcome(examined=1)
    readme = tree.skills / "README.md"
    text, _ = read_text(readme) if readme.is_file() else (None, None)
    if text is None:
        outcome.findings.append(
            "no skills/README.md carries the install and ratchet contract (#1293)"
        )
        return outcome
    titles = [
        match.group("title").lower()
        for line in text.splitlines()
        if (match := HEADING.match(line))
    ]
    for word in ("install", "ratchet"):
        if not any(word in title for title in titles):
            outcome.findings.append(f"skills/README.md has no {word} section (#1293)")
    return outcome


# --------------------------------------------------------------------------------------------
# The command line.
# --------------------------------------------------------------------------------------------

CLASSES: dict[str, tuple[str, Callable[[Tree], Outcome]]] = {
    "body-schema": ("body", body_schema),
    "row-fields": ("body", row_fields),
    "row-ids-unique": ("body", row_ids_unique),
    "walk-agrees": ("body", walk_agrees),
    "probe-runnable": ("body", probe_runnable),
    "reason-slug": ("body", reason_slug),
    "row-ids-catalog-unique": ("body", row_ids_catalog_unique),
    "bar-owned-once": ("body", bar_owned_once),
    "frontmatter": ("skill", frontmatter_class),
    "stage-counts": ("skill", stage_counts),
    "doc-lines": ("skill", doc_lines),
    "links-resolve": ("skill", links_resolve),
    "deny-clean": ("skill", deny_clean),
    "no-work-markers": ("skill", no_work_markers),
    "names-no-seat": ("skill", names_no_seat),
    "rows-named": ("skill", rows_named),
    "stage-counts-stated": ("skill", stage_counts_stated),
    "agent-skill-fields": ("skill", agent_skill_fields),
    "body-length": ("skill", body_length),
    "spec-named": ("spec", spec_named),
    "spec-coverage": ("spec", spec_coverage),
    "spec-references": ("spec", spec_references),
    "catalog-row": ("catalog", catalog_row),
    "leaf-unique": ("catalog", leaf_unique),
    "seat-recipes": ("catalog", seat_recipes),
    "census": ("catalog", census),
    "skills-readme": ("catalog", skills_readme),
}


def report(cls: str, outcome: Outcome, indent: str = "") -> list[str]:
    lines = [f"{indent}{cls}: {finding}" for finding in outcome.findings]
    if outcome.not_applicable:
        lines.append(f"{indent}not-applicable: {outcome.not_applicable}")
    lines.append(f"{indent}examined {outcome.examined}")
    return lines


def lint_rows(checks: Path) -> list[tuple[str, str, str]]:
    """(row id, severity, class) for every row of `checks` that runs this script's `check`."""
    try:
        body = json.loads(checks.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Usage(f"cannot read {checks}: {error}") from error
    rows = []
    for entry in body.get("checks", []):
        command = (
            entry.get("probe", {}).get("command", [])
            if isinstance(entry.get("probe"), dict)
            else []
        )
        if "check" in command and command.index("check") + 1 < len(command):
            cls = command[command.index("check") + 1]
            if cls in CLASSES:
                rows.append((str(entry.get("id")), str(entry.get("severity")), cls))
    if not rows:
        raise Usage(f"{checks} holds no row that runs pack-lint.py check")
    return rows


def lint(tree: Tree | None, void: str | None, checks: Path) -> tuple[int, list[str]]:
    lines = []
    failed = False
    for ident, severity, cls in lint_rows(checks):
        outcome = (
            CLASSES[cls][1](tree)
            if tree is not None
            else Outcome([void or "unreadable"], 0)
        )
        code = outcome.exit_code()
        verdict = {EXIT_OK: "green", EXIT_FINDING: "red", EXIT_VOID: "void"}[code]
        if verdict == "red" and severity == ADVISORY:
            verdict = ADVISORY
        failed = failed or (severity != ADVISORY and verdict in ("red", "void"))
        lines.append(f"{ident} {severity} {verdict}")
        if code != EXIT_OK or outcome.not_applicable:
            lines.extend(report(cls, outcome, indent="  ")[:-1])
    return (EXIT_FINDING if failed else EXIT_OK), lines


def parse(argv: Sequence[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="pack-lint.py", description=__doc__.split("\n\n")[0]
    )
    parser.add_argument(
        "--root", required=True, help="the repository whose packs are judged"
    )
    parser.add_argument("--skills", help="its skills directory; default ROOT/skills")
    parser.add_argument("--pack", help="judge one pack, by slug or catalog id")
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check", help="run one class")
    check.add_argument("cls", metavar="CLASS")
    commands.add_parser("list", help="print every class and its stage")
    run_all = commands.add_parser(
        "lint", help="run every row of the pack's checks.json"
    )
    run_all.add_argument(
        "--checks",
        default=str(
            HERE.parent / "skills" / "packs" / "pack-authoring" / "checks.json"
        ),
        help="the checks.json whose rows to run",
    )
    return parser.parse_args(argv)


def emit(lines: Iterable[str]) -> None:
    for line in lines:
        print(line)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse(argv)
    if args.command == "list":
        emit(f"{cls} {stage}" for cls, (stage, _) in CLASSES.items())
        return EXIT_OK
    root = Path(args.root).resolve()
    if not root.is_dir():
        print(f"pack-lint: --root {args.root} is not a directory")
        return EXIT_USAGE
    if args.command == "check" and args.cls not in CLASSES:
        print(f"pack-lint: unknown class {args.cls}; `list` names them")
        return EXIT_USAGE
    skills = Path(args.skills).resolve() if args.skills else root / "skills"
    tree: Tree | None = None
    void: str | None = None
    try:
        tree = load_tree(root, skills, args.pack)
    except Unreadable as error:
        void = str(error)
    except Usage as error:
        print(f"pack-lint: {error}")
        return EXIT_USAGE
    if args.command == "lint":
        try:
            code, lines = lint(tree, void, Path(args.checks))
        except Usage as error:
            print(f"pack-lint: {error}")
            return EXIT_USAGE
        emit(lines)
        return code
    if tree is None:
        emit([f"{args.cls}: {void}", "examined 0"])
        return EXIT_VOID
    outcome = CLASSES[args.cls][1](tree)
    emit(report(args.cls, outcome))
    return outcome.exit_code()


if __name__ == "__main__":
    sys.exit(main())

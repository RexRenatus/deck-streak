#!/usr/bin/env python3
"""The app campaign's threat model reader (SPEC-375 R5, ADR-386 D5).

It reads every `docs/schematics/*.md`. A model is a file whose front matter carries
`trace: threat-model`. Its surfaces table names each surface; each surface's section holds a
STRIDE table; and every citation in an `entry point`, `control` or `pinned by` cell must name a
file in the tree whose cited line or lines contain its quote. A schematic that declares no trace
but holds a control table under a threat-model heading is refused. Standard library only.
"""

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMATICS = "docs/schematics"
PATTERN = "*.md"
TRACE = "trace: threat-model"
STRIDE = "STRIDE"
SKIPPED = {".git", "target", "node_modules"}
GIT = ".git"
PIPE = "|"
CONTROL = "control"
SURFACE_COLUMNS = ["surface", "entry point"]
ROW_COLUMNS = ["id", "threat", "asset", "control", "pinned by"]

FRONT_MATTER = re.compile(r"(?s)---\n(.*?)\n---(?:\n|\Z)")
HEADING = re.compile(r"(#+) (.*)")
SECTION = re.compile(r"## [0-9]+\. Surface: (.+)")
NAMED = re.compile(r"(?i)threat[ -]model")
RULE = re.compile(r"\|[-:| ]+\|")
TOKEN = re.compile(r"`([^`]*)`")
CITATION = re.compile(r"([^\s:]+):([0-9]+)(?:-([0-9]+))?(?::(.*))?")
IDENT = re.compile(r"[STRIDE][0-9]+")

FINDING = "{}:{}: {}: {}"
EXAMINED = "examined {} model(s), {} surface(s), {} row(s), {} citation(s)"
SURFACES = "surfaces"
SECTIONS = "sections"
TRACED = "trace"
MOVED = "quote not on a cited line: "
NO_FILE = "names no file: "
NO_QUOTE = "no quote: "
NOT_CITATION = "not a citation: "
CONTROL_NOTHING = "the control cites nothing"
PIN_NOTHING = "the pin cites nothing"
NOT_STRIDE = "not a STRIDE id"
REPEATS = "repeats an id"
NO_ROW = "surface {}: no row for {}"
NO_TABLE = "surface {}: no table"
NO_SURFACE = "section names no declared surface: "
UNDECLARED = "a control table under a threat-model heading, and the document declares no trace"


@dataclass(frozen=True)
class Report:
    """What one reading of a tree found (SPEC-375 R5)."""

    declared: list[str]
    surfaces: list[str]
    categories: dict[str, frozenset[str]]
    rows: list[str]
    citations: int
    findings: list[str]


def split(line: str) -> list[str]:
    """A table line's cells, trimmed."""
    return [cell.strip() for cell in line.strip().strip(PIPE).split(PIPE)]


class Reading:
    """One pass over a tree's schematics; `hollow` names each model that declares no surface."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.declared: list[str] = []
        self.surfaces: list[str] = []
        self.categories: dict[str, frozenset[str]] = {}
        self.rows: list[str] = []
        self.citations = 0
        self.findings: list[str] = []
        self.hollow: list[str] = []
        for path in sorted((root / SCHEMATICS).glob(PATTERN)):
            doc = path.relative_to(root).as_posix()
            text = path.read_text(encoding="utf-8")
            front = FRONT_MATTER.match(text)
            if front is not None and TRACE in front.group(1).splitlines():
                self.declared.append(doc)
                self.model(doc, text.splitlines())
            else:
                self.undeclared(doc, text.splitlines())

    def find(self, doc: str, n: int, subject: str, reason: str) -> None:
        """Record one finding line."""
        self.findings.append(FINDING.format(doc, n, subject, reason))

    def model(self, doc: str, lines: list[str]) -> None:
        """Judge one declared model: its rows and citations, then each surface's classes."""
        named: dict[str, int] = {}
        sections: list[tuple[int, str]] = []
        classes: dict[str, set[str]] = {}
        seen: set[str] = set()
        columns: list[str] | None = None
        stride: set[str] | None = None
        section: str | None = None
        for n, line in enumerate(lines, 1):
            if not line.startswith(PIPE):
                columns = None
                if HEADING.match(line) is not None:
                    found = SECTION.fullmatch(line)
                    section = None
                    if found is not None:
                        section = found.group(1)
                        sections.append((n, section))
            elif columns is None:
                columns = split(line)
                stride = None
                if section is not None and columns == ROW_COLUMNS:
                    stride = classes.setdefault(section, set())
            elif RULE.fullmatch(line) is None:
                cells = split(line)
                if columns == SURFACE_COLUMNS:
                    name, entry = cells
                    self.surfaces.append(name)
                    named[name] = n
                    self.cell(doc, n, SURFACES, entry, None)
                elif stride is not None:
                    ident, _threat, _asset, control, pin = cells
                    self.rows.append(ident)
                    if IDENT.fullmatch(ident) is None:
                        self.find(doc, n, ident, NOT_STRIDE)
                    elif ident in seen:
                        self.find(doc, n, ident, REPEATS)
                    seen.add(ident)
                    stride.add(ident[:1])
                    self.cell(doc, n, ident, control, CONTROL_NOTHING)
                    self.cell(doc, n, ident, pin, PIN_NOTHING)
        if not named:
            self.hollow.append(doc)
        for name, n in named.items():
            if name in classes:
                self.categories[name] = frozenset(classes[name])
                missing = "".join(letter for letter in STRIDE if letter not in classes[name])
                if missing:
                    self.find(doc, n, SURFACES, NO_ROW.format(name, missing))
            else:
                self.find(doc, n, SURFACES, NO_TABLE.format(name))
        for n, name in sections:
            if name not in named:
                self.find(doc, n, SECTIONS, NO_SURFACE + name)

    def cell(self, doc: str, n: int, subject: str, text: str, nothing: str | None) -> None:
        """Judge each backticked token of one cell; `nothing` is the finding for a cell with none."""
        tokens = TOKEN.findall(text)
        if nothing and not tokens:
            self.find(doc, n, subject, nothing)
        for token in tokens:
            found = CITATION.fullmatch(token)
            if found is None:
                self.find(doc, n, subject, NOT_CITATION + token)
            else:
                self.cite(doc, n, subject, token, found.groups())

    def cite(self, doc: str, n: int, subject: str, token: str, groups: tuple) -> None:
        """Judge one citation: it names a file in the tree, and its quote is on a cited line."""
        self.citations += 1
        path, first, last, quote = groups
        parts = Path(path).parts
        target = self.root / path
        if not quote:
            self.find(doc, n, subject, NO_QUOTE + token)
        elif (
            SKIPPED.intersection(parts)
            or not target.is_file()
            or any(
                (self.root.joinpath(*parts[:depth]) / GIT).exists()
                for depth in range(1, len(parts))
            )
        ):
            self.find(doc, n, subject, NO_FILE + token)
        else:
            cited = target.read_text(encoding="utf-8").splitlines()
            window = cited[int(first) - 1 : int(last or first)]
            if not any(quote in line for line in window):
                self.find(doc, n, subject, MOVED + token)

    def undeclared(self, doc: str, lines: list[str]) -> None:
        """Refuse a control table under a threat-model heading in a document with no trace."""
        level = opened = None
        for n, line in enumerate(lines, 1):
            heading = HEADING.match(line)
            if heading is not None:
                depth = len(heading.group(1))
                if opened is not None and depth <= level:
                    opened = None
                if NAMED.search(heading.group(2)) is not None:
                    level, opened = depth, n
            elif opened is not None and line.startswith(PIPE) and CONTROL in split(line):
                self.find(doc, opened, TRACED, UNDECLARED)


def judge(root: Path) -> Report:
    """Read every schematic under the root and report what the models hold."""
    reading = Reading(root)
    return Report(
        reading.declared,
        reading.surfaces,
        reading.categories,
        reading.rows,
        reading.citations,
        reading.findings,
    )


def main(argv: list[str] | None = None) -> int:
    """Print each finding, then the examined line; exit 3 (VOID), 1 on a finding, else 0."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    reading = Reading(args.root)
    for finding in reading.findings:
        print(finding)
    print(
        EXAMINED.format(
            len(reading.declared),
            len(reading.surfaces),
            len(reading.rows),
            reading.citations,
        )
    )
    if not reading.declared or reading.hollow:
        return 3
    return 1 if reading.findings else 0


if __name__ == "__main__":
    sys.exit(main())

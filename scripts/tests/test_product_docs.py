"""The product documents follow ADR-589 by dated notes that keep their text (SPEC-651 A1 to A3;
ADR-673).

DeckStreak's surfaces are the web app and the universal iPhone and iPad app, and the Telegram Mini
App and the bot are deprecated (ADR-589). The PRD and the design system say so in one dated note
each, and every section of either that names a Telegram surface also says it is deprecated: the
section is the unit, because a wrapped sentence names a surface on one line and the word on the
next. SPEC-001 and SPEC-334 are records: each keeps its earlier lines byte for byte, held by their
sha256, and gains one dated note after them. Each checker first refuses a planted inline text,
beside a control that reads clean, and every population it examines is refused at zero.
"""

import hashlib
import re
import unittest
from pathlib import Path

from _support import REPO, examined

PRD = Path("docs/PRD.md")
DESIGN_SYSTEM = Path("docs/DESIGN_SYSTEM.md")
PRODUCT_DOCS = (PRD, DESIGN_SYSTEM)
# SPEC-651 R3, R4 and A3: each campaign spec, the number of lines it held before its note, and the
# sha256 of those lines, each with its newline.
CAMPAIGN_SPECS = (
    (
        Path("docs/specs/SPEC-001-campaign-prd-parity-and-waves.md"),
        433,
        "8f532904fcee5400ed7f08243e37acef99f3959c244b1bd88708d616108741cb",
    ),
    (
        Path("docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md"),
        256,
        "f187f6a5f68de754f8a9e75fd29f3634ca592de161814ecb9f91672f4acd5dab",
    ),
)
# SPEC-651 R6, A1: a section's heading, a Telegram surface, and the word that stands beside it.
HEADING = re.compile(r"^#{1,2} ")
SURFACE = re.compile(r"(?i)mini[\s-]?apps?|telegram|\bbots?\b|initdata")
DEPRECATED = re.compile(r"(?i)deprecat")
LEAD = "(the text before the first heading)"
# SPEC-651 R6, A2 and A3: the dated note's opening, its mark, and the words each note names.
NOTE_OPENING = "Amendment (2026-10-29, ADR-589)"
NOTE_MARK = "(2026-10-29, ADR-589)"
NOTE_NAMES = ("web app", "iPhone and iPad", "deprecated")
SPEC_NOTE_NAMES = (*NOTE_NAMES, "Rex")


def flat(text):
    """`text` with every run of whitespace collapsed to one space."""
    return " ".join(text.split())


def read(path):
    """The text of the repository file at `path`."""
    return (REPO / path).read_text(encoding="utf-8")


def sections(text):
    """(heading, text) of each section of `text`, split at the lines matching `^#{1,2} `. Each
    section holds its heading line; the text before the first heading, if any, is its own."""
    found, heading, lines = [], None, []
    for line in text.split("\n"):
        if HEADING.match(line):
            if heading is not None or "".join(lines).strip():
                found.append((heading or LEAD, "\n".join(lines)))
            heading, lines = line, [line]
        else:
            lines.append(line)
    if heading is not None or "".join(lines).strip():
        found.append((heading or LEAD, "\n".join(lines)))
    return found


def section_findings(path, text):
    """A1: each section of `text` that names a Telegram surface and not that it is deprecated."""
    return [
        f"{path}: {heading}: names a Telegram surface and not that it is deprecated"
        for heading, body in sections(text)
        if SURFACE.search(body) and not DEPRECATED.search(body)
    ]


def paragraphs(text):
    """Each paragraph of `text` (the lines between blank lines), whitespace normalised."""
    return [flat(block) for block in re.split(r"\n[ \t]*\n", text) if block.strip()]


def note_findings(path, text):
    """A2: how `text` differs from holding exactly one paragraph that opens the dated note and
    names each of `NOTE_NAMES`."""
    notes = [block for block in paragraphs(text) if block.startswith(NOTE_OPENING)]
    if len(notes) != 1:
        return [f"{path}: {len(notes)} paragraphs open {NOTE_OPENING!r}, not one"]
    return [
        f"{path}: the dated note names no {name!r}" for name in NOTE_NAMES if name not in notes[0]
    ]


def kept_findings(path, text, count, pin):
    """A3: how `text` differs from its first `count` lines, each with its newline, hashing to
    `pin`, and the text after them holding the dated mark exactly once and naming each of
    `SPEC_NOTE_NAMES`, whitespace normalised."""
    lines = text.split("\n")
    head = "".join(f"{line}\n" for line in lines[:count])
    findings = []
    if text.count("\n") < count or hashlib.sha256(head.encode("utf-8")).hexdigest() != pin:
        findings.append(f"{path}: its first {count} lines are not the text the pin holds")
    after = flat("\n".join(lines[count:]))
    marks = after.count(NOTE_MARK)
    if marks != 1:
        findings.append(f"{path}: after line {count}, {NOTE_MARK!r} occurs {marks} times, not once")
        return findings
    return findings + [
        f"{path}: the note after line {count} names no {name!r}"
        for name in SPEC_NOTE_NAMES
        if name not in after
    ]


class ProductDocsTest(unittest.TestCase):
    def test_every_section_naming_a_telegram_surface_says_deprecated(self):
        """A1: every section of the PRD and the design system that names a Telegram surface also
        says it is deprecated."""
        planted = "# Planted\n\nA calm page.\n\n## Look\n\nHow the Mini App looks.\n"
        self.assertEqual(
            section_findings("planted.md", planted),
            ["planted.md: ## Look: names a Telegram surface and not that it is deprecated"],
        )
        marked = planted.replace("How the Mini App", "How the deprecated Mini App")
        self.assertEqual(
            section_findings("planted.md", marked), [], "the marked plant is the control"
        )
        for path in PRODUCT_DOCS:
            with self.subTest(document=str(path)):
                text = read(path)
                findings = section_findings(path, text)
                examined(f"sections of {path}", sections(text))
                self.assertEqual(findings, [])

    def test_each_product_doc_holds_one_dated_adr_589_note(self):
        """A2: each product document holds exactly one dated ADR-589 note, and it names the web
        app, the iPhone and iPad app and the deprecation."""
        note = f"{NOTE_OPENING}: the web app and the iPhone and\niPad app; the bot is deprecated."
        self.assertEqual(
            note_findings("planted.md", f"# Planted\n\n{note}\n"),
            [],
            "the planted note is the control",
        )
        self.assertEqual(
            note_findings("planted.md", "# Planted\n\nA calm page.\n"),
            [f"planted.md: 0 paragraphs open {NOTE_OPENING!r}, not one"],
        )
        self.assertEqual(
            note_findings("planted.md", f"# Planted\n\n{note}\n\n{note}\n"),
            [f"planted.md: 2 paragraphs open {NOTE_OPENING!r}, not one"],
        )
        self.assertEqual(
            note_findings("planted.md", f"# Planted\n\n{note.replace('deprecated', 'kept')}\n"),
            ["planted.md: the dated note names no 'deprecated'"],
        )
        for path in PRODUCT_DOCS:
            with self.subTest(document=str(path)):
                text = read(path)
                findings = note_findings(path, text)
                examined(f"paragraphs of {path}", paragraphs(text))
                self.assertEqual(findings, [])

    def test_each_campaign_spec_keeps_its_text_and_gains_one_dated_note(self):
        """A3: each campaign spec's earlier lines hash to their pin, and the text after them holds
        one dated note naming the web app, the iPhone and iPad app, the deprecation and Rex."""
        kept = "".join(f"{line}\n" for line in ("# Planted", "", "A kept line."))
        pin = hashlib.sha256(kept.encode("utf-8")).hexdigest()
        note = (
            f"- **A planted note {NOTE_MARK}.** The web app and the iPhone and iPad app; the bot\n"
            "  is deprecated, and Rex schedules.\n"
        )
        self.assertEqual(
            kept_findings("planted.md", kept + note, 3, pin), [], "the planted note is the control"
        )
        self.assertEqual(
            kept_findings("planted.md", kept.replace("kept", "kepT") + note, 3, pin),
            ["planted.md: its first 3 lines are not the text the pin holds"],
        )
        self.assertEqual(
            kept_findings("planted.md", kept, 3, pin),
            [f"planted.md: after line 3, {NOTE_MARK!r} occurs 0 times, not once"],
        )
        reached = []
        for path, count, spec_pin in CAMPAIGN_SPECS:
            with self.subTest(document=str(path)):
                findings = kept_findings(path, read(path), count, spec_pin)
                reached.append(path)
                self.assertEqual(findings, [])
        self.assertEqual(len(examined("campaign specs", reached)), len(CAMPAIGN_SPECS))


if __name__ == "__main__":
    unittest.main()

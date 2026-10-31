"""DeckStreak's identity text: the README's opening and its Built on section, the root NOTICE and
AGENTS.md's overview line (SPEC-580 A1 to A9; ADR-599 D2 and D3).

The README's opening, every line before its first level-2 heading, names no Anki, no Telegram, no
bot and no Mini App, and its first paragraph is the repository's description. Its Built on section
names each thing DeckStreak still takes from Anki, with a trademark line that names no holder.
NOTICE names what DeckStreak is, the licences and the trademarks. AGENTS.md's line 8 is the
identity line. Each refusal is planted over a complete tree this file writes, and that tree reads
green as the control.
"""

import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

README = Path("README.md")
NOTICE = Path("NOTICE")
AGENTS = Path("AGENTS.md")
# SPEC-580 R1: the words the opening never names.
BARRED = re.compile(r"(?i)anki|telegram|\bbots?\b|mini ?apps?")
# SPEC-580 R1: the repository's description, which the opening's first paragraph quotes exactly.
DESCRIPTION = (
    "A spaced-repetition study game for the web, iPhone and iPad: XP, streaks, quests and a"
    " morning reading from a mentor per subject that primes the day's new cards. Rex,"
    " DeckStreak's own memory model, becomes its only scheduler before 1.0."
)
BUILT_ON = "## Built on"
CONTENTS_LINE = "- [Built on](#built-on)"
# SPEC-580 R2: the bold lead each Built on item opens with, in order.
BUILT_ON_ITEMS = (
    "**The engine.**",
    "**The collection format and the sync protocol.**",
    "**DeckStreak's own sync server.**",
    "**AnkiMobile, until the switch.**",
    "**The name.**",
)
# SPEC-580 R2 and R4: the trademark line, which names no holder.
TRADEMARK = (
    "Anki® is a registered trademark, and DeckStreak is not affiliated with or endorsed by"
    " its holder."
)
# SPEC-580 R4: each item NOTICE names, by a sentence it holds.
NOTICE_ITEMS = {
    "what": "DeckStreak is a spaced-repetition study game for the web, iPhone and iPad.",
    "own licence": (
        "DeckStreak's own code is free software under the GNU Affero General Public License,"
        " version 3 or (at your option) any later version (AGPL-3.0-or-later)."
    ),
    "engine": (
        "DeckStreak includes Anki's Rust engine, through the maintainer's fork of it, pinned by"
        " commit in Cargo.toml. The engine is licensed under the GNU Affero General Public"
        " License, version 3 or later"
    ),
    "brand": (
        "no licence is granted to use them, as LICENSES/LicenseRef-DeckStreak-Brand.txt states."
    ),
    "DeckStreak's trademark": "The DeckStreak name and logo are trademarks of DeckStreak.",
    "Anki's trademark": TRADEMARK,
}
# SPEC-580 R5: AGENTS.md's line 8.
AGENTS_LINE_8 = (
    "DeckStreak is a spaced-repetition study game for the web, iPhone and iPad, with daily"
    " pre-study"
)


def flat(text):
    """`text` with every run of whitespace collapsed to one space."""
    return " ".join(text.split())


def read(root, path):
    """The text at `root`/`path`, or None when no such file exists."""
    full = Path(root) / path
    return full.read_text(encoding="utf-8") if full.is_file() else None


def reasons(findings):
    """(reason, path) of each finding."""
    return [tuple(finding.split(": ", 2)[:2]) for finding in findings]


def opening_findings(root):
    """Every way the README's opening differs from SPEC-580 R1, as
    "<reason>: README.md: <detail>"."""
    lines = (read(root, README) or "").split("\n")
    end = next((n for n, line in enumerate(lines) if line.startswith("## ")), len(lines))
    findings = [
        f"barred-word: README.md: line {n + 1} names {match.group(0)!r}"
        for n, line in enumerate(lines[:end])
        for match in BARRED.finditer(line)
    ]
    blocks = [flat(block) for block in "\n".join(lines[:end]).split("\n\n")]
    prose = [block for block in blocks if block and block[0] not in "#<>!"]
    if not prose or prose[0] != DESCRIPTION:
        first = prose[0][:60] if prose else ""
        findings.append(f"description-missing: README.md: the first paragraph is {first!r}")
    return findings


def built_on_findings(root):
    """Every way the README's Built on section differs from SPEC-580 R2, as
    "<reason>: README.md: <detail>"."""
    lines = (read(root, README) or "").split("\n")
    if BUILT_ON not in lines:
        return ["built-on-missing: README.md: no `## Built on` heading"]
    start = lines.index(BUILT_ON) + 1
    end = next((n for n in range(start, len(lines)) if lines[n].startswith("## ")), len(lines))
    section = lines[start:end]
    findings = [
        f"built-on-item-missing: README.md: no item opens with {lead}"
        for lead in BUILT_ON_ITEMS
        if not any(line.startswith(f"- {lead}") for line in section)
    ]
    if TRADEMARK not in flat("\n".join(section)):
        findings.append("trademark-line: README.md: the Built on section lacks the trademark line")
    if CONTENTS_LINE not in lines:
        findings.append(f"contents-missing: README.md: no contents line {CONTENTS_LINE!r}")
    return findings


def notice_findings(root):
    """Every way NOTICE differs from SPEC-580 R4, as "<reason>: NOTICE: <detail>"."""
    text = read(root, NOTICE)
    if text is None:
        return ["notice-missing: NOTICE: no NOTICE at the root"]
    flattened = flat(text)
    return [
        f"notice-item-missing: NOTICE: {name}"
        for name, sentence in NOTICE_ITEMS.items()
        if sentence not in flattened
    ]


def agents_findings(root):
    """Every way AGENTS.md's line 8 differs from SPEC-580 R5, as
    "<reason>: AGENTS.md: <detail>"."""
    lines = (read(root, AGENTS) or "").split("\n") + [""] * 8
    if lines[7] != AGENTS_LINE_8:
        return [f"agents-line-8: AGENTS.md: line 8 is {lines[7][:60]!r}"]
    return []


def planted_readme():
    """A README whose opening is the description and whose Built on section is complete."""
    items = [f"- {lead} A planted item." for lead in BUILT_ON_ITEMS[:-1]]
    items.append(f"- {BUILT_ON_ITEMS[-1]} {TRADEMARK}")
    header = ["# DeckStreak", "", DESCRIPTION, "", "## Contents", "", CONTENTS_LINE, ""]
    return "\n".join([*header, BUILT_ON, "", *items, ""])


PLANTED_NOTICE = "\n".join(NOTICE_ITEMS.values()) + "\n"
PLANTED_AGENTS = "\n".join(
    [
        "# AGENTS.md",
        "",
        "A planted line.",
        "",
        "## Overview",
        "",
        "",
        AGENTS_LINE_8,
        "A planted line.",
    ]
)


def plant(directory):
    """A complete planted tree under `directory`: the README, NOTICE and AGENTS.md."""
    root = Path(directory)
    (root / README).write_text(planted_readme(), encoding="utf-8")
    (root / NOTICE).write_text(PLANTED_NOTICE, encoding="utf-8")
    (root / AGENTS).write_text(PLANTED_AGENTS + "\n", encoding="utf-8")
    return root


class TheReadmeOpening(unittest.TestCase):
    def test_the_readme_opening_is_the_identity(self):
        # A1 (R1).
        self.assertIn(DESCRIPTION, flat(read(REPO, README)))
        self.assertEqual(opening_findings(REPO), [])

    def test_an_opening_naming_a_barred_word_is_refused(self):
        # A2 (R1): each barred word in a second paragraph of the opening, in turn.
        for word in examined("barred words", ["Anki", "Telegram", "bot", "Mini App"]):
            with self.subTest(word), tempfile.TemporaryDirectory() as tmp:
                root = plant(tmp)
                self.assertEqual(opening_findings(root), [], "the planted README is the control")
                planted = f"{DESCRIPTION}\n\nStudy with the {word} here."
                text = planted_readme().replace(DESCRIPTION, planted)
                (root / README).write_text(text, encoding="utf-8")
                self.assertEqual(reasons(opening_findings(root)), [("barred-word", "README.md")])


class TheBuiltOnSection(unittest.TestCase):
    def test_the_readme_built_on_names_each_item(self):
        # A3 (R2).
        text = read(REPO, README)
        found = examined(
            "Built on items", [lead for lead in BUILT_ON_ITEMS if f"\n- {lead}" in text]
        )
        self.assertEqual(found, list(BUILT_ON_ITEMS))
        self.assertEqual(built_on_findings(REPO), [])

    def test_a_built_on_section_missing_an_item_is_refused(self):
        # A4 (R2): each item's lead replaced, in turn.
        for lead in examined("Built on items", list(BUILT_ON_ITEMS)):
            with self.subTest(lead), tempfile.TemporaryDirectory() as tmp:
                root = plant(tmp)
                self.assertEqual(built_on_findings(root), [], "the planted README is the control")
                text = planted_readme().replace(f"- {lead}", "- **Another lead.**")
                (root / README).write_text(text, encoding="utf-8")
                found = reasons(built_on_findings(root))
                self.assertEqual(found, [("built-on-item-missing", "README.md")])

    def test_a_trademark_line_naming_a_holder_is_refused(self):
        # A5 (R2): the trademark line with a holder named in it.
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            named = TRADEMARK.replace("its holder.", "its holder, a planted company.")
            (root / README).write_text(planted_readme().replace(TRADEMARK, named), encoding="utf-8")
            self.assertEqual(reasons(built_on_findings(root)), [("trademark-line", "README.md")])


class TheNotice(unittest.TestCase):
    def test_notice_names_each_item(self):
        # A6 (R4).
        text = flat(read(REPO, NOTICE) or "")
        found = examined(
            "NOTICE items",
            [name for name, line in NOTICE_ITEMS.items() if line in text],
        )
        self.assertEqual(found, list(NOTICE_ITEMS))
        self.assertEqual(notice_findings(REPO), [])

    def test_a_missing_or_incomplete_notice_is_refused(self):
        # A7 (R4): no NOTICE, then each item replaced, in turn.
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            (root / NOTICE).unlink()
            self.assertEqual(reasons(notice_findings(root)), [("notice-missing", "NOTICE")])
        for name, sentence in examined("NOTICE items", list(NOTICE_ITEMS.items())):
            with self.subTest(name), tempfile.TemporaryDirectory() as tmp:
                root = plant(tmp)
                self.assertEqual(notice_findings(root), [], "the planted NOTICE is the control")
                text = PLANTED_NOTICE.replace(sentence, "A planted line.")
                (root / NOTICE).write_text(text, encoding="utf-8")
                found = reasons(notice_findings(root))
                self.assertEqual(found, [("notice-item-missing", "NOTICE")])


class TheAgentsOverview(unittest.TestCase):
    def test_agents_line_8_is_the_identity(self):
        # A8 (R5): line 8 in the tree, then a planted line 8 that frames another product.
        self.assertEqual(read(REPO, AGENTS).split("\n")[7], AGENTS_LINE_8)
        self.assertEqual(agents_findings(REPO), [])
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            lines = PLANTED_AGENTS.split("\n")
            lines[7] = "DeckStreak is a planted chat app."
            (root / AGENTS).write_text("\n".join(lines) + "\n", encoding="utf-8")
            self.assertEqual(reasons(agents_findings(root)), [("agents-line-8", "AGENTS.md")])


class TheControl(unittest.TestCase):
    def test_a_complete_planted_tree_reads_green(self):
        # A9: the control for A2, A4, A5 and A7.
        with tempfile.TemporaryDirectory() as tmp:
            root = plant(tmp)
            examined(
                "planted files",
                [path for path in (README, NOTICE, AGENTS) if (root / path).is_file()],
            )
            found = (
                opening_findings(root)
                + built_on_findings(root)
                + notice_findings(root)
                + agents_findings(root)
            )
            self.assertEqual(found, [])


if __name__ == "__main__":
    unittest.main()

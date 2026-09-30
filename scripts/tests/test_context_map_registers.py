"""The context map's registers count to their prose (#420).

Class rule: every count the Ownership register's prose states equals the unique names of the
register it describes, no name repeats inside one register, and a name that sits in both the
predecessor's register and DeckStreak's own has the same owning context in both. A count
sentence is a digit run beside `tables`, `rows` or `names`; a register is a markdown table whose
first header cell ends in `table`. A qualifier before that word
(`v9 table`) pairs the register with the sentences that say the same word before their number.
The section is read as GitHub renders it: every table in it is a register, and a table's rows run
to the first blank line or block, with or without their outer pipes. A blank line holds only spaces
and tabs, a block is one CommonMark opens (a heading, a fence, a quote, an HTML block, indented
code), a header has as many cells as its delimiter row, and a table inside a quote is a table.
Names compare as SQLite compares table names: the cell's one code span's text, trimmed, with ASCII
case folded, and nothing else folded. Counts are read from the prose as it renders.
The predecessor's register (`v9 table`) names exactly the predecessor's tables, a closed set,
because a name in both registers with the same owner cannot tell a carried table from one
only DeckStreak has.

`CONTEXT_MAP_PATH` names another copy of the file, so a population of altered copies can be judged
by this same check.
"""

import html
import os
import re
import unittest
from pathlib import Path

from _support import REPO, examined

MAP = Path(os.environ.get("CONTEXT_MAP_PATH") or REPO / "docs" / "CONTEXT-MAP.md")
COUNT = re.compile(r"(\d+)\s+(?:unique\s+)?(?:tables|rows|names)\b")
LEAD = 60
# The predecessor's tables: the 64 of its schema 24 (SPEC-001), as the register's first binding
# text named them (the context map of 2026-09-27). The predecessor is frozen, so the set is too, and
# a name outside it is not one of the predecessor's tables however the counts read.
PREDECESSOR = frozenset(
    {
        "badges_earned",
        "band_milestones",
        "beeminder_posts",
        "buffs",
        "can_do_unlocks",
        "celebration_log",
        "chests",
        "coaching_kv",
        "coin_ledger",
        "committed_windows",
        "contract_changes",
        "contract_days",
        "contracts",
        "cron_fires",
        "crown_days",
        "daily_lang_stats",
        "daily_rollup",
        "deck_names",
        "drill_xp_grants",
        "focus_log",
        "focus_timer",
        "freeze_events",
        "ghosts",
        "governor_state",
        "habit_strength",
        "hardmode_windows",
        "inventory",
        "language_progress",
        "leech_remediation",
        "leech_snapshot",
        "market_positions",
        "notifications",
        "nudge_ablation",
        "pardons",
        "penalty_ledger",
        "pity",
        "preread_notes",
        "preread_run_events",
        "preread_runs",
        "quest_offers",
        "quests",
        "race_results",
        "reading_log",
        "records",
        "review_cursor",
        "schema_versions",
        "season_nodes",
        "session_debrief",
        "settings_kv",
        "skip_card_snapshot",
        "skip_days",
        "sprints",
        "streak_state",
        "sync_runs",
        "tripwire_events",
        "tripwire_state",
        "wagers",
        "weekly_quests",
        "widget_state",
        "window_events",
        "writing_log",
        "xp_ledger",
        "xp_state",
        "xp_tokens",
    }
)


def section(text):
    """The text under the `## Ownership register` heading, up to the next `## ` heading."""
    parts = re.split(r"(?m)^## ", text)
    found = [p for p in parts if p.startswith("Ownership register")]
    if len(found) != 1:
        raise AssertionError(f"expected one Ownership register section, found {len(found)}")
    return found[0]


DELIMITER = re.compile(r"^ {0,3}\|?[ \t]*:?-+:?[ \t]*(\|[ \t]*:?-+:?[ \t]*)*\|?[ \t]*$")
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
HEADING = re.compile(r"^ {0,3}#{1,6}(?:[ \t]|$)")
QUOTE = re.compile(r"^ {0,3}> ?")
COMMENT = re.compile(r"^ {0,3}<!--")
LIST = re.compile(r"^ {0,3}(?:[-+*]|\d{1,9}[.)])( {1,4})(?=\S)")
HTML_BLOCK = re.compile(
    r"^ {0,3}</?(?:address|article|aside|blockquote|details|dialog|div|dl|fieldset|figure|footer"
    r"|form|h[1-6]|header|hr|li|main|nav|ol|p|pre|section|summary|table|tbody|td|th|thead|tr|ul)"
    r"(?:[ \t>]|/>|$)",
    re.IGNORECASE,
)
CODE = re.compile(r"(?<!`)(`+)(?!`)(.+?)(?<!`)\1(?!`)|<code>(.*?)</code>", re.DOTALL)
INLINE = (
    (re.compile(r"<!--.*?-->", re.DOTALL), ""),
    (re.compile(r"!?\[([^\]]*)\]\([^)]*\)"), r"\1"),
    (re.compile(r"<[^>]+>"), ""),
    (re.compile(r"[`*_~]"), ""),
)


def cells(line):
    """A table line's cells as GitHub renders them: outer pipes optional, an escaped pipe kept."""
    text = line.strip()
    text = text[1:] if text.startswith("|") else text
    text = text[:-1] if text.endswith("|") and not text.endswith("\\|") else text
    return [c.replace("\\|", "|").strip() for c in re.split(r"(?<!\\)\|", text)]


def fold(text):
    """ASCII case folding only: SQLite folds no other letter when it compares table names."""
    return "".join(chr(ord(c) + 32) if "A" <= c <= "Z" else c for c in text)


def key(cell):
    """A name as the cell renders and SQLite compares it: its one code span's text, else the cell's
    text; trimmed, with ASCII case folded."""
    spans = [m.group(2) if m.group(1) else m.group(3) for m in CODE.finditer(cell)]
    return fold((spans[0] if len(spans) == 1 else cell).strip())


def blank(line):
    """Blank as CommonMark reads it: spaces and tabs only, and no other whitespace."""
    return not line.strip(" \t")


def indent(line):
    """The line's indentation in columns, a tab reaching the next multiple of four."""
    column = 0
    for char in line:
        if char == " ":
            column += 1
        elif char == "\t":
            column += 4 - column % 4
        else:
            break
    return column


def unquote(line):
    """The line without its blockquote markers, and how many it had."""
    depth = 0
    while match := QUOTE.match(line):
        line, depth = line[match.end() :], depth + 1
    return depth, line


def fence_of(line):
    """The (character, length) of the code fence the line opens, or None: a backtick fence's info
    string holds no backtick."""
    match = FENCE.match(line)
    if match and (match.group(1)[0] == "~" or "`" not in match.group(2)):
        return match.group(1)[0], len(match.group(1))
    return None


def closes(line, fence):
    """Whether the line closes the fence: the same character, at least as long, nothing after."""
    match = FENCE.match(line)
    return bool(
        match
        and match.group(1)[0] == fence[0]
        and len(match.group(1)) >= fence[1]
        and blank(match.group(2))
    )


def interrupts(line):
    """A line that ends a table: blank, a heading, a fence, a quote, an HTML block, or code."""
    return (
        blank(line)
        or HEADING.match(line)
        or fence_of(line)
        or QUOTE.match(line)
        or COMMENT.match(line)
        or HTML_BLOCK.match(line)
        or indent(line) >= 4
    )


def contained(lines):
    """Each line as (container, content): the quote depth and list item it sits in, their markers
    and the item's indentation removed, so a table inside a quote or an item is still a table."""
    out, item, width = [], 0, None
    for raw in lines:
        depth, content = unquote(raw)
        if width is not None and (blank(content) or indent(content) >= width):
            content = "" if blank(content) else content[width:]
        else:
            width = None
            match = LIST.match(content)
            if match:
                item, width = item + 1, match.end()
                content = content[match.end() :]
        out.append(((depth, item if width is not None else 0), content))
    return out


def blocks(body):
    """The section as GitHub renders it: its tables as (header, rows), and its prose lines."""
    lines = body.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    entries = contained(lines)
    tables, prose = [], []
    index, fence, paragraph = 0, None, False
    while index < len(lines):
        depth, line = entries[index]
        if fence:
            fence = None if closes(line, fence) else fence
            index += 1
            continue
        if fence_of(line):
            fence, paragraph = fence_of(line), False
            index += 1
            continue
        if COMMENT.match(line):
            while index < len(lines) and "-->" not in lines[index]:
                index += 1
            index, paragraph = index + 1, False
            continue
        if HTML_BLOCK.match(line):
            while index < len(lines) and not blank(entries[index][1]):
                prose.append(entries[index][1])
                index += 1
            paragraph = False
            continue
        following = entries[index + 1] if index + 1 < len(lines) else (depth, "")
        if (
            not blank(line)
            and indent(line) < 4
            and "|" in line
            and following[0] == depth
            and DELIMITER.match(following[1])
            and len(cells(line)) == len(cells(following[1]))
        ):
            rows = []
            index += 2
            while index < len(lines):
                row_depth, row = entries[index]
                if row_depth != depth or interrupts(row):
                    break
                rows.append(row)
                index += 1
            tables.append((line, rows))
            paragraph = False
            continue
        if blank(line):
            paragraph = False
        elif paragraph or indent(line) < 4:
            prose.append(line)
            paragraph = True
        index += 1
    return tables, prose


def registers(body):
    """Each table of the section as (qualifier, [(name, owner)]), read as GitHub renders it."""
    found = []
    for header, rows in blocks(body)[0]:
        pairs = [(key((cells(row) + [""])[0]), key((cells(row) + [""])[1])) for row in rows]
        found.append((key(cells(header)[0]).removesuffix("table").strip(), pairs))
    return found


def count_sentences(body):
    """Each (number, words before it) the rendered prose states, tables and code left out."""
    text = " ".join(blocks(body)[1])
    for pattern, replacement in INLINE:
        text = pattern.sub(replacement, text)
    text = re.sub(r"\s+", " ", html.unescape(text))
    return [
        (int(m.group(1)), text[max(0, m.start() - LEAD) : m.start()]) for m in COUNT.finditer(text)
    ]


def pairing(sentences, regs):
    """Pair each sentence with a register by qualifier, and name what cannot be paired."""
    problems = []
    paired = {}
    qualifiers = [q for q, _ in regs if q]
    for number, lead in sentences:
        hits = [q for q in qualifiers if q in lead.split()[-6:] or f"{q}'s" in lead]
        if hits:
            key = hits[0]
        elif [q for q, _ in regs if not q]:
            key = ""
        else:
            problems.append(f"count sentence {number} pairs with no register")
            continue
        paired.setdefault(key, []).append(number)
    for qualifier, _ in regs:
        if qualifier and qualifier not in paired:
            problems.append(f"register '{qualifier} table' pairs with no count sentence")
    return problems, paired


def problems(text):
    """Every way the Ownership register disagrees with its prose, each named."""
    body = section(text)
    regs = registers(body)
    sentences = count_sentences(body)
    found, paired = pairing(sentences, regs)
    for qualifier, rows in regs:
        label = f"{qualifier} table" if qualifier else "table"
        names = [n for n, _ in rows]
        for name in sorted({n for n in names if names.count(n) > 1}):
            found.append(f"register '{label}' repeats {name}")
        for number in paired.get(qualifier, []):
            if number != len(set(names)):
                found.append(
                    f"register '{label}' has {len(set(names))} unique names, prose says {number}"
                )
    for qualifier, rows in regs:
        if qualifier != "v9":
            continue
        names = {n for n, _ in rows}
        for name in sorted(names - PREDECESSOR):
            found.append(f"register 'v9 table' names {name}, which is not one of the predecessor's")
        for name in sorted(PREDECESSOR - names):
            found.append(f"register 'v9 table' lacks the predecessor's {name}")
    owners = [dict(rows) for _, rows in regs]
    for name in sorted(set.intersection(*[set(o) for o in owners])) if len(owners) > 1 else []:
        if len({o[name] for o in owners}) > 1:
            found.append(f"{name} is in both registers with different owners")
    return found


class ContextMapRegistersCountToTheirProse(unittest.TestCase):
    def test_the_registers_count_to_their_prose(self):
        text = MAP.read_text(encoding="utf-8")
        body = section(text)
        regs = examined("registers", registers(body))
        sentences = examined("count sentences", count_sentences(body))
        self.assertEqual(
            len(regs), 2, "the section holds the predecessor's register and DeckStreak's"
        )
        self.assertIn("v9", [q for q, _ in regs], "the predecessor's register is qualified `v9`")
        self.assertEqual(len(PREDECESSOR), 64, "the predecessor's schema 24 holds 64 tables")
        self.assertEqual(problems(text), [])
        print(f"registers {[(q, len(r)) for q, r in regs]} count sentences {sentences}")


if __name__ == "__main__":
    unittest.main()

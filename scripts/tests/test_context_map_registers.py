"""The context map's registers count to their prose (#420).

Class rule: every count the Ownership register's prose states equals the unique names of the
register it describes, no name repeats inside one register, and a name that sits in both the
predecessor's register and DeckStreak's own is a carried table, so both registers give it the
same owning context. A count sentence is a digit run beside `tables`, `rows` or `names`; a
register is a markdown table whose first header cell ends in `table`. A qualifier before that word
(`v9 table`) pairs the register with the sentences that say the same word before their number.

`CONTEXT_MAP_PATH` names another copy of the file, so a population of altered copies can be judged
by this same check.
"""

import os
import re
import unittest
from pathlib import Path

from _support import REPO, examined

MAP = Path(os.environ.get("CONTEXT_MAP_PATH") or REPO / "docs" / "CONTEXT-MAP.md")
COUNT = re.compile(r"(\d+)\s+(?:unique\s+)?(?:tables|rows|names)\b")
LEAD = 60


def section(text):
    """The text under the `## Ownership register` heading, up to the next `## ` heading."""
    parts = re.split(r"(?m)^## ", text)
    found = [p for p in parts if p.startswith("Ownership register")]
    if len(found) != 1:
        raise AssertionError(f"expected one Ownership register section, found {len(found)}")
    return found[0]


def registers(body):
    """Each register as (qualifier, [(name, owner)]), read from the section's tables."""
    found = []
    lines = body.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        cells = [c.strip() for c in line.strip().strip("|").split("|")] if line[:1] == "|" else []
        if cells and cells[0].endswith("table") and index + 1 < len(lines):
            rows = []
            index += 2
            while index < len(lines) and lines[index][:1] == "|":
                row = [c.strip() for c in lines[index].strip().strip("|").split("|")]
                rows.append((row[0].strip("`"), row[1].strip("`")))
                index += 1
            found.append((cells[0].removesuffix("table").strip(), rows))
            continue
        index += 1
    return found


def count_sentences(body):
    """Each (number, words before it) the prose states, tables and code fences left out."""
    prose = re.sub(r"(?m)^\|.*$", "", re.sub(r"(?s)```.*?```", "", body))
    prose = re.sub(r"\s+", " ", prose)
    return [
        (int(m.group(1)), prose[max(0, m.start() - LEAD) : m.start()])
        for m in COUNT.finditer(prose)
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
        self.assertEqual(problems(text), [])
        print(f"registers {[(q, len(r)) for q, r in regs]} count sentences {sentences}")


if __name__ == "__main__":
    unittest.main()

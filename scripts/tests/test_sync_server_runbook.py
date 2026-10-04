"""The sync server's cutover runbook (SPEC-337 A9; R6; ADR-347 D6, D10, D11).

The schematic is the one source of the cutover: its state diagram orders the states and its table
says where each happens and whose go starts it. The runbook is read against both: one step per
state, in the diagram's order, each marked as the table marks it, and the hold on the full upload
named before the first step. Nothing runs; two files are read.
"""

import re
import unittest

from _support import REPO, examined

SCHEMATIC = REPO / "docs" / "schematics" / "sync-server-packaging-and-cutover.md"
RUNBOOK = REPO / "docs" / "runbooks" / "sync-server-cutover.md"
CUTOVER = "## The cutover as a state sequence"
OWNERS_GO = "the owner's go (#161)"
OWN_ACT = "the owner's own act"
START = "[*]"
ROLLBACK = "rolled_back"
EDGE = re.compile(r"^\s*(\[\*\]|\w+) --> (\[\*\]|\w+)", re.M)
ROW = re.compile(r"^\| `(\w+)` \| [^|]+ \| ([^|]+?) \|$", re.M)
STEP = re.compile(r"^### `(\w+)`", re.M)
# What R6 keeps out of the runbook: an address, a date and a URL that would name a host.
NAMES_A_HOST = (
    re.compile(r"\b\d{1,3}(?:\.\d{1,3}){3}\b"),
    re.compile(r"\b\d{4}-\d{2}-\d{2}\b"),
    re.compile(r"\bhttps?://"),
)


def section(text, heading):
    """The text under one `## ` heading, up to the next `## ` heading."""
    at = text.index(heading + "\n") + len(heading) + 1
    nxt = text.find("\n## ", at)
    return text[at:] if nxt < 0 else text[at:nxt]


def sequence(schematic):
    """The cutover's states in the diagram's order: the forward path from the start, then the
    rollback. Each state has one forward edge; a second one is refused, since it would leave the
    order undecided."""
    block = section(schematic, CUTOVER)
    diagram = block[block.index("```mermaid") : block.index("```\n", block.index("```mermaid") + 3)]
    forward = {}
    for source, target in EDGE.findall(diagram):
        if target == ROLLBACK or source == ROLLBACK:
            continue
        if source in forward:
            raise AssertionError(f"{source} has two forward edges, {forward[source]} and {target}")
        forward[source] = target
    states, at = [], START
    while forward.get(at, START) != START:
        at = forward[at]
        if at in states:
            raise AssertionError(f"the diagram returns to {at}")
        states.append(at)
    if any(ROLLBACK in edge for edge in EDGE.findall(diagram)):
        states.append(ROLLBACK)
    return states


def gates(schematic):
    """Each state's go, as the schematic's table marks it."""
    return dict(ROW.findall(section(schematic, CUTOVER)))


def steps(runbook):
    """Each step's state and its text, in the runbook's order."""
    found = list(STEP.finditer(runbook))
    return [
        (
            match.group(1),
            runbook[match.end() : found[n + 1].start() if n + 1 < len(found) else None],
        )
        for n, match in enumerate(found)
    ]


class TheCutoverRunbook(unittest.TestCase):
    def test_the_runbook_holds_every_state_of_the_cutover_in_order_each_host_step_the_owners_go(
        self,
    ):
        schematic = SCHEMATIC.read_text(encoding="utf-8")
        states = examined("cutover state(s)", sequence(schematic))
        table = gates(schematic)
        self.assertEqual(sorted(table), sorted(states), "the table names every state, and no other")
        for state, go in table.items():
            self.assertIn(go, (OWNERS_GO, OWN_ACT), f"{state}: the table's go is {go!r}")
        self.assertTrue(RUNBOOK.is_file(), f"no runbook at {RUNBOOK.relative_to(REPO)}")
        runbook = RUNBOOK.read_text(encoding="utf-8")
        found = steps(runbook)
        self.assertEqual([state for state, _ in found], states, "one step per state, in order")
        for state, text in examined("runbook step(s)", found):
            with self.subTest(state=state):
                self.assertIn(table[state], text, f"{state} is not marked {table[state]!r}")
                other = OWN_ACT if table[state] == OWNERS_GO else OWNERS_GO
                self.assertNotIn(other, text, f"{state} is also marked {other!r}")
        self.assertIn("\n## The hold\n", runbook, "the hold on the full upload has no section")
        self.assertLess(
            runbook.index("\n## The hold\n"), runbook.index("\n### `"), "the hold follows a step"
        )
        for pattern in NAMES_A_HOST:
            self.assertIsNone(pattern.search(runbook), f"the runbook holds {pattern.pattern}")


if __name__ == "__main__":
    unittest.main()

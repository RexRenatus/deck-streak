"""SPEC-434: FSM-TERMINAL, a declared terminal state is never left (ADR-448 D4).

The rule examines the reached states in a table's `terminal`, and one with a row that leaves it, a
self-loop included, is a hit. A sink not declared terminal is FSM-DEAD-END's hit and an unreached
terminal FSM-REACH's, so neither is this rule's. A9 reads the tree and prints its summary line,
which CI's log carries and no test pins. A10 reads a fixture tree under a temporary directory, its
hits stated as a literal.
"""

import unittest

import _fsm_tables
from _support import REPO, examined
from test_fsm_table_lints import LIB, SUMMARY, echoed, planted, register_text, table, tree_register

# A reached terminal with a row out, a reached terminal whose only row is a self-loop, a terminal
# sink, an unreached terminal with a row, and a reached sink not declared terminal.
TERMINALS = """\
pub enum HatchState {
    Start,
    Closed,
    Reopened,
    Spinning,
    Final,
    Attic,
    Basement,
    Loose,
}
"""
TERMINAL_TABLES = [
    table(
        "HatchState",
        ["Start"],
        ["Closed", "Spinning", "Final", "Attic"],
        [
            ("Start", "Closed", "close"),
            ("Closed", "Reopened", "reopen"),
            ("Reopened", "Spinning", "spin"),
            ("Spinning", "Spinning", "spin"),
            ("Reopened", "Final", "finish"),
            ("Attic", "Basement", "fall"),
            ("Reopened", "Loose", "drop"),
        ],
    ),
]
TERMINAL_HITS = [(LIB, 3, "HatchState::Closed"), (LIB, 5, "HatchState::Spinning")]


class EveryReachedTerminalIsNeverLeft(unittest.TestCase):
    def test_the_tree_reading_reads_every_reached_terminal(self):
        """A9: the tree's reading reads every table the register names and prints its summary
        line."""
        tables = examined("tables the register names", tree_register()["tables"])
        reading = _fsm_tables.read(REPO, "FSM-TERMINAL")
        lines = echoed(reading)
        self.assertRegex("\n".join(lines), SUMMARY.format(rule="FSM-TERMINAL", unit="terminals"))
        self.assertEqual(reading.tables, len(tables))

    def test_only_a_reached_terminal_with_an_outgoing_row_is_a_hit(self):
        """A10: a reached terminal with a row out and one with a self-loop are hits; a terminal
        sink, an unreached terminal with a row and a sink not declared terminal are not."""
        files = {LIB: TERMINALS, "config/fsm-tables.json": register_text(TERMINAL_TABLES)}
        with planted(files) as root:
            reading = _fsm_tables.read(root, "FSM-TERMINAL")
        self.assertEqual(sorted(reading.new), TERMINAL_HITS)


if __name__ == "__main__":
    unittest.main()

"""SPEC-434: FSM-DEAD-END, every reached state outside the terminal set has a row that leaves it
(ADR-448 D3).

The rule examines the reached states outside a table's `terminal`, so an unreached state is
FSM-REACH's alone, and a self-loop is a row. A7 reads the tree and prints its summary line, which
CI's log carries and no test pins. A8 reads a fixture tree under a temporary directory, its hits
stated as a literal.
"""

import unittest

import _fsm_tables
from _support import REPO, examined
from test_fsm_table_lints import LIB, SUMMARY, echoed, planted, register_text, table, tree_register

# A reached sink outside the terminal set, a terminal sink, an unreached sink, and a state whose
# only row is a self-loop.
DEAD_ENDS = """\
pub enum SinkState { Begin, Stuck, Done, Ghost }
pub enum RetryState { Trying }
"""
DEAD_END_TABLES = [
    table(
        "SinkState", ["Begin"], ["Done"], [("Begin", "Stuck", "fall"), ("Begin", "Done", "finish")]
    ),
    table("RetryState", ["Trying"], [], [("Trying", "Trying", "retry")]),
]
DEAD_END_HITS = [(LIB, 1, "SinkState::Stuck")]


class EveryReachedStateOutsideTheTerminalSetIsLeft(unittest.TestCase):
    def test_the_tree_reading_reads_every_reached_state_outside_the_terminal_set(self):
        """A7: the tree's reading reads every table the register names and prints its summary
        line."""
        tables = examined("tables the register names", tree_register()["tables"])
        reading = _fsm_tables.read(REPO, "FSM-DEAD-END")
        lines = echoed(reading)
        self.assertRegex("\n".join(lines), SUMMARY.format(rule="FSM-DEAD-END", unit="states"))
        self.assertEqual(reading.tables, len(tables))

    def test_only_a_reached_sink_outside_the_terminal_set_is_a_hit(self):
        """A8: a reached sink outside the terminal set is a hit; a terminal sink, an unreached
        sink and a state whose only row is a self-loop are not."""
        files = {LIB: DEAD_ENDS, "config/fsm-tables.json": register_text(DEAD_END_TABLES)}
        with planted(files) as root:
            reading = _fsm_tables.read(root, "FSM-DEAD-END")
        self.assertEqual(sorted(reading.new), DEAD_END_HITS)


if __name__ == "__main__":
    unittest.main()

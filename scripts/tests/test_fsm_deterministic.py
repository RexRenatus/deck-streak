"""SPEC-434: FSM-DETERMINISTIC, one state on one event moves to at most one state (ADR-448 D5).

The rule examines every distinct `(from, on)` pair of a table's rows, and a pair whose rows name
two or more `to` states is a hit at the enum's line. A11 reads the tree and prints its summary
line, which CI's log carries and no test pins. A12 reads a fixture tree under a temporary
directory, its hits stated as a literal.
"""

import unittest

import _fsm_tables
from _support import REPO, examined
from test_fsm_table_lints import LIB, SUMMARY, echoed, planted, register_text, table, tree_register

# One state on one event to two states, one state on two events, and two states on one event,
# each of the last two to two different states.
FORKS = "pub enum ForkState { Root, Left, Right, Other }\n"
FORK_TABLES = [
    table(
        "ForkState",
        ["Root"],
        ["Other"],
        [
            ("Root", "Left", "split"),
            ("Root", "Right", "split"),
            ("Left", "Other", "go"),
            ("Left", "Right", "turn"),
            ("Right", "Root", "go"),
        ],
    ),
]
FORK_HITS = [(LIB, 1, "ForkState::Root on split")]


class OneEventMovesOneStateOneWay(unittest.TestCase):
    def test_the_tree_reading_reads_every_state_and_event_pair(self):
        """A11: the tree's reading reads every distinct state and event pair of the register's
        rows and prints its summary line."""
        pairs = examined(
            "state and event pairs the register's rows hold",
            {
                (entry["file"], entry["enum"], move["from"], move["on"])
                for entry in tree_register()["tables"]
                for move in entry["moves"]
            },
        )
        reading = _fsm_tables.read(REPO, "FSM-DETERMINISTIC")
        lines = echoed(reading)
        self.assertRegex("\n".join(lines), SUMMARY.format(rule="FSM-DETERMINISTIC", unit="pairs"))
        self.assertGreaterEqual(len(reading.examined), len(pairs))

    def test_only_one_event_moving_one_state_two_ways_is_a_hit(self):
        """A12: one state on one event to two states is a hit naming the state and the event; one
        state on two events and two states on one event are not."""
        files = {LIB: FORKS, "config/fsm-tables.json": register_text(FORK_TABLES)}
        with planted(files) as root:
            reading = _fsm_tables.read(root, "FSM-DETERMINISTIC")
        self.assertEqual(sorted(reading.new), FORK_HITS)


if __name__ == "__main__":
    unittest.main()

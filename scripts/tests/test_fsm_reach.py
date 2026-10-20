"""SPEC-434: FSM-REACH, every state reached from its table's declared initial set (ADR-448 D2).

A table's reached set is its initial states and every state a row leads to from a reached one,
breadth first; a state outside it is a hit. A5 reads the tree and prints its summary line, which
CI's log carries and no test pins. A6 reads a fixture tree under a temporary directory, its hits
stated as a literal.
"""

import unittest

import _fsm_tables
from _support import REPO, examined
from test_fsm_table_lints import LIB, SUMMARY, echoed, planted, register_text, table, tree_register

# A chain, a cycle, a self-loop, two initial states, and a state no row enters that leads to one
# no other state leads to.
REACH = """\
pub enum ChainState { First, Second, Third }
pub enum LoopState { Up, Down }
pub enum SpinState { Turning }
pub enum PairState { Left, Right, Joined }
pub enum LostState { Start, Island, Shore }
"""
REACH_TABLES = [
    table(
        "ChainState",
        ["First"],
        ["Third"],
        [("First", "Second", "next"), ("Second", "Third", "next")],
    ),
    table("LoopState", ["Up"], [], [("Up", "Down", "flip"), ("Down", "Up", "flip")]),
    table("SpinState", ["Turning"], [], [("Turning", "Turning", "spin")]),
    table("PairState", ["Left", "Right"], ["Left", "Joined"], [("Right", "Joined", "join")]),
    table("LostState", ["Start"], ["Start", "Shore"], [("Island", "Shore", "swim")]),
]
REACH_HITS = [(LIB, 5, "LostState::Island"), (LIB, 5, "LostState::Shore")]


class EveryStateIsReachedFromTheInitialSet(unittest.TestCase):
    def test_the_tree_reading_reads_every_state_of_every_table(self):
        """A5: the tree's reading examines at least as many states as the register names, and
        prints its summary line."""
        named = examined(
            "states the register names",
            {
                (entry["file"], entry["enum"], state)
                for entry in tree_register()["tables"]
                for state in [
                    *entry["initial"],
                    *entry["terminal"],
                    *(move[end] for move in entry["moves"] for end in ("from", "to")),
                ]
            },
        )
        reading = _fsm_tables.read(REPO, "FSM-REACH")
        lines = echoed(reading)
        self.assertRegex("\n".join(lines), SUMMARY.format(rule="FSM-REACH", unit="states"))
        self.assertGreaterEqual(
            len(reading.examined),
            len(named),
            f"FSM-REACH examined {len(reading.examined)} states of {len(named)} the register names",
        )

    def test_only_a_state_no_walk_from_the_initial_set_reaches_is_a_hit(self):
        """A6: over a chain, a cycle, a self-loop and two initial states, only an unreached state
        and a state reached only from an unreached one are hits."""
        files = {LIB: REACH, "config/fsm-tables.json": register_text(REACH_TABLES)}
        with planted(files) as root:
            reading = _fsm_tables.read(root, "FSM-REACH")
        self.assertEqual(sorted(reading.new), REACH_HITS)


if __name__ == "__main__":
    unittest.main()

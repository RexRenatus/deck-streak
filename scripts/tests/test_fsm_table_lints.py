"""SPEC-434: the behaviour the five state-machine table rules share, and the oracle the graph
rules answer to (ADR-448).

A13 to A17 run over fixture trees written under a temporary directory, and A17 reads the tree's own
register for its levels. A18 and A19 read every table of a fixed population of small tables: every
move relation on three states with one event, with every non-empty initial set and every terminal
set (28,672 tables), and every move relation on two states with two events, from the first state
with the second terminal (256 tables). The oracle derives each rule's hits from the transitive
closure of a table's move relation, by Warshall's algorithm, and never calls the helper. Every
planted fixture states its hits and its refusals as literals.

The other four modules of these rules import this module's fixture writers and line readers. A
fixture reading's lines are kept out of the job log, so only the tree's own readings print
`FSM-LINT` lines there.
"""

import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path

import _fsm_tables
from _support import REPO, examined

RULES = ("FSM-TABLE", "FSM-REACH", "FSM-DEAD-END", "FSM-TERMINAL", "FSM-DETERMINISTIC")
GRAPH_RULES = ("FSM-REACH", "FSM-DEAD-END", "FSM-TERMINAL", "FSM-DETERMINISTIC")
ONE_DEFECT_RULES = ("FSM-REACH", "FSM-DEAD-END", "FSM-TERMINAL")
LIB = "crates/demo/src/lib.rs"
RULING = "docs/rulings/listing.md"
# A summary line, whatever a sibling's tree holds (R13).
SUMMARY = (
    r"(?m)^FSM-LINT {rule} level=(warn|fail) examined=[1-9][0-9]* {unit} hits=[0-9]+ "
    r"new=[0-9]+ listed=[0-9]+ stale=[0-9]+$"
)


def table(enum, initial, terminal, moves=(), file=LIB):
    """One table of a planted crate, each move a `(from, to, on)` row; a table with no moves says
    why."""
    entry = {
        "file": file,
        "enum": enum,
        "initial": list(initial),
        "terminal": list(terminal),
        "moves": [{"from": source, "to": target, "on": on} for source, target, on in moves],
    }
    if not moves:
        entry["no_moves"] = "planted with no moves"
    return entry


def register_text(tables, levels=None, listed=None):
    """A register's text: every rule at `warn` with no listed entry, unless `levels` or `listed`
    says otherwise for a rule."""
    levels, listed = levels or {}, listed or {}
    rules = {
        rule: {"level": levels.get(rule, "warn"), "listed": listed.get(rule, [])} for rule in RULES
    }
    return json.dumps({"rules": rules, "tables": tables}, indent=2) + "\n"


@contextlib.contextmanager
def planted(files):
    """A fixture tree under a temporary directory, holding `files` at their paths."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        for path, text in files.items():
            (root / path).parent.mkdir(parents=True, exist_ok=True)
            (root / path).write_text(text, encoding="utf-8")
        yield root


def printed(reading):
    """The lines `report` prints for a fixture's reading, kept out of the job log."""
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        _fsm_tables.report(reading)
    return out.getvalue().splitlines()


def echoed(reading):
    """The lines `report` prints for the tree's own reading, printed to the job log as well, where
    the promotion reader counts them."""
    out = io.StringIO()
    try:
        with contextlib.redirect_stdout(out):
            _fsm_tables.report(reading)
    finally:
        print(out.getvalue(), end="")
    return out.getvalue().splitlines()


def tree_register():
    """The tree's own register, read as JSON here and never by the helper."""
    return json.loads((REPO / "config" / "fsm-tables.json").read_text(encoding="utf-8"))


def small_tables():
    """The population of A18 and A19: each table as its states, initial set, terminal set and
    moves."""
    three = ("A", "B", "C")
    pairs = [(source, target) for source in three for target in three]
    subsets = [[state for bit, state in enumerate(three) if mask >> bit & 1] for mask in range(8)]
    for mask in range(1 << len(pairs)):
        moves = [
            (source, target, "go") for bit, (source, target) in enumerate(pairs) if mask >> bit & 1
        ]
        for initial in subsets[1:]:
            for terminal in subsets:
                yield three, initial, terminal, moves
    two = ("A", "B")
    rows = [(source, target, on) for source in two for on in ("go", "stop") for target in two]
    for mask in range(1 << len(rows)):
        yield two, ["A"], ["B"], [row for bit, row in enumerate(rows) if mask >> bit & 1]


def oracle(states, initial, terminal, moves):
    """Each graph rule's hits by its definition, from the transitive closure of the table's move
    relation."""
    index = {state: at for at, state in enumerate(states)}
    size = len(states)
    closure = [[False] * size for _row in range(size)]
    for source, target, _on in moves:
        closure[index[source]][index[target]] = True
    for middle in range(size):
        for source in range(size):
            if closure[source][middle]:
                for target in range(size):
                    if closure[middle][target]:
                        closure[source][target] = True
    reached = {
        state
        for state in states
        if state in initial or any(closure[index[start]][index[state]] for start in initial)
    }
    sources = {source for source, _target, _on in moves}
    targets = {}
    for source, target, on in moves:
        targets.setdefault((source, on), set()).add(target)
    return {
        "FSM-REACH": {state for state in states if state not in reached},
        "FSM-DEAD-END": {
            state for state in reached if state not in terminal and state not in sources
        },
        "FSM-TERMINAL": {state for state in reached if state in terminal and state in sources},
        "FSM-DETERMINISTIC": {pair for pair, found in targets.items() if len(found) > 1},
    }


def judged(states, initial, terminal, moves):
    """Each graph rule's hits as the helper reads them."""
    found = _fsm_tables.rules_over(states, initial, terminal, moves)
    return {rule: set(found[rule][1]) for rule in GRAPH_RULES}


# Three tables whose hits are stated by hand, which the oracle and the helper must each read.
SPOT_CHECKS = [
    (
        (("A", "B", "C"), ["A"], ["C"], [("A", "B", "go"), ("B", "A", "go")]),
        {
            "FSM-REACH": {"C"},
            "FSM-DEAD-END": set(),
            "FSM-TERMINAL": set(),
            "FSM-DETERMINISTIC": set(),
        },
    ),
    (
        (("A", "B", "C"), ["A"], ["B"], [("A", "B", "go"), ("A", "C", "go"), ("B", "B", "go")]),
        {
            "FSM-REACH": set(),
            "FSM-DEAD-END": {"C"},
            "FSM-TERMINAL": {"B"},
            "FSM-DETERMINISTIC": {("A", "go")},
        },
    ),
    (
        (("A", "B"), ["A"], ["B"], [("A", "B", "go"), ("A", "B", "stop"), ("B", "A", "stop")]),
        {
            "FSM-REACH": set(),
            "FSM-DEAD-END": set(),
            "FSM-TERMINAL": {"B"},
            "FSM-DETERMINISTIC": set(),
        },
    ),
]

GATE = "pub enum GateState {\n    Open,\n    Shut,\n    Lost,\n}\n"
GATE_TABLE = table("GateState", ["Open"], ["Open", "Shut"], [("Open", "Shut", "close")])
LEVELS = "pub enum GateState { Open, Shut }\npub enum DoorState { Open, Shut }\n"
DOOR_TABLE = table("DoorState", ["Open", "Shut"], ["Open", "Shut"])
GATE_PAIR_TABLE = table("GateState", ["Open", "Shut"], ["Open", "Shut"])
SITES = (
    "//! A planted crate whose hit lines name sites, never text.\n"
    "pub enum GateState {\n"
    "    Open,\n"
    "    Shut,\n"
    "    Lost, // the text a hit line never carries\n"
    "}\n"
)
ROW = {"from": "Open", "to": "Shut", "on": "close"}
VALID = {
    "rules": {rule: {"level": "warn", "listed": []} for rule in RULES},
    "tables": [table("GateState", ["Open"], ["Open"])],
}


class TheTableLintsShareOneReading(unittest.TestCase):
    def test_every_rule_is_void_with_nothing_to_examine(self):
        """A13: no rule reads a tree with no `crates/`, a tree with no register, or a tree with
        nothing to examine as a pass, and FSM-TABLE is VOID when its walk's count of product files
        differs from the independent listing's."""
        with planted({"config/fsm-tables.json": register_text([])}) as root:
            for rule in RULES:
                with self.assertRaisesRegex(_fsm_tables.Void, r"the tree has no crates/"):
                    _fsm_tables.read(root, rule)
        with planted({LIB: GATE}) as root:
            for rule in RULES:
                with self.assertRaisesRegex(
                    _fsm_tables.Void, r"the register config/fsm-tables\.json is absent"
                ):
                    _fsm_tables.read(root, rule)
        files = {LIB: "pub fn nothing() {}\n", "config/fsm-tables.json": register_text([])}
        with planted(files) as root:
            for rule in RULES:
                unit = _fsm_tables.UNITS[rule]
                with self.assertRaisesRegex(_fsm_tables.Void, f"VOID: examined 0 {unit}$"):
                    _fsm_tables.read(root, rule)
        files = {LIB: GATE, "config/fsm-tables.json": register_text([GATE_TABLE])}
        with planted(files) as root:
            with self.assertRaisesRegex(
                _fsm_tables.Void,
                r"its walk read 1 product file\(s\) and an independent listing 2$",
            ):
                _fsm_tables.read(
                    root, "FSM-TABLE", listing=lambda tree: [LIB, "crates/demo/src/ghost.rs"]
                )

    def test_a_listed_entry_needs_a_ruling_that_exists_and_reads_stale_without_its_hit(self):
        """A14: a listed entry is refused without a ruling, with one naming no file and with one
        outside `docs/rulings/`; with one that exists it lists its hit, and a listed entry no hit
        matches reads `stale` at its site."""
        refused = [
            ({"file": LIB, "key": "GateState::Lost"}, r"carries no ruling"),
            (
                {"file": LIB, "key": "GateState::Lost", "ruling": "docs/rulings/absent.md"},
                r"names docs/rulings/absent\.md, which does not exist",
            ),
            (
                {"file": LIB, "key": "GateState::Lost", "ruling": "docs/specs/elsewhere.md"},
                r"names docs/specs/elsewhere\.md, outside docs/rulings/",
            ),
        ]
        rulings = {RULING: "A ruling.\n", "docs/specs/elsewhere.md": "Not a ruling.\n"}
        for entry, refusal in refused:
            listed = {"FSM-REACH": [entry]}
            files = {LIB: GATE, "config/fsm-tables.json": register_text([GATE_TABLE], None, listed)}
            with planted({**files, **rulings}) as root:
                with self.assertRaisesRegex(_fsm_tables.Refused, refusal):
                    _fsm_tables.read(root, "FSM-REACH")
        listed = {
            "FSM-REACH": [
                {"file": LIB, "key": "GateState::Lost", "ruling": RULING},
                {"file": LIB, "key": "GateState::Shut", "ruling": RULING},
            ]
        }
        files = {LIB: GATE, "config/fsm-tables.json": register_text([GATE_TABLE], None, listed)}
        with planted({**files, **rulings}) as root:
            reading = _fsm_tables.read(root, "FSM-REACH")
            self.assertEqual(reading.new, [])
            self.assertEqual(reading.listed, [(LIB, 4, "GateState::Lost")])
            self.assertEqual(reading.stale, [(LIB, 3, "GateState::Shut")])
            self.assertEqual(
                printed(reading),
                [
                    "FSM-LINT FSM-REACH level=warn examined=3 states hits=1 new=0 listed=1 stale=1",
                    "FSM-LINT FSM-REACH warn listed crates/demo/src/lib.rs:4 GateState::Lost",
                    "FSM-LINT FSM-REACH warn stale crates/demo/src/lib.rs:3 GateState::Shut",
                ],
            )

    def test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them(self):
        """A15: at `fail` a `new` hit fails and a `stale` one fails; at `warn` each prints its line
        and passes."""
        new = [DOOR_TABLE]
        stale = [DOOR_TABLE, GATE_PAIR_TABLE]
        listed = {"FSM-TABLE": [{"file": LIB, "key": "DoorState", "ruling": RULING}]}
        rulings = {RULING: "A ruling.\n"}

        def lines(tables, level, entries):
            """The lines FSM-TABLE's reading of the planted crate prints at `level`."""
            text = register_text(tables, {"FSM-TABLE": level}, entries)
            with planted({LIB: LEVELS, "config/fsm-tables.json": text, **rulings}) as root:
                return printed(_fsm_tables.read(root, "FSM-TABLE"))

        with self.assertRaisesRegex(_fsm_tables.Failed, r"at fail and reads 1 new and 0 stale"):
            lines(new, "fail", None)
        self.assertIn(
            "FSM-LINT FSM-TABLE warn new crates/demo/src/lib.rs:1 GateState",
            lines(new, "warn", None),
        )
        with self.assertRaisesRegex(_fsm_tables.Failed, r"at fail and reads 0 new and 1 stale"):
            lines(stale, "fail", listed)
        self.assertIn(
            "FSM-LINT FSM-TABLE warn stale crates/demo/src/lib.rs:2 DoorState",
            lines(stale, "warn", listed),
        )

    def test_a_hit_line_names_its_site_and_never_its_source(self):
        """A16: a hit line carries its rule, level, kind, path, line and key, and none of the text
        of the line it names."""
        with planted({LIB: SITES, "config/fsm-tables.json": register_text([])}) as root:
            table_lines = printed(_fsm_tables.read(root, "FSM-TABLE"))
        self.assertEqual(
            table_lines,
            [
                "FSM-LINT FSM-TABLE level=warn examined=1 state-enums "
                "hits=1 new=1 listed=0 stale=0",
                "FSM-LINT FSM-TABLE warn new crates/demo/src/lib.rs:2 GateState",
            ],
        )
        files = {LIB: SITES, "config/fsm-tables.json": register_text([GATE_TABLE])}
        with planted(files) as root:
            reach_lines = printed(_fsm_tables.read(root, "FSM-REACH"))
        self.assertEqual(
            reach_lines,
            [
                "FSM-LINT FSM-REACH level=warn examined=3 states hits=1 new=1 listed=0 stale=0",
                "FSM-LINT FSM-REACH warn new crates/demo/src/lib.rs:5 GateState::Lost",
            ],
        )
        for line in table_lines + reach_lines:
            for text in ("pub enum", "Lost,", "never carries", "planted crate"):
                self.assertNotIn(text, line)

    def test_the_register_names_five_rules_at_warn_and_refuses_the_malformed(self):
        """A17: the tree's register names exactly the five rules, each at `warn`, and the reader
        refuses an unknown rule, level or key, a missing key, an empty initial set, a duplicate
        table, a duplicate row, a table with no moves and no `no_moves`, and one with moves and a
        `no_moves`."""
        levels = {
            rule: entry["level"] for rule, entry in _fsm_tables.register(REPO)["rules"].items()
        }
        self.assertEqual(levels, {rule: "warn" for rule in RULES})

        def unknown_rule(data):
            data["rules"]["FSM-EXTRA"] = {"level": "warn", "listed": []}

        def unknown_level(data):
            data["rules"]["FSM-REACH"]["level"] = "loud"

        def unknown_key(data):
            data["tables"][0]["colour"] = "red"

        def missing_key(data):
            del data["tables"][0]["terminal"]

        def empty_initial(data):
            data["tables"][0]["initial"] = []

        def duplicate_table(data):
            data["tables"].append(copy.deepcopy(data["tables"][0]))

        def duplicate_row(data):
            data["tables"][0]["moves"] = [dict(ROW), dict(ROW)]
            data["tables"][0]["terminal"] = ["Shut"]
            del data["tables"][0]["no_moves"]

        def no_reason(data):
            del data["tables"][0]["no_moves"]

        def moves_and_reason(data):
            data["tables"][0]["moves"] = [dict(ROW)]

        malformed = [
            (unknown_rule, r"rules are not exactly FSM-TABLE, FSM-REACH"),
            (unknown_level, r"FSM-REACH's level 'loud' is neither warn nor fail"),
            (unknown_key, r"table 1 holds an unknown key: colour"),
            (missing_key, r"table 1 has no 'terminal'"),
            (empty_initial, r"table 1 has an empty initial set"),
            (duplicate_table, r"two tables name GateState in crates/demo/src/lib\.rs"),
            (duplicate_row, r"table 1 holds the row Open Shut close twice"),
            (no_reason, r"table 1 has no moves and no reason in no_moves"),
            (moves_and_reason, r"table 1 has moves and a no_moves"),
        ]
        for change, refusal in malformed:
            data = copy.deepcopy(VALID)
            change(data)
            text = json.dumps(data, indent=2)
            with planted({"config/fsm-tables.json": text}) as root:
                with self.assertRaisesRegex(_fsm_tables.Refused, refusal):
                    _fsm_tables.register(root)

    def test_every_graph_rule_agrees_with_an_independent_oracle_over_every_small_table(self):
        """A18: over every small table, each graph rule's hits equal the oracle's; the oracle
        itself reads three tables whose hits are stated by hand."""
        for (states, initial, terminal, moves), hits in SPOT_CHECKS:
            self.assertEqual(oracle(states, initial, terminal, moves), hits)
            self.assertEqual(judged(states, initial, terminal, moves), hits)
        population = examined("small tables, each judged by the oracle", list(small_tables()))
        self.assertEqual(len(population), 28928)
        for states, initial, terminal, moves in population:
            expected = oracle(states, initial, terminal, moves)
            found = judged(states, initial, terminal, moves)
            if found != expected:
                self.fail(
                    f"over {states} from {initial} with {terminal} terminal and {moves}: "
                    f"the helper reads {found}, the oracle {expected}"
                )

    def test_no_state_is_a_hit_of_two_graph_rules(self):
        """A19: over the same population, no state is a hit of two of FSM-REACH, FSM-DEAD-END
        and FSM-TERMINAL, and each of the three has hits there, so the check is not vacuous."""
        population = examined("small tables", list(small_tables()))
        counts = {rule: 0 for rule in ONE_DEFECT_RULES}
        for states, initial, terminal, moves in population:
            found = _fsm_tables.rules_over(states, initial, terminal, moves)
            seen = {}
            for rule in ONE_DEFECT_RULES:
                for state in found[rule][1]:
                    if state in seen:
                        self.fail(
                            f"{state} is a hit of {seen[state]} and of {rule} over {states} "
                            f"from {initial} with {terminal} terminal and {moves}"
                        )
                    seen[state] = rule
                counts[rule] += len(found[rule][1])
        for rule, count in counts.items():
            examined(f"{rule} hits over the small tables", range(count))


if __name__ == "__main__":
    unittest.main()

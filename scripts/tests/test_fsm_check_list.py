"""FSM-CHECK-LIST (SPEC-433 R3, R4, R5 and R11; ADR-447 D1): each stored state column's CHECK list
against its enum's spellings, over the schema SQLite builds from `migrations/*.sql`.

A1 reads this repository's own tree and prints the rule's lines into the job's log. A2 to A6 each
read a fixture tree written under a temporary directory, and print nothing."""

import tempfile
import unittest
from pathlib import Path

import _fsm_lints
from _support import REPO, examined

#: The register's seeded entries (SPEC-433 section 4), frozen here. R2 admits an `unpaired` entry
#: with no `ruling` only when SEED holds it. PAIRS are the seven pairs this rule landed with, which
#: the tree reading must read among the register's.
SEED = (("notification_queue", "state"), ("inbox_captures", "state"))
PAIRS = (
    ("sync_runs", "status"),
    ("chests", "state"),
    ("skip_days", "state"),
    ("reading_topic_days", "state"),
    ("cron_fires", "last_outcome"),
    ("reading_runs", "outcome"),
    ("agent_runs", "verdict"),
)
SUMMARY = (
    r"^FSM-LINT FSM-CHECK-LIST level=(warn|fail) examined=[1-9][0-9]* columns"
    r" hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$"
)

#: A2: one pair spelled by its fn, with a doubled quote, a literal holding a comma and a variant
#: never stored, and one spelled in snake case; each drifts from its list by one spelling each way.
DRIFT_TREE = {
    "migrations/001_lamps.sql": """\
CREATE TABLE lamps (
    id INTEGER PRIMARY KEY,
    state TEXT NOT NULL CHECK (state IN ('on', 'o''ff', 'dim, low', 'warm'))
);
CREATE TABLE taps (
    id INTEGER PRIMARY KEY,
    step TEXT NOT NULL CHECK (step IN ('no_flow', 'full_flow', 'drop'))
);
""",
    "crates/home/src/lamp.rs": """\
pub enum LampState {
    On,
    Off,
    Dim,
    Glow,
    Hot,
}

impl LampState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "o'ff",
            Self::Dim => "dim, low",
            Self::Glow => "glow",
            Self::Hot => "hot",
        }
    }
}
""",
    "crates/home/src/tap.rs": """\
pub enum TapStep {
    NoFlow,
    FullFlow,
    Drip,
}
""",
}
DRIFT_RULE = {
    "level": "warn",
    "pairs": [
        {
            "table": "lamps",
            "column": "state",
            "enum": "LampState",
            "file": "crates/home/src/lamp.rs",
            "spelling": {"fn": "name"},
            "unstored": [{"variant": "Hot", "why": "a lamp too hot to read is never stored"}],
        },
        {
            "table": "taps",
            "column": "step",
            "enum": "TapStep",
            "file": "crates/home/src/tap.rs",
            "spelling": "snake_case",
        },
    ],
    "unpaired": [],
}
DRIFT_HITS = {
    (
        "new",
        "migrations/001_lamps.sql",
        3,
        ("column=lamps.state", "lacks=glow", "unspelled=warm"),
    ),
    (
        "new",
        "migrations/001_lamps.sql",
        7,
        ("column=taps.step", "lacks=drip", "unspelled=drop"),
    ),
}

#: A3: a table rebuilt under a new name, the old one renamed away and the new one renamed into
#: place, then given an added column.
REBUILT_TREE = {
    "migrations/0001_jobs.sql": """\
CREATE TABLE jobs (
    id INTEGER PRIMARY KEY,
    state TEXT NOT NULL CHECK (state IN ('queued', 'done'))
);
""",
    "migrations/0002_jobs_rebuilt.sql": """\
CREATE TABLE jobs_new (
    id INTEGER PRIMARY KEY,
    state TEXT NOT NULL CHECK (state IN ('queued', 'running', 'done'))
);
INSERT INTO jobs_new (id, state) SELECT id, state FROM jobs;
ALTER TABLE jobs RENAME TO jobs_retired;
ALTER TABLE jobs_new RENAME TO jobs;
""",
    "migrations/0003_jobs_mode.sql": """\
ALTER TABLE jobs ADD COLUMN mode TEXT CHECK (mode IN ('fast', 'slow'));
""",
}
REBUILT_COLUMNS = {
    ("jobs", "state"): ("text", ("queued", "running", "done")),
    ("jobs", "mode"): ("text", ("fast", "slow")),
    ("jobs_retired", "state"): ("text", ("queued", "done")),
}
REBUILT_HITS = {
    ("new", "migrations/0001_jobs.sql", 3, ("column=jobs_retired.state",)),
    ("new", "migrations/0002_jobs_rebuilt.sql", 3, ("column=jobs.state",)),
    ("new", "migrations/0003_jobs_mode.sql", 1, ("column=jobs.mode",)),
}

#: A4: one whole-body domain, beside a CHECK with text before its column, one with text after its
#: list, a CHECK commented out by each comment form, and an integer list.
DOMAIN_TREE = {
    "migrations/001_gauges.sql": """\
CREATE TABLE gauges (
    id INTEGER PRIMARY KEY,
    status TEXT NOT NULL CHECK (status IN ('on', 'off')),
    phase TEXT CHECK (verdict IS NULL AND phase IN ('a', 'b')),
    stage TEXT CHECK (stage IN ('a', 'b') OR stage IS NULL),
    -- outcome TEXT CHECK (outcome IN ('x', 'y')),
    /* step TEXT CHECK (step IN ('x', 'y')), */
    mode INTEGER NOT NULL CHECK (mode IN (0, 1)),
    verdict TEXT
);
""",
}
DOMAIN_COLUMNS = {
    ("gauges", "status"): ("text", ("on", "off")),
    ("gauges", "mode"): ("integer", ("0", "1")),
}
DOMAIN_HITS = {("new", "migrations/001_gauges.sql", 3, ("column=gauges.status",))}

#: A5: a state column of each of the eight names and of the `_` ending, none paired or listed, a
#: column no state name marks, and one state column listed `unpaired`.
STATE_TREE = {
    "migrations/001_machines.sql": """\
CREATE TABLE machines (
    id INTEGER PRIMARY KEY,
    state TEXT CHECK (state IN ('a', 'b')),
    status TEXT CHECK (status IN ('a', 'b')),
    outcome TEXT CHECK (outcome IN ('a', 'b')),
    verdict TEXT CHECK (verdict IN ('a', 'b')),
    phase TEXT CHECK (phase IN ('a', 'b')),
    stage TEXT CHECK (stage IN ('a', 'b')),
    step TEXT CHECK (step IN ('a', 'b')),
    mode TEXT CHECK (mode IN ('a', 'b')),
    last_state TEXT CHECK (last_state IN ('a', 'b')),
    colour TEXT CHECK (colour IN ('a', 'b'))
);
CREATE TABLE queues (
    id INTEGER PRIMARY KEY,
    state TEXT CHECK (state IN ('held', 'sent'))
);
""",
}
STATE_RULE = {
    "level": "warn",
    "pairs": [],
    "unpaired": [{"table": "queues", "column": "state", "why": "its store writes SQL literals"}],
}
STATE_SEED = (("queues", "state"),)
STATE_HITS = {
    ("new", "migrations/001_machines.sql", 3, ("column=machines.state",)),
    ("new", "migrations/001_machines.sql", 4, ("column=machines.status",)),
    ("new", "migrations/001_machines.sql", 5, ("column=machines.outcome",)),
    ("new", "migrations/001_machines.sql", 6, ("column=machines.verdict",)),
    ("new", "migrations/001_machines.sql", 7, ("column=machines.phase",)),
    ("new", "migrations/001_machines.sql", 8, ("column=machines.stage",)),
    ("new", "migrations/001_machines.sql", 9, ("column=machines.step",)),
    ("new", "migrations/001_machines.sql", 10, ("column=machines.mode",)),
    ("new", "migrations/001_machines.sql", 11, ("column=machines.last_state",)),
    ("listed", "migrations/001_machines.sql", 16, ("column=queues.state",)),
}

#: A6: one pair that reads, and one pair per absence R11 names.
ABSENT_TREE = {
    "migrations/001_t.sql": """\
CREATE TABLE t (
    id INTEGER PRIMARY KEY,
    state TEXT NOT NULL CHECK (state IN ('a', 'b'))
);
""",
    "crates/k/src/lib.rs": """\
pub enum TState {
    A,
    B,
}

const B_TEXT: &str = "b";

impl TState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => B_TEXT,
        }
    }
}
""",
}
PRESENT_PAIR = {
    "table": "t",
    "column": "state",
    "enum": "TState",
    "file": "crates/k/src/lib.rs",
    "spelling": {"fn": "name"},
}
ABSENT_PAIRS = (
    ("an absent table", dict(PRESENT_PAIR, table="nowhere")),
    ("an absent column", dict(PRESENT_PAIR, column="missing")),
    ("an absent enum", dict(PRESENT_PAIR, enum="Ghost")),
    ("an absent fn", dict(PRESENT_PAIR, spelling={"fn": "label"})),
    ("a spelling arm that is not a literal", dict(PRESENT_PAIR, spelling={"fn": "code"})),
)


def plant(root, files):
    """Write a fixture tree: each repository-relative path with its text."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def hits(reading):
    return {(hit.kind, hit.path, hit.line, hit.key) for hit in reading.hits}


def read(tree, rule, seed=()):
    """FSM-CHECK-LIST's reading of a fixture tree."""
    with tempfile.TemporaryDirectory() as scratch:
        plant(scratch, tree)
        return _fsm_lints.check_list(Path(scratch), rule, seed)


class CheckListLint(unittest.TestCase):
    def test_the_tree_reading_reads_every_migration_and_every_pair(self):
        body = _fsm_lints.read_register(REPO)["rules"]["FSM-CHECK-LIST"]
        reading = _fsm_lints.check_list(REPO, body, SEED)
        named = [(pair["table"], pair["column"]) for pair in body["pairs"]]
        self.assertEqual(reading.pairs, named)
        self.assertLessEqual(set(PAIRS), set(reading.pairs))
        listing = examined(
            "migrations",
            sorted(p for p in (REPO / "migrations").iterdir() if p.suffix == ".sql"),
        )
        self.assertEqual(reading.files, len(listing))
        self.assertEqual(reading.examined, len(reading.columns))
        printed = _fsm_lints.report(reading)
        self.assertRegex(printed[0], SUMMARY)
        self.assertEqual(len(printed), 1 + len(reading.hits))

    def test_a_planted_drift_names_its_column_and_each_missing_and_extra_spelling(self):
        reading = read(DRIFT_TREE, DRIFT_RULE)
        self.assertEqual(hits(reading), DRIFT_HITS)
        self.assertEqual(reading.pairs, [("lamps", "state"), ("taps", "step")])

    def test_a_rebuilt_and_renamed_table_is_read_as_the_last_migration_leaves_it(self):
        reading = read(REBUILT_TREE, {"level": "warn", "pairs": [], "unpaired": []})
        self.assertEqual(reading.columns, REBUILT_COLUMNS)
        self.assertEqual(hits(reading), REBUILT_HITS)
        self.assertEqual(reading.files, 3)

    def test_only_a_whole_body_check_is_a_column_domain(self):
        reading = read(DOMAIN_TREE, {"level": "warn", "pairs": [], "unpaired": []})
        self.assertEqual(reading.columns, DOMAIN_COLUMNS)
        self.assertEqual(hits(reading), DOMAIN_HITS)
        self.assertEqual(reading.examined, 2)

    def test_a_state_column_neither_paired_nor_listed_is_a_new_hit(self):
        reading = read(STATE_TREE, STATE_RULE, STATE_SEED)
        self.assertEqual(hits(reading), STATE_HITS)
        self.assertEqual(reading.examined, 11)

    def test_a_pair_naming_what_is_absent_is_void(self):
        for case, pair in ABSENT_PAIRS:
            with self.subTest(case=case), self.assertRaises(_fsm_lints.Void):
                read(ABSENT_TREE, {"level": "warn", "pairs": [pair], "unpaired": []})
        reading = read(ABSENT_TREE, {"level": "warn", "pairs": [PRESENT_PAIR], "unpaired": []})
        self.assertEqual(reading.pairs, [("t", "state")])
        self.assertEqual(hits(reading), set())


if __name__ == "__main__":
    unittest.main()

"""The query plan census (SPEC-658, ADR-680): each SQL statement the services prepare, read
through SQLite's EXPLAIN QUERY PLAN over the schema its migrations build, and each full scan of a
large table reported, at warn first, with today's sites listed.

The census reads every shipped Rust source (`crates/*/src/**/*.rs`, less test code) as text and
takes each string literal that opens with a statement's first word in capitals. It builds the
service's schema from `migrations/` and the collection store's from `crates/collection/migrations/`
in an in-memory SQLite, binds NULL to every parameter and reads each statement's plan. A finding
is a plan line that scans a large table, or builds an automatic index on one, in a statement that
constrains its rows (a WHERE, an ON or a USING). A plan line shows an aliased table by its alias,
which the statement's own FROM and JOIN items map back to its table; a name that maps to no table
is reported, never dropped. A statement with no constraint reads its tables whole by its own
text, so it is counted and is never a finding.

`PlantedTrees` judges the trees of `fixtures/query-plan-census/trees.json` and runs anywhere;
`Repository` judges this repository's own sources and migrations, and runs in CI.
"""

import json
import re
import sqlite3
import unittest
import warnings

from _support import REPO, examined

TREES = REPO / "scripts" / "tests" / "fixtures" / "query-plan-census" / "trees.json"

# The level (ADR-680 D5): at "warn" an unlisted finding prints its WARN line and passes; at "fail"
# it fails naming the site. A listed entry that matches no finding fails at both levels.
LEVEL = "warn"

# Where each schema's migrations live, and the sources whose statements run on the store; every
# other shipped source's statements run on the service's own database.
SERVICE_MIGRATIONS = "migrations"
STORE_MIGRATIONS = "crates/collection/migrations"
STORE_SOURCES = "crates/collection/src/"

# Sources whose statements run on the learner's own Anki file, which no migration of the tree
# builds and the services open read-only (ADR-680 D3): counted, never planned.
FOREIGN_SOURCES = {
    "crates/anki-format/src/": "the schema-18 reader and the defensive opener (SPEC-602)",
    "crates/engine-core/src/": "the engine's own connection to the learner's collection",
    "crates/ingest/src/search_parity.rs": "the search parity reads through the foreign handle",
}

# The tables whose rows the schema bounds, each with its migration's own words (ADR-680 D2). A
# one-row table, whose key a CHECK pins to 1, is small too; every other table is large.
SMALL = {
    "service": {
        "badges_earned": "one row per badge key and tier",
        "band_milestones": "one row per course and band reached",
        "instrument_reports": "one row per instrument",
        "language_progress": "one row per course",
        "notification_settings": "the owner's settings, by key",
        "passkeys": "one row per registered credential",
        "records": "one row per record kind",
        "relight_due": "a day leaves once the router has decided it",
        "sensitive_decks": "one row per marked deck",
        "streak_state": "one row per track",
    },
    "store": {
        "device_setting": "one row per setting key",
    },
}

# Today's findings (SPEC-658 section 1), each with why its plan scans. Each is listed, not cured,
# here: a cure is a product change of its own (SPEC-658 section 5).
LISTED = {
    "crates/collection/src/sql.rs review_log 0036055921": "kind and ease lead no index",
    "crates/collection/src/sql.rs review_log 1749300986": (
        "the DISTINCT on card_id walks review_log_card in its order instead of the id range"
    ),
    "crates/coordination/src/maintenance.rs cron_fires 0042281050": (
        "fire_date follows job_id in the table's key (SPEC-043 R16a)"
    ),
    "crates/economy/src/tariff.rs coin_ledger 2389345188": (
        "source and reference follow study_day in coin_ledger_one_movement"
    ),
    "crates/economy/src/wallet.rs coin_ledger 2780143425": (
        "source and reference follow study_day in coin_ledger_one_movement"
    ),
    "crates/economy/src/wallet.rs coin_ledger 4010787531": (
        "the page's first-page arm keeps the key from seeking; it reads in index order to its limit"
    ),
    "crates/habits/src/store.rs writing_log 0651436989": (
        "study_day follows code in the table's key"
    ),
    "crates/habits/src/store.rs writing_log 3068042599": (
        "study_day follows code in the table's key"
    ),
    "crates/ingest/src/sync_runs.rs sync_runs 0488525014": (
        "trigger and status follow study_day in sync_runs_by_study_day"
    ),
    "crates/ingest/src/sync_runs.rs sync_runs 0874718251": "status leads no index",
    "crates/ingest/src/sync_runs.rs sync_runs 3426342427": "status leads no index",
    "crates/notifications/src/ledger.rs in_app_feed 2419713313": "seen_at leads no index",
    "crates/notifications/src/ledger.rs in_app_feed 3713335016": "seen_at leads no index",
    "crates/notifications/src/ledger.rs notification_decisions 2350833558": (
        "arm, kind and study_day lead no index"
    ),
    "crates/notifications/src/ledger.rs notification_queue 0421539410": "state leads no index",
    "crates/notifications/src/ledger.rs notification_queue 1209561785": "state leads no index",
    "crates/notifications/src/ledger.rs notification_queue 1793498150": "state leads no index",
    "crates/notifications/src/ledger.rs notification_queue 2329724518": "state leads no index",
    "crates/notifications/src/ledger.rs notification_queue 3728200079": "state leads no index",
    "crates/notifications/src/ledger.rs notification_queue 3921118979": "state leads no index",
    "crates/progression/src/ledger.rs xp_ledger 1974305248": (
        "track follows source in xp_ledger_once and study_day in xp_ledger_per_day"
    ),
    "crates/progression/src/ledger.rs xp_settlement 1974305248": (
        "track follows study_day and source in xp_settlement_per_day"
    ),
    "crates/progression/src/settle.rs xp_settlement 3036714759": (
        "source and track follow study_day in xp_settlement_per_day"
    ),
    "crates/quests/src/chest_store.rs chests 0767112370": "state leads no index",
    "crates/quests/src/tokens.rs xp_tokens 2379248186": "activated_at leads no index",
    "crates/quests/src/tokens.rs xp_tokens 2769033599": "consumed and activated_at lead no index",
    "crates/quests/src/tokens.rs xp_tokens 3734706590": "consumed and activated_at lead no index",
    "crates/quests/src/tokens.rs xp_tokens 4087179258": "consumed and activated_at lead no index",
    "crates/quests/src/tokens.rs xp_tokens 4163372389": "consumed and activated_at lead no index",
    "crates/streaks/src/store.rs freeze_events 1174700539": (
        "reason follows study_day in freeze_events_fold_once, a partial index of other reasons"
    ),
}

STATEMENT = re.compile(r"(?:SELECT|INSERT|UPDATE|DELETE|WITH|REPLACE)\b")
CONSTRAINT = re.compile(r"\b(?:WHERE|ON|USING)\b", re.IGNORECASE)
QUOTED = re.compile(r"'(?:[^']|'')*'|\"(?:[^\"]|\"\")*\"")
SCAN = re.compile(r"SCAN (\w+)(?: AS \w+)?(?: USING (?:COVERING )?INDEX \w+)?")
AUTOMATIC = re.compile(r"SEARCH (\w+)(?: AS \w+)? USING AUTOMATIC .+")
ITEM = re.compile(r"\b(?:FROM|JOIN|UPDATE|INTO)\s+(\w+)(?:\s+AS\b)?(?:\s+(\w+))?", re.I)
KEYWORDS = frozenset(
    "ABORT AND AS CROSS DEFAULT DO ELSE END EXCEPT FAIL FROM FULL GROUP HAVING IGNORE INDEXED"
    " INNER INTERSECT JOIN LEFT LIMIT NATURAL NOT ON OR ORDER OUTER REPLACE RETURNING RIGHT"
    " ROLLBACK SELECT SET THEN UNION USING VALUES WHEN WHERE WINDOW".split()
)
NUMBERED = re.compile(r"\?(\d+)")
BINDINGS = "Incorrect number of bindings supplied"
ONE_ROW = re.compile(r"\bid\s+INTEGER\s+PRIMARY\s+KEY\s+CHECK\s*\(\s*id\s*=\s*1\s*\)", re.I)
TEST_GATE = re.compile(r"#\[(?:test|cfg\((?:test|any\([^\]]*\btest\b[^\]]*\))\))\]")
OUT_OF_LINE = re.compile(r"\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
TOKEN = re.compile(r"//|/\*|(\w*)(#*)\"|'")
ESCAPES = {"n": "\n", "r": "\r", "t": "\t", "\\": "\\", "0": "\0", "'": "'", '"': '"'}


def census(sources, migrations, level=LEVEL, listed=LISTED, small=SMALL, foreign=FOREIGN_SOURCES):
    """(lines, refused): the census of `sources` ({path: text}) over the schemas `migrations`
    builds ({"service": [(name, text)], "store": [(name, text)]}), at `level`."""
    summary = (
        "query-plan-census: examined 0 shipped source(s), 0 statement(s): 0 on the service, "
        "0 on the store, 0 on the learner's file (not planned); 0 read whole tables; "
        f"0 large table(s); SQLite {sqlite3.sqlite_version}"
    )
    return [summary], False


def repository_migrations(folder):
    """The migrations of `folder`, in the order the runner applies them: [(name, text)]."""
    paths = sorted((REPO / folder).glob("*.sql"))
    return [(path.name, path.read_text(encoding="utf-8")) for path in paths]


def planted(name):
    """A planted tree of trees.json: its Rust sources, and its migrations as the service's."""
    tree = json.loads(TREES.read_text(encoding="utf-8"))[name]
    sources = {path: text for path, text in tree.items() if path.endswith(".rs")}
    service = sorted((path, text) for path, text in tree.items() if path.endswith(".sql"))
    return sources, {"service": service, "store": []}


def judged(name, level="warn", listed=None, small=None, foreign=None):
    """The census of a planted tree with nothing listed, small or foreign unless given, its
    summary cut before the SQLite version, which the runner's library decides. A deprecation
    warning is an error here, so a bind a later Python refuses fails on this one too."""
    with warnings.catch_warnings():
        warnings.simplefilter("error", DeprecationWarning)
        lines, refused = census(
            *planted(name),
            level=level,
            listed=listed or {},
            small=small or {"service": {}, "store": {}},
            foreign=foreign or {},
        )
    return [lines[0].rsplit("; SQLite ", 1)[0], *lines[1:]], refused


def summary(shipped, found, service, foreign, whole, large):
    """The summary line a planted census prints, as `judged` cuts it."""
    return (
        f"query-plan-census: examined {shipped} shipped source(s), {found} statement(s): "
        f"{service} on the service, 0 on the store, {foreign} on the learner's file (not planned); "
        f"{whole} read whole tables; {large} large table(s)"
    )


class PlantedTrees(unittest.TestCase):
    """The census over the planted trees, each judged in memory; no tree is written to disk."""

    def test_a_statement_that_searches_by_an_index_is_not_reported(self):
        lines, refused = judged("searched")
        self.assertEqual(lines, [summary(1, 2, 2, 0, 0, 1)])
        self.assertFalse(refused)

    def test_the_level_decides_whether_a_full_scan_of_a_large_table_fails(self):
        found = [
            "full-scan crates/events/src/lib.rs events 1903069248: SCAN events",
            "full-scan crates/events/src/lib.rs events 4215932591: SCAN events",
            "full-scan crates/events/src/lib.rs kinds 4215932591: "
            "SEARCH kinds USING AUTOMATIC COVERING INDEX (name=?)",
        ]
        for level in examined("levels", ("warn", "fail")):
            lines, refused = judged("scanned", level=level)
            expected = [f"{level.upper()} {line}" for line in found]
            self.assertEqual(lines, [summary(1, 2, 2, 0, 0, 2), *expected])
            self.assertEqual(refused, level == "fail")

    def test_a_scan_of_a_small_table_is_not_reported(self):
        lines, refused = judged("small")
        switches = "WARN full-scan crates/settings/src/lib.rs switches 1895693771: SCAN switches"
        self.assertEqual(lines, [summary(1, 2, 2, 0, 0, 1), switches])
        self.assertFalse(refused)
        named = {"service": {"switches": "one row per switch key"}, "store": {}}
        lines, refused = judged("small", small=named)
        self.assertEqual(lines, [summary(1, 2, 2, 0, 0, 0)])
        self.assertFalse(refused)

    def test_a_listed_finding_passes_and_a_stale_entry_fails(self):
        key = "crates/events/src/lib.rs events 1903069248"
        lines, refused = judged("scanned", listed={key: "planted"})
        self.assertIn(f"LISTED {key}: SCAN events (planted)", lines)
        self.assertFalse(refused)
        stale = "crates/events/src/lib.rs events 0000000000"
        cases = [
            ({"listed": {stale: "planted"}}, f"STALE {stale}"),
            ({"small": {"service": {"gone": "planted"}, "store": {}}}, "STALE small service gone"),
            ({"foreign": {"crates/gone/src/": "planted"}}, "STALE foreign crates/gone/src/"),
        ]
        for given, line in examined("stale entries", cases):
            for level in ("warn", "fail"):
                lines, refused = judged("scanned", level=level, **given)
                self.assertIn(line, lines)
                self.assertTrue(refused)

    def test_an_empty_population_is_void(self):
        lines, refused = judged("empty")
        self.assertEqual(lines, [summary(1, 0, 0, 0, 0, 1), "VOID examined 0 planned statement(s)"])
        self.assertTrue(refused)

    def test_a_statement_that_reads_its_tables_whole_is_counted_and_never_reported(self):
        lines, refused = judged("whole")
        self.assertEqual(lines, [summary(1, 2, 2, 0, 2, 1)])
        self.assertFalse(refused)

    def test_test_code_comments_and_prose_are_not_read(self):
        lines, refused = judged("unread")
        self.assertEqual(lines, [summary(1, 1, 1, 0, 0, 1)])
        self.assertFalse(refused)

    def test_a_statement_on_the_learners_file_is_counted_and_never_planned(self):
        lines, refused = judged("foreign", foreign={"crates/reader/src/": "planted"})
        self.assertEqual(lines, [summary(2, 2, 1, 1, 0, 1)])
        self.assertFalse(refused)

    def test_a_statement_that_does_not_prepare_is_reported_by_the_level(self):
        for level in examined("levels", ("warn", "fail")):
            lines, refused = judged("unprepared", level=level)
            missing = f"{level.upper()} unprepared crates/events/src/lib.rs:1: no such table: gone"
            self.assertEqual(lines, [summary(1, 1, 1, 0, 0, 1), missing])
            self.assertEqual(refused, level == "fail")

    def test_a_full_scan_under_an_alias_is_reported_by_its_table(self):
        lines, refused = judged("aliased")
        found = [
            "WARN full-scan crates/events/src/lib.rs events 1441810155: SCAN a",
            "WARN full-scan crates/events/src/lib.rs events 1955906640: SCAN e",
        ]
        self.assertEqual(lines, [summary(1, 2, 2, 0, 0, 1), *found])
        self.assertFalse(refused)

    def test_a_name_the_rule_cannot_map_is_reported_by_the_level(self):
        shown = [(1, "SCAN e"), (2, "SCAN a"), (2, "SCAN a")]
        for level in examined("levels", ("warn", "fail")):
            lines, refused = judged("unmapped", level=level)
            expected = [
                f"{level.upper()} unmapped crates/events/src/lib.rs:{at}: {step}"
                for at, step in shown
            ]
            self.assertEqual(lines, [summary(1, 2, 2, 0, 0, 2), *expected])
            self.assertEqual(refused, level == "fail")


class Repository(unittest.TestCase):
    """The census over this repository's shipped sources and migrations; it runs in CI."""

    def test_the_repositorys_findings_are_its_listing_at_warn(self):
        paths = examined("shipped source file(s)", sorted(REPO.glob("crates/*/src/**/*.rs")))
        sources = {
            path.relative_to(REPO).as_posix(): path.read_text(encoding="utf-8") for path in paths
        }
        migrations = {
            "service": examined("service migration(s)", repository_migrations(SERVICE_MIGRATIONS)),
            "store": examined("store migration(s)", repository_migrations(STORE_MIGRATIONS)),
        }
        lines, refused = census(sources, migrations)
        print("\n".join(lines))
        self.assertEqual(LEVEL, "warn")
        self.assertTrue(lines[0].startswith("query-plan-census: examined "), lines[0])
        listed = {line[7:].split(":", 1)[0] for line in lines if line.startswith("LISTED ")}
        self.assertEqual(listed, set(LISTED))
        self.assertFalse(refused, "\n".join(lines))


if __name__ == "__main__":
    unittest.main()

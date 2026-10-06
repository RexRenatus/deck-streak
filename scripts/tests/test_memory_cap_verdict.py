"""The verdict reads each leg's memory-scope record and names the mutant the cap stopped (SPEC-196
A9 to A16).

Every test builds shard directories as the artifacts land (`mutants.out/outcomes.json`,
`mutants.out/log/*.log`, `cargo-mutants.exit`, `memory-scope.json`) and runs
`scripts/mutation-verdict.py` as the workflows do. A kill is nextest's own status line: its first
token `SIGKILL`, a bracketed duration, and the test's binary and name as the last two tokens.
"""

import ast
import contextlib
import io
import json
import re
import tempfile
import unittest
from pathlib import Path

from _support import examined
from test_mutation_verdict import (
    LIB,
    LIB_TEXT,
    ROW_ON_CONSTANT,
    VERDICT,
    ZERO_SCOPE,
    Fixture,
    listing,
    run_shards,
    shard_outcomes,
    sharded,
    table,
    verdict_module,
)

CAP_TEXT = "the memory cap stopped its tests; neither caught nor a timeout"
IN_RUN = "     SIGKILL [   4.209s] (1/1) fix tests::doubles"
SUMMARY = "     SIGKILL [   4.209s] (1/1) fix tests::doubles"
OTHER = "     SIGKILL [   5.110s] (2/3) fix tests::triples"
PLANT_FILE = "crates/kernel/src/memory_cap_plant.rs"
PLANT_MUTANTS = [
    f"{PLANT_FILE}:15:5: replace plant_chunks -> Vec<Vec<u8>> with vec![]",
    f"{PLANT_FILE}:15:5: replace plant_chunks -> Vec<Vec<u8>> with vec![vec![]]",
    f"{PLANT_FILE}:15:5: replace plant_chunks -> Vec<Vec<u8>> with vec![vec![0]]",
    f"{PLANT_FILE}:15:5: replace plant_chunks -> Vec<Vec<u8>> with vec![vec![1]]",
    f"{PLANT_FILE}:17:13: replace > with == in plant_chunks",
    f"{PLANT_FILE}:17:13: replace > with < in plant_chunks",
    f"{PLANT_FILE}:17:13: replace > with >= in plant_chunks",
    f"{PLANT_FILE}:20:11: replace /= with %= in plant_chunks",
    f"{PLANT_FILE}:20:11: replace /= with *= in plant_chunks",
]


def scope(oom=0, oom_kill=0, **changes):
    return {**ZERO_SCOPE, "oom": oom, "oom_kill": oom_kill, **changes}


def slug(name):
    return re.sub(r"[^A-Za-z0-9]+", "_", name).strip("_")[:80]


# How a scenario log opens. cargo-mutants 27.1.0 opens every log with one blank line, then
# `*** <scenario>`; the axis crosses the blank and whitespace-only lines before that line with the
# line ending, so the verdict's reading is judged on every opening and not on the one it was built on.
LEADS = {
    "no blank line": [],
    "one blank line": [""],
    "several blank and whitespace-only lines": ["", "  ", "\t", ""],
}
ENDINGS = {"LF": "\n", "CRLF": "\r\n"}
OPENINGS = [(lead, ending) for lead in LEADS for ending in ENDINGS]
REAL_OPENING = ("one blank line", "LF")


def write_log(directory, base, first, lines, opening=REAL_OPENING):
    lead, ending = opening
    text = ENDINGS[ending].join(
        [*LEADS[lead], f"*** {first}", "", "     PASS [   0.004s] fix tests::other", *lines, ""]
    )
    (directory / "mutants.out" / "log" / f"{base}.log").write_bytes(text.encode("utf-8"))


def cap_shard(
    root,
    directory_name,
    report,
    *,
    kills=None,
    ghosts=None,
    baseline=(),
    code="0",
    record=ZERO_SCOPE,
    package="fix",
    opening=REAL_OPENING,
):
    """One leg's artifact: `report`, each outcome's log (`kills` maps a mutant to the lines its
    log holds, `ghosts` the same for a scenario whose outcome was never written), the exit and the
    memory-scope record (None: absent; a string: written as is)."""
    directory = Path(root) / directory_name
    (directory / "mutants.out" / "log").mkdir(parents=True)
    for outcome in report["outcomes"]:
        scenario = outcome["scenario"]
        if scenario == "Baseline":
            base, first, lines = "baseline", "baseline", list(baseline)
        else:
            name = scenario["Mutant"]["name"]
            scenario["Mutant"]["package"] = package
            base, first, lines = slug(name), name, (kills or {}).get(name, [])
        outcome["log_path"] = f"log/{base}.log"
        write_log(directory, base, first, lines, opening)
    for name, lines in (ghosts or {}).items():
        write_log(directory, slug(name), name, lines, opening)
    (directory / "mutants.out" / "outcomes.json").write_text(json.dumps(report), encoding="utf-8")
    (directory / "cargo-mutants.exit").write_text(f"{code}\n", encoding="utf-8")
    if record is not None:
        text = record if isinstance(record, str) else json.dumps(record)
        (directory / "memory-scope.json").write_text(text, encoding="utf-8")
    return directory


def mutation_lines(done):
    """The verdict's report lines with their `mutation: rust: ` prefix removed, then its own."""
    prefix = "mutation: rust: "
    return [x.removeprefix(prefix) for x in done.stdout.splitlines() if x.startswith(prefix)]


def fixture_package():
    costs = verdict_module().SECONDS_PER_MUTANT
    return max(costs, key=costs.get)


class Scenes(unittest.TestCase):
    def world(self):
        fixture, planned = sharded(self)
        self.fixture, self.planned = fixture, planned
        return fixture, planned

    def write_all(self, name, **per_shard):
        """Every planned shard whole and quiet, unless `per_shard` bends shard k as `k{k}`."""
        root = self.fixture.out / name
        for shard, names in enumerate(self.planned):
            bend = dict(per_shard.get(f"k{shard}", {}))
            missed = bend.pop("missed", ())
            code = bend.pop("code", "2" if missed else "0")
            report = bend.pop("report", None) or shard_outcomes(names, missed=missed)
            cap_shard(root, f"mutation-rust-shard-{shard}", report, code=code, **bend)
        return root

    def judge(self, root):
        return self.fixture.judge("rust", "--shard-reports", str(root))


class TheRecord(Scenes):
    def test_the_verdict_reads_every_promised_shards_record(self):
        self.world()
        bent = {
            "absent": (None, "no memory-scope.json"),
            "unreadable": ("{not json", "memory-scope.json is unreadable"),
            "running": (scope(state="running"), "memory-scope.json is not done"),
            "not a dict": ("[1, 2]", "memory-scope.json is unreadable"),
            "a count that is no integer": (scope(oom="1"), "memory-scope.json is unreadable"),
            "not in force": (
                scope(in_force=False, reason="memory.max is not the cap"),
                "memory.max is not the cap",
            ),
        }
        for label, (record, phrase) in examined(
            "records", list(bent.items()) and [(k, v) for k, v in bent.items()]
        ):
            with self.subTest(record=label):
                root = self.write_all(f"record-{slug(label)}", k1={"record": record})
                done = self.judge(root)
                self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
                void = [
                    x for x in mutation_lines(done) if x.startswith("VOID mutation-rust-shard-1")
                ]
                self.assertEqual(len(void), 1, done.stdout)
                self.assertIn(phrase, void[0])
                self.assertNotIn("never tested", done.stdout)
        control = self.judge(self.write_all("record-control"))
        self.assertEqual(control.returncode, 0, control.stdout + control.stderr)


class TheCarriedRows(unittest.TestCase):
    def test_a_stopped_mutant_and_a_carried_row_are_each_counted_once(self):
        fixture = Fixture(self, rows=[("MUTATIONS", ROW_ON_CONSTANT)])
        changed = LIB_TEXT.replace("x * 2", "x + x").replace(
            "pub const LAST_HOUR: u8 = 23;",
            "pub const LAST_HOUR: u8 = 23; // the day's last hour",
        )
        fixture.head({LIB: changed})
        fixture.plan()
        done, plan, _ = run_shards(fixture, listing(["fix", "fix", "fix"]))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        names = plan["shards"]["shards"][0]["mutants"]
        self.assertEqual(len(names), 3)
        root = fixture.out / "carried"
        cap_shard(
            root,
            "mutation-rust-shard-0",
            shard_outcomes(names),
            kills={names[0]: [IN_RUN, SUMMARY]},
            record=scope(1, 1),
        )
        rows = fixture.report(
            "rows.json", [{"id": "S00050-LAST-HOUR", "verdict": "KILLED", "target": LIB}]
        )
        judged = fixture.judge("rust", "--shard-reports", str(root), "--rows", str(rows))
        self.assertEqual(judged.returncode, 1, judged.stdout + judged.stderr)
        found = mutation_lines(judged)
        self.assertIn("examined 2 by cargo-mutants and 1 by rows", found)
        self.assertEqual(judged.stdout.splitlines()[-1], "examined 3")


class TheUntouchedShard(Scenes):
    def test_a_shard_the_cap_never_touched_is_judged_as_before(self):
        self.world()
        names = self.planned[0]
        root = self.write_all(
            "untouched",
            k0={"kills": {names[1]: [IN_RUN, SUMMARY]}, "baseline": [IN_RUN]},
        )
        done = self.judge(root)
        # Measured from the verdict of the tree before this SPEC, which reads no record.
        self.assertEqual(
            mutation_lines(done),
            [
                "crates/fix/src/lib.rs: 1 changed code line(s)",
                "cargo-mutants examined 143 (caught 143, missed 0, timeout 0), unviable 0, "
                "of 143 on the diff",
                "missed 0: equivalent 0, unexplained 0",
                "examined 143 by cargo-mutants and 0 by rows",
                "verdict: ok",
            ],
        )
        self.assertEqual(done.stdout.splitlines()[-1], "examined 143")
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)


class TheNamedKill(Scenes):
    def test_a_mutant_the_cap_stopped_fails_its_leg_by_name_and_the_rest_stand(self):
        self.world()
        names = self.planned[0]
        stopped, missed = names[1], names[2]
        root = self.write_all(
            "named",
            k0={
                "missed": [missed],
                "kills": {stopped: [IN_RUN, SUMMARY]},
                "record": scope(1, 1, max=1, peak_percent=100),
            },
        )
        done = self.judge(root)
        found = mutation_lines(done)
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertEqual(
            [x for x in found if "MEMORY-CAP" in x],
            [f"mutation-rust-shard-0: MEMORY-CAP {stopped}: {CAP_TEXT}"],
        )
        self.assertIn(f"mutation-rust-shard-0: MISSED {missed}", found)
        self.assertIn("verdict: FAIL: 2 finding(s)", found)
        self.assertIn(
            "cargo-mutants examined 143 (caught 142, missed 1, timeout 0), unviable 0, "
            "of 143 on the diff",
            found,
        )
        self.assertIn("examined 142 by cargo-mutants and 0 by rows", found)
        self.assertEqual(done.stdout.splitlines()[-1], "examined 142")
        # The control: the same shard whose record counts no kill reads its log for nothing.
        quiet = self.judge(
            self.write_all(
                "named-quiet",
                k0={"missed": [missed], "kills": {stopped: [IN_RUN, SUMMARY]}},
            )
        )
        self.assertEqual(
            [x for x in mutation_lines(quiet) if "MEMORY-CAP" in x or "FAIL" in x],
            ["verdict: FAIL: 1 finding(s)"],
        )


AMBIGUOUS = re.compile(
    r"mutation-rust-shard-0: MEMORY-CAP AMBIGUOUS: the scope counted (\d+) out-of-memory "
    r"event\(s\) and (\d+) kill\(s\); the logs place (\d+): (.*)"
)


class TheAttribution(Scenes):
    def ambiguous(self, label, record, kills=None, baseline=(), ghosts=None):
        root = self.write_all(
            label,
            k0={"record": record, "kills": kills or {}, "baseline": baseline, "ghosts": ghosts},
        )
        done = self.judge(root)
        return done, [m for x in mutation_lines(done) if (m := AMBIGUOUS.fullmatch(x))]

    def test_an_attribution_that_does_not_match_fails_the_leg(self):
        self.world()
        first, second = self.planned[0][1], self.planned[0][2]
        cases = {
            "no placed kill": (scope(1, 1), {}, (1, 1, 0)),
            "two placed kills": (
                scope(1, 1),
                {first: [IN_RUN, SUMMARY], second: [OTHER]},
                (1, 1, 2),
            ),
            "an event with no kill": (scope(1, 0), {}, (1, 0, 0)),
            "an event with no kill and a placed one": (
                scope(1, 0),
                {first: [IN_RUN, SUMMARY]},
                (1, 0, 1),
            ),
        }
        for label, (record, kills, counts) in examined("attributions", list(cases.items())):
            with self.subTest(case=label):
                done, found = self.ambiguous(f"amb-{slug(label)}", record, kills)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertEqual(len(found), 1, done.stdout)
                self.assertEqual(tuple(int(x) for x in found[0].groups()[:3]), counts)
                for name in kills:
                    self.assertIn(name, found[0].group(4))
                # No mutant is scored: nothing is named, and every mutant stands as counted.
                self.assertFalse([x for x in mutation_lines(done) if f"MEMORY-CAP {first}" in x])
                self.assertEqual(done.stdout.splitlines()[-1], "examined 143")

    def test_a_status_line_is_placed_or_refused_as_the_verdict_reads_it(self):
        self.world()
        target = self.planned[0][1]
        placed = {
            "in-run line with nextest's counter": [IN_RUN],
            "in-run line without a counter": ["     SIGKILL [   4.209s] fix tests::doubles"],
            "in-run line and its summary repeat": [IN_RUN, SUMMARY],
            "a line with more room in its columns": [
                "SIGKILL [ 121.500s] (10/12) fix tests::doubles"
            ],
            "the same test repeated three times": [IN_RUN, SUMMARY, SUMMARY],
        }
        refused = {
            "a SIGKILL inside a test's own output": [
                "note: SIGKILL [   4.209s] fix tests::doubles"
            ],
            "a SIGTERM status": ["     SIGTERM [   4.209s] (1/1) fix tests::doubles"],
            "the word with no duration": ["     child received SIGKILL and stopped"],
            "a status with too few tokens": ["     SIGKILL [   4.209s] fix"],
        }
        for label, lines in examined("status lines", [*placed.items(), *refused.items()]):
            with self.subTest(line=label):
                done, found = self.ambiguous(f"line-{slug(label)}", scope(1, 1), {target: lines})
                if label in placed:
                    self.assertEqual(found, [], done.stdout)
                    self.assertIn(
                        f"mutation-rust-shard-0: MEMORY-CAP {target}: {CAP_TEXT}", done.stdout
                    )
                else:
                    self.assertEqual(len(found), 1, done.stdout)
                    self.assertEqual(found[0].group(3), "0", done.stdout)
        two, found = self.ambiguous(
            "line-two-tests", scope(2, 2), {target: [IN_RUN, SUMMARY, OTHER]}
        )
        self.assertEqual(found, [], two.stdout)
        self.assertEqual(two.stdout.count(f"MEMORY-CAP {target}"), 1, two.stdout)


class TheBaselineAndThePartialShard(Scenes):
    def test_a_cap_kill_in_the_baseline_or_a_partial_shard_fails_by_name(self):
        self.world()
        names = self.planned[0]
        partial = shard_outcomes(names[:5], total=len(names))
        shapes = {
            "the baseline": ("baseline", {}, None),
            "a mutant with an outcome": ("partial", {names[3]: [IN_RUN, SUMMARY]}, None),
            "a mutant with a log and no outcome": (
                "partial",
                {},
                {names[9]: [IN_RUN, SUMMARY]},
            ),
        }
        # The population: every opening of a scenario log crossed with every place a kill sits.
        members = [(opening, label) for opening in OPENINGS for label in shapes]
        members = examined("log openings by kill places", members)
        self.assertEqual(len(members), len(LEADS) * len(ENDINGS) * len(shapes))
        for opening, label in members:
            kind, kills, ghosts = shapes[label]
            with self.subTest(opening=opening, kill=label):
                key = f"{slug(label)}-{slug(opening[0])}-{opening[1]}"
                if kind == "baseline":
                    done = self.judge(
                        self.write_all(
                            f"baseline-{key}",
                            k0={
                                "record": scope(1, 1),
                                "baseline": [IN_RUN, SUMMARY],
                                "opening": opening,
                            },
                        )
                    )
                    self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                    found = [x for x in mutation_lines(done) if "MEMORY-CAP" in x]
                    self.assertEqual(len(found), 1, done.stdout)
                    self.assertTrue(
                        found[0].startswith(
                            "mutation-rust-shard-0: MEMORY-CAP the unmutated baseline"
                        ),
                        found,
                    )
                    continue
                # A shard cut short at exit 137: VOID as before, and the kill named all the same.
                stopped = names[3] if kills else names[9]
                root = self.write_all(
                    f"partial-{key}",
                    k0={
                        "report": json.loads(json.dumps(partial)),
                        "code": "137",
                        "record": scope(1, 1),
                        "kills": kills,
                        "ghosts": ghosts,
                        "opening": opening,
                    },
                )
                done = self.judge(root)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                found = mutation_lines(done)
                self.assertIn(f"mutation-rust-shard-0: MEMORY-CAP {stopped}: {CAP_TEXT}", found)
                self.assertTrue(
                    any(
                        x.startswith("VOID mutation-rust-shard-0: cargo-mutants exit 137")
                        for x in found
                    ),
                    done.stdout,
                )


class TheWeeklyReaders(Scenes):
    def test_the_weekly_battery_and_table_refuse_a_cap_kill_by_name(self):
        self.world()
        names = self.planned[0][:4]
        stopped = names[1]
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        reports = Path(scratch.name) / "reports"
        cap_shard(
            reports, "mutants-shard-0", shard_outcomes(names),
            kills={stopped: [IN_RUN, SUMMARY]}, record=scope(1, 1),
        )  # fmt: skip
        cap_shard(reports, "mutants-shard-1", shard_outcomes(self.planned[1][:3]), record=None)
        cap_shard(reports, "mutants-shard-2", shard_outcomes(self.planned[2][:3]))
        battery = self.fixture.verdict("battery", "--reports", str(reports), "--shards", "3")
        self.assertNotEqual(battery.returncode, 0, battery.stdout)
        lines = battery.stdout.splitlines()
        self.assertIn(f"battery: mutants-shard-0: MEMORY-CAP {stopped}: {CAP_TEXT}", lines)
        absent = [x for x in lines if x.startswith("battery: mutants-shard-1: ")]
        self.assertEqual(len(absent), 1, battery.stdout)
        self.assertIn("memory-scope.json", absent[0])
        self.assertFalse([x for x in lines if x.startswith("battery: mutants-shard-2: ")], lines)
        tree = Path(scratch.name) / "tree"
        tree.mkdir()
        done = table(reports, tree)
        self.assertNotEqual(done.returncode, 0, done.stdout)
        lines = done.stdout.splitlines()
        self.assertIn(f"table: mutants-shard-0: MEMORY-CAP {stopped}: {CAP_TEXT}", lines)
        self.assertTrue(
            any(
                x.startswith("table: VOID mutants-shard-1: ") and "memory-scope.json" in x
                for x in lines
            ),
            done.stdout,
        )
        # Tallied nowhere: 4 + 3 + 3 mutants reported, one of them the stopped one.
        self.assertIn(
            "table: fix: listed 9, killed 9, equivalent 0, unexplained 0, unviable 0", lines
        )
        # The control: the same reports with every record quiet count all ten.
        quiet = Path(scratch.name) / "quiet"
        for shard, planned in enumerate((names, self.planned[1][:3], self.planned[2][:3])):
            cap_shard(quiet, f"mutants-shard-{shard}", shard_outcomes(planned))
        counted = table(quiet, tree)
        self.assertIn(
            "table: fix: listed 10, killed 10, equivalent 0, unexplained 0, unviable 0",
            counted.stdout.splitlines(),
        )


class TheOnePlace(Scenes):
    def test_a_named_cap_kill_is_scored_in_one_place(self):
        self.world()
        names = self.planned[0]
        stopped = names[1]
        module = verdict_module()

        def replacement(fail, say, where, name):
            fail(f"{where}REPLACED {name}")
            return False

        module.score_memory_cap = replacement
        root = self.write_all(
            "one-place", k0={"record": scope(1, 1), "kills": {stopped: [IN_RUN, SUMMARY]}}
        )
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        reports = Path(scratch.name) / "reports"
        cap_shard(reports, "mutants-shard-0", shard_outcomes(names[:3]),
                  kills={names[1]: [IN_RUN, SUMMARY]}, record=scope(1, 1))  # fmt: skip
        tree = Path(scratch.name) / "tree"
        tree.mkdir()
        runs = {
            "judge": [
                "judge", "--plan", str(self.fixture.out / "plan.json"), "--class", "rust",
                "--root", str(self.fixture.root), "--shard-reports", str(root),
            ],
            "battery": ["battery", "--reports", str(reports), "--shards", "1"],
            "table": ["table", "--reports", str(reports), "--root", str(tree)],
        }  # fmt: skip
        for verb, argv in examined("verbs", list(runs.items())):
            with self.subTest(verb=verb):
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    module.main(argv)
                self.assertIn(f"REPLACED {stopped}", out.getvalue())
                self.assertNotIn(f"MEMORY-CAP {stopped}", out.getvalue())
        source = VERDICT.read_text(encoding="utf-8")
        self.assertEqual(source.count("MEMORY-CAP {name}"), 1)
        tree_ast = ast.parse(source)
        scorer = next(
            n
            for n in ast.walk(tree_ast)
            if isinstance(n, ast.FunctionDef) and n.name == "score_memory_cap"
        )
        self.assertIn("MEMORY-CAP {name}", ast.get_source_segment(source, scorer))


class ThePlant(Scenes):
    def test_the_plant_fails_its_leg_naming_that_mutant_and_only_it(self):
        fixture = Fixture(self)
        fixture.head({LIB: "pub fn double(x: i64) -> i64 {\n    x + x\n}\n"})
        fixture.plan()
        self.fixture = fixture
        package = fixture_package()
        listed = [
            {"name": n, "package": package, "file": LIB, "genre": "FnValue"} for n in PLANT_MUTANTS
        ]
        done, plan, _ = run_shards(fixture, listed)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(len(plan["shards"]["shards"]), 1)
        runaway = PLANT_MUTANTS[6]
        self.assertTrue(runaway.endswith("replace > with >= in plant_chunks"))
        root = fixture.out / "plant"
        cap_shard(
            root,
            "mutation-rust-shard-0",
            shard_outcomes(PLANT_MUTANTS),
            kills={runaway: [IN_RUN, SUMMARY]},
            record=scope(1, 1, max=1, peak_percent=100),
        )
        result = fixture.judge("rust", "--shard-reports", str(root))
        found = mutation_lines(result)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(
            [x for x in found if "MEMORY-CAP" in x or "MISSED" in x],
            [f"mutation-rust-shard-0: MEMORY-CAP {runaway}: {CAP_TEXT}"],
        )
        self.assertIn("verdict: FAIL: 1 finding(s)", found)
        self.assertIn(f"examined {len(PLANT_MUTANTS) - 1} by cargo-mutants and 0 by rows", found)
        self.assertFalse([x for x in found if x.startswith("VOID")])


class TheWholeValues(Scenes):
    """The counts, names and branches of the verdict's memory-cap reading, asserted whole
    (SPEC-196 A9 to A16, judged by the Python mutation class)."""

    def test_a_stopped_mutant_is_not_examined_whatever_cargo_mutants_called_it(self):
        self.world()
        names = self.planned[0]
        stopped = names[1]
        for summary, code in examined(
            "summaries", [("CaughtMutant", "0"), ("MissedMutant", "2"), ("Timeout", "3")]
        ):
            with self.subTest(summary=summary):
                report = shard_outcomes(names)
                for outcome in report["outcomes"]:
                    scenario = outcome["scenario"]
                    if isinstance(scenario, dict) and scenario["Mutant"]["name"] == stopped:
                        outcome["summary"] = summary
                report["caught"] = len(names) - (summary != "CaughtMutant")
                report["missed"] = int(summary == "MissedMutant")
                report["timeout"] = int(summary == "Timeout")
                root = self.write_all(
                    f"stopped-{summary}",
                    k0={
                        "report": report,
                        "code": code,
                        "kills": {stopped: [IN_RUN, SUMMARY]},
                        "record": scope(1, 1),
                    },
                )
                done = self.judge(root)
                found = mutation_lines(done)
                self.assertEqual(
                    [x for x in found if "MEMORY-CAP" in x or "MISSED" in x],
                    [f"mutation-rust-shard-0: MEMORY-CAP {stopped}: {CAP_TEXT}"],
                    done.stdout,
                )
                self.assertIn("examined 142 by cargo-mutants and 0 by rows", found)
                missed = int(summary == "MissedMutant")
                self.assertIn(f"missed {missed}: equivalent 0, unexplained {missed}", found)
                self.assertIn("verdict: FAIL: 1 finding(s)", found)
                self.assertEqual(done.stdout.splitlines()[-1], "examined 142")

    def test_a_miss_the_report_counts_and_never_lists_is_still_unnamed_beside_a_stopped_one(self):
        self.world()
        names = self.planned[0]
        stopped = names[1]
        report = shard_outcomes(names)
        for outcome in report["outcomes"]:
            scenario = outcome["scenario"]
            if isinstance(scenario, dict) and scenario["Mutant"]["name"] == stopped:
                outcome["summary"] = "MissedMutant"
        report["caught"] = len(names) - 2
        report["missed"] = 2
        root = self.write_all(
            "unlisted-miss",
            k0={
                "report": report,
                "code": "2",
                "kills": {stopped: [IN_RUN, SUMMARY]},
                "record": scope(1, 1),
            },
        )
        found = mutation_lines(self.judge(root))
        self.assertIn("MISSED 1 mutant(s), unnamed in the report", found)

    def test_a_report_of_a_leg_that_did_not_finish_names_nothing_in_battery(self):
        self.world()
        names = self.planned[0]
        stopped = names[1]
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        reports = Path(scratch.name) / "reports"
        leg = cap_shard(
            reports, "mutants-shard-0", shard_outcomes(names[:3]), code="1",
            kills={stopped: [IN_RUN, SUMMARY]}, record=scope(1, 1),
        )  # fmt: skip
        log = leg / "mutants.out" / "log" / f"{slug(stopped)}.log"
        log.write_text(
            log.read_text("utf-8").replace(f"*** {stopped}", "*** named-by-the-log"), "utf-8"
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            verdict_module().main(["battery", "--reports", str(reports), "--shards", "1"])
        self.assertIn(f"MEMORY-CAP named-by-the-log: {CAP_TEXT}", out.getvalue())
        self.assertNotIn(f"MEMORY-CAP {stopped}", out.getvalue())

    def test_a_mutant_is_named_by_its_outcome_and_a_log_by_its_own_first_line_without_one(self):
        module = verdict_module()
        kill = "     SIGKILL [   1.000s] (1/1) bin tests::t"
        mutant = {"scenario": {"Mutant": {"name": "true-name"}}, "log_path": "log/a.log"}
        cases = {
            "a mutant named by its outcome, whatever else the report holds": (
                {"a.log": f"\n*** wrong-first-line\n\n{kill}\n"},
                [
                    5,
                    {"scenario": {"Mutant": {"name": "x"}}, "log_path": 7},
                    {"scenario": "Baseline", "summary": "Success"},
                    {"scenario": {"Other": {}}, "log_path": "log/x.log"},
                    {"scenario": "Baseline", "log_path": "log/b.log"},
                    mutant,
                ],
                {("true-name", "bin", "tests::t")},
            ),
            "the baseline named by its outcome": (
                {"b.log": f"\n*** something else\n\n{kill}\n"},
                [{"scenario": "Baseline", "summary": "Success", "log_path": "log/b.log"}],
                {(module.BASELINE, "bin", "tests::t")},
            ),
            "a log named by its first line when the report is not whole": (
                {"c.log": f"\n*** by-first-line\n\n{kill}\n"},
                None,
                {("by-first-line", "bin", "tests::t")},
            ),
            "a kill on the line after the opening": (
                {"d.log": f"*** by-first-line\n{kill}\n"},
                [],
                {("by-first-line", "bin", "tests::t")},
            ),
            "a log holding bytes that are not UTF-8 is still read": (
                {"e.log": b"\n*** by-bytes\n\n\xff\xfe junk\n" + kill.encode() + b"\n"},
                None,
                {("by-bytes", "bin", "tests::t")},
            ),
            "a log that cannot be read is passed over": (
                {"a.log": None, "b.log": f"\n*** after-the-unreadable\n\n{kill}\n"},
                None,
                {("after-the-unreadable", "bin", "tests::t")},
            ),
            "a log of blank lines places nothing and stops nothing": (
                {"a.log": "\n  \n", "b.log": f"\n*** later\n\n{kill}\n"},
                None,
                {("later", "bin", "tests::t")},
            ),
        }
        for label, (logs, outcomes, wanted) in examined("namings", list(cases.items())):
            with self.subTest(label=label), tempfile.TemporaryDirectory() as directory:
                out = Path(directory)
                (out / "log").mkdir()
                for name, text in logs.items():
                    if text is None:
                        (out / "log" / name).mkdir()
                        continue
                    (out / "log" / name).write_bytes(
                        text if isinstance(text, bytes) else text.encode("utf-8")
                    )
                report = None if outcomes is None else {"outcomes": outcomes}
                self.assertEqual(module.placed_kills(out, report), wanted)

    def test_a_leg_is_read_when_it_wrote_an_outcomes_file_or_an_exit_and_not_when_neither(self):
        module = verdict_module()

        def read(make):
            with tempfile.TemporaryDirectory() as directory:
                leg = Path(directory)
                (leg / "mutants.out").mkdir()
                make(leg)
                failed, voided = [], []
                found = module.memory_cap(failed.append, voided.append, print, "leg: ", leg, None)
                return found, failed, voided

        def only_outcomes(leg):
            (leg / "mutants.out" / "outcomes.json").write_text("{}", encoding="utf-8")

        def only_exit(leg):
            (leg / "cargo-mutants.exit").write_text("0\n", encoding="utf-8")

        def neither(leg):
            pass

        absent = "leg: no memory-scope.json: the leg left no record of its memory scope"
        wanted = {
            "only outcomes": (only_outcomes, [absent]),
            "only exit": (only_exit, [absent]),
            "neither": (neither, []),
        }
        for label, (make, voids) in examined("legs", list(wanted.items())):
            with self.subTest(leg=label):
                found, failed, voided = read(make)
                self.assertEqual((found, failed, voided), (set(), [], voids))

    def test_a_kill_is_named_by_the_outcome_in_every_verb_when_the_report_is_whole(self):
        self.world()
        names = self.planned[0]
        stopped = names[1]
        module = verdict_module()
        root = self.write_all(
            "named-by-outcome", k0={"record": scope(1, 1), "kills": {stopped: [IN_RUN, SUMMARY]}}
        )
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        reports = Path(scratch.name) / "reports"
        cap_shard(reports, "mutants-shard-0", shard_outcomes(names[:3]),
                  kills={stopped: [IN_RUN, SUMMARY]}, record=scope(1, 1))  # fmt: skip
        for leg in (root / "mutation-rust-shard-0", reports / "mutants-shard-0"):
            log = leg / "mutants.out" / "log" / f"{slug(stopped)}.log"
            log.write_text(
                log.read_text("utf-8").replace(f"*** {stopped}", "*** a-wrong-first-line"), "utf-8"
            )
        tree = Path(scratch.name) / "tree"
        tree.mkdir()
        runs = {
            "judge": [
                "judge", "--plan", str(self.fixture.out / "plan.json"), "--class", "rust",
                "--root", str(self.fixture.root), "--shard-reports", str(root),
            ],
            "battery": ["battery", "--reports", str(reports), "--shards", "1"],
            "table": ["table", "--reports", str(reports), "--root", str(tree)],
        }  # fmt: skip
        for verb, argv in examined("verbs", list(runs.items())):
            with self.subTest(verb=verb):
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    module.main(argv)
                self.assertIn(f"MEMORY-CAP {stopped}: {CAP_TEXT}", out.getvalue())
                self.assertNotIn("a-wrong-first-line", out.getvalue())


if __name__ == "__main__":
    unittest.main()

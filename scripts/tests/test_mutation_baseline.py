"""SPEC-480 section 9 (R11, R16): each Rust leg's own bounded baseline and each Rust package's
Timeouts. `scripts/mutation-baseline.sh` runs with a stand-in `cargo` and a stand-in `timeout`
first on `PATH`, each logging the argv it was given, and `scripts/mutation-timeouts.sh` runs over
a fixed three-leg reports directory; both run from the repository's root, as the workflow runs
them."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

BASELINE = "scripts/mutation-baseline.sh"
TIMEOUTS = "scripts/mutation-timeouts.sh"

#: The stand-in `cargo`: it logs its argv and the snapshot pair it was given into
#: `$FAKE_LOG.cargo`, prints one timed status line, and then sleeps, ignores the budget's signal
#: and exits as the test asks.
CARGO = r"""#!/usr/bin/env bash
printf "%s\n" "$@" > "$FAKE_LOG.cargo"
printf "INSTA_UPDATE=%s INSTA_FORCE_PASS=%s\n" "${INSTA_UPDATE-unset}" "${INSTA_FORCE_PASS-unset}" >> "$FAKE_LOG.cargo"
echo "        PASS [   0.004s] (1/1) a tests::one"
if [ "${FAKE_IGNORE_TERM:-0}" = 1 ]; then trap "" TERM; fi
sleep "${FAKE_SLEEP:-0}"
exit "${FAKE_EXIT:-0}"
"""

#: The stand-in `timeout`: it logs its argv into `$FAKE_LOG.timeout` and runs the command it
#: bounds, unbounded.
TIMEOUT = r"""#!/usr/bin/env bash
printf "%s\n" "$@" > "$FAKE_LOG.timeout"
shift 2
exec "$@"
"""

#: The plan's listing of seven mutants, packages a, b, a, c, b, d, a in listing order.
LISTED = (
    '[{"name": "m0", "package": "a"}, {"name": "m1", "package": "b"}, '
    '{"name": "m2", "package": "a"}, {"name": "m3", "package": "c"}, '
    '{"name": "m4", "package": "b"}, {"name": "m5", "package": "d"}, '
    '{"name": "m6", "package": "a"}]\n'
)

#: What cargo-mutants' own baseline runs, the leg's packages between `--verbose` and `--profile`.
NEXTEST = ("nextest", "run", "--verbose")
PROFILE = ("--profile", "mutants")
#: The environment cargo-mutants' own baseline runs in (SPEC-480 R11).
SNAPSHOTS = "INSTA_UPDATE=no INSTA_FORCE_PASS=0"


def text_of(path):
    """A file's text, or None when it is missing, so a missing output fails an equality."""
    try:
        return Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        return None


def lines_of(path):
    """A file's lines, or None when it is missing."""
    text = text_of(path)
    return None if text is None else text.splitlines()


class Bed:
    """One test's temporary directory: the stand-ins under `bin/`, which goes first on `PATH`,
    and the listing."""

    def __init__(self, test, stand_ins=("cargo", "timeout")):
        scratch = tempfile.TemporaryDirectory()
        test.addCleanup(scratch.cleanup)
        self.tmp = Path(scratch.name)
        (self.tmp / "bin").mkdir()
        for name in stand_ins:
            path = self.tmp / "bin" / name
            path.write_text({"cargo": CARGO, "timeout": TIMEOUT}[name], encoding="utf-8")
            path.chmod(0o700)
        self.listed = self.tmp / "listed.json"
        self.listed.write_text(LISTED, encoding="utf-8")

    def run(self, shard, shards, script=BASELINE, **fakes):
        """The baseline script for leg `shard` of `shards`, its stand-ins logging under
        `s<shard>` and its log `log<shard>`, with the snapshot pair set so that a logged pair is
        the script's own."""
        env = dict(
            os.environ,
            PATH=f"{self.tmp / 'bin'}{os.pathsep}{os.environ.get('PATH', '')}",
            INSTA_UPDATE="always",
            INSTA_FORCE_PASS="1",
            FAKE_LOG=str(self.tmp / f"s{shard}"),
            **fakes,
        )
        args = [str(self.listed), str(shard), str(shards), str(self.tmp / f"log{shard}")]
        return subprocess.run(
            ["bash", str(script), *args],
            cwd=REPO,
            capture_output=True,
            text=True,
            env=env,
            timeout=120,
            check=False,
        )


class ALegRunsItsOwnBoundedBaseline(unittest.TestCase):
    """SPEC-480 A14 to A17 (R11): the leg's packages, dealt as cargo-mutants deals its
    round-robin shards, run by one nextest run of the mutants profile in cargo-mutants' own
    baseline environment under `timeout --kill-after=60 3845`, and the script's exit."""

    def test_the_packages_are_dealt_as_cargo_mutants_deals_them(self):
        bed = Bed(self)
        done = bed.run(0, 3)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        # Mutants 0, 3 and 6 of the listing: packages a, c and a, each once.
        dealt = ["--package=a", "--package=c"]
        self.assertEqual(lines_of(bed.tmp / "s0.cargo"), [*NEXTEST, *dealt, *PROFILE, SNAPSHOTS])
        self.assertEqual(
            lines_of(bed.tmp / "s0.timeout"),
            ["--kill-after=60", "3845", "cargo", *NEXTEST, *dealt, *PROFILE],
        )
        legs = [(1, ["--package=b"]), (2, ["--package=a", "--package=d"])]
        for shard, packages in examined("legs dealt a package", legs):
            leg = bed.run(shard, 3)
            self.assertEqual(leg.returncode, 0, leg.stdout + leg.stderr)
            logged = lines_of(bed.tmp / f"s{shard}.cargo") or []
            self.assertEqual([word for word in logged if word.startswith("--package")], packages)
        # Leg 7 of 8 is dealt no mutant of the seven, so no package: refused, with no test run.
        empty = bed.run(7, 8)
        self.assertEqual(empty.returncode, 2, empty.stdout + empty.stderr)
        self.assertEqual(
            empty.stdout, f"mutation-baseline: shard 7 of 8 is dealt no package by {bed.listed}\n"
        )
        self.assertIsNone(lines_of(bed.tmp / "s7.cargo"))

    def test_a_green_baseline_exits_0_and_keeps_its_log(self):
        bed = Bed(self)
        done = bed.run(0, 3)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(text_of(bed.tmp / "log0"), "        PASS [   0.004s] (1/1) a tests::one\n")
        self.assertRegex(
            done.stdout,
            r"\Amutation-baseline: shard 0 of 3: exit 0 after \d+ s of its 3845 s budget\n\Z",
        )

    def test_a_failed_baseline_exits_its_own_code(self):
        bed = Bed(self)
        done = bed.run(0, 3, FAKE_EXIT="100")
        self.assertEqual(done.returncode, 100, done.stdout + done.stderr)
        self.assertRegex(
            done.stdout,
            r"\Amutation-baseline: shard 0 of 3: exit 100 after \d+ s of its 3845 s budget\n\Z",
        )

    def test_a_baseline_that_reaches_its_budget_exits_124(self):
        # The real `timeout`, over a copy of the script at a 1 s budget and a 1 s kill delay.
        bed = Bed(self, stand_ins=("cargo",))
        text = (REPO / BASELINE).read_text(encoding="utf-8")
        copy = bed.tmp / "mutation-baseline.sh"
        copy.write_text(
            text.replace("budget=3845", "budget=1").replace("--kill-after=60", "--kill-after=1"),
            encoding="utf-8",
        )
        runs = [
            # The run obeys the budget's signal: `timeout` exits 124.
            ("ends at the signal", {"FAKE_SLEEP": "5"}),
            # The run ignores it and is killed a second later: `timeout` exits 137.
            ("killed after it", {"FAKE_IGNORE_TERM": "1", "FAKE_SLEEP": "6"}),
        ]
        for name, fakes in examined("runs past the budget", runs):
            done = bed.run(0, 3, script=copy, **fakes)
            self.assertEqual(done.returncode, 124, f"{name}: {done.stdout}{done.stderr}")
            self.assertRegex(done.stdout, r"exit 124 after \d+ s of its 1 s budget", name)
        self.assertEqual(text.count("budget=3845"), 1)
        self.assertEqual(text.count("--kill-after=60"), 1)


#: The three legs of A20 under the verdict job's reports directory: {path: text}.
LEGS = {
    "mutation-rust-shard-0/baseline/baseline.exit": "0\n",
    "mutation-rust-shard-0/baseline/baseline.log": (
        "   Compiling deck-streak-kernel v0.1.0\n"
        "        PASS [   3.000s] ( 1/4) deck-streak-kernel clock::tests::now\n"
        "        SLOW [>1200.000s] ( 2/4) deck-streak-progression::settle_census census\n"
        "        PASS [1469.349s] ( 2/4) deck-streak-progression::settle_census census\n"
        "        PASS [   2.000s] ( 3/4) deck-streak-kernel clock::tests::tick\n"
        "        PASS [ 100.000s] ( 4/4) deck-streak-progression xp::tests::level\n"
        "     Summary [1471.000s] 4 tests run: 4 passed, 0 skipped\n"
    ),
    "mutation-rust-shard-0/mutants.out/outcomes.json": (
        '{"outcomes": [{"scenario": {"Mutant": {"name": "crates/progression/src/settle.rs:40:9: '
        'replace settle -> u32 with 0", "package": "deck-streak-progression"}}, "summary": '
        '"Timeout"}, {"scenario": {"Mutant": {"name": "crates/progression/src/settle.rs:52:13: '
        'replace < with <= in settle", "package": "deck-streak-progression"}}, "summary": '
        '"Timeout"}, {"scenario": {"Mutant": {"name": "crates/progression/src/xp.rs:7:5: '
        'replace xp -> u32 with 1", "package": "deck-streak-progression"}}, "summary": '
        '"CaughtMutant"}, {"scenario": {"Mutant": {"name": "crates/kernel/src/clock.rs:3:5: '
        'replace now -> u64 with 0", "package": "deck-streak-kernel"}}, "summary": '
        '"CaughtMutant"}, {"scenario": {"Mutant": {"name": "crates/kernel/src/clock.rs:9:5: '
        'replace tick with ()", "package": "deck-streak-kernel"}}, "summary": "MissedMutant"}]}'
    ),
    "mutation-rust-shard-1/baseline/baseline.exit": "0\n",
    "mutation-rust-shard-1/baseline/baseline.log": (
        "        PASS [1500.000s] deck-streak-kernel::soak soak::tests::long\n"
        "        PASS [   1.000s] deck-streak-web tests::serve\n"
        "        PASS [  10.000s] deck-streak-progression xp::tests::level\n"
        "        PASS [1600.000s] deck-streak-review queue::tests::due\n"
    ),
    "mutation-rust-shard-1/mutants.out/outcomes.json": (
        '{"outcomes": [{"scenario": {"Mutant": {"name": "crates/kernel/src/clock.rs:20:9: '
        'replace + with - in elapsed", "package": "deck-streak-kernel"}}, "summary": '
        '"Timeout"}, {"scenario": {"Mutant": {"name": "crates/web/src/lib.rs:5:5: replace '
        'serve -> u16 with 0", "package": "deck-streak-web"}}, "summary": "CaughtMutant"}, '
        '{"scenario": {"Mutant": {"name": "crates/review/src/queue.rs:12:5: replace due -> '
        'bool with true", "package": "deck-streak-review"}}, "summary": "CaughtMutant"}]}'
    ),
    "mutation-rust-shard-2/baseline/baseline.exit": "0\n",
    "mutation-rust-shard-2/baseline/baseline.log": (
        "        PASS [   2.000s] (1/1) deck-streak-web tests::serve\n"
    ),
    "mutation-rust-shard-2/mutants.out/outcomes.json": (
        '{"outcomes": [{"scenario": {"Mutant": {"name": "crates/ingest/src/memory_state.rs:85:16: '
        'replace += with *= in leniently", "package": "deck-streak-ingest"}}, "summary": '
        '"Timeout"}]}'
    ),
}

#: The Timeouts script's stdout over LEGS: progression's tests sum to 1569.349 s in leg 0, and
#: 1.5 times that passes 2300 s; ingest's no log times; kernel's largest sum is leg 1's 1500 s,
#: within the margin; review and web have no Timeout.
PRINTED = (
    "Timeouts by package, over 3 leg report(s) and 3 green baseline log(s): READ BY NAME marks a "
    "package with a Timeout whose baseline tests, times 1.5, pass the 2300 s per-mutant timeout, "
    "or that no log times\n"
    "deck-streak-ingest: 1 Timeout(s) of 1 mutant(s), its baseline tests untimed: READ BY NAME\n"
    "deck-streak-ingest: READ BY NAME mutation-rust-shard-2: "
    "crates/ingest/src/memory_state.rs:85:16: replace += with *= in leniently\n"
    "deck-streak-kernel: 1 Timeout(s) of 3 mutant(s), its baseline tests 1500.0 s\n"
    "deck-streak-progression: 2 Timeout(s) of 3 mutant(s), its baseline tests 1569.3 s: "
    "READ BY NAME\n"
    "deck-streak-progression: READ BY NAME mutation-rust-shard-0: "
    "crates/progression/src/settle.rs:40:9: replace settle -> u32 with 0\n"
    "deck-streak-progression: READ BY NAME mutation-rust-shard-0: "
    "crates/progression/src/settle.rs:52:13: replace < with <= in settle\n"
    "deck-streak-review: 0 Timeout(s) of 1 mutant(s), its baseline tests 1600.0 s\n"
    "deck-streak-web: 0 Timeout(s) of 1 mutant(s), its baseline tests 2.0 s\n"
)


class EachPackagesTimeoutsArePrinted(unittest.TestCase):
    """SPEC-480 A20 (R16): each package's Timeouts beside its baseline tests' seconds, and each
    Timeout of a package over the margin or untimed printed by leg and name."""

    def test_each_package_prints_its_timeouts_and_an_over_margin_one_is_read_by_name(self):
        with tempfile.TemporaryDirectory() as scratch:
            reports = Path(scratch) / "reports"
            for relative, text in examined("files of the three legs", list(LEGS.items())):
                path = reports / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
            done = subprocess.run(
                ["bash", TIMEOUTS, str(reports)],
                cwd=REPO,
                capture_output=True,
                text=True,
                timeout=120,
                check=False,
            )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertEqual(done.stdout, PRINTED)


if __name__ == "__main__":
    unittest.main()

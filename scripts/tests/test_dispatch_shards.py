"""A package dispatch is sharded by its projected weight (SPEC-129 A1 to A5).

The sizing verb is run as the workflow runs it, on fixture listings written to a temporary
directory; the workflow is read as text with the helpers `test_mutation_workflows.py` uses. The
plants put the fixed 32 back into each place that must read the one count.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import examined
from test_mutation_workflows import VERDICT, WEEKLY, jobs, listed, shard, workflow

WHOLE = 32
COUNT = "${{ needs.size.outputs.shards }}"


def entries(package, count):
    """`count` listed mutants of `package`, as cargo-mutants lists them, in listing order."""
    return [listed(f" ({n})", package=package) for n in range(count)]


def size(listing, package=None, *, raw=None):
    """Run `size` as the workflow does; returns (exit code, stdout, the step's outputs)."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        path = root / "package.json"
        path.write_text(raw if raw is not None else json.dumps(listing), encoding="utf-8")
        sink = root / "output"
        sink.touch()
        args = [sys.executable, str(VERDICT), "size", "--listed", str(path)]
        if package is not None:
            args += ["--package", package]
        done = subprocess.run(
            args,
            capture_output=True,
            text=True,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GITHUB_OUTPUT=str(sink)),
            timeout=120,
            check=False,
        )
        written = dict(
            line.split("=", 1) for line in sink.read_text(encoding="utf-8").splitlines() if line
        )
        return done.returncode, done.stdout + done.stderr, written


def count_problems(text):
    """What the workflow leaves fixed, or unread, where the one count belongs."""
    found = jobs(text)
    problems = []
    if "size" not in found:
        return ["no size job"]
    if not re.search(r"(?m)^    outputs:\n(?:      .*\n)*?      shards: ", found["size"]):
        problems.append("the size job outputs no shards")
    if not re.search(r"(?m)^    outputs:\n(?:      .*\n)*?      matrix: ", found["size"]):
        problems.append("the size job outputs no matrix")
    rust = found.get("rust", "")
    if not re.search(r"(?m)^    needs: \[?size\]?$", rust):
        problems.append("the rust job does not need size")
    if "shard: ${{ fromJSON(needs.size.outputs.matrix) }}" not in rust:
        problems.append("the rust matrix is not the sized one")
    if f"--shard ${{{{ matrix.shard }}}}/{COUNT}" not in rust:
        problems.append("the rust legs' --shard does not read the sized count")
    survivors = found.get("survivors", "")
    if not re.search(r"(?m)^    needs: \[[^\]]*\bsize\b[^\]]*\]$", survivors):
        problems.append("the survivors job does not need size")
    if f'battery --reports "$reports" --shards {COUNT}' not in survivors:
        problems.append("the battery's --shards does not read the sized count")
    for name, job in found.items():
        if name == "rehearsal":
            continue
        for hit in re.findall(r"--shards? (?:\$\{\{ matrix\.shard \}\}/)?\d+", job):
            problems.append(f"{name}: a fixed count, {hit}")
    return problems


class TheDispatchIsSizedFromItsListing(unittest.TestCase):
    """A1 (R1, R2): the fewest round-robin shards whose slowest fits the bound, by the plan's own
    function; a projection past the matrix's limit is refused with its projection."""

    def test_a_small_package_takes_one_shard_and_a_large_one_the_fewest_within_the_bound(self):
        # Costs and bound are the plan's: the daemon costs 80 s a mutant on a 371 s baseline, and
        # 40 mutants project to 3571 s of a 3600 s bound; the ingest costs 126 s.
        cases = [
            ("deck-streak-kernel", 5, 1),
            ("deck-streak-daemon", 40, 1),
            ("deck-streak-daemon", 41, 2),
            ("deck-streak-ingest", 60, 3),
        ]
        for package, count, expected in examined("listings sized", cases):
            code, out, written = size(entries(package, count), package)
            self.assertEqual(code, 0, out)
            self.assertEqual(written.get("shards"), str(expected), f"{package} {count}: {out}")
            self.assertEqual(json.loads(written.get("matrix", "null")), list(range(expected)), out)
            self.assertIn(f"{expected} shard(s) for {count} listed mutant(s)", out)

    def test_the_sizing_is_the_per_pull_request_plans_own_function(self):
        for package, count in examined(
            "packages", [("deck-streak-ingest", 200), ("deck-streak-api", 900)]
        ):
            listing = entries(package, count)
            code, out, written = size(listing, package)
            self.assertEqual(code, 0, out)
            with tempfile.TemporaryDirectory() as scratch:
                plan = Path(scratch) / "plan.json"
                plan.write_text(json.dumps({"classes": {"rust": {"applies": True}}}), "utf-8")
                whole = Path(scratch) / "listed.json"
                whole.write_text(json.dumps(listing), encoding="utf-8")
                done = subprocess.run(
                    [
                        sys.executable,
                        str(VERDICT),
                        "shards",
                        "--plan",
                        str(plan),
                        "--listed",
                        str(whole),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=120,
                    env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
                )
                self.assertEqual(done.returncode, 0, done.stdout)
                planned = json.loads(plan.read_text("utf-8"))["shards"]["count"]
            self.assertEqual(written.get("shards"), str(planned), out)

    def test_a_projection_past_the_limit_is_refused_with_its_projection_never_capped(self):
        code, out, written = size(entries("deck-streak-ingest", 7000), "deck-streak-ingest")
        self.assertEqual(code, 1, out)
        self.assertIn("REFUSED", out)
        self.assertIn("7000 mutant(s), projected at 882000 s serially", out)
        self.assertEqual(written, {}, "a refused sizing wrote outputs")

    def test_a_package_that_lists_nothing_or_a_non_listing_is_not_sized(self):
        code, out, written = size([], "deck-streak-ingest")
        self.assertEqual((code, written.get("shards")), (0, "1"), out)
        code, out, written = size(None, "deck-streak-ingest", raw="not json")
        self.assertEqual(code, 3, out)
        self.assertIn("VOID", out)
        self.assertEqual(written, {}, out)


class TheWholeTreeKeepsThirtyTwo(unittest.TestCase):
    """A2 (R3): a scheduled run and a dispatch with no package read no listing and keep 32."""

    def test_no_package_is_thirty_two_shards_whatever_the_listing(self):
        for raw in examined("listings", [None, "not json", json.dumps(entries("a", 3))]):
            code, out, written = size(None, None, raw=raw or "")
            self.assertEqual(code, 0, out)
            self.assertEqual(written.get("shards"), str(WHOLE), out)
            self.assertEqual(json.loads(written["matrix"]), list(range(WHOLE)), out)


class TheWorkflowReadsTheOneCount(unittest.TestCase):
    """A3 (R4): the rust matrix, its --shard argument and the battery's --shards read one count."""

    def test_the_matrix_the_argument_and_the_battery_read_the_sized_count(self):
        text = workflow(WEEKLY)
        for reader in (f"--shard ${{{{ matrix.shard }}}}/{COUNT}", f"--shards {COUNT}"):
            self.assertIn(reader, text)
        self.assertEqual(count_problems(text), [])

    def test_a_plant_that_puts_the_fixed_count_back_goes_red(self):
        text = workflow(WEEKLY)
        plants = [
            (f"--shard ${{{{ matrix.shard }}}}/{COUNT}", "--shard ${{ matrix.shard }}/32"),
            ("shard: ${{ fromJSON(needs.size.outputs.matrix) }}", "shard: [0, 1, 2]"),
            (f"--shards {COUNT}", "--shards 32"),
        ]
        for original, planted in examined("plants", plants):
            self.assertIn(original, text, f"the workflow lacks {original}")
            self.assertNotEqual(count_problems(text.replace(original, planted)), [], planted)

    def test_the_size_job_lists_the_package_with_the_gates_own_bounds_and_sizes_it(self):
        size_job = jobs(workflow(WEEKLY)).get("size", "")
        command = re.search(r"cargo mutants [^\n]*", size_job)
        self.assertIsNotNone(command, "the size job lists nothing")
        for flag in (
            "--no-shuffle",
            "--list",
            "--json",
            "--in-place",
            "--timeout 300",
            "--build-timeout 600",
            '${PACKAGE:+--package "$PACKAGE"}',
        ):
            self.assertIn(flag, command.group(0))
        for block in re.findall(r"(?ms)^        run: [|]?\n?(.*?)(?=^      - |\Z)", size_job):
            self.assertNotIn("inputs.package", block, "the size job interpolates the input")
        self.assertIn("mutation-verdict.py size", size_job)


class TheBatteryRefusesAForeignShardCount(unittest.TestCase):
    """A4 (R5): a report set whose shard count differs from n fails the battery by name."""

    def run_battery(self, reports_count, shards):
        with tempfile.TemporaryDirectory() as scratch:
            reports = Path(scratch) / "reports"
            scope = entries("deck-streak-fix", 4)
            (reports / "listing").mkdir(parents=True)
            (reports / "listing" / "whole.json").write_text(json.dumps(scope), "utf-8")
            for number in range(reports_count):
                shard(reports, number, [(scope[number % 4], "CaughtMutant")], code="0")
            done = subprocess.run(
                [
                    sys.executable,
                    str(VERDICT),
                    "battery",
                    "--reports",
                    str(reports),
                    "--shards",
                    str(shards),
                    "--package",
                    "deck-streak-fix",
                    "--listed",
                    str(reports / "listing" / "whole.json"),
                ],
                capture_output=True,
                text=True,
                check=False,
                timeout=120,
                env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            )
            return done.returncode, done.stdout

    def test_a_report_beyond_the_count_fails_the_battery(self):
        code, out = self.run_battery(3, 2)
        self.assertEqual(code, 1, out)
        self.assertIn("battery: FOREIGN mutants-shard-2: the run was sized at 2 shard(s)", out)

    def test_a_report_set_of_exactly_the_count_passes(self):
        code, out = self.run_battery(2, 2)
        self.assertEqual(code, 0, out)
        self.assertIn("battery: counted 3 of 3 reports whole", out)


class TheExaminedTotalIsTheListing(unittest.TestCase):
    """A5 (R6): the sized shards take every listed mutant once, as the 32 shards do."""

    def test_the_union_of_the_sized_shards_equals_the_union_of_the_thirty_two(self):
        listing = entries("deck-streak-daemon", 133)
        code, out, written = size(listing, "deck-streak-daemon")
        self.assertEqual(code, 0, out)
        count = int(written["shards"])
        names = [entry["name"] for entry in listing]

        def taken(total):
            return [name for i in range(total) for n, name in enumerate(names) if n % total == i]

        self.assertGreater(WHOLE, count)
        self.assertEqual(sorted(taken(count)), sorted(taken(WHOLE)))
        self.assertEqual(len(taken(count)), len(names))


if __name__ == "__main__":
    unittest.main()

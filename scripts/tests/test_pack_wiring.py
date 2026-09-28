"""Every vendored pack has an honest wiring state (ADR-004; SPEC-002 A4), and the runner refuses a
state its rows have outgrown: a pending pack whose rows all pass, and a deferred row that passes
(SPEC-030 R6 to R8). It runs rows in a bounded pool that decides exactly what one row at a time
decides (SPEC-038 A8 and A9)."""

import json
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

WIRING = REPO / ".packs" / "wiring.json"
MANIFEST = REPO / "docs" / "issues-manifest.json"
ISSUE = re.compile(r"^#(\d+)$")
# The synthetic pack SPEC-030's A6 to A8 and SPEC-038's A8 and A9 plant beside the vendored ones;
# its rows never run the real packs.
SYNTHETIC = "planted-honesty"


def issue_numbers():
    if not MANIFEST.is_file():
        return set()
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    return {entry["number"] for entry in data["issues"].values()}


def box_waiting(box):
    """The box-pack runner's expectations (SPEC-030 R12, R13): each names the issue it waits on."""
    waiting = []
    for pack, entry in box.get("packs", {}).items():
        if "pending" in entry:
            waiting.append((f"box {pack}", entry["pending"]))
        for row, issue in entry.get("expected_red", {}).items():
            waiting.append((f"box {pack}:{row}", issue))
    scan = box.get("proxy-client-scan", {})
    if "pending" in scan:
        waiting.append(("box proxy-client-scan", scan["pending"]))
    return waiting


def run_rows(tmp, state, rows, deferred=None, checks=None, args=()):
    """pack-rows.py over a copy of .packs/ and scripts/, with one synthetic pack added whose rows
    are one-line probes: (id, severity, exit), where 0 is green, 1 red, 2 an error and 3 VOID. A
    caller may give the pack's `checks` whole instead, and more arguments for the runner."""
    shutil.copytree(REPO / ".packs", tmp / ".packs")
    shutil.copytree(REPO / "scripts", tmp / "scripts", ignore=shutil.ignore_patterns("tests"))
    if checks is None:
        checks = [row_check(ident, severity, code) for ident, severity, code in rows]
    pack = tmp / ".packs" / "skills" / "packs" / SYNTHETIC
    pack.mkdir()
    checks_file = {"schema": "phx.skills.checks.v1", "pack": f"packs/{SYNTHETIC}", "checks": checks}
    (pack / "checks.json").write_text(json.dumps(checks_file))
    wiring = json.loads((tmp / ".packs" / "wiring.json").read_text(encoding="utf-8"))
    entry = {"state": state}
    if state in ("pending", "deferred"):
        entry["enforced_by"] = "#23"
    if deferred:
        entry["deferred_rows"] = deferred
    wiring["packs"][SYNTHETIC] = entry
    (tmp / ".packs" / "wiring.json").write_text(json.dumps(wiring))
    runner = tmp / "scripts" / "pack-rows.py"
    return subprocess.run(
        [sys.executable, str(runner), "--root", str(tmp), "--pack", SYNTHETIC, *args],
        capture_output=True,
        text=True,
        check=False,
    )


def row_check(ident, severity, code, seconds=0, timeout=60):
    """A synthetic row whose probe sleeps `seconds`, reports one examined item and exits `code`."""
    code_line = f"import sys, time; time.sleep({seconds}); print('examined 1'); sys.exit({code})"
    return {
        "id": ident,
        "severity": severity,
        "scope": "tree",
        "probe": {"command": [sys.executable, "-c", code_line], "timeout_seconds": timeout},
    }


# One row of every verdict the runner gives. The first finishes last, so a pool that reported rows
# in the order they finished would put it anywhere but first.
EVERY_VERDICT = [
    row_check("slow-green", "block", 0, seconds=0.8),
    row_check("fast-red", "block", 1),
    row_check("void", "block", 3),
    row_check("error", "block", 2),
    row_check("advisory", "advisory", 1),
    row_check("times-out", "block", 0, seconds=5, timeout=1),
    row_check("lifted", "block", 0),
    row_check("still-red", "block", 1),
]
EVERY_VERDICT_DEFERRED = {"lifted": "#23", "still-red": "#23"}
# A probe that registers itself while it runs, and records the most rows it saw running at once.
PEAK_PROBE = """\
import pathlib, sys, time
state = pathlib.Path(sys.argv[1]) / "concurrency"
running = state / "running"
running.mkdir(parents=True, exist_ok=True)
flag = running / sys.argv[2]
flag.touch()
peak, until = 0, time.monotonic() + 1.5
while time.monotonic() < until:
    peak = max(peak, len(list(running.iterdir())))
    time.sleep(0.02)
(state / ("peak-" + sys.argv[2])).write_text(str(peak))
flag.unlink()
print("examined 1")
"""


def verdicts(done):
    """What a --json run decided, without its timings: examined, failures, the deferred pass, each
    row, and the exit."""
    payload = json.loads(done.stdout)
    rows = [
        {key: value for key, value in row.items() if key != "seconds"} for row in payload["rows"]
    ]
    deferred = {key: value for key, value in payload["deferred_pass"].items() if key != "seconds"}
    return payload["examined"], payload["failures"], deferred, rows, done.returncode


class PackWiringIsHonest(unittest.TestCase):
    def setUp(self):
        self.data = json.loads(WIRING.read_text(encoding="utf-8"))
        self.wiring = self.data["packs"]

    def test_every_vendored_pack_has_a_wiring_state(self):
        vendored = [
            p.name
            for p in (REPO / ".packs" / "skills" / "packs").iterdir()
            if (p / "checks.json").is_file()
        ]
        for pack in examined("vendored packs", vendored):
            self.assertIn(pack, self.wiring, f"{pack} has no wiring state")

    def test_every_waiting_pack_or_row_names_an_open_issue_by_number(self):
        numbers = issue_numbers()
        waiting = []
        for pack, entry in self.wiring.items():
            if entry["state"] in ("pending", "deferred"):
                waiting.append((pack, entry["enforced_by"]))
            for row, issue in entry.get("deferred_rows", {}).items():
                waiting.append((f"{pack}:{row}", issue))
        waiting += box_waiting(self.data.get("box", {}))
        for name, issue in examined("waiting packs and rows", waiting):
            match = ISSUE.match(issue)
            self.assertIsNotNone(match, f"{name} waits on {issue!r}, which is not an issue number")
            self.assertIn(
                int(match.group(1)),
                numbers,
                f"{name} waits on {issue}, which is not in the manifest",
            )

    def test_the_runner_refuses_a_wiring_that_forgets_a_pack(self):
        with tempfile.TemporaryDirectory() as scratch:
            tmp = Path(scratch)
            shutil.copytree(REPO / ".packs", tmp / ".packs")
            shutil.copytree(REPO / "scripts", tmp / "scripts")
            data = json.loads((tmp / ".packs" / "wiring.json").read_text())
            dropped = sorted(data["packs"])[0]
            del data["packs"][dropped]
            (tmp / ".packs" / "wiring.json").write_text(json.dumps(data))
            done = subprocess.run(
                [sys.executable, str(tmp / "scripts" / "pack-rows.py"), "--root", str(tmp)],
                capture_output=True,
                text=True,
                check=False,
            )
        self.assertEqual(done.returncode, 2, done.stdout)
        self.assertIn(dropped, done.stdout)


class StaleStatesAreRefused(unittest.TestCase):
    def test_a_pending_pack_whose_rows_all_pass_is_refused_as_stale(self):
        rows = [("first", "block", 0), ("second", "block", 0), ("heuristic", "advisory", 1)]
        with tempfile.TemporaryDirectory() as tmp:
            done = run_rows(Path(tmp), "pending", rows)
        self.assertRegex(done.stdout, rf"(?m)^FAIL STALE +{SYNTHETIC}:\* .*must say enforced")
        self.assertEqual(done.returncode, 1, done.stdout)
        # A blocking row that examined nothing keeps the pack pending: its subject is not built.
        rows = [("first", "block", 0), ("unbuilt", "block", 3)]
        with tempfile.TemporaryDirectory() as tmp:
            done = run_rows(Path(tmp), "pending", rows)
        self.assertRegex(done.stdout, rf"(?m)^ +pending +{SYNTHETIC}:unbuilt ")
        self.assertEqual(done.returncode, 0, done.stdout)

    def test_a_deferred_row_that_passes_is_refused_as_stale(self):
        rows = [("first", "block", 0), ("lifted", "block", 0)]
        with tempfile.TemporaryDirectory() as tmp:
            done = run_rows(Path(tmp), "enforced", rows, deferred={"lifted": "#23"})
        self.assertRegex(done.stdout, rf"(?m)^FAIL STALE +{SYNTHETIC}:lifted .*#23")
        self.assertIn("deferred pass: ran 1 row(s)", done.stdout)
        self.assertEqual(done.returncode, 1, done.stdout)

    def test_a_deferred_row_that_is_still_red_fails_nothing(self):
        rows = [
            ("first", "block", 0),
            ("red", "block", 1),
            ("void", "block", 3),
            ("error", "block", 2),
        ]
        deferred = {"red": "#23", "void": "#23", "error": "#23"}
        with tempfile.TemporaryDirectory() as tmp:
            done = run_rows(Path(tmp), "enforced", rows, deferred=deferred)
        for row, code in (("red", 1), ("void", 3), ("error", 2)):
            still = rf"(?m)^ +deferred +{SYNTHETIC}:{row} deferred to #23; still exit {code}"
            self.assertRegex(done.stdout, still)
        self.assertIn("deferred pass: ran 3 row(s)", done.stdout)
        self.assertEqual(done.returncode, 0, done.stdout)


class RowsRunInABoundedPool(unittest.TestCase):
    def test_a_parallel_run_gives_the_serial_verdicts_in_row_order(self):
        cases = [
            ("enforced", EVERY_VERDICT, EVERY_VERDICT_DEFERRED),
            ("pending", [row_check("first", "block", 0), row_check("second", "block", 0)], None),
        ]
        decided = {}
        for state, checks, deferred in examined("pack states", cases):
            runs = {}
            for jobs in (1, 4):
                with tempfile.TemporaryDirectory() as tmp:
                    args = ("--json", "--jobs", str(jobs))
                    done = run_rows(Path(tmp), state, [], deferred, checks=checks, args=args)
                runs[jobs] = verdicts(done)
            self.assertEqual(runs[4], runs[1], f"{state}: the pool changed what the run decided")
            ids = [check["id"] for check in checks]
            self.assertEqual([row["row"] for row in runs[4][3]][: len(ids)], ids)
            decided[state] = runs[4]
        # The runs gave every verdict the runner has, so the comparison judged each kind.
        rows = {
            row["row"]: (row["verdict"], row["exit"], row["tail"]) for row in decided["enforced"][3]
        }
        self.assertEqual(
            rows,
            {
                "slow-green": ("ok", 0, "examined 1"),
                "fast-red": ("RED", 1, "examined 1"),
                "void": ("VOID", 3, "examined 1"),
                "error": ("ERROR", 2, "examined 1"),
                "advisory": ("advisory", 1, "examined 1"),
                "times-out": ("ERROR", None, "timeout"),
                "lifted": ("STALE", 0, "examined 1"),
                "still-red": ("deferred", 1, "examined 1"),
            },
        )
        self.assertEqual(decided["enforced"][4], 1)
        self.assertEqual([row["verdict"] for row in decided["pending"][3]], ["ok", "ok", "STALE"])

    def test_the_pool_runs_at_most_its_bound_of_rows_at_once(self):
        bound = 3
        checks = [
            {
                "id": f"probe-{number}",
                "severity": "block",
                "scope": "tree",
                "probe": {
                    "command": [sys.executable, "-c", PEAK_PROBE, "{root}", f"probe-{number}"],
                    "timeout_seconds": 60,
                },
            }
            for number in range(2 * bound)
        ]
        with tempfile.TemporaryDirectory() as tmp:
            args = ("--jobs", str(bound))
            done = run_rows(Path(tmp), "enforced", [], checks=checks, args=args)
            records = examined("probe records", sorted((Path(tmp) / "concurrency").glob("peak-*")))
            peaks = [int(path.read_text()) for path in records]
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertEqual(len(peaks), 2 * bound, peaks)
        self.assertEqual(max(peaks), bound, f"the most rows seen running at once: {peaks}")


if __name__ == "__main__":
    unittest.main()

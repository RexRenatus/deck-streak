"""The SLO evaluator pages once per burn episode (SPEC-031 A5, R4; ADR-031).

The evaluator runs as its unit runs it, with a stub `journalctl` first on its PATH that records its
arguments and answers with a synthetic journal export: the API's response events, each the JSON line
SPEC-025's trace layer writes, as `journalctl --output=json --output-fields=MESSAGE` exports them
(`scripts/tests/fixtures/journal/api-burn.jsonl`). The clock is fixed with `--now`, and
`$STATE_DIRECTORY` is a temporary directory, so no run reads the host or depends on time passing.

The fixture ends at NOW. Over its last six hours the API answered twenty requests an hour, and the
last eight of the final ten failed with 5xx; before that it answered two an hour for three days,
none failing. Beside them sit what must never count: failures four days old and one after NOW, the
`response failed` event each 5xx also writes, a start line, systemd's plain line, a message held as
bytes (counted: it is a response) and one too large to export (skipped).
"""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

EVALUATOR = REPO / "deploy" / "scripts" / "slo-evaluate.py"
SLO = REPO / "deploy" / "slo.json"
EXPORT = REPO / "scripts" / "tests" / "fixtures" / "journal" / "api-burn.jsonl"
NOW = 1_700_000_000
MINUTE, HOUR, DAY = 60, 3_600, 86_400
RESPONSE = "finished processing request"
# What the fixture holds at NOW, by window: (failed responses, responses).
COUNTS = {"30m": (8, 10), "6h": (8, 120), "3d": (8, 252)}

STUB = """#!{python}
import json, os, sys
with open(os.path.join(os.environ["STUB_LOG"], "journalctl.jsonl"), "a", encoding="utf-8") as out:
    out.write(json.dumps(sys.argv[1:]) + "\\n")
if os.environ.get("STUB_FAIL"):
    sys.stderr.write("No journal files were opened due to insufficient permissions.\\n")
    sys.exit(1)
with open(os.environ["STUB_EXPORT"], encoding="utf-8") as export:
    sys.stdout.write(export.read())
"""


class Evaluator:
    """Runs of the evaluator over one state directory, with the stub journal first on PATH."""

    def __init__(self, scratch, fail=False):
        root = Path(scratch)
        self.state, self.log, stubs = root / "state", root / "log", root / "bin"
        for folder in (self.state, self.log, stubs):
            folder.mkdir()
        stub = stubs / "journalctl"
        stub.write_text(STUB.format(python=sys.executable), encoding="utf-8")
        stub.chmod(0o755)
        self.env = {
            "PATH": f"{stubs}:{os.environ.get('PATH', '/usr/bin:/bin')}",
            "STATE_DIRECTORY": str(self.state),
            "STUB_LOG": str(self.log),
            "STUB_EXPORT": str(EXPORT),
        }
        if fail:
            self.env["STUB_FAIL"] = "1"

    def run(self, now):
        """One run at `now`, as the unit runs it: the declaration, and the fixed clock."""
        done = subprocess.run(
            [sys.executable, str(EVALUATOR), str(SLO), "--now", str(now)],
            env=self.env,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        lines = done.stdout.splitlines()
        return done.returncode, lines, done.stderr

    def run_with(self, declaration, now, env=None):
        """One run at `now` over another declaration path, or with another environment."""
        done = subprocess.run(
            [sys.executable, str(EVALUATOR), str(declaration), "--now", str(now)],
            env=self.env if env is None else env,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        return done.returncode, done.stdout.splitlines(), done.stderr

    def journal_reads(self):
        path = self.log / "journalctl.jsonl"
        text = path.read_text(encoding="utf-8") if path.exists() else ""
        return [json.loads(line) for line in text.splitlines()]


def at(lines, priority, alert):
    """The lines of one run printed at `priority` about `alert` (`page` or `ticket`)."""
    return [line for line in lines if line.startswith(f"<{priority}>slo api-availability {alert}:")]


def load_evaluator():
    """The evaluator as a module, for its window arithmetic."""
    spec = importlib.util.spec_from_file_location("slo_evaluate", EVALUATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class TheEvaluatorPagesOncePerEpisode(unittest.TestCase):
    def test_the_evaluator_pages_once_when_the_page_window_burns(self):
        with tempfile.TemporaryDirectory() as scratch:
            evaluator = Evaluator(scratch)
            # Both of the page's windows burn at NOW: it pages, naming what it counted.
            code, lines, stderr = evaluator.run(NOW)
            self.assertEqual(
                code,
                1,
                "the evaluator did not page when the page window burned:\n"
                + "\n".join(lines)
                + stderr,
            )
            paged = at(lines, 3, "page")
            self.assertEqual(len(paged), 1, lines)
            failed_long, total_long = COUNTS["6h"]
            failed_short, total_short = COUNTS["30m"]
            self.assertIn(f"{failed_long} of {total_long} response(s) failed over 6h", paged[0])
            self.assertIn(f"{failed_short} of {total_short} over 30m", paged[0])
            # A minute later it still burns, and pages no one: the episode is remembered.
            code, lines, stderr = evaluator.run(NOW + MINUTE)
            self.assertEqual(code, 0, "\n".join(lines) + stderr)
            self.assertEqual(at(lines, 3, "page"), [], "it paged twice in one episode")
            self.assertEqual(len(at(lines, 4, "page")), 1, lines)
            # An hour on, the short window holds no response: the episode ends.
            code, lines, stderr = evaluator.run(NOW + HOUR)
            self.assertEqual(code, 0, "\n".join(lines) + stderr)
            self.assertEqual(len(at(lines, 5, "page")), 1, lines)
            # The same burn again is a new episode, and pages again.
            code, lines, stderr = evaluator.run(NOW)
            self.assertEqual(code, 1, "\n".join(lines) + stderr)
            self.assertEqual(len(at(lines, 3, "page")), 1, lines)
            # Every run read the API's journal once, over its longest window, up to its now.
            reads = examined("journal read(s)", evaluator.journal_reads())
            self.assertEqual(len(reads), 4)
            first = reads[0]
            self.assertIn("--unit=deck-streak-api.service", first)
            self.assertIn("--output=json", first)
            self.assertIn(f"--since=@{NOW - 3 * DAY}", first)
            self.assertIn(f"--until=@{NOW}", first)

    def test_the_ticket_reaches_the_owner_once_and_reads_its_own_windows(self):
        with tempfile.TemporaryDirectory() as scratch:
            evaluator = Evaluator(scratch)
            code, lines, _ = evaluator.run(NOW)
            self.assertEqual(code, 1)
            ticket = at(lines, 3, "ticket")
            self.assertEqual(len(ticket), 1, lines)
            failed_long, total_long = COUNTS["3d"]
            failed_short, total_short = COUNTS["6h"]
            self.assertIn(f"{failed_long} of {total_long} response(s) failed over 3d", ticket[0])
            self.assertIn(f"{failed_short} of {total_short} over 6h", ticket[0])
            # An hour on, the page cools and the ticket, whose windows still hold the burn, does not.
            code, lines, _ = evaluator.run(NOW + HOUR)
            self.assertEqual(code, 0)
            self.assertEqual(len(at(lines, 4, "ticket")), 1, lines)
            self.assertEqual(at(lines, 3, "ticket"), [], lines)

    def test_a_journal_the_evaluator_cannot_read_pages_once(self):
        with tempfile.TemporaryDirectory() as scratch:
            evaluator = Evaluator(scratch, fail=True)
            code, lines, _ = evaluator.run(NOW)
            self.assertEqual(code, 1, lines)
            failed = [line for line in lines if line.startswith("<3>slo evaluation failed")]
            self.assertEqual(len(failed), 1, lines)
            self.assertIn("journalctl exited 1", failed[0])
            code, lines, _ = evaluator.run(NOW + 5 * MINUTE)
            self.assertEqual(code, 0, lines)
            self.assertEqual([line for line in lines if line.startswith("<3>")], [], lines)
            self.assertEqual(
                len([line for line in lines if line.startswith("<4>slo evaluation failed")]), 1
            )

    def test_a_declaration_it_cannot_read_pages_once_and_then_measures_again(self):
        with tempfile.TemporaryDirectory() as scratch:
            evaluator = Evaluator(scratch)
            missing = Path(scratch) / "absent.json"
            code, lines, _ = evaluator.run_with(missing, NOW)
            self.assertEqual(code, 1, lines)
            self.assertEqual(
                [line for line in lines if line.startswith("<3>")],
                [
                    "<3>slo evaluation failed: the declaration cannot be read: No such file or directory"
                ],
            )
            code, lines, _ = evaluator.run_with(missing, NOW + MINUTE)
            self.assertEqual(code, 0, lines)
            # The burn it could not see pages once it measures again, and the blind episode ends.
            code, lines, _ = evaluator.run(NOW)
            self.assertEqual(code, 1, lines)
            self.assertEqual(len(at(lines, 3, "page")), 1, lines)
            self.assertIn("<5>slo evaluation measures again", lines)

    def test_a_record_it_cannot_read_starts_afresh(self):
        with tempfile.TemporaryDirectory() as scratch:
            evaluator = Evaluator(scratch)
            evaluator.run(NOW)
            (evaluator.state / "burning.json").write_text("not json", encoding="utf-8")
            code, lines, _ = evaluator.run(NOW + MINUTE)
            self.assertEqual(code, 1, lines)
            self.assertIn("<4>slo evaluation found its record unreadable, and starts afresh", lines)
            self.assertEqual(len(at(lines, 3, "page")), 1, lines)

    def test_without_a_state_directory_it_refuses_to_run(self):
        with tempfile.TemporaryDirectory() as scratch:
            evaluator = Evaluator(scratch)
            env = {key: value for key, value in evaluator.env.items() if key != "STATE_DIRECTORY"}
            code, lines, _ = evaluator.run_with(SLO, NOW, env=env)
            self.assertEqual(code, 2, lines)
            self.assertEqual(
                lines, ["<3>slo evaluation has no $STATE_DIRECTORY to keep its episodes in"]
            )
            self.assertEqual(
                evaluator.journal_reads(), [], "it read the journal with nowhere to remember"
            )


class TheWindowsAreTheWorkbooks(unittest.TestCase):
    def test_an_alert_burns_only_past_its_burn_rate_in_both_windows(self):
        evaluator = load_evaluator()
        page = {"severity": "page", "long_window": "6h", "short_window": "30m", "burn_rate": 5.6}
        budget = evaluator.error_budget(0.99)
        # 5.6 times a 1% budget is 7 failures in 125: exactly there is not past it.
        self.assertFalse(evaluator.burning(page, budget, (7, 125), (7, 125)))
        self.assertTrue(evaluator.burning(page, budget, (8, 125), (8, 125)))
        # Both windows must burn, and a window with no response never does.
        self.assertFalse(evaluator.burning(page, budget, (8, 125), (0, 0)))
        self.assertFalse(evaluator.burning(page, budget, (0, 0), (8, 125)))
        self.assertFalse(evaluator.burning(page, budget, (1, 125), (8, 10)))

    def test_a_window_counts_only_its_own_responses(self):
        evaluator = load_evaluator()
        responses = [(NOW - 30 * MINUTE, 500), (NOW - 30 * MINUTE + 1, 200), (NOW, 503)]
        responses += [(NOW + 1, 500)]
        # A window is the half-open span (now - window, now]: its start is out and now is in.
        self.assertEqual(evaluator.count(responses, NOW, 30 * MINUTE), (1, 2))
        self.assertEqual(evaluator.count(responses, NOW, 30 * MINUTE + 1), (2, 3))

    def test_a_response_event_is_read_flattened_or_nested_and_nothing_else(self):
        evaluator = load_evaluator()
        flattened = json.dumps({"message": RESPONSE, "status": 503, "latency": "4 ms"})
        nested = json.dumps({"fields": {"message": RESPONSE, "status": 200}, "target": "t"})
        self.assertEqual(evaluator.response_status(flattened), 503)
        self.assertEqual(evaluator.response_status(nested), 200)
        # Not a response: another event, a status that is no integer, a line that is no JSON.
        others = [
            json.dumps({"message": "response failed", "status": 500}),
            json.dumps({"message": RESPONSE, "status": "500"}),
            json.dumps({"message": RESPONSE, "status": True}),
            json.dumps([RESPONSE, 500]),
            "{not json",
            "Started a unit.",
        ]
        for text in examined("message(s) that are not response events", others):
            self.assertIsNone(evaluator.response_status(text), text)
        # journald's own encodings of a message: bytes as numbers, and null past its size limit.
        encoded = list(flattened.encode("utf-8"))
        self.assertEqual(evaluator.message_text({"MESSAGE": encoded}), flattened)
        self.assertIsNone(evaluator.message_text({"MESSAGE": None}))
        self.assertIsNone(evaluator.message_text({"MESSAGE": [300, 1]}))

    def test_the_fixture_holds_the_api_trace_events_shape(self):
        # Every response event of the fixture is the flattened event SPEC-025's layer writes: its
        # message, an integer status, a latency, the trace layer's target and the matched route.
        entries = [json.loads(line) for line in EXPORT.read_text(encoding="utf-8").splitlines()]
        events = []
        for entry in examined("journal entr(ies)", entries):
            message = entry["MESSAGE"]
            if isinstance(message, list):
                message = bytes(message).decode("utf-8")
            if not isinstance(message, str) or not message.startswith("{"):
                continue
            event = json.loads(message)
            if event.get("message") == RESPONSE:
                events.append(event)
        for event in examined("response event(s)", events):
            self.assertIsInstance(event["status"], int, event)
            self.assertEqual(event["target"], "tower_http::trace::on_response")
            self.assertTrue(event["latency"].endswith(" ms"), event)
            self.assertTrue(event["span"]["route"].startswith("/api/"), event)
            self.assertNotIn("fields", event, "the kernel's format flattens the event (ADR-020)")


if __name__ == "__main__":
    unittest.main()

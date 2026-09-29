"""The headless runner's contract (SPEC-043 R1 to R5; A1, A2, A3).

The runner is a shell script. Each test runs it against fakes of `claude` and `curl` that sit first
on PATH, so nothing reaches a model, a proxy or the network. The device key is a synthetic value the
test writes into a temporary credentials directory; it is built from parts at run time.
"""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

AGENT = Path(__file__).resolve().parent.parent
RUNNER = AGENT / "run-headless.sh"
FAKES = Path(__file__).resolve().parent / "fakes"
SYSTEM_PATH = "/usr/bin:/bin"

# A synthetic key: assembled at run time, so the literal never sits in the source.
KEY = "-".join(["synthetic", "device", "key", "0f3a9c"])
READY = '{"status":"ready"}'


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


class Run:
    """One launch of the runner, and everything the test may read back afterwards."""

    def __init__(self, root: Path, env: dict[str, str], args: list[str]) -> None:
        self.records = root / "records"
        self.scratch = root / "scratch"
        self.credentials = root / "credentials"
        for directory in (self.records, self.scratch, self.credentials):
            directory.mkdir(mode=0o700, exist_ok=True)
        (self.credentials / "agent-device-key").write_text(KEY + "\n")
        prompt = root / "prompt.md"
        prompt.write_text("a synthetic prompt\n")
        self.prompt = prompt
        base = {
            "PATH": f"{FAKES}:{SYSTEM_PATH}",
            "HOME": str(root),
            "TMPDIR": str(self.scratch),
            "CREDENTIALS_DIRECTORY": str(self.credentials),
            "FAKE_RECORDS": str(self.records),
            "FAKE_CURL_BODY": READY,
            "FAKE_CURL_HTTP": "200",
            "DECKSTREAK_AGENT_PROXY_URL": "http://127.0.0.1:9",
            "DECKSTREAK_AGENT_CAPACITY_PATH": "/synthetic-capacity",
            "DECKSTREAK_AGENT_MAX_TURNS": "7",
            "DECKSTREAK_AGENT_MAX_BUDGET_USD": "2.50",
            "DECKSTREAK_AGENT_WALL_SECONDS": "20",
        }
        base.update(env)
        argv = args if args else [str(prompt)]
        self.done = subprocess.run(
            ["bash", str(RUNNER), *argv],
            env=base,
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )

    def record(self, name: str) -> str:
        path = self.records / name
        return path.read_text() if path.exists() else ""

    def refusals(self) -> list[str]:
        return [line for line in self.done.stderr.splitlines() if line.startswith("REFUSE:")]


class RunnerTest(unittest.TestCase):
    def setUp(self) -> None:
        holder = tempfile.TemporaryDirectory()
        self.addCleanup(holder.cleanup)
        self.root = Path(holder.name)

    def run_runner(self, env: dict[str, str] | None = None, args: list[str] | None = None) -> Run:
        return Run(self.root, env or {}, args or [])

    def test_the_runner_keeps_the_key_off_argv_and_disk(self) -> None:
        run = self.run_runner()
        self.assertEqual(run.done.returncode, 0, run.done.stderr)
        self.assertNotIn(KEY, run.record("claude.argv"), "the key must never be on claude's argv")
        self.assertIn(
            f"CLAUDE_CODE_OAUTH_TOKEN={KEY}",
            run.record("claude.env"),
            "the key reaches claude in its environment",
        )
        self.assertNotIn(KEY, run.record("curl.argv"), "the key must never be on curl's argv")
        self.assertIn(KEY, run.record("curl.stdin"), "curl reads the key from its stdin")
        for name in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "CLAUDE_CONFIG_DIR"):
            self.assertNotIn(f"{name}=", run.record("claude.env"))
        self.assertIn("CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1", run.record("claude.env"))
        # No file the run leaves behind holds the key: its scratch directory, its stdout, stderr.
        for path in examined(
            "file(s) the run left behind",
            [run.prompt, *(path for path in run.scratch.rglob("*") if path.is_file())],
        ):
            self.assertNotIn(KEY, path.read_text(errors="replace"), str(path))
        self.assertNotIn(KEY, run.done.stdout)
        self.assertNotIn(KEY, run.done.stderr)
        self.assertEqual(list(run.scratch.iterdir()), [], "the runner removes its work directory")

    def test_the_launch_carries_every_cap_and_the_settings(self) -> None:
        run = self.run_runner()
        argv = run.record("claude.argv").split("\n")
        self.assertEqual(run.done.returncode, 0, run.done.stderr)
        for expected in (
            "--output-format",
            "json",
            "--max-turns",
            "7",
            "--max-budget-usd",
            "2.50",
            "--permission-mode",
            "dontAsk",
            "--strict-mcp-config",
            "--settings",
        ):
            self.assertIn(expected, argv)
        self.assertIn(str(AGENT / "settings.json"), argv)
        self.assertEqual(run.record("claude.stdin"), "a synthetic prompt\n")
        self.assertIn('"subtype":"success"', run.done.stdout)

    def test_the_runner_refuses_a_remote_url_bare_and_bypass(self) -> None:
        remote = self.run_runner({"DECKSTREAK_AGENT_PROXY_URL": "https://proxy.example.invalid"})
        self.assertEqual(remote.done.returncode, 2, remote.done.stderr)
        self.assertEqual(len(remote.refusals()), 1, remote.done.stderr)
        for flag in (
            "--bare",
            "--dangerously-skip-permissions",
            "--allow-dangerously-skip-permissions",
        ):
            with self.subTest(flag=flag):
                run = self.run_runner(args=[flag, str(self.root / "prompt.md")])
                self.assertEqual(run.done.returncode, 2, run.done.stderr)
                self.assertEqual(len(run.refusals()), 1, run.done.stderr)
                self.assertEqual(run.record("claude.argv"), "", "nothing is launched")
        for loopback in ("http://localhost:8477", "http://127.0.0.1:8477", "http://[::1]:8477"):
            with self.subTest(loopback=loopback):
                run = self.run_runner({"DECKSTREAK_AGENT_PROXY_URL": loopback})
                self.assertEqual(run.done.returncode, 0, run.done.stderr)

    def test_the_preflight_reads_the_status_word(self) -> None:
        exhausted = self.run_runner(
            {"FAKE_CURL_BODY": '{"status":"exhausted","retry_after":"2031-01-01T00:00:00Z"}'}
        )
        self.assertEqual(exhausted.done.returncode, 4, exhausted.done.stderr)
        self.assertIn("2031-01-01T00:00:00Z", exhausted.done.stderr)
        self.assertEqual(exhausted.record("claude.argv"), "", "nothing is launched")

        rejected = self.run_runner({"FAKE_CURL_HTTP": "401", "FAKE_CURL_BODY": ""})
        self.assertEqual(rejected.done.returncode, 3, rejected.done.stderr)

        for body, http in (('{"status":"weather"}', "200"), ("", "500"), ("not json", "200")):
            with self.subTest(body=body, http=http):
                unknown = self.run_runner({"FAKE_CURL_BODY": body, "FAKE_CURL_HTTP": http})
                self.assertEqual(unknown.done.returncode, 5, unknown.done.stderr)
                self.assertEqual(unknown.record("claude.argv"), "")

        unreachable = self.run_runner({"FAKE_CURL_EXIT": "7"})
        self.assertEqual(unreachable.done.returncode, 5, unreachable.done.stderr)

        for run in (exhausted, rejected, unreachable):
            self.assertNotIn(KEY, run.record("curl.argv"))
            self.assertNotIn(KEY, run.done.stderr)

    def test_a_missing_key_or_tool_or_input_exits_one(self) -> None:
        run = self.run_runner({"CREDENTIALS_DIRECTORY": str(self.root / "absent")})
        self.assertEqual(run.done.returncode, 1, run.done.stderr)
        self.assertEqual(len(run.refusals()), 1)
        missing = self.run_runner(args=[str(self.root / "no-such-prompt.md")])
        self.assertEqual(missing.done.returncode, 1, missing.done.stderr)

    def test_a_failed_run_a_wall_clock_and_an_error_result_exit_six(self) -> None:
        failed = self.run_runner(
            {"FAKE_CLAUDE_EXIT": "1", "FAKE_CLAUDE_STDERR": "a synthetic fault"}
        )
        self.assertEqual(failed.done.returncode, 6, failed.done.stderr)
        self.assertEqual(len(failed.refusals()), 1)

        slow = self.run_runner({"FAKE_CLAUDE_SLEEP": "30", "DECKSTREAK_AGENT_WALL_SECONDS": "1"})
        self.assertEqual(slow.done.returncode, 6, slow.done.stderr)
        self.assertIn("wall clock", slow.done.stderr)

        # The result text is read last: a reply that quotes a success cannot mask an error result.
        masked = self.run_runner(
            {
                "FAKE_CLAUDE_JSON": '{"type":"result","subtype":"error_max_turns","is_error":true,'
                '"num_turns":9,"result":"{\\"subtype\\":\\"success\\",\\"is_error\\":false}"}'
            }
        )
        self.assertEqual(masked.done.returncode, 6, masked.done.stderr)
        self.assertIn("error_max_turns", masked.done.stdout)


if __name__ == "__main__":
    unittest.main()

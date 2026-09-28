"""The vault adapter's content rails and the vault-duties pack agree on every planted fixture
(SPEC-042 A3), and the pack's rails rows are green on the committed synthetic run (A9). The
adapter's build runs under a bound, and outliving it fails by name (SPEC-054 R5).

The fixtures are `crates/vault/tests/fixtures/rails/`: one planted note for each rail row of the
pack's vendored `rails.json`, and one clean reading note, indexed by `rows.json`. Each is staged in
a temporary run (a staging directory with a `phx.duty.vault.run.v1` record), where the pack's own
probe judges it with its `no-executable` class, and the adapter judges the same file through its
`rails_verdicts` example; the two verdicts must be the same. The committed synthetic run,
`crates/vault/tests/runs/clean/`, is judged by the pack's probe as the executor's gate runs it.

Planted runs are built in temporary directories and never committed: the pack's walk from the root
would find them. The probe runs without the maintainer's private inputs, so it judges the public
shapes, as CI does.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined

PACK = REPO / ".packs" / "skills" / "packs" / "vault-duties"
PROBE = REPO / ".packs" / "scripts" / "vault-duties-probe.py"
FIXTURES = REPO / "crates" / "vault" / "tests" / "fixtures" / "rails"
SYNTHETIC_RUN = REPO / "crates" / "vault" / "tests" / "runs" / "clean"
#: The vault-duties rows of the pack's `rails` stage.
RAILS_ROWS = ("write-confinement", "no-executable", "never-deletes")
#: The private inputs the probe reads from the environment, dropped so it judges the public shapes.
PRIVATE_INPUTS = ("PERSONA_CORE_DENY_LIST", "VAULT_DUTIES_LAYOUT")
EXAMINED = re.compile(r"^examined (\d+)$", re.MULTILINE)
#: The bound on the adapter's build and run, in seconds (SPEC-054 R5). A cold build and run of the
#: example took 21.1 s on the maintainer's machine. The bound is over 40 times that, so a slower
#: runner, a cache miss or a wait for a build slot still fits, and inside CI's 30-minute hygiene
#: job, so a hung build fails here by name before the runner kills the job.
CARGO_BOUND_S = 900


def rail_rows(rails):
    """The rail rows `rails.json` declares, named as the adapter names them (`rails.rs`)."""
    rows = [f"fence_known:{name}" for name in rails["fence_known"]]
    rows += [f"fence_prefixes:{prefix}" for prefix in rails["fence_prefixes"]]
    rows += ["fence_allow", "templater_open"]
    rows += [f"inline_query_prefixes:{prefix}" for prefix in rails["inline_query_prefixes"]]
    rows += ["html_allow", "html_attributes_allow"]
    rows += [f"executable_schemes:{scheme}" for scheme in rails["executable_schemes"]]
    rows += [f"math_macros_refused:{name}" for name in rails["math_macros_refused"]]
    rows += [f"dynamic_embed_extensions:{ext}" for ext in rails["dynamic_embed_extensions"]]
    return sorted(rows)


def family(row):
    """A rail row as the pack's probe can name it: an inline query by its family alone."""
    return "inline_query_prefixes" if row.startswith("inline_query_prefixes:") else row


def pack_row(problem, rails):
    """The rail row a `no-executable` finding names, read from the probe's own wording."""
    for name, what in rails["fence_known"].items():
        if problem == f"a ```{name} fence: {what}":
            return f"fence_known:{name}"
    for prefix, what in rails["fence_prefixes"].items():
        if problem.startswith(f"a ```{prefix}") and problem.endswith(f" fence: {what}"):
            return f"fence_prefixes:{prefix}"
    if problem.startswith("a ```") and " fence, off the allow-list" in problem:
        return "fence_allow"
    if problem.startswith(f"a Templater command ({rails['templater_open']})"):
        return "templater_open"
    if problem.startswith("an obsidian: URI"):
        return "executable_schemes:obsidian"
    if problem.startswith("a Dataview inline query"):
        return "inline_query_prefixes"
    if problem.startswith("HTML attribute "):
        return "html_attributes_allow"
    if problem.startswith("HTML <") and problem.endswith("> is off the allow-list"):
        return "html_allow"
    for name in rails["math_macros_refused"]:
        if problem == f"the MathJax \\{name} macro, a link inside math":
            return f"math_macros_refused:{name}"
    for scheme in rails["executable_schemes"]:
        if problem == f"a {scheme}: link, which acts when it is opened":
            return f"executable_schemes:{scheme}"
    for ext in rails["dynamic_embed_extensions"]:
        if problem == f"an embedded {ext} view renders and edits other notes":
            return f"dynamic_embed_extensions:{ext}"
    raise AssertionError(f"a no-executable finding that names no rail row: {problem!r}")


def probe(name, run):
    """One class of the pack's probe over the run directory `run`."""
    env = {key: value for key, value in os.environ.items() if key not in PRIVATE_INPUTS}
    return subprocess.run(
        [sys.executable, str(PROBE), "--root", str(run), "--subject", str(run), "check", name],
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
        check=False,
    )


def judged_by_the_pack(fixture, rails):
    """The probe's exit and the rail rows its `no-executable` class names, for `fixture` staged
    as the one note a temporary daily-reading run creates."""
    with tempfile.TemporaryDirectory() as scratch:
        run = Path(scratch)
        staged = run / "12-Readings" / fixture.name
        staged.parent.mkdir()
        staged.write_bytes(fixture.read_bytes())
        record = {
            "schema": "phx.duty.vault.run.v1",
            "duty": "daily-reading",
            "vault": [],
            "ops": [{"op": "create", "path": f"12-Readings/{fixture.name}"}],
        }
        (run / "duty-run.json").write_text(json.dumps(record), encoding="utf-8")
        done = probe("no-executable", run)
    prefix = "no-executable: "
    problems = [
        line[len(prefix) :].split(": ", 1)[1]
        for line in done.stdout.splitlines()
        if line.startswith(prefix) and ": VOID" not in line
    ]
    return done.returncode, {pack_row(problem, rails) for problem in problems}


def bounded(command, timeout, what):
    """`command`, run from the repository root with its output captured, and killed once it
    outlives `timeout` seconds. `what` then fails by name, with the bound and the command, rather
    than holding the gate's python stage with no reason given (SPEC-054 R5)."""
    try:
        return subprocess.run(
            command, cwd=REPO, capture_output=True, text=True, timeout=timeout, check=False
        )
    except subprocess.TimeoutExpired as expired:
        raise AssertionError(
            f"{what} did not finish within {timeout} s: {' '.join(command)}"
        ) from expired


def judged_by_the_adapter(fixtures):
    """The adapter's rail rows for each fixture, by file name, from its `rails_verdicts` example.

    cargo may wait for a build slot on a shared machine, so the build's bound is generous.
    """
    done = bounded(
        [
            "cargo",
            "run",
            "--quiet",
            "--locked",
            "-p",
            "deck-streak-vault",
            "--example",
            "rails_verdicts",
            "--",
            *(str(fixture) for fixture in fixtures),
        ],
        CARGO_BOUND_S,
        "the adapter's rails_verdicts",
    )
    if done.returncode != 0:
        raise AssertionError(f"the adapter's rails_verdicts failed: {done.stderr[-2000:]}")
    verdicts = {}
    for line in done.stdout.splitlines():
        entry = json.loads(line)
        verdicts[entry["file"]] = set(entry["rows"])
    return verdicts


def examined_count(output):
    """The count a probe class printed on its last `examined N` line."""
    counts = EXAMINED.findall(output)
    return int(counts[-1]) if counts else 0


class TheAdapterRailsAgreeWithThePack(unittest.TestCase):
    def test_the_adapter_rails_agree_with_the_pack_on_every_fixture(self):
        rails = json.loads((PACK / "rails.json").read_text(encoding="utf-8"))
        rows = examined("rail row(s) of rails.json", rail_rows(rails))
        index = json.loads((FIXTURES / "rows.json").read_text(encoding="utf-8"))
        self.assertEqual(
            sorted(index["rows"]), rows, "rows.json plants one fixture for every rail row"
        )
        planted = {FIXTURES / name: row for row, name in index["rows"].items()}
        clean = FIXTURES / index["clean"]
        fixtures = examined("fixture(s), the clean note among them", [*sorted(planted), clean])
        self.assertEqual(
            sorted(FIXTURES.glob("*.md")), sorted(fixtures), "every fixture on disk is indexed"
        )
        adapter = judged_by_the_adapter(fixtures)
        for fixture in fixtures:
            with self.subTest(fixture.name):
                expected = {planted[fixture]} if fixture in planted else set()
                code, pack = judged_by_the_pack(fixture, rails)
                self.assertEqual(
                    (code, pack),
                    (1 if expected else 0, {family(row) for row in expected}),
                    f"the pack's verdict on {fixture.name}",
                )
                self.assertEqual(
                    adapter.get(fixture.name),
                    expected,
                    f"the adapter and the pack disagree on {fixture.name}",
                )


class TheRailsRowsAreGreenOnTheSyntheticRun(unittest.TestCase):
    def test_the_vault_duties_rails_rows_are_green_on_the_synthetic_run(self):
        self.assertTrue(
            (SYNTHETIC_RUN / "duty-run.json").is_file(), "the synthetic run is not committed"
        )
        for name in examined("rails row(s) of the vault-duties pack", list(RAILS_ROWS)):
            with self.subTest(name):
                done = probe(name, SYNTHETIC_RUN)
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertGreater(examined_count(done.stdout), 0, done.stdout)

    def test_no_blocking_row_is_red_on_the_synthetic_run(self):
        checks = json.loads((PACK / "checks.json").read_text(encoding="utf-8"))
        blocking = examined(
            "blocking row(s) of the vault-duties pack",
            [check["id"] for check in checks["checks"] if check["severity"] == "block"],
        )
        green = []
        for name in blocking:
            with self.subTest(name):
                done = probe(name, SYNTHETIC_RUN)
                if done.returncode == 3:
                    # A class that judges another duty's notes examines nothing of this run.
                    self.assertIn(f"{name}: VOID: examined nothing", done.stdout)
                else:
                    self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                    green.append(name)
        self.assertGreater(len(green), len(RAILS_ROWS), f"the green rows: {green}")


class TheAdapterBuildIsBounded(unittest.TestCase):
    """Proved on planted commands and a recorded call: neither test runs cargo."""

    def test_a_command_that_outlives_its_bound_fails_by_name(self):
        slow = [sys.executable, "-c", "import time; time.sleep(10)"]
        with self.assertRaises(AssertionError) as raised:
            bounded(slow, 0.5, "a planted slow command")
        self.assertEqual(
            str(raised.exception),
            f"a planted slow command did not finish within 0.5 s: {' '.join(slow)}",
        )
        quick = bounded([sys.executable, "-c", "print('done')"], 60, "a planted quick command")
        self.assertEqual((quick.returncode, quick.stdout), (0, "done\n"))

    def test_the_adapter_build_runs_under_its_bound(self):
        recorded = []

        def run(command, **options):
            recorded.append((command, options))
            return subprocess.CompletedProcess(command, 0, '{"file": "clean.md", "rows": []}\n', "")

        with mock.patch.object(subprocess, "run", run):
            verdicts = judged_by_the_adapter([FIXTURES / "clean.md"])
        self.assertEqual(verdicts, {"clean.md": set()})
        self.assertEqual(len(recorded), 1, recorded)
        command, options = recorded[0]
        self.assertEqual(command[:2], ["cargo", "run"])
        self.assertIn("rails_verdicts", command)
        self.assertEqual(options.get("timeout"), CARGO_BOUND_S)
        self.assertEqual(CARGO_BOUND_S, 900)


if __name__ == "__main__":
    unittest.main()

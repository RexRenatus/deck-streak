"""actionlint reads every workflow at its pinned release, and every gate job installs one tool set
(SPEC-663, ADR-685). Both checks start at warn, under ADR-450 D6's promotion rule: a finding or a
parity site that no listing holds prints a warning line and fails nothing, and an empty
population, a saved output that is missing or unread, or an exit other than 0 or 1 is VOID and
fails. The workflows are read through the reader `test_ci_workflows.py` holds. Nothing here runs
actionlint: the `workflow-lint` job runs it and names its saved output by `ACTIONLINT_OUTPUT`."""

import json
import os
import re
import unittest
from pathlib import Path

from _support import REPO, examined
from test_ci_workflows import WORKFLOWS, read_workflow, workflow_file_text, workflow_files

# The level both checks hold (SPEC-663 R6): `warn` prints, `fail` would refuse. A move to `fail`
# is its own delivery under ADR-450 D6; a move back is a weakening.
LEVEL = "warn"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "workflow-lint"
OUTPUT_ENV = "ACTIONLINT_OUTPUT"
# The one actionlint command, word for word (SPEC-663 R1).
ACTIONLINT_WORDS = ["actionlint", "-shellcheck=", "-pyflakes=", "-format", "'{{json", ".}}'"]
# Today's findings at the delivery's base, {(file, line, kind): reason}, each read from the
# delivery's own first `workflow-lint` run and named with that run's id (SPEC-663 R5).
LISTED_FINDINGS = {}
# Today's parity sites, {tool: reason} (SPEC-663 R8).
LISTED_SITES = {}
# The workflow files whose every job is a gate job: the required checks `fragment` and
# `dependency-review` (SPEC-663 R7).
GATE_FILES = ("changelog.yml", "dependency-review.yml")
AGGREGATE = ("ci.yml", "ci")
FINDING_KEYS = ("filepath", "line", "column", "kind", "message")
INSTALL_ACTION = "taiki-e/install-action"
NOT_TOOLS = (
    "actions/checkout",
    "actions/cache",
    "actions/upload-artifact",
    "actions/download-artifact",
)
SETUP = {
    "actions/setup-node": ("node", "node-version-file"),
    "pnpm/action-setup": ("pnpm", "version"),
}
# A checked download's saved file is its `-o` target, and a check names the file it reads.
DOWNLOAD = re.compile(
    r"curl -fsSL -o (\S+) https://github\.com/[^/]+/[^/]+/releases/download/([^/]+)/\S+"
)
CHECKED = re.compile(r'echo "\$([A-Z0-9_]+)_SHA256  (\S+)" \| sha256sum -c -')
CARGO_INSTALL = re.compile(r"cargo install --locked (\S+) --version (\S+)")


def saved_text(path):
    """A saved file's text, or None when no such file is there."""
    if not Path(path).is_file():
        return None
    return Path(path).read_text(encoding="utf-8")


def saved_findings(output):
    """The findings of one saved actionlint run, as (file, line, column, kind), from its JSON
    output and the exit beside it (`actionlint.exit`). VOID, by AssertionError, when either file is
    missing, the exit is not 0 or 1, the output does not read as a list of findings each holding
    every key of FINDING_KEYS, or the exit and the findings disagree."""
    return []


def finding_problems(found, names, listed):
    """The warning lines for `found` against `listed`: each finding no listing holds, and each
    listed finding no longer reported. A finding outside the workflow files `names` is VOID."""
    return []


def gate_jobs(directory):
    """Every gate job of the workflows in `directory`, as (file, job id, job): the aggregate job
    `ci`, every job it needs, and every job of GATE_FILES. Prints `examined N job(s)`; refuses 0."""
    read = {
        path.name: read_workflow(workflow_file_text(path)) for path in workflow_files(directory)
    }
    found = []
    jobs = (read.get(AGGREGATE[0]) or {}).get("jobs") or {}
    if AGGREGATE[1] in jobs:
        needs = jobs[AGGREGATE[1]].get("needs") or []
        for job_id in [*([needs] if isinstance(needs, str) else needs), AGGREGATE[1]]:
            found.append((AGGREGATE[0], job_id, jobs.get(job_id) or {}))
    for name in GATE_FILES:
        for job_id, job in ((read.get(name) or {}).get("jobs") or {}).items():
            found.append((name, job_id, job))
    return examined("job(s)", found)


def unquoted(word):
    """A shell word with its quotes removed, so a download's target and a check's file compare."""
    return word.replace('"', "").replace("'", "")


def installs(job):
    """Each tool one job installs, as (tool, version, route), in step order. A checked download
    takes the tool of the check that names its saved file, or `unchecked` when no check does."""
    found = []
    for step in job.get("steps") or []:
        if not isinstance(step, dict):
            continue
        uses = str(step.get("uses") or "").partition("@")
        given = step.get("with") if isinstance(step.get("with"), dict) else {}
        if uses[0].lower() == INSTALL_ACTION:
            for entry in re.split(r"[,\s]+", str(given.get("tool") or "").strip()):
                if entry:
                    tool, _at, version = entry.partition("@")
                    found.append((tool, version, "install-action"))
        elif uses[0] in SETUP:
            tool, key = SETUP[uses[0]]
            found.append((tool, str(given.get(key) or "package.json"), uses[0]))
        elif uses[0] and not uses[0].startswith(NOT_TOOLS):
            found.append((uses[0], uses[2], "action"))
        lines = [line.strip() for line in str(step.get("run") or "").splitlines()]
        checks = {}
        for line in lines:
            check = CHECKED.fullmatch(line)
            if check:
                checks[unquoted(check.group(2))] = check.group(1).lower()
        for line in lines:
            download = DOWNLOAD.fullmatch(line)
            cargo = CARGO_INSTALL.search(line)
            if download:
                tool = checks.get(unquoted(download.group(1)), "unchecked")
                found.append((tool, download.group(2), "checksum-download"))
            elif cargo:
                found.append((cargo.group(1), cargo.group(2), "cargo-install"))
            elif line == "rustup show":
                found.append(("rust", "rust-toolchain.toml", "toolchain-file"))
    return found


def parity_sites(jobs):
    """The tools held at two versions or by two routes across `jobs`, as (tool, [(version, route,
    file, job)]), sorted by tool. A tool only one job installs, once or more, is never a site."""
    return []


def site_problems(sites, listed):
    """The warning lines for each parity site `listed` does not hold, and each listed tool that is
    no longer a site."""
    warnings = []
    for tool, uses in sites:
        if tool not in listed:
            held = ", ".join(f"{job} {version} by {route}" for version, route, _f, job in uses)
            warnings.append(f"warn: {tool} is held two ways: {held}")
    for tool in sorted(set(listed) - {tool for tool, _uses in sites}):
        warnings.append(f"warn: {tool} is listed and no longer a parity site")
    return warnings


def report(warnings):
    """Print each warning; at `fail`, refuse any."""
    for line in warnings:
        print(line)
    if LEVEL == "fail" and warnings:
        raise AssertionError(f"{len(warnings)} warning(s) at fail")


class TheWorkflowsAreLintedByThePinnedActionlintAtWarn(unittest.TestCase):
    def test_the_workflow_lint_job_runs_the_one_actionlint_command(self):
        jobs = read_workflow(workflow_file_text(WORKFLOWS / "ci.yml"))["jobs"]
        runs = [
            line.split()
            for step in jobs["workflow-lint"]["steps"]
            for line in str(step.get("run") or "").splitlines()
            if line.split()[:1] == ["actionlint"]
        ]
        self.assertEqual([words[: len(ACTIONLINT_WORDS)] for words in runs], [ACTIONLINT_WORDS])

    def test_the_saved_output_is_read_and_each_unlisted_finding_warns(self):
        names = [path.name for path in examined("workflow(s)", workflow_files(WORKFLOWS))]
        output = os.environ.get(OUTPUT_ENV)
        if not output:
            self.skipTest("the saved output exists only in the workflow-lint job")
        report(finding_problems(saved_findings(output), names, LISTED_FINDINGS))

    def test_a_clean_saved_output_reads_no_finding(self):
        self.assertEqual(saved_findings(FIXTURES / "clean" / "actionlint.json"), [])

    def test_a_saved_finding_warns_unless_it_is_listed(self):
        found = saved_findings(FIXTURES / "finding" / "actionlint.json")
        self.assertEqual(
            finding_problems(found, ["planted.yml"], {}),
            ["warn: planted.yml:9:7: syntax-check: no listing holds this finding"],
        )
        listed = {("planted.yml", 9, "syntax-check"): "a planted finding"}
        self.assertEqual(finding_problems(found, ["planted.yml"], listed), [])

    def test_an_unread_saved_output_is_void(self):
        with self.assertRaisesRegex(AssertionError, "VOID: actionlint exited '2'"):
            saved_findings(FIXTURES / "fatal" / "actionlint.json")
        with self.assertRaisesRegex(AssertionError, "VOID: no saved actionlint output"):
            saved_findings(FIXTURES / "empty" / "actionlint.json")


class EveryGateJobInstallsOneToolSet(unittest.TestCase):
    def test_the_gate_jobs_tool_sets_are_read_and_each_unlisted_site_warns(self):
        report(site_problems(parity_sites(gate_jobs(WORKFLOWS)), LISTED_SITES))

    def test_two_jobs_that_install_one_tool_at_one_version_hold_no_site(self):
        self.assertEqual(parity_sites(gate_jobs(FIXTURES / "one-version")), [])

    def test_two_jobs_that_install_one_tool_at_two_versions_are_a_site(self):
        sites = parity_sites(gate_jobs(FIXTURES / "two-versions"))
        self.assertEqual(
            site_problems(sites, {}),
            ["warn: zizmor is held two ways: a 1.0.0 by install-action, b 1.0.1 by install-action"],
        )

    def test_two_jobs_that_install_one_tool_by_two_routes_are_a_site(self):
        sites = parity_sites(gate_jobs(FIXTURES / "two-routes"))
        self.assertEqual(
            site_problems(sites, {}),
            [
                "warn: protoc is held two ways: b 1.0.0 by checksum-download, a 1.0.0 by install-action"
            ],
        )

    def test_one_job_that_downloads_two_tools_names_each_by_its_check_and_holds_no_site(self):
        jobs = gate_jobs(FIXTURES / "one-job-two-tools")
        self.assertEqual(
            [installs(job) for _name, _job_id, job in jobs],
            [
                [
                    ("beta", "2.0.0", "checksum-download"),
                    ("alpha", "1.0.0", "checksum-download"),
                    ("beta", "2.0.1", "install-action"),
                ],
                [],
            ],
        )
        self.assertEqual(parity_sites(jobs), [])

    def test_an_empty_population_is_void(self):
        with self.assertRaisesRegex(AssertionError, "examined 0"):
            gate_jobs(FIXTURES / "empty")


if __name__ == "__main__":
    unittest.main()

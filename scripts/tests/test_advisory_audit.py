"""A scheduled run re-audits dev's dependencies (SPEC-038 R21 to R25, ADR-055's amendment): one
workflow runs the `audit-rust` and `audit-web` stages of `scripts/check.sh` over `dev` once a day
and on dispatch, and on no pull request and no push, with a read-only token that no job widens,
every action pinned by a full commit SHA, every checkout of `dev` without its credentials kept,
nothing that lets an audit fail quietly, the stage logs uploaded whatever the audits read, and the
`cargo-deny` version `ci.yml`'s rust job installs. The census reads every workflow file, `.yml` and
`.yaml` alike, through the reader `test_ci_workflows.py` holds, and a planted workflow for each
weakening shows that it refuses one by name. Nothing here runs a workflow."""

import re
import tempfile
import unittest
from pathlib import Path

from _support import examined
from test_ci_workflows import (
    GATE_CALL,
    INSTALL_ACTION,
    PINNED,
    WORKFLOWS,
    action,
    entries,
    gate_stages,
    read_workflow,
    tool_entries,
    workflow_file_text,
    workflow_files,
)

AUDIT_STAGES = ("audit-rust", "audit-web")
# What a bare `bash scripts/check.sh` runs: every stage check.sh defines.
EVERY_STAGE = "every stage"
EVENTS = ["schedule", "workflow_dispatch"]
TOKEN = {"contents": "read"}
ALWAYS = "${{ always() }}"
CHECKOUT = "actions/checkout"
UPLOAD = "actions/upload-artifact"
# One cron a day: a fixed minute and a fixed hour, every day.
DAILY = re.compile(r"[0-9]{1,2} [0-9]{1,2} \* \* \*")
DENY = "cargo-deny@"
PLANTED_SHA = "0" * 40
# The cargo-deny entry a planted workflow installs, so that no plant depends on what ci.yml holds.
PLANTED_DENY = "cargo-deny@0.0.1"


def mapping(value):
    """A value read as a mapping, or an empty one when it is anything else."""
    return value if isinstance(value, dict) else {}


def jobs_of(workflow):
    """A read workflow's jobs that are mappings, as (job id, job), in order."""
    jobs = mapping(workflow.get("jobs"))
    return [(job_id, job) for job_id, job in jobs.items() if isinstance(job, dict)]


def steps_of(job):
    """A job's steps that are mappings, in order."""
    steps = job.get("steps")
    return [step for step in (steps if isinstance(steps, list) else []) if isinstance(step, dict)]


def stages_run(step):
    """The stages the `bash scripts/check.sh` lines of a step's script run, comments dropped, in
    order: a bare call runs every stage, and is named so."""
    script = re.sub(r"(?m)#.*$", "", str(step.get("run", "")))
    found = []
    for call in GATE_CALL.finditer(script):
        found += call.group(1).split() or [EVERY_STAGE]
    return found


def events_of(workflow):
    """The events a read workflow's `on` names, sorted: a mapping's keys, a list's items, or one
    name."""
    on = workflow.get("on")
    if isinstance(on, (dict, list)):
        return sorted(str(event) for event in on)
    return [str(on)]


def crons_of(workflow):
    """The cron of each schedule a read workflow holds, or none."""
    schedule = mapping(workflow.get("on")).get("schedule")
    items = schedule if isinstance(schedule, list) else []
    return [str(item.get("cron")) for item in items if isinstance(item, dict)]


def runs_an_audit(workflow):
    """Whether a step of a read workflow runs an audit stage through check.sh."""
    return any(
        stage in AUDIT_STAGES or stage == EVERY_STAGE
        for _job_id, job in jobs_of(workflow)
        for step in steps_of(job)
        for stage in stages_run(step)
    )


def scheduled_audits(directory):
    """The workflows of `directory` that run on a schedule and run an audit stage, as (file name,
    read workflow), and every other workflow's cron minutes, as (file name, minute), in the order
    the files are read."""
    audits, minutes = [], []
    for path in workflow_files(directory):
        workflow = read_workflow(workflow_file_text(path))
        if "schedule" in events_of(workflow) and runs_an_audit(workflow):
            audits.append((path.name, workflow))
            continue
        minutes += [(path.name, cron.split()[0]) for cron in crons_of(workflow) if cron.split()]
    return audits, minutes


def ci_deny_pin():
    """The cargo-deny entry ci.yml's rust job installs: the one version both runs of the Rust audit
    read."""
    ci = read_workflow(workflow_file_text(WORKFLOWS / "ci.yml"))
    rust = mapping(mapping(ci.get("jobs")).get("rust"))
    found = [
        entry
        for step in steps_of(rust)
        if action(step).lower() == INSTALL_ACTION
        for entry in tool_entries(step)
        if entry.startswith(DENY)
    ]
    (pin,) = examined("cargo-deny entries of ci.yml's rust job", found)
    return pin


def audit_problems(directory, deny_pin):
    """What the workflows of `directory` get wrong about the scheduled audit, each by name, and
    what was judged: (problems, {population: [...]}). There are no problems when exactly one
    workflow runs an audit stage on a schedule and it meets SPEC-038 R21 to R24, `deny_pin` being
    the cargo-deny entry it must install."""
    judged = {
        "jobs": [],
        "action references": [],
        "checkouts": [],
        "audit steps": [],
        "log uploads": [],
    }
    audits, minutes = scheduled_audits(directory)
    if len(audits) != 1:
        return [f"{len(audits)} scheduled workflows run an audit stage, not 1"], judged
    name, workflow = audits[0]
    problems = []
    events = events_of(workflow)
    if events != EVENTS:
        problems.append(f"{name} runs on {events}, not {EVENTS}")
    crons = crons_of(workflow)
    if len(crons) != 1 or not DAILY.fullmatch(crons[0]):
        problems.append(f"{name} schedules {crons}, not one cron a day")
    else:
        minute = crons[0].split()[0]
        problems += [
            f"{name} schedules minute {minute}, as {other} does"
            for other, theirs in minutes
            if theirs == minute
        ]
    token = workflow.get("permissions")
    if token != TOKEN:
        problems.append(f"{name}: the token is {token}, not {TOKEN}")
    for ref in entries(workflow, "uses"):
        judged["action references"].append(ref)
        if not (isinstance(ref, str) and PINNED.fullmatch(ref)):
            problems.append(f"{name} uses {ref}, not a full commit SHA")
    runs = {stage: [] for stage in AUDIT_STAGES}
    installed, artifacts = [], []
    for job_id, job in jobs_of(workflow):
        where = f"{name}: {job_id}"
        judged["jobs"].append(job_id)
        if "permissions" in job:
            problems.append(f"{where} sets a token of its own: {job['permissions']}")
        for key in ("if", "continue-on-error"):
            if key in job:
                problems.append(f"{where} sets {key}")
        steps = steps_of(job)
        for at, step in enumerate(steps):
            given = mapping(step.get("with"))
            if action(step) == CHECKOUT:
                judged["checkouts"].append(f"{where}: step {at + 1}")
                if str(given.get("persist-credentials")) != "false":
                    problems.append(f"{where} persists the checkout's credentials")
                if given.get("ref") != "dev":
                    problems.append(f"{where} checks out {given.get('ref')}, not dev")
            if action(step).lower() == INSTALL_ACTION:
                installed += [entry for entry in tool_entries(step) if entry.startswith(DENY)]
            if "continue-on-error" in step:
                problems.append(f"{where}: step {at + 1} sets continue-on-error")
            stages = stages_run(step)
            if not stages:
                continue
            judged["audit steps"].append(f"{where}: step {at + 1}")
            for stage in stages:
                if stage in runs:
                    runs[stage].append(job_id)
                else:
                    problems.append(f"{where} runs {stage}, which is not an audit stage")
            if "if" in step:
                problems.append(f"{where}: its audit runs under a condition")
            env = dict(mapping(job.get("env")), **mapping(step.get("env")))
            logs = str(env.get("CHECK_LOG_DIR", "")).rstrip("/")
            kept = [
                later
                for later in steps[at + 1 :]
                if action(later) == UPLOAD
                and later.get("if") == ALWAYS
                and str(mapping(later.get("with")).get("path", "")).rstrip("/") == logs
            ]
            if not logs or len(kept) != 1:
                problems.append(f"{where} uploads no stage logs whatever its audit read")
                continue
            judged["log uploads"].append(f"{where}: {logs}")
            artifacts.append(str(mapping(kept[0].get("with")).get("name")))
    for stage, found in runs.items():
        if len(found) != 1:
            problems.append(f"{name} runs {stage} in {len(found)} job(s), not 1")
    if installed != [deny_pin]:
        problems.append(f"{name} installs {installed}, not ci.yml's {deny_pin}")
    problems += [
        f"{name}: two jobs upload {artifact}"
        for artifact in sorted(set(artifacts))
        if artifacts.count(artifact) > 1
    ]
    return problems, judged


# A planted checkout of dev that keeps no credential, the shape both planted jobs open with.
PLANTED_CHECKOUT = "".join(
    [
        f"      - uses: {CHECKOUT}@{PLANTED_SHA}\n",
        "        with:\n",
        "          ref: dev\n",
        "          persist-credentials: false\n",
    ]
)
# A planted workflow that schedules a cron and runs no audit.
PLANTED_OTHER = "".join(
    [
        "on:\n",
        "  schedule:\n",
        '    - cron: "23 4 * * 1"\n',
        "jobs:\n",
        "  other:\n",
        "    runs-on: ubuntu-24.04\n",
        "    steps:\n",
        "      - run: echo other\n",
    ]
)


def uploaded(side):
    """A planted step that uploads one job's stage logs whatever its audit read."""
    return "".join(
        [
            "      - name: upload stage logs\n",
            "        if: ${{ always() }}\n",
            f"        uses: {UPLOAD}@{PLANTED_SHA}\n",
            "        with:\n",
            f"          name: check-stage-logs-{side}\n",
            "          path: ${{ runner.temp }}/check-logs/\n",
        ]
    )


def audited(pin):
    """A planted workflow that meets every rule, in the shape of the repository's own, installing
    the cargo-deny entry `pin`."""
    return "".join(
        [
            "name: advisory-audit\n",
            "on:\n",
            "  schedule:\n",
            '    - cron: "23 5 * * *"\n',
            "  workflow_dispatch:\n",
            "permissions:\n",
            "  contents: read\n",
            "jobs:\n",
            "  audit-rust:\n",
            "    runs-on: ubuntu-24.04\n",
            "    timeout-minutes: 30\n",
            "    steps:\n",
            PLANTED_CHECKOUT,
            f"      - uses: {INSTALL_ACTION}@{PLANTED_SHA}\n",
            "        with:\n",
            f"          tool: {pin}\n",
            "      - name: audit-rust\n",
            "        env:\n",
            "          CHECK_LOG_DIR: ${{ runner.temp }}/check-logs\n",
            "        run: bash scripts/check.sh audit-rust\n",
            uploaded("rust"),
            "  audit-web:\n",
            "    runs-on: ubuntu-24.04\n",
            "    timeout-minutes: 15\n",
            "    steps:\n",
            PLANTED_CHECKOUT,
            "      - name: audit-web\n",
            "        env:\n",
            "          CHECK_LOG_DIR: ${{ runner.temp }}/check-logs\n",
            "        run: bash scripts/check.sh audit-web\n",
            uploaded("web"),
        ]
    )


def edited(text, old, new):
    """`text` with its one `old` replaced by `new`; a plant whose edit does not find exactly one
    is refused by name, never written."""
    if text.count(old) != 1:
        raise AssertionError(f"a plant finds {text.count(old)} of {old!r}, not 1")
    return text.replace(old, new)


class DevIsReauditedOnASchedule(unittest.TestCase):
    def test_the_workflows_hold_one_scheduled_audit_that_meets_every_rule(self):
        stages = examined("check.sh stages", gate_stages())
        for stage in examined("audit stages", list(AUDIT_STAGES)):
            self.assertIn(stage, stages, f"check.sh defines no stage {stage}")
        problems, judged = audit_problems(WORKFLOWS, ci_deny_pin())
        self.assertEqual(problems, [])
        audits, minutes = scheduled_audits(WORKFLOWS)
        self.assertEqual([name for name, _workflow in audits], ["advisory-audit.yml"])
        examined("other workflows' cron minutes", minutes)
        for population, found in judged.items():
            examined(population, found)

    def test_each_planted_weakening_is_refused_by_name(self):
        good = audited(PLANTED_DENY)
        name = "audit.yml"
        dispatch = "  workflow_dispatch:\n"
        cron = '    - cron: "23 5 * * *"\n'
        top = "permissions:\n  contents: read\n"
        rust = "    timeout-minutes: 30\n"
        web = "  audit-web:\n    runs-on: ubuntu-24.04\n"
        writes = "  audit-web:\n    permissions:\n      contents: read\n      issues: write\n"
        opened = "    timeout-minutes: 30\n    steps:\n" + PLANTED_CHECKOUT
        kept = "          persist-credentials: false\n"
        install = f"      - uses: {INSTALL_ACTION}@{PLANTED_SHA}\n"
        tool = f"          tool: {PLANTED_DENY}\n"
        gate = "      - name: audit-rust\n"
        logs = "        env:\n          CHECK_LOG_DIR: ${{ runner.temp }}/check-logs\n"
        rust_call = "        run: bash scripts/check.sh audit-rust\n"
        web_call = "        run: bash scripts/check.sh audit-web\n"
        always = "        if: " + ALWAYS + "\n"
        rust_upload, web_upload = uploaded("rust"), uploaded("web")
        events = f", not {EVENTS}"
        silent = f"{name}: audit-rust uploads no stage logs whatever its audit read"
        installs = f", not ci.yml's {PLANTED_DENY}"
        issues = "{'contents': 'read', 'issues': 'write'}"
        plants = {
            "the audited shape": ({name: good}, []),
            "the audited shape saved as .yaml": ({"audit.yaml": good}, []),
            "no schedule": (
                {name: edited(good, "  schedule:\n" + cron, "")},
                ["0 scheduled workflows run an audit stage, not 1"],
            ),
            "two scheduled audits": (
                {name: good, "again.yaml": good},
                ["2 scheduled workflows run an audit stage, not 1"],
            ),
            "a pull request trigger": (
                {name: edited(good, dispatch, dispatch + "  pull_request:\n    branches: [dev]\n")},
                [f"{name} runs on ['pull_request', 'schedule', 'workflow_dispatch']{events}"],
            ),
            "a push trigger": (
                {name: edited(good, dispatch, dispatch + "  push:\n    branches: [dev]\n")},
                [f"{name} runs on ['push', 'schedule', 'workflow_dispatch']{events}"],
            ),
            "no dispatch": (
                {name: edited(good, dispatch, "")},
                [f"{name} runs on ['schedule']{events}"],
            ),
            "a cron every hour": (
                {name: edited(good, '"23 5 * * *"', '"23 * * * *"')},
                [f"{name} schedules ['23 * * * *'], not one cron a day"],
            ),
            "a minute another workflow schedules": (
                {name: good, "other.yml": PLANTED_OTHER},
                [f"{name} schedules minute 23, as other.yml does"],
            ),
            "a token that writes": (
                {name: edited(good, top, "permissions:\n  contents: write\n")},
                [f"{name}: the token is {{'contents': 'write'}}, not {TOKEN}"],
            ),
            "no token": (
                {name: edited(good, top, "")},
                [f"{name}: the token is None, not {TOKEN}"],
            ),
            "a job that writes issues": (
                {name: edited(good, web, writes + "    runs-on: ubuntu-24.04\n")},
                [f"{name}: audit-web sets a token of its own: {issues}"],
            ),
            "a tag in place of a SHA": (
                {name: edited(good, install, f"      - uses: {INSTALL_ACTION}@v2\n")},
                [f"{name} uses {INSTALL_ACTION}@v2, not a full commit SHA"],
            ),
            "a persisted credential": (
                {name: edited(good, opened, opened.replace("false", "true"))},
                [f"{name}: audit-rust persists the checkout's credentials"],
            ),
            "credentials left to the checkout's default": (
                {name: edited(good, opened, opened.replace(kept, ""))},
                [f"{name}: audit-rust persists the checkout's credentials"],
            ),
            "a checkout of main": (
                {name: edited(good, opened, opened.replace("ref: dev", "ref: main"))},
                [f"{name}: audit-rust checks out main, not dev"],
            ),
            "audit-web left out": (
                {name: edited(good, web_call, "        run: pnpm audit\n")},
                [f"{name} runs audit-web in 0 job(s), not 1"],
            ),
            "both audits in one job": (
                {name: edited(good, rust_call, rust_call.replace("rust", "rust audit-web"))},
                [f"{name} runs audit-web in 2 job(s), not 1"],
            ),
            "a stage that is not an audit": (
                {name: edited(good, rust_call, rust_call.replace("rust", "rust test"))},
                [f"{name}: audit-rust runs test, which is not an audit stage"],
            ),
            "every stage": (
                {name: edited(good, rust_call, "        run: bash scripts/check.sh\n")},
                [
                    f"{name}: audit-rust runs every stage, which is not an audit stage",
                    f"{name} runs audit-rust in 0 job(s), not 1",
                ],
            ),
            "an audit that continues on error": (
                {name: edited(good, gate, gate + "        continue-on-error: true\n")},
                [f"{name}: audit-rust: step 3 sets continue-on-error"],
            ),
            "a job that continues on error": (
                {name: edited(good, rust, rust + "    continue-on-error: true\n")},
                [f"{name}: audit-rust sets continue-on-error"],
            ),
            "a condition on the job": (
                {name: edited(good, rust, rust + "    if: false\n")},
                [f"{name}: audit-rust sets if"],
            ),
            "a condition on the audit": (
                {name: edited(good, gate, gate + "        if: false\n")},
                [f"{name}: audit-rust: its audit runs under a condition"],
            ),
            "no upload": ({name: edited(good, rust_upload, "")}, [silent]),
            "an upload on success alone": (
                {name: edited(good, rust_upload, rust_upload.replace(always, ""))},
                [silent],
            ),
            "an upload of another directory": (
                {name: edited(good, rust_upload, rust_upload.replace("check-", "other-"))},
                [silent],
            ),
            "no log directory": ({name: edited(good, gate + logs, gate)}, [silent]),
            "one artifact name for both jobs": (
                {name: edited(good, web_upload, rust_upload)},
                [f"{name}: two jobs upload check-stage-logs-rust"],
            ),
            "a cargo-deny version of its own": (
                {name: edited(good, tool, "          tool: cargo-deny@0.0.2\n")},
                [f"{name} installs ['cargo-deny@0.0.2']{installs}"],
            ),
            "no cargo-deny": (
                {name: edited(good, install + "        with:\n" + tool, "")},
                [f"{name} installs []{installs}"],
            ),
        }
        for label, (files, refusal) in examined("planted audits", list(plants.items())):
            with self.subTest(label), tempfile.TemporaryDirectory() as scratch:
                for file_name, text in files.items():
                    (Path(scratch) / file_name).write_text(text, encoding="utf-8")
                problems, _judged = audit_problems(Path(scratch), PLANTED_DENY)
                self.assertEqual(problems, refusal)


if __name__ == "__main__":
    unittest.main()

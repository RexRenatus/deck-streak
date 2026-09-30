"""SPEC-087 amendment: the verdict step fails when any one judge fails (#454).

The class rule: every exit the verdict step collects is folded into the step's status, so for each
command the step runs, the step fails when that command alone fails and every other one succeeds.
The commands are read from the step's own script, and the non-zero exits from the verdict tool's
own exit constants; nothing here lists a judge by name.
"""

import os
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_mutation_workflows import CI, jobs, steps, workflow
from test_not_started_legs import bash, needs_of, rendered_env, shell_of, step_script

TOOL = REPO / "scripts" / "mutation-verdict.py"
RUN = "python3 scripts/mutation-verdict.py"
#: The shim answers every call of the verdict tool with success, logs it, and fails the ONE command
#: whose key (its verb, and its class when it has one) is `FAIL_KEY`, with the exit `FAIL_RC`.
SHIM = """#!/bin/bash
printf '%s\\n' "$*" >> "$SHIM_LOG"
key=$2
previous=""
for arg in "$@"; do
  if [ "$previous" = "--class" ]; then key="$key:$arg"; fi
  previous=$arg
done
if [ "$key" = "$FAIL_KEY" ]; then exit "$FAIL_RC"; fi
exit 0
"""


def verdict_step():
    job = jobs(workflow(CI)).get("mutation-verdict", "")
    (step,) = [s for s in steps(job) if f"{RUN} judge" in s]
    return job, step


def commands_of(script):
    """[(key, the variable its exit is collected into, or None, its line)] per verdict command."""
    found = []
    for line in script.splitlines():
        if RUN not in line:
            continue
        verb = re.search(rf"{re.escape(RUN)} (\w+)", line).group(1)
        klass = re.search(r"--class (\w+)", line)
        key = f"{verb}:{klass.group(1)}" if klass else verb
        into = re.search(r"\|\| (\w+)=\$\?", line)
        found.append((key, into.group(1) if into else None, line))
    return found


def census(script):
    """Every command whose exit the step captures, each one a command the harness can drive.

    A capture (`$?` or PIPESTATUS) on a line that runs no verdict-tool command is a command the
    shim cannot fail, so it is refused by name rather than passed over.
    """
    for line in script.splitlines():
        if RUN not in line and re.search(r"\$\?|PIPESTATUS", line):
            raise AssertionError(f"the step captures an exit the test cannot drive: {line.strip()}")
    return commands_of(script)


def nonzero_exits():
    """The tool's non-zero exits, read from its own constants."""
    text = TOOL.read_text(encoding="utf-8")
    names = re.search(r"^(EXIT_[A-Z_, ]+) = ([0-9, ]+)$", text, re.MULTILINE)
    values = dict(zip(names.group(1).split(", "), map(int, names.group(2).split(", "))))
    return sorted(value for value in values.values() if value != 0)


#: Each pass-through pipe form, inserted before a command's `||` capture, `{n}` numbering a log.
PIPES = [' | tee "$RUNNER_TEMP/{n}.log"', " | cat", " 2>&1 | cat"]


def keyed(script):
    """[(key, line)] for each verdict-tool command."""
    return [(key, line) for key, _, line in census(script)]


def piped(script, key, form, n):
    """The script with `form` piped onto the one command whose key is `key`, in memory."""
    lines = script.splitlines()
    (where,) = [i for i, line in enumerate(lines) if (key, line) in keyed(script)]
    head, tail = lines[where].rsplit(" || ", 1)
    lines[where] = f"{head}{form.format(n=n)} || {tail}"
    return "\n".join(lines) + "\n"


class Driver:
    """Runs the verdict step's script with the shim standing in for the verdict tool."""

    def __init__(self, case):
        scratch = tempfile.TemporaryDirectory()
        case.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        (self.root / "bin").mkdir()
        shim = self.root / "bin" / "python3"
        shim.write_text(SHIM, encoding="utf-8")
        shim.chmod(0o700)
        self.runs = 0
        self.job, self.step = verdict_step()

    def run(self, script, key, rc):
        results = {
            "mutation-plan": "success",
            "mutation-rust": "success",
            "mutation-rows": "success",
        }
        self.runs += 1
        log = self.root / f"calls-{self.runs}.log"
        env = dict(
            rendered_env(self.step, needs_of(self.job), results),
            PATH=f"{self.root / 'bin'}{os.pathsep}{os.environ['PATH']}",
            RUNNER_TEMP=str(self.root),
            SHIM_LOG=str(log),
            FAIL_KEY=key or "",
            FAIL_RC=str(rc),
        )
        done = bash(script, env, shell_of(self.step, self.job, workflow(CI)))
        return done, log.read_text(encoding="utf-8").splitlines()


class TheVerdictStepFailsOnEachJudgeAlone(unittest.TestCase):
    def test_the_step_fails_when_any_one_command_alone_fails_with_any_of_its_exits(self):
        driver = Driver(self)
        script = step_script(driver.step)
        commands = census(script)
        print(f"found {len(commands)} verdict-tool commands in the step: {commands}")
        self.assertEqual(len(commands), script.count(RUN), "a command was not read")
        self.assertGreater(len(commands), 0)
        for key, into, _ in commands:
            self.assertIsNotNone(into, f"{key}: the step records its exit nowhere")
        exits = nonzero_exits()
        self.assertEqual(exits[0], 1)
        population = [("no command", None, 0)] + [
            (f"{key} alone, exit {rc}", key, rc) for key, _, _ in commands for rc in exits
        ]
        for where, key, rc in examined("verdict step runs", population):
            done, calls = driver.run(script, key, rc)
            self.assertEqual(done.returncode, rc, f"{where}: {done.stdout}{done.stderr}")
            self.assertEqual(len(calls), len(commands), f"{where}: a command never ran: {calls}")

    def test_the_harness_sees_a_pipe_that_loses_a_recorded_exit(self):
        driver = Driver(self)
        script = step_script(driver.step)
        commands = census(script)
        exits = nonzero_exits()
        pairs = [(key, form) for key, _, _ in commands for form in PIPES]
        pairs = examined("pass-through pipes applied to the step's commands", pairs)
        self.assertEqual(len(pairs), len(commands) * len(PIPES))
        for n, (key, form) in enumerate(pairs):
            mutated = piped(script, key, form, n)
            self.assertNotEqual(mutated, script, f"{key}: the pipe was not applied")
            for rc in exits:
                done, _ = driver.run(mutated, key, rc)
                self.assertNotEqual(
                    done.returncode, rc, f"{key} piped through{form} kept its exit {rc}"
                )

    def test_a_captured_command_the_harness_cannot_drive_is_refused_by_name(self):
        script = step_script(verdict_step()[1])
        planted = script.replace('exit "$status"', 'other=0\npytest -q || other=$?\nexit "$status"')
        self.assertNotEqual(planted, script)
        with self.assertRaises(AssertionError) as refused:
            census(planted)
        print(f"census refused: {refused.exception}")
        self.assertIn("pytest -q || other=$?", str(refused.exception))


def planted(workflow_shell=None, job_shell=None, step_shell=None):
    """A workflow's text with a `shell:` at each named placement, and its job and step."""
    header = f"defaults:\n  run:\n    shell: {workflow_shell}\n" if workflow_shell else ""
    defaults = f"    defaults:\n      run:\n        shell: {job_shell}\n" if job_shell else ""
    own = f"        shell: {step_shell}\n" if step_shell else ""
    text = (
        f"name: t\non: push\n{header}jobs:\n  a:\n    runs-on: x\n{defaults}    steps:\n"
        f"      - name: s\n{own}        run: echo hi\n"
    )
    job = jobs(text)["a"]
    return text, job, steps(job)[0]


class TheShellIsResolvedFromTheWorkflowText(unittest.TestCase):
    def test_each_placement_and_the_precedence_resolve_to_the_shell_github_runs(self):
        explicit = ["bash", "--noprofile", "--norc", "-eo", "pipefail"]
        cases = [
            ("a step's shell", ("", "", "bash"), explicit),
            ("a job's default", ("", "bash", ""), explicit),
            ("the workflow's default", ("bash", "", ""), explicit),
            ("no shell named", ("", "", ""), ["bash", "-e"]),
            ("the step over the job", ("", "sh", "bash"), explicit),
            ("the job over the workflow", ("bash", "sh", ""), ["sh", "-e"]),
            ("the step over the job and the workflow", ("sh", "sh", "bash"), explicit),
            ("the workflow's, the job silent", ("sh", "", ""), ["sh", "-e"]),
        ]
        for where, (in_workflow, in_job, in_step), argv in examined("shell placements", cases):
            text, job, step = planted(in_workflow, in_job, in_step)
            self.assertEqual(shell_of(step, job, text), argv, where)

    def test_a_shell_no_scenario_models_is_refused_by_name(self):
        text, job, step = planted(step_shell="pwsh")
        with self.assertRaises(AssertionError) as refused:
            shell_of(step, job, text)
        self.assertIn("pwsh", str(refused.exception))

    def test_the_verdict_step_runs_under_bash_e_while_ci_names_no_shell(self):
        job, step = verdict_step()
        self.assertEqual(shell_of(step, job, workflow(CI)), ["bash", "-e"])


if __name__ == "__main__":
    unittest.main()

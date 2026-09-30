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
from test_not_started_legs import bash, needs_of, rendered_env, step_script

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
    """[(key, the variable its exit is collected into, or None)] for each verdict-tool command."""
    found = []
    for line in script.splitlines():
        if RUN not in line:
            continue
        verb = re.search(rf"{re.escape(RUN)} (\w+)", line).group(1)
        klass = re.search(r"--class (\w+)", line)
        key = f"{verb}:{klass.group(1)}" if klass else verb
        into = re.search(r"\|\| (\w+)=\$\?", line)
        found.append((key, into.group(1) if into else None))
    return found


def nonzero_exits():
    """The tool's non-zero exits, read from its own constants."""
    text = TOOL.read_text(encoding="utf-8")
    names = re.search(r"^(EXIT_[A-Z_, ]+) = ([0-9, ]+)$", text, re.MULTILINE)
    values = dict(zip(names.group(1).split(", "), map(int, names.group(2).split(", "))))
    return sorted(value for value in values.values() if value != 0)


class TheVerdictStepFailsOnEachJudgeAlone(unittest.TestCase):
    def test_the_step_fails_when_any_one_command_alone_fails_with_any_of_its_exits(self):
        job, step = verdict_step()
        script = step_script(step)
        commands = commands_of(script)
        print(f"found {len(commands)} verdict-tool commands in the step: {commands}")
        self.assertEqual(len(commands), script.count(RUN), "a command was not read")
        self.assertGreater(len(commands), 0)
        for key, into in commands:
            self.assertIsNotNone(into, f"{key}: the step records its exit nowhere")
        exits = nonzero_exits()
        self.assertEqual(exits[0], 1)
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        (root / "bin").mkdir()
        shim = root / "bin" / "python3"
        shim.write_text(SHIM, encoding="utf-8")
        shim.chmod(0o700)
        results = {
            "mutation-plan": "success",
            "mutation-rust": "success",
            "mutation-rows": "success",
        }
        population = [("no command", None, 0)] + [
            (f"{key} alone, exit {rc}", key, rc) for key, _ in commands for rc in exits
        ]
        for n, (where, key, rc) in enumerate(examined("verdict step runs", population)):
            log = root / f"calls-{n}.log"
            env = dict(
                rendered_env(step, needs_of(job), results),
                PATH=f"{root / 'bin'}{os.pathsep}{os.environ['PATH']}",
                RUNNER_TEMP=str(root),
                SHIM_LOG=str(log),
                FAIL_KEY=key or "",
                FAIL_RC=str(rc),
            )
            done = bash(script, env)
            self.assertEqual(done.returncode, rc, f"{where}: {done.stdout}{done.stderr}")
            calls = log.read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(calls), len(commands), f"{where}: a command never ran: {calls}")


if __name__ == "__main__":
    unittest.main()

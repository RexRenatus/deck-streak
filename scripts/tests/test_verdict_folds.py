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


def logical_lines(script):
    """The script's commands as the shell reads them: a backslash-newline joins two physical lines,
    and blank and comment-only lines are not commands."""
    joined, held = [], ""
    for physical in script.splitlines():
        line = physical.strip()
        if not held and (not line or line.startswith("#")):
            continue
        line = held + line
        if (len(line) - len(line.rstrip("\\"))) % 2 == 1:
            held = line[:-1]
            continue
        held = ""
        joined.append(line)
    if held:
        joined.append(held)
    return joined


#: The only shapes of command this harness judges, each matched whole. A line of any other shape is
#: a command whose exit may not reach the step's exit unaltered (a condition, a negation, a pipe,
#: `$(...)`, a backgrounded or `||`-ed command, a `set +e` block), so it is refused by name.
SHAPES = [
    re.compile(r'[a-z_]+=("[^"$`\\]*"|"\$RUNNER_TEMP/[^"`$\\]*"|[0-9]+)'),
    re.compile(rf"{re.escape(RUN)} [^|;&`!<>()]* \|\| [a-z_]+=\$\?"),
    re.compile(r'if \[ "\$status" -eq 0 \]; then status=\$[a-z_]+; fi'),
    re.compile(r'exit "\$status"'),
]


def commands_of(script):
    """[(key, the variable its exit is collected into, or None, its line)] per verdict command."""
    found = []
    for line in logical_lines(script):
        if RUN not in line:
            continue
        verb = re.search(rf"{re.escape(RUN)} (\w+)", line).group(1)
        klass = re.search(r"--class (\w+)", line)
        key = f"{verb}:{klass.group(1)}" if klass else verb
        into = re.search(r"\|\| (\w+)=\$\?", line)
        found.append((key, into.group(1) if into else None, line))
    return found


def census(script):
    """Every verdict-tool command of the step, after judging EVERY line by its shape.

    Default-deny: a line that matches no known shape is refused by name, whether or not it holds
    `$?` or PIPESTATUS, because a command's place in the script (not its text) decides whether its
    exit reaches the step's.
    """
    for line in logical_lines(script):
        if not any(shape.fullmatch(line) for shape in SHAPES):
            raise AssertionError(f"the step runs a line the test cannot drive or judge: {line}")
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
    lines = logical_lines(script)
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
        shell = shell_of(driver.step, driver.job, workflow(CI))
        keeps = "pipefail" in shell
        pairs = [(key, form) for key, _, _ in commands for form in PIPES]
        pairs = examined("pass-through pipes applied to the step's commands", pairs)
        self.assertEqual(len(pairs), len(commands) * len(PIPES))
        for n, (key, form) in enumerate(pairs):
            mutated = piped(script, key, form, n)
            self.assertNotEqual(mutated, script, f"{key}: the pipe was not applied")
            for rc in exits:
                done, _ = driver.run(mutated, key, rc)
                # the expectation is the resolved shell's: pipefail keeps a piped command's exit,
                # and `-e` alone loses it to the pass-through's own success
                if keeps:
                    self.assertEqual(done.returncode, rc, f"{key}{form} under {shell} lost {rc}")
                else:
                    self.assertNotEqual(done.returncode, rc, f"{key}{form} under {shell} kept {rc}")


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

    def test_the_resolver_reads_the_verdict_step_of_the_workflow_it_is_given(self):
        job, step = verdict_step()
        argv = shell_of(step, job, workflow(CI))
        self.assertIn(argv[0], ("bash", "sh"), argv)


# ---- the class rule, generated: the harness sees what CI sees, or refuses by name --------------

STEP_AT = "      - name: the verdict, from every shard's report and the rows' (the plan names a not-applicable case)\n"
RUN_END = '          exit "$status"\n'
JOB_AT = "  mutation-verdict:\n    needs: [mutation-plan, mutation-rust, mutation-python, mutation-rows]\n"
WF_AT = "permissions:\n  contents: read\n"
FIRST_JOB = "jobs:\n  rust:\n"
EXPLICIT_BASH = ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail"]
UNSPECIFIED = ["bash", "-e"]


def _once(text, anchor):
    assert text.count(anchor) == 1, anchor
    return text


def _after(anchor, added):
    return lambda text: _once(text, anchor).replace(anchor, anchor + added)


def _replace(old, new):
    return lambda text: _once(text, old).replace(old, new)


def _seq(*edits):
    def apply(text):
        for edit in edits:
            text = edit(text)
        return text

    return apply


def _step(value):
    return _after(STEP_AT, f"        shell: {value}\n")


def _job(value):
    return _after(JOB_AT, f"    defaults:\n      run:\n        shell: {value}\n")


def _flow(block):
    return _after(JOB_AT, block)


def _flow_wf(block):
    return _after(WF_AT, block)


def _wf(value):
    return _flow_wf(f"defaults:\n  run:\n    shell: {value}\n")


#: (name, the edit to ci.yml's text, the argv GitHub runs or None when it runs nothing the harness
#: models, and whether the resolver must READ it (else refusing by name is also safe)).
PLACEMENTS = [
    ("step bash", _step("bash"), EXPLICIT_BASH, True),
    ("step sh", _step("sh"), ["sh", "-e"], True),
    ("step pwsh", _step("pwsh"), None, False),
    ("step python", _step("python"), None, False),
    ("step powershell", _step("powershell"), None, False),
    ("step cmd", _step("cmd"), None, False),
    ("step template bash {0}", _step("bash {0}"), ["bash"], True),
    ("step template bash -e {0}", _step("bash -e {0}"), ["bash", "-e"], True),
    (
        "step template bash -eo pipefail {0}",
        _step("bash -eo pipefail {0}"),
        ["bash", "-e", "-o", "pipefail"],
        True,
    ),
    ("step template sh {0}", _step("sh {0}"), ["sh"], True),
    ("step template perl {0}", _step("perl {0}"), ["perl"], False),
    ("step template quoted 'bash {0}'", _step("'bash {0}'"), ["bash"], False),
    ('step "bash" (double-quoted)', _step('"bash"'), EXPLICIT_BASH, False),
    ("step 'sh' (single-quoted)", _step("'sh'"), ["sh", "-e"], False),
    ("step Bash (case)", _step("Bash"), None, False),
    ("step BASH (case)", _step("BASH"), None, False),
    ("step bash with a trailing comment", _step("bash  # explicit"), EXPLICIT_BASH, False),
    ("step sh with a trailing comment", _step("sh  # posix"), ["sh", "-e"], False),
    ("step two spaces before bash", _step(" bash"), EXPLICIT_BASH, False),
    ("step bash then trailing spaces", _step("bash   "), EXPLICIT_BASH, False),
    (
        "step block scalar >- bash",
        _after(STEP_AT, "        shell: >-\n          bash\n"),
        EXPLICIT_BASH,
        False,
    ),
    (
        "step block scalar | bash {0}",
        _after(STEP_AT, "        shell: |-\n          bash {0}\n"),
        ["bash"],
        False,
    ),
    (
        "step bash after the run block",
        _after(RUN_END, "        shell: bash\n"),
        EXPLICIT_BASH,
        True,
    ),
    ("step sh after the run block", _after(RUN_END, "        shell: sh\n"), ["sh", "-e"], True),
    ("job bash", _job("bash"), EXPLICIT_BASH, True),
    ("job sh", _job("sh"), ["sh", "-e"], True),
    ("job pwsh", _job("pwsh"), None, False),
    (
        "job defaults run without shell",
        _flow("    defaults:\n      run:\n        working-directory: .\n"),
        UNSPECIFIED,
        True,
    ),
    (
        "job bash after working-directory",
        _flow("    defaults:\n      run:\n        working-directory: .\n        shell: bash\n"),
        EXPLICIT_BASH,
        True,
    ),
    ("job flow map defaults", _flow("    defaults: {run: {shell: bash}}\n"), EXPLICIT_BASH, False),
    ("job flow map run", _flow("    defaults:\n      run: {shell: bash}\n"), EXPLICIT_BASH, False),
    ("job bash with a comment", _job("bash  # c"), EXPLICIT_BASH, False),
    ("job template bash {0}", _job("bash {0}"), ["bash"], True),
    ("workflow bash", _wf("bash"), EXPLICIT_BASH, True),
    ("workflow sh", _wf("sh"), ["sh", "-e"], True),
    (
        "workflow defaults run without shell",
        _flow_wf("defaults:\n  run:\n    working-directory: .\n"),
        UNSPECIFIED,
        False,
    ),
    (
        "workflow bash after working-directory",
        _flow_wf("defaults:\n  run:\n    working-directory: .\n    shell: bash\n"),
        EXPLICIT_BASH,
        True,
    ),
    (
        "workflow flow map defaults",
        _flow_wf("defaults: {run: {shell: bash}}\n"),
        EXPLICIT_BASH,
        False,
    ),
    (
        "workflow anchored defaults",
        _flow_wf("defaults: &d\n  run:\n    shell: bash\n"),
        EXPLICIT_BASH,
        False,
    ),
    (
        "workflow anchored shell value",
        _flow_wf("defaults:\n  run:\n    shell: &s bash\n"),
        EXPLICIT_BASH,
        False,
    ),
    (
        "workflow defaults with a comment",
        _flow_wf("defaults:  # every run step\n  run:\n    shell: bash\n"),
        EXPLICIT_BASH,
        False,
    ),
    (
        "workflow defaults after jobs (key order)",
        lambda text: text.rstrip("\n") + "\ndefaults:\n  run:\n    shell: bash\n",
        EXPLICIT_BASH,
        True,
    ),
    ("workflow template bash {0}", _wf("bash {0}"), ["bash"], True),
    ("step sh over job bash", _seq(_step("sh"), _job("bash")), ["sh", "-e"], True),
    ("step bash over job sh", _seq(_step("bash"), _job("sh")), EXPLICIT_BASH, True),
    ("job sh over workflow bash", _seq(_job("sh"), _wf("bash")), ["sh", "-e"], True),
    ("job bash over workflow sh", _seq(_job("bash"), _wf("sh")), EXPLICIT_BASH, True),
    (
        "step sh over job bash over workflow bash",
        _seq(_step("sh"), _job("bash"), _wf("bash")),
        ["sh", "-e"],
        True,
    ),
    (
        "step 'sh # c' over workflow bash",
        _seq(_step("sh  # posix"), _wf("bash")),
        ["sh", "-e"],
        False,
    ),
    ("step template sh {0} over workflow bash", _seq(_step("sh {0}"), _wf("bash")), ["sh"], True),
    ("step template bash {0} over job bash", _seq(_step("bash {0}"), _job("bash")), ["bash"], True),
    ("step two-space sh over job bash", _seq(_step(" sh"), _job("bash")), ["sh", "-e"], False),
    (
        "step sh after run over job bash",
        _seq(_after(RUN_END, "        shell: sh\n"), _job("bash")),
        ["sh", "-e"],
        True,
    ),
    (
        "job sh after working-directory over workflow bash",
        _seq(
            _flow("    defaults:\n      run:\n        working-directory: .\n        shell: sh\n"),
            _wf("bash"),
        ),
        ["sh", "-e"],
        True,
    ),
    (
        "job flow-map sh over workflow bash",
        _seq(_flow("    defaults: {run: {shell: sh}}\n"), _wf("bash")),
        ["sh", "-e"],
        False,
    ),
    (
        "job alias to an anchored sh over workflow bash",
        _seq(
            _replace(
                FIRST_JOB, FIRST_JOB + "    defaults: &posix\n      run:\n        shell: sh\n"
            ),
            _flow("    defaults: *posix\n"),
            _wf("bash"),
        ),
        ["sh", "-e"],
        False,
    ),
    (
        "step alias *s to workflow &s bash",
        _seq(_flow_wf("defaults:\n  run:\n    shell: &s bash\n"), _step("*s")),
        EXPLICIT_BASH,
        False,
    ),
]


def resolved_by(text):
    """('argv', argv) when the resolver reads the verdict step's shell, ('refused', why) when it
    refuses by name, and ('crashed', why) when it fails any other way (never a safe answer)."""
    job = jobs(text)["mutation-verdict"]
    (step,) = [s for s in steps(job) if f"{RUN} judge" in s]
    try:
        return "argv", shell_of(step, job, text)
    except AssertionError as refused:
        return "refused", str(refused)
    except Exception as other:  # noqa: BLE001 - any other failure is the finding
        return "crashed", f"{type(other).__name__}: {other}"


def normalised(argv):
    """An argv with a combined `-eo` split, so `-eo pipefail` and `-e -o pipefail` compare equal."""
    return [flag for word in argv for flag in (["-e", "-o"] if word == "-eo" else [word])]


def without_shells(text):
    """The workflow's text with the verdict step's own `shell:` line, its job's `defaults:` block
    and the workflow's top-level `defaults:` block removed, so each placement is planted onto a
    text that names none (and a workflow that later names one still plants cleanly)."""
    job = jobs(text)["mutation-verdict"]
    bare = re.sub(r"(?m)^        shell: .*\n", "", job)
    bare = re.sub(r"(?m)^    defaults:\n      run:\n(?:        .*\n)*", "", bare)
    text = text.replace(job, bare)
    return re.sub(r"(?m)^defaults:\n  run:\n(?:    .*\n)*", "", text)


class TheResolverReadsWhatGitHubRunsOrRefusesByName(unittest.TestCase):
    def test_every_placement_of_the_shell_is_read_correctly_or_refused_by_name(self):
        base = without_shells(workflow(CI))
        self.assertEqual(resolved_by(base), ("argv", UNSPECIFIED))
        tally = {"CORRECT": 0, "SAFE": 0, "ESCAPE": 0}
        escapes = []
        must_read = 0
        for name, edit, truth, read in examined("resolver placements", PLACEMENTS):
            text = edit(base)
            self.assertNotEqual(text, base, name)
            kind, got = resolved_by(text)
            must_read += read
            if kind == "refused":
                verdict = "SAFE"
                if read:
                    escapes.append(f"{name}: refused a form it must read: {got}")
            elif kind == "crashed":
                verdict = "ESCAPE"
                escapes.append(f"{name}: failed without naming the form: {got}")
            elif truth is not None and normalised(got) == normalised(truth):
                verdict = "CORRECT"
            else:
                verdict = "ESCAPE"
                escapes.append(f"{name}: read {got} where GitHub runs {truth}")
            tally[verdict] += 1
        print(f"resolver placements: {tally}")
        self.assertEqual(escapes, [], f"{len(escapes)} placements escaped: {escapes[:3]}")
        self.assertGreaterEqual(tally["CORRECT"], must_read)
        self.assertEqual(tally["ESCAPE"], 0)
        self.assertGreater(tally["CORRECT"], 0)


def _before_legs(*lines, fold=False):
    def edit(script):
        block = "other=0\n" + "".join(f"{line}\n" for line in lines)
        if fold:
            block += 'if [ "$status" -eq 0 ]; then status=$other; fi\n'
        return _at_line(script, "legs=0", block)

    return edit


def _at_line(script, line, block):
    hits = [m for m in re.finditer(rf"(?m)^([ \t]*){re.escape(line)}\n", script)]
    assert len(hits) == 1, line
    indent = hits[0].group(1)
    indented = "".join(indent + part + "\n" for part in block.splitlines())
    return script[: hits[0].start()] + indented + script[hits[0].start() :]


def _edit_line(script, old, new):
    assert script.count(old) == 1, old
    return script.replace(old, new)


SCRIPTS_JUDGE = 'python3 scripts/mutation-verdict.py judge --plan "$reports/mutation-plan/plan.json" --class scripts'
SCRIPTS_TAIL = (
    ' --python "$reports" --rows "$reports/mutation-rows/rows.json"'
    ' --python-whole "$reports/mutation-plan/python-whole.json"'
)

#: (name, the edit to the step's script, the text a refusal must quote, whether the harness drives it)
CENSUS_PLANTS = [
    ("N1 `; other=$?`", _before_legs("plantcheck; other=$?"), "plantcheck; other=$?", False),
    (
        "N2 `if ! cmd; then other=1; fi`",
        _before_legs("if ! plantcheck; then other=1; fi"),
        "if ! plantcheck",
        False,
    ),
    (
        "N2m `if ! cmd; then` multi-line",
        _before_legs("if ! plantcheck; then", "  other=1", "fi"),
        "if ! plantcheck",
        False,
    ),
    ("N3 `cmd || true`", _before_legs("plantcheck || true"), "plantcheck || true", False),
    ("N4 `set +e` block", _before_legs("set +e", "plantcheck", "set -e"), "set +e", False),
    (
        "N4c `set +e` block, `other=$?`",
        _before_legs("set +e", "plantcheck", "other=$?", "set -e"),
        "set +e",
        False,
    ),
    (
        "N5 PIPESTATUS",
        _before_legs('plantcheck | tee "$RUNNER_TEMP/p.log"; other=${PIPESTATUS[0]}'),
        "PIPESTATUS",
        False,
    ),
    (
        "N6a `$(...)` then `|| other=$?`",
        _before_legs("out=$(plantcheck) || other=$?"),
        "out=$(plantcheck)",
        False,
    ),
    (
        "N6b `$(...)` then `|| other=1`",
        _before_legs("out=$(plantcheck) || other=1"),
        "out=$(plantcheck)",
        False,
    ),
    ("N6c `$(...)` inside echo", _before_legs('echo "$(plantcheck)"'), "plantcheck", False),
    ("N7a `cmd || other=1`", _before_legs("plantcheck || other=1"), "plantcheck || other=1", False),
    (
        "N7b `cmd && ok=1 || other=1`",
        _before_legs("plantcheck && other=0 || other=1"),
        "plantcheck && other=0",
        False,
    ),
    (
        "N7c `cmd || { other=1; }`",
        _before_legs("plantcheck || { other=1; }"),
        "plantcheck || {",
        False,
    ),
    (
        "N8a continuation, `|| other=$?`",
        _before_legs("plantcheck \\", "  || other=$?"),
        "plantcheck",
        False,
    ),
    (
        "N8b continuation, `|| other=1`",
        _before_legs("plantcheck \\", "  || other=1"),
        "plantcheck",
        False,
    ),
    (
        "N9 `if cmd; then :; else other=1; fi`",
        _before_legs("if plantcheck; then :; else other=1; fi"),
        "if plantcheck",
        False,
    ),
    ("N10 `! cmd`", _before_legs("! plantcheck"), "! plantcheck", False),
    ("N11 `cmd &` (background)", _before_legs("plantcheck &"), "plantcheck &", False),
    (
        "N12 `cmd | tee` (a pipe loses it)",
        _before_legs('plantcheck | tee "$RUNNER_TEMP/p.log"'),
        "plantcheck | tee",
        False,
    ),
    (
        "F7a `cmd || other=1` folded",
        _before_legs("plantcheck || other=1", fold=True),
        "plantcheck || other=1",
        False,
    ),
    (
        "F2 `if ! cmd` folded",
        _before_legs("if ! plantcheck; then other=1; fi", fold=True),
        "if ! plantcheck",
        False,
    ),
    (
        "M1 `cmd || other=$?;` before the scripts judge, one line",
        lambda script: _edit_line(
            script, SCRIPTS_JUDGE, "plantcheck || other=$?; " + SCRIPTS_JUDGE
        ),
        "plantcheck || other=$?;",
        False,
    ),
    (
        "M2 `cmd &&` before the scripts judge, one line",
        lambda script: _edit_line(script, SCRIPTS_JUDGE, "plantcheck && " + SCRIPTS_JUDGE),
        "plantcheck && python3",
        False,
    ),
    (
        "V1 judge `; scripts=$?`",
        lambda script: _edit_line(script, "|| scripts=$?", "; scripts=$?"),
        "; scripts=$?",
        False,
    ),
    (
        "V2 judge `if ! ...; then scripts=1; fi`",
        lambda script: _edit_line(
            _edit_line(script, SCRIPTS_JUDGE, "if ! " + SCRIPTS_JUDGE),
            SCRIPTS_TAIL + " || scripts=$?",
            SCRIPTS_TAIL + "; then scripts=1; fi",
        ),
        "if ! python3",
        False,
    ),
    (
        "V3 judge `|| true || scripts=$?`",
        lambda script: _edit_line(script, "|| scripts=$?", "|| true || scripts=$?"),
        "|| true || scripts=$?",
        False,
    ),
    (
        "V4 judge `$(...)` in echo, captured",
        lambda script: _edit_line(
            _edit_line(script, SCRIPTS_JUDGE, 'echo "$(' + SCRIPTS_JUDGE),
            SCRIPTS_TAIL + " || scripts=$?",
            SCRIPTS_TAIL + ')" || scripts=$?',
        ),
        'echo "$(python3',
        False,
    ),
    (
        "V5 judge continuation, capture on the next line",
        lambda script: re.sub(r" \|\| scripts=\$\?", " \\\n  || scripts=$?", script, count=1),
        None,
        True,
    ),
    (
        "V6 judge PIPESTATUS",
        lambda script: _edit_line(
            script, "|| scripts=$?", '| tee "$RUNNER_TEMP/s.log"; scripts=${PIPESTATUS[0]}'
        ),
        "PIPESTATUS",
        False,
    ),
    (
        "V7 judge in a `set +e` block",
        lambda script: _edit_line(
            _edit_line(script, "|| scripts=$?", "\nscripts=$?\nset -e"),
            "scripts=0\n",
            "scripts=0\nset +e\n",
        ),
        "set +e",
        False,
    ),
]


class TheCensusJudgesEveryCommandByWhereItSits(unittest.TestCase):
    def test_every_command_form_is_driven_by_the_harness_or_refused_by_name(self):
        driver = Driver(self)
        script = step_script(driver.step)
        plantbin = Path(driver.root) / "bin"
        plantcheck = plantbin / "plantcheck"
        plantcheck.write_text("#!/bin/bash\nexit ${PLANT_RC:-0}\n", encoding="utf-8")
        plantcheck.chmod(0o700)
        silent, driven = [], 0
        for name, edit, needle, drives in examined("census plants", CENSUS_PLANTS):
            planted_script = edit(script)
            self.assertNotEqual(planted_script, script, name)
            try:
                commands = census(planted_script)
            except AssertionError as refused:
                if needle is None:
                    silent.append(f"{name}: refused a form the harness can drive: {refused}")
                elif needle not in str(refused):
                    silent.append(f"{name}: refused without quoting {needle!r}: {refused}")
                continue
            except Exception as other:  # noqa: BLE001 - not a refusal by name
                silent.append(f"{name}: failed without a name: {type(other).__name__}: {other}")
                continue
            if not drives:
                silent.append(f"{name}: passed the census unseen")
                continue
            driven += 1
            self.assertEqual(len(commands), planted_script.count(RUN), name)
            for key, into, _ in commands:
                self.assertIsNotNone(into, f"{name}: {key} records its exit nowhere")
                for rc in nonzero_exits():
                    done, calls = driver.run(planted_script, key, rc)
                    self.assertEqual(done.returncode, rc, f"{name}: {key} alone, exit {rc}")
                    self.assertEqual(len(calls), len(commands), name)
        print(
            f"census plants: {len(CENSUS_PLANTS) - driven - len(silent)} refused by name, {driven} driven, {len(silent)} silent"
        )
        self.assertEqual(silent, [], f"{len(silent)} plants escaped the census: {silent[:3]}")
        self.assertGreater(driven, 0)


if __name__ == "__main__":
    unittest.main()

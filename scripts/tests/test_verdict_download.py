"""The mutation verdict reads each report by name (SPEC-126 A1 to A4, issue #351).

`actions/download-artifact` at the pinned v8.0.1 lays a download out by how many artifacts matched
(`src/download-artifact.ts`): it extracts into `path` itself when the download is by `name`, when
`merge-multiple` is true, or when exactly one artifact matched, and into `path/<artifact name>` only
for two or more matches without `merge-multiple`. These tests apply that rule, in pure Python, to
the verdict's download steps and the shard job's output directory as `ci.yml` writes them, over
every combination of shard count, rows report and `mutation-web` upload, and assert the paths the
judge reads. Nothing here runs GitHub: the pull request's own verdict run is the live check.

The workflow is read as text, job by job, the way `test_mutation_workflows.py` reads it.
"""

import fnmatch
import posixpath
import re
import shlex
import unittest

from _support import examined
from test_mutation_workflows import CI, jobs, steps, workflow

TEMP = "T"
SHARD_COUNTS = (0, 1, 3)
DOWNLOAD = "actions/download-artifact@"
UPLOAD = "actions/upload-artifact@"
ROWS_PLANNED = "needs.mutation-plan.outputs.rows == 'true'"


REPORTS = "$reports"
PLAN = f"{REPORTS}/mutation-plan/plan.json"
ROWS = f"{REPORTS}/mutation-rows/rows.json"
# The flags each named class's judge line must carry, and the path each reads. A class the table
# does not name (a later class another change adds) is neither asserted nor an error.
JUDGE_FLAGS = {
    "rust": {
        "--plan": PLAN,
        "--shard-reports": REPORTS,
        "--rows": ROWS,
        "--whole": f"{REPORTS}/mutation-plan/whole.json",
    },
    "oracle": {"--plan": PLAN, "--rows": ROWS},
}
JUDGE = re.compile(
    r"(?m)^[ \t]*python3[ \t]+scripts/mutation-verdict\.py[ \t]+judge[ \t]+"
    r"(.*?)(?: \|\| \w+=\$\?)?$"
)
# Two private-use characters mark what `shlex` would erase: a `$` the shell reads as text (inside
# single quotes, or escaped) and a `$` outside every quote, where the shell word-splits the value.
LITERAL = "\ue000"
UNQUOTED = "\ue001"
BRACED = re.compile(r"\$\{(\w+)\}")
CONTINUATION = re.compile(r"(?<!\\)((?:\\\\)*)\\\n")

JUDGE_CMD = "python3 scripts/mutation-verdict.py judge"


def judge_pair(sub, reports='"$reports"', flags=("--plan", "--class", "--rows", "--whole")):
    """The verdict's `rust` and `oracle` judge lines as ci.yml writes them, with `sub(path)`
    spelling each path under the reports directory and `flags` spelling the four flag names."""
    plan, cls, rows, whole = flags
    rust = (
        f"{JUDGE_CMD} {plan} {sub('mutation-plan/plan.json')} {cls} rust "
        f"--shard-reports {reports} {rows} {sub('mutation-rows/rows.json')} "
        f"{whole} {sub('mutation-plan/whole.json')} || status=$?"
    )
    oracle = (
        f"{JUDGE_CMD} {plan} {sub('mutation-plan/plan.json')} {cls} oracle "
        f"{rows} {sub('mutation-rows/rows.json')} || oracle=$?"
    )
    return rust, oracle


def double_quoted(path):
    return f'"$reports/{path}"'


def as_verdict(pair):
    """A verdict job's text around a pair of judge lines, indented as in a `run: |` block."""
    return f'        reports="$RUNNER_TEMP/reports"\n        {pair[0]}\n        {pair[1]}\n'


CANONICAL = judge_pair(double_quoted)
# The spellings issue #374 names: each is an argument list the shell builds identically to the
# canonical one, so the test must accept every one of them.
EQUIVALENT = {
    "braces": judge_pair(lambda p: f'"${{reports}}/{p}"', reports='"${reports}"'),
    "the expansion closed before the slash": judge_pair(lambda p: f'"$reports"/{p}'),
    "the tail in single quotes": judge_pair(lambda p: f"\"$reports\"'/{p}'"),
    "a quoted flag name": judge_pair(
        double_quoted, flags=('"--plan"', "'--class'", '"--rows"', "--whole")
    ),
    "a backslash continuation": tuple(
        line.replace(" --rows", " \\\n          --rows") for line in CANONICAL
    ),
    "--flag=value with the value quoted": tuple(
        line.replace("--plan ", "--plan=")
        .replace("--rows ", "--rows=")
        .replace("--whole ", "--whole=")
        for line in CANONICAL
    ),
    "--flag=value with the quotes closing early": tuple(
        line.replace('--plan "$reports/', '--plan="$reports"/').replace(
            'mutation-plan/plan.json"', "mutation-plan/plan.json"
        )
        for line in CANONICAL
    ),
}
# The paths the shell reads differently: each must stay refused.
WRONG = {
    "a single-quoted path is literal text": judge_pair(lambda p: f"'$reports/{p}'"),
    "a single-quoted head with an unquoted tail": judge_pair(lambda p: f"'$reports'/{p}"),
    "an escaped dollar is literal text": judge_pair(lambda p: f'"\\$reports/{p}"'),
    "a path in a different directory": judge_pair(
        lambda p: double_quoted(p.replace("mutation-plan", "mutation-plans"))
    ),
    "a different variable": judge_pair(lambda p: f'"$report/{p}"'),
    "an unquoted expansion": judge_pair(lambda p: f"$reports/{p}"),
    "a flag moved to the other judge line": (
        CANONICAL[0].replace(' --whole "$reports/mutation-plan/whole.json"', ""),
        CANONICAL[1].replace(" ||", ' --whole "$reports/mutation-plan/whole.json" ||'),
    ),
    "a flag dropped from one line": (
        CANONICAL[0].replace(' --rows "$reports/mutation-rows/rows.json"', ""),
        CANONICAL[1],
    ),
    "a flag only in a comment": (
        CANONICAL[0].replace(' --rows "$reports/mutation-rows/rows.json"', "")
        + ' # --rows "$reports/mutation-rows/rows.json"',
        CANONICAL[1],
    ),
    "a flag after a control operator": (
        CANONICAL[0]
        .replace(' --rows "$reports/mutation-rows/rows.json"', "")
        .replace(" ||", ' && : --rows "$reports/mutation-rows/rows.json" ||'),
        CANONICAL[1],
    ),
    "a line break after judge with no backslash": (
        CANONICAL[0].replace("judge --plan", "judge\n        --plan"),
        CANONICAL[1],
    ),
}


def mark_dollars(command):
    """The command with each `$` the shell reads as text replaced by LITERAL (in single quotes,
    or escaped by a backslash) and each `$` outside every quote preceded by UNQUOTED. `shlex`
    removes the quotes and the backslash, so this pass keeps the fact it would lose. The command
    ends where the shell ends it: at a control operator or a comment outside every quote."""
    out, quote, i, start = [], None, 0, True
    while i < len(command):
        char = command[i]
        if char == "\\" and quote != "'" and i + 1 < len(command):
            out.append(LITERAL if command[i + 1] == "$" else char + command[i + 1])
            i, start = i + 2, False
            continue
        if quote is None and (char in ";&|" or (char == "#" and start)):
            break
        start = quote is None and char in " \t"
        if quote is None and char in "'\"":
            quote = char
        elif quote == char:
            quote = None
        if char == "$":
            out.append(LITERAL if quote == "'" else (UNQUOTED if quote is None else "") + char)
        else:
            out.append(char)
        i += 1
    return "".join(out)


def shell_words(command):
    """The words the shell splits `command` into (quote removal, backslash-newline continuations
    joined). An expansion reads `$name` whether it was written `$name` or `${name}`; one outside
    every quote reads `(unquoted)$name`; a `$` that is text reads `\\$`."""
    joined = CONTINUATION.sub(r"\1", command)
    words = shlex.split(mark_dollars(joined), comments=False, posix=True)
    return [
        BRACED.sub(r"$\1", w).replace(UNQUOTED, "(unquoted)").replace(LITERAL, "\\$") for w in words
    ]


def judge_flags(words):
    """{flag: value} for the `--flag value` and `--flag=value` pairs among `words`."""
    flags = {}
    for i, word in enumerate(words):
        if not word.startswith("--"):
            continue
        name, equals, value = word.partition("=")
        if equals:
            flags[name] = value
        elif i + 1 < len(words):
            flags[name] = words[i + 1]
    return flags


def judge_lines(verdict):
    """{class: {flag: value}} for each `mutation-verdict.py judge` command of the verdict job,
    read as the shell splits it; a command with no `--class` is refused, and zero commands too."""
    found = {}
    joined = CONTINUATION.sub(r"\1", verdict)
    for command in examined("judge command lines", JUDGE.findall(joined)):
        flags = judge_flags(shell_words(command))
        if "--class" not in flags:
            raise AssertionError(f"a judge line names no --class: {command}")
        if flags["--class"] in found:
            raise AssertionError(f"two judge lines of class {flags['--class']}: {command}")
        found[flags["--class"]] = flags
    return found


def check_judge(verdict):
    """Assert each named class's judge line reads its flags at the paths JUDGE_FLAGS names."""
    lines = judge_lines(verdict)
    for cls, wanted in examined("judge classes asserted", list(JUDGE_FLAGS.items())):
        if cls not in lines:
            raise AssertionError(f"the verdict has no judge line for class {cls}")
        for flag, path in wanted.items():
            if lines[cls].get(flag) != path:
                raise AssertionError(
                    f"{lines[cls].get(flag)!r} != {path!r} : "
                    f"the {cls} judge line does not read {flag} at {path}"
                )


class StepFailed(Exception):
    """A download step that the action would fail."""


def inputs(step):
    """The `with:` inputs of a step, as {key: value}."""
    found = {}
    block = re.search(r"(?ms)^        with:\n(.*?)(?=^      - |^        \w|\Z)", step)
    for line in (block.group(1) if block else "").splitlines():
        match = re.match(r"^          ([a-z-]+): (.*)$", line)
        if match:
            found[match.group(1)] = match.group(2).strip()
    return found


def local(path):
    """A workflow path under the runner's temp directory, as a path relative to it."""
    return posixpath.normpath(path.replace("${{ runner.temp }}", TEMP)).strip("/")


def shard_artifact(shard):
    """(name, {file inside the artifact}) for one shard, from the shard job's own text."""
    rust = jobs(workflow(CI))["mutation-rust"]
    made = re.search(r'out="\$RUNNER_TEMP/([^"]+)"', rust)
    upload = next(s for s in steps(rust) if UPLOAD in s and "mutation-rust-shard-" in s)
    given = inputs(upload)
    if made is None or "path" not in given:
        raise AssertionError("mutation-rust does not name its output directory and its upload")
    out = local(f"{TEMP}/" + made.group(1).replace("$SHARD", str(shard)))
    root = local(given["path"])
    if out != root and not out.startswith(root + "/"):
        raise AssertionError(f"the shard writes to {out}, outside what it uploads ({root})")
    name = given["name"].replace("${{ matrix.shard }}", str(shard))
    inside = posixpath.relpath(out, root)
    prefix = "" if inside == "." else inside + "/"
    files = {
        f"{prefix}mutants.out/outcomes.json",
        f"{prefix}cargo-mutants.exit",
        f"{prefix}memory-scope.json",
    }
    return name, files


def artifacts(shards, rows, web):
    """{artifact name: {file inside it}} the run uploaded, as the jobs' own text produces them."""
    found = {"mutation-plan": {"plan.json", "whole.json", "git.diff"}}
    for shard in range(shards):
        name, files = shard_artifact(shard)
        found[name] = files
    if rows:
        found["mutation-rows"] = {"rows.json"}
    if web:
        found["mutation-web"] = {"plan.json", "stryker/mutation.json"}
    return found


def runs(step, rows):
    """Whether the step's `if:` holds; a condition this test does not model is refused."""
    match = re.search(r"(?m)^        if: (.*)$", step)
    if match is None:
        return True
    condition = match.group(1).replace("${{", "").replace("}}", "").strip()
    if condition == ROWS_PLANNED:
        return rows
    raise AssertionError(f"a download condition this test does not model: {condition}")


def layout(uploaded, rows):
    """The files under the temp directory after the verdict's downloads, as the pinned action
    lays them out. A download by name of an artifact that does not exist fails the step."""
    verdict = jobs(workflow(CI))["mutation-verdict"]
    placed = set()
    for step in steps(verdict):
        if DOWNLOAD not in step or not runs(step, rows):
            continue
        given = inputs(step)
        destination = local(given.get("path", ""))
        if "name" in given:
            if given["name"] not in uploaded:
                raise StepFailed(f"no artifact named {given['name']}")
            matched = [given["name"]]
        else:
            matched = sorted(n for n in uploaded if fnmatch.fnmatchcase(n, given["pattern"]))
        flat = "name" in given or given.get("merge-multiple") == "true" or len(matched) == 1
        for name in matched:
            base = destination if flat else posixpath.join(destination, name)
            placed |= {posixpath.join(base, f) for f in uploaded[name]}
    return placed


def scenarios():
    return [
        (shards, rows, web)
        for shards in SHARD_COUNTS
        for rows in (False, True)
        for web in (False, True)
    ]


class TheVerdictReadsEachReportByName(unittest.TestCase):
    def test_the_layout_holds_for_every_count_of_artifacts(self):
        reports = f"{TEMP}/reports"
        for shards, rows, web in examined("artifact scenarios", scenarios()):
            where = f"{shards} shard(s), rows {rows}, web {web}"
            placed = layout(artifacts(shards, rows, web), rows)
            self.assertIn(f"{reports}/mutation-plan/plan.json", placed, where)
            self.assertIn(f"{reports}/mutation-plan/whole.json", placed, where)
            if rows:
                self.assertIn(f"{reports}/mutation-rows/rows.json", placed, where)
            for shard in range(shards):
                self.assertIn(
                    f"{reports}/mutation-rust-shard-{shard}/mutants.out/outcomes.json",
                    placed,
                    where,
                )
                self.assertIn(
                    f"{reports}/mutation-rust-shard-{shard}/cargo-mutants.exit", placed, where
                )

    def test_the_verdict_reads_no_artifact_of_a_job_it_does_not_need(self):
        verdict = jobs(workflow(CI))["mutation-verdict"]
        needs = re.search(r"(?m)^    needs: \[(.*)\]$", verdict)
        self.assertIsNotNone(needs, "the verdict names no needs")
        needed = [name.strip() for name in needs.group(1).split(",")]
        self.assertNotIn("mutation-web", needed)
        downloads = [s for s in steps(verdict) if DOWNLOAD in s]
        for step in examined("verdict download steps", downloads):
            given = inputs(step)
            wanted = given.get("name") or given.get("pattern") or "*"
            self.assertFalse(
                fnmatch.fnmatchcase("mutation-web", wanted),
                f"a verdict download reaches mutation-web: {wanted}",
            )
        for shards, rows, _ in examined("artifact scenarios", scenarios()):
            with_web = layout(artifacts(shards, rows, True), rows)
            without = layout(artifacts(shards, rows, False), rows)
            self.assertEqual(with_web, without, f"{shards} shard(s), rows {rows}")

    def test_a_producer_that_uploaded_nothing_leaves_the_reading_unchanged(self):
        verdict = jobs(workflow(CI))["mutation-verdict"]
        self.assertNotIn("continue-on-error", verdict)
        rows_steps = [
            s for s in steps(verdict) if DOWNLOAD in s and inputs(s).get("name") == "mutation-rows"
        ]
        self.assertEqual(len(rows_steps), 1, "the rows' report is not downloaded by name")
        self.assertIn(ROWS_PLANNED, rows_steps[0], "the rows' download does not track the rows job")
        for shards, _, web in examined("artifact scenarios", scenarios()):
            uploaded = artifacts(shards, False, web)
            placed = layout(uploaded, False)
            stray = [p for p in placed if "/mutation-rows/" in p]
            self.assertEqual(stray, [], f"{shards} shard(s), web {web}")
            with_rows = layout(artifacts(shards, True, web), True)
            self.assertIn(f"{TEMP}/reports/mutation-rows/rows.json", with_rows)

    def test_the_judge_reads_the_paths_the_downloads_lay_down(self):
        verdict = jobs(workflow(CI))["mutation-verdict"]
        self.assertIn('reports="$RUNNER_TEMP/reports"', verdict)
        check_judge(verdict)
        for step in [s for s in steps(verdict) if DOWNLOAD in s]:
            destination = local(inputs(step).get("path", ""))
            self.assertTrue(
                destination == f"{TEMP}/reports" or destination.startswith(f"{TEMP}/reports/"),
                f"a download lands outside the judge's reports directory: {destination}",
            )

    def test_every_spelling_the_shell_reads_alike_passes(self):
        for name, pair in examined("equivalent spellings", EQUIVALENT.items()):
            with self.subTest(spelling=name):
                check_judge(as_verdict(pair))

    def test_every_path_the_shell_reads_differently_is_refused(self):
        check_judge(as_verdict(CANONICAL))
        for name, pair in examined("wrong paths", WRONG.items()):
            with self.subTest(wrong=name):
                with self.assertRaises(AssertionError):
                    check_judge(as_verdict(pair))

    def test_an_expansion_is_kept_apart_from_text_and_from_an_unquoted_one(self):
        spelled = {
            '"$reports"': "$reports",
            '"${reports}"': "$reports",
            "\"$reports\"'/x'": "$reports/x",
            "'$reports'": "\\$reports",
            "\\$reports": "\\$reports",
            "$reports": "(unquoted)$reports",
        }
        for written, read in examined("spellings of an expansion", spelled.items()):
            self.assertEqual(shell_words(written), [read], written)


if __name__ == "__main__":
    unittest.main()

"""The agent documents name the jobs, stages and scopes the tree holds (SPEC-685).

docs/BUILDER-BRIEF.md, CLAUDE.md and docs/TESTING.md tell an agent which CI jobs judge a pull
request, which stages the gate runs and which scopes the weekly mutation battery sweeps. Each test
reads one claim of a document and the file the claim describes, both as text, and asserts that
they agree: the jobs of .github/workflows/ci.yml and the needs of its `ci` job, the `STAGES_ALL`
list of scripts/check.sh, and the values .github/workflows/mutation-weekly.yml compares its
`package` input with. Nothing is run, and of the tree only `_support` is imported.
"""

import re
import unittest

from _support import REPO, examined

CI = REPO / ".github" / "workflows" / "ci.yml"
WEEKLY = REPO / ".github" / "workflows" / "mutation-weekly.yml"
CHECK = REPO / "scripts" / "check.sh"
BRIEF = REPO / "docs" / "BUILDER-BRIEF.md"
RULES_DOC = REPO / "CLAUDE.md"
TESTING = REPO / "docs" / "TESTING.md"
# The words the documents spell a count of jobs with.
NUMBERS = {
    "two": 2,
    "three": 3,
    "four": 4,
    "five": 5,
    "six": 6,
    "seven": 7,
    "eight": 8,
    "nine": 9,
    "ten": 10,
}
# The separator CLAUDE.md's gate comment writes between two stages.
DOT = "·"
JOB = re.compile(r"  ([A-Za-z0-9_-]+):")
NEEDS = re.compile(r"    needs: \[([^\]]*)\]")
GATE_RUN = re.compile(r"\s+run: bash scripts/check\.sh((?: [a-z-]+)*)")
PYTHON_SHARD = re.compile(
    r"\s+shard: \$\{\{ fromJSON\(needs\.mutation-plan\.outputs\.python\w*_matrix\) \}\}"
)
PACKAGE_VALUE = re.compile(r"inputs\.package [!=]= '([^']+)'")
STAGES = re.compile(r"STAGES_ALL=\(([^)]*)\)")
TICKED = re.compile(r"`([^`]+)`")
JOBS_BULLET = r"- \*\*\w+ CI jobs judge every pull request,\*\*"


def text(path):
    """One file of the tree, read as text."""
    return path.read_text(encoding="utf-8")


def prose(path):
    """One document as one line: each run of whitespace read as one space."""
    return " ".join(text(path).split())


def jobs():
    """{job id: the lines of its block} for each job of ci.yml, in order, comment lines left out."""
    found, current, inside = {}, None, False
    for line in text(CI).split("\n"):
        if line.strip().startswith("#"):
            continue
        if not inside:
            inside = line.rstrip() == "jobs:"
            continue
        if line.strip() and not line.startswith(" "):
            break
        match = JOB.fullmatch(line.rstrip())
        if match:
            current = match.group(1)
            found[current] = []
        elif current is not None:
            found[current].append(line.rstrip())
    return found


def needs(job):
    """The jobs one job of ci.yml needs, as its one-line `needs` list names them."""
    for line in jobs()[job]:
        match = NEEDS.fullmatch(line)
        if match:
            return [name.strip() for name in match.group(1).split(",") if name.strip()]
    return []


def holding(fragment):
    """The jobs of ci.yml with a line that holds `fragment`, in order."""
    return [name for name, block in jobs().items() if any(fragment in line for line in block)]


def matching(form):
    """The jobs of ci.yml with a line that `form` matches whole, in order."""
    return [name for name, block in jobs().items() if any(form.fullmatch(line) for line in block)]


def gate_runs():
    """{job id: the stages it hands scripts/check.sh} for each job of ci.yml that runs the gate."""
    runs = {}
    for name, block in jobs().items():
        for line in block:
            match = GATE_RUN.fullmatch(line)
            if match:
                runs[name] = match.group(1).split()
    return runs


def stages():
    """The gate's stages, in order, as `STAGES_ALL` in scripts/check.sh names them."""
    for line in text(CHECK).split("\n"):
        match = STAGES.fullmatch(line.rstrip())
        if match:
            return match.group(1).split()
    return []


def item(path, opening):
    """The list item of a document whose first line `opening` matches, as one line, or ''."""
    held = text(path).split("\n")
    for index, line in enumerate(held):
        if re.match(opening, line):
            end = index + 1
            while end < len(held) and held[end].startswith("  "):
                end += 1
            return " ".join(" ".join(held[index:end]).split())
    return ""


def ticked(fragment):
    """The names a fragment of a document writes in backticks, in order."""
    return TICKED.findall(fragment)


class AgentDocumentsTest(unittest.TestCase):
    def test_the_builder_brief_counts_and_names_every_mutation_job_ci_needs(self):
        expected = examined(
            "mutation job(s) ci needs",
            sorted(name for name in needs("ci") if name.startswith("mutation-")),
        )
        bullet = item(BRIEF, JOBS_BULLET)
        self.assertTrue(bullet, "docs/BUILDER-BRIEF.md holds no bullet of the CI jobs")
        word = bullet.split()[1].strip("*").lower()
        self.assertEqual(NUMBERS.get(word), len(expected), f"the bullet counts {word} CI jobs")
        named = sorted({name for name in ticked(bullet) if name.startswith("mutation-")})
        self.assertEqual(named, expected, "the mutation jobs the bullet names")

    def test_the_builder_brief_names_the_job_that_runs_strykerjs(self):
        runners = examined("job(s) that run StrykerJS", holding("stryker run"))
        bullet = item(BRIEF, JOBS_BULLET)
        named = re.findall(
            r"`([a-z0-9-]+)` runs StrykerJS over every changed Mini App file", bullet
        )
        self.assertEqual(named, runners, "the job the bullet says runs StrykerJS")
        judges = re.findall(r"`([a-z0-9-]+)` unites the shards' reports and judges", bullet)
        self.assertEqual(len(judges), 1, "the one job the bullet says unites the shards' reports")
        self.assertTrue(set(runners) <= set(needs(judges[0])), f"{judges[0]} needs {runners}")

    def test_the_gate_comment_names_every_stage_check_sh_runs(self):
        expected = examined("gate stage(s)", stages())
        held = text(RULES_DOC).split("\n")
        fence = held.index("```sh", held.index("## The gate"))
        end = held.index("```", fence + 1)
        comment = " ".join(line.split("#", 1)[1] for line in held[fence + 1 : end] if "#" in line)
        named = [part.split()[0] for part in comment.split(DOT) if part.split()]
        self.assertEqual(named, expected, "the stages the gate comment names")

    def test_the_ci_table_names_the_jobs_and_stages_ci_and_check_sh_hold(self):
        held = text(TESTING).split("\n")
        start = held.index("| job | stages |") + 2
        table = {}
        for line in held[start:]:
            if not line.startswith("|"):
                break
            job, cell = [part.strip() for part in line.strip().strip("|").split("|")]
            table[ticked(job)[0]] = [] if cell.startswith("none") else ticked(cell)[0].split()
        examined("row(s) of the CI table", table)
        self.assertEqual(sorted(set(table) - set(jobs())), [], "jobs the table names, ci.yml not")
        runs = gate_runs()
        examined("job(s) that run scripts/check.sh", runs)
        given = {job: named for job, named in table.items() if named}
        self.assertEqual(given, runs, "the stages each job runs")
        self.assertEqual(
            sorted(stage for named in table.values() for stage in named),
            sorted(stages()),
            "every stage in exactly one job",
        )
        counts = re.findall(r"in (\w+) jobs that start together", prose(TESTING))
        self.assertEqual([NUMBERS.get(word) for word in counts], [len(runs)], "the jobs it counts")

    def test_the_coverage_section_names_every_python_shard_job(self):
        shards = examined("job(s) dealt the plan's Python shards", matching(PYTHON_SHARD))
        found = re.findall(
            r"are mutated by `scripts/mutation_python\.py`, (.*?) \(ADR-073\)", prose(TESTING)
        )
        self.assertEqual(len(found), 1, "the one sentence of the Python mutation jobs")
        named = sorted(name for name in ticked(found[0]) if name.startswith("mutation-"))
        self.assertEqual(named, sorted(shards), "the Python mutation jobs the sentence names")

    def test_the_builder_brief_names_every_scope_the_weekly_dispatch_takes(self):
        values = []
        for value in PACKAGE_VALUE.findall(text(WEEKLY)):
            if value not in values:
                values.append(value)
        examined("value(s) the weekly battery compares its package input with", values)
        found = re.findall(r"with `package` set sweeps (.*?), and ends with", prose(BRIEF))
        self.assertEqual(len(found), 1, "the one sentence of the weekly dispatch")
        self.assertEqual(sorted(ticked(found[0])), sorted(values), "the scopes the sentence names")


if __name__ == "__main__":
    unittest.main()

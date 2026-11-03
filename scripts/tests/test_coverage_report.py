"""The advisory coverage report (SPEC-592, ADR-613): A1 to A10.

The behaviour tests drive `scripts/coverage-report.py` as a child process over planted files in a
temporary directory: lcov files, a unified diff, a changed-path list and a lockfile. The census
tests read the named files of the tree as text, each with planted texts its reader must refuse.
Two of them list the whole tree (`git ls-files`) and run in CI only. Nothing here runs GitHub.
"""

import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "coverage-report.py"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "coverage"
CONFIG = REPO / "config" / "coverage.json"
WORKFLOW = REPO / ".github" / "workflows" / "coverage.yml"
LANES = ("rust", "web", "python")
KINDS = ("deprecated", "timing", "test", "declaration", "generated")
FLOAT = re.compile(r"(?<![\w/.-])[0-9]+[.][0-9]+(?![\w/.-])")
HINTS = re.compile(
    r"LCOV_EXCL|pragma: no cover|istanbul ignore|v8 ignore|c8 ignore|coverage\(off\)|llvm-cov:ignore"
)


def run(*args):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def said(done):
    return done.stdout + done.stderr


def write(folder, name, text):
    path = Path(folder) / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return str(path)


def report(folder, lcov_text, lane="python", diff_text=None, event="pull_request"):
    """Run `report` over an lcov text, and a diff text when one is given."""
    args = [
        "report",
        "--lane",
        lane,
        "--lcov",
        write(folder, "in.lcov", lcov_text),
        "--root",
        str(folder),
    ]
    if diff_text is not None:
        args += ["--diff", write(folder, "in.diff", diff_text), "--event", event]
    return run(*args)


def lcov(path, lines):
    body = "".join(f"DA:{n},{hits}\n" for n, hits in lines)
    return f"TN:\nSF:{path}\n{body}end_of_record\n"


def plan(folder, event, changed, base="dev", head="feat/x", head_repo="o/r", repo="o/r"):
    out = Path(folder) / "github-output.txt"
    done = run(
        "plan",
        "--event",
        event,
        "--base-ref",
        base,
        "--head-ref",
        head,
        "--head-repo",
        head_repo,
        "--repo",
        repo,
        "--changed",
        write(folder, "changed.txt", "".join(p + "\n" for p in changed)),
        "--github-output",
        str(out),
    )
    values = {}
    if out.exists():
        for line in out.read_text(encoding="utf-8").splitlines():
            key, _, value = line.partition("=")
            values[key] = value
    return done, values


class TheReportReadsVoidOrMeasures(unittest.TestCase):
    def test_a_lane_that_examined_no_line_reads_void(self):
        with tempfile.TemporaryDirectory() as tmp:
            done = report(tmp, lcov("docs/outside.txt", [(1, 1)]))
        self.assertEqual(done.returncode, 1, said(done))
        self.assertRegex(said(done), r"VOID lane python")

    def test_one_examined_line_is_a_measured_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            done = report(tmp, lcov("scripts/one.py", [(1, 1)]))
        self.assertEqual(done.returncode, 0, said(done))
        self.assertIn("coverage-report: lane python examined 1 lines in 1 files", done.stdout)

    def test_a_planted_uncovered_changed_line_is_printed_by_file_and_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            done = report(
                tmp,
                (FIXTURES / "uncovered.lcov").read_text(encoding="utf-8"),
                diff_text=(FIXTURES / "uncovered.diff").read_text(encoding="utf-8"),
            )
        self.assertEqual(done.returncode, 0, said(done))
        self.assertIn("not executed: scripts/planted_control.py:3", done.stdout)
        self.assertIn("changed executable lines 3, executed 2", done.stdout)

    def test_every_figure_is_an_integer_count(self):
        with tempfile.TemporaryDirectory() as tmp:
            summary = Path(tmp) / "summary.md"
            out_dir = Path(tmp) / "out"
            done = run(
                "report",
                "--lane",
                "python",
                "--lcov",
                str(FIXTURES / "uncovered.lcov"),
                "--root",
                tmp,
                "--diff",
                str(FIXTURES / "uncovered.diff"),
                "--event",
                "pull_request",
                "--summary",
                str(summary),
                "--out-dir",
                str(out_dir),
            )
            self.assertEqual(done.returncode, 0, said(done))
            self.assertNotIn("%", done.stdout)
            self.assertIsNone(FLOAT.search(done.stdout), done.stdout)
            texts = {
                "report.txt": (out_dir / "report.txt").read_text(encoding="utf-8"),
                "summary": summary.read_text(encoding="utf-8"),
            }
            raw = (out_dir / "report.json").read_text(encoding="utf-8")
            texts["report.json"] = raw

            def refuse_float(text):
                raise AssertionError("a float in report.json: " + text)

            figures = json.loads(raw, parse_float=refuse_float)
        for name, text in examined("outputs", texts.items()):
            self.assertNotIn("%", text, name)
            self.assertIsNone(FLOAT.search(text), name)
        self.assertEqual(figures["changed_executable"], 3)
        self.assertEqual(figures["changed_executed"], 2)

    def test_an_unreadable_lcov_reads_void(self):
        cases = {
            "an empty file": "",
            "a DA record that is not an integer": "SF:scripts/a.py\nDA:x,1\nend_of_record\n",
            "a DA record outside a record": "DA:1,1\nSF:scripts/a.py\nend_of_record\n",
            "a record left open": "SF:scripts/a.py\nDA:1,1\n",
        }
        with tempfile.TemporaryDirectory() as tmp:
            for name, text in examined("unreadable lcov cases", cases.items()):
                with self.subTest(name):
                    done = report(tmp, text)
                    self.assertEqual(done.returncode, 1, name + ": " + said(done))
                    self.assertIn("VOID", said(done), name)
            missing = run(
                "report",
                "--lane",
                "python",
                "--lcov",
                str(Path(tmp) / "absent.lcov"),
                "--root",
                tmp,
            )
            self.assertEqual(missing.returncode, 1, said(missing))
            self.assertIn("VOID", said(missing))


class TheProviderIsLockedAtTheVitestVersion(unittest.TestCase):
    def lock(self, folder, vitest, provider):
        text = "packages:\n\n"
        for version in provider:
            text += f"  '@vitest/coverage-v8@{version}':\n    resolution: {{}}\n\n"
        for version in vitest:
            text += f"  vitest@{version}:\n    resolution: {{}}\n\n"
        return write(folder, "lock.yaml", text)

    def test_a_provider_locked_at_another_version_reads_void(self):
        cases = {
            "a mixed lock": (["5.0.5"], ["5.0.4"]),
            "a lock with no provider": (["5.0.5"], []),
            "a lock with no vitest": ([], ["5.0.5"]),
            "a lock with two provider versions": (["5.0.5"], ["5.0.5", "5.0.4"]),
        }
        with tempfile.TemporaryDirectory() as tmp:
            for name, (vitest, provider) in examined("lock cases", cases.items()):
                with self.subTest(name):
                    done = run("provider", "--lockfile", self.lock(tmp, vitest, provider))
                    self.assertEqual(done.returncode, 1, name + ": " + said(done))
                    self.assertIn("VOID", said(done), name)
            ok = run("provider", "--lockfile", self.lock(tmp, ["5.0.5"], ["5.0.5"]))
            self.assertEqual(ok.returncode, 0, said(ok))
            self.assertIn(
                "coverage-provider: @vitest/coverage-v8 5.0.5 equals vitest 5.0.5", ok.stdout
            )

    def test_the_committed_lockfile_locks_the_provider_at_the_vitest_version(self):
        text = (REPO / "pnpm-lock.yaml").read_text(encoding="utf-8")
        vitest = sorted(set(re.findall(r"(?m)^  vitest@([0-9][0-9A-Za-z.+-]*):$", text)))
        provider = sorted(
            set(re.findall(r"(?m)^  '@vitest/coverage-v8@([0-9][0-9A-Za-z.+-]*)':$", text))
        )
        self.assertEqual(len(vitest), 1, vitest)
        self.assertEqual(provider, vitest)
        done = run("provider", "--lockfile", str(REPO / "pnpm-lock.yaml"))
        self.assertEqual(done.returncode, 0, said(done))
        self.assertIn(
            f"coverage-provider: @vitest/coverage-v8 {vitest[0]} equals vitest {vitest[0]}",
            done.stdout,
        )


class ThePlanNamesTheLanes(unittest.TestCase):
    def test_the_plan_runs_only_the_lanes_a_diff_touches(self):
        cases = {
            "a Rust source": (["crates/kernel/src/lib.rs"], (True, False, False)),
            "a web source": (["web/app/src/lib/a.ts"], (False, True, False)),
            "a Python source": (["scripts/tool.py"], (False, False, True)),
            "a document": (["docs/readme.md"], (False, False, False)),
            "a shared coverage file": ([".github/workflows/coverage.yml"], (True, True, True)),
            "a lane configuration": (["Cargo.lock"], (True, False, False)),
        }
        with tempfile.TemporaryDirectory() as tmp:
            for name, (changed, want) in examined("plan cases", cases.items()):
                with self.subTest(name):
                    done, values = plan(tmp, "pull_request", changed)
                    self.assertEqual(done.returncode, 0, said(done))
                    got = tuple(values.get(lane) == "true" for lane in LANES)
                    self.assertEqual(got, want, name + ": " + done.stdout)
                    self.assertEqual(values.get("scope"), "diff", name)
                    self.assertEqual(values.get("retention"), "30", name)
                    self.assertIn(f"examined {len(changed)} changed paths", done.stdout, name)

    def test_a_dev_push_and_the_release_pull_request_run_every_lane(self):
        with tempfile.TemporaryDirectory() as tmp:
            done, push = plan(tmp, "push", [])
            self.assertEqual(done.returncode, 0, said(done))
            self.assertEqual(push.get("scope"), "whole", done.stdout)
            self.assertEqual([push.get(lane) for lane in LANES], ["true"] * 3, done.stdout)
            self.assertEqual(push.get("retention"), "90")
            done, release = plan(tmp, "pull_request", ["docs/readme.md"], base="main", head="dev")
            self.assertEqual(release.get("scope"), "whole", done.stdout)
            self.assertEqual([release.get(lane) for lane in LANES], ["true"] * 3, done.stdout)
            self.assertEqual(release.get("retention"), "30")
            done, fork = plan(
                tmp,
                "pull_request",
                ["docs/readme.md"],
                base="main",
                head="dev",
                head_repo="someone/fork",
            )
            self.assertEqual(fork.get("scope"), "diff", done.stdout)
            self.assertEqual([fork.get(lane) for lane in LANES], ["false"] * 3, done.stdout)


def glob_regex(glob):
    """The grammar of config/coverage.json: `**`, `*`, `{a,b}` and the literals, read here apart
    from the script that applies it."""
    out = ""
    i = 0
    while i < len(glob):
        if glob.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
        elif glob.startswith("**", i):
            out += ".*"
            i += 2
        elif glob[i] == "*":
            out += "[^/]*"
            i += 1
        elif glob[i] == "{":
            end = glob.index("}", i)
            out += "(?:" + "|".join(re.escape(a) for a in glob[i + 1 : end].split(",")) + ")"
            i = end + 1
        else:
            out += re.escape(glob[i])
            i += 1
    return re.compile("^" + out + "$")


def row_problems(rows):
    """Every way an exclusion row can be less than one glob with its lane, kind and reason."""
    problems = []
    seen = set()
    for row in rows:
        name = str(row.get("glob"))
        for key in ("glob", "lane", "kind", "reason"):
            if not row.get(key):
                problems.append(name + " lacks " + key)
        if row.get("lane") not in LANES:
            problems.append(name + " names no lane")
        if row.get("kind") not in KINDS:
            problems.append(name + " has an unknown kind")
        if len(str(row.get("reason", ""))) < 20:
            problems.append(name + " has no reason")
        if row.get("kind") == "deprecated" and row.get("cite") != "ADR-589":
            problems.append(name + " is deprecated and does not cite ADR-589")
        if row.get("kind") == "generated" and not row.get("generated_by"):
            problems.append(name + " is generated and names no generator")
        key = (row.get("glob"), row.get("lane"))
        if key in seen:
            problems.append(name + " is listed twice")
        seen.add(key)
    return problems


class TheExclusionList(unittest.TestCase):
    def rows(self):
        return json.loads(CONFIG.read_text(encoding="utf-8"))["exclusions"]

    def test_every_exclusion_is_one_row_with_its_reason(self):
        rows = examined("exclusion rows", self.rows())
        self.assertEqual(row_problems(rows), [])
        good = {
            "glob": "a/**",
            "lane": "rust",
            "kind": "test",
            "reason": "Test code, which is not measured.",
        }
        planted = {
            "a row with no reason": dict(good, reason=""),
            "a row with no lane": {k: v for k, v in good.items() if k != "lane"},
            "a deprecated row that cites nothing": dict(good, kind="deprecated"),
            "a lane that is not one": dict(good, lane="swift"),
        }
        for name, row in examined("planted rows", planted.items()):
            self.assertNotEqual(row_problems([row]), [], name)
        self.assertNotEqual(row_problems([good, good]), [], "a row listed twice")
        self.assertEqual(row_problems([good]), [])
        deprecated = [r for r in rows if r["kind"] == "deprecated"]
        self.assertEqual(len(deprecated), 4, "the deprecated Telegram rows")

    def test_the_rust_run_flags_come_from_the_rows(self):
        done = run("args", "--lane", "rust")
        self.assertEqual(done.returncode, 0, said(done))
        packages = [r["package"] for r in self.rows() if r["lane"] == "rust" and "package" in r]
        binaries = [r["binary"] for r in self.rows() if r["lane"] == "rust" and "binary" in r]
        want = []
        for package in packages:
            want += ["--exclude", package]
        want += [
            "-E",
            "not (" + " | ".join("binary_id(=" + b + ")" for b in binaries) + ")",
        ]
        self.assertEqual(done.stdout.splitlines(), want)
        self.assertEqual(packages, ["deck-streak-bot"])
        self.assertEqual(binaries, ["deck-streak-ingest::engine_budget"])
        self.assertEqual(run("args", "--lane", "web").stdout, "")

    def test_no_inline_hint_without_a_row(self):
        listing = subprocess.run(
            ["git", "-C", str(REPO), "ls-files", "-z"], capture_output=True, text=True, check=False
        )
        self.assertEqual(listing.returncode, 0, listing.stderr)
        data = json.loads(CONFIG.read_text(encoding="utf-8"))
        measures = {
            lane: [glob_regex(g) for g in spec["measures"]] for lane, spec in data["lanes"].items()
        }
        rows = [(r["lane"], glob_regex(r["glob"])) for r in data["exclusions"]]
        tracked = [p for p in listing.stdout.split("\0") if p]
        scanned = []
        hinted = []
        for path in tracked:
            lanes = [lane for lane, pats in measures.items() if any(p.match(path) for p in pats)]
            if not lanes:
                continue
            scanned.append(path)
            text = (REPO / path).read_text(encoding="utf-8", errors="replace")
            if HINTS.search(text) and not any(
                lane in lanes and pat.match(path) for lane, pat in rows
            ):
                hinted.append(path)
        examined("measured files", scanned)
        self.assertEqual(hinted, [])
        self.assertIsNotNone(HINTS.search("x = 1  # pragma: no cover"))
        self.assertIsNone(HINTS.search("x = 1"))

    def test_every_row_matches_a_tracked_file(self):
        listing = subprocess.run(
            ["git", "-C", str(REPO), "ls-files", "-z"], capture_output=True, text=True, check=False
        )
        self.assertEqual(listing.returncode, 0, listing.stderr)
        tracked = [p for p in listing.stdout.split("\0") if p]
        rows = examined("exclusion rows", self.rows())
        for row in rows:
            if row["kind"] == "generated":
                continue
            pattern = glob_regex(row["glob"])
            with self.subTest(row["glob"]):
                self.assertTrue(
                    any(pattern.match(p) for p in tracked), row["glob"] + " matches no tracked file"
                )
        self.assertIsNone(glob_regex("crates/bot/**").match("web/app/src/a.ts"))


def census_problems(text):
    """The ways a coverage workflow text can fail to be read-only, bounded and pinned."""
    problems = []
    top = re.search(r"(?m)^permissions:\n((?:  .*\n)+)", text)
    if top is None or top.group(1).strip() != "contents: read":
        problems.append("the token is not contents: read alone")
    if re.search(r"(?m)^[ \t]+permissions:", text):
        problems.append("a job sets its own permissions")
    for needle in ("continue-on-error", "secrets.", "github.token"):
        if needle in text:
            problems.append("the workflow holds " + needle)
    for use in re.findall(r"(?m)^\s+(?:- )?uses: (\S+)", text):
        if not re.search(r"@[0-9a-f]{40}$", use):
            problems.append("an action is not pinned by its full commit: " + use)
    jobs = job_texts(text)
    if len(jobs) != 4:
        problems.append("the workflow does not hold four jobs")
    for name, body in jobs.items():
        if not re.search(r"(?m)^    timeout-minutes: [0-9]+$", body):
            problems.append(name + " has no timeout")
    for lane in LANES:
        body = jobs.get("coverage-" + lane, "")
        want = f"    if: ${{{{ needs.coverage-plan.outputs.{lane} == 'true' }}}}\n"
        if want not in body:
            problems.append(lane + " does not read its plan output")
    if "--ignore-run-fail " not in jobs.get("coverage-rust", ""):
        problems.append("the Rust run does not keep going past a failing test")
    for name, body in jobs.items():
        if name != "coverage-plan" and "if: ${{ always() }}" not in body:
            problems.append(name + " does not upload under always")
    return problems


def job_texts(text):
    head, _, rest = text.partition("\njobs:\n")
    out = {}
    parts = re.split(r"(?m)^  ([a-z-]+):\n", rest)
    for i in range(1, len(parts) - 1, 2):
        out[parts[i]] = parts[i + 1]
    return out


def needs_named(text):
    """Every name a `needs:` of a workflow text lists, inline, scalar or block."""
    names = []
    for m in re.finditer(r"(?m)^\s+needs:[ \t]*(.*)$", text):
        rest = m.group(1).strip()
        if rest:
            names += [n.strip() for n in rest.strip("[]").split(",") if n.strip()]
        else:
            tail = text[m.end() + 1 :].splitlines()
            for line in tail:
                item = re.match(r"^\s+- (\S+)\s*$", line)
                if not item:
                    break
                names.append(item.group(1))
    return names


class TheWorkflowIsAdvisory(unittest.TestCase):
    def advisory_exits(self):
        """A lane exits 1 on VOID and 0 on a measured report whatever its changed lines read."""
        with tempfile.TemporaryDirectory() as tmp:
            void = report(tmp, (FIXTURES / "examined-none.lcov").read_text(encoding="utf-8"))
            self.assertEqual(void.returncode, 1, said(void))
            measured = report(
                tmp,
                (FIXTURES / "uncovered.lcov").read_text(encoding="utf-8"),
                diff_text=(FIXTURES / "uncovered.diff").read_text(encoding="utf-8"),
            )
            self.assertEqual(measured.returncode, 0, said(measured))
            self.assertIn("not executed: scripts/planted_control.py:3", measured.stdout)

    def test_no_needs_and_no_ruleset_names_a_coverage_job(self):
        workflows = examined(
            "workflow files", sorted(p for p in (REPO / ".github" / "workflows").glob("*.yml"))
        )
        found = []
        for path in workflows:
            if path.name == "coverage.yml":
                continue
            found += [
                (path.name, n)
                for n in needs_named(path.read_text(encoding="utf-8"))
                if "coverage" in n
            ]
        self.assertEqual(found, [])
        rulesets = examined("ruleset files", sorted((REPO / ".github" / "rulesets").glob("*.json")))
        for path in rulesets:
            self.assertNotIn("coverage", path.read_text(encoding="utf-8").lower(), path.name)
        planted = {
            "an inline list": "  a:\n    needs: [build, coverage-rust]\n",
            "a scalar": "  a:\n    needs: coverage-plan\n",
            "a block list": "  a:\n    needs:\n      - build\n      - coverage-web\n",
        }
        for name, text in examined("planted needs", planted.items()):
            self.assertTrue([n for n in needs_named(text) if "coverage" in n], name)
        self.assertEqual(needs_named("  a:\n    needs: [build]\n"), ["build"])
        self.advisory_exits()

    def test_the_coverage_workflow_is_read_only_bounded_and_pinned(self):
        text = WORKFLOW.read_text(encoding="utf-8")
        self.assertEqual(census_problems(text), [])
        planted = {
            "a write token": text.replace("  contents: read", "  contents: write", 1),
            "a swallowed failure": text.replace(
                "    steps:\n", "    steps:\n      - continue-on-error: true\n", 1
            ),
            "a secret": text.replace(
                "EVENT: ${{ github.event_name }}", "EVENT: ${{ secrets.EVENT }}", 1
            ),
            "an unpinned action": re.sub(
                r"(actions/checkout@)[0-9a-f]{40}", r"\g<1>v7", text, count=1
            ),
            "a job with no timeout": text.replace("    timeout-minutes: 5\n", "", 1),
            "a Rust lane that always runs": text.replace(
                "    if: ${{ needs.coverage-plan.outputs.rust == 'true' }}",
                "    if: ${{ always() }}",
                1,
            ),
            "a Rust run that stops at a failing test": text.replace("--ignore-run-fail ", "", 1),
        }
        for name, planted_text in examined("planted workflows", planted.items()):
            self.assertNotEqual(planted_text, text, name)
            self.assertNotEqual(census_problems(planted_text), [], name)
        self.advisory_exits()

    def test_each_lane_runs_its_planted_controls_first(self):
        text = WORKFLOW.read_text(encoding="utf-8")
        jobs = job_texts(text)
        counts = {"rust": 3, "web": 4, "python": 3}
        for lane in examined("lanes", LANES):
            body = jobs["coverage-" + lane]
            steps = [s for s in re.split(r"(?m)^      - ", body)[1:]]
            self.assertTrue(steps[0].startswith("uses: actions/checkout@"), lane)
            self.assertIn(
                f"run: python3 scripts/coverage-report.py controls --lane {lane}", steps[1], lane
            )
            done = run("controls", "--lane", lane)
            self.assertEqual(done.returncode, 0, said(done))
            n = counts[lane]
            self.assertIn(
                f"coverage-controls: {n} of {n} planted cases read as planted", done.stdout, lane
            )
        provider = "python3 scripts/coverage-report.py provider --lockfile pnpm-lock.yaml"
        self.assertEqual(text.count(provider), 1)
        web = jobs["coverage-web"]
        self.assertLess(web.index(provider), web.index("vitest run --coverage"))
        for path in (REPO / ".github" / "workflows").glob("*.yml"):
            if path.name != "coverage.yml":
                self.assertNotIn(
                    "coverage-report.py provider", path.read_text(encoding="utf-8"), path.name
                )
        self.assertTrue(controls_first(jobs["coverage-python"], "python"))
        steps = re.split(r"(?m)^      - ", jobs["coverage-python"])
        reordered = "\n      - ".join([steps[0], steps[1], *steps[3:], steps[2]])
        self.assertFalse(controls_first(reordered, "python"))
        self.advisory_exits()

    def test_a_planted_case_that_misreads_makes_the_controls_void(self):
        with tempfile.TemporaryDirectory() as tmp:
            for fixture in examined("fixtures", sorted(FIXTURES.iterdir())):
                write(tmp, fixture.name, fixture.read_text(encoding="utf-8"))
            write(tmp, "examined-none.lcov", lcov("scripts/planted_control.py", [(1, 1)]))
            done = run("controls", "--lane", "python", "--fixtures", tmp)
            self.assertEqual(done.returncode, 1, said(done))
            self.assertIn("examined-none.lcov", said(done))
            self.assertIn("VOID", said(done))
            write(
                tmp,
                "examined-none.lcov",
                (FIXTURES / "examined-none.lcov").read_text(encoding="utf-8"),
            )
            write(tmp, "provider-mismatch.txt", "  vitest@5.0.5:\n  '@vitest/coverage-v8@5.0.5':\n")
            web = run("controls", "--lane", "web", "--fixtures", tmp)
            self.assertEqual(web.returncode, 1, said(web))
            self.assertIn("provider-mismatch.txt", said(web))
        self.advisory_exits()


def controls_first(body, lane):
    """True when the second step of a lane job, after the checkout, runs its planted controls."""
    steps = re.split(r"(?m)^      - ", body)[1:]
    return len(steps) > 1 and (
        f"run: python3 scripts/coverage-report.py controls --lane {lane}" in steps[1]
    )


def gate_suites():
    text = (REPO / "scripts" / "check.sh").read_text(encoding="utf-8")
    found = re.findall(r"for suite in ([a-z/ -]+); do", text)
    return found


def workflow_suites(text):
    return re.findall(r"for suite in ([a-z/ -]+); do", text)


def hash_problems(text):
    problems = []
    lines = [ln for ln in text.splitlines() if ln and not ln.startswith("#")]
    if not lines or not re.fullmatch(r"coverage==[0-9]+[.][0-9]+[.][0-9]+ \\", lines[0]):
        problems.append("no exact pin")
    hashes = lines[1:]
    if not hashes:
        problems.append("no hashes")
    for i, ln in enumerate(hashes):
        want = r"    --hash=sha256:[0-9a-f]{64}" + (r" \\" if i < len(hashes) - 1 else "")
        if not re.fullmatch(want, ln):
            problems.append("a hash line is not the shape: " + ln[:30])
    return problems


class ThePythonLane(unittest.TestCase):
    def test_the_python_lane_runs_the_suites_the_gate_runs(self):
        gate = examined("gate suite lists", gate_suites())
        self.assertEqual(gate, ["scripts/tests tools/parity-oracle agent/tests"])
        lane = job_texts(WORKFLOW.read_text(encoding="utf-8"))["coverage-python"]
        self.assertEqual(workflow_suites(lane), gate)
        self.assertIn("coverage run -m unittest discover", lane)
        self.assertEqual(
            workflow_suites("for suite in scripts/tests tools/parity-oracle; do"),
            ["scripts/tests tools/parity-oracle"],
        )
        self.assertNotEqual(
            workflow_suites("for suite in scripts/tests tools/parity-oracle; do"), gate
        )

    def test_coverage_py_is_hash_pinned_on_the_ctrace_core(self):
        requirements = (REPO / "tools" / "coverage" / "requirements.txt").read_text(
            encoding="utf-8"
        )
        self.assertEqual(hash_problems(requirements), [])
        planted = {
            "a range": requirements.replace("coverage==", "coverage>=", 1),
            "no hashes": requirements.split("--hash=")[0].rstrip(" \\\n") + "\n",
            "a short hash": requirements.replace("--hash=sha256:", "--hash=sha256:ab", 1),
        }
        for name, text in examined("planted requirements", planted.items()):
            self.assertNotEqual(hash_problems(text), [], name)
        toml = (REPO / "tools" / "coverage" / "coverage.toml").read_text(encoding="utf-8")
        for line in examined(
            "settings",
            ['core = "ctrace"', "branch = true", 'patch = ["subprocess"]', "parallel = true"],
        ):
            self.assertRegex(toml, "(?m)^" + re.escape(line) + "$")
        lane = job_texts(WORKFLOW.read_text(encoding="utf-8"))["coverage-python"]
        self.assertIn("pip install --require-hashes --no-deps --only-binary :all:", lane)
        self.assertIn("-r tools/coverage/requirements.txt", lane)
        self.assertIn("COVERAGE_RCFILE: ${{ github.workspace }}/tools/coverage/coverage.toml", lane)


def vitest_problems(text):
    problems = []
    block = re.search(r"(?s)    coverage: \{\n(.*?)\n    \},", text)
    if block is None:
        return ["no coverage block"]
    body = block.group(1)
    for want in ("provider: 'v8',", "enabled: false,", "reportOnFailure: true"):
        if want not in body:
            problems.append("the block lacks " + want)
    if "'src/**/*.{ts,js,svelte}'" not in body or "'web/app/src/**/*.{ts,js,svelte}'" not in body:
        problems.append("the block does not include every app source file")
    return problems


def catalog_problems(workspace, manifest):
    problems = []
    entry = re.search(r"(?m)^  '@vitest/coverage-v8': ([^\n]+)$", workspace)
    vitest = re.search(r"(?m)^  vitest: ([^\n]+)$", workspace)
    if entry is None or vitest is None:
        return ["the catalog lacks the provider or vitest"]
    if not re.fullmatch(r"[0-9]+[.][0-9]+[.][0-9]+", entry.group(1)):
        problems.append("the provider is not one exact version")
    lines = workspace.splitlines()
    if abs(lines.index(entry.group(0)) - lines.index(vitest.group(0))) != 1:
        problems.append("the provider is not beside vitest")
    if '"@vitest/coverage-v8": "catalog:"' not in manifest:
        problems.append("the root manifest does not take the provider from the catalog")
    return problems


class TheWebLane(unittest.TestCase):
    def test_the_vitest_coverage_block_stays_off_unless_asked(self):
        text = (REPO / "vitest.config.ts").read_text(encoding="utf-8")
        self.assertEqual(vitest_problems(text), [])
        planted = {
            "a block that is on": text.replace("enabled: false,", "enabled: true,", 1),
            "another provider": text.replace("provider: 'v8',", "provider: 'istanbul',", 1),
            "no report on failure": text.replace(
                "reportOnFailure: true", "reportOnFailure: false", 1
            ),
            "no block": text.replace("coverage: {", "other: {", 1),
        }
        for name, planted_text in examined("planted configs", planted.items()):
            self.assertNotEqual(planted_text, text, name)
            self.assertNotEqual(vitest_problems(planted_text), [], name)

    def test_the_catalog_pins_the_provider_beside_vitest(self):
        workspace = (REPO / "pnpm-workspace.yaml").read_text(encoding="utf-8")
        manifest = (REPO / "package.json").read_text(encoding="utf-8")
        self.assertEqual(catalog_problems(workspace, manifest), [])
        planted = {
            "a caret": workspace.replace("coverage-v8': ", "coverage-v8': ^", 1),
            "a provider apart from vitest": workspace.replace(
                "  vitest: ", "  other: 1.0.0\n  vitest: ", 1
            ),
        }
        for name, text in examined("planted catalogs", planted.items()):
            self.assertNotEqual(text, workspace, name)
            self.assertNotEqual(catalog_problems(text, manifest), [], name)
        self.assertNotEqual(catalog_problems(workspace, manifest.replace("catalog:", "5", 1)), [])


def pip_problems(text):
    entries = re.findall(r"(?m)^  - package-ecosystem: pip\n((?:    .*\n|      .*\n)+)", text)
    if len(entries) != 1:
        return ["not exactly one pip entry"]
    body = entries[0]
    problems = []
    for want in (
        "    directory: /tools/coverage\n",
        "      interval: weekly\n",
        "    target-branch: dev\n",
        'update-types: ["version-update:semver-major"]',
    ):
        if want not in body:
            problems.append("the pip entry lacks " + want.strip())
    if not re.search(r"(?m)^    cooldown:\n      default-days: [0-9]+$", body):
        problems.append("the pip entry has no cooldown")
    return problems


class TheUpdatePath(unittest.TestCase):
    def test_the_requirements_file_has_a_dependabot_update_path(self):
        text = (REPO / ".github" / "dependabot.yml").read_text(encoding="utf-8")
        self.assertEqual(pip_problems(text), [])
        self.assertEqual(text.count("package-ecosystem: pip"), 1)
        planted = {
            "no entry": text.split("  - package-ecosystem: pip")[0],
            "the root directory": text.replace(
                "    directory: /tools/coverage", "    directory: /", 1
            ),
            "a directory that is not the file's": text.replace("/tools/coverage", "/tools", 1),
            "no cooldown": text.replace(
                "    cooldown:\n      default-days: 7\n    target-branch", "    target-branch"
            ),
        }
        for name, planted_text in examined("planted configs", planted.items()):
            self.assertNotEqual(planted_text, text, name)
            self.assertNotEqual(pip_problems(planted_text), [], name)


if __name__ == "__main__":
    unittest.main()

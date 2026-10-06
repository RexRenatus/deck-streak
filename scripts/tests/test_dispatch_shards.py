"""A package dispatch is sharded by its projected weight (SPEC-129 A1 to A6).

The sizing verb is run as the workflow runs it, on fixture listings written to a temporary
directory; the workflow is read as text with the helpers `test_mutation_workflows.py` uses. The
plants put the fixed 32 back into each place that must read the one count.
"""

import argparse
import contextlib
import functools
import importlib.util
import io
import itertools
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
import tomllib
import unittest
from pathlib import Path
from unittest import mock

from _mutants_finder import BOUNDS, Refused, mutants_in, run_texts
from _support import REPO, examined
from test_mutation_verdict import verdict_module
from test_mutation_workflows import CI, VERDICT, WEEKLY, WORKFLOWS, jobs, listed, shard, workflow

WHOLE = 32
COUNT = "${{ needs.size.outputs.shards }}"
# A command is `cargo mutants` where bash itself runs `cargo` (or the `cargo-mutants` binary) as
# the program and the first word past cargo's own flags is `mutants`; `mutants_in()` below finds
# each (imported from `_mutants_finder`) with the words bash passes it, or refuses the workflow. The bounds are matched whole, so a
# digit or a decimal appended to either value is not the gate's bound.
# The bounds are four whole words: only a blank or an end may stand on either side.
BOUNDED = re.compile(r"(?<!\S)" + re.escape(BOUNDS) + r"(?!\S)")
#: The bounds' four words. A fixture that stands for the gate's bounds is spelled from them, so it
#: passes or fails for its own reason (a quoting, a redirection, a matrix value), never because
#: the bound moved (SPEC-327).
TIMEOUT_FLAG, TIMEOUT_SECONDS, BUILD_FLAG, BUILD_SECONDS = BOUNDS.split()
TIMEOUT, BUILD = f"{TIMEOUT_FLAG} {TIMEOUT_SECONDS}", f"{BUILD_FLAG} {BUILD_SECONDS}"


def entries(package, count):
    """`count` listed mutants of `package`, as cargo-mutants lists them, in listing order."""
    return [listed(f" ({n})", package=package) for n in range(count)]


def size(listing, package=None, *, raw=None):
    """Run `size` as the workflow does; returns (exit code, stdout, the step's outputs)."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        path = root / "package.json"
        path.write_text(raw if raw is not None else json.dumps(listing), encoding="utf-8")
        sink = root / "output"
        sink.touch()
        args = [sys.executable, str(VERDICT), "size", "--listed", str(path)]
        if package is not None:
            args += ["--package", package]
        done = subprocess.run(
            args,
            capture_output=True,
            text=True,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1", GITHUB_OUTPUT=str(sink)),
            timeout=120,
            check=False,
        )
        written = dict(
            line.split("=", 1) for line in sink.read_text(encoding="utf-8").splitlines() if line
        )
        return done.returncode, done.stdout + done.stderr, written


def count_problems(text):
    """What the workflow leaves fixed, or unread, where the one count belongs."""
    found = jobs(text)
    problems = []
    if "size" not in found:
        return ["no size job"]
    if not re.search(r"(?m)^    outputs:\n(?:      .*\n)*?      shards: ", found["size"]):
        problems.append("the size job outputs no shards")
    if not re.search(r"(?m)^    outputs:\n(?:      .*\n)*?      matrix: ", found["size"]):
        problems.append("the size job outputs no matrix")
    rust = found.get("rust", "")
    if not re.search(r"(?m)^    needs: \[?size\]?$", rust):
        problems.append("the rust job does not need size")
    if "shard: ${{ fromJSON(needs.size.outputs.matrix) }}" not in rust:
        problems.append("the rust matrix is not the sized one")
    if f"SHARDS: {COUNT}" not in rust or '--shard "$SHARD/$SHARDS"' not in rust:
        problems.append("the rust legs' --shard does not read the sized count")
    survivors = found.get("survivors", "")
    if not re.search(r"(?m)^    needs: \[[^\]]*\bsize\b[^\]]*\]$", survivors):
        problems.append("the survivors job does not need size")
    if f"SHARDS: {COUNT}" not in survivors or '--shards "$SHARDS"' not in survivors:
        problems.append("the battery's --shards does not read the sized count")
    for name, job in found.items():
        if name == "rehearsal":
            continue
        for hit in re.findall(r"--shards? (?:\"\$SHARD/)?\d+", job):
            problems.append(f"{name}: a fixed count, {hit}")
    return problems


class TheDispatchIsSizedFromItsListing(unittest.TestCase):
    """A1 (R1, R2): the fewest round-robin shards whose slowest fits the bound, by the plan's own
    function; a projection past the matrix's limit is refused with its projection."""

    def test_a_small_package_takes_one_shard_and_a_large_one_the_fewest_within_the_bound(self):
        # Costs and bound are the plan's: the daemon costs 80 s a mutant on a 1768 s baseline, and
        # 112 mutants project to 10728 s of a 10800 s bound; the ingest costs 126 s, 71 to a leg.
        cases = [
            ("deck-streak-kernel", 5, 1),
            ("deck-streak-daemon", 112, 1),
            ("deck-streak-daemon", 113, 2),
            ("deck-streak-ingest", 150, 3),
        ]
        for package, count, expected in examined("listings sized", cases):
            code, out, written = size(entries(package, count), package)
            self.assertEqual(code, 0, out)
            self.assertEqual(written.get("shards"), str(expected), f"{package} {count}: {out}")
            self.assertEqual(json.loads(written.get("matrix", "null")), list(range(expected)), out)
            self.assertIn(f"{expected} shard(s) for {count} listed mutant(s)", out)

    def test_the_sizing_is_the_per_pull_request_plans_own_function(self):
        for package, count in examined(
            "packages", [("deck-streak-ingest", 200), ("deck-streak-api", 900)]
        ):
            listing = entries(package, count)
            code, out, written = size(listing, package)
            self.assertEqual(code, 0, out)
            with tempfile.TemporaryDirectory() as scratch:
                plan = Path(scratch) / "plan.json"
                plan.write_text(json.dumps({"classes": {"rust": {"applies": True}}}), "utf-8")
                whole = Path(scratch) / "listed.json"
                whole.write_text(json.dumps(listing), encoding="utf-8")
                done = subprocess.run(
                    [
                        sys.executable,
                        str(VERDICT),
                        "shards",
                        "--plan",
                        str(plan),
                        "--listed",
                        str(whole),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=120,
                    env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
                )
                self.assertEqual(done.returncode, 0, done.stdout)
                planned = json.loads(plan.read_text("utf-8"))["shards"]["count"]
            self.assertEqual(written.get("shards"), str(planned), out)

    def test_a_projection_past_the_limit_is_refused_with_its_projection_never_capped(self):
        code, out, written = size(entries("deck-streak-ingest", 17000), "deck-streak-ingest")
        self.assertEqual(code, 1, out)
        self.assertIn("REFUSED", out)
        self.assertIn("17000 mutant(s), projected at 2142000 s serially", out)
        self.assertEqual(written, {}, "a refused sizing wrote outputs")

    def test_a_package_that_lists_nothing_or_a_non_listing_is_not_sized(self):
        code, out, written = size([], "deck-streak-ingest")
        self.assertEqual((code, written.get("shards")), (0, "1"), out)
        code, out, written = size(None, "deck-streak-ingest", raw="not json")
        self.assertEqual(code, 3, out)
        self.assertIn("VOID", out)
        self.assertEqual(written, {}, out)


#: #691's listing, the release range whose legs one run must hold (SPEC-362 A1): each package's
#: mutant count in the listing's own package order, from the plan artifact of run 37410215011.
RELEASE_691 = (
    ("agent", 239),
    ("analytics", 335),
    ("api", 164),
    ("bot", 437),
    ("coordination", 977),
    ("curriculum", 198),
    ("daemon", 211),
    ("economy", 137),
    ("engine-core", 131),
    ("ffi", 55),
    ("fsrs7", 95),
    ("habits", 129),
    ("identity", 143),
    ("ingest", 710),
    ("insights", 93),
    ("kernel", 781),
    ("mcp", 122),
    ("notifications", 599),
    ("privacy", 29),
    ("progression", 399),
    ("push", 138),
    ("quests", 250),
    ("readings", 239),
    ("streaks", 318),
    ("vault", 1338),
    ("web-engine", 100),
)
#: The most jobs one workflow run holds, GitHub's matrix limit: the legs and every other job of
#: the workflow share it (SPEC-362 R3).
RUN_JOBS = 256
#: The axis the pull request's Python matrix reads: the plan writes at most `PYTHON_MAX_SHARDS`.
PYTHON_AXIS = "${{ fromJSON(needs.mutation-plan.outputs.python_matrix) }}"


def plan_shards(listing):
    """Run `shards` as the plan's job does, over a plan whose Rust class applies and `listing`:
    (exit code, stdout and stderr, the plan as it was left)."""
    with tempfile.TemporaryDirectory() as scratch:
        plan = Path(scratch) / "plan.json"
        plan.write_text(json.dumps({"classes": {"rust": {"applies": True}}}), "utf-8")
        whole = Path(scratch) / "listed.json"
        whole.write_text(json.dumps(listing), encoding="utf-8")
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        env.pop("GITHUB_OUTPUT", None)
        done = subprocess.run(
            [sys.executable, str(VERDICT), "shards", "--plan", str(plan), "--listed", str(whole)],
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
            env=env,
        )
        return done.returncode, done.stdout + done.stderr, json.loads(plan.read_text("utf-8"))


def generated(job, python_most):
    """The most jobs one workflow job can generate: one, or the product of its matrix's axes, a
    listed axis by its length and the Python plan's axis by the most shards the plan writes. An
    axis this cannot count is refused, never counted as one."""
    block = re.search(r"(?ms)^      matrix:\n(.*?)(?=^      \S|^    \S|\Z)", job)
    if block is None:
        return 1
    most = 1
    for line in block.group(1).splitlines():
        if not line.strip() or line.strip().startswith("#"):
            continue
        axis = re.fullmatch(r"        ([\w-]+): (.+)", line)
        if axis is None:
            raise AssertionError(f"a matrix line this cannot count: {line!r}")
        value = axis.group(2)
        if value.startswith("[") and value.endswith("]"):
            most *= len([each for each in value[1:-1].split(",") if each.strip()])
        elif value == PYTHON_AXIS:
            most *= python_most
        else:
            raise AssertionError(f"a matrix axis this cannot count: {line!r}")
    return most


class TheReleaseFitsOneRun(unittest.TestCase):
    """SPEC-362 A1 to A4, A10 and A11 (R1 to R4, R11): a release's legs are sized within the run
    that judges them, at half the hosted job's limit, and the battery is sized the same way."""

    def test_the_release_listing_fits_one_run(self):
        listing = [
            entry for name, count in RELEASE_691 for entry in entries(f"deck-streak-{name}", count)
        ]
        self.assertEqual(len(examined("release mutants", listing)), 8367)
        code, out, planned = plan_shards(listing)
        self.assertEqual(code, 0, out)
        self.assertEqual(planned["shards"]["count"], 200, out)
        self.assertLessEqual(planned["shards"]["count"], 209, out)
        slowest = max(leg["projected_seconds"] for leg in planned["shards"]["shards"])
        self.assertLessEqual(slowest, 10800, out)

    def test_the_bound_is_half_of_every_legs_timeout(self):
        minutes = {}
        for path, leg in examined("leg jobs", [(CI, "mutation-rust"), (WEEKLY, "rust")]):
            job = jobs(workflow(path)).get(leg, "")
            found = re.findall(r"(?m)^    timeout-minutes: (\d+)$", job)
            self.assertEqual(found, ["360"], leg)
            minutes[leg] = int(found[0])
        module = verdict_module()
        for leg, value in minutes.items():
            self.assertEqual(module.SHARD_BOUND_SECONDS, value * 60 // 2, leg)

    def test_the_ceiling_is_the_runs_job_budget(self):
        module = verdict_module()
        derived = {}
        for key, path, leg in examined(
            "workflows that run legs", [("ci", CI, "mutation-rust"), ("battery", WEEKLY, "rust")]
        ):
            found = jobs(workflow(path))
            self.assertIn(leg, found, key)
            others = [
                generated(job, module.PYTHON_MAX_SHARDS)
                for name, job in found.items()
                if name != leg
            ]
            derived[key] = RUN_JOBS - sum(others)
        self.assertEqual(getattr(module, "LEG_CEILING", None), derived)
        self.assertEqual(derived, {"ci": 209, "battery": 234})

    def test_a_listing_beyond_the_ceiling_is_refused_whole(self):
        # 71 ingest mutants of 126 s fit a leg's 10800 s after its 1768 s baseline, so the
        # battery's 234 legs hold 16614 and the pull request's 209 legs hold 14839.
        code, out, written = size(entries("deck-streak-ingest", 16615), "deck-streak-ingest")
        self.assertEqual(code, 1, out)
        self.assertIn(
            "REFUSED: 16615 mutant(s), projected at 2093490 s serially, need more than 234 legs",
            out,
        )
        self.assertEqual(written, {}, "a refused sizing wrote outputs")
        code, out, written = size(entries("deck-streak-ingest", 16614), "deck-streak-ingest")
        self.assertEqual((code, written.get("shards")), (0, "234"), out)
        code, out, planned = plan_shards(entries("deck-streak-ingest", 14840))
        self.assertEqual(code, 1, out)
        self.assertIn(
            "REFUSED: 14840 mutant(s), projected at 1869840 s serially, need more than 209 legs",
            out,
        )
        self.assertNotIn("shards", planned)

    def test_the_whole_tree_is_sized_from_its_listing(self):
        # 14840 ingest mutants need 210 legs: one past the pull request's ceiling, within the
        # battery's, which sizes the whole tree as it sizes a package.
        cases = [(entries("deck-streak-kernel", 5), 1), (entries("deck-streak-ingest", 14840), 210)]
        for listing, expected in examined("whole-tree listings", cases):
            code, out, written = size(listing)
            self.assertEqual(code, 0, out)
            self.assertEqual(written.get("shards"), str(expected), out)
            self.assertEqual(json.loads(written.get("matrix", "null")), list(range(expected)), out)
            self.assertIn(f"{expected} shard(s) for {len(listing)} listed mutant(s)", out)
        code, out, written = size(None, None, raw="not json")
        self.assertEqual((code, written), (3, {}), out)
        code, out, written = size(entries("a", 3), "miniapp")
        self.assertEqual((code, written.get("shards")), (0, "1"), out)

    def test_every_sizing_prints_its_headroom(self):
        code, out, _ = size(entries("deck-streak-daemon", 113), "deck-streak-daemon")
        self.assertEqual(code, 0, out)
        self.assertIn("legs 2 of ceiling 234", out)
        code, out, _ = size(entries("deck-streak-kernel", 5))
        self.assertEqual(code, 0, out)
        self.assertIn("legs 1 of ceiling 234", out)
        code, out, _ = plan_shards(entries("deck-streak-daemon", 113))
        self.assertEqual(code, 0, out)
        self.assertIn("legs 2 of ceiling 209", out)


class TheWorkflowReadsTheOneCount(unittest.TestCase):
    """A3 (R4): the rust matrix, its --shard argument and the battery's --shards read one count."""

    def test_the_matrix_the_argument_and_the_battery_read_the_sized_count(self):
        text = workflow(WEEKLY)
        for reader in (f"SHARDS: {COUNT}", '--shard "$SHARD/$SHARDS"', '--shards "$SHARDS"'):
            self.assertIn(reader, text)
        self.assertEqual(count_problems(text), [])

    def test_a_plant_that_puts_the_fixed_count_back_goes_red(self):
        text = workflow(WEEKLY)
        plants = [
            ('--shard "$SHARD/$SHARDS"', '--shard "$SHARD/32"'),
            (f"SHARDS: {COUNT}", "SHARDS: 32"),
            ("shard: ${{ fromJSON(needs.size.outputs.matrix) }}", "shard: [0, 1, 2]"),
            ('--shards "$SHARDS"', "--shards 32"),
        ]
        for original, planted in examined("plants", plants):
            self.assertIn(original, text, f"the workflow lacks {original}")
            self.assertNotEqual(count_problems(text.replace(original, planted)), [], planted)

    def test_the_size_job_lists_the_package_with_the_gates_own_bounds_and_sizes_it(self):
        size_job = jobs(workflow(WEEKLY)).get("size", "")
        commands = re.findall(r"cargo mutants [^\n]*", size_job)
        self.assertEqual(len(commands), 2, "the size job lists in one branch per package state")
        for flag in (
            "--no-shuffle",
            "--list",
            "--json",
            "--in-place",
            TIMEOUT,
            BUILD,
        ):
            for command in commands:
                self.assertIn(flag, command)
        self.assertIn('--package="$PACKAGE"', commands[0])
        self.assertNotIn("--package", commands[1])
        for block in re.findall(r"(?ms)^        run: [|]?\n?(.*?)(?=^      - |\Z)", size_job):
            self.assertNotIn("inputs.package", block, "the size job interpolates the input")
        self.assertIn("mutation-verdict.py size", size_job)


class TheBatteryRefusesAForeignShardCount(unittest.TestCase):
    """A4 (R5): a report set whose shard count differs from n fails the battery by name."""

    def run_battery(self, reports_count, shards):
        with tempfile.TemporaryDirectory() as scratch:
            reports = Path(scratch) / "reports"
            scope = entries("deck-streak-fix", 4)
            (reports / "listing").mkdir(parents=True)
            (reports / "listing" / "whole.json").write_text(json.dumps(scope), "utf-8")
            for number in range(reports_count):
                shard(reports, number, [(scope[number % 4], "CaughtMutant")], code="0")
            done = subprocess.run(
                [
                    sys.executable,
                    str(VERDICT),
                    "battery",
                    "--reports",
                    str(reports),
                    "--shards",
                    str(shards),
                    "--package",
                    "deck-streak-fix",
                    "--listed",
                    str(reports / "listing" / "whole.json"),
                ],
                capture_output=True,
                text=True,
                check=False,
                timeout=120,
                env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1"),
            )
            return done.returncode, done.stdout

    def test_a_report_beyond_the_count_fails_the_battery(self):
        code, out = self.run_battery(3, 2)
        self.assertEqual(code, 1, out)
        self.assertIn("battery: FOREIGN mutants-shard-2: the run was sized at 2 shard(s)", out)

    def test_a_report_set_of_exactly_the_count_passes(self):
        code, out = self.run_battery(2, 2)
        self.assertEqual(code, 0, out)
        self.assertIn("battery: counted 3 of 3 reports whole", out)


class TheExaminedTotalIsTheListing(unittest.TestCase):
    """A5 (R6): the sized shards take every listed mutant once, as the 32 shards do."""

    def test_the_union_of_the_sized_shards_equals_the_union_of_the_thirty_two(self):
        listing = entries("deck-streak-daemon", 133)
        code, out, written = size(listing, "deck-streak-daemon")
        self.assertEqual(code, 0, out)
        count = int(written["shards"])
        names = [entry["name"] for entry in listing]

        def taken(total):
            return [name for i in range(total) for n, name in enumerate(names) if n % total == i]

        self.assertGreater(WHOLE, count)
        self.assertEqual(sorted(taken(count)), sorted(taken(WHOLE)))
        self.assertEqual(len(taken(count)), len(names))


# The memory scope's wrapper (SPEC-196) runs the words after its `--` as the command, unchanged:
# `TheMemoryScopeRunsTheWordsAfterItsSeparator` measures that of the file, so its form is read
# through. A value bash hands on as exactly one word: literal, or double quotes around literal text
# and named expansions (no split, no glob, no `@`).
WRAPPER = ("python3", "scripts/memory_scope.py")
ONE_WORD = re.compile(r'"(?:[^"\\$`]|\$[A-Za-z_][A-Za-z0-9_]*|\$\{[A-Za-z_][A-Za-z0-9_]*\})*"')


@functools.cache
def wrapper_options():
    """The options the wrapper's own parser declares that take one value, read from the parser."""
    return declared_options(wrapper_module(MEMORY_SCOPE))


def wrapped(words, program):
    """Where the command the wrapper runs begins, when `words[program:]` is the wrapper's form:
    `python3 scripts/memory_scope.py`, then only options its parser declares, each with one value
    that bash hands on as one word, then a literal `--` and a command; else None. A computed word
    where the parser reads an option or the `--` is not the form: it could be `--` (R5)."""
    k = program + len(WRAPPER)
    if [w.value for w in words[program:k]] != list(WRAPPER) or any(
        w.dynamic for w in words[program:k]
    ):
        return None
    options = wrapper_options()
    while k < len(words):
        word = words[k]
        joined = next((o for o in options if word.value.startswith(o + "=")), None)
        if not word.dynamic and word.value == "--":
            return k + 1 if k + 1 < len(words) else None
        if not word.dynamic and word.value in options and k + 1 < len(words):
            value = words[k + 1]
            if value.dynamic and not ONE_WORD.fullmatch(value.raw):
                return None
            k += 2
        elif joined and (not word.dynamic or ONE_WORD.fullmatch(word.raw[len(joined) + 1 :])):
            k += 1
        else:
            return None
    return None


def mutants_commands(directory):
    """{workflow name: its `cargo mutants` commands} for the workflows of a directory that run one;
    a workflow the reader refuses answers with the refusal, which carries no bounds."""
    found = {}
    for path in sorted(directory.iterdir()):
        if path.suffix not in (".yml", ".yaml"):
            continue
        try:
            lines = [
                line
                for text in run_texts(workflow(path))
                for line in mutants_in(text, wrapped=wrapped)
            ]
        except Refused as refusal:
            lines = [f"refused: {refusal}"]
        if lines:
            found[path.name] = lines
    return found


def plant_workflow(run):
    """A workflow whose one step runs `run`, saved as `planted.yml` in a fresh directory."""
    scratch = tempfile.TemporaryDirectory()
    text = f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: {run}\n"
    (Path(scratch.name) / "planted.yml").write_text(text, encoding="utf-8")
    return scratch


# The class (#395) is where a `#` starts a comment, as bash reads it. Its members are generated: a
# fragment that carries a would-be comment or a would-be closer, in a context (and a group inside
# each substitution), then a `#` glued or after a blank, then an unbounded command after the bounded
# one, or the bounds after an unbounded one, or a continued line with a `#` and either after it; or
# a context left open at its line's end or continued inside, its closer and a `#` on the next line
# or the one after, and an unbounded command or the bounds after it. Bash reads every member.
CLASS_WORDS = (
    "@",
    "$(@)",
    "<(@)",
    ">(@)",
    "`@`",
    "$((@))",
    "${X:-@}",
    "${X#@}",
    "${X//@/}",
    '"@"',
    "'@'",
    "$'@'",
    "<<< @",
)
CLASS_GROUPS = ("case x in x) @;; esac", "case x in (x) @;; esac", "{ @; }", "( @ )")
CLASS_COMMANDS = CLASS_GROUPS + ("(( @ ))", "a=(@)", "[[ x =~ @ ]]")
CLASS_FRAGMENTS = ("#", ";#", ")#", "}#", '"}"', "')'", '"#"', "\\#", "a#", "\\'", ";;", "true")
CLASS_SIZE = 4291
UNBOUNDED = "cargo mutants --in-place"
# Bash runs each member from a list, without `-e`, with `cargo` a function that logs its words and
# no other command on the path; a substitution's `cargo` is awaited through the pipe it holds.
ORACLE = r"""
cargo() { local a r="$M"; for a in "$@"; do r+=$'\037'"$a"; done; printf '%s\036' "$r" >> "$LOG"; }
while IFS= read -r f; do
  M=${f##*/}
  if "$SHELL_UNDER_TEST" --noprofile --norc -n "$f" 2>/dev/null; then
    { ( . "$f"; wait ) </dev/null >/dev/null 2>&1; } 9>&1 | { while read -r _; do :; done; }
  fi
done < "$LIST"
"""


def class_members():
    """Every member of the class, as the script bash runs."""
    words, commands = [], []
    for context in CLASS_WORDS:
        quotes = context[-1] in "\"'"
        words += [context.replace("@", f) for f in CLASS_FRAGMENTS if quotes or f != "\\'"]
        if context in ("$(@)", "<(@)", ">(@)", "`@`"):
            for group in CLASS_GROUPS:
                words += [context.replace("@", group.replace("@", f)) for f in CLASS_FRAGMENTS]
    for command in CLASS_COMMANDS:
        commands += [command.replace("@", f) for f in CLASS_FRAGMENTS if f != "\\'"]
    members = []
    for text, lead in [(w, f"cargo mutants {BOUNDS} ") for w in words] + [
        (c, f"cargo mutants {BOUNDS}; ") for c in commands
    ]:
        for mark in ("#", " #"):
            members.append(f"{lead}{text}{mark}; {UNBOUNDED}\n")
            if text in words:
                members.append(f"{UNBOUNDED} {text}{mark} {BOUNDS}\n")
        members.append(f"{lead}{text} \\\n  #; {UNBOUNDED}\n")
        if text in words:
            members.append(f"{UNBOUNDED} {text} \\\n  # {BOUNDS}\n")
    for group in CLASS_GROUPS:
        for fragment in CLASS_FRAGMENTS[:-3] + CLASS_FRAGMENTS[-2:]:
            for mark in ("#", " #"):
                members.append(f"{group.replace('@', f'{UNBOUNDED} {fragment}')}{mark} {BOUNDS}\n")
    for context in CLASS_WORDS + CLASS_COMMANDS:
        start, _, closer = context.partition("@")
        word = context in CLASS_WORDS
        lead = f"cargo mutants {BOUNDS}{' ' if word else '; '}"
        for f in CLASS_FRAGMENTS if closer.strip() else ():
            if f != "\\'" or context[-1] in "\"'":
                for end, line in itertools.product(("", "\\", "\n"), (f"{closer}#", f"#{closer}")):
                    members.append(f"{lead}{start}{f}{end}\n{line}; {UNBOUNDED}\n")
                    if word:
                        members.append(f"{UNBOUNDED} {start}{f}{end}\n{line} {BOUNDS}\n")
    return list(dict.fromkeys(members))  # `"#"` bare and `#` in `"@"` are one member


def unbounded_by_bash(scripts):
    """The indices of the scripts in which bash runs `cargo mutants` without the gate's bounds."""
    shell, bounds = shutil.which("bash"), BOUNDS.split()
    workers = min(8, os.cpu_count() or 1)
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        (root / "empty").mkdir()
        (root / "oracle.sh").write_text(ORACLE, encoding="utf-8")
        for n, text in enumerate(scripts):
            (root / f"m{n}").write_text(text, encoding="utf-8")
        runs = []
        for w in range(workers):
            names = "".join(f"{root / f'm{n}'}\n" for n in range(w, len(scripts), workers))
            (root / f"list{w}").write_text(names, encoding="utf-8")
            env = {
                "PATH": str(root / "empty"),
                "SHELL_UNDER_TEST": shell,
                "LIST": str(root / f"list{w}"),
                "LOG": str(root / f"log{w}"),
            }
            runs.append(
                subprocess.Popen([shell, "--noprofile", "--norc", "oracle.sh"], cwd=root, env=env)
            )
        unbounded = set()
        for w, run in enumerate(runs):
            run.wait(timeout=600)
            log = root / f"log{w}"
            records = log.read_text(encoding="utf-8").split("\x1e")[:-1] if log.exists() else []
            for record in records:
                name, *words = record.split("\x1f")
                mutants = next((word for word in words if word[:1] not in "+-"), None) == "mutants"
                if mutants and not any(words[k : k + 4] == bounds for k in range(len(words))):
                    unbounded.add(int(name[1:]))
        return unbounded


# The grammar class (#395, #447): what bash runs from a `run:` value, drawn at the grammars rather
# than at the guard. Each member is a shell text and one way a workflow spells it: YAML's literal
# and folded blocks, its double-quoted scalar (with `\\`, `\x23` and escaped line breaks), its
# single-quoted and plain scalars, and a `${{ }}` value before, inside or after the command. The
# texts cross a run of 1 to 4 backslashes with each quote it can stand in and what follows it (a
# line break, a blank, a `#`, a quote, the end), each spelling of each word of the command and its
# bounds, here-documents and here-strings, the operators that end a command inside and outside an
# expansion, redirections among the bounds, the programs that run another program, the settings
# that change what bash reads, a matrix value that decides the bounds, and a text handed on (to a
# shell, through a pipe, a variable or a here-document, or inside an expansion) in each spelling.
LEAD = f"cargo mutants {BOUNDS}"
GRAMMAR_SIZE = 6485
GRAMMAR_WRAPPERS = ("env", "nice", "timeout", "xargs", "sh")


def grammar_texts():
    """{axis: the shell texts of that axis}; the first two texts of the first axis are controls."""
    u, b, lead = UNBOUNDED, BOUNDS, LEAD
    axes = {"control": [f"{u}\n", f"{lead}\n"]}
    closers = {"": "", '"': '"', "'": "'", "$'": "'"}
    followers = ("\n", "\n  ", " ", " # ", "#", '"', "'", "")
    axes["backslash"] = [
        f"{first} {quote}x{run}{mid}{second}\n"
        for run in ("\\" * k for k in range(1, 5))
        for quote, closer in closers.items()
        for follower in followers
        for mid in dict.fromkeys((follower + closer, closer + follower))
        for first, second in ((u, b), (lead, u))
    ] + [
        # A run at the end of the value, on its first line or a later one, where a plain
        # scalar's comment can follow it.
        f"{line}{first} x" + "\\" * k
        for k in range(1, 5)
        for first in (u, lead)
        for line in ("", "true\n")
    ]
    words = lead.split()
    spelled = []
    for n, word in enumerate(words):
        for form in (
            "'{w}'",
            '"{w}"',
            "{h}\\{t}",
            "$'{w}'",
            '{w}""',
            "{w}''",
            '{w}"0"',
            "{w}$'0'",
            "{w}\\\n0",
            "{w}{{,0}}",
            "{w}\\ ",
            '{h}"{t}"',
            "{w}\\\n",
        ):
            text = form.format(w=word, h=word[:1], t=word[1:])
            spelled.append(" ".join(words[:n] + [text] + words[n + 1 :]) + "\n")
    spelled += [
        f'cargo mutants "{TIMEOUT}" "{BUILD}"\n',
        "cargo mu$()tants --in-place\n",
        "cargo mu${X-}tants --in-place\n",
        "c$()argo mutants --in-place\n",
        f"cargo mutants {TIMEOUT_FLAG}\\ {TIMEOUT_SECONDS} {BUILD_FLAG}\\ {BUILD_SECONDS}\n",
        f"cargo mutants '{BOUNDS}'\n",
        f"cargo mutants {TIMEOUT}\\\n {BUILD}\n",
        f"cargo mutants {BOUNDS} -- x\n",
        f"cargo mutants -- {BOUNDS}\n",
        "$'\\x63argo' $'\\x6dutants' --in-place\n",
    ]
    axes["words"] = spelled
    axes["here"] = [
        f"{u} <<< {b}\n",
        f"{u} <<< '{b}'\n",
        f'{u} <<< "{b}"\n',
        f"{u} <<E\n{b}\nE\n",
        f"{u} <<'E'\n{b}\nE\n",
        f"{u} <<-E\n\t{b}\n\tE\n",
        f"cat <<E\n{u}\nE\n",
        f"cat <<'E'\n{u}\nE\n",
        f"bash <<E\n{u}\nE\n",
        f"bash <<'E'\n{u}\nE\n",
        f'bash <<"E"\n{u}\nE\n',
        f"bash <<\\E\n{u}\nE\n",
        f"cat <<E\n$({u})\nE\n",
        f"cat <<'E'\n$({u})\nE\n",
        f"cat <<E\nx\\\nE\n{u}\n",
        f"cat <<'E'\nx\\\nE\n{u}\n",
        f"cat <<E; {u}\nx\nE\n",
        f"cat <<E\n{lead}\nE\n{u}\n",
        f"cat <<E <<'F'\nx\nE\ny\nF\n{u}\n",
        f"bash <<< '{u}'\n",
        f"bash <<< '{lead}'\n",
        f"cat <<E\n{u}\n",
        f"bash <<'E'\n{u} # {b}\nE\n",
        f"bash <<< '{u} # {b}'\n",
        f"bash -c '{u} # {b}'\n",
        f"bash -c '{u}; echo {b}'\n",
        f"bash -c '{u} \\\n# {b}'\n",
        f"bash -c '{lead} # {u}'\n",
        f"eval '{u} # {b}'\n",
        "bash -c \"cargo \\$'mutants' --in-place\"\n",
        # An expansion in an expanded here-document, glued to a `#` that the shell reading the
        # document then reads as part of a word.
        f"bash <<E\n$(echo x)#; {u}\nE\n",
    ]
    ends = [";", "&&", "||", "|", "&", "\n", "|&", " ;", "; "]
    axes["operators"] = [f"{u}{end} echo {b}\n" for end in ends] + [
        f"echo {context.replace('@', inner)} {b}\n"
        for context in ("$(@)", "`@`", "<(@)", ">(@)", '"$(@)"', "${X:-$(@)}", "$( (@) )")
        for inner in (u, f"{u}; echo", f"{u} &&", f"{lead}; {u}")
    ]
    axes["redirections"] = [
        f"cargo mutants {TIMEOUT}>x {BUILD}\n",
        f"cargo mutants {TIMEOUT} 2>/dev/null {BUILD}\n",
        f"cargo mutants {TIMEOUT} {{fd}}>/dev/null {BUILD}\n",
        f"cargo mutants {TIMEOUT} >x {BUILD_SECONDS}\n",
        f"cargo mutants {BOUNDS}>/dev/null\n",
        f"cargo mutants {BOUNDS} 2>&1\n",
        f"cargo mutants {TIMEOUT} {BUILD_FLAG} <<<{BUILD_SECONDS}\n",
        f">x {lead}\n",
        f"{u} > >(echo {b})\n",
    ]
    wrapped = []
    for inner in (u, lead):
        wrapped += [
            f"{program} {inner}\n"
            for program in ("env", "env X=1", "command", "exec", "nice", "timeout 60", "time")
        ]
        wrapped += [
            f"bash -c '{inner}'\n",
            f"sh -c '{inner}'\n",
            f"eval '{inner}'\n",
            f'eval "{inner}"\n',
            f"echo {inner.split(' ', 1)[1]} | xargs cargo\n",
            f"f() {{ {inner}; }}; f\n",
            f"if true; then {inner}; fi\n",
            f"while :; do {inner}; break; done\n",
            f"case x in x) {inner};; esac\n",
            f"{{ {inner}; }}\n",
            f"( {inner} )\n",
            f"X=$({inner})\n",
            f"true && {inner}\n",
            f"false || {inner}\n",
            f"! {inner}\n",
        ]
    # A program that runs cargo with words from its input: the subcommand decoded from it, an
    # argument put before the bounds, and the subcommand held in a variable the text assigns.
    wrapped += [
        "printf 'mu%s\\n' tants | xargs cargo\n",
        f"echo -- | xargs -I@ cargo mutants @ {b}\n",
        "M=mutants; echo $M | xargs -I@ cargo @ --in-place\n",
    ]
    axes["wrappers"] = wrapped
    axes["state"] = [
        f"set -o pipefail\n{lead}\n",
        f"set -euo pipefail\n{u}\n",
        f"set -k\n{lead}\n",
        f"set -o posix\n{u}\n",
        f'shopt -s expand_aliases\nalias m="{u}"\nm\n',
        f'builtin shopt -s expand_aliases\nbuiltin alias m="{u}"\nm\n',
        f"command set -H\n{lead}\n",
        "set -k\ncargo X=1 mutants --in-place\n",
        "set -o keyword\ncargo X=1 mutants --in-place\n",
        "command set -k\ncargo X=1 mutants --in-place\n",
        "shopt -s expand_aliases\nBASH_ALIASES[m]=cargo\nm mutants --in-place\n",
    ]
    axes["comments"] = [
        f"{u}{end}# {b}\n" for end in (";", "&", "|", "&&", "||", ";;", " ", "\t", "\n")
    ] + [f"{lead}{end}#; {u}\n" for end in (";", "&", " ", "\n")]
    axes["expressions"] = [
        f"{u} ${{{{ matrix.shard }}}}\n",
        f"cargo mutants --shard ${{{{ matrix.shard }}}}/2 {b}\n",
        f"echo ${{{{ matrix.shard }}}}\n{u}\n",
        f"{lead}\necho ${{{{ matrix.shard }}}}\n",
        f"echo ${{{{ inputs.bounds }}}}\n{lead}\n",
        "cargo mutants ${{ inputs.bounds }}\n",
        f"{lead}\necho ${{{{ inputs.bounds }}}}\n",
        f"cargo mutants {b} ${{{{ inputs.bounds }}}}\n",
        f"cargo mutants {TIMEOUT[:-1]}${{{{ matrix.shard }}}} {BUILD}\n",
    ]
    # A text handed on (to a shell, through a pipe or a here-document, or spelled inside an
    # expansion), each carrying the command in a spelling of its own.
    inner = [
        u,
        lead,
        f"{u} # {b}",
        f"{lead}; {u}",
        "ca${Z}rgo mutants --in-place",
        "cargo mu${Z}tants --in-place",
        'ca""rgo mutants --in-place',
        "c\\argo mutants --in-place",
        "$'\\x63argo' $'\\x6dutants' --in-place",
        "C=cargo; $C mutants --in-place",
        "M=mutants; cargo $M --in-place",
        "X='cargo mutants --in-place'; $X",
        "{cargo,} mutants --in-place",
        "ca$(:)rgo mutants --in-place",
        "printf 'cargo %s --in-place\\n' mutants | bash",
        "bash -c 'cargo mutants --in-place'",
        "echo cargo mutants --in-place | bash",
        "${X:-cargo mutants --in-place}",
    ]
    carriers = (
        "bash -c {q}",
        "sh -c {q}",
        "bash <<< {q}",
        "env bash -c {q}",
        "echo {q} | bash",
        "bash <<'E'\n{t}\nE",
        "bash <<E\n{t}\nE",
        'bash -c "$(printf %s {q})"',
    )
    handed = [
        carrier.format(q="'" + text.replace("'", "'\\''") + "'", t=text) + "\n"
        for carrier in carriers
        for text in inner
    ]
    handed += [f"{text}\n" for text in inner[9:]] + [
        'bash -c "$""\'\\x63argo\' $""\'\\x6dutants\' --in-place"\n',
        "C=cargo; export C; cat <<E | bash\n\\$C mutants --in-place\nE\n",
        f"cat <<E | bash\n{u}\nE\n",
    ]
    # A text in a variable that the snippet assigns a literal, handed to the builtin that reads its
    # arguments as shell, or to a shell after `-c` with and without operands after it; the variable
    # quoted and not, spelled plainly and in braces.
    for text in inner:
        q = "'" + text.replace("'", "'\\''") + "'"
        for use in ('"$X"', "$X", '"${X}"', "${X}"):
            handed.append(f"X={q}; eval {use}\n")
            for shell in ("bash", "sh"):
                handed += [f"X={q}; {shell} -c {use}{tail}\n" for tail in ("", " x y")]
    axes["data"] = handed
    # Each unit bash's token recognition reads as one word (bash 5.2, parse.y, read_token_word),
    # holding text that ends a command or begins a comment for a reader that does not read the
    # unit, alone and glued to a word, where bash reads it: among a command's words, in the pattern
    # after `==`, `!=` and `=` in `[[ ]]`, and as an array's element; then the command ends, with
    # and without a `#` glued to what bash reads as the unit's end.
    units = ["$[ ]", "$(( ))", "$( )", "${ }", "<( )", ">( )", "[ ]", "` `"]
    units += ["' '", '" "', "$' '", '$" "'] + [f"{c}( )" for c in "@*+?!"]
    places = ("false && echo @", "[[ x == @ ]]", "[[ x != @ ]]", "[[ x = @ ]]", "a=(@)", "a=(x @)")
    axes["units"] = [
        place.replace("@", glue + unit.replace(" ", inner)) + f"{tail}; {u}\n"
        for unit in units
        for inner in (";# ", ")# ", " # ")
        for place in places
        for glue in ("", "x")
        for tail in ("", "#")
    ]
    return axes


def yaml_double(text, hashes=False, breaks=False):
    """`text` as one YAML double-quoted scalar: `\\`, `"`, a tab and a line break escaped, a `#` as
    `\\x23` when `hashes`, and each escaped line break followed by a real, escaped one when
    `breaks` (YAML drops an escaped break and the next line's leading blanks)."""
    out = []
    for c in text:
        if c in '\\"':
            out.append("\\" + c)
        elif c == "\n":
            out.append("\\n\\\n          " if breaks else "\\n")
        elif c == "\t":
            out.append("\\t")
        elif c == "#" and hashes:
            out.append("\\x23")
        else:
            out.append(c)
    return '"' + "".join(out) + '"'


def yaml_spellings(text):
    """[(a spelling of a `run:` value, the text YAML reads from it)]: `text` itself, or `text`
    without its final line break where the form cannot end in one (single-quoted and plain)."""
    body, tail = text.rstrip("\n"), len(text) - len(text.rstrip("\n"))
    lines = body.split("\n")
    spellings = [(yaml_double(text), text), (yaml_double(text, hashes=True), text)]
    if not any(line[:1] in " \t" for line in text.split("\n")[1:]):
        spellings.append((yaml_double(text, breaks=True), text))
    if not body:
        return spellings
    chomp = {0: "-", 1: ""}.get(tail, "+")
    indicator = "2" if body[:1] in " \t\n" else ""
    block = "".join(f"          {line}\n" if line else "\n" for line in lines)
    spellings.append((f"|{indicator}{chomp}\n{block}" + "\n" * max(tail - 1, 0), text))
    if not any(line[:1] in " \t" for line in lines) and body[:1] != "\n":
        for join in (False, True):
            out = []
            for line in lines:
                parts = re.split(r"(?<=\S) (?=\S)", line) if join else [line]
                out.append("".join(f"          {part}\n" for part in parts) if line else "")
            spellings.append((f">{chomp}\n" + "\n".join(out) + "\n" * max(tail - 1, 0), text))
    if tail <= 1 and "\n\n" not in body:
        if not any(line[:1] in " \t" or line[-1:] in " \t" for line in lines):
            spellings.append(("'" + "\n\n          ".join(lines).replace("'", "''") + "'", body))
        if (
            body[:1].isalnum()
            and not re.search(r"[:#]\s|\s#|:$|[ \t]$|[ \t]\n|\n[ \t#]", body)
            and not re.search(r"(?m)^[-?:,\[\]{}#&*!|>'\"%@`]", body)
        ):
            plain = "\n\n          ".join(lines)
            spellings += [(plain, body), (f"{plain} # {BOUNDS}", body)]
    return spellings


def grammar_workflow(value, matrix=False, shell=None):
    """A workflow whose one step runs `value` (a spelling of a `run:` value)."""
    strategy = "    strategy:\n      matrix:\n        shard: [0, 1]\n" if matrix else ""
    shell = f"        shell: {shell}\n" if shell else ""
    end = "" if value.endswith("\n") else "\n"
    return (
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n{strategy}    steps:\n"
        f"      - run: {value}{end}{shell}"
    )


def grammar_members():
    """[(axis, a workflow, the texts bash runs from it, or None where the text is not stated)];
    the first two are the controls, an unbounded command and a bounded one in a literal block."""
    members = []
    for axis, texts in grammar_texts().items():
        for text in texts:
            spellings = yaml_spellings(text)
            if axis in ("control", "data", "units"):
                spellings = [(value, read) for value, read in spellings if value[:1] == "|"]
            for value, read in spellings:
                scripts = [read.replace("${{ matrix.shard }}", v) for v in ("0", "1")]
                stated = "inputs." not in read
                workflow = grammar_workflow(value, "matrix." in read)
                members.append((axis, workflow, list(dict.fromkeys(scripts)) if stated else None))
    # YAML's node forms around a `run:` value: a quoted or spaced key, a flow mapping, an anchor, an
    # alias, a tag, a complex key, a value on the next line, a document marker, a later key, two
    # steps, a folded block's more-indented line, and a `shell:` other than bash.
    u, lead = UNBOUNDED, LEAD
    folded = f"cargo mutants\n  --in-place\n{BOUNDS}\n"
    structure = [
        f'jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - "run": {u}\n',
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run : {u}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - {{run: {u}}}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: &c {u}\n",
        f"x: &c {u}\njobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: *c\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: !!str {u}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - ? run\n        : {u}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run:\n          {u}\n",
        f"---\njobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: {u}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - name: x\n        run: {u}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: {lead}\n"
        f"      - run: {u}\n",
        f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: >\n"
        f"          cargo mutants\n            --in-place\n          {BOUNDS}\n",
    ]
    for n, workflow in enumerate(structure):
        texts = (
            [lead, u] if n == len(structure) - 2 else [folded] if n == len(structure) - 1 else [u]
        )
        members.append(("structure", workflow, texts))
    for shell in ("sh", "bash {0}", "bash -e {0}"):
        members.append(("structure", grammar_workflow(u, shell=shell), [u]))
    # Another shell reads the text by its own grammar, which the oracle does not run: not stated.
    members.append(("structure", grammar_workflow(lead, shell="sh"), None))
    # A matrix `include` adds a value that the key's own list does not hold.
    included = grammar_workflow(f"cargo mutants {TIMEOUT[:-1]}${{{{ matrix.shard }}}} {BUILD}")
    included = included.replace(
        "    steps:",
        "    strategy:\n      matrix:\n        shard: [0]\n"
        "        include:\n          - shard: 1\n    steps:",
    )
    texts = [f"cargo mutants {TIMEOUT[:-1]}{v} {BUILD}\n" for v in "01"]
    members.append(("structure", included, texts))
    return members


# The oracle runs each text as a script, under a timeout, twice (with the stub's status 0 and 1, so
# an `||` or an `&&` takes each branch), with a path that holds only the stubs: `cargo` and
# `cargo-mutants` log their words, each call to a file of its own process, and the programs that
# run another program pass it on.
GRAMMAR_ORACLE = r"""
while IFS= read -r f; do
  for ORACLE_STATUS in 0 1; do
    export ORACLE_TAG="${f##*/}" ORACLE_STATUS RUNNER_TEMP="$f.d"
    { cd "$f.d" && "$TIMEOUT" -k 1 10 "$SHELL_UNDER_TEST" --noprofile --norc "$f" </dev/null >/dev/null 2>&1; } \
      9>&1 | { while read -r _; do :; done; }
    printf '%s\036' "$ORACLE_TAG:$ORACLE_STATUS" >> "$DONE"
  done
done < "$LIST"
"""
GRAMMAR_STUB = r"""#!{shell}
r="${{ORACLE_TAG:-?}}:${{ORACLE_STATUS:-?}}"$'\037'"${{0##*/}}"
for a in "$@"; do r+=$'\037'"$a"; done
printf '%s\036' "$r" >> {log}/$$
exit "${{ORACLE_STATUS:-0}}"
"""
# The oracle's pythons run as given, except that a file named `memory_scope.py` runs with the seams
# of its run() planted, so the scope is in force here and the wrapper runs its command as it would
# on the runner. Each script runs in its own directory, beside its own copy of the tree's wrapper.
MEMORY_SCOPE = REPO / "scripts" / "memory_scope.py"
ORACLE_PYTHONS = ("python3", "python", "python3.12")
WRAPPER_PLANT = r'''
import importlib.util
import os
from pathlib import Path

EVENTS = "low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\noom_group_kill 0\n"


def plant_wrapper(script, root):
    """The wrapper loaded from `script`, with its run() seams, and nothing else, pointed at a
    machine under `root` whose scope holds this process with the cap in force."""
    spec = importlib.util.spec_from_file_location("memory_scope", script)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    root = Path(root) / str(os.getpid())
    unit = f"memory-scope-{os.getpid()}.scope"
    total, page = 16777216, 4096
    group = root / "cgroup" / "system.slice" / unit
    group.mkdir(parents=True, exist_ok=True)
    (root / "meminfo").write_text(f"MemTotal: {total} kB\n", "utf-8")
    (root / "proc-cgroup").write_text(f"0::/system.slice/{unit}\n", "utf-8")
    cap = total * 1024 * 15 // 16 // page * page
    files = {"max": f"{cap}\n", "swap.max": "0\n", "oom.group": "0\n", "events": EVENTS}
    for name, text in {**files, "peak": "0\n"}.items():
        (group / f"memory.{name}").write_text(text, "utf-8")
    for name, line in (("sudo", "exit 0"), ("systemctl", "echo continue")):
        (root / name).write_text(f"#!/bin/sh\n{line}\n", "utf-8")
        (root / name).chmod(0o700)
    module.run.__kwdefaults__.update(
        meminfo=root / "meminfo",
        page=page,
        sudo=(str(root / "sudo"),),
        systemctl=(str(root / "systemctl"),),
        proc_cgroup=root / "proc-cgroup",
        cgroup_root=root / "cgroup",
        reads=1,
    )
    return module
'''
WRAPPER_PYTHON = (
    WRAPPER_PLANT
    + r"""
import re
import sys

machine, *words = sys.argv[1:]
k = 0
while k < len(words) and re.fullmatch(r"-[bBdEiIOPqsSuvx]+|-[WX].*", words[k]):
    k += 2 if words[k] in ("-W", "-X") else 1
script = Path(words[k]) if k < len(words) else None
if script and script.name == "memory_scope.py" and script.is_file():
    try:
        wrapper = plant_wrapper(script, machine)
    except Exception as failure:
        sys.exit(f"plant_wrapper failed: {failure!r}")
    sys.argv = [str(script), *words[k + 1 :]]
    sys.exit(wrapper.main())
os.execv(sys.executable, [sys.executable, *words])
"""
)


def wrapper_module(script):
    """The wrapper's module, loaded from its file by path."""
    spec = importlib.util.spec_from_file_location("memory_scope", script)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def declared_options(module):
    """The options the module's own parser declares that take exactly one value."""
    return frozenset(
        option
        for action in module.build_parser()._actions
        if type(action) is argparse._StoreAction and action.nargs is None
        for option in action.option_strings
    )


def cargo_mutants_arguments(program, args):
    """The arguments cargo-mutants reads when this call runs it (up to a `--`), else None."""
    if program == "cargo-mutants":
        rest = args[1:] if args[:1] == ["mutants"] else None
    else:
        k = 0
        while k < len(args) and args[k][:1] in "+-" and args[k] != "--":
            k += 2 if args[k] in ("--config", "--color", "-C", "-Z", "--explain") else 1
        rest = args[k + 1 :] if args[k : k + 1] == ["mutants"] else None
    return None if rest is None else rest[: rest.index("--")] if "--" in rest else rest


def bash_runs(scripts):
    """(the indices of the scripts in which bash runs cargo mutants without the gate's bounds,
    those in which it runs it with them); it refuses to run a script that names a network device,
    and it fails unless every script ran twice and the two controls read as they must."""
    shell, bounds = shutil.which("bash"), BOUNDS.split()
    assert shell and shutil.which("timeout"), "the oracle needs bash and timeout"
    assert not any("/dev/tcp" in s or "/dev/udp" in s for s in scripts), "a network device"
    workers = min(8, os.cpu_count() or 1)
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        stubs = root / "stubs"
        stubs.mkdir()
        log = root / "log"
        log.mkdir()
        for name in ("cargo", "cargo-mutants"):
            (stubs / name).write_text(GRAMMAR_STUB.format(shell=shell, log=log), encoding="utf-8")
        for name in GRAMMAR_WRAPPERS + ("bash",):
            real = shutil.which("dash" if name == "sh" else name)
            if real:
                (stubs / name).write_text(f'#!{shell}\nexec {real} "$@"\n', encoding="utf-8")
        (root / "python.py").write_text(WRAPPER_PYTHON, encoding="utf-8")
        for name in ORACLE_PYTHONS:
            (stubs / name).write_text(
                f'#!{shell}\nexec {sys.executable} -I {root / "python.py"} {root / "machine"} "$@"\n',
                encoding="utf-8",
            )
        for stub in stubs.iterdir():
            stub.chmod(0o700)
        (root / "oracle.sh").write_text(GRAMMAR_ORACLE, encoding="utf-8")
        wrapper = MEMORY_SCOPE.read_bytes()
        for n, text in enumerate(scripts):
            (root / f"m{n}").write_text(text, encoding="utf-8")
            (root / f"m{n}.d").mkdir()
            if "python" in text:
                (root / f"m{n}.d" / "scripts").mkdir()
                (root / f"m{n}.d" / "scripts" / "memory_scope.py").write_bytes(wrapper)
        runs = []
        for w in range(workers):
            names = "".join(f"{root / f'm{n}'}\n" for n in range(w, len(scripts), workers))
            (root / f"list{w}").write_text(names, encoding="utf-8")
            env = {
                "PATH": str(stubs),
                "SHELL_UNDER_TEST": shell,
                "TIMEOUT": shutil.which("timeout"),
                "LIST": str(root / f"list{w}"),
                "DONE": str(root / f"done{w}"),
            }
            runs.append(
                subprocess.Popen([shell, "--noprofile", "--norc", "oracle.sh"], cwd=root, env=env)
            )
        for run in runs:
            assert run.wait(timeout=900) == 0, "the oracle's own shell failed"
        done = [
            d
            for w in range(workers)
            if (root / f"done{w}").exists()
            for d in (root / f"done{w}").read_text(encoding="utf-8").split("\x1e")[:-1]
        ]
        assert len(done) == 2 * len(scripts) == len(set(done)), "a script did not run twice"
        records = [
            record
            for path in sorted(log.iterdir())
            for record in path.read_text(encoding="utf-8").split("\x1e")[:-1]
        ]
        unbounded, bounded = set(), set()
        for record in records:
            tag, program, *args = record.split("\x1f")
            assert re.fullmatch(r"m[0-9]+:[01]", tag), f"a call no script owns: {record!r}"
            rest = cargo_mutants_arguments(program, args)
            if rest is not None:
                n = int(tag[1:].split(":")[0])
                hit = any(rest[k : k + 4] == bounds for k in range(len(rest)))
                (bounded if hit else unbounded).add(n)
        assert 0 in unbounded and 1 in bounded and 1 not in unbounded, "a control misread"
        return unbounded, bounded


#: The settle census's measured need in seconds, the one literal SPEC-327 A1 holds the gate's
#: budget and the sizer's census term to, re-derived by SPEC-362 R6: the slowest census test passed
#: at 1378.452 s in push run 37392351782's `rust` job, and nextest started the census tests up to
#: 51 s into a leg's baseline run, so a run that holds them needs 51 + 1379 = 1430 s.
CENSUS_NEED_SECONDS = 1430


class TheGatesTimeoutCoversTheCensus(unittest.TestCase):
    """SPEC-327 A1 (R1, R3): the per-mutant budget covers the census's need with a 1.5 margin,
    and the sizer charges that need as its census term."""

    def test_the_gates_timeout_covers_the_census_with_its_margin(self):
        words = BOUNDS.split()
        timeout = int(words[words.index("--timeout") + 1])
        self.assertGreaterEqual(timeout, math.ceil(1.5 * CENSUS_NEED_SECONDS), BOUNDS)
        module = verdict_module()
        self.assertEqual(module.CENSUS_SECONDS["deck-streak-progression"], CENSUS_NEED_SECONDS)


class EveryMutationCommandKeepsTheGatesBounds(unittest.TestCase):
    """A6 (R5): every `cargo mutants` command line in every workflow carries the gate's bounds."""

    def test_a_command_spelled_with_a_toolchain_is_found(self):
        with plant_workflow("cargo +nightly mutants --in-place") as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {"planted.yml": ["cargo +nightly mutants --in-place"]})

    def test_the_cargo_mutants_binary_form_is_found(self):
        with plant_workflow("cargo-mutants mutants --in-place") as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {"planted.yml": ["cargo-mutants mutants --in-place"]})

    def test_a_second_command_on_one_line_is_its_own_command(self):
        with plant_workflow(f"cargo mutants {BOUNDS} ; cargo mutants --in-place") as scratch:
            found = mutants_commands(Path(scratch))
        lines = found.get("planted.yml", [])
        self.assertEqual(len(lines), 2, lines)
        self.assertRegex(lines[0], BOUNDED)
        self.assertNotRegex(lines[1], BOUNDED)

    def test_a_cargo_flag_with_a_separate_value_is_part_of_the_command(self):
        with plant_workflow("cargo --config net.retry=2 mutants --in-place") as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {"planted.yml": ["cargo --config net.retry=2 mutants --in-place"]})

    def test_a_comment_is_no_command_and_bounds_nothing(self):
        with plant_workflow(f"cargo mutants --in-place # {BOUNDS}") as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {"planted.yml": ["cargo mutants --in-place"]})
        # A `#` right after an operator (`;`, `&`, `)`) starts a word too, so it opens a comment.
        with plant_workflow(f"cargo mutants --in-place;# {BOUNDS}") as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {"planted.yml": ["cargo mutants --in-place"]})
        # A `)` that closes a substitution, or an operator inside `${ }`, starts no word, so a `#`
        # after it is text and the unbounded command after it is still read.
        for text in examined("in-word hashes", ["$(true)#", "<(true)#", "$((1))#", "${X//;#/}"]):
            with plant_workflow(f"cargo mutants {BOUNDS} {text}; cargo mutants --in-place") as s:
                found = mutants_commands(Path(s))
            self.assertEqual(found["planted.yml"][1:], ["cargo mutants --in-place"], text)
        with plant_workflow(f"echo planted # cargo mutants {BOUNDS}") as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {})
        # In a `run: |` block a `#` inside shell quotes is text, so the command after it is read.
        block = '|\n          echo "a # b" && cargo mutants --in-place'
        with plant_workflow(block) as scratch:
            found = mutants_commands(Path(scratch))
        self.assertEqual(found, {"planted.yml": ["cargo mutants --in-place"]})

    def test_every_unbounded_command_bash_runs_past_a_hash_is_found(self):
        members = examined("class members", class_members())
        self.assertEqual(len(members), CLASS_SIZE)
        runs = unbounded_by_bash(members)
        with tempfile.TemporaryDirectory() as scratch:
            for n, script in enumerate(members):
                block = "".join(f"          {line}\n" for line in script.splitlines())
                text = f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: |\n{block}"
                (Path(scratch) / f"m{n}.yml").write_text(text, encoding="utf-8")
            found = mutants_commands(Path(scratch))
        missed = [
            members[n]
            for n in sorted(runs)
            if all(BOUNDED.search(line) for line in found.get(f"m{n}.yml", []))
        ]
        self.assertEqual(missed[:1], [], f"{len(missed)} of {len(runs)} unbounded members pass")

    def test_a_valued_flag_spelling_after_a_bounded_command_is_its_own_command(self):
        with plant_workflow(f"cargo mutants {BOUNDS} && cargo -C crates mutants --in-place") as s:
            found = mutants_commands(Path(s))
        lines = found.get("planted.yml", [])
        self.assertEqual(len(lines), 2, lines)
        self.assertRegex(lines[0], BOUNDED)
        self.assertEqual(lines[1], "cargo -C crates mutants --in-place")

    def test_a_text_bash_computes_for_a_shell_to_read_is_refused_as_computed(self):
        # Drawn from the grammar's own members, each in a literal block: a text that bash computes
        # (a substitution, or a variable the snippet assigns) for a shell after `-c` or for the
        # builtin that reads its arguments as shell is refused for that alone, before the text is
        # read; and a literal text handed to either is read, and refused for the command it holds,
        # whatever its bounds.
        texts = grammar_texts()
        computed = [
            t
            for t in texts["data"]
            if LEAD in t and (t.startswith('bash -c "$(') or t.startswith(f"X='{LEAD}'; "))
        ]
        handed = re.compile(r"\S+(?: -c)? '" + re.escape(LEAD) + r"'\n")
        literal = [t for t in texts["wrappers"] if handed.fullmatch(t)]
        self.assertEqual(len(literal), 3, literal)
        chosen = computed + literal
        with tempfile.TemporaryDirectory() as scratch:
            for n, text in enumerate(chosen):
                value = next(v for v, _ in yaml_spellings(text) if v[:1] == "|")
                (Path(scratch) / f"m{n}.yml").write_text(grammar_workflow(value), encoding="utf-8")
            found = mutants_commands(Path(scratch))
        programs = set()
        for n in examined("computed texts", range(len(computed))):
            program = chosen[n].removeprefix(f"X='{LEAD}'; ").split()[0]
            programs.add(program)
            self.assertEqual(
                found.get(f"m{n}.yml"),
                [f"refused: a text bash computes for `{program}` to read as shell"],
                chosen[n],
            )
        self.assertEqual(sorted(programs), ["bash", "eval", "sh"], programs)
        for n in examined("literal texts", range(len(computed), len(chosen))):
            self.assertEqual(
                found.get(f"m{n}.yml"),
                ["refused: `cargo mutants` where another program decides its arguments"],
                chosen[n],
            )

    def test_the_reading_is_declared_what_it_reads_is_found_and_what_it_does_not_is_refused(self):
        # The declared reading, each text in a literal block. A bounded command after each word
        # after which bash runs the next word as the program (its reserved words, `!`, `time`, and
        # the builtins that run a command) is found, bounded, as bash runs it. A command that
        # changes how bash reads what follows, a continued line in an expanded here-document, and
        # an expression whose value the workflow does not state are refused for that.
        frames = (
            "if @; then :; fi",
            "if :; then @; fi",
            "if false; then :; else @; fi",
            "if false; then :; elif @; then :; fi",
            "for x in 1; do @; done",
            "while @; do break; done",
            "until @; do break; done",
            "{ @; }",
            "! @",
            "time @",
            "command @",
            "exec @",
            "builtin command @",
        )
        changes = (
            ("set", "set -k"),
            ("set", "set -H"),
            ("set", "set -o keyword"),
            ("set", "set -o posix"),
            ("set", "set -o histexpand"),
            ("shopt", "shopt -s extglob"),
            ("enable", "enable -n echo"),
            ("alias", "alias x=y"),
            ("set", "builtin set -k"),
            ("shopt", "command shopt -s extglob"),
        )
        found_texts = [frame.replace("@", LEAD) + "\n" for frame in frames]
        unbounded, bounded = bash_runs([f"{UNBOUNDED}\n", f"{LEAD}\n", *found_texts])
        refused = [
            (f"`{name}`, which changes how bash reads what follows", f"{change}\n{LEAD}\n")
            for name, change in changes
        ]
        refused += [
            ("a continued line in a here-document", f"cat <<E\nx\\\nE\n{LEAD}\n"),
            (
                "the expression ${{ inputs.x }}, whose value is not stated",
                "echo ${{ inputs.x }}\n" + f"{LEAD}\n",
            ),
        ]
        chosen = found_texts + [text for _, text in refused]
        with tempfile.TemporaryDirectory() as scratch:
            for n, text in enumerate(chosen):
                value = next(v for v, _ in yaml_spellings(text) if v[:1] == "|")
                (Path(scratch) / f"m{n}.yml").write_text(grammar_workflow(value), encoding="utf-8")
            found = mutants_commands(Path(scratch))
        for n in examined("leading words", range(len(frames))):
            self.assertTrue(n + 2 in bounded and n + 2 not in unbounded, found_texts[n])
            self.assertEqual(found.get(f"m{n}.yml"), [LEAD], found_texts[n])
        for k in examined("texts the reading does not read", range(len(refused))):
            n = len(frames) + k
            verdict = found.get(f"m{n}.yml") or [""]
            self.assertEqual(len(verdict), 1, chosen[n])
            self.assertTrue(
                verdict[0].startswith(f"refused: {refused[k][0]}"), (verdict, chosen[n])
            )

    def test_every_cargo_mutants_command_carries_the_gates_own_bounds(self):
        found = mutants_commands(WORKFLOWS)
        for name in ("ci.yml", "mutation-weekly.yml"):
            self.assertIn(name, found, f"{name} runs no cargo mutants command")
            self.assertGreaterEqual(len(found[name]), 1, name)
        self.assertEqual(len(found["mutation-weekly.yml"]), 6, "each package branch is a command")
        for name, lines in examined("workflow files", list(found.items())):
            for line in examined(f"{name} commands", lines):
                self.assertRegex(line, BOUNDED, f"{name}: {line}")

    def test_every_command_bash_runs_from_a_run_value_is_found_or_refused(self):
        members = examined("grammar members", grammar_members())
        self.assertEqual(len(members), GRAMMAR_SIZE)
        scripts = list(dict.fromkeys(s for _, _, texts in members for s in texts or ()))
        unbounded, _ = bash_runs(scripts)
        index = {script: n for n, script in enumerate(scripts)}
        with tempfile.TemporaryDirectory() as scratch:
            for n, (_, text, _) in enumerate(members):
                (Path(scratch) / f"m{n}.yml").write_text(text, encoding="utf-8")
            found = mutants_commands(Path(scratch))
        self.assertEqual(found.get("m1.yml"), [LEAD], "the guard reads the bounded control")
        runs = [
            n
            for n, (_, _, texts) in enumerate(members)
            if texts is None or any(index[s] in unbounded for s in texts)
        ]
        missed = [
            members[n][1]
            for n in runs
            if all(BOUNDED.search(line) for line in found.get(f"m{n}.yml", []))
        ]
        self.assertEqual(missed[:1], [], f"{len(missed)} of {len(runs)} unbounded members pass")


#: The head's two package-bearing commands, verbatim: the literal the pin measures the rewrite against.
HEAD_COMMANDS = {
    "size": (
        'cargo mutants --no-shuffle --list --json --in-place ${PACKAGE:+--package "$PACKAGE"} '
        '--timeout 2200 --build-timeout 600 > "$RUNNER_TEMP/size/package.json"'
    ),
    "rust": (
        'cargo mutants --no-shuffle -vV --in-place ${PACKAGE:+--package "$PACKAGE"} '
        '--sharding round-robin --shard "$SHARD/$SHARDS" --timeout 2200 --build-timeout 600 '
        '--output "$RUNNER_TEMP/mutation" || rc=$?'
    ),
}
#: Values a package input could hold that no workspace declares: each is one shell word to bash.
HOSTILE_PACKAGES = [
    "a b",
    "*",
    "a=b",
    "-",
    "--",
    "-x",
    "-p",
    "--package",
    "--scratch",
    "a\nb",
    'a"b',
    "a'b",
    "$(echo x)",
    "`echo x`",
]


def workspace_packages():
    """Every package name the repository's Cargo.toml files declare, read without cargo."""
    names = set()
    for manifest in sorted(REPO.rglob("Cargo.toml")):
        if "target" in manifest.relative_to(REPO).parts:
            continue
        declared = tomllib.loads(manifest.read_text(encoding="utf-8")).get("package", {})
        if "name" in declared:
            names.add(declared["name"])
    return sorted(names)


def rewritten_blocks(text):
    """The two if/else blocks of the weekly workflow, dedented, in file order (size, then rust)."""
    pattern = r'(?ms)^( +)if \[ -n "\$PACKAGE" \]; then\n.*?^\1fi\n'
    blocks = [m.group(0) for m in re.finditer(pattern, text)]
    return [textwrap.dedent(block) for block in blocks]


def argv_of(script, package):
    """The argv the stub cargo received when bash ran the script; PACKAGE unset when None. A wrapped
    command runs through the wrapper beside it, with its seams planted: never the machine's scope."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        (root / "bin").mkdir()
        (root / "size").mkdir()
        (root / "scripts").mkdir()
        (root / "scripts" / "memory_scope.py").write_bytes(MEMORY_SCOPE.read_bytes())
        (root / "python.py").write_text(WRAPPER_PYTHON, encoding="utf-8")
        for name in ORACLE_PYTHONS:
            python = root / "bin" / name
            python.write_text(
                f'#!/bin/bash\nexec {sys.executable} -I {root / "python.py"} {root / "machine"} "$@"\n',
                encoding="utf-8",
            )
            python.chmod(0o700)
        log = root / "argv.log"
        stub = root / "bin" / "cargo"
        stub.write_text('#!/bin/bash\nprintf \'%s\\0\' "$@" >> "$STUB_LOG"\n', encoding="utf-8")
        stub.chmod(0o700)
        file = root / "step.sh"
        file.write_text(script, encoding="utf-8")
        env = {
            "PATH": f"{root / 'bin'}:/usr/bin:/bin",
            "STUB_LOG": str(log),
            "RUNNER_TEMP": str(root),
            "SHARD": "3",
            "SHARDS": "8",
        }
        if package is not None:
            env["PACKAGE"] = package
        done = subprocess.run(
            ["bash", str(file)], env=env, capture_output=True, timeout=30, cwd=root
        )
        assert done.returncode == 0, (done.returncode, done.stderr)
        argv = log.read_bytes().replace(str(root).encode(), b"<tmp>").split(b"\0")[:-1]
        return argv


class TheWeeklySweepNamesItsPackageInLiteralWords(unittest.TestCase):
    """`--package=V` and `--package V` are one option to the parser cargo-mutants uses.

    clap's tutorial gives both spellings the one value (`--name=bob` and `--name bob`), and its
    `require_equals` documentation reads: "Requires that options use the `--option=val` syntax.
    Setting this requires that the option have an equals sign between it and the associated value."
    cargo-mutants declares `--package` as a plain `Vec<String>` option without `require_equals` or
    `allow_hyphen_values`, so a value that begins with `-` is an unknown argument in the spaced form
    and a package name that matches nothing in the equals form: neither selects a mutant.
    """

    def test_the_rewrite_hands_cargo_exactly_the_words_the_head_did(self):
        blocks = rewritten_blocks(workflow(WEEKLY))
        self.assertEqual(len(blocks), 2, "one block per package-bearing command")
        population = [None, ""] + workspace_packages() + HOSTILE_PACKAGES
        examined("package values", population)
        self.assertGreaterEqual(len(population), 2 + len(HOSTILE_PACKAGES) + 1)
        members = 0
        for step, block in zip(("size", "rust"), blocks):
            old = "rc=0\n" + HEAD_COMMANDS[step] + "\n"
            new = "rc=0\n" + block
            for value in population:
                before, after = argv_of(old, value), argv_of(new, value)
                members += 1
                if value in (None, ""):
                    self.assertEqual(before, after, f"{step}: {value!r}")
                    self.assertNotIn(b"--package", after)
                    continue
                at = before.index(b"--package")
                self.assertEqual(before[at + 1], value.encode(), f"{step}: {value!r}")
                expected = before[:at] + [b"--package=" + value.encode()] + before[at + 2 :]
                self.assertEqual(after, expected, f"{step}: {value!r}")
        examined("old-against-new argvs", range(members))
        self.assertEqual(members, 2 * len(population))

    def test_a_dash_led_value_selects_nothing_in_the_step_after_the_listing(self):
        with tempfile.TemporaryDirectory() as scratch:
            listing = Path(scratch) / "package.json"
            listing.write_text("[]", encoding="utf-8")
            dashed = [v for v in HOSTILE_PACKAGES if v.startswith("-")]
            for value in examined("dash-led values", dashed):
                done = subprocess.run(
                    [
                        sys.executable,
                        str(VERDICT),
                        "size",
                        "--package",
                        value,
                        "--listed",
                        str(listing),
                    ],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
                out = done.stdout + done.stderr
                first = (done.stderr.splitlines() or done.stdout.splitlines() or [""])[0]
                print(f"size --package {value!r}: exit {done.returncode}: {first}")
                self.assertTrue(
                    done.returncode == 2 or "0 listed mutant(s)" in out,
                    f"{value!r}: exit {done.returncode}: {out}",
                )

    def test_the_rewritten_blocks_are_the_only_package_words_in_the_two_cargo_commands(self):
        text = workflow(WEEKLY)
        commands = examined(
            "weekly cargo mutants commands", re.findall(r"cargo mutants [^\n]*", text)
        )
        self.assertNotIn("${PACKAGE:+--package", "\n".join(commands))
        self.assertTrue(any("--package=" in command for command in commands))


# R5 (#465's remainder, closed by refusal): a word before the bounds that bash can expand to exactly
# `--` ends cargo's options there, so the bounds after it belong to the test tool. Its members are
# generated: each expansion form the reading reads, in a word of one to three segments (an expansion,
# a literal before it or after it, a literal between two), each segment quoted or not, over the
# literal contents none, `-`, `--`, other. The oracle is bash: a member can be `--` where it runs
# cargo mutants with the bounds after a `--` (`X`, `$1` and `$@` hold `--` in the preamble).
R5_EXPANSIONS = (
    "$X",
    "${X}",
    "${X:-y}",
    "${X:+--}",
    "$@",
    "$*",
    "$1",
    "$(echo --)",
    "`echo --`",
    "$((1))",
)
R5_LITERALS = ("-", "--", "a", "-a")
R5_PREAMBLE = "X=--; set -- --; "
R5_SIZE = 980


def r5_quote(kind, text, style):
    """One segment as written: an expansion in no quotes or double; a literal in none, single or double."""
    return {"none": text, "double": f'"{text}"', "single": f"'{text}'"}[style]


def r5_members():
    """[(segments, word, script)]: every word shape over every expansion, quoting and literal."""
    shapes = (
        [("E",)]
        + [("l", "E", lit) for lit in R5_LITERALS]
        + [("E", "l", lit) for lit in R5_LITERALS]
    )
    shapes += [("E", "l", "E", lit) for lit in R5_LITERALS]
    members = []
    for expansion in R5_EXPANSIONS:
        for shape in shapes:
            kinds = [k for k in shape if k in ("E", "l")]
            literal = shape[-1] if shape[-1] not in ("E", "l") else ""
            plan = [(k, expansion if k == "E" else literal) for k in kinds]
            for styles in itertools.product(
                *[("none", "double") if k == "E" else ("none", "single", "double") for k, _ in plan]
            ):
                word = "".join(r5_quote(k, t, st) for (k, t), st in zip(plan, styles))
                script = f"{R5_PREAMBLE}cargo mutants --in-place {word} {BOUNDS}\n"
                members.append(
                    (tuple(zip((k for k, _ in plan), (t for _, t in plan), styles)), word, script)
                )
    return list(dict.fromkeys(members))


def r5_refused_by_rule(segments):
    """The rule as specified: (i) an unquoted expansion, or (ii) an expansion and no literal
    character besides `-`."""
    unquoted_expansion = any(k == "E" and style == "none" for k, _, style in segments)
    other_literal = any(c != "-" for k, t, _ in segments if k == "l" for c in t)
    return unquoted_expansion or not other_literal


def r5_workflow(script):
    body = "".join(f"          {line}\n" for line in script.split("\n")[:-1])
    return f"jobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: |\n{body}"


class AComputedWordBeforeTheBoundsIsRefused(unittest.TestCase):
    def outcomes(self, scripts):
        """{index: what the guard says of that script, as its workflow}: the found lines."""
        verdicts = {}
        with tempfile.TemporaryDirectory() as scratch:
            for n, script in enumerate(scripts):
                (Path(scratch) / f"m{n}.yml").write_text(r5_workflow(script), encoding="utf-8")
            found = mutants_commands(Path(scratch))
        for n in range(len(scripts)):
            verdicts[n] = found.get(f"m{n}.yml", [])
        return verdicts

    def test_every_word_bash_can_expand_to_dashes_before_the_bounds_is_refused(self):
        members = examined("R5 members", r5_members())
        self.assertEqual(len(members), R5_SIZE)
        scripts = [UNBOUNDED + "\n", f"{LEAD}\n"] + [script for _, _, script in members]
        unbounded, _ = bash_runs(scripts)
        runs = sorted(n - 2 for n in unbounded if n >= 2)
        examined("R5 members bash runs without the bounds", runs)
        verdicts = self.outcomes([script for _, _, script in members])
        escaped = [members[n][2] for n in runs if all(BOUNDED.search(line) for line in verdicts[n])]
        self.assertEqual(
            escaped[:1], [], f"{len(escaped)} of {len(runs)} members that lose the bounds pass"
        )

    def test_the_rule_refuses_exactly_an_unquoted_expansion_or_one_with_no_literal_but_dash(self):
        members = examined("R5 members", r5_members())
        verdicts = self.outcomes([script for _, _, script in members])
        wrong = [
            (word, r5_refused_by_rule(segments), verdicts[n])
            for n, (segments, word, _) in enumerate(members)
            if r5_refused_by_rule(segments)
            != any(line.startswith("refused") for line in verdicts[n])
        ]
        self.assertEqual(
            wrong[:1], [], f"{len(wrong)} of {len(members)} members differ from the rule"
        )
        rules = [r5_refused_by_rule(segments) for segments, _, _ in members]
        self.assertTrue(any(rules) and not all(rules), "the rule both refuses and accepts members")

    def test_the_designs_three_members_are_refused(self):
        scripts = [
            f"X=--; cargo mutants $X {BOUNDS}\n",
            f'X=--; cargo mutants "$X" {BOUNDS}\n',
            f'set -- --; cargo mutants "$@" {BOUNDS}\n',
        ]
        verdicts = self.outcomes(examined("named R5 members", scripts))
        self.assertEqual(
            [any(line.startswith("refused") for line in verdicts[n]) for n in range(3)],
            [True, True, True],
        )

    def test_the_real_tree_commands_are_found_bounded_and_not_refused(self):
        found = mutants_commands(WORKFLOWS)
        lines = examined("weekly commands", found["mutation-weekly.yml"])
        self.assertEqual(len(lines), 6)
        examined("real-tree commands", [line for ls in found.values() for line in ls])
        for name, commands in found.items():
            for line in commands:
                self.assertFalse(line.startswith("refused"), f"{name}: {line}")
                self.assertRegex(line, BOUNDED, f"{name}: {line}")


SPY = """#!{shell}
{{ printf '%s\\0' "${{0##*/}}"; for a in "$@"; do printf '%s\\0' "$a"; done; }} > "$SPY_LOG"
"""
SPY_WORDS = ("--", "-", "-x", "--report", "--report=x", "-h", "", " ", "a b", "\n", "é", "cargo")
SPY_WORDS += (TIMEOUT_FLAG, TIMEOUT_SECONDS)
SPY_VALUES = ("r", "", " ", "a b", "é", "-1", "--", "-x", "--report", "--report=r")
SPY_SIZE = 374


def spy_cases(options):
    """[(the wrapper's argv, the words its command must be, whether it must run)]: the declared form,
    for each option the parser declares, each value in both spellings and twice, before commands of
    dash-led words, `--` again, empty words, blanks, a newline and non-ASCII. A dash-led value may
    instead end at the parser's usage error; the value `r` must run."""
    commands = [["spy", *rest] for n in range(3) for rest in itertools.product(SPY_WORDS, repeat=n)]
    commands += [[program, "a"] for program in ("--", "-x", "--report")]
    cases = [([min(options), "r", "--", *command], command, True) for command in commands]
    for option in sorted(options):
        for value in SPY_VALUES:
            for spelled in ([option, value], [f"{option}={value}"]):
                runs = value == "r"
                cases += [([*spelled, "--", *c], c, runs) for c in commands[:4] + commands[-3:]]
                cases.append(([*spelled, option, "r", "--", "spy", "a"], ["spy", "a"], runs))
    return cases


def spy_runs(script, cases):
    """[(the words the spy received, or None when nothing ran; the exit status)] for each case, the
    wrapper's own main() run from `script` in this process, with its run() seams planted."""
    namespace = {}
    exec(WRAPPER_PLANT, namespace)
    results = []
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        module = namespace["plant_wrapper"](script, root / "machine")
        (root / "spies").mkdir()
        (root / "cwd").mkdir()
        for name in ("spy", "--", "-x", "--report"):
            (root / "spies" / name).write_text(SPY.format(shell=shutil.which("bash")), "utf-8")
            (root / "spies" / name).chmod(0o700)
        with (
            mock.patch.dict(os.environ, {"PATH": str(root / "spies")}),
            contextlib.chdir(root / "cwd"),
        ):
            for n, (argv, *_) in enumerate(cases):
                log = root / f"log{n}"
                os.environ["SPY_LOG"] = str(log)
                with (
                    contextlib.redirect_stdout(io.StringIO()),
                    contextlib.redirect_stderr(io.StringIO()),
                ):
                    try:
                        status = module.main(list(argv))
                    except SystemExit as stop:
                        status = stop.code
                ran = log.read_bytes().split(b"\0")[:-1] if log.exists() else None
                results.append(([os.fsdecode(w) for w in ran] if ran is not None else None, status))
    return results


def spy_wrong(script, cases):
    """The cases whose command the wrapper did not run as exactly the words after its `--`, where
    a case that need not run may end at the parser's usage error (status 2) instead."""
    return [
        (argv, ran, status)
        for (argv, words, runs), (ran, status) in zip(cases, spy_runs(script, cases))
        if ran != words and (runs or ran is not None or status != 2)
    ]


WRAPPER_FORMS = {
    "exact": ("", "python3 scripts/memory_scope.py --report r -- "),
    "joined": ("", "python3 scripts/memory_scope.py --report=r -- "),
    "quoted value": ("out=r; ", 'python3 scripts/memory_scope.py --report "$out" -- '),
    "braced value": ("out=r; ", 'python3 scripts/memory_scope.py --report="${out}/x" -- '),
    "unquoted value": ("out=r; ", "python3 scripts/memory_scope.py --report $out -- "),
    "positional value": ("set -- r; ", 'python3 scripts/memory_scope.py --report "$@" -- '),
    "value --": ("", "python3 scripts/memory_scope.py --report -- -- "),
    "no separator": ("", "python3 scripts/memory_scope.py --report r "),
    "computed separator": ("S=--; ", "python3 scripts/memory_scope.py --report r $S "),
    "quoted computed separator": ("S=--; ", 'python3 scripts/memory_scope.py --report r "$S" '),
    "computed option": ("O=--report; ", "python3 scripts/memory_scope.py $O r -- "),
    "dot path": ("", "python3 ./scripts/memory_scope.py --report r -- "),
    "computed path": ("W=scripts/memory_scope.py; ", "python3 $W --report r -- "),
    "quoted computed path": ("W=scripts/memory_scope.py; ", 'python3 "$W" --report r -- '),
    "absolute path": ("", 'python3 "$PWD/scripts/memory_scope.py" --report r -- '),
    "python": ("", "python scripts/memory_scope.py --report r -- "),
    "python3.12": ("", "python3.12 scripts/memory_scope.py --report r -- "),
    "env python3": ("", "env python3 scripts/memory_scope.py --report r -- "),
    "python option": ("", "python3 -u scripts/memory_scope.py --report r -- "),
    "undeclared option": ("", "python3 scripts/memory_scope.py --report r -x -- "),
    "undeclared long option": ("", "python3 scripts/memory_scope.py --report r --cap 1 -- "),
    "abbreviation": ("", "python3 scripts/memory_scope.py --rep r -- "),
    "help": ("", "python3 scripts/memory_scope.py -h --report r -- "),
    "declared option after --": ("", "python3 scripts/memory_scope.py -- --report r "),
    "twice": ("", "python3 scripts/memory_scope.py --report r --report s -- "),
    "separator twice": ("", "python3 scripts/memory_scope.py --report r -- -- "),
    "nested": (
        "",
        "python3 scripts/memory_scope.py --report r -- "
        "python3 scripts/memory_scope.py --report s -- ",
    ),
    "exec": ("", "exec python3 scripts/memory_scope.py --report r -- "),
    "command": ("", "command python3 scripts/memory_scope.py --report r -- "),
}
# The forms the class reads through, around a bounded command: each is found, bounded.
WRAPPER_DECLARED = ("exact", "joined", "quoted value", "braced value", "twice", "nested")
WRAPPER_DECLARED += ("exec", "command")
WRAPPER_COMMANDS = {
    "bounded": ("", LEAD),
    "unbounded": ("", UNBOUNDED),
    "bounds after --": ("", f"cargo mutants -- {BOUNDS}"),
    "R5": ("X=--; ", f"cargo mutants $X {BOUNDS}"),
    "toolchain": ("", "cargo +stable mutants --in-place"),
    "shell text": ("", f"bash -c '{UNBOUNDED}'"),
    "bounded shell text": ("", f"bash -c '{LEAD}'"),
    "another program": ("", f"env {UNBOUNDED}"),
    "python text": ("", f"""python3 -c 'import os; os.system("{UNBOUNDED}")'"""),
    "no cargo": ("", "bash -c :"),
}
WRAPPER_CONTEXTS = ("@", "if true; then @; fi", "( @ )", "@ || rc=$?")
WRAPPER_SIZE = 1160


def wrapper_members():
    """[(form, command, context, script)]: each form of the wrapper's command line, around each
    command, in each context."""
    return [
        (form, command, context, pre + setup + context.replace("@", prefix + text) + "\n")
        for (form, (pre, prefix)), (command, (setup, text)), context in itertools.product(
            WRAPPER_FORMS.items(), WRAPPER_COMMANDS.items(), WRAPPER_CONTEXTS
        )
    ]


class TheMemoryScopeRunsTheWordsAfterItsSeparator(unittest.TestCase):
    def outcomes(self, scripts):
        """{index: the lines the guard finds in that script, as its workflow}."""
        with tempfile.TemporaryDirectory() as scratch:
            for n, script in enumerate(scripts):
                (Path(scratch) / f"m{n}.yml").write_text(r5_workflow(script), encoding="utf-8")
            found = mutants_commands(Path(scratch))
        return {n: found.get(f"m{n}.yml", []) for n in range(len(scripts))}

    def test_the_wrapper_runs_exactly_the_words_after_its_separator(self):
        options = examined(
            "options the wrapper declares", declared_options(wrapper_module(MEMORY_SCOPE))
        )
        cases = examined("wrapper argvs", spy_cases(options))
        self.assertEqual(len(cases), SPY_SIZE)
        wrong = spy_wrong(MEMORY_SCOPE, cases)
        self.assertEqual(wrong[:1], [], f"{len(wrong)} of {len(cases)} argvs")

    def test_a_wrapper_that_drops_or_adds_a_word_goes_red(self):
        source = MEMORY_SCOPE.read_text("utf-8")
        line = 'command = args.command[1:] if args.command[:1] == ["--"] else args.command'
        self.assertEqual(source.count(line), 1)
        whole = line.split(" = ", 1)[1]
        plants = {
            "drops the last word": f"command = ({whole})[:-1]",
            "drops a bound": f'command = [w for w in ({whole}) if w != "--timeout"]',
            "adds a word": f'command = [*({whole}), "--in-place"]',
        }
        cases = spy_cases(declared_options(wrapper_module(MEMORY_SCOPE)))
        cases = cases[:20] + [case for case in cases if "--timeout" in case[1]][:20]
        with tempfile.TemporaryDirectory() as scratch:
            for label, planted in examined("wrapper plants", plants.items()):
                path = Path(scratch) / label.replace(" ", "-") / "memory_scope.py"
                path.parent.mkdir()
                path.write_text(source.replace(line, planted), "utf-8")
                with self.subTest(plant=label):
                    self.assertNotEqual(spy_wrong(path, cases), [], label)

    def test_every_wrapped_command_bash_runs_without_the_bounds_is_found_or_refused(self):
        members = examined("wrapper-axis members", wrapper_members())
        self.assertEqual(len(members), WRAPPER_SIZE)
        scripts = [UNBOUNDED + "\n", f"{LEAD}\n"] + [script for *_, script in members]
        unbounded, _ = bash_runs(scripts)
        runs = examined(
            "wrapper-axis members bash runs without the bounds",
            sorted(n - 2 for n in unbounded if n >= 2),
        )
        verdicts = self.outcomes([script for *_, script in members])
        escaped = [members[n] for n in runs if all(BOUNDED.search(line) for line in verdicts[n])]
        self.assertEqual(
            escaped[:1], [], f"{len(escaped)} of {len(runs)} members that lose the bounds pass"
        )

    def test_the_declared_form_around_a_bounded_command_is_found_bounded(self):
        members = [
            (n, member)
            for n, member in enumerate(wrapper_members())
            if member[0] in WRAPPER_DECLARED and member[1] == "bounded"
        ]
        examined("declared-form members", members)
        self.assertEqual(len(members), len(WRAPPER_DECLARED) * len(WRAPPER_CONTEXTS))
        scripts = [UNBOUNDED + "\n", f"{LEAD}\n"] + [member[3] for _, member in members]
        unbounded, bounded = bash_runs(scripts)
        verdicts = self.outcomes([member[3] for _, member in members])
        wrong = [
            (member[:3], verdicts[k])
            for k, (_, member) in enumerate(members)
            if k + 2 in unbounded
            or k + 2 not in bounded
            or not verdicts[k]
            or any(line.startswith("refused") for line in verdicts[k])
            or not all(BOUNDED.search(line) for line in verdicts[k])
        ]
        self.assertEqual(wrong[:1], [], f"{len(wrong)} of {len(members)} members")


RAISING_PLANT = """
import os
from pathlib import Path


def plant_wrapper(script, root):
    raise RuntimeError("the plant could not be made")
"""
PLANT_THAT_WORKS = """
import os
from pathlib import Path


class Planted:
    def main(self):
        Path(os.environ["PLANT_MARKS"], "planted").write_text("ran", "utf-8")
        return 0


def plant_wrapper(script, root):
    return Planted()
"""
STAND_IN_SCRIPT = """
import os
from pathlib import Path

Path(os.environ["PLANT_MARKS"], "words").write_text("ran", "utf-8")
"""


class AStandInThatCannotPlantTheWrapperFailsClosed(unittest.TestCase):
    def run_stand_in(self, plant):
        """(exit status, standard error, marks left) of the stand-in with `plant` as its plant."""
        self.assertEqual(WRAPPER_PYTHON.count(WRAPPER_PLANT), 1)
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "marks").mkdir()
            (root / "stand_in.py").write_text(
                WRAPPER_PYTHON.replace(WRAPPER_PLANT, plant), encoding="utf-8"
            )
            (root / "memory_scope.py").write_text(STAND_IN_SCRIPT, encoding="utf-8")
            done = subprocess.run(
                [
                    sys.executable,
                    "-I",
                    str(root / "stand_in.py"),
                    str(root / "machine"),
                    str(root / "memory_scope.py"),
                    "--",
                    "echo",
                    "x",
                ],
                capture_output=True,
                text=True,
                env={"PLANT_MARKS": str(root / "marks"), "PATH": os.environ["PATH"]},
                timeout=60,
            )
            marks = sorted(path.name for path in (root / "marks").iterdir())
        return done.returncode, done.stderr, marks

    def test_a_failed_plant_exits_non_zero_names_the_failure_and_runs_no_words(self):
        status, stderr, marks = self.run_stand_in(RAISING_PLANT)
        self.assertNotEqual(status, 0)
        self.assertIn("the plant could not be made", stderr)
        self.assertEqual(marks, [], "the words ran after the plant failed")

    def test_a_plant_that_works_runs_the_wrapper_and_not_the_script(self):
        status, stderr, marks = self.run_stand_in(PLANT_THAT_WORKS)
        self.assertEqual((status, stderr), (0, ""))
        self.assertEqual(marks, ["planted"])


if __name__ == "__main__":
    unittest.main()

"""A package dispatch is sharded by its projected weight (SPEC-129 A1 to A6).

The sizing verb is run as the workflow runs it, on fixture listings written to a temporary
directory; the workflow is read as text with the helpers `test_mutation_workflows.py` uses. The
plants put the fixed 32 back into each place that must read the one count.
"""

import itertools
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import examined
from test_mutation_workflows import VERDICT, WEEKLY, WORKFLOWS, jobs, listed, shard, workflow

WHOLE = 32
COUNT = "${{ needs.size.outputs.shards }}"
BOUNDS = "--timeout 300 --build-timeout 600"
# A command is `cargo mutants` wherever it sits on its line, argument or none, with a toolchain or
# a cargo flag before the subcommand, or the `cargo-mutants mutants` binary itself; a second command
# on the same line is its own command. The bounds are matched whole, so a digit or a decimal
# appended to either value is not the gate's bound.
# A global flag that takes its value as the next word (`cargo -C dir mutants`) is part of the start.
VALUED = r"(?:--config|--color|-C|-Z)\s+\S+"
START = r"(?:\bcargo(?:\s+(?:" + VALUED + r"|[+-]\S+))*\s+mutants\b|\bcargo-mutants\s+mutants\b)"
COMMAND = re.compile(START + r"(?:(?!" + START + r")[^\n])*")
BOUNDED = re.compile(r"(?<![\w-])" + re.escape(BOUNDS) + r"(?![\w.])")


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
        # Costs and bound are the plan's: the daemon costs 80 s a mutant on a 371 s baseline, and
        # 40 mutants project to 3571 s of a 3600 s bound; the ingest costs 126 s.
        cases = [
            ("deck-streak-kernel", 5, 1),
            ("deck-streak-daemon", 40, 1),
            ("deck-streak-daemon", 41, 2),
            ("deck-streak-ingest", 60, 3),
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
        code, out, written = size(entries("deck-streak-ingest", 7000), "deck-streak-ingest")
        self.assertEqual(code, 1, out)
        self.assertIn("REFUSED", out)
        self.assertIn("7000 mutant(s), projected at 882000 s serially", out)
        self.assertEqual(written, {}, "a refused sizing wrote outputs")

    def test_a_package_that_lists_nothing_or_a_non_listing_is_not_sized(self):
        code, out, written = size([], "deck-streak-ingest")
        self.assertEqual((code, written.get("shards")), (0, "1"), out)
        code, out, written = size(None, "deck-streak-ingest", raw="not json")
        self.assertEqual(code, 3, out)
        self.assertIn("VOID", out)
        self.assertEqual(written, {}, out)


class TheWholeTreeKeepsThirtyTwo(unittest.TestCase):
    """A2 (R3): a scheduled run and a dispatch with no package read no listing and keep 32."""

    def test_no_package_is_thirty_two_shards_whatever_the_listing(self):
        for raw in examined("listings", [None, "not json", json.dumps(entries("a", 3))]):
            code, out, written = size(None, None, raw=raw or "")
            self.assertEqual(code, 0, out)
            self.assertEqual(written.get("shards"), str(WHOLE), out)
            self.assertEqual(json.loads(written["matrix"]), list(range(WHOLE)), out)


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
        command = re.search(r"cargo mutants [^\n]*", size_job)
        self.assertIsNotNone(command, "the size job lists nothing")
        for flag in (
            "--no-shuffle",
            "--list",
            "--json",
            "--in-place",
            "--timeout 300",
            "--build-timeout 600",
            '${PACKAGE:+--package "$PACKAGE"}',
        ):
            self.assertIn(flag, command.group(0))
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


# A `#` after one of these begins a word, so bash may read it as a comment.
BOUNDARY = " \t;&|()<>`"
# A here-document's body is no shell: bash starts no comment in it, and runs its substitutions.
HEREDOC = re.compile(r"(?<!<)<<(?!<)")


def opens(line, i, word):
    """Bash reads on from `line[i]` in a mode this reader does not model: a substitution, an
    expansion, an ANSI-C string, a backquote, an arithmetic or array parenthesis (a `(` glued to
    what comes before it), a subscript or `[[`."""
    char = line[i]
    return (
        char == "`"
        or line.startswith(("$(", "${", "$[", "$'"), i)
        or (char == "(" and i > 0 and line[i - 1] not in " \t;&|")
        or (char == "[" and (word or line.startswith("[[", i)))
    )


def uncommented(text):
    """`text` with each comment cut, in YAML and in the shell alike, wherever this reader can know
    that bash starts one. Bash starts a comment at a `#` that begins a word outside every quote,
    and a `#` inside a word or a quote is text. The reader models quotes, blanks and the operators,
    and trusts that model on a line only up to the first place bash reads in a mode it lacks (see
    `opens`), and not at all on a continued line or after a here-document. Before that place a `#`
    that begins a word cuts the rest of the line: its text is no command, and bounds written in it
    bound nothing. After it a `#` glued to a word is text, and any other `#` ends the command before
    it and hides nothing, so a command after it is still read. So a `#` after the `)` that closes a
    substitution, or after an operator inside `${ }`, is no comment, and the reader can refuse a
    line bash would pass but never pass a line whose unbounded command bash runs."""
    kept, continued, heredoc = [], False, False
    for line in text.split("\n"):
        pieces, piece, i, cut = [], 0, 0, False
        sure, quote, word = not (continued or heredoc), None, False
        while i < len(line):
            char = line[i]
            if char == "\\" and not (sure and quote == "'"):
                i, word = i + 2, True
                continue
            if sure and quote == "'":
                quote = None if char == "'" else quote
            elif sure and quote == '"':
                if char == '"':
                    quote = None
                elif char == "`" or line.startswith(("$(", "${", "$["), i):
                    sure, quote = False, None
            elif sure and char == "#" and not word:
                cut = True
                break
            elif sure and char in "'\"":
                quote = char
            elif sure:
                sure = not opens(line, i, word)
            elif char == "#" and not word:
                pieces.append(line[piece:i].rstrip(" \t"))
                piece = i
            word = quote is not None or char not in BOUNDARY
            i += 1
        pieces.append(line[piece:i].rstrip(" \t") if cut else line[piece:])
        kept.append("\n".join(pieces))
        continued = not cut and pieces[-1].endswith("\\")
        heredoc = heredoc or bool(HEREDOC.search(kept[-1]))
    return "\n".join(kept)


def mutants_commands(directory):
    """{workflow name: its `cargo mutants` commands} for the workflows of a directory that run one."""
    found = {}
    for path in sorted(directory.iterdir()):
        if path.suffix not in (".yml", ".yaml"):
            continue
        # A shell continuation is one command: join it before the command is read, and after the
        # comments are cut, because a comment's backslash continues nothing.
        lines = COMMAND.findall(uncommented(workflow(path)).replace("\\\n", " "))
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
        self.assertEqual(found, {"planted.yml": ["cargo mutants --in-place;"]})
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

    def test_every_cargo_mutants_command_carries_the_gates_own_bounds(self):
        found = mutants_commands(WORKFLOWS)
        for name in ("ci.yml", "mutation-weekly.yml"):
            self.assertIn(name, found, f"{name} runs no cargo mutants command")
            self.assertGreaterEqual(len(found[name]), 1, name)
        for name, lines in examined("workflow files", list(found.items())):
            for line in examined(f"{name} commands", lines):
                self.assertRegex(line, BOUNDED, f"{name}: {line}")


if __name__ == "__main__":
    unittest.main()

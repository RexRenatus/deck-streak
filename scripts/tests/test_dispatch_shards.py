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
    `opens`), and not at all on a continued line, after a here-document, or after a line that ends
    inside a quote or past such a place, where bash may still be inside it. Before that place a `#`
    that begins a word cuts the rest of the line: its text is no command, and bounds written in it
    bound nothing. After it a `#` glued to a word is text, and any other `#` ends the command before
    it and hides nothing, so a command after it is still read. So a `#` after the `)` that closes a
    substitution, or after an operator inside `${ }`, is no comment, and the reader can refuse a
    line bash would pass but never hides a command that bash runs."""
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
        continued = not cut and (pieces[-1].endswith("\\") or quote is not None or not sure)
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
        'cargo mutants "--timeout 300" "--build-timeout 600"\n',
        "cargo mu$()tants --in-place\n",
        "cargo mu${X-}tants --in-place\n",
        "c$()argo mutants --in-place\n",
        "cargo mutants --timeout\\ 300 --build-timeout\\ 600\n",
        "cargo mutants '--timeout 300 --build-timeout 600'\n",
        "cargo mutants --timeout 300\\\n --build-timeout 600\n",
        "cargo mutants --timeout 300 --build-timeout 600 -- x\n",
        "cargo mutants -- --timeout 300 --build-timeout 600\n",
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
        "cargo mutants --timeout 300>x --build-timeout 600\n",
        "cargo mutants --timeout 300 2>/dev/null --build-timeout 600\n",
        "cargo mutants --timeout 300 {fd}>/dev/null --build-timeout 600\n",
        "cargo mutants --timeout 300 >x 600\n",
        "cargo mutants --timeout 300 --build-timeout 600>/dev/null\n",
        "cargo mutants --timeout 300 --build-timeout 600 2>&1\n",
        "cargo mutants --timeout 300 --build-timeout <<<600\n",
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
        "cargo mutants --timeout 30${{ matrix.shard }} --build-timeout 600\n",
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
    included = grammar_workflow("cargo mutants --timeout 30${{ matrix.shard }} --build-timeout 600")
    included = included.replace(
        "    steps:",
        "    strategy:\n      matrix:\n        shard: [0]\n"
        "        include:\n          - shard: 1\n    steps:",
    )
    texts = [f"cargo mutants --timeout 30{v} --build-timeout 600\n" for v in "01"]
    members.append(("structure", included, texts))
    return members


# The oracle runs each text as a script, under a timeout, twice (with the stub's status 0 and 1, so
# an `||` or an `&&` takes each branch), with a path that holds only the stubs: `cargo` and
# `cargo-mutants` log their words, each call to a file of its own process, and the programs that
# run another program pass it on.
GRAMMAR_ORACLE = r"""
while IFS= read -r f; do
  for ORACLE_STATUS in 0 1; do
    export ORACLE_TAG="${f##*/}" ORACLE_STATUS
    { "$TIMEOUT" -k 1 10 "$SHELL_UNDER_TEST" --noprofile --norc "$f" </dev/null >/dev/null 2>&1; } \
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
        for stub in stubs.iterdir():
            stub.chmod(0o700)
        (root / "oracle.sh").write_text(GRAMMAR_ORACLE, encoding="utf-8")
        for n, text in enumerate(scripts):
            (root / f"m{n}").write_text(text, encoding="utf-8")
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


if __name__ == "__main__":
    unittest.main()

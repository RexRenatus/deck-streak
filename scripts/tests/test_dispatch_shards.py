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
import os
import re
import shutil
import string
import subprocess
import sys
import tempfile
import textwrap
import tomllib
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined
from test_mutation_workflows import VERDICT, WEEKLY, WORKFLOWS, jobs, listed, shard, workflow

WHOLE = 32
COUNT = "${{ needs.size.outputs.shards }}"
BOUNDS = "--timeout 300 --build-timeout 600"
# A command is `cargo mutants` where bash itself runs `cargo` (or the `cargo-mutants` binary) as
# the program and the first word past cargo's own flags is `mutants`; `mutants_in()` below finds
# each with the words bash passes it, or refuses the workflow. The bounds are matched whole, so a
# digit or a decimal appended to either value is not the gate's bound.
# The bounds are four whole words: only a blank or an end may stand on either side.
BOUNDED = re.compile(r"(?<!\S)" + re.escape(BOUNDS) + r"(?!\S)")


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
        commands = re.findall(r"cargo mutants [^\n]*", size_job)
        self.assertEqual(len(commands), 2, "the size job lists in one branch per package state")
        for flag in (
            "--no-shuffle",
            "--list",
            "--json",
            "--in-place",
            "--timeout 300",
            "--build-timeout 600",
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


# The reader (#395, #447). A `run:` value reaches bash in three steps, and the reader takes each as
# its grammar defines it, or it refuses: YAML 1.2.2 decodes the scalar, GitHub substitutes each
# `${{ }}` expression, and bash splits the text into commands and words (POSIX XCU 2.3 and bash's
# quoting). A refusal is reported as a command that carries no bounds, so it fails the guard too.


class Refused(Exception):
    """A workflow the reader cannot read exactly; the message says where and why."""


YAML_KEY = re.compile(r"([A-Za-z0-9_][A-Za-z0-9_.-]*):(?:[ \t]+|$)")
BLOCK_HEADER = re.compile(r"([|>])(?:([1-9])([-+]?)|([-+])([1-9]?))?[ \t]*(?:#.*)?$")
FLOW_LIST = re.compile(r"\[([^\[\]{}'\"#]*)\][ \t]*(?:#.*)?$")
YAML_ESCAPES = dict(zip('0abt\tnvfre "/\\N_LP', '\0\a\b\t\t\n\v\f\r\x1b "/\\\x85\xa0\u2028\u2029'))
YAML_HEX = {"x": 2, "u": 4, "U": 8}


def indent_of(line):
    return len(line) - len(line.lstrip(" "))


def flow_scalar(lines, k, i, parent):
    """(value, line, column after it) of the quoted scalar opening at `lines[k][i]`, folded as
    YAML folds a flow scalar: a line break is a space, each empty line a newline."""
    quote, line, i, out, blanks = lines[k][i], lines[k], i + 1, [], ""
    while True:
        if i >= len(line):
            k, empty = k + 1, 0
            while k < len(lines) and not lines[k].strip(" \t"):
                k, empty = k + 1, empty + 1
            if k == len(lines) or indent_of(lines[k]) <= parent or lines[k].startswith("---"):
                raise Refused(f"line {k}: a quoted scalar this reader cannot close")
            out.append("\n" * empty or " ")
            line, i, blanks = lines[k], len(lines[k]) - len(lines[k].lstrip(" \t")), ""
            continue
        c = line[i]
        if c in " \t":
            blanks, i = blanks + c, i + 1
        elif c == quote and not (quote == "'" and line.startswith("''", i)):
            return "".join(out) + blanks, k, i + 1
        elif quote == "'":
            out.append(blanks + c)
            blanks, i = "", i + (2 if c == "'" else 1)
        elif c != "\\":
            out.append(blanks + c)
            blanks, i = "", i + 1
        elif i + 1 == len(line):
            if k + 1 == len(lines) or not lines[k + 1].strip(" \t"):
                raise Refused(f"line {k}: an escaped line break before an empty line")
            out.append(blanks)
            k, blanks = k + 1, ""
            line, i = lines[k], len(lines[k]) - len(lines[k].lstrip(" \t"))
        elif line[i + 1] in YAML_ESCAPES:
            out.append(blanks + YAML_ESCAPES[line[i + 1]])
            blanks, i = "", i + 2
        elif line[i + 1] in YAML_HEX:
            n = YAML_HEX[line[i + 1]]
            digits = line[i + 2 : i + 2 + n]
            if len(digits) < n or not all(d in "0123456789abcdefABCDEF" for d in digits):
                raise Refused(f"line {k}: a malformed escape")
            out.append(blanks + chr(int(digits, 16)))
            blanks, i = "", i + 2 + n
        else:
            raise Refused(f"line {k}: an escape YAML does not define")


def block_scalar(lines, k, header, parent):
    """(value, next line) of the literal or folded block whose header `header` ends line `k`."""
    style, chomp = header.group(1), header.group(3) or header.group(4) or ""
    step = header.group(2) or header.group(5)
    first = k + 1
    while first < len(lines) and not lines[first].strip(" "):
        first += 1
    auto = indent_of(lines[first]) if first < len(lines) else 0
    n = parent + int(step) if step else max(auto, parent + 1)
    if any(len(line) > n for line in lines[k + 1 : first]):
        raise Refused(f"line {k}: a leading empty line deeper than its block")
    body, trailing, k = [], 0, k + 1
    while k < len(lines) and (not lines[k].strip(" ") or indent_of(lines[k]) >= n):
        body.append(lines[k][n:])
        k += 1
    while body and not body[-1]:
        body.pop()
        trailing += 1
    kept = "\n" * trailing if chomp == "+" else ""
    if not body:
        return kept, k
    if style == ">":
        if any(x[:1] in (" ", "\t") for x in body):
            raise Refused(f"line {k}: a more-indented line in a folded block")
        text, empty = "", 0
        for x in body:
            if not x:
                empty += 1
                continue
            text += ("\n" * empty or " ") if text else "\n" * empty
            text, empty = text + x, 0
    else:
        text = "\n".join(body)
    return (text, k) if chomp == "-" else (text + "\n" + kept, k)


def yaml_entries(text):
    """[(key path, value)] for every key of a workflow's block mappings: a scalar decoded by YAML's
    rules, a one-line flow list as its items, and None for a nested block. Anything else (a flow
    mapping, an anchor, an alias, a tag, a quoted or complex key, a directive) is refused."""
    if "\r" in text or "\ufeff" in text:
        raise Refused("a carriage return or a byte-order mark")
    lines = text.split("\n")
    entries, stack, k = [], [], 0
    while k < len(lines):
        line = lines[k]
        body = line.lstrip(" ")
        if not body.strip(" \t") or body.startswith("#"):
            k += 1
            continue
        col = indent_of(line)
        if body[0] == "\t" or (col == 0 and body.startswith(("%", "---", "..."))):
            raise Refused(f"line {k}: a tab indent, a directive or a document marker")
        while body == "-" or body.startswith("- "):
            while stack and (stack[-1][0] > col or (stack[-1][0] == col and not stack[-1][2])):
                stack.pop()
            rest = body[1:].lstrip(" ")
            col, body = col + len(body) - len(rest), rest
        if not body:
            k += 1
            continue
        key = YAML_KEY.match(body)
        if not key:
            if body[0] in "'\"":
                _, k, i = flow_scalar(lines, k, col, col - 1)
                after = lines[k][i:].strip(" \t")
            elif body[0] == "[" and FLOW_LIST.match(body):
                after = ""
            elif body[0] in "[{&*!|>?%@`" or re.search(r":(?:[ \t]|$)", body.split(" #")[0]):
                after = ":"
            else:
                after = ""
            if after and not after.startswith("#"):
                raise Refused(f"line {k}: a key or a node this reader does not read")
            k += 1
            continue
        while stack and stack[-1][0] >= col:
            stack.pop()
        name, value = key.group(1), body[key.end() :]
        path = tuple(entry[1] for entry in stack) + (name,)
        k, decoded = yaml_value(lines, k, col, value)
        stack.append((col, name, decoded is None))
        entries.append((path, decoded))
    return entries


def yaml_value(lines, k, col, value):
    """(next line, decoded value) of the value `value` that follows a key at column `col`."""
    if not value or value.startswith("#"):
        nxt = next(
            (x for x in lines[k + 1 :] if x.strip(" \t") and not x.lstrip().startswith("#")), ""
        )
        if indent_of(nxt) > col and not (
            YAML_KEY.match(nxt.lstrip(" ")) or nxt.lstrip(" ")[:2] == "- "
        ):
            raise Refused(f"line {k}: a scalar on the line after its key")
        return k + 1, None
    header = BLOCK_HEADER.match(value)
    if header:
        text, k = block_scalar(lines, k, header, col)
        return k, text
    if value[0] in "'\"":
        text, k, i = flow_scalar(lines, k, len(lines[k]) - len(value), col)
        if lines[k][i:].strip(" \t") and not lines[k][i:].strip(" \t").startswith("#"):
            raise Refused(f"line {k}: text after a quoted scalar")
        return k + 1, text
    if value[0] == "[":
        items = FLOW_LIST.match(value)
        if not items:
            raise Refused(f"line {k}: a flow list this reader does not read")
        return k + 1, [item.strip() for item in items.group(1).split(",") if item.strip()]
    if value[0] in "{&*!|>?%@`":
        raise Refused(f"line {k}: a flow mapping, an anchor, an alias or a tag")
    # A comment ends a plain scalar, on its first line or a later one.
    comment = re.search(r"[ \t]#", value)
    text, parts, k = value[: comment.start() if comment else None].rstrip(" \t"), [], k + 1
    while not comment and k < len(lines):
        part = lines[k].strip(" \t")
        if part and (indent_of(lines[k]) <= col or part.startswith("#")):
            break
        comment = re.search(r"[ \t]#", part)
        parts.append(part[: comment.start() if comment else None].rstrip(" \t"))
        k += 1
    after = next((x for x in lines[k:] if x.strip(" \t")), "")
    if indent_of(after) > col and not after.lstrip(" \t").startswith("#"):
        raise Refused(f"line {k}: text after a comment that ends a plain scalar")
    while parts and not parts[-1]:
        parts.pop()
    if any(": " in part or ":\t" in part for part in parts):
        raise Refused(f"line {k}: a key inside a plain scalar")
    empty = 0
    for part in parts:
        if not part:
            empty += 1
            continue
        text, empty = text + ("\n" * empty or " ") + part, 0
    if ": " in text or text.endswith(":"):
        raise Refused(f"line {k}: a plain scalar that holds a key")
    return k, text


EXPRESSION = re.compile(r"\$\{\{(.*?)\}\}", re.DOTALL)


def run_texts(text):
    """Each text GitHub can hand bash from a workflow's `run:` values. A `${{ matrix.<key> }}`
    takes each value its job lists; any other expression's value is not in the workflow, and a
    value can end one command and start another (GitHub's hardening guide), so it is refused."""
    entries = yaml_entries(text)
    for path, value in entries:
        if path[-1] == "shell" and value != "bash":
            raise Refused(f"the shell {value!r}, which is not bash")
    for path, value in entries:
        if path[-1] != "run" or not isinstance(value, str):
            continue
        job = path[:2] if path[:1] == ("jobs",) else None
        lists = {
            p[-1]: v
            for p, v in entries
            if job and p[:2] == job and "matrix" in p and isinstance(v, list)
        }
        if any(p[:2] == job and {"include", "exclude"} & set(p) for p, _ in entries):
            lists = {}
        variants = [value]
        for expression in dict.fromkeys(EXPRESSION.findall(value)):
            name = re.fullmatch(r"\s*matrix\.([A-Za-z0-9_-]+)\s*", expression)
            if not name or name.group(1) not in lists:
                raise Refused(f"the expression ${{{{{expression}}}}}, whose value is not stated")
            spelled = "${{" + expression + "}}"
            variants = [v.replace(spelled, item) for v in variants for item in lists[name.group(1)]]
        for variant in variants:
            if "${{" in variant:
                raise Refused("an unclosed expression")
            yield variant


SEPARATORS = {";", ";;", ";&", ";;&", "&&", "||", "|", "|&", "&", "(", ")", "\n"}
REDIRECTS = {"<", ">", ">>", ">|", "<>", "<&", ">&", "&>", "&>>", "<<", "<<-", "<<<"}
OPERATORS = sorted(SEPARATORS - {"\n"} | REDIRECTS, key=len, reverse=True)
ANSI_C = dict(zip("abeEfnrtv\\'\"?", "\a\b\x1b\x1b\f\n\r\t\v\\'\"?"))
PARAMETER = re.compile(r"[#!]?(?:[A-Za-z_][A-Za-z0-9_]*|[0-9]+|[@*#?$!-])")
ARITHMETIC = set("0123456789_ \t\n+-*/%<>=!&|^~?:,#") | set(string.ascii_letters)
SHELL_STATE = {"shopt", "enable", "set", "alias"}
ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?\+?=")
# The words after which bash runs the next word as a program, with the words after it unchanged.
LEADERS = {"if", "then", "else", "elif", "do", "while", "until", "{", "!", "time", "command"}
LEADERS |= {"builtin", "exec"}
# The programs whose text is Python, which this guard does not read (SPEC-129 section 8).
PYTHON = ("python3", "python")
# The memory scope's wrapper (SPEC-196) runs the words after its `--` as the command, unchanged:
# `TheMemoryScopeRunsTheWordsAfterItsSeparator` measures that of the file, so its form is read
# through. A value bash hands on as exactly one word: literal, or double quotes around literal text
# and named expansions (no split, no glob, no `@`).
WRAPPER = ("python3", "scripts/memory_scope.py")
ONE_WORD = re.compile(r'"(?:[^"\\$`]|\$[A-Za-z_][A-Za-z0-9_]*|\$\{[A-Za-z_][A-Za-z0-9_]*\})*"')
# The builtin that reads its arguments as shell, and the shells that read the text after `-c`.
EVALUATORS = ("eval",)
SHELLS = ("bash", "sh")
# The characters that make bash read a text as other than the one plain word it spells.
SPECIAL = re.compile(r"[\s'\"\\$`;&|()<>#*?\[\]{}~=!\0]")
# The `set` options that change what bash reads from text or passes as arguments.
SET_READING = {"keyword", "posix", "histexpand"}


class Word:
    """One word as bash reads it: `value` after quote removal (a NUL where an expansion goes),
    `raw` as written, `dynamic` when an expansion decides it, `io` for a redirection's number,
    and `texts`, what it hands on: its value, an assignment's value, or a here-document's body."""

    def __init__(self, value, raw, dynamic, io, texts=None, bare=False):
        self.value, self.raw, self.dynamic, self.io = value, raw, dynamic, io
        self.texts = [] if texts is None else texts
        self.bare = bare  # an expansion stands outside double quotes, where bash splits its result

    def can_be_dashes(self):
        """Whether an expansion can leave the word as exactly `--`: bash splits an unquoted one into
        words of its own, and a word with no literal character besides `-` is whatever it expands to."""
        return "\0" in self.value and (self.bare or all(c in "\0-" for c in self.value))

    def shown(self):
        text = self.raw if self.dynamic else self.value
        return text.replace(" ", "\\x20").replace("\t", "\\t").replace("\n", "\\n")


class Shell:
    """Bash's reading of one script: its simple commands (in every substitution too) as words,
    and `data`, the texts that can spell `cargo` or `mutants` and that it hands on (a word's or an
    assignment's value, an expansion's operand, a here-document or a here-string, except a Python
    program's), which bash or another program may read as shell."""

    def __init__(self, text):
        self.text, self.i = text, 0
        self.commands, self.data, self.heredocs = [], [], []

    def refuse(self, why):
        raise Refused(f"{why}, at {self.text[max(0, self.i - 20) : self.i + 20]!r}")

    def at(self, k=0):
        return self.text[self.i + k : self.i + k + 1]

    def read(self, closer=False):
        """Read commands to the end, or to a substitution's unmatched `)`."""
        tokens, depth, pending = [], 0, len(self.heredocs)
        while True:
            while self.at() in (" ", "\t") or (self.at() == "\\" and self.at(1) == "\n"):
                self.i += 2 if self.at() == "\\" else 1
            c = self.at()
            if not c:
                if closer:
                    self.refuse("an unclosed substitution")
                break
            if c == "#":
                end = self.text.find("\n", self.i)
                self.i = len(self.text) if end < 0 else end
            elif c == "\n":
                self.i += 1
                tokens.append("\n")
                self.here_documents()
            elif c == ")" and closer and depth == 0:
                if len(self.heredocs) > pending:
                    self.refuse("a here-document inside a substitution's one line")
                self.i += 1
                break
            elif self.text.startswith(("<(", ">("), self.i):
                tokens.append(self.word())
            elif self.text.startswith("((", self.i):
                self.i += 2
                self.arithmetic()
            elif c in ";&|()<>":
                op = next(op for op in OPERATORS if self.text.startswith(op, self.i))
                self.i += len(op)
                depth += {"(": 1, ")": -1}.get(op, 0) if closer else 0
                tokens.append(op)
                if op in ("<<", "<<-"):
                    body = Word("", "", False, True)
                    self.here_document(op, body)
                    tokens.append(body)
            else:
                word = self.word()
                if not word.dynamic and (word.value == "=~" or (closer and word.value == "case")):
                    self.refuse("a regular expression or a case inside a substitution")
                tokens.append(word)
        words, texts, target = [], [], False
        for token in tokens + ["\n"]:
            if isinstance(token, str):
                target = token in REDIRECTS
                if token in SEPARATORS and (words or texts):
                    self.command(words, texts)
                    words, texts = [], []
                continue
            texts += token.texts
            if target or token.io:
                target = False
            else:
                words.append(token)

    def command(self, words, texts):
        program = next((w for w in words if not ASSIGNMENT.match(w.value)), None)
        python = program and not program.dynamic and program.value.rsplit("/", 1)[-1] in PYTHON
        # A python command that could run the memory scope's wrapper runs a command, not Python:
        # one with an argument that names the wrapper, or that bash computes. Its texts are read.
        arguments = words[words.index(program) + 1 :] if python else []
        if not python or any(w.dynamic or "memory_scope" in w.value for w in arguments):
            self.data += texts
        if not words:
            return
        k = 0
        while k + 1 < len(words) and words[k].value in ("builtin", "command"):
            k += 1
        head, rest = words[k], words[k + 1 :]
        if head.value in SHELL_STATE and not head.dynamic:
            options = [w for w in rest if w.value[:1] in "-+" and w.value != "--"]
            named = [
                rest[n + 1] for n, w in enumerate(rest[:-1]) if w in options and "o" in w.value
            ]
            if (
                head.value != "set"
                or any(w.dynamic or set("kH") & set(w.value) for w in options)
                or any(w.dynamic or w.value in SET_READING for w in named)
            ):
                self.refuse(f"`{head.value}`, which changes how bash reads what follows")
        self.commands.append(words)

    def word(self, element=False):
        start, value, dynamic, bare = self.i, [], False, False
        if self.text.startswith(("<(", ">("), self.i):
            self.i += 2
            self.read(closer=True)
            value, dynamic, bare = ["\0"], True, True
        while True:
            c = self.at()
            if not c or c in " \t\n;&|<>)":
                if c in "<>" and self.at(1) == "(" and self.i > start:
                    self.refuse("a process substitution glued to a word")
                break
            if c == "(":
                if self.i > start and self.text[self.i - 1] in "@*+?!":
                    # A pattern character glued to `(` is one word to bash where extended globbing
                    # is on, as it always is after `==`, `!=` and `=` in `[[ ]]`.
                    self.refuse("an extended pattern, which bash reads as one word")
                if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*\+?=", self.text[start : self.i]):
                    break
                self.array()
                value, dynamic, bare = value + ["\0"], True, True
            elif c == "\\":
                if self.at(1) != "\n":
                    value.append(self.at(1) or "\\")
                self.i += 2
            elif c == "'":
                end = self.text.find("'", self.i + 1)
                if end < 0:
                    self.refuse("an unclosed single quote")
                value.append(self.text[self.i + 1 : end])
                self.i = end + 1
            elif c == '"':
                dynamic = self.double_quoted(value) or dynamic
            elif c == "$":
                expansions = value.count("\0")
                dynamic = self.dollar(value, quoted=False) or dynamic
                bare = bare or value.count("\0") > expansions
            elif c == "`":
                self.backquote()
                value, dynamic, bare = value + ["\0"], True, True
            elif c == "[" and (
                # bash reads a subscript as one word after a name, and at an array element's start
                re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", self.text[start : self.i])
                or (element and self.i == start)
            ):
                end = self.text.find("]", self.i)
                if end < 0 or not re.fullmatch(r"[A-Za-z0-9_$+*/%-]*", self.text[self.i + 1 : end]):
                    self.refuse("a subscript")
                value.append(self.text[self.i : end + 1])
                self.i, dynamic = end + 1, True
            else:
                glob = (
                    c in "*?[" or (c in "{}" and self.i > start) or (c == "~" and self.i == start)
                )
                glob = glob or (c == "{" and self.at(1) not in ("", " ", "\t", "\n"))
                value.append(c)
                self.i, dynamic = self.i + 1, dynamic or glob
        raw, text = self.text[start : self.i], "".join(value)
        io = self.at() in ("<", ">") and bool(
            re.fullmatch(r"[0-9]+|\{[A-Za-z_][A-Za-z0-9_]*\}", raw)
        )
        # A value is handed on where reading it again can give other words than this one word; an
        # assignment's value is handed on whole, as `$name` gives it back.
        assignment = ASSIGNMENT.match(text)
        texts = [text] if SPECIAL.search(text) else []
        texts = [t for t in texts + [text[assignment.end() :] if assignment else ""] if suspect(t)]
        return Word(text, raw, dynamic or "\0" in text, io, texts, bare)

    def double_quoted(self, value):
        self.i += 1
        dynamic = False
        while True:
            c = self.at()
            if not c:
                self.refuse("an unclosed double quote")
            if c == '"':
                self.i += 1
                return dynamic
            if c == "\\":
                if self.at(1) in ("$", "`", '"', "\\"):
                    value.append(self.at(1))
                elif self.at(1) != "\n":
                    value.append("\\")
                    self.i -= 1
                self.i += 2
            elif c == "$" and self.at(1) not in ("'", '"'):
                dynamic = self.dollar(value, quoted=True) or dynamic
            elif c == "`":
                self.backquote()
                value.append("\0")
                dynamic = True
            else:
                value.append(c)
                self.i += 1

    def dollar(self, value, quoted):
        """Read the `$` form at the cursor into `value`; True when an expansion decides it."""
        nxt = self.at(1)
        if nxt == "'" and not quoted:
            self.i += 2
            value.append(self.ansi_c())
            return False
        if nxt == '"' and not quoted:
            self.i += 1
            return self.double_quoted(value)
        value.append("\0")
        if self.text.startswith("$((", self.i):
            self.i += 3
            self.arithmetic()
        elif nxt == "(":
            self.i += 2
            self.read(closer=True)
        elif nxt == "{":
            self.parameter(quoted)
        elif nxt == "[":
            self.refuse("`$[`, an arithmetic expansion bash reads as one word")
        elif nxt and (nxt.isalnum() or nxt in "_@*#?$!-"):
            name = PARAMETER.match(self.text, self.i + 1)
            self.i = name.end() if nxt.isalpha() or nxt == "_" else self.i + 2
        else:
            value[-1] = "$"
            self.i += 1
            return False
        return True

    def ansi_c(self):
        out = []
        while True:
            c = self.at()
            if not c:
                self.refuse("an unclosed ANSI-C string")
            if c == "'":
                self.i += 1
                return "".join(out).split("\0")[0]
            if c != "\\":
                out.append(c)
                self.i += 1
                continue
            d = self.at(1)
            octal = re.match(r"[0-7]{1,3}", self.text[self.i + 1 : self.i + 4])
            sized = {"x": 2, "u": 4, "U": 8}.get(d)
            digits = re.match(r"[0-9a-fA-F]+", self.text[self.i + 2 : self.i + 2 + (sized or 0)])
            if d in ANSI_C:
                out.append(ANSI_C[d])
                self.i += 2
            elif octal:
                out.append(chr(int(octal.group(0), 8) & 0xFF))
                self.i += 1 + len(octal.group(0))
            elif sized and digits:
                out.append(chr(int(digits.group(0), 16)))
                self.i += 2 + len(digits.group(0))
            elif d == "c" and self.at(2):
                out.append(chr(ord(self.at(2)) & 0x1F))
                self.i += 3
            else:
                out.append("\\")
                self.i += 1

    def parameter(self, quoted):
        self.i += 2
        name = PARAMETER.match(self.text, self.i)
        if not name:
            self.refuse("a parameter expansion this reader does not read")
        self.i = start = name.end()
        while True:
            c = self.at()
            if not c or c == "{" or (c == "'" and quoted) or c == "[":
                self.refuse("a parameter expansion this reader does not read")
            if c == "}":
                if suspect(self.text[start : self.i]):
                    self.data.append(self.text[start : self.i])
                self.i += 1
                return
            if c == "\\":
                self.i += 2
            elif c == "'":
                end = self.text.find("'", self.i + 1)
                if end < 0:
                    self.refuse("an unclosed single quote")
                self.i = end + 1
            elif c == '"':
                self.double_quoted([])
            elif c == "$":
                self.dollar([], quoted)
            elif c == "`":
                self.backquote()
            else:
                self.i += 1

    def arithmetic(self):
        depth = 0
        while True:
            c = self.at()
            if c == ")" and depth == 0:
                if self.at(1) != ")":
                    self.refuse("an arithmetic expansion this reader does not read")
                self.i += 2
                return
            if c == "$":
                self.dollar([], quoted=True)
                continue
            if c not in ARITHMETIC | {"(", ")"} or not c:
                self.refuse("an arithmetic expansion this reader does not read")
            depth += {"(": 1, ")": -1}.get(c, 0)
            self.i += 1

    def array(self):
        self.i += 1
        while True:
            while self.at() in (" ", "\t", "\n") or (self.at() == "\\" and self.at(1) == "\n"):
                self.i += 2 if self.at() == "\\" else 1
            c = self.at()
            if c == ")":
                self.i += 1
                return
            if c == "#":
                end = self.text.find("\n", self.i)
                self.i = len(self.text) if end < 0 else end
            elif not c or c in ";&|(<>":
                self.refuse("an array this reader does not read")
            else:
                self.word(element=True)

    def backquote(self):
        end = self.text.find("`", self.i + 1)
        if end < 0 or "\\" in self.text[self.i : end]:
            self.refuse("a backquote this reader does not read")
        inner = Shell(self.text[self.i + 1 : end])
        inner.read()
        self.commands += inner.commands
        self.data += inner.data
        self.i = end + 1

    def here_document(self, op, body):
        while self.at() in (" ", "\t"):
            self.i += 1
        word = self.word()
        if word.dynamic or not word.raw:
            self.refuse("a here-document this reader does not read")
        self.heredocs.append((word.value, word.raw != word.value, op == "<<-", body))

    def here_documents(self):
        """Read each body the line opened, to its delimiter's line or to the end of the text, as
        the program it goes to reads it: an unquoted delimiter's body with each expansion a NUL."""
        while self.heredocs:
            delimiter, quoted, tabs, body = self.heredocs.pop(0)
            out = []
            while self.i < len(self.text):
                end = self.text.find("\n", self.i)
                end = len(self.text) if end < 0 else end
                while tabs and self.at() == "\t":
                    self.i += 1
                if self.text[self.i : end] == delimiter:
                    self.i = min(end + 1, len(self.text))
                    break
                if quoted:
                    out.append(self.text[self.i : end])
                    self.i = end
                while self.i < end:
                    c = self.at()
                    if c == "\\" and self.i + 1 == end:
                        self.refuse("a continued line in a here-document")
                    if c == "$" and self.at(1) not in ("'", '"'):
                        self.dollar(out, quoted=True)
                    elif c == "`":
                        self.backquote()
                        out.append("\0")
                    elif c == "\\" and self.at(1) in ("$", "`", "\\"):
                        out.append(self.at(1))
                        self.i += 2
                    else:
                        out.append(c)
                        self.i += 1
                if self.i > end:
                    self.refuse("an expansion that runs past its here-document line")
                out.append("\n")
                self.i = end + 1
            text = "".join(out)
            if suspect(text):
                body.texts.append(text)


CARGO = ("cargo", "cargo-mutants")
VALUED_FLAGS = {"--config", "--color", "-C", "-Z", "--explain"}
PLAIN_FLAGS = {
    "--version",
    "--list",
    "--verbose",
    "--quiet",
    "--locked",
    "--offline",
    "--frozen",
    "--help",
}


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


def mutants_of(words, handed=False):
    """The `cargo mutants` commands among one simple command's words, each shown from its program
    word; the words after a `--` go to the test tool, so they are shown joined and bound nothing.
    A command is found only where bash itself runs `cargo` as the program, or where the wrapper's
    form runs it as the first word after its `--` (`wrapped`). Refused: a literal
    `cargo` word whose subcommand bash computes or never gives; `cargo mutants` anywhere else, or
    in a text another program runs (`handed`), where that program decides its arguments; and a
    literal `mutants` word that is not a found command's subcommand. A text bash computes for a
    program that reads it as shell (`EVALUATORS`, or `-c` to one of `SHELLS`) is refused."""
    program = 0
    while program < len(words) and (
        ASSIGNMENT.match(words[program].value) or words[program].value in LEADERS
    ):
        program += 1
    # The wrapper runs its command without a shell: its first word is the program, as it stands.
    while program < len(words) and (after := wrapped(words, program)) is not None:
        program = after
    name = words[program].value.rsplit("/", 1)[-1] if program < len(words) else ""
    rest = words[program + 1 :]
    if name in EVALUATORS or (
        name in SHELLS and any(re.fullmatch(r"-[a-z]*c[a-z]*", w.value) for w in rest)
    ):
        text = rest if name in EVALUATORS else [w for w in rest if w.value[:1] not in "-+"][:1]
        upto = rest.index(text[-1]) + 1 if text else len(rest)
        if any(w.dynamic for w in rest[:upto]):
            raise Refused(f"a text bash computes for `{words[program].raw}` to read as shell")
    found, used = [], set()
    for p, word in enumerate(words):
        if word.dynamic or word.value.rsplit("/", 1)[-1] not in CARGO:
            continue
        q, valued = p + 1, False
        while q < len(words):
            w = words[q]
            if w.dynamic:
                raise Refused(f"a word bash computes where cargo reads its subcommand: {w.raw}")
            if w.value == "mutants" and (p != program or handed):
                raise Refused(f"`{word.raw} mutants` where another program decides its arguments")
            if w.value == "mutants":
                args = [x.shown() for x in words[p:]]
                cut = next((k for k in range(q - p, len(args)) if args[k] == "--"), len(args))
                # R5: a word before the bounds that bash can expand to exactly `--` ends cargo's
                # options there, so the bounds after it are the test tool's, and the guard cannot
                # tell from the text whether it does.
                bounds = BOUNDS.split()
                where = next(
                    (
                        k
                        for k in range(q - p, len(args))
                        if args[k : k + len(bounds)] == bounds and k < cut
                    ),
                    len(args),
                )
                for early in words[q + 1 : p + where]:
                    if early.can_be_dashes():
                        raise Refused(f"a word bash computes before the bounds: {early.raw}")
                found.append(
                    " ".join(args[:cut] + (["\\x20".join(args[cut:])] if args[cut:] else []))
                )
                used.add(q)
                break
            if w.value[:1] in "+-":
                valued = w.value in VALUED_FLAGS or (
                    w.value[:2] == "--"
                    and "=" not in w.value
                    and w.value not in PLAIN_FLAGS | {"--"}
                )
            elif valued:
                valued = False
            else:
                break
            q += 1
        else:
            raise Refused(f"`{word.raw}` whose subcommand comes from its input or another program")
    for q, word in enumerate(words):
        if word.value == "mutants" and q not in used:
            raise Refused(f"`mutants` that is not the subcommand of a cargo bash runs: {word.raw}")
    return found


def unquoted(text):
    """The text with every quote mark, backslash, `$` and line break gone: a superset of what
    quote removal, in the text and in every text it hands on, can spell from it."""
    return re.sub(r"['\"\\$\n]", "", text)


def suspect(text):
    """Whether quote removal, here or in a text handed on, can spell `mutants` or `cargo` from
    this text, or an ANSI-C string can (a `$` and a `'` with only quoting between them)."""
    bare = unquoted(text)
    return "mutants" in bare or "cargo" in bare or bool(re.search(r"\$['\"\\$\n]*'", text))


def mutants_in(text, handed=False):
    """The `cargo mutants` commands bash would run from one script, as words; each text it hands
    on is read again by the same grammar, and any command there is refused. A text that cannot
    spell `cargo` or `mutants` is not read."""
    if not suspect(text):
        return []
    shell = Shell(text)
    shell.read()
    found = [line for words in shell.commands for line in mutants_of(words, handed)]
    for data in shell.data:
        if data != text:
            mutants_in(data, handed=True)
    return found


def mutants_commands(directory):
    """{workflow name: its `cargo mutants` commands} for the workflows of a directory that run one;
    a workflow the reader refuses answers with the refusal, which carries no bounds."""
    found = {}
    for path in sorted(directory.iterdir()):
        if path.suffix not in (".yml", ".yaml"):
            continue
        try:
            lines = [line for text in run_texts(workflow(path)) for line in mutants_in(text)]
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
    except Exception:
        wrapper = None
    if wrapper is not None:
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
        '--timeout 300 --build-timeout 600 > "$RUNNER_TEMP/size/package.json"'
    ),
    "rust": (
        'cargo mutants --no-shuffle -vV --in-place ${PACKAGE:+--package "$PACKAGE"} '
        '--sharding round-robin --shard "$SHARD/$SHARDS" --timeout 300 --build-timeout 600 '
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
SPY_WORDS += ("--timeout", "300")
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


if __name__ == "__main__":
    unittest.main()

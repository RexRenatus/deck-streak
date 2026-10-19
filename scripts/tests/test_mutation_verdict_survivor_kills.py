"""The verdict's survivors of the release run, each killed by a whole observable (SPEC-492 R2).

Each test loads `scripts/mutation-verdict.py` in process under its own name and calls its functions
over a literal input, or over a fixture tree in a temporary directory, and compares the whole result
with its expected value spelled literally: a mutant of the token a test names changes that value. A
test that stands in for the verdict's git read (`git`), its report readers (`whole_reports`,
`table_rust`, `table_web`, `table_python`) or the rows runner's git read (`load_revision`) keeps
every other input. A scan a mutant could turn into an endless loop runs under a deadline, so that
mutant fails its test rather than holding its leg.
"""

import contextlib
import dataclasses
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest
from argparse import Namespace
from pathlib import Path
from unittest import mock

from _support import REPO, examined

VERDICT = REPO / "scripts" / "mutation-verdict.py"
SCRIPTS = REPO / "scripts"


def verdict():
    spec = importlib.util.spec_from_file_location("mutation_verdict_survivor_kills", VERDICT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


V = verdict()

#: The verdict loaded in a child as `verdict()` loads it here, registered before it runs, since its
#: dataclasses look their module up by name.
CHILD_LOAD = (
    "spec = importlib.util.spec_from_file_location('verdict_child', sys.argv[-1])\n"
    "module = importlib.util.module_from_spec(spec)\n"
    "sys.modules[spec.name] = module\n"
    "spec.loader.exec_module(module)\n"
)
#: A child that imports what the verdict imports and turns bytecode back on before it loads the
#: verdict, so only the verdict's own line can turn it off again.
CHILD_BYTECODE = (
    "import importlib.util, sys\n"
    "sys.path.insert(0, sys.argv[1])\n"
    "import mutation_python, mutation_rows\n"
    "sys.dont_write_bytecode = False\n"
    f"{CHILD_LOAD}"
    "print(sys.dont_write_bytecode)\n"
)
#: A child started by `-c`, whose first search path is the empty string until the verdict loads.
CHILD_PATH = f"import importlib.util, sys\n{CHILD_LOAD}print(sys.path[0])\n"


def scratch(test):
    holder = tempfile.TemporaryDirectory()
    test.addCleanup(holder.cleanup)
    return Path(holder.name)


def tree(test, files):
    """A fixture root holding `files`, {path from the root: text}."""
    root = scratch(test)
    for relative, text in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return root


def child(test, code, *argv):
    """What a fresh interpreter prints running `code`, started without PYTHONDONTWRITEBYTECODE and
    with its bytecode cache in the test's own scratch."""
    env = {name: value for name, value in os.environ.items() if name != "PYTHONDONTWRITEBYTECODE"}
    env["PYTHONPYCACHEPREFIX"] = str(scratch(test))
    done = subprocess.run(
        [sys.executable, "-c", code, *map(str, argv)],
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
        check=False,
    )
    test.assertEqual(done.returncode, 0, done.stderr)
    return done.stdout


def within(seconds, call, *args):
    """`call(*args)`, failed once `seconds` pass: a mutant that never ends the scan fails here."""
    answer = []

    def run():
        try:
            answer.append((True, call(*args)))
        except BaseException as error:
            answer.append((False, error))

    worker = threading.Thread(target=run, daemon=True)
    worker.start()
    worker.join(seconds)
    if not answer:
        raise AssertionError(f"{call.__name__} did not return within {seconds} s")
    returned, value = answer[0]
    if not returned:
        raise value
    return value


class TheModuleSetup(unittest.TestCase):
    def test_loading_the_verdict_turns_off_bytecode(self):
        self.assertEqual(child(self, CHILD_BYTECODE, SCRIPTS, VERDICT), "True\n")

    def test_the_scripts_directory_is_searched_first(self):
        self.assertEqual(child(self, CHILD_PATH, VERDICT), f"{SCRIPTS}\n")

    def test_the_usage_exit_is_the_one_argparse_gives(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as stop:
            V.main(["no-such-verb"])
        self.assertEqual(stop.exception.code, 2)
        self.assertEqual(V.EXIT_USAGE, stop.exception.code)

    def test_a_record_and_a_mutant_are_frozen(self):
        record = V.Record("c.json", 1, {})
        mutant = V.Mutant("f.rs", "replace + with -", (1, 1), (1, 2), "f.rs:1:1: replace + with -")
        with self.assertRaises(dataclasses.FrozenInstanceError):
            record.index = 2
        with self.assertRaises(dataclasses.FrozenInstanceError):
            mutant.name = "n"


#: {case: ((text, start, quote, multiline), the end `quoted_end` gives)}.
QUOTED = {
    "an empty literal": (('""x"', 0, '"', True), 2),
    "an escaped quote": (('"a\\"b"', 0, '"', True), 6),
    "an escape before the closing quote": (('"\\a"bcd', 0, '"', True), 4),
    "a one-line literal left open": (('"ab\ncd"', 0, '"', False), 3),
    "a literal left open": (('"abc', 0, '"', True), 4),
}


class TheQuotedEnd(unittest.TestCase):
    def check(self, names):
        for name in examined("quoted literals", names):
            arguments, end = QUOTED[name]
            self.assertEqual(V.quoted_end(*arguments), end, name)

    def test_each_string_of_quoted_end_is_observed(self):
        self.check(["an escaped quote", "a one-line literal left open"])

    def test_each_integer_of_quoted_end_is_observed(self):
        self.check(["an escape before the closing quote", "an empty literal"])

    def test_each_loop_step_of_quoted_end_is_observed(self):
        self.check(["an escape before the closing quote"])

    def test_each_return_value_of_quoted_end_is_observed(self):
        self.check(["a one-line literal left open", "a literal left open"])


#: {case: (text, language, the lines `c_code_lines` marks)}. Each `/[/*]/` reads as a regular
#: expression where a regular expression may begin, and else as a division and a bracket before a
#: block comment that hides the lines after it.
LINES = {
    "a regex at the start": ("/[/*]/\nx\n*/", "js", {1, 2, 3}),
    "a first line": ("x\n\ny", "js", {1, 3}),
    "a svelte comment": ("<!-- a\nb -->\nc", "svelte", {3}),
    "a script's comment opener": ("<!-- a\nb -->\nc", "js", {1, 2, 3}),
    "svelte code": ("x\ny\n", "svelte", {1, 2}),
    "an empty svelte comment": ("<!---->\nx", "svelte", {2}),
    "an unclosed svelte comment": ("<!-- a\nb", "svelte", set()),
    "a closed svelte comment": ("<!-- a -->\nc", "svelte", {2}),
    "code after a svelte comment": ("<!-- a -->c\nd", "svelte", {1, 2}),
    "a brace after a template": ("`${a}`\n}\n\n//x\n", "js", {1, 2}),
    "an empty template": ("``\n/*\nx*/", "js", {1}),
    "a division after a template": ("`a`/[/*]/\nx\n*/", "js", {1}),
    "a regex after return": ("return /[/*]/\nx\n*/", "js", {1, 2, 3}),
    "a division after return's template": ("return`a`/[/*]/\nx\n*/", "js", {1}),
    "a division after a name": ("x /[/*]/\ny\n*/", "js", {1}),
    "a division after a string": ("'a'/[/*]/\nx\n*/", "js", {1}),
    "a name led by an underscore": ("_return /[/*]/\nx\n*/", "js", {1}),
    "a name led by a dollar": ("$return /[/*]/\nx\n*/", "js", {1}),
    "a dollar name": ("$a\nb", "js", {1, 2}),
    "a regex after new": ("new/[/*]/\nx\n*/", "js", {1, 2, 3}),
    "a punctuation line": (";\nx", "js", {1, 2}),
    "a block across a template's lines": ("`${ {a}\n/* c\n*/ }`\nx", "js", {1, 3, 4}),
    "a block inside a template": ("`${ {a} }`\n// c\nx", "js", {1, 3}),
    "an operator inside a template": ("`${a+b}`\n// c\nx", "js", {1, 3}),
    "a comment after punctuation": (";/*\nx\n*/", "js", {1}),
}


class TheCCodeLines(unittest.TestCase):
    def check(self, names):
        for name in examined("texts whose code lines are marked", names):
            text, language, lines = LINES[name]
            self.assertEqual(within(10, V.c_code_lines, text, language), lines, name)

    def test_each_string_of_c_code_lines_is_observed(self):
        self.check(
            [
                "a regex at the start",
                "a svelte comment",
                "svelte code",
                "a brace after a template",
                "a division after a template",
                "a regex after return",
                "a division after a name",
                "a division after a string",
                "a name led by an underscore",
                "a name led by a dollar",
                "a dollar name",
                "a block across a template's lines",
                "a block inside a template",
            ]
        )

    def test_each_operand_of_c_code_lines_is_observed(self):
        self.check(
            [
                "a name led by an underscore",
                "a script's comment opener",
                "a division after a name",
                "a regex after return",
            ]
        )

    def test_each_negation_of_c_code_lines_is_observed(self):
        self.check(["a name led by a dollar", "a dollar name"])

    def test_each_integer_of_c_code_lines_is_observed(self):
        self.check(
            [
                "a regex after new",
                "a punctuation line",
                "a block inside a template",
                "an operator inside a template",
                "a comment after punctuation",
                "a block across a template's lines",
                "a first line",
                "an empty svelte comment",
                "code after a svelte comment",
                "an empty template",
            ]
        )

    def test_each_arithmetic_of_c_code_lines_is_observed(self):
        self.check(["a punctuation line", "a svelte comment", "a closed svelte comment"])

    def test_each_comparison_of_c_code_lines_is_observed(self):
        self.check(
            [
                "a svelte comment",
                "a script's comment opener",
                "an unclosed svelte comment",
                "a closed svelte comment",
                "a regex after return",
                "a division after return's template",
                "a division after a name",
            ]
        )


LIB = "crates/c/src/lib.rs"
LIB_TEXT = "fn f(a: u8, b: u8) -> u8 {\n    a + b\n}\n"
WEB_FILE = "web/app/src/x.ts"


def cargo_mutant(description):
    """A cargo-mutants mutant of `a + b`'s `+`, as its listing and its outcomes name it."""
    return {
        "file": LIB,
        "name": f"{LIB}:2:7: {description}",
        "span": {"start": {"line": 2, "column": 7}, "end": {"line": 2, "column": 8}},
    }


def outcome(summary, description):
    return {"summary": summary, "scenario": {"Mutant": cargo_mutant(description)}}


def a_record(file, mutant):
    return {
        "file": file,
        "mutant": mutant,
        "anchor": "a + b",
        "reason": "r",
        "evidence": "e",
        "issue": "#1",
    }


#: A Rust record bound to `- for +` in `LIB`, and a Mini App record bound to Stryker's `a - b`.
RECORDS = {
    "scripts/mutation-equivalent.d/c.json": json.dumps(
        {"records": [a_record(LIB, "replace + with -")]}
    ),
    "scripts/mutation-equivalent.d/miniapp.json": json.dumps(
        {"records": [a_record(WEB_FILE, "ArithmeticOperator: a - b")]}
    ),
    LIB: LIB_TEXT,
}


def stryker(mutator, replacement, status, column):
    return {
        "mutatorName": mutator,
        "replacement": replacement,
        "status": status,
        "location": {
            "start": {"line": 1, "column": column},
            "end": {"line": 1, "column": column + 5},
        },
    }


#: One report of each kind: cargo-mutants' outcomes (one missed mutant a record binds, one it does
#: not, one caught), Stryker's (one survivor a record binds, one it does not, one uncovered, one
#: killed) and a rows proof (one survived, one void with no target, one killed).
REPORTS = {
    "rust/outcomes.json": json.dumps(
        {
            "outcomes": [
                outcome("MissedMutant", "replace + with -"),
                outcome("MissedMutant", "replace + with *"),
                outcome("CaughtMutant", "replace + with /"),
            ]
        }
    ),
    "web/mutation.json": json.dumps(
        {
            "files": {
                "src/x.ts": {
                    "source": "export const f = (a, b) => a + b;\n",
                    "mutants": [
                        stryker("ArithmeticOperator", "a - b", "Survived", 28),
                        stryker("ArithmeticOperator", "a * b", "Survived", 28),
                        stryker("BlockStatement", "{}", "NoCoverage", 1),
                        stryker("ArithmeticOperator", "a / b", "Killed", 28),
                    ],
                }
            }
        }
    ),
    "rows/rows.json": json.dumps(
        [
            {"id": "S1-A", "target": "scripts/a.py", "verdict": "SURVIVED", "reason": "s"},
            {"id": "S2-B", "verdict": "VOID", "reason": "v"},
            {"id": "S3-C", "target": "scripts/c.py", "verdict": "KILLED", "reason": "k"},
        ]
    ),
}
SURVIVORS = (
    {
        LIB: [f"{LIB}:2:7: replace + with *"],
        WEB_FILE: [
            f"{WEB_FILE}:1: Survived: ArithmeticOperator -> 'a * b'",
            f"{WEB_FILE}:1: NoCoverage: BlockStatement -> '{{}}'",
        ],
        "scripts/a.py": ["row S1-A: SURVIVED: s"],
        "<no target>": ["row S2-B: VOID: v"],
    },
    2,
)


class TheSurvivorsIn(unittest.TestCase):
    def check(self):
        found, excused = V.survivors_in(tree(self, REPORTS), tree(self, RECORDS))
        self.assertEqual((dict(found), excused), SURVIVORS)

    def test_each_operand_of_survivors_in_is_observed(self):
        self.check()

    def test_each_integer_of_survivors_in_is_observed(self):
        self.check()

    def test_each_loop_step_of_survivors_in_is_observed(self):
        self.check()

    def test_each_string_of_survivors_in_is_observed(self):
        self.check()

    def test_each_comparison_of_survivors_in_is_observed(self):
        self.check()


class TheChangedLines(unittest.TestCase):
    def changed(self, text):
        """`changed_lines` over a diff whose text is `text`, as git would print it."""
        seen = []

        def git(root, *args):
            seen.append((root, args))
            return text

        root = scratch(self)
        with mock.patch.object(V, "git", git):
            found = V.changed_lines(root, "b", "h")
        self.assertEqual(
            seen, [(root, ("diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", "b...h"))]
        )
        return found

    def test_each_string_of_changed_lines_is_observed(self):
        no_sides = "diff --git a/x b/x\n--- /dev/null\n+++ /dev/null\n@@ -0,0 +1 @@\n+a\n"
        self.assertEqual(self.changed(no_sides), {None: {"added": [1], "deleted": 0}})

    def test_each_flag_of_changed_lines_is_observed(self):
        headers_outside_a_file = (
            "--- a/z\n+++ b/z\n"
            "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n"
            "--- a/y\n+++ b/y\n"
        )
        self.assertEqual(self.changed(headers_outside_a_file), {"x": {"added": [1], "deleted": 1}})

    def test_each_integer_of_changed_lines_is_observed(self):
        counted = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,2 +7,1 @@\n"
        self.assertEqual(self.changed(counted), {"x": {"added": [7], "deleted": 2}})


def judge_args(root, **cells):
    fields = {
        "rows": None,
        "klass": "web",
        "shard_reports": None,
        "root": str(root),
        "whole": None,
        "listed": None,
        "plan": None,
    }
    fields.update(cells)
    return Namespace(**fields)


def a_plan(files=(), shards=None):
    plan = {"row_overlaps": {}, "classes": {"rust": {"applies": True}}, "files": list(files)}
    if shards is not None:
        plan["shards"] = {"shards": shards}
    return plan


def say(*lines):
    return "".join(f"mutation: rust: {line}\n" for line in lines)


#: A plan with a Rust file of no test lines and a Python file of test lines, judged over one whole
#: report that counts no timeout or unviable mutant and names none of its missed one.
UNNAMED = (
    say(
        f"{LIB}: 1 changed code line(s)",
        "cargo-mutants examined 2 (caught 1, missed 1, timeout 0), unviable 0, of 3 on the diff",
        "MISSED <unnamed mutant>",
        "missed 1: equivalent 0, unexplained 1",
        "examined 2 by cargo-mutants and 0 by rows",
        f"unexamined: {LIB}: no mutant and no row covers its changed lines",
    ),
    ["MISSED <unnamed mutant>"],
    [],
    2,
)
EXCUSED = (
    say(
        "cargo-mutants examined 1 (caught 0, missed 1, timeout 0), unviable 0, of 1 on the diff",
        f"EQUIVALENT {LIB}:2:7: replace + with -: r (#1)",
        "missed 1: equivalent 1, unexplained 0",
        "examined 1 by cargo-mutants and 0 by rows",
    ),
    [],
    [],
    1,
)
SUMMED = (
    say(
        "cargo-mutants examined 2 (caught 2, missed 0, timeout 0), unviable 0, of 2 on the diff",
        "missed 0: equivalent 0, unexplained 0",
        "VOID the shards' reports count 2 mutant(s), and the plan's listing holds 0",
        "examined 2 by cargo-mutants and 0 by rows",
    ),
    [],
    ["the shards' reports count 2 mutant(s), and the plan's listing holds 0"],
    2,
)
NEVER_TESTED = "never tested: m, listed for mutation-rust-shard-0"
NO_BASELINE = (
    "mutation-rust-shard-0: it left no record of its unmutated baseline, so no mutant of it was "
    "judged"
)
NOTHING = "production code changed and nothing was examined"
UNSUMMED = (
    say(
        "cargo-mutants examined 0 (caught 0, missed 0, timeout 0), unviable 0, of 0 on the diff",
        "missed 0: equivalent 0, unexplained 0",
        f"VOID {NEVER_TESTED}",
        f"VOID {NO_BASELINE}",
        "examined 0 by cargo-mutants and 0 by rows",
        f"VOID {NOTHING}",
    ),
    [],
    [NEVER_TESTED, NO_BASELINE, NOTHING],
    0,
)


class TheJudgeRust(unittest.TestCase):
    def judge(self, plan, args, whole):
        """`judge_rust` over `plan` with `whole` as the run's whole reports: (what it said, its
        failures, its voids, what it examined)."""
        out = io.StringIO()
        judged = V.Verdict("rust")
        with (
            contextlib.redirect_stdout(out),
            mock.patch.object(V, "whole_reports", lambda _verdict, _plan, _args: whole),
        ):
            V.judge_rust(judged, plan, args)
        return out.getvalue(), judged.failures, judged.voids, judged.examined

    def unnamed(self):
        files = [
            {"path": LIB, "class": "rust", "code": [2], "test": []},
            {"path": "scripts/x.py", "class": "python", "code": [1], "test": [3]},
        ]
        report = {
            "caught": 1,
            "missed": 1,
            "total_mutants": 3,
            "outcomes": [{"summary": "MissedMutant", "scenario": {"Mutant": {}}}],
        }
        found = self.judge(a_plan(files), judge_args(scratch(self)), [("", report)])
        self.assertEqual(found, UNNAMED)

    def excused(self):
        root = tree(
            self, dict(RECORDS, **{"listing.json": json.dumps([cargo_mutant("replace + with -")])})
        )
        report = {
            "caught": 0,
            "missed": 1,
            "timeout": 0,
            "unviable": 0,
            "total_mutants": 1,
            "outcomes": [outcome("MissedMutant", "replace + with -")],
        }
        args = judge_args(root, whole=str(root / "listing.json"))
        self.assertEqual(self.judge(a_plan(), args, [("", report)]), EXCUSED)

    def test_each_integer_of_judge_rust_is_observed(self):
        self.unnamed()
        self.excused()

    def test_each_string_of_judge_rust_is_observed(self):
        self.unnamed()

    def test_each_comparison_of_judge_rust_is_observed(self):
        empty = tree(self, {"listed.json": "[]"})
        counted = {
            "caught": 2,
            "missed": 0,
            "timeout": 0,
            "unviable": 0,
            "total_mutants": 2,
            "outcomes": [],
        }
        args = judge_args(empty, shard_reports=str(empty), listed=str(empty / "listed.json"))
        self.assertEqual(self.judge(a_plan(shards=[]), args, [("", counted)]), SUMMED)
        listed = tree(self, {"listed.json": json.dumps([{"name": "m"}])})
        nothing = dict(counted, caught=0, total_mutants=0)
        args = judge_args(listed, shard_reports=str(listed), listed=str(listed / "listed.json"))
        shards = [{"shard": 0, "mutants": ["m"]}]
        whole = [("mutation-rust-shard-0: ", nothing)]
        self.assertEqual(self.judge(a_plan(shards=shards), args, whole), UNSUMMED)

    def test_each_operand_of_judge_rust_is_observed(self):
        self.unnamed()


class TheLoadRecords(unittest.TestCase):
    def test_each_loop_step_of_load_records_is_observed(self):
        root = tree(
            self,
            {
                "scripts/mutation-equivalent.d/a.json": '{"records": [], "records": []}',
                "scripts/mutation-equivalent.d/b.json": '{"x": 1}',
                "scripts/mutation-equivalent.d/c.json": '{"records": [{"file": "f"}]}',
            },
        )
        self.assertEqual(
            V.load_records(root),
            (
                [V.Record("c.json", 1, {"file": "f"})],
                [
                    "scripts/mutation-equivalent.d/a.json repeats the key 'records' in one object",
                    'b.json: holds no records list, {"records": [...]}',
                ],
            ),
        )


#: {case: ((text, start, nested), the end `block_end` gives)}.
BLOCKS = {
    "a comment that does not nest": (("/* a */ b */", 0, False), 7),
    "an empty comment": (("/**/x", 0, True), 4),
    "a comment before code": (("/* */x", 0, True), 5),
    "a comment left open": (("/* abc", 0, True), 6),
}


class TheBlockEnd(unittest.TestCase):
    def check(self, names):
        for name in examined("block comments", names):
            arguments, end = BLOCKS[name]
            self.assertEqual(V.block_end(*arguments), end, name)

    def test_each_integer_of_block_end_is_observed(self):
        self.check(["a comment that does not nest", "an empty comment", "a comment before code"])

    def test_each_return_value_of_block_end_is_observed(self):
        self.check(["a comment left open"])


class TheCensus(unittest.TestCase):
    def test_each_loop_step_of_census_is_observed(self):
        root = tree(
            self,
            {
                "scripts/mutation-equivalent.d/a.json": '{"records": [{}]}',
                "scripts/mutation-equivalent.d/b.json": '{"records": [{}]}',
            },
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = V.census(root)
        self.assertEqual(
            (code, out.getvalue()),
            (
                1,
                "census: a.json: named for no package of the workspace, nor miniapp\n"
                "census: b.json: named for no package of the workspace, nor miniapp\n"
                "examined 2 record(s)\n",
            ),
        )


class TheSourcesOffset(unittest.TestCase):
    def sources(self):
        sources = V.Sources(scratch(self))
        sources.put("f", "ab\ncd\n")
        return sources

    def test_each_operand_of_sources_offset_is_observed(self):
        sources = self.sources()
        self.assertIsNone(sources.offset("f", 2, 0))
        self.assertIsNone(sources.offset("f", 4, 1))
        self.assertEqual(sources.offset("f", 2, 1), 3)

    def test_each_integer_of_sources_offset_is_observed(self):
        sources = self.sources()
        self.assertEqual(sources.offset("f", 2, 1), 3)
        self.assertEqual(sources.offset("f", 1, 2), 1)


#: {case: (text, the end `rust_literal` gives for the literal that opens at offset 2)}.
RUST_LITERALS = {
    "a byte string": ('= b"x";', 6),
    "a name before a quote": ("= x'a';", None),
    "a raw byte string": ('= br"a";', 7),
    "a C character": ("= c'x';", 6),
    "a raw string": ('= r"a";', 6),
    "a hashed raw string": ('= r#"a"#;', 8),
    "a hashed raw string holding a hash": ('= r#"#"#;', 8),
    "an empty raw string": ('= r"";', 5),
    "a character": ("= 'a'; x", 5),
    "an escaped character": ("= '\\n';", 6),
    "a long escape": ("= '\\x41';", 8),
    "a lifetime": ("= 'a> x", None),
    "a quoted line break": ("= '\n';", None),
}


class TheRustLiteral(unittest.TestCase):
    def check(self, names):
        for name in examined("Rust literals", names):
            text, end = RUST_LITERALS[name]
            self.assertEqual(V.rust_literal(text, 2), end, name)

    def test_each_comparison_of_rust_literal_is_observed(self):
        self.check(
            ["a byte string", "a raw string", "a character", "a lifetime", "an escaped character"]
        )

    def test_each_string_of_rust_literal_is_observed(self):
        self.check(
            [
                "a byte string",
                "a raw byte string",
                "a C character",
                "a raw string",
                "a hashed raw string",
                "a character",
                "an escaped character",
                "a long escape",
                "a quoted line break",
            ]
        )

    def test_each_operand_of_rust_literal_is_observed(self):
        self.check(["a name before a quote", "a lifetime", "a byte string", "a character"])

    def test_each_arithmetic_of_rust_literal_is_observed(self):
        self.check(
            [
                "a byte string",
                "a raw string",
                "a hashed raw string",
                "a hashed raw string holding a hash",
                "an escaped character",
                "a long escape",
                "a character",
                "a quoted line break",
            ]
        )

    def test_each_integer_of_rust_literal_is_observed(self):
        self.check(
            [
                "a byte string",
                "a raw string",
                "a hashed raw string",
                "an empty raw string",
                "an escaped character",
                "a character",
                "a quoted line break",
            ]
        )

    def test_each_return_value_of_rust_literal_is_observed(self):
        self.check(["a raw string", "an escaped character", "a character"])


#: An anchor of two lines, its last ending in its line break, from the file's third line.
SPAN_TEXT, SPAN_FIND = "x\ny\na\nb\nz\n", "a\nb\n"


class TheRowSpan(unittest.TestCase):
    def test_each_arithmetic_of_row_span_is_observed(self):
        self.assertEqual(V.row_span(SPAN_TEXT, SPAN_FIND), range(3, 5))

    def test_each_string_of_row_span_is_observed(self):
        self.assertEqual(V.row_span(SPAN_TEXT, SPAN_FIND), range(3, 5))

    def test_each_integer_of_row_span_is_observed(self):
        self.assertEqual(V.row_span(SPAN_TEXT, SPAN_FIND), range(3, 5))


class TheLanguageOf(unittest.TestCase):
    def check(self):
        languages = {
            "web/app/src/a.svelte": "svelte",
            "web/app/src/a.ts": "js",
            "scripts/a.py": "python",
            LIB: "rust",
        }
        for path in examined("paths given a language", sorted(languages)):
            self.assertEqual(V.language_of(path), languages[path], path)

    def test_each_string_of_language_of_is_observed(self):
        self.check()

    def test_each_return_value_of_language_of_is_observed(self):
        self.check()


class TheModuleTables(unittest.TestCase):
    def test_test_items_holds_each_entry(self):
        self.assertEqual(V.TEST_ITEMS, frozenset({"fn", "mod", "impl", "trait"}))

    def test_qualifiers_holds_each_entry(self):
        self.assertEqual(
            V.QUALIFIERS,
            frozenset({"pub", "const", "async", "unsafe", "extern", "default", "auto"}),
        )

    def test_brackets_holds_each_entry(self):
        self.assertEqual(V.BRACKETS, {"(": ")", "[": "]", "{": "}"})

    def test_stryker_examined_holds_each_entry(self):
        self.assertEqual(V.STRYKER_EXAMINED, ("Killed", "Survived", "NoCoverage", "Timeout"))

    def test_tool_exits_holds_each_entry(self):
        self.assertEqual(
            V.TOOL_EXITS,
            {
                1: "a usage error",
                4: "a red baseline: the tests failed before any mutant",
                5: "a diff that does not match the tree",
                6: "a diff it could not read",
                70: "an internal error",
            },
        )

    def test_leg_results_holds_each_entry(self):
        self.assertEqual(V.LEG_RESULTS, ("success", "failure", "cancelled", "skipped"))

    def test_regex_words_holds_each_entry(self):
        self.assertEqual(
            V.REGEX_WORDS,
            frozenset(
                {
                    "return",
                    "typeof",
                    "instanceof",
                    "case",
                    "do",
                    "else",
                    "in",
                    "of",
                    "new",
                    "delete",
                    "void",
                    "throw",
                    "yield",
                    "await",
                }
            ),
        )


class TheSourcesWindow(unittest.TestCase):
    def test_each_operand_of_sources_window_is_observed(self):
        sources = V.Sources(scratch(self))
        self.assertIsNone(sources.window("missing.rs", "a"))
        sources.put("f", "xay")
        self.assertEqual(sources.window("f", "a"), (1, 2))


TABLE_TEXT = (
    "table: verdict: ok\n"
    "examined 18 mutant(s) in 2 report(s)\n"
    "table: core: listed 11, killed 5, equivalent 3, unexplained 2, unviable 1\n"
    "table: miniapp: listed 7, killed 7, equivalent 0, unexplained 0, unviable 0\n"
)


class TheTable(unittest.TestCase):
    def check(self):
        """`table` over two packages' tallies, each set by its report reader's stand-in."""

        def rust(_root, _reports, _scope, _records, _listed, tallies, _fail, _void):
            core = tallies["core"]
            core.killed, core.equivalent, core.unexplained, core.unviable = 5, 3, 2, 1
            return 1

        def web(_root, _reports, _records, tally, _fail, _void):
            tally.killed = 7
            return 1

        out = io.StringIO()
        args = Namespace(root=str(scratch(self)), reports=str(scratch(self)), package=None)
        args.listed = None
        with (
            contextlib.redirect_stdout(out),
            mock.patch.object(V, "table_rust", rust),
            mock.patch.object(V, "table_web", web),
            mock.patch.object(V, "table_python", lambda *_cells: 0),
        ):
            code = V.table(args)
        self.assertEqual((code, out.getvalue()), (0, TABLE_TEXT))

    def test_each_integer_of_table_is_observed(self):
        self.check()

    def test_each_string_of_table_is_observed(self):
        self.check()

    def test_each_arithmetic_of_table_is_observed(self):
        self.check()

    def test_each_comparison_of_table_is_observed(self):
        self.check()


#: A crate row whose killer runs in `crates/c/src` and a scripts row on `scripts/t.py`'s second
#: line whose killer's file is `scripts/tests/m.py`.
ROWS = {
    "_": [
        "target spelling: MUTATIONS is crate-relative",
        "target spelling: SCRIPT_MUTATIONS is repo-rooted",
    ],
    "arities": {"MUTATIONS": [7], "SCRIPT_MUTATIONS": [6]},
    "tables": {
        "MUTATIONS": [["S1-A", "c", "src/lib.rs", "fn a", "fn b", "lib::t", "d"]],
        "SCRIPT_MUTATIONS": [["S2-B", "scripts/t.py", "b = 2", "b = 3", "d", "m.C.t"]],
    },
}


class TheSelectedRows(unittest.TestCase):
    def selected(self, changed):
        """(the rows `selected_rows` chooses, the rows it finds on changed lines) for `changed`,
        with the base holding the same rows."""
        root = tree(
            self,
            {
                "scripts/mutation-rows.json": json.dumps(ROWS),
                "scripts/t.py": "a = 1\nb = 2\nc = 3\n",
                "scripts/tests/m.py": "",
            },
        )
        plan = V.Plan(base="b", head="h")
        base = json.loads(json.dumps(ROWS))
        with mock.patch.object(V.mutation_rows, "load_revision", lambda _root, _base: base):
            V.selected_rows(root, "b", "h", changed, plan)
        return plan.rows, plan.row_overlaps

    def check(self, cases):
        for changed, expected in examined("changes the rows are selected for", cases):
            self.assertEqual(self.selected(changed), expected, changed)

    def test_each_string_of_selected_rows_is_observed(self):
        self.check(
            [
                ({"scripts/tests/m.py/x": {"added": [1], "deleted": 0}}, ([], {})),
                ({"crates/c/src-gen/x.rs": {"added": [1], "deleted": 0}}, ([], {})),
                ({"crates/c/src/x.rs": {"added": [1], "deleted": 0}}, (["S1-A"], {})),
            ]
        )

    def test_each_operand_of_selected_rows_is_observed(self):
        self.check(
            [
                ({"crates/c/src-gen/x.rs": {"added": [1], "deleted": 0}}, ([], {})),
                ({"scripts/t.py": {"added": [3], "deleted": 0}}, (["S2-B"], {})),
                (
                    {"scripts/t.py": {"added": [2], "deleted": 0}},
                    (["S2-B"], {"S2-B": "scripts/t.py"}),
                ),
            ]
        )

    def test_each_arithmetic_of_selected_rows_is_observed(self):
        self.check(
            [
                ({"crates/c/src-gen/x.rs": {"added": [1], "deleted": 0}}, ([], {})),
                ({"scripts/t.py": {"added": [3], "deleted": 0}}, (["S2-B"], {})),
            ]
        )


#: The Python files a diff changes when it changes no Rust production file: one more than the
#: twenty `not_applicable` names.
TWENTY_ONE = list("abcdefghijklmnopqrstu")
NO_RUST = "mutation: rust: not-applicable: the diff changes no rust production file; it changes "
TEST_ONLY_LINE = (
    "mutation: rust: not-applicable: crates/c/src/lib.rs: 2 changed line(s): 1 test-only, inside "
    "an item marked #[cfg(test)] or with a test attribute, which cargo-mutants never mutates\n"
)


class TheNotApplicable(unittest.TestCase):
    def said(self, files):
        """What `not_applicable` says of a plan whose files are `files`, for the Rust class."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            V.not_applicable(V.Verdict("rust"), {"files": files}, "rust")
        return out.getvalue()

    def changes(self, paths):
        """What `not_applicable` says of a diff that changes only the Python files `paths`."""
        return self.said([{"path": path, "class": "python"} for path in paths])

    def test_each_integer_of_not_applicable_is_observed(self):
        self.assertEqual(
            self.changes(TWENTY_ONE),
            f"{NO_RUST}a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, and 1 more\n",
        )

    def test_each_comparison_of_not_applicable_is_observed(self):
        self.assertEqual(
            self.changes(TWENTY_ONE),
            f"{NO_RUST}a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, and 1 more\n",
        )
        self.assertEqual(self.changes(["a"]), f"{NO_RUST}a\n")

    def test_each_string_of_not_applicable_is_observed(self):
        self.assertEqual(self.changes(["a"]), f"{NO_RUST}a\n")
        self.assertEqual(self.changes([]), f"{NO_RUST}nothing\n")
        test_only = {"path": LIB, "class": "rust", "added": 2, "test": [3], "quiet": []}
        self.assertEqual(self.said([test_only]), TEST_ONLY_LINE)


def actions(test, **variables):
    """(the step-outputs file, a patch of `os.environ` to a run under GitHub Actions that sets only
    `variables`): its outputs land in the test's own scratch, so no test writes CI's own step
    outputs or reads CI's run identity."""
    outputs = scratch(test) / "github-output"
    environment = dict(variables, GITHUB_OUTPUT=str(outputs))
    return outputs, mock.patch.dict(os.environ, environment, clear=True)


class TheSize(unittest.TestCase):
    def test_each_return_value_of_size_is_observed(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = V.size(None, None)
        self.assertEqual(
            (code, out.getvalue()),
            (3, "mutation: size: VOID no --listed holds no cargo-mutants listing\n"),
        )
        outputs, environment = actions(self)
        out = io.StringIO()
        with environment, contextlib.redirect_stdout(out):
            code = V.size(None, "miniapp")
        self.assertEqual(
            (code, out.getvalue(), outputs.read_text(encoding="utf-8")),
            (
                0,
                "mutation: size: 1 shard(s) for the Mini App (miniapp), which runs no Rust leg and "
                "reads no listing; legs 1 of ceiling 234\n",
                "shards=1\nmatrix=[0]\n",
            ),
        )


class TheCargoMutant(unittest.TestCase):
    def test_each_operand_of_cargo_mutant_is_observed(self):
        span = cargo_mutant("replace + with -")["span"]
        self.assertIsNone(V.cargo_mutant({"file": 5, "name": "5:2:7: x", "span": span}))
        self.assertIsNone(V.cargo_mutant({"file": LIB, "name": 5, "span": span}))
        self.assertEqual(
            V.cargo_mutant(cargo_mutant("replace + with -")),
            V.Mutant(LIB, "replace + with -", (2, 7), (2, 8), f"{LIB}:2:7: replace + with -"),
        )


#: What every survivor draft says after its list of survivors.
TRIAGE = (
    "Triage each one (SPEC-057 R1): kill it with a test that asserts the behaviour the mutant "
    "breaks, or, when no test can tell it apart, record it EQUIVALENT in "
    "`scripts/mutation-equivalent.d/<package>.json`, `miniapp.json` for the Mini App "
    "(ADR-070): its `file`, `mutant`, `anchor`, `reason`, `evidence`, a Rust mutant's "
    "`reached_by`, and `issue`. An uncovered Mini App mutant first gains a test that reaches "
    "it. A row that SURVIVED or is VOID is repaired in its band fragment under "
    "`scripts/mutation-rows.d/`.\n"
)
BATTERY = "The weekly mutation battery"


def drafted(source):
    """The draft of `m1` and `m2` on `scripts/a.py`, as `source` found them."""
    return (
        f"{source} found 2 mutant(s) of `scripts/a.py` that no test killed (SPEC-039 R12).\n\n"
        f"- `m1`\n- `m2`\n\n{TRIAGE}"
    )


class TheDraftBody(unittest.TestCase):
    def body(self, **variables):
        _, environment = actions(self, **variables)
        with environment:
            return V.draft_body("scripts/a.py", ["m1", "m2"])

    def test_each_string_of_draft_body_is_observed(self):
        self.assertEqual(self.body(), drafted(BATTERY))
        self.assertEqual(
            self.body(GITHUB_RUN_ID="7", GITHUB_REPOSITORY="o/r"),
            drafted(f"{BATTERY}'s run https://github.com/o/r/actions/runs/7"),
        )
        self.assertEqual(
            self.body(
                GITHUB_RUN_ID="7",
                GITHUB_REPOSITORY="o/r",
                GITHUB_SERVER_URL="https://ghe.test",
            ),
            drafted(f"{BATTERY}'s run https://ghe.test/o/r/actions/runs/7"),
        )

    def test_each_operand_of_draft_body_is_observed(self):
        self.assertEqual(self.body(GITHUB_RUN_ID="7"), drafted(BATTERY))
        self.assertEqual(self.body(GITHUB_REPOSITORY="o/r"), drafted(BATTERY))


class TheRustTokens(unittest.TestCase):
    def test_each_flag_of_rust_tokens_is_observed(self):
        self.assertEqual(V.rust_tokens("/* /* */ */a"), [(11, 12)])


#: A rows report of a survived row on `scripts/a.py` and a void row with no target.
ROW_SURVIVORS = {
    "rows/rows.json": json.dumps(
        [
            {
                "id": "S1-A",
                "target": "scripts/a.py",
                "verdict": "SURVIVED",
                "reason": "s",
            },
            {"id": "S2-B", "verdict": "VOID", "reason": "v"},
        ]
    )
}
DRAFTED = (
    "survivors: Mutation survivors: <no target>: 1 mutant(s): to file\n"
    "survivors: Mutation survivors: scripts/a.py: 1 mutant(s): already open, not filed again\n"
    "survivors: 0 mutant(s) the equivalence record excuses, drafted as no issue\n"
    "examined 2 file(s) with survivors\n"
)
MANIFEST = (
    '[\n  {\n    "title": "Mutation survivors: <no target>",\n    "body": "draft-001.md",\n'
    '    "open": false\n  },\n  {\n    "title": "Mutation survivors: scripts/a.py",\n'
    '    "body": "draft-002.md",\n    "open": true\n  }\n]\n'
)


class TheSurvivors(unittest.TestCase):
    def check(self, out):
        """`survivors` into `out`, with `scripts/a.py`'s issue already open: its exit, what it
        printed, the files it wrote and its manifest."""
        titles = scratch(self) / "titles.json"
        titles.write_text(json.dumps(["Mutation survivors: scripts/a.py"]), encoding="utf-8")
        args = Namespace(
            reports=str(tree(self, ROW_SURVIVORS)),
            out=str(out),
            open_titles=str(titles),
            root=str(scratch(self)),
        )
        printed = io.StringIO()
        _, environment = actions(self)
        with environment, contextlib.redirect_stdout(printed):
            code = V.survivors(args)
        self.assertEqual(
            (
                code,
                printed.getvalue(),
                sorted(path.name for path in out.iterdir()),
                (out / "drafts.json").read_text(encoding="utf-8"),
            ),
            (0, DRAFTED, ["draft-001.md", "draft-002.md", "drafts.json"], MANIFEST),
        )

    def test_each_flag_of_survivors_is_observed(self):
        out = scratch(self) / "made" / "drafts"
        self.check(out)
        self.check(out)

    def test_each_integer_of_survivors_is_observed(self):
        self.check(scratch(self) / "drafts")

    def test_each_string_of_survivors_is_observed(self):
        self.check(scratch(self) / "drafts")

    def test_each_return_value_of_survivors_is_observed(self):
        self.check(scratch(self) / "drafts")


NO_EXIT = "no cargo-mutants exit recorded, so its report may be partial"


class ThePartialReason(unittest.TestCase):
    def test_each_string_of_partial_reason_is_observed(self):
        self.assertEqual(V.partial_reason({"caught": 0}, None, "s"), NO_EXIT)

    def test_each_return_value_of_partial_reason_is_observed(self):
        self.assertEqual(V.partial_reason({"caught": 0}, None, "s"), NO_EXIT)

    def test_each_integer_of_partial_reason_is_observed(self):
        cases = [
            ({"caught": 1, "total_mutants": 1}, None),
            ({"caught": 0}, None),
            (
                {"caught": 2, "total_mutants": 3},
                "the report counts 2 of 3 mutants reported",
            ),
        ]
        for report, reason in examined("reports that leave counts out", cases):
            self.assertEqual(V.partial_reason(report, 0, "s"), reason, report)


class TheBinds(unittest.TestCase):
    def setUp(self):
        self.sources = V.Sources(scratch(self))
        self.sources.put("f.rs", "a + b\nXX\n")

    def binds(self, mutant, **fields):
        """`binds` of a record of `- for +` in `f.rs`, with `fields` set, to `mutant`."""
        record = V.Record(
            "c.json", 1, dict({"file": "f.rs", "mutant": "replace + with -"}, **fields)
        )
        return V.binds(record, mutant, self.sources)

    def test_each_return_value_of_binds_is_observed(self):
        plus = V.Mutant("f.rs", "replace + with -", (1, 3), (1, 4), "f.rs:1:3: replace + with -")
        self.assertIs(self.binds(plus, anchor="a + b"), True)
        self.assertIs(self.binds(plus, file="g.rs", anchor="a + b"), False)
        self.assertIs(self.binds(plus, anchor="nope"), False)

    def test_each_string_of_binds_is_observed(self):
        second = V.Mutant("f.rs", "replace + with -", (2, 1), (2, 2), "f.rs:2:1: replace + with -")
        self.assertIs(self.binds(second), False)


#: The verdict's command line's description: the first paragraph of its docstring.
DESCRIPTION = (
    "mutation-verdict: a pull request's mutation plan and verdict, the weekly battery's "
    "survivors as\nissue drafts and its count of reports, and the tools' configurations and "
    "exclusions (SPEC-039 R2 to\nR6, R10, R12; ADR-057)."
)
#: Each verb's usage check, (its command line, the error it gives).
USAGE_ERRORS = (
    (("plan", "--base", "b"), "plan needs --base and --out"),
    (("shards",), "shards needs --plan"),
    (("judge", "--plan", "p"), "judge needs --plan and --class"),
    (("legs", "--plan", "p"), "legs needs --plan, --rust-leg and --rows-leg"),
    (("survivors", "--reports", "r"), "survivors needs --reports and --out"),
    (("battery", "--reports", "r"), "battery needs --reports and --shards, at least 1"),
    (("table",), "table needs --reports"),
)
#: Each usage check's operands, each alone false, the others true: (its command line, the error).
USAGE_OPERANDS = (
    (("plan", "--base", "b"), "plan needs --base and --out"),
    (("plan", "--out", "o"), "plan needs --base and --out"),
    (("judge", "--plan", "p"), "judge needs --plan and --class"),
    (("judge", "--class", "rust"), "judge needs --plan and --class"),
    (
        ("legs", "--rust-leg", "s", "--rows-leg", "s"),
        "legs needs --plan, --rust-leg and --rows-leg",
    ),
    (
        ("legs", "--plan", "p", "--rows-leg", "s"),
        "legs needs --plan, --rust-leg and --rows-leg",
    ),
    (
        ("legs", "--plan", "p", "--rust-leg", "s"),
        "legs needs --plan, --rust-leg and --rows-leg",
    ),
    (("survivors", "--reports", "r"), "survivors needs --reports and --out"),
    (("survivors", "--out", "o"), "survivors needs --reports and --out"),
    (("battery", "--shards", "2"), "battery needs --reports and --shards, at least 1"),
    (("battery", "--reports", "r"), "battery needs --reports and --shards, at least 1"),
    (
        ("battery", "--reports", "r", "--shards", "0"),
        "battery needs --reports and --shards, at least 1",
    ),
)


class TheMain(unittest.TestCase):
    """`main` with each verb's own work stood in for, so a mutant that lets a verb past its usage
    check returns the stand-in's 77, or the plan's 0, and reads no tree."""

    def stubs(self, calls):
        """Stand-ins for the plan's steps and for the checked verbs; each plan step logs its
        arguments to `calls`."""

        def scope_of(*cells):
            calls.append(("scope_of", cells))
            return "diff", "r"

        def release_run(*cells):
            calls.append(("release_run", cells))
            return False

        def plan_diff(_root, base, head, _out, *rest):
            calls.append(("plan_diff", base, head, rest))
            return "the plan"

        def say_plan(plan):
            calls.append(("say_plan", plan))

        def verb(*_cells):
            return 77

        return {
            "scope_of": scope_of,
            "release_run": release_run,
            "plan_diff": plan_diff,
            "say_plan": say_plan,
            "judge": verb,
            "legs": verb,
            "survivors": verb,
            "battery": verb,
        }

    def main(self, argv, stubs):
        """(what `main(argv)` returned or exited with, what it wrote to stderr)."""
        err = io.StringIO()
        with contextlib.ExitStack() as stack:
            stack.enter_context(contextlib.redirect_stderr(err))
            for name, stub in stubs.items():
                stack.enter_context(mock.patch.object(V, name, stub))
            try:
                code = V.main(list(argv))
            except SystemExit as stop:
                code = stop.code
        return code, err.getvalue()

    def refused(self, argv):
        """(the exit, the usage error after its program's name) of `main(argv)` over a scratch
        root, every verb stood in for."""
        code, err = self.main([*argv, "--root", str(scratch(self))], self.stubs([]))
        lines = err.splitlines()
        return code, lines[-1].partition(": error: ")[2] if lines else ""

    def planned(self, stubs):
        root = scratch(self)
        return self.main(
            ["plan", "--base", "b", "--out", str(root / "o"), "--root", str(root)],
            stubs,
        )

    def test_each_integer_of_main_is_observed(self):
        described = []

        def parse_args(parser, _argv):
            described.append(parser.description)
            raise SystemExit(2)

        with mock.patch.object(V.argparse.ArgumentParser, "parse_args", parse_args):
            self.assertEqual(self.main(["census"], {}), (2, ""))
        self.assertEqual(described, [DESCRIPTION])

    def test_each_string_of_main_is_observed(self):
        calls = []
        self.assertEqual(self.planned(self.stubs(calls)), (0, ""))
        self.assertEqual(
            calls,
            [
                ("scope_of", ("", "", "")),
                ("release_run", ("", "", None, "", "")),
                ("plan_diff", "b", "HEAD", (("diff", "r"), False)),
                ("say_plan", "the plan"),
            ],
        )
        for argv, error in examined("usage checks", USAGE_ERRORS):
            self.assertEqual(self.refused(argv), (2, error), argv)

    def test_each_operand_of_main_is_observed(self):
        for argv, error in examined("usage checks' operands", USAGE_OPERANDS):
            self.assertEqual(self.refused(argv), (2, error), argv)

    def test_each_return_value_of_main_is_observed(self):
        stubs = self.stubs([])
        self.assertEqual(self.planned(stubs), (0, ""))

        def refuse(*_cells):
            raise V.mutation_rows.PopulationRefused("refused")

        stubs["plan_diff"] = refuse
        self.assertEqual(self.planned(stubs), (1, "mutation: plan: REFUSED: refused\n"))


#: A leg's memory-scope record: in force, done, and with no out-of-memory event.
SCOPED = json.dumps({"state": "done", "in_force": True, "oom": 0, "oom_kill": 0})
#: A whole cargo-mutants report of one caught mutant.
COUNTED = {
    "caught": 1,
    "missed": 0,
    "timeout": 0,
    "unviable": 0,
    "total_mutants": 1,
    "outcomes": [],
}


def a_leg(name, code, report=None):
    """A leg's files under `name`: its exit, its memory-scope record and, given one, its report."""
    files = {
        f"{name}/cargo-mutants.exit": f"{code}\n",
        f"{name}/memory-scope.json": SCOPED,
    }
    if report is not None:
        files[f"{name}/mutants.out/outcomes.json"] = json.dumps(report)
    return files


def core_outcome(summary, name):
    return {
        "summary": summary,
        "scenario": {"Mutant": {"package": "core", "name": name}},
    }


NO_OUTCOMES = "no report: its outcomes.json holds no cargo-mutants outcomes"


class TheTableRust(unittest.TestCase):
    def tabled(self, files):
        """`table_rust` over the shards' reports `files`: (the reports it read, the core
        package's tally, its failures, its voids, what it printed)."""
        core = V.Tally()
        failures, voids = [], []
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            read = V.table_rust(
                scratch(self),
                tree(self, files),
                None,
                [],
                None,
                {"core": core},
                failures.append,
                voids.append,
            )
        return read, dataclasses.astuple(core), failures, voids, out.getvalue()

    def skipped_then_read(self):
        legs = dict(a_leg("a", 0), **a_leg("b", 0, COUNTED))
        self.assertEqual(self.tabled(legs), (1, (None, 0, 0, 0, 0), [], [], ""))

    def failed_without_report(self):
        found = self.tabled(a_leg("a", 1))
        self.assertEqual(found, (1, (None, 0, 0, 0, 0), [], [f"a: {NO_OUTCOMES}"], ""))

    def test_each_integer_of_table_rust_is_observed(self):
        self.skipped_then_read()

    def test_each_comparison_of_table_rust_is_observed(self):
        self.skipped_then_read()
        self.failed_without_report()

    def test_each_loop_step_of_table_rust_is_observed(self):
        self.skipped_then_read()
        report = dict(
            COUNTED,
            timeout=1,
            total_mutants=2,
            outcomes=[core_outcome("Weird", "m1"), core_outcome("CaughtMutant", "m2")],
        )
        self.assertEqual(
            self.tabled(a_leg("a", 0, report)),
            (
                1,
                (None, 1, 0, 0, 0),
                [],
                ["a: m1: an outcome this table does not know, Weird"],
                "",
            ),
        )

    def test_each_string_of_table_rust_is_observed(self):
        self.failed_without_report()


class TheIsTestMark(unittest.TestCase):
    def test_each_string_of_is_test_mark_is_observed(self):
        marks = [["test", "[", "x", "]"], ["test", "{", "}"], ["test", "=", '"x"']]
        for inner in examined("test marks a bracket or an equals sign ends", marks):
            self.assertIs(V.is_test_mark(inner), True, inner)


class TheClassCase(unittest.TestCase):
    def test_each_return_value_of_class_case_is_observed(self):
        self.assertEqual(
            V.class_case([], "rust"),
            "not-applicable: the diff changes no rust production file",
        )


class TheWholeReports(unittest.TestCase):
    def judged(self, plan, **cells):
        """(the whole reports `whole_reports` returns, what it said, its voids)."""
        out = io.StringIO()
        verdict = V.Verdict("rust")
        args = judge_args(scratch(self), **dict({"outcomes": None, "tool_exit": None}, **cells))
        with contextlib.redirect_stdout(out):
            whole = V.whole_reports(verdict, plan, args)
        return whole, out.getvalue(), verdict.voids

    def legs(self, files, *mutants):
        """`judged` over the shards' reports `files`, shard k listing `mutants[k]` mutants."""
        shards = [{"shard": k, "mutants": ["m"] * count} for k, count in enumerate(mutants)]
        return self.judged({"shards": {"shards": shards}}, shard_reports=str(tree(self, files)))

    def no_outcomes(self):
        why = "no report: no --outcomes holds no cargo-mutants outcomes"
        self.assertEqual(self.judged({}), ([], f"mutation: rust: VOID {why}\n", [why]))

    def test_each_operand_of_whole_reports_is_observed(self):
        self.no_outcomes()
        missing = scratch(self) / "missing.json"
        why = f"no report: {missing} holds no cargo-mutants outcomes"
        self.assertEqual(
            self.judged({}, outcomes=str(missing)),
            ([], f"mutation: rust: VOID {why}\n", [why]),
        )

    def test_each_string_of_whole_reports_is_observed(self):
        self.no_outcomes()
        why = f"mutation-rust-shard-0: {NO_OUTCOMES}"
        self.assertEqual(self.legs({}, 1), ([], f"mutation: rust: VOID {why}\n", [why]))

    def test_each_comparison_of_whole_reports_is_observed(self):
        self.assertEqual(
            self.legs({}, 0),
            (
                [],
                "mutation: rust: mutation-rust-shard-0: not started: the plan lists no mutant "
                "for it\n",
                [],
            ),
        )
        report = {"mutation-rust-shard-0/mutants.out/outcomes.json": json.dumps(COUNTED)}
        why = f"mutation-rust-shard-0: {NO_EXIT}"
        self.assertEqual(self.legs(report, 0), ([], f"mutation: rust: VOID {why}\n", [why]))

    def test_each_loop_step_of_whole_reports_is_observed(self):
        second = {
            "mutation-rust-shard-1/mutants.out/outcomes.json": json.dumps(COUNTED),
            "mutation-rust-shard-1/cargo-mutants.exit": "0\n",
        }
        whole = [("mutation-rust-shard-1: ", COUNTED)]
        self.assertEqual(
            self.legs(second, 0, 1),
            (
                whole,
                "mutation: rust: mutation-rust-shard-0: not started: the plan lists no mutant "
                "for it\n",
                [],
            ),
        )
        listed_none = dict(second, **{"mutation-rust-shard-0/cargo-mutants.exit": "0\n"})
        self.assertEqual(
            self.legs(listed_none, 0, 1),
            (
                whole,
                "mutation: rust: mutation-rust-shard-0: no mutant listed, and cargo-mutants "
                "reports none\n",
                [],
            ),
        )


#: A Stryker sweep's one whole report.
SWEPT = json.dumps({"files": {}})
OWES_NONE = "battery: mutants-shard-{}: the scope lists no mutant for it, so it owes no report\n"
#: {case: ((the reports' files, shards, package, the listing or None), (exit, what it printed))}.
BATTERIES = {
    "a scope that lists one shard of two": (
        (
            a_leg("mutants-shard-0", 0, {"caught": 1, "total_mutants": 1}),
            2,
            "core",
            [{"package": "core"}],
        ),
        (
            0,
            f"{OWES_NONE.format(1)}battery: counted 2 of 2 reports whole\nexamined 2 report(s)\n",
        ),
    ),
    "the Mini App's scope with a listing": (
        ({"stryker/mutation.json": SWEPT}, 1, "miniapp", []),
        (
            0,
            f"{OWES_NONE.format(0)}battery: counted 2 of 2 reports whole\nexamined 2 report(s)\n",
        ),
    ),
    "two sweeps": (
        (
            {"stryker/a/mutation.json": SWEPT, "stryker/b/mutation.json": SWEPT},
            0,
            "miniapp",
            None,
        ),
        (
            1,
            "battery: PARTIAL stryker: 2 mutation.json, not one report\n"
            "battery: counted 0 of 1 reports whole\nexamined 1 report(s)\n",
        ),
    ),
    "a scope the listing does not name": (
        ({}, 0, "core", [{"package": "web"}, {"package": "a"}]),
        (
            1,
            "battery: the scope core lists no mutant; the listing's packages are a, web, and the "
            "Mini App is miniapp\nbattery: counted 1 of 1 reports whole\nexamined 1 report(s)\n",
        ),
    ),
    "the whole tree with no row and no sweep": (
        (
            dict(
                a_leg("mutants-shard-0", 0, dict(COUNTED, caught=0, timeout=1)),
                **a_leg("mutants-shard-1", 0, dict(COUNTED, caught=0, unviable=1)),
                **{
                    "rows/rows.json": "[]",
                    "web/mutation.json": SWEPT,
                    "mutants-shard-5/x": "",
                },
            ),
            2,
            None,
            None,
        ),
        (
            1,
            "battery: PARTIAL rows: rows.json proves no row\n"
            "battery: MISSING stryker: no mutation.json\n"
            "battery: FOREIGN mutants-shard-5: the run was sized at 2 shard(s)\n"
            "battery: counted 2 of 4 reports whole\nexamined 4 report(s)\n",
        ),
    ),
    "rows that are no list and a sweep that is no object": (
        (
            {
                "rows/rows.json": json.dumps({"a": 1}),
                "stryker/mutation.json": '["files"]',
            },
            0,
            None,
            None,
        ),
        (
            1,
            "battery: PARTIAL rows: rows.json proves no row\n"
            "battery: PARTIAL stryker: 1 mutation.json, not one report\n"
            "battery: counted 0 of 2 reports whole\nexamined 2 report(s)\n",
        ),
    ),
    "the Mini App's scope with a sweep of no files": (
        ({"stryker/mutation.json": "{}"}, 2, "miniapp", None),
        (
            1,
            f"{OWES_NONE.format(0)}{OWES_NONE.format(1)}"
            "battery: PARTIAL stryker: 1 mutation.json, not one report\n"
            "battery: counted 0 of 1 reports whole\nexamined 1 report(s)\n",
        ),
    ),
}


class TheBattery(unittest.TestCase):
    def check(self, names):
        for name in examined("battery runs", names):
            (files, shards, package, listing), expected = BATTERIES[name]
            if listing is not None:
                files = dict(files, **{"listing.json": json.dumps(listing)})
            reports = tree(self, files)
            listed = None if listing is None else str(reports / "listing.json")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = V.battery(reports, shards, package, listed, 0)
            self.assertEqual((code, out.getvalue()), expected, name)

    def test_each_integer_of_battery_is_observed(self):
        self.check(
            [
                "a scope that lists one shard of two",
                "the Mini App's scope with a listing",
                "two sweeps",
            ]
        )

    def test_each_comparison_of_battery_is_observed(self):
        self.check(["a scope the listing does not name", "the Mini App's scope with a listing"])

    def test_each_string_of_battery_is_observed(self):
        self.check(
            [
                "a scope the listing does not name",
                "the whole tree with no row and no sweep",
            ]
        )

    def test_each_loop_step_of_battery_is_observed(self):
        self.check(["the Mini App's scope with a sweep of no files"])

    def test_each_operand_of_battery_is_observed(self):
        self.check(
            [
                "the whole tree with no row and no sweep",
                "rows that are no list and a sweep that is no object",
                "two sweeps",
                "the Mini App's scope with a sweep of no files",
            ]
        )


#: {case: ((text, start, regex_may_begin), the end `web_literal` gives)}.
WEB_LITERALS = {
    "a one-line string left open": (('"ab\ncd"', 0, False), 3),
    "a class holding a slash": (("/[/]/", 0, True), 5),
    "a regex": (("/a/", 0, True), 3),
    "a regex left open": (("/ab", 0, True), None),
    "a regex a line break ends": (("/a\n/", 0, True), None),
    "an escaped slash": (("/\\//", 0, True), 4),
    "a regex before a semicolon": (("/a/;", 0, True), 3),
    "a regex with a flag": (("/a/g", 0, True), 4),
    "a regex with three flags": (("/a/gim", 0, True), 6),
}


class TheWebLiteral(unittest.TestCase):
    def check(self, names):
        for name in examined("web literals", names):
            arguments, end = WEB_LITERALS[name]
            self.assertEqual(V.web_literal(*arguments), end, name)

    def test_each_flag_of_web_literal_is_observed(self):
        self.check(["a one-line string left open", "a regex", "a class holding a slash"])

    def test_each_integer_of_web_literal_is_observed(self):
        self.check(
            [
                "a class holding a slash",
                "an escaped slash",
                "a regex before a semicolon",
                "a regex",
                "a regex with three flags",
            ]
        )

    def test_each_operand_of_web_literal_is_observed(self):
        self.check(["a regex left open", "a regex a line break ends", "a regex"])
        self.check(["a class holding a slash"])

    def test_each_string_of_web_literal_is_observed(self):
        self.check(["a regex a line break ends", "an escaped slash", "a class holding a slash"])

    def test_each_loop_step_of_web_literal_is_observed(self):
        self.check(["an escaped slash"])

    def test_each_comparison_of_web_literal_is_observed(self):
        self.check(["a regex", "a regex with a flag", "a class holding a slash"])

    def test_each_negation_of_web_literal_is_observed(self):
        self.check(["a regex", "a class holding a slash"])


class TheExcusesBind(unittest.TestCase):
    def test_each_string_of_excuses_bind_is_observed(self):
        record = V.Record("c.json", 1, a_record(LIB, "replace + with -"))
        excuses = V.Excuses(tree(self, {LIB: LIB_TEXT}), [record], "rust")
        both = [
            V.Mutant(LIB, "replace + with -", (2, 7), (2, 8), f"{LIB}:2:7: replace + with -"),
            V.Mutant(LIB, "replace + with -", (2, 5), (2, 6), f"{LIB}:2:5: replace + with -"),
        ]
        failures = []
        excuses.bind(both, failures.append)
        self.assertEqual(
            failures,
            [
                f"AMBIGUOUS c.json record 1 ({LIB}: replace + with -): binds 2 listed mutants: "
                f"{LIB}:2:7: replace + with -; {LIB}:2:5: replace + with -"
            ],
        )
        self.assertEqual(excuses.valid, [])


#: {case: (words, the index `item_keyword` gives from 0)}.
ITEMS = {
    "a restricted visibility": (["pub", "(", "crate", ")", "fn"], 4),
    "a visibility left open": (["pub", "(", "crate"], None),
    "a plain visibility": (["pub", "fn"], 1),
    "a qualifier before brackets": (["async", "(", ")", "fn"], None),
    "an extern ABI": (["extern", '"C"', "fn"], 2),
    "an extern without an ABI": (["extern", "fn"], 1),
    "an extern at the end": (["extern"], None),
    "a qualifier before a string": (["unsafe", '"x"', "fn"], None),
}


class TheItemKeyword(unittest.TestCase):
    def check(self, names):
        for name in examined("items", names):
            words, index = ITEMS[name]
            self.assertEqual(V.item_keyword(words, 0), index, name)

    def test_each_comparison_of_item_keyword_is_observed(self):
        self.check(
            [
                "a restricted visibility",
                "a plain visibility",
                "a visibility left open",
                "an extern ABI",
            ]
        )

    def test_each_string_of_item_keyword_is_observed(self):
        self.check(["a restricted visibility", "an extern ABI", "an extern without an ABI"])

    def test_each_operand_of_item_keyword_is_observed(self):
        self.check(
            [
                "a qualifier before brackets",
                "a plain visibility",
                "a qualifier before a string",
                "an extern without an ABI",
                "an extern at the end",
            ]
        )

    def test_each_arithmetic_of_item_keyword_is_observed(self):
        self.check(["a restricted visibility", "an extern ABI"])

    def test_each_integer_of_item_keyword_is_observed(self):
        self.check(["a restricted visibility", "an extern ABI", "an extern at the end"])


#: A Mini App mutant with no status, which Stryker leaves Pending.
PENDING = {
    "mutatorName": "BlockStatement",
    "replacement": "{}",
    "location": {"start": {"line": 1, "column": 1}, "end": {"line": 1, "column": 3}},
}


class TheJudgeWeb(unittest.TestCase):
    def judged(self, files, records=None):
        """`judge_web` over a Stryker report of `files`, with `records` in its root: (what it
        said, its failures, its voids, what it examined)."""
        root = tree(self, dict(records or {}, **{"stryker.json": json.dumps({"files": files})}))
        plan = {"row_overlaps": {}, "classes": {"web": {"applies": True}}, "files": []}
        out = io.StringIO()
        verdict = V.Verdict("web")
        with contextlib.redirect_stdout(out):
            V.judge_web(verdict, plan, judge_args(root, stryker=str(root / "stryker.json")))
        return out.getvalue(), verdict.failures, verdict.voids, verdict.examined

    def excused(self):
        """A survivor the Mini App record binds in the report's own copy of the source."""
        mutants = [
            stryker("ArithmeticOperator", "a - b", "Survived", 28),
            stryker("ArithmeticOperator", "a / b", "Killed", 28),
        ]
        source = "export const f = (a, b) => a + b;\n"
        found = self.judged({"src/x.ts": {"source": source, "mutants": mutants}}, RECORDS)
        said = (
            "mutation: web: EQUIVALENT web/app/src/x.ts:1: ArithmeticOperator -> 'a - b': r (#1)\n"
            "mutation: web: Stryker examined 2 (Killed 1, NoCoverage 0, Survived 1, Timeout 0)\n"
            "mutation: web: survived 1: equivalent 1, unexplained 0\n"
        )
        self.assertEqual(found, (said, [], [], 2))

    def pending(self):
        never_run = "Pending web/app/src/x.ts:1: never run"
        said = (
            f"mutation: web: VOID {never_run}\n"
            "mutation: web: Stryker examined 0 (Killed 0, NoCoverage 0, Pending 1, Survived 0, "
            "Timeout 0)\n"
            "mutation: web: survived 0: equivalent 0, unexplained 0\n"
            f"mutation: web: VOID {NOTHING}\n"
        )
        self.assertEqual(
            self.judged({"src/x.ts": {"mutants": [PENDING]}}),
            (said, [], [never_run, NOTHING], 0),
        )

    def test_each_string_of_judge_web_is_observed(self):
        self.excused()
        self.pending()

    def test_each_integer_of_judge_web_is_observed(self):
        self.excused()
        self.pending()
        killed = [stryker("ArithmeticOperator", "a / b", "Killed", 28)]
        said = (
            "mutation: web: Stryker examined 1 (Killed 1, NoCoverage 0, Survived 0, Timeout 0)\n"
            "mutation: web: survived 0: equivalent 0, unexplained 0\n"
        )
        self.assertEqual(self.judged({"src/x.ts": {"mutants": killed}}), (said, [], [], 1))


class TheHeldTwice(unittest.TestCase):
    TWICE = [
        V.Record("c.json", 1, {"file": "f", "mutant": "m"}),
        V.Record("d.json", 2, {"file": "f", "mutant": "m"}),
    ]
    HELD = (
        ": held twice, by c.json record 1 (f: m) and d.json record 2 (f: m), so neither excuses it"
    )

    def test_each_integer_of_held_twice_is_observed(self):
        self.assertEqual(V.held_twice(self.TWICE), self.HELD)
        self.assertEqual(V.held_twice(self.TWICE[:1]), "")

    def test_each_return_value_of_held_twice_is_observed(self):
        self.assertEqual(V.held_twice(self.TWICE), self.HELD)


#: The plan.json of a not-applicable run.
NOT_APPLICABLE_PLAN = (
    json.dumps(
        {
            "base": "",
            "head": "",
            "files": [],
            "classes": {
                name: {"applies": False, "files": []}
                for name in ("rust", "web", "oracle", "scripts")
            },
            "rows": [],
            "row_overlaps": {},
            "stryker_mutate": [],
            "scope": {"decision": "not-applicable", "reason": "r"},
        },
        indent=2,
    )
    + "\n"
)
#: A diff that adds a line to a file of no class, changes a Rust line and adds a Mini App line.
PLAN_DIFF = (
    "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -0,0 +1 @@\n+x\n"
    f"diff --git a/{LIB} b/{LIB}\n--- a/{LIB}\n+++ b/{LIB}\n@@ -2 +2 @@\n-    a - b\n+    a + b\n"
    f"diff --git a/{WEB_FILE} b/{WEB_FILE}\n--- a/{WEB_FILE}\n+++ b/{WEB_FILE}\n@@ -0,0 +1 @@\n"
    "+export const a = 1;\n"
)
PLAN_TEXTS = {"README.md": "x\n", LIB: LIB_TEXT, WEB_FILE: "export const a = 1;\n"}
PLANNED = (
    [
        {
            "path": "README.md",
            "class": "other",
            "added": 1,
            "deleted": 0,
            "code": [],
            "quiet": [1],
            "test": [],
        },
        {
            "path": LIB,
            "class": "rust",
            "added": 1,
            "deleted": 1,
            "code": [2],
            "quiet": [],
            "test": [],
        },
        {
            "path": WEB_FILE,
            "class": "web",
            "added": 1,
            "deleted": 0,
            "code": [1],
            "quiet": [],
            "test": [],
        },
    ],
    ["src/x.ts"],
)


class ThePlanDiff(unittest.TestCase):
    def not_applicable(self, out):
        """(the plan's base, its head, the plan.json it wrote into `out`) of a not-applicable
        run."""
        plan = V.plan_diff(scratch(self), "b", "h", out, ("not-applicable", "r"))
        return plan.base, plan.head, (out / "plan.json").read_text(encoding="utf-8")

    def planned(self):
        """(the files, the Mini App files Stryker mutates) of `plan_diff` over `PLAN_DIFF`, git's
        answers stood in for and no rows file in the root."""

        def git(_root, *args):
            if args[0] == "rev-parse":
                return {"b^{commit}": "1111\n", "h^{commit}": "2222\n"}[args[-1]]
            if args[0] == "diff":
                return PLAN_DIFF
            self.assertEqual(args[0], "show")
            return PLAN_TEXTS[args[1].removeprefix("2222:")]

        with mock.patch.object(V, "git", git):
            plan = V.plan_diff(scratch(self), "b", "h", scratch(self) / "plan", ("diff", "r"))
        return plan.files, plan.stryker_mutate

    def test_each_flag_of_plan_diff_is_observed(self):
        out = scratch(self) / "a" / "b"
        self.assertEqual(self.not_applicable(out), ("", "", NOT_APPLICABLE_PLAN))

    def test_each_string_of_plan_diff_is_observed(self):
        out = scratch(self) / "plan"
        self.assertEqual(self.not_applicable(out), ("", "", NOT_APPLICABLE_PLAN))
        self.assertEqual(self.planned(), PLANNED)

    def test_each_comparison_of_plan_diff_is_observed(self):
        self.assertEqual(self.planned(), PLANNED)


if __name__ == "__main__":
    unittest.main()

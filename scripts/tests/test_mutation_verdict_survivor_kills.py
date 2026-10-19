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


if __name__ == "__main__":
    unittest.main()

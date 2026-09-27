#!/usr/bin/env python3
"""tdd-probe: test-driven development, judged on any repository root (SPEC-V2-2186 R3).

    python3 scripts/tdd-probe.py --root <repo> check <class>|all
    python3 scripts/tdd-probe.py --root <repo> list

Four classes:

  acceptance-has-a-test    every command in a SPEC's ```acceptance fence names a test that is in
                           the tree: `cargo test` (`-p`, `--test`, filters, `--exact`), `cargo
                           nextest run`, `python3 -m unittest` (`discover -s -p -k`, dotted
                           names), `pytest` (paths, `::` node ids, `-k` expressions), `vitest`
                           and `jest` (paths, `-t`) and `node --test`. Nothing is run.
  red-first-recorded       every SPEC with acceptance commands has a record,
                           `<red_first>/<SPEC id>.md`, whose ```red-first fence gives each
                           criterion a red line and a green line at two commits, or discloses it
                           `not red` with the reason
  absence-only-assertions  no test asserts only absences (`assertNotIn`, `is None`, `.not.`,
                           `assert!(!x)`, `is_empty()`): a mutant that deletes the behaviour
                           passes such a test. A heuristic: it reads assertion shapes.
  examined-counts          every test file that enumerates (a glob, a directory walk, a
                           `git ls-files`) calls the examined contract (`examined(...)`), which
                           reports the count and refuses a population of zero

The record grammar, one line per fact:

    A1: red at <sha>: <the failure observed>
    A1: green at <sha>
    A2: not red: <why this criterion could not be red first>

Tests are the files `test_globs` matches: Python (`test_*.py`, `*_test.py`), TypeScript and
JavaScript (`*.test.ts`, `*.spec.tsx`, ...), and Rust files holding a `#[test]` function.
"""

from __future__ import annotations

import ast
import fnmatch
import json
import re
import shlex
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import methodology_probe as mp  # noqa: E402

PYTHON = (".py",)
SCRIPT = (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts")
RUST = (".rs",)
RED = re.compile(rf"^({mp.CRITERION})\s*:\s*red at ([0-9a-f]{{7,40}})\s*:\s*(\S.*)$")
GREEN = re.compile(rf"^({mp.CRITERION})\s*:\s*green at ([0-9a-f]{{7,40}})\s*$")
NOT_RED = re.compile(rf"^({mp.CRITERION})\s*:\s*not red\s*:\s*(\S.*)$")
RUST_TEST = re.compile(
    r"#\[(?:[\w:]+::)?test\b[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*"
    r"(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+([A-Za-z_]\w*)"
)
SCRIPT_TEST = re.compile(
    r"\b(?:it|test)(?:\.(?:only|skip|concurrent|fails))*\s*\(\s*(['\"`])((?:\\.|(?!\1).)*?)\1"
)
SCRIPT_TITLE = re.compile(
    r"\b(?:it|test|describe)(?:\.(?:only|skip|concurrent|fails))*\s*\(\s*(['\"`])"
    r"((?:\\.|(?!\1).)*?)\1"
)
ENUMERATION_VERBS = ("glob", "walk", "iterdir", "scandir", "listdir")
GIT_LISTINGS = ("ls-files", "ls-tree")
SCRIPT_ENUMERATION = re.compile(
    r"\b(readdirSync|readdir|opendirSync|opendir|globSync|glob|fastGlob|walkSync|walk)\s*\("
)
RUST_ENUMERATION = re.compile(
    r"\b(read_dir|glob|walk_dir|WalkDir|walk)\s*(?:::\s*new\s*)?\("
)

# Python assertions that hold only when something is absent, false or empty.
ABSENT_METHODS = frozenset(
    {
        "assertNotIn",
        "assertIsNone",
        "assertFalse",
        "assertNotEqual",
        "assertIsNot",
        "assertNotIsInstance",
        "assertNotRegex",
        "assertNotAlmostEqual",
        "assertNoLogs",
    }
)
EQUALITY_METHODS = frozenset(
    {
        "assertEqual",
        "assertListEqual",
        "assertDictEqual",
        "assertSetEqual",
        "assertTupleEqual",
        "assertSequenceEqual",
        "assertCountEqual",
    }
)
PRESENT_METHODS = frozenset(
    {
        "assertTrue",
        "assertIn",
        "assertIs",
        "assertIsNotNone",
        "assertIsInstance",
        "assertRegex",
        "assertGreater",
        "assertGreaterEqual",
        "assertLess",
        "assertLessEqual",
        "assertAlmostEqual",
        "assertRaises",
        "assertRaisesRegex",
        "assertWarns",
        "assertWarnsRegex",
        "assertLogs",
        "assertMultiLineEqual",
        "raises",
        "fail",
    }
)
SCRIPT_ABSENT_MATCHERS = frozenset(
    {"toBeUndefined", "toBeNull", "toBeFalsy", "toBeNaN"}
)
SCRIPT_EMPTY = frozenset(
    {"false", "0", "null", "undefined", "''", '""', "``", "[]", "{}"}
)
NODE_ABSENT = frozenset(
    {
        "notEqual",
        "notStrictEqual",
        "notDeepEqual",
        "notDeepStrictEqual",
        "doesNotThrow",
        "doesNotReject",
        "doesNotMatch",
    }
)
RUST_EMPTY = frozenset(
    {"None", "false", "0", '""', "vec![]", "[]", "Vec::new()", "String::new()"}
)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def balanced(text: str, open_at: int) -> int:
    """The index just past the bracket closing the one at `open_at` (on blanked text)."""
    pairs = {"(": ")", "[": "]", "{": "}"}
    stack = []
    for index in range(open_at, len(text)):
        char = text[index]
        if char in pairs:
            stack.append(pairs[char])
        elif stack and char == stack[-1]:
            stack.pop()
            if not stack:
                return index + 1
    return len(text)


def top_level_split(text: str) -> list[str]:
    """`text` split at the commas outside every bracket."""
    parts, depth, current = [], 0, []
    for char in text:
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
        if char == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(char)
    if "".join(current).strip():
        parts.append("".join(current).strip())
    return parts


# ------------------------------------------------------------------------------ acceptance


class Resolver:
    """Decides, statically, whether an acceptance command names a test present in the tree."""

    def __init__(self, context: mp.Context):
        self.root = context.root
        self.context = context
        self._workspace: dict[str, Path] | None = None

    def resolve(self, command: str) -> tuple[bool | None, str]:
        """(True, what) when it names a test, (False, why) when it names none, None: no shape."""
        try:
            words = shlex.split(command)
        except ValueError as error:
            return None, str(error)
        words = self.strip_environment(words)
        if not words:
            return None, "empty"
        head = words[0]
        if re.fullmatch(r"python3?(\.\d+)?", head) and words[1:3] == ["-m", "unittest"]:
            return self.unittest(words[3:])
        if re.fullmatch(r"python3?(\.\d+)?", head) and words[1:3] == ["-m", "pytest"]:
            return self.pytest(words[3:])
        if head in ("pytest", "py.test"):
            return self.pytest(words[1:])
        if head == "cargo" and words[1:2] == ["test"]:
            return self.cargo(words[2:])
        if head == "cargo" and words[1:3] == ["nextest", "run"]:
            return self.cargo(words[3:])
        runner, args = self.script_runner(words)
        if runner in ("vitest", "jest"):
            return self.script(runner, args)
        if runner == "node" and "--test" in args:
            return self.node_test(args)
        return None, "no known runner"

    @staticmethod
    def strip_environment(words: list[str]) -> list[str]:
        while words and re.fullmatch(r"[A-Za-z_]\w*=.*", words[0]):
            words = words[1:]
        if words[:1] == ["env"]:
            words = words[1:]
            while words and (words[0].startswith("-") or "=" in words[0]):
                takes_value = words[0] in ("-u", "--unset", "-C", "--chdir")
                words = words[2:] if takes_value else words[1:]
        return words

    # -- python

    @staticmethod
    def python_tests(path: Path, unittest_only: bool) -> list[tuple[str, str]]:
        """(class or "", test name) for every test in a Python file."""
        try:
            tree = ast.parse(read(path))
        except SyntaxError:
            return []
        found = []
        for node in tree.body:
            if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef):
                if not unittest_only and node.name.startswith("test"):
                    found.append(("", node.name))
            elif isinstance(node, ast.ClassDef):
                collected = bool(node.bases) or (
                    not unittest_only and node.name.startswith("Test")
                )
                if not collected:
                    continue
                for item in node.body:
                    if isinstance(
                        item, ast.FunctionDef | ast.AsyncFunctionDef
                    ) and item.name.startswith("test"):
                        found.append((node.name, item.name))
        return found

    def unittest(self, args: list[str]) -> tuple[bool | None, str]:
        patterns, rest = [], []
        index = 0
        while index < len(args):
            word = args[index]
            if word == "-k" and index + 1 < len(args):
                patterns.append(args[index + 1])
                index += 2
                continue
            rest.append(word)
            index += 1
        if rest[:1] != ["discover"]:
            names = [word for word in rest if not word.startswith("-")]
            if not names:
                rest = ["discover"]
            else:
                return self.unittest_names(names, patterns)
        options = {"-s": ".", "-p": "test*.py", "-t": None}
        positional = []
        index = 1
        while index < len(rest):
            word = rest[index]
            long = {
                "--start-directory": "-s",
                "--pattern": "-p",
                "--top-level-directory": "-t",
            }
            key = long.get(word, word)
            if key in options and index + 1 < len(rest):
                options[key] = rest[index + 1]
                index += 2
                continue
            if not word.startswith("-"):
                positional.append(word)
            index += 1
        for key, value in zip(("-s", "-p", "-t"), positional, strict=False):
            options[key] = value
        start = self.root / options["-s"]
        if not start.is_dir():
            return False, f"no directory {options['-s']}"
        top = self.root / options["-t"] if options["-t"] else start
        files = [
            path
            for path in mp.files_under(self.root, start, PYTHON)
            if fnmatch.fnmatch(path.name, options["-p"])
        ]
        if not files:
            return False, f"no file matches {options['-p']} under {options['-s']}"
        ids = []
        for path in files:
            module = ".".join(path.relative_to(top).with_suffix("").parts)
            for cls, name in self.python_tests(path, unittest_only=True):
                ids.append(f"{module}.{cls}.{name}")
        return self.select(ids, patterns, substring_case=True)

    @staticmethod
    def select(
        ids: list[str], patterns: list[str], substring_case: bool
    ) -> tuple[bool, str]:
        if not ids:
            return False, "the files hold no test"
        if not patterns:
            return True, f"{len(ids)} test(s)"
        chosen = []
        for test in ids:
            for pattern in patterns:
                if "*" in pattern:
                    hit = fnmatch.fnmatchcase(test, pattern)
                elif substring_case:
                    hit = pattern in test
                else:
                    hit = pattern.lower() in test.lower()
                if hit:
                    chosen.append(test)
                    break
        if not chosen:
            return False, f"no test matches -k {' '.join(patterns)}"
        return True, f"{len(chosen)} test(s)"

    def unittest_names(self, names: list[str], patterns: list[str]) -> tuple[bool, str]:
        ids = []
        for name in names:
            if name.endswith(".py") or "/" in name:
                path = self.root / name
                if not path.is_file():
                    return False, f"no file {name}"
                module = ".".join(Path(name).with_suffix("").parts)
                ids.extend(
                    f"{module}.{c}.{t}" for c, t in self.python_tests(path, True)
                )
                continue
            parts = name.split(".")
            for size in range(len(parts), 0, -1):
                path = self.root.joinpath(*parts[:size]).with_suffix(".py")
                if path.is_file():
                    module = ".".join(parts[:size])
                    wanted = ".".join(parts[size:])
                    tests = [
                        f"{module}.{c}.{t}" for c, t in self.python_tests(path, True)
                    ]
                    tests = [
                        t
                        for t in tests
                        if not wanted or f"{t}.".startswith(f"{module}.{wanted}.")
                    ]
                    if not tests:
                        return False, f"{name} names no test"
                    ids.extend(tests)
                    break
            else:
                return False, f"no module for {name}"
        return self.select(ids, patterns, substring_case=True)

    def pytest(self, args: list[str]) -> tuple[bool, str]:
        valued = {"-k", "-m", "-p", "-c", "--rootdir", "-n", "--maxfail", "--tb", "-W",
                  "--deselect", "--ignore", "--junitxml", "-o", "--cov", "--durations"}  # fmt: skip
        expression, targets, index = None, [], 0
        while index < len(args):
            word = args[index]
            if word in valued and index + 1 < len(args):
                if word == "-k":
                    expression = args[index + 1]
                index += 2
                continue
            if word.startswith("-k") and len(word) > 2:
                expression = word[2:]
            elif not word.startswith("-"):
                targets.append(word)
            index += 1
        tests = []
        for target in targets or ["."]:
            path_text, *node = target.split("::")
            path = self.root / path_text
            if path.is_dir():
                files = [
                    p
                    for p in mp.files_under(self.root, path, PYTHON)
                    if fnmatch.fnmatch(p.name, "test_*.py")
                    or fnmatch.fnmatch(p.name, "*_test.py")
                ]
            elif path.is_file():
                files = [path]
            else:
                return False, f"no path {path_text}"
            for file in files:
                for cls, name in self.python_tests(file, unittest_only=False):
                    parts = [cls, name] if cls else [name]
                    stripped = [re.sub(r"\[.*\]$", "", part) for part in node]
                    if stripped and parts[: len(stripped)] != stripped:
                        continue
                    tests.append(" ".join([file.stem, *parts]))
        if not tests:
            return False, "the paths hold no test"
        if expression is None:
            return True, f"{len(tests)} test(s)"
        chosen = [test for test in tests if self.keyword(expression, test)]
        if not chosen:
            return False, f"no test matches -k {expression}"
        return True, f"{len(chosen)} test(s)"

    @staticmethod
    def keyword(expression: str, name: str) -> bool:
        """pytest's `-k`: identifiers are case-insensitive substrings, joined by and/or/not."""
        tokens = re.findall(r"\(|\)|[\w.\[\]-]+", expression)
        rendered = []
        for token in tokens:
            if token in ("and", "or", "not", "(", ")"):
                rendered.append(token)
            else:
                rendered.append(str(token.lower() in name.lower()))
        try:
            return bool(eval(" ".join(rendered), {"__builtins__": {}}, {}))  # noqa: S307
        except SyntaxError:
            return False

    # -- cargo

    def workspace(self) -> dict[str, Path]:
        if self._workspace is None:
            self._workspace = mp.workspace(self.root)
        return self._workspace

    def cargo(self, args: list[str]) -> tuple[bool, str]:
        cargo_args, test_args = args, []
        if "--" in args:
            split = args.index("--")
            cargo_args, test_args = args[:split], args[split + 1 :]
        valued = {"-p", "--package", "--manifest-path", "--test", "--bin", "--features",
                  "-F", "--target", "--exclude", "-j", "--jobs", "--profile", "--example",
                  "--bench", "-E", "--filterset", "--target-dir", "--color"}  # fmt: skip
        packages, targets, filters, lib, expressions = [], [], [], False, []
        index = 0
        while index < len(cargo_args):
            word = cargo_args[index]
            if word in valued and index + 1 < len(cargo_args):
                value = cargo_args[index + 1]
                if word in ("-p", "--package"):
                    packages.append(value)
                elif word == "--test":
                    targets.append(value)
                elif word in ("-E", "--filterset"):
                    expressions.append(value)
                index += 2
                continue
            if word == "--lib":
                lib = True
            elif not word.startswith("-"):
                filters.append(word)
            index += 1
        exact = "--exact" in test_args
        index = 0
        while index < len(test_args):
            word = test_args[index]
            if word in ("--skip", "--test-threads", "--format", "--color", "-Z"):
                index += 2
                continue
            if not word.startswith("-"):
                filters.append(word)
            index += 1
        for expression in expressions:
            filters.extend(re.findall(r"test\(\s*[=~]?([\w:]+)\s*\)", expression))
        crates = self.workspace()
        if packages:
            missing = [name for name in packages if name not in crates]
            if missing:
                return False, f"no workspace package {', '.join(missing)}"
            dirs = [crates[name] for name in packages]
        else:
            dirs = list(crates.values()) or [self.root]
        files: list[Path] = []
        for crate in dirs:
            if targets:
                for target in targets:
                    found = [crate / "tests" / f"{target}.rs"] + mp.files_under(
                        self.root, crate / "tests" / target, RUST
                    )
                    found = [path for path in found if path.is_file()]
                    if not found:
                        return False, f"{crate.name} has no test target {target}"
                    files.extend(found)
            elif lib:
                files.extend(mp.files_under(self.root, crate / "src", RUST))
            else:
                files.extend(mp.files_under(self.root, crate / "src", RUST))
                files.extend(mp.files_under(self.root, crate / "tests", RUST))
        names = [name for path in files for _, name, _ in rust_tests(read(path))]
        if not names:
            return False, "the targets hold no #[test] function"
        for text in filters:
            leaf = text.rsplit("::", 1)[-1]
            hit = [n for n in names if (n == leaf if exact else leaf in n)]
            if not hit:
                return False, f"no test matches {text}"
        return True, f"{len(names)} test(s) in scope"

    # -- scripts

    def script_runner(self, words: list[str]) -> tuple[str | None, list[str]]:
        while words and words[0] in ("npx", "bunx", "pnpm", "yarn", "bun"):
            words = words[1:]
            if words and words[0] in ("exec", "dlx"):
                words = words[1:]
        if words[:1] == ["npm"]:
            args = words[words.index("--") + 1 :] if "--" in words else []
            if words[1:2] == ["test"] or words[1:3] == ["run", "test"]:
                script = self.package_test_script()
                if script:
                    return script[0], script[1:] + args
            return None, []
        if not words:
            return None, []
        return words[0], words[1:]

    def package_test_script(self) -> list[str]:
        manifest = self.root / "package.json"
        if not manifest.is_file():
            return []
        try:
            data = json.loads(read(manifest))
        except json.JSONDecodeError:
            return []
        script = (data.get("scripts") or {}).get("test", "")
        return shlex.split(script) if script else []

    def script_files(self) -> list[Path]:
        globs = self.context.section("tdd")["test_globs"]
        return [p for p in mp.files_matching(self.root, globs) if p.suffix in SCRIPT]

    def script(self, runner: str, args: list[str]) -> tuple[bool, str]:
        if runner == "vitest" and args[:1] and args[0] in ("run", "watch", "related"):
            args = args[1:]
        valued = {"-c", "--config", "--project", "--dir", "-r", "--root", "--reporter",
                  "--environment", "--shard"}  # fmt: skip
        name, filters, index = None, [], 0
        while index < len(args):
            word = args[index]
            if word in ("-t", "--testNamePattern") and index + 1 < len(args):
                name = args[index + 1]
                index += 2
                continue
            if word.startswith("--testNamePattern="):
                name = word.split("=", 1)[1]
            elif word in valued:
                index += 2
                continue
            elif not word.startswith("-"):
                filters.append(word)
            index += 1
        files = [
            path
            for path in self.script_files()
            if all(self.path_hit(mp.rel(self.root, path), text) for text in filters)
        ]
        if not files:
            return (
                False,
                f"no test file matches {' '.join(filters) or 'the test globs'}",
            )
        if name is None:
            return True, f"{len(files)} test file(s)"
        titles = []
        for path in files:
            titles.extend(match.group(2) for match in SCRIPT_TITLE.finditer(read(path)))
        joined = [
            " ".join(titles[i : j + 1])
            for i in range(len(titles))
            for j in range(i, len(titles))
        ]
        if not any(self.title_hit(name, title) for title in titles + joined):
            return False, f"no test title matches -t {name}"
        return True, "a test title matches"

    @staticmethod
    def path_hit(relative: str, text: str) -> bool:
        if text in relative:
            return True
        try:
            return re.search(text, relative) is not None
        except re.error:
            return False

    @staticmethod
    def title_hit(pattern: str, title: str) -> bool:
        if pattern in title:
            return True
        try:
            return re.search(pattern, title) is not None
        except re.error:
            return False

    def node_test(self, args: list[str]) -> tuple[bool, str]:
        paths = [word for word in args if not word.startswith("-")]
        files = []
        for text in paths:
            files.extend(p for p in [self.root / text] if p.is_file())
        if not files:
            return False, "node --test names no test file"
        if not any(SCRIPT_TEST.search(read(path)) for path in files):
            return False, "the files hold no test"
        return True, f"{len(files)} test file(s)"


def spec_commands(context: mp.Context):
    judged, notes, _, _ = mp.judged_specs(context)
    return judged, notes


def acceptance_has_a_test(context: mp.Context) -> mp.Result:
    judged, notes = spec_commands(context)
    result = mp.Result(0, "command(s)", notes=notes)
    result.void_reason = "no judged SPEC states a fenced acceptance command"
    resolver = Resolver(context)
    for doc in judged:
        for _, criterion, command in mp.acceptance(read(doc.path)).lines:
            result.examined += 1
            found, why = resolver.resolve(command)
            if found is None:
                result.findings.append(
                    f"{doc.rel}: {criterion} is in no shape this probe can resolve: {command}"
                )
            elif not found:
                result.findings.append(
                    f"{doc.rel}: {criterion} selects no test: {command} ({why})"
                )
    return result


# ------------------------------------------------------------------------------ red first


def red_first_recorded(context: mp.Context) -> mp.Result:
    judged, notes = spec_commands(context)
    directory = context.section("tdd")["red_first"]
    result = mp.Result(0, "SPEC(s)", notes=list(notes))
    result.void_reason = "no judged SPEC states a fenced acceptance command"
    disclosed = 0
    for doc in judged:
        criteria = {c for _, c, _ in mp.acceptance(read(doc.path)).lines}
        if not criteria:
            continue
        result.examined += 1
        record = context.root / directory / f"{doc.ident}.md"
        where = mp.rel(context.root, record)
        if not record.is_file():
            result.findings.append(f"{doc.rel}: no red-first record at {where}")
            continue
        chosen = [b for b in mp.blocks(read(record)) if b.info == "red-first"]
        if not chosen:
            result.findings.append(f"{where}: no ```red-first fence")
            continue
        reds: dict[str, str] = {}
        greens: dict[str, str] = {}
        nots: dict[str, str] = {}
        for number, line in (line for block in chosen for line in block.lines):
            text = line.strip()
            if not text:
                continue
            for pattern, into in ((RED, reds), (GREEN, greens), (NOT_RED, nots)):
                matched = pattern.match(text)
                if matched:
                    if matched.group(1) in into:
                        result.findings.append(
                            f"{where}:{number}: {matched.group(1)} is recorded twice"
                        )
                    into[matched.group(1)] = matched.group(2)
                    break
            else:
                result.findings.append(
                    f"{where}:{number}: not a red-first line: {text}"
                )
        for criterion in sorted(set(reds) | set(greens) | set(nots)):
            if criterion not in criteria:
                result.findings.append(
                    f"{where}: {criterion} is not a criterion of {doc.ident}"
                )
        for criterion in sorted(criteria):
            red, green = reds.get(criterion), greens.get(criterion)
            if criterion in nots:
                disclosed += 1
                if red or green:
                    result.findings.append(
                        f"{where}: {criterion} is disclosed not red and also recorded"
                    )
            elif red is None and green is None:
                result.findings.append(
                    f"{where}: {criterion} is neither recorded red nor disclosed not red"
                )
            elif green is None:
                result.findings.append(
                    f"{where}: {criterion} is recorded red and never green"
                )
            elif red is None:
                result.findings.append(
                    f"{where}: {criterion} is recorded green and never red"
                )
            elif red.startswith(green) or green.startswith(red):
                result.findings.append(
                    f"{where}: {criterion} is red and green at one commit, {red}"
                )
    if disclosed:
        result.notes.append(f"{disclosed} disclosed not red")
    return result


# ------------------------------------------------------------------------------ assertions


def rust_tests(text: str) -> list[tuple[int, str, str]]:
    """(line, name, body) for every `#[test]`-like function; the body is blanked text."""
    code = mp.blank_out(text, rust=True, strings=True)
    found = []
    for matched in RUST_TEST.finditer(code):
        brace = code.find("{", matched.end())
        if brace < 0:
            continue
        body = code[brace : mp_balanced(code, brace)]
        marker = "should_panic" if "should_panic" in matched.group(0) else ""
        found.append(
            (mp.line_of(code, matched.start(1)), matched.group(1), marker + body)
        )
    return found


def mp_balanced(code: str, brace: int) -> int:
    return balanced(code, brace)


def is_empty_python(node: ast.AST) -> bool:
    if isinstance(node, ast.List | ast.Tuple | ast.Set) and not node.elts:
        return True
    if isinstance(node, ast.Dict) and not node.keys:
        return True
    if isinstance(node, ast.Constant) and node.value in (None, False, 0, "", b""):
        return node.value is not True
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
        return node.func.id in ("set", "list", "dict", "tuple") and not node.args
    return False


def python_assertion(node: ast.AST) -> tuple[str, str] | None:
    """(kind, label) for one assertion node: kind `absent` or `present`; None when not one."""
    if isinstance(node, ast.Assert):
        test = node.test
        if isinstance(test, ast.UnaryOp) and isinstance(test.op, ast.Not):
            return "absent", "assert not"
        if isinstance(test, ast.Compare) and len(test.ops) == 1:
            op, right = test.ops[0], test.comparators[0]
            if isinstance(op, ast.NotIn):
                return "absent", "assert not in"
            if isinstance(op, ast.NotEq):
                return "absent", "assert !="
            if isinstance(op, ast.Is) and is_empty_python(right):
                return "absent", "assert is None"
            if isinstance(op, ast.Eq) and (
                is_empty_python(right) or is_empty_python(test.left)
            ):
                return "absent", "assert == empty"
        return "present", "assert"
    if not isinstance(node, ast.Call):
        return None
    func = node.func
    name = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")
    if name in ABSENT_METHODS:
        return "absent", name
    if name in EQUALITY_METHODS:
        if any(is_empty_python(arg) for arg in node.args[:2]):
            return "absent", name
        sizes = [
            a
            for a in node.args[:2]
            if isinstance(a, ast.Call) and getattr(a.func, "id", "") == "len"
        ]
        if sizes and any(
            isinstance(a, ast.Constant) and a.value == 0 for a in node.args[:2]
        ):
            return "absent", name
        return "present", name
    if name == "assertTrue" and node.args:
        first = node.args[0]
        if isinstance(first, ast.UnaryOp) and isinstance(first.op, ast.Not):
            return "absent", name
    if name in PRESENT_METHODS:
        return "present", name
    return None


def python_findings(path: Path, relative: str) -> tuple[int, list[str]]:
    tree = ast.parse(read(path))
    tests = [
        node
        for node in ast.walk(tree)
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef)
        and node.name.startswith("test")
    ]
    findings = []
    for test in tests:
        kinds = [python_assertion(node) for node in ast.walk(test)]
        kinds = [kind for kind in kinds if kind is not None]
        if kinds and all(kind == "absent" for kind, _ in kinds):
            labels = list(dict.fromkeys(label for _, label in kinds))
            findings.append(
                f"{relative}:{test.lineno}: {test.name} asserts only absences ({', '.join(labels)})"
            )
    return len(tests), findings


def script_assertions(body: str) -> list[tuple[str, str]]:
    found = []
    for matched in re.finditer(r"\bexpect\s*\(", body):
        close = balanced(body, matched.end() - 1)
        chain = re.match(
            r"\s*((?:\.\s*(?:not|resolves|rejects)\s*)*)\.\s*(\w+)\s*\(", body[close:]
        )
        if chain is None:
            continue
        negated = "not" in chain.group(1)
        matcher = chain.group(2)
        open_at = close + chain.end() - 1
        argument = body[open_at + 1 : balanced(body, open_at) - 1].strip()
        label = (".not." if negated else ".") + matcher
        empty = argument in SCRIPT_EMPTY or (
            matcher == "toHaveLength" and argument == "0"
        )
        absent = (
            negated
            or matcher in SCRIPT_ABSENT_MATCHERS
            or (
                matcher in ("toBe", "toEqual", "toStrictEqual", "toHaveLength")
                and empty
            )
        )
        found.append(("absent" if absent else "present", label))
    for matched in re.finditer(r"\bassert(?:\.(\w+))?\s*\(", body):
        name = matched.group(1) or "ok"
        open_at = matched.end() - 1
        argument = body[open_at + 1 : balanced(body, open_at) - 1].strip()
        parts = top_level_split(argument)
        absent = name in NODE_ABSENT or (
            name == "ok" and argument.startswith("!")
        ) or (name in ("equal", "strictEqual", "deepEqual", "deepStrictEqual")
              and len(parts) > 1 and parts[1] in SCRIPT_EMPTY)  # fmt: skip
        found.append(("absent" if absent else "present", f"assert.{name}"))
    return found


def script_findings(path: Path, relative: str) -> tuple[int, list[str]]:
    text = read(path)
    plain = mp.blank_out(text, rust=False, strings=False)
    code = mp.blank_out(text, rust=False, strings=True)
    findings = []
    tests = 0
    for matched in SCRIPT_TEST.finditer(plain):
        tests += 1
        brace = code.find("{", matched.end())
        if brace < 0:
            continue
        body = code[brace : balanced(code, brace)]
        kinds = script_assertions(body)
        if kinds and all(kind == "absent" for kind, _ in kinds):
            labels = list(dict.fromkeys(label for _, label in kinds))
            findings.append(
                f"{relative}:{mp.line_of(plain, matched.start())}: {matched.group(2)} "
                f"asserts only absences ({', '.join(labels)})"
            )
    return tests, findings


def rust_assertions(body: str) -> list[tuple[str, str]]:
    found = []
    if body.startswith("should_panic"):
        found.append(("present", "should_panic"))
    macro = re.compile(r"\b(debug_)?(assert|assert_eq|assert_ne)!\s*\(")
    for matched in macro.finditer(body):
        open_at = matched.end() - 1
        parts = top_level_split(body[open_at + 1 : balanced(body, open_at) - 1])
        kind = matched.group(2)
        if kind == "assert_ne":
            found.append(("absent", "assert_ne!"))
        elif kind == "assert_eq":
            empty = any(re.sub(r"\s+", "", part) in RUST_EMPTY for part in parts[:2])
            sized = any(
                part.replace(" ", "").endswith(".len()") for part in parts[:2]
            ) and any(part.strip() == "0" for part in parts[:2])
            label = "assert_eq! against an empty value"
            found.append(
                ("absent", label) if empty or sized else ("present", "assert_eq!")
            )
        else:
            first = parts[0] if parts else ""
            compact = first.replace(" ", "")
            if compact.startswith("!"):
                found.append(("absent", "assert!(!..)"))
            elif compact.endswith((".is_none()", ".is_empty()")):
                found.append(("absent", "." + compact.rsplit(".", 1)[-1]))
            else:
                found.append(("present", "assert!"))
    return found


def rust_findings(path: Path, relative: str) -> tuple[int, list[str]]:
    findings = []
    tests = rust_tests(read(path))
    for line, name, body in tests:
        kinds = rust_assertions(body)
        if kinds and all(kind == "absent" for kind, _ in kinds):
            labels = list(dict.fromkeys(label for _, label in kinds))
            findings.append(
                f"{relative}:{line}: {name} asserts only absences ({', '.join(labels)})"
            )
    return len(tests), findings


def test_files(context: mp.Context) -> list[Path]:
    globs = context.section("tdd")["test_globs"]
    return [
        p
        for p in mp.files_matching(context.root, globs)
        if p.suffix in PYTHON + SCRIPT + RUST
    ]


def absence_only_assertions(context: mp.Context) -> mp.Result:
    result = mp.Result(0, "test(s)")
    files = 0
    for path in test_files(context):
        relative = mp.rel(context.root, path)
        try:
            if path.suffix in PYTHON:
                count, findings = python_findings(path, relative)
            elif path.suffix in SCRIPT:
                count, findings = script_findings(path, relative)
            else:
                count, findings = rust_findings(path, relative)
        except SyntaxError as error:
            result.findings.append(f"{relative}: does not parse: {error.msg}")
            continue
        if count:
            files += 1
        result.examined += count
        result.findings.extend(findings)
    result.unit = f"test(s) in {files} file(s)"
    result.void_reason = "no test matches the test globs"
    return result


# ------------------------------------------------------------------------------ examined


def enumeration_sites(path: Path) -> tuple[bool, list[tuple[int, str]], bool]:
    """(is a test file, where it enumerates, whether it calls the examined contract)."""
    text = read(path)
    if path.suffix in PYTHON:
        tree = ast.parse(text)
        sites, contract = [], False
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            func = node.func
            name = (
                func.attr
                if isinstance(func, ast.Attribute)
                else getattr(func, "id", "")
            )
            if name in EXAMINED_CALLS:
                contract = True
            if any(verb in name for verb in ENUMERATION_VERBS):
                sites.append((node.lineno, name))
            elif any(g in ast.unparse(arg) for arg in node.args for g in GIT_LISTINGS):
                sites.append(
                    (
                        node.lineno,
                        "git "
                        + next(
                            g
                            for g in GIT_LISTINGS
                            if any(g in ast.unparse(a) for a in node.args)
                        ),
                    )
                )
        return True, sorted(sites), contract
    rust = path.suffix in RUST
    if rust and not rust_tests(text):
        return False, [], False
    code = mp.blank_out(text, rust=rust, strings=True)
    plain = mp.blank_out(text, rust=rust, strings=False)
    pattern = RUST_ENUMERATION if rust else SCRIPT_ENUMERATION
    sites = [(mp.line_of(code, m.start()), m.group(1)) for m in pattern.finditer(code)]
    for listing in GIT_LISTINGS:
        for matched in re.finditer(re.escape(listing), plain):
            sites.append((mp.line_of(plain, matched.start()), f"git {listing}"))
    names = "|".join(re.escape(name) for name in EXAMINED_CALLS)
    contract = bool(names) and re.search(rf"\b(?:{names})\s*!?\s*\(", code) is not None
    return True, sorted(sites), contract


EXAMINED_CALLS: list[str] = []


def examined_counts(context: mp.Context) -> mp.Result:
    EXAMINED_CALLS[:] = context.section("tdd")["examined_calls"]
    result = mp.Result(0, "test file(s)")
    enumerating = 0
    for path in test_files(context):
        relative = mp.rel(context.root, path)
        try:
            is_test, sites, contract = enumeration_sites(path)
        except SyntaxError as error:
            result.findings.append(f"{relative}: does not parse: {error.msg}")
            continue
        if not is_test:
            continue
        result.examined += 1
        if sites:
            enumerating += 1
            if not contract:
                line, name = sites[0]
                result.findings.append(
                    f"{relative}:{line}: enumerates ({name}) and never reports how many it examined"
                )
    result.unit = f"test file(s), {enumerating} enumerating"
    result.void_reason = "no test file matches the test globs"
    return result


CLASSES = {
    "acceptance-has-a-test": (
        "every fenced acceptance command names a test present in the tree",
        acceptance_has_a_test,
    ),
    "red-first-recorded": (
        "every SPEC's criteria are recorded red then green, or disclosed not red",
        red_first_recorded,
    ),
    "absence-only-assertions": (
        "no test asserts only that something is absent, false or empty",
        absence_only_assertions,
    ),
    "examined-counts": (
        "every enumerating test reports its examined count through the contract",
        examined_counts,
    ),
}


if __name__ == "__main__":
    sys.exit(mp.run("TDD", "tdd", CLASSES))

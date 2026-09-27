"""Tests leave no temporary file or directory behind (SPEC-030 R2 to R4).

The predecessor pinned pytest's `tmp_path_retention_policy = "failed"`, so green runs left nothing on
the small shared host. DeckStreak's Python tests are `unittest`, so this lint holds the line
instead: it reads every test file of the repository (the Rust integration tests, the repository's
Python tests and the Mini App's tests), and refuses, naming the file and the line, each form that
keeps what it creates after the test ends. The accepted forms remove what they create on drop or
exit: `tempfile::TempDir` and `NamedTempFile` dropped at scope end, Python's
`tempfile.TemporaryDirectory`, and a `mkdtemp` whose removal the same function registers.

The lint reads text, not paths: a directory built from a variable escapes it, so it refuses the
leaking APIs themselves rather than their paths (SPEC-030 section 6).
"""

import ast
import re
import unittest
from pathlib import Path

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "temp-hygiene"
# R2: every test file of the repository. The planted fixtures sit under FIXTURES with a `.fixture`
# suffix, so no pattern here matches them, and the census drops that directory as well (R4).
CENSUS = (
    "crates/*/tests/**/*.rs",
    "scripts/tests/*.py",
    "tools/parity-oracle/test_*.py",
    "web/app/src/**/*.test.ts",
    "web/app/tests/**/*.ts",
)
LANGUAGES = {".rs": "rust", ".py": "python", ".ts": "typescript"}

# Rust: the calls that keep a temporary directory or file past drop, and the shared directory.
RUST_CALLS = (
    (
        re.compile(r"(?:\.|::)\s*into_path\s*\("),
        "TempDir::into_path keeps the directory",
    ),
    (
        re.compile(r"(?:\.|::)\s*keep\s*\("),
        "keep() persists a TempDir, NamedTempFile or TempPath",
    ),
    (
        re.compile(r"\btemp_dir\s*\(\s*\)"),
        "std::env::temp_dir() is shared, and nothing empties it",
    ),
)
RUST_RAW_STRING = re.compile(r'b?r(#*)"')
RUST_CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
# TypeScript: a made directory, and the removals that count for it.
SCRIPT_MAKES = re.compile(r"\bmkdtemp(?:Sync)?\s*\(")
SCRIPT_BINDS = re.compile(
    r"(?:\b(?:const|let|var)\s+)?([A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)\s*=\s*"
    r"(?:await\s+)?(?:[A-Za-z_$][\w$]*\.)*mkdtemp(?:Sync)?\s*\($"
)
# Python: the calls that leave a path behind, the calls that remove one, and the cleanups.
PYTHON_MAKES = {"mkdtemp", "mkstemp"}
PYTHON_REMOVES = {"rmtree", "remove", "unlink", "rmdir", "removedirs"}
PYTHON_CLEANUPS = {
    "addCleanup",
    "addClassCleanup",
    "addModuleCleanup",
    "addAsyncCleanup",
}


def census(root=REPO):
    """Every test file R2 names, without the planted fixtures (R4)."""
    found = {
        path for pattern in CENSUS for path in root.glob(pattern) if path.is_file()
    }
    return sorted(path for path in found if FIXTURES not in path.parents)


def language_of(path):
    """The file's language; a `.fixture` is read by the extension before its suffix (R4)."""
    name = path.name.removesuffix(".fixture")
    return LANGUAGES[Path(name).suffix]


def leaks(path, root=REPO):
    """Each leaking form in one test file, as `<path>:<line>: <what>`."""
    text = path.read_text(encoding="utf-8")
    language = language_of(path)
    if language == "python":
        found = python_leaks(text)
    else:
        found = script_leaks(text, rust=language == "rust")
    try:
        name = path.relative_to(root).as_posix()
    except ValueError:
        name = path.name
    return [f"{name}:{line}: {what}" for line, what in sorted(found)]


def blank_code(text, rust):
    """(the text with comments and string contents blanked, every string literal as (offset,
    contents)). Offsets and line breaks are kept, so a match's line is its line in the file."""
    out = list(text)
    literals = []
    index, size = 0, len(text)

    def blank(start, end):
        for position in range(start, min(end, size)):
            if out[position] != "\n":
                out[position] = " "

    while index < size:
        char = text[index]
        pair = text[index : index + 2]
        if pair == "//":
            end = text.find("\n", index)
            end = size if end < 0 else end
            blank(index, end)
            index = end
            continue
        if pair == "/*":
            depth, end = 1, index + 2
            while end < size and depth:
                if rust and text.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif text.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
            blank(index, end)
            index = end
            continue
        follows_word = index > 0 and (
            text[index - 1].isalnum() or text[index - 1] == "_"
        )
        if rust and not follows_word:
            raw = RUST_RAW_STRING.match(text, index)
            if raw:
                close = '"' + raw.group(1)
                end = text.find(close, raw.end())
                end = size if end < 0 else end
                literals.append((index, text[raw.end() : end]))
                blank(raw.end(), end)
                index = end + len(close)
                continue
        if rust and char == "'":
            quoted = RUST_CHAR.match(text, index)
            if quoted:
                blank(index + 1, quoted.end() - 1)
                index = quoted.end()
            else:
                index += 1
            continue
        if char == '"' or (not rust and char in "'`"):
            end = index + 1
            while end < size and text[end] != char:
                if text[end] == "\\":
                    end += 2
                    continue
                if text[end] == "\n" and char != "`" and not rust:
                    break
                end += 1
            literals.append((index, text[index + 1 : end]))
            blank(index + 1, end)
            index = end + 1
            continue
        index += 1
    return "".join(out), literals


def line_of(text, offset):
    return text.count("\n", 0, offset) + 1


def script_leaks(text, rust):
    """Rust and TypeScript: the calls that keep what they make, and literals under /tmp/."""
    code, literals = blank_code(text, rust)
    found = []
    for offset, contents in literals:
        if contents.startswith("/tmp/"):
            found.append(
                (line_of(text, offset), "a string literal that begins with /tmp/")
            )
    if rust:
        for pattern, what in RUST_CALLS:
            found += [(line_of(code, m.start()), what) for m in pattern.finditer(code)]
        return found
    for made in SCRIPT_MAKES.finditer(code):
        bound = SCRIPT_BINDS.search(code[: made.end()])
        name = bound.group(1) if bound and bound.end() == made.end() else None
        removal = name and re.search(
            rf"\b(?:rmSync|rm)\s*\(\s*{re.escape(name)}\b", code
        )
        if not removal:
            what = (
                "is never removed"
                if name
                else "is bound to no name, so nothing removes it"
            )
            found.append(
                (line_of(code, made.start()), f"a directory from mkdtemp {what}")
            )
    return found


def python_leaks(text):
    """Python: mkdtemp, mkstemp and NamedTemporaryFile(delete=False), unless the same function
    registers the removal of the path they return (addCleanup, or a finally that removes it)."""
    tree = ast.parse(text)
    imported = {
        alias.asname or alias.name: alias.name
        for node in ast.walk(tree)
        if isinstance(node, ast.ImportFrom) and node.module == "tempfile"
        for alias in node.names
    }
    parents = {
        child: node for node in ast.walk(tree) for child in ast.iter_child_nodes(node)
    }
    found = []
    for node in ast.walk(tree):
        what = leaking_call(node, imported)
        if what is None:
            continue
        scope = enclosing_function(node, parents) or tree
        names = bound_names(node, parents)
        if not (names and removes(scope, names)):
            found.append(
                (node.lineno, f"{what} and nothing in its function removes the path")
            )
    return found


def leaking_call(node, imported):
    if not isinstance(node, ast.Call):
        return None
    func = node.func
    if (
        isinstance(func, ast.Attribute)
        and getattr(func.value, "id", None) == "tempfile"
    ):
        name = func.attr
    elif isinstance(func, ast.Name) and func.id in imported:
        name = imported[func.id]
    else:
        return None
    if name in PYTHON_MAKES:
        return f"tempfile.{name}("
    kept = any(
        keyword.arg == "delete"
        and isinstance(keyword.value, ast.Constant)
        and keyword.value.value is False
        for keyword in node.keywords
    )
    if name == "NamedTemporaryFile" and kept:
        return "NamedTemporaryFile(delete=False)"
    return None


def enclosing_function(node, parents):
    while node in parents:
        node = parents[node]
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda):
            return node
    return None


def bound_names(node, parents):
    """The names the call's result is bound to: an assignment's targets, or a `with ... as`."""
    child = node
    while child in parents:
        parent = parents[child]
        if isinstance(parent, ast.withitem) and parent.optional_vars is not None:
            return {ast.unparse(parent.optional_vars)}
        if isinstance(parent, ast.Assign | ast.AnnAssign | ast.NamedExpr):
            targets = (
                parent.targets if isinstance(parent, ast.Assign) else [parent.target]
            )
            return {name for target in targets for name in target_names(target)}
        if isinstance(parent, ast.stmt):
            return set()
        child = parent
    return set()


def target_names(target):
    """`a, self.b = ...` binds `a` and `self.b`; the `self` inside `self.b` is not bound."""
    if isinstance(target, ast.Tuple | ast.List):
        return {name for element in target.elts for name in target_names(element)}
    if isinstance(target, ast.Starred):
        return target_names(target.value)
    return {ast.unparse(target)}


def mentions(node, names):
    return any(
        ast.unparse(part) in names
        for part in ast.walk(node)
        if isinstance(part, ast.Name | ast.Attribute | ast.Subscript)
    )


def removes(scope, names):
    for node in ast.walk(scope):
        if isinstance(node, ast.Call) and called(node) in PYTHON_CLEANUPS:
            if any(mentions(arg, names) for arg in [*node.args, *node.keywords]):
                return True
        if isinstance(node, ast.Try | ast.TryStar):
            for statement in node.finalbody:
                for call in ast.walk(statement):
                    if isinstance(call, ast.Call) and called(call) in PYTHON_REMOVES:
                        if any(mentions(arg, names) for arg in call.args):
                            return True
    return False


def called(call):
    func = call.func
    return func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")


def planted_lines(path, marker):
    """The lines of a fixture that end with a `planted:` or `accepted:` comment."""
    lines = path.read_text(encoding="utf-8").splitlines()
    return [number for number, line in enumerate(lines, 1) if f" {marker}: " in line]


class TestsLeaveNoTemporaryFiles(unittest.TestCase):
    def test_every_test_file_removes_what_it_creates(self):
        files = examined("test file(s)", census())
        self.assertEqual(
            sorted({language_of(path) for path in files}),
            ["python", "rust", "typescript"],
        )
        planted = sorted(FIXTURES.glob("*.fixture"))
        self.assertEqual(len(planted), 3, "the three planted fixtures are present")
        for path in planted:
            self.assertNotIn(
                path, files, f"{path.name} is planted, never part of the census"
            )
        found = [leak for path in files for leak in leaks(path)]
        self.assertEqual(found, [], "\n".join(found))

    def assert_planted_leaks_are_refused(self, name):
        path = FIXTURES / name
        refused = planted_lines(path, "planted")
        accepted = planted_lines(path, "accepted")
        self.assertGreater(len(refused), 0, f"{name} plants no leak")
        self.assertGreater(len(accepted), 0, f"{name} plants no accepted form")
        found = leaks(path)
        lines = sorted({int(leak.split(":")[1]) for leak in found})
        self.assertEqual(lines, refused, "\n".join(found))
        for leak in found:
            self.assertTrue(
                leak.startswith(f"scripts/tests/fixtures/temp-hygiene/{name}:"), leak
            )

    def test_a_planted_rust_leak_is_refused(self):
        self.assert_planted_leaks_are_refused("leaks_tempdir.rs.fixture")

    def test_a_planted_python_leak_is_refused(self):
        self.assert_planted_leaks_are_refused("leaks_mkdtemp.py.fixture")

    def test_a_planted_typescript_leak_is_refused(self):
        self.assert_planted_leaks_are_refused("leaks_mkdtemp.ts.fixture")


if __name__ == "__main__":
    unittest.main()

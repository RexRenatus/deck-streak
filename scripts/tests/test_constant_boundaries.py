"""Each comparison against a named constant has a boundary test (SPEC-666 R1 to R12).

A comparison of a value with a named constant (`count <= MAX_ITEMS`, `n >= Self::MIN`,
`kind == DEFAULT`) changes its answer at the constant, so only a test that holds the compared value
at the constant and one step to each side can tell `<` from `<=`. This census reads every shipped
source file of the repository in the six languages it holds, finds each comparison whose one
operand is a named constant by that language's constant forms, and looks each one up in the
register `scripts/constant-boundaries.json`, which names a test below, at and above the constant
for each comparison it covers. A comparison the register does not cover with three tests the tree
defines is a site. An unlisted site prints `WARN constant-boundary <key>: <reason>`, which passes
at warn and fails at fail; a listed entry that names no current site prints
`STALE constant-boundary <key>` and fails at both levels.

The census reads text: it never imports, compiles or runs what it reads. A comparison against a
bare literal, a comparison of two constants and a comparison inside test code are counted apart
and are outside the rule. An operand the census cannot decide (an expression of constants, a
capitals path that no shipped `const` or `static` declares, a capitals name with no binding in its
file) prints `UNDECIDED` and is never a site.
"""

import ast
import json
import re
import subprocess
import unittest
from collections import Counter
from pathlib import Path

from _support import REPO, examined

RULE = "constant-boundary"
LEVEL = "warn"
REGISTER = "scripts/constant-boundaries.json"
FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "constant-boundaries"
LANGUAGES = ("python", "rust", "shell", "svelte", "swift", "typescript")
POINTS = ("below", "at", "above")
CAPITALS = re.compile(r"^_?[A-Z][A-Z0-9_]+$")

# R2, R3: which files are shipped source and which are test code, by path.
WEB_TEST = re.compile(r"\.(?:test|spec)\.[cm]?[jt]s$|\.test\.support\.ts$")
WEB_TEST_DIRS = ("web/app/tests/", "web/app/bench/")
SWIFT_TEST = re.compile(r"(?:^|/)[^/]*Tests/")
TEST_PARTS = {"tests", "benches", "examples"}

# R4: a comparison is a spaced operator, the formatters' form; its operands end at these.
COMPARISON = re.compile(r"(?<=\s)(===|!==|==|!=|<=|>=|<|>)(?=\s)")
OPERATORS = {"===": "==", "!==": "!=", "==": "==", "!=": "!=", "<=": "<=", ">=": ">="}
LEFT_STOP = re.compile(
    r"[(\[{,;]|&&|\|\||=>|(?<![=!<>])=(?!=)|\b(?:return|if|while|guard|match|in)\b|\?|(?<!:):(?!:)"
)
RIGHT_STOP = re.compile(r"[)\]},;{?]|&&|\|\||=>|\b(?:else|then)\b|\bas\s+(?=[A-Za-z])")
LITERAL = re.compile(
    r"^-?(?:0x[0-9a-fA-F_]+|\d[\d_]*(?:\.\d+)?(?:e-?\d+)?)(?:[a-z]\w*)?$"
    r"|^[\"'`]|^(?:true|false|nil|null|undefined|None)$"
)

# R5: each language's named-constant forms.
RUST_DECLARED = re.compile(r"\b(?:const|static(?:\s+mut)?)\s+(_?[A-Z][A-Z0-9_]+)\s*:")
RUST_PRIMITIVE = re.compile(r"^(?:[iu](?:8|16|32|64|128|size)|f32|f64|char)::[A-Z][A-Z0-9_]*$")
RUST_PATH = re.compile(r"^(?:(?:crate|self|super|Self|[A-Za-z_]\w*)::)*(_?[A-Z][A-Z0-9_]+)$")
RUST_EXPRESSION = re.compile(r"^[\w:\s+\-*/()]+$")
RUST_TYPE_WORD = re.compile(r"^(?:[iu]\d+|[iu]size|f32|f64|as)$")
RUST_TEST_ITEM = re.compile(r"#\[(?:test|cfg\((?:(?:all|any)\(\s*)?(?:test|kani)\b[^\]]*)\]")
RUST_TEST_MODULE = re.compile(
    r'#\[cfg\((?:test|kani)\)\]\s*(?:#\[path\s*=\s*"([^"]+)"\]\s*)?'
    r"(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;"
)
RUST_RAW = re.compile(r'b?r(#*)"')
RUST_CHAR = re.compile(r"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]{1,6}\}|.)|[^\\'\n])'")
SCRIPT_DECLARED = re.compile(r"(?m)^\s*(?:export\s+)?const\s+(_?[A-Z][A-Z0-9_]+)\b")
SCRIPT_IMPORT = re.compile(r"\bimport\s+([^;]*?)\bfrom\b", re.S)
SCRIPT_NAME = re.compile(r"^(?:[A-Za-z_$][\w$]*\.)*(_?[A-Z][A-Z0-9_]+)(?:\.[A-Za-z_$][\w$]*)?$")
SVELTE_SCRIPT = re.compile(r"<script[^>]*[>](.*?)</script>", re.S)
SVELTE_EXPRESSION = re.compile(r"\{([#:/@]?[^{}]*)\}")
SWIFT_STATIC = re.compile(r"\bstatic\s+let\s+(\w+)")
SWIFT_FILE_LET = re.compile(r"(?m)^(?:(?:private|fileprivate|public|internal)\s+)?let\s+(\w+)")
SWIFT_NAME = re.compile(r"^(?:([A-Z]\w*|Self)\.)?([A-Za-z_]\w*)$")
SHELL_READONLY = re.compile(r"(?m)^\s*(?:readonly|declare\s+-r)\s+([A-Z_][A-Z0-9_]*)=")
SHELL_ASSIGNED = re.compile(r"(?m)^([A-Z_][A-Z0-9_]*)=")
SHELL_TEST_LINE = re.compile(r"\[\[?|\btest\b|\(\(")
SHELL_COMPARISON = re.compile(
    r"(\"?\$\{?\w+\}?\"?|\w+)\s+(-lt|-le|-gt|-ge|-eq|-ne|==|!=|=|<=|>=|<|>)\s+"
    r"(\"?\$\{?\w+\}?\"?|\w+)"
)
SHELL_OPERATORS = {"-lt": "<", "-le": "<=", "-gt": ">", "-ge": ">=", "-eq": "==", "-ne": "!="}
SHELL_NAME = re.compile(r'^"?\$\{?([A-Z_][A-Z0-9_]*)\}?"?$|^([A-Z_][A-Z0-9_]*)$')
PYTHON_OPERATORS = {
    ast.Lt: "<",
    ast.LtE: "<=",
    ast.Gt: ">",
    ast.GtE: ">=",
    ast.Eq: "==",
    ast.NotEq: "!=",
}


def classify(path):
    """(language, 'shipped' or 'test') for a tracked path, or None for a file in no language the
    rule reads. A planted `.fixture` is never part of a census (R2, R3, R11)."""
    if path.endswith(".fixture"):
        return None
    parts = set(path.split("/")[:-1])
    if path.endswith(".rs"):
        return ("rust", "test" if parts & TEST_PARTS else "shipped")
    if path.endswith(".py"):
        test = "tests" in parts or path.rsplit("/", 1)[-1].startswith("test_")
        return ("python", "test" if test else "shipped")
    if path.endswith((".ts", ".js", ".mjs", ".cjs", ".svelte")):
        language = "svelte" if path.endswith(".svelte") else "typescript"
        test = WEB_TEST.search(path) or path.startswith(WEB_TEST_DIRS)
        return (language, "test" if test else "shipped")
    if path.endswith(".swift"):
        return ("swift", "test" if SWIFT_TEST.search(path) else "shipped")
    if path.endswith(".sh"):
        return ("shell", "shipped")
    return None


def tracked_sources(root):
    """{path: text} for every tracked file the rule reads, and the register (R1, R2)."""
    command = ["git", "-C", str(root), "ls-files", "-z"]
    listing = subprocess.run(command, capture_output=True, check=False)
    if listing.returncode != 0:
        raise AssertionError(f"git ls-files exited {listing.returncode}: {listing.stderr!r}")
    sources = {}
    for path in listing.stdout.decode("utf-8").split("\0"):
        if path and (classify(path) or path == REGISTER):
            file = root / path
            if file.is_file() and not file.is_symlink():
                sources[path] = file.read_text(encoding="utf-8", errors="replace")
    return sources


def planted(case):
    """{path: text} for one planted tree: each `.fixture` under the case, by its path there without
    the suffix, read as text and never run (R11)."""
    root = FIXTURES / case
    found = sorted(root.rglob("*.fixture"))
    return {
        path.relative_to(root).as_posix().removesuffix(".fixture"): path.read_text(encoding="utf-8")
        for path in found
    }


def blank(text, language):
    """The text with comments and the contents of string and character literals blanked; offsets,
    quote marks and line breaks are kept, so a comparison in a comment or a string is never read
    and a quoted operand still reads as a literal (R4)."""
    out = list(text)
    size = len(text)
    index = 0

    def wipe(start, end):
        for position in range(start, min(end, size)):
            if out[position] != "\n":
                out[position] = " "

    while index < size:
        pair = text[index : index + 2]
        if pair == "//":
            end = text.find("\n", index)
            end = size if end < 0 else end
            wipe(index, end)
            index = end
            continue
        if pair == "/*":
            depth, end = 1, index + 2
            while end < size and depth:
                if language != "typescript" and text.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif text.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
            wipe(index, end)
            index = end
            continue
        char = text[index]
        after_word = index > 0 and (text[index - 1].isalnum() or text[index - 1] == "_")
        if language == "rust" and not after_word:
            raw = RUST_RAW.match(text, index)
            if raw:
                close = '"' + raw.group(1)
                end = text.find(close, raw.end())
                end = size if end < 0 else end + len(close)
                wipe(index, end)
                index = end
                continue
            if char == "'":
                literal = RUST_CHAR.match(text, index)
                if literal:
                    wipe(index + 1, literal.end() - 1)
                    index = literal.end()
                    continue
        if language == "swift" and text.startswith('"""', index):
            end = text.find('"""', index + 3)
            end = size if end < 0 else end + 3
            wipe(index, end)
            index = end
            continue
        if char == '"' or (language == "typescript" and char in "'`"):
            end = index + 1
            while end < size and text[end] != char and (text[end] != "\n" or char == "`"):
                end += 2 if text[end] == "\\" else 1
            wipe(index + 1, end)
            index = end + 1
            continue
        index += 1
    return "".join(out)


def svelte_code(text):
    """A Svelte file's script blocks and markup expressions, everything else blanked (R4)."""
    keep = [" " if char != "\n" else "\n" for char in text]
    markup = list(keep)
    scripts = [(match.start(1), match.end(1)) for match in SVELTE_SCRIPT.finditer(text)]
    for start, end in scripts:
        keep[start:end] = text[start:end]
    outside = list(text)
    for start, end in scripts:
        outside[start:end] = markup[start:end]
    for match in SVELTE_EXPRESSION.finditer("".join(outside)):
        keep[match.start(1) : match.end(1)] = text[match.start(1) : match.end(1)]
    return "".join(keep)


def script_code(path, text):
    """A TypeScript, JavaScript or Svelte file's code, blanked (R4)."""
    return blank(svelte_code(text) if path.endswith(".svelte") else text, "typescript")


def rust_test_spans(code):
    """Each item under `#[test]`, `#[cfg(test)]` or `#[cfg(kani)]`, to its closing brace or its
    semicolon: test and proof code, read apart (R3)."""
    spans = []
    for match in RUST_TEST_ITEM.finditer(code):
        index, depth = match.end(), 0
        while index < len(code):
            char = code[index]
            if char in "([{":
                depth += 1
            elif char in ")]}":
                depth -= 1
                if depth == 0 and char == "}":
                    index += 1
                    break
            elif char == ";" and depth == 0:
                index += 1
                break
            index += 1
        spans.append((match.start(), index))
    return spans


def rust_test_modules(sources):
    """The out-of-line modules a `#[cfg(test)]` or `#[cfg(kani)]` declaration names (R3)."""
    modules = set()
    for path, text in sources.items():
        if not path.endswith(".rs"):
            continue
        parent = Path(path)
        for match in RUST_TEST_MODULE.finditer(text):
            if match.group(1):
                modules.add((parent.parent / match.group(1)).as_posix())
                continue
            own_dir = parent.name in ("lib.rs", "main.rs", "mod.rs")
            base = parent.parent if own_dir else parent.with_suffix("")
            modules.add((base / f"{match.group(2)}.rs").as_posix())
            modules.add((base / match.group(2) / "mod.rs").as_posix())
    return modules


def keep_spans(code, spans, inside):
    """The code with what lies outside the spans blanked (inside) or what lies inside them."""
    mask = [False] * len(code)
    for start, end in spans:
        mask[start:end] = [True] * (end - start)
    return "".join(
        char if char == "\n" or mask[index] == inside else " " for index, char in enumerate(code)
    )


def strip_operand(text):
    text = re.sub(r"^[&*!\s]+", "", text.strip())
    text = re.sub(r"\s+as\s+[A-Za-z0-9_:]+$", "", text)
    text = re.sub(r"^\(\s*(.*?)\s*\)$", r"\1", text)
    return text.strip()


def operands(code, match):
    """(left, right) operand texts around a comparison, each within its line (R4)."""
    start = code.rfind("\n", 0, match.start()) + 1
    left_text = code[start : match.start()]
    cut = 0
    for stop in LEFT_STOP.finditer(left_text):
        cut = stop.end()
    end = code.find("\n", match.end())
    right_text = code[match.end() : len(code) if end < 0 else end]
    stop = RIGHT_STOP.search(right_text)
    right = right_text[: stop.start()] if stop else right_text
    return strip_operand(left_text[cut:]), strip_operand(right)


def pick(verdicts, left, right):
    """The operand a key names: the constant, else the undecided one (R7)."""
    for wanted in ("constant", "undecided"):
        for verdict_, operand in zip(verdicts, (left, right)):
            if verdict_ == wanted:
                return operand
    return right


class Tree:
    """The declarations of a tree that decide which operands are named constants (R5)."""

    def __init__(self, sources, groups):
        def shipped(language):
            return [(path, sources[path]) for path in groups.get((language, "shipped"), [])]

        self.rust = {
            name
            for _, text in shipped("rust")
            for name in RUST_DECLARED.findall(blank(text, "rust"))
        }
        self.script = {
            name
            for language in ("typescript", "svelte")
            for path, text in shipped(language)
            for name in SCRIPT_DECLARED.findall(script_code(path, text))
        }
        self.swift = {
            name
            for _, text in shipped("swift")
            for name in SWIFT_STATIC.findall(blank(text, "swift"))
        }


def judge(language, operand, tree, scope):
    """'constant', 'undecided', 'literal' or None for one operand (R4, R5, R8)."""
    if not operand:
        return None
    if LITERAL.match(operand):
        return "literal"
    if language == "rust":
        if RUST_PRIMITIVE.match(operand):
            return "constant"
        path = RUST_PATH.match(operand)
        if path:
            return "constant" if path.group(1) in tree.rust else "undecided"
        names = re.findall(r"[A-Za-z_]\w*", operand.replace("::", " "))
        lower = [name for name in names if name[0].islower() and not RUST_TYPE_WORD.match(name)]
        if RUST_EXPRESSION.match(operand) and any(map(CAPITALS.match, names)) and not lower:
            return "undecided"
        return None
    if language == "typescript":
        if SCRIPT_NAME.match(operand):
            parts = set(re.findall(r"[A-Za-z_$][\w$]*", operand))
            if parts & (tree.script | scope):
                return "constant"
        return None
    if language == "swift":
        name = SWIFT_NAME.match(operand)
        if name and name.group(1):
            return "constant" if name.group(2) in tree.swift else None
        if name and name.group(2) in scope:
            return "constant"
    return None


def scope_of(language, code):
    """The names a file binds that read as constants inside it alone (R5)."""
    if language == "typescript":
        clauses = SCRIPT_IMPORT.findall(code)
        return {name for clause in clauses for name in re.findall(r"[A-Za-z_$][\w$]*", clause)}
    if language == "swift":
        return set(SWIFT_STATIC.findall(code)) | set(SWIFT_FILE_LET.findall(code))
    return set()


def record(reading, language, path, line, operator, operand, verdicts, part):
    """One comparison, classed (R6 to R8): test code, two constants and literals are only counted."""
    if part == "test":
        kind = "test-code" if "constant" in verdicts else None
    elif verdicts == ("constant", "constant"):
        kind = "two-constants"
    elif "constant" in verdicts:
        kind = "constant"
    elif "undecided" in verdicts:
        kind = "undecided"
    elif "literal" in verdicts:
        kind = "bare-literal"
    else:
        kind = None
    if kind:
        reading["counts"][(language, kind)] += 1
    if kind in ("constant", "undecided"):
        reading[kind].append((path, line, operator, operand))


def scan_text(language, path, code, tree, reading, part):
    """Each comparison of a brace language's code, into the reading (R4 to R8)."""
    reader = "typescript" if language == "svelte" else language
    scope = scope_of(reader, code)
    for match in COMPARISON.finditer(code):
        line = code.count("\n", 0, match.start()) + 1
        left, right = operands(code, match)
        verdicts = (judge(reader, left, tree, scope), judge(reader, right, tree, scope))
        operator = OPERATORS.get(match.group(1), match.group(1))
        found = pick(verdicts, left, right)
        record(reading, language, path, line, operator, found, verdicts, part)


def scan_python(path, text, reading, part):
    """Each comparison of one Python file, by its syntax tree (R4, R5)."""
    module = ast.parse(text)
    declared, imported = set(), set()
    for node in module.body:
        if isinstance(node, ast.Assign):
            names = [target.id for target in node.targets if isinstance(target, ast.Name)]
            declared |= {name for name in names if CAPITALS.match(name)}
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            if CAPITALS.match(node.target.id) or "Final" in ast.unparse(node.annotation):
                declared.add(node.target.id)
    for node in ast.walk(module):
        if isinstance(node, ast.ImportFrom):
            imported |= {alias.asname or alias.name for alias in node.names}

    def judge_node(node):
        if isinstance(node, ast.UnaryOp):
            node = node.operand
        if isinstance(node, ast.Constant):
            return "literal"
        if isinstance(node, ast.Name) and node.id in declared:
            return "constant"
        if isinstance(node, ast.Name) and CAPITALS.match(node.id):
            return "constant" if node.id in imported else "undecided"
        if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
            return "constant" if CAPITALS.match(node.attr) else None
        return None

    found = [node for node in ast.walk(module) if isinstance(node, ast.Compare)]
    for node in sorted(found, key=lambda node: (node.lineno, node.col_offset)):
        lefts = [node.left, *node.comparators[:-1]]
        for operator, left, right in zip(node.ops, lefts, node.comparators):
            if type(operator) not in PYTHON_OPERATORS:
                continue
            verdicts = (judge_node(left), judge_node(right))
            operand = ast.unparse(pick(verdicts, left, right))
            symbol = PYTHON_OPERATORS[type(operator)]
            record(reading, "python", path, node.lineno, symbol, operand, verdicts, part)


def scan_shell(path, text, reading):
    """Each test-command comparison of one shell script (R4, R5)."""
    lines = [re.sub(r"'[^']*'", "''", line) for line in text.split("\n")]
    lines = [re.sub(r"(^|\s)#.*$", r"\1", line) for line in lines]
    code = "\n".join(lines)
    readonly = set(SHELL_READONLY.findall(code))
    assigned = Counter(SHELL_ASSIGNED.findall(code))
    for number, line in enumerate(lines, 1):
        if not SHELL_TEST_LINE.search(line):
            continue
        for match in SHELL_COMPARISON.finditer(line):
            if match.group(2) in ("<", ">", "<=", ">=") and "((" not in line:
                continue
            verdicts, names = [], []
            for operand in (match.group(1), match.group(3)):
                name = SHELL_NAME.match(operand)
                names.append(name and (name.group(1) or name.group(2)))
                if LITERAL.match(operand.strip('"')):
                    verdicts.append("literal")
                elif name and (names[-1] in readonly or assigned[names[-1]] == 1):
                    verdicts.append("constant")
                else:
                    verdicts.append("undecided" if name else None)
            operator = SHELL_OPERATORS.get(match.group(2), match.group(2).replace("=", "==", 1))
            if match.group(2) in ("==", "!=", "<=", ">="):
                operator = match.group(2)
            found = pick(verdicts, names[0], names[1])
            record(reading, "shell", path, number, operator, found, tuple(verdicts), "shipped")


def census(sources):
    """The reading of a tree given as {path: text}: the shipped files by language, each comparison
    against a named constant and each undecided one, and the counts kept apart (R2 to R8, R10)."""
    groups = {}
    for path in sorted(sources):
        kind = classify(path)
        if kind:
            groups.setdefault(kind, []).append(path)
    shipped = [path for language in LANGUAGES for path in groups.get((language, "shipped"), [])]
    examined("shipped source file(s)", shipped)
    tree = Tree(sources, groups)
    modules = rust_test_modules(sources)
    reading = {"examined": {}, "counts": Counter(), "constant": [], "undecided": []}
    for language in LANGUAGES:
        reading["examined"][language] = groups.get((language, "shipped"), [])
        for part in ("shipped", "test"):
            for path in groups.get((language, part), []):
                scan_file(language, part, path, sources[path], tree, modules, reading)
    reading["comparisons"] = keyed(reading["constant"])
    reading["undecided"] = keyed(reading["undecided"])
    return reading


def scan_file(language, part, path, text, tree, modules, reading):
    """One file, by its language's reader; a Rust file's test items are read as test code (R3)."""
    if language == "python":
        scan_python(path, text, reading, part)
    elif language == "shell":
        scan_shell(path, text, reading)
    elif language in ("svelte", "typescript"):
        scan_text(language, path, script_code(path, text), tree, reading, part)
    elif language == "rust" and part == "shipped":
        code = blank(text, "rust")
        if path in modules:
            scan_text(language, path, code, tree, reading, "test")
            return
        spans = rust_test_spans(code)
        scan_text(language, path, keep_spans(code, spans, False), tree, reading, part)
        if spans:
            scan_text(language, path, keep_spans(code, spans, True), tree, reading, "test")
    else:
        scan_text(language, path, blank(text, language), tree, reading, part)


def keyed(found):
    """{key: line}, a key being `<file> <operator> <constant> #<ordinal>`: the ordinal counts the
    same operator and constant in the file in reading order, so a line move keeps the key (R7)."""
    seen, keys = Counter(), {}
    for path, line, operator, operand in sorted(found, key=lambda item: (item[0], item[1])):
        seen[(path, operator, operand)] += 1
        keys[f"{path} {operator} {operand} #{seen[(path, operator, operand)]}"] = line
    return keys


def defines(sources, test):
    """Whether `test`, `<path>::<name>`, names test code the tree defines (R6)."""
    path, _, name = test.partition("::")
    kind = classify(path) if path in sources else None
    if not name or kind is None:
        return False
    language, part = kind
    text = sources[path]
    if language == "rust":
        code = blank(text, "rust")
        if part == "shipped":
            code = keep_spans(code, rust_test_spans(code), True)
        return re.search(rf"\bfn\s+{re.escape(name)}\s*[(<]", code) is not None
    if part != "test":
        return False
    if language == "python":
        found = re.search(rf"(?m)^\s*(?:async\s+)?def\s+{re.escape(name)}\s*\(", text)
    elif language == "swift":
        found = re.search(rf"\bfunc\s+{re.escape(name)}\s*\(", blank(text, "swift"))
    else:
        found = re.search(rf"\b(?:test|it)\(\s*([\"'`]){re.escape(name)}\1", text)
    return found is not None


def verdict(reading, register, sources, level):
    """(printed lines, failures): an unlisted site warns, and fails at `fail`; a listed entry or a
    register entry that names no current site is stale and fails at both levels (R6, R9)."""
    tests, listed = register.get("boundary_tests", {}), register.get("listed", {})
    lines, failures, sites = [], [], set()
    for key in reading["comparisons"]:
        points = tests.get(key, {})
        missing = [point for point in POINTS if not defines(sources, points.get(point, ""))]
        if not missing:
            continue
        sites.add(key)
        if key in listed:
            continue
        reason = "no boundary test" if key not in tests else "no test " + ", ".join(missing)
        lines.append(f"WARN {RULE} {key}: {reason}")
        if level == "fail":
            failures.append(lines[-1])
    stale = sorted(set(listed) - sites) + sorted(set(tests) - set(reading["comparisons"]))
    for key in stale:
        lines.append(f"STALE {RULE} {key}")
        failures.append(lines[-1])
    lines.extend(f"UNDECIDED {RULE} {key}" for key in reading["undecided"])
    return lines, failures


def run(sources, level):
    """The census and its verdict over one tree, its lines printed (R1, R9)."""
    reading = census(sources)
    register = json.loads(sources.get(REGISTER, "{}"))
    lines, failures = verdict(reading, register, sources, level)
    for language in LANGUAGES:
        print(f"examined {len(reading['examined'][language])} shipped {language} file(s)")
    for line in lines:
        print(line)
    return reading, lines, failures


class PlantedTrees(unittest.TestCase):
    """The planted trees under `fixtures/constant-boundaries`, read as text (R11)."""

    def reading(self, level="warn", register=None):
        sources = planted("planted")
        if register is not None:
            sources[REGISTER] = json.dumps(register)
        return run(sources, level)

    def test_each_file_is_classed_by_language_and_test_code_is_read_apart(self):
        reading, _, _ = self.reading()
        self.assertEqual(
            reading["examined"],
            {
                "python": ["scripts/demo_limits.py"],
                "rust": ["crates/demo/src/limits.rs"],
                "shell": ["scripts/demo-limits.sh"],
                "svelte": ["web/app/src/lib/demo/Badge.svelte"],
                "swift": ["ios/App/Limits.swift"],
                "typescript": ["web/app/src/lib/demo/limits.ts"],
            },
        )
        tested = sorted(key[0] for key in reading["counts"] if key[1] == "test-code")
        self.assertEqual(tested, ["python", "rust", "swift", "typescript"])
        self.assertEqual(reading["counts"][("rust", "test-code")], 2)

    def test_a_comparison_with_no_boundary_test_is_a_site_in_each_language(self):
        _, lines, failures = self.reading()
        self.assertEqual(
            sorted(line for line in lines if line.endswith(": no boundary test")),
            [
                "WARN constant-boundary crates/demo/src/limits.rs < SOFT_CAP #1: no boundary test",
                "WARN constant-boundary ios/App/Limits.swift < Limits.softCards #1: no boundary test",
                "WARN constant-boundary scripts/demo-limits.sh >= SOFT_JOBS #1: no boundary test",
                "WARN constant-boundary scripts/demo_limits.py != SOFT_LIMIT #1: no boundary test",
                "WARN constant-boundary web/app/src/lib/demo/Badge.svelte < PAGE_SIZE #1: "
                "no boundary test",
                "WARN constant-boundary web/app/src/lib/demo/limits.ts < SOFT_WIDTH #1: "
                "no boundary test",
            ],
        )
        self.assertEqual(failures, [])

    def test_a_test_below_at_and_above_the_constant_makes_a_comparison_clean(self):
        reading, lines, _ = self.reading()
        clean = [
            "crates/demo/src/limits.rs <= MAX_ITEMS #1",
            "ios/App/Limits.swift > Limits.maxCards #1",
            "scripts/demo-limits.sh > MAX_JOBS #1",
            "scripts/demo_limits.py >= MAX_RETRIES #1",
            "web/app/src/lib/demo/Badge.svelte >= MIN_QUERY #1",
            "web/app/src/lib/demo/limits.ts > PAGE_SIZE #1",
        ]
        for key in clean:
            self.assertIn(key, reading["comparisons"])
            self.assertEqual([line for line in lines if f" {key}" in line], [], key)
        self.assertEqual(len(reading["comparisons"]), 18)

    def test_a_test_at_the_constant_alone_leaves_a_site(self):
        _, lines, _ = self.reading()
        self.assertEqual(
            sorted(line for line in lines if line.endswith(": no test below, above")),
            [
                "WARN constant-boundary crates/demo/src/limits.rs >= MIN_ITEMS #1: "
                "no test below, above",
                "WARN constant-boundary ios/App/Limits.swift >= Limits.minCards #1: "
                "no test below, above",
                "WARN constant-boundary scripts/demo-limits.sh < MIN_FREE #1: no test below, above",
                "WARN constant-boundary scripts/demo_limits.py > MIN_DELAY #1: no test below, above",
                "WARN constant-boundary web/app/src/lib/demo/Badge.svelte <= SOFT_WIDTH #1: "
                "no test below, above",
                "WARN constant-boundary web/app/src/lib/demo/limits.ts >= MIN_QUERY #1: "
                "no test below, above",
            ],
        )

    def test_a_test_the_tree_does_not_define_leaves_its_point_missing(self):
        register = json.loads(planted("planted")[REGISTER])
        entry = register["boundary_tests"]["crates/demo/src/limits.rs <= MAX_ITEMS #1"]
        entry["above"] = "crates/demo/tests/limits.rs::ten_items_do_not_fit"
        _, lines, _ = self.reading(register=register)
        self.assertIn(
            "WARN constant-boundary crates/demo/src/limits.rs <= MAX_ITEMS #1: no test above", lines
        )

    def test_a_bare_literal_test_code_and_an_undecided_operand_are_never_sites(self):
        reading, lines, _ = self.reading()
        for language in LANGUAGES:
            self.assertEqual(reading["counts"][(language, "bare-literal")], 1, language)
        self.assertEqual(
            sorted(reading["undecided"]),
            [
                "crates/demo/src/limits.rs == Tier::GOLD #1",
                "scripts/demo-limits.sh == DEMO_TARGET #1",
                "scripts/demo_limits.py > CUTOFF_HOUR #1",
            ],
        )
        self.assertIn("UNDECIDED constant-boundary scripts/demo_limits.py > CUTOFF_HOUR #1", lines)
        for key in reading["undecided"]:
            self.assertNotIn(key, reading["comparisons"])
        self.assertEqual(len([line for line in lines if line.startswith("WARN ")]), 12)

    def test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does(self):
        _, lines, failures = self.reading(level="fail")
        self.assertEqual(failures, [line for line in lines if line.startswith("WARN ")])
        self.assertEqual(len(failures), 12)
        register = json.loads(planted("planted")[REGISTER])
        register["listed"] = {
            "crates/demo/src/limits.rs < SOFT_CAP #1": "planted: listed",
            "crates/demo/src/limits.rs > GONE #1": "planted: no such site",
        }
        listed = "WARN constant-boundary crates/demo/src/limits.rs < SOFT_CAP #1: no boundary test"
        for level in ("warn", "fail"):
            _, lines, failures = self.reading(level=level, register=register)
            self.assertIn("STALE constant-boundary crates/demo/src/limits.rs > GONE #1", failures)
            self.assertNotIn(listed, lines)
        _, _, failures = self.reading(level="warn", register=register)
        self.assertEqual(failures, ["STALE constant-boundary crates/demo/src/limits.rs > GONE #1"])

    def test_an_empty_population_is_void(self):
        sources = planted("void")
        expected = ["crates/demo/tests/limits.rs", "scripts/tests/test_demo_limits.py"]
        self.assertEqual(sorted(sources), expected)
        with self.assertRaisesRegex(AssertionError, r"^examined 0 shipped source file\(s\)"):
            census(sources)


class RepositoryCensus(unittest.TestCase):
    """The census over the repository at warn. It reads the whole tree, so CI runs it (R1, R12)."""

    def test_the_repository_reads_every_language_and_holds_no_stale_entry(self):
        sources = tracked_sources(REPO)
        self.assertIn(REGISTER, sources)
        reading, _, failures = run(sources, LEVEL)
        self.assertEqual(LEVEL, "warn")
        for language in LANGUAGES:
            self.assertGreater(len(reading["examined"][language]), 0, language)
        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main()

"""A census of the test doubles that fall back to a real program when they cannot plant their seam.

It reads text and nothing else: each file is parsed with `ast.parse`, and no test module is
imported or run, which is the point, because the modules it judges include tests that are never
run where this census runs.

An arm is a `try` whose `except` handler does not leave (it holds no `raise`, `return`, `continue`
or `break` and calls no `exit` or `sys.exit`, outside any function or class nested in it), with a
call that runs a real program within reach of that handler: in the handler's own body, in the
try's `finally` block, or anywhere after the try in its block and in each block enclosing it, up
to the function, class or module that holds it. A real program is `os.system`, `os.popen`,
`os.exec*`, `os.spawn*`, `os.posix_spawn*`, `subprocess.*` or `runpy.*`, also when it is called
through a name the file's imports bind. Such a handler turns a failure to plant into a run of the
real program, which reads as a passing run (#497, #532).

Each string constant is also parsed on its own, one level deep, since a test may hold a stand-in's
source as text; an arm found there is listed at the constant's line, with its line inside the
constant. A constant that does not parse is skipped and counted. Out of reach, by design (SPEC-129
section 12): a call through a name computed at run time, a program run by a function the handler
calls, the try's `else` block, a sibling handler, the body of a nested function, lambda or class,
and a source assembled from pieces that do not parse alone, or held in an f-string.

Run as a script it prints one line per arm, then the counts it examined.
"""

import ast
import re
import sys
import warnings
from pathlib import Path

HERE = Path(__file__).parent
REAL_PROGRAM = re.compile(
    r"(os\.(system|popen|exec\w*|spawn\w*|posix_spawn\w*)|subprocess\.\w+|runpy\.\w+)"
)
EXITS = {"exit", "sys.exit"}
SCOPES = (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda, ast.ClassDef)
TRIES = (ast.Try, ast.TryStar)
#: The fields of a statement that hold statements: blocks, or clauses (a handler, a match case)
#: whose body is a block.
BLOCKS = ("body", "handlers", "orelse", "finalbody", "cases")


def aliases(tree):
    """{local name: {qualified names}} that the imports anywhere in `tree` bind."""
    names = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.asname:
                    names.setdefault(alias.asname, set()).add(alias.name)
                else:
                    top = alias.name.split(".")[0]
                    names.setdefault(top, set()).add(top)
        elif isinstance(node, ast.ImportFrom) and node.module and not node.level:
            for alias in node.names:
                if alias.name != "*":
                    local = alias.asname or alias.name
                    names.setdefault(local, set()).add(f"{node.module}.{alias.name}")
    return names


def qualified(func, names):
    """The qualified names a call's callee can be, through the file's import aliases; a callee
    that is not a dotted name (a call's result, a subscript) has none."""
    attrs = []
    while isinstance(func, ast.Attribute):
        attrs.append(func.attr)
        func = func.value
    if not isinstance(func, ast.Name):
        return set()
    tail = "".join(f".{a}" for a in reversed(attrs))
    return {base + tail for base in names.get(func.id, {func.id})}


def inside(statements):
    """Every node of `statements`, without entering a nested function, lambda or class."""
    stack = list(statements)
    while stack:
        node = stack.pop()
        yield node
        if isinstance(node, SCOPES):
            continue
        stack.extend(ast.iter_child_nodes(node))


def leaves(handler, names):
    """Whether the handler leaves on some path: a `raise`, `return`, `continue` or `break`, or a
    call to `exit` or `sys.exit`, outside any function or class nested in it."""
    for node in inside(handler.body):
        if isinstance(node, (ast.Raise, ast.Return, ast.Continue, ast.Break)):
            return True
        if isinstance(node, ast.Call) and qualified(node.func, names) & EXITS:
            return True
    return False


def runs_a_program(statements, names):
    """Whether `statements` call a real program, outside any function or class nested in them."""
    return any(
        isinstance(node, ast.Call)
        and any(REAL_PROGRAM.fullmatch(name) for name in qualified(node.func, names))
        for node in inside(statements)
    )


def later(path):
    """The statements after the one at the end of `path`, in its block and each enclosing one."""
    after = []
    for block, k in reversed(path):
        after += [s for s in block[k + 1 :] if not isinstance(s, SCOPES)]
    return after


def walk_scope(body, names, path, found):
    """Append to `found` the handler line of each arm in one scope's statements; `path` is
    [(block, index)] from the scope down to `body`."""
    for k, statement in enumerate(body):
        here = [*path, (body, k)]
        if isinstance(statement, SCOPES):
            walk_scope(statement.body, names, [], found)
            continue
        if isinstance(statement, TRIES):
            for handler in statement.handlers:
                reach = [*handler.body, *statement.finalbody, *later(here)]
                if not leaves(handler, names) and runs_a_program(reach, names):
                    found.append(handler.lineno)
        for field, value in ast.iter_fields(statement):
            if field in BLOCKS and isinstance(value, list) and value:
                if isinstance(value[0], ast.stmt):
                    walk_scope(value, names, here, found)
                else:
                    for clause in value:
                        walk_scope(clause.body, names, here, found)


def handler_lines(tree):
    """The line of each arm's handler in a parsed module."""
    found = []
    walk_scope(tree.body, aliases(tree), [], found)
    return found


def str_constants(tree):
    """Every string constant of `tree` in source order; an f-string's pieces are not visited, as
    none of them is a whole source."""
    held, stack = [], [tree]
    while stack:
        node = stack.pop()
        if isinstance(node, ast.JoinedStr):
            continue
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            held.append(node)
        stack.extend(ast.iter_child_nodes(node))
    return sorted(held, key=lambda node: (node.lineno, node.col_offset))


def arms_of(text):
    """([(line, arm)], str constants parsed, str constants skipped) for `text`: an arm of the
    module itself is its handler's line and text, and an arm of a string constant is the
    constant's line, with the arm's line inside the constant and its text."""
    tree = ast.parse(text)
    lines = text.splitlines()
    arms = [(n, lines[n - 1].strip()) for n in handler_lines(tree)]
    parsed = skipped = 0
    for constant in str_constants(tree):
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                inner = ast.parse(constant.value)
        except (SyntaxError, ValueError):
            skipped += 1
            continue
        parsed += 1
        inner_lines = constant.value.splitlines()
        arms += [
            (constant.lineno, f"str constant, its line {n}: {inner_lines[n - 1].strip()}")
            for n in handler_lines(inner)
        ]
    return arms, parsed, skipped


def reading(directory=HERE):
    """(files examined, str constants parsed, str constants skipped, [(file, line, arm)]) over
    every `*.py` file under `directory`, at any depth, outside `__pycache__`; each file is named
    by its path relative to `directory`."""
    root = Path(directory)
    files = sorted(p for p in root.rglob("*.py") if "__pycache__" not in p.parts)
    found, parsed, skipped = [], 0, 0
    for path in files:
        arms, p, s = arms_of(path.read_text(encoding="utf-8"))
        parsed += p
        skipped += s
        found += [(path.relative_to(root).as_posix(), line, arm) for line, arm in arms]
    return len(files), parsed, skipped, found


def census(directory=HERE):
    """(files examined, [(file, line, arm)]) over every `*.py` file under `directory`."""
    files, _, _, found = reading(directory)
    return files, found


def main(directory=HERE):
    """Print each arm as `file:line: arm`, then the counts examined; 1 when an arm is found."""
    files, parsed, skipped, found = reading(directory)
    for name, line, arm in found:
        print(f"{name}:{line}: {arm}")
    print(
        f"examined {files} files, {parsed} str constants parsed, {skipped} skipped, "
        f"{len(found)} arms"
    )
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())

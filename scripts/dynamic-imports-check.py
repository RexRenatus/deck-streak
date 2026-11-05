#!/usr/bin/env python3
"""Refuse, before the push, a changed scripts/tests module that loads code and the register lacks.

SPEC-406, ADR-420. The census in the hygiene job (SPEC-190 R12 part 1, ADR-292) lists every site
in `scripts/tests` that imports or runs code by a name held in data in the `DYNAMIC_IMPORTS`
register of `scripts/tests/test_ci_workflows.py`, and it runs only in CI. This check reads, from
the one commit `HEAD` names when it starts (SPEC-612, ADR-634), each `.py` module under
`scripts/tests` that the push adds or modifies,
and refuses one whose syntax tree loads code through `importlib`, `runpy` or `exec` while the
register names no site of that module. It reads text only: nothing it judges is imported or run,
and a register or module it cannot read is VOID (exit 2), never OK.

Exit 0: OK or NOT-APPLICABLE. Exit 1: REFUSED. Exit 2: VOID.
Standard library only.
"""

import argparse
import ast
import subprocess
import sys
from pathlib import Path, PurePosixPath

REGISTER = "scripts/tests/test_ci_workflows.py"
TESTS = "scripts/tests"
REGISTER_NAME = "DYNAMIC_IMPORTS"
REGISTER_WRITERS = frozenset(
    {"__delitem__", "__ior__", "__setitem__", "clear", "pop", "popitem", "setdefault", "update"}
)
LOADER_MODULES = ("importlib", "runpy")
LOADER_BUILTIN = "exec"
REPO = Path(__file__).resolve().parents[1]


class Unreadable(Exception):
    """An input the check cannot read; its text is the VOID cause."""


def _bound_names(node):
    """The names `node` binds, by every binding kind the census's `bindings()` reads."""
    named = (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.MatchAs, ast.MatchStar)
    if isinstance(node, (*named, ast.ExceptHandler, ast.TypeVar, ast.ParamSpec, ast.TypeVarTuple)):
        return [node.name]
    if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
        return [node.id]
    if isinstance(node, ast.arg):
        return [node.arg]
    if isinstance(node, ast.MatchMapping):
        return [node.rest]
    if isinstance(node, (ast.Global, ast.Nonlocal)):
        return node.names
    if isinstance(node, (ast.Import, ast.ImportFrom)):
        return [alias.asname or alias.name.split(".")[0] for alias in node.names]
    return []


def _is_register(node):
    """Whether `node` is the bare name of the register."""
    return isinstance(node, ast.Name) and node.id == REGISTER_NAME


def _writes_register(node):
    """Whether `node` writes the register: a binding of its name (a `global` or `nonlocal`
    statement only declares it), a store or delete of an item, or a member that changes it."""
    if isinstance(node, ast.Subscript) and not isinstance(node.ctx, ast.Load):
        return _is_register(node.value)
    if isinstance(node, ast.Attribute) and node.attr in REGISTER_WRITERS:
        return _is_register(node.value)
    return not isinstance(node, (ast.Global, ast.Nonlocal)) and REGISTER_NAME in _bound_names(node)


def loader_sites(source):
    """Each load in `source` as (line, loader), in source order.

    A load is a use of a name an import of `importlib` or `runpy` binds, or of the bare name
    `exec` when the module binds no `exec` itself, by any binding the census reads, anywhere in
    the module. A string, a comment, an unused import and a star import hold none.
    """
    tree = ast.parse(source)
    bound = {}
    own_exec = False
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                top = alias.name.split(".")[0]
                name = alias.asname or top
                if top in LOADER_MODULES:
                    bound[name] = top
        elif isinstance(node, ast.ImportFrom):
            top = node.module.split(".")[0] if node.module and not node.level else None
            for alias in node.names:
                name = alias.asname or alias.name
                if top in LOADER_MODULES:
                    bound[name] = top
        own_exec = own_exec or LOADER_BUILTIN in _bound_names(node)
    found = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load):
            if node.id in bound:
                found.append((node.lineno, node.col_offset, bound[node.id]))
            elif node.id == LOADER_BUILTIN and not own_exec:
                found.append((node.lineno, node.col_offset, LOADER_BUILTIN))
    return [(line, loader) for line, _col, loader in sorted(found)]


def _site_modules(call):
    """The module names one `**allowed(reason, site, ...)` entry leads its sites with."""
    modules = set()
    for site in call.args[1:]:
        first = site.elts[0] if isinstance(site, ast.Tuple) and site.elts else None
        if not (isinstance(first, ast.Constant) and isinstance(first.value, str)):
            raise Unreadable(f"{REGISTER_NAME} site at line {site.lineno} has no string module")
        modules.add(first.value)
    return modules


def register_modules(source):
    """The module names the register's sites lead with; Unreadable names the line at fault."""
    try:
        tree = ast.parse(source)
    except SyntaxError as error:
        raise Unreadable(f"{REGISTER} does not parse: {error.msg} at line {error.lineno}")
    assigned = [
        node
        for node in tree.body
        if isinstance(node, (ast.Assign, ast.AnnAssign))
        and REGISTER_NAME
        in [
            t.id
            for t in (node.targets if isinstance(node, ast.Assign) else [node.target])
            if isinstance(t, ast.Name)
        ]
    ]
    if not assigned:
        raise Unreadable(f"{REGISTER} has no {REGISTER_NAME} assignment")
    if len(assigned) > 1:
        raise Unreadable(f"{REGISTER_NAME} is assigned a second time at line {assigned[1].lineno}")
    value = assigned[0].value
    if not isinstance(value, ast.Dict):
        raise Unreadable(f"{REGISTER_NAME} at line {assigned[0].lineno} is not a dict display")
    added = [
        node
        for node in tree.body[tree.body.index(assigned[0]) + 1 :]
        if isinstance(node, ast.AugAssign)
        and isinstance(node.op, ast.BitOr)
        and _is_register(node.target)
        and isinstance(node.value, ast.Dict)
    ]
    plain = assigned[0].targets if isinstance(assigned[0], ast.Assign) else [assigned[0].target]
    read = [*plain, *(node.target for node in added)]
    for node in ast.walk(tree):
        if _writes_register(node) and node not in read:
            raise Unreadable(
                f"{REGISTER_NAME} is written at line {node.lineno} in a shape the check does not read"
            )
    displays = [value, *(node.value for node in added)]
    modules = set()
    for key, entry in [pair for each in displays for pair in zip(each.keys, each.values)]:
        if key is not None:
            raise Unreadable(f"{REGISTER_NAME} has a literal key at line {key.lineno}")
        is_group = (
            isinstance(entry, ast.Call)
            and isinstance(entry.func, ast.Name)
            and entry.func.id == "allowed"
        )
        if not is_group:
            raise Unreadable(
                f"{REGISTER_NAME} entry at line {entry.lineno} is not an allowed group"
            )
        modules |= _site_modules(entry)
    if not modules:
        raise Unreadable(f"{REGISTER_NAME} names no module")
    return modules


def module_name(path):
    """The dotted name of `path` under scripts/tests; an `__init__` names its package."""
    parts = PurePosixPath(path).relative_to(TESTS).with_suffix("").parts
    return ".".join(parts[:-1] if parts[-1] == "__init__" else parts)


def _git(root, *args, path=None):
    """The stdout of `git -C root ...`, decoded strictly as UTF-8; Unreadable carries git's own
    message on failure, and names `path` (else the command) when the output is not UTF-8 text."""
    done = subprocess.run(
        ["git", "-C", str(root), *args],
        capture_output=True,
        check=False,
    )
    if done.returncode != 0:
        raise Unreadable(f"git {args[0]} failed: {done.stderr.decode('utf-8', 'replace').strip()}")
    try:
        return done.stdout.decode("utf-8")
    except UnicodeDecodeError:
        raise Unreadable(f"{path or 'the output of git ' + args[0]} is not UTF-8 text")


def changed_modules(root, base, head):
    """The .py paths under scripts/tests that the commit `head` adds or modifies against `base`."""
    listing = _git(
        root,
        "diff",
        "--name-only",
        "--no-renames",
        "--diff-filter=AM",
        f"{base}...{head}",
        "--",
        TESTS,
    )
    return [line for line in listing.splitlines() if line.endswith(".py")]


def _judge(root, base):
    """(lines, status) for one run over the one commit `HEAD` names; Unreadable stops it."""
    head = _git(root, "rev-parse", "--verify", "HEAD^{commit}").strip()
    registered = register_modules(_git(root, "show", f"{head}:{REGISTER}", path=REGISTER))
    lines = [f"dynamic-imports: register: {len(registered)} module(s) in {REGISTER_NAME} at HEAD"]
    paths = changed_modules(root, base, head)
    refused = []
    loaders = 0
    for path in paths:
        try:
            sites = loader_sites(_git(root, "show", f"{head}:{path}", path=path))
        except SyntaxError as error:
            raise Unreadable(f"{path} does not parse: {error.msg} at line {error.lineno}")
        loaders += bool(sites)
        if sites and module_name(path) not in registered:
            line, loader = sites[0]
            refused.append(
                f"dynamic-imports: REFUSED: {path}: loads code by {loader} at line {line}; "
                f"{REGISTER_NAME} lists no site of {module_name(path)}"
            )
    lines.append(
        f"dynamic-imports: examined {len(paths)} changed {TESTS} module(s), {loaders} loader-style"
    )
    lines += refused
    if refused:
        return lines + ["dynamic-imports: REFUSED"], 1
    if not paths:
        return lines + ["dynamic-imports: NOT-APPLICABLE"], 0
    return lines + ["dynamic-imports: OK"], 0


def main(argv=None):
    """Run the check; return the exit status."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=str(REPO))
    parser.add_argument("--base", default="origin/dev")
    args = parser.parse_args(argv)
    try:
        lines, status = _judge(Path(args.root), args.base)
    except Unreadable as error:
        print(f"dynamic-imports: VOID: {error}")
        return 2
    print("\n".join(lines))
    return status


if __name__ == "__main__":
    sys.exit(main())

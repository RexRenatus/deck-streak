#!/usr/bin/env python3
"""Refuse, before the push, a changed scripts/tests module that loads code and the register lacks.

SPEC-406, ADR-420. The census in the hygiene job (SPEC-190 R12 part 1, ADR-292) lists every site
in `scripts/tests` that imports or runs code by a name held in data in the `DYNAMIC_IMPORTS`
register of `scripts/tests/test_ci_workflows.py`, and it runs only in CI. This check reads, from
the commit `HEAD` names, each `.py` module under `scripts/tests` that the push adds or modifies,
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
LOADER_MODULES = ("importlib", "runpy")
LOADER_BUILTIN = "exec"
REPO = Path(__file__).resolve().parents[1]


class Unreadable(Exception):
    """An input the check cannot read; its text is the VOID cause."""


def loader_sites(source):
    """Each load in `source` as (line, loader), in source order.

    A load is a use of a name an import of `importlib` or `runpy` binds, or of the bare name
    `exec` when the module binds no `exec` itself. A string, a comment, an unused import and a
    star import hold none.
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
                elif name == LOADER_BUILTIN:
                    own_exec = True
        elif isinstance(node, ast.ImportFrom):
            top = (node.module or "").split(".")[0]
            for alias in node.names:
                name = alias.asname or alias.name
                if top in LOADER_MODULES and not node.level and alias.name != "*":
                    bound[name] = top
                elif name == LOADER_BUILTIN:
                    own_exec = True
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            own_exec = own_exec or node.name == LOADER_BUILTIN
        elif isinstance(node, ast.arg):
            own_exec = own_exec or node.arg == LOADER_BUILTIN
        elif isinstance(node, ast.Name) and isinstance(node.ctx, (ast.Store, ast.Del)):
            own_exec = own_exec or node.id == LOADER_BUILTIN
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
    modules = set()
    for key, entry in zip(value.keys, value.values):
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


def _git(root, *args):
    """The stdout of `git -C root ...`; Unreadable carries git's own message on failure."""
    done = subprocess.run(
        ["git", "-C", str(root), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if done.returncode != 0:
        raise Unreadable(f"git {args[0]} failed: {done.stderr.strip()}")
    return done.stdout


def changed_modules(root, base):
    """The .py paths under scripts/tests that HEAD adds or modifies against `base`."""
    listing = _git(
        root,
        "diff",
        "--name-only",
        "--no-renames",
        "--diff-filter=AM",
        f"{base}...HEAD",
        "--",
        TESTS,
    )
    return [line for line in listing.splitlines() if line.endswith(".py")]


def _judge(root, base):
    """(lines, status) for one run; Unreadable stops it."""
    registered = register_modules(_git(root, "show", f"HEAD:{REGISTER}"))
    lines = [f"dynamic-imports: register: {len(registered)} module(s) in {REGISTER_NAME} at HEAD"]
    paths = changed_modules(root, base)
    refused = []
    loaders = 0
    for path in paths:
        try:
            sites = loader_sites(_git(root, "show", f"HEAD:{path}"))
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
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=str(REPO), help="the repository to judge")
    parser.add_argument("--base", default="origin/dev", help="the ref the push is judged against")
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

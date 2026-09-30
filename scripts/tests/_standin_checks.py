"""The checks SPEC-298 relies on, as functions of the text they read (SPEC-298; ADR-298; #502).

Each takes the TEXT it judges, so a killer can run the real check on a copy with one member
planted. A check either returns or raises `AssertionError`; a member it cannot see is an escape.
"""

import ast
import re

ELEVATE = "DECKSTREAK_DEPLOY_ELEVATE"


def elevate_values(tree):
    """Every string the module gives `DECKSTREAK_DEPLOY_ELEVATE`, in a dict or a keyword; a value the
    tree cannot read as a constant is refused, so no environment escapes the derivation."""
    values = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Dict):
            for key, value in zip(node.keys, node.values, strict=True):
                if isinstance(key, ast.Constant) and key.value == ELEVATE:
                    values.append(value)
        if isinstance(node, ast.keyword) and node.arg == ELEVATE:
            values.append(node.value)
    found = []
    for value in values:
        assert isinstance(value, ast.Constant) and isinstance(value.value, str), (
            f"line {value.lineno}: an elevation value that is not a string constant"
        )
        found.append(value.value)
    return found


def derived_argv0(deploy_text, source_text):
    """The `argv[0]` values the stand-in receives from the calls the tests make: the word deploy.sh
    puts after the elevation command, when the tests give the setting empty, and the first word of
    each non-empty elevation command the tests give."""
    shape = re.compile(
        re.escape('"${HOST_CMD[@]}" ${ELEVATE_CMD[@]+"${ELEVATE_CMD[@]}"} ') + r"(\S+) -c "
    )
    found = shape.findall(deploy_text)
    assert len(found) == 1, f"deploy.sh makes {len(found)} host calls of the shape read: {found}"
    values = elevate_values(ast.parse(source_text))
    assert values, "no test environment gives the elevation setting"
    argv0 = set()
    for value in values:
        words = value.split()
        argv0.add(words[0] if words else found[0])
    return argv0


def check_stubs(source_text):
    """The only stub that runs its first argument is the host stand-in."""
    tree = ast.parse(source_text)
    runs_argv = re.compile(
        r"^\s*(?:(?:exec|command|eval|env|nohup|builtin)\s+)*\"?\$(?:@|\{@\}|1|\{1\})\"?(?=\s|$)",
        re.MULTILINE,
    )
    names = set()
    scanned = 0
    for node in ast.walk(tree):
        if not isinstance(node, ast.Assign):
            continue
        for part in ast.walk(node.value):
            if isinstance(part, ast.Constant) and isinstance(part.value, str):
                scanned += 1
                if runs_argv.search(part.value):
                    names |= {t.id for t in node.targets if isinstance(t, ast.Name)}
    assert names == {"HOST"}, f"stubs that run their argv: {sorted(names)}"
    return scanned


STARTERS = {
    "subprocess": {"run", "Popen", "call", "check_call", "check_output", "getoutput"},
    "os": {"system", "popen"},
}
ALLOWED = {
    "launch",
    "World.git",
    "NoDeployScriptNamesAPrivateValue.test_no_deploy_script_names_a_private_value",
}


def starters(tree):
    """Each call that can start a program, by the function that holds it."""
    found = {}

    def visit(node, scope):
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            scope = [*scope, node.name]
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
            base = node.func.value
            if isinstance(base, ast.Name) and base.id in STARTERS:
                named = node.func.attr in STARTERS[base.id]
                if named or (base.id == "os" and node.func.attr.startswith(("exec", "spawn"))):
                    found.setdefault(".".join(scope), []).append(node.lineno)
        for child in ast.iter_child_nodes(node):
            visit(child, scope)

    visit(tree, [])
    return found


def check_census(texts, target):
    """No call site but the helper, the git wrapper and the scrub starts a program."""
    tree = ast.parse(texts[target])
    imports = [
        node
        for node in ast.walk(tree)
        if (isinstance(node, ast.ImportFrom) and node.module in ("subprocess", "os", "pty"))
        or (isinstance(node, ast.Import) and any(a.asname for a in node.names))
    ]
    assert not imports, "a name that starts a program is imported bare"
    found = starters(tree)
    assert set(found) == ALLOWED, "a call site that starts a program is not an allowed one"
    return [line for lines in found.values() for line in lines]

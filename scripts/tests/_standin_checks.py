"""The checks SPEC-298 relies on, as functions of the text they read (SPEC-298; ADR-298; #502).

Each takes the TEXT it judges, so a killer can run the real check on a copy with one member
planted. A check either returns or raises `AssertionError`. The rule all of them follow: find every
member of the class by any spelling, or REFUSE what cannot be read. A check that passes a member it
cannot see is an escape.
"""

import ast
import collections
import re

ELEVATE = "DECKSTREAK_DEPLOY_ELEVATE"
HOST_CMD_NAMES = ("HOST_CMD", "DECKSTREAK_DEPLOY_HOST", "ELEVATE_CMD", ELEVATE)
OPAQUE = "\x00"

# --- the derivation -----------------------------------------------------------------------------

# The lines of deploy.sh that mention the host or elevation setting, each reviewed. The one call
# line names the word after the elevation command; every other mention must equal one of these.
REVIEWED_HOST_LINES = (
    re.compile(r"DECKSTREAK_DEPLOY_REPO DECKSTREAK_DEPLOY_HOST DECKSTREAK_DEPLOY_ELEVATE .*"),
    re.compile(r'\[ -n "\$\{DECKSTREAK_DEPLOY_HOST:-\}" \] \|\| die "DECKSTREAK_DEPLOY_HOST .*"'),
    re.compile(r'read -r -a HOST_CMD <<<"\$DECKSTREAK_DEPLOY_HOST"'),
    re.compile(r'read -r -a ELEVATE_CMD <<<"\$\{DECKSTREAK_DEPLOY_ELEVATE-sudo\}"'),
)
HOST_CALL = re.compile(
    re.escape('"${HOST_CMD[@]}" ${ELEVATE_CMD[@]+"${ELEVATE_CMD[@]}"} ')
    + r'(\S+) -c "\$script" deck-streak-host "\$@"'
)


def host_words(deploy_text):
    """The word after the elevation command in the one host call; any other line that mentions the
    host or the elevation setting, and is not reviewed, is refused by its line."""
    joined = deploy_text.replace("\\\n", " ")
    calls = []
    for number, line in enumerate(joined.splitlines(), 1):
        text = line.strip()
        if text.startswith("#") or not any(name in text for name in HOST_CMD_NAMES):
            continue
        call = HOST_CALL.fullmatch(text)
        if call:
            calls.append(call.group(1))
        elif not any(shape.fullmatch(text) for shape in REVIEWED_HOST_LINES):
            raise AssertionError(
                f"deploy.sh line {number} uses the host in a shape not read: {text}"
            )
    assert len(calls) == 1, f"deploy.sh makes {len(calls)} host calls of the shape read: {calls}"
    return calls[0]


def _mentions(node):
    """Whether a constant or a folded expression holds the setting's name."""
    return ELEVATE in (fold(node, {}) or "")


def elevate_values(tree):
    """Every string the tests give `DECKSTREAK_DEPLOY_ELEVATE`. Every mention of it (the name, a
    constant, a built key) is classified; a mention in a context not read is refused by its line."""
    env = _name_table(tree)
    found = []
    reviewed = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Dict):
            for key, value in zip(node.keys, node.values, strict=True):
                if key is not None and _is_setting(key, env):
                    _read_value(value, found)
                    reviewed.add(id(key))
        if isinstance(node, ast.keyword) and node.arg == ELEVATE:
            _read_value(node.value, found)
        if isinstance(node, ast.FunctionDef) and node.name in ("launch", "setting_names"):
            reviewed |= {id(part) for part in ast.walk(node)}
        if isinstance(node, ast.Assign) and _declares(node):
            reviewed |= {id(part) for part in ast.walk(node)}
    for node in ast.walk(tree):
        if id(node) in reviewed:
            continue
        name = isinstance(node, ast.Name) and node.id == "ELEVATE"
        built = isinstance(node, (ast.Constant, ast.JoinedStr, ast.BinOp)) and _mentions(node)
        if name or built:
            raise AssertionError(
                f"line {node.lineno}: the elevation setting is used in a shape not read"
            )
    return found


def _declares(node):
    return (
        len(node.targets) == 1
        and isinstance(node.targets[0], ast.Name)
        and node.targets[0].id == "ELEVATE"
        and isinstance(node.value, ast.Constant)
    )


def _is_setting(key, env):
    if isinstance(key, ast.Name) and key.id == "ELEVATE":
        return True
    return fold(key, env) == ELEVATE or (
        isinstance(key, ast.Constant) and _text(key.value) == ELEVATE
    )


def _read_value(value, found):
    assert isinstance(value, ast.Constant) and isinstance(value.value, str), (
        f"line {value.lineno}: an elevation value that is not a string constant"
    )
    found.append(value.value)


def derived_argv0(deploy_text, source_text):
    """The `argv[0]` values the stand-in receives from the calls the tests make: the word deploy.sh
    puts after the elevation command, when the tests give the setting empty, and the first word of
    each non-empty elevation command the tests give."""
    word = host_words(deploy_text)
    values = elevate_values(ast.parse(source_text))
    assert values, "no test environment gives the elevation setting"
    argv0 = set()
    for value in values:
        words = value.split()
        argv0.add(words[0] if words else word)
    return argv0


# --- folding text the tests build ---------------------------------------------------------------


def _text(value):
    return value.decode("latin-1") if isinstance(value, bytes) else value


def _name_table(tree):
    """Every name assigned a text anywhere in the module, folded, so a built stub is read."""
    table = {}
    held = {id(n) for f in ast.walk(tree) if isinstance(f, ast.FunctionDef) for n in ast.walk(f)}
    for _ in range(3):
        for node in ast.walk(tree):
            if id(node) in held:
                continue
            target = None
            if isinstance(node, ast.Assign) and len(node.targets) == 1:
                target = node.targets[0]
            elif isinstance(node, ast.AnnAssign) and node.value is not None:
                target = node.target
            if isinstance(target, ast.Name):
                text = fold(node.value, table)
                if text is not None:
                    table[target.id] = text
    return table


_METHODS_KEEP = {"encode", "decode", "strip", "lstrip", "rstrip", "lower", "upper", "format_map"}


def fold(node, table):
    """The text an expression builds, with OPAQUE for each piece that cannot be read; None when
    the expression is not text at all."""
    if isinstance(node, ast.Constant):
        return _text(node.value) if isinstance(node.value, (str, bytes)) else None
    if isinstance(node, ast.Name):
        return table.get(node.id)
    if isinstance(node, ast.JoinedStr):
        parts = []
        for part in node.values:
            if isinstance(part, ast.Constant):
                parts.append(part.value)
            else:
                inner = fold(part.value, table)
                parts.append(OPAQUE if inner is None else inner)
        return "".join(parts)
    if isinstance(node, ast.BinOp) and isinstance(node.op, (ast.Add, ast.Mod)):
        left = fold(node.left, table)
        if left is None:
            return None
        if isinstance(node.op, ast.Add):
            right = fold(node.right, table)
            return left + (OPAQUE if right is None else right)
        args = node.right.elts if isinstance(node.right, ast.Tuple) else [node.right]
        pieces = [fold(arg, table) for arg in args]
        pieces = [OPAQUE if piece is None else piece for piece in pieces]
        return re.sub(r"%[sdrx]", lambda _m: pieces.pop(0) if pieces else OPAQUE, left)
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
        return _fold_method(node, table)
    return None


def _fold_method(node, table):
    attr = node.func.attr
    receiver = fold(node.func.value, table)
    if receiver is None:
        return None
    args = [fold(arg, table) for arg in node.args]
    args = [OPAQUE if arg is None else arg for arg in args]
    if attr == "replace" and len(args) >= 2:
        return receiver.replace(args[0], args[1])
    if attr == "format":
        pieces = list(args)
        return re.sub(r"\{[^{}]*\}", lambda _m: pieces.pop(0) if pieces else OPAQUE, receiver)
    if attr == "join" and node.args:
        seq = node.args[0]
        if isinstance(seq, (ast.List, ast.Tuple)):
            items = [fold(e, table) for e in seq.elts]
            return receiver.join(OPAQUE if item is None else item for item in items)
        return receiver + OPAQUE
    if attr in _METHODS_KEEP:
        return receiver
    return receiver + OPAQUE


# --- the stub scan ------------------------------------------------------------------------------

EXPANSION = re.compile(
    r"\$[@*#1-9]|\$\{[@*1-9]|\$\{#\}|\$\{!\d|BASH_ARGV|BASH_ARGC|/proc/(?:self|\$\$)/cmdline"
    r"|\$\x00|\$\{\x00"
)
PROBE = "$(ls \"$STUB_ROOT/releases\" | tr '\\n' ,)"
LOG_ECHO = re.compile(
    r'echo "(?:[^"`$\\\x00]|\$\*|\$[1-9]|\$[a-z_]+|\$\{[a-z_]+\}|@PROBE@)*"'
    r' >> "\$STUB_LOG/[\w.-]+\.log"'
)
# Reviewed lines that hold an argument expansion and do not run what the arguments name.
REVIEWED_LINES = frozenset(
    [
        '[ -f "$3" ] || exit 3',
        'case "$1 $2" in',
        'case "$1" in',
        'case "${1:-}" in',
        "tag=$3; dir=",
        'while [ $# -gt 0 ]; do [ "$1" = --dir ] && dir=$2; shift; done',
        '[ "$2" = "$STUB_BAD_UNIT" ] && [ "$cur" = "$STUB_BAD_TAG" ] && exit 1',
        'for u in "$@"; do',
        '[ "$1" = --adapter ] && adapter=$2',
        '[ "$1" = --config ] && conf=$2',
        'all="$*"; first=$1; conf=; adapter=',
        "while [ $# -gt 0 ]; do",
        'exec /usr/bin/mv "$@"',
        'exec /usr/bin/@NAME@ "$@"',
        "src=${@: -2:1}; dst=${@: -1}",
        '[ "${1-}" != "$shape" ] || ok=1',
        'printf \'%s\\n\' "${1-}" >> "$STUB_LOG/host-argv0.log"',
        "printf 'host stand-in: refusing %s: not a command the deploy tests use\\n' \"${1-}\" >&2",
        (
            'trap \'printf "%s\\0%s\\0%s\\0" "$record_run" "${BASH_SOURCE[0]:-}" '
            '"$BASH_COMMAND" >>"$RECORD_LOG"\' DEBUG'
        ),
    ]
)
REVIEWED_PATTERNS = (
    re.compile(r"\"\$\(dirname \"\$\{BASH_SOURCE\[0\]\}\"\)\"|.*\$\{BASH_SOURCE\[0\]\}.*"),
)
# The stubs that run what their arguments name, and why each is not a way to a privilege.
ARGUMENT_RUNNERS = {
    'HOST: exec "$@"': "the host stand-in, which refuses any argv[0] not on its list",
    'PYTHON_TRACE: \' "$@"': (
        "hands its first argument to runpy, which runs a python file; it cannot start a binary"
    ),
    'sourced: set -a; . "$1"; set +a; shift; . "$@"': (
        "the tests' own launch of a deploy script, through the one helper"
    ),
}
LISTED = collections.Counter(dict.fromkeys(ARGUMENT_RUNNERS, 1))
REVIEWED_INSTALLS = collections.Counter(
    {
        "HOST": 1,
        "UNREADY_CURL": 1,
        "body": 2,
        "MOVE_ONTO_BLOCK_FAILS": 1,
        "LOGGED.replace('@NAME@', name)": 1,
        "self.PYTHON_TRACE": 1,
        "LOGGED.replace('@NAME@', 'mv')": 1,
    }
)


def _reviewed(line):
    probe = line.replace(PROBE, "@PROBE@")
    return (
        line in REVIEWED_LINES
        or bool(LOG_ECHO.fullmatch(probe))
        or any(shape.fullmatch(line) for shape in REVIEWED_PATTERNS)
    )


def _owner(stack):
    return stack[-1] if stack else "text"


def stub_lines(source_text):
    """Every line of every text the module holds or builds that carries an argument expansion, and
    the count of texts read. Constants, concatenations, formats, joins, f-strings and method calls
    on text are folded; a piece that cannot be read becomes an opaque mark, so a `$` before one
    counts as an expansion."""
    tree = ast.parse(source_text)
    table = _name_table(tree)
    flagged = collections.Counter()
    count = [0]

    def scan(text, owner):
        count[0] += 1
        for raw in text.splitlines():
            line = raw.strip()
            if EXPANSION.search(line) and not _reviewed(line):
                flagged[f"{owner}: {line}"] += 1

    def visit(node, stack):
        name = None
        if isinstance(node, ast.Assign) and len(node.targets) == 1:
            name = node.targets[0]
        elif isinstance(node, ast.AnnAssign):
            name = node.target
        if isinstance(name, ast.Name):
            stack = [*stack, name.id]
        if isinstance(node, ast.Call) and ast.unparse(node) in REVIEWED_INSTALLS:
            count[0] += 1
            return
        foldable = isinstance(node, (ast.Constant, ast.JoinedStr, ast.BinOp)) or (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and fold(node.func.value, table) is not None
        )
        if foldable:
            text = fold(node, table)
            if text is not None:
                scan(text, _owner(stack))
                return
        for child in ast.iter_child_nodes(node):
            visit(child, stack)

    visit(tree, [])
    return flagged, count[0]


def installs(source_text):
    """The second argument of every `.script(` call, unparsed."""
    found = collections.Counter()
    for node in ast.walk(ast.parse(source_text)):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "script"
            and len(node.args) >= 2
        ):
            found[ast.unparse(node.args[1])] += 1
    return found


def check_stubs(source_text):
    """Only the listed stubs hold an argument expansion outside a reviewed logging shape, and every
    way a stub is installed is reviewed. Returns the number of texts read."""
    flagged, count = stub_lines(source_text)
    assert flagged == LISTED, (
        f"stub lines that hold an argument expansion and are not reviewed: "
        f"{sorted((flagged - LISTED).elements())}; "
        f"reviewed lines no longer found: "
        f"{sorted((LISTED - flagged).elements())}"
    )
    got = installs(source_text)
    assert got == REVIEWED_INSTALLS, (
        f"stub installs not reviewed: {sorted((got - REVIEWED_INSTALLS).elements())}"
    )
    return count


# --- the launch census --------------------------------------------------------------------------

SAFE = {
    "os": {"environ", "fsencode", "killpg", "link", "mkfifo", "readlink"},
    "subprocess": {"CompletedProcess", "DEVNULL", "PIPE", "TimeoutExpired"},
    "sys": {"executable"},
}
WATCHED = {
    "os", "subprocess", "pty", "asyncio", "multiprocessing", "importlib", "ctypes", "runpy",
    "concurrent", "posix", "sys",
}  # fmt: skip
UNSAFE_IMPORTS = WATCHED - {"sys", "os", "subprocess"}
UNSAFE_NAMES = {
    "__import__", "importlib", "eval", "exec", "compile", "globals", "vars", "locals", "setattr",
    "delattr", "builtins",
}  # fmt: skip
GIT_COMMANDS = {"init", "clone", "checkout", "push", "add", "commit", "tag"}
GIT_FLAGS = {"--bare", "-b", "-q", "-B", "-A", "-m", "-am", "-a"}
# The reviewed count of start sites per module, scope and kind.
EXPECTED = {
    "<target>": {
        ("launch", "subprocess.Popen"): 1,
        ("World.git", "subprocess.run"): 1,
        (
            "NoDeployScriptNamesAPrivateValue.test_no_deploy_script_names_a_private_value",
            "subprocess.run",
        ): 2,
    },
    "test_deploy_standin": {
        (
            "TheStandInRefusesWhatItDoesNotKnow.test_a_command_outside_the_list_is_refused_by_name_and_run_by_nothing",
            "subprocess.run",
        ): 1,
        ("TheStandInRefusesWhatItDoesNotKnow.test_a_listed_command_is_run", "subprocess.run"): 1,
    },
}


def _scopes(tree):
    """Each node with the name of the function or class that holds it."""
    out = []

    def visit(node, scope):
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            scope = [*scope, node.name]
        out.append((node, ".".join(scope) or "<module>"))
        for child in ast.iter_child_nodes(node):
            visit(child, scope)

    visit(tree, [])
    return out


def sites(text, module):
    """Every site in a module that can start a program or hide one, by scope and kind."""
    tree = ast.parse(text)
    found = collections.Counter()
    based = {id(n.value) for n in ast.walk(tree) if isinstance(n, ast.Attribute)}
    for node, scope in _scopes(tree):
        if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name):
            base = node.value.id
            if base in SAFE and node.attr not in SAFE[base]:
                found[(scope, f"{base}.{node.attr}")] += 1
            if node.attr in WATCHED:
                found[(scope, f"attribute {node.attr}")] += 1
        elif isinstance(node, ast.Attribute) and node.attr in WATCHED:
            found[(scope, f"attribute {node.attr}")] += 1
        if isinstance(node, ast.Name) and node.id in UNSAFE_NAMES:
            found[(scope, f"name {node.id}")] += 1
        if isinstance(node, ast.Name) and node.id == "subprocess" and id(node) not in based:
            found[(scope, "subprocess")] += 1
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            roots = (
                [a.name.split(".")[0] for a in node.names]
                if isinstance(node, ast.Import)
                else [(node.module or "").split(".")[0]]
            )
            for root in roots:
                if root in UNSAFE_IMPORTS:
                    found[(scope, f"import {root}")] += 1
            if isinstance(node, ast.ImportFrom) and (node.module or "") in SAFE:
                found[(scope, f"from {node.module}")] += 1
            if isinstance(node, ast.Import) and any(
                a.asname and a.name in WATCHED for a in node.names
            ):
                found[(scope, "import alias")] += 1
        if (
            isinstance(node, ast.Constant)
            and isinstance(node.value, (str, bytes))
            and _text(node.value) in WATCHED
        ):
            found[(scope, f"text {_text(node.value)}")] += 1
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "getattr"
        ):
            second = node.args[1] if len(node.args) > 1 else None
            ok = (
                isinstance(second, ast.Constant)
                and isinstance(second.value, str)
                and second.value not in WATCHED
            )
            if not ok:
                found[(scope, "getattr")] += 1
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
            if node.func.attr == "git":
                _read_git_call(node, module)
    return found


def _read_git_call(node, module):
    """A `.git(...)` call is the wrapper's one way to start git; its command and flags must be
    reviewed, and a starred argument or an unreviewed keyword is refused."""
    where = f"{module} line {node.lineno}"
    for keyword in node.keywords:
        if keyword.arg != "cwd":
            raise AssertionError(f"{where}: git keyword {keyword.arg} is not reviewed")
    for index, arg in enumerate(node.args):
        if isinstance(arg, ast.Starred):
            raise AssertionError(f"{where}: a starred git argument is not read")
        folded = fold(arg, {})
        if index == 0 and not (isinstance(arg, ast.Constant) and folded in GIT_COMMANDS):
            raise AssertionError(f"{where}: the git command is not one reviewed")
        if folded is not None and folded.startswith("-"):
            if not (isinstance(arg, ast.Constant) and folded in GIT_FLAGS):
                raise AssertionError(f"{where}: git flag {folded} is not reviewed")


def imports_target(text, target):
    """How a module relates to the target: 'imports', 'names it' (a mention it cannot read as an
    import) or None."""
    if target not in text:
        return None
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return "names it"
    for node in ast.walk(tree):
        if isinstance(node, ast.Import) and any(a.name.split(".")[0] == target for a in node.names):
            return "imports"
        if isinstance(node, ast.ImportFrom) and (
            (node.module or "").split(".")[-1] == target
            or any(a.name == target for a in node.names)
        ):
            return "imports"
    return "names it"


def check_census(texts, target, expected=None, own=None):
    """No call site but the reviewed ones starts a program, counted per scope and kind, in the
    target and in every module that imports it. A module that names the target without a readable
    import is refused by name. Returns the number of start sites counted."""
    expected = EXPECTED if expected is None else expected
    total = 0
    for module, text in sorted(texts.items()):
        if module == "_standin_checks":
            continue
        relation = "imports" if module == target else imports_target(text, target)
        if relation is None:
            continue
        if relation != "imports":
            raise AssertionError(f"{module} names {target} without an import the census can read")
        found = sites(text, module)
        want = collections.Counter(
            {
                key: n
                for key, n in (
                    expected.get(module, expected.get("<target>") if module == target else {})
                ).items()
            }
        )
        assert found == want, (
            f"{module}: sites not reviewed: {sorted((found - want).items())}; "
            f"reviewed sites missing: {sorted((want - found).items())}"
        )
        total += sum(found.values())
    return total

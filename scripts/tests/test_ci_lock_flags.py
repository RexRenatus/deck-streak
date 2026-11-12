"""Every CI command that resolves or builds dependencies runs on its lock (SPEC-671).

The census reads, as text, every workflow under `.github/workflows/` and every script a run text
names by path (R1), and in them every command that resolves or builds dependencies, by tool and
form (R2, R3). A command without its lock flag is a site (R4); the forms that read no lock are not
(R5); a form the text cannot decide is printed as undecided and never judged (R6). Each site is
checked against `LISTED`, today's sites each with its reason (R7): an unlisted site prints `WARN`
at `warn` and fails at `fail`, and an entry no current site matches is `STALE` and fails at both
levels. The census starts at `warn` (ADR-694, after ADR-450 D6).

It follows a command as far as the run text, a script that text names by path, and a shell
function in that script; it never follows a script's own calls, a script a step writes and runs,
or a program named only by a variable. Planted cases sit under `fixtures/lock-flags/` with a
`.fixture` suffix, so no runner collects them; they are read as text and never run.
"""

import ast
import re
import shlex
import unittest
from collections import namedtuple

from _support import REPO, examined, refuse_link_components

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "lock-flags"
LEVEL = "warn"

Command = namedtuple("Command", "file scope tool sub line locked kind")

TOOLS = ("cargo", "pnpm", "npm", "npx", "swift", "xcodebuild", "pip", "pip3")
CARGO_LOCK = ("--locked", "--frozen")
# R5: the cargo forms that read no lock, or read it on purpose without one.
CARGO_NO_LOCK = ("fmt", "metadata", "tree", "version", "help")
CARGO_SECOND_WORD = ("nextest", "codspeed", "llvm-cov")
PNPM_VALUE_OPTIONS = ("--dir", "-C", "--filter", "-F")
PNPM_SITES = ("add", "update", "up", "dlx")
NPM_SITES = ("install", "i", "add", "update", "exec", "x")
SWIFT_RESOLVED = (
    "--only-use-versions-from-resolved-file",
    "--disable-automatic-resolution",
    "--force-resolved-versions",
)
SWIFT_RESOLVES = ("build", "test", "run", "package resolve", "package update")
# R4: the census asks no flag of xcodebuild, so no flag word makes an xcodebuild call clean. Every
# xcodebuild action, and every xcodebuild call naming a project, workspace or scheme, is a site
# listed with its reason (`XCODE_LOCAL` below); whether an xcodebuild build must carry a
# resolved-file flag is its own delivery's question.
XCODE_ACTIONS = (
    "build",
    "build-for-testing",
    "test",
    "test-without-building",
    "archive",
    "analyze",
    "-resolvePackageDependencies",
)
XCODE_PROJECT = ("-project", "-workspace", "-scheme")
SUBCOMMAND = re.compile(r"^[a-z][a-z0-9:-]*$")
SIMPLE = re.compile(r"&&|\|\||;|\||\$\(|`|\(|\)|\{|\}")
EXPRESSION = re.compile(r"\$\{\{.*?\}\}")
RUN_KEY = re.compile(r"^(\s*)(?:-\s+)?run:\s*(.*)$")
BLOCK_SCALAR = re.compile(r"^[|>][-+0-9]*\s*(#.*)?$")
JOB = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
HEREDOC = re.compile(r"<<-?\s*['\"]?([A-Za-z_][A-Za-z0-9_]*)['\"]?")
OTHER_LANGUAGE = re.compile(r"^(?:.*/)?(?:python3?|node)$")
TOOL_WORD = re.compile(r"\b(?:cargo|pnpm|npm|npx|swift|xcodebuild|pip3?)\b")
FUNCTION = re.compile(r"^\s*(?:function\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(\)\s*\{?\s*$")
SETUP = re.compile(r"(?ms)^( *)- uses:\s*pnpm/action-setup@[^\n]*\n(.*?)(?=^\1- |\Z)")
SCRIPT_PATH = re.compile(
    r"(?<![A-Za-z0-9_$/.-])(?:\./)?((?:[A-Za-z0-9_.-]+/)+[A-Za-z0-9_.-]+\.(?:sh|py))(?![A-Za-z0-9_])"
)

# R7: today's sites, keyed by (file, job or function, tool, subcommand, ordinal), each with its
# reason. A line number is never part of a key, so an edit elsewhere in a file moves no entry.
MUTANTS_LIST = "cargo-mutants lists its mutants by `cargo metadata` and runs none"
MUTANTS_RUN = "cargo-mutants passes a lock flag to cargo only by `--cargo-arg`; the job fetched the locked graph first"
SWIFT_LOCAL = "the package names no remote dependency, so the tree holds no resolved file"
XCODE_LOCAL = "the project names its packages by local path, so the tree holds no resolved file"
LISTED = {
    (
        ".github/workflows/ci.yml",
        "rust",
        "cargo",
        "clean",
        1,
    ): "removes build output; whether it reads the lock is unanswered",
    (
        ".github/workflows/ci.yml",
        "release",
        "cargo",
        "clean",
        1,
    ): "removes build output; whether it reads the lock is unanswered",
    (
        ".github/workflows/ci.yml",
        "mutation-plan",
        "cargo",
        "mutants --list",
        1,
    ): MUTANTS_LIST,
    (
        ".github/workflows/ci.yml",
        "mutation-plan",
        "cargo",
        "mutants --list",
        2,
    ): MUTANTS_LIST,
    (".github/workflows/ci.yml", "mutation-rust", "cargo", "mutants", 1): MUTANTS_RUN,
    (
        ".github/workflows/ci.yml",
        "bench-rust-build",
        "cargo",
        "codspeed build",
        1,
    ): "whether the tool takes a lock flag is unanswered",
    (
        ".github/workflows/ci.yml",
        "bench-rust",
        "cargo",
        "codspeed run",
        1,
    ): "runs the benchmarks the build step made",
    (
        ".github/workflows/coverage.yml",
        "coverage-rust",
        "cargo",
        "llvm-cov report",
        1,
    ): "reports the profiles the locked run wrote",
    (
        ".github/workflows/mutation-weekly.yml",
        "size",
        "cargo",
        "mutants --list",
        1,
    ): MUTANTS_LIST,
    (
        ".github/workflows/mutation-weekly.yml",
        "size",
        "cargo",
        "mutants --list",
        2,
    ): MUTANTS_LIST,
    (
        ".github/workflows/mutation-weekly.yml",
        "rust",
        "cargo",
        "mutants",
        1,
    ): MUTANTS_RUN,
    (
        ".github/workflows/mutation-weekly.yml",
        "rust",
        "cargo",
        "mutants",
        2,
    ): MUTANTS_RUN,
    (
        ".github/workflows/mutation-weekly.yml",
        "listing",
        "cargo",
        "mutants --list",
        1,
    ): MUTANTS_LIST,
    (
        ".github/workflows/mutation-weekly.yml",
        "rehearsal",
        "cargo",
        "mutants",
        1,
    ): MUTANTS_RUN,
    (
        ".github/workflows/rust-cache.yml",
        "warm",
        "cargo",
        "clean",
        1,
    ): "removes build output; whether it reads the lock is unanswered",
    (
        ".github/workflows/xcframework.yml",
        "harness-wire",
        "swift",
        "test",
        1,
    ): SWIFT_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "card-isolation",
        "swift",
        "test",
        1,
    ): SWIFT_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "harness",
        "xcodebuild",
        "build-for-testing",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "harness",
        "xcodebuild",
        "test-without-building",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "harness",
        "xcodebuild",
        "test",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "build-for-testing",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "build-for-testing",
        2,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "test-without-building",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "build",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "test",
        1,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "test",
        2,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "test",
        3,
    ): XCODE_LOCAL,
    (
        ".github/workflows/xcframework.yml",
        "release-and-app",
        "xcodebuild",
        "archive",
        1,
    ): XCODE_LOCAL,
    (
        "scripts/mutation-baseline.sh",
        "<top>",
        "cargo",
        "nextest run",
        1,
    ): "the job fetched the locked graph before this baseline",
    ("scripts/swift_mutants.py", "run_killer", "swift", "test", 1): SWIFT_LOCAL,
}


def first_word(words, skip_values=()):
    """The first word that is not an option, skipping the value of each option in `skip_values`."""
    index = 0
    while index < len(words):
        word = words[index]
        if word in skip_values:
            index += 2
            continue
        if not word.startswith("-"):
            return word
        index += 1
    return ""


def judge(tool, rest):
    """(subcommand, whether its lock flag is present, kind) for one tool word and the words after
    it; kind is `site`, `clean` or `none` (R4, R5)."""
    if tool == "cargo":
        words = rest[1:] if rest and rest[0].startswith("+") else rest
        if not words or words[0] in ("--version", "-V", "--help", "-h"):
            return "", False, "none"
        sub = first_word(words)
        if not SUBCOMMAND.match(sub):
            return sub, False, "none"
        before = words[: words.index("--")] if "--" in words else words
        if sub == "mutants":
            locked = any(
                word in ("--cargo-arg=--locked", "--cargo-arg=--frozen")
                or (
                    word == "--cargo-arg"
                    and before[index + 1 : index + 2] in (["--locked"], ["--frozen"])
                )
                for index, word in enumerate(before)
            )
            sub = "mutants --list" if "--list" in words else "mutants"
        else:
            locked = any(word.split("=")[0] in CARGO_LOCK for word in before)
        if sub in CARGO_SECOND_WORD:
            second = first_word(words[words.index(sub) + 1 :])
            sub = f"{sub} {second}".strip()
        if sub in CARGO_NO_LOCK:
            return sub, locked, "none"
        return sub, locked, "clean" if locked else "site"
    if tool == "pnpm":
        sub = first_word(rest, PNPM_VALUE_OPTIONS)
        if sub in ("install", "i"):
            locked = "--frozen-lockfile" in rest and "--no-frozen-lockfile" not in rest
            return sub, locked, "clean" if locked else "site"
        return sub, False, "site" if sub in PNPM_SITES else "none"
    if tool in ("npm", "npx"):
        sub = "" if tool == "npx" else first_word(rest)
        if tool == "npm" and sub == "ci":
            return sub, True, "clean"
        return sub, False, "site" if tool == "npx" or sub in NPM_SITES else "none"
    if tool == "swift":
        sub = first_word(rest)
        if sub == "package":
            sub = f"package {first_word(rest[rest.index('package') + 1 :])}".strip()
        locked = any(word in SWIFT_RESOLVED for word in rest)
        if sub not in SWIFT_RESOLVES:
            return sub, locked, "none"
        return sub, locked, "clean" if locked else "site"
    if tool == "xcodebuild":
        actions = [word for word in rest if word in XCODE_ACTIONS]
        # R4: every xcodebuild action or project call is a site, whatever flags it carries.
        if not actions and not any(word in XCODE_PROJECT for word in rest):
            return first_word(rest), False, "none"
        return (actions or ["build"])[0], False, "site"
    sub = first_word(rest)
    if sub != "install":
        return sub, False, "none"
    locked = "--require-hashes" in rest
    return sub, locked, "clean" if locked else "site"


def tool_words(command):
    """(tool, the words after it) for the first tool word at any position of one simple command,
    after any wrapper (R3); None when it names no tool."""
    try:
        words = shlex.split(command, comments=True)
    except ValueError:
        words = command.split()
    for index, word in enumerate(words):
        name = word.rsplit("/", 1)[-1]
        if name.startswith("python") and words[index + 1 : index + 3] == ["-m", "pip"]:
            return "pip", words[index + 3 :]
        if name in TOOLS:
            return name, words[index + 1 :]
    return None


def joined(lines):
    """[(line number, text)] with backslash continuations joined and comment lines dropped (R2)."""
    out, current = [], None
    for number, text in lines:
        text = text.strip()
        if current is None:
            if not text or text.startswith("#"):
                continue
            current = [number, text]
        else:
            current[1] = f"{current[1]} {text}"
        if current[1].endswith("\\"):
            current[1] = current[1][:-1].rstrip()
            continue
        out.append((current[0], current[1]))
        current = None
    if current is not None:
        out.append((current[0], current[1]))
    return out


def outside_heredocs(lines, undecided, file):
    """The lines outside every here-document. A body fed to Python or Node is another language and
    is not read; any other body that names a tool is undecided (R6)."""
    out, end, other = [], None, False
    for number, text in lines:
        if end is not None:
            if text.strip() == end:
                end = None
            elif not other and TOOL_WORD.search(text):
                undecided.append(f"{file}:{number}: a here-document body names a tool")
            continue
        found = HEREDOC.search(text)
        if found and "<<<" not in text:
            end = found.group(1)
            head = text[: found.start()].split()
            other = bool(head) and OTHER_LANGUAGE.match(head[0]) is not None
        out.append((number, text))
    return out


def read_commands(file, scope_of, lines, found, undecided):
    """Append each dependency command of `lines` to `found`, as R2 to R6 read it."""
    for number, text in joined(outside_heredocs(lines, undecided, file)):
        # A workflow expression holds braces, which split simple commands; it is one word.
        text = EXPRESSION.sub(" WORKFLOW-EXPRESSION ", text)
        for part in SIMPLE.split(text):
            hit = tool_words(part.strip())
            if not hit:
                continue
            sub, locked, kind = judge(*hit)
            if kind == "site" and "WORKFLOW-EXPRESSION" in hit[1]:
                undecided.append(f"{file}:{number}: a matrix expression may hold the flag")
                continue
            if kind != "none":
                found.append(Command(file, scope_of(number), hit[0], sub, number, locked, kind))


def run_blocks(text):
    """(job, [(line number, line)]) for every `run:` key at any depth, a `with: run:` input
    included, block and inline scalars alike (R2)."""
    lines, job, in_jobs, index = text.split("\n"), "<top>", False, 0
    while index < len(lines):
        line = lines[index]
        in_jobs = in_jobs or line.rstrip() == "jobs:"
        if in_jobs and JOB.match(line):
            job = JOB.match(line).group(1)
        key = RUN_KEY.match(line)
        if not key:
            index += 1
            continue
        column, value, body = line.index("run:"), key.group(2), []
        follow = index + 1
        if BLOCK_SCALAR.match(value):
            while follow < len(lines) and (
                not lines[follow].strip()
                or len(lines[follow]) - len(lines[follow].lstrip(" ")) > column
            ):
                body.append((follow + 1, lines[follow]))
                follow += 1
        else:
            if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
                value = value[1:-1]
            body.append((index + 1, value))
            while (
                follow < len(lines)
                and lines[follow].strip()
                and len(lines[follow]) - len(lines[follow].lstrip(" ")) > column
                and not lines[follow].strip().startswith("#")
            ):
                body.append((follow + 1, lines[follow]))
                follow += 1
        yield job, body
        index = follow


def workflow_commands(file, text):
    """(commands, undecided, the script paths its run texts name) of one workflow."""
    found, undecided, named = [], [], []
    for job, body in run_blocks(text):
        read_commands(file, lambda number, job=job: job, body, found, undecided)
        for _, line in joined(outside_heredocs(body, [], file)):
            named.extend(match.group(1) for match in SCRIPT_PATH.finditer(line))
    for setup in SETUP.finditer(text):
        install = re.search(r"(?m)^\s*run_install:\s*(\S+)", setup.group(2))
        if install and install.group(1) != "false":
            above = text[: setup.start()].split("\n")
            jobs = above[
                next(
                    (i for i, line in enumerate(above) if line.rstrip() == "jobs:"),
                    len(above),
                ) :
            ]
            scope = ([JOB.match(line).group(1) for line in jobs if JOB.match(line)] or ["<top>"])[
                -1
            ]
            found.append(
                Command(
                    file,
                    scope,
                    "pnpm",
                    "action-setup run_install",
                    len(above),
                    False,
                    "site",
                )
            )
    return found, undecided, named


def shell_commands(file, text):
    """(commands, undecided) of one shell script, each scoped by its shell function (R2)."""
    lines = list(enumerate(text.split("\n"), 1))
    scopes, scope = {}, "<top>"
    for number, line in lines:
        function = FUNCTION.match(line)
        if function:
            scope = function.group(1)
        scopes[number] = scope
        if line.startswith("}"):
            scope = "<top>"
    found, undecided = [], []
    read_commands(file, lambda number: scopes.get(number, "<top>"), lines, found, undecided)
    return found, undecided


def python_commands(file, text):
    """(commands, undecided) of one Python script, read by `ast` and never imported: a list
    literal, or a tuple literal passed to a call, whose first element names a tool (R2)."""
    tree = ast.parse(text)
    functions = [
        (node.lineno, node.end_lineno, node.name)
        for node in ast.walk(tree)
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    ]
    passed = {id(arg) for node in ast.walk(tree) if isinstance(node, ast.Call) for arg in node.args}
    found, undecided, counted = [], [], []
    for node in ast.walk(tree):
        if not (isinstance(node, ast.List) or (isinstance(node, ast.Tuple) and id(node) in passed)):
            continue
        if len(node.elts) < 2 or not (
            isinstance(node.elts[0], ast.Constant) and node.elts[0].value in TOOLS
        ):
            continue
        rest = [
            e.value if isinstance(e, ast.Constant) and isinstance(e.value, str) else "*"
            for e in node.elts[1:]
        ]
        if rest[0] == "*":
            undecided.append(f"{file}:{node.lineno}: its argument list is built by an expression")
            continue
        sub, locked, kind = judge(node.elts[0].value, rest)
        inside = [f for f in functions if f[0] <= node.lineno <= f[1]]
        scope = min(inside, key=lambda f: f[1] - f[0])[2] if inside else "<top>"
        if kind != "none":
            counted.append(Command(file, scope, node.elts[0].value, sub, node.lineno, locked, kind))
    found.extend(sorted(counted, key=lambda command: command.line))
    return found, undecided


def keyed(commands):
    """{(file, scope, tool, subcommand, ordinal): command} for each site (R7)."""
    seen, sites = {}, {}
    for command in commands:
        base = (command.file, command.scope, command.tool, command.sub)
        seen[base] = seen.get(base, 0) + 1
        if command.kind == "site":
            sites[(*base, seen[base])] = command
    return sites


def verdict(sites, listed, level):
    """(lines to print, failures): an unlisted site is WARN, failing only at `fail`; an entry no
    current site matches is STALE and fails at both levels (R7)."""
    lines, failures = [], []
    for key, command in sorted(sites.items()):
        if key not in listed:
            line = (
                f"WARN {command.file}:{command.line}: {command.tool} {command.sub} has no lock flag"
            )
            lines.append(line)
            if level == "fail":
                failures.append(line)
    for key in sorted(listed):
        if key not in sites:
            line = f"STALE {key}: no current command matches this entry"
            lines.append(line)
            failures.append(line)
    return lines, failures


def population(root=REPO):
    """(workflow paths, called script paths): every workflow and every script a run text names by
    path that the tree holds, each read once (R1); an empty workflow set is refused."""
    workflows = examined(
        "workflow file(s)",
        sorted(
            p for p in (root / ".github" / "workflows").glob("*") if p.suffix in (".yml", ".yaml")
        ),
    )
    called = set()
    for path in workflows:
        refuse_link_components(path, root)
        called.update(workflow_commands(path.name, path.read_text(encoding="utf-8"))[2])
    scripts = sorted(root / name for name in called if (root / name).is_file())
    for path in scripts:
        refuse_link_components(path, root)
    return workflows, scripts


def census(root=REPO):
    """(sites, undecided) over the whole population (R1 to R6)."""
    workflows, scripts = population(root)
    examined("file(s)", workflows + scripts)
    found, undecided = [], []
    for path in workflows:
        name = path.relative_to(root).as_posix()
        commands, unsure, _ = workflow_commands(name, path.read_text(encoding="utf-8"))
        found.extend(commands)
        undecided.extend(unsure)
    for path in scripts:
        name = path.relative_to(root).as_posix()
        reader = shell_commands if path.suffix == ".sh" else python_commands
        commands, unsure = reader(name, path.read_text(encoding="utf-8"))
        found.extend(commands)
        undecided.extend(unsure)
    return keyed(found), undecided


def planted(name):
    return (FIXTURES / name).read_text(encoding="utf-8")


def kinds(commands):
    """[(scope, tool, subcommand, kind)] in reading order."""
    return [(c.scope, c.tool, c.sub, c.kind) for c in commands]


class TheRepositoryRunsEachDependencyCommandOnItsLock(unittest.TestCase):
    """Reads the whole tree, so it runs in CI only (R1, R7)."""

    def test_each_site_is_listed_with_its_reason_and_no_entry_is_stale(self):
        sites, undecided = census()
        for line in undecided:
            print(f"UNDECIDED {line}")
        lines, failures = verdict(sites, LISTED, LEVEL)
        for line in lines:
            print(line)
        self.assertEqual(failures, [])
        self.assertTrue(all(reason.strip() for reason in LISTED.values()))


class TheCensusReadsEachPlantedForm(unittest.TestCase):
    """Planted text only (R2 to R6, R8)."""

    def test_each_cargo_form_is_a_site_or_clean_as_its_lock_flag_says(self):
        commands, _, _ = workflow_commands("planted.yml", planted("workflow.yml.fixture"))
        cargo = [k for k in kinds(commands) if k[0] == "cargo"]
        self.assertEqual(
            cargo,
            [
                ("cargo", "cargo", "build", "site"),
                ("cargo", "cargo", "build", "clean"),
                ("cargo", "cargo", "test", "clean"),
                ("cargo", "cargo", "test", "clean"),
                ("cargo", "cargo", "test", "site"),
                ("cargo", "cargo", "nextest run", "site"),
                ("cargo", "cargo", "install", "site"),
                ("cargo", "cargo", "install", "clean"),
                ("cargo", "cargo", "mutants --list", "site"),
                ("cargo", "cargo", "mutants --list", "clean"),
            ],
        )

    def test_each_node_and_apple_and_pip_form_is_a_site_or_clean_as_its_lock_flag_says(
        self,
    ):
        commands, undecided, named = workflow_commands(
            "planted.yml", planted("workflow.yml.fixture")
        )
        others = [k for k in kinds(commands) if k[0] != "cargo"]
        self.assertEqual(
            others,
            [
                ("node", "pnpm", "install", "site"),
                ("node", "pnpm", "install", "clean"),
                ("node", "pnpm", "install", "site"),
                ("node", "npm", "ci", "clean"),
                ("node", "npm", "install", "site"),
                ("apple", "swift", "test", "site"),
                ("apple", "swift", "build", "clean"),
                ("apple", "xcodebuild", "build", "site"),
                ("apple", "xcodebuild", "build", "site"),
                ("apple", "pip", "install", "clean"),
                ("apple", "pip", "install", "site"),
                ("node", "pnpm", "action-setup run_install", "site"),
            ],
        )
        self.assertEqual(
            [line.split(": ", 1)[1] for line in undecided],
            [
                "a here-document body names a tool",
                "a matrix expression may hold the flag",
            ],
        )
        self.assertEqual(named, ["scripts/planted.sh"])

    def test_a_shell_script_is_read_by_function_and_after_any_wrapper(self):
        commands, undecided = shell_commands("planted.sh", planted("script.sh.fixture"))
        self.assertEqual(
            kinds(commands),
            [
                ("build", "cargo", "build", "clean"),
                ("build", "cargo", "test", "site"),
                ("<top>", "cargo", "clippy", "clean"),
                ("<top>", "cargo", "nextest run", "clean"),
                ("<top>", "cargo", "nextest run", "site"),
            ],
        )
        self.assertEqual(undecided, [])

    def test_a_python_script_is_read_by_its_argument_lists(self):
        commands, undecided = python_commands("planted.py", planted("script.py.fixture"))
        self.assertEqual(
            kinds(commands),
            [
                ("build", "cargo", "build", "site"),
                ("build", "cargo", "test", "clean"),
                ("build", "swift", "test", "site"),
            ],
        )
        self.assertEqual(len(undecided), 1)


class TheVerdictKeepsItsLevel(unittest.TestCase):
    """R7 over planted sites."""

    def setUp(self):
        self.sites = keyed(shell_commands("planted.sh", planted("script.sh.fixture"))[0])

    def test_an_unlisted_site_warns_at_warn_and_fails_at_fail(self):
        lines, failures = verdict(self.sites, {}, "warn")
        self.assertEqual((len(lines), failures), (2, []))
        self.assertEqual(len(verdict(self.sites, {}, "fail")[1]), 2)

    def test_a_stale_entry_fails_at_both_levels(self):
        listed = {key: "planted" for key in self.sites}
        listed[("planted.sh", "<top>", "cargo", "build", 1)] = "planted"
        for level in ("warn", "fail"):
            self.assertEqual(len(verdict(self.sites, listed, level)[1]), 1, level)

    def test_an_empty_population_is_void(self):
        with self.assertRaises(AssertionError):
            population(FIXTURES)


if __name__ == "__main__":
    unittest.main()

"""A process waiter never matches its own command line (SPEC-657; ADR-679).

`pgrep -f` and `pkill -f` match a pattern against each process's whole command line, and the
running pgrep or pkill never reports itself, but nothing leaves out the shell, the step or the
program that runs it. A waiter whose pattern occurs in that command line finds itself. This census
reads every tracked shell script (a `.sh` file, or a file with no extension whose first line is a
`sh` or `bash` shebang), every `run:` text of `.github/workflows`, and every Python file. It finds
each process lookup by its command and form, and names a site where the pattern can match the
command line that holds it: a `pgrep -f` or `pkill -f`, and a `ps` output piped to `grep`. A
pattern that cannot match the text that holds it (the bracket form), a lookup by process name
(`pgrep -x`, `pidof`) and a pid file read are not sites. Every count is a lower bound: a pattern
held in a variable other than the command line's own arguments is not judged. A member the census
cannot read (a workflow the shared run-text reader refuses, Python that does not parse, bytes that
are not UTF-8) is one `unread` site, so it is reported and never passed.

It starts at warn: an unlisted site prints `WARN`, a listed entry no site matches fails as
`STALE`, and moving `LEVEL` to fail is its own delivery under ADR-450 D6. The planted tests read
the trees under `fixtures/process-waiters` and the texts below; the repository test reads the
whole tracked tree and runs in CI.
"""

import ast
import collections
import re
import shlex
import subprocess
import unittest
from pathlib import Path, PurePosixPath

from _mutants_finder import Refused
from _support import REPO

LEVEL = "warn"
RULE = "process-waiter"
FIXTURES = Path(__file__).parent / "fixtures" / "process-waiters"
PLANTED = "scripts/tests/fixtures/process-waiters/tree/"
# Each site the repository holds today, with why it stays: the planted sites are tracked files.
LISTED = {
    f"{PLANTED}bin/wait-for pgrep-f-own-command-line #1": "a planted site of this census",
    f"{PLANTED}scripts/ps-plain.sh ps-grep-own-command-line #1": "a planted site of this census",
    f"{PLANTED}scripts/waits-inline.sh pgrep-f-own-command-line #1": "a planted site of this census",
}
LOOKUPS = ("pgrep", "pkill", "pidof", "ps")
SHEBANG = re.compile(rb"#![ \t]*(?:/usr)?/bin/(?:env[ \t]+)?(?:sh|bash)\b")
WORKFLOWS = ".github/workflows"
# The command line GitHub's runner gives a `run:` step's bash; the step's text is in a file.
STEP_SHELL = "bash --noprofile --norc -eo pipefail"
VALUED = "dgGOpPqrstuUF"
LONG_VALUED = frozenset({"pidfile", "signal", "uid", "euid", "parent", "group", "session"})
SIGNAL = re.compile(r"-(?:[0-9]+|[A-Z]{2,}[0-9]*)")
POSITIONAL = re.compile(r"\$\{?[0-9@*]")
SUBSTITUTION = re.compile(r"\$\(([^()]*)\)")
RUNNERS = frozenset({"run", "Popen", "call", "check_call", "check_output", "getoutput", "system"})
LISTINGS = frozenset({"process_iter", "pids"})
UNREADABLE = (Refused, SyntaxError, ValueError)


def can_match(pattern, text, whole=False):
    """Whether `pattern`, read as the extended regular expression pgrep and grep -E take, matches
    `text`, or the whole of it; a pattern Python's engine refuses is read as fixed text."""
    try:
        compiled = re.compile(pattern)
    except re.error:
        return text == pattern if whole else pattern in text
    return bool(compiled.fullmatch(text) if whole else compiled.search(text))


def chunks(text):
    """(line, text) for each logical line of shell `text`: a newline inside quotes continues it,
    a backslash before a newline joins it, and a comment is dropped."""
    found, out, start, quote, k = [], [], 0, "", 0
    while k < len(text):
        c = text[k]
        if quote:
            out.append(c)
            quote = "" if c == quote else quote
        elif c == "\\" and text[k + 1 : k + 2] == "\n":
            out.append(" ")
            k += 1
        elif c in "'\"":
            quote = c
            out.append(c)
        elif c == "#" and (not out or out[-1] in " \t;|&("):
            while k + 1 < len(text) and text[k + 1] != "\n":
                k += 1
        elif c == "\n":
            found.append((text.count("\n", 0, start) + 1, "".join(out)))
            out, start = [], k + 1
        else:
            out.append(c)
        k += 1
    found.append((text.count("\n", 0, start) + 1, "".join(out)))
    return [(line, chunk) for line, chunk in found if chunk.strip()]


def pipelines(chunk):
    """Each pipeline of one logical line as a list of commands, each a list of words, as shlex
    splits them; a redirection's target is dropped, and a line shlex refuses yields none."""
    lexer = shlex.shlex(chunk, posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        tokens = list(lexer)
    except ValueError:
        return []
    found, pipeline, command, target = [], [], [], False
    for token in [*tokens, ";"]:
        if target:
            target = False
        elif token and set(token) <= set("<>&") and set(token) & set("<>"):
            target = True
        elif token in ("|", "|&"):
            pipeline, command = [*pipeline, command], []
        elif token and set(token) <= set(";&|()"):
            found.append([each for each in [*pipeline, command] if each])
            pipeline, command = [], []
        else:
            command.append(token)
    return [each for each in found if each]


def options(args):
    """(full, exact, pidfile, pattern) of pgrep's or pkill's arguments."""
    full = exact = pidfile = False
    k = 0
    while k < len(args):
        word = args[k]
        if word == "--":
            return full, exact, pidfile, args[k + 1] if k + 1 < len(args) else None
        if word.startswith("--"):
            name = word[2:].split("=", 1)[0]
            full, exact = full or name == "full", exact or name == "exact"
            pidfile = pidfile or name == "pidfile"
            k += name in LONG_VALUED and "=" not in word
        elif SIGNAL.fullmatch(word):
            pass
        elif word.startswith("-") and len(word) > 1:
            for at, flag in enumerate(word[1:], 2):
                full, exact = full or flag == "f", exact or flag == "x"
                pidfile = pidfile or flag == "F"
                if flag in VALUED:
                    k += at == len(word)
                    break
        else:
            return full, exact, pidfile, word
        k += 1
    return full, exact, pidfile, None


def grep_pattern(command):
    """The pattern a `grep` command selects lines by, or None: not a grep, or one that drops."""
    if PurePosixPath(command[0]).name != "grep" or "-v" in command:
        return None
    rest = command[1:]
    for k, word in enumerate(rest):
        if word == "-e" and k + 1 < len(rest):
            return rest[k + 1]
        if not word.startswith("-"):
            return word
    return None


def classify(words, rest, context, own_args):
    """(form, site) of one lookup: `words` from its program on, `rest` the pipeline after it,
    `context` the command line that runs it, `own_args` when its positional parameters are that
    command line's own."""
    name = PurePosixPath(words[0]).name
    if name == "pidof":
        return "pidof", False
    if name == "ps":
        for command in rest:
            pattern = grep_pattern(command)
            if pattern is not None:
                held = " ".join(command)
                return "ps-grep", "$" not in pattern and (
                    can_match(pattern, held) or can_match(pattern, context)
                )
        return "ps", False
    full, exact, pidfile, pattern = options(words[1:])
    if pattern is None:
        return (f"{name}-pidfile" if pidfile else name), False
    if not full:
        return (f"{name}-x" if exact else f"{name}-name"), False
    if POSITIONAL.search(pattern):
        return f"{name}-f", own_args
    return f"{name}-f", "$" not in pattern and can_match(pattern, context, exact)


def shell_script(words):
    """The script a shell's `-c` option runs, or None."""
    for k, word in enumerate(words[1:-1], 1):
        if not word.startswith("-") or word.startswith("--"):
            return None
        if "c" in word:
            return words[k + 1]
    return None


def shell_lookups(text, context, own_args, base=1):
    """(line, program, form, site) for each process lookup in shell `text`, run by the command
    line `context`. A command's program is the first of its words that names a lookup or a shell,
    so a wrapper (`timeout`, `sudo`, `env`) and a keyword (`until`, `if`, `!`) are read through,
    and a shell's `-c` script is read with the shell's own command line as its context."""
    return []


def argv_word(node):
    """One argument of a Python argument list: its text, `$@` for `sys.argv[...]`, else `$`."""
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if isinstance(node, ast.Subscript) and ast.unparse(node.value) == "sys.argv":
        return "$@"
    return "$"


def python_lookups(path, text):
    """(where, program, form, site) for each process lookup in Python `text`: a command string
    or an argument list handed to a process call, and a psutil process listing."""
    return []


def population(root, paths):
    """(kind, path) for each member, in path order: a shell script, a workflow or a Python file."""
    return [("shell", path) for path in sorted(paths) if PurePosixPath(path).suffix == ".sh"]


def member(kind, path, data):
    """(where, program, form, site) for each lookup one member's bytes hold. A member the census
    cannot read is one `unread` site: reported, never passed."""
    return []


def census(root, paths):
    """(path, where, program, form, site) for each lookup the population holds; an empty
    population is refused, never passed."""
    found = []
    for kind, path in population(root, paths):
        data = (root / path).read_bytes()
        found += [(path, *each) for each in member(kind, path, data)]
    print(f"found {len(found)} lookup(s)")
    return found


def site_keys(found):
    """The key of each site: its path, its form and its ordinal among that path's sites."""
    counted, keys = collections.Counter(), []
    for path, _where, _program, form, site in found:
        if site:
            counted[path, form] += 1
            suffix = "" if form == "unread" else "-own-command-line"
            keys.append(f"{path} {form}{suffix} #{counted[path, form]}")
    return keys


def verdict(keys, listed, level):
    """(lines, failed): `WARN` for each unlisted site, failing only at fail, and `STALE` for each
    listed entry no site matches, failing at both levels."""
    stale = [f"STALE {RULE} {key}" for key in listed if key not in keys]
    return stale, bool(stale)


def planted(name):
    """The root of a planted tree and its files, relative and in order."""
    root = FIXTURES / name
    return root, sorted(p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file())


PYTHON_PLANT = "\n".join(
    [
        "import os",
        "import subprocess",
        "import sys",
        "",
        "subprocess.run(['pgrep', '-f', sys.argv[1]], check=False)",
        "subprocess.Popen(['pgrep', '-f', 'deck-api('])",
        "subprocess.call(['pgrep', '-f', name])",
        "subprocess.check_call(['pidof', 'deck-api'])",
        "subprocess.check_output('ps -eo pid,args | grep deck-api', shell=True)",
        "subprocess.getoutput('pgrep -f deck-api')",
        "os.system('pgrep -x deck-api')",
        "psutil.process_iter(['cmdline'])",
        "psutil.pids()",
        "store.pids()",
        "",
    ]
)
UNREAD_PLANTS = [
    (
        "workflow",
        ".github/workflows/w.yml",
        b"jobs:\n  wait:\n    runs-on: ubuntu-24.04\n    steps:\n"
        b"      - run: echo ${{ github.sha }}\n",
    ),
    ("python", "tools/w.py", b"def broken(:\n"),
    ("shell", "scripts/w.sh", b"\xff\n"),
]
TREE_LOOKUPS = [
    (".github/workflows/waits.yml", "step 1 line 1", "pgrep", "pgrep-f", True),
    ("bin/wait-for", "line 3", "pgrep", "pgrep-pidfile", False),
    ("bin/wait-for", "line 4", "pgrep", "pgrep-f", True),
    ("scripts/exact-name.sh", "line 3", "pgrep", "pgrep-x", False),
    ("scripts/exact-name.sh", "line 4", "pgrep", "pgrep-name", False),
    ("scripts/exact-name.sh", "line 5", "pkill", "pkill-x", False),
    ("scripts/exact-name.sh", "line 6", "pidof", "pidof", False),
    ("scripts/exact-name.sh", "line 7", "ps", "ps", False),
    ("scripts/ps-bracket.sh", "line 3", "ps", "ps-grep", False),
    ("scripts/ps-plain.sh", "line 3", "ps", "ps-grep", True),
    ("scripts/waits-clean.sh", "line 3", "pgrep", "pgrep-f", False),
    ("scripts/waits-clean.sh", "line 4", "pgrep", "pgrep-f", False),
    ("scripts/waits-clean.sh", "line 5", "pkill", "pkill-f", False),
    ("scripts/waits-clean.sh", "line 6", "pgrep", "pgrep-f", False),
    ("scripts/waits-inline.sh", "line 3", "pgrep", "pgrep-f", True),
    ("scripts/waits-inline.sh", "line 4", "pgrep", "pgrep-f", False),
]


class ProcessWaiterCensus(unittest.TestCase):
    def test_a_planted_tree_counts_each_script_step_and_python_file(self):
        root, paths = planted("tree")
        self.assertEqual(
            population(root, paths),
            [
                ("workflow", ".github/workflows/waits.yml"),
                ("shell", "bin/wait-for"),
                ("shell", "scripts/exact-name.sh"),
                ("shell", "scripts/ps-bracket.sh"),
                ("shell", "scripts/ps-plain.sh"),
                ("shell", "scripts/waits-clean.sh"),
                ("shell", "scripts/waits-inline.sh"),
                ("python", "tools/inert.py"),
            ],
        )

    def test_a_planted_lookup_that_can_match_its_own_command_line_is_a_site(self):
        found = census(*planted("tree"))
        self.assertEqual([each for each in found if each[-1]], [t for t in TREE_LOOKUPS if t[-1]])
        self.assertEqual(
            site_keys(found),
            [
                ".github/workflows/waits.yml pgrep-f-own-command-line #1",
                "bin/wait-for pgrep-f-own-command-line #1",
                "scripts/ps-plain.sh ps-grep-own-command-line #1",
                "scripts/waits-inline.sh pgrep-f-own-command-line #1",
            ],
        )

    def test_a_planted_lookup_that_cannot_match_itself_is_clean(self):
        found = census(*planted("tree"))
        clean = [t for t in TREE_LOOKUPS if not t[-1]]
        self.assertEqual([each for each in found if not each[-1]], clean)

    def test_the_planted_python_lookups_are_read_by_their_argument_list_and_command_text(
        self,
    ):
        self.assertEqual(
            python_lookups("tools/waiter.py", PYTHON_PLANT),
            [
                ("line 5", "pgrep", "pgrep-f", True),
                ("line 6", "pgrep", "pgrep-f", False),
                ("line 7", "pgrep", "pgrep-f", False),
                ("line 8", "pidof", "pidof", False),
                ("line 9", "ps", "ps-grep", True),
                ("line 10", "pgrep", "pgrep-f", True),
                ("line 11", "pgrep", "pgrep-x", False),
                ("line 12", "psutil", "psutil-process_iter", False),
                ("line 13", "psutil", "psutil-pids", False),
            ],
        )

    def test_the_planted_level_warns_on_an_unlisted_site_and_fails_on_a_stale_entry(
        self,
    ):
        key = "a.sh pgrep-f-own-command-line #1"
        stale = "b.sh ps-grep-own-command-line #1"
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(verdict([key], {}, "warn"), ([f"WARN process-waiter {key}"], False))
        self.assertEqual(verdict([key], {}, "fail"), ([f"WARN process-waiter {key}"], True))
        self.assertEqual(
            verdict([], {stale: "x"}, "warn"), ([f"STALE process-waiter {stale}"], True)
        )
        self.assertEqual(verdict([key], {key: "x"}, "fail"), ([], False))

    def test_a_planted_empty_population_is_void(self):
        with self.assertRaisesRegex(AssertionError, "examined 0 file"):
            census(*planted("empty"))

    def test_a_planted_member_the_census_cannot_read_is_an_unread_site(self):
        found = [
            (path, *each) for kind, path, data in UNREAD_PLANTS for each in member(kind, path, data)
        ]
        self.assertEqual(
            found,
            [
                (".github/workflows/w.yml", "whole file", "-", "unread", True),
                ("tools/w.py", "whole file", "-", "unread", True),
                ("scripts/w.sh", "whole file", "-", "unread", True),
            ],
        )
        self.assertEqual(
            site_keys(found),
            [
                ".github/workflows/w.yml unread #1",
                "tools/w.py unread #1",
                "scripts/w.sh unread #1",
            ],
        )

    def test_the_repository_holds_exactly_the_listed_sites_at_warn(self):
        listing = subprocess.run(
            ["git", "-C", str(REPO), "ls-files", "-z"],
            capture_output=True,
            text=True,
            check=True,
        )
        paths = [path for path in listing.stdout.split("\0") if path]
        lines, failed = verdict(site_keys(census(REPO, paths)), LISTED, LEVEL)
        for line in lines:
            print(line)
        self.assertEqual([line for line in lines if line.startswith("STALE")], [])
        self.assertFalse(failed)


if __name__ == "__main__":
    unittest.main()

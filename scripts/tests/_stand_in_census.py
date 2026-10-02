"""A census of the test doubles that fall back to a real program when they cannot plant their seam.

It reads text and nothing else: no test module is imported or run, which is the point, because the
modules it judges (the dispatch-shard guard's tests and the wrapper's own) never run on the box.

An arm is an `except` handler whose `try` block plants something and whose handler does not leave:
it holds no `raise`, `return`, `exit` or `continue`, and a call that executes a real program
(`os.exec*`, `subprocess.*`, `runpy.*`) follows it within a few lines at the same indent or an outer
one. Such a handler turns a failure to plant into a run of the real program, which reads as a
passing run (#497). Run as a script it prints one line per arm and the count examined.
"""

import re
import sys
from pathlib import Path

HERE = Path(__file__).parent
EXCEPT = re.compile(r"^(\s*)except\b[^\n]*:\s*$")
TRY = re.compile(r"^(\s*)try:\s*$")
REAL_PROGRAM = re.compile(r"\b(os\.exec\w*|subprocess\.\w+|runpy\.\w+)\(")
LEAVES = re.compile(r"\b(raise|return|continue|break)\b|\bexit\(")
REACH = 12


def indent(line):
    """The width of the leading blanks of `line`."""
    return len(line) - len(line.lstrip())


def arms_of(text):
    """[(line, arm)] for every failed-plant fallback in `text`, line numbers from 1."""
    lines = text.splitlines()
    arms = []
    for n, line in enumerate(lines):
        handler = EXCEPT.match(line)
        if not handler:
            continue
        depth = len(handler.group(1))
        body = []
        for later in lines[n + 1 :]:
            if later.strip() and indent(later) <= depth:
                break
            body.append(later)
        planted = False
        for earlier in reversed(lines[:n]):
            if TRY.match(earlier) and indent(earlier) == depth:
                break
            planted = planted or "plant" in earlier
        else:
            continue
        if not planted or any(LEAVES.search(b) for b in body):
            continue
        after = lines[n + 1 + len(body) : n + 1 + len(body) + REACH]
        if any(REAL_PROGRAM.search(a) and indent(a) <= depth for a in after if a.strip()):
            arms.append((n + 1, line.strip()))
    return arms


def census(directory=HERE):
    """(files examined, [(file name, line, arm)]) over every `*.py` file of `directory`."""
    files = sorted(Path(directory).glob("*.py"))
    found = [
        (path.name, line, arm)
        for path in files
        for line, arm in arms_of(path.read_text(encoding="utf-8"))
    ]
    return len(files), found


def reading(directory=HERE):
    """(files examined, str constants parsed, str constants skipped, arms) over `directory`."""
    files, found = census(directory)
    return files, 0, 0, found


def main(directory=HERE):
    """Print each arm as `file:line: arm`, then `examined N files, M arms`."""
    examined, found = census(directory)
    for name, line, arm in found:
        print(f"{name}:{line}: {arm}")
    print(f"examined {examined} files, {len(found)} arms")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())

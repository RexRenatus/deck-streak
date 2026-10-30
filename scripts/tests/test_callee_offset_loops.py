"""A loop that takes its next position from a callee ends a stalled step by name (SPEC-094 R9).

A loop that reads `let (.., next) = reader(..)` and then moves its position with `at = next`
trusts the reader to advance. A reader that stays or steps back makes it spin, or push without
bound, which is memory rather than time. ADR-095's amendment guards such a loop with a strict
advance, `if next <= at`, whose body returns a named `Err`. A guard that `break`s instead ends the
loop on what it has kept, and a caller then judges a partial result as a whole one: a template
token scan that stopped early reports fields the template does render as dark.

The guard's own tests inject readers that stay or step back. Under a mutant of the compare
(`<=` to `<`) such a reader is accepted, so a reader that does not count its own calls runs the
loop until memory ends it (measured on `wire_progress`: the test process was killed). So every
reader a test injects bounds its calls with an assertion, and the loop ends by a named failure there
too.

The guard enumerates the loops by walking `crates/*/src`: a `while` or `loop` whose body binds the
last element of a tuple from a call and later assigns that name to the loop's position, where the
position is an integer offset (`let mut at = 0` or a `mut at: usize` parameter in the enclosing
function). Each must hold the strict-advance compare, and the compare's first statement must be
`return Err(`. A loop that walks a slice with `split_once` is not one: its tail is shorter than
what it split by the delimiter's length, so it cannot stay. The functions that hold such a loop,
and those that pass their own injected `step` on to one, are the ones whose calls in
`crates/*/tests` must hand in a bounded reader.
"""

import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

HEAD = re.compile(r"^(\s*)(?:while\b.*|loop)\s*\{\s*$")
BIND = re.compile(r"^\s*let\s*\((?:[^()]*,\s*)?(\w+)\)\s*=\s*[\w:.]+\(")
INTEGER = r"[ui](?:size|8|16|32|64)"
OFFSET = re.compile(rf"\blet\s+mut\s+(\w+)(?:\s*:\s*{INTEGER})?\s*=\s*\d")
PARAMETER = re.compile(rf"\bmut\s+(\w+)\s*:\s*{INTEGER}\b")
FUNCTION = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?fn\s+(\w+)")
BOUND = re.compile(r"\bassert!\(\s*\w+\s*<")


def loops(root):
    """Every loop under `crates/*/src` whose position comes from a callee.

    Each is `(file, line, verdict, function)`: `refuses` when the strict-advance compare returns an
    `Err`, `silent` when it does anything else, and `unguarded` when there is no compare.
    """
    found = []
    for path in sorted((root / "crates").glob("*/src/**/*.rs")):
        lines = path.read_text(encoding="utf-8").splitlines()
        for start, line in enumerate(lines):
            head = HEAD.match(line)
            if not head:
                continue
            close = head.group(1) + "}"
            end = next((k for k in range(start + 1, len(lines)) if lines[k] == close), len(lines))
            function, positions = enclosing(lines, start)
            verdict = judge(lines[start + 1 : end], positions)
            if verdict:
                found.append((path.relative_to(root).as_posix(), start + 1, verdict, function))
    return found


def enclosing(lines, start):
    """The name of the function enclosing line `start`, and the integer offsets it declares."""
    names = set()
    for line in reversed(lines[:start]):
        names.update(OFFSET.findall(line))
        names.update(PARAMETER.findall(line))
        function = FUNCTION.match(line)
        if function:
            return function.group(1), names
    return None, names


def judge(body, positions):
    """The loop's verdict, or None when no callee hands it its next integer position."""
    for index, line in enumerate(body):
        bound = BIND.match(line)
        if not bound:
            continue
        name = bound.group(1)
        advance = re.compile(rf"^\s*(\w+)\s*=\s*{name}\s*;")
        for later in range(index + 1, len(body)):
            moved = advance.match(body[later])
            if not moved or moved.group(1) not in positions:
                continue
            compare = re.compile(rf"^\s*if\s+{name}\s*<=\s*{moved.group(1)}\s*\{{\s*$")
            for at in range(index + 1, later):
                if compare.match(body[at]):
                    then = body[at + 1] if at + 1 < later else ""
                    return "refuses" if re.match(r"^\s*return\s+Err\(", then) else "silent"
            return "unguarded"
    return None


def functions(root, found):
    """The functions that hold a loop in `found`, and those that pass their `step` on to one."""
    names = {function for _, _, _, function in found if function}
    sources = [path.read_text(encoding="utf-8") for path in (root / "crates").glob("*/src/**/*.rs")]
    grown = True
    while grown:
        grown = False
        for text in sources:
            starts = [(m.start(), m.group(1)) for m in re.finditer(FUNCTION.pattern, text, re.M)]
            for index, (start, name) in enumerate(starts):
                end = starts[index + 1][0] if index + 1 < len(starts) else len(text)
                body = text[start:end]
                calls = rf"\b(?:{'|'.join(sorted(names))})\([^;]*\bstep\b"
                passes = re.search(r"\bstep\s*:\s*F\b", body) and re.search(calls, body)
                if name not in names and passes:
                    names.add(name)
                    grown = True
    return names


def arguments(text, open_at):
    """The text between the parenthesis at `open_at` and the one that closes it."""
    depth = 0
    for index in range(open_at, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if depth == 0:
                return text[open_at + 1 : index]
    return text[open_at + 1 :]


def readers(root, names):
    """Each closure a test under `crates/*/tests` hands to one of `names`: (file, line, bounded)."""
    found = []
    call = re.compile(rf"\b(?:{'|'.join(sorted(names))})\(")
    for path in sorted((root / "crates").glob("*/tests/**/*.rs")):
        text = path.read_text(encoding="utf-8")
        for match in call.finditer(text):
            given = arguments(text, match.end() - 1)
            if "|" not in given:
                continue
            line = text.count("\n", 0, match.start()) + 1
            found.append((path.relative_to(root).as_posix(), line, bool(BOUND.search(given))))
    return found


FIXTURE = """
fn refuses(data: &[u8], mut step: F) -> Result<usize, &'static str> {
    let mut at = 0;
    while at < data.len() {
        let (_, next) = step(data, at);
        if next <= at {
            return Err("no progress");
        }
        at = next;
    }
    Ok(at)
}

fn silent(data: &[u8]) -> usize {
    let mut at = 0;
    while at < data.len() {
        let (_, next) = step(data, at);
        if next <= at {
            break;
        }
        at = next;
    }
    at
}

fn unguarded(data: &[u8]) -> usize {
    let mut at = 0;
    loop {
        let (_, next) = step(data, at);
        at = next;
        if at >= data.len() {
            return at;
        }
    }
}

fn counted(data: &[u8]) -> usize {
    let mut at = 0;
    while at < data.len() {
        let (_, width) = step(data, at);
        at += 1 + width;
    }
    at
}

fn sliced(text: &str) -> usize {
    let mut rest = text;
    while let Some((_, after)) = rest.split_once("{{") {
        let (_, tail) = split(after);
        rest = tail;
    }
    rest.len()
}

pub fn passes_on(data: &[u8], mut step: F) -> usize {
    refuses(data, &mut step).unwrap_or(0)
}
"""

FIXTURE_TEST = """
fn stays() {
    let mut calls = 0;
    let got = refuses(&[1, 2], |_, at| {
        calls += 1;
        assert!(calls < 3, "went on");
        ((), at)
    });
}

fn steps_back() {
    let got = passes_on(&[1, 2], |_, at| ((), at.saturating_sub(1)));
}

fn real() {
    let got = refuses(&[1, 2], step);
}
"""


class CalleeOffsetLoops(unittest.TestCase):
    def test_every_callee_offset_loop_refuses_a_stalled_step_by_name(self):
        found = examined("loops that take their next position from a callee", loops(REPO))
        for file, line, verdict, _ in found:
            with self.subTest(loop=f"{file}:{line}"):
                self.assertEqual(
                    verdict,
                    "refuses",
                    f"{file}:{line}: a step that does not advance must return a named Err",
                )

    def test_every_reader_a_test_injects_bounds_its_calls(self):
        names = functions(REPO, loops(REPO))
        found = examined(f"readers handed to {sorted(names)}", readers(REPO, names))
        for file, line, bounded in found:
            with self.subTest(reader=f"{file}:{line}"):
                self.assertTrue(
                    bounded,
                    f"{file}:{line}: the reader must count its calls and assert a bound, so a "
                    "mutant that accepts a stalled step ends by a named failure, not by memory",
                )

    def test_the_guard_tells_a_named_refusal_from_a_silent_or_missing_one(self):
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            source = root / "crates" / "fixture" / "src" / "lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text(FIXTURE, encoding="utf-8")
            test = root / "crates" / "fixture" / "tests" / "progress.rs"
            test.parent.mkdir(parents=True)
            test.write_text(FIXTURE_TEST, encoding="utf-8")
            found = examined("fixture loops", loops(root))
            names = functions(root, found)
            injected = examined("fixture readers", readers(root, names))
        self.assertEqual(
            [verdict for _, _, verdict, _ in found], ["refuses", "silent", "unguarded"]
        )
        self.assertEqual(
            [line for _, line, _, _ in found],
            [4, 16, 28],
            "the counted and sliced loops are not ones",
        )
        self.assertEqual(names, {"refuses", "silent", "unguarded", "passes_on"})
        self.assertEqual(
            [(line, bounded) for _, line, bounded in injected],
            [(4, True), (12, False)],
            "a function item such as `step` is no injected reader",
        )


if __name__ == "__main__":
    unittest.main()

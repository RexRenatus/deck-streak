#!/usr/bin/env python3
"""mutation-verdict: a pull request's mutation plan and verdict, the weekly battery's survivors as
issue drafts and its count of reports, and the tools' configurations and exclusions (SPEC-039 R2 to
R6, R10, R12; ADR-057).

    python3 scripts/mutation-verdict.py plan --base REF [--head REF] [--root DIR] --out DIR
                                            [--event E --base-ref B --subject S]
    python3 scripts/mutation-verdict.py shards --plan FILE [--listed FILE]
    python3 scripts/mutation-verdict.py judge --plan FILE --class rust|web|oracle
                                             [--outcomes FILE] [--tool-exit N]
                                             [--shard-reports DIR]
                                             [--stryker FILE] [--rows FILE]
    python3 scripts/mutation-verdict.py survivors --reports DIR --out DIR [--open-titles FILE]
    python3 scripts/mutation-verdict.py battery --reports DIR --shards N
    python3 scripts/mutation-verdict.py configs [--root DIR]
    python3 scripts/mutation-verdict.py exclusions [--root DIR]

PLAN first decides the run's scope from the event that started it (R3), because a job is never
skipped: `ci` reads a skipped need as failed. A pull request into `dev` is judged on its diff, and
a release pull request into `main` on its merge diff, every change `dev` carries since the last
release; a push that merges a pull request (`Merge pull request #N`) is not-applicable, naming
`#N`, whose jobs judged that same tree; a push that names none is judged on its first-parent
diff. For a diff it reads `git diff BASE...HEAD` (on a pull request's merge
ref, BASE is `HEAD^1`) and writes `plan.json` and `git.diff` into `--out`: every changed path with
its class (R2), each production file's changed lines split into code lines, blank or comment lines
and, in Rust, test-only lines, those inside an item cargo-mutants never mutates for a test attribute
(SPEC-057 R22), the rows the diff selects (R10), and the web files Stryker mutates whole. It prints
each class's case by name: why it applies, or why it is not-applicable. Under GitHub Actions it
writes the step outputs `scope`, `case`, `rust`, `web`, `oracle`, `rows` and `mutate`.

SHARDS sizes the Rust run from cargo-mutants' own listing of the diff's mutants (`--list --json
--in-diff`), so no shard reaches its job's timeout (R18). Each round-robin shard's time is projected
as the unmutated baseline's plus its mutants' measured costs, mutant `i` in shard `i mod n` as the
tool assigns them, and the fewest shards whose slowest is projected within the bound are written
into the plan, with each shard's mutants, and as the step outputs `shards` and `matrix`. A diff that
needs more shards than a job matrix holds is refused with its projection, never capped. A listing
file that holds nothing is the tool's own empty listing, since it prints nothing when no mutant
overlaps the diff; a missing listing is VOID (SPEC-057 R22).

JUDGE reads a tool's own report, never its exit alone (R4); under `--shard-reports`, every shard's
the plan promised, from `0` to `n-1`, each missing or partial one VOID by name, and the reports
together must hold every listed mutant once (R18). Examined is caught plus missed plus
timed out (Stryker: killed, survived, no coverage and timed out); an unviable mutant, a compile or
runtime error, is not examined. A missed or uncovered mutant, or a selected row that was not
KILLED, fails (exit 1). A class whose production files changed a code line and whose examined
count, the tool's and the rows' on its changed lines, is zero is VOID (exit 3). So is a missing
report, and a partial one: cargo-mutants writes its report as it goes, so a report is read only when
the tool's exit is 0, 2 or 3 and its caught, missed, timed-out and unviable counts sum to its total.
A class whose changed lines are all blank or comments, or only deletions, or in Rust test-only,
reads `not-applicable` by name, with its count.

SURVIVORS turns the weekly battery's reports into one issue draft per file, titled
`Mutation survivors: <path>`, and marks a draft whose title is already an open issue (R12). The
workflow scrubs the drafts with `scripts/public-scrub.py` before it files any.

BATTERY counts the reports the battery's jobs promise: each of N shards' `outcomes.json`, whole by
the rule JUDGE applies, the rows' report and the Stryker sweep. It names each report that is missing
or partial and fails (exit 1), so a runner shut down mid-run never reads as a shard with no
survivor.

CONFIGS judges `.cargo/mutants.toml` and `web/app/stryker.config.json` by what each tool itself
refuses (cargo-mutants 27.1.0 denies an unknown key or a mistyped value; StrykerJS refuses a
threshold out of 0 to 100, `high` below `low`, JSON it cannot parse, and `ignoreStatic` without
per-test coverage), by what the verdict needs of Stryker's (the Vitest runner, the json report and
R2's production code as its `mutate`), and refuses a second Stryker configuration it would read.

EXCLUSIONS holds every exclusion to its reason and issue (R5): an `exclude_re`, `exclude_globs` or
`skip_calls` entry in `.cargo/mutants.toml` needs `# EQUIVALENT: <reason> (#N)` on its line or the
line above, a `Stryker disable` comment needs `EQUIVALENT: <reason> (#N)` on its own line, and a
`mutants::skip` attribute is refused outright (ADR-057 D6). A tree may hold none.
"""

from __future__ import annotations

import argparse
import bisect
import io
import json
import os
import pathlib
import re
import subprocess
import sys
import tokenize
import tomllib
from collections import Counter, defaultdict
from dataclasses import dataclass, field

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import mutation_rows  # noqa: E402

EXIT_OK, EXIT_FAIL, EXIT_USAGE, EXIT_VOID = 0, 1, 2, 3
RUST = re.compile(r"crates/[^/]+/src/.+\.rs")
WEB = re.compile(r"web/app/src/.+\.(?:ts|js|svelte)")
WEB_NOT_PRODUCTION = re.compile(r"\.(?:test|spec)\.[^/]+$|\.d\.ts$|^web/app/src/lib/paraglide/")
ORACLE = frozenset({"tools/parity-oracle/generate.py"})
WEB_ROOT = "web/app/"
HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
#: cargo-mutants' exits after a run that tested every mutant it listed: all caught (0), some missed
#: (2), some timed out (3). Any other exit, its own (1, 4, 5, 6, 70) or a signal's (137 is SIGKILL),
#: means the run stopped early, and the report it writes as it goes is partial (its book's "Exit
#: codes").
WHOLE_EXITS = frozenset({0, 2, 3})
TOOL_EXITS = {
    1: "a usage error",
    4: "a red baseline: the tests failed before any mutant",
    5: "a diff that does not match the tree",
    6: "a diff it could not read",
    70: "an internal error",
}
#: The subject GitHub writes for a pull request's merge commit.
MERGE_SUBJECT = re.compile(r"Merge pull request #(\d+) from ")
#: The first line GitHub writes for a squash merge: the pull request's title, then ` (#N)`. A title
#: that already ends with an issue's `(#M)` gains one more, so the last number names the pull
#: request (SPEC-057 R18).
SQUASH_SUBJECT = re.compile(r" \(#(\d+)\)$")


def scope_of(event: str, base_ref: str, subject: str) -> tuple[str, str]:
    """(`diff` or `not-applicable`, why) for the event that started the run (SPEC-039 R3). A job is
    never skipped, because `ci` reads a skipped need as failed, so each case says why by name."""
    if event == "pull_request" and base_ref == "main":
        return (
            "diff",
            "a release pull request into main is judged on its merge diff, every change dev "
            "carries since the last release, in as many shards as fit their bound",
        )
    if event == "pull_request":
        return "diff", f"the pull request into {base_ref or 'its base'} is judged on its diff"
    if event == "push":
        merged = MERGE_SUBJECT.match(subject)
        squashed = SQUASH_SUBJECT.search(subject.splitlines()[0].rstrip() if subject else "")
        if merged or squashed:
            number, how = (merged.group(1), "") if merged else (squashed.group(1), " by squash")
            return (
                "not-applicable",
                f"this push merges #{number}{how}, whose mutation jobs judged this tree on its "
                "merge ref; dev and main accept a pull request only with an up-to-date head "
                "(ADR-034)",
            )
        return "diff", "this push names no pull request, so its first-parent diff is judged"
    return "diff", f"the {event or 'local'} run is judged on its diff"


TITLE = "Mutation survivors: {}"


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *args], capture_output=True, text=True, check=True
    ).stdout


def classify(path: str) -> str:
    """A path's class under R2: `rust`, `web`, `oracle`, or `other`."""
    if RUST.fullmatch(path):
        return "rust"
    if WEB.fullmatch(path) and not WEB_NOT_PRODUCTION.search(path):
        return "web"
    if path in ORACLE:
        return "oracle"
    return "other"


#: Python's tokens that carry no code: a line holding only these is blank or a comment.
QUIET_TOKENS = frozenset(
    {
        tokenize.COMMENT,
        tokenize.NL,
        tokenize.NEWLINE,
        tokenize.INDENT,
        tokenize.DEDENT,
        tokenize.ENDMARKER,
    }
)
#: The characters after which a `/` begins a regular expression rather than a division.
REGEX_AFTER = frozenset("(,=:[!&|?{};+-*%<>~^")
REGEX_WORDS = frozenset(
    {
        "return",
        "typeof",
        "instanceof",
        "case",
        "do",
        "else",
        "in",
        "of",
        "new",
        "delete",
        "void",
        "throw",
        "yield",
        "await",
    }
)


def comment_or_blank(text: str, language: str) -> set[int]:
    """The 1-based numbers of the lines of `text` that are blank or hold nothing but comments.

    A line holding any code, or any part of a string or character literal, is code, so a comment
    opener inside a literal opens nothing. Python is read by its own tokenizer, and a text it cannot
    read is all code. Rust is read with its strings, raw strings (`r#"..."#`), byte and C strings,
    character literals (a lifetime's quote is code) and nested block comments; TypeScript,
    JavaScript and Svelte with their quoted strings, template literals and the code in their `${}`,
    regular-expression literals where one may begin, and, in Svelte, `<!-- -->`. The lexer errs
    toward calling a line code: a line it misreads as code makes a class apply, never pass.
    """
    lines = set(range(1, len(text.splitlines()) + 1))
    if language == "python":
        code = python_code_lines(text)
        return set() if code is None else lines - code
    return lines - c_code_lines(text, language)


def python_code_lines(text: str) -> set[int] | None:
    """The lines Python's tokenizer finds a token of code on, or None when it cannot read `text`."""
    code: set[int] = set()
    try:
        for token in tokenize.generate_tokens(io.StringIO(text).readline):
            if token.type not in QUIET_TOKENS:
                code.update(range(token.start[0], token.end[0] + 1))
    except (tokenize.TokenError, SyntaxError):
        return None
    return code


def quoted_end(text: str, start: int, quote: str, multiline: bool) -> int:
    """The end of the quoted literal that opens at `start`, past its escapes. A literal that may
    not span lines ends at its line's end when it is not closed."""
    at = start + 1
    while at < len(text):
        char = text[at]
        if char == "\\":
            at += 2
            continue
        if char == quote:
            return at + 1
        if char == "\n" and not multiline:
            return at
        at += 1
    return len(text)


def block_end(text: str, start: int, nested: bool) -> int:
    """The end of the block comment that opens at `start`; Rust's nest, JavaScript's do not."""
    depth, at = 0, start
    while at < len(text):
        if text.startswith("/*", at):
            depth = depth + 1 if nested else 1
            at += 2
        elif text.startswith("*/", at):
            depth, at = depth - 1, at + 2
            if depth == 0:
                return at
        else:
            at += 1
    return len(text)


def rust_literal(text: str, start: int) -> int | None:
    """The end of the Rust string, raw string, byte or C string, or character literal that opens at
    `start`, or None: an identifier, or a lifetime's or a label's quote, opens none."""
    at = start
    if text[at] in "bc" and text[at + 1 : at + 2] in ("r", '"', "'"):
        at += 1
    if text[at] == "r":
        hashes = at + 1
        while text[hashes : hashes + 1] == "#":
            hashes += 1
        if text[hashes : hashes + 1] != '"':
            return None
        closing = '"' + "#" * (hashes - at - 1)
        end = text.find(closing, hashes + 1)
        return len(text) if end < 0 else end + len(closing)
    if text[at] == '"':
        return quoted_end(text, at, '"', multiline=True)
    if text[at] == "'":
        if text[at + 1 : at + 2] == "\\":
            end = text.find("'", at + 3)
            return len(text) if end < 0 else end + 1
        if text[at + 2 : at + 3] == "'" and text[at + 1 : at + 2] not in ("", "\n"):
            return at + 3
    return None


def web_literal(text: str, start: int, regex_may_begin: bool) -> int | None:
    """The end of the quoted string, or of the regular expression, that opens at `start`, or None."""
    char = text[start]
    if char in "\"'":
        return quoted_end(text, start, char, multiline=False)
    if char != "/" or not regex_may_begin:
        return None
    at, in_class = start + 1, False
    while at < len(text) and text[at] != "\n":
        if text[at] == "\\":
            at += 2
            continue
        if text[at] == "[":
            in_class = True
        elif text[at] == "]":
            in_class = False
        elif text[at] == "/" and not in_class:
            at += 1
            while at < len(text) and text[at].isalpha():
                at += 1
            return at
        at += 1
    return None


def template_part(text: str, start: int) -> tuple[int, bool]:
    """(the end of a template literal's part from `start`, whether that part ends by opening a
    `${` rather than by closing the template)."""
    at = start
    while at < len(text):
        if text[at] == "\\":
            at += 2
        elif text[at] == "`":
            return at + 1, False
        elif text.startswith("${", at):
            return at + 2, True
        else:
            at += 1
    return len(text), False


def c_code_lines(text: str, language: str) -> set[int]:
    """The lines a character of code, or of a literal, stands on, in Rust or in the Mini App's
    TypeScript, JavaScript and Svelte."""
    rust = language == "rust"
    breaks = [at for at, char in enumerate(text) if char == "\n"]
    code: set[int] = set()

    def mark(start: int, end: int) -> None:
        first = bisect.bisect_left(breaks, start) + 1
        code.update(range(first, bisect.bisect_left(breaks, max(start, end - 1)) + 2))

    templates: list[int] = []
    depth, at, last, word = 0, 0, "", ""
    while at < len(text):
        char = text[at]
        if char.isspace():
            at += 1
        elif text.startswith("//", at):
            end = text.find("\n", at)
            at = len(text) if end < 0 else end
        elif text.startswith("/*", at):
            at = block_end(text, at, nested=rust)
        elif language == "svelte" and text.startswith("<!--", at):
            end = text.find("-->", at + 4)
            at = len(text) if end < 0 else end + 3
        elif not rust and (char == "`" or (char == "}" and templates and depth == templates[-1])):
            if char == "}":
                templates.pop()
            end, opened = template_part(text, at + 1)
            if opened:
                templates.append(depth)
            mark(at, end)
            at, last = end, "value"
        else:
            if rust:
                end = rust_literal(text, at)
            else:
                may_begin = last in ("", *REGEX_AFTER) or (last == "word" and word in REGEX_WORDS)
                end = web_literal(text, at, may_begin)
            if end is not None:
                mark(at, end)
                at, last = end, "value"
            elif char.isalnum() or char == "_" or (char == "$" and not rust):
                end = at
                while end < len(text) and (
                    text[end].isalnum() or text[end] == "_" or (text[end] == "$" and not rust)
                ):
                    end += 1
                mark(at, end)
                at, last, word = end, "word", text[at:end]
            else:
                mark(at, at + 1)
                depth += {"{": 1, "}": -1}.get(char, 0)
                at, last = at + 1, char
    return code


def language_of(path: str) -> str:
    if path.endswith(".py"):
        return "python"
    if path.endswith(".rs"):
        return "rust"
    return "svelte" if path.endswith(".svelte") else "js"


#: The items cargo-mutants 27.1.0 never mutates for an attribute (`attrs_excluded` in its
#: `visit.rs`): a function, an `impl` or `trait` block, or a module (SPEC-057 R22). It checks no
#: other item's attributes, so a `#[cfg(test)]` constant, statement or `use` stays production code.
TEST_ITEMS = frozenset({"fn", "mod", "impl", "trait"})
#: The words that may stand between an item's attributes and its keyword.
QUALIFIERS = frozenset({"pub", "const", "async", "unsafe", "extern", "default", "auto"})
BRACKETS = {"(": ")", "[": "]", "{": "}"}
#: How a test-only line is named, in the plan's case and the verdict's lines.
TEST_ONLY = (
    "inside an item marked #[cfg(test)] or with a test attribute, which cargo-mutants never mutates"
)


def rust_tokens(text: str) -> list[tuple[int, int]]:
    """(start, end) of each token of Rust code in `text`, read as `c_code_lines` reads Rust: a
    word, a literal whole, or one other character. Whitespace and comments hold no token, so a
    brace or an attribute inside a string, a raw string, a character or a comment is no token."""
    tokens: list[tuple[int, int]] = []
    at = 0
    while at < len(text):
        char = text[at]
        if char.isspace():
            at += 1
            continue
        if text.startswith("//", at):
            stop = text.find("\n", at)
            at = len(text) if stop < 0 else stop
            continue
        if text.startswith("/*", at):
            at = block_end(text, at, nested=True)
            continue
        stop = rust_literal(text, at)
        if stop is None and (char.isalnum() or char == "_"):
            stop = at
            while stop < len(text) and (text[stop].isalnum() or text[stop] == "_"):
                stop += 1
        stop = at + 1 if stop is None else stop
        tokens.append((at, stop))
        at = stop
    return tokens


def closing(words: list[str], at: int) -> int | None:
    """The index of the bracket that closes the one at `at`, or None when none does."""
    depth = 0
    for index in range(at, len(words)):
        if words[index] == words[at]:
            depth += 1
        elif words[index] == BRACKETS[words[at]]:
            depth -= 1
            if depth == 0:
                return index
    return None


def is_test_mark(inner: list[str]) -> bool:
    """Whether an attribute's inner tokens mark its item as test code the way cargo-mutants 27.1.0
    reads them: exactly `cfg(test)`, or a path whose last segment is `test` (`test`,
    `tokio::test(...)`). A `cfg` that joins `test` to another predicate marks nothing, since the
    tool mutates such an item."""
    if "".join(inner) == "cfg(test)":
        return True
    path = []
    for word in inner:
        if word in ("(", "[", "{", "="):
            break
        path.append(word)
    return "".join(path).split("::")[-1] == "test"


def item_keyword(words: list[str], at: int) -> int | None:
    """The index of the keyword of the item whose attributes end before `at`, past its visibility
    and qualifiers (`pub(crate)`, `async`, `unsafe`, `extern "C"`), when it is one of TEST_ITEMS."""
    while at < len(words):
        word = words[at]
        if word in TEST_ITEMS:
            return at
        if word not in QUALIFIERS:
            return None
        at += 1
        if word == "pub" and words[at : at + 1] == ["("]:
            shut = closing(words, at)
            if shut is None:
                return None
            at = shut + 1
        elif word == "extern" and words[at : at + 1] and words[at].startswith('"'):
            at += 1
    return None


def item_end(words: list[str], at: int) -> int | None:
    """The index of the token that ends the item whose keyword is at `at`: the `}` that closes its
    body, or the `;` of an item without one (`mod tests;`), or None when nothing ends it."""
    depth = 0
    for index in range(at, len(words)):
        word = words[index]
        if word in ("(", "["):
            depth += 1
        elif word in (")", "]"):
            depth -= 1
        elif depth == 0 and word == ";":
            return index
        elif depth == 0 and word == "{":
            return closing(words, index)
    return None


def rust_test_spans(text: str, tokens: list[tuple[int, int]]) -> list[tuple[int, int]]:
    """The character spans of the items cargo-mutants never mutates for an attribute, each from
    its first outer attribute to the token that ends it (SPEC-057 R22). An attribute or an item
    that nothing closes ends the reading: an unread item stays production code."""
    words = [text[start:end] for start, end in tokens]
    spans: list[tuple[int, int]] = []
    at = 0
    while at < len(words):
        if words[at] != "#" or words[at + 1 : at + 2] != ["["]:
            at += 1
            continue
        first, marked = at, False
        while words[at : at + 1] == ["#"] and words[at + 1 : at + 2] == ["["]:
            shut = closing(words, at + 1)
            if shut is None:
                return spans
            marked = marked or is_test_mark(words[at + 2 : shut])
            at = shut + 1
        keyword = item_keyword(words, at) if marked else None
        last = item_end(words, keyword) if keyword is not None else None
        if last is not None:
            spans.append((tokens[first][0], tokens[last][1]))
            at = last + 1
    return spans


def rust_test_lines(text: str) -> set[int]:
    """The 1-based lines of Rust `text` whose every token lies inside a test item (SPEC-057 R22).
    A line that holds any token outside one, a brace that closes a test module before production
    code, say, is production code: the reading errs toward applying, never toward passing."""
    tokens = rust_tokens(text)
    spans = rust_test_spans(text, tokens)
    # The spans are disjoint and in order, since each reading resumes after the item it ended.
    starts = [low for low, _ in spans]
    breaks = [at for at, char in enumerate(text) if char == "\n"]
    inside: set[int] = set()
    outside: set[int] = set()
    for start, stop in tokens:
        lines = range(
            bisect.bisect_left(breaks, start) + 1,
            bisect.bisect_left(breaks, max(start, stop - 1)) + 2,
        )
        span = bisect.bisect_right(starts, start) - 1
        within = span >= 0 and start < spans[span][1]
        (inside if within else outside).update(lines)
    return inside - outside


def changed_lines(root: pathlib.Path, base: str, head: str) -> dict[str, dict]:
    """{path: {added: [new-side line numbers], deleted: n}} for every path the diff changes."""
    text = git(
        root, "diff", "-U0", "--no-color", "--no-ext-diff", "--no-renames", f"{base}...{head}"
    )
    files: dict[str, dict] = {}
    current = None
    old_path = None
    in_header = False
    for line in text.splitlines():
        if line.startswith("diff --git "):
            current, old_path, in_header = None, None, True
        elif in_header and line.startswith("--- "):
            old_path = line[6:] if line.startswith("--- a/") else None
        elif in_header and line.startswith("+++ "):
            path = line[6:] if line.startswith("+++ b/") else old_path
            current = files.setdefault(path, {"added": [], "deleted": 0})
        elif current is not None and (match := HUNK.match(line)):
            # A hunk's lines are content, never headers, until the next `diff --git`.
            in_header = False
            old_count = int(match.group(2)) if match.group(2) is not None else 1
            new_start = int(match.group(3))
            new_count = int(match.group(4)) if match.group(4) is not None else 1
            current["deleted"] += old_count
            current["added"].extend(range(new_start, new_start + new_count))
    return files


def row_span(text: str, find: str) -> range | None:
    """The 1-based lines the anchor `find` covers in `text`, when it occurs exactly once."""
    if text.count(find) != 1:
        return None
    start = text[: text.index(find)].count("\n") + 1
    return range(start, start + find.rstrip("\n").count("\n") + 1)


@dataclass
class Plan:
    base: str
    head: str
    files: list[dict] = field(default_factory=list)
    classes: dict[str, dict] = field(default_factory=dict)
    rows: list[str] = field(default_factory=list)
    row_overlaps: dict[str, str] = field(default_factory=dict)
    stryker_mutate: list[str] = field(default_factory=list)
    scope: dict = field(default_factory=dict)


def selected_rows(root, base, head, changed, plan):
    """R10: the rows on a changed path, the rows added or changed, and the rows whose killer's
    file changed; and for each, whether its anchor covers a changed line."""
    try:
        now = mutation_rows.rows_of(mutation_rows.load_tree(root))
    except FileNotFoundError:
        return
    before = mutation_rows.load_revision(root, base)
    earlier = {row.id: row for row in mutation_rows.rows_of(before)} if before else {}
    paths = set(changed)
    chosen = []
    for row in now:
        killer_file = None
        try:
            killer_file = mutation_rows.locate_killer(root, row).file
        except mutation_rows.KillerUnresolved:
            killer_file = None
        touches_killer = killer_file is not None and (
            killer_file in paths
            or (
                killer_file.endswith("/src") and any(p.startswith(killer_file + "/") for p in paths)
            )
        )
        if row.target in paths or row.id not in earlier or earlier[row.id] != row or touches_killer:
            chosen.append(row.id)
        entry = changed.get(row.target)
        if entry and entry["added"] and (root / row.target).is_file():
            span = row_span((root / row.target).read_text(encoding="utf-8"), row.find)
            if span and set(span) & set(entry["added"]):
                plan.row_overlaps[row.id] = row.target
    plan.rows = sorted(chosen)


def class_case(files: list[dict], name: str) -> str:
    """Why a class applies, or why it is not-applicable, by name, from its files' changed lines:
    its production code lines, and in Rust the test-only lines set apart (SPEC-057 R22)."""
    mine = [entry for entry in files if entry["class"] == name]
    code = sum(len(entry["code"]) for entry in mine)
    test = sum(len(entry["test"]) for entry in mine)
    if code:
        coded = sum(1 for entry in mine if entry["code"])
        case = f"{code} production code line(s) in {coded} file(s)"
        return case + (f"; {test} test-only line(s) set apart, {TEST_ONLY}" if test else "")
    if not mine:
        return f"not-applicable: the diff changes no {name} production file"
    if test:
        tested = sum(1 for entry in mine if entry["test"])
        return (
            f"not-applicable: no production code line changed; {test} test-only line(s) in "
            f"{tested} file(s), {TEST_ONLY}"
        )
    return (
        "not-applicable: no production code line changed; every changed line is blank or a "
        "comment, or deleted"
    )


def plan_diff(
    root: pathlib.Path, base: str, head: str, out: pathlib.Path, scope: tuple[str, str]
) -> Plan:
    out.mkdir(parents=True, exist_ok=True)
    decision, reason = scope
    if decision == "not-applicable":
        plan = Plan(base="", head="", scope={"decision": decision, "reason": reason})
        plan.classes = {name: {"applies": False, "files": []} for name in ("rust", "web", "oracle")}
        (out / "plan.json").write_text(json.dumps(plan.__dict__, indent=2) + "\n", "utf-8")
        return plan
    base_sha = git(root, "rev-parse", "--verify", f"{base}^{{commit}}").strip()
    head_sha = git(root, "rev-parse", "--verify", f"{head}^{{commit}}").strip()
    full = git(root, "diff", "--no-color", "--no-ext-diff", "--no-renames", f"{base}...{head}")
    (out / "git.diff").write_text(full, encoding="utf-8")
    changed = changed_lines(root, base, head)
    plan = Plan(base=base_sha, head=head_sha, scope={"decision": decision, "reason": reason})
    classes = {name: {"applies": False, "files": []} for name in ("rust", "web", "oracle")}
    for path in sorted(changed):
        entry = changed[path]
        klass = classify(path)
        record = {"path": path, "class": klass, "added": len(entry["added"])}
        record["deleted"] = entry["deleted"]
        if klass != "other" and entry["added"]:
            text = git(root, "show", f"{head_sha}:{path}")
            quiet = comment_or_blank(text, language_of(path))
            # A Rust line inside a test item is test-only, never a code line (SPEC-057 R22).
            test = rust_test_lines(text) if klass == "rust" else set()
            record["code"] = [n for n in entry["added"] if n not in quiet and n not in test]
            record["quiet"] = [n for n in entry["added"] if n in quiet]
            record["test"] = [n for n in entry["added"] if n in test and n not in quiet]
        else:
            record["code"], record["quiet"] = [], list(entry["added"])
            record["test"] = []
        plan.files.append(record)
        if klass != "other":
            classes[klass]["files"].append(path)
            if record["code"]:
                classes[klass]["applies"] = True
                if klass == "web":
                    plan.stryker_mutate.append(path[len(WEB_ROOT) :])
    for name, klass in classes.items():
        klass["case"] = class_case(plan.files, name)
    plan.classes = classes
    selected_rows(root, base_sha, head_sha, changed, plan)
    document = json.dumps(plan.__dict__, indent=2) + "\n"
    (out / "plan.json").write_text(document, encoding="utf-8")
    return plan


def say_plan(plan: Plan) -> None:
    print(f"mutation: plan: {plan.scope['decision']}: {plan.scope['reason']}")
    counts = defaultdict(int)
    for entry in plan.files:
        counts[entry["class"]] += 1
    print(
        f"mutation: plan: {len(plan.files)} changed path(s): rust {counts['rust']}, "
        f"web {counts['web']}, oracle {counts['oracle']}, other {counts['other']} "
        f"(base {plan.base[:7]}, head {plan.head[:7]})"
    )
    for name, klass in plan.classes.items():
        verb = "applies" if klass["applies"] else "does not apply"
        case = klass.get("case")
        print(f"mutation: plan: {name} {verb}" + (f": {case}" if case else ""))
    print(f"mutation: plan: {len(plan.rows)} row(s) selected: {' '.join(plan.rows) or 'none'}")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as sink:
            for name, klass in plan.classes.items():
                sink.write(f"{name}={'true' if klass['applies'] else 'false'}\n")
            sink.write(f"rows={'true' if plan.rows else 'false'}\n")
            sink.write(f"scope={plan.scope['decision']}\n")
            sink.write(f"case={plan.scope['decision']}: {plan.scope['reason']}\n")
            sink.write(f"mutate={','.join(plan.stryker_mutate)}\n")


# --------------------------------------------------------------------------- the shards

#: Seconds one mutant costs, its build and its tests, by package: the mean over the weekly
#: battery's 31 reported shards of run 36384080819 on GitHub's ubuntu-24.04 runners, rounded up
#: (R18). A package the table does not name costs the table's highest.
SECONDS_PER_MUTANT = {
    "deck-streak-ingest": 126,
    "deck-streak-daemon": 80,
    "deck-streak-coordination": 64,
    "deck-streak-api": 54,
    "deck-streak-kernel": 13,
    "deck-streak-identity": 8,
    "deck-streak-vault": 8,
}
#: The unmutated baseline each shard builds and tests before its first mutant: the mean over the
#: same 31 shards, rounded up.
BASELINE_SECONDS = 371
#: A shard's projected time may reach an hour, half its job's timeout-minutes of 120: the shards of
#: runs 36373915578 and 36384080819 took from 0.66 to 1.33 times this table's projection (R18).
SHARD_BOUND_SECONDS = 3600
#: The most jobs a matrix may generate in one workflow run (GitHub's workflow syntax).
MAX_SHARDS = 256


def projected(costs: list[int], count: int) -> list[int]:
    """Each of `count` round-robin shards' projected seconds: the baseline, then mutant `i` in
    shard `i mod count`, as cargo-mutants assigns them."""
    totals = [BASELINE_SECONDS] * count
    for index, cost in enumerate(costs):
        totals[index % count] += cost
    return totals


def mutant_costs(packages: list[str]) -> list[int]:
    """Each mutant's projected seconds, by its package; a package the table does not name costs
    the table's highest."""
    highest = max(SECONDS_PER_MUTANT.values())
    return [SECONDS_PER_MUTANT.get(package, highest) for package in packages]


def fewest_shards(costs: list[int]) -> int | None:
    """The fewest round-robin shards whose slowest is projected within the bound, or None when
    even `MAX_SHARDS` do not fit. The one function the per-pull-request plan and a package
    dispatch's sizing share (SPEC-129 R2)."""
    fitting = (
        n for n in range(1, MAX_SHARDS + 1) if max(projected(costs, n)) <= SHARD_BOUND_SECONDS
    )
    return next(fitting, None)


#: The shards a scheduled run and a dispatch with no package sweep (SPEC-129 R4).
WHOLE_SHARDS = 32


def size(listed_path: str | None, package: str | None) -> int:
    """The shards a dispatch runs: `WHOLE_SHARDS` with no package or for the Mini App, which
    reads no listing, else the fewest that fit the bound for the package's listing (SPEC-129
    R2 to R4). A projection past the matrix's limit is refused and never capped."""
    if package in (None, "", MINIAPP):
        count, note = WHOLE_SHARDS, "the whole tree, which is never sized"
    else:
        listed = read_listing(listed_path)
        if not isinstance(listed, list):
            print(
                f"mutation: size: VOID {listed_path or 'no --listed'} holds no cargo-mutants listing"
            )
            return EXIT_VOID
        costs = mutant_costs([str(entry.get("package")) for entry in listed])
        fewest = fewest_shards(costs)
        if fewest is None:
            print(
                f"mutation: size: REFUSED: {len(listed)} mutant(s), projected at {sum(costs)} s "
                f"serially, need more than {MAX_SHARDS} shards within {SHARD_BOUND_SECONDS} s each, "
                "the most a job matrix holds: never capped"
            )
            return EXIT_FAIL
        count = fewest
        times = projected(costs, count)
        note = (
            f"{len(listed)} listed mutant(s), projected at {sum(costs)} s serially, the slowest "
            f"at {max(times)} s of its {SHARD_BOUND_SECONDS} s bound"
        )
    print(f"mutation: size: {count} shard(s) for {note}")
    announce(count)
    return EXIT_OK


def read_listing(path: str | None) -> object | None:
    """A cargo-mutants listing, or None when `path` names no readable one. A file that holds
    nothing is the empty listing: `--list --json --in-diff` prints nothing, not `[]`, when no
    mutant overlaps the diff, since 27.1.0 exits 0 before it lists (SPEC-057 R22). A listing step
    that fails stops its job before `shards` runs, so the empty file is the tool's own answer."""
    try:
        if path and not pathlib.Path(path).read_text(encoding="utf-8").strip():
            return []
    except (OSError, ValueError):
        return None
    return read_json(path)


def announce(count: int) -> None:
    """Write the shard count and its matrix, 0 to count-1, as the step's outputs under GitHub
    Actions: the one place both the plan's `shards` and a dispatch's `size` say them."""
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as sink:
            sink.write(f"shards={count}\n")
            sink.write(f"matrix={json.dumps(list(range(count)))}\n")


def shards(plan_path: pathlib.Path, listed_path: str | None) -> int:
    """The fewest round-robin shards whose slowest is projected within the bound (R18), written
    into the plan with each shard's mutants and, under GitHub Actions, as the matrix's outputs."""
    plan = read_json(str(plan_path))
    if not isinstance(plan, dict) or "classes" not in plan:
        print(f"mutation: shards: VOID {plan_path} is not a mutation plan")
        return EXIT_VOID
    mutants: list[tuple[str, str]] = []
    if plan["classes"]["rust"]["applies"]:
        listed = read_listing(listed_path)
        if not isinstance(listed, list):
            print(
                f"mutation: shards: VOID the rust class applies and {listed_path or 'no --listed'} "
                "holds no cargo-mutants listing"
            )
            return EXIT_VOID
        if not listed:
            # An empty listing never makes the class not-applicable: a changed production line
            # that no tool mutates, a constant's, still needs a row, or the verdict is VOID.
            print(
                "mutation: shards: the listing is empty, cargo-mutants' answer when no mutant "
                "overlaps the diff: each changed production code line needs a row, or the verdict "
                "reads VOID"
            )
        mutants = [(str(entry.get("name")), str(entry.get("package"))) for entry in listed]
    costs = mutant_costs([package for _, package in mutants])
    count = fewest_shards(costs)
    if count is None:
        print(
            f"mutation: shards: REFUSED: {len(mutants)} mutant(s), projected at {sum(costs)} s "
            f"serially, need more than {MAX_SHARDS} shards within {SHARD_BOUND_SECONDS} s each, the "
            "most a job matrix holds: split the change, since a run is never capped"
        )
        return EXIT_FAIL
    times = projected(costs, count)
    plan["shards"] = {
        "count": count,
        "bound_seconds": SHARD_BOUND_SECONDS,
        "baseline_seconds": BASELINE_SECONDS,
        "serial_seconds": sum(costs),
        "shards": [
            {
                "shard": shard,
                "mutants": [
                    name for index, (name, _) in enumerate(mutants) if index % count == shard
                ],
                "projected_seconds": times[shard],
            }
            for shard in range(count)
        ],
    }
    plan_path.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    print(
        f"mutation: shards: {count} shard(s) for {len(mutants)} listed mutant(s), projected at "
        f"{sum(costs)} s serially; the slowest at {max(times)} s of its {SHARD_BOUND_SECONDS} s bound"
    )
    announce(count)
    return EXIT_OK


# --------------------------------------------------------------------------- the verdict


def read_json(path: str | None) -> object | None:
    if not path:
        return None
    try:
        return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


class Verdict:
    """What a class's judgement found: failures, VOIDs, and its report lines."""

    def __init__(self, klass: str) -> None:
        self.klass = klass
        self.failures: list[str] = []
        self.voids: list[str] = []
        self.examined = 0

    def say(self, text: str) -> None:
        print(f"mutation: {self.klass}: {text}")

    def fail(self, text: str) -> None:
        self.failures.append(text)
        self.say(text)

    def void(self, text: str) -> None:
        self.voids.append(text)
        self.say(f"VOID {text}")

    def close(self) -> int:
        if self.failures:
            self.say(f"verdict: FAIL: {len(self.failures)} finding(s)")
            code = EXIT_FAIL
        elif self.voids:
            self.say(f"verdict: VOID: {len(self.voids)} measurement(s) the class needs are missing")
            code = EXIT_VOID
        else:
            self.say("verdict: ok")
            code = EXIT_OK
        print(f"examined {self.examined}")
        return code


def not_applicable(verdict: Verdict, plan: dict, klass: str) -> None:
    files = [entry for entry in plan["files"] if entry["class"] == klass]
    if not files:
        paths = [entry["path"] for entry in plan["files"]]
        shown = ", ".join(paths[:20]) + (f", and {len(paths) - 20} more" if len(paths) > 20 else "")
        verdict.say(
            f"not-applicable: the diff changes no {klass} production file; it changes "
            f"{shown or 'nothing'}"
        )
    for entry in files:
        test, quiet = entry.get("test") or [], len(entry["quiet"])
        if test:
            # SPEC-057 R22: lines cargo-mutants never mutates are named, never VOID.
            verdict.say(
                f"not-applicable: {entry['path']}: {entry['added']} changed line(s): {len(test)} "
                f"test-only, {TEST_ONLY}" + (f"; {quiet} blank or comments" if quiet else "")
            )
        elif entry["added"]:
            verdict.say(
                f"not-applicable: {entry['path']}: {entry['added']} changed line(s), all blank "
                "or comments"
            )
        else:
            verdict.say(
                f"not-applicable: {entry['path']}: {entry['deleted']} line(s) deleted, none added"
            )


def judge_rows(verdict: Verdict, plan: dict, rows: object, klass: str) -> int:
    """Every selected row must be KILLED; returns the rows that carry this class's changed lines."""
    if not plan["rows"]:
        return 0
    if not isinstance(rows, list):
        verdict.void(f"{len(plan['rows'])} row(s) selected and no rows report")
        return 0
    proved = {entry.get("id"): entry for entry in rows if isinstance(entry, dict)}
    carried = 0
    for identifier in plan["rows"]:
        entry = proved.get(identifier)
        if entry is None:
            verdict.void(f"{identifier}: selected and never proved")
        elif entry.get("verdict") != "KILLED":
            verdict.fail(f"{identifier}: {entry.get('verdict')}: {entry.get('reason', '')}")
        else:
            path = plan["row_overlaps"].get(identifier)
            if path is not None and classify(path) == klass:
                carried += 1
    return carried


def partial_reason(report: object, code: int | None, source: str) -> str | None:
    """Why a cargo-mutants report cannot be read whole, or None when it can (R4): no report, an
    exit other than 0, 2 or 3, or counts short of its total."""
    if not isinstance(report, dict) or "caught" not in report:
        return f"no report: {source} holds no cargo-mutants outcomes"
    if code not in WHOLE_EXITS:
        if code is None:
            return "no cargo-mutants exit recorded, so its report may be partial"
        return f"cargo-mutants exit {code}: {TOOL_EXITS.get(code, 'a run that stopped early')}"
    caught, missed = int(report.get("caught", 0)), int(report.get("missed", 0))
    timeout, unviable = int(report.get("timeout", 0)), int(report.get("unviable", 0))
    total = int(report.get("total_mutants", 0))
    if caught + missed + timeout + unviable != total:
        # The tool writes its report as it goes: counts short of its total are a partial run.
        return (
            f"the report counts {caught + missed + timeout + unviable} of {total} mutants reported"
        )
    return None


def whole_reports(verdict: Verdict, plan: dict, args: argparse.Namespace) -> list[tuple[str, dict]]:
    """(where, report) for each whole cargo-mutants report the run promised; each one missing or
    partial is VOID, by name. Under --shard-reports it promised every shard's, from 0 to n-1 (R18),
    but a shard the plan gave no mutant, which cargo-mutants leaves without a report; else the one
    --outcomes names."""
    if not args.shard_reports:
        promised = [
            ("", read_json(args.outcomes), args.tool_exit, args.outcomes or "no --outcomes")
        ]
        listed = [None]
    else:
        planned = (plan.get("shards") or {}).get("shards") or []
        if not planned:
            verdict.void("the plan names no shards, so no shard's report was promised")
        root = pathlib.Path(args.shard_reports)
        promised, listed = [], []
        for shard in range(len(planned)):
            name = root / f"mutation-rust-shard-{shard}"
            promised.append(
                (
                    f"mutation-rust-shard-{shard}: ",
                    read_json(str(name / "mutants.out" / "outcomes.json")),
                    read_exit(name / "cargo-mutants.exit"),
                    "its outcomes.json",
                )
            )
            listed.append(len(planned[shard]["mutants"]))
    whole = []
    for (where, report, code, source), mutants in zip(promised, listed, strict=True):
        if report is None and code == 0 and mutants == 0:
            # cargo-mutants exits 0 and writes no report when it has no mutant to test.
            verdict.say(f"{where}no mutant listed, and cargo-mutants reports none")
            continue
        reason = partial_reason(report, code, source)
        if reason is None:
            whole.append((where, report))
        else:
            verdict.void(f"{where}{reason}")
    return whole


def partition(verdict: Verdict, plan: dict, whole: list[tuple[str, dict]], complete: bool) -> None:
    """The shards' reports hold every mutant the plan listed, each once (R18): one tested in more
    shards than it was listed fails, and, when no shard was missing or partial, one in none is VOID.
    A missing shard is named once, not once for each of its mutants."""
    planned = (plan.get("shards") or {}).get("shards") or []
    listed = Counter(name for shard in planned for name in shard["mutants"])
    home = {
        name: f"mutation-rust-shard-{shard['shard']}"
        for shard in planned
        for name in shard["mutants"]
    }
    tested: dict[str, list[str]] = defaultdict(list)
    for where, report in whole:
        for outcome in report.get("outcomes", []):
            scenario = outcome.get("scenario")
            if isinstance(scenario, dict) and isinstance(scenario.get("Mutant"), dict):
                tested[scenario["Mutant"].get("name")].append(where.rstrip(": "))
    for name, shards_of in sorted(tested.items()):
        if len(shards_of) > listed[name]:
            verdict.fail(
                f"{name}: tested in {len(shards_of)} shard(s) ({', '.join(shards_of)}), "
                f"listed {listed[name]} time(s)"
            )
    if complete:
        for name, times in sorted(listed.items()):
            if len(tested.get(name, [])) < times:
                verdict.void(f"never tested: {name}, listed for {home[name]}")


def judge_rust(verdict: Verdict, plan: dict, args: argparse.Namespace) -> None:
    rows = read_json(args.rows)
    applies = plan["classes"]["rust"]["applies"]
    carried = judge_rows(verdict, plan, rows, "rust") if klass_rows(plan, args) else 0
    if not applies:
        not_applicable(verdict, plan, "rust")
        return
    for entry in plan["files"]:
        if entry["class"] == "rust" and entry["code"]:
            verdict.say(f"{entry['path']}: {len(entry['code'])} changed code line(s)")
        if entry["class"] == "rust" and entry.get("test"):
            verdict.say(
                f"{entry['path']}: {len(entry['test'])} test-only line(s) set apart, {TEST_ONLY}"
            )
    excuses = rust_excuses(verdict, args)
    voids = len(verdict.voids)
    whole = whole_reports(verdict, plan, args)
    complete = len(verdict.voids) == voids
    stopped: set[tuple[str, str]] = set()
    if args.shard_reports:
        held = dict(whole)
        for shard in range(len((plan.get("shards") or {}).get("shards") or [])):
            where = f"mutation-rust-shard-{shard}: "
            directory = pathlib.Path(args.shard_reports) / f"mutation-rust-shard-{shard}"
            stopped |= {
                (where, name)
                for name in memory_cap(
                    verdict.fail, verdict.void, verdict.say, where, directory, held.get(where)
                )
            }
    caught, missed, timeout, unviable, total = (
        sum(int(report.get(key, 0)) for _, report in whole)
        for key in ("caught", "missed", "timeout", "unviable", "total_mutants")
    )
    tool = caught + missed + timeout
    verdict.say(
        f"cargo-mutants examined {tool} (caught {caught}, missed {missed}, timeout {timeout}), "
        f"unviable {unviable}, of {total} on the diff"
    )
    named = equivalent = capped = 0
    for where, report in whole:
        for outcome in report.get("outcomes", []):
            scenario = outcome.get("scenario")
            if not isinstance(scenario, dict):
                continue
            name = scenario.get("Mutant", {}).get("name", "<unnamed mutant>")
            if (where, name) in stopped:
                # The cap stopped this mutant's tests: named by memory_cap, examined by nobody.
                if outcome.get("summary") in ("CaughtMutant", "MissedMutant", "Timeout"):
                    capped += 1
                if outcome.get("summary") == "MissedMutant":
                    named += 1
                continue
            excused = excuses.of(cargo_mutant(scenario.get("Mutant")))
            if outcome.get("summary") == "MissedMutant":
                named += 1
                # R9: a missed mutant exactly one record binds is equivalent, counted apart.
                if len(excused) == 1:
                    equivalent += 1
                    verdict.say(f"{where}EQUIVALENT {name}: {excuse_line(excused[0])}")
                else:
                    verdict.fail(f"{where}MISSED {name}{held_twice(excused)}")
                continue
            for record in excused:
                refuted = refutation(record, name, outcome.get("summary"))
                if refuted is not None:
                    verdict.fail(f"{where}{refuted}")
    if missed > named:
        verdict.fail(f"MISSED {missed - named} mutant(s), unnamed in the report")
    verdict.say(f"missed {missed}: equivalent {equivalent}, unexplained {missed - equivalent}")
    if args.shard_reports:
        partition(verdict, plan, whole, complete=complete)
    verdict.examined = tool - capped + carried
    verdict.say(f"examined {tool - capped} by cargo-mutants and {carried} by rows")
    touched = {mutated_file(o) for _, report in whole for o in report.get("outcomes", [])} - {None}
    for entry in plan["files"]:
        path = entry["path"]
        if entry["class"] == "rust" and entry["code"] and path not in touched:
            if path not in plan["row_overlaps"].values():
                verdict.say(f"unexamined: {path}: no mutant and no row covers its changed lines")
    if verdict.examined == 0:
        verdict.void("production code changed and nothing was examined")


def klass_rows(plan: dict, args: argparse.Namespace) -> bool:
    """The rust job proves the rows; the web job never does."""
    return args.klass in ("rust", "oracle")


def mutated_file(outcome: dict) -> str | None:
    scenario = outcome.get("scenario")
    if isinstance(scenario, dict) and isinstance(scenario.get("Mutant"), dict):
        return scenario["Mutant"].get("file")
    return None


KILL_STATUS = re.compile(r"^\s*SIGKILL\s+\[\s*[\d.]+s\]\s+(?:\(\s*\d+/\d+\)\s+)?(\S+)\s+(\S+)\s*$")
BASELINE = "baseline"


def read_scope_record(directory: pathlib.Path) -> tuple[dict | None, str | None]:
    """The leg's memory-scope record and, when it cannot be used, why not (SPEC-196 R8)."""
    path = directory / "memory-scope.json"
    if not path.is_file():
        return None, "no memory-scope.json: the leg left no record of its memory scope"
    try:
        record = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return None, f"memory-scope.json is unreadable: {error}"
    if not isinstance(record, dict):
        return None, "memory-scope.json is unreadable: it holds no object"
    for key in ("oom", "oom_kill"):
        value = record.get(key)
        if isinstance(value, bool) or not isinstance(value, int):
            return None, f"memory-scope.json is unreadable: {key} is no integer count"
    if record.get("state") != "done":
        return None, f"memory-scope.json is not done (state {record.get('state')})"
    if record.get("in_force") is not True:
        return None, f"the memory scope was not in force: {record.get('reason')}"
    return record, None


def placed_kills(mutants_out: pathlib.Path, report: dict | None) -> set[tuple[str, str, str]]:
    """Each distinct (scenario, binary, test) a scenario log shows the kernel stopped: a nextest
    status line whose first token is SIGKILL, and the summary repeat of it counts once. A scenario
    is named by the outcome whose `log_path` names its log when the report is whole, else by the
    `*** <scenario>` line the log opens with."""
    named: dict[str, str] = {}
    for outcome in (report or {}).get("outcomes", []):
        if not isinstance(outcome, dict) or not isinstance(outcome.get("log_path"), str):
            continue
        scenario = outcome.get("scenario")
        if scenario == "Baseline":
            named[pathlib.PurePosixPath(outcome["log_path"]).name] = BASELINE
        elif isinstance(scenario, dict) and isinstance(scenario.get("Mutant"), dict):
            named[pathlib.PurePosixPath(outcome["log_path"]).name] = str(
                scenario["Mutant"].get("name")
            )
    placed: set[tuple[str, str, str]] = set()
    for log in sorted((mutants_out / "log").glob("*.log")):
        try:
            lines = log.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        first = lines[0].removeprefix("*** ").strip() if lines else ""
        scenario = named.get(log.name) or first or log.stem
        for line in lines[1:]:
            status = KILL_STATUS.match(line)
            if status is not None:
                placed.add((scenario, status.group(1), status.group(2)))
    return placed


def score_memory_cap(fail, say, where: str, name: str) -> bool:
    """The one place a mutant the memory cap stopped is scored: a failure by name, and never
    caught and never a timeout, so the mutant is not examined."""
    fail(
        f"{where}MEMORY-CAP {name}: the memory cap stopped its tests; neither caught nor a timeout"
    )
    return False


def memory_cap(
    fail, void, say, where: str, directory: pathlib.Path, report: dict | None
) -> set[str]:
    """Read one leg's memory-scope record and name each mutant the cap stopped (SPEC-196 R8 to
    R11). Returns the names NOT examined. A leg with no report and no exit was never run, so
    nothing is read."""
    mutants_out = directory / "mutants.out"
    if (
        not (mutants_out / "outcomes.json").is_file()
        and not (directory / "cargo-mutants.exit").is_file()
    ):
        return set()
    record, why = read_scope_record(directory)
    if record is None:
        void(f"{where}{why}")
        return set()
    oom, kills = record["oom"], record["oom_kill"]
    if oom == 0 and kills == 0:
        return set()
    placed = placed_kills(mutants_out, report)
    scenarios = sorted({scenario for scenario, _, _ in placed})
    if kills == 0 or len(placed) != kills:
        fail(
            f"{where}MEMORY-CAP AMBIGUOUS: the scope counted {oom} out-of-memory event(s) and "
            f"{kills} kill(s); the logs place {len(placed)}: {', '.join(scenarios)}"
        )
        return set()
    stopped: set[str] = set()
    for scenario in scenarios:
        if scenario == BASELINE:
            fail(
                f"{where}MEMORY-CAP the unmutated baseline: the memory cap stopped its tests, "
                "so no mutant of this leg was judged against a passing baseline"
            )
        elif not score_memory_cap(fail, say, where, scenario):
            stopped.add(scenario)
    return stopped


def judge_oracle(verdict: Verdict, plan: dict, args: argparse.Namespace) -> None:
    carried = judge_rows(verdict, plan, read_json(args.rows), "oracle")
    if not plan["classes"]["oracle"]["applies"]:
        not_applicable(verdict, plan, "oracle")
        return
    verdict.examined = carried
    verdict.say(f"examined {carried} by rows (the oracle's Python has no generated mutants)")
    if carried == 0:
        verdict.void("the oracle's generator changed and no row covers a changed line")


STRYKER_EXAMINED = ("Killed", "Survived", "NoCoverage", "Timeout")


def judge_web(verdict: Verdict, plan: dict, args: argparse.Namespace) -> None:
    if not plan["classes"]["web"]["applies"]:
        not_applicable(verdict, plan, "web")
        return
    report = read_json(args.stryker)
    if not isinstance(report, dict) or not isinstance(report.get("files"), dict):
        verdict.void(f"no report: {args.stryker or 'no --stryker'} holds no mutation report")
        return
    root = pathlib.Path(args.root)
    records, problems = load_records(root)
    for problem in problems:
        verdict.fail(f"the record: {problem}")
    excuses = Excuses(root, records, "web")
    counts: dict[str, int] = defaultdict(int)
    survived = equivalent = 0
    for path, entry in sorted(report["files"].items()):
        file = f"{WEB_ROOT}{path}"
        if isinstance(entry.get("source"), str):
            excuses.sources.put(file, entry["source"])
        mutants = [(mutant, stryker_mutant(file, mutant)) for mutant in entry.get("mutants", [])]
        # R10: a record of a file the run mutated binds exactly one of that file's mutants.
        mine = [record for record in excuses.records if record.get("file") == file]
        parsed = [mutant for _, mutant in mutants if mutant]
        excuses.bind(parsed, verdict.fail, records=mine, noun=f"mutant of {file}")
        for mutant, bound in mutants:
            status = mutant.get("status", "Pending")
            counts[status] += 1
            where = f"{WEB_ROOT}{path}:{mutant.get('location', {}).get('start', {}).get('line')}"
            what = f"{mutant.get('mutatorName')} -> {mutant.get('replacement')!r}"
            excused = excuses.of(bound)
            if status == "Survived":
                survived += 1
                if len(excused) == 1:
                    equivalent += 1
                    verdict.say(f"EQUIVALENT {where}: {what}: {excuse_line(excused[0])}")
                else:
                    verdict.fail(f"{status} {where}: {what}{held_twice(excused)}")
                continue
            if status == "NoCoverage":
                verdict.fail(f"{status} {where}: {what}")
            elif status == "Ignored":
                verdict.fail(
                    f"Ignored {where}: {what}: no mutant may be ignored, by a disable comment, an "
                    "ignorer or an excluded mutator (SPEC-057 R10, R11)"
                )
            elif status == "Pending":
                verdict.void(f"Pending {where}: never run")
            for record in excused:
                refuted = refutation(record, f"{where}: {what}", status)
                if refuted is not None:
                    verdict.fail(refuted)
    verdict.examined = sum(counts[status] for status in STRYKER_EXAMINED)
    summary = ", ".join(f"{status} {counts[status]}" for status in sorted(counts))
    verdict.say(f"Stryker examined {verdict.examined} ({summary or 'no mutant'})")
    verdict.say(
        f"survived {survived}: equivalent {equivalent}, unexplained {survived - equivalent}"
    )
    if verdict.examined == 0:
        verdict.void("production code changed and nothing was examined")


def judge(args: argparse.Namespace) -> int:
    plan = read_json(args.plan)
    verdict = Verdict(args.klass)
    if not isinstance(plan, dict) or "classes" not in plan:
        verdict.void(f"no plan: {args.plan} is not a mutation plan")
        return verdict.close()
    scope = plan.get("scope") or {}
    if scope.get("decision") == "not-applicable":
        verdict.say(f"not-applicable: {scope.get('reason')}")
        return verdict.close()
    {"rust": judge_rust, "web": judge_web, "oracle": judge_oracle}[args.klass](verdict, plan, args)
    return verdict.close()


# --------------------------------------------------------------------------- the weekly survivors


def survivors_in(reports: pathlib.Path, root: pathlib.Path) -> tuple[dict[str, list[str]], int]:
    """{file: [each survivor, one line]} across every report under `reports`, less each missed or
    survived mutant that exactly one record binds, for which no issue is drafted (SPEC-057 R12);
    and how many those were."""
    found: dict[str, list[str]] = defaultdict(list)
    records, _ = load_records(root)
    excused = 0
    rust = Excuses(root, records, "rust")
    documents = [read_json(str(path)) for path in sorted(reports.rglob("outcomes.json"))]
    outcomes = [
        outcome
        for document in documents
        if isinstance(document, dict)
        for outcome in document.get("outcomes", [])
    ]
    tested = [cargo_mutant(outcome_mutant(outcome)) for outcome in outcomes]
    rust.bind([mutant for mutant in tested if mutant], lambda _: None)
    for outcome, mutant in zip(outcomes, tested, strict=True):
        if outcome.get("summary") == "MissedMutant" and mutated_file(outcome):
            if len(rust.of(mutant)) == 1:
                excused += 1
                continue
            found[mutated_file(outcome)].append(outcome["scenario"]["Mutant"]["name"])
    web = Excuses(root, records, "web")
    for path in sorted(reports.rglob("mutation.json")):
        document = read_json(str(path)) or {}
        for name, entry in sorted((document.get("files") or {}).items()):
            file = f"{WEB_ROOT}{name}"
            if isinstance(entry.get("source"), str):
                web.sources.put(file, entry["source"])
            parsed = [(mutant, stryker_mutant(file, mutant)) for mutant in entry.get("mutants", [])]
            mine = [record for record in web.records if record.get("file") == file]
            web.bind([bound for _, bound in parsed if bound], lambda _: None, records=mine)
            for mutant, bound in parsed:
                if mutant.get("status") == "Survived" and len(web.of(bound)) == 1:
                    excused += 1
                    continue
                if mutant.get("status") in ("Survived", "NoCoverage"):
                    line = mutant.get("location", {}).get("start", {}).get("line")
                    found[file].append(
                        f"{file}:{line}: {mutant.get('status')}: "
                        f"{mutant.get('mutatorName')} -> {mutant.get('replacement')!r}"
                    )
    for path in sorted(reports.rglob("rows.json")):
        document = read_json(str(path)) or []
        for entry in document if isinstance(document, list) else []:
            if entry.get("verdict") in ("SURVIVED", "VOID"):
                found[entry.get("target", "<no target>")].append(
                    f"row {entry.get('id')}: {entry.get('verdict')}: {entry.get('reason', '')}"
                )
    return found, excused


def draft_body(path: str, lines: list[str]) -> str:
    run = ""
    if os.environ.get("GITHUB_RUN_ID") and os.environ.get("GITHUB_REPOSITORY"):
        server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        run = (
            f"{server}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}"
        )
    listed = "\n".join(f"- `{line}`" for line in lines)
    source = f"The weekly mutation battery's run {run}" if run else "The weekly mutation battery"
    return (
        f"{source} found {len(lines)} mutant(s) of `{path}` that no test killed (SPEC-039 R12).\n\n"
        f"{listed}\n\n"
        "Triage each one (SPEC-057 R1): kill it with a test that asserts the behaviour the mutant "
        "breaks, or, when no test can tell it apart, record it EQUIVALENT in "
        "`scripts/mutation-equivalent.d/<package>.json`, `miniapp.json` for the Mini App "
        "(ADR-070): its `file`, `mutant`, `anchor`, `reason`, `evidence`, a Rust mutant's "
        "`reached_by`, and `issue`. An uncovered Mini App mutant first gains a test that reaches "
        "it. A row that SURVIVED or is VOID is repaired in its band fragment under "
        "`scripts/mutation-rows.d/`.\n"
    )


def survivors(args: argparse.Namespace) -> int:
    reports, out = pathlib.Path(args.reports), pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    open_titles = set(read_json(args.open_titles) or [])
    manifest = []
    found, excused = survivors_in(reports, pathlib.Path(args.root))
    for number, (path, lines) in enumerate(sorted(found.items()), start=1):
        name = f"draft-{number:03d}.md"
        (out / name).write_text(draft_body(path, lines), encoding="utf-8")
        title = TITLE.format(path)
        manifest.append({"title": title, "body": name, "open": title in open_titles})
        state = "already open, not filed again" if title in open_titles else "to file"
        print(f"survivors: {title}: {len(lines)} mutant(s): {state}")
    (out / "drafts.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"survivors: {excused} mutant(s) the equivalence record excuses, drafted as no issue")
    print(f"examined {len(manifest)} file(s) with survivors")
    return EXIT_OK


def read_exit(path: pathlib.Path) -> int | None:
    """The exit a job recorded beside its report, or None when it recorded none."""
    try:
        return int(path.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        return None


def battery(
    reports: pathlib.Path, shards: int, package: str | None = None, listed: str | None = None
) -> int:
    """Every report the weekly battery's jobs promise, counted whole (R12): each shard's
    `outcomes.json` with an exit of 0, 2 or 3 and counts that sum to its total, the rows' report
    with at least one row proved, and the Stryker sweep's `mutation.json`. A runner shut down
    mid-run uploads nothing, and a shard stopped early leaves a partial report: either is named, and
    fails the battery, rather than read as a shard with no survivor. A dispatch scoped to one
    package (SPEC-057 R14) promises the shards its scope gave a mutant, which the whole tree's
    listing counts, since shard k holds a mutant when the scope lists more than k; scoped to the
    Mini App it promises the Stryker sweep alone."""
    findings: list[str] = []

    def findings_append(text: str) -> None:
        findings.append(f"battery: {text}")

    whole = promised = 0
    owed = range(shards)
    if listed is not None:
        promised += 1
        listing = read_json(listed)
        if not isinstance(listing, list):
            findings.append(f"battery: MISSING listing: {listed} holds no cargo-mutants listing")
        else:
            whole += 1
            entries = [entry for entry in listing if isinstance(entry, dict)]
            count = sum(1 for entry in entries if package in (None, entry.get("package")))
            if package not in (None, MINIAPP) and not count:
                packages = ", ".join(sorted({str(entry.get("package")) for entry in entries}))
                findings.append(
                    f"battery: the scope {package} lists no mutant; the listing's packages are "
                    f"{packages or 'none'}, and the Mini App is {MINIAPP}"
                )
            owed = range(0 if package == MINIAPP else min(shards, count))
    elif package == MINIAPP:
        owed = range(0)
    for shard in range(shards):
        name = f"mutants-shard-{shard}"
        if shard not in owed:
            print(f"battery: {name}: the scope lists no mutant for it, so it owes no report")
            continue
        promised += 1
        report = read_json(str(reports / name / "mutants.out" / "outcomes.json"))
        code = read_exit(reports / name / "cargo-mutants.exit")
        if not isinstance(report, dict) or "total_mutants" not in report:
            findings.append(f"battery: MISSING {name}: no outcomes.json")
        elif code is None:
            findings.append(f"battery: PARTIAL {name}: no cargo-mutants exit recorded")
        elif code not in WHOLE_EXITS:
            findings.append(f"battery: PARTIAL {name}: cargo-mutants exit {code}")
        else:
            counted = sum(int(report.get(key, 0)) for key in ("caught", "missed", "timeout"))
            counted += int(report.get("unviable", 0))
            if counted != int(report["total_mutants"]):
                findings.append(
                    f"battery: PARTIAL {name}: {counted} of {report['total_mutants']} mutants "
                    "reported"
                )
            else:
                whole += 1
        memory_cap(
            findings_append,
            findings_append,
            lambda text: print(f"battery: {text}"),
            f"{name}: ",
            reports / name,
            report if isinstance(report, dict) and code in WHOLE_EXITS else None,
        )
    if package is None:
        promised += 1
        rows = read_json(str(reports / "rows" / "rows.json"))
        if rows is None:
            findings.append("battery: MISSING rows: no rows.json")
        elif not isinstance(rows, list) or not rows:
            findings.append("battery: PARTIAL rows: rows.json proves no row")
        else:
            whole += 1
    if package in (None, MINIAPP):
        promised += 1
        sweep = reports / "stryker"
        found = sorted(sweep.rglob("mutation.json")) if sweep.is_dir() else []
        documents = [read_json(str(path)) for path in found]
        if not found:
            findings.append("battery: MISSING stryker: no mutation.json")
        elif len(found) > 1 or not isinstance(documents[0], dict) or "files" not in documents[0]:
            findings.append(f"battery: PARTIAL stryker: {len(found)} mutation.json, not one report")
        else:
            whole += 1
    for foreign in sorted(reports.glob("mutants-shard-*")):
        number = foreign.name.removeprefix("mutants-shard-")
        if number.isdigit() and int(number) >= shards:
            findings.append(
                f"battery: FOREIGN {foreign.name}: the run was sized at {shards} shard(s)"
            )
    for finding in findings:
        print(finding)
    print(f"battery: counted {whole} of {promised} reports whole")
    print(f"examined {promised} report(s)")
    return EXIT_FAIL if findings else EXIT_OK


# --------------------------------------------------------------------------- the configurations

#: The keys `.cargo/mutants.toml` may hold, and what each takes: cargo-mutants 27.1.0's `Config`
#: (`src/config.rs`, `#[serde(default, deny_unknown_fields)]`, so any other key refuses the file)
#: and the `Common` options it flattens in (`src/options.rs`: `test_tool` and `sharding`; its
#: `emit_diffs` is `#[serde(skip)]`, a command-line option only).
CARGO_MUTANTS_KEYS: dict[str, object] = {
    "additional_cargo_args": "strings",
    "additional_cargo_test_args": "strings",
    "all_features": "boolean",
    "build_timeout_multiplier": "number",
    "cap_lints": "boolean",
    "copy_vcs": "boolean",
    "copy_target": "boolean",
    "error_values": "strings",
    "examine_globs": "strings",
    "examine_re": "strings",
    "exclude_globs": "strings",
    "exclude_re": "strings",
    "gitignore": "boolean",
    "features": "strings",
    "minimum_test_timeout": "number",
    "no_default_features": "boolean",
    "output": "string",
    "profile": "string",
    "skip_calls": "strings",
    "skip_calls_defaults": "boolean",
    "test_package": "strings",
    "test_workspace": "boolean",
    "timeout_multiplier": "number",
    "test_tool": ("cargo", "nextest"),
    "sharding": ("slice", "round-robin"),
}
#: StrykerJS's `thresholds` (`packages/api/schema/stryker-core.json`, `mutationScoreThresholds`,
#: `additionalProperties: false`): each a percentage from 0 to 100, `break` also null; the options
#: validator refuses `high` below `low` once these defaults apply.
STRYKER_THRESHOLDS = {"high": 80, "low": 60, "break": None}
#: The names StrykerJS reads a configuration from in its working directory, the first it finds
#: winning: `stryker.conf` or `stryker.config`, with or without a leading dot, in `.json`, `.js`,
#: `.mjs` or `.cjs` (its config-file chapter).
STRYKER_CONFIG_NAMES = tuple(
    f"{dot}stryker.{stem}{extension}"
    for dot in ("", ".")
    for stem in ("conf", "config")
    for extension in (".json", ".js", ".mjs", ".cjs")
)
#: R2's web production code as Stryker's `mutate` globs, from `web/app`.
STRYKER_MUTATE = (
    "src/**/*.ts",
    "src/**/*.js",
    "src/**/*.svelte",
    "!src/**/*.test.*",
    "!src/**/*.spec.*",
    "!src/**/*.d.ts",
    "!src/lib/paraglide/**",
)


def takes(value: object, kind: object) -> bool:
    if isinstance(kind, tuple):
        return value in kind
    if kind == "strings":
        return isinstance(value, list) and all(isinstance(item, str) for item in value)
    if kind == "boolean":
        return isinstance(value, bool)
    if kind == "number":
        return isinstance(value, int | float) and not isinstance(value, bool)
    return isinstance(value, str)


def cargo_mutants_findings(text: str) -> list[str]:
    where = ".cargo/mutants.toml"
    try:
        document = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        return [f"configs: {where}: is not TOML: {error}"]
    findings = []
    for key, value in document.items():
        kind = CARGO_MUTANTS_KEYS.get(key)
        if kind is None:
            findings.append(f"configs: {where}: {key} is not a cargo-mutants 27.1.0 key")
        elif not takes(value, kind):
            wanted = f"not one of {', '.join(kind)}" if isinstance(kind, tuple) else f"not a {kind}"
            if kind == "strings":
                wanted = "not a list of strings"
            shown = f"{value!r}, " if isinstance(kind, tuple) else ""
            findings.append(f"configs: {where}: {key} is {shown}{wanted}")
    return findings


def reject_constant(name: str) -> object:
    raise ValueError(f"{name} is not JSON")


def stryker_findings(text: str) -> list[str]:
    where = "web/app/stryker.config.json"
    try:
        document = json.loads(text, parse_constant=reject_constant)
    except ValueError:
        return [f"configs: {where}: is not JSON (StrykerJS parses it strictly, with no comment)"]
    if not isinstance(document, dict):
        return [f"configs: {where}: is not an object"]
    findings = []
    if document.get("testRunner") != "vitest":
        findings.append(f"configs: {where}: testRunner is not vitest (SPEC-039 R6)")
    if "json" not in (document.get("reporters") or []):
        findings.append(f"configs: {where}: no json reporter, so no verdict can read a report")
    # StrykerJS finds a static mutant only by per-test coverage, and refuses ignoreStatic without it.
    coverage = document.get("coverageAnalysis", "perTest")
    if document.get("ignoreStatic") is True and coverage != "perTest":
        findings.append(
            f"configs: {where}: ignoreStatic needs coverageAnalysis perTest, not {coverage}"
        )
    if sorted(document.get("mutate") or []) != sorted(STRYKER_MUTATE):
        findings.append(f"configs: {where}: mutate is not R2's web production code")
    thresholds = document.get("thresholds", {})
    if not isinstance(thresholds, dict):
        return [*findings, f"configs: {where}: thresholds is not an object"]
    values = dict(STRYKER_THRESHOLDS)
    for key, value in thresholds.items():
        if key not in STRYKER_THRESHOLDS:
            findings.append(f"configs: {where}: thresholds.{key} is not a threshold")
        elif value is None and key == "break":
            values[key] = None
        elif not takes(value, "number"):
            findings.append(f"configs: {where}: thresholds.{key} is not a number")
        elif not 0 <= value <= 100:
            findings.append(f"configs: {where}: thresholds.{key} {value} is outside 0 to 100")
        else:
            values[key] = value
    if takes(values["high"], "number") and values["high"] < values["low"]:
        findings.append(
            f"configs: {where}: thresholds.high {values['high']} is below thresholds.low "
            f"{values['low']}"
        )
    return findings


def configs(root: pathlib.Path) -> int:
    """Each mutation tool's configuration, judged by what the tool itself refuses (R6), and by the
    two things the verdict needs of Stryker's: the Vitest runner and its json report."""
    findings: list[str] = []
    examined = 0
    for relative, judge_text in (
        (".cargo/mutants.toml", cargo_mutants_findings),
        ("web/app/stryker.config.json", stryker_findings),
    ):
        path = root / relative
        if not path.is_file():
            findings.append(f"configs: {relative}: is absent")
            continue
        examined += 1
        findings += judge_text(path.read_text(encoding="utf-8"))
    for name in STRYKER_CONFIG_NAMES:
        if name != "stryker.config.json" and (root / "web" / "app" / name).is_file():
            findings.append(
                f"configs: web/app/{name}: a second Stryker configuration; StrykerJS reads the "
                "first of its default names it finds, which may not be stryker.config.json"
            )
    for finding in findings:
        print(finding)
    print(f"examined {examined} configuration(s)")
    if findings:
        return EXIT_FAIL
    return EXIT_OK if examined else EXIT_VOID


# --------------------------------------------------------------------------- the exclusions

#: What hides a mutant from cargo-mutants' listing or from StrykerJS's run (SPEC-057 R11, ADR-070 D1
#: and D6): the keys that filter the listing, the attributes that skip an item's mutants, a disable
#: comment, and the Stryker options that leave a mutant ignored.
EXCLUDING_KEYS = ("exclude_re", "exclude_globs", "examine_re", "examine_globs", "skip_calls")
EXCLUDING_ATTRIBUTE = re.compile(r"\bmutants::(?:skip|exclude_re)\b")
WEB_SOURCES = frozenset({".ts", ".js", ".svelte", ".mts", ".cts", ".mjs", ".cjs", ".tsx", ".jsx"})


def exclusions(root: pathlib.Path) -> int:
    """No exclusion hides a mutant (R11): each form is refused by name, with a reason or without,
    and a key even when it is empty. It counts the files it reads, and a tree of none is VOID."""
    findings: list[str] = []
    examined = 0
    config = root / ".cargo" / "mutants.toml"
    if config.is_file():
        examined += 1
        try:
            document = tomllib.loads(config.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as error:
            document = {}
            findings.append(
                f"exclusions: .cargo/mutants.toml: not TOML, so no key is read: {error}"
            )
        for key in EXCLUDING_KEYS:
            if key in document:
                findings.append(
                    f"exclusions: .cargo/mutants.toml: the {key} key hides mutants from "
                    "cargo-mutants' listing (SPEC-057 R11)"
                )
    crates = root / "crates"
    for path in sorted(crates.glob("*/src/**/*.rs")) if crates.is_dir() else []:
        examined += 1
        where = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            attribute = EXCLUDING_ATTRIBUTE.search(line)
            if attribute and not line.strip().startswith("//"):
                findings.append(
                    f"exclusions: {where}:{number}: {attribute.group(0)} hides its item's mutants "
                    "from cargo-mutants' listing (SPEC-057 R11)"
                )
    web = root / "web" / "app" / "src"
    for path in sorted(web.rglob("*")) if web.is_dir() else []:
        if path.suffix not in WEB_SOURCES or not path.is_file():
            continue
        examined += 1
        where = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if "Stryker disable" in line:
                findings.append(
                    f"exclusions: {where}:{number}: a Stryker disable comment leaves a mutant "
                    "ignored, never run (SPEC-057 R11)"
                )
    stryker = root / "web" / "app" / "stryker.config.json"
    if stryker.is_file():
        examined += 1
        document = read_json(str(stryker))
        if not isinstance(document, dict):
            document = {}
            findings.append("exclusions: web/app/stryker.config.json: not JSON, so nothing is read")
        mutator = document.get("mutator") if isinstance(document.get("mutator"), dict) else {}
        for name in mutator.get("excludedMutations") or []:
            findings.append(
                f"exclusions: web/app/stryker.config.json: excludes the mutator {name} "
                "(SPEC-057 R11)"
            )
        for name in document.get("ignorers") or []:
            findings.append(
                f"exclusions: web/app/stryker.config.json: the ignorer {name} leaves mutants "
                "ignored (SPEC-057 R11)"
            )
        if document.get("ignoreStatic") is True:
            findings.append(
                "exclusions: web/app/stryker.config.json: ignoreStatic leaves every static "
                "mutant ignored (SPEC-057 R11)"
            )
    for finding in findings:
        print(finding)
    print(f"examined {examined} file(s)")
    if findings:
        return EXIT_FAIL
    return EXIT_OK if examined else EXIT_VOID


# --------------------------------------------------------------------------- the equivalence record

#: The equivalence record (SPEC-057 R4, ADR-070 D1): one fragment per Cargo package, named for it,
#: and one for the Mini App, each `{"records": [...]}`.
RECORDS = "scripts/mutation-equivalent.d"
MINIAPP = "miniapp"
#: What every record carries (R5). A Rust record also names `reached_by`; `span` is written only when
#: two mutants of one description start at one position inside the anchor.
RECORD_FIELDS = ("file", "mutant", "anchor", "reason", "evidence", "issue")
ISSUE = re.compile(r"#[1-9][0-9]*")


@dataclass(frozen=True)
class Record:
    """One equivalence claim, and where it is held."""

    fragment: str
    index: int
    fields: dict

    @property
    def klass(self) -> str:
        return "web" if self.fragment == f"{MINIAPP}.json" else "rust"

    @property
    def package(self) -> str:
        return self.fragment.removesuffix(".json")

    def get(self, name: str) -> str | None:
        """The field's text, or None when it is absent, not a text or blank."""
        value = self.fields.get(name)
        return value if isinstance(value, str) and value.strip() else None

    @property
    def named(self) -> str:
        return f"record {self.index} ({self.get('file') or '?'}: {self.get('mutant') or '?'})"

    def __str__(self) -> str:
        return f"{self.fragment} {self.named}"


def load_records(root: pathlib.Path) -> tuple[list[Record], list[str]]:
    """Every record of every fragment, and each fragment that holds no records list, by name."""
    records: list[Record] = []
    problems: list[str] = []
    directory = root / RECORDS
    for path in sorted(directory.glob("*.json")) if directory.is_dir() else []:
        try:
            document = mutation_rows.parse_document(
                f"{RECORDS}/{path.name}", path.read_text(encoding="utf-8")
            )
        except mutation_rows.PopulationRefused as refusal:
            problems.append(str(refusal))
            continue
        except (OSError, UnicodeDecodeError):
            document = None
        entries = document.get("records") if isinstance(document, dict) else None
        if not isinstance(entries, list):
            problems.append(f'{path.name}: holds no records list, {{"records": [...]}}')
            continue
        for index, entry in enumerate(entries, start=1):
            records.append(Record(path.name, index, entry if isinstance(entry, dict) else {}))
    return records, problems


class Sources:
    """Each file a record or a mutant names: its text, and the offset where each line starts."""

    def __init__(self, root: pathlib.Path) -> None:
        self.root = root
        self.texts: dict[str, str | None] = {}
        self.starts: dict[str, list[int]] = {}

    def put(self, file: str, text: str) -> None:
        """Read `file` as `text`: the Stryker report's copy of what it mutated."""
        self.texts[file] = text
        self.starts.pop(file, None)

    def text(self, file: str) -> str | None:
        if file not in self.texts:
            path = self.root / file
            try:
                self.texts[file] = path.read_text(encoding="utf-8") if path.is_file() else None
            except (OSError, UnicodeDecodeError):
                self.texts[file] = None
        return self.texts[file]

    def offset(self, file: str, line: int, column: int) -> int | None:
        """The offset of a 1-based line and column, as cargo-mutants and StrykerJS report them."""
        text = self.text(file)
        if text is None:
            return None
        if file not in self.starts:
            self.starts[file] = [0] + [at + 1 for at, char in enumerate(text) if char == "\n"]
        starts = self.starts[file]
        if not 1 <= line <= len(starts) or column < 1:
            return None
        return starts[line - 1] + column - 1

    def window(self, file: str, anchor: str) -> tuple[int, int] | None:
        """Where `anchor` lies in `file`, when it occurs there exactly once."""
        text = self.text(file)
        if text is None or not anchor or text.count(anchor) != 1:
            return None
        start = text.index(anchor)
        return start, start + len(anchor)


@dataclass(frozen=True)
class Mutant:
    """A mutant as a record binds it: its file, the tool's own description, and its span."""

    file: str
    description: str
    start: tuple[int, int]
    end: tuple[int, int]
    name: str


def span_of(location: object) -> tuple[tuple[int, int], tuple[int, int]] | None:
    try:
        start, end = location["start"], location["end"]
        return (int(start["line"]), int(start["column"])), (
            int(end["line"]),
            int(end["column"]),
        )
    except (KeyError, TypeError, ValueError):
        return None


def cargo_mutant(entry: object) -> Mutant | None:
    """A mutant of cargo-mutants' listing (`--list --json`) or of an outcome's `scenario.Mutant`,
    whose description is its name after `<file>:<line>:<column>: `."""
    if not isinstance(entry, dict):
        return None
    span, file, name = span_of(entry.get("span")), entry.get("file"), entry.get("name")
    if span is None or not isinstance(file, str) or not isinstance(name, str):
        return None
    prefix = f"{file}:{span[0][0]}:{span[0][1]}: "
    if not name.startswith(prefix):
        return None
    return Mutant(file, name[len(prefix) :], span[0], span[1], name)


def stryker_mutant(file: str, entry: object) -> Mutant | None:
    """A mutant of a Stryker report, whose description is `<mutatorName>: <replacement>`."""
    if not isinstance(entry, dict):
        return None
    span = span_of(entry.get("location"))
    if span is None:
        return None
    description = f"{entry.get('mutatorName')}: {entry.get('replacement')}"
    return Mutant(file, description, span[0], span[1], f"{file}:{span[0][0]}: {description}")


def outcome_mutant(outcome: object) -> dict | None:
    """The mutant a cargo-mutants outcome tested, or None for the baseline."""
    scenario = outcome.get("scenario") if isinstance(outcome, dict) else None
    mutant = scenario.get("Mutant") if isinstance(scenario, dict) else None
    return mutant if isinstance(mutant, dict) else None


def binds(record: Record, mutant: Mutant, sources: Sources) -> bool:
    """R6: the record's file and description, a span that starts inside the anchor's one
    occurrence, and, when the record names one, the mutated text."""
    if record.get("file") != mutant.file or record.get("mutant") != mutant.description:
        return False
    window = sources.window(mutant.file, str(record.fields.get("anchor") or ""))
    at = sources.offset(mutant.file, *mutant.start)
    if window is None or at is None or not window[0] <= at < window[1]:
        return False
    if "span" not in record.fields:
        return True
    end = sources.offset(mutant.file, *mutant.end)
    return end is not None and sources.text(mutant.file)[at:end] == record.fields["span"]


class Excuses:
    """One class's records, each held to exactly one mutant of the population it is bound against
    (ADR-070 D4): the whole tree's listing (R8), or else the mutants the run reports. A record that
    binds none is STALE and one that binds two AMBIGUOUS, and neither excuses a mutant."""

    def __init__(self, root: pathlib.Path, records: list[Record], klass: str) -> None:
        self.sources = Sources(root)
        self.records = [record for record in records if record.klass == klass]
        self.valid: list[Record] = []

    def bind(self, population, fail, records=None, noun: str = "listed mutant") -> None:
        for record in self.records if records is None else records:
            bound = [mutant for mutant in population if binds(record, mutant, self.sources)]
            if len(bound) == 1:
                self.valid.append(record)
            elif not bound:
                fail(f"STALE {record}: binds no {noun}")
            else:
                named = "; ".join(mutant.name for mutant in bound)
                fail(f"AMBIGUOUS {record}: binds {len(bound)} {noun}s: {named}")

    def of(self, mutant: Mutant | None) -> list[Record]:
        """The records that excuse `mutant`: more than one is a record held twice."""
        if mutant is None:
            return []
        return [record for record in self.valid if binds(record, mutant, self.sources)]


def held_twice(records: list[Record]) -> str:
    if len(records) < 2:
        return ""
    return f": held twice, by {' and '.join(map(str, records))}, so neither excuses it"


def refutation(record: Record, name: str, outcome: str) -> str | None:
    """What an outcome other than missed or survived makes of the record bound to it (R9, R10)."""
    if outcome in ("CaughtMutant", "Killed"):
        return f"REFUTED {record}: its mutant {name} was caught"
    if outcome == "Timeout":
        return f"REFUTED {record}: its mutant {name} timed out"
    if outcome == "Unviable":
        return f"UNNEEDED {record}: its mutant {name} is unviable"
    if outcome in ("CompileError", "RuntimeError"):
        return f"UNNEEDED {record}: its mutant {name} is a {outcome}"
    if outcome == "NoCoverage":
        return (
            f"UNCOVERED {record}: its mutant {name} is uncovered: no test reaches it, so it is "
            "untested, not equivalent"
        )
    return None


def excuse_line(record: Record) -> str:
    return f"{record.get('reason')} ({record.get('issue')})"


def rust_excuses(verdict: Verdict, args: argparse.Namespace) -> Excuses:
    """The Rust records, bound against the whole tree's listing the plan made (R8)."""
    root = pathlib.Path(args.root)
    records, problems = load_records(root)
    for problem in problems:
        verdict.fail(f"the record: {problem}")
    excuses = Excuses(root, records, "rust")
    if not excuses.records:
        return excuses
    listing = read_json(args.whole)
    if not isinstance(listing, list):
        verdict.void(
            f"{len(excuses.records)} Rust record(s) and no whole-tree listing to bind them "
            f"against: {args.whole or 'no --whole'} holds none (SPEC-057 R8)"
        )
        return excuses
    excuses.bind([mutant for mutant in map(cargo_mutant, listing) if mutant], verdict.fail)
    return excuses


def workspace_packages(root: pathlib.Path) -> dict[str, str]:
    """{package name: its directory under crates/}."""
    found = {}
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        try:
            name = tomllib.loads(manifest.read_text(encoding="utf-8"))["package"]["name"]
        except (OSError, KeyError, TypeError, tomllib.TOMLDecodeError):
            continue
        found[name] = manifest.parent.name
    return found


def same_words(one: str, other: str) -> bool:
    return " ".join(one.split()).casefold() == " ".join(other.split()).casefold()


def record_problems(
    root: pathlib.Path, record: Record, crate: str | None, sources: Sources
) -> list[str]:
    """What the census refuses in one record (R7), each by name."""
    fields = RECORD_FIELDS + (("reached_by",) if record.klass == "rust" else ())
    problems = [f"lacks {name}" for name in fields if record.get(name) is None]
    if "span" in record.fields and record.get("span") is None:
        problems.append("its span is not a text: leave it out, or give the mutated text")
    reason, evidence, issue = (
        record.get("reason"),
        record.get("evidence"),
        record.get("issue"),
    )
    file, anchor = record.get("file"), record.get("anchor")
    if reason and len(reason.strip().splitlines()) > 1:
        problems.append(f"its reason spans {len(reason.strip().splitlines())} lines")
    if reason and evidence and same_words(reason, evidence):
        problems.append("its evidence repeats its reason")
    if issue and not ISSUE.fullmatch(issue):
        problems.append(f"its issue {issue!r} is not #N")
    if file:
        if record.klass == "web":
            inside = classify(file) == "web"
        else:
            inside = classify(file) == "rust" and file.startswith(f"crates/{crate}/src/")
        if not inside:
            problems.append(
                f"its file {file} lies outside {record.package}'s production code (SPEC-039 R2)"
            )
        text = sources.text(file)
        if text is None:
            problems.append(f"its file {file} does not exist")
        elif anchor and text.count(record.fields["anchor"]) != 1:
            problems.append(
                f"its anchor occurs {text.count(record.fields['anchor'])} times in {file}"
            )
    reached = record.get("reached_by")
    if record.klass == "rust" and reached:
        row = mutation_rows.Row(
            id=f"{record.fragment}:{record.index}",
            table="MUTATIONS",
            target=file or "",
            find="",
            replace="",
            killer=reached,
            crate=crate,
            description="",
        )
        try:
            mutation_rows.resolve_killer(root, row)
        except mutation_rows.KillerUnresolved as refusal:
            problems.append(f"reached_by: {refusal}")
    return problems


def census(root: pathlib.Path) -> int:
    """Every record held whole, with no mutation tool (R7): each field, a one-line reason, evidence
    that is not the reason again, an issue, an anchor that occurs once in its file, a file of its
    fragment's package, a `reached_by` that resolves to one test of that package, a fragment named
    for a package, and no record held twice. A tree may hold none."""
    records, problems = load_records(root)
    findings = [f"census: {problem}" for problem in problems]
    packages = workspace_packages(root)
    sources = Sources(root)
    unnamed: set[str] = set()
    held: dict[str, Record] = {}
    for record in records:
        if record.klass == "rust" and record.package not in packages:
            if record.fragment not in unnamed:
                unnamed.add(record.fragment)
                findings.append(
                    f"census: {record.fragment}: named for no package of the workspace, nor "
                    f"{MINIAPP}"
                )
            continue
        crate = packages.get(record.package)
        for problem in record_problems(root, record, crate, sources):
            findings.append(f"census: {record.fragment}: {record.named}: {problem}")
        key = json.dumps([record.fields.get(name) for name in ("file", "mutant", "anchor", "span")])
        if key in held:
            findings.append(
                f"census: {record.fragment}: {record.named}: held twice, as record "
                f"{held[key].index}"
            )
        else:
            held[key] = record
    for finding in findings:
        print(finding)
    print(f"examined {len(records)} record(s)")
    return EXIT_FAIL if findings else EXIT_OK


# --------------------------------------------------------------------------- the table


@dataclass
class Tally:
    """One package's row of the campaign's table (SPEC-057 section 7)."""

    listed: int | None = None
    killed: int = 0
    equivalent: int = 0
    unexplained: int = 0
    unviable: int = 0

    def line(self, package: str) -> str:
        counted = self.killed + self.equivalent + self.unexplained + self.unviable
        listed = counted if self.listed is None else self.listed
        return (
            f"table: {package}: listed {listed}, killed {self.killed}, equivalent "
            f"{self.equivalent}, unexplained {self.unexplained}, unviable {self.unviable}"
        )


def table(args: argparse.Namespace) -> int:
    """R13: one line per package of a battery's reports, in section 7's columns."""
    root, reports, scope = (
        pathlib.Path(args.root),
        pathlib.Path(args.reports),
        args.package,
    )
    findings: list[str] = []
    voids: list[str] = []

    def fail(text: str) -> None:
        findings.append(text)
        print(f"table: {text}")

    def void(text: str) -> None:
        voids.append(text)
        print(f"table: VOID {text}")

    records, problems = load_records(root)
    for problem in problems:
        fail(f"the record: {problem}")
    tallies: dict[str, Tally] = defaultdict(Tally)
    read = 0
    if scope != MINIAPP:
        read += table_rust(root, reports, scope, records, args.listed, tallies, fail, void)
    if scope in (None, MINIAPP):
        read += table_web(root, reports, records, tallies[MINIAPP], fail, void)
    if scope is not None and scope not in tallies:
        void(f"no listing or report holds a mutant of {scope}")
    if voids:
        print(f"table: verdict: VOID: {len(voids)} measurement(s) the table needs are missing")
    elif findings:
        print(f"table: verdict: FAIL: {len(findings)} finding(s)")
    else:
        print("table: verdict: ok")
    counted = sum(
        tally.killed + tally.equivalent + tally.unexplained + tally.unviable
        for tally in tallies.values()
    )
    print(f"examined {counted} mutant(s) in {read} report(s)")
    for package in sorted(tallies, key=lambda name: (name == MINIAPP, name)):
        print(tallies[package].line(package))
    if voids:
        return EXIT_VOID
    return EXIT_FAIL if findings else EXIT_OK


def table_rust(root, reports, scope, records, listed_path, tallies, fail, void) -> int:
    """The Rust packages' rows: every shard's report read, each one missing or partial VOID by
    name, and with a listing, each listed mutant no report tested VOID by name. Returns the
    reports read."""
    shards = {path.parent for path in reports.rglob("cargo-mutants.exit")}
    shards |= {path.parent.parent for path in reports.rglob("outcomes.json")}
    outcomes: list[tuple[str, dict]] = []
    stopped: set[tuple[str, str]] = set()
    read = 0
    for directory in sorted(shards):
        where = directory.relative_to(reports).as_posix()
        report = read_json(str(directory / "mutants.out" / "outcomes.json"))
        code = read_exit(directory / "cargo-mutants.exit")
        if report is None and code == 0:
            continue
        read += 1
        reason = partial_reason(report, code, "its outcomes.json")
        if reason is not None:
            void(f"{where}: {reason}")
        stopped |= {
            (where, name)
            for name in memory_cap(
                fail,
                void,
                lambda text: print(f"table: {text}"),
                f"{where}: ",
                directory,
                report if reason is None else None,
            )
        }
        if isinstance(report, dict):
            outcomes += [(where, outcome) for outcome in report.get("outcomes", [])]

    def in_scope(package: object) -> bool:
        return scope is None or package == scope

    excuses = Excuses(root, records, "rust")
    listing = read_json(listed_path) if listed_path else None
    listed = (
        [entry for entry in listing if isinstance(entry, dict)] if isinstance(listing, list) else []
    )
    if listed_path and not isinstance(listing, list):
        void(f"{listed_path} holds no cargo-mutants listing")
    if isinstance(listing, list):
        # Every record, of every package, against the whole tree's listing (R8, R12).
        excuses.bind([mutant for mutant in map(cargo_mutant, listed) if mutant], fail)
    else:
        tested = [cargo_mutant(outcome_mutant(outcome)) for _, outcome in outcomes]
        mine = [record for record in excuses.records if in_scope(record.package)]
        excuses.bind([mutant for mutant in tested if mutant], fail, records=mine)
    for entry in listed:
        if in_scope(entry.get("package")):
            tally = tallies[str(entry.get("package"))]
            tally.listed = (tally.listed or 0) + 1
    seen: Counter = Counter()
    for where, outcome in outcomes:
        entry = outcome_mutant(outcome)
        if entry is None or not in_scope(entry.get("package")):
            continue
        package, name, summary = (
            str(entry.get("package")),
            entry.get("name"),
            outcome.get("summary"),
        )
        seen[name] += 1
        if (where, name) in stopped:
            continue
        tally = tallies[package]
        excused = excuses.of(cargo_mutant(entry))
        if summary == "MissedMutant":
            if len(excused) == 1:
                tally.equivalent += 1
                print(f"table: {package}: EQUIVALENT {name}: {excuse_line(excused[0])}")
            else:
                tally.unexplained += 1
                fail(f"{package}: UNEXPLAINED {name}{held_twice(excused)}")
            continue
        if summary in ("CaughtMutant", "Timeout"):
            tally.killed += 1
        elif summary == "Unviable":
            tally.unviable += 1
        else:
            void(f"{where}: {name}: an outcome this table does not know, {summary}")
            continue
        for record in excused:
            fail(f"{package}: {refutation(record, name, summary)}")
    if isinstance(listing, list):
        times = Counter(entry.get("name") for entry in listed if in_scope(entry.get("package")))
        for name in sorted(set(times) | set(seen), key=str):
            if seen[name] < times[name]:
                void(f"never tested: {name}")
            elif seen[name] > times[name]:
                fail(f"{name}: tested {seen[name]} time(s), listed {times[name]} time(s)")
    return read


def table_web(root, reports, records, tally, fail, void) -> int:
    """The Mini App's row, from the sweep's one Stryker report: a survived mutant one record binds
    is equivalent; a survived mutant with none, an uncovered one and an ignored one are unexplained
    (R10). Returns the reports read."""
    found = sorted(reports.rglob("mutation.json"))
    document = read_json(str(found[0])) if len(found) == 1 else None
    if not isinstance(document, dict) or not isinstance(document.get("files"), dict):
        void(f"stryker: {len(found)} mutation.json, not one whole report")
        return 0
    excuses = Excuses(root, records, "web")
    swept = set()
    for path, entry in sorted(document["files"].items()):
        file = f"{WEB_ROOT}{path}"
        swept.add(file)
        if isinstance(entry.get("source"), str):
            excuses.sources.put(file, entry["source"])
        mutants = [(mutant, stryker_mutant(file, mutant)) for mutant in entry.get("mutants", [])]
        mine = [record for record in excuses.records if record.get("file") == file]
        bound = [mutant for _, mutant in mutants if mutant]
        excuses.bind(bound, fail, records=mine, noun=f"mutant of {file}")
        for mutant, parsed in mutants:
            status = mutant.get("status", "Pending")
            where = f"{file}:{mutant.get('location', {}).get('start', {}).get('line')}"
            what = f"{mutant.get('mutatorName')} -> {mutant.get('replacement')!r}"
            excused = excuses.of(parsed)
            if status == "Survived" and len(excused) == 1:
                tally.equivalent += 1
                print(f"table: {MINIAPP}: EQUIVALENT {where}: {what}: {excuse_line(excused[0])}")
                continue
            if status in ("Survived", "NoCoverage", "Ignored"):
                tally.unexplained += 1
                fail(f"{MINIAPP}: UNEXPLAINED {where}: {status}: {what}{held_twice(excused)}")
            elif status in ("Killed", "Timeout"):
                tally.killed += 1
            elif status in ("CompileError", "RuntimeError"):
                tally.unviable += 1
            else:
                void(f"{where}: {status}: never run")
                continue
            for record in excused if status != "Survived" else []:
                refuted = refutation(record, f"{where}: {what}", status)
                if refuted is not None:
                    fail(f"{MINIAPP}: {refuted}")
    for record in excuses.records:
        if record.get("file") not in swept:
            fail(f"STALE {record}: the sweep mutated no such file")
    return 1


# --------------------------------------------------------------------------- the command line


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "verb",
        choices=[
            "plan",
            "shards",
            "size",
            "judge",
            "survivors",
            "battery",
            "configs",
            "exclusions",
            "census",
            "table",
        ],
    )
    parser.add_argument("--root", default=str(pathlib.Path(__file__).resolve().parents[1]))
    parser.add_argument("--base")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--out")
    parser.add_argument("--plan")
    parser.add_argument("--class", dest="klass", choices=["rust", "web", "oracle"])
    parser.add_argument("--outcomes")
    parser.add_argument("--tool-exit", type=int)
    parser.add_argument("--stryker")
    parser.add_argument("--rows")
    parser.add_argument("--reports")
    parser.add_argument("--open-titles")
    parser.add_argument("--event", default="")
    parser.add_argument("--base-ref", default="")
    parser.add_argument("--subject", default="")
    parser.add_argument("--shards", type=int)
    parser.add_argument("--listed")
    parser.add_argument("--shard-reports")
    parser.add_argument("--whole")
    parser.add_argument("--package")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()
    if args.verb == "plan":
        if not args.base or not args.out:
            parser.error("plan needs --base and --out")
        scope = scope_of(args.event, args.base_ref, args.subject)
        try:
            plan = plan_diff(root, args.base, args.head, pathlib.Path(args.out), scope)
        except mutation_rows.PopulationRefused as refusal:
            print(f"mutation: plan: REFUSED: {refusal}", file=sys.stderr)
            return EXIT_FAIL
        say_plan(plan)
        return EXIT_OK
    if args.verb == "shards":
        if not args.plan:
            parser.error("shards needs --plan")
        return shards(pathlib.Path(args.plan), args.listed)
    if args.verb == "size":
        return size(args.listed, args.package)
    if args.verb == "judge":
        if not args.plan or not args.klass:
            parser.error("judge needs --plan and --class")
        return judge(args)
    if args.verb == "survivors":
        if not args.reports or not args.out:
            parser.error("survivors needs --reports and --out")
        return survivors(args)
    if args.verb == "battery":
        if not args.reports or args.shards is None or args.shards < 1:
            parser.error("battery needs --reports and --shards, at least 1")
        return battery(pathlib.Path(args.reports), args.shards, args.package, args.listed)
    if args.verb == "configs":
        return configs(root)
    if args.verb == "census":
        return census(root)
    if args.verb == "table":
        if not args.reports:
            parser.error("table needs --reports")
        return table(args)
    return exclusions(root)


if __name__ == "__main__":
    sys.exit(main())

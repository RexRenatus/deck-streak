#!/usr/bin/env python3
"""mutation-verdict: a pull request's mutation plan and verdict, the weekly battery's survivors as
issue drafts and its count of reports, and the tools' configurations and exclusions (SPEC-039 R2 to
R6, R10, R12; ADR-057).

    python3 scripts/mutation-verdict.py plan --base REF [--head REF] [--root DIR] --out DIR
                                            [--event E --base-ref B --subject S]
    python3 scripts/mutation-verdict.py judge --plan FILE --class rust|web|oracle
                                             [--outcomes FILE] [--tool-exit N]
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
its class (R2), each production file's changed lines split into code lines and blank or comment
lines, the rows the diff selects (R10), and the web files Stryker mutates whole. Under GitHub
Actions it writes the step outputs `scope`, `rust`, `web`, `oracle`, `rows` and `mutate`.

JUDGE reads a tool's own report, never its exit alone (R4). Examined is caught plus missed plus
timed out (Stryker: killed, survived, no coverage and timed out); an unviable mutant, a compile or
runtime error, is not examined. A missed or uncovered mutant, or a selected row that was not
KILLED, fails (exit 1). A class whose production files changed a code line and whose examined
count, the tool's and the rows' on its changed lines, is zero is VOID (exit 3). So is a missing
report, and a partial one: cargo-mutants writes its report as it goes, so a report is read only when
the tool's exit is 0, 2 or 3 and its caught, missed, timed-out and unviable counts sum to its total.
A class whose changed lines are all blank or comments, or only deletions, reads `not-applicable` by
name, with its count.

SURVIVORS turns the weekly battery's reports into one issue draft per file, titled
`Mutation survivors: <path>`, and marks a draft whose title is already an open issue (R12). The
workflow scrubs the drafts with `scripts/public-scrub.py` before it files any.

BATTERY counts the reports the battery's jobs promise: each of N shards' `outcomes.json`, whole by
the rule JUDGE applies, the rows' report and the Stryker sweep. It names each report that is missing
or partial and fails (exit 1), so a runner shut down mid-run never reads as a shard with no
survivor.

CONFIGS judges `.cargo/mutants.toml` and `web/app/stryker.config.json` by what each tool itself
refuses (cargo-mutants 27.1.0 denies an unknown key or a mistyped value; StrykerJS refuses a
threshold out of 0 to 100, `high` below `low`, and JSON it cannot parse), and by what the verdict
needs of Stryker's: the Vitest runner and the json report.

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
from collections import defaultdict
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
REASON = re.compile(r"EQUIVALENT: \S.*\(#\d+\)")
#: The subject GitHub writes for a pull request's merge commit.
MERGE_SUBJECT = re.compile(r"Merge pull request #(\d+) from ")


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
        if merged:
            return (
                "not-applicable",
                f"this push merges #{merged.group(1)}, whose mutation jobs judged this tree on its "
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
            record["code"] = [n for n in entry["added"] if n not in quiet]
            record["quiet"] = [n for n in entry["added"] if n in quiet]
        else:
            record["code"], record["quiet"] = [], list(entry["added"])
        plan.files.append(record)
        if klass != "other":
            classes[klass]["files"].append(path)
            if record["code"]:
                classes[klass]["applies"] = True
                if klass == "web":
                    plan.stryker_mutate.append(path[len(WEB_ROOT) :])
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
        print(f"mutation: plan: {name} {'applies' if klass['applies'] else 'does not apply'}")
    print(f"mutation: plan: {len(plan.rows)} row(s) selected: {' '.join(plan.rows) or 'none'}")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as sink:
            for name, klass in plan.classes.items():
                sink.write(f"{name}={'true' if klass['applies'] else 'false'}\n")
            sink.write(f"rows={'true' if plan.rows else 'false'}\n")
            sink.write(f"scope={plan.scope['decision']}\n")
            sink.write(f"mutate={','.join(plan.stryker_mutate)}\n")


# --------------------------------------------------------------------------- the shards

#: Seconds one mutant costs, its build and its tests, by package: the mean over the weekly
#: battery's shards on GitHub's ubuntu-24.04 runners (R18). A package the table does not name costs
#: the table's highest.
SECONDS_PER_MUTANT = {
    "deck-streak-ingest": 124,
    "deck-streak-daemon": 84,
    "deck-streak-api": 51,
    "deck-streak-coordination": 40,
    "deck-streak-kernel": 12,
    "deck-streak-identity": 8,
    "deck-streak-vault": 7,
}
#: The unmutated baseline each shard builds and tests before its first mutant: the mean over the
#: 30 shards of run 36373915578.
BASELINE_SECONDS = 346
#: A shard's projected time may reach an hour, half its job's timeout-minutes of 120: measured
#: shards took from 0.72 to 1.39 times their projection (R18).
SHARD_BOUND_SECONDS = 3600
#: The most jobs a matrix may generate in one workflow run (GitHub's workflow syntax).
MAX_SHARDS = 256


def shards(plan_path: pathlib.Path, listed_path: str | None) -> int:
    """The red-first stub: it accepts the plan and the listing and sizes nothing."""
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
            self.say("verdict: VOID: the class applies and nothing was measured")
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
        if entry["added"]:
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
    report = read_json(args.outcomes)
    if not isinstance(report, dict) or "caught" not in report:
        verdict.void(
            f"no report: {args.outcomes or 'no --outcomes'} holds no cargo-mutants outcomes"
        )
        return
    if args.tool_exit not in WHOLE_EXITS:
        if args.tool_exit is None:
            verdict.void("no cargo-mutants exit recorded, so its report may be partial")
        else:
            meaning = TOOL_EXITS.get(args.tool_exit, "a run that stopped before its last mutant")
            verdict.void(f"cargo-mutants exit {args.tool_exit}: {meaning}")
        return
    caught, missed = int(report.get("caught", 0)), int(report.get("missed", 0))
    timeout, unviable = int(report.get("timeout", 0)), int(report.get("unviable", 0))
    total = int(report.get("total_mutants", 0))
    if caught + missed + timeout + unviable != total:
        # The tool writes its report as it goes: counts short of its total are a partial run.
        verdict.void(
            f"the report counts {caught + missed + timeout + unviable} of {total} mutants reported"
        )
        return
    tool = caught + missed + timeout
    verdict.say(
        f"cargo-mutants examined {tool} (caught {caught}, missed {missed}, timeout {timeout}), "
        f"unviable {unviable}, of {report.get('total_mutants', 0)} on the diff"
    )
    for outcome in report.get("outcomes", []):
        scenario = outcome.get("scenario")
        if outcome.get("summary") == "MissedMutant" and isinstance(scenario, dict):
            verdict.fail(f"MISSED {scenario.get('Mutant', {}).get('name', '<unnamed mutant>')}")
    if missed and not any(text.startswith("MISSED") for text in verdict.failures):
        verdict.fail(f"MISSED {missed} mutant(s), unnamed in the report")
    verdict.examined = tool + carried
    verdict.say(f"examined {tool} by cargo-mutants and {carried} by rows")
    touched = {mutated_file(o) for o in report.get("outcomes", [])} - {None}
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
    counts: dict[str, int] = defaultdict(int)
    for path, entry in sorted(report["files"].items()):
        for mutant in entry.get("mutants", []):
            status = mutant.get("status", "Pending")
            counts[status] += 1
            where = f"{WEB_ROOT}{path}:{mutant.get('location', {}).get('start', {}).get('line')}"
            what = f"{mutant.get('mutatorName')} -> {mutant.get('replacement')!r}"
            if status in ("Survived", "NoCoverage"):
                verdict.fail(f"{status} {where}: {what}")
            elif status == "Ignored" and not REASON.search(str(mutant.get("statusReason", ""))):
                verdict.fail(f"Ignored {where} with no EQUIVALENT reason and issue: {what}")
            elif status == "Pending":
                verdict.void(f"Pending {where}: never run")
    verdict.examined = sum(counts[status] for status in STRYKER_EXAMINED)
    summary = ", ".join(f"{status} {counts[status]}" for status in sorted(counts))
    verdict.say(f"Stryker examined {verdict.examined} ({summary or 'no mutant'})")
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


def survivors_in(reports: pathlib.Path) -> dict[str, list[str]]:
    """{file: [each survivor, one line]} across every report under `reports`."""
    found: dict[str, list[str]] = defaultdict(list)
    for path in sorted(reports.rglob("outcomes.json")):
        document = read_json(str(path)) or {}
        for outcome in document.get("outcomes", []):
            if outcome.get("summary") == "MissedMutant" and mutated_file(outcome):
                found[mutated_file(outcome)].append(outcome["scenario"]["Mutant"]["name"])
    for path in sorted(reports.rglob("mutation.json")):
        document = read_json(str(path)) or {}
        for name, entry in sorted((document.get("files") or {}).items()):
            for mutant in entry.get("mutants", []):
                if mutant.get("status") in ("Survived", "NoCoverage"):
                    line = mutant.get("location", {}).get("start", {}).get("line")
                    found[f"{WEB_ROOT}{name}"].append(
                        f"{WEB_ROOT}{name}:{line}: {mutant.get('status')}: "
                        f"{mutant.get('mutatorName')} -> {mutant.get('replacement')!r}"
                    )
    for path in sorted(reports.rglob("rows.json")):
        document = read_json(str(path)) or []
        for entry in document if isinstance(document, list) else []:
            if entry.get("verdict") in ("SURVIVED", "VOID"):
                found[entry.get("target", "<no target>")].append(
                    f"row {entry.get('id')}: {entry.get('verdict')}: {entry.get('reason', '')}"
                )
    return found


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
        "Triage each one (SPEC-039 R5): kill it with a test that asserts the behaviour the mutant "
        "breaks, or, when no test can tell it apart, record it as `EQUIVALENT: <reason> (#N)` in "
        "`.cargo/mutants.toml` or a `Stryker disable next-line` comment. A row that SURVIVED or is "
        "VOID is repaired in its band fragment under `scripts/mutation-rows.d/`.\n"
    )


def survivors(args: argparse.Namespace) -> int:
    reports, out = pathlib.Path(args.reports), pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    open_titles = set(read_json(args.open_titles) or [])
    manifest = []
    for number, (path, lines) in enumerate(sorted(survivors_in(reports).items()), start=1):
        name = f"draft-{number:03d}.md"
        (out / name).write_text(draft_body(path, lines), encoding="utf-8")
        title = TITLE.format(path)
        manifest.append({"title": title, "body": name, "open": title in open_titles})
        state = "already open, not filed again" if title in open_titles else "to file"
        print(f"survivors: {title}: {len(lines)} mutant(s): {state}")
    (out / "drafts.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"examined {len(manifest)} file(s) with survivors")
    return EXIT_OK


def read_exit(path: pathlib.Path) -> int | None:
    """The exit a job recorded beside its report, or None when it recorded none."""
    try:
        return int(path.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        return None


def battery(reports: pathlib.Path, shards: int) -> int:
    """Every report the weekly battery's jobs promise, counted whole (R12): each shard's
    `outcomes.json` with an exit of 0, 2 or 3 and counts that sum to its total, the rows' report
    with at least one row proved, and the Stryker sweep's `mutation.json`. A runner shut down
    mid-run uploads nothing, and a shard stopped early leaves a partial report: either is named, and
    fails the battery, rather than read as a shard with no survivor."""
    findings: list[str] = []
    whole = 0
    for shard in range(shards):
        name = f"mutants-shard-{shard}"
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
    rows = read_json(str(reports / "rows" / "rows.json"))
    if rows is None:
        findings.append("battery: MISSING rows: no rows.json")
    elif not isinstance(rows, list) or not rows:
        findings.append("battery: PARTIAL rows: rows.json proves no row")
    else:
        whole += 1
    sweep = reports / "stryker"
    found = sorted(sweep.rglob("mutation.json")) if sweep.is_dir() else []
    documents = [read_json(str(path)) for path in found]
    if not found:
        findings.append("battery: MISSING stryker: no mutation.json")
    elif len(found) > 1 or not isinstance(documents[0], dict) or "files" not in documents[0]:
        findings.append(f"battery: PARTIAL stryker: {len(found)} mutation.json, not one report")
    else:
        whole += 1
    for finding in findings:
        print(finding)
    print(f"battery: counted {whole} of {shards + 2} reports whole")
    print(f"examined {shards + 2} report(s)")
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
    for finding in findings:
        print(finding)
    print(f"examined {examined} configuration(s)")
    if findings:
        return EXIT_FAIL
    return EXIT_OK if examined else EXIT_VOID


# --------------------------------------------------------------------------- the exclusions


def exclusions(root: pathlib.Path) -> int:
    findings, examined = [], 0
    config = root / ".cargo" / "mutants.toml"
    if config.is_file():
        text = config.read_text(encoding="utf-8")
        lines = text.splitlines()
        document = tomllib.loads(text)
        for key in ("exclude_re", "exclude_globs", "skip_calls"):
            for entry in document.get(key, []):
                examined += 1
                literal = json.dumps(entry)[1:-1]
                numbers = [n for n, line in enumerate(lines) if literal in line or entry in line]
                justified = any(
                    REASON.search(lines[n]) or (n > 0 and REASON.search(lines[n - 1]))
                    for n in numbers
                )
                if not justified:
                    findings.append(
                        f"exclusions: .cargo/mutants.toml: {key} {entry!r} names no EQUIVALENT "
                        "reason and issue"
                    )
    web = root / "web" / "app" / "src"
    for path in sorted(web.rglob("*")) if web.is_dir() else []:
        if path.suffix not in (".ts", ".js", ".svelte") or not path.is_file():
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if "Stryker disable" in line:
                examined += 1
                if not REASON.search(line):
                    where = path.relative_to(root).as_posix()
                    findings.append(
                        f"exclusions: {where}:{number}: a Stryker disable names no EQUIVALENT "
                        "reason and issue"
                    )
    crates = root / "crates"
    for path in sorted(crates.glob("*/src/**/*.rs")) if crates.is_dir() else []:
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if "mutants::skip" in line and not line.strip().startswith("//"):
                examined += 1
                where = path.relative_to(root).as_posix()
                findings.append(
                    f"exclusions: {where}:{number}: mutants::skip skips every mutant of its item "
                    "(ADR-057 D6)"
                )
    for finding in findings:
        print(finding)
    print(f"examined {examined} exclusion(s)")
    return EXIT_FAIL if findings else EXIT_OK


# --------------------------------------------------------------------------- the command line


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "verb",
        choices=["plan", "shards", "judge", "survivors", "battery", "configs", "exclusions"],
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
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()
    if args.verb == "plan":
        if not args.base or not args.out:
            parser.error("plan needs --base and --out")
        scope = scope_of(args.event, args.base_ref, args.subject)
        plan = plan_diff(root, args.base, args.head, pathlib.Path(args.out), scope)
        say_plan(plan)
        return EXIT_OK
    if args.verb == "shards":
        if not args.plan:
            parser.error("shards needs --plan")
        return shards(pathlib.Path(args.plan), args.listed)
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
        return battery(pathlib.Path(args.reports), args.shards)
    if args.verb == "configs":
        return configs(root)
    return exclusions(root)


if __name__ == "__main__":
    sys.exit(main())

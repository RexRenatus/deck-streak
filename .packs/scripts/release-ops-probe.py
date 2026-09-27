#!/usr/bin/env python3
"""release-ops-probe: judge a repository's release path from its tree (SPEC-V2-2221).

A public repository with a `dev` branch and a protected `main` ships a release as a pull request from
`dev` into `main`, an annotated SemVer tag on `main`, a release published from that tag, and a deploy
that runs only from a tag on `main`, with a rollback to the previous tag. This check reads what the
tree says about each of those, and nothing else:

- `.github/rulesets/*.json`: the rulesets, declared in the JSON GitHub's REST API and its ruleset
  import accept (GitHub does not read this directory: the owner applies it);
- `.github/workflows/*.yml|*.yaml`, through a strict YAML subset parser that refuses anchors,
  aliases, tags and multiple documents by name rather than guessing at them;
- the Cargo and npm manifests, `CHANGELOG.md`, the changelog fragment directory, the release runbook
  and `deploy/`.

    python3 scripts/release-ops-probe.py --root R [--protected-branch B] [--default-branch D] check <class>
    python3 scripts/release-ops-probe.py list

It is standard-library Python (3.11+, for `tomllib`), fetches nothing, and calls no API. Every class
prints one line per finding, `<class>: <finding>`, then `examined N`. Exit 0 is green, 1 a finding,
2 a usage error, 3 VOID: nothing was examined, which is never a pass.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import sys
import tomllib
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, field
from pathlib import Path

EXIT_OK = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

RULE_TYPES = frozenset(
    {
        "creation",
        "update",
        "deletion",
        "required_linear_history",
        "merge_queue",
        "required_deployments",
        "required_signatures",
        "pull_request",
        "required_status_checks",
        "non_fast_forward",
        "commit_message_pattern",
        "commit_author_email_pattern",
        "committer_email_pattern",
        "branch_name_pattern",
        "tag_name_pattern",
        "workflows",
        "code_scanning",
        "code_quality",
        "code_coverage",
        "copilot_code_review",
        "license_compliance_scanning",
        "file_path_restriction",
        "max_file_path_length",
        "file_extension_restriction",
        "max_file_size",
    }
)
PULL_REQUEST_REQUIRED = (
    "dismiss_stale_reviews_on_push",
    "require_code_owner_review",
    "require_last_push_approval",
    "required_approving_review_count",
    "required_review_thread_resolution",
)
MERGE_METHODS = ("merge", "squash", "rebase")
#: GitHub Actions' own app id: a required check with this integration_id is a workflow job.
ACTIONS_APP_ID = 15368
KEEP_A_CHANGELOG = ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security")
TOWNCRIER_TYPES = ("feature", "bugfix", "doc", "removal", "misc")
BUMPS = ("major", "minor", "patch")
SEMVER = re.compile(
    r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?"
    r"(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"
)
RELEASE_ACTIONS = (
    "softprops/action-gh-release",
    "ncipollo/release-action",
    "actions/create-release",
)
GOOD_TAGS = ("v1.2.3", "1.2.3")
BAD_TAGS = ("vfoo", "v1", "v1.2", "v1.2.3.4", "latest", "nightly", "v1.2.3/x")
MOVING_LABELS = ("ubuntu-latest", "windows-latest", "macos-latest")
STANDARD_LABEL = re.compile(
    r"ubuntu-\d{2}\.\d{2}(?:-arm)?|windows-\d{4}|windows-11-arm|macos-\d{2}(?:-intel)?"
)
DEPLOY_SCRIPT = re.compile(r"(?:^|[\s;&|(])(?:\./|bash\s+|sh\s+)?deploy/[\w.-]+")
ALWAYS_IF = ("always()", "!cancelled()")
RUNBOOK_NAMES = (
    "RELEASING.md",
    "docs/RELEASING.md",
    "docs/releasing.md",
    ".github/RELEASING.md",
)
FRAGMENT_IGNORED = re.compile(
    r"(?i)^(?:readme(?:\.\w+)?|\.gitignore|\.gitkeep|.*\.template\..*|template\.\w+)$"
)
SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    "build",
    ".svelte-kit",
}
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(?P<hashes>#{1,6})\s+(?P<title>.+?)\s*#*\s*$")


@dataclass
class Outcome:
    """A class's verdict: its findings and the size of the population it read."""

    findings: list[str] = field(default_factory=list)
    examined: int = 0
    void: str | None = None

    def exit_code(self) -> int:
        if self.void is not None or self.examined == 0:
            return EXIT_VOID
        return EXIT_FINDING if self.findings else EXIT_OK


# --------------------------------------------------------------------------------------------
# A strict YAML subset: what GitHub workflow files use, and a refusal for what they should not.
# --------------------------------------------------------------------------------------------


class YamlError(ValueError):
    """The text is outside the subset this reader accepts; the message names the line."""


_PLAIN_KEY = r"[^\s#'\"\[\]{},&*!|>%@`-][^#]*?|-[^\s#][^#]*?"
_ENTRY = re.compile(
    r"^(?P<key>\"(?:[^\"\\]|\\.)*\"|'(?:[^']|'')*'|"
    + _PLAIN_KEY
    + r")\s*:(?:\s+(?P<rest>.*)|$)$"
)
_INT = re.compile(r"[-+]?\d+")


def _scalar(text: str) -> object:
    """A plain scalar under the YAML 1.2 core schema's booleans, nulls and integers. Other numbers
    stay strings, so a version such as 3.10 is never read as 3.1."""
    if text in ("true", "True", "TRUE"):
        return True
    if text in ("false", "False", "FALSE"):
        return False
    if text in ("", "~", "null", "Null", "NULL"):
        return None
    if _INT.fullmatch(text):
        return int(text)
    return text


def _strip_comment(text: str) -> str:
    """`text` without a trailing ` #` comment, keeping any `#` inside quotes or brackets."""
    quote = None
    depth = 0
    for index, char in enumerate(text):
        if quote:
            if char == quote:
                if quote == "'" and text[index + 1 : index + 2] == "'":
                    continue
                quote = None
            elif char == "\\" and quote == '"':
                continue
        elif char in "'\"" and (index == 0 or text[index - 1] in " \t[{,:"):
            quote = char
        elif char in "[{":
            depth += 1
        elif char in "]}":
            depth = max(0, depth - 1)
        elif char == "#" and (index == 0 or text[index - 1] in " \t"):
            return text[:index].rstrip()
    return text.rstrip()


def _double_quoted(body: str, number: int) -> str:
    escapes = {
        "n": "\n",
        "t": "\t",
        "r": "\r",
        '"': '"',
        "\\": "\\",
        "/": "/",
        "0": "\0",
        " ": " ",
    }
    out = []
    index = 0
    while index < len(body):
        char = body[index]
        if char == "\\":
            if index + 1 >= len(body):
                raise YamlError(
                    f"line {number}: a dangling escape in a double-quoted string"
                )
            code = body[index + 1]
            if code in escapes:
                out.append(escapes[code])
                index += 2
            elif code in "xuU":
                width = {"x": 2, "u": 4, "U": 8}[code]
                digits = body[index + 2 : index + 2 + width]
                if len(digits) != width or not re.fullmatch(r"[0-9a-fA-F]+", digits):
                    raise YamlError(f"line {number}: a bad \\{code} escape")
                out.append(chr(int(digits, 16)))
                index += 2 + width
            else:
                raise YamlError(f"line {number}: an unknown escape \\{code}")
        else:
            out.append(char)
            index += 1
    return "".join(out)


def _quoted(text: str, number: int) -> tuple[str, str]:
    """The quoted scalar at the start of `text` and what follows it."""
    quote = text[0]
    index = 1
    while index < len(text):
        char = text[index]
        if quote == '"' and char == "\\":
            index += 2
            continue
        if char == quote:
            if quote == "'" and text[index + 1 : index + 2] == "'":
                index += 2
                continue
            body = text[1:index]
            value = (
                body.replace("''", "'")
                if quote == "'"
                else _double_quoted(body, number)
            )
            return value, text[index + 1 :]
        index += 1
    raise YamlError(f"line {number}: an unterminated quoted string")


class _Flow:
    """A flow collection, `[a, 'b']` or `{a: b}`, read from one joined string."""

    def __init__(self, text: str, number: int) -> None:
        self.text = text
        self.index = 0
        self.number = number

    def error(self, why: str) -> YamlError:
        return YamlError(f"line {self.number}: {why} in a flow collection")

    def skip(self) -> None:
        while self.index < len(self.text) and self.text[self.index] in " \t\n":
            self.index += 1

    def value(self) -> object:
        self.skip()
        if self.index >= len(self.text):
            raise self.error("a missing value")
        char = self.text[self.index]
        if char == "[":
            return self.sequence()
        if char == "{":
            return self.mapping()
        if char in "&*!":
            raise self.error("an anchor, alias or tag")
        if char in "'\"":
            value, rest = _quoted(self.text[self.index :], self.number)
            self.index = len(self.text) - len(rest)
            return value
        start = self.index
        while self.index < len(self.text):
            here = self.text[self.index]
            if here in ",]}":
                break
            if here == ":" and self.text[self.index + 1 : self.index + 2] in (
                " ",
                ",",
                "]",
                "}",
                "",
            ):
                break
            self.index += 1
        return _scalar(self.text[start : self.index].strip())

    def sequence(self) -> list:
        self.index += 1
        items: list = []
        while True:
            self.skip()
            if self.index >= len(self.text):
                raise self.error("an unclosed [")
            if self.text[self.index] == "]":
                self.index += 1
                return items
            items.append(self.value())
            self.skip()
            if self.index < len(self.text) and self.text[self.index] == ",":
                self.index += 1
            elif self.index < len(self.text) and self.text[self.index] != "]":
                raise self.error("a missing comma")

    def mapping(self) -> dict:
        self.index += 1
        items: dict = {}
        while True:
            self.skip()
            if self.index >= len(self.text):
                raise self.error("an unclosed {")
            if self.text[self.index] == "}":
                self.index += 1
                return items
            key = self.value()
            self.skip()
            value: object = None
            if self.index < len(self.text) and self.text[self.index] == ":":
                self.index += 1
                self.skip()
                if self.index < len(self.text) and self.text[self.index] not in ",}":
                    value = self.value()
            if not isinstance(key, str):
                key = json.dumps(key)
            if key in items:
                raise self.error(f"the key {key!r} twice")
            items[key] = value
            self.skip()
            if self.index < len(self.text) and self.text[self.index] == ",":
                self.index += 1
            elif self.index < len(self.text) and self.text[self.index] != "}":
                raise self.error("a missing comma")

    def whole(self) -> object:
        value = self.value()
        self.skip()
        if self.index != len(self.text):
            raise self.error("text after the closing bracket")
        return value


class _Yaml:
    """Block YAML by indentation, over a list of lines it may rewrite (a `- key: value` item is read
    as a mapping one column further in)."""

    def __init__(self, text: str) -> None:
        self.lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
        self.index = 0

    @staticmethod
    def indent_of(line: str) -> int:
        return len(line) - len(line.lstrip(" "))

    def content(self, index: int) -> bool:
        stripped = self.lines[index].strip()
        return bool(stripped) and not stripped.startswith("#")

    def peek(self) -> int | None:
        index = self.index
        while index < len(self.lines) and not self.content(index):
            index += 1
        return index if index < len(self.lines) else None

    def check_line(self, index: int) -> None:
        line = self.lines[index]
        leading = line[: len(line) - len(line.lstrip(" \t"))]
        if "\t" in leading:
            raise YamlError(f"line {index + 1}: a tab in the indentation")

    def document(self) -> object:
        first = self.peek()
        if first is None:
            return None
        stripped = self.lines[first].strip()
        if stripped.startswith("%"):
            raise YamlError(f"line {first + 1}: a directive")
        if stripped == "---" or stripped.startswith("--- "):
            rest = stripped[3:].strip()
            if rest:
                raise YamlError(f"line {first + 1}: content on the document marker")
            self.index = first + 1
        value = self.block(0)
        tail = self.peek()
        if tail is not None:
            stripped = self.lines[tail].strip()
            if stripped in ("---", "..."):
                self.index = tail + 1
                if self.peek() is not None:
                    raise YamlError(f"line {tail + 1}: more than one document")
            else:
                raise YamlError(
                    f"line {tail + 1}: text outside the document's structure"
                )
        return value

    def block(self, minimum: int) -> object:
        at = self.peek()
        if at is None:
            return None
        self.check_line(at)
        indent = self.indent_of(self.lines[at])
        if indent < minimum:
            return None
        text = self.lines[at].strip()
        if text == "-" or text.startswith("- "):
            return self.sequence(indent)
        if _ENTRY.match(_strip_comment(text)):
            return self.mapping(indent)
        self.index = at
        return self.inline(text, indent - 1, at)

    def mapping(self, indent: int) -> dict:
        result: dict = {}
        while True:
            at = self.peek()
            if at is None:
                return result
            self.check_line(at)
            here = self.indent_of(self.lines[at])
            text = self.lines[at].strip()
            if here < indent:
                return result
            if here > indent:
                raise YamlError(f"line {at + 1}: unexpected indentation")
            if text == "-" or text.startswith("- ") or text in ("---", "..."):
                return result
            match = _ENTRY.match(_strip_comment(text))
            if match is None:
                raise YamlError(f"line {at + 1}: not a key: value entry")
            raw_key = match.group("key").strip()
            if raw_key.startswith("<<"):
                raise YamlError(f"line {at + 1}: a merge key")
            if raw_key[0] in "'\"":
                key, _ = _quoted(raw_key, at + 1)
            else:
                key = raw_key
            if key in result:
                raise YamlError(f"line {at + 1}: the key {key!r} twice")
            rest = (match.group("rest") or "").strip()
            self.index = at + 1
            result[key] = self.value_after(rest, indent, at)

    def sequence(self, indent: int) -> list:
        items: list = []
        while True:
            at = self.peek()
            if at is None:
                return items
            self.check_line(at)
            here = self.indent_of(self.lines[at])
            text = self.lines[at].strip()
            if here != indent or not (text == "-" or text.startswith("- ")):
                if here > indent:
                    raise YamlError(f"line {at + 1}: unexpected indentation")
                return items
            rest = text[1:].lstrip(" ")
            offset = here + (len(text) - len(rest))
            if not rest or rest.startswith("#"):
                self.index = at + 1
                nxt = self.peek()
                if nxt is not None and self.indent_of(self.lines[nxt]) > indent:
                    items.append(self.block(indent + 1))
                else:
                    items.append(None)
                continue
            stripped = _strip_comment(rest)
            if (
                stripped == "-"
                or stripped.startswith("- ")
                or (_ENTRY.match(stripped) and stripped[0] not in "'\"[{")
            ):
                self.lines[at] = " " * offset + rest
                self.index = at
                items.append(self.block(offset))
                continue
            if stripped[:1] in "'\"" and _ENTRY.match(stripped):
                self.lines[at] = " " * offset + rest
                self.index = at
                items.append(self.block(offset))
                continue
            self.index = at + 1
            items.append(self.value_after(rest, indent, at))

    def value_after(self, rest: str, indent: int, at: int) -> object:
        """The value of an entry or item whose own line holds `rest`, at `indent`."""
        text = _strip_comment(rest)
        if not text:
            nxt = self.peek()
            if nxt is None:
                return None
            self.check_line(nxt)
            below = self.indent_of(self.lines[nxt])
            stripped = self.lines[nxt].strip()
            if below > indent:
                return self.block(indent + 1)
            if below == indent and (stripped == "-" or stripped.startswith("- ")):
                return self.sequence(indent)
            return None
        if text[0] in "&*!":
            raise YamlError(f"line {at + 1}: an anchor, alias or tag")
        if text[0] in "|>":
            return self.block_scalar(text, indent, at)
        return self.inline(text, indent, at)

    def inline(self, text: str, indent: int, at: int) -> object:
        """A value that starts on its own line: quoted, flow, or plain, perhaps continued below."""
        if text[0] in "[{":
            joined = text
            while True:
                try:
                    return _Flow(joined, at + 1).whole()
                except YamlError as error:
                    if "unclosed" not in str(error):
                        raise
                nxt = self.peek()
                if nxt is None:
                    raise YamlError(f"line {at + 1}: an unclosed flow collection")
                joined += " " + _strip_comment(self.lines[nxt].strip())
                self.index = nxt + 1
        if text[0] in "'\"":
            joined = text
            while True:
                try:
                    value, after = _quoted(joined, at + 1)
                    if _strip_comment(after).strip():
                        raise YamlError(f"line {at + 1}: text after a quoted scalar")
                    return value
                except YamlError as error:
                    if "unterminated" not in str(error):
                        raise
                nxt = self.index if self.index < len(self.lines) else None
                if nxt is None:
                    raise
                line = self.lines[nxt].strip()
                joined += ("\n" if not line else " ") + line
                self.index = nxt + 1
        parts = [text]
        while True:
            nxt = self.peek()
            if nxt is None or self.indent_of(self.lines[nxt]) <= indent:
                break
            line = _strip_comment(self.lines[nxt].strip())
            if _ENTRY.match(line) or line.startswith("- "):
                break
            parts.append(line)
            self.index = nxt + 1
        return _scalar(" ".join(parts))

    def block_scalar(self, header: str, indent: int, at: int) -> str:
        match = re.fullmatch(r"([|>])([-+]?)(\d?)([-+]?)", header)
        if match is None:
            raise YamlError(f"line {at + 1}: a bad block scalar header {header!r}")
        style, chomp = match.group(1), match.group(2) or match.group(4)
        width = int(match.group(3)) if match.group(3) else 0
        lines: list[str] = []
        block_indent = indent + width if width else None
        index = self.index
        while index < len(self.lines):
            line = self.lines[index]
            if not line.strip():
                lines.append("")
                index += 1
                continue
            here = self.indent_of(line)
            if block_indent is None:
                if here <= indent:
                    break
                block_indent = here
            if here < block_indent:
                break
            lines.append(line[block_indent:])
            index += 1
        self.index = index
        while lines and lines[-1] == "" and chomp != "+":
            lines.pop()
        if style == "|":
            body = "\n".join(lines)
        else:
            folded: list[str] = []
            for line in lines:
                if (
                    folded
                    and line
                    and not line.startswith(" ")
                    and folded[-1]
                    and not folded[-1].startswith(" ")
                ):
                    folded[-1] += " " + line
                else:
                    folded.append(line)
            body = "\n".join(folded)
        return body if chomp == "-" else body + "\n"


def parse_yaml(text: str) -> object:
    """Parse `text` as the strict YAML subset, or raise YamlError naming the line it refused."""
    return _Yaml(text).document()


# --------------------------------------------------------------------------------------------
# Patterns.
# --------------------------------------------------------------------------------------------


def filter_regex(pattern: str) -> re.Pattern[str]:
    """GitHub's workflow filter pattern as a regular expression: `*` stops at `/`, `**` does not,
    `?` and `+` quantify the character before them, `[]` is a class, `\\` escapes."""
    out: list[str] = []
    index = 0
    while index < len(pattern):
        char = pattern[index]
        if char == "\\" and index + 1 < len(pattern):
            out.append(re.escape(pattern[index + 1]))
            index += 2
        elif char == "*":
            if pattern.startswith("**", index):
                out.append(".*")
                index += 2
            else:
                out.append("[^/]*")
                index += 1
        elif char in "?+" and out and out[-1] not in (".*", "[^/]*"):
            out[-1] = f"(?:{out[-1]}){char}"
            index += 1
        elif char == "[":
            close = pattern.find("]", index + 1)
            if close == -1:
                out.append(re.escape(char))
                index += 1
            else:
                out.append("[" + pattern[index + 1 : close].replace("\\", "\\\\") + "]")
                index = close + 1
        else:
            out.append(re.escape(char))
            index += 1
    return re.compile("".join(out) + r"\Z")


def filter_admits(patterns: Sequence[str], name: str) -> bool:
    """Whether a `branches`/`tags` filter admits `name`: patterns in order, a `!` pattern excluding
    what an earlier positive one admitted."""
    admitted = False
    for pattern in patterns:
        if pattern.startswith("!"):
            if filter_regex(pattern[1:]).match(name):
                admitted = False
        elif filter_regex(pattern).match(name):
            admitted = True
    return admitted


def fnmatch_regex(pattern: str) -> re.Pattern[str]:
    """A ruleset's ref pattern: `**` matches anything, `*` and `?` stop at `/`."""
    out: list[str] = []
    index = 0
    while index < len(pattern):
        char = pattern[index]
        if pattern.startswith("**", index):
            out.append(".*")
            index += 2
        elif char == "*":
            out.append("[^/]*")
            index += 1
        elif char == "?":
            out.append("[^/]")
            index += 1
        elif char == "[":
            close = pattern.find("]", index + 1)
            if close == -1:
                out.append(re.escape(char))
                index += 1
            else:
                out.append("[" + pattern[index + 1 : close] + "]")
                index = close + 1
        else:
            out.append(re.escape(char))
            index += 1
    return re.compile("".join(out) + r"\Z")


# --------------------------------------------------------------------------------------------
# The tree.
# --------------------------------------------------------------------------------------------


@dataclass
class Ruleset:
    path: str
    document: dict | None
    error: str | None

    @property
    def target(self) -> str:
        value = self.document.get("target", "branch") if self.document else "branch"
        return value if isinstance(value, str) else "branch"

    @property
    def enforcement(self) -> object:
        return self.document.get("enforcement") if self.document else None

    def refs(self, key: str) -> list[str]:
        conditions = self.document.get("conditions") if self.document else None
        ref_name = conditions.get("ref_name") if isinstance(conditions, dict) else None
        value = ref_name.get(key) if isinstance(ref_name, dict) else None
        return (
            [item for item in value if isinstance(item, str)]
            if isinstance(value, list)
            else []
        )

    def rules(self) -> list[dict]:
        value = self.document.get("rules") if self.document else None
        return (
            [rule for rule in value if isinstance(rule, dict)]
            if isinstance(value, list)
            else []
        )

    def covers(self, ref: str, default_branch: str) -> bool:
        def one(pattern: str) -> bool:
            if pattern == "~ALL":
                return True
            if pattern == "~DEFAULT_BRANCH":
                return ref == f"refs/heads/{default_branch}"
            return bool(fnmatch_regex(pattern).match(ref))

        return any(one(p) for p in self.refs("include")) and not any(
            one(p) for p in self.refs("exclude")
        )


@dataclass
class Workflow:
    path: str
    document: dict | None
    error: str | None

    def events(self) -> dict[str, object]:
        if not self.document:
            return {}
        value = self.document.get("on", self.document.get(True))
        if isinstance(value, str):
            return {value: None}
        if isinstance(value, list):
            return {item: None for item in value if isinstance(item, str)}
        return dict(value) if isinstance(value, dict) else {}

    def jobs(self) -> dict[str, dict]:
        value = self.document.get("jobs") if self.document else None
        return (
            {key: job for key, job in value.items() if isinstance(job, dict)}
            if isinstance(value, dict)
            else {}
        )


@dataclass
class Tree:
    root: Path
    protected: str
    default: str
    _rulesets: list[Ruleset] | None = None
    _workflows: list[Workflow] | None = None

    def display(self, path: Path) -> str:
        return path.relative_to(self.root).as_posix()

    def rulesets(self) -> list[Ruleset]:
        if self._rulesets is None:
            found = []
            directory = self.root / ".github" / "rulesets"
            for path in sorted(directory.glob("*.json")) if directory.is_dir() else []:
                try:
                    document = json.loads(path.read_text(encoding="utf-8"))
                except (OSError, UnicodeDecodeError, ValueError) as error:
                    found.append(
                        Ruleset(self.display(path), None, f"is not JSON: {error}")
                    )
                    continue
                if not isinstance(document, dict):
                    found.append(
                        Ruleset(self.display(path), None, "is not a JSON object")
                    )
                else:
                    found.append(Ruleset(self.display(path), document, None))
            self._rulesets = found
        return self._rulesets

    def workflows(self) -> list[Workflow]:
        if self._workflows is None:
            found = []
            directory = self.root / ".github" / "workflows"
            paths = sorted(
                path
                for path in (directory.iterdir() if directory.is_dir() else [])
                if path.is_file() and path.suffix in (".yml", ".yaml")
            )
            for path in paths:
                try:
                    document = parse_yaml(path.read_text(encoding="utf-8"))
                except (OSError, UnicodeDecodeError) as error:
                    found.append(
                        Workflow(self.display(path), None, f"cannot be read: {error}")
                    )
                    continue
                except YamlError as error:
                    found.append(
                        Workflow(
                            self.display(path), None, f"is not readable YAML: {error}"
                        )
                    )
                    continue
                if not isinstance(document, dict):
                    found.append(
                        Workflow(self.display(path), None, "is not a YAML mapping")
                    )
                else:
                    found.append(Workflow(self.display(path), document, None))
            self._workflows = found
        return self._workflows

    def files(self, name: str) -> list[Path]:
        """Every file called `name`, pruning build output and installed packages as it walks."""
        found = []
        for directory, subdirectories, names in os.walk(self.root):
            subdirectories[:] = sorted(d for d in subdirectories if d not in SKIP_DIRS)
            if name in names:
                found.append(Path(directory) / name)
        return sorted(found)


def main_rules(tree: Tree) -> tuple[list[Ruleset], list[dict]]:
    """The active branch rulesets that cover the protected branch, and their rules, aggregated as
    GitHub layers every ruleset that applies to a ref."""
    ref = f"refs/heads/{tree.protected}"
    applying = [
        ruleset
        for ruleset in tree.rulesets()
        if ruleset.document is not None
        and ruleset.target == "branch"
        and ruleset.enforcement == "active"
        and ruleset.covers(ref, tree.default)
    ]
    return applying, [rule for ruleset in applying for rule in ruleset.rules()]


def rules_of(rules: list[dict], kind: str) -> list[dict]:
    return [rule for rule in rules if rule.get("type") == kind]


def parameters(rule: dict) -> dict:
    value = rule.get("parameters")
    return value if isinstance(value, dict) else {}


def required_contexts(rules: list[dict]) -> list[str]:
    contexts: list[str] = []
    for rule in rules_of(rules, "required_status_checks"):
        checks = parameters(rule).get("required_status_checks")
        for check in checks if isinstance(checks, list) else []:
            if not isinstance(check, dict) or not isinstance(check.get("context"), str):
                continue
            # A check another GitHub App reports cannot be matched to a workflow job from the tree.
            if check.get("integration_id") not in (None, ACTIONS_APP_ID):
                continue
            if check["context"] not in contexts:
                contexts.append(check["context"])
    return contexts


def pull_request_config(workflow: Workflow) -> tuple[bool, dict]:
    """Whether the workflow runs on pull requests, and the first pull-request trigger's filters."""
    events = workflow.events()
    for event in ("pull_request", "pull_request_target"):
        if event in events:
            value = events[event]
            return True, value if isinstance(value, dict) else {}
    return False, {}


def strings(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [item for item in value if isinstance(item, str)]
    return []


def admits_branch(config: dict, branch: str) -> bool:
    if "branches" in config:
        return filter_admits(strings(config["branches"]), branch)
    if "branches-ignore" in config:
        return not any(
            filter_regex(p).match(branch) for p in strings(config["branches-ignore"])
        )
    return True


def check_name(job_id: str, job: dict) -> str | None:
    name = job.get("name")
    if name is None:
        return job_id
    if isinstance(name, str) and "${{" not in name:
        return name
    return None


def is_matrix(job: dict) -> bool:
    strategy = job.get("strategy")
    return isinstance(strategy, dict) and strategy.get("matrix") is not None


def resolve_context(tree: Tree, context: str) -> list[tuple[Workflow, str, dict, str]]:
    """Every job in a pull-request workflow whose check name could be `context`: (workflow, job id,
    job, how it matched)."""
    found = []
    for workflow in tree.workflows():
        runs, _ = pull_request_config(workflow)
        if not runs:
            continue
        for job_id, job in workflow.jobs().items():
            base = check_name(job_id, job)
            if base is None:
                continue
            if isinstance(job.get("uses"), str) and context.startswith(base + " / "):
                found.append((workflow, job_id, job, "reusable"))
            elif is_matrix(job):
                if context == base:
                    found.append((workflow, job_id, job, "bare-matrix"))
                elif context.startswith(base + " ("):
                    found.append((workflow, job_id, job, "matrix"))
            elif context == base:
                found.append((workflow, job_id, job, "exact"))
    return found


def steps(job: dict) -> list[dict]:
    value = job.get("steps")
    return (
        [step for step in value if isinstance(step, dict)]
        if isinstance(value, list)
        else []
    )


def step_texts(job: dict) -> list[str]:
    """Every string a job's steps carry: runs, envs, withs, conditions."""
    texts: list[str] = []

    def walk(value: object) -> None:
        if isinstance(value, str):
            texts.append(value)
        elif isinstance(value, dict):
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)

    walk(job.get("steps"))
    return texts


def runs(job: dict) -> list[str]:
    return [step["run"] for step in steps(job) if isinstance(step.get("run"), str)]


def uses(job: dict) -> list[str]:
    return [step["uses"] for step in steps(job) if isinstance(step.get("uses"), str)]


def normalised_if(value: object) -> str:
    text = str(value).strip()
    if text.startswith("${{") and text.endswith("}}"):
        text = text[3:-2]
    return re.sub(r"\s+", "", text)


def needs_of(job: dict) -> list[str]:
    return strings(job.get("needs"))


# --------------------------------------------------------------------------------------------
# Stage `protection`.
# --------------------------------------------------------------------------------------------


def unreadable_rulesets(tree: Tree, outcome: Outcome) -> None:
    for ruleset in tree.rulesets():
        if ruleset.error:
            outcome.findings.append(f"{ruleset.path} {ruleset.error}")


def check_main_ruleset(tree: Tree) -> Outcome:
    """An active branch ruleset covers the protected branch with deletion, non_fast_forward, a pull
    request and required status checks, every rule one the API accepts."""
    outcome = Outcome(examined=len(tree.rulesets()))
    if not tree.rulesets():
        outcome.void = "no ruleset declaration under .github/rulesets/"
        return outcome
    unreadable_rulesets(tree, outcome)
    ref = f"refs/heads/{tree.protected}"
    for ruleset in tree.rulesets():
        if ruleset.document is None:
            continue
        for rule in (
            ruleset.document.get("rules", [])
            if isinstance(ruleset.document.get("rules"), list)
            else []
        ):
            kind = rule.get("type") if isinstance(rule, dict) else None
            if kind not in RULE_TYPES:
                outcome.findings.append(
                    f"{ruleset.path} declares a rule type {kind!r} the API does not accept"
                )
            elif kind == "pull_request":
                missing = [
                    key for key in PULL_REQUEST_REQUIRED if key not in parameters(rule)
                ]
                if missing:
                    outcome.findings.append(
                        f"{ruleset.path}'s pull_request rule omits {', '.join(missing)}, which the API requires"
                    )
        if (
            ruleset.target == "branch"
            and ruleset.covers(ref, tree.default)
            and ruleset.enforcement != "active"
        ):
            outcome.findings.append(
                f"{ruleset.path} covers {tree.protected} with enforcement {ruleset.enforcement!r}; only active is enforced"
            )
    applying, rules = main_rules(tree)
    if not applying:
        bare = [
            r.path
            for r in tree.rulesets()
            if r.document is not None and tree.protected in r.refs("include")
        ]
        hint = (
            f" ({', '.join(bare)} names {tree.protected!r}, not refs/heads/{tree.protected})"
            if bare
            else ""
        )
        outcome.findings.append(
            f"no active branch ruleset covers refs/heads/{tree.protected}{hint}"
        )
        return outcome
    for kind in (
        "deletion",
        "non_fast_forward",
        "pull_request",
        "required_status_checks",
    ):
        if not rules_of(rules, kind):
            outcome.findings.append(f"{tree.protected} carries no {kind} rule")
    if rules_of(rules, "required_status_checks") and not required_contexts(rules):
        outcome.findings.append(
            f"{tree.protected}'s required_status_checks names no check"
        )
    return outcome


def check_release_merge_method(tree: Tree) -> Outcome:
    """The release pull request from the default branch merges into the protected one with a merge
    commit: squash or rebase would replay the long-running branch's commits on every release."""
    outcome = Outcome()
    applying, rules = main_rules(tree)
    outcome.examined = len(applying)
    if not applying:
        outcome.void = f"no active ruleset covers {tree.protected}"
        return outcome
    pull_requests = rules_of(rules, "pull_request")
    if not pull_requests:
        outcome.findings.append(
            f"{tree.protected} carries no pull_request rule to restrict the merge method"
        )
    allowed = set(MERGE_METHODS)
    for rule in pull_requests:
        methods = parameters(rule).get("allowed_merge_methods")
        if methods is None:
            continue
        allowed &= set(strings(methods))
    if pull_requests and allowed != {"merge"}:
        outcome.findings.append(
            f"{tree.protected} allows {', '.join(sorted(allowed)) or 'no method'}: set allowed_merge_methods to "
            f'["merge"] so the release pull request from {tree.default} keeps its commits'
        )
    if rules_of(rules, "required_linear_history"):
        outcome.findings.append(
            f"{tree.protected} requires linear history, which refuses the merge commit a release pull request needs"
        )
    return outcome


def check_required_checks_exist(tree: Tree) -> Outcome:
    """Every required context names a job that reports on pull requests into the protected branch."""
    outcome = Outcome()
    _, rules = main_rules(tree)
    contexts = required_contexts(rules)
    outcome.examined = len(contexts)
    if not contexts:
        outcome.void = f"no required status check is declared for {tree.protected}"
        return outcome
    for workflow in tree.workflows():
        if workflow.error:
            outcome.findings.append(f"{workflow.path} {workflow.error}")
    for context in contexts:
        matches = resolve_context(tree, context)
        if not matches:
            unnamed = [
                f"{w.path}:{job_id}"
                for w in tree.workflows()
                for job_id, job in w.jobs().items()
                if check_name(job_id, job) is None
            ]
            extra = (
                f" (the names of {', '.join(unnamed)} are expressions it cannot read)"
                if unnamed
                else ""
            )
            outcome.findings.append(
                f"required check {context!r} names no job of a pull-request workflow{extra}"
            )
            continue
        admitting = [
            m
            for m in matches
            if admits_branch(pull_request_config(m[0])[1], tree.protected)
        ]
        if not admitting:
            outcome.findings.append(
                f"required check {context!r} runs on no pull request into {tree.protected}: "
                f"{', '.join(sorted({m[0].path for m in matches}))} filters it out"
            )
            continue
        if all(kind == "bare-matrix" for _, _, _, kind in admitting):
            outcome.findings.append(
                f"required check {context!r} is a matrix job, which reports one check per combination "
                f"named '{context} (…)', never the bare name"
            )
    return outcome


def check_required_checks_not_skipped(tree: Tree) -> Outcome:
    """No required check can be skipped: a skipped workflow stays Pending, a job skipped by its
    condition reports Success, and a job whose need failed is skipped."""
    outcome = Outcome()
    _, rules = main_rules(tree)
    resolved = []
    for context in required_contexts(rules):
        for workflow, job_id, job, kind in resolve_context(tree, context):
            if kind != "bare-matrix" and admits_branch(
                pull_request_config(workflow)[1], tree.protected
            ):
                resolved.append((context, workflow, job_id, job))
    outcome.examined = len(resolved)
    if not resolved:
        outcome.void = "no required check resolves to a job"
        return outcome
    for context, workflow, job_id, job in resolved:
        _, config = pull_request_config(workflow)
        where = f"required check {context!r} ({workflow.path}:{job_id})"
        for key in ("paths", "paths-ignore"):
            if key in config:
                outcome.findings.append(
                    f"{where}: the workflow filters pull requests by {key}, so a pull request touching "
                    "other files leaves the check Pending"
                )
        types = strings(config.get("types")) if "types" in config else None
        if types is not None and "synchronize" not in types:
            outcome.findings.append(
                f"{where}: pull_request types omit synchronize, so a new push is never checked"
            )
        condition = job.get("if")
        if condition is not None and normalised_if(condition) not in ALWAYS_IF:
            outcome.findings.append(
                f"{where}: the job's if can skip it, and a job skipped by its condition reports Success"
            )
        if needs_of(job):
            if condition is None or normalised_if(condition) not in ALWAYS_IF:
                outcome.findings.append(
                    f"{where}: it needs {', '.join(needs_of(job))} without if: always(), so a failed need "
                    "skips it and the merge is not blocked"
                )
            elif not any(
                re.search(r"\bneeds\s*\.|toJSON\(\s*needs\s*\)", text)
                for text in step_texts(job)
            ):
                outcome.findings.append(
                    f"{where}: it runs always() but never reads needs.*.result, so it passes a failed need"
                )
    return outcome


def check_tag_ruleset(tree: Tree) -> Outcome:
    """An active tag ruleset covers the release tags and blocks their deletion and any update."""
    outcome = Outcome(examined=len(tree.rulesets()))
    if not tree.rulesets():
        outcome.void = "no ruleset declaration under .github/rulesets/"
        return outcome
    covering = [
        ruleset
        for ruleset in tree.rulesets()
        if ruleset.document is not None
        and ruleset.target == "tag"
        and ruleset.enforcement == "active"
        and any(ruleset.covers(f"refs/tags/{tag}", tree.default) for tag in GOOD_TAGS)
    ]
    if not covering:
        outcome.findings.append(
            "no active tag ruleset covers the release tags (refs/tags/v1.2.3)"
        )
        return outcome
    rules = [rule for ruleset in covering for rule in ruleset.rules()]
    for kind in ("deletion", "update"):
        if not rules_of(rules, kind):
            outcome.findings.append(
                f"the release tags carry no {kind} rule, so a published tag can be "
                f"{'deleted' if kind == 'deletion' else 'moved'}"
            )
    return outcome


def check_ruleset_bypass(tree: Tree) -> Outcome:
    """A ruleset guarding the protected branch or the release tags lets nobody bypass it at any time
    (advisory)."""
    outcome = Outcome()
    guarding, _ = main_rules(tree)
    guarding = guarding + [
        r for r in tree.rulesets() if r.document is not None and r.target == "tag"
    ]
    outcome.examined = len(guarding)
    if not guarding:
        outcome.void = "no ruleset guards the protected branch or the tags"
        return outcome
    for ruleset in guarding:
        actors = ruleset.document.get("bypass_actors") if ruleset.document else None
        for actor in actors if isinstance(actors, list) else []:
            if (
                isinstance(actor, dict)
                and actor.get("bypass_mode", "always") == "always"
            ):
                outcome.findings.append(
                    f"{ruleset.path}: {actor.get('actor_type')} {actor.get('actor_id')} may bypass it at any "
                    "time; prefer no bypass actor, or bypass_mode pull_request"
                )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `version`.
# --------------------------------------------------------------------------------------------


@dataclass
class Version:
    where: str
    value: str
    inherited: bool = False


def load_toml(path: Path) -> dict | None:
    try:
        return tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError):
        return None


def load_json(path: Path) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError):
        return None


def workspace_version(tree: Tree) -> str | None:
    root = load_toml(tree.root / "Cargo.toml")
    package = (root or {}).get("workspace", {}).get("package", {})
    value = package.get("version") if isinstance(package, dict) else None
    return value if isinstance(value, str) else None


def versions(tree: Tree) -> tuple[list[Version], list[str]]:
    """Every first-party version a manifest declares, and the manifests that could not be read."""
    found: list[Version] = []
    unreadable: list[str] = []
    inherited = workspace_version(tree)
    for path in tree.files("Cargo.toml"):
        document = load_toml(path)
        where = tree.display(path)
        if document is None:
            unreadable.append(where)
            continue
        workspace = document.get("workspace", {})
        if isinstance(workspace, dict) and isinstance(workspace.get("package"), dict):
            value = workspace["package"].get("version")
            if isinstance(value, str):
                found.append(Version(f"{where} [workspace.package]", value))
        package = document.get("package")
        if isinstance(package, dict):
            value = package.get("version")
            if isinstance(value, str):
                found.append(Version(where, value))
            elif (
                isinstance(value, dict) and value.get("workspace") is True and inherited
            ):
                found.append(Version(where, inherited, inherited=True))
    for name in ("package.json", "tauri.conf.json"):
        for path in tree.files(name):
            document = load_json(path)
            where = tree.display(path)
            if not isinstance(document, dict):
                unreadable.append(where)
                continue
            value = document.get("version")
            if isinstance(value, str) and not value.endswith(".json"):
                found.append(Version(where, value))
    return found, unreadable


def release_version(tree: Tree) -> tuple[str | None, str]:
    inherited = workspace_version(tree)
    if inherited:
        return inherited, "Cargo.toml [workspace.package]"
    root = load_toml(tree.root / "Cargo.toml") or {}
    package = root.get("package")
    if isinstance(package, dict) and isinstance(package.get("version"), str):
        return package["version"], "Cargo.toml"
    document = load_json(tree.root / "package.json")
    if isinstance(document, dict) and isinstance(document.get("version"), str):
        return document["version"], "package.json"
    return None, ""


def check_versions_semver(tree: Tree) -> Outcome:
    """Every first-party version is SemVer 2.0.0."""
    outcome = Outcome()
    found, unreadable = versions(tree)
    outcome.examined = len(found) + len(unreadable)
    if outcome.examined == 0:
        outcome.void = (
            "no Cargo.toml, package.json or tauri.conf.json declares a version"
        )
        return outcome
    for where in unreadable:
        outcome.findings.append(f"{where} cannot be read")
    for version in found:
        if not version.inherited and not SEMVER.fullmatch(version.value):
            outcome.findings.append(
                f"{version.where} declares {version.value!r}, which is not SemVer 2.0.0"
            )
    return outcome


def check_versions_agree(tree: Tree) -> Outcome:
    """One product carries one version (advisory)."""
    outcome = Outcome()
    found, _ = versions(tree)
    outcome.examined = len(found)
    if not found:
        outcome.void = "no manifest declares a version"
        return outcome
    distinct = sorted({version.value for version in found})
    if len(distinct) > 1:
        listed = "; ".join(f"{v.where} {v.value}" for v in found if not v.inherited)
        outcome.findings.append(
            f"the manifests declare {len(distinct)} versions: {listed}"
        )
    return outcome


def check_changelog_has_version(tree: Tree) -> Outcome:
    """The released version, when it is not a pre-release, has a level-2 section in CHANGELOG.md."""
    outcome = Outcome()
    version, where = release_version(tree)
    if version is None:
        outcome.void = "no root manifest declares the product's version"
        return outcome
    outcome.examined = 1
    if "-" in version.split("+", 1)[0] or version.split("+", 1)[0] == "0.0.0":
        return outcome
    changelog = tree.root / "CHANGELOG.md"
    if not changelog.is_file():
        outcome.findings.append(
            f"{where} releases {version} and there is no CHANGELOG.md"
        )
        return outcome
    heading = re.compile(r"^##\s+\[?v?(?P<version>[0-9A-Za-z.+-]+)\]?(?:\s|$)")
    for line in changelog.read_text(encoding="utf-8").splitlines():
        match = heading.match(line)
        if match and match.group("version") == version:
            return outcome
    outcome.findings.append(
        f"{where} releases {version} and CHANGELOG.md has no ## [{version}] section"
    )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `changelog`.
# --------------------------------------------------------------------------------------------


@dataclass
class Scheme:
    name: str
    directory: Path
    types: tuple[str, ...] = ()
    ignored: tuple[str, ...] = ()


def towncrier_config(tree: Tree) -> dict | None:
    for name in ("towncrier.toml", "pyproject.toml"):
        document = load_toml(tree.root / name)
        section = (document or {}).get("tool", {}).get("towncrier")
        if isinstance(section, dict):
            return section
    return None


def fragment_scheme(tree: Tree) -> Scheme | None:
    if (tree.root / ".changeset" / "config.json").is_file():
        return Scheme("changesets", tree.root / ".changeset")
    config = towncrier_config(tree)
    if config is not None:
        directory = (
            config.get("directory")
            if isinstance(config.get("directory"), str)
            else "newsfragments"
        )
        types: list[str] = []
        if isinstance(config.get("type"), list):
            for entry in config["type"]:
                if isinstance(entry, dict):
                    name = entry.get("directory") or str(entry.get("name", "")).lower()
                    if name:
                        types.append(name)
        if isinstance(config.get("fragment"), dict):
            types += [key for key in config["fragment"]]
        ignored = tuple(
            item for item in config.get("ignore", []) if isinstance(item, str)
        )
        return Scheme(
            "towncrier", tree.root / directory, tuple(types) or TOWNCRIER_TYPES, ignored
        )
    if (tree.root / "changelog.d").is_dir():
        return Scheme("keep-a-changelog", tree.root / "changelog.d")
    return None


def keep_a_changelog_fragment(text: str) -> list[str]:
    problems: list[str] = []
    headings = 0
    bullets_in_section = None
    for line in text.splitlines():
        stripped = line.strip()
        match = HEADING.match(stripped)
        if match:
            if bullets_in_section == 0:
                problems.append("a section with no bullet")
            if len(match.group("hashes")) != 3:
                problems.append(
                    f"a {'#' * len(match.group('hashes'))} heading where only ### change types belong"
                )
            elif match.group("title") not in KEEP_A_CHANGELOG:
                problems.append(
                    f"### {match.group('title')} is not one of {', '.join(KEEP_A_CHANGELOG)}"
                )
            headings += 1
            bullets_in_section = 0
        elif stripped:
            if headings == 0:
                problems.append("text before the first ### heading")
            elif stripped.startswith(("- ", "* ")):
                bullets_in_section = (bullets_in_section or 0) + 1
    if headings == 0:
        problems.append("no ### change-type heading")
    elif bullets_in_section == 0:
        problems.append("a section with no bullet")
    return sorted(set(problems))


def towncrier_name(name: str, types: Sequence[str]) -> bool:
    parts = name.split(".")
    for index in range(1, len(parts)):
        if parts[index] in types:
            tail = parts[index + 1 :]
            if (
                not tail
                or (
                    len(tail) == 1
                    and (tail[0].isdigit() or tail[0] in ("md", "rst", "txt"))
                )
                or (
                    len(tail) == 2
                    and tail[0].isdigit()
                    and tail[1] in ("md", "rst", "txt")
                )
            ):
                return bool(parts[0])
    return False


def changeset(text: str) -> list[str]:
    lines = text.split("\n")
    if not lines or lines[0].strip() != "---":
        return ["no --- frontmatter"]
    try:
        close = next(
            index for index in range(1, len(lines)) if lines[index].strip() == "---"
        )
    except StopIteration:
        return ["an unclosed frontmatter"]
    problems = []
    entries = [line for line in lines[1:close] if line.strip()]
    for line in entries:
        match = re.fullmatch(r"\s*(\"[^\"]+\"|'[^']+'|[^:\s]+)\s*:\s*(\S+)\s*", line)
        if match is None or match.group(2) not in BUMPS:
            problems.append(
                f"{line.strip()!r} is not a package: {'|'.join(BUMPS)} bump"
            )
    if entries and not "\n".join(lines[close + 1 :]).strip():
        problems.append("a bump with no summary")
    return problems


def check_fragment_shape(tree: Tree) -> Outcome:
    """Every changelog fragment parses in its scheme: Keep a Changelog headings in changelog.d/,
    towncrier names, or changesets bumps."""
    outcome = Outcome(examined=1)
    scheme = fragment_scheme(tree)
    if scheme is None:
        outcome.findings.append(
            "no fragment directory: changelog.d/, a towncrier directory, or .changeset/"
        )
        return outcome
    if not scheme.directory.is_dir():
        outcome.findings.append(
            f"the {scheme.name} fragment directory {tree.display(scheme.directory)} does not exist"
        )
        return outcome
    for path in sorted(scheme.directory.iterdir()):
        if (
            not path.is_file()
            or FRAGMENT_IGNORED.match(path.name)
            or path.name in scheme.ignored
        ):
            continue
        if scheme.name == "changesets" and path.name == "config.json":
            continue
        outcome.examined += 1
        where = tree.display(path)
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            outcome.findings.append(f"{where} cannot be read: {error}")
            continue
        if scheme.name == "towncrier":
            if not towncrier_name(path.name, scheme.types):
                outcome.findings.append(
                    f"{where} is not <issue>.<type> for a type of {', '.join(scheme.types)}: towncrier fails on it"
                )
            elif not text.strip():
                outcome.findings.append(f"{where} is empty")
            continue
        if path.suffix != ".md":
            outcome.findings.append(f"{where} is not a Markdown fragment")
            continue
        problems = (
            changeset(text)
            if scheme.name == "changesets"
            else keep_a_changelog_fragment(text)
        )
        for problem in problems:
            outcome.findings.append(f"{where}: {problem}")
    return outcome


def check_fragment_check_in_ci(tree: Tree) -> Outcome:
    """A pull-request workflow refuses a pull request that adds no fragment."""
    outcome = Outcome()
    candidates = [w for w in tree.workflows() if pull_request_config(w)[0]]
    outcome.examined = len(candidates)
    if not candidates:
        outcome.void = "no workflow runs on pull_request"
        return outcome
    scheme = fragment_scheme(tree)
    directory = (
        tree.display(scheme.directory)
        if scheme and scheme.directory.exists()
        else "changelog.d"
    )
    for workflow in candidates:
        for job in workflow.jobs().values():
            for text in runs(job):
                if "towncrier check" in text or "changeset status" in text:
                    return outcome
                if directory in text and re.search(r"\bgit\s+(?:diff|log)\b", text):
                    return outcome
    outcome.findings.append(
        "no pull-request workflow refuses a missing fragment (towncrier check, changeset status, or a "
        f"git diff over {directory}/)"
    )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `release`.
# --------------------------------------------------------------------------------------------


def runbook(tree: Tree) -> Path | None:
    for name in RUNBOOK_NAMES:
        path = tree.root / name
        if path.is_file():
            return path
    return None


def check_release_runbook(tree: Tree) -> Outcome:
    """The runbook names the release pull request, an annotated tag, the back-merge and a rollback."""
    outcome = Outcome(examined=1)
    path = runbook(tree)
    if path is None:
        outcome.findings.append(f"no release runbook: {', '.join(RUNBOOK_NAMES)}")
        return outcome
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    where = tree.display(path)
    protected, default = re.escape(tree.protected), re.escape(tree.default)
    # The release pull request itself: a line about a pull request that names both branches, or
    # `gh pr create --base <protected> --head <default>`. A line naming both branches is not
    # enough: the back-merge names them too.
    if not any(
        (
            re.search(r"(?i)\bpull request\b|\bPR\b", line)
            and re.search(rf"\b{default}\b", line)
            and re.search(rf"\b{protected}\b", line)
        )
        or (
            re.search(r"\bgh\s+pr\s+create\b", line)
            and re.search(rf"--base[\s=]+{protected}\b", line)
            and re.search(rf"--head[\s=]+{default}\b", line)
        )
        for line in lines
    ):
        outcome.findings.append(
            f"{where} never names the release pull request from {tree.default} into {tree.protected}"
        )
    if not re.search(
        r"\bgit\s+tag\s+(?:[^\n]*\s)?(?:-[a-zA-Z]*[asum][a-zA-Z]*|--annotate|--sign|--message)\b",
        text,
    ):
        outcome.findings.append(
            f"{where} tags no release with git tag -a or -s: annotated tags are meant for release"
        )
    back_merge = any(
        (re.search(r"\bgit\s+merge\b", line) and re.search(rf"\b{protected}\b", line))
        or (
            re.search(rf"--base\s+{default}\b", line)
            and re.search(rf"--head\s+{protected}\b", line)
        )
        for line in lines
    )
    if not back_merge:
        outcome.findings.append(
            f"{where} never merges {tree.protected} back into {tree.default} after a release"
        )
    headings = [
        m.group("title").lower() for line in lines if (m := HEADING.match(line.strip()))
    ]
    if not any("rollback" in title or "roll back" in title for title in headings):
        outcome.findings.append(f"{where} has no rollback section")
    return outcome


def publishes(workflow: Workflow) -> bool:
    for job in workflow.jobs().values():
        if any(step.split("@", 1)[0] in RELEASE_ACTIONS for step in uses(job)):
            return True
        if any(
            re.search(r"\bgh\s+release\s+(?:create|upload)\b", text)
            for text in runs(job)
        ):
            return True
    return False


def disallowed_triggers(workflow: Workflow) -> list[str]:
    """The triggers a release or a deploy must not run from: anything but a tag push, a published
    release, or a manual dispatch."""
    bad = []
    for event, config in workflow.events().items():
        if event == "push":
            filters = config if isinstance(config, dict) else {}
            if "branches" in filters or "branches-ignore" in filters:
                bad.append("push to a branch")
            elif "tags" not in filters and "tags-ignore" not in filters:
                bad.append("every push")
        elif event not in ("release", "workflow_dispatch"):
            bad.append(event)
    return bad


def check_release_on_tag(tree: Tree) -> Outcome:
    """A release is published only from a tag push (or a published release, or a manual dispatch)."""
    outcome = Outcome()
    publishing = [w for w in tree.workflows() if publishes(w)]
    outcome.examined = len(publishing)
    if not publishing:
        outcome.void = "no workflow publishes a GitHub release"
        return outcome
    for workflow in publishing:
        for trigger in disallowed_triggers(workflow):
            outcome.findings.append(
                f"{workflow.path} publishes a release on {trigger}, not a tag"
            )
    return outcome


def deploy_jobs(tree: Tree) -> list[tuple[Workflow, str, dict]]:
    found = []
    for workflow in tree.workflows():
        for job_id, job in workflow.jobs().items():
            environment = job.get("environment")
            name = (
                environment.get("name")
                if isinstance(environment, dict)
                else environment
            )
            if name == "github-pages":
                continue
            label = f"{job_id} {job.get('name') if isinstance(job.get('name'), str) else ''}".lower()
            if (
                environment
                or "deploy" in label
                or any(DEPLOY_SCRIPT.search(text) for text in runs(job))
            ):
                found.append((workflow, job_id, job))
    return found


def tag_patterns(workflow: Workflow) -> list[str]:
    push = workflow.events().get("push")
    return strings(push.get("tags")) if isinstance(push, dict) else []


def check_tag_filter_semver(tree: Tree) -> Outcome:
    """Every tag filter a release or a deploy runs from admits SemVer tags and nothing else
    (advisory)."""
    outcome = Outcome()
    relevant = {w.path: w for w in tree.workflows() if publishes(w)}
    relevant.update({w.path: w for w, _, _ in deploy_jobs(tree)})
    filtered = [w for w in relevant.values() if tag_patterns(w)]
    outcome.examined = len(filtered)
    if not filtered:
        outcome.void = "no release or deploy workflow filters its tags"
        return outcome
    for workflow in filtered:
        patterns = tag_patterns(workflow)
        admitted_bad = [tag for tag in BAD_TAGS if filter_admits(patterns, tag)]
        if not any(filter_admits(patterns, tag) for tag in GOOD_TAGS):
            outcome.findings.append(
                f"{workflow.path}'s tag filter {patterns} admits no v1.2.3 or 1.2.3 tag"
            )
        if admitted_bad:
            outcome.findings.append(
                f"{workflow.path}'s tag filter {patterns} admits {', '.join(admitted_bad)}: use v[0-9]+.[0-9]+.[0-9]+"
            )
    return outcome


#: `gh release create`'s flags that take a value: the word after one is not an asset.
VALUED_FLAGS = frozenset(
    {
        "-n",
        "--notes",
        "-F",
        "--notes-file",
        "-t",
        "--title",
        "--target",
        "-R",
        "--repo",
        "--discussion-category",
        "--notes-start-tag",
    }
)


def release_create_args(args: str) -> tuple[set[str], list[str]]:
    """`gh release create`'s flags, and the assets it attaches: every positional word after the tag."""
    try:
        words = shlex.split(args, comments=True)
    except ValueError:
        words = args.split()
    flags: set[str] = set()
    positional: list[str] = []
    skip = False
    for word in words:
        if skip:
            skip = False
            continue
        if word in ("&&", "||", ";", "|"):
            break
        if word.startswith("-"):
            flag = word.split("=", 1)[0]
            flags.add(flag)
            skip = flag in VALUED_FLAGS and "=" not in word
            continue
        positional.append(word)
    return flags, positional[1:]


def check_release_draft_first(tree: Tree) -> Outcome:
    """A release that attaches assets is created as a draft and published after its last upload
    (advisory): an immutable release locks its assets when it is published."""
    outcome = Outcome()
    publishing = [w for w in tree.workflows() if publishes(w)]
    outcome.examined = len(publishing)
    if not publishing:
        outcome.void = "no workflow publishes a GitHub release"
        return outcome
    for workflow in publishing:
        for job_id, job in workflow.jobs().items():
            texts = []
            for step in steps(job):
                if isinstance(step.get("run"), str):
                    texts.append(("run", step["run"], {}))
                if isinstance(step.get("uses"), str):
                    texts.append(
                        (
                            "uses",
                            step["uses"],
                            step.get("with")
                            if isinstance(step.get("with"), dict)
                            else {},
                        )
                    )
            uploads = False
            drafted = False
            published_after = False
            for kind, text, inputs in texts:
                if kind == "uses" and text.split("@", 1)[0] in RELEASE_ACTIONS:
                    if inputs.get("files") or inputs.get("artifacts"):
                        uploads = True
                    if (
                        inputs.get("draft") is True
                        or str(inputs.get("draft")).lower() == "true"
                    ):
                        drafted = True
                    continue
                if kind != "run":
                    continue
                for create in re.finditer(
                    r"\bgh\s+release\s+create\b(?P<args>[^\n]*)", text
                ):
                    flags, assets = release_create_args(create.group("args"))
                    if "--draft" in flags or "-d" in flags:
                        drafted = True
                    if assets:
                        uploads = True
                if re.search(r"\bgh\s+release\s+upload\b", text):
                    uploads = True
                if (
                    re.search(r"\bgh\s+release\s+edit\b[^\n]*--draft=false", text)
                    and drafted
                ):
                    published_after = True
            if uploads and not (drafted and published_after):
                outcome.findings.append(
                    f"{workflow.path}:{job_id} attaches assets to a release it does not create as a draft "
                    "and publish after the last upload"
                )
    return outcome


def runner_labels(job: dict) -> list[str] | None:
    value = job.get("runs-on")
    if isinstance(value, dict):
        return None if value.get("group") is None else ["<group>"]
    return strings(value)


def check_hosted_runner_pinned(tree: Tree) -> Outcome:
    """A release or deploy job runs on a versioned standard GitHub-hosted image (advisory)."""
    outcome = Outcome()
    relevant = {w.path: w for w in tree.workflows() if publishes(w)}
    relevant.update({w.path: w for w, _, _ in deploy_jobs(tree)})
    jobs = [
        (w, job_id, job) for w in relevant.values() for job_id, job in w.jobs().items()
    ]
    outcome.examined = len(jobs)
    if not jobs:
        outcome.void = "no release or deploy workflow"
        return outcome
    for workflow, job_id, job in jobs:
        labels = runner_labels(job) or []
        if "self-hosted" in labels or any("${{" in label for label in labels):
            continue
        for label in labels:
            if label in MOVING_LABELS:
                outcome.findings.append(
                    f"{workflow.path}:{job_id} runs on {label}, which moves to a new image: pin ubuntu-24.04 or the like"
                )
            elif not STANDARD_LABEL.fullmatch(label):
                outcome.findings.append(
                    f"{workflow.path}:{job_id} runs on {label}, not a standard hosted label: larger runners are "
                    "billed even in a public repository"
                )
    return outcome


# --------------------------------------------------------------------------------------------
# Stage `deploy`.
# --------------------------------------------------------------------------------------------


def check_deploy_from_tags(tree: Tree) -> Outcome:
    """A deploy runs only from a tag push, a published release, or a manual dispatch."""
    outcome = Outcome()
    jobs = deploy_jobs(tree)
    outcome.examined = len(jobs)
    if not jobs:
        outcome.void = (
            "no deploy job: none names an environment, deploy, or a deploy/ script"
        )
        return outcome
    for workflow in {w.path: w for w, _, _ in jobs}.values():
        for trigger in disallowed_triggers(workflow):
            outcome.findings.append(
                f"{workflow.path} deploys on {trigger}, not a tag on {tree.protected}"
            )
    return outcome


def proves_ancestry(job: dict, protected: str) -> tuple[bool, bool]:
    """Whether the job runs `git merge-base --is-ancestor … <protected>` and has the history for it."""
    branch = re.escape(protected)
    ancestry = any(
        re.search(
            rf"\bgit\s+merge-base\s+--is-ancestor\s+\S+\s+[\"']?(?:origin/|refs/remotes/origin/|refs/heads/)?{branch}\b",
            text,
        )
        for text in runs(job)
    )
    history = any(
        isinstance(step.get("with"), dict)
        and str(step.get("uses", "")).startswith("actions/checkout")
        and step["with"].get("fetch-depth") in (0, "0")
        for step in steps(job)
    ) or any(
        re.search(rf"\bgit\s+fetch\b[^\n]*\b{branch}\b", text) for text in runs(job)
    )
    return ancestry, history


def check_deploy_tag_on_main(tree: Tree) -> Outcome:
    """A deploy job, or a job it needs, proves the tag's commit is on the protected branch."""
    outcome = Outcome()
    jobs = deploy_jobs(tree)
    outcome.examined = len(jobs)
    if not jobs:
        outcome.void = "no deploy job"
        return outcome
    for workflow, job_id, job in jobs:
        all_jobs = workflow.jobs()
        seen: set[str] = set()
        frontier = [job_id]
        proved = history_missing = False
        while frontier:
            current = frontier.pop()
            if current in seen or current not in all_jobs:
                continue
            seen.add(current)
            ancestry, history = proves_ancestry(all_jobs[current], tree.protected)
            if ancestry and history:
                proved = True
            elif ancestry:
                history_missing = True
            frontier += needs_of(all_jobs[current])
        if proved:
            continue
        if history_missing:
            outcome.findings.append(
                f"{workflow.path}:{job_id} checks ancestry on a shallow checkout: fetch-depth: 0 or git fetch "
                f"{tree.protected} first"
            )
        else:
            outcome.findings.append(
                f"{workflow.path}:{job_id} never proves its tag is on {tree.protected} "
                f'(git merge-base --is-ancestor "$GITHUB_SHA" origin/{tree.protected})'
            )
    return outcome


def check_rollback_entrypoint(tree: Tree) -> Outcome:
    """A manual dispatch that takes a tag and deploys it, or a rollback script under deploy/."""
    outcome = Outcome()
    deploying = {w.path for w, _, _ in deploy_jobs(tree)}
    scripts = (
        sorted(p for p in (tree.root / "deploy").glob("*rollback*") if p.is_file())
        if (tree.root / "deploy").is_dir()
        else []
    )
    outcome.examined = len(tree.workflows()) + len(scripts) + 1
    if scripts:
        return outcome
    dispatches = []
    for workflow in tree.workflows():
        dispatch = workflow.events().get("workflow_dispatch")
        if "workflow_dispatch" not in workflow.events():
            continue
        inputs = dispatch.get("inputs") if isinstance(dispatch, dict) else None
        taking = [
            key
            for key in (inputs or {})
            if re.search(r"tag|version|ref", str(key), re.IGNORECASE)
        ]
        dispatches.append((workflow, taking))
        if taking and workflow.path in deploying:
            return outcome
    if any(taking for _, taking in dispatches):
        outcome.findings.append("a dispatch takes a tag but runs no deploy job")
    elif dispatches:
        outcome.findings.append(
            "no dispatch takes the tag to roll back to (an input named tag or version)"
        )
    outcome.findings.append(
        "no rollback path: a workflow_dispatch with a tag input that deploys, or a deploy/ rollback script"
    )
    return outcome


def check_deploy_environment(tree: Tree) -> Outcome:
    """A deploy job names an environment, so its tag policy and reviewers apply (advisory)."""
    outcome = Outcome()
    jobs = deploy_jobs(tree)
    outcome.examined = len(jobs)
    if not jobs:
        outcome.void = "no deploy job"
        return outcome
    for workflow, job_id, job in jobs:
        if not job.get("environment"):
            outcome.findings.append(
                f"{workflow.path}:{job_id} deploys with no environment"
            )
    return outcome


def check_deploy_concurrency(tree: Tree) -> Outcome:
    """One deploy at a time, never cancelling one in flight (advisory)."""
    outcome = Outcome()
    jobs = deploy_jobs(tree)
    outcome.examined = len(jobs)
    if not jobs:
        outcome.void = "no deploy job"
        return outcome
    for workflow, job_id, job in jobs:
        concurrency = job.get("concurrency") or (workflow.document or {}).get(
            "concurrency"
        )
        if not concurrency:
            outcome.findings.append(
                f"{workflow.path}:{job_id} sets no concurrency group, so two deploys can race"
            )
        elif isinstance(concurrency, dict) and concurrency.get(
            "cancel-in-progress"
        ) not in (None, False, "false"):
            outcome.findings.append(
                f"{workflow.path}:{job_id} cancels a deploy in flight (cancel-in-progress)"
            )
    return outcome


def check_rollback_keeps_previous(tree: Tree) -> Outcome:
    """A deploy script keeps releases side by side and switches `current` by an atomic rename
    (advisory)."""
    outcome = Outcome()
    directory = tree.root / "deploy"
    scripts = (
        sorted(
            p
            for p in directory.glob("*.sh")
            if p.is_file() and "rollback" not in p.name
        )
        if (directory.is_dir())
        else []
    )
    outcome.examined = len(scripts)
    if not scripts:
        outcome.void = "no deploy/*.sh script"
        return outcome
    for path in scripts:
        text = "\n".join(
            line
            for line in path.read_text(encoding="utf-8").splitlines()
            if not line.lstrip().startswith("#")
        )
        where = tree.display(path)
        atomic = re.search(
            r"\bmv\s+(?:-[a-zA-Z]*T[a-zA-Z]*|--no-target-directory)\b", text
        )
        if "releases/" not in text:
            outcome.findings.append(
                f"{where} replaces the running release in place: keep releases/<tag>/ side by side"
            )
        elif not atomic:
            outcome.findings.append(
                f"{where} switches releases without an atomic rename: link under a temporary name, then mv -T"
            )
    return outcome


# --------------------------------------------------------------------------------------------
# The command line.
# --------------------------------------------------------------------------------------------

CLASSES: dict[str, tuple[str, Callable[[Tree], Outcome]]] = {
    "main-ruleset": ("protection", check_main_ruleset),
    "release-merge-method": ("protection", check_release_merge_method),
    "required-checks-exist": ("protection", check_required_checks_exist),
    "required-checks-not-skipped": ("protection", check_required_checks_not_skipped),
    "tag-ruleset": ("protection", check_tag_ruleset),
    "ruleset-bypass": ("protection", check_ruleset_bypass),
    "versions-semver": ("version", check_versions_semver),
    "changelog-has-version": ("version", check_changelog_has_version),
    "versions-agree": ("version", check_versions_agree),
    "fragment-shape": ("changelog", check_fragment_shape),
    "fragment-check-in-ci": ("changelog", check_fragment_check_in_ci),
    "release-runbook": ("release", check_release_runbook),
    "release-on-tag": ("release", check_release_on_tag),
    "tag-filter-semver": ("release", check_tag_filter_semver),
    "release-draft-first": ("release", check_release_draft_first),
    "hosted-runner-pinned": ("release", check_hosted_runner_pinned),
    "deploy-from-tags": ("deploy", check_deploy_from_tags),
    "deploy-tag-on-main": ("deploy", check_deploy_tag_on_main),
    "rollback-entrypoint": ("deploy", check_rollback_entrypoint),
    "deploy-environment": ("deploy", check_deploy_environment),
    "deploy-concurrency": ("deploy", check_deploy_concurrency),
    "rollback-keeps-previous": ("deploy", check_rollback_keeps_previous),
}


def emit(lines: Iterable[str]) -> None:
    for line in lines:
        print(line)


def run_class(name: str, tree: Tree) -> int:
    _, judge = CLASSES[name]
    outcome = judge(tree)
    lines = [f"{name}: {finding}" for finding in outcome.findings]
    if outcome.void is not None:
        lines.append(f"{name}: VOID: {outcome.void}")
        outcome.examined = 0
    lines.append(f"examined {outcome.examined}")
    emit(lines)
    return outcome.exit_code()


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="release-ops-probe.py",
        description="Judge a repository's release path (rulesets, tags, releases, deploys) from its tree.",
    )
    parser.add_argument("--root", default=".", help="the repository root to judge")
    parser.add_argument(
        "--protected-branch", default="main", help="the branch releases are tagged on"
    )
    parser.add_argument(
        "--default-branch", default="dev", help="the branch pull requests target"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="print every class and its stage")
    check = commands.add_parser("check", help="run one class")
    check.add_argument("name", help="the class to run")
    parse = commands.add_parser(
        "parse", help="print a YAML file as this probe reads it, as JSON"
    )
    parse.add_argument("file", help="the YAML file, such as a workflow")
    try:
        args = parser.parse_args(argv)
    except SystemExit as exit_:
        return EXIT_USAGE if exit_.code else EXIT_OK
    if args.command == "list":
        emit(f"{stage} {name}" for name, (stage, _) in CLASSES.items())
        return EXIT_OK
    if args.command == "parse":
        try:
            document = parse_yaml(Path(args.file).read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, YamlError) as error:
            print(f"parse: {error}")
            return EXIT_FINDING
        print(json.dumps(document, sort_keys=True))
        return EXIT_OK
    if args.name not in CLASSES:
        print(
            f"release-ops-probe: no class {args.name!r}; `list` names them",
            file=sys.stderr,
        )
        return EXIT_USAGE
    root = Path(args.root)
    if not root.is_dir():
        print(
            f"release-ops-probe: --root {args.root} is not a directory", file=sys.stderr
        )
        return EXIT_USAGE
    return run_class(
        args.name, Tree(root.resolve(), args.protected_branch, args.default_branch)
    )


if __name__ == "__main__":
    sys.exit(main())

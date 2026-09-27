#!/usr/bin/env python3
"""accessibility-probe: static checks of a web tree against WCAG 2.2 AA (SPEC-V2-2223).

The packs/accessibility pack owns WCAG 2.2 level AA as ONE coverage matrix,
`skills/packs/accessibility/coverage.json`. It maps each of the 55 A and AA success criteria to
the rows that check it:

- the sibling packs' rows (web-launch, vibecode-polish, ux-laws, ui-styles, cjk-typography,
  stack-selection), named `pack:row` and never copied;
- this script's classes, for what no sibling checks;
- the axe-core rules a rendered audit runs, and what only a person can judge.

The classes read the tree OFFLINE: its markup (HTML, Svelte, Astro, Vue), its styles (CSS files
and `<style>` blocks), its scripts (JS and TS files and `<script>` blocks), its web app manifest,
its Svelte configuration and its CI workflows. None fetches, renders or runs a page, so a class
names a pattern the Understanding documents record as a failure, or, for an advisory class, a
pattern worth a person's look. What only a browser can decide is the runtime audit's, and
`runtime-audit` checks that one is wired.

Two classes judge the pack's own data rather than the tree: `matrix-complete` (every criterion
mapped exactly once, against the Recommendation's own list) and `matrix-rows-resolve` (every
`pack:row` the matrix names exists, and every row here is mapped).

A criterion with an exception for an essential use (1.3.4, 2.2.1, 2.5.2, 2.5.4, 3.2.2's advised
change) is waived at one site by a comment `a11y-exception: <reason>` on the finding's line or in
the three lines above it. The reason is required, and the waiver is printed.

Classes (`check <class>`) print one line per finding, `<class>: <finding>`, and end with
`examined N`: the files read to decide (the matrix entries for the two matrix classes).

Exit codes: 0 green; 1 at least one finding; 2 usage; 3 VOID, meaning nothing was examined or an
input could not be read. VOID is never a pass.

Standard library only, so it vendors into any repository.

Usage:
    accessibility-probe.py classes
    accessibility-probe.py criteria
    accessibility-probe.py --root R [--skills S] check <class>
"""

from __future__ import annotations

import argparse
import bisect
import html
import json
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

HERE = Path(__file__).resolve().parent
DEFAULT_SKILLS = HERE.parent / "skills"
PACK_SLUG = "accessibility"
COVERAGE_SCHEMA = "phx.accessibility.coverage.v1"

EXIT_GREEN = 0
EXIT_FINDING = 1
EXIT_USAGE = 2
EXIT_VOID = 3

# WCAG 2.2, the W3C Recommendation (https://www.w3.org/TR/WCAG22/): the 55 success criteria
# of levels A and AA, in document order, parsed from the Recommendation's own markup. 4.1.1
# Parsing is obsolete and removed, and the 31 AAA criteria are outside the AA target.
WCAG22_AA = (
    ("1.1.1", "Non-text Content", "A"),
    ("1.2.1", "Audio-only and Video-only (Prerecorded)", "A"),
    ("1.2.2", "Captions (Prerecorded)", "A"),
    ("1.2.3", "Audio Description or Media Alternative (Prerecorded)", "A"),
    ("1.2.4", "Captions (Live)", "AA"),
    ("1.2.5", "Audio Description (Prerecorded)", "AA"),
    ("1.3.1", "Info and Relationships", "A"),
    ("1.3.2", "Meaningful Sequence", "A"),
    ("1.3.3", "Sensory Characteristics", "A"),
    ("1.3.4", "Orientation", "AA"),
    ("1.3.5", "Identify Input Purpose", "AA"),
    ("1.4.1", "Use of Color", "A"),
    ("1.4.2", "Audio Control", "A"),
    ("1.4.3", "Contrast (Minimum)", "AA"),
    ("1.4.4", "Resize Text", "AA"),
    ("1.4.5", "Images of Text", "AA"),
    ("1.4.10", "Reflow", "AA"),
    ("1.4.11", "Non-text Contrast", "AA"),
    ("1.4.12", "Text Spacing", "AA"),
    ("1.4.13", "Content on Hover or Focus", "AA"),
    ("2.1.1", "Keyboard", "A"),
    ("2.1.2", "No Keyboard Trap", "A"),
    ("2.1.4", "Character Key Shortcuts", "A"),
    ("2.2.1", "Timing Adjustable", "A"),
    ("2.2.2", "Pause, Stop, Hide", "A"),
    ("2.3.1", "Three Flashes or Below Threshold", "A"),
    ("2.4.1", "Bypass Blocks", "A"),
    ("2.4.2", "Page Titled", "A"),
    ("2.4.3", "Focus Order", "A"),
    ("2.4.4", "Link Purpose (In Context)", "A"),
    ("2.4.5", "Multiple Ways", "AA"),
    ("2.4.6", "Headings and Labels", "AA"),
    ("2.4.7", "Focus Visible", "AA"),
    ("2.4.11", "Focus Not Obscured (Minimum)", "AA"),
    ("2.5.1", "Pointer Gestures", "A"),
    ("2.5.2", "Pointer Cancellation", "A"),
    ("2.5.3", "Label in Name", "A"),
    ("2.5.4", "Motion Actuation", "A"),
    ("2.5.7", "Dragging Movements", "AA"),
    ("2.5.8", "Target Size (Minimum)", "AA"),
    ("3.1.1", "Language of Page", "A"),
    ("3.1.2", "Language of Parts", "AA"),
    ("3.2.1", "On Focus", "A"),
    ("3.2.2", "On Input", "A"),
    ("3.2.3", "Consistent Navigation", "AA"),
    ("3.2.4", "Consistent Identification", "AA"),
    ("3.2.6", "Consistent Help", "A"),
    ("3.3.1", "Error Identification", "A"),
    ("3.3.2", "Labels or Instructions", "A"),
    ("3.3.3", "Error Suggestion", "AA"),
    ("3.3.4", "Error Prevention (Legal, Financial, Data)", "AA"),
    ("3.3.7", "Redundant Entry", "A"),
    ("3.3.8", "Accessible Authentication (Minimum)", "AA"),
    ("4.1.2", "Name, Role, Value", "A"),
    ("4.1.3", "Status Messages", "AA"),
)

# WAI-ARIA 1.2 (a W3C Recommendation): the concrete roles, then the six roles the 1.3 Working
# Draft adds, which browsers ship. Abstract roles are never for authors.
ARIA_ROLES = frozenset(
    """alert alertdialog application article banner blockquote button caption cell checkbox code
    columnheader combobox complementary contentinfo definition deletion dialog directory document
    emphasis feed figure form generic grid gridcell group heading img insertion link list listbox
    listitem log main marquee math meter menu menubar menuitem menuitemcheckbox menuitemradio
    navigation none note option paragraph presentation progressbar radio radiogroup region row
    rowgroup rowheader scrollbar search searchbox separator slider spinbutton status strong
    subscript superscript switch tab table tablist tabpanel term textbox time timer toolbar
    tooltip tree treegrid treeitem comment image mark sectionfooter sectionheader
    suggestion""".split()
)
ARIA_ABSTRACT_ROLES = frozenset(
    """command composite input landmark range roletype section sectionhead select structure
    widget window""".split()
)
ARIA_DEPRECATED_ROLES = frozenset({"directory"})
# WAI-ARIA 1.2's states and properties, its two deprecated ones, and the 1.3 draft's additions.
ARIA_ATTRIBUTES = frozenset(
    """aria-activedescendant aria-atomic aria-autocomplete aria-busy aria-checked aria-colcount
    aria-colindex aria-colspan aria-controls aria-current aria-describedby aria-details
    aria-disabled aria-errormessage aria-expanded aria-flowto aria-haspopup aria-hidden
    aria-invalid aria-keyshortcuts aria-label aria-labelledby aria-level aria-live aria-modal
    aria-multiline aria-multiselectable aria-orientation aria-owns aria-placeholder
    aria-posinset aria-pressed aria-readonly aria-relevant aria-required aria-roledescription
    aria-rowcount aria-rowindex aria-rowspan aria-selected aria-setsize aria-sort aria-valuemax
    aria-valuemin aria-valuenow aria-valuetext aria-braillelabel aria-brailleroledescription
    aria-colindextext aria-description aria-rowindextext""".split()
)
ARIA_DEPRECATED_ATTRIBUTES = frozenset({"aria-grabbed", "aria-dropeffect"})

CLASSES = (
    "matrix-complete",
    "matrix-rows-resolve",
    "label-in-name",
    "aria-valid",
    "aria-hidden-focus",
    "keyboard-operable",
    "video-captions",
    "meta-refresh",
    "context-change",
    "orientation-lock",
    "auth-autofill",
    "status-messages",
    "pointer-cancel",
    "motion-actuation",
    "key-shortcuts",
    "inline-spacing",
    "contrast-pairs",
    "focus-not-obscured",
    "hover-reveal",
    "flash-rate",
    "svelte-a11y-not-disabled",
    "svelte-a11y-fails-ci",
    "runtime-audit",
)
MATRIX_CLASSES = frozenset({"matrix-complete", "matrix-rows-resolve"})

# Build output, installed packages and tool state are never the product's own source. Every
# directory whose name starts with "." is skipped too, except the two workflow homes.
SKIPPED_DIRS = frozenset(
    {
        "node_modules",
        "target",
        "dist",
        "build",
        "coverage",
        "__pycache__",
        "venv",
        "vendor",
    }
)
WORKFLOW_DIRS = (".github/workflows", ".depot/workflows")
# Tests exercise the product and are not the product a person uses: a test that plants a failure
# to prove a guard is not one. `runtime-audit` reads them; nothing else does.
TEST_DIRS = frozenset(
    {
        "tests",
        "test",
        "__tests__",
        "__mocks__",
        "fixtures",
        "e2e",
        "cypress",
        "playwright",
    }
)
TEST_FILE = re.compile(r"\.(spec|test)\.[cm]?[jt]sx?$")
MARKUP_EXT = frozenset({".html", ".htm", ".svelte", ".astro", ".vue"})
STYLE_EXT = frozenset({".css", ".scss", ".pcss", ".postcss", ".less"})
SCRIPT_EXT = frozenset({".js", ".mjs", ".cjs", ".ts", ".mts", ".cts", ".jsx", ".tsx"})
DATA_EXT = frozenset({".json", ".webmanifest", ".yml", ".yaml"})
MAX_BYTES = 1 << 20

WAIVER = re.compile(r"a11y-exception:[ \t]*(\S[^\n]*?)\s*(?:-->|\*/)?\s*$", re.M)
ROW_REF = re.compile(r"^[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9._-]*$")
AXE_RULE = re.compile(r"^[a-z0-9][a-z0-9-]*$")
VOID_ELEMENTS = frozenset(
    "area base br col embed hr img input link meta param source track wbr".split()
)
RAW_TEXT = frozenset({"script", "style", "textarea", "title"})


class Void(Exception):
    """An input the class needs cannot be read, so the class examined nothing."""


@dataclass(frozen=True)
class Context:
    root: Path
    skills: Path


@dataclass(frozen=True)
class Outcome:
    findings: list[str]
    examined: int
    waived: list[str] = field(default_factory=list)


# --- the tree -------------------------------------------------------------------------------------


@dataclass
class Source:
    """One file of the judged tree, read on first use."""

    rel: str
    path: Path | None
    ext: str
    test: bool
    _text: str | None = None
    _lines: list[int] | None = None

    @property
    def text(self) -> str:
        if self._text is None:
            self._text = ""
            if self.path is not None:
                try:
                    if self.path.stat().st_size <= MAX_BYTES:
                        self._text = self.path.read_bytes().decode(
                            "utf-8", errors="replace"
                        )
                except OSError:
                    self._text = ""
        return self._text

    def line(self, offset: int) -> int:
        if self._lines is None:
            self._lines = [i for i, ch in enumerate(self.text) if ch == "\n"]
        return bisect.bisect_left(self._lines, offset) + 1

    def at(self, offset: int) -> str:
        return f"{self.rel}:{self.line(offset)}"


@dataclass
class Tree:
    files: list[Source]

    def of(self, exts: frozenset, tests: bool = False) -> list[Source]:
        return [f for f in self.files if f.ext in exts and (tests or not f.test)]

    def markup(self) -> list[Source]:
        return self.of(MARKUP_EXT)

    def workflows(self) -> list[Source]:
        return [
            f
            for f in self.files
            if f.rel.startswith(WORKFLOW_DIRS) and f.ext in (".yml", ".yaml")
        ]

    def packages(self) -> list[Source]:
        return [f for f in self.files if Path(f.rel).name == "package.json"]


def is_test(parts: tuple[str, ...], name: str) -> bool:
    return any(part in TEST_DIRS for part in parts[:-1]) or bool(TEST_FILE.search(name))


def keep_dir(parent: str, name: str) -> bool:
    """Whether the walk enters `name` under `parent` ("" is the root)."""
    if name in SKIPPED_DIRS:
        return False
    if name.startswith("."):
        return parent == "" and name in (".github", ".depot")
    return True


def scan(root: Path) -> Tree:
    """Every source file the classes may read, found by a pruned walk and read lazily."""
    files: list[Source] = []
    wanted = MARKUP_EXT | STYLE_EXT | SCRIPT_EXT | DATA_EXT
    for dirpath, dirnames, filenames in os.walk(root):
        here = Path(dirpath)
        parent = here.relative_to(root).as_posix()
        parent = "" if parent == "." else parent
        dirnames[:] = sorted(name for name in dirnames if keep_dir(parent, name))
        if parent.startswith((".github", ".depot")) and not parent.startswith(
            WORKFLOW_DIRS
        ):
            continue
        for name in sorted(filenames):
            ext = Path(name).suffix.lower()
            if ext not in wanted:
                continue
            rel = f"{parent}/{name}" if parent else name
            files.append(
                Source(rel, here / name, ext, is_test(tuple(rel.split("/")), name))
            )
    return Tree(files)


# --- markup ---------------------------------------------------------------------------------------


@dataclass
class Element:
    name: str
    attrs: dict[str, str | None]
    start: int
    end: int
    parent: "Element | None" = None
    children: list = field(default_factory=list)

    def has(self, name: str) -> bool:
        return name in self.attrs

    def get(self, name: str) -> str | None:
        return self.attrs.get(name)

    def literal(self, name: str) -> str | None:
        """An attribute's static value, or None when it is absent, boolean or an expression."""
        value = self.attrs.get(name)
        if value is None or "{" in value:
            return None
        return value

    def ancestors(self):
        node = self.parent
        while node is not None:
            yield node
            node = node.parent

    def descendants(self):
        for child in self.children:
            if isinstance(child, Element):
                yield child
                yield from child.descendants()

    def text(self) -> str:
        parts = []
        for child in self.children:
            parts.append(child.text() if isinstance(child, Element) else child)
        return "".join(parts)


@dataclass
class Markup:
    source: Source
    elements: list[Element]
    comments: list[tuple[int, str]]
    scripts: list[tuple[int, str]]
    styles: list[tuple[int, str]]
    texts: list[tuple[int, str, Element | None]]


TAG_OPEN = re.compile(r"<(/?)([A-Za-z][A-Za-z0-9:._-]*)")
ATTR_NAME = re.compile(r"[^\s=/>\"'{}]+")


def read_braced(text: str, at: int) -> int:
    """The index just past the `}` that closes the `{` at `at`, honouring quotes and nesting."""
    depth = 0
    quote = None
    i = at
    while i < len(text):
        ch = text[i]
        if quote:
            if ch == "\\":
                i += 2
                continue
            if ch == quote:
                quote = None
        elif ch in "\"'`":
            quote = ch
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return len(text)


def read_tag(text: str, at: int) -> tuple[dict[str, str | None], int, bool] | None:
    """Attributes from `at` (just past the tag name) to the closing `>`, the end, self-closing."""
    attrs: dict[str, str | None] = {}
    i = at
    n = len(text)
    while i < n:
        ch = text[i]
        if ch.isspace():
            i += 1
            continue
        if ch == ">":
            return attrs, i + 1, False
        if text.startswith("/>", i):
            return attrs, i + 2, True
        if ch == "/":
            i += 1
            continue
        if ch == "{":
            end = read_braced(text, i)
            attrs.setdefault("{" + text[i + 1 : end - 1].strip() + "}", None)
            i = end
            continue
        match = ATTR_NAME.match(text, i)
        if not match:
            i += 1
            continue
        name = match.group(0).lower()
        i = match.end()
        while i < n and text[i] in " \t":
            i += 1
        if i < n and text[i] == "=":
            i += 1
            while i < n and text[i].isspace():
                i += 1
            if i < n and text[i] in "\"'":
                quote = text[i]
                close = text.find(quote, i + 1)
                if close < 0:
                    return None
                attrs[name] = html.unescape(text[i + 1 : close])
                i = close + 1
            elif i < n and text[i] == "{":
                end = read_braced(text, i)
                attrs[name] = text[i:end]
                i = end
            else:
                match = re.compile(r"[^\s>]+").match(text, i)
                value = match.group(0) if match else ""
                attrs[name] = value
                i += len(value)
        else:
            attrs[name] = None
    return None


def parse_markup(source: Source) -> Markup:
    text = source.text
    elements: list[Element] = []
    comments: list[tuple[int, str]] = []
    scripts: list[tuple[int, str]] = []
    styles: list[tuple[int, str]] = []
    texts: list[tuple[int, str, Element | None]] = []
    stack: list[Element] = []
    i = 0
    n = len(text)

    def add_text(start: int, end: int) -> None:
        if end > start:
            chunk = text[start:end]
            parent = stack[-1] if stack else None
            texts.append((start, chunk, parent))
            if parent is not None:
                parent.children.append(html.unescape(chunk))

    while i < n:
        lt = text.find("<", i)
        if lt < 0:
            add_text(i, n)
            break
        if text.startswith("<!--", lt):
            add_text(i, lt)
            close = text.find("-->", lt + 4)
            end = n if close < 0 else close + 3
            comments.append((lt, text[lt + 4 : end - 3 if close >= 0 else end]))
            i = end
            continue
        match = TAG_OPEN.match(text, lt)
        if not match:
            add_text(i, lt + 1)
            i = lt + 1
            continue
        add_text(i, lt)
        closing = match.group(1) == "/"
        name = match.group(2).lower()
        parsed = read_tag(text, match.end())
        if parsed is None:
            i = match.end()
            continue
        attrs, end, self_close = parsed
        if closing:
            for depth in range(len(stack) - 1, -1, -1):
                if stack[depth].name == name:
                    del stack[depth:]
                    break
            i = end
            continue
        element = Element(name, attrs, lt, end, stack[-1] if stack else None)
        if element.parent is not None:
            element.parent.children.append(element)
        elements.append(element)
        if name in RAW_TEXT and not self_close:
            close = re.compile(rf"</{re.escape(name)}\s*>", re.I).search(text, end)
            body_end = close.start() if close else n
            body = text[end:body_end]
            if name == "script":
                scripts.append((end, body))
            elif name == "style":
                styles.append((end, body))
            else:
                element.children.append(html.unescape(body))
            i = close.end() if close else n
            continue
        if not self_close and name not in VOID_ELEMENTS:
            stack.append(element)
        i = end
    return Markup(source, elements, comments, scripts, styles, texts)


_MARKUP_CACHE: dict[int, Markup] = {}


def markup_of(source: Source) -> Markup:
    key = id(source)
    if key not in _MARKUP_CACHE:
        _MARKUP_CACHE[key] = parse_markup(source)
    return _MARKUP_CACHE[key]


# --- scripts and styles ---------------------------------------------------------------------------


def strip_js_comments(text: str) -> str:
    """JS text with `//` and `/* */` comments blanked (offsets kept), strings left alone."""
    out = list(text)
    i = 0
    n = len(text)
    quote = None
    while i < n:
        ch = text[i]
        if quote:
            if ch == "\\":
                i += 2
                continue
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch in "\"'`":
            quote = ch
            i += 1
            continue
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            for k in range(i, end):
                out[k] = " "
            i = end
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            end = n if end < 0 else end + 2
            for k in range(i, end):
                if out[k] != "\n":
                    out[k] = " "
            i = end
            continue
        i += 1
    return "".join(out)


@dataclass
class Script:
    source: Source
    offset: int
    code: str


def scripts_of(tree: Tree) -> list[Script]:
    found: list[Script] = []
    for source in tree.of(SCRIPT_EXT):
        found.append(Script(source, 0, strip_js_comments(source.text)))
    for source in tree.markup():
        for offset, body in markup_of(source).scripts:
            found.append(Script(source, offset, strip_js_comments(body)))
    return found


@dataclass
class Rule:
    selectors: list[str]
    decls: list[tuple[str, str, bool]]
    media: str
    source: Source
    offset: int
    at: str | None = None

    def value(self, prop: str) -> str | None:
        for name, value, _important in reversed(self.decls):
            if name == prop:
                return value
        return None


def strip_css_comments(text: str) -> str:
    return re.sub(
        r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), text, flags=re.S
    )


def parse_css(source: Source, text: str, base: int) -> list[Rule]:
    """Every rule of a stylesheet, nested rules and at-rule bodies included, with its offset."""
    text = strip_css_comments(text)
    rules: list[Rule] = []

    def block(start: int, end: int, parents: list[str], media: str) -> None:
        i = start
        prelude_start = start
        decl_start = start
        own: list[tuple[str, str, bool]] = []
        own_offset = start
        while i < end:
            ch = text[i]
            if ch in "\"'":
                close = text.find(ch, i + 1)
                i = end if close < 0 else close + 1
                continue
            if ch == "{":
                prelude = text[prelude_start:i].strip()
                close = matching(text, i, end)
                if prelude.startswith("@"):
                    keyword = re.match(r"@([\w-]+)", prelude)
                    word = keyword.group(1).lower() if keyword else ""
                    if word == "font-face":
                        rules.append(
                            Rule(
                                [],
                                declarations(text[i + 1 : close]),
                                media,
                                source,
                                base
                                + prelude_start
                                + (
                                    len(text[prelude_start:i])
                                    - len(text[prelude_start:i].lstrip())
                                ),
                                "font-face",
                            )
                        )
                    elif word in ("keyframes", "-webkit-keyframes"):
                        pass
                    else:
                        block(i + 1, close, parents, f"{media} {prelude}".strip())
                else:
                    selectors = split_selectors(prelude)
                    if parents:
                        selectors = [
                            child.replace("&", parent)
                            if "&" in child
                            else f"{parent} {child}"
                            for parent in parents
                            for child in selectors
                        ]
                    lead = len(text[prelude_start:i]) - len(
                        text[prelude_start:i].lstrip()
                    )
                    inner = text[i + 1 : close]
                    rules.append(
                        Rule(
                            selectors,
                            declarations(strip_nested(inner)),
                            media,
                            source,
                            base + prelude_start + lead,
                        )
                    )
                    block(i + 1, close, selectors, media)
                i = close + 1
                prelude_start = i
                decl_start = i
                continue
            if ch == ";":
                prelude_start = i + 1
                decl_start = i + 1
            i += 1
        del own, own_offset, decl_start

    block(0, len(text), [], "")
    return rules


def matching(text: str, at: int, end: int) -> int:
    depth = 0
    i = at
    while i < end:
        ch = text[i]
        if ch in "\"'":
            close = text.find(ch, i + 1)
            i = end if close < 0 else close + 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return end


def strip_nested(body: str) -> str:
    """A block's own declarations: nested blocks removed."""
    out = []
    depth = 0
    for ch in body:
        if ch == "{":
            depth += 1
            out.append(";")
            continue
        if ch == "}":
            depth -= 1
            continue
        if depth == 0:
            out.append(ch)
    return "".join(out)


def split_selectors(prelude: str) -> list[str]:
    parts = []
    depth = 0
    current = []
    for ch in prelude:
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
            continue
        current.append(ch)
    parts.append("".join(current).strip())
    return [" ".join(part.split()) for part in parts if part]


def declarations(body: str) -> list[tuple[str, str, bool]]:
    decls = []
    depth = 0
    current = []
    pieces = []
    for ch in body:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == ";" and depth == 0:
            pieces.append("".join(current))
            current = []
            continue
        current.append(ch)
    pieces.append("".join(current))
    for piece in pieces:
        if ":" not in piece:
            continue
        name, _, value = piece.partition(":")
        name = name.strip().lower()
        if not re.fullmatch(r"-?-?[a-z][a-z0-9-]*", name):
            continue
        value = " ".join(value.split())
        important = bool(re.search(r"!\s*important\s*$", value, re.I))
        value = re.sub(r"\s*!\s*important\s*$", "", value, flags=re.I)
        decls.append((name, value, important))
    return decls


def rules_of(tree: Tree) -> tuple[list[Rule], int]:
    """Every CSS rule the tree's product code declares, and how many sources were read."""
    rules: list[Rule] = []
    read = 0
    for source in tree.of(STYLE_EXT):
        read += 1
        rules.extend(parse_css(source, source.text, 0))
    for source in tree.markup():
        blocks = markup_of(source).styles
        if blocks:
            read += 1
        for offset, body in blocks:
            rules.extend(parse_css(source, body, offset))
    return rules, read


# --- shared helpers -------------------------------------------------------------------------------


def waiver(source: Source, offset: int) -> str | None:
    """The `a11y-exception:` reason on the finding's line or in the three lines above it."""
    line = source.line(offset)
    lines = source.text.split("\n")
    for number in range(max(1, line - 3), line + 1):
        match = WAIVER.search(lines[number - 1])
        if match and match.group(1).strip():
            return match.group(1).strip()
    return None


def handler_attrs(
    element: Element, events: tuple[str, ...]
) -> list[tuple[str, str | None]]:
    """(attribute, value) for every handler of `events`: `onX`, `on:X`, `@X` and `v-on:X`."""
    found = []
    for name, value in element.attrs.items():
        base = name.split("|", 1)[0]
        for event in events:
            if base in (f"on{event}", f"on:{event}", f"@{event}", f"v-on:{event}"):
                found.append((name, value))
    return found


def function_body(scripts: list[tuple[int, str]], name: str) -> str | None:
    """The body of the function or arrow bound to `name` in a component's own scripts."""
    head = re.compile(
        rf"(?:function\s+{re.escape(name)}\s*\(|(?:const|let|var)\s+{re.escape(name)}\s*=\s*"
        rf"(?:async\s*)?(?:function\b[^(]*\(|\([^)]*\)\s*=>|[A-Za-z_$][\w$]*\s*=>)|"
        rf"\b{re.escape(name)}\s*=\s*(?:async\s*)?function\b)"
    )
    for _offset, code in scripts:
        code = strip_js_comments(code)
        match = head.search(code)
        if not match:
            continue
        brace = code.find("{", match.end() - 1)
        arrow = code.rfind("=>", match.start(), match.end())
        rest = code[match.end() :]
        if arrow >= 0 and not rest.lstrip().startswith("{"):
            line_end = code.find("\n", match.end())
            return code[match.end() : line_end if line_end >= 0 else len(code)]
        if brace < 0:
            return None
        return code[brace : read_braced(code, brace)]
    return None


def handler_code(markup: Markup, value: str | None) -> str | None:
    """What a handler attribute runs: its inline code, or the body of the function it names."""
    if value is None:
        return None
    if value.startswith("{") and value.endswith("}"):
        inner = value[1:-1].strip()
        if re.fullmatch(r"[A-Za-z_$][\w$.]*", inner):
            return function_body(markup.scripts, inner.split(".")[-1])
        return inner
    return value


def emit(
    outcome_findings: list[str],
    waived: list[str],
    source: Source,
    offset: int,
    message: str,
    waivable: bool,
) -> None:
    if waivable:
        reason = waiver(source, offset)
        if reason:
            waived.append(f"{source.at(offset)} ({reason})")
            return
    outcome_findings.append(f"{source.at(offset)} {message}")


def tag_label(element: Element) -> str:
    return f"<{element.name}>"


def visible_text(element: Element) -> str | None:
    """An element's static visible text, or None when any of it is an expression."""
    text = element.text()
    if "{" in text:
        return None
    return " ".join(text.split())


def normal(text: str) -> str:
    """Case-folded, punctuation and symbols dropped, whitespace collapsed (Understanding 2.5.3)."""
    return " ".join(
        "".join(ch if ch.isalnum() else " " for ch in text.casefold()).split()
    )


# --- the matrix ---------------------------------------------------------------------------------


def read_json(path: Path, what: str) -> object:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Void(f"cannot read {what} {path}: {error}") from None


def load_coverage(context: Context) -> dict:
    path = context.skills / "packs" / PACK_SLUG / "coverage.json"
    coverage = read_json(path, "the coverage matrix")
    if not isinstance(coverage, dict):
        raise Void(f"the coverage matrix {path} is not an object")
    return coverage


def criteria_of(coverage: dict) -> list:
    criteria = coverage.get("criteria")
    if not isinstance(criteria, list) or not criteria:
        raise Void("the coverage matrix lists no criteria")
    return criteria


def string_list(value: object) -> list[str] | None:
    if value is None:
        return []
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return value
    return None


def check_matrix_complete(context: Context) -> Outcome:
    coverage = load_coverage(context)
    criteria = criteria_of(coverage)
    canon = {sc: (name, level) for sc, name, level in WCAG22_AA}
    findings: list[str] = []
    if coverage.get("schema") != COVERAGE_SCHEMA:
        findings.append(
            f"the matrix's schema is {coverage.get('schema')!r}, not {COVERAGE_SCHEMA}"
        )
    counts: Counter = Counter()
    for entry in criteria:
        if not isinstance(entry, dict):
            findings.append(f"a criteria entry is not an object: {entry!r}")
            continue
        sc = entry.get("sc")
        counts[sc] += 1
        if sc not in canon:
            findings.append(f"{sc} is not an A or AA criterion of WCAG 2.2")
            continue
        name, level = canon[sc]
        if entry.get("level") != level:
            findings.append(
                f"{sc} is level {level} in WCAG 2.2, not {entry.get('level')}"
            )
        if entry.get("name") != name:
            findings.append(
                f"{sc} is named {name!r} in WCAG 2.2, not {entry.get('name')!r}"
            )
        static = string_list(entry.get("static"))
        runtime = string_list(entry.get("runtime"))
        if static is None:
            findings.append(f"{sc}'s static is not a list of pack:row names")
            static = []
        if runtime is None:
            findings.append(f"{sc}'s runtime is not a list of axe-core rule ids")
            runtime = []
        taught = string_list(entry.get("taught_by"))
        if taught is None:
            findings.append(f"{sc}'s taught_by is not a list of pack:row names")
            taught = []
        for ref in static + taught:
            if not ROW_REF.match(ref):
                findings.append(f"{sc} names {ref!r}, not pack:row")
        for rule in runtime:
            if not AXE_RULE.match(rule):
                findings.append(f"{sc} names {rule!r}, not an axe-core rule id")
        manual = entry.get("manual")
        if (
            not static
            and not runtime
            and not (isinstance(manual, str) and manual.strip())
        ):
            findings.append(
                f"{sc} has no static row, no runtime rule and no manual reason"
            )
        teach = entry.get("teach")
        if not isinstance(teach, str) or not teach.strip():
            findings.append(f"{sc} teaches nothing")
    for sc, (name, _level) in canon.items():
        if counts[sc] == 0:
            findings.append(f"{sc} {name} is not mapped")
    for sc, count in sorted(counts.items(), key=lambda item: str(item[0])):
        if count > 1 and sc in canon:
            findings.append(f"{sc} is mapped {count} times")
    removed = coverage.get("removed")
    if not isinstance(removed, list) or not any(
        isinstance(item, dict) and item.get("sc") == "4.1.1" for item in removed
    ):
        findings.append("4.1.1 Parsing is not recorded as removed")
    return Outcome(findings, len(criteria))


def pack_rows(skills: Path, slug: str) -> set[str] | None:
    try:
        body = json.loads(
            (skills / "packs" / slug / "checks.json").read_text(encoding="utf-8")
        )
    except (OSError, ValueError):
        return None
    checks = body.get("checks") if isinstance(body, dict) else None
    if not isinstance(checks, list):
        return None
    return {row.get("id") for row in checks if isinstance(row, dict)}


def check_matrix_rows_resolve(context: Context) -> Outcome:
    coverage = load_coverage(context)
    criteria = criteria_of(coverage)
    own = pack_rows(context.skills, PACK_SLUG)
    if own is None:
        raise Void(f"cannot read packs/{PACK_SLUG}/checks.json under {context.skills}")
    findings: list[str] = []
    cache: dict[str, set[str] | None] = {}
    referenced: set[str] = set()
    examined = 0
    for entry in criteria:
        if not isinstance(entry, dict):
            continue
        sc = entry.get("sc")
        refs = (string_list(entry.get("static")) or []) + (
            string_list(entry.get("taught_by")) or []
        )
        for ref in refs:
            if not ROW_REF.match(ref):
                continue
            examined += 1
            slug, _, row = ref.partition(":")
            if slug == PACK_SLUG:
                referenced.add(row)
            if slug not in cache:
                cache[slug] = pack_rows(context.skills, slug)
            rows = cache[slug]
            if rows is None:
                findings.append(
                    f"{sc} names {ref}, and there is no packs/{slug}/checks.json"
                )
            elif row not in rows:
                findings.append(f"{sc} names {ref}, and packs/{slug} has no row {row}")
    for row in sorted(own):
        examined += 1
        if row not in MATRIX_CLASSES and row not in referenced:
            findings.append(f"{PACK_SLUG}:{row} is mapped to no criterion")
    return Outcome(findings, examined)


# --- markup classes -------------------------------------------------------------------------------


def markup_sources(tree: Tree) -> list[Source]:
    return tree.markup()


NAMED_ROLES = frozenset(
    {
        "button",
        "link",
        "menuitem",
        "menuitemcheckbox",
        "menuitemradio",
        "tab",
        "checkbox",
        "radio",
        "switch",
        "option",
        "treeitem",
    }
)


def check_label_in_name(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            label = element.literal("aria-label")
            if label is None or not label.strip():
                continue
            if element.name == "input":
                kind = (element.literal("type") or "text").lower()
                if kind not in ("submit", "button", "reset"):
                    continue
                shown = element.literal("value")
            elif (
                element.name in ("button", "a")
                or (element.literal("role") or "") in NAMED_ROLES
            ):
                shown = visible_text(element)
            else:
                continue
            if not shown or not normal(shown):
                continue
            if normal(shown) not in normal(label):
                findings.append(
                    f"{source.at(element.start)} {tag_label(element)} shows {shown!r} but its name is {label!r}"
                )
    return Outcome(findings, len(sources))


def check_aria_valid(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            role = element.literal("role")
            if role is not None:
                for token in role.split():
                    token = token.lower()
                    if token in ARIA_ABSTRACT_ROLES:
                        findings.append(
                            f"{source.at(element.start)} role {token!r} is abstract; authors may not use it"
                        )
                    elif token in ARIA_DEPRECATED_ROLES:
                        findings.append(
                            f"{source.at(element.start)} role {token!r} is deprecated in WAI-ARIA 1.2"
                        )
                    elif token not in ARIA_ROLES:
                        findings.append(
                            f"{source.at(element.start)} role {token!r} is not a WAI-ARIA role"
                        )
            for name in element.attrs:
                base = name.split("|", 1)[0]
                if not base.startswith("aria-"):
                    continue
                if base in ARIA_DEPRECATED_ATTRIBUTES:
                    findings.append(
                        f"{source.at(element.start)} {base!r} is deprecated in WAI-ARIA 1.2"
                    )
                elif base not in ARIA_ATTRIBUTES:
                    findings.append(
                        f"{source.at(element.start)} {base!r} is not a WAI-ARIA attribute"
                    )
    return Outcome(findings, len(sources))


def focusable(element: Element) -> bool:
    """Whether an element takes sequential keyboard focus as written."""
    tabindex = element.literal("tabindex")
    if tabindex is not None:
        try:
            return int(tabindex.strip()) >= 0
        except ValueError:
            return True
    if element.has("tabindex"):
        return True
    if element.has("disabled") and element.name in (
        "button",
        "input",
        "select",
        "textarea",
    ):
        return False
    if element.name == "a":
        return element.has("href")
    if element.name == "input":
        return (element.literal("type") or "").lower() != "hidden"
    if element.name in ("button", "select", "textarea", "summary", "iframe"):
        return True
    if element.name in ("audio", "video"):
        return element.has("controls")
    editable = element.get("contenteditable")
    return editable is not None and editable.lower() != "false"


def check_aria_hidden_focus(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            if (element.literal("aria-hidden") or "").strip().lower() != "true":
                continue
            if focusable(element):
                findings.append(
                    f"{source.at(element.start)} {tag_label(element)} is aria-hidden and takes focus"
                )
            for inner in element.descendants():
                if focusable(inner):
                    findings.append(
                        f"{source.at(inner.start)} {tag_label(inner)} takes focus inside an aria-hidden "
                        f"{tag_label(element)} (line {source.line(element.start)})"
                    )
    return Outcome(findings, len(sources))


POINTER_EVENTS = (
    "click",
    "mousedown",
    "mouseup",
    "pointerdown",
    "pointerup",
    "touchstart",
    "touchend",
    "dblclick",
)
KEY_EVENTS = ("keydown", "keyup", "keypress")
POINTER_EXEMPT = frozenset(
    {
        "button",
        "input",
        "select",
        "textarea",
        "summary",
        "option",
        "label",
        "dialog",
        "form",
        "body",
        "html",
        "details",
    }
)


def check_keyboard_operable(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            pointer = handler_attrs(element, POINTER_EVENTS)
            if not pointer:
                continue
            if element.name.startswith("svelte:") or element.name in POINTER_EXEMPT:
                continue
            if element.name == "a" and element.has("href"):
                continue
            if any(focusable(inner) for inner in element.descendants()):
                continue
            takes_focus = focusable(element)
            keyed = bool(handler_attrs(element, KEY_EVENTS))
            if takes_focus and keyed:
                continue
            handler = pointer[0][0]
            if not takes_focus and not keyed:
                gap = "takes no keyboard focus and handles no key"
            elif not takes_focus:
                gap = "takes no keyboard focus"
            else:
                gap = "handles no key"
            findings.append(
                f"{source.at(element.start)} {tag_label(element)} handles {handler} but {gap}"
            )
    return Outcome(findings, len(sources))


def check_video_captions(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            if element.name != "video" or element.has("muted"):
                continue
            kinds = [
                (inner.literal("kind") or "").lower()
                for inner in element.descendants()
                if inner.name == "track"
            ]
            if "captions" in kinds:
                continue
            note = " (subtitles are not captions)" if "subtitles" in kinds else ""
            findings.append(
                f"{source.at(element.start)} <video> has sound and no captions track{note}"
            )
    return Outcome(findings, len(sources))


REFRESH = re.compile(r"^\s*(\d+)(?:\.\d+)?\s*(?:[;,]|$)")
TWENTY_HOURS = 72000


def check_meta_refresh(tree: Tree) -> Outcome:
    findings: list[str] = []
    waived: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            if (
                element.name != "meta"
                or (element.literal("http-equiv") or "").strip().lower() != "refresh"
            ):
                continue
            match = REFRESH.match(element.literal("content") or "")
            if not match:
                continue
            delay = int(match.group(1))
            if 0 < delay <= TWENTY_HOURS:
                emit(
                    findings,
                    waived,
                    source,
                    element.start,
                    f"refreshes the page after {delay} seconds",
                    True,
                )
    return Outcome(findings, len(sources), waived)


CONTEXT_CALL = re.compile(
    r"\.blur\s*\(|\blocation\s*(?:\.href\s*)?=[^=]|\blocation\.(?:assign|replace|reload)\s*\(|"
    r"\bwindow\.open\s*\(|\bgoto\s*\(|\.submit\s*\(|\brequestSubmit\s*\(|\bnavigate\s*\("
)
FORM_CONTROLS = frozenset({"select", "input", "textarea"})


def check_context_change(tree: Tree) -> Outcome:
    findings: list[str] = []
    waived: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        markup = markup_of(source)
        for element in markup.elements:
            checks: list[tuple[str, str | None, str]] = []
            for name, value in handler_attrs(element, ("focus", "focusin")):
                checks.append((name, value, "moves focus away or navigates"))
            if element.name in FORM_CONTROLS:
                for name, value in handler_attrs(element, ("change", "input")):
                    checks.append((name, value, "submits or navigates"))
                kind = (element.literal("type") or "").lower()
                if element.name == "input" and kind in ("radio", "checkbox"):
                    for name, value in handler_attrs(element, ("click",)):
                        checks.append((name, value, "submits or navigates"))
            for name, value, what in checks:
                code = handler_code(markup, value)
                if code and CONTEXT_CALL.search(code):
                    emit(
                        findings,
                        waived,
                        source,
                        element.start,
                        f"{tag_label(element)} {name} {what}",
                        True,
                    )
    return Outcome(findings, len(sources), waived)


ORIENTATION_LOCK = re.compile(
    r"(?<![\w$])(?:screen\.orientation\.lock|lockOrientation)\s*\("
)
LOCKING_ORIENTATIONS = frozenset(
    {
        "portrait",
        "portrait-primary",
        "portrait-secondary",
        "landscape",
        "landscape-primary",
        "landscape-secondary",
    }
)


def is_manifest(source: Source, data: object) -> bool:
    name = Path(source.rel).name
    if source.ext == ".webmanifest":
        return True
    return (
        name.endswith("manifest.json")
        and isinstance(data, dict)
        and ("start_url" in data or "display" in data)
    )


def check_orientation_lock(tree: Tree) -> Outcome:
    findings: list[str] = []
    waived: list[str] = []
    scripts = scripts_of(tree)
    examined = len({script.source.rel for script in scripts})
    for script in scripts:
        for match in ORIENTATION_LOCK.finditer(script.code):
            emit(
                findings,
                waived,
                script.source,
                script.offset + match.start(),
                "locks the display orientation",
                True,
            )
    for source in tree.files:
        if source.ext not in (".webmanifest", ".json") or source.test:
            continue
        try:
            data = json.loads(source.text)
        except ValueError:
            continue
        if not is_manifest(source, data):
            continue
        examined += 1
        orientation = data.get("orientation") if isinstance(data, dict) else None
        if isinstance(orientation, str) and orientation in LOCKING_ORIENTATIONS:
            if waiver(source, 0) is None:
                findings.append(f"{source.rel} locks orientation to {orientation!r}")
    return Outcome(findings, examined, waived)


def check_auth_autofill(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            if (
                element.name != "input"
                or (element.literal("type") or "").lower() != "password"
            ):
                continue
            if (element.literal("autocomplete") or "").strip().lower() == "off":
                findings.append(
                    f'{source.at(element.start)} a password field sets autocomplete="off"'
                )
                continue
            for outer in element.ancestors():
                if (
                    outer.name == "form"
                    and (outer.literal("autocomplete") or "").strip().lower() == "off"
                ):
                    findings.append(
                        f"{source.at(element.start)} a password field sits in a form that sets "
                        f'autocomplete="off" (line {source.line(outer.start)})'
                    )
                    break
    return Outcome(findings, len(sources))


STATUS_WORDS = frozenset(
    {"toast", "toaster", "snackbar", "flash", "notification", "notice"}
)
LIVE_ROLES = frozenset({"status", "alert", "log", "progressbar", "marquee", "timer"})


def announces(element: Element) -> bool:
    role = (element.literal("role") or "").lower().split()
    return bool(LIVE_ROLES.intersection(role)) or element.has("aria-live")


def status_word(value: str | None) -> bool:
    if not value:
        return False
    for token in value.split():
        if token.lower() in STATUS_WORDS:
            return True
    return False


def check_status_messages(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        markup = markup_of(source)
        stem = re.split(r"[.\-_]", Path(source.rel).stem)[0].lower()
        if stem.rstrip("s") in {w.rstrip("s") for w in STATUS_WORDS} or stem in (
            "snackbar",
            "toast",
            "toaster",
        ):
            if not any(announces(element) for element in markup.elements):
                findings.append(
                    f"{source.rel} is a status surface with no role=status, alert or log and no aria-live"
                )
            continue
        for element in markup.elements:
            if not (
                status_word(element.literal("class"))
                or status_word(element.literal("id"))
            ):
                continue
            if (
                announces(element)
                or any(announces(outer) for outer in element.ancestors())
                or any(announces(inner) for inner in element.descendants())
            ):
                continue
            label = element.literal("class") or element.literal("id")
            findings.append(
                f'{source.at(element.start)} <{element.name} class="{label}"> is a status surface with no '
                "role=status, alert or log and no aria-live"
            )
    return Outcome(findings, len(sources))


# --- script classes -------------------------------------------------------------------------------

DOWN_EVENTS = ("mousedown", "pointerdown", "touchstart")
UP_EVENTS = ("mouseup", "pointerup", "touchend", "click")
LISTENER = re.compile(r"addEventListener\s*\(\s*['\"`]([a-z]+)['\"`]")


def check_pointer_cancel(tree: Tree) -> Outcome:
    findings: list[str] = []
    waived: list[str] = []
    sources = markup_sources(tree)
    examined = len(sources)
    for source in sources:
        for element in markup_of(source).elements:
            down = handler_attrs(element, DOWN_EVENTS)
            if down and not handler_attrs(element, UP_EVENTS):
                emit(
                    findings,
                    waived,
                    source,
                    element.start,
                    f"{tag_label(element)} acts on {down[0][0]} with no up or click handler",
                    True,
                )
    scripts = scripts_of(tree)
    by_file: dict[str, list[Script]] = {}
    for script in scripts:
        by_file.setdefault(script.source.rel, []).append(script)
    for parts in by_file.values():
        events = {m.group(1) for part in parts for m in LISTENER.finditer(part.code)}
        if events & set(UP_EVENTS):
            continue
        for part in parts:
            for match in LISTENER.finditer(part.code):
                if match.group(1) in DOWN_EVENTS:
                    emit(
                        findings,
                        waived,
                        part.source,
                        part.offset + match.start(),
                        f"acts on {match.group(1)} and nothing in the file listens for the up event",
                        True,
                    )
    examined += len(by_file)
    return Outcome(findings, examined, waived)


MOTION = re.compile(
    r"addEventListener\s*\(\s*['\"`](?:devicemotion|deviceorientation|deviceorientationabsolute)['\"`]|"
    r"\bondevice(?:motion|orientation)\s*=|\bnew\s+(?:Accelerometer|Gyroscope|LinearAccelerationSensor|"
    r"GravitySensor|AbsoluteOrientationSensor|RelativeOrientationSensor)\s*\(|"
    r"\b(?:Accelerometer|Gyroscope|DeviceOrientation)\.start\s*\("
)


def check_motion_actuation(tree: Tree) -> Outcome:
    findings: list[str] = []
    waived: list[str] = []
    scripts = scripts_of(tree)
    for script in scripts:
        for match in MOTION.finditer(script.code):
            emit(
                findings,
                waived,
                script.source,
                script.offset + match.start(),
                "responds to device motion; give it a control and a way to turn it off",
                True,
            )
    return Outcome(findings, len({s.source.rel for s in scripts}), waived)


SINGLE_KEY = re.compile(r"""(?:\.key\s*={2,3}\s*|case\s+)(['"])([^'"\\\s])\1""")
MODIFIERS = re.compile(r"\b(?:ctrlKey|metaKey|altKey)\b")
GLOBAL_KEY_LISTENER = re.compile(
    r"\b(?:window|document|document\.body|globalThis)\s*\.\s*addEventListener\s*\(\s*['\"`](keydown|keyup|keypress)['\"`]\s*,\s*"
)


def key_findings(code: str) -> list[str]:
    if MODIFIERS.search(code):
        return []
    return [match.group(2) for match in SINGLE_KEY.finditer(code)]


def check_key_shortcuts(tree: Tree) -> Outcome:
    findings: list[str] = []
    scripts = scripts_of(tree)
    examined = len({s.source.rel for s in scripts})
    for script in scripts:
        for match in GLOBAL_KEY_LISTENER.finditer(script.code):
            rest = script.code[match.end() :]
            if rest.startswith(("(", "function", "async")):
                brace = rest.find("{")
                body = (
                    rest[brace : read_braced(rest, brace)]
                    if brace >= 0
                    else rest.split("\n", 1)[0]
                )
            else:
                name = re.match(r"[A-Za-z_$][\w$]*", rest)
                body = (
                    function_body([(0, script.code)], name.group(0)) if name else None
                )
            for key in key_findings(body or ""):
                findings.append(
                    f"{script.source.at(script.offset + match.start())} a page-wide key handler acts on the single key {key!r}; let it be turned off, remapped or scoped to focus"
                )
    for source in markup_sources(tree):
        markup = markup_of(source)
        for element in markup.elements:
            if element.name not in (
                "svelte:window",
                "svelte:document",
                "svelte:body",
                "body",
            ):
                continue
            for _name, value in handler_attrs(element, KEY_EVENTS):
                for key in key_findings(handler_code(markup, value) or ""):
                    findings.append(
                        f"{source.at(element.start)} a page-wide key handler acts on the single key {key!r}; let it be turned off, remapped or scoped to focus"
                    )
    return Outcome(findings, max(examined, len(markup_sources(tree))))


# --- style classes --------------------------------------------------------------------------------

SPACING_PROPS = ("line-height", "letter-spacing", "word-spacing")


def check_inline_spacing(tree: Tree) -> Outcome:
    findings: list[str] = []
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            style = element.literal("style")
            if style:
                for name, _value, important in declarations(style):
                    if name in SPACING_PROPS and important:
                        findings.append(
                            f"{source.at(element.start)} {tag_label(element)} locks {name} with !important"
                        )
            for attr in element.attrs:
                match = re.fullmatch(r"style:([a-z-]+)((?:\|[a-z]+)*)", attr)
                if (
                    match
                    and match.group(1) in SPACING_PROPS
                    and "|important" in match.group(2)
                ):
                    findings.append(
                        f"{source.at(element.start)} {tag_label(element)} locks {match.group(1)} with !important"
                    )
    return Outcome(findings, len(sources))


BASIC_COLORS = {
    "black": "#000000",
    "silver": "#c0c0c0",
    "gray": "#808080",
    "grey": "#808080",
    "white": "#ffffff",
    "maroon": "#800000",
    "red": "#ff0000",
    "purple": "#800080",
    "fuchsia": "#ff00ff",
    "magenta": "#ff00ff",
    "green": "#008000",
    "lime": "#00ff00",
    "olive": "#808000",
    "yellow": "#ffff00",
    "navy": "#000080",
    "blue": "#0000ff",
    "teal": "#008080",
    "aqua": "#00ffff",
    "cyan": "#00ffff",
    "orange": "#ffa500",
}


def parse_color(
    value: str, variables: dict[str, str], depth: int = 0
) -> tuple[int, int, int] | None:
    """An opaque sRGB colour, or None when the value is not one a static read can resolve."""
    value = value.strip().lower()
    if depth > 4:
        return None
    var = re.fullmatch(r"var\(\s*(--[\w-]+)\s*(?:,\s*(.+))?\)", value)
    if var:
        if var.group(1) in variables:
            return parse_color(variables[var.group(1)], variables, depth + 1)
        return parse_color(var.group(2), variables, depth + 1) if var.group(2) else None
    if value in BASIC_COLORS:
        value = BASIC_COLORS[value]
    hexa = re.fullmatch(r"#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})", value)
    if hexa:
        digits = hexa.group(1)
        if len(digits) in (3, 4):
            digits = "".join(ch * 2 for ch in digits)
        if len(digits) == 8 and digits[6:] != "ff":
            return None
        return int(digits[0:2], 16), int(digits[2:4], 16), int(digits[4:6], 16)
    rgb = re.fullmatch(
        r"rgba?\(\s*([\d.]+%?)[\s,]+([\d.]+%?)[\s,]+([\d.]+%?)\s*(?:[,/]\s*([\d.]+%?)\s*)?\)",
        value,
    )
    if rgb:
        alpha = rgb.group(4)
        if (
            alpha is not None
            and (float(alpha.rstrip("%")) / (100 if alpha.endswith("%") else 1)) < 1
        ):
            return None
        channels = []
        for part in rgb.group(1, 2, 3):
            number = float(part.rstrip("%"))
            channels.append(
                round(number * 2.55) if part.endswith("%") else round(number)
            )
        return channels[0], channels[1], channels[2]
    return None


def luminance(rgb: tuple[int, int, int]) -> float:
    """Relative luminance, WCAG 2.2's definition."""
    out = []
    for channel in rgb:
        c = channel / 255
        out.append(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
    return 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2]


def contrast(a: tuple[int, int, int], b: tuple[int, int, int]) -> float:
    la, lb = sorted((luminance(a), luminance(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def px(value: str | None) -> float | None:
    if not value:
        return None
    match = re.fullmatch(r"([\d.]+)(px|rem|em|pt)", value.strip().lower())
    if not match:
        return None
    number = float(match.group(1))
    return {"px": number, "rem": number * 16, "em": number * 16, "pt": number * 4 / 3}[
        match.group(2)
    ]


def large_text(rule: Rule) -> bool:
    """WCAG's large-scale text: 18 point (24 px), or 14 point (about 18.66 px) and bold."""
    size = px(rule.value("font-size"))
    if size is None:
        return False
    weight = (rule.value("font-weight") or "").strip().lower()
    bold = weight == "bold" or (weight.isdigit() and int(weight) >= 700)
    return size >= 24 or (bold and size >= 18.66)


def hexed(rgb: tuple[int, int, int]) -> str:
    return "#%02x%02x%02x" % rgb


def check_contrast_pairs(tree: Tree) -> Outcome:
    rules, read = rules_of(tree)
    variables: dict[str, str] = {}
    for rule in rules:
        if any(sel in (":root", "html") for sel in rule.selectors) and not rule.media:
            for name, value, _important in rule.decls:
                if name.startswith("--"):
                    variables[name] = value
    findings: list[str] = []
    for rule in rules:
        if rule.at:
            continue
        fg_value = rule.value("color")
        bg_value = rule.value("background-color") or rule.value("background")
        if not fg_value or not bg_value:
            continue
        fg = parse_color(fg_value, variables)
        bg = parse_color(bg_value, variables)
        if fg is None or bg is None:
            continue
        need = 3.0 if large_text(rule) else 4.5
        ratio = contrast(fg, bg)
        if ratio < need:
            findings.append(
                f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` text {hexed(fg)} on {hexed(bg)} "
                f"is {ratio:.2f}:1, under {need:g}:1"
            )
    return Outcome(findings, read)


STUCK = re.compile(r"^(?:fixed|sticky|-webkit-sticky)$")
EDGE_CLASSES = re.compile(r"^(?:top|bottom|inset|inset-y|inset-x)-")
SCROLL_PADDING_CLASS = re.compile(r"^scroll-p[tby]?-")


def check_focus_not_obscured(tree: Tree) -> Outcome:
    rules, read = rules_of(tree)
    padded = any(
        name.startswith("scroll-padding")
        for rule in rules
        for name, _v, _i in rule.decls
    )
    sources = markup_sources(tree)
    class_sites: list[tuple[Source, Element, str, str]] = []
    for source in sources:
        for element in markup_of(source).elements:
            classes = (element.literal("class") or "").split()
            if any(SCROLL_PADDING_CLASS.match(c) for c in classes):
                padded = True
            position = next((c for c in classes if c in ("fixed", "sticky")), None)
            edge = next((c for c in classes if EDGE_CLASSES.match(c)), None)
            if position and edge:
                class_sites.append((source, element, position, edge.split("-")[0]))
    findings: list[str] = []
    if not padded:
        for rule in rules:
            position = (rule.value("position") or "").strip().lower()
            if not STUCK.match(position):
                continue
            inset = rule.value("inset")
            top = rule.value("top") is not None
            bottom = rule.value("bottom") is not None
            if inset is not None and inset.split()[0] in ("0", "0px"):
                continue
            if top and bottom:
                continue
            if not (top or bottom or inset is not None):
                continue
            edge = "top" if top else "bottom" if bottom else "inset"
            findings.append(
                f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` is {position.replace('-webkit-', '')} "
                f"at the {edge} edge and no rule sets scroll-padding, so it can cover a focused control"
            )
        for source, element, position, edge in class_sites:
            if edge == "inset":
                continue
            findings.append(
                f"{source.at(element.start)} {tag_label(element)} is {position} at the {edge} edge and no rule "
                "sets scroll-padding, so it can cover a focused control"
            )
    return Outcome(findings, read + len(sources))


REVEAL = re.compile(r"^(?:display|visibility|opacity)$")
FOCUS_TWINS = (":focus-within", ":focus-visible", ":focus")
TW_REVEAL = re.compile(
    r"^(group-hover|peer-hover|hover):(block|flex|grid|inline|inline-block|inline-flex|table|visible|opacity-100)$"
)


def reveals(rule: Rule) -> bool:
    for name, value, _important in rule.decls:
        if not REVEAL.match(name):
            continue
        value = value.strip().lower()
        if name == "display" and value != "none":
            return True
        if name == "visibility" and value == "visible":
            return True
        if name == "opacity":
            try:
                if float(value) > 0:
                    return True
            except ValueError:
                return True
    return False


def check_hover_reveal(tree: Tree) -> Outcome:
    rules, read = rules_of(tree)
    selectors = {sel for rule in rules for sel in rule.selectors}
    findings: list[str] = []
    for rule in rules:
        if not reveals(rule):
            continue
        for selector in rule.selectors:
            if ":hover" not in selector:
                continue
            twins = {selector.replace(":hover", twin) for twin in FOCUS_TWINS}
            if twins & selectors:
                continue
            findings.append(
                f"{rule.source.at(rule.offset)} `{selector}` reveals content on hover and no :focus rule "
                "reveals it for the keyboard"
            )
    sources = markup_sources(tree)
    for source in sources:
        for element in markup_of(source).elements:
            classes = (element.literal("class") or "").split()
            for token in classes:
                match = TW_REVEAL.match(token)
                if not match:
                    continue
                prefix = match.group(1).replace("hover", "")
                wanted = {
                    f"{prefix}{variant}:{match.group(2)}"
                    for variant in ("focus", "focus-within", "focus-visible")
                }
                if not wanted.intersection(classes):
                    findings.append(
                        f"{source.at(element.start)} {tag_label(element)} reveals on {token} with no focus variant"
                    )
    return Outcome(findings, read + len(sources))


DURATION = re.compile(r"(?<![\w.-])([\d.]+)(ms|s)\b")


def seconds(token: str) -> float | None:
    match = DURATION.fullmatch(token.strip())
    if not match:
        return None
    value = float(match.group(1))
    return value / 1000 if match.group(2) == "ms" else value


def check_flash_rate(tree: Tree) -> Outcome:
    rules, read = rules_of(tree)
    findings: list[str] = []
    for rule in rules:
        if rule.at:
            continue
        duration = None
        count: float | None = 1
        shorthand = rule.value("animation")
        if shorthand:
            for token in shorthand.split(","):
                words = token.split()
                times = [seconds(w) for w in words if seconds(w) is not None]
                if times:
                    duration = times[0]
                if "infinite" in words:
                    count = float("inf")
                else:
                    numbers = [w for w in words if re.fullmatch(r"\d+(?:\.\d+)?", w)]
                    count = float(numbers[0]) if numbers else 1
        explicit = rule.value("animation-duration")
        if explicit and seconds(explicit.split(",")[0]) is not None:
            duration = seconds(explicit.split(",")[0])
        iterations = rule.value("animation-iteration-count")
        if iterations:
            first = iterations.split(",")[0].strip()
            count = (
                float("inf")
                if first == "infinite"
                else float(first)
                if re.fullmatch(r"\d+(?:\.\d+)?", first)
                else count
            )
        if not duration or duration <= 0 or count is None or count <= 3:
            continue
        rate = 1 / duration
        if rate > 3:
            findings.append(
                f"{rule.source.at(rule.offset)} `{', '.join(rule.selectors)}` cycles {rate:.1f} times a second; "
                "if it changes brightness or red it can pass three flashes a second"
            )
    return Outcome(findings, read)


# --- toolchain classes ----------------------------------------------------------------------------

SVELTE_CONFIG = re.compile(r"^(?:svelte|vite)\.config\.[cm]?[jt]s$")
FILTER_HEAD = re.compile(r"\b(?:warningFilter|onwarn)\s*[:=(]")
COMPILER_WARNINGS = re.compile(r"--compiler-warnings[=\s]+(['\"]?)([^'\"\n]+)\1")
IGNORE_COMMENT = re.compile(r"svelte-ignore\s+([^\n]*)")
# A filter that DROPS a11y warnings, not one that merely names them: a negated a11y match
# (`!w.code.startsWith('a11y')`), an inequality with an a11y code, or an early return on one.
DROP_A11Y = re.compile(r"!\s*/\^?a11y|!\s*[\w$.]+\s*\(\s*['\"`]a11y|!==?\s*['\"`]a11y")


def closing_paren(text: str, at: int) -> int:
    """The index just past the `)` that closes the `(` at `at`, honouring quotes."""
    depth = 0
    quote = None
    for i in range(at, len(text)):
        ch = text[i]
        if quote:
            if ch == quote:
                quote = None
        elif ch in "\"'`":
            quote = ch
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return i + 1
    return len(text)


def drops_a11y(body: str) -> bool:
    """Whether a warning filter's code drops a11y warnings rather than merely naming them."""
    if DROP_A11Y.search(body):
        return True
    for match in re.finditer(r"\bif\s*\(", body):
        end = closing_paren(body, match.end() - 1)
        if "a11y" not in body[match.end() : end]:
            continue
        after = body[end:].lstrip()
        after = after[1:].lstrip() if after.startswith("{") else after
        if re.match(r"return\b(?!\s+true)", after):
            return True
    return False


def svelte_tree(tree: Tree) -> bool:
    if any(source.ext == ".svelte" for source in tree.files):
        return True
    return any('"svelte"' in source.text for source in tree.packages())


def svelte_check_lines(tree: Tree) -> list[tuple[Source, int, str, str | None]]:
    """(file, offset, the command, the package script's name) for every svelte-check invocation."""
    found = []
    for source in tree.packages():
        try:
            data = json.loads(source.text)
        except ValueError:
            continue
        scripts = data.get("scripts") if isinstance(data, dict) else None
        for name, command in (scripts or {}).items():
            if isinstance(command, str) and "svelte-check" in command:
                found.append(
                    (source, source.text.find(json.dumps(command)[1:-1]), command, name)
                )
    for source in tree.workflows():
        for match in re.finditer(r"[^\n]*svelte-check[^\n]*", source.text):
            found.append((source, match.start(), match.group(0), None))
    return found


def check_svelte_a11y_not_disabled(tree: Tree) -> Outcome:
    findings: list[str] = []
    examined = 0
    for source in tree.files:
        if not SVELTE_CONFIG.match(Path(source.rel).name):
            continue
        examined += 1
        code = strip_js_comments(source.text)
        for match in FILTER_HEAD.finditer(code):
            rest = code[match.end() :]
            brace = rest.find("{")
            arrow = rest.find("=>")
            if 0 <= arrow < (brace if brace >= 0 else len(rest)) and not rest[
                arrow + 2 :
            ].lstrip().startswith("{"):
                end = min(
                    (i for i in (rest.find(","), rest.find("\n")) if i >= 0),
                    default=len(rest),
                )
                body = rest[:end]
            else:
                body = (
                    rest[brace : read_braced(rest, brace)] if brace >= 0 else rest[:200]
                )
            if drops_a11y(body):
                findings.append(
                    f"{source.at(match.start())} a warning filter drops the compiler's a11y warnings"
                )
    for source, _offset, command, _name in svelte_check_lines(tree):
        examined += 1
        for match in COMPILER_WARNINGS.finditer(command):
            for pair in match.group(2).split(","):
                code, _, level = pair.strip().partition(":")
                if code.startswith(("a11y_", "a11y-")) and level.strip() == "ignore":
                    findings.append(
                        f"{source.rel} ignores {code}, one of the compiler's a11y warnings"
                    )
    # A tree without Svelte owes nothing here, but the files read to decide that still count, so
    # a scan that read nothing is VOID rather than green.
    examined += len(tree.markup()) + len(tree.packages()) + len(tree.workflows())
    return Outcome(findings, examined)


def check_svelte_a11y_fails_ci(tree: Tree) -> Outcome:
    findings: list[str] = []
    examined = 0
    for source, offset, command, name in svelte_check_lines(tree):
        examined += 1
        if "--fail-on-warnings" in command:
            continue
        if name is not None:
            findings.append(
                f"{source.rel} script {name} runs svelte-check without --fail-on-warnings"
            )
        else:
            findings.append(
                f"{source.at(offset)} runs svelte-check without --fail-on-warnings"
            )
    for source in tree.markup():
        examined += 1
        for offset, comment in markup_of(source).comments:
            match = IGNORE_COMMENT.search(comment)
            if not match:
                continue
            codes = [
                c.strip()
                for c in re.split(r"[,\s]+", match.group(1).split("(")[0])
                if c.strip()
            ]
            if any(
                c.startswith(("a11y_", "a11y-")) for c in codes
            ) and "(" not in match.group(1):
                findings.append(
                    f"{source.at(offset)} svelte-ignore {', '.join(codes)} gives no reason"
                )
    return Outcome(findings, examined)


AXE_IMPORT = re.compile(
    r"""@axe-core/(?:playwright|webdriverjs|puppeteer)|['"]axe-core['"]|vitest-axe|jest-axe|axe-playwright"""
)
WCAG_TAGS = ("wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa")
TAGS_CALL = re.compile(
    r"(?:withTags|runOnly)\s*[:(]\s*(\[[^\]]*\]|\{[^}]*\}|['\"][^'\"]*['\"]|[A-Za-z_$][\w$]*)",
    re.S,
)


def tag_list(text: str, argument: str) -> str | None:
    """The tags an audit passes: a literal, or the array a same-file constant of that name holds."""
    if not re.fullmatch(r"[A-Za-z_$][\w$]*", argument):
        return argument
    bound = re.search(
        rf"(?:const|let|var)\s+{re.escape(argument)}\s*(?::[^=]+)?=\s*(\[[^\]]*\])",
        text,
    )
    return bound.group(1) if bound else None


PLAYWRIGHT_RUN = re.compile(r"playwright\s+test")


def check_runtime_audit(tree: Tree) -> Outcome:
    findings: list[str] = []
    tests = [s for s in tree.files if s.test and s.ext in SCRIPT_EXT]
    examined = len(tests) + len(tree.markup())
    audits = [s for s in tests if AXE_IMPORT.search(s.text)]
    if not tree.markup():
        return Outcome(findings, examined)
    if not audits:
        findings.append(
            "no test runs axe-core over a rendered page (@axe-core/playwright with WCAG 2.2 AA tags)"
        )
        return Outcome(findings, examined)
    for source in audits:
        for match in TAGS_CALL.finditer(source.text):
            listed = tag_list(source.text, match.group(1))
            if listed is None:
                continue
            named = set(re.findall(r"wcag\d+a+", listed))
            missing = [tag for tag in WCAG_TAGS if tag not in named]
            if missing:
                findings.append(
                    f"{source.at(match.start())} the axe audit's tags leave out {', '.join(missing)}"
                )
    runs = [s for s in tree.workflows() if PLAYWRIGHT_RUN.search(s.text)]
    scripted = {
        name
        for source in tree.packages()
        for name, command in (
            (
                json.loads(source.text) if source.text.strip().startswith("{") else {}
            ).get("scripts")
            or {}
        ).items()
        if isinstance(command, str) and PLAYWRIGHT_RUN.search(command)
    }
    via_script = any(
        re.search(rf"\b(?:run\s+)?{re.escape(name)}\b", workflow.text)
        for workflow in tree.workflows()
        for name in scripted
    )
    if not runs and not via_script:
        findings.append(
            "no workflow runs the audit (playwright test), so it gates nothing"
        )
    return Outcome(findings, examined)


# --- the command line -----------------------------------------------------------------------------

TREE_CHECKS: dict[str, Callable[[Tree], Outcome]] = {
    "label-in-name": check_label_in_name,
    "aria-valid": check_aria_valid,
    "aria-hidden-focus": check_aria_hidden_focus,
    "keyboard-operable": check_keyboard_operable,
    "video-captions": check_video_captions,
    "meta-refresh": check_meta_refresh,
    "context-change": check_context_change,
    "orientation-lock": check_orientation_lock,
    "auth-autofill": check_auth_autofill,
    "status-messages": check_status_messages,
    "pointer-cancel": check_pointer_cancel,
    "motion-actuation": check_motion_actuation,
    "key-shortcuts": check_key_shortcuts,
    "inline-spacing": check_inline_spacing,
    "contrast-pairs": check_contrast_pairs,
    "focus-not-obscured": check_focus_not_obscured,
    "hover-reveal": check_hover_reveal,
    "flash-rate": check_flash_rate,
    "svelte-a11y-not-disabled": check_svelte_a11y_not_disabled,
    "svelte-a11y-fails-ci": check_svelte_a11y_fails_ci,
    "runtime-audit": check_runtime_audit,
}
CONTEXT_CHECKS: dict[str, Callable[[Context], Outcome]] = {
    "matrix-complete": check_matrix_complete,
    "matrix-rows-resolve": check_matrix_rows_resolve,
}


def run_check(context: Context, name: str) -> int:
    try:
        if name in CONTEXT_CHECKS:
            outcome = CONTEXT_CHECKS[name](context)
        else:
            outcome = TREE_CHECKS[name](scan(context.root))
    except Void as void:
        print(f"{name}: VOID {void}")
        print("examined 0")
        return EXIT_VOID
    for finding in outcome.findings:
        print(f"{name}: {finding}")
    for site in outcome.waived:
        print(f"waived {site}")
    print(f"examined {outcome.examined}")
    if outcome.examined == 0:
        return EXIT_VOID
    return EXIT_FINDING if outcome.findings else EXIT_GREEN


def parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", type=Path, default=argparse.SUPPRESS)
    common.add_argument("--skills", type=Path, default=argparse.SUPPRESS)
    top = argparse.ArgumentParser(
        description="Judge a web tree against WCAG 2.2 AA, statically.",
        parents=[common],
    )
    verbs = top.add_subparsers(dest="verb", required=True)
    check = verbs.add_parser("check", parents=[common], help="run one class")
    check.add_argument("klass", metavar="CLASS", choices=CLASSES)
    verbs.add_parser("classes", help="print the classes, one per line")
    verbs.add_parser(
        "criteria", help="print WCAG 2.2's A and AA criteria: sc, level, name"
    )
    return top


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    if args.verb == "classes":
        for name in CLASSES:
            print(name)
        return EXIT_GREEN
    if args.verb == "criteria":
        for sc, name, level in WCAG22_AA:
            print(f"{sc}\t{level}\t{name}")
        return EXIT_GREEN
    root = Path(getattr(args, "root", Path("."))).resolve()
    if not root.is_dir():
        print(f"accessibility-probe: --root {root} is not a directory", file=sys.stderr)
        return EXIT_USAGE
    skills = Path(getattr(args, "skills", DEFAULT_SKILLS)).resolve()
    return run_check(Context(root=root, skills=skills), args.klass)


if __name__ == "__main__":
    sys.exit(main())

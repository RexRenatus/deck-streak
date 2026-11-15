"""The architecture page's code map and drawing, and the context map's units, held to the tree and
to the deprecation of the Telegram surfaces (SPEC-623).

The code map is the table under ARCHITECTURE.md's `## Code map` heading; a row's paths are the
code spans of its first cell, a brace group expanded. A unit is a table row, a list item with its
indented continuation lines, a paragraph (a heading included), or a context-map fence row's
description; fenced blocks are skipped except the context-map fence, and the drawing is read
alone. Code spans are removed before any match.
"""

import re
import unittest

from _support import REPO, examined

ARCH = REPO / "ARCHITECTURE.md"
MAP = REPO / "docs" / "CONTEXT-MAP.md"
FENCE = "`" * 3
CODE_MAP = "## Code map"
SURFACE = re.compile(r"mini[\s-]?app|telegram|\bbots?\b|initdata", re.IGNORECASE)
DEPRECATED = re.compile(r"deprecat", re.IGNORECASE)
SPAN = re.compile(r"`[^`]*`")
CELL_PATH = re.compile(r"`([^`]+)`")
BRACE = re.compile(r"^(.*)\{([^}]*)\}(.*)$")
LIST_ITEM = re.compile(r"^\s*(?:- |\d+\. )")
ROW = re.compile(r"^(\S+)\s+(?:(\S+)\s+)?\((.*)\)\s+depends on: (.*)$")
IOS_PATH = re.compile(r"ios/[A-Za-z0-9_-]+")
PREFIX = "deck-streak-"
LABEL = re.compile(r"\[([^\]]*)\]")
APP_LABELS = ("web app", "iPhone and iPad app")

CUT_DRAWING = "\n".join(
    [
        FENCE + "mermaid",
        "flowchart LR",
        "  sync[Anki sync server] --> ingest",
        "  ingest --> coordination",
        "  coordination --> domain[domain contexts]",
        "  coordination --> agent",
        "  agent -.->|when enabled| proxy[subscription proxy]",
        "  coordination --> vault[(second-brain vault)]",
        "  api --> coordination",
        "  bot --> coordination",
        "  miniapp[Mini App] --> api",
        "  telegram[Telegram] <--> bot",
        FENCE,
    ]
)


def tree_exists(path):
    return (REPO / path).exists()


def crate_folders():
    """Every folder under `crates/`, read from the file system (SPEC-623 R1)."""
    return sorted(p.name for p in (REPO / "crates").iterdir() if p.is_dir())


def expand(path):
    match = BRACE.match(path)
    if not match:
        return [path]
    head, parts, tail = match.groups()
    return [head + part.strip() + tail for part in parts.split(",") if part.strip()]


def code_map_rows(text):
    """`(line number, paths, description)` for each row of the code map's table, the header and
    its rule left out (SPEC-623 R1, R3)."""
    lines = text.split("\n")
    start = next((i for i, line in enumerate(lines) if line.strip() == CODE_MAP), None)
    if start is None:
        return []
    rows = []
    for number, line in enumerate(lines[start + 1 :], start + 2):
        if line.startswith("## "):
            break
        if not line.startswith("|"):
            continue
        cells = line.split("|")
        if len(cells) < 3 or set(cells[1].strip()) <= set("-: "):
            continue
        paths = [p for span in CELL_PATH.findall(cells[1]) for p in expand(span)]
        if paths == ["path"] or cells[1].strip() == "path":
            continue
        rows.append((number, paths, "|".join(cells[2:])))
    return rows


def crate_findings(text, folders):
    """The crate folders the code map names zero times or more than once, and the crate folders
    it names that the tree lacks (SPEC-623 R1)."""
    examined("crate folders", folders)
    named = [p for _, paths, _ in examined("code-map rows", code_map_rows(text)) for p in paths]
    counts = {}
    for path in named:
        match = re.match(r"^crates/([^/{}]+)/$", path)
        if match:
            counts[match.group(1)] = counts.get(match.group(1), 0) + 1
    findings = []
    for folder in folders:
        if folder not in counts:
            findings.append(f"the code map does not name crates/{folder}/")
        elif counts[folder] > 1:
            findings.append(f"the code map names crates/{folder}/ {counts[folder]} times")
    for name in sorted(set(counts) - set(folders)):
        findings.append(f"the code map names crates/{name}/, which the tree lacks")
    return findings


def ios_row_findings(text, exists):
    """The code map's one `ios/` row names the app and the harness, each in the tree (SPEC-623
    R2)."""
    rows = [row for row in examined("code-map rows", code_map_rows(text)) if row[1] == ["ios/"]]
    if len(rows) != 1:
        return [f"code-map rows whose path cell is ios/: {len(rows)}"]
    return named_ios_findings("the code map's ios/ row", rows[0][2], exists)


def named_ios_findings(what, description, exists):
    names = sorted(set(IOS_PATH.findall(description)))
    findings = [
        f"{what} names no {want}" for want in ("ios/App", "ios/Harness") if want not in names
    ]
    findings += [f"{what} names {name}, which the tree lacks" for name in names if not exists(name)]
    return findings


def fence_rows(text):
    """`(line number, name, path, description)` for each row of the context-map fence."""
    lines = text.split("\n")
    start = next((i for i, line in enumerate(lines) if line.strip() == FENCE + "context-map"), None)
    rows = []
    if start is None:
        return rows
    for number, line in enumerate(lines[start + 1 :], start + 2):
        if line.strip() == FENCE:
            break
        match = ROW.match(line)
        if match:
            rows.append((number, match.group(1), match.group(2), match.group(3)))
        elif line.strip():
            rows.append((number, line.split()[0], None, line))
    return rows


def path_findings(arch_text, map_text, exists):
    """Every code-map path is in the tree, or its row says `(planned)` and a fence path row under
    it says planned (SPEC-623 R3)."""
    planned = [
        path.rstrip("/")
        for _, name, path, note in fence_rows(map_text)
        if path is not None and "planned" in note
    ]
    findings = []
    for _, paths, description in examined("code-map rows", code_map_rows(arch_text)):
        for path in examined("code-map paths of a row", paths):
            if exists(path):
                continue
            base = path.rstrip("/")
            held = any(p == base or p.startswith(base + "/") for p in planned)
            if not ("(planned)" in description and held):
                findings.append(
                    f"the code map names {path}, which the tree lacks and no planned row holds"
                )
    return findings


def units(text):
    """`(line number, text, first line)` for each unit of a page (SPEC-623 R4, R6)."""
    found, current, kind, block = [], None, None, None

    def close():
        if current is not None:
            found.append((current[0], "\n".join(current[1]), current[1][0]))

    for number, line in enumerate(text.split("\n"), 1):
        stripped = line.strip()
        if block is not None:
            if stripped == FENCE:
                block = None
            elif block == "context-map" and stripped:
                match = ROW.match(line)
                note = match.group(3) if match else line
                name = match.group(1) if match else stripped.split()[0]
                found.append((number, note, f"{name} ({note})"))
            continue
        if stripped.startswith(FENCE):
            close()
            current, kind, block = None, None, stripped[len(FENCE) :]
            continue
        if not stripped:
            close()
            current, kind = None, None
        elif line.startswith("|"):
            close()
            found.append((number, line, line))
            current, kind = None, None
        elif LIST_ITEM.match(line):
            close()
            current, kind = (number, [line]), "item"
        elif current is not None and (kind == "para" or line[:1].isspace()):
            current[1].append(line)
        else:
            close()
            current, kind = (number, [line]), "para"
    close()
    return found


def surface_findings(text):
    """Each unit that names a Telegram surface, code spans set aside, without the word deprecated
    (SPEC-623 R4, R6)."""
    findings = []
    for number, unit, first in examined("units", units(text)):
        bare = SPAN.sub("", unit)
        if SURFACE.search(bare) and not DEPRECATED.search(bare):
            findings.append(f"line {number}: {first.strip()[:100]}")
    return findings


def drawing_findings(text):
    """The first mermaid block draws the two apps and no Telegram surface (SPEC-623 R5)."""
    lines = text.split("\n")
    start = next((i for i, line in enumerate(lines) if line.strip() == FENCE + "mermaid"), None)
    if start is None:
        return ["the page has no mermaid drawing"]
    body = []
    for line in lines[start + 1 :]:
        if line.strip() == FENCE:
            break
        body.append(line)
    examined("drawing lines", body)
    findings = [f"the drawing draws {line.strip()}" for line in body if SURFACE.search(line)]
    labels = [label for line in body for label in LABEL.findall(line)]
    findings += [
        f"the drawing labels no {want}"
        for want in APP_LABELS
        if not any(want in label for label in labels)
    ]
    return findings


def harness_findings(text, exists):
    """The fence's `ios-harness  ios/` row names the app at ios/App and the harness at ios/Harness,
    each in the tree, with no backtick (SPEC-623 R7)."""
    rows = [row for row in examined("fence rows", fence_rows(text)) if row[1] == "ios-harness"]
    if len(rows) != 1 or rows[0][2] != "ios/":
        return [f"fence rows named ios-harness at ios/: {len(rows)}"]
    note = rows[0][3]
    findings = named_ios_findings("the ios-harness row", note, exists)
    if "`" in note:
        findings.append("the ios-harness row holds a backtick")
    return findings


def fence_path_findings(text, exists):
    """Every fence path row names a path in the tree or says planned (SPEC-623 R8)."""
    rows = [
        row for row in examined("fence rows", fence_rows(text)) if not row[1].startswith(PREFIX)
    ]
    findings = []
    for _, name, path, note in examined("fence path rows", rows):
        if path is None:
            findings.append(f"the fence row {name} has no path")
        elif not exists(path) and "planned" not in note:
            findings.append(
                f"the fence path row {name} names {path}, which the tree lacks and its row does not plan"
            )
    return findings


def planted_code_map(*rows):
    return "\n".join(
        [
            "# Architecture",
            "",
            CODE_MAP,
            "",
            "| path | what lives there |",
            "|---|---|",
            *rows,
            "",
            "## Invariants",
        ]
    )


def planted_fence(*rows):
    return "\n".join(["# Context map", "", FENCE + "context-map", *rows, FENCE, ""])


class TheCodeMap(unittest.TestCase):
    maxDiff = None

    def test_the_code_map_names_every_crate_folder_exactly_once(self):
        """SPEC-623 R1: every crate folder once, alone or in a brace group, and none the tree lacks."""
        folders = ["api", "bot", "ingest", "kernel", "xp"]
        good = [
            "| `crates/kernel/` | k |",
            "| `crates/{ingest,xp}/` | g |",
            "| `crates/api/`, `crates/bot/` | a |",
        ]
        self.assertEqual([], crate_findings(planted_code_map(*good), folders))
        missing = [good[0], "| `crates/ingest/` | g |", good[2]]
        self.assertEqual(
            ["the code map does not name crates/xp/"],
            crate_findings(planted_code_map(*missing), folders),
        )
        ghost = [*good, "| `crates/ghost/` | g |"]
        self.assertEqual(
            ["the code map names crates/ghost/, which the tree lacks"],
            crate_findings(planted_code_map(*ghost), folders),
        )
        twice = [*good, "| `crates/xp/` | x |"]
        self.assertEqual(
            ["the code map names crates/xp/ 2 times"],
            crate_findings(planted_code_map(*twice), folders),
        )
        self.assertEqual([], crate_findings(ARCH.read_text(encoding="utf-8"), crate_folders()))

    def test_the_code_map_has_a_row_for_the_iphone_and_ipad_app_folder(self):
        """SPEC-623 R2: one `ios/` row naming ios/App and ios/Harness, each in the tree."""
        absent = planted_code_map("| `crates/kernel/` | k |")
        self.assertEqual(
            ["code-map rows whose path cell is ios/: 0"], ios_row_findings(absent, tree_exists)
        )
        ghost = planted_code_map(
            "| `ios/` | the app (`ios/App`), the harness (`ios/Harness`), `ios/Ghost` |"
        )
        self.assertEqual(
            ["the code map's ios/ row names ios/Ghost, which the tree lacks"],
            ios_row_findings(ghost, tree_exists),
        )
        self.assertEqual([], ios_row_findings(ARCH.read_text(encoding="utf-8"), tree_exists))

    def test_every_code_map_path_exists_or_is_planned_in_the_context_map(self):
        """SPEC-623 R3: every code-map path is in the tree, or planned on both pages."""
        fence = planted_fence(
            "landing   web/site/src    (the landing page, planned)  depends on: nothing internal"
        )
        rows = ["| `crates/kernel/` | k |", "| `web/site/` | the landing page (planned) |"]
        self.assertEqual([], path_findings(planted_code_map(*rows), fence, tree_exists))
        ghost = planted_code_map(*rows, "| `web/ghost/` | a ghost |")
        self.assertEqual(
            ["the code map names web/ghost/, which the tree lacks and no planned row holds"],
            path_findings(ghost, fence, tree_exists),
        )
        self.assertEqual(
            [],
            path_findings(
                ARCH.read_text(encoding="utf-8"), MAP.read_text(encoding="utf-8"), tree_exists
            ),
        )


class TheArchitecturePage(unittest.TestCase):
    maxDiff = None

    def test_the_architecture_names_a_telegram_surface_only_beside_deprecated(self):
        """SPEC-623 R4: outside the drawing, a unit naming a Telegram surface says deprecated."""
        planted = "\n".join(
            [
                "The bot sends the digest.",
                "",
                "A paragraph that names the Mini",
                "App across a line.",
                "",
                "- The Telegram bot is deprecated.",
                "",
                "The `bot` crate holds code.",
            ]
        )
        self.assertEqual(
            ["line 1: The bot sends the digest.", "line 3: A paragraph that names the Mini"],
            surface_findings(planted),
        )
        self.assertEqual([], surface_findings(ARCH.read_text(encoding="utf-8")))

    def test_the_drawing_shows_the_two_apps_and_no_telegram_surface(self):
        """SPEC-623 R5: the drawing labels the two apps and draws no Telegram node or edge."""
        self.assertEqual(
            [
                "the drawing draws bot --> coordination",
                "the drawing draws miniapp[Mini App] --> api",
                "the drawing draws telegram[Telegram] <--> bot",
                "the drawing labels no web app",
                "the drawing labels no iPhone and iPad app",
            ],
            drawing_findings(CUT_DRAWING),
        )
        self.assertEqual([], drawing_findings(ARCH.read_text(encoding="utf-8")))


class TheContextMap(unittest.TestCase):
    maxDiff = None

    def test_the_context_map_names_a_telegram_surface_only_beside_deprecated(self):
        """SPEC-623 R6: a context-map unit naming a Telegram surface says deprecated; a fence row's
        name and edge list are set aside."""
        planted = planted_fence(
            "deck-streak-bot           (Telegram Bot API adapter, deprecated)  depends on: kernel",
            "deck-streak-relay         (the Telegram relay)  depends on: kernel, bot",
        )
        self.assertEqual(
            ["line 5: deck-streak-relay (the Telegram relay)"], surface_findings(planted)
        )
        self.assertEqual([], surface_findings(MAP.read_text(encoding="utf-8")))

    def test_the_context_maps_ios_row_names_the_app_and_the_harness(self):
        """SPEC-623 R7: the ios-harness row names ios/App and ios/Harness, each in the tree."""
        ghost = planted_fence(
            "ios-harness  ios/  (the app at ios/App, the harness at ios/Harness, and ios/Ghost)  depends on: nothing internal"
        )
        self.assertEqual(
            ["the ios-harness row names ios/Ghost, which the tree lacks"],
            harness_findings(ghost, tree_exists),
        )
        self.assertEqual([], harness_findings(MAP.read_text(encoding="utf-8"), tree_exists))

    def test_every_context_map_path_row_exists_or_is_planned(self):
        """SPEC-623 R8: every fence path row names a path in the tree or says planned."""
        planted = planted_fence(
            "later   web/later/src   (a later front end, planned)  depends on: nothing internal",
            "ghost   web/ghost/src   (a ghost front end)  depends on: nothing internal",
        )
        self.assertEqual(
            [
                "the fence path row ghost names web/ghost/src, which the tree lacks and its row does not plan"
            ],
            fence_path_findings(planted, tree_exists),
        )
        self.assertEqual([], fence_path_findings(MAP.read_text(encoding="utf-8"), tree_exists))


if __name__ == "__main__":
    unittest.main()

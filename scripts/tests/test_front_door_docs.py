"""The front door names DeckStreak's two apps and its merge queue, and the docs index lists every
living doc, whose relative links all resolve (SPEC-622, ADR-644).

Every function takes a root and the tracked names, so a planted tree in a temporary folder needs
no git; only the production calls list the tree, by `git ls-files`, names only. Only living docs
are opened, and a link's target only when the link carries a fragment and the target is a Markdown
file.
"""

import posixpath
import re
import subprocess
import tempfile
import unittest
from pathlib import Path
from urllib.parse import unquote

from _support import REPO, examined

INDEX = "docs/README.md"
QUEUE_FILE = ".mergify.yml"
NOT_LIVING = (
    "docs/specs/",
    "docs/decisions/",
    "docs/schematics/",
    "docs/red-first/",
    "docs/rulings/",
    "changelog.d/",
    "agent/",
)
RECORD_FOLDERS = (
    "docs/decisions",
    "docs/specs",
    "docs/specs/planned",
    "docs/schematics",
    "docs/red-first",
    "docs/rulings",
    "changelog.d",
)
DEPRECATED_SURFACE = re.compile(r"mini[\s-]?app|telegram|\bbots?\b|initdata", re.IGNORECASE)
SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
LIST_ITEM = re.compile(r"^\s*(?:[-*+]|[0-9]+[.)])\s+")
HEADING = re.compile(r"^ {0,3}(#{1,6})(?:\s+(.*?))?(?:\s+#+)?\s*$")
INLINE_LINK = re.compile(r"\]\(\s*(<[^>\n]*>|[^)\s]+)(?:\s+(?:\"[^\"]*\"|'[^']*'|\([^)]*\)))?\s*\)")
REFERENCE = re.compile(r"^ {0,3}\[[^\]\n]+\]:\s*(<[^>\n]*>|\S+)", re.MULTILINE)
HTML_TARGET = re.compile(r"""\b(?:href|src)\s*=\s*(?:"([^"]*)"|'([^']*)')""", re.IGNORECASE)
HTML_ANCHOR = re.compile(r"""\b(?:id|name)\s*=\s*(?:"([^"]*)"|'([^']*)')""", re.IGNORECASE)
CODE_SPAN = re.compile(r"(?<!`)(`+)(?!`)(.+?)(?<!`)\1(?!`)", re.DOTALL)
EMPHASIS = re.compile(r"\*\*|__|\*|(?<!\w)_|_(?!\w)")
NUMBER_WORDS = {
    1: "one",
    2: "two",
    3: "three",
    4: "four",
    5: "five",
    6: "six",
    7: "seven",
    8: "eight",
    9: "nine",
    10: "ten",
    11: "eleven",
    12: "twelve",
}
MERGE_METHODS = {
    "merge": "merges each by a merge commit",
    "squash": "squashes each into one commit",
    "rebase": "rebases each onto `dev`",
    "fast-forward": "fast-forwards `dev` to each",
}


def tracked_names(root):
    """Every tracked path at `root`, names only."""
    listing = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"], capture_output=True, text=True, check=True
    )
    return [name for name in listing.stdout.split("\0") if name]


def living_docs(names):
    """The living docs among `names` (SPEC-622 R3), the docs index itself excepted."""
    docs = []
    for name in names:
        folders = name.split("/")[:-1]
        if not name.endswith(".md") or name.startswith(NOT_LIVING) or name == INDEX:
            continue
        if "tests" in folders or "fixtures" in folders:
            continue
        docs.append(name)
    return docs


def folders_of(names):
    """Every folder that holds a tracked name, the root as `.` included."""
    folders = {"."}
    for name in names:
        parts = name.split("/")[:-1]
        for end in range(1, len(parts) + 1):
            folders.add("/".join(parts[:end]))
    return folders


def outside_code(text):
    """The text with fenced blocks, inline code spans and HTML comments blanked, line breaks kept."""
    text = re.sub(r"<!--[\s\S]*?-->", lambda match: "\n" * match.group(0).count("\n"), text)
    lines, fence = [], None
    for line in text.split("\n"):
        opening = FENCE.match(line)
        if fence:
            if opening and opening.group(1)[0] == fence[0] and line.strip() == opening.group(1):
                if len(opening.group(1)) >= len(fence):
                    fence = None
            lines.append("")
            continue
        if opening:
            fence = opening.group(1)
            lines.append("")
            continue
        lines.append(line)
    paragraphs = re.split(r"(\n[ \t]*\n)", "\n".join(lines))
    return "".join(
        CODE_SPAN.sub(lambda match: "\n" * match.group(0).count("\n"), part) for part in paragraphs
    )


def destinations(text):
    """Each link destination of a Markdown text: inline links and images, reference definitions
    and HTML `href` and `src` values, outside fenced code and inline code spans."""
    prose = outside_code(text)
    found = [match.group(1) for match in INLINE_LINK.finditer(prose)]
    found += [match.group(1) for match in REFERENCE.finditer(prose)]
    found += [match.group(1) or match.group(2) for match in HTML_TARGET.finditer(prose)]
    return [dest[1:-1] if dest.startswith("<") and dest.endswith(">") else dest for dest in found]


def resolve(doc, dest):
    """(path, fragment) a relative destination names from `doc`, or None for one with a scheme."""
    if SCHEME.match(dest) or dest.startswith("//"):
        return None
    path, _, fragment = dest.partition("#")
    path = unquote(path.split("?", 1)[0])
    if not path:
        return doc, unquote(fragment)
    base = "" if path.startswith("/") else posixpath.dirname(doc)
    return posixpath.normpath(posixpath.join(base, path.lstrip("/"))), unquote(fragment)


def slug(heading):
    """A heading's anchor by GitHub's documented rule, before the suffix for a repeat."""
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", heading)
    text = re.sub(r"<[^>]+>", "", text)
    parts = re.split(r"(`+[^`]*`+)", text)
    text = "".join(
        part.strip("`") if part.startswith("`") else EMPHASIS.sub("", part) for part in parts
    )
    text = re.sub(r"[^\w\- ]", "", text.strip().lower())
    return text.replace(" ", "-")


def anchors(text):
    """Every anchor a Markdown text exposes: its headings' slugs, a repeat numbered, and HTML ids."""
    found, seen, fence = set(), {}, None
    for line in text.split("\n"):
        opening = FENCE.match(line)
        if fence:
            if opening and opening.group(1)[0] == fence[0] and line.strip() == opening.group(1):
                fence = None
            continue
        if opening:
            fence = opening.group(1)
            continue
        heading = HEADING.match(line)
        if heading:
            base = slug(heading.group(2) or "")
            count = seen.get(base, 0)
            found.add(base if count == 0 else f"{base}-{count}")
            seen[base] = count + 1
    for match in HTML_ANCHOR.finditer(outside_code(text)):
        found.add(match.group(1) or match.group(2))
    return found


def broken_links(root, names):
    """(the links examined, each relative link in a living doc or the index that does not
    resolve, as `<doc>: <destination> (<why>)`)."""
    root, tracked = Path(root), set(names)
    folders, cache = folders_of(names), {}
    docs = living_docs(names) + ([INDEX] if INDEX in tracked else [])
    examined_links, broken = [], []
    for doc in docs:
        for dest in destinations((root / doc).read_text(encoding="utf-8")):
            resolved = resolve(doc, dest)
            if resolved is None:
                continue
            examined_links.append(f"{doc}: {dest}")
            path, fragment = resolved
            if path not in tracked and path not in folders:
                broken.append(f"{doc}: {dest} (no tracked file or folder {path})")
                continue
            if fragment and path in tracked and path.endswith(".md"):
                if path not in cache:
                    cache[path] = anchors((root / path).read_text(encoding="utf-8"))
                if fragment not in cache[path]:
                    broken.append(f"{doc}: {dest} (no heading anchor #{fragment} in {path})")
    return examined_links, broken


def index_misses(root, names):
    """(the targets wanted, each living doc or record folder the docs index does not link)."""
    listed = set()
    for dest in destinations((Path(root) / INDEX).read_text(encoding="utf-8")):
        resolved = resolve(INDEX, dest)
        if resolved is not None:
            listed.add(resolved[0].rstrip("/"))
    wanted = living_docs(names) + list(RECORD_FOLDERS)
    return wanted, [target for target in wanted if target not in listed]


def without_comment(line):
    """A queue-file line without its comment; a `#` inside quotes is kept."""
    quote = None
    for at, char in enumerate(line):
        if quote:
            quote = None if char == quote else quote
        elif char in "\"'":
            quote = char
        elif char == "#" and (at == 0 or line[at - 1] in " \t"):
            return line[:at].rstrip()
    return line.rstrip()


def unquoted(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


def queue_rules(text):
    """The queue rules of a `.mergify.yml` in its block form, each a dict of scalars and lists."""
    rules, rule, key, rule_indent, inside = [], None, None, None, False
    for raw in text.split("\n"):
        line = without_comment(raw)
        if not line.strip():
            continue
        if re.search(r"(^|[\s:])[&*!][A-Za-z]|^\s*[\[{]|:\s*[\[{]", line):
            raise AssertionError(f"the queue reader does not know this form: {line.strip()}")
        indent = len(line) - len(line.lstrip())
        if indent == 0:
            inside, rule = line.strip() == "queue_rules:", None
            continue
        if not inside:
            continue
        new_rule = re.match(r"^(\s*)-\s+name:\s*(.+)$", line)
        if new_rule:
            rule_indent = len(new_rule.group(1)) + 2
            rule, key = {"name": unquoted(new_rule.group(2))}, None
            rules.append(rule)
            continue
        if rule is None:
            continue
        pair = re.match(r"^\s*([A-Za-z_]+):\s*(.*)$", line)
        if pair and indent == rule_indent:
            key = pair.group(1)
            rule[key] = unquoted(pair.group(2)) if pair.group(2) else []
            continue
        item = re.match(r"^\s*-\s+(.*)$", line)
        if item and indent > rule_indent and isinstance(rule.get(key), list):
            rule[key].append(unquoted(item.group(1)))
    return rules


def queue_facts(text):
    """The label, the required checks, the batch size, the merge method and the draft and review
    conditions of the one queue rule a `.mergify.yml` holds."""
    rules = queue_rules(text)
    if len(rules) != 1:
        raise AssertionError(f"the queue file holds {len(rules)} queue rules; the pages name one")
    rule = rules[0]
    conditions = rule.get("queue_conditions", [])
    labels = [m.group(1) for m in (re.match(r"^label\s*=\s*(\S+)$", c) for c in conditions) if m]
    checks = [
        m.group(1) for m in (re.match(r"^check-success\s*=\s*(\S+)$", c) for c in conditions) if m
    ]
    if len(labels) != 1 or not checks:
        raise AssertionError(f"the queue rule names {len(labels)} labels and {len(checks)} checks")
    return {
        "label": labels[0],
        "checks": checks,
        "batch_size": int(rule.get("batch_size") or 1),
        "merge_method": rule.get("merge_method", ""),
        "not_draft": "-draft" in conditions,
        "no_open_thread": any(
            c.replace(" ", "") == "#review-threads-unresolved=0" for c in conditions
        ),
    }


def flat(text):
    return " ".join(text.split())


def section(text, heading):
    """The lines of a section, from its heading to the next heading of its level or higher,
    outside fenced code; an absent heading gives an empty section."""
    level = len(heading) - len(heading.lstrip("#"))
    lines, out, fence, inside = text.split("\n"), [], None, False
    for line in lines:
        opening = FENCE.match(line)
        if not fence and not opening:
            found = HEADING.match(line)
            if found and len(found.group(1)) <= level:
                if inside:
                    break
                inside = line.strip() == heading
        if opening:
            fence = None if fence else opening.group(1)
        if inside:
            out.append(line)
    return "\n".join(out)


def branch_model_misses(page, facts):
    """(the statements wanted, each one the `## Branch model` section of the page lacks)."""
    words = NUMBER_WORDS.get(facts["batch_size"])
    method = MERGE_METHODS.get(facts["merge_method"])
    wanted = [
        "`dev` is the default branch",
        "the base of every pull request",
        "`main` changes only through a release pull request from `dev`",
        "merged with a merge commit",
        "Mergify merge queue",
        f"the label `{facts['label']}`",
        *[f"`{check}`" for check in facts["checks"]],
        f"up to {words} pull request" if words else f"a batch size of {facts['batch_size']}",
        method or f"an unknown merge method {facts['merge_method']}",
    ]
    if facts["not_draft"]:
        wanted.append("not a draft")
    if facts["no_open_thread"]:
        wanted.append("no unresolved review thread")
    text = flat(section(page, "## Branch model"))
    return wanted, [statement for statement in wanted if statement not in text]


def blocks(text):
    """Each blank-line-separated paragraph, each list item with its continuation lines, and each
    line inside a fenced code block."""
    out, current, fence = [], [], None

    def flush():
        if current:
            out.append("\n".join(current))
            current.clear()

    for line in text.split("\n"):
        opening = FENCE.match(line)
        if fence:
            if opening and opening.group(1)[0] == fence[0] and line.strip() == opening.group(1):
                fence = None
            elif line.strip():
                out.append(line)
            continue
        if opening:
            flush()
            fence = opening.group(1)
        elif not line.strip():
            flush()
        elif LIST_ITEM.match(line):
            flush()
            current.append(line)
        else:
            current.append(line)
    flush()
    return out


def surface_blocks(text):
    """(the blocks naming the Mini App, the bot, Telegram or initData, each one that neither says
    deprecated nor cites ADR-589)."""
    named = [block for block in blocks(text) if DEPRECATED_SURFACE.search(block)]
    current = [b for b in named if "deprecated" not in b.lower() and "ADR-589" not in b]
    return named, current


def readme_owned(text):
    """The README from its `## Install` heading up to its `## Source code` heading."""
    start, end = text.find("\n## Install\n"), text.find("\n## Source code\n")
    if start < 0 or end < start:
        raise AssertionError("the README lacks its `## Install` or `## Source code` heading")
    return text[start + 1 : end + 1]


def agents_owned(text):
    """AGENTS.md without the first sentence of its Overview paragraph, which the identity text owns."""
    lines = text.split("\n")
    if "## Overview" not in lines:
        raise AssertionError("AGENTS.md lacks its `## Overview` heading")
    at = lines.index("## Overview") + 1
    while at < len(lines) and not lines[at].strip():
        at += 1
    end = at
    while end < len(lines) and lines[end].strip():
        end += 1
    paragraph = "\n".join(lines[at:end])
    first = re.match(r"[\s\S]*?[.!?](?=\s|$)", paragraph)
    rest = paragraph[first.end() :] if first else paragraph
    return "\n".join(lines[:at] + [rest] + lines[end:])


def read(name):
    return (REPO / name).read_text(encoding="utf-8")


class TheDocsIndex(unittest.TestCase):
    def test_the_docs_index_links_every_living_doc_and_record_folder(self):
        with tempfile.TemporaryDirectory() as planted:
            root = Path(planted)
            (root / "docs").mkdir()
            (root / "docs" / "README.md").write_text(
                "- [Guide](GUIDE.md)\n- [Readme](../README.md)\n"
                + "".join(f"- [{name}](../{name}/)\n" for name in RECORD_FOLDERS),
                encoding="utf-8",
            )
            names = ["README.md", "CHANGELOG.md", "docs/GUIDE.md", INDEX]
            _, misses = index_misses(root, names)
            self.assertEqual(misses, ["CHANGELOG.md"], "a planted index missing one living doc")
        wanted, misses = index_misses(REPO, tracked_names(REPO))
        examined("living doc(s) and record folder(s) the index owes", wanted)
        self.assertEqual(misses, [], "the docs index does not link these")


class TheLinks(unittest.TestCase):
    def test_every_relative_link_in_a_living_doc_resolves(self):
        with tempfile.TemporaryDirectory() as planted:
            root = Path(planted)
            (root / "docs").mkdir()
            (root / "README.md").write_text("# Title\n\n## Install\n", encoding="utf-8")
            plants = {
                "a missing file": ("[x](docs/GONE.md)", "no tracked file or folder docs/GONE.md"),
                "a missing folder": ("[x](gone/)", "no tracked file or folder gone"),
                "a missing anchor": ("[x](README.md#usage)", "no heading anchor #usage"),
            }
            for what, (link, why) in plants.items():
                (root / "GUIDE.md").write_text(f"# Guide\n\n{link}\n", encoding="utf-8")
                _, broken = broken_links(root, ["README.md", "GUIDE.md"])
                self.assertEqual(len(broken), 1, what)
                self.assertIn(why, broken[0], what)
            (root / "GUIDE.md").write_text(
                "# Guide\n\n[x](README.md#install) [y](#guide) [z](docs/)\n\n```\n[a](GONE.md)\n```\n\n"
                "Code: `[b](GONE.md)`.\n",
                encoding="utf-8",
            )
            (root / "docs" / "NOTE.md").write_text("# Note\n", encoding="utf-8")
            planted_examined, broken = broken_links(root, ["README.md", "GUIDE.md", "docs/NOTE.md"])
            self.assertEqual(broken, [], "a link in a fence or a code span is not read")
            self.assertEqual(len(planted_examined), 3, "the three resolving planted links are read")
        links, broken = broken_links(REPO, tracked_names(REPO))
        examined("relative link(s) in the living docs", links)
        self.assertEqual(broken, [], "these relative links do not resolve")


class TheMergeQueue(unittest.TestCase):
    def test_contributing_states_the_branch_model_and_the_queue_file_facts(self):
        planted = read(QUEUE_FILE).replace("label = queue", "label = ready")
        planted_facts = queue_facts(planted)
        self.assertEqual(
            planted_facts["label"], "ready", "the planted queue file names another label"
        )
        _, planted_misses = branch_model_misses(read("CONTRIBUTING.md"), planted_facts)
        self.assertIn("the label `ready`", planted_misses, "a page naming another label is refused")
        facts = queue_facts(read(QUEUE_FILE))
        wanted, misses = branch_model_misses(read("CONTRIBUTING.md"), facts)
        examined("branch model statement(s)", wanted)
        self.assertEqual(misses, [], "the branch model in CONTRIBUTING.md lacks these")

    def test_the_readme_and_agents_name_the_merge_queue_and_its_label(self):
        label = queue_facts(read(QUEUE_FILE))["label"]
        places = [("README.md", "## Contributing"), ("AGENTS.md", "## Pull requests")]
        misses = []
        for name, heading in examined("front door section(s)", places):
            text = flat(section(read(name), heading))
            for wanted in ("Mergify merge queue", f"`{label}`"):
                if wanted not in text:
                    misses.append(f"{name} {heading}: {wanted}")
        self.assertEqual(misses, [], "these sections do not name the merge queue and its label")


class TheSurfaces(unittest.TestCase):
    def test_the_front_door_names_the_web_app_and_the_iphone_and_ipad_app(self):
        install = flat(section(read("README.md"), "## Install"))
        overview = flat(section(read("AGENTS.md"), "## Overview"))
        wanted = [
            ("README.md ## Install", install, "Rust workspace"),
            ("README.md ## Install", install, "web app"),
            ("README.md ## Install", install, "`web/app`"),
            ("README.md ## Install", install, "iPhone and iPad app"),
            ("README.md ## Install", install, "`ios/`"),
            ("AGENTS.md ## Overview", overview, "web app"),
            ("AGENTS.md ## Overview", overview, "`web/app`"),
            ("AGENTS.md ## Overview", overview, "iPhone and iPad app"),
            ("AGENTS.md ## Overview", overview, "`ios/`"),
        ]
        misses = [
            f"{where}: {name}"
            for where, text, name in examined("surface name(s)", wanted)
            if name not in text
        ]
        self.assertEqual(misses, [], "the front door does not name these")

    def test_the_front_door_names_the_mini_app_and_bot_only_as_deprecated(self):
        _, current = surface_blocks("The Mini App is the front end.\n\nThe bot is deprecated.\n")
        self.assertEqual(current, ["The Mini App is the front end."], "a planted current block")
        _, current = surface_blocks("- The Telegram Mini App is deprecated\n  (see ADR-589).\n")
        self.assertEqual(current, [], "a planted block saying deprecated passes")
        owned = {
            "README.md": readme_owned(read("README.md")),
            "AGENTS.md": agents_owned(read("AGENTS.md")),
            "CONTRIBUTING.md": read("CONTRIBUTING.md"),
            INDEX: read(INDEX),
        }
        named_all, refused = [], []
        for name, text in owned.items():
            named, current = surface_blocks(text)
            named_all += named
            refused += [f"{name}: {flat(block)[:120]}" for block in current]
        examined("owned block(s) naming the Mini App, the bot or Telegram", named_all)
        self.assertEqual(refused, [], "these blocks name a deprecated surface as current")


if __name__ == "__main__":
    unittest.main()

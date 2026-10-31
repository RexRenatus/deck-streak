"""The crate graph is the listed edges (SPEC-431 A1 to A3, R1 to R5; ADR-445 D1, D4, D5 and D8).

Every internal dependency between workspace members, in `[dependencies]`, `[dev-dependencies]` and
`[build-dependencies]`, at the top level and under every `[target.<key>]`, has exactly one row in
`config/crate-edges.json`, and every row has its edge (A1, A2). A member is named by its package
name less the `deck-streak-` prefix; an entry resolves to a member by its package (its `package`,
else its key) or by its `path`, and a `workspace = true` entry through the `[workspace.dependencies]`
entry of its key. Each row cites, in `adr`, exactly one `docs/decisions/ADR-<n>-*.md` file whose
text names the edge as `<from> -> <to>` in backticks (A3).

Each test judges the real tree first, then prints what it examined (0 is VOID), then refuses each
planted defect by name. The census reads committed files with `tomllib` and `json`, and starts no
process.
"""

import json
import re
import tempfile
import tomllib
import unittest
from pathlib import Path, PurePosixPath

from _support import REPO, examined

PREFIX = "deck-streak-"
DEV = "dev-dependencies"
BUILD = "build-dependencies"
TABLES = ("dependencies", DEV, BUILD)
UNDERSCORED = ("dev_dependencies", "build_dependencies")
LIST = "config/crate-edges.json"
ROW_KEYS = ("from", "to", "table", "target", "adr")
ADR_ID = re.compile(r"ADR-[0-9]+")
DECISIONS = "docs/decisions"


def workspace(root):
    """The root manifest's `[workspace]` table."""
    data = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    return data.get("workspace", {})


def lexical(path):
    """`path` with each `.` dropped and each `..` taken against the part before it, or None when it
    climbs above where it starts."""
    parts = []
    for part in PurePosixPath(path).parts:
        if part == "..":
            if not parts:
                return None
            parts.pop()
        elif part != ".":
            parts.append(part)
    return "/".join(parts)


def pattern_members(root, pattern):
    """The directories one `members` pattern matches that hold a `Cargo.toml`, relative to `root`."""
    return [
        path.relative_to(root).as_posix()
        for path in sorted(root.glob(pattern))
        if (path / "Cargo.toml").is_file()
    ]


def member_directories(root):
    """Each directory a `[workspace] members` pattern matches, less `exclude`, that holds a
    `Cargo.toml` (R2)."""
    table = workspace(root)
    excluded = {lexical(path) for path in table.get("exclude", [])}
    found = set()
    for pattern in table.get("members", []):
        found |= set(pattern_members(root, pattern))
    return sorted(found - excluded)


def manifests(root):
    """`{member directory: its parsed Cargo.toml}`."""
    return {
        directory: tomllib.loads((root / directory / "Cargo.toml").read_text(encoding="utf-8"))
        for directory in member_directories(root)
    }


def package_name(manifest):
    """A manifest's package name, or the empty string."""
    return manifest.get("package", {}).get("name", "")


def member_name(manifest):
    """A member's name: its package name less the prefix (R2)."""
    return package_name(manifest).removeprefix(PREFIX)


def scopes(manifest):
    """The manifest's top level and each `[target.<key>]` table, with that key or None (R3)."""
    found = [(None, manifest)]
    for key, table in sorted(manifest.get("target", {}).items()):
        found.append((key, table))
    return found


def resolve(directory, key, entry, inherited, packages, directories):
    """The member a dependency entry of the member at `directory` resolves to, or None: by its
    package (its `package`, else its key) or by its `path`, relative to the member's directory; a
    `workspace = true` entry through the `[workspace.dependencies]` entry of its key, whose
    `package` and `path` it takes, its `path` relative to the workspace root (R3)."""
    source, base = entry, directory
    if isinstance(entry, dict) and entry.get("workspace") is True:
        source, base = inherited.get(key, {}), ""
    if not isinstance(source, dict):
        source = {}
    package = source.get("package", key)
    if package in packages:
        return packages[package]
    path = source.get("path")
    if isinstance(path, str):
        return directories.get(lexical(f"{base}/{path}" if base else path))
    return None


def order(edge):
    """An edge's place in the list: by `from`, `to`, `table` and `target`, `null` first."""
    source, to, table, target = edge
    return (source, to, table, "" if target is None else target)


def manifest_edges(root):
    """Every internal edge the members' manifests state, as `(from, to, table, target)`, sorted
    (R3)."""
    read = manifests(root)
    inherited = workspace(root).get("dependencies", {})
    packages = {package_name(manifest): member_name(manifest) for manifest in read.values()}
    directories = {directory: member_name(manifest) for directory, manifest in read.items()}
    edges = set()
    for directory, manifest in read.items():
        for target, scope in scopes(manifest):
            for table in TABLES:
                for key, entry in scope.get(table, {}).items():
                    to = resolve(directory, key, entry, inherited, packages, directories)
                    if to is not None:
                        edges.add((member_name(manifest), to, table, target))
    return sorted(edges, key=order)


def read_list(root):
    """`config/crate-edges.json` parsed, or None when it is absent."""
    path = root / LIST
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None


def edge_rows(root):
    """The rows of `config/crate-edges.json`, or none when the list is absent."""
    data = read_list(root)
    rows = data.get("edges") if isinstance(data, dict) else None
    return rows if isinstance(rows, list) else []


def label(edge):
    """An edge or a row's key as a finding names it: `<from> -> <to> [<table>, <target>]`."""
    source, to, table, target = edge
    where = table if target is None else f"{table}, {target}"
    return f"{source} -> {to} [{where}]"


def row_problems(row, number):
    """A row's unknown and missing keys (R1)."""
    if not isinstance(row, dict):
        return [f"row {number}: not an object"]
    unknown = [f"row {number}: unknown key {key}" for key in row if key not in ROW_KEYS]
    return unknown + [f"row {number}: missing key {key}" for key in ROW_KEYS if key not in row]


def complete(row):
    """Whether a row holds every key, so it has an edge's key to compare."""
    return isinstance(row, dict) and all(key in row for key in ROW_KEYS)


def row_key(row):
    """A complete row's `(from, to, table, target)`."""
    return (row["from"], row["to"], row["table"], row["target"])


def edge_findings(root):
    """Every way the members' manifests and `config/crate-edges.json` disagree (R1 to R4)."""
    table = workspace(root)
    findings = [
        f"members pattern {pattern}: matches no manifest"
        for pattern in table.get("members", [])
        if not pattern_members(root, pattern)
    ]
    read = manifests(root)
    if not read:
        return [*findings, "examined 0 member manifest(s): VOID"]
    for directory, manifest in read.items():
        if not package_name(manifest).startswith(PREFIX):
            findings.append(
                f"{directory}: package {package_name(manifest)} lacks the {PREFIX} prefix"
            )
        for target, scope in scopes(manifest):
            for spelling in UNDERSCORED:
                if spelling in scope:
                    where = spelling if target is None else f"target.{target}.{spelling}"
                    findings.append(
                        f"{directory}/Cargo.toml: {where} is spelled with an underscore, "
                        "so the census refuses it"
                    )
    edges = manifest_edges(root)
    if not edges:
        findings.append("examined 0 manifest edge(s): VOID")
    data = read_list(root)
    rows = data.get("edges") if isinstance(data, dict) else None
    if not isinstance(rows, list):
        return [*findings, f'{LIST}: the "edges" list is absent']
    keys = []
    for number, row in enumerate(rows, 1):
        findings += row_problems(row, number)
        if not complete(row):
            continue
        key = row_key(row)
        if key in keys:
            findings.append(f"{label(key)}: a row listed twice")
        elif keys and order(key) < order(keys[-1]):
            findings.append(
                f"{label(key)}: a row out of order, sorted by from, to, table and target"
            )
        keys.append(key)
    findings += [
        f"{label(edge)}: an edge with no row in {LIST}" for edge in edges if edge not in keys
    ]
    stale = sorted(set(keys) - set(edges), key=order)
    return findings + [f"{label(key)}: a row with no edge in any manifest" for key in stale]


def admission_findings(root):
    """Each row whose `adr` names no single ADR file that holds its edge's token (R5)."""
    texts, findings = {}, []
    for row in edge_rows(root):
        if not complete(row):
            continue
        edge, adr = row_key(row), row["adr"]
        if not isinstance(adr, str) or not ADR_ID.fullmatch(adr):
            findings.append(f"{label(edge)}: adr {adr!r} is not ADR-<n>")
            continue
        files = sorted((root / DECISIONS).glob(f"{adr}-*.md"))
        if len(files) != 1:
            findings.append(
                f"{label(edge)}: {adr} is held by {len(files)} files in {DECISIONS}, not one"
            )
            continue
        if files[0] not in texts:
            texts[files[0]] = files[0].read_text(encoding="utf-8")
        token = f"`{edge[0]} -> {edge[1]}`"
        if token not in texts[files[0]]:
            held = files[0].relative_to(root).as_posix()
            findings.append(f"{label(edge)}: {held} does not hold {token}")
    return findings


def listed(what, rows):
    """Print how many rows a list holds; a list may legitimately be empty, so 0 is not refused."""
    rows = list(rows)
    print(f"listed {len(rows)} {what}")
    return rows


def planted(finder, files):
    """What `finder` finds in a scratch tree that holds only `files`, each `{path: text}`."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        for name, text in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        return finder(root)


WORKSPACE = (
    '[workspace]\nmembers = ["crates/*"]\n\n'
    '[workspace.dependencies]\ndeck-streak-kernel = { path = "crates/kernel" }\n'
)
ROW = {"from": "api", "to": "kernel", "table": "dependencies", "target": None, "adr": "ADR-1"}


def member(name, body=""):
    """A member manifest whose package is `deck-streak-<name>`, then `body`."""
    return f'[package]\nname = "deck-streak-{name}"\n\n{body}'


KERNEL = member("kernel")
API = member("api", '[dependencies]\ndeck-streak-kernel.workspace = true\nserde = "1"\n')
BOT = member("bot", "[dependencies]\ndeck-streak-kernel.workspace = true\n")


def tree(rows=(ROW,), workspace_text=WORKSPACE, listing=None, **members):
    """A workspace of `kernel` and `api` (`api -> kernel`), each member given by name replacing or
    joining them, and the list of `rows` (or `listing`, verbatim)."""
    files = {"Cargo.toml": workspace_text, "crates/kernel/Cargo.toml": KERNEL}
    files["crates/api/Cargo.toml"] = API
    for name, text in members.items():
        files[f"crates/{name}/Cargo.toml"] = text
    files[LIST] = json.dumps({"edges": list(rows)}) if listing is None else listing
    return files


def row(source, to, table="dependencies", target=None, adr="ADR-1"):
    """One list row."""
    return {"from": source, "to": to, "table": table, "target": target, "adr": adr}


def admitting(rows, decisions):
    """A tree holding only the list of `rows` and the ADR files `{name: text}`."""
    files = {LIST: json.dumps({"edges": list(rows)})}
    for name, text in decisions.items():
        files[f"{DECISIONS}/{name}"] = text
    return files


UNLISTED = "an edge with no row in config/crate-edges.json"
STALE = "a row with no edge in any manifest"
UNIX = "cfg(unix)"


class TheCrateGraphIsTheListedEdges(unittest.TestCase):
    def test_every_manifest_edge_has_one_row_and_every_row_an_edge(self):
        self.assertEqual(edge_findings(REPO), [])
        examined("member manifest(s)", member_directories(REPO))
        examined("manifest edge(s)", manifest_edges(REPO))
        listed("edge row(s)", edge_rows(REPO))
        bot_row = row("bot", "kernel")
        plants = {
            "an unlisted edge": (tree(bot=BOT), [f"bot -> kernel [dependencies]: {UNLISTED}"]),
            "a row with no edge": (
                tree(rows=(ROW, row("api", "kernel", DEV))),
                [f"api -> kernel [dev-dependencies]: {STALE}"],
            ),
            "a duplicated row": (
                tree(rows=(ROW, ROW)),
                ["api -> kernel [dependencies]: a row listed twice"],
            ),
            "a row out of order": (
                tree(rows=(bot_row, ROW), bot=BOT),
                [
                    "api -> kernel [dependencies]: a row out of order, "
                    "sorted by from, to, table and target"
                ],
            ),
            "an unknown key": (
                tree(rows=({**ROW, "note": "x"},)),
                ["row 1: unknown key note"],
            ),
            "an absent list": (
                tree(listing="{}"),
                ['config/crate-edges.json: the "edges" list is absent'],
            ),
            "a member whose package name lacks the prefix": (
                tree(kernel='[package]\nname = "kernel"\n'),
                ["crates/kernel: package kernel lacks the deck-streak- prefix"],
            ),
            "a pattern that matches no manifest": (
                tree(workspace_text=WORKSPACE.replace('"crates/*"', '"crates/*", "tools/*"')),
                ["members pattern tools/*: matches no manifest"],
            ),
            "a workspace with no member (VOID)": (
                {"Cargo.toml": "[workspace]\nmembers = []\n", LIST: json.dumps({"edges": []})},
                ["examined 0 member manifest(s): VOID"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(edge_findings, files), expected)

    def test_each_dependency_table_and_spelling_resolves_to_its_member(self):
        self.assertEqual(edge_findings(REPO), [])
        examined("member manifest(s)", member_directories(REPO))
        examined("manifest edge(s)", manifest_edges(REPO))
        on_unix = f"[target.'{UNIX}'.dependencies]\ndeck-streak-kernel.workspace = true\n"
        renamed = WORKSPACE + 'clock = { path = "crates/kernel", package = "deck-streak-kernel" }\n'
        plants = {
            "a [dev-dependencies] edge": (
                tree(api=API + "\n[dev-dependencies]\ndeck-streak-kernel.workspace = true\n"),
                [f"api -> kernel [dev-dependencies]: {UNLISTED}"],
            ),
            "a [build-dependencies] edge": (
                tree(api=API + "\n[build-dependencies]\ndeck-streak-kernel.workspace = true\n"),
                [f"api -> kernel [build-dependencies]: {UNLISTED}"],
            ),
            "an edge under a target table": (
                tree(api=API + "\n" + on_unix),
                [f"api -> kernel [dependencies, {UNIX}]: {UNLISTED}"],
            ),
            "that edge moved to [dependencies], so its old row is stale": (
                tree(rows=(row("api", "kernel", target=UNIX),)),
                [
                    f"api -> kernel [dependencies]: {UNLISTED}",
                    f"api -> kernel [dependencies, {UNIX}]: {STALE}",
                ],
            ),
            "a package = rename": (
                tree(
                    bot=member("bot", '[dependencies]\ncore = { package = "deck-streak-kernel" }\n')
                ),
                [f"bot -> kernel [dependencies]: {UNLISTED}"],
            ),
            "an inherited entry that renames by package": (
                tree(
                    workspace_text=renamed,
                    bot=member("bot", "[dependencies]\nclock.workspace = true\n"),
                ),
                [f"bot -> kernel [dependencies]: {UNLISTED}"],
            ),
            "a path under another key": (
                tree(bot=member("bot", '[dependencies]\nk = { path = "../kernel" }\n')),
                [f"bot -> kernel [dependencies]: {UNLISTED}"],
            ),
            "an optional = true edge": (
                tree(
                    bot=member(
                        "bot",
                        "[dependencies]\ndeck-streak-kernel = { workspace = true, optional = true }\n",
                    )
                ),
                [f"bot -> kernel [dependencies]: {UNLISTED}"],
            ),
            "a [features] entry, which adds none": (
                tree(api=API + '\n[features]\nclock = ["deck-streak-kernel/std"]\n'),
                [],
            ),
            "an external dependency, which is no edge": (
                tree(api=API + 'tokio = { version = "1", features = ["rt"] }\n'),
                [],
            ),
            "dev_dependencies": (
                tree(api=API + "\n[dev_dependencies]\ndeck-streak-kernel.workspace = true\n"),
                [
                    "crates/api/Cargo.toml: dev_dependencies is spelled with an underscore, "
                    "so the census refuses it"
                ],
            ),
            "build_dependencies": (
                tree(api=API + "\n[build_dependencies]\ndeck-streak-kernel.workspace = true\n"),
                [
                    "crates/api/Cargo.toml: build_dependencies is spelled with an underscore, "
                    "so the census refuses it"
                ],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(edge_findings, files), expected)

    def test_every_edge_is_admitted_by_an_adr_that_names_it(self):
        self.assertEqual(admission_findings(REPO), [])
        listed("edge row(s)", edge_rows(REPO))
        names = {"ADR-1-edges.md": "Admits `api -> kernel`.\n"}
        plants = {
            "a row citing an absent ADR": (
                admitting([row("api", "kernel", adr="ADR-2")], names),
                [
                    "api -> kernel [dependencies]: ADR-2 is held by 0 files in docs/decisions, not one"
                ],
            ),
            "a row citing an ADR whose text lacks the token": (
                admitting([ROW], {"ADR-1-edges.md": "Admits `api -> bot`.\n"}),
                [
                    "api -> kernel [dependencies]: docs/decisions/ADR-1-edges.md "
                    "does not hold `api -> kernel`"
                ],
            ),
            "a malformed adr": (
                admitting([row("api", "kernel", adr="ADR 1")], names),
                ["api -> kernel [dependencies]: adr 'ADR 1' is not ADR-<n>"],
            ),
            "an id two files share": (
                admitting([ROW], {**names, "ADR-1-again.md": "Admits `api -> kernel`.\n"}),
                [
                    "api -> kernel [dependencies]: ADR-1 is held by 2 files in docs/decisions, not one"
                ],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(admission_findings, files), expected)


if __name__ == "__main__":
    unittest.main()

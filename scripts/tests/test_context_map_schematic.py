"""The context-map schematic draws every workspace crate and every manifest edge (SPEC-394 A1 to A4;
ADR-408 D1 and D2).

`docs/schematics/context-map.md` draws the crate graph that the fence in `docs/CONTEXT-MAP.md`
records and the crates' manifests state. This census reads, by name:

- the directories under `crates/`: each is exactly one node, named by its directory (A1);
- every manifest, by `tomllib`, and the fence: each arrow is one manifest edge, each manifest edge
  is drawn once, and the fence names the same edges (A2);
- the drawing's layers: every crate sits in one declared layer and every arrow points down (A3);
- the drawing's header: it names exactly one full commit (A4).

Each test judges the real tree first, then says how much it examined, then refuses each planted
defect by name, so a census that went blind fails on its own controls. It reads files only and
starts no process.
"""

import re
import tomllib
import unittest

from _support import REPO, examined

PREFIX = "deck-streak-"
CRATES = REPO / "crates"
CONTEXT_MAP = REPO / "docs" / "CONTEXT-MAP.md"
SCHEMATIC = REPO / "docs" / "schematics" / "context-map.md"

FENCE = "`" * 3
ID = r"[A-Za-z][A-Za-z0-9_]*"
ROW = re.compile(r"^(\S+)\s+(?:(\S+)\s+)?\((.*)\)\s+depends on: (.*)$")
SUBGRAPH = re.compile(rf"^subgraph ({ID})\[(.+)\]$")
ARROW = re.compile(rf"^({ID}) --> ({ID})$")
NODE = re.compile(rf'^({ID})(?:\[(?:"([^"]*)"|([^\]"]*))\])?$')
ARROWISH = ("-->", "---", "==>", "-.-", "&")
COMMIT = re.compile(r"(?<![0-9a-f])[0-9a-f]{40}(?![0-9a-f])")
DEPENDENCY_TABLES = ("dependencies", "build-dependencies")


def dependency_name(key, value):
    """The crate an entry names: its `package` value when it is a table that has one, else its key."""
    if isinstance(value, dict) and isinstance(value.get("package"), str):
        return value["package"]
    return key


def manifest_edges(manifests):
    """`{directory: Cargo.toml text}` to the set of `(crate, dependency)` pairs it names.

    Reads `dependencies` and `build-dependencies`, and the same two names under every
    `target.<cfg>` table. Never reads `dev-dependencies`.
    """
    edges = set()
    for crate, text in manifests.items():
        data = tomllib.loads(text)
        tables = [data.get(name, {}) for name in DEPENDENCY_TABLES]
        for target in data.get("target", {}).values():
            tables.extend(target.get(name, {}) for name in DEPENDENCY_TABLES)
        for table in tables:
            for key, value in table.items():
                name = dependency_name(key, value)
                if name.startswith(PREFIX):
                    edges.add((crate, name[len(PREFIX) :]))
    return edges


def fence_rows(text, dirs):
    """The classified rows of the fence and the findings for rows of no kind.

    Returns `(rows, findings)`; a row is `(kind, name, dependencies)` where kind is `crate`,
    `planned` or `path`, and name is the short name of a crate row.
    """
    lines = text.splitlines()
    start = next((i for i, line in enumerate(lines) if line.strip() == f"{FENCE}context-map"), None)
    rows, findings = [], []
    if start is None:
        return rows, ["the fence is absent"]
    for line in lines[start + 1 :]:
        if line.strip() == FENCE:
            break
        if not line.strip():
            continue
        match = ROW.match(line)
        if not match:
            findings.append(f"fence row of no kind: {line.split()[0]}")
            continue
        name, path, note, listed = match.groups()
        listed = listed.strip()
        deps = (
            set()
            if listed in ("nothing", "nothing internal")
            else {part.strip() for part in listed.split(",") if part.strip()}
        )
        if name.startswith(PREFIX) and path is None:
            short = name[len(PREFIX) :]
            if short in dirs:
                rows.append(("crate", short, deps))
            elif "planned" in note:
                rows.append(("planned", short, deps))
            else:
                findings.append(f"fence row of no kind: {name}")
        elif not name.startswith(PREFIX) and path is not None and listed == "nothing internal":
            rows.append(("path", name, deps))
        else:
            findings.append(f"fence row of no kind: {name}")
    return rows, findings


def drawing(text):
    """The header, the direction, the layers, nodes and arrows of the mermaid drawing.

    Lines it cannot place are kept, with their numbers, for the criterion that refuses them.
    """
    lines = text.splitlines()
    first = next((i for i, line in enumerate(lines) if line.strip() == f"{FENCE}mermaid"), None)
    parsed = {
        "header": lines if first is None else lines[:first],
        "direction": None,
        "layers": [],
        "nested": [],
        "nodes": [],
        "arrows": [],
        "arrowish": [],
        "unreadable": [],
    }
    if first is None:
        return parsed
    stack = []
    for number, raw in enumerate(lines[first + 1 :], start=first + 2):
        line = raw.strip()
        if line == FENCE:
            break
        if not line or line.startswith("%%"):
            continue
        if parsed["direction"] is None:
            parsed["direction"] = line
            continue
        match = SUBGRAPH.match(line)
        if match:
            if stack:
                parsed["nested"].append((number, match.group(1), stack[0]))
            else:
                parsed["layers"].append(match.group(1))
            stack.append(match.group(1))
            continue
        if line == "end" and stack:
            stack.pop()
            continue
        match = ARROW.match(line)
        if match:
            parsed["arrows"].append((number, match.group(1), match.group(2), line))
            continue
        if any(token in line for token in ARROWISH):
            parsed["arrowish"].append((number, line))
            continue
        match = NODE.match(line)
        if match:
            label = match.group(2) if match.group(2) is not None else match.group(3)
            parsed["nodes"].append((number, match.group(1), label, stack[0] if stack else None))
            continue
        parsed["unreadable"].append((number, line))
    return parsed


def dir_of(node_id):
    """A node id spells each hyphen of its directory's name as an underscore."""
    return node_id.replace("_", "-")


def a1_findings(dirs, parsed):
    """A1: every directory is one node, named by it, and every node names a directory."""
    findings = []
    counts = {name: 0 for name in dirs}
    for _number, node_id, label, _layer in parsed["nodes"]:
        name = dir_of(node_id)
        if name not in counts:
            findings.append(f"{node_id}: names no directory under crates/")
            continue
        counts[name] += 1
        if label is not None and label != name:
            findings.append(f"{name}: label '{label}' is not the directory's name")
    for name, count in counts.items():
        if count == 0:
            findings.append(f"{name}: no node")
        elif count > 1:
            findings.append(f"{name}: {count} nodes")
    for number, text in parsed["unreadable"]:
        findings.append(f"line {number}: the census cannot read it: {text}")
    return sorted(findings)


def crate_arrows(dirs, parsed):
    """The arrows whose two ends are crates, as `(line, from, to)`, and the lines that are not."""
    drawn, findings = [], []
    for number, left, right, text in parsed["arrows"]:
        a, b = dir_of(left), dir_of(right)
        if a in dirs and b in dirs:
            drawn.append((number, a, b))
        else:
            findings.append(f"line {number}: an arrow that names no crate: {text}")
    return drawn, findings


def a2_findings(dirs, manifests, fence_text, parsed):
    """A2: the arrows, the manifests and the fence name the same edges, each drawn once."""
    wanted = manifest_edges(manifests)
    drawn, findings = crate_arrows(dirs, parsed)
    seen = {}
    for _number, a, b in drawn:
        seen[(a, b)] = seen.get((a, b), 0) + 1
    for a, b in wanted - set(seen):
        findings.append(f"{a} --> {b}: a manifest edge no arrow draws")
    for (a, b), count in seen.items():
        if (a, b) not in wanted:
            findings.append(f"{a} --> {b}: an arrow no manifest names")
        elif count > 1:
            findings.append(f"{a} --> {b}: drawn twice")
    for number, text in parsed["arrowish"]:
        findings.append(f"line {number}: an arrow line the census cannot read: {text}")
    rows, fence_findings = fence_rows(fence_text, dirs)
    findings.extend(fence_findings)
    fenced = {name: deps for kind, name, deps in rows if kind == "crate"}
    for name in dirs:
        if name not in fenced:
            findings.append(f"{name}: no fence row")
            continue
        named = {b for a, b in wanted if a == name}
        for extra in sorted(fenced[name] - named):
            findings.append(f"{name}: the fence names {extra}, the manifest does not")
        for extra in sorted(named - fenced[name]):
            findings.append(f"{name}: the manifest names {extra}, the fence does not")
    return sorted(findings)


def a3_findings(dirs, parsed):
    """A3: a top-down flowchart, flat layers, every crate in one layer, every arrow downward."""
    findings = []
    first = parsed["direction"]
    if first != "flowchart TB":
        findings.append(f"the drawing is {first or ''}, not flowchart TB")
    for number, node_id, outer in parsed["nested"]:
        findings.append(f"line {number}: subgraph {node_id} inside {outer}")
    layer_of = {}
    for _number, node_id, _label, layer in parsed["nodes"]:
        name = dir_of(node_id)
        if name not in dirs:
            continue
        if layer is None:
            findings.append(f"{name}: outside every layer")
        else:
            layer_of[name] = layer
    order = {layer: index for index, layer in enumerate(parsed["layers"])}
    drawn, _ = crate_arrows(dirs, parsed)
    for _number, a, b in drawn:
        if a not in layer_of or b not in layer_of:
            continue
        la, lb = layer_of[a], layer_of[b]
        if la == lb:
            findings.append(f"{a} --> {b}: stays in layer {la}")
        elif order[lb] < order[la]:
            findings.append(f"{a} --> {b}: points up from {la} to {lb}")
    return sorted(findings)


def a4_findings(parsed):
    """A4: the header names exactly one full commit."""
    commits = COMMIT.findall("\n".join(parsed["header"]))
    if len(commits) != 1:
        return [f"the header names {len(commits)} full commits, not one"]
    return []


def judge(dirs, manifests, fence_text, schematic_text):
    """All four criteria over one tree, as `{criterion: findings}`."""
    parsed = drawing(schematic_text)
    return {
        "A1": a1_findings(dirs, parsed),
        "A2": a2_findings(dirs, manifests, fence_text, parsed),
        "A3": a3_findings(dirs, parsed),
        "A4": a4_findings(parsed),
    }


def real_tree():
    """The repository's directories, manifests, fence text and drawing text."""
    dirs = sorted(p.name for p in CRATES.iterdir() if p.is_dir())
    manifests = {d: (CRATES / d / "Cargo.toml").read_text(encoding="utf-8") for d in dirs}
    fence_text = CONTEXT_MAP.read_text(encoding="utf-8")
    return dirs, manifests, fence_text, SCHEMATIC.read_text(encoding="utf-8")


def fail_on(findings):
    """Fail with every finding named, or return when there is none."""
    if findings:
        raise AssertionError(f"findings: {len(findings)}\n" + "\n".join(findings))


FIXTURE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
FIXTURE_DIRS = ["ingest", "kernel", "web-engine"]
FIXTURE_MANIFESTS = {
    "kernel": '[package]\nname = "deck-streak-kernel"\n',
    "ingest": '[package]\nname = "deck-streak-ingest"\n\n[dependencies]\n'
    "deck-streak-kernel.workspace = true\nserde.workspace = true\n",
    "web-engine": '[package]\nname = "deck-streak-web-engine"\n\n'
    "[target.'cfg(target_arch = \"wasm32\")'.dependencies]\n"
    "deck-streak-kernel.workspace = true\n",
}
FIXTURE_INGEST_ROW = "deck-streak-ingest  (the anti-corruption layer)  depends on: kernel"
FIXTURE_FENCE = "\n".join(
    [
        "# Fixture map",
        "",
        f"{FENCE}context-map",
        "deck-streak-kernel  (the shared kernel)  depends on: nothing",
        FIXTURE_INGEST_ROW,
        "deck-streak-web-engine  (the web engine)  depends on: kernel",
        "deck-streak-migration  (a one-off import, planned)  depends on: kernel, ingest",
        "miniapp   web/app/src  (the web client)  depends on: nothing internal",
        FENCE,
        "",
    ]
)
ADAPTERS = '  subgraph adapters[adapters]\n    ingest\n    web_engine["web-engine"]\n  end\n'
FOUNDATIONS = "  subgraph foundations[no workspace dependency]\n    kernel\n  end\n"
FIXTURE_DRAWING = (
    f"# Schematic: fixture\n\nRead at `dev` {FIXTURE_COMMIT}.\n\n{FENCE}mermaid\nflowchart TB\n"
    f"{ADAPTERS}{FOUNDATIONS}\n  ingest --> kernel\n  web_engine --> kernel\n{FENCE}\n"
)


def swap(text, old, new):
    """Replace one exact fixture line, asserting that it occurs once."""
    assert text.count(old) == 1, f"{old!r} occurs {text.count(old)} times in the fixture"
    return text.replace(old, new)


def line_of(text, needle):
    """The 1-based number of the first line holding `needle`."""
    for number, line in enumerate(text.splitlines(), start=1):
        if needle in line:
            return number
    raise AssertionError(f"{needle!r} is on no line")


def fixture_findings(drawing_text=FIXTURE_DRAWING, fence_text=FIXTURE_FENCE):
    """The four criteria over the fixture tree, with the drawing or the fence replaced."""
    return judge(FIXTURE_DIRS, FIXTURE_MANIFESTS, fence_text, drawing_text)


class TheSchematicDrawsTheCrateGraph(unittest.TestCase):
    def assertFixtureClean(self):
        """The positive control: the clean fixture reads no finding in any criterion."""
        self.assertEqual(fixture_findings(), {"A1": [], "A2": [], "A3": [], "A4": []})

    def test_every_crate_directory_is_one_node_named_by_it(self):
        dirs, manifests, fence_text, schematic = real_tree()
        fail_on(judge(dirs, manifests, fence_text, schematic)["A1"])
        examined("crate directories", dirs)
        self.assertFixtureClean()
        plants = [
            (
                swap(FIXTURE_DRAWING, FOUNDATIONS, FOUNDATIONS.replace("    kernel\n", "")),
                "kernel: no node",
            ),
            (swap(FIXTURE_DRAWING, "    ingest\n", "    ingest\n    ingest\n"), "ingest: 2 nodes"),
            (
                swap(FIXTURE_DRAWING, '["web-engine"]', '["web engine"]'),
                "web-engine: label 'web engine' is not the directory's name",
            ),
            (
                swap(FIXTURE_DRAWING, "    ingest\n", "    ingest\n    ghost\n"),
                "ghost: names no directory under crates/",
            ),
        ]
        for text, finding in plants:
            self.assertIn(finding, fixture_findings(text)["A1"])
        text = swap(
            FIXTURE_DRAWING,
            "\n  ingest --> kernel\n",
            "\n  classDef x fill:#fff\n  ingest --> kernel\n",
        )
        self.assertIn(
            f"line {line_of(text, 'classDef')}: the census cannot read it: classDef x fill:#fff",
            fixture_findings(text)["A1"],
        )

    def test_every_arrow_is_a_manifest_edge_the_fence_names(self):
        dirs, manifests, fence_text, schematic = real_tree()
        fail_on(judge(dirs, manifests, fence_text, schematic)["A2"])
        examined("manifest edges", manifest_edges(manifests))
        rows, _ = fence_rows(fence_text, dirs)
        examined("fence rows", rows)
        self.assertFixtureClean()
        arrow = "  ingest --> kernel\n"
        plants = [
            (swap(FIXTURE_DRAWING, arrow, ""), "ingest --> kernel: a manifest edge no arrow draws"),
            (
                swap(FIXTURE_DRAWING, arrow, arrow + "  kernel --> ingest\n"),
                "kernel --> ingest: an arrow no manifest names",
            ),
            (swap(FIXTURE_DRAWING, arrow, arrow + arrow), "ingest --> kernel: drawn twice"),
        ]
        for text, finding in plants:
            self.assertIn(finding, fixture_findings(text)["A2"])
        text = swap(FIXTURE_DRAWING, arrow, "  ingest --> foundations\n")
        self.assertIn(
            f"line {line_of(text, '--> foundations')}: an arrow that names no crate: "
            "ingest --> foundations",
            fixture_findings(text)["A2"],
        )
        text = swap(FIXTURE_DRAWING, arrow, "  ingest --> kernel & web_engine\n")
        self.assertIn(
            f"line {line_of(text, '&')}: an arrow line the census cannot read: "
            "ingest --> kernel & web_engine",
            fixture_findings(text)["A2"],
        )
        fence = swap(
            FIXTURE_FENCE,
            FIXTURE_INGEST_ROW,
            "deck-streak-ingest  (the anti-corruption layer)  depends on: kernel, web-engine",
        )
        self.assertIn(
            "ingest: the fence names web-engine, the manifest does not",
            fixture_findings(fence_text=fence)["A2"],
        )
        fence = swap(
            FIXTURE_FENCE,
            FIXTURE_INGEST_ROW,
            "deck-streak-ingest  (the anti-corruption layer)  depends on: nothing",
        )
        self.assertIn(
            "ingest: the manifest names kernel, the fence does not",
            fixture_findings(fence_text=fence)["A2"],
        )
        fence = swap(
            FIXTURE_FENCE,
            f"{FENCE}context-map\n",
            f"{FENCE}context-map\ndeck-streak-ghost (a ghost)  depends on: kernel\n",
        )
        self.assertIn(
            "fence row of no kind: deck-streak-ghost", fixture_findings(fence_text=fence)["A2"]
        )
        fence = swap(FIXTURE_FENCE, FIXTURE_INGEST_ROW, "")
        self.assertIn("ingest: no fence row", fixture_findings(fence_text=fence)["A2"])
        planted = {
            "ingest": (
                '[package]\nname = "deck-streak-ingest"\n\n'
                "[dependencies]\ndeck-streak-kernel.workspace = true\n"
                'serde = "1"\n\n'
                "[dev-dependencies]\ndeck-streak-dev.workspace = true\n\n"
                "[build-dependencies]\ndeck-streak-built.workspace = true\n\n"
                "[target.'cfg(unix)'.dependencies]\ndeck-streak-targeted.workspace = true\n\n"
                "[target.'cfg(unix)'.build-dependencies]\ndeck-streak-targetbuilt = { workspace = true }\n\n"
                '[dependencies.alias]\npackage = "deck-streak-renamed"\nworkspace = true\n'
            )
        }
        self.assertEqual(
            manifest_edges(planted),
            {
                ("ingest", name)
                for name in ("kernel", "built", "targeted", "targetbuilt", "renamed")
            },
        )

    def test_every_arrow_points_down_through_declared_layers(self):
        dirs, manifests, fence_text, schematic = real_tree()
        fail_on(judge(dirs, manifests, fence_text, schematic)["A3"])
        drawn, _ = crate_arrows(dirs, drawing(schematic))
        examined("arrows", drawn)
        self.assertFixtureClean()
        self.assertIn(
            "the drawing is flowchart LR, not flowchart TB",
            fixture_findings(swap(FIXTURE_DRAWING, "flowchart TB", "flowchart LR"))["A3"],
        )
        text = swap(
            FIXTURE_DRAWING, FOUNDATIONS, "  kernel\n" + FOUNDATIONS.replace("    kernel\n", "")
        )
        self.assertIn("kernel: outside every layer", fixture_findings(text)["A3"])
        text = swap(
            FIXTURE_DRAWING,
            ADAPTERS + FOUNDATIONS,
            ADAPTERS.replace("  end\n", "    kernel\n  end\n")
            + FOUNDATIONS.replace("    kernel\n", ""),
        )
        self.assertIn("ingest --> kernel: stays in layer adapters", fixture_findings(text)["A3"])
        text = swap(FIXTURE_DRAWING, ADAPTERS + FOUNDATIONS, FOUNDATIONS + ADAPTERS)
        self.assertIn(
            "ingest --> kernel: points up from adapters to foundations",
            fixture_findings(text)["A3"],
        )
        text = swap(
            FIXTURE_DRAWING, "    ingest\n", "    subgraph inner[inner]\n    end\n    ingest\n"
        )
        self.assertIn(
            f"line {line_of(text, 'subgraph inner')}: subgraph inner inside adapters",
            fixture_findings(text)["A3"],
        )

    def test_the_header_names_the_full_commit_it_was_read_at(self):
        dirs, manifests, fence_text, schematic = real_tree()
        parsed = drawing(schematic)
        fail_on(judge(dirs, manifests, fence_text, schematic)["A4"])
        examined("header lines", parsed["header"])
        self.assertFixtureClean()
        short = swap(FIXTURE_DRAWING, FIXTURE_COMMIT, FIXTURE_COMMIT[:7])
        self.assertIn("the header names 0 full commits, not one", fixture_findings(short)["A4"])
        two = swap(FIXTURE_DRAWING, f"{FIXTURE_COMMIT}.", f"{FIXTURE_COMMIT} and {'b' * 40}.")
        self.assertIn("the header names 2 full commits, not one", fixture_findings(two)["A4"])


if __name__ == "__main__":
    unittest.main()

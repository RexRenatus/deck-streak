# SPEC-394: the context-map schematic draws every workspace crate and every manifest edge, held by a census

- **Wave:** none; a repair of the crate graph's drawing (#689).
  **Issue:** #689. **Context(s):** none changes; the context map's drawing,
  `docs/schematics/context-map.md`, and its binding record, the fence in `docs/CONTEXT-MAP.md`.
- **Decided by:** ADR-408, ADR-002 (the crate graph is the context map, and the fence is binding).
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-394.md`.

## 1. The problem, measured

Every figure below was read at `dev` `164ac20690e0d6d0385474d04663d14df5781e19` (DEV).

| fact | figure | how it was read |
|---|---|---|
| directories under `crates/` | 31 | `git ls-tree -d --name-only DEV crates/` |
| manifests that name an internal crate | 27 of 31 | `git grep -c -E '^deck-streak-[a-z0-9-]+\.workspace = true' DEV` over `crates/*/Cargo.toml` |
| internal dependencies across the manifests | 84 | the same grep, summed; each line sits under `[dependencies]`, or `web-engine`'s `wasm32` target table |
| internal dependencies under `[dev-dependencies]` or `[build-dependencies]` | 0 | each manifest read by `tomllib` |
| fence rows | 35: 31 crates, 1 planned crate, 3 contexts named by a path | the ```context-map fence of `docs/CONTEXT-MAP.md` |
| fence edges that differ from the manifests | 0 | each crate row's `depends on:` beside its manifest |
| nodes in the drawing | 25; `engine-core`, `ffi`, `fsrs7`, `mcp`, `push` and `web-engine` are missing | `git show DEV:docs/schematics/context-map.md` |
| nodes whose label is not their directory's name | 10 (`daemon: deckstreakd` and nine more role labels) | the same file |
| arrow lines, and the edges they denote | 13 lines, 55 edges: 7 single arrows, 3 that chain targets with `&`, 3 that point at a layer | the same file |
| manifest edges no arrow denotes | 29 | 84 less 55 |
| crates outside every layer | 2 (`kernel`, `xp`) | the same file |
| the commit the header names | `main` at a 7-character prefix | the same file, line 3 |
| tests that read the drawing | 0; three read the fence (`test_context_map_register.py`, `test_parity_matrix.py`, `test_xp_crate_graph.py`) | `git grep -l` over `scripts/tests` |

So the drawing lacks six crates and 29 edges, and nothing in the repository's checks says so.

## 2. Requirements

R1. Every directory under `crates/` is exactly one node of the drawing. A node's id is its
directory's name with each hyphen spelt as an underscore, and its label, when it has one, is the
directory's name. A node that names no directory is refused.

R2. Every arrow is one line `<a> --> <b>` from one crate's node to another's, and the drawn edges
equal the internal dependencies the crates' manifests name: under `[dependencies]` or
`[build-dependencies]`, or under a target's table of either name, by key or by a `package` rename.
A `[dev-dependencies]` entry is not an edge. An edge drawn twice, an arrow that names a layer and a
line that chains targets are each refused. The fence in `docs/CONTEXT-MAP.md` names the same edges,
crate by crate, and every fence row is one of three kinds: a crate with a directory, a planned
crate whose note says planned, or a context named by a path that depends on nothing internal. A
row of no kind is refused.

R3. The drawing is a `flowchart TB`. Its layers are subgraphs declared top to bottom, none inside
another. Every crate's node sits in exactly one layer, and every arrow leaves its layer for a later
one, so it points down.

R4. The drawing's header, the text above its mermaid fence, names exactly one commit, as 40
lowercase hexadecimal characters: the commit the drawing was read at.

R5. `scripts/tests/test_context_map_schematic.py` holds R1 to R4, one test per criterion. Each
judges the real tree first, then prints what it examined through `_support.examined`, which
refuses an empty population, then refuses each of its planted defects by name. It reads files
only and starts no process.

R6. `scripts/mutation-rows.d/S39400-S39499.json` holds seven script rows, S39400 to S39406. Each
is a mutant of the drawing that exactly one criterion's test kills.

R7. No manifest, no line of the fence and no product code changes.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | Every directory under `crates/` is one node, named by its directory (R1) | ADR-408 D1, D2 |
| A2 | Every arrow is one manifest edge, every manifest edge is drawn once, and the fence names the same edges (R2) | ADR-408 D1, D2 |
| A3 | Every crate sits in one declared layer, and every arrow points down (R3) | ADR-408 D1, D2 |
| A4 | The header names the full commit the drawing was read at (R4) | ADR-408 D2 |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py -k test_every_crate_directory_is_one_node_named_by_it
A2: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py -k test_every_arrow_is_a_manifest_edge_the_fence_names
A3: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py -k test_every_arrow_points_down_through_declared_layers
A4: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py -k test_the_header_names_the_full_commit_it_was_read_at
```

At the base drawing each test is red for its own criterion. A1 names 16 findings: the six crates
with no node, and ten labels that are not their directory's name. A2 names 83: the 77 manifest
edges that no readable arrow draws, the three arrows that name a layer, and the three lines that
chain targets with `&`. A3 names two: `kernel` and `xp` sit outside every layer. A4 names one: the
header holds no 40-character commit. Each turns green with the drawing alone.

## 4. File manifest

| path | change | context |
|---|---|---|
| `docs/specs/SPEC-394-the-context-map-schematic-draws-every-workspace-crate-and-every-manifest-edge-held-by-a-census.md` | added | this SPEC |
| `docs/decisions/ADR-408-the-schematic-draws-each-manifest-edge-as-its-own-downward-arrow-and-a-census-holds-it-to-the-fence.md` | added | its decision |
| `docs/schematics/context-map.md` | changed | the drawing: 31 nodes, 84 arrows, six layers |
| `scripts/tests/test_context_map_schematic.py` | added | the census, A1 to A4 |
| `scripts/mutation-rows.d/S39400-S39499.json` | added | seven script rows |
| `docs/red-first/SPEC-394.md` | added | the red-first record |
| `changelog.d/context-map-every-crate-394.md` | added | the changelog fragment |

No other path changes: no `crates/*/Cargo.toml`, no line of `docs/CONTEXT-MAP.md`, and no code.

## 5. What this does NOT cover

- The clients' edges to `xp`, from `ffi` and `web-engine`: the delivery that adds them to the
  manifests and the fence draws them (#639).
- `push`'s edge to `notifications` and the daemon's edge to `push`, which native push carries
  through the one router (#640).
- A node for the planned `deck-streak-migration`: it has no directory and no manifest, so it is
  drawn by the delivery that builds it (#689 draws directories).
- The web client's code under `web/app` and the native harness under `ios/`: the fence lists each
  as depending on nothing internal, and neither is a crate (#689).
- A rendering of the drawing in the repository's checks: the census reads the text, and no
  renderer runs (#689 asks for a drawing held to the manifests).
- Development dependencies: they are not edges, and the fence's own holder reads none (#689).
- The open web client sync work changes no manifest and adds no crate; an edge it adds later is
  drawn in that change (#748).

## 6. Risks

- The census reads a narrower grammar than mermaid does, so a drawing a renderer accepts (an edge
  label, a style line) can read red. The census names each line it cannot read, so the failure
  says what to change; widening the grammar is an amendment of this SPEC.
- No renderer runs in the repository's checks, so a drawing the census admits might not render.
  Node ids are plain identifiers, a hyphenated name is a quoted label, and the design's offline
  lint read the drawing clean.
- A manifest form the reader misses would drop an edge before it is compared. The reader takes
  keys and `package` renames from every table that can hold an edge, target tables included, and a
  planted control holds each form.
- A fence row the parser cannot place is refused as a row of no kind, never skipped, so a fence
  edit cannot drop a crate from the comparison.
- A later edit of the drawing can make a row's find absent or ambiguous. Each find occurs once
  today, and the rows census of the repository's checks reads every row.

## 7. Mutation rows

Each row mutates `docs/schematics/context-map.md`, and its killer is a test of
`test_context_map_schematic.TheSchematicDrawsTheCrateGraph`.

| row | mutant | killed by |
|---|---|---|
| `S39400-CRATE-NODE-DROPPED` | the `fsrs7` node becomes a comment | A1 |
| `S39401-NODE-LABEL-RENAMED` | `web-engine`'s label loses its hyphen | A1 |
| `S39402-MANIFEST-EDGE-UNDRAWN` | the `progression --> xp` arrow becomes a comment | A2 |
| `S39403-ARROW-AHEAD-OF-MANIFEST` | a `push --> notifications` arrow is drawn, which no manifest names | A2 |
| `S39404-NODE-OUTSIDE-LAYERS` | `kernel` moves above the last layer, outside every layer | A3 |
| `S39405-ARROW-WITHIN-A-LAYER` | `coordination` moves into the domain layer, so its arrows to the domain stay in one layer | A3 |
| `S39406-HEADER-SHORT-COMMIT` | the header's commit becomes a 7-character prefix | A4 |

Each mutant reads red in its killer alone; the other three tests stay green on it.

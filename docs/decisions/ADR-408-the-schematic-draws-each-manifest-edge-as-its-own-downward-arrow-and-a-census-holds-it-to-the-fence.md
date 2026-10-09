---
status: accepted
decision-makers: "the owner, the DeckStreak architect"
---

# The schematic draws each manifest edge as its own downward arrow, and a census holds it to the fence

## Context and Problem Statement

`docs/schematics/context-map.md` draws the crate graph by layer, and its own text says that an
arrow is a Cargo dependency that points down. The binding record is the fence in
`docs/CONTEXT-MAP.md`, which the ddd probe holds equal to every manifest in both
directions. Nothing holds the drawing. Measured at `dev` 164ac20690e0d6d0385474d04663d14df5781e19:

- 31 directories under `crates/`, each with a `Cargo.toml` whose package is
  `deck-streak-<directory>`; the drawing has 25 nodes and lacks `engine-core`, `ffi`, `fsrs7`,
  `mcp`, `push` and `web-engine` (#689).
- 84 internal dependencies across the manifests, every one a `deck-streak-<name>.workspace = true`
  line under `[dependencies]`, or, for `web-engine`, under its `wasm32` target's dependency table.
  No internal crate is named under `[dev-dependencies]` or `[build-dependencies]`.
- The fence names the same 84 edges, crate by crate. There is no disagreement between the fence and
  the manifests, so the fence needs no amendment.
- The drawing's 13 arrow lines denote 55 of the 84 edges: 7 are single arrows, 3 chain targets
  with `&`, and 3 point at a layer (`coordination --> domain`, `domain --> kernel`,
  `acl --> kernel`). The other 29 are not drawn, and `kernel` and `xp` sit outside every layer.
- Its header names `main` at a 7-character commit.
- Three tests read the fence (`test_context_map_register.py`, `test_parity_matrix.py`,
  `test_xp_crate_graph.py`); none reads the drawing.

The XP crate's delivery drew its own node and edge and left the rest to #689. This ADR decides how
the drawing is drawn, how it is held, what the delivery owes, whether it owes a formal model, and
what happens when another delivery changes the graph.

## Decision Drivers

- The crate graph is the context map (ADR-002), and the fence is its binding record: the drawing
  draws the fence, and it never decides an edge.
- A drawing in which one arrow stands for many edges cannot be compared with a manifest line.
- A census is held by a positive artifact and a planted control, and prints what it examined.
- A list that drops members before they are asserted is a weakening.
- A crate's role has one home, its line in the fence.

## Considered Options (the alternatives it was chosen against)

### D1, the drawing

- Chosen: an edge-for-edge drawing, because only it can be held equal to the fence, which names
  every edge: one node per directory under `crates/`, and one `a --> b` line per internal
  dependency. The nodes sit in six declared layers, top to bottom: the composition root
  (`daemon`); the adapters (`api`, `bot`, `mcp`, `push`, `ffi`, `web-engine`); the application
  (`coordination`); the domain contexts; the anti-corruption layers (`ingest`, `vault`,
  `identity`, `agent`); and the crates that depend on no workspace crate (`kernel`, `xp`,
  `engine-core`, `fsrs7`). A node id spells a hyphen as an underscore, and its label keeps the
  directory's name.

Chosen against:

- A transitive reduction, which drops `daemon --> kernel`: lost, because the fence and the
  manifests name all 84 edges, and a reduced drawing could be compared with neither line by line.
- The base form, an arrow to a layer or an arrow chained with `&`: lost, because one arrow then
  stands for up to fifteen manifest lines, which is how 29 edges went undrawn with no line looking
  wrong.
- Role labels on each node, such as the base's `daemon: deckstreakd`: lost, because they repeat the
  fence's notes in a second home that drifts, and the fence already holds each crate's role.
- A separate layer for the client adapters: lost, because a layer exists to order arrows, `ffi`
  and `web-engine` translate one transport at the edge as the other adapters do, and the daemon's
  own arrows show which adapters it composes.
- A node for the planned `deck-streak-migration`: lost, because it has no directory and no
  manifest, so none of its arrows could be a manifest dependency, and a node without them would
  misdraw its fence line.
- Amending the fence where it and the drawing disagree: lost, because the fence and the manifests
  agree on all 84 edges, and a real disagreement is a design question for its own ADR, never a fix
  inside a drawing.

### D2, held by a census

- Chosen: a census, `scripts/tests/test_context_map_schematic.py`, with seven script rows, because
  the drawing drifted by six crates and 29 edges with nothing to say so.
  It holds one test per criterion, and each judges the real tree, then prints what it examined,
  then refuses planted defects by name. The rows sit in
  `scripts/mutation-rows.d/S39400-S39499.json`.
  An edge is read from `[dependencies]`, `[build-dependencies]` and each target's table of either
  name, by key or by a `package` rename, as `test_xp_crate_graph.py` reads one crate's edges. The
  header names the full 40-character commit. The census starts no process and reads files only.

Chosen against:

- Prose only, with the header's commit as the reader's warning: lost, because that is the base
  form, and it drifted through six crate deliveries without a red.
- Widening `test_xp_crate_graph.py`: lost, because its population is one crate's edges (SPEC-360
  A6), and widening it would tie the whole graph to one crate's SPEC.
- Generating the drawing from the manifests by a script: lost, because the layers are a design the
  manifests do not hold, and a generator plus its check is two tools where one census does.
- Leaving it to the ddd probe: lost, because the probe reads the fence and the manifests and never
  the drawing, and it runs outside the repository's own checks.
- Counting `[dev-dependencies]` as edges: lost, because the ddd pack reads no dev-dependency table,
  so the drawing would hold an edge the fence's own holder never counts.
- A 7-character commit in the header: lost, because a short prefix grows ambiguous as the history
  grows, and the base header already held one, so a check of the short form could never be red.
- A census that resolves the header's commit in git: lost, because a CI checkout may not hold that
  commit, so the build re-measures it once instead.

### D3, what the artifact bar needs

- Chosen: SPEC-394 and this ADR, because the delivery adds a test and its rows, which owe criteria
  in an acceptance fence and a red-first record, and a SPEC is decided by an ADR.

Chosen against:

- This ADR alone, as a decision that changes no behaviour takes: lost, because a test is added, and
  a test with no SPEC has no criterion for its red-first record.
- A drawing change with neither document: lost, because the rule that every arrow is a manifest
  edge that points down was chosen against named alternatives, and the census needs its criteria
  where a probe reads them.

### D4, a formal model, decided by surface

- Chosen: not applicable, because the census is one process that reads committed files and
  compares sets: no second actor, no shared state written, no check followed by an act, and no
  protocol step. None of the 228 `@phx covers` lines in the tree names a path this delivery adds or
  changes.

Chosen against:

- A TLA+ model of the census: lost, because it has one actor and no interleaving, so a model would
  restate the test.
- A Lean proof that every arrow points down: lost, because the census is no algorithm of the gate,
  and its planted controls and rows already measure that the comparison can fail.

### D5, drift

- Chosen: the delivery that lands second draws, because once this census lands, a delivery that
  adds a crate or an internal edge draws its node or arrow in the change that amends the fence.
  This delivery installs the drawing read at the commit its header names. Its builder re-measures
  the crate directories, the 84 edges, the fence block and the base drawing at the cut, and stops on
  any change.

Chosen against:

- An allow-list of known undrawn crates or edges in the census: lost, because a list that drops
  members before they are asserted is a weakening, and it is how drift becomes permanent.
- A follow-up issue for each drift, as the XP crate's delivery left this one: lost, because six
  crates and 29 edges accumulated that way.
- Holding other pull requests that change a manifest until this lands: lost, because the open web
  client work (#748) changes no crate manifest and adds no crate, and a later change draws its own
  arrow.

## Decision Outcome

Chosen options: D1 to D5 as each "Chosen:" bullet states, because together they make the drawing a
picture of the fence that a census compares with every manifest, edge by edge, on every change.

### Consequences

- Good: a crate or an edge that is not drawn is a named failure in the repository's own checks,
  whose message is the line to add.
- Good: the drawing names its commit in full, and each arrow can be found in one manifest line.
- Bad: the drawing has 84 arrow lines, and the daemon's 24 make its corner dense.
- Bad: every delivery that adds a crate or an internal edge now edits the drawing as well as the
  fence.

### Confirmation

SPEC-394's four acceptance criteria, each a test of `scripts/tests/test_context_map_schematic.py`,
red at the base drawing for its own reason (`docs/red-first/SPEC-394.md`), and the seven rows of
`scripts/mutation-rows.d/S39400-S39499.json`, each killed by exactly one of those tests.

### What would make this wrong

- A crate whose edges the fence states in a form the census cannot read, such as a dependency on
  another context through a path include.
- A renderer that refuses a node id the census admits.
- A decision that the drawing should show a reduced graph; D1 would then be re-decided.

## More Information

#689; #635 (the XP crate's delivery, which drew its own node and edge); #639 (the clients' edges to
`xp`); #640 (`push`'s edge to `notifications` and its join in the daemon); ADR-002; ADR-371;
SPEC-360; SPEC-394.

---
status: accepted
decision-makers: "the DeckStreak architect"
---

# A workspace entry takes the package rename of its workspace dependencies entry, read from the root manifest

## Context and Problem Statement

`scripts/tests/test_context_map_schematic.py` holds the context map's drawing to every internal
edge the crates' manifests name (SPEC-394, ADR-408). It names an entry's crate by the entry's own
`package` value, else by its key. A member that inherits a dependency writes only
`workspace = true`, and Cargo takes the dependency, its `package` rename included, from the root
manifest's `[workspace.dependencies]` entry of the same key. The census never opens that table, so
an inherited rename of a workspace crate is either a lost edge (an `alias` key) or the wrong crate
(a `deck-streak-` key renamed to another crate). None stands on `dev` today (SPEC-617 section 1).

## Decision Drivers

- The census opens files only and starts no process (SPEC-394 R5).
- An edge is named the way Cargo resolves it, so the drawing, the fence and the manifests name one
  crate the same way.
- A census is held by a planted control, and prints what it examined.
- A reading that silently drops an input is a weakening.

## Considered Options (the alternatives it was chosen against)

### D1, where an inherited rename is found

- Chosen: a table written `workspace = true` takes the `package` value of the root manifest's
  `[workspace.dependencies]` entry of its key, when that entry has one; the member's own `package`
  value still comes first, as SPEC-394 has it, and a key with neither still names its own crate.
- Against: run `cargo metadata` and take its resolved graph. It starts a process and needs the
  toolchain in a census that opens files only.
- Against: resolve every entry by its `path` as well. The drawing and the fence name crates by
  package, and no entry on `dev` reaches a workspace crate by path alone.
- Against: keep naming by keys only. The miss stays.

### D2, how the table reaches the census

- Chosen: the real tree's reading parses `Cargo.toml` by `tomllib` and passes its
  `[workspace.dependencies]` table to every reading of the edges, and each fixture passes its own.
  No reading takes a default, so a reading that forgot the table fails rather than going blind.
- Against: a default of an empty table. A caller that forgot it would name every inherited rename
  by its key, which is the miss this decision cures.

### D3, a formal model, decided by surface

- Chosen: none. A pure mapping from one table entry to one name has no protocol, concurrency or
  state to model.

## Decision Outcome

Chosen options: D1 to D3 as each "Chosen:" bullet states, because together they name an inherited
rename the way Cargo resolves it, from files alone, with no reading that can skip the table.

### Consequences

- Good: an inherited rename of a workspace crate is compared as the crate it names, and a lost or
  misnamed edge is a named failure.
- Good: the edges the census names from the tree today are unchanged.
- Bad: the census now parses the root manifest as well as each member's.

### Confirmation

SPEC-617's two acceptance criteria, each a test of `scripts/tests/test_context_map_schematic.py`,
red with the resolver unchanged for its own reason (`docs/red-first/SPEC-617.md`), and the two rows
of `scripts/mutation-rows.d/S61700-S61799.json`, each killed by its criterion's test.

### What would make this wrong

- A Cargo change that lets a member entry carry its own `package` beside `workspace = true` with a
  meaning other than the root entry's: D1's order would then be re-decided.
- A workspace crate reached by `path` under a key that names another crate.

## More Information

#881; SPEC-394; ADR-408; ADR-002.

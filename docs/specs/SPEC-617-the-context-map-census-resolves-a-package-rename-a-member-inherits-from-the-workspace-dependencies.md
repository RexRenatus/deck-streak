# SPEC-617: the context-map census resolves a package rename a member inherits from the workspace dependencies

- **Wave:** none; a repair of the context-map census (#881).
  **Issue:** #881. **Context(s):** none changes; the census that holds the crate graph's drawing,
  `scripts/tests/test_context_map_schematic.py`, which SPEC-394 delivered.
- **Decided by:** ADR-639, ADR-408 (a census holds the drawing to every manifest edge).
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-617.md`.

## 1. The problem, measured

Every figure below was read at the `dev` commit this delivery branched from (DEV), each manifest
by `tomllib`.

| fact | figure | how it was read |
|---|---|---|
| entries in the root manifest's `[workspace.dependencies]` | 84 | `git show DEV:Cargo.toml` |
| of them keyed `deck-streak-*` | 35 | the same table |
| of them that declare a `package` rename | 2: `fsrs6` and `fsrs7`, each naming `fsrs`, which is no workspace crate | the same table |
| member entries written `workspace = true` in a table SPEC-394 R2 reads | 227 | each `crates/*/Cargo.toml` at DEV |
| of them whose root entry declares a rename | 1: `fsrs7` in `crates/fsrs7` | the same manifests |
| where the census looks for a rename | the member's own entry only: `dependency_name` takes the entry's `package`, else its key | `scripts/tests/test_context_map_schematic.py` at DEV, lines 40 to 44 |
| internal edges the census names, and the edges it names with R1 | 91 and 91, the same set | both readings over the same manifests |

So no rename of a workspace crate is inherited today, and the census could not see one: a member
that inherits `alias = { package = "deck-streak-kernel", path = "crates/kernel" }` as
`alias.workspace = true` would lose its edge, and one that inherits a rename under a
`deck-streak-` key would name the wrong crate.

## 2. Requirements

R1. The census names the crate a manifest entry resolves to: the entry's own `package` value when
it is a table that has one; else, for a table written `workspace = true`, the `package` value of
the entry of the same key in the root manifest's `[workspace.dependencies]` table, when that entry
is a table that has one; else the entry's key. It holds in every table SPEC-394 R2 takes an edge
from: `[dependencies]`, `[build-dependencies]` and a target's table of either name. A
`[dev-dependencies]` entry is still not an edge.

R2. The census parses the root manifest, `Cargo.toml`, by `tomllib`, and passes its
`[workspace.dependencies]` table to every reading of the manifests' edges, on the real tree and on
each fixture. No reading defaults the table, so each fixture states its own.

R3. `scripts/tests/test_context_map_schematic.py` holds R1 and R2, one test per criterion. Each
prints what it examined through `_support.examined`, which refuses an empty population, and
refuses its planted defect by name. The census still opens files only and starts no process.

R4. `scripts/mutation-rows.d/S61700-S61799.json` holds two script rows, S61700 and S61701. Each is
a mutant of the census that its criterion's test kills.

R5. No manifest, no line of the fence, no drawing and no product code changes, and the edges the
census names from the tree are the same before and after.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | A `workspace = true` entry takes the `package` rename of its key's workspace entry in every edge table, and never names its key's crate (R1) | ADR-639 D1 |
| A2 | The real tree's census parses the root manifest's `[workspace.dependencies]` table and passes it to the edge reading (R2) | ADR-639 D2 |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py -k test_an_inherited_package_rename_names_the_renamed_crate
A2: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py -k test_the_real_tree_reads_the_workspace_dependencies
```

With the resolver unchanged each test is red for its own criterion: A1 names the inherited rename
`alias` as no edge and `deck-streak-old` as an edge to `old`, and A2's planted rename on the real
table moves no edge. Each turns green with the resolver alone.

## 4. File manifest

| path | change | context |
|---|---|---|
| `docs/specs/SPEC-617-the-context-map-census-resolves-a-package-rename-a-member-inherits-from-the-workspace-dependencies.md` | added | this SPEC |
| `docs/decisions/ADR-639-a-workspace-entry-takes-the-package-rename-of-its-workspace-dependencies-entry-read-from-the-root-manifest.md` | added | its decision |
| `docs/schematics/context-map-census-inherited-renames.md` | added | the census data flow, with the root manifest |
| `scripts/tests/test_context_map_schematic.py` | changed | the resolver, the root manifest table, A1 and A2 |
| `scripts/mutation-rows.d/S61700-S61799.json` | added | two script rows |
| `docs/red-first/SPEC-617.md` | added | the red-first record |
| `changelog.d/context-map-inherited-rename-617.md` | added | the changelog fragment |

No other path changes: no `Cargo.toml`, no `crates/*/Cargo.toml`, no line of
`docs/CONTEXT-MAP.md` or of `docs/schematics/context-map.md`, and no code.

## 5. What this does NOT cover

- Resolving an entry by its `path`: the drawing and the fence name crates by package, and no entry
  on `dev` reaches a workspace crate by path alone (#881 asks for the rename).
- A member's own `package` beside `workspace = true`: Cargo refuses that form, and the census keeps
  SPEC-394's reading of it, which its planted control holds.
- Development dependencies: they are still not edges (SPEC-394 R2).

## 6. Risks

- A real rename of a workspace crate through the root table now changes the edges the census
  names; the drawing and the fence then follow the crate the rename names, which is R1's point.
- A2's planted rename anchors on the `deck-streak-xp` entry and on `progression`'s edge to it,
  which SPEC-394's rows also use. If either leaves the tree, the test fails by name, and the change
  that removes it moves the anchor.
- A `workspace = true` entry whose key has no root entry is a manifest Cargo refuses; the census
  names it by its key, as before.

## 7. Mutation rows

Each row mutates `scripts/tests/test_context_map_schematic.py`, and its killer is a test of
`test_context_map_schematic.TheSchematicDrawsTheCrateGraph`.

| row | mutant | killed by |
|---|---|---|
| `S61700-INHERITED-RENAME-IGNORED` | the resolver returns the key where an inherited rename stands | A1 |
| `S61701-WORKSPACE-TABLE-UNREAD` | the real tree passes an empty table for the root manifest's workspace dependencies | A2 |

## 8. Amendments: A1's fixture covers every edge table, and every fixture reading passes its table

A later delivery closes three gaps that a review of this delivery found. It changes no reading of
the real tree: the census names the same edges before and after (R5).

- **A1: the fixture covers every edge table R1 names.** Plain `[build-dependencies]` and a
  target's `[dependencies]` join `[dependencies]` and a target's `[build-dependencies]` in the
  fixture of `test_an_inherited_package_rename_names_the_renamed_crate`. Each new entry inherits a
  rename to a crate that no other table of the fixture names, so a census that skipped either
  table would lose that crate's edge.
- **R4: two more rows.** `S61702-PLAIN-BUILD-TABLE-UNREAD` joins S61700 and S61701. Its mutant
  drops the plain `[build-dependencies]` table from the edge reading, which A1's fixture before
  this amendment let pass, and A1 kills it. `S61703-TARGET-DEPENDENCIES-UNREAD` drops a target's
  `[dependencies]` table, where the clean fixture A1 checks first holds an edge, and A1 kills it.
- **R2: the fixture helper takes no default.** `fixture_findings` takes the workspace dependencies
  table as a required keyword, and every fixture reading passes its table, the fixture tree's own
  `FIXTURE_INHERITED` or a renaming one, so no reading defaults the table.
- **The red-first record.** `docs/red-first/SPEC-617.md` now says that, before the resolver
  changed, the census passed the root manifest's table on and the resolver never read it.

| path | change |
|---|---|
| `scripts/tests/test_context_map_schematic.py` | changed: A1's fixture, and the fixture helper's table |
| `scripts/mutation-rows.d/S61700-S61799.json` | changed: rows S61702 and S61703 |
| `docs/red-first/SPEC-617.md` | changed: one sentence, and a paragraph on this amendment |
| `docs/specs/SPEC-617-the-context-map-census-resolves-a-package-rename-a-member-inherits-from-the-workspace-dependencies.md` | changed: this section |
| `changelog.d/context-map-rename-tables-617.md` | added |

No manifest, no line of the fence, no drawing, no decision record and no product code changes.

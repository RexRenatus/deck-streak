# SPEC-431: three fitness tests hold the crate graph, the formal entries' vectors tests and the production clock reads to listed rows

- **Wave:** none; three fitness tests for the architecture's standing facts.
  **Issue:** none; the pull request closes nothing. **Context(s):** none changes; the checks read
  every member's manifest, the formal entries and every production Rust file.
- **Decided by:** ADR-445. Read and not changed: SPEC-020 R6 (the one system clock), SPEC-295
  (`config/formal.json`), SPEC-360 and SPEC-394 (the two crate-graph censuses), ADR-002 and
  ADR-408.
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-431.md`.

## 1. The problem, measured

Every figure was read at `dev` `b25ed04ab10f24a89a1ec0eed71f8a365f411521`, and each census below
re-derives its own on every run.

| fact | figure | command |
|---|---|---|
| workspace members | 31 | `git ls-tree --name-only b25ed04a crates/` (each holds a `Cargo.toml`; the root names `members = ["crates/*"]`) |
| internal edges in the members' manifests | 85: 85 `[dependencies]`, 0 `[dev-dependencies]`, 0 `[build-dependencies]`; 1 under a target table | `git grep -n -E '^deck-streak-[a-z0-9-]+(\.workspace = true\| = \{)' b25ed04a -- 'crates/*/Cargo.toml'` |
| the one target-table edge | `web-engine -> engine-core`, under `[target.'cfg(target_arch = "wasm32")'.dependencies]` | `git show b25ed04a:crates/web-engine/Cargo.toml` |
| Lean entries (a `Formal/<E>.lean` with `-- @phx` header lines) | 13, each with `@phx covers` and one `@phx vectors` line | `git grep -l -E '^-- @phx vectors ' b25ed04a -- formal/lean` |
| TLA+ entries | 27, each with `@phx covers` and no `@phx vectors` line | `git ls-tree -r --name-only b25ed04a formal/tla` (the `<E>/<E>.tla` files) |
| vectors files | 13 | `git ls-tree --name-only b25ed04a formal/vectors/` |
| vectors tests | 11, each naming one `formal/vectors/<slug>.jsonl` once in a string literal | `git ls-tree -r --name-only b25ed04a crates/ \| grep '/tests/formal_vectors_'` |
| implemented Lean entries whose vectors no test reads | 2: `lean/SensitiveDeck`, `lean/TableCensusReach` | the two rows above, compared |
| production clock reads | 7 in 6 files: 4 `SystemTime::now`, 1 `js_sys::Date::now`, 2 `Instant::now` | `git grep -n -E '(SystemTime\|Utc\|Local\|Instant\|Date)::now' b25ed04a -- 'crates/*/src/*'` |
| clock reads in `#[cfg(test)]` module files under `src/` | 1, in `crates/ingest/src/skip_write/tests.rs` | the same command; `crates/ingest/src/skip_write.rs` declares the module under `#[cfg(test)]` |

What no check holds today:

- `scripts/tests/test_context_map_schematic.py` (SPEC-394) holds the manifests, the fence in
  `docs/CONTEXT-MAP.md` and the drawing equal, but no record names the ADR that admitted an edge;
  it reads no `[dev-dependencies]`, and it reads the `package` of a member's own entry, never the
  `package` of the `[workspace.dependencies]` entry a `workspace = true` line inherits.
  `scripts/tests/test_xp_crate_graph.py` (SPEC-360) holds the per-review XP crate alone.
- Nothing holds a formal entry to a test that reads its vectors, or a vectors test to a live entry.
  `config/formal.json` names no implementation, and SPEC-295 holds it to the checker's own fields.
- SPEC-020 R6 makes `SystemClock` the only reader of the system time in production code, and no
  test holds it: two reads in `crates/ingest/src/skip_write.rs` sit beside the kernel's `Clock`.

## 2. Requirements

**The crate graph** (`scripts/tests/test_crate_edges.py`, `config/crate-edges.json`):

R1. The census reads the workspace manifest and every member's manifest with `tomllib`, and holds
    the set of internal edges equal to the rows of `config/crate-edges.json` in both directions: an
    edge with no row fails, and a row with no edge fails. The list is `{"edges": [row, ...]}`, each
    row `{"from", "to", "table", "target", "adr"}`; a duplicate row, a row out of sorted order
    (by `from`, `to`, `table`, `target`), an unknown key and an absent list each fail by name.
R2. The population is every directory a `[workspace] members` pattern matches, less `exclude`,
    that holds a `Cargo.toml`. A member is named by its package name less the `deck-streak-`
    prefix. A member whose package name lacks the prefix, and a pattern that matches no manifest,
    each fail by name.
R3. An edge is an entry of `[dependencies]`, `[dev-dependencies]` or `[build-dependencies]`, at the
    top level or under any `[target.<key>]`, that resolves to a member: by its package (its
    `package` value, else its key) or by its `path`, relative to the member's directory. A
    `workspace = true` entry resolves through the `[workspace.dependencies]` entry of its key,
    taking that entry's `package` and `path`. The edge's row key is its `from`, `to`, `table` and
    `target` (the `[target.<key>]` key as the manifest spells it, or `null`). A table spelled with
    an underscore (`dev_dependencies`, `build_dependencies`) fails by name.
R4. The census prints the members, the edges and the rows it examined, and 0 of any fails as VOID.
R5. Each edge's row names, in `adr`, exactly one file `docs/decisions/ADR-<n>-*.md`, and that
    file's text holds the edge as `` `<from> -> <to>` ``. An edge whose row cites no such file, or a
    file that does not hold its token, fails as unadmitted.

**Formal-to-code parity** (`scripts/tests/test_formal_parity.py`, `config/vectors-parity.json`):

R6. The census derives the entries as the formal checker does: each
    `formal/lean/Formal/<E>.lean` whose leading comment lines carry a `-- @phx` line is the entry
    `lean/<E>`, and each `formal/tla/<E>/<E>.tla` is the entry `tla/<E>`. An entry is implemented
    when its header carries at least one `@phx covers` line, and it declares vectors with
    `-- @phx vectors <path>`.
R7. Every implemented Lean entry declares exactly one vectors path, that file exists, and exactly
    one vectors test names it; otherwise it has an `owed` row. A Lean entry with no `@phx covers`
    line has an `unimplemented` row. A row whose condition does not hold fails as stale. A TLA+
    entry that declares vectors fails by name.
R8. Every `crates/*/tests/formal_vectors_*.rs` names exactly one `formal/vectors/<slug>.jsonl`
    inside a string literal, outside every comment; its file name is `formal_vectors_<slug>.rs`
    with each `-` of the slug written `_`; and that path is the declared vectors path of exactly one
    live Lean entry. Every `formal/vectors/*.jsonl` is declared by exactly one entry.
R9. The list is `{"owed": [row, ...], "unimplemented": [row, ...]}`, each row `{"entry", "adr"}`,
    sorted by `entry`, with no entry twice and no unknown key; each row's ADR file holds its entry
    as `` `lean/<E>` ``. The census prints the entries (Lean and TLA+ apart), the vectors tests and
    the vectors files it examined, and 0 of any fails as VOID.

**The clock-read allow-list** (`scripts/tests/test_clock_reads.py`, `config/clock-reads.json`):

R10. The census reads every production Rust file, each `.rs` under a member's `src/` and its
     `build.rs` that is not test code (R12), and holds each file's count of clock reads equal to
     its row: a file whose count differs from its row in either direction fails, and a row for a
     file with no read fails. The list is `{"sites": [row, ...]}`, each row
     `{"path", "count", "adr", "why"}`; a duplicate path, a row out of sorted order, an unknown
     key and an absent list each fail by name.
R11. A clock read is `<T>::now` where `<T>` is `SystemTime`, `Instant`, `Utc`, `Local` or `Date`,
     or a name a `use ... as <name>` binds to one of them, or a `type <name> = ...;` makes one,
     anywhere in the population; or `OffsetDateTime::now_utc` or `OffsetDateTime::now_local`. Any
     path prefix is allowed, whitespace around `::` is allowed, no identifier character may
     continue the name on either side, and a reference with no call is a read. It is matched after
     comments (line, nested block and doc comments) and the contents of string, raw string, byte
     string and character literals are removed.
R12. Test code is never read as production: a file under a member's `tests/`, `examples/` or
     `benches/` directory; a module file that a production file declares under `#[cfg(test)]` or
     `#[cfg(all(test, ...))]`, by `mod <name>;` or with `#[path = "..."]`, and every module beneath
     it; and an item under one of those attributes inside a production file, whose balanced body
     is removed before matching. `#[cfg(not(test))]` and `#[cfg(any(test, ...))]` mark production
     code.
R13. The census prints the production files it read, the files it set aside as test code and the
     reads it found, and 0 production files fails as VOID.
R14. Each row's ADR file holds its path as `` `<path>` ``; a file whose reads have no such row fails
     as unadmitted.

**The listings and the homes:**

R15. At the base the three lists hold exactly ADR-445's appendix: 85 edge rows, each
     `"table": "dependencies"` and `"adr": "ADR-445"`, `web-engine -> engine-core` with
     `"target": "cfg(target_arch = \"wasm32\")"` and every other with `"target": null`; `owed`
     rows `lean/SensitiveDeck` and `lean/TableCensusReach` and no `unimplemented` row; and six
     clock rows, `crates/agent/src/runner.rs` 1, `crates/daemon/src/snapshot_lister.rs` 1,
     `crates/ffi/src/engine.rs` 1, `crates/ingest/src/skip_write.rs` 2,
     `crates/kernel/src/clock.rs` 1 and `crates/web-engine/src/wasm.rs` 1. No product file
     changes.
R16. Each module reads committed files with the standard library and `_support`'s `REPO` and
     `examined`, starts no process, and imports, runs or reaches no code by a name held in data.
     Each test judges the real tree first, then prints what it examined, then refuses each planted
     defect by name.
R17. Each check's kill criteria are ADR-445 D6's, read over the last 30 merged pull requests.

## 3. Acceptance criteria of SPEC-431

| id | criterion | decided by |
|---|---|---|
| A1 | Every manifest edge has one row and every row an edge; the population is every member, printed, and 0 is VOID (R1, R2, R4) | ADR-445 D1, D4 |
| A2 | Each dependency table, target table, rename, inherited entry and path resolves to its member, and an underscore table spelling fails (R3) | ADR-445 D1 |
| A3 | Every edge is admitted by an ADR file that names it (R5) | ADR-445 D1 |
| A4 | Every implemented Lean entry has its vectors test or a row, every row holds, and a TLA+ entry declaring vectors fails (R6, R7, R9) | ADR-445 D2 |
| A5 | Every vectors test names one live entry from a string literal, by its own slug, and every vectors file one entry (R8) | ADR-445 D2 |
| A6 | Every production file's clock reads equal its row's count, and 0 production files is VOID (R10, R13) | ADR-445 D3, D4 |
| A7 | Each clock spelling, path prefix and alias is a read (R11) | ADR-445 D3 |
| A8 | Test code by directory, by `#[cfg(test)]` module file and by `#[cfg(test)]` item is never read as production (R12) | ADR-445 D3 |
| A9 | Comments, doc comments and every literal form hold no read (R11) | ADR-445 D3 |
| A10 | Every file with reads is admitted by an ADR file that names it (R14) | ADR-445 D3 |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_crate_edges.py -k test_every_manifest_edge_has_one_row_and_every_row_an_edge
A2: python3 -m unittest discover -s scripts/tests -p test_crate_edges.py -k test_each_dependency_table_and_spelling_resolves_to_its_member
A3: python3 -m unittest discover -s scripts/tests -p test_crate_edges.py -k test_every_edge_is_admitted_by_an_adr_that_names_it
A4: python3 -m unittest discover -s scripts/tests -p test_formal_parity.py -k test_every_implemented_lean_entry_has_its_vectors_test_or_a_row
A5: python3 -m unittest discover -s scripts/tests -p test_formal_parity.py -k test_every_vectors_test_names_one_live_entry_and_every_file_one_entry
A6: python3 -m unittest discover -s scripts/tests -p test_clock_reads.py -k test_every_production_files_clock_reads_equal_its_row
A7: python3 -m unittest discover -s scripts/tests -p test_clock_reads.py -k test_each_clock_spelling_and_alias_is_a_read
A8: python3 -m unittest discover -s scripts/tests -p test_clock_reads.py -k test_test_code_is_never_read_as_production
A9: python3 -m unittest discover -s scripts/tests -p test_clock_reads.py -k test_comments_and_literals_hold_no_read
A10: python3 -m unittest discover -s scripts/tests -p test_clock_reads.py -k test_every_file_with_reads_is_admitted_by_an_adr_that_names_it
```

Each test's planted defects, each refused by name with the finding its fixture spells:

- A1: an unlisted edge; a row with no edge; a duplicated row; a row out of order; an unknown key;
  an absent list; a member whose package name lacks the prefix; a pattern that matches no
  manifest; a workspace with no member (VOID).
- A2: a `[dev-dependencies]` edge; a `[build-dependencies]` edge; an edge under a target table;
  that edge moved to `[dependencies]` (a new key, so its old row is stale); a `package =` rename;
  a `workspace = true` entry whose `[workspace.dependencies]` entry renames by `package`; a `path`
  under another key; an `optional = true` edge; a `[features]` entry, which adds none; an
  external dependency, which is no edge; `dev_dependencies` and `build_dependencies`.
- A3: a row citing an absent ADR; a row citing an ADR whose text lacks the token; a malformed
  `adr`; an id two files share.
- A4: an implemented entry with no test and no row; an `owed` row for an entry that has a test; an
  implemented entry with no `@phx vectors` line; a declared vectors file that is absent; an entry
  with no `@phx covers` line and no row; an `unimplemented` row for an entry with covers; a TLA+
  entry declaring vectors; a row whose ADR lacks its token; no entry (VOID).
- A5: a test naming a path no entry declares; a test naming its path only in a comment; a test
  naming two paths; a test whose file name disagrees with its slug; a vectors file no entry
  declares; one path two entries declare; no vectors test (VOID).
- A6: an unlisted read; a count one below its file's reads; a count one above; a row for a file
  with no read; a read in a `build.rs`; a duplicate path; an absent list; no production file
  (VOID).
- A7: `std::time::SystemTime::now()`, `Instant :: now()`, `tokio::time::Instant::now()`,
  `chrono::Utc::now()`, `chrono::Local::now()`, `js_sys::Date::now()`,
  `time::OffsetDateTime::now_utc()`, `SystemTime::now` as a function pointer, a `use ... as`
  alias in another file, and a `type` alias, each a read; `StudyDate::now()`,
  `SystemTime::nowhere()` and `clock.now()` none.
- A8: a read under a member's `tests/`, `examples/` and `benches/`; in a module file declared under
  `#[cfg(test)]` by `mod`; in one declared with `#[path]`; in a submodule beneath it; in a
  `#[cfg(test)] mod tests { ... }` block; in a `#[cfg(all(test, ...))]` function; each set aside;
  one under `#[cfg(not(test))]` and one under `#[cfg(any(test, ...))]`, each read.
- A9: a read in a line comment, a block comment, a nested block comment, a doc comment holding a
  doc test, a string literal with an escaped quote, a raw string `r#"..."#`, a byte string and a
  character literal `'"'`, each none; a read after a lifetime `'a` and after a closed block
  comment, each one.
- A10: a row citing an absent ADR; a row citing an ADR whose text lacks the path.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_crate_edges.py` | scripts | added: the crate-graph census, A1 to A3 |
| `scripts/tests/test_formal_parity.py` | scripts | added: the formal parity census, A4 and A5 |
| `scripts/tests/test_clock_reads.py` | scripts | added: the clock-read census, A6 to A10 |
| `config/crate-edges.json` | config | added: the 85 edge rows (R15) |
| `config/vectors-parity.json` | config | added: two `owed` rows, no `unimplemented` row (R15) |
| `config/clock-reads.json` | config | added: the six clock rows (R15) |
| `docs/specs/SPEC-431-three-fitness-tests-hold-the-crate-graph-the-formal-entries-vectors-tests-and-the-production-clock-reads-to-listed-rows.md` | docs | added: this SPEC |
| `docs/decisions/ADR-445-each-fitness-test-is-its-own-census-over-a-listed-file-and-every-row-names-the-adr-that-admits-it.md` | docs | added: its decision |
| `docs/schematics/fitness-tests-crate-edges-formal-vectors-and-clock-reads.md` | docs | added: the three censuses' data flow |
| `docs/red-first/SPEC-431.md` | docs | added: the red-first record |
| `scripts/mutation-rows.d/S43100-S43199.json` | scripts | added: rows S43100 to S43133 (section 7) |
| `changelog.d/fitness-tests-431.md` | changelog | added: the fragment |

No other path changes: no product file, no `Cargo.toml`, no line of `docs/CONTEXT-MAP.md`, no
workflow, no other SPEC or ADR, and neither `scripts/tests/test_ci_workflows.py` nor any other
existing test module.

## 5. What this does NOT cover

- The clients' planned edges to `xp`, from `ffi` and `web-engine`: the delivery that adds them to
  the manifests adds their rows, with its ADR naming each (#639).
- `push`'s edge to `notifications` and the daemon's edge to `push`, which native push carries
  through the one router: their rows come with that delivery's ADR (#640).
- The two reads of `SystemTime::now` in `crates/ingest/src/skip_write.rs`, listed as found:
  reading them through the kernel's `Clock`, as SPEC-020 R6 asks of production code, is its own
  delivery (#877).
- The vectors tests `lean/SensitiveDeck` and `lean/TableCensusReach` owe, listed as `owed` rows:
  each is written by its own delivery, which removes its row (#878).
- A clock-read census for the web client's TypeScript and the native harness's Swift (#879).
- A source-level reach into another member's code by `#[path]` or `include!`, which no manifest
  states and the crate census cannot see (#880).
- Resolving an inherited `package` rename in `scripts/tests/test_context_map_schematic.py`, which
  reads only a member's own entry (#881).
- Further fitness tests, and the remaining defect-linked lint rules (#882).

## 6. Risks

- **A sibling lands an edge, a read or an entry first.** The census then fails on this branch
  until its row is added: that is the check working. The build re-counts each listing at its cut
  and again before the push, and a count that differs from section 1 is a stop.
- **The lexer misreads a literal form.** A raw string with several `#`, a character literal beside
  a lifetime, or a nested comment could shift what is matched. Each form is a planted case of A9,
  and because counts are exact, a misread shows as a count that differs from its row, never as a
  silent pass.
- **An ADR names an edge only in passing.** The token rule proves that the ADR discusses the
  edge, not that it decided it; the reviewer of the row reads the ADR, and ADR-445 D6 counts the
  rows each list gains.
- **Cargo's manifest grows a key the census does not read.** A dependency spelled through a key
  outside R3 would be an edge the census cannot see; ADR-445 names that as what would make it
  wrong.
- **The interpreter lacks `tomllib`.** The module then fails at import, which is a failure, never
  a pass (ADR-445 D6).

## 7. Mutation rows (S43100-S43199)

Each row is killed by exactly one test, named by its acceptance criterion. Rows on a new module
anchor on a line its green commit writes.

| id | target | mutant | killer |
|---|---|---|---|
| S43100-EDGE-ROW-DROPPED | `config/crate-edges.json` | the `progression -> xp` row removed | A1 |
| S43101-UNADMITTED-MANIFEST-EDGE | `crates/push/Cargo.toml` | `deck-streak-notifications.workspace = true` added | A1 |
| S43102-STALE-EDGE-ROW | `config/crate-edges.json` | a `push -> notifications` row added | A1 |
| S43103-TARGET-EDGE-UNCONDITIONAL | `crates/web-engine/Cargo.toml` | the engine core also under `[dependencies]` | A1 |
| S43104-TARGET-TABLES-UNREAD | `scripts/tests/test_crate_edges.py` | `[target.<key>]` tables skipped | A1 |
| S43105-MEMBERS-EMPTY | `scripts/tests/test_crate_edges.py` | the member patterns match nothing | A1 |
| S43106-DEV-TABLE-UNREAD | `scripts/tests/test_crate_edges.py` | `dev-dependencies` dropped from the tables | A2 |
| S43107-BUILD-TABLE-UNREAD | `scripts/tests/test_crate_edges.py` | `build-dependencies` dropped from the tables | A2 |
| S43108-RENAME-UNRESOLVED | `scripts/tests/test_crate_edges.py` | an entry's `package` ignored | A2 |
| S43109-INHERITED-RENAME-UNRESOLVED | `scripts/tests/test_crate_edges.py` | the `[workspace.dependencies]` entry not read | A2 |
| S43110-PATH-UNRESOLVED | `scripts/tests/test_crate_edges.py` | an entry's `path` ignored | A2 |
| S43111-UNDERSCORE-TABLE-ADMITTED | `scripts/tests/test_crate_edges.py` | the underscore refusal removed | A2 |
| S43112-EDGE-ADR-ABSENT | `config/crate-edges.json` | one row's `adr` an id no file holds | A3 |
| S43113-EDGE-TOKEN-UNNAMED | `docs/decisions/ADR-445-...md` | the token `` `publishing -> kernel` `` altered | A3 |
| S43114-EDGE-TOKEN-UNREAD | `scripts/tests/test_crate_edges.py` | the ADR token check removed | A3 |
| S43115-OWED-ROW-DROPPED | `config/vectors-parity.json` | the `lean/TableCensusReach` row removed | A4 |
| S43116-OWED-ROW-STALE | `config/vectors-parity.json` | a `lean/Wallet` row added | A4 |
| S43117-TLA-VECTORS-ADMITTED | `scripts/tests/test_formal_parity.py` | the TLA+ vectors refusal removed | A4 |
| S43118-ENTRIES-EMPTY | `scripts/tests/test_formal_parity.py` | the entry walk finds nothing | A4 |
| S43119-UNIMPLEMENTED-UNDECLARED | `scripts/tests/test_formal_parity.py` | an entry with no covers passes unlisted | A4 |
| S43120-VECTORS-PATH-MOVED | `crates/economy/tests/formal_vectors_wallet.rs` | the vectors path in `include_str!` renamed | A5 |
| S43121-COMMENT-CITATION-COUNTED | `scripts/tests/test_formal_parity.py` | the citation read with comments kept | A5 |
| S43122-ORPHAN-VECTORS-ADMITTED | `scripts/tests/test_formal_parity.py` | the undeclared vectors file check removed | A5 |
| S43123-CLOCK-ROW-DROPPED | `config/clock-reads.json` | the `crates/kernel/src/clock.rs` row removed | A6 |
| S43124-CLOCK-COUNT-LOWERED | `config/clock-reads.json` | `crates/ingest/src/skip_write.rs` counted 1 | A6 |
| S43125-UNLISTED-CLOCK-READ | `crates/push/src/lib.rs` | a `std::time::SystemTime::now` pointer added | A6 |
| S43126-BUILD-SCRIPT-UNREAD | `scripts/tests/test_clock_reads.py` | `build.rs` left out of the population | A6 |
| S43127-SOURCES-EMPTY | `scripts/tests/test_clock_reads.py` | the source walk finds nothing | A6 |
| S43128-DATE-SPELLING-DROPPED | `scripts/tests/test_clock_reads.py` | `Date` dropped from the clock types | A7 |
| S43129-ALIAS-UNCOLLECTED | `scripts/tests/test_clock_reads.py` | `use ... as` aliases not collected | A7 |
| S43130-CFG-TEST-FILE-READ | `scripts/tests/test_clock_reads.py` | a `#[cfg(test)]` module file read as production | A8 |
| S43131-CFG-TEST-ITEM-READ | `scripts/tests/test_clock_reads.py` | a `#[cfg(test)]` item's body kept | A8 |
| S43132-COMMENT-READ | `scripts/tests/test_clock_reads.py` | comments kept before matching | A9 |
| S43133-CLOCK-TOKEN-UNNAMED | `docs/decisions/ADR-445-...md` | the token `` `crates/daemon/src/snapshot_lister.rs` `` altered | A10 |

## 8. Formal model

Not applicable, decided by surface (ADR-445 D8): each census is one process that reads committed
files and compares sets, with no second actor, no shared state written and no check followed by an
act; and none of the 251 `@phx covers` lines under `formal/` names a path this delivery adds or
changes. Each comparison is shown able to fail by its planted controls, whose expected findings
each fixture spells, and by the rows of section 7.

## 9. What a reading decides, not a test

ADR-445 D6's kill criteria are read, for each check, over the last 30 merged pull requests: the
rows each list gained, from each pull request's diff of the list, and the fixes, the pull requests
on which the check reported a finding and whose diff adds no row to its list. The reader records
each reading beside the window it read; a demotion it proposes goes to the owner as a signed
ruling.

## 10. Amendments: the lists at the rebuild's dev, the as-found vectors rows, and their acceptance criteria

This section amends the requirements above for the rebuild of this delivery on a later `dev`, as
measured at the rebuild's cut. R15's counts are the first base's; R18 supersedes them at the
rebuild's cut.

R18. The three lists hold what the rebuild's cut holds, each row citing `ADR-445`, with no product
     change (ADR-445 D10): 95 edge rows, the 85 of R15 and the ten edges the cut adds; ten formal
     entry rows, eight `owed` (the two of R15 and six the cut adds) and two `unimplemented`; eight
     clock rows, the six of R15 with `crates/ffi/src/engine.rs` counting 2, and the two production
     files the cut adds, each counting 1; and four `as_found` rows (R19). The appendix ADR-445
     adds at the rebuild names each new row by its token.
R19. `config/vectors-parity.json` takes a third key, `as_found`, whose rows are `{"path", "adr"}`,
     sorted by `path`, with no path twice and no unknown key; an absent `as_found` key reads as an
     empty list. Each `path` is a `crates/*/tests/formal_vectors_*.rs` or a
     `formal/vectors/*.jsonl` path. R8 itself is unchanged: an `as_found` row admits only the one
     R8 finding class it is listed for, never every finding on its path; for a vectors test, the
     file-name finding (`<test>: names <vectors path>, so its file name is <expected>`), and for a
     vectors file, the undeclared-file finding (`<path>: a vectors file no Lean entry declares`).
     Any other R8 finding on an `as_found` path stays a finding. A row whose path has no finding
     of its class fails as stale (`<path>: an as_found row with no finding`), so a path of any
     other shape, which can hold no such finding, fails as stale too. Each row's ADR file holds its
     path as `` `<path>` ``, else the row fails as unadmitted
     (`` <path>: <ADR file> does not hold `<path>` ``). From this amendment on, R9's key set reads
     `owed`, `unimplemented` and `as_found`.

| id | criterion | decided by |
|---|---|---|
| A11 | Every `as_found` row admits one vectors finding of its own class on its path, every other finding on that path stays, a row with no such finding is stale, and each row's ADR holds its path (R19) | ADR-445 D10 |

```acceptance
A11: python3 -m unittest discover -s scripts/tests -p test_formal_parity.py -k test_every_as_found_row_admits_a_vectors_finding_of_its_path
```

A11's planted defects, each refused by name with the finding its fixture spells: a finding with no
row; a stale row; a row whose ADR lacks its path; an unknown key in a row; a row out of order; one
path twice; and an R8 finding of another class on an `as_found` path, which stays a finding, on a
vectors test (a path no entry declares) and on a vectors file (a file two entries declare, whose
row is then also stale).

R18 holds again at the dev that this pull request merges when it is checked: the lists gain the
rows that dev adds over the rebuild's cut, each citing ADR-445 and named in the last part of
ADR-445's appendix, with no product change and no test changed.

## 11. Amendment: source-level reaches out of a member, their list, and their acceptance criteria

This section amends the requirements above with a fourth census, which closes the reach section 5
leaves to #880: a member's Rust source that reaches past its own directory by a `path` attribute,
or by an `include!`, `include_str!` or `include_bytes!` call, which no manifest states and the
crate census cannot see (ADR-445 D11). Read at `dev` `b9a08e602fc10c9f3ecd88a1544f3f595d55cbda`:

| what | at `dev` |
|---|---|
| sites | 265: 181 `#[path]`, each on a `mod <name>;`, and 84 `include_str!`; no `include!`, no `include_bytes!`, no `cfg_attr` path, none inside an inline block, and no argument built by `concat!` |
| by kind of code | 27 production, 230 test, 1 bench, 7 example, none in a build script |
| own | 80, each target in its own member's directory |
| into another member | 5, each a `#[path]` in a test |
| into files no member owns | 180: 126 into four Rust files under `tools/`, which hold no site, and 54 into data files |
| reach keys | 96, over 95 `from -> to` pairs |
| refused | 0 |

The census prints its own counts, and the red-first record holds them at this amendment's cut.

R20. The census's population is every `.rs` file under each workspace member's directory (R2's
     members), printed as `examined <n> member Rust file(s)`, and 0 is VOID; then each existing
     file outside every member's directory that a module's `path` attribute or an `include!`
     names, read in turn until no new file is named, printed as `listed <n> brought-in file(s)`.
     A site is a `path` attribute, outer or inner, set directly or through `cfg_attr` at any
     depth, or a call of `include!`, `include_str!` or `include_bytes!` with any path prefix and
     any of the three brackets. Sites are matched on the source with comments and literal
     contents blanked by R11's lexer, and each literal is read back from the source at its span.
R21. A site's target resolves as the compiler resolves it. An include's literal, and a module's
     `path` outside every inline `mod` block, resolve from the directory of the file that holds
     the site. A `path` inside inline blocks, and an inline block's own `path`, resolve from that
     directory, then the file's stem unless the file is a crate root or a `mod.rs`, then each
     enclosing block's `path`, else its name. `concat!(env!("CARGO_MANIFEST_DIR"), <literals>)`
     is the member's directory followed by the joined literals, and `concat!(env!("OUT_DIR"),
     ...)` is the member's build output. Each `..` is taken against the part before it.
R22. A site whose target sits in its own member's directory, or in its build output, is own: it is
     counted and printed as `listed <n> own site(s)`, with no row, because the file is the
     member's own and the crate graph holds no edge from a member to itself. Every other site is
     a reach, keyed `(from, to, code)`: `from` is the member's name (R2) or a brought-in file's
     path, `to` is the target's path, and `code` is the kind of code that holds the site:
     `production` (under `src/`), `test` (under `tests/`, or under `src/` in a module file or an
     item R12 sets aside as test code), `bench`, `example`, `build` (the member's `build.rs`),
     `other` (anywhere else in the member) or `outside` (a brought-in file).
R23. `config/source-reaches.json` holds `{"reaches": [...]}`, each row
     `{"from", "to", "code", "count", "adr"}`, sorted by `from`, `to` and `code`, with no key
     twice, no unknown or missing key, `from`, `to` and `code` strings, and `count` a whole number
     above 0. Each reach key has exactly one row whose `count` equals its sites, and every row has
     its sites. A finding names the key as `<from> -> <to> [<code>]` and each site as
     `<path>:<line>`: `<n> site(s) and no row in config/source-reaches.json: <sites>`,
     `<n> site(s), its row counts <m>: <sites>`, or `a row with no reach`.
R24. Each row's `adr` is `ADR-<n>`, held by exactly one `docs/decisions/ADR-<n>-*.md` file whose
     text holds `` `<from> -> <to>` ``, else the row is unadmitted. Each of the 96 rows cites
     `ADR-445`, whose appendix names its pair.
R25. A site the census cannot resolve is refused, and no row can admit it: a `path` attribute
     whose value is not one string literal, an inner `path` attribute, an include whose argument
     is neither one literal nor R21's `concat!` form, a literal with an escape, an absolute path,
     a path above the repository root, and an `env!` form in a brought-in file. A refusal reads
     `<path>:<line>: <form>, <reason>, is refused`.

| id | criterion | decided by |
|---|---|---|
| A12 | Every reach out of its member has one row with its exact count and every row its sites; the population is every member's Rust file, printed, and 0 is VOID (R20, R22, R23) | ADR-445 D11 |
| A13 | Each site form, each kind of code, each inline-block and manifest-relative path and each brought-in file resolves to its key, own sites are not reaches, comments and literals hold no site, and each unresolvable path is refused (R20, R21, R22, R25) | ADR-445 D11 |
| A14 | Every reach row is admitted by an ADR file that names it (R24) | ADR-445 D11 |

```acceptance
A12: python3 -m unittest discover -s scripts/tests -p test_source_reaches.py -k test_every_reach_out_of_its_member_has_one_row_and_every_row_its_sites
A13: python3 -m unittest discover -s scripts/tests -p test_source_reaches.py -k test_each_form_kind_and_path_resolves_and_an_unresolvable_path_is_refused
A14: python3 -m unittest discover -s scripts/tests -p test_source_reaches.py -k test_every_reach_row_is_admitted_by_an_adr_that_names_it
```

Each test's planted defects, each refused by name with the finding its fixture spells:

- A12: an unlisted reach; a row whose count differs; a row with no reach; a duplicated row; a row
  out of order; an unknown key; a missing key; a count that is not a whole number; an absent
  list; a workspace with no member Rust file (VOID).
- A13: an outer `path` attribute in test code; a `cfg_attr` path in production code; `include!` in
  production code; `include_str!` with a path prefix in a bench; `include_bytes!` in brackets in
  an example; `include_str!` in braces in a build script; a raw string elsewhere in the member; a
  `#[cfg(test)]` item in production code; a module file declared under `#[cfg(test)]`; a `path`
  in an inline block of a crate root and of a module file; an inline block's own `path`; a
  manifest-relative `concat!`; a file outside the members and the reach it brings in; and, as
  passes, a site inside its own member, a climb back into its own member, the build output, and
  comments and literals. Refused: a `path` value that is not a literal, an inner `path`, an
  argument the census cannot resolve, an escape, an absolute path, a path above the repository
  root, and an environment-relative path in a brought-in file.
- A14: a row citing an absent ADR; a row citing an ADR whose text lacks the token; a malformed
  `adr`; an id two files share.

What the census does not see, each named so a reader meets it here: a file a link names, since
the census reads the tree as stored; a file-reading macro from another crate; a manifest target's
`path` key and `[package] build`, which name files the crate graph reads; a plain `mod <name>;`
inside a brought-in file; whether the compiler reads a file a `path` attribute loads as a crate
root, since the census reads each file by its own name; a file whose test-only `cfg` is an inner
attribute, which R12 reads as production; and an `include!` target inside another member that is
not a `.rs` file, which is keyed but not read. A module file one member loads from another
member's directory is read once, as the other member's file.

File manifest of this amendment (section 4's rows):

| path | part | change |
|---|---|---|
| `scripts/tests/test_source_reaches.py` | scripts | added: the source-reach census, A12 to A14 |
| `config/source-reaches.json` | config | added: the 96 reach rows (R23) |
| `docs/specs/SPEC-431-three-fitness-tests-hold-the-crate-graph-the-formal-entries-vectors-tests-and-the-production-clock-reads-to-listed-rows.md` | docs | changed: this section |
| `docs/decisions/ADR-445-each-fitness-test-is-its-own-census-over-a-listed-file-and-every-row-names-the-adr-that-admits-it.md` | docs | changed: D11 and its appendix |
| `docs/schematics/fitness-tests-crate-edges-formal-vectors-and-clock-reads.md` | docs | changed: section 5 |
| `docs/red-first/SPEC-431.md` | docs | changed: the record of A12 to A14 |
| `scripts/mutation-rows.d/S43100-S43199.json` | scripts | changed: rows S43137 to S43164 |
| `changelog.d/source-reaches-431.md` | changelog | added: the fragment |

No other path changes: no product file, no `Cargo.toml`, no workflow, neither
`scripts/tests/test_crate_edges.py` nor `config/crate-edges.json`, neither
`scripts/tests/test_clock_reads.py` nor `scripts/tests/_support.py`, and not
`scripts/tests/test_ci_workflows.py`, since the census loads no module by a name held in data.

Mutation rows of this amendment, each killed by exactly one test, named by its acceptance
criterion; rows on the new module anchor on a line its green commit writes:

| id | target | mutant | killer |
|---|---|---|---|
| S43137-REACH-ROW-DROPPED | `config/source-reaches.json` | the `daemon -> crates/bot/tests/support/fake_bot_api.rs` row removed | A12 |
| S43138-REACH-COUNT-LOWERED | `config/source-reaches.json` | `coordination -> tools/parity-oracle/golden.rs` counted 15 | A12 |
| S43139-STALE-REACH-ROW | `config/source-reaches.json` | a `bot -> crates/streaks/src/lib.rs` row added | A12 |
| S43140-UNLISTED-SOURCE-REACH | `crates/bot/tests/streak_commands.rs` | the `#[path]` into `streaks` retargeted | A12 |
| S43141-PATH-ATTRIBUTE-UNREAD | `scripts/tests/test_source_reaches.py` | the `path` attribute never matched | A13 |
| S43142-CFG-ATTR-PATH-UNREAD | `scripts/tests/test_source_reaches.py` | `cfg_attr` never read | A13 |
| S43143-INCLUDE-BYTES-UNREAD | `scripts/tests/test_source_reaches.py` | `include_bytes` dropped from the names | A13 |
| S43144-INCLUDE-MACRO-UNREAD | `scripts/tests/test_source_reaches.py` | `include` dropped from the names | A13 |
| S43145-REACH-COMMENT-READ | `scripts/tests/test_source_reaches.py` | sites matched on the unlexed text | A13 |
| S43146-TEST-CODED-PRODUCTION | `scripts/tests/test_source_reaches.py` | test code under `src/` coded as production | A13 |
| S43147-REACH-TEST-ITEM-UNREAD | `scripts/tests/test_source_reaches.py` | the `#[cfg(test)]` item check removed | A13 |
| S43148-REACH-TEST-FILE-UNREAD | `scripts/tests/test_source_reaches.py` | the `#[cfg(test)]` module file check removed | A13 |
| S43149-OWN-SITE-COUNTED-AS-REACH | `scripts/tests/test_source_reaches.py` | own sites keyed as reaches | A13 |
| S43150-REFUSAL-DROPPED | `scripts/tests/test_source_reaches.py` | refusals dropped | A13 |
| S43151-ESCAPE-ADMITTED | `scripts/tests/test_source_reaches.py` | the escape check removed | A13 |
| S43152-ABSOLUTE-PATH-ADMITTED | `scripts/tests/test_source_reaches.py` | the absolute path check removed | A13 |
| S43153-ROOT-CLIMB-ADMITTED | `scripts/tests/test_source_reaches.py` | a climb above the root kept as spelled | A13 |
| S43154-INLINE-BLOCK-UNREAD | `scripts/tests/test_source_reaches.py` | the inline block names dropped | A13 |
| S43155-MODULE-STEM-DROPPED | `scripts/tests/test_source_reaches.py` | the file stem dropped | A13 |
| S43156-MANIFEST-DIRECTORY-DROPPED | `scripts/tests/test_source_reaches.py` | the member directory dropped | A13 |
| S43157-BROUGHT-IN-UNREAD | `scripts/tests/test_source_reaches.py` | brought-in files never read | A13 |
| S43158-BUILD-SCRIPT-UNCODED | `scripts/tests/test_source_reaches.py` | the build kind removed | A13 |
| S43159-MEMBER-FILES-EMPTY | `scripts/tests/test_source_reaches.py` | the member walk finds nothing | A12 |
| S43160-REACH-COUNT-UNCHECKED | `scripts/tests/test_source_reaches.py` | the count check removed | A12 |
| S43161-STALE-REACH-ROW-PASSED | `scripts/tests/test_source_reaches.py` | the stale row check removed | A12 |
| S43162-REACH-TOKEN-UNREAD | `scripts/tests/test_source_reaches.py` | the ADR token check removed | A14 |
| S43163-REACH-ADR-ABSENT | `config/source-reaches.json` | one row's `adr` an id no file holds | A14 |
| S43164-REACH-TOKEN-UNNAMED | `docs/decisions/ADR-445-...md` | the token `` `engine-core -> crates/web-engine/src/study.rs` `` altered | A14 |

Formal model: not applicable, decided by surface (ADR-445 D8). The census is one process that
reads committed files and compares sets, with no second actor, no shared state written and no
check followed by an act, and none of the 364 `@phx covers` lines under `formal/` at `dev` names
a path this amendment adds or changes.

ADR-445 D6's kill criteria hold for `config/source-reaches.json` as a fourth list, with its own
precision figure over the last 30 merged pull requests.

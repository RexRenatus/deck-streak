---
status: accepted
decision-makers: "the owner, the DeckStreak architect"
---

# Each fitness test is its own census over a listed file, and every row names the ADR that admits it

## Context and Problem Statement

SPEC-431 adds three fitness tests to DeckStreak's own checks. Each holds one architectural fact of
the tree to a committed list, so a change that breaks the fact fails by name until an ADR admits
it. Read at `dev` `b25ed04ab10f24a89a1ec0eed71f8a365f411521`:

- **The crate graph.** The workspace manifest's `members = ["crates/*"]` names 31 members. Their
  manifests name 85 internal edges, every one under `[dependencies]`, one of them
  (`web-engine -> engine-core`) under `[target.'cfg(target_arch = "wasm32")'.dependencies]`, and
  none under `[dev-dependencies]` or `[build-dependencies]`. The fence in `docs/CONTEXT-MAP.md` is
  the binding map, and `scripts/tests/test_context_map_schematic.py` (SPEC-394) holds the
  manifests, the fence and the drawing equal. Nothing records which ADR admitted each edge, so an
  edge can join the manifest, the fence and the drawing in one change with no decision behind it.
  The census reads no `[dev-dependencies]` table, and it reads a member's own `package` key, never
  the `package` of the `[workspace.dependencies]` entry a `workspace = true` line inherits.
  `scripts/tests/test_xp_crate_graph.py` (SPEC-360) holds the per-review XP crate alone.
- **Formal entries and their vectors tests.** The formal checker derives its entries from the
  tree: 13 Lean modules `formal/lean/Formal/<E>.lean` that carry `-- @phx` header lines, and 27
  TLA+ models `formal/tla/<E>/<E>.tla`. Every one of the 40 carries at least one `@phx covers` line,
  the field that names the code it ports or models. Each Lean entry declares one
  `-- @phx vectors formal/vectors/<slug>.jsonl`, and 13 vectors files exist. Eleven tests
  `crates/*/tests/formal_vectors_*.rs` read a vectors file, each naming its path once in a string
  literal. `lean/SensitiveDeck` and `lean/TableCensusReach` write vectors that no test reads.
  `config/formal.json` holds the checker's settings and, by SPEC-295, no other field.
- **Clock reads.** SPEC-020 R6 makes `SystemClock` the only reader of the system time in the
  workspace's production code. Seven reads of a clock sit in six production Rust files (the
  appendix names each): four of `SystemTime::now`, one of `js_sys::Date::now` and, monotonic, two
  of `Instant::now`, one of them `tokio::time::Instant::now`. One more `SystemTime::now` sits in
  `crates/ingest/src/skip_write/tests.rs`, a module `skip_write.rs` declares under `#[cfg(test)]`.
  No check holds any of them.

## Decision Drivers

- A new edge, a new clock read or a formal entry left without its vectors test fails by name, and
  the message names the row to add and the ADR it must cite.
- Each check lands at zero by listing what the tree holds today, and changes no product code.
- Each check prints what it examined and refuses an empty population, so a check that went blind
  fails on its own.
- Each check is read alone: its own module, its own list, its own rows and its own kill criteria.
- A defect a check finds today is a question for its own delivery, never a cure folded in here.

## Considered Options (the alternatives it was chosen against)

### D1, the crate graph

- Chosen: a register of admitted edges, `config/crate-edges.json`, held equal to the manifests in
  both directions by a new census, `scripts/tests/test_crate_edges.py`, because it adds what the
  fence does not carry (the admitting ADR, the table and the target) without changing the fence
  or the two censuses that read it. Its shape is `{"edges": [row, ...]}`, each row
  `{"from": <member>, "to": <member>, "table": <table>, "target": <key or null>, "adr": "ADR-<n>"}`,
  where a member is named by its package name less the `deck-streak-` prefix, `table` is one of
  `dependencies`, `dev-dependencies` and `build-dependencies`, and `target` is a `[target.<key>]`
  key as the manifest spells it. Rows are sorted by `from`, `to`, `table` and `target`, and no key
  repeats.
  - The population is every workspace member: each directory a `[workspace] members` pattern
    matches, less `exclude`, that holds a `Cargo.toml` (31 at `dev`). A member whose package name
    lacks the prefix is a finding, and so is a pattern that matches no manifest.
  - The tables are `[dependencies]`, `[dev-dependencies]` and `[build-dependencies]`, each also
    under every `[target.<key>]`. A table spelled with an underscore (`dev_dependencies`,
    `build_dependencies`) is a finding by name, so the census never guesses at a spelling.
  - An entry is internal when it resolves to a member: by its package (the entry's `package`, else
    its key) or by its `path` (relative to the member's directory). A `workspace = true` entry
    resolves through its `[workspace.dependencies]` entry, whose `package` and `path` it inherits,
    since a member may add only `optional`, `features` and `default-features` to it. An entry that
    names a member's package from a registry or a git source counts too, so the census refuses
    rather than passes.
  - Every manifest edge has exactly one row, and every row has its edge. A row with no edge behind
    it fails, as an unlisted edge does.
  - Each row's `adr` names exactly one file `docs/decisions/ADR-<n>-*.md`, and that file's text
    holds the edge as `` `<from> -> <to>` ``. Today's 85 rows cite this ADR, whose appendix names
    each; a later edge cites the ADR its delivery writes.
  - The known classes: a `package =` rename and a `workspace = true` inheritance are resolved, an
    `optional = true` edge is an edge, a `[features]` entry adds none (it enables an edge already
    declared), and a target table is part of the row's key. A `#[path]` module include or an
    `include!` that reaches into another member's source is an edge no manifest states, so the
    census cannot see it; nor does it see a `[patch]` that points an external package at a member.

Chosen against:

- Warning on a row with no edge: lost, because a stale row is a standing admission, and the next
  edge it matches would enter with no ADR read.
- Generalising `test_xp_crate_graph.py` to the whole graph: lost, because its tests hold facts of
  one crate (its closure, its I/O names, its one dependent), and widening them would edit
  assertions and rows that pin those facts.
- An ADR column in the fence of `docs/CONTEXT-MAP.md`: lost, because two censuses parse each fence
  row's `depends on:` list, and the fence states no table and no target.
- Reading the graph with `cargo metadata`: lost, because the census reads committed files and
  starts no process, as the house's other censuses do, and a toolchain-dependent reader adds a way
  for the check to be VOID.
- Excluding `[dev-dependencies]`, as the fence does: lost, because a test that reaches another
  context is still an edge an ADR should admit, and none exists today, so holding it costs nothing.
- Accepting any existing ADR id: lost, because a row could then cite an ADR that never discussed
  its edge.

### D2, formal-to-code parity

- Chosen: a new census, `scripts/tests/test_formal_parity.py`, with a list of declared
  exceptions, `config/vectors-parity.json`, because the entry's own header already states both
  facts the parity needs, and the checker reads the same lines. An entry has an implementation
  when its header carries at least one `@phx covers` line; it writes vectors when it carries
  `-- @phx vectors <path>`.
  - The population is every entry the checker derives: each `formal/lean/Formal/<E>.lean` whose
    leading comment lines carry a `-- @phx` line, and each `formal/tla/<E>/<E>.tla`. It prints 40
    entries (13 Lean, 27 TLA), the vectors tests and the vectors files it examined.
  - A Lean entry is the kind that writes vectors. A TLA+ entry is model-checked over its own state
    space and writes none, and its tie to code is the checker's `covers` digest; a TLA+ entry that
    declares `@phx vectors` is a finding.
  - Entry to test: every implemented Lean entry declares exactly one vectors path, that file
    exists, and exactly one `crates/*/tests/formal_vectors_*.rs` names it; or the entry has an
    `owed` row.
  - Test to entry: every `crates/*/tests/formal_vectors_*.rs` names exactly one
    `formal/vectors/<slug>.jsonl` inside a string literal, outside every comment; its file name
    reads `formal_vectors_<slug>.rs` with each `-` of the slug as `_`; and the path is the declared
    vectors path of exactly one live Lean entry. Every `formal/vectors/*.jsonl` is declared by
    exactly one entry.
  - The list is `{"owed": [row, ...], "unimplemented": [row, ...]}`, each row
    `{"entry": "lean/<E>", "adr": "ADR-<n>"}`, sorted, with no key twice. `owed` holds an
    implemented entry with no vectors test. `unimplemented` holds a Lean entry with no `@phx covers`
    line, so such an entry is declared, never silently exempt. A row whose condition no longer holds
    fails as stale. Each row's ADR names the entry as `` `lean/<E>` ``. Today `owed` holds
    `lean/SensitiveDeck` and `lean/TableCensusReach`, and `unimplemented` is empty.

Chosen against:

- An implementation field in `config/formal.json`: lost, because SPEC-295 holds that file to the
  fields the checker reads and no other, and its census refuses any other field.
- Holding TLA+ entries to a vectors test: lost, because a model writes no vectors, so every one of
  the 27 would need a row that says nothing.
- Citing the entry by its Lean module path in the test's doc comment: lost, because a comment can
  name an entry the test never reads, while the vectors path in a string literal is the file the
  test opens; one of the eleven tests names no Lean path at all.
- A list of every entry-test pair: lost, because the pairs are derived from the tree, and a typed
  copy of them is a second record that can drift.

### D3, the clock-read allow-list

- Chosen: a new census, `scripts/tests/test_clock_reads.py`, with `config/clock-reads.json`, whose
  unit is a file and its exact count of reads, because a new read in a listed file is still a new
  read, and an exact count also fails a row the code has outgrown.
  - The matcher removes comments (line, nested block and doc comments, so a doc test goes with its
    comment) and the contents of string, raw string, byte string and character literals, then
    matches `<T>::now` where `<T>` is `SystemTime`, `Instant`, `Utc`, `Local` or `Date`, or
    `OffsetDateTime::now_utc` and `OffsetDateTime::now_local`. Any path prefix is allowed
    (`std::time::`, `tokio::time::`, `chrono::`, `js_sys::`), whitespace around `::` is allowed, a
    name may not continue on either side, and a reference with no call (a function pointer) is a
    read. Every name a `use ... as <alias>` binds to one of those types, or a
    `type <alias> = ...;` makes one, is collected over the whole population first, and
    `<alias>::now` is a read in every file, so a re-export under another name is seen.
  - Test code is outside the population: a file under a member's `tests/`, `examples/` or
    `benches/` directory (never opened); a module file a production file declares under
    `#[cfg(test)]` or `#[cfg(all(test, ...))]`, by `mod <name>;` or by `#[path = "..."]`, and the
    modules beneath it; and an item under one of those attributes inside a production file, whose
    balanced body is removed before matching. `#[cfg(not(test))]` and `#[cfg(any(test, ...))]` mark
    production code.
  - The population is every `.rs` file under a member's `src/`, and its `build.rs`, that is not test
    code. The census prints the files it read, the files it set aside as test code and the reads
    it found.
  - The list is `{"sites": [row, ...]}`, each row
    `{"path": <path>, "count": <n>, "adr": "ADR-<n>", "why": <text>}`, sorted by path, with no path
    twice. A file whose count differs from its row in either direction fails, and so does a row
    for a file with no read. Each row's ADR names its path as `` `<path>` ``.
  - The known classes: a comment, a string and a doc test are never reads; a `macro_rules!` body
    that spells a read counts once where it is written, and one that builds the path from its
    arguments is not seen; an `.elapsed()` call reads the clock with no `now`, and every
    production one today is on an `Instant` its own file's listed read made, so a `SystemTime`'s
    `.elapsed()` is the class the census does not see; a type of DeckStreak's own named like a
    clock type with a `now` function reads as a clock read, which refuses rather than passes;
    clocks read inside a library DeckStreak calls are not DeckStreak's reads; and a production
    file included by `#[path]` from outside `src/` is not walked (both such includes today are
    under `#[cfg(test)]`).

Chosen against:

- A file-only unit: lost, because a second read in a listed file would pass unread.
- A module path unit: lost, because the walk reads files, and a module's file is what a review
  sees in a diff.
- The four spellings alone (`SystemTime`, `Utc`, `Local`, `Instant`): lost, because
  `js_sys::Date::now` reads the wall clock in `crates/web-engine/src/wasm.rs` and they miss it.
- Matching `.elapsed()`: lost, because the receiver's type is not in the text, and every
  production call today is already tied to a listed read.
- Treating every file under `src/` as production: lost, because
  `crates/ingest/src/skip_write/tests.rs` is a `#[cfg(test)]` module and would read as a
  production site.

### D4, the homes and their population

- Chosen: three modules under `scripts/tests/`, `test_crate_edges.py`, `test_formal_parity.py` and
  `test_clock_reads.py`, each reading committed files with the standard library and `_support`'s
  `REPO` and `examined`, and each holding its own list under `config/`, because each check then
  fails, is listed and is measured alone. Each test judges the real tree first, then prints what
  it examined (0 is VOID through `examined`), then refuses each planted defect by name. No module
  imports, runs or reaches code by a name held in data, so none adds a site to the register of such
  sites in `scripts/tests/test_ci_workflows.py`, which this delivery does not change. CI's existing
  run of `scripts/tests` discovers the three modules; no workflow changes.

Chosen against:

- One combined module for the three checks: lost, because one module's failure would hide the
  other two checks' readings, and each check's rows and kill criteria are counted apart.
- A clippy rule, `disallowed-methods` for the clock types in `clippy.toml`: lost, because every
  listed read and every test read would carry an `#[expect]` in product code, the native lint run
  never compiles the `wasm32` module that reads `Date::now`, and clippy reads neither manifests
  nor formal headers.
- A cargo-deny rule, `[bans]` with wrappers for each member: lost, because it reads the resolved
  graph with no ADR per edge, skips development dependencies by default, and couples the check to
  the audit stage's tool.
- A Rust test in a member crate: lost, because reading every member's manifest from one crate's
  tests would make that crate's tests depend on the whole workspace's layout, and a Rust test would
  run in the Rust leg rather than beside the other censuses.

### D5, the listings at `dev`

- Chosen: each check lands at zero by listing what the tree holds at `dev`
  `b25ed04ab10f24a89a1ec0eed71f8a365f411521`, and no product code changes, because a fitness test
  that cured what it found would ship its first rows as code changes nobody asked for. The
  listing is 85 edges, two owed entries and seven reads in six files (the appendix). Of these,
  three look like defects, and each is listed as found, its cure left to its own delivery:
  `crates/ingest/src/skip_write.rs` reads `SystemTime::now` twice though it may take the kernel's
  `Clock`, which SPEC-020 R6 asks of production code; and `lean/SensitiveDeck` and
  `lean/TableCensusReach` write vectors that no test reads. The reads in `crates/ffi/src/engine.rs`
  and `crates/web-engine/src/wasm.rs` are the client adapters' own clocks, outside SPEC-020 R6's
  `SystemClock` because neither crate depends on the kernel (ADR-345 D1, ADR-348); the two
  `Instant::now` reads are monotonic and read no system time.

Chosen against:

- Curing the hits in this delivery: lost, because each cure is a behaviour change with its own
  tests, and folding it in would make the fitness test's first diff a product change.
- Leaving a hit unlisted so the check lands red: lost, because a check that is red on arrival
  blocks every pull request until the cure lands, and the cure is not this delivery's.

### D6, kill criteria

- Chosen: for each check, over the last 30 merged pull requests: live precision is fixes ÷ (fixes
  + rows added to that check's list), where a fix is a merged pull request on which the check
  reported a finding and whose diff adds no row to that list; below 0.8 with at least 5 hits
  (fixes plus rows added) proposes a demotion. More than 2 rows added to one check's list proposes
  a review of that check. A check that prints `examined 0`, or that cannot run because its
  interpreter or a module it imports is missing or off its pin, is VOID, and VOID blocks; it is
  never a pass. A demotion is a weakening, and goes to the owner as a signed ruling. A window
  reads "the last N merged pull requests", never a number of days.

Chosen against:

- A window counted in days: lost, because a quiet stretch would read as precision.
- One precision figure for the three checks together: lost, because a busy list would hide a
  check that is only ever listed and never fixed.
- A precision bar with no hit floor: lost, because the first listed row would read as precision 0
  and propose a demotion on one data point.

### D7, what this delivery does not do

- Chosen: the three checks alone, with no other fitness test, no other lint rule, no cure of a
  hit D5 lists, no new package (the checks use the standard library only), and no check of the
  web client's or the native harness's clock reads, because each of those is its own decision and
  SPEC-431 section 5 cites each.

Chosen against:

- Adding a Swift or TypeScript clock check beside the Rust one: lost, because those languages
  need their own lexers and their own lists, and their reads have not been measured.

### D8, tests, rows and the formal model

- Chosen: one test per acceptance criterion, each red first at a census whose finders return no
  finding, so each planted edge, entry or read in its fixture goes unrefused, never at a plant in
  the product tree; no assertion reads a wall clock. The rows are `S43100` onward in
  `scripts/mutation-rows.d/S43100-S43199.json`, each killed by exactly one of the tests. Formal:
  not applicable, because each census is one process reading committed files and comparing sets:
  no second actor, no shared state written, no check followed by an act; and none of the 251
  `@phx covers` lines under `formal/` names a path this delivery adds or changes.

Chosen against:

- A TLA+ model of a census: lost, because it has one actor and no interleaving, so a model would
  restate the test.
- A Lean proof of a census's comparison: lost, because the censuses are no algorithm a gate
  decides by, and their planted controls, whose expected findings are spelled in each fixture,
  and their rows measure that each comparison can fail.
- Reds planted in product files: lost, because a plant in the product tree is a product change,
  and a fixture states the same defect with no side effect.

### D9, the shape of the record

- Chosen: one delivery under SPEC-431 and ADR-445 carrying all three checks, with no amendment to
  another SPEC or ADR, because the three share one review rule (a row names the ADR that admits
  it) and one set of kill criteria, and none of them changes a decision already recorded.

Chosen against:

- Amending SPEC-020 R6 to name the listed reads: lost, because the list is the register R6's
  exceptions are read from, and rewording R6 is a decision about the ingest reads that D5 leaves to
  its own delivery.
- Amending `docs/CONTEXT-MAP.md` to name the register: lost, because the fence stays the binding
  map, and the census's failure message names the register and the rule where a reader meets it.
- Three deliveries, one per check: lost, because the review rule and the kill criteria would then
  be decided three times.

## Decision Outcome

Chosen options: D1 to D9 as each "Chosen:" bullet states, because together they make the crate
graph, the formal entries' vectors tests and the production clock reads three listed facts, each
held by its own census that names what it examined, and each changed only by a row that cites the
ADR admitting it.

### Consequences

- Good: an edge, a clock read or an entry without its vectors test that no ADR admits fails by
  name, and the message names the list, the row to add and the token the ADR must hold.
- Good: a development dependency between contexts and an edge moved off its target table are now
  reviewed, though the fence states neither.
- Good: the two entries whose vectors no test reads, and the two ingest reads beside the kernel's
  clock, are recorded where every later reader of the lists sees them.
- Bad: a new internal edge now touches four places: its manifest, the fence, the drawing and the
  register, with the ADR that names it.
- Bad: the clock census carries its own Rust lexer, which a change of Rust's literal syntax could
  outgrow.
- Bad: this ADR's appendix grows stale as edges leave; a row that leaves needs no edit here, since
  the census reads only from row to ADR.

### Confirmation

SPEC-431's acceptance criteria, each a test of one of the three modules, red at the census commit
for its own reason (`docs/red-first/SPEC-431.md`), and the rows of
`scripts/mutation-rows.d/S43100-S43199.json`, each killed by exactly one of those tests.

### What would make this wrong

- A member whose internal edges Cargo resolves in a way the census does not read, such as an
  inherited key Cargo adds to the ones a member may set.
- A clock read spelled in a form D3's matcher cannot see that becomes common in production code.
- A formal entry kind other than Lean that writes vectors.
- Live precision below D6's bar on any check.

## More Information

#639 (the clients' edges to `xp`) and #640 (`push`'s edge to `notifications` and the daemon's to
`push`) add rows with their own ADRs. SPEC-020 R6, SPEC-295, SPEC-360, SPEC-394 and ADR-408 are
read and not changed.

## Appendix: the rows this ADR admits

The census reads each token below exactly as it is spelled, so each sits on one line.

Every internal edge at `dev` `b25ed04ab10f24a89a1ec0eed71f8a365f411521`, each a `[dependencies]`
row, `web-engine -> engine-core` under the `wasm32` target table:

- `agent -> kernel`.
- `analytics -> ingest`, `analytics -> kernel`.
- `api -> coordination`, `api -> identity`, `api -> kernel`, `api -> notifications`.
- `bot -> coordination`, `bot -> identity`, `bot -> kernel`, `bot -> notifications`.
- `coordination -> agent`, `coordination -> analytics`, `coordination -> curriculum`,
  `coordination -> discipline`, `coordination -> economy`, `coordination -> focus`,
  `coordination -> habits`, `coordination -> identity`, `coordination -> ingest`,
  `coordination -> insights`, `coordination -> kernel`, `coordination -> markets`,
  `coordination -> notifications`, `coordination -> privacy`, `coordination -> progression`,
  `coordination -> publishing`, `coordination -> quests`, `coordination -> readings`,
  `coordination -> streaks`, `coordination -> vault`.
- `curriculum -> ingest`, `curriculum -> kernel`.
- `daemon -> agent`, `daemon -> analytics`, `daemon -> api`, `daemon -> bot`,
  `daemon -> coordination`, `daemon -> curriculum`, `daemon -> discipline`, `daemon -> economy`,
  `daemon -> focus`, `daemon -> habits`, `daemon -> identity`, `daemon -> ingest`,
  `daemon -> insights`, `daemon -> kernel`, `daemon -> markets`, `daemon -> mcp`,
  `daemon -> notifications`, `daemon -> privacy`, `daemon -> progression`, `daemon -> publishing`,
  `daemon -> quests`, `daemon -> readings`, `daemon -> streaks`, `daemon -> vault`.
- `discipline -> ingest`, `discipline -> kernel`.
- `economy -> kernel`.
- `engine-core -> fsrs7`.
- `ffi -> engine-core`.
- `focus -> kernel`.
- `habits -> kernel`.
- `identity -> kernel`.
- `ingest -> kernel`.
- `insights -> ingest`, `insights -> kernel`.
- `markets -> kernel`.
- `mcp -> coordination`, `mcp -> kernel`.
- `notifications -> kernel`.
- `privacy -> kernel`.
- `progression -> ingest`, `progression -> kernel`, `progression -> xp`.
- `publishing -> kernel`.
- `push -> kernel`.
- `quests -> ingest`, `quests -> kernel`.
- `readings -> ingest`, `readings -> kernel`.
- `streaks -> kernel`.
- `vault -> kernel`.
- `web-engine -> engine-core`.

Every file with production clock reads, with its count:

- `crates/agent/src/runner.rs`: 1, the agent runner's elapsed time, `tokio::time::Instant::now`.
- `crates/daemon/src/snapshot_lister.rs`: 1, the snapshot lister's time budget, `Instant::now`.
- `crates/ffi/src/engine.rs`: 1, the native adapter's answer time, `SystemTime::now`.
- `crates/ingest/src/skip_write.rs`: 2, the skip write's preview and its instant now,
  `SystemTime::now`.
- `crates/kernel/src/clock.rs`: 1, `SystemClock`, `SystemTime::now`.
- `crates/web-engine/src/wasm.rs`: 1, the web engine's wall clock, `js_sys::Date::now`.

Every implemented Lean entry with no vectors test:

- `lean/SensitiveDeck`.
- `lean/TableCensusReach`.

# SPEC-120: the collection atlas is a series refreshed from changed cards after each sync, and the agent reads it in pages

- **Wave:** W6. **Issue:** #282 (the collection atlas as a data series the agent reads) (epic #7).
  **Context(s):** `deck-streak-ingest` (the atlas reads of the copy); `deck-streak-insights` (the
  series, its family rule, its refresh, its pages and the tables `collection_atlas` and
  `collection_atlas_state`); `deck-streak-coordination` (the refresh after a sync, and the page read
  the adapters call); `deck-streak-mcp` (the resources); `deck-streak-daemon` (the wiring).
- **Decided by:** ADR-054 (no-AI mode is the default), ADR-059 (public text describes DeckStreak
  only), ADR-085 (no chart image is rendered on the host), ADR-119 and ADR-121 (the MCP server and
  its guard) and ADR-120 (the atlas is a table refreshed from the cards whose stamp changed, and
  read in pages).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-022, SPEC-023, SPEC-041, SPEC-071, SPEC-077, SPEC-085
  and SPEC-119. **Mutation band:** `S12000-S12099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-120.md` (ADR-016).

## 1. The problem, measured

- **The predecessor's atlas** (`charts.py:read_collection_atlas`, at `27ee2bc`) reads every card of
  the collection per request, ordered by note creation (`nid, ord, id`), with the card's deck, its
  deck family (`charts.py:_deck_family`: the deck name's top-level segment, `(unknown)` for an empty
  or missing name) and its memory state (stability, decay, the last review, from
  `fsrs.py:parse_fsrs`), as of the start of the study day holding the request
  (`charts.py:_rollover_instant`). A cards read that fails answers no rows marked unmeasured, never
  a silent empty collection. The agent read it as an image (`server.py:_chart_collection_atlas`).
- **SPEC-085 left it out** (R11, A14, #270); the owner revived it at #282 as a data series: never an
  image, never published, read by the agent.
- **A whole read per request does not fit.** The collection is the largest thing the host reads,
  and the memory budget allows no per-request read and parse of every card. The series is held in
  the ledger, refreshed after each sync from the cards whose stamp changed, and read in pages
  (ADR-120).
- **A mark on the stamp would miss changes.** A card changed on another device keeps that device's
  stamp (`cards.mod`) when it syncs in, so a change made offline before the last refresh arrives
  with a stamp below any mark. The refresh therefore compares each card's stamp with the stamp its
  row holds, in either direction (ADR-120).
- **What already exists.** SPEC-077 R1 publishes each card's memory state from `cards.data`;
  SPEC-023 R2 scopes the read and R4 keeps every name out of SQL; SPEC-020 R1 and R2 give the
  study-day rule; SPEC-119 serves resources behind its guard.

## 2. Requirements

The reads (ingest)

R1. `ingest::atlas` adds two reads of the private copy to the collection reader. `keys(after_id)`
    answers, in card id order after `after_id`, at most 500 pairs of a card id and its stamp
    (`cards.mod`) for the cards in scope (SPEC-023 R2). `rows(ids)` answers, for at most 500 card
    ids bound as one JSON array, each card's id, note id, template ordinal, current deck id, stamp
    and the memory state of SPEC-077 R1 (absent when `cards.data` holds none or cannot be parsed).
    Their predicates and orderings touch integer columns only (SPEC-023 R4). A read that fails
    answers a `ReadError`, and a collection with no card answers an empty page, so the two are
    never confused.
R2. The deck names come from the reader's existing name read (by id, never compared in SQL).

The series (insights)

R3. A row of `collection_atlas` holds a card's id, note id, ordinal, deck id, family, stability,
    decay, last review (each of the last three absent without a memory state) and stamp. The family
    is the deck name's top-level segment (the text before the first `\x1f`), or `(unknown)` when the
    deck's name is empty or the deck id names no deck (`goldens/atlas_family.json`,
    `charts.py:_deck_family`).
R4. **The refresh** (`insights::atlas::refresh`) walks the copy's keys (R1) and the series' rows in
    card id order, one page of each at a time. For each copy key with no row it reads the card and
    inserts its row; for each key whose stamp differs from the row's, earlier or later, it reads the
    card and replaces its row; for each row with no copy key it deletes the row; a key whose stamp
    equals the row's is not read. It reads cards in batches of at most 500 and writes each batch in
    one `BEGIN IMMEDIATE` transaction. The walk ends at the first key page shorter than 500.
R5. **A renamed deck.** The refresh keeps a SHA-256 digest of the deck-name map. When the digest
    differs, it sets the family of every row by its deck id, one deck at a time, and stores the new
    digest.
R6. **The state.** `collection_atlas_state` holds one row: `measured`, the deck-name digest and the
    instant of the last refresh. A refresh that reaches its end stores `measured` true; a refresh
    whose read fails (R1) or whose write fails keeps every row it has, stores `measured` false, and
    the next refresh walks again from the first key. Before the first refresh, the series reads as
    unmeasured.
R7. **A page** (`insights::atlas::page(n, now)`): the rows in series order (note id, ordinal, card
    id), 2000 to a page, pages counted from 0; the page count is the row count divided by 2000,
    rounded up, and at least 1. Each page carries `as_of`, `measured`, the last refresh's instant,
    the row count and the page count. While `measured` is false, the row count reads 0 and page 0
    answers no rows. A page at or past the page count is refused.
R8. **`as_of`** is the first instant of the study day holding `now`, by the kernel's study-day rule
    (SPEC-020 R1, R2: the rollover hour and the UTC offset), never the collection's own
    configuration (SPEC-023 R7). It equals `goldens/atlas_as_of.json`
    (`charts.py:_rollover_instant`) for the same instant, hour and offset.
R9. **Parity.** Over the same synthetic collection, the rows of every page joined, and `measured`,
    equal `goldens/collection_atlas_rows.json` (`charts.py:read_collection_atlas`), each row with
    the predecessor's eight keys (`id`, `nid`, `ord`, `did`, `family`, `stability`, `decay`,
    `last_review_sec`).

The schedule and the resources

R10. `coordination::sync_cycle` runs the refresh after a sync that ran and succeeded
     (`SyncReport::Ran` with an ok outcome), as the router's flush is run (SPEC-041 R7); a refused
     or debounced sync made no request and is not followed by a refresh. A refresh error is logged
     and leaves the cycle's report and outcome as they were. `coordination::atlas::page` is the
     page read the adapters call.
R11. SPEC-119's server gains the resource `charts://collection-atlas` (the summary: `as_of`,
     `measured`, the last refresh's instant, the row count and the page count) and the resource
     template `charts://collection-atlas/{page}` (one page, R7), both `application/json`, under the
     `core` scope and SPEC-119's guard. A page that R7 refuses is a resource error. SPEC-119's A37
     admits these two beside SPEC-085 R6's names, by a dated amendment note appended to SPEC-119 in
     this delivery.
R12. No model reads or writes the series in this SPEC, so it behaves the same with the AI route
     absent (ADR-054). Nothing renders it as an image, and the public publish never reads it
     (ADR-085, #156).

Data rights and the census

R13. `collection_atlas` and `collection_atlas_state`
     (`migrations/012001_insights_collection_atlas.sql`, `STRICT`, `created_at` on both, the series
     order indexed) are the privacy category `collection-atlas`: exported whole and erased whole
     through insights' `data_rights.rs` port. An erase leaves the series unmeasured, and the next
     refresh reads every card again.
R14. SPEC-085's A14 is amended by a dated amendment note appended to SPEC-085 in this delivery: its
     census refuses a source that renders the atlas as an image and a publish source that names it,
     and admits the files this SPEC names (#282 revives what #270 left out). It keeps its planted
     fixture for each refusal.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | over 1201 cards in scope, `keys` answers pages of 500, 500 and 201 in card id order, and `rows` answers each asked card's id, note id, ordinal, deck, stamp and memory state | `the_atlas_reads_page_the_cards_in_scope` |
| A2 | a copy with no cards table answers a `ReadError`, and a collection with no card answers an empty page | `a_failed_atlas_read_errs_and_an_empty_one_answers_no_card` |
| A3 | `rows` answers no memory state for a card with no `data` and for one whose `data` cannot be parsed | `the_atlas_rows_answer_no_state_without_one` |
| A4 | over a copy whose deck names carry the engine's collation, both reads succeed with no collation registered | `the_atlas_reads_bind_no_name_column` |
| A5 | a card whose original deck is outside the scope is in neither read | `the_atlas_reads_keep_to_the_scope` |
| A6 | the family equals its golden: a top-level name, a nested one, and the empty name | `the_family_matches_the_golden` |
| A7 | a refresh inserts a new card's row, replaces the row of a card whose stamp rose and of one whose stamp fell, deletes the row of a card gone from the copy, and reads no card whose stamp is unchanged | `a_refresh_reads_exactly_the_changed_cards` |
| A8 | a renamed deck sets the family of its cards' rows, and no other row changes | `a_renamed_deck_rewrites_its_cards_family` |
| A9 | a refresh whose read fails keeps every row and stores `measured` false, and the next refresh restores every row and `measured` true | `a_failed_refresh_keeps_the_rows_and_is_unmeasured` |
| A10 | `as_of` equals its golden: 03:59:59 and 04:00:00 local and noon, at offsets of 0, +540 and -300 minutes | `as_of_matches_the_golden` |
| A11 | the rows of every page joined, and `measured`, equal the golden in each of its three cases | `the_pages_match_the_golden` |
| A12 | before the first refresh and after a failed one, the row count reads 0 and page 0 answers no rows | `an_unmeasured_series_answers_no_rows` |
| A13 | 0 rows answer one empty page; 2000 rows answer one page; 2001 rows answer pages of 2000 and 1; page 2 of 2001 is refused | `the_pages_hold_2000_rows` |
| A14 | a first refresh over 1201 cards asks for exactly three key pages and reads cards in batches of at most 500; the scripted reader fails the test on a fourth key read | `a_refresh_walks_the_copy_in_pages` |
| A15 | the refresh follows a sync that ran and succeeded, and follows none that ran and failed, was refused today or was debounced | `the_atlas_refreshes_after_a_successful_sync_only` |
| A16 | a refresh that errs leaves the cycle's report and outcome as they were | `a_failed_atlas_refresh_never_fails_the_cycle` |
| A17 | the summary and each page answer as R11 says, `application/json`, and a refused page is a resource error | `the_atlas_resources_answer_the_summary_and_pages` |
| A18 | both atlas resources answer 401 without a granted bearer | `the_atlas_resources_need_a_granted_bearer` |
| A19 | every `collection_atlas` and `collection_atlas_state` row is exported, and an erase leaves none | `collection_atlas_export_and_erase_are_symmetric` |
| A20 | no source renders the atlas as an image and no publish source names it; a planted fixture of each is refused | `test_the_collection_atlas_is_never_an_image_and_never_published` |

```acceptance
A1: cargo test -p deck-streak-ingest --test atlas -- --exact the_atlas_reads_page_the_cards_in_scope
A2: cargo test -p deck-streak-ingest --test atlas -- --exact a_failed_atlas_read_errs_and_an_empty_one_answers_no_card
A3: cargo test -p deck-streak-ingest --test atlas -- --exact the_atlas_rows_answer_no_state_without_one
A4: cargo test -p deck-streak-ingest --test atlas -- --exact the_atlas_reads_bind_no_name_column
A5: cargo test -p deck-streak-ingest --test atlas -- --exact the_atlas_reads_keep_to_the_scope
A6: cargo test -p deck-streak-insights --test atlas -- --exact the_family_matches_the_golden
A7: cargo test -p deck-streak-insights --test atlas -- --exact a_refresh_reads_exactly_the_changed_cards
A8: cargo test -p deck-streak-insights --test atlas -- --exact a_renamed_deck_rewrites_its_cards_family
A9: cargo test -p deck-streak-insights --test atlas -- --exact a_failed_refresh_keeps_the_rows_and_is_unmeasured
A10: cargo test -p deck-streak-insights --test atlas -- --exact as_of_matches_the_golden
A11: cargo test -p deck-streak-insights --test atlas -- --exact the_pages_match_the_golden
A12: cargo test -p deck-streak-insights --test atlas -- --exact an_unmeasured_series_answers_no_rows
A13: cargo test -p deck-streak-insights --test atlas -- --exact the_pages_hold_2000_rows
A14: cargo test -p deck-streak-insights --test atlas -- --exact a_refresh_walks_the_copy_in_pages
A15: cargo test -p deck-streak-coordination --test atlas_refresh -- --exact the_atlas_refreshes_after_a_successful_sync_only
A16: cargo test -p deck-streak-coordination --test atlas_refresh -- --exact a_failed_atlas_refresh_never_fails_the_cycle
A17: cargo test -p deck-streak-mcp --test atlas_resource -- --exact the_atlas_resources_answer_the_summary_and_pages
A18: cargo test -p deck-streak-mcp --test atlas_resource -- --exact the_atlas_resources_need_a_granted_bearer
A19: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact collection_atlas_export_and_erase_are_symmetric
A20: python3 -m unittest discover -s scripts/tests -p test_charts_no_host_rendering.py -k test_the_collection_atlas_is_never_an_image_and_never_published
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr pack is already enforced, and this
delivery changes no pack's state, so the private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/insights/src/data_rights.rs`: the `collection-atlas` category names `collection_atlas` and `collection_atlas_state` with purpose, basis and retention, and export and erase cover both | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/atlas.rs` | `deck-streak-ingest` | added: `keys` and `rows` on the collection reader |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/atlas.rs` | `deck-streak-ingest` | added: A1 to A5 |
| `crates/insights/src/atlas.rs` | `deck-streak-insights` | added: the family, the refresh, the state, `as_of` and the page |
| `crates/insights/src/lib.rs`, `crates/insights/Cargo.toml` | `deck-streak-insights` | changed: the modules; `sha2` (already in the workspace) for the digest |
| `crates/insights/tests/atlas.rs` | `deck-streak-insights` | added: A6 to A14, against a scripted reader |
| `migrations/012001_insights_collection_atlas.sql` | `deck-streak-insights` | added: both tables and the series-order index |
| `crates/insights/src/data_rights.rs` | `deck-streak-insights` | added: both tables exported and erased |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: both tables registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows, A19 |
| `docs/CONTEXT-MAP.md` | docs | changed: both tables in insights' own tables |
| `privacy.json`, `PRIVACY.md` | repo | changed: the category `collection-atlas` |
| `crates/coordination/src/atlas.rs` | `deck-streak-coordination` | added: the refresh step and the page read |
| `crates/coordination/src/sync_cycle.rs`, `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the refresh after a successful sync; the module |
| `crates/coordination/tests/atlas_refresh.rs` | `deck-streak-coordination` | added: A15, A16 |
| `crates/mcp/src/resources.rs` | `deck-streak-mcp` | changed: the summary and the page template |
| `crates/mcp/tests/atlas_resource.rs` | `deck-streak-mcp` | added: A17, A18 |
| `crates/mcp/tests/resources.rs` | `deck-streak-mcp` | changed: A37's list admits the two atlas resources (R11) |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the atlas store for the sync cycle and the `mcp` role |
| `scripts/tests/test_charts_no_host_rendering.py` | repo | changed: A20 replaces SPEC-085's A14 test (R14) |
| `docs/specs/SPEC-085-charts-are-drawn-on-the-client-from-json-series.md` | docs | changed: a dated amendment note appended (R14) |
| `docs/specs/SPEC-119-the-mcp-server-serves-the-predecessors-tools-on-loopback-and-refuses-every-request-without-a-granted-bearer.md` | docs | changed: a dated amendment note appended (R11) |
| `tools/parity-oracle/registry/spec_120.py` | repo | added: the goldens of §7 |
| `tools/parity-oracle/goldens/collection_atlas_rows.json`, `atlas_family.json`, `atlas_as_of.json` | repo | added: generated by §7 |
| `scripts/mutation-rows.d/S12000-S12099.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-120-the-collection-atlas-is-a-series-refreshed-from-changed-cards-after-each-sync-and-the-agent-reads-it-in-pages.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-120.md` | docs | added |
| `docs/schematics/collection-atlas-refresh.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It draws no atlas, on the host or in the Mini App, and adds no bot command; the issue keeps it an
  input to the agent (#282).
- It puts nothing of the atlas in the public export; the publish job keeps its own charts (#156).
- It gives no W6 duty the atlas as an input; a duty that wants it names it in its own SPEC (#29).
- It reads no card outside the study scope. With the scope unset every deck is read, as the
  predecessor read them; whether a scoped owner's atlas should reach past the scope is the owner's
  question (#282).
- It revives no other chart the predecessor served as an image (#74, #133).

## 6. Risks

- **A change that the refresh misses.** R4 compares stamps in both directions; A7 names a card whose
  stamp fell.
- **A stale family after a rename.** R5 compares the deck-name digest; detected by A8.
- **An empty collection mistaken for a failed read.** R1 and R6 keep them apart; detected by A2, A9
  and A12.
- **A walk that reads the whole copy at once, or never ends.** R4 reads in pages and ends at the
  first short page; A14 counts the pages and fails on a fourth.
- **Float parity.** Stability and decay are compared with the golden bit for bit, so the reader
  parses them with the house float rule (SPEC-077 R1).

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_120.py` and generated on the owner's checkout of
the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `collection_atlas_rows` | `charts.py:read_collection_atlas` | adapter | three synthetic collection files: one with nested decks, a filtered deck, notes created out of id order, several templates per note, a card with no memory state, a card with an unparsable one, and a card whose deck id names no deck; one with no cards table (no rows, unmeasured); and one with no card (no rows, measured). Each answers its rows and `measured` |
| `atlas_family` | `charts.py:_deck_family` | function | synthetic names: a top-level deck, a nested one, and the empty name |
| `atlas_as_of` | `charts.py:_rollover_instant` | function | instants at 03:59:59, 04:00:00 and 12:00:00 local, at offsets of 0, +540 and -300 minutes, rollover hour 4 |

The image (`charts.py:collection_atlas`, `render_collection_atlas`) is left out on purpose (R12).

## 8. Tables and the v9 import

`collection_atlas` and `collection_atlas_state` (insights,
`migrations/012001_insights_collection_atlas.sql`). The predecessor kept no table for its atlas and
read the collection per request, so W8's import maps nothing into them; the first refresh after the
cutover fills them.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S12001-STAMP-DIFFERS` | `crates/insights/src/atlas.rs` | a row is re-read when its stamp differs; the test names a stamp that fell | `atlas::a_refresh_reads_exactly_the_changed_cards` |
| `S12002-SKIP-UNCHANGED` | `crates/insights/src/atlas.rs` | an unchanged stamp is not read; the scripted reader counts reads | `atlas::a_refresh_reads_exactly_the_changed_cards` |
| `S12003-DROP-GONE` | `crates/insights/src/atlas.rs` | a row with no copy key is deleted | `atlas::a_refresh_reads_exactly_the_changed_cards` |
| `S12004-KEY-PAGE` | `crates/ingest/src/atlas.rs` | 500 keys to a page; the test names pages of 500, 500 and 201 | `atlas::the_atlas_reads_page_the_cards_in_scope` |
| `S12005-READ-ERR` | `crates/ingest/src/atlas.rs` | a failed read is an error, never an empty page | `atlas::a_failed_atlas_read_errs_and_an_empty_one_answers_no_card` |
| `S12006-SCOPE` | `crates/ingest/src/atlas.rs` | only cards whose original deck is in scope | `atlas::the_atlas_reads_keep_to_the_scope` |
| `S12007-FAMILY-UNKNOWN` | `crates/insights/src/atlas.rs` | the empty name is `(unknown)` | `atlas::the_family_matches_the_golden` |
| `S12008-RENAME` | `crates/insights/src/atlas.rs` | a changed digest rewrites the family | `atlas::a_renamed_deck_rewrites_its_cards_family` |
| `S12009-UNMEASURED` | `crates/insights/src/atlas.rs` | a failed refresh stores `measured` false | `atlas::a_failed_refresh_keeps_the_rows_and_is_unmeasured` |
| `S12010-ROLLOVER` | `crates/insights/src/atlas.rs` | `as_of` is the study day's first instant; the golden names 03:59:59 and 04:00:00 | `atlas::as_of_matches_the_golden` |
| `S12011-SERIES-ORDER` | `crates/insights/src/atlas.rs` | note id, ordinal, card id; the golden's notes are created out of id order | `atlas::the_pages_match_the_golden` |
| `S12012-PAGE-SIZE` | `crates/insights/src/atlas.rs` | 2000 to a page; the test names 2000 and 2001 rows | `atlas::the_pages_hold_2000_rows` |
| `S12013-UNMEASURED-EMPTY` | `crates/insights/src/atlas.rs` | an unmeasured series answers no rows | `atlas::an_unmeasured_series_answers_no_rows` |
| `S12014-WALK-END` | `crates/insights/src/atlas.rs` | the walk ends at the first short page; the scripted reader fails on a fourth key read, so the mutant is bounded | `atlas::a_refresh_walks_the_copy_in_pages` |
| `S12015-AFTER-SUCCESS` | `crates/coordination/src/sync_cycle.rs` | the refresh follows a successful sync only | `atlas_refresh::the_atlas_refreshes_after_a_successful_sync_only` |
| `S12016-DEGRADE` | `crates/coordination/src/sync_cycle.rs` | a refresh error does not reach the cycle's outcome | `atlas_refresh::a_failed_atlas_refresh_never_fails_the_cycle` |

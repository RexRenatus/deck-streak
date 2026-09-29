# SPEC-099: the Can-Do ladder unlocks a rung in the owner's own words after each sync

- **Wave:** W4. **Issues:** #92 (epic #5). **Context(s):** `deck-streak-ingest` (the Can-Do read);
  `deck-streak-curriculum` (the rungs, maturity, `can_do_unlocks` and `can_do_ladder`);
  `deck-streak-coordination` (the unlock pass after each sync's recompute, the ladder's read model);
  `deck-streak-api` and `deck-streak-bot` (the ladder route, `/cando`); the Mini App (the ladder
  screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-095 (the reads keep SPEC-023's
  scope), ADR-096 (the owner's note conventions, the Can-Do field among them, load once in the
  kernel) and ADR-099 (the unlock pass runs after each sync's recompute, off its write path, and
  records each unlock once).
- **Prerequisites:** SPEC-023 (the read and its scope), SPEC-029, SPEC-071 (each card's course),
  SPEC-077 (curriculum's data-rights port) and SPEC-094 (the conventions file and the `unicase` rule). **Mutation band:** `S09900-S09999`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-099.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 curriculum holds no ladder, and ingest reads no note's
  fields. SPEC-094 (planned) loads the conventions file, which names the Can-Do field.
- **What is ported.** The Can-Do ladder (`cando.py:_read_can_do_notes`, `_is_mature`,
  `cards_until_unlock`, `unlock_state`, `unlock_pass` and `render_cando`), with its command
  (`bot.py:CommandBot._handle_cando`) and its store (`database.py:GamifyStore.record_can_do_unlock`
  and `get_recent_can_do_unlocks`).
- **The rule.** Each distinct, non-empty Can-Do statement is a rung of one course. A note carries
  a statement when its note type has the Can-Do field. A card is mature at an interval of 21 days
  or more and at most 1 lapse. A rung unlocks when ANY note carrying it has every one of its cards
  mature, and its cards remaining are the fewest immature cards of any of its notes.
- **Traps a hand port falls into.**
  - The note's language is the course of its cards' home decks; a note whose cards span courses
    takes the first code in sorted order. A card whose home deck has no course still counts as an
    immature card of its note, and a note with no card in a course carries no rung.
  - The statement is trimmed as Python's `str.strip` trims, which counts the separators U+001C to
    U+001E as space where Rust's `trim` does not.
  - The Can-Do field is found by its name through the predecessor's collation, which lowercases
    ASCII and drops every other character before comparing (`cando.py:_unicase_collate`). Anki
    collates that column `unicase`; as SPEC-094 R4 has it, the read scans the fields with no
    collation and compares in Rust.
  - An unlock is recorded once, keyed by statement and course, and keeps the instant it was first
    recorded; a rung that later falls back is still listed as unlocked, and is also a locked rung
    while it is locked.
  - The recent unlocks sort by that instant, newest first, then by statement; the locked rungs by
    cards remaining, then course, then statement. Each list shows 5 in the bot.
  - A failed read is a failure, never an empty ladder.
- **Corrections to the issue.**
  - #92 scans on demand, so an unlock's date is the day the owner next asked. Here the pass runs
    after each sync's recompute (ADR-099), as the issue's port note suggests, so an unlock is dated
    by the sync that first saw it, and the ladder never waits on a scan.
  - The predecessor read every note of the collection. Here the read keeps SPEC-023's scope
    (ADR-095): a card outside the scope is never read, so it cannot hold a rung locked.
- **What the parity oracle proves.** The field-name comparison; maturity and the cards remaining;
  the rungs of a synthetic collection; and the constants.
- **Prerequisites.** SPEC-023, SPEC-029, SPEC-071, SPEC-077 and SPEC-094, as the header names them.

## 2. Requirements

The read (ADR-095)

R1. `crates/ingest/src/can_do_reads.rs` reads, from the private copy and read-only, keeping
    SPEC-023's scope: every field row's note type, ordinal and name, compared with the conventions'
    Can-Do field as `goldens/can_do_fold.json` compares them (`cando.py:_unicase_collate`); then,
    for each note of a matching note type, its id, its statement at that ordinal and each of its
    cards' interval, lapses and home deck. The read registers no collation and orders, groups or
    seeks on no name column (SPEC-094 R4); it returns its rows and the name of any read that
    failed, and no note's other fields leave ingest.
R2. With the conventions unset or naming no Can-Do field, the pass reads nothing and the ladder says
    that it is not configured.

The ladder (#92)

R3. `crates/curriculum/src/can_do.rs` builds the rungs from those rows and each home deck's course
    (SPEC-071 R2), and maturity, the cards remaining and the unlock state equal
    `goldens/can_do_maturity.json` (`cando.py:_is_mature`, `cards_until_unlock`, `unlock_state`).
R4. The rungs of a synthetic collection, read through R1 and built by R3, equal
    `goldens/can_do_notes.json` (`cando.py:_read_can_do_notes` over the same collection, with the
    predecessor's table of language decks patched to the synthetic courses).
R5. The constants (21 days, 1 lapse and 5 rows) equal `goldens/can_do.constants.json`.

The pass (ADR-099)

R6. After each sync's recompute commits, and before the instruments step (SPEC-094 R7),
    coordination runs the unlock pass through the kernel's offload: the read of R1, the rungs of R3,
    then one write that records each unlocked rung in `can_do_unlocks` with the pass's instant and
    study day, unless it is already recorded, and replaces `can_do_ladder` with the pass's outcome.
    A rung already recorded keeps its first instant.
R7. `can_do_ladder` holds the latest pass: its instant, whether its read failed and which read, the
    count of unlocked rungs, and every locked rung with its course and cards remaining. A failed
    read replaces it with the failure and records no unlock; the unlocks already recorded stay.
R8. `can_do_unlocks` and `can_do_ladder` are `STRICT` tables with `created_at`, created by
    `migrations/009901_curriculum_can_do.sql` (SPEC-020 R15, R18). Both are registered in the
    context map's register of DeckStreak's own tables, declared in `privacy.json` (the category
    `can-do-ladder`), given a line in `PRIVACY.md`, and exported and erased by curriculum's
    data-rights port; the symmetry test seeds both.

The surfaces

R9. `GET /api/curriculum/can-do` serves every recorded unlock, newest first, with its statement,
    course and study day, and the latest pass's locked rungs, nearest first, with the pass's
    instant, or its failure. It answers the owner's session only; any other caller is answered 401
    or 403 with no data. Before the first pass it reads as pending.
R10. `/cando` answers with the 5 most recent unlocks and the 5 nearest locked rungs with their cards
     remaining, each statement escaped for Telegram's HTML, or the latest pass's failure; it runs no
     scan.
R11. The Mini App's ladder screen lists every unlock with its date and the locked rungs, nearest
     first, with their cards remaining, and renders a failed pass as a failure, never as an empty
     ladder.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the Can-Do field is found by the predecessor's comparison for every golden case | `the_can_do_field_matches_by_the_predecessors_fold` |
| A2 | a card outside the scope is never read | `a_card_outside_the_scope_is_not_read` |
| A3 | maturity, the cards remaining and the unlock state equal the golden | `maturity_matches_the_predecessors_golden` |
| A4 | a rung unlocks when any note carrying it is fully mature, and counts the fewest cards remaining | `a_rung_unlocks_when_any_of_its_notes_is_mature` |
| A5 | the ladder's constants equal the golden | `the_can_do_constants_equal_the_predecessors` |
| A6 | an unlock is recorded once and keeps its first instant | `an_unlock_is_recorded_once_with_its_first_instant` |
| A7 | both tables are exported and erased | `the_can_do_tables_are_exported_and_erased` |
| A8 | the rungs of a synthetic collection equal the predecessor's reader | `the_ladder_of_a_synthetic_collection_matches_the_predecessors_reader` |
| A9 | the pass runs after the recompute commits, never inside it | `the_pass_runs_after_the_recompute_commits` |
| A10 | a failed read is stored as a failure, records no unlock, and keeps the recorded ones | `a_failed_read_is_stored_as_a_failure` |
| A11 | with no Can-Do field configured, the pass reads nothing and says so | `the_pass_without_a_can_do_field_reads_nothing` |
| A12 | the ladder route answers the owner and refuses every other caller with no data | `the_can_do_route_answers_only_the_owner` |
| A13 | `/cando` shows 5 unlocks and 5 locked rungs, escaped | `cando_shows_five_of_each_escaped` |
| A14 | `/cando` renders a failed pass as a failure | `cando_renders_a_failed_pass_as_a_failure` |
| A15 | the ladder lists every unlock with its date and the locked rungs nearest first | `lists every unlock with its date and the locked rungs nearest first` |
| A16 | the ladder renders a failed pass as a failure, never an empty ladder | `renders a failed pass as a failure` |

```acceptance
A1: cargo test -p deck-streak-ingest --test can_do_reads -- --exact the_can_do_field_matches_by_the_predecessors_fold
A2: cargo test -p deck-streak-ingest --test can_do_reads -- --exact a_card_outside_the_scope_is_not_read
A3: cargo test -p deck-streak-curriculum --test can_do -- --exact maturity_matches_the_predecessors_golden
A4: cargo test -p deck-streak-curriculum --test can_do -- --exact a_rung_unlocks_when_any_of_its_notes_is_mature
A5: cargo test -p deck-streak-curriculum --test can_do -- --exact the_can_do_constants_equal_the_predecessors
A6: cargo test -p deck-streak-curriculum --test can_do_store -- --exact an_unlock_is_recorded_once_with_its_first_instant
A7: cargo test -p deck-streak-curriculum --test can_do_store -- --exact the_can_do_tables_are_exported_and_erased
A8: cargo test -p deck-streak-coordination --test can_do_step -- --exact the_ladder_of_a_synthetic_collection_matches_the_predecessors_reader
A9: cargo test -p deck-streak-coordination --test can_do_step -- --exact the_pass_runs_after_the_recompute_commits
A10: cargo test -p deck-streak-coordination --test can_do_step -- --exact a_failed_read_is_stored_as_a_failure
A11: cargo test -p deck-streak-coordination --test can_do_step -- --exact the_pass_without_a_can_do_field_reads_nothing
A12: cargo test -p deck-streak-api --test can_do_routes -- --exact the_can_do_route_answers_only_the_owner
A13: cargo test -p deck-streak-bot --test can_do_commands -- --exact cando_shows_five_of_each_escaped
A14: cargo test -p deck-streak-bot --test can_do_commands -- --exact cando_renders_a_failed_pass_as_a_failure
A15: pnpm exec vitest run web/app/src/lib/cando/CanDoLadder.test.ts -t "lists every unlock with its date and the locked rungs nearest first"
A16: pnpm exec vitest run web/app/src/lib/cando/CanDoLadder.test.ts -t "renders a failed pass as a failure"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and accessibility packs stay
enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md`, `migrations/009901_curriculum_can_do.sql` and `crates/curriculum/src/data_rights.rs`: the `can-do-ladder` category names `can_do_unlocks` and `can_do_ladder`, the owner's statements they hold, their purpose, basis and retention, and export and erase cover both | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/can-do/+page.svelte` and every file under `web/app/src/lib/cando/`: the ladder and its failure pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/can_do_reads.rs` | `deck-streak-ingest` | added: the field rows and the Can-Do notes' statements and cards |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/can_do_reads.rs` | `deck-streak-ingest` | added: A1, A2 |
| `crates/curriculum/src/can_do.rs` | `deck-streak-curriculum` | added: the rungs, maturity and the two orders |
| `crates/curriculum/src/can_do_store.rs` | `deck-streak-curriculum` | added: the repository over the two tables |
| `crates/curriculum/src/data_rights.rs` | `deck-streak-curriculum` | changed: the port exports and erases both tables |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the modules |
| `crates/curriculum/tests/can_do.rs` | `deck-streak-curriculum` | added: A3 to A5 |
| `crates/curriculum/tests/can_do_store.rs` | `deck-streak-curriculum` | added: A6, A7 |
| `migrations/009901_curriculum_can_do.sql` | `deck-streak-curriculum` | added: `can_do_unlocks` and `can_do_ladder` |
| `crates/coordination/src/can_do.rs` | `deck-streak-coordination` | added: the unlock pass and the ladder's read model |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the pass after the recompute, before the instruments step |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/can_do_step.rs` | `deck-streak-coordination` | added: A8 to A11 |
| `crates/api/src/can_do_routes.rs` | `deck-streak-api` | added: the ladder route |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the route behind the owner's session |
| `crates/api/tests/can_do_routes.rs` | `deck-streak-api` | added: A12 |
| `crates/bot/src/can_do_commands.rs` | `deck-streak-bot` | added: the cando command |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command joins the table |
| `crates/bot/tests/can_do_commands.rs` | `deck-streak-bot` | added: A13, A14 |
| `web/app/src/routes/can-do/+page.svelte` | miniapp | added: the ladder screen |
| `web/app/src/lib/cando/CanDoLadder.svelte` | miniapp | added |
| `web/app/src/lib/cando/CanDoLadder.test.ts` | miniapp | added: A15, A16 |
| `web/app/src/lib/cando/cando.ts` | miniapp | added: the ladder's type and fetch |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains both tables |
| `privacy.json` | repo | changed: the `can-do-ladder` category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry lists both tables under curriculum's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for each table |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `tools/parity-oracle/registry/spec_099.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/can_do_fold.json` | repo | added: the golden of `cando.py:_unicase_collate` (function) |
| `tools/parity-oracle/goldens/can_do_maturity.json` | repo | added: the golden of `cando.py:_is_mature`, `cards_until_unlock` and `unlock_state` (function) |
| `tools/parity-oracle/goldens/can_do_notes.json` | repo | added: the golden of `cando.py:_read_can_do_notes` (adapter; a temporary synthetic collection and synthetic courses) |
| `tools/parity-oracle/goldens/can_do.constants.json` | repo | added: the ladder's constants (constants) |
| `scripts/mutation-rows.d/S09900-S09999.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-099-the-can-do-ladder-unlocks-a-rung-in-the-owners-own-words-after-each-sync.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/curriculum-readouts-and-the-can-do-pass.md` | docs | added by the W4 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-099.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It builds no settings screen for the conventions file (#57).
- It serves the ladder to no agent's machine read tool (#157).
- It carries the predecessor's unlocks only through the v9 import (#61).

## 6. Risks

- **A collated read fails on the live collection.** Prevented by R1's full scan and detected by A1,
  whose golden holds names that differ only in case and in characters outside ASCII.
- **One note's maturity is read as the rung's.** Detected by A4, whose rung has a mature note and an
  immature one.
- **A re-run dates an old unlock anew.** Detected by A6.
- **A failed read clears the ladder.** Detected by A10, A14 and A16.
- **A statement carrying markup breaks the bot's message.** Detected by A13, whose statements hold
  `<`, `>` and `&`.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_099.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic: the adapter patches the predecessor's
table of language decks to synthetic courses, so no deck name of the owner's enters a golden.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `can_do_fold` | `cando.py:_unicase_collate` | function | name pairs differing in case, in characters outside ASCII and in punctuation |
| `can_do_maturity` | `cando.py:_is_mature`, `cards_until_unlock`, `unlock_state` | function | intervals and lapses at and around 21 and 1, and an empty card list |
| `can_do_notes` | `cando.py:_read_can_do_notes` | adapter | a temporary collection file with two note types carrying the field, notes spanning courses, a card in a deck of no course, blank and padded statements |
| `can_do.constants` | the module's constants | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `can_do_unlocks` | `deck-streak-curriculum` | `migrations/009901_curriculum_can_do.sql` | `can_do_unlocks`, row for row, by the v9 import (#61): the language becomes the course code, and the unlock time an instant and its study day | exported and erased |
| `can_do_ladder` | `deck-streak-curriculum` | `migrations/009901_curriculum_can_do.sql` | nothing: the first pass writes it | exported and erased |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09901-MATURE-INTERVAL` | `crates/curriculum/src/can_do.rs` | an interval of 21 days is mature | `can_do::maturity_matches_the_predecessors_golden` |
| `S09902-MATURE-LAPSES` | `crates/curriculum/src/can_do.rs` | one lapse is still mature | `can_do::maturity_matches_the_predecessors_golden` |
| `S09903-ANY-NOTE-UNLOCKS` | `crates/curriculum/src/can_do.rs` | the fewest cards remaining of any note decides | `can_do::a_rung_unlocks_when_any_of_its_notes_is_mature` |
| `S09904-RECORD-ONCE` | `crates/curriculum/src/can_do_store.rs` | a recorded unlock keeps its first instant | `can_do_store::an_unlock_is_recorded_once_with_its_first_instant` |
| `S09905-FAILURE-NOT-EMPTY` | `crates/coordination/src/can_do.rs` | a failed read is stored as a failure | `can_do_step::a_failed_read_is_stored_as_a_failure` |
| `S09906-FOLD-DROPS-NON-ASCII` | `crates/ingest/src/can_do_reads.rs` | the comparison drops characters outside ASCII | `can_do_reads::the_can_do_field_matches_by_the_predecessors_fold` |

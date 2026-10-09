# SPEC-387: DeckStreak proposes the scheduler's default parameters for a preset, and the owner applies them in Anki

- **Issue:** `#757` (filed from section 2; the build writes its number here and nowhere else in
  this file). The collection's main preset is to move to the current scheduler generation's
  default parameters, as a preset change and not a refit, and DeckStreak reads no preset today.
  This SPEC adds a read of every preset, a proposal the owner applies in their own Anki app, a
  record that keeps the parameters it replaces, and a check that settles the proposal once the
  change has synced.
- **Context(s):** `ingest` (`crates/ingest/src/preset.rs`, `crates/ingest/src/engine.rs`,
  `crates/ingest/src/data_rights.rs`, one migration), `daemon` (`crates/daemon/src/main.rs`,
  `crates/daemon/src/role_preset.rs`), `docs` (`PRIVACY.md`, `privacy.json`) and the records
  section 4 names.
- **Decided by:** ADR-401 (this SPEC's own). It amends SPEC-368 and SPEC-342 by one insert-only
  section each, appended at the end of each file (section 10); no existing line of either changes.
  It declares one write class on ADR-301's advisory rung, under ADR-301's rule, and works under
  the owner's no-dwell ruling (#744).
- **Schematic:** `docs/schematics/scheduler-presets-and-their-parameters.md` (new): the preset from
  its store through the engine and sync to each client, before and after the move.
- **Assumption:** the engine stores a preset's parameters in three fields, one per scheduler
  generation, and schedules with the newest that holds values (R2). The build re-measures this at
  its cut in the engine's own source; a different precedence stops the build.
- **Status:** one delivery. **Mutation band:** S38700-S38799 (section 8). **Model:** none, by
  surface (section 7).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `3ca06142` by `git show`, `git grep -n` and
`git ls-tree`. Nothing was run: every figure is a read.

### 1a. The current scheduler generation, and where its defaults live

- The engine is the Anki crate at a released tag (`Cargo.toml:91`, with the fork patch at
  `:171-173`). Its scheduler is the released FSRS-6 package, which the workspace also names as
  `fsrs6` (`Cargo.toml:145`; the lockfile's registry entry, `Cargo.lock:2022-2025`).
- FSRS-6 holds 21 parameters. Its default vector is the package's `DEFAULT_PARAMETERS`:
  `crates/fsrs7/tests/coexistence.rs:9` builds a scheduler from it, and `:48-52` asserts that the
  released package's defaults are 21 values.
- FSRS-7 lives only in the isolated `crates/fsrs7`, pinned to an unreleased revision
  (`Cargo.toml:141`; SPEC-342; ADR-338). ADR-338 keeps the normal scheduler on stock FSRS-6 on the
  released package (`:23`), and records that the upstream branch also changes FSRS-6's defaults
  (`:14-15`). So the current generation is FSRS-6, and its defaults are the released package's.
- Only the engine and the FSRS-7 crate's own test use the released package:
  `scripts/tests/test_fsrs7_pin.py:33` holds `RELEASED_USERS = ["anki", CRATE]`, under SPEC-342 R2
  (`:66-73`). `crates/ingest/Cargo.toml` names no scheduler package.

### 1b. Where a preset lives, and what DeckStreak does with it today

- A preset is a deck config in the collection, and each deck names its preset's id:
  `crates/engine-core/src/face.rs:126-138` reads a deck's config id, and `:36` holds
  `DEFAULT_PRESET = 1`. A preset's parameters and its desired retention sit side by side in it.
- DeckStreak writes no preset. The engine core's ordinary table has 18 rows and no deck-config
  write (`crates/engine-core/src/table.rs:122-249`). The engine core refuses `UpdateDeckConfigs` as
  unbounded, because one call saves presets, removes presets, toggles the scheduler for the whole
  collection and reschedules (ADR-356:120-123; SPEC-345:78, :213).
- Ingest's one write port is `CollectionWrite` (`crates/ingest/src/engine.rs:254-299`), which only
  `RslibEngine` implements (`:308`) and only the skip day's write module names (census A24,
  `crates/ingest/tests/skip_census.rs`; ADR-321 D14).
- DeckStreak reads no preset. Ingest reads each card's stored desired retention
  (`crates/ingest/src/memory_state.rs:20-21`, `:117`), but no preset's parameters, name or decks.

### 1c. What a parameter change does to a collection

- When new parameters are saved, the engine recomputes and rewrites the memory state of every
  non-new card in the preset's decks from that card's own review history. It does this even with
  "Reschedule cards on change" off.
  - With that option off, no due date, interval or review-log row changes.
  - With it on, every moved card gains a review-log row and a new due date. That is a
    mass-reschedule (ADR-301:129-135), and it is on ADR-301's never-list.
  - The build re-measures both facts in the engine's source at its cut.
- ADR-301 counts a preset's options and parameters as a write (`:59-61`).
- Under ADR-301, every class starts on the advisory rung, where it proposes its batch to the owner
  and writes nothing (`:172-176`).
- The owner's no-dwell ruling (#744) gives a dwell of none to a class whose only changes are a
  preset's parameters, its desired retention or the order of due cards, and which moves no card's
  due date.

### 1d. What the privacy page says

- `PRIVACY.md:120-129` is `## Models and your data`. Its paragraph is pinned by
  `scripts/tests/test_privacy_policy.py:180-191` (`PARAGRAPH`), under SPEC-368 R2. SPEC-368 R3
  says any change to that behaviour owes an amendment of the section in its own delivery.
- The paragraph says DeckStreak does not "fit a model" and does not fit the parameters to your
  reviews. It does not say which kind of model is never trained. It has no place for a record
  that DeckStreak keeps about a preset.
- `privacy.json` declares each ingest record by category, with its export and erase
  (`privacy.json:16-37`, `skip-days` and `ingest-state`).

### 1e. The daemon's host roles

`crates/daemon/src/main.rs:56` holds `NAMES = ["api", "bot", "job", "data", "mcp"]`, and `:59-71`
parse them. The `data` role is the owner's own act on the host (`crates/daemon/src/role_data.rs:1-26`).

## 2. Requirements

R1. **The preset read.** Ingest gains a read port, `PresetRead`, which only `RslibEngine`
    implements. It reads every preset from the private copy, under the collection's shared lock,
    and returns for each:
    - its id and name;
    - its stored parameter vector, and the field that vector came from;
    - its desired retention;
    - the ids of the decks that name it;
    - the count of its non-new cards.

    It names no engine write method.

R2. **Which field holds the vector.** The stored vector comes from the first field that holds
    values: the FSRS-6 field, then the FSRS-5 field, then the FSRS-4 field. If none holds values,
    the vector is empty. This is the precedence the engine applies when it schedules.

R3. **The defaults.** The proposed vector is the released FSRS-6 package's `DEFAULT_PARAMETERS`:
    all 21 values, as the engine reports them through the deck-options read R11 names. It is never
    a copied literal, and never anything from the FSRS-7 crate. A proposal changes no desired
    retention, no other option and no deck.

R4. **The proposal.** `deckstreakd preset propose <preset id>` records one proposal for that
    preset, with state `open` and `created_at`. The proposal holds:
    - the preset's id and name;
    - its stored vector and that vector's field, which are the undo values;
    - the proposed vector;
    - its desired retention;
    - the count of non-new cards whose memory state the change recomputes.

    The command prints the proposal (R6). Two cases record nothing:
    - a preset whose stored vector already equals the defaults is reported as on the defaults;
    - a preset with an open proposal has that proposal printed again.

    At most one proposal per preset is open, and a partial unique index holds that.

R5. **The listing.** `deckstreakd preset list` prints every preset, the one with the most non-new
    cards first, so the first line is the main preset. Each line gives:
    - its id, name and field;
    - whether its vector equals the defaults;
    - its desired retention;
    - its count of decks and its count of non-new cards.

R6. **The proposal text.** It names the preset, then states:
    - **The values.** All 21, on one line and comma-separated. Each is printed so that it parses
      back to the same value.
    - **The steps.** Open the preset's options in Anki, paste the values into its FSRS parameters,
      leave "Reschedule cards on change" off, leave desired retention as it is, save, then sync.
    - **What changes.** The memory state of N non-new cards is recomputed from their own reviews.
    - **What does not change.** No review, due date or interval.
    - **The undo.** The prior values on one line, or "clear the parameters box" when the field was
      empty.
    - **A replaced fit.** When the stored vector is an FSRS-6 vector unequal to the defaults, one
      line says the defaults replace a fit of the current generation.

R7. **Verify.** `deckstreakd preset verify <proposal id>` re-reads the preset from the copy as of
    DeckStreak's last sync, and settles an open proposal once:
    - **`moved`** when the stored vector equals the proposed one. It records when this was
      observed, and whether desired retention is unchanged.
    - **`diverged`** when the vector equals neither the prior one nor the proposed one.
    - **Still open** while the vector still equals the prior one. The command says so, and records
      nothing.

    A settled proposal is never settled again. The observed time is the preset's change point.

R8. **No write to the collection.** Every preset path records zero uploads and zero local changes
    against the recording fake sync server, and leaves the private copy's bytes as they were. The
    preset module and the role name no engine write method, which a census with a planted control
    holds. The class ADR-401 declares sits on ADR-301's advisory rung.

R9. **The record's rights.** The table `preset_proposals` is created by migration
    `038701_ingest_preset_proposals.sql`. It is STRICT, has `created_at`, and has CHECKs on its
    state, on its field, and on settled-time agreeing with state.
    - `privacy.json` declares it as category `preset-proposals`: source `derived`, a purpose,
      lawful basis `contract`, retention until account deletion, export `true`, erase `delete`.
    - A row in PRIVACY.md's table names it.
    - Ingest's data-rights port exports and erases it.

R10. **The privacy page.** The paragraph under PRIVACY.md's `## Models and your data` is replaced
     by the paragraph below, and the pin in `scripts/tests/test_privacy_policy.py` changes with
     it. SPEC-368's censuses (R6 and R7) are unchanged, and they pass.

     > DeckStreak does not train or fine-tune a neural network or a language model on your data,
     > and does not build a dataset from it. The scheduling parameters in your collection are your
     > own scheduler's: either a fit of that scheduler to your own reviews, made by your Anki app,
     > or the scheduler's defaults. DeckStreak reads them, and the memory state Anki stores for
     > each card, to schedule the cards you study, and does not fit them itself. When you ask,
     > DeckStreak proposes the scheduler's defaults for one preset and keeps a record of the
     > proposal and of the parameters it would replace, so the change can be undone; your Anki app
     > applies it, not DeckStreak. An AI duty runs only when the host's AI route is configured,
     > which it is not by default. Each run sends the cards that duty covers and a summary of your
     > leeches, lapses and graded practice, not your journal, as the context of that one run, and
     > DeckStreak's database keeps neither the prompt nor the reply. A reply that passes its checks
     > is written to your own vault. What the model's provider keeps is set by that provider's
     > terms.

R11. **The pin.** `RELEASED_USERS` in `scripts/tests/test_fsrs7_pin.py` does not change: it
     names `anki` and the FSRS-7 crate alone. Ingest reads the defaults through the engine it
     already depends on, whose deck-options read, `Collection::get_deck_configs_for_update`,
     returns them as its `defaults`, so `crates/ingest/Cargo.toml` names no scheduler package in
     any dependency table. SPEC-342 gains one insert-only section that says so. `PINNED_USERS`
     does not change.

R12. **The changelog.** `changelog.d/main-preset-defaults-387.md` carries one `### Added` bullet,
     for the preset role, and one `### Changed` bullet, for the privacy page's section.

## 3. Acceptance criteria of SPEC-387

The base is the red commit. It holds the stubs that ADR-401 D4 names, and every criterion below
fails on an assertion for the reason in its row.

The fixture holds one preset for each field case of R2, one preset on the defaults, and one with
a desired retention other than the default. Its cards and reviews come from fixed seeds, built by
`crates/ingest/tests/support/synthetic.rs`. No test pins a memory state the engine computes: R1's
count is the fixture's own count of non-new cards, and R3's oracle is the engine's own `defaults`, which the test reads itself and never through the preset module.

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | `PresetRead` returns every fixture preset, with its id, name, vector, field, desired retention, deck ids and non-new card count | the stub returns no preset: `left: []` | `crates/ingest/tests/preset.rs` |
| A2 | The stored vector comes from the first field that holds values: FSRS-6, then FSRS-5, then FSRS-4, else empty (R2) | the stub returns no preset, so the `(id, field)` pairs read `[]` | `crates/ingest/tests/preset.rs` |
| A3 | A proposal records `DEFAULT_PARAMETERS` (21 values) as its proposed vector, and the stored vector, its field, desired retention and the non-new count as the undo values; the fixture's stored vector differs from the defaults | the stub records the stored vector as the proposed one | `crates/ingest/tests/preset.rs` |
| A4 | A preset already on the defaults gets no proposal, and the record count does not move | the stub records a row for every call | `crates/ingest/tests/preset.rs` |
| A5 | A second `propose` returns the open proposal and records nothing. Two connections racing on one preset leave exactly one open row. A direct second open insert is refused by the index | the stub records a second row, and the red migration has no partial index | `crates/ingest/tests/preset.rs` |
| A6 | `verify` settles `moved` (with observed time and whether retention was kept), stays open while the prior vector stands, settles `diverged` otherwise, and never settles twice | the stub settles nothing and reports open | `crates/ingest/tests/preset.rs` |
| A7 | Every preset path records zero uploads and zero local changes against the recording fake sync server, and the copy's sha256 is unchanged; a planted path that writes through the write port records uploads, and is refused first | not red: the stubs write nothing to the collection. This is a guard, and its plant proves it can fail | `crates/ingest/tests/preset_zero_upload.rs` |
| A8 | Neither `crates/ingest/src/preset.rs` nor `crates/daemon/src/role_preset.rs` names an engine write method; it prints `examined 2 files` and refuses zero, and a planted line naming `CollectionWrite` is refused by that name first | not red: the stubs name none. This is a guard with a plant | `crates/ingest/tests/preset.rs` |
| A9 | The proposal text holds the preset's name, the 21 values on one line that parse back to the same values, the steps of R6, the count, the no-change line, the undo line, and the replaced-fit line for an FSRS-6 fit | the stub's text is empty | `crates/ingest/tests/preset.rs` |
| A10 | The listing's first line is the preset with the most non-new cards, and every preset has a line | the stub's listing is empty | `crates/ingest/tests/preset.rs` |
| A11 | `deckstreakd` parses `preset list`, `preset propose <id>` and `preset verify <id>`, refuses every other shape, and lists six role names | the stub parses no `preset` command: `None` | `crates/daemon/src/main.rs` |
| A12 | Ingest's data-rights port exports every `preset_proposals` row, and erases them all | the stub declares no `preset_proposals` table | `crates/ingest/tests/data_rights.rs` |
| A13 | `PRIVACY.md` holds R10's paragraph under `## Models and your data` | `assertIn` fails: the page still holds the old paragraph | `scripts/tests/test_privacy_policy.py` |
| A14 | The registry's scheduler is still used by `anki` and the FSRS-7 crate alone, and the git one by the FSRS-7 crate alone: ingest names no scheduler package, and the test does not change | not red: the shipped test passes at the base and stays as it is. This is a guard, and a plant that lists `deck-streak-ingest` as a third user of the registry's scheduler is refused first: "the registry's scheduler is used by [...], not [...]" | `scripts/tests/test_fsrs7_pin.py` |
| A15 | `privacy.json` declares `preset-proposals` as R9 states, and `PRIVACY.md`'s table has its row | the category and the row are absent | `scripts/tests/test_privacy_policy.py` |

```acceptance
A1: cargo test -p deck-streak-ingest --test preset -- --exact every_preset_is_read_with_its_vector_retention_decks_and_non_new_cards
A2: cargo test -p deck-streak-ingest --test preset -- --exact the_stored_vector_comes_from_the_newest_field_that_holds_one
A3: cargo test -p deck-streak-ingest --test preset -- --exact a_proposal_carries_the_released_defaults_and_records_the_undo_values
A4: cargo test -p deck-streak-ingest --test preset -- --exact a_preset_already_on_the_defaults_records_no_proposal
A5: cargo test -p deck-streak-ingest --test preset -- --exact a_preset_holds_at_most_one_open_proposal
A6: cargo test -p deck-streak-ingest --test preset -- --exact verify_settles_an_open_proposal_once_by_what_the_copy_holds
A7: cargo test -p deck-streak-ingest --test preset_zero_upload -- --exact every_preset_path_records_zero_uploads_and_leaves_the_copy_unchanged
A8: cargo test -p deck-streak-ingest --test preset -- --exact no_preset_path_names_an_engine_write_method
A9: cargo test -p deck-streak-ingest --test preset -- --exact the_proposal_text_names_the_values_the_steps_the_reach_and_the_undo
A10: cargo test -p deck-streak-ingest --test preset -- --exact the_listing_puts_the_main_preset_first
A11: cargo test -p deck-streak-daemon --bin deckstreakd -- --exact tests::the_preset_role_parses_its_three_commands_and_refuses_every_other_shape
A12: cargo test -p deck-streak-ingest --test data_rights -- --exact preset_proposals_are_exported_and_erased
A13: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_states_what_deckstreak_trains_and_fits
A14: python3 -m unittest discover -s scripts/tests -p test_fsrs7_pin.py -k test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler
A15: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_preset_proposals_record_is_declared_and_named
```

The red-first record is `docs/red-first/SPEC-387.md`, with one fence line per fact:
- `red at <sha>: <failure>` and `green at <sha>` for A1 to A6, A9 to A13 and A15;
- `not red: <why>` for A7, A8 and A14.

A13 keeps its test name and its body; only the constant it reads, `PARAGRAPH`, changes. A14's
test does not change, and neither does `RELEASED_USERS`.

Shipped tests that this delivery leaves as they are:
- every other test in `test_privacy_policy.py` and `test_fsrs7_pin.py`;
- `crates/ingest/tests/skip_census.rs` (A24 still finds `CollectionWrite` named only by the skip
  day's write module);
- `crates/fsrs7/tests/coexistence.rs`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-387-deckstreak-proposes-the-schedulers-default-parameters-for-a-preset-and-the-owner-applies-them-in-anki.md` | docs | added |
| `docs/decisions/ADR-401-a-preset-moves-to-the-schedulers-defaults-through-an-advisory-write-class-not-a-refit-and-not-a-deckstreak-write.md` | docs | added |
| `docs/schematics/scheduler-presets-and-their-parameters.md` | docs | added |
| `docs/red-first/SPEC-387.md` | docs | added |
| `changelog.d/main-preset-defaults-387.md` | docs | added (R12) |
| `docs/specs/SPEC-368-the-privacy-page-says-exactly-what-deckstreak-trains-or-fits-on-your-data.md` | docs | section 10 below, appended at its end |
| `docs/specs/SPEC-342-fsrs-7-replay-time-undo-and-the-full-sync-choice-are-measured.md` | docs | section 10 below, appended at its end |
| `PRIVACY.md` | docs | the paragraph under `## Models and your data` replaced (R10); one row added to the table of `## What DeckStreak keeps` (R9) |
| `privacy.json` | docs | one category, `preset-proposals`, added (R9) |
| `migrations/038701_ingest_preset_proposals.sql` | ingest | added (R9) |
| `.sqlx/` | ingest | one entry added per new compile-checked query, if the module uses them |
| `crates/ingest/Cargo.toml` | ingest | a `[[test]]` target for `preset_zero_upload` if it needs its own process, as `skip_zero_upload` does (`:41-46`); no scheduler package in any table (R11) |
| `crates/ingest/src/lib.rs` | ingest | `pub mod preset;` |
| `crates/ingest/src/preset.rs` | ingest | added: `PresetRead` and its `RslibEngine` impl, propose, verify, the listing and the proposal text |
| `crates/ingest/src/engine.rs` | ingest | `open` (`:532`) and `bounded` (`:591`) made `pub(crate)`; no other line |
| `crates/ingest/src/data_rights.rs` | ingest | the `preset_proposals` table's export and erase (R9) |
| `crates/ingest/tests/preset.rs` | tests | added: A1 to A6 and A8 to A10 |
| `crates/ingest/tests/preset_zero_upload.rs` | tests | added: A7 |
| `crates/ingest/tests/support/synthetic.rs` | tests | a preset builder for the fixture, only if it has none |
| `crates/ingest/tests/data_rights.rs` | tests | A12 added |
| `crates/daemon/src/main.rs` | daemon | the `preset` role: its name in `NAMES`, its parse, the usage line, the dispatch, and A11 in `tests` |
| `crates/daemon/src/role_preset.rs` | daemon | added: the role's run over ingest's preset module |
| `scripts/tests/test_privacy_policy.py` | tests | `PARAGRAPH` replaced (A13); A15 added |
| `scripts/mutation-rows.d/S38700-S38799.json` | tests | added: the rows of section 8 |

## 5. What this does NOT cover

- It writes no preset. DeckStreak's own write of a preset's parameters, which is this class's
  approval rung, is later work under the write classes (#514).
- It fits no parameters and evaluates none. A refit judged on held-out reviews is later work for
  the same class (#514). Any fit DeckStreak makes owes this page's section its own amendment
  (#722).
- It moves no preset to FSRS-7, and it changes neither the FSRS-7 crate nor its pin's
  `PINNED_USERS` (#641).
- It changes no desired retention, no other preset option and no deck's preset (#514).
- It adds no row to the engine core's allow-list, and `UpdateDeckConfigs` stays refused (#623).
- It adds no web or iOS screen for presets. The clients' preset screens belong to the later
  parity phases (#611).
- It gives the web and iOS apps no export or erase path for the new record (#721).
- It names no model provider and adds no AI duty (#722).
- It checks the drill surface for nothing (#158).

## 6. Risks

| risk | what detects it |
|---|---|
| The defaults predict recall worse than the preset's current fit. Published benchmarks find a fitted vector beats the defaults for most collections | R6's replaced-fit line warns before the move. R7's change point lets a later held-out refit and the weekly predicted-against-observed report judge it |
| Another client edits the same preset before it has synced the move. Deck configs merge last writer wins, so that edit replaces the move whole | `verify` reads `diverged`. The schematic's sync section names the order to follow |
| The private copy lags the server until DeckStreak's next sync | `verify` says the proposal is still open, and records nothing |
| A pasted value parses to a different number | A9 pins the round-trip. `verify` reads `diverged` |
| The engine's field precedence, or its recompute on save, differs from section 1c at the build's cut | the Assumption: the build stops |
| An engine upgrade changes the deck-options read or its `defaults` | the build's re-measure at its cut stops on any difference (R11) |

## 7. Formal model

None, by surface. The checks:
- Each check-then-act, propose's lookup and insert and verify's read and settle, sits in one
  immediate transaction under the kernel's repository base. A partial unique index backs propose,
  and a guarded update backs verify. A5 and A6 drive the racing shape directly.
- The private copy is read under the collection's existing shared lock.
- DeckStreak writes nothing to the collection.
- No `@phx covers` line names a file in section 4 (0 of 13 that name `ingest` or `daemon` paths).

The build raises this to required if an implementation splits a check and its act across two
transactions.

## 8. Mutation rows

Band `scripts/mutation-rows.d/S38700-S38799.json`. The build writes each exact `find` from the
committed code, proves that it occurs once, and proves its killer kills.

| stem | file | find -> replace (the property) | killer |
|---|---|---|---|
| S38700-PRESET-FIELD-ORDER | `crates/ingest/src/preset.rs` | the FSRS-6 field read before the FSRS-5 one -> the order swapped | A2 |
| S38701-PRESET-EMPTY-FIELD | `crates/ingest/src/preset.rs` | an empty field skipped -> an empty field taken | A2 |
| S38702-PRESET-NON-NEW-COUNT | `crates/ingest/src/preset.rs` | new cards left out of the count -> counted | A1 |
| S38703-PRESET-DECK-IDS | `crates/ingest/src/preset.rs` | decks kept by their preset id -> every deck kept | A1 |
| S38704-PROPOSAL-DEFAULTS | `crates/ingest/src/preset.rs` | the proposed vector read from `DEFAULT_PARAMETERS` -> the stored vector | A3 |
| S38705-PROPOSAL-UNDO-VALUES | `crates/ingest/src/preset.rs` | the stored vector bound as the prior one -> the proposed vector | A3 |
| S38706-PROPOSAL-RETENTION | `crates/ingest/src/preset.rs` | the preset's desired retention bound -> a fixed 0.9 | A3 |
| S38707-ON-DEFAULTS | `crates/ingest/src/preset.rs` | the on-defaults test -> `false` | A4 |
| S38708-OPEN-REUSED | `crates/ingest/src/preset.rs` | the open-proposal lookup's hit returned -> skipped | A5 |
| S38709-VERIFY-MOVED | `crates/ingest/src/preset.rs` | the stored vector compared with the proposed one -> with the prior one | A6 |
| S38710-VERIFY-STAYS-OPEN | `crates/ingest/src/preset.rs` | the prior vector leaves the proposal open -> settles it `diverged` | A6 |
| S38711-VERIFY-RETENTION-KEPT | `crates/ingest/src/preset.rs` | retention compared for equality -> for inequality | A6 |
| S38712-VERIFY-SETTLE-ONCE | `crates/ingest/src/preset.rs` | the settle's `state = 'open'` guard -> the statement re-spelt at run time without it | A6 |
| S38713-TEXT-STEPS | `crates/ingest/src/preset.rs` | the "Reschedule cards on change" step -> removed | A9 |
| S38714-TEXT-VALUES | `crates/ingest/src/preset.rs` | each value printed to parse back -> printed to four places | A9 |
| S38715-TEXT-UNDO | `crates/ingest/src/preset.rs` | the undo line from the prior vector -> from the proposed one | A9 |
| S38716-LISTING-ORDER | `crates/ingest/src/preset.rs` | most non-new cards first -> fewest first | A10 |
| S38717-ROLE-NAME | `crates/daemon/src/main.rs` | the `preset` arm's name -> `presets` | A11 |
| S38718-ONE-OPEN-INDEX | `migrations/038701_ingest_preset_proposals.sql` | the partial unique index's `WHERE state = 'open'` -> dropped (a repository-rooted row, killed by a cargo test) | A5 |
| S38719-RIGHTS-ERASE | `crates/ingest/src/data_rights.rs` | the `preset_proposals` erase -> its table name changed | A12 |

## 9. What only CI proves

| # | what | where |
|---|---|---|
| C1 | The whole Rust and Python suites pass with the new module, the role and the edited page | CI on the train's pull request |
| C2 | The band's rows are killed by the gate's own runner | CI's mutation jobs |
| C3 | The artifact bar and the public-text scan read the SPEC, the ADR, the schematic, the fragment and the page clean | CI on the train's pull request |

## 10. The insert-only sections this delivery appends

To the end of SPEC-368, one blank line before it:

    ## 10. Amendments

    ### SPEC-387: R2's paragraph is replaced

    SPEC-387 R10 replaces R2's paragraph with the text quoted there, in its own delivery, as R3
    and R7 require. R1 and R3 to R7 stand, and every census of R6 and R7 is unchanged. DeckStreak
    still fits nothing: a fit, when one exists, is made by the learner's Anki app, and DeckStreak
    now proposes the scheduler's defaults for one preset on request and records the proposal.

To the end of SPEC-342, one blank line before it:

    ## 8. Amendments

    ### SPEC-387: the ingest crate reads the released scheduler's defaults

    R2 gains no user. The ingest crate reads the released scheduler package's default parameters
    through the engine it already depends on (SPEC-387 R11), so it names no scheduler package, and
    `RELEASED_USERS` in `scripts/tests/test_fsrs7_pin.py` still names only the engine and the
    FSRS-7 crate. An engine upgrade that moves the released package moves the defaults SPEC-387
    proposes with it. `PINNED_USERS` is unchanged, and so is every other line of this SPEC.

## 11. Amendments

### The four paths the tests already on dev require

Section 4 named 24 paths. Four more are required by tests already on dev, and are part of this delivery:

| path | why | change |
|---|---|---|
| `docs/CONTEXT-MAP.md` | `crates/kernel/tests/schema.rs` and `crates/coordination/tests/data_rights_symmetry.rs` read its table register | one row under `### DeckStreak's own tables`: `preset_proposals`, owned by `ingest` |
| `crates/coordination/tests/data_rights_symmetry.rs` | its probe asserts that every table of the schema holds a seeded row | one seed for `preset_proposals` in `SEEDS` |
| `crates/daemon/tests/roles.rs` | two assertions pin the role list the usage text prints | both lists name the new role beside the five |
| `scripts/mutation-rows.d/S11900-S11999.json` | row `S11937-MCP-ROLE` anchors on the whole role-name line | its `find` and `replace` move to the new line with the same mutation; its killer is unchanged and is re-measured red on the mutant |

No assertion is removed and no row loses its killer.

### A preset read never writes the copy

The engine's deck-options read computes the day's timing first. In client mode that computation writes the copy when the copy's configured UTC offset differs from the process zone, or when its rollover hour is unset. The preset read therefore refuses, before any engine read, when either write would happen, and the copy stays byte-identical (R8, A7). A7's fixture sets the configured offset away from the process zone and asserts the refusal; a second case with the offset equal to the process zone reads the defaults and still records zero writes.

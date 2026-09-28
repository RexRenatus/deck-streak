# SPEC-093: a leech is snapshotted each sync, and its remediation pays once and can be undone

- **Wave:** W4. **Issue:** #133 (epic #5). **Context(s):** `deck-streak-curriculum` (the leech rows,
  the board, the protocol, the summary, the leech port, `leech_snapshot` and `leech_remediation`);
  `deck-streak-progression` (the derived source `leech:<card id>` in the settlement's registry);
  `deck-streak-insights` (the leech breakdown's shape); `deck-streak-coordination` (the leech step
  in phase 4, the remediate and undo use cases, the law block's wiring, the chart's read model);
  `deck-streak-api`, `deck-streak-bot` and the Mini App (the routes, `/leeches`, `/leech`,
  `/unleech`, the leech list and its action sheets).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-072 (derived XP is settled, and
  the owner's correction replaces an amount), ADR-085 (charts render on the client), ADR-091
  (curriculum's readouts are computed in the current study day's step), ADR-092 (ingest gives a
  card's law subject) and ADR-093 (a remediation settles `leech:<card id>` once, and an undo is the
  owner's correction to zero).
- **Prerequisites:** SPEC-029, SPEC-071 (the fold, the leech threshold), SPEC-072 (the settlement
  and the day base), SPEC-077 (each card's difficulty and the law block's leech port), SPEC-085 (the
  chart route and loader) and SPEC-092 (the strand parse and the law subject). **Mutation band:**
  `S09300-S09399`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-093.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` dd98601 `crates/curriculum/src/` holds only `lib.rs`. SPEC-077
  (planned) renders the active law leeches and the law mastery pillar as pending until this SPEC
  wires the leech port (its R12), and SPEC-071 (planned) counts leeches in the card snapshot without
  the workflow (its §5).
- **What is ported.** `leeches.py:cards_to_leech_rows`, `build_board`, `remediation_protocol` and
  `leech_summary`; the snapshot's refresh and the two owner actions
  (`pipeline.py:GamifyPipeline._persist_leeches`, `leech_board`, `remediate_leech` and
  `unremediate_leech`); the once-per-card XP (`database.py:GamifyStore.grant_leech_xp_once`) and its
  undo (`database.py:GamifyStore.delete_xp_source`); and the leech breakdown chart
  (`charts.py:leech_breakdown`).
- **Traps a hand port falls into.**
  - A leech is a card with lapses at or above the threshold (`DECKSTREAK_LEECH_THRESHOLD`, 8 by
    default, SPEC-071), active OR suspended; the card snapshot's leech count (SPEC-071) leaves out
    suspended cards, so the two counts differ by design.
  - A law card's row carries the track marker `law` and its law subject (SPEC-092) in the course
    slots; any other card carries its course, or nothing. The strand comes from SPEC-092's parse and
    is empty when there is none.
  - The status is `active` without a remediation, `regressed` when the lapses now exceed the
    lapses recorded at the remediation, else `holding`. The board sorts regressed, active, holding,
    then lapses and difficulty descending, then the card id, and keeps the first 15 (at least 1).
  - The summary counts the CAPPED board, not the snapshot, and its worst strand is the strand with
    the most active or regressed rows, the first such in the board's order on a tie.
  - The protocol splits at difficulty 8.0 or 12 lapses; the cross-link step appears only when the
    card has a strand.
  - The law block's "active law leeches" is every law row of the snapshot, remediated or not
    (`pipeline_layers/digests.py:DigestsLayer.law_track_summary`), not the board's active status.
  - The XP is 40 once per card, ever: a re-remediation of a regressed card pays nothing; an undo
    removes the pay, after which a remediation pays again. The source is on the `language` track
    for every card, a law card included, as the predecessor's ledger records it (ADR-093).
  - The day base leaves `leech:` sources out (`database.py:GamifyStore.day_base_xp`), so a
    remediation never feeds the consistency bonus or the coin mint.
  - A remediation's action is cut to 40 characters (empty reads `fixed`) and its note to 200; a card
    not in the snapshot is refused. DeckStreak never writes to the collection.
- **Corrections to the accepted documents.** `docs/schematics/leeches-state-machine.md` says an undo
  removes "the XP grant"; here the XP is a settlement an undo sets to zero on its day (ADR-093), which
  a new schematic records beside it. `docs/LEXICON.md` glosses a leech as not suspended, which is the
  card snapshot's count; the board's leech includes suspended cards, and the gloss says both.
- **What the parity oracle proves.** The rows over synthetic cards, law paths and courses; the board
  over every status and tie; the protocol at its edges; the summary's capped counts and tie; the
  breakdown's bars; and the constants.
- **Prerequisites.** SPEC-029, SPEC-071, SPEC-072, SPEC-077, SPEC-085 and SPEC-092, as the header
  names them.

## 2. Requirements

The leeches

R1. The leech rows equal `goldens/leech_rows.json` (`leeches.py:cards_to_leech_rows`), the board
    equals `goldens/leech_board.json` (`leeches.py:build_board`, with the limit of 15), the protocol
    equals `goldens/remediation_protocol.json` (`leeches.py:remediation_protocol`), and the summary
    equals `goldens/leech_summary.json` (`leeches.py:leech_summary`).
R2. The leech step registers in phase 4 of SPEC-071's fold and runs in the current study day's step
    only: coordination passes the threshold, and the step replaces `leech_snapshot` in one write. A
    remediation outlives its card's leaving the snapshot.
R3. Curriculum's leech port gives the active law leeches (every law row of the snapshot), which
    SPEC-077's law block and law mastery pillar read in place of their pending value.

The remediation (ADR-093)

R4. Remediating a card in the snapshot records its action, note, study day and current lapses in
    `leech_remediation` (a second remediation replaces the first), and, when no `leech:<card id>`
    settlement with a positive amount exists on any day, settles 40 on the current study day on the
    `language` track, as the owner's correction, in the same transaction. A card not in the
    snapshot is refused and nothing is written.
R5. Undoing a remediation deletes its record and settles every `leech:<card id>` row to 0 on its own
    study day, as the owner's correction; the level is read again from the totals (SPEC-040 R8).
R6. Progression's derived registry gains `leech:<card id>` (ADR-072's closed list, extended by this
    SPEC). The day base leaves every `leech:` source out, as the predecessor's rule does (SPEC-072
    R17), and `economy.json` declares the 40 as `xp.bonuses.leech_remediation` and names `leech` in
    `xp.day_base_excludes`.

The tables

R7. `leech_snapshot` and `leech_remediation` are `STRICT` tables with `created_at`, created by
    `migrations/009301_curriculum_leeches.sql` (SPEC-020 R15, R18). Both are registered in the
    context map's register of DeckStreak's own tables, declared in `privacy.json` (the category
    `leech-remediation`), given a line in `PRIVACY.md`, and exported and erased by curriculum's
    data-rights port; the symmetry test seeds both.

The surfaces

R8. `GET /api/curriculum/leeches` serves the board, its summary and the top card's protocol;
    `POST /api/curriculum/leeches/{card_id}/remediate` and `POST /api/curriculum/leeches/{card_id}/undo`
    run R4 and R5. All three answer the owner's session only; any other caller is answered 401 or
    403 with no data and no write.
R9. `/leeches` states the board and the top card's protocol; `/leech` with a card id (and an
    optional action) remediates it and `/unleech` with a card id undoes it, each answering as the
    routes do.
R10. The Mini App lists the board with each card's id, course or law subject, strand, lapses and
     status, and never its content; each card opens an action sheet offering Fix on an active card
     and Undo on a remediated one. `leech_breakdown` joins SPEC-085's closed set of charts: its bars,
     colours, title and empty state equal `goldens/charts_leech_breakdown.json`
     (`charts.py:leech_breakdown`), shaped in insights over three counts and a strand and drawn on
     the client.
R11. Every constant this SPEC uses (the 40 XP, the board's 15, the protocol's 8.0 and 12, the law
     marker, the action's 40 and the note's 200 characters) equals `goldens/leeches.constants.json`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the rows equal the golden of `leeches.py:cards_to_leech_rows`, suspended and law cards included | `leech_rows_match_the_predecessors_golden` |
| A2 | the board equals the golden of `leeches.py:build_board` | `the_leech_board_matches_the_predecessors_golden` |
| A3 | the protocol equals the golden of `leeches.py:remediation_protocol` | `the_remediation_protocol_matches_the_predecessors_golden` |
| A4 | the summary equals the golden of `leeches.py:leech_summary` over the capped board | `the_leech_summary_matches_the_predecessors_golden` |
| A5 | every leech constant equals `goldens/leeches.constants.json` | `the_leech_constants_equal_the_predecessors` |
| A6 | the active law leeches count every law row, and no law row reaches a course's count | `active_law_leeches_count_every_law_row` |
| A7 | a remediation settles 40 once, on the current study day and the `language` track | `a_remediation_settles_forty_once` |
| A8 | re-remediating a regressed card settles nothing more | `re_remediating_a_regressed_card_never_settles_again` |
| A9 | an undo settles the source to 0 on its own study day, a closed day included | `an_undo_settles_the_source_to_zero_on_its_day` |
| A10 | a remediation after an undo settles 40 on the new study day | `a_remediation_after_an_undo_settles_again` |
| A11 | a card outside the snapshot is refused and nothing is written | `a_card_outside_the_snapshot_is_refused` |
| A12 | the snapshot is replaced at each recompute, and a remediation outlives its card's leaving | `the_snapshot_is_replaced_and_remediations_outlive_it` |
| A13 | the day base leaves `leech:` settlements out | `the_day_base_leaves_out_leech_settlements` |
| A14 | both tables are exported and erased by curriculum's port | `the_leech_tables_are_exported_and_erased` |
| A15 | the three routes answer the owner and refuse every other caller with no write | `the_leech_routes_answer_only_the_owner` |
| A16 | `/leech` and `/unleech` answer as the routes do | `leech_and_unleech_answer_as_the_routes_do` |
| A17 | the breakdown's shape equals the golden of `charts.py:leech_breakdown` | `the_leech_breakdown_matches_the_predecessors_golden` |
| A18 | the action sheet offers Fix on an active card and Undo on a remediated one | `offers fix on an active leech and undo on a remediated one` |
| A19 | the list shows card ids and strands and never card content | `shows card ids and strands and never card content` |

```acceptance
A1: cargo test -p deck-streak-curriculum --test leech_goldens -- --exact leech_rows_match_the_predecessors_golden
A2: cargo test -p deck-streak-curriculum --test leech_goldens -- --exact the_leech_board_matches_the_predecessors_golden
A3: cargo test -p deck-streak-curriculum --test leech_goldens -- --exact the_remediation_protocol_matches_the_predecessors_golden
A4: cargo test -p deck-streak-curriculum --test leech_goldens -- --exact the_leech_summary_matches_the_predecessors_golden
A5: cargo test -p deck-streak-curriculum --test leech_goldens -- --exact the_leech_constants_equal_the_predecessors
A6: cargo test -p deck-streak-curriculum --test leech_port -- --exact active_law_leeches_count_every_law_row
A7: cargo test -p deck-streak-coordination --test leech_remediation -- --exact a_remediation_settles_forty_once
A8: cargo test -p deck-streak-coordination --test leech_remediation -- --exact re_remediating_a_regressed_card_never_settles_again
A9: cargo test -p deck-streak-coordination --test leech_remediation -- --exact an_undo_settles_the_source_to_zero_on_its_day
A10: cargo test -p deck-streak-coordination --test leech_remediation -- --exact a_remediation_after_an_undo_settles_again
A11: cargo test -p deck-streak-coordination --test leech_remediation -- --exact a_card_outside_the_snapshot_is_refused
A12: cargo test -p deck-streak-coordination --test leech_step -- --exact the_snapshot_is_replaced_and_remediations_outlive_it
A13: cargo test -p deck-streak-progression --test leech_source -- --exact the_day_base_leaves_out_leech_settlements
A14: cargo test -p deck-streak-curriculum --test leech_store -- --exact the_leech_tables_are_exported_and_erased
A15: cargo test -p deck-streak-api --test leech_routes -- --exact the_leech_routes_answer_only_the_owner
A16: cargo test -p deck-streak-bot --test leech_commands -- --exact leech_and_unleech_answer_as_the_routes_do
A17: cargo test -p deck-streak-insights --test charts_leech -- --exact the_leech_breakdown_matches_the_predecessors_golden
A18: pnpm exec vitest run web/app/src/lib/leeches/LeechSheet.test.ts -t "offers fix on an active leech and undo on a remediated one"
A19: pnpm exec vitest run web/app/src/lib/leeches/LeechList.test.ts -t "shows card ids and strands and never card content"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, accessibility and game-economy packs
stay enforced; no check is deferred or lifted for this delivery, so the private wiring does not
change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/curriculum/src/data_rights.rs`: the `leech-remediation` category names `leech_snapshot` and `leech_remediation` with purpose, basis and retention, and export and erase cover both | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/leeches/+page.svelte` and every file under `web/app/src/lib/leeches/`: the list, the action sheet and the breakdown pass the accessibility audit in both Telegram colour schemes | the accessibility pack |
| B3 | over `economy.json`'s `xp` section: the leech remediation's 40 is declared once, and the day base's exclusions name `leech` | the game-economy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/curriculum/src/leeches.rs` | `deck-streak-curriculum` | added: the rows, the board, the protocol, the summary and the leech port |
| `crates/curriculum/src/leech_store.rs` | `deck-streak-curriculum` | added: `leech_snapshot` and `leech_remediation` |
| `crates/curriculum/src/data_rights.rs` | `deck-streak-curriculum` | changed: the port exports and erases both tables |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the modules |
| `crates/curriculum/tests/leech_goldens.rs` | `deck-streak-curriculum` | added: A1 to A5 |
| `crates/curriculum/tests/leech_port.rs` | `deck-streak-curriculum` | added: A6 |
| `crates/curriculum/tests/leech_store.rs` | `deck-streak-curriculum` | added: A14 |
| `migrations/009301_curriculum_leeches.sql` | `deck-streak-curriculum` | added: `leech_snapshot` and `leech_remediation` |
| `crates/progression/src/settle.rs` | `deck-streak-progression` | changed: the derived registry gains `leech:<card id>` |
| `crates/progression/tests/leech_source.rs` | `deck-streak-progression` | added: A13 |
| `economy.json` | repo | changed: `xp.bonuses.leech_remediation` and `leech` in `xp.day_base_excludes` |
| `crates/insights/src/charts.rs` | `deck-streak-insights` | changed: the leech breakdown's shape |
| `crates/insights/tests/charts_leech.rs` | `deck-streak-insights` | added: A17 |
| `crates/coordination/src/recompute/leeches.rs` | `deck-streak-coordination` | added: the leech step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the leech step in phase 4 |
| `crates/coordination/src/leeches.rs` | `deck-streak-coordination` | added: the board's read model and the remediate and undo use cases |
| `crates/coordination/src/law/mod.rs` | `deck-streak-coordination` | changed: the law block reads the leech port |
| `crates/coordination/src/charts/mod.rs` | `deck-streak-coordination` | changed: `leech_breakdown` joins the closed set |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/leech_remediation.rs` | `deck-streak-coordination` | added: A7 to A11 |
| `crates/coordination/tests/leech_step.rs` | `deck-streak-coordination` | added: A12 |
| `crates/api/src/leech_routes.rs` | `deck-streak-api` | added: the three routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes behind the owner's session |
| `crates/api/tests/leech_routes.rs` | `deck-streak-api` | added: A15 |
| `crates/bot/src/leech_commands.rs` | `deck-streak-bot` | added: the board, remediate and undo commands |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the commands join the table |
| `crates/bot/tests/leech_commands.rs` | `deck-streak-bot` | added: A16 |
| `web/app/src/routes/leeches/+page.svelte` | miniapp | added: the leech list and its breakdown |
| `web/app/src/lib/leeches/leeches.ts` | miniapp | added: the routes' client and types |
| `web/app/src/lib/leeches/LeechList.svelte` | miniapp | added |
| `web/app/src/lib/leeches/LeechList.test.ts` | miniapp | added: A19 |
| `web/app/src/lib/leeches/LeechSheet.svelte` | miniapp | added: the action sheet |
| `web/app/src/lib/leeches/LeechSheet.test.ts` | miniapp | added: A18 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /leeches joins `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains both tables |
| `docs/LEXICON.md` | docs | changed: the leech gloss names the board's leech and the snapshot's count |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry lists both tables under curriculum's port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for each table |
| `privacy.json` | repo | changed: the `leech-remediation` category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_093.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/leech_rows.json` | repo | added: the golden of `leeches.py:cards_to_leech_rows` (adapter; synthetic courses and law root) |
| `tools/parity-oracle/goldens/leech_board.json` | repo | added: the golden of `leeches.py:build_board` (function) |
| `tools/parity-oracle/goldens/remediation_protocol.json` | repo | added: the golden of `leeches.py:remediation_protocol` (function) |
| `tools/parity-oracle/goldens/leech_summary.json` | repo | added: the golden of `leeches.py:leech_summary` (adapter; boards from `build_board`) |
| `tools/parity-oracle/goldens/charts_leech_breakdown.json` | repo | added: the golden of `charts.py:leech_breakdown` (adapter; the bars captured, never an image) |
| `tools/parity-oracle/goldens/leeches.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S09300-S09399.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-093-a-leech-is-snapshotted-each-sync-and-its-remediation-pays-once-and-can-be-undone.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-093.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It prepares no explanation, mnemonic or contrast for a leech; the leech doctor does (#47).
- It reads no card's content, so the list shows ids and strands; a note-field read path would be
  its own decision (#133).
- It writes nothing to the collection: the owner edits the card in Anki (#133).
- It serves no board to the agent's machine read tool (#157).
- It builds no settings screen for the leech threshold (#57).
- It imports the predecessor's remediation records with the v9 import, never here (#61).

## 6. Risks

- **A double pay** from a re-remediation or a retried request. Detected by A8, and prevented by the
  any-day check running in the settle's transaction.
- **An undo that leaves the pay on a closed day,** because a recompute's settle never lowers a
  closed day. Prevented by the owner's-correction cause, and detected by A9.
- **The law block counts only unremediated law rows,** unlike the predecessor. Detected by A6.
- **A remediation feeds the consistency bonus.** Detected by A13.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_093.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `leech_rows` | `leeches.py:cards_to_leech_rows` | adapter | patches the law root, the year bands and the courses where the module reads them; cards at and under the threshold, suspended, borrowed, law and outside every course |
| `leech_board` | `leeches.py:build_board` | function | none: snapshot rows and remediations for every status, ties on lapses and difficulty, and limits 0, 1 and 15 |
| `remediation_protocol` | `leeches.py:remediation_protocol` | function | none: difficulty 7.99 and 8.0, lapses 11 and 12, with and without a strand |
| `leech_summary` | `leeches.py:leech_summary` | adapter | boards from `build_board` over more than 15 rows and a worst-strand tie |
| `charts_leech_breakdown` | `charts.py:leech_breakdown` | adapter | records the axes' bars, labels, colours, title and empty text instead of rendering |
| `leeches.constants` | `constants.LEECH_REMEDIATION_XP`, `LEECH_BOARD_LIMIT`; `leeches.LAW_LEECH_TRACK`; the action's and note's cuts in `pipeline.py:GamifyPipeline.remediate_leech` | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `leech_snapshot` | `deck-streak-curriculum` | `migrations/009301_curriculum_leeches.sql` | `leech_snapshot`, which the import does not carry: the first recompute replaces it | exported and erased |
| `leech_remediation` | `deck-streak-curriculum` | `migrations/009301_curriculum_leeches.sql` | `leech_remediation`, row for row, by the v9 import (#61); its `leech:` rows go to `xp_settlement` (ADR-072) | exported and erased; an erase settles no XP back, since the XP tables are erased by their own port |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09301-SUSPENDED-IS-A-LEECH` | `crates/curriculum/src/leeches.rs` | a suspended card with enough lapses is a row | `leech_goldens::leech_rows_match_the_predecessors_golden` |
| `S09302-REGRESSED-FIRST` | `crates/curriculum/src/leeches.rs` | the board's status order | `leech_goldens::the_leech_board_matches_the_predecessors_golden` |
| `S09303-BOARD-LIMIT` | `crates/curriculum/src/leeches.rs` | the board's 15 | `leech_goldens::the_leech_constants_equal_the_predecessors` |
| `S09304-SPLIT-DIFFICULTY` | `crates/curriculum/src/leeches.rs` | the protocol's split at 8.0 | `leech_goldens::the_remediation_protocol_matches_the_predecessors_golden` |
| `S09305-ONCE-PER-CARD` | `crates/coordination/src/leeches.rs` | the any-day check before a settle | `leech_remediation::re_remediating_a_regressed_card_never_settles_again` |
| `S09306-UNDO-IS-A-CORRECTION` | `crates/coordination/src/leeches.rs` | the undo's cause is the owner's correction | `leech_remediation::an_undo_settles_the_source_to_zero_on_its_day` |
| `S09307-LEECH-XP` | `crates/curriculum/src/leeches.rs` | the remediation's 40 XP | `leech_goldens::the_leech_constants_equal_the_predecessors` |
| `S09308-UNDO-ON-ITS-OWN-DAY` | `crates/coordination/src/leeches.rs` | an undo settles each row on the row's own study day, never today | `leech_remediation::an_undo_settles_the_source_to_zero_on_its_day` |

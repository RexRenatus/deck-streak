# SPEC-140: each context imports its own v9 rows, and the recovered card state keeps its provenance

- **Wave:** W8. **Issue:** #61 (epic #9), its first and third criteria for the study record.
  **Context(s):** `deck-streak-kernel` (the import port, its counts and the source rows); the owners
  of the study record, each writing only its own tables: `deck-streak-analytics` (`daily_rollup`,
  `daily_lang_stats`, and the card state's origin), `deck-streak-progression` (`xp_ledger`,
  `xp_settlement`, `buffs`, `badges_earned`, `records`, `season_nodes`, `season_targets`,
  `xp_price_changes`), `deck-streak-streaks` (`streak_state`, `freeze_events`, `habit_strength`,
  `governor_state`), `deck-streak-curriculum` (`language_progress`, `band_milestones`, `law_dues`,
  `leech_remediation`, `can_do_unlocks`), `deck-streak-habits` (`minutes_log`, `writing_log`),
  `deck-streak-focus` (`focus_log`, `focus_timer`), `deck-streak-ingest` (`skip_days`,
  `skip_card_snapshot`), `deck-streak-readings` (`readings`, `reading_attempts`,
  `reading_topic_days`, `reading_runs`), `deck-streak-vault` (`drill_grades`) and
  `deck-streak-markets` (`market_positions`); `deck-streak-coordination` (a test that the
  recompute keeps an imported day's card state).
- **Decided by:** ADR-008 (the v9 import follows `data-migration.json`), ADR-011 (side by side),
  ADR-012 (the parity oracle proves the math), ADR-071 (the recompute settles each study day once,
  with its end-of-day state), ADR-072 (the import writes the derived sources to
  `xp_settlement` as closed days), ADR-140 (each owner writes its own imported rows, in one
  transaction, and the predecessor's record wins up to its last study day) and ADR-141 (the
  recovered card state keeps its origin in its own column).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-029, SPEC-040, SPEC-045 and SPEC-071 (landed); SPEC-046,
  SPEC-047, SPEC-072, SPEC-073, SPEC-074, SPEC-076, SPEC-077, SPEC-078, SPEC-079, SPEC-083,
  SPEC-093, SPEC-099, SPEC-107 and SPEC-110 (planned, unlanded), and SPEC-138 (W7, in its plan's
  review, unlanded). **Mutation band:** `S14000-S14099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-140.md` (ADR-016).

## 1. The problem, measured

- **No context can take an imported row.** At `dev` a4036b3 no crate has an import module, and the
  kernel names no import type. The v9 register in `docs/CONTEXT-MAP.md` assigns each of the
  predecessor's 64 tables (schema 24) an owning context, and every SPEC that creates a table names
  in its §8 which predecessor table maps into it; nothing yet writes one.
- **Four censuses refuse a second writer by name.** SPEC-040 A9 refuses any crate but progression
  whose code names `xp_ledger`; SPEC-072 R9 does the same for `xp_settlement`, SPEC-138 for
  `xp_price_changes` and SPEC-137 for the publishing tables. A one-off crate that translated and
  wrote every table would be refused by all four, and rightly: it would be a second writer. So each
  owner writes its own imported rows, and the import crate (SPEC-142) only composes them.
- **The recovered card state has no column that can hold its provenance.** `daily_rollup`'s
  `card_state_src` is checked `GLOB 'live:[0-9]*'` (`migrations/007101_analytics_daily_rollup.sql`),
  and a table check ties a NULL source to NULL counts. The predecessor stamps each day's card
  state `live:<instant>` when it read it on the day, `backup:<instant>` when its one-shot recovery
  (`backfill_card_state_from_backups.py:plan_backfill`) restored it from its own retained backups,
  and `void:fossil` when that recovery proved the stored counts a fabrication and zeroed them; a
  day with no stamp was never recorded, or was recorded before the stamp existed. Its daily view
  (`pipeline_layers/base.py:PipelineBase._render_daily_from_rollup`) renders no card snapshot for
  an absent or void stamp, and leaves the learning term out for a recovered one. #61's third
  criterion asks that this history arrive with its provenance; SPEC-071's §8 promises the same.
- **A closed day is written by the recompute alone.** SPEC-071 R9 lets only the recompute that
  evaluates or settles a day write its card state, R16 never settles a day twice, and R18 re-rolls a
  day only when its fingerprint changes. An imported day must arrive settled, and must not be
  re-settled or have its card state overwritten by the first recompute.
- **DeckStreak's own rows overlap the predecessor's.** Side by side (ADR-011), DeckStreak computes
  its own record for the same study days. For every day up to the predecessor's last, the two hold
  rows for one key, and a ledger could count one day twice.

## 2. Requirements

The port

R1. The kernel gains `crates/kernel/src/import.rs`: `SourceRow` (one predecessor row, its columns by
    name, each a NULL, an integer, a real, a text or a blob), `SourceTables` (the rows of every
    source table by the table's name, read-only), `ImportCount` (`inserted`, `replaced`,
    `unchanged`, `superseded`, `kept`, and `filtered`, the source rows a pair's declared filter
    leaves out, which only a pair whose §8 rule is `filtered` answers; `add`, and `written`, the
    sum of `inserted`, `replaced` and `superseded`),
    `ImportContext` (the cutoff, which is the latest study day the source's `daily_rollup` holds,
    and the configured courses, by which a language code becomes a course code), and the trait
    `ImportPort`: `writers()`, the pairs of source table and target table the owner writes, and
    `write(connection, source, context)`, which writes them inside the transaction the caller holds
    on `connection` and answers one `ImportCount` per pair. A port opens no connection and no
    transaction of its own, as SPEC-021's erase does not.
R2. Each owner named in the header implements `ImportPort` in `crates/<context>/src/import.rs`,
    writing only its own tables, and translating the predecessor's rows itself: the predecessor's
    columns, day strings and codes are read nowhere else. A writer reads the source tables its §8
    row names and no other.

The precedence (ADR-140)

R3. A predecessor row whose key DeckStreak does not hold is `inserted`; one whose key DeckStreak holds
    with every column equal (`updated_at` aside) is `unchanged`; one whose key DeckStreak holds with
    any other column different is `replaced` by the predecessor's. A table with no study day in its
    key (a singleton, a per-track row) therefore takes the predecessor's row.
    A DeckStreak row whose key the predecessor does not hold is `kept`, and counted `kept` when its
    study day is on or before the cutoff, so the dry run shows what DeckStreak alone recorded in
    the predecessor's days (an owner's action in DeckStreak's own surfaces among it). The one
    exception is a day reading, a table holding one derived reading per study day that the recompute
    writes (`daily_rollup`, `daily_lang_stats`, `xp_settlement` and `habit_strength`): there a
    DeckStreak row on or before the cutoff whose key the predecessor does not hold is `superseded`
    (deleted), because two systems' readings of one day cannot both stand, and a row after the
    cutoff is kept, counted nowhere.
R4. Every imported row's `created_at` and `updated_at` come from the predecessor's row, or, where it
    has none, from the start of the row's study day in epoch milliseconds; never from the clock. So a
    second write of the same source answers every pair `unchanged` with `written` 0 (#61's second
    criterion).
R5. An imported row is history, not an event. No writer calls the grant port, the settle, the coin
    mint or the router, and no occasion is raised for an imported level, badge, record, season node
    or streak (SPEC-072 R14 compares levels within one recompute, so a level the import reaches is
    never a crossing). Progression's writers are the one other statement beside SPEC-040 R10's erase
    that changes `xp_ledger`, and they live beside the grant port in progression, so SPEC-040 A9's
    census and SPEC-072 R9's stand unchanged.

The study record

R6. Analytics maps each `daily_rollup` row to one row, its metrics and score unchanged, its day XP and
    track column dropped (SPEC-071 §8), with `settled_at` the predecessor's `updated_at` in epoch
    milliseconds, `score_at_close` its score, and `fingerprint` the literal `imported`, which no
    digest equals, so the recompute re-rolls an imported day inside its window (SPEC-071 R18)
    without settling it again (R16) or writing its card state (R9). `daily_lang_stats` maps row for
    row, the language code becoming the course code.
R7. The card state's origin (ADR-141): `migrations/014001_analytics_card_state_origin.sql` adds
    `card_state_origin TEXT` to `daily_rollup`, checked `NULL`, `'backup'` with a non-NULL source, or
    `'void'` with a NULL source. The import maps `live:<instant>` to `live:<epoch ms>` with a NULL
    origin; `backup:<instant>` to `live:<epoch ms of that instant>` with origin `backup`;
    `void:fossil` to NULL counts, a NULL source and origin `void`; no stamp with NULL counts to
    NULL; and no stamp with any count (a reading taken before the stamp existed) to NULL counts and a
    NULL source, counted `unstamped` in the analytics report (the conservative answer to owner
    question 2 of the plan; ADR-141). The recompute never writes the origin.
R8. Analytics' `card_state_reading(source, origin)` answers `Absent`, `Recorded`, `Recovered` or
    `Voided`, and equals the golden `import_card_state_reading`: each stamp maps to the reading
    the predecessor's daily view gave it. The day view and export carry the origin with its row
    (SPEC-021: the analytics data-rights port exports the new column).
R9. Progression maps the predecessor's ledger by source: the derived sources (review XP, the daily
    bonuses, consistency, Ascendant, and the habit, focus, token and `leech:` sources ADR-072
    names) go to `xp_settlement` as closed days, and every other source to `xp_ledger`, so the sum
    of both equals the source ledger's sum (the count rule `sum`). `xp_state` is not read: the total
    and the level are derived at read (SPEC-040 R7, R8). The Ascendant rows of `buffs` go to
    `buffs`; its chest-lock rows are discipline's (SPEC-141). `badges_earned`, `records` and
    `season_nodes` map as their §8 rows say; `season_targets` and `xp_price_changes` take the
    predecessor's per-month target keys and its multiplier setting (SPEC-138 §8), the multipliers
    clamped as the golden `import_xp_multipliers` reads them.
R10. Progression's `day_base_xp(rows, study_day)` sums a day's imported base XP leaving out the
    families the predecessor's day base leaves out (`readgoal:`, `leech:`, `focus`, `chest`, `2x:`,
    surprise, consistency, Ascendant), and equals the golden `import_day_base_xp`; SPEC-142's
    reconcile compares it per day with the source.
R11. Streaks, curriculum, habits, readings, markets and ingest map row for row as their §8 rows say:
    `coaching_kv`'s law due key only, to `law_dues` (the other keys are recomputed, SPEC-090,
    SPEC-091, SPEC-092); `leech_snapshot` not at all (SPEC-093 recomputes it); `skip_days` only the
    rows that were applied (SPEC-083); `preread_notes` to `readings`, `preread_run_events` to
    `reading_attempts`, and `preread_runs` to `reading_topic_days` and `reading_runs`.
R12. Focus maps `focus_timer`'s one row idle: a block running at the predecessor's stop arrives
    stopped, its elapsed time unrecorded, since no clock can prove it across the stop (owner question
    5's conservative answer). `focus_log` maps row for row.
R13. The vault maps `drill_xp_grants` to `drill_grades` row for row with each drill's XP, which it is
    handed by the composition as a map from drill id to XP that progression's
    `import::drill_xp(source)` reads from the predecessor's `drill:` ledger rows; the vault names no
    ledger.
R14. Every writer's counts reconcile with its source under its §8 rule, and a writer that cannot map a
    row (an unknown language code, a day string that is not a date, a value outside its table's
    check) refuses the whole write by the table, the column and the reason, never skipping it.
R15. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no
    fabricated history (a count the predecessor never read arrives NULL) and no unbounded faucet
    (the import mints nothing).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the count adds per field, `written` leaves `unchanged` out, and a port's pairs are named by source and target | `the_import_count_adds_and_leaves_unchanged_out` |
| A2 | over a DeckStreak day equal to the source, one differing, one missing, a DeckStreak-only day on the cutoff and one after it: unchanged, replaced, inserted, superseded and kept | `the_rollup_rows_follow_the_precedence` |
| A3 | a second write of the same source answers every analytics pair unchanged, and no row's `created_at` or `updated_at` is the clock's | `a_second_rollup_import_writes_nothing` |
| A4 | each stamp arrives as R7 maps it: live, recovered, void, absent, and an unstamped reading dropped and counted | `each_card_state_stamp_arrives_with_its_provenance` |
| A5 | the origin check refuses `backup` with a NULL source and `void` with a source, and accepts the three legal shapes | `the_origin_check_refuses_a_mismatched_origin` |
| A6 | the card-state reading equals the golden for every case | `the_card_state_reading_matches_the_predecessors_golden` |
| A7 | an imported day inside the window is re-rolled by the next recompute, keeps its card state, its origin and its `settled_at`, and is not settled again | `an_imported_day_is_re_rolled_and_keeps_its_card_state` |
| A8 | the ledger splits into grants and settled days, and the two sums equal the source's | `the_ledger_splits_into_grants_and_settled_days` |
| A9 | the day base XP equals the golden, each excluded family and an empty day among its cases | `the_day_base_xp_matches_the_predecessors_golden` |
| A10 | the multipliers equal the golden: missing, malformed, non-finite, below and above the clamp | `the_multipliers_match_the_predecessors_golden` |
| A11 | the progression tables follow the precedence, a second write writes nothing, and the Ascendant buffs alone are taken | `the_progression_rows_follow_the_precedence` |
| A12 | the streak rows follow the precedence, the singletons take the predecessor's row, and a second write writes nothing | `the_streak_rows_follow_the_precedence` |
| A13 | the curriculum rows follow the precedence, `law_dues` takes the one key, and a second write writes nothing | `the_curriculum_rows_follow_the_precedence` |
| A14 | the habit rows follow the precedence with course codes, and a second write writes nothing | `the_habit_rows_follow_the_precedence` |
| A15 | a running block arrives idle, the log maps row for row, and a second write writes nothing | `a_running_focus_block_arrives_idle` |
| A16 | only applied skip days are taken, with their card snapshots, and a second write writes nothing | `only_applied_skip_days_are_imported` |
| A17 | the reading rows map to their four tables, and a second write writes nothing | `the_reading_rows_map_to_their_four_tables` |
| A18 | a drill grade carries its ledger row's XP, and a grade with no ledger row refuses the write | `a_drill_grade_carries_its_ledger_xp` |
| A19 | the market positions map row for row, a DeckStreak-only position on the cutoff is kept and counted `kept`, and a second write writes nothing | `the_market_positions_follow_the_precedence` |
| A20 | a row a writer cannot map refuses the whole write by table, column and reason, and nothing is written | `an_unmappable_row_refuses_the_whole_write` |
| A21 | an imported history that crosses levels, badges and a record makes progression write no occasion row and call no port | `an_imported_history_raises_nothing` |

```acceptance
A1: cargo test -p deck-streak-kernel --test import -- --exact the_import_count_adds_and_leaves_unchanged_out
A2: cargo test -p deck-streak-analytics --test import -- --exact the_rollup_rows_follow_the_precedence
A3: cargo test -p deck-streak-analytics --test import -- --exact a_second_rollup_import_writes_nothing
A4: cargo test -p deck-streak-analytics --test import -- --exact each_card_state_stamp_arrives_with_its_provenance
A5: cargo test -p deck-streak-analytics --test import -- --exact the_origin_check_refuses_a_mismatched_origin
A6: cargo test -p deck-streak-analytics --test import -- --exact the_card_state_reading_matches_the_predecessors_golden
A7: cargo test -p deck-streak-coordination --test import_recompute -- --exact an_imported_day_is_re_rolled_and_keeps_its_card_state
A8: cargo test -p deck-streak-progression --test import -- --exact the_ledger_splits_into_grants_and_settled_days
A9: cargo test -p deck-streak-progression --test import -- --exact the_day_base_xp_matches_the_predecessors_golden
A10: cargo test -p deck-streak-progression --test import -- --exact the_multipliers_match_the_predecessors_golden
A11: cargo test -p deck-streak-progression --test import -- --exact the_progression_rows_follow_the_precedence
A12: cargo test -p deck-streak-streaks --test import -- --exact the_streak_rows_follow_the_precedence
A13: cargo test -p deck-streak-curriculum --test import -- --exact the_curriculum_rows_follow_the_precedence
A14: cargo test -p deck-streak-habits --test import -- --exact the_habit_rows_follow_the_precedence
A15: cargo test -p deck-streak-focus --test import -- --exact a_running_focus_block_arrives_idle
A16: cargo test -p deck-streak-ingest --test import -- --exact only_applied_skip_days_are_imported
A17: cargo test -p deck-streak-readings --test import -- --exact the_reading_rows_map_to_their_four_tables
A18: cargo test -p deck-streak-vault --test import -- --exact a_drill_grade_carries_its_ledger_xp
A19: cargo test -p deck-streak-markets --test import -- --exact the_market_positions_follow_the_precedence
A20: cargo test -p deck-streak-streaks --test import -- --exact an_unmappable_row_refuses_the_whole_write
A21: cargo test -p deck-streak-progression --test import -- --exact an_imported_history_raises_nothing
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/import.rs` | `deck-streak-kernel` | added: `SourceRow`, `SourceTables`, `ImportCount`, `ImportContext`, `ImportPort` |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the module |
| `crates/kernel/tests/import.rs` | `deck-streak-kernel` | added: A1 |
| `migrations/014001_analytics_card_state_origin.sql` | `deck-streak-analytics` | added: `card_state_origin` and its check |
| `crates/analytics/src/import.rs` | `deck-streak-analytics` | added: the rollup and per-language writers, the stamp map, `card_state_reading` |
| `crates/analytics/src/rollup.rs` | `deck-streak-analytics` | changed: the origin is read with its row and never written by the recompute |
| `crates/analytics/src/data_rights.rs` | `deck-streak-analytics` | changed: the export carries the origin |
| `crates/analytics/src/lib.rs` | `deck-streak-analytics` | changed: the module |
| `crates/analytics/tests/import.rs` | `deck-streak-analytics` | added: A2 to A6 |
| `crates/coordination/tests/import_recompute.rs` | `deck-streak-coordination` | added: A7 |
| `crates/progression/src/import.rs` | `deck-streak-progression` | added: the ledger split, `day_base_xp`, `drill_xp`, the buffs, badges, records, season and multiplier writers |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the module |
| `crates/progression/tests/import.rs` | `deck-streak-progression` | added: A8 to A11, A21 |
| `crates/progression/Cargo.toml` | `deck-streak-progression` | changed: serde_json's `float_roundtrip` as a dev-dependency feature, because the multiplier golden's floats are compared bit for bit |
| `crates/streaks/src/import.rs` | `deck-streak-streaks` | added: the four writers |
| `crates/streaks/src/lib.rs` | `deck-streak-streaks` | changed: the module |
| `crates/streaks/tests/import.rs` | `deck-streak-streaks` | added: A12, A20 |
| `crates/curriculum/src/import.rs` | `deck-streak-curriculum` | added: the five writers |
| `crates/curriculum/src/lib.rs` | `deck-streak-curriculum` | changed: the module |
| `crates/curriculum/tests/import.rs` | `deck-streak-curriculum` | added: A13 |
| `crates/habits/src/import.rs` | `deck-streak-habits` | added: the two writers |
| `crates/habits/src/lib.rs` | `deck-streak-habits` | changed: the module |
| `crates/habits/tests/import.rs` | `deck-streak-habits` | added: A14 |
| `crates/focus/src/import.rs` | `deck-streak-focus` | added: the two writers |
| `crates/focus/src/lib.rs` | `deck-streak-focus` | changed: the module |
| `crates/focus/tests/import.rs` | `deck-streak-focus` | added: A15 |
| `crates/ingest/src/import.rs` | `deck-streak-ingest` | added: the two skip writers |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/import.rs` | `deck-streak-ingest` | added: A16 |
| `crates/readings/src/import.rs` | `deck-streak-readings` | added: the four writers |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed: the module |
| `crates/readings/tests/import.rs` | `deck-streak-readings` | added: A17 |
| `crates/vault/src/import.rs` | `deck-streak-vault` | added: the drill-grade writer |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the module |
| `crates/vault/tests/import.rs` | `deck-streak-vault` | added: A18 |
| `crates/markets/src/import.rs` | `deck-streak-markets` | added: the positions writer |
| `crates/markets/src/lib.rs` | `deck-streak-markets` | changed: the module |
| `crates/markets/tests/import.rs` | `deck-streak-markets` | added: A19 |
| `crates/*/tests/fixtures/import/` | each owner | added: synthetic predecessor rows, invented values only |
| `tools/parity-oracle/migration/generate.py` | tools | added: the import goldens' generator, in the data-migration pack's golden shape (§7) |
| `tools/parity-oracle/migration/goldens/import_card_state_reading.json`, `import_day_base_xp.json`, `import_xp_multipliers.json` | tools | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the `daily_rollup` register row names the origin's migration |
| `scripts/mutation-rows.d/S14000-S14099.json` | repo | added: the rows of §9 |
| `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-140-each-context-imports-its-own-v9-rows-and-the-recovered-card-state-keeps-its-provenance.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/v9-import-run.md` | docs | added by the W8 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-140.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It writes no quest, chest, coin, fine, discipline, notification or publishing row, and splits no
  runtime setting of those contexts; SPEC-141 does (#61).
- It reads no predecessor file, verifies no snapshot, takes no backup and opens no transaction; the
  import tool and its runbook do (#61).
- It moves no side-by-side switch and stops nothing; the cutover checklist does after the owner's go
  (#62, #164).
- It re-runs no card-state recovery against the predecessor's backups; it carries what the
  predecessor recovered (#61).
- It writes no stats file into the vault (#153).
- It builds no screen for the card state's origin (#57).

## 6. Risks

- **A day counted twice** from DeckStreak's side-by-side rows beside the predecessor's. Prevented by
  R3's supersede on or before the cutoff, and detected by A2, A8 and row S14002.
- **A recovered count re-labelled as a live reading.** Prevented by R7's origin and its check, and
  detected by A4, A5 and rows S14004 and S14005.
- **A fabricated count carried as data.** Prevented by R7's void map, and detected by A4 and row
  S14006.
- **An imported day re-settled, or its card state overwritten**, by the first recompute. Prevented by
  `settled_at` and SPEC-071 R9, and detected by A7 and rows S14008 and S14009.
- **A second apply that changes a row** because an instant came from the clock. Prevented by R4, and
  detected by A3 and row S14003.
- **A celebration storm** after the import. Prevented by R5, and detected by A21 here and by
  SPEC-142's end-to-end test.

## 7. Parity goldens

Generated by `tools/parity-oracle/migration/generate.py` on the owner's checkout of the predecessor at
`27ee2bc`, in the data-migration pack's golden shape (the schema, the function, the source commit,
the generator and its digest, the inputs, the seed and the cases), in their own folder beside
SPEC-029's registry, whose house shape adds keys that shape refuses. Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `import_card_state_reading` | `pipeline_layers/base.py:PipelineBase._render_daily_from_rollup` | adapter | one synthetic rollup row per case: an absent stamp, a live stamp, a recovered stamp, a void stamp, an unknown prefix, and an unstamped row with counts; it records whether a snapshot and a learning term are rendered |
| `import_day_base_xp` | `database.py:GamifyStore.day_base_xp` | adapter | a temporary store with one ledger row of each excluded family, one of each counted source on the day and the day after, and an empty day |
| `import_xp_multipliers` | `exchange.py:get_multipliers` | adapter | a temporary store whose multiplier setting is missing, malformed JSON, a non-finite value, a value below the clamp, on each bound, and above it |

## 8. Tables and the v9 import

This SPEC adds one column, `card_state_origin` on `daily_rollup` (analytics,
`migrations/014001_analytics_card_state_origin.sql`), exported and erased with its row. Its
writers fill these tables; the rule is the count rule SPEC-142 reconciles.

| table | owner | from the predecessor's | rule |
|---|---|---|---|
| `daily_rollup` | analytics | `daily_rollup` | equal; card state by R7 |
| `daily_lang_stats` | analytics | `daily_lang_stats` | equal |
| `xp_ledger`, `xp_settlement` | progression | `xp_ledger` | sum over both |
| `buffs` | progression | `buffs`, the Ascendant kind | filtered (with discipline's `chest_locks`, a sum) |
| `badges_earned`, `records`, `season_nodes` | progression | the tables of those names | equal |
| `season_targets`, `xp_price_changes` | progression | `settings_kv`, the per-month target keys and the multiplier key | declared (SPEC-142 reconciles the whole split) |
| `streak_state`, `freeze_events`, `habit_strength`, `governor_state` | streaks | the tables of those names | equal |
| `language_progress`, `band_milestones`, `leech_remediation`, `can_do_unlocks` | curriculum | the tables of those names | equal |
| `law_dues` | curriculum | `coaching_kv`, the law due key | filtered: the other keys are recomputed |
| `minutes_log`, `writing_log` | habits | `reading_log`, `writing_log` | equal |
| `focus_log`, `focus_timer` | focus | the tables of those names | equal; the timer idle |
| `skip_days`, `skip_card_snapshot` | ingest | the tables of those names | filtered: applied rows only |
| `readings`, `reading_attempts` | readings | `preread_notes`, `preread_run_events` | equal |
| `reading_topic_days`, `reading_runs` | readings | `preread_runs` | declared: one topic day per run's last day, one run per row |
| `drill_grades` | vault | `drill_xp_grants` | equal |
| `market_positions` | markets | `market_positions` | equal |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S14001-UNCHANGED-EQUAL` | `crates/kernel/src/import.rs` | `written` leaves `unchanged` out | `import::the_import_count_adds_and_leaves_unchanged_out` |
| `S14002-SUPERSEDE-ON-CUTOFF` | `crates/analytics/src/import.rs` | a DeckStreak-only day on the cutoff is superseded, one after it kept | `import::the_rollup_rows_follow_the_precedence` |
| `S14003-SOURCE-INSTANT` | `crates/analytics/src/import.rs` | `created_at` comes from the source, not the clock | `import::a_second_rollup_import_writes_nothing` |
| `S14004-ORIGIN-CHECK` | `migrations/014001_analytics_card_state_origin.sql` | the origin's check (a script-mutation row) | `import::the_origin_check_refuses_a_mismatched_origin` |
| `S14005-BACKUP-ORIGIN` | `crates/analytics/src/import.rs` | a recovered stamp keeps origin `backup` | `import::each_card_state_stamp_arrives_with_its_provenance` |
| `S14006-VOID-NULLS` | `crates/analytics/src/import.rs` | a void stamp arrives with NULL counts | `import::each_card_state_stamp_arrives_with_its_provenance` |
| `S14007-UNSTAMPED-DROPPED` | `crates/analytics/src/import.rs` | an unstamped count arrives NULL | `import::each_card_state_stamp_arrives_with_its_provenance` |
| `S14008-FINGERPRINT-IMPORTED` | `crates/analytics/src/import.rs` | the fingerprint literal that forces the re-roll | `import_recompute::an_imported_day_is_re_rolled_and_keeps_its_card_state` |
| `S14009-SETTLED-AT` | `crates/analytics/src/import.rs` | an imported day arrives settled | `import_recompute::an_imported_day_is_re_rolled_and_keeps_its_card_state` |
| `S14010-DERIVED-SPLIT` | `crates/progression/src/import.rs` | a derived source goes to `xp_settlement` | `import::the_ledger_splits_into_grants_and_settled_days` |
| `S14011-BASE-EXCLUDED` | `crates/progression/src/import.rs` | an excluded family is left out of the day base; the golden names each | `import::the_day_base_xp_matches_the_predecessors_golden` |
| `S14012-CLAMP-LOW` | `crates/progression/src/import.rs` | the lower clamp; the golden names the bound and below it | `import::the_multipliers_match_the_predecessors_golden` |
| `S14013-CLAMP-HIGH` | `crates/progression/src/import.rs` | the upper clamp; the golden names the bound and above it | `import::the_multipliers_match_the_predecessors_golden` |
| `S14014-TIMER-IDLE` | `crates/focus/src/import.rs` | a running block arrives idle | `import::a_running_focus_block_arrives_idle` |
| `S14015-APPLIED-ONLY` | `crates/ingest/src/import.rs` | a skip never applied is not taken | `import::only_applied_skip_days_are_imported` |
| `S14016-DRILL-XP` | `crates/vault/src/import.rs` | a grade with no ledger XP refuses | `import::a_drill_grade_carries_its_ledger_xp` |
| `S14017-REFUSE-WHOLE` | `crates/streaks/src/import.rs` | an unmappable row refuses the write rather than skipping it | `import::an_unmappable_row_refuses_the_whole_write` |
| `S14018-ASCENDANT-ONLY` | `crates/progression/src/import.rs` | only the Ascendant kind of `buffs` is progression's | `import::the_progression_rows_follow_the_precedence` |
| `S14019-EVENT-KEPT` | `crates/markets/src/import.rs` | a DeckStreak-only row of an event table is kept, never superseded | `import::the_market_positions_follow_the_precedence` |

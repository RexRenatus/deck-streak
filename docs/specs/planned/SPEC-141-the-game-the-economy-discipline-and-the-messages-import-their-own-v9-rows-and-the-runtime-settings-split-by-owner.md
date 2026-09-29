# SPEC-141: the game, the economy, discipline and the messages import their own v9 rows, and the runtime settings split by owner

- **Wave:** W8. **Issue:** #61 (epic #9), its first and second criteria for the game and the
  messages. **Context(s):** the owners, each writing only its own tables through SPEC-140's import
  port: `deck-streak-quests` (`quests`, `quest_offers`, `weekly_quests`, `crown_days`, `ghosts`,
  `race_results`, `quest_state`, `chests`, `pity`, `xp_tokens`, `inventory`, `perfect_weeks`,
  `chest_settings`), `deck-streak-economy` (`coin_ledger`, `economy_state`, `penalty_ledger`),
  `deck-streak-discipline` (`tripwire_events`, `tripwire_state`, `sprints`, `chest_locks`,
  `discipline_state`, `committed_windows`, `window_events`, `hardmode_windows`, `wagers`,
  `contracts`, `contract_days`, `contract_changes`, `pardons`), `deck-streak-notifications`
  (`notification_decisions`, `notification_deliveries`, `notification_queue`,
  `notification_settings`, `nudge_holdout_arms`, `debrief_ratings`, `widget_messages`,
  `owner_last_message`) and `deck-streak-publishing` (`publishing_state`).
- **Decided by:** ADR-011 (side by side: one writer per contract at every moment), ADR-012 (the
  parity oracle proves the math), ADR-106 (the real-money rung is not built), ADR-140 (each owner
  writes its own imported rows, and the predecessor's record wins on a shared key) and ADR-143 (the
  checklist moves each side-by-side switch, one at a time, after the owner's go).
- **Prerequisites:** SPEC-021 and SPEC-041 (landed); SPEC-080, SPEC-081, SPEC-082, SPEC-084, SPEC-100,
  SPEC-101, SPEC-102, SPEC-103, SPEC-104, SPEC-105 and SPEC-106 (planned, unlanded); SPEC-130 and
  SPEC-137 (W7, planned, unlanded); and SPEC-140 (this wave, unlanded).
  **Mutation band:** `S14100-S14199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-141.md` (ADR-016).

## 1. The problem, measured

- **The game and the messages have no import writer.** SPEC-140 gives the study record's owners
  theirs. The quests, economy, discipline, notifications and publishing contexts own the 38 tables
  this SPEC fills and four it leaves empty (§8). The §8 rows of the SPECs that created them name
  the predecessor table each maps from, except the router's tables (SPEC-041 has no §8), whose
  mapping this SPEC decides.
- **One predecessor table feeds many owners.** The predecessor keeps its runtime settings in one
  key-value table, `settings_kv`. SPEC-130 R1 declares each DeckStreak setting beside its owner, and
  the §8 rows of SPEC-074, SPEC-080, SPEC-081, SPEC-082, SPEC-084, SPEC-102, SPEC-104, SPEC-105 and
  SPEC-138 each name the keys their tables take. The keys found in the predecessor's code at
  `27ee2bc` fall into four groups: the keys an owner takes (the chest cap and vault hour, the
  quest seed and ghost win streak, the per-week ghost keys, the per-month node targets and the
  multiplier setting (SPEC-140), the pass surcharge, the latest owner message, the discipline,
  hard-mode and tripwire keys, the celebration intensity, the quiet window, the landmark mark, the
  holdout's seed and percentage, the widget mood); three switches the cutover checklist moves
  (#62); the keys DeckStreak keeps for itself (the sync gate's two, the ingest base); and the keys
  of features DeckStreak does not carry (the keepsake art counters, the money rung's four, the
  tripwire's channel, the publishing baseline).
- **Three split tables.** `celebration_log` feeds the decision ledger and, for a held celebration,
  the queue's dice (SPEC-102); `notifications` feeds the delivery ledger and quests'
  `perfect_weeks` (SPEC-081); `buffs` feeds progression's Ascendant rows and discipline's
  `chest_locks` (SPEC-104).
- **A switch's meaning differs by key.** The predecessor reads `comeback_enabled` as on unless
  `"0"` (`pipeline_layers/nudges.py:NudgesLayer.run_morning_brief`), but `last_chance_enabled` and
  `widget_enabled` as on only when absent or `"1"` (`NudgesLayer.run_last_chance_nudge`,
  `pipeline_layers/showcase.py:ShowcaseLayer._update_widget`), and the widget's mood the same way
  (`ShowcaseLayer._widget_payload`). DeckStreak's router withholds a kind only at the value `"0"`
  (SPEC-041 R4's first rule, the policy's disable value), so a stored value other than `"0"` and
  `"1"` would turn on, in DeckStreak, a kind the owner had off: a move that copied the stored text
  would change the owner's choice.
- **The predecessor's delivery ledger is a latch.** Its `notifications` rows (a kind, a reference,
  whether the send succeeded) are written by `database.py:GamifyStore.mark_notified`, and
  `GamifyStore.was_notified` treats a row as sent whatever its success flag, so a failed send is
  never repeated. A delivery DeckStreak records is the same latch (SPEC-041 R5).
- **The messages are events.** Once a kind moves (#62), DeckStreak's delivery rows record what it
  sent; the import must keep them, or a dedupe key could send twice.

## 2. Requirements

The writers

R1. Quests, economy, discipline, notifications and publishing each implement SPEC-140's `ImportPort`
    in `crates/<context>/src/import.rs`, writing only their own tables, translating the
    predecessor's rows themselves, under SPEC-140 R3 and R4 (the predecessor's record wins on a
    shared key; a DeckStreak-only row is kept and counted `kept`; every instant from the source; a
    second write writes nothing). None of these tables is a day reading, so no row here is ever
    superseded.
R2. Quests maps `quests`, `quest_offers`, `weekly_quests`, `crown_days`, `race_results`, `chests`,
    `pity`, `xp_tokens` and `inventory` as their §8 rows say (a chest's challenge and weekly markers
    becoming the origins `challenge` and `weekly`); `ghosts` with each week's daily series and
    leader from the per-week keys `ghost_you_days:<week>` and `ghost_lead:<week>`; `quest_state`
    from the quest reintroduction seed and `ghost_win_streak`; `chest_settings` from
    `chests_per_day` and `chest_vault_hour`; and `perfect_weeks` from the weeks the predecessor's
    `notifications` ledger marks settled for a smoke bomb. `smoke_bomb_spends` takes nothing
    (SPEC-108).
R3. Economy maps `coin_ledger` and `penalty_ledger` row for row, and `economy_state` from the end of
    the active scroll pass (the predecessor's token row of the pass's kind) and
    `pass_surcharge_until`.
R4. Discipline maps `tripwire_events`, `sprints`, `committed_windows`, `window_events`,
    `hardmode_windows`, `wagers`, `contracts`, `contract_days`, `contract_changes` and `pardons` as
    their §8 rows say; `chest_locks` from the chest-lock rows of `buffs`; `tripwire_state` from
    `tripwire_scope`, `tw_snooze_day`, `freespin_day` and `freespin_ts`, never the bound source's
    channel (#170); and `discipline_state` from `discipline_disabled`, `panic_at`,
    `hardmode_enabled`, `hardmode_start_min` and `hardmode_dur_min`. The predecessor's
    `beeminder_posts` and its four money-rung keys map to nothing (ADR-106).
R5. Notifications maps:
    - `notifications` to `notification_deliveries`, one row per predecessor row whatever its
      success flag (the latch), through a kind map in its `import.rs` that names each kind the
      predecessor's code writes at `27ee2bc` once: a DeckStreak kind, with the dedupe scope the
      policy declares for it and the reference as the dedupe key; quests' perfect-week kind
      (R2); or not carried, with its reason (the money rung's kind, ADR-106);
    - `celebration_log` to `notification_decisions`, one per row (its event key as the dedupe key,
      its type as the kind, its tiers as `T0` to `T5`), the arm `defer` for a row still pending and
      `send` otherwise; and each pending row to one `notification_queue` row held, with its text and
      dice from its pending payload;
    - `nudge_ablation` to `nudge_holdout_arms`, `session_debrief` to `debrief_ratings`,
      `widget_state` to `widget_messages` and `last_owner_msg` to `owner_last_message`, as their §8
      rows say;
    - to `notification_settings`: `celebration_intensity`, `quiet_start_min`, `quiet_end_min` and
      `landmark_high_water` as stored; the holdout's seed and percentage as `holdout_seed` and
      `holdout_pct` (SPEC-100 R5), the predecessor's seed replacing one DeckStreak minted side by
      side so the draws continue its sequence (SPEC-100's mint stays `INSERT OR IGNORE`); and
      `widget_mood` as the predecessor's effective reading, `"1"` or `"0"`, equal to the golden
      `import_switch_widget_mood`.
R6. Publishing writes its `publishing_state` singleton with the switch off (SPEC-137 §8), so the owner
    switches it on anew; the predecessor's history baseline key maps to nothing (#348).

The settings

R7. Each owner declares, beside its writers, the key families it takes from `settings_kv` (an exact
    key, or a prefix ending in `:` for the per-week and per-month keys). A carried value is checked
    against the owner's own declaration (SPEC-130 R1); a value outside it arrives as the
    declaration's default and is named, by key and never by value, in the owner's count as `kept`
    at its default. SPEC-142's plan refuses a key that no family takes and no list leaves out.
R8. No writer writes a checklist-owned switch (ADR-143): the three the predecessor kept
    (`comeback_enabled`, `last_chance_enabled`, `widget_enabled`) and every other kind switch an
    earlier SPEC seeds `"0"` side by side (SPEC-049 R7, SPEC-100 R27, SPEC-101 R28, SPEC-102 R23).
    Notifications' `import::switch_values(source)` answers, for each of the three kept switches,
    the predecessor's own reading of its stored value as DeckStreak spells it (`"1"` on, `"0"`
    off), equal to the goldens `import_switch_comeback`, `import_switch_last_chance` and
    `import_switch_widget`. The checklist (SPEC-143) moves each switch,
    one at a time, to the value its item records.
R9. The messages are events (ADR-140): a DeckStreak delivery, decision, queue or widget row whose
    key (a delivery's kind, dedupe key and scope; a decision's kind and dedupe key; a widget's
    study day) the predecessor does not hold is kept on every day, and a shared key takes the
    predecessor's row, so a kind DeckStreak already sends keeps its dedupe latch.
R10. Every writer's counts reconcile under its §8 rule; the three split tables' counts are declared:
    `celebration_log`'s rows equal the decisions it writes, and its pending rows the queue rows;
    `notifications`' rows equal the deliveries, plus the perfect-week rows, plus the rows of a kind
    not carried (each counted by kind); `buffs`' rows equal progression's Ascendant rows plus
    discipline's chest locks. Each owner answers its key families (R7), so SPEC-142 reconciles
    `settings_kv` as a sum across the owners, the checklist's switches and the keys not carried.
R11. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no money
    at stake (the money rung maps to nothing) and no nag (an imported switch never turns a kind on
    that the owner had off).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the quest rows follow the precedence, a DeckStreak-only row is kept and counted, and a second write writes nothing | `the_quest_rows_follow_the_precedence` |
| A2 | the ghosts take their week's series and leader from the per-week keys, and the quest state its two keys | `the_ghosts_take_their_per_week_keys` |
| A3 | the chest markers become the two origins, and the chest settings take their two keys | `the_chest_markers_become_their_origins` |
| A4 | the perfect weeks are the distinct weeks the source ledger marks settled for a smoke bomb | `the_perfect_weeks_come_from_the_source_ledger` |
| A5 | the coin and penalty ledgers map row for row, the economy state takes the pass token's end and the surcharge key, and a second write writes nothing | `the_economy_rows_follow_the_precedence` |
| A6 | the discipline rows follow the precedence, the chest locks come from the chest-lock buffs alone, and a second write writes nothing | `the_discipline_rows_follow_the_precedence` |
| A7 | the tripwire state carries its four keys and never a channel, and the money rung's rows and keys map to nothing | `the_tripwire_state_carries_no_channel` |
| A8 | the deliveries (a failed send included), decisions, queue rows with their dice, arms, ratings, widget messages and latest owner message map as R5 says, and the seed and percentage take their DeckStreak keys | `the_message_rows_map_to_their_tables` |
| A9 | a DeckStreak delivery on a day before the cutoff is kept, and a shared dedupe key takes the predecessor's row | `a_sent_message_is_kept_across_the_import` |
| A10 | no writer writes a checklist-owned switch, whatever the source holds | `no_checklist_switch_is_written` |
| A11 | the switch values and the widget mood equal the four goldens for each stored value | `the_switch_values_match_the_predecessors_golden` |
| A12 | a setting outside its declaration arrives at its default and is counted by key | `a_setting_outside_its_declaration_arrives_at_its_default` |
| A13 | the publishing state is written with its switch off and no baseline | `the_publishing_state_arrives_switched_off` |
| A14 | discipline's count of the buffs names the chest-lock rows it wrote and no other kind | `the_chest_lock_count_is_disciplines_share` |
| A15 | the kind map names each ledger kind of the predecessor's code once, and a ledger row of a kind it does not name refuses the write by table, column and reason | `every_ledger_kind_is_mapped_once` |
| A16 | the log's rows equal the decisions and its pending rows the queue rows, and the ledger's rows equal the deliveries plus the perfect-week and not-carried rows by kind | `the_message_counts_reconcile` |

```acceptance
A1: cargo test -p deck-streak-quests --test import -- --exact the_quest_rows_follow_the_precedence
A2: cargo test -p deck-streak-quests --test import -- --exact the_ghosts_take_their_per_week_keys
A3: cargo test -p deck-streak-quests --test import -- --exact the_chest_markers_become_their_origins
A4: cargo test -p deck-streak-quests --test import -- --exact the_perfect_weeks_come_from_the_source_ledger
A5: cargo test -p deck-streak-economy --test import -- --exact the_economy_rows_follow_the_precedence
A6: cargo test -p deck-streak-discipline --test import -- --exact the_discipline_rows_follow_the_precedence
A7: cargo test -p deck-streak-discipline --test import -- --exact the_tripwire_state_carries_no_channel
A8: cargo test -p deck-streak-notifications --test import -- --exact the_message_rows_map_to_their_tables
A9: cargo test -p deck-streak-notifications --test import -- --exact a_sent_message_is_kept_across_the_import
A10: cargo test -p deck-streak-notifications --test import -- --exact no_checklist_switch_is_written
A11: cargo test -p deck-streak-notifications --test import -- --exact the_switch_values_match_the_predecessors_golden
A12: cargo test -p deck-streak-notifications --test import -- --exact a_setting_outside_its_declaration_arrives_at_its_default
A13: cargo test -p deck-streak-publishing --test import -- --exact the_publishing_state_arrives_switched_off
A14: cargo test -p deck-streak-discipline --test import -- --exact the_chest_lock_count_is_disciplines_share
A15: cargo test -p deck-streak-notifications --test import -- --exact every_ledger_kind_is_mapped_once
A16: cargo test -p deck-streak-notifications --test import -- --exact the_message_counts_reconcile
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/quests/src/import.rs` | `deck-streak-quests` | added: the thirteen writers and the key families |
| `crates/quests/src/lib.rs` | `deck-streak-quests` | changed: the module |
| `crates/quests/tests/import.rs` | `deck-streak-quests` | added: A1 to A4 |
| `crates/economy/src/import.rs` | `deck-streak-economy` | added: the three writers and the key family |
| `crates/economy/src/lib.rs` | `deck-streak-economy` | changed: the module |
| `crates/economy/tests/import.rs` | `deck-streak-economy` | added: A5 |
| `crates/discipline/src/import.rs` | `deck-streak-discipline` | added: the thirteen writers and the key families |
| `crates/discipline/src/lib.rs` | `deck-streak-discipline` | changed: the module |
| `crates/discipline/tests/import.rs` | `deck-streak-discipline` | added: A6, A7, A14 |
| `crates/notifications/src/import.rs` | `deck-streak-notifications` | added: the writers, the kind map, the key families and `switch_values` |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the module |
| `crates/notifications/tests/import.rs` | `deck-streak-notifications` | added: A8 to A12, A15, A16 |
| `crates/publishing/src/import.rs` | `deck-streak-publishing` | added: the state writer |
| `crates/publishing/src/lib.rs` | `deck-streak-publishing` | changed: the module |
| `crates/publishing/tests/import.rs` | `deck-streak-publishing` | added: A13 |
| `crates/*/tests/fixtures/import/` | each owner | added: synthetic predecessor rows and settings, invented values only |
| `tools/parity-oracle/migration/generate.py` | tools | changed: the four switch goldens |
| `tools/parity-oracle/migration/goldens/import_switch_comeback.json`, `import_switch_last_chance.json`, `import_switch_widget.json`, `import_switch_widget_mood.json` | tools | added |
| `scripts/mutation-rows.d/S14100-S14199.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-141-the-game-the-economy-discipline-and-the-messages-import-their-own-v9-rows-and-the-runtime-settings-split-by-owner.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/v9-import-run.md` | docs | added by the W8 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-141.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It moves no side-by-side switch; the checklist does, one at a time, after the owner's go (#62,
  #164).
- It carries no keepsake the predecessor drew and none of its image counters (#125).
- It carries no bound source's channel for the tripwire (#170).
- It carries no money-rung pledge or key (#116).
- It carries no publishing baseline and publishes nothing (#348, #156).
- It builds no screen for an imported setting (#57).

## 6. Risks

- **A kind the owner had off switched on** by a copied value. Prevented by R8 (nothing writes a
  switch; the move takes the predecessor's effective value), and detected by A10, A11 and rows
  S14108 and S14109.
- **A second send** after a DeckStreak delivery is superseded. Prevented by R9, and detected by A9
  and row S14107.
- **A chest lock counted as a buff**, or both. Prevented by R4's kind filter, and detected by A6, A14
  and row S14104; the sum across the two owners is SPEC-142's reconcile.
- **A stray setting blocking the import.** Prevented by R7's default, and detected by A12.
- **A ledger kind silently dropped.** Prevented by R5's kind map and SPEC-140 R14's refusal, and
  detected by A15, A16 and row S14114.
- **A channel carried into DeckStreak's configuration.** Prevented by R4, and detected by A7.

## 7. Parity goldens

Generated by `tools/parity-oracle/migration/generate.py` (SPEC-140) on the owner's checkout of the
predecessor at `27ee2bc`, in the data-migration pack's golden shape. Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `import_switch_comeback` | `pipeline_layers/nudges.py:NudgesLayer.run_morning_brief` | adapter | a stub store inside a lapse per case, `comeback_enabled` absent, `"1"`, `"0"`, an empty string and another word, every step after the gate stubbed, recording whether the comeback branch ran |
| `import_switch_last_chance` | `pipeline_layers/nudges.py:NudgesLayer.run_last_chance_nudge` | adapter | the same five cases for `last_chance_enabled`, recording whether the gate let the nudge through |
| `import_switch_widget` | `pipeline_layers/showcase.py:ShowcaseLayer._update_widget` | adapter | the same five cases for `widget_enabled`, recording whether the widget was refreshed |
| `import_switch_widget_mood` | `pipeline_layers/showcase.py:ShowcaseLayer._widget_payload` | adapter | the same five cases for `widget_mood`, recording whether the payload carries a mood |

## 8. Tables and the v9 import

This SPEC adds no table. Its writers fill these; the rule is the count rule SPEC-142 reconciles.

| table | owner | from the predecessor's | rule |
|---|---|---|---|
| `quests`, `quest_offers`, `weekly_quests`, `crown_days`, `race_results`, `chests`, `pity`, `xp_tokens`, `inventory` | quests | the tables of those names | equal |
| `ghosts` | quests | `ghosts`, and the per-week `settings_kv` keys | equal |
| `quest_state`, `chest_settings` | quests | `settings_kv` | declared (the settings split) |
| `perfect_weeks` | quests | `notifications`, the smoke-bomb settled rows | declared: one per distinct week |
| `smoke_bomb_spends` | quests | nothing (SPEC-108) | not carried: the predecessor spent none |
| `coin_ledger`, `penalty_ledger` | economy | the tables of those names | equal |
| `economy_state` | economy | the pass token's end and `settings_kv` | declared |
| `tripwire_events`, `sprints`, `committed_windows`, `window_events`, `hardmode_windows`, `wagers`, `contracts`, `contract_days`, `contract_changes`, `pardons` | discipline | the tables of those names | equal |
| `chest_locks` | discipline | `buffs`, the chest-lock kind | sum with progression's `buffs` |
| `tripwire_state` | discipline | `tripwire_state` and `settings_kv` | equal |
| `discipline_state` | discipline | `settings_kv` | declared |
| `notification_deliveries` | notifications | `notifications` | declared, with `perfect_weeks` |
| `notification_decisions`, `notification_queue` | notifications | `celebration_log` | declared |
| `nudge_holdout_arms`, `debrief_ratings`, `widget_messages` | notifications | `nudge_ablation`, `session_debrief`, `widget_state` | equal |
| `owner_last_message`, `notification_settings` | notifications | `settings_kv` | declared |
| `nudge_snoozes`, `in_app_feed` | notifications | nothing | not carried: the predecessor kept no snooze and no feed |
| `publishing_state` | publishing | nothing | written off (SPEC-137 §8) |
| `published_files` | publishing | nothing | not carried: the predecessor's pages depart with its repository (#348) |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S14101-QUEST-KEPT` | `crates/quests/src/import.rs` | a DeckStreak-only quest row is kept and counted | `import::the_quest_rows_follow_the_precedence` |
| `S14102-GHOST-WEEK-KEY` | `crates/quests/src/import.rs` | the per-week key's prefix | `import::the_ghosts_take_their_per_week_keys` |
| `S14103-CHEST-ORIGIN` | `crates/quests/src/import.rs` | the weekly marker becomes the `weekly` origin | `import::the_chest_markers_become_their_origins` |
| `S14104-CHEST-LOCK-KIND` | `crates/discipline/src/import.rs` | only the chest-lock kind is discipline's | `import::the_discipline_rows_follow_the_precedence` |
| `S14105-PASS-KIND` | `crates/economy/src/import.rs` | the pass's end comes from the pass kind's token | `import::the_economy_rows_follow_the_precedence` |
| `S14106-NO-CHANNEL` | `crates/discipline/src/import.rs` | the channel key is never carried | `import::the_tripwire_state_carries_no_channel` |
| `S14107-MESSAGE-KEPT` | `crates/notifications/src/import.rs` | a DeckStreak delivery is kept on every day | `import::a_sent_message_is_kept_across_the_import` |
| `S14108-NO-SWITCH-WRITE` | `crates/notifications/src/import.rs` | a checklist-owned switch is skipped | `import::no_checklist_switch_is_written` |
| `S14109-COMEBACK-ZERO` | `crates/notifications/src/import.rs` | the comeback switch is off only at `"0"`; the golden names the empty string and another word | `import::the_switch_values_match_the_predecessors_golden` |
| `S14110-WIDGET-ONE` | `crates/notifications/src/import.rs` | the widget switch is on only when absent or `"1"`; the golden names another word | `import::the_switch_values_match_the_predecessors_golden` |
| `S14111-DEFAULT-ON-INVALID` | `crates/notifications/src/import.rs` | a value outside its declaration arrives at its default | `import::a_setting_outside_its_declaration_arrives_at_its_default` |
| `S14112-PUBLISH-OFF` | `crates/publishing/src/import.rs` | the publishing switch arrives off | `import::the_publishing_state_arrives_switched_off` |
| `S14113-PERFECT-DISTINCT` | `crates/quests/src/import.rs` | one perfect week per distinct week | `import::the_perfect_weeks_come_from_the_source_ledger` |
| `S14114-UNKNOWN-KIND` | `crates/notifications/src/import.rs` | an unnamed ledger kind refuses rather than being skipped | `import::every_ledger_kind_is_mapped_once` |
| `S14115-LATCH` | `crates/notifications/src/import.rs` | a failed send is still a delivery | `import::the_message_rows_map_to_their_tables` |
| `S14116-PENDING-DEFER` | `crates/notifications/src/import.rs` | a pending celebration is a `defer` decision with a queue row | `import::the_message_counts_reconcile` |

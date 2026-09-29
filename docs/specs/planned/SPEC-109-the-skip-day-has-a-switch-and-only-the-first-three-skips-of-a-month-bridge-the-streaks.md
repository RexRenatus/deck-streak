# SPEC-109: the skip day has a switch, and only the first three skips of a month bridge the streaks

- **Wave:** W5. **Issue:** #280 (epic #6). **Context(s):** `deck-streak-ingest` (the switch, its
  table and setter, and the take's refusal); `deck-streak-coordination` (the bridged set, the
  preview's two answers, and the steps that read the bridged set); `deck-streak-economy` (the cap,
  read from `economy.json`); `deck-streak-notifications` (the stakes preview's rows);
  `deck-streak-api`, `deck-streak-bot` and the Mini App (`web/app`).
- **Decided by:** ADR-089 (the skip day's take and its exact undo), ADR-071 (each study day is
  settled once, in order, so a changed set changes only what later recomputes read), ADR-012 (the
  parity oracle proves every number) and ADR-016 (a planned SPEC is promoted by its delivery).
- **Prerequisites:** SPEC-083 (the skip record, the take, the preview, the tariff, the undo and the
  skip set), SPEC-076 (both streaks, the governor's silent run and the streak view), SPEC-072 (the
  consistency run and Ascendant), SPEC-100 (the stakes preview and `nu:skip`), SPEC-082 (the
  wallet and its coin ledger), SPEC-071 (the fold), SPEC-022 (the recording layer), SPEC-024 (the
  owner's session), SPEC-026 (the bot's owner gate), SPEC-028 (the Mini App shell), SPEC-021 (the
  six files of a table), SPEC-020 (migrations), SPEC-029 (the goldens) and SPEC-038 (its section 8
  ruling (i), under which SPEC-083 is amended). **Mutation band:** `S10900-S10999`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-109.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` af0693f, `crates/ingest/src/` holds no skip module, and no context
  holds a runtime setting for the skip day. SPEC-083 (planned) records a skip, previews, takes and
  undoes it, prices it by the tariff, and serves the skip set every recompute step reads; its
  section 1 and section 5 leave the switch and the cap unenforced for #280. `economy.json` declares
  `streak.skip_bridge_monthly_cap` (3), which no code reads. The kernel's
  `Db::bump_settings_generation` bumps the settings generation inside a caller's write, and the
  CONTEXT-MAP moves each runtime setting to the context that reads it.
- **What is ported** (at `27ee2bc`): the switch's default, on (`config.py:Settings.skip_enabled`),
  and the cap's value, 3 (`constants.SKIP_BRIDGE_MONTHLY_CAP`, in SPEC-083's golden
  `skip.constants`). Nothing else: the predecessor reads the switch and never checks it, and
  defines the cap and never enforces it. `pipeline_layers/skip.py:SkipDaysLayer.take_skip_day`
  takes a skip with no check of either, and `skip.py:real_misses` bridges every skip day it is
  given. The rules are new, by the owner's decision at #269, so no golden can prove them: this
  SPEC's tests state them.
- **The outcome sets.** A take: `skip_disabled` while the switch is off, before every outcome of
  SPEC-083. A skip day, at a recompute: `bridged` (among the first 3 skip days of its calendar
  month, in study-day order) or `past the cap` (every later one). The preview: `off`, or on with
  `bridges` true or false.
- **Traps a hand port falls into.**
  - The switch read before the take's write lets a switch turned off between the two take a skip:
    it is read inside the write that checks `already_skipped`.
  - Moving every reader to the bridged set fines a declared day off and fails its quests. Only the
    rules that bridge a missed day read it (R6); a skip past the cap is still a skip day to the
    rest.
  - The tariff and the cap counted in two months disagree at a month's edge: one function names a
    study day's month for both (R5).
  - The take's outcome copied from the predecessor says the streak is kept, which is false past
    the cap (R8).
- **Deviations.**
  - The switch is a stored runtime setting, not configuration read at start, so the settings
    screen can set it (#57); its default is the predecessor's.
  - The predecessor bridges every skip day; here a skip day past the cap bridges none (#269).

## 2. Requirements

The switch (#280)

R1. `deck-streak-ingest` owns `skip_settings` (`migrations/010901_ingest_skip_settings.sql`,
    `STRICT`, `created_at`): one row, seeded with `enabled` set to 1 (the switch on, the default of
    `config.py:Settings.skip_enabled`). `CHECK` constraints hold it to one row and `enabled` to 0
    or 1.
R2. Ingest's setter writes the switch inside the kernel's `BEGIN IMMEDIATE` write and, when the
    value changes, bumps the settings generation in the same write (`Db::bump_settings_generation`,
    the LEXICON's rule for a runtime setting). Writing the value the row already holds writes
    nothing and bumps nothing. No surface calls the setter in this wave: the settings screen shows
    and sets it (#57).
R3. A take reads the switch inside the write that checks `already_skipped` (SPEC-083 R2), and before
    that check. While the switch is off, the take is refused with `skip_disabled`: it writes no
    row, sends no request and reaches no purchase, so nothing is recorded, charged or written. The
    switch gates the take only: the undo and its refund, the summary and the skip set (SPEC-083
    R4, R5, R6 and R9) answer as they do with it on.
R4. While the switch is off, no surface offers a skip. Coordination's preview (SPEC-083 R7) answers
    `off` and reads nothing else: no due count, card list or tariff. The surfaces answer it
    (R12 to R14), and the stakes preview leaves out its Cheat day row (`nu:skip`, SPEC-100 R15)
    and keeps its other rows. A confirm that arrives after the switch went off meets R3's refusal.

The bridge cap (#280)

R5. The bridged set is, for each calendar month, the first `streak.skip_bridge_monthly_cap` study
    days of the skip set (SPEC-083 R4: the study days holding an `applied` skip not undone), in
    study-day order. A study day's calendar month is the proleptic Gregorian year and month of the
    kernel's `StudyDay`, named by one function in `crates/coordination/src/skip/days.rs` that the
    tariff's count (SPEC-083 R8) calls too. The skip-set port answers both sets.
R6. The rules that bridge a missed day read the bridged set, and a skip day outside it counts as a
    missed day in each: the language streak (`skip.py:real_misses` and the gap rules of SPEC-076),
    the law streak (`analytics.py:bridged_streak`, SPEC-076 R11), the governor's silent run
    (SPEC-076 R15), the consistency run (`GovernorLayer._on_pace_run` over
    `gamification/governor.py:tier_down_run`, SPEC-072), and the streak view's skip markers
    (SPEC-076 R20), so a skip past the cap shows as the miss the streak counts. Every other reader
    keeps the skip set: the day's quests are voided, no chest is rolled and a race week counts it
    (SPEC-083 R12), Ascendant never arms on it (SPEC-072), no fine is booked on it (SPEC-104 R12),
    no window is judged on it (SPEC-105), and the nudges decline on it (SPEC-100).
R7. A skip past the cap is still taken as SPEC-083 specifies and priced by its tariff (SPEC-083
    R8, the ladder's last price repeating); the cap changes what the next recompute reads, never
    the record. An undo takes its skip out of the skip set (SPEC-083 R4, R5), so the month's next
    skip joins the bridged set, and the next recompute reads it bridged. Transitions already
    settled stay as they were (SPEC-083 R13, ADR-071).
R8. The preview says, before the confirm, whether the skip will bridge: `bridges` is false when the
    current study day's calendar month already holds the cap's number of bridged skips. The bot and
    the Mini App then say, above the Confirm, that this skip is taken and priced but bridges no
    streak and counts as a missed day, naming the month's cap from the preview. The take's outcome
    on such a skip never says that a streak is kept.
R9. The cap is read from `economy.json`'s `streak.skip_bridge_monthly_cap`, which equals the golden
    constant `constants.SKIP_BRIDGE_MONTHLY_CAP` (3, `goldens/skip.constants.json`), by the reader
    SPEC-083 R10 builds for the tariff, once at start. The reader takes the text it parses, so a
    test hands it a copy. No cap number is typed in code: a copy holding 2 bridges two skips a
    month.

Rights and the amendment

R10. `skip_settings` is declared in ingest's data-rights port as exported, and reset in place on
    erase (the switch on again, one row kept). It joins the `skip-days` category in `privacy.json`
    and that category's `PRIVACY.md` line, the register of DeckStreak's own tables gains its row,
    and coordination's symmetry test seeds it (SPEC-021's six files).
R11. SPEC-083 is amended insert-only (SPEC-038 section 8, ruling (i)): a line after its R2 naming
    R3's refusal, which comes first; a line after its R11 saying that its bridges read the bridged
    set of R5 and R6; and a dated amendment section naming each insertion. None of its criteria is
    retired: with 3 skips or fewer in a month, the bridged set is the skip set.

Surfaces

R12. The API's `GET /api/skip/preview` carries `enabled` and, while it is true, `bridges`; while the
    switch is off, `POST /api/skip` answers 409 and records nothing. Both answer the owner's
    session only (SPEC-024).
R13. The bot's `/skip`, `/cheat` and `nu:skip` answer, while the switch is off, that the skip day is
    switched off, with no Confirm button; `/skip` stays in the owner's menu. `/skipstats` then names
    no way to take one. With the switch on, the preview's answer gains R8's line when `bridges` is
    false, and the take's outcome gains the line that this skip bridges no streak.
R14. The Mini App's `/skip` route shows, while the switch is off, that the skip day is switched off
    and no confirm; with it on, R8's notice above the confirm when `bridges` is false. Its strings
    live in each locale's catalog, and none threatens a loss beyond the fact that the day counts as
    missed.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | with the switch off, a take on a free study day and a take on a day already skipped are both refused with `skip_disabled` within 5 seconds, writing no skip row and recording no request through the recording layer | `a_take_while_the_switch_is_off_is_refused_first_writing_and_sending_nothing` |
| A2 | the seeded row is on; turning it off bumps the settings generation by one in the same write, writing off again bumps nothing, and a raw insert of a second row or of an `enabled` outside 0 and 1 is refused | `the_switch_starts_on_and_a_change_bumps_the_settings_generation_once` |
| A3 | the export holds the switch's row, and an erase leaves one row, on | `the_switch_is_exported_and_reset_on_erase` |
| A4 | the cap read from `economy.json` is 3, equal to the golden constant, and read from a copy's text holding 2 it is 2 | `the_bridge_cap_is_read_from_economy_json_and_equals_the_golden` |
| A5 | with the switch off, coordination's take over ingest's record and write, pointed at the recording layer, and economy's wallet is refused within 5 seconds and leaves no skip row, no coin ledger row and no recorded request | `a_take_while_the_switch_is_off_writes_no_skip_no_coins_and_no_request` |
| A6 | with the switch off, the preview answers `off` and holds no due count, card list or tariff | `the_preview_offers_no_skip_while_the_switch_is_off` |
| A7 | with the switch off, the undo of an applied skip is accepted and refunds what it paid, and the summary and the skip set are as with the switch on | `the_undo_and_the_summary_answer_while_the_switch_is_off` |
| A8 | four skips taken in one calendar month are each applied and charged the tariff's price; at the next recompute the first three bridge both streaks with no freeze consumed, and the 4th is a missed day for both streaks, the governor's silent run and the consistency run | `the_first_three_skips_of_a_month_bridge_and_the_fourth_is_a_missed_day` |
| A9 | three applied skips, the 3rd undone, then a later skip in the same month: it bridges at the next recompute | `an_undone_skip_frees_its_bridge_for_a_later_skip` |
| A10 | three skips on the last study days of a month and one on the first study day of the next: all four bridge, and the tariff prices the 4th as its month's first | `the_cap_counts_each_calendar_month_apart` |
| A11 | the preview's `bridges` is true with 2 bridged skips in the month and false with 3, and the price is the tariff's in both | `the_preview_says_when_a_skip_will_not_bridge` |
| A12 | over a cap read as 2 from a copy, the 3rd applied skip of a month does not bridge | `a_cap_read_as_two_bridges_two_skips_a_month` |
| A13 | a 4th skip of a month stays in the skip set and leaves the bridged set, and the streak view marks its day a miss | `the_skip_set_keeps_a_skip_past_the_cap_and_the_view_marks_it_a_miss` |
| A14 | no file under `crates/*/src` reads the bridged set but its port, the streak step, the day-bonus step, the streak view and the preview | `only_the_bridge_rules_read_the_bridged_set` |
| A15 | with the skip day off, the stakes preview has no Cheat day row and its other rows and lines equal the golden's case | `the_stakes_preview_offers_no_cheat_day_while_the_skip_day_is_off` |
| A16 | with the switch off, `/skip`, `/cheat` and `nu:skip` answer that the skip day is off with no Confirm, `/skip` stays in the owner's menu, and `/skipstats` offers no take | `skip_cheat_and_nu_skip_offer_no_skip_while_the_switch_is_off` |
| A17 | the `/skip` answer carries the will-not-bridge line when `bridges` is false and not when it is true, and the outcome of a take past the cap says it bridges no streak and never that a streak is kept | `skip_says_before_the_confirm_when_a_skip_will_not_bridge` |
| A18 | to the owner's session, the preview route carries `enabled` false while the switch is off and `bridges` false past the cap, the take route answers 409 while off and records no skip, and neither answers another session | `the_skip_routes_carry_the_switch_and_the_bridge` |
| A19 | the skip sheet shows the off notice with no confirm, and the will-not-bridge notice above the confirm when `bridges` is false | `says when a skip will not bridge, and offers no skip while the skip day is off` |

```acceptance
A1: cargo test -p deck-streak-ingest --test skip_switch -- --exact a_take_while_the_switch_is_off_is_refused_first_writing_and_sending_nothing
A2: cargo test -p deck-streak-ingest --test skip_switch -- --exact the_switch_starts_on_and_a_change_bumps_the_settings_generation_once
A3: cargo test -p deck-streak-ingest --test skip_switch -- --exact the_switch_is_exported_and_reset_on_erase
A4: cargo test -p deck-streak-economy --test skip_bridge_cap -- --exact the_bridge_cap_is_read_from_economy_json_and_equals_the_golden
A5: cargo test -p deck-streak-coordination --test skip_switch_flow -- --exact a_take_while_the_switch_is_off_writes_no_skip_no_coins_and_no_request
A6: cargo test -p deck-streak-coordination --test skip_switch_flow -- --exact the_preview_offers_no_skip_while_the_switch_is_off
A7: cargo test -p deck-streak-coordination --test skip_switch_flow -- --exact the_undo_and_the_summary_answer_while_the_switch_is_off
A8: cargo test -p deck-streak-coordination --test skip_bridges -- --exact the_first_three_skips_of_a_month_bridge_and_the_fourth_is_a_missed_day
A9: cargo test -p deck-streak-coordination --test skip_bridges -- --exact an_undone_skip_frees_its_bridge_for_a_later_skip
A10: cargo test -p deck-streak-coordination --test skip_bridges -- --exact the_cap_counts_each_calendar_month_apart
A11: cargo test -p deck-streak-coordination --test skip_bridges -- --exact the_preview_says_when_a_skip_will_not_bridge
A12: cargo test -p deck-streak-coordination --test skip_bridges -- --exact a_cap_read_as_two_bridges_two_skips_a_month
A13: cargo test -p deck-streak-coordination --test skip_bridges -- --exact the_skip_set_keeps_a_skip_past_the_cap_and_the_view_marks_it_a_miss
A14: cargo test -p deck-streak-coordination --test skip_bridges -- --exact only_the_bridge_rules_read_the_bridged_set
A15: cargo test -p deck-streak-notifications --test stakes_skip_switch -- --exact the_stakes_preview_offers_no_cheat_day_while_the_skip_day_is_off
A16: cargo test -p deck-streak-bot --test skip_switch_commands -- --exact skip_cheat_and_nu_skip_offer_no_skip_while_the_switch_is_off
A17: cargo test -p deck-streak-bot --test skip_switch_commands -- --exact skip_says_before_the_confirm_when_a_skip_will_not_bridge
A18: cargo test -p deck-streak-api --test skip_switch_routes -- --exact the_skip_routes_carry_the_switch_and_the_bridge
A19: pnpm exec vitest run web/app/src/lib/skip/skip-bridge.test.ts -t "says when a skip will not bridge, and offers no skip while the skip day is off"
```

A5's test includes the recording layer from `crates/ingest/tests/support/recording.rs` by path, so
the one recording layer SPEC-022 built is shared, not copied. The layer relays to an address where
nothing listens; the refused take sends no request, so nothing is ever relayed.

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, game-economy, ux-laws and
accessibility packs stay enforced, and no row is deferred or lifted for this delivery, so the
private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | `skip_settings` is declared under the `skip-days` category with its data, purpose, basis, retention and erase mode (reset in place), over `privacy.json`, `PRIVACY.md`, `migrations/010901_ingest_skip_settings.sql` and `crates/ingest/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the economy stays equal to its reference, over `economy.json`, examining `streak.skip_bridge_monthly_cap` (3), now read by the code, and the skip tariff | the game-economy pack |
| B3 | the off notice and the will-not-bridge notice shame no choice, carry no false urgency and threaten no loss beyond the missed day, over the skip strings in `web/app/messages/*.json`, every file under `web/app/src/lib/skip/`, and `crates/bot/src/skip_commands.rs` | the ux-laws pack |
| B4 | the `/skip` route with the off notice and with the will-not-bridge notice, in both colour schemes, meets WCAG 2.2 AA, over `web/app/src/routes/skip/+page.svelte` and `web/app/src/lib/skip/SkipSheet.svelte` | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/skip.rs` | `deck-streak-ingest` | changed: the switch's read inside the take's first write, before `already_skipped` (R3); the setter and its bump (R2) |
| `crates/ingest/src/data_rights.rs` | `deck-streak-ingest` | changed: `skip_settings`, exported and reset in place (R10) |
| `migrations/010901_ingest_skip_settings.sql` | `deck-streak-ingest` | added: the table, `STRICT`, one seeded row, on (R1) |
| `crates/ingest/tests/skip_switch.rs` | `deck-streak-ingest` | added: A1 to A3 |
| `crates/economy/src/tariff.rs` | `deck-streak-economy` | changed: the reader takes the text it parses and answers the cap beside the ladder (R9) |
| `crates/economy/tests/skip_bridge_cap.rs` | `deck-streak-economy` | added: A4 |
| `crates/coordination/src/skip/days.rs` | `deck-streak-coordination` | changed: the bridged set beside the skip set, and the month of a study day (R5) |
| `crates/coordination/src/skip/mod.rs` | `deck-streak-coordination` | changed: the preview's `off` and `bridges`, and the tariff's count through the month function (R4, R5, R8) |
| `crates/coordination/src/recompute/streaks.rs` | `deck-streak-coordination` | changed: both streaks and the silent run read the bridged set (R6) |
| `crates/coordination/src/recompute/day_bonuses.rs` | `deck-streak-coordination` | changed: the consistency run reads the bridged set, and Ascendant keeps the skip set (R6) |
| `crates/coordination/src/streak_views.rs` | `deck-streak-coordination` | changed: the skip markers read the bridged set (R6) |
| `crates/coordination/src/nudges/stakes.rs` | `deck-streak-coordination` | changed: the switch passed to the stakes preview (R4) |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | unchanged: ingest's port is registered already (SPEC-021); listed under SPEC-021's six-file rule |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `skip_settings` row |
| `crates/coordination/tests/skip_switch_flow.rs` | `deck-streak-coordination` | added: A5 to A7 |
| `crates/coordination/tests/skip_bridges.rs` | `deck-streak-coordination` | added: A8 to A14 |
| `crates/coordination/Cargo.toml` | `deck-streak-coordination` | changed: `zstd` (the workspace's) and tokio's `net` and `io-util` as dev-dependencies, for the recording layer A5 includes |
| `crates/notifications/src/nudges.rs` | `deck-streak-notifications` | changed: the stakes preview's rows leave out Cheat day while the skip day is off (R4) |
| `crates/notifications/tests/stakes_skip_switch.rs` | `deck-streak-notifications` | added: A15 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the switch's read joined to the preview and the stakes preview, and the cap's reader to the skip-set port |
| `crates/api/src/skip_routes.rs` | `deck-streak-api` | changed: `enabled`, `bridges` and the take's 409 (R12) |
| `crates/api/tests/skip_switch_routes.rs` | `deck-streak-api` | added: A18 |
| `crates/bot/src/skip_commands.rs` | `deck-streak-bot` | changed: the off answer, the will-not-bridge line and the outcome's line (R13) |
| `crates/bot/tests/skip_switch_commands.rs` | `deck-streak-bot` | added: A16, A17 |
| `web/app/src/lib/skip/SkipSheet.svelte` | miniapp | changed: the off notice and the will-not-bridge notice (R14) |
| `web/app/src/lib/skip/api.ts` | miniapp | changed: the preview's `enabled` and `bridges` |
| `web/app/src/lib/skip/skip-bridge.test.ts` | miniapp | added: A19 |
| `web/app/messages/*.json` | miniapp | changed: the two notices, in each locale's catalog (R14) |
| `scripts/mutation-rows.d/S10900-S10999.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `skip_settings` |
| `privacy.json` | repo | changed: `skip_settings` joins the `skip-days` category |
| `PRIVACY.md` | repo | changed: the `skip-days` line names the switch |
| `docs/specs/SPEC-083-the-skip-day-is-recorded-by-deckstreak-and-bridges-the-game-without-writing-to-anki.md` | docs | changed: the insert-only amendment of R11 |
| `docs/schematics/skip-switch-and-the-bridge-cap.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `docs/specs/SPEC-109-the-skip-day-has-a-switch-and-only-the-first-three-skips-of-a-month-bridge-the-streaks.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-109.md` | docs | added |
| `changelog.d/feat-ingest-skip-switch-109.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It adds no control for the switch: the settings screen shows and sets it (#57).
- It changes no rule of the take, its write, its undo or its tariff, and no reader's own rule for
  a skip day: each stays with SPEC-083 and the reader's SPEC (#108).
- It refuses no skip past the cap: such a skip is taken and priced, and bridges nothing, as the
  owner decided (#269).
- It re-settles no closed study day: a changed bridged set changes only what later recomputes
  read (#108).
- It imports nothing: the predecessor holds its switch as configuration, not as a row (#61).

## 6. Risks

- **A switch turned off races a take.** Both write inside `BEGIN IMMEDIATE`, and the take reads the
  switch in its own write (R3), so the take that follows the switch's write is refused.
- **A skip past the cap promises the streak.** The preview's notice and the outcome's line (R8) are
  asserted by A17 and A19, and judged by the ux-laws pack (B3).
- **The tariff and the cap disagree about a month.** One function names the month for both (R5),
  and A10 proves the edge.
- **A reader of a declared day off moves to the bridged set and fines it.** A14's census holds the
  bridged set to the bridge rules.
- **The cap is typed back into code.** A12 fails when the cap read is not the one used.

## 7. Parity goldens

No new golden. The cap's value is the golden constant `SKIP_BRIDGE_MONTHLY_CAP` in SPEC-083's
`goldens/skip.constants.json`, which A4 reads. The switch and the cap are enforced by the owner's
decision at #269, where the predecessor enforces neither, so no function of the predecessor holds
their rules.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `skip_settings` | `ingest` | `migrations/010901_ingest_skip_settings.sql` (SPEC-109) | none: the predecessor holds the switch as configuration (`config.py:Settings.skip_enabled`), so the import writes nothing and the switch starts on | exported; reset in place on erase |

## 9. Mutation rows

The band is `S10900-S10999`, in `scripts/mutation-rows.d/S10900-S10999.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S10901-THE-SWITCH-STARTS-ON` | `migrations/010901_ingest_skip_settings.sql` | the seeded row's `enabled` is 1, a value held in the migration (a script-mutation row with a cargo killer) | `skip_switch::the_switch_starts_on_and_a_change_bumps_the_settings_generation_once` |
| `S10902-THE-SWITCH-REFUSES-FIRST` | `crates/ingest/src/skip.rs` | the switch is read before `already_skipped`; the mutant that drops the read is bounded by A1's 5 seconds | `skip_switch::a_take_while_the_switch_is_off_is_refused_first_writing_and_sending_nothing` |
| `S10903-A-CHANGE-BUMPS-THE-GENERATION` | `crates/ingest/src/skip.rs` | the setter bumps the settings generation in its write | `skip_switch::the_switch_starts_on_and_a_change_bumps_the_settings_generation_once` |
| `S10904-AN-UNCHANGED-SWITCH-BUMPS-NOTHING` | `crates/ingest/src/skip.rs` | the value compared before the bump (the on-boundary case: off written twice) | `skip_switch::the_switch_starts_on_and_a_change_bumps_the_settings_generation_once` |
| `S10905-THE-CAP-KEEPS-THE-FIRST-THREE` | `crates/coordination/src/skip/days.rs` | a month's count compared below the cap (the on-boundary case: the 4th) | `skip_bridges::the_first_three_skips_of_a_month_bridge_and_the_fourth_is_a_missed_day` |
| `S10906-THE-FIRST-BY-STUDY-DAY` | `crates/coordination/src/skip/days.rs` | the skip days taken in study-day order | `skip_bridges::the_first_three_skips_of_a_month_bridge_and_the_fourth_is_a_missed_day` |
| `S10907-THE-MONTH-OF-THE-STUDY-DAY` | `crates/coordination/src/skip/days.rs` | the month read from the study day's year and month (the on-boundary case: a month's last study day and the next month's first) | `skip_bridges::the_cap_counts_each_calendar_month_apart` |
| `S10908-THE-PREVIEW-FLAGS-PAST-THE-CAP` | `crates/coordination/src/skip/mod.rs` | `bridges` false at the cap's count (the on-boundary case: 2 bridged against 3) | `skip_bridges::the_preview_says_when_a_skip_will_not_bridge` |
| `S10909-THE-STREAKS-READ-THE-BRIDGED-SET` | `crates/coordination/src/recompute/streaks.rs` | the streak step passes the bridged set, not the skip set | `skip_bridges::the_first_three_skips_of_a_month_bridge_and_the_fourth_is_a_missed_day` |
| `S10910-THE-CAP-IS-READ` | `crates/economy/src/tariff.rs` | the cap read from its key in the text | `skip_bridge_cap::the_bridge_cap_is_read_from_economy_json_and_equals_the_golden` |
| `S10911-NO-CHEAT-DAY-WHILE-OFF` | `crates/notifications/src/nudges.rs` | the Cheat day row left out while the skip day is off | `stakes_skip_switch::the_stakes_preview_offers_no_cheat_day_while_the_skip_day_is_off` |

# SPEC-047: the owner's tap marks a reading read, its new cards measure it studied, and each earns its XP once

- **Wave:** W1. **Issue:** #33 (epic #2). **Context(s):** `deck-streak-readings` (the studied rule, the XP amounts, the read state); `deck-streak-coordination` (the tap and settle use cases); `deck-streak-api` (the read route).
- **Decided by:** ADR-006 (owner-only requests), ADR-012 (the parity oracle), ADR-019 (the owner's
  read-tap and XP decisions), and ADR-047 (the studied window, the XP constants' home, the grant
  keys and the tap-only tick).
- **Status:** promoted from `docs/specs/planned/` by the delivery that builds it, with its tests and
  `docs/red-first/SPEC-047.md` (ADR-016).

## 1. The problem, measured

- **A reading earned nothing.** The predecessor attached no XP, streak, habit or quest to a reading.
  Its "Studied" box was ticked by code at the next nightly run once 80% of the covered cards were
  reviewed (`preread_tracking.py:is_studied`, `reading_notes.py:stamp_studied`), up to a day late;
  its "I read it" box was never read by code, and the column meant to mirror it was never written.
- **The owner's decisions.** The Mini App's "I read it" writes the vault note's box only on the
  owner's tap; code never ticks it on its own and never clears it. XP is 40 when a reading is marked
  read, plus 60 when at least 80% of its new cards are reviewed within two study days, at most 100
  per reading, granted once, and no new streak.
- **A rule to keep from the charter.** A time-boxed verdict is revisable against late-arriving
  reviews (constraint 6): a review made inside the window but synced after it still counts.
- **What the parity oracle proves.** `preread_tracking.py:is_studied`, including a covered count of
  zero and the exact 80% boundary.
- **Prerequisites.** SPEC-040 (the grant port), SPEC-042 (the read tick and the Studied stamp),
  SPEC-046 (a stored ready reading and its id), SPEC-023 (qualifying reviews by card), SPEC-024 and
  SPEC-025 (the owner's authenticated API). SPEC-053 schedules the settle step; SPEC-051 shows the
  tap and the chip.

## 2. Requirements

R1. `POST /api/readings/{reading id}/read` answers the authenticated owner only. The first tap sets
    `read_at`, grants 40 XP and ticks the vault note's `I read it` line through SPEC-042's read tick.
    A later tap changes no state, grants nothing, writes nothing, and answers with the first
    `read_at`; when the first tap's vault tick failed, a later tap retries the vault tick alone.
R2. Code never ticks `I read it` on its own and never clears it: the read tick has one caller, the
    tap use case, and no job, sync step, roll or regeneration writes that line.
R3. A reading generated on study day d is studied when studied × 100 ≥ covered × 80 in integers,
    where covered is its covered card count and studied is the number of those cards with a
    qualifying review (type 0 to 3, ease 1 or more) made at or after the reading's generation instant
    and on study day d or d + 1; a reading with no covered card is never studied. The rule equals the
    golden of `preread_tracking.py:is_studied`.
R4. The settle step runs after every successful sync (scheduled by SPEC-053). For each reading
    whose window is open it stores the studied count; when the count crosses the threshold it grants
    60 XP and stamps the vault's Studied line. At the rollover that starts study day d + 2 a reading
    below the threshold becomes `retired`.
R5. A retired reading is measured again whenever a sync brings reviews made on a study day inside
    its window, and turns `studied`, with its 60 XP and its stamp, when they cross the threshold.
R6. XP: 40 on read with the source `reading:<reading id>:read` and 60 on studied with the source
    `reading:<reading id>:studied`, both with scope `once`, on the reading's track (`law` or
    `language`), through progression's grant port. A reading never earns more than 100. A grant
    needs a stored ready reading, so a failed or missing reading earns nothing. The amounts are
    constants of the readings context whose source is the owner's decision (ADR-019), not keys of
    `economy.json` (ADR-047).
R7. No streak is created: no readings use case names the streaks context, and no readings table or
    event is read by a streak.
R8. The reading stores `read_at`, `studied_count`, `studied_verdict` (`open`, `studied` or `retired`),
    `studied_at` and `vault_tick` (`none`, `written` or `pending`), columns that
    `migrations/004701_readings_read_and_studied.sql` adds to `readings`; the XP itself lives only in
    the ledger.
R9. The reading view the API serves carries its `covered` and `studied` counts and its verdict, as
    of the last settle pass.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a first tap sets `read_at`, grants 40 XP and ticks the vault line; a second tap changes nothing, grants nothing and writes nothing | `a_second_read_tap_changes_nothing` |
| A2 | the read tick is called only by the tap use case (a census with a planted caller refused), and the settle names no read tick; a roll and a regeneration leave the line as they found it by SPEC-042's A5 and A13 | `the_read_line_is_written_only_by_the_tap` |
| A3 | a later tap retries only a vault tick that failed, and grants nothing again | `a_later_tap_retries_only_a_failed_vault_tick` |
| A4 | the studied rule equals the golden of `preread_tracking.py:is_studied` | `the_studied_rule_matches_the_parity_golden` |
| A5 | reviews made after study day d + 1, or before the generation instant, do not count (injected clock) | `only_reviews_inside_the_two_study_day_window_count` |
| A6 | a settle after a sync stores the count, and on crossing the threshold grants 60 XP and stamps Studied once | `a_settle_grants_and_stamps_once_on_crossing_the_threshold` |
| A7 | a reading below the threshold is retired at the rollover that starts d + 2, and a late sync of reviews made inside its window turns it studied with its 60 XP | `a_retired_reading_turns_studied_on_late_reviews_inside_its_window` |
| A8 | a read and studied reading earns exactly 100 XP, and replaying both grants on later study days writes nothing | `a_read_and_studied_reading_earns_exactly_100_xp_once` |
| A9 | no XP is granted for a failed topic or an unknown reading id | `no_xp_is_granted_for_a_failed_or_missing_reading` |
| A10 | no module of the readings use cases names the streaks context (a census with a planted import refused, examined count reported) | `the_reading_use_cases_touch_no_streak` |
| A11 | the read route answers only the owner, and a stranger gets no state change | `the_read_route_answers_only_the_owner` |
| A12 | the window is read in the configured offset and rollover hour: with the rollover at UTC+9, the last instant of study day d + 1 counts and the instant the next study day begins does not, and the window is over exactly then | `the_window_follows_the_configured_offset` |
| A13 | a reading whose stamp of the Studied line failed is not counted studied and its verdict stays open with no studied instant; the next pass stamps it, counts it once and grants nothing again | `a_failed_stamp_leaves_the_reading_open_for_the_next_pass` |
| A14 | the settle and the tap read the window and the study day in the configured rule: over sixteen rules generated as the product of eight offsets (west of, at, on the half hour east of and east of UTC) and two rollover hours, a reading whose cards were reviewed at the last instant of study day d + 1 is counted studied, the same reviews at the instant d + 2 begins are not, the reading stays open until that instant and retires at it, and the day handed to the XP grant, to the stamp's note lookup and to the tap's grant is the configured study day (examined count reported) | `the_settle_and_the_tap_read_the_configured_study_day` |
| A15 | generation dates its run and its readings in the configured study day, and reads the topic's days in it, over the same sixteen generated rules (examined count reported) | `the_generation_dates_its_run_and_readings_in_the_configured_study_day` |
| A16 | the resolution reads its pause window and dates its run in the configured study day: a review at the start of study day d is resolved and one instant before it pauses, over the same sixteen rules (examined count reported) | `the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule` |
| A17 | every exported reading column holds, in every seeded row, a value distinct from every other column of its kind and from its own column default, so an export that writes one column from another is refused; the count of column pairs examined is reported | `the_exported_tables_equal_the_erased_tables_over_every_port` |

```acceptance
A1: cargo test -p deck-streak-coordination --test readings_read_tap -- --exact a_second_read_tap_changes_nothing
A2: cargo test -p deck-streak-coordination --test readings_read_tap -- --exact the_read_line_is_written_only_by_the_tap
A3: cargo test -p deck-streak-coordination --test readings_read_tap -- --exact a_later_tap_retries_only_a_failed_vault_tick
A4: cargo test -p deck-streak-readings --test studied -- --exact the_studied_rule_matches_the_parity_golden
A5: cargo test -p deck-streak-readings --test studied -- --exact only_reviews_inside_the_two_study_day_window_count
A6: cargo test -p deck-streak-coordination --test readings_settle -- --exact a_settle_grants_and_stamps_once_on_crossing_the_threshold
A7: cargo test -p deck-streak-coordination --test readings_settle -- --exact a_retired_reading_turns_studied_on_late_reviews_inside_its_window
A8: cargo test -p deck-streak-coordination --test readings_settle -- --exact a_read_and_studied_reading_earns_exactly_100_xp_once
A9: cargo test -p deck-streak-coordination --test readings_read_tap -- --exact no_xp_is_granted_for_a_failed_or_missing_reading
A10: cargo test -p deck-streak-coordination --test readings_census -- --exact the_reading_use_cases_touch_no_streak
A11: cargo test -p deck-streak-api --test readings_read -- --exact the_read_route_answers_only_the_owner
A12: cargo test -p deck-streak-readings --test studied -- --exact the_window_follows_the_configured_offset
A13: cargo test -p deck-streak-coordination --test readings_settle -- --exact a_failed_stamp_leaves_the_reading_open_for_the_next_pass
A14: cargo test -p deck-streak-coordination --test readings_settle -- --exact the_settle_and_the_tap_read_the_configured_study_day
A15: cargo test -p deck-streak-coordination --test readings_generate_rule -- --exact the_generation_dates_its_run_and_readings_in_the_configured_study_day
A16: cargo test -p deck-streak-coordination --test readings_resolve -- --exact the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule
A17: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact the_exported_tables_equal_the_erased_tables_over_every_port
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/readings/src/studied.rs` | `deck-streak-readings` | added: the rule and the window |
| `crates/readings/src/xp.rs` | `deck-streak-readings` | added: the amounts and the grant sources |
| `crates/readings/src/store.rs` | `deck-streak-readings` | changed: the read and studied state, and its reads and writes |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed |
| `migrations/004701_readings_read_and_studied.sql` | `deck-streak-readings` | added |
| `crates/readings/src/data_rights.rs` | `deck-streak-readings` | changed: the export carries the new columns |
| `crates/readings/tests/studied.rs` | `deck-streak-readings` | added |
| `crates/readings/tests/reading_xp.rs` | `deck-streak-readings` | added: pins the amounts, sources and track |
| `crates/readings/tests/progress.rs` | `deck-streak-readings` | added: the stored read and studied state |
| `crates/coordination/src/readings/mod.rs` | `deck-streak-coordination` | changed: declares the tap and the settle |
| `crates/coordination/src/readings/read_tap.rs` | `deck-streak-coordination` | added |
| `crates/coordination/src/readings/settle.rs` | `deck-streak-coordination` | added |
| `crates/coordination/tests/readings_read_tap.rs` | `deck-streak-coordination` | added |
| `crates/coordination/tests/readings_settle.rs` | `deck-streak-coordination` | added |
| `crates/coordination/tests/readings_generate_rule.rs` | `deck-streak-coordination` | added: generation in the configured rule, in its own file so the stacked change to `readings_generate.rs` never shares its lines |
| `crates/coordination/tests/readings_resolve.rs` | `deck-streak-coordination` | changed: the pause window and the run's day in the configured rule |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: the seeded reading rows carry distinct read and studied values |
| `crates/coordination/tests/readings_census.rs` | `deck-streak-coordination` | added |
| `crates/api/src/readings_routes.rs` | `deck-streak-api` | added: the read route |
| `crates/api/src/router.rs`, `crates/api/src/lib.rs` | `deck-streak-api` | changed: mounts the readings routes |
| `crates/api/tests/readings_read.rs` | `deck-streak-api` | added |
| `crates/api/tests/state_debug.rs` | `deck-streak-api` | added: pins the state's hand-written `Debug` |
| `scripts/mutation-rows.d/S04700-S04799.json` | repo | added |
| `changelog.d/read-tap-047.md` | repo | added |
| `tools/parity-oracle/registry/spec_047.py` | repo | added: registers `preread_tracking.py:is_studied` (SPEC-029's registry) |
| `tools/parity-oracle/goldens/is_studied.json` | repo | added |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-047-readings-read-tap-studied-and-xp.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-047-studied-window-xp-constants-and-tap-only-tick.md` | docs | changed: accepted |
| `docs/red-first/SPEC-047.md` | docs | added |

## 5. What this does NOT do

- It draws no button and no chip; the Mini App's reader does (#37).
- It mints no coins from reading XP (#106).
- It celebrates no level reached through reading XP (#128).
- It shows reading XP on no progress screen (#70).
- It schedules no settle step (#39).
- It composes neither use case into the daemon: the api role mounts no read route, and no vault
  or review adapter behind `ReadTick`, `StudiedStamp` or `ReviewsByCard` is built, so the route
  answers the owner only once they are composed (#39, #45). Composing an adapter that forwards to
  the vault tree's `tick_read` must also narrow A2's census, which allows exactly one caller of
  the read tick in the crates' sources.
- It offers no comeback reading; a comeback reading read by the owner earns its 40 XP through this
  same tap (#35).

## 6. Risks

- **A review made inside the window arrives after it.** Handled by R5 and tested by A7 (constraint 6).
- **A double grant through a changed id.** The id is derived from the reading's identity (SPEC-046),
  and both grants are `once`-scoped; A8 replays them.
- **The vault is unreachable at the tap.** The read state and the XP stand, the tick is recorded
  `pending`, and only the owner's next tap retries it (A3); the Mini App shows it pending (SPEC-051).
- **A settle pass is slow on a large backlog of readings.** Each pass reads every reading not yet
  studied, retired ones included, and asks the review port only for their covered cards since
  generation; narrowing a pass to the windows a sync's reviews touch, and logging its duration,
  belong to the job that schedules it (#39).

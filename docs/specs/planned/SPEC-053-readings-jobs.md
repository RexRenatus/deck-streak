# SPEC-053: the readings jobs read the study day's sync and never sync, keep off the predecessor's slots, are each claimed once, stay quiet with no AI route, and never write the vault as a second writer

- **Wave:** W1. **Issue:** #39 (epic #2). **Context(s):** `deck-streak-coordination` (the job table, the jobs, the settle step of the sync cycle); `deck-streak-daemon` (the `job` role's dispatch); `deploy` (the unit and timer templates).
- **Decided by:** ADR-010 (scheduled jobs as systemd timers, units per role), ADR-011 (side by side:
  off the predecessor's slots, one writer per vault contract), ADR-019 (readings generate whenever
  the last sync succeeded), ADR-037 (one scheduled sync per study day, and no job but `sync` syncs),
  ADR-038 (the device key as a credential from the credential socket), ADR-054 (the AI route is
  optional, and no-AI mode is the default), and ADR-053 (fixed slots that read the study day's sync,
  the settle after each successful sync, one morning job, catch-up, and the vault archive switch).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-053.md` (ADR-016).

## 1. The problem, measured

- **What runs when.** The predecessor fired its readings job at the rollover hour plus 33 minutes,
  settled the studied box only at the next night's run, kept its paid generation out of the start-up
  catch-up, and placed every fixed-minute job off every legal sync-tick set (`timebase.py:tick_minutes`,
  tested exhaustively), with each fire recorded in the cron-fire ledger (its features
  `cron-ledger-and-scheduler` and `startup-catchup`).
- **DeckStreak's sync is not the predecessor's (ADR-037).** DeckStreak syncs once per study day, at
  the rollover hour, minute 7 (04:07 with the default rollover), plus the owner's explicit `/sync`;
  no other job syncs. A readings job therefore reads the study day's sync outcome; it never runs a
  sync of its own, which the W0 plan's sync-first jobs did.
- **No AI route is the default (ADR-054).** The first deploy has no device key, no tunnel and no
  proxy. Without the flagship's route, every night must be whole and quiet: an absent route is a
  configuration, never a failure or a page.
- **Side by side.** The predecessor keeps its own schedule: the sync at :02, :17, :32 and :47; the
  stats file at 04:05; the database maintenance at 04:21; its readings job at 04:33; the morning brief
  at 08:00; the digest at 09:05; the weekly report at 09:10; the evening nudges at 20:00 and 22:00; the
  landmarks at 22:43; the drill poll at :09 and :39 and the liveness watch at :51 every hour. The second
  brain's own nightly pass runs at 04:00, its probe at 04:20 and its conflict scan at 04:30. DeckStreak
  keeps off all of them (ADR-011).
- **One writer.** The predecessor's own readings job still fires every night. It has never written a
  note, but it would the first night its gate passes, so DeckStreak must not write the vault's readings
  folder until that job is disabled with the owner's go (the first live night, #45).
- **Prerequisites.** SPEC-027 (the job table, the `job` role, the cron-fire ledger, the catch-up and
  the daily sync slot), SPEC-022 and SPEC-023 (the sync, its ledger and the study day's sync
  outcome, and the sync cycle whose successful syncs the settle step follows), SPEC-026 (the owner's
  `/sync`), SPEC-032 (the unit templates and the host budget), SPEC-041 (the router's flush and
  alert kind), SPEC-043 (the AI route port), SPEC-045 to SPEC-048 (the resolution, the generation,
  the settle and the topic lock), SPEC-052 and then SPEC-049 (the morning job's two branches).

## 2. Requirements

R1. Two readings jobs join SPEC-027's job table, each with its own service and timer: the generation
    job `readings-generate` and the morning readings job `readings-morning`. The settle is a step after
    every successful sync, scheduled or owner-triggered, beside the router's flush, not a job of its
    own.
R2. The generation job fires at the rollover hour plus 40 minutes (04:40 with the default 04:00
    rollover) and the morning job at 08:10, outside the quiet hours. A test over the schedule proves
    both slots are off the daily sync slot (the rollover hour, minute 7, SPEC-027) and off every
    predecessor and second-brain slot listed in section 1, the predecessor's sync minutes included.
R3. The generation job claims its fire for the study day in the cron-fire ledger before it acts; a
    second fire on the same study day does nothing. It never syncs (ADR-037). It then reads the AI
    route (SPEC-043): with the route `Absent`, every topic ends `ai_route_absent` (SPEC-046) and the
    job does nothing more. Otherwise it reads the study day's sync outcome (SPEC-022), after taking
    the collection lock in its shared mode so that a sync still in flight ends first: the job's own
    re-read under that lock, not the outcome the runner handed it before the lock. When no sync has
    succeeded in the study day yet (a restart missed the scheduled slot, `sync`'s catch-up has not
    run, and the owner has triggered none), the job runs no sync and waits for none (SPEC-027 R5):
    the outcome is not succeeded. The shared lock is a wait for read consistency on a sync already
    in flight, never a wait for a sync to be run. The generation unit is ordered `After=` the `sync`
    job's unit, so a start of both queued together runs the sync to its end first (R9). The outcome
    succeeded when a sync that started in the study day succeeded, which is the scheduled sync unless
    the owner has triggered one since. Only then does it resolve the topics (SPEC-045), roll forward
    and generate (SPEC-046). When the study day's sync did not succeed, every topic ends
    `could_not_tell` with `sync_failed`, the fire records it, and the owner can trigger a sync and
    regenerate (SPEC-048).
R4. When a configured route fails (the agent cannot be reached), the generation job records each
    topic's outcome (`failed`, `agent_unavailable:<cause>`), raises one alert for the run through the
    router rather than one per topic, and changes no reading already delivered: earlier readings,
    their read state and their vault notes are untouched. With the route `Absent`, the night records
    `ai_route_absent` for every topic and raises nothing (ADR-054).
R5. The morning job claims its fire for the study day, and takes the comeback branch in a lapse
    (SPEC-049) and the ready line otherwise (SPEC-052); it never takes both.
R6. The settle step (SPEC-047) runs after every successful sync, scheduled or owner-triggered, and
    records its passes in the cron-fire ledger under its own job id.
R7. Both jobs are in SPEC-027's catch-up: a fire missed by at most 360 minutes is run once at start,
    claimed first, and a later one is recorded missed. Unlike the predecessor, the generation is caught
    up: the owner set no spend cap, and a reading generated late is still read that day (ADR-053).
R8. The vault archive switch `readings_vault_archive` defaults to off. While it is off, the generation
    stores and serves every reading in the Mini App and writes nothing to the vault, recording
    `vault_archive_off`. The first live night's checklist switches it on only after the predecessor's
    readings job is disabled with the owner's go (#45), so the readings folder has one
    writer at every moment.
R9. The templates `deploy/systemd/deck-streak-readings-generate.service` and `.timer`, and
    `deck-streak-readings-morning.service` and `.timer`, carry placeholder values only. As committed,
    the generation unit runs in no-AI mode (ADR-054): the runner's settings (the route setting and
    the proxy URL, SPEC-043) and the device key credential are optional, and without them the route
    is `Absent`. When the owner enables the proxy route, the private rail (#41) adds them: the route
    setting and the proxy URL through the unit's environment file, and the device key as a
    credential, `LoadCredential=<id>:/run/deck-streak-credentials/socket`, in a drop-in beside the
    template (ADR-038). The device key is never an `Environment=` value, and no secret name or
    project is in the unit. The unit has a `RuntimeMaxSec` above one run of every configured topic at
    its caps, `OnFailure=` the alert template unit, and the shared lock directory and the vault root
    in `ReadWritePaths=`. `deploy/host-budget.json` gives both units their memory limits. The
    generation unit is ordered `After=` the `sync` job's unit (SPEC-032's job template): when both
    starts are queued together, as a boot that fires both missed timers can queue them, the
    generation starts once the sync's `oneshot` run has ended (R3). `After=` holds a start of the
    generation back only while a start of the sync is pending.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the generation reads the study day's sync outcome and never runs a sync: it generates after the study day's successful sync, and a recording fake of the sync cycle sees no call | `the_generation_reads_the_study_days_sync_and_never_syncs` |
| A2 | two fires of the generation job on one study day generate once | `two_fires_of_the_generation_on_one_study_day_generate_once` |
| A3 | every readings slot is off the daily sync slot and off the predecessor's slots, its sync minutes included, and the second brain's (examined count reported) | `every_readings_slot_is_off_the_daily_sync_slot_and_the_predecessors_slots` |
| A4 | with a configured route, an unreachable agent is recorded per topic and alerted once for the run, and the readings already delivered, their read state and their vault notes are unchanged | `an_unreachable_agent_is_recorded_and_alerted_once_and_delivered_readings_stand` |
| A5 | a study day whose sync did not succeed ends every topic `could_not_tell` with `sync_failed`, runs no sync and calls the agent zero times | `a_failed_sync_ends_every_topic_could_not_tell_without_generation` |
| A6 | the morning job takes the comeback branch in a lapse and the ready line otherwise, never both | `the_morning_job_takes_one_branch` |
| A7 | the settle step runs after each successful sync, scheduled or owner-triggered, and is recorded under its own job id | `the_settle_step_runs_after_each_successful_sync` |
| A8 | a generation missed by 300 minutes is caught up once, and one missed by 400 minutes is recorded missed | `a_missed_generation_is_caught_up_once_within_360_minutes` |
| A9 | with the vault archive switch off, a generation writes no vault byte and records `vault_archive_off` | `the_vault_archive_stays_off_until_it_is_switched_on` |
| A10 | the readings unit templates carry no private value and pass the durable-services unit rows | durable-services unit rows; `test_the_readings_units_pass_the_unit_rows_with_placeholders_only` |
| A11 | a night with the route absent records `ai_route_absent` for every topic, raises no alert, stores nothing and reads no sync | `a_night_with_the_route_absent_records_ai_route_absent_and_raises_nothing` |
| A12 | the generation unit's template is ordered `After=` the `sync` job's unit (R3, R9) | `test_the_generation_unit_is_ordered_after_the_sync_job` |

```acceptance
A1: cargo test -p deck-streak-coordination --test readings_jobs -- --exact the_generation_reads_the_study_days_sync_and_never_syncs
A2: cargo test -p deck-streak-coordination --test readings_jobs -- --exact two_fires_of_the_generation_on_one_study_day_generate_once
A3: cargo test -p deck-streak-coordination --test readings_jobs -- --exact every_readings_slot_is_off_the_daily_sync_slot_and_the_predecessors_slots
A4: cargo test -p deck-streak-coordination --test readings_jobs -- --exact an_unreachable_agent_is_recorded_and_alerted_once_and_delivered_readings_stand
A5: cargo test -p deck-streak-coordination --test readings_jobs -- --exact a_failed_sync_ends_every_topic_could_not_tell_without_generation
A6: cargo test -p deck-streak-coordination --test readings_jobs -- --exact the_morning_job_takes_one_branch
A7: cargo test -p deck-streak-coordination --test readings_jobs -- --exact the_settle_step_runs_after_each_successful_sync
A8: cargo test -p deck-streak-coordination --test readings_jobs -- --exact a_missed_generation_is_caught_up_once_within_360_minutes
A9: cargo test -p deck-streak-coordination --test readings_jobs -- --exact the_vault_archive_stays_off_until_it_is_switched_on
A10: python3 -m unittest discover -s scripts/tests -p test_readings_units.py -k test_the_readings_units_pass_the_unit_rows_with_placeholders_only
A11: cargo test -p deck-streak-coordination --test readings_jobs -- --exact a_night_with_the_route_absent_records_ai_route_absent_and_raises_nothing
A12: python3 -m unittest discover -s scripts/tests -p test_readings_units.py -k test_the_generation_unit_is_ordered_after_the_sync_job
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/jobs/readings.rs` | `deck-streak-coordination` | added: the two jobs and their slots |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job table registers them and their catch-up |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the settle step after a successful sync |
| `crates/coordination/src/readings/generate.rs` | `deck-streak-coordination` | changed: the claim, the route check, the study day's sync outcome, the vault archive switch |
| `crates/coordination/tests/readings_jobs.rs` | `deck-streak-coordination` | added |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the `job` role dispatches the two job ids |
| `deploy/systemd/deck-streak-readings-generate.service` | deploy | added |
| `deploy/systemd/deck-streak-readings-generate.timer` | deploy | added |
| `deploy/systemd/deck-streak-readings-morning.service` | deploy | added |
| `deploy/systemd/deck-streak-readings-morning.timer` | deploy | added |
| `deploy/host-budget.json` | deploy | changed: the two units |
| `scripts/tests/test_readings_units.py` | repo | added |
| `docs/specs/SPEC-053-readings-jobs.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-053-readings-slots-settle-after-sync-and-one-writer.md` | docs | added |
| `docs/red-first/SPEC-053.md` | docs | added |

## 5. What this does NOT do

- It installs no unit on the host and enables no timer (#42).
- It opens no tunnel and adds no device key (#43).
- It enables no AI route: with none configured, every night runs in no-AI mode until the owner's
  choice at gate 3 (#162).
- It pre-creates no vault folder, disables no predecessor job and switches the vault archive on for
  no one: that is the first live night, behind the owner's go (#45).
- It changes no sync schedule and no scheduler framework (#15, #20).
- It retires none of the predecessor's jobs (#63).

## 6. Risks

- **Two writers of the readings folder.** The predecessor's nightly readings job still fires, and it
  would write the first night its gate passed. Prevented by R8: the vault archive stays off until the
  first live night disables that job with the owner's go (#45), and tested by A9.
- **The sync slot moves.** A3 reads the daily sync slot from SPEC-027's job table, and the
  predecessor's minutes from its schedule's goldens, not from a copied list.
- **The scheduled sync is still retrying at the generation's slot.** The generation reads the
  outcome under the collection lock's shared mode, which waits for a sync in flight (R3), so it never
  reads a sync that has not ended.
- **The host is down across the sync slot.** `sync` is SPEC-027's catch-up job (its R1), so a sync
  missed by at most 360 minutes runs once when its timer's `Persistent=` activates it. A generation
  that runs while the study day has no successful sync (the host was down longer, the sync failed,
  or the generation's start was not queued beside the sync's, R9) ends every topic `could_not_tell`
  with `sync_failed` (R3, A5). The owner's `/sync` and a regeneration (SPEC-048) recover the day.
- **A heavy day runs past the unit's runtime bound.** Each topic's run is capped, and the health check
  pages a run that did not finish (SPEC-050).
- **A caught-up generation costs an extra run after a restart.** It is claimed once and bounded to 360
  minutes late; the owner set no spend cap and receives the readings that day.

## 7. Amendments before delivery

- **R3 (2026-09-28): the generation job never runs the scheduled sync, even after a restart.** R3
  said that when a restart had missed the sync's slot, the job would first run that scheduled sync
  itself (ADR-037's first reading). SPEC-027, delivered in #224, decides that no job but `sync` runs
  the sync cycle, and A1 here already said the job never runs a sync. R3 now reads the missing
  outcome as not succeeded (every topic `could_not_tell` with `sync_failed`, as A5 requires), and
  the generation unit is ordered `After=` the `sync` job's unit, so that when both starts are queued
  together the sync's run ends first (R9, A12). ADR-037 carries the matching note.
- **Section 6 (2026-09-28): the host-down risk.** It said SPEC-027 does not catch the sync up;
  SPEC-027 as delivered makes `sync` its catch-up job (its R1). The risk now names what the catch-up
  recovers and what still ends `sync_failed`.

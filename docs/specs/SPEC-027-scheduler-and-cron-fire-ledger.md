# SPEC-027: scheduled jobs run as timers through one runner that claims every daily fire, catches up at most six hours late, and pages when the sync stops

- **Wave:** W0. **Issue:** #20 (epic #1). **Context(s):** `deck-streak-coordination` (the job table, the runner, the ledger, the watch), `deck-streak-daemon` (the `job` role).
- **Decided by:** ADR-010 (scheduled jobs as timers, one binary), ADR-011 (DeckStreak's jobs keep off the predecessor's slots while both run), ADR-012 (goldens), ADR-020 (one migration sequence), ADR-037 (one scheduled sync per study day, and no job but `sync` syncs), and this SPEC's ADR-027 (systemd timers with a ledger-owning runner, the job table as the one schedule, DeckStreak's slots).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-027.md`, and seven goldens
  (`cron_ledger`, `catchup`, `predecessor_schedule`, `tick_minutes`, `signed_skew`,
  `rollover_skew`, `scheduler.constants`) generated at the predecessor's `27ee2bc`. The delivery
  made §1, R5, R7, A11, the manifest and §6 exact where the code decided them (§7).

## 1. The problem, measured

- **The predecessor's guards, which port verbatim except its sync layout** (predecessor `27ee2bc`,
  names only):
  - the cron-fire ledger `cron_fires`, one row per (job, local fire date) with ok, error, catchup
    and missed counters and a last outcome (`database.py`, migration 18; `CRON_OUTCOMES`);
  - the claim: a conditional upsert that succeeds only while the row records no attempt (ok, error
    and catchup all zero; a `missed` is not an attempt), writing the attempt BEFORE the job runs so
    a crash loop can never send twice (`database.py:GamifyStore.claim_cron_fire`);
  - the release: after a replay that attempted a send and got no message id, the claim is undone so
    a later start may try again; a replay that never engaged the notifier keeps its claim
    (`database.py:GamifyStore.release_cron_fire`, `scheduler.py:run_startup_catchup`);
  - the record: `last_fire_at` advances only for outcomes that ran, and a `missed` is suppressed
    when positive evidence says the fire already has a verdict
    (`database.py:GamifyStore.record_cron_fire`, `pipeline_layers/ops.py:OpsLayer.record_cron_fire`);
  - the catch-up: only the allowlisted notification jobs are ever replayed, a fire more than
    `scheduler.py:CATCHUP_MAX_LATE_MIN` minutes late is recorded `missed` and not sent, replays run
    in fire order after the first sync (`scheduler.py:CATCHUP_JOB_IDS`, `run_startup_catchup`);
    DeckStreak's replays run no sync first (ADR-037);
  - the layout, which does NOT port: fixed-minute jobs sit off the sync-tick set
    (`timebase.py:tick_minutes`, `constants.py:SYNC_TICK_OFFSET_MIN`), and every notification job
    syncs first without gating its send on the sync (`scheduler.py:_guarded_sync`,
    `SYNC_GUARD_WAIT_SECS`). DeckStreak syncs once per study day, and no job but `sync` syncs: a job
    reads the study day's sync outcome instead (ADR-037). The tick function is kept only to name the
    predecessor's sync minutes, which DeckStreak's slots keep off;
  - the dead-man watch and the drift check (`pipeline_layers/ops.py:OpsLayer._check_deadman`,
    `_check_rollover_drift`, `LIVENESS_STALE_INTERVAL_MULTIPLE`, `LIVENESS_MIN_STALE_SECS`,
    `LIVENESS_BOOT_GRACE_SECS`, `ROLLOVER_DRIFT_TOLERANCE_MIN`, `timebase.py:signed_skew_minutes`,
    `timebase.py:rollover_skew_minutes`);
  - sync-failure alerting after `SYNC_FAIL_ALERT_THRESHOLD` consecutive failures, and daily
    maintenance: `wal_checkpoint(TRUNCATE)`, `optimize`, and ledger rows past
    `database.py:CRON_FIRES_RETENTION_DAYS` deleted (`database.py:GamifyStore.maintenance`).
- **What the parity oracle proves** (registered in `tools/parity-oracle/registry/spec_027.py`): the
  ledger's claim, release and record sequences (`goldens/cron_ledger.json`, an adapter that opens
  the predecessor's store on a synthetic temporary database and runs each case's sequence of
  operations, returning every result and the row after each step); the catch-up decision
  (`goldens/catchup.json`, an adapter over `run_startup_catchup` with a stub scheduler holding one
  cron trigger and a stub pipeline recording claims, records and invocations); the tick sets that
  name the predecessor's sync minutes (`goldens/tick_minutes.json`, `timebase.py:tick_minutes`); the
  skew arithmetic
  (`goldens/signed_skew.json`, `goldens/rollover_skew.json`); the predecessor's in-process schedule
  (`goldens/predecessor_schedule.json`, an adapter over `scheduler.py:create_scheduler` with
  default settings and a stub pipeline, listing each job's trigger fields); and the constants
  (`goldens/scheduler.constants.json`).
- **The job table also keeps off three reserved minutes**, 0, 25 and 39 (ADR-011).
- **Nothing schedules anything yet** in DeckStreak (read at `main` e05dfa5).

**Order.** After SPEC-022 (the sync cycle and `sync_runs`) and SPEC-025 (the binary this SPEC adds
the `job` role to). It can run beside SPEC-023. SPEC-021 declares `cron_fires` exempt and SPEC-032
writes the timers from this SPEC's job table, so both land after it.

## 2. Requirements

R1. `coordination::jobs::TABLE` is the one schedule (ADR-027): each job has an id, a schedule (daily
    at the rollover hour and a minute, daily at a local time, or hourly at a minute) and one flag,
    `catch_up`. No job syncs first: only `sync` syncs, and every other job reads the study day's sync
    outcome (SPEC-022, ADR-037). At W0 it holds `sync` (daily at the rollover hour, minute 7: the one
    scheduled sync of the study day, claimed per study day like every once-a-day job, because its
    slot follows the rollover and its fire date is the study day it runs in), `maintenance` (daily at
    the rollover hour, minute 28) and `liveness` (hourly at minute 14). `sync` is `catch_up` (a fire
    missed by at most `CATCHUP_MAX_LATE_MIN` minutes runs once at start, claimed for its study day);
    the others are not.
R2. No DeckStreak job shares a minute with the predecessor's schedule while both run, or with a
    reserved minute: the in-process slots of `goldens/predecessor_schedule.json`, the predecessor's
    sync ticks (its tick function at its offset, `goldens/tick_minutes.json`), and the reserved
    minutes 0, 25 and 39 (ADR-011); and no other DeckStreak job shares the daily `sync` slot.
R3. `cron_fires` (`migrations/002701_coordination_cron_fires.sql`, `STRICT`, `created_at`) keeps the
    predecessor's columns and constraint: `job_id`, `fire_date` (the LOCAL calendar date of the
    scheduled fire under the configured offset, which the predecessor called `fire_day`; a calendar
    date, deliberately not a study day), the first-seen, updated and last-fire instants, the four
    counters, and `last_outcome` checked against `ok`, `error`, `catchup`, `missed`; primary key
    (`job_id`, `fire_date`).
R4. `coordination::ledger` ports claim, release and record with the predecessor's semantics, each in
    one `BEGIN IMMEDIATE` write, and their results and rows equal `goldens/cron_ledger.json` for
    every case.
R5. `coordination::runner::run(job)` is the one entrance every timer reaches (`deckstreakd job <id>`):
    it finds the job's latest scheduled instant at or before now, and a `catch_up` job looks only
    at the fires of now's local calendar day, as the predecessor's catch-up did, doing nothing when
    none has elapsed; no job but `sync` runs the sync
    cycle, and a job that needs the study day's data reads the study day's sync outcome and never
    waits on a sync; a `catch_up` job more than `CATCHUP_MAX_LATE_MIN` minutes late records `missed`
    (unless positive evidence suppresses it) and exits without acting; a once-a-day job, `sync`
    included, claims its (job, fire date) and does nothing when the claim fails; after the job, the
    outcome is recorded; a job that returned "not delivered" after the notifier reported an
    attempted send with no message id releases its claim, and one that never engaged the notifier
    keeps it. The decision equals `goldens/catchup.json` for every case.
R6. The notifier's attempted and delivered counts reach the runner through a `DeliveryMarker` port of
    `coordination`, which the bot's transport implements in the composition root (SPEC-026).
R7. The runner exits 0 when the job ran, skipped, or was recorded `missed`; it exits 1 to PAGE (the
    unit fails and `OnFailure=` sends the one alert, SPEC-031) only on a transition: the first error
    of a job's error streak (a repeat only logs); the first failed scheduled sync of an episode,
    which is an error of the `sync` job, after its bounded retries (with one scheduled sync a day
    the predecessor's `SYNC_FAIL_ALERT_THRESHOLD` of consecutive failures would span days, so
    DeckStreak pages on the first; a hard failure, `engine_failed` or `open_failed`, pages the same
    way, so it needs no transition of its own; an owner-triggered sync that fails never pages,
    because its reply shows the owner); the first liveness check that finds the sync dead; and the
    first check that sees a maintenance fire drift beyond tolerance, once per fire date. It exits 2
    on an unknown job id. Its ERROR line carries reason codes and integers only.
R8. The liveness job pages when no successful sync is newer than
    `SYNC_CADENCE_SECS + CATCHUP_MAX_LATE_MIN * 60` seconds (30 hours by default): one study day
    (`SYNC_CADENCE_SECS`, 86400, ADR-037) plus the catch-up window. This is a recorded divergence
    from the predecessor's `max(LIVENESS_STALE_INTERVAL_MULTIPLE * interval, LIVENESS_MIN_STALE_SECS)`,
    whose multiple was sized for a 15-minute interval and would wait days at one sync a day; or when no sync has ever succeeded
    and the first recorded sync attempt is older than `LIVENESS_BOOT_GRACE_SECS`; and when the
    maintenance job's last fire, read at the configured offset, lies more than
    `ROLLOVER_DRIFT_TOLERANCE_MIN` minutes from its slot (`signed_skew_minutes`,
    `rollover_skew_minutes`).
R9. The maintenance job runs `PRAGMA wal_checkpoint(TRUNCATE)` and `PRAGMA optimize`, and deletes
    `cron_fires` rows whose fire date is more than `CRON_FIRES_RETENTION_DAYS` days old.
R10. `coordination`'s data-rights port declares `cron_fires` EXEMPT, with the reason that an erase
    must never re-arm the catch-up double-send guard (CHARTER 13); the table is registered to
    `coordination` in docs/CONTEXT-MAP.md.
R11. The `job` role of `deckstreakd` runs one job and exits (a `oneshot` unit); it installs logging
    and loads settings like every role, and sends no sd_notify message.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | two concurrent runs of one daily job for one fire date act once | `ledger` test |
| A2 | the ledger's claim, release and record equal the predecessor's | `ledger` test over `goldens/cron_ledger.json` |
| A3 | a catch-up job five hours late is claimed and run once | `runner` test |
| A4 | a catch-up job seven hours late is recorded `missed` and not run | `runner` test |
| A5 | the catch-up decision equals the predecessor's | `runner` test over `goldens/catchup.json` |
| A6 | an attempted send with no message id releases the claim; a job that never sent keeps it | `runner` test |
| A7 | the dead-man watch pages when no sync succeeded within the window, once per episode | `liveness` test on a manual clock |
| A8 | a drift of the maintenance fire beyond the tolerance pages | `liveness` test over `goldens/signed_skew.json` and `goldens/rollover_skew.json` |
| A9 | the daily `sync` slot keeps off the predecessor's sync ticks and every other job's slot, and those ticks equal the predecessor's function | `job_table` test over `goldens/tick_minutes.json` |
| A10 | no job shares a minute with the predecessor's schedule | `job_table` test over `goldens/predecessor_schedule.json` |
| A11 | three consecutive failed scheduled syncs, each after its bounded retries, page once: the first pages, and a repeat error only logs | `runner` test |
| A12 | no job but `sync` runs the sync cycle: every other job reads the study day's sync outcome, and a recording fake of the cycle sees no call from it | `runner` test |
| A13 | maintenance checkpoints, optimises, and prunes ledger rows past the retention | `maintenance` test |
| A14 | the ledger is declared exempt from export and erase, with its reason | `data_rights` test |
| A15 | the scheduler's constants equal the predecessor's | `job_table` test over `goldens/scheduler.constants.json` |
| A16 | the `job` role runs a job by id and refuses an unknown id with code 2 | daemon `roles` test |
| A17 | the job table holds `sync` to one daily slot, the rollover hour at minute 7, claimed per study day like every once-a-day job | `job_table` test |

```acceptance
A1: cargo test -p deck-streak-coordination --test ledger -- --exact two_concurrent_runs_of_one_daily_job_for_one_fire_date_act_once
A2: cargo test -p deck-streak-coordination --test ledger -- --exact the_ledger_matches_the_predecessors_golden
A3: cargo test -p deck-streak-coordination --test runner -- --exact a_catch_up_job_five_hours_late_is_claimed_and_run_once
A4: cargo test -p deck-streak-coordination --test runner -- --exact a_catch_up_job_seven_hours_late_is_recorded_missed_and_not_run
A5: cargo test -p deck-streak-coordination --test runner -- --exact the_catch_up_decision_matches_the_predecessors_golden
A6: cargo test -p deck-streak-coordination --test runner -- --exact an_attempted_send_without_a_message_id_releases_the_claim
A7: cargo test -p deck-streak-coordination --test liveness -- --exact the_dead_man_watch_pages_once_when_no_sync_succeeded_within_the_window
A8: cargo test -p deck-streak-coordination --test liveness -- --exact a_maintenance_fire_drifting_past_the_tolerance_pages
A9: cargo test -p deck-streak-coordination --test job_table -- --exact the_daily_sync_slot_keeps_off_the_predecessors_ticks_and_every_other_slot
A10: cargo test -p deck-streak-coordination --test job_table -- --exact no_job_shares_a_minute_with_the_predecessors_schedule
A11: cargo test -p deck-streak-coordination --test runner -- --exact the_third_consecutive_sync_failure_pages_once
A12: cargo test -p deck-streak-coordination --test runner -- --exact no_job_but_sync_runs_the_sync_cycle
A13: cargo test -p deck-streak-coordination --test maintenance -- --exact maintenance_checkpoints_optimises_and_prunes_the_ledger
A14: cargo test -p deck-streak-coordination --test data_rights -- --exact the_cron_fire_ledger_is_declared_exempt_with_its_reason
A15: cargo test -p deck-streak-coordination --test job_table -- --exact the_scheduler_constants_equal_the_predecessors
A16: cargo test -p deck-streak-daemon --test roles -- --exact the_job_role_runs_a_job_by_id_and_refuses_an_unknown_one
A17: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day
```

The runner tests register a synthetic `catch_up` notification job with a fake `DeliveryMarker`,
because no real notification job exists before W1; every clock is a `ManualClock`, and A12 runs
each job against a recording fake of the sync cycle. A1 runs two runner tasks against one temporary
database.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | added: the job table, `sync` as one daily slot, and the port of the predecessor's tick function |
| `crates/coordination/src/ledger.rs` | `deck-streak-coordination` | added: claim, release, record |
| `crates/coordination/src/runner.rs` | `deck-streak-coordination` | added: the one entrance, the catch-up, the paging transitions |
| `crates/coordination/src/liveness.rs` | `deck-streak-coordination` | added: the dead-man watch and the drift check |
| `crates/coordination/src/maintenance.rs` | `deck-streak-coordination` | added |
| `crates/coordination/src/delivery.rs` | `deck-streak-coordination` | added: the `DeliveryMarker` port |
| `crates/coordination/src/data_rights.rs`, `crates/coordination/src/lib.rs`, `crates/coordination/Cargo.toml` | `deck-streak-coordination` | added or changed |
| `crates/coordination/tests/ledger.rs`, `crates/coordination/tests/runner.rs`, `crates/coordination/tests/liveness.rs`, `crates/coordination/tests/job_table.rs`, `crates/coordination/tests/maintenance.rs`, `crates/coordination/tests/data_rights.rs` | `deck-streak-coordination` | added: A1 to A15, A17 |
| `Cargo.lock` | workspace | changed: coordination's edges to sqlx and tracing (ADR-003), and its tests' tempfile and tokio (§7) |
| `crates/daemon/src/role_job.rs`, `crates/daemon/src/main.rs` | `deck-streak-daemon` | added or changed: the `job` role |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A16 |
| `migrations/002701_coordination_cron_fires.sql` | `deck-streak-coordination` | added |
| `.sqlx/` | workspace | changed |
| `tools/parity-oracle/registry/spec_027.py` | repo | added: the ledger, catch-up and schedule adapters, the tick and skew functions, the constants |
| `tools/parity-oracle/goldens/cron_ledger.json`, `catchup.json`, `tick_minutes.json`, `signed_skew.json`, `rollover_skew.json`, `predecessor_schedule.json`, `scheduler.constants.json` | repo | added |
| `docs/CONTEXT-MAP.md` | repo | changed: `cron_fires` registered to `coordination`, exempt |
| `docs/schematics/cron-fire-ledger-and-catch-up.md` | repo | changed: planned with the SPEC; the delivery adds the runner's ports and the paging rules (§7) |
| `docs/decisions/ADR-027-scheduled-jobs-as-systemd-timers-with-a-ledger.md` | repo | added |
| `docs/red-first/SPEC-027.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It writes no timer or unit file: the templates, written from this table, are SPEC-032's
  (#25).
- It builds no notification job; the digest, the morning brief and the evening nudges are W5's
  (#129, #122, #117,
  #118), and the readings' jobs W1's (#39).
- It sends no recovery message when the sync recovers, and no digest-window check: both need the
  router's alert kind and the digest (#27, #129).
- It prunes no readings ledger (#36).
- It delivers no page itself: the runner fails its unit, and the alert unit sends the message
  (#24).
- It adds no job for the vault bridge, drills, landmarks or the weekly report
  (#153, #136, #127,
  #130).
- It raises no sync cadence: one scheduled sync per study day holds until cutover decides otherwise
  (#164).

## 6. Risks

- **The timers and the table disagree.** SPEC-032's test holds every timer's calendar equal to its
  job's table entry, and the drift check pages when the maintenance fire lands off its slot.
- **A timer fires twice for one fire date** (a restart with `Persistent=`, a manual start). The
  claim makes the second a no-op; A1 proves it under concurrency, and A17 holds `sync` to a claimed
  daily slot.
- **A `TRUNCATE` checkpoint meets Litestream's read lock.** The checkpoint then completes partially
  and returns busy, which is not an error. The restore drill (W2) proves the replica (#44).
- **A paging job pages on every run.** R7 pages only on transitions recorded in the ledger and
  `sync_runs`; the liveness job runs hourly, and A7 and A11 prove "once per episode".
- **The host is down across the sync slot.** `sync` is a catch-up job (R1), so a sync missed by at
  most six hours runs once when its timer's `Persistent=` activates it. Down longer, that study day
  has no scheduled sync: the jobs that read it record `sync_failed`, and the owner's `/sync`
  (SPEC-026) recovers the day (ADR-037).
- **The sync fails once and the day waits.** With one scheduled sync a day, the failure alert pages
  on R7's transitions, and the dead-man window is a multiple of a study day (R8); the owner's
  `/sync` is the remedy in between (ADR-037).

## 7. Amendments at delivery

- **§6 and ADR-027: `sync` is a catch-up job.** R1 and ADR-037 make it one; the risk that said it
  was not, and ADR-027's sentence that no catch-up job exists at W0, predated ADR-037's amendment of
  the plan. ADR-027 now says `sync` is the one catch-up job at W0, and §6 says what its timer's
  `Persistent=` recovers.
- **R5: the catch-up window is the local calendar day.** The predecessor's catch-up looked only at
  the fires that elapsed since local midnight (`scheduler.py:_latest_elapsed_fire_today`), and
  `goldens/catchup.json` holds the runner to it: its `after_midnight` cases refuse to replay a 22:00
  fire at 01:00. "The latest scheduled instant at or before now" alone would replay it.
- **R7 and A11: one page per episode.** A11's "third consecutive sync failure" predated R7's
  amendment for ADR-037, which pages on the first failure. The test keeps its name, so the fenced
  command is unchanged, and proves what R7 now says: three consecutive failed scheduled syncs, each
  after its three bounded attempts, page once, on the first, and the repeats only log. A failed
  scheduled sync is an error of the `sync` job, so the first-error-of-a-streak rule is the one that
  pages it; the predecessor's separate page on a hard failure existed only because its other
  failures waited for three in a row, and every failure now pages at once. The runner never runs an
  owner-triggered sync, so none can page.
- **R8: the watch's transitions are read from instants.** The first check to find the sync dead is
  the one whose previous check ran at or before the instant the sync died, and the first to see a
  maintenance fire off its slot is the one whose previous check ran before that fire; the previous
  check is the `liveness` job's own `last_fire_at` in the ledger. The drift is the skew between the
  zone that puts the maintenance fire on its slot and the configured one (`signed_skew_minutes`,
  then `rollover_skew_minutes`). The reason codes are `sync_dead`, `sync_never_succeeded` and
  `maintenance_drift`.
- **R9: prune, optimise, then checkpoint.** The prune and `PRAGMA optimize` share one write, so the
  optimiser sees the ledger the prune just used, and `PRAGMA wal_checkpoint(TRUNCATE)` runs last,
  outside any transaction, so it truncates the log the prune wrote. The predecessor checkpointed
  first.
- **R3: `created_at` beside the first-seen instant.** The table keeps the predecessor's
  `first_seen_at` and adds the workspace's `created_at`; both hold the instant of the row's first
  write.
- **§1: three reserved minutes.** The job table keeps off minutes 0, 25 and 39 (ADR-011), and A10's
  test holds them as one constant, `RESERVED_MINUTES`.
- **Manifest: `Cargo.lock` and the schematic.** Coordination's ledger runs its queries through the
  kernel's `Db` (sqlx, checked into `.sqlx/`) and its runner logs through tracing, both admitted by
  ADR-003; its tests add tempfile and tokio, which the workspace already holds. The lockfile gains
  those four edges and no version. The schematic was planned with the SPEC, so the delivery changes
  it rather than adding it.
- **The `job` role is a module of the binary.** `crates/daemon/src/role_job.rs` is compiled into
  `deckstreakd` through `main.rs`, so the daemon's library, which the manifest does not name, is
  unchanged; A16 drives the role through the binary.

Amendment (2026-09-28): passages describing another service's operations were replaced with neutral
reserved minutes, or removed, under the public-text rule (ADR-059).

## 8. Amendments, 2026-10-02: what each count of the job table's spread pins (#462)

Insert-only: every earlier byte is kept in order, and this amendment inserts sections 8 and 9.

- **Two counts, two pins.** A17's spread judges the scheduled sync at every rollover hour (24), at
  five UTC offsets and on 30 study days. The examined count is the number of fires judged: it pins
  the population's size, 3,600. The distinct count is the number of different members the judges
  were handed: a member is the rollover hour and the UTC offset of the rule a judge is handed and
  the instant it is handed, each recorded inside the judge's own call from what it was handed,
  never from the generator's loop variables. It pins the spread itself, 3,600, a figure the test
  computes as 24 x 5 x 30 outside the generator.
- **Why both.** A generator that folds members (an offset listed twice, an hour listed twice, a
  day listed twice) still judges 3,600 fires, so the examined count cannot see it; the distinct
  count reads 2,880, 3,450 or 3,480 and refuses it. The check that one sync follows the last one's
  study day by exactly one runs after the spread is judged, so a repeated day is refused by the
  distinct count, the criterion's reason, and not by that check.
- **What A18 holds.** Three generator plants, a repeated offset, a repeated hour and a repeated
  day, each read red by the distinct count, with the examined count unchanged. The mutation row
  S02701 folds the member record (it records one offset for every judge) and reads KILLED by A17.

The amendment changes `crates/coordination/tests/job_table.rs` (A17's spread, now judged by
members it records, and A18), `docs/red-first/SPEC-027.md` (the lines of this date) and this SPEC,
and it adds `scripts/mutation-rows.d/S02700-S02799.json` (the band's first row, S02701) and shares
the changelog fragment `changelog.d/fold-settles-once-311.md` with SPEC-071's amendment of this
date. A17's criterion and its fenced command are unchanged.

Section 4's rows that the amendment leaves as they are:

- `crates/coordination/src/jobs.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/ledger.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/runner.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/liveness.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/maintenance.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/delivery.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/data_rights.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/src/lib.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/Cargo.toml`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/tests/ledger.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/tests/runner.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/tests/liveness.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/tests/maintenance.rs`: unchanged by the amendment of 2026-10-02.
- `crates/coordination/tests/data_rights.rs`: unchanged by the amendment of 2026-10-02.
- `Cargo.lock`: unchanged by the amendment of 2026-10-02.
- `crates/daemon/src/role_job.rs`: unchanged by the amendment of 2026-10-02.
- `crates/daemon/src/main.rs`: unchanged by the amendment of 2026-10-02.
- `crates/daemon/tests/roles.rs`: unchanged by the amendment of 2026-10-02.
- `migrations/002701_coordination_cron_fires.sql`: unchanged by the amendment of 2026-10-02.
- `.sqlx/`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/registry/spec_027.py`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/cron_ledger.json`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/catchup.json`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/tick_minutes.json`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/signed_skew.json`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/rollover_skew.json`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/predecessor_schedule.json`: unchanged by the amendment of 2026-10-02.
- `tools/parity-oracle/goldens/scheduler.constants.json`: unchanged by the amendment of 2026-10-02.
- `docs/CONTEXT-MAP.md`: unchanged by the amendment of 2026-10-02.
- `docs/schematics/cron-fire-ledger-and-catch-up.md`: unchanged by the amendment of 2026-10-02.
- `docs/decisions/ADR-027-scheduled-jobs-as-systemd-timers-with-a-ledger.md`: unchanged by the amendment of 2026-10-02.

## 9. Acceptance criteria of the 2026-10-02 spread amendment

| id | criterion | decided by |
|---|---|---|
| A18 | a spread whose generator folds its members (a repeated offset, a repeated hour, a repeated day) reads red by its distinct member count, with its examined count unchanged | `a_spread_that_folds_its_members_reads_red_by_the_distinct_count` |

```acceptance
A18: cargo test -p deck-streak-coordination --test job_table -- --exact a_spread_that_folds_its_members_reads_red_by_the_distinct_count
```

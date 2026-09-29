# SPEC-146: the nightly stats file keeps the predecessor's contract, and is written after the day's one sync by one writer

- **Wave:** W8. **Issue:** #153 (epic #9), the vault stats bridge: the nightly stats snapshot the
  vault's dashboard reads. **Context(s):** `deck-streak-coordination` (the payload's composition,
  its degradation signal and the `vault_stats` job), `deck-streak-vault` (the stats file's
  contract), `deck-streak-analytics` (the unopened projection), `deck-streak-ingest` (the unopened
  read of the copy), `deck-streak-daemon` (the wiring), `deploy` (the timer and its drop-ins).
- **Decided by:** ADR-011 (one writer per vault contract at every moment: DeckStreak writes the
  stats file only once the owner's go has made it that writer), ADR-027 (scheduled jobs as timers
  with a ledger), ADR-037 (one scheduled sync per study day; a job reads the day's sync outcome and
  never runs a sync) and ADR-146 (the stats job fires at the rollover hour plus 12 minutes, after
  the day's one sync, reads its outcome, and never syncs).
- **Prerequisites:** SPEC-020, SPEC-022, SPEC-023, SPEC-027, SPEC-040, SPEC-042, SPEC-045 and
  SPEC-071 (landed); SPEC-053, SPEC-076, SPEC-077, SPEC-078, SPEC-093, SPEC-110 and SPEC-121
  (planned, unlanded: the reserved job slots, the streaks, the law block, the habits summary, the
  leech board, the drills and the law progress this file reports); SPEC-143 (this wave, unlanded:
  the checklist to which this delivery adds its item, so the job is gated from its first commit).
  **Mutation band:** `S14600-S14699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-146.md` (ADR-016).

## 1. The problem, measured

- **The vault's dashboard reads one file that nothing in DeckStreak writes.** The predecessor's
  `vault_bridge.py:build_vault_stats` composes a snapshot of the language and law tracks, and
  `write_vault_stats` writes it into the vault as one JSON object, contract version 2
  (`STATS_CONTRACT_VERSION`), with 19 top-level keys. SPEC-001 §7 lists it as `vault-stats-bridge`,
  W8, build. Measured on dev: no crate names the file, the contract or the job.
- **Its blocks come from ports DeckStreak plans elsewhere.** The language and law numbers are
  SPEC-076's streaks, SPEC-040's level curve, SPEC-071's score and SPEC-077's law block; the leech
  counts are SPEC-093's board summary; the subject table, the drill backlog and the weeks are
  SPEC-121's law progress; the habits are SPEC-078's summary. One block has no port anywhere: the
  unopened projection (`unopened.py`), which reads the collection directly and projects, per
  subject, when its last unseen card gets its first look at the observed intake rate.
- **A block that fails must not read as a measured zero.** The predecessor marks each failed
  block in `degraded`, derives `measured` per block and `stats_ok` from it, and fails closed
  (`_apply_degradation_signal`). A port that loses this turns a failed read into a healthy-looking
  zero on the owner's dashboard.
- **#153 places the job at minute 5 after the rollover.** Under ADR-037 the day's one scheduled
  sync runs at minute 7 (SPEC-027 R5), so a write at minute 5 reports the day before the sync, a
  day stale. ADR-146 moves it to minute 12, after the sync, and makes it read the sync's outcome.
- **The file has one writer at every moment (ADR-011).** Until the owner's go (#164) and the
  checklist's switch of its item (SPEC-143 R10), the job must write nothing.

## 2. Requirements

The contract

R1. **The keys.** The stats file is one JSON object whose top-level keys are, as a set,
    `generated_utc`, `day`, `language`, `law`, `leeches`, `contract`, `law_subjects`,
    `law_subjects_omitted`, `law_subjects_truncated`, `drills`, `habits`, `weeks`,
    `weeks_omitted`, `weekly_history`, `weekly_trend`, `unopened`, `degraded`, `measured` and
    `stats_ok` (19), and every nested object carries the key set the predecessor's cold payload
    carries at that depth (`goldens/stats_bridge_cold_payload.json`). `contract` is 2. The
    constants `STATS_CONTRACT_VERSION` (2), `LAW_SUBJECT_OMITTED` (`cards`, `last_review`),
    `WEEKS_OMITTED` (`law_reviews`, `law_leeches`, `streaks`) and `_DEGRADABLE_BLOCKS`
    (`law_subjects`, `drills`, `habits`, `weeks`, `weekly_history`) equal
    `goldens/stats_bridge.constants.json`.
R2. **The first blocks, from their sources, as a set** (`build_vault_stats`):
    - `generated_utc`: the instant of the composition in UTC, rendered as Python's
      `datetime.isoformat` renders an aware UTC instant: `YYYY-MM-DDTHH:MM:SS.ffffff+00:00`, the
      fraction omitted when it is zero (the clock's milliseconds times 1000 as the microseconds);
    - `day`: the current study day (SPEC-020 R1, the 04:00 rollover) as an ISO date;
    - `language`: `streak`, the `language` track's current streak (SPEC-076 R1); `level`, the level
      of the lifetime XP of every track on the shared curve (SPEC-040); `total_xp`, the lifetime
      `language`-track XP over both XP tables, as SPEC-077 R11 reads the law's; `score`, the
      current study day's `daily_rollup` score (SPEC-071), 0 when the day has no row;
    - `law`: `level`, `streak` and `total_xp` from SPEC-077 R11's law block; `dues` from SPEC-077
      R10, `null` while pending, never 0; `weak_subjects` from R4;
    - `leeches`: `active`, `holding` and `regressed` from SPEC-093 R1's summary.
    These reads are not degradable: a failure of any of them fails the run (R14), and no file is
    written.
R3. **The later blocks, from their sources, as a set:** `law_subjects` (at most 64 rows) and
    `law_subjects_truncated` from SPEC-121 R7; `drills` from SPEC-121 R8, its seven keys
    `active`, `awaiting_grading`, `oldest_age_days`, `oldest_created`, `graded_total`,
    `unmatched_active` and `deferred`; `weeks`, `weekly_history` and `weekly_trend` from SPEC-121
    R9 and R10; `habits` from SPEC-078 R12's summary, as R5 shapes it; `unopened` from R9;
    `law_subjects_omitted` and `weeks_omitted` from R1's constants.
R4. **The weak subjects** are up to three law subjects with an `active` or `regressed` leech, in
    the order the law board lists them, each once, read from SPEC-093's board filtered to the law
    rows before any limit: the law rows of SPEC-093's leech snapshot in the order SPEC-093 R1's
    board sorts them, never cut to its limit (`_law_weak_subjects` over
    `leeches.py:fetch_law_board`), and equal
    `goldens/stats_bridge_weak_subjects.json`. When the subject table fails (R6) the field is
    `null`, never `[]`: `[]` means measured, with no weakness.
R5. **The habits block** is, for each configured course in the courses file's order (the kernel's
    `Courses`), `code`, `letter` (the course's one-letter alias), `reading_minutes_week`,
    `reading_goal_min`, `reading_met`, `reading_streak_weeks`, `writing_tracked`,
    `writing_done_today` and `writing_streak_days`, with 0, `false` and `false` for a course the
    summary does not list, and the summary's `reading_goal_streak_weeks` and
    `writing_all_streak_days`; it equals `goldens/stats_bridge_habits_block.json`
    (`vault_bridge.py:_habits_block`).

Degradation

R6. Each later block is computed on its own, and a failure falls back to its cold shape and is
    named in `degraded`, in this order (`build_vault_stats`):
    - a failure of the subject table names `law_subjects`, leaves `law.weak_subjects` `null`, and
      names `drills` too, because unmatched drills cannot be counted without the table;
    - a failure of the drill backlog alone names `drills`;
    - a failure of the habits names `habits`;
    - a failure of the weeks names `weeks`, and the history and the trend then take their cold
      shape without being named;
    - with weeks read and not empty, a failure of the history or the trend names
      `weekly_history`; with no weeks, the history is `[]` and the trend is cold, unnamed;
    - a failure of the unopened projection gives `unopened` its cold shape
      (`unopened.py:cold_unopened_block`: `measured` false, `window_days` 90,
      `unclassified_unseen` `null`, `subjects` `[]`) and is never named in `degraded`.
    SPEC-121 R11 answers the weeks and the history as one block; this file keeps the predecessor's
    two names, so a weeks block that SPEC-121 marks unmeasured is named `weeks` here.
R7. **The signal.** `measured` maps each of `_DEGRADABLE_BLOCKS` to whether it is absent from
    `degraded`; `stats_ok` is true only when every value is true AND `degraded` is empty, so a
    name outside the map still fails it (`_apply_degradation_signal`, equal to
    `goldens/stats_bridge_degradation_signal.json`). If deriving the signal fails, every `measured`
    value is false and `stats_ok` is false (fail closed). A cold ledger's payload equals
    `goldens/stats_bridge_cold_payload.json`, `generated_utc` aside.

The unopened projection

R8. **The read** (`ingest::unopened`, `unopened.py:read_unopened_rows`) is SPEC-023 R1's
    read-only read of the copy under the shared collection lock, inside SPEC-023 R2's scope, and
    returns five sets: unseen-candidate counts by home deck (the original deck when the card is
    filtered), queue and type; each card's first study event (SPEC-023 R2's study-event rule); the
    distinct (home deck, study day) pairs of study events after the window's start, the study day
    by SPEC-020 R1; each deck's id, name and kind; and each deck preset's id and config. The
    window's start is 90 days of milliseconds before now, floored at 0
    (`window_start_ms`, equal to `goldens/stats_bridge_window_start.json`). An unreadable or
    missing copy is an error, never an empty read. With the include list unset it reads what the
    predecessor read; with it set, a deck outside the scope is not read, so its unseen cards count
    in no subject and not in `unclassified_unseen`.
R9. **The decoders** read the protobuf wire format of the deck's kind and the preset's config with
    no schema: the deck's preset id is field 1 of field 1 (`_decode_deck_config_id`, equal to
    `goldens/stats_bridge_deck_config_id.json`: no blob, no normal kind, or a malformed blob is
    unknown); the preset's new cards per day is field 9 (`_decode_new_per_day`, equal to
    `goldens/stats_bridge_new_per_day.json`: an absent field is 0, a missing blob or a malformed
    one is unknown). A deck's limit is unknown, known positive or known zero (`_classify_limit`).
R10. **The aggregate and the projection** (`analytics::unopened`, pure, no I/O):
    - a card is unseen when its queue is not suspended or buried (-1, -2, -3) and its type or its
      queue is new (0); a subject is `language/<code>` by SPEC-071's course of the deck name, else
      `law/<subject>` by SPEC-045's law subject, the bare law root excluded, else it counts in
      `unclassified_unseen` (`_classify_subject`); introductions after the window's start split
      at its midpoint (45 days) into the leading and trailing halves (`aggregate_subjects`, equal
      to `goldens/stats_bridge_unopened_aggregate.json`);
    - each subject answers exactly one of six outcomes, the gates in this order
      (`project_subject`, equal to `goldens/stats_bridge_unopened_projection.json`):
      `too_short` when its active days are under 14 (`MIN_INTAKE_DAYS`, threshold 14);
      `non_stationary` when the halves' ratio is over 1.75 (`STATIONARITY_MAX_RATIO`), the ratio
      being unmeasured when the smaller half is under 10 (`STATIONARITY_MIN_HALF_COUNT`), or, with
      one half empty, unmeasured under 20 introductions and infinite from 20; `limits_unknown` when
      the share of its decks with an unknown limit is over 0.5 (`UNKNOWN_LIMIT_MAX_SHARE`);
      `stalled` when its rate, introductions over active days, is at most 0.0
      (`STALLED_MIN_RATE`); `beyond_horizon` when the days to finish,
      `ceil(unseen / rate * 90 / active_days)`, pass the last date Python's `date` can hold
      (9999-12-31), the threshold being the days left to it; else `determined`, with the finish
      date and the days to it;
    - each refusal carries its reason, its governing constant as `threshold`, the observed rate
      (`null` with no active day) and the predecessor's `detail` text, byte for byte: the counts
      with thousands separators, the rate to two places as Python's `format` rounds it, the ratio
      as `<r>x` or `inf`, the share as a whole percentage;
    - the constants (`INTAKE_WINDOW_DAYS` 90, `MIN_INTAKE_DAYS` 14, `STATIONARITY_MAX_RATIO` 1.75,
      `STATIONARITY_MIN_HALF_COUNT` 10, `UNKNOWN_LIMIT_MAX_SHARE` 0.5, `STALLED_MIN_RATE` 0.0 and
      the six outcome names) equal `goldens/unopened.constants.json`.
R11. **The report and the block.** The report lists every subject sorted by its key, with
     `window_days` 90 and `unclassified_unseen` (`build_report`, equal to
     `goldens/stats_bridge_unopened_report.json`). The block adds `measured` true and renders
     each subject's rate rounded to four places, its dates as ISO dates
     (`unopened_block_to_dict`, equal to `goldens/stats_bridge_unopened_block.json`).
     `exam_date` and `days_before_exam` are always `null`: the predecessor's one caller passes no
     exam dates (§5).

The file and the job

R12. **The file** (`vault::stats`). Its path is `DECKSTREAK_VAULT_STATS_FILE`, relative to the
     vault root (SPEC-042 R1), example value `Dashboard/.deckstreak-stats.json`. The write
     refuses, writing nothing, with `not_configured` (the setting or the vault root unset),
     `root_missing` (the root is not a directory), `folder_missing` (the file's folder does not
     exist: it is never created, where the predecessor made it), `outside_root` (the path is
     absolute, has a `..` part, or resolves outside the root) and `rail_refused` (SPEC-042 R3).
     The text is `json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False)` plus a final
     newline, a float as Python's `repr` renders it, and equals
     `goldens/stats_bridge_file_text.json`. It lands through `vault::atomic::write`: SPEC-042 R2's
     temp `.<name>.<pid>.tmp` in the file's own folder, which ends in `.tmp` as the predecessor's
     temp name did, renamed over the file.
R13. **The job.** `coordination::jobs::TABLE` gains `vault_stats`,
     `Schedule::DailyAtRollover { minute: 12 }`, `catch_up` true (ADR-146): off every minute of
     the predecessor's schedule that SPEC-027 R2 names, off the sync's minute 7 and every other
     job's slot, and off every slot of the reserved-slot list the private deploy rail provides,
     which CI proves with a synthetic list, as SPEC-053 R2 does. Its timer
     `deploy/systemd/deck-streak-job@vault_stats.timer` is written at 04:12 UTC, the neutral zone,
     with `Persistent=true` and the job table's waiver line, and `deploy/rail-contract.json` gains
     its `OnCalendar` row. The drop-in
     `deploy/systemd/deck-streak-job@vault_stats.service.d/30-after-sync.conf` orders it
     `After=deck-streak-job@sync.service`, which holds a start back only while a start of the sync
     is pending, as SPEC-053 R3's generation unit is ordered. Write access reaches the unit only
     through the optional drop-in `deploy/optional/vault-stats/deck-streak-job@vault_stats.conf`
     (the vault's group, `UMask=0002`, and write access to the stats file's folder only, with a
     placeholder path), which the private rail (#41) installs at the item's switch, as SPEC-065 R3
     does for the readings folder.
R14. **The run.** The work (`coordination::vault_stats::StatsWork`) takes the collection lock in
     its shared mode, so that a sync still in flight ends first, then reads the study day's sync
     outcome (SPEC-027 R5), as SPEC-053 R3's generation does:
     - not synced: it composes nothing, writes nothing, keeps the file as it is, logs
       `vault_stats skipped reason=sync_not_succeeded` and returns `Done::Done`, which records the
       fire `ok` and pages no one (the sync's own failure has paged);
     - synced: it composes R1 to R11's payload, the unopened read inside the shared lock, and
       writes it (R12), logging `vault_stats written stats_ok=<bool> degraded=<n>`;
     - a first-block failure (R2) or a refused write returns `Err` with its reason, and the runner
       pages the first error of the streak and only logs a repeat (SPEC-027 R7).
R15. **The item.** `crates/coordination/data/cutover.json` gains `vault-stats`: `side` `moves`,
     `source` `vault_bridge.py:write_vault_stats`, `gates` [`job:vault_stats`], after
     `weekly-synthesis`. So SPEC-143 R10 holds every fire until the item switches, and the job
     writes nothing before the owner's go (#164). `deploy/cutover.md` lists the item's four
     commands (SPEC-143 R14); its verification counts the job's `cron_fires` since the switch, and
     the runbook names a night whose sync succeeded as the item's evidence, since R14's skipped
     night also records `ok`.
R16. **Wiring.** `Runner::run_job` gains the stats job's arm, handed the stats port that
     `crates/daemon/src/role_job.rs` builds from the environment: the vault's file writer and the
     ingest reader of the copy.
R17. The eleven anti-goals hold (CHARTER): the job grants no XP, sends no message, runs no sync,
     deletes no vault file, creates no folder, and writes one file only.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the file's key tree equals the cold payload golden's at every depth, and `contract` is 2 | `the_file_holds_the_contracts_keys_at_every_depth` |
| A2 | the stats constants equal the predecessor's | `the_stats_constants_equal_the_predecessors` |
| A3 | the weak subjects equal the golden, a fourth subject and a holding leech excluded | `the_weak_subjects_match_the_predecessors_golden` |
| A4 | the degradation signal equals the golden, a name outside the map failing `stats_ok` | `the_degradation_signal_matches_the_predecessors_golden` |
| A5 | the habits block equals the golden, a course with no summary entry included | `the_habits_block_matches_the_predecessors_golden` |
| A6 | a cold ledger's payload equals the golden, `generated_utc` aside | `a_cold_ledger_writes_the_predecessors_cold_payload` |
| A7 | each failed block is named as R6 orders, the table's failure naming `drills` and nulling the weak subjects, and a failed projection is cold and unnamed | `a_failed_block_is_named_and_carries_its_cold_shape` |
| A8 | `generated_utc` renders as Python's `isoformat`, with and without a fraction | `generated_utc_is_rendered_as_python_isoformat` |
| A9 | with the day's sync not succeeded, the run writes nothing and records `ok` | `no_file_is_written_when_the_days_sync_did_not_succeed` |
| A10 | a failed first block pages the first run of the streak and only logs the second | `a_failed_run_pages_once_per_streak` |
| A11 | a fire before the item's switch writes no file and no `cron_fires` row | `the_stats_job_is_held_until_its_item_switches` |
| A12 | the stats job's slot is minute 12 after the rollover, after the sync, off every predecessor minute and every other slot | `the_stats_job_fires_after_the_sync_on_a_free_minute` |
| A13 | the file's text equals the golden: sorted keys, indent 2, raw non-ASCII, a final newline | `the_file_text_matches_the_predecessors_golden` |
| A14 | the write lands through a temp in the file's own folder, and a failed rename leaves the file as it was | `the_stats_write_goes_through_a_temp_in_its_folder` |
| A15 | each of `not_configured`, `root_missing`, `folder_missing` and `outside_root` writes nothing and creates no folder | `each_stats_refusal_writes_nothing` |
| A16 | a payload the rails refuse is `rail_refused` and writes nothing | `a_rail_refusal_writes_no_stats_file` |
| A17 | the aggregate equals the golden | `the_unopened_aggregate_matches_the_predecessors_golden` |
| A18 | the projection equals the golden over every outcome and each gate's on-boundary case | `the_unopened_projection_matches_the_predecessors_golden` |
| A19 | the report equals the golden | `the_unopened_report_matches_the_predecessors_golden` |
| A20 | the block equals the golden, and the cold block is the predecessor's | `the_unopened_block_matches_the_predecessors_golden` |
| A21 | the unopened constants equal the predecessor's | `the_unopened_constants_equal_the_predecessors` |
| A22 | the window's start equals the golden | `the_window_start_matches_the_predecessors_golden` |
| A23 | the new-per-day decoder equals the golden | `the_new_per_day_decoder_matches_the_predecessors_golden` |
| A24 | the preset-id decoder equals the golden | `the_deck_config_id_decoder_matches_the_predecessors_golden` |
| A25 | the read of a synthetic copy answers its five sets, inside the scope and the study-event rule | `the_unopened_read_answers_its_five_sets` |
| A26 | a missing or unreadable copy is an error, never an empty read | `an_unreadable_copy_is_an_unopened_read_error` |
| A27 | the job role runs `vault_stats` with its work: with the stats file unset, the run exits 1 and writes nothing | `the_vault_stats_job_runs_its_work` |
| A28 | the optional drop-in grants write access to the stats file's folder only | `test_the_stats_drop_in_writes_only_its_folder` |
| A29 | over stub ports, each of R2's first blocks carries its port's value in its key, and a failed one fails the run with no file | `the_first_blocks_carry_their_ports_values` |
| A30 | over stub ports, each of R3's later blocks carries its port's value in its key | `the_later_blocks_carry_their_ports_values` |

```acceptance
A1: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_file_holds_the_contracts_keys_at_every_depth
A2: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_stats_constants_equal_the_predecessors
A3: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_weak_subjects_match_the_predecessors_golden
A4: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_degradation_signal_matches_the_predecessors_golden
A5: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_habits_block_matches_the_predecessors_golden
A6: cargo test -p deck-streak-coordination --test vault_stats -- --exact a_cold_ledger_writes_the_predecessors_cold_payload
A7: cargo test -p deck-streak-coordination --test vault_stats -- --exact a_failed_block_is_named_and_carries_its_cold_shape
A8: cargo test -p deck-streak-coordination --test vault_stats -- --exact generated_utc_is_rendered_as_python_isoformat
A9: cargo test -p deck-streak-coordination --test vault_stats -- --exact no_file_is_written_when_the_days_sync_did_not_succeed
A10: cargo test -p deck-streak-coordination --test vault_stats -- --exact a_failed_run_pages_once_per_streak
A11: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_stats_job_is_held_until_its_item_switches
A12: cargo test -p deck-streak-coordination --test job_table -- --exact the_stats_job_fires_after_the_sync_on_a_free_minute
A13: cargo test -p deck-streak-vault --test stats_file -- --exact the_file_text_matches_the_predecessors_golden
A14: cargo test -p deck-streak-vault --test stats_file -- --exact the_stats_write_goes_through_a_temp_in_its_folder
A15: cargo test -p deck-streak-vault --test stats_file -- --exact each_stats_refusal_writes_nothing
A16: cargo test -p deck-streak-vault --test stats_file -- --exact a_rail_refusal_writes_no_stats_file
A17: cargo test -p deck-streak-analytics --test unopened -- --exact the_unopened_aggregate_matches_the_predecessors_golden
A18: cargo test -p deck-streak-analytics --test unopened -- --exact the_unopened_projection_matches_the_predecessors_golden
A19: cargo test -p deck-streak-analytics --test unopened -- --exact the_unopened_report_matches_the_predecessors_golden
A20: cargo test -p deck-streak-analytics --test unopened -- --exact the_unopened_block_matches_the_predecessors_golden
A21: cargo test -p deck-streak-analytics --test unopened -- --exact the_unopened_constants_equal_the_predecessors
A22: cargo test -p deck-streak-analytics --test unopened -- --exact the_window_start_matches_the_predecessors_golden
A23: cargo test -p deck-streak-ingest --test unopened_read -- --exact the_new_per_day_decoder_matches_the_predecessors_golden
A24: cargo test -p deck-streak-ingest --test unopened_read -- --exact the_deck_config_id_decoder_matches_the_predecessors_golden
A25: cargo test -p deck-streak-ingest --test unopened_read -- --exact the_unopened_read_answers_its_five_sets
A26: cargo test -p deck-streak-ingest --test unopened_read -- --exact an_unreadable_copy_is_an_unopened_read_error
A27: cargo test -p deck-streak-daemon --test roles -- --exact the_vault_stats_job_runs_its_work
A28: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_stats_drop_in_writes_only_its_folder
A29: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_first_blocks_carry_their_ports_values
A30: cargo test -p deck-streak-coordination --test vault_stats -- --exact the_later_blocks_carry_their_ports_values
```

The Rust tests build their vault in a temporary directory and their copy from a synthetic
collection; the coordination tests feed each port a stub, and none reads a real collection.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/vault_stats.rs` | `deck-streak-coordination` | added: the payload's composition, R4, R5, the degradation signal (R6, R7) and `StatsWork` (R14) |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: `VAULT_STATS` joins the table (R13) |
| `crates/coordination/src/runner.rs` | `deck-streak-coordination` | changed: `run_job`'s stats arm (R16) |
| `crates/coordination/data/cutover.json` | `deck-streak-coordination` | changed: the `vault-stats` item (R15) |
| `crates/coordination/Cargo.toml` | `deck-streak-coordination` | changed: serde_json's `float_roundtrip` as a dev-dependency feature, because the payload goldens' floats are compared bit for bit |
| `crates/coordination/tests/vault_stats.rs` | `deck-streak-coordination` | added: A1-A11, A29, A30 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A12, and `vault_stats` joins the claimed jobs |
| `crates/vault/src/stats.rs` | `deck-streak-vault` | added: the stats file's write and its refusals (R12) |
| `crates/vault/src/config.rs` | `deck-streak-vault` | changed: `VAULT_STATS_FILE` |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the module |
| `crates/vault/Cargo.toml` | `deck-streak-vault` | changed: serde_json's `float_roundtrip` as a dev-dependency feature, for the file-text golden |
| `crates/vault/tests/stats_file.rs` | `deck-streak-vault` | added: A13-A16 |
| `crates/analytics/src/unopened.rs` | `deck-streak-analytics` | added: the aggregate, the projection, the report and the block (R10, R11) |
| `crates/analytics/src/lib.rs` | `deck-streak-analytics` | changed: the module |
| `crates/analytics/tests/unopened.rs` | `deck-streak-analytics` | added: A17-A22 |
| `crates/ingest/src/unopened.rs` | `deck-streak-ingest` | added: the read and the two decoders (R8, R9) |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/unopened_read.rs` | `deck-streak-ingest` | added: A23-A26 |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the stats port's wiring (R16) |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A27 |
| `deploy/systemd/deck-streak-job@vault_stats.timer` | deploy | added: the timer (R13) |
| `deploy/systemd/deck-streak-job@vault_stats.service.d/30-after-sync.conf` | deploy | added: the ordering after the sync (R13) |
| `deploy/optional/vault-stats/deck-streak-job@vault_stats.conf` | deploy | added: the write access the rail installs at the switch (R13) |
| `deploy/rail-contract.json` | deploy | changed: the timer's `OnCalendar` row |
| `deploy/cutover.md` | deploy | changed: the `vault-stats` item's commands (R15) |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A28 |
| `.env.example` | repo | changed: `DECKSTREAK_VAULT_STATS_FILE` with its example value |
| `tools/parity-oracle/registry/spec_146.py` | repo | added: §7's goldens |
| `tools/parity-oracle/goldens/stats_bridge_*.json`, `stats_bridge.constants.json`, `unopened.constants.json` | repo | added: §7's goldens |
| `scripts/mutation-rows.d/S14600-S14699.json` | repo | added: §9's rows |
| `docs/schematics/cutover-checklist-and-sequence.md` | docs | added by the W8 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/schematics/vault-stats-bridge.md` | docs | added by the W8 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-146.md` | docs | added: the red-first record |
| `docs/specs/SPEC-146-the-nightly-stats-file-keeps-the-predecessors-contract-and-is-written-after-the-days-one-sync-by-one-writer.md` | docs | moved from `docs/specs/planned/` |
| `changelog.d/feat-vault-stats-bridge.md` | repo | added: the fragment |

## 5. What this does NOT do

- It frames no projection against an exam date: the predecessor's `unopened.py:build_report`
  takes exam dates and its one caller passes none, so `exam_date` and `days_before_exam` are always
  `null`; the framing is inert in v9, and reviving it is the owner's decision (#390).
- It runs no sync and waits on none: it reads the study day's one sync outcome (ADR-037, #153).
- It creates no folder: a missing stats folder is refused, and the owner makes it (#153).
- It builds none of the ports it reads: the streaks (#81), the law block (#134), the habits
  summary (#95), the leech board (#133), the drills (#136) and the law progress (#55).
- It writes nothing before the owner's go: the item switches only on the checklist (#62, #164).

## 6. Risks

- **A failed block read as a measured zero.** Prevented by R6 and R7's signal and its floor, and
  detected by A4, A7 and rows S14602 and S14604.
- **A snapshot of the day before the sync.** Prevented by R13's minute and R14's outcome read,
  and detected by A9, A12 and row S14610.
- **Two writers of the file.** Prevented by R15's item behind SPEC-143 R10's gate, and detected by
  A11 (SPEC-143's own row guards the gate).
- **A folder made where the vault has none.** Prevented by R12's `folder_missing`, and detected by
  A15 and row S14609.
- **A projection that drifts from the predecessor's** at a gate's boundary. Prevented by R10's
  constants and golden cases, and detected by A18 and rows S14605 to S14608.
- **A file the consumer cannot diff**, keys unsorted. Detected by A13 and row S14611.
- **A skipped night counted as a move's evidence.** R14 records a skipped night `ok`; R15's
  runbook names a night whose sync succeeded.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_146.py` (SPEC-029's house shape), generated on
the predecessor at `27ee2bc`, every case synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `stats_bridge.constants` | `vault_bridge.py` constants | constants | `STATS_CONTRACT_VERSION`, `LAW_SUBJECT_OMITTED`, `WEEKS_OMITTED`, `_DEGRADABLE_BLOCKS` |
| `stats_bridge_weak_subjects` | `vault_bridge.py:_law_weak_subjects` | function | boards with language rows, holding and resolved law rows, a repeated subject, an empty subject, and four weak subjects |
| `stats_bridge_degradation_signal` | `vault_bridge.py:_apply_degradation_signal` | function | `degraded` lists: empty, each block alone, `law_subjects` with `drills`, and a name outside the map |
| `stats_bridge_habits_block` | `vault_bridge.py:_habits_block` | adapter | a stub pipeline whose summary lists synthetic courses, one with no reading entry and one with no writing entry, the module's course codes and aliases patched to synthetic ones |
| `stats_bridge_cold_payload` | `vault_bridge.py:build_vault_stats` | adapter | a stub pipeline over an empty temporary store and no collection, `generated_utc` removed |
| `stats_bridge_file_text` | `vault_bridge.py:write_vault_stats` | adapter | a temporary root and payloads with nested keys out of order, non-ASCII text, `null`, the floats 0.0001, 3.0 and 0.1235; it records the file's text |
| `stats_bridge_window_start` | `unopened.py:window_start_ms` | function | now at 0, inside the first 90 days, on the 90-day boundary, and after it |
| `stats_bridge_new_per_day` | `unopened.py:_decode_new_per_day` | function | no blob, an empty blob, field 9 as 0, as 20 and as a multi-byte varint, field 9 absent, a truncated varint |
| `stats_bridge_deck_config_id` | `unopened.py:_decode_deck_config_id` | function | no blob, a filtered kind, a normal kind with and without field 1, a truncated blob |
| `stats_bridge_unopened_aggregate` | `unopened.py:aggregate_subjects` | adapter | synthetic raw rows: suspended and buried cards, a new card in a learning queue, a filtered card's original deck, a language deck, a law subject, the bare law root, an unclassified deck, introductions on the window's start and its midpoint |
| `stats_bridge_unopened_projection` | `unopened.py:project_subject` | function | active days 13 and 14; halves 9 and 10; one empty half at 19 and 20 introductions; ratios 1.75 and 1.80; unknown shares 0.5 and 0.75; a rate of 0; a rate of 0.125 (a tie at two places); a finish past 9999-12-31; days to finish exact and fractional |
| `stats_bridge_unopened_report` | `unopened.py:build_report` | adapter | synthetic raw rows over three subjects with `new_limit_reader`, and `exam_dates` omitted as its one caller omits it |
| `stats_bridge_unopened_block` | `unopened.py:unopened_block_to_dict` and `cold_unopened_block` | adapter | the report golden's reports, a rate needing rounding to four places, and the cold block |
| `unopened.constants` | `unopened.py` constants | constants | the six tunables and the six outcome names |

## 8. Tables and the v9 import

No table. The file is the vault's, and the job's fires are rows of SPEC-027's `cron_fires`.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S14601-CONTRACT-2` | `crates/coordination/src/vault_stats.rs` | the contract version is 2 | `vault_stats::the_file_holds_the_contracts_keys_at_every_depth` |
| `S14602-STATS-OK-FLOOR` | `crates/coordination/src/vault_stats.rs` | `stats_ok` needs an empty `degraded` | `vault_stats::the_degradation_signal_matches_the_predecessors_golden` |
| `S14603-WEAK-THREE` | `crates/coordination/src/vault_stats.rs` | at most three weak subjects | `vault_stats::the_weak_subjects_match_the_predecessors_golden` |
| `S14604-DRILLS-CASCADE` | `crates/coordination/src/vault_stats.rs` | a failed table names `drills` | `vault_stats::a_failed_block_is_named_and_carries_its_cold_shape` |
| `S14605-MIN-INTAKE` | `crates/analytics/src/unopened.rs` | under 14 active days is `too_short` | `unopened::the_unopened_projection_matches_the_predecessors_golden` |
| `S14606-STATIONARITY` | `crates/analytics/src/unopened.rs` | a ratio over 1.75 is refused | `unopened::the_unopened_projection_matches_the_predecessors_golden` |
| `S14607-UNKNOWN-SHARE` | `crates/analytics/src/unopened.rs` | a share over 0.5 is refused | `unopened::the_unopened_projection_matches_the_predecessors_golden` |
| `S14608-STALLED` | `crates/analytics/src/unopened.rs` | a rate of at most 0 is `stalled` | `unopened::the_unopened_projection_matches_the_predecessors_golden` |
| `S14609-NO-FOLDER` | `crates/vault/src/stats.rs` | a missing folder is refused, never made | `stats_file::each_stats_refusal_writes_nothing` |
| `S14610-SYNC-FIRST` | `crates/coordination/src/vault_stats.rs` | no write without the day's sync | `vault_stats::no_file_is_written_when_the_days_sync_did_not_succeed` |
| `S14611-SORTED-KEYS` | `crates/vault/src/stats.rs` | the keys are written sorted | `stats_file::the_file_text_matches_the_predecessors_golden` |

No row's mutant makes anything wait or loop. Each boundary row's killer is a golden whose case
list names the on-boundary case (14 active days, a ratio of exactly 1.75, a share of exactly 0.5,
a rate of exactly 0). No killer calls a network: the vault is a temporary directory, the copy is
synthetic, and every port is a stub.

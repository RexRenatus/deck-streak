# SPEC-128: a refused owner sync is recorded, answered and not retried

- **Wave:** W2. **Issue:** #323 (epic #3). **Context(s):** `deck-streak-ingest` (the record, beside
  the pending flag), `deck-streak-daemon` (the job writes it and the bot's port reads it),
  `deck-streak-bot` (its existing reply carries the answer).
- **Decided by:** ADR-128 (the refusal lives on the request's own state); ADR-066 (the owner's
  request is a stored flag) and ADR-037 (one scheduled sync per study day plus the owner's
  triggers) hold.
- **Mutation band:** `S12800-S12899`.
- **Status:** delivered (SPEC-059 gains an insert-only amendment section at its end that points
  here, and its section 5 bullet and section 6 risk stand as the record of the state before it).

## 1. The problem, measured

- **A refused request is only logged.** `serve_owner_request` in `crates/daemon/src/role_job.rs`
  has two refusal arms, the recompute's setup failing to load and the owner cycle returning a
  `SyncRefusal`; each writes one log line and nothing else.
- **The owner is told "still running".** `SqliteRequestLedger::progress` answers `Waiting` while
  `ingest_state.rescore_pending` is set, and only the recompute's `write_anchor` clears it, so a
  refusal leaves it set and `/sync` answers `StillRunning` at the bound (SPEC-059 section 6).
- **Every job run retries it.** The flag stays set, so each run of the sync job runs the owner
  cycle again and is refused again, until the cause is fixed and a recompute clears the flag.
- **The refusal codes exist already.** `OwnerSyncCycle::run` returns a `SyncRefusal` whose
  `reason` is one of `rescore_unrecorded`, `sync_settings_refused`, `credentials_directory_refused`,
  `scope_settings_refused`, `sync_record_failed`, `obligations_unreadable` and `recompute_failed`;
  the scheduled path names the recompute setup's failure `recompute_refused`.

## 2. Requirements

R1. When the job cannot load the recompute's setup for the owner's request, or the owner cycle
    refuses it, ONE write records the refusal's reason and the instant and clears the pending
    flag (`SqliteIngestState::record_refusal`). The reason is one of eight codes, a closed set
    enforced by the type (`RefusalReason`) and by a `CHECK` in the migration: the seven cycle
    codes above and `recompute_refused` for the recompute's setup, which is the scheduled path's
    own name for that failure. No error text, path, endpoint or credential is stored.
R2. A job run after a refusal runs no owner cycle for it, because the flag is clear; the scheduled
    run goes on as before.
R3. A new request clears the refused record in the same write that sets the flag
    (`request_rescore`), so an old refusal never answers a new request.
R4. `progress(since)` answers the new `Progress::Refused { reason }` for a refusal recorded at or
    after `since`, and ignores a refusal recorded before `since`. A pending request still answers
    `Waiting`, and a refusal is answered before any run.
R5. The bot's answer to the owner names the refusal and never says "still running": a refused
    request is answered `SyncOutcome::NotRun { reason }` with `Scores::Unchanged`, which the bot's
    existing reply words as "No sync ran" with the code. It does not flush the router (SPEC-059 R8
    flushes only after a sync that ran and succeeded). The wording is the existing one because a
    sentence per reason would put the names of internal settings into a chat message, while the
    code is what the owner reports and what a log line carries.
R6. Data rights: `ingest_state` is exported and reset in place (SPEC-023 R13), so the two new
    columns are exported with the row and cleared to NULL by an erase, as the anchor and the
    window's base are.

Columns (migration `migrations/012801_ingest_refused_owner_request.sql`): `refused_at INTEGER`
and `refused_reason TEXT`, both NULL when no refusal stands; the reason `CHECK` allows NULL or one
of the eight codes, and pairs with `refused_at` (both NULL or both set).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the migration refuses a reason outside the closed set and an instant without a reason | `cargo test -p deck-streak-ingest --test refusal -- --exact the_migration_refuses_a_reason_outside_the_closed_set` |
| A2 | a refusal stores its reason and instant and clears the pending flag in one write | `cargo test -p deck-streak-ingest --test refusal -- --exact a_refusal_stores_its_reason_and_instant_and_clears_the_flag` |
| A3 | a new request clears the refused record | `cargo test -p deck-streak-ingest --test refusal -- --exact a_new_request_clears_the_refused_record` |
| A4 | every one of the eight reasons round trips through its code and an unknown code is refused | `cargo test -p deck-streak-ingest --test refusal -- --exact every_reason_round_trips_through_its_code` |
| A5 | the store answers `Refused` for a refusal at or after the request and ignores an older one | `cargo test -p deck-streak-daemon --test sync_request -- --exact the_store_answers_a_refusal_at_or_after_the_request_only` |
| A6 | a refused request is answered with its reason, not `StillRunning`, and never flushes | `cargo test -p deck-streak-daemon --test sync_request -- --exact a_refused_request_is_answered_with_its_reason_and_never_flushes` |
| A7 | the sync job records a cycle refusal and the next run does not retry it | `cargo test -p deck-streak-daemon --test roles -- --exact a_refused_owner_request_is_recorded_and_the_next_run_does_not_retry_it` |
| A8 | the sync job records a recompute setup refusal | `cargo test -p deck-streak-daemon --test roles -- --exact a_refused_recompute_setup_is_recorded_for_the_owner` |
| A9 | the refused record is exported and an erase clears it | `cargo test -p deck-streak-ingest --test data_rights -- --exact the_refused_record_is_exported_and_an_erase_clears_it` |

```acceptance
A1: cargo test -p deck-streak-ingest --test refusal -- --exact the_migration_refuses_a_reason_outside_the_closed_set
A2: cargo test -p deck-streak-ingest --test refusal -- --exact a_refusal_stores_its_reason_and_instant_and_clears_the_flag
A3: cargo test -p deck-streak-ingest --test refusal -- --exact a_new_request_clears_the_refused_record
A4: cargo test -p deck-streak-ingest --test refusal -- --exact every_reason_round_trips_through_its_code
A5: cargo test -p deck-streak-daemon --test sync_request -- --exact the_store_answers_a_refusal_at_or_after_the_request_only
A6: cargo test -p deck-streak-daemon --test sync_request -- --exact a_refused_request_is_answered_with_its_reason_and_never_flushes
A7: cargo test -p deck-streak-daemon --test roles -- --exact a_refused_owner_request_is_recorded_and_the_next_run_does_not_retry_it
A8: cargo test -p deck-streak-daemon --test roles -- --exact a_refused_recompute_setup_is_recorded_for_the_owner
A9: cargo test -p deck-streak-ingest --test data_rights -- --exact the_refused_record_is_exported_and_an_erase_clears_it
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `migrations/012801_ingest_refused_owner_request.sql` | `deck-streak-ingest` | added: the two columns and their `CHECK` |
| `crates/ingest/src/state.rs` | `deck-streak-ingest` | changed: `RefusalReason`, `Refusal`, `record_refusal`, the load, and `request_rescore` clearing the record |
| `crates/ingest/src/data_rights.rs` | `deck-streak-ingest` | changed: the export and the reset carry the two columns (R6) |
| `crates/ingest/tests/refusal.rs` | `deck-streak-ingest` | added: A1 to A4 |
| `crates/ingest/tests/data_rights.rs` | `deck-streak-ingest` | changed: A9 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: the seed moves the two columns off their reset value |
| `crates/daemon/src/sync_request.rs` | `deck-streak-daemon` | changed: `Progress::Refused`, the store's read and the answer |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: both refusal arms record (R1) |
| `crates/daemon/tests/sync_request.rs` | `deck-streak-daemon` | changed: A5, A6 |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A7, A8 |
| `.sqlx/` | `deck-streak-ingest` | changed: the refreshed query cache |
| `privacy.json` | privacy | changed: the `ingest-state` entry names the refused request |
| `docs/CONTEXT-MAP.md` | docs | changed: the `ingest_state` row names the refusal in the reset |
| `docs/specs/SPEC-059-*.md` | docs | changed: an insert-only amendment section at its end |
| `docs/specs/SPEC-128-*.md`, `docs/decisions/ADR-128-*.md`, `docs/schematics/refused-owner-request.md`, `docs/red-first/SPEC-128.md`, `changelog.d/feat-refused-sync-128.md`, `scripts/mutation-rows.d/S12800-S12899.json` | docs and tests | added |

ADR-066 is not edited. The bot crate changes no file: its reply for `NotRun` is the existing one.

## 5. What this does NOT do

- It does not widen SPEC-022 R9's closed set of sync failure codes, and writes no `sync_runs` row
  for a refusal, because a refusal is not a sync (#323).
- It does not retry a refused request on the next job run, and adds no backoff or retry count (#323).
- It does not tell the owner why in a sentence per reason: the reply carries the code (#323).
- It does not change the scheduled run, its reasons or its paging (#323).

## 6. Risks

- **A transient refusal drops the request.** A database error inside the cycle
  (`sync_record_failed`, `recompute_failed`) clears the flag too, so the owner sends `/sync`
  again; the answer names the code, and the daily timer still serves the next study day.
- **A refusal lands after a newer request.** Two requests can overlap a running job: the second
  request clears the record and sets the flag, then the first request's refusal clears the flag
  and records itself at a later instant, so it answers both. Both were refused by the same job
  for the same setting, so the answer is right; the record never answers a request made after
  its own instant, because `progress` reads only a refusal at or after `since`.
- **A code added to the cycle without the set.** The type refuses an unknown code and A4 walks the
  set; the migration `CHECK` refuses it as a second line (A1).

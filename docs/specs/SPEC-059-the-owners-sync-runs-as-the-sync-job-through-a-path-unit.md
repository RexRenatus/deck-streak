# SPEC-059: the owner's /sync runs as the sync job, started by a path unit, and the bot runs no sync

- **Wave:** W2. **Issue:** #286 (epic #3). **Context(s):** `deck-streak-daemon` (the bot's port and
  the job's entry), `deck-streak-ingest` (one read of the owner's last run), `deck-streak-bot`
  (one reply).
- **Decided by:** ADR-066 (the doorbell is a path unit and the request is a stored flag); ADR-037
  (one scheduled sync per study day plus the owner's triggers) and ADR-026 (the owner gate) hold.
- **Mutation band:** `S05900-S05999`.
- **Status:** delivered (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-059.md`, ADR-016).

## 1. The problem, measured

- **A sync needs more memory than the bot's limit.** The bot's unit caps memory at 96M
  (`MemoryMax=` in `deploy/systemd/deck-streak-bot.service`); the job unit is sized for a sync
  (`MemoryMax=384M` in `deck-streak-job@.service`). One sync of the owner's collection needs more
  than the bot's limit (measured on the maintainer's side, a figure kept private).
- **The bot runs the cycle today.** `crates/daemon/src/wiring.rs` implements the bot's `OwnerSync`
  port with `OwnerSyncCycle`, which builds the engine and calls `sync_cycle` in the bot's process;
  `crates/daemon/src/role_bot.rs` passes it to `Commands`. On the first deploy that kills the bot
  mid-sync.
- **The job cannot serve an owner request as it stands.** `crates/coordination/src/runner.rs`
  claims a scheduled fire, so `deckstreakd job sync` started off schedule finds its fire claimed
  and does nothing. `$TRIGGER_PATH` cannot tell the job why it started: `systemd.exec(5)` calls it
  lossy and best-effort and says it should not be relied upon.
- **The pending flag already exists.** `SqliteIngestState::request_rescore` sets
  `ingest_state.rescore_pending`, and `recomputed()` clears it in the same write as the anchor, so
  a cleared flag means the cycle completed (`crates/coordination/src/sync_cycle.rs`).

## 2. Requirements

R1. The bot's `/sync` port (`SyncRequester`) runs no sync cycle: `role_bot.rs` names neither
    `OwnerSyncCycle`, `sync_cycle` nor `RslibEngine`, and `OwnerSyncCycle` no longer implements
    `OwnerSync`.
R2. The request is a trigger only. The port records the owner's request in the store
    (`request_rescore`, behind the owner gate) and touches one file; the job reads the stored flag
    and nothing from the file, so a planted file or payload changes nothing.
R3. A path unit `deck-streak-job@sync` (path unit) with `PathChanged=` on the request file starts
    `deck-streak-job@sync` (service unit) and loads no credential. The request directory is created by a
    tmpfiles line, owned by the service user, mode `0700`; only the bot unit has write access to it
    (`ReadWritePaths=`); the job unit has none.
R4. Requests during a running sync start at most one more run (`PathChanged=` is edge triggered),
    the port rings at most once per `RING_GAP_SECS` (15 s, under the job unit's start limit), and
    R17's reuse window (`OWNER_SYNC_DEBOUNCE_SECS`, 300 s) still answers: a request inside it runs
    no second cycle and is answered `Reused`.
R5. The owner always gets an answer. The port waits at most `ANSWER_BOUND_SECS` (120 s), polling
    every `POLL_SECS` (2 s) on an injected clock, and answers the outcome (`Synced`, `Failed`,
    `Reused`) or `StillRunning`, a reply that says so. It never returns no reply.
R6. When the store holds a pending flag, `deckstreakd job sync` runs the owner's cycle
    (`Trigger::Owner`) first, then the scheduled run, which stays claimed once per study day
    (ADR-037). No timer, calendar or schedule is added. Only the owner reaches the port (ADR-026,
    unchanged).
R7. The bot unit loses the sync login (`anki-sync-*` credentials), which only the job needs.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the port requests the job and never calls the cycle | `cargo test -p deck-streak-daemon --test sync_request -- --exact the_bot_port_requests_the_job_and_never_runs_the_cycle` |
| A2 | a planted request payload changes nothing; only the stored flag starts an owner cycle, decided by running the job | `cargo test -p deck-streak-daemon --test roles -- --exact two_planted_request_payloads_change_nothing_the_sync_job_serves` |
| A3 | the path unit's exact watch set, the directory's owner and mode, and the bot-only write access by a census of every unit | `python3 -m unittest discover -s scripts/tests -p test_sync_path.py` |
| A4 | a second request inside the gap rings no second time | `cargo test -p deck-streak-daemon --test sync_request -- --exact a_second_request_waits_out_the_ring_gap` |
| A4 | a request inside the reuse window is answered `Reused` | `cargo test -p deck-streak-daemon --test sync_request -- --exact a_request_inside_the_reuse_window_is_answered_reused` |
| A5 | the owner is answered by the bound, on an injected clock | `cargo test -p deck-streak-daemon --test sync_request -- --exact the_owner_is_answered_within_the_bound` |
| A6 | only the owner reaches the port | `cargo test -p deck-streak-bot --test gate -- --exact an_update_from_anyone_but_the_owner_is_dropped_without_a_reply` |
| A6 | the job table keeps one daily sync slot | `cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day` |

```acceptance
A1: cargo test -p deck-streak-daemon --test sync_request -- --exact the_bot_port_requests_the_job_and_never_runs_the_cycle
A2: cargo test -p deck-streak-daemon --test roles -- --exact two_planted_request_payloads_change_nothing_the_sync_job_serves
A3: python3 -m unittest discover -s scripts/tests -p test_sync_path.py
A4: cargo test -p deck-streak-daemon --test sync_request -- --exact a_second_request_waits_out_the_ring_gap
A4: cargo test -p deck-streak-daemon --test sync_request -- --exact a_request_inside_the_reuse_window_is_answered_reused
A5: cargo test -p deck-streak-daemon --test sync_request -- --exact the_owner_is_answered_within_the_bound
A6: cargo test -p deck-streak-bot --test gate -- --exact an_update_from_anyone_but_the_owner_is_dropped_without_a_reply
A6: cargo test -p deck-streak-coordination --test job_table -- --exact the_job_table_holds_sync_to_one_daily_slot_claimed_per_study_day
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/daemon/src/sync_request.rs` | `deck-streak-daemon` | added: `SyncRequester`, the ring, the wait |
| `crates/daemon/src/lib.rs` | `deck-streak-daemon` | changed: one module line |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the cycle is no bot port |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: builds the requester |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the owner cycle first when pending |
| `crates/daemon/tests/sync_request.rs` | `deck-streak-daemon` | added: the port, the ring, the wait and the doorbell's file |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: only the sync job serves the stored request, and two planted payloads change nothing (A2) |
| `crates/daemon/tests/role_bot.rs` | `deck-streak-daemon` | changed: the bot answers a request |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the `StillRunning` outcome and reply |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: the `StillRunning` reply is sent |
| `crates/bot/tests/messages/sync-still-running.msg.json` | `deck-streak-bot` | added: the golden for that reply |
| `crates/ingest/src/sync_runs.rs` | `deck-streak-ingest` | changed: `owner_run_since` |
| `crates/ingest/tests/owner_run.rs` | `deck-streak-ingest` | changed: the owner's latest run since an instant is read |
| `.sqlx/` | `deck-streak-ingest` | changed: the refreshed query cache |
| `scripts/tests/test_deploy_templates.py` | tests | changed: the bot reads no sync credential |
| `deploy/systemd/deck-streak-job@sync (path unit)` | deploy | added |
| `deploy/systemd/deck-streak-bot.service` | deploy | changed: request directory, no sync login |
| `deploy/tmpfiles.d/deck-streak-sync-request.conf` | deploy | added |
| `deploy/README.md`, `deploy/deck-streak.env.example` | deploy | changed: the units, the request directory and the first-deploy order |
| `scripts/tests/test_sync_path.py` | tests | added |
| `scripts/mutation-rows.d/S05900-S05999.json` | tests | added |
| `docs/specs/SPEC-059-*.md`, `docs/decisions/ADR-066-*.md`, `docs/schematics/sync-request-doorbell.md`, `docs/red-first/SPEC-059.md`, `changelog.d/feat-sync-path-059.md` | docs | added |

## 5. What this does NOT do

- It adds no schedule and no timer: ADR-037's one scheduled sync is unchanged (#286).
- It does not start units from the bot and adds no polkit or sudo rule (#286).
- It does not move the sync login into a drop-in; SPEC-062 owns that (#286).
- It does not install any unit: the first deploy does (#286).

## 6. Risks

- The path unit cannot be run without the host's service manager; A3 pins its text and the box run
  judges the rest. `systemd.path(5)` is the cited semantics.
- A change written while a sync runs can be missed by the edge trigger; the stored flag stays
  pending, so the next request or the daily timer serves it, and the owner's wait answers
  `StillRunning`.
- A refusal on the job side (a recompute load error, or a cycle refusal such as a malformed sync
  scope) leaves the request pending too, so the owner is told the sync is still running until the
  answer bound and the next request or the daily timer serves it again. The ledger records no
  refused owner run, and this delivery adds no table for one.

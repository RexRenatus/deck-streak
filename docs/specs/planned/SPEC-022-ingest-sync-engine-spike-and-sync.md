# SPEC-022: a measured spike settles the Anki engine, and ingest syncs a private copy that never uploads

- **Wave:** W0. **Issue:** #15 (epic #1). **Context(s):** `deck-streak-ingest`, `deck-streak-coordination` (the sync cycle use case).
- **Decided by:** ADR-009 (the engine, proposed until this spike), ADR-008 (the private copy, read-only reads), ADR-037 (one scheduled sync per study day plus the owner's triggers, and never an upload), ADR-038 (credentials from the secret manager at unit start, superseding ADR-010's storage), ADR-012 (goldens), ADR-018 (licence compatibility), and this SPEC's ADR-022 (the spike's fixed protocol and budgets).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-022.md` (ADR-016).

## 1. The problem, measured

- **The decision this settles.** ADR-009 proposes Anki's own Rust engine (the `anki` crate in
  Anki's `rslib`, AGPL-3.0-or-later like DeckStreak) as a pinned git dependency of `ingest`, keeps
  a minimal Python sidecar running the predecessor's proven sync as the fallback, and makes the
  choice depend on a measurement nobody has taken: build time and binary size in CI, and resident
  memory while opening a large collection and resolving its new-card queue.
- **The budgets are fixed before the measurement** (ADR-022), so the numbers decide rather than
  justify. At promotion this section gains the measured table: cold build minutes, stripped binary
  MiB, peak RSS MiB for open and queue, peak RSS MiB for a full download, incremental-sync seconds.
- **The predecessor's sync, which the port keeps** (predecessor `27ee2bc`, names only):
  `sync.py:AnkiSyncer.sync_now` and `_sync_blocking` log in and call the engine's collection sync
  with media off; a full-sync demand is met by a full DOWNLOAD under a cross-process lock
  (`sync.py:AnkiSyncer._swap_lock`); an empty server is refused as `full_upload_required` so a
  populated copy is never wiped; `pipeline.py:GamifyPipeline._sync_attempts` retries with an
  exponential backoff and jitter under a per-attempt timeout (`constants.py:SYNC_RETRY_ATTEMPTS`,
  `SYNC_RETRY_JITTER_FRAC`, `SYNC_TIMEOUT_SECS`); `pipeline.py:_bounded_sync_error` keeps only a
  bounded code, never exception text; `sync.py:classify_open_error` retries only a locked
  collection (`COLLECTION_OPEN_RETRIES`). The cadence is 15 minutes plus once before each
  notification job.
- **DeckStreak's rule is not the predecessor's cadence (ADR-037).** DeckStreak reuses the
  predecessor's Anki login, which the owner approved at gate 6 on testable conditions: no upload
  path, proven against a server that records every request, and at most one scheduled sync per study
  day plus the owner's explicit triggers, so DeckStreak never contends with the predecessor's sync or
  loads the owner's server. The scheduled sync runs once per study day, at the rollover hour, minute
  7 (SPEC-027); the owner's trigger is the bot's `/sync` (SPEC-026); no other job syncs.
- **What the parity oracle proves.** The retry schedule, against `goldens/sync_retry.json` (an
  adapter over `pipeline.py:GamifyPipeline._sync_attempts` with a failing stub syncer, a recording
  sleep and a fixed jitter draw); the constants, against `goldens/sync.constants.json`. Both are
  registered in `tools/parity-oracle/registry/spec_022.py`.
- **Nothing exists yet**: `crates/ingest/src/lib.rs` is documentation only (read at `main`
  e05dfa5).

**Order.** After SPEC-020 (settings, credentials, `Db`, `Offload`, the data-rights port) and
SPEC-029 (goldens). The spike is this delivery's FIRST commit: if a budget fails, the delivery
stops after recording the numbers (A1 to A3), ADR-009 is marked superseded by a new ADR the
architect numbers, and this SPEC is amended for the sidecar before A4 onward is built. SPEC-023
(the read and the change gate, same crate) and SPEC-027 (which schedules this sync) land after it.

## 2. Requirements

The spike (settles ADR-009)

R1. The engine is added to `ingest` only, as `anki` from Anki's repository pinned to the release tag
    of the line the predecessor's pinned Python package comes from, behind an `AnkiEngine` port that
    keeps every engine type inside `ingest` (the anti-corruption layer); `deny.toml`'s
    `allow-git` names that repository and the one fork the engine's own manifest pins by revision
    (`ankitects/rust-url`, for `percent-encoding-iri`), and nothing else (§7).
R2. The measurement follows ADR-022 exactly: the synthetic collection it defines, a cold build on a
    GitHub-hosted `ubuntu-24.04` runner by `.github/workflows/engine-measure.yml` (pull requests
    that change `crates/ingest/**` or `Cargo.lock`, read-only token, pinned actions), the stripped
    size of `crates/ingest/examples/engine_probe.rs` built in release, and peak resident memory
    (`VmHWM`) read by the budget tests in their own process.
R3. The spike passes only if every ADR-022 budget holds: cold build at most 20 minutes, stripped
    binary at most 100 MiB, peak RSS at most 256 MiB for opening the collection and resolving its
    new-card queue, peak RSS at most 256 MiB for a full download, an incremental sync of 100 new
    reviews within 60 seconds; and `cargo deny` passes, any licence added to `deny.toml`'s allow
    list being compatible with AGPL-3.0-or-later (ADR-018) and named in ADR-022's outcome, and any
    advisory the engine's tree brings being an exception `deny.toml` names by its id and reason
    and ADR-022's outcome lists (§7). A tool
    the engine's build needs beyond the pinned toolchain (a protobuf compiler, say) is installed in
    the workflow and counted inside the build budget.
R4. ADR-009 records the numbers in its Confirmation section and its status becomes `accepted`, or
    `superseded` with the numbers when a budget fails.

The sync (#15)

R5. `ingest::Syncer` syncs a private copy of the collection from the configured sync server
    (`DECKSTREAK_SYNC_ENDPOINT`, required; the credentials `anki-sync-username` and
    `anki-sync-password` through the kernel's loader), with media never synced.
R6. A normal sync is incremental. When the server demands a full sync, the copy is replaced by a
    full DOWNLOAD, written beside the copy and swapped in only when complete. When the server holds
    no collection (a full upload would be needed), the sync is refused with `full_upload_required`
    and the local copy is left as it was. No DeckStreak code path uploads a collection or sends a
    local change (R14); the skip day, the predecessor's one write back to Anki, is not built here.
R7. Every sync, a full download and every open of the copy holds the collection lock
    (`<state directory>/collection.lock`, an exclusive `flock` for a sync, shared for a read),
    released by an explicit unlock before the file closes; a second sync waits for the first and
    never overlaps it.
R8. A sync is attempted at most `SYNC_RETRY_ATTEMPTS` times; each attempt is bounded by
    `SYNC_TIMEOUT_SECS`; between attempts the wait follows the golden schedule (an exponential base
    plus a jitter of at most `SYNC_RETRY_JITTER_FRAC` of it). Every wait goes through tokio's timer,
    so the tests run on paused time and never sleep. A locked collection on open is retried
    `COLLECTION_OPEN_RETRIES` times inside one attempt.
R9. A failed sync ends in one reason code from a closed set: `missing_credentials`,
    `auth_rejected`, `network_unreachable`, `server_error`, `sync_timeout`,
    `full_upload_required`, `collection_locked`, `open_failed`, `engine_failed`. No error text,
    path, endpoint or credential is stored or logged with it.
R10. Each sync records one `sync_runs` row (`migrations/002201_ingest_sync_runs.sql`, `STRICT`,
    `created_at`): when it started and finished, its `trigger` (`scheduled` or `owner`, held by a
    `CHECK`, R16), `ok` or `error`, the reason code, the attempts used and whether it was a full
    download. `ingest` exposes the count of consecutive failures, the instant of the last success,
    and the study day's sync outcome (whether a sync that started in a given study day succeeded,
    with its trigger). The scheduler's alerting and the dead-man watch read the first two
    (SPEC-027); the jobs that need the study day's data read the outcome instead of syncing
    (ADR-037). The change gate adds `skipped` rows, each with its cycle's trigger (SPEC-023).
R11. `coordination::sync_cycle` is the one use case that runs a sync and records it. The scheduler's
    daily `sync` job and the owner's `/sync` call it (SPEC-027, SPEC-026); no other job does
    (ADR-037).
R12. `ingest` implements the data-rights port: `sync_runs` is exported and erased. The table is
    registered in docs/CONTEXT-MAP.md's register of DeckStreak's own tables.
R13. An endpoint whose scheme is `http:` sends the credential in the clear: the service logs one WARN
    at start naming the setting, never its value, and the transport is the owner's decision
    (docs/OWNER-SETUP.md).

The cadence and the no-upload census (ADR-037)

R14. The no-upload census: a recording fake sync server (the in-process sync server behind a layer
    that keeps every request) drives every sync scenario: a normal sync, a sync with nothing new, a
    full-sync demand and an empty server. It fails the test on any full-upload request and on any
    request that carries a local change. On a full-sync demand the client downloads, or refuses with
    `full_upload_required` when the server holds no collection (R6); it never uploads, in any
    scenario.
R15. A scheduled sync runs at most once per study day (the kernel's study day, SPEC-020), and a
    second is refused before any request, recording no row. Once SPEC-027 lands, the claim that
    holds this is its cron-fire ledger's: the `sync` job's fire is claimed per study day before the
    sync is asked for. Until then the refusal reads `sync_runs` (a `scheduled` row that started in
    the same study day), and that check remains the syncer's own guard afterwards.
R16. `sync_runs` records each run's trigger: `scheduled` for the scheduler's daily `sync` job, and
    `owner` for the owner's explicit trigger (the bot's `/sync`, SPEC-026, and any later Mini App
    action that names itself one).
R17. An owner trigger less than `OWNER_SYNC_DEBOUNCE_SECS` (300, ADR-037's 5 minutes) after a
    successful sync finished returns that sync's result without syncing: no request and no row. A
    later owner trigger syncs like any run, under the collection lock (R7).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | ADR-009 records the measured build minutes, binary MiB and peak RSS against ADR-022's budgets, and a final status | `test_engine_spike_record.py`; `engine-measure.yml`'s report |
| A2 | opening the large synthetic collection and resolving its new-card queue stays inside the memory budget | `engine_budget` test |
| A3 | a full download of the large synthetic collection stays inside the memory budget | `engine_budget` test |
| A4 | a sync pulls a review made on another client | `sync` test against a local sync server |
| A5 | a second sync with no change on the server pulls nothing | `sync` test |
| A6 | a full-sync demand downloads the collection and never uploads it | `sync` test |
| A7 | a server with no collection is refused with `full_upload_required`, the copy untouched | `sync` test |
| A8 | the retry schedule equals the predecessor's | `retry` test over `goldens/sync_retry.json` |
| A9 | the sync constants equal the predecessor's | `retry` test over `goldens/sync.constants.json` |
| A10 | an attempt past its timeout is recorded as `sync_timeout` | `retry` test on paused time |
| A11 | a failed sync records one bounded reason code and no error text | `retry` test |
| A12 | a second sync waits for the collection lock and never overlaps the first | `lock` test |
| A13 | the ingest port declares `sync_runs` exported and erased | `data_rights` test |
| A14 | a missing sync endpoint refuses start by name | `settings` test |
| A15 | a recording fake sync server sees no full-upload request and no request carrying a local change in any scenario: a normal sync, nothing new, a full-sync demand, an empty server | `sync` test against the recording server |
| A16 | a second scheduled sync in one study day is refused before any request, and the first is recorded with the trigger `scheduled` | `sync` test on a manual clock |
| A17 | an owner trigger less than 5 minutes after a successful sync returns that result with no request and no row, and one after 5 minutes syncs and is recorded with the trigger `owner` | `sync` test on a manual clock |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_engine_spike_record.py -k adr_009_records_the_measured_numbers_and_a_final_status
A2: cargo test -p deck-streak-ingest --test engine_budget -- --exact opening_a_large_synthetic_collection_and_its_new_card_queue_stays_inside_the_memory_budget
A3: cargo test -p deck-streak-ingest --test engine_budget -- --exact a_full_download_of_the_large_synthetic_collection_stays_inside_the_memory_budget
A4: cargo test -p deck-streak-ingest --test sync -- --exact a_sync_pulls_a_review_made_on_another_client
A5: cargo test -p deck-streak-ingest --test sync -- --exact a_second_sync_with_no_change_pulls_nothing
A6: cargo test -p deck-streak-ingest --test sync -- --exact a_full_sync_demand_downloads_and_never_uploads
A7: cargo test -p deck-streak-ingest --test sync -- --exact a_server_with_no_collection_is_refused_with_full_upload_required
A8: cargo test -p deck-streak-ingest --test retry -- --exact the_retry_schedule_matches_the_predecessors_golden
A9: cargo test -p deck-streak-ingest --test retry -- --exact the_sync_constants_equal_the_predecessors
A10: cargo test -p deck-streak-ingest --test retry -- --exact an_attempt_past_its_timeout_is_recorded_as_sync_timeout
A11: cargo test -p deck-streak-ingest --test retry -- --exact a_failed_sync_records_a_bounded_reason_code_and_no_error_text
A12: cargo test -p deck-streak-ingest --test lock -- --exact a_second_sync_waits_for_the_collection_lock_and_never_overlaps
A13: cargo test -p deck-streak-ingest --test data_rights -- --exact the_ingest_port_declares_sync_runs_exported_and_erased
A14: cargo test -p deck-streak-ingest --test settings -- --exact a_missing_sync_endpoint_refuses_start_by_name
A15: cargo test -p deck-streak-ingest --test sync -- --exact a_sync_run_sends_no_upload_and_no_local_change
A16: cargo test -p deck-streak-ingest --test sync -- --exact a_second_scheduled_sync_in_one_study_day_is_refused
A17: cargo test -p deck-streak-ingest --test sync -- --exact an_owner_trigger_within_five_minutes_of_a_success_returns_it_without_syncing
```

The sync tests run the engine's own sync server in process on a loopback port, with a synthetic
user and a synthetic collection built by `crates/ingest/tests/support/synthetic.rs`; nothing
reaches the owner's server. A15 puts a recording layer in front of that server
(`crates/ingest/tests/support/recording.rs`) that keeps every request and fails the test on an
upload or a local change; A16 and A17 run on a manual clock. The budget tests build ADR-022's
collection with bulk inserts, so they finish in about a minute; they run in the gate like every
other test.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/Cargo.toml` | `deck-streak-ingest` | changed: `anki` (git, pinned tag), kernel, sqlx (§7), tokio, thiserror, tracing; dev: tempfile, tokio (the tests' and the probe's runtime), serde, serde_json, zstd (§7) |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed |
| `crates/ingest/src/engine.rs` | `deck-streak-ingest` | added: the `AnkiEngine` port and its adapter over the engine |
| `crates/ingest/src/sync.rs` | `deck-streak-ingest` | added: `Syncer`, retries, the reason codes, the per-study-day refusal and the owner debounce |
| `crates/ingest/src/lock.rs` | `deck-streak-ingest` | added: the collection lock |
| `crates/ingest/src/sync_runs.rs` | `deck-streak-ingest` | added: the record with its trigger, consecutive failures, last success, the study day's outcome |
| `crates/ingest/src/settings.rs` | `deck-streak-ingest` | added: endpoint and paths |
| `crates/ingest/src/data_rights.rs` | `deck-streak-ingest` | added |
| `crates/ingest/examples/engine_probe.rs` | `deck-streak-ingest` | added: the binary the size budget measures |
| `crates/ingest/tests/engine_budget.rs`, `crates/ingest/tests/sync.rs`, `crates/ingest/tests/retry.rs`, `crates/ingest/tests/lock.rs`, `crates/ingest/tests/data_rights.rs`, `crates/ingest/tests/settings.rs` | `deck-streak-ingest` | added: A2 to A17 |
| `crates/ingest/tests/support/synthetic.rs`, `crates/ingest/tests/support/mod.rs` | `deck-streak-ingest` | added: the seeded synthetic collection and the local sync server |
| `crates/ingest/tests/support/recording.rs` | `deck-streak-ingest` | added: the recording layer the no-upload census runs through |
| `crates/coordination/Cargo.toml`, `crates/coordination/src/lib.rs`, `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | added or changed: the sync cycle use case |
| `migrations/002201_ingest_sync_runs.sql` | `deck-streak-ingest` | added: `sync_runs`, its `trigger` checked to `scheduled` or `owner` |
| `.sqlx/` | workspace | changed |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: `anki` admitted by ADR-022; `zstd` for the census's tests (§7); `libsqlite3-sys` held at a version the engine and the kernel both accept (§7) |
| `deny.toml` | workspace | changed: `allow-git` for Anki's repository and the fork its engine pins; any compatible licence the engine needs; the engine's advisories, each by id and reason (§7) |
| `.github/workflows/engine-measure.yml` | repo | added: the cold-build and size measurement |
| `.github/workflows/ci.yml` | repo | changed: the gate job installs the protobuf compiler the engine's build needs (§7) |
| `scripts/check.sh` | repo | changed: the toolchain stage names the protobuf compiler (§7) |
| `scripts/tests/test_engine_spike_record.py` | repo | added: A1 |
| `tools/parity-oracle/registry/spec_022.py` | repo | added: the retry adapter and the sync constants |
| `tools/parity-oracle/goldens/sync_retry.json`, `sync.constants.json` | repo | added |
| `docs/decisions/ADR-009-ingest-from-the-anki-sync-server.md` | repo | changed: status and the measured numbers |
| `docs/decisions/ADR-022-the-ingest-spike-protocol-and-budgets.md` | repo | added |
| `docs/CONTEXT-MAP.md` | repo | changed: `sync_runs` registered to `ingest` |
| `docs/OWNER-SETUP.md` | repo | changed: the sync transport item |
| `.env.example` | repo | changed: the sync settings, by name |
| `docs/schematics/sync-cycle-and-change-gate.md` | repo | added (shared with SPEC-023) |
| `docs/red-first/SPEC-022.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It reads no review or card from the copy and runs no change gate (#16).
- It schedules nothing and pages nobody: the daily sync job, the failure alert and the dead-man
  watch are the scheduler's (#20).
- It resolves no day set for the readings, though the spike measures the queue call they will use
  (#31).
- It writes nothing back to Anki, ever. The predecessor's one write, the skip day, is recorded in
  DeckStreak's own database instead and never reaches the collection (ADR-037, #108).
- It copies no predecessor database: the v9 import is W8's (#61).
- It provisions no credential on the host (#41).
- It raises no sync cadence: one scheduled sync per study day holds until cutover decides otherwise
  (#164).

## 6. Risks

- **The engine does not build outside Anki's own build system.** That is a measured outcome, not a
  surprise: A1 records it, and the sidecar is ADR-009's named fallback.
- **The engine's sync protocol version drifts from the owner's server.** The tag is pinned; an Anki
  upgrade is a deliberate delivery (ADR-009). A server that refuses the client's protocol ends in
  `server_error` on every attempt, and three consecutive failures page (SPEC-027).
- **A budget test measures more than the engine.** Each budget test runs alone in its own process
  (the acceptance command selects one test), and `VmHWM` is read at its end; nextest also runs each
  test in its own process.
- **The build budget is spent on caching, not compiling.** The workflow measures a cold build with
  no cache restored, so the number is the worst case CI pays.
- **A full download fills the disk.** The download is written beside the copy before the swap, so
  it needs one collection's worth of free space; the host's disk headroom is the host inventory's (#40),
  and a failed write ends in `engine_failed` with the old copy intact.
- **One sync a day makes a failed sync cost the day's data.** Its retries belong to its one run
  (R8); the jobs that read the study day then record `sync_failed` (ADR-037), SPEC-027 pages on the
  failure transitions, and the owner's `/sync` recovers the day, followed by a regeneration
  (SPEC-048).
- **An engine upgrade changes the requests a sync sends.** The tag is pinned (R1), so an upgrade is a
  deliberate delivery, and it reruns the census (A15) against the recording server before it merges.

## 7. Amendments at delivery

- **R1: two git sources, both Anki's.** The engine's own manifest at `26.05` takes
  `percent-encoding-iri` from its fork `ankitects/rust-url`, pinned by revision, so a lockfile with
  the engine holds a second git source and "that repository and nothing else" could not hold.
  `deny.toml` names exactly the two, and `unknown-git` still refuses any third.
- **R3: the engine's advisories are named exceptions.** The gate's audit stage runs
  `cargo deny check advisories` over every crate in the lockfile. The engine's tree brings eight
  RustSec "unmaintained" notices and no vulnerability: `paste` (RUSTSEC-2024-0436), five `unic-*`
  crates (RUSTSEC-2025-0075, -0080, -0081, -0094, -0098), `rustls-pemfile` (RUSTSEC-2025-0134) and
  `bincode` (RUSTSEC-2025-0141). Each is an exception with its reason, as `deny.toml` requires,
  and ADR-022's outcome lists them; the predecessor's Python package carries the same engine and
  the same crates.
- **Manifest: `ci.yml` and `check.sh`.** The engine's build scripts compile Anki's protobuf
  definitions with `prost-build`, which needs `protoc` (on `PATH`, or named by `PROTOC`). The
  gate's clippy and test stages build the engine, so the gate job installs the same `protoc` the
  measurement does, and the toolchain stage names it, so a machine without it fails there by name
  rather than deep inside a build script.
- **§3: the tests' sync server runs in a child process, not in process.** The engine's server
  reads its users only from `SYNC_USER1` in its process environment, and setting an environment
  variable is `unsafe` in edition 2024, which this workspace forbids. A test that needs the server
  re-executes its own test binary as the server with `Command::env`; the server exits when the
  test closes its standard input (`crates/ingest/tests/support/mod.rs`). A budget test's measured
  operation runs in a third process for the same reason the budgets need it: `VmHWM` is a process's
  peak, so the fixture's build and the server stay out of the measured one.
- **`Cargo.lock`: one bundled SQLite for the workspace.** A dependency graph may hold one crate
  that links the native `sqlite3`. The engine's `rusqlite` 0.36 accepts only `libsqlite3-sys`
  0.34, and the kernel's `sqlx` 0.9 accepts 0.30.1 up to 0.37, so the lockfile holds 0.34.0, which
  both declare they accept (dev had locked 0.37.0). An upgrade of either must keep one version
  both accept.
- **`crates/ingest/Cargo.toml`: sqlx.** The kernel's `Db` hands out sqlx types (a `BEGIN
  IMMEDIATE` transaction, the read pool), and `sync_runs`' queries are compile-time checked into
  `.sqlx/`, which the manifest already names. ADR-003 admits sqlx for the workspace; ingest inherits
  it with no feature of its own.
- **`crates/ingest/Cargo.toml`: a zstd dev-dependency.** A15's recording layer reads each request
  the engine sends, and the engine compresses every request body with zstd; the census decompresses
  it to prove no body carries a local change. It is the zstd the engine already brings (one copy in
  the lockfile), a dev-dependency of ingest only.
- **R8 and the tests: the retry schedule is a value.** `RetrySchedule::PREDECESSOR` holds the
  golden-proved constants and is what production runs; a test of anything but timing runs the same
  loop with `RetrySchedule::IMMEDIATE`, whose waits are zero-length waits on tokio's timer. A8 and
  A10 run the predecessor's schedule on paused time.
- **R10 and the tests: the run's record sits behind a port.** `SyncRunStore` is what the syncer
  asks (was a scheduled run recorded this study day, the last success) and tells (record a run);
  `SqliteSyncRuns` is its implementation over the kernel's `Db`. sqlx's pool times its acquire on
  tokio's timer, and paused time jumps to a pending timer while the database answers on its own
  thread, so the paused-time retry tests hand the syncer an in-memory store; every other test uses
  the database.
- **R10 and R15: `sync_runs` records the run's study day.** The refusal of a second scheduled run
  reads the study day the kernel's rule gave the first when it started, so a row is found by its
  day rather than by an instant range the rule would have to invert.

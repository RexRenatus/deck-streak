# SPEC-025: the API service answers health, bounds every request, tells systemd it is alive, and drains on SIGTERM

- **Wave:** W0. **Issue:** #18 (epic #1). **Context(s):** `deck-streak-api`, `deck-streak-daemon` (the `deckstreakd` binary, its roles and its lifecycle).
- **Decided by:** ADR-003 (axum 0.8, tower-http's layers, tracing, anyhow only in the binary), ADR-007 (loopback only, `/api` behind Caddy), ADR-008 (the database under the unit's `StateDirectory=`), ADR-010 (one release binary, units per role, `Type=notify` with a watchdog), and this SPEC's ADR-025 (health on the one listener and closed at the edge, shedding instead of queueing, `tower` admitted, and the decisions of §7).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-025.md`, and the golden
  `watchdog.constants` generated at the predecessor's `27ee2bc`. The delivery made R1, R2 and R4 to
  R8 exact where the code decided them, and added R11 and A15, which close the start-up race of two
  roles opening one fresh database (§7).

## 1. The problem, measured

- **Nothing serves yet.** `crates/api/src/lib.rs` and `crates/daemon/src/lib.rs` hold documentation
  only, and the workspace builds no binary (read at `main` e05dfa5); `rust-service` is enforced in
  `.packs/wiring.json`, so the first HTTP service must meet its lifecycle and memory rows from its
  first commit.
- **The predecessor's rules this ports** (predecessor `27ee2bc`, names only):
  - liveness separate from readiness, readiness failing with 503
    (`server.py:create_server`, `pipeline_layers/ops.py:OpsLayer.health_status`);
  - `Type=notify`: `READY=1` once serving, a heartbeat every `WatchdogSec` divided by
    `watchdog.py:_HEARTBEAT_DIVISOR`, never below `watchdog.py:_MIN_WATCHDOG_SEC`, `STOPPING=1` on
    shutdown, a no-op outside systemd (`watchdog.py:SdNotifier`, `run_sd_watchdog`); the watchdog
    is deliberately not coupled to sync health;
  - the goldens: `goldens/watchdog.constants.json` (both constants), registered in
    `tools/parity-oracle/registry/spec_025.py`.
- **The rows that judge it** (rust-service, enforced; observability, pending until SPEC-031):
  `rs.axum-route-syntax`, `rs.graceful-shutdown`, `rs.sigterm-handled`, `rs.notify-ready`,
  `rs.watchdog-ping`, `rs.bounded-channels`, `rs.bounded-body-reads`, `rs.concurrency-bound`,
  `rs.catch-panic`, `rs.request-timeout`, `obs.http-trace-layer`, `obs.request-events-visible`,
  `obs.span-route`, `obs.request-id`, `obs.sensitive-headers`, `obs.subscriber-installed`,
  `obs.structured-logs`.
- **The pack's reference values** (the rust-service pack's `templates/service-main.rs.template`):
  64 requests in flight and a 10-second request timeout; axum's default 2 MB body limit for its
  extractors.

**Order.** After SPEC-020 (settings, `Db`, `logging::install`). It can run beside SPEC-022.
SPEC-024 (the session route and the owner extractor) lands after it, and so do SPEC-026 and
SPEC-027, which add the `bot` and `job` roles to the binary this SPEC creates; SPEC-032's unit
templates run this binary.

## 2. Requirements

R1. `crates/daemon` builds one binary, `deckstreakd`; its first argument names a role (`api` here;
    `bot` and `job` arrive with SPEC-026 and SPEC-027). An unknown or missing role exits with code 2
    and a usage line naming the known roles. The binary's `main` is the one reader of the process
    environment (SPEC-020), and every role calls the kernel's `logging::install` before anything
    else it does.
R2. `deck_streak_api::router` serves `GET /api/livez` (200 while the process runs) and
    `GET /api/readyz` (503 until the database is open and migrated, 200 after); both bodies carry
    only a status word and the release version, never a path, a count or a setting.
R3. Every route is built on axum 0.8's path syntax, and a test builds the router (a `:` or `*`
    segment would panic there).
R4. The layers, outermost first: a request id is set (`x-request-id`, `MakeRequestUuid`); the
    `authorization`, `cookie` and `set-cookie` headers are marked sensitive; a `TraceLayer` whose span
    carries the method, the MATCHED route (`MatchedPath`) and the request id, and whose response
    event at INFO carries the status and the latency; a panic catcher (500); a timeout of 10 seconds
    answering 408 (`TimeoutLayer::with_status_code`); a concurrency bound of 64 in flight that SHEDS
    the next request with 503 (ADR-025); the request id is copied to the response.
R5. Request bodies keep axum's default 2 MB limit for its extractors, and no route disables it; a
    larger body is answered 413 (rust-service `rs.bounded-body-reads`, web-security
    `ws.request-body-limit`).
R6. The listen address comes from `DECKSTREAK_API_LISTEN` and must be a loopback address; any other
    address refuses start by name (ADR-007).
R7. `deck_streak_api::serve` runs `axum::serve(..).with_graceful_shutdown(..)`: on the shutdown
    signal it stops accepting, lets every in-flight request finish, and returns; the role then
    exits 0.
R8. `crates/daemon/src/lifecycle.rs` owns the process lifecycle every long-running role shares: the
    sd_notify client (a datagram to the socket named by `NOTIFY_SOCKET`, handed in by `main`; a
    no-op when it is absent), `READY=1` once the role serves, a watchdog task sending `WATCHDOG=1`
    every `WATCHDOG_USEC` divided by `_HEARTBEAT_DIVISOR` (never more often than
    `_MIN_WATCHDOG_SEC` allows), `STOPPING=1` on shutdown, and the shutdown future resolving on
    SIGTERM (and SIGINT at a terminal).
R9. No channel the API opens is unbounded (rust-service `rs.bounded-channels`).
R10. `.env.example` names `DECKSTREAK_API_LISTEN` with a loopback example.
R11. (Added at delivery, §7.) Every role opens the database, `deck_streak.db` under
    `$STATE_DIRECTORY` (ADR-008), through the daemon's `wiring::open_database`, which holds an
    exclusive lock on `deck_streak.db-open.lock` beside it while the kernel's `Db::open` sets the
    pragmas and applies the migrations, and releases it explicitly. Two roles opening one fresh
    database at the same moment both start.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `/api/livez` answers 200 while the process runs | `health` test |
| A2 | `/api/readyz` answers 503 until the database is open, then 200 | `health` test |
| A3 | the router builds on axum 0.8's route syntax | `health` test; rust-service `rs.axum-route-syntax` |
| A4 | a body over the limit is refused with 413 | `limits` test; `rs.bounded-body-reads` |
| A5 | a request past the concurrency bound is shed with 503 | `limits` test; `rs.concurrency-bound` |
| A6 | a handler past the timeout answers 408 | `limits` test; `rs.request-timeout` |
| A7 | a panicking handler answers 500 and the service keeps serving | `limits` test; `rs.catch-panic` |
| A8 | every response carries a request id, and an INFO event names its matched route and status | `trace` test; `obs.http-trace-layer`, `obs.span-route`, `obs.request-id`, `obs.request-events-visible` |
| A9 | a cookie or authorization header value never reaches the log | `trace` test; `obs.sensitive-headers` |
| A10 | the shutdown signal lets an in-flight request finish before `serve` returns | `drain` test; `rs.graceful-shutdown` |
| A11 | the running binary sends `READY=1`, `WATCHDOG=1` and, on SIGTERM, `STOPPING=1`, then exits 0 | daemon `lifecycle` test; `rs.notify-ready`, `rs.watchdog-ping`, `rs.sigterm-handled` |
| A12 | the watchdog interval follows the predecessor's divisor and floor | daemon `lifecycle` test over `goldens/watchdog.constants.json` |
| A13 | a non-loopback listen address refuses start by name | `listen` test |
| A14 | the binary runs a role by name and refuses an unknown one with code 2 | daemon `roles` test |
| A15 | two roles opening one fresh database at the same moment both start (added at delivery) | daemon `roles` test |

```acceptance
A1: cargo test -p deck-streak-api --test health -- --exact livez_answers_while_the_process_runs
A2: cargo test -p deck-streak-api --test health -- --exact readyz_is_false_until_the_database_is_open
A3: cargo test -p deck-streak-api --test health -- --exact the_router_builds_with_axum_08_route_syntax
A4: cargo test -p deck-streak-api --test limits -- --exact a_body_over_the_limit_is_refused_with_413
A5: cargo test -p deck-streak-api --test limits -- --exact a_request_past_the_concurrency_bound_is_shed_with_503
A6: cargo test -p deck-streak-api --test limits -- --exact a_handler_past_the_timeout_answers_408
A7: cargo test -p deck-streak-api --test limits -- --exact a_panicking_handler_answers_500_and_the_service_keeps_serving
A8: cargo test -p deck-streak-api --test trace -- --exact every_response_carries_a_request_id_and_an_info_event_with_the_matched_route
A9: cargo test -p deck-streak-api --test trace -- --exact sensitive_header_values_never_reach_the_log
A10: cargo test -p deck-streak-api --test drain -- --exact the_shutdown_signal_lets_an_in_flight_request_finish
A11: cargo test -p deck-streak-daemon --test lifecycle -- --exact the_binary_notifies_ready_watchdog_and_stopping_then_exits_zero
A12: cargo test -p deck-streak-daemon --test lifecycle -- --exact the_watchdog_interval_follows_the_predecessors_divisor
A13: cargo test -p deck-streak-api --test listen -- --exact a_non_loopback_listen_address_is_refused_by_name
A14: cargo test -p deck-streak-daemon --test roles -- --exact the_binary_runs_a_role_by_name_and_refuses_an_unknown_one
A15: cargo test -p deck-streak-daemon --test roles -- --exact two_roles_opening_one_fresh_database_at_once_both_start
```

A4 to A7 drive the router with `tower::ServiceExt::oneshot` and test-only routes that sleep on
tokio's paused clock, block on a channel or panic. A11 runs the built binary
(`env!("CARGO_BIN_EXE_deckstreakd")`) with a temporary database, a temporary datagram socket as its
`NOTIFY_SOCKET` and a short `WATCHDOG_USEC`, sends SIGTERM with the system's `kill`, and reads the
socket; the child's environment is set on the `Command`, never on the test process. A15 opens one
fresh database from two tasks released together, 64 times.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/api/Cargo.toml` | `deck-streak-api` | changed: axum, tower (limit, load-shed), tower-http (trace, timeout, request-id, sensitive-headers, catch-panic), tokio, tracing, thiserror; dev: tempfile, tokio (the paused clock, io-util, net), tower (util) |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed |
| `crates/api/src/router.rs` | `deck-streak-api` | added: routes and layers |
| `crates/api/src/health.rs` | `deck-streak-api` | added: livez and readyz |
| `crates/api/src/serve.rs` | `deck-streak-api` | added: `serve` with graceful shutdown |
| `crates/api/src/settings.rs` | `deck-streak-api` | added: the loopback listen address |
| `crates/api/tests/health.rs`, `limits.rs`, `trace.rs`, `drain.rs`, `listen.rs` | `deck-streak-api` | added: A1 to A10, A13 |
| `crates/daemon/Cargo.toml` | `deck-streak-daemon` | changed: `[[bin]] deckstreakd`, anyhow, tokio, tracing, thiserror (§7); dev: tempfile, serde, serde_json, tokio |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | added: the role dispatch |
| `crates/daemon/src/lifecycle.rs` | `deck-streak-daemon` | added: sd_notify, the watchdog task, the shutdown signal |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | added |
| `crates/daemon/src/wiring.rs`, `crates/daemon/src/lib.rs` | `deck-streak-daemon` | added or changed: the state directory, the database's one opener and its open lock (R11) |
| `crates/daemon/tests/lifecycle.rs`, `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | added: A11, A12, A14, A15 |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: axum, tower-http, anyhow (ADR-003), tower (ADR-025) |
| `tools/parity-oracle/registry/spec_025.py`, `tools/parity-oracle/goldens/watchdog.constants.json` | repo | added |
| `.env.example` | repo | changed: `DECKSTREAK_API_LISTEN`, and `STATE_DIRECTORY` (§7) |
| `docs/schematics/service-lifecycle.md` | repo | changed: written at planning, redrawn as built, with the layers and the open lock |
| `docs/decisions/ADR-025-api-health-bounds-and-shedding.md` | repo | changed: accepted, with the decisions the delivery made |
| `docs/red-first/SPEC-025.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It validates no `initData`, opens no session and guards no route with the owner extractor
  (#17).
- It exports no Prometheus `/metrics`: the request events in the journal are the service's metrics,
  and the SLO evaluator counts them (#24).
- It serves no HTML dashboard; the Mini App replaces it (#21).
- Its readiness does not follow the sync's freshness: the dead-man watch pages on a stale sync, and
  readiness answers for the API alone (#20).
- It carries no in-app notification transport (#27).
- It serves no MCP endpoint (#157).
- It writes no unit or Caddy block, and no alert path (#25, #24).

## 6. Risks

- **The drain outlasts the unit's stop timeout.** The 10-second request timeout bounds every
  in-flight request, so a drain ends within it; SPEC-032's unit sets `TimeoutStopSec=` above it, and
  durable-services' `service.timeout-stop` reads the pair.
- **The watchdog pings from a task that survives a wedged runtime.** The ping task runs on the same
  runtime as the server, so a blocked runtime stops the pings and systemd restarts the unit; the
  kernel's `Offload` keeps blocking work off the runtime (SPEC-020).
- **A test that sends SIGTERM flakes on a loaded machine.** A11 waits on the socket's messages, not
  on a timer, and bounds its whole run with one generous timeout that fails with the messages seen.
- **Shedding hides overload.** Each shed request is a 503 response event in the journal, which the
  availability SLO counts (SPEC-031), so sustained shedding burns the error budget visibly.
- **A role waits on the open lock behind a wedged opener.** The wait runs on the offload, off the
  runtime, so the heartbeat keeps pinging and readiness answers 503 for as long as it lasts: the
  wait is visible, never a silent hang. A holder that dies releases the lock with its process.

## 7. Amendments at delivery

- **R1: the logging is installed before the role is dispatched,** so a refused role is itself a
  JSON ERROR event with its priority (SPEC-031 R1). A missing role, an unknown one and an argument
  the role does not take (`deckstreakd api extra`) all exit 2 with the usage line
  `usage: deckstreakd <role>, the one argument; the roles are: api`. A role that refuses start or
  fails exits 1; the role table is `main.rs`'s `Role`, which SPEC-026 and SPEC-027 extend.
- **R2: readiness holds the opened database.** `Readiness` becomes ready only when it is handed the
  `Db` that `Db::open` returned, and keeps it for later routes. The bodies are
  `{"status":"alive","version":"0.1.0"}`, `{"status":"ready",...}` and `{"status":"starting",...}`,
  as `application/json`, written whole from two build-time strings.
- **R4: the concurrency bound is `GlobalConcurrencyLimitLayer`, inside `LoadShedLayer`, behind
  axum's `HandleErrorLayer` answering 503.** The stack is applied with `Router::layer`, which axum
  0.8.9 applies to every route and every method separately (`PathRouter::layer`,
  `MethodRouter::layer`), and `ConcurrencyLimitLayer::layer` makes a new semaphore each time, so the
  `ConcurrencyLimitLayer` ADR-025 named would have bounded each route at 64 and the service at 64
  times its routes. A5 proves the shared bound by shedding a second route while the first holds
  every slot (ADR-025, "Decided at delivery").
- **R4: the request id is copied to the response directly inside the trace,** outside the panic
  catcher, the timeout and the shed. The order R4 listed put it innermost, where a 500, a 408 or a
  503 made by an outer layer would never carry it, which A8's "every response" refutes; A8 proves a
  caught panic's 500 and the fallback's 404 carry it.
- **R4: the sensitive headers are marked twice:** on the request outside the trace, as R4 orders,
  and on the response inside it, which is where tower-http's `sensitive_headers` documentation
  says a response header must be marked for the trace to see the mark. The span records the
  method, the matched route (absent for the fallback's 404) and the request id, and never the URI,
  whose query string may carry `initData`; A8 proves no path or query reaches the log. The
  response event is tower-http's `DefaultOnResponse` at INFO, `finished processing request` with
  `status` and `latency`: the event SPEC-031's SLI counts.
- **R5: the body limit is stated,** `DefaultBodyLimit::max(BODY_LIMIT_BYTES)` at 2 MiB, which is
  axum's own default (axum-core 0.5.6, `DEFAULT_LIMIT = 2_097_152`). The number is written where the
  web-security row reads it, and it is never `usize::MAX`. A4 therefore could not be red first:
  axum's default already refused the body at the stub.
- **R6: `DECKSTREAK_API_LISTEN` is required.** Unset, it refuses start as `SettingsError::Missing`;
  not a socket address (a host name included), as `SettingsError::Malformed`; not loopback, as
  `ApiError::NotLoopback`, each naming the setting and never the value. Loopback is IPv4's
  127.0.0.0/8 and IPv6's `::1`; the unspecified addresses are refused.
- **R7: after the drain the role closes the database and exits 0.** A database that fails to open
  stops the role the same way, after a drain, and the role exits 1 naming the failure.
- **R8: the watchdog is the predecessor's, exactly.** Below `_MIN_WATCHDOG_SEC` no heartbeat is
  armed and one WARN names the unit to fix; a `WATCHDOG_PID` naming another process arms none; the
  first ping follows `READY=1` at once. The predecessor's one-second floor on the interval
  (`max(1.0, wd / 3)`) cannot bind above the five-second minimum and is not written out. The
  SIGTERM and SIGINT handlers are installed before `READY=1`, so a signal the moment after it drains
  the role. The notify socket may be a path or an abstract name (`@name`), as the predecessor's;
  another form disables the client with one WARN.
- **R11 (added): the start-up race is closed by a lock, not made rarer.** sqlx's SQLite migrator
  takes no lock. Two tasks opening one fresh database at once, through a bare `Db::open` at the
  red-first commit, failed in 29 of 30 runs of 24 rounds with `SQLITE_BUSY` (a fresh file's switch
  to WAL, or a migration's write, colliding), and once with `table settings_generation already
  exists` (both applying migration 2001). Of the three ways the brief offered, the lock was chosen
  (ADR-025 records the others and why each was rejected): every role opens through
  `wiring::open_database`, which takes an exclusive `flock` on `deck_streak.db-open.lock` on the
  kernel's offload, runs `Db::open`, and unlocks explicitly. A15 opens one fresh database from two
  tasks released together, 64 times, and both start every time.
- **The database's place** is `$STATE_DIRECTORY/deck_streak.db` (ADR-008; the deployment
  schematic; SPEC-032 sets `StateDirectory=deck-streak`). `STATE_DIRECTORY` is a required setting of
  every role that opens the database, named in `.env.example` beside `CREDENTIALS_DIRECTORY`.
- **Every role reads the kernel's settings at start,** so a malformed kernel setting refuses start
  in the `api` role too, and the offload bound the open lock's wait runs on is the configured one.
- **The manifest:** the daemon also takes `tracing` (the lifecycle's events) and `thiserror` (its
  library's typed errors), both admitted by ADR-003; tokio's `test-util`, `io-util` and `net` and
  tower's `util` are dev features of admitted crates. tower-http is 0.7.1, the current release; the
  layers R4 names keep their API there.

## 8. Amendment, 2026-09-29: a failed lifecycle test leaves no daemon running

Issue 366. Three `deckstreakd api` daemons and one `deckstreakd bot` were found running long after
the tests that started them.

- **Measured.** A lifecycle test that failed before its stop step left its daemon running when the
  daemon was silent (no log output, no watchdog): a logging daemon aborts on the broken pipe at its
  next write, so only a silent one survives. The `bot` role's child, with its output on null,
  survived a SIGKILL of the test, which runs no destructor.
- **The guard.** `crates/daemon/tests/lifecycle.rs` owns its child through `Daemon`, which keeps the
  waiter thread. Its drop sends SIGTERM through the system `kill` (the crates forbid unsafe code and
  admit no signal dependency), waits on the waiter's message for a bound, sends SIGKILL if the child
  is still running, and joins the waiter only once the child is reaped. A flag the waiter sets
  before it reports keeps the signals off a reused pid. Every test that spawns the daemon uses it.
- **A16.** A test runs a scenario that fails before its stop step through this binary and the
  harness, and asserts the daemon is gone; the daemon was running before the failure. Fence:
  `cargo test -p deck-streak-daemon --test lifecycle -- --exact
  a_failing_lifecycle_test_leaves_no_daemon_running`.
- **The `bot` case, ruled.** The orchestrator ruled a process-group kill in the row runner's killer
  run (both kinds; the parse and build-only calls unchanged): the killer leads its own group, a
  timeout kills the group and reaps, and any other exit kills it while the leader lives. **A17**
  is its test, with a grandchild that outlives the bound. Chosen against: a parent-death signal
  set in `pre_exec` (needs `unsafe`), a daemon that watches its parent (a production change), and
  the drop guard alone (a destructor does not run on SIGKILL). `process_group(0)` was measured
  sufficient, so no new session is made. Limits: an external SIGKILL of the test alone, a SIGTERM
  of the runner itself, and a descendant that leaves the group are not covered.
- **Rows** S02501 to S02505, in `scripts/mutation-rows.d/S02500-S02599.json`: the guard's drop made
  a no-op, its SIGKILL fallback removed, the group kill removed, the group not created, and the
  cleanup on an interrupted run removed. One known survivor: the `ProcessLookupError` suppression
  in `kill_group`, which only tolerates a group that is already gone.

## 9. Acceptance criteria of the 2026-09-29 amendment

| id | criterion | decided by |
|---|---|---|
| A16 | a lifecycle test that fails before its stop step leaves no daemon running | daemon `lifecycle` test |
| A17 | a killer that outlives the row runner's bound leaves no descendant running when the run returns | `test_mutation_rows_group` test |

```acceptance
A16: cargo test -p deck-streak-daemon --test lifecycle -- --exact a_failing_lifecycle_test_leaves_no_daemon_running
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_group.py -k test_the_grandchild_of_a_timed_out_killer_is_gone_when_the_run_returns
```

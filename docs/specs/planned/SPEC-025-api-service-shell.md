# SPEC-025: the API service answers health, bounds every request, tells systemd it is alive, and drains on SIGTERM

- **Wave:** W0. **Issue:** #18 (epic #1). **Context(s):** `deck-streak-api`, `deck-streak-daemon` (the `deckstreakd` binary, its roles and its lifecycle).
- **Decided by:** ADR-003 (axum 0.8, tower-http's layers, tracing, anyhow only in the binary), ADR-007 (loopback only, `/api` behind Caddy), ADR-010 (one release binary, units per role, `Type=notify` with a watchdog), and this SPEC's ADR-025 (health on the one listener and closed at the edge, shedding instead of queueing, `tower` admitted).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-025.md` (ADR-016).

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
```

A4 to A7 drive the router with `tower::ServiceExt::oneshot` and test-only routes that sleep on
tokio's paused clock, block on a channel or panic. A11 runs the built binary
(`env!("CARGO_BIN_EXE_deckstreakd")`) with a temporary database, a temporary datagram socket as its
`NOTIFY_SOCKET` and a short `WATCHDOG_USEC`, sends SIGTERM with the system's `kill`, and reads the
socket; the child's environment is set on the `Command`, never on the test process.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/api/Cargo.toml` | `deck-streak-api` | changed: axum, tower (limit, load-shed, util), tower-http (trace, timeout, request-id, sensitive-headers, catch-panic), tokio, tracing, thiserror; dev: tempfile |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed |
| `crates/api/src/router.rs` | `deck-streak-api` | added: routes and layers |
| `crates/api/src/health.rs` | `deck-streak-api` | added: livez and readyz |
| `crates/api/src/serve.rs` | `deck-streak-api` | added: `serve` with graceful shutdown |
| `crates/api/src/settings.rs` | `deck-streak-api` | added: the loopback listen address |
| `crates/api/tests/health.rs`, `limits.rs`, `trace.rs`, `drain.rs`, `listen.rs` | `deck-streak-api` | added: A1 to A10, A13 |
| `crates/daemon/Cargo.toml` | `deck-streak-daemon` | changed: `[[bin]] deckstreakd`, anyhow, tokio; dev: tempfile, serde, serde_json |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | added: the role dispatch |
| `crates/daemon/src/lifecycle.rs` | `deck-streak-daemon` | added: sd_notify, the watchdog task, the shutdown signal |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | added |
| `crates/daemon/src/wiring.rs`, `crates/daemon/src/lib.rs` | `deck-streak-daemon` | added or changed |
| `crates/daemon/tests/lifecycle.rs`, `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | added: A11, A12, A14 |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: axum, tower-http, anyhow (ADR-003), tower (ADR-025) |
| `tools/parity-oracle/registry/spec_025.py`, `tools/parity-oracle/goldens/watchdog.constants.json` | repo | added |
| `.env.example` | repo | changed |
| `docs/schematics/service-lifecycle.md` | repo | added |
| `docs/decisions/ADR-025-api-health-bounds-and-shedding.md` | repo | added |
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

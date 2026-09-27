---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
name: rust-service
description: Rust HTTP and bot services on axum 0.8, tokio and tracing, run under systemd on a small VM. Lint levels and forbidden unsafe code, typed errors, graceful shutdown on SIGTERM with sd_notify readiness and watchdog, secrets read from systemd credentials never from the environment, and a memory budget of bounded concurrency and no unbounded buffers. A static probe judges any Rust workspace through --root.
---

# packs/rust-service

How a DeckStreak service is written in Rust, and the check that keeps it so (SPEC-V2-2220,
ADR-V2-2220). A service here is an axum 0.8 HTTP server or a Telegram bot on tokio, run as a
systemd unit on one small VM (2 vCPU, 1.9 GiB RAM) beside the other services of the stack. The
owner's scope for this pack: axum 0.8, tokio, tracing and error types; configuration from the
environment and LoadCredential, never a secret in the environment; graceful shutdown on SIGTERM;
a memory budget of bounded concurrency, streaming and no unbounded buffers; clippy lint levels and
`#![forbid(unsafe_code)]`.

`scripts/rust-service-probe.py` is the check. It is standard-library Python, it judges any tree
through `--root`, and every row below runs it. It reads the systemd units through
`scripts/durable-unit-lint.py`'s own parser, so a unit means here what it means to the
durable-services pack. Which seats consume this pack is its catalog row's `consumes`, the one
record of that edge (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack rust-service --root PATH --format json
```

## Running it

- **Through phxd**, from this repository against a DeckStreak checkout: the command above. Every
  row runs `python3 {skills}/../scripts/rust-service-probe.py --root {root} check <id>` under a
  60-second wall; `{skills}` is the skills directory the catalog was read from, so the script
  always comes from this pack.
- **Directly**: `python3 scripts/rust-service-probe.py --root PATH check <id>`, and
  `python3 scripts/rust-service-probe.py classes` lists the ids.
- **Vendored into DeckStreak's CI**: copy `scripts/rust-service-probe.py` and
  `scripts/durable-unit-lint.py` into one directory; the probe imports the second from beside it,
  and is VOID without it.

Every class prints one line per finding, `<id>: <finding>`, naming the file and line or the unit,
and ends with `examined N`. It exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when
VOID: no `Cargo.toml` at the root, an unreadable manifest or source, or a missing sibling script.
A VOID is never a pass. The card turns a `block` row's non-zero exit red (exit 4), and prints an
`advisory` row's finding as `advisory` without reddening the card. A card row carries only the
exit (i1879), so read a red row's reason by running the class directly.

## What it reads

- **The workspace.** The root `Cargo.toml` and every `[workspace] members` crate: its targets by
  Cargo's own auto-discovery (`src/lib.rs`, `src/main.rs`, `src/bin/*`) and any `[lib]` and
  `[[bin]]` tables; its dependencies with renames and `workspace = true` resolved; its `[lints]`.
  `Cargo.lock` decides axum's version where it exists.
- **Production Rust only.** Every `.rs` under a crate's `src/`, never `tests/`, `benches/` or
  `examples/`. Comments are blanked, and so is every item under `#[cfg(test)]` or `#[test]`.
  String literals are read as literals: a name in a string is not a call, and a call in a comment
  is not code. A `use` line imports and does not use: a layer counts where it is applied.
- **A binary's sources** are its root file, its crate's library side and the library side of
  every workspace crate it reaches by path. A handler anywhere in that set counts for the binary.
- **The units.** `deploy/**/*.service` and their drop-ins, as durable-services reads them. A
  unit's first `ExecStart=` names the binary it runs; one that runs a workspace binary ties that
  binary to its unit's `Type=`, `WatchdogSec=`, `KillSignal=` and credentials.
- **CI.** `.github/workflows/*`, `.depot/workflows/*` and `.gitlab-ci.yml`, plus every repository
  script a workflow runs by path, the task file behind `just`, `make` or `task`, and `xtask/src`
  behind `cargo xtask`.

## Severity

A `block` row is a firm requirement: a statement of the official documentation (axum panics on a
0.7 route; systemd fails a notify unit that never says READY=1; systemd.exec(5) calls the
environment "not suitable for passing secrets"; Cargo does not inherit `[workspace.lints]`), or an
owner decision for DeckStreak (unsafe code forbidden, a SIGTERM drain, no unbounded buffer, a
concurrency bound). An `advisory` row is practice the sources recommend and leave to judgement. It
reports and never refuses.

## The rows: 21 in seven stages (14 blocking, 7 advisory)

### Stage `lints`: 7 rows (5 blocking, 2 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.unsafe-forbidden` | block | `unsafe-not-forbidden` | a crate neither inherits nor sets `unsafe_code = "forbid"`, and some root of it (the library and EACH binary root) lacks `#![forbid(unsafe_code)]` | the owner's rule; rustc lint levels: forbid cannot be lowered |
| `rs.lints-inherited` | block | `lints-not-inherited` | the root has `[workspace.lints]` and a member has no `[lints]` table (a dotted `lints.workspace = true` counts) | Cargo reference: workspace lints are not inherited implicitly; Cargo's `missing_lints_inheritance` |
| `rs.lint-group-priority` | block | `lint-group-priority-tie` | a lint group (`all`, `pedantic`, `rust_2018_idioms`...) and a single lint at another level share one priority in one table | Cargo `[lints]` priority; clippy's deny-by-default `lint_groups_priority`, which does not fire in a workspace manifest (rust-clippy #12729) |
| `rs.no-restriction-group` | block | `restriction-group-enabled` | the whole `clippy::restriction` group is raised in a lint table, a crate attribute or a CI flag | the clippy book: "You shouldn't enable the whole lint group" |
| `rs.clippy-ci` | block | `clippy-not-denying-warnings` | no CI command runs `cargo clippy` with warnings denied (`-D warnings`, or `RUSTFLAGS` doing so for that workflow) | the clippy book, continuous integration |
| `rs.fmt-ci` | advisory | `fmt-not-checked` | no CI command runs `cargo fmt` with `--check` | rustfmt |
| `rs.panic-lints` | advisory | `unwrap-unlinted` | a crate leaves `clippy::unwrap_used` at allow, by table or by root attribute | clippy's `unwrap_used` (restriction, cherry-picked) |

### Stage `errors`: 2 rows (0 blocking, 2 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.typed-lib-errors` | advisory | `anyhow-in-library-api` | a library's `pub fn` returns `anyhow` (spelled out, or an imported `anyhow::Result`), or a `pub type` aliases it | thiserror and anyhow READMEs: thiserror for libraries, anyhow for applications |
| `rs.catch-panic` | advisory | `panic-not-caught` | an HTTP service applies no `CatchPanicLayer` (or `.catch_panic()`) | tower-http catch_panic |

### Stage `http`: 2 rows (1 blocking, 1 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.axum-route-syntax` | block | `axum-v07-route-syntax` | on axum 0.8 or later, a `.route`, `.route_service`, `.nest` or `.nest_service` path has a segment starting `:` or `*`, with no `without_v07_checks()` in the file | axum 0.8's path router panics on it |
| `rs.request-timeout` | advisory | `request-timeout-missing` | an HTTP service applies no `TimeoutLayer` (or `ServiceBuilder::timeout`); a body timeout alone does not count | tower-http timeout; tower timeout |

### Stage `lifecycle`: 4 rows (4 blocking, 0 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.graceful-shutdown` | block | `shutdown-not-graceful` | an `axum::serve(..)` statement (or an imported `serve(..)`) has no `.with_graceful_shutdown(..)` | axum's graceful-shutdown example |
| `rs.sigterm-handled` | block | `stop-signal-unhandled` | a long-running unit's binary, or an HTTP service, handles no signal its unit stops it with: `KillSignal=`, SIGTERM by default (`ctrl_c` alone is SIGINT) | systemd.kill(5) KillSignal=; tokio's shutdown topic |
| `rs.notify-ready` | block | `ready-never-sent` | a `Type=notify` or `notify-reload` unit's binary never sends READY=1 | systemd.service(5); sd_notify(3) |
| `rs.watchdog-ping` | block | `watchdog-never-pinged` | a unit with a non-zero `WatchdogSec=` (drop-ins included) runs a binary that never sends WATCHDOG=1 | sd_notify(3): the keep-alive ping services "need to issue" |

### Stage `config`: 2 rows (1 blocking, 1 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.no-secret-env` | block | `secret-read-from-environment` | production code reads a secret-named variable by `env::var`, `var_os`, `env!`, `option_env!`, dotenv or clap's `env =` | systemd.exec(5) Environment=; proc_pid_environ(5) |
| `rs.credentials-read` | advisory | `credential-never-read` | a unit loads a credential and its binary never reads `$CREDENTIALS_DIRECTORY`, and no `%d/` path is passed on its command | systemd.exec(5) LoadCredential= |

A variable's name is judged by durable-services' own `SECRET_NAME` (TOKEN, SECRET, PASSWORD,
API_KEY, PRIVATE_KEY, CREDENTIAL, DSN...) and `REFERENCE_NAME` (`*_SECRET`, `*_SECRET_NAME`: a
secret's id in a store, not the secret). A name ending `_FILE`, `_PATH`, `_DIR` or `_DIRECTORY`
is a path, and `CREDENTIALS_DIRECTORY` itself is the interface systemd documents.

### Stage `memory`: 3 rows (3 blocking, 0 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.bounded-channels` | block | `unbounded-channel` | production code opens `unbounded_channel`, `mpsc::unbounded`, `flume::unbounded`, `crossbeam_channel::unbounded`, or std's `mpsc::channel()` | tokio: an unbounded channel "may cause the process to run out of memory"; the owner's memory budget |
| `rs.bounded-body-reads` | block | `unbounded-body-read` | `to_bytes(.., usize::MAX)`, `Limited::new(.., usize::MAX)`, `DefaultBodyLimit::max(usize::MAX)`, `RequestBodyLimitLayer::new(usize::MAX)`, or hyper 0.14's `body::to_bytes` | axum `to_bytes`; the owner's memory budget |
| `rs.concurrency-bound` | block | `concurrency-unbounded` | an HTTP service applies no `ConcurrencyLimitLayer`, `GlobalConcurrencyLimitLayer`, `.concurrency_limit(..)`, `LoadShedLayer`, or an acquired `Semaphore` | tower limit; the owner's memory budget |

### Stage `runtime`: 1 row (0 blocking, 1 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `rs.no-blocking-in-async` | advisory | `blocking-in-async` | `thread::sleep` or `block_on` inside an `async fn` or `async` block, outside `spawn_blocking`, `block_in_place` or `thread::spawn` | tokio `spawn_blocking` |

## The practice, beyond what a row can see

### axum 0.8

- Paths are `/{id}` and `/{*rest}`. A model trained on 0.7 writes `/:id`, which panics when the
  router is built; `rs.axum-route-syntax` finds it before a deploy does.
- A custom extractor implements `FromRequestParts` with a native `async fn`; `#[async_trait]` is
  gone. An `Option<T>` extractor needs `OptionalFromRequestParts`. The compiler enforces both.
- A handler returns `Result<T, E>` where `E: IntoResponse`: one error type per service that maps
  each variant to a status. A timeout or a load-shed from tower is an error that
  `HandleErrorLayer` turns into a response; tower-http's `TimeoutLayer::with_status_code`
  answers 408 itself (`TimeoutLayer::new` is deprecated).
- The default body limit for `Bytes`, `String`, `Json` and `Form` is 2 MB. Raising it is
  `DefaultBodyLimit::max(n)`; switching it off without `RequestBodyLimitLayer` is
  web-security's `ws.request-body-limit`.

### tokio on 2 vCPU

- The multi-thread runtime starts one worker per core, two here. The blocking pool grows to 512
  threads by default, and a task queued past that waits in an unbounded queue. A service with
  heavy blocking work sets `max_blocking_threads` on a `runtime::Builder`.
- Blocking work goes to `spawn_blocking`; a `thread::sleep` or a `block_on` in an async body stalls
  a worker for every task on it.
- Shutdown in three steps (tokio's shutdown topic): detect it (the SIGTERM future), tell every
  task (a `CancellationToken`, cloned per task), and wait for them (a `TaskTracker`, closed, then
  awaited). A task can flush before it returns. Finish inside the unit's `TimeoutStopSec=`, after
  which systemd sends SIGKILL.

### Errors

A library crate returns its own `thiserror` enum, with `#[source]` or `#[from]` so the chain
survives; a caller matches on it. A binary uses `anyhow` with `.context(..)` at its edge. An axum
service converts its error type to a response in one `IntoResponse` impl, logging the chain there.

### Configuration and secrets

Configuration that varies between deploys comes from the environment (the twelve-factor app);
greenfield's `env-example` row keeps `.env.example` naming each variable. A secret never does:
systemd.exec(5) says environment variables "are not suitable for passing secrets", and
`/proc/<pid>/environ` keeps the initial environment for the life of the process. The unit loads it
with `LoadCredential=` (or `LoadCredentialEncrypted=` from `systemd-creds`), and the binary reads
the file under `$CREDENTIALS_DIRECTORY`. Parse all of it once, at start, into a typed struct, and
refuse to start on a bad value.

### The memory budget

- **The ceiling is the unit's.** `MemoryHigh=` throttles, `MemoryMax=` is the last line of
  defense, and the stack's share of the host is `deploy/host-budget.json`: durable-services'
  `resources.*` rows judge them, and observability's memory watch pages on what they catch.
- **Below the ceiling, bound everything that grows with load:** in-flight requests (a
  concurrency limit), request bodies (the 2 MB default, or a stated limit), queues (bounded
  channels, whose `send().await` is backpressure), and fan-out (`buffer_unordered(n)`, a
  `JoinSet` with a permit per task). Stream a large body (`Body::into_data_stream`) instead of
  collecting it.
- **Size the numbers from measurement.** In-flight requests times the largest body a request may
  hold should sit well under `MemoryHigh=`.
- **glibc's allocator** keeps per-thread arenas; `glibc.malloc.arena_max`, set through
  `GLIBC_TUNABLES`, caps them where resident memory creeps. Measure before setting it.

### The house template

`templates/` holds a service that every row here is green on:

- `service-main.rs.template`, which clippy passes with `-D warnings` under the lint table below;
- `workspace-lints.template.toml`, the lint levels, with groups at priority -1;
- `clippy.template.toml`;
- `service.template.service`, its unit.

Logging comes from the observability pack's `logging.rs.template`. The CI workflow is greenfield's
`ci.template.yml`, which already runs `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets --locked -- -D warnings`.

## What this pack composes, and never copies

| practice | owner | row |
|---|---|---|
| the toolchain pinned; the workspace shape and `[workspace.package]`; `.env.example` | greenfield | `toolchain-pinned`, `cargo-workspace`, `cargo-workspace-package`, `env-example` |
| axum 0.8, tokio 1, tracing 0.1 on their pinned majors | stack-selection | `pinned-majors` |
| a body limit switched off with none in its place; a rate bound; a key in source | web-security | `ws.request-body-limit`, `ws.rate-limit`, `ws.hide-keys` |
| the RustSec audit (`deny.toml`, `cargo deny` or `cargo audit` in CI) | web-security, with the policy shape of SPEC-V2-2199 | `ws.dep-audit-config` |
| a secret in a unit's environment | web-security; durable-services | `ws.unit-secret-env`; `secrets.environment-literal`, `secrets.credentials` |
| unit hardening, restart, stop timeout, MemoryHigh and MemoryMax | durable-services | `sandbox.*`, `service.*`, `resources.*` |
| SQLite through sqlx 0.9: WAL, `BEGIN IMMEDIATE`, migrations | ledger-sqlite | its portable rows (SPEC-V2-2197) |
| mutation testing of the crates | mutation-rows | its portable practice stage (SPEC-V2-2208) |
| tracing spans, logs, SLOs and alerts | observability | every `obs.*` row |

## Measured on a real service

Run against phoenix-v2's own proxy crate, a Rust axum service, the probe found what this pack
exists to catch: `axum::serve` with no graceful shutdown, no SIGTERM handler, a request body read
with `to_bytes(body, usize::MAX)`, no concurrency bound, and two credentials read from the
environment. phoenix-v2 is not this pack's subject (the pack is DeckStreak's), so those are
recorded in SPEC-V2-2220 §1, not refused here.

## What this pack does not do

- It never builds, runs or benchmarks a binary. Every row is a static read; a rule only runtime
  can show (the watchdog interval, the drain finishing inside `TimeoutStopSec=`, resident memory
  under load) is taught above.
- It does not follow data flow. A layer anywhere in a binary's sources counts for it, and a
  variable read through a computed name is not seen.
- It judges no SQL, no unit hardening and no dependency advisory; the table above names who does.

## Sources

The dated list, with the Context7 ids that answered, is SPEC-V2-2220's References.

- axum: https://docs.rs/axum/latest/axum/ · https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
- tokio: https://tokio.rs/tokio/topics/shutdown · https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.unbounded_channel.html ·
  https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html
- tower and tower-http: https://docs.rs/tower/latest/tower/limit/ · https://docs.rs/tower-http/latest/tower_http/
- errors: https://github.com/dtolnay/thiserror · https://github.com/dtolnay/anyhow
- lints: https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section ·
  https://doc.rust-lang.org/rustc/lints/levels.html · https://doc.rust-lang.org/clippy/ ·
  https://github.com/rust-lang/rust-clippy/issues/12729
- systemd: systemd.exec(5), systemd.service(5), systemd.kill(5), sd_notify(3) (systemd 255 man pages) ·
  https://12factor.net/config

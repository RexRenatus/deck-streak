---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
name: observability
description: Observability for Rust services on a small VM with journald. tracing spans and fields, structured JSON logs whose levels reach journald's priority, no secret in a span or event, request traces with the matched route and a request id, metric naming and label cardinality, SLOs with error budgets and multiwindow burn-rate alerts in one checkable deploy/slo.json, alerts delivered on one Telegram alert path, and a memory watch over MemoryHigh, MemoryMax and OOM kills. A static probe judges any repository through --root.
---

# packs/observability

What a DeckStreak service must let its owner see, and the check that keeps it so (SPEC-V2-2220,
ADR-V2-2220). The owner's scope for this pack: tracing spans and fields and structured logs to
journald; metrics; alerts through the Telegram alert path; SLOs and error budgets; and a memory
watch (MemoryHigh and MemoryMax, OOM signals).

The stack keeps it small on purpose: JSON logs on stdout, kept and rotated by journald, no
collector on a 1.9 GiB VM (the house radar's `journald` entry). A service's request events ARE its
metrics: the SLO evaluator counts them in the journal. A metrics exporter is optional, and the
metric rows judge one only where the code registers metrics.

`scripts/observability-probe.py` is the check. It is standard-library Python, it judges any tree
through `--root`, and every row below runs it. It reads the Rust workspace through
`scripts/rust-service-probe.py`'s model and the units through `scripts/durable-unit-lint.py`'s
parser. Which seats consume this pack is its catalog row's `consumes`, the one record of that
edge (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack observability --root PATH --format json
```

## Running it

- **Through phxd**: the command above. Every row runs
  `python3 {skills}/../scripts/observability-probe.py --root {root} check <id>` under a 60-second
  wall.
- **Directly**: `python3 scripts/observability-probe.py --root PATH check <id>`; add
  `--slo FILE` to read the declaration from somewhere other than `deploy/slo.json`.
- **Vendored**: copy `observability-probe.py`, `rust-service-probe.py` and
  `durable-unit-lint.py` into one directory. Missing either sibling is VOID.

Output and exits are rust-service's: one `<id>: <finding>` line per finding, then `examined N`;
0 green, 1 a finding, 2 usage, 3 VOID. The `slo`, `alerts` and `memory` stages judge a deployed
service, so a tree with no `deploy/` unit is VOID for them, never green. A card row carries only
the exit (i1879); run the class to read its findings.

## The subject

- **A service binary** is a workspace binary a `deploy/` service unit runs, long-running or
  timer-activated. An HTTP service is a binary whose sources call `axum::serve`; with no unit
  naming a workspace binary, the HTTP services stand in, and a CLI that prints to its terminal
  by design is never judged as a service.
- **`deploy/slo.json`**, schema `phx.slo.v1`, is the one checkable declaration of the SLOs, their
  error budget policy, the alert path, the evaluator and the memory watch. Unknown keys are
  refused. Its shape is `templates/slo.template.json`:
  - `slos[]`: `id`, `unit` (a deploy/ service), `sli` (`kind` availability or latency,
    `source` journal, metrics or probe, `good` and `total` stated in words, `threshold_ms` for
    latency, `probe_unit` for a probe), `objective` (strictly between 0 and 1), `window_days`,
    `rationale`, `alerts[]`, and optionally `expected_events_per_hour` and `low_traffic`;
  - each alert: `severity` (page or ticket), `long_window`, `short_window` (like `5m`, `1h`,
    `3d`), `burn_rate`, `budget_consumed`, `route`;
  - `error_budget_policy`: `on_exhaustion` (a list of actions), `escalation`, and optionally
    `postmortem_budget_fraction`;
  - `alerting`: `unit` (the template alert unit) and `channel` (`telegram`);
  - `evaluator`: its `unit` and `timer`; `memory_watch`: `unit`, `timer`, `units` (a list, or
    `"all"`).

## Severity

A `block` row is a firm requirement: a statement of the official documentation (tracing discards
events no subscriber collects; Prometheus says "Do not use labels to store dimensions with high
cardinality"), a security standard (OWASP's Logging Cheat Sheet on tokens, passwords, keys and
session ids), or an owner decision (JSON logs to journald and a TraceLayer on the house radar;
SLOs, error budgets, the Telegram alert path and the memory watch in this pack's scope). An
`advisory` row is practice the sources recommend; it reports and never refuses.

## The rows: 25 in six stages (13 blocking, 12 advisory)

### Stage `logs`: 7 rows (4 blocking, 3 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `obs.subscriber-installed` | block | `subscriber-missing` | a service binary installs no subscriber: no tracing-subscriber statement ending in `.init()` or `.try_init()`, and no `set_global_default(..)` | tracing: executables must install a subscriber, and events outside one are not collected |
| `obs.structured-logs` | block | `logs-unstructured` | a service binary's subscriber writes plain text: no `.json()` in a tracing-subscriber statement and no tracing-journald layer | the house radar's journald entry; OWASP: encode for the log format (CR and LF cannot forge a line) |
| `obs.journal-priority` | advisory | `journal-priority-flat` | JSON reaches stdout with no `<N>` sd-daemon prefix and no journald layer, or the unit sets `SyslogLevelPrefix=no` | systemd.exec(5) SyslogLevel=, SyslogLevelPrefix=; the predecessor measured every line at PRIORITY=6 (i343) |
| `obs.log-level` | advisory | `log-level-fixed-or-verbose` | the filter is not taken from the environment (`EnvFilter::try_from_default_env`), a level is hard-coded at DEBUG or TRACE, or a long-running unit sets `RUST_LOG` with debug or trace | journald.conf(5): past RateLimitBurst (10000 per 30s) lines are dropped |
| `obs.no-print-logging` | advisory | `print-logging` | a service binary's sources call `println!`, `print!`, `eprintln!`, `eprint!` or `dbg!` | clippy's `print_stdout` and `dbg_macro`; no level, no fields |
| `obs.no-secret-fields` | block | `secret-logged` | a span or event records a secret-like name: an `#[instrument]` argument not skipped, a `fields(..)` or event field, a message's `{name}` or a formatted argument | OWASP Logging Cheat Sheet, data to exclude |
| `obs.sensitive-headers` | block | `headers-logged-unmasked` | `include_headers(true)` with no `SetSensitiveRequestHeadersLayer` (or `SetSensitiveHeadersLayer`) applied | tower-http sensitive_headers; OWASP |

A secret-like name holds a part such as `token`, `secret`, `password`, `credential`, `apikey`,
`authorization`, `cookie` or `dsn`, or a pair such as `api_key`, `private_key`, `init_data`,
`session_id`, `database_url`; a name ending in `count`, `len`, `name`, `path`, `hash` or the like
is about a secret, not the secret (`token_count`, `secret_name`).

### Stage `traces`: 4 rows (1 blocking, 3 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `obs.http-trace-layer` | block | `requests-untraced` | an HTTP service applies no `TraceLayer::new_for_http()` (or `.trace_for_http()`) | the house radar's tower-http entry; the four golden signals |
| `obs.request-events-visible` | advisory | `request-events-debug` | the TraceLayer keeps `DefaultOnResponse`'s level, DEBUG, or `DefaultMakeSpan`'s, so at `info` no request is logged, or its event carries no method or path | tower-http trace: both default to `Level::DEBUG` |
| `obs.span-route` | advisory | `span-route-missing` | a traced service never uses axum's `MatchedPath`, so its span names the raw URI | OpenTelemetry HTTP semantic conventions: `http.route` is low-cardinality, and the URI path cannot substitute it |
| `obs.request-id` | advisory | `request-id-missing` | an HTTP service does not both set (`SetRequestIdLayer`) and propagate (`PropagateRequestIdLayer`) an `x-request-id` | tower-http request_id |

### Stage `metrics`: 2 rows (1 blocking, 1 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `obs.metric-names` | advisory | `metric-name-nonstandard` | a registered metric is not lower snake case, a counter lacks `_total` or another kind has it, a unit is not a base unit (`_ms` for `_seconds`), or the names carry more than one application prefix | Prometheus metric and label naming |
| `obs.metric-labels` | block | `metric-label-unbounded` | a metric label key is an id (`id`, `*_id`), an email, an IP, a URL or path, a username, a chat, a token or a session | Prometheus: "Do not use labels to store dimensions with high cardinality" |

### Stage `slo`: 7 rows (4 blocking, 3 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `obs.slo-declared` | block | `slo-undeclared` | `deploy/slo.json` is missing or malformed (a wrong schema, an unknown key, an objective of 1 or a string, a boolean window, a latency SLI with no threshold, an unknown unit), or an HTTP service's unit has no SLO | SRE book, service level objectives; SRE workbook, implementing SLOs |
| `obs.error-budget-policy` | block | `error-budget-policy-missing` | there is no `error_budget_policy`, no action on exhaustion, or no escalation | SRE workbook, example error budget policy |
| `obs.burn-rate-alerts` | block | `burn-rate-alerts-incoherent` | an SLO has no page alert; an alert's burn rate is not `budget_consumed x window hours / long window hours`; a short window is not shorter; or `burn_rate x (1 - objective)` exceeds 1, so the alert can never fire (i397) | SRE workbook, alerting on SLOs |
| `obs.slo-measurable` | block | `slo-unmeasured` | no evaluator unit with a timer that starts it; a journal SLI whose service has no TraceLayer or keeps its request events at DEBUG; a metrics SLI whose service registers none; a probe SLI whose probe unit is not shipped | the SLI must exist to be counted (i397) |
| `obs.slo-window-weeks` | advisory | `slo-window-not-weeks` | `window_days` is not a multiple of seven | SRE workbook: an integral number of weeks keeps the weekends constant |
| `obs.alert-short-window` | advisory | `short-window-not-twelfth` | a short window is not within a quarter of a twelfth of its long window | SRE workbook: the short window is 1/12 of the long one |
| `obs.low-traffic` | advisory | `low-traffic-pages` | the traffic is unstated, or so low that one failure among the long window's events pages; unless `low_traffic` names `synthetic`, `grouped`, `longer-window` or `lower-objective` | SRE workbook, low-traffic services |

### Stage `alerts`: 3 rows (2 blocking, 1 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `obs.alert-route` | block | `alert-path-broken` | `alerting.unit` is not a shipped template unit (`name@.service`), its command never reaches `api.telegram.org` or `sendMessage`, the channel is not `telegram`, or an SLO alert routes elsewhere | the owner's one Telegram alert path |
| `obs.failure-alerts` | block | `failure-unalerted` | a long-running or timer-activated service lacks `OnFailure=` naming an instance of that template with a specifier (`%n`, `%p`, `%i`) | systemd.unit(5) OnFailure=; systemd.exec(5): $MONITOR_* is not passed when several units share one handler |
| `obs.alert-names-result` | advisory | `alert-result-unnamed` | the alert's script never reads `$MONITOR_SERVICE_RESULT`, or names neither `$MONITOR_UNIT` nor `%i` (shell comments do not count) | systemd.exec(5), $MONITOR_* (systemd 251 and later) |

### Stage `memory`: 2 rows (1 blocking, 1 advisory)

| id | severity | reason | refuses when | source |
|---|---|---|---|---|
| `obs.memory-watch` | block | `memory-unwatched` | no `memory_watch`; its unit or timer is not shipped or not linked; its command reads no memory accounting (`memory.events`, `memory.pressure`, `memory.current`, `MemoryCurrent`); or a long-running service is outside its scope | the kernel's cgroup v2 memory.events; the predecessor's render job had no MemoryMax, OOMPolicy or page (i345) |
| `obs.oom-policy` | advisory | `oom-kill-silenced` | a long-running service sets `OOMPolicy=continue`, drop-ins included | systemd.service(5) OOMPolicy= |

## The practice, beyond what a row can see

### Logs and spans

- Install one subscriber at start: JSON events on stdout, the level from `RUST_LOG` (info when
  unset), each line prefixed with its sd-daemon level so journald keeps the priority
  (`templates/logging.rs.template`). tracing-journald's layer is the accepted alternative: native
  journal fields and priorities, no JSON.
- Name fields once, the same in every service (`route`, `status`, `latency`, `request_id`), so the
  evaluator and a person searching read one vocabulary. Declare a field a span records later with
  `tracing::field::Empty`; recording an undeclared field does nothing.
- `#[instrument(skip_all, fields(..))]` on a handler records only what it names. Never a token,
  never Telegram's init data, never a whole config struct.
- Log the security events OWASP lists: authentication failures, authorization failures, input
  validation failures, start and stop.

### Traces as the service's metrics

The TraceLayer's response event carries `status` and `latency` for every request. At INFO it is in
the journal, and the SLO evaluator counts it: that is the four golden signals' latency, traffic
and errors, with no exporter. Saturation is the memory watch. Keep a slow error visible as an
error: a latency SLI counts every request, failed or not.

### A metrics exporter, if one is added

Prometheus naming: one application prefix, base units (`_seconds`, `_bytes`), `_total` on a
counter, a histogram for latency (never an average). Labels stay bounded (route, status, method);
a user or chat id is a new series each. On this VM it is a deviation from the house radar, so it
needs an ADR (stack-selection's `deviation-has-adr`).

### SLOs and error budgets

- An SLI is good events over total events. An objective is strictly below 100%; the budget is the
  rest. Start loose and tighten; keep few SLOs, each one a user would notice.
- A four-week rolling window. The page alerts, for a 28-day window: 2% of the budget in 1h
  (13.44x, short window 5m) and 5% in 6h (5.6x, 30m); the ticket: 10% in 3d (0.93x, 6h). For a
  30-day window the same fractions give 14.4x, 6x and 1x.
- A low-traffic service pages on one failure. Group it, add a synthetic probe, lengthen the
  window, or lower the objective, and say which in `low_traffic`.
- The error budget policy says what stops when the budget is spent, and who decides a disputed
  count.

### The alert path

One template unit, `deploy/<product>-alert@.service`, reached by `OnFailure=` from every service,
scheduled job, the evaluator and the memory watch. Its script (`templates/alert-telegram.template.sh`)
reads the bot token and chat id from `$CREDENTIALS_DIRECTORY` and sends the failed unit, its
`$MONITOR_SERVICE_RESULT` and its last error lines. The evaluator and the watch page by failing:
they print the reason at `<3>` and exit 1, edge-triggered through `$STATE_DIRECTORY`, so a burn
pages once. Telegram allows about one message a second per chat; retries and 429 handling are the
telegram-platform pack's.

### The memory watch

`templates/memory-watch.template.sh` reads each service's `memory.events` every minute: a new
`oom_kill` or `max` event pages, a new `high` event (MemoryHigh throttling) is logged, and usage
above 90% of `memory.max` is logged. `MemoryPressureWatch=on` gives a service
`$MEMORY_PRESSURE_WATCH` to react itself (drop caches, `malloc_trim`); systemd calls that optional.

## Templates

`templates/` holds the deployable pieces, every row green on them:

- `logging.rs.template`, which clippy passes with `-D warnings`;
- `slo.template.json`;
- `slo-evaluate.template.py`, with its `.service` and `.timer`;
- `alert@.template.service` and `alert-telegram.template.sh`;
- `memory-watch.template.sh`, with its `.service` and `.timer`.

## What this pack composes, and never copies

| practice | owner | row |
|---|---|---|
| MemoryHigh below MemoryMax, the host budget, MemoryMax on every job | durable-services | `resources.memory-order`, `resources.budget`, `resources.memory-max`, `resources.memory-high` |
| journald's size cap, output to the journal, a stable identifier | durable-services | `logging.journal-cap`, `logging.journal`, `logging.identifier` |
| an OnFailure= at all, a watchdog in the unit | durable-services | `service.on-failure`, `service.watchdog` |
| the alert unit's secrets through LoadCredential | durable-services; web-security | `secrets.credentials`; `ws.unit-secret-env` |
| Telegram's rate limits, 429 `retry_after`, message length | telegram-platform | its Bot API rows (SPEC-V2-2219) |
| the notification policy for users, never for alerts | notifications-policy | its router rows (SPEC-V2-2219) |
| the subscriber's crate on its pinned major; OpenTelemetry at assess | stack-selection | `pinned-majors`, `deviation-has-adr` |
| graceful shutdown, sd_notify, secrets in code | rust-service | `rs.*` |

## What this pack does not do

- It queries no journal, no Prometheus and no live unit. Whether the evaluator runs, whether a
  page reached the phone, and what the burn rate is now are the box's facts.
- It does not judge the evaluator's arithmetic in production; the template is the tested
  reference, and a project that writes its own evaluator owns its correctness.
- It does not follow data flow: a secret renamed before it is logged is not seen.

## Sources

The dated list, with the Context7 ids that answered, is SPEC-V2-2220's References.

- tracing: https://docs.rs/tracing/latest/tracing/ · https://docs.rs/tracing-subscriber/latest/tracing_subscriber/ ·
  https://docs.rs/tracing-journald/latest/tracing_journald/ · https://docs.rs/tower-http/latest/tower_http/trace/
- SRE: https://sre.google/sre-book/service-level-objectives/ · https://sre.google/sre-book/monitoring-distributed-systems/ ·
  https://sre.google/workbook/implementing-slos/ · https://sre.google/workbook/alerting-on-slos/ ·
  https://sre.google/workbook/error-budget-policy/
- Prometheus: https://prometheus.io/docs/practices/naming/ · https://prometheus.io/docs/practices/instrumentation/
- OWASP: https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
- OpenTelemetry: https://opentelemetry.io/docs/specs/semconv/http/http-spans/
- systemd and the kernel: systemd.exec(5), systemd.service(5), systemd.resource-control(5), journald.conf(5)
  (systemd 255 man pages) · https://systemd.io/PRESSURE/ · https://docs.kernel.org/admin-guide/cgroup-v2.html
- Telegram: https://core.telegram.org/bots/faq

# SPEC-031: every role logs one way, the API has an SLO with burn-rate pages, and every failure reaches the owner through one alert unit

- **Wave:** W0. **Issue:** #24 (epic #1). **Context(s):** `deck-streak-daemon` (the roles' logging), `deck-streak-kernel` (the panic hook of the one logging setup), `deploy` (the alert unit, the SLO evaluator, the memory watch, `deploy/slo.json`), `repo` (`.packs/wiring.json`).
- **Decided by:** ADR-003 (tracing JSON to journald), ADR-010 (`OnFailure=` the Telegram alert template unit), ADR-038 (credentials by `LoadCredential=` from the credential socket, superseding ADR-010's `LoadCredentialEncrypted=`), ADR-020 (the kernel's one logging setup), ADR-025 (the API's trace layer), ADR-032 (the budget of these units), and this SPEC's ADR-031 (an SLO sized for one owner's traffic, and an alert path that never puts the token on a command line).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-031.md`. The delivery
  amended §1, R1, R3 to R7, §3 (A8), §4 and §6, each for the reason §7 gives.

## 1. The problem, measured

- **What exists at this SPEC's base.** The kernel's `logging::install` (SPEC-020), which `main`
  already calls before a role reads any setting (SPEC-025, SPEC-027), the API's trace layer with
  the matched route and request id (SPEC-025), and the unit templates with
  `OnFailure=deck-streak-alert@%n.service` naming a unit that does not exist yet (SPEC-032).
  `.packs/wiring.json` holds observability pending on this issue, with seven rows (`obs.slo-declared`,
  `obs.error-budget-policy`, `obs.burn-rate-alerts`, `obs.slo-measurable`, `obs.alert-route`,
  `obs.failure-alerts`, `obs.memory-watch`) deferred to it by SPEC-032.
- **A panic bypasses the one logging setup.** Rust's default panic hook prints a panic's message to
  stderr as plain text, past the redacting writer, so a credential the message carried would reach
  the journal whole (found by SPEC-025's delivery).
- **The predecessor's operations this carries** (predecessor `27ee2bc`, names only): logs as one JSON
  object per line with a syslog priority prefix (`logging_setup.py:configure`); one Telegram path for
  infrastructure pages whose bodies carry integers only
  (`pipeline_layers/ops.py:OpsLayer._check_deadman`); a memory ceiling whose breach went unpaged on a
  render job (the observability pack's lesson on the predecessor's daily render job). The predecessor
  had no SLO.
- **The pack's template has a leak this SPEC must not copy.** The observability pack's
  `alert-telegram.template.sh` passes the bot token inside the request URL on `curl`'s command line,
  where any local user can read it in the process table while the request runs; CHARTER 15 forbids a
  secret on argv. DeckStreak's script hands the URL to `curl` through a configuration read on
  standard input.
- **One owner's traffic** is a few hundred API requests on a study day and none at night, so the
  pack template's one-hour page at 99.5% would page on two failed requests (ADR-031).

**Order.** After SPEC-032 (the API unit the SLO names, and the units whose `OnFailure=` this SPEC's
alert unit answers) and SPEC-025 (the trace events the SLO counts). It is the last W0 delivery on
its branch; nothing in W0 waits for it.

## 2. Requirements

R1. Every role of `deckstreakd` installs the kernel's logging before it reads a setting, so its first
    line, even a refusal to start, is a JSON event prefixed with its journal priority; and that
    logging replaces Rust's default panic hook, so a panic on any thread is one ERROR event through
    the same redacting writer, never the default hook's plain text on stderr.
R2. `deploy/slo.json` (schema `phx.slo.v1`) declares `api-availability` for
    `deck-streak-api.service`: a journal SLI (good: a response event whose status is below 500; total:
    every response event), objective 0.99 over 28 days, `low_traffic: longer-window`,
    `expected_events_per_hour` 20, a page alert at 5% of the budget over 6h (short window 30m, burn
    rate 5.6) and a ticket at 10% over 3d (short window 6h, burn rate 0.9333), each routed to
    `telegram`; an error budget policy that freezes feature releases on exhaustion and names the owner
    as the arbiter of a disputed count; `alerting` naming `deck-streak-alert@.service`; the evaluator
    `deck-streak-slo.service` and its timer; and the memory watch over every unit (ADR-031).
R3. `deploy/systemd/deck-streak-alert@.service` (a `oneshot`) runs `deploy/scripts/alert-telegram.sh`
    with `%i`; the script reads the bot token and the chat id from `$CREDENTIALS_DIRECTORY`
    (`telegram-bot-token`, `owner-user-id`: a private chat's id is its user's), names the failed unit
    (`$MONITOR_UNIT`, else `%i`) and its result (`$MONITOR_SERVICE_RESULT`), quotes at most the failed
    run's last five error lines (`$MONITOR_INVOCATION_ID`, else the unit's) within 3500 bytes cut at
    a character boundary, and gives `curl` the request URL and its fields through a configuration on
    standard input, so neither the token nor the chat id appears on a command line.
R4. `deploy/scripts/slo-evaluate.py` (standard library only), run by `deck-streak-slo.timer` every five
    minutes (the pack's `slo-evaluate.template.timer`), counts the API's response events in the
    journal for each alert's long and short windows and exits 1 (paging through `OnFailure=`) when
    both exceed the alert's burn rate, once per burn episode, remembering the episode in
    `$STATE_DIRECTORY`. A ticket reaches the owner on that path too, and a run that cannot measure
    pages once per episode of its own.
R5. `deploy/scripts/memory-watch.sh`, run by `deck-streak-memory-watch.timer` every minute (the pack's
    `memory-watch.template.timer`), reads the `memory.events` of every DeckStreak unit's cgroup, a
    template's instances included: a new `oom_kill` or `max` event pages (exit 1), and a new `high`
    event and usage crossing 90% of `memory.max` are logged; each event pages once, remembered in
    `$STATE_DIRECTORY` with its cgroup's identity.
R6. The three units carry the budget ADR-032 gives them and the hardening of SPEC-032's units,
    narrowed where a unit needs less: none reads the settings file, the alert writes nothing, the
    evaluator and the watch keep their episodes in directories of their own and open no network
    socket, and only the alert and the evaluator join the journal's group. The alert unit loads each
    of its two credentials as `LoadCredential=<id>:/run/deck-streak-credentials/socket` (ADR-038),
    never `LoadCredentialEncrypted=` or an `Environment=` value; `deploy/host-budget.json` gains
    their entries.
R7. `.packs/wiring.json` keeps observability `enforced` and removes six of the seven deferrals SPEC-032
    placed on it; every observability row is green over the tree but `obs.slo-declared`, which stays
    deferred to #42 (§7).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every role of the binary logs JSON with its priority from its first line | daemon `logging` test; `obs.subscriber-installed`, `obs.structured-logs`, `obs.journal-priority` |
| A2 | the API's SLO burn rates equal the budget consumed times the window over the long window, and none is unreachable | `test_slo_declaration.py`; `obs.burn-rate-alerts`, `obs.slo-declared` |
| A3 | the alert script reads its token and chat id from the credentials directory and names the failed unit and its result | `test_alert_unit.py`; `obs.alert-route`, `obs.alert-names-result` |
| A4 | the bot token never appears on the alert's command line | `test_alert_unit.py` |
| A5 | the evaluator pages once when the page window burns, and not again while it burns | `test_slo_evaluator.py`; `obs.slo-measurable` |
| A6 | the memory watch pages on a new `oom_kill`, and only logs a new `high` event | `test_memory_watch.py`; `obs.memory-watch` |
| A7 | every service and timer-activated unit names the alert template on failure with a specifier | `test_alert_unit.py`; `obs.failure-alerts` |
| A8 | a panic whose message holds a registered credential logs one redacted JSON event at error priority, and nothing on stderr | kernel `logging` test; `obs.no-secret-fields` |

```acceptance
A1: cargo test -p deck-streak-daemon --test logging -- --exact every_role_logs_json_with_its_priority_from_its_first_line
A2: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k the_api_slo_burn_rates_equal_budget_times_window_over_long_window
A3: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_alert_script_reads_its_credentials_and_names_the_unit_and_result
A4: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_bot_token_never_appears_on_the_command_line
A5: python3 -m unittest discover -s scripts/tests -p test_slo_evaluator.py -k the_evaluator_pages_once_when_the_page_window_burns
A6: python3 -m unittest discover -s scripts/tests -p test_memory_watch.py -k the_watch_pages_on_a_new_oom_kill_and_logs_a_new_high_event
A7: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k every_unit_names_the_alert_template_on_failure
A8: cargo test -p deck-streak-kernel --test logging -- --exact a_panic_logs_one_redacted_json_event_and_never_the_value
```

A3 and A4 run the script as its unit runs it, with stub `curl` and `journalctl` executables first on
its `PATH` (each records its argument vector, standard input and environment to a file in a
`TemporaryDirectory`) and every other command the script may call wrapped to record the same before
it runs the real one, a temporary credentials directory holding a synthetic value for each credential
the unit loads, and `MONITOR_UNIT` and `MONITOR_SERVICE_RESULT` set on the child process. A5 feeds
the evaluator a synthetic journal export under a fixed `--now`; A6 a synthetic `memory.events` tree
through `--cgroup-root`; neither reads the host. A1 and A8 read a child process's own stdout and
stderr.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/daemon/tests/logging.rs` | `deck-streak-daemon` | added: A1 |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | unchanged: `main` already installs the logging before a role reads a setting (§7) |
| `crates/kernel/src/logging.rs` | `deck-streak-kernel` | changed: `install` replaces the default panic hook (R1) |
| `crates/kernel/tests/logging.rs` | `deck-streak-kernel` | changed: A8 |
| `deploy/slo.json` | deploy | added |
| `deploy/systemd/deck-streak-alert@.service` | deploy | added |
| `deploy/scripts/alert-telegram.sh` | deploy | added |
| `deploy/systemd/deck-streak-slo.service`, `deploy/systemd/deck-streak-slo.timer` | deploy | added |
| `deploy/scripts/slo-evaluate.py` | deploy | added |
| `deploy/systemd/deck-streak-memory-watch.service`, `deploy/systemd/deck-streak-memory-watch.timer` | deploy | added |
| `deploy/scripts/memory-watch.sh` | deploy | added |
| `deploy/host-budget.json` | deploy | changed: the three units' entries |
| `deploy/README.md` | deploy | changed: the three units, their credentials, their scripts' paths and the runbook |
| `scripts/tests/test_slo_declaration.py`, `test_alert_unit.py`, `test_slo_evaluator.py`, `test_memory_watch.py` | repo | added: A2 to A7 |
| `scripts/tests/test_deploy_templates.py` | repo | changed: SPEC-032's tables extended to the three units |
| `scripts/tests/fixtures/journal/`, `scripts/tests/fixtures/cgroup/` | repo | added: synthetic journal export and memory accounting |
| `.packs/wiring.json` | repo | changed: six of SPEC-032's seven deferrals removed |
| `docs/schematics/alert-and-slo-path.md` | repo | changed: redrawn for the delivery |
| `docs/decisions/ADR-031-an-slo-sized-for-one-owner.md` | repo | changed: accepted, and decided at delivery |
| `docs/specs/SPEC-031-observability-foundation.md` | repo | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-031.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It exports no Prometheus metric and registers none: the request events are the metrics
  (#18).
- It declares no latency SLO and no SLO for the bot or the jobs; each gains one when its traffic is
  measured on the host (#42).
- It routes no alert through the notification router; the router's alert kind carries the in-app
  and recovery messages (#27).
- It installs nothing on a host (#42).
- It measures no web vitals for the Mini App (#60).

## 6. Risks

- **The page window is too slow for an outage.** A 6-hour window with a 5.6 burn pages after about
  twenty minutes of total failure (5.6% of six hours), plus up to one five-minute evaluation; the
  dead-man watch and the units' own failures page faster for the outages that stop a service, and
  ADR-031 records the trade.
- **The evaluator miscounts because the trace event changed shape.** A5's fixture is a journal export
  of the event SPEC-025 emits, and a test holds every response event of it to that event's flattened
  fields; a changed field name fails A5 before it reaches a host. The evaluator also reads an event
  nested under `fields`, the JSON formatter's default, should the kernel's format stop flattening.
- **Several pages on a crash loop.** systemd starts `OnFailure=` units on the failed state an
  automatic restart passes through (systemd 254 and later, with `RestartMode=` at its default), so a
  daemon in a crash loop pages at each failure until its start limit (5 in 300 seconds, SPEC-032)
  ends the loop, a handful of pages in about a minute. A page that is still running absorbs the next
  failure's start. The evaluator and the watch page once per episode.
- **Telegram is unreachable when a page is sent.** `curl` retries three times with a delay; a page
  that still fails leaves the alert unit failed, visible in `systemctl --failed` and the journal.
- **The secret manager is unreachable when a page is due.** The alert unit fetches its two
  credentials at each start (ADR-038), so it cannot start and the page is lost with it; the failure
  stays visible in `systemctl --failed` and the journal, like a Telegram outage.
- **A hardening option stops `journalctl`.** The alert and the evaluator read the journal under
  SPEC-032's sandbox; a denied read quotes no line in a page, or pages the evaluator's own failure
  once, and the first start on the host shows it (#42).
- **Every ceiling at once exceeds the share.** ADR-032 sizes DeckStreak's 640M share as the daemons'
  ceilings plus the largest oneshot's (608M), but the evaluator and the watch run beside the jobs, so
  every ceiling reached at once is 768M. Their ceilings are bounds, far above what a script uses, and
  the memory watch reports any unit nearing its own; a new share is an owner decision (ADR-032).

## 7. Amended in delivery

The code, the packs' measurements and systemd's behaviour changed these statements of the planned
SPEC. Each is corrected above; the reasons are these.

- **R1, §1 and A8: the panic hook.** SPEC-025's delivery found the default panic hook writing a
  handler's message to stderr past the redacting writer, and R1's promise that every line is a JSON
  event did not hold for a panic. The kernel's `install` now replaces the hook, and A8 proves it: a
  panic on a thread of its own, whose message carries a registered credential, leaves exactly one
  ERROR event with the redaction marker, and nothing on stderr. The hook lives in the kernel's one
  setup rather than in `main`, so no process that logs can install half of it (ADR-031, decided at
  delivery).
- **A1 and the manifest's `main.rs`: not red.** `main` already installed the logging as its first
  statement (SPEC-025, SPEC-027), so A1 passed at its red commit; `main.rs` is unchanged. A1 reads
  the roles and the jobs from the binary's own usage line, so it judges a role added later without
  a change.
- **R3: the failed run, bytes, and the chat id.** The alert quotes the failed run's own lines,
  matched by `$MONITOR_INVOCATION_ID`, so an OOM kill that wrote nothing quotes nothing rather than an
  older run's lines. The bound is 3500 bytes, cut at a character boundary: a byte bound can never
  exceed 3500 characters, and the pack template's `cut -c1-3500` bounds each line rather than the
  text and, in GNU `cut`, counts bytes and splits a character. The owner's id is a credential too, so
  it travels on standard input beside the token.
- **R4: tickets and a blind evaluator.** R2 routes the ticket to `telegram`, and on a one-owner
  service a ticket printed only to the journal reaches no one, so the ticket pages once per episode on
  the same path, worded as a ticket. A run that cannot measure would otherwise fail its unit, and page,
  every five minutes; it is an episode of its own instead.
- **R5: which units, and a restart's events.** The template takes its units by name, which misses a
  template's instances; the watch finds every `deck-streak-*.service` cgroup under the system slice.
  A counter compared with the last run alone misses the events of a cgroup a restart made anew, whose
  count may equal the old one's, so each unit's counters are kept with its cgroup's inode. The 90%
  line is logged when usage crosses it, not every minute it stays.
- **R6: narrower hardening.** SPEC-032's `EnvironmentFile=` is required, so an alert unit carrying it
  could not start, and could not page, when the settings file was missing; none of the three reads a
  setting. The alert needs no writable directory, and the evaluator and the watch need neither the
  API's directory nor the network. `scripts/tests/test_deploy_templates.py` extends SPEC-032's tables
  by content: the role and credential tables, the timer prefix, the oneshots, the waived set, and one
  per-service table for the state directory, the settings file, the address families and the journal
  group, each value exact, SPEC-032's own unchanged; its reader of ADR-032's table accepts the
  `(SPEC-031)` that three rows carry.
- **R7: one deferral stays.** With the SLO declared, `obs.slo-declared` refuses the bot's unit: the
  probe reads a unit as an HTTP service when its binary serves HTTP, and `deckstreakd` is one binary
  whose `api` role serves while its `bot` role polls. The bot's SLO is excluded until its traffic is
  measured on the host (§5, #42), and one declared now over the journal would count no response and
  never burn. The row stays deferred to #42, where it turns stale, and so must be lifted, the day the
  bot gains its SLO; the other six rows are lifted, and every other observability row is green.
- **§6: a crash loop pages more than once.** systemd's NEWS for 254 adds `RestartMode=direct` to skip
  the inactive and failed states an automatic restart otherwise passes through, "so that dependent
  units are not notified"; at the default, each failure a restart follows starts `OnFailure=` units,
  as systemd's service state machine shows. The units keep the default, since a crash that a restart
  heals should still page.
- **The manifest.** The kernel's logging and its tests, SPEC-032's template tests, and
  `deploy/README.md` join it for the reasons above; the schematic and ADR-031 are changed rather than
  added, since the plan wrote them.
- **The timers.** Each spreads its fires with `RandomizedDelaySec=` and `AccuracySec=1us`, as the
  durable pack's `timers.spread` asks, since neither keeps a minute of its own, and each waives
  `timers.catch-up` with its why: a run missed while the host was down has nothing a later run lacks.

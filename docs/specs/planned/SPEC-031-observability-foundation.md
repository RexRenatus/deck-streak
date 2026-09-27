# SPEC-031: every role logs one way, the API has an SLO with burn-rate pages, and every failure reaches the owner through one alert unit

- **Wave:** W0. **Issue:** #24 (epic #1). **Context(s):** `deck-streak-daemon` (the roles' logging), `deploy` (the alert unit, the SLO evaluator, the memory watch, `deploy/slo.json`), `repo` (`.packs/wiring.json`).
- **Decided by:** ADR-003 (tracing JSON to journald), ADR-010 (`OnFailure=` the Telegram alert template unit; credentials by `LoadCredentialEncrypted=`), ADR-020 (the kernel's one logging setup), ADR-025 (the API's trace layer), ADR-032 (the budget of these units), and this SPEC's ADR-031 (an SLO sized for one owner's traffic, and an alert path that never puts the token on a command line).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-031.md` (ADR-016).

## 1. The problem, measured

- **What exists at this SPEC's base.** The kernel's `logging::install` (SPEC-020), the API's trace
  layer with the matched route and request id (SPEC-025), and the unit templates with
  `OnFailure=deck-streak-alert@%n.service` naming a unit that does not exist yet (SPEC-032).
  `.packs/wiring.json` holds observability pending on this issue, with seven rows (`obs.slo-declared`,
  `obs.error-budget-policy`, `obs.burn-rate-alerts`, `obs.slo-measurable`, `obs.alert-route`,
  `obs.failure-alerts`, `obs.memory-watch`) deferred to it by SPEC-032.
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
    line, even a refusal to start, is a JSON event prefixed with its journal priority.
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
    (`$MONITOR_UNIT`, else `%i`) and its result (`$MONITOR_SERVICE_RESULT`), quotes at most the unit's
    last five error lines within 3500 characters (the pack's alert script template), and gives
    `curl` the request URL through a configuration on standard input, so the token never appears on
    a command line.
R4. `deploy/scripts/slo-evaluate.py` (standard library only), run by `deck-streak-slo.timer` every five
    minutes (the pack's `slo-evaluate.template.timer`), counts the API's response events in the
    journal for each alert's long and short windows and exits 1 (paging through `OnFailure=`) when
    both exceed the alert's burn rate, once per burn episode, remembering the episode in
    `$STATE_DIRECTORY`.
R5. `deploy/scripts/memory-watch.sh`, run by `deck-streak-memory-watch.timer` every minute (the pack's
    `memory-watch.template.timer`), reads each DeckStreak unit's `memory.events`: a new `oom_kill` or
    `max` event pages (exit 1), a new `high` event and a usage above 90% of `memory.max` are logged;
    each event pages once, remembered in `$STATE_DIRECTORY`.
R6. The three units carry the budget ADR-032 gives them, the hardening of SPEC-032's units, and
    `LoadCredentialEncrypted=` for the alert unit's two credentials; `deploy/host-budget.json` gains
    their entries.
R7. `.packs/wiring.json` moves observability to `enforced` and removes the seven deferrals SPEC-032
    placed on it; every observability row is green over the tree.

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

```acceptance
A1: cargo test -p deck-streak-daemon --test logging -- --exact every_role_logs_json_with_its_priority_from_its_first_line
A2: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k the_api_slo_burn_rates_equal_budget_times_window_over_long_window
A3: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_alert_script_reads_its_credentials_and_names_the_unit_and_result
A4: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k the_bot_token_never_appears_on_the_command_line
A5: python3 -m unittest discover -s scripts/tests -p test_slo_evaluator.py -k the_evaluator_pages_once_when_the_page_window_burns
A6: python3 -m unittest discover -s scripts/tests -p test_memory_watch.py -k the_watch_pages_on_a_new_oom_kill_and_logs_a_new_high_event
A7: python3 -m unittest discover -s scripts/tests -p test_alert_unit.py -k every_unit_names_the_alert_template_on_failure
```

A3 and A4 run the script with stub `curl` and `journalctl` executables first on its `PATH` (each
records its argument vector and standard input to a file in a `TemporaryDirectory`), a temporary
credentials directory holding synthetic values, and `MONITOR_UNIT` and `MONITOR_SERVICE_RESULT` set
on the child process. A5 feeds the evaluator a synthetic journal export; A6 a synthetic
`memory.events` tree; neither reads the host.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/daemon/tests/logging.rs` | `deck-streak-daemon` | added: A1 |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | changed only if a role reads a setting before logging is installed |
| `deploy/slo.json` | deploy | added |
| `deploy/systemd/deck-streak-alert@.service` | deploy | added |
| `deploy/scripts/alert-telegram.sh` | deploy | added |
| `deploy/systemd/deck-streak-slo.service`, `deploy/systemd/deck-streak-slo.timer` | deploy | added |
| `deploy/scripts/slo-evaluate.py` | deploy | added |
| `deploy/systemd/deck-streak-memory-watch.service`, `deploy/systemd/deck-streak-memory-watch.timer` | deploy | added |
| `deploy/scripts/memory-watch.sh` | deploy | added |
| `deploy/host-budget.json` | deploy | changed: the three units' entries |
| `scripts/tests/test_slo_declaration.py`, `test_alert_unit.py`, `test_slo_evaluator.py`, `test_memory_watch.py` | repo | added: A2 to A7 |
| `scripts/tests/fixtures/journal/`, `scripts/tests/fixtures/cgroup/` | repo | added: synthetic journal export and memory accounting |
| `.packs/wiring.json` | repo | changed: observability enforced, SPEC-032's deferrals removed |
| `docs/schematics/alert-and-slo-path.md` | repo | added |
| `docs/decisions/ADR-031-an-slo-sized-for-one-owner.md` | repo | added |
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
  twenty minutes of total failure (5.6% of six hours), plus up to one five-minute evaluation; the dead-man watch and the units' own failures page faster for the
  outages that stop a service, and ADR-031 records the trade.
- **The evaluator miscounts because the trace event changed shape.** A5's fixture is a journal export
  of the event SPEC-025 emits; a changed field name fails A5 before it reaches a host.
- **An alert storm on a crash loop.** A unit's start limit (5 in 300 seconds, SPEC-032) ends a crash
  loop in a failed state, which pages once; the evaluator and the watch page once per episode.
- **Telegram is unreachable when a page is sent.** `curl` retries three times with a delay; a page
  that still fails leaves the alert unit failed, visible in `systemctl --failed` and the journal.

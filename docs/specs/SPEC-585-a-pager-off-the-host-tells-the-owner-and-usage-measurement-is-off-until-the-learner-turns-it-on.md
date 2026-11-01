# SPEC-585: a pager off the host tells the owner, the server counts its requests, and usage measurement is off until the learner turns it on

- **Issue:** #1069. **Context(s):** `deploy` (`deploy/slo.json`), `docs` (the pager's
  declaration, the privacy page, the records), `repo` (`scripts/tests`, the rows band).
- **Decided by:** ADR-604 (this SPEC's own: the pager, what replaces the second route, the log
  store's window, the request counters, the usage switch, the records, the shape and FORMAL),
  ADR-601 (the Telegram-free install and its journal sender), ADR-031 and SPEC-031 (the SLO and the
  one alert path, amended here), ADR-410 and SPEC-396 (the second route, superseded here).
- **Status:** delivered with its tests and `docs/red-first/SPEC-585.md`.

## 1. The problem, measured

Each fact below was read at the `dev` commit this SPEC's pull request names as its design base,
with `git show <sha>:<path>` and `git grep -n`, and the Telegram-free install's facts from
SPEC-582 and ADR-601.

- **The route.** `deploy/slo.json` routes its four alerts to `telegram` (lines 25, 33, 57 and 65)
  and names `alerting` `{"unit": "deck-streak-alert@.service", "channel": "telegram"}` (lines
  77-80). SPEC-582 R8 makes every route and the channel `journal`.
- **The sender.** The alert template runs a Telegram sender with two credentials
  (`deploy/systemd/deck-streak-alert@.service` lines 17 and 25-26) and names no `OnFailure=`
  (lines 9-10). SPEC-582 R7 gives it the journal sender: one line to standard error with the `<3>`
  prefix, `DeckStreak: <unit> failed (<result>[, status <n>])`, then the failed run's last five
  error lines, no credential and no `curl`. Its identifier is `SyslogIdentifier=%N` (line 29), so
  every alert instance's line carries an identifier starting `deck-streak-alert@`.
- **The second route.** `deck-streak-second-route.service` and its timer report a failed or silent
  alert sender to two addresses off the host (SPEC-396 R1 to R11). SPEC-582 R12 takes the unit and
  the timer out of the install and keeps the script, the model and the tests.
- **What then pages a person.** Nothing: ADR-601's consequences say a failure is a line in the
  journal and an entry in `systemctl --failed` until a monitoring delivery lands.
- **The evaluator's own trace.** `deploy/scripts/slo-evaluate.py`'s `main` prints only on a page, a
  burn still running, an episode's end or a run that cannot measure: a quiet run prints nothing.
  `deck-streak-slo.timer` fires every five minutes (`OnCalendar=*-*-* *:00/5:00 UTC`), and
  `deck-streak-backup.timer` once a day (`OnCalendar=*-*-* 03:24:00 UTC`).
- **The SLO declaration's schema.** The observability pack's `phx.slo.v1` refuses a key it does not
  know, so a pager's declaration cannot live in `deploy/slo.json`.
- **The logs' window.** `privacy.json` declares `"logs": {..., "retention": "P14D"}` (line 451),
  journald keeps 14 days (`deploy/journald.conf.d/deck-streak.conf` line 8), and `PRIVACY.md` says
  so (lines 106-107).
- **The response event.** The API's and the MCP server's trace layers open a span with `method`,
  `route` (axum's `MatchedPath`) and `request_id` (`crates/api/src/router.rs` lines 298-315;
  `crates/mcp/src/server.rs` lines 103-120), and log `finished processing request` with `latency`
  and `status`; the evaluator's journal fixture holds that event
  (`scripts/tests/fixtures/journal/api-burn.jsonl` line 1). No span field names an account, an
  address or a user agent.
- **Usage measurement today.** No source of the web app or the iPhone and iPad app sends an event
  about how it is used, the web app's dependencies name no measurement library (two optional peer
  entries in `pnpm-lock.yaml` are not installed), and the app's privacy manifest declares no
  collected data type and no tracking (`ios/App/PrivacyInfo.xcprivacy` lines 5-10;
  `scripts/tests/test_ios_app_tree.py` lines 137-142). `privacy.json`'s 40 categories each have the
  contract as their basis. The `deck-streak-analytics` context is the learner's own statistics,
  served behind the owner's session; it measures no one's use.

## 2. Requirements

R1. **The route.** Every alert of `deploy/slo.json` routes to `pager`, `alerting.channel` is
    `pager`, and `alerting.unit` is still `deck-streak-alert@.service`. Every other value of the
    file is unchanged but its `note`.
R2. **The declaration.** `docs/monitoring/pager.json` (`deckstreak.pager.v1`) names the route
    `pager`, the log store's window, the lines that ship, four pages, the channels and the request
    counters.
R3. **The alert line.** The page `an-alert-line` matches a line whose identifier starts with the
    alert template's name up to and including its `@`, at priority 3 or more urgent, opening
    `DeckStreak: `, as the template's journal sender writes it.
R4. **The sender's own failure.** The page `an-alert-instance-failed` matches the service
    manager's record that a unit whose name starts `deck-streak-alert@` failed. The alert template
    still names no `OnFailure=`.
R5. **Silence.** The page `the-evaluator-is-silent` fires when no successful run of
    `deck-streak-slo.service`, started by `deck-streak-slo.timer`, is recorded for 20 minutes: at
    least two and at most four of the timer's periods.
R6. **A late backup.** The page `the-daily-backup-is-late` fires when no successful run of
    `deck-streak-backup.service`, started by the daily `deck-streak-backup.timer`, is recorded for
    1560 minutes: more than a day, and at most a day and two hours.
R7. **What ships, and for how long.** Only the lines of units named `deck-streak-*` and the service
    manager's lines about them ship. The log store keeps a line for at most `privacy.json`'s
    `logs.retention`, set before any line ships.
R8. **A person.** A page reaches a person through at least two notification channels of at least
    two kinds, one of them email.
R9. **The request counters.** The counters count the response events of `deck-streak-api.service`
    and `deck-streak-mcp.service` with exactly four labels: the unit, the span's `route` (with
    `unmatched` when the span names none), the span's `method`, and the status class from
    `status`. No other field of the event, the request id among them, enters a counter.
R10. **The journal's copy.** `PRIVACY.md`'s bullet on the service journal says that the monitoring
    service that pages the owner keeps its copy of DeckStreak's lines for at most the journal's
    window.
R11. **Measuring use.** `PRIVACY.md` gains the section `## Measuring use`: the server counts its
    requests by route, method and status class with no account, address, device, request id or
    content; the apps measure nothing about how they are used; any measurement the apps add is off
    until the learner turns it on, by one switch.
R12. **No usage data collected.** While the switch does not exist, the app's privacy manifest
    declares no tracking and no data collected for analytics, and `PRIVACY.md` says the apps
    measure nothing.
R13. **The records.** ADR-604; the appended amendments to SPEC-031 (R2, R3), SPEC-396 and ADR-410;
    SPEC-582's A17 table row and fence line name the renamed test; this SPEC's schematic; one
    changelog fragment.
R14. **Nothing else moves.** No unit, script, credential, rail-contract row, host-budget entry or
    `OnFailure=` line changes; the second route's script, its model, its rows and its tests stay as
    SPEC-582 leaves them.

## 3. Acceptance criteria of SPEC-585

| id | criterion | decided by |
|---|---|---|
| A1 | every SLO alert and the alert channel route to `pager` through the one template, the units the declaration names are shipped, and the MCP server's SLO pages and tickets on it with its exact burn rates | `test_slo_declaration.py` `test_every_alert_routes_to_the_one_alert_path`, `test_the_mcp_slo_pages_and_tickets_on_the_one_alert_path_with_exact_burn_rates` |
| A2 | the declaration carries the route every SLO alert names, matches the alert template's line by its identifier at error priority as the sender writes it, and pages a failed alert instance on the service manager's record, the template naming no `OnFailure=` | `test_pager_declaration.py` `test_the_pager_carries_the_route_every_slo_alert_names`, `test_the_pager_matches_the_alert_templates_line_at_error_priority`, `test_a_failed_alert_instance_pages_without_its_own_line` |
| A3 | the evaluator's silence pages within two to four of its timer's periods, and a daily backup more than a day and at most two hours late pages | `test_pager_declaration.py` `test_the_evaluators_silence_pages_within_four_of_its_periods`, `test_a_daily_backup_two_hours_late_pages` |
| A4 | only DeckStreak's own lines ship, none is kept past the journal's window, and a page reaches a person by two channels of two kinds, one of them email | `test_pager_declaration.py` `test_only_deck_streak_lines_ship_and_none_outlives_the_journal`, `test_a_page_reaches_a_person_by_two_channels_of_two_kinds` |
| A5 | the request counters label only the unit, the route, the method and the status class, and every other field of the response event, the request id among them, stays out | `test_pager_declaration.py` `test_the_request_counters_carry_no_request_id_and_no_identity` |
| A6 | the privacy page bounds the monitoring service's copy of the journal by the journal's window, states what the server counts, that the apps measure nothing and that any later measurement is off until the learner turns it on by one switch, and agrees with the app's privacy manifest that nothing is collected for analytics and nothing tracks | `test_privacy_policy.py` `test_the_policy_bounds_the_monitoring_copy_of_the_journal`, `test_the_policy_states_how_use_is_measured`, `test_the_policy_and_the_privacy_manifest_agree_no_usage_is_collected` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k test_every_alert_routes_to_the_one_alert_path
A1: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k test_the_mcp_slo_pages_and_tickets_on_the_one_alert_path_with_exact_burn_rates
A2: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_the_pager_carries_the_route_every_slo_alert_names
A2: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_the_pager_matches_the_alert_templates_line_at_error_priority
A2: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_a_failed_alert_instance_pages_without_its_own_line
A3: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_the_evaluators_silence_pages_within_four_of_its_periods
A3: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_a_daily_backup_two_hours_late_pages
A4: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_only_deck_streak_lines_ship_and_none_outlives_the_journal
A4: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_a_page_reaches_a_person_by_two_channels_of_two_kinds
A5: python3 -m unittest discover -s scripts/tests -p test_pager_declaration.py -k test_the_request_counters_carry_no_request_id_and_no_identity
A6: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_bounds_the_monitoring_copy_of_the_journal
A6: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_states_how_use_is_measured
A6: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k test_the_policy_and_the_privacy_manifest_agree_no_usage_is_collected
```

The tests read files only: `deploy/slo.json`, `docs/monitoring/pager.json`, the alert template, the
journal sender's text, the two timers, `privacy.json`, `PRIVACY.md`, the app's privacy manifest
and the evaluator's journal fixture. None executes a deploy script. Each of A1 to A6 is red at the
red commit by an assertion, against the base's `deploy/slo.json` and `PRIVACY.md` and a stub
declaration that carries every key with the values the journal-only install implies (route
`journal`, no page, no channel, no label).

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-585-a-pager-off-the-host-tells-the-owner-and-usage-measurement-is-off-until-the-learner-turns-it-on.md` | docs | added |
| `docs/decisions/ADR-604-a-pager-off-the-host-replaces-the-second-route-and-usage-measurement-waits-for-one-switch.md` | docs | added |
| `docs/schematics/the-pager-the-request-counters-and-the-usage-switch.md` | docs | added |
| `docs/monitoring/pager.json` | docs | added: R2 to R9 |
| `deploy/slo.json` | deploy | changed: R1, the routes, the channel and the note |
| `PRIVACY.md` | docs | changed: R10, R11, R12 |
| `scripts/tests/test_slo_declaration.py` | tests | changed: A1 (one test renamed) |
| `scripts/tests/test_pager_declaration.py` | tests | added: A2 to A5 |
| `scripts/tests/test_privacy_policy.py` | tests | changed: A6 |
| `scripts/mutation-rows.d/S58500-S58599.json` | rows | added |
| `scripts/mutation-rows.d/S33300-S33399.json` | rows | changed: one killer renamed |
| `docs/red-first/SPEC-585.md` | docs | added |
| `docs/specs/SPEC-031-observability-foundation.md` | docs | appended |
| `docs/specs/SPEC-396-a-second-route-tells-the-owner-when-the-alert-sender-fails-or-goes-silent.md` | docs | appended |
| `docs/decisions/ADR-410-a-second-route-started-by-its-own-timer-reports-a-failed-or-absent-alert-sender-and-checks-in.md` | docs | appended |
| `docs/specs/SPEC-582-the-install-carries-no-telegram-surface.md` | docs | changed: A17's table row and fence line name the renamed test |
| `changelog.d/monitoring-m1-585.md` | docs | added |

## 5. What this does NOT cover

- The monitoring service's policies, its notification channels and their credentials, the log
  shipper on the host and its filter, the log store's 14-day window and every other setting of
  the monitoring service and the host: the host's acts, outside this repository (#42).
- The uptime checks that page when a host or an edge stops answering: the host's acts, with the
  pager's channels (#42).
- A page into DeckStreak's own app, which joins as a channel when native push lands (#640).
- A latency SLO, and SLOs for the jobs (#42); a metrics endpoint in the API (#18).
- The retirement of the second route's script, its model, its rows and its tests
  (#1103).
- The usage switch in the web app (#1100), in the iPhone and iPad app with its privacy
  manifest and the store's privacy details (#1101), and the server's collector for
  usage events with its store and its privacy category (#1102).
- What the sealed sync's daily facts carry, and which aggregates the server may read under it
  (#1084).

No change here touches the engine, the scheduler, a web screen, a unit, a deploy script or any
credential.

## 6. Risks

- **The service manager's run records do not reach the shipper.** The absence pages would then
  fire at once and keep firing; the hand-run verify (section 8) shows it before the pager is
  relied on, and ADR-604 D3's heartbeat line is the cure.
- **The monitoring service bounds an absence window below 26 hours.** The late-backup page is then
  made as a daily count below one, recorded in ADR-604 by an amendment.
- **A page is slower than the old path.** It waits on the shipper and the monitoring service's
  evaluation; the template still writes its line at once, and `systemctl --failed` still shows the
  failure on the host.
- **A tree census reads the new directory.** `docs/monitoring/` is new; a test that walks `docs/`
  meets it. The build lists every `scripts/tests` module that names `REPO / "docs"` before its
  push, and reads their CI verdicts by name.
- **A later span field.** A field added to the trace layer's span reaches the journal and the log
  store; A5 refuses it until ADR-604 D5 is amended, so the counters never take it unseen.
- **The observability pack's alert-route row.** It asks for a Telegram sender and channel, so it
  reads this install as a broken alert path until the pack learns another channel.

## 7. FORMAL, decided by surface

NOT APPLICABLE, TLA+ and Lean (ADR-604 D11). The delivery changes a data file, a declaration and
documents. The pages are matched and decided by the monitoring service, outside the tree; no code
of DeckStreak's gains a state, an interleaving or a pure function. `formal/tla/SecondRoute` keeps
its covers over `deploy/scripts/second-route.sh`, which is unchanged.

## 8. The host's acts and the hand-run verify

No repository test proves the pager. Who: the host's operator. When: after the log store's window
is set to 14 days and before the first line ships; then after the shipper, the four policies and
the channels are made, and before the pager is relied on. The verify, in order:

1. The log store reads a 14-day window before the shipper starts.
2. An alert instance started by hand for a shipped unit (`systemctl start
   deck-streak-alert@deck-streak-slo.service`) writes its line, and the page reaches both channels.
3. An alert instance made to fail on purpose, by a runtime drop-in that sets its `ExecStart=` to a
   command that exits non-zero, removed after, reaches both channels as `an-alert-instance-failed`
   and writes no line.
4. `deck-streak-slo.timer` stopped for 25 minutes reaches both channels as
   `the-evaluator-is-silent`; started again, the page clears.
5. The late-backup policy reads back a window of 26 hours.
6. A shipped response line carries no field the declaration's counters do not name or refuse.

The record: the operator's deploy record names each observation.

## 9. The usage switch's contract, for the deliveries that build it

These hold for #1100, #1101 and #1102; this SPEC builds none
of them (ADR-604 D6 to D8).

- One switch per learner, kept with the account with when it last changed, shown in the account
  settings of the web app and of the iPhone and iPad app, off by default at every age.
- When it is on, an app sends only these events, each with the app (web or iPhone and iPad), its
  version and the study day's date: a screen opened, from a fixed list of screens; a feature used,
  from a fixed list; an error shown, by its code; and once a study day, coarse learning measures
  made on the device (the review count, the minutes and the retention, each in coarse bands).
- Each event carries an id the device makes anew each study day and never derives from, links to
  or sends with the account.
- Never sent, on or off: the account, an email or sign-in identifier, a device or advertising
  identifier, a card, a note, a deck's name, a search, free text, a review one by one, a time of
  day and a location.
- Turning it off stops the sending at once and forgets the day's id.
- The learning measures are sent only through the switch, never inside the sealed sync's daily
  facts, and the server never reads those facts to measure anyone's use.
- The collector is DeckStreak's own, on its own servers, under the house's request bounds; no
  measurement library or service of another party.
- The switch's delivery adds the category to `privacy.json` with consent as its basis and the
  switch as its withdrawal, a line per category to `PRIVACY.md`, and the privacy manifest's and the
  store's declarations of product interaction, usage and diagnostic data for analytics, not linked
  and not tracking; A6 moves with them.

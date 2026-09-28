# Deploy templates

DeckStreak's systemd units and timers, its Caddy site block and its host budget (SPEC-032). Every
file here is a template: it holds neutral values that are valid as written, so the packs judge
exactly the units that will run, and the private deploy rail replaces each neutral value with the
deployment's own when it installs the templates (ADR-032, #41). Nothing in this repository installs
a unit, reloads Caddy or touches a host; the first deploy is #42's.

## The files

| file | what it is |
|---|---|
| `systemd/deck-streak-api.service` | the `api` role: the HTTP service the Mini App calls, `Type=notify` with a watchdog |
| `systemd/deck-streak-bot.service` | the `bot` role: the Telegram bot's long-polling transport, `Type=notify` with a watchdog |
| `systemd/deck-streak-job@.service` | one run of one job of coordination's job table, `deckstreakd job <id>`, a `oneshot` |
| `systemd/deck-streak-job@<id>.timer` | one timer per job of the table (`sync`, `maintenance`, `liveness`), each starting the job instance of its own name |
| `systemd/deck-streak-alert@.service` | the one alert path, a `oneshot` every other service names with `OnFailure=`: it pages the owner on Telegram that its instance failed (SPEC-031) |
| `systemd/deck-streak-slo.service`, `.timer` | the SLO evaluator, every five minutes: it pages once per burn episode of the API's SLO (SPEC-031) |
| `systemd/deck-streak-memory-watch.service`, `.timer` | the memory watch, every minute: it pages once per new OOM kill or `MemoryMax` event of any DeckStreak unit (SPEC-031) |
| `scripts/alert-telegram.sh`, `scripts/slo-evaluate.py`, `scripts/memory-watch.sh` | the three units' programs, run from the release |
| `slo.json` | the API's SLO, its error budget policy and its burn-rate alerts, which the evaluator reads (ADR-031) |
| `caddy/deck-streak.caddy` | the one site block: the Mini App, `/api/*`, the closed health routes, `robots.txt` and the security headers |
| `deck-streak.env.example` | the non-secret settings every service reads, by name, with neutral values |
| `host-budget.json` | DeckStreak's share of the host and each unit's `MemoryHigh=` and `MemoryMax=` (ADR-032) |

## What the private rail fills

Each neutral value below is valid as committed, and the rail supplies the deployment's own.

| neutral value | where | what the rail supplies |
|---|---|---|
| `/usr/local/lib/deck-streak/current/bin/deckstreakd` | every service's `ExecStart=` | the release root, whose `current` link switches by an atomic rename (ADR-010) |
| `/etc/deck-streak/deck-streak.env` | every service's `EnvironmentFile=`, required | the settings file, from `deck-streak.env.example` |
| `/usr/local/lib/deck-streak/current/deploy/` | the alert's, the evaluator's and the watch's `ExecStart=` | the release's copy of this directory's `scripts/` and `slo.json`, under the same root |
| UTC, at the default rollover hour 4 | every job timer's `OnCalendar=` | the deployment's zone and rollover hour, rendered with the two settings that name them (ADR-027); the evaluator's and the watch's timers run every few minutes in any zone |
| `{$DECKSTREAK_HOST}`, `{$DECKSTREAK_WEB_ROOT}`, `{$DECKSTREAK_API_UPSTREAM}` | the Caddy block | the Mini App's host name, the release's web build, and the API's listen address |
| the system user and group `deck-streak` | every service's `User=` and `Group=` | the user itself |
| `/run/deck-streak-credentials/socket` | every `LoadCredential=` line | the credential socket, its fetch helper and its map (ADR-038) |

`DECKSTREAK_API_UPSTREAM` is the same address as the setting `DECKSTREAK_API_LISTEN`: Caddy proxies
`/api/*` to the API's own loopback listener (ADR-007).

## Credentials

A secret reaches a unit only as `LoadCredential=<id>:/run/deck-streak-credentials/socket`, read from
the private rail's socket at every start and never stored (ADR-038). No template uses
`LoadCredentialEncrypted=`, passes a secret in an `Environment=` line or names one in the settings
file, and no template carries a secret's value.

| unit | credential ids | why |
|---|---|---|
| `deck-streak-api.service` | `owner-user-id`, `telegram-bot-token` | the owner gate over Telegram's launch data (SPEC-024) |
| `deck-streak-bot.service` | `owner-user-id`, `telegram-bot-token`, `anki-sync-username`, `anki-sync-password` | the transport and the owner gate, and the owner's `/sync`, which runs a sync cycle in this role (SPEC-026) |
| `deck-streak-job@.service` | `anki-sync-username`, `anki-sync-password` | the `sync` job's account (SPEC-022); only that job reads it |
| `deck-streak-alert@.service` | `owner-user-id`, `telegram-bot-token` | the page: the bot's token, and the owner's id, which is the owner's private chat (SPEC-031) |

systemd names the unit in the address it binds for each credential, so a job's credentials reach
the socket under the job instance's name; the rail's map decides which instances it answers (#41).

## The schedule

Coordination's job table (`crates/coordination/src/jobs.rs`) is the one schedule, and every timer is
written from it; `crates/coordination/tests/job_table.rs` holds each timer's calendar equal to its
job's slot (ADR-027).

| timer's job | slot | calendar here | `Persistent=` |
|---|---|---|---|
| `sync` | daily, the rollover hour, minute 7 | `*-*-* 04:07:00 UTC` | `true`: the table's one catch-up job |
| `maintenance` | daily, the rollover hour, minute 28 | `*-*-* 04:28:00 UTC` | none, waived with its why |
| `liveness` | hourly, minute 14 | `*-*-* *:14:00 UTC` | none, waived with its why |

No job timer carries a random delay: the table already places each job on its own minute, clear of
the others, and a delay would move a fire off it. Each timer says so in its `X-DurableServices-Waive=`.

The SLO evaluator's and the memory watch's timers are not jobs of the table. The evaluator runs every
five minutes and the watch every minute, each with a short random delay, since neither keeps a minute
of its own; neither catches up a run missed while the host was down, and each timer says why.

## Runbook

- Start one job by hand with `systemctl start deck-streak-job@<id>.service`, the id being one of the
  table's. The run exits 0 when the job ran, skipped or recorded a missed fire, 1 when it pages
  (the unit fails, and `OnFailure=` sends the one alert), and 2 for an id the table does not hold.
- Run `maintenance` by hand only at its slot, or expect one drift page: the next `liveness` check
  reads a maintenance fire off its slot as drift, once (ADR-027).
- A unit that fails pages through `deck-streak-alert@.service` (SPEC-031), which names the failed
  unit and its result and quotes that run's last five error lines. A job that hangs is ended after
  30 minutes and pages the same way. A daemon in a crash loop pages at each failure until its start
  limit ends the loop.
- A page that cannot be sent leaves its alert instance failed, listed by `systemctl --failed`; no
  page is sent about a failed page.
- The SLO evaluator pages when both windows of an alert of `slo.json` burn, once per episode, and the
  memory watch once per new OOM kill or `MemoryMax` event; each keeps what it has paged in its own
  state directory, and each pages once, too, when it cannot measure.
- Each service logs under its own identifier: `deck-streak-api`, `deck-streak-bot`,
  `deck-streak-slo`, `deck-streak-memory-watch`, and the instance's full name for a job or a page.

## The Caddy block

One site block serves the Mini App's static build, with `try_files {path} {path}.html /index.html`
answering every route the client renders, and proxies `/api/*` to the API. It answers `404` for
`/api/livez` and `/api/readyz`, which only the host itself reads (ADR-025), and it answers
`robots.txt` itself, disallowing everything before the build's own file is reached. Every response
carries HSTS, `nosniff`, the page's `same-origin` referrer policy, `X-Robots-Tag: noindex`, and a
Content-Security-Policy of `frame-ancestors https://web.telegram.org; object-src 'none'; base-uri
'self'`, and Caddy's `Server` header is removed. The script sources are the page's own meta policy,
with its build's hashes (SPEC-028), so the header sets none.

## The host budget

`host-budget.json` records DeckStreak's share, `"memory": "640M"` and `"cpus": 2`, and each unit's
ceilings, which the unit's `MemoryHigh=` and `MemoryMax=` equal (ADR-032). The long-running units'
ceilings plus the largest oneshot's fit the share, and the API's and the bot's `CPUQuota=` fit its
CPUs. The alert template, the SLO evaluator and the memory watch carry their own entries (SPEC-031);
since they run beside the jobs, every ceiling reached at once passes the share (SPEC-031 §6).

## Writing about an instance

Never write a template instance's whole name, a template, `@`, an instance, a dot and a unit type,
in a committed file: the public scrub reads that shape as an email address (SPEC-032 R10). Write the
instance as `<id>`, or build the name at run time, as the tests do.

## How the templates are judged

- The durable-services pack's rows, and the rust-service rows that read the units beside
  `deckstreakd`'s code, both judged on the maintainer's box (ADR-069).
- `scripts/tests/test_deploy_templates.py` and the job table's timer test (SPEC-032's acceptance
  criteria).
- The observability rows, judged on the maintainer's box (ADR-069), and SPEC-031's
  `test_slo_declaration.py`, `test_alert_unit.py`, `test_slo_evaluator.py` and
  `test_memory_watch.py`, which run the three scripts over synthetic credentials, journal and cgroups.
- The public scrub over every file here.
- The web-security rows over the Caddy block, on the maintainer's box (`scripts/box-packs.sh`).

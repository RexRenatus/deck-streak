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
| `caddy/deck-streak.caddy` | the one site block: the Mini App, `/api/*`, the closed health routes, `robots.txt` and the security headers |
| `deck-streak.env.example` | the non-secret settings every service reads, by name, with neutral values |
| `host-budget.json` | DeckStreak's share of the host and each unit's `MemoryHigh=` and `MemoryMax=` (ADR-032) |

## What the private rail fills

Each neutral value below is valid as committed, and the rail supplies the deployment's own.

| neutral value | where | what the rail supplies |
|---|---|---|
| `/usr/local/lib/deck-streak/current/bin/deckstreakd` | every service's `ExecStart=` | the release root, whose `current` link switches by an atomic rename (ADR-010) |
| `/etc/deck-streak/deck-streak.env` | every service's `EnvironmentFile=`, required | the settings file, from `deck-streak.env.example` |
| UTC, at the default rollover hour 4 | every timer's `OnCalendar=` | the deployment's zone and rollover hour, rendered with the two settings that name them (ADR-027) |
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

No timer carries a random delay: the table already places each job on its own minute, clear of the
others, and a delay would move a fire off it. Each timer says so in its `X-DurableServices-Waive=`.

## Runbook

- Start one job by hand with `systemctl start deck-streak-job@<id>.service`, the id being one of the
  table's. The run exits 0 when the job ran, skipped or recorded a missed fire, 1 when it pages
  (the unit fails, and `OnFailure=` sends the one alert), and 2 for an id the table does not hold.
- Run `maintenance` by hand only at its slot, or expect one drift page: the next `liveness` check
  reads a maintenance fire off its slot as drift, once (ADR-027).
- A unit that fails pages through `deck-streak-alert@.service` (SPEC-031), which names the failed
  unit and its result. A job that hangs is ended after 30 minutes and pages the same way.
- Each service logs under its own identifier: `deck-streak-api`, `deck-streak-bot`, and the job
  instance's full name for a job.

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
CPUs. The alert unit, the SLO evaluator and the memory watch add their entries with SPEC-031.

## Writing about an instance

Never write a template instance's whole name, a template, `@`, an instance, a dot and a unit type,
in a committed file: the public scrub reads that shape as an email address (SPEC-032 R10). Write the
instance as `<id>`, or build the name at run time, as the tests do.

## How the templates are judged

- The durable-services pack's rows, and the rust-service rows that read the units beside
  `deckstreakd`'s code, both judged on the maintainer's box (ADR-069).
- `scripts/tests/test_deploy_templates.py` and the job table's timer test (SPEC-032's acceptance
  criteria).
- The public scrub over every file here.
- The web-security rows over the Caddy block, on the maintainer's box (`scripts/box-packs.sh`).

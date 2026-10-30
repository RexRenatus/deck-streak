# Deploy templates

DeckStreak's systemd units and timers, its Caddy site block and its host budget (SPEC-032). Every
file here is a template: it holds neutral values that are valid as written, so the packs judge
exactly the units that will run, and the private deploy rail replaces each neutral value with the
deployment's own when it installs the templates (ADR-032, #41). Nothing in this repository installs
a unit, reloads Caddy or touches a host on its own: `deploy.sh` and `rollback.sh` do, from the
maintainer's machine, only when run there (SPEC-062, below).

## The files

| file | what it is |
|---|---|
| `systemd/deck-streak-api.service` | the `api` role: the HTTP service the Mini App calls, `Type=notify` with a watchdog |
| `systemd/deck-streak-bot.service` | the `bot` role: the Telegram bot's long-polling transport, `Type=notify` with a watchdog |
| `systemd/deck-streak-mcp.service` | the `mcp` role: the MCP server the owner's agent calls on a loopback address, `Type=notify` with a watchdog; installed by a deploy and first started by the owner (SPEC-119, ADR-332) |
| `systemd/deck-streak-sync-server.service`, `scripts/sync-server.sh` | the engine's own sync server for the owner's Anki clients, which the release ships as `bin/anki-sync-server`, `Type=exec` on a loopback address; its launcher reads the two sync users from the unit's credentials, refuses an entry that is not a user name and a pbkdf2-sha256 hash, and execs the server (SPEC-337, ADR-347) |
| `systemd/deck-streak-job@.service` | one run of one job of coordination's job table, `deckstreakd job <id>`, a `oneshot` |
| `systemd/deck-streak-job@<id>.timer` | one timer per job of the table (`sync`, `maintenance`, `liveness`, `drill_postback`, `held_flush`), each starting the job instance of its own name |
| `systemd/deck-streak-job@sync (path unit)` | the owner's `/sync` doorbell: a change of the request file starts `deck-streak-job@sync` (service unit), and it loads no credential (SPEC-059) |
| `tmpfiles.d/deck-streak-sync-request.conf` | the request directory, the service user's alone, mode `0700`; only the bot unit may write it (SPEC-059) |
| `systemd/deck-streak-alert@.service` | the one alert path, a `oneshot` every other service names with `OnFailure=`: it pages the owner on Telegram that its instance failed (SPEC-031) |
| `systemd/deck-streak-slo.service`, `.timer` | the SLO evaluator, every five minutes: it pages once per burn episode of the API's SLO (SPEC-031) |
| `systemd/deck-streak-memory-watch.service`, `.timer` | the memory watch, every minute: it pages once per new OOM kill or `MemoryMax` event of any DeckStreak unit (SPEC-031) |
| `systemd/deck-streak-second-route.service`, `.timer`, `scripts/second-route.sh` | the second route, every ten minutes: it reads the alert path from the service manager, reports a failed alert instance or an absent alert template to a receiver off the host, and checks in; it shares no credential and no process with the alert sender (SPEC-396, ADR-410) |
| `scripts/alert-telegram.sh`, `scripts/slo-evaluate.py`, `scripts/memory-watch.sh` | the three units' programs, run from the release |
| `slo.json` | the API's SLO, its error budget policy and its burn-rate alerts, which the evaluator reads (ADR-031) |
| `caddy/deck-streak.caddy` | the one site block: the Mini App, `/api/*`, the closed health routes, `robots.txt`, the security headers, and one access log of the sync route alone, with no request header and no `k` parameter (SPEC-340 R6) |
| `fail2ban/filter.d/deck-streak-sync.conf`, `fail2ban/jail.d/deck-streak-sync.conf` | the sync route's login bound: a filter over the edge's access log of the sync route, and a jail that bans an address after five refused sync logins within ten minutes, for one hour (SPEC-340 R5; ADR-351 D3) |
| `deck-streak.env.example` | the non-secret settings every service reads, by name, with neutral values |
| `host-budget.json` | DeckStreak's share of the host and each unit's `MemoryHigh=` and `MemoryMax=` (ADR-032) |
| `rail-contract.json` | the neutral values the private rail overrides, by unit and key, with the name of its drop-in and the credential socket's path (SPEC-061, ADR-061) |
| `scripts/credential-pairs.py`, `scripts/effective-check.py`, `scripts/guards-check.py` | the rail contract's three checks: the credential pairs the templates declare, a unit's effective configuration, and the pinned guards against their manifest (SPEC-061) |

## What the private rail fills

Each neutral value below is valid as committed, and the rail supplies the deployment's own. The
rail installs every template byte for byte, and a unit's own values reach it in one drop-in beside
it, `<unit>.d/10-rail.conf`, which resets and then sets each value `rail-contract.json` lists for
that unit (ADR-061).

| neutral value | where | what the rail supplies |
|---|---|---|
| `/usr/local/lib/deck-streak/current/bin/deckstreakd` | every service's `ExecStart=` | the release root, whose `current` link switches by an atomic rename (ADR-010) |
| `/etc/deck-streak/deck-streak.env` | every service's `EnvironmentFile=`, required | the settings file, from `deck-streak.env.example` |
| `/usr/local/lib/deck-streak/current/deploy/` | the alert's, the evaluator's, the watch's and the second route's `ExecStart=` | the release's copy of this directory's `scripts/` and `slo.json`, under the same root |
| UTC, at the default rollover hour 4 | every timer's `OnCalendar=` | the deployment's zone, and each job timer's rollover hour, rendered with the two settings that name them (ADR-027); the evaluator's and the watch's timers fire every few minutes in any zone, and take the deployment's zone all the same, so no calendar is left in UTC |
| `{$DECKSTREAK_HOST}`, `{$DECKSTREAK_WEB_ROOT}`, `{$DECKSTREAK_API_UPSTREAM}`, `{$DECKSTREAK_SYNC_UPSTREAM}` | the Caddy block | the Mini App's host name, the release's web build, the API's listen address and the sync server's |
| the system user and group `deck-streak` | every service's `User=` and `Group=` but the sync family's | the user itself |
| the system user and group `deck-streak-sync` | the sync server's, its window's, its archive's and its drill's `User=` and `Group=` | the user itself, with no login shell, no home and no other group (SPEC-340 R2; ADR-351 D1) |
| `_SYSTEMD_UNIT=caddy.service` | the ban jail's `journalmatch=`, `fail2ban/jail.d/deck-streak-sync.conf` | the edge's unit on the host; the rail installs the jail and its filter into the host's ban service, the filter checked first against one refused login, on the owner's go (SPEC-340 R5; ADR-351 D3) |
| `/usr/bin/age --recipients-file /etc/deck-streak/snapshot-recipients.txt` | the setting `DECKSTREAK_SNAPSHOT_SEAL`, `deck-streak.env.example`, which the archive unit runs | the seal command and the owner's public recipients file, which holds the offline key's public half alone; the rail installs both and checks the seal under the archive unit's sandbox, on the owner's go (SPEC-340 R12; ADR-351 D2) |
| `/run/deck-streak-credentials/socket` | every `LoadCredential=` line | the credential socket, its fetch helper and its map (ADR-038) |

`DECKSTREAK_API_UPSTREAM` is the same address as the setting `DECKSTREAK_API_LISTEN`: Caddy proxies
`/api/*` to the API's own loopback listener (ADR-007). `DECKSTREAK_SYNC_UPSTREAM` is the same
address as `DECKSTREAK_SYNC_SERVER_LISTEN`: Caddy proxies `/anki-sync/` to the sync server's own
loopback listener, with the prefix stripped, its health route closed and the request body bounded
at the server's own payload limit (SPEC-337 R4; ADR-347 D4).

## Credentials

A secret reaches a unit only as `LoadCredential=<id>:/run/deck-streak-credentials/socket`, read from
the private rail's socket at every start and never stored (ADR-038). No template uses
`LoadCredentialEncrypted=`, passes a secret in an `Environment=` line or names one in the settings
file, and no template carries a secret's value.

| unit | credential ids | why |
|---|---|---|
| `deck-streak-api.service` | `owner-user-id`, `telegram-bot-token` | the owner gate over Telegram's launch data (SPEC-024) |
| `deck-streak-bot.service` | `owner-user-id`, `telegram-bot-token` | the transport and the owner gate (SPEC-026); the owner's `/sync` holds no login, it asks the sync job (SPEC-059) |
| `deck-streak-mcp.service` | `mcp-core-token`, `mcp-law-track-token` | the MCP server's bearer guard: the core token is required, and the law-track token grants the law track (SPEC-119 R6) |
| `deck-streak-job@.service` | none | the sync login is loaded by the sync job alone: its instance's drop-in in `systemd/` carries `anki-sync-username` and `anki-sync-password` (SPEC-022, SPEC-062 R14), and the rail's map answers them to that instance alone |
| `deck-streak-job@.service`, `held_flush` instance | `owner-user-id`, `telegram-bot-token` | the held flush alone sends to the owner's chat (#291): its instance's drop-in in `systemd/` carries the two, and no other job requests them |
| `deck-streak-alert@.service` | `owner-user-id`, `telegram-bot-token` | the page: the bot's token, and the owner's id, which is the owner's private chat (SPEC-031) |
| `deck-streak-second-route.service` | `second-route-check-in`, `second-route-report` | the second route's two addresses, https, each the receiver's, off the host: the check-in address takes a request with no body, the report address a plain-text body; neither is the alert sender's, and the rail's map names both for this unit alone (SPEC-396) |
| `deck-streak-sync-server.service` | `sync-server-owner`, `sync-server-staging` | the sync server's two users, the owner's and the staging user (ADR-344), each a user name and a pbkdf2-sha256 hash, never a password; its launcher, `scripts/sync-server.sh`, refuses any other shape and hands them to the server (SPEC-337 R2, ADR-347) |

systemd names the unit in the address it binds for each credential, so a job's credentials reach
the socket under the job instance's name. The rail's map names the template, and an instance
matches its template's row; a row is never a pattern over unit names (SPEC-061 R4). The sync
login's rows name the `sync` instance instead, the one job that reads it, and the sync login is
loaded by the sync job alone: the job template requests no credential, and the `sync` instance's
drop-in under `systemd/` carries the two `LoadCredential=` lines (SPEC-061 §8, SPEC-062 R14).

## The rail's contract

The private deploy rail lives outside this repository and is never published (SPEC-061). It holds
the credential socket's fetch helper, its socket unit and its map of (unit, credential id) pairs
(ADR-038); the settings file it renders from private configuration, installed root-owned with mode
`0600`; the drop-ins; and, only with the AI route, the guards' pinned copies and their manifest
(SPEC-063). No secret's value passes through the rail: a value travels from the secret manager,
through the helper, into systemd's credentials directory for the unit that starts. This repository
holds what the rail reads and the checks it runs:

| check | what it reads | what it refuses |
|---|---|---|
| `scripts/credential-pairs.py --root <release> [--optional <set>]` | every unit template with its drop-ins, and each optional set it names | `LoadCredentialEncrypted=`, `SetCredential=`, `SetCredentialEncrypted=`, `ImportCredential=`, a `LoadCredential=` whose source is not the socket, and a line systemd would read otherwise |
| `scripts/effective-check.py --root <release> [<output> ...]` | `systemctl cat` of installed units, from files or standard input | a neutral value left in force, a credential not from the socket, an `Environment=` variable whose name says it holds a secret, any other route a value takes into the unit (a secret-named `PassEnvironment=`, standard input written in the file or read from one, a second `EnvironmentFile=`), a line systemd would read otherwise, and a drop-in other than the rail's own |
| `scripts/effective-check.py --root <release> --census` | `rail-contract.json` and the templates | a neutral value the contract does not name, and a row no template carries |
| `scripts/guards-check.py <manifest>` | the guards' manifest and each file it names | a missing or changed file, a file anyone but root owns or could write, a mode other than the manifest's, and a manifest that names no file |

`credential-pairs.py` prints its pairs as JSON, a template unit named as the template it is, with
the files and lines it examined; the rail refuses to install when its map's pairs differ from that
list in either direction. The contract names a job's timer by its template and its instance, and
the checks join them. Each check reads a unit file line by line as systemd reads it, and refuses a
construct systemd would read otherwise instead of guessing (SPEC-061 §8). Each check prints how much
it examined and never a secret's value. `credential-pairs.py` and `effective-check.py` exit 0 when
everything passes, 1 on a refusal, and 2 when they judged nothing; `guards-check.py` exits 0 or 1,
since a manifest that is absent, unreadable or names no file is itself refused, and the agent's
launch never starts on it (SPEC-061 R8).

A credential that arrives empty, with no bytes or only a newline, refuses start by its id as a
missing one does (SPEC-066, ADR-067). A role refuses it through the kernel's loader, and the page
quotes the line that names it; the `sync` job records it as `missing_credentials`. The alert unit
refuses one in its script before any request, and stays failed in `systemctl --failed`, since
nothing pages about the alert unit itself (#285).

## The schedule

Coordination's job table (`crates/coordination/src/jobs.rs`) is the one schedule, and every timer is
written from it; `crates/coordination/tests/job_table.rs` holds each timer's calendar equal to its
job's slot (ADR-027).

| timer's job | slot | calendar here | `Persistent=` |
|---|---|---|---|
| `sync` | daily, the rollover hour, minute 7 | `*-*-* 04:07:00 UTC` | `true`: the table's one catch-up job |
| `maintenance` | daily, the rollover hour, minute 28 | `*-*-* 04:28:00 UTC` | none, waived with its why |
| `liveness` | hourly, minute 14 | `*-*-* *:14:00 UTC` | none, waived with its why |
| `drill_postback` | hourly, minute 19 | `*-*-* *:19:00 UTC` | none, waived with its why |
| `held_flush` | daily, 07:36 local, outside the quiet window | `*-*-* 07:36:00 UTC` | `true`: a missed flush runs once, still outside the window |

The owner's `/sync` adds no slot and no timer: the bot stores the request and touches the request
file, `deck-streak-job@sync` (path unit) starts the sync job, and the job serves the stored request before
its scheduled run, which stays claimed once per study day (SPEC-059, ADR-037).

No job timer carries a random delay: the table already places each job on its own minute, clear of
the others, and a delay would move a fire off it. Each timer says so in its `X-DurableServices-Waive=`.

The second route's timer is not a job of the table either: it runs every ten minutes with a short random delay, reads the service manager's current state and catches up nothing, and its timer says why.

The SLO evaluator's and the memory watch's timers are not jobs of the table. The evaluator runs every
five minutes and the watch every minute, each with a short random delay, since neither keeps a minute
of its own; neither catches up a run missed while the host was down, and each timer says why.

## Runbook

- First deploy, for the owner's `/sync`: create the request directory before the bot restarts, with
  `systemd-tmpfiles --create` over `tmpfiles.d/deck-streak-sync-request.conf`, because the bot's
  `ReadWritePaths=` names the directory without a `-` prefix and the bot does not start while it is
  absent. Then restart the bot, and enable the path unit of the sync instance (`systemctl enable --now`
  on the `deck-streak-job` path template, instance `sync`).
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

## Deploying a release

`deploy/deploy.sh vX.Y.Z` and `deploy/rollback.sh vX.Y.Z` run on the maintainer's machine and reach
the host only through the host command they are given (SPEC-062; ADR-062). They are configured by
environment variables: `DECKSTREAK_DEPLOY_REPO` (the repository whose release is installed) and
`DECKSTREAK_DEPLOY_HOST` (a command that runs its argv on the host as given: the private rail's host command) are the
two a maintainer sets; `DECKSTREAK_DEPLOY_ELEVATE` (default `sudo`), `_CHECKOUT`, `_ROOT`,
`_UNIT_DIR`, `_ENV_FILE`, `_CADDY_DIR`, `_CADDYFILE`, `_READY_SECONDS`, `_READY_POLL` and `_KEEP`
default to the host layout the templates assume, and a test points them at a temporary tree.

Before the host is touched, the tag must be SemVer, annotated, and its commit an ancestor of
`origin/main` after a fetch; the tarball's build attestation must verify against this repository's
release workflow; and the tarball's digest must match a line of `SHA256SUMS` that names it. Any
failure stops the deploy with the host untouched. On the host the tarball is unpacked as
`releases/<tag>.partial`, checked against its `MANIFEST.sha256`, renamed to `releases/<tag>`, and
`current` is replaced by one `mv -T` of a link built beside it. The units are installed byte for
byte (a drop-in the release no longer ships is removed; the rail's own `10-rail.conf` is left),
systemd is reloaded, `effective-check.py` judges every unit, and the API then the bot restart. The
API must answer `/api/readyz` on its loopback listener within the bound. When a unit does not
become ready, `current` goes back to the release it replaced, that release's units are reinstalled,
the units restart and the deploy exits non-zero naming the unit. The host keeps the current release
and the two before it, and prunes only after a ready switch.

`rollback.sh vX.Y.W` makes a kept release current again with no download, or deploys a release the
host no longer keeps through the same verification. `deploy.sh caddy-install vX.Y.Z` renders the
block with `scripts/render-caddy.py` from the private configuration (`DECKSTREAK_DEPLOY_CADDY_CONFIG`,
a JSON object with `host`, `web_root`, `api_upstream` and `sync_upstream`), adds it and one `import` line to a copy
of the Caddyfile, runs `caddy validate` and `caddy adapt --validate` on the copy, moves it into
place and reloads; a refusal leaves the live file as it was, and a reload that fails puts the
previous block and Caddyfile back, reloads them and exits non-zero (SPEC-127).
The `import` line names the block by its absolute path in the Caddy directory, and each step
writes and checks its candidate in the live Caddyfile's own directory, so the site is served
whether the Caddyfile is in the Caddy directory or set apart by `DECKSTREAK_DEPLOY_CADDYFILE`;
either step refuses a Caddy directory that is not an absolute path of ASCII letters, digits and
`._@+/-` before it reads or writes anything (SPEC-353).
`rollback.sh caddy-remove` reverses it under the same rule. Either Caddy step refuses, before it
reads or writes anything, any entry of its environment whose name starts with `DECKSTREAK_DEPLOY_` and
is not one of the settings above, whatever follows the prefix, and a setting it receives twice or
without a value, and names it. It reads its environment from `/proc/self/environ`, so it runs only
where that file is readable (Linux). It leaves every name outside the prefix alone, and one that the
shell reads as code when it starts can run before the refusal and stop it: such an entry can already
run any code in the step, more than an unlisted setting can do, and the refusal guards against a
misconfigured setting, not against code already placed in the step's environment (ADR-198).

## The MCP server's first start

`deck-streak-mcp.service` runs the `mcp` role (SPEC-119). A deploy installs it with the other units
and never starts it: `deploy.sh` restarts the API and the bot alone, and the role refuses start
without its core token. Its first start is the owner's, in two steps (ADR-332):

1. Store the core token in the private rail under its credential id, `mcp-core-token`, and, to
   grant the law track, its token under `mcp-law-track-token` (SPEC-119 R6, R7).
2. Enable and start the unit on the host (`systemctl enable --now deck-streak-mcp.service`), then
   read its journal: a token the role refuses is named by its id, never by its value.

## The host budget

`host-budget.json` records DeckStreak's share, `"memory": "1152M"` and `"cpus": 2`, and each unit's
ceilings, which the unit's `MemoryHigh=` and `MemoryMax=` equal (ADR-032). The five long-running
units' ceilings plus the largest oneshot's fill the share exactly, and their `CPUQuota=` values,
75% each for the API and the sync server, 20% for the bot and 15% each for the replicator and the
MCP server, divide its CPUs exactly (ADR-064 and ADR-032 as SPEC-337 amends them, ADR-347). The
alert template, the SLO evaluator and the memory watch carry their own entries (SPEC-031); since
they run beside the jobs, every ceiling reached at once passes the share (SPEC-031 §6).

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
- `scripts/tests/test_rail_contract.py` (SPEC-061's acceptance criteria), and
  `effective-check.py --census`, which holds `rail-contract.json` equal to the neutral values the
  templates carry. At each install the rail compares its map with `credential-pairs.py` and runs
  `effective-check.py` over every installed unit.
- The public scrub over every file here.
- The web-security rows over the Caddy block, on the maintainer's box (`scripts/box-packs.sh`).

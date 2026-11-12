---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Deploy templates carry neutral, lint-valid values the private rail replaces, and DeckStreak's share of the host is 640 MiB with a ceiling per unit

## Context and Problem Statement

ADR-010 decides hardened systemd units per role, parameterised "by environment variables with
placeholder values", with `MemoryHigh` and `MemoryMax` from `deploy/host-budget.json`; ADR-007 adds
a Caddy block "parameterised by environment variables". The repository is public (CHARTER 11), so no
concrete host, path or secret may appear; but the durable-services and web-security packs judge the
committed files as written, and a placeholder such as `@ROOT@` is not a valid absolute path to
systemd. And no ADR gives the budget a number. How are the templates parameterised, and what is the
budget?

## Decision Drivers

- The packs read the templates as committed; a placeholder must still be a valid value.
- CHARTER 11: nothing private enters the tree; the private deploy rail holds the concrete values.
- DeckStreak's share of the host is a stated budget (CHARTER 3), and it must hold through the
  side-by-side cutover (ADR-011).
- Each unit fails on its own ceiling rather than the kernel's global OOM killer choosing
  (durable-services `resources.budget`).

## Considered Options (the alternatives it was chosen against)

- Templates holding neutral values that are valid as written (an example release root under `/usr/local`, UTC calendars, credential ids with no path, Caddy's own `{$VAR}` placeholders), which the private rail replaces at deploy — chosen: every pack judges a real unit, and nothing private is committed.
- Committed concrete values — rejected because the host's paths, names and zone are private (CHARTER 11).
- A template engine that renders the units at deploy (Jinja, or `envsubst` over non-valid placeholders) — rejected because the packs could only judge the unrendered text, which is not a unit systemd would load, so the gate would judge nothing real.
- One slice whose `MemoryMax` caps the whole stack — rejected as the only control because a runaway job would then throttle the API and the bot inside the same slice; a ceiling per unit keeps each failure its own, and the per-unit sum is what the budget row checks.

## Decision Outcome

Chosen option. The budget, which `deploy/host-budget.json` records and every unit's `MemoryHigh=` and
`MemoryMax=` must equal:

| unit | memory_high | memory_max | why |
|---|---|---|---|
| `deck-streak-api.service` | 96M | 128M | an axum service bounded to 64 requests and 2 MB bodies |
| `deck-streak-bot.service` | 64M | 96M | one poll loop and one transport |
| `deck-streak-job@.service` | 320M | 384M | the sync's budget is 256 MiB of resident memory (ADR-022), a fifth below `MemoryHigh` |
| `deck-streak-alert@.service` (SPEC-031) | 32M | 48M | a shell script and one HTTPS request |
| `deck-streak-slo.service` (SPEC-031) | 48M | 64M | a standard-library Python evaluator over the journal |
| `deck-streak-memory-watch.service` (SPEC-031) | 32M | 48M | a shell script over the units' memory accounting |

The share is `"memory": "640M"`: the long-running units' ceilings (224 MiB) plus the largest oneshot's
(384 MiB) are 608 MiB, which fits with room to spare. Why 640 MiB: the share must hold through the
side-by-side cutover (ADR-011) and still leave the kernel headroom when every ceiling is reached at
once; the memory watch measures the real figures from the first deploy, and a change is a new ADR.

Caddy substitutes its native `{$DECKSTREAK_HOST}`, `{$DECKSTREAK_WEB_ROOT}` and
`{$DECKSTREAK_API_UPSTREAM}` from the environment when the block is adapted, and the private rail
supplies their values when it installs the block (#41).

### Decided at delivery (SPEC-032 §7)

The delivery decided what this record left open, each against its alternatives:

- **The neutral paths.** Every service runs `/usr/local/lib/deck-streak/current/bin/deckstreakd
  <role>` and reads one required settings file, `/etc/deck-streak/deck-streak.env`: an example
  release root under `/usr/local`, as chosen above, and the conventional place for a service's
  configuration. Chosen against a path that names the deployment, which is private (CHARTER 11), and
  against a placeholder such as `@ROOT@`, which systemd would not load.
- **The daemons' processor and task caps.** The API runs with `CPUQuota=100%` and the bot with
  `CPUQuota=50%`, together within the share's 2 CPUs, and each with `TasksMax=64`, a bound on a
  thread or process leak well above what either role starts with its default settings. Chosen
  against leaving them unset, which lets a runaway daemon take every processor and a leak grow
  without bound, and against one slice capping both, rejected
  above for memory for the same reason. A job yields instead of being capped: `Nice=10` and the idle
  IO class (SPEC-032 R1).
- **The job template's start timeout.** `TimeoutStartSec=30min`. systemd gives a oneshot no start
  timeout by default, so a hung run would hold its unit active and its timer could never start it
  again; the hourly watch would fall silent. Thirty minutes is thirty times ADR-022's incremental
  sync budget and shorter than the watch's hour, so a hung run is ended, and pages, before the watch
  is due again. Chosen against no timeout, and against a timeout per job, which would need a
  drop-in per instance whose name SPEC-032 R10 keeps out of committed files.
- **The timers carry no random delay.** Each waives the pack's `timers.spread` with its why: the job
  table places each job on its own minute (ADR-027), and a random delay would move a fire off it. Chosen against `RandomizedDelaySec=`, which spreads
  timers that share a minute, and none here do.
- **Each unit's credentials.** The API loads `owner-user-id` and `telegram-bot-token`; the bot loads
  those and the sync's `anki-sync-username` and `anki-sync-password`, because the owner's `/sync`
  runs a sync cycle in its role (SPEC-026 R11); the job template loads the sync's pair, which only
  the `sync` job reads, and the rail's map decides which instances it answers (ADR-038, #41). Chosen
  against a drop-in carrying the pair for the `sync` instance alone, whose committed name would be a
  template instance's (SPEC-032 R10), and against a job template that loads none, which would leave
  the sync without its account.
- **The journal's size cap stays the host's.** The pack's advisory `logging.journal-cap` reads a
  `journald.conf.d` drop-in, which is host-wide configuration rather than a unit's, so no template
  sets it, and the advisory stays open. Chosen against shipping a journald drop-in, whose cap would
  apply to every unit's journal, not DeckStreak's alone.

### Consequences

- Good, because the gate judges exactly the units that will run, and the public tree stays clean.
- Good, because a job's spike throttles and fails the job alone.
- Bad, because the rail must replace every neutral value; a missed one shows as a unit that cannot
  find its binary or a timer that fires in UTC, which the first start and the liveness job's drift
  check reveal.

### Confirmation

SPEC-032's acceptance tests; durable-services `resources.*` rows, enforced; the memory watch's first
week on the host (SPEC-031, W2).

## What would make this wrong

- The memory watch shows a unit living at its `MemoryHigh` in normal use (its ceiling is too low), or
  the host's available memory falls below the share when every ceiling is reached (a resize is an
  owner decision, ADR-011).
- A pack gains a way to judge rendered templates, which would allow real placeholders.

## More Information

ADR-007; ADR-010; ADR-011; ADR-022; ADR-025; SPEC-032; `docs/schematics/deployment.md`; the
durable-services and observability packs.

Amended by ADR-061 (proposed): the rail fills the Caddy block's placeholders by rendering the block at
install and importing the rendered file with one line, so Caddy's environment carries none of their
values; the committed template keeps its placeholders, valid as written.

## Note, 2026-09-29: the share is 704 MiB

ADR-064 raises DeckStreak's share to 704 MiB to hold the replicator's ceiling of 64 MiB; its budget table
decides the replicator's, the daily backup's and the restore drill's ceilings.

## Note, 2026-10-04: the MCP server's unit (ADR-332)

ADR-332 adds `deck-streak-mcp.service`, SPEC-119's MCP server, as a fourth long-running unit inside
the share ADR-064 decides. Its ceilings, in this record's form:

| unit | memory_high | memory_max | why |
|---|---|---|---|
| `deck-streak-mcp.service` (SPEC-119) | 24M | 32M | an axum service bounded to 8 requests and 64 KiB bodies |

The daemons' processor caps move with it: the MCP server's quota is taken from the API's, whose cap
stays a ceiling and not a reservation. The bot's and the replicator's caps are unchanged.

| unit | CPUQuota | TasksMax |
|---|---|---|
| `deck-streak-api.service` | 75% | 64 |
| `deck-streak-mcp.service` | 25% | 32 |

The rest of this record stands.

## Amendment (SPEC-337): the share is 1152 MiB

ADR-064's amendment of this delivery moves DeckStreak's share to 1152 MiB to hold ADR-347's sync
server, `deck-streak-sync-server.service`, a fifth long-running unit. Its ceilings, in this record's
form:

| unit | memory_high | memory_max | why |
|---|---|---|---|
| `deck-streak-sync-server.service` (SPEC-337) | 384M | 448M | one full upload of ADR-022's synthetic collection peaks at about 319 MiB resident, under `MemoryHigh` |
| `deck-streak-sync-snapshot.service` (SPEC-337) | 48M | 64M | the stopped-server window's copy streams the online backup page by page, as the daily backup does (ADR-347 D12) |

The daemons' processor caps divide the share's two processors exactly, as ADR-347 D7 splits them:
the sync server's three quarters of a processor are taken from the bot's, the replicator's and the
MCP server's, and the API keeps its cap, since it serves the clients' path.

| unit | CPUQuota | TasksMax |
|---|---|---|
| `deck-streak-api.service` | 75% | 64 |
| `deck-streak-sync-server.service` | 75% | 64 |
| `deck-streak-bot.service` | 20% | 64 |
| `deck-streak-litestream.service` | 15% | 64 |
| `deck-streak-mcp.service` | 15% | 32 |

The rest of this record stands.

## Amendment (SPEC-340): the sync family's archive and drill

ADR-351 D1 moves the archive of the sync server's snapshot out of the daily backup and the
snapshot's restore out of the drill, each into a oneshot of the sync family's own user. Each takes
the ceilings of the unit it was split from, in this record's form:

| unit | memory_high | memory_max | why |
|---|---|---|---|
| `deck-streak-sync-archive.service` (SPEC-340) | 48M | 64M | the daily backup's work it was, which reads each member of a generation in chunks through its digest, and each database through its check |
| `deck-streak-sync-restore-drill.service` (SPEC-340) | 96M | 128M | the drill's work it was, which opens each restored collection to count its cards |

Both are jobs, so the share is unchanged: the largest job's ceiling is still the job template's, and
the daemons' caps do not move. The rest of this record stands.

## Amendment (SPEC-396): the second route's ceilings

ADR-410 adds `deck-streak-second-route.service`, a oneshot a timer starts every ten minutes, a shell
script and two HTTPS requests as small as the alert template's. It takes that template's ceilings
in this record's form:

| unit | memory_high | memory_max | why |
|---|---|---|---|
| `deck-streak-second-route.service` (SPEC-396) | 32M | 48M | a shell script over the service manager's unit list and two HTTPS requests, the alert template's work |

It is a job, so the share is unchanged: the largest job's ceiling is still the job template's, and
the daemons' caps do not move. The rest of this record stands.

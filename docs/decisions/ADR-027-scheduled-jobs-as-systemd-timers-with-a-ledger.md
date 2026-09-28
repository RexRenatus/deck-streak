---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Scheduled jobs run as systemd timers through one runner that owns the cron-fire ledger, and the job table is the one schedule

## Context and Problem Statement

The predecessor ran its twelve jobs in-process under APScheduler, with a cron-fire ledger for
catch-up, a shared sync guard and an hourly dead-man watch. ADR-010 decided DeckStreak runs "units
per role (API, bot, scheduled jobs as timers)", and the deployment schematic draws
`deck-streak-job@<name>` oneshots behind timers, but no ADR weighs timers against an in-process
scheduler or says how the predecessor's catch-up and claim semantics survive the move. SPEC-027
builds the scheduler; which mechanism runs the jobs, where the schedule lives, and at which minutes?

## Decision Drivers

- The host: 1.9 GiB shared with the predecessor; a long-running process keeps the memory of its
  largest job, and the sync is the largest (ADR-022).
- Each job needs its own memory ceiling, its own failure page (`OnFailure=`), its own hardening and
  its own journal identity (the durable-services and observability packs).
- The predecessor's guards port verbatim (CHARTER 9): the claim before acting, the release after an
  undelivered send, the six-hour cap, the allowlist. Its sync-first guard does not: since ADR-037 no
  job but `sync` syncs, and a job reads the study day's sync outcome instead.
- One schedule, not two that can drift.
- ADR-011: while both run, DeckStreak's jobs keep off the predecessor's slots.

## Considered Options (the alternatives it was chosen against)

- A `.timer` per job activating `deck-streak-job@<id>.service`, a `oneshot` running `deckstreakd job <id>`, with the runner owning the ledger, the catch-up rule and the paging transitions, and `coordination::jobs::TABLE` as the schedule every timer is held equal to — chosen: each run returns its memory to the host at exit, each job has its own ceiling and page, and systemd needs no scheduler crate in the binary.
- An in-process scheduler in a long-running unit (tokio-cron-scheduler, or a hand-written tick loop) — rejected because the process keeps the peak memory of its largest job between runs, one crash stops every job at once, and one unit gets one ceiling and one page for twelve jobs.
- A scheduler unit that spawns child processes per job — rejected because it rebuilds, in the binary, what systemd timers already are, and still needs its own supervision.
- The predecessor's boot-time catch-up pass as its own unit — rejected because `Persistent=true` on a catch-up job's timer triggers the missed run at activation, and the runner's lateness check and claim give the same once-only, six-hour-capped result per job.

## Decision Outcome

Chosen option. The runner's rules are the predecessor's: a once-a-day job claims (job, local fire
date) before acting (the claim succeeds only while no attempt is recorded); a catch-up job more than
360 minutes late is recorded `missed`; a release follows only a positively failed delivery. A job
that repeats within the day (the liveness watch) records its outcome without a claim, because
running it twice changes nothing. The sync is a once-a-day job (ADR-037), claimed like the others:
its slot follows the rollover, so its fire date is the study day it runs in, and a second fire, a
restart or a manual run that day does nothing. `Persistent=true` is set only on catch-up jobs'
timers, as the predecessor replayed only its allowlist; at W0 that is `sync` alone (ADR-037, and
SPEC-027 R1: a sync missed by at most six hours runs once when its timer activates).

DeckStreak's slots, chosen to keep off the predecessor's minutes (its in-process slots, its sync
ticks and its three systemd timers) and off one another:

| job | slot | why here |
|---|---|---|
| `sync` | daily at the rollover hour, minute 7 (ADR-037) | ADR-037 decides this row: one scheduled sync per study day. The predecessor's sync ticks are 2, 17, 32 and 47; 5 minutes after its first tick of the study day gives its sync room to finish |
| `maintenance` | daily at the rollover hour, minute 28 | after the rollover, between the predecessor's 25 and 33 |
| `liveness` | hourly at minute 14 | clear of every predecessor minute, and 7 minutes after the daily sync slot |

The timers' calendars are rendered in the owner's zone by the private deploy rail; the templates
carry UTC as the neutral example, and the liveness job's drift check pages when a fire lands more
than 30 minutes off its slot, so a wrong zone is caught on the first day.

### How the runner decides (decided in SPEC-027's delivery)

Each choice below is the runner's, and each names what it was chosen against.

- **A catch-up job looks only at its fires of the local day it runs in.** With none elapsed it does
  nothing, and more than 360 minutes late it records `missed`: the predecessor's
  `scheduler.py:_latest_elapsed_fire_today`, which `goldens/catchup.json` holds it to. Chosen against
  the latest scheduled instant at or before now alone, which would replay a 22:00 fire after
  midnight, as the golden's `after_midnight` cases refuse.
- **Every page is a transition read from rows that already exist.** The first error of a job's error
  streak reads the job's last recorded outcome before the run; the first check to find the sync dead
  compares the watch's previous check with the instant the sync died; the first check to see a
  maintenance fire off its slot compares that fire with the previous check. Chosen against a table of
  paging state, which would be a second record of what `cron_fires` and `sync_runs` already hold, and
  against the predecessor's process-lifetime latch, which a `oneshot` process cannot keep.
- **A failed scheduled sync is an error of the `sync` job.** Its first failure of an episode pages,
  after its bounded retries, and a repeat only logs until a sync succeeds. That also covers the
  predecessor's immediate page on a hard failure (`engine_failed`, `open_failed`), which existed only
  because its other failures waited for three in a row. Chosen against a page every study day while
  the sync fails, since the dead-man watch pages again once the episode outlives its window, and
  against the predecessor's `SYNC_FAIL_ALERT_THRESHOLD` of three, which at one sync a day is three
  days.
- **The runner writes through ports**: `CronLedger`, `SyncCycle` and `DeliveryMarker`. The catch-up
  decision then runs against the golden's recording stub exactly as against `cron_fires`, and the
  `sync` job's cycle is SPEC-022's, through `coordination::sync_cycle`, built only when `sync` runs.
  Chosen against calling SQLite and the syncer from the runner, which no stub could stand in for, and
  against building the syncer for every job, which would refuse `maintenance` and `liveness` for want
  of the sync's own settings.
- **The drift check reads the timer's zone.** The offset that puts the maintenance fire on its slot
  (`signed_skew_minutes`) is compared with the configured offset (`rollover_skew_minutes`), so a timer
  rendered in the wrong zone reads as the skew between the two zones. Chosen against the
  predecessor's comparison of its scheduler's zone with the collection's own rollover, which needs
  the collection's configuration, and DeckStreak reads none at W0.
- **The boot grace counts from the first recorded sync attempt** in `sync_runs`. Chosen against the
  predecessor's process uptime, which a `oneshot` process does not have.
- **The upkeep prunes, then optimises, then checkpoints.** Chosen against the predecessor's order,
  checkpoint first, which leaves the prune's own frames in the write-ahead log until the next day.

### Consequences

- Good, because every run's peak memory, the sync's included, is released when the run exits, and
  each job's ceiling is its own.
- Good, because a failed job pages through the one alert path by failing its unit.
- Bad, because the schedule exists twice, in the table and in the timer files; SPEC-032's test holds
  them equal, and the drift check watches the live host.
- Bad, because each run starts a process and opens the database; for a Rust binary that is
  milliseconds against the hourly liveness watch, the most frequent job.

### Confirmation

SPEC-027's acceptance tests (the ledger and catch-up goldens, the schedule tests) and SPEC-032's
timer-versus-table test; durable-services' `timers.*` rows over `deploy/`.

## What would make this wrong

- A job must run more often than a process can start and finish (none at W0 comes close).
- The predecessor retires: ADR-011's reason for these slots goes, and the slots may move by a new
  ADR (the table and the timers move together).

## More Information

ADR-010; ADR-011; ADR-037 (the `sync` row); SPEC-027; SPEC-032; `docs/schematics/deployment.md`;
`docs/schematics/cron-fire-ledger-and-catch-up.md`; the durable-services pack's timer rows.

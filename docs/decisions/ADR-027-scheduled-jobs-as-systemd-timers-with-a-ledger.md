---
status: proposed
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
  undelivered send, the six-hour cap, the allowlist, the sync first.
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
that repeats within the day (the sync, the liveness watch) records its outcome without a claim,
because running it twice changes nothing; a per-date claim on a 15-minute job would run it once a
day. `Persistent=true` is set only on catch-up jobs' timers, as the predecessor replayed only its
allowlist; none exists at W0.

DeckStreak's slots, chosen to keep off the predecessor's minutes (its in-process slots and its three
systemd timers) and off DeckStreak's own tick set:

| job | slot | why here |
|---|---|---|
| `sync` | every 15 minutes at minutes 7, 22, 37, 52 (tick offset 7) | the predecessor's sync ticks are 2, 17, 32 and 47; 5 minutes after each gives its sync room to finish |
| `maintenance` | daily at the rollover hour, minute 28 | after the rollover, between the predecessor's 25 and 33 |
| `liveness` | hourly at minute 14 | clear of every predecessor minute and 7 minutes after a sync tick |

The timers' calendars are rendered in the owner's zone by the private deploy rail; the templates
carry UTC as the neutral example, and the liveness job's drift check pages when a fire lands more
than 30 minutes off its slot, so a wrong zone is caught on the first day.

### Consequences

- Good, because the sync's peak memory is released every 15 minutes, and each job's ceiling is its
  own.
- Good, because a failed job pages through the one alert path by failing its unit.
- Bad, because the schedule exists twice, in the table and in the timer files; SPEC-032's test holds
  them equal, and the drift check watches the live host.
- Bad, because each run starts a process and opens the database; for a Rust binary that is
  milliseconds against a 15-minute interval.

### Confirmation

SPEC-027's acceptance tests (the ledger and catch-up goldens, the schedule tests) and SPEC-032's
timer-versus-table test; durable-services' `timers.*` rows over `deploy/`.

## What would make this wrong

- A job must run more often than a process can start and finish (none at W0 comes close).
- The predecessor retires: ADR-011's reason for these slots goes, and the slots may move by a new
  ADR (the table and the timers move together).

## More Information

ADR-010; ADR-011; SPEC-027; SPEC-032; `docs/schematics/deployment.md`;
`docs/schematics/cron-fire-ledger-and-catch-up.md`; the durable-services pack's timer rows.

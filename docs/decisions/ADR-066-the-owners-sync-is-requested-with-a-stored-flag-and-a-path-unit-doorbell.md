---
status: "accepted"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The owner's sync is requested with a stored flag and a path unit doorbell

## Context and Problem Statement

A sync needs more memory than the bot's limit, so the bot cannot run it (SPEC-059). The sync job's
unit is sized for it. How does the owner's `/sync` reach the job, and how does the job know the
owner asked, when the runner claims only scheduled fires?

## Decision Drivers

- The bot holds one privilege for this: to ask. It must not start units.
- The job must not trust anything a writer of the request file puts in it.
- ADR-037: one scheduled sync per study day, the owner's explicit triggers, no new schedule.
- `systemd.path(5)`: `PathChanged=` fires on close after write and is edge triggered.

## Considered Options

- Raise the bot's memory limit. Rejected: the bot then holds a sync's memory all day for one
  command, and the host budget has no room for it.
- The bot starts the unit itself with `systemctl` through polkit or sudo. Rejected: a privilege
  broader than one trigger path.
- Run the sync in the bot within its limit. Rejected: the engine's read is the measured cost.
- `$TRIGGER_PATH` as the job's discriminator. Rejected: `systemd.exec(5)` calls it lossy,
  best-effort and not to be relied on, and it dates from v252.
- `PathExists=` with the job consuming the file. Rejected: the job would need write access to the
  directory, and a level trigger restarts the job while the file remains.
- A new request table. Rejected: the pending flag already exists and clears with the recompute.
- An owner-specific job id. Rejected: it changes the job table ADR-037 holds.

## Decision Outcome

Chosen: the bot stores the request (`request_rescore`), touches a file only the bot can write, and
`deck-streak-job@sync.path` starts `deck-streak-job@sync.service`. The job reads the stored flag
and, when set, runs the owner's cycle before the scheduled run. The bot polls the store for the
outcome within a bound and answers, or says the sync is still running.

### Consequences

- The file is a doorbell: its content is never read, so a planted payload changes nothing.
- Requests coalesce: an edge trigger starts at most one more run; the reuse window still answers.
- The bot unit drops the sync login and adds one writable directory.
- The path unit's live behaviour is cited from `systemd.path(5)`; a user manager was not available
  to measure it in the build sandbox.

### Confirmation

`crates/daemon/tests/sync_request.rs` and `scripts/tests/test_sync_path.py`, and the S059xx rows.

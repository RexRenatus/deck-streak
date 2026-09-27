---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The topic lock is an exclusive file lock in one shared directory, taken without waiting on the owner's tap

## Context and Problem Statement

The owner's on-demand ruling requires one named lock that serialises on-demand and nightly work per
topic, so that one note never has two writers. The writers run in different processes: the API
answers the Mini App's tap, the bot answers a callback, and the nightly generation runs in its own
job unit (ADR-010). What mechanism holds that lock across processes, survives a crashed holder, and
answers the owner's tap at once?

## Decision Drivers

- Never a second writer of one topic's reading.
- The owner's tap is acknowledged immediately; generation runs off the request path.
- A crashed or killed holder must not leave the topic locked until someone intervenes.
- No new dependency and no new long-running process on a small host.

## Considered Options (the alternatives it was chosen against)

- An exclusive advisory file lock per topic (`std::fs::File::try_lock`, stable in the pinned toolchain) on a file in one lock directory shared by every unit that writes readings; the tap uses `try_lock` and answers `busy` when it is held, and the nightly run waits a bounded time — chosen: the kernel releases the lock when its holder closes the file or exits, so no lease can go stale, and no crate is added.
- An in-process mutex keyed by topic — rejected because the API, the bot and the job are separate processes, so it would serialise nothing between them.
- A lease row in SQLite with an expiry — rejected because a crashed holder blocks the owner's tap until the lease expires, and a holder that outlives its lease needs fencing tokens on every write to stay the only writer.
- A single readings worker fed by a queue table — rejected because it needs a long-running worker and a way for the API and the bot to wake it, which the timer-based job design does not have.
- Blocking the tap until the lock is free — rejected because a tap would hang for up to a reading's wall clock.

## Decision Outcome

Chosen option. The lock file is `readings-<topic hash>.lock` in the configured lock directory; the
file is opened with the close-on-exec flag the standard library sets, so a child such as the agent's
runner never inherits the lock. The tap takes it with `try_lock` and answers `busy` when it is held;
the nightly run waits at most one reading's wall clock and then leaves the topic to its holder. A
regeneration is addressed by a pick token (the study day's ordinal and a hash of the topic key), so
no topic is ever typed and a stale listing is refused.

### Consequences

- Good, because a crash or a kill frees the topic at once, with no cleanup job.
- Good, because the owner sees `started` or `busy` immediately.
- Bad, because every writing unit must share one lock directory; a unit pointed at another directory
  would lock alone, so each checks the directory at start.
- Bad, because an advisory lock binds only code that takes it; every writer of a reading goes through
  one use case that takes it first.

### Confirmation

SPEC-048's tests: the concurrent-writer test (A4), the crashed-holder test (A6) and the busy answer
(A3).

## What would make this wrong

- Readings writers move to more than one host (then the lock needs a shared store).
- A future unit writes readings without the shared lock directory (the start check refuses it).

## More Information

SPEC-048; ADR-010 (the units); the Rust standard library's `File::lock`, `File::try_lock` and
`TryLockError::WouldBlock`.

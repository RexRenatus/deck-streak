---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The held queue is flushed by a scheduled step outside the quiet window

## Context and Problem Statement

SPEC-041 R7 flushes the held queue after every successful sync. The scheduled sync runs inside the
default quiet window (23:00 to 07:30), so on that path the flush finds the window closed and delivers
nothing: a celebration held overnight waits for some sync that lands after 07:30. Issue #291 asks for
the held queue to be flushed when the window ends. Two further facts shaped the answer. Only the
bot's process held a router, so the scheduled job's process had none to flush with. And the
flush's first transaction was committed before its sends, so two flushers over one queue could each
read the same held row and send it twice. What triggers a flush outside the window, and how do two
flushers share one queue?

## Decision Drivers

- Every held notification reaches the owner exactly once or is abandoned by name, on every path that
  can flush; a flush delivers only while the window is open when it sends.
- The step's calendar is derived from the window the router reads, never a second copy of it.
- A missed fire does not strand a hold past its age limit (720 minutes).
- No schema change: a `state` change to the queue is a table rebuild.
- The two flushers are an interleaving, so a model states the properties before the code (the
  entry `formal/tla/HeldFlush/`).

## Considered Options (the alternatives it was chosen against)

The options were measured against one generated population of 96 flush cases (3 flushers x 8 clock
positions x 4 hold states) and against the model, whose witness configurations switch each rejected
shape on.

- A scheduled job of its own, `held_flush` at 07:36 UTC with catch-up — chosen because over the 96
  cases it delivers or abandons each hold exactly as expected, and the model holds both properties
  clean at 558 distinct states with its lease on.
- Flushing inside the sync job only — rejected because it is the shape that failed: the model's
  witness with no scheduled flusher violates `HeldReachesOrAbandons`, and at dev 12 of the 96 cases
  (the scheduled-step arm with the window open) delivered nothing. A sync that is refused, skipped
  or late never flushes.
- A readings job that does not exist yet — rejected because the readings jobs (#39) are not built,
  so the flush would wait on them, and a readings step runs on a readings job's failure and retry
  rules, which a flush must not inherit.
- A delay loop in the daemon that sleeps until the window ends — rejected because it adds a second
  clock beside the timer, is lost at every restart where a `Persistent=` timer replays the miss, and
  lives in the long-running process rather than the job a ledger records.

## Decision Outcome

Chosen option: "A scheduled job of its own", `held_flush`, a daily job at 07:36 UTC with catch-up,
with its own timer, drop-in and job-table entry, because it is the only shape that flushes on a
path with no sync, and its calendar is checked against the policy's window by a test over the job
table and the deploy templates.

- **The calendar.** 07:36 is six minutes after the default window ends. With `Persistent=true` a
  missed fire is replayed up to 360 minutes late (13:36), still outside the window, and the oldest
  hold then aged at most 510 minutes against the 720 limit, so a missed fire never costs a hold its
  delivery.
- **The catch-up decision.** `catch_up: true`, because a flush is idempotent: a replayed fire finds
  an empty queue or delivers what the first fire would have, and the lease keeps two fires from
  overlapping.
- **The lease, chosen against a schema change.** A flush takes a lease before its first send, held
  as the setting `flush_lease` whose value is the instant it lapses (ten minutes on), and releases it
  when it ends. A flush that finds an unlapsed lease answers `Busy` and sends nothing. The
  alternative, a `flushing` state on the queue's rows, was rejected: the queue's `state` column is
  constrained, and a change to it is a table rebuild of a ledger the deployed database already holds.
  The lease uses the non-macro query path, so the offline query cache does not change.
- **The job's router.** The job process builds its own router from the policy, the owner's chat and
  the bot's credentials, loaded by a service drop-in with the same two credentials the bot loads.

### Consequences

- Good, because a hold raised in the window reaches the owner at the first scheduled flush after it
  ends, on a day when no sync has landed.
- Good, because two flushers never send one item twice: the model's witness with no lease violates
  `NoDoubleDelivery`, and the concurrency test shows the second flush answering `Busy`.
- Bad, because a flush that starts just before the window opens checks the window once, at its start,
  and does not re-check it at each send; a long flush started at 22:59 can send after 23:00. Held
  items are few (the queue is bounded at 20), so the exposure is seconds.
- Bad, because the lease is a row in the settings table, so the data-rights export lists
  `flush_lease` while a flush holds it.
- Bad, because a delete-by-token release is not mutation-tested: dropping its `AND value = ?` is
  equivalent except in a flush that outlives its own lease, which the test clock cannot reach.

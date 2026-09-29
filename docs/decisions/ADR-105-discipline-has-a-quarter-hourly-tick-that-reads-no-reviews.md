---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# Discipline's clock is a quarter-hourly tick that reads no reviews, and the standby notice waits for it

## Context and Problem Statement

A committed window's reminder is due 0 to 15 minutes before the window starts (#109), and the
standby notice is due when the governor's verdict enters standby (SPEC-076 R17, left to #109).
Neither reads a review: the reminder reads the booked windows, the clock and the day's due count;
the notice reads the governor's stored state.

DeckStreak's job table (SPEC-027 R1) knows three schedule kinds, none shorter than an hour, and
reads reviews once a study day plus the owner's `/sync` (ADR-037). The predecessor raises both
duties inside its sync, which runs every few minutes
(`pipeline_layers/committed_windows.py:CommittedWindowsLayer._window_reminders`,
`pipeline_layers/governor.py:GovernorLayer._update_governor`, predecessor `27ee2bc`).

SPEC-076 recorded that the notice's rule never falls due at the scheduled settle, which runs inside
quiet hours by default. The router withholds a nudge raised inside quiet hours, and defers only a
celebration (SPEC-041 R4, step 4). A notice raised at the settle would therefore be dropped.

Which clock raises a reminder and the notice, and what does it read?

## Decision Drivers

- ADR-037: no job adds a sync; a review is read by the sync cycle only.
- A reminder after its window has started is worthless, and one an hour early is not a reminder.
- Every message goes through the one router, with its quiet hours, its lapse rule and a setting.
- SPEC-027 R2: no job shares a minute with the predecessor's schedule while both run, or with a
  reserved minute.
- A notice must be raised once, whatever runs at once on the host.

## Considered Options (the alternatives it was chosen against)

- A quarter-hourly job at minutes 4, 19, 34 and 49 that reads no review, raises the reminders, and raises a notice the settle left pending — chosen, because each window start on the 30-minute grid falls inside exactly one tick's 15-minute lookahead and the notice reaches the owner outside quiet hours.
- An hourly job with a 60-minute lookahead — rejected because a reminder could arrive up to 71 minutes before its window, where the predecessor's lookahead is 15 minutes (`windows.py:REMINDER_LOOKAHEAD_MIN`).
- Raise both from the sync cycle, as the predecessor does — rejected because under ADR-037 a cycle runs at the rollover and at the owner's `/sync`, so almost no reminder would fall due.
- Raise more syncs so the cycle comes often enough — rejected because ADR-037 leaves the cadence to the owner at cutover (#164), and the tick needs no review.
- Raise the notice at the settle through the router — rejected because the settle runs inside quiet hours by default and the router withholds a nudge there (SPEC-041 R4).
- Raise the notice as an alert — rejected because an alert is exempt from quiet hours and has no owner setting, so an informational notice could wake the owner.
- Raise the notice as a celebration, which the router defers — rejected because standby is not a reward, and the celebration kind's dedupe is once-ever, so a second entry into standby could never be announced.
- Fold the notice into the daily digest — rejected because the digest is its own kind behind its own setting, so an owner who switched the digest off would never learn that the engine stood down (#129).
- Wait for the router's flush at the end of quiet hours (#291) — rejected because that flush delivers deferred celebrations, and a nudge is withheld, not deferred.

## Decision Outcome

Chosen option: "a quarter-hourly tick that reads no reviews", built by SPEC-105.

- **The schedule kind.** The job table gains a quarter-hourly kind: it fires at a minute from 0 to
  14 and every 15 minutes after it. The job `discipline_tick` fires at minutes 4, 19, 34 and 49,
  none a predecessor minute (the in-process slots and the sync ticks), a reserved minute (0, 25,
  39) or the sync's slot. It does not catch up.
- **What it does.** It raises each reminder whose window starts 0 to 15 minutes ahead, and a
  pending standby notice. It reads no review, syncs nothing and judges nothing: every verdict stays
  with the sync cycle (ADR-104). SPEC-106 joins it with the stakes' messages a settle left pending
  and the Sunday review.
- **The notice.** The governor's step of a settle stores the notice as pending when SPEC-076's
  rule holds with quiet hours set aside. The first tick outside quiet hours raises it under the kind
  `discipline` while the verdict is still standby and no lapse is open, clearing the pending day
  and recording the notice's study day in one write. A verdict out of standby, or a lapse, drops it.

- **A sync step's messages.** The rail's step (SPEC-104 R15 and R16) and the markets step (SPEC-107
  R16) record each message pending in the ledger, as SPEC-106 R17 does. An owner's sync carries a
  router, so the cycle's flush delivers the message at once and nothing stays pending; a scheduled
  sync carries none, so the message waits for `discipline_tick`, which delivers it through the
  router on its next run. Rejected:
  - a router for the scheduled sync: rejected because the sync job would load the bot credentials,
    against ADR-124's census that `sync` does not send, and whether it ever does is #291's;
  - leave the messages to #291: rejected because the plan would carry a known hole where a scheduled
    sync's rail and market messages are never sent;
  - a new sending job that only flushes: rejected because `discipline_tick` already runs
    quarter-hourly on the sending template and already flushes pending discipline messages.

### Consequences

- Good, because reminders keep the predecessor's 15-minute lookahead without a sync.
- Good, because the notice is raised the morning after the settle instead of never.
- Good, because a later discipline duty that reads no review has a clock to join.
- Bad, because DeckStreak runs 96 more job fires a day, each a short read of discipline's state.
- Bad, because a new schedule kind extends SPEC-027 R1, and the job-table test must hold four
  minutes for one job.

### Confirmation

SPEC-105's A14 (the minutes and the calendar), A15 and A16 (the reminder), A17 and A18 (the notice),
SPEC-104's A30 and SPEC-107's A40 (a scheduled sync's step messages wait for the tick), and the
hand-proved rows `S10513` and `S10515`.

## What would make this wrong

- The owner raises the sync cadence to minutes at cutover: the sync cycle could then raise the
  reminders, and the tick would be kept for the notice only.
- The router gains a deferral for nudges held in quiet hours: the notice could then be raised at
  the settle, and the pending state would go.

## More Information

Cites ADR-011 (the reserved minutes), ADR-027 (jobs are timers with a ledger), ADR-037 (the
cadence), ADR-104, SPEC-027, SPEC-041 and SPEC-076. The schematic is
`docs/schematics/discipline-tick-and-standby-notice.md`.

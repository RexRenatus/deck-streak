---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The readings jobs: fixed slots that sync first, the settle after every sync, one morning job, a caught-up generation, and a vault archive switched on only when DeckStreak is the single writer

## Context and Problem Statement

ADR-010 decided that scheduled jobs are systemd timers, one unit per job. The readings need a nightly
generation "after the first sync of the study day", a studied measure that keeps pace with the owner's
study, a morning message that is either the comeback reading or the ready line, and a start-up
catch-up policy. During side by side the predecessor keeps its own schedule, and its nightly readings
job still fires against the same vault folder. When does each piece run, and when may DeckStreak write
the vault?

## Decision Drivers

- Readings generate whenever the last sync succeeded (ADR-019), after the study day has turned.
- DeckStreak keeps off the predecessor's slots and every legal sync tick (ADR-011, the predecessor's
  tested rule).
- The studied chip should move within one sync interval of the owner's study.
- One writer per vault contract at every moment (ADR-011).

## Considered Options (the alternatives it was chosen against)

- A generation timer at the rollover hour plus 40 minutes that makes sure a sync of the new study day succeeded before it resolves; a morning timer at 08:10 that branches between the comeback reading and the ready line; the settle as a step after every successful sync; both jobs in the start-up catch-up within 360 minutes; a vault archive switch that stays off until the predecessor's readings job is disabled — chosen: fixed, provably free slots, a fresh chip, and one writer.
- Chain the generation off the first successful sync after the rollover — rejected because its start time would vary with the sync and could land on a slot the predecessor or the second brain uses.
- Give the settle its own timer — rejected because it needs another free slot and still lags the sync that brought the reviews; after the sync it measures exactly what that sync brought.
- Settle once a night, as the predecessor did — rejected because the chip would lag the owner's study by up to a day.
- Two morning jobs, one per branch — rejected because both could fire on one morning; one job taking one branch cannot.
- Keep the generation out of the catch-up, as the predecessor did to save paid calls — rejected because the owner set no spend cap and a reading generated late in the morning is still read that day.
- Write the vault archive from the first deploy — rejected because the predecessor's readings job still fires and would become a second writer the first night its gate passed.

## Decision Outcome

Chosen option. `readings-generate` fires at the rollover hour plus 40 minutes and `readings-morning` at
08:10; a test proves both minutes free of every legal sync tick and of the predecessor's and the second
brain's slots. The settle runs in the post-sync step list with the router's flush. The catch-up bound
is SPEC-027's 360 minutes. `readings_vault_archive` defaults to off, and the first live night switches
it on after the owner's go disables the predecessor's readings job.

### Consequences

- Good, because a reading's state and chip are never more than one sync behind.
- Good, because at no moment do two services write one note.
- Bad, because until the first live night the vault holds no archive copy; the Mini App holds every
  reading meanwhile.
- Bad, because a restart after downtime can run a generation late in the morning.

### Confirmation

SPEC-053's tests (the claim, the sync-first rule, the schedule test, the switch) and the durable-services
unit rows over the new templates.

## What would make this wrong

- The owner moves the rollover hour so far that 08:10 falls before the new study day's generation (then
  the morning slot follows the rollover).
- The predecessor is retired before W2's first live night (then the switch can default on).

## More Information

SPEC-053; ADR-010; ADR-011; ADR-019; the predecessor's `timebase.py:tick_minutes`;
docs/schematics/readings-generation-flow.md.

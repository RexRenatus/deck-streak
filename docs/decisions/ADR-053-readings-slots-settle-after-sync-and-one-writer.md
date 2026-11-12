---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The readings jobs: fixed slots that read the study day's sync, the settle after every successful sync, one morning job, a caught-up generation, and a vault archive switched on only when DeckStreak is the single writer

## Context and Problem Statement

ADR-010 decided that scheduled jobs are systemd timers, one unit per job. The readings need a nightly
generation after the study day's sync, a studied measure that keeps pace with the owner's study, a
morning message that is either the comeback reading or the ready line, and a start-up catch-up
policy. ADR-037 syncs once per study day, at the rollover hour, minute 7, plus the owner's `/sync`,
and lets no other job sync. During side by side, DeckStreak's jobs keep off the predecessor's slots
(ADR-011), and the vault's readings folder keeps one writer. When does each piece run, and when may
DeckStreak write the vault?

## Decision Drivers

- Readings generate whenever the last sync succeeded (ADR-019), after the study day has turned, and
  no job but the daily sync syncs (ADR-037).
- DeckStreak keeps off the predecessor's slots, its sync ticks included, and off its own daily sync
  slot (ADR-011, ADR-037).
- The studied chip should move with every sync that brings the owner's study, the owner's triggers
  included.
- One writer per vault contract at every moment (ADR-011).

## Considered Options (the alternatives it was chosen against)

- A generation timer at the rollover hour plus 40 minutes that reads the study day's sync outcome, never syncs, and resolves only after that sync succeeded; a morning timer at 08:10 that branches between the comeback reading and the ready line; the settle as a step after every successful sync; both jobs in the start-up catch-up within 360 minutes; a vault archive switch that stays off until DeckStreak is the readings folder's one writer — chosen: fixed, provably free slots, a fresh chip, and one writer.
- Sync before the generation, as the W0 plan's sync-first jobs did — rejected because each such job adds a sync to the day, which ADR-037 forbids; the generation reads the study day's sync outcome instead.
- Chain the generation off the first successful sync after the rollover — rejected because its start time would vary with the sync and could land on a slot it must keep off.
- Give the settle its own timer — rejected because it needs another free slot and still lags the sync that brought the reviews; after the sync it measures exactly what that sync brought.
- Settle once a night, as the predecessor did — rejected because a sync the owner triggers would bring reviews that the chip then shows only the next night.
- Two morning jobs, one per branch — rejected because both could fire on one morning; one job taking one branch cannot.
- Keep the generation out of the catch-up, as the predecessor did to save paid calls — rejected because the owner set no spend cap and a reading generated late in the morning is still read that day.
- Write the vault archive from the first deploy — rejected because the readings folder keeps one writer, and DeckStreak becomes it only on the owner's go.

## Decision Outcome

Chosen option. `readings-generate` fires at the rollover hour plus 40 minutes and `readings-morning` at
08:10; a test proves both slots free of the daily sync slot, of the predecessor's slots, its sync
ticks included, and of every slot the private deploy rail reserves for the host's other services,
proved in CI on a synthetic list. The generation never syncs: it reads the study day's sync
outcome, and when that did not succeed every topic is `could_not_tell` with `sync_failed` until the
owner triggers a sync and regenerates (ADR-037, SPEC-048). The settle runs as a step of the sync
cycle (`coordination::sync_cycle`) after each successful sync, scheduled or owner-triggered, beside
the router's flush. The catch-up bound is SPEC-027's 360 minutes. `readings_vault_archive` defaults
to off, and the first live night switches it on once the owner's go makes DeckStreak the readings
folder's one writer.

### Consequences

- Good, because a reading's state and chip are never more than one sync behind.
- Good, because at no moment do two services write one note.
- Bad, because until the first live night the vault holds no archive copy; the Mini App holds every
  reading meanwhile.
- Bad, because a restart after downtime can run a generation late in the morning.

### Confirmation

SPEC-053's tests (the claim, the rule that the generation never syncs, the schedule test, the switch)
and the durable-services unit rows over the new templates.

## What would make this wrong

- The owner moves the rollover hour so far that 08:10 falls before the new study day's generation (then
  the morning slot follows the rollover).
- The predecessor is retired before W2's first live night (then the switch can default on).

## More Information

SPEC-053; ADR-010; ADR-011; ADR-019; ADR-037; the predecessor's `timebase.py:tick_minutes`;
docs/schematics/readings-generation-flow.md.

Met by ADR-065 (proposed): the owner's go makes DeckStreak the readings folder's one writer once the
private rail has fenced every other writer it lists from the folder with a read-only mount, and has
proved each fence by a refused write. The rest of this ADR stands.

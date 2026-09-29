---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The Can-Do unlock pass runs after each sync's recompute and records each unlock once

## Context and Problem Statement

The Can-Do ladder (#92) lists the owner's own statements of what they can do, each unlocked when
enough of its course's cards are mature. The predecessor scans every note when the owner asks
(`cando.py:unlock_pass` behind the command, predecessor `27ee2bc`) and dates an unlock by
that scan, so the date is the day the owner next asked, and the command waits on a full scan. The
issue's port note suggests the pass run after a sync. When does the pass run, and what does it
keep?

## Decision Drivers

- An unlock's date is a fact about the owner's study, not about when they asked.
- A full scan of the notes stays off the recompute's write path (SPEC-020's offload).
- A rung that falls back after an unlock is still a rung the owner once reached.
- The ladder answers at once in the bot and the Mini App.
- A failed read is a failure, never an empty ladder.

## Considered Options (the alternatives it was chosen against)

- After each sync's recompute commits, coordination runs the pass through the offload and makes one write that records each unlock once in `can_do_unlocks` and replaces `can_do_ladder` with the latest pass — chosen: an unlock is dated by the sync that first saw it, and the ladder never waits on a scan.
- On demand when the owner asks, as the predecessor did — rejected because the unlock's date then depends on when the owner asked, and the command waits on a full scan.
- Inside the recompute's own transaction — rejected because a scan of every scoped note would sit on the fold's write path.
- Derived at read time with nothing stored — rejected because it loses each unlock's date, and a rung that falls back would vanish from the unlocked list.

## Decision Outcome

Chosen option: "A pass after each sync's recompute, with unlocks recorded once", because the date
belongs to the study that earned it and the scan never blocks a reader or the fold.

- The pass runs after the recompute commits and before the instruments step (SPEC-094's frame),
  one at a time through the offload (SPEC-099 R6).
- `can_do_unlocks` is keyed by statement and course and keeps the first instant; a rung already
  recorded is never rewritten.
- `can_do_ladder` holds the latest pass, replaced in the same write; a failed read replaces it with
  the failure and records no unlock, and the recorded unlocks stay (SPEC-099 R7).

### Consequences

- Good, because the ladder is ready whenever the owner asks, and an unlock's date is the sync's.
- Good, because a relapsed rung stays unlocked in the record and is also listed as locked.
- Bad, because every sync pays one more read of the scoped notes; the pass reads only the Can-Do
  field's notes, off the write path.
- Bad, because an unlock between two syncs is dated by the later sync.

### Confirmation

SPEC-099's criteria on the pass's placement after the recompute, the record kept once with its first
instant, the failed read, and export and erase of both tables.

## What would make this wrong

- The pass's read is slow enough to delay the next sync; it then becomes a weekly instrument
  under ADR-094, and an unlock is dated by that run.
- The owner wants an unlock dated by the card that crossed the threshold; the pass then reads the
  review log and dates the unlock by that answer.

## More Information

ADR-002; SPEC-020 (the offload); SPEC-071 (the recompute); SPEC-094 (the frame); SPEC-099, which
builds the ladder; #92.

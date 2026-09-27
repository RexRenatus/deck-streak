---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Reading XP and the studied measure: a two-study-day window that late reviews can still reach, constants held by the readings, and a tick only the owner's tap writes

## Context and Problem Statement

ADR-019 recorded the owner's decisions: 40 XP when a reading is marked read, plus 60 when at least
80% of its new cards are reviewed within two study days, at most 100, granted once; "I read it"
written only on the owner's tap and never cleared by code; no new streak. Five points remain open:
exactly which reviews fall inside "two study days", whether the verdict is final when the window
closes, where the two amounts live, how a grant is keyed so it is paid once, and what happens when
the vault is unreachable at the moment of the tap.

## Decision Drivers

- A false verdict against the owner is worse than a late one (constraint 6: time-boxed verdicts are
  revisable against late-arriving reviews).
- The day turns at 04:00, never at midnight (constraint 7).
- One home for each constant: `economy.json` is the game's, and its pack refuses an unknown key.
- The owner's tick is the owner's: code never writes it on its own.

## Considered Options (the alternatives it was chosen against)

- The window is the generation study day and the next one, counting reviews made at or after generation; a retired reading is re-measured whenever a sync brings reviews made inside its window; the amounts are constants of the readings context sourced to the owner's decision; both grants are `once`-scoped with sources built from the reading's id, itself derived from the reading's identity; a failed vault tick is recorded pending and retried only by the owner's next tap — chosen: it follows the owner's words and the charter's rule on revisable verdicts.
- A 48-hour window from generation — rejected because the owner said study days, and a study day turns at 04:00 local.
- Close the verdict for good when the window ends — rejected because a review made inside the window but synced after it would be lost, the false verdict constraint 6 forbids.
- Put the amounts in `economy.json` under the XP bonuses — rejected because the game-economy probe's `economy-declared` row refuses a key its reference lacks, and changing the reference is a delivery of that pack on the owner's decision.
- Retry a failed vault tick in the background — rejected because the line would then be written by code rather than by the owner's tap.
- Settle the studied measure only in the nightly run, as the predecessor did — rejected because the Mini App's chip would lag the owner's study by up to a day.

## Decision Outcome

Chosen option. The readings context holds `READ_XP = 40` and `STUDIED_XP = 60`, pinned by a test to
the owner's decision. The settle step runs after each successful sync, grants and stamps on
crossing, retires at the rollover that starts the third study day, and revises a retired reading
when late reviews made inside its window arrive. The read tick's only caller is the tap use case;
a failed tick waits for the owner's next tap.

### Consequences

- Good, because a studied reading is never missed for want of a sync, and never paid twice.
- Good, because the chip moves within one sync interval of the owner's study.
- Bad, because the game-economy pack cannot see the reading amounts; if the owner wants them
  audited with the rest of the economy, the pack's reference must learn a readings section first.
- Bad, because a vault tick that failed stays pending until the owner taps again; the Mini App
  shows it.

### Confirmation

SPEC-047's tests and the golden of `preread_tracking.py:is_studied`.

## What would make this wrong

- The game-economy pack gains a readings section in its reference (then the constants move into
  `economy.json` and this ADR is superseded on that point).
- The owner asks that a vault tick missed at the tap be written as soon as the vault is back.

## More Information

SPEC-047; ADR-019; SPEC-040's `once` scope; the game-economy pack's `economy-declared` row.

---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Cutover: run beside v9 under a new bot, verify parity, retire v9 only on the owner's word

## Context and Problem Statement

The predecessor serves the owner every day, and it holds contracts other systems rely on: the
stats file it writes to the vault, the drill post-back, and (by design) the readings folder. A
big-bang switch would risk every one of them.

## Decision Drivers

- The owner: the new stack runs side by side with v9 under the new bot's token, and v9 keeps running until parity is verified.
- Each vault contract has exactly one writer at any time.
- The notification router must never double-message the owner across the two bots.

## Considered Options (the alternatives it was chosen against)

- Side by side, contract by contract: DeckStreak runs read-only against its own collection copy and its own database, writes only the readings archive (the folder v9 has never written), and takes over each other vault contract and each notification kind one at a time — chosen: every step is reversible and verified.
- A big-bang switch at one moment — rejected because a regression in any of 131 features would hit the owner at once.
- Keep v9 as the backend and build only the Mini App — rejected because the owner chose a Rust port with the VM as its backend.

## Decision Outcome

Chosen option. During side by side, DeckStreak's jobs keep off v9's schedule slots, and its
notifications are limited to the kinds v9 does not send (the readings first). A cutover checklist
(W8) moves each remaining contract: the stats file, the drill post-back and the digests, each with
a verification run. The v9 import (ADR-008) runs last, from a verified backup. v9 is stopped and
retired only by the owner's explicit go (owner gate 5), and it is kept disabled, not deleted,
until the owner approves its removal.

### Consequences

- Good, because the owner can compare the two surfaces day by day.
- Bad, because both run for a while, and DeckStreak's host budget is sized to hold through it; a resize is an owner decision if it does not fit.

### Confirmation

The W8 cutover SPEC's checklist, each item a command with its output; the owner's recorded go.

## What would make this wrong

- DeckStreak exceeds its memory budget while both run (the memory watch pages); that forces an owner decision on a resize or a faster cutover.

## More Information

ADR-008; ADR-010; docs/schematics/data-flow.md.

Amendment (2026-09-28): passages describing the predecessor as a running service and the host's
memory, in the context, the outcome, a consequence and a condition that would make this wrong, were
redacted under the public-prose rule (ADR-059).

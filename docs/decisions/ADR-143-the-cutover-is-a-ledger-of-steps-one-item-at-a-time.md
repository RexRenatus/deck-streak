---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The cutover is a ledger of steps, the rail's stop before each switch, one item at a time

## Context and Problem Statement

ADR-011 keeps one writer per vault contract and per notification kind at every moment, and gives
the stop and the retirement of the predecessor to the owner's explicit go. DeckStreak has 21
contracts and kinds the predecessor's code also writes, each behind a switch, a staged duty, a job
or the readings archive switch. How does each move from the predecessor to DeckStreak, and what
proves the order?

## Decision Drivers

- One writer per contract at every step (ADR-011).
- The predecessor's operation is the owner's and the private rail's (#41); the public repository
  names none of it (ADR-059).
- A failed move must be traced to one contract, and undone.
- The evidence of each move must be recorded without its personal values.

## Considered Options (the alternatives it was chosen against)

- An append-only ledger of steps, one item in flight: chosen, because the ledger proves the order,
  and one item in flight makes every failure one contract's. The steps are `go`, `stopped`,
  `switched`, `verified` and `reverted`, and the rail's stop is recorded before the switch.
- A big-bang switch: rejected because every contract would have two writers or none at the same
  moment, and a fault could not be traced to one of them.
- Switches flipped by hand, with no ledger: rejected because nothing would prove that a stop came
  before its switch, and a hand-set switch reads the same as a move.
- DeckStreak stopping the predecessor itself: rejected because the predecessor's operation belongs
  to the private rail and the owner, and a public tool that names it breaks ADR-059.
- Several items in flight: rejected because a failed verification would not name its contract, and
  the window with an unproved writer would grow with every item.

## Decision Outcome

Chosen option: the ledger of steps, one item at a time, because it makes ADR-011's single writer a
recorded fact at every step. The verification's output stays private; the ledger holds its
command line and sha256. SPEC-143 holds it, and SPEC-144 adds `retired` and `alone`.

### Consequences

- Good, because `status` shows the whole cutover, and drift (a switch set by hand) fails every
  verification.
- Bad, because 21 items one at a time take as many verification runs, each waiting on a
  delivered run of its gate.

### Confirmation

SPEC-143's refusal, drift, gate and census criteria, and the runbook test.

## More Information

#41, #62, #164, ADR-011, ADR-059, SPEC-143, SPEC-144.

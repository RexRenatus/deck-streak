---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Each vault duty is its own staged run, the curator returns a plan, and a journal capture is never an input

## Context and Problem Statement

#49 and #50 add three vault duties: the inbox curator, tomorrow's daily note and the Sunday
synthesis. SPEC-042 R4's executor applies a staged run only after the vault-duties pack's blocking
classes are green, and a red class discards the whole run. The inbox holds the owner's captures,
including journal-shaped ones (#56), which must never reach a model. What does the model write,
how many staged runs does one pass make, and how does a journal capture stay out?

## Decision Drivers

- A capture is the owner's only copy: it moves byte for byte under its own name, or not at all.
- A model's output is advisory until a deterministic rule accepts it.
- No-AI mode is the default (ADR-054): whatever can be written without a model should be.
- The journal never leaves the vault, not even as a caption in a prompt.

## Considered Options (the alternatives it was chosen against)

- Three staged runs, curator first; the curator returns a filing plan the engine checks and turns
  into moves; the engine composes the daily note; journal captures leave the snapshot: chosen,
  because each duty's red discards only its own run and no model writes a path or a byte.
- One staged run for the whole pass: rejected because one red class would discard the other two
  duties' green work, and the owner would get no daily note on a day the curator erred.
- The model writes the move records itself: rejected because a model-written path or name could
  rename or alter a capture, and the executor can only check a move, not intent.
- The model writes the daily note from the template: rejected because its sections are links the
  engine already knows, so a model adds cost, a way to be withheld, and a gap with the route off.
- Pass journal captures to the curator marked "leave": rejected because the caption would still
  travel to the model provider, which the journal rule forbids.

## Decision Outcome

Chosen option: "three staged runs, curator first, a checked filing plan, an engine-composed daily
note, and journal captures out of the snapshot", because it is the only option where every byte
the vault receives is either the owner's own or the engine's, and the model only chooses.

- **The plan.** One destination among the layout's `moves_to`, or `leave`, per snapshot capture id.
  An unknown id, a foreign destination or a capture named twice refuses the plan whole.
- **The moves.** The engine re-reads each source before it moves; a changed hash or a source gone
  from the inbox refuses the run whole.
- **The order.** Curator, then the daily note, then (on a Sunday) the synthesis; one discard does
  not stop the next.

### Consequences

- Good, because the daily note is written every night, with the route absent too.
- Good, because a curator error leaves the inbox as it was, which is the state the owner already
  knows.
- Bad, because a capture the model misfiles moves to a wrong folder. It keeps its name and bytes,
  so the owner can move it back, and it never leaves the layout's destinations.

### Confirmation

SPEC-116's A1 to A6, A8, A14 and A15, and its rows S11601 to S11605 and S11612.

## What would make this wrong

- A curator that must write a note of its own (a summary of the capture): that would be a `create`
  the model writes, and it would need its own rails and its own decision.
- A journal the owner wants synthesised: that is a revival of what the journal rule forbids, and an
  owner decision first.

## More Information

SPEC-116, SPEC-042 R4, SPEC-118 (the captures), ADR-042, ADR-054, and the W6 schematic
`docs/schematics/inbox-capture-and-curation.md`.

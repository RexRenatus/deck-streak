---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The owned scrub rules equal their source over the fields they keep, and are refreshed when the source moves

## Context and Problem Statement

`scripts/scrub-rules/persona-core.json` is DeckStreak's owned copy of a pack's public rule shapes
(SPEC-056 R5), and `scripts/public-scrub.py` reads it, so the copy is behaviour. The box run judges
it against its source over the fields it keeps (R10). At the source the box run now uses, the
`email` rule's regex differs: it passes a systemd instance unit's path and decides every other
address as before. The old copy flags a unit path of a type the scrub's own code does not name.
How does the owned copy stop failing the drift check, and how does it stay equal?

## Decision Drivers

- The box run must read the packs verdict as all green, with no waiver.
- The owned copy is read by the scrub, so a refreshed regex must keep every real address caught.
- A refresh must be small and decided by a public test, not by the private source.

## Considered Options (the alternatives it was chosen against)

- Chosen: the copy's `patterns[10].regex` becomes the source's, byte for byte, because the rule is
  then equal over every kept field and the next source move fails the same check by name.
- (a) An expected-red entry for the drift in the private wiring: rejected, because it is a waiver
  that hides the stale copy and weakens the box run's verdict, which the owner's ruling forbids.
- (b) A DeckStreak-local regex that passes unit paths its own way: rejected, because it drifts from
  the source again at the next move and the drift check fails again.
- (c) Leaving the old source: rejected, because the pull request held for the new source cannot
  land while the box run reads one failure.

## Decision Outcome

Chosen option: "the copy follows the source", because it is the only option that leaves the drift
check unwaived and the copy equal. A public test decides the behaviour through the scrub's entry
point: unit paths pass, real addresses in every context still fail, reserved domains still pass
(SPEC-056 section 9, A21 to A23).

### Consequences

- Good, because the drift check reads equal over every kept field.
- Good, because the change is data: no code arm of the scrub changes and no mutation row is added.
- Bad, because the scrub's own code still names its own unit types, so a type the source adds is
  decided by both until that code is next amended.

### Confirmation

The three tests of A21 to A23, and the box run's drift check (SPEC-056 R10, A14).

## What would make this wrong

- A real address in a context A22 does not list, passed by the new regex: the population grows by
  that context first.
- A source that moves again before this lands: the copy is refreshed again, the same way.

## More Information

SPEC-056 section 9; ADR-069 (the box driver's shape); ADR-056 (the packs stay box-only).

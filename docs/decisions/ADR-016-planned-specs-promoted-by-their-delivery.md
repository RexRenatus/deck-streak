---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Planned SPECs live in docs/specs/planned until the delivery that builds them

## Context and Problem Statement

The architect writes each wave's SPECs before its builders start, but the tdd pack judges every
SPEC in `docs/specs/` for tests that exist and a red-first record, so a SPEC written ahead of its
code would read red on `dev` until the code lands. The packs repository avoids this because a SPEC lands in
the same delivery as its tests.

## Decision Drivers

- The tdd pack's `acceptance-has-a-test` and `red-first-recorded` judge every SPEC directly in `docs/specs/`.
- The gate on `dev` must stay green between deliveries.
- Builders need the SPEC before they start.

## Considered Options (the alternatives it was chosen against)

- Write each planned SPEC in `docs/specs/planned/`; the delivery that builds it moves it into `docs/specs/` in the same pull request as its tests and its red-first record — chosen: the judged population is exactly the delivered SPECs, and the plan is still reviewable in the tree.
- Commit planned SPECs to `docs/specs/` and mark the tdd classes advisory — rejected because advisory must name a stronger gate that holds the class, and none exists here.
- Raise `adopted_from` above the planned numbers — rejected because it would exclude those SPECs from judgement forever.
- Commit failing or ignored tests with the SPEC — rejected because a test that cannot compile against the base is not red for its criterion's reason.

## Decision Outcome

Chosen option. Numbers come from the architect's claims ledger, so a planned SPEC's number is
reserved before it is promoted, and `scripts/tests/test_planned_specs.py` refuses a planned
number that collides with a judged one. A delivery's pull request moves `docs/specs/planned/SPEC-NNN-*.md`
to `docs/specs/`, adds the tests its fence names and `docs/red-first/SPEC-NNN.md`.

### Consequences

- Good, because `dev` stays green and the plan stays visible.
- Bad, because the sdd census (`next spec --across-refs`) does not see planned SPECs; the claims ledger and the planned-SPEC test cover numbering.

### Confirmation

`python3 -m unittest scripts/tests/test_planned_specs.py`; the tdd rows over `docs/specs/`.

## What would make this wrong

- The tdd probe learns a planned status; then this ADR is superseded by that status.

## More Information

The sdd and tdd packs; docs/BUILDER-BRIEF.md.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).

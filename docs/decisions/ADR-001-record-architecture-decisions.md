---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Record architecture decisions as MADR files in docs/decisions

## Context and Problem Statement

DeckStreak is built by many builders over many waves. Each decision's reasons must outlive the
builder who made it, sit next to the code it shapes, and name what it was chosen against, so a
later reader can revisit the trade instead of re-deriving it.

## Decision Drivers

- The sdd pack requires an ADR per real decision, naming at least one rejected alternative and its reason.
- Decisions must be reviewed in the same pull request as the change they explain.

## Considered Options (the alternatives it was chosen against)

- MADR 4.0.0 files in `docs/decisions/`, numbered `ADR-NNN` — chosen: plain Markdown, reviewed with the code, one stable name per decision, and `adr-template.md` keeps the next one cheap.
- A wiki — rejected because it is versioned apart from the code, so it drifts from what it describes.
- No decision log — rejected because a reversed decision would leave no record of why it was made.

## Decision Outcome

Chosen option: MADR 4.0.0 files in `docs/decisions/`, three-digit numbers from the
architect's claims ledger (numbers never collide or move), each with the section
"Considered Options (the alternatives it was chosen against)" that the sdd probe reads, and a
"What would make this wrong" section naming the measurement that would reverse it.

### Consequences

- Good, because every architecturally significant pull request carries its reasoning.
- Bad, because a decision record is one more file; the template keeps it short.

### Confirmation

`python3 scripts/sdd-probe.py --root . check adr-alternatives` and `check adr-linked` run in `scripts/check.sh`.

## What would make this wrong

- A reviewer cannot find why a past decision was made from its ADR alone.

## More Information

The sdd pack (`.packs/skills/packs/sdd/SKILL.md`); greenfield's `adr-log` row.

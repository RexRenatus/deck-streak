---
status: "accepted"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The owner's note conventions are private configuration the kernel loads

## Context and Problem Statement

Several W4 instruments read the owner's note conventions: the Illusion Ledger's direction rules
(#137), the Hanzi Dividend's transfer fields (#140), Dark Fields' special names (#142) and the
Can-Do ladder's field (#92). The predecessor holds them as literals: note-type, template and field
names from the owner's own collection (`direction.py`, `transfer.py`, `cando.py`, predecessor
`27ee2bc`). Those names describe the owner's notes. How does DeckStreak learn them with no personal
literal in the repository?

## Decision Drivers

- CHARTER 11: a personal default is configuration with a neutral example value.
- ADR-002: ingest, curriculum and insights all read these names. The kernel already loads the
  owner's private files (`crates/kernel/src/courses.rs`, ADR-087), and a note convention is the
  owner's configuration, not a collection fact.
- ADR-012: the goldens patch synthetic conventions into the predecessor's modules.
- A rule the predecessor enforces in code (the forbidden direction tokens) stays code.

## Considered Options (the alternatives it was chosen against)

- A private conventions file, schema `deckstreak.conventions.v1`, loaded once by the kernel into `Conventions` — chosen: one typed value every context reads, with no owner name in the repository.
- The predecessor's literals — rejected because they are the owner's note and field names.
- Ingest loads it — rejected because a note convention is the owner's configuration, not a collection fact, and the kernel already loads the owner's private files (`crates/kernel/src/courses.rs`, ADR-087).
- The courses file — rejected because it describes decks and courses (ADR-087), not note types and fields, and a change to one would reroll the other's days.
- Environment variables — rejected because rule lists with patterns and exemptions do not fit one variable without a second parser.
- A table the owner edits — rejected because there is no settings screen until #57.

## Decision Outcome

Chosen option: "A private conventions file loaded once by the kernel", because it keeps the
owner's notes out of the public tree and gives each context one typed value.

- `DECKSTREAK_CONVENTIONS_FILE` names the file: the direction rules and their exempt tokens, the
  transfer fields, and the Can-Do field. An unreadable or malformed file refuses start, naming the
  setting and never a value; unset, the instruments that need it report that they are not
  configured.
- A direction rule whose name contains a forbidden token and is not exempt refuses start; the
  forbidden tokens are code.
- `deploy/config/conventions.example.json` shows the shape with neutral values.

### Consequences

- Good, because no note-type, template or field name of the owner's enters the repository.
- Bad, because the operator keeps a third private file beside the courses and the readings
  taxonomy.

### Confirmation

SPEC-094's criteria on the file's load and refusals; SPEC-096's and SPEC-099's goldens, whose
adapters patch synthetic conventions in.

## What would make this wrong

- The settings screen (#57) lets the owner edit conventions; the file then becomes its source or a
  table, and this record is amended.

## More Information

ADR-002; ADR-012; ADR-087; SPEC-094, which builds the loader; SPEC-096; SPEC-099.

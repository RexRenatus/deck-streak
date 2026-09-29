---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A repeated key in a mutation-row band file is refused at every read, so a clean merge cannot drop rows

## Context and Problem Statement

A band file under `scripts/mutation-rows.d/` holds `{"tables": {...}}` (SPEC-039 R8). Two branches
that each add a table under one key, at different places in the file, merge in git without a
conflict, because their hunks do not touch. The merged file repeats the key. Python's JSON reader
keeps the last value of a repeated key, so the rows under the first are never read, and `ids`,
`census` and `prove` all pass while those rows are gone (#334, measured in SPEC-122 section 1).
Where is a repeated key refused, so that no reader of the population can drop rows silently?

## Decision Drivers

- The rows guard invariants, so a row that vanishes with every check green is the worst failure
  the population can have (ADR-057).
- `scripts/mutation_rows.py` is the one reader (SPEC-039 R8), so one rule there holds for the
  working tree, for a revision read by `git show`, and for `scripts/mutation-verdict.py`, which
  reads through it.
- The refusal must name the file and the key, so the author knows what to merge by hand.
- A refusal must not depend on where the reader runs: the maintainer's machine and CI read the
  same way.

## Considered Options (the alternatives it was chosen against)

- The reader refuses a repeated key, at any depth, in every document the population is made of,
  and names the file and the key: chosen, because every reader already goes through the module, so
  no reader can be left out, and the author is told at once what is wrong.
- Reading the last key, as `json.loads` does today: rejected, because it silently drops the rows
  under the first occurrence, and the merge that produces it raises no conflict.
- Merging the repeated tables into one: rejected, because the result depends on the order the keys
  appear in, and it hides an authoring error that a person should resolve on purpose (two rows
  with one id, or two tables that meant one).
- A lint only in CI: rejected, because every local reader (`ids`, `census`, `prove` on the
  maintainer's machine, `mutation-verdict.py`) would stay wrong, and the rows would already be gone
  from a run before the lint job spoke.
- One file per row: rejected, because it turns every added row into a new file and churns the
  population and its band names, for a defect that one rule in the reader closes.

## Decision Outcome

`mutation_rows.parse_document(name, text)` reads a document with an object-pairs hook that refuses
a key it has already seen in the same object, and turns the refusal into a `PopulationRefused`
naming the file and the key. The header and every fragment are read through it, in the tree and in
a revision. The `retired` verb catches the refusal at the verb as `count` and `ids` do.

### Consequences

- Good, because the merge that used to drop rows silently now fails the next read, on the
  maintainer's machine and in CI alike.
- Good, because the rule sits in the one module every reader uses.
- Bad, because a band file that repeated a key on purpose would now be refused; none does (A4).

### Confirmation

SPEC-122's A1 to A4, and its rows S12201 to S12206, proved by the `mutation-rows` job.

## What would make this wrong

- A band file that must hold one key twice: then the population's shape, not the reader, would
  have to change, and this ADR would be superseded by the decision that changes it.

## More Information

SPEC-122, SPEC-039 R8 to R11, ADR-057, issue #334.

---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The population reader checks each table's and each row's shape where it assembles them, and refuses the first wrong one by its place

## Context and Problem Statement

`scripts/mutation_rows.py` is the one reader of the hand-proved mutation rows (ADR-057 D5,
ADR-122): `load_tree` and `load_revision` parse the header, `scripts/mutation-rows.json`, and each
band file under `scripts/mutation-rows.d/`, and `assemble` joins them into one population that
`rows_of` reads row by row. Measured at the base `bd953ba7` (SPEC-646 section 1), `assemble`
checks the header's keys and a band file's keys and table names, and that a band file's table is
an array, but not the shape of a header table, nor the shape of any row:

- a header table that is a number or `null` raises `TypeError` in `list(rows)`, an object reads as
  an empty table, and a string reads its characters as rows;
- a row that is a number raises `TypeError` in `rows_of`'s `len(row)`, and a string or an object is
  measured by its characters or keys, refused, if at all, with a sentence that names no file;
- in a band file, a row that is not an array is refused only as having no id, naming no shape;
- a row whose target is `null` raises `TypeError` in `row_target`, and a row whose id or find is a
  number is read as a row.

A traceback exits 1, where every other refusal of the population is one named sentence and exit 2.
ADR-482 D6 left "the population reader's tables and rows" for an issue of their own, and #951 is
that issue. How does the reader refuse each such shape by name, with its file and its place,
without changing what a well-formed population reads?

## Decision Drivers

- Every refusal of the population reads as the ones before it: a `PopulationRefused` sentence that
  names the file, exit 2 from every verb that reads the population, and no traceback.
- A refusal names the place of the wrong table or row, so the maintainer can find it.
- No committed row is refused, and a well-formed population reads exactly as at the base.
- The reader stays standard-library Python, with no import added and no existing line changed.
- One test per shape class, red at the base by assertion, and one row per check.

## Decision Outcome, and what each decision was chosen against

### D1. Where: in `assemble`, by one helper

The checks sit in `assemble`, the one place every population passes: `load_tree` and
`load_revision` both return through it, `rows_of` reads only what they return, and
`scripts/mutation-verdict.py` reads through the same two. The header's tables are checked before
the header's ids are held, and each band file's rows before they are read for their ids, by one
helper, `_refuse_misshapen_rows`, called for the header's rows and for every band file's.

- Chosen against: checking in `rows_of`, because the assembled population no longer knows which file a row came from, so its refusal could name no file, and a second check beside the first would make the first's mutants equivalent.
- Chosen against: checking in `parse_document`, because it parses every document, the retired list included, whose shape is its own reader's (ADR-482).

### D2. The shapes and their sentences

A table is an array of rows, a row is an array, and every cell is a string, as every reader of a
row indexes it and reads each cell as text. A header table that is not an array is refused with the
sentence a band file's table already gets, `<file>: <table> is not an array of rows`; a row that
is not an array as `<file>: <table>[<index>] is not an array of cells`; a row holding a cell that
is not a string as `<file>: <table>[<index>] holds a cell that is not a string`. The first wrong
row is named by its index from 0, as SPEC-465 names `retired[<index>]`.

- Chosen against: naming a wrong row by its id, because a wrong row may hold no string id, and its index always exists.
- Chosen against: checking the arity here, because `rows_of` already refuses an arity the header does not declare for the table.
- Chosen against: refusing every wrong row at once, because every other refusal of the reader names the first.

### D3. What stays

A band file's table check and its sentence (`scripts/mutation_rows.py:192-193` at the base), the
refusal of a row with no id (`:195-197`), the header's object and arities checks (`:157-161`),
`rows_of`, `row_target` and `retirement_records` are each unchanged. The base's 3323 committed rows
are all arrays of strings, so no committed file is refused.

### D4. Tests, rows, mutation and formal

Three criteria are red by assertion at the base (SPEC-646 A1 to A3), and two guard what stays: A4
is new, A5 is SPEC-122's existing test. Rows S64600 to S64602 pin each check's removal to its
criterion's test. The three generated `not` mutants of the new lines are killed by every read of a
well-formed fixture. Formal is not required: the three guards precede decisions they leave
unchanged for every population they admit, and `assemble` carries no formal entry at the base
(SPEC-646 section 7).

- Chosen against: a Lean entry for the checks, because ADR-482 D5 gave the retired list's reader one when its cure decided that reader's whole answer, while here three guards precede decisions they leave unchanged, `assemble` carries no entry at the base, and an entry for it would be a delivery of its own that also ports the band and id rules this delivery does not touch.

### D5. Not in this delivery

The header's `arities` object's shape, the `--rows-from` plan's shape (#950), SIGINT in the window
before a killer's group can end (#952) and the verdict script's handling of a refusal are each left
as they are (SPEC-646 section 5).

## Consequences

- Good: no table or row of the wrong shape reaches code that indexes it, and each refusal names its
  file and its place.
- Bad: a header table `{}` that read as empty is now refused; no committed file holds one.

## Confirmation

SPEC-646's A1 to A5 by name, the rows S64600 to S64602 proved KILLED by
`python3 scripts/mutation_rows.py prove --band S64600-S64699`, and the `mutation-python` legs of
the pull request's own run.

## What would make this wrong

- A table whose rows hold a number cell by design: R3 would then have to read the table's layout,
  as `rows_of` does.
- A header that keeps a table as an object on purpose.

## More Information

- SPEC-646, its schematic `docs/schematics/mutation-rows-population-shape.md` and its red-first
  record `docs/red-first/SPEC-646.md`.
- ADR-482 (the retired list's shape, whose D6 left these shapes for #951), ADR-122 (the one
  parser), ADR-057 D5 (the population's shape), SPEC-039 R8 to R11 and SPEC-122.

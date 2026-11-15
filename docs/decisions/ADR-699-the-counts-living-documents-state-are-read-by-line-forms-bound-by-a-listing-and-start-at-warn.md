---
status: "accepted"
---

# ADR-699: the counts living documents state are read by line forms, bound by a listing, and start at warn

Decides SPEC-676. Path:
`docs/decisions/ADR-699-the-counts-living-documents-state-are-read-by-line-forms-bound-by-a-listing-and-start-at-warn.md`.
Amends nothing. Cites SPEC-436 and ADR-450 (D6, the promotion rule a census at warn moves to fail
by). Read at dev `b9a08e602fc10c9f3ecd88a1544f3f595d55cbda`.

## Context and Problem Statement

A document can state a count of something the tree holds, such as five jobs or seven message
catalogs, and no check derives that count again, so it can go stale unseen. Which lines does a
check read, which documents does it leave out, how is a stated count tied to the derivation that
checks it, and how does the check start?

## Decision Drivers

- No document changes in this delivery, and no dependency is added.
- A count inside code, a quote, a table of measured results or a line naming its run id is not a
  claim about today's tree.
- A dated record keeps its counts as history, and leaves only by the rule that makes it one.
- A new check reads its whole population, prints what it examined, refuses an empty population,
  and starts at warn (ADR-450 D6).

## Considered Options (the alternatives it was chosen against)

- D1. Line forms in the census itself: fenced and indented code, front matter, block quotes,
  HTML comments and table rows are told apart by their line shapes, as CommonMark and GFM define
  them, and only prose lines are read — chosen. A Markdown library — lost: it adds a dependency
  to the python stage for six line shapes. Reading table rows as prose — lost: most table cells
  hold measured results, and the choice to read them is its own delivery.
- D2. Records leave by rule, by path: a delivered SPEC, an ADR, a red-first record, a changelog
  fragment, a signed ruling and the released changelog sections, each with the rule at dev that
  makes it append-only — chosen. Leaving whole folders out by name — lost: a planned SPEC, a
  schematic and the fragment guide are living text and would leave unread.
- D3. Binding and listing in the census, keyed on words: a `BOUND` entry names a claim's document
  and words with its derivation, and a `LISTED` entry names a site's key with its reason — chosen.
  Marker comments in the documents — lost: every bound document would change. Keys holding line
  numbers — lost: an unrelated edit above a claim would stale its entry.
- D4. Levels: the census lands at `warn` with today's sites listed from the first push's own CI
  run, a stale entry failing at both levels — chosen. Landing at `fail` — lost: a site a later
  delivery adds would turn every pull request red before it is listed or reworded.

## Decision Outcome

Chosen options: D1, D2, D3 and D4, because together they read every living document, leave out
only what a rule makes history, and tie each stated count to the derivation that checks it
without changing any document.

### Consequences

- Good, because a bound count that drifts from the tree is a `mismatch` site, and a new unbound
  count is printed at warn the first time CI reads it.
- Good, because a listing entry that no longer names a claim fails, so the listing cannot hide a
  reworded or removed count.
- Bad, because the forms read some numbers that count no tree thing; each is listed with its
  reason until its wording or a narrower form is ruled.

### Confirmation

SPEC-676 R1 to R11, by the eight tests of `scripts/tests/test_doc_counts.py`, and the rows band
`scripts/mutation-rows.d/S67600-S67699.json` on its arms.

## More Information

#1395, its epic #913, SPEC-436 and ADR-450. Moving `LEVEL` to `fail` is its own delivery under
ADR-450 D6.

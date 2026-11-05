---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-630: the harness report reads the planted suite bundle like every other test bundle

Delivered with SPEC-608.

## Context and Problem Statement

The `harness` job runs the card view's planted suite on the iPhone and then the iPad and writes its result bundle to
`$RESULTS/card-probe.xcresult`. The report step reads five other bundles through `cases_in(...)` and lists each case
with its result on each simulator, but it never reads the planted suite's bundle. When a planted card test failed on
one simulator (jobs 115343048693 and 115407471652), the report held no row for it, so the job did not say which
simulator failed. How should the report name the simulator a planted card test failed on?

## Decision Drivers

- One reading path for every test bundle, so a planted case is listed the way every other case is.
- The two report bodies of the split stay one text (ADR-612), so the sibling job must name the job that measures
  the new rows instead of failing on them.
- No change to the planted suite, its waits or its assertions.

## Considered Options (the alternatives it was chosen against)

1. **Read the bundle through `cases_in("card-probe")` and list it under `Card probe` (chosen).** It reuses the
   reader, the details files and `per_simulator`, and one owners entry names `harness` for the sibling.
2. **Parse the planted suite's console for failing tests.** It reads text no schema holds, and the console names a
   destination only by its order.
3. **Add a separate report step for the planted suite.** It splits one table into two and repeats the reader.

## Decision Outcome

Option 1. The report body gains `card_probe_cases = cases_in("card-probe")` beside the other readers,
`("Card probe", card_probe_cases)` in the table loop after the review actions, and `"card-probe": "harness"` in the
owners map, applied to both bodies.

### Consequences

- Good: each planted card test has a row naming each simulator with its result, and its details are written beside
  the report.
- Good: the sibling's report names `harness` for those rows, as it does for the Debug rows.
- Neutral: two more files per planted case sit in the uploaded report folder.

### Confirmation

SPEC-608 A1 holds the three lines in both bodies; A2 runs each job's live report step over a planted `xcrun` and
reads the rows it writes.

## More Information

SPEC-591 R6 (one report body), SPEC-349 R10 and R11 (the planted suite and its load wait), #841 and #843.

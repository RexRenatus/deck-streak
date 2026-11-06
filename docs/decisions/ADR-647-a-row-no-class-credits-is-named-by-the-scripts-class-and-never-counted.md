---
status: "accepted"
---

# ADR-647: a row no class credits is named by the scripts class and never counted

Decides SPEC-625. Schematic: `docs/schematics/mutation-rows-left-to-the-leg-report.md`.

## Context and Problem Statement

The mutation verdict judges each selected row in `judge_rows` and credits a KILLED row to the
judged class's examined count only when the row's changed line is in a path of that class. A row
whose changed line is in a path of no judged class, such as the Python mutation map
`scripts/mutation-python.json`, or a row that covers no changed line, is judged KILLED and then
counted and named by no class. In job 124459129894 the scripts class read `examined 71: generated
59, rows 12` and `verdict: ok` while the rows leg had killed 13 rows: the 13th, S42308, targets the
map, and only the leg report showed it.

## Decision Drivers

- No class's examined count may grow from a row of no file of that class: a class that examined
  nothing must still read VOID.
- The verdict log alone should name every row the leg killed that no class line counts.
- The change must leave every definition the formal entries cover byte for byte.

## Considered Options (the alternatives each was chosen against)

D1, the rows no class credits. Chosen against:

- Chosen: the scripts class names each such row on its own line, with the path of its changed line
  or the words that it covers none, under one count line, and adds none of them to any count,
  because naming a row weakens nothing and leaves every examined count as it was.
- Crediting them to the scripts count: rejected because a scripts class that examined no script
  would then read ok on a row of no script, which lifts its zero-examined VOID.
- Classifying the map as `scripts`: rejected because the plan would then read a JSON file as a
  script and hand it to the Python mutant runner, which parses only Python.
- Naming them in every class: rejected because the rust, oracle and scripts classes each judge
  every selected row, so each row would be named three times.

## Decision Outcome

Chosen: D1's first option. After its loop, `judge_rows` calls one new function, which returns at
once unless the class is `scripts`, collects in plan order each selected row whose rows-report
entry reads KILLED and whose overlap path is absent or classifies outside `rust`, `oracle` and
`scripts`, and says the count line and one line per row through `Verdict.say`. It returns nothing,
so `judge_rows` returns the same count as before.

### Consequences

- Good: the verdict log names every killed row the class lines leave out, with its reason.
- Good: no count, verdict, FAIL or VOID changes, and a row the leg did not kill still fails by name.
- Neutral: a pull request that changes `scripts/mutation-verdict.py` selects every row on it, and
  most of them cover no changed line, so the scripts class may name nearly all of them.

### Confirmation

SPEC-625's A1 to A5 hold the lines and the counts, and the rows S62500 to S62506 prove each test
observes its behaviour.

## What would make this wrong

- A class that starts to credit rows on another path, such as a web job that proves rows: the set
  of crediting classes would then need that class too, or its rows would be named left.
- A plan that records an overlap for a row on an unchanged line: the no-line reason would then
  never print, and A2 reads red.

## More Information

SPEC-087 R11 (the rows the diff selects on a class's changed lines) and SPEC-039 R10 (every
selected row is proved KILLED).

---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A docstring-only script change is named by its syntax tree, never read as VOID

## Context and Problem Statement

The mutation plan reads a guard script (`scripts/*.py`, the class `scripts`) with Python's own
tokenizer and counts every changed line that is neither blank nor a comment as a code line
(SPEC-039 R4). A docstring is a string token, so a change to docstrings alone makes the class
apply. The Python runner never mutates a docstring: its lister skips the first statement of a
module, class, function or async function body when that statement is a string constant. So the
class applies, examines nothing, and its verdict reads VOID (#485). Measured at `56ce963`: a
fixture script whose module and function docstrings were reworded read `scripts applies: 2
production code line(s) in 1 file(s)`, the runner `listed 0`, and `judge --class scripts` printed
`VOID the scripts class applies and nothing was examined` and exited 3. Which rule names that
change by its own name without narrowing what the class examines for a change that alters
behaviour?

## Decision Drivers

- No narrowing: every change that alters a script's behaviour keeps the class applying, and the
  runner lists every mutant it listed before.
- A case named once, in one stable spelling, that the plan and the verdict both print.
- Fail closed: whatever the rule cannot read leaves the class applying exactly as R4 makes it.
- One definition of a docstring, the one the runner already uses to skip it.

## Considered Options (the alternatives it was chosen against)

- (a) Read every string expression as a comment: rejected, because it narrows: a string is code
  wherever its value is used (a message, a path, a pattern, `NAME = "a"` changed to `"b"`), the
  runner mutates such a string, and a change to it would stop the class applying.
- (b) Make the runner emit a docstring mutant: rejected, because such a mutant examines nothing
  real: no test asserts a docstring, so it would be killed by no test, or excused by a record that
  says only that docstrings are text.
- (c) An allow-list of files whose docstring changes are excused: rejected, because it names no
  rule, so it cannot say why a file is on it, and a code change to a listed file would be excused
  with the docstring beside it.
- Chosen, because the syntax tree is what Python executes, so equal trees once docstrings are set
  aside mean the change altered no code, and the docstring it sets aside is exactly the statement
  the runner never mutates: compare, per changed script, the trees at the merge-base and at the
  head, positions excluded, with only the first bare string of a module, class, function or async
  function body set aside, and name the case `docstring-only` when every changed script's trees
  are equal.

## Decision Outcome

Chosen option: the syntax tree decides. When R4 makes the `scripts` class apply, the plan reads
each changed `scripts/*.py` at the diff's merge-base and at its head, parses each as UTF-8 with
`ast.parse`, sets aside the first statement of every module, class, function and async function
body when that statement is an expression of a `str` constant, and compares `ast.dump` of the two
trees, which excludes positions. When every changed script's trees are equal, the class does not
apply, its case is `not-applicable: docstring-only: ` followed by each file whose change it set
aside, and the verdict names each such file on a line of its own and passes. A script added or
deleted (so renamed, since the plan reads the diff with `--no-renames`), a script that does not
parse or is not UTF-8 at either side, a diff with other than one merge-base, and any other tree
difference leave the class applying as R4 makes it.

### Consequences

- Good, because a pull request that only corrects docstrings, such as #455's, reads a named
  `not-applicable` case instead of VOID.
- Good, because nothing examined is lost: the rule sets aside only lines the runner never mutates,
  and a code change beside a docstring change keeps the class applying with every mutant of its
  code line listed.
- Good, because every arm the rule cannot read fails toward the class applying, so an error in
  reading a script can make the class apply, never pass.
- Bad, because the start of six guard scripts' module docstrings is their `--help` description, so
  a docstring-only change alters that text and reads `not-applicable`. The runner never mutated it,
  so no mutant examined it before either.
- Bad, because the rule is the tree, so a change that re-lays code without changing its tree reads
  the same case, by the same name. Its behaviour is the base's; the weekly battery still sweeps
  every listed file whole (SPEC-087 R14).
- Bad, because a script that is not UTF-8 on a line the plan reads as text still stops the plan, as
  it did before this decision; the rule changes no refusal it did not make.

### Confirmation

SPEC-039 A61 to A64 (`scripts/tests/test_mutation_verdict.py`): a docstring-only change named and
a code change beside it examined, end to end in CI's order; a population of script edits with the
named members' set-aside lines checked against the runner's own listing, and a planted narrowing
caught; the definition's edges; and the module docstring's PLAN paragraph held to `ci.yml` and the
plan's outputs.

## What would make this wrong

- A Python version whose `ast.dump` differs between two equal programs: the class would apply, the
  fail-closed direction.
- A docstring that a guard reads as data: a change to it would read `not-applicable`. The runner
  never mutated such a docstring either, so a test of that text is the remedy, not the class.
- The runner starting to mutate a docstring: the rule would then set aside lines that hold mutants,
  and A62's check of each named member against the runner's listing would fail.

## More Information

SPEC-039 sections 27 to 29 and R4, SPEC-087 R1 and R14, issues #485 and #455, ADR-057, ADR-073.

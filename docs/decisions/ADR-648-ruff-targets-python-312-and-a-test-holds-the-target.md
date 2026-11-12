---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-648: ruff targets Python 3.12 and a test holds the target

Decides SPEC-626 (R1 to R4).

## Context and Problem Statement

ruff reads this tree as Python 3.11 code: `ruff.toml` sets `target-version = "py311"`. The tests
need Python 3.12, since the test-module census reads syntax-tree classes that 3.12 added, and the
jobs of the `ci` workflow run the runner's own Python 3.12 (the claim covers that workflow's jobs
alone). A target older than the running Python can misjudge syntax that Python accepts: ruff
reports syntax its target lacks, and its version-gated rules and its formatter read the target.
Which Python does ruff read the tree as, where is that set, and what holds it there?

## Decision Drivers

- ruff reads the code as the Python that runs it.
- The target is reviewable where it is set.
- A change of target is a decision, never a drift.

## Considered Options (the alternatives it was chosen against)

- Chosen: (a) `target-version = "py312"` in `ruff.toml`, because that is where ruff reads its
  target, and a test that parses the file and one mutation row hold it there.
- (b) A `requires-python` in a new `pyproject.toml`: rejected because the tree holds no Python
  package, and a new manifest adds a packaging surface for one value.
- (c) Leave py311: rejected because the target and the running Python then disagree, which is the
  finding this decides.
- (d) Pin the Python of CI with a setup step: rejected because it is a workflow change for a value
  the runner already provides, and it moves CI, not ruff.

## Decision Outcome

Chosen: (a). `ruff.toml` line 9 reads `target-version = "py312"`, and no other byte of the file
changes. `scripts/tests/test_ruff_target.py` parses the committed file with `tomllib` and asserts
that its top-level `target-version` is `py312`, and row S62600 sets the line back to py311 and is
killed by that test. The new target raises no finding under the tree's settings and leaves ruff's
findings under every rule, and its formatting of every file it examines, as they were.

### Consequences

- Good, because ruff and the running Python agree, so ruff reads the tree as the code that runs.
- Good, because the test reds on a drift of the target, so a change of target is reviewed.
- Bad, because the gate's Python floor and the README's prerequisite line still name 3.11, which
  SPEC-626 section 5 leaves to a separate change.

### Confirmation

SPEC-626 A1, red at the commit that adds the test while `ruff.toml` still targets py311 and green
at the commit that changes the line; row S62600; and the ruff measurements of SPEC-626 section 1,
run again at the delivery's head.

## More Information

- SPEC-626 (the measurements, the requirements and the file manifest).
- What would make this wrong: CI moving to a Python other than 3.12, which is then a change of
  target of its own, with the test and the row moving with it.

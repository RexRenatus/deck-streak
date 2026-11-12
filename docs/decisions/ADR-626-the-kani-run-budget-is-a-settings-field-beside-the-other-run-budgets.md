---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-626: the Kani run budget is a settings field beside the other run budgets

Decides SPEC-605 (R1, R2).

## Context and Problem Statement

The tree holds eight Kani harnesses, four in `crates/media/src/kani_proofs.rs` and four in
`crates/sync/src/kani_proofs.rs`. The formal checker, outside this repository, reads
`budgets.kani_seconds` as the most one Kani run, a harness or a witness, may take, and refuses a
selected Kani entry as a settings error when the settings file names no such field. The file states
budgets for a TLA+ run, a Lean run and one entry, and none for a Kani run. Where is the Kani run
budget stated, with what value, and how does the settings test read it?

## Decision Drivers

- A run budget is reviewable in the pull request that sets it, beside the other run budgets.
- No value is claimed larger than a measurement supports: a budget that hides a slow harness reads
  clean where it should read as a timeout.
- The settings test reads each field the way the formal checker reads it, and still holds the
  committed file to every field it states.

## Considered Options (the alternatives it was chosen against)

- Chosen: (a) `budgets.kani_seconds` with a value of 300, because it equals `entry_seconds`, the
  budget all runs of one entry share unless the entry names its own, so one Kani run never claims
  more than its entry's whole default.
- (b) A larger value, such as that of `lean_seconds`: rejected because no Kani run of this tree is
  measured whole, so the value would rest on no measurement and would hide a slow harness.
- (c) Leave the field out: rejected because the formal checker then refuses every selected Kani entry
  of this tree as a settings error, and no harness is ever checked.

## Decision Outcome

Chosen: (a), in two parts.

- **D1.** The file states `budgets.kani_seconds` with a value of 300, on its own line between
  `lean_seconds` and `entry_seconds`. The value equals `entry_seconds`, so one Kani run never claims
  more than its entry's whole default, and no larger value is claimed without a measured whole run.
- **D2.** The settings test's reader takes the field as an optional positive integer, as the formal
  checker reads it, and the committed-file census (SPEC-605 A1) requires it present, since that census
  admits only `entry_seconds` and `entries` as absent.

A budget is a settings value, not one of the formal checker's weakening classes (those name `k`, an
added axiom and the trust root), so a later measured raise is an ordinary change and needs no signed
ruling.

### Consequences

- Good: a selected Kani entry of this tree is no longer refused for a missing budget.
- Good: a Kani run past the budget reads as a timeout, never as clean, so a slow harness is seen.
- Bad: a harness that needs more than the value reads as a timeout until a change measures a whole run
  and raises it.
- Neutral: no per-entry budget is added for a Kani entry; the runs of each Kani entry share the
  default `entry_seconds`.

### Confirmation

SPEC-605 A1, red at the commit that adds the field to the test's tables only, with rows S60500 and
S60501; A2 with row S60502.

## More Information

- SPEC-295 and ADR-295 (the settings file and its test), SPEC-605 R1 and R2.
- What would make this wrong: a measured whole Kani run of one of this tree's harnesses that needs
  more than the value, which is then a raise of its own; or a formal checker that starts to read the
  field as required, which would make the reader's optional reading a drift for the test to catch.

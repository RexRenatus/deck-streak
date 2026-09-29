---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner, through the maintainer), the DeckStreak architect"
---

# The scan carries an expected red as a pack's entry does

## Context and Problem Statement

SPEC-030 R12 and ADR-030 let a pack's wiring entry name a row expected red and the open issue that
builds its subject, so a known gap is deferred by name and comes off when its issue closes. The
proxy client scan's entry takes only `pending` and `note`. Delivery #29 makes the scan examine a
settings document, one row reads red, and the maintainer ruled that row substituted by ADR-038's
shape and deferred to #341 until the scan learns the credential socket. How does the box run defer
one row of the scan without hiding the others and without outliving its issue?

## Decision Drivers

- A deferral names its row and its issue, and stops holding the moment either changes.
- Every other row of the scan is still judged.
- The scan is pinned from its upstream and is not edited here.
- ADR-038's rule stands: no secret-manager read in the application.
- The scan's summary keeps the shape its parser reads.

## Considered Options (the alternatives it was chosen against)

- The scan's entry admits `expected_red` as a pack's does — chosen: it carries a decided semantics (SPEC-030 R12) to the one entry that lacked it, defers exactly one row, and goes stale by rule when the issue closes or the row stops reading red.
- Keep the row unexpected and hold #29 — rejected because it blocks a delivery that follows ADR-038 on a reading gap in a pinned scanner, and the maintainer ruled the row substituted, not waived.
- `pending` with a settings document — rejected because the runner already marks that stale the moment the scan examines a row, and `pending` would hide every row of the scan, not the one.
- A local fork of the scan with the row's check changed — rejected because the scan is pinned from its upstream and a fork drifts from it, and it changes the judge to fit one deferral.
- Move the runner to a secret-manager read inside the application — rejected because ADR-038 rejects a secret-manager read in the application, and the row would then read green for a shape the decision forbids.

## Decision Outcome

Chosen option: the scan's entry admits `expected_red`.

- The entry is a mapping of a row id to `#NNN`. It is refused beside `pending`, when empty, and when
  a value is no issue. The scan takes no other key beyond `pending` and `note`; the helper's entry
  takes no `expected_red`.
- A red row the mapping names is expected and counted on the scan's line. Any other red row is
  unexpected. The red total must equal the expected count.
- The expectation is stale when its issue is closed or its row does not read RED.

### Consequences

- Good, because #29 can land on ADR-038's shape and the run still fails on any new red row.
- Good, because the deferral ends by rule, with #341, and needs no one to remember it.
- Bad, because the maintainer's private wiring must name the row after this merges, by hand.

### Confirmation

`test_box_scan_expected.py` (SPEC-123 A1 to A8) drives the runner, and the rows S12301 to S12311
prove each invariant killed.

## More Information

SPEC-123, SPEC-030 R10 to R14, ADR-030, ADR-038; #29, #341, #342.

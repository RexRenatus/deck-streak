---
status: "accepted"
---

# ADR-700: a quarantine listing binds each flaky test to its runs, and a marker census starts at warn

Decides SPEC-677.

## Context

A test that passes and fails on one tree hides a defect or a race, and a skip that no one records
turns a test off unseen. ADR-450 D6 gives the promotion rule a new census follows.

## Decision

D1. One listing, `scripts/tests/quarantine.json`, holds the quarantined tests and today's sites.
D2. An entry is bound to its record by the runs it names: two or more on one head that disagreed.
D3. A marker is bound to the listing by file, stack and form, with its count, so a moved line keeps
its entry.
D4. A platform condition and a named child run are admitted; nothing else is (default-deny).
D5. The level starts at warn; promotion and demotion follow ADR-450 D6.

## What it was chosen AGAINST

- A retry setting that hides a flake: rejected because a retried pass reads green and the record is lost.
- Deleting or skipping a flaky test: rejected because it removes a test, which narrows a gate.
- Keying a site by line: rejected because any edit above the marker would stale the listing.

## Consequences

- Good: a flake is kept running and carries its runs; every skip is listed with a reason.
- Bad: at warn an unlisted site passes; it is printed in the `hygiene` job's output.

## What would make this wrong

- If ADR-450 D6's live precision falls below 0.8 with at least five hits, D6 proposes a demotion.

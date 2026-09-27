---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Ingest: sync a private collection copy with Anki's own Rust engine, measured before it is final

## Context and Problem Statement

The predecessor syncs a private copy of the collection from the owner's Anki sync
server every 15 minutes with Anki's Python package, then reads the copy read-only with SQLite. It
also asks Anki's scheduler for today's queued new cards (the readings' day set) and writes back
only for the skip day. Anki's Python package is itself a binding over Anki's Rust engine
(`rslib`), which is licensed AGPL-3.0-or-later, the same licence as DeckStreak.

## Decision Drivers

- The owner's instruction: port to Rust, and measure before choosing.
- The day set must be the scheduler's own answer, which only Anki's engine gives.
- The VM has 1.9 GiB of RAM and never compiles; the binary must fit a memory budget.

## Considered Options (the alternatives it was chosen against)

- Anki's `rslib` as a pinned git dependency of `ingest` — proposed: the same engine the predecessor's Python package wraps, native sync, the scheduler's queue and `set_due_date`, one language.
- A native client of the sync protocol — rejected because reimplementing incremental sync is the unmaintainable option the predecessor's own ADR-002 rejected, and a full download every cycle would move the whole collection every 15 minutes.
- A minimal Python sidecar running the predecessor's proven sync — kept as the fallback: proven today, but it keeps Python on the VM and a second language in the port; chosen only if the measurement below fails.
- AnkiConnect — rejected because it needs a desktop Anki running.

## Decision Outcome

Proposed option: `ingest` depends on `rslib` from Anki's repository at the tag matching the
collection's version, behind an `AnkiEngine` port so the rest of the workspace sees only its own
types. The first W0 ingest delivery is a measured spike that makes this ADR final or selects the
sidecar: it builds the dependency in CI and records the build time and the release binary's size,
syncs a synthetic collection against a local sync server, and records resident memory while
opening a large synthetic collection and resolving the new-card queue. The decision is accepted
if resident memory stays inside the ingest budget in `deploy/host-budget.json` and the build fits
the CI time budget; otherwise the sidecar is chosen and this ADR is superseded. Reads stay
read-only SQLite over the copy (`mode=ro`), bounded to the 400-day window.

### Consequences

- Good, because the day set and the skip day use the scheduler's own code.
- Bad, because `rslib` is not a published crate: it is pinned by revision, and an Anki upgrade is a deliberate delivery.

### Confirmation

The W0 ingest spike's measurements, recorded in its SPEC's problem section and in this ADR's status change.

## What would make this wrong

- The spike's resident memory or build time exceeds its budget.
- The sync server's version stops matching a buildable engine tag.

## More Information

The predecessor's ADR-002 (sync via the Anki package, read via stdlib SQLite); the vm-survey's capacity figures; ADR-008.

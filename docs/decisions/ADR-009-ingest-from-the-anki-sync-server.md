---
status: accepted
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

- Anki's `rslib` as a pinned git dependency of `ingest` — chosen: the same engine the predecessor's Python package wraps, native sync, the scheduler's queue and `set_due_date`, one language.
- A native client of the sync protocol — rejected because reimplementing incremental sync is the unmaintainable option the predecessor's own ADR-002 rejected, and a full download every cycle would move the whole collection on every sync.
- A minimal Python sidecar running the predecessor's proven sync — kept as the fallback: proven today, but it keeps Python on the VM and a second language in the port; chosen only if the measurement below fails.
- AnkiConnect — rejected because it needs a desktop Anki running.

## Decision Outcome

Chosen option: `ingest` depends on `rslib` from Anki's repository at the tag matching the
collection's version, behind an `AnkiEngine` port so the rest of the workspace sees only its own
types. The first W0 ingest delivery was a measured spike that would make this ADR final or select
the sidecar: it built the dependency in CI and recorded the build time and the release binary's
size, synced a synthetic collection against a local sync server, and recorded resident memory
while opening a large synthetic collection and resolving the new-card queue, and during a full
download. ADR-022 fixed every budget before the measurement; every one held (Confirmation), so the
engine is chosen and the sidecar stays unbuilt. Reads stay read-only SQLite over the copy
(`mode=ro`), bounded to the 400-day window.

### Consequences

- Good, because the day set and the skip day use the scheduler's own code.
- Bad, because `rslib` is not a published crate: it is pinned by revision, and an Anki upgrade is a deliberate delivery.

### Confirmation

SPEC-022's spike measured the engine at tag `26.05` with ADR-022's protocol, in
`engine-measure.yml` run 36357990387 at 0d8c102, on a GitHub-hosted `ubuntu-24.04` runner with 4
CPUs: a cold build with no cache restored, the stripped `engine_probe`, the budget tests in the
release profile that build compiled, and `cargo deny check licenses`.

| measure | budget | measured | verdict |
|---|---|---|---|
| cold build | at most 20 minutes | 4.7 minutes | pass |
| binary size | at most 100 MiB | 20.6 MiB | pass |
| open and queue | at most 256 MiB | 29.4 MiB | pass |
| full download | at most 256 MiB | 234.0 MiB | pass |
| incremental sync | at most 60 seconds | 0.12 seconds | pass |
| licences | pass | pass | pass |

The full download is the tightest: the engine holds the whole downloaded collection in memory
before it writes it beside the copy, so its peak grows with the collection. The gate re-asserts
every memory and time budget on every run, in its test profile, so a regression past a budget is
a red gate rather than a surprise on the host.

## What would make this wrong

- The spike's resident memory or build time exceeds its budget.
- The sync server's version stops matching a buildable engine tag.

## More Information

The predecessor's ADR-002 (sync via the Anki package, read via stdlib SQLite); the vm-survey's capacity figures; ADR-008.

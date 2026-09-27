---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The ingest engine spike runs a fixed protocol against budgets fixed before it measures

## Context and Problem Statement

ADR-009 proposes Anki's own Rust engine for ingest and names a Python sidecar as the fallback, and
leaves the choice to a W0 spike: build time and binary size in CI, resident memory while opening a
large collection and resolving its new-card queue, judged against "the ingest budget" and "the CI
time budget". Neither budget has a number, and no synthetic collection is defined. A measurement
whose pass mark is set after the numbers are known decides nothing. What exactly is measured, on
what, and against which numbers?

## Decision Drivers

- The host: two vCPU and 1.9 GiB of RAM shared with the predecessor and a co-hosted stack; the
  predecessor's own unit is capped far below the host, and DeckStreak's share is SPEC-032's host
  budget.
- The host never compiles (CHARTER 3); CI builds on GitHub-hosted runners whose gate job has a
  60-minute timeout (SPEC-002's `ci.yml`).
- The disk, not memory, is the host's binding limit (ADR-010), and releases are kept side by side.
- The measurement must be repeatable by anyone from the public repository: synthetic data only.

## Considered Options (the alternatives it was chosen against)

- Fix the protocol and the budgets now, measure once in the delivery, and let the numbers select the engine — chosen: the pass mark cannot bend toward the measurement, and a failure selects the fallback ADR-009 already names.
- Measure first and set the budgets from what was measured — rejected because a budget read off the measurement accepts whatever was measured.
- Accept the engine if it builds — rejected because memory, not compilation, is what the host cannot give back.
- Measure on the host — rejected because the host never compiles, is shared and live, and the synthetic collection would sit on its constrained disk; the host's own view arrives with the memory watch in W2.
- Measure against a copy of the owner's collection — rejected because the repository is public and the measurement must be reproducible by anyone; the synthetic collection is sized to exceed a large personal collection.

## Decision Outcome

Chosen option: this protocol and these budgets.

The synthetic collection, built by `crates/ingest/tests/support/synthetic.rs` from a fixed seed with
bulk inserts into a collection the engine created:

| parameter | value |
|---|---|
| cards | 250,000, one per note, two text fields of 200 characters each |
| decks | 20, under two roots (one law, one language) |
| reviews | 200,000 study reviews spread over the last 400 days |
| new cards per day | 20 per deck in the deck configuration |

The budgets:

| measure | how | budget |
|---|---|---|
| cold build | `engine-measure.yml` on `ubuntu-24.04`, no cache restored, `cargo build --release --locked --example engine_probe -p deck-streak-ingest`, including any tool the engine's build needs | at most 20 minutes |
| binary size | the stripped `engine_probe` | at most 100 MiB |
| open and queue | peak `VmHWM` of the `engine_budget` test that opens the collection and resolves today's new-card queue | at most 256 MiB |
| full download | peak `VmHWM` of the test that full-downloads the collection from a local sync server | at most 256 MiB |
| incremental sync | wall time to pull 100 new reviews from a local sync server | at most 60 seconds |
| licences | `cargo deny check licenses` with the engine | pass; a licence added to the allow list must be compatible with AGPL-3.0-or-later |

Why these numbers: the job unit that runs a sync gets a `MemoryHigh` of 320 MiB in SPEC-032's host
budget, so 256 MiB leaves a fifth of headroom below throttling; 100 MiB per binary keeps two
releases on disk well inside the host's free space; 20 minutes of engine build keeps the whole gate
job under 40 of its 60 minutes.

The engine is admitted as a git dependency of `ingest` pinned to a release tag, with `deny.toml`'s
`allow-git` naming Anki's repository only. If any budget fails, the delivery records the numbers,
ADR-009 is superseded by a new ADR choosing the sidecar, and SPEC-022 is amended before its sync
criteria are built.

### Consequences

- Good, because the choice is made by numbers anyone can reproduce.
- Good, because the same synthetic collection later sizes the readings' day-set tests.
- Bad, because the budget tests add about a minute to the gate; they are the price of a memory
  budget that cannot silently erode.

### Confirmation

SPEC-022's A1 to A3 and `engine-measure.yml`'s report; the numbers are written into ADR-009's
Confirmation section, and ADR-009's status changes with them.

## What would make this wrong

- The host is resized or the predecessor retires, changing the memory the job unit can have (then
  SPEC-032's budget and these numbers move together, by a new ADR).
- GitHub's hosted runners change size, making the build budget measure a different machine.
- Anki's engine grows past a budget in a later release; the pinned tag holds until an upgrade
  delivery re-runs this protocol.

## More Information

ADR-009; ADR-010; ADR-018; SPEC-022; SPEC-032; the vm-survey's capacity figures (private input).

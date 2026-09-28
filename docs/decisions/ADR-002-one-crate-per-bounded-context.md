---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# One Rust crate per bounded context, and the context map is binding

## Context and Problem Statement

DeckStreak ports 131 features of its predecessor, which had grown into one Python package where
any module could import any other. The new backend must keep contexts apart by construction, so
that parallel builders cannot tangle them and a boundary violation is caught before review.

## Decision Drivers

- The ddd pack: a dependency graph equal to the map in both directions, checked by a probe.
- Many builders working at once need crates that rarely conflict.
- Rust's compiler can refuse an undeclared dependency for free.

## Considered Options (the alternatives it was chosen against)

- One crate per bounded context, the crate graph equal to docs/CONTEXT-MAP.md — chosen: an undeclared edge is a compile error, the ddd probe holds manifests and map equal, and a builder edits inside one crate.
- One crate with a module per context — rejected because Rust lets any module import any other, so the boundary would be a review comment, not a compile error.
- A few large crates (domain, adapters, app) — rejected because they give no boundary between the contexts that matter (economy and progression, readings and habits) and every builder would edit the same crate.
- Services per context (microservices) — rejected because one small host within a stated memory budget cannot afford twenty processes, and the domain has no independent scaling need.

## Decision Outcome

Chosen option: one crate per context under `crates/`, packaged `deck-streak-<context>`,
arranged in three layers: the shared `kernel`; domain contexts that depend only on the kernel
and on `ingest` where they read Anki data; and `coordination` (use cases and scheduled jobs across
contexts), the `api` and `bot` adapters, and the `daemon` composition root. Cross-context traits
are joined in `crates/daemon/src/wiring.rs` behind newtypes. The map's fence is the one record of
every edge.

### Consequences

- Good, because a new cross-context dependency is visible in a manifest diff and needs an ADR.
- Good, because `coordination` is the one place a use case crosses contexts, so the bot and the Mini App share code paths.
- Bad, because 24 crates add manifest boilerplate; the workspace inherits every shared key to keep it small.
- Bad, because `coordination` is wide; it holds no rules, only order, which review must keep true.

### Confirmation

`python3 scripts/ddd-probe.py --root . check context-map-parses` and `check declared-edges-match-imports`, and `cargo test -p deck-streak-daemon --test workspace`.

## What would make this wrong

- A use case that genuinely needs a domain context to call another directly appears in more than one wave; that is the signal to add an upstream edge by ADR rather than route through coordination.
- Build times on CI exceed the budget because of crate count (measure `cargo build --timings`).

## More Information

docs/CONTEXT-MAP.md; the ddd pack; phoenix-v2's CONTEXT-MAP as the reference implementation.

Amendment (2026-09-28): one passage stating the host's size, in a considered option, was redacted
under the public-prose rule (ADR-059).

---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# One golden reader, compiled into each proving crate's tests by path, over a registry kept per SPEC

## Context and Problem Statement

ADR-012 makes the predecessor's own functions the oracle: a generator on the owner's private
checkout writes goldens, and the Rust tests read them. SPEC-002 left the generator with one
registry dict inside `generate.py` and no Rust reader. From W0 on, every context that ports a rule
(kernel, ingest, coordination, then the whole game in W3) must read goldens, the context map
(ADR-002) admits no crate it does not list, and six builders register functions at once. Where
does the reader live, and how is the registry shaped so that parallel deliveries neither conflict
nor stale each other's goldens?

## Decision Drivers

- The crate graph is the context map: a new workspace member needs a map line and an ADR, and
  every context that proves a rule would depend on it.
- The kernel is shared code for two contexts' production needs, not a home for test support.
- One source of truth for how a golden is read; copies drift.
- A golden must read stale when the code that produced it changed, and only then.
- CHARTER 8: the game math is proved against the predecessor, never re-derived or retyped.

## Considered Options (the alternatives it was chosen against)

- One file, `tools/parity-oracle/golden.rs`, included into each proving crate's integration tests with `#[path]`; `serde` and `serde_json` as dev-dependencies only — chosen: no crate is added, no production dependency changes, and every crate compiles the same reader, so a change is tested everywhere at once.
- A dev-dependency-only crate (for example `deck-streak-oracle`) — rejected because it is a workspace member the context map must list, the ddd probe would judge its edges, and it would become a crate every proving context names in its manifest for no runtime reason.
- A `test-util` feature of the kernel — rejected because it puts test support and a JSON parser on the shared kernel's surface, the thing CONTEXT-MAP calls "a context hiding in shared code".
- A copy of the reader in each crate's `tests/support/` — rejected because copies drift, and a drifted reader is a golden read two ways.
- Keep one `FUNCTIONS` dict in `generate.py` — rejected because six parallel deliveries would conflict on one literal, and one edit would change the digest every golden records, staling all of them together.

## Decision Outcome

Chosen option: the path-included reader, and a registry of one module per SPEC.

- `tools/parity-oracle/golden.rs` parses `phx.parity-golden.v1`, refuses a wrong schema, an empty
  case list or a missing field, and prints the examined count. A crate uses it from its
  `tests/*.rs` with `#[path = "../../../tools/parity-oracle/golden.rs"] mod golden;`.
- `tools/parity-oracle/registry/spec_NNN.py` holds one SPEC's registrations. A golden records
  `generator_sha256` and `registry_sha256`; either changing makes that golden stale, and editing
  one module stales only its own goldens.
- A registration is a `function`, an `adapter` (glue in the registry module that builds non-JSON
  arguments or patches a sleep, then CALLS the predecessor's function; never a re-implementation)
  or `constants` (attributes read from the predecessor's modules).
- No golden holds a calendar date: instants are epoch milliseconds, days are epoch day numbers.
  That keeps every golden clear of the charter's no-dates rule and makes the Rust comparison exact.
- `serde` (with `derive`) and `serde_json` are admitted to `[workspace.dependencies]` by this ADR;
  later ADRs cite it rather than admitting them again.

### Consequences

- Good, because W3's many proving crates add two dev-dependency lines each and nothing else.
- Good, because a stale golden names itself and its module, not the whole oracle.
- Bad, because `#[path]` reaches outside the crate directory; the workspace is `publish = false`,
  so no package boundary is crossed, and rustfmt and clippy follow the module as usual.
- Bad, because an adapter is code a reviewer must read; its contract (call, never compute) is
  stated in the golden's `note` and checked in review.

### Confirmation

SPEC-029's acceptance tests: `test_generate.py` and `test_goldens.py` in the gate's python stage,
and `cargo test -p deck-streak-coordination --test golden_reader` in its test stage.

## What would make this wrong

- A crate needs the reader in production code (it should not: goldens are test inputs).
- rustfmt, clippy or cargo stops following a `#[path]` module outside the crate, measured by the
  gate's fmt and clippy stages.
- The registry grows past what one module per SPEC can review (then split per function family).

## More Information

ADR-012; SPEC-029; `docs/schematics/parity-oracle-goldens.md`; the data-migration pack's oracle
rows, which W8 judges over the same goldens.

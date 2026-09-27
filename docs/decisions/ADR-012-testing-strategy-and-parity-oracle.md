---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Testing: red first, a parity oracle from v9's own functions, and one runner per layer

## Context and Problem Statement

A port is correct when it behaves as its predecessor did, not as someone remembers it. The
predecessor's game math has traps a hand port gets wrong (Python's round-half-to-even, floor
division, the 04:00 rollover). The owner chose the testing tools "best for Linux building".

## Decision Drivers

- The tdd pack: red first with a recorded failure, positive assertions, examined counts, mutation.
- The data-migration pack's oracle: goldens that v9's own functions produced over synthetic inputs, committed and read by the port's tests.
- The golden paths `testing-rust` and `testing-js`.

## Considered Options (the alternatives it was chosen against)

- cargo-nextest with `cargo test --doc` and insta for Rust; Vitest 4 and Playwright (headless Chromium) for the Mini App; goldens from `tools/parity-oracle/` — chosen: the golden paths, and numbers proved against the predecessor rather than re-derived.
- Hand-written expected values for the game math — rejected because they encode the porter's reading, which is exactly what goes wrong at a rounding tie.
- Running v9's Python in DeckStreak's CI — rejected because v9 is private and CI is public; the generator runs on the owner's checkout and only synthetic goldens are committed.

## Decision Outcome

Chosen option. Every acceptance criterion names a test in its SPEC's fence and is recorded red
then green in `docs/red-first/<SPEC>.md`. `tools/parity-oracle/generate.py` runs on the owner's
private checkout of v9, imports v9's own functions, calls each over seeded synthetic inputs
(including tie, negative and rollover cases), and writes one golden per function
(`phx.parity-golden.v1`, strict JSON, `sort_keys`, `allow_nan=False`, the v9 commit and the
generator's sha256). The goldens are committed under `tools/parity-oracle/goldens/` and read by
the Rust tests that prove the port. An intended difference carries `diverges` with its ADR.
Every enumerating test reports what it examined and refuses zero.

### Consequences

- Good, because a port bug is a failing golden, named by function and case.
- Bad, because regenerating goldens needs the owner's v9 checkout; the generator records its own hash so a stale golden reads red.

### Confirmation

`cargo nextest run --workspace`, `cargo test --doc`, `pnpm -r test`, `pnpm -r test:e2e` and the tdd probe in `scripts/check.sh`; the data-migration oracle rows once `data-migration.json` lands.

## What would make this wrong

- A golden's function cannot be isolated from v9's database or clock (the generator then needs an adapter, recorded in the golden's note).

## More Information

docs/TESTING.md; the tdd and data-migration packs.

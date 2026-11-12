---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The open lapse's silent-day count is a named saturating step, pinned by an exact assert at its bound

## Context and Problem Statement

`open_lapse` in `crates/streaks/src/lapse.rs` counts silent days with `saturating_add(1)`.
Replacing it with `wrapping_add(1)` is not caught by any test or mutation row, because the two
forms differ only after the counter reaches `u32::MAX`, which no current input reaches. The Lean
entry `lean/OpenLapse` (ADR-305) proves the port never overflows, but that proof covers the port,
not the Rust, so the two can drift apart at the bound (#534). How does the Rust side pin the
behaviour at the bound without walking 2^32 days?

## Considered Options (the alternatives it was chosen against)

- A pure `const fn next_silent_count(silent: u32) -> u32` that `open_lapse` calls, tested directly with exact values at the bound: chosen, because the step is reachable by a test with no long walk, and a mutant of it is killed by an exact assert (#534).
- A 2^32-day walk in a test: rejected, because the issue forbids a long-running loop, and a walk that long cannot run in the gate (#534).
- The `wrapping_add` mutant declared EQUIVALENT by unreachability: rejected, because the issue asks for a Rust-side pin and the Lean proof covers the port, not the Rust (#534).
- A generic counter type: rejected, because it adds more surface than the defect and changes no behaviour (#534).

## Decision Outcome

Chosen option: the named step. `next_silent_count` saturates at `u32::MAX`; `open_lapse` calls it
in place of the inline step and nothing else in `open_lapse` changes. The Lean entry gains a second
`@phx covers` line for the new anchor and its module doc says `saturatingAdd` models it. A test
asserts 0 to 1, 7 to 8, `u32::MAX - 1` to `u32::MAX` and `u32::MAX` to `u32::MAX`, and that the
saturated count is at least each threshold in `[0, LAPSE_AFTER_SILENT_DAYS, u32::MAX]`. A mutation
row replaces `saturating_add(1)` with `wrapping_add(1)` and is killed by the `u32::MAX` test.

### Confirmation

`cargo test -p deck-streak-streaks --test open_lapse_bound` passes on the real code and fails by
assertion on a `wrapping_add` stub; the row S07651 reads KILLED; `formal check` reads no STALE.

## What would make this wrong

- A counter that stops being `u32`: the helper's signature and the Lean `UInt32` model would both
  change, and the digest of each covered span would read STALE.

## More Information

Issue #534; ADR-305; SPEC-076 sections 30 to 35.

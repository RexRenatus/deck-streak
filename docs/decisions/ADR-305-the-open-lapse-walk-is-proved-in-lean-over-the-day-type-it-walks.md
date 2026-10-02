---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The open lapse walk is proved in Lean, over the day type it walks, and held to the Rust by vectors

## Context and Problem Statement

`open_lapse` in `crates/streaks/src/lapse.rs` walks back over the window's days, from today to the
window's first day, and answers the open lapse's id. Since #446 it walks a bounded day range, with
a guard at the smallest day the type admits. #472 states its property: for every today and every
window the day type admits, the walk answers after at most one step per day from the window's
first day to today, never overflows at the smallest day, and answers the rule's lapse day, with one
exception at the smallest day. Today the tests check it over a generated population
(`crates/streaks/tests/open_lapse_bound.rs`, A41), which holds the smallest day but cannot hold
every day. How is the property proved for every input, and how is the proof held to the code?

## Decision Drivers

- The property quantifies over every day the type admits, about 1.8e19 of them, so no population
  can check it whole.
- The smallest day is the property's own edge: a proof over a type with no smallest day proves
  nothing about it.
- The repository's formal checker pins Lean 4.34.0 with Batteries, and allows three axioms
  (`config/formal.json`).
- A proof of a port is only as good as the port's agreement with the code it ports.
- A theorem that every function satisfies is vacuous, and nothing about the theorem shows it.

## Considered Options (the alternatives it was chosen against)

- One Lean entry, `lean/OpenLapse`, in core Lean, a port faithful to the Rust and to its day type: chosen, because the pinned checker builds, audits and judges it, and the port is held to the code by a digest and by vectors (#472).
- Proving the Rust directly with a tool outside the pin (a model checker or verifier for Rust): rejected, because the checker pins Lean and TLA+ only, so such a proof would be judged by nothing the repository's checks run, and a change to its tool would not be reviewable here (#472).
- Mathlib, or Batteries beyond what the pack allows: rejected, because core Lean's `Int64`, `UInt32`, `List` and `omega` carry every step, and Mathlib's cache and cold build would cost hours for nothing this proof needs (#472).
- Tests over the generated population only, today's state: rejected, because a population checks the inputs it holds, and the property is about every day the type admits (#472).
- A port over unbounded integers (`Int` or `Nat`): rejected, because an unbounded type has no smallest day, so "never overflows at the smallest day" would be vacuous (#472).
- Theorems with a hypothesis the issue does not state (today at or after the window's first day, or a window that does not reach the smallest day): rejected, because each would prove a narrower property than #472's and leave its edge unproved (#472).
- Theorems with no witness: rejected, because a theorem that holds of every port says nothing about this one, and only a witness the checker catches shows it does not (#472).

## Decision Outcome

Chosen option: one Lean entry, `lean/OpenLapse` (`formal/lean/Formal/OpenLapse.lean`), with these
six rulings.

1. **Core Lean.** The package requires nothing (`lake-manifest.json` holds no package). Each
   theorem rests on `propext`, `Classical.choice` and `Quot.sound` at most, the three
   `config/formal.json` allows, with no `sorry` and no new axiom.
2. **The port is faithful to the type.** The day is `Int64`, the type `StudyDay` wraps
   (`pub struct StudyDay(i64);`, `crates/kernel/src/study_day.rs`), so the smallest day is
   `Int64.minValue`. The range is ported as core's `RangeInclusive::next_back`, branch for branch,
   and the count saturates as `u32::saturating_add` does. The entry covers
   `crates/streaks/src/lapse.rs` at `anchor=open_lapse` with the digest the checker derives.
3. **The theorems state #472's property, nothing narrower.** Each quantifies over every today,
   every window (a list of entries in key order, so duplicate keys are admitted too, which a map
   never holds), every set of skip days and every threshold, with no hypothesis:
   - `at_most_one_step_per_day`: the walk reads at most one day per day from the window's first
     day to today, and none when today is before it;
   - `never_overflows`: no step back overflows;
   - `answers_the_rule`: the answer is the rule's, stated over `Int` apart from the port, with the
     one exception at the smallest day.
   No narrowing was needed.
4. **Conformance by vectors.** The entry declares `formal/vectors/open-lapse.jsonl`, written by
   `Formal/OpenLapseVectors.lean` from five axes (the window's first day, the threshold, the
   width, each day's state, today). `crates/streaks/tests/formal_vectors_open_lapse.rs` derives
   the same population from the same axes, refuses a file that differs from it, answers every
   input with the Rust function, and prints `examined N vector(s)` with N the derived count.
5. **A witness for every theorem.** Each is a port that breaks one claim at one input, proved by
   the kernel's evaluation (`decide +kernel`): a walk whose range starts the day before the
   window's first day breaks the step bound; a loop that steps back from a day without a check
   overflows at the smallest day; a walk without the guard answers a lapse at the smallest day.
6. **`config/formal.json` is unchanged.** The entry builds well inside the existing
   `entry_seconds` and `lean_seconds` budgets, so no entry budget is added.

### Consequences

- Good, because the property is proved for every day the type admits, the smallest included, and
  A41's population remains as a second, independent check.
- Good, because a change to `open_lapse` moves the covered span's digest, so every theorem reads
  `STALE` until the port is re-read.
- Good, because the Rust function answers every vector, so a port that drifts from the code is red
  in the crate's own tests, and a planted change to the function is red too.
- Bad, because the port is a second copy of the walk, which a change to the function must carry.
  The digest names when.
- Bad, because the port takes fuel for structural recursion, which the Rust loop has none of. The
  fuel exceeds any window of `Int64` days, and `answers_the_rule` proves the walk never runs out.

### Confirmation

The full formal check at the head reads FORMAL OK with `lean/OpenLapse` built, its three witnesses
caught and its vectors byte-equal; the ratchet reads no weakening; and the crate's vectors test
passes over every derived vector.

## What would make this wrong

- A `StudyDay` that stops wrapping `i64`: the port's type would no longer be the code's, and the
  digest of `open_lapse` does not cover `study_day.rs`. The vectors hold the type's smallest and
  largest days, so the test reads both ends.
- A checker that stops auditing a theorem's axioms, or stops building witnesses: the theorems would
  stand unjudged. The checker's own tests hold both.

## More Information

Issues #472 and #446; SPEC-076 sections 16 and 17 (A41); the schematic
`docs/schematics/open-lapse-proof-and-its-vectors.md`.

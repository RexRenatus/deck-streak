---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The XP ledger: one row per study day, source and track, a once scope for grants that pay one time ever, and an amount that cannot be negative

## Context and Problem Statement

Every context that grants XP writes through progression's grant port (the context map calls it an
upstream supplier). The predecessor keyed its ledger by study day and source, carried the track as
a plain column, and guarded its once-ever grants (drills) with a second table in another module,
because a grant replayed on a later study day has a new key. The readings now add two once-ever
grants per reading (SPEC-047), and the charter forbids any path that takes XP away (constraint 5).
How should the ledger key, the once-ever guard and the amount be shaped so that a double grant and
a debit are impossible by construction?

## Decision Drivers

- Idempotency guards travel with their feature (constraint 9), and a guard in a second context
  cannot share the ledger's transaction.
- XP is never confiscable (constraint 5): a debit should fail to compile, not fail a review.
- The level must match the predecessor exactly (constraint 8), from an integer square root.
- One grant port serves every wave: readings now, drills and study-owned sources later.

## Considered Options (the alternatives it was chosen against)

- A ledger unique on (study day, source, track), plus a partial unique index on (source, track) for rows whose scope is `once`, an unsigned amount newtype, and the level derived from the summed total at read time — chosen: both guards live in one table and one transaction, a debit cannot be expressed, and no stored level can drift.
- The predecessor's shape, unique on (study day, source) with the track as a plain column — rejected because a source granted on both tracks collides or needs track-suffixed source names, and it gives no once-ever guard.
- A per-feature guard table beside the ledger (the predecessor's `drill_xp_grants` pattern) for once-ever grants — rejected because the guard and the grant would live in two contexts and two transactions, so a crash between them either loses the grant or pays it twice.
- A signed amount with a runtime check that refuses negatives — rejected because a penalty path would compile and the check would be the only barrier.
- A stored level updated on each grant — rejected because it can drift from the ledger, and the predecessor derives it from the total.
- A `SELECT` for the key before the insert, beside the unique indexes — rejected at acceptance because it states the key a second time, in code, where it can drift from the index; the insert's own conflict with the indexes is the existence check.
- A rustdoc `compile_fail` doctest on `XpAmount::new` instead of `trybuild`, which needs no dependency — rejected because rustdoc passes a `compile_fail` block that fails to compile for any reason, so a typo or a moved path would read as the proof (measured on the pinned toolchain: a misspelt type, E0433, and a missing module, E0432, both pass, and stable ignores an error code written after `compile_fail`), while `trybuild` compares the compiler's refusal with the recorded `.stderr`, so A6 fails unless the refusal is the u32/i64 mismatch.

## Decision Outcome

Chosen option: the ledger key (study day, source, track), a `scope` column of `per-day` or `once`
with a partial unique index `(source, track) WHERE scope = 'once'`, an unsigned amount newtype
with no signed constructor and a `CHECK (amount >= 0)` column, and the level computed from the
total with the predecessor's integer formula. `trybuild` is admitted as a dev-dependency to prove
the signed amount does not compile.

At acceptance: a grant is one `INSERT ... ON CONFLICT DO NOTHING` inside the port's
`BEGIN IMMEDIATE` write, so the key lives in the migration alone; when the insert writes nothing,
the row that holds the key is read in the same transaction for the `AlreadyGranted` answer. The
amount is an unsigned 32-bit newtype, the total an unsigned 64-bit one, and the level is computed in
128-bit integers, so no total a ledger can hold overflows it
(`docs/schematics/xp-grant-port.md`).

### Consequences

- Good, because a replayed grant, on any day and from any surface, writes nothing and says so.
- Good, because the reading's two grants, and later the drill grants, need no guard table of their own.
- Bad, because a caller must choose the scope correctly; SPEC-047's tests pin the readings' choice.
- Bad, because the total is summed per read until a measured cache is needed.

### Confirmation

SPEC-040's acceptance tests A1 to A10, in `crates/progression/tests/`; the golden of
`gamification/xp.py:level_for_xp` in `tools/parity-oracle/goldens/level_for_xp.json`; and the
hand-proved rows of the level's floor, the two keys and the source's refusal in
`scripts/mutation-rows.d/S04000-S04099.json`.

## What would make this wrong

- A grant source that must pay once per study day on one track and once ever on another (then the
  scope moves to a per-source registry).
- The summed total becomes a measured latency problem on the host (the API's SLO pages).

## More Information

SPEC-040; SPEC-047 (the reading grants); the predecessor's `gamification/xp.py:level_for_xp` and
`database.py:GamifyStore.upsert_xp_grant`; the game-economy pack's `xp-table` row over `economy.json`.

---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Each owner writes its own imported rows, in one transaction, and the predecessor's record wins up to its last study day

## Context and Problem Statement

The v9 import (#61, ADR-008) carries the predecessor's 64 tables at schema 24 into DeckStreak's
contexts. Four censuses already refuse a second writer of a table by name: SPEC-040 A9 for
`xp_ledger`, SPEC-072 R9 for `xp_settlement`, SPEC-138 for `xp_price_changes` and SPEC-137 for the
publishing tables. Side by side (#62, ADR-011) DeckStreak also records its own rows for days the
predecessor recorded, so an imported key can meet a row DeckStreak already holds. Who writes the
imported rows, in how many transactions, and whose row stands on a shared key?

## Decision Drivers

- One writer per table (the contexts' own-tables register in `docs/CONTEXT-MAP.md`).
- A failed import must leave nothing half-written.
- Every derived value is proved by a golden from the predecessor's own function (ADR-012).
- A second apply of the same copy must change nothing (#61's second criterion).

## Considered Options (the alternatives it was chosen against)

- Each context's `ImportPort` in one `Db::write` transaction: chosen, because the owner stays the
  one writer, the transaction makes the apply all or nothing, and the predecessor was the record
  until the go. The migration crate composes the ports, and the predecessor's row wins on a shared
  key.
- One migration crate writing every table: rejected because it is a second writer of every table,
  which four censuses refuse by name, and rightly.
- One transaction per context: rejected because a failure in a later context leaves the earlier
  ones imported, and the rollback becomes the only way back from an ordinary refusal.
- `ATTACH` and `INSERT ... SELECT` per table: rejected because the derived values (card-state
  stamps, a day's base XP, the multipliers) follow the predecessor's functions, which plain SQL
  neither ports nor proves against their goldens, and it bypasses the owners.
- DeckStreak's row wins, the import filling only gaps: rejected because DeckStreak's side-by-side
  rows are its own readings of days the predecessor was the record of; on a shared key the
  predecessor's row replaces DeckStreak's, and a DeckStreak row the predecessor lacks is kept,
  except a day reading on or before the cutoff, which is superseded (SPEC-140 R3).

## Decision Outcome

Chosen option: each owner's `ImportPort`, in one transaction, the predecessor winning up to its
last study day, because it keeps every table's single writer and makes the apply atomic.
SPEC-140 R1 to R5 and SPEC-141 hold it; SPEC-142 R7 composes it.

### Consequences

- Good, because the censuses stay true, and an owner's import is tested in its own package.
- Good, because every instant written comes from the source, so a second apply is `unchanged`.
- Bad, because every owner of a table carries an import module, and the migration crate depends on
  every one of them.

### Confirmation

SPEC-140's and SPEC-141's criteria per owner; SPEC-142's idempotency, reconcile and rollback
criteria; the four censuses, unchanged.

## More Information

#61, #62, ADR-008, ADR-011, ADR-012, SPEC-140, SPEC-141, SPEC-142.

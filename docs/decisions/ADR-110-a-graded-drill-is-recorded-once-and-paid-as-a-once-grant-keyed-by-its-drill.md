---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A graded drill is recorded once and paid as a `once` grant keyed by its drill

## Context and Problem Statement

The predecessor pays a graded law drill in two writes: it inserts the drill's id into
`drill_xp_grants` and, when that row is new, writes an XP row with the source `drill:<id>` on the
law track (`database.py:GamifyStore.grant_drill_xp_once`). The amount is the graded note's `xp`
override, which a model wrote, clamped to 10..25, or 15 (`vault_bridge.py:_parse_graded_drill`).
Its poll re-scans every graded note on each call and relies on the guard table for idempotency
(`vault_bridge.py:poll_drill_postbacks`).

DeckStreak's XP has one writer, the grant port (SPEC-040 R10), whose `once` scope already holds at
most one row per (source, track) across every study day (SPEC-040 R4) inside one `BEGIN IMMEDIATE`
write (R5). The idempotency guard must travel with the feature (the ten rules), and the port's
source grammar is `^[a-z0-9][a-z0-9:._-]{0,127}$` (R2), which a drill note's stem need not fit.
The drill-grades memory source (SPEC-044 R7) and the progress screens (#55) need the grade itself.
How is a graded drill recorded and paid, once?

## Decision Drivers

- One guard per pay: two guards can disagree after a crash between them.
- The grant port is the only writer of XP (SPEC-040 R10).
- A model's number is advisory until a deterministic rule accepts it (the W6 plan's content rule).
- The v9 import (#61) must carry the predecessor's grants without paying twice.

## Considered Options (the alternatives it was chosen against)

- A `once` grant keyed `drill:<id>`, and a separate `drill_grades` record: chosen, because the
  port's own scope is the guard, the record is idempotent on its own key, and either write
  completes on the next poll.
- A `drill_xp_grants` guard table beside the grant, as the predecessor had: rejected because it is
  a second guard that can split from the ledger's: a crash after the guard's insert and before the
  grant leaves a drill marked paid and never paid.
- A derived settlement under ADR-072, like the leech remediation: rejected because a grade is a
  one-time event with no undo and no per-day recompute, and a settlement would let a recompute
  re-price it.
- The model's `xp` unclamped, or the engine's own formula from the scores: rejected because the
  first lets a hallucinated value over-credit the ledger, and the second breaks parity with the
  predecessor's pay, which its golden proves.
- Rewriting an unkeyable stem (lower-casing it, replacing spaces): rejected because two stems can
  rewrite to one key and the second drill would never pay.

## Decision Outcome

Chosen option: "a `once` grant keyed `drill:<id>`, and a separate `drill_grades` record", because
it keeps one guard, and the pay and the record each repair themselves on the next poll.

- **The pay.** The poll asks the grant port for the clamped amount (10..25, default 15) on the
  study day of the poll, source `drill:<id>`, track `law`, scope `once`. An `AlreadyGranted`
  answer is the expected answer of a re-poll.
- **The key.** An id is kept as-is when `drill:<id>` fits the port's grammar and the id does not
  itself begin with `h.`. Every other id is keyed `drill:h.` followed by the first 32 hex
  characters of the SHA-256 of the id, so a kept key and a hashed key can never be equal. The v9
  import applies the same rule to the predecessor's `drill:` rows.
- **The record.** `drill_grades` (vault) holds the drill's type, subject, accepted XP and study
  day, one row per drill, written `INSERT OR IGNORE`. It maps from the predecessor's
  `drill_xp_grants`, row for row.

### Consequences

- Good, because a crash between the two writes is repaired by the next poll, and no drill is ever
  marked paid without a ledger row.
- Good, because the import brings the predecessor's `drill:` rows as `once` grants, so the first
  poll after it answers `AlreadyGranted` for every imported drill.
- Bad, because a drill whose stem is unkeyable shows a hashed source in the ledger's export. The
  grade record keeps the stem, so the export still names the drill.

### Confirmation

SPEC-110's A9 (one pay on the study day), A13 (an interrupted poll completes without a second pay)
and A11 (the key rule), and its rows S11008, S11009 and S11016.

## What would make this wrong

- A drill that is graded again after a regrade and should pay again: the `once` scope would refuse
  it. No W6 issue asks for a regrade; one would be its own decision.
- A grant port that lost its `once` scope's any-day check: the pay would repeat. SPEC-040's own
  rows guard it.

## More Information

SPEC-040 (the grant port), SPEC-110 (the drills), ADR-072 (derived settlements, which this is not),
ADR-012 (the goldens), the W6 schematic `docs/schematics/law-drill-answer-grade-and-pay.md`.

---
status: "accepted"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Ingest walks the protobuf wire format by hand, and the instruments' reads keep the scope

## Context and Problem Statement

Two questions arise for every W4 instrument read.

First, Anki stores templates, note-type configuration, deck kinds and presets as protobuf blobs.
The predecessor walks the wire format with a pure-stdlib reader (`darkfields.py:_iter_fields`,
`runway.py:_iter_pb_fields`, predecessor `27ee2bc`) and reads a handful of numbered fields.

Second, the predecessor's instrument reads disagree about scope and window. Some keep the deck
filter and a window (the fluency audit, Divestment Day, the tilt test); some read the whole log with
no deck filter (the Echo Test, the price of a day off); some compute a lag against each card's
previous review. SPEC-023 reads within a scope (`DECKSTREAK_INCLUDE_DECKS`) and a window
(`INGEST_WINDOW_DAYS`, 400 days). Which rule do the W4 reads follow?

## Decision Drivers

- ADR-012: the goldens drive the predecessor's own readers over synthetic collections.
- A decode failure must be a named failure, never a partial result read as a measurement.
- Decks the owner left out of the scope must stay out of every report.
- A full read is slow, so a read of answers stays within its window.
- A lag computed only inside the window reads the first answer there as having no prior.

## Considered Options (the alternatives it was chosen against)

- A hand-ported wire walker in ingest proved by the predecessor's golden, and one read rule: every read keeps SPEC-023's scope, a read of answers keeps its window's floor, a set of ids ever reviewed reads the whole scoped log, and a lag is computed over the whole scoped log before the floor applies — chosen: it reads the same bytes the predecessor reads, fails where it fails, and gives every instrument one scope.
- Generated types from Anki's `.proto` files — rejected because it adds code generation and a schema that changes across Anki's versions for three or four numbered fields, and its errors differ from the predecessor's.
- The predecessor's mix of scoped and unscoped reads — rejected because an unscoped read puts decks the owner excluded into a report.
- Every read inside the window, lags included — rejected because the first answer in the window would read as having no prior, and a set of ids ever reviewed would shrink.

## Decision Outcome

Chosen option: "A hand-ported wire walker, and one read rule", because both keep parity where it
matters and make the departures explicit.

- `crates/ingest/src/wire.rs` walks varints, fixed and length-delimited fields with no schema; a
  truncation or an unknown wire type is an error (SPEC-094).
- Every instrument read keeps SPEC-023's scope. The runway's deck tree, kinds and presets are read
  whole, because a scoped deck's budget may be set on a root outside the scope (SPEC-098).
- A read of answers counts only answers after its floor. A read of ids ever reviewed reads the whole
  scoped log. A lag (the lateness bands, the tilt pairs) is computed over the whole scoped log, and
  only answers after the floor are counted.
- Where the predecessor read the whole collection, its golden is generated with the scope set to
  every deck, and a separate test holds the scope.
- No read registers a collation or orders, groups or seeks on a `unicase` name column; a full scan
  is complete without it (SPEC-094 R4).

### Consequences

- Good, because a report never includes a deck the owner excluded.
- Good, because the walker is a few dozen lines proved by a golden, with no generated code.
- Bad, because the Echo Test and the price of a day off change from whole-collection to scoped;
  with every deck in scope the result is the predecessor's.
- Bad, because a field Anki renumbers breaks a read; the read then fails by name.

### Confirmation

SPEC-094's wire-walk golden; SPEC-095's lateness and review reads; SPEC-097's tilt pairs; SPEC-098's
scoped counts; SPEC-099's scoped Can-Do read.

## What would make this wrong

- DeckStreak needs a large part of Anki's schema; generated types then pay for themselves.
- The owner wants an instrument over decks outside the scope; that instrument then names its own
  scope in its SPEC.

## More Information

ADR-012; SPEC-023; SPEC-094 to SPEC-099.

## Amendment 2026-09-29: the wire walk refuses a pass that does not advance

The walk loop took its next position from a callee. A mutant, or a future edit, that let a pass
return the position it started from would push one field per pass without bound. That is an
allocation problem, not a time one, and the per-run timeout of the mutation lane does not protect a
small runner from it. Measured with a 240 s timeout on `varint -> Ok((0, 0))`: 247 s of test time
and a peak resident size of about 116 GB.

Decision: `walk(data)` stays the public entry and calls `walk_with(data, step)`, which refuses a
pass that does not strictly advance with the named refusal `NO_PROGRESS` before keeping its field.
The guard's own mutants are killed by `crates/ingest/tests/wire_progress.rs`, three readers (one
that stays, one that steps back, one that advances by one), so no equivalent record is added and
the closed campaign row for `deck-streak-ingest` keeps its count.

Chosen against:

- A workflow `--exclude-re`, a timeout change or a memory change: each weakens the lane and needs
  the owner's signed ruling.
- A bound on the field count, with equivalent records: it is untested unreachable code, and the
  campaign row for the crate is closed.
- Relying on the runner's timeout: it does not protect a small runner from memory.

The same guard is applied to the template token scan of Dark Fields (`tokens_with` over `token_step`
in `crates/insights/src/dark_fields.rs`), whose stalled mutants spun a test for its whole timeout:
`tokens_with` refuses a step that does not strictly advance with the named refusal
`TOKEN_NO_PROGRESS`, and `config_tokens_with` marks that template failed, as a wire walk's
`NO_PROGRESS` does, so its note type is named unparseable. Its tests are
`crates/insights/tests/token_progress.rs`. Chosen against for the scan: ending it with `break` and
answering the tokens it kept. A stalled scan then judges a note type on part of its templates and
reports fields they do render as dark, with no failure named.

The class rule, as a guard: `scripts/tests/test_callee_offset_loops.py` enumerates every loop under
`crates/*/src` whose integer position comes from a callee (two: `walk_with` and `tokens_with`), and
refuses one with no strict-advance compare, or one whose compare does not return an `Err`.

Class sweep of the loops this delivery adds or touches (ingest and insights): `wire.rs` `varint` (each pass takes one byte
from `data` and ends at its end), `wire.rs` `walk_with` (guarded) and `dark_fields.rs` `tokens_with` (guarded). Three loops, two guarded, one bounded by its input.
The sweep names only loops in files this delivery changes, and its count is the `while` and `loop` constructs the delivery adds under
`crates/ingest/src` and `crates/insights/src`, counted by script.

---
status: "accepted"
date: "2026-10-01"
decision-makers: "the DeckStreak architect seat, ruling on the model's trace"
---

# An award carries its celebration mark and is offered again until the router answers

## Context and Problem Statement

SPEC-073 awards a badge or a record in phase 7 of the settle fold and celebrates it through the one
router. The fold's write commits first and the celebration reaches the router afterwards, so a crash
between them can lose the celebration, and a replay of the day must not send it twice. The seat's
ruling: each award is written once and celebrated at most once, and never silently lost.

## Decision Drivers

- The write that awards must be one `BEGIN IMMEDIATE` write, and the router is a separate act after it.
- The router writes through a connection of its own: `router.rs::route` opens `self.db.write()`, and
  `db.rs::write` begins `BEGIN IMMEDIATE`, so no offer can run inside the fold's write.
- The router already keeps each dedupe key to one send (SPEC-041 R4 rule 2, the unique index
  `notification_deliveries_scoped_key`), so a repeated offer is safe.
- A records row holds one kind, so a later day's write can replace a row whose celebration was never offered.

## Considered Options (the alternatives it was chosen against)

- The literal reading of R3 and R4: only the evaluation whose insert wrote the row raises the celebration, and `AlreadyAwarded` raises none. Lost, because TLC reaches a trace in which the evaluation crashes after the write and before the router, and no later evaluation ever offers it (`NoSilentLoss` fails; `formal/tla/AwardOnce/witness/a-celebration-raised-only-by-the-inserting-evaluation.cfg`).
- A `celebrated_at` mark on the award row, offered between the fold's writes until the router answers, with a records write that re-reads its row inside its own `BEGIN IMMEDIATE` and names a record it replaces unmarked (chosen): the model reads clean on all three properties with a crash between any two transactions and a router call that does not answer, and the router's key keeps each to one send.
- A separate outbox table of pending celebrations: lost, because it adds a table, six data-rights files and a second writer for a fact one nullable column on the award already holds.
- Offering an earlier day's unmarked record from inside the records write's own transaction: lost, because the router cannot run there: `Router::route` opens its own `BEGIN IMMEDIATE` write on another pooled connection, which waits on the writer lock its caller holds until the busy timeout.
- Deferring a records write while the stored row is unmarked and from another day: lost, because a router that never answers stalls every later record for ever, a liveness gap; drain then write lets the day's write go on and names what it replaced.
- Offers between the writes with no re-check inside the write: lost, because TLC reaches a trace in which the offer before a later day's write goes unanswered and that write replaces the unmarked record with nothing naming it (`NoSilentLoss` fails; `formal/tla/AwardOnce/witness/a-later-day-overwrites-an-unoffered-record.cfg`).
- Naming the lost celebration only in a log line, with no retry: lost, because the seat ruled "never silently lost" and a name is the fallback for a record superseded before it could be offered, not the design.
- The Lean package scaffolding under `formal/lean/` (a `lakefile.toml`, `lean-toolchain`, an empty-packages `lake-manifest.json`, `Formal.lean`): accepted, chosen against a lone `.lean` file outside a package, because the checker builds through lake, and against a mathlib dependency, because the next-milestone rule compares exact rationals by cross-multiplication and needs none.

## Decision Outcome

Chosen option: "a `celebrated_at` mark on the award row, offered between the fold's writes until the
router answers, with a re-checked records write", because it is the smallest change that makes an
award neither lost nor sent twice while the router keeps its own transactions.

The protocol, each step one transaction, with a crash possible between any two:

1. before each settled day's write and the current day's write, the fold offers every unmarked badge
   and record to the router, and marks each one the router answered in a write of its own, which
   marks it only while its row still holds it;
2. the day's write re-reads the record row inside its own `BEGIN IMMEDIATE`; when it replaces another
   day's row whose mark is still unset, it names that record in a log line (its kind, its day and
   "celebration not sent"), then writes the award or the record with its mark unset;
3. after the fold's last write, the same offers run again.

### Consequences

- Good, because a crash at any point leaves the row unmarked and the next offers, of this evaluation
  or of any later one, offer it again.
- Good, because the day's write never waits on the router, so a router that does not answer delays no
  award.
- Bad, because a record superseded before the router answered is lost to the owner's feed and is only
  named in a log line.

### Confirmation

`formal/tla/AwardOnce/` (three properties, four witnesses), `formal/lean/Formal/NextMilestone.lean`,
and the tests `badges_steps` and `records_steps` that stop an evaluation after the write and after the
router's answer and assert exactly one send over the following evaluations.

## More Information

SPEC-073, ADR-041, ADR-071; issues #74, #75 and #76.

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
- The router already keeps each dedupe key to one send (SPEC-041 R4 rule 2, the unique index
  `notification_deliveries_scoped_key`), so a repeated offer is safe.
- A records row holds one kind, so a later day's write can replace a row whose celebration was never offered.

## Considered Options (the alternatives it was chosen against)

- The literal reading of R3 and R4: only the evaluation whose insert wrote the row raises the celebration, and `AlreadyAwarded` raises none. Lost, because TLC reaches a trace in which the evaluation crashes after the write and before the router, and no later evaluation ever offers it (`NoSilentLoss` fails; `formal/tla/AwardOnce/witness/a-celebration-raised-only-by-the-inserting-evaluation.cfg`).
- A `celebrated_at` mark on the award row, offered again at each evaluation until the router answers (chosen): the model reads clean on all three properties, and the router's key keeps each to one send.
- A separate outbox table of pending celebrations: lost, because it adds a table, six data-rights files and a second writer for a fact one nullable column on the award already holds.
- A pre-offer of every pending record before each records write, as an option outside the write: lost, because TLC refuted it (a later day overwrote an unoffered record between the offer and the write); the guard is inside the write's own read (`a-later-day-overwrites-an-unoffered-record.cfg`).
- Naming the lost celebration only in a log line, with no retry: lost, because the seat ruled "never silently lost" and a name is the fallback for a record superseded before it could be offered, not the design.
- The Lean package scaffolding under `formal/lean/` (a `lakefile.toml`, `lean-toolchain`, an empty-packages `lake-manifest.json`, `Formal.lean`): accepted, chosen against a lone `.lean` file outside a package, because the checker builds through lake, and against a mathlib dependency, because the next-milestone rule compares exact rationals by cross-multiplication and needs none.

## Decision Outcome

Chosen option: "a `celebrated_at` mark on the award row, offered again until the router answers", because
it is the smallest change that makes an award neither lost nor sent twice.

### Consequences

- Good, because a crash at any point leaves the row unmarked and the next evaluation of any day offers it again.
- Good, because the guarded records write offers an earlier day's unmarked record before replacing it.
- Bad, because a record superseded before it could be offered is lost to the owner's feed and is only named in a log line.

### Confirmation

`formal/tla/AwardOnce/` (three properties, four witnesses), `formal/lean/Formal/NextMilestone.lean`,
and the tests `badges_steps` and `records_steps` that stop an evaluation after the write and after the
router's answer and assert exactly one send over the following evaluations.

## More Information

SPEC-073, ADR-041, ADR-071; issues #74, #75 and #76.

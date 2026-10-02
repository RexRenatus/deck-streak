---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The fold settles from the cursor it re-reads in each day's write, and a day before the first settle owes no row

## Context and Problem Statement

Issue #311 asks two things of the recompute's fold (`Fold::run` in
`crates/coordination/src/recompute/mod.rs`, SPEC-071 R15 to R17, ADR-071).

First, two folds can overlap. The scheduled cycle and the owner's recompute each run the fold after
their own successful sync, and either can reach it before the other has cleared the rescore mark.
At `9b0bf65f9` the fold reads the settle cursor once, in its first write (`mod.rs:479`), and opens
each owed day's write with no re-read (`mod.rs:503`). Each write is one `BEGIN IMMEDIATE`
transaction, so two writes never interleave, but the decision of which day to settle was taken from
a read in an earlier transaction: two folds that both read the same cursor settle the same closed
day twice, against R16. The model `formal/tla/FoldSettlesOnce` reaches it in eight states (both
cycles read cursor 0 and owe day 1; each settles it), and reaches a settle out of order in nine.

Second, a closed day before the first settle. Measured at `9b0bf65f9`, with reviews on D0 and D0+3:
a recompute on D0+2 after a sync that started on D0+1 backfills D0 and settles nothing (D0+1 closed
but no sync started after its close); a recompute on D0+3 still finds no cursor, backfills D0 again
and settles D0+2; a recompute on D0+4 settles D0+3. D0+1 never has a row. The issue asks whether
such a day owes one.

## Considered Options (the alternatives it was chosen against)

The overlap:

- Re-read the settle cursor inside each owed day's `BEGIN IMMEDIATE` write; the day owed is the day after it, or the run's own day when there is no cursor yet, and a write whose day is not the owed one commits nothing while the run goes on from the owed day: chosen, because the write lock already orders every write on the ledger file across connections and processes, so the cursor read inside the write is the one the commit acts on, and two overlapping folds settle exactly what one fold after the other would (#311).
- Re-read the cursor and settle the run's day unless the cursor is at or past it: rejected, because it only skips forward, and two folds that both read no cursor leave a gap: a scheduled fold whose closed day is D+1 settles D+1, the owner's fold whose closed day is D+3 re-reads cursor D+1 and settles D+3, and D+2 is never settled (#311).
- Re-read the cursor and the rescore mark: rejected, because the fold never reads the mark (the owner's `/sync` sets it and `sync_cycle.rs` clears it after the fold), so only the cursor decides which days are owed (R16) and a second read would decide nothing (#311).
- A process-local mutex around the fold: rejected, because the scheduled job and the bot are two processes on one ledger file, and a mutex in one process does not serialise the other (#311).
- A lock row with a lease: rejected, because it is a second protocol to model and to test, and a lease left behind by a crash has to be recovered before any fold can run (#311).

A closed day before the first settle:

- (b) It owes no row unless it is a study day of the window or some recompute's current day, and SPEC-071 states the rule: chosen, because the backfill already rolls up every study day of the window in the historical form, a day with no reviews has nothing to roll up, and after the first settled day the cursor settles every day in turn, gap days included (#311).
- (a1) The backfill starts at the first closed day the ledger knows, not at the first day with reviews: rejected, because it is partial: with no reviews on days 1 to 4, a fold on day 2 whose sync started on day 1 and a fold on day 5 leave day 3 with no row while days 2 and 4 have one (#311).
- (a) The day owes a row, kept by a persisted first-owed marker: rejected, because it costs a migration and a ledger surface for continuity before the first settle, which no criterion asks for (#311).
- (a2) Every calendar day before the first settle gets a row: rejected, because it contradicts R17 and A19 (the first recompute rolls up the window's study days) and adds a row per calendar day (#311).

## Decision Outcome

Chosen options: the in-write re-read, settling from the cursor; and (b).

Inside each owed day's write, before the day's steps run, the fold re-reads
`rollup::settle_cursor`. With a cursor, the owed day is the day after it; with none, it is the
run's own day. When the owed day is not the run's day, the write is dropped uncommitted and the
loop goes on from the owed day, so its own condition (the day has closed, and the run's sync
started after its close, R15) decides whether anything is still owed. Otherwise the day is settled
as before, its steps and its `settled_at` in the one write (R16). The fold reads the cursor once
more per owed day and nothing else changes: the first write, the current day's write and the
revisit stay as they were. The model's re-read arm is the same rule, and it gains the property
`SettleInTurn` (no settle lands past the day after the cursor), with a witness without the re-read
that violates it.

For (b), the fold does not change: SPEC-071's amendment of this date states which days before the
first settle have a row, and a pin test holds the base's behaviour to it. No mutation row pins (b),
because there is no code arm to mutate: it is the base's behaviour, stated.

### Confirmation

A27 runs two folds on two connections to one ledger file, held by a barrier after each fold's first
read of the cursor, so that both have read it before either settles, over 72 runs: 18 distinct
members, each released four ways. In two releases the test forces the order of every write after
the barrier through the fold's own offers port: each later offer of a fold hands the turn to the
other fold and waits for its own, so each write runs alone, the scheduled fold's turn first in one
release and the owner's in the other, and the test asserts that the turns alternated. In the other
two, both folds leave the barrier together and race to the write lock, joined in each order, as
A27's 36 runs did before fix round 1. At `9b0bf65f9` a closed day is settled twice, and with the
re-read every day from the first settled one to the last closed one is settled exactly once, oldest
first.

What each release catches was measured with the fold's owed-day block replaced. A fold that settles
the run's day unless the cursor is at or past it (the second option above) fails A27 on every run,
in a forced release. A fold that re-reads the cursor in a transaction of its own, before the write,
fails A27 only on a run where the race puts both reads before either write, so it is caught at a
rate, not on every run: the forced releases never caught it, because each fold's read and write
then run with no write of the other between them. Its structural guard is the covers of
`FoldSettlesOnce` on `Fold::run` and `Db::write`: moving the re-read out of the write changes
`run`'s span, and the cover reads STALE until the model is re-read against it. With that change
committed, `formal covers` read `run` STALE and the entry's other four covers FRESH.

A28 pins (b) and is green at the base, recorded as a pin. `formal check` reads `SettleOnce`,
`SettleOldestFirst` and `SettleInTurn` clean on the built protocol, and each witness violates its
property. The mutation row S07114, which removes the re-read, reads KILLED by A27. S07115, which
owes the day after the run's own when there is no cursor, reads KILLED by A16, and S07116, which
goes on from the day after a passed write instead of the owed day, reads KILLED by A27.

## What would make this wrong

- A cursor that stops being the last settled day (a write that clears `settled_at`, or one that
  settles outside `Fold::run`'s loop): the day after it would no longer be the owed day. The covers
  of `settle_cursor` and `record_settled` in `FoldSettlesOnce` read STALE on such an edit.
- A day's settle split across two transactions: the re-read would no longer guard the write that
  commits. `db.rs::write` is covered for that reason.

## More Information

Issue #311; ADR-071 (the fold); ADR-303 (offers between the fold's writes); SPEC-071 sections 11
and 12; `formal/tla/FoldSettlesOnce`; `docs/schematics/recompute-settles-each-study-day.md`.

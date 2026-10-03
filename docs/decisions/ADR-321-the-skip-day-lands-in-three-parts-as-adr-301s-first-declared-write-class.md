---
status: accepted
date: "2026-10-03"
decision-makers: "@RexRenatus (owner, at #266 and #514), the DeckStreak architect"
---

# The skip day lands in three parts, as ADR-301's first declared write class, with its backup, counts, stop and formal decision

## Context and Problem Statement

SPEC-083 (planned) was written before ADR-301. Its first branch was parked at stage 1: the record,
both migrations, the rights port, the recorder control and the seven goldens. Since then ADR-301
(#514) has made the skip day the first declared write class, and its part (b) requires every write
batch to take a whole-collection backup proved by a restore drill, and to record counts before and
after it runs. Its part (c) has each class's ADR state its change budget, its dwell time between
changes, and the band a result must clear. ADR-301's #518 note on (b) states no place and no
retention for the backup. ADR-089's #518 budget note leaves the dwell and the band to #108.
SPEC-082 has since landed the wallet's floor-clipped debit and its refund, each once per key.

Seven questions follow:
- How does SPEC-083 land now?
- What are ADR-083's status and the branch's two body edits?
- Where does the backup live, and how long is it kept?
- What stops the class?
- Which study day carries the tariff and its refund?
- What is the class's formal decision?
- Which of the month's skips does a take or a retry count when it prices the skip?

## Decision Drivers

- #108 asks for the skip day whole. Its record and its effects on the game need no write, so they
  can land and be proved now. The write waits on ADR-301 (b)'s backup, restore drill and counts,
  which nothing has built yet.
- A builder's slice of a SPEC moves only the criteria it delivers. SPEC-082 and SPEC-091 section 3c
  are the house shape: a moved row keeps its text, and section 10 records each change to the body.
- The coin ledger's key is `(study_day, source, reference)`
  (`migrations/008201_economy_wallet_and_shop.sql`), and both of the wallet's ports are once per key.
- No agent drives a take, an undo or a sync against the owner's real collection, tests included.
- The context map is binding: coordination already depends on ingest and on economy, and nothing
  here adds an edge.

## Considered Options (the alternatives it was chosen against)

- D1 (R-a), resume the parked branch in its worktree by one house merge of dev, the merge commit resolving conflicts only, pushed as a fast-forward: chosen, because the branch holds stage 1's commits and their red-first record, and SPEC-083, ADR-083 and the schematic are unchanged on dev since its merge-base (#108).
- D1 (R-a), a fresh branch from dev that cherry-picks the parked commits: rejected, because it meets the same conflicts with one more step and separates the stage-1 record from the commits it measured (#108).
- D2 (R-b), restore the two body lines the move commit edited to dev's planned text, and carry the settings row's change as section 10's T6: chosen, because the body then stays the planned text and section 10 holds every change, as SPEC-091 section 10 does (#108).
- D2 (R-b), keep the body edits: rejected, because the body would then differ from the planned file with no amendment that records the change (#108).
- D3 (R-c), restore ADR-083 to `proposed` with one insert-only note that reads it under ADR-301, and accept it at E4c, by the architect, only after an independent hostile verify reads E4b and E4c ready: chosen, by the ruling stamped 2026-10-03T07:47:12Z (reading 1) (#108).
- D3 (R-c), keep the branch's `accepted`, set before ADR-301 existed: rejected, because no part of SPEC-083 has made the write reachable, and nothing yet proves its guardrails (#108).
- D3 (R-c), reading 2, the acceptance as an owner item at E4c: rejected, by the same ruling. The owner's decision on the write is already ADR-089's (#266). What remains to decide at E4c is whether the guardrails it set are proved, and a hostile verify of E4b and E4c decides that (#108).
- D4 (R-d), the backup beside the private copy, mode 0600, host-only, one at a time, removed by the data-rights erase and restored only by the owner's own import: chosen, because it keeps a batch's backup the batch's own act and out of every standing copy (#108).
- D4 (R-d), the backup in ADR-064's daily copy, in Litestream's replica or in a bucket: rejected, because ADR-064 keeps the collection copy out of its units, and an off-host copy of the owner's collection is a standing copy of it (#108).
- D4 (R-d), keep every take's backup: rejected, because the largest file DeckStreak would keep would then grow by one collection per skip (#108).
- D4 (R-d), DeckStreak restoring the backup: rejected, because a restore reaches the server only by a full upload, which ADR-301 (a) 4 forbids (#108).
- D5 (R-e), one ledger row that stops every declared write class, set by a count that moved or by the owner, cleared only by the owner's authenticated command handler, recording who set it and why, and holding every write while it is set, an undo included: chosen, because ADR-301 (b) says a count that moves "stops the class", and that needs a state to hold it (#108).
- D5 (R-e), no stop at the approval rung, since the owner simply does not confirm: rejected, because (b)'s stop then has no state to hold it. The next confirm would write again into a collection whose counts already disagreed (#108).
- D5 (R-e), a stop the undo ignores: rejected, because an undo is a write too, and a count that moved means the class's model of the collection no longer holds for the inverse either (#108).
- D6 (R-f), the charge's movement on the skip's own study day and the refund's on the undo's recorded study day: chosen, because the ledger's key is then the same on every retry, so a settlement retried on a later day charges once and an undo's refund retried on a later day credits once (#108).
- D6 (R-f), the study day the settlement runs on: rejected, because a settlement that retries across midnight would find a new key and charge twice (#108).
- D7 (R-g), three parts: E4a, the record and the game; E4b, the take's write with its backup, restore drill, counts and stop; E4c, the undo, the take and undo use cases, the bot, the API, the Mini App, the daemon's wiring and ADR-083's acceptance: chosen, because each part is proved before the next makes more of the write reachable (#108).
- D7 (R-g), SPEC-083 in one pull request: rejected, because the record and the game would wait on a write whose guardrails ADR-301 (b) has only just set (#108).
- D7 (R-g), two parts, the record and then the whole write path: rejected, because the take's write, its backup and its counts would land in the same pull request as every caller that can reach them (#108).
- D8 (R37), no dwell time between changes and no band, the change budget staying `SKIP_MAX_CARDS`: chosen, because the class's ceiling is the approval rung. Every batch is the owner's own confirm, and a skip is reversed only by the owner's undo (#108).
- D8 (R37), a fixed dwell between skips: rejected, because R2 already holds a study day to one pending or applied skip not undone. A dwell would only refuse the owner a declaration the rung already approves (#108).
- D8 (R37), a band on the next study day's load, reversing a skip when that day's due count passes a bound: rejected, because the class has no autonomous rung, no trial and no guard metric. Such a band would make a guard metric of the load the skip itself moves, and reverse the owner's own declaration (#108).
- D9 (R38), TLA+ `SkipDayOnce` and Lean `SkipTariff` in E4a, and TLA+ `SkipDayWrite` in E4b, extended with the undo in E4c, each citing #108: chosen, because the two confirms, the take's settlement, the start-up settlement and the undo each check and then act on `skip_days` and the coin ledger. A model enumerates those interleavings, where a test only samples them, and the tariff's price and the amount paid are a total function with bounds (#108).
- D9 (R38), not applicable: rejected, because each of those actors checks a row and then acts on it, and another actor can change the row in between (#108).
- D9 (R38), a new proof of the wallet's floor: rejected, because `tla/WalletFloor` already covers `debit_floored_on` and `refund_on`, and no part of SPEC-083 edits them (#108).
- D10, the skip's settle to `applied` and its charge in one write transaction through the kernel's `BEGIN IMMEDIATE`, the charge keyed per D6: chosen, because no committed state then holds an applied skip without its charge, or a charge without its applied skip (#108).
- D10, the charge first with its stable key, and the settle in a second transaction: rejected, because a crash between the two leaves a charged skip `pending` until a retry repairs it, a state the single transaction never has (#108).
- D11, economy's `tariff.rs` holding the ladder read from `economy.json`, the pure price, and the read of what one skip paid by its source and reference: chosen, because the wallet's ports answer no read by reference, and economy owns the coin ledger (#108).
- D11, a new reader on the wallet: rejected, because `wallet.rs` is SPEC-082's, and no part of SPEC-083 edits it (#108).
- D11, filtering the wallet's movement list: rejected, because a movement carries no reference, so the list cannot name the skip that paid (#108).
- D12, the open lapse (`lapse.rs`) taking the skip set as a parameter that its caller reads from `skip/days.rs`: chosen, because the function is synchronous over a window its caller read, and it opens no connection (#108).
- D12, a skip set carried on the recompute's facts: rejected, because the fold would then read the set for every step, and the two views outside the fold would still need the port, so there would be two ways to reach one set (#108).
- D13, the tariff's price on a take or a retry counting the other applied skips not undone in the skip's calendar month on an earlier study day only: chosen, because the count then depends only on days before the skip's own, so a retry prices the skip as its first attempt did. All 13 cases of the `skip_tariff` golden agree, since each holds its rows on days at or before the skip's day (#108).
- D13, every other applied skip of the month not undone: rejected, because a retry of a free skip would then be charged once a later skip of the month had applied (#108).
- D13, storing the price on the row at take time: rejected, because the row would become a second write site for the tariff, and the refund would then read a row value rather than the sum the ledger records as paid (#108).

## Decision Outcome

Chosen options: D1 to D13 as chosen above, with these rulings.

1. **The slice (D7).** E4a delivers R1, R2's record half, R3's search, day spec and setting, R4 to
   R13, R17 and R33. Its criteria are A1 to A4, A7, A8, A10 to A18, A23, A33, A45 and A46.
   SPEC-083 section 3c names every other criterion with the part that delivers it. Section 10
   records T1 to T15, and section 11 holds A45 and A46.
2. **ADR-083 (D3).** It is restored to `proposed` and gains one insert-only note. ADR-083 is
   accepted at E4c, by the architect, only after an independent hostile verify reads E4b and E4c
   ready. That verify covers the recording fake server (zero uploads on every other path, and only
   the skip's exact changes and their inverse), the restore check before any write, the counts'
   stop, the switch, and the full or one-way sync abort, each red-first. No agent ever drives a
   take, an undo or a sync against the owner's real collection, tests included. The first real
   write is the owner's own skip declaration.
3. **The backup (D4; SPEC-083 R34).** One backup file, beside the private copy, mode 0600 and named
   for its skip, proved by its restore drill before any card changes. It is host-only: never in a
   bucket, never in Litestream's replica and never in ADR-064's daily copy. The older backup is
   removed only after the newer one passes its check. ADR-064's "never" covers standing copies, not
   a write batch's own backup. Adopted by the ruling stamped 2026-10-03T07:47:12Z.
4. **The class's stop (D5; R36).** The row records who set it and why. Only the owner's
   authenticated command handler clears it, and a later part's test proves that no other path does.
   While it is set nothing writes, an undo included. The owner may reshape it by a later appended
   amendment. Adopted by the ruling stamped 2026-10-03T07:47:12Z.
5. **The tariff's days (D6; T3, T4).** The charge is `debit_floored` with source `skip_tariff`, on
   the skip's own study day. The refund is `refund` with source `skip_tariff_refund`, on the undo's
   study day as the row records it. Each takes the skip's id as its reference.
6. **Dwell, band and change points (D8; R37).** None applies. The change budget stays ADR-089's:
   R21's `SKIP_MAX_CARDS`.
7. **The formal decision (D9; R38).** As chosen above.
8. **The tariff's count (D13).** A take or a retry prices the skip at the count of the other applied
   skips not undone in its calendar month on an earlier study day. Adopted by the architect's
   ruling 73 of 2026-10-03.

### Consequences

- Good, because the record, the tariff and every effect on the game land proved, while no line of
  this part can write to the collection.
- Good, because a retried settlement or refund finds the same ledger key, so it moves coins once.
- Good, because ADR-301 (b)'s backup, drill and counts, and (c)'s dwell and band, are now decided
  for the skip day in its own SPEC, before its write is built.
- Bad, because SPEC-083 is now split across three pull requests, and A6, A40 and A44 are whole only
  when both E4b and E4c have landed. The part that lands second moves each row back.
- Bad, because a debit the empty wallet clips to 0 writes a movement of 0, where the predecessor
  wrote none. The golden's comparison reads both as nothing paid.
- Bad, because a tariff narrows a later fine's cap room on the skip's own day, as the predecessor's
  did, while the tariff itself stays outside the cap.

### Confirmation

- SPEC-083's section 3 and section 11 fences: A10 to A18, A45 and A46 red by assertion first, then
  green.
- `formal check --entry tla/SkipDayOnce` and `--entry lean/SkipTariff`, each witness caught.
- The mutation rows S08305 to S08308 and S08320 to S08326.
- `scripts/tests/test_declared_write_classes.py`, which holds ADR-089's notes as they were.

## What would make this wrong

- A measured retry that moves coins twice for one skip. The key would then not be what D6 says it
  is.
- An engine whose restore of a backup is not byte-faithful, so the restore check would prove
  nothing. R34 then needs another check before E4b writes.
- An owner decision to give the skip day an autonomous rung. A dwell, a band and a guard metric
  would then be owed, and R37 would no longer hold.

## More Information

SPEC-083 sections 3c, 10 and 11; ADR-301 parts (b) and (c) and their #518 notes; ADR-089 and its
#518 notes; ADR-083; ADR-064; ADR-037; SPEC-082 R7; `formal/tla/SkipDayOnce/`;
`formal/lean/Formal/SkipTariff.lean`; `docs/schematics/skip-day-record-and-effects.md`; #108; #266;
#514; #518.

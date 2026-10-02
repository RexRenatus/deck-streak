---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The wallet writes each movement in one immediate transaction, over a ledger whose sum is the balance

## Context and Problem Statement

SPEC-082 gives the coin wallet of #106 seven ports (R7): `credit`, `credit_once`, `settle_mint`,
`purchase`, `debit_floored`, `refund` and `debit_capped`. They run from callers that do not wait
for each other: the fold settles each study day's mint, a fine of the discipline wave takes a
capped debit, the shop buys, and a payout settled on a later study day credits once ever. Each port
reads before it writes. A purchase reads the balance to refuse below the price, a capped debit
reads the wallet at the day's start and the day's debits to clip the request, and `credit_once`
reads whether its source and reference were ever paid. R2 holds the balance at the floor
(`WALLET_FLOOR`, 0) under any interleaving, and R4 holds a settled day's mint from falling. Where
does each port's read happen relative to its write, what holds one movement per key, how far does
the once-ever guard look, and how does a day's mint change?

## Decision Drivers

- Two debits that both read a balance of 10 and both pay 10 leave it at minus 10: the floor holds
  only if no write lands between a port's read and its own write.
- The kernel's base is the one place a write begins, with `BEGIN IMMEDIATE` (`Db::write`), so two
  writers serialise on the write lock instead of failing mid-transaction.
- The balance is the sum of every movement (R2), and the predecessor's wallet reads are its parity
  goldens (`coin_balance_before`, `coin_debits_for_day`).
- A key the database refuses survives every caller; a key a caller checks survives only the callers
  that check it (ADR-040 holds the XP ledger's keys in its migration for this reason).
- A payout settled on a later study day (SPEC-074's season node) is still the same payout, while a
  mint, a fine or a purchase may repeat its source and reference on another day.
- No port deletes a movement (R7), and the owner's erase is the only delete.

## Considered Options (the alternatives it was chosen against)

- Every port reads and writes inside ONE `BEGIN IMMEDIATE` transaction from `Db::write`, over a ledger whose sum is the balance: chosen, because the write lock is held from the read to the commit, so no other port can write between them, and the balance has one source (#106).
- A balance column updated in place beside the ledger: rejected, because it is a second record of the same number that an erase, an import or a failed write can leave apart from the ledger's sum, and the goldens judge the sum (#106).
- A floor check before the transaction, then the write in its own transaction: rejected, because it is check then act, so two debits can both pass the check against the same balance and both pay (the TLA+ witness of `tla/WalletFloor`) (#106).
- A deferred transaction (`BEGIN`) that takes the write lock at its first write: rejected, because two ports holding read locks deadlock at the upgrade and one fails mid-transaction, which the kernel's base exists to rule out (#106).
- A trigger that refuses a movement below the floor: rejected, because the clip needs the balance and the day's cap before the movement exists, so the port reads them anyway, and a trigger would hold a second copy of the rule in a migration (#106).
- The once-ever guard read across every study day, inside the port's transaction: chosen, because a payout settled on a later study day must find the earlier one, and the transaction serialises two such credits (#106).
- The once-ever guard as a unique index on (source, reference): rejected, because a mint repeats (`mint`, empty reference) every day, and a per-day source may repeat a reference on another day (#106).
- The once-ever guard checked on its own study day only: rejected, because that is the per-day key the index already holds, so a payout requested on two study days pays twice (the TLA+ witness of `tla/WalletFloor`) (#106).
- `settle_mint` as the one port that updates a movement, the day's one mint: chosen, because the key holds one mint per day and R4 needs it to move, raised on a settled day and following its base on the open day (#106).
- A new movement for each change of a day's mint: rejected, because the key (study day, source, reference) holds one movement, and the ledger would grow at every recompute (#106).
- An insert-once mint: rejected, because R4 raises a settled day's mint at a later recompute and makes the open day's follow its base (#106).
- The coin-adding ports named `deposit` and `deposit_once` in code (`deposit_on`, `deposit_once_on`, `DepositAnswer`): chosen, because docs/LEXICON.md locks `credit` out of a declaration in deck-streak-economy, and `deposit` is the counterpart of the `debit_*` ports (#106).
- Amending docs/LEXICON.md so `credit` is no longer a word `coin` replaces: rejected, because it weakens a lexicon lock to fit one delivery's names, and the lock is the ubiquitous language the probe holds (#106).
- `grant` for the coin-adding movement: rejected, because SPEC-082 already says grant for the freeze port's grant (A12), so one word would name two concepts inside one SPEC (#106).

## Decision Outcome

Chosen option: the ports of `crates/economy/src/wallet.rs` are each one read and one write inside
one `BEGIN IMMEDIATE` transaction, with these rulings.

1. **One transaction, typed.** A port runs on a transaction from `Db::write`: the repository's
   method opens one and commits it, and the form that runs inside a caller's own write (the fold
   of SPEC-071, the shop's purchase of E3) takes that caller's `Transaction`, never a bare
   connection, so no caller can run its read outside the write lock.
2. **The balance is the sum.** `coin_ledger` has no balance column. The balance, the wallet at a
   day's start (the movements of earlier study days) and the day's debits (every negative movement
   of the day, purchases included, R8) are sums read inside the port's transaction.
3. **One movement per key, held by the migration.** `coin_ledger` is unique on (study day, source,
   reference). Every write is an insert that does nothing on that conflict, so a second call of a
   held key writes nothing. A debit whose request is positive writes its movement even when it
   pays nothing, so the key is held and a retry never pays.
4. **The floor.** No port writes a movement that leaves the balance below the floor: a purchase
   refuses below its price plus the floor and writes nothing; a floored debit pays
   `min(amount, max(0, balance - floor))`; a capped debit pays `clip_debit(requested, balance -
   floor, remainder)`; and a lowering of the open day's mint goes no further than the balance above
   the floor.
5. **The once-ever guard.** `credit_once` writes nothing when a movement of its source and
   reference exists on any study day, read in the same transaction as its insert.
6. **`settle_mint` is the only update.** The day's mint is one movement (`mint`, an empty
   reference). With none held it is inserted when positive; on a day the caller reports `closed`
   it becomes the greater of the held and the new mint; on the open day it follows the new mint,
   lowered at most to the floor (ruling 4). No other port updates a movement, and none deletes one.
7. **The coin-adding movement is named `deposit` in code.** The ports R7 calls `credit` and
   `credit_once` are `deposit` and `deposit_once`, with their `_on` forms and their answer
   `DepositAnswer` (`Deposited`, `AlreadyDeposited`), because docs/LEXICON.md locks `credit` out of
   a declaration in deck-streak-economy. The rulings above keep R7's words.

### Consequences

- Good, because two ports never interleave between a read and its write, so the floor, the key and
  the once-ever guard hold for any number of concurrent callers.
- Good, because the balance has one source, which the parity goldens and the owner's export read.
- Good, because a caller that holds a write, the fold or the shop, runs the port inside it, so a
  coin movement lands in the same commit as what it pays for.
- Bad, because each port sums the ledger, a read that grows with its rows. One owner writes a few
  movements a day, so the sum stays small; an index on the study day serves the day's reads.
- Bad, because every port waits for the write lock, so a long write elsewhere delays a purchase.
  The kernel's busy timeout bounds the wait.

### Confirmation

The TLA+ entry `tla/WalletFloor` models the ports as concurrent actors over the ledger and checks
four properties: the balance never falls below the floor, one movement per (study day, source,
reference), at most one `credit_once` movement per (source, reference) on any day, and a settled
day's mint never lowered. Its witnesses (the floor read outside the write transaction, and the
once-ever guard checked on its own day only, among others) are caught by TLC, and the fixed model
is clean at its state floor. The Lean entry for the wallet's rules proves the clip and mint bounds
over `Int`. `wallet_ports::the_wallet_never_goes_negative_under_a_burst` runs concurrent capped
debits and purchases against one file-backed database.

## What would make this wrong

- A port that reads on the pool (`Db::reader`) and writes in a later transaction: the floor would
  hold only by luck. The `Transaction` parameter rules it out at compile time, and the model's
  check-then-act witness shows what it would cost.
- A second writer of `coin_ledger` outside the economy context: `wallet_census` refuses one.
- A caller that reports a settled day as open: its mint could be lowered. The fold settles a day
  once, in order, and never reopens it (ADR-071).

## More Information

Issue #106; SPEC-082 R1, R2, R4, R5, R7, R8 and R18; ADR-040 (the key in the migration), ADR-071
and ADR-072 (a settled day never falls); the schematic `docs/schematics/coin-wallet-and-its-ports.md`.

---
status: "accepted"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A chest is rolled once, from the operating system's generator, and stored with its pity in one write

## Context and Problem Statement

Every study session with real effort earns a chest whose rarity decides its reward, and a pity
system bends the odds toward an Epic and a Legendary after long dry runs (SPEC-081). The
predecessor rolls each chest once, when it is earned, with `random.SystemRandom` (the operating
system's generator), stores the rarity and the payout in the chest's row, and advances the pity
counters beside it; every later recompute reads the row and never rolls again
(`pipeline_layers/loot.py:LootLayer._grant_session_chests`, predecessor `27ee2bc`). Its pure module
takes each uniform draw as an input, so the parity oracle proves the fold from a draw to a rarity
and a payout, and not the draw itself. DeckStreak must choose where the draws come from, when they
happen, and how the chest, its payout and the pity counters are kept consistent, so that no
recompute, restart or tap can roll a chest twice, and nobody can predict a roll.

## Decision Drivers

- The charter's guard for random events: one stored roll per random event (constraint 9).
- The game-economy pack's rule, which the owner adopted: roll once, on the server, from a
  cryptographic source, and never re-roll on a re-sync; the Mini App only animates the stored
  rarity.
- Parity: the predecessor's folds take a 53-bit uniform fraction, compared at percent thresholds
  (`gamification/chests.py:roll_rarity`, `payout_xp`).
- No new dependency without an ADR: `getrandom` is already admitted (ADR-024).
- A pity counter that drifts from its chests would silently move every later chest's odds.

## Considered Options (the alternatives it was chosen against)

- Two draws from `getrandom` when the chest is earned, each a 53-bit fraction in [0, 1), stored with the rarity, the payout and the pity counters in one write — chosen: it is the predecessor's own design with an admitted source, it cannot be predicted or replayed, and one write keeps the counters equal to the chests.
- A pseudo-random generator seeded once by the service — rejected because a known or leaked seed predicts every roll, and a restart that reseeds lets a crash re-roll a chest whose row was not yet written.
- A draw derived from a hash of the session's key — rejected because anyone who knows the reviews knows the rarity before the chest is earned, and the owner could shape a session to land a better roll.
- Rolling when the owner opens the chest — rejected because the pity counters would advance in the order chests are opened rather than earned, an unopened chest swept at the rollover would need a roll of its own, and the predecessor rolls when the chest is earned.
- Rolling in the Mini App — rejected because the server must be the only roller: a client roll can be replayed or forged, and the bot and the Mini App would not agree on one chest.
- The `rand` crate's generator over the operating system's source — rejected because it adds a dependency for what `getrandom::u64` already gives.
- A 64-bit fraction, or a draw scaled straight to a percent — rejected because the predecessor's thresholds are compared on a 53-bit fraction, and another resolution moves the boundary cases the golden pins.

## Decision Outcome

Chosen option: "Two draws from `getrandom` when the chest is earned, each a 53-bit fraction in
[0, 1), stored with the rarity, the payout and the pity counters in one write", because it keeps
the predecessor's design and its parity, needs no new dependency, and gives no one a way to
predict, replay or repeat a roll.

- **The draw.** `crates/quests/src/draw.rs` takes `getrandom::u64()`, keeps its top 53 bits and
  scales them by 2^-53, which gives every multiple of 2^-53 in [0, 1) with equal probability, as
  the predecessor's `random.SystemRandom().random()` does. Its source is a port, so a test injects
  fixed draws and a failing source; production code has one implementation.
- **When.** The recompute's chest step draws when it grants a chest: once for the rarity, then
  once for the payout, in that order, as the predecessor draws them. The daily quests' challenge
  chest draws the same way. The weekly chest is an Epic without a draw.
- **One write.** The chest row (its study day, origin, session start, rarity, payout and state)
  and the pity counters after it are written in one `BEGIN IMMEDIATE` write through the kernel's
  repository base. The row's unique key (study day, origin, session start) is in the migration, so
  a second writer for the same key writes nothing. A draw that fails writes nothing: no chest and no
  counter change, and the next recompute rolls that session.
- **Never again.** Every later recompute reads the stored row. Opening a chest, choosing an Epic's
  prize and the rollover's sweep pay what the row holds; none of them draws.
- **The surfaces.** The API and the bot return the stored rarity, and the Mini App animates it.
  The draw's value is never logged, sent or stored beyond the rarity and payout it decided.

### Consequences

- Good, because a roll cannot be predicted from the reviews, the seed of a generator, or the time a
  chest is opened.
- Good, because the pity counters and the chests are written together, so the counters always
  equal the chests that moved them.
- Good, because the parity oracle proves every fold from a draw to a rarity and a payout, with
  draws exactly on each threshold.
- Bad, because a failed draw delays that chest to the next recompute, which under one daily sync
  (ADR-037) can be the next study day.
- Bad, because the draws themselves cannot be replayed from stored state: a disputed rarity is
  explained by the stored row and the disclosed odds, never by re-deriving the draw.

### Confirmation

SPEC-081's A2 to A6: `goldens/roll_rarity.json` and `goldens/payout_xp.json` over threshold
draws; `goldens/session_chests_granted.json` over the grant step with patched draws; a second
recompute of one study day that rolls nothing; and a failing source that writes no chest and no
counter. The hand-proved row `S08110-ONE-CHEST-PER-SESSION-KEY` proves the migration's unique key.

## What would make this wrong

- The owner decides that a chest may be bought: every rule for sold randomized rewards (store
  guidelines, odds disclosure laws) would then apply to the roll, and the campaign's boundary
  against sold randomness (#172) would have to be revisited first.
- A second process grants chests concurrently with the recompute: the one write and the unique key
  still hold, but the order of the pity updates would need a stated rule.
- The host's generator is unavailable for long periods: chests would then wait, and a bounded
  retry inside one recompute would be worth its cost.

## More Information

SPEC-081 (the chests, their pity and their surfaces); ADR-024 (the admission of `getrandom`);
ADR-071 (the recompute that grants them); the predecessor's `gamification/chests.py` and
`pipeline_layers/loot.py`; the game-economy pack's teaching on rolls; `getrandom`'s `u64`
(Context7 `/rust-random/getrandom`).

## Amendment (2026-10-02)

The one write is the caller's. The chest row and the pity counters after it are written inside
the connection the caller passes, which the caller holds inside its own write (the fold's write of
the study day, opened by the kernel's `Db::write` with `BEGIN IMMEDIATE`), as the progression
ledger's `grant_on` and `settle` are. The chest port never opens, commits or rolls back a write of
its own. As the code holds it (`crates/quests/src/chests.rs`, `crates/quests/src/chest_store.rs`):

- `grant_session_chests_on` and `grant_challenge_chest_on` take `&mut SqliteConnection` and take
  both draws for a chest, the rarity's and the payout's, before anything is written for it.
- A draw's `Err` returns before any write in that step: no chest row and no pity change for the
  session it was taken for, and none for any later session of the request.
- `insert_chest` writes the row with `ON CONFLICT DO NOTHING` against the migration's unique
  key, and `set_pity` runs only when the insert took a row, so a held key moves no pity.
- Chests granted earlier in the same step stay in the caller's open write, each with its pity.
  Whether that write commits is the caller's decision: committing keeps them, rolling back keeps
  none, and either way every stored chest has its pity and the failed session is left for a
  later recompute to roll.
- A store error is answered the same way, with the write open. Between a chest's insert and its
  pity it leaves the chest without its pity in that write, so a caller rolls back on one.

### Considered Options (the alternatives it was chosen against)

- The chest and its pity in the caller's write - chosen: one commit keeps the counters equal to
  the chests, and the fold already holds a write for the study day.
- A separate write of the port's own after the fold - rejected because a crash between the two
  writes re-rolls the chest or loses the pity: the fold's day is stored and the chest is not, or
  the chest is stored and its pity is not.
- A deferred queue of chest grants, written later - rejected because it needs a second store,
  and the queue and the chests would then have to be kept consistent across their own writes.

### Confirmation

`formal/tla/ChestRolledOnce` checks that recomputes with draws that fail, under any order of the
callers' writes and either end of each, store at most one chest per key, keep the pity equal to
the stored chests and store no chest for a failed draw; each property has a witness the checker
catches (no unique key, the pity in a write of its own, the insert before the draw).
`formal/lean/Formal/Chest.lean` proves the roll's guarantees, the Epic odds' ceiling and the
payout's cap for every input, and its vectors tie the port to the Rust rules
(`crates/quests/tests/formal_vectors_chest.rs`).

# SPEC-103: a fine is recorded once beside its coins, and refunded once when it is reversed

- **Wave:** W5. **Issue:** the fine and its reversal that #110, #111, #113 and #279 share (epic #6).
  **Context(s):** `deck-streak-economy` (the penalty ledger, the fine port, the reversal port, the
  listing of a study day's standing fines and the setter of the pass surcharge's end);
  `deck-streak-coordination` (the data-rights registry only). No other context changes: every caller
  of these ports arrives with its own SPEC (SPEC-104, SPEC-106, SPEC-108).
- **Decided by:** ADR-103 (this SPEC's: a fine is one economy port that records the debited amount
  beside its coin movement, and a reversal refunds that amount once), ADR-012 (the parity oracle
  proves the math) and ADR-071 (a settled study day's value never falls).
- **Prerequisites:** SPEC-082 (the coin ledger, the capped debit and the refund port), SPEC-021 (the
  data-rights rule) and SPEC-020 (migrations). **Mutation band:** `S10300-S10399`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-103.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` 26263de, `crates/economy/src/` holds only `lib.rs`; SPEC-082 (planned)
  builds the coin ledger and the capped debit `debit_capped(day, source, reference, requested)`, and
  its section 5 says it "raises no fine and keeps no penalty ledger: the capped debit waits for the
  first fine (#110), the confession (#111) and contract breaches (#113)". The context map's register
  already gives the predecessor's `penalty_ledger` to `economy`.
- **The rule that binds it.** Coins are the only confiscable stake (CHARTER), and a false fine is
  worse than a missed fine: every fine must be reversible, and a reversal must return exactly what
  was taken, once.
- **What is ported.** The predecessor's fine path,
  `pipeline_layers/economy.py:EconomyLayer._debit_fine` at `27ee2bc`: it reads the wallet, the daily
  loss cap over the wallet at the day's start, and the day's debits, clips the request
  (`gamification/economy.py:clip_debit`), records the DEBITED amount in `penalty_ledger` (a fully
  forgiven fine records 0, so a later refund can never mint coins that were never taken), and writes
  the coin movement `fine` only when the debit is positive. The ledger's functions
  `database.py:GamifyStore.insert_penalty` (once per key), `GamifyStore.reverse_penalty` (true only
  for the call that reversed it), `GamifyStore.penalty_amount` and
  `GamifyStore.penalties_total_for_day` (the standing fines of a study day, reversed ones left out).
- **Measured in the predecessor.** Its three fine references are `tw:<event>` (a doomscroll
  defection), `confess:<event>` (a confession) and `contract:<contract>:<day>` (a contract breach),
  and its two refund sources are `grace_refund` (a defection closed within its grace) and
  `pardon_refund` (the monthly pardon). The ledger's `kind` column holds one value, `tripwire`, on
  every path, so it carries no information.
- **Traps a hand port falls into.**
  - The penalty row and the coin movement are two writes in the predecessor; a crash between them
    leaves a fine recorded with no coins taken, or coins taken with no record to refund.
  - The predecessor keys a penalty by its study day, kind and reference, and reverses by kind and
    reference: a reference fined again on another study day would be a second row, and one reversal
    would mark both.
  - A reversal that refunds the requested amount instead of the recorded one mints coins whenever
    the loss cap or the wallet floor forgave part of the fine.
- **Corrections to the issues.** None of the four issues names the ledger's key: this SPEC keys a
  fine by its reference alone, so a reference is fined at most once ever and a reversed fine can
  never be levied again (#279's "a cancelled fine stays cancelled"). The `kind` column is dropped.
- **What the parity oracle proves.** The debited amount, the forgiveness flag, the penalty row and
  the coin movement of a fine over a synthetic wallet and day; the reversal's once-only answer; a
  study day's standing total (section 7).

## 2. Requirements

The ledger

R1. `penalty_ledger` holds one row per fine: its study day, its reference (an opaque token,
    `^[a-z0-9][a-z0-9:._-]*$`, never a calendar date), the amount debited (a whole number, 0 or
    more), whether any part was forgiven, whether it is reversed, the study day and reason of its
    reversal when it is, and `created_at`. It is `STRICT`, unique on the reference, and created by
    `migrations/010301_economy_penalty_ledger.sql` (SPEC-020 R15 and R18).
R2. Only the economy context writes `penalty_ledger`, and no other crate names it in a query.

The fine port (#110, #111, #113)

R3. `fine(day, reference, requested)` runs in one `BEGIN IMMEDIATE` write through the kernel's
    repository base: when a row of the reference exists it writes nothing and answers that row's
    amount, forgiveness and reversal; otherwise it clips the request exactly as SPEC-082 R5's capped
    debit does (the wallet, the cap over the wallet at the day's start, and the day's debits),
    writes the coin movement of source `fine` and the reference when the debited amount is positive,
    and writes the penalty row with the debited amount, in the same transaction. Its answer and its
    writes equal the golden of `_debit_fine`.
R4. A request of 0 or less debits nothing and records a row of amount 0, as the predecessor's clip
    answers 0 for it; a caller that has nothing to fine does not call the port.

The reversal port (#110, #113, #279, and the revision of ADR-104)

R5. `reverse_fine(reference, refund_day, reason)` runs in one `BEGIN IMMEDIATE` write: when the
    reference has a row not yet reversed, it marks the row reversed with the refund day and reason
    and, when the recorded amount is positive, writes one coin movement of that amount through
    SPEC-082's refund port on the refund day, under the reason's source; it answers the amount
    refunded. A reference with no row, or a reversed one, writes nothing and answers 0. It equals
    the golden of `reverse_penalty` over each case's sequence.
R6. The reason is one of a closed set, each with its own coin source: `grace` (`grace_refund`),
    `pardon` (`pardon_refund`), `revision` (`revision_refund`) and `smoke_bomb`
    (`smoke_bomb_refund`). The first two are the predecessor's sources; the last two are new with
    ADR-104's revision and #279's spend.
R7. A reversed fine is never levied again: `fine` of its reference answers the reversed row and
    writes nothing (R3), whatever the study day it names.

Reading the fines

R8. `standing_fines(day)` lists the fines of a study day that are not reversed, each with its
    reference and amount, and `standing_total(day)` is their sum, equal to the golden of
    `penalties_total_for_day`. `fines_since(day)` lists every fine of a study day from `day` on,
    reversed or not, with its study day, for the revision step (ADR-104) and the discipline screens.

A fine's surcharge

R9. `extend_surcharge(until)` sets the scroll pass's surcharge end in `economy_state` (SPEC-082 R12)
    to the later of its stored end and `until`, in one write; it never shortens an end and never
    touches the pass's own end. It is the one writer of the surcharge's end, called by the rung-2
    fine (#110), and the price rule stays SPEC-082 R11's.

Rights

R10. `penalty_ledger` is declared once in economy's data-rights port as exported and erased, under
    the category `fines` in `privacy.json` and `PRIVACY.md`, with its own-tables row in the context
    map and a seeded row in coordination's symmetry test (SPEC-021).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a fine's debited amount, forgiveness, penalty row and coin movement equal the golden of `_debit_fine` for every case (examined count reported, zero refused) | `the_fine_matches_the_parity_golden` |
| A2 | the reversal's once-only answers and a study day's standing total equal the golden of `reverse_penalty` and `penalties_total_for_day` over each case's sequence | `the_reversal_and_the_days_total_match_the_parity_golden` |
| A3 | a fine of one reference requested twice, on one study day or two, writes one row and one movement, and the second call answers the first row | `a_fine_of_one_reference_is_recorded_once` |
| A4 | the penalty row and its coin movement are written in one transaction: a write that fails after the movement leaves neither | `a_fine_and_its_coins_are_written_together` |
| A5 | a fine the cap or the floor forgives in full records 0 and writes no movement, and its reversal refunds 0 and writes no movement | `a_forgiven_fine_records_zero_and_refunds_zero` |
| A6 | a reversal refunds the recorded amount once, on its refund day, under its reason's source, and a second reversal refunds nothing | `a_second_reversal_refunds_nothing` |
| A7 | a reversed fine is never levied again: a later fine of its reference, on any study day, writes nothing | `a_reversed_fine_is_never_levied_again` |
| A8 | a burst of concurrent fines and purchases on one study day never debits past the cap's remainder or below zero | `concurrent_fines_never_pass_the_cap_or_the_floor` |
| A9 | each refund reason writes its own coin source, and no other source is accepted | `each_refund_reason_writes_its_own_source` |
| A10 | a study day's standing fines list each unreversed fine, and the list is empty once each is reversed | `the_standing_fines_of_a_day_are_listed` |
| A11 | no crate but economy names `penalty_ledger` in a query, and no migration but economy's; a planted fixture that does is refused (examined count reported) | `only_the_wallet_writes_the_penalty_ledger` |
| A12 | economy's data-rights port exports the penalty ledger, and after an erase it is empty | `an_erase_empties_the_penalty_ledger` |
| A13 | a surcharge end is extended to a later end, kept against an earlier one, and the pass's end is untouched | `the_surcharge_end_only_moves_later` |

```acceptance
A1: cargo test -p deck-streak-economy --test fine_goldens -- --exact the_fine_matches_the_parity_golden
A2: cargo test -p deck-streak-economy --test fine_goldens -- --exact the_reversal_and_the_days_total_match_the_parity_golden
A3: cargo test -p deck-streak-economy --test fine_ports -- --exact a_fine_of_one_reference_is_recorded_once
A4: cargo test -p deck-streak-economy --test fine_ports -- --exact a_fine_and_its_coins_are_written_together
A5: cargo test -p deck-streak-economy --test fine_ports -- --exact a_forgiven_fine_records_zero_and_refunds_zero
A6: cargo test -p deck-streak-economy --test fine_ports -- --exact a_second_reversal_refunds_nothing
A7: cargo test -p deck-streak-economy --test fine_ports -- --exact a_reversed_fine_is_never_levied_again
A8: cargo test -p deck-streak-economy --test fine_ports -- --exact concurrent_fines_never_pass_the_cap_or_the_floor
A9: cargo test -p deck-streak-economy --test fine_ports -- --exact each_refund_reason_writes_its_own_source
A10: cargo test -p deck-streak-economy --test fine_ports -- --exact the_standing_fines_of_a_day_are_listed
A11: cargo test -p deck-streak-economy --test fine_census -- --exact only_the_wallet_writes_the_penalty_ledger
A12: cargo test -p deck-streak-economy --test fine_rights -- --exact an_erase_empties_the_penalty_ledger
A13: cargo test -p deck-streak-economy --test fine_ports -- --exact the_surcharge_end_only_moves_later
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and game-economy packs stay enforced,
and no row is deferred or lifted for this delivery, so the private wiring does not change when it
merges.

| id | criterion | decided by |
|---|---|---|
| B1 | the table this delivery creates is declared under a category with its data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010301_economy_penalty_ledger.sql` and `crates/economy/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | only coins are confiscable, and the loss cap, the fine's share and the zero floor still equal the reference economy, over `economy.json`'s `coins` and `unconfiscable` sections | the game-economy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/economy/src/fines.rs` | `deck-streak-economy` | added: the penalty ledger's repository, the fine and reversal ports, the reasons, the listings |
| `crates/economy/src/lib.rs` | `deck-streak-economy` | changed: the module above |
| `crates/economy/src/wallet.rs` | `deck-streak-economy` | changed: the capped debit and the refund take a transaction, so a fine and a reversal write in one; the surcharge's port (R9) |
| `crates/economy/src/data_rights.rs` | `deck-streak-economy` | changed: `penalty_ledger`, exported and erased |
| `migrations/010301_economy_penalty_ledger.sql` | `deck-streak-economy` | added: the table, `STRICT`, unique on the reference |
| `crates/economy/tests/fine_goldens.rs` | `deck-streak-economy` | added: A1, A2 |
| `crates/economy/tests/fine_ports.rs` | `deck-streak-economy` | added: A3 to A10, A13 |
| `crates/economy/tests/fine_census.rs` | `deck-streak-economy` | added: A11 |
| `crates/economy/tests/fine_rights.rs` | `deck-streak-economy` | added: A12 |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the penalty ledger joins economy's entry |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `penalty_ledger` |
| `tools/parity-oracle/registry/spec_103.py` | repo | added: this SPEC's registrations (SPEC-029's registry) |
| `tools/parity-oracle/goldens/debit_fine.json` | repo | added: the golden of `pipeline_layers/economy.py:EconomyLayer._debit_fine` (adapter over a temporary store database) |
| `tools/parity-oracle/goldens/penalty_reversal.json` | repo | added: the golden of `database.py:GamifyStore.insert_penalty`, `reverse_penalty`, `penalty_amount` and `penalties_total_for_day` (adapter over a temporary store database) |
| `scripts/mutation-rows.d/S10300-S10399.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `penalty_ledger` |
| `privacy.json` | repo | changed: the category `fines` |
| `PRIVACY.md` | repo | changed: one line for the category `fines` |
| `docs/schematics/fine-verdict-and-refund.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-103-a-fine-is-recorded-once-beside-its-coins-and-refunded-once-when-reversed.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-103.md` | docs | added |
| `changelog.d/feat-economy-fines-103.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It raises no fine: the doomscroll rail and the confession call the fine port (#110, #111), and a
  contract breach does (#113).
- It decides no revision: which verdicts are re-judged, for how long and on what evidence is the
  discipline wave's (ADR-104, #110, #113).
- It spends no smoke bomb: the spend reverses a night's standing fines through this port (#279).
- It grants no pardon and no grace: both reverse through this port with their features (#114, #110).
- It shows no fine on any screen or in any message: the discipline screens and the digest do (#110,
  #129).
- It imports none of the predecessor's penalty rows (#61).

## 6. Risks

- **A reversal mints coins.** It refunds the recorded amount, never the requested one (R5), and a
  forgiven fine records 0 (A5); the golden's cases include a fine the cap forgives in part.
- **A fine is taken twice by a retried verdict.** The migration's key on the reference (S10301) and
  A3 hold one row per reference across study days.
- **A crash leaves coins taken with no record to refund.** Both writes share one transaction (A4).
- **A second writer grows beside the port.** Detected by A11's census.

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_103.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch milliseconds; no golden holds a calendar date.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/debit_fine.json` | `pipeline_layers/economy.py:EconomyLayer._debit_fine` | adapter | a stand-in layer over a temporary store database seeded with the case's synthetic movements on the study day and the days before it; returns the debited amount, the forgiveness flag, the penalty row and the movements written. Cases: a request under the cap's remainder, one equal to it, one coin over it, a request over the wallet, an empty wallet, a request of 0, and a second fine of one reference |
| `goldens/penalty_reversal.json` | `database.py:GamifyStore.insert_penalty`, `reverse_penalty`, `penalty_amount` and `penalties_total_for_day` | adapter | a temporary store database driven through each case's sequence of inserts and reversals; returns each call's answer and the day's standing total after each step. Cases: a reversal of a fine never booked, a first and a second reversal, a fine of amount 0, and two fines on one day with one reversed |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `penalty_ledger` | `economy` | `migrations/010301_economy_penalty_ledger.sql` (SPEC-103) | `penalty_ledger`, one row per fine, its day as an epoch day, its reference unchanged and its single-valued kind dropped; a reversed row carries no reversal day or reason, which the import leaves empty | exported and erased |

## 9. Mutation rows

The band is `S10300-S10399`, in `scripts/mutation-rows.d/S10300-S10399.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S10301-ONE-FINE-PER-REFERENCE` | `migrations/010301_economy_penalty_ledger.sql` | the unique key on the reference, a key held in the migration (a script-mutation row with a cargo killer) | `fine_ports::a_fine_of_one_reference_is_recorded_once` |
| `S10302-A-FINE-RECORDS-THE-DEBIT` | `crates/economy/src/fines.rs` | the row records the debited amount, not the requested one | `fine_ports::a_forgiven_fine_records_zero_and_refunds_zero` |
| `S10303-A-REVERSAL-REFUNDS-ONCE` | `crates/economy/src/fines.rs` | a reversed row refunds nothing again | `fine_ports::a_second_reversal_refunds_nothing` |
| `S10304-A-REVERSED-FINE-STAYS-REVERSED` | `crates/economy/src/fines.rs` | a fine of a reversed reference writes nothing | `fine_ports::a_reversed_fine_is_never_levied_again` |
| `S10305-A-FINE-PASSES-THE-CLIP` | `crates/economy/src/fines.rs` | the request passes through the capped debit's clip | `fine_goldens::the_fine_matches_the_parity_golden` |
| `S10306-A-REVERSAL-USES-ITS-SOURCE` | `crates/economy/src/fines.rs` | each reason's coin source | `fine_ports::each_refund_reason_writes_its_own_source` |
| `S10307-A-SURCHARGE-NEVER-SHORTENS` | `crates/economy/src/wallet.rs` | the later of the two ends is kept | `fine_ports::the_surcharge_end_only_moves_later` |

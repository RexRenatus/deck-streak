# Red-first record: SPEC-082

This record is E1's, the first of SPEC-082's three pull requests. Part 1 of E1 delivers A1, A2, A3
and A10 (the wallet's goldens and the constants); the later parts of E1 add A4 to A6, A8, A9, A14
and A19, and E1b and E3 add their criteria's lines when they move them back into the acceptance
fence (SPEC-082 section 3c).

The drafts at da0b1cd compiled and answered nothing: every constant 0, a mint, a cap and a fine of 0
and a clip of `(0, false)`.

The order of work: the SPEC moved out of `docs/specs/planned/` and given its section 3c; the
registry and the goldens; the wallet's tests beside inert drafts that compiled and answered nothing;
and their implementation.

```red-first
A1: red at da0b1cd: assertion `left == right` failed: the mint of {"base_xp":25}; left: Number(0), right: Number(1)
A1: green at fb331f6
A2: red at da0b1cd: assertion `left == right` failed: the cap of {"wallet_at_rollover":4}; left: Number(0), right: Number(1)
A2: green at fb331f6
A10: red at da0b1cd: constants.COIN_MINT_XP_DIVISOR: ours 0, the predecessor's 25
A10: green at fb331f6
```

## Part 2, 2026-10-01: the wallet's ports

Part 2 of E1 delivers A3, A4, A5, A6, A8, A9, A14 and A19: the day-start wallet and the day's debits
against their goldens, the seven ports over the coin ledger, the census of who names the ledger, and
the ledger's erase. A3's test was written in this part, beside the others, so the paragraph above
that counts A3 in part 1 means its golden, registered there.

The red commit is 6280965, the tests beside a stub of the same public API whose bodies never touch
the database: a credit, a once-ever credit and a refund answered `AlreadyCredited`, both debits
`NothingRequested`, a settle `Settled(0)`, a purchase `Refused(NotPositive)` and every sum 0. Each
criterion was run there, selecting its own test, and failed by assertion, not by a compile error, a
missing fixture or an empty selection: over the economy crate's tests 3 passed, 8 failed and none was
ignored, the 3 being part 1's goldens. The green commit is f047f9d, the seven ports over
`coin_ledger`; it changes no test in this crate.

Two test files changed after the red commit, and neither is a criterion's:

- `crates/coordination/tests/relight_order.rs`, at f047f9d: its register of every `static`
  coordination links gains the economy data-rights port's, from 14 entries to 15. Without it,
  `the_route_keeps_no_per_day_failure_state_a_give_up_could_read` refused the new static.
- `crates/economy/tests/formal_vectors_wallet.rs`, added after green with the Lean proof: it checks
  `clip_debit` and `mint_for_base_xp` against the proof's vectors. It is a parity check of code
  that was already green, so it has no red commit.

After them, the rename of ADR-308 ruling 7 (commit 33998c7) changed two criteria's test files,
`crates/economy/tests/wallet_ports.rs` and `crates/economy/tests/wallet_rights.rs`, in the names
they call and compare only: `credit` and `credit_once` became `deposit` and `deposit_once`, and
`CreditAnswer::Credited` and `CreditAnswer::AlreadyCredited` became `DepositAnswer::Deposited` and
`DepositAnswer::AlreadyDeposited`. No value, test name or selection moved. The lines below are as
measured, in the names of their commits.

Then cargo-mutants, over the diff from dev's tip to 8cc35ec, missed ten of the economy crate's
mutants. Five tests added after 8cc35ec kill seven of them, and they are MUTATION COVERAGE, not red
first: the code they observe was already green, and the commit that adds them changes no production
file. They are `wallet_ports::the_balance_is_the_sum_of_every_movement`,
`wallet_ports::a_purchase_of_the_whole_balance_is_bought_once`,
`wallet_ports::a_retried_debit_answers_what_it_paid`,
`wallet_ports::a_capped_debit_pays_no_more_than_the_days_cap_left` and
`wallet_rights::the_economy_declares_the_ledger_erased_and_the_row_reset`. The other three turn
`- WALLET_FLOOR` into `+ WALLET_FLOOR` with a floor of 0, and
`scripts/mutation-equivalent.d/deck-streak-economy.json` records them as equivalent.

```red-first
A3: red at 6280965: assertion `left == right` failed: the wallet before day 20000 of [(19999, "mint", "", 40), (20000, "mint", "", 25), (20000, "shop", "pass:1", -40), (20000, "fine", "a", -15), (20000, "quest", "q", 10), (20001, "fine", "b", -9), (19998, "shop", "freeze:1", -150)]; left: Number(0), right: Number(-110)
A3: green at f047f9d
A4: red at 6280965: assertion `left == right` failed; left: AlreadyCredited, right: Credited(120)
A4: green at f047f9d
A5: red at 6280965: assertion `left == right` failed; left: [AlreadyCredited, AlreadyCredited, AlreadyCredited, AlreadyCredited], right: [Credited(15), AlreadyCredited, Credited(15), NotPositive]
A5: green at f047f9d
A6: red at 6280965: assertion `left == right` failed; left: [Settled(0), Settled(0), Settled(0), Settled(0), Settled(0), Settled(0), Settled(0), Settled(0)], right: [Settled(10), Settled(25), Settled(15), Settled(15), Settled(30), Settled(30), Negative, Settled(0)]
A6: green at f047f9d
A8: red at 6280965: assertion `left == right` failed; left: AlreadyCredited, right: Credited(30)
A8: green at f047f9d
A9: red at 6280965: assertion `left == right` failed: the wallet, the data-rights port and economy's migration name the table, and nothing else; left: {"crates/economy/src/data_rights.rs", "migrations/008201_economy_wallet_and_shop.sql"}, right: {"crates/economy/src/data_rights.rs", "crates/economy/src/wallet.rs", "migrations/008201_economy_wallet_and_shop.sql"}
A9: green at f047f9d
A14: red at 6280965: assertion `left == right` failed; left: AlreadyCredited, right: Credited(50)
A14: green at f047f9d
A19: red at 6280965: assertion `left == right` failed; left: [AlreadyCredited, AlreadyCredited, AlreadyCredited, AlreadyCredited, AlreadyCredited], right: [Credited(25), AlreadyCredited, AlreadyCredited, Credited(25), Credited(10)]
A19: green at f047f9d
```

## Addendum, 2026-10-02: a test added after its code (round 1 of PR #543)

MUTATION COVERAGE, not red-first. `wallet_ports::a_refund_on_a_later_day_of_a_held_key_writes_its_own_movement`
was added at `b48aabde`, after the code, to guard A19 and R7: a refund on a later study day of a key
held earlier writes its own movement. Its cases are the three seed members K-009 (a deposit, then a
refund), K-021 (a once-ever deposit, then a refund) and K-033 (a refund, then a refund). It is green at
the round's base `2e56a41f` and at its own commit, and it was never red before the code. It is proved
by a plant: with `refund_on` routed through `deposit_once_on`, the test reads red by assertion at
`wallet_ports.rs:245` and the frozen key population escapes 3 of 49 members. No fence line is added:
A19's red and green above stand.

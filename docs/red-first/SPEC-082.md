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

## E1b, 2026-10-02: the fold's mint and the wallet's history

E1b delivers A7, A15's wallet arm, A18 and A20, which SPEC-082's amendment moves back into the
acceptance fence. The red commit is 872102b, the tests beside stubs of the same public API: a mint
step registered in its phase whose evaluation writes nothing, a wallet route that answers an empty
object, a page of movements that is always empty, and a header and a history screen that render no
wallet. Each criterion was run there, selecting its own test, and failed by assertion, not by a
compile error, a missing fixture or an empty selection. The green commit is 46fc2a7.

The same red commit also held three tests that are not a criterion's, each red by assertion there
and green at 46fc2a7:

- `wallet_mint::the_mint_commits_or_rolls_back_with_the_days_write`, at `wallet_mint.rs:423:9`:
  `fail true: the ledger on the fold's own write`, left: 0, right: 13.
- `wallet_ports::the_movements_come_newest_first_a_page_at_a_time`, the economy page test, at
  `wallet_ports.rs:643:5`: left: [], right: the twenty movements of the first page, newest first,
  from (13, 22) to (10, 21). It is the killer of row S08215, and it runs the page's constant
  statement against a migrated SQLite file.
- The daemon's `the_recompute_fold_registers_the_analytics_xp_and_streak_steps_in_their_phases`, at
  `wiring.rs:688:9`: the registered steps lacked `(CoinMint, "economy.coin_mint")`; 11 passed and 1
  failed.

The whole web run at 872102b read 3 test files failed and 29 passed (32), and 9 tests failed and 149
passed (158). At 46fc2a7 it read 32 files and 158 tests passed.

```red-first
A7: red at 872102b: assertion `left == right` failed: backfilled day 20000: the coins the day holds against the golden mint of its base 100; left: 0, right: 4
A7: green at 46fc2a7
A15: red at 872102b: assertion `left == right` failed; left: Object {}, right: Object {"balance": Number(103), "loss_cap": Number(30), "loss_cap_left": Number(25), "movements": Array [Object {"amount": Number(-5), "id": Number(4), "source": String("fine"), "study_day": String("2025-01-14")}, Object {"amount": Number(8), "id": Number(3), "source": String("mint"), "study_day": String("2025-01-14")}, Object {"amount": Number(40), "id": Number(2), "source": String("mint"), "study_day": String("2025-01-13")}, Object {"amount": Number(60), "id": Number(1), "source": String("payout"), "study_day": String("2025-01-12")}], "next": Null, "study_day": String("2025-01-14")}
A15: green at 46fc2a7
A18: red at 872102b: TestingLibraryElementError: Unable to find role="link" and name "Coins: 103"
A18: green at 46fc2a7
A20: red at 872102b: AssertionError: expected [] to deeply equal [ '4', '3', '2' ]
A20: green at 46fc2a7
```

The green commit 46fc2a7 changed four test files. None of them removes an assertion, and only the
first changes a measured quantity:

- `crates/coordination/tests/wallet_mint.rs`: the savepoint test,
  `the_mint_commits_or_rolls_back_with_the_days_write`, now measures the current day's own
  movement, and production is unchanged. Its first green run read left: 17, right: 13, seen
  [(349, 17, 4)]: the fold's settle of the day before D0, a day with no reviews, holds the XP row
  `backlog_zero` of 100 and mints 4 for it, and that mint commits in the settle's own write, before
  the current day's write. Inside the current write the ledger read [(19999, mint, 4), (20000, mint,
  13)]. The predecessor's `_recompute_day` writes the mint for every day it recomputes, so the
  settle's mint is right and the test's premise, that only D0 mints, was not. Inside the write,
  `SELECT COALESCE(SUM(delta), 0) FROM coin_ledger` became the same sum `WHERE study_day = ?1` bound
  to D0; outside it, and after the fold, `SqliteWallet::new(..).balance()` became `minted(db, D0)`,
  the day's movements read through the wallet. The assertions keep their operands and their
  outcome: `assert_eq!(inside, mint, "fail {fail}: the ledger on the fold's own write")` became
  `assert_eq!(inside, mint, "fail {fail}: the day's mint on the fold's own write")`;
  `assert_eq!(outside, 0, "fail {fail}: the balance a second connection reads")` became
  `assert_eq!(outside, 0, "fail {fail}: the day's mint a second connection reads")`; and
  `assert_eq!(balance, 0, ...)` and `assert_eq!(balance, mint, ...)` became `assert_eq!(held, 0,
  ...)` and `assert_eq!(held, mint, ...)` with `held = minted(&db, D0)`. It is not a criterion's
  test: A7's is `the_mint_reads_the_settled_days_final_base`.
- `crates/api/tests/wallet_routes.rs`, `crates/coordination/tests/wallet_mint.rs` and
  `crates/economy/tests/wallet_ports.rs`: 872102b committed them unformatted, and `cargo fmt --all`
  at green changed them in 1, 5 and 2 hunks, format only (46fc2a7's diff of `wallet_mint.rs` reads 6 hunks, plain and with -w: those 5 and the `Probe` step's body, which is the savepoint test's edit alone). rustfmt run over each file as 872102b holds
  it gives 46fc2a7's file byte for byte for `wallet_routes.rs` and `wallet_ports.rs`; for
  `wallet_mint.rs` the only lines that differ are the savepoint test's edits named above.
- `web/app/tests/a11y.spec.ts`: the accessibility run's page now answers `/api/wallet` with a
  fixed wallet, so the header on every screen and the `/wallet` screen are audited with a balance,
  two movements and an older page. It adds a route stub and no assertion.

MUTATION COVERAGE, not red-first. cargo-mutants, over the diff from the merge-base 340d896 to
276bc76, missed `crates/economy/src/wallet.rs:191:31: replace > with >= in SqliteWallet::movements`,
the page's look-ahead. `wallet_ports::a_page_that_holds_the_last_movement_names_no_next_page` was
added at df6f05c, after the code, to observe it: twenty movements, spelt literally, fill one page
that names no next page, and a twenty-first makes the page name its last movement. It is green at
its own commit and was never red before the code. It is proved by a plant: with `>` turned to `>=`
at `wallet.rs:191`, it reads red by assertion at `wallet_ports.rs:698:5`, `a page that holds the
last movement names no next page`, left: Some(1), right: None, and the file restored with its
sha256 equal reads green. Rows S08216 and S08217 pin the look-ahead and the page size's literal
with it as their killer. No fence line is added for it.

MUTATION COVERAGE, not red-first. cargo-mutants over the diff from 340d896 to de272f4, one package
at a time, missed five mutants that no test of their own package observed. Three tests were added
at 48db384, after the code; each is green at its own commit and was never red before the code, and
each is proved by a plant that reads red by assertion and green again with the source restored to
its sha256:
- `wallet_routes::a_wallet_that_cannot_be_read_answers_500_with_a_reason_code_alone`, for
  `wallet_routes.rs:123:5: replace unreadable -> Response with Default::default()`: red at
  `wallet_routes.rs:297:5`, left: 200, right: 500.
- `wallet_mint::the_fold_reports_the_mint_step_by_its_name_in_the_coin_mint_phase`, for
  `mint.rs:26:9: replace <impl DayStep for MintStep>::name -> &'static str` with `""` and with
  `"xyzzy"`: red at `wallet_mint.rs:477:5`, left: [(CoinMint, "")] and [(CoinMint, "xyzzy")],
  right: [(CoinMint, "economy.coin_mint")].
- `wallet_mint::the_wallet_view_leaves_what_the_days_debits_spare_of_its_loss_cap`, for
  `wallet_view.rs:41:35: replace - with +` and `replace - with /`: red at `wallet_mint.rs:512:5`,
  `a fine of 5 leaves 25 of the day's cap of 30`, left: (95, 30, 35) and (95, 30, 6), right:
  (95, 30, 25).

MUTATION COVERAGE, not red-first. Stryker's run over the seven web files at df6f05c left 20
survivors and 6 uncovered mutants of the wallet's body parser and its screen. bfd0f03 adds six
tests after the code: the screen's headings and back link, its loading, refusal and unavailable
states, an older page's alert clearing, and the parser's refusal of a malformed date, list or
movement. Four mutants no input can tell apart are recorded as equivalent in
`scripts/mutation-equivalent.d/miniapp.json`, each with its reason. One survivor needed a change
to production order, at 64a312f: the screen's effect set the answer before the lines it shows, so a
mutant that took every answer for ok (`if (answer.kind === 'ok')` planted as `if (true)`) left the
alert on screen and failed only by an unhandled rejection, and all 14 tests passed. With the lines
set first, the same plant fails 2 tests by `Unable to find role="alert"`, and the file restored
with its sha256 equal reads green. No fence line is added for any of them.

A post-GREEN test edit, not red-first. edadd79, after the code, gives the parser's two refusal tests
a positive control: each first asserts that the untouched body parses to its real values, so every
refusal below it is the refusal of its one changed field. A `parseWallet` planted to return null for
every body fails both, and the file restored with its sha256 equal reads green. An assertion is
added to each and none is removed; production is unchanged. No fence line is added.

MUTATION COVERAGE at the fix round, not red-first. Five commits (8b0b02d2, 13c467be, 377a0d2a,
5d03fe1c, ec638f64) add tests and rows after the code, each green at the base, and no fence line is
added for any of them. The package at 5d03fe1c read `examined 14 plant(s): survived 3, killed 11,
void 0`. The eleven killed plants are held by the rows S08218 to S08228 in
`scripts/mutation-rows.d/S08200-S08299.json`, and their prove line reads `rows: examined 11:
killed 11, survived 0, void 0`. The three survivors are recorded here, none with a test or a
production seam:

- M4 (mint.rs `savepoint.commit().await?`) and O1 (mint.rs `let mut savepoint =
  write.begin().await?;`): EQUIVALENT-BY-CONSTRAINT-AND-ENGINE. The constraint is
  `Db::write`'s `begin_with("BEGIN IMMEDIATE")` at crates/kernel/src/db.rs:103, which makes the
  mint's `write.begin()` a nested SAVEPOINT and `savepoint.commit()` a RELEASE inside an open write
  transaction. The engine's rule is that a nested RELEASE commits nothing and a deferred
  foreign-key violation surfaces only at the outermost COMMIT, so neither `?` arm is reachable from
  any state the code controls. The evidence is a measurement with the host's python sqlite3
  (SAVEPOINT and RELEASE inside BEGIN IMMEDIATE succeed under query_only, foreign_keys and
  defer_foreign_keys, and RELEASE of a nested savepoint never fails), plus the engine's
  documentation of SAVEPOINT, RELEASE and deferred foreign keys. Disclosed limit: the measurement
  was not taken through the engine the crate links.
- O2 (daemon wiring.rs `fold.register(Phase::CoinMint, Box::new(MintStep))?;`):
  EQUIVALENT-BY-CONSTRAINT. `Fold::register` errors only on `if step.phase() != phase`
  (crates/coordination/src/recompute/mod.rs:435), and `MintStep::phase()` returns
  `Phase::CoinMint`. The phase is held by the killed row S08212, so a mutant that reaches the arm is
  one S08212 already kills.

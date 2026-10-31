# SPEC-432: three domain checks list the economy's float sites, the append-only tables' sanctioned rewrites and every sync write's idempotency

- **Issue:** none; the pull request closes nothing. **Context(s):** `deck-streak-economy` (the float
  census, test code), `deck-streak-daemon` (the append-only register and its census, test code) and
  `deck-streak-ingest` (the sync write census, test code). No production source changes.
- **Decided by:** ADR-446 (the three checks are Rust tests in economy, the composition root and
  ingest, each listing today's sites at fail).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-432.md`. **Mutation band:** `S43200-S43299`.

## 1. The problem, measured

Every figure was read at `b25ed04a` with the command beside it, and each check's own test
re-derives it (A1, A5, A10).

- **Floats in the economy.** `git grep -n -w -E 'f32|f64' b25ed04a -- crates/economy/src` lists six
  lines: `constants.rs` 14, 16 and 24 (`DAILY_LOSS_CAP_WALLET_FRAC: f64 = 0.30`,
  `FINE_WALLET_FRAC: f64 = 0.02`, `PASS_SURCHARGE_MULT: f64 = 1.5`), `rules.rs` 4 (a doc comment), 32
  (`wallet_at_rollover as f64 * DAILY_LOSS_CAP_WALLET_FRAC`) and 47 (`(wallet as f64 *
  FINE_WALLET_FRAC).ceil()`). Read as tokens with the comments dropped, that is 8 float tokens in 5
  items: three type words and three decimal literals in the three constants, and two `as f64`
  casts in `daily_loss_cap` and `scaled_fine`. `PASS_SURCHARGE_MULT` is read by no source file of
  the crate (`git grep -n PASS_SURCHARGE_MULT b25ed04a -- crates` lists `constants.rs` and
  `tests/wallet_goldens.rs` only). `crates/economy/src` holds 6 `.rs` files and no `#[cfg(test)]`.
- **Append-only tables.** No register of them exists: `git grep -l -i 'append-only' b25ed04a --
  docs crates migrations` names prose in ADR-072, ADR-143, SPEC-072 and five other SPECs, and no
  list. Four migration headers declare a table written once: `004001_progression_xp_ledger.sql`
  (`xp_ledger`), `008201_economy_wallet_and_shop.sql` (`coin_ledger`, `settle_mint` its one
  updater), `007301_progression_badges_earned.sql` (`badges_earned`, the celebration mark its one
  update) and `007701_curriculum_road_to_c2_and_law.sql` (`band_milestones`, written once and marked
  on its celebration). `git grep -n -i -E 'TRIGGER|RAISE' b25ed04a -- migrations` finds no trigger.
- **Rewrites of those tables.** `git grep -n -E '(UPDATE|DELETE FROM)[[:space:]]+(xp_ledger|coin_ledger|badges_earned|band_milestones)' b25ed04a -- crates migrations`, read
  in `src` and the migrations, lists seven: `economy/src/wallet.rs:439` (`settle_mint_on`),
  `economy/src/data_rights.rs:106`, `progression/src/data_rights.rs:142` and `:151`,
  `curriculum/src/data_rights.rs:134` (each an `erase`), `coordination/src/recompute/badges.rs:198`
  (`offer_badges`) and `curriculum/src/store.rs:386` (`mark_band_up`). Every insert into the four
  tables ends `ON CONFLICT DO NOTHING` (`wallet.rs:579`, `progression/src/ledger.rs:124`,
  `progression/src/badges/award.rs:124`, `curriculum/src/store.rs:271`). No literal in `src` joins a
  write keyword to a `{` placeholder or ends with one.
- **Sync writes.** The ingest crate owns syncing and the one write back to Anki
  (`crates/ingest/src/lib.rs:3-6`). Its SQL writes sit in six modules: the sync modules `skip.rs`
  (6), `state.rs` (4) and `sync_runs.rs` (2), and the owner's modules `data_rights.rs`,
  `sensitive.rs` and `write_class_stop.rs`. Of the twelve sync writes, four are upserts of the one
  `ingest_state` row (`state.rs:195`, `:222`, `:251`, `:291`), one is a keyed insert
  (`skip.rs:363`), four are updates of absolute values (`skip.rs:409`, `:563`, `:588`, `:689`), and
  three are plain inserts: `skip.rs:657` (`record_prior`, under `UNIQUE (skip_id, card_id)`) and
  `sync_runs.rs:368` and `:432` (`record_skipped`, `record`), whose table's one unique key is its row id
  (`migrations/002201_ingest_sync_runs.sql`). No assignment names its own column. The skip writes
  are covered by the `SkipDayOnce` and `SkipDayWrite` models; `sync_runs.rs` and `state.rs` by none
  (`git grep -n '@phx covers crates/ingest/src/sync_runs.rs' b25ed04a -- formal` lists nothing).

## 2. Requirements

R1. `crates/economy/tests/integer_money.rs` reads every `.rs` file under `crates/economy/src/`
through `tools/table-census/table_census.rs::lex` and counts a float token by ADR-446 D1's five
rules, F1 to F5, naming each by its item.

R2. The found map of (file, item) to count EQUALS the listed map of five items: `constants.rs`
`DAILY_LOSS_CAP_WALLET_FRAC` 2, `FINE_WALLET_FRAC` 2, `PASS_SURCHARGE_MULT` 2, and `rules.rs`
`daily_loss_cap` 1 and `scaled_fine` 1. An unlisted item, a count that grew or shrank and a listed
item gone are each refused by name.

R3. `crates/daemon/tests/append_only_tables.rs` carries the register: `coin_ledger` (economy,
`migrations/008201_economy_wallet_and_shop.sql`), `xp_ledger` (progression,
`migrations/004001_progression_xp_ledger.sql`), `badges_earned` (progression,
`migrations/007301_progression_badges_earned.sql`) and `band_milestones` (curriculum,
`migrations/007701_curriculum_road_to_c2_and_law.sql`). Each row's migration exists.

R4. It reads every string literal of every `.rs` file under `crates/*/src/` (decoded by the shared
lexer) and every `migrations/*.sql` file with its `--` and `/* */` comments dropped, and finds each
UPDATE (with or without `OR <action>`), DELETE FROM, REPLACE INTO, INSERT OR REPLACE INTO and
upsert (`INSERT ... INTO` the table with `DO UPDATE` in its statement) of a registered table,
written bare, quoted, bracketed or `main.`-qualified, case-insensitive and bounded as a whole word.
Each hit is named by file, item, kind and table.

R5. A literal in `crates/*/src/` in which `UPDATE`, `DELETE FROM` or `INTO` is followed by a `{`
placeholder or ends the literal, and any sqlx `query_file` macro there, is refused as a write the
check cannot read.

R6. The found list of rewrites EQUALS the sanctioned list of seven, counts included:
`economy/src/wallet.rs` `settle_mint_on` UPDATE `coin_ledger`; `economy/src/data_rights.rs` `erase`
DELETE `coin_ledger`; `progression/src/data_rights.rs` `erase` DELETE `xp_ledger` and DELETE
`badges_earned`; `curriculum/src/data_rights.rs` `erase` DELETE `band_milestones`;
`coordination/src/recompute/badges.rs` `offer_badges` UPDATE `badges_earned`; and
`curriculum/src/store.rs` `mark_band_up` UPDATE `band_milestones`. An unsanctioned rewrite, a
sanctioned one grown and a sanctioned one gone are each refused by name.

R7. `crates/ingest/tests/sync_write_idempotency.rs` reads every string literal of every `.rs` file
under `crates/ingest/src/`. A literal whose first SQL word is INSERT, UPDATE, DELETE or REPLACE is a
write; a literal whose first word is WITH and that holds one of them, or in which a write keyword
meets a `{` placeholder, is unreadable and refused. Every module holding a write is a sync module
(`skip.rs`, `state.rs`, `sync_runs.rs`) or an exempt module with its reason (`data_rights.rs`,
`sensitive.rs`, `write_class_stop.rs`); an unclassified module, and a listed module that holds no
write, are refused.

R8. Each write in a sync module is classed KEYED, UPSERT, SET, DELETE or PLAIN by ADR-446 D3. An
assignment whose right side names its own column outside `excluded.` is an INCREMENT and is
refused. The PLAIN sites EQUAL the listed three, each with its reason: `skip.rs` `record_prior`
(the key refuses a second row), `sync_runs.rs` `record` and `record_skipped` (an event record with
no key). The per-class counts are printed.

R9. Every walk prints `examined <n> <what>` (files, tokens, literals, migrations, statements) and
refuses zero.

R10. No production source changes, no Python module is added, and
`scripts/tests/test_ci_workflows.py` is untouched.

## 3. Acceptance criteria of SPEC-432

| id | criterion | decided by |
|---|---|---|
| A1 | the economy's float tokens at the head equal the listed five items and counts, with the files and tokens examined printed | `the_economy_holds_floats_only_at_its_listed_sites` |
| A2 | a planted tree holding one site per rule F1 to F5, each in its own item, is refused item by item | `a_float_planted_in_the_economy_is_refused_by_its_rule` |
| A3 | a field path, a nested field path, a range, a method on an integer, a hex literal, a comment and a string are counted as no float | `a_whole_number_shape_is_not_a_float` |
| A4 | an unlisted item, a grown count, a shrunk count and a listed item gone are each refused by name | `a_listed_float_site_gone_or_grown_is_refused` |
| A5 | the rewrites of the registered tables at the head equal the seven sanctioned writes, with the files, literals and migrations examined printed, and each register row's migration exists | `the_append_only_tables_take_only_their_sanctioned_writes` |
| A6 | a planted crate and a planted migration holding each kind (UPDATE, UPDATE OR, DELETE FROM, REPLACE INTO, INSERT OR REPLACE, upsert) in each spelling (bare, quoted, bracketed, qualified, lower case) are refused by file, item, kind and table | `a_planted_rewrite_of_an_append_only_table_is_refused_by_kind` |
| A7 | a keyed insert, an insert-or-ignore, a read, a table whose name extends a registered one, another table, and a rewrite inside a Rust or SQL comment are not refused | `an_insert_a_read_or_another_table_is_not_a_rewrite` |
| A8 | a write joined to a placeholder, a write keyword ending a literal and a `query_file` macro are each refused as unreadable | `a_write_the_check_cannot_read_fails_closed` |
| A9 | an unsanctioned rewrite, a sanctioned one doubled and a sanctioned one gone are each refused by name | `a_sanctioned_write_gone_or_doubled_is_refused` |
| A10 | every write in the ingest sync modules at the head is classed, the PLAIN sites equal the listed three, no write is an INCREMENT or unreadable, and the modules, literals, writes and per-class counts are printed | `every_sync_write_names_its_idempotency` |
| A11 | a planted unlisted plain insert, a self-incrementing update, a self-incrementing upsert and an unreadable write in sync modules are each refused by name and class | `a_planted_sync_write_that_can_apply_twice_is_refused` |
| A12 | a keyed insert, an insert-or-ignore, an upsert of `excluded.` values, a guarded update of absolute values and a delete are classed KEYED, KEYED, UPSERT, SET and DELETE, and none is refused | `an_idempotent_write_shape_is_classed_not_refused` |
| A13 | a module holding a write that is neither sync nor exempt, a listed module holding none, and a listed PLAIN site gone are each refused by name | `a_write_module_is_classified_and_a_listed_one_holds_a_write` |

```acceptance
A1: cargo test -p deck-streak-economy --test integer_money -- --exact the_economy_holds_floats_only_at_its_listed_sites
A2: cargo test -p deck-streak-economy --test integer_money -- --exact a_float_planted_in_the_economy_is_refused_by_its_rule
A3: cargo test -p deck-streak-economy --test integer_money -- --exact a_whole_number_shape_is_not_a_float
A4: cargo test -p deck-streak-economy --test integer_money -- --exact a_listed_float_site_gone_or_grown_is_refused
A5: cargo test -p deck-streak-daemon --test append_only_tables -- --exact the_append_only_tables_take_only_their_sanctioned_writes
A6: cargo test -p deck-streak-daemon --test append_only_tables -- --exact a_planted_rewrite_of_an_append_only_table_is_refused_by_kind
A7: cargo test -p deck-streak-daemon --test append_only_tables -- --exact an_insert_a_read_or_another_table_is_not_a_rewrite
A8: cargo test -p deck-streak-daemon --test append_only_tables -- --exact a_write_the_check_cannot_read_fails_closed
A9: cargo test -p deck-streak-daemon --test append_only_tables -- --exact a_sanctioned_write_gone_or_doubled_is_refused
A10: cargo test -p deck-streak-ingest --test sync_write_idempotency -- --exact every_sync_write_names_its_idempotency
A11: cargo test -p deck-streak-ingest --test sync_write_idempotency -- --exact a_planted_sync_write_that_can_apply_twice_is_refused
A12: cargo test -p deck-streak-ingest --test sync_write_idempotency -- --exact an_idempotent_write_shape_is_classed_not_refused
A13: cargo test -p deck-streak-ingest --test sync_write_idempotency -- --exact a_write_module_is_classified_and_a_listed_one_holds_a_write
```

Every planted tree is a temporary directory the test writes; no test plants in the product tree
and none asserts a wall-clock time. A3 and A7 are negative controls: their classifiers refuse
nothing at the red commit, so each is recorded `not red` and is held by the rows that widen an arm.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/economy/tests/integer_money.rs` | `deck-streak-economy`, test code | added: the float census and A1 to A4 |
| `crates/daemon/tests/append_only_tables.rs` | `deck-streak-daemon`, test code | added: the register, the rewrite census and A5 to A9 |
| `crates/ingest/tests/sync_write_idempotency.rs` | `deck-streak-ingest`, test code | added: the sync write census and A10 to A13 |
| `scripts/mutation-rows.d/S43200-S43299.json` | the rows | added: S43200 to S43236 (section 9) |
| `docs/specs/SPEC-432-three-domain-checks-list-the-economys-float-sites-the-append-only-tables-sanctioned-rewrites-and-every-sync-writes-idempotency.md` | the spec | added |
| `docs/decisions/ADR-446-the-domain-checks-are-rust-tests-in-economy-the-composition-root-and-ingest-each-listing-todays-sites-at-fail.md` | the decision | added |
| `docs/schematics/domain-integrity-checks.md` | the schematic | added |
| `docs/red-first/SPEC-432.md` | the record | added |
| `changelog.d/domain-integrity-432.md` | the fragment | added |

`tools/table-census/table_census.rs` is included by path and unchanged.

## 5. What this does NOT cover

- Every other lint rule; each is a later delivery of its own (#882).
- The rule that a destructive migration change needs a ruling, and its census (#892).
- The cure of the two share rules computed through `f64` and of the three float constants; they
  stay listed until a cure proves an integer form equal to the parity goldens (#890).
- A key for the `sync_runs` record, or a model that one run writes one row; both sites stay listed
  (#891).
- Money handled outside `crates/economy/src/` (#893).
- The recompute fold's writes after a sync, and the clients' sync writes (#894).
- Tables written only by inserts but declared append-only nowhere: `freeze_events`, `sync_runs`,
  `notification_decisions` and `agent_runs` (#895).
- Rust outside `src` (tests, benches, examples, build scripts) and a statement assembled by
  `concat!` or from constants within one file, which no literal shows; the readers' routes beyond
  literals are tracked by #585.

## 6. Risks

- **A pull request that adds an ingest module holding a write** reds A10 until it lists the module
  as sync or exempt. Whichever lands second lists it; the refusal names the module.
- **A sanctioned writer added later** reds A5 until its row joins the sanctioned list with its
  owning decision; that row counts against the check's precision (ADR-446 D6).
- **A change to the shared lexer** reaches three more test targets; A2, A6 and A11 plant every
  literal form the checks rely on, so a lexer regression reds them.
- **A literal that is prose** naming a rewrite of a registered table is refused; it is reworded or
  listed, and the listing counts against precision.
- **The daemon's test target builds the whole workspace**; CI already builds it for its other
  tests.

## 7. What only CI proves

The three targets on the pull request's merge ref, in the workspace run; and the mutation verdict:
a diff of test code only lists no generated mutant, so the hand rows of section 9 are this
delivery's mutation evidence.

## 8. Formal model

Not applicable by surface: no covered span is edited (every `@phx covers` line names a production
file, and this delivery edits none), the checks add no actor and no shared state, and the economy's
formal vectors (`formal/vectors/wallet.jsonl`, `formal/vectors/skip-tariff.jsonl`) are integer and
untouched.

## 9. Mutation rows (S43200-S43299)

Band `S43200-S43299`, in `scripts/mutation-rows.d/S43200-S43299.json`, table `MUTATIONS`: S43200 to
S43236, one per matcher rule, guard and judge arm, each killed by the test named.

| row | file | the mutant | killer |
|---|---|---|---|
| S43200 | `tests/integer_money.rs` | F1 drops `f64` | A2 |
| S43201 | `tests/integer_money.rs` | F2 disabled | A2 |
| S43202 | `tests/integer_money.rs` | F3 disabled | A2 |
| S43203 | `tests/integer_money.rs` | F3 loses its after-a-dot guard | A3 |
| S43204 | `tests/integer_money.rs` | F4 disabled | A2 |
| S43205 | `tests/integer_money.rs` | F4 loses its radix-prefix guard | A3 |
| S43206 | `tests/integer_money.rs` | F5 disabled | A2 |
| S43207 | `tests/integer_money.rs` | F5 loses its range guard | A3 |
| S43208 | `tests/integer_money.rs` | the item is not named | A1 |
| S43209 | `tests/integer_money.rs` | the judge's gone arm disabled | A4 |
| S43210 | `tests/integer_money.rs` | the judge compares counts by at-least | A4 |
| S43211 | `tests/integer_money.rs` | a listed count changed | A1 |
| S43212 | `tests/append_only_tables.rs` | UPDATE disabled | A6 |
| S43213 | `tests/append_only_tables.rs` | UPDATE OR not read | A6 |
| S43214 | `tests/append_only_tables.rs` | DELETE FROM disabled | A6 |
| S43215 | `tests/append_only_tables.rs` | REPLACE INTO and INSERT OR REPLACE disabled | A6 |
| S43216 | `tests/append_only_tables.rs` | the upsert arm disabled | A6 |
| S43217 | `tests/append_only_tables.rs` | quoted, bracketed and qualified names not read | A6 |
| S43218 | `tests/append_only_tables.rs` | the whole-word bound dropped | A7 |
| S43219 | `tests/append_only_tables.rs` | SQL comments kept | A7 |
| S43220 | `tests/append_only_tables.rs` | the migrations not read | A6 |
| S43221 | `tests/append_only_tables.rs` | the placeholder arm disabled | A8 |
| S43222 | `tests/append_only_tables.rs` | the join-prefix arm disabled | A8 |
| S43223 | `tests/append_only_tables.rs` | the `query_file` arm disabled | A8 |
| S43224 | `tests/append_only_tables.rs` | the judge's gone arm disabled | A9 |
| S43225 | `tests/append_only_tables.rs` | a register row's table respelled | A5 |
| S43226 | `tests/sync_write_idempotency.rs` | PLAIN classed KEYED | A11 |
| S43227 | `tests/sync_write_idempotency.rs` | the UPDATE increment arm disabled | A11 |
| S43228 | `tests/sync_write_idempotency.rs` | the `excluded.` exemption dropped | A12 |
| S43229 | `tests/sync_write_idempotency.rs` | the upsert increment arm disabled | A11 |
| S43230 | `tests/sync_write_idempotency.rs` | `DO NOTHING` classed PLAIN | A12 |
| S43231 | `tests/sync_write_idempotency.rs` | `INSERT OR IGNORE` classed PLAIN | A12 |
| S43232 | `tests/sync_write_idempotency.rs` | the unclassified-module arm disabled | A13 |
| S43233 | `tests/sync_write_idempotency.rs` | the listed-module-without-a-write arm disabled | A13 |
| S43234 | `tests/sync_write_idempotency.rs` | the unreadable arm disabled | A11 |
| S43235 | `tests/sync_write_idempotency.rs` | `state.rs` dropped from the sync modules | A10 |
| S43236 | `tests/sync_write_idempotency.rs` | the PLAIN judge's gone arm disabled | A13 |

---
status: "accepted"
decision-makers: "the DeckStreak architect seat"
---

# The domain checks are Rust tests in economy, the composition root and ingest, each listing today's sites at fail

## Context and Problem Statement

Three properties of DeckStreak's data are stated in prose and held by no check that reads the whole
tree:

- **Money is whole coins.** The coin wallet keeps every movement as an integer (`coin_ledger.delta
  INTEGER`, `crates/economy/src/constants.rs`), yet two of its rules compute a wallet's share
  through `f64`: `rules.rs::daily_loss_cap` (`wallet_at_rollover as f64 * DAILY_LOSS_CAP_WALLET_FRAC`)
  and `rules.rs::scaled_fine` (`(wallet as f64 * FINE_WALLET_FRAC).ceil()`), each the predecessor's
  own order of operations, held by the parity goldens. Nothing refuses a third.
- **An append-only table is never rewritten.** Four migrations declare their table written once:
  `004001_progression_xp_ledger.sql` ("Nothing updates a grant, and only the owner's erase deletes
  one"), `008201_economy_wallet_and_shop.sql` ("`settle_mint` alone updates a row ... and only the
  owner's erase deletes one"), `007301_progression_badges_earned.sql` ("written once and never
  updated except for the celebration mark") and `007701_curriculum_road_to_c2_and_law.sql`
  (`band_milestones` "written once"). The tree holds no register of these tables, and no migration
  holds a trigger. The wallet census and the ledger census (SPEC-324) hold WHO may write
  `coin_ledger` and `xp_ledger`; none holds WHICH statements may.
- **A sync write applied twice changes nothing.** The ingest context owns syncing the private copy
  and the one write back to Anki (`crates/ingest/src/lib.rs`). Its SQL writes are an upsert, a keyed
  insert, a guarded update or a plain insert, and no check names which.

Measured at `b25ed04a` (SPEC-432 section 1 holds each command): 8 float tokens in 5 items of
`crates/economy/src/`; 7 rewrite statements against the four declared tables, every one the
sanction its migration names; 12 write statements in the ingest sync modules, 3 of them plain
inserts.

## Decision Drivers

- A check lands by listing today's sites, so it fails on the first new one and changes no product
  code.
- Each check prints what it examined, and an empty population is VOID, never a pass.
- A check reads SQL and Rust as the compiler reads them: literals decoded, comments dropped.
- One lexer reads Rust for every check: the tree's shared one, never a second copy.
- Kill criteria are counted over the last 30 merged PRs, never over a number of days.

## Decision Outcome

### D1. Integer money in the economy crate

**Money** is every amount the crate's own types carry: a coin movement's delta, a balance, a mint,
a fine, a loss cap, a price (the shop's and the skip ladder's) and every share or multiple applied
to one. The whole of `crates/economy/src/` is the money path: the crate owns the coin wallet and
nothing else (its `lib.rs` header), so a float anywhere in it is a float on a money path.

**The matcher** reads each `.rs` file under `crates/economy/src/` through the shared lexer
(`tools/table-census/table_census.rs::lex`, which drops comments and decodes literals) and counts a
float token by five rules:
- F1, the type word `f32` or `f64` (a cast `as f64` is this rule);
- F2, a word ending `_f32` or `_f64`, or a digit-led word ending `f32` or `f64` (`as_f64`,
  `to_f64`, `5_f32`);
- F3, a decimal literal: a digit-led word, a `.`, then a digit-led word, the first not itself after
  a `.` (so `t.0.1` is a field path);
- F4, an exponent literal: a digit-led word, not `0x`, `0o` or `0b`, holding `e` or `E` after a digit
  (`1e3`, and `2e` before a `-` or `+`);
- F5, a trailing-dot literal: a digit-led word, then a `.` followed by neither a word nor a `.`.

Each token is named by its item, the identifier after the last `fn`, `const`, `static`, `type`,
`struct`, `enum` or `trait` keyword before it.

**Population and level.** 6 source files at `b25ed04a`. The rule has today's sites, so it lands at
fail with them listed by item and count: `constants.rs` `DAILY_LOSS_CAP_WALLET_FRAC` 2,
`FINE_WALLET_FRAC` 2 and `PASS_SURCHARGE_MULT` 2 (each a type word and a decimal literal), and
`rules.rs` `daily_loss_cap` 1 and `scaled_fine` 1 (each an `as f64` cast). The found map must EQUAL
the list: a new item, a grown count, a shrunk count and a listed item gone are each refused by name.

**The six float mentions** a census of the crate's text reads (`f64` on six lines) are triaged one
by one: three are the share and multiple constants (ratios applied to money, F1 and F3), two are
the casts inside the share rules (F1), and one is the doc comment of `rules.rs`, which the lexer
drops and no rule counts.

**Known false positives:** a ratio or probability that is no amount of money is still counted,
because in this crate every ratio scales money; a float inside a `#[cfg(test)]` module of `src` is
counted (none exists at `b25ed04a`); an identifier that ends `_f64` and is no float is counted.
**Known false negatives:** a use of a float type alias (only its declaration names `f64`); a float
reached through another crate's type or an inferred `.parse()`; money held outside the crate
(section 5 of SPEC-432); Rust outside `src` (#585).

Chosen against: the D1 bullets of the section below.

### D2. Append-only tables take no UPDATE, DELETE or insert-or-replace

**The register.** The tree declares append-only tables only in their migrations' header sentences,
so the check carries its own register, one row per declared table, each citing the migration whose
header declares it: `coin_ledger` (economy, `008201`), `xp_ledger` (progression, `004001`),
`badges_earned` (progression, `007301`) and `band_milestones` (curriculum, `007701`). Tables written
only by inserts but declared nowhere (`freeze_events`, `sync_runs`, `notification_decisions`,
`agent_runs`) are not listed. **Who adds a row:** the delivery that declares a table append-only, in
its migration's header or its SPEC, adds the register row and the table's sanctioned writes in the
same pull request.

**The matcher** reads every string literal of every `.rs` file under `crates/*/src/` (decoded by the
shared lexer) and every `migrations/*.sql` file (`--` and `/* */` comments dropped), and finds, for a
registered table written bare, quoted (`"t"`, `` `t` ``, `[t]`) or `main.`-qualified, case-insensitive
and bounded as a whole word:
- an `UPDATE`, or `UPDATE OR <action>`, of the table;
- a `DELETE FROM` the table;
- a `REPLACE INTO` or `INSERT OR REPLACE INTO` the table;
- an `INSERT ... INTO` the table whose statement holds `DO UPDATE` (an upsert).

It fails closed on a write it cannot read, whatever the table: a literal in which a write keyword
(`UPDATE`, `DELETE FROM`, `INTO`) is followed by a `{` placeholder or ends the literal (a join
prefix), and any `query_file` macro of sqlx in `src`. Each hit is named by its file, its item (the
same rule as D1), its kind and its table.

**Population and level.** Every crate's `src` and every migration. The rule has today's sites, so it
lands at fail with the seven sanctioned writes listed, each the sanction its migration names:
`economy/src/wallet.rs` `settle_mint_on` UPDATE `coin_ledger`; the four erases, `economy/src/
data_rights.rs`, `progression/src/data_rights.rs` (two) and `curriculum/src/data_rights.rs`, each an
`erase` DELETE; and the two celebration marks, `coordination/src/recompute/badges.rs`
`offer_badges` UPDATE `badges_earned` and `curriculum/src/store.rs` `mark_band_up` UPDATE
`band_milestones`. The found set must EQUAL the list, counts included.

**Known false positives:** a literal that is prose naming a rewrite (an error message) is counted;
a test fixture's own table inside a `src` test module is counted when it bears a registered name.
**Known false negatives:** a statement assembled by `concat!` or from constants inside one file,
which no placeholder shows (#585 holds the readers' routes beyond literals); a view or a trigger
that writes the table (no migration holds a trigger at `b25ed04a`); a foreign key that cascades a
delete (none of the four tables carries one); an alias (`UPDATE t AS a` is still read, but a
statement through a view of another name is not); Rust outside `src` (#585).

Chosen against: the D2 bullets of the section below.

### D3. Idempotent sync writes, measured first

**The measurement**, read-only at `b25ed04a`: every SQL write statement in the ingest sync modules
(`skip.rs`, `state.rs`, `sync_runs.rs`), each with how a second application is absorbed:

| site | statement | class |
|---|---|---|
| `state.rs` `request_rescore`, `record_refusal`, `write_anchor`, `write_window_base` | `INSERT ... ON CONFLICT (id) DO UPDATE SET` of absolute values | UPSERT |
| `skip.rs` `begin` | `INSERT INTO skip_days ... ON CONFLICT (study_day) ... DO NOTHING` | KEYED |
| `skip.rs` `settle_failed`, `settle_applied_on`, `mark_undone_on`, `record_left` | `UPDATE ... SET` of absolute values under a state or key guard | SET |
| `skip.rs` `record_prior` | `INSERT INTO skip_card_snapshot` with no clause, under `UNIQUE (skip_id, card_id)` | PLAIN: the key refuses a second row and the write rolls back |
| `sync_runs.rs` `record`, `record_skipped` | `INSERT INTO sync_runs` with no clause and no unique key | PLAIN: none |

The engine's writes (`normal_sync`, `full_download`, `set_due_date`, `write_sync`) carry no SQL and
are held where they may be called by the skip census (`crates/ingest/tests/skip_census.rs`) and the
`SkipDayWrite` model; the skip record's writes are modelled by `SkipDayOnce` and `SkipDayWrite`.
No model covers `sync_runs.rs` or `state.rs`.

**The matcher** reads every string literal of every `.rs` file under `crates/ingest/src/`. A literal
whose first SQL word is `INSERT`, `UPDATE`, `DELETE` or `REPLACE` is a write; one whose first word is
`WITH` and that holds a write keyword is unreadable. Each write in a sync module is classed: KEYED
(`ON CONFLICT ... DO NOTHING`, `INSERT OR IGNORE`), UPSERT (`DO UPDATE SET`, `REPLACE`, `INSERT OR
REPLACE`), SET (an `UPDATE`), DELETE, or PLAIN (an `INSERT` with no clause). An assignment in a
`SET` list whose right side names its own column, outside `excluded.`, is an INCREMENT, refused in
any class. Every module of `crates/ingest/src/` that holds a write is a sync module or a named exempt
one (`data_rights.rs`, the owner's erase and export; `sensitive.rs`, the owner's sensitive-deck
marks; `write_class_stop.rs`, the owner's stop), and each listed module holds a write.

**Level, from the measurement.** The rule has today's sites, so it lands at fail with the three PLAIN
sites listed, each with its reason; every other class is printed and not listed; an INCREMENT, an
unreadable write and an unclassified module are refused at zero sites. The two `sync_runs` sites
hold no idempotency at `b25ed04a`: each records a distinct run, and whether one run can be recorded
twice rests on the collection lock and the cycle, which no model covers. That is a question for the
maintainer (SPEC-432 section 5), never a cure folded in here.

**Known false positives:** a self-referencing assignment that is idempotent (`x = max(x, ?1)`) reads
INCREMENT. **Known false negatives:** a write made outside ingest after a sync (the recompute fold's
writes, held by their own keys and models); the clients' sync (the engine core's one-way write);
Rust outside `src` (#585).

Chosen against: the D3 bullets of the section below.

### D4. One home per check, each printing its examined count

| check | home | population |
|---|---|---|
| integer money | `crates/economy/tests/integer_money.rs` | `crates/economy/src/**/*.rs` |
| append-only tables | `crates/daemon/tests/append_only_tables.rs` | `crates/*/src/**/*.rs`, `migrations/*.sql` |
| sync writes | `crates/ingest/tests/sync_write_idempotency.rs` | `crates/ingest/src/**/*.rs` |

Each is a Rust integration test of the crate that owns its population: economy owns the money,
ingest owns syncing, and the append-only register names tables of three contexts, which only the
composition root may know (as `crates/daemon/tests/workspace.rs` holds the workspace's shape). Each
includes the shared lexer by path, as the table censuses do, prints `examined <n> <what>` for every
population it walks and refuses zero. No Python module is added, so `scripts/tests/
test_ci_workflows.py`'s registers are untouched.

Chosen against: the D4 bullets of the section below.

### D5. Today's sites are listed, never cured here

Each check lands by listing the sites it finds at the head it is built on, and no product file
changes. A listed site that looks like a defect is a question with options, recorded in SPEC-432
section 5: the two share rules computed through `f64`, and the two `sync_runs` inserts with no key.

Chosen against: the D5 bullets of the section below.

### D6. Kill criteria, per check

For each of the three checks:
- **Live precision** is fixes ÷ (fixes + new allow-list entries) over the last 30 merged PRs, where a
  fix is a listed site cured and its row removed, and an allow-list entry is a row added to the
  float list, the sanctioned list or the PLAIN list. Below 0.8 with at least 5 hits proposes a
  demotion.
- **Zero-hit arms** (F2, F4 and F5; the upsert, replace, `UPDATE OR` and fail-closed arms; INCREMENT,
  unreadable and unclassified-module): 2 or more false hits among an arm's first 5 propose a
  demotion of that arm.
- **Tooling:** `examined 0` for any population, or the shared lexer missing, is VOID and blocks; it
  is never a pass.
- **A demotion** is a weakening: it needs an owner-signed ruling (`config/owner-allowed-signers`).

### D7. Not in this delivery

Every other lint rule, the rule that a destructive migration needs a ruling, the cure of any site
D5 lists, money outside the economy crate, the fold's and the clients' writes, and the tables
append-only in practice but undeclared. SPEC-432 section 5 cites the issue that holds each.

### D8. Tests and rows

Each acceptance criterion has a test that is red at its red commit for the criterion's reason,
except the two negative controls (A3, A7), recorded `not red` in `docs/red-first/SPEC-432.md`. Each
check's red is a site planted in a temporary tree, never in the product tree; no test asserts a
wall-clock time. The red commit keeps every walk and the lexer, and stubs only the classifiers, so
each population is examined and each assertion fails on the missing behaviour. Rows are
`scripts/mutation-rows.d/S43200-S43299.json`, one per matcher rule, guard and judge arm, each killed
by a planted test. **Formal:** not applicable by surface: no covered span is edited, the checks add
no actor and no shared state, and the economy's formal vectors stay integer and untouched.

### D9. One delivery, no amendment

One delivery under SPEC-432 and this ADR carries the three checks. No SPEC or ADR on the base is
amended: the checks state no new requirement of SPEC-022, SPEC-023, SPEC-040, SPEC-073, SPEC-077,
SPEC-082 or SPEC-083, and cite their rules as they stand. A new schematic,
`docs/schematics/domain-integrity-checks.md`, draws the three checks.

### Consequences

- A float added to the economy, a rewrite of a declared table and a sync write that can apply twice
  each fail CI by name on the pull request that adds it.
- A delivery that cures a listed site removes its row in the same commit, or the check refuses the
  stale row.
- A new ingest module holding a write names its class before it lands.
- Three more test targets include the shared lexer; a change to `lex` reaches them too.

### Confirmation

`cargo test -p deck-streak-economy --test integer_money`, `cargo test -p deck-streak-daemon --test
append_only_tables` and `cargo test -p deck-streak-ingest --test sync_write_idempotency`, each
printing its examined counts; the rows of `S43200-S43299`, each proved killed.

## What each decision was chosen against

- D1, a clippy `disallowed_types` entry for `f32` and `f64`: lost because it names types by path, so it sees neither a float literal nor a float an expression infers, and a `clippy.toml` in the crate would shadow the workspace's for the economy crate.
- D1, judging only `as f64` casts: lost because the constants that hold the shares carry no cast, and a new float constant would pass.
- D1, a value-level judge of which floats are money: lost because every float in the crate scales money, so a judge would accept what the rule exists to list.
- D2, a database trigger on each declared table: lost because a trigger is a migration and a product change, it refuses at run time on the live database, and it cannot tell the sanctioned writers (the mint's settle, the erase, the celebration marks) from any other.
- D2, extending the wallet and ledger censuses: lost because they hold which crate may name a table, not which statement may write it, and two of the four tables have no census.
- D2, a register derived from the migrations' prose: lost because the declaring sentences differ in wording, so a phrase matcher would miss the next one.
- D3, choosing a level before the measurement: lost because the classes were unknown until every site was read.
- D3, curing the two keyless inserts in this delivery: lost because whether one run can be recorded twice is an interleaving question no model answers yet.
- D3, listing every classified write: lost because a keyed, upsert or guarded write applied twice changes nothing, so only the PLAIN class needs a reason.
- D4, one combined module in one crate: lost because it would name two foreign contexts' tables and modules from a third, which only the composition root may do, and one failure would hide the other two checks' results.
- D4, Python under `scripts/tests/`: lost because it needs a second Rust literal lexer beside the tree's shared one, and the two would drift.
- D4, the append-only check in each owning crate: lost because the register is one list, and three copies of its walk would drift.
- D5, curing a listed site in this delivery: lost because a cure is a product change with its own goldens, tests and formal surface, and a lint delivery changes no product code.
- D9, amending SPEC-082 and SPEC-022 with the checks: lost because the checks add no requirement to either context; the SPECs' rules are cited as they stand.

## What would make this wrong

- A sanctioned writer the register does not know: a delivery that adds one must add its row, or the
  check refuses it, and a precision below 0.8 would show the register is too narrow.
- A shared-lexer change that stops decoding a literal form: the planted fixtures of A2, A6 and A11
  would go red.
- The predecessor's float order proving unequal to an integer form over some wallet: the float rows
  stay listed until a cure proves equality.

## More Information

SPEC-432 holds the measurements, the requirements, the acceptance criteria and the file manifest.
The shared lexer is SPEC-324's (ADR-325). The wallet's write rules are ADR-308's; the XP ledger's are
ADR-040's and ADR-072's.

# SPEC-435: a destructive migration names its ruling, and every unchecked money or counter operator, SQL escape, shell and environment read is listed

Path:
`docs/specs/SPEC-435-a-destructive-migration-names-its-ruling-and-every-unchecked-money-or-counter-operator-sql-escape-shell-and-environment-read-is-listed.md`.

- **Issue:** none; this SPEC closes nothing.
- **Context(s):** the kernel's and the economy's integration tests; no product code changes.
- **Decided by:** ADR-449 (three census tests hold today's destructive migrations, unchecked
  operators and escape hatches in registers, and refuse any other). It amends no SPEC and no ADR.
- **Schematic:** `docs/schematics/data-integrity-ii-checks.md` (added).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-435.md`.

## 1. The problem, measured

Every figure below is read from the tree at dev `b25ed04a`, by a reading separate from the tests:
a model of each matcher, written before the tests and run over the tree's text. Each census this
SPEC adds must equal it.

**Migrations.** SPEC-020 R18 says "a migration is additive", and nothing holds it.
`crates/kernel/tests/schema.rs` (`migration_files`) checks each migration's name, and that every
table is `STRICT` with `created_at`; it reads no statement for a removal. `migrations/` holds 33
files, and the census reads every one. The separate reading read 32 of them, and one of those is
destructive: `migrations/004102_notifications_queue_claim.sql` removes a table at line
31 and renames one at line 32, the two halves of a `STRICT` rebuild. SPEC-041 section 8 admits that
rebuild by name, and its file manifest names `migrations/004102_notifications_queue_claim.sql`. No
other statement of the 32 removes, renames or deletes anything. One comment,
`migrations/004302_agent_runs_created_at_index.sql:2`, names a delete of rows that runs in code; a
comment is no statement. No record in the tree admits a destructive migration: SPEC-020 R18's
sentence is the only rule, and SPEC-041 is the only ruling.

**Arithmetic in the money and counter paths.** The workspace sets no `[profile.release]`, so the
shipped build keeps Cargo's release defaults, with overflow checks off (SPEC-330, #473): an integer
sum that overflows in the shipped daemon wraps, while under `cargo test` it panics. The
workspace's lint table raises `clippy::all` and `clippy::pedantic` and four restriction lints
(`unwrap_used`, `expect_used`, `dbg_macro`, `print_stdout`); it raises no arithmetic lint, and
`clippy.toml` names none. Over the 57 sources under `src/` of the six crates that compute coins,
prices, XP, levels, streaks, freezes, chests, pity counters, tokens, sessions and the writing
streak (`economy`, `xp`, `progression`, `quests`, `streaks`, `habits`), the census of R8 to R13
finds 114 unchecked operator sites in 46 functions, 3 more at item level, and 17 `checked_`,
`saturating_`, `wrapping_` or `overflowing_` calls. By class: `Float` 5 functions and 9 sites,
`Widened` 4 and 11, `Calendar` 13 and 32, `Count` 6 and 15, `Domain` 18 and 47. Every site is
bounded today, and none can overflow under the invariants the code states: a level is built only
by `level_for`; `add_freeze` is refused at its cap in the same write; the pity counters reset at
their guarantees; the wallet's floor is 0; the day's loss cap is 0 to 100; a price at or below 0 is
refused; and `formal/tla/WalletFloor` bounds `purchase_on`'s subtraction.

**Escape hatches.** sqlx accepts a built `String` as a statement only inside `AssertSqlSafe`; a
process spawn can start a shell; and SPEC-020 R10 makes the daemon's `main` the one reader of the
process environment. The census reads all 349 sources under `crates/*/src`, `crates/*/build.rs`
and `crates/*/examples`. The separate reading read 341 of them, which hold one
`AssertSqlSafe(` (`crates/ingest/src/reader.rs:372`), five `Command::new(` (one launches `bash`,
three launch a configured program, and one launches `python3`), and seven environment reads in six
files. `crates/agent/tests/deck_gate_census.rs` declares every `Command::new(` under
`crates/*/src` for the deck gate (SPEC-381), and `crates/mcp/tests/settings.rs` holds the mcp
crate's own reads; neither holds a register of shells or environment reads with their reasons.

| census | test | population | examined | read by the separate reading | sites listed | register rows |
|---|---|---|---|---|---|---|
| migrations | `crates/kernel/tests/additive_migrations.rs` | 33 | 33 | 32 | 2 | 2 |
| arithmetic | `crates/economy/tests/bounded_arithmetic.rs` | 57 | 57 | 57 | 114 | 46 |
| escape hatches | `crates/kernel/tests/allow_list_arms.rs` | 349 | 349 | 341 | 12 | 11 |

The three registers list 128 sites. The arithmetic census also counts 3 item-level sites and 17
bounded calls, which it prints and never judges, and the arms census examines one spawn of a
literal program, `python3` in `crates/agent/src/gate.rs`, which passes. The registers hold the
separate reading's sites alone: a site the census finds in any other file fails its test.

## 2. Requirements

### The migration census (`crates/kernel/tests/additive_migrations.rs`)

R1. The population is every `.sql` file directly in `migrations/`, in name order, and every one
is read: the census names no file to skip. It prints `examined N migration(s)`.

R2. Before it splits a file, the census blanks `--` comments to the end of their line, `/* */`
comments, single-quoted literals (a doubled `''` stays inside the literal) and double-quoted
identifiers, each replaced by spaces, keeping every line break.

R3. The blanked text is split at `;`. Each statement is upper-cased and split into words of
`[A-Z0-9_]`. A statement's line is the line of its first character that is not white space, so a
statement split across lines is one statement at its first line.

R4. Each statement is judged against seven classes, each at most once per statement:
- **removes a table**: the removal verb, then `TABLE`;
- **removes a column**: a statement whose first two words are `ALTER TABLE` and that holds the
  removal verb;
- **removes an index**, **removes a view**, **removes a trigger**: the removal verb, then
  `INDEX`, `VIEW` or `TRIGGER`;
- **renames**: a statement whose first two words are `ALTER TABLE` and that holds `RENAME`;
- **deletes rows**: `DELETE` then `FROM`, or `OR` then `REPLACE`, or a statement whose first two
  words are `REPLACE INTO`.

A table rebuild is found by its parts: it removes a table and renames one.

R5. The removal verb is joined from two parts at run time and is spelled whole nowhere in the
tree. Every planted statement that needs it is built from the same parts.

R6. The register is `ADMITTED`: one row per migration and class, holding the migration's file
name, the class, the count of its hits, the SPEC that admits it and the reason. The census refuses:
a hit that no row lists; a row whose count differs from the census's; a row the census does not
reach; and a row whose SPEC file does not name the migration's file name. A row's SPEC file is the
one file `docs/specs/SPEC-<n>-*.md`, found by name; none or two is a refusal. A row is added by
the delivery that adds its destructive
migration, in the same pull request, citing its own SPEC, whose text names the migration. Today's
register, every hit already on dev entered as admitted history:

| migration | class | count | SPEC | reason |
|---|---|---|---|---|
| `004102_notifications_queue_claim.sql` | removes a table | 1 | SPEC-041 | a `STRICT` rebuild that adds the claim column and the sending state (section 8) |
| `004102_notifications_queue_claim.sql` | renames | 1 | SPEC-041 | the same rebuild's rename of the new table into place (section 8) |

R7. The census prints each hit as `<path>:<line> <class>`, never a statement's text, and fails
at `examined 0`, which is VOID and never a pass.

### The arithmetic census (`crates/economy/tests/bounded_arithmetic.rs`)

R8. The population is every `.rs` file under `src/` of `crates/economy`, `crates/xp`,
`crates/progression`, `crates/quests`, `crates/streaks` and `crates/habits`, read as text. Every
source is read: the census names no file to skip, so a money path is never skipped.

R9. The census blanks `//` comments, nested `/* */` comments and string literals (plain, byte, C
and raw with any number of hashes) to spaces, and turns a character or byte-character literal into
`0` followed by spaces, so the literal reads as an operand; it keeps every line break. It then
blanks every `#[cfg(test)]` item, from the attribute to the first `;` at bracket depth 0 or to the
brace that matches the first `{`, whichever comes first.

R10. A function body is the brace-matched block after `fn <name>`: the first `{` at parenthesis
and bracket depth 0 before any `;` at that depth. A site inside a body is named by its innermost
function. The operators are read by longest match over `+=`, `-=`, `*=`, `->`, `=>`, `+`, `-` and
`*`:
- `+=`, `-=` and `*=` are always sites;
- `->` and `=>` are never sites;
- `+`, `-` and `*` are sites when the previous character that is not white space is a letter, a
  digit, `_`, `)`, `]` or `?`, and, when it ends a word, that word is not one of `return`, `in`,
  `if`, `else`, `match`, `while`, `let`, `mut`, `break`, `move`, `for`, `loop`, `where`, `impl`,
  `dyn`, `const`, `static`, `ref`, `as`, `yield`, `box` or `unsafe`;
- a `+` or `-` directly after the `e` or `E` that ends a decimal numeric literal (a word that
  starts with a digit and carries no `0x`, `0o` or `0b` prefix) is the literal's exponent sign and
  no site;
- a `.sum(` or `.product(` call, with or without a turbofish, is a site.

R11. A site outside every function body is item level: a constant expression, which the compiler
evaluates and refuses on overflow, or a trait bound. It is counted, printed and never judged.
`checked_`, `saturating_`, `wrapping_` and `overflowing_` calls are counted and printed, never
judged.

R12. The register is `LISTED`: one row per source and function, holding the count of its sites,
its class and the bound in words. The classes are `Float` (an operand is a float, which the text
cannot type), `Widened` (each operand is widened before the sum), `Domain` (a constant, cap, ladder
or guarantee the code enforces bounds it), `Calendar` (a real date or time bounds it) and `Count`
(a count over a finite set bounds it). The census holds its sites equal to the register, function
by function, and refuses a function no row lists, a row whose count differs, and a row the census
does not reach. A new site passes by being written in a bounded form, or by a row whose reason a
reviewer reads. Today's register, 46 rows and 114 sites (paths under `crates/`):

| source | function | sites | class | bound |
|---|---|---|---|---|
| `economy/src/rules.rs` | `daily_loss_cap` | 1 | Float | a float share of the wallet |
| `economy/src/rules.rs` | `scaled_fine` | 1 | Float | a float share of the wallet |
| `economy/src/wallet.rs` | `debit_capped_on` | 2 | Domain | the day's cap of 0 to 100 less the day's non-negative debits; a balance less its floor of 0 |
| `economy/src/wallet.rs` | `debit_floored_on` | 1 | Domain | a balance less its floor of 0 |
| `economy/src/wallet.rs` | `movements` | 1 | Domain | a page constant plus 1 |
| `economy/src/wallet.rs` | `purchase_on` | 1 | Domain | a positive price from a balance kept at or above its floor (`formal/tla/WalletFloor`) |
| `economy/src/wallet.rs` | `settle_mint_on` | 2 | Domain | a balance less its floor of 0; the day's mint of at most 40 less a non-negative room |
| `habits/src/minutes.rs` | `week_end` | 2 | Calendar | a week's day numbers |
| `habits/src/minutes.rs` | `week_start` | 2 | Calendar | a week's day numbers |
| `habits/src/writing.rs` | `streak_ending` | 3 | Count | the days of a finite set, walked back by a cursor |
| `progression/src/badges/hours.rs` | `local_hour` | 2 | Widened | widened to `i128` |
| `progression/src/consistency.rs` | `consistency_multiplier` | 2 | Float | a float multiplier |
| `progression/src/consistency.rs` | `day_base_xp` | 1 | Widened | `u32` terms widened to `i64`, over one day's rows |
| `progression/src/consistency.rs` | `day_bonuses` | 4 | Count | two review counts of one day summed; the rest float |
| `progression/src/consistency.rs` | `projected_multiplier_drop` | 1 | Domain | a run of at least 0 less a configured step, floored |
| `progression/src/consistency.rs` | `tier_down_run` | 2 | Count | a fold over a finite list, 1 per day; less a step floored at 0 |
| `progression/src/exchange.rs` | `exchange_rates` | 2 | Count | sums of per-row XP and per-day graduations over the rows read |
| `progression/src/level.rs` | `level_info` | 3 | Domain | 100 times a level; a total less its level's start, which is at most the total |
| `progression/src/milestone.rs` | `candidate` | 2 | Domain | a target less a value below it; the share is float |
| `progression/src/milestone.rs` | `next_milestone` | 1 | Domain | a non-empty constant ladder's length less 1 |
| `progression/src/milestone.rs` | `smaller` | 4 | Domain | the ladder's targets and the progress |
| `progression/src/records.rs` | `chase` | 1 | Domain | two non-negative counts |
| `progression/src/xp.rs` | `level_for` | 6 | Widened | widened to `u128` |
| `progression/src/xp.rs` | `xp_to_reach` | 3 | Domain | a level from `level_for`, whose start is at most the total |
| `quests/src/chests.rs` | `epic_odds_pts` | 4 | Domain | a counter below its guarantee less a constant; float odds |
| `quests/src/chests.rs` | `grant_session_chests_on` | 2 | Count | a session's start difference in milliseconds; chests held at most the day's cap |
| `quests/src/chests.rs` | `payout_xp` | 5 | Domain | a constant payout range; float shares |
| `quests/src/chests.rs` | `roll_rarity` | 6 | Domain | the pity counters below their guarantees plus 1; float odds |
| `quests/src/chests.rs` | `sweep_stale_chests_on` | 1 | Calendar | a real time in milliseconds |
| `quests/src/draw.rs` | `fraction` | 1 | Float | a float fraction |
| `quests/src/pity.rs` | `after` | 4 | Domain | each counter stays below its guarantee: an Epic by the 14th, a Legendary by the 40th, reset by `roll_rarity` |
| `quests/src/sessions.rs` | `measure` | 2 | Count | the reviews of one session; seconds are float |
| `quests/src/sessions.rs` | `session_bounds` | 1 | Calendar | a real time in milliseconds |
| `quests/src/tokens.rs` | `activate_token_on` | 2 | Calendar | the time now in milliseconds plus a constant window |
| `streaks/src/calendar.rs` | `first_day` | 3 | Calendar | a real day number |
| `streaks/src/calendar.rs` | `language` | 1 | Calendar | a real day number |
| `streaks/src/calendar.rs` | `law` | 1 | Calendar | a real day number |
| `streaks/src/freeze.rs` | `civil_month` | 15 | Calendar | the civil date of a real day number |
| `streaks/src/freeze.rs` | `freeze_events_for` | 2 | Widened | `u32` freezes widened to `i64` |
| `streaks/src/governor.rs` | `silence_walk` | 1 | Calendar | a bounded walk back over real days |
| `streaks/src/governor.rs` | `standby_notice` | 1 | Calendar | a real day number |
| `streaks/src/law.rs` | `bridged_streak` | 1 | Calendar | a real day number |
| `streaks/src/store.rs` | `add_freeze` | 1 | Domain | `admit` refuses at the cap of 3 in the same write |
| `streaks/src/strength.rs` | `advance` | 4 | Float | float strengths |
| `streaks/src/strength.rs` | `persists` | 1 | Calendar | a real day number |
| `xp/src/review_xp.rs` | `review_xp` | 5 | Domain | `ease - 1` inside 1 to 4; four float products |

R13. The census prints `examined N source(s)`, its sites, item-level sites and bounded calls by
count, and each unlisted or recounted function as `<path> <function> <count>`; it prints no line's
text. It fails at `examined 0`, which is VOID.

### The escape-hatch census (`crates/kernel/tests/allow_list_arms.rs`)

R14. The population is every `.rs` file under `crates/*/src/`, every `crates/*/build.rs` and
every `.rs` file under `crates/*/examples/`, read as text and blanked as R9 blanks comments and
literals; `#[cfg(test)]` items are examined, not blanked. Every source is read: the census names
no file to skip, and prints `examined N source(s)`.

R15. The arms, read in the blanked text:
- **sql-escape**: `AssertSqlSafe`, then `(`; a `use` line naming it is no site;
- **env-read**: `env`, `::`, then `var`, `var_os`, `vars` or `vars_os`, then `(`;
- **shell** and **configured**: every `Command::new(` is examined, and its first argument, read
  from the unblanked text, decides it. A string literal whose last `/` part is `bash`, `sh`,
  `dash`, `zsh`, `ksh` or `fish` is a **shell**; an argument that is not a string literal is a
  **configured** program, which a configuration can make a shell; any other string literal is
  examined and passes.

R16. The register is `ARMS`: one row per source and arm, holding the count and the reason. The
census holds each arm's sites equal to the register, file by file, and refuses a site no row
lists, a row whose count differs, and a row the census does not reach. Today's register, 11 rows
and 12 sites:

| source | arm | count | reason |
|---|---|---|---|
| `crates/ingest/src/reader.rs` | sql-escape | 1 | the reader's one door that runs a caller's statement on the read-only copy (SPEC-023 R1) |
| `crates/agent/src/runner.rs` | shell | 1 | `bash <script> <file>` by argument vector, with no `-c` (ADR-043) |
| `crates/daemon/src/snapshot_lister.rs` | configured | 1 | the archive's list command from configuration, run with a cleared environment |
| `crates/vault/src/staged.rs` | configured | 1 | the vault gate's configured interpreter runs a pack probe; no model is called |
| `crates/ingest/src/skip_write/tests.rs` | configured | 1 | a test module re-runs its own test binary |
| `crates/daemon/src/main.rs` | env-read | 1 | `main` reads the process environment once (SPEC-020 R10) |
| `crates/ingest/src/skip_write.rs` | env-read | 1 | the zone pin's `TZ` (SPEC-083) |
| `crates/ingest/src/skip_write/tests.rs` | env-read | 1 | the re-run child reads its role |
| `crates/progression/build.rs` | env-read | 1 | the build script's census switch |
| `crates/fsrs7/examples/replay_report.rs` | env-read | 1 | an example that is its own `main` writes a CI step's summary file when one is set |
| `crates/ingest/examples/engine_probe.rs` | env-read | 2 | an example that is its own `main` reads its endpoint and the credentials directory it hands to the kernel's credential loader |

R17. The census prints each site as `<path>:<line> <arm>`, never a line's text, and fails at
`examined 0`, which is VOID.

### All three

R18. No product file changes: no source under `crates/*/src`, no migration, no manifest, no lint
table and no `clippy.toml`.

R19. Every red is a planted site in a fixture, never in the product tree. The migration census
builds its planted migrations in a temporary directory at run time; the arithmetic census reads
`crates/economy/tests/fixtures/bounded_arithmetic/plants.rs.fixture`; the arms census reads
`crates/kernel/tests/fixtures/allow_list_arms/plants.rs.fixture`. No census names a file to skip:
each reads every file of its population.

## 3. Acceptance criteria of SPEC-435

| id | criterion | decided by |
|---|---|---|
| A1 | over planted migrations, one for each class of R4 and one for each form of "deletes rows", plus an additive control that creates a table, adds a column and names `rename` as a column, the census finds exactly the planted (file, class) pairs and nothing in the control | `every_destructive_class_is_found_in_a_planted_migration` |
| A2 | a removal in a `--` comment, in a `/* */` comment, in a single-quoted literal and in a double-quoted identifier is no hit, and a removal split over three lines is one hit at its first line | `a_comment_or_a_literal_is_no_hit_and_a_split_statement_is_one` |
| A3 | four planted admissions are each refused by R6's reason: a hit with no row, a row whose count differs, a row whose planted SPEC omits the migration's file name, and a row the census does not reach | `an_unlisted_recounted_unruled_or_stale_admission_is_refused` |
| A4 | over `migrations/`, the census's hits equal R6's register, SPEC-041's file names the migration, and the examined count equals section 1's | `every_destructive_migration_on_the_tree_is_admitted_by_its_ruling` |
| A5 | over an empty directory the census refuses at `examined 0`, and over a directory holding one additive plant it examines 1 and finds no hit | `an_empty_migration_directory_is_void` |
| A6 | over the planted source, the census finds each planted site of R10 by its function, no site in a comment, a literal, a `#[cfg(test)]` module, an arrow, a unary or dereferencing operator, a keyword's operand or an exponent, and the constant expression at item level, all against literal counts | `the_matcher_finds_each_unchecked_operator_in_a_planted_source_and_nothing_else` |
| A7 | a planted function no row lists, a row whose count differs and a row the census does not reach are each refused by R12's reason | `an_unlisted_recounted_or_stale_fn_is_refused` |
| A8 | over the six crates, the census's sites equal R12's register function by function, and its examined count equals section 1's | `every_unchecked_operator_in_the_money_and_counter_paths_is_listed_with_its_bound` |
| A9 | over a temporary tree whose six crates hold no source the census refuses at `examined 0`, and over one planted source in each of the six crates it examines 6 | `an_empty_population_is_refused` |
| A10 | over the planted source, each arm finds its planted sites (one SQL escape, the four environment forms, the six shells, one configured program) and nothing in its `use` line, a comment, a literal, an `env!` or the literal non-shell program | `each_arm_finds_its_planted_site_and_nothing_else` |
| A11 | a planted site no row lists, a row whose count differs and a row the census does not reach are each refused by R16's reason | `an_unlisted_recounted_or_stale_arm_row_is_refused` |
| A12 | over the tree, every arm's sites equal R16's register file by file, the build script's and the examples' rows included, and the examined count equals section 1's | `every_arm_site_on_the_tree_is_listed_with_its_reason` |
| A13 | over a temporary tree holding one source under `src/`, one `build.rs` and one example, each with one environment read, the census examines 3 and finds the 3 reads; over an empty tree it refuses at `examined 0` | `an_empty_source_set_is_void_and_every_source_kind_is_read` |

```acceptance
A1: cargo test -p deck-streak-kernel --test additive_migrations -- --exact every_destructive_class_is_found_in_a_planted_migration
A2: cargo test -p deck-streak-kernel --test additive_migrations -- --exact a_comment_or_a_literal_is_no_hit_and_a_split_statement_is_one
A3: cargo test -p deck-streak-kernel --test additive_migrations -- --exact an_unlisted_recounted_unruled_or_stale_admission_is_refused
A4: cargo test -p deck-streak-kernel --test additive_migrations -- --exact every_destructive_migration_on_the_tree_is_admitted_by_its_ruling
A5: cargo test -p deck-streak-kernel --test additive_migrations -- --exact an_empty_migration_directory_is_void
A6: cargo test -p deck-streak-economy --test bounded_arithmetic -- --exact the_matcher_finds_each_unchecked_operator_in_a_planted_source_and_nothing_else
A7: cargo test -p deck-streak-economy --test bounded_arithmetic -- --exact an_unlisted_recounted_or_stale_fn_is_refused
A8: cargo test -p deck-streak-economy --test bounded_arithmetic -- --exact every_unchecked_operator_in_the_money_and_counter_paths_is_listed_with_its_bound
A9: cargo test -p deck-streak-economy --test bounded_arithmetic -- --exact an_empty_population_is_refused
A10: cargo test -p deck-streak-kernel --test allow_list_arms -- --exact each_arm_finds_its_planted_site_and_nothing_else
A11: cargo test -p deck-streak-kernel --test allow_list_arms -- --exact an_unlisted_recounted_or_stale_arm_row_is_refused
A12: cargo test -p deck-streak-kernel --test allow_list_arms -- --exact every_arm_site_on_the_tree_is_listed_with_its_reason
A13: cargo test -p deck-streak-kernel --test allow_list_arms -- --exact an_empty_source_set_is_void_and_every_source_kind_is_read
```

Every criterion is red first, at the red commit, on an assertion of its behaviour. The red commit
holds every test, both fixtures and the three registers, with stubs that keep every input and drop
the behaviour: each census returns an empty result without listing or reading any file, so it finds
no site and refuses no empty population, and each register check accepts whatever it is given. A
tree test asserts its sites against its register before it asserts any count, so A4, A8 and A12
are red on the register; a refusal test asserts each refusal's reason, so A3, A7 and A11 are red on
a refusal that never comes; A5, A9 and A13 are red on their empty population, which the stub
accepts, and on their positive control, which the stub does not examine. A4, A8 and A12 read the
real tree, so they run in CI alone: their red is read on the red commit's push by name, and their
green on the next push (section 7). Every other criterion reads only a fixture or a temporary
tree, and is read red and green locally by cargo as well as in CI. Every expected value is a
literal the test spells, never one the code under test builds, and every enumerating test prints
its examined count and refuses zero.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/kernel/tests/additive_migrations.rs` | `deck-streak-kernel`, test code | added: the migration census (R1-R7), `ADMITTED`, the tests of A1-A5 |
| `crates/kernel/tests/allow_list_arms.rs` | `deck-streak-kernel`, test code | added: the escape-hatch census (R14-R17), `ARMS`, the tests of A10-A13 |
| `crates/kernel/tests/fixtures/allow_list_arms/plants.rs.fixture` | `deck-streak-kernel`, test fixture | added: A10's planted source, never compiled |
| `crates/economy/tests/bounded_arithmetic.rs` | `deck-streak-economy`, test code | added: the arithmetic census (R8-R13), `LISTED`, the tests of A6-A9 |
| `crates/economy/tests/fixtures/bounded_arithmetic/plants.rs.fixture` | `deck-streak-economy`, test fixture | added: A6's planted source, never compiled |
| `scripts/mutation-rows.d/S43500-S43599.json` | the mutation rows | added: the rows of section 9 |
| `docs/specs/SPEC-435-a-destructive-migration-names-its-ruling-and-every-unchecked-money-or-counter-operator-sql-escape-shell-and-environment-read-is-listed.md` | the SPEC | added: this file |
| `docs/decisions/ADR-449-three-census-tests-hold-todays-destructive-migrations-unchecked-operators-and-escape-hatches-in-registers-and-refuse-any-other.md` | the decision | added |
| `docs/schematics/data-integrity-ii-checks.md` | the schematic | added: the three censuses, their populations, matchers, counts and sites |
| `docs/red-first/SPEC-435.md` | the red-first record | added |
| `changelog.d/data-integrity-ii-435.md` | the changelog fragment | added |

## 5. What this does NOT cover

- It sets no release overflow checks: the shipped build keeps Cargo's release defaults, #473.
- It adds no clippy lint to the workspace's tables, `arithmetic_side_effects` and
  `disallowed_methods` included, and it holds no other recommended lint rule; each is its own
  delivery (#901).
- It adds no integer-money, append-only or idempotent-sync check; those are held by their own
  delivery (#902).
- It cures no listed site: no money or counter sum is rewritten into a checked or saturating
  form, and no register row is removed (#903).
- It reads no counter outside the six crates of R8, such as those of `analytics`, `curriculum`,
  `notifications`, `ingest`, `kernel` and `coordination` (#904).
- It examines no test code under `crates/*/tests/` for the arms of R15 (#905).

## 6. Risks

- The separate reading of section 1 read 32 of the 33 migrations and 341 of the 349 sources, so
  section 1 lists the sites of those files alone. A site the census finds in one of the other nine
  files turns A4 or A12 red in CI, by its `path:line` and class: closed, never silent.
- A destructive statement in a form the word split misreads lands unseen: a class word inside a
  bracketed or backticked identifier reads as a word, and a column declared
  `ON CONFLICT REPLACE` deletes rows at run time with no `OR REPLACE` in the text. ADR-449 names
  each false-positive and false-negative class; a class the census misses grows the census by its
  own delivery.
- A statement inside a trigger body is judged as a statement, so a trigger that deletes rows
  reads "deletes rows" and needs a row whose reason says so.
- The arithmetic census reads text, so it cannot type an operand: a float or a `String` operand
  is a site, absorbed by its row's class, and a sum a macro expands to, an aliased `sum` or an
  operator outside the set (`<<`, `/`, `%`, `pow`, a unary `-`) is outside it.
- An escape reaches the tree through an aliased import or a function passed without a call. The
  arms read only the spelled forms; a new form grows the arms by its own delivery.
- A change that moves any population moves a census's count, and the census fails until the same
  pull request updates its register. That is the check working, and it is closed, never silent.
- A register that only grows is retired on evidence by ADR-449 D6's kill criteria, never by a
  number of days.

## 7. What only CI proves

- A4, A8 and A12, the three tests over the real tree, run in CI alone: their red is read on the
  red commit's push and their green on the next push, by name, in the `rust` job. Locally, the
  builder reads every other red and green of the two touched packages by cargo, over fixtures and
  temporary trees only. CI's `rust` job runs every package's suite and clippy with warnings
  denied, and it alone reads the workspace whole.
- The censuses read the tree CI checks out: a sibling pull request that lands first and moves a
  population turns this one red in CI until its register is updated.

## 8. Formal model

No TLA+ model: no actor and no shared state is added, only three read-only walks under
`cargo test`. No Lean entry: ADR-449 D8 declines it, because the three matchers are test-side text
classifiers that move no coin, ledger row or release; they are closed by the planted hostile
populations of A1, A2, A6 and A10 and by section 1's counts, made by a separate reading of the
tree, which each census must equal. Of the 251 `@phx covers` lines under `formal/`, none names a
path of section 4. The economy's covers in `formal/tla/WalletFloor`,
`formal/lean/Formal/Wallet.lean` and `formal/lean/Formal/SkipTariff.lean` cover sources the
arithmetic census reads as text and never edits, so no digest moves.

## 9. Mutation rows (S43500-S43599)

74 rows, S43500 to S43520, S43522 to S43525, S43530 to S43556, S43560 to S43578 and S43580 to
S43582, in
`scripts/mutation-rows.d/S43500-S43599.json`, table `MUTATIONS`. Each
row's crate cell is `kernel` or `economy`, its file is `tests/<target>.rs`, its find is taken
from the green commit's text and occurs exactly once in its file, and its killer is
`<target>::<test>`. Each mutates one rule of a census's own code, and its description says the
census is test code, which cargo-mutants never mutates. No find holds the whole removal verb.

| row | target | the rule the mutant disables | killer |
|---|---|---|---|
| S43500 | `additive_migrations` | removes a table | A1 |
| S43501 | `additive_migrations` | removes a column | A1 |
| S43502 | `additive_migrations` | removes an index | A1 |
| S43503 | `additive_migrations` | removes a view | A1 |
| S43504 | `additive_migrations` | removes a trigger | A1 |
| S43505 | `additive_migrations` | renames | A1 |
| S43506 | `additive_migrations` | deletes rows by `DELETE FROM` | A1 |
| S43507 | `additive_migrations` | deletes rows by `OR REPLACE` | A1 |
| S43508 | `additive_migrations` | deletes rows by a statement opening `REPLACE INTO` | A1 |
| S43509 | `additive_migrations` | removes a column only in a statement opening `ALTER TABLE` | A1 |
| S43510 | `additive_migrations` | renames only in a statement opening `ALTER TABLE` | A1 |
| S43511 | `additive_migrations` | `--` comments are blanked | A2 |
| S43512 | `additive_migrations` | `/* */` comments are blanked | A2 |
| S43513 | `additive_migrations` | single-quoted literals are blanked | A2 |
| S43514 | `additive_migrations` | double-quoted identifiers are blanked | A2 |
| S43515 | `additive_migrations` | a statement ends at `;` | A2 |
| S43516 | `additive_migrations` | a statement's line is its first character that is not white space | A2 |
| S43517 | `additive_migrations` | a hit no row lists is refused | A3 |
| S43518 | `additive_migrations` | a recounted row is refused | A3 |
| S43519 | `additive_migrations` | a row whose SPEC omits the file name is refused | A3 |
| S43520 | `additive_migrations` | a row the census does not reach is refused | A3 |
| S43522 | `additive_migrations` | `examined 0` is VOID | A5 |
| S43523 | `additive_migrations` | the removal verb's parts | A1 |
| S43524 | `additive_migrations` | removes a column only in a statement that holds the removal verb | A1 |
| S43525 | `additive_migrations` | renames only in a statement that holds `RENAME` | A1 |
| S43530 | `bounded_arithmetic` | `+=` is a site | A6 |
| S43531 | `bounded_arithmetic` | `-=` is a site | A6 |
| S43532 | `bounded_arithmetic` | `*=` is a site | A6 |
| S43533 | `bounded_arithmetic` | `+` is an operator | A6 |
| S43534 | `bounded_arithmetic` | `-` is an operator | A6 |
| S43535 | `bounded_arithmetic` | `*` is an operator | A6 |
| S43536 | `bounded_arithmetic` | `)` ends an operand | A6 |
| S43537 | `bounded_arithmetic` | `]` ends an operand | A6 |
| S43538 | `bounded_arithmetic` | `?` ends an operand | A6 |
| S43539 | `bounded_arithmetic` | a word ends an operand | A6 |
| S43540 | `bounded_arithmetic` | a keyword ends no operand | A6 |
| S43541 | `bounded_arithmetic` | `->` and `=>` are never sites | A6 |
| S43542 | `bounded_arithmetic` | `#[cfg(test)]` items are blanked | A6 |
| S43543 | `bounded_arithmetic` | a site outside every body is item level | A6 |
| S43544 | `bounded_arithmetic` | the innermost function names a site | A6 |
| S43545 | `bounded_arithmetic` | a character literal reads as an operand | A6 |
| S43546 | `bounded_arithmetic` | `.sum(` is a site | A6 |
| S43547 | `bounded_arithmetic` | `.product(` is a site | A6 |
| S43548 | `bounded_arithmetic` | a turbofish before the call | A6 |
| S43549 | `bounded_arithmetic` | an exponent's sign is part of its literal | A6 |
| S43550 | `bounded_arithmetic` | string literals are blanked | A6 |
| S43551 | `bounded_arithmetic` | `//` comments are blanked | A6 |
| S43552 | `bounded_arithmetic` | `/* */` comments are blanked | A6 |
| S43553 | `bounded_arithmetic` | a function no row lists is refused | A7 |
| S43554 | `bounded_arithmetic` | a recounted row is refused | A7 |
| S43555 | `bounded_arithmetic` | a row the census does not reach is refused | A7 |
| S43556 | `bounded_arithmetic` | `examined 0` is VOID | A9 |
| S43560 | `allow_list_arms` | the SQL escape arm | A10 |
| S43561 | `allow_list_arms` | the SQL escape needs its `(` | A10 |
| S43562 | `allow_list_arms` | `var` is an environment read | A10 |
| S43563 | `allow_list_arms` | `var_os` is an environment read | A10 |
| S43564 | `allow_list_arms` | `vars` is an environment read | A10 |
| S43565 | `allow_list_arms` | `vars_os` is an environment read | A10 |
| S43566 | `allow_list_arms` | `bash` is a shell | A10 |
| S43567 | `allow_list_arms` | `sh` is a shell | A10 |
| S43568 | `allow_list_arms` | `dash` is a shell | A10 |
| S43569 | `allow_list_arms` | `zsh` is a shell | A10 |
| S43570 | `allow_list_arms` | `ksh` is a shell | A10 |
| S43571 | `allow_list_arms` | `fish` is a shell | A10 |
| S43572 | `allow_list_arms` | an argument that is not a literal is a configured program | A10 |
| S43573 | `allow_list_arms` | a literal program that is no shell passes | A10 |
| S43574 | `allow_list_arms` | a shell is judged by its last `/` part | A10 |
| S43575 | `allow_list_arms` | comments and literals are blanked | A10 |
| S43576 | `allow_list_arms` | a site no row lists is refused | A11 |
| S43577 | `allow_list_arms` | a recounted row is refused | A11 |
| S43578 | `allow_list_arms` | a row the census does not reach is refused | A11 |
| S43580 | `allow_list_arms` | `examined 0` is VOID | A13 |
| S43581 | `allow_list_arms` | build scripts join the population | A13 |
| S43582 | `allow_list_arms` | examples join the population | A13 |

S43521, S43526 to S43529, S43557 to S43559, S43579 and S43583 to S43599 are unused. A row on a compound rule has a
plant in which its operand alone decides the result: each form of "deletes rows" has its own
plant, and A1's additive control holds an `ALTER TABLE` that adds a column and a column named
`rename`, so a mutant that drops either half of an `ALTER TABLE` rule finds a hit there.

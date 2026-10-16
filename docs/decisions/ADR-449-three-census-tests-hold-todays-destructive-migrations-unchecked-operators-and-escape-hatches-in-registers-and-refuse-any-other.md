---
status: "accepted"
---

# ADR-449: three census tests hold today's destructive migrations, unchecked operators and escape hatches in registers, and refuse any other

Decides SPEC-435. Path:
`docs/decisions/ADR-449-three-census-tests-hold-todays-destructive-migrations-unchecked-operators-and-escape-hatches-in-registers-and-refuse-any-other.md`.
Amends no SPEC and no ADR. Read at dev `b25ed04a`.

## Context

Three data-integrity rules have prose behind them and no check that holds them.

- **Migrations.** SPEC-020 R18 says "a migration is additive". Nothing reads `migrations/` for a
  destructive statement: `crates/kernel/tests/schema.rs` (`migration_files`, `:80`) checks each
  file's name and that every table is `STRICT` with `created_at`, and no more. At `b25ed04a`
  `migrations/` holds 33 files. Of the 32 that SPEC-435 section 1's separate reading read, one is
  destructive:
  `migrations/004102_notifications_queue_claim.sql` removes a table (`:31`) and renames one (`:32`),
  a STRICT rebuild that SPEC-041 section 8 admits by name ("migration 004102, a rebuild of the
  table"). No other statement of the 32 removes, renames or deletes anything; one comment,
  `migrations/004302_agent_runs_created_at_index.sql:2`, names a row delete that runs in code, not
  in the migration.
- **Arithmetic in money and counter paths.** The shipped build keeps Cargo's release defaults, with
  `overflow-checks = false` (SPEC-330; the workspace sets no `[profile.release]`, #473), so an
  integer sum that overflows in the shipped daemon wraps silently, while the same sum panics under
  `cargo test`. The workspace's clippy table raises `all` and `pedantic` and cherry-picks four
  restriction lints (`Cargo.toml:182-194`); it raises no arithmetic lint. Measured at `b25ed04a`
  over the 57 sources of the six crates that keep coins, prices, XP, levels, streaks, freezes,
  chests, pity counters, tokens, sessions and the writing streak (`economy`, `xp`, `progression`,
  `quests`, `streaks`, `habits`): 114 unchecked operator sites in 46 functions, 3 more at item
  level (constant expressions and a trait bound), and 17 `checked_`, `saturating_`, `wrapping_`
  or `overflowing_` calls. Every one of the 114 is bounded today: by a float operand, a widening
  cast, a constant, cap, ladder or guarantee the code enforces, a real date, or a finite count.
  None of them can overflow under the invariants the code states.
- **Escape hatches.** sqlx accepts a built `String` only inside `AssertSqlSafe`; a process spawn
  can start a shell; and SPEC-020 R10 makes the daemon's `main` the one reader of the process
  environment. Measured at `b25ed04a` by SPEC-435 section 1's separate reading, over 341 of the
  349 sources under `crates/*/src`, `crates/*/build.rs` and `crates/*/examples`:
  one `AssertSqlSafe(` (`crates/ingest/src/reader.rs:372`), five `Command::new(` (one launches
  `bash`, `crates/agent/src/runner.rs:104`; three launch a configured program; one launches
  `python3`), and seven environment reads in six files.

## Decision Drivers

- A rule with sites today lands with those sites listed and refuses any other; a rule with none
  lands at fail.
- A check prints how many items it examined and fails at zero, which is VOID, never a pass.
- Every red is a planted site in a fixture, never in the product tree, and is read by cargo
  before the green exists.
- No product code changes. A listed site that looks like a defect is a question, never a cure.
- The checks stay disjoint from every other census's file and functions.

## Decisions

**D1. A destructive migration is one the census finds, and only a ruling admits it.** The census
reads every `.sql` file in `migrations/`. It blanks `--` and `/* */` comments, single-quoted
literals and double-quoted identifiers, keeping line breaks; splits the text at `;`; upper-cases
each statement and splits it into words of `[A-Z0-9_]`. It finds seven classes, each at most once
per statement: **removes a table** (the removal verb, then `TABLE`), **removes a column** (a
statement opening `ALTER TABLE` that holds the removal verb), **removes an index**, **removes a
view** and **removes a trigger** (the removal verb, then `INDEX`, `VIEW` or `TRIGGER`), **renames**
(a statement opening `ALTER TABLE` that holds `RENAME`) and **deletes rows** (`DELETE FROM`,
`OR REPLACE`, or a statement opening `REPLACE INTO`). A table rebuild is found by its parts: it
removes a table and renames one. The removal verb is joined from two parts at run time and is
spelled whole nowhere in the tree, and every planted statement is built from parts the same way.
The register is `ADMITTED` in `crates/kernel/tests/additive_migrations.rs`: one row per migration
and class, holding the migration's file name, the class, the count, the SPEC that admits it and
the reason. The check holds the census's hits equal to the register, refuses a row whose SPEC file
under `docs/specs/` does not name the migration's file name, and refuses a row the census does not
reach. Today's register is the one migration on dev: `004102_notifications_queue_claim.sql`,
removes a table 1 and renames 1, admitted by SPEC-041. A row is added by the delivery that adds the
destructive migration, in the same pull request, citing its own SPEC, whose text names the
migration. Every hit on dev enters as admitted history; none is cured. Level: the rule has today's
sites, so it lands with them listed, and any other hit fails.

Known false positives: a statement inside a trigger body is judged as a statement (a trigger that
deletes rows reads "deletes rows"), and a class word in a bracketed or backticked identifier is
read as a word. Known false negatives: a migration outside `migrations/` (the kernel's test
fixtures, a migration held in a string), a destructive statement run by code at run time, and an
`UPDATE` that overwrites values. A class word in a comment or a literal is blanked, so it is no
hit, and a statement split across lines is one statement, so it is one hit.

**D2. Bounded arithmetic: the money and counter paths are six crates' sources, and every unchecked
operator in them is listed by its function.** The population is every `.rs` under `src/` of
`economy`, `xp`, `progression`, `quests`, `streaks` and `habits`: the crates that compute coins,
prices, XP, levels, streaks, freezes, chests, pity counters, tokens, session counts and the writing
streak. The matcher blanks comments, strings and character literals (a character literal becomes
an operand), blanks every `#[cfg(test)]` item, and judges only sites inside a function body, named
by the innermost function. A site is a `+=`, `-=` or `*=`; a `+`, `-` or `*` whose previous
significant character ends an operand (`[A-Za-z0-9_)\]?]`) and whose previous word, if it ends one,
is not a keyword of the fixed list SPEC-435 R10 gives; or a `.sum(` or `.product(` call, with or
without a turbofish. `->` and `=>` are never sites. A site outside every function body is item
level: a constant expression, which the compiler evaluates and refuses on overflow, or a trait
bound. It is counted and never judged. `checked_`, `saturating_`, `wrapping_` and `overflowing_`
calls are counted and printed, never judged. The register is `LISTED` in
`crates/economy/tests/bounded_arithmetic.rs`: one row per source and function, with the count of
its sites, its class (`Float`, `Widened`, `Domain`, `Calendar` or `Count`) and the bound in words.
The check holds the census equal to the register, function by function. Level: the measurement
found 114 sites in 46 functions, so the rule lands with all of them listed, and any new site, any
recount and any stale row fails. A new site passes by being written with a bounded form, or by a
row whose reason a reviewer reads.

Known false positives, each absorbed by its row's class: a float operand the text cannot type, a
widening cast before the sum, a loop index or a count over a finite set, a `String` concatenation,
and a trait-bound `+` inside a function body. Known false negatives: a sum a macro's expansion
makes (a `macro_rules!` body is item level, and a derive's output is not text), arithmetic in SQL
text (SQLite's `sum()` raises on integer overflow, and a `+ 1` in a statement is outside the
census), a counter outside the six crates, an aliased call to `sum`, and the operators outside the
set (`<<`, `/`, `%`, `pow`, a unary `-`).

**D3. Allow-list arms: one census, three arms, each with its register.** The population is every
`.rs` under `crates/*/src/`, every `crates/*/build.rs` and every `.rs` under `crates/*/examples/`,
with comments and literals blanked, test modules included. Every source is read: the census names
no file to skip. The arms:
- **an SQL escape**: `AssertSqlSafe` then `(`. One site today, the ingest reader's one door that
  runs a caller's statement on the read-only copy (SPEC-023 R1).
- **a shell from Rust**: every `Command::new(` is examined, and its first argument decides it. A
  string literal whose last `/` part is `bash`, `sh`, `dash`, `zsh`, `ksh` or `fish` is a shell; any
  argument that is not a string literal is a configured program, which a configuration can make a
  shell; another string literal is examined and passes. Five spawns today, of which four are listed:
  the agent runner's `bash` (`crates/agent/src/runner.rs`), the snapshot lister's configured
  command (`crates/daemon/src/snapshot_lister.rs`), the vault gate's configured interpreter
  (`crates/vault/src/staged.rs`) and a test module's re-run of its own binary
  (`crates/ingest/src/skip_write/tests.rs`). The output gate's `python3`
  (`crates/agent/src/gate.rs`) passes.
- **an environment read**: `env` `::` `var`, `var_os`, `vars` or `vars_os`, then `(`. Seven sites in
  six files today: `main`'s read of the whole environment (SPEC-020 R10), the zone pin's `TZ`
  (SPEC-083), the zone test module's child role, the progression build script's census switch,
  and three reads in two examples that are their own `main`.
The register is `ARMS` in `crates/kernel/tests/allow_list_arms.rs`: one row per source and arm,
with the count and the reason. The check holds every arm's sites equal to the register, file by
file. Level: each arm has today's sites, so each lands with them listed, and any other site fails
at the arm's level. The arms stand in their own home and extend no other guard (D4).

Known false positives: a test module under `src/` is examined, so its sites are listed. Known false
negatives: an aliased import (`use std::env::var as read`, `use std::process::Command as Cmd`), a
function passed without a call, the compile-time `env!` and `option_env!` (already a
`disallowed-macros` entry, `clippy.toml:23-26`), a spawn through a crate other than `std` and
`tokio`, a `-c` argument given to a configured program, and test code under `tests/`.

**D4. Three homes, each a Rust integration test in the crate that owns its rule.** The migration
census is `crates/kernel/tests/additive_migrations.rs`, beside `schema.rs`, because the kernel owns
`migrations/` and SPEC-020 R18. The arithmetic census is `crates/economy/tests/bounded_arithmetic.rs`,
because the money is the economy crate's; it reads the five other crates' sources as text, which
adds no dependency edge, as `crates/agent/tests/deck_gate_census.rs` reads every crate's. The arms
are `crates/kernel/tests/allow_list_arms.rs`, because the kernel owns the environment's one reader
(SPEC-020 R10) and the SQL connection (SPEC-020 R17). Each prints `examined N` and its sites as
`path:line class`, never a statement's or a line's text, and fails at zero examined. Each reads
its plants from fixtures: the migration census builds its planted
migrations in a temporary directory at run time, and the other two read
`tests/fixtures/<check>/plants.rs.fixture` in their crates. No new loader-style Python module is
added, so `scripts/tests/test_ci_workflows.py`'s `DYNAMIC_IMPORTS` is untouched. No file and no
function is shared with any other census.

**D5. The listings at dev are the registers, and no product code changes.** Every site the three
censuses find at `b25ed04a` is listed with its reason in SPEC-435 sections 2 and 3, and the
registers equal them. The measurement found no destructive migration without a ruling, no money or
counter sum that can overflow under the code's stated invariants, and no shell, configured program
or environment read without an owner in the tree. A site that a later reading finds to be a defect
is a question to the maintainers with options, never a cure in this delivery.

**D6. Kill criteria, for each of the three checks.**
- **Live precision** is fixes ÷ (fixes + new allow-list entries) over the last 30 merged PRs. A
  check with at least 5 hits whose precision falls below 0.8 proposes a demotion. A register row
  added counts as an allow-list entry; a site rewritten into a bounded form, a migration rewritten
  additive, or an escape removed counts as a fix.
- **A zero-hit class.** No check lands at zero hits. A class or arm the register holds no row for
  (for example "removes a column") that meets 2 or more false hits among its first 5 proposes a
  demotion of that class.
- **Tooling.** `examined 0`, or a check that cannot read its population, is VOID, which blocks. It
  is never a pass.
- **Demotion** is a weakening and goes to the owner as a signed ruling.

**D7. What this delivery does not do.** It cures no listed site, sets no release overflow checks,
adds no lint to the workspace's tables, and holds no other lint rule.
Each is named in SPEC-435 section 5, with its issue.

**D8. Tests and rows.** Thirteen acceptance criteria, each with one test, each red first: the red
commit holds every test, the registers and the fixtures, with stubs that keep every input, read
nothing, return no site and refuse no empty population. Every red is read by `cargo test` at the
red commit; push 1 is that commit alone, for size safety, and push 2 carries
the greens, the rows and the records. The rows are band S43500-S43599: each mutates one rule of a
census's own code, which cargo-mutants never mutates because it is test code, and is killed by
that rule's plant test. Formal: no TLA+ model, because no actor and no shared state is added: three
read-only walks under `cargo test`. No Lean entry: the three matchers are test-side text
classifiers that move no coin, ledger row or release, closed by the planted hostile populations of
A1, A2, A6 and A10 and by SPEC-435 section 1's count, made by a separate reading of the tree, which
each census must equal. No file a `@phx covers` line names is edited, so no digest moves.

**D9. One delivery, no amendment.** SPEC-435 and this ADR record the three rules. SPEC-020 R18
("a migration is additive") and R10 ("the one reader of the process environment") are held by D1
and D3 as written, and SPEC-041 section 8 already names the rebuild D1 admits, so no SPEC or ADR on
dev is amended.

## What each decision was chosen against

### D1

- A database-side guard (an SQLite authorizer callback, or a trigger that refuses DDL at migrate time): rejected because it fires only when a migration runs, after the merge, and needs product code in the kernel's connection path.
- A review-only rule: rejected because SPEC-020 R18's "a migration is additive" is that rule, and nothing in the tree proved that 004102's rebuild had a ruling until it was read by hand.
- A comment in the migration naming a backup: rejected because a comment is not a ruling, nothing resolves it, and a check of it sees neither a rename nor an index removal.
- Curing 004102 into an additive form: rejected because it has applied on every database already, and an applied migration is never edited.
- A name-based exclusion list in committed code: it hides real sites from the census and writes a private boundary into public code.

### D2

- clippy's `arithmetic_side_effects` across the workspace: rejected because it is a restriction lint over every crate, which reports every operator of every type (floats aside) with no register of reasons, so it either floods the 31 crates or is allowed back by the hundred.
- `arithmetic_side_effects` scoped to the six crates by crate attributes: rejected because it is a product code change in six crate roots, and a site it reports can be admitted only by an `#[expect]` at the site, which is a product edit too.
- Converting every listed site to `checked_` or `saturating_`: rejected because it is a cure of 114 sites, none of which can overflow today, and it changes the arithmetic the economy's formal entries cover.
- One register line per file with a total count: rejected because a new site in a long file could only be reasoned about file-wide, and its reason would cover unrelated functions.

### D3

- clippy's `disallowed_methods` for `std::env::var`, `std::env::var_os`, `Command::new` and `AssertSqlSafe` with no register: rejected because each of today's 12 listed sites would need an `#[expect]` in product code, and nothing would hold the set of sites or their reasons in one place.
- A Python text guard over the shipped Rust sources, holding the arms beside rules kept at zero: rejected because a rule kept at zero has no register, a Python module's red is read only in CI, and the guard's file would be shared with another delivery.
- Listing every `Command::new(`, a literal `python3` included: rejected because the defect class is a shell or a program a configuration names; a literal non-shell program carries neither.
- Counting environment reads in `src/` alone: rejected because a build script and an example read the environment as a `main` does, and the arm would miss three of today's seven.
- A name-based exclusion list in committed code: it hides real sites from the census and writes a private boundary into public code.

### D4

- One combined module holding all three rules: rejected because three populations in one file put a red from one rule beside the other two's runs, and the migration census's owner (the kernel) is not the money's owner (the economy).
- Python modules under `scripts/tests/`: rejected because a loader-style module's red is read only in CI, while a Rust test's red is read locally by cargo at the red commit.
- One arithmetic census per crate: rejected because six copies of one matcher would drift apart, and a red in one would say nothing of the other five.

### D5

- Curing a listed site in this delivery: rejected because a cure is a product change with its own tests and rows, and folding it into a listing hides it from the listing's review.
- Listing only the sites that look unsafe: rejected because the register must equal the census, or a new site could not be told from an old one.

### D6

- A kill window of a number of days: rejected because the window reads "the last N merged PRs" here, so a quiet stretch cannot empty it.
- No kill criteria: rejected because a register that only grows is a rule nobody can retire on evidence.

### D7

- Holding the remaining lint rules here: rejected because a delivery holds three to five related rules, and these three share one subject.

### D8

- A wall-clock or timing assertion: rejected because the checks read text, and nothing in them waits.
- A red planted in the product tree: rejected because a red must never change product code, even for one commit.
- A TLA+ model of the census: rejected because a read-only walk under `cargo test` has no second actor and no shared state.

### D9

- An insert-only amendment of SPEC-020 naming SPEC-435: rejected because SPEC-020's sentences stay true as written, and the census is the check that holds them.

## Consequences

- A new destructive migration needs a register row citing a SPEC that names it, in the same pull
  request.
- A new unchecked operator in the six crates needs a bounded form or a row with its reason.
- A new SQL escape, shell, configured program or environment read needs a row with its reason.
- A change that moves any population changes a census's count: the change updates the register in
  the same pull request, and the census fails until it does.

## Confirmation

SPEC-435's acceptance criteria, each read red at the red commit and green at the green commit by
`cargo test`, and recorded in `docs/red-first/SPEC-435.md`. The rows in
`scripts/mutation-rows.d/S43500-S43599.json`, each proved killed by its plant test. Each census's
counts at the build's cut equal SPEC-435 section 1's.

## What would make this wrong

- A destructive migration the census does not see (a class outside the seven, or a statement form
  the word split misreads) lands. Then D1 grows a class, by its own delivery.
- The arithmetic register's precision falls below 0.8 over the last 30 merged PRs. Then D2's
  population or matcher is too wide, and the demotion goes to the owner.
- A site listed `Domain` or `Count` is shown to overflow. Then its row's reason was wrong, and the
  site is cured by its own delivery.
- An escape reaches the tree through a form the arms do not read (an aliased import). Then D3's
  matcher reads that form, by its own delivery.

## More Information

SPEC-020 R10, R17 and R18; SPEC-023 R1; SPEC-041 section 8; SPEC-083; SPEC-330 (#473); ADR-043;
`crates/kernel/tests/schema.rs`; `crates/agent/tests/deck_gate_census.rs`; `clippy.toml`;
`formal/tla/WalletFloor`, the wallet floor model whose invariant bounds `purchase_on`'s subtraction.

# SPEC-427: one guard holds four Rust house rules at zero over the shipped sources

Path:
`docs/specs/SPEC-427-one-guard-holds-four-rust-house-rules-at-zero-over-the-shipped-sources.md`.

- **Issue:** none. This SPEC closes nothing; it lands on `dev` after the release pull request #822.
- **Context(s):** the repository's guard tests (`scripts/tests/`), their rows, and the kernel's
  SPEC's record. No crate, workflow or ruleset changes.
- **Decided by:** ADR-441 (the Rust house rules are one text guard over the shipped sources, each
  rule counting what it examined). SPEC-020 is amended insert-only.
- **Schematic:** `docs/schematics/rust-house-rules-guard.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests, its rows and
  `docs/red-first/SPEC-427.md`.

## 1. The problem, measured

Four rules hold in the shipped Rust sources and nothing the pull request runs refuses their first
break. Each fact below is read at the base by `git show <base>:<path>` and `git grep` over
`crates/*/src`; A8's test prints each rule's examined count over the whole population, and
`docs/red-first/SPEC-427.md` records the counts CI read.

- **SQLite in one place.** `crates/kernel/src/db.rs` builds the options and the pool at `:60` and
  `:67` (`Db::open_with`) and at `:83` and `:88` (`Db::open_foreign_read_only`). SPEC-020 R17
  says no other file does. Outside test code no shipped source names `SqliteConnectOptions`,
  `SqlitePoolOptions`, `PoolOptions` or a `connect*` call on a pool or a connection; four
  integration test files and one `#[cfg(test)]` module (`crates/daemon/src/wiring.rs:1378`) do,
  which is why the population is the shipped code alone.
- **No `format!` into a query.** The shipped sources call `sqlx::query` 14 times, `query_as` 12,
  `query_scalar` 12 and `raw_sql` once, and hold one `AssertSqlSafe` site,
  `crates/ingest/src/reader.rs:372`, which wraps a caller's statement and no `format!`. None of the
  40 sites receives a `format!`, directly or through a name bound to one. The literal-only macros
  (`sqlx::query!` 210, `query_as!` 10, `query_scalar!` 39) take checked literals. Five integration
  test sites pass a `format!` through `AssertSqlSafe`.
- **No secret-named name in a log macro.** Every shipped `tracing` macro is called by its path
  (`tracing::warn!`, `error!`, `info!`, `info_span!`); none is imported bare, no `log` macro and no
  `#[instrument]` is used. None passes a name the list of ADR-441 D3 refuses.
- **Randomness only through `getrandom`.** The shipped draws are `getrandom::fill` at
  `crates/identity/src/linking.rs:89`, `passkeys.rs:185` and `session.rs:262`, and `getrandom::u64`
  at `crates/quests/src/draw.rs:35`; `crates/identity/src/passkeys.rs:413` takes `Uuid::new_v4`,
  whose lock entry depends on `getrandom` and on no `rand`-family package. No member manifest
  depends on `rand`, `rand_core`, `rand_chacha` or `fastrand`.
- **The cost.** At the base the guard's stage, python, ran in CI's `hygiene` job as follows:
  job 116828746762, python stage 727 s. The guard adds about a second.

## 2. Requirements

R1. One test module, `scripts/tests/test_rust_house_rules.py`, holds the four rules and is
    discovered by the python stage's `python3 -m unittest discover -s scripts/tests -p
    'test_*.py'`. It imports only standard modules and `_support`, writes fixtures only under a
    `tempfile.TemporaryDirectory()`, and holds no allow-list and no exclusion by name.
R2. The population is every `crates/*/src/**/*.rs` file, sorted, less each file an out-of-line
    `#[cfg(test)] mod <name>;` declares (resolved from its declaring file as the compiler
    resolves it, its `#[path = "..."]` honoured), and less the span of every `#[cfg(test)]` item
    (from the attribute to the item's closing brace or semicolon, its further attributes
    included). `crates/*/tests`, `crates/*/examples` and each crate's `build.rs` are outside it.
R3. Before a rule reads a file's code, line comments, nested block comments, doc comments and the
    contents of string, byte-string, C-string and raw-string literals and of character literals
    are blanked, every offset and line kept. A format string's text is read from the literal.
R4. `sqlite-one-place` refuses, in a population file other than `crates/kernel/src/db.rs`, the
    identifiers `SqliteConnectOptions`, `SqlitePoolOptions`, `PoolOptions`, `rusqlite` and
    `libsqlite3_sys`; a `connect`, `connect_with`, `connect_lazy` or `connect_lazy_with` call on a
    path ending in `SqlitePool`, `Pool`, `SqliteConnection` or `Connection`, qualified forms
    included; and a `.connect_with(`, `.connect_lazy(` or `.connect_lazy_with(` method call.
R5. `no-format-into-query` examines every call of `sqlx::query`, `query_as`, `query_scalar`,
    `query_with`, `query_as_with`, `query_scalar_with` and `raw_sql` (with its `sqlx::` path, or by
    a bare name the file imports from sqlx; a turbofish allowed), and every `AssertSqlSafe(...)`
    site. It refuses one whose argument holds a `format!`, a `.leak()`, or a name that a `let`
    binds, or an assignment sets, to a `format!` earlier in the same function.
R6. `no-secret-in-log` examines every macro called by a `tracing::` or `log::` path, every bare
    macro whose name the file imports from `tracing` or `log`, and every `#[instrument]` or
    `#[tracing::instrument]` attribute. A call's passed names are its field keys (an identifier, a
    dotted name, or a string literal split at each character that is not a letter or a digit), each
    path chain in a value with call parentheses dropped, the shorthand, `%` and `?` forms, its
    positional format arguments and each inline `{name}` capture; an attribute's are its
    function's parameters unless `skip_all`, less `skip(..)`, plus its `fields(..)`. A `target:`,
    `parent:` or `name:` argument and a `Level::` argument pass no name, and `log`'s `;` ends its
    key-values.
R7. A passed name is refused when a segment of it, split at `_` and at each lower-to-upper case
    change and lower-cased, singular or plural, holds a pair of ADR-441 D3 (`api key`,
    `private key`, `signing key`, `init data`, `session id`); otherwise when its last segment ends
    in an about-name (`count`, `len`, `length`, `size`, `total`, `name`, `path`, `file`, `kind`,
    `hash`, `id`, `at`) it is not refused; otherwise it is refused when a segment holds a part
    (`token`, `secret`, `password`, `credential`, `cookie`, `bearer`, `authorization`, `apikey`,
    `initdata`).
R8. `randomness-through-getrandom` refuses, in a population file, a path through `rand`,
    `rand_core`, `rand_chacha` or `fastrand`; `ring::rand`; the identifiers `OsRng`, `ThreadRng`,
    `thread_rng`, `StdRng`, `SmallRng` and `SeedableRng`; a `use` or `extern crate` of a refused
    crate; and `PyRandom::new`. In each `crates/*/Cargo.toml` it refuses a `dependencies`,
    `build-dependencies` or `target.*.dependencies` entry that names a refused crate by its key or
    its `package`.
R9. A `Uuid::new_v4`, `Uuid::new_v7` or `Uuid::now_v7` path is admitted while the `uuid` package
    of `Cargo.lock` depends on no `rand`-family package, and refused at each site otherwise.
    `getrandom::fill`, `fill_uninit`, `u32` and `u64` are the allowed draws; `RandomState` is not
    matched (ADR-441 D4).
R10. Each rule prints one line per refused site, `<rule>: <path>:<line>: <what>`, and its count
    through `_support.examined` as `examined <n> <unit>`: `shipped source files` for
    `sqlite-one-place`, `query sites` for `no-format-into-query`, `log macro calls` for
    `no-secret-in-log`, and `shipped source files and manifests` for
    `randomness-through-getrandom`. A rule that examined nothing fails.
R11. Over the repository's own tree every rule refuses nothing, and `sqlite-one-place` finds at
    least one opener in `crates/kernel/src/db.rs`.
R12. SPEC-020 gains ADR-441 D9's paragraph at its end, insert-only; the schematic, the red-first
    record and the changelog fragment are added.

## 3. Acceptance criteria of SPEC-427

Every test is in `scripts/tests/test_rust_house_rules.py`. A1 to A7 build fixture trees in a
temporary directory at test time; each compares the exact set of refused sites, or of examined
files, with a set its test spells as literals.

| id | criterion | decided by |
|---|---|---|
| A1 | R4: each opener name, receiver, connect name and the method arm is refused outside `db.rs`, the same text in `db.rs` is not, and a `SqlitePool` used as a type is not | `test_sqlite_is_opened_only_in_the_kernels_database_base` |
| A2 | R5: each family function, an imported bare name and `AssertSqlSafe` are refused with a direct `format!`, a `.leak()`, a `let`-bound and an assigned `format!`; a literal, a name bound in another function and a literal-only macro are not | `test_no_format_reaches_a_query` |
| A3 | R6, R7: each macro form and each passed-name form carrying each part and each pair is refused; each about-name, a `target:` and a `Level::` argument are not; `session_id` is | `test_no_secret_named_name_reaches_a_log_macro` |
| A4 | R8, R9: each refused path, identifier, import, `PyRandom::new` and manifest dependency is refused; `getrandom` draws are not; a uuid constructor is refused under a lock whose uuid depends on `rand` and admitted under one that does not | `test_randomness_is_drawn_only_through_getrandom` |
| A5 | R2: the examined files are the shipped sources, less out-of-line and `#[path]` test modules; an inline `#[cfg(test)]` module's violations, a `tests/` file, an example and a `build.rs` are not refused | `test_the_population_is_the_shipped_sources_without_test_code` |
| A6 | R3: a violation of each rule in a line comment, a nested block comment, a string and a raw string is not refused, while a format string's `{token}` capture is | `test_comments_and_literals_are_not_code_but_a_format_capture_is_read` |
| A7 | R10: each rule over an empty population fails, naming `examined 0` and its unit | `test_each_rule_refuses_an_empty_population` |
| A8 | R10, R11: over the repository, `sqlite-one-place` finds an opener in `db.rs`, every rule refuses nothing, and each prints its examined count above zero | `test_the_repository_holds_every_rule_at_zero` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_sqlite_is_opened_only_in_the_kernels_database_base
A2: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_no_format_reaches_a_query
A3: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_no_secret_named_name_reaches_a_log_macro
A4: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_randomness_is_drawn_only_through_getrandom
A5: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_the_population_is_the_shipped_sources_without_test_code
A6: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_comments_and_literals_are_not_code_but_a_format_capture_is_read
A7: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_each_rule_refuses_an_empty_population
A8: python3 -m unittest discover -s scripts/tests -p test_rust_house_rules.py -k test_the_repository_holds_every_rule_at_zero
```

Every criterion is red at the red commit, on an assertion of its behaviour. The red commit holds
the tests and stubs that keep every input but the missing behaviour: the walk returns every
`crates/*/src/**/*.rs` file with no test module removed, the lexer returns the text unblanked,
each rule's matcher refuses nothing, and the empty-population refusal is not made. A1 to A6 are
red on their refused or examined sets, A7 on the missing refusal, and A8 on its first assertion,
the opener in `db.rs` the stub does not see. Each behaviour's assertion runs before its
`examined` call, so no red is the examined count's refusal standing in for a behaviour.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_rust_house_rules.py` | the guard | added: the walk, the lexer, the four rules and A1 to A8 |
| `scripts/mutation-rows.d/S42700-S42799.json` | the rows | added: S42700-S42795 (section 9) |
| `docs/specs/SPEC-427-one-guard-holds-four-rust-house-rules-at-zero-over-the-shipped-sources.md` | the spec | added |
| `docs/decisions/ADR-441-the-rust-house-rules-are-one-text-guard-over-the-shipped-sources-each-rule-counting-what-it-examined.md` | the decision | added |
| `docs/specs/SPEC-020-kernel-foundation.md` | the kernel's spec | changed: one paragraph appended, insert-only |
| `docs/schematics/rust-house-rules-guard.md` | the schematic | added |
| `docs/red-first/SPEC-427.md` | the record | added |
| `changelog.d/rust-house-rules-427.md` | the fragment | added |

Two censuses read the new module and stay unchanged: `scripts/tests/test_ci_workflows.py`, whose
whole-directory census refuses a dynamic import or a name-held namespace in any test module (the
module adds none), and `scripts/tests/test_temp_hygiene.py`, which reads every
`scripts/tests/*.py` for a temporary path kept past its test (the module keeps none).

## 5. What this does NOT cover

- Landing before the release: this delivery is built and verified now and lands on `dev` only
  after #822.
- An allow-list rule for `AssertSqlSafe` sites, which admits a site by name (#857).
- An allow-list rule for a shell interpreter spawned from Rust, such as
  `crates/agent/src/runner.rs:104` (#858).
- An allow-list rule for environment reads in shipped code (#859).
- A clippy-at-zero rule, which needs a measurement run first (#855).

## 6. Risks

- **A declared class hides a defect.** `RandomState` (`crates/ingest/src/sync.rs:393`), the
  engine's own SQLite and each false-negative class of ADR-441 D1 to D4 pass the guard. Detected by
  review and by the kill criteria's root review; a class that hides a leak becomes a matched arm.
- **A false positive blocks a pull request.** The game's tokens and a model's token counts share
  the word `token`. Detected by the guard's own line naming the site; cured by a rename, never an
  exclusion; a pattern is a demotion proposal under ADR-441 D6.
- **The walk examines less than it claims.** A renamed directory or a glob that stops matching
  empties a rule. Detected by `examined 0`, which fails (A7), and by A8's examined lines.
- **A sibling's addition reds this guard.** The population is the whole shipped tree, so a pull
  request that adds a hit anywhere reads red here; that red is the rule working.
- **The census of test modules refuses the module.** Detected by `test_ci_workflows.py` and
  `test_temp_hygiene.py` in the same `hygiene` run.

## 7. What only CI proves

The module walks every shipped source, so its reds and greens are read in CI's `hygiene` job: the
red at the red commit's push, and the green at the next push's head, which holds the green commit,
the rows and the records. No run reads the green commit alone. CI's `mutation-rows` job proves the
rows.

## 8. Formal model

None. The guard is one process reading the tree's text: no actor, no shared state and no ordering
is added, and the delivery edits no span a `formal/` entry covers.

## 9. Mutation rows (S42700-S42799)

Rows S42700-S42795, in `scripts/mutation-rows.d/S42700-S42799.json` as `SCRIPT_MUTATIONS`, each
on `scripts/tests/test_rust_house_rules.py`, one per member of each closed list and one per arm.
Each find is written from the green commit's landed line, occurs exactly once in the file, and is
killed by the criterion's test named below.

| rows | what each mutant removes or breaks | killer |
|---|---|---|
| S42700-S42704 | one opener name: `SqliteConnectOptions`, `SqlitePoolOptions`, `PoolOptions`, `rusqlite`, `libsqlite3_sys` | A1 |
| S42705-S42708 | one receiver: `SqlitePool`, `Pool`, `SqliteConnection`, `Connection` | A1 |
| S42709-S42712 | one connect name: `connect`, `connect_with`, `connect_lazy`, `connect_lazy_with` | A1 |
| S42713 | the method-call arm | A1 |
| S42714 | the allowed file's path | A1 |
| S42715-S42721 | one family function: `query`, `query_as`, `query_scalar`, `query_with`, `query_as_with`, `query_scalar_with`, `raw_sql` | A2 |
| S42722-S42728 | the `AssertSqlSafe` arm, the imported bare name, the direct `format!`, `.leak()`, the `let` binding, the assignment, the same-function bound | A2 |
| S42729-S42733 | the `tracing::` path, the `log::` path, the bare imported macro, `#[instrument]`, `skip` and `skip_all` | A3 |
| S42734-S42741 | the field key, the literal key, the value's chain, the sigil strip, the positional argument, the inline capture, the skipped keyword and `Level::` arguments, `log`'s `;` | A3, but S42739 (the inline capture) by A6 |
| S42742-S42750 | one part: `token`, `secret`, `password`, `credential`, `cookie`, `bearer`, `authorization`, `apikey`, `initdata` | A3 |
| S42751-S42755 | one pair: `api key`, `private key`, `signing key`, `init data`, `session id` | A3 |
| S42756-S42767 | one about-name: `count`, `len`, `length`, `size`, `total`, `name`, `path`, `file`, `kind`, `hash`, `id`, `at` | A3 |
| S42768 | a pair decided before an about-name | A3 |
| S42769-S42772 | one refused crate path: `rand`, `rand_core`, `rand_chacha`, `fastrand` | A4 |
| S42773-S42779 | `ring::rand`; one identifier: `OsRng`, `ThreadRng`, `thread_rng`, `StdRng`, `SmallRng`, `SeedableRng` | A4 |
| S42780-S42783 | `PyRandom::new`, the `use` and `extern crate` arm, the manifest arm, the lock's uuid arm | A4 |
| S42784-S42787 | the inline `#[cfg(test)]` span, the out-of-line module, `#[path]`, the `*/src/**/*.rs` walk | A5 |
| S42788-S42791 | the line comment, the nested block comment, the string literal, the raw string | A6 |
| S42792-S42795 | one rule's empty-population refusal, in R10's order | A7 |

A row whose mutant no plant can observe is dropped from the band with its reason in the delivery's
record, and never kept as a kill. No row is added to `scripts/mutation-rows.json`.

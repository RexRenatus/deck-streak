---
status: "accepted"
---

# ADR-441: the Rust house rules are one text guard over the shipped sources, each rule counting what it examined

Decides SPEC-427. Path:
`docs/decisions/ADR-441-the-rust-house-rules-are-one-text-guard-over-the-shipped-sources-each-rule-counting-what-it-examined.md`.
Amends SPEC-020 insert-only, through SPEC-427.

## Context

Four rules of the house are true of the shipped Rust sources today and are held by nothing the
tree runs:

- SQLite is opened only in `crates/kernel/src/db.rs`. SPEC-020 R17 says so; its two openers are
  `db.rs:60` and `:67` (`Db::open_with`) and `:83` and `:88` (`Db::open_foreign_read_only`). No
  test of the tree refuses a second opener.
- No `format!` reaches a query. The locked sqlx takes `impl SqlSafeStr` in `query()` and its
  related functions, so a built `String` reaches one only through `AssertSqlSafe` or a leaked
  `&'static str`. The one shipped `AssertSqlSafe` site, `crates/ingest/src/reader.rs:372`, runs a
  caller's statement on a read-only connection and wraps no `format!`.
- No secret-named name is passed to a `tracing` or `log` macro. The redactor (SPEC-020 R12, R13)
  scrubs registered values and the bot token's shape at run time; a name nothing registered is
  not scrubbed.
- Randomness is drawn only through `getrandom`. ADR-024 admits it, and ADR-081 rejected the `rand`
  crate's generator for the chests.

`clippy.toml:11-18` holds the one compiler-side precedent, `disallowed-types` and
`disallowed-methods` for the request client. A census of the shipped sources at the base measured
each rule at zero (SPEC-427 section 1). The guard tests beside `scripts/check.sh`'s `stage_python`
(`scripts/tests/test_callee_offset_loops.py:49`, `test_setting_shapes.py:95`) already walk
`crates/*/src/**/*.rs` as text, print what they examined through `_support.examined`, and refuse
zero.

## Decision Drivers

- A rule at zero stays at zero only if a check the pull request runs refuses the first hit.
- Each rule counts what it examined; an examined count of zero is a failure, never a pass.
- A hit at the base is a question for the maintainer, never a cure folded into the guard.
- No allow-list arm: a rule holds with no site admitted by name.
- The rules are defect-linked: each refuses a shape that has leaked a secret, opened a second
  database base, or drawn a predictable value somewhere, never a style.

## Decisions

**D1. SQLite in one place.** The matcher refuses, in a shipped source other than
`crates/kernel/src/db.rs`:
- the opener names `SqliteConnectOptions`, `SqlitePoolOptions`, `PoolOptions`, `rusqlite` and
  `libsqlite3_sys`, each as a whole identifier;
- a `connect`, `connect_with`, `connect_lazy` or `connect_lazy_with` call on a path ending in
  `SqlitePool`, `Pool`, `SqliteConnection` or `Connection`, a qualified form
  (`<sqlx::SqliteConnection as sqlx::Connection>::connect_with`) included;
- a `.connect_with(`, `.connect_lazy(` or `.connect_lazy_with(` method call on any receiver.

These are every way the locked sqlx opens a SQLite connection or pool (`SqlitePool::connect`,
`connect_with`, `connect_lazy`, `connect_lazy_with`, `SqlitePoolOptions`, `ConnectOptions::connect`
on options) and the two crates that open SQLite directly. The workspace enables sqlx's `sqlite`
driver alone (`Cargo.toml:63`), so the `any` driver's names cannot compile here; a delivery that
enables it extends this list. The engine's collection, opened by `CollectionBuilder::build`
(`crates/ingest/src/engine.rs:533`), and the web engine's storage install
(`crates/web-engine/src/wasm.rs:136`) are the engine's own SQLite, which the engine opens through
its own API; the rule holds the workspace's base, as SPEC-020 R17 states it, and matches neither.
Known false positives: a `PoolOptions`, `Connection::connect*` or `.connect_with(` of another
crate. Known false negatives: a bare `.connect()` on options built in another file; an opener
re-exported from `db.rs` under another name; a build script and an example, outside the
population.

**D2. No `format!` into a query.** The query family is every sqlx function whose statement is
`impl SqlSafeStr`: `sqlx::query`, `query_as`, `query_scalar`, `query_with`, `query_as_with`,
`query_scalar_with` and `raw_sql`, written with the `sqlx::` path or imported from sqlx by name in
the file, plus every `AssertSqlSafe(...)` site. A site is refused when its argument holds a
`format!`, a `.leak()`, or a name that a `let` binds, or an assignment sets, to a `format!` earlier
in the same function. The literal-only macros (`sqlx::query!` and its kin) take a string literal
the compiler checks, so they are counted apart and not judged. Known false negatives, stated: a
statement built in another function and returned; built by `push_str`, `+`, `write!`,
`format_args!`, `to_string` or `replace`; passed through a field, a closure or a collection;
rebound under a second name; a `QueryBuilder` push (no use in the shipped sources); a family
function reached through an alias; an executor method handed a statement directly. Known false
positives: a `format!` whose every part is a fixed name from the code, and a name shadowed by a
literal between its binding and the call.

**D3. No secret-named name in a `tracing` or `log` macro.** The matched calls are every macro
called by a `tracing::` or `log::` path, a bare macro name the file imports from either crate, and
an `#[instrument]` attribute (its function's parameters unless `skip_all`, less `skip(..)`, plus
`fields(..)`). The names a call passes are its field keys (an identifier, a dotted name, or a
string literal split at every character that is not a letter or a digit), every path chain in a
value (`a.b.c()`, the call parentheses dropped), the shorthand and `%` and `?` forms, the
positional format arguments, and every inline `{name}` capture in the format string. A `target:`,
`parent:` or `name:` argument and a `Level::` argument are not passed names, and `log`'s `;`
separates its key-values from its message.

A name is secret-named when one of its segments, split at `_` and at a lower-to-upper case change,
lower-cased, singular or plural, holds:
- a pair: `api key`, `private key`, `signing key`, `init data`, `session id`; or
- a part: `token`, `secret`, `password`, `credential`, `cookie`, `bearer`, `authorization`,
  `apikey`, `initdata`;

except that a chain whose last segment ends in an about-name is about a secret and not the secret,
unless a pair is present: `count`, `len`, `length`, `size`, `total`, `name`, `path`, `file`, `kind`,
`hash`, `id`, `at` (`token_count`, `token.len()`, `secret_name`, `credential_id`). The list is the
tree's own secrets: the bot token, the webhook secret, session ids, linking codes, passkey
credentials, the Mini App's init data, cookies and the sync password. Known false positives: the
game's tokens share the word (`tokens`, `token_bonus_xp`), a usage count of model tokens, and a
secret passed through a function inside the macro (a digest). Known false negatives: a secret
renamed before the macro, a struct holding a secret logged whole by `?`, a field named by a
constant (`{ NAME }`), a wrapper macro's metavariables, a header read by a string name, and the
values `#[instrument(ret, err)]` records.

**D4. Randomness only through `getrandom`.** Allowed: `getrandom::fill`, `fill_uninit`, `u32` and
`u64`, and uuid's random constructors (`Uuid::new_v4`, `new_v7`, `now_v7`) while the lock's `uuid`
package depends on no `rand`-family package, which is uuid's own `getrandom` backend. Refused in a
shipped source: a path through `rand`, `rand_core`, `rand_chacha` or `fastrand`, the lock's
`rand`-family packages; `ring::rand`; `OsRng`, `ThreadRng`, `thread_rng`, `StdRng`, `SmallRng` and
`SeedableRng`; a `use` or `extern crate` of a refused crate; `PyRandom::new`, the kernel's seeded
port of the predecessor's generator (`crates/kernel/src/pynum.rs:314`), which only tests construct;
and, in a member's `Cargo.toml`, a dependency on a refused crate by its key or its `package`.
The standard library's `RandomState`, used as a value source at `crates/ingest/src/sync.rs:393` for
retry jitter, is a declared class the rule does not match: its keys come from the operating
system's source, and the site needs spread, not secrecy (SPEC-427 section 6 names the risk). A
dependency's own draws (the passkey ceremonies, the push encryption, the engine) and a hand-written
generator are outside the matcher.

**D5. One guard and its population.** One test module, `scripts/tests/test_rust_house_rules.py`,
holds the four rules, one lexer and one walk, and `stage_python` runs it in CI's `hygiene` job with
the other guard tests. Its population is every `crates/*/src/**/*.rs` file, less each
`#[cfg(test)]` item's span and each file an out-of-line `#[cfg(test)] mod` declares (its
`#[path]` honoured). Comments, doc comments and the contents of string, byte-string and raw-string
literals are blanked before a rule reads the code; a format string's inline captures are read from
the literal. Each rule prints `examined <n> <unit>` through `_support.examined` and fails at zero.
The walk has no name exclusion: every shipped file is judged, and the verdict is CI's.

**D6. Kill criteria.** Each rule is judged over the last 30 merged PRs:
- **Live precision.** Fixes ÷ (fixes + new suppressions or allow-list entries). A fix is a change
  that removes a site the rule refused; a suppression is any edit that narrows the rule's
  population, list or matcher. Below 0.8 with at least 5 hits, a demotion is proposed.
- **First hits.** If 2 or more of a rule's first 5 hits are false, a demotion is proposed.
- **Demotion.** A demotion is a weakening and goes to the owner as a signed ruling.
- **Tooling.** `examined 0`, or a missing or off-pin interpreter, is VOID, which blocks; it is
  never a pass.
- **Root review.** More than 2 allow-list additions over the last 30 merged PRs opens a review of
  the rule's design, since this delivery admits none.

**D7. Not in this delivery.** The allow-list rules for `AssertSqlSafe`, a shell interpreter
spawned from Rust (`crates/agent/src/runner.rs:104`) and environment reads, and a clippy-at-zero
measurement. Each needs an allow-list or a measurement this delivery does not make; SPEC-427
section 5 cites each by its issue.

**D8. Tests and rows.** A1 to A8 (SPEC-427 section 3) are structural: fixture trees written into a
temporary directory at test time, and the repository's own tree; no assertion reads a clock. Their
reds and greens are read in CI's `hygiene` job, at two pushes: the red commit alone, then the green
commit with the rows and the records. Rows S42700-S42795 (SPEC-427 section 9) mutate the module's
own lists, arms, walk and lexer, each killed by its criterion's test. No formal entry is written:
the guard is one process reading text, with no actor or shared state, and it edits no span an
existing entry covers.

**D9. The record.** One delivery under SPEC-427 and ADR-441. SPEC-020 gains one paragraph,
insert-only at its end: R17's one-place sentence and R12's redactor now have a static guard in the
tree. A new schematic, `docs/schematics/rust-house-rules-guard.md`, draws the guard.

## What it was chosen against

### D1

- A `disallowed-types` entry in `clippy.toml` for the options and pools: rejected because the list applies workspace-wide, so `db.rs`'s own openers would need an `#[expect]` at each site, an allow-list arm.
- Matching `SqliteConnectOptions::new(` alone: rejected because a pool opened from a URL (`SqlitePool::connect("sqlite:...")`) builds no options by that name.
- Counting the engine's `CollectionBuilder` as an opener: rejected because it is a hit at the base in the engine adapter, which opens the engine's own SQLite; admitting it needs an allow-list arm.
- A generic `.connect(` method arm: rejected because it refuses every socket and client connect a future service writes, for no hit the named arms miss at the base.

### D2

- Refusing every `AssertSqlSafe` site: rejected because it is a hit at the base (`crates/ingest/src/reader.rs:372`) whose admission needs an allow-list arm.
- Relying on `SqlSafeStr` alone: rejected because `AssertSqlSafe` and `String::leak` pass a `format!` the type system admits.
- Following a statement across functions: rejected because it needs a Rust parser and call graph the text guard does not carry, for no hit at the base.

### D3

- The runtime redactor alone: rejected because it scrubs registered values and the token shape only, so a name passed before registration, or never registered, leaves verbatim.
- A clippy lint: rejected because no clippy lint reads a macro's field names or its format captures.
- Judging a chain's last segment alone: rejected because `cfg.bot_token.expose()` ends in a method name and would pass.
- Refusing every name holding `key` or `code`: rejected because the base passes `key`, `celebration.key`, `landmark.key` and `row.dedupe_key` and an HTTP status `code`, none a secret.
- Counting only the five level macros: rejected because `event!`, `span!` and the `*_span!` macros take the same fields.

### D4

- Refusing `RandomState`: rejected because it is a hit at the base (`crates/ingest/src/sync.rs:393`) for retry jitter that needs spread, not secrecy, so it is no defect.
- Refusing uuid's random constructors outright: rejected because `crates/identity/src/passkeys.rs:413` draws its user handle through uuid's `getrandom` backend, a hit that is no defect.
- Ignoring uuid: rejected because a feature any crate of the graph enables moves its constructors to `rand` with no change to the call site.
- Refusing every `rand`-family package in the lock: rejected because the engine and the passkey crate bring them, a hit no delivery of this workspace can cure.

### D5

- Four test modules, one per rule: rejected because they would walk and lex the tree four times with four lexers that drift.
- A clippy lint: rejected because `clippy.toml`'s lists cannot scope by file, read a macro's fields, or see a `format!` through `AssertSqlSafe`.
- A Rust test crate under `crates/`: rejected because it adds a workspace member and a parser to the build for a check the text walk answers.
- A `scripts/` program with its own test: rejected because it joins the Python mutation population and its capped share, where the house's guards are test modules with hand rows.
- A name exclusion in the walk: rejected because it leaves shipped files unjudged, which narrows the population.

### D6

- A window of days: rejected because the rate of merges varies, so a fixed number of merged pull requests measures a stable amount of use.
- Demoting a rule automatically on the threshold: rejected because a demotion is a weakening, which only the owner admits.

### D7

- Folding the allow-list rules in: rejected because each needs an allow-list arm, which this delivery does not admit.
- A clippy-at-zero rule now: rejected because its count needs a measurement run first.

### D8

- Reading each red on a workstation: rejected because the module walks every shipped file, and its verdict is CI's.
- A formal model of the guard: rejected because the guard has one actor and no shared state, and no covered span moves.

### D9

- Amending ADR-008 instead of SPEC-020: rejected because R17's one-place sentence is SPEC-020's, and ADR-008 decides the store, not its checks.
- An insert into an existing schematic: rejected because no schematic draws the kernel's database base or the guard tests' walk.

## Consequences

- A pull request that opens SQLite outside `db.rs`, passes a `format!` to a query, logs a
  secret-named name, or draws randomness outside `getrandom` reads red in `hygiene`, naming the
  file, the line and the rule.
- The python stage gains one module, of about a second.
- A false positive is cured by renaming or restructuring the site, never by an exclusion; a
  pattern of false positives is a demotion proposal under D6.

## Confirmation

SPEC-427's fence: A1 to A7 over fixture trees, A8 over the repository's own tree, and the rows
S42700-S42795 in CI's `mutation-rows` job.

## What would make this wrong

- A rule's live precision below 0.8 with at least 5 hits, or 2 false hits of its first 5 (D6).
- A shipped site the declared classes hide that leaks a secret, opens a second base, or draws a
  predictable value: the class becomes a matched arm.
- The guard's cost in the python stage growing past a few seconds.

## More Information

SPEC-427; SPEC-020 R12, R13 and R17; ADR-008; ADR-024; ADR-081; `clippy.toml:11-18`;
`scripts/tests/_support.py:10` (`examined`).

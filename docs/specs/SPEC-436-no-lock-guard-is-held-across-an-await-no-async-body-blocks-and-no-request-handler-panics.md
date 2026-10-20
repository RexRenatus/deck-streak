# SPEC-436: no lock guard is held across an await, no async body blocks, and no request handler panics

Path:
`docs/specs/SPEC-436-no-lock-guard-is-held-across-an-await-no-async-body-blocks-and-no-request-handler-panics.md`.

- **Issue:** none. This SPEC refers to #822, the release pull request, after which it lands, and
  closes nothing.
- **Context(s):** the workspace lint table (`Cargo.toml`), the daemon's tests and dev-dependencies,
  the lock file, the rows band and the documents. No product source, workflow, `scripts/check.sh`,
  `clippy.toml` or test-runner configuration changes.
- **Decided by:** ADR-450 (the await lints are denied by name, and blocking calls and handler
  panics are token censuses that start at warn). It cites ADR-293, SPEC-002, SPEC-020, SPEC-025
  and SPEC-041, and amends none of them.
- **Schematic:** `docs/schematics/async-robustness-checks.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-436.md`. Read at dev `b25ed04a`.

## 1. The problem, measured

Three defect classes of the server's async code have no check that names them, at dev `b25ed04a`:

- **A lock guard held across an await.** Clippy's `await_holding_lock` and
  `await_holding_refcell_ref` are in its `suspicious` group, which the `all` group holds.
  `Cargo.toml:184` sets `all` to warn at priority -1, and `scripts/check.sh:85-88` runs clippy over
  `--workspace --all-targets` with `-D warnings`, so both already fail the stage, but only through
  the group and the flag together. `git grep -n await_holding b25ed04a -- Cargo.toml clippy.toml
  crates` prints nothing: no table names either lint, and no test refuses a lowering of either one
  or of `suspicious` (SPEC-041 A18's audit covers `all`, `style` and `warnings`, for its own two
  lints). The stage reports no guard held across an await, which this delivery's CI run
  re-derives with the two lints at deny.
- **A blocking call in an async body.** No clippy lint is scoped to async bodies, and
  `disallowed-methods` is crate-wide. Eleven calls sit in async bodies, counted by
  `cargo test -p deck-streak-daemon --test blocking_census -- --exact
  the_repository_lists_exactly_todays_blocking_sites` (A8), which prints each one's key:

  | file | function | call | ordinal | line at `b25ed04a` |
  |---|---|---|---|---|
  | `crates/agent/src/gate.rs` | `check` | `std::fs::remove_file` | 1 | 205 |
  | `crates/agent/src/gate.rs` | `check` | `std::fs::remove_file` | 2 | 206 |
  | `crates/agent/src/gate.rs` | `check_input` | `std::fs::remove_file` | 1 | 220 |
  | `crates/agent/src/runner.rs` | `run` | `std::fs::remove_file` | 1 | 91 |
  | `crates/ingest/src/skip_write.rs` | `steps` | `std::fs::copy` | 1 | 302 |
  | `crates/ingest/src/skip_write.rs` | `backup` | `std::fs::copy` | 1 | 511 |
  | `crates/ingest/src/skip_write.rs` | `backup` | `std::fs::set_permissions` | 1 | 512 |
  | `crates/ingest/src/skip_write.rs` | `backup` | `std::fs::rename` | 1 | 519 |
  | `crates/ingest/src/skip_write.rs` | `restore_check` | `std::fs::read` | 1 | 795 |
  | `crates/ingest/src/skip_write.rs` | `restore_check` | `std::fs::read` | 2 | 795 |
  | `crates/ingest/src/skip_write.rs` | `restore_check` | `std::fs::write` | 1 | 804 |

  The four agent calls are inside `async move` blocks of synchronous functions that return a
  boxed future. The seven skip-day calls copy, read and write whole collection files on a runtime
  worker; at dev only the ingest crate's tests call the take (`crates/ingest/tests/skip_write.rs`).
  The census examines 338 shipped files: the 340 under `crates/*/src`, less the two out-of-line
  test modules `crates/ingest/src/skip_write/tests.rs` (`crates/ingest/src/skip_write.rs:1087-1088`)
  and `crates/vault/src/unicode_other_tests.rs` (`crates/vault/src/unicode_other.rs:753-755`).
- **A panic in a request handler.** The API and MCP routers answer a panic with a 500
  (`crates/api/src/router.rs:319`, `crates/mcp/src/server.rs:124`); the bot's long poll has none.
  Clippy's `unwrap_used` and `expect_used` (`Cargo.toml:187-188`) already fail an unwrap or expect
  under the stage's flag; nothing refuses `panic!`, `unreachable!`, `todo!`, `unimplemented!` or a
  release-build assertion. The request-handler crates `api`, `bot`, `mcp` and `daemon` hold 55
  shipped files (22, 14, 7 and 12), and none of those forms outside test code, counted by
  `cargo test -p deck-streak-daemon --test handler_panic_census -- --exact
  the_handler_crates_hold_exactly_todays_listing` (A13). Each such form in the four crates sits in
  a `#[cfg(test)]` module (`crates/daemon/src/wiring.rs:816-1783`, `crates/mcp/src/grants.rs:218`,
  `crates/daemon/src/lifecycle.rs:278`, `crates/daemon/src/main.rs:150`,
  `crates/daemon/src/role_job.rs:295`).

## 2. Requirements

The await lints (ADR-450 D1):

R1. `[workspace.lints.clippy]` in `Cargo.toml` holds `await_holding_lock = "deny"` and
    `await_holding_refcell_ref = "deny"` at the default priority, directly after
    `print_stdout = "warn"`, under one comment line that cites SPEC-436 R1.
R2. The audit's manifest reader reads the root manifest's `[workspace.lints.clippy]` table alone,
    with `#` comments removed, and reads a level written plainly (`= "deny"`) or in an inline table
    (`= { level = "deny", ... }`). Each of the two lints passes at `deny` or `forbid`. A missing
    entry, a lower level, an entry only in a comment, an entry in another table, and a
    `suspicious` group entry alone are each refused, naming the lint.
R3. No Rust source under `crates/` (`src`, `tests`, `benches`, `examples` and build scripts) holds
    an attribute, outer, inner or inside a `cfg_attr`, that sets `clippy::await_holding_lock`,
    `clippy::await_holding_refcell_ref` or `clippy::suspicious` to `allow`, `expect` or `warn`,
    except an `expect` with a `reason` that the audit's `SUPPRESSIONS` listing names by file and
    lint. No member manifest's `[lints.clippy]` sets one of the three below `deny`, and no
    `-A`, `--allow`, `-W` or `--warn` flag beside one of the three names appears in `scripts/*.sh`,
    `.github/workflows/*.yml` or a `.cargo/*.toml` file. Attributes are read as tokens, so a name
    in a comment, a doc comment or a string is not an attribute. `SUPPRESSIONS` is empty.
R4. The audit prints `examined <n> manifests`, `examined <n> sources` and `examined <n> flag
    files`, and fails on 0 of any, naming it.

The blocking census (ADR-450 D2):

R5. Its population is each workspace member's `src/**/*.rs`, less each out-of-line test module a
    file declares under `#[cfg(test)]` (`mod x;`, read from `x.rs` or `x/mod.rs` beside the
    declaring file's module directory, or the file a `#[path = ".."]` names). Each file is read as
    `proc-macro2` tokens after one byte order mark is removed; a file that does not lex fails the
    census, naming the file.
R6. An item that carries `#[cfg(test)]` or `#[test]` is test code, and nothing in it is examined.
R7. An async body is an `async fn`'s body, an `async { }` or `async move { }` block, or an async
    closure's body (`async |..| ..`, `async move |..| ..`).
R8. Not an async body: the closure argument of a rail, with everything inside it, nested async
    blocks included, where a rail is a call of `spawn_blocking` or `block_in_place` (any path
    ending in that name) or a `run` method called on a receiver whose last segment is `offload`
    (SPEC-020 R14); the body of a nested `fn` item; a `const` item.
R9. A path is resolved through the file's `use` declarations: a single path, a `{ }` group, a
    `self` member and an `as` rename each bind their last name. A path whose first segment no
    `use` binds is taken as written. A glob import is not resolved.
R10. The blocking set is: `std::fs::read`, `std::fs::read_to_string`, `std::fs::read_dir`,
    `std::fs::write`, `std::fs::copy`, `std::fs::rename`, `std::fs::remove_file`,
    `std::fs::remove_dir`, `std::fs::remove_dir_all`, `std::fs::create_dir`,
    `std::fs::create_dir_all`, `std::fs::metadata`, `std::fs::symlink_metadata`,
    `std::fs::set_permissions`, `std::fs::canonicalize`, `std::fs::File::open`,
    `std::fs::File::create`, `std::fs::File::create_new`, `std::fs::OpenOptions::new`,
    `std::thread::sleep`, `std::net::TcpStream::connect`, `std::net::TcpListener::bind`,
    `std::net::UdpSocket::bind`, `std::process::Command::new`, `std::io::stdin`, any path under
    `reqwest::blocking`, a call whose path ends in `block_on` or a `.block_on(..)` method call, and
    a `.recv()` method call in a file whose `use` declarations name `std::sync::mpsc`.
R11. A site is a call: a resolved path, or a method name, followed by a parenthesised group. A
    path named and not called is not a site.
R12. A site's key is its file, its enclosing function's name, its resolved path (or its method
    form, `.block_on` or `.recv`) and its ordinal among that function's sites of that path, from 1.
    The census prints `examined <n> shipped files` and `examined <n> async bodies`, and fails on 0
    of either, naming it.

The handler panic census (ADR-450 D3):

R13. The request-handler crates are `api`, `bot`, `daemon` and `mcp`, named in the census. The
    census prints `examined <n> shipped files in <k> handler crates`, and fails when a named crate
    has no `src` directory or the population is empty, naming it.
R14. The population is the four crates' shipped sources, read as R5 and R6 read them.
R15. A site is an invocation of `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `assert!`,
    `assert_eq!` or `assert_ne!`, bare or prefixed by `std::` or `core::`, or a method call
    `.unwrap()`, `.expect(..)`, `.unwrap_err()` or `.expect_err(..)`.
R16. Not a site: `debug_assert!`, `debug_assert_eq!` and `debug_assert_ne!`; anything inside a
    `const` item's body or a `const { }` block; test code (R6); `unwrap_or`, `unwrap_or_else`,
    `unwrap_or_default` and any other method name; a name in a comment or a string.
R17. A site's key is its file, its enclosing function's name, its form (`panic!`, `.unwrap`, ...)
    and its ordinal among that function's sites of that form, from 1.

Homes (ADR-450 D4):

R18. The audit is `crates/daemon/tests/await_lints.rs`, the censuses are
    `crates/daemon/tests/blocking_census.rs` and `crates/daemon/tests/handler_panic_census.rs`,
    and both censuses include the shared reader `crates/daemon/tests/shipped_sources/mod.rs` by
    `mod shipped_sources;`. The reader holds the walk, the test-code exclusion, the tokens, the key
    and the level's verdict; the blocking census holds the `use` resolution, the async bodies and
    the rails.
R19. Each census and audit function takes its sources as data, a list of (path, text); only the
    repository tests read the tree, from `CARGO_MANIFEST_DIR`'s parent. No test writes a file.
R20. `crates/daemon/Cargo.toml`'s `[dev-dependencies]` gains `proc-macro2.workspace = true` under
    one comment line citing ADR-293 and SPEC-436, and `Cargo.lock` gains that one dependency edge
    and no package.

Levels and listings (ADR-450 D5):

R21. The reader's `Level` has two values, `Warn` and `Fail`. Its verdict over the found sites and a
    listing prints `WARN <rule> <key>` for each unlisted site and passes at `Warn`, fails naming
    each unlisted site at `Fail`, and fails naming `STALE <rule> <key>` for each listed entry no
    site matches, at both levels. `<rule>` is `blocking-in-async` or `handler-panic`; a key prints
    as `<file> <function> <path or form> #<ordinal>`.
R22. The blocking census's `LEVEL` is `Level::Warn` and its `LISTED` holds exactly the eleven keys
    of section 1, in that order.
R23. The handler panic census's `LEVEL` is `Level::Warn` and its `LISTED` is empty.
R24. Moving a `LEVEL` to `Fail` is its own delivery, under ADR-450 D6.

## 3. Acceptance criteria of SPEC-436

| id | criterion | decided by |
|---|---|---|
| A1 | R1: the repository's root manifest names both lints at `deny` in `[workspace.lints.clippy]` | `await_lints::the_workspace_denies_both_await_lints` |
| A2 | R2: each of a missing entry, `warn`, `allow`, an inline-table `warn`, a commented entry, an entry under `[workspace.lints.rust]` and a `suspicious = "deny"` alone is refused naming its lint; a plain `deny`, an inline-table `deny` and a `forbid` pass | `await_lints::a_missing_lowered_or_misplaced_await_lint_entry_is_refused` |
| A3 | R3: each of the three names set to each of `allow`, `expect` and `warn` by an outer, an inner and a `cfg_attr` attribute, by a member manifest and by each flag spelling is refused naming its file; the names in a line comment, a doc comment and a string literal, and a listed `expect` with a reason, are not | `await_lints::a_lowering_of_an_await_lint_or_its_group_is_refused_wherever_it_is_planted` |
| A4 | R3, R4: over the repository, the audit examined the root manifest and every `crates/*/Cargo.toml`, every `.rs` file under `crates/` and every flag file, refuses nothing, and prints each count above zero | `await_lints::the_repository_lowers_no_await_lint_anywhere` |
| A5 | R9, R10, R11: each member of the blocking set called inside an `async fn` is found once fully qualified and once through a single `use`, a group, a `self` member and an `as` rename, keyed as R12 says | `blocking_census::every_blocking_call_is_found_in_an_async_fn_body_however_it_is_named` |
| A6 | R7: a blocking call in an `async { }` block, an `async move { }` block inside a synchronous function, a boxed `async move` block and an async closure is found | `blocking_census::an_async_block_and_an_async_closure_are_async_bodies` |
| A7 | R5, R6, R8, R11: the same calls inside each rail's closure (with a nested async block), a nested `fn`, a `#[cfg(test)]` module, a `#[test]` function, an out-of-line and a `#[path]` test module, a synchronous function and a `const` item are not found, nor are `tokio::fs::read`, `tokio::time::sleep`, `tokio::process::Command::new` imported by `use`, a path handed and not called, `std::fs::Permissions::from_mode`, and a call in a comment or a string | `blocking_census::a_rail_a_nested_fn_test_code_and_the_async_equivalents_are_not_blocking_sites` |
| A8 | R5, R12, R22: over the repository, the found keys equal `LISTED`, the eleven keys of section 1; the examined files include `crates/coordination/src/recompute/mod.rs` and exclude the two out-of-line test modules; `LEVEL` is `Warn`; both counts print above zero | `blocking_census::the_repository_lists_exactly_todays_blocking_sites` |
| A9 | R21: a planted unlisted site passes with its exact `WARN` line at `Warn` and fails naming it at `Fail`; a planted stale entry fails with its exact `STALE` line at both | `blocking_census::the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does` |
| A10 | R12: an empty file list fails naming `examined 0 shipped files`, and files with no async body fail naming `examined 0 async bodies` | `blocking_census::the_blocking_census_refuses_an_empty_population` |
| A11 | R15, R17: each macro, bare, `std::`- and `core::`-prefixed, and each method form inside a handler crate's function is found, keyed as R17 says | `handler_panic_census::every_panic_form_in_a_handler_crate_is_found` |
| A12 | R16: the three `debug_assert` forms, a site in a `const` item and in a `const { }` block, test code, `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, and a form in a comment or a string are not found | `handler_panic_census::test_code_debug_assertions_const_contexts_and_lookalikes_are_not_handler_panics` |
| A13 | R13, R14, R23: over the repository, the examined crates are exactly `api`, `bot`, `daemon` and `mcp`, the found keys equal the empty `LISTED`, `LEVEL` is `Warn`, and the count prints above zero | `handler_panic_census::the_handler_crates_hold_exactly_todays_listing` |
| A14 | R13: an empty population, and a named crate with no `src` directory, each fail naming it | `handler_panic_census::the_panic_census_refuses_an_empty_population` |

```acceptance
A1: cargo test -p deck-streak-daemon --test await_lints -- --exact the_workspace_denies_both_await_lints
A2: cargo test -p deck-streak-daemon --test await_lints -- --exact a_missing_lowered_or_misplaced_await_lint_entry_is_refused
A3: cargo test -p deck-streak-daemon --test await_lints -- --exact a_lowering_of_an_await_lint_or_its_group_is_refused_wherever_it_is_planted
A4: cargo test -p deck-streak-daemon --test await_lints -- --exact the_repository_lowers_no_await_lint_anywhere
A5: cargo test -p deck-streak-daemon --test blocking_census -- --exact every_blocking_call_is_found_in_an_async_fn_body_however_it_is_named
A6: cargo test -p deck-streak-daemon --test blocking_census -- --exact an_async_block_and_an_async_closure_are_async_bodies
A7: cargo test -p deck-streak-daemon --test blocking_census -- --exact a_rail_a_nested_fn_test_code_and_the_async_equivalents_are_not_blocking_sites
A8: cargo test -p deck-streak-daemon --test blocking_census -- --exact the_repository_lists_exactly_todays_blocking_sites
A9: cargo test -p deck-streak-daemon --test blocking_census -- --exact the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A10: cargo test -p deck-streak-daemon --test blocking_census -- --exact the_blocking_census_refuses_an_empty_population
A11: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact every_panic_form_in_a_handler_crate_is_found
A12: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact test_code_debug_assertions_const_contexts_and_lookalikes_are_not_handler_panics
A13: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact the_handler_crates_hold_exactly_todays_listing
A14: cargo test -p deck-streak-daemon --test handler_panic_census -- --exact the_panic_census_refuses_an_empty_population
```

Every planted control is text the test spells literally, and every expected key, line and count is
a literal the test writes, never a value the census computes. Each test asserts its positive
artifact (the found keys, the printed lines, the examined crates) before any examined count is
refused.

## 4. File manifest

| path | part | change |
|---|---|---|
| `Cargo.toml` | the workspace lint table | changed: R1's comment line and two entries, insert-only |
| `Cargo.lock` | the lock | changed: the daemon's dependency list gains `proc-macro2` (R20) |
| `crates/daemon/Cargo.toml` | the daemon's dev-dependencies | changed: R20's comment line and entry, insert-only |
| `crates/daemon/tests/await_lints.rs` | the await lints' audit | added: R2 to R4, A1 to A4 |
| `crates/daemon/tests/blocking_census.rs` | the blocking census | added: R7 to R12, R22, A5 to A10 |
| `crates/daemon/tests/handler_panic_census.rs` | the handler panic census | added: R13 to R17, R23, A11 to A14 |
| `crates/daemon/tests/shipped_sources/mod.rs` | the shared reader | added: R5, R6, R12's key, R21 |
| `scripts/mutation-rows.d/S43600-S43699.json` | the rows | added: S43600 to S43690 (section 9) |
| `docs/specs/SPEC-436-no-lock-guard-is-held-across-an-await-no-async-body-blocks-and-no-request-handler-panics.md` | the spec | added |
| `docs/decisions/ADR-450-the-await-lints-are-denied-by-name-and-blocking-calls-and-handler-panics-are-token-censuses-that-start-at-warn.md` | the decision | added |
| `docs/schematics/async-robustness-checks.md` | the schematic | added |
| `docs/red-first/SPEC-436.md` | the record | added |
| `changelog.d/async-robustness-436.md` | the fragment | added |

## 5. What this does NOT cover

- It does not land before the release pull request, and changes nothing the release run reads,
  #822.
- It does not touch the test runner's configuration, which the release pull request edits, #822.
- It does not move the skip day's whole-file copy, reads and write onto the blocking pool; those
  seven sites stay listed, #907.
- It does not replace the agent's four staged-file removals with tokio's; those sites stay
  listed, #908.
- It does not keep a passing census's warn lines in CI's logs; they print where a run asks for a
  passing test's output, #909.
- It does not follow a blocking call through a synchronous helper, a method on a value or a set
  member it does not name (ADR-450 D2's false negatives), #910.
- It does not read indexing, slicing, arithmetic or a borrow that can panic in the handler
  crates, or a panic in a crate they call (ADR-450 D3's false negatives), #911.
- It does not check cookie flags, response security headers or outbound fetches, #912.
- It does not add any other lint rule, and lowers no rule in force, #913.
- It does not sweep clippy's other lints that hold zero hits today, #914.

## 6. Risks

- **A site this SPEC's count missed.** The census counts every shipped file; a count at
  the build's cut that differs from section 1 stops the build for a new reading, never a silent
  relisting.
- **A tree-scanning guard reads the new files.** `crates/daemon/tests/workspace.rs` and
  `crates/engine-core/tests/graph.rs` read the daemon's manifest (the entry is written in the
  `name.workspace = true` form both read); `crates/notifications/tests/request_allow_list.rs` reads
  every Rust source for a silenced `all`, `style` or `warnings`, so no planted control spells one;
  `scripts/tests/test_temp_hygiene.py` reads every Rust test, and these write no file;
  `scripts/tests/test_callee_offset_loops.py` reads loops that take their position from a callee,
  and the token walks index by their own counter. Each is run before each push, or, for `request_allow_list.rs`, `test_temp_hygiene.py` and `test_callee_offset_loops.py`, which read the whole tree, in CI at each push.
- **A sibling change adds an async body or a handler panic.** Any pull request that lands first
  and changes shipped sources can move A8 or A13. Detected by the census itself at the merge.
- **The tokens reader refuses a file rustc accepts.** The census fails naming the file; closed.
- **A mutant hangs a token walk.** Each walk advances by its own index over a finite token list,
  and no loop takes its next position from a callee.

## 7. What only CI proves

- The clippy stage over every member with both lints at deny, and the whole workspace's test
  stage, are read in the pull request's CI run.
- `scripts/tests/test_temp_hygiene.py` and `scripts/tests/test_callee_offset_loops.py` are read
  in CI by name.

## 8. Formal model

No state, actor or write path is added, and no covered span is edited: no `@phx covers` line under
`formal/` names a file this delivery edits. The three checks are pure readers of committed text;
their classification is closed by the planted populations of A2, A3, A5 to A7, A11 and A12, whose
expected keys are spelt literally, and by section 9's rows. `formal/tla/SkipDayWrite` covers
`take` and `restore_check` in `crates/ingest/src/skip_write.rs`, which this delivery lists and does
not edit, so their digests stay; the cure tracked by #907 re-reads that model. No model checker
runs.

## 9. Mutation rows (S43600-S43699)

Rows S43600 to S43690, in `scripts/mutation-rows.d/S43600-S43699.json`, each in the crate
`daemon`, its file relative to the crate. Each find is taken from the green commit's text and
occurs exactly once in its file. Each mutant compiles under `-D warnings`.

| rows | file | what each mutant removes or breaks | killer |
|---|---|---|---|
| S43600-S43601 | `tests/await_lints.rs` | one required lint: `await_holding_lock`, `await_holding_refcell_ref` | A1 |
| S43602 | `tests/await_lints.rs` | `deny` among the accepted levels | A1 |
| S43603 | `tests/await_lints.rs` | `forbid` among the accepted levels | A2 |
| S43604 | `tests/await_lints.rs` | the accepted levels admit `warn` | A2 |
| S43605-S43607 | `tests/await_lints.rs` | the clippy table's header, the comment strip, the inline table's `level` | A2 |
| S43608-S43610 | `tests/await_lints.rs` | one audited name: `await_holding_lock`, `await_holding_refcell_ref`, `suspicious` | A3 |
| S43611-S43613 | `tests/await_lints.rs` | one lowering level: `allow`, `expect`, `warn` | A3 |
| S43614-S43616 | `tests/await_lints.rs` | the `cfg_attr` arm, the member-manifest arm, the flag arm | A3 |
| S43617 | `tests/await_lints.rs` | the repository walk's flag-file family | A4 |
| S43618-S43621 | `tests/shipped_sources/mod.rs` | the `#[cfg(test)]` skip, the `#[test]` skip, the out-of-line module, the `#[path]` module | A7 |
| S43622 | `tests/shipped_sources/mod.rs` | the walk's descent into a nested directory | A8 |
| S43623-S43626 | `tests/blocking_census.rs` | one `use` form: the single path, the group, `self`, the `as` rename | A5 |
| S43627 | `tests/blocking_census.rs` | the `async fn` arm | A5 |
| S43628-S43630 | `tests/blocking_census.rs` | the `async` block, the `async move` block, the async closure | A6 |
| S43631-S43635 | `tests/blocking_census.rs` | the nested `fn`, one rail (`spawn_blocking`, `block_in_place`, `offload`'s `run`), the call requirement | A7 |
| S43636-S43638 | `tests/shipped_sources/mod.rs` | the verdict's warn pass, its fail refusal, its stale refusal | A9 |
| S43639-S43640 | `tests/shipped_sources/mod.rs` | the key's ordinal, the key's function name | A8 |
| S43641-S43668 | `tests/blocking_census.rs` | one member of R10's blocking set, in R10's order | A5 |
| S43669 | `tests/blocking_census.rs` | `LEVEL`'s `Warn` | A8 |
| S43670-S43671 | `tests/blocking_census.rs` | the empty-files refusal, the no-async-body refusal | A10 |
| S43672-S43678 | `tests/handler_panic_census.rs` | one macro: `panic`, `unreachable`, `todo`, `unimplemented`, `assert`, `assert_eq`, `assert_ne` | A11 |
| S43679 | `tests/handler_panic_census.rs` | the `std::` and `core::` prefix arm | A11 |
| S43680-S43683 | `tests/handler_panic_census.rs` | one method: `unwrap`, `expect`, `unwrap_err`, `expect_err` | A11 |
| S43684 | `tests/handler_panic_census.rs` | the `const` skip | A12 |
| S43685-S43688 | `tests/handler_panic_census.rs` | one handler crate: `api`, `bot`, `daemon`, `mcp` | A13 |
| S43689 | `tests/handler_panic_census.rs` | `LEVEL`'s `Warn` | A13 |
| S43690 | `tests/handler_panic_census.rs` | the empty-population refusal | A14 |

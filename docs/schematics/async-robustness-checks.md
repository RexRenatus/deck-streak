# Schematic: what the await lints' audit, the blocking census and the handler panic census read

Kind: component, and data flow for each check. Read at DeckStreak dev `b25ed04a`. Decided by
ADR-450; built by SPEC-436.

Three checks hold the server's async code: the await lints, named at deny and read by clippy with
an audit that refuses any lowering of them; a census of blocking calls inside async bodies; and a
census of panic forms in the request-handler crates. The two censuses start at warn.

## Where each check lives

```mermaid
flowchart LR
  k_table["Cargo.toml #91;workspace.lints.clippy#93;: await_holding_lock and await_holding_refcell_ref at deny"]
  k_stage["the lint stage, scripts/check.sh:85-88: cargo clippy over --workspace --all-targets with -D warnings"]
  k_ci["CI's rust job: the lint stage and the test stage"]
  k_audit["crates/daemon/tests/await_lints.rs"]
  k_blk["crates/daemon/tests/blocking_census.rs"]
  k_pan["crates/daemon/tests/handler_panic_census.rs"]
  k_rdr["crates/daemon/tests/shipped_sources/mod.rs: the walk, the test-code exclusion, the tokens, the key, the Level verdict"]
  k_lex["proc-macro2 tokens, a dev-dependency of the daemon, ADR-293"]
  k_mans["the root manifest and the 31 member manifests"]
  k_srcs["every .rs file under crates/"]
  k_flags["flag files: scripts/*.sh, .github/workflows/*.yml, .cargo/*.toml"]
  k_shipped["each member's src/**/*.rs"]
  k_run["cargo test -p deck-streak-daemon, one binary per check"]
  k_table --> k_stage
  k_stage --> k_ci
  k_audit -->|reads the table| k_table
  k_audit -->|reads| k_mans
  k_audit -->|reads attributes| k_srcs
  k_audit -->|reads| k_flags
  k_audit --> k_lex
  k_blk -->|mod shipped_sources| k_rdr
  k_pan -->|mod shipped_sources| k_rdr
  k_blk -->|use resolution, async bodies, rails| k_lex
  k_rdr --> k_lex
  k_rdr -->|reads| k_shipped
  k_audit --> k_run
  k_blk --> k_run
  k_pan --> k_run
  k_run -->|locally, and in| k_ci
```

Clippy reads the types a coroutine keeps across each suspension point, so the await lints have no
census: their population is every target of every member, tests included, and at dev they report
0 in every crate. The three test binaries are read locally by cargo and in CI's `rust` job; the
lint stage over the whole workspace is read in CI. Each census function takes its sources as data,
a list of (path, text), and only the repository tests read the tree, from `CARGO_MANIFEST_DIR`'s
parent; no test writes a file.

## The await lints' audit

```mermaid
flowchart TD
  a_root["the root Cargo.toml"] --> a_tbl["the #91;workspace.lints.clippy#93; table alone, #35; comments removed"]
  a_tbl --> a_lvl{"each await lint's level, plain or in an inline table"}
  a_lvl -->|deny or forbid| a_ok1["passes"]
  a_lvl --> a_bad1["refused, naming the lint: missing, lower, only in a comment, in another table, or only the suspicious group"]
  a_mem["the 31 member manifests"] --> a_mlv{"#91;lints.clippy#93; sets await_holding_lock, await_holding_refcell_ref or suspicious below deny?"}
  a_mlv -->|yes| a_bad2["refused, naming the file"]
  a_src["every .rs file under crates/: src, tests, benches, examples, build scripts"] --> a_tok["proc-macro2 tokens: a name in a comment, a doc comment or a string is not an attribute"]
  a_tok --> a_att{"an outer, inner or cfg_attr attribute sets one of the three names to allow, expect or warn?"}
  a_att -->|an expect with a reason that SUPPRESSIONS lists by file and lint| a_ok2["admitted, and SUPPRESSIONS is empty at dev"]
  a_att -->|any other| a_bad3["refused, naming the file"]
  a_flg["scripts/*.sh, .github/workflows/*.yml, .cargo/*.toml"] --> a_fl{"-A, --allow, -W or --warn beside one of the three names?"}
  a_fl -->|yes| a_bad4["refused, naming the file"]
  a_root --> a_c1["examined #60;n#62; manifests"]
  a_mem --> a_c1
  a_src --> a_c2["examined #60;n#62; sources"]
  a_flg --> a_c3["examined #60;n#62; flag files"]
  a_c1 --> a_void["0 of any count: fails, naming it"]
  a_c2 --> a_void
  a_c3 --> a_void
```

It prints `examined <n> manifests`, `examined <n> sources` and `examined <n> flag files`.

## The blocking census

```mermaid
flowchart TD
  b_walk["walk each member's src/**/*.rs: 340 files"] --> b_drop["less each out-of-line or #35;#91;path#93; test module a file declares under #35;#91;cfg(test)#93;: 2 files"]
  b_drop --> b_files["338 shipped files"]
  b_files --> b_lex["one byte order mark removed, then proc-macro2 tokens"]
  b_lex -->|a file that does not lex| b_badlex["fails, naming the file"]
  b_lex --> b_test["items under #35;#91;cfg(test)#93; or #35;#91;test#93; dropped"]
  b_test --> b_use["the file's use map: a single path, a group, a self member, an as rename, never a glob"]
  b_test --> b_body["async bodies: an async fn's body, an async block, an async move block, an async closure's body"]
  b_body --> b_rail["less a rail's closure, nested async blocks included: spawn_blocking, block_in_place, a run method on a receiver named offload, and less a nested fn and a const item"]
  b_rail --> b_call["calls: a path or a method name followed by a parenthesised group"]
  b_use --> b_call
  b_call --> b_set{"resolved into the 28-member blocking set of SPEC-436 R10?"}
  b_set -->|no| b_quiet["not a site"]
  b_set -->|yes| b_key["key: file, enclosing function, resolved path or method form, ordinal from 1"]
  b_key --> b_ver{"the verdict at Level::Warn against LISTED, 11 keys"}
  b_ver -->|an unlisted site| b_warn["WARN blocking-in-async #60;key#62;, and the test passes"]
  b_ver -->|a listed entry no site matches| b_stale["STALE blocking-in-async #60;key#62;, and the test fails"]
  b_files --> b_c1["examined #60;n#62; shipped files"]
  b_body --> b_c2["examined #60;n#62; async bodies"]
  b_c1 --> b_void["0 of either count: fails, naming it"]
  b_c2 --> b_void
```

It prints `examined <n> shipped files` and `examined <n> async bodies`, then
`WARN blocking-in-async <key>` for each unlisted site and `STALE blocking-in-async <key>` for each
listed entry no site matches. A key prints as `<file> <function> <path or form> #<ordinal>`. At
`Level::Fail` an unlisted site fails, naming it. The two test modules left out are
`crates/ingest/src/skip_write/tests.rs` and `crates/vault/src/unicode_other_tests.rs`.

## The handler panic census

```mermaid
flowchart TD
  p_src["the src of api 22, bot 14, daemon 12 and mcp 7: 55 files"] --> p_excl["the same exclusion and tokens: test modules and test items dropped"]
  p_excl --> p_mac["a macro: panic!, unreachable!, todo!, unimplemented!, assert!, assert_eq!, assert_ne!, bare or after std:: or core::"]
  p_excl --> p_meth["a method call: .unwrap(), .expect(..), .unwrap_err(), .expect_err(..)"]
  p_excl --> p_not["not a site: the three debug_assert forms, a const item's body, a const block, unwrap_or, unwrap_or_else, unwrap_or_default, any other name"]
  p_mac --> p_key["key: file, enclosing function, form, ordinal from 1"]
  p_meth --> p_key
  p_key --> p_ver{"the verdict at Level::Warn against LISTED, empty"}
  p_ver -->|an unlisted site| p_warn["WARN handler-panic #60;key#62;, and the test passes"]
  p_ver -->|a listed entry no site matches| p_stale["STALE handler-panic #60;key#62;, and the test fails"]
  p_src --> p_c1["examined #60;n#62; shipped files in #60;k#62; handler crates"]
  p_c1 --> p_void["an empty population, or a named crate with no src directory: fails, naming it"]
```

It prints `examined <n> shipped files in <k> handler crates`. The four crates are named in the
census, never matched by a pattern.

## Where each runs, and what it examines at dev

| check | runs | examined at dev | sites at dev |
|---|---|---|---|
| the await lints | clippy's lint stage, in CI's `rust` job | every target of every member | 0 |
| the await lints' audit | `cargo test -p deck-streak-daemon --test await_lints`, locally and in CI's `rust` job | the root manifest, the 31 member manifests, every `.rs` file under `crates/` and every flag file | 0 lowerings; `SUPPRESSIONS` empty |
| the blocking census | `cargo test -p deck-streak-daemon --test blocking_census`, locally and in CI's `rust` job | 338 shipped files | 11, listed below |
| the handler panic census | `cargo test -p deck-streak-daemon --test handler_panic_census`, locally and in CI's `rust` job | 55 shipped files in 4 handler crates | 0; `LISTED` empty |

The counts of sources, flag files and async bodies are printed by each run, and each test refuses a
count of 0.

## The blocking sites listed at dev

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

A key holds no line number, so an edit above a site keeps its entry; the line column is where each
site was read at `b25ed04a`. A fix removes the site's entry from `LISTED`, which is how a fix is
counted (ADR-450 D5, D6).

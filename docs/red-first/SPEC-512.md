# Red-first record: SPEC-512

The SPEC, ADR-530 and the three schematics were installed first (69e835fd). The fence's tests came next, whole, over
a stub that keeps every shape but the behaviour (329c28ee): the stub's opener opens any path with no pragma, no header
check and no role check; it embeds no migration, so its file holds no table and no trigger; the store's statement
register and the schema-18 adapter's register are empty; both adapters answer every read with nothing; the
data-rights port exports an empty object and erases nothing; and the seven ingest files still hold their 19
statements. a6ee5c21 turns them green: the base migration, the opener, the store adapter of the read port, the
data-rights port, the schema-18 adapter with the 19 statements moved into it byte for byte, and the map, its
schematic, the lexicon, the privacy inventory, the policy and the CI steps. Each red below is quoted from a run of the
fence's tests on the tree of 329c28ee, each green from a run on the tree of a6ee5c21.

```red-first
A1: red at 329c28ee: crates/collection/tests/opener.rs:62: assertion `left == right` failed: the pragmas of each role (left: ["server: application_id is not set (OpenSettings { application_id: 0, user_version: 0, journal_mode: \"delete\", synchronous: 2, foreign_keys: true, secure_delete: false, query_only: false, busy_timeout_ms: 5000 })", ...], right: []); opener.rs:93: the reader (left: ["the reader is not query_only (...)", ...]); opener.rs:127: the role check (left: ["the server's store as a device's: answered Ok(Store { ... })", ...])
A1: green at a6ee5c21
A2: red at 329c28ee: crates/collection/tests/support/mod.rs:44: examined 0 migration file(s): the population is empty, so nothing was judged; crates/collection/tests/opener.rs:220: a newer store (left: ["the opener answered Ok(Store { ... })", ...], right: [])
A2: green at a6ee5c21
A3: red at 329c28ee: crates/collection/tests/schema.rs:307: assertion `left == right` failed: the tables (left: ["store_meta is missing", "store_meta holds [], not [\"id\", \"role\", \"origin\", \"slot\", \"id_floor\", \"next_seq\", \"import_source\"]", ...], right: []); schema.rs:331: the checks (left: ["the base rows were refused: no such table: study_rule", "store_meta.id: refused for another reason: no such table: store_meta", ...], right: [])
A3: green at a6ee5c21
A4: red at 329c28ee: crates/collection/tests/append_only.rs:97: assertion `left == right` failed: the append-only log (left: ["the base rows were refused: no such table: study_rule", "review_log: an update was not refused as append-only", ...], right: []); append_only.rs:123: the server's order (left: ["server: the base rows were refused: no such table: study_rule", ...]); append_only.rs:144: crates/collection/src/data_rights.rs names the gate it opens
A4: green at a6ee5c21
A5: red at 329c28ee: crates/collection/tests/support/mod.rs:44: examined 0 registered statement(s): the population is empty, so nothing was judged; crates/collection/tests/statement_census.rs:150: assertion `left == right` failed (left: ["crates/collection/src/sql.rs:15: READ_APPLICATION_ID is not registered", ...], right: []); crates/ingest/tests/collection_read.rs:148: examined 0 adapter statement(s): the population is empty, so nothing was judged
A5: green at a6ee5c21
A6: red at 329c28ee: crates/ingest/tests/collection_read.rs:224: assertion `left == right` failed: the moved statements (left: ["DECK_NAMES (from reader.rs) is not in the adapter", "CARDS (from reader.rs) is not in the adapter", ...], right: []); crates/collection/tests/read_port.rs:199: the store adapter (left: ["the fixture rows were refused: no such table: notetypes", "deck names: answered {}, expected {1: \"Default\", 2: \"Two\"}", ...]); crates/ingest/tests/collection_read.rs:979: 3122 finding(s), the first: "case 0 (normal), schema 18: deck names: answered {}, expected {1: \"Default\", 1000: \"Ds Home\", 1001: \"Ds Home\\u{1f}Garnet\"}"; test_context_map_schematic.py: test_every_arrow_is_a_manifest_edge_the_fence_names: AssertionError: findings: 3; test_every_crate_directory_is_one_node_named_by_it: AssertionError: findings: 1
A6: green at a6ee5c21
A8: red at 329c28ee: crates/collection/tests/opener.rs:261: assertion `left == right` failed: foreign files (left: ["no id, one table: the opener answered Ok(Store { ... })", ...], right: []); crates/ingest/tests/collection_read.rs:304: the refusals (left: ["the schema-18 adapter on a store: answered Ok(())", "the store's opener on a schema-18 file: answered Ok(())", "the store's reader on a schema-18 file: answered Ok(())"], right: [])
A8: green at a6ee5c21
A9: not red: the store's builds for wasm32-unknown-unknown and aarch64-apple-ios run only in CI's web-engine and xcframework jobs (SPEC-512 section 7); this delivery's local runs build the host target alone, so the criterion is read by name on CI at the head. Its two workflow censuses read green at a6ee5c21.
A10: red at 329c28ee: crates/collection/tests/support/mod.rs:44: examined 0 declared table(s) (the export, the server's erase and the client record's erase) and examined 0 store table(s) (the inventory); crates/collection/tests/data_rights.rs:204: a device's erase (left: ["the base rows were refused: no such table: study_rule", "<the temporary store path> is still there"], right: [])
A10: green at a6ee5c21
```

## What each pair disclosed

- **Twenty-three of the twenty-four Rust fence tests are RED-FIRST:** each failed by name at 329c28ee (the collection
  crate's six binaries: 19 failed, 1 passed; ingest's `collection_read`: 4 failed) and passed at a6ee5c21 (20 passed;
  4 passed).
- **One fence test is green at the stub by construction:** `no_other_file_of_the_workspace_builds_a_sqlite_connection`
  (A1) holds a rule the base already keeps, since no crate built a `SQLite` connection before the store, and the
  stub's own opener is the one file that does. Its red is its planted control: a second builder in a planted tree is
  refused by file and line.
- **a6ee5c21 changes three fence test files, none of them in what it asserts.** `crates/collection/tests/read_port.rs`
  and `statement_census.rs` change by a lint allowance and a lint fix. `crates/ingest/tests/collection_read.rs` changes
  by lint fixes and by one line of its store loader: 32 golden cases (the 8 of each of the suspended, buried and
  filtered-deck classes, and 8 of the 24 drawn) hold two cards of one note at one ordinal, which schema 18 admits and
  the store's unique note and ordinal refuse,
  so the loader gives a later duplicate the note's next free ordinal. No value of the read port carries the ordinal,
  and every expected value is still computed from the case's own rows.
- **The parity test examined 112 golden cases**, each read through both adapters over three deck scopes, two floors
  and two limits.
- **Ingest's own suites run unchanged and stay green:** at a6ee5c21 `reader`, `study_days`, `window`, `gate`,
  `structure` and `skip_write` read 54 passed, 0 failed, their files unchanged.
- **The service database's symmetry census** (`crates/coordination/tests/data_rights_symmetry.rs`) changes to leave a
  store table to the store's own census, and reads 4 passed at a6ee5c21; the Python censuses of the context map, its
  register, the privacy policy, the CI workflows, the release workflow, the workflow concurrency and the TestFlight
  lanes each read OK at a6ee5c21. `cargo fmt --all --check` exits 0, and `cargo clippy -- -D warnings` exits 0 over the
  collection crate's targets, ingest's library and `collection_read`, and the symmetry census's target.
- **What was not run here:** the workspace's whole test run and clippy (CI's `rust` job), and the store's builds for
  wasm32 and iOS (A9).

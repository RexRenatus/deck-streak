# Red-first record: SPEC-021

The tests were committed (949d1a3) beside stubs that compiled and did the wrong thing, each a first
draft that repeats a gap the predecessor closed or breaks a rule this SPEC adds:

- coordination's registry held the kernel's and ingest's ports and forgot coordination's own;
- the export kept the tables a port empties and left out the singletons it resets, the gap the
  predecessor's dump had until its singletons joined it (`database.py:_ERASE_SINGLETONS` at
  `27ee2bc`);
- the erase ran each port's erase and then emptied every table the port declares, exempt ones
  included, one transaction per port, with no `secure_delete` and no compaction;
- the `data` role erased with or without its confirmation word;
- `privacy.json` and `PRIVACY.md` named `sync-history` alone and disclosed nothing an erase leaves.

Each criterion was run there with the SPEC's own fenced command, and failed by assertion for its own
reason, not by a compile error, a missing fixture or an empty selection. A4's failure names the
schema version table, emptied by the stub's erase; its seeded ledger rows came first and survived,
because the stub's registry never reached coordination's port.

The implementation followed (e522ce8). Between the two commits the tests took lint fixes only:
`SQLite` in backticks in four doc comments, and three JSON comparisons without an owned value.

Six more tests were written with the criteria and were red at 949d1a3 for their own reasons. They
pin what the criteria leave loose, for the diff-scoped mutation job SPEC-039 brings, and carry no
line below:

- `the_erase_transaction_zeroes_the_rows_it_deletes` (privacy, unit): the transaction alone leaves
  no deleted byte in the page it writes to the log, so `secure_delete` is proved apart from the
  compaction;
- `the_compaction_leaves_no_erased_value_in_the_file_or_its_log` (privacy, unit): after a plain
  delete, `VACUUM` and the TRUNCATE checkpoint alone clear both files;
- `a_port_whose_export_leaves_out_a_declared_table_is_refused` and
  `two_ports_declaring_one_table_or_the_schema_key_are_refused` (privacy): the export's refusals;
- `the_data_role_writes_the_export_as_one_line_of_standard_output` (daemon): R8's export;
- `test_the_readme_and_the_about_page_link_the_policy` (repo): the policy's entry points.

After green, three tests pinned what no test above could fail on, found by asking which mutant of
the changed code each test would miss (e19daaa). They passed when written, since the code they pin
was green already, and carry no line below:

- `a_port_whose_erase_leaves_rows_is_refused_and_rolled_back`: the erase's own check that an
  exported table is empty, which no synthetic port had tripped;
- `only_an_answer_of_one_turns_secure_delete_on`: the answer check, drawn out of the transaction so
  a 0, or `FAST`'s 2, can be put to it;
- the export's closing newline, asserted in the export test.

At verification, the orchestrator's hand mutant of the erase's check of a singleton's reset, every
declared column compared with its reset value, survived the whole privacy suite: no synthetic port
had yet left its singleton holding a value its reset row does not.
`a_port_whose_reset_leaves_a_wrong_value_is_refused_and_rolled_back` was written to kill it (row H14
below): a port resets its counter and forgets its streak, and the erase is refused by its context and
table, with every port's work rolled back. It passed when written, and carries no line below.

```red-first
A1: red at 949d1a3: assertion `left == right` failed: the export carries whole exactly the tables the erase clears or resets; left: {"sync_runs"}, right: {"_sqlx_migrations", "ingest_state", "settings_generation", "sync_runs"}
A1: green at e522ce8
A2: red at 949d1a3: assertion `left == right` failed; left: ["cron_fires is declared by no port"], right: []
A2: green at e522ce8
A3: red at 949d1a3: assertion `left == right` failed: the singleton's row after the erase; left: [], right: [(1, 0, 1000)]
A3: green at e522ce8
A4: red at 949d1a3: assertion `left == right` failed: _sqlx_migrations changed across the erase; left: [], right: the four applied migrations
A4: green at e522ce8
A5: red at 949d1a3: the write-ahead log still holds the erased value
A5: green at e522ce8
A6: red at 949d1a3: assertion `left == right` failed: alpha_rows after the failed erase; left: [], right: its three seeded rows
A6: green at e522ce8
A7: red at 949d1a3: assertion `left == right` failed: one key per table a port exports or resets; left: {"alpha_rows", "beta_notes"}, right: {"alpha_rows", "alpha_singleton", "beta_notes"}
A7: green at e522ce8
A8: red at 949d1a3: assertion `left == right` failed; left: ["ingest_state is named by no category", "settings_generation is named by no category"], right: []
A8: green at e522ce8
A9: red at 949d1a3: AssertionError: Lists differ: ['the backup replica', 'the service journal', 'the private copy of the collection', 'the cron-fire ledger'] != []
A9: green at e522ce8
A10: red at 949d1a3: assertion `left == right` failed: ["data", "erase"]: exit status: 0; left: Some(0), right: Some(2)
A10: green at e522ce8
```

## Hand proofs (the builder's, at e19daaa)

The diff-scoped mutation job arrives with SPEC-039, so the erase's and the export's invariants were
proved by hand on the committed tree. Each mutant was installed once, at an anchor that occurs once,
its killer selected exactly one test (`cargo test -j 1 ... -- --exact`), and the file was restored
from `HEAD` and checked by sha256 before the next row.

| row | mutant | killer | verdict |
|---|---|---|---|
| H1 | `crates/privacy/src/erase.rs`: the erase's `PRAGMA secure_delete = ON` becomes `SELECT 1`, so the check passes and `secure_delete` stays off | `the_erase_transaction_zeroes_the_rows_it_deletes` | killed: the erase's page in the log held the deleted row |
| H1b | the same mutant as H1 | A5 | survived: the compaction alone clears both files |
| H2 | the same file: `VACUUM` becomes `SELECT 1` | `the_compaction_leaves_no_erased_value_in_the_file_or_its_log` | killed: the database file held the erased row |
| H2b | the same mutant as H2 | A5 | survived: `secure_delete` alone zeroes the pages the checkpoint copies |
| H3 | the same file: the checkpoint is `PASSIVE`, not `TRUNCATE` | A5 | killed: the write-ahead log held the erased value |
| H4 | the same file: an exported table counts as emptied whatever it holds | `a_port_whose_erase_leaves_rows_is_refused_and_rolled_back` | killed |
| H5 | the same file: the check of every port's work returns at once | A3 | killed: the report named no emptied table |
| H6 | `crates/coordination/src/data_rights_registry.rs`: the registry forgets coordination's port | A2 | killed: `cron_fires` is declared by no port |
| H7 | `crates/daemon/src/role_data.rs`: the confirmation word must differ from `ERASE` | A10 | killed: an unconfirmed erase exited 0 |
| H8 | `crates/privacy/src/lib.rs`: an export that leaves out a declared table is accepted | `a_port_whose_export_leaves_out_a_declared_table_is_refused` | killed |
| H9 | the same file: a table declared twice is let through | `two_ports_declaring_one_table_or_the_schema_key_are_refused` | killed |
| H10 | the same file: a singleton counts as unexported | A7 | killed: the export refused the singleton |
| H11 | `crates/privacy/src/erase.rs`: the checkpoint's busy flag is read inverted | A5 | killed |
| H12 | the same file: any answer but 0 turns `secure_delete` on | `only_an_answer_of_one_turns_secure_delete_on` | killed: `FAST`'s 2 was accepted |
| H13 | `crates/daemon/src/role_data.rs`: the export's line ends with a space, not a newline | `the_data_role_writes_the_export_as_one_line_of_standard_output` | killed |
| H14 | `crates/privacy/src/erase.rs`: a singleton's reset is never compared, `.all(\|(_column, _value)\| true)`, at 58382dd with the test added | `a_port_whose_reset_leaves_a_wrong_value_is_refused_and_rolled_back` | killed: the erase committed the half-reset row and returned `Ok` |

H14 survived every other test: the orchestrator's verification found it. H1b and H2b are why the two
unit tests exist: A5 proves the erase leaves no value behind, and either
`secure_delete` or the compaction alone would pass it, so each is proved apart from the other.

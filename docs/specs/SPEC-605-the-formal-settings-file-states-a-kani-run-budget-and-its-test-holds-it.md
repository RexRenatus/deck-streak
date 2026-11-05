# SPEC-605: the formal settings file states a Kani run budget, and its test holds the file to it

- **Issue:** none; the settings file names no Kani run budget while the tree holds Kani entries.
  **Context(s):** `config/` and its tests (`scripts/tests/`); no crate, harness or workflow changes.
- **Decided by:** ADR-626 (the field, its value and its reading). It amends SPEC-295 R1 and ADR-295 by
  one field, as section 8 states; no earlier byte of either file changes.
- **Status:** delivered by the pull request that adds this file, with its test lines and
  `docs/red-first/SPEC-605.md`. **Mutation band:** S60500-S60599.

## 1. The problem, measured

- **The tree holds Kani harnesses.** At the development branch's commit `e08ba4a`,
  `grep -c 'kani::proof' crates/media/src/kani_proofs.rs crates/sync/src/kani_proofs.rs` counts four
  lines in each file, eight in all. `crates/media/src/kani_proofs.rs` declares `k_blob_path`, `k_bind`,
  `k_sweep` and `k_scan` (its `#[kani::proof]` lines 29, 59, 95 and 126), and
  `crates/sync/src/kani_proofs.rs` declares `sync_codec`, `key_order`, `hlc_tick_no_overflow` and
  `next_id_holds_its_residue_and_bounds` (lines 44, 82, 95 and 107).
- **The settings file states three run budgets and no Kani one.** `git show e08ba4a:config/formal.json`
  holds, under `budgets`, `tla_seconds` 480 (line 4), `lean_seconds` 600 (line 5) and
  `entry_seconds` 300 (line 6), then a map of twelve per-entry budgets (lines 8 to 19), each keyed by a
  `tla/` entry. No line of the file names `kani_seconds`.
- **The formal checker, outside this repository, needs the field for a Kani entry.** It reads
  `budgets.kani_seconds` as the most one Kani run, a harness or a witness, may take. When a selected
  entry is a Kani entry and the file names no such field, it refuses the check as a settings error
  before any run; a run past the field reads as a timeout, never as clean. So every Kani entry of this
  tree is refused today.
- **The settings test holds the file to its own table.** `scripts/tests/test_formal_config.py` states
  the declared file as `EXPECTED` and the reader's rules per field as `FIELDS`, and A1 of SPEC-295
  refuses a file whose fields differ from the table. A field added to the file alone is refused as an
  extra field, so the table and the file move together.

## 2. Requirements

- **R1.** `config/formal.json` states `budgets.kani_seconds` with a value of 300, on its own line
  between `lean_seconds` and `entry_seconds`. No other byte of the file changes, and the file stays the
  settings test's table pretty-printed with an indent of two and a final newline.
- **R2.** The settings test's table `EXPECTED` states the same field with the same value in the same
  place, and its reader field table `FIELDS` gains the field as a positive integer that is optional
  to the reader, after `lean_seconds`. The field is optional to the reader because the formal checker
  reads it as optional and refuses only a selected Kani entry without it. The committed file must
  still carry it: A1 admits only `entry_seconds` and `entries` as absent. No other line of the test
  module changes.
- **R3.** No other field, value or entry budget of the settings file changes, and no per-entry budget
  is added for a Kani entry. No harness, witness, proof, crate or workflow changes, and no Rust file
  changes.
- **R4.** Three mutation rows in `scripts/mutation-rows.d/S60500-S60599.json` pin the field in the
  file, the field in the test's table and the field in the reader's table, each proved killed.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the committed file holds `budgets.kani_seconds` with a value of 300 in R1's place, equal to the test's table, and the file is that table's pretty-printed text (R1, R2) | `test_formal_config.py` `test_the_committed_file_holds_exactly_the_declared_fields` |
| A2 | the test's reader admits the committed file with the field, and refuses each planted fault the field table generates (R2) | `test_formal_config.py` `test_the_reader_refuses_each_planted_fault` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k test_the_committed_file_holds_exactly_the_declared_fields
A2: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k test_the_reader_refuses_each_planted_fault
```

A1 reads red at the red commit, which adds the field to the test's table and to the reader's field
table and leaves the settings file unchanged: its assertion naming `('budgets', 'kani_seconds')`
fails, because the field is absent from the file and is not one of the two fields A1 admits as
absent. It reads green once the file states the field. A2 is not red: the red commit extends the field
table A2 is generated from, and the file without the field is admitted, the field being optional to
the reader. Row S60502 proves A2 can fail.

## 4. File manifest

| path | part | change |
|---|---|---|
| `config/formal.json` | settings | changed: one line, `"kani_seconds": 300,` between `"lean_seconds": 600,` and `"entry_seconds": 300,` (R1) |
| `scripts/tests/test_formal_config.py` | settings tests | changed: two lines, the `EXPECTED` row and the `FIELDS` row (R2) |
| `scripts/mutation-rows.d/S60500-S60599.json` | settings tests | added: three mutation rows (R4) |
| `docs/specs/SPEC-605-the-formal-settings-file-states-a-kani-run-budget-and-its-test-holds-it.md` | docs | added: this delivery's SPEC |
| `docs/decisions/ADR-626-the-kani-run-budget-is-a-settings-field-beside-the-other-run-budgets.md` | docs | added: the field, its value and its reading |
| `docs/schematics/kani-run-budget.md` | docs | added: the data flow from the field to a Kani run |
| `docs/red-first/SPEC-605.md` | docs | added: the red-first record, with its red and green lines |
| `changelog.d/kani-run-budget-605.md` | docs | added: the changelog fragment |

## 5. What this does NOT do

- It changes no other field or value of the settings file and adds no per-entry budget for a Kani
  entry: no Kani run of this tree has been measured whole, and the rule on the entries map's keys is
  an open item of its own (#802).
- It adds no range test for the field: holding each budget field to the range the formal checker
  reads is an open item of its own (#802).
- It changes no harness, witness, proof, crate or workflow, and no Rust file: the field is read by the
  formal checker, outside this repository, and held here by the settings test alone, whose further
  holding of the budget fields is an open item of its own (#802).

## 6. Risks

- **A value too small for a harness.** A Kani run past `budgets.kani_seconds` reads as a timeout,
  never as clean, so a value too small for a harness shows as a timed-out check, not as a silent
  pass. Detected by the formal check's card for the entry.
- **A later raise is its own change.** A larger value is claimed only by a change that measures a
  whole Kani run of this tree, with the file, the test's table and the rows moving together; A1
  refuses a file and a table that disagree.
- **Another delivery edits the same test module.** Its hunks and this one's touch different lines;
  whichever lands second is brought onto the development branch by its lander.

## 7. Mutation rows

| row | file | mutant | killer |
|---|---|---|---|
| `S60500` | `config/formal.json` | the file names a Kani run budget other than the declared one | A1 |
| `S60501` | `scripts/tests/test_formal_config.py` | the test's table names a Kani run budget other than the file's | A1 |
| `S60502` | `scripts/tests/test_formal_config.py` | the reader's field table no longer knows the Kani run budget | A2 |

## 8. What this keeps, supersedes and amends

| clause | reading |
|---|---|
| SPEC-295 R1 | amended by one field: the file also states `budgets.kani_seconds` with a value of 300; every other field and value is kept |
| ADR-295 | amended by one field, `budgets.kani_seconds`, decided by ADR-626; its decision is kept |
| SPEC-295 A1 and A3 | kept: the same tests, generated from the table this change extends |

No byte of SPEC-295 or ADR-295 changes.

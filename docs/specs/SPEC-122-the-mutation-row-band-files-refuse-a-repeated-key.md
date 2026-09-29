# SPEC-122: the mutation-row band files refuse a repeated key, at every read and at any depth

- **Wave:** W4. **Issue:** #334. **Context(s):** `repo` (`scripts/mutation_rows.py`, its tests, the
  band file of this SPEC and `docs/`).
- **Decided by:** ADR-122 (this SPEC's own: a repeated key is refused, and what that was chosen
  against), ADR-057 (the rows, the reader and the runner; it takes a dated note naming ADR-122),
  ADR-016 (a planned SPEC is promoted by the delivery that builds it) and SPEC-039 R8 to R11 (the
  rows, the one reader, the census and the retirement check; it takes a dated amendment).
- **Status:** delivered. It waited in `docs/specs/planned/` from its own commit
  until its tests were green, and the delivery moved it to `docs/specs/` (ADR-016: "the delivery
  that builds it moves it into `docs/specs/` in the same pull request as its tests and its
  red-first record"). It holds `docs/red-first/SPEC-122.md`.

## 1. The problem, measured

Measured at `dev` af0693f.

- **A band file holds `{"tables": {...}}`, and two branches that each add a table under one key
  merge without a conflict.** Take a band file whose `tables` object holds two tables, one per
  line block. One branch adds a `MUTATIONS` table before them and another adds a `MUTATIONS`
  table after them. The hunks do not touch, so `git merge` completes and the file now holds
  `"MUTATIONS"` twice in one object. A1 builds exactly this in a temporary repository and reads
  the merge's exit status.
- **Python's JSON reader keeps the last value of a repeated key and says nothing.**
  `scripts/mutation_rows.py` reads a fragment with `json.loads` (`_fragment`), and the header
  with `json.loads` in `load_tree` and in `load_revision`. Appending a second top-level
  `"tables"` holding one row to `S05700-S05799.json` in a copy of `scripts/` made
  `mutation_rows.py ids` exit 0 and list 197 ids instead of 206: the nine rows of the first
  `tables` were gone, and neither `census` nor `prove` (which read the same population) failed.
- **A revision read goes the same way, and it crashes when it does fail.** `retired --base <rev>`
  reads the base's header and fragments through `git show`, then `_fragment`. `retired` was not
  wrapped in `main`, so a refusal there would have been a traceback, not the exit 2 the other
  verbs give.
- **The readers of the band files** (searched over `scripts/`, `.github/` and `docs/`): the
  module's `load_tree` and `load_revision`, through which `count`, `ids`, `census`, `prove` and
  `retired` read; and `scripts/mutation-verdict.py`, which names the directory in a message and
  reads the population by calling those two functions (`mutation_rows.load_tree`,
  `mutation_rows.load_revision`), never by parsing a band file itself. The retirement record
  `scripts/mutation-rows.retired.json` and a `--rows-from` plan are not band files.

## 2. Requirements

R1. **One parser reads every document the population is made of.** `mutation_rows.parse_document`
    takes the document's name and its text and returns the parsed value, refusing a key that
    appears twice in one object, at any depth, in the top object and in any object inside it.
    Its refusal is a `PopulationRefused` that names the file and the key, in the words held by
    the module constant `REPEATED_KEY`: `<file> repeats the key '<key>' in one object`. Text that
    is not JSON is refused by the same parser as `<file> is not JSON: <reason>`.
R2. **Every read goes through it.** The header (`scripts/mutation-rows.json`) and every fragment
    (`scripts/mutation-rows.d/*.json`) are read through `parse_document`, in the working tree
    (`load_tree`, `tree_fragments`) and in a revision (`load_revision`, whose `git show` text is
    handed to it). No other reader of a band file exists (§1); a new one goes through it too.
R3. **A refusal is a refusal at the verb.** `count` and `ids` exit 2 with
    `mutation_rows: REFUSED: <the refusal>`, `census` reports `the population does not assemble`
    with the refusal and exits 1, `prove` refuses as it already does, and `retired` exits 2 the
    same way as `count`, where it used to raise. A tree with no repeated key reads exactly as it
    did.
R4. **The population on `dev` reads unchanged.** Every committed band file, read by the parser,
    holds the rows a plain `json.load` of it holds (A4).
R5. **The refusal's invariant carries hand-proved rows** in `S12200-S12299` (§7), each killed by
    one test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a base band file with two tables, two branches that each add a table under the key `MUTATIONS` at different places in the file, and a `git merge` that completes without a conflict: the merged file is still valid JSON, and the reader refuses it naming the file and the key | `test_band_repeated_key.py` `a_merge_git_completes_without_a_conflict_and_the_reader_refuses_the_result` |
| A2 | a key repeated inside a fragment's `tables` object and one repeated at the fragment's top are each refused naming the file and the key, at the verb (`ids`, exit 2) and by the library | `test_band_repeated_key.py` `a_key_repeated_at_the_top_or_inside_tables_is_refused_naming_it` and `a_key_repeated_at_any_depth_is_refused_by_the_parser` |
| A3 | `retired --base <rev>`, over a revision whose band file repeats a key, exits 2 naming the file and the key; and a revision whose header repeats a key is refused | `test_band_repeated_key.py` `retired_over_a_revision_whose_band_file_repeats_a_key_is_refused` and `a_revision_whose_header_repeats_a_key_is_refused` |
| A4 | the header in the tree, repeating a key, is refused naming it, and every committed band file reads as a plain `json.load` reads it (the same rows, none dropped) | `test_band_repeated_key.py` `a_header_that_repeats_a_key_is_refused_in_the_tree` and `every_committed_band_file_reads_as_plain_json_reads_it` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k a_merge_git_completes_without_a_conflict_and_the_reader_refuses_the_result
A2: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k a_key_repeated_at_the_top_or_inside_tables_is_refused_naming_it -k a_key_repeated_at_any_depth_is_refused_by_the_parser
A3: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k retired_over_a_revision_whose_band_file_repeats_a_key_is_refused -k a_revision_whose_header_repeats_a_key_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k a_header_that_repeats_a_key_is_refused_in_the_tree -k every_committed_band_file_reads_as_plain_json_reads_it
```

A1 builds its repository at run time in a temporary directory and asserts the merge's exit status
0 before it asserts the refusal, so the fixture proves the hazard is real. A2, A3 and A4 each
select both tests their row names, with two `-k` patterns. A4's control examines every committed
band file and prints its count.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/mutation_rows.py` | repo | changed: R1 to R3, `parse_document`, `REPEATED_KEY`, the readers, `retired`'s refusal |
| `scripts/tests/test_band_repeated_key.py` | repo | added: A1 to A4 |
| `scripts/mutation-rows.d/S12200-S12299.json` | repo | added: R5, six rows |
| `docs/specs/SPEC-122-the-mutation-row-band-files-refuse-a-repeated-key.md` | repo | added, from `docs/specs/planned/` |
| `docs/decisions/ADR-122-a-repeated-key-in-a-band-file-is-refused-at-every-read.md` | repo | added |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | repo | changed: a dated amendment at its end (insert-only) |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | repo | changed: a dated note at its end |
| `docs/red-first/SPEC-122.md` | repo | added |
| `changelog.d/fix-band-repeated-key-122.md` | repo | added |

No schematic: the change adds no component and no state machine. The data flow is one line, and
it is the one SPEC-039's reader already draws: a document's text goes through one parser before
it is assembled, and R2 says where that parser stands.

## 5. What this does NOT do

- It changes no other JSON reader. The retirement record and a `--rows-from` plan keep
  `json.loads`, because #334's wanted list names the band files, and neither is one.
- It adds no merge driver, no attribute and no lint job. A lint that runs only in CI leaves every
  local read wrong (ADR-122), and #334 asks for the refusal in the reader.
- It changes no row's proof, selection or retirement rule. How a row is proved is SPEC-039's
  (#217), and the census's findings are unchanged.
- It builds no generated-mutant class for the gate's own Python. `mutation_rows.py` is one of the
  scripts #218 waits to cover, and this delivery adds hand-proved rows, not a mutation class.
- It changes no Rust and no workflow, because the defect and its fix are in a Python reader (#334).

## 6. Risks

- **A file that repeated a key on purpose would now be refused.** None does: A4 reads every
  committed band file and the header, and a band file written by hand holds one key per table
  (SPEC-039 R8).
- **The refusal is a text match on a key, not a structural rule.** It refuses two spellings of
  one key only when they are the same string after JSON's own unescaping (`"a"` and `"\u0061"`
  are one key to the parser), which is what a consumer that keeps the last value would also
  fold together.

## 7. Amended in delivery, and the hand-proved rows

`S12200-S12299` holds six rows (`SCRIPT_MUTATIONS`), each with one killer that names one test:

| row | mutant | killer |
|---|---|---|
| S12201 | the hook no longer refuses a key it has seen | A1 |
| S12202 | the refusal's words are replaced | A1 |
| S12203 | a revision's header is read by `json.loads` | `a_revision_whose_header_repeats_a_key_is_refused` |
| S12204 | the tree's header is read by `json.loads` | `a_header_that_repeats_a_key_is_refused_in_the_tree` |
| S12205 | a fragment is read by `json.loads` | A1 |
| S12206 | `retired`'s refusal is not caught at the verb | `retired_over_a_revision_whose_band_file_repeats_a_key_is_refused` |

## 8. References

Issue #334; SPEC-039 R8 to R11; ADR-057; ADR-016; ADR-122.

## 9. Amendment, 2026-09-29: the verdict's plan and the equivalence records refuse a repeated key

Issue #345. Two readers outside `mutation_rows.py` still met a repeated key in a way §2 does not
cover. Measured at `dev` b5eb413: `mutation-verdict.py plan` over a tree whose band file repeats a
key ended in an uncaught `PopulationRefused` traceback (exit 1), and a record fragment under
`scripts/mutation-equivalent.d/` that repeats a key, at its top or inside one record, was read by a
plain `json.loads` that keeps the last value and says nothing (the census printed
`examined 0 record(s)` for a fragment whose first `records` list held one).

- **R6. The plan refuses, in one line.** `plan` catches the `PopulationRefused` that
  `mutation_rows.load_tree` and `load_revision` raise, prints `mutation: plan: REFUSED: <the
  sentence>` on stderr and exits 1 with no traceback. The sentence is the one `parse_document`
  forms, `<where> repeats the key '<key>' in one object`, and the plan writes no `plan.json`.
- **R7. The equivalence records are read by the same parser.** `load_records` reads each fragment
  through `mutation_rows.parse_document` with `scripts/mutation-equivalent.d/<name>` as `<where>`,
  so a key repeated at any depth is a named problem that every verb reading the records reports
  (the census as a finding, exit 1; `judge` as a failure). The verdict script already imports
  `mutation_rows`, so it reuses the hook and the sentence; there is no second copy, and nothing
  moves.

The rows join `S12200-S12299` (`SCRIPT_MUTATIONS`), each with one killer that names one test:

| row | mutant | killer |
|---|---|---|
| S12207 | the plan's handler names another exception, so the refusal is a traceback again | `a_band_file_that_repeats_a_key_is_refused_by_the_plan_without_a_traceback` |
| S12208 | `load_records` reads a fragment by `json.loads` | `a_record_fragment_that_repeats_a_key_at_the_top_is_refused` |
| S12209 | `load_records` drops the refusal instead of reporting it | `a_record_that_repeats_a_key_inside_an_entry_is_refused` |

Files added or changed by this amendment: `scripts/mutation-verdict.py` (changed),
`scripts/tests/test_verdict_repeated_key.py` (added), `scripts/mutation-rows.d/S12200-S12299.json`
(three rows), `docs/red-first/SPEC-122.md` (a dated addendum) and
`changelog.d/fix-verdict-repeated-key-345.md` (added). It still changes no Rust and no workflow
(#345).

## 10. Acceptance criteria of the amendment (A5 and A6)

| id | criterion | decided by |
|---|---|---|
| A5 | a band file that repeats a key makes `plan` exit 1 with the whole sentence on stderr and no `Traceback`; a well-formed tree is planned and its selected-row count printed | `test_verdict_repeated_key.py` `a_band_file_that_repeats_a_key_is_refused_by_the_plan_without_a_traceback` and `a_well_formed_band_file_is_planned_and_its_row_count_printed` |
| A6 | a record fragment that repeats a key at its top, and one that repeats a key inside a record, are each refused by the census (exit 1) with the whole sentence and no `Traceback`; a well-formed fragment is read and its record count printed | `test_verdict_repeated_key.py` `a_record_fragment_that_repeats_a_key_at_the_top_is_refused`, `a_record_that_repeats_a_key_inside_an_entry_is_refused` and `a_well_formed_record_fragment_is_read_and_counted` |

```acceptance
A5: python3 -m unittest discover -s scripts/tests -p test_verdict_repeated_key.py -k a_band_file_that_repeats_a_key_is_refused_by_the_plan_without_a_traceback -k a_well_formed_band_file_is_planned_and_its_row_count_printed
A6: python3 -m unittest discover -s scripts/tests -p test_verdict_repeated_key.py -k a_record_fragment_that_repeats_a_key_at_the_top_is_refused -k a_record_that_repeats_a_key_inside_an_entry_is_refused -k a_well_formed_record_fragment_is_read_and_counted
```

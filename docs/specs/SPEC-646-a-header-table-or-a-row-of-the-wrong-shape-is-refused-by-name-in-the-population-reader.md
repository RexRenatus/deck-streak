# SPEC-646: a header table or a row of the wrong shape is refused by name in the population reader

- **Issue:** #951 (follow-up to #421). **Context(s):** `repo` (`scripts/mutation_rows.py`, its
  tests, this SPEC's band and `docs/`).
- **Decided by:** ADR-669 (this SPEC's own: where the shapes are checked, which sentences refuse
  them, and what each decision was chosen against), ADR-122 (the one parser), ADR-057 D5 (the
  population's shape) and SPEC-039 R8 to R11 (the rows, the runner and the retirement check).
  SPEC-465, SPEC-122, ADR-482 and ADR-122 are unchanged.
- **Mutation band:** `S64600-S64699`, rows S64600 to S64602 (section 7).
- **Status:** delivered in one pull request with its tests, its rows and its red-first record,
  `docs/red-first/SPEC-646.md` (ADR-016).

## 1. The problem, measured

Every figure below was read at the base `bd953ba7`, the source by
`git show bd953ba7:scripts/mutation_rows.py` and `grep -n`, the population by the command beside
each figure, run from the repository's root.

**Where the population is assembled.** `load_tree` (`:264`) and `load_revision` (`:375`) parse the
header, `scripts/mutation-rows.json`, and every band file under `scripts/mutation-rows.d/` through
`parse_document` (`:234`), and hand both to `assemble` (`:153`). `assemble` refuses a header that
is not an object and one whose `arities` or `tables` is not an object (`:157-161`), a band file not
named for one SPEC's band and two bands that overlap (`:162-172`). It then copies the header's
tables with no check of their shape:

- `:174` reads `    merged = {name: list(rows) for name, rows in tables.items()}`, so a header
  table that is a number or `null` raises `TypeError` there, an object is read as its keys, and a
  string as its characters.
- The header's ids are held next (`:175-181`), skipping any table that is not an array and any row
  with no string id.

For each band file it refuses a document whose one key is not `tables` (`:184-185`), a `tables`
that is not an object (`:187-188`) and a table the header does not declare (`:189-191`). It checks
that a band file's table is an array, `:192` `            if not isinstance(rows, list):`, with the
sentence `<file>: <table> is not an array of rows` (`:193`), and then reads each row for its id:
`:197` refuses a row with no string first cell as `<file>: a <table> row has no id`, which names
neither the row's place nor its shape. Nothing checks that a row is an array of strings.

**Where a row of the wrong shape ends.** `rows_of` (`:465`) measures each row by
`            if len(row) not in arities.get(table, ()):` (`:475`), so a row that is a number raises
`TypeError`, and a string or an object is measured by its characters or keys. `row_target`
(`:420`) reads a repo-rooted row's target as `posixpath.normpath(row[1])` (`:432`), so a `null`
target raises `TypeError`. A cell that is a number reaches a census or a proof that reads it as
text.

**Each shape, as the source reads it.** The tests plant each shape after one well-formed row,
in a fixture whose header declares `SCRIPT_MUTATIONS` with the arity `[6]`:

| planted shape | where | at the base | with SPEC-646 |
|---|---|---|---|
| a header table `1` or `null` | the header | `list(rows)` at `:174` raises `TypeError`: a traceback, exit 1 | R1's sentence, exit 2 |
| a header table `{}` | the header | read as an empty table: exit 0 | R1's sentence, exit 2 |
| a header table `"rows"` | the header | its characters read as rows: `None: 1 cells, where SCRIPT_MUTATIONS declares [6]`, exit 2, naming no file | R1's sentence, exit 2 |
| a row `1` after a good row | the header | `len(row)` at `:475` raises `TypeError`: a traceback, exit 1 | R2's sentence for `[1]` |
| a row `"S00081-ROW"` or `{}` after a good row | the header | `None: 10 cells, ...` or `None: 0 cells, ...`, exit 2, naming no file | R2's sentence for `[1]` |
| a row `1`, a string or `{}` after a good row | a band file | `<file>: a SCRIPT_MUTATIONS row has no id`, exit 2, naming no shape | R2's sentence for `[1]` |
| a row whose id is `7` | the header; a band file | read as a row with a number id, exit 0; `... row has no id`, exit 2 | R3's sentence for `[1]` |
| a row whose target is `null` | either | `posixpath.normpath(None)` at `:432` raises `TypeError`: a traceback, exit 1 | R3's sentence for `[1]` |
| a row whose find is `5` | either | read as a row, exit 0 (a later census or proof reads a number as text) | R3's sentence for `[1]` |
| a band file's table `{}` | a band file | `<file>: SCRIPT_MUTATIONS is not an array of rows`, exit 2 | unchanged |
| an empty row `[]` after a good row | a band file | `<file>: a SCRIPT_MUTATIONS row has no id`, exit 2 | unchanged |

**The committed population.** Every committed row already has the shape this SPEC requires:

- `python3 scripts/mutation_rows.py count --root .` reads `3323`.
- `jq -s '[.[] | .tables[][]] | length' scripts/mutation-rows.d/*.json` reads `3323` rows in the
  band files.
- `jq -s '[.[] | .tables[][] | select(type != "array" or any(.[]; type != "string"))] | length'
  scripts/mutation-rows.d/*.json` reads `0`: no committed row is other than an array of strings.
- `jq -c '[(.tables | keys), ([.tables[]] | map(length))]' scripts/mutation-rows.json` reads
  `[["CARGO_KILLED_SCRIPT_MUTATIONS","MUTATIONS","SCRIPT_MUTATIONS"],[0,0,0]]`: the header's three
  tables are empty.
- `jq -r '.tables | to_entries[] | select(.key != "MUTATIONS") | .value[] | select(.[1] ==
  "scripts/mutation_rows.py") | .[0]' scripts/mutation-rows.d/*.json | wc -l` reads `108` rows
  whose target is the reader itself; none of their finds lies in or next to the three places the
  cure inserts its lines.
- `python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py` prints
  `examined 183 committed band files`.

**Why now.** ADR-482 D6, which closed the retired list's shape (#421), left these shapes for an
issue of their own: "Two other readers whose input shapes go unchecked, the `--rows-from` plan's
and the population reader's tables and rows, are left for issues of their own: neither is the
retired list." #951 is that issue for the population reader's tables and rows.

## 2. Requirements

R1. **A header table that is not an array is refused.** When a table of the header's `tables` is
    not an array, `assemble` raises `PopulationRefused("scripts/mutation-rows.json: <table> is not
    an array of rows")`, the sentence a band file's table already gets (`:193`).
R2. **A row that is not an array is refused at its place.** A row that is not an array, in a
    table of the header or of a band file, is refused naming the file, the table and the row's
    index from 0: `<file>: <table>[<index>] is not an array of cells`. The first wrong row is the
    one refused: the header's tables in their order, then the band files in band order, each
    table's rows in order.
R3. **A row holding a cell that is not a string is refused at its place.** A row that is an array
    holding a cell that is not a string is refused the same way:
    `<file>: <table>[<index>] holds a cell that is not a string`.
R4. **Each refusal reads as every other refusal of the population.** Each is the reader's
    `PopulationRefused`, raised by `assemble` before any row is read for its id, so every verb that
    reads the population reports it as it reports the others: `count`, `ids` and `retired` print
    one `mutation_rows: REFUSED: <sentence>` line on stderr and exit 2; `prove` prints
    `prove: REFUSED: <sentence>` and exits 2 (`legs` and `merge` likewise, under their own verb);
    `census` reports its `the population does not assemble` finding. None ends in a traceback.
R5. **Nothing else moves.** A well-formed population reads as at the base; a band file's table
    that is not an array and a row with no string id (an empty array) keep their sentences; every
    committed band file reads as plain JSON reads it; the three rows join `S64600-S64699`, so
    `count` reads three more than at the base; no import is added and no existing line of
    `scripts/mutation_rows.py` or `scripts/tests/test_mutation_rows.py` changes.

## 3. Acceptance criteria

| id | criterion | kind | decided by |
|---|---|---|---|
| A1 | a header table of `1`, `{}`, `"rows"` or `null` is refused with R1's sentence, exit 2, no `Traceback`, and the reader raises the same sentence in this process (R1, R4) | red-first | `test_a_header_table_that_is_not_an_array_is_refused_naming_it` |
| A2 | a row of `1`, `"S00081-ROW"` or `{}` after a well-formed row, in the header and in a band file, is refused with R2's sentence for `[1]`, as in A1 (R2, R4) | red-first | `test_a_row_that_is_not_an_array_is_refused_naming_its_place` |
| A3 | a row whose id is `7`, whose target is `null` or whose find is `5`, after a well-formed row, in the header and in a band file, is refused with R3's sentence for `[1]`, as in A1 (R3, R4) | red-first | `test_a_row_holding_a_cell_that_is_not_a_string_is_refused_naming_its_place` |
| A4 | a band file holding one well-formed row reads `1`, exit 0, and that row's id in this process; a band file's table `{}` and an empty row after a well-formed one keep their sentences (R5) | green at base | `test_a_well_formed_population_reads_as_before_beside_the_refusals_it_kept` |
| A5 | every committed band file reads as plain JSON reads it, the new band file included (R5) | green at base | `test_every_committed_band_file_reads_as_plain_json_reads_it` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_header_table_that_is_not_an_array_is_refused_naming_it
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_row_that_is_not_an_array_is_refused_naming_its_place
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_row_holding_a_cell_that_is_not_a_string_is_refused_naming_its_place
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_well_formed_population_reads_as_before_beside_the_refusals_it_kept
A5: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k test_every_committed_band_file_reads_as_plain_json_reads_it
```

Every new test lives in `scripts/tests/test_mutation_rows.py`, in
`ThePopulationReaderRefusesAMisshapenTableOrRow`; A5 is SPEC-122's existing test, unchanged. Each
asserts a whole stderr line, an exit status and the in-process sentence by equality, never builds
an expected value with the code under test, and reads no byte of `scripts/mutation_rows.py`. A1 to
A4 print the count they examined and refuse zero: `examined 4 planted header tables`,
`examined 6 planted rows`, `examined 6 planted cells` and `examined 2 kept refusals`.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation_rows.py` | #951 | changed: the helper _refuse_misshapen_rows, the header's table and row check in assemble, and the band file's row check; insert-only |
| `scripts/tests/test_mutation_rows.py` | #951 | changed: one class of four tests, A1 to A4, inserted after the census refusal tests; insert-only |
| `scripts/mutation-rows.d/S64600-S64699.json` | #951 | added: the three rows of section 7 |
| `docs/specs/SPEC-646-a-header-table-or-a-row-of-the-wrong-shape-is-refused-by-name-in-the-population-reader.md` | #951 | added: this SPEC |
| `docs/decisions/ADR-669-the-population-reader-checks-each-table-and-row-shape-where-it-assembles-them.md` | #951 | added |
| `docs/schematics/mutation-rows-population-shape.md` | #951 | added |
| `docs/red-first/SPEC-646.md` | #951 | added |
| `changelog.d/mutation-rows-population-shape-646.md` | #951 | added |

No other file changes: no workflow, no other test module, no other band file, no formal file and
not the Python mutation legs' configuration.

## 5. What this does NOT cover

- The header's `arities` object: a header table the arities lack while it holds a row, or an arity
  that is not an array, still reaches `rows_of`'s arity check (`:475-478`) unchecked. It is not one
  of the shapes #951 names, and is left for an issue of its own.
- The `--rows-from` plan's shape (#950).
- SIGINT in the window before a killer's group can end (#952).
- `scripts/mutation-verdict.py`, which reads the population through the same two loaders and
  handles a refusal of it as it did before; #951 names only the reader's tables and rows.
- The retired list's reader and its Lean entry, `lean/RetiredListShape`, unchanged (#421).

## 6. Risks

- **A newly strict refusal.** A header table `{}` or `""` used to read as an empty table and is now
  refused. No committed file holds one: section 1's `jq` reads find the header's three tables empty
  arrays and no committed row of another shape.
- **A future table whose cells are not all text.** A table with a number column by design would
  need R3 to read its layout, as `rows_of` does; ADR-669's "What would make this wrong" names it.
- **The process tests' cost.** They build one git fixture per planted shape, nineteen in all (four
  for A1, six each for A2 and A3, and three for A4), each bounded by the fixture's own timeout.

## 7. Formal, rows and mutation

**Formal: NOT REQUIRED by surface.** The cure adds three refusals ahead of the reader's existing
decisions and leaves every population of arrays of strings to read as at the base. `assemble` and
`rows_of` carry no formal entry at the base, and `retirement_records`, the item of
`scripts/mutation_rows.py` that `lean/RetiredListShape` covers, is byte-identical, so that entry's
stamp holds (ADR-669 D4).

**Rows.** Each row's mutant is `        if False:` in place of its check's line, and each is killed by
its criterion's test, proved KILLED by `python3 scripts/mutation_rows.py prove --band
S64600-S64699`:

| row | requirement | the check it removes | killer |
|---|---|---|---|
| S64600 | R1 | `if not isinstance(header_rows, list):` | `test_a_header_table_that_is_not_an_array_is_refused_naming_it` |
| S64601 | R2 | `if not isinstance(cells, list):` | `test_a_row_that_is_not_an_array_is_refused_naming_its_place` |
| S64602 | R3 | `if not all(isinstance(cell, str) for cell in cells):` | `test_a_row_holding_a_cell_that_is_not_a_string_is_refused_naming_its_place` |

**Mutation.** The generated mutants of the new lines are the three `not` removals, each of which
refuses every well-formed table or row, so every test that reads a fixture kills them; the
`mutation-python` legs run them, and `scripts/mutation-python.json` is unchanged.

## 8. Amendments

None.

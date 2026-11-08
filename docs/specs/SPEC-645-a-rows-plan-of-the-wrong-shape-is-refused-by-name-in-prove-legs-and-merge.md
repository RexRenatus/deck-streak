# SPEC-645: a rows plan of the wrong shape is refused by name in prove, legs and merge

- **Issue:** #950 (the follow-up #421's design filed). **Context(s):** `repo`
  (`scripts/mutation_rows.py`, its tests, this SPEC's band and `docs/`).
- **Decided by:** ADR-668 (this SPEC's own: where the plan's shape is checked, how a wrong one is
  refused, and what each was chosen against), and ADR-482 (the retired list's reader, whose shape
  and words this follows, D2, D5 and D6), SPEC-039 (the rows, the runner) and SPEC-417 (`legs` and
  `merge`).
- **Mutation band:** `S64500-S64599`, claimed, holding nine rows.
- **Status:** delivered in one pull request with its tests, its rows and its red-first record,
  `docs/red-first/SPEC-645.md`.

## 1. The problem, measured

Measured at the base `bd953ba7`, by `git show <base>:<path>` and `grep -n`; every `:line` below is
in `scripts/mutation_rows.py` at that base.

`chosen` (`:1194`) gathers the rows the selectors name. For `--rows-from` it parses the plan,
`plan = json.loads(pathlib.Path(args.rows_from).read_text(encoding="utf-8"))` (`:1207`), and adds
`plan.get("rows", [])` (`:1208`) with no check of either shape. It then looks each id up
(`missing = [identifier for identifier in wanted if identifier not in by_id]`, `:1210`) and refuses
the ids no row holds by joining them into `no row holds the id(s) ...` (`:1212`).

Three verbs read the plan through `chosen`: `prove` at `:1223`, `legs` at `:1304` and `merge` at
`:1335`, each `rows = chosen(rows_of(load_tree(root)), args)` inside a `try` whose `except` is
`(OSError, json.JSONDecodeError, PopulationRefused, UnresolvableTarget)` (`:1224`, `:1305` and
`:1336`); `grep -n -F 'chosen('` at the base reads 1194, 1223, 1304 and 1335. Each prints such a
refusal as `<verb>: REFUSED: ...` on stdout and returns 2. `dispatch` calls `prove` inside a `try`
that catches only `ToolMissing` (`:1576-1579`), and calls `legs` (`:1583`) and `merge` (`:1587`)
with no `try` at all. #950 names `prove` and `legs`; `merge` is cured beside them because it reads
the same plan through the same `chosen` and crashes the same way.

| the plan | at the base |
|---|---|
| an array, a string, a number, a boolean or null | `AttributeError` at `:1208`, traceback, exit 1 |
| an object whose `rows` is a number or null | `TypeError` at `:1208`, traceback, exit 1 |
| an object whose `rows` is a string | its characters read as row ids and refused as `no row holds the id(s) S, 0, ...` |
| an object whose `rows` is an object | its keys read as row ids, silently |
| a `rows` entry that is a number | `TypeError` at the join of `:1212`, traceback, exit 1 |
| a `rows` entry that is an array or an object | `TypeError` at the lookup of `:1210`, traceback, exit 1 |

The retired list's reader, `retirement_records` (`:1396-1423`), already refuses its own wrong
shapes by sentences naming the file and the shape it needed, and `dispatch` prints such a refusal
as one `mutation_rows: REFUSED: <sentence>` line on stderr with exit 2 (`:1592-1594`), as it does
for `count` and `ids` (`:1554-1556`). ADR-482 D6 named the `--rows-from` plan's reader as left for
an issue of its own; this SPEC is that issue's cure.

## 2. Requirements

- **R1.** A `--rows-from` plan whose text is JSON but not an object (an array, a string, a
  number, `true`, `false` or `null`) is refused with the sentence
  `<plan> must hold a top-level object`, `<plan>` being the path as `--rows-from` gave it.
- **R2.** A plan object whose `rows` key is present and not an array is refused with
  ``<plan>: `rows` is not an array of row ids``; an absent `rows` still selects no row from the
  plan.
- **R3.** An entry of `rows` that is not a string is refused with
  `<plan>: rows[<i>] is not a string`, `<i>` the first such entry's index from 0.
- **R4.** `prove`, `legs` and `merge` each print an R1 to R3 refusal as exactly one line
  `mutation_rows: REFUSED: <sentence>` on stderr, print nothing on stdout, exit 2 with no
  traceback, prove no row, and write no report (`merge`) and no step outputs (`legs`).
- **R5.** A well-formed plan reads as before: any other key is ignored, its row ids are added to
  the other selectors' as before, and an id no row holds is refused as before
  (`<verb>: REFUSED: no row holds the id(s) ...` on stdout, exit 2). A plan that cannot be read or
  is not JSON keeps its present refusal (`<verb>: REFUSED: ...` on stdout, exit 2), and
  `prove --all` reads no plan, as before.

## 3. Acceptance criteria of SPEC-645

| id | criterion | kind | decided by |
|---|---|---|---|
| A1 | the plan `["S00001-FIRST"]` is refused with R1's sentence by `prove`, `legs` and `merge`, each one stderr line, exit 2, empty stdout, no report and no outputs, and the reader raises the same sentence in this process (R1, R4) | red-first | `test_a_plan_that_is_an_array_is_refused_naming_the_shape` |
| A2 | the plan `"S00001-FIRST"` is refused as in A1 (R1, R4) | red-first | `test_a_plan_that_is_a_string_is_refused_naming_the_shape` |
| A3 | the plans `7`, `1.5`, `true`, `false` and `null` are each refused as in A1 (R1, R4) | red-first | `test_a_plan_that_is_a_number_a_boolean_or_null_is_refused_naming_the_shape` |
| A4 | a `rows` value of `7`, `"S00001-FIRST"`, `null` and `{"S00001-FIRST": 1}` is each refused with R2's sentence, as in A1 (R2, R4) | red-first | `test_a_rows_value_that_is_not_an_array_is_refused_naming_the_shape` |
| A5 | `rows` as `["S00001-FIRST", 1]` is refused with R3's sentence for index 1, and `[["S00001-FIRST"]]` for index 0, as in A1 (R3, R4) | red-first | `test_a_row_id_that_is_not_a_string_is_refused_naming_its_place` |
| A6 | a plan of `_`, the three ids and a `scope` object reads 3 rows, a plan of `_` alone reads 0 rows, and `{"rows": ["S99999-GHOST"]}` is refused as before (R5) | not red | `test_a_well_formed_plan_still_selects_its_rows` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_plan_that_is_an_array_is_refused_naming_the_shape
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_plan_that_is_a_string_is_refused_naming_the_shape
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_plan_that_is_a_number_a_boolean_or_null_is_refused_naming_the_shape
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_rows_value_that_is_not_an_array_is_refused_naming_the_shape
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_row_id_that_is_not_a_string_is_refused_naming_its_place
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_well_formed_plan_still_selects_its_rows
```

Every new test lives in `TheRowsPlanRefusesAWrongShape` in `scripts/tests/test_mutation_rows.py`.
A1 to A5 run the three verbs as processes over a fixture repository and assert, for each, the
whole stderr line, the exit status and an empty stdout, then that no report and no step output was
written, then call the reader in this process and assert the sentence it raises; each expected
sentence is spelt in the test from the plan's path, never built by the code under test. The loops
of A3 to A6 print the count they examined and refuse zero. A6 runs `legs` as a process only, so it
reads green at the base, where the reader does not exist.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation_rows.py` | #950 | changed: `PlanRefused`, the reader `plan_rows`, `chosen`'s plan read through it, and `dispatch`'s three `PlanRefused` arms (R1 to R5) |
| `scripts/tests/test_mutation_rows.py` | #950 | changed, insert-only: three docstring lines, the sentence constants and `TheRowsPlanRefusesAWrongShape` (section 3) |
| `scripts/mutation-rows.d/S64500-S64599.json` | #950 | added: the nine rows of section 7 |
| `docs/specs/SPEC-645-a-rows-plan-of-the-wrong-shape-is-refused-by-name-in-prove-legs-and-merge.md` | #950 | added: this SPEC |
| `docs/decisions/ADR-668-the-rows-plan-is-shape-checked-in-its-own-reader-and-a-wrong-one-is-refused-on-stderr.md` | #950 | added |
| `docs/schematics/mutation-rows-plan-read.md` | #950 | added |
| `docs/red-first/SPEC-645.md` | #950 | added |
| `changelog.d/mutation-rows-plan-shape-645.md` | #950 | added |

## 5. What this does NOT cover

- A plan that cannot be read or is not JSON keeps the verb's own refusal on stdout, R5 (#950).
- `prove --all` reads no plan, so it is unchanged (#950).
- The population reader's own unchecked shapes, a header table or a row of the wrong shape, are
  another issue's; nothing here changes `assemble` or `rows_of` (#421).
- No workflow changes: the `mutation-plan` and rows jobs run the same commands, and a wrongly
  shaped plan now fails them with exit 2 where it failed with exit 1 (#950).
- The Python mutation legs' price for this file grows by the new tests' cost; refreshing a price
  row whose suite grew is the stale-price reading's subject, the open pull request #873.

## 6. Risks

- **A refusal is newly strict.** A `rows` object or string that read as its keys or its
  characters is now refused. The plan writer emits only an array of strings, so no committed plan
  reads differently; a refusal names the file and the shape, so a writer at fault is found by name.
- **The exit status moves.** A wrong shape that crashed with exit 1 now refuses with exit 2, which
  every job that runs these verbs already reads as a refusal.
- **The census modules read the runner.** `test_mutation_rows_missing_tool.py` admits only the
  runner's current imports and its spawn helpers; the cure adds no import and no spawn, and the
  module reads green at the cure.

## 7. Formal, rows and mutation

**Formal:** not required, ADR-668 D4. The reader decides no verdict, and the span of
`retirement_records` that `lean/RetiredListShape` covers is byte-identical after the cure.

**Rows:** all in `scripts/mutation-rows.d/S64500-S64599.json`, `SCRIPT_MUTATIONS`, target
`scripts/mutation_rows.py`, each killer a `test_mutation_rows.TheRowsPlanRefusesAWrongShape.<test>`
that selects one test.

| row | mutant | killer |
|---|---|---|
| S64500 | R1's check removed | A1 |
| S64501 | R2's check removed | A4 |
| S64502 | R3's check removed | A5 |
| S64503 | the selection reads `plan.get("rows", [])` again, past the reader | A1 |
| S64504 | the reader reads another key than `rows` | A6 |
| S64505 | the reader returns nothing in place of the plan's row ids | A6 |
| S64506 | `prove`'s refusal arm returns exit 0 | A1 |
| S64507 | `legs`' refusal arm returns exit 0 | A1 |
| S64508 | `merge`'s refusal arm returns exit 0 | A1 |

**Mutation:** the pull request's own Python mutation legs judge the diff's mutants of
`scripts/mutation_rows.py`, and the rows pin each check, the reader's call and its return, and each
verb's exit.

## 8. References

Issues #950 and #421. ADR-668, ADR-482. SPEC-465, SPEC-039, SPEC-417.

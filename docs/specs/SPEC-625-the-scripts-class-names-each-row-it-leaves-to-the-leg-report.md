# SPEC-625: the scripts class names each row it leaves to the leg report

- **Issue:** none; refers SPEC-087 R11 (the rows the diff selects on a class's changed lines) and
  SPEC-039 R10 (every selected row is proved KILLED). **Context(s):** the mutation scripts
  (`scripts/`); no workflow, no map and no crate changes.
- **Decided by:** ADR-647 (a row no class credits is named by the scripts class and never counted).
- **Schematic:** `docs/schematics/mutation-rows-left-to-the-leg-report.md`.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-625.md`. **Mutation band:** S62500-S62599. **Models:** none; the eleven
  definitions of `scripts/mutation-verdict.py` that the Lean and TLA+ entries cover by digest are
  re-read and kept byte for byte.

## 1. The problem, measured

- **A killed row the verdict names nowhere.** In job 124459129894, the `mutation-verdict` job of a
  pull request run, the scripts class printed `mutation: scripts: examined 71: generated 59, rows
  12` and `mutation: scripts: verdict: ok`, while the `mutation-rows` leg's artifact held 13 rows,
  all KILLED (`gh run view --job 124459129894 --log`). The 13th,
  `S42308-THE-MAP-JUDGES-THE-VERDICT-SCRIPT-WITH-THE-RUST-TESTS`, targets
  `scripts/mutation-python.json` (`grep -n S42308 scripts/mutation-rows.d/S42300-S42399.json`:
  line 69), and only the leg report showed it.
- **Why.** `judge_rows` in `scripts/mutation-verdict.py` checks that every selected row is KILLED,
  but credits a row to the judged class only when `classify(plan["row_overlaps"][id])` equals that
  class. `classify` places a path in `scripts` only by `SCRIPTS = re.compile(r"scripts/[^/]+\.py")`,
  so a JSON target reads `other`; `klass_rows` credits rows only in `rust`, `oracle` and `scripts`,
  since the web job never proves rows; and a selected row whose anchor covers no changed line has
  no overlap at all. Such a row is judged KILLED, and is counted and named by no class
  (`grep -n -E '^def (judge_rows|classify|klass_rows)' scripts/mutation-verdict.py`).
- **What it costs.** A reader of the verdict cannot tell, from the verdict alone, that the rows
  leg proved a row the class lines never mention; the only record is the leg report.

## 2. Requirements

- **R1.** After `judge_rows` judges the selected rows, the scripts class, and only it, collects the
  rows left to the leg report: each id of `plan["rows"]`, in that order, whose rows-report entry
  reads `KILLED` and whose `plan["row_overlaps"]` path is absent or classifies outside `rust`,
  `oracle` and `scripts`, the classes whose verdict credits rows (`klass_rows`).
- **R2.** When at least one row is left, the scripts class says, through `Verdict.say` (so each
  line begins `mutation: scripts: `), first `rows left to the leg report: <n> of <s> selected`
  (`<s>` is `len(plan["rows"])`), then one line per left row: `row left to the leg report: <id>:
  its changed line is in <path>` when it has an overlap path, else `row left to the leg report:
  <id>: it covers no changed line`. When none is left it says nothing new.
- **R3.** No count, verdict, FAIL or VOID changes: `judge_rows` returns the same carried count for
  every input, a left row adds to no class's `examined`, a row the leg did not kill still FAILs or
  VOIDs by name as before and is never named left, and a scripts class that applies and examined
  nothing still reads `VOID the scripts class applies and nothing was examined`.
- **R4.** The lines print whether or not the scripts class applies (`judge_python` calls
  `judge_rows` before its applies test, as before), and never in the rust or oracle class.
- **R5.** The eleven definitions the formal entries cover keep their source text: `python_excuses`,
  `Excuses` and `judge_python` (`formal/lean/Formal/RecordClassBinding.lean`), `rust_tokens` and
  `block_end` (`formal/lean/Formal/TokenProgress.lean`), and `shards`, `whole_reports`,
  `partition`, `examined_sum`, `population_gaps` and `python_by_cost`
  (`formal/tla/EveryLegCounted/EveryLegCounted.tla`).

## 3. Acceptance criteria of SPEC-625

Each test is in the class `TheScriptsClassNamesTheRowsItLeavesToTheLegReport` of
`scripts/tests/test_mutation_python_verdict.py`, over the module's own `Fixture`.

| criterion | holds | decided by |
|---|---|---|
| A1 | a KILLED row on the guard script's changed line and one on a changed line of `scripts/mutation-python.json`: the scripts class exits 0, holds `examined 1: generated 0, rows 1`, exactly the two lines `rows left to the leg report: 1 of 2 selected` and `row left to the leg report: <the map row>: its changed line is in scripts/mutation-python.json`, and names the guard row on no such line (R1, R2, R3) | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_killed_row_on_a_changed_map_line_is_named_left_to_the_leg_report` |
| A2 | KILLED rows on a script, the oracle's generator, a Rust source, the map, and one that covers no changed line: the scripts class names exactly the map row and the no-line row, in plan order, under `rows left to the leg report: 2 of 5 selected`, and the oracle and the rust class over the same plan and report hold no such text (R1, R2, R4) | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_only_the_scripts_class_names_and_only_rows_no_class_credits` |
| A3 | every selected row on a changed script line, all KILLED: no `left to the leg report` text (R2) | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_no_left_line_prints_when_every_selected_row_is_credited` |
| A4 | the map row SURVIVED: the scripts class exits 1 with `<id>: SURVIVED: <reason>` and no `left to the leg report` text (R1, R3) | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_row_the_leg_did_not_kill_fails_and_is_never_named_left` |
| A5 | a changed script code line, a Python report with no mutant, and only the KILLED map row: the scripts class reads `VOID the scripts class applies and nothing was examined` and `examined 0` (R3) | `python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_left_row_adds_nothing_to_the_examined_count` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_killed_row_on_a_changed_map_line_is_named_left_to_the_leg_report
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_only_the_scripts_class_names_and_only_rows_no_class_credits
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_no_left_line_prints_when_every_selected_row_is_credited
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_row_the_leg_did_not_kill_fails_and_is_never_named_left
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_left_row_adds_nothing_to_the_examined_count
```

A1 and A2 are red at the base, which names no left row. A3, A4 and A5 guard what the base already
holds: no such line when every row is credited, a FAIL by name for a row the leg did not kill, and
the zero-examined VOID.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-625-the-scripts-class-names-each-row-it-leaves-to-the-leg-report.md` | records | added: this file |
| `docs/decisions/ADR-647-a-row-no-class-credits-is-named-by-the-scripts-class-and-never-counted.md` | records | added: D1 |
| `docs/schematics/mutation-rows-left-to-the-leg-report.md` | records | added: one flowchart |
| `changelog.d/rows-left-625.md` | records | added: the fragment |
| `docs/red-first/SPEC-625.md` | records | added: the reds and greens of A1 to A5 |
| `scripts/mutation-verdict.py` | mutation scripts | changed: one call after `judge_rows`' loop, and a new constant and function directly after it (R1, R2) |
| `scripts/tests/test_mutation_python_verdict.py` | tests | changed: the class of A1 to A5 |
| `scripts/mutation-rows.d/S62500-S62599.json` | mutation rows | added: S62500-S62506 |

## 5. What this does NOT cover

- It credits no left row to any class's count: a row on a path of no judged class stays proved by
  the rows leg alone, as the Python mutant campaign tracks (#322).
- It does not classify the Python mutation map, or any other JSON file, as a script (#322).
- It adds no row-proving step to the web job, which proves no rows (#322).
- It changes no workflow, no map entry and no job's bound (#322).

## 6. Risks

- **A long list on a wide diff.** A pull request that changes `scripts/mutation-verdict.py`
  selects every row that targets it, and most of them cover no changed line, so the scripts class
  may name nearly all of them. The count line states `<n> of <s> selected`, so the length is read
  from one line.
- **A new line equal to a committed anchor.** A line this SPEC adds that equals a short committed
  `find` would make that row's anchor ambiguous; every committed row on the file is checked to
  find exactly once before the cure is committed.
- **A port that drifts.** The formal entries cover eleven definitions of the file by digest; the
  cure lives in `judge_rows` and new code after it, and the eleven are compared byte for byte.

## 7. What only CI proves

- The seven rows of section 9 read KILLED, by id, in the `mutation-rows` job of the second push.
- The generated mutants of the added lines are judged by the Python legs of that run, and the
  `mutation-verdict` job's log holds the scripts class's left lines for the rows that run selects.

## 8. Formal model

None. One reporting function and its call print lines and change no count, verdict or state
machine. The eleven definitions of R5 keep their source text, so every Lean and TLA+ entry that
covers `scripts/mutation-verdict.py` stays valid.

## 9. Mutation rows

| row | behaviour | mutant | killer |
|---|---|---|---|
| S62500 | only the scripts class names left rows | the scripts-only gate inverted | A2 |
| S62501 | an oracle row is credited, never left | `oracle` dropped from the crediting classes | A2 |
| S62502 | a Rust row is credited, never left | `rust` dropped from the crediting classes | A2 |
| S62503 | only a KILLED row is named left | the KILLED test dropped | A4 |
| S62504 | no line prints when none is left | the none-left guard made always true | A3 |
| S62505 | each left row carries its own reason | the overlap test inverted, so the two reasons swap | A1 |
| S62506 | a left row adds to no count | a row of no crediting class added to the class count | A5 |

# SPEC-421: each Python file is priced at its scoped test cost, measured by a dispatched run, and the plan deals scoped legs by it

- **Issue:** #837, the Python mutation legs priced at what a scoped leg runs. This SPEC refers to
  it and closes nothing. It also refers to #825, the Python legs' fit, and #822, the release.
- **Context(s):** the mutation scripts (`scripts/`), their tests and fixtures, one new
  dispatch-only workflow, and the release runbook. `ci.yml` and `mutation-weekly.yml` are
  unchanged, and no crate changes.
- **Decided by:** ADR-435 (the Python plan prices scoped legs from a dispatched price run, with
  each figure naming its run and job). SPEC-418 and ADR-434 are amended insert-only.
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 8, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests, the price run's
  committed records and `docs/red-first/SPEC-421.md`.
- **Mutation band:** S42100-S42199.
- **Models:** `formal/tla/EveryLegCounted`, re-read and re-stamped; `formal/tla/TracedSetCovers`,
  unchanged.
- **Builds on:** SPEC-420 and SPEC-418, whose scoped test sets and cost deal it keeps.

## 1. The problem, measured

Every `path:line` below was read at dev `c726af67`.

- The plan prices every Python mutant at its file's whole-suite control. `PYTHON_CONTROL_SECONDS`
  (`scripts/mutation-verdict.py:135-143`) holds seven whole-suite prices from battery run
  38242806596, whose Python legs each ran to the job's 120-minute bound; `scripts/mutation-verdict.py`
  is priced there at 224 s. `python_by_cost` (:1071-1135) holds `k = PYTHON_LEG_BOUND_SECONDS //
  price - 2` mutants a leg, so that file deals six mutants a leg.
- A pull request's leg runs each mutant against its scoped test set only (SPEC-420), and
  `scripts/mutation_python.py:1138` takes the trace only in `run --plan`. The plan never reads
  what a scoped mutant costs.
- #836's run 38333401795 ran five priced legs of `scripts/mutation-verdict.py` in 456.07 s to 884.44 s,
  read from the run's job list:

| job | runner-step duration | what the leg paid |
|---|---|---|
| 115088058737 | 456.07 s | control and traced sentinel, about 224 s each, then its scoped mutants |
| 115088058742 | 879.88 s | the same |
| 115088058864 | 884.44 s | the same |
| 115088058658 | 713.44 s | the same |
| 115088058757 | 803.78 s | the same |

- Over those legs a scoped mutant took about 60 s (58 to 67 s; job 115088058737 and the four job ids above), against the 224 s the plan
  charged it.
- The plan's deal (`scripts/mutation-verdict.py shards`) over the release's 6186 listed Python
  mutants reads 447 priced legs and 139 unpriced legs: 586, against the share of 32.
  `scripts/mutation-verdict.py` alone takes 439 of the 447 priced legs.
- Nine listed files carry no price at all: `scripts/mutation_python.py`,
  `scripts/mutation_rows.py`, `scripts/public-scrub.py`, `scripts/testflight_age.py`,
  `scripts/threat_model.py`, `scripts/web-engine-size.py`, `scripts/web_mutation_shards.py`,
  `scripts/write-back-issues.py` and `tools/parity-oracle/generate.py`.
- No run measures a scoped price. The weekly battery runs `run --all`, whole and in path order, and
  a pull request's legs list only its diff.

## 2. Requirements

R1. `scripts/mutation_python.py run` gains `--scoped`, accepted with `--file` alone. A
`--file --scoped` run traces each file's sentinel and runs each mutant against its scoped set,
with every fallback, exactly as `run --plan` does.

R2. `run` gains `--sample S`, accepted with `--file --scoped` alone, where S is a whole number of 1
or more. Over m selected mutants, it runs the mutants at positions 0, t, 2t and so on, in listing
order, where t = ceil(m / S). A run without `--sample` runs every selected mutant.

R3. `run` refuses each of these by name before any mutant runs, with a line naming the flags and a
non-zero exit: `--scoped` beside `--plan`; `--scoped` beside `--all`; `--sample` beside `--plan`;
`--sample` beside `--all`; `--sample` without `--scoped`. No verdict run samples, and the battery
stays whole.

R4. `scripts/mutation_python.py` holds `SCOPED_SELECTIONS = frozenset({"plan"})` on a line of its
own, and `selection(args)`, which returns `plan`, `all` or `file` as the report's selection does.
`prepare` traces when `selection(args) in SCOPED_SELECTIONS or args.scoped`. `planned_leg`,
`child_trace` and `scoped_sets` are unchanged.

R5. A traced sentinel appends one record to `costs.jsonl`:
`{"kind": "sentinel", "path": <path>, "seconds": <s>, **where}`. A whole run writes no sentinel
record. `where` gains `"job": os.environ.get("MUTATION_JOB_ID")`, so every control, uncovered and
sentinel record names its job. The mutant record is unchanged.

R6. `scripts/mutation-verdict.py` holds `PYTHON_SCOPED_SECONDS: dict[str, tuple[int, int, int]]`
after `PYTHON_CONTROL_SECONDS`. Each line reads
`    "<path>": (<control>, <sentinel>, <mutant>),  # run <r> attempt <a> job <j>: <n> scoped mutant record(s)`.
`PYTHON_CONTROL_SECONDS` stays byte-identical.

R7. `mutation-verdict.py prices --scoped --costs DIR` reads every `costs.jsonl` under DIR and
prints one R6 line per file of `scripts/mutation-python.json` that its records price, in path order;
a record for any other path is examined and prints no line. The control figure is max(1,
ceil(the largest control seconds)); the sentinel figure is max(1, ceil(the largest sentinel
seconds)); the mutant figure is max(1, ceil(the mean mutant seconds)). A file with an `uncovered`
record reads (1, 1, 1). Then it prints `mutation: prices: <path> unpriced: <reason>` for each file
of `scripts/mutation-python.json` it cannot price, and last `examined N record(s) in F file(s);
torn T`. A line that is not a record of a known kind is torn, counted and never read.

R8. The verb reads VOID, exits `EXIT_FAIL` and prints no R6 line when: it finds no record; a
control, uncovered or sentinel record names no run; such a record names no job; a mutant record's
`why` is `all`; one path's records name two jobs; one path's records name two attempts.

R9. `python_price(path, prices, scoped)` returns `scoped[path]` when `scoped` is given and holds the
path, else `(p, p, p)` when `prices` holds `p`, else None. `python_by_cost(listing, prices,
scoped=None)` deals by setup = control + sentinel and k = (1800 - setup) // each, refuses at k < 1
with SPEC-418 R3 (b)'s line reading the control and setup + each, and sorts runs by
-(setup + len x each). `python_deal` loads a leg by size x each, plus setup when the leg does not
hold the file. `python_unpriced_legs` takes the same `scoped=None`. A call without `scoped` is
byte-identical to the base's, and the partition and the count are unchanged.

R10. `shards` passes `PYTHON_SCOPED_SECONDS` when `"plan"` is in
`mutation_python.SCOPED_SELECTIONS`, and None otherwise. When at least one file is priced scoped,
it prints `mutation: shards: python legs priced scoped: <s> file(s) at their scoped price, <w> at
their whole-suite price, <u> unpriced` after SPEC-418's `python legs dealt by cost` line. Otherwise
nothing new prints.

R11. `.github/workflows/mutation-prices.yml` is new. It triggers on `workflow_dispatch`, and on
`pull_request` into `dev` with `paths` naming itself. Its permissions are read-only, its actions are
pinned as `ci.yml` pins them, and its jobs run on the runner class of `ci.yml`'s `mutation-python`
job. A population job emits the files of `scripts/mutation-python.json` on a dispatch, and one
named population file on the pull request event. A matrix job runs, per file,
`python3 scripts/mutation_python.py run --file "$FILE" --scoped --sample 20 --failfast --report
"$out/report.json"` with `MUTATION_JOB_ID: ${{ job.check_run_id }}` and `timeout-minutes: 120`,
and uploads its report directory as `mutation-prices-<job-index>` under `if: always()`. It files
and judges nothing.

R12. The price run's records are committed as `scripts/tests/fixtures/mutation-scoped-prices-<run>/`,
in the download's layout, `costs.jsonl` files only, from the legs whose artifact holds
`report.json`, in a commit before the table's. `PYTHON_SCOPED_SECONDS` is the R7 verb's output
over that fixture, and a test re-derives it byte for byte. A refresh replaces the fixture and the
table together, in one delivery.

R13. `RELEASING.md` section 2 gains one paragraph after SPEC-420's, naming
`PYTHON_SCOPED_SECONDS`, `prices --scoped` and `mutation-prices.yml`, and how a refresh is made.

R14. Unchanged: the share `PYTHON_MAX_SHARDS` 32, `PYTHON_SHARE_CAP` 128, `RUN_JOBS` 256,
`LEG_CEILING`, the Rust ceiling 190, `PYTHON_LEG_BOUND_SECONDS` 1800, `ci.yml`,
`mutation-weekly.yml` and every refusal's text. No mutant leaves the plan, and no skipped-mutant
record exists.

R15. Insert-only amendment sections, every earlier byte kept: SPEC-418 (R3 (b), R3 (d) and R4 read
the triple) and ADR-434 (its rejected second, scoped figure is decided by ADR-435).
`docs/schematics/mutation-testing-python.md` gains section 8.

R16. `formal/tla/EveryLegCounted` is re-read against its `shards` and `python_by_cost` spans
(section 8). Rows S42100-S42199 in `scripts/mutation-rows.d/S42100-S42199.json` (section 9).

## 3. Acceptance criteria of SPEC-421

| id | criterion | decided by |
|---|---|---|
| A1 | a `--file --scoped` run over the scope fixture runs each mutant against the tests its line's trace names, and writes exactly one sentinel record per file | `test_mutation_python.py` `test_a_scoped_file_run_scopes_each_mutant_and_records_its_sentinel` |
| A2 | each of R3's five combinations is refused by a line naming its flags, before any mutant runs | `test_mutation_python.py` `test_scoped_is_refused_beside_a_verdict_run` and `test_sample_is_refused_outside_a_scoped_file_run` |
| A3 | `--sample 3` over 14 selected mutants runs the mutants at positions 0, 5 and 10, in that order | `test_mutation_python.py` `test_a_sample_runs_every_ceil_m_over_s_th_mutant_in_order` |
| A4 | with `MUTATION_JOB_ID` set, the control, uncovered and sentinel records name that job; unset, they name `None` | `test_mutation_python.py` `test_each_setup_record_names_its_job` |
| A5 | `run --all` and `run --file` without `--scoped` write no sentinel record | `test_mutation_python.py` `test_a_whole_run_writes_no_sentinel_record` |
| A6 | `shards` deals by the scoped table exactly when `"plan"` is in `SCOPED_SELECTIONS` | `test_mutation_python_shard_binding.py` `test_the_plan_prices_scoped_exactly_when_the_legs_trace` |
| A7 | over a fixture of known records, the verb prints each file's triple from the largest control, the largest sentinel and the mean mutant, rounded up, then the unpriced and examined lines | `test_mutation_python_shard_binding.py` `test_the_scoped_prices_verb_prints_one_triple_per_file` |
| A8 | each of R8's six conditions reads VOID with no table line, one test per condition | `test_mutation_python_shard_binding.py`, the six tests named in the fence |
| A9 | `PYTHON_SCOPED_SECONDS` is byte for byte the verb's output over the committed fixture, and every line names its run, attempt and job | `test_mutation_python_shard_binding.py` `test_the_scoped_table_is_the_verbs_output_over_the_recorded_price_run` |
| A10 | a triple deals a file's mutants by its setup and its mutant figure, (224, 224, 60) holding 22 a leg | `test_mutation_python_shard_binding.py` `test_a_triple_deals_by_its_setup_and_its_mutant_figure` |
| A11 | a file is priced by its scoped triple, else by its whole price thrice, else by the count rule | `test_mutation_python_shard_binding.py` `test_a_file_is_priced_scoped_then_whole_then_by_the_count_rule` |
| A12 | with an empty or absent scoped table, every whole-priced deal and its printed lines are byte-identical to the two-argument call's | `test_mutation_python_shard_binding.py` `test_an_empty_or_absent_scoped_table_leaves_every_whole_deal_byte_identical` |
| A13 | under mixed scoped, whole and unpriced files, every listed mutant is in exactly one leg and the leg count equals the matrix count | `test_mutation_python_shard_binding.py` `test_mixed_prices_keep_the_partition_and_the_count` |
| A14 | the scoped line prints with its three counts when a file is priced scoped, and is absent when none is | `test_mutation_python_shard_binding.py` `test_the_plan_prints_how_many_files_it_priced_scoped` |
| A15 | section 2 of `RELEASING.md` names `PYTHON_SCOPED_SECONDS`, `prices --scoped` and `mutation-prices.yml` | `test_rulesets.py` `test_the_runbook_names_the_scoped_price_table_its_verb_and_its_workflow` |
| A16 | the price workflow's matrix job runs on `ci.yml`'s `mutation-python` runner class with `--scoped --sample 20 --failfast`, takes `MUTATION_JOB_ID` from `job.check_run_id`, uploads under `if: always()`, and holds no write permission | `test_ci_workflows.py` `test_the_price_workflow_measures_scoped_on_the_python_legs_runner_class` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_scoped_file_run_scopes_each_mutant_and_records_its_sentinel
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_scoped_is_refused_beside_a_verdict_run
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_sample_is_refused_outside_a_scoped_file_run
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_sample_runs_every_ceil_m_over_s_th_mutant_in_order
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_setup_record_names_its_job
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_whole_run_writes_no_sentinel_record
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_prices_scoped_exactly_when_the_legs_trace
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_prints_one_triple_per_file
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_over_no_record
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_a_record_with_no_run
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_a_record_with_no_job
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_a_whole_run_mutant_record
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_one_path_across_two_jobs
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_prices_verb_is_void_on_one_path_across_two_attempts
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_scoped_table_is_the_verbs_output_over_the_recorded_price_run
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_triple_deals_by_its_setup_and_its_mutant_figure
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_file_is_priced_scoped_then_whole_then_by_the_count_rule
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_an_empty_or_absent_scoped_table_leaves_every_whole_deal_byte_identical
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_mixed_prices_keep_the_partition_and_the_count
A14: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_prints_how_many_files_it_priced_scoped
A15: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_scoped_price_table_its_verb_and_its_workflow
A16: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_price_workflow_measures_scoped_on_the_python_legs_runner_class
```

A5, A12, A13 and A16 are not red first. The base writes no sentinel record (A5); the red commit's
stubs accept and ignore the scoped table, so the whole deal, the partition and the count already
hold (A12, A13); and the workflow is a new file, whose absence is not its behaviour (A16).
`docs/red-first/SPEC-421.md` records each as `not red`, with that reason. A9 is red at the fixture
commit, on the assertion that the table is not the verb's output. Every other criterion is red at
the red commit, on an assertion of its behaviour.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation_python.py` | mutation scripts | changed: `--scoped` and `--sample` with their refusals; `SCOPED_SELECTIONS` and `selection()`; `prepare`'s trace condition; the sentinel record; `where`'s `job` |
| `scripts/mutation-verdict.py` | mutation scripts | changed: `PYTHON_SCOPED_SECONDS` after the whole table; `python_price()`; `python_by_cost`, `python_deal` and `python_unpriced_legs` read a triple; `shards` passes the scoped table and prints the scoped line; `prices --scoped` |
| `.github/workflows/mutation-prices.yml` | CI | added: the dispatch-only price workflow and its pull request rehearsal |
| `scripts/tests/test_mutation_python.py` | mutation tests | changed: A1-A5; the control record's pinned dict gains `"job": None` |
| `scripts/tests/test_mutation_python_shard_binding.py` | mutation tests | changed: A6-A14 |
| `scripts/tests/test_rulesets.py` | runbook tests | changed: A15 |
| `scripts/tests/test_ci_workflows.py` | workflow tests | changed: A16; any census entry the new workflow's reads need |
| `scripts/tests/fixtures/mutation-scoped-prices-<run>/` | test fixture | added: the price run's `costs.jsonl` files in the download's layout; `<run>` is written at the fill with the run's slowest leg duration |
| `scripts/mutation-rows.d/S41800-S41899.json` | mutation rows | changed: S41801 re-anchored, keeping its id, behaviour and killer |
| `scripts/mutation-rows.d/S42000-S42099.json` | mutation rows | changed: S42025 re-anchored, keeping its id, behaviour and killer |
| `scripts/mutation-rows.d/S42100-S42199.json` | mutation rows | added: section 9's rows |
| `RELEASING.md` | release runbook | changed: one paragraph in section 2 |
| `docs/specs/SPEC-418-the-python-mutation-legs-are-dealt-by-each-files-measured-cost-every-listed-mutant-examined-or-the-plan-refused-whole.md` | records | changed: an insert-only amendment section |
| `docs/decisions/ADR-434-the-python-share-is-one-capped-value-the-price-table-is-derived-from-a-recorded-run-and-each-mutant-runs-the-tests-its-line-traced.md` | records | changed: an insert-only amendment section |
| `docs/schematics/mutation-testing-python.md` | records | changed: section 8, insert-only |
| `docs/specs/SPEC-421-each-python-file-is-priced-at-its-scoped-test-cost-measured-by-a-dispatched-run-and-the-plan-deals-scoped-legs-by-it.md` | records | added: this file |
| `docs/decisions/ADR-435-the-python-plan-prices-scoped-legs-from-a-dispatched-price-run-with-each-figure-naming-its-run-and-job.md` | records | added |
| `docs/red-first/SPEC-421.md` | records | added |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | formal model | changed: the re-read recorded in its comments, and the `shards` and `python_by_cost` digests re-stamped |
| `changelog.d/python-scoped-price-421.md` | changelog | added |

## 5. What this does NOT cover

- It raises no share and no cap. The share stays 32 and the cap 128, and the release's fit waits
  on the plan's own line after the land (#825).
- It measures no release figure. The plan's line over the release's listing, after this lands,
  is the figure; the projection in ADR-435 D5 is not (#822).
- It schedules no price run. A refresh is a dispatch and a delivery (#837).
- It leaves the weekly battery whole and unsampled (#825).
- It re-prices nothing in `PYTHON_CONTROL_SECONDS`, which stays the fallback (#837).
- It exempts no file and records no skipped mutant (#825).

## 6. Risks

- **A sample that misprices its file.** Twenty mutants are a sample of a file's mutants. Detected
  by a leg that runs past its `projected_seconds`, which the plan prints per shard. The bound of
  1800 s is half the job's limit, so a figure up to twice too low still finishes.
- **A dispatch at the branch refused.** Detected when the dispatch is made. The machinery then lands
  with an empty scoped table, every deal byte-identical by A12, and a later delivery fills it.
- **A price leg cut at its 120-minute limit.** It leaves no `report.json`, so its records are not
  copied, and its file reads `unpriced` by name and keeps its fallback.
- **The runner class drifting.** A16 holds the price workflow's runner class equal to `ci.yml`'s
  `mutation-python` job's, so a change to one is red until both move.

## 7. What only CI proves

- Every red and green of A1-A16. No test module of this delivery runs outside CI.
- The price run itself: one dispatch at the branch, after the commit that adds the workflow, read
  complete, one attempt, before the fixture commit.
- The plan line of this delivery's own run, within the share at 32.
- The release's plan line with the filled table, read when #822 re-runs. This delivery does not
  run it.

## 8. Formal model

**`formal/tla/EveryLegCounted`: re-read and re-stamped.** It covers `shards`, `whole_reports`,
`partition`, `examined_sum`, `population_gaps` and `python_by_cost` in
`scripts/mutation-verdict.py`, and `planned_leg` in `scripts/mutation_python.py`. This delivery
edits two of those spans, `shards` and `python_by_cost`. Each hunk changes a leg's weight, the
triple that prices it, and one printed line; none moves the partition of listed mutants over legs
or the count of legs, which are the model's variables. The build re-reads each hunk against the
model, records the re-read in the model's comments, and moves each digest by the digest rule. The
rule is computed at the base, where it must equal the recorded digest, and at the head. A hunk
that moves a model variable stops the build. No new model: each price leg writes its own artifact,
and the fill reads them after the run completes, so the fill is a function of its inputs.

**`formal/tla/TracedSetCovers`: unchanged.** It covers `child_trace` and `scoped_sets`, which
this delivery does not edit.

## 9. Mutation rows

Each row's find is written by the build on the line it lands, and occurs exactly once at its
target.

| row | behaviour | mutant | killer |
|---|---|---|---|
| S41801 | a leg's setup counts the control and the sentinel | re-anchored: `setup = control + sentinel` to `setup = control` | its killer, unchanged |
| S42025 | the battery stays whole | re-anchored: `SCOPED_SELECTIONS = frozenset({"plan"})` to `frozenset({"plan", "all"})` | its killer, unchanged |
| S42100 | `shards` reads the scoped table when the legs trace | the scoped arm given None | A6 |
| S42101 | k divides by the mutant figure | the divisor read as the control | A10 |
| S42102 | `python_price` prefers the scoped triple | the scoped lookup skipped | A11 |
| S42103 | a whole price is taken thrice | the whole triple's third field set to 1 | A12 |
| S42104 | the control figure is the largest | `max` to `min` over the control seconds | A7 |
| S42105 | the sentinel figure is the largest | `max` to `min` over the sentinel seconds | A7 |
| S42106 | the mutant figure is rounded up | the mean's ceiling to its floor | A7 |
| S42107 | a record with no job is refused | the job check disabled | A8 |
| S42108 | a whole run's mutant record is refused | the `why` check disabled | A8 |
| S42109 | one path across two jobs is refused | the job-count check disabled | A8 |
| S42110 | the sample step is ceil(m / S) | the step's ceiling to its floor | A3 |
| S42111 | `--scoped` beside `--all` is refused | that refusal disabled | A2 |
| S42112 | a traced sentinel is recorded | the sentinel record left unwritten | A1 |
| S42113 | a setup record names its job | `where`'s `job` dropped | A4 |
| S42114 | the scoped line prints only when a file is priced scoped | its condition made always true | A14 |
| S42115 | a record with no run is refused | the run check disabled | A8 |
| S42116 | one path across two attempts is refused | the attempt-count check disabled | A8 |
| S42117 | no record reads VOID | the empty-population refusal disabled | A8 |
| S42118 | `--sample` without `--scoped` is refused | that refusal disabled | A2 |
| S42120 onward | each table line's mutant figure | the figure plus 1, one row per line | A9 |
| two rows after them | `scripts/mutation-verdict.py`'s control and sentinel figures | each figure plus 1 | A9 |

The table's rows are written at the fill, one per line of `PYTHON_SCOPED_SECONDS`, from S42120
upward, then the two rows on `scripts/mutation-verdict.py`'s line. With sixteen lines they are
S42120-S42135, then S42136 and S42137.

# SPEC-417: the rows leg is dealt into legs that fit their bound

- **Issue:** #830, which records the rows leg cut at its bound on a release and the weekly
  rows job's same limit; this delivery refers to it and closes nothing. **Context(s):** CI
  (`.github/workflows/`), its scripts (`scripts/`) and their tests (`scripts/tests/`); no crate
  and no Mini App source changes.
- **Decided by:** ADR-431 (the rows are dealt round robin into the fewest legs whose largest is
  projected within half of the rows job's kept 90-minute bound, at the measured mean cost; one
  merged report for the unchanged judge; the run's Rust-leg ceiling falls from 209 to 193).
- **Schematic:** `docs/schematics/mutation-testing.md`, a section appended for SPEC-417: the plan,
  the legs, the bound, the merge and the verdict, before and after.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-417.md`. **Mutation band:** `S41700-S41799` (section 9). **Model:** none
  (section 8). No job's bound is raised: `mutation-rows` keeps 90 minutes, now each leg's. Amends
  SPEC-039 (R3's `mutation-rows`; R18's 90 stands) and SPEC-362 (R3's ceiling of 209 and A1's
  release of 200 legs, section 10), each by an amendment section in its own file.

## 1. The problem, measured

- **The rows leg was cut.** Job 114529704482 (`mutation-rows`, the release pull request #822's
  run) read `Cache not found` (log line 250), fetched the graph in 38 s, and proved rows S02001 to
  S04909 for 5357 s: 331 KILLED, 0 SURVIVED. It was cancelled at its 90-minute bound
  (`ci.yml:579`; log line 1216) with 331 of the 2512 selected rows examined.
- **A cut leg leaves nothing.** The runner writes its report once, after its last row
  (`scripts/mutation_rows.py:1217-1218`), so the cut job uploaded no report.
- **What a row costs.** From the same log: mean 16.2 s, median 4.3 s, 90th percentile 22.9 s,
  largest 198.5 s (S04150). The 88 rows of the band S03900-S03999 took 3369 s in job 114529704482, 38.3 s each.
- **What a release needs.** 2512 rows at the mean are about 40,694 s, about 678 minutes: more than
  the 360 minutes one hosted job may run (SPEC-362 R2). One job cannot hold them at any bound.
- **What the run can hold.** The run's jobs share one budget (`RUN_JOBS`,
  `scripts/tests/test_dispatch_shards.py:208`); `LEG_CEILING = {"ci": 209, "battery": 234}`
  (`scripts/mutation-verdict.py:879`) is that budget less `ci.yml`'s 47 other jobs, the rows job
  among them as one. SPEC-362 pins a release of 200 Rust legs inside it.
- **What the verdict already does.** It reads the rows report at one path (`ci.yml:689-694`) and
  reads every selected row absent from it as VOID, "selected and never proved"
  (`scripts/mutation-verdict.py:1176`).

## 2. Requirements

- **R1.** `scripts/mutation_rows.py legs --rows-from <plan>` counts the rows that
  `prove --rows-from <plan>` would prove (the runner's `chosen`), and answers the fewest legs
  `n >= 1` whose largest leg, `ceil(R / n)` rows at `ROW_SECONDS` each, is projected within
  `ROWS_BOUND_SECONDS`. No rows answer 1 leg.
- **R2.** When more than `ROWS_MAX_LEGS` legs would be needed, `legs` refuses by name (exit 2),
  naming the count and the most legs, and writes no output. It never caps.
- **R3.** `legs` writes `rows_legs=<n>` and `rows_matrix=<the JSON list 0 to n-1>` to the file
  `GITHUB_OUTPUT` names, and prints one line naming `n`, the largest leg's rows and its projection.
- **R4.** `ROW_SECONDS = 17`, `ROWS_BOUND_SECONDS = 2700` and `ROWS_MAX_LEGS = 17` (ADR-431 D2).
- **R5.** `prove --leg k/n` proves exactly the chosen rows `[k::n]`, in the plan's order. Without
  `--leg`, `prove` proves every chosen row, as at the base.
- **R6.** A `--leg` that is not `k/n` with `0 <= k < n` and `n >= 1` is refused (exit 2) before
  any row runs.
- **R7.** `merge --rows-from <plan> --legs <n> --from <dir> --report <file>` writes to `<file>` the
  rows of every `rows-leg-<k>.json` in `<dir>`, `k` from 0 to `n-1`, in leg order. It names each
  absent leg on its own line and prints how many legs and rows it examined. It exits 0. The judge
  then reads each absent leg's rows as VOID by id (section 1).
- **R8.** A leg whose ids are not exactly its deal, or an id held by two legs, is refused by name
  (exit 2), and `merge` writes no report.
- **R9.** In `.github/workflows/ci.yml`:
  - the plan job writes `rows_legs` and `rows_matrix` as outputs, from a step running `legs`;
  - `mutation-rows` is a matrix over `fromJSON(needs.mutation-plan.outputs.rows_matrix)` with
    `fail-fast: false`; each leg proves `--leg "$LEG/$LEGS"` into `rows-leg-$LEG.json` and
    uploads it as `mutation-rows-leg-<leg>`;
  - the retirement check runs in leg 0 only;
  - the verdict's job downloads `mutation-rows-leg-*` merged into `reports/mutation-rows`, then
    runs `merge` there under the same `if:`, before the judge step;
  - the judge step's lines are unchanged.
- **R10.** `mutation-rows`' `timeout-minutes` stays 90, now each leg's, and `ROWS_BOUND_SECONDS`
  is half of it in seconds.
- **R11.** `LEG_CEILING["ci"]` is 193: the run's budget less every other `ci.yml` job, with the
  rows job counted at `ROWS_MAX_LEGS`. The ceiling test's derivation counts the rows axis at
  `ROWS_MAX_LEGS`, read from `scripts/mutation_rows.py`'s text, never imported. The comment above
  `LEG_CEILING` names 63 other `ci.yml` jobs in place of 47.
- **R12.** Row S36201 is re-anchored with the same mutant, because the change moves its find.
  S36206 keeps its find, which stays unique because `mutation-rows` keeps 90 minutes. No row is
  retired.
- **R13.** Nothing changes in `scripts/mutation_python.py`, `scripts/mutation-python.json`, any
  `PYTHON_*` setting, the `mutation-python` job, the judge step's lines, or any span a `@phx
  covers` names.
- **R14.** Every test pin of the `ci` ceiling, or of a figure derived from it, follows 193 and
  keeps its relation to the ceiling:
  - `test_the_release_listing_fits_one_run`: #691's listing of 8367 mutants is refused whole by
    name by the pull request's plan (`plan_shards`), the refusal naming 193, and sizes at 200 legs
    by the battery's `size`;
  - `test_a_listing_beyond_the_ceiling_is_refused_whole`: the pull request's case is one mutant
    past 193 legs of 68, 13125 ingest mutants, refused naming 193, and its comment says 193 legs
    hold 13124; the battery's 15913 and 15912 stay;
  - `test_the_whole_tree_is_sized_from_its_listing` and
    `test_the_whole_tree_is_sized_at_the_batterys_ceiling`: 14213 still sizes at 210 within the
    battery's 234, and only the comments that name 209 name 193;
  - every `legs 2 of ceiling 209` (`test_every_sizing_prints_its_headroom`, and the whole-tree
    test in `scripts/tests/test_mutation_verdict.py`) reads `legs 2 of ceiling 193`;
  - `test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap`: `("ci", 193)`, and with
    no ceiling named, 193 listings of 6000 s size at 193 and 194 are refused;
  - `test_the_sizing_constants_are_the_measured_ones`: `LEG_CEILING` equals
    `{"ci": 193, "battery": 234}`;
  - `scripts/tests/test_ci_workflows.py`'s register entry for the refusal test names the call it
    moves to, `plan_shards(entries('deck-streak-ingest', 13125))`, with its count of 1.

## 3. Acceptance criteria of SPEC-417

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | `legs` answers 1 leg for 0, 1 and 158 rows, 2 for 159, 3 for 317, 16 for 2512 and 2528, and 17 for 2529 and 2686 (R1, R4) | red against the red commit's stub, which answers 1 leg for every count: 159 rows read 1, want 2 | `TheRowsLegsFitTheirBound.test_the_fewest_legs_whose_largest_leg_fits_the_bound` |
| A2 | 2687 rows are refused with exit 2, a line naming 2687 and 17, and no output written (R2) | red: the stub exits 0 | `TheRowsLegsFitTheirBound.test_a_population_past_the_most_legs_is_refused_by_name` |
| A3 | for 2512 rows `GITHUB_OUTPUT` holds `rows_legs=16` and `rows_matrix` as the JSON list 0 to 15, and the printed line names 16 legs, 157 rows and 2669 s (R3) | red: the stub writes no output | `TheRowsLegsFitTheirBound.test_the_legs_reach_the_workflow_as_a_count_and_a_matrix` |
| A4 | on a fixture of three rows, legs 0/2 and 1/2 prove the first and third, then the second; together they prove each row once (R5) | red: the stub ignores `--leg`, so leg 0/2 proves all three | `TheRunnerProvesOnlyItsLegsRows.test_each_leg_proves_its_dealt_rows_and_the_legs_cover_the_plan_once` |
| A5 | `--leg 2/2`, `--leg 0/0`, `--leg -1/2` and `--leg 1` are each refused with exit 2 and no row proved (R6) | red: the stub accepts them | `TheRunnerProvesOnlyItsLegsRows.test_a_malformed_leg_is_refused_before_any_row_runs` |
| A6 | from planted leg reports, every present leg's rows reach the merged report in leg order, and with leg 1 of 3 absent, its rows stay out and one line names it (R7) | red: the stub names no absent leg | `TheMergeKeepsEveryLeg.test_the_merge_writes_every_legs_rows_and_names_a_missing_leg` |
| A7 | a planted id held by two legs, and one in a leg the deal did not give it, are each refused with exit 2 and no report written (R8) | red: the stub merges both | `TheMergeKeepsEveryLeg.test_the_merge_refuses_a_row_the_deal_did_not_give_its_leg` |
| A8 | `ci.yml` holds R9's every step and output, the merge between the rows download and the judge (R9) | red at the base: `mutation-rows` has no matrix over `rows_matrix` | `TheRowsLegsRunAsPlanned.test_the_rows_legs_run_as_planned_and_the_verdict_merges_them_first` |
| A9 | `ROWS_BOUND_SECONDS` times 2 equals `mutation-rows`' `timeout-minutes` times 60, which stays 90 (R10) | not red: the red commit's stub carries `ROWS_BOUND_SECONDS = 2700` and the base keeps 90 minutes, so it is green there; it guards the kept bound as the killer of S41701 and S41712 | `TheRowsLegsRunAsPlanned.test_the_rows_bound_is_half_of_the_rows_jobs_timeout` |
| A10 | the derived ceilings, the rows axis counted at `ROWS_MAX_LEGS`, equal `{"ci": 193, "battery": 234}` and `LEG_CEILING` (R11) | red at the base: the rows job counts as one, so `ci` derives 209 | `TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget` |
| A11 | the verdict's downloads lay every leg's report under `reports/mutation-rows`, for 1, 2 and 17 legs, and nothing there when no rows are planned (R9) | red at the base: the download asks for `mutation-rows` by name, which no leg uploads | `TheVerdictReadsEachReportByName`'s two rows tests |
| A12 | every pin of the `ci` ceiling follows 193 and keeps its relation: #691's listing refused by the pull request's plan and sized at 200 by the battery's, 13125 ingest mutants refused naming 193, the cap cases at 193 and 194, the headroom lines and `LEG_CEILING`'s pin (R14) | red at the base: the ceiling is still 209, so #691's listing plans 200 legs with exit 0 | `TheReleaseFitsOneRun`'s release, refusal and headroom tests; `test_mutation_verdict.py`'s cap, constants and whole-tree tests |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_fewest_legs_whose_largest_leg_fits_the_bound
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_population_past_the_most_legs_is_refused_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_legs_reach_the_workflow_as_a_count_and_a_matrix
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_each_leg_proves_its_dealt_rows_and_the_legs_cover_the_plan_once
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_malformed_leg_is_refused_before_any_row_runs
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_merge_writes_every_legs_rows_and_names_a_missing_leg
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_the_merge_refuses_a_row_the_deal_did_not_give_its_leg
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rows_legs_run_as_planned_and_the_verdict_merges_them_first
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_rows_bound_is_half_of_the_rows_jobs_timeout
A10: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget
A11: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k test_the_layout_holds_for_every_count_of_artifacts
A11: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k test_a_producer_that_uploaded_nothing_leaves_the_reading_unchanged
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_release_listing_fits_one_run
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_listing_beyond_the_ceiling_is_refused_whole
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_sizing_prints_its_headroom
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_sizing_constants_are_the_measured_ones
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_the_whole_tree_is_sized_at_the_batterys_ceiling
```

Every criterion is read red and green from CI's `hygiene` job, which runs `scripts/tests` through
`scripts/check.sh python`. A1 to A7 are red against stubs in the red commit that parse every new
flag and verb, carry the three constants, and answer without the behaviour. A8, A10, A11 and A12
are red against the base's workflow and ceiling. A9 is not red: it holds a relation the base
already keeps, against the constant the stub carries. No test module runs off CI for this
delivery.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: the plan job's two outputs and its `legs` step; `mutation-rows` as a matrix of legs that keeps its 90 minutes, proving and uploading its leg, the retirement check in leg 0; the verdict's rows download by pattern and its `merge` step |
| `scripts/mutation_rows.py` | CI scripts | changed: three constants, the `legs` and `merge` verbs, `prove --leg` |
| `scripts/mutation-verdict.py` | CI scripts | changed: `LEG_CEILING["ci"]`, 209 to 193, and its comment's count of other `ci.yml` jobs, 47 to 63 |
| `scripts/tests/test_mutation_rows.py` | CI tests | changed: A1 to A7 |
| `scripts/tests/test_mutation_workflows.py` | CI tests | changed: A8 and A9 |
| `scripts/tests/test_dispatch_shards.py` | CI tests | changed: `generated()` counts the rows axis; the ceiling literal (A10); the release, refusal, whole-tree and headroom pins (A12, R14) |
| `scripts/tests/test_verdict_download.py` | CI tests | changed: the rows legs' artifacts in its model, and its two rows tests (A11) |
| `scripts/tests/test_mutation_verdict.py` | CI tests | changed: the cap, constants and whole-tree pins of the `ci` ceiling, and a comment (A12, R14) |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: three register entries: the refusal test's moved call, `plan_shards(entries('deck-streak-ingest', 13125))` (R14); the release test's new `size(listing)` call (R14); and `test_mutation_workflows`' new `rows_constant` read of `scripts/mutation_rows.py`'s text (R11, A9, A10) |
| `scripts/mutation-rows.d/S41700-S41799.json` | CI scripts | added: rows S41700 to S41715 (section 9) |
| `scripts/mutation-rows.d/S36200-S36299.json` | CI scripts | changed: S36201 re-anchored (R12) |
| `docs/specs/SPEC-417-the-rows-leg-is-dealt-into-legs-that-fit-their-bound.md` | docs | added: this file |
| `docs/decisions/ADR-431-the-rows-are-dealt-by-measured-cost-into-legs-within-their-kept-bound.md` | docs | added |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | docs | changed: an amendment section appended, naming SPEC-417 |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | docs | changed: an amendment section appended, naming SPEC-417 (section 10) |
| `docs/schematics/mutation-testing.md` | docs | changed: a section appended for SPEC-417 |
| `docs/red-first/SPEC-417.md` | docs | added: A1 to A12 |
| `changelog.d/mutation-rows-417.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The weekly battery's `rows` job (`.github/workflows/mutation-weekly.yml`) proves every row in
  one job of 120 minutes, which 2512 rows at the measured cost also outgrow; it is not part of a
  release's run and stays open under #830.
- It prices every row alike. A per-row price table, dealt by cost, waits on more than one job's
  measurement (#830).
- It does not size the Python legs or change any Python mutation setting; that sizing is its own
  delivery before the release's re-run (#822).
- It changes no row's proof, killer or bound inside the runner (`BUILD_SECONDS`, `TEST_SECONDS`)
  (#830).
- It adds no formal model of the legs (section 8; #830).
- It raises no job's bound: `mutation-rows` keeps 90 minutes, now each leg's, and a raise of any
  job's bound is the maintainer's own decision (#830).

## 6. Risks

- **The price is one job's mean.** The 331 rows measured are the first in id order; the rest may
  cost more. A leg of at most 158 rows holds up to 34.2 s a row inside its job's 90 minutes, 2.1
  times the mean. Past that, the leg is cut and its rows read VOID by id. Detected by each leg's
  time against two thirds of its bound, read by job id.
- **The run's budget is shared.** The Python legs' deliveries and this one all draw on it, and
  each can edit `LEG_CEILING`'s derivation and `scripts/tests/test_dispatch_shards.py`. Counted at
  their most, 32 Python legs and 17 rows legs leave the Rust legs 193: #822's 192 fit, with 1 to
  spare, and a release of 200 does not (section 10). Whichever delivery lands later re-derives the
  ceiling at its cut.
- **The population grows.** 17 legs hold 2686 rows, 174 more than the 2512 measured, and every
  delivery adds rows: #822's re-run selects at least 2539, with SPEC-415's 8, SPEC-416's 3 and this delivery's
  16. A release whose diff selects more than 2686 is refused whole by name on its plan job, never
  cut; the most legs is then re-derived with the ceiling (ADR-431 D2).
  Detected by `legs`' refusal line, read by name.
- **Each leg starts cold.** Each pays its own restore, fetch and build. The measured mean holds one
  cold start, spread over fewer rows than a leg proves.
- **The red commit's own run.** It keeps the base's 90-minute rows job, which can be cut again on
  that run's selection; that cut is this SPEC's problem, recorded, not a new red.

## 7. What only CI proves

A1 to A12's reds and greens, the rows' census and proof, and A10 and A12 at all (no module of
theirs runs off CI) are CI's, read by name. The release's re-run shows each rows leg's time, read
by job id; the slowest leg within 1.3 times the mean leg says the deal balances.

## 8. Formal model

None (ADR-431 D8). The legs share no written object, and the verdict's job reads their reports
only after every leg has ended, so the merge is a total function of the reports present, held by
A6 and A7. Five covers name `scripts/mutation-verdict.py` (`formal/tla/EveryLegCounted`: `shards`,
`whole_reports`, `partition`, `examined_sum`, `population_gaps`); none of their spans is edited.
`shards` reads `LEG_CEILING`, whose value reaches no variable of the model. No cover names
`ci.yml` or `scripts/mutation_rows.py`.

## 9. Mutation rows

| stem | file | mutant | killer |
|---|---|---|---|
| `S41700-THE-ROW-PRICE-IS-THE-MEASURED-MEAN` | `scripts/mutation_rows.py` | `ROW_SECONDS` is 16 | `test_mutation_rows.TheRowsLegsFitTheirBound.test_the_fewest_legs_whose_largest_leg_fits_the_bound` |
| `S41701-THE-ROWS-BOUND-IS-HALF-OF-ITS-JOBS-TIMEOUT` | `scripts/mutation_rows.py` | `ROWS_BOUND_SECONDS` is 2701 | `test_mutation_workflows.TheRowsLegsRunAsPlanned.test_the_rows_bound_is_half_of_the_rows_jobs_timeout` |
| `S41702-THE-MOST-ROWS-LEGS-IS-SEVENTEEN` | `scripts/mutation_rows.py` | `ROWS_MAX_LEGS` is 18 | `test_mutation_rows.TheRowsLegsFitTheirBound.test_a_population_past_the_most_legs_is_refused_by_name` |
| `S41703-THE-LEGS-ROUND-UP` | `scripts/mutation_rows.py` | a leg's rows round down | `test_mutation_rows.TheRowsLegsFitTheirBound.test_the_fewest_legs_whose_largest_leg_fits_the_bound` |
| `S41704-A-POPULATION-PAST-THE-MOST-IS-CAPPED` | `scripts/mutation_rows.py` | the refusal answers `ROWS_MAX_LEGS` | `test_mutation_rows.TheRowsLegsFitTheirBound.test_a_population_past_the_most_legs_is_refused_by_name` |
| `S41705-NO-ROWS-RUN-NO-LEG` | `scripts/mutation_rows.py` | no rows answer 0 legs | `test_mutation_rows.TheRowsLegsFitTheirBound.test_the_fewest_legs_whose_largest_leg_fits_the_bound` |
| `S41706-THE-MATRIX-NEVER-REACHES-THE-WORKFLOW` | `scripts/mutation_rows.py` | the `rows_matrix` line is not written | `test_mutation_rows.TheRowsLegsFitTheirBound.test_the_legs_reach_the_workflow_as_a_count_and_a_matrix` |
| `S41707-A-LEG-PROVES-EVERY-ROW` | `scripts/mutation_rows.py` | the deal's slice step is 1 | `test_mutation_rows.TheRunnerProvesOnlyItsLegsRows.test_each_leg_proves_its_dealt_rows_and_the_legs_cover_the_plan_once` |
| `S41708-A-LEG-PAST-ITS-COUNT-IS-ACCEPTED` | `scripts/mutation_rows.py` | the `k < n` check is gone | `test_mutation_rows.TheRunnerProvesOnlyItsLegsRows.test_a_malformed_leg_is_refused_before_any_row_runs` |
| `S41709-THE-MERGE-DROPS-THE-LAST-LEG` | `scripts/mutation_rows.py` | the merge reads legs 0 to n-2 | `test_mutation_rows.TheMergeKeepsEveryLeg.test_the_merge_writes_every_legs_rows_and_names_a_missing_leg` |
| `S41710-THE-MERGE-KEEPS-A-DUPLICATE` | `scripts/mutation_rows.py` | the duplicate check is gone | `test_mutation_rows.TheMergeKeepsEveryLeg.test_the_merge_refuses_a_row_the_deal_did_not_give_its_leg` |
| `S41711-THE-MERGE-KEEPS-A-MISDEALT-ROW` | `scripts/mutation_rows.py` | the deal check is gone | `test_mutation_rows.TheMergeKeepsEveryLeg.test_the_merge_refuses_a_row_the_deal_did_not_give_its_leg` |
| `S41712-THE-ROWS-LEG-KEEPS-NINETY-MINUTES` | `.github/workflows/ci.yml` | `mutation-rows`' bound is 91 | `test_mutation_workflows.TheRowsLegsRunAsPlanned.test_the_rows_bound_is_half_of_the_rows_jobs_timeout` |
| `S41713-THE-ROWS-JOB-RUNS-ONE-LEG` | `.github/workflows/ci.yml` | the rows matrix is `[0]` | `test_mutation_workflows.TheRowsLegsRunAsPlanned.test_the_rows_legs_run_as_planned_and_the_verdict_merges_them_first` |
| `S41714-EACH-LEG-PROVES-THE-WHOLE-PLAN` | `.github/workflows/ci.yml` | the prove line loses `--leg "$LEG/$LEGS"` | `test_mutation_workflows.TheRowsLegsRunAsPlanned.test_the_rows_legs_run_as_planned_and_the_verdict_merges_them_first` |
| `S41715-THE-VERDICT-JUDGES-UNMERGED-ROWS` | `.github/workflows/ci.yml` | the merge step's `if:` reads `== 'false'` | `test_mutation_workflows.TheRowsLegsRunAsPlanned.test_the_rows_legs_run_as_planned_and_the_verdict_merges_them_first` |

One existing row is re-anchored with its mutant unchanged (R12): S36201's find `"ci": 209`
becomes `"ci": 193` (replacement `"ci": 194`). S36206's find, `mutation-rust`'s
`timeout-minutes: 360` line, stays unique because `mutation-rows` keeps 90 minutes, and S41712's
find, `mutation-rows`' `timeout-minutes: 90` line, is the only such line in `ci.yml` at the base.

## 10. SPEC-362's pins, amended (a tightening)

SPEC-362 R3 makes the pull request's ceiling for Rust legs the run's job budget, 256, less the most
jobs every other `ci.yml` job can generate; that rule is unchanged. Counting `mutation-rows` at its
most legs moves the value R3 derives, and the release SPEC-362 holds inside it:

| SPEC-362 pin | old | new | derivation |
|---|---|---|---|
| R3's value, `LEG_CEILING["ci"]` (`scripts/mutation-verdict.py:879`) | 209 | 193 | at the base, 256 less 47: 13 single jobs, the rows job among them, the engine's 2 slices and 32 Python legs; after it, 256 less 63: 12 single jobs, the 2 slices, 32 Python legs and 17 rows legs |
| A1, the release's listing at #691 within the `ci` ceiling | 200 legs, within 209 | refused whole by name by the pull request's plan, and sized at 200 legs within the battery's 234 (R14) | 200 is more than 193 |
| section 6's risk, the whole tree's 200 legs of a ceiling of 209 | 9 legs to spare | 7 legs past the pull request's 193, refused by name; #822's 192 fit with 1 to spare | 193 less 192 |

No bound rises: the ceiling falls by 16, and no other bound moves. The value is re-derived at this
delivery's cut, after the Python plan's refusal lands, and written in SPEC-362's amendment section
with the count it was derived from.

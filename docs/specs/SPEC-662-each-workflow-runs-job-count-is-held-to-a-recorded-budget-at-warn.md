# SPEC-662: each workflow run's job count is held to a recorded budget, at warn

Path: `docs/specs/SPEC-662-each-workflow-runs-job-count-is-held-to-a-recorded-budget-at-warn.md`.
Decided by ADR-684. Follows SPEC-436 and ADR-450 (a new rule starts at warn; ADR-450 D6's
promotion rule) and sits beside SPEC-422 R14 and ADR-436 (the limit of one matrix), for #1382.

## 1. The problem, measured

`.github/workflows/matrix-limit.yml` measures the limit of one matrix (SPEC-422 R14), and
`scripts/tests/test_dispatch_shards.py` derives the mutation legs' ceilings from the jobs of
`ci.yml` and `mutation-weekly.yml` (SPEC-362 R3). Nothing counts the jobs each workflow's run can
start, so a workflow change that adds jobs to a run is read by no check.

| fact | value | command |
|---|---|---|
| workflow files | 18 | `git ls-tree --name-only HEAD .github/workflows/ \| grep -c -E '\.ya?ml$'` |
| runs a push to dev or a pull request into dev starts | 14, a lower bound | this census at push 2, the hygiene job's RUN lines |
| jobs counted over those runs | 37 for a push, 70 for a pull request, a lower bound | the same lines |
| jobs whose matrix is computed at run time | 9, a lower bound | the same job's undecided lines |

The figures are counted syntactically, by the rule of R2, at this delivery's base. Push 1 carries
the red commit alone, over stubs that count every job once; the hygiene job of push 2, which
carries the green, prints the figures over every workflow file.

## 2. Requirements

R1. A run is counted for a push to dev (`push`) and a pull request into dev (`pull_request`,
    `pull_request_target`). An event starts a run when it has no branch filter and is not filtered
    by tags alone, when its `branches` lists dev or a pattern, or when its `branches-ignore` does
    not list dev. No other event is counted.
R2. A run's count: each job is one; a static matrix is its combinations, each `exclude` entry
    dropping every combination that holds all its values, then each `include` entry that extends
    no combination adding one; a job-level call of a workflow of the same directory (the
    `$/.github/workflows/<file>` form) is that workflow's count, once per combination of the
    caller's matrix; a job its condition or a path filter can skip is counted; a matrix computed at
    run time, and any other call, is counted once and named as undecided with what it reads.
R3. The population is every workflow file of `.github/workflows/`, read through the loader and the
    reader of `scripts/tests/test_ci_workflows.py`. A directory with no workflow file fails as
    VOID, and a file the reader refuses fails by its name and line, at both levels.
R4. Every count is a lower bound. This SPEC states no exact count over the population beyond what
    the hygiene job of push 2 prints.
R5. The repository test prints `examined <n> workflow(s)`, the runs it examined, and one RUN line
    per run with its count and its undecided jobs.
R6. `BUDGET` maps each run, (workflow file, event), to the most jobs it may start and a reason.
    Its figures are each run's count at this delivery's base. A change that raises a run past its
    figure records the new figure and its reason in the same change.
R7. `UNDECIDED` maps each job whose count the census cannot decide, (workflow file, job), to what
    bounds it.
R8. The verdict prints `WARN job-budget <file> <event>: <n> job(s), past <budget>` for a run past
    its budget, `... no budget` for a run with no entry, `WARN job-budget-undecided <file> <job>:
    <source>` for an undecided job not listed, `WARN job-budget <file> <event>: no reason` for an
    entry with an empty reason, `STALE job-budget <file> <event>` for a budget entry no run
    matches, and `STALE job-budget-undecided <file> <job>` for an undecided entry no job matches.
R9. `LEVEL` is `Level.WARN`: every line of R8 prints, each STALE line fails, and every other line
    passes. A STALE line fails at both levels, as SPEC-436 R21 holds for its census. At
    `Level.FAIL` every line of R8 fails. Moving `LEVEL` to `Level.FAIL` is its own delivery under
    ADR-450 D6; demoting it once at fail is a weakening.
R10. No workflow, job, matrix, trigger or condition changes, and no existing test, pin, census or
    row changes.

## 3. Acceptance criteria of SPEC-662

| id | criterion | decided by |
|---|---|---|
| A1 | R2: the planted static matrix with `exclude` and `include` counts 6 | `test_a_static_matrix_counts_its_combinations_after_exclude_then_include` |
| A2 | R2: the planted call from a two-combination matrix counts the called workflow's 3 jobs twice | `test_a_job_level_call_counts_the_called_jobs_per_combination` |
| A3 | R2: the planted job its condition skips is counted with its matrix | `test_a_job_its_condition_can_skip_is_still_counted` |
| A4 | R2: the planted computed matrix and the call to another repository are each counted once and named undecided | `test_a_computed_matrix_or_a_foreign_call_is_counted_once_and_undecided` |
| A5 | R1: exactly the planted runs a push to dev or a pull request into dev starts are counted | `test_only_a_push_to_dev_and_a_pull_request_into_dev_start_a_run` |
| A6 | R8, R9: a planted run past its budget warns and its line does not fail at warn, and one at or under it does not warn | `test_a_run_past_its_budget_warns_and_a_run_at_or_under_it_does_not` |
| A7 | R6, R8, R9: each unlisted run, unlisted undecided job, stale entry and missing reason is named; each stale entry fails at warn, and every line fails at fail | `test_each_unlisted_run_or_job_stale_entry_and_missing_reason_is_named` |
| A8 | R3: an empty population fails as VOID | `test_an_empty_population_is_void` |
| A9 | R3 to R9: over the repository the census examines every workflow file, prints each run and passes at warn | `test_every_run_is_counted_and_held_to_its_budget_at_warn` |

```acceptance
A1: python3 -m unittest test_ci_job_budget.TheRunsAreCountedByOneRule.test_a_static_matrix_counts_its_combinations_after_exclude_then_include
A2: python3 -m unittest test_ci_job_budget.TheRunsAreCountedByOneRule.test_a_job_level_call_counts_the_called_jobs_per_combination
A3: python3 -m unittest test_ci_job_budget.TheRunsAreCountedByOneRule.test_a_job_its_condition_can_skip_is_still_counted
A4: python3 -m unittest test_ci_job_budget.TheRunsAreCountedByOneRule.test_a_computed_matrix_or_a_foreign_call_is_counted_once_and_undecided
A5: python3 -m unittest test_ci_job_budget.TheRunsAreCountedByOneRule.test_only_a_push_to_dev_and_a_pull_request_into_dev_start_a_run
A6: python3 -m unittest test_ci_job_budget.TheVerdictHoldsEachRunToItsBudget.test_a_run_past_its_budget_warns_and_a_run_at_or_under_it_does_not
A7: python3 -m unittest test_ci_job_budget.TheVerdictHoldsEachRunToItsBudget.test_each_unlisted_run_or_job_stale_entry_and_missing_reason_is_named
A8: python3 -m unittest test_ci_job_budget.TheVerdictHoldsEachRunToItsBudget.test_an_empty_population_is_void
A9: python3 -m unittest test_ci_job_budget.TheRepositoryRunsHoldTheirBudget.test_every_run_is_counted_and_held_to_its_budget_at_warn
```

Each command runs from `scripts/tests`. A9 reads every workflow file and is read in CI by name.

## 4. File manifest

| file | change |
|---|---|
| `scripts/tests/test_ci_job_budget.py` | added: the census, its budget and its tests |
| `scripts/tests/fixtures/job-budget/planted/*.yml` | added: ten planted workflow files |
| `scripts/tests/fixtures/job-budget/empty/README.md` | added: the empty population |
| `scripts/mutation-rows.d/S66200-S66299.json` | added: the hand rows |
| `docs/specs/SPEC-662-each-workflow-runs-job-count-is-held-to-a-recorded-budget-at-warn.md` | added: this SPEC |
| `docs/decisions/ADR-684-each-workflow-runs-job-count-is-a-census-at-warn-against-a-recorded-budget.md` | added: the decision |
| `docs/red-first/SPEC-662.md` | added: the red-first record |
| `changelog.d/ci-job-budget-NEW.md` | added: the fragment |

## 5. What this does NOT cover

- It does not count runs started by other events, a push to main, a tag, a schedule, a dispatch
  or a run another workflow's completion starts, #1382.
- It does not decide a matrix computed at run time; its bound stays where SPEC-362, SPEC-417,
  SPEC-420, SPEC-421 and SPEC-422 hold it, #1382.
- It does not change any workflow, and does not move `LEVEL` to fail, #1382.
- It does not record timing, #1382.

## 6. Risks

- **A sibling change adds or removes jobs.** A pull request that lands first and adds a job moves
  a count, and the census prints a WARN line at the merge; one that removes a workflow or an
  undecided job leaves its entry stale, and the census fails at the merge naming it. Each list is
  re-derived at the build's base.
- **The reader refuses a workflow GitHub accepts.** The census fails naming the file; the
  hardening tests read the same files with the same reader, so the two fail together.
- **A tree-scanning census reads the new files.** `test_ci_workflows.py`'s read census counts the
  new module in its population; the module reads files only through the loader. A census that
  reads every `.yml` file would meet the planted workflows; each is run before the push.

## 7. What only CI proves

A9 reads every workflow file, so it is read in the hygiene job by name; its lines are the counts
section 1 records.

## 8. Formal model

No state, actor or write path is added; the census is a pure reader of committed text, closed by
its planted population and section 9's rows. No model checker runs.

## 9. Mutation rows (S66200-S66299)

Rows S66200 to S66227 in `scripts/mutation-rows.d/S66200-S66299.json`, each on
`scripts/tests/test_ci_job_budget.py`, each find occurring once, each killed by a planted test of
the module by its full id, and each description naming its requirement.

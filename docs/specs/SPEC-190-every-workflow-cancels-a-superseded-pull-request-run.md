# SPEC-190: every workflow with a pull-request trigger cancels the run a newer push supersedes

- **Wave:** W4. **Issue:** #369. **Context(s):** `repo` (`.github/workflows/`, its tests and `docs/`).
- **Decided by:** ADR-190 (this SPEC's own: one concurrency rule for every workflow, and what it
  was chosen against), ADR-055 (the rule for `ci.yml` and why GitHub's one pending run per group
  shapes it) and SPEC-038 R8 (the pull-request concurrency rule; it takes an insert-only amendment).
- **Status:** delivered. It waited in `docs/specs/planned/` from its own commit until its tests
  were green, and the delivery moved it to `docs/specs/` (ADR-016). It holds
  `docs/red-first/SPEC-190.md`.

## 1. The problem, measured

- **`ci.yml` cancels a superseded pull-request run and two other workflows do not.** `ci.yml` groups
  a pull request's runs by its ref with `cancel-in-progress` true for a pull request (SPEC-038 R8).
  `changelog.yml` sets no concurrency block, so every superseded run finishes. `engine-measure.yml`
  groups by ref with `cancel-in-progress: false`, so a superseded run finishes and the next one
  waits behind it. Each holds a job slot for a result nobody reads (#369).
- **Source:** `grep -n -A3 '^concurrency:' .github/workflows/*.yml` lists the four blocks that exist
  (`ci.yml`, `engine-measure.yml`, `mutation-weekly.yml`, `release.yml`), and `changelog.yml` has none.
- **The blocks that already follow the rule are `ci.yml` and `mutation-weekly.yml`,** whose group is
  `<workflow>-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}`. Nothing
  holds a new or changed workflow to it.

## 2. Requirements

R1. **Every workflow with a `pull_request` trigger carries `ci.yml`'s workflow-level block,** with
the workflow's own name as the group's prefix: `group: <workflow name>-${{ github.event_name ==
'pull_request' && github.ref || github.run_id }}` and `cancel-in-progress: ${{ github.event_name ==
'pull_request' }}`. No job sets a group of its own (SPEC-038 R8).

R2. **`changelog.yml` gains the block.** It had none.

R3. **`engine-measure.yml`'s block becomes the rule's.** Its group was `engine-measure-${{ github.ref }}`
and its `cancel-in-progress` was `false`.

R4. **A run that is not a pull request's is never cancelled.** For every workflow with a
`pull_request` trigger, two push, tag, schedule or dispatch runs have two different groups (the run id arm), and
`cancel-in-progress` is false for each.

R5. **`release.yml` keeps `cancel-in-progress: false`.** It runs on a push of a tag only, so it has
no pull-request trigger and the rule does not apply; it never cancels a run.

R6. **`ci.yml`'s and `mutation-weekly.yml`'s blocks are unchanged,** byte for byte.

R7. **A test holds every workflow file in the directory to R1, R4 and R5,** present and future, and
prints how many workflows it examined.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every workflow with a `pull_request` trigger carries the block of R1, with its own name as the prefix, and sets no job-level group; in the scenarios a newer run of a pull request cancels the one it supersedes, two pull requests do not share a group, and a pull request never shares a push's group | `test_workflow_concurrency.py` `every_workflow_with_a_pull_request_trigger_follows_the_rule` |
| A2 | for every workflow with a `pull_request` trigger, two runs of a push, a tag, a schedule or a dispatch have different groups, and none is cancelled | `test_workflow_concurrency.py` `a_run_that_is_not_a_pull_requests_is_unique_and_never_cancelled` |
| A3 | `release.yml` has no pull-request trigger and its `cancel-in-progress` is false | `test_workflow_concurrency.py` `the_release_workflow_never_cancels` |
| A4 | `ci.yml`'s and `mutation-weekly.yml`'s blocks are the rule's, unchanged | `test_workflow_concurrency.py` `the_blocks_that_already_followed_the_rule_are_unchanged` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k every_workflow_with_a_pull_request_trigger_follows_the_rule
A2: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k a_run_that_is_not_a_pull_requests_is_unique_and_never_cancelled
A3: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_release_workflow_never_cancels
A4: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_blocks_that_already_followed_the_rule_are_unchanged
```

The test reads each workflow with the reader `test_ci_workflows.py` uses and renders the group and
the condition in scenarios (a pull request, pushes to dev and main, a tag, a schedule, a dispatch)
with that file's evaluator. Each test prints how many workflows it examined and refuses zero. The
set of checks a pull request's newest run reports is the same before and after: `ci`, `changelog`'s
`fragment`, `engine-measure`'s job and `mutation-weekly`'s `rehearsal` keep their names, so only a
superseded run differs.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/changelog.yml` | repo | changed: R2, the block |
| `.github/workflows/engine-measure.yml` | repo | changed: R3, the block |
| `scripts/tests/test_workflow_concurrency.py` | repo | added: A1 to A4 |
| `scripts/mutation-rows.d/S19000-S19099.json` | repo | added: three hand-proved rows |
| `docs/specs/SPEC-190-every-workflow-cancels-a-superseded-pull-request-run.md` | repo | added, from `docs/specs/planned/` |
| `docs/decisions/ADR-190-one-concurrency-rule-for-every-workflow-with-a-pull-request-trigger.md` | repo | added |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | repo | changed: an insert-only amendment at its end |
| `docs/red-first/SPEC-190.md` | repo | added |
| `changelog.d/ci-concurrency-190.md` | repo | added |

No schematic: the change adds no component; the rule is `ci.yml`'s, applied to two more files.

## 5. What this does NOT do

- It does not change the strict up-to-date rule on `dev`, because that is a ruleset and not a
  workflow (#369).
- It does not change `mutation-weekly.yml`'s block, which already follows the rule (#369).
- It changes no Rust and no Python outside the new test, because the defect is in two workflows'
  concurrency blocks (#369).
- It does not cancel a push, tag, schedule or dispatch run, because a land's full run must finish
  (ADR-055, #369).
- It changes no check's name and no job, so the set of checks a pull request's newest run reports
  is the one it reported before (#369).

## 6. Risks

- **GitHub keeps one pending run per group.** A pull request's group is its ref, so a third push
  cancels a pending second one. That is the intent for a pull request; a push has a group of its
  own, so it is never affected (ADR-055).
- **A path-filtered workflow on a pull request.** `engine-measure` runs only when its paths change;
  a newer push that no longer changes them starts no run, so the superseded run is not cancelled and
  finishes. That is today's behaviour for `ci.yml` too and is left as it is.

## 7. References

Issue #369; SPEC-038 R8; ADR-055; ADR-016; ADR-190.

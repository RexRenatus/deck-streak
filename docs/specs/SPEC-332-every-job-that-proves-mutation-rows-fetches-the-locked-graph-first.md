# SPEC-332: every job that proves mutation rows fetches the locked graph first

- **Issue:** #606. **Context(s):** none (the CI workflows, not a bounded context).
- **Decided by:** ADR-333 (one `cargo fetch --locked` step per proving job, chosen against three other cures).
- **Status:** delivered by the pull request that adds this file, with its test and
  `docs/red-first/SPEC-332.md`. **Mutation band:** S33200-S33299 (section 6).

## 1. The problem, measured

- A job that proves rows restores the Rust cache by key. With no exact hit, `actions/cache/restore`
  falls back to an older cache by key prefix, and that cache's registry can lack a package the
  current `Cargo.lock` adds.
- The rows runner then runs `scripts/mutation_rows.py prove`. A killer whose first cargo call is
  offline (the settle census's `cargo metadata --format-version 1 --locked --offline`) fails at
  once, every row on it reads VOID, and `mutation-verdict` fails for a reason that is not in the
  tree. Measured on pull request #605 at its head, and red at dev under the same condition (#606).
- The population, from `grep -n 'mutation_rows.py prove' .github/workflows/*.yml` at dev
  `69fe44ee`: `ci.yml` job `mutation-rows` (cache restore at line 574, prove at line 591),
  `mutation-weekly.yml` job `rows` (restore at line 273, prove at line 287) and
  `mutation-weekly.yml` job `rehearsal` (restore at line 414, prove at line 444). Three jobs,
  none of which fetches.

## 2. Requirements

R1. Every job in `.github/workflows/` that runs `mutation_rows.py prove` has a step running
    `cargo fetch --locked`, placed after the job's last cache-restore step and before its first
    prove step.
R2. The fetch step fails its job when the fetch fails: it carries no `continue-on-error`, no `||`
    and no `set +e`.
R3. No cache key, restore-key or save step, no action and no pin changes.
R4. A test over the workflow files fails when any proving job lacks the step, orders it outside
    that window, or lets it swallow its exit code, and it fails when it examines no job.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every proving job has the fetch step after its last cache restore and before its first prove | `python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_each_proving_job_fetches_after_its_cache_restore_and_before_prove` |
| A2 | the fetch step has no `continue-on-error`, no `\|\|` and no `set +e` | `python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_the_fetch_step_fails_its_job_when_the_fetch_fails` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_each_proving_job_fetches_after_its_cache_restore_and_before_prove
A2: python3 -m unittest discover -s scripts/tests -p test_rows_jobs_fetch_graph.py -k test_the_fetch_step_fails_its_job_when_the_fetch_fails
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/tests/test_rows_jobs_fetch_graph.py` | none | added |
| `.github/workflows/ci.yml` | none | changed: one step in `mutation-rows` |
| `.github/workflows/mutation-weekly.yml` | none | changed: one step in `rows`, one in `rehearsal` |
| `scripts/mutation-rows.d/S33200-S33299.json` | none | added |
| `docs/specs/SPEC-332-every-job-that-proves-mutation-rows-fetches-the-locked-graph-first.md` | none | added |
| `docs/decisions/ADR-333-the-proving-job-fetches-the-graph-in-its-own-step.md` | none | added |
| `docs/red-first/SPEC-332.md` | none | added |
| `changelog.d/rows-fetch-graph-606.md` | none | added |

## 5. What this does NOT do

- It does not change the settle census's own offline call: the census measures the tree it is given (#606).
- It does not touch a job that runs an offline cargo call behind a restore-only cache without running `prove` (#606).
- It does not change a cache key, a restore-key or a save step (#606).

## 6. Risks

- A job's fetch goes network-bound: a registry outage then fails the job at the fetch rather than at
  the first killer. That is a named failure in the right place, and the step is not retried.
- A future proving job is added without the step. The test enumerates every workflow, so it fails then.
- Mutation rows: S33200 and S33201 mutate `ci.yml`'s fetch step (delete it, move it after prove)
  and are killed by the new test.

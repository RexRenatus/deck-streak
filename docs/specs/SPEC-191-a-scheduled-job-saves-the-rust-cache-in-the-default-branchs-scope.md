# SPEC-191: a scheduled job saves the Rust cache in the default branch's scope, so a run on any ref can restore it

- **Wave:** W4. **Issue:** #370. **Context(s):** `repo` (`.github/workflows/rust-cache.yml`, its
  tests and `docs/`).
- **Decided by:** ADR-191 (this SPEC's own: a scheduled job, and what it was chosen against),
  ADR-055 and SPEC-038 R1 and R2 (the cache and who saves it; SPEC-038 takes an insert-only
  amendment).
- **Status:** delivered. It holds `docs/red-first/SPEC-191.md`.

## 1. The problem, measured

- **A cache is restorable only from a run's own ref, its pull request's base branch and the default
  branch.** `actions/cache`'s README: "The cache is scoped to the key, version, and branch. The
  default branch cache is available to other branches." Its tips add: "Reusing cache across feature
  branches is not allowed today ... if both feature branches are from the default branch, a good way
  to achieve this is to ensure that the default branch has a cache."
- **Every Rust cache saved so far is in `dev`'s scope.** SPEC-038 R2 lets only a push to `dev` or
  `main` save one, and `dev` is not the default branch. A `workflow_dispatch` run on a feature
  branch (a per-package mutation dispatch, for example) therefore restores no Rust cache and
  compiles every dependency from nothing (#370).
- **A scheduled run executes the default branch's copy of the workflow and runs in its scope.** So
  an entry a scheduled job saves is restorable by a run on any ref. A schedule runs only from the
  default branch's copy of a workflow: the job takes effect when the release pull request carries
  `rust-cache.yml` to `main`, and not before.
- **The action's own outputs make the stop cheap.** `actions/cache/restore` takes `lookup-only`
  ("Only check existence, skip download"), and its `cache-hit` output is "`true` for an exact
  primary-key match, otherwise `false`"; `cache-primary-key` is "the resolved primary key passed in
  the input".

## 2. Requirements

R1. A new workflow, `.github/workflows/rust-cache.yml`, runs on `schedule` (`43 */2 * * *`: every
    two hours, at a minute no other workflow's cron uses) and `workflow_dispatch`, and on no
    `pull_request` or `push`, so it never saves into a feature branch's scope on its own.
R2. It has `permissions: contents: read` and `concurrency: {group: rust-cache, cancel-in-progress:
    false}`: a run waits for the one before it and none is cancelled while it saves.
R3. Its one job checks out `dev` with `persist-credentials: false`, so the tree whose lockfile it
    keys on is the tree `ci.yml` keys on for a push to `dev`.
R4. It asks for `ci.yml`'s Rust cache key over that tree, with `actions/cache/restore` and
    `lookup-only: true` and no `restore-keys`. The key expression is character for character the
    one `ci.yml`'s and `mutation-weekly.yml`'s Rust cache restore steps carry. On an exact hit every
    later step is skipped, so the job ends green having built and saved nothing.
R5. On a miss it installs what `ci.yml`'s `rust` job installs before it builds (the pinned toolchain
    by `rustup show`, `cargo-nextest` and `cargo-deny` by the same action, and `protoc` 31.1 by the
    same checksum-verified step), restores by `ci.yml`'s prefix (`restore-keys`, the key up to the
    lockfile's hash) so a lockfile change recompiles only what changed, and paths equal to
    `ci.yml`'s cache paths.
R6. It compiles what `ci.yml`'s `rust` job compiles and runs no test: `cargo clippy --workspace
    --all-targets --locked` (the `clippy` stage's build, minus `-D warnings`, which changes no
    dependency's artifacts and would only stop the job on a lint), `cargo nextest run --workspace
    --locked --no-run` (the `test` and `test-engine` stages' test binaries, built and not run). The
    `doctest` stage has no build-only form: cargo refuses `cargo test --doc --no-run` with "can't
    skip running doc tests with --no-run", and the stage uses the `test` profile, whose dependency
    artifacts the nextest build already made, so no step stands for it and `ci.yml`'s own
    `doctest` stage is unchanged. The dependency graph, the profile, the features, the target and
    `CARGO_INCREMENTAL: "0"` are the ones the stages use, so the dependency artifacts land under
    the same fingerprints. `audit-rust` builds nothing.
R7. It runs `cargo clean --workspace`, as `ci.yml` does, after the builds and before the save, so
    the entry holds the dependencies' artifacts and not the workspace's own.
R8. It saves the same paths with `actions/cache/save`, under `${{ steps.<id>.outputs.cache-primary-key
    }}` of the restore step whose key is R4's, only when R4's lookup missed.
R9. SPEC-038 R2's rule stands for every other workflow: a cache is saved only by a push to `dev` or
    `main` that missed its key. The one further admission is a workflow whose events are only
    `schedule` and `workflow_dispatch` and whose save runs only when a lookup missed; the test that
    judges R2 judges that shape in scenarios, and refuses a scheduled workflow whose save has no
    such condition and any workflow that adds a `push` or `pull_request` trigger to it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the workflow runs on a two-hourly schedule and on `workflow_dispatch`, and on nothing else | `test_rust_cache_workflow.py` `it_runs_on_a_two_hourly_schedule_and_on_dispatch_only` |
| A2 | the job checks out `dev` first, without persisting credentials | `test_rust_cache_workflow.py` `it_checks_out_dev_without_persisting_credentials` |
| A3 | the key it looks up is `ci.yml`'s and `mutation-weekly.yml`'s, character for character, and its paths are theirs | `test_rust_cache_workflow.py` `its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore` |
| A4 | the lookup is `lookup-only` on that key with no `restore-keys`, and every later step runs only on a miss | `test_rust_cache_workflow.py` `it_looks_the_exact_key_up_and_every_later_step_stops_on_a_hit` |
| A5 | on a miss it installs what `ci.yml` installs, restores by `ci.yml`'s prefix, and compiles the two builds of R6 with no test run | `test_rust_cache_workflow.py` `it_installs_restores_by_prefix_and_compiles_without_running_a_test` |
| A6 | `cargo clean --workspace` is the step before the save, after every build, and the save is keyed on the exact key's primary-key output | `test_rust_cache_workflow.py` `it_cleans_the_workspace_before_it_saves_under_the_exact_key` |
| A7 | the save-rule test admits this shape and only this one: planted scheduled workflows with no condition, with a push trigger or with a pull request trigger are refused | `test_rust_cache_workflow.py` `the_save_rule_admits_a_scheduled_save_only_when_a_lookup_missed` |
| A8 | its cron minute is no other workflow's cron minute | `test_rust_cache_workflow.py` `its_cron_minute_collides_with_no_other_workflows` |
| A9 | it defaults its token to read-only and queues instead of cancelling | `test_rust_cache_workflow.py` `it_reads_only_and_queues_instead_of_cancelling` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k it_runs_on_a_two_hourly_schedule_and_on_dispatch_only
A2: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k it_checks_out_dev_without_persisting_credentials
A3: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore
A4: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k it_looks_the_exact_key_up_and_every_later_step_stops_on_a_hit
A5: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k it_installs_restores_by_prefix_and_compiles_without_running_a_test
A6: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k it_cleans_the_workspace_before_it_saves_under_the_exact_key
A7: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k the_save_rule_admits_a_scheduled_save_only_when_a_lookup_missed
A8: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k its_cron_minute_collides_with_no_other_workflows
A9: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k it_reads_only_and_queues_instead_of_cancelling
```

The tests read the workflows through `test_ci_workflows.py`'s reader and print how many steps,
workflows or scenarios each examined, refusing zero. The live proof is two dispatched runs of the
new workflow on the delivery's branch, quoted in the pull request: the first builds and saves, the
second stops at the lookup. The dispatched entry is deleted by its cache id afterwards.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/rust-cache.yml` | repo | added: R1 to R8 |
| `scripts/tests/test_rust_cache_workflow.py` | repo | added: A1 to A9 |
| `scripts/tests/test_ci_workflows.py` | repo | changed: `cache_problems` admits R9's shape and refuses its planted variants |
| `scripts/mutation-rows.d/S19100-S19199.json` | repo | added: four script rows on the workflow |
| `docs/specs/SPEC-191-a-scheduled-job-saves-the-rust-cache-in-the-default-branchs-scope.md` | repo | added |
| `docs/decisions/ADR-191-a-scheduled-job-saves-the-rust-cache-in-the-default-branchs-scope.md` | repo | added |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | repo | changed: an insert-only amendment section at its end |
| `docs/red-first/SPEC-191.md` | repo | added |
| `changelog.d/ci-rust-cache-191.md` | repo | added |

No schematic: the change adds one workflow and no component; section 1 says where its entry lands
and who can restore it.

## 5. What this does NOT do

- It does not make `dev` the default branch, which would give every run on `dev` the same reach; that
  is a repository setting and the owner's call (#370).
- It adds no paid storage and raises no storage limit: the only new entries are the job's own, one
  per `Cargo.lock` state, under the storage the repository's cache already uses (#370).
- It does not stop `ci.yml` saving in `dev`'s scope: a push to `dev` still saves as SPEC-038 R2
  says, and stopping that is a follow-up once this job is measured live on `main` (#370).
- It changes no gate, no test selection, no timeout and no mutation count: every check examines what
  it did before, and a cache changes only where a build's inputs come from (#370).
- It does not touch `mutation-weekly.yml`, `changelog.yml` or `engine-measure.yml`, which other
  deliveries change (#370).
- It does not save a cache for a pull request or a push to a feature branch, because no other ref
  could restore that entry (#370).

## 6. Risks

- **Until the release carries the workflow to `main`, it never runs on its schedule.** Only a
  dispatch on a branch runs it; its entry then sits in that branch's scope. The live proof deletes
  it. The workflow file's copy on `main` is what a schedule runs.
- **`dev` moves between the lookup and the save.** The job checks `dev` out once, so its key and its
  tree agree; the entry may be one push behind, which the next run replaces (a new lockfile is a new
  key). A2 pins the single checkout.
- **A build flag that changes fingerprints.** R6 uses the stages' own flags, and A5 pins the
  commands; if a stage's build changes, its test fails a compare only as far as the commands are
  copied, so the tests name each stage the command stands for.
- **The doctest stage has no step here.** Its dependency artifacts are argued to be the ones the
  nextest build made (the Cargo book: `cargo test` uses the `test` profile, and the selected profile
  applies to all Cargo targets); that is not built here. The first `ci.yml` run after the release
  restores the entry: its `doctest` stage compiling no dependency confirms it.

## 7. References

Issue #370; SPEC-038 R1, R2 and its amendment; ADR-055; ADR-191; `actions/cache` README (cache
scopes and the tips on feature branches) and its `restore` action's inputs and outputs.

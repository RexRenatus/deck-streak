# SPEC-038: CI runs the gate in four parallel jobs, and only a push to dev or main saves a cache

- **Wave:** W0. **Issue:** #207 (epic #1). **Context(s):** `repo` (`.github/workflows/ci.yml`, `scripts/check.sh`, `scripts/pack-rows.py`).
- **Decided by:** ADR-017 (hosted CI runs the local gate), ADR-035 (the required checks come from GitHub Actions), ADR-004 (the vendored packs and their runner), ADR-055 (this SPEC's own: the four jobs, the stages that follow the tools, the caches and who saves them, the pool, and which runs are cancelled).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-038.md` (ADR-016).

## 1. The problem, measured

- **CI takes minutes, and all of it is one job.** The maintainer measured the single `gate` job on
  three runs at `dev` before this SPEC, the job being the whole of each run's time:

  | run | event | wall time |
  |---|---|---|
  | 36356940848 | a code pull request | 4m57s |
  | 36356754966 | a docs-only pull request | 2m43s |
  | 36357723675 | a push to `dev` | 3m53s |

  Its slowest steps, one after another: the packs stage 57 s, clippy 37 s, the Playwright install
  21 to 30 s, the web stage 30 s, nextest and the doctests 28 s. No step caches Rust: every run
  compiles every dependency from nothing.
- **The pack rows run one at a time.** `python3 scripts/pack-rows.py --json` at `dev` 8f91667
  examined 398 rows of 29 packs in 67 s on the maintainer's machine, one process after another,
  with one core busy the whole time (61 s of user time). The longest row,
  `tdd:acceptance-has-a-test`, takes 6.4 s; the rest take under 3 s each.
- **The pack rows need no build.** The same 410 report lines gave the same verdicts on a fresh
  checkout and again after `pnpm install`, the web build and the Rust build had filled the tree, so a
  packs job can run on a bare checkout.
- **The stage logs never reached a run.** Every run's `upload stage logs` step warns
  `No files were found with the provided path: .check-logs/. No artifacts will be uploaded.`
  (run 36357723675), because `actions/upload-artifact` skips hidden files unless
  `include-hidden-files` is set, and `.check-logs/` is hidden. A red stage in CI can be read only
  from its one summary line.
- **What a Rust cache would hold.** After one gate run at `dev` 8f91667, `target/` held 1.9 GiB, and
  `cargo clean --workspace --dry-run` would remove 1.0 GiB of it: the workspace's own test binaries,
  libraries and incremental state. A fresh checkout rebuilds those anyway, because Cargo judges a
  path package by its sources' modification times, so they only enlarge a cache.
- **A pull request already saves a cache.** `actions/setup-node` with `cache: pnpm` saves the pnpm
  store in its post step whenever its key missed, on any event, pull requests included (it writes
  into the pull request's own scope, which `dev` never reads).
- **A push can be cancelled today.** `ci.yml` groups runs by `github.ref` with
  `cancel-in-progress: false`. That never cancels a running push, but GitHub keeps at most one
  pending run per group and cancels the older pending run when a newer one queues, so of three quick
  pushes to `dev` the second is cancelled before it starts. A superseded pull-request run, meanwhile,
  runs to the end.
- **The `toolchain` stage cannot live in one job of four.** It checks every tool of every stage
  (cargo, cargo-nextest, cargo-deny, node, pnpm, python3, gitleaks), and no single job will hold
  them all. The `audit` stage runs `cargo deny` and `pnpm audit`, which two different jobs hold.
- **The gate itself, on the maintainer's machine** (`bash scripts/check.sh` at `dev` 8f91667,
  CHECK OK in 157 s): toolchain 0 s, fmt 0 s, clippy 17 s, test 17 s, doctest 1 s, web 35 s,
  python 8 s, packs 69 s, scrub 8 s, audit 2 s, secrets 0 s.

**Order.** PR #204 (SPEC-022, Anki's engine) edits `ci.yml` and `check.sh` too: it adds a
checksum-pinned `protoc` step to the gate job and a `protoc` check to the `toolchain` stage. When it
lands, this delivery merges `dev` in, carries the `protoc` step into the one job that compiles Rust
and the `protoc` check into the stages that compile, leaves `engine-measure.yml` as it is, and
measures cold against warm with the engine in the workspace.

## 2. Requirements

R1. The `rust` job restores `~/.cargo/registry/index/`, `~/.cargo/registry/cache/`,
    `~/.cargo/git/db/` and `target/` with `actions/cache/restore`, pinned by its full commit SHA,
    under the key `rust-<runner os>-<hashFiles('rust-toolchain.toml')>-<hashFiles('Cargo.lock')>`,
    falling back to the newest entry of the same toolchain pin. After its stages it saves the same
    paths under the same key with `actions/cache/save`, only when R2 allows it, and only after
    `cargo clean --workspace` has left the dependencies' artifacts alone in `target/`. It sets
    `CARGO_INCREMENTAL=0`, because incremental state is rebuilt from a fresh checkout anyway.
R2. Every cache in every workflow is saved only by a push to `dev` or `main` that missed its exact
    key. A pull request, from this repository or a fork, restores and never saves. No step uses an
    action that saves a cache by itself: `actions/cache` (which saves in its post step on any
    event), `actions/setup-node` with a `cache` input or without `package-manager-cache: false`, or
    `pnpm/action-setup` with `cache: true`. The pnpm store moves from `setup-node`'s cache to a
    restore and save pair under this rule.
R3. The gate runs in four jobs that need nothing and so start together, each calling
    `bash scripts/check.sh` once with its own stages:
    - `rust`: `fmt clippy test doctest audit-rust`;
    - `web`: `web audit-web`;
    - `packs`: `packs`;
    - `hygiene`: `python scrub secrets`.
    Every stage `check.sh` defines runs in exactly one job. The aggregate `ci` needs these four,
    `workflow-lint` and `base-is-dev`. The required contexts (`ci` and `fragment`, from the app
    15368) and the rulesets are unchanged. Each gate step's name says what its stages run, because
    the vendored stack-selection row `svelte-check-ci` reads a workflow's text for `svelte-check`
    and cannot follow `check.sh` (amended by the delivery: a name without it read RED).
R4. Each stage checks the tools it runs before it runs them, with the install hint the `toolchain`
    stage gave: `cargo` for `fmt`, `clippy` and `doctest`; `cargo` then `cargo-nextest` for `test`;
    `cargo` then `cargo-deny` for `audit-rust`; `node` (24 or later) then `pnpm` for `web` and
    `audit-web`; `python3` (3.11 or later) for `python`, `packs` and `scrub`; `gitleaks` for
    `secrets`. A missing tool fails that stage by name, locally and in CI. The `toolchain` stage is
    removed.
R5. The `audit` stage is split: `audit-rust` runs
    `cargo deny --locked check advisories bans licenses sources` in the `rust` job, and `audit-web`
    runs `pnpm audit --prod` in the `web` job.
R6. The jobs that read history check out all of it (`fetch-depth: 0`): `hygiene`, whose `secrets`
    stage scans every commit with `CHECK_HISTORY=1` and whose `scrub` stage reads every blob
    reachable from `HEAD`, and `packs`, whose sdd numbering row reads every branch. `rust` and `web`
    check out one commit.
R7. `scripts/pack-rows.py` runs rows in a bounded thread pool: `--jobs N` (an integer of at least 1),
    by default the smaller of 8 and the CPUs the process may use. Each row keeps its own timeout, its
    verdict and the runner's exit codes (0, 1, 2, 3) are unchanged, and the report lists rows in row
    order whatever order they finish in. The deferred rows still run in a pass of their own after the
    others. `--jobs 1` runs one row at a time.
R8. A pull request's runs share one concurrency group, its ref, and a newer run cancels the one it
    supersedes. Every push to `dev` or `main` takes a group of its own (its run id), so it is never
    cancelled, and never replaced while it waits. No job sets a concurrency group of its own.
R9. The `web` job restores Playwright's browsers (`~/.cache/ms-playwright`) under the key
    `playwright-<runner os>-<the Playwright version pnpm-lock.yaml locks>-chromium-headless-shell`.
    A step reads that version from the lockfile and fails when the lockfile locks none, so the key
    is never built from nothing. `playwright install chromium --only-shell` runs only on a miss, and
    the cache is saved under R2. `--with-deps` is dropped only once a run without it has passed the
    end-to-end tests on `ubuntu-24.04` (section 7 records it).
R10. `check.sh` writes `timings.tsv` in its log directory: a header `stage seconds verdict exit`, then
    one row per stage in the order it ran, appended as each stage ends.
R11. Every gate job uploads its stage logs, `timings.tsv` among them, with `if: always()`, from a
    directory outside the checkout that `actions/upload-artifact` does not skip as hidden
    (`${{ runner.temp }}/check-logs`), one artifact per job.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the Rust cache is restored before the Rust stages under a key of the toolchain pin and the lockfile, and saved from the same paths after the workspace's own artifacts are cleaned | `test_ci_workflows.py` `the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile` |
| A2 | every cache save runs only on a push to dev or main that missed its key, and no step saves a cache by itself; planted defects are refused | `test_ci_workflows.py` `a_cache_is_saved_only_by_a_push_to_dev_or_main` |
| A3 | the stages run in the four jobs the owner named, which need nothing | `test_ci_workflows.py` `the_gate_runs_in_four_parallel_jobs` |
| A4 | every stage check.sh defines runs in exactly one CI job; a dropped or doubled stage is refused | `test_ci_workflows.py` `every_stage_runs_in_exactly_one_ci_job` |
| A5 | every stage fails by name, with an install hint, without each tool it runs | `test_check_gate.py` `every_stage_fails_by_name_without_each_tool_it_runs` |
| A6 | the gate has no toolchain stage, and its audit is split by toolchain | `test_check_gate.py` `the_gate_has_no_toolchain_stage_and_splits_the_audit` |
| A7 | the jobs that read history fetch all of it | `test_ci_workflows.py` `the_jobs_that_read_history_fetch_all_of_it` |
| A8 | a parallel run gives the serial run's verdicts, exit and row order, timeouts included | `test_pack_wiring.py` `a_parallel_run_gives_the_serial_verdicts_in_row_order` |
| A9 | the pool runs at most its bound of rows at once, and reaches it | `test_pack_wiring.py` `the_pool_runs_at_most_its_bound_of_rows_at_once` |
| A10 | only a superseded pull-request run is cancelled; two pushes never share a group | `test_ci_workflows.py` `only_a_superseded_pull_request_run_is_cancelled` |
| A11 | the browser cache is keyed on the Playwright version the lockfile locks, and the browser is installed on a miss | `test_ci_workflows.py` `the_browser_cache_is_keyed_on_the_locked_playwright_version` |
| A12 | each stage run is timed in timings.tsv with its verdict and exit | `test_check_gate.py` `each_stage_run_is_timed_in_timings_tsv` |
| A13 | every gate job uploads its stage logs from a directory the upload does not skip | `test_ci_workflows.py` `every_job_uploads_its_stage_logs_from_a_visible_directory` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k a_cache_is_saved_only_by_a_push_to_dev_or_main
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_gate_runs_in_four_parallel_jobs
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k every_stage_runs_in_exactly_one_ci_job
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k every_stage_fails_by_name_without_each_tool_it_runs
A6: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k the_gate_has_no_toolchain_stage_and_splits_the_audit
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_jobs_that_read_history_fetch_all_of_it
A8: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k a_parallel_run_gives_the_serial_verdicts_in_row_order
A9: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k the_pool_runs_at_most_its_bound_of_rows_at_once
A10: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k only_a_superseded_pull_request_run_is_cancelled
A11: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_browser_cache_is_keyed_on_the_locked_playwright_version
A12: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k each_stage_run_is_timed_in_timings_tsv
A13: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k every_job_uploads_its_stage_logs_from_a_visible_directory
```

The workflow tests read `.github/workflows/*.yml` with a small reader of the block YAML the
workflows use, without a YAML library, as the existing tests do. A2, A10 and A11 judge a condition
by evaluating the workflow's own expression under GitHub's rules (`&&` and `||` return an operand,
strings compare without case, a missing step output is null), in scenarios the test states: a pull
request into `dev` or `main`, a fork's pull request, a push to `dev` or `main`, a cache hit and a
miss. A11 runs the workflow's own version step over the repository's `pnpm-lock.yaml` and over two
planted lockfiles.

A5 and A12 run `check.sh` with a `PATH` that holds only the shell tools `check.sh` needs and a stub
that exits 0 for each tool the case provides, so they run no cargo, pnpm or gitleaks. A8 and A9 run
the runner over a copy of `.packs/` and `scripts/` with a synthetic pack, as SPEC-030's A6 to A8 do.
A9's rows are probes that register themselves in a directory while they run and record the most
they ever saw running at once.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | repo | changed: R1 to R3, R6, R8, R9, R11 |
| `scripts/check.sh` | repo | changed: R4, R5, R10 |
| `scripts/pack-rows.py` | repo | changed: R7 |
| `scripts/tests/test_ci_workflows.py` | repo | changed: A1 to A4, A7, A10, A11, A13; SPEC-002's A9 reads hyphenated stage names |
| `scripts/tests/test_check_gate.py` | repo | added: A5, A6, A12 |
| `scripts/tests/test_pack_wiring.py` | repo | changed: A8, A9 |
| `docs/TESTING.md` | repo | changed: how CI runs the gate, the caches, the pool and `timings.tsv` |
| `deny.toml` | repo | changed: its comment names the `audit-rust` stage |
| `docs/decisions/ADR-055-the-gate-runs-in-parallel-jobs-and-only-a-push-saves-a-cache.md` | repo | added |
| `docs/schematics/ci-jobs-and-caches.md` | repo | added: the jobs, the caches and who reads and writes them |
| `docs/red-first/SPEC-038.md` | repo | added |
| `changelog.d/feat-ci-speed-038.md` | repo | added |

## 5. What this does NOT do

- It skips no job by path: every job runs on every pull request, a docs-only one included, by the
  owner's scope for this package (#207).
- It caches no Rust toolchain: `rustup show` installs the pinned toolchain in each Rust job, in
  about 9 s (#207).
- It adds no cache to `engine-measure.yml`, whose cold build with no cache restored is the
  measurement it exists for (#15).
- It shards no pack rows across jobs: the pool inside one job does it without paying a second job's
  start-up (#207).
- It changes no required context and no ruleset: `ci` and `fragment` stay the required checks, and
  `ci` still needs every job (#207).
- It does not measure the warm runs. Only a push to `dev` saves a cache, so the warm numbers exist
  only after this merges, and the orchestrator measures them (#207).

## 6. Risks

- **A poisoned cache.** Only a push to `dev` or `main`, which carries reviewed code, saves; a pull
  request restores what `dev` saved and can write nothing (A2). GitHub also confines a pull
  request's own caches to its merge ref.
- **A stale cache.** The Rust key moves with the toolchain pin and the lockfile. A fallback entry
  holds an older lockfile's dependencies, and Cargo rebuilds whatever its fingerprints reject, so a
  stale entry costs time, never a wrong verdict.
- **The cache budget.** The repository's caches share 10 GB, evicted oldest first. One Rust entry
  per toolchain pin and lockfile, pruned to the dependencies, stays small beside it; section 7
  records its size.
- **A pack verdict that changes under the pool.** A8 compares a parallel run with a serial one on
  every verdict kind, a timeout included. Each row keeps its own timeout, 30 to 300 s, against rows
  that take under 7 s.
- **A stage that drops out of CI.** A4 refuses a stage in no job or in two; A3 pins the owner's
  layout.
- **A browser without its system libraries.** Section 7 records the run that passed the end-to-end
  tests without `--with-deps`. If a later runner image drops a library, the browser fails to launch
  and the web stage fails loudly.
- **Four jobs pay four start-ups.** Each checks out and sets up on its own; section 7 records each
  job's time beside the wall time.

## 7. Measurements

Recorded by the delivery: the CI runs before and after, per job and in wall time, for a code pull
request, a docs-only pull request and a push to `dev`, with each cache's hit or miss; the run that
passed the end-to-end tests without `--with-deps`; and one run of the local gate's `timings.tsv`.

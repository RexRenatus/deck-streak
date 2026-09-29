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
- **A guard test compiles Rust.** SPEC-042's `scripts/tests/test_vault_rails_rows.py`, merged into
  `dev` while this was built, runs `cargo run -p deck-streak-vault --example rails_verdicts` from the
  python stage. In CI that put a toolchain install and a cold vault build into `hygiene`: its python
  stage took 57 s, twice (run 36362357632 and its re-run), against 11 to 16 s before it (runs
  36361746236 and 36362157262). Amended in by the delivery, with R1 and R4.
- **The gate itself, on the maintainer's machine** (`bash scripts/check.sh` at `dev` 8f91667,
  CHECK OK in 157 s): toolchain 0 s, fmt 0 s, clippy 17 s, test 17 s, doctest 1 s, web 35 s,
  python 8 s, packs 69 s, scrub 8 s, audit 2 s, secrets 0 s.

**Order.** PR #204 (SPEC-022, Anki's engine) edited `ci.yml` and `check.sh` too: it added a
checksum-pinned `protoc` 31.1 step to the gate job and a `protoc` check to the `toolchain` stage,
because the engine's build scripts compile its protobuf definitions (ADR-022). It landed in `dev` as
b1ce32d while this was built. This delivery merges `dev` in, carries the `protoc` step into every job
that compiles Rust and the `protoc` check into the stages that compile the engine (R4, R12), leaves
`engine-measure.yml` as #204 made it, and measures the gate with the engine in the workspace.
ADR-022 records a finding that bears on the cache: the engine's `anki_proto` build script registers
every file it generates as a rerun input and rewrites some of them on each run, so each cargo
command that builds the engine compiles it again. Section 7 measures what that costs.

## 2. Requirements

R1. The `rust` job restores `~/.cargo/registry/index/`, `~/.cargo/registry/cache/`,
    `~/.cargo/git/db/` and `target/` with `actions/cache/restore`, pinned by its full commit SHA,
    under the key `rust-<runner os>-<hashFiles('rust-toolchain.toml')>-<hashFiles('Cargo.lock')>`,
    falling back to the newest entry of the same toolchain pin. After its stages it saves the same
    paths under the same key with `actions/cache/save`, only when R2 allows it, and only after
    `cargo clean --workspace` has left the dependencies' artifacts alone in `target/`. It sets
    `CARGO_INCREMENTAL=0`, because incremental state is rebuilt from a fresh checkout anyway. The
    `hygiene` job, whose python stage builds a Rust example, installs the pinned toolchain and
    restores the same cache under the same key, and never saves it: only the `rust` job, which
    builds every target, saves (amended by the delivery, after SPEC-042's test).
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
    `audit-web`; `python3` (3.11 or later) then `cargo` for `python`, whose guard tests build a Rust
    example; `python3` for `packs` and `scrub`; `gitleaks` for `secrets`. The stages that compile
    Anki's engine (`clippy`, `test` and `doctest`) then check `protoc`, which the engine's build
    runs from `PROTOC` when it is set, which must then name an executable, and from `PATH`
    otherwise (ADR-022). A missing tool fails that stage by name, locally and in CI. The
    `toolchain` stage is removed.
    Amended by SPEC-058 (section 10): `audit-web` then checks `python3` (3.11 or later), which runs
    the verdict that reads pnpm's report.
R5. The `audit` stage is split: `audit-rust` runs
    `cargo deny --locked check advisories bans licenses sources` in the `rust` job, and `audit-web`
    runs `pnpm audit --prod` in the `web` job. Amended by SPEC-058 (section 10): the audit covers
    the development dependencies too, and refuses a run that examined nothing. `audit-web` runs
    `pnpm audit --json --audit-level low`, which names no dependency class, and its verdict is read
    from pnpm's report.
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
R12. Every CI job that compiles Rust (`rust`, and `hygiene`, whose guard tests build a Rust
    example) installs `protoc` 31.1 before its stages from the release archive, checked against the
    digest ADR-022 pins, and puts it on `PATH`, as #204's gate job did. Every workflow that pins
    `protoc` pins that digest, `engine-measure.yml` included, and that workflow is unchanged: its
    cold build restores no cache, on purpose.
R13. The engine set is one nextest filterset, `ENGINE_TESTS` in `scripts/check.sh`, stated nowhere
    else: SPEC-022's two test binaries that drive Anki's engine end to end,
    `binary_id(=deck-streak-ingest::sync) | binary_id(=deck-streak-ingest::engine_budget)`. The
    `test` stage runs `cargo nextest run --workspace --locked --no-fail-fast -E 'not (<the set>)'`,
    and a new `test-engine` stage runs the same command with `-E '<the set>'`. The two commands
    differ only in the filterset, so every test of the workspace runs in exactly one of them, under
    one build scope and one feature resolution. `test-engine` checks `cargo`, `cargo-nextest` and
    then `protoc`, as `test` does (R4), and `check.sh` with no arguments runs both (section 8).
R14. A fifth gate job, `engine`, runs `bash scripts/check.sh test-engine` beside R3's four. It needs
    nothing and has no job-level condition, so it starts with them on every event. It installs the
    pinned toolchain, `cargo-nextest` and R12's `protoc` before its stage, restores the Rust cache
    under R1's key with `actions/cache/restore` and never saves it (only `rust` saves: two jobs
    saving one key race, and `rust` builds every target), sets `CARGO_INCREMENTAL=0`, uploads its
    stage logs under R11, and has a `timeout-minutes` sized from section 8's measurements. The
    aggregate `ci` needs it beside R3's needs, so the engine's tests stay required (section 8).
R15. The target of a warm code pull request is revised in writing: the engine set's run bounds its
    critical path, which is the `engine` job. Section 8 states that path and the `rust` job's, each
    as its measured components, and holds them to it, with the gate measured from the first job's
    start to the end of `ci`, excluding each job's wait for a runner. A change that lengthens
    either past its bound says so in its pull request, with the run that measured it.
R16. The `engine` job runs the engine set in two slices, one per runner, from a matrix whose one
    dimension counts the slices from 1 to N (`slice: [1, 2]`). Each leg hands `test-engine` its own
    slice as `ENGINE_SLICE=<m>/<N>`, N being the matrix's size (`strategy.job-total`), and
    `test-engine` passes it to nextest as `--partition slice:<m>/<N>`. That is nextest's
    cross-binary round robin over the one list the filterset selects, so the slices hold every test
    of the set once. Without `ENGINE_SLICE`, as in the local gate, `test-engine` runs the whole set.
    A value that names no slice m/n with 1 <= m <= n fails the stage by name before any build.
    `fail-fast` is false, so each slice runs to its end. Section 8's measurement chose two slices.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every job that compiles Rust installs the pinned toolchain and restores the Rust cache before its stages, under a key of the toolchain pin and the lockfile; one job, the one that builds every target, saves it after the workspace's own artifacts are cleaned | `test_ci_workflows.py` `the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile` |
| A2 | every cache save runs only on a push to dev or main that missed its key, and no step saves a cache by itself; planted defects are refused | `test_ci_workflows.py` `a_cache_is_saved_only_by_a_push_to_dev_or_main` |
| A3 | the stages run in the four jobs the owner named, which need nothing | `test_ci_workflows.py` `the_gate_runs_in_four_parallel_jobs` |
| A4 | every stage check.sh defines runs in exactly one CI job; a dropped or doubled stage is refused | `test_ci_workflows.py` `every_stage_runs_in_exactly_one_ci_job` |
| A5 | every stage fails by name, with an install hint, without each tool it runs | `test_check_gate.py` `every_stage_fails_by_name_without_each_tool_it_runs` |
| A6 | the gate has no toolchain stage, and its audit is split by toolchain | `test_check_gate.py` `the_gate_has_no_toolchain_stage_and_splits_the_audit` |
| A7 | the jobs that read history fetch all of it | `test_ci_workflows.py` `the_jobs_that_read_history_fetch_all_of_it` |
| ~~A8~~ | a parallel run gives the serial run's verdicts, exit and row order, timeouts included | `test_pack_wiring.py` `a_parallel_run_gives_the_serial_verdicts_in_row_order` |
| ~~A9~~ | the pool runs at most its bound of rows at once, and reaches it | `test_pack_wiring.py` `the_pool_runs_at_most_its_bound_of_rows_at_once` |
| A10 | only a superseded pull-request run is cancelled; two pushes never share a group | `test_ci_workflows.py` `only_a_superseded_pull_request_run_is_cancelled` |
| A11 | the browser cache is keyed on the Playwright version the lockfile locks, and the browser is installed on a miss | `test_ci_workflows.py` `the_browser_cache_is_keyed_on_the_locked_playwright_version` |
| A12 | each stage run is timed in timings.tsv with its verdict and exit | `test_check_gate.py` `each_stage_run_is_timed_in_timings_tsv` |
| A13 | every gate job uploads its stage logs from a directory the upload does not skip | `test_ci_workflows.py` `every_job_uploads_its_stage_logs_from_a_visible_directory` |
| A14 | every job that compiles Rust installs protoc 31.1, checked against ADR-022's digest, before its stages, and every workflow pins that digest | `test_ci_workflows.py` `every_job_that_compiles_rust_installs_the_pinned_protoc_first` |
| A15 | the stages that compile the engine fail by name without protoc, and honour a `PROTOC` that names an executable | `test_check_gate.py` `the_stages_that_compile_the_engine_need_protoc` |
| A16 | the engine set is defined once, `test` runs its complement and `test-engine` runs it, and their commands differ only in the filterset; a stage holding its own copy of the set, a `test` that runs the set itself, a second build scope and a second definition are refused | `test_check_gate.py` `both_test_stages_take_the_engine_set_from_its_one_definition` |
| A17 | the engine set is a union of whole test binaries, each a test target of a workspace crate that holds tests; a set naming nothing, a missing target and an unknown package are refused | `test_check_gate.py` `the_engine_set_names_test_binaries_that_hold_tests` |
| A18 | the `engine` job runs `test-engine` alone, starts with the other jobs on every event and is a need of `ci`; it installs `cargo-nextest` before its stage, restores the Rust cache and saves none, builds without incremental state, and has a timeout in its measured band; a planted job that breaks each rule is refused | `test_ci_workflows.py` `the_engine_job_runs_the_engine_set_beside_the_rust_job` |
| A19 | `test-engine` hands nextest the slice it is given, adding only `--partition slice:<m>/<n>` to its command; given none, it runs the whole set; given a value that names no slice, it fails by name and builds nothing | `test_check_gate.py` `the_engine_stage_runs_the_slice_it_is_given` |
| A20 | the `engine` job's matrix counts its slices from 1 to N once each, N at least 2, each leg hands `test-engine` its own slice of the matrix's size, and no leg's failure cancels another; a slice run twice, a leg that names the wrong count and a cancelling matrix are refused | `test_ci_workflows.py` `the_engine_job_runs_each_slice_of_the_engine_set_once` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k a_cache_is_saved_only_by_a_push_to_dev_or_main
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_gate_runs_in_four_parallel_jobs
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k every_stage_runs_in_exactly_one_ci_job
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k every_stage_fails_by_name_without_each_tool_it_runs
A6: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k the_gate_has_no_toolchain_stage_and_splits_the_audit
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_jobs_that_read_history_fetch_all_of_it
```
```retired
A8: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k a_parallel_run_gives_the_serial_verdicts_in_row_order
A9: python3 -m unittest discover -s scripts/tests -p test_pack_wiring.py -k the_pool_runs_at_most_its_bound_of_rows_at_once
```
```acceptance
A10: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k only_a_superseded_pull_request_run_is_cancelled
A11: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_browser_cache_is_keyed_on_the_locked_playwright_version
A12: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k each_stage_run_is_timed_in_timings_tsv
A13: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k every_job_uploads_its_stage_logs_from_a_visible_directory
A14: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k every_job_that_compiles_rust_installs_the_pinned_protoc_first
A15: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k the_stages_that_compile_the_engine_need_protoc
A16: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k both_test_stages_take_the_engine_set_from_its_one_definition
A17: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k the_engine_set_names_test_binaries_that_hold_tests
A18: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_engine_job_runs_the_engine_set_beside_the_rust_job
A19: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k the_engine_stage_runs_the_slice_it_is_given
A20: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_engine_job_runs_each_slice_of_the_engine_set_once
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

A16 runs `check.sh`'s `test` and `test-engine` stages the same way, with a `cargo` that records the
arguments it is given, twice: once as written, and once with a planted set written in the
definition's place. A stage that states the set itself, rather than reading the one definition,
keeps the old set in the second run and is refused. A17 resolves each binary the set names to its
crate's `tests/<target>.rs` and counts the tests there. A19 runs `test-engine` with and without
`ENGINE_SLICE`, over slices and over values that name none, and A20 judges the `engine` job's
matrix and each leg's slice beside a planted matrix that runs a slice twice. Since the amendment,
the populations of A1, A3 to A6 and A13 to A15 include the new stage and the new job: A3's layout
holds five gate jobs, A5 and A15 judge `test-engine`'s tools and `protoc`, and A1, A13 and A14
judge the `engine` job's toolchain, cache, logs and `protoc` like any job that compiles Rust. They
grow through the tables those tests read, each extended by one entry and no assertion changed:
`OWNER_LAYOUT` and `COMPILES_RUST` in `test_ci_workflows.py`, and `TOOLS` and `ENGINE_STAGES` in
`test_check_gate.py`.

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
| `.github/workflows/ci.yml` | repo | changed by the amendment (section 8): R14 and R16, the `engine` job, its two slices and its `ci` need |
| `scripts/check.sh` | repo | changed by the amendment: R13 and R16, the engine set, the `test` stage's filterset and the `test-engine` stage with its slice |
| `scripts/tests/test_check_gate.py` | repo | changed by the amendment: A16, A17 and A19; A5, A6 and A15 judge `test-engine` too |
| `scripts/tests/test_ci_workflows.py` | repo | changed by the amendment: A18 and A20; A1, A3, A13 and A14 judge the `engine` job too |
| `docs/TESTING.md` | repo | changed by the amendment: the `engine` job and the engine set |
| `docs/schematics/ci-jobs-and-caches.md` | repo | changed by the amendment: the `engine` job, and the engine set split between two stages |
| `docs/decisions/ADR-055-the-gate-runs-in-parallel-jobs-and-only-a-push-saves-a-cache.md` | repo | changed by the amendment: a note at its end (the ADR is accepted, so it is appended to) |
| `docs/red-first/SPEC-038.md` | repo | changed by the amendment: A16 to A20 |
| `changelog.d/feat-ci-engine-038.md` | repo | added by the amendment: its own fragment, beside the delivery's |

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
- The amendment moves no test out of the gate and skips none: `ci` needs the `engine` job, which
  runs on every event, and the local gate runs both test stages (#207).
- The amendment changes none of SPEC-022's tests and builds none of them optimised, so the engine
  set's run stays the floor of a warm pull request; shortening it is a decision about SPEC-022's
  tests (#15).
- The amendment does not stop a cargo command from recompiling the engine. That is ADR-022's
  finding about the engine's build script, and each cargo command that builds the engine still
  pays it: three in `rust`, and one in each `engine` leg (#228).
- The amendment narrows no build: `test-engine` builds the workspace's scope, as `test` does,
  because a build of the ingest package alone resolves other features than the build the cache
  holds, and would recompile what the cache already has (section 8) (#207).
- The amendment runs the engine set in two slices, not three or more: a slice cannot finish
  before its slowest test, 74 to 82 s, and each slice costs another runner (section 8) (#207).

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
- **The engine compiles again in every cargo command.** ADR-022's finding: a warm cache saves every
  dependency but the engine and what depends on it, which `clippy`, `test` and `doctest` each
  compile once. Section 7 measures it; the fix belongs to the engine's build script, not to CI.
- **A slow test outside the engine set.** The set holds whole binaries, so a test added to
  SPEC-022's sync or budget binary goes with it. A slow test added to any other binary runs in
  `rust` and lengthens it, and R15 has its pull request say so.
- **A stale engine set.** A renamed binary would leave the set naming a target that is gone. Its
  tests would still run, in `rust`, because the complement drops none; A17 refuses a set that names
  a missing target, and nextest fails a run that selects no test.
- **The engine job repeats the test build.** `engine` builds the same workspace scope that `rust`'s
  `test` stage builds, one forced engine recompile included. Section 8 measures it and says why a
  narrower build would cost more than it saves.

## 7. Measurements

Recorded by the delivery. Every run after this change is cold: only a push to `dev` saves a cache,
so until this merges every pull request misses the Rust, pnpm and Playwright caches alike.

| case | before: one `gate` job | after: four parallel jobs, every cache missed |
|---|---|---|
| a code pull request | 4m57s, run 36356940848 (`gate` 3m46s) | 1m45s, run 36361746236 (this delivery's own) |
| a docs-only pull request | 2m43s, run 36356754966 (`gate` 2m35s) | 2m00s, run 36361940482, of which 24 s queued behind the run it cancelled; its jobs took 1m35s |
| a push to `dev` | 3m53s, run 36357723675 (`gate` 3m45s) | measured after the merge, whose push saves the caches |

Each job of run 36361746236, and its stages from the job's own `timings.tsv`:

| job | job time | stages |
|---|---|---|
| `rust` | 1m34s | fmt 0 s, clippy 37 s, test 37 s, doctest 1 s, audit-rust 1 s (every dependency compiled) |
| `web` | 60 s | web 31 s, audit-web 0 s; the browser installed in 6 s |
| `packs` | 32 s | packs 25 s, 398 rows at most 4 at once |
| `hygiene` | 22 s | python 11 s, scrub 4 s, secrets 1 s |
| `workflow-lint`, `base-is-dev`, `ci` | 4 s, 3 s, 3 s | |

- **`--with-deps` is dropped (R9).** Run 36361746236's `web` job installed Chromium's headless shell
  without it, in 6 s against 21 to 30 s before, into `~/.cache/ms-playwright`, and its web stage
  passed the end-to-end tests on `ubuntu-24.04` (Playwright: 7 passed).
- **A superseded run is cancelled (R8).** On the throwaway docs-only pull request #213, a second
  docs-only commit cancelled run 36361916181 mid-flight (`rust`, `web`, `packs` and `hygiene`
  cancelled), and run 36361940482 ran to the end.
- **No pull request saved a cache (R2).** GitHub's cache list held no entry for either pull request's
  merge ref after both ran, while the old workflow's `setup-node` cache had saved four from other
  pull requests' merge refs.
- **The local gate**, one run on the maintainer's machine at 4145df2 (`timings.tsv`): fmt 0 s,
  clippy 1 s, test 1 s, doctest 2 s, audit-rust 1 s, web 32 s, audit-web 1 s, packs 15 s, python
  14 s, scrub 9 s, secrets 1 s; CHECK OK in 77 s, against 157 s at `dev` 8f91667. Its Rust stages
  reused that checkout's own build.
- **The pool** on the maintainer's machine, over one tree: `--jobs 1` took 70.8 s and `--jobs 8` (the
  default there) 15.1 s, with all 410 report lines identical; `--jobs 4` took 20.0 s and `--jobs 16`
  13.1 s on another tree, identical to each other. Past 8, the longest row and the deferred pass set
  the time.

### With Anki's engine in the workspace (#204, merged into `dev` as b1ce32d)

| case | before: one `gate` job | after: four parallel jobs, every cache missed |
|---|---|---|
| a code pull request | 7m11s, run 36363375903, #204's last (`gate` 7m04s: clippy 93 s, test 171 s, doctest 19 s) | 7m38s, run 36366758034 at f68bce5 (this delivery's own) |
| a push to `dev` | 9m09s, run 36366072041 at b1ce32d (`gate` 9m00s: clippy 107 s, test 228 s, doctest 23 s) | measured after the merge |
| a docs-only pull request | no docs-only run went through the old job with the engine in; with no path filter and no cache its time never depended on the change, so it cost what a code pull request did | measured after the merge |

Each job of run 36366758034, and its stages from the job's own `timings.tsv`:

| job | job time | stages |
|---|---|---|
| `rust` | 7m25s | fmt 1 s, clippy 136 s, test 259 s (the build 118 s, the run 141 s), doctest 27 s, audit-rust 4 s |
| `hygiene` | 1m57s | python 89 s (the vault example built cold), scrub 9 s, secrets 1 s |
| `web` | 56 s | web 31 s, audit-web 1 s |
| `packs` | 26 s | packs 19 s |

A second cold sample, the `rust` job of the same run re-run (attempt 2): 7m21s, with clippy 126 s,
test 264 s (the build 124 s, the run 140 s), doctest 29 s. Both samples ran about 30% slower than
the old job's Rust stages in every stage, `doctest` included, whose cost is recompiling `anki`, a git
dependency that `CARGO_INCREMENTAL` never touches; the runners, not the split, account for it. Cold,
the split is no faster with the engine in: its gain is the warm run.

- **The engine compiles again in every cargo command, measured.** With nothing changed, `clippy`,
  `test` and `doctest` each recompiled the same 14 units: `anki_proto`, `anki`, `deck-streak-ingest`
  and the 11 workspace crates that depend on it. On the maintainer's machine, run unchanged at
  f68bce5, they took 17 s, 151 s and 62 s, against 36 s, 158 s and 53 s the first time. In CI, the
  `doctest` stage of run 36366758034 recompiled exactly those 14 after nextest had built everything,
  in 25.99 s: the price of ADR-022's finding for each cargo command on a 4-CPU runner.
- **The engine's own tests set a floor.** Run 36366758034 ran 102 tests in 141 s; the slowest are
  SPEC-022's sync and budget tests, at 81, 72, 66, 60, 41 and 39 s. A warm cache removes the
  dependencies' compile, but neither that run nor the engine's recompile in each of the three
  commands. Estimated from those measurements, a warm code pull request's `rust` job takes about 4
  to 4.5 minutes, so the target of about a minute and a half cannot be met while these tests run in
  the gate in a debug build. Moving them, or building the engine optimised for tests, is a decision
  about SPEC-022's tests, which this delivery leaves to the owner.
- **The local gate with the engine**, one run on the maintainer's machine at f68bce5
  (`timings.tsv`): fmt 0 s, clippy 36 s, test 158 s, doctest 53 s, audit-rust 3 s, web 54 s,
  audit-web 1 s, packs 18 s, python 38 s, scrub 11 s, secrets 1 s; CHECK OK in 373 s.
- **Still to measure, after this merges** (the orchestrator): the merge's own push to `dev` (every
  cache missed, then saved; the save step names the Rust entry's size), a code pull request and a
  docs-only pull request that restore them, and a later push to `dev` with the same lockfile (an
  exact hit that saves nothing).

## 8. Amendment, 2026-09-28: the engine's slow tests run in a job of their own

Made after the delivery, under the owner's delegation, on issue #207. It inserts, and changes no
earlier byte:

- section 2: R13 to R16, after R12;
- section 3: rows A16 to A20 of the criteria table, lines A16 to A20 of the acceptance fence, and
  the paragraph that begins "A16 runs";
- section 4: the nine rows marked "by the amendment";
- section 5: the five bullets that begin "The amendment";
- section 6: the three risks "A slow test outside the engine set", "A stale engine set" and "The
  engine job repeats the test build";
- this section.

The title and R3 still say four jobs: the layout A3 pins now holds five gate jobs, `engine` among
them. ADR-055 carries a note at its end. Two rulings on the amendment's form, made by the
orchestrator under the owner's delegation:

- (i) A delivered SPEC may be amended insert-only: every existing byte kept in order, and each insertion listed in the dated amendment section. A byte-wise append is impossible, because the sdd probe's acceptance-fenced class reads the criteria from section 3 only.
- (ii) Growing `TOOLS`, `COMPILES_RUST` and `ENGINE_STAGES` is accepted: they extend tables the amendment needs, no assertion changed, and each is disclosed (section 3's paragraph that begins "A16 runs", and the red-first record).

### The gate, as this section measures it

The gate is measured from the first job's start to the end of `ci`, excluding each job's wait for a
runner, which this workflow does not control (1 to 144 s in the runs below). The gate jobs need
nothing and `ci` needs them all, so the gate is the longest gate job's run time plus `ci`'s own.
Every gate time in this section is that one, taken from the jobs' start and end times.

### The problem, measured

Section 7 left the warm numbers to be measured. `dev`'s caches were saved by run 36369172368, the
push of #212 at `dev` 63e6671, whose Rust entry is 1,005.3 MB (1,005,277,540 bytes). A code pull
request that restored them by exact key still spent most of its `rust` job in one nextest run:

| run | what it is | `rust` job, and the gate | stages (`timings.tsv`) |
|---|---|---|---|
| 36368711222, attempt 2 | #212's last pull-request run, its `rust` job re-run: its merge commit holds `dev` 63e6671's tree exactly | 4m19s; only `rust` re-ran, so no gate | fmt 0 s, clippy 16 s, test 186 s (the build 45.7 s, the run 139.4 s), doctest 28 s, audit-rust 3 s |
| 36371239856 | this amendment's red commit, whose workflow was still the single `rust` job | 4m22s; the gate 4m26s | fmt 1 s, clippy 17 s, test 186 s (the build 46.9 s, the run 139.3 s), doctest 28 s, audit-rust 4 s |

A push to `dev` in that layout came after #224 had changed `Cargo.lock`: run 36372781643, at `dev`
c0dbf2a, restored the 63e6671 entry by its fallback key. Its `rust` job took 4m44s (test 194 s: the
build 54.8 s, 145 tests in 139.0 s) and its gate 4m46s, and it saved a new entry of 1,005.2 MB
(1,005,227,749 bytes) under the new key.

The run of 126 tests took 139 s, and ten of them set it: SPEC-022's `sync` and `engine_budget`
binaries. The other 116 took 5.8 s together, none over 1.5 s. It was decided, under the owner's
delegation: "A parallel `engine` job runs SPEC-022's slow sync and budget tests, and `rust` runs
the rest. Every test stays required: `ci` needs both." And: "revise the target in writing: the warm
critical path is bounded by the engine floor."

### The engine set (R13), chosen by the measured runtimes

| test | binary | seconds, one job (36368711222 attempt 2) |
|---|---|---|
| `an_incremental_sync_of_one_hundred_new_reviews_stays_inside_the_time_budget` | `engine_budget` | 80.8 |
| `a_full_sync_demand_downloads_and_never_uploads` | `sync` | 73.5 |
| `a_sync_run_sends_no_upload_and_no_local_change` | `sync` | 64.4 |
| `a_full_download_of_the_large_synthetic_collection_stays_inside_the_memory_budget` | `engine_budget` | 59.6 |
| `a_second_sync_with_no_change_pulls_nothing` | `sync` | 44.0 |
| `a_sync_pulls_a_review_made_on_another_client` | `sync` | 40.1 |
| `a_server_with_no_collection_is_refused_with_full_upload_required` | `sync` | 33.0 |
| `opening_a_large_synthetic_collection_and_its_new_card_queue_stays_inside_the_memory_budget` | `engine_budget` | 20.4 |
| `a_second_scheduled_sync_in_one_study_day_is_refused`, `an_owner_trigger_within_five_minutes_of_a_success_returns_it_without_syncing` | `sync` | under 0.1 each |
| the other 116 tests of the workspace, in 61 binaries | | 5.8 together, at most 1.5 |

The set holds the two binaries whole. Each of their slow tests builds a large synthetic collection
or starts the engine's sync server, so a test added to either binary is likely to be slow as well,
and it goes with the binary; a renamed test cannot slip out. The two fast tests ride along at no
cost. The rest of the ingest package stays in `test`, because its tests take under a second.

`test-engine` builds the workspace's scope, as `test` does. Cargo unifies features over the packages
it builds, so an ingest-only build resolves `tokio` and what depends on it differently from the
`--workspace` build the cache holds. Measured on the maintainer's machine, in a target warm from the
workspace's own test build: `cargo nextest run -p deck-streak-ingest --no-run` recompiled 22
dependencies (`anki` and `anki_proto`, `burn` and `fsrs`, `prost`, `reqwest`, `axum`, `tower`
among them) and took 58.6 s, where `test-engine`'s own build of the whole workspace took 46.1 s.

### Two slices (R16), measured

Sharding lowers the floor only as far as the slowest test allows, so it was measured: one `engine`
job at c8b1d3e, then two slices at 25b102c, each run twice (the second attempt a re-run), and the
two slices five more times on this branch's later heads. A leg's time is its job's run time; its
`test-engine` stage is given with the stage's test build and its tests' run.

| run | layout | the slower `engine` leg, then the other | `rust` | the gate |
|---|---|---|---|---|
| 36371536468, attempt 1 | one engine job | 3m39s: test-engine 194 s (the build 48.6 s, the run 141.3 s) | 2m17s | 3m41s |
| 36371536468, attempt 2 | one engine job | 3m38s: test-engine 193 s (the build 46.3 s, the run 142.0 s) | 2m01s | 3m41s |
| 36372201275, attempt 1 | two slices | slice 1 2m32s: test-engine 128 s (the build 47.3 s, the run 75.3 s); slice 2 1m59s: 83 s (32.9 s, 44.2 s) | 1m52s | 2m35s |
| 36372201275, attempt 2 | two slices | slice 1 2m32s: 126 s (46.7 s, 75.3 s); slice 2 2m24s: 118 s (45.8 s, 67.8 s) | 2m04s | 2m36s |
| 36373161165 | two slices, `dev` c0dbf2a merged in, restored by the fallback key | slice 1 2m38s: 132 s (52.8 s, 74.4 s); slice 2 2m36s: 117 s (53.7 s, 60.2 s) | 2m22s | 2m41s |
| 36373786208 | two slices at f607dcd, restoring c0dbf2a's entry by exact key | slice 1 2m46s: 133 s (55.0 s, 74.3 s); slice 2 2m40s: 127 s (53.9 s, 69.1 s) | 2m19s | 2m48s |
| 36374302129 | two slices at 2bc6350, the same key | slice 2 3m07s: 152 s (67 s, 80.6 s); slice 1 2m51s: 137 s (56.6 s, 76.2 s) | 2m17s | 3m09s |
| 36374807751 | two slices at c12bc7e, the same key | slice 1 2m49s: 140 s (54.5 s, 82.2 s); slice 2 2m39s: 126 s (52.8 s, 68.7 s) | 2m15s | 2m51s; 3m37s with the gate jobs' waits for a runner in it, and 3m50s with `ci`'s 13 s wait too |
| 36379817026 | two slices at 094260f, `dev` 3f5470e merged in, restored by the fallback key | slice 1 2m57s: 142 s (62 s, 76.0 s); slice 2 2m43s: 136 s (62 s, 69.0 s) | 2m31s | 3m00s |

Each slice ran five tests. Slice 1's run, 74.3 to 82.2 s in the seven sliced runs, is the time of
its slowest test, `a_sync_run_sends_no_upload_and_no_local_change`. Two slices took 66 s and 65 s
off the gate in the two pairs of runs of one tree (3m41s to 2m35s and 2m36s), so the engine set
runs in two. A third slice would still wait for that test, and would cost another runner for a
gain the two samples do not show.

The later runs build a larger workspace, because both test stages build all of it. With #224 merged
in (c0dbf2a), an `engine` leg's test build took 52.8 to 56.6 s, once 67 s on a slower runner,
against 32.9 to 48.6 s at 63e6671; with `dev` 3f5470e merged in, 62 s. #224 also brought two tests
(`deck-streak-coordination::ledger`) of 0.3 to 13.6 s each, outside the engine set, which run in
`rust`: its 135 tests took 2.1 to 14.5 s, and at 3f5470e its 163 tests 5.5 s.

The price is runner time. Each slice pays its own setup, restore and test build, so in run
36372201275's two attempts the Rust jobs took 6m23s and 7m00s of runner time together (`rust` and
both slices), against 4m22s for the single `rust` job before the amendment, for a gate 1m51s and
1m50s shorter.

The `rust` job's stages in the first four runs, at 63e6671's workspace: fmt 0 to 1 s, clippy 14 to
16 s, test 44 to 51 s (the build 30.3 to 45.8 s, the 116 tests 2.0 to 12.9 s), doctest 18 to 27 s,
audit-rust 2 to 4 s; its restore took 14 to 25 s, and each `engine` leg's 11 to 22 s. On the
maintainer's machine, run at c8b1d3e with its own build: fmt 1 s, clippy 48 s, test 91 s (116
tests), doctest 85 s, audit-rust 6 s, test-engine 150 s (the build 46 s, the run of the whole set
103 s).

### The revised target (R15)

The warm code pull request's target of about a minute and a half is revised. The engine set's run
bounds it: its slowest test takes 74 to 82 s in a slice, after a test build that recompiles the
engine once (ADR-022's finding, #228), 33 to 67 s with the workspace. Measured over the seven
sliced runs, the warm critical path is the slower `engine` leg:

- **an `engine` leg:** 13 to 18 s of setup and upload (the checkout, `rustup show` 7 to 10 s,
  `cargo-nextest`, `protoc`, the stage logs), the restore 11 to 22 s, and `test-engine` 83 to 152 s
  (the workspace's test build 33 to 67 s, one forced engine recompile among it, and the slice's run
  44 to 82 s). The slower leg took 2m32s, 2m32s, 2m38s, 2m46s, 3m07s, 2m49s and 2m57s: about 2m46s,
  **held to 3m15s**.
- **the `rust` job:** 14 to 20 s of setup and upload, the restore 11 to 25 s, and four cargo
  commands, three of them recompiling the engine: clippy 14 to 17 s, test 44 to 65 s, doctest 18 to
  31 s, audit-rust 2 to 4 s. In the nine runs since the split it took 1m52s to 2m31s: about 2m13s,
  **held to 2m30s**.
- **the gate:** the slower of the two, then the aggregate `ci`, which runs in 2 to 4 s. It took
  2m35s, 2m36s, 2m41s, 2m48s, 3m09s, 2m51s and 3m00s, against 4m26s before the amendment and 3m41s
  with one engine job: about 2m49s, **held to 3m20s**.

One run broke a bound, and R15 has it said here: in run 36379817026, with `dev` 3f5470e merged in
and the cache restored by its fallback key, `rust` took 2m31s, 1 s past its 2m30s. Its test build
took 59.0 s for 163 tests in 72 binaries, against 30.3 to 55.0 s before; the gate held at 3m00s,
because the slower `engine` leg is the critical path. The bound is kept, and this run is recorded
against it.

The engine legs and `rust` build the whole workspace, so their builds grow as it does; a delivery
that takes a job past its bound says so (R15), and the fix is then the test build, not the split.

These bounds are for the gate's five jobs. SPEC-039's mutation jobs, when they are needs of `ci`,
state their own.

The `engine` job's `timeout-minutes` is 20, sized from these runs: a warm leg took 1m59s to 3m07s,
and a cold one repeats the test stage's cold build (102 s and 126 s in runs 36368711222 and
36369172368) before a slice of at most 82 s, about 4 minutes. Twenty minutes holds five cold legs,
and a hung sync test is stopped in a twentieth of GitHub's default six hours.

### What remains for the orchestrator

- The merge's own push to `dev`. This pull request leaves `Cargo.lock` as `dev` has it, so the push
  restores the entry `dev`'s own push saved for that lockfile (1,005.2 MB, 1,005,229,631 bytes) by
  exact key, and saves nothing: the first warm push measured in five jobs.
- A cold run of this layout. A change to `Cargo.lock` still restores the newest entry of the same
  toolchain, so only a new toolchain pin or an evicted entry runs cold, and its first push to `dev`
  saves the new key.
- A docs-only pull request, which does the same work as a code pull request, since no job is
  skipped by path (section 5).

## 9. Amendment, 2026-09-28: criteria whose tests SPEC-056 removed

Made by SPEC-056 (ADR-069), insert-only under ruling (i) of SPEC-038 section 8: every earlier byte
is kept in order. It inserts:

- section 3: `~~` around A8 and A9 in the criteria table, so the table no longer states them;
- section 3: the fence lines that set A8 and A9 apart in a `` ```retired `` fence between A7 and
  A10, splitting the acceptance fence where their lines stood;
- this section.

The retired criteria, why their subject is gone, and what judges it now:

- A8 and A9 (the row runner's pool gives the serial run's verdicts, and keeps its bound): SPEC-056
  removed the row runner and its tests (SPEC-056 A1). The box run runs each pack's rows through the
  packs' own binary.

## 10. Amendment, 2026-09-28: the web audit covers the development dependencies too

Made by SPEC-058 (#260) and ADR-055's note of 2026-09-28, insert-only under ruling (i) of section
8: every earlier byte is kept in order. It inserts:

- section 2: the sentence that ends R4, which adds `python3` to `audit-web`'s tools;
- section 2: the sentences after "in the `web` job." in R5, which name SPEC-058's command;
- this section.

The audit covers the development dependencies too, and refuses a run that examined nothing.
SPEC-058 decides the stage's command, its level and its verdict, and its tests judge them
(`scripts/tests/test_audit_web.py`). Under ruling (ii) of section 8, SPEC-058's A5 grows this
SPEC's table `TOOLS` in `test_check_gate.py` by one entry, `python3` after `pnpm` for `audit-web`,
and changes no assertion, so A5 of this SPEC judges the new tool too.

## 11. Amendment, 2026-09-29: R8 holds for every workflow with a pull-request trigger

Made by SPEC-190 (#369), insert-only: every earlier byte is kept in order. R8's rule, one group per
pull request that a newer run cancels and a group of its own for every other event, is now carried
by every workflow with a `pull_request` trigger, not `ci.yml` alone. SPEC-190 decides the blocks of
`changelog.yml` and `engine-measure.yml` and a test over every workflow file
(`scripts/tests/test_workflow_concurrency.py`); this SPEC's A10 keeps judging `ci.yml`'s.

## 12. Amendment, 2026-09-29: a scheduled job may save the Rust cache in the default branch's scope (SPEC-191, #370)

Made by SPEC-191 and ADR-191, insert-only under ruling (i) of section 8: every earlier byte is kept
in order. R2 still holds for every workflow that has a `push` or `pull_request` trigger: a cache is
saved only by a push to `dev` or `main` that missed its exact key. SPEC-191 R9 admits one further
shape, in the workflow `rust-cache.yml` alone, whose events are only `schedule` and `workflow_dispatch` and whose save runs only
when a lookup of the same key missed, so that a scheduled run in the default branch's scope saves the
entry a run on any ref can restore. The test that judges R2 (`cache_problems` in `test_ci_workflows.py`)
judges that shape, in the workflow `rust-cache.yml` only, in scenarios and refuses its planted
variants and the same shape in any other workflow (SPEC-191 A7).

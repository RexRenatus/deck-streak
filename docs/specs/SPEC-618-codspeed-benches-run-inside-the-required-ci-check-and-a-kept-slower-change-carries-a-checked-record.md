# SPEC-618: CodSpeed benches run inside the required ci check, and a pull request that keeps a slower change carries a checked regression record

- **Context(s):** `repo` (`scripts/perf-plan.py`, `scripts/perf-record.py`, `scripts/tests`,
  `scripts/mutation-python.json`; in the second part `.github/workflows/ci.yml`, the Cargo and pnpm
  manifests, `config/coverage.json` and `perf/`), `engine` and `web` (the benches, in the second
  part), `docs`.
- **Decided by:** ADR-640 (this SPEC own: the four jobs, the build and upload split, OIDC, the fork
  arm, the check outside the ruleset, the regression record), SPEC-038 and ADR-055 (the gate jobs
  and the aggregate `ci`), ADR-017 (hosted runners), ADR-589 (the deprecated code the exclusions
  name), ADR-613 (coverage stays advisory beside the benches).
- **Status:** in progress on its pull request: the first part delivered the plan and the record
  checker with their tests; the second part adds the bench jobs, the benches, the benchmark list and
  the exclusions with criteria A15 to A24, as section 7 says.

## 1. The problem, measured

At dev `de4c4d4d`:

- No bench is in the tree: `git ls-tree -r --name-only de4c4d4d | grep -c -E '(^|/)benches/|[.]bench[.]ts$'`
  reads 0. The fixed size and memory budgets read no instruction count, so a change that makes a
  hot path slower passes every check.
- The aggregate `ci` needs 17 jobs (`.github/workflows/ci.yml` line 1162), and the workflow default
  token is `contents: read` (lines 19-20).
- The one OIDC grant in the tree is job-scoped: `release.yml` lines 36-39 give `id-token: write`
  with `attestations: write` to the release job alone, and every deploy verifies the release
  tarball with `--signer-workflow` pinned to `release.yml` (`deploy/deploy.sh` lines 239-240, held by
  `scripts/tests/test_deploy_scripts.py` line 509).

## 2. Requirements

R1. `.github/workflows/ci.yml` gains four jobs, a build job and an upload job per lane:
`bench-rust-build` and `bench-rust`, `bench-web-build` and `bench-web`, and `ci`'s `needs:` names all
four, 21 jobs in all. No other workflow is added for benches.

R2. A build job holds `contents: read` only; it runs the plan, the toolchain and every build script
or install script, and archives what its upload job runs with `tar` as a one-day artifact. An upload
job holds `contents: read` and `id-token: write`, unpacks the archive and runs the pinned CodSpeed
action in `mode: simulation`; it runs no package install, no build script and no `cargo build`.

R3. The upload authenticates by OIDC: no repository secret is created, no step receives one, and the
action takes no `token` input. The `ci.yml` header names the two upload jobs' grant.

R4. Each bench job always starts; its plan step decides and every measured step carries the plan's
`if`, so a job whose steps were all skipped concludes `success`. The build job publishes the plan as
the job outputs `applicable`, `upload` and `reason`; its upload job reads them, prints the reason
and gates every other step on `upload`.

R5. `scripts/perf-plan.py --lane rust|web` (standard library only) prints `applicable=true|false`,
`upload=true|false`, `reason=<why>` and one line, `bench-<lane>: applicable: <why>` when the
results upload or `bench-<lane>: not applicable: <why>` when they do not. A push applies always. A
pull request applies when its diff reaches the lane: the Rust lane by any `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml`, `.github/workflows/ci.yml`, a file under a bench target's folder, or a `.rs`
file of a package in the bench packages' workspace dependency closure; the web lane by a
`*.bench.ts` file, `vitest.bench.config.ts`, `web/app/package.json`, `package.json`,
`pnpm-lock.yaml`, `pnpm-workspace.yaml` or a file under `web/app/src/` that is no `.test.` or
`.spec.` file. The reason names the first changed path that reaches the lane, or the count of
changed paths it examined.

R6. The interface of R5. The changed paths come from the file `--changed` names, one per line, or
from `git diff --name-only <base> <head>` in this repository (`--base`, `--head`). The bench
packages are the packages with a target of kind `bench` in the `cargo metadata --format-version 1`
document `--metadata` names; their closure follows every path dependency of every kind, which errs
toward running a bench. The three outputs are appended to the file `--github-output` names and the
line to the file `--step-summary` names; the workflow passes `"$GITHUB_OUTPUT"` and
`"$GITHUB_STEP_SUMMARY"`, and a blank or absent option writes nothing. An event other than `push`
and `pull_request`, a blank `--repo`, a pull request with no changed-path source, a Rust pull
request with no `--metadata`, and a file or a git diff the plan cannot read each print
`bench-<lane>: VOID <what>` and exit 1, so an input the plan cannot read fails the build job in view.

R7. The fork arm. A pull request whose head repository (`--head-repo`, from
`github.event.pull_request.head.repo.full_name`) is not `--repo` prints `upload=false` and the
reason `fork PR (GitHub mints no OIDC token for a fork; the push to dev measures the merged change)`;
`applicable` still follows its diff, so a fork's Rust benches are still built. A blank head
repository (a deleted fork) reads as a fork.

R8. `scripts/perf-record.py check --dir <folder> --branch <head ref> --benchmarks <file>`
(standard library only) reads every `*.json` file directly in the folder, each a record of the
closed schema `deckstreak.perf-regression.v1` (keys `schema`, `branch`, `benchmarks`, `cause`,
`why`, `spec`, `alternatives`, `remediation`; each benchmark `name`, `change_pct`, `base`, `head`,
`instrument`). It refuses a record by arm: an unknown key or a missing key at either level, a blank
`cause`, `why` or `spec`, a `change_pct` that is no number above 0, a benchmark name that is not a
line of the `--benchmarks` file (the caller passes the names its benches define), a `branch` other
than `--branch`, and any other value outside the schema (a sha that is not 7 to 40 lowercase hex, an
instrument other than `simulation`, no benchmark, alternatives that are no list of text, a
remediation that is neither `requested` nor `not-needed: <reason>`). It prints one line per
refusal or `OK` per record, then `perf-record: examined <n> records, <m> refused`; an absent or
empty folder reads `examined 0` and exits 0, a refusal exits 1. The record never carries a verdict.

R9. The benches, wave 1, each Rust measured body a named `#[inline(never)]` function
`<bench>_measured` in its bench file: the scheduler's day queue, its event check and the FSRS fold
and schedule step in `crates/engine-core/benches/scheduling.rs`; the FSRS replay in
`crates/fsrs7/benches/replay.rs`; the native adapter's answer and face in
`crates/ffi/benches/grading.rs` and `crates/ffi/benches/render.rs`; and the web card frame, the
review state machine and the review screen in four `*.bench.ts` files under `web/app/bench/`,
outside `web/app/src`, SPEC-039 R2's root, run by `vitest.bench.config.ts` with the CodSpeed Vitest
plugin. `[profile.bench]` inherits release with line-table debug information; the release profile
is unchanged.

R10. Bench files lie outside `web/app/src`, so neither Stryker's mutate list nor SPEC-039 R2's web
production code reaches them, and they join the coverage exclusions; the CodSpeed check joins no
ruleset, and the merge step reads it beside the regression record.

R11. The record check of R8 runs on each pull request, in `bench-rust-build`, which holds
`contents: read` only. Its one step runs under `github.event_name == 'pull_request'`, copies only
the records the pull request adds or changes under `perf/regressions.d/`
(`git diff --name-only --diff-filter=AM HEAD^1 HEAD -- perf/regressions.d`) into
`$RUNNER_TEMP/records`, and runs `scripts/perf-record.py check --dir "$RUNNER_TEMP/records"
--branch "$HEAD_REF" --benchmarks perf/benchmarks.txt`, the head ref passed through the step's
environment. A refused record fails the build job and so `ci`; a pull request that carries no
record examines none. `perf/regressions.d/README.md` says what a record is and when a pull request
carries one.

R12. The benchmark list `perf/benchmarks.txt` holds one name per line, each
`<file stem>::<bench>`: for a Rust bench file the name of each `#[divan::bench]` function, and for a
web bench file the name each `bench('<name>'` call passes. A test derives the names from the bench
files and holds the list equal to them, so a bench added, renamed or removed without the list fails,
and the record check refuses a name no bench defines.

R13. The bench sources. Each Rust bench package takes `divan` as a dev-dependency, the package
`codspeed-divan-compat` at one exact version through one root `[workspace.dependencies]` entry, and
each bench file is a `[[bench]]` target with `harness = false`; a dev-dependency is no edge of the
context map. The answer bench in `crates/ffi/benches/grading.rs` builds its 2,000-card collection by
a seeded builder of its own over the `anki` dev-dependency `crates/ffi` already declares, and takes
a fresh copy for each iteration through Divan's `with_inputs`, outside the timed section; no bench
adds a dependency edge other than the bench crate. A web bench file takes Vitest 5's form,
`test('<name>', async ({ bench }) => { await bench('<name>', fn).run() })`. Every bench input is
seeded and fixed, and no bench asserts a computed value.

R14. The one install of the Rust bench lane. R2's package install is a package manager's install.
The Rust build job, which holds `contents: read` alone, installs one tool, `cargo-codspeed` at its
pinned version, by `cargo install --locked`, and carries the binary in its archive; the Rust
upload job puts the unpacked binary's folder on its path and installs nothing, and the web upload
job sets up Node and pnpm as its build job does and installs no package. Neither upload job runs
`cargo build`, `cargo install`, `pnpm install`, `npm` or `protoc`.

R15. The coverage exclusions are entries of `config/coverage.json`, the register the coverage
report applies: `crates/*/benches/**` in the Rust lane and `web/app/bench/**` in the web lane, each
of kind `bench` and citing this SPEC. Stryker's mutate list is unchanged, since it reaches only
`src/**` and no bench file lies there. The Vitest workspace config keeps its coverage block as it
is, and its test project includes only test and spec files, so no bench file loads there.

## 3. Acceptance criteria of SPEC-618

| id | criterion | decided by |
|---|---|---|
| A1 | a fork pull request whose diff reaches a bench builds and never uploads, with the fork reason, and a deleted fork reads as a fork (R7) | `test_perf_plan.py` `test_a_fork_pull_request_builds_but_never_uploads` |
| A2 | a same-repository pull request uploads what it builds, and every push applies and uploads (R5, R7) | `test_perf_plan.py` `test_a_same_repository_pull_request_uploads_what_it_builds`, `test_a_push_always_applies_and_uploads` |
| A3 | the Rust lane applies only where the diff reaches a manifest, the lockfile, the toolchain pin, the ci workflow, a bench file or a Rust file of the bench closure, and names the first such path (R5, R6) | `test_perf_plan.py` `test_the_rust_lane_applies_only_where_the_diff_reaches_a_rust_bench` |
| A4 | the web lane applies only where the diff reaches a bench file, the bench config, a web manifest, the lockfile or web app source outside its tests (R5) | `test_perf_plan.py` `test_the_web_lane_applies_only_where_the_diff_reaches_a_web_bench` |
| A5 | the three outputs and the reason line are printed, and appended to the output and summary files (R4, R6) | `test_perf_plan.py` `test_the_reason_is_printed_and_written_as_job_outputs` |
| A6 | the changed paths come from git between base and head, and a diff git refuses reads VOID (R6) | `test_perf_plan.py` `test_the_changed_paths_come_from_git_between_base_and_head` |
| A7 | an input the plan cannot read reads VOID and exits 1, and a missing option is a usage error (R6) | `test_perf_plan.py` `test_an_input_the_plan_cannot_read_is_refused` |
| A8 | a complete record reads OK, only `*.json` files directly in the folder are read, and an absent or empty folder examines none (R8) | `test_perf_record.py` `test_a_complete_record_reads_ok_and_a_missing_folder_examines_none` |
| A9 | an unknown key at either level is refused, a verdict key included (R8) | `test_perf_record.py` `test_an_unknown_key_is_refused` |
| A10 | a blank cause or why is refused (R8) | `test_perf_record.py` `test_a_blank_cause_or_why_is_refused` |
| A11 | a change of 0 or less is refused (R8) | `test_perf_record.py` `test_a_change_of_zero_or_less_is_refused` |
| A12 | a benchmark no bench defines is refused (R8) | `test_perf_record.py` `test_a_benchmark_no_bench_defines_is_refused` |
| A13 | a record for another branch is refused (R8) | `test_perf_record.py` `test_a_record_for_another_branch_is_refused` |
| A14 | every other value outside the closed schema is refused, each problem on its own line (R8) | `test_perf_record.py` `test_a_record_outside_the_schema_is_refused` |
| A15 | the four bench jobs exist and the aggregate `ci` needs each of them beside its other jobs (R1) | `test_ci_workflows.py` `test_the_four_bench_jobs_are_needs_of_ci`, `test_the_gate_runs_in_four_parallel_jobs` |
| A16 | each build job holds `contents: read` alone, each upload job holds `contents: read` and `id-token: write`, no other `ci.yml` job holds `id-token`, and the header names the upload jobs' grant (R2, R3) | `test_ci_workflows.py` `test_only_the_two_upload_jobs_hold_an_id_token_grant` |
| A17 | the Rust build job installs the pinned CodSpeed cargo tool and carries it in its archive; an upload job prints the reason, checks out, downloads and unpacks the archive (the Rust lane puts the tool's folder on its path) and runs the pinned action in simulation mode with no `token`, and no upload step builds or installs a package or a tool (R2, R3, R14) | `test_ci_workflows.py` `test_an_upload_job_unpacks_and_uploads_and_builds_nothing` |
| A18 | each bench job always starts, its plan publishes the three outputs, and every later step carries the plan's gate, so a run the plan finds not applicable skips every measured step (R4) | `test_ci_workflows.py` `test_each_bench_job_starts_and_its_plan_gates_every_step` |
| A19 | on each pull request the Rust build job checks the records the diff adds or changes under `perf/regressions.d/` against the head ref and the benchmark list (R8, R11) | `test_ci_workflows.py` `test_the_record_check_reads_the_records_a_pull_request_adds` |
| A20 | `[profile.bench]` inherits release with line-table debug information, and the root manifest holds no release profile (R9) | `test_perf_benches.py` `test_the_bench_profile_inherits_release_with_line_tables` |
| A21 | each bench package takes the renamed Divan crate through the workspace entry at one exact version, each bench file is a target with `harness = false`, and each measured body is an `#[inline(never)]` function (R9, R13) | `test_perf_benches.py` `test_each_rust_bench_is_a_harnessless_target_with_a_measured_body` |
| A22 | `perf/benchmarks.txt` equals the names the bench files define (R12) | `test_perf_benches.py` `test_the_benchmark_list_equals_the_names_the_bench_files_define` |
| A23 | `vitest.bench.config.ts` runs the CodSpeed plugin under jsdom over `*.bench.ts` files, and the web bench files of R9 exist (R9, R13) | `test_perf_benches.py` `test_the_web_benches_run_under_the_codspeed_plugin` |
| A24 | no bench file lies under `web/app/src`, the root Stryker's mutate list and SPEC-039 R2 reach, and bench files are in the coverage exclusions of `config/coverage.json` (R10, R15) | `test_perf_benches.py` `test_bench_files_are_excluded_from_mutation_and_coverage` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_a_fork_pull_request_builds_but_never_uploads
A2: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_a_same_repository_pull_request_uploads_what_it_builds
A2: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_a_push_always_applies_and_uploads
A3: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_the_rust_lane_applies_only_where_the_diff_reaches_a_rust_bench
A4: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_the_web_lane_applies_only_where_the_diff_reaches_a_web_bench
A5: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_the_reason_is_printed_and_written_as_job_outputs
A6: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_the_changed_paths_come_from_git_between_base_and_head
A7: python3 -m unittest discover -s scripts/tests -p test_perf_plan.py -k test_an_input_the_plan_cannot_read_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_a_complete_record_reads_ok_and_a_missing_folder_examines_none
A9: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_an_unknown_key_is_refused
A10: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_a_blank_cause_or_why_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_a_change_of_zero_or_less_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_a_benchmark_no_bench_defines_is_refused
A13: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_a_record_for_another_branch_is_refused
A14: python3 -m unittest discover -s scripts/tests -p test_perf_record.py -k test_a_record_outside_the_schema_is_refused
A15: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_four_bench_jobs_are_needs_of_ci
A15: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_gate_runs_in_four_parallel_jobs
A16: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_only_the_two_upload_jobs_hold_an_id_token_grant
A17: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_an_upload_job_unpacks_and_uploads_and_builds_nothing
A18: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_bench_job_starts_and_its_plan_gates_every_step
A19: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_record_check_reads_the_records_a_pull_request_adds
A20: python3 -m unittest discover -s scripts/tests -p test_perf_benches.py -k test_the_bench_profile_inherits_release_with_line_tables
A21: python3 -m unittest discover -s scripts/tests -p test_perf_benches.py -k test_each_rust_bench_is_a_harnessless_target_with_a_measured_body
A22: python3 -m unittest discover -s scripts/tests -p test_perf_benches.py -k test_the_benchmark_list_equals_the_names_the_bench_files_define
A23: python3 -m unittest discover -s scripts/tests -p test_perf_benches.py -k test_the_web_benches_run_under_the_codspeed_plugin
A24: python3 -m unittest discover -s scripts/tests -p test_perf_benches.py -k test_bench_files_are_excluded_from_mutation_and_coverage
```

Each test drives its script as a child process over planted files in a temporary directory; no
test runs cargo, makes a repository or reaches GitHub. A1 and A3 to A14 are red at the red commit
by an assertion against a stub that keeps every argument and lacks each behaviour: the plan stub
reads every run applicable and uploading and writes no output, and the record stub examines no
record. A2 is not red: its two tests are the controls beside the fork arm, a same-repository pull
request and a push, which upload whatever the plan reads, so the stub reads them green at both
pushes.

A15 to A19 are decided over `.github/workflows/ci.yml` by `test_ci_workflows.py`, whose verdicts
the gate's python stage gives by name; A20 to A24 are decided over the tree by
`test_perf_benches.py`, which loads no workflow and runs on any checkout. Each is red at the second
part's red commit by an assertion against what is absent there: the four jobs and their grants, the
record step, the bench profile, the bench targets and files, the benchmark list, the bench config
and the bench exclusions. The sorted needs list that `test_the_gate_runs_in_four_parallel_jobs`
pins gains the four jobs in the same commit, so that test is red beside A15 until the jobs land,
while `test_the_aggregate_check_needs_every_job_and_always_runs` stays green throughout: it holds
every job the file defines among the needs of `ci`. None of A15 to A24 is a control, and none pins
an engine-computed value: every bench input is seeded and fixed, and no bench asserts what it
computes.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-618-codspeed-benches-run-inside-the-required-ci-check-and-a-kept-slower-change-carries-a-checked-record.md` | docs | added |
| `docs/decisions/ADR-640-codspeed-benches-build-without-a-token-and-upload-by-oidc-inside-the-required-ci-check.md` | docs | added |
| `docs/schematics/codspeed-benches.md` | docs | added |
| `docs/red-first/SPEC-618.md` | docs | added |
| `changelog.d/codspeed-benches-618.md` | docs | added |
| `scripts/perf-plan.py` | scripts | added: the bench plan of one lane |
| `scripts/perf-record.py` | scripts | added: the regression record checker |
| `scripts/tests/test_perf_plan.py` | tests | added |
| `scripts/tests/test_perf_record.py` | tests | added |
| `scripts/tests/fixtures/perf/` | tests | added: a cargo metadata document, a record and a benchmark list |
| `scripts/mutation-python.json` | mutation | changed: two entries, each script with its test module |
| `docs/OWNER-SETUP.md` | docs | changed: section 10, the CodSpeed setup steps |
| `perf/regressions.d/README.md` | docs | added: what a record is and when a pull request carries one |
| `scripts/tests/test_ci_workflows.py` | tests | changed: the aggregate check's needs pin gains the four jobs, and one appended class holds the bench jobs |
| `scripts/tests/test_perf_benches.py` | tests | added: the bench profile, manifests, bench files, benchmark list, bench config and exclusions |
| `.github/workflows/ci.yml` | ci | changed: the four bench jobs, the aggregate check's needs and the header sentence on the upload grant |
| `Cargo.toml` | engine | changed: `[profile.bench]` and the workspace entry of the renamed Divan crate |
| `Cargo.lock` | engine | changed by cargo: the Divan compatibility crate and its dependencies |
| `crates/engine-core/Cargo.toml` | engine | changed: the Divan dev-dependency and the bench target |
| `crates/fsrs7/Cargo.toml` | engine | changed: the Divan dev-dependency and the bench target |
| `crates/ffi/Cargo.toml` | engine | changed: the Divan dev-dependency and the two bench targets |
| `crates/engine-core/benches/scheduling.rs` | engine | added: the day queue, event check, fold and schedule step benches |
| `crates/fsrs7/benches/replay.rs` | engine | added: the FSRS replay bench |
| `crates/ffi/benches/grading.rs` | engine | added: the answer bench over a seeded collection |
| `crates/ffi/benches/render.rs` | engine | added: the face bench |
| `vitest.bench.config.ts` | web | added: the bench run under the CodSpeed Vitest plugin |
| `package.json` | web | changed: the CodSpeed Vitest plugin at an exact version |
| `pnpm-lock.yaml` | web | changed by pnpm: the plugin and its dependencies |
| `web/app/bench/**/*.bench.ts` | web | added: the card frame, review step and review screen benches |
| `web/app/tsconfig.json` | web | changed: svelte-check covers the bench folder |
| `config/coverage.json` | config | changed: the two bench exclusions |
| `perf/benchmarks.txt` | perf | added: the benchmark list |
| `scripts/mutation-rows.d/S61800-S61899.json` | mutation | added: rows on the new workflow lines |
| `scripts/mutation-verdict.py` | scripts | changed: `LEG_CEILING["ci"]` 185, and its comment's 71 other `ci.yml` jobs with the signed record's path (section 9) |
| `scripts/mutation-rows.d/S36200-S36299.json` | mutation | changed: `S36201` re-anchored on `"ci": 185`, its mutant kept (section 9) |
| `scripts/tests/test_dispatch_shards.py` | tests | changed: the pins of the `ci` ceiling at 185 (A25) |
| `scripts/tests/test_mutation_verdict.py` | tests | changed: the pins of the `ci` ceiling at 185 (A26) |
| `scripts/tests/test_release_rust_ceiling.py` | tests | changed: SPEC-423's pins of the `ci` ceiling at 185 (A27) |
| `scripts/tests/test_release_python_ceiling.py` | tests | changed: the joint line and the kept values at 185 (A28) |
| `scripts/tests/test_fsrs7_pin.py` | tests | changed: the dev-dependency pin admits `divan` and `fsrs6` by name, and a planted third dev-dependency is refused (A29) |
| `scripts/tests/test_coverage_report.py` | tests | changed: the `bench` kind (A30) |
| `docs/specs/SPEC-417-the-rows-leg-is-dealt-into-legs-that-fit-their-bound.md` | docs | changed: section 12, R11, R14 and the SPEC-362 pins at 185 |
| `docs/specs/SPEC-342-fsrs-7-replay-time-undo-and-the-full-sync-choice-are-measured.md` | docs | changed: section 8, A1's dev-dependency pin by name |

## 5. What this does NOT cover

- Wave 2 benches (opening a collection and queueing, a full and an incremental sync, the wasm
  adapter's next card): each joins only when the bench census puts its functions in its membership.
- A tokenless upload for a fork pull request: it joins only by its own pull request, red first, with
  a fork run shown green (ADR-640 D3).
- The CodSpeed check as a required check of any ruleset, which ADR-640 D5 refuses.

## 6. Risks

- **The plan reads too little.** A Rust change outside every rule would skip the benches on its
  pull request; the closure follows every path dependency and every manifest, and the push to dev
  after the merge measures every change whatever its pull request read.
- **A new trigger reaches the plan.** An event the plan does not read fails the build job in view,
  so a trigger added to `ci.yml` needs the plan to name it first.
- **A tree census reads the new fixtures.** `scripts/tests/fixtures/perf/` is new; the gate python
  stage verdicts of every module are read by name on this pull request before it lands.
- **The upload fails.** A failed upload turns `ci` red until a re-run; CodSpeed's analysis and its
  check stay outside `ci` (ADR-640 D5).

## 7. What each part of this pull request delivers

The first part delivered the plan and the record checker with their tests (R5 to R8, A1 to A14).
The second part delivers, in this order: the tests of A15 to A24, red first; the four jobs, their
grants and the record step in
`.github/workflows/ci.yml` (R1 to R4, R11, R14); the Rust benches with the bench profile and their
manifests (R9, R13); the web benches with their config and the CodSpeed Vitest plugin (R9, R13);
and the exclusions, the benchmark list and the mutation rows on the new workflow lines (R10, R12,
R15). The steps an owner takes once on CodSpeed are section 10 of `docs/OWNER-SETUP.md`; its step
that sets the default branch for analysis follows the first upload. The first upload is checked on
the pull request's last head, by the `CodSpeed Performance Analysis` check run on that head and the
upload job's OIDC authentication line, and recorded on the pull request.

## 8. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani (ADR-640 D7). The first part adds documents, two
standard-library scripts and their tests; the second part adds workflow jobs, benches and
configuration, and its benches call engine functions and assert nothing about them. No protocol,
state machine, interleaving or engine path gains or changes.

## 9. The pull request run's Rust ceiling is the signed 185, and the gate names the bench words

The four bench jobs are four more jobs a pull request run generates, so the run's ceiling for Rust
legs falls from 189 to 185 by SPEC-417 R11's unchanged rule: the run's 256 jobs less every other
`ci.yml` job, 71 with the four bench jobs, the rows job counted at its 18 legs. The owner signed
that figure in `docs/rulings/OWNER-RULING-2026-11-05-rust-ceiling-185.md`, which amends the two
bullets of `docs/rulings/OWNER-RULING-2026-10-18-rows-legs-822.md` on the ceiling and on the joint
line. The bench lanes also add `divan` to the FSRS-7 crate's dev-dependencies and the `bench` kind
to two coverage exclusions, and the gate's pins name both. This section adds four requirements and
their criteria; every requirement and criterion above stands as written.

1. **The ceiling.** `LEG_CEILING["ci"]` in `scripts/mutation-verdict.py` is 185: 256 less 71 other
   `ci.yml` jobs, 67 and the four bench jobs. The comment above it names the 71 jobs and cites the
   signed record. Row `S36201` keeps its id, stem, mutant and killer and is re-anchored: its find
   `"ci": 189` becomes `"ci": 185`, and its replacement stays `"ci": 194`. `LEG_CEILING["battery"]`
   234, `RUST_RELEASE_CEILING` 192, `RUST_MATRIX_LEGS` 256, `ROWS_MAX_LEGS` 18 and
   `PYTHON_MAX_SHARDS` 32 do not move. A pull request whose Rust mutants need more than 185 legs is
   refused whole by name, never cut.
2. **The pins.** Every test pin of the `ci` ceiling, or of a figure derived from it, follows 185 and
   keeps its relation to the ceiling, by SPEC-417 R14 at 185 (SPEC-417 section 12): each refusal
   that names the pull request run's ceiling names 185, and each `legs N of ceiling 189` reads
   `legs N of ceiling 185`; with no ceiling named, 185 one-leg mutants size at 185 legs and 186 are
   refused; the ingest listing past the ceiling is 12581 mutants, one more than the 12580 that 185
   legs of 68 hold, projected at 1585206 s serially and refused naming 185; the pull request run's
   joint line shares 217 legs, 32 Python and 185 Rust, and its runner-minutes are
   `(32 x 1800 + 185 x 10800) // 60`, 34260; SPEC-423's placeholder release ceiling is 185, the
   figure one past it 186 and the figure one below it 184, the range's ends are 185 and 256, and
   the range's refusal reads `outside 185..256`; a release ceiling of 200 or 256 keeps its figures.
   The 128 beside the ceiling in `rust_deal(self, listed, scripts, release, 185, 128)` is not the
   ceiling. The call register of `scripts/tests/test_ci_workflows.py` names each call these pins
   change, each with its count of 1.
3. **Divan by name.** The pin of the FSRS-7 crate's dev-dependencies in
   `scripts/tests/test_fsrs7_pin.py` admits `["divan", "fsrs6"]` exactly, the bench harness and the
   released scheduler; its dependency pin `["fsrs7"]` does not move. A planted crate whose
   dev-dependencies are `divan`, `fsrs6` and `tempfile` is still refused, its finding naming the
   three (SPEC-342 section 8).
4. **The bench kind.** `KINDS` in `scripts/tests/test_coverage_report.py` gains `"bench"`, so the
   two bench exclusions of `config/coverage.json` carry a known kind; nothing else in that module
   changes.

### Acceptance criteria of section 9

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A25 | the derived ceilings, 71 other `ci.yml` jobs counted, equal `{"ci": 185, "battery": 234}` and `LEG_CEILING`, and the module's refusal, listing and headroom pins follow 185 (requirements 1 and 2) | red at a commit whose `LEG_CEILING["ci"]` is 189: the derived 185 differs from it | `TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget`, `TheReleaseFitsOneRun.test_the_release_listing_fits_one_run`, `TheReleaseFitsOneRun.test_a_listing_beyond_the_ceiling_is_refused_whole`, `TheReleaseFitsOneRun.test_every_sizing_prints_its_headroom` |
| A26 | the shard cap, the sizing constants and the whole tree's headroom line follow 185 (requirement 2) | red at a commit whose `LEG_CEILING["ci"]` is 189: `LEG_CEILING` differs from the pinned 185 | `TheShardsFitTheirBound.test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap`, `TheLegsAreSizedToTheRun.test_the_sizing_constants_are_the_measured_ones`, `TheLegsAreSizedToTheRun.test_the_whole_tree_is_sized_at_the_batterys_ceiling` |
| A27 | SPEC-423's pins follow 185: the placeholder, one past it, one below it, the range's ends, its refusal, the pull request run's refusal and the joint line (requirement 2) | red at a commit whose `LEG_CEILING["ci"]` is 189: the range's refusal reads `outside 189..256` | `TheReleaseRunSizesItsRustLegsToItsCeiling.test_only_a_true_release_mark_holds_the_rust_ceiling`, `TheReleaseRunSizesItsRustLegsToItsCeiling.test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run`, `TheReleaseRunSizesItsRustLegsToItsCeiling.test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling`, `TheReleaseRunSizesItsRustLegsToItsCeiling.test_a_rust_ceiling_outside_its_range_is_refused_by_name`, `TheReleaseRunSizesItsRustLegsToItsCeiling.test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone`, `TheReleaseRunSizesItsRustLegsToItsCeiling.test_the_rust_release_line_stands_beside_the_python_release_line`, `WhatTheReleaseRunKeeps.test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder` |
| A28 | the pull request plan's joint line shares 217 legs, 32 Python and 185 Rust, of 34260 runner-minutes (requirement 2) | red at a commit whose `LEG_CEILING["ci"]` is 189: the line reads 221 legs and 189 Rust | `TheReleaseRunDealsUpToItsCeiling.test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run` |
| A29 | the FSRS-7 crate's dev-dependencies `divan` and `fsrs6` are admitted, and a planted `tempfile` beside them is refused by a finding naming the three (requirement 3) | red at a commit whose pin admits `fsrs6` alone: the crate's `divan` is refused | `EachResolvesItsOwnScheduler.test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler` |
| A30 | every coverage exclusion's kind is known, the two bench rows' `bench` included (requirement 4) | red at a commit whose `KINDS` lacks `bench`: the two bench rows have an unknown kind | `TheExclusionList.test_every_exclusion_is_one_row_with_its_reason` |

```acceptance
A25: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget -k test_the_release_listing_fits_one_run -k test_a_listing_beyond_the_ceiling_is_refused_whole -k test_every_sizing_prints_its_headroom
A26: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap -k test_the_sizing_constants_are_the_measured_ones -k test_the_whole_tree_is_sized_at_the_batterys_ceiling
A27: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_only_a_true_release_mark_holds_the_rust_ceiling -k test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run -k test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling -k test_a_rust_ceiling_outside_its_range_is_refused_by_name -k test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone -k test_the_rust_release_line_stands_beside_the_python_release_line -k test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder
A28: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run
A29: python3 -m unittest discover -s scripts/tests -p test_fsrs7_pin.py -k test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler
A30: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_every_exclusion_is_one_row_with_its_reason
```

Every red and green of A25 to A30 is recorded in `docs/red-first/SPEC-618.md`. A25's tests, and the
call register's own test, are read in CI by name, from the job that runs `scripts/tests`.

# SPEC-618: CodSpeed benches run inside the required ci check, and a pull request that keeps a slower change carries a checked regression record

- **Context(s):** `repo` (`scripts/perf-plan.py`, `scripts/perf-record.py`, `scripts/tests`,
  `scripts/mutation-python.json`; later in this pull request `.github/workflows/ci.yml` and the
  Cargo and pnpm manifests), `engine` and `web` (the benches, later in this pull request), `docs`.
- **Decided by:** ADR-640 (this SPEC own: the four jobs, the build and upload split, OIDC, the fork
  arm, the check outside the ruleset, the regression record), SPEC-038 and ADR-055 (the gate jobs
  and the aggregate `ci`), ADR-017 (hosted runners), ADR-589 (the deprecated code the exclusions
  name), ADR-613 (coverage stays advisory beside the benches).
- **Status:** in progress on its pull request: this part delivers the plan and the record checker
  with their tests; section 7 names the parts that follow in the same pull request.

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
review state machine and the review screen in four `*.bench.ts` files under `web/app/src/lib/`,
run by `vitest.bench.config.ts` with the CodSpeed Vitest plugin. `[profile.bench]` inherits release
with line-table debug information; the release profile is unchanged.

R10. `*.bench.ts` files join the Stryker mutate exclusions and the coverage exclusions; the
CodSpeed check joins no ruleset, and the merge step reads it beside the regression record.

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
```

Each test drives its script as a child process over planted files in a temporary directory; no
test runs cargo, makes a repository or reaches GitHub. A1 and A3 to A14 are red at the red commit
by an assertion against a stub that keeps every argument and lacks each behaviour: the plan stub
reads every run applicable and uploading and writes no output, and the record stub examines no
record. A2 is not red: its two tests are the controls beside the fork arm, a same-repository pull
request and a push, which upload whatever the plan reads, so the stub reads them green at both
pushes.

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

## 5. What this does NOT cover

- Wave 2 benches (opening a collection and queueing, a full and an incremental sync, the wasm
  adapter's next card): each joins only when the bench census puts its functions in its membership.
- A tokenless upload for a fork pull request: it joins only by its own pull request, red first, with
  a fork run shown green.
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

## 7. The later parts of this pull request

The workflow jobs (R1 to R4), the benches and their bench configuration (R9), the exclusions and
the ruleset reading (R10), and the owner setup steps in `docs/OWNER-SETUP.md` come later in this
same pull request, each with its own manifest rows and its own criteria added to this SPEC.

## 8. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani (ADR-640 D7). This part adds documents, two standard-library
scripts and their tests; no protocol, state machine, interleaving or engine path gains or changes.

# SPEC-592: an advisory coverage report runs on every pull request and every dev push, and never gates a merge

- **Context(s):** `repo` (`.github/workflows/coverage.yml`, `.github/dependabot.yml`,
  `.github/workflows/changelog.yml`, `config/coverage.json`, `scripts/coverage-report.py`,
  `tools/coverage/`, `scripts/tests`, the two rows bands), `web` (`vitest.config.ts`, the root
  manifest, the pnpm catalog and lockfile), `docs`.
- **Decided by:** ADR-613 (this SPEC own: where coverage runs and why it never gates, the VOID
  arms, the one exclusion list, the tools and the report), ADR-589 (the deprecated Telegram code the
  list excludes), SPEC-038 and ADR-055 (the gate jobs and the aggregate `ci`, unchanged here),
  ADR-005 with its amendments and ADR-517 (the advisories rule the provider dependencies and the
  Python pin fall under), SPEC-190 (the fragment rule for a Dependabot pull request, whose census
  gains the pip entry and whose R13 directory clause R12 amends by name).
- **Status:** delivered with its tests and `docs/red-first/SPEC-592.md`.

## 1. The problem, measured

At dev `00dcad26`:

- No workflow measures coverage: `git ls-tree --name-only 00dcad26 .github/workflows/ | grep -c coverage`
  reads 0.
- No coverage provider is locked: `grep -c -E "^  .@vitest/coverage-" pnpm-lock.yaml` reads 0, and the
  catalog holds vitest alone (`pnpm-workspace.yaml`).
- No update entry covers a Python tool: `.github/dependabot.yml` names three ecosystems, cargo, npm
  and github-actions, each from `/` (`grep -c package-ecosystem .github/dependabot.yml` reads 3).
- The `fragment` step passes a Dependabot pull request only when each path it changes matches,
  whole, one of its three patterns, cargo, npm and actions, joined in its one `grep -v -x -E`
  alternation (`grep -c -E '^ +(cargo|npm|actions)=' .github/workflows/changelog.yml` reads 3), so
  it refuses a Dependabot update of `tools/coverage/requirements.txt` and asks for a fragment.
- The required checks are `ci`, `fragment` and `dependency-review` on dev and `ci` and `fragment` on
  main (`.github/rulesets/dev.json`, `.github/rulesets/main.json`); the aggregate `ci` needs all 17
  other jobs of `ci.yml` (`ci.yml` line 1162).
- The gate jobs a lane mirrors, on one dev push at `00dcad26`:
  - the rust job ran 25 min 44 s on a cache hit (job 125852730835);
  - the hygiene job, whose python stage runs the three suites, ran 19 min 34 s (job 125852730858);
  - the web job ran 2 min 10 s (job 125852731001);
  - the release job, which compiles its own build, ran 36 min 50 s (job 125852731046).
- The provider only warns on a version mismatch: vitest `coverage.ts` `_initialize` logs "Running
  mixed versions is not supported" and goes on, so no tool refuses a mixed lock.

## 2. Requirements

R1. `.github/workflows/coverage.yml` (workflow `coverage`) runs on `pull_request` into dev or main
(types opened, synchronize, reopened) and on `push` to dev. Its token is `contents: read`, its
concurrency group is `coverage-` plus the house expression, and every action is pinned by its full
commit SHA to the pin `ci.yml` uses.

R2. Its jobs `coverage-plan`, `coverage-rust`, `coverage-web` and `coverage-python` are named in no
`needs:` of any other workflow and in no ruleset; no job of `ci.yml` and no ruleset file changes. No
step uses `continue-on-error`, no expression reads a secret, and every job carries a
`timeout-minutes` bound.

R3. `coverage-plan` reads the event and, on a pull request, `git diff --name-only HEAD^1 HEAD`
through `scripts/coverage-report.py plan`: a lane applies when a changed path matches the lane
measured globs, its configuring globs or the shared coverage files. A push to dev and the release
pull request (dev into main from this repository) run every lane over the whole tree. It names
each lane and why, and outputs the artifact retention: 30 days for a pull request, 90 for a dev
push.

R4. Each lane exits 0 on a measured report whatever the figures read and exits 1 only on VOID:
zero examined lines in a lane that applies, an lcov the report cannot read, or (web lane only) a
coverage provider locked at another version than vitest. A failing test does not fail a lane; the
report prints the runner exit beside its figures. Before measuring, each lane runs the planted
controls: a planted empty lane and a planted unreadable lcov must read VOID, and a planted uncovered
changed line must be printed; the web lane also runs a planted provider mismatch. A control that
reads otherwise is VOID.

R5. `config/coverage.json` is the one exclusion list: each row holds one glob, its lane, its kind and
its reason, and the rows for the deprecated Telegram code cite ADR-589. The report applies the list
to every lane and prints its row count; the Rust run two time-saving flags (the deprecated bot
package left out, the timing binary left out) are printed by `scripts/coverage-report.py args` from
the rows that name a package or a binary. No inline exclusion hint is in the tree without a row,
and every row but a generated one matches a tracked file.

R6. On a pull request, the report intersects the lines `git diff -U0 HEAD^1 HEAD` adds in the lane
measured, non-excluded files with the lane lcov `DA` records and prints "changed executable lines
N, executed E, not executed: file:line ...", then the changed files the lcov lacks. Every figure it
prints or writes is an integer count; it prints no percentage.

R7. Each lane writes a step summary table, one row per crate, module or package (lines, lines
executed, lines not executed, functions, functions executed, branches, branches taken, and regions
for Rust), and uploads `coverage-<lane>` (the tool lcov, its summary and the report) with the
retention R3 names, under `if: always()`.

R8. The Rust lane runs cargo-llvm-cov over cargo-nextest, both installed by the pinned install
action at exact versions with no fallback, adds `llvm-tools` inside the lane only, restores the
Rust cache and saves none, and takes the region figures from the tool summary.

R9. The web lane uses `@vitest/coverage-v8` at exactly the version the lockfile resolves for vitest,
in the pnpm catalog beside vitest, through a `coverage` block in the root `vitest.config.ts` that
stays off unless `--coverage` is passed, includes every app source file and reports on failure.

R10. The Python lane installs coverage.py from `tools/coverage/requirements.txt` (one exact pin with
its hashes) with `--require-hashes` into a virtual environment under the runner temp directory, and
measures the three suites the gate python stage runs, with the settings in
`tools/coverage/coverage.toml`: branch measurement, the `ctrace` core, the subprocess patch and
parallel data files, combined into one lcov.

R11. The requirements file has an update path: `.github/dependabot.yml` holds one `pip` entry for
the directory `/tools/coverage`, weekly into dev with the cooldown and the major-version ignore the
other entries carry, so a release that fixes an advisory reaches the Python lane by a pull request;
the `fragment` check passes a Dependabot pull request that changes only that file, as it does for
the files of the other entries.

R12. The clause of SPEC-190 R13 that reads "each ecosystem with `directory: /`" is amended, by
name, to admit one named directory more: `/tools/coverage`, for the `pip` ecosystem alone. So the
`fragment` check passes a Dependabot pull request that changes only
`tools/coverage/requirements.txt`: the Dependabot arm of the step gains that one exact path,
`tools/coverage/requirements\.txt`, as one more alternative of its one `grep -v -x -E`
alternation, while the author check against `dependabot[bot]` and the whole-line match stay as
they are and no glob is added. As SPEC-190 R14 says of every other pull request, that file changed
beside a path no ecosystem updates, that file changed by any other author, and a path that only
resembles it (`tools/coverage/requirements.txt.bak`, `tools/coverage/x/requirements.txt`) each
still need a fragment; and SPEC-190 A14 reads four ecosystems, cargo, npm and github-actions from
`/` and pip from `/tools/coverage`.

## 3. Acceptance criteria of SPEC-592

| id | criterion | decided by |
|---|---|---|
| A1 | a lane whose lcov leaves no examined line after the list and the lane globs reads VOID and exits 1, and one examined line is a measured report that exits 0 (R4) | `test_coverage_report.py` `test_a_lane_that_examined_no_line_reads_void`, `test_one_examined_line_is_a_measured_report` |
| A2 | a planted changed line that no test executed is printed by file and line in the changed-line figure, and every figure the report prints or writes is an integer count (R6, R7) | `test_coverage_report.py` `test_a_planted_uncovered_changed_line_is_printed_by_file_and_line`, `test_every_figure_is_an_integer_count` |
| A3 | a coverage provider locked at another version than vitest, or none locked, reads VOID, and the committed lockfile locks the provider at the vitest version (R4, R9) | `test_coverage_report.py` `test_a_provider_locked_at_another_version_reads_void`, `test_the_committed_lockfile_locks_the_provider_at_the_vitest_version` |
| A4 | an lcov that is missing, empty, malformed or left open reads VOID and exits 1 (R4) | `test_coverage_report.py` `test_an_unreadable_lcov_reads_void` |
| A5 | the plan runs only the lanes a pull request diff touches, every lane on a dev push and on the release pull request, and names the retention for each (R3) | `test_coverage_report.py` `test_the_plan_runs_only_the_lanes_a_diff_touches`, `test_a_dev_push_and_the_release_pull_request_run_every_lane` |
| A6 | every exclusion is one row with its lane, kind and reason, the deprecated rows cite ADR-589, the Rust run flags come from the rows, the report prints the row count, no inline hint is in the tree without a row, and every row but a generated one matches a tracked file (R5) | `test_coverage_report.py` `test_every_exclusion_is_one_row_with_its_reason`, `test_the_rust_run_flags_come_from_the_rows`, `test_no_inline_hint_without_a_row`, `test_every_row_matches_a_tracked_file` |
| A7 | no workflow `needs:` and no ruleset names a coverage job, the workflow is read-only, bounded, pinned and free of `continue-on-error` and secrets, the provider check runs in `coverage.yml` only, and each lane runs its planted controls before it measures, a planted case that misreads making the controls VOID (R1, R2, R4) | `test_coverage_report.py` `test_no_needs_and_no_ruleset_names_a_coverage_job`, `test_the_coverage_workflow_is_read_only_bounded_and_pinned`, `test_each_lane_runs_its_planted_controls_first`, `test_a_planted_case_that_misreads_makes_the_controls_void` |
| A8 | the Python lane runs the three suites the gate python stage runs, from a hash-pinned requirements file, on the `ctrace` core with branches and the subprocess patch (R10) | `test_coverage_report.py` `test_the_python_lane_runs_the_suites_the_gate_runs`, `test_coverage_py_is_hash_pinned_on_the_ctrace_core` |
| A9 | the root `vitest.config.ts` coverage block uses the v8 provider, stays off unless asked, includes every app source file and reports on failure, and the catalog pins the provider beside vitest (R9) | `test_coverage_report.py` `test_the_vitest_coverage_block_stays_off_unless_asked`, `test_the_catalog_pins_the_provider_beside_vitest` |
| A10 | the requirements file has its update path: one Dependabot `pip` entry for `/tools/coverage`, weekly into dev with a cooldown and the major-version ignore (R11) | `test_coverage_report.py` `test_the_requirements_file_has_a_dependabot_update_path` |
| A11 | a Dependabot pull request that changes only `tools/coverage/requirements.txt` passes `fragment` and says so, alone and in the census of every file an ecosystem updates in this tree, pip read from `/tools/coverage`; the same file beside a source file, the same file changed by a person, and the two paths that only resemble it, `tools/coverage/requirements.txt.bak` and `tools/coverage/x/requirements.txt`, are each refused with the message of before (R11, R12) | `test_changelog_fragment.py` `test_a_dependabot_pull_request_that_changes_only_dependency_files_needs_no_fragment`, `test_every_file_an_ecosystem_updates_in_this_tree_passes_alone`, `test_a_dependabot_pull_request_that_changes_another_path_is_refused`, `test_every_other_author_reads_as_before` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_a_lane_that_examined_no_line_reads_void
A1: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_one_examined_line_is_a_measured_report
A2: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_a_planted_uncovered_changed_line_is_printed_by_file_and_line
A2: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_every_figure_is_an_integer_count
A3: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_a_provider_locked_at_another_version_reads_void
A3: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_committed_lockfile_locks_the_provider_at_the_vitest_version
A4: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_an_unreadable_lcov_reads_void
A5: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_plan_runs_only_the_lanes_a_diff_touches
A5: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_a_dev_push_and_the_release_pull_request_run_every_lane
A6: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_every_exclusion_is_one_row_with_its_reason
A6: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_rust_run_flags_come_from_the_rows
A6: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_no_inline_hint_without_a_row
A6: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_every_row_matches_a_tracked_file
A7: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_no_needs_and_no_ruleset_names_a_coverage_job
A7: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_coverage_workflow_is_read_only_bounded_and_pinned
A7: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_each_lane_runs_its_planted_controls_first
A7: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_a_planted_case_that_misreads_makes_the_controls_void
A8: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_python_lane_runs_the_suites_the_gate_runs
A8: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_coverage_py_is_hash_pinned_on_the_ctrace_core
A9: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_vitest_coverage_block_stays_off_unless_asked
A9: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_catalog_pins_the_provider_beside_vitest
A10: python3 -m unittest discover -s scripts/tests -p test_coverage_report.py -k test_the_requirements_file_has_a_dependabot_update_path
A11: python3 -m unittest discover -s scripts/tests -p test_changelog_fragment.py -k test_a_dependabot_pull_request_that_changes_only_dependency_files_needs_no_fragment
A11: python3 -m unittest discover -s scripts/tests -p test_changelog_fragment.py -k test_every_file_an_ecosystem_updates_in_this_tree_passes_alone
A11: python3 -m unittest discover -s scripts/tests -p test_changelog_fragment.py -k test_a_dependabot_pull_request_that_changes_another_path_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_changelog_fragment.py -k test_every_other_author_reads_as_before
```

The behaviour tests (A1 to A5, and the misread case of A7) drive `scripts/coverage-report.py` as a
child process over planted files in a temporary directory: lcov files, a unified diff, a
changed-path list and a lockfile. The census tests (the rest of A6 to A10) read the named files of
the tree; two of them, the inline-hint census and the tracked-file match, list the whole tree and
run in CI only. The A11 tests run the `fragment` step of `changelog.yml` itself over planted pull
requests, in CI only. A1, A2, A3, A4, A5 and A7 are red at the red commit by an assertion against a
stub report that keeps every argument and lacks each behaviour, and A11 is red there by an
assertion against the step as it was, which refuses the requirements file; A6, A8, A9 and A10 pin
properties their files hold from their first commit, each with planted controls that show the
census refuses the forbidden shape.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-592-an-advisory-coverage-report-runs-on-every-pull-request-and-dev-push-and-never-gates-a-merge.md` | docs | added |
| `docs/decisions/ADR-613-coverage-runs-in-its-own-workflow-as-an-advisory-report-and-never-gates-a-merge.md` | docs | added |
| `docs/schematics/coverage-report.md` | docs | added |
| `docs/red-first/SPEC-592.md` | docs | added |
| `changelog.d/coverage-report-592.md` | docs | added |
| `.github/workflows/coverage.yml` | ci | added |
| `.github/dependabot.yml` | ci | changed: one weekly pip entry for the coverage requirements file |
| `.github/workflows/changelog.yml` | ci | changed: one exact path, the coverage requirements file, in the Dependabot arm of the `fragment` step |
| `config/coverage.json` | config | added: the one exclusion list and the lane globs |
| `scripts/coverage-report.py` | scripts | added: `plan`, `provider`, `args`, `controls` and `report` |
| `tools/coverage/requirements.txt` | python lane | added: one exact pin with its hashes |
| `tools/coverage/coverage.toml` | python lane | added |
| `scripts/tests/test_coverage_report.py` | tests | added |
| `scripts/tests/test_changelog_fragment.py` | tests | changed: the pip ecosystem and its requirements file in the fragment census, and the four planted cases of the pip arm |
| `scripts/tests/fixtures/coverage/examined-none.lcov` | tests | added: a planted empty lane |
| `scripts/tests/fixtures/coverage/unclosed.lcov` | tests | added: a planted unreadable lcov |
| `scripts/tests/fixtures/coverage/uncovered.lcov` | tests | added: a planted uncovered line |
| `scripts/tests/fixtures/coverage/uncovered.diff` | tests | added: the diff that adds it |
| `scripts/tests/fixtures/coverage/provider-mismatch.txt` | tests | added: a planted mixed lock |
| `scripts/mutation-python.json` | mutation | changed: one entry, the new script and its test module |
| `scripts/mutation-verdict.py` | mutation | changed: the scoped price table, each line regenerated from one price run over every population file |
| `scripts/mutation-equivalent.d/python.json` | mutation | changed: fourteen equivalence records for the coverage-report survivors no test can observe, each bound to one mutant |
| `scripts/tests/fixtures/mutation-scoped-prices-41760230352/` | mutation | added: that run's cost records, one `costs.jsonl` per price leg |
| `scripts/tests/fixtures/mutation-scoped-prices-39439842824/` | mutation | removed: the earlier run's records, as the table test holds exactly one run |
| `scripts/mutation-rows.d/S42100-S42199.json` | rows | changed: the fifteen table rows re-pinned on their regenerated lines |
| `scripts/mutation-rows.d/S59200-S59299.json` | rows | added |
| `scripts/mutation-rows.d/S19000-S19099.json` | rows | changed: S19063 re-anchored on the arm line it pins |
| `vitest.config.ts` | web | changed: a coverage block, off unless asked |
| `package.json` | web | changed: one development dependency, from the catalog |
| `pnpm-workspace.yaml` | web | changed: one catalog entry beside vitest |
| `pnpm-lock.yaml` | web | changed by the pinned pnpm only |

## 5. What this does NOT cover

- A Swift lane: the iPhone and iPad app tests, its packages and its harness join as a second wave
  after the harness job split (#1120); no Swift lane, no change to `xcframework.yml` or any `apple-*`
  workflow, and no `ios/` path is in this delivery.
- A coverage threshold, a ratchet, a required coverage check and a pull request comment (#1122),
  which ADR-613 D1 and D5 refuse.
- The removal of the deprecated Telegram code (#1122); its rows leave the list with that removal
  delivery (ADR-589).

Tracked in #1122: no change here touches `ci.yml`, a ruleset, a crate, the web app source, the
engine or any credential.

## 6. Risks

- **A lane outlasts its bound.** The bounds are set from the gate jobs of section 1, each measured
  with its job id; the delivery own run measures each lane, and a later change raises a bound only
  with the run that needs it.
- **A test that traces fails under the ctrace core.** The lane still exits 0 and prints each suite
  exit; `ci` judges the test, and the lane figures for that file read lower.
- **A tree census reads a new directory.** `config/coverage.json`, `tools/coverage/` and
  `scripts/tests/fixtures/coverage/` are new; the build lists every `scripts/tests` module that names
  `config`, `tools` or the fixtures tree before its push, and reads their CI verdicts by name.
- **The provider brings a dependency with an advisory.** The `dependency-review` check and the web
  audit refuse it on the pull request, and the advisories rule decides the cure (a fixed release, an
  override or our own fixed copy), never an ignore list and never the coverage figure.
- **The install action manifest lacks the pinned tool version.** With no fallback, the Rust lane
  fails at install, in view, and reads no figure.
- **A Dependabot update leaves the hash lines behind.** The Python lane install refuses a release
  whose hashes the file lacks, so that update pull request shows the lane failing at install, in
  view, before it merges.
- **An update writes a second file for the pip entry.** The arm admits the one requirements file, so
  a Dependabot pull request that also writes another file reads `fragment` red, in view, and takes a
  fragment by hand until an amendment names that path.
- **The workflow audit reads the new file.** `workflow-lint` runs zizmor over every workflow; the new
  file copies the shapes `ci.yml` already passes with (pinned uses, no persisted credentials, every
  context through `env:`).

## 7. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani (ADR-613 D6). The delivery adds a workflow, a data file, a
standard-library script and tests; no protocol, state machine, interleaving or engine path gains
or changes, and no Rust source changes.

## 8. The lines the report cannot measure

- Swift, until its wave (#1120).
- Rust doctests, which the instrumented run does not build, and code compiled for another target
  only.
- Python lines run only by a child process whose environment drops the coverage variables, or
  that starts Python without its site packages: they read as not executed, and the first dev push
  report counts the files left at zero executed lines.
- Code generated into the build directory, which lies outside every lane glob.

## Amendment (SPEC-649): a test's child exits on its own before any kill, so its raw profile is whole

The Rust lane's merge refuses every profile of a run when one raw profile is cut short, and a
test's child process writes its raw profile in its exit handlers. A test that stops a child it
started, as the sync server helpers of the ingest, engine-core and ffi test support do, closes the
child's standard input, waits a bounded grace for the child to exit on its own, and kills it only
past that grace; it never kills a child at once. So every child's raw profile is written whole and
counted, and a profile cut short for any other reason still fails the lane as a broken instrument
(ADR-613 D2). The merge's failure mode stays the one that refuses: no lane flag drops, skips or
ignores a profile, and no step re-runs the lane to hide its first result.

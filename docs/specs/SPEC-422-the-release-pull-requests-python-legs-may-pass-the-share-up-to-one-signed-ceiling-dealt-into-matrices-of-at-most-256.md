# SPEC-422: the release pull request's Python legs may pass the share up to one signed ceiling, dealt into matrices of at most 256

Path:
`docs/specs/SPEC-422-the-release-pull-requests-python-legs-may-pass-the-share-up-to-one-signed-ceiling-dealt-into-matrices-of-at-most-256.md`.

- **Issue:** #822, the release pull request from `dev` into `main`. This SPEC refers to it and
  closes nothing. It also refers to #825, the Python legs' fit.
- **Context(s):** the mutation scripts (`scripts/`), their tests and fixtures, `ci.yml`, one new
  dispatch-only workflow, the release runbook and one formal model's comments. No crate changes.
- **Decided by:** ADR-436 (the release run is named by a fail-closed predicate, and its Python legs
  pass the share only up to one signed ceiling, dealt over two matrices of at most 256). ADR-373,
  ADR-434, SPEC-362 and SPEC-290 are amended insert-only. SPEC-415 R2 and SPEC-418 R4 are the
  rules the release-run exception narrows; neither is amended.
- **Schematic:** `docs/schematics/mutation-testing-python.md` section 9, added insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests, the measurement
  run's committed record and `docs/red-first/SPEC-422.md`. The ceiling's figure is set by a later,
  separate commit of the same pull request, after the release's final union is measured.

## 1. The problem, measured

Every figure below is read from the tree at dev `26ecc2ad`, by the command beside it.

| fact | value | command |
|---|---|---|
| the share, the most Python legs a run deals | 32 | `grep -n '^PYTHON_MAX_SHARDS = ' scripts/mutation-verdict.py` reads `125:` |
| the cap no signed share may pass | 128 | `grep -n '^PYTHON_SHARE_CAP = ' scripts/mutation-verdict.py` reads `127:` |
| a Python leg's bound | 1800 s | `grep -n '^PYTHON_LEG_BOUND_SECONDS = ' scripts/mutation-verdict.py` reads `130:` |
| the Rust leg ceilings | `{"ci": 190, "battery": 234}` | `grep -n '^LEG_CEILING = ' scripts/mutation-verdict.py` reads `925:` |
| the jobs a matrix generates | 256 | `grep -n '^RUN_JOBS = ' scripts/tests/test_dispatch_shards.py` reads `218:` |
| the cost deal's refusal past the share | one, by name | `grep -n 'count + unpriced_legs > PYTHON_MAX_SHARDS' scripts/mutation-verdict.py` reads `1161:` |
| the count rule's refusal past the share | one, by name | `grep -n "the pull request run's {PYTHON_MAX_SHARDS} python legs" scripts/mutation-verdict.py` reads `1224:` |
| the Python matrix jobs in `ci.yml` | 1, `mutation-python` | `grep -n -E '^  mutation-python[a-z-]*:$' .github/workflows/ci.yml` reads `543:` alone |
| the legs `legs` judges | 2, `mutation-rust` and `mutation-rows` | `sed -n '2196,2203p' scripts/mutation-verdict.py` names the two, in `legs` (`:2186`) |
| the skips `ci` admits | 2 | `grep -n 'for leg in "mutation-rust' .github/workflows/ci.yml` reads `982:` |
| a workflow that measures the limit | 0 | `git ls-tree --name-only HEAD .github/workflows/ \| grep -c matrix-limit` reads 0 |

So every run, the release pull request's included, refuses a Python listing whose legs pass 32,
whole and by name, by the cost deal when a listed file is priced (SPEC-418 R4) and by the count
rule when none is (SPEC-415 R2). A release's listing is every Python mutant `dev` gained since the
last release, so a release that needs more legs than the share cannot be judged at all.

An owner-signed ruling admits the release pull request's run alone above `PYTHON_SHARE_CAP`, up to
one ceiling: the figure measured on the release's final union, with every mutant run, every shard
counted and no skipped-mutant record. Pull requests into `dev` keep the share 32 and the cap 128.
Two facts bound the answer. A matrix generates at most 256 jobs, and ADR-373 D1 read that limit
per run, which no workflow of this repository has measured (the last row above). And this
delivery's own lines add Python mutants, so no figure read before its last code commit is the
release's.

## 2. Requirements

R1. **The release run is named by a fail-closed predicate.** `release_run(event, base_ref,
    head_ref, head_repo, repo) -> bool` in `scripts/mutation-verdict.py` is true exactly when
    `event == "pull_request"`, `base_ref == "main"`, `head_ref == "dev"`, `repo != ""` and
    `head_repo == repo`, and false for every other input, an empty or missing one included.
R2. **The plan records it only when it holds.** The `plan` verb takes `--head-ref`, `--head-repo`
    and `--repo`: `--head-repo` and `--repo` default to the empty string and `--head-ref` to no
    value, which R1 reads as false exactly as it reads an empty one. `plan_diff` writes the
    top-level key `"release": true`
    exactly when R1 holds, and writes no `release` key otherwise. `ci.yml`'s plan step passes the
    head ref, the head repository's full name and this repository's full name through `env:` as
    `HEAD_REF`, `HEAD_REPO` and `REPO`, and its command reads them as `"$HEAD_REF"`,
    `"$HEAD_REPO"` and `"$REPO"`, never as expressions in the command's text.
R3. **Only `True` is the release run.** `shards` treats a plan as the release run exactly when
    `plan.get("release") is True`. A missing key, `false`, the string `"true"`, `1` or `null` is not
    the release run.
R4. **One ceiling constant.** `PYTHON_RELEASE_CEILING = 128` directly follows the
    `PYTHON_SHARE_CAP` line, with one `#:` comment line directly above it. A helper returns the
    ceiling only for a release plan whose ceiling is above `PYTHON_SHARE_CAP`, and None otherwise.
    Under None, `shards` deals, refuses and prints exactly as it does for a pull request into
    `dev`: at the constant's placeholder value the release run is the pull request run.
R5. **A ceiling outside its range is refused by name.** On every run, before any deal, beside
    `python_share_refused`, a `PYTHON_RELEASE_CEILING` that is not an `int`, is below
    `PYTHON_SHARE_CAP`, or is above `PYTHON_MATRIX_LEGS * PYTHON_MATRICES` makes `shards` print
    `mutation: shards: REFUSED: the release ceiling PYTHON_RELEASE_CEILING = {ceiling} is outside
    {PYTHON_SHARE_CAP}..{PYTHON_MATRIX_LEGS * PYTHON_MATRICES}; a ceiling outside it is never
    dealt` and exit 1 before any leg is dealt.
R6. **The release run deals up to its ceiling and refuses past it whole.** Under a ceiling,
    `python_by_cost(listing, prices, scoped, ceiling)` holds the priced and unpriced legs to the
    ceiling in place of `PYTHON_MAX_SHARDS`. Past it, it returns
    `mutation: shards: REFUSED: {len(listing)} python mutant(s) need {count} leg(s) priced at most
    {PYTHON_LEG_BOUND_SECONDS} s each and {unpriced_legs} unpriced leg(s) of at most
    {PYTHON_SHARD_MUTANTS}, more than the release run's {ceiling} python legs
    (PYTHON_RELEASE_CEILING): a larger figure needs a new measurement and a new owner-signed
    ruling, since a run is never capped`, and `shards` exits 1. A deal within it prints
    `mutation: shards: release run: {n} python leg(s) of ceiling {ceiling}
    (PYTHON_RELEASE_CEILING): {first} in mutation-python and {second} in
    mutation-python-release, at most {PYTHON_MATRIX_LEGS} a matrix`. Under None, the share's check
    and its refusal text are byte for byte as before.
R7. **The share holds wherever the ceiling does not apply.** A plan that is not the release run
    keeps the share and both refusals at any ceiling. A release plan whose listing has no priced
    file is dealt by the count rule (`python_shards`) and keeps the share and SPEC-415's refusal,
    at any ceiling.
R8. **The legs split into matrices of at most 256.** `PYTHON_MATRIX_LEGS = 256` and
    `PYTHON_MATRICES = 2`, each with its `#:` line. For a deal of n legs, `shards` writes the step
    outputs `python_shards={n}`, `python_matrix=` the JSON list of 0 to min(n,
    `PYTHON_MATRIX_LEGS`) - 1, `python_release_matrix=` the JSON list of `PYTHON_MATRIX_LEGS` to
    n - 1 (`[]` when there is none) and `python_release_legs={max(0, n - PYTHON_MATRIX_LEGS)}`.
    Each shard from 0 to n - 1 is in exactly one of the two lists. `mutation-plan` exports both new
    outputs.
R9. **The second matrix is the first on the release axis.** `ci.yml`'s new job
    `mutation-python-release` equals `mutation-python` in every key and step but two: the
    job-level condition `if: ${{ needs.mutation-plan.outputs.python_release_legs != '0' }}` and
    the axis `shard: ${{ fromJSON(needs.mutation-plan.outputs.python_release_matrix) }}`. Each of
    its legs runs `--shard "$SHARD/$SHARDS"` over the total and keeps its report as
    `mutation-python-shard-<k>`. `PYTHON_MATRICES` equals the number of `ci.yml` jobs whose axis
    reads a Python matrix output.
R10. **Every leg of both matrices is judged.** `mutation-verdict` and `ci` each need
    `mutation-python-release`. The verdict's download of `mutation-python-shard-*` and
    `python_reports`' count of shards 0 to n - 1 are unchanged and read both matrices.
R11. **The second matrix's skip is admitted once and judged.** `ci` admits a `skipped` result from
    `mutation-python-release` once, beside `mutation-rust` and `mutation-rows`, through
    `"mutation-python-release:$PYTHON_RELEASE_LEG"` in its loop. The verdict's `legs` call passes
    `--python-release-leg "$PYTHON_RELEASE_LEG"` after its existing arguments. `legs` owes the leg
    exactly when the plan's Python count passes `PYTHON_MATRIX_LEGS`. A started leg prints
    `mutation-python-release: started, <result>`, as the other two do; owed and `skipped`, or owed
    with no `--python-release-leg`, is VOID by name; not owed and `skipped` prints
    `mutation-python-release: not started: the plan deals no python leg past PYTHON_MATRIX_LEGS`;
    not owed with no argument prints nothing, so every earlier output stays byte for byte.
R12. **Each Python matrix fits.** A test in `scripts/tests/test_dispatch_shards.py` derives each
    Python matrix's most jobs on the release run and holds each at most `RUN_JOBS`, and holds
    `PYTHON_MATRIX_LEGS == RUN_JOBS`. Its existing derivation counts the release axis at
    max(0, `PYTHON_MAX_SHARDS` - 256), so `LEG_CEILING` derives unchanged.
R13. **The tree cites the signed ruling.** At `PYTHON_SHARE_CAP` the ceiling's `#:` line cites no
    path under `docs/rulings/`. Above it, the line cites exactly one `docs/rulings/<file>.md`, that
    file is in the tree, and its first line holds the ceiling's value as a whole word and the word
    `release`. No test and no mutation row pins the ceiling's value.
R14. **The per-matrix reading is measured in public.** `.github/workflows/matrix-limit.yml` runs on
    `pull_request` into `dev` when its own file changes (two legs a matrix) and on
    `workflow_dispatch` with one input, `legs`, from 1 to 256. Its token reads contents only. Its
    jobs are `size` (writes the two axes from `legs`), `first` and `second` (one matrix each, every
    leg keeping an artifact `matrix-limit-<matrix>-<leg>` that holds its matrix, its leg and
    `strategy.job-total`), `empty` (a job whose condition is false over `[]`) and `count` (needs
    every other job, runs `if: ${{ always() }}`, downloads `matrix-limit-*` merged, and uploads
    `matrix-limit.json`: each matrix's reported legs and job totals, every job's result and the
    run's job count).
R15. **The measurement's record is committed and read.** One dispatch at `legs` 256 is recorded as
    `scripts/tests/fixtures/matrix-limit-<run>/`, and a test reads it: each matrix reported 256
    legs, each leg read a job total of 256, `first` and `second` succeeded, `empty` was skipped,
    and the run held more than 256 jobs. A planted copy less one leg fails the same reading.
R16. **The runbook says the method.** `RELEASING.md` gains one sentence after line 73 and one
    paragraph between line 98 and line 100, insert-only, naming `PYTHON_RELEASE_CEILING`,
    `mutation-python-release` and `PYTHON_MATRIX_LEGS`, saying a release listing past the ceiling
    is "refused whole, by name, never capped" and that a larger figure needs "a new measurement
    and a new signature". They hold no number outside a SPEC or ADR id.
R17. **What stays.** `PYTHON_MAX_SHARDS`, `PYTHON_SHARE_CAP`, `PYTHON_LEG_BOUND_SECONDS` and
    `LEG_CEILING` are each assigned on one line of `scripts/mutation-verdict.py`, byte for byte as
    at the base, `RUN_JOBS` is unchanged, and every new line reads each by name.

## 3. Acceptance criteria of SPEC-422

Every test is in `scripts/tests/test_release_python_ceiling.py` unless its row names another module.

| id | criterion | decided by |
|---|---|---|
| A1 | `release_run` is true for the one release input, and each of its five conditions alone, falsified, makes it false; empty inputs read false | `test_release_run_holds_only_for_this_repositorys_dev_into_main` |
| A2 | the plan writes `"release": true` for the release input and no `release` key for a pull request into `dev`, a push, a schedule or a dispatch | `test_the_plan_records_release_only_when_the_predicate_holds` |
| A3 | a plan whose `release` is missing, `false`, `"true"`, `1` or `null` is dealt and refused as a pull request plan under a set ceiling, while `true` is dealt as the release | `test_a_release_mark_that_is_not_true_is_not_the_release_run` |
| A4 | the plan step passes `HEAD_REF`, `HEAD_REPO` and `REPO` through `env:` and reads them by name in its command | `test_the_plan_step_passes_head_ref_head_repo_and_repo_by_env` |
| A5 | at the placeholder ceiling, a release plan's outputs and printed lines equal the pull request plan's golden lines, while a set ceiling changes them | `test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run` |
| A6 | a ceiling that is not an `int`, below the cap, or above `PYTHON_MATRIX_LEGS * PYTHON_MATRICES` is refused by R5's text before any deal; the cap and the product are admitted | `test_a_ceiling_outside_its_range_is_refused_by_name` |
| A7 | a release listing that needs exactly the ceiling's legs is dealt and prints R6's release line; one more leg is refused whole by R6's text, exit 1 | `test_the_release_run_deals_up_to_the_ceiling_and_refuses_one_leg_past_it` |
| A8 | a plan that is not the release run keeps the share and its refusal text under a set ceiling | `test_a_run_that_is_not_the_release_keeps_the_share_at_any_ceiling` |
| A9 | a release listing with no priced file keeps the share and SPEC-415's refusal under a set ceiling, while the same listing priced is dealt | `test_the_count_rule_keeps_the_share_on_the_release_run` |
| A10 | for n of 1, 256, 257 and 300 legs the four outputs are R8's, and every shard is in exactly one list | `test_the_release_legs_split_into_matrices_each_shard_once` |
| A11 | `mutation-python-release` equals `mutation-python` but for R9's condition and axis | `test_the_release_job_is_the_python_job_on_the_release_axis` |
| A12 | `legs` reads R11's five cases: started, owed and skipped, owed with no argument, not owed and skipped, not owed with no argument | `test_legs_judges_the_release_leg_against_the_plan` |
| A13 | `ci` needs and admits the release leg's skip once, and no other skip | `test_not_started_legs.py` `test_ci_admits_a_not_started_leg_and_no_other_skip` and `test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need`, over the widened `LEGS` |
| A14 | `mutation-verdict` needs both matrix jobs, its download pattern names both jobs' artifacts, its `legs` call passes `--python-release-leg`, and `python_reports` examines all 300 shards of a 300-leg plan | `test_the_verdict_counts_every_shard_of_both_matrices` |
| A15 | each Python matrix of the release run generates at most `RUN_JOBS`, and `PYTHON_MATRIX_LEGS == RUN_JOBS` | `test_dispatch_shards.py` `test_each_python_matrix_of_the_release_run_fits_the_run_jobs` |
| A16 | R13: one ceiling assignment with its `#:` line; no ruling path at the cap; above it one path, present, whose first line holds the value and `release`; four planted sources each refused | `test_the_ceiling_cites_the_signed_ruling_only_above_the_cap` |
| A17 | `PYTHON_MATRICES` equals the `ci.yml` jobs whose axis reads a Python matrix output | `test_python_matrices_equals_the_workflows_python_matrix_jobs` |
| A18 | `matrix-limit.yml` has R14's triggers, token, jobs, needs, conditions, axes and artifacts | `test_the_matrix_limit_workflow_measures_two_matrices_in_one_run` |
| A19 | the committed record reads R15's six facts, and the planted copy less one leg is refused | `test_the_recorded_matrix_limit_run_ran_each_matrix_whole` |
| A20 | the runbook holds R16's sentence and paragraph at their places, with no number outside a SPEC or ADR id | `test_the_runbook_names_the_release_ceiling_its_job_and_its_refusal` |
| A21 | R17: each of the four values has one assignment line and every new function reads it by name | `test_the_release_run_assigns_no_share_cap_bound_or_ceiling_of_its_own` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_release_run_holds_only_for_this_repositorys_dev_into_main
A2: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_plan_records_release_only_when_the_predicate_holds
A3: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_a_release_mark_that_is_not_true_is_not_the_release_run
A4: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_plan_step_passes_head_ref_head_repo_and_repo_by_env
A5: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run
A6: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_a_ceiling_outside_its_range_is_refused_by_name
A7: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_run_deals_up_to_the_ceiling_and_refuses_one_leg_past_it
A8: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_a_run_that_is_not_the_release_keeps_the_share_at_any_ceiling
A9: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_count_rule_keeps_the_share_on_the_release_run
A10: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_legs_split_into_matrices_each_shard_once
A11: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_job_is_the_python_job_on_the_release_axis
A12: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_legs_judges_the_release_leg_against_the_plan
A13: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_not_started_leg_and_no_other_skip
A13: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need
A14: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_verdict_counts_every_shard_of_both_matrices
A15: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_each_python_matrix_of_the_release_run_fits_the_run_jobs
A16: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_ceiling_cites_the_signed_ruling_only_above_the_cap
A17: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_python_matrices_equals_the_workflows_python_matrix_jobs
A18: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_matrix_limit_workflow_measures_two_matrices_in_one_run
A19: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_recorded_matrix_limit_run_ran_each_matrix_whole
A20: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_runbook_names_the_release_ceiling_its_job_and_its_refusal
A21: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_release_run_assigns_no_share_cap_bound_or_ceiling_of_its_own
```

A18, A19 and A21 are not red first. The workflow is a new file, whose absence is not its behaviour
(A18). The record is the measurement itself, committed after the dispatched run; its planted copy
less one leg is its control, and the record reads the control refused (A19). The base already
assigns each of the four values once and reads it by name (A21). `docs/red-first/SPEC-422.md`
records each as `not red`, with that reason. Every other criterion is red at the red commit, on an
assertion of its behaviour: the red commit carries stubs that accept every new argument and
ignore it (`release_run` returns False, the `plan` and `legs` verbs take and drop their new flags,
and `PYTHON_MATRIX_LEGS` and `PYTHON_MATRICES` exist), and no `PYTHON_RELEASE_CEILING`. A test
that sets a ceiling patches it with `mock.patch.object(..., create=True)`, so it reaches its
behavioural assertion at the red commit. A3, A5, A8 and A9 are each red on their positive arm (a
`true` mark, a set ceiling, a release context or a priced listing that the base does not deal),
which is the behaviour each criterion's negative arms are measured against. A16 is red on its
first assertion: the base holds no `PYTHON_RELEASE_CEILING` assignment with its `#:` line.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/mutation-verdict.py` | the plan and the verdict | `release_run`; `PYTHON_RELEASE_CEILING`, `PYTHON_MATRIX_LEGS`, `PYTHON_MATRICES` with their `#:` lines; the ceiling helper and the range refusal; `python_by_cost`'s `ceiling` argument, release refusal and release line; `plan_diff`'s `release` key; `shards`' two new outputs and its `python_matrix` bound; `legs`' optional arm; the `--head-ref`, `--head-repo`, `--repo` and `--python-release-leg` arguments and the usage text |
| `scripts/mutation-python.json` | the map | the `scripts/mutation-verdict.py` entry gains `test_release_python_ceiling`, so the tests that observe the new lines judge their mutants; its six modules stay |
| `.github/workflows/ci.yml` | the workflow | the plan step's `HEAD_REF`, `HEAD_REPO`, `REPO` and their flags; `mutation-plan`'s outputs `python_release_matrix` and `python_release_legs`; the new job `mutation-python-release`; the needs of `mutation-verdict` and `ci`; `ci`'s admission; the `legs` call's flag and its `PYTHON_RELEASE_LEG` |
| `.github/workflows/matrix-limit.yml` | the measurement | added (R14) |
| `scripts/tests/fixtures/matrix-limit-<run>/` | the measurement's record | added in the later push, after the dispatched run (R15) |
| `scripts/tests/test_release_python_ceiling.py` | the tests | added: A1 to A12, A14, A16 to A21 |
| `scripts/tests/test_dispatch_shards.py` | the job budget | `generated()` counts the release axis at max(0, `PYTHON_MAX_SHARDS` - 256); A15 added; `RUN_JOBS` (`:218`) unchanged |
| `scripts/tests/test_not_started_legs.py` | the admitted legs | `LEGS` (`:27`) widened to three, every old member kept; a condition scenario set for the third |
| `scripts/tests/test_mutation_python_workflows.py` | a pin | the verdict's needs pattern (`:83`) widened to name `mutation-python-release`, every old member kept |
| `scripts/tests/test_mutation_workflows.py` | a pin | the verdict's needs pattern (`:410`) widened the same way |
| `scripts/tests/test_ci_workflows.py` | a census and a pin | `NOT_WORKFLOW_READS` and `DYNAMIC_IMPORTS` gain one entry for each read and import site the new and edited test modules add, each with its exact count and reason; the gate test's `mutation` list of `ci`'s needs gains `mutation-python-release`, every old member kept; every old entry and every test kept |
| `scripts/tests/test_verdict_folds.py` | a pin | the verdict step's driver gives `mutation-python-release` the result `skipped`, and `JOB_AT`'s needs gain `mutation-python-release`, every old member kept |
| `scripts/tests/test_verdict_download.py` | a pin, read | unchanged: its `FAMILIES` pattern for `mutation-python-shard-*` already names the release legs' artifacts |
| `scripts/tests/test_rulesets.py` | a pin, read | unchanged: its reading of `RELEASING.md` section 2 holds over the two inserts |
| `scripts/mutation-rows.d/S42200-S42299.json` | the rows | added: S42200 to S42218 (section 9) |
| `formal/tla/EveryLegCounted/EveryLegCounted.tla` | the formal record | `\*` comment lines and the two `digest=` values of `shards` and `python_by_cost` (section 8) |
| `RELEASING.md` | the runbook | insert-only (R16) |
| `docs/specs/SPEC-422-the-release-pull-requests-python-legs-may-pass-the-share-up-to-one-signed-ceiling-dealt-into-matrices-of-at-most-256.md` | this SPEC | added |
| `docs/decisions/ADR-436-the-release-run-is-named-by-a-fail-closed-predicate-and-its-python-legs-pass-the-share-only-up-to-one-signed-ceiling-over-two-matrices.md` | the decision | added |
| `docs/decisions/ADR-373-a-release-is-judged-in-one-run-of-round-robin-legs-sized-to-the-hosted-job.md` | an amendment | insert-only, after its last line |
| `docs/decisions/ADR-434-the-python-share-is-one-capped-value-the-price-table-is-derived-from-a-recorded-run-and-each-mutant-runs-the-tests-its-line-traced.md` | an amendment | insert-only, after its last line |
| `docs/specs/SPEC-362-a-releases-mutants-are-judged-in-one-run-whose-legs-fit-the-run.md` | an amendment | insert-only, after its last line |
| `docs/specs/SPEC-290-a-leg-with-nothing-to-examine-is-not-started.md` | an amendment | insert-only, section 10 after its last line |
| `docs/schematics/mutation-testing-python.md` | the schematic | insert-only, section 9 after its last line |
| `docs/red-first/SPEC-422.md` | the red-first record | added |
| `changelog.d/release-python-ceiling-422.md` | the changelog | added |

`scripts/mutation_python.py` is not changed: its `--shard k/n` already reads any k below n. The
owner-signed ruling file the ceiling cites is not this delivery's file: it is committed to `dev`
on its own, and this pull request takes it from `dev` before it lands.

## 5. What this does NOT cover

- It does not set the ceiling's figure: the figure is read on the release's final union and set by
  its own later commit, and #822 is the release it is read for.
- It does not raise the share or the cap for any run, or move the Rust share, #825.
- It does not add a verdict across runs, or a third matrix job, #691.
- It does not change which head reaches `main`: `base-is-dev` and ADR-034's model stand, #691.
- It does not lengthen any leg or change any leg's bound, #825.
- It does not sample, skip or exempt any mutant, and adds no skipped-mutant record, #822.
- It does not amend SPEC-415 or SPEC-418: their refusals hold on every run but the release run
  under a set ceiling, which ADR-436 D2 names, #822.

## 6. Risks

- The 256-job limit is counted per run. Then A19 fails on the committed record, and the delivery
  does not land: the two-matrix design fails closed.
- A fork names its branch `dev`. R1 compares the head repository with this one, so the fork's run
  is not the release run.
- The event carries no head ref or head repository. R1 reads false, and the release run deals as a
  pull request run, which refuses past the share: closed, not open.
- `mutation-plan` writes no `python_release_legs`. The empty output is not `'0'`, so the release
  job starts with no axis and fails, and `ci` reads it: red, never a silent pass.
- On the release run the existing joint line still prints the share as its Python denominator.
  The release line beside it states the ceiling, and neither line is a verdict.
- A later release needs more legs than the signed ceiling. R6 refuses it whole, by name, and it
  waits for a new measurement and a new signature.
- A re-indent under `if ceiling is None:` moves a dev row's anchor. Every row on a touched file is
  censused at each commit; S41800's anchor stays a substring by construction.
- The measurement run uploads one artifact a leg. `count` downloads them merged, and its own
  record is the one artifact the test reads.

## 7. What only CI proves

- Every red and every green is read in CI. `scripts/tests/test_dispatch_shards.py` (A15) runs only
  there.
- The measurement is one dispatch of `matrix-limit.yml` at 256 legs a matrix, at this delivery's
  branch, after its `pull_request` run has registered the file. Its record (R15) is committed in a
  later push, and its job ids appear in the record beside each job's duration.
- The release pull request's own run, after this delivery lands, must reproduce the signed figure
  in R6's release line.

## 8. Formal model

`formal/tla/EveryLegCounted` covers `shards` and `python_by_cost` in `scripts/mutation-verdict.py`,
and this delivery edits both. Each hunk is re-read against the model and recorded in its `\*`
comments by item name (`mutation-verdict.py::shards`, `mutation-verdict.py::python_by_cost`), and
the two `digest=` values move to the digest rule's value at the head, with the base's recorded
values as the rule's control. No variable, action or property changes: the release deal changes
which bound holds the count, not the partition of listed mutants over legs; the second matrix's
legs are the model's per-leg actions for shards 256 and up; and an owed leg that is skipped leaves
a leg with no report, which the model already reads as void. No new model is written, and no
model checker runs: this repository's CI has no formal job.

## 9. Mutation rows (S42200-S42299)

Rows S42200 to S42218, in `scripts/mutation-rows.d/S42200-S42299.json`. Each find is taken from
the green commit's text and occurs exactly once in its file; none sits on a ceiling constant or its
`#:` line.

| row | file | find (the text the mutant changes) | mutant | killer |
|---|---|---|---|---|
| S42200 | `.github/workflows/ci.yml` | the release job's `python_release_legs != '0'` | `== '0'` | A11 |
| S42201 | `.github/workflows/ci.yml` | the release job's axis `outputs.python_release_matrix` | `outputs.python_matrix` | A11 |
| S42202 | `.github/workflows/ci.yml` | `mutation-verdict`'s needs entry `mutation-python-release` | dropped | A14 |
| S42203 | `.github/workflows/ci.yml` | `ci`'s needs entry `mutation-python-release` | dropped | A13 |
| S42204 | `.github/workflows/ci.yml` | `ci`'s loop entry `"mutation-python-release:$PYTHON_RELEASE_LEG"` | dropped | A13 |
| S42205 | `.github/workflows/ci.yml` | the `legs` call's ` --python-release-leg "$PYTHON_RELEASE_LEG"` | dropped | A14 |
| S42206 | `.github/workflows/ci.yml` | the plan step's `HEAD_REPO:` value | this repository's name | A4 |
| S42207 | `.github/workflows/ci.yml` | the plan command's ` --head-ref "$HEAD_REF"` | ` --head-ref "$BASE_REF"` | A4 |
| S42208 | `scripts/mutation-verdict.py` | R6's refusal text `a new owner-signed` | `an owner-signed` | A7 |
| S42209 | `scripts/mutation-verdict.py` | R5's refusal text `is outside` | `is within` | A6 |
| S42210 | `scripts/mutation-verdict.py` | the output `python_release_legs=` | `python_release_shards=` | A10 |
| S42211 | `scripts/mutation-verdict.py` | the output `python_release_matrix=` | `python_extra_matrix=` | A10 |
| S42212 | `scripts/mutation-verdict.py` | `python_matrix=`'s bound by `PYTHON_MATRIX_LEGS` | the whole count | A10 |
| S42213 | `scripts/mutation-verdict.py` | R6's release line `in mutation-python-release` | `in mutation-python` | A7 |
| S42214 | `scripts/mutation-verdict.py` | R11's not-started text `past PYTHON_MATRIX_LEGS` | `past PYTHON_MAX_SHARDS` | A12 |
| S42215 | `RELEASING.md` | R16's sentence `PYTHON_RELEASE_CEILING` | `PYTHON_SHARE_CAP` | A20 |
| S42216 | `RELEASING.md` | R16's paragraph `a new measurement and a new signature` | `a new measurement` | A20 |
| S42217 | `.github/workflows/matrix-limit.yml` | `count`'s `if: ${{ always() }}` | `if: ${{ success() }}` | A18 |
| S42218 | `.github/workflows/matrix-limit.yml` | `second`'s axis output name | `first`'s | A18 |

The listing tool's own mutants of the new Python lines (the predicate's comparisons, the helper's
bounds, the range check's three arms, the split's bounds and the `legs` arm) are judged by the
delivery's own mutation run, and each operand of R1 and R5 has a test row in which it alone
decides the result (A1, A6).

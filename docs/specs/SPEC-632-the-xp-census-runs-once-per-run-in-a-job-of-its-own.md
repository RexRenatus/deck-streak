# SPEC-632: the xp census runs once per run, in a job of its own

- **Issue:** none. Amends SPEC-330 (R2: the release stage runs every test the test stage runs) and
  SPEC-038 (R3, the gate's job layout, and R13, the engine set), each by an insert-only section
  appended at its end. Restates the pins of SPEC-618 section 9 at the new ceiling.
  **Context(s):** the local gate (`scripts/check.sh`), the pull request workflow
  (`.github/workflows/ci.yml`), the gate's tests and the mutation dealer's leg ceiling; no crate,
  dependency or lockfile changes.
- **Decided by:** ADR-655 (the census set runs once per run in its own ci job, and the pull
  request run's Rust leg ceiling falls by that one job).
- **Schematic:** `docs/schematics/census-once-filtersets-and-jobs.md`, added.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-632.md`. **Mutation band:** none: no row is added; row `S36201` is
  re-anchored (section 8). **Models:** none.

## 1. The problem, measured

- `crates/progression/tests/xp_census.rs` is the only test file of the workspace that spawns cargo
  with a target directory of its own: at this change's base, `git grep -l -F '"--target-dir"'`
  names it alone among the `crates/*/tests/*.rs` files. It holds 52 `#[test]` attributes
  (`grep -c '#\[test\]' crates/progression/tests/xp_census.rs`).
- Its verdict does not depend on the outer profile: `fn cargo` clears the child's environment
  (`command.env_clear()`), the profile, flag and wrapper variables are scrubbed, and the nested
  `cargo check --workspace ... --target-dir` passes no `--release`.
- Yet the binary runs in both test stages: `stage_test` and `stage_test_release` in
  `scripts/check.sh` each run `-E "not ($ENGINE_TESTS)"`, so the census runs once in the `rust`
  job and again in the `release` job, and its slowest test set the floor of both stages
  (1,357 s and 1,045 s in run 41962635280).

## 2. Requirements

- **R1.** The census set is one nextest filterset, `CENSUS_TESTS` in `scripts/check.sh`, defined
  once beside `ENGINE_TESTS` and stated nowhere else:
  `CENSUS_TESTS='binary_id(=deck-streak-progression::xp_census)'`.
- **R2.** `stage_test` and `stage_test_release` run `-E "not ($ENGINE_TESTS | $CENSUS_TESTS)"`.
  nextest's filterset language takes `|` as the union of two sets and `not` as the complement, and
  a `binary_id(=<package>::<target>)` term selects one whole test binary.
- **R3.** A new stage `test-census`, in `stage_test_engine`'s form with no slice, runs
  `cargo nextest run --workspace --locked --no-fail-fast -E "$CENSUS_TESTS"` with one
  `--test <target>` per whole binary the set names, derived from `CENSUS_TESTS` by the loop
  `stage_test_engine` uses, so cargo builds that test target and no other. It checks `cargo`,
  `cargo-nextest` and then `protoc`, and `STAGES_ALL` lists it.
- **R4.** A new `ci.yml` job `census`, with no matrix and no needs, runs
  `bash scripts/check.sh test-census` on `ubuntu-24.04` within `timeout-minutes: 45`. It carries
  the engine job's checkout, toolchain, nextest and protoc steps and its Rust cache restore with no
  save, the locked fetch the census reads offline (`cargo metadata --locked --offline`), and the
  stage-log upload `check-stage-logs-census`. The aggregate `ci` needs it, right after `engine`.
- **R5.** The gate's pins follow: `OWNER_LAYOUT` gains `"census": ["test-census"]` and
  `COMPILES_RUST` gains `test-census` in `scripts/tests/test_ci_workflows.py`;
  `scripts/tests/test_check_gate.py` reads the census set once as it reads the engine set, its
  `test` split wants `not (<engine set> | <census set>)` and its `test-census` split the census set,
  its target judge wants the census set's derived targets for `test-census`, its planted check.sh
  plants both definitions, and `TOOLS` and `ENGINE_STAGES` gain `"test-census": ["cargo",
  "cargo-nextest"]`. No assertion is removed.
- **R6.** `test_census_set_spawns_its_own_cargo` holds the set's class: each binary the set names
  spawns cargo with a cleared environment and a target directory of its own, so a test that proves
  runtime behaviour in the shipped profile cannot join the set.
- **R7.** The pull request run's Rust leg ceiling is 184 (section 8).

## 3. Acceptance criteria of SPEC-632

Each command runs from the repository root. A1 to A5 and A8 to A10 run on any checkout; A6 and A7
run in CI's `hygiene` job, whose python stage runs every module.

| id | criterion | decided by |
|---|---|---|
| A1 | `stage_test` and `stage_test_release` exclude `CENSUS_TESTS`, `stage_test_census` runs it alone, and the `census` job runs `test-census` and is a need of `ci` (R1 to R4) | `test_census_once.py` |
| A2 | each binary the census set names spawns cargo with a cleared environment and its own target directory (R6) | `test_census_once.py` |
| A3 | the gate's stages equal the stages `TOOLS` names, `test-census` among them (R5) | `test_check_gate.py` |
| A4 | the test stages take both sets from their one definitions: `test` runs `not (<engine> \| <census>)`, `test-engine` the engine set and `test-census` the census set, as written and planted (R2, R5) | `test_check_gate.py` |
| A5 | `test-engine` and `test-census` name exactly the targets their sets derive, and `test` none, as written and planted (R3, R5) | `test_check_gate.py` |
| A6 | the gate's layout, cache and protoc pins name the `census` job and its stage (R4, R5) | `test_ci_workflows.py` |
| A7 | the derived ceilings, 72 other `ci.yml` jobs counted, equal `{"ci": 184, "battery": 234}` and `LEG_CEILING`, and the dispatcher's refusal, listing and headroom pins follow 184 (R7) | `test_dispatch_shards.py` |
| A8 | the shard cap, the sizing constants and the whole tree's headroom line follow 184 (R7) | `test_mutation_verdict.py` |
| A9 | SPEC-423's pins follow 184: the placeholder, one past it, one below it, the range's ends, its refusal, the pull request run's refusal and the joint line (R7) | `test_release_rust_ceiling.py` |
| A10 | the pull request plan's joint line shares 216 legs, 32 Python and 184 Rust (R7) | `test_release_python_ceiling.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_census_once.py -k test_the_census_set_runs_once_in_its_own_job
A2: python3 -m unittest discover -s scripts/tests -p test_census_once.py -k test_census_set_spawns_its_own_cargo
A3: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_the_gate_has_no_toolchain_stage_and_splits_the_audit
A4: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_both_test_stages_take_the_engine_set_from_its_one_definition
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_the_engine_stage_builds_only_the_targets_the_set_names
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_gate_runs_in_four_parallel_jobs -k test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile -k test_every_job_that_compiles_rust_installs_the_pinned_protoc_first
A7: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget -k test_the_release_listing_fits_one_run -k test_a_listing_beyond_the_ceiling_is_refused_whole -k test_every_sizing_prints_its_headroom
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap -k test_the_sizing_constants_are_the_measured_ones -k test_the_whole_tree_is_sized_at_the_batterys_ceiling
A9: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_only_a_true_release_mark_holds_the_rust_ceiling -k test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run -k test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling -k test_a_rust_ceiling_outside_its_range_is_refused_by_name -k test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone -k test_the_rust_release_line_stands_beside_the_python_release_line -k test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder
A10: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/check.sh` | gate | changed: `CENSUS_TESTS`, the two filters, `stage_test_census` and `test-census` in `STAGES_ALL` (R1 to R3) |
| `.github/workflows/ci.yml` | workflow | changed: the `census` job and `census` in the needs of `ci` (R4) |
| `scripts/tests/test_census_once.py` | tests | added: A1 and A2 |
| `scripts/tests/test_check_gate.py` | tests | changed: the census set beside the engine set (R5, A3 to A5) |
| `scripts/tests/test_ci_workflows.py` | tests | changed: `OWNER_LAYOUT`, `COMPILES_RUST` and three call-register entries (R5, A6, section 8) |
| `scripts/tests/test_dispatch_shards.py` | tests | changed: the ceiling pins at 184 (A7) |
| `scripts/tests/test_mutation_verdict.py` | tests | changed: the ceiling pins at 184 (A8) |
| `scripts/tests/test_release_rust_ceiling.py` | tests | changed: the ceiling pins at 184 (A9) |
| `scripts/tests/test_release_python_ceiling.py` | tests | changed: the ceiling pins at 184 (A10) |
| `scripts/mutation-verdict.py` | gate | changed: `LEG_CEILING["ci"]` 184 and its comment (R7) |
| `scripts/mutation-rows.d/S36200-S36299.json` | rows | changed: row `S36201` re-anchored on `"ci": 184` |
| `docs/specs/SPEC-330-the-tests-prove-their-rules-in-the-build-the-product-ships.md` | records | changed: an insert-only amendment section appended |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | records | changed: an insert-only amendment section appended |
| `docs/specs/SPEC-632-the-xp-census-runs-once-per-run-in-a-job-of-its-own.md` | records | added: this SPEC |
| `docs/decisions/ADR-655-the-census-set-runs-once-per-run-in-its-own-ci-job.md` | records | added: its decision |
| `docs/schematics/census-once-filtersets-and-jobs.md` | records | added: the filtersets, stages and jobs |
| `docs/red-first/SPEC-632.md` | records | added: the red-first record |
| `changelog.d/census-once-632.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- `skip_zero_upload` in the ingest crate: it is a behavioural test against the engine's own sync
  server, so SPEC-330's reason, proving shipped behaviour in the shipped profile, applies to it,
  and it stays in both test stages.
- Path gates that skip a job when its inputs are unchanged, and the release job's own dependency
  cache: other changes to the same workflow.
- The census tests themselves: no Rust file changes, and no census rule moves.
- `docs/schematics/ci-jobs-and-caches.md`: its job and cache tables are not edited here.
- `LEG_CEILING["battery"]` 234, `RUST_RELEASE_CEILING` 192, `RUST_MATRIX_LEGS` 256,
  `ROWS_MAX_LEGS` 18 and `PYTHON_MAX_SHARDS` 32: none moves.

## 6. Risks

- **A census test that needs the shipped profile:** none does today (section 1); R6's test refuses
  a set binary that does not spawn its own cargo, so a runtime test cannot be moved into the set.
- **The census job's bound:** 45 minutes against the slowest census test and one debug build of
  its target; the job's own duration in CI decides whether it moves.
- **One more hosted job per run:** counted in the ceiling (section 8), so a pull request run still
  fits the 256 jobs GitHub generates for it.
- **A later `ci.yml` job:** lowers the ceiling again by its own job count, with its pins, in the
  same pull request (section 8).

## 7. What only CI proves, measured after the land

The acceptance of the delivery as a whole is one push run on dev whose `census` job runs every
test of the census set and whose `rust` and `release` nextest summaries list no `xp_census` test.
It is measured after the land, and its run and job ids are recorded with it.

## 8. The pull request run's Rust ceiling is 184

The `census` job is one more job a pull request run generates, so the run's ceiling for Rust legs
falls from 185 to 184 by SPEC-417 R11's unchanged rule: the run's 256 jobs less every other
`ci.yml` job, 72 with the four bench jobs and `census`, the rows job counted at its 18 legs. The
maintainers' rule is that each later `ci.yml` job lowers it by its own job count, the figure and
its pins moving in the same pull request; the step from 185 needs no signed record of its own.

1. **The ceiling.** `LEG_CEILING["ci"]` in `scripts/mutation-verdict.py` is 184: 256 less 72 other
   `ci.yml` jobs, 67, the four bench jobs and `census`. The comment above it names the 72 jobs,
   keeps its cite of the signed 185 record and cites this SPEC for the one-job step. Row `S36201`
   keeps its id, stem, mutant, reason and killer and is re-anchored: its find `"ci": 185` becomes
   `"ci": 184`, and its replacement stays `"ci": 194`. `LEG_CEILING["battery"]` 234,
   `RUST_RELEASE_CEILING` 192, `RUST_MATRIX_LEGS` 256, `ROWS_MAX_LEGS` 18 and `PYTHON_MAX_SHARDS`
   32 do not move. A pull request whose Rust mutants need more than 184 legs is refused whole by
   name, never cut.
2. **The pins.** Every test pin of the `ci` ceiling, or of a figure derived from it, follows 184
   and keeps its relation to the ceiling: each refusal that names the pull request run's ceiling
   names 184, and each `legs N of ceiling 185` reads `legs N of ceiling 184`; with no ceiling
   named, 184 one-leg mutants size at 184 legs and 185 are refused; the ingest listing past the
   ceiling is 12513 mutants, one more than the 12512 that 184 legs of 68 hold, refused naming 184
   with its projection; the pull request run's joint line shares 216 legs, 32 Python and 184 Rust,
   and its runner-minutes are `(32 x 1800 + 184 x 10800) // 60`; SPEC-423's placeholder release
   ceiling is 184, the figure one past it 185 and the figure one below it 183, the range's ends
   are 184 and 256, and the range's refusal reads `outside 184..256`; a release ceiling of 200 or
   256 keeps its figures. The 128 beside the ceiling in
   `rust_deal(self, listed, scripts, release, 184, 128)` is not the ceiling. The call register of
   `scripts/tests/test_ci_workflows.py` names each call these pins change, each with its count
   of 1.

## 9. FORMAL

Not required by surface: the change moves two stage filters, adds a stage, a workflow job, two
census tests and a module constant, and no function a formal digest covers changes its body.

## 10. Mutation rows

None added. Row `S36201` is re-anchored (section 8); the six rows on the needs line of `ci`
(`S34103`, `S42203` and `S61800` to `S61803`) keep their finds, each found once.

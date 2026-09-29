# SPEC-126: the mutation verdict reads each report by name, so its layout never depends on how many artifacts exist

- **Wave:** W4. **Issue:** #351. **Context(s):** `repo` (`.github/workflows/ci.yml`, its tests and
  `docs/`).
- **Decided by:** ADR-126 (this SPEC's own: one download per single-artifact producer, and what that
  was chosen against), ADR-057 (the mutation jobs and the verdict) and SPEC-039 R3 and R18 (the
  plan, the shards and the verdict; it takes a dated amendment).
- **Status:** delivered. It waited in `docs/specs/planned/` from its own commit until its tests were
  green, and the delivery moved it to `docs/specs/` (ADR-016). It holds `docs/red-first/SPEC-126.md`.

## 1. The problem, measured

- **`actions/download-artifact` at the pinned v8.0.1 lays a download out by how many artifacts
  matched.** Its source (`src/download-artifact.ts`) extracts into `path` itself when the download
  is by `name`, when `merge-multiple` is true, or when exactly one artifact matched; only two or
  more matches, without `merge-multiple`, extract each into `path/<artifact name>`. Its README says
  the same: "This change also applies to patterns that only match a single artifact."
- **The verdict's one download is a pattern.** `mutation-verdict` downloads `pattern: mutation-*`
  into `reports` and then reads `reports/mutation-plan/plan.json`, `reports/mutation-rows/rows.json`
  and `reports/mutation-plan/whole.json`, and the shards from `reports/mutation-rust-shard-<i>/`.
- **The pattern also matches `mutation-web`, which the verdict does not wait for.** The verdict's
  `needs:` are `mutation-plan`, `mutation-rust` and `mutation-rows`. On a pull request that changes
  no Rust, `mutation-rust` and `mutation-rows` upload nothing (their report directories are never
  made), so when the verdict starts before `mutation-web` has uploaded, the pattern matches
  `mutation-plan` alone. The plan then lands at `reports/plan.json`, the judge reads no plan and
  exits `VOID no plan`, and a re-run of the failed jobs, which finds two matches, passes (#351).
- **The shards carry the same hazard, masked.** The plan sizes 1 to 256 shards. A pull request whose
  diff needs one shard uploads one `mutation-rust-shard-0` artifact, and a pattern that matches it
  alone extracts it flat, so `reports/mutation-rust-shard-0/mutants.out/outcomes.json` is absent.
  Today the plan's own artifact makes the pattern match two, which hides it.

## 2. Requirements

R1. **The verdict's report layout never depends on how many artifacts match.** The plan is
downloaded by `name:` into `reports/mutation-plan`; the rows' report is downloaded by `name:` into
`reports/mutation-rows`; the shards are downloaded by `pattern: mutation-rust-shard-*` with
`merge-multiple: true` into `reports`, and each shard's artifact carries its own top directory
`mutation-rust-shard-<i>/`, so one shard and many land at the same paths.

R2. **The verdict reads no artifact of a job outside its `needs:`.** `mutation-web` stays out of
`needs:` (it is the slowest job and the verdict has no use for its report), and no download step of
the verdict names or matches `mutation-web`.

R3. **A producer that uploaded nothing leaves the verdict's reading unchanged.** A `name:` download
of an artifact that was never uploaded fails the step, so the rows' download runs only when the plan
said the rows ran (`needs.mutation-plan.outputs.rows == 'true'`); otherwise `reports/mutation-rows/`
is absent, as it is today, and the judge reads the not-applicable case from the plan. No step
carries `continue-on-error`. The shards' pattern matching nothing is not an error.

R4. **A test pins the layout.** It reads the verdict's and the shard job's steps from `ci.yml`,
applies the pinned action's layout rule to every combination of shard count, rows present and web
present, and asserts the paths the judge reads.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | in every combination of 0, 1 and 3 shards, rows present or not and `mutation-web` present or not, the plan is at `reports/mutation-plan/plan.json`, the rows' report at `reports/mutation-rows/rows.json` when it exists, and each shard's outcomes at `reports/mutation-rust-shard-<i>/mutants.out/outcomes.json` | `test_verdict_download.py` `the_layout_holds_for_every_count_of_artifacts` |
| A2 | the layout with `mutation-web` present equals the layout without it, no verdict download names or matches it, and it is not among the verdict's `needs:` | `test_verdict_download.py` `the_verdict_reads_no_artifact_of_a_job_it_does_not_need` |
| A3 | with the rows never uploaded, no download step of the verdict fails and no file of `reports/mutation-rows/` exists; and no step carries `continue-on-error` | `test_verdict_download.py` `a_producer_that_uploaded_nothing_leaves_the_reading_unchanged` |
| A4 | the judge command lines still read the plan, the rows and the whole listing at the paths above | `test_verdict_download.py` `the_judge_reads_the_paths_the_downloads_lay_down` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k the_layout_holds_for_every_count_of_artifacts
A2: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k the_verdict_reads_no_artifact_of_a_job_it_does_not_need
A3: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k a_producer_that_uploaded_nothing_leaves_the_reading_unchanged
A4: python3 -m unittest discover -s scripts/tests -p test_verdict_download.py -k the_judge_reads_the_paths_the_downloads_lay_down
```

The test emulates the pinned action's layout rule (quoted in section 1) in pure Python over the
steps it parses from `ci.yml`; it does not run GitHub. Each test prints how many scenarios it
examined and refuses zero. The live proof is the pull request's own `mutation-verdict` run, a
docs-only diff, whose download step logs are quoted in the pull request.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | repo | changed: R1 to R3, the verdict's downloads and the shard job's output directory |
| `scripts/tests/test_verdict_download.py` | repo | added: A1 to A4 |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: the assertion that read `pattern: mutation-*` now reads the three downloads |
| `docs/specs/SPEC-126-the-mutation-verdict-reads-each-report-by-name.md` | repo | added, from `docs/specs/planned/` |
| `docs/decisions/ADR-126-the-verdict-downloads-each-single-artifact-producer-by-name.md` | repo | added |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | repo | changed: a dated amendment at its end (insert-only) |
| `docs/red-first/SPEC-126.md` | repo | added |
| `changelog.d/fix-verdict-download-126.md` | repo | added |

No schematic: the change adds no component. The data flow is the one SPEC-039 draws (a job uploads
a report, the verdict downloads it), and section 2 says where each report lands.

## 5. What this does NOT do

- It changes no Rust and no Python outside the tests, because the defect is in a workflow's download
  layout (#351).
- It does not touch `mutation-weekly.yml`: its survivors job downloads every artifact of its own
  run and reads them by recursive search, which does not depend on the layout (#351).
- It adds no mutation-row band: the invariant lives in a workflow and is pinned by A1 to A4, and no
  row selects a workflow target (#351).
- It gives the reader no tolerance for a second layout, because a reader that accepts both layouts
  hides the defect (ADR-126, #351).
- It does not add a Python mutation job; the open pull request that adds `mutation-python` follows
  the same rule for its shards when it lands (#351).

## 6. Risks

- **The shard artifact's shape changes.** `mutation-rust` writes each shard under
  `shards/mutation-rust-shard-<i>/` and uploads `shards/`. Anything that reads the pull-request
  shard artifacts by path breaks; none does in the tree (A1 reads the layout the judge reads).
- **A `name:` download of a missing artifact fails the step.** R3 guards the rows' download with the
  plan's own output. If the rows job crashes before it makes its report directory, the download
  fails loudly instead of the judge reading VOID; both are red.
- **The emulation is not GitHub.** It applies the rule read from the pinned source; the pull
  request's live run is the check that the rule was read right.

## 7. References

Issue #351; SPEC-039 R3 and R18; ADR-057; ADR-016; ADR-126; `actions/download-artifact` v8.0.1
(`src/download-artifact.ts`, `README.md`).

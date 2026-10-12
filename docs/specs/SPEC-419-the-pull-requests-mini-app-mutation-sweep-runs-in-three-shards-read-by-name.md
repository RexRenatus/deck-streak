# SPEC-419: the pull request's Mini App mutation sweep runs in three shards, read by name

- **Issue:** #693 ("Shard the mutation-web job"); this delivery refers to it and closes nothing.
  **Context(s):** CI (`.github/workflows/ci.yml`), one script (`scripts/web_mutation_shards.py`)
  and their tests (`scripts/tests/`); no crate, no Mini App source and no StrykerJS setting
  changes.
- **Decided by:** ADR-433 (three shards, whole files dealt by StrykerJS's own mutant count, largest
  first onto the lightest shard, and one job of the leg's own name that reads every shard by name;
  a raised bound, a fold into the Python legs' pricing, a narrower sweep and more StrykerJS workers
  are rejected).
- **Schematic:** `docs/schematics/mutation-testing.md` section 10 (data flow: the plan, the count,
  the deal, the shards, the union read by name, and the judge).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-419.md`. **Mutation band:** `S41900-S41999` (section 9). **Model:** none
  (section 8). Amends SPEC-039 (R3's one `mutation-web` job) by an amendment section in its own
  file.

## 1. The problem, measured

`ci.yml`'s `mutation-web` job runs StrykerJS over every web production file a pull request
changes, whole (SPEC-039 R3, ADR-057 D3), in one job bounded at 100 minutes (SPEC-379). On a
release pull request that is most of the Mini App. Every figure below is read from the job's own
log or from StrykerJS's own report.

- **The release's one run was cut at its bound.** Run 38242723211, `mutation-web` job 114789042867
  ran 100m13s against its `timeout-minutes: 100` (job 114789042867), so the job reads cancelled,
  although StrykerJS logged "Done in 99 minutes and 30 seconds" (job 114789042867), every one of
  its 7805 mutants carries a status and the verdict step read `verdict: ok`. A required leg at
  100.2 percent of its bound is cut by any growth or any slower runner.
- **What one mutant costs.** In that job, mutation testing ran 5849.24 s over 7805 mutants, 0.7494 s
  a mutant (job 114789042867). The weekly battery's whole sweep ran 4484.84 s over 6446 mutants,
  0.6958 s a mutant (job 113901239242).
- **What one shard costs before its first mutant.** Set-up 34 s, StrykerJS's start to the end of
  its initial test run 122 s, and the report's tail 6.7 s: 162.7 s a shard (job 114789042867).
  Every shard repeats the initial test run, because StrykerJS measures each test's coverage there.
- **The sweep's population.** The release's report holds 7805 mutants in 106 files (job
  114789042867); its largest file holds 1464.
- **The projection.** The slowest shard's minutes are (162.7 s + 0.7494 s x its mutants) / 60, with
  the per-file counts of that report dealt largest first onto the lightest shard (job 114789042867):
  one shard holds 7805 mutants, 100.2 minutes; two, 3903 and 3902, 51.5 minutes; three, 2602, 2602
  and 2601, 35.2 minutes; four, 1952 and three of 1951, 27.1 minutes. Weighted by the tests each
  mutant ran instead (21484 tests in job 114789042867), the slowest of two shards reads 57.4
  minutes, of three 41.2 and of four 35.9.

## 2. Requirements

R1. **Three shards.** `ci.yml`'s job `mutation-web-shard` is one matrix over `shard: [0, 1, 2]`
    with `fail-fast: false`, so one shard's failure cancels no other, and
    `scripts/web_mutation_shards.py`'s `SHARDS` is 3.
R2. **The deal is by StrykerJS's own count.** Each shard reads the plan's `stryker_mutate` files,
    asks StrykerJS's own mutation server (`stryker serve stdio`, its `configure` then `discover`
    methods, run in `web/app` with `--mutate` set to exactly those files, so the server reads the
    run's configuration and the run's file set) for each file's mutants, and counts them; a
    planned file the server names no mutant for counts 0, and a file it names that the plan does
    not is refused. Whole files are dealt by that count, largest first, each onto the shard with
    the fewest mutants so far: equal counts go by path, and equally light shards by the lowest
    index. Every planned file is in exactly one shard, and the shards' mutants sum to the count.
    A count that fails, or a server that refuses, reads VOID and writes no record.
R3. **Each shard keeps the leg's bound and settings.** Each shard declares `timeout-minutes: 100`
    and runs `stryker run --mutate` over its own dealt files, whole, with
    `web/app/stryker.config.json` unchanged; it records StrykerJS's exit in `stryker.exit` and
    keeps StrykerJS's own report as `mutation.json`.
R4. **Each shard keeps its record under its index.** Each shard writes `shard.json` (the schema,
    its index, the shard count, every file's count, the whole deal, its own files, its mutants and
    whether it is empty) and uploads its directory as `mutation-web-shard-<k>`, whatever the job's
    outcome.
R5. **An empty shard is an explicit record.** A shard dealt no mutant, because the plan names no
    web file or because the deal gave it none, writes `shard.json` with `"empty": true`, runs no
    StrykerJS and uploads that record. It is read by name like any other, never absent.
R6. **One job of the leg's own name reads every shard by name.** `mutation-web` is one job, not a
    matrix, so the check that reports `mutation-web` still reports under that name. It needs
    `mutation-web-shard`, runs `if: ${{ always() }}`, keeps `timeout-minutes: 100`, downloads every
    `mutation-web-shard-*` artifact and runs `web_mutation_shards.py union`, which reads
    `mutation-web-shard-0`, `-1` and `-2` each by name and reads VOID, exit 3, writing nothing,
    when any shard's record is missing or is not its shard; when the shards' job result is not
    `success`; when a record counted other files than the plan names, or holds files that are not
    its share of the deal of its counts; when a shard with mutants has no `stryker.exit` or no
    report; when a report holds a file not dealt to it, or a dealt file with other than its counted
    mutants; or when the shards together do not hold every planned file exactly once. It prints
    `web shards: counted <k> of 3 reports whole` and `examined 3 report(s)`.
R7. **The union is judged as one report.** When every shard is whole, `union` writes the shards'
    files merged into one report at the path the unchanged judge step reads,
    `mutation-verdict.py judge --class web --stryker "$RUNNER_TEMP/mutation/stryker/mutation.json"`.
    The StrykerJS configuration and its thresholds, `scripts/mutation-verdict.py`, every record
    under `scripts/mutation-equivalent.d/` and the survivors table are byte-identical to the base.
R8. **`ci` needs the shards.** `ci`'s `needs:` gains `mutation-web-shard`, because the aggregate
    check needs every job of the workflow; no other job of any workflow changes.

## 3. Acceptance criteria of SPEC-419

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | `ci.yml` runs the web sweep as `mutation-web-shard`, a matrix of exactly shards 0, 1 and 2 with `fail-fast: false`, each 100 minutes, that deals its own shard, runs StrykerJS over its deal's files and keeps `mutation-web-shard-<k>` whatever its outcome; and `mutation-web`, not a matrix, needs it, runs always, downloads every shard and reads each by name before its judge, which reads the union (R1, R3, R4, R6) | red at the base, which has no shard job | `test_the_web_sweep_runs_in_three_shards_read_by_name` |
| A2 | each defect planted into a copy of `ci.yml` is refused by its own line: one, two or four shards, a shard bound of 101, `fail-fast` removed, StrykerJS over the plan's files instead of the deal's, an artifact name without the index, a verdict job that can be skipped, that does not need the shards or that downloads one shard (R1, R3, R4, R6) | red at the base, whose control holds no shard job to plant into | `test_a_web_shard_defect_is_refused_by_name` |
| A3 | `ci` needs `mutation-web-shard` beside the other mutation jobs (R8) | red at the base, whose `ci` needs no shard job | `test_the_gate_runs_in_four_parallel_jobs` |
| A4 | the mutation jobs' conditions: `mutation-web` runs `if: ${{ always() }}`, `mutation-web-shard` has no condition and keeps its report always, and StrykerJS runs in the shard job and not in `mutation-web` (R1, R6) | red at the base, where `mutation-web` has no condition | `test_the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache` |
| A5 | the not-started rule's census reads `mutation-web` as always run and `mutation-web-shard` as never skipped (R6) | red at the base, where `mutation-web` has no condition | `test_each_legs_job_condition_reads_the_plans_listing_alone` |
| A6 | the deal puts the largest file first onto the lightest shard, over a fixed table of counts (R2) | red at C1, whose stub deals nothing | `test_the_deal_puts_the_largest_file_first_onto_the_lightest_shard` |
| A7 | equal counts go by path and equally light shards by the lowest index (R2) | red at C1, whose stub deals nothing | `test_equal_counts_go_by_path_and_equal_shards_by_the_lowest_index` |
| A8 | every file is dealt to exactly one shard and the total is kept, for one to four shards (R2) | red at C1, whose stub deals nothing | `test_every_file_is_dealt_to_exactly_one_shard_and_the_total_is_kept` |
| A9 | a shard dealt nothing writes an explicit empty record, and its outputs run no StrykerJS (R4, R5) | red at C1, whose stub writes no record | `test_a_shard_dealt_nothing_writes_an_explicit_empty_record` |
| A10 | the count is StrykerJS's own discover, asked in the app over exactly the planned files, and a refusal, a silent server, a stray file or a missing binary reads VOID (R2) | red at C1, whose stub asks nothing | `test_the_count_is_strykers_own_discover_over_the_planned_files` |
| A11 | the union reads every shard by name and merges their files into one report (R6, R7) | red at C1, whose stub reads nothing | `test_the_union_reads_every_shard_by_name_and_merges_their_files` |
| A12 | a missing, cut, partial, foreign or misdealt shard, and an empty shard left absent, read VOID by name and write no union (R5, R6) | red at C1, whose stub reads nothing | `test_a_missing_cut_or_partial_shard_reads_void_and_writes_nothing` |
| A13 | the command line refuses a missing argument, an unknown shard and an unreadable plan, and writes the deal's outputs (R2, R4) | red at C1, whose stub parses nothing | `test_the_command_line_refuses_what_it_cannot_read` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_web_sweep_runs_in_three_shards_read_by_name
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_web_shard_defect_is_refused_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_gate_runs_in_four_parallel_jobs
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache
A5: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k test_each_legs_job_condition_reads_the_plans_listing_alone
A6: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_deal_puts_the_largest_file_first_onto_the_lightest_shard
A7: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_equal_counts_go_by_path_and_equal_shards_by_the_lowest_index
A8: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_every_file_is_dealt_to_exactly_one_shard_and_the_total_is_kept
A9: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_a_shard_dealt_nothing_writes_an_explicit_empty_record
A10: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_count_is_strykers_own_discover_over_the_planned_files
A11: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_union_reads_every_shard_by_name_and_merges_their_files
A12: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_a_missing_cut_or_partial_shard_reads_void_and_writes_nothing
A13: python3 -m unittest discover -s scripts/tests -p test_web_mutation_shards.py -k test_the_command_line_refuses_what_it_cannot_read
```

All thirteen are read red and green from CI's `hygiene` job, which runs `scripts/tests`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: `mutation-web-shard` added (three shards); `mutation-web` reads every shard by name before its unchanged judge; `ci` needs the shards |
| `scripts/web_mutation_shards.py` | CI | added: the count, the deal, the shard's record and the union read by name |
| `scripts/mutation-python.json` | CI | changed: the new script's entry, its killer module `test_web_mutation_shards` (the Python runner's census needs every `scripts/*.py`) |
| `scripts/tests/test_web_mutation_shards.py` | CI tests | added: A6 to A13 |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: the shard checker and A1, A2; A3's list of `ci`'s needs |
| `scripts/tests/test_mutation_workflows.py` | CI tests | changed: `JOB_CONDITIONS` and A4's jobs |
| `scripts/tests/test_not_started_legs.py` | CI tests | changed: A5's conditions |
| `scripts/mutation-rows.d/S41900-S41999.json` | CI tests | added: rows `S41900` onward |
| `docs/specs/SPEC-419-the-pull-requests-mini-app-mutation-sweep-runs-in-three-shards-read-by-name.md` | docs | added: this file |
| `docs/decisions/ADR-433-the-web-sweep-is-dealt-by-mutant-count-into-three-shards-and-judged-by-name.md` | docs | added: the decision |
| `docs/red-first/SPEC-419.md` | docs | added: each criterion's red and green |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | docs | changed: an amendment section |
| `docs/schematics/mutation-testing.md` | docs | changed: section 10 |
| `changelog.d/web-mutation-shards-419.md` | docs | added: the fragment |

## 5. What this does NOT cover

- It does not change the weekly battery's `web` job, which fits its bound (#693): its whole sweep
  of 6446 mutants ran 76m53s (job 113901239242), and `.github/workflows/mutation-weekly.yml` is
  byte-identical.
- It does not price or reshape the Python legs, their shards or their plan's sizing, none of which
  #693 asks for; `scripts/mutation-python.json` gains only the new script's own entry.
- It does not settle the weekly sweep's break threshold, which read 99.78 against its 100, or the
  20 web files that sweep did not reach. Each is an item on the architect's queue with no issue
  yet; the web sweep's sharding issue is #693.
- It changes no StrykerJS setting: a narrower `mutate` set, `ignoreStatic` and more workers are each
  rejected in ADR-433 (#693).
- It does not change how a mutant is judged: `scripts/mutation-verdict.py`, the equivalence records
  and the survivors table are untouched, and the judge reads the union as it read one report
  (#693).

## 6. Risks

- **The count and the run disagree.** The deal trusts StrykerJS's discover to name the mutants its
  run will test. If they differ, a shard's report holds other than its counted mutants and the
  union reads VOID by file, never ok, so the disagreement is detected, not judged around.
- **The mutation server changes its protocol.** A server that answers other than one reply per
  request, or no `files`, makes the count read VOID in every shard, which the union then reads as
  missing records; the pinned lockfile holds the server's protocol until a deliberate bump.
- **A shard outgrows its bound.** Each shard keeps 100 minutes; at 7805 mutants the slowest shard
  is projected at 35.2 minutes under the count model and 41.2 under the tests-weighted model (job
  114789042867). A shard cut at its bound reads `cancelled`, and the union reads VOID by name.
- **A hung mutation server holds a shard.** The count has no bound of its own; a server that never
  answers holds its shard to the job's 100 minutes, where the cut reads VOID.
- **A row's anchor moves.** `ci.yml` now holds three `timeout-minutes: 100` lines, so every row on
  one names its job; a find that no longer occurs exactly once is refused by the rows census by
  stem.

## 7. What only CI proves

A1 to A5's red and green, A6 to A13's red and green in the `hygiene` job, the rows' census and
proof, and the three shards' first whole sweep are CI's, read by name. This pull request changes no
web file, so its own run reads the web class not applicable through every shard's explicit empty
record and the union; the first sweep dealt into three shards is the next run whose plan names a
web file.

## 8. Formal model

None. The shards share no written object: each writes its own directory and uploads its own
artifact, and the verdict job reads them only after every shard has ended (`needs:`), whatever
their outcome. Three readers of one plan, joined once, have no interleaving to model.

## 9. Mutation rows

Filled with the rows commit; each row's killer is named in `scripts/mutation-rows.d/S41900-S41999.json`.

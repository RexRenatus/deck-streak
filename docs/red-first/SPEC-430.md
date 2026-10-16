# Red-first record: SPEC-430

SPEC-430 (R1 to R13, A1 to A9). The SPEC, ADR-444, the new section of
`docs/schematics/ci-jobs-and-caches.md`, the changelog fragment and the tests of A1 to A9 were
committed first and alone, in 9e9dc1ae6642df8f61ecc51b35450a16d723561a. In
`scripts/tests/test_ci_workflows.py` that commit inserts, above
`class TheEngineBuildsInEveryRustJob`, the `SHAPE_*` constants, the class
`TheCachesHoldTheirShape` and four census helpers that keep every parameter and return no problems
and no population: `shape_cargo_census(directory, root, reads)`, `shape_writer_census(directory)`,
`shape_key_census(directory)` and `shape_run_unique_census(directory)` each returned `[], []`.
Each red below is therefore its criterion's assertion and never a missing name or an empty
population: A1, A3, A5 and A7 fail on their controls, read by code the census did not build,
before their counts, and A2, A4, A6, A8 and A9 on their expected refusals against `[]`. The module
runs only in CI, so every red is quoted from the pull request's checks over that commit, in the job
`hygiene`, the job that runs `scripts/tests`.

The module anchors the SPEC, ADR-444 and the schematic quote were each read by `grep -n` at `dev`
b25ed04ab10f24a89a1ec0eed71f8a365f411521, the commit the branch starts from, and each equals the
number they quote. The new block's 1487 lines start at `:2723`, so at the green commit every anchor
from `:2723` on sits 1487 lines further down; the anchors above it do not move.

| anchor | at `dev` | quoted |
|---|---|---|
| `import ast` | `:11` | `:11` |
| `from _support import REPO, examined` | `:29` | `:29` |
| `WORKFLOWS` | `:32` | `:32` |
| `LOCAL_CALL` | `:37` | `:37` |
| `STAGES` | `:38` | `:38` |
| `RUST_CACHE` | `:71` | `:71` |
| `BROWSERS` | `:72` | `:72` |
| `COMPILES_RUST` | `:75` | `:75` |
| `GATE_CALL` | `:85` | `:85` |
| `CACHE_BY_THEMSELVES` | `:87` | `:87` |
| `workflow_file_text` | `:110` | `:110` |
| `workflow_files` | `:118` | `:118` |
| `WorkflowsAreHardened` | `:127` | `:127` |
| `read_workflow` | `:1691` | `:1691` |
| `action` | `:2041` | `:2041` |
| `lines_of` | `:2046` | `:2046` |
| `paths` | `:2051` | `:2051` |
| `stage_calls` | `:2059` | `:2059` |
| `condition` | `:2200` | `:2200` |
| `cache_problems` | `:2257` | `:2257` |
| `scheduled_save_problems` | `:2294` | `:2294` |
| `save_problems` | `:2324` | `:2324` |
| `TheGateRunsInParallelJobs` | `:2379` | `:2379` |
| `OnlyAPushSavesACache` | `:2560` | `:2560` |
| its Rust restore test, save test and browser key test | `:2575-2622`, `:2624-2642`, `:2661-2697` | `:2575-2622`, `:2624-2642`, `:2661-2697` |
| `TheEngineBuildsInEveryRustJob`, the insertion point | `:2723` | `:2723` |
| `engine_job_problems`, its matcher and its cache check | `:2844`, `:2870`, `:2873-2878` | `:2844`, `:2870`, `:2873-2878` |
| `NOT_WORKFLOW_READS` | `:4793` | `:4793` |
| `DYNAMIC_IMPORTS` | `:7172` | `:7172` |
| `WorkflowFilesAreReadAsBytes` | `:8207` | `:8207` |

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_job_that_runs_cargo_restores_the_one_shared_rust_key
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_cargo_census_refuses_each_planted_shape
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_only_the_warm_job_or_a_push_to_dev_or_main_saves_the_rust_key
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_writer_census_refuses_each_planted_shape
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_cache_key_hashes_its_lock_and_toolchain_inputs
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_key_census_refuses_each_planted_shape
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_no_cache_key_carries_a_run_unique_part
A8: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_run_unique_census_refuses_each_planted_shape
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_python_read_follows_the_subcommand_the_step_names
```

The reds were read in job 116942482703, `hygiene` over the red commit, a job of 1252 s:
`Ran 1294 tests` and `FAILED (failures=9)`. The nine failures are the nine tests the fence above
names, compared as sets with none missing and none extra, and no other test failed. Each failing
line below is quoted as CI printed it, unittest's own `[N chars]` cuts included.

Test edits after the red commit, each named here:
- The green commit, c2438d41d15e34c7778262c841b8b23f3720cd3c, replaces the four helpers' stub
  lines `    return [], []` with their bodies and adds the helpers those bodies call, all inside the
  inserted block. Against the red commit the module reads `527 4` by `git diff --numstat`, the
  four removed lines being those four stub returns; against `dev` it reads `1487 0`, so no existing
  line of the module changes. It changes no line of a criterion's test.
- The rows commit and this record's commit edit no test module. After the rows commit,
  `git -C /tmp/ds-worktrees/cache-shape-430 diff --quiet c2438d41d15e34c7778262c841b8b23f3720cd3c HEAD -- scripts/tests/test_ci_workflows.py`
  exits 0: the module at the second push's head is the green commit's, byte for byte.

```red-first
A1: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: {'engine': None, 'hygiene': None, 'rust': None} != {'engine': 'shared', 'hygiene': 'shared', 'rust': 'shared'}
A1: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A2: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != ['callee.yaml:called: a release job reads [3031 chars]ore']
A2: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A3: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != ['ci.yml:rust:8', 'ci.yml:web:9', 'ci.yml:web:11', 'rust-cache.yml:warm:9']
A3: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A4: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != ['planted.yaml:anypush:save: saves on a pu[740 chars]key"]
A4: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A5: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != [('ci.yml:rust:4', 'Rust', ["rust-${{ runn[1417 chars] [])]
A5: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A6: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != ["planted.yaml:drifted:0: the Rust key rus[1191 chars]ets']
A6: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A7: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != [('ci.yml:rust:4', "rust-${{ runner.os }}-[3277 chars]}}")]
A7: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A8: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != ['planted.yaml:salted:0: build-${{ inputs.[3491 chars]gen']
A8: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
A9: red at 9e9dc1ae6642df8f61ecc51b35450a16d723561a: AssertionError: Lists differ: [] != ['planted.yaml:prove-unrestored: restores [551 chars]nce']
A9: green at c2438d41d15e34c7778262c841b8b23f3720cd3c
```

No run reads the green commit alone, so each green is read at the second push's head, which holds
the green commit, the rows commit and this record. That push's checks, read by name, confirm each
green: the `hygiene` job passes every test of the class, its four tree tests each print their
`examined` line, and the rows S43000-S43060 are proved by the `mutation-rows` job's verdict line
for each row.

# Red-first record: SPEC-428

SPEC-428 (R1 to R9, A1 to A7). The SPEC, ADR-442, the note on ADR-055, the schematic, the
changelog fragment and the tests of A1 to A7 were committed first and alone, with no workflow
changed and `engine_job_problems` unchanged: the pin record `ENGINE_NEXTEST_PIN` landed in the
tests' own block, read there only by A7. So each red below is its criterion's assertion and never
a missing name. The test modules run only in CI, so every red is quoted from the pull request's
checks over that commit, in the job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_install_action_tool_names_an_exact_version
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_planted_tool_pin_defect_is_refused_by_its_own_line
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_job_that_runs_steps_sets_its_own_timeout
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_planted_timeout_defect_is_refused_by_its_own_line
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_zizmor_audits_the_github_directory_and_its_dependabot_configuration
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_planted_zizmor_defect_is_refused_by_its_own_line
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_engine_check_counts_only_the_pinned_nextest
```

The reds were read in job 116895827881, `hygiene` over the red commit, a job of 876 s:
`Ran 1292 tests` and `FAILED (failures=9)`. The nine failures are the seven tests the fence
above names, every one of them, and the two sibling tests whose expected tool strings moved with
the pin, each red at the red commit and green at the cure, outside this record's fence:
`test_rust_cache_workflow.TheRustCacheWorkflow.test_it_installs_restores_by_prefix_and_compiles_without_running_a_test`
read `AssertionError: 'cargo-nextest,cargo-deny' != 'cargo-nextest@0.9.146,cargo-deny@0.20.2'`,
and `test_release_workflow.TheReleaseBuildsTheSyncServer.test_the_release_waits_on_the_server_audit`
read `AssertionError: 'cargo-deny' != 'cargo-deny@0.20.2'`. No failure was an error, a missing
fixture or an empty population.

Each red is its criterion's. A1's list holds the 10 entries that name no version, the first
`ci.yml: rust: step 3: cargo-nextest names no exact version`. A2, A4 and A6 are red on their
controls, the unplanted live text, which each asserts before any plant. A3's two problems are
`fragment`'s and `rerelease`'s. A5's derived inputs are the 15 workflow files, without
`.github/dependabot.yml`. A7's pin record plant reads as no test runner, since the base's matcher
reads the pinned entry as another tool. Each failing line is quoted as CI printed it, unittest's
own `[N chars]` cuts included; `[cut]` marks where this record shortens CI's text or joins two of
its lines.

The cure, a1a0eb92, changes one line of a test module: `engine_job_problems`' matcher in
`scripts/tests/test_ci_workflows.py` compares with `ENGINE_NEXTEST_PIN` where it compared with
the bare `"cargo-nextest"` (SPEC-428 R7). It changes no assertion. CI runs only the head of the
push that carries the cure, the rows commit and this record together, never the cure alone, and
this record cannot name its own commit, so each green line names the cure. The cure's tree equals
that head's on every workflow and test file this delivery changes; this command exits 0:

```text
git -C /tmp/ds-worktrees/ci-tool-pins-428 diff --quiet a1a0eb9253e23f42211722fe335982b3d914e48e HEAD -- .github/workflows/changelog.yml .github/workflows/ci.yml .github/workflows/engine-measure.yml .github/workflows/release.yml .github/workflows/rust-cache.yml .github/workflows/testflight-rerelease-check.yml scripts/tests/test_ci_workflows.py scripts/tests/test_release_workflow.py scripts/tests/test_rust_cache_workflow.py
```

The one commit between the cure and this record is the rows commit, e5bad822, which touches only
`scripts/mutation-rows.d/S42800-S42899.json`. That push's CI, read by name, confirms each green.

```red-first
A1: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: Lists differ: ['ci.yml: rust: step 3: cargo-nextest name[617 chars]ion'] != []
A1: green at a1a0eb9253e23f42211722fe335982b3d914e48e
A2: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: Lists differ: [cut] != [] [cut] : the control reads ['ci.yml: rust: step 3: cargo-nextest names no exact version']
A2: green at a1a0eb9253e23f42211722fe335982b3d914e48e
A3: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: Lists differ: ['changelog.yml: fragment: sets no timeout[75 chars]tes'] != [] [cut] 'testflight-rerelease-check.yml: rerelease: [cut]
A3: green at a1a0eb9253e23f42211722fe335982b3d914e48e
A4: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: Lists differ: [cut] != [] [cut] : the control reads ['changelog.yml: fragment: sets no timeout-minutes']
A4: green at a1a0eb9253e23f42211722fe335982b3d914e48e
A5: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: '.github/dependabot.yml' not found in ['.github/workflows/apple-on-change.yml', '.github/workflows/apple-on-tag.yml', [cut]
A5: green at a1a0eb9253e23f42211722fe335982b3d914e48e
A6: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: [cut] : the control reads ['ci.yml: workflow-lint: zizmor runs as zizmor --no-online-audits --persona regular .github/workflows, [cut]
A6: green at a1a0eb9253e23f42211722fe335982b3d914e48e
A7: red at 4a94005207ff5c4576a2871a16ba20dc0160aca8: AssertionError: Lists differ: ['the engine job installs no cargo-nextest before its stage'] != [] [cut] : cargo-nextest@0.9.146
A7: green at a1a0eb9253e23f42211722fe335982b3d914e48e
```

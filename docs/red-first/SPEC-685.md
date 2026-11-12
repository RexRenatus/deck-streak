# Red-first record: SPEC-685

SPEC-685 (R1 to R8, and its six acceptance criteria). The SPEC, the fragment and the six tests of
`scripts/tests/test_agent_docs.py` were committed first, over the three documents as they stood.
Each red below is therefore its criterion's assertion against the claim it pins, never a missing
name, and each line names its test. The three corrected documents were committed next, and the
module did not change. The module runs by name, and CI runs it in the `hygiene` job, which runs
`scripts/tests`.

```red-first
A1: red at 5f3582a: test_the_builder_brief_counts_and_names_every_mutation_job_ci_needs: AssertionError: 6 != 8 : the bullet counts six CI jobs
A1: green at 87e741c
A2: red at 5f3582a: test_the_builder_brief_names_the_job_that_runs_strykerjs: AssertionError: Lists differ: ['mutation-web'] != ['mutation-web-shard']
A2: green at 87e741c
A3: red at 5f3582a: test_the_gate_comment_names_every_stage_check_sh_runs: AssertionError: Lists differ: ['fmt', 'clippy', 'nextest', 'doctests', 'web', 'oracle', 'scrub', 'secrets'] != ['fmt', 'clippy', 'test', 'doctest', 'audit-rust', 'test-eng[66 chars]ets']
A3: green at 87e741c
A4: red at 5f3582a: test_the_ci_table_names_the_jobs_and_stages_ci_and_check_sh_hold: AssertionError: Lists differ: ['packs'] != []
A4: green at 87e741c
A5: red at 5f3582a: test_the_coverage_section_names_every_python_shard_job: AssertionError: Lists differ: ['mutation-python'] != ['mutation-python', 'mutation-python-release']
A5: green at 87e741c
A6: red at 5f3582a: test_the_builder_brief_names_every_scope_the_weekly_dispatch_takes: AssertionError: Lists differ: ['miniapp'] != ['miniapp', 'python']
A6: green at 87e741c
```

The green commit `87e741c` carries the three corrected documents. Over the module,
`git diff --quiet 87e741c HEAD -- scripts/tests/test_agent_docs.py` reads rc 0.

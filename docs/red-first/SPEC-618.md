# SPEC-618: the red-first record

```red-first
A2: not red: its two tests are the controls beside the fork arm, a same-repository pull request and a push, which upload whatever the plan reads, so the stub that uploads every run reads them green
A1: red at 6883288c: test_perf_plan.py:70: AssertionError: 'true' != 'false' (job 126694654074)
A1: green at 6739ab7f
A3: red at 6883288c: test_perf_plan.py:140: AssertionError: 'true' != 'false' (job 126694654074)
A3: green at 6739ab7f
A4: red at 6883288c: test_perf_plan.py:165: AssertionError: 'true' != 'false' (job 126694654074)
A4: green at 6739ab7f
A5: red at 6883288c: test_perf_plan.py:193: AssertionError: 'earlier=kept\n' != 'earlier=kept\napplicable=false\nupload=false\nreason=th[52 chars]s)\n' (job 126694654074)
A5: green at 6739ab7f
A6: red at 6883288c: test_perf_plan.py:217: AssertionError: 'true' != 'false' (job 126694654074)
A6: green at 6739ab7f
A7: red at 6883288c: test_perf_plan.py:277: AssertionError: 0 != 1 : bench-rust: VOID --repo names no repository: applicable=true (job 126694654074)
A7: green at 6739ab7f
A8: red at 6883288c: test_perf_record.py:85: AssertionError: 'perf-record: examined 0 records, 0 refused\n' != 'perf-record: keep.json: OK\nperf-record: other.json: O[43 chars]ed\n' (job 126694654074)
A8: green at 6739ab7f
A9: red at 6883288c: test_perf_record.py:115: AssertionError: 0 != 1 : perf-record: examined 0 records, 0 refused (job 126694654074)
A9: green at 6739ab7f
A10: red at 6883288c: test_perf_record.py:137: AssertionError: 0 != 1 : perf-record: examined 0 records, 0 refused (job 126694654074)
A10: green at 6739ab7f
A11: red at 6883288c: test_perf_record.py:148: AssertionError: 0 != 1 : perf-record: examined 0 records, 0 refused (job 126694654074)
A11: green at 6739ab7f
A12: red at 6883288c: test_perf_record.py:158: AssertionError: 0 != 1 : perf-record: examined 0 records, 0 refused (job 126694654074)
A12: green at 6739ab7f
A13: red at 6883288c: test_perf_record.py:177: AssertionError: 0 != 1 : perf-record: examined 0 records, 0 refused (job 126694654074)
A13: green at 6739ab7f
A14: red at 6883288c: test_perf_record.py:190: AssertionError: 0 != 1 : perf-record: examined 0 records, 0 refused (job 126694654074)
A14: green at 6739ab7f
A15: red at efc7acfe: test_the_four_bench_jobs_are_needs_of_ci: AssertionError: 'bench-rust-build' not found in {'rust': {'runs-on': 'ubuntu-24.04' (job 126898689180)
A15: green at 9897e72b
A16: red at efc7acfe: test_only_the_two_upload_jobs_hold_an_id_token_grant: AssertionError: ci.yml has no bench job ['bench-rust-build', 'bench-rust', 'bench-web-build', 'bench-web'] (job 126898689180)
A16: green at 9897e72b
A17: red at efc7acfe: test_an_upload_job_unpacks_and_uploads_and_builds_nothing: AssertionError: ci.yml has no bench job ['bench-rust-build', 'bench-rust', 'bench-web-build', 'bench-web'] (job 126898689180)
A17: green at 0832e745
A18: red at efc7acfe: test_each_bench_job_starts_and_its_plan_gates_every_step: AssertionError: ci.yml has no bench job ['bench-rust-build', 'bench-rust', 'bench-web-build', 'bench-web'] (job 126898689180)
A18: green at 9897e72b
A19: red at efc7acfe: test_the_record_check_reads_the_records_a_pull_request_adds: AssertionError: ci.yml has no bench job ['bench-rust-build', 'bench-rust', 'bench-web-build', 'bench-web'] (job 126898689180)
A19: green at 9897e72b
A20: red at efc7acfe: test_the_bench_profile_inherits_release_with_line_tables: AssertionError: None != {'inherits': 'release', 'debug': 'line-tables-only'} : the root manifest has no [profile.bench] that inherits release with line tables (job 126898689180)
A20: green at 12fd31c9
A21: red at efc7acfe: test_each_rust_bench_is_a_harnessless_target_with_a_measured_body: AssertionError: None != {'package': 'codspeed-divan-compat', 'version': '=6.0.0'} : the workspace has no exact entry for the renamed Divan crate (job 126898689180)
A21: green at 12fd31c9
A22: red at efc7acfe: test_the_benchmark_list_equals_the_names_the_bench_files_define: AssertionError: False is not true : perf/benchmarks.txt is absent (job 126898689180)
A22: green at b3f776ba
A23: red at efc7acfe: test_the_web_benches_run_under_the_codspeed_plugin: AssertionError: False is not true : vitest.bench.config.ts is absent (job 126898689180)
A23: green at 89b6d5e1
A24: red at efc7acfe: test_bench_files_are_excluded_from_mutation_and_coverage: AssertionError: '!src/**/*.bench.*' not found in ['src/**/*.ts', 'src/**/*.js', 'src/**/*.svelte', '!src/**/*.test.*', '!src/**/*.spec.*', '!src/**/*.d.ts', '!src/lib/paraglide/**'] (job 126898689180)
A24: green at b3f776ba
A26: red at fff4d401: test_mutation_verdict.py:1090: AssertionError: {'ci': 189, 'battery': 234} != {'ci': 185, 'battery': 234} (read on the box: 3 of 44 failing, each on the ceiling)
A27: red at fff4d401: test_release_rust_ceiling.py:308: AssertionError: Tuples differ: (1, '[71 chars]00 is outside 189..256; a ceiling outside it is never dealt\n') != (1, '[71 chars]00 is outside 185..256; a ceiling outside it is never dealt\n') (read on the box: 7 of 13 failing, each on the ceiling)
A28: red at fff4d401: test_release_python_ceiling.py:345: AssertionError: Tuples differ: (0, '[366 chars]ng 189\nmutation: shards: run legs 4 of 221: p[68 chars]0\n') != (0, '[366 chars]ng 185\nmutation: shards: run legs 4 of 217: p[68 chars]0\n') (read on the box: 1 of 19 failing)
A25: red at ab8ee5ee: test_the_ceiling_is_the_runs_job_budget: AssertionError: {'ci': 189, 'battery': 234} != {'ci': 185, 'battery': 234} (job 127064675575)
A29: red at ab8ee5ee: test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler, read red by name (job 127064675575)
A30: red at ab8ee5ee: test_every_exclusion_is_one_row_with_its_reason, read red by name (job 127064675575)
```

A1 and A3 to A14 were committed with two stub scripts that keep every argument and decide
nothing (6883288c); each read red by name in the gate python stage of the pull request run. The
two scripts replaced the stubs in 6739ab7f, with every test unchanged, and the run at this
delivery head reads them green. The commit after 6739ab7f changes only this record.

The second part's criteria, A15 to A24, were committed with their tests before any job, bench,
list or exclusion they read. A20 to A24, in `test_perf_benches.py`, read red on any checkout at
that commit, each by an assertion on what is absent there: the bench profile, the workspace entry
of the renamed Divan crate, `perf/benchmarks.txt`, `vitest.bench.config.ts` and the Stryker
exclusion of bench files. A15 to A19, in `test_ci_workflows.py`, are read by name in the gate python
stage of that commit's pull request run, beside `test_the_gate_runs_in_four_parallel_jobs`, whose
sorted needs pin gains the four bench jobs in the same commit; the dynamic
`test_the_aggregate_check_needs_every_job_and_always_runs` stays green. None of the ten is a
control. Their red and green lines join the fence when each is read.

A15 to A24 were committed with their tests before any job, bench, list or exclusion they read
(efc7acfe); each read red by name in the gate python stage of that commit's pull request run. The
four bench jobs joined `ci` in 9897e72b (A15, A16, A18 and A19); the bench profile, the renamed
Divan crate and the Rust benches came in 12fd31c9 (A20 and A21), and the web benches and their
config in 3c68c795. The cargo-codspeed install moved into the Rust build job in 0832e745 (A17), the
web benches moved to `web/app/bench` in 89b6d5e1 (A23), and the benchmark list and the coverage
exclusions came in b3f776ba (A22 and A24); the run at this delivery head reads them green. A17's
body was restated in 0832e745 after its red was read at efc7acfe, and the restated body fails at
efc7acfe the same way, because the four bench jobs it reads are absent from that commit's ci.yml.
A23's root and A24's body were restated in 89b6d5e1 after their reds were read at efc7acfe, and
both restated tests fail at efc7acfe: A23 on the same line, since `vitest.bench.config.ts` is
absent there and no bench file lies under `web/app/bench`, and A24 because that commit's coverage
register holds neither bench row. The commit after b3f776ba changes only this record.

The third part's criteria, A25 to A30, hold the pull request run's Rust ceiling at the signed 185
(`docs/rulings/OWNER-RULING-2026-11-05-rust-ceiling-185.md`) and name the two words the bench
lanes need: `divan` among fsrs7's dev-dependencies, and `bench` among the kinds of a coverage
exclusion. A25, A29 and A30 read red by name in the `hygiene` job of the run at the delivery head
before this part, a tree whose ceiling is 189, whose fsrs7 pin admits no `divan` and whose
coverage kinds lack `bench`. A26 to A28 are the pins of the ceiling in `test_mutation_verdict.py`,
`test_release_rust_ceiling.py` and `test_release_python_ceiling.py`. They are re-pinned to 185 in
one commit before the ceiling moves, with A25's pins in `test_dispatch_shards.py` and the call
register's three entries in `test_ci_workflows.py`, and each of the three modules reads red there
on the ceiling alone. Their red and green lines join the fence when each is read.

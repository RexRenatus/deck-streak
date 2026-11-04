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
```

A1 and A3 to A14 were committed with two stub scripts that keep every argument and decide
nothing (6883288c); each read red by name in the gate python stage of the pull request run. The
two scripts replaced the stubs in 6739ab7f, with every test unchanged, and the run at this
delivery head reads them green. The commit after 6739ab7f changes only this record.

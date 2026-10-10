# Red-first record: SPEC-397

The first push carries the tests and a stub of `scripts/swift_mutants.py` alone, so each criterion
is read red by name in CI's `hygiene` job; the second push carries the module, the workflow edits
and this record's lines.

Every red below is the assertion's own line from the first failing subcase, read by the test's
name in the python stage of run 38002167046's `hygiene` job (job 114062570202), at the first
push's head, 3beb65219c09f5f24738cae0c2695365dde1681a. That stage failed 29 tests: A1 to A11's
subcases (1, 1, 7, 6, 1, 1, 2, 2, 1, 4 and 1), the R14 pin and the planned-specs test named below.
At that head the stub's `census` read nothing and exited 3, `retired` refused nothing, `sweep`
judged nothing, `verdict()` answered VOID for every class, and one import from outside the
standard library sat in a function nothing calls.

```red-first
A1: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: 'examined 0 row(s) in 0 file(s)\n' != 'examined 67 row(s) in 2 file(s)\n' (run 38002167046, job hygiene 114062570202)
A1: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A2: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Tuples differ: (3, 'examined 0 row(s) in 0 file(s)\n') != (0, 'examined 2 row(s) in 2 file(s)\n') (run 38002167046, job hygiene 114062570202)
A2: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A3: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Lists differ: [] != [('killer-form', 'ios/Alpha/swift-mutants.json', 'SW00001')] (run 38002167046, job hygiene 114062570202; 7 subcases red)
A3: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A4: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Lists differ: ['examined 0'] != ['examined 3'] (run 38002167046, job hygiene 114062570202; 6 subcases red)
A4: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A5: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Regex didn't match: '^SW00010:\\ KILLED, \\d+\\.\\d s$' not found in 'SW00010: none' : [] (run 38002167046, job hygiene 114062570202)
A5: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A6: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: 'VOID: the stub judges nothing' != 'VOID: its find occurs 0 times' (run 38002167046, job hygiene 114062570202)
A6: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A7: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Lists differ: [] != ['card-isolation'] (run 38002167046, job hygiene 114062570202; 2 subcases red)
A7: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A8: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: False is not true : card-isolation: timeout-minutes 30, not 5 to 10 (run 38002167046, job hygiene 114062570202; 2 subcases red)
A8: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A9: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: {'pul[239 chars]b/workflows/apple-on-change.yml']}} != {'pul[239 chars]b/workflows/apple-on-change.yml', 'scripts/swift_mutants.py']}} (run 38002167046, job hygiene 114062570202)
A9: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A10: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Lists differ: ['cal[46 chars]AD^1'] != ['cal[46 chars]AD^1', 'called scripts/swift_mutants.py retired --base HEAD^1'] (run 38002167046, job hygiene 114062570202; 4 subcases red)
A10: green at 5db3288859caf7ab141d54f0c639212120e85bc8
A11: red at 3beb65219c09f5f24738cae0c2695365dde1681a: AssertionError: Lists differ: [(112, 'yaml')] != [] (run 38002167046, job hygiene 114062570202)
A11: green at 5db3288859caf7ab141d54f0c639212120e85bc8
```

The greens are at the fix commit, 5db3288859caf7ab141d54f0c639212120e85bc8. A1 to A6 and A11 (the
whole of `test_swift_mutants.py`, 7 tests) and A10 (the whole of `test_mutation_workflows.py`, 29
tests) were run from `scripts/tests` at that commit and passed. A7, A8 and A9 live in
`test_ci_workflows.py`, which runs only in CI, so their greens are read by name in the `hygiene` job
of the second push. That head leaves `test_ci_workflows.py`, the module and the three workflows as
the fix commit wrote them, so A7 to A9 read the fix commit's bytes there. No test changed between
its red and its green: the fix commit touches the module, the three workflows and the SPEC's status
line, and no file under `scripts/tests/`.

The card view's sweep predicate (SPEC-397 R14) is no criterion of SPEC-397. Its pin,
`test_the_card_probe_suite_runs_on_both_simulators`, was red at the first push's head, by
`AssertionError: Lists differ: ['card-isolation: it sweeps no mutant of ios/CardIsolation'] != []`
(run 38002167046, job hygiene 114062570202), because the predicate reads the module's invocation and
the workflow still ran the inline program. It reads green with A7 at the second push's head.

`test_planned_specs.PlannedSpecsNeverCollide.test_no_judged_spec_reads_planned` is no criterion of
SPEC-397 either. It was red at the first push's head, by `AssertionError: Lists differ:
['SPEC-397-every-swift-mutant-row-is-held-[57 chars].md'] != []` (run 38002167046, job hygiene
114062570202), because the SPEC's status line opened with the word "planned". The fix commit,
5db3288859caf7ab141d54f0c639212120e85bc8, rewrites that one line to open with "delivered", and the
test is read green by name in the `hygiene` job of the second push.

The bound's constants (SPEC-397 R8) were measured on one successful run of the change caller,
durations only:

- harness-wire's seconds outside its sweep: 61 s (run 37967458391, job 113946410023)
- card-isolation's seconds outside its sweep: 45 s (run 37967458391, job 113946410059)
- the seconds per killer run: 116 s over 46 runs, card-isolation's sweep step (run 37967458391, job 113946410059)
- harness-wire's sweep step read 76 s over 35 runs, the smaller (run 37967458391, job 113946410023)

From them and the live counts, `harness-wire` takes `timeout-minutes: 5` and `card-isolation` takes
`timeout-minutes: 10`, and each sweep takes `--run-seconds 60`.

The rows S39700 to S39720 in `scripts/mutation-rows.d/S39700-S39799.json` hold the module's
decisions and the changed workflow lines, each killed by one criterion's test. This delivery adds,
removes and re-anchors no Swift row, so no package's sweep line is recorded here; the second push's
run of the change caller is the module's first sweep of both packages, and its two last lines are
read by name from the `harness-wire` and `card-isolation` job logs.

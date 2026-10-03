# Red-first record: SPEC-298

Recorded 2026-09-30. The nine tests of A1 to A9 live in a new module. They were committed alone
(0154acbc) with no change to the deploy tests, so each new name they read is absent and they fail
by assertion, never by an import or attribute error. The test of A2 read its record file inside an
eagerly built message, so at that commit it ended in an error (a missing file) and not in a failed
assertion; 52cd5f5d reads the record only when a program ran, and at 52cd5f5d it fails by assertion.
The stand-in, the helper and the list (0eb6d89b) turn them green.

The failing lines of the red CI run at 52cd5f5d, job hygiene, `Ran 662 tests`, `FAILED (failures=8)`:

```text
A1: AssertionError: unexpectedly None : the stand-in's allowed shapes are not listed: HOST_ALLOWED
A2: AssertionError: '<dir>/host: line 3: exec: : not found\n' != 'host stand-in: refusing : not a command the deploy tests use\n'
A3: AssertionError: unexpectedly None : the stand-in's allowed shapes are not listed: HOST_ALLOWED
A4: AssertionError: False is not true : the stand-in logged no argv[0]
A5: AssertionError: False is not true : no one helper starts a deploy script: launch
A6: AssertionError: False is not true : no one helper starts a deploy script: launch
A7: AssertionError: Items in the first set but not the second:
A8: AssertionError: 0 != 1 : no one helper starts a deploy script: launch
```

The round-1 verifier found four of the checks SPEC-298 relies on passed members of their own class. A10 to A13
are the killers added for them, committed alone at b4c65f62 with the checks still the old ones, so each fails
by assertion. The job hygiene of that commit printed `Ran 666 tests`, `FAILED (failures=4)`; the four failing
lines are the A10 to A13 lines below, each naming every member the old check passed (the stub scan passed 27 of
its 35 members, the other 8 were already refused). The green was measured by CI at the commit that adds this
record, whose test code equals that of 47f31731. The first green push (59e4752f) still failed one test by an
error and not by an assertion, a defect in the body of the A7 test, printed by CI as
`TypeError: 'int' object is not iterable` (the test handed a count where a population was expected); 47f31731
repairs that body and nothing else. The A7 body has therefore changed since its red commit, and the failure
of its newest body against the old checks was not measured; the red line of A7 below is the one its first
body printed.

The bodies of the tests of A5 and A6 changed after their red commits: they pass an `env=` to the helper. That
keyword already existed in the old helper, so the new bodies do not fail against it, and the failing evidence for
the refusal of an environment the program will see is the A13 line. No failure of the new bodies was measured
before the change.

```red-first
A1: red at 0154acbc: AssertionError: unexpectedly None : the stand-in's allowed shapes are not listed: HOST_ALLOWED
A1: green at 47f31731
A2: red at 52cd5f5d: AssertionError: '<dir>/host: line 3: exec: : not found\n' != 'host stand-in: refusing : not a command the deploy tests use\n'
A2: green at 47f31731
A3: red at 0154acbc: AssertionError: unexpectedly None : the stand-in's allowed shapes are not listed: HOST_ALLOWED
A3: green at 47f31731
A4: red at 0154acbc: AssertionError: False is not true : the stand-in logged no argv[0]
A4: green at 47f31731
A5: red at 0154acbc: AssertionError: False is not true : no one helper starts a deploy script: launch
A5: green at 47f31731
A6: red at 0154acbc: AssertionError: False is not true : no one helper starts a deploy script: launch
A6: green at 47f31731
A7: red at 0154acbc: AssertionError: Items in the first set but not the second:
A7: green at 47f31731
A8: red at 0154acbc: AssertionError: 0 != 1 : no one helper starts a deploy script: launch
A8: green at 47f31731
A9: not red: at b4c65f62 it passed, because the stubs of the module that run their first argument were already the ones it lists
A10: red at b4c65f62: AssertionError: Lists differ: ['route: a semicolon after the list', 'rou[793 chars]ute'] != [] : 27 of 35 stub member(s) passed the check
A10: green at 47f31731
A11: red at b4c65f62: AssertionError: Lists differ: ['host call: the host array unquoted, then[711 chars]ron'] != [] : 16 of 16 derivation member(s) passed the check
A11: green at 47f31731
A12: red at b4c65f62: AssertionError: Lists differ: ['start: subprocess.getstatusoutput', 'sta[579 chars]ing'] != [] : 16 of 16 launch census member(s) passed the check
A12: green at 47f31731
A13: red at b4c65f62: AssertionError: Lists differ: ['received names it; env is a mapping with[208 chars] it'] != [] : 5 of 5 launch member(s) passed the check
A13: green at 47f31731
```

## Not measured locally

The deploy class is run by CI only. Nothing here ran the new module, the deploy tests, either
deploy script or any row that targets the deploy script; only a compile, ruff, shellcheck and reading were
done, and pure syntax-tree drivers that import only the check functions and no deploy code. NOT MEASURED
LOCALLY: the green lines above, the verdicts of rows S29800 to S29813, and the launch refusal of A13.

The route that hands a stub's arguments to the shell's own evaluator has no member in any population. It is
closed by construction and not planted: the stub scan refuses any argument expansion outside the reviewed
logging shapes, so no spelling of it needs listing.

## Disclosure: a commit between red and green that changes a test file

Commit 06987a2a (`test(deploy): the four class checks find every member or refuse it`) changes the
test files `scripts/tests/test_deploy_scripts.py`, `scripts/tests/test_deploy_standin.py` and
`scripts/tests/_standin_checks.py`, and the mutation rows, between the red commits and the green
commit of the criteria above. The pre-flight's red-first check reports that it rewrites an assertion
in the test files. The rewrite was the builder's, in an earlier round; its content is not re-read
in this fix round, which is kept away from those test bodies by the delivery's fence. This note
discloses the change from the commit's metadata (subject and file list) alone and quotes no assertion.

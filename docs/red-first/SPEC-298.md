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

```red-first
A1: red at 0154acbc: AssertionError: unexpectedly None : the stand-in's allowed shapes are not listed: HOST_ALLOWED
A1: green at 0eb6d89b
A2: red at 52cd5f5d: AssertionError: '<dir>/host: line 3: exec: : not found\n' != 'host stand-in: refusing : not a command the deploy tests use\n'
A2: green at 0eb6d89b
A3: red at 0154acbc: AssertionError: unexpectedly None : the stand-in's allowed shapes are not listed: HOST_ALLOWED
A3: green at 0eb6d89b
A4: red at 0154acbc: AssertionError: False is not true : the stand-in logged no argv[0]
A4: green at 0eb6d89b
A5: red at 0154acbc: AssertionError: False is not true : no one helper starts a deploy script: launch
A5: green at 0eb6d89b
A6: red at 0154acbc: AssertionError: False is not true : no one helper starts a deploy script: launch
A6: green at 0eb6d89b
A7: red at 0154acbc: AssertionError: Items in the first set but not the second:
A7: green at 0eb6d89b
A8: red at 0154acbc: AssertionError: 0 != 1 : no one helper starts a deploy script: launch
A8: green at 0eb6d89b
A9: not red: the only stub that runs its first argument is the host stand-in already at the development branch
```

## Not measured locally

The deploy class is run by CI only. Nothing here ran the new module, the deploy tests, either
deploy script or any row that targets the deploy script on this machine; only a compile, ruff and
reading were done. The green lines above and the verdicts of rows S29800 to S29808 are CI's.

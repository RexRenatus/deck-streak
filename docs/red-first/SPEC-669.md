# Red-first record: SPEC-669

The red commit holds the census's tests, its planted fixtures, the workflow test's one inserted
name and a census whose comparison returns `clean` for every suite and whose readers return no
figure; the green commit holds the census and the job the aggregate check needs.

```red-first
A1: not red: the red commit's census already reads every suite clean; the test guards the comparison against a false site
A2: red at 1d5ac45: AssertionError: 'clean' != 'site'
A2: green at ea23980
A3: not red: the red commit's census already reads every suite clean; the test guards the margin's floor
A4: red at 1d5ac45: AssertionError: 'clean' != 'faster'
A4: green at ea23980
A5: red at 1d5ac45: AssertionError: 'clean' != 'untimed'
A5: green at ea23980
A6: red at 1d5ac45: AssertionError: Lists differ: [] != [('nextest', 'alpha::removed')]
A6: green at ea23980
A7: red at 1d5ac45: AssertionError: AssertionError not raised
A7: green at ea23980
A8: red at 1d5ac45: KeyError: ('nextest', 'alpha::over')
A8: green at ea23980
A9: red at 1d5ac45: AssertionError: Lists differ: ['bas[277 chars]', 'rust', 'web', 'web-engine', 'workflow-lint'] != ['bas[277 chars]', 'rust', 'suite-times', 'web', 'web-engine', 'workflow-lint']
A9: green at ea23980
```

The green commit `ea23980` carries the census, and `git diff --quiet ea23980 HEAD` over the module and its planted files reads rc 0.

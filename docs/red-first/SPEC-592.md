# SPEC-592: the red-first record

```red-first
A6: not red: its census tests pin properties config/coverage.json and the Rust run flags hold from their first commit, each with planted controls the census refuses
A8: not red: its census tests pin the Python lane suites, the hash-pinned requirements file and the ctrace core, which hold from their first commit, each with planted controls
A9: not red: its census tests pin the vitest coverage block and the catalog entry, which hold from their first commit, each with planted controls
A10: not red: its census test pins the Dependabot pip entry, which holds from its first commit, with planted controls the census refuses
A1: red at d5856b0f: test_coverage_report.py:110: AssertionError: 'coverage-report: lane python examined 1 lines in 1 files' not found in 'coverage-report: lane python examined 1 lines\nchanged executable lines 0, executed 0, not executed: none\ncove (job 126122406615)
A1: green at b33b74bb
A2: red at d5856b0f: test_coverage_report.py:120: AssertionError: 'not executed: scripts/planted_control.py:3' not found in 'coverage-report: lane python examined 3 lines\nchanged executable lines 0, executed 0, not executed: none\ncovered 0.0%\n' (job 126122406615)
A2: green at b33b74bb
A3: red at d5856b0f: test_coverage_report.py:228: AssertionError: 'coverage-provider: @vitest/coverage-v8 5.0.5 equals vitest 5.0.5' not found in 'coverage-provider: @vitest/coverage-v8 0 equals vitest 0\n' (job 126122406615)
A3: green at b33b74bb
A4: red at d5856b0f: test_coverage_report.py:175: AssertionError: 0 != 1 : an empty file: coverage-report: lane python examined 0 lines (job 126122406615)
A4: green at b33b74bb
A5: red at d5856b0f: test_coverage_report.py:259: AssertionError: 'diff' != 'whole' (job 126122406615)
A5: green at b33b74bb
A7: red at d5856b0f: test_coverage_report.py:516: AssertionError: 0 != 1 : coverage-report: lane python examined 2 lines (job 126122406615)
A7: green at b33b74bb
A11: red at d5856b0f: test_changelog_fragment.py:189: AssertionError: 1 != 0 : Add a fragment under changelog.d/ (see changelog.d/README.md). (job 126122406615)
A11: green at 93be50ad
```

A1 to A5 and A7 were committed with a stub report that keeps every argument and lacks each
behaviour, and A11 with the `fragment` step as it was (d5856b0f); each read red by name in the
gate python stage of the pull request run. The report replaced the stub in b33b74bb and the step
gained the requirements file in 93be50ad, with every test unchanged, and the run at this delivery
head reads them green. 1e4ab9e9 then added one assertion to the exclusion list test, which a
surviving mutation row showed was missing; it changes no criterion test. The commits after
1e4ab9e9 change only documents.

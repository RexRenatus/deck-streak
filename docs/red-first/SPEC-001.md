# Red-first record: SPEC-001

Each criterion's test was run before the change that makes it pass, and the failure read (the tdd
pack). The shas are commits on this repository's first-parent history.

```red-first
A1: not red: the matrix and its test are generated from one data source in one step; the test pins the population of 131 predecessor ids against later drift
A2: red at e05dfa5: AssertionError: Regex didn't match: '#\d+' not found in '(issue pending)' : `inert-churn-tax` names no owner decision issue
A2: green at d54db93
A3: red at e05dfa5: AssertionError: 'pipeline' not found in the declared contexts : `cron-ledger-and-scheduler` names context pipeline
A3: green at ce3683d
A4: red at e05dfa5: AssertionError: [] is not true : `anki-sync-download` names no issue
A4: green at d54db93
A5: not red: the second-brain rows and their test are generated from one data source in one step; the test pins all 22 against later drift
```

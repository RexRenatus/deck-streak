# Red-first record: SPEC-035

The tests were committed (263b9cd) before the setup script and the documents changed. A1 and A3 were
run there for their own reasons. A2 pins calls the script already made, so it is disclosed as not
red.

```red-first
A1: red at 263b9cd: AssertionError: Lists differ: ['api -X PUT repos/owner/name/automated-security-fixes'] != ['api -X DELETE repos/owner/name/automated-security-fixes']
A1: green at 619b6d7
A2: not red: the setup already kept alerts, secret scanning and private reporting on; the test pins them beside A1's reversal
A3: red at 263b9cd: AssertionError: 'automated security fixes stay off' not found in SECURITY.md
A3: green at 619b6d7
```

A1 and A2 run `scripts/github-setup.sh security` against a fake `gh` that records its calls, with
stdin closed, so no test ever reaches GitHub.

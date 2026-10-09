# Red-first record: SPEC-396

SPEC-396 (R1 to R13, A1 to A9). The SPEC, ADR-410, the tests of A1 to A9 and a stub
`deploy/scripts/second-route.sh` that declares its two credential ids and makes no request were
committed alone (f63c60dbcd2f5ed50c5272403c95b63fef2c22d5) and pushed alone. Their modules execute
deploy scripts, so they never run on the build host: each red below is quoted from CI's run at that
commit, the `hygiene` job's `python` stage log (run 37993302270, job 114032777749, artifact
`check-stage-logs-hygiene`, `python.log`, which reads `FAILED (failures=10)` over 1105 tests). Each
of the nine failed by its own assertion, never by an error. The second route's script, unit, timer,
census rows, host budget, rail contract, README, schematic citations and model were committed next
(50d4d1912814d6c92046956db720c408e36c84ee), and the nine are green from that commit on, read in
CI on the delivery's second push.

```red-first
A1: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: 0 != 2 : one report, then one check-in
A2: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: 0 != 1 : one request, the check-in
A3: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: False is not true : the second route's unit is not shipped
A4: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: False is not true : the second route's unit is not shipped
A5: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: Lists differ: [] != [['https://report.invalid/decade'], ['https://checkin.invalid/c0ffee']]
A6: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: Lists differ: [] != [['https://report.invalid/decade'], ['https://checkin.invalid/c0ffee']]
A7: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: 0 != 1 : (the first run's exit status, against the stub that printed no line)
A8: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: 0 != 1 : second-route-check-in holding ''
A9: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: 'deck-streak-second-route.service' not found in {'deck-streak-alert@.service': Unit(name='deck-streak-alert@.service', ...), ...}
A1: green at 50d4d1912814d6c92046956db720c408e36c84ee
A2: green at 50d4d1912814d6c92046956db720c408e36c84ee
A3: green at 50d4d1912814d6c92046956db720c408e36c84ee
A4: green at 50d4d1912814d6c92046956db720c408e36c84ee
A5: green at 50d4d1912814d6c92046956db720c408e36c84ee
A6: green at 50d4d1912814d6c92046956db720c408e36c84ee
A7: green at 50d4d1912814d6c92046956db720c408e36c84ee
A8: green at 50d4d1912814d6c92046956db720c408e36c84ee
A9: green at 50d4d1912814d6c92046956db720c408e36c84ee
```

The same run read one more failure at the first commit, an existing test and no criterion of this
SPEC: A9's test adds lines to `scripts/tests/test_deploy_templates.py` above seven lines that the
threat-model schematic cites, and the second commit re-derives each of the seven citations to the
line that now holds its quote, removing none. It is quoted here, below the criteria, so that no
criterion is recorded twice.

```text
test_threat_model.TheModelHolds.test_every_control_cites_a_line_that_holds: red at f63c60dbcd2f5ed50c5272403c95b63fef2c22d5: AssertionError: Lists differ: 7 findings, the first 'docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md:90: T3: quote not on a cited line: scripts/tests/test_deploy_templates.py:2049:def test_every_service_carries_the_hardening_r2_names'
test_threat_model.TheModelHolds.test_every_control_cites_a_line_that_holds: green at 50d4d1912814d6c92046956db720c408e36c84ee
```

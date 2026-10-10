# Red-first record: SPEC-396

SPEC-396 (R1 to R13, A1 to A9). The SPEC, ADR-410, the tests of A1 to A9 and a stub
`deploy/scripts/second-route.sh` that declares its two credential ids and makes no request were
committed alone (f63c60dbcd2f5ed50c5272403c95b63fef2c22d5) and pushed alone. Their modules execute
deploy scripts, so they never run on the build host: each red below is quoted from CI's run at that
commit, the `hygiene` job's `python` stage log (run 37993302270, job 114032777749, artifact
`check-stage-logs-hygiene`, `python.log`, which reads `FAILED (failures=10)` over 1105 tests). Each
of the nine failed by its own assertion, never by an error. The second route's script, unit, timer,
census rows, host budget, rail contract, README, schematic citations and model were committed next
(50d4d1912814d6c92046956db720c408e36c84ee), and eight of the nine are green from that commit on,
read in CI on the delivery's second push. A6 is green from the commit its line names, which reads
the service manager's list on a descriptor of its own (the note below the criteria).

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
A6: green at 2b737847c9802fad0a35533c20cbeb5c09b236c8
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

A6 read red once more on the second push (cf4833e5e8e533b230bea20defccd2852e9c0574), in the
`hygiene` job's `python` stage log (run 38003582876, job 114067126393, artifact
`check-stage-logs-hygiene`, `python.log`, which reads `FAILED (failures=1)` over 1105 tests), by its
own assertion on the read too long for one report: the report named one key and no count line. The
test and the script at 50d4d1912814d6c92046956db720c408e36c84ee are byte for byte those of that
push, so A6 was not green there. The script read the service manager's list of failed alert
instances on its standard input, which each command it ran inherited, so a command that reads its
input, as the test's stand-in for the service manager does, took the rest of the list with the
first invocation id it was asked for. 2b737847c9802fad0a35533c20cbeb5c09b236c8 reads the list on
descriptor 3 and changes no test; A6's one green line names it, and neither the test nor the script
changes after it in this delivery. Between A6's red and that green, one more commit changes a test
module: 220e93be5b4f9258ff161067ec8107716c175e22 moves `scripts/tests/test_formal_config.py`'s
`EXPECTED` with `config/formal.json`, the per-run cap from 300 to 480 and a `tla/SecondRoute` budget
of 480 added, and removes no assertion; A6's module is not among the files it changes. The replay is
quoted here, below the criteria, so that no criterion is recorded twice.

```text
test_alert_unit.ASecondRouteTellsTheOwner.test_each_failed_invocation_is_reported_once: red at cf4833e5e8e533b230bea20defccd2852e9c0574: AssertionError: Lists differ: [] != ['failed deck-streak-alert@deck-streak-uni[51 chars]f90']
```

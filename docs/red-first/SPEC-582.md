# Red-first record: SPEC-582

SPEC-582 and ADR-601 were committed first (016515d9). Every criterion test the base could compile
was committed next and alone (9ac8302e), over the base's production code and deploy files
unchanged; beside the tests it changes only the killers three mutation rows name
(`scripts/mutation-rows.d/S06200-S06299.json`, `scripts/mutation-rows.d/S33300-S33399.json`),
which follow their renamed tests. That commit is the first push. The change follows at b2b891d1,
a change to the alert sender and its tests at 3fcff046, and the rows of
`scripts/mutation-rows.d/S58200-S58299.json` at 04cf9bfa. Each red below is its criterion's own
assertion at 9ac8302e, never a missing name.

## The fence, line by line

Each of the 22 lines of SPEC-582 section 3's fence resolves to one test, selected by `--exact` or
`-k`.

| # | criterion | test file | added or named |
|---|---|---|---|
| 1 | A1 | `crates/daemon/tests/lifecycle.rs` | added at 9ac8302e |
| 2 | A2 | `crates/daemon/tests/lifecycle.rs` | added at 9ac8302e |
| 3 | A3 | `crates/daemon/tests/lifecycle.rs` | named: body unchanged |
| 4 | A4 | `crates/identity/tests/owner.rs` | added at b2b891d1 |
| 5 | A5 | `crates/identity/tests/owner.rs` | added at b2b891d1 |
| 6 | A6 | `crates/api/tests/telegram_off.rs` | added at b2b891d1 |
| 7 | A6 | `crates/api/tests/telegram_off.rs` | added at b2b891d1 |
| 8 | A8 | `crates/daemon/tests/roles.rs` | added at 9ac8302e |
| 9 | A9 | `crates/notifications/tests/one_router.rs` | named: body changed at 9ac8302e |
| 10 | A10 | `scripts/tests/test_deploy_templates.py` | added at 9ac8302e |
| 11 | A11 | `scripts/tests/test_deploy_templates.py` | renamed at 9ac8302e |
| 12 | A12 | `scripts/tests/test_deploy_templates.py` | renamed at 9ac8302e |
| 13 | A13 | `scripts/tests/test_alert_unit.py` | added at 9ac8302e; body changed at 3fcff046 |
| 14 | A14 | `scripts/tests/test_alert_unit.py` | added at 9ac8302e |
| 15 | A15 | `scripts/tests/test_alert_unit.py` | added at 3fcff046, in place of two tests named at 9ac8302e |
| 16 | A16 | `scripts/tests/test_slo_declaration.py` | named: body changed at 9ac8302e |
| 17 | A17 | `scripts/tests/test_slo_declaration.py` | renamed at 9ac8302e |
| 18 | A18 | `scripts/tests/test_deploy_scripts.py` | added at 9ac8302e |
| 19 | A19 | `scripts/tests/test_deploy_templates.py` | named: body changed at 9ac8302e |
| 20 | A20 | `scripts/tests/test_rail_contract.py` | added at 9ac8302e |
| 21 | A21 | `scripts/tests/test_deploy_templates.py` | renamed at 9ac8302e |
| 22 | A22 | `scripts/tests/test_deploy_templates.py` | named: body changed at 9ac8302e |

## Where each red was read

A1, A2, A8 and A9 were run by their fence lines at 9ac8302e on a local checkout, and each failed
on the assertion its line below quotes. The first push's CI run, 41233885736, read the same four
failures and no other Rust test failure in its `rust` job (124633483975) and its `release` job
(124633484108); A3 passed in both. The test modules of A10 to A22 are read in CI: the `hygiene`
job (124633484126) of the same run read `scripts/tests` with 21 failures, among them the thirteen
tests A10 to A21 then had (A15 was two tests), and A22 passed. The module of A16 and A17 also
runs locally, and both failed there at 9ac8302e on the same assertions. Each CI quote below is from the stage log its job
attaches (`test.log`, `test-release.log`, `python.log`), where the failing assertion is printed:
unittest's own `[N chars]` cuts are kept, and in a quoted JSON log line `...` stands for the
fields left out. A21 quotes the assertion its traceback names, at
`scripts/tests/test_deploy_templates.py:1362`, in place of its `AssertionError` line, which lists
every shipped unit.

The other eight `hygiene` failures at 9ac8302e are not criteria. Six are tests whose
expectations 9ac8302e moved to the change's tree beside its criteria, each made true by b2b891d1
and read in CI at the second push: `test_the_render_refuses_placeholders_and_keeps_the_headers`
in `scripts/tests/test_caddy_render.py`,
`test_the_deploy_installs_beside_and_switches_current_atomically` in
`scripts/tests/test_deploy_scripts.py`, and
`test_every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal`,
`test_no_unit_passes_a_secret_through_its_environment`,
`test_every_service_runs_its_role_with_the_lifecycle_r1_names` and
`test_the_sync_server_runs_hardened_within_its_entry_and_the_share_holds_it` in
`scripts/tests/test_deploy_templates.py`. Two follow from 9ac8302e's own edits: the mutation row
census, because row S04113's find was the exemption line A9's test drops, and the threat model's
cites, because 9ac8302e's insertions moved seven lines of `scripts/tests/test_deploy_templates.py`
that its stride table cites. b2b891d1 anchors S04113 on the refusal loop that now holds every
source outside the bot, and points the seven cites in
`docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md`
at the lines where their tests now stand. The same run's `mutation-verdict` job read 47 rows VOID
because each one's killer is among the reds above, and S04113 VOID for its anchor.

```red-first
A1: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: panicked at crates/daemon/tests/lifecycle.rs:837:9: the api role refused start with neither credential: the api role did not serve; its lines: ["<3>{...\"error\":\"the api role: the credential owner-user-id is missing from the credentials directory\"...}"]
A1: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A2: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: panicked at crates/daemon/tests/lifecycle.rs:852:33: the api role refused start with no credentials: the api role did not serve; its lines: ["<3>{...\"error\":\"the api role: the setting CREDENTIALS_DIRECTORY is required and is not set\"...}"]
A2: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A3: not red: pins the base's refusal of half a pair; its red is hand row S58202, killed by A3
A4: not red: a function the first push cannot compile; its red is hand rows S58201 and S58202, killed by A4
A5: not red: a function the first push cannot compile; its red is hand row S58205, killed by A5
A6: not red: functions the first push cannot compile; its red is hand rows S58203 and S58204, killed by A6
A8: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: panicked at crates/daemon/tests/roles.rs:1057:5: assertion `left == right` failed: exit status: 1; stdout: <3>{..."error":"the job role: the setting CREDENTIALS_DIRECTORY is required and is not set"...}
A8: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A9: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: panicked at crates/notifications/tests/one_router.rs:2848:5: assertion `left == right` failed: a delivery goes around the port; left: ["deploy/scripts/alert-telegram.sh:68: names api.telegram.org", "deploy/scripts/alert-telegram.sh:68: names sendMessage"]
A9: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A10: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: Lists differ: ['deck-streak-bot.service', 'deck-streak-j[49 chars]ath'] != []
A10: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A11: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: Lists differ: ['deploy/systemd/deck-streak-alert@.servic[369 chars]onf'] != []
A11: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A12: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: "frame-ancestors https://web.telegram.org; object-src 'none'; base-uri 'self'" != "frame-ancestors 'none'; object-src 'none'; base-uri 'self'"
A12: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A13: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: Lists differ: ['/usr/local/lib/deck-streak/current/deploy/scripts/alert-telegram.sh %i'] != ['/usr/local/lib/deck-streak/current/deploy/scripts/alert-journal.sh %i']
A13: green at 3fcff046a3049d1b73b29accc1dd399b60f90c93
A14: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: Lists differ: ['owner-user-id', 'telegram-bot-token'] != []
A14: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A15: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: each of its two tests then: Lists differ: ['/usr/local/lib/deck-streak/current/deploy/scripts/alert-telegram.sh %i'] != ['/usr/local/lib/deck-streak/current/deploy/scripts/alert-journal.sh %i']
A15: green at 3fcff046a3049d1b73b29accc1dd399b60f90c93
A16: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: {'unit': 'deck-streak-alert@.service', 'channel': 'telegram'} != {'unit': 'deck-streak-alert@.service', 'channel': 'journal'}
A16: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A17: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: {'pag[25 chars].6, 'telegram'), 'ticket': ('3d', '6h', 0.1, 0[14 chars]am')} != {'pag[25 chars].6, 'journal'), 'ticket': ('3d', '6h', 0.1, 0.9333, 'journal')}
A17: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A18: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: 1 != 0 : effective-check: examined 3 unit(s), 4 file(s); 0 refusal(s); deploy: deck-streak-bot.service did not restart
A18: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A19: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: Lists differ: ['dec[31 chars]reak-bot.service', 'deck-streak-litestream.ser[63 chars]ice'] != ['dec[31 chars]reak-litestream.service', 'deck-streak-mcp.ser[36 chars]ice']
A19: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A20: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: Lists differ: [{'unit': 'deck-streak-bot.service', 'sect[529 chars]C']}] != []
A20: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A21: red at 9ac8302ec5cb97f4d4e80b6e268d03e1eaa51b59: scripts/tests/test_deploy_templates.py:1362: self.assertNotIn(service, shipped, "the second route's unit is shipped")
A21: green at b2b891d15ce6a9f99d13b0400de850b16b915b91
A22: not red: pins the identity census over the alert template; its red is row S58210, killed by A22
```

## The greens

Each green line names the commit whose change turns its criterion green: 3fcff046 for A13 and
A15, whose tests it changes, and b2b891d1 for every other red criterion. No Rust source or test
changes after b2b891d1 (`git diff --name-only b2b891d1 HEAD -- crates` lists nothing), and on this
record's parent, 04cf9bfa, the fence lines of A1 to A6, A8 and A9 each read `ok` in their
targets' whole runs: `--test lifecycle` 12 passed and 1 ignored (ignored on the base as well),
`--test roles` 20 passed, `--test owner` 6 passed, `--test telegram_off` 2 passed,
`--test one_router` 12 passed, `--test session_routes` 14 passed and `--test linking_routes` 8
passed. `test_slo_declaration` read 8 tests OK, A16's and A17's among them; neither that module
nor `deploy/slo.json` changes after b2b891d1. The other criteria's modules run only in CI, which
reads them first at the second push, the first run over a commit that carries the change. After
b2b891d1, 3fcff046 also changes the alert sender and four comment lines of the alert template,
which other criteria's tests read as well (A10, A11, A14 and A22 among them), so CI reads every
criterion over both commits.

## Commits between red and green

b2b891d1, the change, also changes test files, and changes no test of a recorded criterion
between 9ac8302e and itself, compared function by function. It adds the tests of A4 to A6 in
`crates/identity/tests/owner.rs` and `crates/api/tests/telegram_off.rs`. It deletes
`scripts/tests/test_sync_path.py`, whose five tests read the doorbell that leaves. In
`scripts/tests/test_alert_unit.py` it deletes the three tests that read the Telegram sender's
credentials, its command line and an empty credential, moving their alert-template plants whole
into the key-off test; it points `test_a_json_error_line_reaches_the_page_as_written` at the
journal line; it drives the second route's test with the journal sender failing on a closed
standard error; and it lets the two share-nothing tests read a planted unit text. In
`scripts/tests/test_deploy_templates.py` it removes the table entries of the bot's unit and the
second route's unit, the two Telegram credentials' sources, and the held flush from the instance
drop-in allowlist, with the units the change removes.

3fcff046 changes A13's test: its expected line loses the quoted suffix, and its journal-read
assertions become the assertion that the sender reads no journal. It replaces A15's two tests
with one, `test_the_alert_line_carries_no_line_of_the_failed_run`, and deletes
`test_a_json_error_line_reaches_the_page_as_written`, whose quote leaves. In
`scripts/tests/test_memory_watch.py` and its cgroup fixture, the bot's unit becomes the MCP
server's, with every test name, assertion and planted case kept.

A15 changed with the design at 3fcff046: the alert line no longer quotes the failed run's last
error lines, because error text in an alert can carry user data and the unit's own lines already
reach the journal. A15's red at 9ac8302e is the red of its two earlier tests, which failed on the
template's ExecStart line, the assertion A13's test also makes, before reaching the quote they
named. The red of the property A15 now states is its test's planted control: the sender as
b2b891d1 wrote it, byte for byte, which the test runs and requires to fail the same check. That
control, with row S58216, is read in CI at the second push.

## The rows

Each hand row was proved on a local checkout: the target's bytes and sha256 saved, the mutant
installed by script, the killer run by its fence line's form (`running 1 test`), its failing
assertion read, the saved bytes written back and the sha256 compared, equal for every row, and the
killer run again, green, on the restored tree.

| row | target | killer | under the mutant |
|---|---|---|---|
| S58201 | `crates/identity/src/owner.rs` | A4 | `crates/identity/tests/owner.rs:396:14`: `neither held refuses nothing: Credential(Missing { id: "owner-user-id" })` |
| S58202 | `crates/identity/src/owner.rs` | A4 | `crates/identity/tests/owner.rs:420:17`: `half a pair, owner-user-id missing, did not refuse start` |
| S58202 | `crates/identity/src/owner.rs` | A3 | `crates/daemon/tests/lifecycle.rs:304:13`: `the api role started without the credential owner-user-id` |
| S58203 | `crates/api/src/session_routes.rs` | A6 | `crates/api/tests/telegram_off.rs:105:13`: `/api/session` answered `(403, "{\"reason\":\"not_owner\"}")` where `(404, "{\"reason\":\"telegram_off\"}")` is required |
| S58204 | `crates/api/src/linking_routes.rs` | A6 | `crates/api/tests/telegram_off.rs:149:9`: `/api/link/code` answered `(404, "{\"reason\":\"linking_off\"}")` where `(404, "{\"reason\":\"telegram_off\"}")` is required |
| S58205 | `crates/identity/src/lib.rs` | A5 | `crates/identity/tests/owner.rs:463:5`: assertion `left == right` failed, left 401, right 404 |
| S58206 | `crates/daemon/src/role_job.rs` | A8 | `crates/daemon/tests/roles.rs:1057:5`: assertion `left == right` failed: exit status: 1 |
| S58215 | `crates/daemon/src/role_api.rs` | A2 | `crates/daemon/tests/lifecycle.rs:852:33`: `the api role refused start with no credentials: the api role did not serve` |
| S58213 | `deploy/slo.json` | A16 | `AssertionError: {'unit': 'deck-streak-alert@.service', 'channel': 'telegram'} != {'unit': 'deck-streak-alert@.service', 'channel': 'journal'}` |
| S04113 | `crates/notifications/tests/one_router.rs` | A9 | `crates/notifications/tests/one_router.rs:2684:5`: the census lacks the refusals `crates/api/src/notifications_routes.rs:3: names api.telegram.org` and `crates/api/src/notifications_routes.rs:3: names sendMessage` |

S58207, S58210, S58211, S58212, S58214 and S58216 have killers in modules that run only in CI;
CI's `mutation-rows` job reads each by its id at the second push. The mutants of the three shell
rows, S58207 and S58216 over `deploy/scripts/alert-journal.sh` and S58211 over
`deploy/deploy.sh`, each parse (`bash -n` exits 0 on each, and on a planted file with an unclosed
`if` it exits 2).

## Corrections

The amendments 016515d9 appends to ADR-347 and SPEC-396 cite SPEC-582's criteria by an earlier
draft's numbers: ADR-347's `SPEC-582 A14` is A19 here, the stack-share test, and SPEC-396's
`SPEC-582 A15` is A21, the second route's unit.

The changelog fragment's alert sentence states the line as 3fcff046 ships it: it names the failed
unit and its result. The design's draft of the fragment, written before 3fcff046, also named the
run's last error lines; that phrase alone is changed.

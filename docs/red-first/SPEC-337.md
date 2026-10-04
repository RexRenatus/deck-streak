# Red-first record: SPEC-337

SPEC-337, ADR-347 and the packaging schematic were committed first (55f27ca3). The two R1 tests and
their one census entry were committed alone (40bd535d), with the release workflow untouched. At that
commit both fail by assertion, because no step of the release's one job runs `cargo install`
(`FAILED (failures=2)`).

```red-first
A1: red at 40bd535d: AssertionError: no step has `cargo install` in its run
A2: red at 40bd535d: AssertionError: no step has `cargo install` in its run
A1: green at 39323fad
A2: green at 39323fad
```

At 39323fad the module reads `Ran 6 tests ... OK`, and A2 prints `examined 5 manifest roots`: the
pinned plant, the three refusals (no patch entry, a branch for a commit, a short commit) and the
tree's own manifest, each run through the step's own text under bash with cargo stubbed.

The unit's tests (A3, A4) and the launcher's (A5) were committed alone (33139e68), with no unit and
no launcher in the tree; at that commit the module reads `FAILED (failures=11)`, each by assertion.
A5's expected refusal for two users of one name was then changed, alone, to name no user
(3d443f29), and A5 was read red again there. The unit, the launcher, the budget entry and the share
followed (71c77362).

```red-first
A3: red at 33139e68: AssertionError: 0 != 1 : no deck-streak-sync-server.service under deploy/systemd
A4: red at 33139e68: AssertionError: 0 != 1 : no deck-streak-sync-server.service under deploy/systemd
A5: red at 3d443f29: AssertionError: False is not true : no launcher at deploy/scripts/sync-server.sh
A3: green at 71c77362
A4: green at 71c77362
A5: green at 71c77362
```

A5 was red at both commits, and the fence holds the second, the test as it went green, because the
probe records one red per criterion; the first:

```text
A5: red at 33139e68: AssertionError: False is not true : no launcher at deploy/scripts/sync-server.sh
```

At 71c77362 A4 prints `examined 573 file(s) under deploy/ and docs/` and A5 `examined 17 launcher
case(s)`: the two starts and fifteen refusals, each run through the launcher with the server
stubbed.

The route's tests (A6) were committed alone (d0fd24c7), with the block and render-caddy untouched:
render-caddy's test gives the configuration a fourth key, `sync_upstream`, and the block's test asks
for the two sync handles. At that commit both fail by assertion; the block's existing test fails too,
because it counts five handles. The deploy scripts' Caddy install gains the same fourth key there;
that module is decided by CI by name and never run on the box. The block's route and render-caddy's
fourth key followed (c949e7e5).

```red-first
A6: red at d0fd24c7: AssertionError: 1 != 0 : REFUSE: the configuration holds sync_upstream, which the block has no placeholder for
A6: green at c949e7e5
```

A6's second command was red at the same commit, by assertion:

```text
A6: red at d0fd24c7: AssertionError: unexpectedly None : no handle for /anki-sync
```

At c949e7e5 render-caddy's test prints `examined 4 required key(s)` and `examined 2 upstream
key(s)`, each upstream refused under its own key for twelve shapes, and test_deploy_templates reads
`Ran 28 tests ... OK`.

The deploy scripts' Caddy install (`test_deploy_scripts.py`) is decided by CI by name and never run
on the box, so its red at d0fd24c7 is CI's to read. d0fd24c7 was pushed alone for that reading, and
no workflow ran on it: the branch then conflicted with its base in `deploy/README.md`, and a pull
request in conflict runs none. That red is not read yet.

The runbook's test (A9) was committed alone (9af948bd), with no runbook in the tree. At that commit
it reads `examined 10 cutover state(s)` from the schematic and fails by assertion. The schematic's
order was then corrected alone (4e1e8959, the unit started before the staging rehearsal), with the
test unchanged and still red for the same reason, and the runbook followed (39e64303).

```red-first
A9: red at 9af948bd: AssertionError: False is not true : no runbook at docs/runbooks/sync-server-cutover.md
A9: green at 39e64303
```

At 39e64303 the test reads `examined 10 cutover state(s)` and `examined 10 runbook step(s)`, OK.
Six plants of the runbook each turn it red: two steps swapped, a host step marked as the owner's own
act, the hold's section renamed, a step's heading dropped, an address and a date.

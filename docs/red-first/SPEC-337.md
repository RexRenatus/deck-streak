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

# Red-first record: SPEC-195

The SPEC and ADR-195 were committed alone (506d60af). Then came the check (0cf79dda) against the
unchanged diagrams, so every test ran and the one red failed by assertion. The fix of the eight
blocks (2a3615ba) turned it green. The replay ran the check on 0cf79dda's tree: four tests, one red
by assertion.

```red-first
A1: red at 0cf79dda: ['schematics/alert-and-slo-path.md block 1', 'schematics/alert-and-slo-path.md block 2', 'schematics/data-rights-export-and-erase.md block 3', 'schematics/mutation-testing.md block 1', 'schematics/owner-session.md block 1', 'schematics/owner-session.md block 3', 'schematics/service-lifecycle.md block 3', 'schematics/sync-cycle-and-change-gate.md block 5'] != []
A1: green at 2a3615ba
A2: not red: the reader finds the blocks on the unfixed tree too, since a block that does not parse is still a block; the criterion pins that it keeps finding them
A3: not red: the parser refuses a reserved-word id on any tree; the criterion pins that the check itself can refuse
```

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

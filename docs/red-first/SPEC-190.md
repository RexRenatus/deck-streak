# Red-first record: SPEC-190

The SPEC, in `docs/specs/planned/`, and ADR-190 were committed alone (737d547). Then came the test
of A1 to A4 (5a85979) against the unchanged workflows, so every test ran and each red failed by
assertion. The workflow change (6b5c663) turned them green. The replay ran the whole test file on
5a85979's tree: four tests, two red by assertion.

```red-first
A1: red at 5a85979: ['changelog.yml: 0 concurrency blocks, so a job's own or none', 'changelog.yml: carries no concurrency block', "engine-measure.yml: group is 'engine-measure-${{ github.ref }}'", "engine-measure.yml: cancel-in-progress is 'false', so a superseded pull-request run finishes"] != []
A1: green at 6b5c663
A2: red at 5a85979: 'changelog.yml: two runs of a push to dev share a group' and 'engine-measure.yml: two runs of a push to dev share a group' in a list of 10 that should be empty
A2: green at 6b5c663
A3: not red: release.yml was already tag-only and never cancelled; the criterion pins that it stays so
A4: not red: ci.yml's and mutation-weekly.yml's blocks were already the rule's; the criterion pins that they stay so
```

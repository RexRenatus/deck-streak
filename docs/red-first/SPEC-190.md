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
A3: not red: release.yml was already tag-only and never cancelled a run in progress; the criterion pins that it stays so
A4: not red: ci.yml's and mutation-weekly.yml's blocks were already the rule's; the criterion pins that they stay so
A5: not red: the five workflows' names were already distinct; the criterion holds a new workflow to it
A6: not red: no workflow cancelled a run that is not a pull request's, and no job set a block; the criterion holds a new workflow to it
```

## Addendum: A7 (ADR-292, #377)

The test of A7 (b51b933) ran against the unchanged `release.yml`: the criterion's test failed by
assertion on the workflow's missing queue, and no other test failed. The workflow change (6696ef6)
turned it green.
Measured again on 2026-09-30 at b51b933 with the final test body, two tests fail: A7's and `test_a_shape_that_replaces_drops_or_runs_two_at_once_is_refused`, through its `planted(queue, queue)` precondition, so "no other test failed" holds for the first body only.

```red-first
A7: red at b51b933: ['release.yml: queue is None, so a third run of a tag replaces the waiting second'] != []
A7: green at 6696ef6
```


## Addendum: A8, the class as GitHub reads it (#456, verification round 2)

The test of A8 (`the_release_class_is_read_as_github_reads_it`, ff6c290) ran with its tables and its planting helper against the class helpers and reader as they
stood: it failed by assertion, naming 33 planted shapes the class did not refuse, the first a create
named alone, and no other test in the module failed. Reading `create`, each workflow's own name and
events, the block's keys with their case and a key held twice (d09c26b) turned it green.

```red-first
A8: red at ff6c290: ["create as a name: ['queue is None, so a third run of a tag replaces the waiting second']", ...] (33 shapes) != []
A8: green at d09c26b
```

## Addendum: A9, the class closed by construction (#456, verification round 3)

The test of A9 (`the_release_class_is_closed_by_construction`, 4835abf) ran with its constants and
helpers and with `closed_by_construction` returning no problem: it failed by assertion, naming 1313
planted members the class did not refuse, the first a group reading `github.action`. It was the only
test of `test_workflow_concurrency.py` that failed (14 ran, one failed). The rule (445354c) turned it
green: 14 ran and all passed.

The advisory of the A7 addendum above, "the final test body", meant the body of
`test_workflow_concurrency.py` (with `test_ci_workflows.py`) at d09c26b, run on b51b933's tree: two
tests of that one file failed, A7's and `test_a_shape_that_replaces_drops_or_runs_two_at_once_is_refused`
(13 ran), and no test of another file was run.

```red-first
A9: red at 4835abf: ['a group reading github.action', ...] (1313 members) != []
A9: green at 445354c
```

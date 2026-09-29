# Red-first record: SPEC-290

Recorded 2026-09-29. The SPEC, ADR-290, the schematic's section 6 and SPEC-039's insert-only
amendment were committed alone (d80aa22). Then came the seven tests of A1 to A7 and the edited
job-condition guard in `test_mutation_workflows.py` (cd7ce2e), with the reduced recordings under
`scripts/tests/fixtures/not-started-legs/` and an inert stub: `mutation-verdict.py` accepted the
`legs` verb and its two options and returned 0, so every test ran and each red failed by assertion,
never by a usage error. The implementation in `ci.yml` and `mutation-verdict.py` (801d791) and the
commit that kept two SPEC-039 partition rows' anchors whole (ca0c031) turned them green. The replay
ran the new module on a clean export of cd7ce2e's tree: seven tests, seven red by assertion, and the
guard's module: fourteen tests, one red by assertion. The same export of ca0c031 passes both modules. The recorded shard report lies under a
`mutants.out/` path that `.gitignore` names, so it is tracked by force, and the clean exports are
what show it is in each commit's tree.

```red-first
A1: red at cd7ce2e: AssertionError: None != '0' : no Rust change, a listing beside it
A1: green at ca0c031
A2: red at cd7ce2e: AssertionError: 0 != 1 : mutation-rust has no job-level condition
A2: green at ca0c031
A3: red at cd7ce2e: AssertionError: 3 != 0 : mutation: rust: crates/agent/src/gate.rs: 18 changed code line(s)
A3: green at ca0c031
A4: red at cd7ce2e: AssertionError: Regex didn't match: '(?m)^examined 2$' not found in ''
A4: green at ca0c031
A5: red at cd7ce2e: AssertionError: 0 != 3 : mutation: rust: crates/daemon/src/role_job.rs: 3 changed code line(s)
A5: green at ca0c031
A6: red at cd7ce2e: AssertionError: 1 != 0 : mutation-rust not started:
A6: green at ca0c031
A7: red at cd7ce2e: AssertionError: 0 != 3 : legs refuses, the judges passed:
A7: green at ca0c031
```

The job-condition guard in `test_mutation_workflows.py`, the test
`test_the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache`, now pins each
mutation job's condition, and it was red at cd7ce2e by assertion too:

```text
AssertionError: Lists differ: [] != ["${{ needs.mutation-plan.outputs.listed != '0' }}"]
```

## Disclosure: A4's body changed after its red commit

The commit b272377 edits a test file after the red commit: A4's test gains a plan that `shards` has
not yet written its listing into, beside a skipped `mutation-rust`, which `legs` must refuse as
`VOID mutation-rust: not started while the plan names no shards`. The A4 fence line above quotes
the earlier body's failure. The new body, run on cd7ce2e's tree with its stub, fails first at the
same assertion, so its failure there is, verbatim:

```text
A4: red at cd7ce2e: AssertionError: Regex didn't match: '(?m)^examined 2$' not found in ''
```

The added assertion is masked in that run by the earlier one in the same test. Against the stub it
would read an exit of 0 where 3 is owed. Row S29015 removes the refusal it asserts from the head's
`legs`, and A4 kills that mutant.

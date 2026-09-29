# Red-first record: SPEC-126

The SPEC, in `docs/specs/planned/`, and ADR-126 were committed alone (773cca1). Then came the test
of A1 to A4 (781f4a4) against the unchanged `ci.yml`: a pure-Python emulation of the pinned
download action's layout rule over the verdict's steps, so every test ran and each red failed by
assertion. The workflow change (2795033) turned them green. The replay ran the whole test file on
781f4a4's tree: four tests, three red by assertion.

```red-first
A1: red at 781f4a4: 'T/reports/mutation-plan/plan.json' not found in {'T/reports/git.diff', 'T/reports/whole.json', 'T/reports/plan.json'} : 0 shard(s), rows False, web False
A1: green at 2795033
A2: red at 781f4a4: True is not false : a verdict download reaches mutation-web: mutation-*
A2: green at 2795033
A3: red at 781f4a4: 0 != 1 : the rows' report is not downloaded by name
A3: green at 2795033
A4: not red: the judge's command lines and its reports directory were already as the criterion says; it pins that the downloads keep them
```

## Addendum, 2026-09-29 (issue #358): A4 asserts per judge line

The original lines above stand. The head is correct, so the per-line assertion cannot be red on it;
it is proved by plant, in a scratch copy of `ci.yml` that the test module's `CI` constant was
pointed at (the delivery tree's file was never edited). The old test is the module as it stood
before this change.

Plant a: `--plan` removed from the `rust` judge line, the `oracle` line still carrying it.

```text
old test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
new test: AssertionError: None != '$reports/mutation-plan/plan.json' : the rust judge line does not read --plan at $reports/mutation-plan/plan.json
```

Plant b: `--rows` removed from the `oracle` judge line, the `rust` line still carrying it.

```text
old test: test_the_judge_reads_the_paths_the_downloads_lay_down ... ok
new test: AssertionError: None != '$reports/mutation-rows/rows.json' : the oracle judge line does not read --rows at $reports/mutation-rows/rows.json
```

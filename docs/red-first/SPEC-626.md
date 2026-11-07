# Red-first record: SPEC-626

The criterion is decided by `scripts/tests/test_ruff_target.py`, which the `hygiene` job of CI
runs with every module of `scripts/tests`. The first commit carries SPEC-626, ADR-648 and the
changelog fragment. The second (951caccd) adds the test and leaves `ruff.toml` as dev holds it, so the
red is read against the old target, by an assertion and not by an error. The third (44e4f759) changes
the target line and adds the mutation row, and no test file changes in it. The criterion was run
alone by its unittest name filter, at the commit its line names.

```red-first
A1: red at 951caccd: AssertionError: 'py311' != 'py312'
A1: green at 44e4f759
```

The green line names 44e4f759, the last commit that changes `scripts/` or `ruff.toml`. The commits
after it change only documents, and the pull request's run at this delivery's head reads the
criterion green.

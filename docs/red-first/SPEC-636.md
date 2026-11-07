# Red-first record: SPEC-636

The SPEC, ADR-660, the schematic line and the fragment came first (df60bcfd), with the base code as
it was: `classify` returned `other` for every path under `web/app/scripts`, `STRYKER_MUTATE` and
StrykerJS's `mutate` held the seven `src/` globs of SPEC-039 R2, and the exclusions walk read
`web/app/src` alone.

The commit that adds this record adds the tests and nothing else: one new class in
`scripts/tests/test_mutation_verdict.py`, `TheTokenCompilerJoinsTheWebClass`, holding A1 and A2,
and the module docstring's naming of them. Each red below is quoted from one local run of that test
file over the tree of the commit that adds this record.

```red-first
```

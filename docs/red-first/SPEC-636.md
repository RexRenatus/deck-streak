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
A1: red at c84e3293: AssertionError: 'other' != 'web' : web/app/scripts/build-tokens.ts (local run)
A1: green at ee93deee
A2: red at c84e3293: AssertionError: Lists differ: ["configs: web/app/stryker.config.json: mutate is not R2's web production code"] != [] (the widened globs pass the configuration check), and the walk's Lists differ: [] != ['exclusions: web/app/scripts/build-tokens[79 chars]11)'] (local run)
A2: green at ee93deee
```

A1 and A2 were committed with the base code as it was (c84e3293); each read red by name in a
local run of `test_mutation_verdict.py` (`Ran 46 tests`, `FAILED (failures=4)`: A1 once and A2
in its three subtests, every other test OK). The class change replaced the base code in
ee93deee, which also gave `PASSING_MUTATE` in `scripts/tests/test_mutation_workflows.py` the
token compiler's entry, in step with `STRYKER_MUTATE`, so that module's fixture trees keep
passing the configuration check; it removes no assertion and changes no line of A1 or A2. The
local runs at ee93deee read them green (`test_mutation_verdict.py`: `Ran 46 tests`, `OK`;
`test_mutation_workflows.py`: `Ran 57 tests`, `OK`). The commits after ee93deee change only
documents.

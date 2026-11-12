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
`test_mutation_workflows.py`: `Ran 57 tests`, `OK`). The commit after ee93deee, 10e6d727,
changes only documents.

After the greens, CI run 42397705637 (job 128478550062) read SPEC-618 A24's bench-exclusion test
in `scripts/tests/test_perf_benches.py` red at 10e6d727: it held every positive StrykerJS
`mutate` glob to the `src/` root, and the token compiler's entry lies outside it. The commit
after 10e6d727, which adds this paragraph, changes that test file: the test admits R1's one
exact path beside the root by a named constant and an exact-list assertion, and checks that the
path is no bench file. It removes no assertion, keeps the check that no bench file lies under
`web/app/src` and the coverage exclusions, and changes no line of A1 or A2. A local run of
`test_perf_benches.py` read that test red at 10e6d727 and every test OK over the tree of the
commit after it.

```red-first
A3: not red: tests only; each new test passes at the base, and the ranged StrykerJS run before and after is the proof
```

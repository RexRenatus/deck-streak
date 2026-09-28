# Red-first record: SPEC-056

The SPEC and ADR-069 were committed alone (ebf682b), then the tests of A1 to A16 (e81198a). The
vault's parser tests (A6 to A8) compiled against the base's constants and failed when they read the
owned data, which did not exist yet; every other criterion ran against the base's own code. The
owned data and the new driver (aca92de) turned A6 to A15 green, and the removal (da9d6ce) turned A1
to A5 and A16 green. R14 and the apiKeyHelper scan came later in the delivery, each SPEC amendment
committed before its test (b94541e): A17 and A18 were committed red together (9ad1fe5). A17 went
green with the scan (fc54ad4); a case the removed gate step refused, a settings file at a path it
scanned that the scan finds none of, was then added red (991946f) and turned green (eaa3715), so
A17's record is that pair. A18 went green with the retirements (2d42d0d). The reds at e81198a were
re-run from a `git archive` export of that sha; the later reds were observed on the tree each red
commit holds. Where a failure named the maintainer's private tooling or a scratch path, its line
paraphrases it.

```red-first
A1: red at e81198a: AssertionError: Lists differ: ['.packs/VENDORED.json', '.packs/scripts/a[10333 chars]son'] != [] (183 vendored files were tracked)
A1: green at da9d6ce
A2: red at e81198a: AssertionError: 'vendored_from' unexpectedly found in methodology.json's keys
A2: green at da9d6ce
A3: red at e81198a: AssertionError: 'packs' unexpectedly found in ['fmt', 'clippy', 'test', 'doctest', 'audit-rust', 'test-engine', 'web', 'audit-web', 'packs', 'python', 'scrub', 'secrets']
A3: green at da9d6ce
A4: red at e81198a: AssertionError: Lists differ: [('crates/vault/src/rails.rs', 21, 'pub co[12641 chars]he')] != [] (the adapter compiled in the vendored rails)
A4: green at da9d6ce
A5: red at e81198a: AssertionError: the named log directory's path was found in stdout, on the `logs: <dir>` line
A5: green at da9d6ce
A6: red at e81198a: panicked at crates/vault/tests/owned_data.rs:23: the owned rails.json cannot be read (no crates/vault/data/ yet)
A6: green at aca92de
A7: red at e81198a: panicked at crates/vault/tests/owned_data.rs:23: the owned layout.json cannot be read (no crates/vault/data/ yet)
A7: green at aca92de
A8: red at e81198a: panicked at crates/vault/src/staged.rs:1080: the owned gate classes can be read (no crates/vault/data/ yet)
A8: green at aca92de
A9: red at e81198a: AssertionError: Lists differ: [] != ['persona-core.json', 'privacy-gdpr.json'] (the scrub had no rules of its own)
A9: green at aca92de
A10: red at e81198a: AssertionError: False is not true : the scrub reads its lists with its own loader
A10: green at aca92de
A11: red at e81198a: AssertionError: 0 != 1 : the old driver refused VOID asking for its old checkout variable, and never for PACKS_WIRING (in 4 of the subtests)
A11: green at aca92de
A12: red at e81198a: AssertionError: 2 != 0 : the old driver refused VOID asking for its old checkout variable
A12: green at aca92de
A13: red at e81198a: AssertionError: 2 != 0 : the old driver refused VOID asking for its old checkout variable
A13: green at aca92de
A14: red at e81198a: AssertionError: 2 != 0 : the old driver refused VOID asking for its old checkout variable
A14: green at aca92de
A15: red at e81198a: AssertionError: 2 != 0 : the old driver refused VOID asking for its old checkout variable
A15: green at aca92de
A16: red at e81198a: AssertionError: 0 != 1 : one line names the box run's status
A16: green at da9d6ce
A17: red at 991946f: AssertionError: False is not true : pending  no-apikeyhelper      scan   examined 0: pending #29: 0 settings file(s) (a settings file at a path the removed gate step scanned read as pending)
A17: green at eaa3715
A18: red at 9ad1fe5: AssertionError: examined 0 retired criteria: the population is empty, so nothing was judged
A18: green at 2d42d0d
```

At 9ad1fe5, A17 was red for its first reason too: the driver refused VOID with `box has unknown
key(s) ['no-apikeyhelper']`, since it ran no apiKeyHelper scan. A18's planted SPEC and record were
refused by the checker at that sha exactly as the test states, so its red is the population alone.

The three mutation rows of the band S05600 to S05699 were proved at a9ff442: each killer passed
without its mutant and failed with it (killed 3, survived 0, void 0).

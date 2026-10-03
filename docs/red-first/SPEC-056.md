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
A19: red at cb384f3: AssertionError: 2 != 0 : box-packs: VOID: packs.gamma takes only ['deferred_rows', 'enforced_by', 'excluded_rows', 'note', 'state'] (the driver knew no advisory lint)
A19: green at 0b6c110
A20: red at cb384f3: AssertionError: {} != {('planted.timer', 'calendar-not-persistent'): 'a planted why of six words'} (the unit reader's stub read no waiver)
A20: green at 5ff9c08
```

At 9ad1fe5, A17 was red for its first reason too: the driver refused VOID with `box has unknown
key(s) ['no-apikeyhelper']`, since it ran no apiKeyHelper scan. A18's planted SPEC and record were
refused by the checker at that sha exactly as the test states, so its red is the population alone.

R15 and R16 came in a fix round: the SPEC's amendment (66f2f89) came before A19's and A20's tests,
committed red together (cb384f3); A20's red rests on a stub of the unit reader's `waivers()` that
read nothing, and A19's on the driver refusing the unknown key.

The three mutation rows of the band S05600 to S05699 were proved at a9ff442: each killer passed
without its mutant and failed with it (killed 3, survived 0, void 0).

## Addendum, 2026-10-02: the owned email rule (SPEC-056 section 9)

```red-first
A21: red at 7e5c3e7: AssertionError: Lists differ: ['note.md:13: email', 'note.md:14: email', ... ] != [] : each line is a unit path, so none is an address
A21: green at 26cfdb5
A22: not red: pins that a real address is found in each of eight contexts; the old rule already found it, so the test guards the refreshed rule and proves no new behaviour
A23: not red: pins that an address at a reserved domain passes; the old rule already passed it, so the test guards the refreshed rule and proves no new behaviour
```

At 7e5c3e7 the unit path population read `examined 300 lines, mismatches 216`: the old rule flags
a unit path of a type the scrub's own code does not name and a path whose directory suffix is
`.wants`, `.requires` or `.upholds`. A22 read `examined 24 lines, mismatches 0` and A23 passed on
18 lines at both commits. No mutation row is added: the change is data.

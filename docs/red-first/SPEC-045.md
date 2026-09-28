# Red-first record: SPEC-045

The order of work was: the SPEC promoted, ADR-045 accepted and the resolution drawn (b0d8ecc); A12's
census committed alone (a41b7f1); the goldens and the example taxonomy (5e3da60); the acceptance
tests beside stubs (277391c); the implementation (2c414de); the hand-proved rows (fc0a5eb); tests
for the diff's mutants (1d6a90e); and `dev` absorbed by a merge commit that resolved conflicts only
(a0e64ee).

A12's census was committed alone, before any code. It read the tree with the public scrub and
found no example taxonomy: its positive artifact, `deploy/config/readings-taxonomy.example.json`,
did not exist yet. The goldens came next: `tools/parity-oracle/registry/spec_045.py` registered
`resolve_day_sets`, `law_subject` and `digest_for_card_ids`, and the generator wrote them from the
predecessor's own functions at `27ee2bc`, the example taxonomy's synthetic names in place of every
private deck constant the functions read. The registry module's sha256 read the same before and
after the run, the predecessor's checkout was left as it was with no bytecode written into it, and
no other golden changed. The example taxonomy was committed with the goldens, which turned A12
green.

The other tests were committed (277391c) beside a readings crate whose public API was in place and
whose behaviour was stubbed: the taxonomy parsed to no root, no language and no writing root; no
deck had a law subject or a topic; the digest was empty; no answer was saturated; the resolver and
the universe were empty; the resolution resolved nothing and applied no budget; the last sync always
succeeded, the owner always studied and the pause window was today twice; every reason's class was
`rail_broken`, and every state counted as a failure and a refusal; the store wrote and read nothing;
and the data-rights port declared no table. The migration, coordination's use case and registry,
the symmetry probe's seeds, the register of DeckStreak's own tables, `privacy.json` and `PRIVACY.md`
were committed with the tests. Each criterion was run there with the SPEC's own fenced command,
selecting one test, and failed by assertion for its own criterion, not by a compile error, a missing
fixture or an empty selection.

The implementation followed (2c414de). Between the red commit and the green one no test changed
what it asserts: four test files gained the workspace's lint allowance for a test helper's
`expect`, one binding was renamed, a closure became a function path, and `RESOLVE_BUDGET - 1 ms`
became `RESOLVE_BUDGET.saturating_sub(1 ms)`, the same instant. After green, the twelve hand-proved
rows (fc0a5eb) were proved on the committed tree: every row KILLED, each target restored byte for
byte. Then every mutant cargo-mutants lists over the diff was read against the tests, and the ones
no test could tell apart were given one (1d6a90e): the taxonomy's refusals, accessors and debug
form, its path setting, the review read's floor, and the queue's and the read's failures. Three
impls no caller used were removed rather than tested.

```red-first
A1: red at 277391c: assertion `left == right` failed; left: [], right: ["language/qaa", "law/evidence", "law/torts"]
A1: green at 2c414de
A2: red at 277391c: assertion `left == right` failed: the resolution of the every-track case; left: no active topic and no unmapped deck, right: ten topics and four unmapped decks
A2: green at 2c414de
A3: red at 277391c: assertion `left == right` failed: the subject of "Casebook"; left: Null, right: String("Casebook")
A3: green at 2c414de
A4: red at 277391c: assertion `left == right` failed: the digest of []; left: String(""), right: String("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
A4: green at 2c414de
A5: red at 277391c: assertion `left == right` failed; left: {}, right: {11, 12}
A5: green at 2c414de
A6: red at 277391c: assertion failed: !last_sync.succeeded(), for the last run Some(Error)
A6: green at 2c414de
A7: red at 277391c: assertion `left == right` failed; left: [20000, 20000], right: [19999, 19998]
A7: green at 2c414de
A8: red at 277391c: assertion `left == right` failed: every state reads back as itself; left: [], right: the 28 topic days written
A8: green at 2c414de
A9: red at 277391c: assertion failed: saturated(1000, None)
A9: green at 2c414de
A10: red at 277391c: assertion `left == right` failed: the budget is waited out, no longer; left: 0ns, right: 30s
A10: green at 2c414de
A11: red at 277391c: assertion `left == right` failed; left: {}, right: {"language/qaa", "law/evidence", "law/torts"}
A11: green at 2c414de
A12: red at a41b7f1: AssertionError: False is not true : deploy/config/readings-taxonomy.example.json is missing: R1's synthetic taxonomy shows the file's shape
A12: green at 5e3da60
A13: red at 277391c: assertion `left == right` failed: the readings' record is the owner's data: exported and erased (CHARTER 13); left: [], right: [("reading_topic_days", ExportAndErase), ("reading_runs", ExportAndErase)]
A13: green at 2c414de
A14: red at 277391c: assertion failed: an absent route is a setting, not a failure
A14: green at 2c414de
```

In A1 and A5 the stub resolved no topic of the engine's queue; in A2 and A11 no deck mapped to a
topic; in A3 no deck had a law subject; in A4 the digest was empty; in A6 a failed last sync read as
succeeded; in A7 the pause window was today twice; in A8 the store read back nothing; in A9 a root of
1000 cards with no count read as complete; in A10 the resolution returned at once, with no budget;
in A12 the example taxonomy was missing; in A13 the port declared no table; and in A14 an absent AI
route counted as a failure.

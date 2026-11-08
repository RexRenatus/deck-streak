# Red-first record: SPEC-648

SPEC-648, ADR-671, the schematic and the changelog fragment were committed first and alone, in
d2cc6fe88c5859cd804c7adbfca31a322ac6d08c. The commit that adds this record carries the three criteria's tests with the read as
it was: A1 and A2 are red there by name, each by its assertion, and A3 is not red. The
acceptance fence, one test per criterion:

```text
A1: cargo test -p deck-streak-ingest --lib -- --exact reader::tests::a_read_whose_caller_is_dropped_keeps_the_collection_lock_until_it_ends
A2: cargo test -p deck-streak-ingest --lib -- --exact reader::tests::a_read_its_runtime_drops_keeps_the_collection_lock_until_the_process_ends
A3: cargo test -p deck-streak-ingest --lib -- --exact reader::tests::a_read_that_ends_releases_the_collection_lock_and_returns_its_value
```

```red-first
A1: red at 56a19c3632d5e21be8b431bba7553da11b8e6098: assertion `left == right` failed: (the read started, its caller dropped, the lock held while the read ran, the read ended, the lock free after it)
A1: green at 992032be2cbcc3e0df48c46ad3994fef0aa3906b
A2: red at 56a19c3632d5e21be8b431bba7553da11b8e6098: assertion `left == right` failed: (the read started, the lock held after the runtime stopped while the read ran, the read ended, the lock still held after the read ended)
A2: green at 992032be2cbcc3e0df48c46ad3994fef0aa3906b
A3: not red: a read that ends released the lock and returned its value before the cure too; it guards that the cure keeps both
```

## What changed between the red and its green

The green commit, 992032be2cbcc3e0df48c46ad3994fef0aa3906b, changed no test line: it changed `with_copy` in
`crates/ingest/src/reader.rs` and the cover in the model, and the test module it ran is the one
the red commit, 56a19c3632d5e21be8b431bba7553da11b8e6098, added. The rows commit, c3bbb16c8fa30ca4a002cb3c2071ba83675a2a0c, added
`scripts/mutation-rows.d/S64800-S64899.json` only.

## Rows

S64801 was proved KILLED at c3bbb16c8fa30ca4a002cb3c2071ba83675a2a0c: the rows prover installed its mutant, and its killer, the
test of the first criterion, passed without the mutant and failed with it. The proof read
`rows: examined 1: killed 1, survived 0, void 0`.

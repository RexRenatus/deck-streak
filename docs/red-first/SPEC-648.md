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
A3: not red: a read that ends released the lock and returned its value before the cure too; it guards that the cure keeps both
```

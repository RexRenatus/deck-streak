# Red-first record: SPEC-419

Every criterion's test is committed before the code it judges, beside a stub of
`scripts/web_mutation_shards.py` that parses, deals, counts and reads nothing, so each test fails
by its own assertion rather than by an import error. A1 to A5 read `ci.yml` and are red at the
base, which has no shard job; A6 to A13 run the stub. All thirteen run in CI's `hygiene` job,
which runs `scripts/tests`, and their reds are read there, by name, at the commit that adds them.
Each red and green line below is recorded with the fix.

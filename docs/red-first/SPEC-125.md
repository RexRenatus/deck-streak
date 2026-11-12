# Red-first record: SPEC-125

The test (7897052) was committed alone, before the manifest changed: the whole file ran on that
tree, three tests, three red by assertion. The profile sections, the SPEC and ADR-125 (79e2607)
turned them green. A2 also rests on the jobs that build Rust on this diff (`rust`, `engine (1)`
and `engine (2)`) passing on the new profile; the pull request body names that run.
`mutation-rust` builds nothing on a diff that changes no Rust production file.

```red-first
A1: red at 7897052: the dev profile held no debug value: None != 'line-tables-only'; the dependency override held none: None is not False
A1: green at 79e2607
A2: red at 7897052: the manifest declared no profile: 'dev' not found in {}
A2: green at 79e2607
```

## Fix-round addendum

The override scan in the third test missed the equals form (`--config=profile.`). A fourth
test plants the scan's own cases, that spelling included, and was committed alone: red by
assertion on the earlier regex (`the scan misses: cargo build --config=profile.dev.debug=2`).
Widening the regex to `--config[=\s]+` turned it green. A2's original red and green lines
above are unchanged.

The pair, in prose so that A2 keeps its one red and one green line above: red at 2c05bc8, where the
scan missed the equals form (`unexpectedly None : the scan misses: cargo build
--config=profile.dev.debug=2`); green at 600711e.

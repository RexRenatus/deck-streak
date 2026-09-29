# Red-first record: SPEC-125

The test (7897052) was committed alone, before the manifest changed: the whole file ran on that
tree, three tests, three red by assertion. The profile sections, the SPEC and ADR-125 (79e2607)
turned them green. A2 also rests on CI's Rust jobs (`rust`, `engine`, `mutation-rust`) passing
on the new profile; that run is named in the pull request.

```red-first
A1: red at 7897052: the dev profile held no debug value: None != 'line-tables-only'; the dependency override held none: None is not False
A1: green at 79e2607
A2: red at 7897052: the manifest declared no profile: 'dev' not found in {}
A2: green at 79e2607
```

# Red-first record: SPEC-416

The SPEC, ADR-430 and the test module were committed first, with `.github/workflows/ci.yml`
untouched. At that commit the one enumerating test fails by assertion on `mutation-rust`, which
holds no fetch step; the planted-shape controls and SPEC-332's own tests pass.

```red-first
A1: red at 7f3126de: AssertionError: Lists differ: [] != ['ci.yml:mutation-rust: no step runs `cargo fetch --locked`']
A1: green at ea9f4217
A2: not red: it plants its own shapes into copies of a job, so it pins the judge and not the tree
A3: not red: SPEC-332's tests pin the behaviour the base already had, and guard the refactor of the judge's anchor
```

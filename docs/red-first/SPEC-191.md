# Red-first record: SPEC-191

The SPEC and ADR-191 were committed alone (444c9af). Then came the nine tests of A1 to A9 (7725611)
against a tree with no `rust-cache.yml` and the unchanged save rule, so every test ran and each red
failed by assertion. The workflow and the scheduled-save shape in `test_ci_workflows.py` (a43b940)
turned them green. The replay ran the new module on 7725611's tree: nine tests, nine red by assertion.

```red-first
A1: red at 7725611: .github/workflows/rust-cache.yml does not exist
A1: green at a43b940
A2: red at 7725611: .github/workflows/rust-cache.yml does not exist
A2: green at a43b940
A3: red at 7725611: .github/workflows/rust-cache.yml does not exist
A3: green at a43b940
A4: red at 7725611: .github/workflows/rust-cache.yml does not exist
A4: green at a43b940
A5: red at 7725611: .github/workflows/rust-cache.yml does not exist
A5: green at a43b940
A6: red at 7725611: .github/workflows/rust-cache.yml does not exist
A6: green at a43b940
A7: red at 7725611: Tuples differ: (['planted.yml:warm:save: saves on a pull r[324 chars]], 1) != ([], 1) : the planted good shape is refused
A7: green at a43b940
A8: red at 7725611: .github/workflows/rust-cache.yml does not exist
A8: green at a43b940
A9: red at 7725611: .github/workflows/rust-cache.yml does not exist
A9: green at a43b940
```

## Addendum, 2026-09-29: fix round 1 (the guard's admission is scoped to `rust-cache.yml`)

The verifier planted the good scheduled-save shape under another file name and the save rule
admitted it. A7's test body changed: its five planted calls now judge `rust-cache.yml`, and it gains
two assertions. The fence line above quotes the earlier body's failure; the new body's failure at
the tests-only commit 27a017b (with the unchanged guard) is:

```text
A7: red at 27a017b: AssertionError: [] is not true : another workflow's scheduled save is admitted
A7: green at 520540c
```

At 27a017b the whole module ran 9 tests, 8 passing and this one failing. The second new assertion
(the good shape whose save `if:` also holds on a dispatch, judged as `rust-cache.yml`) is red at
27a017b too: the guard returned `[]` for it, that is, admitted a save on a hit, and the message it
would print is "a scheduled save that also saves on a hit is admitted". It is masked in the run by
the first assertion, which fails earlier in the same test. 520540c scopes the admission by file name
and adds the dispatch-on-a-hit scenario; the whole of `test_ci_workflows` and
`test_rust_cache_workflow` (37 tests) pass. Row S19105 kills its mutant with A7 alone.

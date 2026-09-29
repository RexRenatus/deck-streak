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

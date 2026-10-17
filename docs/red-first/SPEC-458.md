# Red-first record: SPEC-458

The SPEC, ADR-475, the schematic and the two amendments were committed first, and the red commit holds tests, one register entry
and this record, with every workflow untouched. At that commit the two enumerating tests and the colour test fail by assertion on
the workflows as they stand: the weekly `rust` leg holds no fetch, the rehearsal fetches after its baseline, the warm-up saves with
no fetch, and `mutation-rust` ends on no colour line. Each module reads a workflow file, so each red is read on CI by test name and
recorded when seen.

```red-first
A3: not red: it plants its own shapes into copies of a saving job, so it pins the judge and not the tree
A5: not red: it plants its own shapes into copies of the Rust leg's step, so it pins the reader and not the tree
A6: not red: SPEC-332's and SPEC-416's tests and test_rust_cache_workflow.py pin what the base already did, and guard the widened judge
```

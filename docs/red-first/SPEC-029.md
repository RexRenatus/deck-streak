# Red-first record: SPEC-029

The tests were committed (e1430ce) before the code they judge, beside stubs that gave that code
its surface and no behaviour: `generate.py` took `--package` and `--registry` and ignored them,
`test_goldens.py`'s checks found nothing, and `golden.rs` accepted every golden. Each criterion
was run there and failed by assertion, not by a compile error, a missing fixture or an empty
selection. The implementation followed (f2b9c19), and no test method changed between the two
commits: only the stubbed checks, the generator and the reader did.

```red-first
A1: red at e1430ce: AssertionError: False is not true : the generator wrote no golden review_xp.json
A1: green at f2b9c19
A2: red at e1430ce: AssertionError: 0 != 2
A2: green at f2b9c19
A3: red at e1430ce: AssertionError: False is not true : the generator wrote no golden local_day.json
A3: green at f2b9c19
A4: red at e1430ce: AssertionError: False is not true : the generator wrote no golden stand.constants.json
A4: green at f2b9c19
A5: red at e1430ce: AssertionError: False is not true : the generator wrote no golden epoch_plus.json
A5: green at f2b9c19
A6: red at e1430ce: AssertionError: Lists differ: [] != ['one.json: generator_sha256 differs from [57 chars] it']
A6: green at f2b9c19
A7: red at e1430ce: AssertionError: Lists differ: [] != ['one.json: registry_sha256 differs from t[65 chars] it']
A7: green at f2b9c19
A8: red at e1430ce: AssertionError: Lists differ: [] != ["planted.json: inputs is 'production', no[61 chars] 20"]
A8: green at f2b9c19
A9: red at e1430ce: AssertionError: Lists differ: [] != ['planted.py:2: calls open'] (and the same for sqlite3, socket, urllib, http and a Path read)
A9: green at f2b9c19
A10: red at e1430ce: AssertionError: Lists differ: [] != ["planted.json: generator holds 'written 0[117 chars]02'"]
A10: green at f2b9c19
A11: red at e1430ce: AssertionError: False is not true : the study-day golden is not committed
A11: green at f2b9c19
A12: red at e1430ce: assertion `left == right` failed: left: 0, right: 50
A12: green at f2b9c19
A13: red at e1430ce: a golden with no case was not refused as one: Ok(Golden { schema: "", kind: "", function: "", ..., cases: [] })
A13: green at f2b9c19
A14: red at e1430ce: a golden of another schema was not refused as one: Ok(Golden { schema: "", kind: "", function: "", ..., cases: [] })
A14: green at f2b9c19
```

In A1 and A3 to A5 the stub generator loaded no registry, so it wrote no golden, and each test's
first assertion names the golden its criterion needed. In A2 two modules registering one golden
name exited 0. In A6 to A10 a planted golden or module carrying the defect drew no finding. In
A11 no study-day golden existed. In A12 the stub reader visited no case of the study-day golden,
and in A13 and A14 it accepted the planted goldens it must refuse.

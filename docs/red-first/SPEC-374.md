# Red-first record: SPEC-374

Part one of #671. The SPEC, ADR-385 and the schematic were committed first, and the criteria's tests
next, with the stubs they compile against and nothing else: `CLIENT_LEVEL`, the outcome and the
sentences in `crates/engine-core/src/handshake.rs`, a statement-URL helper that gives no URL, and
`Dispatcher::handshake` recording nothing, so no call is refused and the static library reads no
statement. Each red below is quoted from the run of its fenced command at that commit; a compile
failure is not one of them. The commit that turns them green follows, and each `green at` line
names it.

```red-first
A1: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: assertion `left == right` failed: below the minimum, each sync call is refused before the engine with the below sentence: left: [("the native sync login", Ok((NetworkError, "A network error occurred.\n\nError details: error sending request for url ()"))), ("the web sync login", Ok((NetworkError, ...))), ("the web normal sync", Ok((InvalidInput, "CollectionNotOpen")))]
A2: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: assertion `left == right` failed: an unread, an undecodable and a never-read statement each refuse every sync pair with its sentence: left: [("a dispatcher never handed a statement", "the native sync login", Ok((NetworkError, "A network error occurred.\n\nError details: error sending request for url ()"))), ...]
A3: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: assertion `left == right` failed: a below statement after an admitting one refuses the login again: left: Ok((NetworkError, "A network error occurred.\n\nError details: error sending request for url ()"))
A4: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: assertion `left == right` failed: below the minimum, the statement's GET is the only request: nothing under the sync path: left: ["POST /sync/hostKey HTTP/1.1"] right: ["GET /api/sync/minimum-client HTTP/1.1"]
A5: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: assertion `left == right` failed: the statement's GET comes first: left: Some("POST /sync/hostKey HTTP/1.1") right: Some("GET /api/sync/minimum-client HTTP/1.1")
A6: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: assertion `left == right` failed: the service answers its minimum client level, exactly: left: (404, "") right: (200, "{\"minimum_client_level\":1}")
A7: red at 8c1fe7e2d3a50033d3551fe5dc3810f4269cae52: AssertionError: Lists differ: ['MINIMUM_CLIENT_LEVEL is defined 0 time(s), not once: []'] != [] : the tree's minimum and level break R3
A1: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A2: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A3: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A4: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A5: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A6: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A7: green at 2db78c8e075b6566c2dc2ca258c53b5e1c6b2870
A16: red at f7d626917b62e15a0b4ce6adbccaf49434cf5cec: Expected: "held"; Received: "offline" at sync.spec.ts:99:47, in both browsers (CI job 113546776315)
A17: red at f7d626917b62e15a0b4ce6adbccaf49434cf5cec: Expected: "held"; Received: "offline" at sync.spec.ts:125:47, in both browsers (CI job 113546776315)
A18: red at f7d626917b62e15a0b4ce6adbccaf49434cf5cec: Expected value: "/anki-sync-moved/sync/hostKey"; Received array: [] at sync.spec.ts:151:33, in both browsers (CI job 113546776315)
A19: red at b36789cca7b05e9e576524e2a4f5b975662c727f: AssertionError: expected undefined to deeply equal Uint8Array[ 123, 34, 109, 105, …(-83) ] at sync.test.ts
A20: red at b36789cca7b05e9e576524e2a4f5b975662c727f: AssertionError: promise resolved "'held'" instead of rejecting at sync.test.ts
A21: red at b36789cca7b05e9e576524e2a4f5b975662c727f: AssertionError: expected [ [ 'handshake', 'nothing' ], …(3) ] to deeply equal [ [ 'handshake', …(1) ], …(3) ] at worker.test.ts
A22: red at b36789cca7b05e9e576524e2a4f5b975662c727f: `fn handshake(` occurs 0 times, not once, at boundary.rs each_boundary_function_reaches_the_engine_through_the_dispatcher
A19: green at 0a9efff1167ec45c98f995f7e5dce8a44bbb683f
A20: green at 0a9efff1167ec45c98f995f7e5dce8a44bbb683f
A21: green at 0a9efff1167ec45c98f995f7e5dce8a44bbb683f
A22: green at 0a9efff1167ec45c98f995f7e5dce8a44bbb683f
A16: green at 97ebc8844d2e8e7c101a7d601aafaf079c576e54, in both browsers (CI job 113583925251)
A17: green at 97ebc8844d2e8e7c101a7d601aafaf079c576e54, in both browsers (CI job 113583925251)
A18: green at 97ebc8844d2e8e7c101a7d601aafaf079c576e54, in both browsers (CI job 113583925251)
```

A7's census reads the index, as at C1: its green was read with the green commit's set staged, before
that commit, and the commit holds exactly that set.

Part one's fix round. A16 to A18 are the web Worker's three sync specs, red at part one's tip as CI's web-engine job read them, on a merge ref whose tree is that commit's: the core refused every web sync pair because the Worker handed it no statement. A19 to A22 were committed next, red, with stubs that keep every input but the behaviour: a statement read that reads nothing, a sync that hands the engine what it read and ignores the answer, and no export. The commit that turns them green follows, and each green line names it. A16 to A18 turn green only in CI's browsers, and their green lines are written when CI has read them.

Part two of #671. The check's tests were committed first, against a stub script that exits 0 and
prints nothing, with no workflow file yet. A9 to A14 are quoted from the run of each fenced command at
that commit; A8 and A15 are quoted from CI's run of theirs.

```red-first
A8: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: False is not true : testflight-rerelease-check.yml does not exist at test_ci_workflows.py test_a8_the_rerelease_check_runs_scheduled_with_admitted_reads_only (CI job 113655743089)
A9: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: 'due: internal build 2 expires 2030-01-13T12:00:00Z, within 7 days; dispatched testflight-internal.yml on dev' not found in ''
A10: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: 'healthy: internal build 2 expires 2030-03-11T12:00:00Z, more than 7 days away' not found in ''
A11: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: 0 == 0
A12: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: 0 == 0
A13: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: 0 == 0
A14: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: 0 != 1 : one signature was asked for
A15: red at b1d5ddf61fdba34bdcd8260ca507e9b9475e45ab: AssertionError: unexpectedly None : LEAD_DAYS is not a module-level literal of testflight_age.py at test_ci_workflows.py test_a15_the_rerelease_lead_covers_the_schedule_interval (CI job 113655743089)
```
```red-first
A9: green at f45d729d58665cc9ee2140c2e63492f817c5f6aa
A10: green at f45d729d58665cc9ee2140c2e63492f817c5f6aa
A11: green at f45d729d58665cc9ee2140c2e63492f817c5f6aa
A12: green at f45d729d58665cc9ee2140c2e63492f817c5f6aa
A13: green at f45d729d58665cc9ee2140c2e63492f817c5f6aa
A14: green at f45d729d58665cc9ee2140c2e63492f817c5f6aa
```

A8 and A15 are CI-only: their green is read from CI's run of the push-2 head, at the seat's CIWATCH.

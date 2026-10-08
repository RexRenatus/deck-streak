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
A1: green at C2
A2: green at C2
A3: green at C2
A4: green at C2
A5: green at C2
A6: green at C2
A7: green at C2
```

A7's census reads the index, as at C1: its green was read with the green commit's set staged, before
that commit, and the commit holds exactly that set.

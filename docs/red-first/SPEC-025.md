# Red-first record: SPEC-025

The watchdog's golden was registered and generated first (cc29f7b), and the schematic drawn
(0b40b62). The tests were then committed (c47100b) beside an API and a daemon whose public surface
was in place and whose behaviour was stubbed: the router served no route and applied no layer,
readiness never became ready, the listen address took any socket address, `serve` dropped every
connection when its signal arrived, the lifecycle sent nothing, armed no heartbeat and heard no
signal, the database's opener called `Db::open` with no lock, and `deckstreakd` exited 0 whatever it
was given.

That first run showed the trace tests reading their child's output wrongly: the test harness prints
`test <name> ... ` on the line a child's first output lands on, so A9 failed for the parser's reason
rather than its own, and A8's child stopped at a panic the stub did not catch before it reported the
rest. 6fcf6e0 corrected the reading and let the child report an escaped panic, changing no
assertion, and every criterion was run again there with the SPEC's own fenced command: fourteen
failed by assertion for their own criterion, each selecting one test, and A4 passed, disclosed
below. The implementation followed (33a877a), where all fifteen passed. Between 6fcf6e0 and 33a877a
no test changed what it asserts: five test files took lint and layout fixes only
(`Duration::from_mins`, `saturating_sub`, a binding's name, two `#[ignore]` reasons, line
wrapping).

A15 is the criterion the delivery added (§7, R11). At 6fcf6e0 its opener had no lock, and it
failed on the migrator race itself, both roles applying migration 2001. The runs that sized its 64
rounds, on the same stub at c47100b, failed with `SQLITE_BUSY` in 29 of 30 runs of 24 rounds.

```red-first
A1: red at 6fcf6e0: assertion `left == right` failed: left: 404, right: 200 (nothing served /api/livez)
A1: green at 33a877a
A2: red at 6fcf6e0: assertion `left == right` failed: left: 404, right: 503 (nothing served /api/readyz)
A2: green at 33a877a
A3: red at 6fcf6e0: assertion `left == right` failed: /api/livez: left: 404, right: 200
A3: green at 33a877a
A4: not red: axum's extractors refuse a body over 2 MiB by default, and R5 keeps that default, now stated as BODY_LIMIT_BYTES; the stub, which applied no layer, still answered 413, so the test guards the limit against a route that would switch it off and proves no new behaviour
A5: red at 6fcf6e0: /api/test/held: the request past the bound was held, not shed
A5: green at 33a877a
A6: red at 6fcf6e0: assertion `left == right` failed: left: 200, right: 408
A6: green at 33a877a
A7: red at 6fcf6e0: the handler's panic escaped the service: Err(JoinError::Panic(Id(1), "a synthetic handler panic", ...))
A7: green at 33a877a
A8: red at 6fcf6e0: the routed response carries no request id (none)
A8: green at 33a877a
A9: red at 6fcf6e0: a sensitive header's value reached the log: ... "headers":"{\"cookie\": \"deckstreak_session=saffron-lantern-quiver\", ...
A9: green at 33a877a
A10: red at 6fcf6e0: serve returned while a request was still in flight
A10: green at 33a877a
A11: red at 6fcf6e0: assertion `left == right` failed: left: Err("the binary exited (exit status: 0) before READY=1; messages seen: []\n"), right: Ok(())
A11: green at 33a877a
A12: red at 6fcf6e0: assertion `left == right` failed: 5s: left: false, right: true (no heartbeat armed at the golden's floor)
A12: green at 33a877a
A13: red at 6fcf6e0: 192.0.2.10:8080 was not refused as a non-loopback address: Ok(ListenAddress(192.0.2.10:8080))
A13: green at 33a877a
A14: red at 6fcf6e0: assertion `left == right` failed: ["frobnicate"]: exit status: 0; left: Some(0), right: Some(2)
A14: green at 33a877a
A15: red at 6fcf6e0: round 1: role 1 failed to start: Err(Kernel(Migrate(ExecuteMigration(Database(SqliteError { code: 1, message: "table settings_generation already exists" }), 2001))))
A15: green at 33a877a
```

## Addendum, 2026-09-29: a failed lifecycle test leaves no daemon running (issue 366)

The child guard was committed as a stub whose drop did nothing (0af311f), beside the tests; the
guard followed (07d9b4e). The runner's process-group kill was committed the same way: a test
beside the unchanged runner (97b11c3), then the kill (9f49316).

```red-first
A16: red at 0af311f: the daemon was still running after its test failed
A16: green at 07d9b4e
A17: red at 97b11c3: the grandchild of a timed-out killer is still running
A17: green at 9f49316
```

Two later commits tighten the tests, not the criteria. aba7e3e stops the guard joining a waiter
whose child outlived every signal, and the killer-group test is bounded in its own thread; both
because a mutant of the guard or of the kill made its killer wait for ever instead of failing.

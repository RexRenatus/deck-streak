# Red-first record: SPEC-059

The SPEC and ADR-066 were planned alone (2fb13a7). The tests and the stubs they need to compile were
committed next (ca3b54b): a `SyncRequester` whose port refuses `unbuilt`, and the `StillRunning`
outcome and its reply. The implementation, the deploy templates and the mutation rows followed in one
commit (f6afb08). Each red below was run over the whole test file, from a `git archive` export of
ca3b54b, so five of the six sync_request tests failed together by assertion and the path test failed
on its own assertion. A6 was already true at the base: the owner gate and the job table are
SPEC-026's and SPEC-027's, and this delivery only adds a caller behind them.

```red-first
A1: red at ca3b54b: panicked at crates/daemon/tests/sync_request.rs:146: the bot role runs no cycle in its own process
A1: green at f6afb08
A2: red at ca3b54b: panicked at crates/daemon/tests/sync_request.rs:177: the stored request does (the stub read no flag)
A2: green at f6afb08
A3: red at ca3b54b: AssertionError: Lists differ: ['PathChanged must be the request file alone'] != [] (no path unit existed)
A3: green at f6afb08
A4: red at ca3b54b: Err(SyncRefusal { reason: "unbuilt" }) for both requests of a_second_request_waits_out_the_ring_gap
A4: green at f6afb08
A5: red at ca3b54b: assertion left == right failed, left: Err(SyncRefusal { reason: "unbuilt" })
A5: green at f6afb08
A6: not red: the owner gate (an_update_from_anyone_but_the_owner_is_dropped_without_a_reply) and the job table's one daily slot are SPEC-026 and SPEC-027 facts, green at the base and unchanged by this delivery
```

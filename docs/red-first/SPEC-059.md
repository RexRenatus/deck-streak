# Red-first record: SPEC-059

The SPEC and ADR-066 were planned alone (2fb13a7). The tests and the stubs they need to compile were
committed next (ca3b54b): a `SyncRequester` whose port refuses `unbuilt`, and the `StillRunning`
outcome and its reply. The implementation, the deploy templates and the mutation rows followed in one
commit (f6afb08). Each red below was run over the whole test file, from a `git archive` export of
ca3b54b, so all five sync_request tests failed together by assertion and the path test failed on its
own assertion. A6 was already true at the base: the owner gate and the job table are SPEC-026's and
SPEC-027's, and this delivery only adds a caller behind them.

The refuse-all stub at ca3b54b cannot be red for a criterion's own reason. It fails the first
positive assertion of a test, often a precondition, so the assertions that decide the criterion
never ran. A4 and A5 are therefore recorded `not red`, naming the stub, and what decides them is
the mutants that now fail them: the ring mutants in `crates/daemon/src/sync_request.rs` (the gap comparison, the reuse window and the answer bound) fail `a_second_request_waits_out_the_ring_gap`, `a_request_inside_the_reuse_window_is_answered_reused` and `the_owner_is_answered_within_the_bound`, and the doorbell body replaced by `Ok(())` fails `the_doorbell_writes_its_file_and_refuses_an_absent_directory`. The A4 reuse test did fail on its own answer at ca3b54b
(`Err(unbuilt)` against `Ok(Reused)`), and the gap test failed on its precondition
`first.is_ok() && second.is_ok()`; one A4 line covers both because the record keeps one status
per criterion. The gap test's final assertion read `>=` when it was first committed green (d1a7eb2)
and reads `==` at the head: d1a7eb2 tightened it after green, so that a ring that waits longer than
the gap is also refused.

Fix round 1 changed how A2 and A3 are decided. The old A2 test only asked the store whether a
request was pending, and never ran the job, so no job could fail it: a job that also served when
the request file held the word "owner" passed it and passed
`roles::only_the_sync_job_serves_the_owners_stored_request`. A2's test is now
`roles::two_planted_request_payloads_change_nothing_the_sync_job_serves` (3b5f7aa), which runs
`deckstreakd job sync` twice with a planted request file: flag clear and a payload that pretends to
command the job (0 owner-request events), flag pending and a malformed payload (exactly 1), the
file untouched byte for byte in both. The old store-predicate test is removed, since the new one
asserts the same predicate through the job and the old one asserted nothing more; the
S05901 row's killer is now the roles test. It is green at its own commit, so A2 is `not red`; it
is red only under the violating job, and under the verifier's violating plant (the job also serves when the request file contains "owner") the roles test `two_planted_request_payloads_change_nothing_the_sync_job_serves` fails on its event-count assertion at roles.rs:434, and it is green at the head. A3's test was strengthened (ad0f449): the
path unit's [Path] section is exactly `PathChanged=` on the request file, and a census of every
unit under `deploy/systemd` refuses zero examined and names the units that write the request
directory. A `PathModified=` line in the path unit and a `ReadWritePaths=` line in the API unit each
left the old suite green and turn the new one red.

```red-first
A1: red at ca3b54b: panicked at crates/daemon/tests/sync_request.rs:146: the bot role runs no cycle in its own process
A1: green at f6afb08
A2: not red: the criterion's test, which runs the job through the binary, is green at its own commit and red only under a job that also obeys the request file (quoted above)
A3: red at ca3b54b: AssertionError: Lists differ: ['PathChanged must be the request file alone'] != [] (no path unit existed)
A3: green at f6afb08
A4: not red: the stub at ca3b54b refused every request unbuilt, so the ring-count and gap assertions of the gap test never ran, and the reuse test failed on Err(unbuilt) against Ok(Reused)
A5: not red: the stub at ca3b54b refused every request unbuilt, so the bound, StillRunning and elapsed-time assertions never ran
A6: not red: the owner gate (an_update_from_anyone_but_the_owner_is_dropped_without_a_reply) and the job table's one daily slot are SPEC-026 and SPEC-027 facts, green at the base and unchanged by this delivery
```

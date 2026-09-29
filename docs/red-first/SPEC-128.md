# Red-first record: SPEC-128

The SPEC and ADR-128 were committed alone (2d432e5). Then came the tests of A1 to A9 (588dcf4)
against stubs that compile and change nothing: a migration without its `CHECK`, a `RefusalReason`
whose code is a placeholder, and a `record_refusal` that writes nothing, so every test ran and each
red failed by assertion. The implementation (210cd3f) turned them green. The replay ran the whole
test files on 588dcf4's tree: `refusal`, `data_rights`, `sync_request`, `roles`.

```red-first
A1: red at 588dcf4: refusal::the_migration_refuses_a_reason_outside_the_closed_set panicked at crates/ingest/tests/refusal.rs:38:5: a code outside the closed set is refused
A1: green at 210cd3f
A2: red at 588dcf4: refusal::a_refusal_stores_its_reason_and_instant_and_clears_the_flag panicked at crates/ingest/tests/refusal.rs:69:5: the refusal clears the flag
A2: green at 210cd3f
A3: red at 588dcf4: refusal::a_new_request_clears_the_refused_record panicked at crates/ingest/tests/refusal.rs:91:5: assertion failed: state.load().await.expect("read").refusal.is_some()
A3: green at 210cd3f
A4: red at 588dcf4: refusal::every_reason_round_trips_through_its_code panicked at crates/ingest/tests/refusal.rs:108:9: left: "unset" right: "rescore_unrecorded"
A4: green at 210cd3f
A5: red at 588dcf4: sync_request::the_store_answers_a_refusal_at_or_after_the_request_only panicked at crates/daemon/tests/sync_request.rs:331:5: left: Waiting right: Refused { reason: "recompute_refused" }
A5: green at 210cd3f
A6: red at 588dcf4: sync_request::a_refused_request_is_answered_with_its_reason_and_never_flushes panicked at crates/daemon/tests/sync_request.rs:354:5: left: Ok(SyncAnswer { sync: Reused, scores: Recomputed })
A6: green at 210cd3f
A7: red at 588dcf4: roles::a_refused_owner_request_is_recorded_and_the_next_run_does_not_retry_it panicked at crates/daemon/tests/roles.rs:502:5: left: (true, None) right: (false, Some("scope_settings_refused"))
A7: green at 210cd3f
A8: red at 588dcf4: roles::a_refused_recompute_setup_is_recorded_for_the_owner panicked at crates/daemon/tests/roles.rs:534:5: left: (true, None) right: (false, Some("recompute_refused"))
A8: green at 210cd3f
A9: red at 588dcf4: data_rights::the_refused_record_is_exported_and_an_erase_clears_it panicked at crates/ingest/tests/data_rights.rs:201:5: left: Null right: "sync_settings_refused"
A9: green at 210cd3f
```

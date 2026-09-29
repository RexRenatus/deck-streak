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

Fix round 1 adds A10 to A13. The tests of A10, A11 and A12 were committed alone (a41cde9) against a
`Progress::RefusedAfterRun` and a `Scores::Refused` that compile and change nothing (the refusal
answers as before, and the bot's new arm words nothing), so each red failed by assertion. The fix
(d958f4a) turned them green. A13 was green when it was written, so it is a `not red:` line, proved
by plants: a refusal-code literal renamed outside the closed set in `impl OwnerSyncCycle` (for
`rescore_unrecorded` and for `credentials_directory_refused`) turns it red by its assertion, "the
owner cycle refuses with rescore_unrecorded_x, which the job cannot record".

```red-first
A10: red at a41cde9: sync_request::a_refusal_after_the_owners_run_is_answered_beside_the_run panicked at crates/daemon/tests/sync_request.rs:377:5: left: Ran { failure: None } right: RefusedAfterRun { failure: None, reason: "recompute_failed" }
A10: green at d958f4a
A11: red at a41cde9: roles::a_refusal_after_the_owners_run_answers_the_request_beside_the_run panicked at crates/daemon/tests/roles.rs:577:5: the refusal recorded after the run answers beside it: Ran { failure: Some("network_unreachable") }
A11: green at d958f4a
A12: red at a41cde9: commands::a_refusal_after_a_run_is_answered_beside_the_syncs_own_line panicked at crates/bot/tests/commands.rs:405:5: the refusal's golden: left text "Synced with your Anki sync server.\n" right text "Synced with your Anki sync server.\nYour scores were not recomputed (<code>recompute_failed</code>), so they stand."
A12: green at d958f4a
A13: not red: written green at the head, because the seven codes the owner cycle records already parse; plants P3 and P5 turn it red by assertion
```

A1's body was strengthened after verification (03cb5de): it now also asserts that an instant with no
reason is refused. Replayed against the old migration it failed at
`crates/ingest/tests/refusal.rs:48:5` with "an instant without its reason is refused", because a
`CHECK` that evaluates to NULL passes; the migration's second disjunct now requires
`refused_reason IS NOT NULL`, and A1 is green at 03cb5de.

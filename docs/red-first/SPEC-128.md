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

## Addendum, 2026-09-29: the reason is a closed enum (#396, ADR-193)

The gap was planted first (e1c6a03): a refusal code defined in a new file and recorded through
`refused` is not read by the scan A13 ran, so the criterion passed while the code bypassed it. The
fix (d34307d) makes `refused` and the owner cycle's error take `RefusalReason`. A14's body at the
red commit is that plant; the green commit edits `crates/daemon/tests/roles.rs`, replacing the plant
with the variants test (the string of each variant equals the code stored today, no two share one,
the cycle's refusal type is the enum) and replacing A13's source scan with a read of the migration's
`CHECK`. The variants test carries a type assertion that does not compile against the old code, so
its own failure at the red commit cannot be quoted, and the red line quotes the plant's.

```red-first
A14: red at e1c6a03: roles::a_refusal_code_is_a_variant_of_the_closed_enum panicked at crates/daemon/tests/roles.rs:570:5: the scan passed a code defined in a new file: planted_unknown_code is recorded through `refused` and no scan reads it
A14: green at d34307d
A15: not red: written green at the head, because it pins behaviour the head already has (each refusal site refuses by its own code); the plants below turn it red by assertion
```

A13 keeps its earlier `not red` line above; its body changed at d34307d as said.

A later commit, the one that follows 3eca7dc4, edits `crates/daemon/tests/roles.rs` again: it moves the
compile-time type assertion into a named helper so clippy passes with `-D warnings`. The test body is
otherwise unchanged and green.

## Addendum, 2026-09-29, round 2: each refusal site is pinned by behaviour (#396)

The scan A13 ran counted the owner cycle's refusal codes, and the count held that every site still
refuses. The closed enum replaced the scan and nothing replaced the count, so a site swallowed into
an `Ok` answer, or a discarded result, was green at the head. The gap was planted first, on the head
before the tests: the credentials-directory refusal swallowed into `Ok(SyncAnswer { .. })` (P7) and
the rescore request's result discarded with `let _ = ..` (P9) each left every daemon test green
(55 passed, 0 failed, over 9 test binaries). A15's tests (ef7fd2da) then turn each plant red by
assertion, and are green at the head:

```text
P7: wiring::tests::the_owners_sync_without_a_credentials_directory_is_refused_by_its_own_code
  left: Ok(SyncAnswer { sync: Reused, scores: Unchanged })
 right: Err(CredentialsDirectoryRefused)
P9: wiring::tests::the_owners_sync_that_cannot_mark_the_rescore_is_refused_by_its_own_code
  left: Err(SyncSettingsRefused)
 right: Err(RescoreUnrecorded)
```

The other sites were planted the same way and are pinned already: swallowing the settings refusal
turns `the_owners_sync_marks_the_rescore_before_it_reads_its_settings` red (left
`Ok(SyncAnswer { sync: Reused, scores: Unchanged })`, right `Err(SyncSettingsRefused)`), swallowing
the scope refusal turns A7 red (left `(true, None)`, right `(false, Some("scope_settings_refused"))`),
and swallowing the cycle's own refusal turns A11 red ("no refusal recorded").

The bot's answer to a request its store cannot record (the `rescore_unrecorded` code in
`sync_request.rs`) was pinned by no test. Its test (d8e111bd) is red under a plant that changes the
code to `rescore_unrecordedx` and green at the head:

```text
H6: sync_request::a_request_the_store_cannot_record_is_refused_with_the_rescore_code
  left: Err(SyncRefusal { reason: "rescore_unrecordedx" })
 right: Err(SyncRefusal { reason: "rescore_unrecorded" })
```

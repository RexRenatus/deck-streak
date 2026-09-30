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

## Addendum, 2026-09-29, round 3: the cycle's own refusal and each `cycle_reason` arm pin their code (#396)

Round 2's tests pinned that the cycle's own refusal EXISTS, not which code it carries, and three arms
of `cycle_reason` (the sync, gate and window failures) were pinned by no assertion. The gap was
planted first, on the head before the tests (a merge of dev at be5a976b), each plant leaving every
daemon test green (58 passed, 0 failed, 1 ignored, over 9 test binaries):

- the cycle's site hard-codes `SyncRecordFailed`;
- the cycle's site remaps the code and still calls `cycle_reason`
  (`RecomputeFailed => SyncRecordFailed, other => other`), which raises no warning;
- each of the sync, gate and window arms of `cycle_reason` alone gives another code.

The sites' other two shapes were planted too: the cycle's refusal swallowed into an `Ok` answer turns
A11 red at `roles.rs:659` ("no refusal recorded"), and dropping the `?` does not compile (`E0308`).
The new assertions (e970b038) are three cases in
`a_cycle_that_cannot_finish_is_refused_with_its_steps_reason_code` and a code assertion in A11, and
turn each code plant red by assertion. They are recorded `not red` under A15 above for the same
reason as its other tests: they pin behaviour the head already has, so there is no green line. With
no plant, the daemon `--tests` read 58 passed, 0 failed over 9 binaries:

```text
S5-code: roles::a_refusal_after_the_owners_run_answers_the_request_beside_the_run
  panicked at crates/daemon/tests/roles.rs:660:5
  left: SyncRecordFailed
 right: RecomputeFailed
S5-code2: the same test, the same line
  left: SyncRecordFailed
 right: RecomputeFailed
S6-sync: wiring::tests::a_cycle_that_cannot_finish_is_refused_with_its_steps_reason_code
  panicked at crates/daemon/src/wiring.rs:516:9
  left: RecomputeFailed
 right: SyncRecordFailed
S6-gate: the same test
  panicked at crates/daemon/src/wiring.rs:524:9
  left: SyncRecordFailed
 right: RecomputeFailed
S6-window: the same test
  panicked at crates/daemon/src/wiring.rs:528:9
  left: SyncRecordFailed
 right: RecomputeFailed
```

The rows `S12814` (the remap, as the mutant) and `S12815` to `S12817` (one arm each) are proved KILLED
on a committed tree and each survives at the head before the tests.

## Addendum, 2026-09-29, round 4: the cycle's refusal of an unreadable run record is driven through `run` (#396)

Round 3 pinned the cycle's own refusal on one of its three codes, so three shapes of the site stayed
green. This round drew the class as (site, code) pairs of `OwnerSyncCycle::run`, drove each pair it
took to be reachable, and disclosed one pair as unreachable in-process. Round 5 (below) found the
disclosure false and the pairs too coarse, and redraws the class by step. The population was
generated by reading `run`: every `map_err(.. refused(..))?` in it (the function holds no other `?`,
`Err(` or `refused(`) and, for each site, every code its mapping can give. That gives five sites and
seven pairs; the eighth code, `recompute_refused`, is given by the job's load of the recompute setup
in `role_job.rs`, outside `run`.

| site in `run` | code it can refuse with | driven through `run` by | pinned by a plant |
|---|---|---|---|
| rescore request (`wiring.rs:333`) | `rescore_unrecorded` | `wiring::tests::the_owners_sync_that_cannot_mark_the_rescore_is_refused_by_its_own_code` | round 2 |
| sync settings (`:335`) | `sync_settings_refused` | `wiring::tests::the_owners_sync_marks_the_rescore_before_it_reads_its_settings` | round 2 |
| credentials directory (`:337`) | `credentials_directory_refused` | `wiring::tests::the_owners_sync_without_a_credentials_directory_is_refused_by_its_own_code` | round 2 |
| read scope (`:339`) | `scope_settings_refused` | `roles::a_refused_owner_request_is_recorded_and_the_next_run_does_not_retry_it`, through `job sync` | round 2 |
| the cycle (`:358`) | `sync_record_failed` | `wiring::tests::the_owners_cycle_that_cannot_read_its_run_record_is_refused_by_the_sync_code` | this round |
| the cycle (`:358`) | `recompute_failed` | `roles::a_refusal_after_the_owners_run_answers_the_request_beside_the_run` | round 3 |
| the cycle (`:358`) | `obligations_unreadable` | not in this round, see below; round 5's table | round 5 |

This round drove no test through `run` to `obligations_unreadable` at the cycle's site. It took the
pair to be unreachable in-process, because the obligations step reads the run record that the history
step reads first. No construction was attempted, and the premise is false: the history step reads
only a run's id and status, so a run whose study day cannot be evaluated fails the obligations step
alone, and round 5 drives it. A remap of that code alone at the site stayed green in this round. The
six `CycleError` variants reach the site through one call of `cycle_reason`, so a (site, code) pair
counts `recompute_failed` once for three steps and `sync_record_failed` once for two. The sync, gate
and window steps reached the site with no test driving them, and a remap or an answer of any one of
them stayed green (round 5).

The gap was planted first, at the head of this round's merge of dev (3034ecc5), each plant leaving
every daemon test green (58 passed, 0 failed, 1 ignored, over 9 test binaries): N5, the site remaps
`SyncRecordFailed` to `RecomputeFailed` and still calls `cycle_reason`; N6, the site hard-codes
`RecomputeFailed` (the only signal is a dead-code warning for `cycle_reason`); N7, the site takes its
code from a stale variable holding `RecomputeFailed`; N8, the site swallows the history and sync
failures into `Ok(SyncAnswer { sync: Reused, scores: Unchanged })`. The new test (38d53c2d) renames
`sync_runs` so the cycle's first step fails, runs the cycle twice and asserts
`Err(RefusalReason::SyncRecordFailed)` each time. It is recorded `not red` under A15 above: it pins
behaviour the head already has, so it is green with no plant (59 passed, 0 failed over 9 binaries)
and takes no green line. Each plant turns it red by assertion. The line of the assertion follows the
length of the planted text above it, so the lines below are this round's plants', formatted as
`rustfmt` leaves them; the values are the point:

```text
N5: wiring::tests::the_owners_cycle_that_cannot_read_its_run_record_is_refused_by_the_sync_code
  panicked at crates/daemon/src/wiring.rs:689:13
  left: Err(RecomputeFailed)
 right: Err(SyncRecordFailed)
N6: the same test
  panicked at crates/daemon/src/wiring.rs:681:13
  left: Err(RecomputeFailed)
 right: Err(SyncRecordFailed)
N7: the same test
  panicked at crates/daemon/src/wiring.rs:685:13
  left: Err(RecomputeFailed)
 right: Err(SyncRecordFailed)
N8: the same test
  panicked at crates/daemon/src/wiring.rs:691:13
  left: Ok(SyncAnswer { sync: Reused, scores: Unchanged })
 right: Err(SyncRecordFailed)
```

The row `S12818` (41d0a854) has N5 as its mutant and this test as its killer, and is proved KILLED on a
committed tree; the killer is new in this round, so the row's survival is the plant measurement
above, not a run of the row at a tree without its killer.

## Addendum, 2026-09-30, round 5: every step whose failure refuses the owner's sync is driven through `run` and named in its refusal (#396)

Round 4 drew the class as (site, code) pairs, and a pair counts a code once however many steps give
it. This round draws it by step. The steps were generated by reading the code, not listed: each
`refused(..)?` in `run` before the cycle's own site, and each kind of error the cycle's steps
return, read from `CycleError` and, where a variant carries the sync's, the gate's or the window's
own error, from that error's kinds. The line stops at the shared causes `KernelError` and
`ReadError`, which name the store or the copy that failed under any step. That gives twelve steps.
Crossed with the eight codes, the population is each step's refusal answered with each other code
(84), the cycle's site answering one fixed code for every step (8), and each step's failure answered
with an `Ok` (12): 104 members. The table's own faults are a further 132: each step's fault replaced
by another step's. Every step is reached in-process by the fault below, and none is disclosed as
unreachable.

| step | where `run` refuses it | code | the fault that reaches it through `run` |
|---|---|---|---|
| `rescore` | the rescore request | `rescore_unrecorded` | the ledger closed once the cycle is built |
| `settings` | the sync settings | `sync_settings_refused` | no sync endpoint |
| `credentials` | the credentials directory | `credentials_directory_refused` | no credentials directory |
| `scope` | the read scope | `scope_settings_refused` | a malformed law-deck root |
| `history` | the cycle's site, `CycleError::History` | `sync_record_failed` | the run record renamed away |
| `sync` | the cycle's site, `SyncError::Store` | `sync_record_failed` | a trigger refusing the sync's write of its run |
| `obligations` | the cycle's site, `CycleError::Obligations` | `obligations_unreadable` | a run whose study day cannot be evaluated, a column the history and the sync do not read |
| `gate_probe` | the cycle's site, `GateError::Read` | `recompute_failed` | no copy of the collection |
| `gate_record` | the cycle's site, `GateError::Record` | `recompute_failed` | a trigger refusing the gate's anchor, after the window and the recompute have run |
| `window_read` | the cycle's site, `WindowError::Read` | `recompute_failed` | the copy's decks renamed away, a table the gate's probe does not read |
| `window_base` | the cycle's site, `WindowError::State` | `recompute_failed` | a trigger refusing the window's base |
| `recompute` | the cycle's site, `CycleError::Recompute` | `recompute_failed` | the analytics rollup renamed away, the first table the fold reads (the XP ledger at bea2ca4f and b9c597c7; a fixture in the daemon may not name that table, SPEC-040 R10) |

Round 4's row for `obligations_unreadable` at the cycle's site is replaced by this table's
`obligations` row.

A15 is narrowed, as section 8 of SPEC-128 records. It said that every refusal site of the owner's
sync refuses by its own code and that a site swallowed into an `Ok` answer or a discarded result is
caught; at the cycle's site a single step's failure could be answered with its tests green. It now
covers the credentials directory's and the rescore request's sites and the bot's answer, which its
three tests pin, and every other step is A16's. A15's fence lines are unchanged.

A16 and A17 are new. The table test and the guard were written first (bea2ca4f) against the head's
code, where `refused` logs no step and the cycle's mapping names six kinds of error: A16 fails at its
first step, and A17 counts the six against the table's eight cycle rows. The rule (b9c597c7) makes
`refused` take a `Step` and log its name, and `Step::of` name each kind. A16's table test drives
every step twice, each on a fresh ledger, and its check fails a step driven once, so a code that
changes on a retry is caught.

```red-first
A16: red at bea2ca4f: wiring::tests::every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name panicked at crates/daemon/src/wiring.rs:929:9: the rescore step's refusal is logged once, under its own name: left [(None, Some("rescore_unrecorded"))], right [(Some("rescore"), Some("rescore_unrecorded"))]
A16: green at b9c597c7
A17: red at bea2ca4f: wiring::tests::the_table_has_a_row_for_every_step_that_refuses_the_owners_sync panicked at crates/daemon/src/wiring.rs:993:9: each kind of cycle error `Step::of` names is a step in the table: left 6, right 8
A17: green at b9c597c7
```

Every member the round-4 table left green, and the gate's and the window's second kinds, was planted
one at a time in the rule's code, and each turns the table test red by assertion over the daemon's
whole suite (63 passed, 0 failed, 1 ignored, over 9 test binaries, with no plant). N3d
answers a retry of the window's base alone by another code, which only the second drive sees. The
table's own mutants are M1, a row dropped; M3, the guard's count off by one; M7, each step driven
once; M2, the `Ok` assertion dropped; M5, the step's name no longer asserted; and M8, the drive
check dropped. M2, M5 and M8 fail the meta tests that expect the table's check to panic. The line of
an assertion follows the length of the planted text above it; the values are the point:

```text
N5: wiring::tests::every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name
  panicked at crates/daemon/src/wiring.rs:1008:28: the history step refuses by its own code
  left: RecomputeFailed
 right: SyncRecordFailed
N5b: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the obligations step refuses by its own code
  left: SyncRecordFailed
 right: ObligationsUnreadable
N6: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the history step refuses by its own code
  left: RecomputeFailed
 right: SyncRecordFailed
N7: the same test
  panicked at crates/daemon/src/wiring.rs:1011:28: the history step refuses by its own code
  left: RecomputeFailed
 right: SyncRecordFailed
N8: the same test
  panicked at crates/daemon/src/wiring.rs:1009:27: the history step's failure was answered: SyncAnswer { sync: Reused, scores: Unchanged }
N3b: the same test
  panicked at crates/daemon/src/wiring.rs:1009:28: the history step refuses by its own code
  left: RecomputeFailed
 right: SyncRecordFailed
N3d: the same test
  panicked at crates/daemon/src/wiring.rs:1009:28: the window_base step refuses by its own code
  left: SyncRecordFailed
 right: RecomputeFailed
NV-sync-ok: the same test
  panicked at crates/daemon/src/wiring.rs:1009:27: the sync step's failure was answered: SyncAnswer { sync: Reused, scores: Unchanged }
NV-sync-remap: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the sync step refuses by its own code
  left: RecomputeFailed
 right: SyncRecordFailed
NV-gate-ok: the same test
  panicked at crates/daemon/src/wiring.rs:1009:27: the gate_probe step's failure was answered: SyncAnswer { sync: Reused, scores: Unchanged }
NV-gate-remap: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the gate_probe step refuses by its own code
  left: SyncRecordFailed
 right: RecomputeFailed
NV-window-remap: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the window_read step refuses by its own code
  left: SyncRecordFailed
 right: RecomputeFailed
NV-history-remap: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the history step refuses by its own code
  left: RecomputeFailed
 right: SyncRecordFailed
NV-recompute-remap: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the recompute step refuses by its own code
  left: SyncRecordFailed
 right: RecomputeFailed
NV-gate-record-ok: the same test
  panicked at crates/daemon/src/wiring.rs:1009:27: the gate_record step's failure was answered: SyncAnswer { sync: Reused, scores: Unchanged }
NV-window-read-remap: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the window_read step refuses by its own code
  left: SyncRecordFailed
 right: RecomputeFailed
NV-window-base-ok: the same test
  panicked at crates/daemon/src/wiring.rs:1009:27: the window_base step's failure was answered: SyncAnswer { sync: Reused, scores: Unchanged }
NS-helper-obligations: the same test
  panicked at crates/daemon/src/wiring.rs:1012:28: the obligations step refuses by its own code
  left: SyncRecordFailed
 right: ObligationsUnreadable
NS-helper-fixed: the same test
  panicked at crates/daemon/src/wiring.rs:1009:28: the rescore step refuses by its own code
  left: RecomputeFailed
 right: RescoreUnrecorded
NS-swap-rescore-scope: the same test
  panicked at crates/daemon/src/wiring.rs:1008:28: the rescore step refuses by its own code
  left: ScopeSettingsRefused
 right: RescoreUnrecorded
M1, a table row dropped: wiring::tests::the_table_has_a_row_for_every_step_that_refuses_the_owners_sync
  panicked at crates/daemon/src/wiring.rs:1073:9: each kind of cycle error `Step::of` names is a step in the table
  left: 8
 right: 7
M3, the guard's count off by one: the same test
  panicked at crates/daemon/src/wiring.rs:1069:9: each `?` in run before the cycle's own is a step in the table
  left: 5
 right: 6
M7, each step driven once: wiring::tests::every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name
  panicked at crates/daemon/src/wiring.rs:1020:9: the rescore step is driven twice, each time on a fresh ledger
  left: 1
 right: 2
M2, the Ok assertion dropped: wiring::tests::the_table_fails_a_step_whose_failure_is_answered - should panic ... FAILED
  note: test did not panic as expected at crates/daemon/src/wiring.rs:1084:8
M5, the step's name no longer asserted: wiring::tests::the_table_fails_a_refusal_logged_under_another_steps_name - should panic ... FAILED
  note: test did not panic as expected at crates/daemon/src/wiring.rs:1101:8
M8, the drive check dropped: wiring::tests::the_table_fails_a_step_driven_once - should panic ... FAILED
  note: test did not panic as expected at crates/daemon/src/wiring.rs:1119:8
```

The rows `S12819` to `S12828` take these shapes as their mutants and the table test or the guard as
their killer. Each is killed when installed alone over the daemon's whole suite. The killers are new
in this round, so each row whose find has an equivalent at the head was installed there, and each
survives (59 passed, 0 failed, 1 ignored, over 9 binaries). `S12825` and `S12827` log a
step's failure under a sibling's name, which the head cannot do, since it names no step.

## Addendum, 2026-09-30, round 6: a refusal is pinned at the error level it is logged at (#396)

A16's log capture answered every level, so a refusal logged at warn, at info or at debug was still
counted as named. It now enables error events only, the level the service keeps and alerts on
(85749242). A16's fence lines are unchanged: the pin narrows what its table test accepts. Each
plant lowers the level of the one `refused` log call. Without the pin (acf615ca) each is green over
the daemon's library tests (13 passed, 0 failed); with it each turns the table test red by
assertion, at the rescore step, the first step driven:

```text
warn: wiring::tests::every_failing_step_refuses_the_owners_sync_by_its_own_code_and_name
  panicked at crates/daemon/src/wiring.rs:1014:9
  left: []
 right: [(Some("rescore"), Some("rescore_unrecorded"))]
info: the same test, the same line, the same left and right
debug: the same test, the same line, the same left and right
```

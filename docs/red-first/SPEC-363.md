# Red-first record: SPEC-363

The SPEC (9faa3403), ADR-374 (7d2b1db0) and the schematic (ad5e9bc9) were committed first, then the
formal model, before any of the code it covers. Each criterion's test was then committed before the
code that turns it green, and each red below is quoted from the run at the red commit. This
delivery is part 1 of SPEC-363: its criteria are A1 to A18; the Worker's store, its operations, the
page's sign-out and the reach census are the next pull request's (section 7).

## The fence, line by line

Each of the 18 lines of SPEC-363 section 3's fence resolves to a test this delivery adds, or to a
test that stands on `dev` and is named as it is.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `scripts/tests/test_formal_config.py` `the_committed_file_holds_exactly_the_declared_fields` | named: dev's test, its `EXPECTED` entries grown by one line, insert-only (step 3) |
| 2 | A2 | `crates/engine-core/tests/credential.rs` `a_login_is_kept_only_at_the_generation_it_started` | added (step 4) |
| 3 | A3 | `crates/engine-core/tests/credential.rs` `a_send_needs_the_held_generation_to_be_current` | added (step 4) |
| 4 | A4 | `crates/engine-core/tests/credential.rs` `only_the_sync_servers_refusal_is_a_refusal` | added (step 4) |
| 5 | A5 | `crates/engine-core/tests/credential.rs` `a_refusal_drops_only_the_generation_it_refused` | added (step 4) |
| 6 | A6 | `crates/engine-core/tests/credential.rs` `a_removal_raises_the_generation_and_it_never_wraps` | added (step 4) |
| 7 | A7 | `crates/engine-core/tests/graph.rs`, whole | named: unchanged |
| 8 | A8 | `crates/web-engine/tests/boundary.rs` `each_boundary_function_reaches_the_engine_through_the_dispatcher` | named: dev's census, its `OWED` list grown from 24 to 29 rows, insert-only (step 5) |
| 9 | A9 | `crates/identity/tests/sync_seal.rs` `the_seal_key_is_the_labelled_hmac_of_the_seal_id` | added (step 6) |
| 10 | A10 | `crates/identity/tests/sync_seal.rs` `a_seal_secret_under_32_bytes_refuses_start` | added (step 6) |
| 11 | A11 | `crates/api/tests/sync_seal_routes.rs` `the_seal_key_is_released_only_to_an_owner_session` | added (step 6) |
| 12 | A12 | `crates/api/tests/sync_seal_routes.rs` `a_seal_id_that_is_not_sixteen_bytes_is_refused` | added (step 6) |
| 13 | A13 | `crates/api/tests/sync_seal_routes.rs` `the_release_has_a_bound_of_its_own` | added (step 6) |
| 14 | A14 | `crates/api/tests/sync_seal_routes.rs` `the_release_is_not_stored_and_not_cross_site` | added (step 6) |
| 15 | A15 | `crates/api/tests/sync_seal_routes.rs` `the_release_is_off_without_the_seal_secret` | added (step 6) |
| 16 | A16 | `crates/api/tests/sync_seal_routes.rs` `no_seal_secret_id_or_key_reaches_a_record` | added (step 6) |
| 17 | A17 | `crates/daemon/src/wiring.rs` `wiring::tests::a_missing_seal_secret_turns_the_release_off` | added (step 6) |
| 18 | A18 | `crates/identity/tests/sync_seal.rs` `a_seal_id_that_is_not_sixteen_bytes_does_not_parse` | added (step 6) |

## The model first

`formal/tla/SyncCredential` was committed at 592c581a, with its configuration and its five
witnesses, before `crates/engine-core/src/credential.rs` or `crates/api/src/sync_seal_routes.rs`
existed. Its six `covers` lines carried a digest of 64 zeros. The paired checker's
`--entry tla/SyncCredential` check at that commit, and again at the floor's commit 93355e7e, read
`FORMAL OK exit=0` with 30 `FORMAL REPORT STALE ... does not resolve` lines (six covers on each of
five properties: the items they name did not exist yet). It reported no `WITNESS_SURVIVED`, `FLOOR`,
`VIOLATION`, `TIMEOUT` or `MODEL_ERROR`, so each witness was caught:

| property | witness |
|---|---|
| `NoKeyAtRestAfterSignOut` | `a-login-kept-after-a-sign-out` |
| `NoSendAfterSignOut` | `a-send-from-memory-after-a-sign-out` |
| `ARefusalDropsOnlyItsOwnKey` | `a-refusal-that-drops-a-newer-key` |
| `AFailureKeepsTheKey` | `a-network-failure-that-drops-the-key` |
| `UnsealOnlyInALiveSession` | `an-unseal-with-no-session` |

The state floor is one hand run of the model's configuration, a measurement only: 648076 states
generated, 47648 distinct states, a complete state graph of depth 17, in 2.6 s, so the
configuration reads `states >= 47648` (93355e7e). The budget is four timed runs of the entry's
check, 24.1 s (before the floor), 23.5 s, 24.2 s and 24.3 s, so ceil(24.3 x 1.5 / 60) x 60 = 60
seconds, which A1 pins.

The covers were stamped at 641eadc3, after the last edit to `credential.rs` and
`sync_seal_routes.rs`: each digest moved from the zeros to its item's span, and the model's comments
record the re-read of each covered item beside the action it maps to. At that commit the entry's
check read `FORMAL OK exit=0`, with no `STALE`, `WITNESS_SURVIVED`, `FLOOR`, `VIOLATION`, `TIMEOUT`
or `MODEL_ERROR`, and each of the five properties clean.

## The reds and greens

Each line's command is the criterion's line in SPEC-363 section 3's fence, run at the commit named.

```red-first
A1: red at 6f239b5b: AssertionError: False is not true : budgets.entries: {... 'tla/StagedNoJournal': 120, 'tla/SyncSnapshotWindow': 60, ...} != {... 'tla/StagedNoJournal': 120, 'tla/SyncCredential': 60, 'tla/SyncSnapshotWindow': 60, ...}
A1: green at 6a367094
A2: red at 8129c172: panicked at crates/engine-core/tests/credential.rs:41: a login started at 3 lands after a removal or another kept login raised the store to 4; left: Store { at: Generation(5) }, right: Discard
A2: green at 3dac1ad4
A3: red at 8129c172: panicked at crates/engine-core/tests/credential.rs:71: a sender holding 1 after the store moved to 2 sends nothing
A3: green at 3dac1ad4
A4: red at 8129c172: panicked at crates/engine-core/tests/credential.rs:98: a network failure keeps the key; left: Refused, right: Failed
A4: green at 3dac1ad4
A5: red at 8129c172: panicked at crates/engine-core/tests/credential.rs:175: a refusal of 2 after another Worker kept 3 leaves the newer key alone
A5: green at 3dac1ad4
A6: red at 8129c172: panicked at crates/engine-core/tests/credential.rs:201: a removal at 0 stores 1; left: Some(Generation(0)), right: Some(Generation(1))
A6: green at 3dac1ad4
A7: not red: the core's dependency census stands on dev unchanged, and this delivery adds no dependency, so it was green at the base and stays green at 3dac1ad4
A8: red at 877aa290: panicked at crates/web-engine/tests/boundary.rs:346:5: each of the five fn credential_<name>( occurs 0 times, not once
A8: green at 19662566
A9: red at eae463f4: panicked at crates/identity/tests/sync_seal.rs:109:9: the sealing key under a 32-byte secret is the labelled HMAC of the seal id; left: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
A9: green at 7294f8f5
A10: red at eae463f4: panicked at crates/identity/tests/sync_seal.rs:134:9: a 0-byte seal secret refuses start
A10: green at 7294f8f5
A11: red at 71c15f5a: assertion `left == right` failed: a link session is refused; left: 200, right: 401
A11: green at d005120e
A12: red at 71c15f5a: assertion `left == right` failed: 15 bytes is refused; left: 200, right: 400
A12: green at d005120e
A13: red at 71c15f5a: assertion `left == right` failed: the 31st is refused; left: 200, right: 429
A13: green at d005120e
A14: red at 71c15f5a: assertion `left == right` failed: no cache keeps a released key; left: None, right: Some([110, 111, 45, 115, 116, 111, 114, 101])
A14: green at d005120e
A15: red at 71c15f5a: assertion `left == right` failed: the owner's session finds the route off; left: 200, right: 404
A15: green at d005120e
A16: red at 71c15f5a: the seal id reaches a record: INFO message=a sealing key was released seal_id=AAECAwQFBgcICQoLDA0ODw
A16: green at d005120e
A17: red at 6bbaee11: no seal secret turns the release off: Err(Credential(Missing { id: "sync-seal-secret" }))
A17: green at 73672a15
A18: red at eae463f4: panicked at crates/identity/tests/sync_seal.rs:195:9: 15 bytes does not parse
A18: green at 7294f8f5
```

## What the record discloses

- **identity's finalize census grew by one site, red first.** The sealing key is the second HMAC
  identity finalizes to derive a key, so SPEC-363 section 4 admits `crates/identity/tests/boundary.rs`
  and its exact finalize list grew, insert-only, from `["init_data.rs"]` to
  `["init_data.rs", "sync_seal.rs"]`. That growth was committed alone at f970c2a5, against the stub
  that finalizes nothing, and read red there: `assertion left == right failed; left: ["init_data.rs"];
  right: ["init_data.rs", "sync_seal.rs"]`. It reads green at 7294f8f5. The list stays an exact
  equality, so a third site still fails, and the clause that refuses any comparison naming a hash
  reads every file under `crates/identity/src`, `sync_seal.rs` included.
- **A red commit also turned one of dev's tests red, and its green restored it unchanged.** At
  71c15f5a the router's state named the seal port in every state, so the API's
  `the_state_debug_line_says_which_ports_it_holds` read red beside A11 to A16. From d005120e the
  state's debug line names the seal port only while it is held, and that test reads green as dev
  wrote it.
- **A16 gained a check at its green.** At d005120e A16 also reads the state's debug line with a seal
  secret held: it names the port and no byte of the secret. That check was green when written; A16's
  red at 71c15f5a is its log line.
- **Two reds are quoted in short.** A3's and A5's reds are `assert!` failures, quoted by their file,
  line and message; the red runs printed no left and right for them.
- **The reader is not composed yet.** A17 proves `seal_secret`, the daemon's reader of the
  `sync-seal-secret` credential. The API's role does not call it in this delivery, so the release
  stays off in production until a later change composes it.

## What the mutation pass added

The diff's own mutants were listed and run before the push, with `cargo mutants --in-diff` over
`git diff origin/dev...HEAD`: 65 listed and 65 tested, of which 51 were caught, 2 missed, 0 timed
out and 12 unviable. The 12 unviable are `Default::default()` replacements that do not compile;
they are recorded as unviable, never as caught.

The two missed mutants each delete a match arm of `credential_on_outcome` in
`crates/web-engine/src/wasm.rs`: arm 0, accepted, and arm 1, refused. That module is built only for
`wasm32`, so a native build compiles each mutant as the unchanged crate, and the boundary census
(A8) reads the body's call to the core's rule, not the arms that pick its outcome. Both are
recorded equivalent in `scripts/mutation-equivalent.d/deck-streak-web-engine.json` under #303, the
class of every record that file already holds:

- Deleting arm 0 changes no answer on any target: 0 then reads as failed, and the core's
  `on_outcome` drops the key only on a refusal, so an accepted answer and a failed one both keep it.
- Deleting arm 1 would read a refusal as failed on `wasm32`. No native test can see it; A5 holds
  the core's rule the export calls, and part 2's Worker store is the export's first caller
  (SPEC-363 section 7).

# Red-first record: SPEC-359

The SPEC, ADR-370 and the schematic were committed first (be83c51). Each criterion's test was
committed red against a stub of part a's types and functions (bodies that answer the wrong value),
before the change that turns it green, and each red below is quoted from the run at the red
commit. The model `formal/tla/PasskeyOnce` was committed alone (a447466) before the first green.

## The fence, line by line

Each of the 36 lines of SPEC-359 section 3's fence resolves to a test this delivery adds, at the
commit named.

| # | criterion | test | added at |
|---|---|---|---|
| 1 | A1 | `crates/identity/tests/linking.rs` `a_link_code_is_random_and_kept_hashed` | 83b0290c |
| 2 | A2 | `crates/identity/tests/linking.rs` `a_link_code_expires_at_six_hundred_seconds` | 83b0290c |
| 3 | A3 | `crates/identity/tests/linking.rs` `a_link_code_redeems_once` | 83b0290c |
| 4 | A4 | `crates/identity/tests/linking.rs` `a_link_code_needs_a_fresh_telegram_session` | 83b0290c |
| 5 | A5 | `crates/identity/tests/linking.rs` `a_ninth_link_code_evicts_the_oldest` | 83b0290c |
| 6 | A6 | `crates/identity/tests/linking.rs` `a_link_session_is_not_an_owner_session` | 83b0290c |
| 7 | A7 | `crates/identity/tests/linking.rs` `a_linked_session_is_an_owner_session_that_mints_nothing` | 83b0290c |
| 8 | A8 | `crates/identity/tests/passkeys.rs` `a_passkey_from_another_origin_is_refused` | 786000ed |
| 9 | A9 | `crates/identity/tests/passkeys.rs` `a_passkey_without_user_verification_is_refused` | 786000ed |
| 10 | A10 | `crates/identity/tests/passkeys.rs` `a_reused_passkey_challenge_is_refused` | 786000ed |
| 11 | A11 | `crates/identity/tests/passkeys.rs` `a_passkey_challenge_expires_at_three_hundred_seconds` | 786000ed |
| 12 | A12 | `crates/identity/tests/passkeys.rs` `a_regressed_counter_is_refused` | 786000ed |
| 13 | A13 | `crates/identity/tests/passkeys.rs` `a_passkey_sign_in_stores_its_counter` | 786000ed |
| 14 | A14 | `crates/identity/tests/passkeys.rs` `the_user_handle_carries_no_personal_data` | 786000ed |
| 15 | A15 | `crates/identity/tests/passkeys.rs` `the_counter_rule_accepts_only_an_advance_or_two_zeros` | 786000ed |
| 16 | A16 | `crates/identity/tests/passkeys.rs` `a_ninth_ceremony_evicts_the_oldest` | 786000ed |
| 17 | A17 | `crates/identity/tests/passkeys.rs` `a_ceremony_finishes_only_in_its_own_session` | 786000ed |
| 18 | A18 | `crates/identity/tests/passkeys.rs` `a_passkey_with_a_foreign_signature_is_refused` | 786000ed |
| 19 | A19 | `crates/identity/tests/linking.rs` `linking_twice_is_refused` | 83b0290c |
| 20 | A20 | `crates/identity/tests/linking.rs` `a_sign_in_with_an_unlinked_identity_creates_nothing` | 83b0290c |
| 21 | A21 | `crates/identity/tests/linking.rs` `a_linked_identity_of_another_user_is_refused` | 83b0290c |
| 22 | A22 | `crates/identity/tests/linking.rs` `a_sign_in_rotates_the_session` | 83b0290c |
| 23 | A23 | `crates/identity/tests/linking.rs` `unlinking_telegram_is_refused` | 83b0290c |
| 24 | A24 | `crates/identity/tests/linking.rs` `unlinking_needs_a_fresh_telegram_session` | 83b0290c |
| 25 | A25 | `crates/identity/tests/linking.rs` `removing_a_passkey_ends_its_sessions` | 83b0290c |
| 26 | A26 | `crates/identity/tests/linking_config.rs` `linking_is_off_without_the_public_origin` | 5d6d124e |
| 27 | A27 | `crates/identity/tests/linking_config.rs` `a_public_origin_that_is_not_https_refuses_start` | 5d6d124e |
| 28 | A28 | `crates/identity/tests/rights.rs` `passkeys_export_and_erase_are_symmetric` | 5d6d124e |
| 29 | A29 | `crates/identity/tests/linking.rs` `no_linking_secret_reaches_a_log` | 56a948d9 |
| 30 | A30 | `crates/api/tests/linking_routes.rs` `a_link_session_reaches_only_the_linking_routes` | 5d6d124e |
| 31 | A31 | `crates/api/tests/linking_routes.rs` `the_ceremony_cookie_is_host_prefixed_strict_and_short_lived` | 5d6d124e |
| 32 | A32 | `crates/api/tests/linking_routes.rs` `a_ceremony_flood_is_bounded_and_spares_the_handshake` | 5d6d124e |
| 33 | A33 | `crates/api/tests/linking_routes.rs` `every_linking_route_refuses_a_cross_site_request` | 56a948d9 |
| 34 | A34 | `crates/api/tests/linking_routes.rs` `the_identities_list_names_methods_without_credential_ids` | 5d6d124e |
| 35 | A35 | `crates/identity/tests/passkeys.rs` `a_counter_moved_since_its_read_is_refused` | 786000ed |
| 36 | A36 | `crates/identity/tests/linking.rs` `each_refusal_answers_its_status` | 83b0290c |

## Red, then green

Each line's command is the criterion's line in SPEC-359 section 3's fence, run at the commit named.

```red-first
A1: red at 83b0290c: assertion `left != right` failed: two mints answered one code: it is not random (both "AAAAAAAAAAAAAAAAAAAAAA")
A2: red at 83b0290c: assertion `left == right` failed: a code 600 seconds old was not refused link_code_expired (left None)
A3: red at 83b0290c: assertion `left == right` failed: a second redeem was not refused link_code_invalid (left None)
A4: red at 83b0290c: assertion `left == right` failed: a Telegram session of 301 seconds minted a code (left None)
A5: red at 83b0290c: assertion `left == right` failed: the oldest code survived a ninth mint (left None)
A6: red at 83b0290c: a link session was admitted as an owner session
A7: red at 83b0290c: assertion `left == right` failed: a linked session minted a link code (left None)
A8: red at 786000ed: passkeys.rs:99: left: Err(Some(ChallengeInvalid)) right: Err(Some(OriginMismatch))
A9: red at 786000ed: passkeys.rs:140: left: Err(Some(ChallengeInvalid)) right: Err(Some(UvRequired))
A10: red at 786000ed: passkeys.rs:196: left: Ok(1) right: Err(Some(ChallengeInvalid))
A11: red at 786000ed: passkeys.rs:253: left: Ok(1) right: Err(Some(ChallengeExpired))
A12: red at 786000ed: passkeys.rs:275: left: Err(Some(ChallengeInvalid)) right: Err(Some(CounterRegressed))
A13: red at 786000ed: passkeys.rs:297: left: Some((5, 0, None)) right: Some((7, 0, Some(1736911820000)))
A14: red at 786000ed: passkeys.rs:328: two registrations' user handles differ (left/right are two random 16-byte handles)
A15: red at 786000ed: passkeys.rs:355: left: true right: false (stored 1, presented 1)
A16: red at 786000ed: passkeys.rs:382: left: Ok(1) right: Err(Some(ChallengeInvalid))
A17: red at 786000ed: passkeys.rs:428: left: Ok(1) right: Err(Some(ChallengeInvalid))
A18: red at 786000ed: passkeys.rs:451: left: Err(Some(ChallengeInvalid)) right: Err(Some(PasskeyInvalid))
A19: red at 83b0290c: assertion `left == right` failed: a credential registered twice was not refused already_linked (left Some(None))
A20: red at 83b0290c: assertion `left == right` failed: a sign-in with no passkey held was not refused not_linked (left None)
A21: red at 83b0290c: assertion `left == right` failed: another user's passkey was not refused not_owner (left Some(Some(ChallengeInvalid)))
A22: red at 83b0290c: assertion `left == right` failed: the session the sign-in arrived with is still live (left Some(Telegram))
A23: red at 83b0290c: assertion `left == right` failed: removing Telegram was not refused last_method (left Some(Some(IdentityUnknown)))
A24: red at 83b0290c: assertion `left == right` failed: a linked session removed a passkey (left None)
A25: red at 83b0290c: assertion `left == right` failed: a session the removed passkey opened is still live (left Some(Linked(1)))
A26: red at 5d6d124e: the public origin Some("") refused start: unset or blank is linking off
A27: red at 5d6d124e: the public origin "http://app.example" was accepted: it is not an https origin
A28: red at 5d6d124e: assertion `left == right` failed: the owner's passkeys are the owner's data, exported whole and erased (left [])
A29: red at 83b0290c: the capture saw no audit event `link_code_minted`: the positive control found none
A30: red at 5d6d124e: assertion `left == right` failed: DELETE /api/identities/1 answered without a session: {"reason":"identity_unknown"} (left 400, right 401)
A31: red at 5d6d124e: assertion `left == right` failed: one Set-Cookie: [] (left 0, right 1)
A32: red at 5d6d124e: assertion `left == right` failed: /api/passkeys/sign-in/start was not bounded (left 200 with options, right 429 too_many_ceremonies)
A33: red at 5d6d124e: assertion `left == right` failed: POST /api/link/code answered a cross-site request otherwise (left 401 no_session, right 403 cross_site_request)
A34: red at 5d6d124e: assertion `left == right` failed: the methods list is not Telegram then the passkey (left Array [])
A35: red at 786000ed: passkeys.rs:490: left: Ok(1) right: Err(Some(CounterRegressed))
A36: red at 83b0290c: assertion `left == right` failed: a refusal answers another status (left ("reauth_required", 400), right ("reauth_required", 401))
A1: green at e2d01121
A2: green at e2d01121
A3: green at e2d01121
A4: green at e2d01121
A5: green at e2d01121
A6: green at e2d01121
A7: green at e2d01121
A8: green at e2d01121
A9: green at e2d01121
A10: green at e2d01121
A11: green at e2d01121
A12: green at e2d01121
A13: green at e2d01121
A14: green at e2d01121
A15: green at e2d01121
A16: green at e2d01121
A17: green at e2d01121
A18: green at e2d01121
A19: green at e2d01121
A20: green at e2d01121
A21: green at e2d01121
A22: green at e2d01121
A23: green at e2d01121
A24: green at e2d01121
A25: green at e2d01121
A26: green at e2d01121
A27: green at e2d01121
A28: green at e2d01121
A29: green at e2d01121
A30: green at 4b154fc6
A31: green at 4b154fc6
A32: green at 4b154fc6
A33: green at 4b154fc6
A34: green at 4b154fc6
A35: green at e2d01121
A36: green at e2d01121
```

## Disclosures

- DISCLOSE: A26's red is the stub refusing a blank origin at start, where the SPEC's column reads
  "the stub serves the routes". Its test asserts that an unset or blank origin turns linking off,
  and the stub refused the blank instead, so the test failed on that assertion first.
- DISCLOSE: A34's red is an empty methods list, where the SPEC's column reads "the stub lists the
  credential id". The stub's list answered no method at all, so the test failed on Telegram's
  missing row before it reached the credential-id assertion.
- DISCLOSE: A30's red has two causes. The stub's `Passkeys::remove` discarded its session, so a
  DELETE /api/identities/1 with no session reached the delete, matched no row and refused
  `identity_unknown`; and the stub's `Refusal::status` answered 400 for that refusal. The SPEC's
  column reads "the stub's routes admit an owner session only".
- DISCLOSE: A29 and A33 were added at 56a948d, red before any code. A29's body grew at 83b0290 and
  A33's at 5d6d124, and each red line above is quoted at the commit where its body grew, so it
  quotes the grown body. Neither body changed after its red commit.
- DISCLOSE: A29 is green with every call into the passkey library's ceremonies run under a
  subscriber that records nothing (`quietly` in `crates/identity/src/passkeys.rs`), because the
  library's core traces ceremony state and credential ids at debug and trace level. ADR-370
  records the decision and what it was chosen against.
- DISCLOSE: three red commits edit lines of an earlier commit's test file beside the tests they
  add, each between another criterion's red and its green. 786000e rewrites one doc-comment line of
  `crates/identity/tests/support/mod.rs` to name the software authenticator and the fixture it
  adds. 83b0290 moves `crates/identity/tests/linking.rs`'s imports and shared constants into that
  fixture and grows A29's body. 5d6d124 rebuilds `crates/api/tests/linking_routes.rs`'s `app`
  helper over the linking state and grows A33's body. None of them removes an assertion.
- DISCLOSE: 4165549 grows the two expected Debug tails of `crates/api/tests/insights_routes.rs`'s
  `the_state_debug_line_says_which_ports_it_holds` by `, linking: false`, insert-only within each
  line, because `ApiState`'s Debug names the linking port this delivery adds. No assertion is
  removed or loosened.
- DISCLOSE: 73d9104 grows `crates/coordination/tests/data_rights_symmetry.rs`'s `SEEDS` by one
  seed for the `passkeys` table, which moves the array's length from 43 to 44. No seed is removed.
- DISCLOSE: c93f9ba respells lines of four test files for clippy's pedantic lints, every value
  unchanged. The clock advances `Duration::from_mins(5)` where it advanced
  `Duration::from_secs(300)` (twice in `crates/identity/tests/linking.rs`, once in
  `crates/identity/tests/passkeys.rs`). The CBOR writer in `crates/identity/tests/support/mod.rs`
  spells its heads `0x18`, `0x19` and `0x1a` where it spelt 24, 25 and 26. Three doc comments put
  backticks round a name (`WebAuthn` in that file, `SQLite` twice in
  `crates/api/tests/linking_routes.rs`). A29's test function gains
  `#[allow(clippy::too_many_lines, reason = ...)]` above it, its body unchanged. No assertion is
  removed or loosened, and no line a red above quotes changes.
- DISCLOSE: a212650 grows `crates/coordination/tests/relight_order.rs`'s `STATICS` by one entry,
  the data-rights registry's `IDENTITY` static that 73d9104 adds, which moves the array's length
  from 19 to 20. The census reads every static in the source of every crate coordination links,
  and the new static is a registry entry like the others, not a count per day. No entry is removed.
- DISCLOSE: f13e5e0 pins A1's code length and A5's mint and live counts by literal (`16`, `0..9`,
  `8`) in `crates/identity/tests/linking.rs`, where the tests read `LINK_CODE_BYTES` and
  `MAX_LIVE_LINK_CODES`, the two constants rows S35901 and S35904 mutate: a test whose expected
  value moves with its mutant cannot fail, and both rows survived before it. Every value is
  unchanged, no assertion is removed or loosened, and no line a red above quotes changes.
- DISCLOSE: each green above names the commit where its criterion first went green, and later
  commits change code and tests those commits ran: faf2aef respells source (durations in minutes,
  routes that borrow their access, an `if let` where a `match` stood) with no behaviour changed,
  c93f9ba and f13e5e0 respell tests with every value unchanged, and 73d9104, 4165549 and a212650
  are disclosed above. So the pushed head is not byte-identical to the green commits. The builder's
  runs before the push measure every criterion green on the pushed head's code and tests, and CI's
  run at the pushed head is the verdict.

# SPEC-359: passkey sign-in on the web, linked to the owner's Telegram account, and every doubtful assertion refused

- **Issue:** #627 (the campaign's passkey sign-in, web first: SPEC-334 R19 and its stretch row
  2.1, a passkeys-only slice of #58). **Context(s):** `deck-streak-identity` (the link code, a
  session's proof, the ceremonies and their refusals, the counter, the `passkeys` table and its
  data-rights port); `deck-streak-api` (the linking routes, the ceremony cookie, their bound);
  `deck-streak-coordination` (identity's port in the data-rights registry); `deck-streak-daemon`
  (the api role's wiring); the web client (`web/app`: the link page, the sign-in page, the
  sign-in methods screen).
- **Decided by:** ADR-370 (this SPEC's: the ceremony carrier, the counter's compare-and-swap,
  the sign-in ceremony's shape, the bounds, removal ending its sessions, the methods screen),
  ADR-132 (passkeys through `webauthn-rs`, the web client's host as the relying party), ADR-006
  (Telegram first, pinned to the owner, other methods linked) and ADR-024 (the in-memory session
  store and its bounds).
- **Delivers from SPEC-131:** R1's origin and `linking_off` for the passkey routes, R3 to R5,
  R10, R11, R15 and R16 for passkeys, R17's sign-in page, R18 and R19 for `passkeys`, and R20,
  under SPEC-131's own test names. SPEC-131 keeps the rest (§5).
- **Mutation band:** `S35900-S35999` (fragment `S35900-S35999.json`).
- **Status:** built in two pull requests. Part a (the server) owns §3's fence. Part b (the web
  screens) is §7, and its delivery moves §7's rows into the fence by amendment.

## 1. The problem, measured

Read at `3fe90969b3927cac1c3630c83a582f9d9c796d66` (`D` below); the tip it was compared with
changed no identity, api or web-client path.

| figure | value | command |
|---|---|---|
| identity's source files | 5: `init_data.rs`, `lib.rs`, `owner.rs`, `session.rs`, `settings.rs` | `git ls-tree --name-only $D crates/identity/src/` |
| files naming WebAuthn or a passkey in code, the manifests, the lock or the migrations | 0 | `git grep -i -c -E 'webauthn\|passkey' $D -- crates web/app/src Cargo.toml Cargo.lock migrations ':(exclude)*drill*' \| wc -l` |
| fields of a live session | 4: digest, owner, began, seen; no proof | `git grep -n -A5 'struct Live' $D -- crates/identity/src/session.rs` (lines 117-121) |
| session bounds | idle 30 min, absolute 8 h, 8 live, cookie `__Host-deckstreak_session` | `git grep -n -E 'SESSION_COOKIE\|IDLE_TIMEOUT:\|ABSOLUTE_LIFETIME:\|MAX_LIVE_SESSIONS:' $D -- crates/identity/src/session.rs` (lines 31-37) |
| the handshake's bound | 30 per minute, per process | `git grep -n 'HANDSHAKES_PER_MINUTE: u32' $D -- crates/api/src/session_routes.rs` (line 43) |
| the web client's way in outside Telegram | none: with no launch data the handshake answers `refused` and every call ends `reopen` | `git grep -n -E "return 'refused'\|return 'reopen'" $D -- web/app/src/lib/api.ts` (lines 115, 144, 150, 167) |
| screens the web client routes | 15 | `git show $D:web/app/src/lib/routes.ts \| grep -c -E "^\s+'/"` |
| Permissions-Policy declared by the edge or the page | 0, so WebAuthn's default allowlist (`self`) holds at top level | `git grep -i -c permissions-policy $D -- deploy/caddy web/app/svelte.config.js \| wc -l` |
| the page's `connect-src` | `'self'` only | `git grep -n connect-src $D -- web/app/svelte.config.js` (line 30) |
| migrations | 30, none creating a passkey table | `git ls-tree --name-only $D migrations/ \| wc -l` |
| SPEC-131's scope | 20 requirements and 58 criteria, of which this SPEC takes 26 (23 in §3, 3 in §7; A40 and A42 for `passkeys` only), by the map in each row | `git grep -c -E '^R[0-9]+\. ' $D -- 'docs/specs/planned/SPEC-131-*'` and `git grep -c -E '^\| A[0-9]+ \|' $D -- 'docs/specs/planned/SPEC-131-*'` |

- **A browser outside Telegram cannot sign in.** The web client and the Mini App are one
  application on one origin; outside Telegram it has no launch data, so it can only ask the owner
  to reopen it in Telegram.
- **The groundwork is decided and unbuilt.** SPEC-131 (planned) and ADR-132 fix the relying
  party, user verification, attestation, the user handle, the challenge's life and the refusal
  names. What they leave open is §2's ceremony carrier, the counter's atomicity, the sign-in
  ceremony's shape, the bounds and removal; ADR-370 decides each.
- **A passkey ceremony cannot run inside Telegram.** Telegram's web client frames the Mini App
  and grants the frame no WebAuthn permission, and the native Telegram apps run it in an in-app
  view. So a passkey is created in the system browser, reached from the Mini App by SPEC-131's
  link code (R3, R4), and used there.

## 2. Requirements

Configuration

R1. `DECKSTREAK_PUBLIC_ORIGIN`, the web client's `https` origin, is configuration held on the
    private deploy rail and is unset in `.env.example`. Unset or empty, every route of this SPEC
    answers 404 `linking_off` and the Telegram handshake is unchanged. A value that is not an
    `https` origin (a scheme, a host, an optional port, nothing else) refuses start by name. The
    relying party id is its host and the expected origin is that origin exactly: one entry, never
    the request's `Origin` header.
R16. At deploy, confirm every shared library of the server binary resolves on the target (`ldd` shows no `not found`), and fail the deploy if one does not.

Sessions

R2. A session records its proof: `telegram` (with its handshake instant), `link`, or `linked`
    (with the `passkeys` row that opened it). `OwnerSession` admits `telegram` and `linked`. A
    `link` session lives 600 seconds from its redeem and is admitted by this SPEC's linking
    routes alone. A `linked` session keeps ADR-024's idle and absolute bounds, never mints a link
    code and never removes a method.

The link (SPEC-131 R3, R4)

R3. POST /api/link/code mints a link code for a session whose proof is `telegram` and whose
    handshake is at most 300 seconds old; any other session is refused 401 `reauth_required`. A
    code is 16 bytes from the operating system's generator, answered once as base64url and kept
    only as its SHA-256 in memory. It lives 600 seconds (599 accepted, 600 refused
    `link_code_expired`) and redeems once (a second redeem, or an unknown code,
    `link_code_invalid`). At most 8 codes are live; a ninth evicts the oldest.
R4. The Mini App opens the link page with the code in the URL fragment, which no server log sees.
    The page posts it in a JSON body to POST /api/link/redeem, which ends the request's session,
    if any, and opens one with proof `link`. The code never travels in a path or a query.

Passkeys (ADR-132, SPEC-131 R10, R11)

R5. Registration runs only inside a `link` session. POST /api/passkeys/register/start asks for
    user verification `required` and attestation `none`, names the owner's registered credential
    ids as excluded, and uses as the user handle the one the owner's passkeys already hold, or a
    fresh random version-4 UUID when none is held. The user name and display name are fixed text
    that carries no personal data. POST /api/passkeys/register/finish verifies against R1's
    relying party and origin and the kept state, then inserts one `passkeys` row for the
    configured owner's Telegram user id. A credential id already present is refused 409
    `already_linked` and writes nothing. No route creates an account.
R6. POST /api/passkeys/sign-in/start needs no session. When the owner holds no passkey it answers
    401 `not_linked` and starts no ceremony; otherwise it starts one over the owner's passkeys.
    POST /api/passkeys/sign-in/finish refuses an unknown credential `not_linked` and a row whose
    Telegram user id is not the configured owner's 403 `not_owner`, applies R8, and on success
    stores the counter, the backup state, the updated credential and the instant of use, ends the
    session the request arrived with, and opens a new one with proof `linked`. Its body names the
    owner's accepted credential ids and user handle, so the page can signal them (§7).
R7. A ceremony's state stays on the server, in memory, under a 32-byte flow id from the operating
    system's generator, kept as its SHA-256. The id reaches the browser only in the cookie
    `__Host-deckstreak_ceremony` (`Path=/`, `Secure`, `HttpOnly`, `SameSite=Strict`,
    `Max-Age=300`), and a finish clears it. A ceremony lives 300 seconds (299 accepted, 300
    refused `challenge_expired`) and is taken out of the store before verification, so it is used
    at most once. A registration ceremony records the `link` session that started it and
    finishes in no other. At most 8 ceremonies are live; a ninth evicts the oldest.
R8. A registration or an assertion is refused, with nothing written: an unknown, reused, evicted
    or foreign-session ceremony, `challenge_invalid`; an expired one, `challenge_expired`; another
    origin or relying party, `origin_mismatch`; no user verification, `uv_required`; a signature
    or a response that does not verify, `passkey_invalid`. The counter rule: an assertion is
    accepted when the presented and stored counters are both zero or the presented one is
    greater; otherwise `counter_regressed`. The stored counter changes only by a compare-and-swap
    on the value the rule read, so two assertions carrying one counter cannot both pass. Each of
    these refusals answers 401; `not_owner` 403, `already_linked` and `last_method` 409,
    `linking_off` and `identity_unknown` 404, each as SPEC-024's `{"reason": ...}` body.

The owner's methods (SPEC-131 R15)

R9. GET /api/identities (`OwnerSession`) lists Telegram as the primary method, then each passkey
    with its row id, its creation and its last use; never a credential id, a public key or the
    user handle. DELETE /api/identities/{id} needs a `telegram` session at most 300 seconds old
    (`reauth_required`), refuses the Telegram method `last_method`, answers an unknown id
    `identity_unknown`, deletes the row, and ends every `linked` session that row opened.

Bounds, storage, rights and logs

R10. Every state-changing route of this SPEC keeps SPEC-024 R9's cross-site bound. The two
     ceremony starts and the redeem share one bound of 30 per minute per process, in a window of
     their own apart from the handshake's: the 31st is refused 429 `too_many_ceremonies` with
     `Retry-After`. The cross-site bound is checked first, then this bound, both before the body
     is read.
R11. Identity owns `passkeys`, created `STRICT` by `migrations/035901_identity_passkeys.sql`:
     an integer row id, `telegram_user_id`, `credential_id` (unique), `user_handle` (16 bytes),
     the serialized credential, `counter` (non-negative), `backup_state` (0 or 1), `created_at`
     and `last_used_at`.
R12. Identity's data-rights port declares `passkeys` exported whole and emptied by an erase, and
     coordination's registry holds it, so both erase paths (the bot's /delete and the data role's
     erase) empty it. `privacy.json` declares the category and `PRIVACY.md` gives its line.
R13. A link code, a flow id, a challenge, a session id, a cookie value, a credential id, a public
     key and the user handle never reach a log line, a span field, an error or a response body
     other than the one that mints them (SPEC-024 R4, SPEC-131 R19). A ceremony's `Debug` is
     redacted. The audit events name the event and the row id only.
R14. This SPEC opens no command with stakes (CHARTER 14) to a `linked` session: the API holds none
     of the four, and its one destructive route, removal, needs a fresh `telegram` proof.
R15. CHARTER 10's anti-goals bind this SPEC as one block; the ones it touches are no unbounded
     work (R3's and R7's caps and lives, R10's bound) and no secret on anything public (R1, R13).

## 3. Acceptance criteria

Each red is the assertion the criterion states, failing against the part a red-first commit's
stub: the module's types and functions with bodies that answer the wrong value (accept-all,
never-expire, keep-all), so the test compiles at that commit and fails on its assertion.

| id | criterion | decided by | the red it shows first |
|---|---|---|---|
| A1 | a link code is 16 random bytes, answered once and kept only as its hash (SPEC-131 A1) | `a_link_code_is_random_and_kept_hashed` | the stub's fixed code: two mints are equal and the store holds the clear code |
| A2 | a code redeemed at 599 seconds is accepted and at 600 refused `link_code_expired` (A2) | `a_link_code_expires_at_six_hundred_seconds` | the stub never expires: 600 is accepted |
| A3 | a second redeem, or an unknown code, is refused `link_code_invalid` (A3) | `a_link_code_redeems_once` | the stub redeems any code twice |
| A4 | a code is minted for a Telegram session of 300 seconds and refused `reauth_required` at 301 and for a `linked` session (A4) | `a_link_code_needs_a_fresh_telegram_session` | the stub mints for every session |
| A5 | a ninth live code evicts the oldest, which is refused `link_code_invalid`, and the eighth still redeems (A57) | `a_ninth_link_code_evicts_the_oldest` | the stub keeps nine |
| A6 | a `link` session is never admitted as an owner session, and it is refused by the linking routes 600 seconds after its redeem and not at 599 (A53) | `a_link_session_is_not_an_owner_session` | the stub admits every proof |
| A7 | a `linked` session is admitted as an owner session and mints no link code | `a_linked_session_is_an_owner_session_that_mints_nothing` | the stub admits only `telegram` |
| A8 | a passkey from another origin is refused `origin_mismatch`, at registration and at sign-in (A27) | `a_passkey_from_another_origin_is_refused` | the stub accepts every origin |
| A9 | a ceremony without user verification is refused `uv_required` (A28) | `a_passkey_without_user_verification_is_refused` | the stub accepts an unverified response |
| A10 | an unknown or reused ceremony is refused `challenge_invalid` (A29) | `a_reused_passkey_challenge_is_refused` | the stub keeps the ceremony after use |
| A11 | a ceremony at 300 seconds is refused `challenge_expired`, and at 299 is not (A30) | `a_passkey_challenge_expires_at_three_hundred_seconds` | the stub never expires |
| A12 | an assertion whose counter does not advance is refused `counter_regressed` and the stored row is unchanged (A31) | `a_regressed_counter_is_refused` | the stub accepts every counter |
| A13 | a sign-in stores the new counter, backup state and instant of use (A32) | `a_passkey_sign_in_stores_its_counter` | the stub writes nothing: the row keeps its seeded counter |
| A14 | the user handle is a random version-4 UUID, reused by a second registration, and the display name carries no personal data (A33) | `the_user_handle_carries_no_personal_data` | the stub mints a fresh handle each time |
| A15 | the counter rule accepts both-zero and an advance, and refuses equal, lower and zero-after-non-zero, over its table | `the_counter_rule_accepts_only_an_advance_or_two_zeros` | the stub's rule answers true |
| A16 | a ninth live ceremony evicts the oldest, which is refused `challenge_invalid` | `a_ninth_ceremony_evicts_the_oldest` | the stub keeps nine |
| A17 | a registration ceremony finished in another session is refused `challenge_invalid` | `a_ceremony_finishes_only_in_its_own_session` | the stub ignores the session |
| A18 | a response whose signature does not verify is refused `passkey_invalid` | `a_passkey_with_a_foreign_signature_is_refused` | the stub refuses it as an invalid challenge |
| A19 | registering a credential already present is refused `already_linked` and writes nothing (A24) | `linking_twice_is_refused` | the stub inserts a second row |
| A20 | a sign-in with no passkey held, or an unknown credential, is refused `not_linked` and writes nothing (A21) | `a_sign_in_with_an_unlinked_identity_creates_nothing` | the stub opens a session |
| A21 | a passkey row of another Telegram user is refused `not_owner` (A22) | `a_linked_identity_of_another_user_is_refused` | the stub skips the owner check |
| A22 | a sign-in opens a new `linked` session and ends the one it arrived with (A23) | `a_sign_in_rotates_the_session` | the stub keeps the arriving session |
| A23 | removing the Telegram method is refused `last_method` (A25) | `unlinking_telegram_is_refused` | the stub removes it |
| A24 | removing a passkey needs a Telegram session of at most 300 seconds (A26) | `unlinking_needs_a_fresh_telegram_session` | the stub removes from a `linked` session |
| A25 | removing a passkey ends the `linked` sessions it opened and no other session | `removing_a_passkey_ends_its_sessions` | the stub ends nothing |
| A26 | an unset public origin turns every route of this SPEC to `linking_off` (A20) | `linking_is_off_without_the_public_origin` | the stub serves the routes |
| A27 | a public origin that is not an `https` origin refuses start by name | `a_public_origin_that_is_not_https_refuses_start` | the stub accepts any value |
| A28 | `passkeys` is exported whole and emptied by an erase, symmetrically (A40 for `passkeys`) | `passkeys_export_and_erase_are_symmetric` | the stub's port declares no table |
| A29 | no secret of R13 reaches a log line or an error, beside a positive control that the capture saw each audit event, and a ceremony's `Debug` rendering is exactly its redacted form (A42) | `no_linking_secret_reaches_a_log` | the stub emits no audit event, so the control finds none |
| A30 | the linking routes answer 401 without a session, and a `link` session reaches no other route (A44) | `a_link_session_reaches_only_the_linking_routes` | the stub's routes admit an owner session only |
| A31 | the ceremony cookie is `__Host-`, `Secure`, `HttpOnly`, `SameSite=Strict`, `Path=/`, lives 300 seconds, and a finish clears it | `the_ceremony_cookie_is_host_prefixed_strict_and_short_lived` | the stub sets no ceremony cookie |
| A32 | the 31st ceremony start or redeem in a minute is refused 429 `too_many_ceremonies` with `Retry-After`, and the handshake still answers | `a_ceremony_flood_is_bounded_and_spares_the_handshake` | the stub's bound admits every request |
| A33 | every state-changing route of this SPEC refuses a cross-site request 403 before its body is read, over an examined count of its routes | `every_linking_route_refuses_a_cross_site_request` | the stub's routes skip the bound |
| A34 | the methods list names Telegram first and each passkey's row id and instants, and no credential id, key or handle | `the_identities_list_names_methods_without_credential_ids` | the stub lists the credential id |
| A35 | a counter that moved between the rule's read and the write is refused `counter_regressed`, and the row keeps the newer counter | `a_counter_moved_since_its_read_is_refused` | the stub writes without the compare-and-swap |
| A36 | each refusal of R8 and R9 answers its status, by a literal table of every reason code | `each_refusal_answers_its_status` | the stub answers 400 for every refusal |

```acceptance
A1: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_is_random_and_kept_hashed
A2: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_expires_at_six_hundred_seconds
A3: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_redeems_once
A4: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_needs_a_fresh_telegram_session
A5: cargo test -p deck-streak-identity --test linking -- --exact a_ninth_link_code_evicts_the_oldest
A6: cargo test -p deck-streak-identity --test linking -- --exact a_link_session_is_not_an_owner_session
A7: cargo test -p deck-streak-identity --test linking -- --exact a_linked_session_is_an_owner_session_that_mints_nothing
A8: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_from_another_origin_is_refused
A9: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_without_user_verification_is_refused
A10: cargo test -p deck-streak-identity --test passkeys -- --exact a_reused_passkey_challenge_is_refused
A11: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_challenge_expires_at_three_hundred_seconds
A12: cargo test -p deck-streak-identity --test passkeys -- --exact a_regressed_counter_is_refused
A13: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_sign_in_stores_its_counter
A14: cargo test -p deck-streak-identity --test passkeys -- --exact the_user_handle_carries_no_personal_data
A15: cargo test -p deck-streak-identity --test passkeys -- --exact the_counter_rule_accepts_only_an_advance_or_two_zeros
A16: cargo test -p deck-streak-identity --test passkeys -- --exact a_ninth_ceremony_evicts_the_oldest
A17: cargo test -p deck-streak-identity --test passkeys -- --exact a_ceremony_finishes_only_in_its_own_session
A18: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_with_a_foreign_signature_is_refused
A19: cargo test -p deck-streak-identity --test linking -- --exact linking_twice_is_refused
A20: cargo test -p deck-streak-identity --test linking -- --exact a_sign_in_with_an_unlinked_identity_creates_nothing
A21: cargo test -p deck-streak-identity --test linking -- --exact a_linked_identity_of_another_user_is_refused
A22: cargo test -p deck-streak-identity --test linking -- --exact a_sign_in_rotates_the_session
A23: cargo test -p deck-streak-identity --test linking -- --exact unlinking_telegram_is_refused
A24: cargo test -p deck-streak-identity --test linking -- --exact unlinking_needs_a_fresh_telegram_session
A25: cargo test -p deck-streak-identity --test linking -- --exact removing_a_passkey_ends_its_sessions
A26: cargo test -p deck-streak-identity --test linking_config -- --exact linking_is_off_without_the_public_origin
A27: cargo test -p deck-streak-identity --test linking_config -- --exact a_public_origin_that_is_not_https_refuses_start
A28: cargo test -p deck-streak-identity --test rights -- --exact passkeys_export_and_erase_are_symmetric
A29: cargo test -p deck-streak-identity --test linking -- --exact no_linking_secret_reaches_a_log
A30: cargo test -p deck-streak-api --test linking_routes -- --exact a_link_session_reaches_only_the_linking_routes
A31: cargo test -p deck-streak-api --test linking_routes -- --exact the_ceremony_cookie_is_host_prefixed_strict_and_short_lived
A32: cargo test -p deck-streak-api --test linking_routes -- --exact a_ceremony_flood_is_bounded_and_spares_the_handshake
A33: cargo test -p deck-streak-api --test linking_routes -- --exact every_linking_route_refuses_a_cross_site_request
A34: cargo test -p deck-streak-api --test linking_routes -- --exact the_identities_list_names_methods_without_credential_ids
A35: cargo test -p deck-streak-identity --test passkeys -- --exact a_counter_moved_since_its_read_is_refused
A36: cargo test -p deck-streak-identity --test linking -- --exact each_refusal_answers_its_status
```

## 3a. Proved on a device, not in CI

| id | criterion | proved by, and when |
|---|---|---|
| D1 | on the deployed origin, a real platform authenticator registers through the link page opened from the Mini App, then signs in at the sign-in page, and a removal from the Mini App ends that browser's session | the owner, in the campaign's acceptance session (#637), after part b is deployed with the origin set |

## 3b. Box rows (judged by the box's packs, never by CI)

| id | over | judged by |
|---|---|---|
| B1 | `crates/identity/src/passkeys.rs`, `crates/identity/src/linking.rs` and the migration: a fixed relying party and origin, server-kept state, required user verification, a random handle, no attestation, `update_credential` called, a binding time | the auth pack |
| B2 | `crates/api/src/linking_routes.rs` and `crates/identity/src/session.rs`: the cookies' prefix and flags, the cross-site bound on every state-changing route, the rate bound | the web-security pack |
| B3 | `privacy.json`, `PRIVACY.md` and `crates/identity/src/data_rights.rs`: the category declared, exported and erased | the privacy-gdpr pack |

## 4. File manifest

Part a:

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-359-passkey-sign-in-on-the-web.md` | docs | added: this SPEC |
| `crates/identity/src/linking.rs` | `deck-streak-identity` | added: the link code, the redeem, the methods list and removal, the owner check at sign-in |
| `crates/identity/src/passkeys.rs` | `deck-streak-identity` | added: the relying party, the ceremonies and their store, the refusals, the counter rule, the `passkeys` store |
| `crates/identity/src/linking_config.rs` | `deck-streak-identity` | added: the public origin and its refusal |
| `crates/identity/src/session.rs` | `deck-streak-identity` | changed: a session's proof, its admission by proof, ending the sessions a passkey opened |
| `crates/identity/src/data_rights.rs` | `deck-streak-identity` | added: `passkeys` exported and erased |
| `crates/identity/src/lib.rs` | `deck-streak-identity` | changed: the modules and the refusal codes |
| `crates/identity/Cargo.toml` | `deck-streak-identity` | changed: `webauthn-rs`, `uuid`, `sqlx` (ADR-132); the software authenticator as a dev-dependency |
| `crates/identity/tests/linking.rs` | `deck-streak-identity` | added: A1 to A7, A19 to A25, A29, A36 |
| `crates/identity/tests/passkeys.rs` | `deck-streak-identity` | added: A8 to A18, A35 |
| `crates/identity/tests/linking_config.rs` | `deck-streak-identity` | added: A26, A27 |
| `crates/identity/tests/rights.rs` | `deck-streak-identity` | added: A28 |
| `crates/identity/tests/support/mod.rs` | `deck-streak-identity` | added: the software authenticator, the fixed clock, the log capture |
| `migrations/035901_identity_passkeys.sql` | `deck-streak-identity` | added: `passkeys` |
| `crates/api/src/linking_routes.rs` | `deck-streak-api` | added: the link, ceremony and methods routes, the ceremony cookie, the ceremony bound |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes joined |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/linking_routes.rs` | `deck-streak-api` | added: A30 to A34 |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: identity's port registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded `passkeys` rows |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role reads the public origin and builds the relying party |
| `.env.example`, `deploy/deck-streak.env.example` | repo, deploy | changed: the public origin's line, unset, with no example and named by its role |
| `privacy.json`, `PRIVACY.md` | repo, docs | changed: the `passkeys` category and its line |
| `docs/CONTEXT-MAP.md` | docs | changed: identity's own-tables row |
| `Cargo.toml`, `Cargo.lock`, `.sqlx/` | workspace | changed: the new crates and the offline query cache |
| `deny.toml` | workspace | unchanged: `MPL-2.0` is already allowed |
| `docs/specs/planned/SPEC-131-passkeys-google-and-apple-link-to-the-owners-telegram-account-and-every-doubtful-assertion-is-refused.md` | docs | changed: a status note naming what this SPEC delivers |
| `docs/decisions/ADR-370-passkey-sign-in-on-the-web.md`, `docs/schematics/passkey-sign-in-on-the-web.md` | docs | added |
| `docs/red-first/SPEC-359.md` | docs | added |
| `scripts/mutation-rows.d/S35900-S35999.json` | repo | added: the band's rows |
| `scripts/mutation-equivalent.d/deck-streak-identity.json` | repo | changed: two records of mutants that cannot differ from the original |
| `docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | docs | changed: section 7's identity row counts the two records (145 listed, 5 equivalent) |
| `formal/tla/PasskeyOnce/PasskeyOnce.tla`, `formal/tla/PasskeyOnce/MCPasskeyOnce.cfg`, `formal/tla/PasskeyOnce/witness/a-take-that-keeps-its-entry.cfg`, `formal/tla/PasskeyOnce/witness/a-counter-read-then-written-in-two-steps.cfg` | formal | added: the model of a ceremony or link code taken once and the counter advanced by one compare-and-swap, with a witness per property |
| `changelog.d/passkey-sign-in-359.md` | repo | added |

`ApiState::with_linking(config, owner)` in `crates/api/src/router.rs` takes the `Owner` beside the
`LinkingConfig`: the owner's access exposes no owner, and `crates/api/src/session_routes.rs` is not
in this manifest.

Part b is §7's manifest.

## 5. What this does NOT do

- It adds no OpenID Connect sign-in, no other linked provider and no provider-token revocation;
  SPEC-131 keeps them (#58).
- It registers no provider client and binds no provider credential (#347).
- It adds no native passkey sign-in, no associated-domains file and no native bearer token in the
  Keychain: those are the native half of #627, built after this web half (#627).
- It adds no discoverable-credential or conditional-UI sign-in, and runs no ceremony inside
  Telegram's frame (#627).
- It adds no section to the settings screen; the sign-in methods screen stands alone until the
  settings screen lands and embeds it (#57).
- It changes none of the Mini App handshake's checks, which stay the Telegram way in and the
  recovery path when every passkey is lost (#17).
- It sets no value for the public origin; the private deploy rail holds it, and until it does every
  route of this SPEC answers `linking_off` (#627).
- It retires no Telegram surface (#63).
- It sends no bot notice when a passkey is bound; whether the campaign needs one is the threat
  model's question (#653).
- It opens no command with stakes on the web; the shipped surfaces' re-check judges any later one
  (#638).

## 6. Risks

- **A forged, replayed or cloned assertion admitted.** R7 and R8 refuse each shape by name, and
  the compare-and-swap closes the race between two assertions; detected by A8 to A18 and A35.
- **A lost authenticator keeps a session.** Removal ends the sessions it opened; detected by A25.
- **The sign-in start names the owner's credential ids to an unauthenticated caller** (ADR-370).
  They are random, scoped to this relying party and carry no personal data, and R10 bounds the
  route; detected by A32.
- **A ceremony lost to a restart.** State is in memory, so a ceremony in flight across a restart
  is refused `challenge_invalid` and the owner starts again; detected by A10.
- **SPEC-131's later delivery recreating `passkeys`.** Its planned migration names the table too;
  the status note this SPEC adds to SPEC-131 hands the table over, and CI's migrations refuse a
  second `CREATE TABLE passkeys`.
- **A later Permissions-Policy that omits WebAuthn.** The edge sets none today, so the default
  `self` allowlist holds; a policy that names neither `publickey-credentials-create` nor
  `publickey-credentials-get` for `self` would stop every ceremony; detected by D1.
- **OpenSSL in the build.** `webauthn-rs` links it (ADR-132); the dependency audit covers it.

## 7. Delivered by the next pull request (part b: the web screens)

| id | criterion | delivered by |
|---|---|---|
| A37 | the link page reads the code from the fragment, posts it in a body, never in a URL, and clears it from the address once redeemed (SPEC-131 A45) | `pnpm exec vitest run web/app/src/routes/link.test.ts -t "the link page posts the code from the fragment"`; red: the stub page posts nothing |
| A38 | outside Telegram with no session, the shell offers the sign-in page, with a passkey when the browser offers WebAuthn (A47) | `pnpm exec vitest run web/app/src/routes/signin.test.ts -t "outside telegram the shell offers sign-in"`; red: the shell answers `reopen` |
| A39 | the sign-in methods screen lists the methods and offers no removal for Telegram (A46) | `pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "the sign-in section offers no unlink for telegram"`; red: the stub screen offers removal on every row |
| A40 | inside Telegram, "Link a passkey" mints a code and opens the link page through the Mini App's link opener, and no ceremony runs in the frame | `pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "link a passkey opens the link page in the browser"`; red: the stub runs the ceremony in place |
| A41 | outside Telegram, a call refused 401 asks for sign-in instead of a reopen | `pnpm exec vitest run web/app/src/lib/api.test.ts -t "outside telegram a refused call asks for sign-in"`; red: the call answers `reopen` |
| A42 | ceremony options and responses round-trip through base64url | `pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "ceremony options and responses round-trip through base64url"`; red: the stub returns its input unconverted |
| A43 | after a passkey sign-in the page signals the accepted credentials where the browser offers it, and calls nothing where it does not | `pnpm exec vitest run web/app/src/lib/passkeys.test.ts -t "a sign-in signals the accepted credentials only where the browser offers it"`; red: the stub never signals |
| A44 | where the browser offers no WebAuthn, the link page redeems nothing and asks the owner to open it in the browser, keeping the code unspent | `pnpm exec vitest run web/app/src/routes/link.test.ts -t "without webauthn the link page keeps the code unspent"`; red: the stub redeems on load |
| A45 | At deploy, confirm every shared library of the server binary resolves on the target (`ldd` shows no `not found`), and fail the deploy if one does not. | the private deploy rail |

Part b's manifest: `web/app/src/routes/link/+page.svelte`, `web/app/src/routes/link.test.ts`,
`web/app/src/routes/signin/+page.svelte`, `web/app/src/routes/signin.test.ts`,
`web/app/src/routes/sign-in-methods/+page.svelte`, `web/app/src/routes/sign-in-methods.test.ts`,
`web/app/src/lib/settings/SignInMethods.svelte`, `web/app/src/lib/passkeys.ts`,
`web/app/src/lib/passkeys.test.ts`, `web/app/src/lib/api.ts`, `web/app/src/lib/api.test.ts`,
`web/app/src/lib/routes.ts`, `web/app/messages/*.json` (each locale's catalog),
`docs/red-first/SPEC-359.md` (its rows), `scripts/mutation-rows.d/S35900-S35999.json` (its rows), and a
`changelog.d/` fragment.

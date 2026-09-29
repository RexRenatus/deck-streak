# SPEC-131: passkeys, Google and Apple link to the owner's Telegram account, and every doubtful assertion is refused

- **Wave:** W7. **Issues:** #58 (linked sign-in with Google, Apple and passkeys) and #347 (the
  owner's action: register the Google and Apple sign-in clients) (epic #8). **Context(s):**
  `deck-streak-identity` (the link code, the ceremonies' state, the OpenID Connect and WebAuthn
  verifiers, the linked identities, the passkeys, Apple's sealed token and its revocation, every
  refusal); `deck-streak-coordination` (the erase's revocation step and the job `link_revocation`);
  `deck-streak-api` (the routes and the flow cookie); `deck-streak-bot` (/delete erases through the
  revocation step); `deck-streak-daemon` (the api, bot, data and job roles' loads and wiring); the
  Mini App (`web/app`: `/link`, `/signin` and the settings screen's section).
- **Decided by:** ADR-131 (this SPEC's: OpenID Connect through `openidconnect` over identity's own
  HTTP port), ADR-132 (this SPEC's: passkeys through `webauthn-rs`, with the Mini App's host as
  the relying party), ADR-133 (this SPEC's: Apple's refresh token is kept sealed and revoked on
  unlink and erase), ADR-006 (Telegram first, pinned to the owner, other methods linked), ADR-024
  (the in-memory session store) and ADR-130 (the settings screen).
- **Prerequisites:** SPEC-020 (settings and migrations), SPEC-021 (data rights and the erase),
  SPEC-024 (the handshake, the sessions and the CSRF bound), SPEC-027 (the job table and its page
  exit), SPEC-028 (the Mini App shell), SPEC-066 (the credential loader and its empty refusal) and
  SPEC-130 (the settings screen). SPEC-130 is unlanded. **Mutation band:** `S13100-S13199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-131.md` (ADR-016).

## 1. The problem, measured

- **Telegram is the only way in.** `crates/identity/src/` holds `init_data.rs`, `owner.rs`,
  `session.rs` and `settings.rs`: the `initData` handshake (SPEC-024), the owner pin and the
  in-memory sessions. A browser outside Telegram cannot sign in at all.
- **The owner's model.** ADR-006 chose Telegram first, pinned to the owner, with Google, Apple and
  passkeys linked to the same account and never a second one. #58 asks for those three, and that
  unlinking the last method is refused.
- **The providers need the owner.** Google's client and Apple's service identity and signing key
  are registered in the owner's accounts, on the owned domain (#168); #347 tracks that action.
  Until it is done no credential exists, so each provider must be off, and the design must hold
  with none.
- **A security class.** Every assertion that reaches this code is an attacker's input until it
  verifies: a callback with no state, a replayed flow, an ID token for another client, a passkey
  from another origin. The auth and web-security packs name the standards' MUSTs (RFC 9700, OpenID
  Connect Core, WebAuthn Level 3, NIST SP 800-63B-4).
- **The predecessor had no linked sign-in.** Its boundary was the bot's owner chat; there is nothing
  to port and no golden.

## 2. Requirements

Configuration and credentials

R1. `DECKSTREAK_PUBLIC_ORIGIN` (an `https` origin; `.env.example` gives `https://app.example.org`)
    is the relying party's origin and the redirect base. Unset or empty, linked sign-in is off: every
    route of this SPEC answers 404 `linking_off`, and Telegram's handshake is unchanged. A value that
    is not an `https` origin refuses start by name (SPEC-020's settings rule).
R2. Google's and Apple's public identifiers are configuration: `DECKSTREAK_GOOGLE_CLIENT_ID`,
    `DECKSTREAK_APPLE_CLIENT_ID`, `DECKSTREAK_APPLE_TEAM_ID` and `DECKSTREAK_APPLE_KEY_ID`, each
    unset in `.env.example`. Their secrets are credentials, read through the kernel's
    `CredentialLoader` (SPEC-066): `google-client-secret`, `apple-signing-key` and `link-token-key`.
    The api role reads all three; the bot and data roles, which erase, and the job
    `link_revocation` (R14) read the last two. The three are optional (ADR-131, ADR-133): a missing
    one, or no credentials directory, turns its provider off (`provider_off`) and leaves Apple's
    revocation to the queue (R14); an empty one refuses start with the loader's `Empty` refusal, by
    its id. No unit names any of the three until #347's delivery binds them.

The link (ADR-006)

R3. `POST /api/link/code` mints a link code for a session whose proof is `telegram` and whose
    handshake is at most 300 seconds old; any other session is refused `reauth_required`. A code is
    16 bytes from the operating system's generator (128 bits), sent once as base64url, and kept only
    as its SHA-256 in memory. It lives 600 seconds: redeemed at 599 it is accepted, at 600 refused
    `link_code_expired`. It redeems once; a second redeem, or an unknown code, is refused
    `link_code_invalid`. At most 8 codes are live, the oldest evicted.
R4. The Mini App opens `<origin>/link#code=<code>` through `openLink`, so the code travels in the
    fragment, which no server log sees. The page posts it to `POST /api/link/redeem`, which on
    success ends the request's session, if any, and starts a new one with proof `link`, living 600
    seconds and admitted only by this SPEC's linking routes.
R5. A link inserts one row into `linked_identities` or `passkeys` for the configured owner's
    Telegram user id. A `(issuer, subject)` or a credential id already present is refused
    `already_linked` and writes nothing. No route creates an account: the account is the owner's
    Telegram identity, and a link attaches to it or is refused.

OpenID Connect (ADR-131)

R6. `GET /api/auth/{provider}/start?purpose=link|sign_in&return_to=<path>` accepts `provider` from
    the closed set `google`, `apple` (anything else 404 `provider_unknown`), and `return_to` from the
    closed allow-list `/` and `/settings` (anything else 400 `redirect_refused`; absent means `/`).
    It keeps a flow in memory (provider, purpose, return target, state, nonce, PKCE verifier,
    instant) under a 32-byte flow id, sets `__Host-deckstreak_flow` (`Path=/`, `Secure`, `HttpOnly`,
    `SameSite=Lax`, `Max-Age=600`), and redirects to the provider's authorization endpoint with
    `response_type=code`, PKCE `S256`, the state, the nonce, and the exact redirect URI
    `<origin>/api/auth/{provider}/callback`. Google's request asks `scope=openid` alone; Apple's asks
    neither `name` nor `email` and `response_mode=query`, so both callbacks are a `GET` that carries
    the `SameSite=Lax` cookie. At most 8 flows are live; a ninth evicts the oldest, whose callback
    then meets R7 step 1 (`state_invalid`).
R7. The callback refuses, in this order, and each refusal consumes nothing but the flow:
    1. no flow cookie, or a flow id with no flow: `state_invalid`;
    2. a flow older than 600 seconds: `state_expired` (599 accepted);
    3. a `state` unequal to the flow's, compared in constant time: `state_invalid`;
    4. a callback at another provider's path than the flow's, or an `iss` parameter, when the
       response carries one, unequal to the flow's issuer (RFC 9207): `issuer_mismatch`;
    5. the code exchange (with the PKCE verifier and, for Apple, R12's client secret) answers no ID
       token: `token_missing`;
    6. the ID token's signature does not verify against the issuer's published keys:
       `token_invalid`;
    7. its `iss` is not the provider's issuer: `wrong_issuer`;
    8. its `aud` does not hold the client id, or it names another authorized party: `wrong_audience`;
    9. its `exp` has passed on the kernel's clock: `token_expired`;
    10. its `nonce` is not the flow's: `nonce_mismatch`.
    The flow is removed before the exchange, so a replayed callback meets step 1.
R8. A verified `link` flow runs R5 inside a `link` session; outside one it is refused
    `reauth_required`. A verified `sign_in` flow looks up `(iss, sub)`: no row is `not_linked`, and
    nothing is written. A row whose Telegram user id is not the configured owner's is `not_owner`.
    Otherwise the request's session, if any, ends and a new one starts with proof `linked`, and the
    response redirects to the flow's allow-listed target. The email claim is never read, so no
    identity is ever joined by email.
R9. The provider's HTTP (discovery, keys, the token exchange, Apple's revocation) goes through
    identity's `ProviderHttp` port. The daemon's wiring builds its one adapter over `reqwest` (the
    0.13 line the bot's client already brings, with rustls), redirects disabled and a 10-second
    bound per call, and hands it to each role that calls a provider: the api role, and the bot, data
    and job roles for Apple's revocation (ADR-131). The tests wire a double that serves a test
    issuer's keys and tokens. No test reaches a network.

Passkeys (ADR-132)

R10. The relying party is the host of `DECKSTREAK_PUBLIC_ORIGIN`, and the expected origin that
     origin exactly. Registration (inside a `link` session) asks for user verification `required`,
     attestation `none`, and a user handle that is a random version-4 UUID (16 bytes, the `Uuid` webauthn-rs 0.5's
     `start_passkey_registration` takes as `user_unique_id`), minted once for the owner and reused,
     with a fixed display name that carries no personal data. The ceremony's state is kept on the
     server under the flow id, never sent to the browser, and lives 300 seconds.
R11. A registration or an assertion refuses: an unknown or already used ceremony,
     `challenge_invalid`; one older than 300 seconds, `challenge_expired` (299 accepted); another
     origin or relying party, `origin_mismatch`; no user verification, `uv_required`; an unknown
     credential at sign-in, `not_linked`; a signature counter that does not advance past the stored
     one (when either is non-zero), `counter_regressed`. A verified assertion stores the new
     counter and backup state, then R8's owner check and session rotation apply with proof
     `linked`.

Apple (ADR-133)

R12. Apple's client secret is an ES256 JWT (`kid` the key id; `iss` the team id; `sub` the client
     id; `aud` Apple's issuer; `exp` 300 seconds after `iat`), signed with `apple-signing-key` for
     each exchange and never stored.
R13. Apple's refresh token from the exchange is sealed with XChaCha20-Poly1305 under
     `link-token-key`, with a random 24-byte nonce and `(issuer, token id)` as associated data, the token id 16 random
     bytes minted at the seal, and stored in `linked_identities.sealed_refresh_token` beside its
     `token_id`. No other token of any provider is kept.
R14. Unlinking Apple, and the erase (R18), call Apple's revocation with the opened token first,
     bounded at 10 seconds, then delete the local row either way. A revocation that fails, times
     out, or cannot be made because the role holds no Apple credential moves the sealed token to
     `identity_revocations` (the sealed token, its issuer and token id, its attempt count, its next attempt instant; no
     subject and no user id), due an hour later. The job `link_revocation` runs hourly at minute 41
     (SPEC-027 R1's hourly kind, a minute R2 admits) on the plain job template, and tries each due
     row once; a failure, or no credential, counts an attempt and doubles the wait (1, 2, 4, 8 and
     16 hours apart). A success deletes the row. The fifth failed attempt deletes the row and the
     job exits with the page code (SPEC-027 R7), so the template's one alert says that a withdrawal
     at Apple could not be completed. A queued row never blocks an unlink, a sign-in or an erase.

The owner's surface

R15. `GET /api/identities` (`OwnerSession`) lists Telegram as the primary method, then each linked
     method's provider, creation and last use. `DELETE /api/identities/{id}` needs a session with
     proof `telegram` at most 300 seconds old (`reauth_required`), refuses the Telegram method
     `last_method`, and runs R14 for Apple. Every state-changing route keeps SPEC-024 R9's CSRF
     bound.
R16. A session records its proof (`telegram` with its handshake instant, `link`, or `linked`).
     `OwnerSession` admits `telegram` and `linked`; a `link` session is admitted by the linking
     routes alone. A `linked` session never mints a link code and never unlinks.
R17. The settings screen (SPEC-130) gains a sign-in section: the methods, an unlink button per
     linked method, and "Link a method", which mints a code and opens the link page. Outside
     Telegram, the shell offers `/signin` (a passkey, and each provider that is on) where it
     would otherwise ask the owner to reopen the Mini App. Nothing is stored on the device.

Rights and rules

R18. Identity owns `linked_identities`, `passkeys` and `identity_revocations`, created `STRICT` by
     `migrations/013101_identity_linked_sign_in.sql`, each with `created_at`, `linked_identities`
     with `UNIQUE (issuer, subject)`. Each owes SPEC-021's six files (§4). An export lists every
     linked method and passkey with its provider, subject or credential id and instants, and says
     that an Apple token is held without its value. Both erase paths, the bot's /delete and the
     data role's erase, run coordination's `erase_with_revocation`: R14's revocation for every Apple
     link first, then the engine (`erase_all`), which empties `linked_identities` and `passkeys`.
     `identity_revocations` is exempt from export and erase (the kernel's `Disposition::Exempt`),
     because each row is the erase's own withdrawal at the provider and holds no subject or user
     id; R14 bounds its life at five attempts.
R19. The link code, the state, the nonce, the verifier, a token, a sealed token, a challenge and a
     session id never reach a log line, a span field, an error or a response body other than the
     one that mints them (SPEC-024 R4).
R20. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no
     unbounded work (R3 and R6's caps, R10's 300-second life, R9's bound, R14's five attempts) and no secret on
     anything public (R2, R19).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a link code is 16 random bytes, returned once and kept only as its hash | `a_link_code_is_random_and_kept_hashed` |
| A2 | a link code redeemed at 599 seconds is accepted and at 600 refused `link_code_expired` | `a_link_code_expires_at_six_hundred_seconds` |
| A3 | a second redeem, or an unknown code, is refused `link_code_invalid` | `a_link_code_redeems_once` |
| A4 | a code is minted for a Telegram session of 300 seconds and refused `reauth_required` at 301, and for a `linked` session | `a_link_code_needs_a_fresh_telegram_session` |
| A5 | a callback with no flow cookie, or an unknown flow, is refused `state_invalid` | `a_callback_without_its_flow_is_refused` |
| A6 | a callback whose state differs is refused `state_invalid` | `a_callback_with_another_state_is_refused` |
| A7 | a flow at 600 seconds is refused `state_expired`, and at 599 is not | `a_flow_expires_at_six_hundred_seconds` |
| A8 | a replayed callback is refused `state_invalid` | `a_replayed_callback_is_refused` |
| A9 | a callback for another provider, or with another `iss`, is refused `issuer_mismatch` | `a_mixed_up_callback_is_refused` |
| A10 | an exchange with no ID token is refused `token_missing` | `an_exchange_without_an_id_token_is_refused` |
| A11 | an ID token signed by another key is refused `token_invalid` | `an_id_token_with_a_foreign_signature_is_refused` |
| A12 | an ID token from another issuer is refused `wrong_issuer` | `an_id_token_from_another_issuer_is_refused` |
| A13 | an ID token for another audience is refused `wrong_audience` | `an_id_token_for_another_audience_is_refused` |
| A14 | an expired ID token is refused `token_expired` | `an_expired_id_token_is_refused` |
| A15 | an ID token with another nonce is refused `nonce_mismatch` | `an_id_token_with_another_nonce_is_refused` |
| A16 | the authorization request carries `S256`, the state, the nonce, `scope=openid` alone and the exact redirect URI | `the_authorization_request_is_pkce_s256_with_the_exact_redirect` |
| A17 | a return target outside the allow-list is refused `redirect_refused` | `a_return_target_outside_the_allow_list_is_refused` |
| A18 | an unknown provider is `provider_unknown`, and a provider without its credential is `provider_off` | `a_provider_is_off_without_its_credential` |
| A19 | an empty provider credential refuses start by its id | `an_empty_provider_credential_refuses_start` |
| A20 | an unset public origin turns every linking route to `linking_off` | `linking_is_off_without_the_public_origin` |
| A21 | a sign-in with an unlinked identity is refused `not_linked` and writes nothing | `a_sign_in_with_an_unlinked_identity_creates_nothing` |
| A22 | a linked identity of another Telegram user is refused `not_owner` | `a_linked_identity_of_another_user_is_refused` |
| A23 | a sign-in starts a new session and ends the one it arrived with | `a_sign_in_rotates_the_session` |
| A24 | linking an identity or a passkey already linked is refused `already_linked` and writes nothing | `linking_twice_is_refused` |
| A25 | unlinking the Telegram method is refused `last_method` | `unlinking_telegram_is_refused` |
| A26 | unlinking needs a Telegram session of at most 300 seconds | `unlinking_needs_a_fresh_telegram_session` |
| A27 | a passkey from another origin is refused `origin_mismatch` | `a_passkey_from_another_origin_is_refused` |
| A28 | a passkey ceremony without user verification is refused `uv_required` | `a_passkey_without_user_verification_is_refused` |
| A29 | an unknown or reused ceremony is refused `challenge_invalid` | `a_reused_passkey_challenge_is_refused` |
| A30 | a ceremony at 300 seconds is refused `challenge_expired`, and at 299 is not | `a_passkey_challenge_expires_at_three_hundred_seconds` |
| A31 | an assertion whose counter does not advance is refused `counter_regressed` | `a_regressed_counter_is_refused` |
| A32 | a passkey sign-in stores the new counter and backup state | `a_passkey_sign_in_stores_its_counter` |
| A33 | the user handle is a random version-4 UUID and the display name carries no personal data | `the_user_handle_carries_no_personal_data` |
| A34 | Apple's client secret is ES256 and expires 300 seconds after it is issued | `the_apple_client_secret_is_es256_for_three_hundred_seconds` |
| A35 | Apple's refresh token is stored sealed, bound to its issuer and token id, opens with the key alone, and does not open under another row's token id | `apples_refresh_token_is_stored_sealed` |
| A36 | unlinking Apple revokes before it deletes | `unlinking_apple_revokes_before_it_deletes` |
| A37 | a revocation that times out at 10 seconds is queued and the unlink completes; the test bounds its own wait at 11 seconds | `a_failed_revocation_is_queued_and_the_unlink_completes` |
| A38 | a queued revocation is tried five times, 1, 2, 4, 8 and 16 hours apart, and the fifth failure drops it and answers the page outcome | `a_queued_revocation_is_tried_five_times_then_dropped` |
| A39 | the erase revokes Apple first and completes when the revocation fails | `the_erase_revokes_apple_first_and_never_waits_on_a_failure` |
| A40 | linked sign-in's export and erase are symmetric, and `identity_revocations` is exempt with its reason | `linked_sign_in_export_and_erase_are_symmetric` |
| A41 | the export states that an Apple token is held and never holds its value | `the_export_never_holds_a_token` |
| A42 | no secret of R19 reaches a log line or an error | `no_linking_secret_reaches_a_log` |
| A43 | the flow cookie is `__Host-`, `Secure`, `HttpOnly`, `SameSite=Lax` and lives 600 seconds | `the_flow_cookie_is_host_prefixed_and_short_lived` |
| A44 | the linking routes answer 401 without a session, and a `link` session reaches no other route | `a_link_session_reaches_only_the_linking_routes` |
| A45 | the link page reads the code from the fragment and posts it in a body, never in a URL | `the link page posts the code from the fragment` |
| A46 | the settings section lists the methods and offers no unlink for Telegram | `the sign-in section offers no unlink for telegram` |
| A47 | outside Telegram with no session, the shell offers sign-in with the providers that are on | `outside telegram the shell offers sign-in` |
| A48 | an erase in a role without Apple's credentials queues each Apple token before it erases | `an_erase_without_the_apple_credential_queues_the_revocation` |
| A49 | the job table holds `link_revocation` hourly at minute 41, a minute SPEC-027 R2 admits | `the_link_revocation_job_is_hourly_at_a_free_minute` |
| A50 | the job role runs `link_revocation` with no credential and an empty queue, and exits 0 | `the_link_revocation_job_runs_without_a_credential` |
| A51 | /delete erases through the revocation step, only after the owner confirms | `delete_revokes_apple_links_before_it_erases` |
| A52 | the job's timer holds the table's calendar | `test_the_link_revocation_timer_holds_the_tables_minute` |
| A53 | a `link` session is never admitted as an owner session | `a_link_session_is_not_an_owner_session` |
| A54 | a revoker without Apple's credentials queues the sealed token and reports it queued | `a_revoker_without_the_credential_queues_the_token` |
| A55 | each role reads exactly R2's credentials: an empty `google-client-secret` refuses the api role's start and no other role's, and an empty `link-token-key` refuses the api, bot and data roles' start and the job `link_revocation`'s run | `each_role_reads_exactly_its_linking_credentials` |
| A56 | a queued revocation opens its sealed token without a subject, and Apple's acceptance deletes its row | `a_queued_revocation_opens_without_a_subject_and_is_deleted_on_success` |
| A57 | a ninth live link code evicts the oldest, which is refused `link_code_invalid`, and the eighth still redeems | `a_ninth_link_code_evicts_the_oldest` |
| A58 | a ninth live flow evicts the oldest | `a_ninth_flow_evicts_the_oldest` |

```acceptance
A1: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_is_random_and_kept_hashed
A2: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_expires_at_six_hundred_seconds
A3: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_redeems_once
A4: cargo test -p deck-streak-identity --test linking -- --exact a_link_code_needs_a_fresh_telegram_session
A5: cargo test -p deck-streak-identity --test oidc -- --exact a_callback_without_its_flow_is_refused
A6: cargo test -p deck-streak-identity --test oidc -- --exact a_callback_with_another_state_is_refused
A7: cargo test -p deck-streak-identity --test oidc -- --exact a_flow_expires_at_six_hundred_seconds
A8: cargo test -p deck-streak-identity --test oidc -- --exact a_replayed_callback_is_refused
A9: cargo test -p deck-streak-identity --test oidc -- --exact a_mixed_up_callback_is_refused
A10: cargo test -p deck-streak-identity --test oidc -- --exact an_exchange_without_an_id_token_is_refused
A11: cargo test -p deck-streak-identity --test oidc -- --exact an_id_token_with_a_foreign_signature_is_refused
A12: cargo test -p deck-streak-identity --test oidc -- --exact an_id_token_from_another_issuer_is_refused
A13: cargo test -p deck-streak-identity --test oidc -- --exact an_id_token_for_another_audience_is_refused
A14: cargo test -p deck-streak-identity --test oidc -- --exact an_expired_id_token_is_refused
A15: cargo test -p deck-streak-identity --test oidc -- --exact an_id_token_with_another_nonce_is_refused
A16: cargo test -p deck-streak-identity --test oidc -- --exact the_authorization_request_is_pkce_s256_with_the_exact_redirect
A17: cargo test -p deck-streak-identity --test oidc -- --exact a_return_target_outside_the_allow_list_is_refused
A18: cargo test -p deck-streak-identity --test linking_config -- --exact a_provider_is_off_without_its_credential
A19: cargo test -p deck-streak-identity --test linking_config -- --exact an_empty_provider_credential_refuses_start
A20: cargo test -p deck-streak-identity --test linking_config -- --exact linking_is_off_without_the_public_origin
A21: cargo test -p deck-streak-identity --test linking -- --exact a_sign_in_with_an_unlinked_identity_creates_nothing
A22: cargo test -p deck-streak-identity --test linking -- --exact a_linked_identity_of_another_user_is_refused
A23: cargo test -p deck-streak-identity --test linking -- --exact a_sign_in_rotates_the_session
A24: cargo test -p deck-streak-identity --test linking -- --exact linking_twice_is_refused
A25: cargo test -p deck-streak-identity --test linking -- --exact unlinking_telegram_is_refused
A26: cargo test -p deck-streak-identity --test linking -- --exact unlinking_needs_a_fresh_telegram_session
A27: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_from_another_origin_is_refused
A28: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_without_user_verification_is_refused
A29: cargo test -p deck-streak-identity --test passkeys -- --exact a_reused_passkey_challenge_is_refused
A30: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_challenge_expires_at_three_hundred_seconds
A31: cargo test -p deck-streak-identity --test passkeys -- --exact a_regressed_counter_is_refused
A32: cargo test -p deck-streak-identity --test passkeys -- --exact a_passkey_sign_in_stores_its_counter
A33: cargo test -p deck-streak-identity --test passkeys -- --exact the_user_handle_carries_no_personal_data
A34: cargo test -p deck-streak-identity --test apple -- --exact the_apple_client_secret_is_es256_for_three_hundred_seconds
A35: cargo test -p deck-streak-identity --test apple -- --exact apples_refresh_token_is_stored_sealed
A36: cargo test -p deck-streak-identity --test apple -- --exact unlinking_apple_revokes_before_it_deletes
A37: cargo test -p deck-streak-identity --test apple -- --exact a_failed_revocation_is_queued_and_the_unlink_completes
A38: cargo test -p deck-streak-identity --test apple -- --exact a_queued_revocation_is_tried_five_times_then_dropped
A39: cargo test -p deck-streak-coordination --test linked_sign_in_erase -- --exact the_erase_revokes_apple_first_and_never_waits_on_a_failure
A40: cargo test -p deck-streak-identity --test rights -- --exact linked_sign_in_export_and_erase_are_symmetric
A41: cargo test -p deck-streak-identity --test rights -- --exact the_export_never_holds_a_token
A42: cargo test -p deck-streak-identity --test linking -- --exact no_linking_secret_reaches_a_log
A43: cargo test -p deck-streak-api --test linking_routes -- --exact the_flow_cookie_is_host_prefixed_and_short_lived
A44: cargo test -p deck-streak-api --test linking_routes -- --exact a_link_session_reaches_only_the_linking_routes
A45: pnpm exec vitest run web/app/src/routes/link.test.ts -t "the link page posts the code from the fragment"
A46: pnpm exec vitest run web/app/src/routes/settings.test.ts -t "the sign-in section offers no unlink for telegram"
A47: pnpm exec vitest run web/app/src/routes/signin.test.ts -t "outside telegram the shell offers sign-in"
A48: cargo test -p deck-streak-coordination --test linked_sign_in_erase -- --exact an_erase_without_the_apple_credential_queues_the_revocation
A49: cargo test -p deck-streak-coordination --test job_table -- --exact the_link_revocation_job_is_hourly_at_a_free_minute
A50: cargo test -p deck-streak-daemon --test roles -- --exact the_link_revocation_job_runs_without_a_credential
A51: cargo test -p deck-streak-bot --test commands -- --exact delete_revokes_apple_links_before_it_erases
A52: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_link_revocation_timer_holds_the_tables_minute
A53: cargo test -p deck-streak-identity --test linking -- --exact a_link_session_is_not_an_owner_session
A54: cargo test -p deck-streak-identity --test apple -- --exact a_revoker_without_the_credential_queues_the_token
A55: cargo test -p deck-streak-daemon --test lifecycle -- --exact each_role_reads_exactly_its_linking_credentials
A56: cargo test -p deck-streak-identity --test apple -- --exact a_queued_revocation_opens_without_a_subject_and_is_deleted_on_success
A57: cargo test -p deck-streak-identity --test linking -- --exact a_ninth_link_code_evicts_the_oldest
A58: cargo test -p deck-streak-identity --test oidc -- --exact a_ninth_flow_evicts_the_oldest
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree. They have no
line in the acceptance fence, because no public test can run a pack's row. The private-wiring change
that enforces B1 and B2 is the auth pack's rows moving from reporting on a tree with no linked
sign-in to judging this one: the delivery hands back the JSON diff that lists them enforced for
DeckStreak.

| id | criterion | decided by |
|---|---|---|
| B1 | over `crates/identity/src/` (the link code, the flows, the verifiers, the passkeys, the Apple client) and `migrations/013101_identity_linked_sign_in.sql`: identities keyed on issuer and subject with a binding time; no email link; a link code of at least 112 bits living at most ten minutes; PKCE `S256`; the issuer, audience and nonce checked; an exact redirect URI and no open redirect; a mix-up defence; a fixed relying party and origin, a server challenge, user verification required, a user handle with no personal data, no attestation asked, the counter stored; Apple's secret ES256 and short-lived, and revoked on deletion; the session rotated at sign-in | the auth pack |
| B2 | over `crates/api/src/linking_routes.rs` and `crates/identity/src/session.rs`: the cookies' prefix and flags, the CSRF bound on every state-changing route, and no redirect taken from the request | the web-security pack |
| B3 | over `privacy.json`, `PRIVACY.md` and `crates/identity/src/data_rights.rs`: the three tables are declared with purpose, basis and retention, and export and erase cover them | the privacy-gdpr pack |
| B4 | over `web/app/src/routes/link/`, `web/app/src/routes/signin/` and the settings section: labels in names, native controls, contrast and reduced motion | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/identity/src/linking.rs` | `deck-streak-identity` | added: the link code, the link session, the link and the unlink, the owner check at sign-in |
| `crates/identity/src/oidc.rs` | `deck-streak-identity` | added: the flows, the authorization request, the callback's refusals, the `ProviderHttp` port |
| `crates/identity/src/passkeys.rs` | `deck-streak-identity` | added: the relying party, the ceremonies, their refusals, the counter |
| `crates/identity/src/apple.rs` | `deck-streak-identity` | added: the client secret, the seal, the revocation and its retry |
| `crates/identity/src/linking_config.rs` | `deck-streak-identity` | added: the origin, the providers' identifiers and the three credentials' loads |
| `crates/identity/src/session.rs` | `deck-streak-identity` | changed: a session's proof, and `OwnerSession`'s admission by proof |
| `crates/identity/src/data_rights.rs` | `deck-streak-identity` | added: the three tables exported and erased, `identity_revocations` exempt |
| `crates/identity/src/lib.rs` | `deck-streak-identity` | changed: the modules |
| `crates/identity/Cargo.toml` | `deck-streak-identity` | changed: `openidconnect`, `webauthn-rs`, `chacha20poly1305`, `p256` (ADR-131, ADR-132, ADR-133) |
| `crates/identity/tests/linking.rs` | `deck-streak-identity` | added: A1 to A4, A21 to A26, A42, A53, A57 |
| `crates/identity/tests/oidc.rs` | `deck-streak-identity` | added: A5 to A17, A58 |
| `crates/identity/tests/linking_config.rs` | `deck-streak-identity` | added: A18 to A20 |
| `crates/identity/tests/passkeys.rs` | `deck-streak-identity` | added: A27 to A33 |
| `crates/identity/tests/apple.rs` | `deck-streak-identity` | added: A34 to A38, A54, A56 |
| `crates/identity/tests/rights.rs` | `deck-streak-identity` | added: A40, A41 |
| `crates/identity/tests/support/mod.rs` | `deck-streak-identity` | added: the test issuer, its keys and tokens, the software authenticator |
| `migrations/013101_identity_linked_sign_in.sql` | `deck-streak-identity` | added: the three tables, `token_id` in both `linked_identities` and `identity_revocations` |
| `crates/coordination/src/linked_sign_in_erase.rs` | `deck-streak-coordination` | added: the erase's revocation step before the engine |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/tests/linked_sign_in_erase.rs` | `deck-streak-coordination` | added: A39, A48 |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job `link_revocation`, hourly at minute 41 |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: A49 |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: /delete erases through `erase_with_revocation` |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: A51 |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role reads the two Apple credentials as optional and joins identity's revoker to the erase |
| `crates/daemon/src/role_data.rs` | `deck-streak-daemon` | changed: the data role's erase runs `erase_with_revocation`, with the same optional reads |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the job `link_revocation` joins identity's revoker, its two credentials optional |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A50 |
| `crates/daemon/tests/lifecycle.rs` | `deck-streak-daemon` | changed: A55 |
| `deploy/systemd/deck-streak-job@link_revocation.timer` | deploy | added: hourly at minute 41, on the plain job template |
| `deploy/rail-contract.json` | deploy | changed: the timer's calendar |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A52 and `WAIVED`'s entries for `deck-streak-job@link_revocation.timer` |
| `scripts/tests/test_rail_contract.py` | repo | changed: `test_only_the_sync_job_reads_the_sync_login` admits `link_revocation`'s optional credential loads |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: identity's port registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for the three tables |
| `crates/api/src/linking_routes.rs` | `deck-streak-api` | added: the link, the flows, the passkeys, the identities |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes joined |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/linking_routes.rs` | `deck-streak-api` | added: A43, A44 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role loads the three credentials through its loader, builds the provider HTTP client and starts the revocation retry |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: identity's `ProviderHttp` adapter over `reqwest`, redirects disabled, 10 seconds per call |
| `crates/daemon/Cargo.toml` | `deck-streak-daemon` | changed: `reqwest`, for the adapter (ADR-131) |
| `web/app/src/routes/link/+page.svelte` | miniapp | added: the link page |
| `web/app/src/routes/link.test.ts` | miniapp | added: A45 |
| `web/app/src/routes/signin/+page.svelte` | miniapp | added: the sign-in page |
| `web/app/src/routes/signin.test.ts` | miniapp | added: A47 |
| `web/app/src/lib/settings/SignInMethods.svelte` | miniapp | added: the settings section |
| `web/app/src/routes/settings/+page.svelte` | miniapp | changed: the section joined |
| `web/app/src/routes/settings.test.ts` | miniapp | changed: A46 |
| `web/app/src/lib/api.ts` | miniapp | changed: the shell offers sign-in outside Telegram |
| `web/app/src/lib/routes.ts` | miniapp | changed: the two routes |
| `web/app/messages/*.json` | miniapp | changed: the pages' strings, in each locale's catalog |
| `.env.example` | repo | changed: the origin and the four identifiers, unset but the origin's neutral example |
| `deploy/README.md` | deploy | changed: the three credentials' rows, each "bound by #347's delivery; no unit names it until then" |
| `docs/auth/identity-model.md` | docs | added: the identity model, its front matter naming Telegram primary and the three linked methods |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables rows for the three tables |
| `privacy.json` | repo | changed: the three tables' purpose, basis and retention |
| `PRIVACY.md` | docs | changed: one line per category |
| `deny.toml` | repo | changed: only if a new licence or source must be allowed, with its reason |
| `Cargo.toml`, `Cargo.lock`, `.sqlx/` | workspace | changed: the four crates of ADR-131 to ADR-133, and `reqwest` at the version the lock already holds for the bot's client |
| `docs/specs/SPEC-131-passkeys-google-and-apple-link-to-the-owners-telegram-account-and-every-doubtful-assertion-is-refused.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/w7-sign-in-and-linking.md` | docs | added by the W7 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-131.md` | docs | added |
| `scripts/mutation-rows.d/S13100-S13199.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It registers no provider client and binds no credential: the owner does, and each provider stays
  off until then (#347).
- It chooses no domain; the public origin is configuration until the owner's choice (#168).
- It changes none of the Mini App handshake's checks, which stay the Telegram way in (#17).
- It adds no bot command for linking: linking starts from the settings screen (#57).
- It reads no email and joins nothing by email (#58).

## 6. Risks

- **A forged or replayed assertion admitted.** R7 and R11 refuse each shape by name; detected by A5
  to A15 and A27 to A31.
- **A linked identity that outlives its owner.** R8's owner check at every sign-in; detected by
  A22.
- **Login CSRF.** The flow cookie binds the state to the browser that started it; detected by A5,
  A6 and A43.
- **A revocation that never completes.** R14 bounds it and pages once; detected by A37, A38 and
  A48.
- **A token in a backup.** R13 seals it under a credential no backup holds; detected by A35.
- **OpenSSL in the build.** `webauthn-rs` links it (ADR-132); the dependency audit covers it.

## 7. Parity goldens

None. The predecessor had no linked sign-in, so nothing is ported and no golden is generated.

## 8. Tables and the v9 import

`linked_identities`, `passkeys` and `identity_revocations` (identity,
`migrations/013101_identity_linked_sign_in.sql`). The predecessor kept no sign-in method beyond its
owner chat, so W8's import maps nothing into them and they start empty.

## 9. Mutation rows

Each row is a plant the verifier can replay: `python3 scripts/mutation_rows.py prove --band
S13100-S13199` applies each one alone, runs its killer, and restores the file byte for byte. Every
killer runs against the test issuer and the software authenticator of `tests/support/`, never a
network or a provider.

| row | target | what it guards | killer |
|---|---|---|---|
| `S13101-CODE-BITS` | `crates/identity/src/linking.rs` | 16 bytes of the operating system's generator | `linking::a_link_code_is_random_and_kept_hashed` |
| `S13102-CODE-LIFE` | `crates/identity/src/linking.rs` | 600 seconds; the test names 599 and 600 | `linking::a_link_code_expires_at_six_hundred_seconds` |
| `S13103-CODE-ONCE` | `crates/identity/src/linking.rs` | a code is removed when redeemed | `linking::a_link_code_redeems_once` |
| `S13104-REAUTH-AGE` | `crates/identity/src/linking.rs` | 300 seconds; the test names 300 and 301 | `linking::a_link_code_needs_a_fresh_telegram_session` |
| `S13105-FLOW-REQUIRED` | `crates/identity/src/oidc.rs` | no flow cookie is `state_invalid` | `oidc::a_callback_without_its_flow_is_refused` |
| `S13106-STATE-EQUAL` | `crates/identity/src/oidc.rs` | the state's comparison | `oidc::a_callback_with_another_state_is_refused` |
| `S13107-FLOW-LIFE` | `crates/identity/src/oidc.rs` | 600 seconds; the test names 599 and 600 | `oidc::a_flow_expires_at_six_hundred_seconds` |
| `S13108-FLOW-ONCE` | `crates/identity/src/oidc.rs` | the flow is removed before the exchange | `oidc::a_replayed_callback_is_refused` |
| `S13109-MIXUP` | `crates/identity/src/oidc.rs` | the callback's provider and `iss` against the flow's | `oidc::a_mixed_up_callback_is_refused` |
| `S13110-ID-TOKEN-REQUIRED` | `crates/identity/src/oidc.rs` | an exchange with no ID token is refused | `oidc::an_exchange_without_an_id_token_is_refused` |
| `S13111-SIGNATURE` | `crates/identity/src/oidc.rs` | the ID token's verifier is the issuer's keys, never an insecure one | `oidc::an_id_token_with_a_foreign_signature_is_refused` |
| `S13112-ISSUER` | `crates/identity/src/oidc.rs` | the expected issuer | `oidc::an_id_token_from_another_issuer_is_refused` |
| `S13113-AUDIENCE` | `crates/identity/src/oidc.rs` | the client id as the audience | `oidc::an_id_token_for_another_audience_is_refused` |
| `S13114-EXPIRY` | `crates/identity/src/oidc.rs` | the kernel's clock as the verifier's time | `oidc::an_expired_id_token_is_refused` |
| `S13115-NONCE` | `crates/identity/src/oidc.rs` | the flow's nonce as the expected one | `oidc::an_id_token_with_another_nonce_is_refused` |
| `S13116-PKCE-S256` | `crates/identity/src/oidc.rs` | the challenge method `S256` | `oidc::the_authorization_request_is_pkce_s256_with_the_exact_redirect` |
| `S13117-ALLOW-LIST` | `crates/identity/src/oidc.rs` | the return targets `/` and `/settings` | `oidc::a_return_target_outside_the_allow_list_is_refused` |
| `S13118-PROVIDER-OFF` | `crates/identity/src/linking_config.rs` | a missing credential turns its provider off | `linking_config::a_provider_is_off_without_its_credential` |
| `S13119-LINKING-OFF` | `crates/identity/src/linking_config.rs` | an unset origin turns linking off | `linking_config::linking_is_off_without_the_public_origin` |
| `S13120-NOT-LINKED` | `crates/identity/src/linking.rs` | an unknown identity writes nothing | `linking::a_sign_in_with_an_unlinked_identity_creates_nothing` |
| `S13121-OWNER-CHECK` | `crates/identity/src/linking.rs` | the row's user id against the configured owner | `linking::a_linked_identity_of_another_user_is_refused` |
| `S13122-ROTATE` | `crates/identity/src/linking.rs` | the arriving session ends at sign-in | `linking::a_sign_in_rotates_the_session` |
| `S13123-UNIQUE-SUBJECT` | `migrations/013101_identity_linked_sign_in.sql` | `UNIQUE (issuer, subject)` (a script-mutation row whose cargo killer is in `deck-streak-identity`) | `linking::linking_twice_is_refused` |
| `S13124-LAST-METHOD` | `crates/identity/src/linking.rs` | the Telegram method is never unlinked | `linking::unlinking_telegram_is_refused` |
| `S13125-UNLINK-REAUTH` | `crates/identity/src/linking.rs` | unlinking checks the Telegram proof's age | `linking::unlinking_needs_a_fresh_telegram_session` |
| `S13126-ORIGIN` | `crates/identity/src/passkeys.rs` | the expected origin is the configured one | `passkeys::a_passkey_from_another_origin_is_refused` |
| `S13127-UV-REQUIRED` | `crates/identity/src/passkeys.rs` | user verification `required` | `passkeys::a_passkey_without_user_verification_is_refused` |
| `S13128-CHALLENGE-ONCE` | `crates/identity/src/passkeys.rs` | a ceremony's state is removed when used | `passkeys::a_reused_passkey_challenge_is_refused` |
| `S13129-CHALLENGE-LIFE` | `crates/identity/src/passkeys.rs` | 300 seconds; the test names 299 and 300 | `passkeys::a_passkey_challenge_expires_at_three_hundred_seconds` |
| `S13130-COUNTER` | `crates/identity/src/passkeys.rs` | a counter that does not advance is refused | `passkeys::a_regressed_counter_is_refused` |
| `S13131-COUNTER-STORED` | `crates/identity/src/passkeys.rs` | the new counter is written | `passkeys::a_passkey_sign_in_stores_its_counter` |
| `S13132-APPLE-SECRET-LIFE` | `crates/identity/src/apple.rs` | 300 seconds; the test reads `exp - iat` | `apple::the_apple_client_secret_is_es256_for_three_hundred_seconds` |
| `S13133-SEALED` | `crates/identity/src/apple.rs` | the token is sealed before it is written | `apple::apples_refresh_token_is_stored_sealed` |
| `S13134-REVOKE-FIRST` | `crates/identity/src/apple.rs` | the revocation precedes the delete | `apple::unlinking_apple_revokes_before_it_deletes` |
| `S13135-REVOKE-BOUND` | `crates/identity/src/apple.rs` | 10 seconds; the test runs on a paused clock and bounds its own wait at 11 seconds, so a mutant that waits longer fails rather than hangs | `apple::a_failed_revocation_is_queued_and_the_unlink_completes` |
| `S13136-FIVE-ATTEMPTS` | `crates/identity/src/apple.rs` | five attempts; the test names the fifth and a sixth that never comes | `apple::a_queued_revocation_is_tried_five_times_then_dropped` |
| `S13137-EXPORT-NO-TOKEN` | `crates/identity/src/data_rights.rs` | the export writes that a token is held, not its value | `rights::the_export_never_holds_a_token` |
| `S13138-LINK-SESSION-SCOPE` | `crates/identity/src/session.rs` | a `link` session is never an owner session | `linking::a_link_session_is_not_an_owner_session` |
| `S13139-ERASE-REVOKES-FIRST` | `crates/coordination/src/linked_sign_in_erase.rs` | the revocation step runs before the engine | `linked_sign_in_erase::the_erase_revokes_apple_first_and_never_waits_on_a_failure` |
| `S13140-NO-CREDENTIAL-QUEUES` | `crates/identity/src/apple.rs` | a revoker without the credential queues the token rather than dropping it | `apple::a_revoker_without_the_credential_queues_the_token` |
| `S13141-QUEUE-OPENS` | `crates/identity/src/apple.rs` | the queued token opens with its issuer and token id | `apple::a_queued_revocation_opens_without_a_subject_and_is_deleted_on_success` |
| `S13142-CODE-CAP` | `crates/identity/src/linking.rs` | 8; the test names the eighth and the ninth | `linking::a_ninth_link_code_evicts_the_oldest` |
| `S13143-FLOW-CAP` | `crates/identity/src/oidc.rs` | 8; the test names the eighth and the ninth | `oidc::a_ninth_flow_evicts_the_oldest` |

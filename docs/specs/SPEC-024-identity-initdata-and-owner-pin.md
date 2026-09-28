# SPEC-024: the server validates Telegram initData, pins the caller to the owner, and opens a short session

- **Wave:** W0. **Issue:** #17 (epic #1). **Context(s):** `deck-streak-identity`, `deck-streak-api` (the session routes), `deck-streak-daemon` (the `api` role's credentials).
- **Decided by:** ADR-006 (Telegram initData first, pinned to the owner, a `__Host-` session cookie), ADR-007 (same origin), ADR-010 and ADR-038 (credentials), ADR-025 (the API's layers), and this SPEC's ADR-024 (an in-memory bounded session store, its lifetimes, the initData freshness bound, and the decisions of §7).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-024.md`. The delivery made
  R1, R3, R5, R6 and R8 exact where the code decided them, did not admit `form_urlencoded`, and
  amended the manifest by three files and three dependencies, each with its reason (§7).

## 1. The problem, measured

- **The safety property.** The predecessor answered only the owner's chat (its bot's gate); the Mini
  App is a web page anyone can load, so the server must prove who calls on every request and serve
  only the owner (CHARTER 14). Nothing validates anything yet: `crates/identity/src/lib.rs` is
  documentation only (read at `main` e05dfa5). The sequence to build is
  `docs/schematics/initdata-auth-sequence.md`.
- **Telegram's rule** (core.telegram.org/bots/webapps, "Validating data received via the Mini App"):
  the data-check-string is every received field except `hash`, sorted by key, written `key=value`
  and joined by a line feed; the secret key is HMAC-SHA-256 of the bot token keyed with the constant
  `WebAppData`; the hash is the hex HMAC-SHA-256 of the data-check-string under that key; checking
  `auth_date` is left to the service. The `signature` field stays in the string for this check (only
  the third-party Ed25519 check excludes it).
- **The rows that judge it, applied BY HAND.** The web-security pack's
  `ws.tg-init-data-verified`, `ws.tg-init-data-constant-time`, `ws.tg-init-data-fresh` and
  `ws.session-cookie-flags` run on the maintainer's box (ADR-004), and their green is not trusted
  until the packs commit carrying the web-security fixes is vendored (ADR-004's
  re-pin). This delivery proves each control with its own tests (A1 to A6, A9) and its pull request
  carries a review of the validator against each row's rule; the box's verdict is posted beside it.
- **The rest that judge it:** web-security `ws.cookie-prefix`, `ws.session-samesite`,
  `ws.upload-csrf-cors`, `ws.rate-limit` (box); observability `obs.no-secret-fields` (gate); the
  auth pack's `auth.session-rotated-on-login` and `auth.logout-server-side`, and cyber-pipeline's
  `cp.session-idle-timeout` (box); privacy-gdpr `telegram-minimised` (gate, once SPEC-021 lands).

**Order.** After SPEC-025 (the router and its layers, into which this SPEC mounts its routes) and
SPEC-020. SPEC-026 reuses this SPEC's owner type for the bot's gate, so it lands after this one; the
Mini App (SPEC-028) calls these routes against a mock and does not wait for them.

## 2. Requirements

R1. `identity::init_data::validate` parses the raw `initData` as `application/x-www-form-urlencoded`,
    refuses a payload with no `hash`, a repeated key or a field that does not decode, builds the
    data-check-string from every field but `hash` (sorted by key, `key=value`, line-feed joined),
    derives the key as HMAC-SHA-256 of the bot token keyed with `WebAppData`, and compares the
    hex-decoded `hash` with `Mac::verify_slice`, which compares in constant time. No other comparison
    of hash bytes exists in the crate.
R2. After a valid signature, `auth_date` must be present, an integer, no more than 60 seconds in the
    future (ADR-024's skew allowance), and no older than `DECKSTREAK_INIT_DATA_MAX_AGE_SECONDS` (default 3600, ADR-024), read
    against the kernel's `Clock`.
R3. A missing, malformed, forged or stale payload is answered 401 with one reason code
    (`init_data_invalid` or `init_data_stale`); a valid, fresh payload whose `user.id` is not the
    owner's is answered 403. The owner's user id is the credential `owner-user-id`, and the bot
    token the credential `telegram-bot-token`, both through the kernel's loader; a missing one
    refuses start by its id.
R4. The raw `initData`, its `hash`, the bot token and a session id never reach a log line, a span
    field, an error or a row; a refusal logs its reason code only (observability
    `obs.no-secret-fields`). Of the user object only the id is kept, in memory (privacy-gdpr
    `telegram-minimised`).
R5. `identity::session::Sessions` is an in-memory store (ADR-024): a successful handshake creates a
    NEW session (an id of 32 bytes from the operating system's generator, stored as its SHA-256),
    never reusing one the request carried; a session ends after 30 minutes without a request or 8
    hours after it began, whichever is first; at most 8 live sessions are kept, the oldest evicted.
R6. The session cookie is `__Host-deckstreak_session`, with `Path=/`, `Secure`, `HttpOnly`,
    `SameSite=Strict`, no `Domain`, and a `Max-Age` equal to the absolute lifetime.
R7. `identity::OwnerSession` is an axum extractor (`FromRequestParts`) that admits a request whose
    cookie names a live session and answers 401 otherwise; it refreshes the session's idle timer.
R8. The API serves `POST /api/session` (a JSON body carrying the raw `initData`; the handshake),
    `DELETE /api/session` (ends the session on the server and clears the cookie) and `GET /api/me`
    (behind `OwnerSession`: the server's study day as its ISO date, from the kernel's rule and
    clock).
R9. A state-changing route refuses a request that is not `application/json`, or whose
    `Sec-Fetch-Site` header is present and not `same-origin`, with 403 (the CSRF bound; Caddy gives
    the Mini App and the API one origin, so CORS stays closed).
R10. Handshakes are bounded: past 30 in a minute (ADR-024), `POST /api/session` answers 429 until the
    minute turns (an in-memory counter on the kernel's clock).
R11. Test payloads are synthetic: the tests' bot tokens never have the Bot API token's shape, and
    their user ids have fewer than seven digits, so the public scrub never reads a fixture as a
    secret or a real id.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a payload signed with the `WebAppData` key of the bot token is accepted | `init_data` test; `ws.tg-init-data-verified` (by hand) |
| A2 | a payload with one field changed after signing is refused with 401 | `init_data` test |
| A3 | a payload signed with another bot token is refused with 401 | `init_data` test |
| A4 | a stale `auth_date` is refused with 401, and a future one past the skew too | `init_data` test; `ws.tg-init-data-fresh` (by hand) |
| A5 | a payload carrying a `signature` field validates with it in the data-check-string | `init_data` test |
| A6 | the hash is compared only through `verify_slice` (a planted `==` comparison is refused) | `boundary` test; `ws.tg-init-data-constant-time` (by hand) |
| A7 | a valid payload for another user is refused with 403 | `owner` test |
| A8 | neither `initData` nor its hash reaches the log, on a refusal or an acceptance | `owner` test; `obs.no-secret-fields` |
| A9 | the session cookie is `__Host-` prefixed, `Secure`, `HttpOnly` and `SameSite=Strict` | api `session_routes` test; `ws.session-cookie-flags` (by hand), `ws.cookie-prefix`, `ws.session-samesite` |
| A10 | each handshake issues a new session id | `session` test; `auth.session-rotated-on-login` |
| A11 | a session ends after its idle timeout, and at its absolute lifetime | `session` test; `cp.session-idle-timeout` |
| A12 | `/api/me` answers the server's study day only with a live session | api `session_routes` test |
| A13 | a cross-site or non-JSON state change is refused with 403 | api `session_routes` test; `ws.upload-csrf-cors` |
| A14 | handshakes past the bound are refused with 429 | api `session_routes` test; `ws.rate-limit` |
| A15 | logging out ends the session on the server | api `session_routes` test; `auth.logout-server-side` |
| A16 | a missing owner credential refuses start by its id | `owner` test |

```acceptance
A1: cargo test -p deck-streak-identity --test init_data -- --exact a_payload_signed_with_the_webappdata_key_is_accepted
A2: cargo test -p deck-streak-identity --test init_data -- --exact a_tampered_field_is_refused_with_401
A3: cargo test -p deck-streak-identity --test init_data -- --exact a_payload_signed_with_another_bot_token_is_refused_with_401
A4: cargo test -p deck-streak-identity --test init_data -- --exact a_stale_or_future_auth_date_is_refused_with_401
A5: cargo test -p deck-streak-identity --test init_data -- --exact a_payload_with_a_signature_field_validates_with_it_in_the_check_string
A6: cargo test -p deck-streak-identity --test boundary -- --exact the_hash_is_compared_only_through_verify_slice
A7: cargo test -p deck-streak-identity --test owner -- --exact a_valid_payload_for_another_user_is_refused_with_403
A8: cargo test -p deck-streak-identity --test owner -- --exact init_data_never_reaches_the_log
A9: cargo test -p deck-streak-api --test session_routes -- --exact the_session_cookie_is_host_prefixed_secure_httponly_and_strict
A10: cargo test -p deck-streak-identity --test session -- --exact each_handshake_issues_a_new_session_id
A11: cargo test -p deck-streak-identity --test session -- --exact an_idle_session_expires_and_an_old_one_ends_at_its_lifetime
A12: cargo test -p deck-streak-api --test session_routes -- --exact me_answers_the_study_day_only_with_a_live_session
A13: cargo test -p deck-streak-api --test session_routes -- --exact a_cross_site_or_non_json_state_change_is_refused
A14: cargo test -p deck-streak-api --test session_routes -- --exact handshakes_past_the_bound_are_refused_with_429
A15: cargo test -p deck-streak-api --test session_routes -- --exact logging_out_ends_the_session_on_the_server
A16: cargo test -p deck-streak-identity --test owner -- --exact a_missing_owner_credential_refuses_start_by_its_id
```

Payloads are built in the tests by signing synthetic fields with a synthetic token, so each test
knows exactly what it changed. Every clock is a `ManualClock`. A6 reads `crates/identity/src/` and
refuses any comparison of the hash other than `verify_slice`, proving the refusal on a planted file
under `crates/identity/tests/fixtures/`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/identity/Cargo.toml` | `deck-streak-identity` | changed: kernel, axum, hmac, sha2, subtle, getrandom, serde, serde_json, thiserror, tracing; dev: tempfile (§7), tower, tokio. `form_urlencoded` is not admitted (§7) |
| `crates/identity/src/lib.rs` | `deck-streak-identity` | changed: the modules, and `Refusal` with its reason codes |
| `crates/identity/src/init_data.rs` | `deck-streak-identity` | added: the one validator and its strict decoder |
| `crates/identity/src/owner.rs` | `deck-streak-identity` | added: the owner pin, its credentials and the gate |
| `crates/identity/src/session.rs` | `deck-streak-identity` | added: the store, the cookie, the extractor |
| `crates/identity/src/settings.rs` | `deck-streak-identity` | added: the freshness bound |
| `crates/identity/tests/init_data.rs`, `boundary.rs`, `owner.rs`, `session.rs` | `deck-streak-identity` | added: A1 to A8, A10, A11, A16, and four tests of §7 |
| `crates/identity/tests/fixtures/planted_compare.rs.fixture` | `deck-streak-identity` | added: the planted comparison A6 refuses |
| `crates/api/src/session_routes.rs`, `crates/api/src/router.rs`, `crates/api/src/lib.rs` | `deck-streak-api` | added or changed: the three routes (the module declared in `lib.rs`), the CSRF bound, the handshake bound, `ApiState::with_owner` (§7) |
| `crates/api/Cargo.toml` | `deck-streak-api` | changed: serde, serde_json (ADR-029; §7) |
| `crates/api/tests/session_routes.rs` | `deck-streak-api` | added: A9, A12 to A15, and three tests of §7 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: loads identity's credentials at start, and gives the router the owner's access |
| `crates/daemon/src/main.rs` | `deck-streak-daemon` | changed (§7): hands the log writer's redactor to the role, so the credentials are registered with it |
| `crates/daemon/tests/lifecycle.rs` | `deck-streak-daemon` | changed (§7): A11's run carries synthetic credentials, and the role refuses start by a missing one's id |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: hmac, sha2, subtle (ADR-006), getrandom (ADR-024) |
| `.env.example` | repo | changed: the freshness bound, and the two credential ids |
| `.packs/wiring.json` | repo | changed (§7): the box's `ws.tg-init-data-verified` expectations that waited on #17 are removed |
| `docs/schematics/owner-session.md` | repo | changed: written at planning, redrawn as built with the parts and a handshake's order |
| `docs/decisions/ADR-024-owner-sessions-in-memory-behind-a-host-cookie.md` | repo | changed: accepted, with the decisions the delivery made |
| `docs/red-first/SPEC-024.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It links no Google, Apple, passkey or Telegram Login method, and writes no identity-model document
  (#58).
- It offers no third-party Ed25519 validation of `initData`; ADR-006 keeps it available for a third
  party, and the linked sign-in work decides whether one is needed (#58).
- It gates no bot update: the bot's owner gate uses this SPEC's owner type (#19).
- It builds no client handshake (#21).
- It exposes no route beyond the session and `/api/me`: each screen's routes arrive with it
  (#37).

## 6. Risks

- **The by-hand rows are wrong where the box says green.** That is why they are by hand: the tests
  above prove each control, and the re-pin that carries train 84's fixes is the every-pack-enforced
  work (#60); until then a green box verdict on those four rows is recorded but not
  counted.
- **A restart logs the owner out.** Sessions are in memory by decision (ADR-024); the Mini App
  re-handshakes once with the launch `initData` (SPEC-028), and after its hour the owner reopens
  the Mini App, which gives Telegram's fresh payload.
- **Clock skew rejects a fresh payload.** The 60-second allowance for a future `auth_date` covers
  ordinary skew; a host clock far off is visible in every log line's timestamp and in the liveness
  job's rollover-drift check.
- **A fixture trips the public scrub.** R11 keeps token shapes and seven-digit ids out of the tests;
  the gate's scrub stage would name the file and rule if one slipped in.

## 7. Amendments at delivery

- **R1: one strict decoder, and `form_urlencoded` is not admitted.** `form_urlencoded` 1.2.2's
  `parse` decodes bytes that are not UTF-8 lossily and keeps a malformed escape (`%zz`) as text
  (`decode`, `decode_utf8_lossy`), so it cannot refuse "a field that does not decode"; a strict
  check beside it would be a second parse of one input, which could disagree with the reading the
  hash was checked against. `identity` decodes with one strict decoder: `+` is a space, `%` must be
  followed by two hex digits, the bytes must be UTF-8; an empty segment, a segment with no `=` and
  a repeated name are refused too. The malformed-payload test signs each of those cases over what a
  lenient decoder reads, so only the strict reading refuses it (ADR-024, "Decided at delivery").
- **R1: the key and the one comparison.** `WebAppKey::from_bot_token` keeps the HMAC-SHA-256 of the
  token keyed with `WebAppData`, never the token. The comparison is hmac 0.13.0's
  `Mac::verify_slice` (digest 0.11.3), which checks the length and compares through
  `ctutils::CtEq`. A6's census reads `crates/identity/src/` with comments and literals blanked,
  refuses a clause that names a hash and compares (`==`, `!=`, `.eq(`, `.ne(`, `.cmp(`,
  `.partial_cmp(`, `ct_eq(`, `starts_with(`, `ends_with(`), and asserts exactly one `verify_slice`
  and exactly one HMAC `finalize`, the key's derivation, so no MAC's bytes leave it to be compared
  under another name. It refuses the planted fixture's two comparisons, on their lines, and not the
  comment and the string beside them that say the same.
- **R2: the setting's range** is 1 to 86400 whole seconds; outside it, or not a number, it refuses
  start by name. The window is inclusive: launch data exactly as old as the bound, or exactly 60
  seconds ahead, is fresh.
- **R3: the reason codes and the answers.** identity answers `init_data_invalid` and
  `init_data_stale` (401), `not_owner` (403) and the extractor's `no_session` (401), each as
  `{"reason":"<code>"}`, logged by the code alone; the API's own refusals are `cross_site_request`
  and `not_json` (403) and `too_many_handshakes` (429). `init_data_stale` covers both ends of the
  window. A body that is not JSON carrying `init_data` is `init_data_invalid`.
- **R3: the credentials.** `owner-user-id` must be a positive whole number and `telegram-bot-token`
  must not be blank; either one missing or malformed refuses start by its id, never by its value.
  The daemon's `main` now hands the log writer's redactor to the role, so both values are
  registered with the one registry the writer reads (SPEC-020 R12;
  `docs/schematics/startup-settings-and-secrets.md`): before, the role could not reach it, and a
  loader over a redactor of its own would have registered them where no line is scrubbed.
- **R5: rotation, eviction and the compare.** A successful handshake ends the session its request
  carried, then opens a new one, so the carried id stops working; a refused handshake leaves a live
  session alone. Opening a ninth session evicts the one that began first. A presented id is hashed
  and compared with each live digest through `subtle` (ADR-006), in constant time.
- **R6: the cookie is one literal.** `__Host-deckstreak_session=<id>; Path=/; Max-Age=28800;
  Secure; HttpOnly; SameSite=Strict`, written whole beside `SET_COOKIE`, where the box's
  web-security rows read a cookie; the logout's is the same, empty, with `Max-Age=0`. The handshake
  answers 200, as both schematics draw it, and the logout 204.
- **R8: `ApiState::with_owner`.** `ApiState::new(readiness)` serves the health routes alone, as
  SPEC-025's tests build it; `with_owner(OwnerAccess)` adds the session routes under the same layers.
  The `api` role always gives it and refuses start without the credentials, so the service always
  serves them. Chosen against requiring the access in `new`, which would make every health-only
  router hold a bot key and an owner id, and against session routes that answer 503 when
  unconfigured, a mode the service never runs.
- **R9, R10: the order.** The cross-site bound runs first, then the handshake bound, both before a
  byte of the body is read; the handshake's own body limit is 16 KiB (ADR-024), inside the shell's
  2 MiB. The bound counts every handshake it admits, forged ones included, which is what caps the
  HMAC work; a 429 carries `Retry-After`, the whole seconds until the minute turns.
- **Manifest amendments.** `crates/daemon/src/main.rs` (the redactor, above);
  `crates/daemon/tests/lifecycle.rs` (the role now refuses start without its credentials, so A11's
  run carries a synthetic credentials directory, and an added test proves the binary refuses start
  by the missing id); `.packs/wiring.json` (the box's `ws.tg-init-data-verified` expectations in
  web-security and cyber-pipeline waited on #17: the row reads green on this delivery, and
  `scripts/box-packs.sh` refuses a green expectation as stale); identity's dev-dependency on
  `tempfile` (ADR-020), for A16's credentials directory, since SPEC-030's temp hygiene refuses
  `std::env::temp_dir()`; the API's dependencies on `serde` and `serde_json` (ADR-029), for the
  handshake's body and the answers.
- **Tests beyond the criteria.** identity: `a_malformed_payload_is_refused_with_401`,
  `the_freshness_bound_is_read_from_its_setting`, `the_oldest_session_is_evicted_past_eight`,
  `the_owner_session_extractor_admits_only_a_live_cookie`; api: `a_handshake_ends_the_session_it_carried`,
  `the_mini_apps_own_requests_open_a_session_and_read_the_day` (the wire contract of SPEC-028,
  sent as `web/app/src/lib/api.ts` sends it), `the_handshake_body_is_bounded_below_the_shells_limit`;
  daemon: `the_api_role_refuses_start_without_an_identity_credential`.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).

Amendment (2026-09-28): R2's future skew is pinned at 60 seconds by a test that spells it (#222).
A4's test builds its dates from `FUTURE_SKEW`, so they move with any mutant of the constant: it
still passed under a skew of 61 or of 120 seconds.
`an_auth_date_60_seconds_ahead_is_fresh_and_61_is_refused_with_401`
(`crates/identity/tests/init_data.rs`), added at 9fc901a, spells both dates literally: launch data
dated 60 seconds ahead of the server's clock is fresh, as the user it names, and launch data dated
61 seconds ahead is refused as `init_data_stale` with 401. Row S02406 holds it, with a skew of 61
seconds; cargo-mutants 27.1.0 lists 5 mutants of `crates/identity/src/settings.rs` and none of the
constant. The test was not red at 3f916ce, where the code already held the bound. On that tree each
of the skews 0, 59, 61 and 120 seconds made it red by assertion, selecting one test: 0 and 59
refused the launch 60 seconds ahead (left `Err(InitDataStale)`, right `Ok(TelegramUserId(4242))`),
and 61 and 120 admitted the one 61 seconds ahead (left `Ok(Caller { user: TelegramUserId(4242) })`,
right `Err(InitDataStale)`). The file was restored byte for byte after each, its sha256
`51ff9221e1d770fa958e3be7b33a6b33a2eb2f7a2d2d8d08ce836eb623006823` before and after.

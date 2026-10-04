# SPEC-343: the service sends APNs and web push to recording fakes, and a web harness reads the 8BitDo remote

- **Issue:** #621, the app campaign's push and remote spike (SPEC-334 row 1.2; R12, R15).
  **Context(s):** `deck-streak-push`, a new adapter at the edge that depends on the kernel alone and
  that the daemon does not compose; and `miniapp` (`web/app/src`), which gains the remote's input
  modules and a harness screen.
- **Decided by:** ADR-341 (native push through the one router) and ADR-342 (the remote, its mapping
  and the screen kept awake), which this spike designs inside; and ADR-354 (who builds the APNs
  request, the web push library, where the senders sit, how a key reaches them, the fakes' shape,
  where retry lives, the endpoint allow-list, and the remote's defaults).
- **Status:** a spike, delivered by the draft pull request that adds this file, with its tests and
  `docs/red-first/SPEC-343.md`. **Mutation band:** `S34300-S34399`.

## 1. The problem, measured

The question: can the service build an APNs notification and a web push message that a platform
would accept, and can a web page read the 8BitDo remote in both of its modes and keep the screen
awake while it drives a review? This spike proves the server half against recording fakes in CI and
the browser half in a harness. A push received on a device, and the remote on a device, are the
owner's first device session (#629).

What was measured before anything was written:

- No sender exists. `git grep -il -E 'apns|web.push|webpush|vapid' 1eec0870 -- crates | wc -l`
  prints 0. The router's one outbound port is the bot's (`BotTransport`,
  `crates/notifications/src/transport.rs:84`), and its outcome `Pushed` holds `Delivered`, `Failed`
  and `Unsupported`, so nothing can yet say "this device is gone", which ADR-341 needs ("the
  transports must report failure for the router's ledger").
- No input code exists in the web app. `git grep -il -E
  'keydown|keyup|gamepad|wake.?lock|service.?worker|vapid|pushmanager' 1eec0870 -- web | wc -l`
  prints 0.
- The workspace compiles no HTTP/2 and no P-256. `Cargo.lock` holds `hyper` (line 2074) with no
  `h2` among its dependencies, and holds no `h2`, `p256`, `ecdsa` or `aes-gcm` package. APNs speaks
  HTTP/2 only, and both the APNs provider token and a VAPID token are ES256 JWTs over P-256.
- The two APNs crates read for this spike, `a2` and `apns-h2`, each choose their endpoint from an
  enum holding Apple's two services alone, with no way to give a URL, a connector or a root store
  (their `Endpoint` and `ClientConfig` items). A test could not point either at a loopback fake, so
  CI could not observe the request either sends (ADR-354 D1).
- The bot's transport already sets the rule a fake needs: its base URL "must be `https:`, or
  `http:` to a loopback host (a local Bot API server, or a test's fake)"
  (`crates/bot/src/transport.rs:60`), and its fake is a recording axum server on a loopback port
  (`crates/bot/tests/support/fake_bot_api.rs:2`).

## 2. Requirements

R1. A new adapter crate, `deck-streak-push` in `crates/push`, depends on `deck-streak-kernel` alone
    (its clock and its time type). The daemon does not compose it. `docs/CONTEXT-MAP.md` gains its
    line, `depends on: kernel`, and a paragraph saying why nothing composes it yet (#640).
R2. The APNs sender posts one notification over HTTP/2 to `/3/device/<token>` at the origin of the
    device's environment (development or production, chosen per device). It sends
    `authorization: bearer <provider token>`, `apns-push-type: alert`, `apns-priority: 10`,
    `apns-topic` (the app's bundle id, from configuration), `apns-expiration` (the clock's epoch
    second plus the notification's time to live, or `0` when the time to live is zero) and, when
    the notification has a collapse key, `apns-collapse-id`. The body is
    `{"aps":{"alert":{"title":T,"body":B}}}`. A body over 4096 bytes is refused before any request.
R3. The provider token is an ES256 JWT whose header is `{"alg":"ES256","kid":<key id>}` and whose
    claims are `{"iss":<team id>,"iat":<the clock's epoch second>}`, signed with the APNs signing
    key. The sender reuses one token until it is 45 minutes old and then mints the next. On a 403
    whose reason is `ExpiredProviderToken` it mints a new token and resends the notification once,
    but only when the token it sent is at least 20 minutes old; otherwise the answer is a refusal of
    the provider token, and no token is minted. The age check and the mint happen under one lock
    with no await inside it, and a refused token is replaced only while it is still the current
    one, so calls refused together mint one token between them and each resends with it.
R4. Each call answers one typed, `#[must_use]` outcome and makes one request, R3's single resend
    apart. The sender waits for nothing and retries nothing else; what to do next is the router's
    (ADR-354 D6). The outcomes, per answer:

    | answer | APNs | web push | outcome |
    |---|---|---|---|
    | accepted | 200 | 201, or any other 2xx | `Delivered` |
    | the device or subscription is gone | 410, reason `Unregistered` or `ExpiredToken`, with Apple's `timestamp` | 404 or 410 | `Gone { since }`, `since` from Apple's timestamp, none for web push |
    | the token or subscription is wrong | 400 `BadDeviceToken` or `DeviceTokenNotForTopic` | none | `Rejected(Token)` |
    | the body is too large | 413, or R2's and R5's checks before sending | 413 | `Rejected(TooLarge)` |
    | the signing key or its token is refused | 403 (R3's resend apart) | 401 or 403 | `Rejected(ProviderToken)` |
    | any other 4xx | any other 4xx | any other 4xx | `Rejected(Request)` |
    | try later | 429, 500, 503 | 429 or 5xx, with `Retry-After` in seconds when sent | `RetryLater { after }` |
    | nothing usable | a connection error, the deadline, a redirect, an unreadable answer | the same | `Failed(..)` |

R5. The web push sender encrypts a JSON body, `{"title":T,"body":B}`, to the subscription's P-256
    key and authentication secret as RFC 8291 says (`Content-Encoding: aes128gcm`, one record, a new
    ephemeral key per message). Plaintext over 3993 bytes is refused before any request. It posts
    to the subscription's endpoint with `TTL` (always sent, `0` included), `Urgency: normal` and,
    when the notification has a collapse key, `Topic`. Its `Authorization` is RFC 8292's
    `vapid t=<JWT>, k=<the VAPID public key, base64url>`, where the JWT is ES256 with
    `{"aud":<the endpoint's origin>,"exp":<the clock plus 12 hours>,"sub":<the contact URI>}`.
R6. A collapse key is 1 to 32 characters of the base64url alphabet, so the one value satisfies both
    `apns-collapse-id` and RFC 8030's `Topic`; anything else is refused when the notification is
    built.
R7. A web push subscription is admitted only when its endpoint's origin is on the sender's list of
    push services. An APNs origin, and every origin on that list, is `https:`, or `http:` to a
    loopback host, with no user, path, query or fragment. A refused subscription or origin is a
    typed error, and no request is made.
R8. A sender is built from its signing key's PKCS#8 PEM text, held in memory and parsed once into a
    P-256 signing key; a key that does not parse refuses the build. Production reads the key
    through the kernel's credential loader by its role (#640). No log line and no `Debug` output
    of any type in the crate carries key material, a provider or VAPID token, a device token, a
    subscription endpoint, its keys or a request path.
R9. Every request runs under a deadline the sender is built with; an error body is read up to a
    fixed bound and no further; no redirect is followed.
R10. The tests' fakes are axum servers on loopback ports that record every request (method, path,
    headers, body) and answer as each test scripts. The APNs fake speaks HTTP/2 without TLS (prior
    knowledge) and verifies the provider token's signature with the test's public key. The push
    service fake decrypts each body with the test subscription's private key and secret, through its
    own RFC 8291 decryption written in the test support, and verifies the VAPID token with `k`.
    Every key is generated in memory when a test starts; every id, topic, contact and token is
    synthetic; nothing is written to disk.
R11. The workspace's censuses hold with the crate in the tree: the one-router census finds no
    delivery around the port, the crate is named for its directory and inherits the workspace
    lints, and every log capture goes through the helper.
R12. The remote's gamepad reader takes a snapshot of each connected gamepad every animation frame
    and fires an action on a button's rising edge only. A gamepad's first snapshot fires nothing.
    Only a gamepad whose `mapping` is `"standard"` fires actions, by section 7's table; any other is
    shown with its raw button and axis indices and fires nothing. The left stick fires the d-pad's
    actions past 0.5 and re-arms below 0.25.
R13. The key reader maps section 7's keys. It fires nothing for a repeated key, a key typed while
    composing, a key whose target is a form control, a link, a button or editable content, or a key
    with Alt, or with Control or Command on any key but the flag's. A mapped key's default action is
    prevented.
R14. Grades fire only on the answer side, as in Anki. Show answer moves the review to the answer
    side; a grade moves it back to the question side.
R15. The wake lock holder wants the lock exactly while a review is open, a gamepad is connected and
    the page is visible. It requests the screen lock when that becomes true and releases it when it
    becomes false. A lock the browser releases while the page is hidden is requested again when the
    page is visible and the condition still holds. A refused request is recorded with the error's
    name and is not repeated until the condition becomes false and then true again. At most one
    request is in flight. A release event from a lock the holder no longer holds changes nothing. A
    browser with no wake lock reads `unsupported`.
R16. The harness screen, `/remote`, joins the route table. It shows each gamepad's id, mapping,
    pressed buttons and axes; each key's `key` and `code`; the review side; the page's visibility;
    the wake lock's state; and a log of the last 200 events, each with its time, source, raw input,
    action, visibility and lock state. A button opens and closes the simulated review. It makes no
    network request of its own.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | an alert reaches the fake at `/3/device/<token>` over HTTP/2 with R2's headers and body, `apns-expiration` from the manual clock, and answers `Delivered` | `cargo test -p deck-streak-push --test apns -- --exact a1_an_alert_reaches_the_fake_with_its_path_headers_and_body` |
| A2 | the provider token is an ES256 JWT with exactly R3's header and claims, which the fake verifies with the test's public key | `cargo test -p deck-streak-push --test apns -- --exact a2_the_provider_token_is_an_es256_jwt_the_fake_verifies` |
| A3 | sends 19 minutes apart carry one token; a send 45 minutes after it was minted carries a new one with the later `iat` | `cargo test -p deck-streak-push --test apns -- --exact a3_the_provider_token_is_reused_inside_its_window_and_reminted_after` |
| A4 | a 403 `ExpiredProviderToken` on a token 20 or more minutes old is answered by one resend with a new token; a second 403 answers `Rejected(ProviderToken)` after exactly two requests | `cargo test -p deck-streak-push --test apns -- --exact a4_an_expired_provider_token_is_reminted_once_and_resent` |
| A5 | a 410 `Unregistered` or `ExpiredToken` answers `Gone` with Apple's timestamp, after one request | `cargo test -p deck-streak-push --test apns -- --exact a5_a_410_reports_the_device_gone_with_its_timestamp` |
| A6 | a 400 `BadDeviceToken` or `DeviceTokenNotForTopic` answers `Rejected(Token)` after one request | `cargo test -p deck-streak-push --test apns -- --exact a6_a_refused_device_token_is_never_retried` |
| A7 | a 429, a 500 and a 503 each answer `RetryLater` after one request | `cargo test -p deck-streak-push --test apns -- --exact a7_try_later_answers_are_reported_and_not_retried` |
| A8 | a 4096-byte body is delivered and a 4097-byte one is refused `TooLarge` with no request | `cargo test -p deck-streak-push --test apns -- --exact a8_an_oversize_payload_is_refused_before_any_request` |
| A9 | a development device's notification reaches the development fake and never the production one, and the reverse | `cargo test -p deck-streak-push --test apns -- --exact a9_each_device_reaches_its_own_environment_only` |
| A10 | an origin that is `http:` to a non-loopback host, or carries a path, query, fragment or user, is refused; `https:` and loopback `http:` are admitted | `cargo test -p deck-streak-push --test origin -- --exact a10_an_origin_is_https_or_loopback_http_and_nothing_more` |
| A11 | a notification reaches the push service fake encrypted `aes128gcm`, and the fake's own decryption yields its JSON; it answers `Delivered` | `cargo test -p deck-streak-push --test web_push -- --exact a11_a_message_reaches_the_fake_encrypted_and_decrypts_to_its_json` |
| A12 | the `Authorization` header is `vapid t=…, k=…`, `k` is the test's public key, and the JWT's `aud`, `exp` and `sub` are R5's, verified with `k` | `cargo test -p deck-streak-push --test web_push -- --exact a12_the_vapid_header_carries_a_jwt_the_fake_verifies_with_k` |
| A13 | `TTL` carries the time to live (zero included), `Urgency` is `normal`, and `Topic` is the collapse key when there is one and absent when there is none | `cargo test -p deck-streak-push --test web_push -- --exact a13_ttl_urgency_and_topic_carry_the_expiry_and_the_collapse_key` |
| A14 | a 404 and a 410 each answer `Gone` after one request | `cargo test -p deck-streak-push --test web_push -- --exact a14_a_404_or_410_reports_the_subscription_gone` |
| A15 | a 429 with `Retry-After: 120` answers `RetryLater` after 120 seconds, a 503 answers `RetryLater`, each after one request | `cargo test -p deck-streak-push --test web_push -- --exact a15_try_later_answers_carry_retry_after_and_are_not_retried` |
| A16 | a subscription whose endpoint's origin is off the list is refused with no request; one on the list is admitted and delivered | `cargo test -p deck-streak-push --test web_push -- --exact a16_an_endpoint_off_the_list_is_refused_before_any_request` |
| A17 | 3993 bytes of plaintext are delivered and 3994 are refused `TooLarge` with no request | `cargo test -p deck-streak-push --test web_push -- --exact a17_an_oversize_plaintext_is_refused_before_any_request` |
| A18 | across sends reaching every outcome, the captured log holds a line per call and no device token, endpoint, request path or token | `cargo test -p deck-streak-push --test logging -- --exact a18_no_log_line_carries_a_token_an_endpoint_or_a_path` |
| A19 | each sender's and subscription's `Debug` names its type and holds no key material, token or endpoint | `cargo test -p deck-streak-push --test debug -- --exact a19_debug_holds_no_key_token_or_endpoint` |
| A20 | the one-router census over the real tree, the push crate included, finds no delivery around the port | `cargo test -p deck-streak-notifications --test one_router -- --exact no_delivery_goes_around_the_port` |
| A21 | the push crate is named for its directory and inherits the workspace lints | `cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_is_named_for_its_context_directory` and `cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_inherits_the_workspace_lints` |
| A22 | the log-capture census over the real tree, the push crate's logging test included, finds every capture going through the helper | `cargo test -p deck-streak-kernel --test log_capture_class -- --exact every_capture_in_the_workspace_goes_through_the_helper` |
| A23 | each standard-mapping button in section 7 fires its action on its rising edge only, and a gamepad's first snapshot fires nothing | `pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "each mapped button fires its action on its rising edge only"` |
| A24 | a gamepad whose mapping is not standard fires no action and reports its raw indices | `pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "a non-standard mapping fires nothing and reports its raw indices"` |
| A25 | the left stick fires the d-pad's actions past 0.5 and fires again only after returning below 0.25 | `pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "the left stick fires past its threshold and re-arms below the lower one"` |
| A26 | section 7's keys fire their actions; Space and Enter show the answer, and answer Good on the answer side | `pnpm exec vitest run web/app/src/lib/remote/keys.test.ts -t "Anki's desktop keys fire their actions"` |
| A27 | a repeated key, a composing key, a key in a control and an unmapped modifier fire nothing, and a grade on the question side fires nothing | `pnpm exec vitest run web/app/src/lib/remote/keys.test.ts -t "a repeat, a control, a modifier or the question side fires nothing"` |
| A28 | the lock is requested when a review is open, a gamepad is connected and the page is visible, and not before all three | `pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "the lock is requested when all three hold"` |
| A29 | the lock is released on disconnect and on the review's close | `pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "the lock is released on disconnect and on close"` |
| A30 | a lock released while the page is hidden, in either order of the release and the visibility change, is requested again when the page is visible | `pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "a lock released while hidden is requested again on visible"` |
| A31 | a refused request is recorded with its error's name and not repeated until the condition falls and rises again | `pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "a refusal is recorded and not repeated until the condition rises again"` |
| A32 | every state answers every event as the holder's transition table in the schematic says, each cell examined | `pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "every state answers every event as the table says"` |
| A33 | the harness shows a stubbed gamepad's id, mapping, pressed buttons and axes, a key's `key` and `code`, the side, the visibility and the lock's state, and its log keeps the last 200 events | `pnpm exec vitest run web/app/src/routes/remote.test.ts -t "the harness shows the remote, the keys and the lock"` |
| A34 | the route table is every screen, `/remote` included, so the accessibility audit covers it in both colour schemes | `pnpm exec vitest run web/app/src/lib/a11y-coverage.test.ts -t "the accessibility audit covers every route in both colour schemes"` |

```acceptance
A1: cargo test -p deck-streak-push --test apns -- --exact a1_an_alert_reaches_the_fake_with_its_path_headers_and_body
A2: cargo test -p deck-streak-push --test apns -- --exact a2_the_provider_token_is_an_es256_jwt_the_fake_verifies
A3: cargo test -p deck-streak-push --test apns -- --exact a3_the_provider_token_is_reused_inside_its_window_and_reminted_after
A4: cargo test -p deck-streak-push --test apns -- --exact a4_an_expired_provider_token_is_reminted_once_and_resent
A5: cargo test -p deck-streak-push --test apns -- --exact a5_a_410_reports_the_device_gone_with_its_timestamp
A6: cargo test -p deck-streak-push --test apns -- --exact a6_a_refused_device_token_is_never_retried
A7: cargo test -p deck-streak-push --test apns -- --exact a7_try_later_answers_are_reported_and_not_retried
A8: cargo test -p deck-streak-push --test apns -- --exact a8_an_oversize_payload_is_refused_before_any_request
A9: cargo test -p deck-streak-push --test apns -- --exact a9_each_device_reaches_its_own_environment_only
A10: cargo test -p deck-streak-push --test origin -- --exact a10_an_origin_is_https_or_loopback_http_and_nothing_more
A11: cargo test -p deck-streak-push --test web_push -- --exact a11_a_message_reaches_the_fake_encrypted_and_decrypts_to_its_json
A12: cargo test -p deck-streak-push --test web_push -- --exact a12_the_vapid_header_carries_a_jwt_the_fake_verifies_with_k
A13: cargo test -p deck-streak-push --test web_push -- --exact a13_ttl_urgency_and_topic_carry_the_expiry_and_the_collapse_key
A14: cargo test -p deck-streak-push --test web_push -- --exact a14_a_404_or_410_reports_the_subscription_gone
A15: cargo test -p deck-streak-push --test web_push -- --exact a15_try_later_answers_carry_retry_after_and_are_not_retried
A16: cargo test -p deck-streak-push --test web_push -- --exact a16_an_endpoint_off_the_list_is_refused_before_any_request
A17: cargo test -p deck-streak-push --test web_push -- --exact a17_an_oversize_plaintext_is_refused_before_any_request
A18: cargo test -p deck-streak-push --test logging -- --exact a18_no_log_line_carries_a_token_an_endpoint_or_a_path
A19: cargo test -p deck-streak-push --test debug -- --exact a19_debug_holds_no_key_token_or_endpoint
A20: cargo test -p deck-streak-notifications --test one_router -- --exact no_delivery_goes_around_the_port
A21: cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_is_named_for_its_context_directory
A21: cargo test -p deck-streak-daemon --test workspace -- --exact every_crate_inherits_the_workspace_lints
A22: cargo test -p deck-streak-kernel --test log_capture_class -- --exact every_capture_in_the_workspace_goes_through_the_helper
A23: pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "each mapped button fires its action on its rising edge only"
A24: pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "a non-standard mapping fires nothing and reports its raw indices"
A25: pnpm exec vitest run web/app/src/lib/remote/gamepad.test.ts -t "the left stick fires past its threshold and re-arms below the lower one"
A26: pnpm exec vitest run web/app/src/lib/remote/keys.test.ts -t "Anki's desktop keys fire their actions"
A27: pnpm exec vitest run web/app/src/lib/remote/keys.test.ts -t "a repeat, a control, a modifier or the question side fires nothing"
A28: pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "the lock is requested when all three hold"
A29: pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "the lock is released on disconnect and on close"
A30: pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "a lock released while hidden is requested again on visible"
A31: pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "a refusal is recorded and not repeated until the condition rises again"
A32: pnpm exec vitest run web/app/src/lib/remote/wake-lock.test.ts -t "every state answers every event as the table says"
A33: pnpm exec vitest run web/app/src/routes/remote.test.ts -t "the harness shows the remote, the keys and the lock"
A34: pnpm exec vitest run web/app/src/lib/a11y-coverage.test.ts -t "the accessibility audit covers every route in both colour schemes"
```

Every Rust test builds its keys in memory and talks only to a fake on a loopback port; no test
reaches a real push service, and no key, token or id in the tree is real. A20 to A22 judge the real
tree with the censuses' own tests, unchanged; they are green at the base and guard the crate's
arrival, so the red-first record discloses them `not red`. A34 is red at the base only once the
screen exists, so its record line is written when the route is added. The fakes' decryption and
signature checks are test support, not criteria. Tests that kill the band's rows without deciding
a criterion (a young token is not re-minted on a 403; an oversize error body is not read past its
bound; a release event from a dropped lock changes nothing) are MUTATION COVERAGE.

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml` | the workspace | the push crate as a member, and the external crates ADR-354 admits |
| `Cargo.lock` | the workspace | the new packages |
| `crates/push/Cargo.toml` | `deck-streak-push` | added |
| `crates/push/src/lib.rs` | `deck-streak-push` | added: the crate's doc, the notification, the collapse key, the outcome |
| `crates/push/src/origin.rs` | `deck-streak-push` | added: the origin rule and the push-service list (R7) |
| `crates/push/src/jwt.rs` | `deck-streak-push` | added: the ES256 signer both tokens use (R3, R5) |
| `crates/push/src/client.rs` | `deck-streak-push` | added: the HTTP client, the deadline and the bounded read (R9) |
| `crates/push/src/apns.rs` | `deck-streak-push` | added: R2 to R4 |
| `crates/push/src/web_push.rs` | `deck-streak-push` | added: R4 to R7 |
| `crates/push/tests/apns.rs` | `deck-streak-push` | added: A1 to A9 |
| `crates/push/tests/origin.rs` | `deck-streak-push` | added: A10 |
| `crates/push/tests/web_push.rs` | `deck-streak-push` | added: A11 to A17 |
| `crates/push/tests/logging.rs` | `deck-streak-push` | added: A18, through the log-capture helper |
| `crates/push/tests/debug.rs` | `deck-streak-push` | added: A19 |
| `crates/push/tests/support/mod.rs` | `deck-streak-push` | added |
| `crates/push/tests/support/fake_apns.rs` | `deck-streak-push` | added: the recording APNs fake |
| `crates/push/tests/support/fake_push_service.rs` | `deck-streak-push` | added: the recording push service fake |
| `crates/push/tests/support/rfc8291.rs` | `deck-streak-push` | added: the fake's own decryption |
| `crates/push/tests/support/keys.rs` | `deck-streak-push` | added: keys generated in memory, synthetic ids |
| `web/app/src/lib/remote/actions.ts` | `miniapp` | added: the actions and the review side (R14) |
| `web/app/src/lib/remote/mapping.ts` | `miniapp` | added: section 7's table |
| `web/app/src/lib/remote/gamepad.ts` | `miniapp` | added: snapshots and edges (R12) |
| `web/app/src/lib/remote/keys.ts` | `miniapp` | added: the key reader (R13) |
| `web/app/src/lib/remote/wake-lock.ts` | `miniapp` | added: the holder and its table (R15) |
| `web/app/src/lib/remote/log.ts` | `miniapp` | added: the bounded event log (R16) |
| `web/app/src/lib/remote/gamepad.test.ts` | `miniapp` | added: A23 to A25 |
| `web/app/src/lib/remote/keys.test.ts` | `miniapp` | added: A26, A27 |
| `web/app/src/lib/remote/wake-lock.test.ts` | `miniapp` | added: A28 to A32 |
| `web/app/src/routes/remote/+page.svelte` | `miniapp` | added: the harness screen (R16) |
| `web/app/src/routes/remote.test.ts` | `miniapp` | added: A33 |
| `web/app/src/lib/routes.ts` | `miniapp` | `/remote` joins the route table (A34) |
| `web/app/messages/en.json` | `miniapp` | the harness's labels, in the base locale |
| `scripts/mutation-rows.d/S34300-S34399.json` | none (the gate) | added: the sender's rows |
| `scripts/mutation-equivalent.d/miniapp.json` | none (the gate) | changed only if an equivalent mutant of the harness survives, each bound by its anchor |
| `docs/CONTEXT-MAP.md` | the map | the push crate's line and paragraph |
| `docs/schematics/push-senders-and-remote-harness.md` | the record | added: the push data flow, the outcome map, the harness's state machines, the CI jobs |
| `docs/red-first/SPEC-343.md` | the record | added |
| `docs/specs/SPEC-343-push-senders-reach-recording-fakes-and-a-web-harness-reads-the-remote.md` | the record | added |
| `docs/decisions/ADR-354-push-senders-build-their-own-requests-against-recording-fakes-and-the-remote-harness-reads-a-gamepad.md` | the record | added |
| `changelog.d/push-remote-343.md` | the record | added |

## 5. What this does NOT do

- It joins no sender to the router: the transport per surface, the generalised port, the
  notifications edge and the join in the composition root are native push on the one router's
  work (#640).
- It stores no device token or subscription, offers no opt-in or revoke, and exports and erases
  nothing; a stored token per device, its removal on `Gone`, and the data-rights census are #640's.
- It changes nothing in `notifications-policy.json`: the surfaces stay the bot and the Mini App,
  and the nudge budget across every transport, at most one a day, is #640's (#640).
- It wires no credential: no unit line loads an APNs or VAPID key, and nothing reads the
  credentials directory in the push crate (#640, with the deploy in #628).
- It names no production push service: the list R7 checks is given to the sender when it is
  built, the tests give it their fakes' origins, and the list production builds the sender with is
  #640's. Until then an origin not on the list is refused (#640).
- It sends nothing to a real push service, holds no real device token and places no key: a push
  received from the service on iPhone, iPad and a browser, with the owner's own keys, is the
  owner's first device session (#629).
- It gives the device session no sending command: a one-shot send that reads the owner's key is
  production wiring, and it is built for that session or with #640 (#629).
- It writes no Swift: registering for remote notifications, the `aps-environment` entitlement per
  build, the GameController reader, key commands and the idle timer are the iPad layouts and
  remote work and the device session (#633, #629).
- It adds no service worker, no push subscription in the browser and no notification display (#640),
  and no web study screen: the actions here drive the harness's simulated review alone, and the web
  client's review takes them from these modules (#630).
- It persists no mapping and builds no mapping screen; the defaults in section 7 are the harness's
  (#630 on the web, #633 on iPad).
- It measures nothing on a device: whether the remote's keyboard mode reaches the page, whether
  gamepad input arrives while the screen is dimmed or the page is hidden, and whether a browser
  takes a key such as Control-1 before the page sees it are read from the harness's log in the
  owner's first device session (#629).
- It holds no wake lock for the remote's keyboard mode, which a page cannot tell from a keyboard:
  ADR-342's web clause names a connected gamepad, and the web study screens decide the keyboard
  case after the device session (#630, #629).
- It changes no response header, permissions policy or edge configuration; the shipped surfaces'
  security re-check reads them (#638).
- It retires neither the bot nor the Mini App (#612, #63).

## 6. Risks

- **The fakes agree with the senders on a wrong reading of a protocol.** One builder writes both.
  Detected in part by independence: the push service fake decrypts with its own RFC 8291
  decryption, not the library's, and the APNs fake parses the JWT itself; detected fully by the
  owner's first device session, where a real service accepts or refuses (#629).
- **A platform refuses a push without saying so.** ADR-341's "what would make this wrong". The
  outcome table maps every documented answer, and an answer it does not know is `Failed`, never
  `Delivered`; detected by A4 to A7 and A14 to A15, and on a device by #629.
- **Apple changes a reason string or the token rules.** The mapping is by status first and reason
  second, so an unknown reason under a known status still lands in the status's row; detected by
  the device session.
- **The new dependencies widen the tree.** HTTP/2, P-256 and RFC 8291 encryption arrive with the
  crate. Detected by `cargo deny` (licences and advisories) and its duplicate-version warning.
- **A browser takes a mapped key before the page.** Control-1 and Command-1 select a tab in some
  browsers. Detected by the harness's log in #629; the mapping screen picks another default (#630).
- **The wake lock is refused inside Telegram's web frame**, where the permissions policy is the
  embedder's. Detected by the harness, which records the refusal's name; the harness is opened in
  a browser tab for the device session.
- **The root layout's requests fail outside Telegram** when the harness is opened in a browser tab.
  They fail closed, as the layout does today in a plain tab, and the harness depends on none of them.
- **Gamepad events stop while the page is hidden or dimmed** (ADR-342's "what would make this
  wrong"). Detected by the log's visibility and lock columns in #629.

## 7. The remote's default mapping

The gamepad column is the W3C Gamepad standard mapping; the keys column is Anki's desktop keys
(its reviewer's shortcuts). A non-standard mapping fires nothing (R12). Button positions name the
standard layout: the right cluster's bottom, right, left and top face buttons are 0, 1, 2 and 3,
whatever letter a remote prints on them.

| action | gamepad (standard mapping) | keys |
|---|---|---|
| show answer (question side) | button 0 or 1, the bottom or right face button | Space, Enter |
| Again | button 14, d-pad left; left stick left | 1 |
| Hard | button 13, d-pad down; left stick down | 2 |
| Good | button 15, d-pad right; left stick right; on the answer side, button 0 or 1 | 3; on the answer side, Space or Enter |
| Easy | button 12, d-pad up; left stick up | 4 |
| undo | button 4, the top left front button | u |
| bury (the card) | button 5, the top right front button | - |
| flag (red) | button 3, the top face button | Control-1, or Command-1 |
| replay audio | button 2, the left face button | r |

Both face buttons show the answer because remotes disagree on which of them confirms. Anki's other
replay key, F5, is left to the browser, which reloads the page on it (ADR-354 D9). Every other
button, axis and key fires nothing and is shown raw.

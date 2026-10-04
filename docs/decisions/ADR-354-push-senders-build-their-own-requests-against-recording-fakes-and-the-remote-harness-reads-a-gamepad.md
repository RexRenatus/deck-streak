---
status: proposed
decision-makers: the owner, the DeckStreak architect
---

# Push senders build their own requests against recording fakes, in an adapter crate the daemon does not yet compose, and the remote harness reads a standard gamepad and Anki's keys

## Context and Problem Statement

SPEC-343 is the app campaign's push and remote spike (#621; SPEC-334 row 1.2, R12 and R15).
ADR-341 decides that APNs and web push become the one router's transports, opt-in and revocable
per device, with the keys the owner's to place; ADR-342 decides that the 8BitDo remote drives study
on the web through the Gamepad API and keyboard events, Anki's desktop keys included, with a screen
wake lock while a gamepad is connected during a review. Neither decides how the service builds a
request a platform accepts, how CI observes that request, where the code sits in the context map,
how a key reaches it, what a sender does with each answer, or which button and key fire which
action by default. This ADR decides those, inside ADR-341 and ADR-342.

Measured before deciding: the workspace has no sender and no input code (SPEC-343 section 1);
`Cargo.lock` compiles no HTTP/2 and no P-256; the two APNs crates read, `a2` and `apns-h2`, each
take their endpoint from an enum holding Apple's two services alone, with no URL, connector or root
store to give; and the bot's transport and fake already set the shape a fake needs: a base URL that
is `https:` or loopback `http:` (`crates/bot/src/transport.rs:60`), and a recording axum server on
a loopback port (`crates/bot/tests/support/fake_bot_api.rs:2`).

## Decision Drivers

- CI must observe what a platform would receive: the path, every header, the provider or VAPID
  token, and the body, encrypted where the protocol encrypts it. A test that sees only a value
  handed to a library proves nothing a platform checks.
- The crate graph is the context map: the senders add no edge into a context, and the router's port
  stays #640's to generalise (ADR-341).
- No key, token or device id enters the repository, real or real-shaped; no test writes key
  material to disk; a secret never travels in the environment.
- ADR-341's "what would make this wrong": a refused push must reach the router's ledger as a
  refusal, and a gone device as gone.
- One place decides retries, so attempts do not multiply across layers.
- The harness's code is what the web study screens reuse, and is held by the web mutation run.

## Considered Options (the alternatives it was chosen against)

The APNs crate was to be chosen on maintenance, its HTTP/2 client, token-based authentication and
its licence against `deny.toml`. Both crates are MIT, both speak HTTP/2 through hyper, and both
sign provider tokens; `apns-h2` is the maintained fork of `a2` and is on the workspace's TLS stack.
A fifth criterion decided it: whether a test can address a fake.

- D1, the crate builds the APNs request itself, over hyper's pooled HTTP/2 client with a rustls
  connector, to origins it is configured with - chosen, because CI can then point it at a loopback
  fake and observe every header, the token and the body, and the request is a page of code over
  Apple's documented shape.
- D1, `apns-h2` - lost: its endpoint is an enum of Apple's two services with no URL, connector or
  root store, so no test can address a fake. It is the runner-up, and becomes the choice if it ever
  accepts an endpoint a test can give.
- D1, `a2` - lost: the same enum, and it brings an older TLS stack than the workspace's, so the lock
  would carry two.
- D1, either crate behind a trait, faked at the trait - lost: the fake would see a notification
  value and never the path, the headers or the token, which are what the spike exists to prove.
- D2, `web-push-native` for RFC 8291's encryption, its own `vapid` feature off, the request sent
  through D1's client and its VAPID token signed by D1's signer - chosen, because it is pure
  RustCrypto on hash and key-derivation crates the workspace already locks, it returns an
  `http::Request` any client can send, and it is MIT or Apache-2.0.
- D2, the `web-push` crate - lost: its clients are an older hyper over a native TLS library, or a
  libcurl client, each a second HTTP and TLS stack in the workspace.
- D2, RFC 8291 written in the crate - lost: more cryptography to own where a library already
  returns the encrypted request; the fake's own decryption (D5) is the independent check.
- D2, `web-push-native`'s `vapid` feature - lost: it brings a general JWT library for one token
  the crate already signs for APNs, with the same curve and algorithm.
- D3, a new adapter crate, `deck-streak-push`, depending on the kernel alone, which the daemon does
  not compose until #640 - chosen, because it adds no edge into a context, holds both senders'
  shared client, signer, origin rule and outcome once, and leaves the router's port to #640, as
  `deck-streak-ffi` stands uncomposed beside the root (ADR-345 D1).
- D3, inside `deck-streak-notifications` - lost: the context owns no transport by its own
  definition (`crates/notifications/src/lib.rs:8`), and it would gain an HTTP, TLS and
  cryptography stack.
- D3, inside `deck-streak-api` - lost: `api` is the inbound adapter for the Mini App and depends on
  coordination; an outbound client to two platforms is another adapter.
- D3, inside `deck-streak-bot` - lost: the bot retires once native push carries the router
  (ADR-341).
- D3, one crate per platform - lost: the two would duplicate the client, the signer, the origin
  rule and the outcome, or need a third crate to share them.
- D3, the push crate depending on notifications now and implementing a router port - lost: ADR-341
  says the port generalises from the bot's to a transport per surface, which is #640's design; a
  spike that implemented today's `BotTransport` would decide that shape early.
- D4, a sender is built from its signing key's PKCS#8 PEM text, parsed once into a P-256 key held
  in memory; production reads that text through the kernel's credential loader, by role (the APNs
  signing key, the VAPID signing key), from a unit's `LoadCredential=` (#640); the key id, team id,
  topic and contact are configuration from the private deploy rail (SPEC-334 R20) - chosen, because
  tests then generate keys in memory and write nothing, and production keeps the workspace's one
  credential path.
- D4, a key file's path in the constructor - lost: every test would write a key to disk.
- D4, the kernel's `Secret` in the constructor - lost: only the credential loader makes one, from a
  file, so every test would write a key to disk too; production passes the secret's text instead.
- D4, an environment variable - lost: systemd's documentation says the environment is not for
  secrets, and the workspace refuses a secret read from it.
- D4, a raw base64url scalar for the VAPID key - lost: a second key format and parser; PKCS#8 PEM,
  the form Apple issues the APNs key in, serves both.
- D5, the fakes are axum servers on loopback ports that record every request and answer as each
  test scripts: the APNs fake over HTTP/2 without TLS (prior knowledge), the push service fake over
  HTTP/1.1; the APNs fake verifies the provider token's signature, and the push service fake
  decrypts each body with its own RFC 8291 decryption, written in the test support from the RFC,
  and verifies the VAPID token with `k` - chosen, because they observe what a platform would
  receive, in the shape the bot's fake set.
- D5, a fake at a trait seam - lost: it sees no header, token or ciphertext.
- D5, TLS fakes with a test root and a host override - lost: production code would carry a
  test-only trust root and name resolution path.
- D5, a mock-server crate - lost: a new dev-dependency for what axum, already in the workspace,
  serves.
- D5, decrypting with the library's own decryption - lost: a key-schedule mistake shared by its
  encryption and its decryption would pass.
- D5, committing RFC 8291's published example keys to prove the fake's decryption - lost: a private
  key's shape in the public tree, which the secrets scan and review refuse; agreement between two
  independent implementations, and the owner's first device session (#629), check it instead.
- D6, the sender reports one typed outcome per call and retries nothing, apart from one resend with
  a new provider token after `ExpiredProviderToken` on a token at least 20 minutes old - chosen,
  because the router already decides what a failed send becomes (a celebration held, a nudge's
  claim released, behind its outage breaker), and Apple and push services already store a message
  and retry its delivery until its expiry; the age check and the mint under one lock, and a
  refused token replaced only while it is still current, so concurrent refusals mint one token.
- D6, waits and attempts inside the sender, as the bot's transport has - lost: the router's attempts
  would multiply them, and waits inside a sender need a recorder in every test because tokio's paused
  time cannot hold a client's own timeout (ADR-026).
- D6, no resend on an expired provider token - lost: the token is the sender's own state, which no
  router decision can renew.
- D7, a subscription is admitted only when its endpoint's origin is on the sender's list of push
  services - chosen, because the endpoint is text a client sends, and the sender is the last place
  every caller passes before the request. The list is given to the sender when it is built and an
  origin it does not hold is refused, so a sender built with no list refuses every endpoint; the
  list production builds it with is #640's.
- D7, any `https:` endpoint - lost: the service would post to any URL a signed-in client names, a
  server-side request forgery.
- D7, checking only where #640 stores a subscription - lost: a caller that reaches the sender
  another way would pass unchecked; #640 may check there as well.
- D8, the harness's logic as pure modules under `web/app/src/lib/remote/`, and one thin screen,
  `/remote`, in the Mini App's route table - chosen, because the web study screens reuse the
  modules, node tests reach them with the browser's APIs stubbed, StrykerJS mutates them, and the
  accessibility audit visits the screen.
- D8, a package of its own - lost: a new workspace member with its own build, test, CI and mutation
  configuration, for a screen the web client replaces.
- D8, a static HTML page outside the app - lost: no test, no mutation run and no audit reach it.
- D9, the defaults of SPEC-343 section 7: the W3C standard mapping only, grades on the answer
  side only, both lower face buttons showing the answer, the left stick as the d-pad, and the lock
  wanted while a review is open, a gamepad is connected and the page is visible, requested again
  whenever that becomes true - chosen, because each default is the standard's or Anki's, and each
  can be changed by the mapping screen (#630, #633) without changing the reader.
- D9, guessing a non-standard mapping's indices - lost: they differ by remote, mode and browser, and
  a wrong guess grades a card; the harness shows them raw for the device session instead.
- D9, F5 for replay, as Anki's desktop has it - lost: a browser reloads the page on it, ending the
  review. This departs from ADR-342's web clause, which maps Anki's desktop keys: F5 alone is left
  to the browser, and `r`, Anki's other replay key, carries the action.
- D9, one confirm button - lost: remotes disagree on whether the bottom or the right face button
  confirms.
- D9, a lock requested once per review - lost: a browser releases the lock whenever the page is
  hidden, so a review after a tab switch would dim.
- D9, retrying a refused lock on a timer - lost: a refusal is a permission or policy answer that
  retrying does not change; it is recorded for the device session.

## Decision Outcome

Proposed options: D1 the crate's own APNs request over hyper's HTTP/2 client; D2 `web-push-native`
for encryption, with the crate's own VAPID token; D3 one adapter crate on the kernel, uncomposed
until #640; D4 keys as PEM text in memory, through the kernel's credential loader in production;
D5 recording loopback fakes that verify tokens and decrypt independently; D6 one typed outcome per
call, retries left to the router; D7 the push-service list; D8 pure modules and one harness screen;
D9 the standard mapping, Anki's keys and the visible-only lock. Together they let CI observe every
byte a platform would judge, keep every key out of the tree and off the disk, add no edge into a
context, and hand #640 a sender whose every answer is typed.

The context map gains, inside its fence:

```
deck-streak-push          (APNs and web push senders for a native or web client: provider and VAPID tokens, RFC 8291; ADR-354)  depends on: kernel
```

and, after the FFI adapter's paragraph: "`deck-streak-push` is an outbound adapter: it builds APNs
and web push requests and reports what each platform answered, and it depends on the kernel's clock
alone (SPEC-343, ADR-354 D3). The daemon does not compose it until native push carries the
one router (#640), which adds its notifications edge and the join in `crates/daemon/src/wiring.rs`."

### Consequences

- Good, because CI holds the request's path, headers, token claims and signature, the ciphertext's
  plaintext and every answer's outcome, against fakes no test can confuse with a platform.
- Good, because `Gone` carries Apple's timestamp, so #640 can drop a device token only when the
  token was registered before it, as Apple's documentation directs.
- Good, because one ES256 signer serves both tokens, so the curve, the encoding and the signature
  form are written and tested once.
- Bad, because the crate owns APNs's request shape, which a crate would otherwise own; a change in
  Apple's API is this repository's to follow, and the device session (#629) is where a change
  shows first.
- Bad, because the fakes and the senders are written by one builder, so a shared misreading of a
  protocol passes CI; the independent decryption narrows it, and #629 closes it.
- Bad, because the workspace gains HTTP/2, P-256 and RFC 8291 encryption; `cargo deny` judges each.
- Good, because the harness's modules are the web client's input layer from the start, held by
  StrykerJS at break 100.
- Bad, because the harness is a route in the Mini App, so the root layout's requests run beside it
  and fail closed in a browser tab, and the screen is reachable inside Telegram, where the
  embedder's permissions policy may refuse the lock.

### Confirmation

SPEC-343 A1 to A9 hold the APNs request, its token and every answer; A10 the origin rule; A11 to
A17 the web push request, its encryption, its VAPID token, its headers and every answer; A18 and
A19 that nothing secret or identifying is logged or printed; A20 to A22 the workspace's censuses;
A23 to A33 the remote's reader, the key reader, the wake lock holder and the screen; A34 the route
table. The band's rows prove each behaviour is observed. `cargo deny` holds the new crates'
licences and advisories.

## What would make this wrong

- A platform refuses a request the fakes accept. The device session (#629) reads it; D1 and D5 are
  revisited with the platform's answer as the oracle.
- `apns-h2` gains an endpoint a test can give. D1 then weighs a maintained crate against owning
  the request.
- A remote's gamepad mode reaches the page with a non-standard mapping in every browser the owner
  uses. D9's "fires nothing" then leaves the web without gamepad mode, and the mapping screen
  must map raw indices (#630).
- A browser releases the lock while the page stays visible. D9's re-request on visible would not
  restore it, and the holder needs another trigger.

## More Information

SPEC-343; SPEC-334 (R12, R15, row 1.2); ADR-341; ADR-342; ADR-345 for the uncomposed adapter;
ADR-026 for waits in tests; Apple's APNs provider documentation (sending notification requests,
handling notification responses, establishing a token-based connection); RFC 8030 (generic event
delivery using HTTP push), RFC 8188 (encrypted content-encoding), RFC 8291 (message encryption for
web push) and RFC 8292 (VAPID); the W3C Gamepad and Screen Wake Lock specifications; Anki's manual
and its reviewer's shortcuts.

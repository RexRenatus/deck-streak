# Passkey sign-in on the web: components and data flows (SPEC-359, ADR-370)

Kinds: component, data flow (as sequences), and the session proof's state machine. Every
`path:line` below was read at `3fe90969b3927cac1c3630c83a582f9d9c796d66`. Names are the ones the
SPEC and the code use: a session's proof is `telegram`, `link` or `linked`; a refusal is its
reason code.

## 1. Components

```mermaid
flowchart LR
  subgraph browser["the owner's browser, outside Telegram"]
    page["web client: link page, sign-in page, sign-in methods screen"]
    auth["platform authenticator: navigator.credentials.create and get"]
    page -->|"options, user gesture"| auth
    auth -->|"signed response"| page
  end
  subgraph tg["Telegram"]
    mini["Mini App: the same web client, framed"]
  end
  subgraph edge["the edge"]
    caddy["reverse proxy: the static client, /api proxied, security headers"]
  end
  subgraph api["deck-streak-api"]
    sess["session_routes.rs: the handshake, log out, StateChange, the handshake bound"]
    link["linking_routes.rs: link code, redeem, ceremonies, methods, ceremony bound, ceremony cookie"]
    owner["owner routes: OwnerSession admits telegram and linked"]
  end
  subgraph identity["deck-streak-identity"]
    sessions["session.rs: Sessions, the proof, OwnerSession"]
    linking["linking.rs: codes in memory as SHA-256, cap 8, 600 s"]
    passkeys["passkeys.rs: the relying party, ceremonies in memory, cap 8, 300 s, counter rule"]
    rights["data_rights.rs: passkeys exported and erased"]
  end
  db[("SQLite: passkeys")]
  coord["deck-streak-coordination: the data-rights registry"]
  mini -->|"POST code, then the link opener with the code in the fragment"| caddy
  page -->|"same-origin JSON, the two cookies"| caddy
  caddy --> sess
  caddy --> link
  caddy --> owner
  link --> sessions
  link --> linking
  link --> passkeys
  sess --> sessions
  owner --> sessions
  passkeys -->|"compare-and-swap on the counter"| db
  rights --> db
  coord --> rights
  subgraph ci["CI gates"]
    rust["rust: cargo test, clippy, the offline query cache"]
    web["web: vitest, svelte-check"]
    mut["mutation-plan, mutation-rust, mutation-rows, mutation-web, mutation-verdict"]
    hyg["hygiene: the censuses, the red-first record"]
  end
  identity -.-> rust
  api -.-> rust
  page -.-> web
  db -.-> rust
```

- The web client and the Mini App are one application on one origin
  (`web/app/svelte.config.js:24-33`, `deploy/caddy/deck-streak.caddy:46-52,89-93`), so the
  relying party ADR-132 names covers both. A ceremony runs only at top level outside Telegram:
  the edge declares no Permissions-Policy (`deploy/caddy/deck-streak.caddy:11-21`), so WebAuthn's
  default `self` allowlist holds there, and Telegram's frame is granted none.
- The page's `connect-src` is `'self'` (`web/app/svelte.config.js:30`). The ceremonies add no
  outbound request: the API verifies against its own configured origin and reaches no network.
- Identity depends on the kernel alone (`docs/CONTEXT-MAP.md:15`); `webauthn-rs`, `uuid` and
  `sqlx` are external crates, so no context edge is added.

## 2. A session's proof

```mermaid
stateDiagram-v2
  [*] --> telegram: handshake with valid initData
  [*] --> link: redeem of a live link code
  [*] --> linked: verified passkey assertion
  telegram --> link: redeem ends the session and opens a link session
  link --> linked: passkey sign-in ends it and opens a linked session
  telegram --> linked: passkey sign-in in the same browser
  linked --> telegram: handshake ends it and opens a telegram session
  telegram --> [*]: idle 30 min, absolute 8 h, log out
  link --> [*]: 600 s from the redeem, log out
  linked --> [*]: idle 30 min, absolute 8 h, log out, or its passkey removed
```

| proof | admitted by | mints a link code | removes a method | registers a passkey |
|---|---|---|---|---|
| `telegram` | `OwnerSession`, the methods routes | yes, at most 300 s after its handshake | yes, at most 300 s after its handshake | no |
| `link` | the linking routes alone | no | no | yes |
| `linked` | `OwnerSession`, the methods list | no (`reauth_required`) | no (`reauth_required`) | no |

Today a live session holds a digest, its owner and two instants (`crates/identity/src/session.rs:117-121`);
the proof is the field this delivery adds. Every transition that opens a session ends the one the
request arrived with first, as the handshake already does (`crates/api/src/session_routes.rs:128-155`).

## 3. Registration from a signed-in session

```mermaid
sequenceDiagram
  participant M as Mini App, telegram session
  participant A as linking_routes
  participant L as linking.rs
  participant B as browser, link page
  participant P as passkeys.rs
  participant K as authenticator
  participant S as passkeys table
  M->>A: POST /api/link/code, same origin, JSON
  A->>L: mint, if the handshake is at most 300 s old
  L-->>A: code, kept only as SHA-256, 600 s, cap 8
  A-->>M: the code, once
  M->>B: link opener, the code in the URL fragment
  B->>B: read the fragment, then clear it
  B->>A: POST /api/link/redeem, the code in the body
  A->>L: redeem once
  A-->>B: session cookie, proof link
  B->>A: POST /api/passkeys/register/start, after a tap
  A->>P: options: UV required, attestation none, the held or a fresh handle, the held ids excluded
  P-->>A: state kept under a flow id, recording this link session
  A-->>B: options and the ceremony cookie, Strict, 300 s
  B->>K: create
  K-->>B: new credential, user verified
  B->>A: POST /api/passkeys/register/finish, the ceremony cookie
  A->>P: take the ceremony out, check its age and its session, verify origin, RP, UV
  P->>S: insert for the owner's Telegram user id, unless the credential id is present
  A-->>B: 201 and the row id, the ceremony cookie cleared
```

## 4. A passkey sign-in

```mermaid
sequenceDiagram
  participant B as browser, sign-in page
  participant A as linking_routes
  participant P as passkeys.rs
  participant K as authenticator
  participant S as passkeys table
  participant X as Sessions
  B->>A: POST /api/passkeys/sign-in/start
  A->>A: cross-site bound, then the ceremony bound, before the body
  A->>P: the owner's passkeys
  P->>S: select the owner's rows
  P-->>A: no row is not_linked, else options over the owner's credential ids
  A-->>B: options and the ceremony cookie
  B->>K: get
  K-->>B: assertion, user verified
  B->>A: POST /api/passkeys/sign-in/finish
  A->>P: take the ceremony out, check its age, verify origin, RP, UV, signature
  P->>P: counter rule: both zero, or presented above stored
  P->>S: update counter, backup state, credential, last use, where the counter still equals the read
  A->>X: end the arriving session, open one with proof linked and its row id
  A-->>B: session cookie, the accepted ids and handle for the signal
  B->>B: signal the accepted credentials, where the browser offers it
```

## 5. A refused doubtful assertion

```mermaid
flowchart TD
  start["POST /api/passkeys/sign-in/finish"] --> cs{"cross-site or not JSON"}
  cs -->|yes| r403["403 cross_site_request or not_json"]
  cs -->|no| cer{"ceremony cookie names a live ceremony"}
  cer -->|no| r1["401 challenge_invalid"]
  cer -->|"yes, taken out of the store"| age{"younger than 300 s"}
  age -->|no| r2["401 challenge_expired"]
  age -->|yes| known{"credential id is a stored row"}
  known -->|no| r6["401 not_linked"]
  known -->|yes| own{"row's Telegram user id is the owner's"}
  own -->|no| r7["403 not_owner"]
  own -->|yes| org{"origin and RP id are the configured ones"}
  org -->|no| r3["401 origin_mismatch"]
  org -->|yes| uv{"user verified flag set"}
  uv -->|no| r4["401 uv_required"]
  uv -->|yes| sig{"signature verifies"}
  sig -->|no| r5["401 passkey_invalid"]
  sig -->|yes| ctr{"both zero, or presented above stored"}
  ctr -->|no| r8["401 counter_regressed"]
  ctr -->|yes| cas{"update matched the counter read"}
  cas -->|no| r8
  cas -->|yes| ok["200, linked session"]
```

Every refusal writes nothing, opens no session and logs its reason code with no value of R13's
list. The ceremony is already out of the store, so the same response replayed meets
`challenge_invalid`.

## 6. A passkey removal

```mermaid
sequenceDiagram
  participant M as Mini App, telegram session
  participant A as linking_routes
  participant L as linking.rs
  participant S as passkeys table
  participant X as Sessions
  M->>A: GET /api/identities
  A-->>M: Telegram first, then each passkey's row id, created, last used
  M->>A: DELETE /api/identities/{id}, same origin
  A->>L: remove, if the telegram handshake is at most 300 s old
  L->>L: the Telegram method is last_method, an unknown id is identity_unknown
  L->>S: delete the row
  L->>X: end every linked session this row opened
  A-->>M: 204
```

Recovery when every passkey is lost is this flow: the owner signs in through Telegram, the
primary identity (ADR-006), removes the lost passkeys, and registers a new one by §3.

## 7. The web client's half: components (SPEC-385, ADR-399)

Kinds for §7 to §11: component, sequence and state machine. Every `path:line` in them was read at
`e7ecf10d6b796eb1f86fe6544e04a96a0583c791`. The server's boxes are §1's, unchanged; the new boxes
are the web client's. The surface test is `telegram.launchData !== null`
(`web/app/src/lib/telegram.svelte.ts:95`), never `telegram.inside` (`telegram.svelte.ts:92`),
because `inside` names whether Telegram's script ran (on every page at `e7ecf10d`, only on a
launch since SPEC-400), never whether the page holds the launch data the handshake reads.

```mermaid
flowchart LR
  subgraph frame["Telegram's frame: no WebAuthn granted"]
    MS["/sign-in-methods, SignInMethods.svelte"]
  end
  subgraph browser["the system browser, the relying party's origin"]
    LP["/link"]
    SP["/signin"]
    SH["the shell, +layout.svelte"]
  end
  PK["passkeys.ts: codec, ceremony posts, refusal map"]
  API["api.ts: owner calls, session gate"]
  AUTH["the authenticator"]
  SRV["linking_routes, then linking.rs and passkeys.rs"]
  MS -- "mint, list, remove, sent once" --> PK
  MS -- "link opener, the code in the fragment" --> LP
  LP --> PK
  SP --> PK
  PK -- "create or get" --> AUTH
  PK -- "same-origin JSON posts" --> SRV
  SH --> API
  API -- "the session cookie" --> SRV
  API -- "a 401 outside Telegram asks for sign-in" --> SP
```

The client holds no challenge beyond one call, never reads the flow id or a session id (both
HttpOnly cookies, `crates/api/src/linking_routes.rs:60`, `crates/identity/src/session.rs:36`), and
writes nothing to browser storage.

## 8. Registration, from the owner's tap in the Mini App to a passkey on the owner's account

```mermaid
sequenceDiagram
  actor O as owner
  participant M as Mini App, sign-in methods screen
  participant T as Telegram's link opener
  participant B as browser, link page
  participant C as passkeys.ts
  participant A as linking_routes
  participant K as authenticator
  O->>M: tap Link a passkey
  M->>A: POST /api/link/code, sent once, telegram session
  alt the handshake is more than 300 s old
    A-->>M: 401 reauth_required
    M-->>O: methods_reopen, no launch data posted again
  else linking is off
    A-->>M: 404 linking_off
    M-->>O: passkey_off
  else minted
    A-->>M: the code, once
    M->>T: open the page origin's /link, the code in the fragment
    T->>B: the system browser opens the link page
  end
  B->>B: read the code from the fragment, check for WebAuthn
  alt no code
    B-->>O: link_no_code
  else no WebAuthn in this browser
    B-->>O: link_open_in_browser, the code unspent
  else ready
    O->>B: tap Create a passkey
    B->>C: redeem
    C->>A: POST /api/link/redeem, the code in a JSON body
    alt spent or expired
      A-->>C: link_code_invalid or link_code_expired
      C-->>B: link_code_spent, the fragment cleared
    else redeemed
      A-->>C: 204, session cookie with proof link
      C-->>B: clear the fragment
      C->>A: POST /api/passkeys/register/start, an empty JSON body
      A-->>C: options under publicKey, ceremony cookie, Strict, 300 s
      C->>C: refuse unless rp.id is the page's host, then decode to bytes
      C->>K: create
      alt the owner cancels, or no grant
        K-->>C: NotAllowedError
        C-->>B: passkey_cancelled, a new start inside the link session
      else created
        K-->>C: attestation and client data
        C->>A: POST /api/passkeys/register/finish, once, the server's JSON shape
        alt refused
          A-->>C: 401 challenge_invalid or challenge_expired, or 409 already_linked, or another refusal
          C-->>B: passkey_start_again, link_already or passkey_refused, never posted again
        else stored
          A-->>C: 201 and the row id, the ceremony cookie cleared
          C-->>B: link_done
        end
      end
    end
  end
```

## 9. A passkey sign-in in the browser, to a `linked` session

```mermaid
sequenceDiagram
  actor O as owner
  participant S as browser, sign-in page
  participant C as passkeys.ts
  participant A as linking_routes
  participant K as authenticator
  O->>S: tap Sign in with a passkey
  S->>C: sign in
  C->>A: POST /api/passkeys/sign-in/start, an empty JSON body
  alt no passkey is linked
    A-->>C: 401 not_linked
    C-->>S: signin_not_linked, the authenticator is not asked
  else too many ceremonies, or linking off
    A-->>C: 429 too_many_ceremonies or 404 linking_off
    C-->>S: passkey_too_many or passkey_off
  else started
    A-->>C: options over the owner's credentials, ceremony cookie
    C->>C: refuse unless rpId is the page's host, then decode to bytes
    C->>K: get
    alt the owner cancels, or no grant
      K-->>C: NotAllowedError
      C-->>S: passkey_cancelled
    else asserted
      K-->>C: authenticator data, client data, signature, user handle
      C->>A: POST /api/passkeys/sign-in/finish, once, the server's JSON shape
      alt refused
        A-->>C: 401 challenge_invalid, challenge_expired, origin_mismatch, uv_required, passkey_invalid, counter_regressed, or 403 not_owner
        C-->>S: passkey_start_again or passkey_refused, no session, never posted again
      else signed in
        A-->>C: 200 accepted credential ids and user handle, session cookie with proof linked
        C->>C: signal the accepted credentials, only where the browser offers it
        C-->>S: go to Today at the fixed path /
      end
    end
  end
```

## 10. The session gate outside Telegram

```mermaid
flowchart TD
  call["a screen's owner call, api.ts"] --> where{"launch data present?"}
  where -- "yes, inside Telegram" --> tg["handshake and one renewal, unchanged"]
  where -- "no, a plain browser" --> open{"route is /link or /signin?"}
  open -- "yes" --> none["the shell makes no owner call"]
  open -- "no" --> send["send with the session cookie, no handshake"]
  send --> answer{"status"}
  answer -- "2xx" --> ok["ok"]
  answer -- "401" --> ask["ask for sign-in once: open /signin, answer reopen"]
  answer -- "other" --> unavailable["unavailable"]
```

The server decides every session: `OwnerSession` admits `telegram` and `linked`
(`crates/identity/src/session.rs:125-132`), a `link` session only registers, and no command with
stakes is open to a `linked` session (SPEC-359 R14).

## 11. The link page and the sign-in page, as state machines with every refusal

```mermaid
stateDiagram-v2
  state "reading the fragment" as Reading
  state "no code" as NoCode
  state "no WebAuthn, code unspent" as NoWebAuthn
  state "ready to create" as Ready
  state "redeeming" as Redeeming
  state "code spent" as Spent
  state "registering" as Registering
  state "refused, start again" as Again
  state "refused" as Refused
  state "already linked" as Already
  state "done" as Done
  [*] --> Reading
  Reading --> NoCode: empty fragment
  Reading --> NoWebAuthn: no credential API
  Reading --> Ready: code and credential API
  Ready --> Redeeming: tap
  Redeeming --> Spent: link_code_invalid or link_code_expired
  Redeeming --> Registering: 204, link session
  Registering --> Again: NotAllowedError, challenge_invalid, challenge_expired
  Again --> Registering: tap, a new start
  Registering --> Spent: no_session after the link session ends
  Registering --> Already: already_linked
  Registering --> Refused: other relying party, origin_mismatch, uv_required, passkey_invalid, too_many_ceremonies, linking_off
  Registering --> Done: 201
```

```mermaid
stateDiagram-v2
  state "offering sign-in" as Offer
  state "no WebAuthn" as NoAuthn
  state "starting" as Starting
  state "not linked" as NotLinked
  state "asserting" as Asserting
  state "refused, start again" as Again
  state "refused" as Refused
  state "signed in, Today" as Today
  [*] --> Offer
  [*] --> NoAuthn: no credential API
  Offer --> Starting: tap
  Starting --> NotLinked: not_linked
  Starting --> Refused: too_many_ceremonies, linking_off, other relying party
  Starting --> Asserting: options
  Asserting --> Again: NotAllowedError, challenge_invalid, challenge_expired
  Again --> Starting: tap, a new start
  Asserting --> Refused: not_owner, origin_mismatch, uv_required, passkey_invalid, counter_regressed
  Asserting --> Today: 200, linked session
```

Every refused state leaves the page where it is: no navigation, no signal and no session. A finish
is posted once per start; a new attempt is a new start, and the server's ceremony store has already
taken the old one, so a replayed response meets `challenge_invalid` (§5 above). Two tabs that each
start a ceremony share one ceremony cookie, so the earlier tab's finish meets a refusal and offers a
new start; `formal/tla/PasskeyOnce/PasskeyOnce.tla` already holds that any request task may take any
id and that a take of an id no live entry has changes nothing.

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
  age -->|yes| org{"origin and RP id are the configured ones"}
  org -->|no| r3["401 origin_mismatch"]
  org -->|yes| uv{"user verified flag set"}
  uv -->|no| r4["401 uv_required"]
  uv -->|yes| sig{"signature verifies"}
  sig -->|no| r5["401 passkey_invalid"]
  sig -->|yes| known{"credential id is a stored row"}
  known -->|no| r6["401 not_linked"]
  known -->|yes| own{"row's Telegram user id is the owner's"}
  own -->|no| r7["403 not_owner"]
  own -->|yes| ctr{"both zero, or presented above stored"}
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

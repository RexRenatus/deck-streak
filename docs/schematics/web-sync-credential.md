# Schematic: the web client's sync key, from login to removal

SPEC-363 and ADR-374. Every `path:line` here was read at DeckStreak `dev` `33033b65`, and
every fork citation at the engine fork's pin, `c538de55`. This schematic is drawn before the code,
and part 2's components are marked as such.

## 1. Components, and what each can reach

```mermaid
flowchart LR
  subgraph Browser["The browser, one origin"]
    Page["Page: SvelteKit screens, the sync form, sign-out, also runs Telegram's script (app.html:6)"]
    %% Note (SPEC-400): app.html:6 is removed; Telegram's script now loads only on a launch from Telegram.
    subgraph WorkerBox["Dedicated Worker (worker.ts:39-46, own-origin messages only)"]
      Cred["credential.ts: the store adapter (part 2)"]
      Engine["web engine, wasm: the engine and the core's rule exports"]
    end
    Frame["Card frame: sandboxed, no same-origin, no script (ADR-352)"]
    IDB[("Indexed database deck-streak-credential: generation, sealed record")]
    OPFS[("Private file system: deck-streak, deck-streak-media")]
    Cookie[("Session cookie __Host-deckstreak_session, HttpOnly")]
  end
  subgraph Service["The service, behind the edge"]
    Edge["Edge: strips cookies and logs no request header on /anki-sync/ (caddy:28, 80-81)"]
    API["API: session routes, POST /api/sync/seal-key (part 1)"]
    Seal["sync_seal.rs: HMAC-SHA-256 of label and seal id under the seal secret"]
    Sync["Sync server: the engine's sync protocol, key in the anki-sync header"]
  end
  subgraph CI["CI gates"]
    Formal["formal: tla/SyncCredential"]
    Tests["cargo tests: core rule, seal, release route, boundary census"]
    Vitest["vitest: store, reach census, sign-out (part 2)"]
    Mut["mutation rows, and the TypeScript mutation run"]
  end
  Page -- "status words, sync form, forget" --> Cred
  Cred -- "status words only" --> Page
  Cred --> Engine
  Cred <--> IDB
  Engine <--> OPFS
  Cred -- "POST seal-key, cookie sent by the browser" --> Edge
  Page -- "DELETE /api/session" --> Edge
  Edge --> API
  API --> Seal
  Engine -- "login and sync, key in header, own origin only" --> Edge
  Edge --> Sync
  Cookie -. "sent with same-origin requests" .-> Edge
  Page -. "renders a card into" .-> Frame
  Formal -. covers .-> Engine
  Formal -. covers .-> API
```

| part | can reach | cannot reach |
|---|---|---|
| card frame | its own face's markup | the origin's storage, the cookie, the Worker, the network: its origin is opaque and it runs no script (ADR-352) |
| a page script, Telegram's included | the sealed record, the release route while a session is live, the sync form while the owner types | the cookie's value; the Worker's memory; the opened key in any reply |
| the Worker | the store, the release, the opened key in memory, its own origin's sync route | any other origin (`connect-src 'self'`, svelte.config.js:30; the core's endpoint guard) |
| the API | the seal secret (its unit's credential `sync-seal-secret`; until the host step loads it the release answers 404 `sync_seal_off`), the session store | the host key, the sync password, the sealed record |
| the edge | the sync route's address and path | any request header on the sync route (caddy:28), any cookie on it (caddy:80-81) |

The accepted residual is the second row: a same-origin script during a live session can open the
record (ADR-374 Consequences). Telegram's script, which loads on every route (app.html:6), is one
(SPEC-400 has since removed app.html:6, so Telegram's script now loads only on a launch from Telegram)
such script, and ADR-374 names it as the accepted residual of this design.

## 2. The key's life: obtain, store, use in a sync, replace, remove

```mermaid
sequenceDiagram
  autonumber
  participant O as Owner
  participant P as Page
  participant W as Worker credential.ts
  participant C as Core rule, via wasm
  participant DB as Indexed database
  participant A as API seal-key
  participant S as Sync route
  Note over O,S: Obtain
  O->>P: types sync user and password
  P->>W: sync-login {user, password}, one message
  W->>DB: read generation g0
  W->>A: POST seal-key {seal_id: fresh 16 bytes}
  alt no owner session
    A-->>W: 401 no_session
    W-->>P: needs-sign-in, password dropped, nothing sent
  else session live
    A-->>W: 200 {key K}
    W->>S: engine login, own origin, through the core's endpoint guard
    S-->>W: host key H
    W->>DB: one transaction: read g, C.on_obtained(g0, g)
    alt Kept at g+1
      W->>DB: write generation g+1 and sealed AES-GCM(K, H, aad endpoint user g+1)
    else Discard: a forget or another login landed first
      W-->>W: drop H and K
    end
  end
  Note over O,S: Use in a sync
  W->>DB: read generation g and sealed record
  opt key not yet open in this Worker
    W->>A: POST seal-key {seal_id from the record}
    A-->>W: 200 K, or 401 kept as needs-sign-in, or unreachable kept as offline
    W-->>W: open, a record that does not open is deleted, generation raised
  end
  W->>C: may_send(held g, current g, sealed)
  W->>S: sync, H in the anki-sync header
  S-->>W: answer
  W->>C: classify(answer) then on_outcome(sent g, current g, outcome)
  alt refused, current generation
    W->>DB: delete record, generation g+1
  else accepted or failed
    W-->>W: keep
  end
  Note over O,S: Replace
  Note right of W: A re-login is Obtain again, the newer generation wins.<br/>A new password or hash on the server is a refusal at the next sync.<br/>A seal-secret rotation makes the record fail to open, so it is deleted.
  Note over O,S: Remove
  O->>P: sign out
  P->>W: credential-forget
  W->>DB: one transaction: delete record, generation C.on_removed(g)
  W-->>W: clear memory, broadcast to the origin's other Workers
  P->>A: DELETE /api/session, after the forget, offline or not
```

## 3. The record's states, as the status operation names them

```mermaid
stateDiagram-v2
  [*] --> absent
  absent --> sealed: a kept login
  sealed --> held: a release and an open in this Worker
  sealed --> needs_sign_in: the release refused 401
  sealed --> offline: the release unreachable
  needs_sign_in --> held: a sign-in, then a release
  offline --> held: back online, then a release
  held --> held: an accepted or failed sync
  held --> absent: refused at the current generation
  sealed --> absent: a record that does not open
  held --> absent: forget
  sealed --> absent: forget
  needs_sign_in --> absent: forget
  offline --> absent: forget
  held --> sealed: another Worker raised the generation, memory cleared
```

Study reads none of these states: every study operation answers the same in each.

## 4. The model's shape, `formal/tla/SyncCredential`

| model variable | stands for |
|---|---|
| `gen` | the stored generation |
| `sealed` | the stored record: none, or the generation and key version it was sealed at |
| `keyver` | the seal secret's version, moved by `Rotate` |
| `mem[w]` | what Worker `w` holds open: none, or a generation |
| `login[w]` | a login in flight in `w`: none, or the generation it started at |
| `inflight[w]` | a send in flight from `w`: none, or the generation it sent |
| `session` | whether an owner session is live |
| history booleans | a sign-out happened and no login started since; used by the properties |

Actions: `StartLogin` (needs `session`), `LandLogin`, `LoginFails`, `SignOut`, `Unseal` (needs
`session`), `DropStale`, `StartSend`, `Accepted`, `Refused`, `Failed`, `Rotate`, `SessionOpens`,
`SessionEnds`, `Restart`. Bounds: two Workers, generations up to 4, two key versions.

| property | the switch its witness turns off |
|---|---|
| `NoKeyAtRestAfterSignOut` | `CheckLoginGeneration` (`LandLogin` keeps every login) |
| `NoSendAfterSignOut` | `CheckSendGeneration` (`StartSend` ignores the stored generation) |
| `ARefusalDropsOnlyItsOwnKey` | `CheckRefusalGeneration` (`Refused` drops whatever is stored) |
| `AFailureKeepsTheKey` | `DropOnlyOnRefusal` (`Failed` drops the record) |
| `UnsealOnlyInALiveSession` | `ReleaseNeedsSession` (`Unseal` ignores `session`) |

Out of the model, by abstraction: the cipher (a record opens exactly under its own key version),
the nonce, the HMAC, and the bytes of the protocol. Each is held by a unit test and a mutation row
instead.

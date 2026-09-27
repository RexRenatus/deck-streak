# Schematic: the Mini App launches, routes its startapp token and signs in once

Kind: sequence and state machine. Read at DeckStreak `main` e05dfa5 (ADR-005, ADR-006, ADR-007,
`docs/schematics/initdata-auth-sequence.md`, the telegram-platform pack). Decided by ADR-028. The
server side of the handshake is `docs/schematics/initdata-auth-sequence.md`; this drawing is the
client's.

```mermaid
sequenceDiagram
  participant T as Telegram client
  participant W as telegram.svelte.ts (the one wrapper)
  participant R as startapp.ts and routes.ts
  participant A as api.ts
  participant S as /api (same origin)
  T->>W: open with tgWebAppData, version, theme in the URL hash
  W->>W: read the hash once, before the router starts
  W->>R: signed start_param (or none)
  R-->>W: a route from the closed table, else Today
  W->>A: raw initData (never initDataUnsafe)
  A->>S: POST /api/session with initData
  alt 200 and a session cookie
    S-->>A: session
    A->>S: GET /api/me (cookie only)
    S-->>A: the server's study day
  else 401 or 403
    S-->>A: refused
    A-->>W: "reopen DeckStreak from Telegram"
  end
  W->>T: ready() once, after the first render
```

```mermaid
stateDiagram-v2
  [*] --> Handshaking
  Handshaking --> Signed: 200
  Handshaking --> Stopped: 401 or 403
  Signed --> Signed: a call answers 2xx
  Signed --> Rehandshaking: a call answers 401
  Rehandshaking --> Signed: 200
  Rehandshaking --> Stopped: refused again
  Stopped --> [*]: the owner reopens the Mini App
```

| token | route |
|---|---|
| `today`, empty, unknown or malformed | `/` |
| `about` | `/about` |

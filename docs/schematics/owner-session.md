# Schematic: an owner session, from the handshake to its end

Kind: state machine, component diagram and sequence. Read at DeckStreak `main` e05dfa5 and `dev`
1534f5d (ADR-006, ADR-024, ADR-025, `docs/schematics/initdata-auth-sequence.md`,
`docs/schematics/mini-app-launch.md`, the auth pack's session practice). Decided by ADR-024; built
by SPEC-024. The validation steps before a session exists are the auth-sequence schematic's; the
wire contract is the Mini App's (`mini-app-launch.md`).

## The session's states

```mermaid
stateDiagram-v2
  [*] --> Guarded: POST /api/session (the shell's layers first)
  Guarded --> Refused403: Sec-Fetch-Site present and not same-origin, or not application/json
  Guarded --> Throttled429: more than 30 handshakes this minute, until the minute turns
  Guarded --> Refused413: a body over the handshake's own limit
  Guarded --> Validating: the body's init_data
  Validating --> Refused401: no hash, repeated key, a field that does not decode, forged, no auth_date or user.id
  Validating --> Refused401: auth_date older than the bound, or more than 60 s ahead (init_data_stale)
  Validating --> Refused403: valid and fresh, but user.id is not the owner
  Validating --> Live: owner: the session the request carried ends#59; a NEW 32-byte id, only its SHA-256 kept#59; oldest evicted past 8
  Live --> Live: a request with the cookie refreshes the idle timer
  Live --> Ended: 30 minutes without a request
  Live --> Ended: 8 hours after it began
  Live --> Ended: DELETE /api/session (cookie cleared)
  Live --> Ended: the process restarts (the store is in memory)
  Ended --> [*]: the next call answers 401#59; the Mini App re-handshakes once
```

## Who holds what

The daemon's `api` role reads the two credentials at start, through the kernel's loader, which
registers each value with the redactor the log writer reads. `identity` keeps no personal data on
disk: the owner's id and a key derived from the bot token live in memory, and the store holds each
session as the SHA-256 of its id. `api` holds the routes, the cross-site bound and the handshake
bound; every route is served under the shell's layers (SPEC-025).

```mermaid
flowchart TB
  subgraph daemon[deck-streak-daemon: role_api]
    creds[CredentialsDirectory then CredentialLoader, with main's Redactor] --> load[OwnerGate::load]
    fresh[Freshness::from_env: DECKSTREAK_INIT_DATA_MAX_AGE_SECONDS] --> load
    clock[SystemClock] --> access
    load --> access[OwnerAccess: gate, sessions, clock, study-day rule, handshake bound]
    access --> state[ApiState::new then with_owner]
    state --> router[deck_streak_api::router: the shell's layers]
  end
  subgraph identity[deck-streak-identity]
    key[init_data::WebAppKey: HMAC-SHA-256 of the bot token keyed with WebAppData]
    validate[init_data::validate: strict decode, check string, verify_slice, auth_date, user.id]
    gate[owner::OwnerGate::admit: validate, then the owner pin; logs a reason code only]
    store[session::Sessions: at most 8, idle 30 min, absolute 8 h, ids from getrandom]
    extractor[session::OwnerSession: FromRequestParts, 401 without a live session]
    key --> validate --> gate
    store --> extractor
  end
  subgraph api[deck-streak-api: session_routes]
    guard[StateChange: JSON only, same-origin when Sec-Fetch-Site is sent]
    slot[HandshakeSlot: 30 a minute on the clock]
    open[POST /api/session: body limit 16 KiB]
    close[DELETE /api/session]
    me[GET /api/me]
  end
  router --> open & close & me
  open --> guard --> slot --> gate
  gate --> store
  close --> guard
  close --> store
  me --> extractor
```

## A handshake, in order

Each step can refuse; nothing after a refusal runs, and a refusal is logged by its reason code
alone.

```mermaid
sequenceDiagram
  participant M as Mini App
  participant L as the shell's layers
  participant R as api session_routes
  participant G as identity OwnerGate
  participant S as identity Sessions
  M->>L: POST /api/session, application/json, {"init_data": "<raw launch data>"}
  L->>R: request id, sensitive headers, trace, panic catcher, timeout, shed
  R->>R: StateChange: 403 cross_site_request or not_json
  R->>R: HandshakeSlot: 429 too_many_handshakes, Retry-After until the minute turns
  R->>R: the body, at most 16 KiB (413), as JSON with init_data (401 init_data_invalid)
  R->>G: admit(raw, now)
  G->>G: validate: 401 init_data_invalid or init_data_stale
  G->>G: the owner pin: 403 not_owner
  G-->>R: the owner
  R->>S: end the session the request carried, then open a new one
  S-->>R: a new id, hex, only here in the clear
  R-->>M: 200 and Set-Cookie __Host-deckstreak_session=id#59; Path=/#59; Max-Age=28800#59; Secure#59; HttpOnly#59; SameSite=Strict
  M->>R: GET /api/me with the cookie alone
  R->>S: OwnerSession: the live session, its idle timer refreshed (else 401 no_session)
  R-->>M: {"study_day": "YYYY-MM-DD"}, from the kernel's rule and clock
```

| guard | where | row |
|---|---|---|
| WebAppData HMAC over the sorted check string | `identity::init_data::validate` | `ws.tg-init-data-verified` (by hand) |
| constant-time hash compare | `identity::init_data` (`verify_slice` only; A6's census) | `ws.tg-init-data-constant-time` (by hand) |
| freshness | `identity::init_data` against the kernel's clock | `ws.tg-init-data-fresh` (by hand) |
| owner pin | `identity::owner::OwnerGate::admit` | CHARTER 14 |
| rotation | a new id at every handshake; the carried one ends | `auth.session-rotated-on-login` |
| idle and absolute end | `identity::session::Sessions` on the kernel's clock | `cp.session-idle-timeout` |
| logout | `DELETE /api/session` ends the session in the store | `auth.logout-server-side` |
| cookie | `__Host-deckstreak_session`; `Path=/`, `Max-Age`, `Secure`, `HttpOnly`, `SameSite=Strict`, no `Domain` | `ws.session-cookie-flags` (by hand), `ws.cookie-prefix`, `ws.session-samesite` |
| CSRF | JSON only; `Sec-Fetch-Site` must be `same-origin` when present | `ws.upload-csrf-cors` |
| handshake bound | 30 a minute, per process | `ws.rate-limit` |
| handshake body | 16 KiB, below the shell's 2 MiB | `ws.request-body-limit` (by hand) |

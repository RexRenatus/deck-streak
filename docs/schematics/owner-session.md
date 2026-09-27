# Schematic: an owner session, from the handshake to its end

Kind: state machine. Read at DeckStreak `main` e05dfa5 (ADR-006,
`docs/schematics/initdata-auth-sequence.md`, the auth pack's session practice). Decided by ADR-024;
built by SPEC-024. The validation steps before a session exists are the auth-sequence schematic's.

```mermaid
stateDiagram-v2
  [*] --> Validating: POST /api/session with raw initData (JSON, same origin)
  Validating --> Refused401: no hash, repeated key, forged, or auth_date older than the bound or 60 s ahead
  Validating --> Refused403: valid and fresh, but user.id is not the owner
  Validating --> Throttled429: more than 30 handshakes this minute
  Validating --> Live: owner: a NEW 32-byte id, only its SHA-256 kept; oldest evicted past 8
  Live --> Live: a request with the cookie refreshes the idle timer
  Live --> Ended: 30 minutes without a request
  Live --> Ended: 8 hours after it began
  Live --> Ended: DELETE /api/session (cookie cleared)
  Live --> Ended: the process restarts (the store is in memory)
  Ended --> [*]: the next call answers 401; the Mini App re-handshakes once
```

| guard | where | row |
|---|---|---|
| constant-time hash compare | `identity::init_data` (`verify_slice` only) | `ws.tg-init-data-constant-time` (by hand) |
| freshness | `identity::init_data` against the kernel's clock | `ws.tg-init-data-fresh` (by hand) |
| rotation | a new id at every handshake | `auth.session-rotated-on-login` |
| cookie | `__Host-deckstreak_session`; `Secure`, `HttpOnly`, `SameSite=Strict`, `Path=/` | `ws.session-cookie-flags` (by hand) |
| CSRF | JSON only; `Sec-Fetch-Site` must be `same-origin` when present | `ws.upload-csrf-cors` |

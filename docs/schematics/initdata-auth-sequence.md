# Schematic: the Mini App's authentication sequence

Kind: sequence. Read at DeckStreak `main` 769ee62 (ADR-006). The rows the web-security pack reads
are named on the step they judge.

```mermaid
sequenceDiagram
  participant T as Telegram client
  participant M as Mini App (web/app)
  participant C as Caddy
  participant A as api (identity extractor)
  T->>M: open with tgWebAppData in the URL hash
  M->>M: telegram.svelte.ts reads initData once, calls ready()
  M->>C: POST /api/session with the raw initData
  C->>A: proxy to loopback
  A->>A: parse, drop hash, sort key=value lines
  A->>A: key = HMAC_SHA256("WebAppData", bot token)
  A->>A: compare HMAC_SHA256(key, data-check-string) with hash in constant time (ws.tg-init-data-constant-time)
  A->>A: refuse an auth_date older than the configured maximum (ws.tg-init-data-fresh)
  alt forged, stale or malformed
    A-->>M: 401, nothing logged but the reason code
  else valid but not the owner's user id
    A-->>M: 403
  else the owner
    A-->>M: 200 and a __Host- session cookie (Secure, HttpOnly, SameSite=Strict)
  end
  M->>C: later calls carry the session cookie only
```

initData is never trusted from `initDataUnsafe`, never stored raw (privacy-gdpr
`telegram-minimised`), and never logged (observability `obs.no-secret-fields`).

# Schematic: the Mini App launches, routes its startapp token and signs in once

Kind: sequence, state machine and data flow. Read at DeckStreak `main` e05dfa5 and `dev` 5257a97
(ADR-005, ADR-006, ADR-007, `docs/schematics/initdata-auth-sequence.md`, the telegram-platform
pack, and Telegram's own `telegram-web-app.js`). Decided by ADR-028; built by SPEC-028. The server
side of the handshake is `docs/schematics/initdata-auth-sequence.md` and
`docs/schematics/owner-session.md`; this drawing is the client's.

## The launch

Telegram's script is the first script in `<head>` and runs before any app code. It parses the
launch parameters out of the URL hash, keeps them in the tab's session storage (so a reload after a
navigation still has them), and sets the `--tg-*` CSS variables. The wrapper reads the script's
object once, when its module loads, which is before the router's first navigation: no navigation
can lose the launch.

```mermaid
sequenceDiagram
  participant T as Telegram client
  participant S as telegram-web-app.js (first in head)
  participant W as telegram.svelte.ts (the one wrapper)
  participant L as +layout.ts and startapp.ts
  participant P as Today (+page.svelte)
  participant A as api.ts
  participant API as /api (same origin)
  T->>S: open with tgWebAppData, version and theme in the URL hash
  S->>S: parse the hash once, keep it for the session, set --tg-theme-* and --tg-safe-area-*
  W->>S: read the object once: signed launch data, start_param, version, theme, viewport, insets
  L->>W: start_param (signed, inside the launch data)
  alt the launch carried a token
    L->>L: routeFor(token): a route of the closed table, else Today
    L-->>P: redirect once to that route
  else no token
    L-->>P: the requested route
  end
  P->>A: me()
  A->>API: POST /api/session {"init_data": raw launch data}
  alt 200 and a session cookie
    API-->>A: session
    A->>API: GET /api/me (cookie only)
    API-->>A: {"study_day": "YYYY-MM-DD"}
    A-->>P: the server's study day
  else 401 or 403
    A-->>P: reopen DeckStreak from Telegram
  end
  W->>S: ready() then expand(), once, after the first render
```

## The session, as the client holds it

```mermaid
stateDiagram-v2
  [*] --> Handshaking: the first call
  Handshaking --> Stopped: no launch data (outside Telegram)
  Handshaking --> Signed: 200
  Handshaking --> Stopped: 401 or 403
  Handshaking --> [*]: 5xx or no answer, so "unavailable" and the next call tries again
  Signed --> Signed: a call answers 2xx
  Signed --> Rehandshaking: a call answers 401
  Rehandshaking --> Signed: 200, and the call is repeated once
  Rehandshaking --> Stopped: refused again, or the repeated call answers 401
  Stopped --> [*]: every call answers "reopen" and sends nothing, until the owner reopens the Mini App
```

## The wire contract SPEC-024 serves

| call | request | answer the client reads |
|---|---|---|
| open a session | `POST /api/session`, `Content-Type: application/json`, body `{"init_data": "<the raw launch data>"}`, `credentials: same-origin` | 2xx and the `__Host-` session cookie; 401 or 403 refuses |
| the owner's day | `GET /api/me`, the session cookie only | `{"study_day": "YYYY-MM-DD"}` (the lexicon's `study_day`); 401 when the session has ended |

The launch data travels only in that one request body: never in a URL, a header, the device's
storage or a log line. The client parses nothing out of it but `start_param`, and that only to
choose a screen from the closed table.

## The startapp tokens

| token | route |
|---|---|
| `today` | `/` |
| `about` | `/about` |
| empty, unknown, or not `[A-Za-z0-9_-]{1,64}` | `/` (Today) |

## The design tokens

```mermaid
flowchart LR
  J["src/lib/design/tokens.json<br/>DTCG 2025.10: a ref tier and a color tier"] --> B["scripts/build-tokens.ts<br/>a Vite plugin runs it when a build or the dev server starts"]
  B --> C["src/lib/design/tokens.css, not tracked<br/>:root: --background is var(--tg-theme-bg-color, #ffffff) ...<br/>@theme inline: --color-background is var(--background) ..."]
  C --> L["src/routes/layout.css<br/>Tailwind 4"]
  TG["Telegram's script<br/>sets --tg-theme-*"] -.-> C
```

A colour that names no Telegram theme parameter fails the compile. `tokens.test.ts` compiles the
same file and judges the result: every colour reads a `--tg-theme-*` variable with a fallback, and
every text colour meets WCAG 2.2 AA on each background it declares, in both of Telegram's default
palettes (`web/app/tests/telegram-palettes.ts`) and in the fallbacks.

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

> **Amended by SPEC-400 (ADR-414).** Telegram's script is no longer in the shell. The app's start
> hook adds it only on a launch, and awaits it before the router's first navigation. The drawing
> below still holds for a launch, except that the hook adds the script's participant rather than
> the shell's `<head>`. "The launch, gated", at the end of this file, draws the start inside and
> outside a launch.

> **Amended by SPEC-403 (ADR-417).** The start hook no longer loads Telegram's script because a
> launch parameter is present. It first sends the fragment's launch data to `POST /api/launch`,
> and it loads the script only when that answers 204. "The launch, validated first", at the end
> of this file, draws the start on the accepted, refused and unanswered paths.

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

## The launch, gated (SPEC-400, ADR-414)

Kind: sequence. Read at dev `ce2bb195`.

The server answers every route with one fallback document (`deploy/caddy/deck-streak.caddy:91`).
The shell's policy admits Telegram's origin (`web/app/svelte.config.js:27`), and the shell holds no
script element that names that origin.

The app's client `init` hook (`web/app/src/hooks.client.ts`) runs once, before the router's first
navigation, and awaits `web/app/src/lib/telegram-launch.ts`. The URL fragment is still whole at
that point. A launch is either of:

- a fragment that names `tgWebAppData`, `tgWebAppVersion` or `tgWebAppPlatform` (presence only, no
  value read);
- the tab's own launch mark in session storage. A fragment launch writes the mark, and the mark
  holds no launch data.

On a launch, the hook adds Telegram's script and waits for it. Outside a launch, it adds a second
policy that refuses Telegram's origin for the page's life. Every policy a document holds applies.
Only then does the wrapper's module load and read the script's object, or find none.

The launch data still travels only in the one request body below, and the server validates it as
before.

```mermaid
sequenceDiagram
  participant T as Telegram client or a plain browser tab
  participant H as the web server (deck-streak.caddy)
  participant K as SvelteKit's start (the shell)
  participant I as hooks.client.ts init
  participant L as telegram-launch.ts
  participant S as telegram-web-app.js
  participant W as telegram.svelte.ts (the one wrapper)
  participant R as +layout.ts (the first navigation)
  participant A as api.ts
  participant API as POST /api/session (session_routes.rs)
  participant V as init_data.rs validate
  T->>H: GET any route, the fragment kept in the tab
  H-->>T: the one fallback document for every route (try_files, line 91)
  Note over T,H: served header policy: frame-ancestors, object-src, base-uri (line 19)
  Note over T,K: shell meta policy: script-src self, telegram.org, wasm-unsafe-eval, and no script element names telegram.org
  T->>K: run the shell's start script
  K->>I: await init()
  I->>I: set the page language and direction
  I->>L: await admitLaunch(window)
  L->>L: a launch is a fragment naming tgWebAppData, tgWebAppVersion or tgWebAppPlatform, or the tab's mark
  alt a launch
    L->>L: write the tab's mark, which holds no launch data
    L->>S: append the script from telegram.org with referrerpolicy same-origin
    S->>S: parse the fragment, keep it for the session, set the --tg-* variables
    S-->>L: load, or error
  else outside a launch
    L->>L: append a meta policy, script-src self and wasm-unsafe-eval
    Note over L: from here a script element naming telegram.org is refused (script-src-elem)
  end
  L-->>I: resolved
  I-->>K: resolved
  K->>R: the router's first navigation
  R->>W: load the wrapper, which reads the script's object once or finds none
  opt the launch carried a start token
    R->>R: redirect once, which drops the fragment
  end
  R->>A: the page's first call
  alt launch data present
    A->>API: POST /api/session with body {"init_data": raw launch data}
    API->>V: admit (session_routes.rs line 140), then validate (init_data.rs line 88)
    V-->>API: the signature (line 99) and the freshness (line 107) hold, or a refusal
    API-->>A: 200 and the session cookie, or 401 or 403
  else no launch data (outside a launch)
    A-->>R: reopen DeckStreak from Telegram, and nothing is sent
  end
```

| route | served by | policy in force after the start hook | Telegram's script |
|---|---|---|---|
| every path of `web/app/src/lib/routes.ts`, inside a launch | the fallback document | the shell's policy alone | loaded once by the hook, before the first navigation |
| every path of `web/app/src/lib/routes.ts`, outside a launch | the fallback document | the shell's policy and the hook's narrower one, together | never requested; an element naming it is refused |
| a reload after the launch's redirect | the fallback document | the shell's policy alone (the tab's mark) | loaded again; it reads the launch from its own session storage |

## The launch, validated first (SPEC-403, ADR-417)

Kind: sequence and data flow. Read at dev `32f62172`.

This section supersedes the detection step of "The launch, gated" above. A fragment's launch
parameters no longer make a launch by their presence. The start hook reads the fragment's
`tgWebAppData` value, decoded once by `URLSearchParams` (the decoding Telegram's script applies),
and asks the server whether it is a launch before any script from Telegram's origin is added.
Everything else in that section still holds: the fallback document, the shell's policy, the
narrowed policy outside a launch, and the wrapper reading the script's object once.

- **Accepted** is a 204 and nothing else. A 200 is the fallback document's status, so it never
  counts.
- **Refused** is a 401 or a 403, carrying the gate's reason code alone.
- **Unanswered** is any other status, a failed request, or no answer within 10 000 ms, the server's
  own request timeout (`crates/api/src/router.rs:78`). The page treats it as it treats a refusal.

The answer and the deadline race in one `Promise.race`, so exactly one outcome settles and a later
answer changes nothing. The tab's mark `deck-streak:launch-accepted` is written only after a 204,
holds no launch data, and lets a reload whose fragment the router has dropped load the script with
no second request. The mark the gated start wrote on presence alone (`deck-streak:launched`) is
never read.

The server side is `POST /api/launch`, which follows `crates/api/src/session_routes.rs`'s last line.
It takes the session handshake's state-change guard, a slot from the handshake's own bound (30 a
minute, shared with `POST /api/session`), and the handshake's 16 KiB body bound. It validates with
the same `OwnerGate::admit` (`crates/identity/src/owner.rs:148-161`, then `init_data.rs` validate).
It opens no session and sets no cookie. `crates/api/src/router.rs` merges it directly after the
session routes (line 258), so it is served exactly when they are.

```mermaid
sequenceDiagram
  participant T as Telegram client or a plain browser tab
  participant H as the web server (deck-streak.caddy)
  participant K as SvelteKit's start (the shell)
  participant I as hooks.client.ts init
  participant L as telegram-launch.ts
  participant V as POST /api/launch (session_routes.rs)
  participant G as OwnerGate admit (owner.rs and init_data.rs)
  participant S as telegram-web-app.js
  participant W as telegram.svelte.ts (the one wrapper)
  participant R as +layout.ts (the first navigation)
  participant A as api.ts
  participant API as POST /api/session (session_routes.rs)
  T->>H: GET any route, the fragment kept in the tab
  H-->>T: the one fallback document for every route
  Note over T,H: an /api/ path is proxied to the service, never answered by the fallback
  T->>K: run the shell's start script
  K->>I: await init()
  I->>I: set the page language and direction
  I->>L: await admitLaunch(window)
  L->>L: read the fragment's tgWebAppData value, decoded once, empty means none
  alt the fragment carries launch data
    L->>V: POST /api/launch, body init_data, same-origin, JSON, with an abort signal
    V->>V: the state-change guard, a handshake slot, the 16 KiB body bound
    V->>G: admit(init_data, now)
    G-->>V: the owner, or a refusal with its reason
    V-->>L: 204 with no body and no cookie, or 401 or 403 with the reason alone
    Note over L,V: the answer races a 10 000 ms deadline, which aborts the request, and the first to settle decides
    alt 204, accepted
      L->>L: mark the tab deck-streak:launch-accepted, which holds no launch data
      L->>S: append the script from Telegram's origin with referrerpolicy same-origin
      S->>S: parse the fragment, keep it for the session, set the --tg-* variables
      S-->>L: load, or error
    else 401 or 403, refused
      L->>L: remove the tab's accepted mark, append the narrowed meta policy
      Note over L: from here a script element naming Telegram's origin is refused
    else any other answer, a failed request, or the deadline, unanswered
      L->>L: remove the tab's accepted mark, append the narrowed meta policy
      Note over L: a later answer, a 204 included, changes nothing
    end
  else no launch data, and the tab carries the accepted mark, a reload
    L->>S: append the script with referrerpolicy same-origin, and send no request
    S-->>L: load, or error
  else no launch data and no accepted mark
    L->>L: append the narrowed meta policy, and send nothing
  end
  L-->>I: resolved
  I-->>K: resolved
  K->>R: the router's first navigation
  R->>W: load the wrapper, which reads the script's object once or finds none
  opt the launch carried a start token
    R->>R: redirect once, which drops the fragment
  end
  R->>A: the page's first call
  alt the wrapper read launch data, because the script loaded
    A->>API: POST /api/session with the same launch data
    API->>G: admit again, so data grown stale since the launch is refused here
    API-->>A: 200 and the session cookie, or 401 or 403
  else no launch data
    A-->>R: reopen DeckStreak from Telegram, and nothing is sent
  end
```

| the start | the request it sends | policy in force after the start hook | Telegram's script |
|---|---|---|---|
| a fragment with launch data, answered 204 | one `POST /api/launch` | the shell's policy alone | loaded once, after the 204, before the first navigation |
| a fragment with launch data, answered 401 or 403 | one `POST /api/launch` | the shell's policy and the hook's narrower one, together | never requested; an element naming it is refused |
| a fragment with launch data, answered otherwise, failed, or past the deadline | one `POST /api/launch`, aborted at the deadline | the shell's policy and the hook's narrower one, together | never requested; an element naming it is refused |
| a reload after an accepted launch, the fragment dropped | none | the shell's policy alone (the tab's accepted mark) | loaded again; it reads the launch from its own session storage |
| no launch data and no accepted mark | none | the shell's policy and the hook's narrower one, together | never requested; an element naming it is refused |

The wire contract gains one call, beside the two that SPEC-024 serves:

| call | request | answer the client reads |
|---|---|---|
| validate a launch | `POST /api/launch`, `Content-Type: application/json`, body `{"init_data": "<the raw launch data>"}`, `credentials: same-origin` | 204 accepts; 401 or 403 with `{"reason": ...}` refuses; anything else, a failed request or no answer within 10 s is unanswered |

The launch data travels only in that request body and in the session handshake's, never in a URL,
a header, the device's storage or a log line.

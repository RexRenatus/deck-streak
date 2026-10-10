# SPEC-400: the page loads Telegram's script only when Telegram launched it, and outside a launch its policy refuses that origin

- **Issue:** #706. It is the residual SPEC-363 records as M11 and ADR-374 accepts in its
  Consequences, cited by #654.
- **Context(s):** `miniapp` (`web/app/src`, `web/app/tests`, `web/app/tests-study`) and the
  documents that describe its launch (`docs/`).
- **Decided by:** ADR-414 (D1 the population, D2 the detection and the load, D3 the policy, D4
  FORMAL, D5 drift and the push count). It works under ADR-005 (the official script behind one
  wrapper), ADR-007 (the policy in hash mode), SPEC-028 R2 (the wrapper is the only reader) and
  SPEC-024 (the server validates the launch data).
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-400.md`.

## 1. The problem, measured

Every fact was read at dev `05774897`. Each command reproduces its fact from the repository root.

### 1.1 Where the script loads, and what admits it

| fact | where | command |
|---|---|---|
| the shell loads Telegram's script, after the referrer meta (line 5) and before `%sveltekit.head%` (line 12) | `web/app/src/app.html:6` | `git grep -n -E 'script\|referrer\|sveltekit.head' -- web/app/src/app.html` |
| the script's URL appears in 4 lines under `web/app/src`: the load, one test pin and two comments | `web/app/src/app.html:6`, `web/app/src/lib/csp.test.ts:49`, `web/app/src/lib/telegram.svelte.ts:4`, `web/app/src/lib/telegram.test.ts:14` | `git grep -n 'telegram-web-app\.js' -- web/app/src` |
| one fallback document answers every route | `deploy/caddy/deck-streak.caddy:91`, `web/app/svelte.config.js:14` | `git grep -n -E 'try_files\|fallback' -- deploy/caddy/deck-streak.caddy web/app/svelte.config.js` |
| the page's policy admits the origin, in hash mode, on every route | `web/app/svelte.config.js:24-33`, its `script-src` at `:27` | `git grep -n -E "mode\|script-src" -- web/app/svelte.config.js` |
| the served header names Telegram's web client only as a frame ancestor | `deploy/caddy/deck-streak.caddy:19` | `git grep -n 'Content-Security-Policy' -- deploy/caddy/deck-streak.caddy` |
| the routes table lists 17 paths | `web/app/src/lib/routes.ts:10` | `git show HEAD:web/app/src/lib/routes.ts \| sed -n '10,28p'` |

### 1.2 Who reads the launch, and when

| reader | where | when |
|---|---|---|
| the wrapper | `web/app/src/lib/telegram.svelte.ts:89-98`, called at `:150-152` | once, when its module loads |
| the root layout's load | `web/app/src/routes/+layout.ts:20-26` | the first navigation; its `redirect` at `:25` is the first app code that rewrites the URL, and it drops the fragment |
| the root layout's `ready()` | `web/app/src/routes/+layout.svelte:26` | after the first render |
| the API client's handshake | `web/app/src/lib/api.ts:114-121`, its launch-data source at `:239` | on the first call to the API |
| the about page's `openLink` | `web/app/src/routes/about/+page.svelte:16` | on a tap |
| the review screen's colour scheme | `web/app/src/lib/study/ReviewScreen.svelte:200` | when it renders |
| the styles | `web/app/src/routes/layout.css:28`, `:42-46` | every `--tg-*` read has a fallback |

The command is `git grep -n 'telegram.svelte' -- web/app/src`.

The start hook runs before all of these readers and reads none of them
(`web/app/src/hooks.client.ts:7-10`).

The server validates the launch data, and this SPEC leaves the validation unchanged:

- `POST /api/session` is routed at `crates/api/src/session_routes.rs:116`. Its body is parsed at
  `:135`, and the launch data is admitted at `:140`.
- The validator is `crates/identity/src/init_data.rs:88`. It checks the signature at `:99` and
  the launch data's freshness at `:107`.

### 1.3 Why the static policy cannot be narrowed, stated as what holds

The policy in the shell is one meta element, which SvelteKit writes once, at build. It cannot
admit the origin only where the script loads:

- One static document answers every route (§1.1), so a policy per route is one policy.
- Whether the page is inside a launch is written in the URL fragment, and no request carries a
  fragment to the server.
- A nonce needs a server that renders each response, and SvelteKit refuses a nonce for a
  prerendered page.
- A hash or subresource integrity pins bytes that Telegram updates in place at one URL.

What holds instead (R4):

- The static policy keeps the origin.
- On every page outside a launch, the start hook adds a second policy that refuses it, before the
  router's first navigation.
- Before that, the shell holds no element that names the origin (R7).

### 1.4 The checks that hold the base's shape

The build restates these, and ADR-414's Consequences lists each one:

- `web/app/src/lib/csp.test.ts:44-50` holds the shell to exactly one script, Telegram's.
- `web/app/src/lib/csp.test.ts:81-88` holds the referrer meta ahead of the first `<script src=`.
- `web/app/tests/smoke.spec.ts:27-31` holds Telegram's script first in the head.
- `web/app/tests/a11y.spec.ts:24-29` and the stand-in in `web/app/tests/streak-calendar.spec.ts`
  are served at the script's URL and expect it on every page.

The threat model's T2 row cites `web/app/src/lib/csp.test.ts:90`.

## 2. Requirements

R1. Outside a launch, no route requests Telegram's script, and none of it runs.

R2. The app's start hook reads the launch before the router's first navigation, from either of:

- the fragment, by the presence of `tgWebAppData`, `tgWebAppVersion` or `tgWebAppPlatform`, with
  no value read;
- the tab's launch mark in session storage.

A launch read from the fragment writes the mark, and the mark holds no launch data. A storage error
leaves the fragment as the only test.

R3. On a launch, the start hook adds Telegram's script to the head with
`referrerpolicy="same-origin"`. The app's first navigation waits until the script has loaded or
failed.

R4. Outside a launch, the start hook adds a second policy to the head, `script-src 'self'
'wasm-unsafe-eval'`. Its value is `svelte.config.js`'s `script-src` without Telegram's origin. A
script element naming that origin is then refused for the page's life.

R5. `web/app/svelte.config.js` is unchanged. §1.3 records why its policy cannot be narrowed.

R6. On a launch, the launch data reaches `POST /api/session` byte for byte, in the request body
only. The server's validation is unchanged.

R7. The page shell (`web/app/src/app.html`) carries no script, inline or by URL. The referrer meta
precedes everything SvelteKit writes into the head.

R8. Only `web/app/src/lib/telegram-launch.ts` names the launch parameters. The start hook never
imports the wrapper. The wrapper stays the only module that reaches the script's object (SPEC-028
R2).

R9. The Mini App's own checks pass unchanged. The browser suites that audit Telegram's palettes
open the page as a launch, and prove that their stand-in ran.

R10. Every document that says the script is first in the head carries an insert-only note naming
this SPEC: SPEC-028, ADR-005 and `docs/schematics/mini-app-launch.md`. The launch schematic gains
the gated launch. SPEC-363's M11 and ADR-374's residual carry a note that this SPEC closes them.
The threat model's web-client table gains a row for the control.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | outside a launch, no route of the routes table requests Telegram's script or runs it, while each route's own same-origin requests are made; the shell's script appears in the head only on a launch | `web/app/tests/telegram-launch.spec.ts`, `web/app/tests/smoke.spec.ts` (Playwright, CI `web`) |
| A2 | outside a launch, a script element naming Telegram's script is refused by the page's policy (`script-src-elem`), and no request leaves | `web/app/tests/telegram-launch.spec.ts` (Playwright, CI `web`) |
| A3 | a launch loads Telegram's script before the first navigation, and its launch data reaches `POST /api/session` unchanged; the server's validator and session routes still hold | `web/app/tests/telegram-launch.spec.ts`; `crates/identity/tests/init_data.rs`; `crates/api/tests/session_routes.rs` |
| A4 | a reload after the launch's first navigation (the fragment gone) loads Telegram's script again | `web/app/tests/telegram-launch.spec.ts` |
| A5 | a launch is read from the fragment's three launch parameters, by presence, and from the tab's mark; nothing else is a launch, and a storage error leaves the fragment test | `web/app/src/lib/telegram-launch.test.ts` |
| A6 | on a launch, the script is added to the head once, with `referrerpolicy="same-origin"`, and the start waits until it loads or fails; the referrer meta precedes `%sveltekit.head%` | `web/app/src/lib/telegram-launch.test.ts`, `web/app/src/lib/csp.test.ts` |
| A7 | outside a launch, the page adds exactly one policy, equal to the configured `script-src` without Telegram's origin, and no script | `web/app/src/lib/telegram-launch.test.ts` |
| A8 | the app's start hook does not resolve until the launch is admitted, and keeps setting the page language and direction | `web/app/src/lib/telegram-launch-hook.test.ts` |
| A9 | only the launch module names Telegram's launch parameters, and the census recognises a planted name | `web/app/src/lib/telegram-boundary.test.ts` |
| A10 | the start hook and the launch module never import the wrapper, and the hook imports the launch module | `web/app/src/lib/telegram-boundary.test.ts` |
| A11 | the page shell carries no script, inline or by URL | `web/app/src/lib/csp.test.ts` |
| A12 | the Mini App's own wrapper, boundary, layout and language checks pass unchanged | the files named in the fence |
| A13 | the palette and calendar audits open the page as a launch and prove their stand-in ran | `web/app/tests/a11y.spec.ts`, `web/app/tests/streak-calendar.spec.ts` |

```acceptance
A1: pnpm --dir web/app exec playwright test tests/telegram-launch.spec.ts -g "outside Telegram no route asks for Telegram's script or runs it"
A1: pnpm --dir web/app exec playwright test tests/smoke.spec.ts -g "telegram-web-app.js is in the head only when Telegram launched the page"
A2: pnpm --dir web/app exec playwright test tests/telegram-launch.spec.ts -g "outside Telegram the page's policy refuses a script from Telegram's origin"
A3: pnpm --dir web/app exec playwright test tests/telegram-launch.spec.ts -g "a launch loads Telegram's script before the first navigation and its launch data reaches the session unchanged"
A3: cargo test -p deck-streak-identity --test init_data -- --exact a_payload_signed_with_the_webappdata_key_is_accepted
A3: cargo test -p deck-streak-api --test session_routes -- --exact me_answers_the_study_day_only_with_a_live_session
A4: pnpm --dir web/app exec playwright test tests/telegram-launch.spec.ts -g "a reload after the launch's first navigation loads Telegram's script again"
A5: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "a launch is read from the fragment's launch parameters and from the tab's mark"
A6: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "Telegram's script is added to the head with no referrer and awaited until it loads or fails"
A6: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page sends no referrer to another origin"
A7: pnpm exec vitest run web/app/src/lib/telegram-launch.test.ts -t "outside a launch the page adds a policy that admits its own scripts and not Telegram's"
A8: pnpm exec vitest run web/app/src/lib/telegram-launch-hook.test.ts -t "the app's start admits the launch before its first navigation"
A9: pnpm exec vitest run web/app/src/lib/telegram-boundary.test.ts -t "only the launch module names Telegram's launch parameters"
A10: pnpm exec vitest run web/app/src/lib/telegram-boundary.test.ts -t "the hook that loads Telegram's script never imports the wrapper"
A11: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page shell carries no script, inline or by URL"
A12: pnpm exec vitest run web/app/src/lib/telegram.test.ts
A12: pnpm exec vitest run web/app/src/lib/telegram.node.test.ts
A12: pnpm exec vitest run web/app/src/lib/telegram-boundary.test.ts -t "only the wrapper touches the Telegram WebApp object"
A12: pnpm exec vitest run web/app/src/routes/layout-load.test.ts
A12: pnpm exec vitest run web/app/src/routes/layout-shell.test.ts
A12: pnpm exec vitest run web/app/src/routes/layout.test.ts
A12: pnpm exec vitest run web/app/src/lib/i18n.test.ts
A13: pnpm --dir web/app exec playwright test tests/a11y.spec.ts
A13: pnpm --dir web/app exec playwright test tests/streak-calendar.spec.ts
```

The study suite (`web/app/tests-study`) also runs the narrowed page, in two browser engines, in CI's
`web-engine` check run. It is not a criterion of its own: it changes by one comment.

## 4. File manifest

| path | context | change |
|---|---|---|
| `docs/specs/SPEC-400-the-page-loads-telegrams-script-only-when-telegram-launched-it-and-refuses-its-origin-otherwise.md` | docs | added: this SPEC |
| `docs/decisions/ADR-414-telegrams-script-loads-from-the-apps-start-hook-only-on-a-launch-and-the-page-narrows-its-policy-otherwise.md` | docs | added: its ADR |
| `docs/schematics/mini-app-launch.md` | docs | insert-only: a note after line 15, and a new section at the end with the gated launch |
| `docs/schematics/web-sync-credential.md` | docs | insert-only: a note directly under line 12 and one directly under line 60, each of which names the removed `web/app/src/app.html` line 6, saying the script now loads only on a launch |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs (threat model) | a new row in section 5, under the next free `E` id; T2's `csp.test.ts` line re-cited where it moved |
| `docs/specs/SPEC-028-mini-app-shell.md` | docs | insert-only: a new last section naming lines 16, 40-41, 54, 139, 206 and 223 as amended by this SPEC |
| `docs/specs/SPEC-363-the-web-client-keeps-the-sync-key-sealed.md` | docs | insert-only: a new last section saying this SPEC closes M11 (line 49) and the residual (lines 272-277) |
| `docs/decisions/ADR-005-mini-app-front-end-sveltekit.md` | docs | insert-only: a new last section naming line 34 as amended by ADR-414 |
| `docs/decisions/ADR-374-the-web-sync-key-is-sealed-under-a-key-the-service-releases-to-the-owners-session.md` | docs | insert-only: a new last section saying ADR-414 closes the residual (lines 286-290) |
| `docs/red-first/SPEC-400.md` | docs | added: the red-first record |
| `changelog.d/chat-script-scope-400.md` | docs | added: the changelog fragment |
| `web/app/src/app.html` | miniapp | line 6 removed |
| `web/app/src/hooks.client.ts` | miniapp | `init` awaits the launch module |
| `web/app/src/lib/telegram-launch.ts` | miniapp | added: the detection, the load, the mark and the narrowing |
| `web/app/src/lib/telegram-launch.test.ts` | miniapp | added: A5, A6, A7 |
| `web/app/src/lib/telegram-launch-hook.test.ts` | miniapp | added: A8 |
| `web/app/src/lib/telegram-boundary.test.ts` | miniapp | A9 and A10 added; its two tests unchanged |
| `web/app/src/lib/csp.test.ts` | miniapp | A11 added and lines 44-50 moved into it; the referrer test restated |
| `web/app/src/lib/telegram.svelte.ts` | miniapp | the header comment (lines 1-12) only |
| `web/app/tests/telegram-launch.spec.ts` | miniapp (browser) | added: A1, A2, A3, A4 |
| `web/app/tests/launch-fragment.ts` | miniapp (browser) | added: the launch fragment, the stand-in and the request count the suites share |
| `web/app/tests/smoke.spec.ts` | miniapp (browser) | lines 27-31 restated as A1's second test |
| `web/app/tests/a11y.spec.ts` | miniapp (browser) | opens each route as a launch; its stand-in comment |
| `web/app/tests/streak-calendar.spec.ts` | miniapp (browser) | opens the page as a launch |
| `web/app/tests-study/study.spec.ts` | miniapp (browser) | the comment at lines 32-34 only |
| `scripts/tests/test_threat_model.py` | hygiene | the web surface's pinned row-id list gains `"E6"`; no assertion is removed |
| `scripts/mutation-equivalent.d/miniapp.json` | mutation | only if a StrykerJS survivor in `telegram-launch.ts` is triaged equivalent |

`web/app/svelte.config.js` and `deploy/caddy/deck-streak.caddy` are unchanged. No migration, and no
hand mutation row.

## 5. What this does NOT cover

- The served header's `frame-ancestors`, which names who may frame the page, is unchanged: #706
  asks about the scripts the page runs.
- A crafted link that carries launch parameters still loads the script in a plain tab, as every
  page did before. The server validates every session (SPEC-024), so the script gains no session
  there. This is recorded under #775.
- The web client's sync page and its route (#631) are not built here. A1 and A13 cover any route
  the routes table lists when they run.
- The card frame's own policy and sandbox (SPEC-341) are unchanged. #661 owns their next step.
- A third browser engine in the test matrix is #652's. A2 is measured in the suite's default
  browser.
- The native clients' card views are not the web client, and are untouched (#664).

## 6. Risks

- **Telegram's script is added after the document is parsed, not first in the head.** If it
  needs to run first, a launch inside Telegram shows no theme and answers "reopen". Detected by
  section 7's person-proved launch.
- **A Telegram client opens the Mini App with none of the three parameters in the fragment.** The
  page then treats itself as outside. Detected by section 7, on each client.
- **A browser ignores a policy added after the document is parsed.** Detected by A2 in the
  default test browser. The study suite runs the narrowed page in two engines (CI `web-engine`).
- **The threat model's T2 citation moves** when `csp.test.ts` changes above line 90. Detected by
  CI's `hygiene` check run (`scripts/tests/test_threat_model.py`), which names the row. The build
  re-cites it in the same delivery.
- **A sentence that still says "first in the head" survives in `docs/`.** Detected by
  `git grep -n -i -E 'first (script )?in (the )?.?<?head' -- docs`. Every hit carries this SPEC's
  note (R10).
- **A check outside this repository that expects the script first in the head reads red on the
  built site.** ADR-414 retires that rule for this app.
- **The first paint inside Telegram waits on the script**, as it did when the script blocked the
  parser.

## 7. What only a person proves

After the deploy, the owner opens the Mini App from Telegram on each client they use, and checks
four things:

- the theme applies;
- the first screen renders, and the loading bar clears;
- the day's study shows, so a session opened;
- after a reload from Telegram's menu, the screen still renders.

The owner then opens a route in a plain browser tab, and the browser's network panel lists no
request to telegram.org.

The record is a comment on #706 that names the clients.

## 8. Mutation testing

There are no hand mutation rows. StrykerJS mutates each changed production file whole with
`thresholds.break: 100`, and CI's `mutation-web` check run is the verdict.

- The files it mutates: `web/app/src/lib/telegram-launch.ts` and `web/app/src/hooks.client.ts`.
- The wrapper's change is to comment lines only, so the mutation plan mutates nothing in it.
- A survivor gets a test that observes its behaviour. Only a survivor proved equivalent is
  recorded, with its reason, in `scripts/mutation-equivalent.d/miniapp.json`.

## 9. Formal

NOT APPLICABLE (ADR-414 D4). The launch data's handoff is unchanged, and the gate adds no actor
over shared state. A formal cover can name no TypeScript file.

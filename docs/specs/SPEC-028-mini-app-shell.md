# SPEC-028: the Mini App opens inside Telegram with one typed wrapper, routes every startapp token, themes from Telegram and signs in once

- **Wave:** W0. **Issue:** #21 (epic #1). **Context(s):** `miniapp` (`web/app/src`).
- **Decided by:** ADR-005 (SvelteKit 2 SPA, runes, Tailwind 4, shadcn-svelte, Paraglide 2, the one wrapper), ADR-006 (initData sent raw, then a session), ADR-007 (same origin, `/api`), ADR-012 (Vitest and Playwright), ADR-018 (the source offer on the about screen), and this SPEC's ADR-028 (path routing with a closed startapp map, the token pipeline, the test packages).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-028.md` (ADR-016). Four
  statements and the manifest were amended in the delivery; section 7 gives each reason.

## 1. The problem, measured

- **What exists.** SPEC-002's skeleton builds `web/app` as a SvelteKit SPA with runes forced,
  `svelte-check` failing on warnings, one Vitest smoke test (`src/lib/smoke.test.ts`) and one
  Playwright smoke test. It loads no Telegram script, reads no launch parameter, has no theme, no
  locale, no route but its first page, and no path to the API.
- **What the platform requires** (the telegram-platform pack, the web-launch and vibecode-polish
  packs' Mini App rows, the accessibility and cjk-typography packs):
  - `telegram-web-app.js` first in `<head>`, `ready()` once the essential UI is up, and every
    method newer than the minimum version gated by `isVersionAtLeast` (`tg-sdk-script`, `tg-ready`,
    `tma-version-gate`);
  - launch parameters arrive in the URL hash, so routing is by path and the hash is read once
    before the router can discard it; `start_param` is signed only inside `initData`;
  - colours from `--tg-theme-*` variables with fallbacks, the theme-change event handled, the
    stable viewport height instead of `100vh`, safe areas (vibecode-polish's `telegram-*` rows);
  - every CJK run tagged with its `lang`, Chinese naming its script (`cjk-lang`,
    `zh-script-subtag`);
  - WCAG 2.2 AA, with a rendered axe audit because a static read cannot measure a rendered colour
    pair (the accessibility pack's `runtime-audit` and its `templates/a11y.spec.ts`).
- **The predecessor had no Mini App**; nothing here is a port. The one rule carried over is the
  study day: every screen shows the server's study day, never the phone's (`analytics.py:study_day`
  and the inventory's port note for `study-day-calendar`).
- **The wiring this lifts.** `.packs/wiring.json` holds `accessibility` and `cjk-typography`
  pending on this issue.

**Order.** This SPEC needs no Rust context: its API calls run against a mocked API in the tests,
matching the routes SPEC-024 serves (`POST /api/session`, `GET /api/me`). It can start as soon as
SPEC-002 has landed, beside SPEC-020. SPEC-021 declares this SPEC's `/about` route as a privacy
entry point, so it lands after this one.

## 2. Requirements

R1. `web/app/src/app.html` loads the official `telegram-web-app.js` from telegram.org as the first
    script in `<head>`; nothing is inlined into it.
R2. `web/app/src/lib/telegram.svelte.ts` is the only module that reads `window.Telegram.WebApp`,
    `TelegramWebviewProxy` or `initData`. It reads the launch data once, exposes typed runes state
    (colour scheme, theme parameters, stable viewport height, safe-area insets, platform, version,
    the signed start parameter), calls `ready()` exactly once after the first render and `expand()`
    once, subscribes to `themeChanged` and `viewportChanged`, and calls every method the official
    script throws `WebAppMethodUnsupported` on only behind `isVersionAtLeast` at that method's
    version. Outside Telegram it reports "not inside Telegram" and never throws.
R3. `web/app/src/lib/startapp.ts` maps a startapp token to a route through a closed table (at W0:
    `today` to `/`, `about` to `/about`). An empty, unknown or malformed token (not
    `[A-Za-z0-9_-]{1,64}`, or anything shaped like a path or a URL) opens Today, never a 404 and
    never a navigation to the token's text.
R4. The app routes by path (`adapter-static`, `fallback: index.html`, `ssr = false`). Telegram's
    script, first in `<head>`, parses the launch hash and keeps it for the session before any app
    code runs; the wrapper reads the script's object once, when it loads, which is before the
    router's first navigation, so a navigation never loses the launch.
R5. Design tokens are one DTCG file, `web/app/src/lib/design/tokens.json`, compiled at build by
    `web/app/scripts/build-tokens.ts` into CSS custom properties that Tailwind 4's `@theme` reads.
    Every colour token resolves to a `--tg-theme-*` variable with a fallback value, and every text
    and background pair the tokens declare meets WCAG 2.2 AA contrast in both of Telegram's
    default palettes (the palettes of the accessibility pack's `templates/a11y.spec.ts`).
R6. Paraglide JS 2 holds the UI strings (source locale `en`); `<html lang>` follows the active
    locale; a Chinese locale is written with its script subtag (`zh-Hans`, `zh-Hant`); the layout's
    stylesheet, `web/app/src/routes/layout.css`, imports the house CJK stylesheet
    `web/app/src/lib/styles/cjk.css`, which carries the CJK rules the house stack names (a `:lang()`
    font stack per language, from the system or sliced by `unicode-range`, `line-break: strict` for
    Japanese, `word-break: keep-all` for Korean, `word-break: auto-phrase` on Japanese and Korean
    headings as a progressive enhancement, never `break-all` on CJK).
R7. `web/app/src/lib/api.ts` is the one API client: at start it posts the raw `initData` string to
    `POST /api/session` once (never `initDataUnsafe`), then calls the API with the session cookie
    only (`credentials: "same-origin"`, same origin, no CORS). On a 401 it runs the handshake once
    more; if that fails too it shows "reopen DeckStreak from Telegram" and stops calling.
R8. Today shows the study day `GET /api/me` returns, never a date computed on the device.
R9. `/about` links the privacy policy and the repository's source code (the AGPL network-use offer,
    ADR-018), both as links opened through the wrapper's `openLink` (web-launch `tg-links`).
R10. `web/app/tests/a11y.spec.ts` (the skeleton's rendered audit, from the accessibility pack's
    template) runs axe-core with the WCAG 2.2 A and AA tags on every route of the route table, in
    both Telegram colour schemes, and runs in the gate's `pnpm -r test:e2e`; a Vitest test holds its
    route list equal to the router's route table.
R11. The acceptance lines below run from the repository root as
    `pnpm exec vitest run web/app/src/<file>.test.ts -t "<title>"`; a root `vitest.config.ts` that
    names `web/app` as its project makes that shape run (the skeleton already has it; SPEC-002).
R12. `.packs/wiring.json` moves `accessibility` and `cjk-typography` to `enforced`. A row with no
    subject until the readings render CJK text or ruby (`furigana-ruby` and `pinyin-tones`, which
    judge mentor texts) is listed under `deferred_rows` with the readings screen's issue; every other
    row is green.
R13. The binary-built packs that judge the built SPA (web-launch, vibecode-polish, ux-laws, ui-styles) run on
    the maintainer's box by `scripts/box-packs.sh`, and their verdicts are posted on the pull
    request (ADR-004).
R14. `web/app/svelte.config.js` sets `kit.csp` in hash mode with `script-src 'self'
    https://telegram.org`, `object-src 'none'`, `base-uri 'self'` and `connect-src 'self'`, so the
    page's own meta policy carries the build's script hashes and no inline script runs without
    one; the directives a meta policy cannot carry (`frame-ancestors`) are the Caddy header's
    (SPEC-032).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | only the wrapper touches `Telegram.WebApp`, `TelegramWebviewProxy` or `initData` (examined count) | `telegram-boundary.test.ts`; telegram-platform `tma-version-gate`, `tma-storage`, `tma-send-data` |
| A2 | an unknown or empty startapp token opens Today | `startapp.test.ts` |
| A3 | a token shaped like a path, a URL or longer than 64 characters opens Today | `startapp.test.ts` |
| A4 | the accessibility audit covers every route in both colour schemes | `a11y-coverage.test.ts`; the Playwright `web/app/tests/a11y.spec.ts` in `test:e2e`; accessibility markup rows |
| A5 | the handshake sends initData raw once, and later calls carry only the session | `api.test.ts` against a mocked API |
| A6 | an expired session re-handshakes once, then asks the owner to reopen | `api.test.ts` |
| A7 | the wrapper calls `ready()` once and gates each version-dependent method | `telegram.test.ts`; web-launch `tg-ready` on the box |
| A8 | every colour token resolves to a Telegram theme variable with a fallback | `tokens.test.ts`; vibecode-polish `telegram-theme-var-fallback` on the box |
| A9 | every declared text and background pair meets AA contrast in both Telegram palettes | `tokens.test.ts`; accessibility `contrast-pairs` |
| A10 | a Chinese locale names its script and the page language follows the locale | `i18n.test.ts`; cjk-typography `cjk-lang`, `zh-script-subtag` |
| A11 | Today shows the server's study day, not the device's date | `today.test.ts` |
| A12 | About links the privacy policy and the source code | `about.test.ts` |
| A13 | the page's policy admits scripts only from itself, its hashes and telegram.org | `csp.test.ts`; web-security `ws.csp-script-strict`, `ws.csp-object-base` on the box |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/telegram-boundary.test.ts -t "only the wrapper touches the Telegram WebApp object"
A2: pnpm exec vitest run web/app/src/lib/startapp.test.ts -t "an unknown or empty startapp token opens Today"
A3: pnpm exec vitest run web/app/src/lib/startapp.test.ts -t "a token shaped like a path or too long opens Today"
A4: pnpm exec vitest run web/app/src/lib/a11y-coverage.test.ts -t "the accessibility audit covers every route in both colour schemes"
A5: pnpm exec vitest run web/app/src/lib/api.test.ts -t "the handshake sends initData once and later calls carry only the session"
A6: pnpm exec vitest run web/app/src/lib/api.test.ts -t "an expired session re-handshakes once, then asks the owner to reopen"
A7: pnpm exec vitest run web/app/src/lib/telegram.test.ts -t "the wrapper calls ready once and gates each method by version"
A8: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every colour token resolves to a Telegram theme variable with a fallback"
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every text and background pair meets AA contrast in both Telegram palettes"
A10: pnpm exec vitest run web/app/src/lib/i18n.test.ts -t "a Chinese locale names its script and the page language follows the locale"
A11: pnpm exec vitest run web/app/src/routes/today.test.ts -t "Today shows the server's study day, not the device date"
A12: pnpm exec vitest run web/app/src/routes/about.test.ts -t "About links the privacy policy and the source code"
A13: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits scripts only from itself, its hashes and telegram.org"
```

The Vitest tests run in jsdom with a stubbed `window.Telegram.WebApp` (the shape of the
accessibility pack's template stub) and a mocked `fetch`; none reaches a network. The rendered axe
audit is Playwright's (`web/app/tests/a11y.spec.ts`), which the tdd probe cannot resolve; A4's
Vitest line proves it covers the routes, and the audit itself runs in the gate's web stage.

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/app.html` | `miniapp` | changed: the Telegram script first in `<head>` |
| `web/app/src/routes/layout.css` | `miniapp` | changed: Tailwind 4 `@theme` from the tokens; it keeps importing the CJK rules of `web/app/src/lib/styles/cjk.css` |
| `web/app/src/lib/telegram.svelte.ts` | `miniapp` | added: the one wrapper |
| `web/app/src/lib/startapp.ts` | `miniapp` | added: the closed token map |
| `web/app/src/lib/api.ts` | `miniapp` | added: the handshake and the session client |
| `web/app/src/lib/routes.ts` | `miniapp` | added: the route table the token map and the audit read |
| `web/app/src/lib/design/tokens.json` | `miniapp` | added: DTCG tokens |
| `web/app/scripts/build-tokens.ts` | `miniapp` | added: tokens to CSS custom properties |
| `web/app/src/routes/+layout.svelte`, `web/app/src/routes/+layout.ts` | `miniapp` | changed: the wrapper, the locale, `ssr = false` |
| `web/app/src/routes/+page.svelte` | `miniapp` | changed: Today's frame with the server's study day |
| `web/app/src/routes/about/+page.svelte` | `miniapp` | added |
| `web/app/messages/en.json`, `web/app/project.inlang/settings.json` | `miniapp` | changed: the shell's strings, in the skeleton's Paraglide 2 project |
| `web/app/src/lib/telegram-boundary.test.ts`, `web/app/src/lib/telegram.test.ts`, `web/app/src/lib/startapp.test.ts`, `web/app/src/lib/a11y-coverage.test.ts`, `web/app/src/lib/api.test.ts`, `web/app/src/lib/i18n.test.ts` | `miniapp` | added: A1 to A7, A10 |
| `web/app/src/lib/design/tokens.test.ts` | `miniapp` | added: A8, A9 |
| `web/app/src/routes/today.test.ts`, `web/app/src/routes/about.test.ts` | `miniapp` | added: A11, A12 |
| `web/app/src/lib/csp.test.ts` | `miniapp` | added: A13 |
| `web/app/tests/a11y.spec.ts` | `miniapp` | changed: the skeleton's audit (from the accessibility pack's template) reads its routes from `routes.ts` |
| `web/app/tests/telegram-palettes.ts` | `miniapp` | added (amended in delivery): Telegram's two default palettes, which the audit paints and A9 resolves |
| `web/app/.gitignore` | `miniapp` | changed (amended in delivery): the compiled `src/lib/design/tokens.css` is build output |
| `web/app/package.json`, `web/app/vite.config.ts`, `web/app/svelte.config.js`, `web/app/playwright.config.ts` | `miniapp` | changed |
| `vitest.config.ts`, `package.json`, `pnpm-lock.yaml` | repo | changed or added: the root Vitest project and the packages ADR-028 admits |
| `.packs/wiring.json` | repo | changed: accessibility and cjk-typography enforced, their subject-less rows deferred |
| `docs/DESIGN_SYSTEM.md` | repo | changed: the token file and its Telegram mapping |
| `docs/schematics/mini-app-launch.md` | repo | added |
| `docs/decisions/ADR-028-mini-app-routing-tokens-and-test-packages.md` | repo | added |
| `docs/red-first/SPEC-028.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It builds no readings screen, reader or history (#37).
- It builds no settings screen and stores nothing on the device (no CloudStorage, DeviceStorage or
  SecureStorage) (#57).
- It draws no chart (#152).
- It offers no linked sign-in with Google, Apple or a passkey (#58).
- It receives no in-app notification: the router's in-app transport is W1's (#27).
- It serves nothing: the Caddy block is a template in W0 and goes live in W2 (#25,
  #42).
- It is not the public landing page (#59).

## 6. Risks

- **Telegram's script changes under a pinned CSP.** The script is loaded from telegram.org by
  design, so the Caddy block's `script-src` names that origin (SPEC-032); a version change surfaces
  as a failing `isVersionAtLeast` gate in A7's stub matrix, never as a silent method call.
- **The launch is lost to a navigation.** Telegram's script parses the hash before any app code
  runs and keeps it for the session, so even a reload after a navigation keeps it; the wrapper
  reads the script's object once, when it loads (A7's companion test pins the launch data and the
  start parameter it reads), and `+layout.ts` routes the startapp token once, only when the launch
  carried one.
- **Contrast passes in the fallback palette and fails inside Telegram.** A9 measures both of
  Telegram's default palettes, and the rendered audit (A4's Playwright spec) measures the painted
  pairs.
- **The root Vitest project differs from the app's own config.** `vitest.config.ts` names the app's
  own `vite.config.ts` as its project, so both runs share one configuration; the skeleton's smoke
  test runs in both shapes.
- **Testing Library's browser build reaches a server render.** Its Vite plugin gives every Vitest
  environment Svelte's browser build, so a component that calls a Svelte lifecycle function while
  it renders breaks the skeleton's server-rendered smoke test (`lifecycle_outside_component`).
  Today loads in an `$effect`, which a server render skips; the smoke test names the failure if a
  screen it renders calls `onMount`.

## 7. Amended in delivery

The code proved four statements of the planned SPEC imprecise and the manifest short by two files.
Each is corrected above; the reasons are these.

- **R4.** Telegram's script, first in `<head>`, already parses the launch hash and keeps it for the
  session before any app code runs. The wrapper reads the script's object once; parsing the hash a
  second time would duplicate Telegram's parser.
- **R6.** The house stylesheet is the cjk-typography pack's template, byte for byte. It gives
  `auto-phrase` as a progressive enhancement (a browser drops a value it does not know) rather than
  behind `@supports`, and it uses the system's font stacks, with `unicode-range` slicing as the rule
  for any web font; every row of the pack reads it green.
- **R12.** `ruby-conformance` judges the app's own markup, which exists, and reads green (examined
  4), so it stays enforced; only `furigana-ruby` and `pinyin-tones` wait on the readings (#37).
- **Risks.** The launch risk now names the mechanism that was built, and the delivery found one new
  risk, the Testing Library build above.
- **The manifest.** `web/app/tests/telegram-palettes.ts` is added because the rendered audit and
  A9's contrast proof must judge the same two palettes, and Vitest cannot import a Playwright spec;
  `web/app/.gitignore` is changed because the compiled `tokens.css` is build output. Four listed
  files needed no change: `web/app/playwright.config.ts`, `web/app/project.inlang/settings.json`,
  the root `vitest.config.ts` and the root `package.json` already held what this delivery needs.
- **What existed.** The skeleton on `dev` had grown past section 1's reading: it already loaded
  Telegram's script first in `<head>`, compiled seven locales with `zh-Hans` and `zh-Hant`, set
  `<html lang>` per locale and imported the CJK stylesheet. That is why A10 was not red first.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).

## Amendment: SPEC-400, Telegram's script loads only on a launch

SPEC-400 (ADR-414) takes Telegram's script out of the page shell. The app's start hook adds it only
when Telegram launched the page, and awaits it before the router's first navigation. Outside a
launch it adds no script, and the page adds a second policy that refuses the script's origin.

So "first in `<head>`" no longer describes the page where this SPEC says it:

- line 16, the platform's requirement;
- lines 40-41, R1: the shell now loads no script, and still inlines nothing (SPEC-400 A11);
- line 54, R4;
- line 139, the manifest's `web/app/src/app.html` row;
- lines 206 and 223, which record the shell as this delivery found and left it.

What "first in `<head>`" was for, the script running before any navigation so that no navigation
loses the launch, is now held by SPEC-400's A3.

---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The Mini App front end: SvelteKit 2 on the house golden path, served as a static SPA

## Context and Problem Statement

The Mini App runs in Telegram's webview on a mid-range phone and talks to the axum API. The
earlier instruction to weigh Rust to WebAssembly against TypeScript was superseded by the owner's
stack decision: Rust-first, TypeScript at the UI, with an owner-approved golden path for app UIs.

## Decision Drivers

- The owner-approved golden path `app-ui`: SvelteKit 2, Svelte 5 runes, TypeScript 6, Tailwind 4, shadcn-svelte and Bits UI, DTCG tokens mapped onto Telegram's theme.
- Svelte's compiler guardrails: runes mode forced and svelte-check in CI.
- The Telegram Mini App SDK must be the official script behind one typed wrapper.

## Considered Options (the alternatives it was chosen against)

- SvelteKit 2 with adapter-static as an SPA, the official `telegram-web-app.js` behind a typed runes wrapper — chosen: the golden path; runes mode turns every Svelte 4 construct a model emits into a compile error.
- Leptos or Dioxus (Rust to WebAssembly) — rejected because the radar holds both for UI work; WebAssembly is admitted only by a measured exception (a hot path over 50 ms on a mid-range phone), which no DeckStreak screen has.
- `@telegram-apps/*` SDK packages — rejected because the radar holds them as deprecated.
- SvelteKit remote functions or SSR — rejected because the radar holds remote functions in Rust-backend repositories, and a Mini App behind Caddy needs no server rendering.

## Decision Outcome

Chosen option: `web/app` is a SvelteKit 2 SPA (`adapter-static`, `fallback: index.html`,
`ssr = false`), with `compilerOptions.runes = true`, Tailwind 4, shadcn-svelte and Bits UI,
Paraglide JS 2 with the CJK locale rules (`zh-Hans`, `zh-Hant`), TanStack Query 6 against the
axum API, and Chart.js 4 lazy-loaded for charts rendered on the client (so no chart is drawn inside
the API's memory budget, ADR-032). `telegram-web-app.js` is loaded first in `<head>`, and
`src/lib/telegram.svelte.ts` is its one typed wrapper, gating each method with
`isVersionAtLeast`. Caddy serves the build with an SPA fallback and proxies `/api/*` to axum on
the same origin, so no CORS is needed.

### Consequences

- Good, because the golden path needs no deviation ADR.
- Good, because charts move to the client, keeping chart rendering out of the API's memory budget.
- Bad, because the team carries two languages; the boundary is the HTTPS API, typed on both sides.

### Confirmation

The stack-selection rows `svelte-runes`, `svelte-check-ci`, `pinned-majors`, `no-hold-items`; the telegram-platform and web-launch rows for the Mini App.

## What would make this wrong

- A screen needs a CPU-bound computation above 50 ms on a mid-range phone (measured), which would admit a WebAssembly exception by ADR.

## More Information

The stack-selection pack; the telegram-platform pack's Mini App rows; ADR-007 for the same-origin API.

Amendment (2026-09-28): `pnpm-workspace.yaml` overrides two development-only transitive
dependencies, because no update inside a parent's major reaches a version that fixes their
advisories. SvelteKit 2 declares `cookie ^0.6.0`, so `@sveltejs/kit@2>cookie` takes `>=0.7.0 <0.8`
(GHSA-pxg6-pf52-xh8x, fixed in 0.7.0). StrykerJS 10 (ADR-057) takes `typed-rest-client ~2.3.0`,
whose 2.3.1 pins `qs` 6.15.1 exactly, so `typed-rest-client@2>qs` takes `>=6.16.0 <7`
(GHSA-q8mj-m7cp-5q26, GHSA-4mjr-xmp4-gh2g and GHSA-x5fp-wj9c-mxmx; 6.16.0 is the first version
outside all three). SvelteKit keeps `cookie` 0.6 in its 2.x line because 0.7 holds a cookie's name,
path and domain to RFC 6265 in `serialize`, which breaks an app that sets a non-conforming name;
the Mini App has no server at runtime and sets no cookie through SvelteKit. Each selector names its
parent's major, so neither entry applies to a later major, which declares a fixed range itself, and
an entry whose parent no longer matches is removed. Rejected: a global `cookie` or `qs` override,
which would also force a later SvelteKit major, or any other consumer, onto the older line; holding
`typed-rest-client` at 2.3.0, whose `qs ^6.14.1` admits a fixed release, in the lockfile alone,
which the next resolution undoes with nothing recording why; and accepting the advisories as
build- and test-time only, while a fix exists that changes no byte of the built Mini App.

Amendment: `pnpm-workspace.yaml` overrides a third development-only transitive dependency.
Mermaid 11 declares a `katex` range that no update reaches past, so none inside it gets a version
that fixes GHSA-238p-pmpm-9mq7, and `mermaid@11>katex` takes `>=0.18.2 <0.19`, the first fixed
line. The same audit also named `source-map-js` (GHSA-68fv-2mgg-jv7q); PostCSS, css-tree and Tailwind's node
package each declare a range that admits the fixed release, so that one moved in the lockfile alone
by an update and takes no override. The selector names Mermaid's major, so it applies to no later
major, and it is removed when the parent no longer matches. Rejected: a global `katex` override,
which would also force any other consumer onto the newer line, and an audit ignore, which would
leave the advisory unfixed.

Amendment (2026-09-28): passages describing another service's operations were replaced with the
API's own memory budget under the public-text rule (ADR-059).

## Amendment: ADR-414, Telegram's script loads only on a launch

Line 34 says `telegram-web-app.js` is loaded first in `<head>`. ADR-414 (SPEC-400) replaces that:
the app's start hook adds the script only when Telegram launched the page, and awaits it before the
router's first navigation, so it still runs before any app code reads the launch. Outside a launch
the page loads no script from Telegram and adds a policy that refuses that origin.
`src/lib/telegram.svelte.ts` stays the script's one typed wrapper, and the rest of this decision is
unchanged.

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

Amendment (2026-09-28): passages describing another service's operations were replaced with the
API's own memory budget under the public-text rule (ADR-059).

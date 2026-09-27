---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The Mini App routes by path through a closed startapp map, compiles its DTCG tokens in-repo, and admits three test packages

## Context and Problem Statement

ADR-005 fixes the Mini App's stack (SvelteKit 2 SPA, runes, Tailwind 4, shadcn-svelte, Paraglide 2,
the official Telegram script behind one typed wrapper). SPEC-028 builds the shell, and three things
ADR-005 leaves open shape every later screen: how a startapp token becomes a screen, how the design
tokens reach the CSS, and which test packages the shell's tests may use.

## Decision Drivers

- Telegram delivers launch parameters in the URL hash; `start_param` is signed only inside
  `initData`, and a deep link's `startapp` is text anyone can craft (the telegram-platform pack).
- A deep link must never land on a 404 or navigate to arbitrary text.
- One source of truth for colours, mapped onto `--tg-theme-*` with fallbacks (vibecode-polish), and
  a contrast proof in both of Telegram's palettes (accessibility).
- No npm package without an ADR (the pack-wave method's R4 to R6, and the builder brief's refusals).

## Considered Options (the alternatives it was chosen against)

- Path routing with a closed token-to-route table, the launch hash read once by the wrapper before the router starts — chosen: a token can only ever open a route the table names, and an unknown one opens Today.
- Hash routing — rejected because Telegram uses the hash for its launch parameters, and a hash router either discards them or has to share the fragment with them.
- A startapp value that is itself a route path — rejected because any crafted link would become a navigation (a 404 at best, an open-redirect shape at worst), and route renames would break old links.
- DTCG tokens compiled to CSS custom properties by a small in-repo script at build — chosen: a few dozen tokens need a loop, not a framework, and the script's output is what Tailwind 4's `@theme` reads.
- Style Dictionary — rejected because it is another dependency and configuration surface for a transform the in-repo script does in one file.
- Hand-written CSS variables beside a tokens document — rejected because two sources of truth drift, and the contrast test would judge the document rather than what ships.
- `@testing-library/svelte` with `jsdom` for component tests, and `@axe-core/playwright` for the rendered audit — chosen: the testing-js golden path's runners (Vitest 4, Playwright) need a DOM and an axe binding, and the accessibility pack's template names `@axe-core/playwright`.
- `happy-dom` instead of `jsdom` — rejected because jsdom is the environment the Svelte and Testing Library documentation use, so a builder's reference examples hold.

## Decision Outcome

Chosen options as above. `web/app/src/lib/routes.ts` is the route table; `startapp.ts` maps
tokens onto it and nothing else; `web/app/tests/a11y.spec.ts` and A4's coverage test read the same
table. `web/app/scripts/build-tokens.ts` runs before `vite build` and before the Vitest run that
reads its output. The packages admitted, as `devDependencies` of `web/app`:
`@testing-library/svelte`, `jsdom`, `@axe-core/playwright`. A root `vitest.config.ts` names `web/app`
as its project so the acceptance commands run from the repository root.

### Consequences

- Good, because a crafted deep link can only open a screen the table lists.
- Good, because the contrast proof judges the exact variables that ship.
- Bad, because a new screen touches the route table as well as its route file; the audit's coverage
  test fails until it does, which is the point.

### Confirmation

SPEC-028's acceptance tests A2, A3, A4, A8 and A9; web-launch `tg-deep-links` and `tg-startapp` and
vibecode-polish `telegram-theme-var-fallback` on the box.

## What would make this wrong

- Telegram stops putting launch parameters in the hash, or signs `startapp` outside `initData`.
- The token set grows past what one file of DTCG tokens can hold readably (then adopt a transform
  tool by a new ADR).

## More Information

ADR-005; ADR-006; SPEC-028; the telegram-platform, accessibility, cjk-typography, web-launch and
vibecode-polish packs; `docs/schematics/mini-app-launch.md`.

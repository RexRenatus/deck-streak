---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The reader: a closed renderer instead of HTML, the day's readings cached by study day, and the accessibility audit in Vitest's browser mode

## Context and Problem Statement

The reader is the flagship's screen. It shows AI text grounded in the owner's own cards, on an
origin that holds the owner's session, and a language reading needs ruby for furigana. The owner
reads before an Anki session, sometimes offline. The accessibility pack asks for an axe-core audit of
every route in both Telegram colour schemes, which needs real layout and colour; the tdd probe
resolves Vitest commands but no Playwright command, so a Playwright line in a SPEC's fence cannot be
checked at promotion. How should the reader render, cache and be audited?

## Decision Drivers

- The reader renders the validated text only, never HTML or links from the body (DS-W1-12's rule).
- Japanese readings need `<ruby>` and `<rt>`; the vault-duties rails allow them, and so must the reader.
- The reading works offline for the day, and the cache is cleared at the rollover.
- Every acceptance line resolves to a test the tdd probe can find.

## Considered Options (the alternatives it was chosen against)

- A closed parser that builds text, headings, paragraphs, lists, blockquotes, emphasis and ruby from their validated form, with links shown as text; the day's readings in the webview's local storage under a key carrying the server's study day; the audit as a Vitest browser-mode test on headless Chromium, admitting `@vitest/browser-playwright` and `axe-core` as dev dependencies — chosen: no body markup ever becomes live, the cache is small and self-clearing, and every line resolves.
- Render the body with `{@html}` after a sanitiser such as DOMPurify — rejected because a sanitiser bug is script on the owner's authenticated origin, and a sanitised link would still be live.
- A service worker that caches the readings routes — rejected because it caches whole responses beyond the readings, has its own update lifecycle in Telegram's webview, and is harder to clear at exactly the study day's rollover.
- Telegram's DeviceStorage for the cache — rejected because it needs a newer client version gate and adds a Telegram storage key the privacy inventory must declare for no gain over local storage.
- The audit as a Playwright spec — rejected for the acceptance line because the tdd probe resolves no Playwright command; the shell's Playwright smoke test keeps running beside it.
- Vitest in its DOM emulation — rejected for the audit because it computes no layout or colour, so axe-core's contrast rule cannot run in either colour scheme.

## Decision Outcome

Chosen option. `render.ts` is the only path from a reading to markup, and a test holds it to golden
readings, ruby included. `offline.ts` keys each cached reading by the server's study day and removes
earlier days when the server's study day changes. The repository root's `vitest.config.ts`, which
declares the Vitest projects, gains a browser project for `web/app/src/**/*.browser.test.ts`, and
`web/app/vite.config.ts`'s own run keeps those files out of Node; the audit runs the WCAG 2.2 A and
AA tag sets in both colour schemes.

### Consequences

- Good, because nothing in a reading's body can run, fetch or navigate.
- Good, because the reader opens offline for the day it was read.
- Bad, because the renderer supports only the constructs the persona outputs use; anything else shows
  as text until the renderer learns it.
- Bad, because two Mini App test harnesses exist (Vitest's browser mode and the shell's Playwright
  smoke test), both on the same Chromium.

### Confirmation

SPEC-051's tests; the accessibility and cjk-typography rows over the readings screens.

## What would make this wrong

- The tdd probe learns to resolve Playwright commands (then the audit may move back to a Playwright spec).
- A reading needs a construct the renderer cannot build safely (then its form changes, not the renderer's rule).

## More Information

SPEC-051; ADR-005; ADR-012; the accessibility pack's runtime audit; the cjk-typography pack; Vitest 4's
browser mode with the Playwright provider.

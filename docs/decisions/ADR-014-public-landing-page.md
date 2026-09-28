---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The public landing page: Astro with Svelte islands, SEO-gated, on an owned domain

## Context and Problem Statement

The Mini App is private to its owner and hidden from search. The product also needs a public
page that explains it, links to the bot, carries the trademark notice for Anki, and ranks for its
keywords. The naming study fixed its title, headings, canonical and JSON-LD.

## Decision Drivers

- The golden path `webpage`: Astro 7 with `@astrojs/svelte` 9 and Svelte islands.
- The seo-pipeline and web-launch packs gate it; both need an https canonical on an owned host.
- No forward-looking dates and no personal data on a public page.

## Considered Options (the alternatives it was chosen against)

- A static Astro site in `web/site`, served by Caddy on the owned apex domain, gated by seo-pipeline and web-launch — chosen: the golden path, and the gates the naming study already measured.
- Serve the landing page from the Mini App's host — rejected because the Mini App's host carries `noindex` and its bundle would trip the landing page's single-CTA and https checks.
- No landing page — rejected because the bot needs a public, crawlable explanation and a privacy policy link.

## Decision Outcome

Chosen option: `web/site` (W7) follows the naming conventions (title, h1, meta description,
canonical, flat `SoftwareApplication` JSON-LD, `robots.txt`, `sitemap.xml`, real `vitals.json`),
links `/privacy.html` and `/terms.html`, and carries the footer "Anki® is a registered trademark of
Ankitects Pty Ltd. DeckStreak is not affiliated with or endorsed by Anki." It goes live only when
the owner provides a domain (OWNER-SETUP); until then it is built and gated, not served.

### Consequences

- Good, because the gates are known before the page is written.
- Bad, because it waits on an owner purchase to be public.

### Confirmation

`<binary> verify seo-pipeline --subject web/site/dist` and `<binary> pack probe --pack web-launch --root web/site/dist` on the box (ADR-004).

## What would make this wrong

- The owner chooses not to buy a domain: the page then stays unpublished, and the bot's description carries the privacy link.

## More Information

The seo-pipeline and web-launch packs; ADR-007.

Amendment (2026-09-28): names of the maintainer's private tooling were replaced with 'the box-run
packs' and neutral names for their repository, binary and checkout under the public-text rule
(ADR-059).

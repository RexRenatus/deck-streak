# SPEC-139: the public landing page is a static Astro site that states only what DeckStreak does, on an origin the owner configures

- **Wave:** W7. **Issue:** #59 (the public landing page) (epic #8). **Context(s):** `landing`
  (`web/site`, the Astro site, planned in the CONTEXT-MAP), `repo` (`scripts/landing_rules.py`,
  the box run's site build, the pnpm workspace and the root Vitest configuration).
- **Decided by:** ADR-014 (the landing page: Astro with Svelte islands, SEO-gated, on an owned
  domain), ADR-139 (this SPEC's: the box run builds the site from the judged commit, and the built
  site is never committed), ADR-059 (public text describes DeckStreak only), ADR-013 (the
  repository layout names `web/site`), ADR-012 (Vitest and Playwright), ADR-030 (the box runner
  uses each pack's own verb).
- **Prerequisites:** SPEC-021 (`PRIVACY.md`, which the privacy page states), SPEC-022 (the sync the
  page describes), SPEC-028 (the pnpm workspace and the root Vitest configuration), SPEC-030 (whose
  exclusion hands the site's build for the box run to #59), SPEC-056 (the box run) and SPEC-058
  (the web audit, which judges the site's packages), and the features its fixed copy names (R4,
  R8): SPEC-072 (XP per review and levels), SPEC-073 (badges), SPEC-076 (streaks), SPEC-080
  (quests) and SPEC-101 (the daily digest). The first six have landed; SPEC-072, SPEC-073,
  SPEC-076, SPEC-080 and SPEC-101 are unlanded. The page is built and
  gated now, and served only once the owner has a domain (#168). **Mutation band:**
  `S13900-S13999`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-139.md` (ADR-016).

## 1. The problem, measured

- **No landing page exists.** `web/` holds `web/app` only; the CONTEXT-MAP names `landing`
  (`web/site/src`) as planned, with no code, depending on nothing internal.
- **The gates are known.** ADR-014 names the seo-pipeline and web-launch packs as the page's gates,
  both needing an https canonical on an owned host, and fixes the trademark footer.
- **The box run cannot see a site yet.** `scripts/box-packs.sh` verifies `web/site/dist` only when
  the judged commit holds it, and SPEC-030 §5 leaves building it for the run to #59. The run
  exports a commit with `git archive`, and a build product is not in a commit.
- **No domain is chosen.** #168 is the owner's decision. Until it lands the page has no public
  origin, and the Mini App's host carries `noindex` and a robots rule that disallows everything
  (`deploy/caddy/deck-streak.caddy`), so the page cannot share it (ADR-014).
- **The product is still arriving.** Features the page could describe land across several waves;
  a section for one that has not landed would claim something the product does not do.

## 2. Requirements

The site

R1. `web/site` is an Astro 7 site with `@astrojs/svelte` 9 (ADR-014), a member of the pnpm
    workspace, built statically to `web/site/dist`. It ships no island and no script beyond the
    JSON-LD block; an island is added only when a section needs one.
R2. The origin is configuration read at build: `PUBLIC_SITE_ORIGIN` (the neutral example
    `https://deckstreak.example`), used as Astro's `site`, and `PUBLIC_BOT_URL` (the neutral example
    `https://t.me/ExampleBot?startapp`) for the one call to action. `web/site/src/site-config.ts`
    refuses an origin that is not `https://` with a lowercase host and no path, and a bot URL that
    is not on `https://t.me/`.
R3. The built tree is flat: `index.html`, `privacy.html`, `terms.html`, `404.html`, `hero.svg`,
    `favicon.svg`, `og-card.png`, `robots.txt`, `sitemap.xml` and, after R12's measurement,
    `vitals.json`. Links between pages are root-relative to files that exist (`/privacy.html`), with
    no query string, no fragment and no extensionless path; a bare `#section` link is allowed.
    `.gitignore` names `web/site/dist/` and `web/site/.astro/`, so no build product is committed
    (ADR-139).

The page's head

R4. Each page's head opens with `<meta charset="utf-8">` and then its meta description, the first
    tag with a `content=` attribute. The description is 50 to 160 characters (seo-pipeline's bound),
    plain ASCII with no
    double quote. The title is `<keyword phrase> | DeckStreak`, 10 to 70 characters (seo-pipeline's
    bound), plain ASCII,
    unique per page. The pages' titles and descriptions are:
    - the landing page: `Gamify Anki in Telegram: XP, Streaks and Quests | DeckStreak`, and
      `DeckStreak turns your Anki reviews into XP, levels, streaks, quests and badges in a Telegram
      mini app, with daily progress digests from its bot.`;
    - the privacy page: `Privacy Policy | DeckStreak`, and `How DeckStreak handles your Anki review
      data, Telegram account id and progress records, what is stored, for how long, and how to
      erase it.`;
    - the terms page: `Terms of Use | DeckStreak`, and `The terms for using DeckStreak, the Telegram
      mini app and bot that turn your Anki reviews into XP, levels, streaks and quests.`;
    - the 404 page: `Page Not Found | DeckStreak`, with a description of its own within the same
      bounds.
R5. `index.html`, `privacy.html` and `terms.html` each carry `<link rel="canonical">` with the page's
    absolute URL on the origin (the root with a trailing slash, the others with `.html`); the same
    string appears byte for byte in `og:url` and in the sitemap's `<loc>`, and the landing page's
    is also the JSON-LD `url`. `404.html` has no canonical and is not in the sitemap.
R6. The first `application/ld+json` block of `index.html` is one flat object: `@context`
    `https://schema.org`, `@type` `SoftwareApplication`, `name` `DeckStreak`, `url` the canonical,
    `applicationCategory` `EducationalApplication`, `operatingSystem` `Telegram`, and `description`
    equal to the meta description. It carries no price, rating or review.
R7. `robots.txt` is exactly `User-agent: *`, `Allow: /` and `Sitemap: <origin>/sitemap.xml`, with no
    `Disallow` line; no page carries a robots meta tag. No `http://` appears in the tree except the
    sitemap's namespace and an SVG's `xmlns`.

The page's body

R8. Each page has exactly one `<h1`, the first heading. The landing page's is `Turn your Anki reviews
    into XP, streaks and quests`, followed by a paragraph of at least 40 characters (seo-pipeline's
    bound) that says how DeckStreak reads reviews (through the user's own Anki sync server,
    SPEC-022), the hero image,
    and the one call to action (`data-cta`), `Open DeckStreak in Telegram`, to `PUBLIC_BOT_URL`.
R9. Each `<h2>` of the landing page names a feature, and `web/site/src/features.json` maps each
    heading to the SPEC that delivered it. A heading whose SPEC is still under
    `docs/specs/planned/`, or that names no SPEC, is refused, so the page claims nothing the product
    does not do (binding decision 11). A later delivery adds its own heading when its SPEC moves.
    The two headings that describe no single feature, `How DeckStreak reads your Anki reviews`
    (SPEC-022) and `Privacy` (SPEC-021), map to their SPECs the same way. R16's checker
    reads the title, the meta description and the `<h1` the same way: each feature they name maps in
    `features.json` to a delivered SPEC, or is a finding (A8).
R10. Every page's footer is ADR-014's notice, `Anki® is a registered trademark of Ankitects Pty Ltd.
     DeckStreak is not affiliated with or endorsed by Anki.`, with links to `/privacy.html` and
     `/terms.html`.
R11. Every `<img>` carries `alt`, `width` and `height`; the landing page has at least one, the hero
     `hero.svg` (`data-lcp`), whose alt text describes what the image shows in 125 characters or
     fewer (a chosen bound). Each raster, `og-card.png` included, is 30,000 bytes or less
     (web-launch's budget).
R12. `vitals.json` is written only by `web/site/scripts/measure-vitals.ts`, which serves `dist` on
     loopback, loads `index.html` in Playwright's Chromium with the `web-vitals` library, and records
     the measured values with the tool versions. It writes into `dist`, which is never committed,
     so the file holds no value that was not measured.
R13. The pages set no cookie, carry no form, no analytics and no resource from another origin. The
     privacy page says so, and states what `PRIVACY.md` states about the Mini App and the bot, with
     no personal value; the terms page states the licence (AGPL-3.0-or-later), the absence of
     warranty and the trademark notice.
R14. Every page is `lang="en"`, meets contrast AA in the light and dark schemes, and honours reduced
     motion. The site's colours are tokens in its own stylesheet: `landing` depends on nothing
     internal, so it never imports the Mini App's.
R15. Every sentence of the site passes the public text rule (ADR-059): it describes DeckStreak only,
     names no host, account or person, and carries no forward-looking date and no promise of a
     feature.

The rules, in the tree and on the box

R16. `scripts/landing_rules.py` checks a built tree against R3 to R11 (the head's order and bounds,
     the canonical's agreement, the JSON-LD's shape, the robots file, the `http://` rule, the links,
     one `<h1` first, one `data-cta`, the footer, the images, the raster sizes, and R9's map read
     against `docs/specs/`) and names each finding; it reads files only. `web/site`'s `test:e2e`
     script runs it over `dist` after the build, then R12's measurement, so the web stage of
     `scripts/check.sh` gates the built site.
R17. The box run builds the site from the judged commit (ADR-139): when the exported tree holds
     `web/site/package.json`, `scripts/box-packs.sh` runs `pnpm install --frozen-lockfile
     --prefer-offline`, then `web/site`'s `build` and its vitals measurement, inside the export,
     with R2's neutral example values, before the box section. A build that fails leaves no
     `web/site/dist`, which the run already reads as pending on the issue its wiring names, or as
     VOID and failed by name when it names none (SPEC-056), never as a pass. The build's lines go to
     the run's output directory, and the run prints the build's exit.
R18. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no dishonest
     copy (R9, R13, R15), no claim the sync cannot honour (R8 says reviews are read through the
     user's sync server, which is what the sync does), and no secret, address or forward-looking
     date on anything public (R2's neutral examples, R15).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the landing page renders R4's title and description, the description the first `content=` tag after the charset | `the landing head carries the title and the description first` |
| A2 | the canonical, `og:url` and the JSON-LD `url` are one string on the configured origin | `the canonical og url and json-ld url agree` |
| A3 | the JSON-LD block is a flat `SoftwareApplication` with no price, rating or review | `the json-ld is a flat software application` |
| A4 | the landing page has one `<h1`, first, and exactly one call to action to the configured bot URL | `the landing page has one h1 first and one call to action` |
| A5 | the landing, privacy, terms and 404 pages each carry the trademark footer and the two policy links | `every page carries the trademark footer` |
| A6 | the 404 page has no canonical, and the sitemap lists the three canonical pages only | `the 404 page has no canonical and is not in the sitemap` |
| A7 | an origin that is not lowercase `https://` with no path, or a bot URL off `https://t.me/`, is refused | `an origin that is not https is refused` |
| A8 | a feature heading whose SPEC is planned, or that names none, is a finding, and a delivered one passes | `test_every_feature_heading_names_a_delivered_spec` |
| A9 | the rules pass over a planted tree that keeps them all | `test_a_tree_that_keeps_every_rule_passes` |
| A10 | a description after another `content=` tag is a finding | `test_a_description_that_is_not_first_is_refused` |
| A11 | a title of 9 or 71 characters is a finding, and 10 and 70 pass | `test_a_title_outside_its_bounds_is_refused` |
| A12 | a JSON-LD `url` that differs from the canonical is a finding | `test_a_json_ld_url_that_differs_is_refused` |
| A13 | a second `<h1`, or an `<h2` before it, is a finding | `test_a_second_or_late_h1_is_refused` |
| A14 | an `http://` outside the two namespaces is a finding | `test_a_plain_http_link_is_refused` |
| A15 | an image without `alt`, `width` or `height`, or a raster of 30,001 bytes, is a finding, and 30,000 passes | `test_an_image_without_its_attributes_or_over_budget_is_refused` |
| A16 | a relative link with a query, a fragment or no extension is a finding, and a bare `#section` passes | `test_a_relative_link_with_a_query_or_fragment_is_refused` |
| A17 | a `Disallow` line in `robots.txt` is a finding | `test_a_disallow_line_is_refused` |
| A18 | with the site in the judged commit, the box run installs, builds and measures inside the export before the box section, with the neutral values (a recording `pnpm` double on the path) | `test_the_site_is_built_in_the_judged_tree_before_the_box_section` |
| A19 | a site build that fails leaves the seo-pipeline card unrun, and the pack VOID by name when its wiring names no pending issue | `test_a_failed_site_build_is_never_a_pass` |

```acceptance
A1: pnpm exec vitest run web/site/src/pages/pages.test.ts -t "the landing head carries the title and the description first"
A2: pnpm exec vitest run web/site/src/pages/pages.test.ts -t "the canonical og url and json-ld url agree"
A3: pnpm exec vitest run web/site/src/pages/pages.test.ts -t "the json-ld is a flat software application"
A4: pnpm exec vitest run web/site/src/pages/pages.test.ts -t "the landing page has one h1 first and one call to action"
A5: pnpm exec vitest run web/site/src/pages/pages.test.ts -t "every page carries the trademark footer"
A6: pnpm exec vitest run web/site/src/pages/pages.test.ts -t "the 404 page has no canonical and is not in the sitemap"
A7: pnpm exec vitest run web/site/src/site-config.test.ts -t "an origin that is not https is refused"
A8: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_every_feature_heading_names_a_delivered_spec
A9: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_tree_that_keeps_every_rule_passes
A10: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_description_that_is_not_first_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_title_outside_its_bounds_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_json_ld_url_that_differs_is_refused
A13: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_second_or_late_h1_is_refused
A14: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_plain_http_link_is_refused
A15: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_an_image_without_its_attributes_or_over_budget_is_refused
A16: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_relative_link_with_a_query_or_fragment_is_refused
A17: python3 -m unittest discover -s scripts/tests -p test_landing_rules.py -k test_a_disallow_line_is_refused
A18: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_the_site_is_built_in_the_judged_tree_before_the_box_section
A19: python3 -m unittest discover -s scripts/tests -p test_box_packs.py -k test_a_failed_site_build_is_never_a_pass
```

## 3a. What the box run judges

The private-wiring change that enforces B1 to B3 lifts the pending entries that wait on #59 for
the seo-pipeline, web-launch and vibecode-polish packs, so each pack is judged over the site R17
builds; the delivery hands it back as a JSON diff. The box builds `web/site` with the neutral
example origin, so the packs judge the page before #168.

| id | criterion | decided by |
|---|---|---|
| B1 | over `web/site/dist/index.html`, the one landing page: title, description, canonical, one h1, and JSON-LD of a known type whose url is the canonical | the seo-pipeline pack |
| B2 | over the files of `web/site/dist/` (R3's ten): language, alt text, contrast, the 404 page, the policy pages, robots and sitemap, one call to action, image budgets, and measured vitals | the web-launch pack |
| B3 | over the four pages of `web/site/dist/`: favicon, titles and meta, the footer, compressed images, and every page usable at phone width | the vibecode-polish pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/site/package.json` | landing | added: astro, `@astrojs/svelte`, `web-vitals`, Playwright; `check`, `test`, `build` and `test:e2e` scripts |
| `web/site/astro.config.mjs` | landing | added: static output, `site` from `PUBLIC_SITE_ORIGIN` |
| `web/site/vitest.config.ts` | landing | added: `getViteConfig` with the site's `root` and config file as its inline Astro configuration, the node environment |
| `web/site/tsconfig.json` | landing | added |
| `web/site/src/site-config.ts` | landing | added: R2's configuration and its refusals |
| `web/site/src/site-config.test.ts` | landing | added: A7 |
| `web/site/src/layouts/Page.astro` | landing | added: the head (R4 to R7), the footer (R10) |
| `web/site/src/pages/index.astro` | landing | added: the landing page |
| `web/site/src/pages/privacy.astro` | landing | added: the privacy page (R13) |
| `web/site/src/pages/terms.astro` | landing | added: the terms page (R13) |
| `web/site/src/pages/404.astro` | landing | added |
| `web/site/src/pages/sitemap.xml.ts` | landing | added: from the origin |
| `web/site/src/pages/robots.txt.ts` | landing | added: from the origin |
| `web/site/src/pages/pages.test.ts` | landing | added: A1 to A6, through Astro's container API |
| `web/site/src/styles/site.css` | landing | added: the site's colour tokens, both schemes, reduced motion (R14) |
| `web/site/src/features.json` | landing | added: each heading and its delivered SPEC |
| `web/site/public/hero.svg` | landing | added |
| `web/site/public/favicon.svg` | landing | added |
| `web/site/public/og-card.png` | landing | added: 30,000 bytes or less |
| `web/site/scripts/measure-vitals.ts` | landing | added: R12 |
| `pnpm-workspace.yaml` | repo | changed: `web/site` joins |
| `pnpm-lock.yaml` | repo | changed |
| `vitest.config.ts` | repo | changed: `web/site/vitest.config.ts` joins as a second project |
| `.gitignore` | repo | changed: `web/site/dist/` and `web/site/.astro/` |
| `scripts/landing_rules.py` | repo | added: R16's checker |
| `scripts/tests/test_landing_rules.py` | repo | added: A8 to A17 |
| `scripts/tests/fixtures/landing/` | repo | added: the planted trees |
| `scripts/box-packs.sh` | repo | changed: R17's site build |
| `scripts/tests/test_box_packs.py` | repo | changed: A18 and A19 |
| `docs/CONTEXT-MAP.md` | docs | changed: `landing` is no longer planned |
| `docs/specs/SPEC-139-the-public-landing-page-is-a-static-astro-site-that-states-only-what-deckstreak-does-on-an-origin-the-owner-configures.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-139.md` | docs | added |
| `scripts/mutation-rows.d/S13900-S13999.json` | repo | added: §9's rows |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It serves nothing and chooses no domain: the Caddy site block for the landing host, its
  redirects and its certificate follow the owner's domain (#168).
- It adds no price, rating or review to the JSON-LD until each is real (#59).
- It describes no feature whose SPEC has not been delivered; each delivery adds its own heading
  (#59).
- It sets no cookie and runs no analytics, so it asks no consent (#59).
- It translates nothing: the landing page is English, and the Mini App keeps its seven locales
  (#59).

## 6. Risks

- **A sentence that claims an unbuilt feature.** R9's delivered-SPEC map; detected by A8.
- **A head the gates read differently.** R16's checker holds the order and the bounds; detected by
  A10 to A13.
- **An invented vital.** R12 writes `vitals.json` into `dist`, and nothing commits it (R3).
- **The root Vitest configuration moves the process into `web/app`** before SvelteKit's plugin
  loads. The site's project names its own `root` and Astro configuration file through
  `getViteConfig`'s second argument, so its tests never read the Mini App's configuration.
- **The box build fails on a missing package.** R17 reads a failed build as a missing site, which
  the run already refuses by name; detected by A19.
- **A page the Mini App's host would hide.** ADR-014 keeps the page off that host; #168 gives it its
  own.

## 7. Parity goldens

None: the landing page is DeckStreak's own.

## 8. Tables and the v9 import

None.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S13901-DESCRIPTION-FIRST` | `scripts/landing_rules.py` | the description is the first `content=` tag | `test_landing_rules.TheHeadKeepsItsOrder.test_a_description_that_is_not_first_is_refused` |
| `S13902-TITLE-BOUNDS` | `scripts/landing_rules.py` | a title of 10 to 70 characters; the test names 9, 10, 70 and 71 | `test_landing_rules.TheHeadKeepsItsOrder.test_a_title_outside_its_bounds_is_refused` |
| `S13903-JSON-LD-URL` | `scripts/landing_rules.py` | the JSON-LD `url` equals the canonical | `test_landing_rules.TheHeadKeepsItsOrder.test_a_json_ld_url_that_differs_is_refused` |
| `S13904-ONE-H1` | `scripts/landing_rules.py` | one `<h1`, and the first heading | `test_landing_rules.TheBodyKeepsItsShape.test_a_second_or_late_h1_is_refused` |
| `S13905-PLAIN-HTTP` | `scripts/landing_rules.py` | no `http://` outside the two namespaces | `test_landing_rules.TheTreeKeepsItsLinks.test_a_plain_http_link_is_refused` |
| `S13906-RASTER-BUDGET` | `scripts/landing_rules.py` | a raster of more than 30,000 bytes; the test names 30,000 and 30,001 | `test_landing_rules.TheBodyKeepsItsShape.test_an_image_without_its_attributes_or_over_budget_is_refused` |
| `S13907-RELATIVE-LINK` | `scripts/landing_rules.py` | no query or fragment on a relative link | `test_landing_rules.TheTreeKeepsItsLinks.test_a_relative_link_with_a_query_or_fragment_is_refused` |
| `S13908-DELIVERED-ONLY` | `scripts/landing_rules.py` | a heading's SPEC must be delivered | `test_landing_rules.TheClaimsAreTrue.test_every_feature_heading_names_a_delivered_spec` |
| `S13909-SITE-BUILD` | `scripts/box-packs.sh` | the site is built in the export before the box section | `test_box_packs.TheBoxRunBuildsTheLandingPage.test_the_site_is_built_in_the_judged_tree_before_the_box_section` |
| `S13910-BUILD-FAILURE` | `scripts/box-packs.sh` | a failed build leaves no site to verify | `test_box_packs.TheBoxRunBuildsTheLandingPage.test_a_failed_site_build_is_never_a_pass` |

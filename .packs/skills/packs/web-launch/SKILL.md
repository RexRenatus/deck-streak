---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/web-launch

Launch readiness for a public web app and a Telegram Mini App, judged headless over an owned
site tree. SPEC-V2-1201 seeded the pack and its verbs (it cites SPEC-V2-1196 R33); SPEC-V2-2189
grew it to full practice and fixed #2056. The Standards Librarian owns the bar text. Skills walk
via `phxd` only, never through a second control plane.

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge
(ADR-V2-1990), so this body names none.

`--root` is an owned site tree. No row fetches a URL, opens a network socket or scans a host the
owner did not point at. The closed forbid set is `skills/deny/DENY.md`, and the repository's
`skills/` is authoritative.

```
phxd pack probe --pack web-launch --root PATH --format json --outbox PATH
phxd pack quality --pack web-launch --root PATH --format json --outbox PATH
phxd pack release --pack web-launch --root PATH --format json --outbox PATH
phxd pack check --id ID --root PATH --format json --outbox PATH
```

`--format json` is the only accepted format (else exit 2). Exit 0 means the probes ran and the
seat admitted or proceeded. Exit 4 is a domain refusal (`pack_red`, `stub_probe`, `unknown_pack`,
deny, catalog). A reserved `reserved-1198` probe is `stub_probe`. Quality and Release both refuse
`pack_red` when a `block` row is red.

## How a row reads the tree

Every row reads a parsed tree, never one concatenated text:

- **The walk.** Every file under `--root`, sorted, never inside `node_modules`, `target` or a
  dot-directory (`.well-known` is walked), and never through a symbolic link to a directory. So
  `--root` may be a built site or a repository root.
- **Pages.** Every `*.html` and `*.htm` file, tokenised into elements and attributes. Attributes
  are read quoted either way or unquoted, and entity-decoded. A **document** is a page with a
  DOCTYPE or an `<html>` element; a partial is a page, not a document.
- **References.** A local `href` or `src` loses its query and fragment and is percent-decoded. A
  leading `/` is the root. A clean URL resolves to `<path>`, `<path>.html` or `<path>/index.html`.
  A scheme (`https:`, `mailto:`, `data:`) or a scheme-relative `//host` is not a local file.
- **robots.txt** is read as RFC 9309 records: field names in any case, `#` comments, groups of
  `user-agent` lines, longest match with an allow winning a tie.
- **JSON.** The web app manifest and `budget.json` are parsed, not searched.
- **Counts.** Every row says what it examined, and a row that examined nothing refuses: a count
  of zero is never a pass.

## Severity

`block` rows are firm requirements of a standard or an official document; a red one turns the
pack red and Quality and Release refuse. `advisory` rows are heuristics and best-practice
judgement: a failed one prints `"color": "advisory"` in the probe card and never changes the
pack's color or its exit (SPEC-V2-2195). `pack check --id` on either kind exits 4 when the row
does not hold, and its `detail` names what it examined and found.

## Closed check set

Closed check set is **57** rows. A missing id or an extra id is a refuse. The founding sixteen
lead `checks.json` in their SPEC-V2-1201 order, with their meaning unchanged.

| stage | rows | blocking | advisory |
|---|---|---|---|
| `launch` | 16 | 16 | 0 |
| `a11y` | 14 | 8 | 6 |
| `pwa` | 7 | 1 | 6 |
| `performance` | 4 | 1 | 3 |
| `practices` | 4 | 2 | 2 |
| `privacy` | 3 | 0 | 3 |
| `mini-app` | 9 | 3 | 6 |
| **total** | **57** | **31** | **26** |

Each row's reason is `<id>_red`, its scope is `tree`, and its probe is
`pack check --id <id> --root {root} --format json`.

## Stage `launch`: the founding rows (block)

| id | holds when |
|---|---|
| `privacy-terms` | `privacy.html` and `terms.html` exist at the root, and some other page links each with an `<a href>` |
| `https` | no `http://` in an html, htm, css, js, txt or xml file, except `www.w3.org` and `www.sitemaps.org` identifiers and the value of an `xmlns` or `xmlns:<prefix>` attribute |
| `cookies` | an element carries `data-cookie-notice` |
| `meta-og-favicon` | a `<meta>` names `og:title` with content, another `og:image` with content, and a `<link>` whose `rel` holds `icon` has an `href` |
| `sitemap-robots` | `sitemap.xml` exists, robots.txt has a `sitemap` record, and the `*` group leaves `/` crawlable |
| `alt-text` | at least one `<img>` exists, and every `<img>` has an `alt` attribute (empty is allowed) |
| `image-compress` | every raster (png, jpg, jpeg, gif, webp, avif) is at most 50,000 B, and a raster or `hero.svg` exists |
| `lcp-speed` | an element carries `data-lcp`, `hero.svg` exists, and every raster is at most 30,000 B |
| `contrast` | an element carries `data-contrast="aa"` |
| `mobile` | a page has `<meta name="viewport">` |
| `custom-404` | `404.html` exists at the root |
| `broken-links` | every local `href` and `src` resolves to a file or directory |
| `form-validation` | a `<form>` holds an `input`, `select` or `textarea` carrying `required` |
| `spam-protect` | an element carries `data-spam-protect` |
| `analytics` | an element carries `data-analytics` and an element carries `data-consent` |
| `single-cta` | exactly one element carries `data-cta` |

A marker is an attribute on a page's element. A selector in a stylesheet or a string in a script
does not count, which is #2056's fix for `data-cta` applied to every marker row.

```
phxd pack check --id privacy-terms --root PATH --format json
phxd pack check --id https --root PATH --format json
phxd pack check --id cookies --root PATH --format json
phxd pack check --id meta-og-favicon --root PATH --format json
phxd pack check --id sitemap-robots --root PATH --format json
phxd pack check --id alt-text --root PATH --format json
phxd pack check --id image-compress --root PATH --format json
phxd pack check --id lcp-speed --root PATH --format json
phxd pack check --id contrast --root PATH --format json
phxd pack check --id mobile --root PATH --format json
phxd pack check --id custom-404 --root PATH --format json
phxd pack check --id broken-links --root PATH --format json
phxd pack check --id form-validation --root PATH --format json
phxd pack check --id spam-protect --root PATH --format json
phxd pack check --id analytics --root PATH --format json
phxd pack check --id single-cta --root PATH --format json
```

## Stage `a11y`: document-level WCAG 2.2 quick wins

| id | severity | holds when | source |
|---|---|---|---|
| `html-lang` | block | every document's `<html>` has a `lang` that is a well-formed BCP 47 tag | WCAG 3.1.1 |
| `page-title` | block | every document has a non-empty `<title>` outside any `<svg>` | WCAG 2.4.2 |
| `button-name` | block | every `<button>`, `input type=image` and `input type=button` has an accessible name | WCAG 4.1.2 |
| `link-name` | block | every `<a href>` has an accessible name: text, `aria-label`, `aria-labelledby`, `title`, or an image with `alt` | WCAG 2.4.4, 4.1.2 |
| `form-labels` | block | every input, select and textarea a user fills has a `<label for>`, a wrapping `<label>`, or an ARIA name, title or placeholder | WCAG 1.3.1, 3.3.2 |
| `duplicate-ids` | block | no `id` repeats in a document, outside `<template>` | HTML standard |
| `iframe-title` | block | every `<iframe>` and `<frame>` has a title or ARIA name | WCAG 4.1.2 |
| `zoom-not-disabled` | block | no viewport sets `user-scalable=no` or a `maximum-scale` under 2 | WCAG 1.4.4, ACT rule b4f0c3 |
| `skip-link` | advisory | every document with a `<nav>` has an in-page link before it whose target exists | WCAG 2.4.1 (G1) |
| `landmarks` | advisory | every document has exactly one `main` landmark | WCAG 2.4.1 (ARIA11) |
| `heading-order` | advisory | every document has a heading, and no heading skips a level downward | WCAG 1.3.1 |
| `input-purpose` | advisory | every email, tel or personal-data input names an `autocomplete` purpose other than `off` | WCAG 1.3.5 |
| `positive-tabindex` | advisory | no element has a `tabindex` above 0 | WCAG 2.4.3 |
| `autoplay-audio` | advisory | no `<audio autoplay>`, and no `<video autoplay>` without `muted` | WCAG 1.4.2 |

```
phxd pack check --id html-lang --root PATH --format json
phxd pack check --id page-title --root PATH --format json
phxd pack check --id button-name --root PATH --format json
phxd pack check --id link-name --root PATH --format json
phxd pack check --id form-labels --root PATH --format json
phxd pack check --id duplicate-ids --root PATH --format json
phxd pack check --id iframe-title --root PATH --format json
phxd pack check --id zoom-not-disabled --root PATH --format json
phxd pack check --id skip-link --root PATH --format json
phxd pack check --id landmarks --root PATH --format json
phxd pack check --id heading-order --root PATH --format json
phxd pack check --id input-purpose --root PATH --format json
phxd pack check --id positive-tabindex --root PATH --format json
phxd pack check --id autoplay-audio --root PATH --format json
```

## Stage `pwa`: the Web App Manifest and an offline fallback

| id | severity | holds when | source |
|---|---|---|---|
| `manifest-valid` | block | every manifest a document links exists, is a JSON object, and names icons, screenshots and shortcut icons that exist; a site linking none holds | W3C appmanifest |
| `manifest-linked` | advisory | the home page links a manifest | web.dev PWA checklist |
| `manifest-installable` | advisory | the linked manifest has `name` or `short_name`, a 192x192 and a 512x512 icon (or `any`), `start_url`, an installable `display`, and no `prefer_related_applications: true` | Chrome install criteria |
| `manifest-recommended` | advisory | it has `id`, `scope`, `theme_color`, `background_color`, `description`, screenshots and a maskable icon | W3C appmanifest, web.dev |
| `theme-color` | advisory | the home page declares `<meta name="theme-color">` | MDN |
| `apple-touch-icon` | advisory | the home page links an `apple-touch-icon` that exists | web.dev |
| `offline-fallback` | advisory | a script registers a service worker that exists and names an `.html` page that exists | web.dev PWA checklist |

The home page is `index.html` at the root, or every document when the root has none. The two
manifest-content rows grade nothing when no manifest is linked or it does not parse: those are
`manifest-linked`'s and `manifest-valid`'s findings.

```
phxd pack check --id manifest-valid --root PATH --format json
phxd pack check --id manifest-linked --root PATH --format json
phxd pack check --id manifest-installable --root PATH --format json
phxd pack check --id manifest-recommended --root PATH --format json
phxd pack check --id theme-color --root PATH --format json
phxd pack check --id apple-touch-icon --root PATH --format json
phxd pack check --id offline-fallback --root PATH --format json
```

## Stage `performance`: budgets, weight and versioned URLs

| id | severity | holds when | source |
|---|---|---|---|
| `budget-met` | block | every document a `budget.json` budget's `path` matches stays within its `resourceSizes` (KiB) and `resourceCounts`; a site with no `budget.json` holds | the budget.json format Lighthouse CI asserts |
| `budget-declared` | advisory | `budget.json` exists at the root | web.dev performance budgets |
| `page-weight` | advisory | every document loads at most 1,600 KiB from the tree | Lighthouse `total-byte-weight` |
| `asset-fingerprint` | advisory | every local script and stylesheet a document loads carries a content version in its name or a `v=` query | web.dev HTTP cache |

A document's weight is measured on disk, uncompressed, so it over-states what travels. It counts
the document, its scripts, stylesheets, images and media, and the fonts its stylesheets name. A
third-party URL is counted as a request, never sized. `timings` in a budget need a browser, and
are not graded.

```
phxd pack check --id budget-met --root PATH --format json
phxd pack check --id budget-declared --root PATH --format json
phxd pack check --id page-weight --root PATH --format json
phxd pack check --id asset-fingerprint --root PATH --format json
```

## Stage `practices`: the Lighthouse best practices a tree decides

| id | severity | holds when | source |
|---|---|---|---|
| `doctype` | block | every document opens with `<!DOCTYPE html>` (only whitespace and comments before it, no public identifier) | HTML standard, Lighthouse `doctype` |
| `paste-allowed` | block | no element's `onpaste` returns false or calls `preventDefault` | Lighthouse `paste-preventing-inputs`, WCAG 3.3.8 |
| `charset` | advisory | every document declares utf-8 in a `<meta>` that ends within its first 1,024 bytes | HTML standard, Lighthouse `charset` |
| `unload-handler` | advisory | no script registers an `unload` listener or assigns `onunload`, and no element carries `onunload` | Chrome's unload deprecation |

`charset` is advisory because a server may declare the encoding in `Content-Type`, as Caddy does.

```
phxd pack check --id doctype --root PATH --format json
phxd pack check --id paste-allowed --root PATH --format json
phxd pack check --id charset --root PATH --format json
phxd pack check --id unload-handler --root PATH --format json
```

## Stage `privacy`: consent and the policy's reach

| id | severity | holds when | source |
|---|---|---|---|
| `consent-before-trackers` | advisory | no document loads a known cookie-setting tracker's script as a runnable script; one held as `type="text/plain"` until consent passes | ePrivacy 5(3), EDPB 05/2020 |
| `privacy-contact` | advisory | `privacy.html` links a `mailto:`, a `tel:` or a contact page | GDPR Art. 13(1)(a) |
| `privacy-reachable` | advisory | every document other than `privacy.html` links it | GDPR Art. 12, Telegram Bot Platform Developer Terms |

```
phxd pack check --id consent-before-trackers --root PATH --format json
phxd pack check --id privacy-contact --root PATH --format json
phxd pack check --id privacy-reachable --root PATH --format json
```

## Stage `mini-app`: a Telegram Mini App wired to Telegram

A **Mini App document** loads `telegram-web-app.js`, or runs a script, inline or local, that names
`Telegram.WebApp`. A site with none is no Mini App, and each row below except the two deep-link
rows holds and says `not a Mini App` with what it examined. The deep-link rows grade every `t.me`
link, wherever it sits.

| id | severity | holds when | source |
|---|---|---|---|
| `tg-sdk-script` | block | every Mini App document loads `https://telegram.org/js/telegram-web-app.js` in its head, before any other script, and no script naming `Telegram.WebApp` is loaded without it | core.telegram.org/bots/webapps |
| `tg-deep-links` | block | every `start` and `startgroup` value in a `t.me` link keeps to `A-Z a-z 0-9 _ -` and 1 to 64 characters | core.telegram.org/bots/features |
| `tg-cloud-storage-keys` | block | every literal `CloudStorage` key keeps to `A-Z a-z 0-9 _ -` and 1 to 128 characters | core.telegram.org/bots/webapps |
| `tg-ready` | advisory | a Mini App script calls `ready()` on `Telegram.WebApp` | core.telegram.org/bots/webapps |
| `tg-expand` | advisory | a Mini App script calls `expand()` or `requestFullscreen()` | core.telegram.org/bots/webapps |
| `tg-startapp` | advisory | every `startapp` value keeps to `A-Z a-z 0-9 _ -` and at most 512 characters | Telegram Mini Apps platform docs |
| `tg-init-data` | advisory | no Mini App script sends `initDataUnsafe`, and one that calls a server sends the raw `initData` | core.telegram.org/bots/webapps |
| `tg-links` | advisory | no Mini App document links out with a plain `<a href>`; use `openLink` or `openTelegramLink` | core.telegram.org/bots/webapps |
| `tg-version-gate` | advisory | a Mini App using a member newer than Bot API 6.0 calls `isVersionAtLeast` | core.telegram.org/bots/webapps |

A value built at run time (it holds `$`, `{` or `+`) is not judged.

```
phxd pack check --id tg-sdk-script --root PATH --format json
phxd pack check --id tg-deep-links --root PATH --format json
phxd pack check --id tg-cloud-storage-keys --root PATH --format json
phxd pack check --id tg-ready --root PATH --format json
phxd pack check --id tg-expand --root PATH --format json
phxd pack check --id tg-startapp --root PATH --format json
phxd pack check --id tg-init-data --root PATH --format json
phxd pack check --id tg-links --root PATH --format json
phxd pack check --id tg-version-gate --root PATH --format json
```

## Pointing it at another project

`--root` is the directory the server serves. For a Rust workspace with a web front end, run it
over the built output (`web/dist`), or over a static source directory with its assets beside it.
For DeckStreak:

- keep the public pages and the Mini App in one served tree, with the Mini App under `app/`, so
  the founding marketing rows judge the landing page and the `mini-app` rows judge `app/`;
- put at the root `index.html`, `privacy.html`, `terms.html`, `404.html`, `robots.txt`,
  `sitemap.xml`, `manifest.webmanifest` and `budget.json`, with `Disallow: /app/` for `*` in
  robots.txt if the Mini App is not a search landing page;
- run `phxd pack probe --pack web-launch --root web/dist --format json`, and read a red row with
  `phxd pack check --id <id> --root web/dist --format json`;
- keep third-party licence banners out of the served tree: `https` reads every js and txt file, so
  a bundled banner naming an `http://` licence URL refuses it. With esbuild, `legalComments:
  "none"` drops them, and `linked` or `external` writes a `.LEGAL.txt` that must stay out of the
  served directory.

## What this pack does not do

- It does not own search: meta descriptions, canonicals, structured data and Open Graph
  completeness are `packs/seo-pipeline`'s.
- It does not own security: security headers, CSP, `security.txt`, exposed keys and server-side
  `initData` validation are `packs/web-security`'s and `packs/cyber-pipeline`'s.
- It does not own focus visibility, reduced motion, UI states, dark mode or Telegram-native
  behaviour (theme changes, BackButton and MainButton, haptics, safe areas and the viewport), which
  are `packs/vibecode-polish`'s; target size, which is `packs/ux-laws`'s; or contrast ratios and
  theme tokens, which are `packs/ui-styles`'s.
- It does not fetch, render or run a page: vitals, console errors, live headers, redirects and
  BotFather's configuration stay out.
- It does not flip pause or the process-stop lever, and it does not open the operator ledger.

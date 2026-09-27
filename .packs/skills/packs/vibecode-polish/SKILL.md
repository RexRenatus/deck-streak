---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/vibecode-polish

Headless polish walk (SPEC-V2-1201; cites SPEC-V2-1196 R33). Standards
Librarian owns the bar text. Skills walk via `phxd` only — not a second
control plane.

Which seats consume this pack is its catalog row's `consumes`, the one
record of that edge (ADR-V2-1990), so this body names none.

`--root` is an owned site tree. Probes do not fetch a URL, do not open
a network socket, and do not scan a host the owner did not point at.
Closed forbid set: `skills/deny/DENY.md`. Repo `skills/` is
authoritative.

```
phxd pack probe --pack vibecode-polish --root PATH --format json --outbox PATH
phxd pack quality --pack vibecode-polish --root PATH --format json --outbox PATH
phxd pack check --id ID --root PATH --format json --outbox PATH
```

`phxd pack release --pack vibecode-polish` is a refuse
(`not_release_preflight`). `--format json` is the only accepted format
(else exit 2). Exit 0 = probes ran and Quality admitted. Exit 4 =
domain refuse (`pack_red`, `stub_probe`, `unknown_pack`,
`not_release_preflight`, deny, catalog). A reserved `reserved-1198`
probe is `stub_probe`.

Closed check set is **20** rows: 11 `block` and 9 `advisory` (SPEC-V2-2195's
severity gate; an advisory refuse never reddens the card or changes the
exit). One stage, `polish`, holds all 20 (11 `block`, 9 `advisory`). A
missing id or an extra id is a refuse. Complement: launch crumbs
(privacy/terms, cookies, sitemap, analytics, CTA) stay on
`packs/web-launch/`.

**Ownership boundary (owner directive, 2026-09-27; full matrix in
SPEC-V2-2190 `## 4.`).** This pack owns keyboard focus-visible, dev-string
leftovers, `prefers-reduced-motion`, the loading/empty/offline UI states,
dark-mode support, the favicon (founding `favicon-titles-meta`), and the
Telegram-native behaviours (theme change and its variable fallbacks,
`BackButton`/`MainButton`, haptics — including argument validity, not just
presence — safe areas, and the `viewportStableHeight`-vs-`100vh` viewport
trap). It does **not** own, and cites instead: document-level a11y — html
lang, alt text, form labels, skip link, landmarks, `tg-links`,
`offline-fallback` (`packs/web-launch/`); contrast, DTCG tokens, the
type/spacing scale, component states, icons (`packs/ui-styles/`); the WCAG
2.5.8 target size, div-as-button, the UX-law heuristics, dark patterns,
gamification ethics, notification consent (`packs/ux-laws/`). This pack's
eleven new ids share one flat `phxd pack check --id` namespace with
`packs/web-launch/`'s ids, and web-launch's dispatch sees its own ids
first, so none of this pack's ids may collide with that list.

**Three host surfaces.** A Mini App reaches its host through
`telegram-web-app.js` (`Telegram.WebApp`, also written `Telegram?.WebApp`),
the raw WebView bridge (`Telegram.WebView`, `TelegramWebviewProxy`, the
`web_app_*` methods and events such as `theme_changed`), or the tma.js SDK
(`@tma.js/*`, which replaced `@telegram-apps/*` on 2025-10-21). Every
Telegram check reads all three. The SDK's lower-camel names (`backButton`,
`hapticFeedback`) count only beside an SDK import, because a plain page
uses them as variable names (SPEC-V2-2190 R10).

## Running it on another repository

Point `--root` at the tree you SERVE, not at the repository root: the walk
reads every `html`/`htm`/`css`/`js`/`txt`/`xml` file below `--root`, and a
repository root drags in `target/`, `node_modules/`, test fixtures and
vendored scripts. For a Rust workspace with a web front end, that is the
front end's static or source directory (for example `web/`). Point it at
source rather than a minified bundle for the Telegram checks: a bundler
rewrites SDK imports and may mangle `backButton`. Run the probe with this
repository's skills tree:

```
phxd pack probe --pack vibecode-polish --root <repo>/web --skills-root <phoenix-v2>/skills --format json
```

Exit 4 with a `red` card means a `block` row refused; an `advisory` row
reports `advisory` and never changes the card or the exit.

## Check table

| id | severity | inspects in `--root` | green | refuse |
|---|---|---|---|---|
| `no-horizontal-scroll` | block | page + css text | no `overflow-x:scroll` / `overflow-x: scroll` | `no-horizontal-scroll_red` |
| `broken-links-buttons` | block | relative `href=` / `src=` in html/htm | every local target exists on disk | `broken-links-buttons_red` |
| `mobile-menu-overflow` | block | page text | `data-mobile-menu` present and no horizontal overflow | `mobile-menu-overflow_red` |
| `favicon-titles-meta` | block | page text | `<title>`, `rel="icon"`, `name="description"` | `favicon-titles-meta_red` |
| `footer-404-copyright` | block | `404.html` + page text | 404 file, `<footer`, `&copy;`, a 4-digit year within 40 bytes of it, either side | `footer-404-copyright_red` |
| `compress-images` | block | raster bytes | each raster ≤ 50_000 B, or `hero.svg` stands in | `compress-images_red` |
| `success-error-messages` | block | page text | `data-success` **and** `data-error` | `success-error-messages_red` |
| `no-placeholder-nav` | block | page text | no `href="#"` and no `lorem` | `no-placeholder-nav_red` |
| `clickable-logo-phone-email` | block | page text | `data-logo`, `href="tel:`, `href="mailto:` | `clickable-logo-phone-email_red` |
| `focus-visible-outline` | block | css/page text | no `outline:` of `none` or zero width, or one paired with `:focus-visible` | `focus-visible-outline_red` |
| `no-dev-strings` | block | page text | no work-left marker (T-O-D-O, F-I-X-M-E, H-A-C-K, X-X-X) as a word, no `console.log(`/`debugger;` | `no-dev-strings_red` |
| `dark-mode-support` | advisory | css/page/js text | `color-scheme` (media feature, property or meta), `colorScheme`, or `var(--tg-theme-` | `dark-mode-support_red` |
| `reduced-motion-support` | advisory | css/js text | no motion declared, or `prefers-reduced-motion` present | `reduced-motion-support_red` |
| `ui-states-loading-empty-offline` | advisory | page text | `data-loading`, `data-empty`, `data-offline` all present | `ui-states-loading-empty-offline_red` |
| `telegram-theme-change-handling` | advisory | js text | a host surface with `themeChanged`, `theme_changed` or `themeParams` | `telegram-theme-change-handling_red` |
| `telegram-theme-var-fallback` | advisory | css text | every `var(--tg-theme-*, ...)` carries a fallback | `telegram-theme-var-fallback_red` |
| `telegram-back-main-button` | advisory | js text | `BackButton`/`MainButton`, `web_app_setup_back_button`/`web_app_setup_main_button`, or the SDK's `backButton`/`mainButton` | `telegram-back-main-button_red` |
| `telegram-haptic-feedback` | advisory | js text | a haptic API on a host surface, and every literal `impactOccurred`/`notificationOccurred` argument is a member of its enum | `telegram-haptic-feedback_red` |
| `telegram-safe-area` | advisory | css/js text | `safeAreaInset`/`SafeAreaInset`, `safe-area-inset`, or `safe_area_changed` | `telegram-safe-area_red` |
| `telegram-viewport-units` | advisory | css/js text | no `100vh`, or `viewportStableHeight`/`--tg-viewport-stable-height` also present | `telegram-viewport-units_red` |

## Per-check tree contract

### no-horizontal-scroll

A polished page does not force a sideways scroll.

- Green: collected text contains neither `overflow-x:scroll` nor
  `overflow-x: scroll`.
- Red / `no-horizontal-scroll_red`: either token is present.

```
phxd pack check --id no-horizontal-scroll --root PATH --format json --outbox PATH
```

### broken-links-buttons

Relative `href` / `src` targets (links **and** button-like anchors)
must exist on disk. `http(s):`, `mailto:`, `tel:`, and `#` fragments
are skipped (placeholder `#` is owned by `no-placeholder-nav`).

- Green: every local target exists.
- Red / `broken-links-buttons_red`: a relative target is missing.

```
phxd pack check --id broken-links-buttons --root PATH --format json --outbox PATH
```

### mobile-menu-overflow

A mobile menu is named, and it does not overflow the viewport
sideways.

- Green: `data-mobile-menu` is present, and the overflow-x scroll
  tokens are absent.
- Red / `mobile-menu-overflow_red`: missing menu marker, or
  horizontal overflow is declared.

```
phxd pack check --id mobile-menu-overflow --root PATH --format json --outbox PATH
```

### favicon-titles-meta

Polish crumbs: a real title, a favicon, a description. Not the
web-launch OG pair and not the full SEO pipeline.

- Green: `<title>`, `rel="icon"`, and `name="description"` all appear.
- Red / `favicon-titles-meta_red`: any of the three is missing.

```
phxd pack check --id favicon-titles-meta --root PATH --format json --outbox PATH
```

### footer-404-copyright

A polished site ships a footer with a copyright year and a custom
404. Fixed as a founding-row defect (SPEC-V2-2190 R4): the check used to
match the literal substring `2026`, so it would have misjudged every
site's footer once the calendar turned over. It now looks for any run of
four ASCII digits within 40 bytes of `&copy;`, on either side of it, so
it stays correct next year and every year after, and `2026 &copy; Acme`
reads as green as `&copy; 2026 Acme`.

- Green: `404.html` exists, collected text contains `<footer`,
  `&copy;`, and a 4-digit year within 40 bytes of it.
- Red / `footer-404-copyright_red`: missing 404, missing footer, or no
  year-shaped digit run near the copyright glyph.

```
phxd pack check --id footer-404-copyright --root PATH --format json --outbox PATH
```

### compress-images

Same raster budget as web-launch `image-compress` (50_000 bytes). An
SVG hero is an accepted stand-in when no rasters exist.

- Green: every raster under `--root` is ≤ 50_000 B, and either a
  raster exists or `hero.svg` exists.
- Red / `compress-images_red`: a raster is over budget, or the tree
  has neither rasters nor `hero.svg`.

```
phxd pack check --id compress-images --root PATH --format json --outbox PATH
```

### success-error-messages

Forms tell the user what happened.

- Green: `data-success` and `data-error` both appear.
- Red / `success-error-messages_red`: either marker is missing.

```
phxd pack check --id success-error-messages --root PATH --format json --outbox PATH
```

### no-placeholder-nav

No unused `#` nav and no lorem copy left in the tree.

- Green: collected text contains neither `href="#"` nor `lorem`
  (case-insensitive).
- Red / `no-placeholder-nav_red`: a hash href or lorem remains.

```
phxd pack check --id no-placeholder-nav --root PATH --format json --outbox PATH
```

### clickable-logo-phone-email

Logo, phone, and email are real links.

- Green: `data-logo` is present, plus `href="tel:` and
  `href="mailto:`.
- Red / `clickable-logo-phone-email_red`: any of the three is
  missing.

```
phxd pack check --id clickable-logo-phone-email --root PATH --format json --outbox PATH
```

### focus-visible-outline

WCAG 2.4.7 Focus Visible / 2.4.11 Focus Not Obscured. A keyboard user must
see which element holds focus.

- Green: no `outline:` declaration whose value is `none` or a zero width
  (`0`, `0px`, `0.0em`), or one is present but paired with a
  `:focus-visible` rule. The value is read: `outline: 0.125rem solid`
  draws a ring and is not a suppression.
- Red / `focus-visible-outline_red`: the outline is suppressed with no
  `:focus-visible` replacement.

```
phxd pack check --id focus-visible-outline --root PATH --format json --outbox PATH
```

### no-dev-strings

No developer-only leftovers in the shipped tree — the class this repo's own
CLAUDE.md forbids in committed code, plus the two most common debug
leftovers. Owns a different set than `no-placeholder-nav`, which owns
`lorem` and `href="#"`.

- Green: none of the four work-left markers (T-O-D-O, F-I-X-M-E, H-A-C-K and X-X-X,
  spelled letter by letter so this file carries none) appears as a word (no
  letter, digit or `_` touching it, the reading the repository's own
  marker guard gives), and neither `console.log(` nor `debugger;` appears.
  `HACKATHON`, an `XXXL` size and a `mktemp` template are not markers.
- Red / `no-dev-strings_red`: any marker or leftover is present.

```
phxd pack check --id no-dev-strings --root PATH --format json --outbox PATH
```

### dark-mode-support

Advisory: the page declares a dark appearance. Inside a Mini App the host
decides night mode, so following the Telegram theme counts. Never refuses.

- Green: `color-scheme` appears (the `prefers-color-scheme` media feature,
  the `color-scheme` property or meta, or `--tg-color-scheme`), or
  `colorScheme`, or a `var(--tg-theme-...)` read.
- Advisory / `dark-mode-support_red`: absent.

```
phxd pack check --id dark-mode-support --root PATH --format json --outbox PATH
```

### reduced-motion-support

Advisory: a page that moves respects the OS-level motion preference
(WCAG 2.3.3 is AAA, hence advisory here). A page that declares no motion
has nothing to reduce. Never refuses.

- Green: no motion is declared (`transition`, `animation`, `@keyframes`,
  `scroll-behavior`, `.animate(`, `requestAnimationFrame`), or
  `prefers-reduced-motion` appears.
- Advisory / `reduced-motion-support_red`: motion without the media query.

```
phxd pack check --id reduced-motion-support --root PATH --format json --outbox PATH
```

### ui-states-loading-empty-offline

Advisory: loading, empty and offline states are marked as explicitly as
`success-error-messages` marks success and error. Never refuses.

- Green: `data-loading`, `data-empty`, and `data-offline` all appear.
- Advisory / `ui-states-loading-empty-offline_red`: any marker is missing.

```
phxd pack check --id ui-states-loading-empty-offline --root PATH --format json --outbox PATH
```

### telegram-theme-change-handling

Advisory: inside a Telegram Mini App, the page listens for the host's theme
instead of shipping one fixed palette. Never refuses.

- Green: a host surface appears (`Telegram.WebApp`, `Telegram?.WebApp`,
  `Telegram.WebView`, `TelegramWebviewProxy`, or an SDK import) together
  with `themeChanged` (the documented event), `theme_changed` (the raw
  bridge's) or `themeParams`.
- Advisory / `telegram-theme-change-handling_red`: no host surface, or a
  host with none of the three tokens.

```
phxd pack check --id telegram-theme-change-handling --root PATH --format json --outbox PATH
```

### telegram-theme-var-fallback

Advisory: a CSS custom property that reads a Telegram theme variable must
carry a fallback, because outside a Mini App no `--tg-theme-*` property is
ever set and an un-fallen-back `var()` resolves to nothing. No usage at all
is not this check's gap (that is `telegram-theme-change-handling`'s), so it
reads green. Never refuses.

- Green: every `var(--tg-theme-...)` occurrence has a `,` before its
  matching close paren, or none occur at all.
- Advisory / `telegram-theme-var-fallback_red`: at least one occurrence has
  no fallback.

```
phxd pack check --id telegram-theme-var-fallback --root PATH --format json --outbox PATH
```

### telegram-back-main-button

Advisory: the page drives the Mini App's own chrome instead of drawing a
second back or submit control inside the viewport. Never refuses.

- Green: `BackButton` or `MainButton`, the raw bridge's
  `web_app_setup_back_button` or `web_app_setup_main_button`, or the SDK's
  `backButton` or `mainButton` beside an SDK import.
- Advisory / `telegram-back-main-button_red`: none appears.

```
phxd pack check --id telegram-back-main-button --root PATH --format json --outbox PATH
```

### telegram-haptic-feedback

Advisory: the page fires haptic feedback on its interactions — the tactile
cue that reads as native. Presence alone is not enough: an invalid
`style`/`type` argument silently no-ops on-device rather than erroring, so
once the API is present this also validates EVERY literal
`impactOccurred(...)` / `notificationOccurred(...)` argument against the
SDK's closed enum, by the whole word (the methods chain and repeat). A
non-literal argument (a variable, a ternary, an interpolated template)
cannot be read by a text heuristic and passes rather than being guessed at.
Never refuses.

- Green: `HapticFeedback`, the raw bridge's
  `web_app_trigger_haptic_feedback`, or the SDK's `hapticFeedback` beside
  an SDK import appears, and every literal argument is exactly one of
  `impactOccurred`'s `light`/`medium`/`heavy`/`rigid`/`soft` or
  `notificationOccurred`'s `error`/`success`/`warning`.
- Advisory / `telegram-haptic-feedback_red`: no haptic API, or a literal
  argument names something outside that enum (`'lightest'` included).

```
phxd pack check --id telegram-haptic-feedback --root PATH --format json --outbox PATH
```

### telegram-safe-area

Advisory: the page accounts for the device safe area (notch, home
indicator, the Mini App header) instead of drawing under it. Never refuses.

- Green: `safeAreaInset` or `SafeAreaInset` (the Bot API 8.0 fields,
  `contentSafeAreaInset` included), `safe-area-inset` (the
  `--tg-safe-area-inset-*` variables and CSS `env(safe-area-inset-*)`), or
  the raw bridge's `safe_area_changed` appears. `viewportStableHeight` is a
  height, not an inset, and does not count.
- Advisory / `telegram-safe-area_red`: none appear.

```
phxd pack check --id telegram-safe-area --root PATH --format json --outbox PATH
```

### telegram-viewport-units

Advisory: a Telegram Mini App's real viewport is `viewportStableHeight`, not
the browser's `100vh` — the WebView chrome and an open keyboard both shrink
the frame without firing a resize a bare `100vh` will ever see. Using
`100vh` as a plain CSS fallback ahead of a
`var(--tg-viewport-stable-height, ...)` override is the documented,
still-green pattern. Never refuses.

- Green: no `100vh` token, or `viewportStableHeight` or its CSS variable
  `--tg-viewport-stable-height` also appears.
- Advisory / `telegram-viewport-units_red`: `100vh` appears with neither
  anywhere in the corpus.

```
phxd pack check --id telegram-viewport-units --root PATH --format json --outbox PATH
```

## What this pack does not do

- Does not seed the other five 1196 R33 packs.
- Does not become a Release preflight (Quality still owns the red).
- Does not crawl the public internet or ship dual-use recipes.
- Does not flip pause or the process-stop lever.
- Does not open the operator ledger.
- Does not grade document-level a11y (html lang, alt text, form labels,
  skip link, landmarks) — that is `packs/web-launch/`.
- Does not grade contrast, DTCG tokens, the type/spacing scale, component
  states, or icons — that is `packs/ui-styles/`.
- Does not grade the WCAG 2.5.8 target size, div-as-button, the UX-law
  heuristics, dark patterns, gamification ethics, or notification consent —
  that is `packs/ux-laws/`.
- Does not open a browser, a device, or a network socket: focus order,
  animation smoothness, real haptic pulses and on-device safe-area insets
  are read as source markers, not rendered or felt (SPEC-V2-2190 `## 6.`).

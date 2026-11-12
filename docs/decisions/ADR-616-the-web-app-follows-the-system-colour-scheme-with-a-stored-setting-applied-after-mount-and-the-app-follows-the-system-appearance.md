---
status: "accepted"
---

# ADR-616: the web app follows the system colour scheme with a stored setting applied after mount, and the iPhone and iPad app follows the system appearance

Decides SPEC-593 with ADR-615, for #1045: one colour-mode rule per client, each held by a test with a planted wrong mode.

## Context and Problem Statement

Measured at dev `6f7d05f9`: `layout.css` sets `color-scheme: var(--tg-color-scheme, light)` (line 28) and declares a `dark` variant on `.dark` (line 13) that nothing sets, so outside Telegram the page is always light. `app.html` holds no script, and `web/app/src/lib/csp.test.ts` pins that (the page shell carries no script, inline or by URL, SPEC-400 R7) together with the script sources of the content security policy that `sveltekit.config.js` sets in `hash` mode. No Swift file sets a colour scheme, and the three Tide colour sets each carry a universal entry and a `luminosity` `dark` entry.

## Decision

D1, the web app. The mode is one of three choices, `system`, `light` and `dark`, kept on the device under one local-storage key, `deckstreak.colour-mode`, and changed in the settings screen. No script is added to the page. Until the app mounts, and while the choice is `system` or nothing is stored, the stylesheet alone sets the mode: a `prefers-color-scheme` rule in `layout.css` gives every colour role its dark Tide fallback unless the browser reports `prefers-color-scheme: light` or the root element carries the `light` class, so the dark-first palette is what a browser with no preference shows, and the page follows a system change with no listener. After the app mounts, the root layout applies a stored `light` or `dark` as that class on the root element and clears it for `system`, so a stored choice opposite the system shows the system mode until then. The stylesheet takes `color-scheme` from `--tg-color-scheme` with the mode as its fallback, so the stored choice never outranks the chat scheme inside Telegram. `app.html` and the content security policy stay unchanged. Inside Telegram the chat theme still paints every role and `--tg-color-scheme` still sets the scheme.

D2, the iPhone and iPad app. The app follows the system appearance and offers no setting of its own: every colour comes from a colour set whose dark entry the system picks, and no view or property list sets a scheme (`preferredColorScheme`, `overrideUserInterfaceStyle` or `UIUserInterfaceStyle`). The system appearance setting is the one switch.

## What it was chosen AGAINST

- Web, chosen: the system scheme painted by a stylesheet rule, with a stored three-way setting applied after the app mounts, because a learner can override a shared or light-only device, the default needs no action, and the page shell stays free of script
- Web, the system scheme alone: rejected because it leaves no way to read in dark on a light system, and the setting costs one key
- Web, a setting alone with a fixed default: rejected because it ignores the scheme the learner already chose for the device
- Web, an inline script in the head of `app.html` that applies the stored choice before the first frame: rejected because the page shell carries no script (SPEC-400 R7), and admitting one would narrow the page-policy tests
- Web, a cookie read on the server: rejected because the app is served as static files, and a cookie adds a request header for one bit
- Web, the mode applied only by the app after it mounts, with no stylesheet rule: rejected because every first frame would paint light whatever the system reports
- Native, chosen: the system appearance, because the colour sets already adapt to it and the platform holds the setting
- Native, an in-app setting through `preferredColorScheme`: rejected because it duplicates the system switch and would need a stored value and a second test per screen

## Consequences

- Good, because each client has one rule a test holds, and the web rule adds no script, so the page shell and its policy stay as SPEC-400 pins them.
- Bad, because a stored choice opposite the system shows the system mode until the app mounts, and the `layout.css` system rule repeats the dark fallbacks the compile emits, which a test holds equal.
- Neutral, because the two clients differ: the web app has a setting, the native app defers to the system.

## What would make this wrong

- A rendered audit, or a learner report, that measures the frames a stored choice opposite the system shows before the app mounts as a flash that matters.
- A learner need for a native setting, raised as an issue.

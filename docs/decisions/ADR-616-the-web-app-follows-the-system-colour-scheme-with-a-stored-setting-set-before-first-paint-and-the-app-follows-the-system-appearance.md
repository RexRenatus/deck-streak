---
status: "accepted"
---

# ADR-616: the web app follows the system colour scheme with a stored setting set before first paint, and the iPhone and iPad app follows the system appearance

Decides SPEC-593 with ADR-615, for #1045: one colour-mode rule per client, each held by a test with a planted wrong mode.

## Context and Problem Statement

Measured at dev `6f7d05f9`: `layout.css` sets `color-scheme: var(--tg-color-scheme, light)` (line 28) and declares a `dark` variant on `.dark` (line 13) that nothing sets, so outside Telegram the page is always light. `app.html` holds no script. `sveltekit.config.js` sets a content security policy in `hash` mode whose `script-src` is `self`, the Telegram script host and `wasm-unsafe-eval`: SvelteKit hashes only the scripts it writes itself, so a hand-written inline script runs only if its own hash is listed. No Swift file sets a colour scheme, and the three Tide colour sets each carry a universal entry and a `luminosity` `dark` entry.

## Decision

D1, the web app. The mode is one of three choices, `system`, `light` and `dark`, kept on the device under one local-storage key, `deckstreak.colour-mode`, and changed in the settings screen. With `system`, or with nothing stored, the page is light only when the browser reports `prefers-color-scheme: light`, and dark otherwise, so the dark-first palette is what a browser with no preference shows. A small inline script in the head of `app.html` reads the key and the media query before the body paints, sets the `dark` class on the root element or clears it, so the first frame is already in its mode, and the stylesheet takes `color-scheme` from `--tg-color-scheme` with the class mode as its fallback, so the stored choice never outranks the chat scheme inside Telegram. Its sha256 is listed in `script-src`, and a test holds the listed hash equal to the script. A media-query listener repaints when the system changes while the choice is `system`. Inside Telegram the chat theme still paints every role and `--tg-color-scheme` still sets the scheme.

D2, the iPhone and iPad app. The app follows the system appearance and offers no setting of its own: every colour comes from a colour set whose dark entry the system picks, and no view or property list sets a scheme (`preferredColorScheme`, `overrideUserInterfaceStyle` or `UIUserInterfaceStyle`). The system appearance setting is the one switch.

## What it was chosen AGAINST

- Web, chosen: the system scheme with a stored three-way setting, because a learner can override a shared or light-only device and the default needs no action
- Web, the system scheme alone: rejected because it leaves no way to read in dark on a light system, and the setting costs one key
- Web, a setting alone with a fixed default: rejected because it ignores the scheme the learner already chose for the device
- Web, the mode applied by the app after it loads: rejected because the first frame paints in the wrong mode, a visible flash
- Web, a cookie read on the server: rejected because the app is served as static files, and a cookie adds a request header for one bit
- Web, a nonce in place of a hash: rejected because prerendered pages cannot carry a fresh nonce, and the policy is already in hash mode
- Native, chosen: the system appearance, because the colour sets already adapt to it and the platform holds the setting
- Native, an in-app setting through `preferredColorScheme`: rejected because it duplicates the system switch and would need a stored value and a second test per screen

## Consequences

- Good, because each client has one rule a test holds, and the web rule paints no wrong first frame.
- Bad, because the web inline script and its listed hash must change together, which the hash test enforces.
- Neutral, because the two clients differ: the web app has a setting, the native app defers to the system.

## What would make this wrong

- A browser that paints before running a head script, measured as a flash in a rendered audit.
- A learner need for a native setting, raised as an issue.

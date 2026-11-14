---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# One universal iPhone and iPad app, driven by touch, keys or an 8BitDo remote

## Context and Problem Statement

The owner asked for the native client to run on both iPhone and iPad (the owner's answer GAM-01,
which replaces the earlier convention of an iPhone-only app) and to be driven by an 8BitDo
Bluetooth remote, as AnkiMobile users do (HW-01). Such a remote works in two modes: as a gamepad,
which iOS reads through the GameController framework, and as a hardware keyboard, which it reads
as key commands. The web client meets the same remote through the Gamepad API and keyboard
events. A remote session leaves the screen untouched for long stretches, so the screen would
dim and lock. How is the client shaped for two device sizes and three kinds of input?

## Decision Drivers

- One build and one TestFlight lane for both devices.
- The review loop works from the remote alone: show the answer, grade, undo, bury, flag and
  replay audio.
- Both of the remote's modes work, since the owner may use either.
- The screen stays on while the remote drives a session, and only then.
- Bottom-anchored answer buttons for touch (UX-04).

## Considered Options (the alternatives it was chosen against)

- One universal SwiftUI app with adaptive layouts, the remote read in both modes through GameController and UIKeyCommand, its buttons mappable, and the idle timer off while a remote is paired — chosen because it is what the owner asked for (GAM-01, HW-01) in one build and one lane.
- An iPhone-only app — rejected because the owner asked for iPad as well (GAM-01).
- Two apps, one per device — rejected because they double the builds and the TestFlight lanes for the same code.
- Keyboard mode only — rejected because it drops the remote's gamepad mode, and the owner asked for both.
- The screen kept awake for every review session — rejected because it holds the screen on when no remote is in use; keeping it awake while a remote is paired is enough.

## Decision Outcome

Proposed option: one universal app, three kinds of input.

- **Layouts.** One SwiftUI target for iPhone and iPad. The iPad uses a split view
  (`NavigationSplitView`): decks beside the review; the iPhone uses a stack. The answer buttons
  sit at the bottom of the review screen on both.
- **The remote, gamepad mode.** The app reads an extended gamepad through the GameController
  framework when one connects.
- **The remote, keyboard mode.** The app declares key commands (`UIKeyCommand`, and SwiftUI's
  `keyboardShortcut`) for the same actions.
- **The mapping.** A settings screen maps the remote's buttons, in either mode, to show answer,
  Again, Hard, Good, Easy, undo, bury, flag and replay audio; the defaults follow Anki's desktop
  keys where a key exists.
- **Screen awake.** While a remote is paired and a review is open, the app turns the idle timer
  off, and it turns it back on when the remote disconnects or the review closes. Input from the
  remote is accepted while the screen is dimmed in that state.
- **The web client.** The same actions map from the Gamepad API and from keyboard events, Anki's
  desktop keys included, and the page holds a screen wake lock while a gamepad is connected
  during a review.
- **Phase 0.** The remote in both modes, on iPhone, iPad and the web, is part of the spikes'
  device matrix (SPEC-334 row 1.2).

### Consequences

- Good, because the owner can study from the remote alone, on either device or in the browser.
- Good, because one build serves both devices.
- Bad, because every screen needs a compact and a regular layout, and both are tested.
- Bad, because two input paths for the remote must stay mapped to the same actions.

### Confirmation

- The input tests: each mapped action fires from a gamepad event and from a key command.
- The idle-timer test: off while a remote is paired and a review is open, on otherwise.
- The owner's device sessions, with the remote in both modes on iPhone, iPad and the web.

## What would make this wrong

- The remote's keyboard mode sends keys that collide with system shortcuts on iPad, which the
  device session reads; the mapping screen then needs per-mode defaults.
- The browser does not deliver gamepad events while the page is backgrounded or dimmed, which the
  web harness measures.

## More Information

- SPEC-334 (rows 1.2, 1.4 and 1.5; R4, R15).
- ADR-335 (the SwiftUI client), ADR-336 (the web client).

## Amendment: the web maps two grades, Again and Good (SPEC-366)

ADR-377 amends the Decision Outcome's mapping as the web client reads it. The web maps show
answer, Again, Good, undo, bury, flag and replay audio: Hard and Easy leave its default map, its
mapping screen and its messages, and a stored Hard or Easy binding is dropped when the mapping is
read. Key `1` is Again and key `3` is Good; on the gamepad, the d-pad's left is Again and its right
is Good; on the stick, left is Again and right is Good. Keys `2` and `4`, the d-pad's up and down
and the stick's up and down fire nothing. The native client's list stands until the native half of
the two-button work amends it. The rest of the record stands.

- Two grades on the web, Again and Good: chosen because every surface that grades a card offers exactly two grades.
- Keys `2` and `4` and the d-pad's up and down kept as aliases of the two grades: rejected because a Hard or Easy press would record a grade the learner did not press.
- Amending this record only when the native client changes too: rejected because it would state four grades for the web while the web offers two.

## Amendment: a native macOS destination for the one app (SPEC-679)

This amendment adds a destination to the Decision Outcome's one SwiftUI target. The target
`DeckStreak` in `ios/app.yml` gains a macOS destination, built natively from the same sources:
no second app target, no second project and no Mac Catalyst build. Its minimum version is macOS
14.0, the first macOS release that carries the Observation framework the shared sources' models
and bindings use, and their sensory feedback. The
iPhone and iPad destinations, their deployment target, their scheme and their tests stand.

- **The Mac idiom.** On a Mac the app sets body text at 13 pt and controls 28 pt high, with menus and windows: the
  review's actions sit in a menu with their keys, the decks and the review open in a resizable
  window, and settings open in a window of their own.
- **The remote and the keys.** One set of menu commands carries the keys, which a Mac shows in its
  menu bar and an iPad reads as key commands; a gamepad is read through the GameController
  framework on both.
- **Screen awake.** While a remote is paired and a review is open, the Mac app keeps the display
  awake through the system's activity assertion, and ends it when the iOS app would turn its idle
  timer back on.
- **Platform code.** A call the macOS SDK lacks stays behind a platform conditional for iOS, and
  gains a macOS counterpart where the Mac needs the behaviour: the card's web view host, the
  VoiceOver state and announcements, and the sync credential's keychain item.
- **The engine.** The engine's XCFramework holds a third slice, for macOS, next to the device and simulator
  ones (ADR-345); SPEC-679 section 9 holds that build and its CI lane.

The rest of the record stands.

- One app target with a native macOS destination: chosen because it adds the Mac with one target, one set of sources and one scheme.
- Mac Catalyst: rejected because it carries the iPad app's UIKit layer to the Mac, so its controls, menus and windows are translations of the Mac's own, which a native destination uses directly.
- A second app target or project for macOS: rejected because it doubles the settings, schemes and lanes for the same sources, as this record's two-app option would.
- The iPad app run unchanged on a Mac: rejected because it keeps touch-sized controls and the iPad layout, with no Mac menus or windows.
- A later minimum version: rejected because each dated API the sources call starts at macOS 14.0, and each later release leaves out the Macs that stop at the one before it.
- An earlier minimum version: rejected because the Observation framework the models use starts at macOS 14.0, so every model and binding would be rewritten first.

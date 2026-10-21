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

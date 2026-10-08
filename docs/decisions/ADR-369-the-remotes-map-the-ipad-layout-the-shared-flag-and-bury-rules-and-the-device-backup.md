# ADR-369: The remote's map, its stored mapping, the screen awake rule, the iPad layout, the shared flag and bury rules, the device backup and the iOS upload's wait

- **Status:** proposed.
- **Decides:** SPEC-358 (issue #633).
- **Builds on:** ADR-342 (layouts, the remote, the screen awake), ADR-337 (the owner's taps),
  ADR-356 (the engine's adapters and their tables), ADR-359 (the adapter's stored choices),
  ADR-368 (the full-sync choice's one rule).
- **Mutation band:** S35800-S35899.

## Context

The iPhone and iPad client reaches Phase 1 parity with the web client in one delivery of three
pull requests: bury and flag with the iPad's two columns (part a); the remote, its mapping
and the screen kept awake (part b); the session's sync with the full-sync choice (part c).
ADR-342, ADR-337, SPEC-345, SPEC-348 and the web client's ruled drafts settle the screens' shape,
the owner's tap for every one-way write, the native allow-list and the choice's order. They leave
seven questions open, each a real fork: where the remote's button map lives, how the mapping is
stored, how the idle timer is scoped to a paired remote, how the iPad splits the review and the
deck list, where the flag and bury rules live once the native adapter needs them, how a closed
collection is backed up on a device, and what the iOS client does with an upload before it holds
the owner's session.

## Decision drivers

- **One copy per rule.** A rule has one definition; a second copy drifts (ADR-342 consequences).
- **Swift stays thin, by census.** Every decision a Swift file takes is counted against a ceiling
  that does not rise; a rule that can be a pure adapter function is one, and is tested on Linux.
- **Offline study keeps every review.** A review made without a network stays on the device and
  syncs later (ruling 309(b)); nothing in this delivery may discard one.
- **The owner's tap for every one-way write.** A write that replaces one side of the collection
  is made on the owner's gesture alone (ADR-337), after a backup, never on a default.
- **The boundary is the compiler.** The crate graph forbids adapter-to-adapter edges; a rule two
  adapters share belongs in the core.

## Decisions, and the alternatives each was chosen against

### D1. The remote's map lives in the native adapter

The map is `crates/ffi/src/remote.rs`: the intents (confirm, again, hard, good, easy, undo, bury,
flag, replay), the side rule as one table, and the defaults per mode, chosen by the control's
position on an extended gamepad (face buttons by position, shoulders, d-pad) and by the key for a
keyboard. The web client keeps its own page map. One census,
`scripts/tests/test_remote_mapping_parity.py`, holds both sets of defaults equal to SPEC-343's
default mapping, and refuses a planted drift on either side by name. The precedent is ADR-356 D3's
per-adapter table with a parity test.

| alternative | why it lost |
|---|---|
| A core-held map that both clients call | the page's key handler must decide synchronously on the main thread while the engine sits in a Worker (SPEC-338), and SPEC-350 part 2 keeps its mapping store in the page |
| One JSON file both clients import | it edits landed web code and collides with SPEC-350 part 2; it stays recorded as the convergence path (#630) |
| A map written in Swift | stored choices cannot use the forbidden preference stores, the decision ceilings are 6 or less, and it cannot be tested on Linux |
| Independent maps with no census | the two drift apart, as ADR-342's consequences warn |
| The map in the core | a button map is not an engine concern (ADR-356 D1) |

### D2. The stored mapping is one adapter file

`RemoteMapping` is one file outside the collection and its media folder, never synced, written as
ADR-359 D3's `VoiceChoices` is: one line per assignment, `mode<TAB>control<TAB>intent`, written
whole to a temporary file and renamed; a line that does not parse is skipped; a mode with no valid
line takes its defaults; assigning a control moves it off its old intent in that mode.

| alternative | why it lost |
|---|---|
| The platform's preference stores (UserDefaults, @AppStorage) | forbidden everywhere by the thin-Swift census |
| The collection's configuration | it syncs, which makes a per-device choice a write to the shared collection |
| The Keychain | a button map is not a secret |
| No persistence | the owner re-assigns every control at every launch |
| Sharing `VoiceChoices`' file | two formats in one file, and one damaged line spoils both |

### D3. The screen awake rule is a pure adapter function

`Awake::wanted` is true exactly when a review is shown AND the scene is active AND (an extended
gamepad is connected OR (a keyboard is connected AND a mapped key has driven this review)). Swift
only assigns the answer to the idle timer, in the one `input` role, at every change of its inputs.
With no remote paired the idle timer stays on. A device-only check (#629) measures whether
keyboard-mode presses reset the timer anyway.

| alternative | why it lost |
|---|---|
| Gamepad only | drops keyboard mode, which ADR-342 names |
| Any keyboard connected | a keyboard case holds every review awake: ADR-342's rejected "every session" |
| Every review | a touch-only review needs no help and the screen would never dim |
| Left to the system | a controller press does not reset the idle timer, so the screen dims mid-review |
| The rule written in Swift | it spends the decision budget and cannot be tested on Linux |

### D4. The iPad layout

Two columns, the decks and the review (ADR-342). One Settings sheet opens from the sidebar's one
toolbar button, Account first and Remote second; the shell's Account button becomes Settings in
part b, which edits SPEC-347's shell UI test step (an edit, named, not a weakening). The sync status
sits at the sidebar's foot. The sync choice is a sheet with Cancel first and focused. Compact width
takes the stack by horizontal size class, never by device. The review's state lives in the model,
so hiding the sidebar keeps the shown card and its side.

| alternative | why it lost |
|---|---|
| A third column | it takes the review's width and adds a choice at every glance (Hick) |
| A TabView | the tab bar and the sidebar are never both shown, and the deck list must stay beside the review |
| A Settings bundle | it is the preference store the census forbids |
| One toolbar button per area | clutter; SPEC-347 holds the shell to one button |

### D5. The flag and bury rules move into the core

`RED`, `toggled_red`, `BURY_USER`, `BuryOf` and `bury_of` move to
`crates/engine-core/src/review.rs`, unchanged in behaviour. The web engine's `wasm32` module takes
them over its existing `wasm32` edge; the native `flag` takes the queued card's flag, as the web
does. Rows are re-anchored, never retired: a moved rule's rows move crate, path and killer in the
same commit.

| alternative | why it lost |
|---|---|
| `crates/ffi` depending on `crates/web-engine` natively | an adapter-to-adapter edge the graph forbids |
| A second copy in the adapter or in Swift | two copies of one rule (ruling 299 OQ7) |
| Swift building the request bytes | Swift would hold the mode and the toggle, and the census counts both |
| A new closed core read of the card's flags | a statement for a value the queued card already carries |
| A protobuf writer in the native adapter for the two requests | a second codec beside the core's; the core already encodes the native press's request (SPEC-365), and encodes these two the same way |

### D6. The native device backup, and retention in one core function

A download's backup closes the collection by (3,1), copies it whole to a temporary file and renames
it, reopens it with the kept open request, and the core reads the copy's ids for
`Confirmed::backed_up`. Retention (keep the two newest; delete the oldest only after the new one is
accepted) is ONE core function both adapters call. Where SPEC-357 part c's landed form places
retention differently, the landed form wins and part c follows it.

| alternative | why it lost |
|---|---|
| Copying an open collection | the engine holds it exclusively; the copy can tear |
| The engine's package backup | a zip whose ids the core cannot read without an import |
| `vacuum into` through the read door | a write through the door ADR-356 D7 closes |
| Dropping the dispatcher to close | an implicit close, with no named step |
| Retention in each adapter | a second copy of the write-before-delete order; the web client's draft says what the adapter keeps, not where the order's code lives |

### D7. The iOS upload waits for the native owner session (#627)

Until the native client holds the owner's session, an upload is refused by name at the snapshot
step, both sides untouched; the download works.

| alternative | why it lost |
|---|---|
| Swift `URLSession` to the API | the census forbids it, and the app holds no owner credential |
| The adapter making the HTTPS call | a second network client and a second credential |
| Skipping the snapshot check | SPEC-334 R8 requires it |
| The owner's word in place of the check | the web client's draft rules that a word is not a check |
| Hiding the upload direction | the owner still needs to see what each side loses (web R16) |

## Consequences

- One new adapter module (`remote.rs`), one stored file and one Swift role (`input`) with a
  ceiling; no existing ceiling rises.
- The web engine loses five items and gains none; its tests for them move with them.
- Five mutation rows are re-anchored to the core and two to the native column; the sync part
  re-anchors one more. The shared rule's rows keep their ids and properties.
- The native allow-list holds ten pairs at the read SPEC-358 names (`allow_list.rs:28`);
  SPEC-365's delivery (#711) leaves nine and the Undo delivery (#714) eight. Part a adds two, ten
  on that base, and part c three more, thirteen. Each part's figure is the count measured at its
  cut plus its pairs.
- Part a builds no native undo: native Undo leaves the allow-list and the native run refuses
  (3,8) (#714).
- An upload from iOS is unavailable until #627; the status and sheet say so by name.
- The two clients keep separate stored mappings; the parity census is their only link until #630.

## What would make this wrong

- A key command never beats a focused card view (P4), so the remedy would be a different input path.
- The gamepad framework delivers no press while the screen is dimmed, so D3's rule cannot keep it lit.
- A scratch download measured too slow on a large collection, which would move the download's
  copy to a streamed form.
- SPEC-357's landed names, retention or backup call differ from D6's.

## Formal models

No new entry. The full-sync choice's order is the shared rule's `FullSyncChoice`, held by the
core's typed states, which this delivery calls and never re-implements. The side rule and the
screen awake rule are total functions over small finite inputs, judged exhaustively by SPEC-358's
A15 and A19.

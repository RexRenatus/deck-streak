# SPEC-600: the iPhone and iPad app syncs at the session's moments and lists the decks a sync brings

- **Context(s):** the app (`ios/App`).
- **Decided by:** ADR-620 (this SPEC's own), under ADR-521 (the native sync facade and the
  full-sync choice) and beside ADR-388 D12 (the web client's session syncs).
- **Beside:** SPEC-503 (the app's sync section and its full-sync choice), SPEC-358 R18 and R33 (the
  session's sync moments) and SPEC-347 (the app shell and its Keychain item).
- **Schematic:** `docs/schematics/native-sync-moments.md` (the sequence of the five moments, their
  guards and the deck list read again after a sync).
- **Mutation band:** S60000-S60099.

## 1. The problem, measured

At the base the app syncs only from the Sync button in the account sheet, and lists its decks once,
at launch:

- `git grep -n -E '\.sync\(\)' <base> -- ios/App/Sources` finds one call site,
  `ios/App/Sources/SyncView.swift:13`, the button's.
- `git grep -n 'decks()' <base> -- ios/App/Sources/AppModel.swift` finds one read,
  `ios/App/Sources/AppModel.swift:61`, inside `start()`, which runs once behind its `started` flag.
- `git grep -c scenePhase <base> -- ios/App/Sources` finds nothing: no file sees the app leave or
  return to the foreground.

So a fresh install that signs in and downloads its collection through the full-sync choice still
lists the decks it had before the download until it is opened again, and nothing syncs at launch,
on a return to the foreground, after a sign-in or when a review closes. SPEC-358 R18 and R33 name
those moments; ADR-388 D12 is the web client's same rule.

## 2. Requirements

- **R1.** When signed in, the app runs one normal sync through the sync section's own path: (a)
  once after the collection opens and its decks are listed at launch; (b) each time the scene
  returns to active after it was in the background, where an inactive turn with no background
  between, and the first active at launch, are no moment; (c) once after a sign-in stores its host
  key; (d) once after the review screen's own close action closes an open review; (e) from the Sync
  button, as before.
- **R2.** No automatic moment calls the engine while signed out, and the sync section's sentence
  stays as it was. A sync asked while one runs, or while the full-sync choice is open with its
  counts shown, is dropped, never queued. Moments (a) and (b) wait while a review is open, because
  a normal sync discards the undo queue (SPEC-358 M10); (c), (d) and (e) are the learner's own acts
  and run.
- **R3.** An automatic sync whose answer needs a full sync stops where SPEC-503 R4 and R7 stop it:
  the unsynced read, then the counts, which wait in the account sheet's sync section. It never
  downloads, confirms, cancels or uploads, and no sheet opens by itself.
- **R4.** The download runs only from the tap SPEC-503 names, and the normal sync after it as
  before.
- **R5.** After every sync that completes (a normal sync answered with no full sync needed, or a
  written download and the normal sync after it), the app lists the decks again through the
  session, so a new install lists the downloaded decks with no relaunch. A chosen deck the new list
  lacks is cleared, and that clearing is no close moment, so listing the decks again never starts a
  second sync.
- **R6.** No sentence, label, status line, notice or empty state is added or changed. The facade,
  the minimum-client statement read, the endpoint rule, media off, the full-sync choice's order,
  the one Keychain item and the sign-in itself are unchanged, and no engine call, pair or
  allow-list entry changes.
- **R7.** The shell's model takes the shell's engine calls through one small seam, `ShellEngine`,
  so a test drives the moments with a recording engine; the app passes its one engine session, and
  a review still takes that same session.
- **R8.** The moments are held by XCTest on one iPhone and one iPad simulator, and the shell's hand
  over of each scene phase and of the review's close by a source census.

## 3. Acceptance criteria

```acceptance
A1: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_a_signed_in_launch_syncs_once_after_the_decks_are_listed
A2: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_a_return_from_the_background_syncs_once_and_an_inactive_turn_does_not
A3: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_signed_out_or_with_a_review_open_no_automatic_sync_reaches_the_engine
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_a_sync_asked_while_one_runs_or_while_the_choice_is_open_is_dropped
A5: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_an_automatic_sync_that_needs_a_full_sync_stops_at_the_counts_and_downloads_nothing
A6: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_a_sign_in_that_stores_a_key_runs_one_sync
A7: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_the_review_close_action_runs_one_sync_and_a_cleared_deck_runs_none
A8: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_the_decks_are_read_again_after_a_sync_and_after_a_written_download
A9: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncMomentsTests/test_a_chosen_deck_the_new_list_lacks_is_cleared
A10: python3 -m unittest discover -s scripts/tests -p test_ios_sync_moments.py
A11: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
A11: python3 -m unittest discover -s scripts/tests -p test_ios_app_tree.py
A11: python3 -m unittest discover -s scripts/tests -p test_ios_undo_reach.py
A12: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/SyncModelTests
```

A1 holds R1 (a); A2 holds R1 (b); A3 holds R2's signed-out and review-open waits, each beside a
control in which the same moment syncs; A4 holds R2's drop, with a control after the run ends and
the choice closes; A5 holds R3 and R4 for an automatic sync; A6 holds R1 (c); A7 holds R1 (d) and
R5's clearing that runs no sync; A8 holds R5 after a normal sync and after a written download; A9
holds R5's chosen deck. A10 holds R8's census: the shell hands each scene phase to the model's one
entry, the review's close goes through the model, and each moment's call site occurs once, with an
examined count refused at zero and planted sources refused by name. A11 holds R6 and R7 against
the thin-Swift, app-tree and undo-reach censuses, unchanged; A12 holds SPEC-503's sync model,
unchanged. The XCTest lines run on the macOS lane only.

## 4. File manifest

| path | change |
|---|---|
| `docs/specs/SPEC-600-the-iphone-and-ipad-app-syncs-at-the-sessions-moments-and-lists-the-decks-a-sync-brings.md` | new: this SPEC |
| `docs/decisions/ADR-620-the-native-app-syncs-after-the-open-on-the-foreground-after-sign-in-and-at-a-reviews-close-and-never-downloads-by-itself.md` | new |
| `docs/schematics/native-sync-moments.md` | new: the sequence |
| `docs/red-first/SPEC-600.md` | new: the red and green lines |
| `changelog.d/native-sync-moments-600.md` | new: the fragment |
| `ios/App/Sources/SyncMoments.swift` | new: the moments, their guards, the drop and the chosen deck kept |
| `ios/App/Sources/AppModel.swift` | changed: the seam, the moments' call sites and the decks listed again |
| `ios/App/Sources/SyncModel.swift` | changed: the completed sync it reports |
| `ios/App/Sources/DeckListView.swift` | changed: the shell hands each scene phase and the close to the model |
| `ios/App/Sources/EngineSession.swift` | changed: one conformance declaration |
| `ios/swift-roles.json` | changed: the two new files and the decision counts |
| `ios/AppTests/SyncMomentsTests.swift` | new: A1 to A9 |
| `scripts/tests/test_ios_sync_moments.py` | new: A10 |
| `scripts/mutation-rows.d/S60000-S60099.json` | new: the band's rows (section 7) |
| `ios/App/Sources/SyncView.swift` | unchanged |
| `ios/App/Sources/AccountView.swift` | unchanged |
| `ios/App/Sources/DeckStreakApp.swift` | unchanged |
| `ios/AppTests/SyncModelTests.swift` | unchanged |

## 5. What this does NOT cover

- Media sync and its pair (1,2): the moments run SPEC-503's normal sync with media off.
- An upload from the native client, as SPEC-503 section 5 leaves it.
- A sync while the app is in the background.
- The status line (SPEC-358 R19), the first-run empty state, and a full-sync notice outside the
  account sheet.
- Any sign-in method other than the existing sync login.
- Every engine, adapter and server change, the server's own sync cadence included.

## 6. Risks

- A return to the foreground on a slow network starts a sync the learner did not ask for; it runs
  through the same facade and timeout as the button's, and a review opened meanwhile is not held by
  it (the review takes the same session, one call at a time).
- A sync answered with a full sync at launch leaves the counts waiting in the account sheet, which
  the learner may not open for a while; nothing is lost, and every later moment is dropped until
  the choice closes (R2).

## 7. Mutation rows

Band S60000-S60099: S60000 the shell's hand over of the scene phase; S60001 the review's close
through the model. Each row's killer is A10's census, named in full in the band's file.

The rows whose killer is an XCTest (A1 to A9) are deferred, as SPEC-503 section 7 defers S50303:
the registry's prover runs cargo and unittest killers only, so each is registered with its XCTest
killer when the macOS lane runs mutation rows.

## 8. Formal models

None: the moments are the app's own triggers on the main actor over the unchanged facade, so the
handshake, the full-sync choice and the credential models see no new step, and no cover names a
changed file.

## 9. What only a device or a person proves

A1 to A9 on the iPhone and iPad simulators, and a fresh install's first sync against the deployed
service, whose decks list with no relaunch.

# SPEC-679: a native macOS destination for the one app target

- **Issue:** none filed for this record. **Context(s):** `repo` (`docs/` and `scripts/tests/`).
- **Decided by:** ADR-342, through the amendment this SPEC's pull request appends to it; no new
  ADR. It cites ADR-345 and amends no other record.
- **Status:** built when its pull request lands: the amendment, this SPEC and their docs test.
  Section 9 holds what the next two pull requests build. It holds `docs/red-first/SPEC-679.md`.

## 1. The problem, measured

Measured at dev's commit `36744c89a7249763c5383e8f221d6ef9ebb820f4` by `git show` of each file
named here and by `git grep -n` over the tree, every exclude paired with a positive `.`:

- ADR-342 decides one SwiftUI target for iPhone and iPad (its Decision Outcome, line 39) and holds
  one appended amendment already (line 83). Its front matter reads `status: proposed` (line 2).
- `ios/app.yml` builds the target `DeckStreak` for `platform: iOS` alone (line 28), with device
  family `1,2` (line 45), and links three local packages, `EnginePackage`, `HarnessWire` and
  `CardIsolation` (lines 35 to 39).
- `HarnessWire` and `CardIsolation` declare macOS beside iOS already, at the minimum this SPEC
  names (`ios/HarnessWire/Package.swift` line 8, `ios/CardIsolation/Package.swift` line 9), and the
  Apple workflow's `swift test` builds both for macOS (`.github/workflows/xcframework.yml` lines
  211 and 238). `EnginePackage` declares iOS alone (`ios/EnginePackage/Package.swift` line 9),
  over an XCFramework of a device slice and a simulator slice, which the workflow builds and
  checks as exactly those two (lines 116 to 126).
- The target's 42 Swift sources (30 under `ios/App/Sources/`, 12 in the two Swift packages) hold
  the platform-bound sites below. A `git grep -n` for `import UIKit`, `UIApplication`, `UIDevice`,
  `UIScreen`, `#if os(`, `#if canImport(`, `UIViewRepresentable`, `UIViewControllerRepresentable`,
  `UIKeyCommand`, `UIImpactFeedbackGenerator`, `GCController` and
  `navigationBarTitleDisplayMode` finds 11 lines in 7 of the target's files, and a second one for
  every other `UI` type and iOS-only modifier adds the rest; no `UIKeyCommand`, `GCController`,
  `UIDevice` or `UIScreen` site exists in them.

| site | path and line | class | macOS form |
|---|---|---|---|
| the card's web view host, a `UIViewRepresentable` | `ios/App/Sources/CardFaceView.swift:41` | needs a macOS counterpart | an `NSViewRepresentable` host of the same factory's view |
| the card's page zoom from `UIFontMetrics` | `ios/App/Sources/CardFaceView.swift:27` | needs a macOS counterpart | a zoom of 1 at the Mac's 13 pt body text, which the card's web view factory states |
| a VoiceOver announcement, `UIAccessibility.post` | `ios/App/Sources/CardRefusalWords.swift:31` | needs a macOS counterpart | an announcement both platforms post |
| the VoiceOver state, `UIAccessibility.isVoiceOverRunning` | `ios/App/Sources/ReviewModel.swift:51` | needs a macOS counterpart | the workspace's VoiceOver state, read in `AppModel.swift` and handed to the review as its default |
| `import UIKit` | `ios/App/Sources/CardFaceView.swift:3`, `CardRefusalWords.swift:2`, `ReviewModel.swift:2` | needs a platform conditional | imported for iOS only where a UIKit name stays; `CardFaceView.swift` and `ReviewModel.swift` keep none |
| the audio session, `AVAudioSession` | `ios/App/Sources/ClipPlayer.swift:35` | needs a platform conditional | iOS only: a Mac app sets no session; the conditional sits in `InstalledVoices.swift`, which the player calls |
| `navigationBarTitleDisplayMode(.inline)` | `ios/App/Sources/CardHealthView.swift:31` | needs a platform conditional | iOS only |
| the sync credential's keychain item | `ios/App/Sources/SyncCredentialStore.swift:38` | needs a platform conditional | the data protection keychain on macOS, so the same access class holds |
| the app's one scene, a `WindowGroup` | `ios/App/Sources/DeckStreakApp.swift:14` | builds on macOS as it is | gains menus and a default window size with no platform conditional; its settings window comes in a later pull request |
| the Info.plist's iOS keys: scenes, launch screen, orientations | `ios/App/Info.plist:31` to `:49` | builds on macOS as it is | read by iOS alone |
| the window, motion and context-menu refusals | `ios/CardIsolation/Sources/CardIsolation/WindowRefusal.swift:62` | builds on macOS as it is, under `#if os(iOS)` | the Mac web view's own menu is held in section 9 |
| inline video playback | `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:198` | builds on macOS as it is, under `#if os(iOS)` | none needed |
| the engine package's platforms and slices | `ios/EnginePackage/Package.swift:9` | needs a macOS counterpart | macOS declared, and a macOS slice |

- The newest APIs the sources call that Apple's documentation dates start at macOS 14.0: the
  Observation framework's models (`@Observable`, six files) and bindings (`@Bindable`, seven
  views in six files) and `sensoryFeedback` (`ios/App/Sources/ReviewView.swift:42`), so macOS
  14.0 is the lowest minimum they allow. The second `sensoryFeedback` form (`:43`) and the change
  handler with no parameters (`ios/App/Sources/DeckListView.swift:32`) are dated again by the macOS build.

## 2. Requirements

- **R1.** ADR-342 carries one appended section whose heading names macOS and SPEC-679, and no
  line ADR-342 held before it is removed or changed: its title, front matter, Decision Outcome and
  earlier amendment stand.
- **R2.** The amendment adds macOS to the destinations of the one target, `DeckStreak` in
  `ios/app.yml`, built natively, and names Mac Catalyst, a second app target or project, and the
  iPad app run unchanged on a Mac as rejected, each with its reason.
- **R3.** The amendment names macOS 14.0 as the destination's minimum version, and this SPEC
  names the same one.
- **R4.** The amendment states the Mac's sizes and structure: body text at 13 pt, controls 28 pt high, and the Mac's menus and windows.
- **R5.** The iPhone and iPad destinations stand: `ios/app.yml` keeps the target's iOS
  deployment target, its device family `1,2`, its scheme and its two test targets.
- **R6.** This SPEC is named by the convention, holds a number no other SPEC holds, and its
  Status line does not read planned.
- **R7.** The macOS build of the target, the engine's macOS slice and the macOS CI lane are left
  to the next two pull requests, each with its criteria in section 9, so each can cite them.

## 3. Acceptance criteria of SPEC-679

| id | criterion | decided by |
|---|---|---|
| A1 | R1, R2: ADR-342 holds exactly one section whose heading names macOS and SPEC-679, after every line it held at the base, and it names Mac Catalyst, a second target and the iPad app run unchanged on a Mac as rejected | `test_macos_destination_record.py` `test_adr_342_carries_one_macos_amendment` |
| A2 | R3: the amendment and this SPEC each name one minimum macOS version, and the two are equal | `test_macos_destination_record.py` `test_the_amendment_and_the_spec_name_one_minimum` |
| A3 | R4: the amendment names body text at 13 pt, controls 28 pt high, and the Mac's menus and windows | `test_macos_destination_record.py` `test_the_amendment_states_the_mac_idiom` |
| A4 | R5: the DeckStreak target keeps its iOS deployment target, read as a string or as the `iOS` key of a map, its device family `1,2`, its scheme and its two test targets | `test_macos_destination_record.py` `test_the_ios_destination_stands` |
| A5 | R6: every SPEC file is named by the convention | `test_planned_specs.py` `test_every_spec_file_is_named_by_the_convention` |
| A6 | R6: no SPEC number is held twice | `test_planned_specs.py` `test_no_number_is_held_twice_across_judged_and_planned` |
| A7 | R6: no SPEC in `docs/specs/` reads planned | `test_planned_specs.py` `test_no_judged_spec_reads_planned` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_macos_destination_record.py -k test_adr_342_carries_one_macos_amendment
A2: python3 -m unittest discover -s scripts/tests -p test_macos_destination_record.py -k test_the_amendment_and_the_spec_name_one_minimum
A3: python3 -m unittest discover -s scripts/tests -p test_macos_destination_record.py -k test_the_amendment_states_the_mac_idiom
A4: python3 -m unittest discover -s scripts/tests -p test_macos_destination_record.py -k test_the_ios_destination_stands
A5: python3 -m unittest discover -s scripts/tests -p test_planned_specs.py -k test_every_spec_file_is_named_by_the_convention
A6: python3 -m unittest discover -s scripts/tests -p test_planned_specs.py -k test_no_number_is_held_twice_across_judged_and_planned
A7: python3 -m unittest discover -s scripts/tests -p test_planned_specs.py -k test_no_judged_spec_reads_planned
```

Each new test reads the committed text as text and asserts a literal it writes: the heading, the
version, the four idiom phrases, the device family. Each prints its examined count, and each is
seen red at the base, where ADR-342 holds no macOS section and this SPEC does not exist, but A4, a guard whose literals hold at the base, which the red-first record lists as not red.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/decisions/ADR-342-one-universal-iphone-and-ipad-app-driven-by-touch-keys-or-an-8bitdo-remote.md` | repo | changed: one amendment section appended (R1 to R4) |
| `docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md` | repo | added |
| `scripts/tests/test_macos_destination_record.py` | repo | added: the four tests of R1 to R5 |
| `docs/red-first/SPEC-679.md` | repo | added |
| `changelog.d/macos-destination-679.md` | repo | added |

## 5. What this does NOT cover

- It changes no file under `ios/`: the target's macOS destination, its platform conditionals,
  its macOS counterparts and the engine's macOS slice are section 9's first pull request
  (ADR-345).
- It changes no workflow and admits no runner: the macOS CI lane is section 9's second pull
  request (ADR-345).
- It changes no web client and no mapping of the remote's buttons (ADR-342).

## 6. Risks

- **A tree-scanning guard reads the new test module.** The python stage's censuses read every
  test module; the build reads each census over the new module before its push, and CI runs the stage whole.
- **The record and the build drift.** A later macOS deployment target in `ios/app.yml` could
  differ from the amendment's minimum; section 9's first criterion holds the two equal by test.
- **The Mac web view's own context menu.** The card's iOS context-menu refusal sits under
  `#if os(iOS)`, so a Mac menu is a surface it does not reach; section 9 holds it.
- **The keychain differs on macOS.** Without the data protection keychain, the access class the
  iOS item sets does not bind on a Mac; section 9 holds the item's macOS form.

## 7. What only CI proves

- The seven criteria run in the CI job that runs `scripts/tests`; none needs a macOS runner.

## 8. Formal model

No state, actor or write path is added: the record is text, and its test reads it. No model
checker runs.

## 9. Delivered by the next pull requests

### 9a. The target's macOS build and the engine's macOS slice

| id | criterion | delivered by |
|---|---|---|
| A8 | `ios/app.yml`'s DeckStreak target declares `supportedDestinations` of iOS and macOS, with a macOS deployment target equal to this SPEC's minimum, and builds neither Mac Catalyst nor the iPad app on a Mac | the target's pull request |
| A9 | `EnginePackage` declares macOS, the XCFramework holds a third slice, for macOS, next to the device and simulator ones, and the workflow's slice check and per-slice typecheck name all three | the target's pull request |
| A10 | every section 1 site classed as needing a platform conditional or a macOS counterpart compiles for macOS in its macOS form, and the iOS build is unchanged | the target's pull request |
| A11 | on macOS, body text is 13 pt and controls are 28 pt high; the review's actions sit in a menu with their keys; the decks and the review open in a resizable window with a default size; settings open in a settings window | the target's pull request, and the settings window a later one |
| A12 | on macOS, the card's web view holds every isolation layer the iOS view holds, its context menu included | the target's pull request |

Its starting facts, each derived again at its own base: XcodeGen's `supportedDestinations` with
a per-platform `deploymentTarget` map; one `CommandMenu` whose buttons carry keyboard shortcuts
serves as Mac menus and as iPad key commands; the workflow's slice check (`xcframework.yml` line
126) and per-slice typecheck (line 131) list the slices exactly.

### 9b. The macOS CI lane

| id | criterion | delivered by |
|---|---|---|
| A13 | a CI job builds the DeckStreak scheme for a macOS destination with the pinned Xcode and runs its unit tests there, whenever a pull request changes an Apple or FFI path | the lane's pull request |
| A14 | the job's runner is admitted by its workflow's file name, as ADR-345 admits the Apple job's, and the lane spends nothing | the lane's pull request |

## 10. References

ADR-342; ADR-345 (D4); ADR-358 and SPEC-347 R4 (the app's generated project); ADR-350 and
SPEC-339 R2 (the engine package); SPEC-349 (the card's isolation); SPEC-393 R10 and R11 (inline
video); SPEC-673 (this SPEC's shape).

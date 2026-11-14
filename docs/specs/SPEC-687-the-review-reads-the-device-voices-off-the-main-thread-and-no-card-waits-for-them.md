# SPEC-687: the review reads the device's voices off the main thread, and no card waits for them

- **Context(s):** the iOS review (`ios/App/Sources/InstalledVoices.swift` and
  `ios/App/Sources/ReviewModel.swift`), its unit tests (`ios/AppTests/ReviewModelTests.swift`) and
  `docs`.
- **Decided by:** ADR-707
- **Status:** delivered with its tests and `docs/red-first/SPEC-687.md`.

Follows SPEC-348 (R13, R14) and SPEC-506.

## 1. The problem, measured

The app's UI tests on the iPhone simulator failed at a card step: a deck row's tap opened the
review, and the app then answered neither the test runner's idle wait nor its queries until the
test failed. Two process samples, which XCTest took during one such failure, show the app's main
thread in one stack in every sample of both: `ReviewModel.next()`, the default `installed` provider
`InstalledVoices.all()` at its `AVSpeechSynthesisVoice.speechVoices()` line, and a wait on a
semaphore inside the speech framework; the first sample also names the deck list's tap,
`AppModel.openReview()` and `ReviewModel.start()` above it. The main thread is blocked in that
wait, so no timer, animation or display link fires there, and no web content process exists yet, so
no card web view has begun to load. The thread did not run again before the test failed.

`ReviewModel` calls its provider on the main actor before every card step, at a reveal and when the
voice picker reads its voices, and the default provider reads the device's list each time. The same
tests pass on the iPad simulator in the same runs. Three more failures at the same steps on the
iPhone kept no process sample; each stalls at a first or next card, which calls the same provider.

## 2. Requirements

R1. The device's voice list is read once per process, off the main actor, from the first time a
review asks for it (`InstalledVoiceList.device`).

R2. No step of a review waits for that read: each step hands the engine the voices listed so far,
none until the read is in, so a line spoken before then speaks in its language's own voice, as a
line whose chosen voice is not installed does today (`EngineSession`, `ClipPlayer`).

R3. Once the read is in, every later step and the voice picker's next reading hand the engine the
whole list.

R4. No existing test, assertion, wait, destination, workflow step or workflow changes, and no retry
or re-run is added.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the device's voice list is read once, off the main thread, and the list keeps what the read returned | `ReviewModelTests/test_a1_the_device_voice_list_is_read_off_the_main_thread` |
| A2 | a review over a list whose read is held shows its first card before the read returns | `ReviewModelTests/test_a2_the_first_card_shows_while_the_voice_list_is_read` |

Each test runs in CI by its name on the iPhone and on the iPad, in the `release-and-app` job's step
"the review screen's tests, Debug, on the iPhone and then the iPad".

```acceptance
A1: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a1_the_device_voice_list_is_read_off_the_main_thread
A2: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a2_the_first_card_shows_while_the_voice_list_is_read
```

## 4. File manifest

| path | context | change |
|---|---|---|
| `ios/App/Sources/InstalledVoices.swift` | ios (speech) | changed: `InstalledVoiceList`, the device's list read once off the main actor, and its `listed()` |
| `ios/App/Sources/ReviewModel.swift` | ios (model) | changed: the default `installed` provider is `InstalledVoiceList.listed` |
| `ios/AppTests/ReviewModelTests.swift` | ios (test) | changed: the two acceptance tests and the read they hold |
| `docs/specs/SPEC-687-the-review-reads-the-device-voices-off-the-main-thread-and-no-card-waits-for-them.md` | docs | added: this SPEC |
| `docs/decisions/ADR-707-a-card-speaks-in-its-language-voice-until-the-device-voice-list-is-in.md` | docs | added: its ADR |
| `docs/red-first/SPEC-687.md` | docs | added: the red-first record |
| `changelog.d/review-voices-off-main-687.md` | changelog | added: the fragment |

`ios/swift-roles.json` is unchanged: `InstalledVoices.swift` keeps its one decision and
`ReviewModel.swift` its six.

## 5. What this does NOT do

- It changes no test, assertion, wait, destination, workflow step or workflow, and adds no retry.
- It does not show that the three failures without a process sample share this cause; the
  delivery's own pushes are read by job and step name on both destinations for that.
- It does not say why the iPad simulator does not stall.
- A voice picker opened before the list is in offers the voices listed so far, and the whole list
  when it next reads its voices.
- The player still resolves a line's voice by identifier or by language on the main actor, as it
  did.

## 6. Risks

- Until the read is in, a line speaks in its language's own voice even where a chosen voice is
  installed (ADR-707).
- A read that never returns leaves every step with the language's own voice and holds one thread
  outside the main actor.

## 7. FORMAL, decided by surface

NOT APPLICABLE, TLA+, Lean and Kani. The change moves one read off the main actor and changes one
default; no protocol, state machine, engine path or crate source gains or changes.

## 8. References

SPEC-348 (R13, R14, R16), SPEC-506, ADR-407, ADR-707.

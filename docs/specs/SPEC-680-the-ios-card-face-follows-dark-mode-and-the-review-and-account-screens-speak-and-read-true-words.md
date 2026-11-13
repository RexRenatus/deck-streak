# SPEC-680: the iOS card face follows dark mode, and the review and account screens speak and read true words

- **Context(s):** the iPhone and iPad app (`ios/App`) and its tests (`ios/AppTests`,
  `ios/AppUITests`).
- **Decided by:** ADR-702 (this SPEC's own), under ADR-615 (every screen reads the Tide roles and
  follows the system appearance) and ADR-523 (the card face announces its refusal once).
- **Beside:** SPEC-347 (the shell and the account sheet), SPEC-348 (the review screen), SPEC-505
  (the card face's refusal words), SPEC-593 (the Tide colours on iOS).
- **Mutation band:** none (section 7). **Model:** none (section 8).

## 1. The problem, measured

At the base (`c1f591d0`), eight gaps a learner meets early on iPhone and iPad:

| id | measured | figure | command |
|---|---|---|---|
| M1 | The card face is always the day face | `EngineSession.swift:206` passes `night: false`, the one `night:` of the app's sources, though the engine's night face carries its night classes (`crates/ffi/src/face.rs:18-19`, `:93-94`) | `git grep -n -F 'night:' c1f591d0 -- ios/App/Sources` |
| M2 | A fresh install shows its one deck with nothing to do and no loading state | 0 `ProgressView` and 0 `ContentUnavailableView` in `ios/App/Sources`; the open makes the engine's default deck alone (`EngineSession.swift:85-86`) | `git grep -c -e ProgressView -e ContentUnavailableView c1f591d0 -- ios/App/Sources` |
| M3 | Every finish reads one sentence | `ReviewView.swift:27-28`, whatever the review sent (`ReviewModel.swift:30`) | `git show c1f591d0:ios/App/Sources/ReviewView.swift` |
| M4 | A grade can take the tap that shows the answer | Again and Good replace Show Answer the moment the answer shows, each enabled at once (`AnswerBar.swift:21-36`, `:65`) | `git show c1f591d0:ios/App/Sources/AnswerBar.swift` |
| M5 | An error that is not a refusal shows its description | `AppModel.swift:105`, read by the list's refusal and the account message (`:63`, `:89`, `:99`) | `git show c1f591d0:ios/App/Sources/AppModel.swift` |
| M6 | The review says nothing to VoiceOver | 0 announcements in `ReviewModel.swift`; the app's one announcement is the card face's refusal (`CardRefusalWords.swift:26-35`) | `git grep -c -e UIAccessibility.post -e AccessibilityNotification c1f591d0 -- ios/App/Sources` |
| M7 | The account form names no content type and cannot submit | 0 `textContentType`, `submitLabel` or `onSubmit`; the user row is unlabelled (`AccountView.swift:15`), Done has no placement (`:51-53`) and the message is not announced (`:38`) | `git grep -c -e textContentType -e submitLabel -e onSubmit c1f591d0 -- ios/App/Sources` |
| M8 | The review's deck name is no header and its counts are three elements | `ReviewChrome.swift:16-18`, `:59-65` | `git show c1f591d0:ios/App/Sources/ReviewChrome.swift` |
| M9 | The budgets | `AnswerBar.swift`, `DeckListView.swift`, `ReviewChrome.swift` and `ReviewView.swift` hold 3 decisions each (the view ceiling), `ReviewModel.swift` 6 (the model ceiling), `EngineSession.swift` and `ReviewSession.swift` 4 each (the session ceiling), `AppModel.swift` 5 and `AccountView.swift` 1 | `git show c1f591d0:ios/swift-roles.json` |

## 2. Requirements

Each requirement's design section comes before its code. A decision that would pass a file's
ceiling goes to a new file of the same role, registered with its count; no ceiling and no role
changes.

- **R1. The card face follows the system appearance.** Every face the review asks for is asked in
  the appearance the system shows: in dark mode the engine's night face, whose body carries the
  night classes.
  *Design.* `EngineSession.face` takes `night` and hands it to the engine's face call in place of
  `false`. `ReviewSession.next` and `reveal` take `night` (false when a caller names none, so every
  existing call reads as before) and pass it on. `ReviewModel` keeps `night`, handed in when it is
  made, and asks each face with it. `AppModel` keeps the appearance and hands it to the open review
  and to each review it opens (`follow(night:)`), and `ShellView` sets it from the environment's
  `colorScheme` on appear and on each change. Nothing overrides the colour scheme. A face already
  shown keeps its appearance until the review asks its next face, the answer or the next card.
  No decision is added. The red step adds the session's two `night` parameters alone, read by
  nothing, so the dark face reads the day face.
- **R2. A first run offers sync, with a loading state.** While the collection opens and its decks
  are listed, the deck list shows a loading state. Once they are listed, with no refusal and no
  account signed in, it offers to bring the collection from the account: one button that opens the
  account sheet, where the sign-in and the sync are.
  *Design.* `AppModel` gains `loading`, true until `start` has opened the collection and listed its
  decks, refusal or not. A new view, `FirstRunOffer.swift`, shows a `ProgressView` while loading
  (identifier `first-run-loading`), then the offer: a sentence and the button "Sync from your
  account" (identifier `first-run-sync`), which opens the account sheet. `DeckListView` holds it in
  a bottom safe-area inset, so the list keeps its rows, the toolbar keeps its one Account button
  and its identifier, and the list keeps its one navigation title. The new view holds 3 decisions,
  the view ceiling; `DeckListView.swift` and `AppModel.swift` gain none. The red step adds the
  test alone.
- **R3. The finish names what was done.** The end of a review names how many answers it sent, or
  that nothing was due, never one sentence for every finish.
  *Design.* A new words file, `FinishWords.swift`, gives `text(answered:)`: words for no answer,
  for one and for more, each naming its count. `ReviewView` shows them at the finish from the
  answers the model counted. The words file holds 2 decisions; `ReviewView.swift` keeps 3 and its
  one impact and one success feedback. The red step adds the words file returning the base's one
  sentence for every count.
- **R4. A grade cannot take the tap that shows the answer.** When an answer shows, its two grades
  stay closed for a settle interval, so a press that lands as Show Answer turns into Again and
  Good is not sent, and the bar shows them disabled until they open.
  *Design.* A new model, `AnswerGate.swift`, closes the grades when an answer shows and opens them
  once its interval has passed, unless another answer has shown since; a zero interval opens them
  at once. `ReviewModel` takes the interval (zero when a caller names none) and its rating's case
  starts only while the gate is open: a press on a closed gate matches no case and falls to the
  default, which sends nothing, as a second tap does today. `AppModel` hands each review the app's
  interval, `AnswerGate.settle`. `AnswerBar` disables the grades while the gate is closed, so a test that taps a
  grade once it reads enabled keeps its reading. The answer clock (`AnswerClock.swift`) times the
  grade's answer and cannot change what the bar shows, so the gate sleeps for its interval instead.
  The gate holds 3 decisions; `ReviewModel.swift` keeps 6 and `AnswerBar.swift` 3. The red step
  adds the model's interval parameter alone, read by nothing.
- **R5. Errors read as the app's words.** A failure the list or the account sheet shows reads a
  refusal's own sentence, which the engine words, or the app's general sentence, never an error's
  description.
  *Design.* A new words file, `ErrorWords.swift`, gives `general` and `text(of:)`. `AppModel`'s
  three failure sites read it, and its own description fallback goes. The words file holds 1
  decision; `AppModel.swift` falls to 4. The red step adds the words file returning the error's
  description, as the app did.
- **R6. VoiceOver hears the review.** The review says each step once: the card shown, the answer
  when it shows, the grade sent, and after a grade the next card with the queue's counts, or the
  finish in its words.
  *Design.* A new model, `ReviewAnnouncer.swift`, posts each sentence as an announcement through a
  poster that `ReviewModel` is handed (the system's announcement unless a test hands in its own),
  the form of the card face's one door (`CardRefusalWords.swift:26-35`). `ReviewModel` calls it
  after Show Answer, after each grade and after each next step. The announcer holds 2 decisions;
  `ReviewModel.swift` keeps 6. The red step adds the model's poster parameter alone, read by
  nothing.
- **R7. The account form fills, submits and speaks.** The user row is labelled, the password field
  names its content type, its return key signs in as the button does, Done sits in the confirming
  place and the message is announced when it shows words.
  *Design.* In `AccountView.swift`: the user row as labelled content, the password field's
  `textContentType(.password)`, `submitLabel(.go)` and `onSubmit` running the sign-in, Done in a
  `ToolbarItem` at `.confirmationAction`, and the message announced on each change to words. Every
  identifier the shell's tests read stays. The view gains 1 decision, to 2 of 3. The red step adds
  the test alone.
- **R8. The review's deck name is a header and its counts one element.** The deck name carries the
  header trait, and the queue's three counts read as one element.
  *Design.* In `ReviewChrome.swift`: the deck name's `.accessibilityAddTraits(.isHeader)`, the house
  header form, and the three counts in their own stack, one element with identifier `counts`,
  labelled "N new, N learning, N to review" in the order shown. The review flow's existing counts
  reading reads the one element and keeps its expected three counts, one per count; no assertion is
  removed or weakened. No decision is added. The red step adds the test alone.

## 3. Acceptance criteria

```acceptance
A1: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a_dark_card_face_reads_its_night_classes
A2: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/FirstRunFlowTests/test_a_fresh_install_offers_sync_from_the_account_sheet
A3: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/FinishWordsTests/test_each_finish_reads_words_that_name_what_was_done
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a_rating_pressed_as_the_answer_shows_is_not_sent
A5: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ErrorWordsTests/test_every_app_error_reads_words_of_its_own
A6: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_the_review_announces_each_answer_grade_card_and_finish
A7: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/AccountFormFlowTests/test_the_return_key_on_the_password_field_signs_in
A8: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug -destination "platform=iOS Simulator,name=iPhone 17,OS=27.0" -destination "platform=iOS Simulator,name=iPad (A16),OS=27.0" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_the_review_counts_read_as_one_element
```
| criterion | requirement | test |
|---|---|---|
| A1 | R1 | `test_a_dark_card_face_reads_its_night_classes` |
| A2 | R2 | `test_a_fresh_install_offers_sync_from_the_account_sheet` |
| A3 | R3 | `test_each_finish_reads_words_that_name_what_was_done` |
| A4 | R4 | `test_a_rating_pressed_as_the_answer_shows_is_not_sent` |
| A5 | R5 | `test_every_app_error_reads_words_of_its_own` |
| A6 | R6 | `test_the_review_announces_each_answer_grade_card_and_finish` |
| A7 | R7 | `test_the_return_key_on_the_password_field_signs_in` |
| A8 | R8 | `test_the_review_counts_read_as_one_element` |

Each criterion holds the requirement of its number: for R1, a dark face's night classes on the
question and the answer, and none on a day face; for R2, the offer on a fresh install and the
account sheet it opens, with the loading state ended once the decks are listed; for R3, every
finish's words against a planted one-sentence control; for R4, a grade pressed as the answer
shows, not sent, beside a control whose open gate sends it; for R5, every error's words against a
planted description control; for R6, the announcements of a whole review, counted per step; for
R7, the return key's sign-in and the refused login's message; for R8, the counts as one element.
The Swift censuses hold every file's decisions in CI's hygiene job.

## 4. File manifest

| path | change |
|---|---|
| `ios/App/Sources/EngineSession.swift` | the face call takes `night` (R1) |
| `ios/App/Sources/ReviewSession.swift` | `next` and `reveal` take `night` (R1) |
| `ios/App/Sources/AppModel.swift` | the appearance it follows (R1), `loading` (R2), the gate's interval (R4), the error words (R5) |
| `ios/App/Sources/DeckListView.swift` | `ShellView` follows `colorScheme` (R1); the first run's inset (R2) |
| `ios/App/Sources/ReviewView.swift` | the finish words (R3) |
| `ios/App/Sources/ReviewModel.swift` | `night` (R1), the gate (R4), the announcer (R6) |
| `ios/App/Sources/AnswerBar.swift` | the grades disabled while the gate is closed (R4) |
| `ios/App/Sources/AccountView.swift` | the labelled user row, the content type, the submit, Done's place, the announced message (R7) |
| `ios/App/Sources/ReviewChrome.swift` | the header trait and the counts as one element (R8) |
| `ios/App/Sources/FirstRunOffer.swift` | (new) the loading state and the offer (R2) |
| `ios/App/Sources/FinishWords.swift` | (new) the finish words (R3) |
| `ios/App/Sources/AnswerGate.swift` | (new) the grades' settle gate (R4) |
| `ios/App/Sources/ErrorWords.swift` | (new) the error words (R5) |
| `ios/App/Sources/ReviewAnnouncer.swift` | (new) the review's announcements (R6) |
| `ios/swift-roles.json` | each new file's role and decisions, and `AppModel.swift`'s and `AccountView.swift`'s new counts |
| `ios/AppTests/ReviewSessionTests.swift` | the test of R1 |
| `ios/AppTests/ReviewModelTests.swift` | the tests of R4 and R6 |
| `ios/AppTests/FinishWordsTests.swift` | (new) the test of R3 |
| `ios/AppTests/ErrorWordsTests.swift` | (new) the test of R5 |
| `ios/AppUITests/FirstRunFlowTests.swift` | (new) the test of R2 |
| `ios/AppUITests/AccountFormFlowTests.swift` | (new) the test of R7 |
| `ios/AppUITests/ReviewFlowTests.swift` | the test of R8, and the counts reading of the review flow test through the one element |
| `docs/specs/SPEC-680-the-ios-card-face-follows-dark-mode-and-the-review-and-account-screens-speak-and-read-true-words.md` | (new) this SPEC, moved from `docs/specs/planned/` |
| `docs/decisions/ADR-702-the-ios-review-follows-the-colour-scheme-and-speaks-through-one-announcement-poster.md` | (new) its decision |
| `docs/red-first/SPEC-680.md` | (new) the red-first record |
| `changelog.d/uiux-ios-batch1-680.md` | (new) the changelog fragment |

## 5. What this does NOT cover

- The web halves of these gaps, and every other screen of the iPhone and iPad app.
- An undo at a finished deck, a keyboard undo, the flag's words and the large content viewer.
- The sync itself: the offer opens the account sheet's existing sync and changes none of it.
- Words in any language but English: the app's other words are English today.

## 6. Risks

- A face shown when the appearance changes keeps its appearance until the next face; a learner who
  switches mode mid-card sees the change at the answer or the next card.
- The finish counts the answers the review sent, so an answer later undone still counts.
- The settle interval is one constant of the app (`AnswerGate.settle`); a learner who reads fast
  waits that long for the grades, which the bar shows disabled meanwhile.
- An announcement can be cut short by VoiceOver's own reading of a focus change.

## 7. Mutation rows

None.

## 8. Formal model

Not applicable, as measured: `config/formal.json` at the base names no `ios/` or Swift path, and
the change touches app views, models and their tests, with no protocol or state machine a formal
entry names.

## 9. What only a device or a person proves

The loading state, which ends before a test can read it on a simulator; the deck name's header
trait, which no UI test reads; the password field's content type, the user row's label, Done's
placement and the message's announcement, none of which the account test reads at runtime; and
VoiceOver speaking each announcement and the night face shown in dark mode, each read by a person
on an iPhone and an iPad.

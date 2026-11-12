---
status: "accepted"
---

# ADR-702: the iOS review follows the colour scheme and speaks through one announcement poster

Decides SPEC-680. It builds on ADR-615, under which every screen follows the system appearance and
nothing overrides it, and on ADR-523, under which the card face announces its refusal once.

## Context and Problem Statement

- The card face asks the engine for the day face whatever the system shows, though the engine
  lights a night face's body with its night classes.
- The review says nothing to VoiceOver after Show Answer, a grade, the next card or the finish,
  and the account form neither submits from the keyboard nor announces its message.
- `ReviewModel.swift` holds 6 decisions, the model ceiling, and `AnswerBar.swift`,
  `DeckListView.swift`, `ReviewChrome.swift` and `ReviewView.swift` hold 3 each, the view ceiling,
  so a new branch in any of them must live in a new file of the same role.

## Decision

- D1. The appearance is read where SwiftUI gives it, the environment's `colorScheme` in the shell,
  and handed down as one flag: the shell's model keeps it and hands it to the open review and to
  each review it opens, and the review asks every face with it. The session and the engine call
  take it as a parameter, false when a caller names none.
- D2. The review speaks through one poster handed to its model, the system's announcement unless a
  test hands in its own, the form of the card face's one door. A new announcer holds the words and
  the one branch between the next card and the finish.
- D3. A grade waits for a settle gate that the model owns and the bar shows: a closed gate's press
  matches no case of the model's one entry, and the bar disables the grades while it is closed.
  The app's reviews take one constant interval, `AnswerGate.settle`; a review made with no
  interval opens at once.
- D4. Each new branch lives in a new file of the role that needs it: the first run's view, the
  finish and error words, the gate and the announcer.

## What it was chosen AGAINST

- Chosen: one flag read from the environment in the shell and handed down, because it is the one
  place SwiftUI states the appearance and it keeps the session free of UI types.
- Reading the trait collection in the model: rejected because outside a drawing pass it reads no
  scene's appearance reliably, and the model would hold a UIKit read no test can hand in.
- Re-asking the shown face on every appearance change: rejected because it races the press in
  flight and needs branches the model's ceiling does not have; the next face follows instead.
- A bar that drops a press on an enabled button: rejected because the review's UI tests tap a grade
  once it reads enabled, and a learner sees no reason for a lost press.
- Timing the gate through the answer clock: rejected because the clock times the grade sent and
  cannot change what the bar shows.
- Announcing from the views with `onChange`: rejected because a test of the model could not read
  the announcements, and three views would each need a branch at their ceiling.

## Consequences

- Good, because the card face, the review's speech and the account form follow the system and the
  keyboard with no setting of the app's own.
- Good, because a test hands the model its poster and its interval and reads each announcement and
  each dropped press.
- Bad, because a face already shown keeps its appearance until the next face.
- Neutral, because the gate's interval is a constant of the app, not a learner's setting.

## What would make this wrong

- A learner who switches the appearance mid-card and reads the old face as a fault.
- A measured double tap slower than the interval that still grades an unseen answer.
- A VoiceOver reading that drops the announcements on a device.

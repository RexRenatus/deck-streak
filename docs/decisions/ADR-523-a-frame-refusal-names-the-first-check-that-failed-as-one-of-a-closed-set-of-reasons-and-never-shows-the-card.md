# ADR-523: A frame refusal names the first check that failed, as one of a closed set of reasons, and never shows the card

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-505 |
| builds on | ADR-352, ADR-361 (none amended) |
| measured at | integration base `d3b09d5e` |

## Context

The web card frame refuses a card that would escape its document, and the iOS card face shows nothing when its
factory builds no view. Both fail closed, as ADR-352 decided, and neither says why: the web's seven checks all answer
`escaped` (`web/app/src/lib/card/frame-document.ts:49`, `:63-69`; `frame-host.ts:46-51`), the review announces one
sentence for all of them (`web/app/src/lib/study/refusal.ts:15`), and iOS prints a Swift case name
(`ios/App/Sources/CardFaceView.swift:30`). A learner who meets a refused card cannot tell a note type's CSS from hidden
markup, so cannot fix the note. This ADR decides the reasons, their order, what the status says, what it never says,
and the iOS words.

## D1. The reason classes

- **Chosen: seven reasons, one per check, as a closed list (`REFUSAL_REASONS`) and a type over it:** `class-list`,
  `style-closed`, `reparse-element`, `reparse-srcset`, `reparse-target`, `body-attributes`, `host-readback`. A reason
  is a class of fault, the way the checks already divide them, and the frame's `data-card-refused` carries it.
- **One generic reason, as today:** lost, because it is the problem: the learner cannot act on it.
- **A reason per element or attribute found (`link`, `meta`, `base`, `template` each):** lost, because the check that
  finds them is one check; four words for one fault add nothing the learner can do differently.

## D2. The first failing check names it

- **Chosen: the checks run in a fixed order, and the first that fails is the reason:** the class list first (it is
  read before any parse), then over the re-parsed document the head, a stripped element, `srcset`, `target`, the body's
  attributes, and last the host. A card that breaks several checks gets one reason, the same every time, so the reason
  is a function of the card, the classes and the title.
- **Every failing check, as a list:** lost, because one cause usually breaks several checks at once (CSS that ends
  its style element also moves what follows into the body), and the list would name effects as if they were causes.
- **One composition for the frame and the status.** `frameCard` composes the document and its host, `CardFrame`
  renders it, and the review's status reads its reason, so the mark and the status cannot name different reasons. The
  review used to read the document alone (`review.ts:247-252`), so a class or host refusal marked the frame and left
  the status quiet.

## D3. What the status says

- **Chosen: today's first sentence, then the reason, then the next step, then that the card can still be answered,
  buried or flagged,** one message per reason in each of the seven locales. The next step is words only ("edit the
  note type's CSS", "open the note in source mode"); a link waits for an editor that saves.
- **The reason code alone:** lost, because `reparse-srcset` means nothing to a learner.

## D4. No card content is shown

- **Chosen: a refusal shows no part of the card,** not the CSS, not the markup, not a line number, in the status, the
  frame's attributes or a log. The refused card is untrusted input, and the reason is a class, not a message built from
  it; every message is fixed text with no placeholder.
- **Showing the offending CSS or markup, so the learner can find it:** lost. It puts untrusted text on the page beside
  the frame that refused it, and a fragment of a mutation is itself the escape.

## D5. iOS words and announces its refusal

- **Chosen: `CardRefusalWords` maps each `CardViewRefusal` case to a DeckStreak sentence, and any other error to one
  general sentence; the card face shows it and posts one VoiceOver announcement with the same words.** The switch is
  exhaustive, so a new case does not compile without its sentence, and `CardViewRefusal` becomes `CaseIterable` so
  the test reads every case.
- **`String(describing:)` as today:** lost: a Swift identifier in no locale.
- **The web's seven reasons on iOS:** not applicable. The iOS factory makes none of the web's checks; its one refusal
  is a rule list that did not compile.

## D6. The refusal state is audited

- **Chosen: the rendered audit visits the review with a refused card in both palettes.** The review route needs the
  engine, which the audit's build does not carry, so the audit serves a stand-in for the engine's bindings that shows
  one card whose CSS ends its own style element, and checks that the status and the mark name `style-closed` before
  axe runs.
- **Chosen: axe runs in its legacy mode,** which audits the screen around the card's frame and does not enter the
  refused card's empty frame. The audit's one axe call serves every route; the review is the one route that renders a
  card frame. The status, the controls and the frame's title are all outside the frame, so the legacy mode still
  audits everything a learner meets on a refused card.
- **axe's default mode, which enters every frame:** lost, because the refused card's frame is sandboxed with no
  script, and axe's default analyze waits on it and never finishes: the review's two audit tests timed out in it
  after checking the mark and the status, so the audit judged nothing.
- **A second audit file:** lost, because the audit's coverage test holds one visit per route, from the route table.

## Consequences

- `escaped` leaves the code, the messages and the tests; any reader of `data-card-refused` reads a reason.
- Seven messages per locale replace one.
- iOS gains one file of role `view` in the thin-Swift register.

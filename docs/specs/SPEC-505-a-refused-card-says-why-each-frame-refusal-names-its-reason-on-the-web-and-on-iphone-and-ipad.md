# SPEC-505: A refused card says why: each frame refusal names its reason, on the web and on iPhone and iPad

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #938 |
| decided by | ADR-523, under ADR-352 and ADR-361 |
| amends | SPEC-341 R4 (the refusal is a reason, not one word); SPEC-350 R10 and R11 (the refused card's status names its reason, in all seven locales) |
| measured at | integration base `d3b09d5e` (every file this SPEC cites is read there) |
| schematic | `docs/schematics/a-frame-refusal-from-its-check-to-the-reason-the-status-and-the-controls.md` (new) |
| mutation band | S50500-S50599 (no hand row: section 9) |

## 1. The problem, measured

A card the frame refuses tells the learner nothing they can act on. What was read at `d3b09d5e`:

- **One word for seven checks.** `frameDocument` returns `{ refused: 'escaped' }` for six different checks
  (`web/app/src/lib/card/frame-document.ts:49` and `:63-69`): the body's class list, the head after the re-parse (CSS
  that ends its own style element breaks it), a stripped `link`, `meta`, `base` or `template` back after the re-parse, a
  `srcset` back, a `target` back, and the body's attributes. `frameHost` returns the same word when the host does not
  read back (`web/app/src/lib/card/frame-host.ts:46-51`). `FrameDocument` admits no other value (`:15-17`).
- **The status names none of them.** The review announces one sentence for every refused card, "This card cannot be
  shown safely here. You can still answer, bury or flag it." (key `study_card_escaped`,
  `web/app/src/lib/study/refusal.ts:15` and `:30`; `web/app/messages/en.json:176`).
- **The status reads a different check from the frame.** The review's `escaped` runs `frameDocument(html, css)` with no
  classes and no host (`web/app/src/lib/study/review.ts:247-252`), while the frame runs both
  (`web/app/src/lib/card/CardFrame.svelte:15-16`), so a refusal of the class list or of the host marks the frame and
  leaves the status quiet.
- **iOS prints a Swift case name.** The card face shows `String(describing: error)` when the factory builds no view
  (`ios/App/Sources/CardFaceView.swift:30`): `ruleListDidNotCompile`, in no locale, and VoiceOver is not told.
  `CardViewRefusal` has one case (`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:13-16`).
- **The locales.** `web/app/messages/` holds seven files, `en`, `es`, `fr`, `ja`, `ko`, `zh-Hans` and `zh-Hant`, each
  with `study_card_escaped` (`en.json:176`, the others at `:76`).

## 2. Requirements

R1. **The reason type.** `frame-document.ts` exports `REFUSAL_REASONS`, the closed list `class-list`, `style-closed`,
`reparse-element`, `reparse-srcset`, `reparse-target`, `body-attributes`, `host-readback`, and the type
`RefusalReason` over it. `FrameDocument`'s refusal is `{ refused: RefusalReason }`; `'escaped'` is gone.

R2. **Each check its own reason, the first failing check naming it.** `frameDocument` refuses a class list outside
the admitted set as `class-list`; then, over the re-parsed document and in this order, a head that is not the policy
and the card's style as `style-closed`, a stripped element back as `reparse-element`, a `srcset` back as
`reparse-srcset`, a `target` back as `reparse-target`, and body attributes other than the written class as
`body-attributes`. `frameHost` refuses a host that does not read back as `host-readback`. The reason is a function of
the card, the classes and the title.

R3. **One composition for the frame and the status.** `frameCard(html, css, title, classes)` in `frame-host.ts`
composes the frame document and its host; `CardFrame.svelte` renders its result and the review's status reads its
reason, so the frame's mark and the status name the same reason.

R4. **The mark.** The refused frame's `data-card-refused` carries the reason, a member of `REFUSAL_REASONS`.

R5. **The status names the reason and the next step, and never the card.** The review's status for a refused face is
the reason (`Status` holds the seven reasons in place of `escaped`), and its words are the reason's message: today's
first sentence, the reason, the next step, and that the card can still be answered, buried or flagged. No status,
mark, title or log line holds any of the refused card's markup or CSS.

R6. **Seven reasons in seven locales.** `study_card_escaped` is replaced in all seven locale files by seven keys,
`study_card_refused_<reason>` (the reason's hyphens written as underscores), each non-empty and distinct within its
locale.

R7. **iOS words a refusal in DeckStreak's words.** `CardRefusalWords.text(of:)` in `ios/App/Sources` gives each
`CardViewRefusal` case a DeckStreak sentence, and any other error one general sentence; `CardFaceView.swift` shows it
and holds no `String(describing:)`. `CardViewRefusal` is `CaseIterable`, so the words are checked for every case.

R8. **iOS announces it once.** When the card face shows a refusal, it posts one VoiceOver announcement whose text is
the words shown.

R9. **What still holds.** The refused card's controls stay (answer, bury, flag), the review's existing refusal tests
pass with the reason in place of the word, and the rendered audit runs axe over the review with a refused card in both
of Telegram's palettes.

## 3. Acceptance criteria of SPEC-505

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1, R2, R4: each check's plant gets its own reason, the seven are the closed list, a clean card is kept (examined 7), and the refused frame's mark is the reason | every plant reads `escaped`, and so does the mark | ADR-523 D1, D2 |
| A2 | R2, R5, R6: the review's status for each plant reachable through the screen is its reason's message, equal to the frame's mark, in every locale; the seven reasons' keys are in all seven locale files | the status is the one sentence, the mark `escaped`, and the keys are absent | ADR-523 D2, D3 |
| A3 | R5: no plant's unique token reaches a status, the frame's attributes or a console line, and the detector catches a planted leak | the status names no reason | ADR-523 D4 |
| A4 | R7, R8: every `CardViewRefusal` case reads a DeckStreak sentence, never its case name, `CardFaceView.swift` holds no `String(describing:)`, and the refusal is announced once, in the words shown | the stub mapper prints the case and announces nothing | ADR-523 D5 |
| A5 | R9: the review's refusal tests pass with the reason | they expect `escaped` and the one sentence | ADR-523 D2 |
| A6 | R9: axe finds no WCAG 2.2 AA violation on the review with a refused card, in both palettes | the review's status names no reason | ADR-523 D6 |

```acceptance
A1: pnpm --dir web/app exec vitest run src/lib/card/frame-document.test.ts -t "each frame check refuses with its own reason, the first failing check naming it"
A1: pnpm --dir web/app exec vitest run src/lib/card/frame-host.test.ts -t "a host that cannot carry the card document or its title byte for byte is refused"
A1: pnpm --dir web/app exec vitest run src/lib/card/card-frame.test.ts -t "a card the frame document refuses renders an empty frame marked with its reason"
A2: pnpm --dir web/app exec vitest run src/lib/study/review-screen.test.ts -t "each card the frame refuses names its reason in the status and the mark, in every locale"
A2: pnpm --dir web/app exec vitest run src/lib/study/refusal.test.ts -t "each refusal names its own message in every locale"
A3: pnpm --dir web/app exec vitest run src/lib/study/review-screen.test.ts -t "a refused card's status, mark and logs hold none of its markup or CSS"
A4: pnpm --dir web/app exec vitest run src/lib/study/refusal.test.ts -t "the iPhone and iPad card face words a refusal through its mapper, never a Swift case name"
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -only-testing:AppTests/CardRefusalWordsTests
A5: pnpm --dir web/app exec vitest run src/lib/study/review-screen.test.ts -t "a refusal, a card the frame refuses and a done deck are announced"
A5: pnpm --dir web/app exec vitest run src/lib/study/review-templates.test.ts -t "the frame's document check refuses none of the template cards"
A5: pnpm --dir web/app exec vitest run src/lib/study/review.test.ts
A6: pnpm --dir web/app exec playwright test tests/a11y.spec.ts -g "/study/review has no WCAG 2.2 AA violation axe can find"
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/src/lib/card/frame-document.ts` | card frame | changed: `REFUSAL_REASONS`, `RefusalReason`, each check's reason |
| `web/app/src/lib/card/frame-host.ts` | card frame | changed: `host-readback`; `frameCard`, the one composition |
| `web/app/src/lib/card/CardFrame.svelte` | card frame | changed: renders `frameCard`'s result |
| `web/app/src/lib/study/review.ts` | study | changed: `refused` reads `frameCard`; `Status` holds the reasons |
| `web/app/src/lib/study/refusal.ts` | study | changed: the seven reasons' keys |
| `web/app/messages/{en,es,fr,ja,ko,zh-Hans,zh-Hant}.json` | messages | changed: seven keys replace `study_card_escaped` |
| `ios/App/Sources/CardRefusalWords.swift` | iOS app | new: the words and the announcement |
| `ios/App/Sources/CardFaceView.swift` | iOS app | changed: shows and announces the words |
| `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | iOS card view | changed: `CardViewRefusal` is `CaseIterable` (one line, no line moves) |
| `ios/swift-roles.json` | iOS register | changed: the new file's role and decisions |
| `web/app/src/lib/card/frame-document.test.ts`, `frame-host.test.ts`, `card-frame.test.ts` | tests | A1 |
| `web/app/src/lib/study/review-screen.test.ts`, `review-templates.test.ts`, `review.test.ts`, `refusal.test.ts` | tests | A2 to A5 |
| `web/app/tests/a11y.spec.ts` | tests | A6: a stand-in engine shows a refused card on the review |
| `ios/AppTests/CardRefusalWordsTests.swift` | tests | new: A4 |
| `docs/specs/SPEC-505-...md`, `docs/decisions/ADR-523-...md`, the schematic, `docs/red-first/SPEC-505.md`, `changelog.d/refused-card-says-why-505.md` | docs | new |

## 5. What this does NOT cover

- Linking the next step to the note editor: the words name the step; a link waits for an editor that saves.
- Any change to what a check admits or refuses: every check keeps its rule; only its answer changes.
- New `CardViewRefusal` cases on iOS, or the web's seven reasons on iOS, whose factory makes no such checks.
- Translating iOS: the app has no string catalog yet, so its sentences are English, as every other iOS string is.

## 6. Risks

- **The status and the mark disagree.** Both read `frameCard`; A2 renders each reachable plant and compares them.
- **A refusal shows the card.** The words are fixed messages with no placeholder; A3 plants a unique token in each
  refused card and reads every status, attribute and console line for it, with a planted leak as its control.
- **A reason no locale words.** A2's census reads every key in every locale file.
- **A plant that never reaches its check.** A1's clean card is kept, and the seven reasons are seven distinct values.

## 7. What only CI and a Mac prove

The Playwright audit (A6) runs where its browsers' system libraries are installed; the iOS tests (A4's XCTest)
run on a Mac's simulators. VoiceOver's reading of the announcement on a device is a person's check.

## 8. Formal model

- **Lean: NOT APPLICABLE.** The reason decides no gate verdict, ledger, money or release; it names a refusal the
  checks already make.
- **TLA+: NOT APPLICABLE.** Nothing interleaves: the reason is a pure function of one card.

## 9. Mutation rows

None. Stryker's generated mutants of `frame-document.ts`, `frame-host.ts`, `review.ts` and `refusal.ts` are killed by
A1 to A3 and A5; the band S50500-S50599 holds no hand row.

# SPEC-380: both reviews withhold an image occlusion question whose masks are not drawn

- **Issue:** `#750`. An image occlusion question hides parts of its image behind masks drawn when
  the card is shown; neither review draws them, so the question would show its own answers. Both
  reviews refuse to show such a question, say so in one line, and record no rating for it.
- **Context(s):** `engine-core` (`crates/engine-core`), `web-engine` (`crates/web-engine`), `ffi`
  (`crates/ffi`), `miniapp` (`web/app/src/lib/engine`, `web/app/src/lib/study`, `web/app/messages`),
  `ios-app` (`ios/App/Sources`, `ios/AppTests`, `ios/swift-roles.json`).
- **Decided by:** ADR-391 (this SPEC's own). It works under ADR-356 D4 (the engine core depends on
  the engine and on no crate of this workspace) and ADR-352 D1 (no card script runs, on either
  platform), both of which it keeps.
- **Schematic:** `docs/schematics/web-study-screens.md`, amended by one section appended at its end
  (section 7, the withheld question from the engine to both reviews), and
  `docs/schematics/ios-review-screen.md`, amended by one section appended at its end (section 6,
  the native review's withheld phase). No existing line of either changes.
- **Status:** one delivery. **Mutation band:** `S38000-S38099` (section 9). **Model:** none
  (section 8).

## 1. The problem, measured

Every `path:line` below is read at `e7ecf10d6b796eb1f86fe6544e04a96a0583c791` (`dev`).

### 1a. Masks are drawn by a script no review runs

- The engine's image occlusion note type draws its masks with a script in the card's template: the
  rendered question carries the image, an empty mask layer and a call that paints the shapes onto
  that layer. The engine's source is not in this tree, so the marker names the rule reads are
  measured by the build on a note the engine itself builds (R1, A4).
- The web review runs no card script: the card is the source document of an empty sandboxed frame
  (ADR-352 D1 and D2). `web/app/src/lib/card/frame-document.ts` keeps a script element in the
  document, and the sandbox stops it running.
- The native review runs no card script: the card scripts switch defaults off
  (`ios/CardIsolation/Sources/CardIsolation/CardScripts.swift:7`), and the native face document
  names no script (`crates/ffi/src/face.rs:69-81`).
- Neither review loads the engine's own reviewer scripts. A case-insensitive search for
  `occlusion` over the tree finds one file, `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md:159`,
  which places image occlusion in a later parity phase. So today no review of this workspace draws
  an occlusion mask, and an occlusion question reaches the learner with its answers visible.

### 1b. What reaches each review today

- The engine core's face renders the card (`crates/engine-core/src/face.rs:167-224`, the render at
  `:176-181`, the question at `:183` and `:187`) and answers a `Face` of text, note CSS, autoplay
  and replay clips and omitted references (`:77-90`).
- The web engine builds the shown card's view in `current_card`
  (`crates/web-engine/src/wasm.rs:634-710`): a full render, the question and answer, the note CSS
  and `late` (`:688-701`); its `faces` answer the core's faces (`:821-861`). The page types the view
  as `CardView` (`web/app/src/lib/engine/protocol.ts:123-133`).
- The ffi answers the native review a `CardFace` of a document and its clips
  (`crates/ffi/src/face.rs:43-53`, built at `:98-108`), through `face`
  (`crates/ffi/src/engine.rs:155-168`).
- The web review's phases and their actions are one table (`web/app/src/lib/study/review.ts:16`,
  `:71-100`); a status maps to one line (`web/app/src/lib/study/refusal.ts`), shown in the status
  region above the card frame (`web/app/src/lib/study/ReviewScreen.svelte`).
- The native review session sets the head card once its face is answered and returns the question
  phase (`ios/App/Sources/ReviewSession.swift:60-63`); the model's `perform` admits show-answer on
  the question, a rating on the answer, and bury and flag on either
  (`ios/App/Sources/ReviewModel.swift:68-102`); the bar disables Show Answer outside the question
  and every rating outside the answer (`ios/App/Sources/AnswerBar.swift:33`, `:58`).

### 1c. The native Swift has budgets

`scripts/tests/test_ios_thin_swift.py` counts each counted Swift file's decisions (`:110-115`) and
holds them equal to `ios/swift-roles.json` and within the role's ceiling (`:38-48`). At the base:

| file | role | decisions | ceiling |
|---|---|---|---|
| `ios/App/Sources/EngineSession.swift` | session | 4 | 4 |
| `ios/App/Sources/ReviewChrome.swift` | view | 3 | 3 |
| `ios/App/Sources/ReviewModel.swift` | model | 6 | 6 |
| `ios/App/Sources/ReviewSession.swift` | session | 3 | 4 |
| `ios/App/Sources/ReviewView.swift` | view | 3 | 3 |

So the native line cannot be a new branch in the review view, and the withheld phase can add one
decision, in the session.

### 1d. The review fixture

The native review's tests open a copy of the review fixture and choose its deck by name
(`ios/AppTests/ReviewSessionTests.swift:11-31`); the fixture's own test holds four cards and two
media files (`crates/ffi/tests/fixture.rs`, `the_review_fixture_holds_four_cards_and_two_files`).

## 2. Requirements

R1. The engine core holds one rule, `occlusion::masks_not_drawn(question)`: true when the rendered
question holds the engine's image occlusion mask layer or an image occlusion shape, by the marker
names the build measures on a note the engine builds; false for every other question. Its doc
comment names each marker and the note it was measured on.

R2. The core's face withholds a question R1 marks, on both sides: `Face.withheld` is true, and its
text, note CSS, autoplay clips, replay clips and omitted references are empty. Every other face is
unchanged and carries `withheld` false.

R3. The web engine's card view carries `withheld`, from R1 over the question `current_card`
renders; when it is true, the view's question, answer and note CSS are empty.

R4. The ffi's card face carries `withheld` from the core's face. A withheld face's document is a
closed page holding the English line of R6, and no note text, no note CSS and no script.

R5. The web review shows a withheld card with no card frame and no late line, and with the line of
R6 in the status region where the card's lines are shown. Its actions are the question side's less
show-answer and replay: undo as the question side offers it, bury and flag. No key, button,
remote or stick sends show-answer or a rating for it.

R6. The web line is `study_card_withheld`, in each of the 7 locales of `web/app/messages`. In `en`
it reads: "This image occlusion card cannot be shown here, because this app does not draw its
masks. You can still bury or flag it." Each other locale's line is written in that locale's own
words, holds the locale's own `study_bury` word, and is not the English text.

R7. The native review shows a withheld card as the ffi's withheld document, in the card's place, in
a phase `withheld` in which Show Answer and every rating are disabled and do nothing, and in which
bury and flag work as they do on the question. The native line is the English line of R6.

R8. No rating is recorded for a withheld card on either review. It stays due until the learner
buries it, and a bury moves the review on to the next card or to its designed end.

R9. The native Swift stays within every role's ceiling: `ReviewSession.swift` gains one decision
(3 to 4), every other counted Swift file keeps its count, and `ios/swift-roles.json` records it.

R10. The review fixture writes a second collection in a directory beside its first, holding one
deck with one image occlusion note built through the engine's own interface from fixed inputs; the
first collection is unchanged.

R11. A source census holds R3 in the wasm32-only view, and a locale census holds R6; each reports
what it examined, refuses zero, and refuses each planted break by name.

## 3. Acceptance criteria of the withheld occlusion question

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | A question holding the mask layer and no shape is marked | the stub rule answers false | `crates/engine-core/tests/occlusion.rs` `a_question_holding_the_mask_layer_is_withheld` (added) |
| A2 | A question holding an occlusion shape and no mask layer is marked | the stub rule answers false | `occlusion.rs` `a_question_holding_an_occlusion_shape_is_withheld` (added) |
| A3 | A plain question, a text cloze, an image, and the word occlusion in text are not marked | not red: the stub answers false; it guards the arms | `occlusion.rs` `a_plain_or_text_cloze_question_is_not_withheld` (added) |
| A4 | The core's face of a note the engine builds is withheld on the question and the answer, with no text, CSS or clips | the stub face's `withheld` is false and its text holds the image | `occlusion.rs` `the_engines_occlusion_card_is_withheld_on_both_sides` (added) |
| A5 | A basic card and a cloze card keep their faces, with `withheld` false | not red: the base shows them; it guards the unchanged faces | `occlusion.rs` `a_basic_and_a_cloze_card_keep_their_faces` (added) |
| A6 | The web engine's card view carries `withheld` from the core's rule, and empties the question, answer and CSS on it; each plant is refused by name | the census finds no `withheld` key in `current_card` | `crates/web-engine/tests/occlusion_view.rs` `the_shown_card_carries_the_cores_withheld_answer` (added) |
| A7 | The ffi face of the fixture's occlusion card is withheld on both sides, its document holds the English line and no image, and it carries no clip | the stub's `withheld` is false and the document holds the image | `crates/ffi/tests/occlusion.rs` `an_occlusion_card_face_is_withheld_with_the_line` (added) |
| A8 | The ffi face of the review fixture's text card is not withheld, and its document is unchanged | not red: the base shows it; it guards the unchanged face | `crates/ffi/tests/occlusion.rs` `a_review_card_face_is_not_withheld` (added) |
| A9 | A withheld card shows the English line, no card frame and no late line | the page draws the card frame and no line | `web/app/src/lib/study/occlusion-guard.test.ts` "a withheld card shows the line and no card" (added) |
| A10 | A withheld card offers undo, bury and flag only, and no key or button sends show-answer or a rating | show-answer is offered on the question side | `occlusion-guard.test.ts` "a withheld card offers no reveal and no grade" (added) |
| A11 | Burying a withheld card moves on to the next card and sends no rating | not red: the base buries a shown question the same way; it guards the way on | `occlusion-guard.test.ts` "burying a withheld card moves on and grades nothing" (added) |
| A12 | Every locale holds `study_card_withheld` with its own bury word, and no locale but `en` holds the English text; each plant is refused by name | all 7 locales lack the key | `occlusion-guard.test.ts` "every locale holds the withheld line" (added) |
| A13 | The native session answers the occlusion card in the `withheld` phase, with the withheld face | the session answers the question phase | `ios/AppTests/ReviewSessionTests.swift` `test_an_occlusion_card_is_withheld` (added) |
| A14 | On a withheld card the native model's Show Answer and every rating change nothing and send nothing, and a bury moves on | Show Answer reveals the answer at the base | `ios/AppTests/ReviewModelTests.swift` `test_a_withheld_card_reveals_nothing_and_takes_no_rating` (added) |
| A15 | Every counted Swift file keeps its role, its doors and a budget within its ceiling | not red: the base keeps every budget; it guards R9 | `scripts/tests/test_ios_thin_swift.py` `test_every_swift_file_keeps_its_role_its_doors_and_its_budget` (named) |

```acceptance
A1: cargo test -p deck-streak-engine-core --test occlusion -- --exact a_question_holding_the_mask_layer_is_withheld
A2: cargo test -p deck-streak-engine-core --test occlusion -- --exact a_question_holding_an_occlusion_shape_is_withheld
A3: cargo test -p deck-streak-engine-core --test occlusion -- --exact a_plain_or_text_cloze_question_is_not_withheld
A4: cargo test -p deck-streak-engine-core --test occlusion -- --exact the_engines_occlusion_card_is_withheld_on_both_sides
A5: cargo test -p deck-streak-engine-core --test occlusion -- --exact a_basic_and_a_cloze_card_keep_their_faces
A6: cargo test -p deck-streak-web-engine --test occlusion_view -- --exact the_shown_card_carries_the_cores_withheld_answer
A7: cargo test -p deck-streak-ffi --test occlusion -- --exact an_occlusion_card_face_is_withheld_with_the_line
A8: cargo test -p deck-streak-ffi --test occlusion -- --exact a_review_card_face_is_not_withheld
A9: pnpm exec vitest run web/app/src/lib/study/occlusion-guard.test.ts -t "a withheld card shows the line and no card"
A10: pnpm exec vitest run web/app/src/lib/study/occlusion-guard.test.ts -t "a withheld card offers no reveal and no grade"
A11: pnpm exec vitest run web/app/src/lib/study/occlusion-guard.test.ts -t "burying a withheld card moves on and grades nothing"
A12: pnpm exec vitest run web/app/src/lib/study/occlusion-guard.test.ts -t "every locale holds the withheld line"
A13: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_an_occlusion_card_is_withheld
A14: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a_withheld_card_reveals_nothing_and_takes_no_rating
A15: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py -k test_every_swift_file_keeps_its_role_its_doors_and_its_budget
```

A1's and A2's questions are literal text in the test, copied from the render of A4's note as the
build measured it, with the layer alone in A1 and a shape alone in A2; no expected value is
computed by `masks_not_drawn`. A4's note, A7's fixture note and A13's are built through the
engine's own interface from fixed inputs (a fixed image's bytes, fixed shapes and a fixed header),
so the render the literals were copied from is the render every run reads. A7 compares the
document's line with a literal in the test, never with the ffi's constant. The censuses of A6 and
A12 report what they examined and refuse zero, and each carries planted refusals. A13 and A14 run
on the continuous integration's simulator step; their reds and greens are read there by name.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-380-both-reviews-withhold-an-image-occlusion-question-whose-masks-are-not-drawn.md` | docs | added |
| `docs/decisions/ADR-391-the-engine-core-marks-an-image-occlusion-question-and-each-review-withholds-it-with-one-line.md` | docs | added |
| `docs/schematics/web-study-screens.md` | docs | one section appended at its end (section 7); no existing line changes |
| `docs/schematics/ios-review-screen.md` | docs | one section appended at its end (section 6); no existing line changes |
| `docs/red-first/SPEC-380.md` | docs | added |
| `changelog.d/occlusion-mask-guard-380.md` | docs | added |
| `scripts/mutation-rows.d/S38000-S38099.json` | docs | added (section 9) |
| `crates/engine-core/src/occlusion.rs` | `engine-core` | added: the rule (R1) |
| `crates/engine-core/src/lib.rs` | `engine-core` | `pub mod occlusion;` and its line in the module list |
| `crates/engine-core/src/face.rs` | `engine-core` | `Face.withheld` and the withholding in `complete` (R2) |
| `crates/engine-core/tests/occlusion.rs` | `engine-core` | added: A1 to A5 |
| `crates/web-engine/src/wasm.rs` | `web-engine` | `current_card`'s view carries `withheld` (R3) |
| `crates/web-engine/tests/occlusion_view.rs` | `web-engine` | added: A6 |
| `crates/ffi/src/face.rs` | `ffi` | `CardFace.withheld` and the withheld document (R4) |
| `crates/ffi/tests/support/occlusion.rs` | `ffi` | added: the second collection's builder (R10) |
| `crates/ffi/examples/review-fixture.rs` | `ffi` | writes the second collection beside the first (R10) |
| `crates/ffi/tests/occlusion.rs` | `ffi` | added: A7, A8 |
| `web/app/src/lib/engine/protocol.ts` | `miniapp` | `CardView.withheld` (R3) |
| `web/app/src/lib/study/review.ts` | `miniapp` | the withheld phase, its actions and its status (R5, R8) |
| `web/app/src/lib/study/refusal.ts` | `miniapp` | the withheld status's line key (R6) |
| `web/app/src/lib/study/ReviewScreen.svelte` | `miniapp` | no frame and no late line on a withheld card (R5) |
| `web/app/messages/en.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/messages/es.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/messages/fr.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/messages/ja.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/messages/ko.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/messages/zh-Hans.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/messages/zh-Hant.json` | `miniapp` | `study_card_withheld` (R6) |
| `web/app/src/lib/study/occlusion-guard.test.ts` | `miniapp` | added: A9 to A12 |
| `web/app/src/lib/study/refusal.test.ts` | `miniapp` | `withheld: 'study_card_withheld'` in its written-out status register (R6); no other change |
| `web/app/src/lib/study/audio.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/late-line.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review-screen.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review-templates.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/voice.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `web/app/src/routes/study.test.ts` | `miniapp` | `withheld: false` in its card view fixtures; no assertion changes |
| `ios/App/Sources/EngineSession.swift` | `ios-app` | `ReviewFace.withheld`, from the ffi's face; no new decision |
| `ios/App/Sources/ReviewSession.swift` | `ios-app` | the withheld phase from the face (R7); one decision |
| `ios/App/Sources/ReviewView.swift` | `ios-app` | `withheld` in the phase's one case list; no new decision |
| `ios/App/Sources/ReviewModel.swift` | `ios-app` | bury and flag admit the withheld phase, in their existing patterns; no new decision |
| `ios/App/Sources/ReviewChrome.swift` | `ios-app` | bury and flag idle on the withheld phase; one decision replaced by one |
| `ios/swift-roles.json` | `ios-app` | `ReviewSession.swift` records 4 decisions (R9) |
| `ios/AppTests/ReviewSessionTests.swift` | `ios-app` | A13 |
| `ios/AppTests/ReviewModelTests.swift` | `ios-app` | A14 |

The seven web fixture files are those whose card view literals carry `late` at the base, which the
type check refuses once `withheld` is required. `ios/App/Sources/AnswerBar.swift` is not changed:
it already disables Show Answer and every rating outside their own phases.

## 5. What this does NOT cover

- It draws no mask: a review that paints the masks, so an occlusion card can be studied, is the
  image occlusion parity phase that SPEC-334 places behind #611.
- It does not see the older occlusion notes whose masks are separate image files and whose
  template holds neither marker; a mask image the media rules omit leaves such a question unmasked
  (#611).
- It does not see a note type edited to drop both the mask layer and every shape; such a question
  shows as the text it renders, the way any card without a script does (#611).
- It adds no localized native line: the native app holds no localized strings, so the native line
  is English, as every native string is; a line in every locale there needs the native string
  catalog first (#738).
- It writes no Swift mutation row: no mutation table takes Swift at the base, and the Swift arms
  are held by A13, A14 and A15 (#650).
- It changes no harness app: `ios/Harness` shows the ffi's withheld document but keeps its own
  answer controls, because it is the ffi's test harness and not a learner's review (#616).
- It proves nothing on a physical device: the line on a device is the first device session's
  (#629).

## 6. Risks

- **The markers are not the engine's.** A rule written from a belief about the template would mark
  nothing. Detected by A4: the core's face of a note the engine builds must be withheld, and A1 and
  A2 copy their questions from that render.
- **A shown surface outside the review.** A card's question shown anywhere else would show the
  answers. At the base the web page fetches a card's view and faces only in the review
  (`web/app/src/lib/study/review.ts:338-350`); the build re-measures every caller of `card(` and
  `faces(` under `web/app/src` and every caller of `face(` under `ios/App/Sources` at its cut.
- **A rating path that bypasses the table.** Detected by A10 and A14: every key, button and model
  action is tried on a withheld card, and the engine's due counts are read after.
- **A new field breaks a literal.** `CardFace` and `ReviewFace` gain a field: a Swift literal of
  either outside the manifest would not compile. The build re-measures them at its cut.
- **The fixture change reaches an existing test.** Detected by the whole ffi suite and the native
  review tests, which still choose their deck by name in the unchanged first collection.
- **A Swift budget is exceeded.** Detected by A15 on every run.

## 7. What only CI or a device proves

| # | what | who, and the record |
|---|---|---|
| C1 | A13 and A14, red on the first push and green on the second, on the simulator step of the native workflow, on both simulators | the continuous integration, read by job and step name; the run ids go in the red-first record |
| C2 | The wasm32 build of `current_card` compiles and the web build ships it | the continuous integration's web job |
| C3 | Every changed web production file's mutants are killed | the `mutation-web` job, at a break of 100 |
| V1 | The native line on a device, in a review of a deck holding an occlusion card | the owner, in the first device session (#629) |

## 8. Formal model

None, by surface. The rule is a total function over one string, computed inside one synchronous
call; the withheld phase is one more arm in two transition tables (`review.ts` and
`ReviewModel.swift`), with no new actor, shared state, timer or write path. No covered span is
edited: of the formal covers at the base, three name `crates/web-engine/src/wasm.rs` (its undo, its
undo offer and its rating call) and one names `crates/ffi/src/engine.rs`; this delivery edits
`current_card` alone in the first and does not edit the second. A transition table in TypeScript or
Swift cannot be named by a cover.

## 9. Mutation rows

The band `S38000-S38099` takes these rows, each in the row's own crate with its killer there. Each
find is written after `cargo fmt` and occurs exactly once in its file.

| stem | crate | file | mutant | killer |
|---|---|---|---|---|
| `S38000-OCCLUSION-LAYER-ARM-DROPPED` | `engine-core` | `src/occlusion.rs` | the mask layer's arm answers false | `occlusion::a_question_holding_the_mask_layer_is_withheld` |
| `S38001-OCCLUSION-SHAPE-ARM-DROPPED` | `engine-core` | `src/occlusion.rs` | the shape's arm answers false | `occlusion::a_question_holding_an_occlusion_shape_is_withheld` |
| `S38002-OCCLUSION-EITHER-BECOMES-BOTH` | `engine-core` | `src/occlusion.rs` | the two arms joined by both instead of either | `occlusion::a_question_holding_the_mask_layer_is_withheld` |
| `S38003-OCCLUSION-LAYER-NAME` | `engine-core` | `src/occlusion.rs` | the mask layer's marker literal changed | `occlusion::a_question_holding_the_mask_layer_is_withheld` |
| `S38004-OCCLUSION-SHAPE-NAME` | `engine-core` | `src/occlusion.rs` | the shape's marker literal changed | `occlusion::a_question_holding_an_occlusion_shape_is_withheld` |
| `S38005-FACE-WITHHELD-NEVER` | `engine-core` | `src/face.rs` | the face's `withheld` set false | `occlusion::the_engines_occlusion_card_is_withheld_on_both_sides` |
| `S38006-FACE-TEXT-KEPT` | `engine-core` | `src/face.rs` | a withheld face keeps its text | `occlusion::the_engines_occlusion_card_is_withheld_on_both_sides` |
| `S38007-FACE-CLIPS-KEPT` | `engine-core` | `src/face.rs` | a withheld face keeps its clips | `occlusion::the_engines_occlusion_card_is_withheld_on_both_sides` |
| `S38008-FACE-CSS-KEPT` | `engine-core` | `src/face.rs` | a withheld face keeps its note CSS | `occlusion::the_engines_occlusion_card_is_withheld_on_both_sides` |
| `S38009-FFI-WITHHELD-NEVER` | `ffi` | `src/face.rs` | the card face's `withheld` set false | `occlusion::an_occlusion_card_face_is_withheld_with_the_line` |
| `S38010-FFI-LINE-DROPPED` | `ffi` | `src/face.rs` | a withheld document wraps the face's text instead of the line | `occlusion::an_occlusion_card_face_is_withheld_with_the_line` |
| `S38011-FFI-LINE-TEXT` | `ffi` | `src/face.rs` | the line's literal changed | `occlusion::an_occlusion_card_face_is_withheld_with_the_line` |
| `S38012-WASM-WITHHELD-NEVER` | `web-engine` | `src/wasm.rs` | the view's `withheld` set false | `occlusion_view::the_shown_card_carries_the_cores_withheld_answer` |
| `S38013-WASM-QUESTION-KEPT` | `web-engine` | `src/wasm.rs` | a withheld view keeps its question | `occlusion_view::the_shown_card_carries_the_cores_withheld_answer` |

`src/wasm.rs`'s view is compiled for wasm32 alone, so its two rows are killed by the source census,
which reads the file's text. The web's changed production files are mutated by StrykerJS on the
pull request at a break of 100, and take no hand row. No Swift row is written (section 5).

# Red-first record: SPEC-380

The SPEC, ADR-391 and the two schematics' sections were committed first (d8ada1d1), before any test
or code. The two censuses, A6 and A12, were then committed alone (35e22a6f). The criteria's tests,
with the stubs that keep every input, followed in two commits: the engine core's and the ffi's
(5011b283), then the web review's and the native review's, which this record entered with. Each
red below is quoted from the run at the commit it names. A line cannot name the commit that writes
it, so the reds of the second test commit, A9, A10, A13 and A14, are written by the commit after
it; A13 and A14 are read on the continuous integration's simulator step, by run, job and step.

## The fence, line by line

Each of the 15 lines of SPEC-380 section 3's fence resolves to a test this delivery adds, or, for
A15, names.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/occlusion.rs` `a_question_holding_the_mask_layer_is_withheld` | added (step 3, core and ffi) |
| 2 | A2 | `crates/engine-core/tests/occlusion.rs` `a_question_holding_an_occlusion_shape_is_withheld` | added (step 3, core and ffi) |
| 3 | A3 | `crates/engine-core/tests/occlusion.rs` `a_plain_or_text_cloze_question_is_not_withheld` | added (step 3, core and ffi) |
| 4 | A4 | `crates/engine-core/tests/occlusion.rs` `the_engines_occlusion_card_is_withheld_on_both_sides` | added (step 3, core and ffi) |
| 5 | A5 | `crates/engine-core/tests/occlusion.rs` `a_basic_and_a_cloze_card_keep_their_faces` | added (step 3, core and ffi) |
| 6 | A6 | `crates/web-engine/tests/occlusion_view.rs` `the_shown_card_carries_the_cores_withheld_answer` | added (step 2) |
| 7 | A7 | `crates/ffi/tests/occlusion.rs` `an_occlusion_card_face_is_withheld_with_the_line` | added (step 3, core and ffi) |
| 8 | A8 | `crates/ffi/tests/occlusion.rs` `a_review_card_face_is_not_withheld` | added (step 3, core and ffi) |
| 9 | A9 | `web/app/src/lib/study/occlusion-guard.test.ts` "a withheld card shows the line and no card" | added (step 3, web and native) |
| 10 | A10 | `web/app/src/lib/study/occlusion-guard.test.ts` "a withheld card offers no reveal and no grade" | added (step 3, web and native) |
| 11 | A11 | `web/app/src/lib/study/occlusion-guard.test.ts` "burying a withheld card moves on and grades nothing" | added (step 3, web and native) |
| 12 | A12 | `web/app/src/lib/study/occlusion-guard.test.ts` "every locale holds the withheld line" | added (step 2) |
| 13 | A13 | `ios/AppTests/ReviewSessionTests.swift` `test_an_occlusion_card_is_withheld` | added (step 3, web and native) |
| 14 | A14 | `ios/AppTests/ReviewModelTests.swift` `test_a_withheld_card_reveals_nothing_and_takes_no_rating` | added (step 3, web and native) |
| 15 | A15 | `scripts/tests/test_ios_thin_swift.py` `test_every_swift_file_keeps_its_role_its_doors_and_its_budget` | named |

## The reds and greens

Each line's command is the criterion's line in SPEC-380 section 3's fence, run at the commit named.

```red-first
A1: red at 5011b283: panicked at crates/engine-core/tests/occlusion.rs:228:5: a question holding the engine's mask layer and no shape is not marked: <div id="image-occlusion-container"> <img src="data:image/png;base64,iVBORw0KGgo="> <canvas id="image-occlusion-canvas"></canvas> </div>
A2: red at 5011b283: panicked at crates/engine-core/tests/occlusion.rs:236:5: a question holding an occlusion shape and no mask layer is not marked: <div style="display: none"><div class="cloze" data-ordinal="1" data-shape="rect" data-left=".2" data-top=".3" data-width=".4" data-height=".1" ></div></div>
A3: not red: the stub answers false; it guards the arms against a plain question, a text cloze, an image and the word occlusion in text
A4: red at 5011b283: panicked at crates/engine-core/tests/occlusion.rs:269:9: assertion `left == right` failed: the engine's image occlusion card is not withheld on its Question side; left: Face { text: "<div></div> ... <canvas id=\"image-occlusion-canvas\"></canvas> ...", css: "#image-occlusion-canvas { ... }", autoplay: [Speech { text: "the parts of a cell", language: "en-US", rate: 0.5 }], replay: [Speech { text: "the parts of a cell", language: "en-US", rate: 0.5 }], omitted: [], withheld: false }, right: Face { text: "", css: "", autoplay: [], replay: [], omitted: [], withheld: true }
A5: not red: the base shows a basic and a cloze card; it guards the unchanged faces
A6: red at 35e22a6f: panicked at crates/web-engine/tests/occlusion_view.rs:174:5: assertion `left == right` failed: the shown card's view does not carry the core's withheld answer, or keeps a withheld text; left: ["current_card's view holds 0 `withheld` key(s), not one", ... seven problems in all], right: []
A7: red at 5011b283: panicked at crates/ffi/tests/occlusion.rs:116:9: assertion `left == right` failed: the occlusion card's face (answer: false) is not withheld with the line; left: CardFace { document: "<!DOCTYPE html>... <img src=\"data:image/png;base64,iVBORw0KGgo=\"> <canvas id=\"image-occlusion-canvas\"></canvas> ...", autoplay: [Speech { text: "the parts of a cell", ... }], ..., withheld: false }, right: CardFace { document: "<!DOCTYPE html>...<body class=\"card\">This image occlusion card cannot be shown here, because this app does not draw its masks. You can still bury or flag it.</body></html>", autoplay: [], replay: [], omitted: [], withheld: true }
A8: not red: the base shows the review fixture's text card; it guards the unchanged face
A11: not red: the base buries a shown question the same way; it guards the way on
A12: red at 35e22a6f: AssertionError: expected { …(7) } to deeply equal { Object (en, es, ...) }: en, es, fr, ja, ko, zh-Hans and zh-Hant each read "<locale> lacks study_card_withheld" (occlusion-guard.test.ts:61)
A15: not red: the base keeps every Swift budget; it guards R9
```

## What the record discloses

- **Step 3 is two commits.** The engine core's and the ffi's tests and stubs are 5011b283; the web
  review's and the native review's are the commit this record entered with. Each red is named at
  the commit whose tree it was measured on.
- **A1, A2, A4 and A7 are quoted in short.** Each run printed the rendered question or document in
  full, over several lines; the record keeps the failing line, the markers and the field that
  differs. A6's left side names seven problems; the record quotes the first.
- **A6 and A12 were read again on the second test commit's tree.** `crates/web-engine/src/wasm.rs`
  is unchanged since 35e22a6f, and A6 reads the same failing line there; A12 reads each of the
  seven locales lacking `study_card_withheld` there too, at the test's moved line.
- **The stubs keep every input.** `occlusion::masks_not_drawn` takes the rendered question and
  answers false; the core's `Face.withheld` and the ffi's `CardFace.withheld` are carried and false;
  the page's `CardView.withheld` is typed, and `withheld: false` in the seven fixture files;
  `ReviewFace.withheld` is mapped from the ffi's face; and `withheld` joins the native phase's one
  case list. So each red is the missing behaviour, never a missing item.
- **The seven web fixture files gained `withheld: false` alone,** which the type check owes once the
  field is required; no assertion of theirs changed.
- **The native tests' fixture helper gained a deck name.** `openedFixture` in
  `ios/AppTests/ReviewSessionTests.swift` takes the deck's name, `Review` unless a test names
  another, so every existing caller opens the deck it opened before; A13 and A14 open the second
  collection's deck `Occlusion`.

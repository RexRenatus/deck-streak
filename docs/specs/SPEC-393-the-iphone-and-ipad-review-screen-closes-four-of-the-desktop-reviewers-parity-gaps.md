# SPEC-393: the iPhone and iPad review screen closes four of the desktop reviewer's parity gaps

- **Wave:** the app campaign, the review screen's parity (part of #632). **Issue:** #666 (Refs; one of its
  five gaps is cut to a follow-up, section 9). **Context(s):** `engine-core` (`crates/engine-core`), `ffi`
  (`crates/ffi`), the iPhone and iPad app (`ios/`), the `CardIsolation` package.
- **Decided by:** ADR-407 (this SPEC's own: D1 the class, D2 the fonts, D3 the voices, D4 inline video, D5
  the cut, D6 the containment, D7 the order, D8 the formal surface, D9 the reds on CI), ADR-359 (the review
  screen's media, face and voice choice, amended here), ADR-360 (the card view) and ADR-366 (card scripts
  stay off).
- **Schematic:** `docs/schematics/ios-review-screen.md` section 6 (this delivery appends it).
- **Status:** one pull request, three pushes (ADR-407 D7, D9). **Mutation band:** `S39300-S39399`.
  **Changelog:** `changelog.d/review-screen-parity-393.md`. **Red-first record:** `docs/red-first/SPEC-393.md`.

Refs read: `$R` is the DeckStreak repository and `DEV` is `164ac20690e0d6d0385474d04663d14df5781e19`.
Every figure below is read at DEV with the command beside it.

## 1. The problem, measured

### 1.1 What the engine already gives the client, gap by gap

| # | measured | figure | command |
|---|---|---|---|
| M1 | #666 lists five gaps: a TTS tag's `voices=` and other options, a sound tag naming a video, CSS fonts by `url()`, the `card<n>` class, an HTML video full screen on an iPhone | 5 gaps; SPEC-348's exclusions cite #666 for them | `gh issue view 666 -R $R --json title,body`; `git show DEV:docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md \| sed -n '331,333p'` |
| M2 | the face is `{text, css, autoplay, replay, omitted}`, with no ordinal | 5 fields | `git show DEV:crates/engine-core/src/face.rs \| sed -n '77,90p'` |
| M3 | the preset is read from the card record, which also holds the template index, unused | one engine call for the card | `git show DEV:crates/engine-core/src/face.rs \| sed -n '125,139p'` |
| M4 | the face's CSS is the rendered note type's CSS, as written | `css: rendered.css` | `git show DEV:crates/engine-core/src/face.rs \| sed -n '217,223p'` |
| M5 | the media rewrite runs over the text only, through the engine's `replace_media_refs` | the CSS is never read for media | `git show DEV:crates/engine-core/src/media.rs \| sed -n '112,117p'` |
| M6 | the closed type table holds images and audio, `mp4` read as audio, and no font | 16 rows | `git show DEV:crates/engine-core/src/media.rs \| sed -n '17,36p'` |
| M7 | a TTS tag becomes `Speech {text, language, rate}`: the speed is applied, the voices are dropped | rate from speed at line 145 | `git show DEV:crates/engine-core/src/face.rs \| sed -n '141,151p'` |
| M8 | the engine's TTS tag carries the voices | `voices` among its five fields (SPEC-348 M15) | `git show DEV:docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md \| sed -n '49p'` |
| M9 | the speed is pinned by a test | rate 0.75 for speed 1.5 | `git show DEV:crates/engine-core/tests/face.rs \| sed -n '448,468p'` |
| M10 | the web writes the template's class from its view's ordinal | `card card${face.view.ordinal + 1}` | `git show DEV:web/app/src/lib/study/ReviewScreen.svelte \| sed -n '200p'`; `git show DEV:crates/web-engine/src/wasm.rs \| sed -n '692p'` |
| M11 | the web renders its view's own CSS, and its reader turns each name it is asked for into a fetch | `css={face.view.css}`; `wanted` | `git show DEV:web/app/src/lib/study/ReviewScreen.svelte \| sed -n '198p'`; `git show DEV:crates/web-engine/src/wasm.rs \| sed -n '814,820p'` |
| M12 | the web's clip value binds the speech clip's three fields by name, with no rest pattern | adding a field breaks its build | `git show DEV:crates/web-engine/src/wasm.rs \| sed -n '884,888p'` |

### 1.2 What the native client does with it

| # | measured | figure | command |
|---|---|---|---|
| M13 | the native document's body class is `card` by day and `card nightMode night_mode` by night | 2 constants | `git show DEV:crates/ffi/src/face.rs \| sed -n '17,19p;69,81p'` |
| M14 | the closed-page test pins that class | `"card"` by day and `"card nightMode night_mode"` by night, asserted as `[classes]` | `git show DEV:crates/ffi/tests/face.rs \| sed -n '104,107p;165,169p'` |
| M15 | the ffi's speech clip carries text, language and rate | 3 fields | `git show DEV:crates/ffi/src/face.rs \| sed -n '22,40p;83,96p'` |
| M16 | the voice is the kept choice when installed, else none | `chosen` | `git show DEV:crates/ffi/src/voices.rs \| sed -n '112,121p'` |
| M17 | the picker's offer for a language: its own voices, else those sharing its primary subtag | `options` | `git show DEV:crates/ffi/src/voices.rs \| sed -n '129,144p'` |
| M18 | Swift asks `chosen` when it maps a speech clip | one call | `git show DEV:ios/App/Sources/EngineSession.swift \| sed -n '228,240p'` |
| M19 | every app file sits at its decision ceiling; only the `speech` role imports the media framework, only `card` imports WebKit | 8 files at ceiling | `git show DEV:ios/swift-roles.json`; `git show DEV:scripts/tests/test_ios_thin_swift.py \| sed -n '38,47p;74,89p'` |
| M20 | the factory's configuration sets no inline playback | no `allowsInlineMediaPlayback` in the file | `git show DEV:ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift \| grep -c allowsInlineMediaPlayback` |
| M21 | the platform's inline default is off on an iPhone and on on an iPad, and an iPhone also needs `playsinline` on the element | SPEC-348 P8 | `git show DEV:docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md \| grep -n allowsInlineMediaPlayback` |
| M22 | the isolation package also builds for its tests' host, where the iPhone-only guard already appears | `#if os(iOS)` at lines 2 and 62 | `git show DEV:ios/CardIsolation/Package.swift`; `git show DEV:ios/CardIsolation/Sources/CardIsolation/WindowRefusal.swift \| grep -n 'os(iOS)'` |
| M23 | the factory's test reads a made view's configuration on both simulators | one readback test | `git show DEV:ios/CardProbeTests/FactoryTests.swift \| sed -n '41,61p'` |
| M24 | the session tests copy the fixture into a fresh directory and open the deck named `Review`, passing the installed voices in | `freshFixture`, `openedFixture`, `installed:` | `git show DEV:ios/AppTests/ReviewSessionTests.swift \| sed -n '13,41p'` |
| M25 | the review fixture is built from bytes in the test and committed as no binary | 4 cards, 2 files | `git show DEV:crates/ffi/tests/support/review.rs`; `git show DEV:crates/ffi/examples/review-fixture.rs` |

### 1.3 The containment the delivery stays inside (#619)

| # | measured | figure | command |
|---|---|---|---|
| M26 | the rule list blocks every load | one rule, `.*` | `git show DEV:ios/CardIsolation/Sources/CardIsolation/RuleList.swift \| sed -n '13p'` |
| M27 | the document's policy admits fonts and media only as `data:` | `font-src data:`, `media-src data:` | `git show DEV:ios/CardIsolation/Sources/CardIsolation/DocumentPolicy.swift \| sed -n '13,15p'` |
| M28 | the planted suite renders a `data:` font, sound and image through the whole frame, and holds a remote font and remote media at the rule list | the `permitted` card; the `font` and `media` channels | `git show DEV:ios/CardProbeTests/Planted.swift \| sed -n '157,164p;547,586p'` |
| M29 | the document reaches the view only as a string with no base URL | L7 | `git show DEV:ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift \| sed -n '55,65p'` |

### 1.4 What each machine can prove

The box runs the Rust tests of `deck-streak-engine-core`, `deck-streak-ffi` and `deck-streak-web-engine`
and the Python censuses. No Swift runs on the box: A14 and A17 run in the native workflow's `harness`
job, in the review screen's step and the card view's planted-suite step, each on the iPhone and then the
iPad simulator (`git show DEV:.github/workflows/xcframework.yml`). That workflow runs on every update of
a pull request into dev whose whole diff touches `crates/ffi`, `crates/engine-core` or `ios`, so a
docs-only push runs it too (`git show DEV:.github/workflows/apple-on-change.yml`).

## 2. Requirements

R1. The face carries `ordinal`, the card's template index, read from the card record the preset is read
from, with no second engine call for the card.

R2. The native document's body class is `card card<n>`, `<n>` the ordinal plus one, followed by
` nightMode night_mode` when the document is asked for night. The constant line that picks the night
classes keeps its text, or its row is re-anchored under its own id.

R3. The core holds a second closed table, `FONT_TYPES`: `ttf` as `font/ttf`, `otf` as `font/otf`, `woff` as
`font/woff` and `woff2` as `font/woff2`, the extension compared ASCII-case-insensitively. `TYPES` is
unchanged.

R4. For a reader that asks, each `url(` in the face's CSS, matched ASCII-case-insensitively, has its
argument read up to its `)`, trimmed, and stripped of one pair of matching quotes. An argument that decodes
(SPEC-348 R3's `%XX` rule) to a plain file name whose extension is in `FONT_TYPES` is judged by SPEC-348
R3's rules in their order. Admitted, it is written `url("data:<type>;base64,<bytes>")`; refused, it is
written `url("")` and its written name joins `omitted` once. Every other argument, and every byte of the CSS
outside the rewritten `url(...)`, stays as written.

R5. The font pass takes from the face's budget after the text's media and the clips.

R6. `Reader::inlines_fonts` answers `false` by default, and for such a reader the face's CSS is the note
type's CSS byte for byte and the reader is asked for no font. The ffi's media folder answers `true`.

R7. `Clip::Speech` carries `voices`, the tag's `voices=` list in its order, empty when the tag names none,
in the core and across the FFI.

R8. `VoiceChoices::voice_for(language, requested, installed)` answers the kept choice for the language
when it is installed; else the first requested entry that names a voice among `options(language,
installed)`, by the voice's identifier, or by its name with each space written `_`, the entry compared
whole or after its first `_`; else nothing.

R9. `EngineSession` speaks each tag with `voice_for`'s answer, binding the new field in its existing `case`;
it adds no decision, and `ios/swift-roles.json` is unchanged.

R10. The factory sets `allowsInlineMediaPlayback = true` on the configuration it builds, before the view
exists, inside the `#if os(iOS)` guard.

R11. The native document writes ` playsinline` after the element name of each `<video` start tag in the
face's text, matched ASCII-case-insensitively, where the name ends at whitespace, `/` or `>`. Every other
element, a `<video-note>` included, is unchanged.

R12. The layer census refuses, each by its own name, a factory without the inline line, one setting it
`false`, and one setting it outside the guard, and prints how many factories it examined.

R13. The web engine's clip value ignores the new field. The web's faces, its view, its fetches and its
class are unchanged.

R14. The card view's containment is unchanged: `RuleList.swift`, `DocumentPolicy.swift`, the navigation
gate and the planted suite are not edited, and every font and video reaches WebKit only as a `data:` URL
inside the document.

R15. The review fixture gains a second collection, `parity/` (deck `Parity`), built in the test from fixed
bytes: one note of a two-template note type whose CSS names `_parity.ttf` (bytes `parity-font`), whose first
template speaks `{{tts en_US voices=Absent_Voice,Desk_Parity_Voice:Front}}`, and whose second shows
`<video src="parity.mp4" controls></video>`, `<VIDEO controls src="parity.mp4"></VIDEO>` and a
`<video-note>` element (`parity.mp4` holds the bytes `parity-video`). The deck `Review`, its four cards and
its two files are unchanged.

## 3. Acceptance criteria of the review screen's parity

| id | criterion | decided by |
|---|---|---|
| A1 | One note of a two-template note type: the forward card's faces carry ordinal 0 and the reverse card's carry 1, on both sides | `cargo test -p deck-streak-engine-core --test face -- --exact a_face_carries_its_cards_template_ordinal` |
| A2 | The parity collection's second card's document has the body class `card card2` by day and `card card2 nightMode night_mode` by night | `cargo test -p deck-streak-ffi --test face -- --exact the_body_class_names_the_cards_template` |
| A3 | The closed-page test reads `card card1` by day and `card card1 nightMode night_mode` by night, every other assertion as it was | `cargo test -p deck-streak-ffi --test face -- --exact the_native_document_is_one_closed_page` |
| A4 | For a reader that asks, `src: url("_parity.ttf")` over the bytes `parity-font` becomes `src: url("data:font/ttf;base64,cGFyaXR5LWZvbnQ=")`, the rest of the CSS byte for byte, and nothing is omitted | `cargo test -p deck-streak-engine-core --test face -- --exact a_font_the_css_names_is_inlined_for_a_reader_that_asks` |
| A5 | For a reader that asks, a font one byte over the file cap and a font with no file each become `url("")` and are named once, in the order met; a scheme, a `data:` URL, a path and an image name stay byte for byte and are not named | `cargo test -p deck-streak-engine-core --test face -- --exact a_refused_font_is_emptied_and_named_and_any_other_url_stays` |
| A6 | For a reader that does not ask, the CSS equals the note type's byte for byte, nothing is omitted, and the reader is asked for no font | `cargo test -p deck-streak-engine-core --test face -- --exact the_css_stays_as_written_for_a_reader_that_does_not_ask` |
| A7 | Three images and one sound tag of exactly the file cap each and a one-byte font: the images and the sound are admitted, and the font is refused and named | `cargo test -p deck-streak-engine-core --test face -- --exact a_font_never_crowds_out_the_cards_own_media` |
| A8 | The parity collection's first card's document carries `url("data:font/ttf;base64,cGFyaXR5LWZvbnQ=")`, and every `url(` in it holds a `data:` URL, the count examined printed and above zero | `cargo test -p deck-streak-ffi --test face -- --exact the_native_document_carries_its_fonts_inline` |
| A9 | `{{tts en_US voices=Absent_Voice,Desk_Parity_Voice:Front}}` becomes a speech clip whose voices are those two in that order, and a tag with no `voices=` gives none | `cargo test -p deck-streak-engine-core --test face -- --exact a_tts_tag_carries_its_voices_in_order` |
| A10 | With no kept choice, `voice_for("en-US", ["Absent_Voice", "Desk_Parity_Voice"], installed)` answers `test.voice.parity`, the installed `en-US` voice named `Parity Voice` | `cargo test -p deck-streak-ffi --test voices -- --exact a_requested_voice_speaks_when_no_choice_is_kept` |
| A11 | With a kept, installed choice for `en-US`, `voice_for` answers the kept identifier though the request names another installed voice | `cargo test -p deck-streak-ffi --test voices -- --exact a_kept_choice_speaks_over_a_requested_voice` |
| A12 | A request naming a voice of another language, then one not installed, then an installed voice's identifier, then a second installed voice, answers that identifier; a request whose every entry is passed over answers nothing | `cargo test -p deck-streak-ffi --test voices -- --exact a_request_the_picker_would_not_offer_is_passed_over` |
| A13 | The parity collection's first card's speech clip carries the voices `Absent_Voice` and `Desk_Parity_Voice` across the FFI | `cargo test -p deck-streak-ffi --test face -- --exact the_speech_clip_carries_the_tags_voices` |
| A14 | On both simulators, the session on the parity collection, given the installed `en-US` voice `Parity Voice` (`test.voice.parity`), shows the first card with a replay speech clip spoken by `test.voice.parity` | `xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak ... -only-testing:DeckStreakTests/ReviewSessionTests/test_the_parity_card_speaks_the_voice_its_template_asks` |
| A15 | The parity collection's second card's document: both video start tags carry `playsinline`, 2 of 2 examined, and the `<video-note>` start tag is as the field wrote it | `cargo test -p deck-streak-ffi --test face -- --exact a_video_plays_inline_in_the_native_document` |
| A16 | The layer census finds the inline line inside the guard in the real factory, and refuses by name a factory without it, one setting it false, and one setting it outside the guard | `python3 -m unittest discover -s scripts/tests -p test_card_web_view_layers.py -k test_the_factory_plays_media_inline_and_every_plant_is_refused` |
| A17 | On both simulators, a view the factory makes reads `allowsInlineMediaPlayback` as true | `xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe ... -only-testing:CardProbeTests/FactoryTests/test_the_factory_plays_media_inline` |
| A18 | The thin-Swift census holds every role at its ceiling and every door where it was, `EngineSession` at 4 | `python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py` |
| A19 | For a reader that asks, `src: url("_a.ttf")`, `src: url("_b.OTF")`, `src: url("_c.woff")` and `src: url("_d.Woff2")`, each over its own bytes, each become `url("data:<type>;base64,<its bytes>")` with `<type>` `font/ttf`, `font/otf`, `font/woff` and `font/woff2` in that order, the extension read in any ASCII case, the rest of the CSS byte for byte, and nothing is omitted | `cargo test -p deck-streak-engine-core --test face -- --exact every_font_type_is_inlined_under_its_own_type` |

A6, A11 and A18 are not red first: A6 and A11 pin what the base already does on the path the change
touches, and A18 guards that the Swift wiring adds no decision. A17 is red on the iPhone only, because
the iPad's default is already on (M21).

```acceptance
A1: cargo test -p deck-streak-engine-core --test face -- --exact a_face_carries_its_cards_template_ordinal
A2: cargo test -p deck-streak-ffi --test face -- --exact the_body_class_names_the_cards_template
A3: cargo test -p deck-streak-ffi --test face -- --exact the_native_document_is_one_closed_page
A4: cargo test -p deck-streak-engine-core --test face -- --exact a_font_the_css_names_is_inlined_for_a_reader_that_asks
A5: cargo test -p deck-streak-engine-core --test face -- --exact a_refused_font_is_emptied_and_named_and_any_other_url_stays
A6: cargo test -p deck-streak-engine-core --test face -- --exact the_css_stays_as_written_for_a_reader_that_does_not_ask
A7: cargo test -p deck-streak-engine-core --test face -- --exact a_font_never_crowds_out_the_cards_own_media
A8: cargo test -p deck-streak-ffi --test face -- --exact the_native_document_carries_its_fonts_inline
A9: cargo test -p deck-streak-engine-core --test face -- --exact a_tts_tag_carries_its_voices_in_order
A10: cargo test -p deck-streak-ffi --test voices -- --exact a_requested_voice_speaks_when_no_choice_is_kept
A11: cargo test -p deck-streak-ffi --test voices -- --exact a_kept_choice_speaks_over_a_requested_voice
A12: cargo test -p deck-streak-ffi --test voices -- --exact a_request_the_picker_would_not_offer_is_passed_over
A13: cargo test -p deck-streak-ffi --test face -- --exact the_speech_clip_carries_the_tags_voices
A14: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_the_parity_card_speaks_the_voice_its_template_asks
A15: cargo test -p deck-streak-ffi --test face -- --exact a_video_plays_inline_in_the_native_document
A16: python3 -m unittest discover -s scripts/tests -p test_card_web_view_layers.py -k test_the_factory_plays_media_inline_and_every_plant_is_refused
A17: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/FactoryTests/test_the_factory_plays_media_inline
A18: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
A19: cargo test -p deck-streak-engine-core --test face -- --exact every_font_type_is_inlined_under_its_own_type
```

## 4. File manifest

| path | context | for |
|---|---|---|
| `crates/engine-core/src/face.rs` | engine-core | R1, R5, R7: `Face.ordinal`, the font pass's place, `Clip::Speech.voices` |
| `crates/engine-core/src/media.rs` | engine-core | R3, R4, R6: `FONT_TYPES`, the CSS pass, `Reader::inlines_fonts` |
| `crates/engine-core/tests/face.rs` | engine-core | A1, A4 to A7, A9, A19; its `speech` helper takes the voices |
| `crates/ffi/src/face.rs` | ffi | R2, R6, R7, R11: the class, the media folder's answer, the clip's voices, `playsinline` |
| `crates/ffi/src/voices.rs` | ffi | R8: `voice_for` |
| `crates/ffi/tests/face.rs` | ffi | A2, A3, A8, A13, A15 |
| `crates/ffi/tests/voices.rs` | ffi | A10 to A12 |
| `crates/ffi/tests/support/parity.rs` | ffi | R15: the parity collection (new) |
| `crates/ffi/tests/support/mod.rs` | ffi | the new support module |
| `crates/ffi/examples/review-fixture.rs` | ffi | R15: writes `parity/` beside the review collection |
| `crates/web-engine/src/wasm.rs` | web-engine | R13: `clip_value` binds the speech clip with a rest pattern |
| `ios/App/Sources/EngineSession.swift` | app | R9: one `case` binding and one call |
| `ios/AppTests/ReviewSessionTests.swift` | app tests | A14 |
| `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | CardIsolation | R10 |
| `ios/CardProbeTests/FactoryTests.swift` | probe tests | A17 |
| `scripts/tests/test_card_web_view_layers.py` | census | R12, A16 |
| `scripts/mutation-rows.d/S39300-S39399.json` | rows | section 8 (new) |
| `docs/specs/SPEC-393-the-iphone-and-ipad-review-screen-closes-four-of-the-desktop-reviewers-parity-gaps.md` | docs | this SPEC (new) |
| `docs/decisions/ADR-407-the-core-carries-the-ordinal-the-fonts-and-the-voices-and-the-card-view-plays-video-inline.md` | docs | its decision (new) |
| `docs/schematics/ios-review-screen.md` | docs | section 6, appended |
| `docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md` | docs | section 11, appended (section 10 below) |
| `docs/decisions/ADR-359-the-review-screens-media-face-voice-choice-deck-tree-rating-and-test-seam.md` | docs | an amendment, appended (section 10 below) |
| `docs/red-first/SPEC-393.md` | docs | the red-first record (new) |
| `changelog.d/review-screen-parity-393.md` | docs | the fragment (new) |

Not changed, and a change to any of them is outside this SPEC: `RuleList.swift`, `DocumentPolicy.swift`,
`ios/App/Sources/CardFaceView.swift`, `ios/App/Sources/ClipPlayer.swift`, `ios/swift-roles.json`,
`scripts/tests/test_ios_thin_swift.py`, the workflows, `crates/ffi/src/engine.rs`,
`crates/engine-core/src/lib.rs` and `ios/CardIsolation/swift-mutants.json`.

## 5. What this does NOT cover

- A sound tag that names a video file still plays as a sound and shows no picture: it is cut to a
  follow-up (section 9; ADR-407 D5), so this delivery refers to #666 and does not close it.
- The web client's voices, its CSS fonts and its video are unchanged: the web's parity with the desktop is
  #630's, and its reader does not ask for fonts.
- No TTS option beyond the voices and the speed is applied: the desktop hands the others to its own players,
  and no voice on an iPhone or iPad reads them (#632).
- An image the note type's CSS names by `url()` stays as written, so the card view does not show it (#632;
  section 9).
- An HTML video does not start by itself: a tap starts it, as the configuration's default asks (#632;
  section 9).
- Captions or a transcript for a card's video are the deck's markup, which this delivery does not write
  (#632).
- No card script runs, and no message handler is added (#651).
- The card view's isolation is unchanged; a font or video reaching WebKit by a route other than a `data:`
  URL inside the document would re-run the planted suite (#619).
- WebKit's connection to a link's host and a followed link's one connection are #664 and #677.
- No Swift mutant is added for the app's files or for the factory's iPhone-only line: the package's mutants
  run on a host where that line is compiled out, and a sweep of the app's Swift is #650.
- That a card renders and plays as the desktop does on a device is a device session (#629,
  #637; section 7).

## 6. Risks

| risk | what detects it |
|---|---|
| A `<video>` source is written `data:audio/mp4` (the shared table reads `mp4` as audio), and a device plays the sound with no picture | the device session of section 7; a fix re-opens ADR-359 D1 and is its own decision |
| A whole CJK font is over the file cap and is left out, so the system's font draws the card | the face's `omitted` names it; A5 pins the refusal |
| A font sliced by `unicode-range` names many files and reaches the face's cap | A7 pins that the font, never the card's own media, is what is left out |
| The `url(` reader misreads a comment or an escape in the CSS | R4 leaves every argument it does not admit as written, so a misread leaves text and fetches nothing; A5 plants each shape |
| A request names a voice of the same name on another system | R8 asks the picker's offer for the language first, and a kept choice still wins (A11) |
| The iPad's inline default is already on, so A17 is not red there | the red-first record names the iPhone destination's failure |
| A path this delivery changes is changed by another pull request first | the builder re-measures each shared path at the cut and before each push |
| The runner's simulators hold different voices | A14 passes a fixed installed list and reads no system voice |

## 7. Proved on a device

A card using the four built behaviours (a `.card2` rule, a font named by `url()`, a `voices=` tag and an
HTML video) renders and plays as the desktop does on an iPhone and an iPad: a device session
(#629, #637), after the green push's CI, recorded on #666. No repository test decides it.

## 8. Mutation rows

The band file `scripts/mutation-rows.d/S39300-S39399.json` holds these rows in the band's existing shape:
`MUTATIONS` rows of seven cells (id, crate, file relative to the crate, find, replace, killer, why) and
`SCRIPT_MUTATIONS` rows of six (id, path from the root, find, replace, why, killer). Each find is written
after formatting and occurs exactly once in its file. A killer named `face::<test>` is in its row's crate.

| id | table | target | the mutant | killer |
|---|---|---|---|---|
| `S39300-THE-FACE-CARRIES-ITS-TEMPLATE-ORDINAL` | `MUTATIONS` | engine-core `src/face.rs` | the ordinal read from the card becomes 0 | `face::a_face_carries_its_cards_template_ordinal` |
| `S39301-THE-FONT-TABLE-TYPES-A-TTF-AS-TTF` | `MUTATIONS` | engine-core `src/media.rs` | the `ttf` row's type becomes `font/otf` | `face::a_font_the_css_names_is_inlined_for_a_reader_that_asks` |
| `S39302-A-URL-ARGUMENT-LOSES-ITS-QUOTES` | `MUTATIONS` | engine-core `src/media.rs` | the quote strip is skipped | `face::a_font_the_css_names_is_inlined_for_a_reader_that_asks` |
| `S39303-A-REFUSED-FONT-IS-EMPTIED` | `MUTATIONS` | engine-core `src/media.rs` | a refused font keeps its written name | `face::a_refused_font_is_emptied_and_named_and_any_other_url_stays` |
| `S39304-ONLY-A-FONT-NAME-IS-REWRITTEN` | `MUTATIONS` | engine-core `src/media.rs` | the font-table filter admits every type | `face::a_refused_font_is_emptied_and_named_and_any_other_url_stays` |
| `S39305-ONLY-A-READER-THAT-ASKS-GETS-FONTS` | `MUTATIONS` | engine-core `src/face.rs` | the reader's answer is taken as yes | `face::the_css_stays_as_written_for_a_reader_that_does_not_ask` |
| `S39306-A-READER-DOES-NOT-ASK-BY-DEFAULT` | `MUTATIONS` | engine-core `src/media.rs` | the trait's default answers yes | `face::the_css_stays_as_written_for_a_reader_that_does_not_ask` |
| `S39307-FONTS-ARE-TAKEN-AFTER-THE-CARDS-MEDIA` | `MUTATIONS` | engine-core `src/face.rs` | a font pass is inserted ahead of the text's media | `face::a_font_never_crowds_out_the_cards_own_media` |
| `S39308-THE-SPEECH-CARRIES-THE-TAGS-VOICES` | `MUTATIONS` | engine-core `src/face.rs` | the clip's voices become empty | `face::a_tts_tag_carries_its_voices_in_order` |
| `S39309-THE-URL-NAME-IS-MATCHED-IN-ANY-CASE` | `MUTATIONS` | engine-core `src/media.rs` | `url(` is matched case-sensitively | `face::a_font_the_css_names_is_inlined_for_a_reader_that_asks` |
| `S39310-THE-BODY-CLASS-COUNTS-FROM-ONE` | `MUTATIONS` | ffi `src/face.rs` | the class writes the ordinal without adding one | `face::the_body_class_names_the_cards_template` |
| `S39311-THE-NATIVE-READER-ASKS-FOR-FONTS` | `MUTATIONS` | ffi `src/face.rs` | the media folder answers no | `face::the_native_document_carries_its_fonts_inline` |
| `S39312-EVERY-VIDEO-PLAYS-INLINE` | `MUTATIONS` | ffi `src/face.rs` | the inserted attribute is empty | `face::a_video_plays_inline_in_the_native_document` |
| `S39313-A-VIDEO-TAG-IS-FOUND-IN-ANY-CASE` | `MUTATIONS` | ffi `src/face.rs` | the element name is compared case-sensitively | `face::a_video_plays_inline_in_the_native_document` |
| `S39314-A-VIDEO-NAME-ENDS-AT-ITS-TAG` | `MUTATIONS` | ffi `src/face.rs` | the name's end test admits any character | `face::a_video_plays_inline_in_the_native_document` |
| `S39315-THE-FFI-SPEECH-CARRIES-ITS-VOICES` | `MUTATIONS` | ffi `src/face.rs` | the ffi clip's voices become empty | `face::the_speech_clip_carries_the_tags_voices` |
| `S39316-A-KEPT-CHOICE-SPEAKS-FIRST` | `MUTATIONS` | ffi `src/voices.rs` | the kept choice is never consulted | `voices::a_kept_choice_speaks_over_a_requested_voice` |
| `S39317-A-REQUESTED-VOICE-IS-ONE-THE-PICKER-OFFERS` | `MUTATIONS` | ffi `src/voices.rs` | every installed voice is a candidate | `voices::a_request_the_picker_would_not_offer_is_passed_over` |
| `S39318-A-REQUESTED-NAME-MAY-CARRY-ONE-PREFIX` | `MUTATIONS` | ffi `src/voices.rs` | the entry is compared whole only | `voices::a_requested_voice_speaks_when_no_choice_is_kept` |
| `S39319-A-REQUESTED-NAME-WRITES-SPACES-AS-UNDERSCORES` | `MUTATIONS` | ffi `src/voices.rs` | spaces stay spaces | `voices::a_requested_voice_speaks_when_no_choice_is_kept` |
| `S39320-A-REQUESTED-IDENTIFIER-SPEAKS` | `MUTATIONS` | ffi `src/voices.rs` | the identifier comparison never matches | `voices::a_request_the_picker_would_not_offer_is_passed_over` |
| `S39321-THE-FIRST-OFFERED-REQUEST-WINS` | `MUTATIONS` | ffi `src/voices.rs` | the requests are read last first | `voices::a_request_the_picker_would_not_offer_is_passed_over` |
| `S39322-THE-FONT-TABLE-TYPES-AN-OTF-AS-OTF` | `MUTATIONS` | engine-core `src/media.rs` | the `otf` row's type becomes `font/ttf` | `face::every_font_type_is_inlined_under_its_own_type` |
| `S39323-THE-FONT-TABLE-TYPES-A-WOFF-AS-WOFF` | `MUTATIONS` | engine-core `src/media.rs` | the `woff` row's type becomes `font/woff2` | `face::every_font_type_is_inlined_under_its_own_type` |
| `S39324-THE-FONT-TABLE-TYPES-A-WOFF2-AS-WOFF2` | `MUTATIONS` | engine-core `src/media.rs` | the `woff2` row's type becomes `font/woff` | `face::every_font_type_is_inlined_under_its_own_type` |
| `S39340-THE-FACTORY-PLAYS-MEDIA-INLINE` | `SCRIPT_MUTATIONS` | `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | the property is set false | `test_card_web_view_layers.TheFactoryPlaysMediaInline.test_the_factory_plays_media_inline_and_every_plant_is_refused` |
| `S39341-THE-INLINE-CENSUS-NAMES-ITS-GUARD` | `SCRIPT_MUTATIONS` | `scripts/tests/test_card_web_view_layers.py` | the census stops requiring the guard | `test_card_web_view_layers.TheFactoryPlaysMediaInline.test_the_factory_plays_media_inline_and_every_plant_is_refused` |

The Swift wiring of R9 is killed by A14 on CI and carries no row here (#650). Every existing row whose target
is a file this delivery changes keeps a find that occurs exactly once, `S34824` included.

## 9. Follow-up issues

Each states a behaviour only.

1. **A sound tag that names a video file shows its picture on iPhone and iPad.** When a card's sound tag names
   a video file, the review screen plays it with its picture and its sound, as the desktop reviewer plays
   it, and replay plays it again. Today such a tag plays as a sound with no picture.
2. **An image the note type's CSS names by `url()` shows on iPhone and iPad.** A background or other image
   that a note type's styling names from the collection's media appears on the card as on the desktop.
   Today the reference stays as written and the card view shows nothing for it.
3. **An HTML video that asks to start by itself starts on iPhone and iPad.** A card's `<video>` that carries
   the attribute asking to start on show starts as on the desktop. Today a tap starts it.

## 10. Amendments this delivery appends

Each block below is appended, every line strictly between its BEGIN and END lines, at the end of the file
its BEGIN line names, after one blank line.

BEGIN docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md
## 11. Amendments: four of #666's gaps are built (SPEC-393, ADR-407)

SPEC-393 builds four of the behaviours section 5 left to #666. This section is the pointer, so no rule
above is read without it:

- R5's body classes become `card card<n>`, `<n>` the card's template index plus one, then the night
  classes when asked; A8's test reads `card card1` by day (SPEC-393 A3).
- The face's CSS carries each font it names by `url()` as a `data:` URL, for the native reader (SPEC-393
  R3 to R6). The HTML rewrite and its type table are unchanged.
- `Clip::Speech` carries the tag's `voices`. The voice that speaks is the kept choice, else the first
  requested voice the picker offers, else the language's own (SPEC-393 R7 to R9; ADR-407 D3).
- The card view plays an HTML video inline on an iPhone (SPEC-393 R10, R11).

A sound tag that names a video still plays as a sound (SPEC-393 section 9, #666).
END

BEGIN docs/decisions/ADR-359-the-review-screens-media-face-voice-choice-deck-tree-rating-and-test-seam.md
## Amendment: fonts, voices and the body class (SPEC-393, ADR-407)

- D1 gains a second closed table, `FONT_TYPES`, read only by the face's CSS pass and only for a reader that
  asks (ADR-407 D2 and D6). `TYPES` and the HTML rewrite are unchanged, and every font reaches the frame as
  a `data:` URL inside the document.
- D3's choice is the first of three: the kept choice, then the tag's requested voices the picker would
  offer, then the language's own (ADR-407 D3).
END

## 11. Formal

NOT APPLICABLE by surface (ADR-407 D8). No actor, timer, shared state or write path is added: the ordinal,
the CSS pass and the voice order are total functions inside one face's completion, and `voice_for` reads
the choices behind the lock `chosen` reads, from the same caller, in its place. Of the 219 `@phx covers`
lines under `formal/` at DEV, three name `crates/web-engine/src/wasm.rs` (`undo`, `undo_offer`, `rate`) and
one names `crates/ffi/src/engine.rs` (`run`); this delivery edits `clip_value` alone in the first and does
not edit the second.

# SPEC-410: a sound tag that names a video plays as video in the iPhone and iPad card view

- **Wave:** the app campaign, the review screen's parity (part of #632). **Issue:** #808 (Closes: the delivery
  builds the gap whole). **Context(s):** `engine-core` (`crates/engine-core`), `ffi` (`crates/ffi`), the
  iPhone and iPad app's tests (`ios/AppTests`).
- **Decided by:** ADR-424 (this SPEC's own: D1 the fix, D2 the containment, D3 the scope, D4 the formal
  surface, D5 the reds on CI), ADR-359 (the review screen's media, amended here) and ADR-407 (D5's cut,
  amended here).
- **Schematic:** `docs/schematics/ios-review-screen.md` section 7 (this delivery appends it).
- **Status:** designed; one pull request, two pushes (ADR-424 D3, D5). **Mutation band:** `S41000-S41099`.
  **Changelog:** `changelog.d/sound-tag-video-410.md`. **Red-first record:** `docs/red-first/SPEC-410.md`.

Refs read: `DEV` is DeckStreak `dev` `ff405424907647ee0f41eb6bbff6276f9dadbda3`. Every fact below is read at
DEV with `git show DEV:<path>`, at the lines beside it.

## 1. The problem, measured

| id | fact | where at DEV |
|---|---|---|
| M1 | The engine names a sound tag `SoundOrVideo(name)` and gives no media kind; the core hands the name to `budget.sound(name)`. `SoundOrVideo`, `is_video` and `VIDEO_EXT` occur in this file alone | `crates/engine-core/src/face.rs:165` |
| M2 | The shared `TYPES` holds images and audio only: `mp4` is `audio/mp4`, `webm` is `audio/webm`, and there is no `mov` and no `m4v` | `crates/engine-core/src/media.rs:17-36` (`:18`, `:34`, `:35`) |
| M3 | `Budget::sound` is `take(written, &TYPES, "audio/")`, so a `.mp4` sound tag crosses as `Clip::Sound` bytes and a `.mov` or `.m4v` one joins `omitted` | `crates/engine-core/src/media.rs:157-159`, `:217-229` |
| M4 | The native player plays a sound clip's bytes as audio; there is no picture | `ios/App/Sources/ClipPlayer.swift:65-77` |
| M5 | A table read only for a reader that asks exists: `FONT_TYPES` behind `Reader::inlines_fonts` (default `false`); the ffi's `MediaFolder` answers `true` | `crates/engine-core/src/media.rs:41-46`, `:60-62`; `crates/ffi/src/face.rs:74-76` |
| M6 | `admit` reads at most `FILE_CAP + 1` bytes, refuses past `FILE_CAP` (4 MiB) and `FACE_CAP` (16 MiB a face); a refusal names the file once in `omitted` | `crates/engine-core/src/media.rs:13`, `:15`, `:217-229`, `:232-254` |
| M7 | The review screen shows the count of omitted files | `ios/App/Sources/ReviewChrome.swift:62-65` |
| M8 | The ffi writes ` playsinline` after each `<video` start tag's name, in any ASCII case | `crates/ffi/src/face.rs:102-121` |
| M9 | The factory plays media inline on iOS and sets no `mediaTypesRequiringUserActionForPlayback`; it loads the one document with `loadHTMLString(html, baseURL: nil)` | `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:198-204`, `:64` |
| M10 | The document's policy admits `media-src data:` and `style-src 'unsafe-inline'`; the rule list blocks every load; card scripts are off | `ios/CardIsolation/Sources/CardIsolation/DocumentPolicy.swift:13-15`, `RuleList.swift:13`, `CardScripts.swift:7` |
| M11 | The thin-Swift census caps each role and grants `WebKit` to the card role alone and `AVFoundation`/`AVFAudio` to the speech role alone | `scripts/tests/test_ios_thin_swift.py:38-47`, `:74-89` |
| M12 | The web plays a sound through the page's audio element from a Blob URL; its reader keeps the defaults; `clip_value` binds `Clip::Sound {name, bytes}` | `web/app/src/lib/study/audio.ts:1-4`, `:71-83`; `crates/web-engine/src/wasm.rs:877-900` |
| M13 | A template's own `<video src>` naming an `.mp4` is written `data:audio/mp4` and pinned so | `crates/ffi/tests/face.rs:336-357` |
| M14 | The review fixture's directory is written by the `review-fixture` example and copied whole into the native harness; the review screen's step runs `-only-testing:DeckStreakTests/ReviewSessionTests` | `.github/workflows/xcframework.yml:127`, `:175`, `:498`, `:505`, `:512` |
| M15 | No cover line under `formal/` names a file this delivery edits | the 246 cover lines at DEV (section 11) |

## 2. Requirements

R1. `crates/engine-core/src/media.rs` holds `pub const VIDEO_TYPES: [(&str, &str); 3]`, exactly
`("mp4", "video/mp4")`, `("m4v", "video/mp4")` and `("mov", "video/quicktime")`, and a file name's extension
is matched against it in any ASCII case.

R2. `Reader` has `fn shows_video(&self) -> bool`, `false` by default. The ffi's `MediaFolder` answers
`true`. The web's reader keeps the default.

R3. For a reader whose `shows_video` is `true`, a sound tag whose extension is in `VIDEO_TYPES` yields no
clip. When the budget admits it, the face's text gains, after the side's rewritten text, one element per
admitted video, in the side's replay order:

```html
<video controls style="display:block;max-width:100%;height:auto;margin:0 auto" src="data:<type>;base64,<bytes>"></video>
```

`<type>` is the type `VIDEO_TYPES` gives the extension, and `<bytes>` the file's bytes in standard base64
with padding. The replay order is the question's videos on the question side; on the answer side, the
question's videos then the answer's own, with the question's left out when the preset skips the question on
replay. The element carries no part of the written name.

R4. Such a video is admitted under SPEC-348 R3's rules and both caps, unchanged, under the kind `video/`. A
refused one adds no element and joins `omitted` once.

R5. The ffi's document carries `playsinline` on each such element by the existing pass, so the native
document holds `<video playsinline controls style="..." src="..."></video>`.

R6. The face's `autoplay` and `replay` hold no clip for such a video. Every other clip is as before.

R7. For a reader that does not ask, every face is as at DEV: a sound tag naming a `.mp4` stays a
`Clip::Sound`, and one naming a `.m4v` or a `.mov` joins `omitted`.

R8. A fixture, `crates/ffi/tests/support/sound_video.rs`, builds the "Sound video" collection from fixed bytes
(section 9), and the `review-fixture` example writes it to `<output>/sound-video` beside `<output>/parity`.

R9. On the iPhone and the iPad simulator, the review session on the fixture shows a first face whose document
holds both question-side elements and whose replay is the one sound `beep.mp3`.

R10. No file of the card view's isolation changes, no Swift product file changes, and the thin-Swift census
holds every role at its ceiling and every door where it was.

## 3. Acceptance criteria of a sound tag's video

| id | criterion | decided by |
|---|---|---|
| A1 | For a reader that asks, the question side of a note whose front is `[sound:sign.mp4][sound:beep.mp3][sound:wave.m4v]` ends its text with sign's element then wave's, both typed `video/mp4` over their bytes' base64 (`c2lnbi12aWRlbw==`, `d2F2ZS12aWRlbw==`); its replay is the one sound `beep.mp3`; nothing is omitted | `cargo test -p deck-streak-engine-core --test face -- --exact a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| A2 | The answer side of that note, with back `[sound:turn.MOV]`, ends its text with sign's, wave's, then turn's element, turn's typed `video/quicktime` (`dHVybi12aWRlbw==`); in a deck whose preset skips the question on replay, it ends with turn's element alone | `cargo test -p deck-streak-engine-core --test face -- --exact the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips` |
| A3 | For a reader that asks, a front `[sound:large.mp4][sound:small.mp4]`, `large.mp4` being 4,194,305 bytes and `small.mp4` the bytes `small-video`, ends its text with small's element alone (`c21hbGwtdmlkZW8=`), omits `["large.mp4"]` once, and replays nothing | `cargo test -p deck-streak-engine-core --test face -- --exact a_video_over_the_file_cap_is_omitted_and_named_once` |
| A4 | For a reader that does not ask, that note's question side replays `sign.mp4` then `beep.mp3` as sounds, omits `["wave.m4v"]`, and its text holds no `<video`; its answer side names `turn.MOV` in `omitted` | `cargo test -p deck-streak-engine-core --test face -- --exact a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound` |
| A5 | The fixture's card, question side, by day: the native document holds sign's element then wave's, each `<video playsinline controls style="display:block;max-width:100%;height:auto;margin:0 auto" src="data:video/mp4;base64,...">`, and 2 of 2 video start tags are examined after that assertion; its replay is the one sound `beep.mp3` | `cargo test -p deck-streak-ffi --test face -- --exact a_sound_tags_video_plays_inline_in_the_native_document` |
| A6 | On both simulators, the review session on the fixture's "Sound video" deck shows a first face whose document holds both elements and whose replay is `[.sound(name: "beep.mp3", bytes: Data("beep-sound".utf8))]` | `xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak ... -only-testing:DeckStreakTests/ReviewSessionTests/test_a_sound_tags_video_shows_in_the_review_screens_face` |
| A7 | The thin-Swift census holds every role at its ceiling and every door where it was | `python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py` |

A4 and A7 are not red first: A4 pins what DEV already does for a reader that does not ask, on the path the
change touches, and A7 guards that no Swift product decision is added. A1, A2, A3, A5 and A6 are red at the
seams, each on an assertion about the missing element; A6 is read on CI, per destination.

```acceptance
A1: cargo test -p deck-streak-engine-core --test face -- --exact a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks
A2: cargo test -p deck-streak-engine-core --test face -- --exact the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips
A3: cargo test -p deck-streak-engine-core --test face -- --exact a_video_over_the_file_cap_is_omitted_and_named_once
A4: cargo test -p deck-streak-engine-core --test face -- --exact a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound
A5: cargo test -p deck-streak-ffi --test face -- --exact a_sound_tags_video_plays_inline_in_the_native_document
A6: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a_sound_tags_video_shows_in_the_review_screens_face
A7: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/engine-core/src/media.rs` | engine-core | R1, R2, R4: `VIDEO_TYPES`, `Reader::shows_video` (default `false`), the budget's video admission and the element |
| `crates/engine-core/src/face.rs` | engine-core | R3, R6: `clips` routes a video's name to the budget and yields no clip; `complete` appends the side's videos in replay order |
| `crates/engine-core/tests/face.rs` | engine-core | A1 to A4, with an in-memory reader that asks for video |
| `crates/ffi/src/face.rs` | ffi | R2, R5: `MediaFolder` answers `shows_video` with `true` |
| `crates/ffi/tests/face.rs` | ffi | A5 over the fixture |
| `crates/ffi/tests/support/sound_video.rs` | ffi | new: R8's fixture (section 9) |
| `crates/ffi/tests/support/mod.rs` | ffi | declares `pub mod sound_video;` |
| `crates/ffi/examples/review-fixture.rs` | ffi | R8: writes `<output>/sound-video` |
| `ios/AppTests/ReviewSessionTests.swift` | the app's tests | A6 |
| `scripts/mutation-rows.d/S41000-S41099.json` | rows | new: section 8's rows |
| `docs/specs/SPEC-410-a-sound-tag-that-names-a-video-plays-as-video-in-the-iphone-and-ipad-card-view.md` | documents | new: this SPEC |
| `docs/decisions/ADR-424-the-core-writes-a-sound-tags-video-into-the-face-as-a-data-url-video-element-for-a-reader-that-asks.md` | documents | new: its decision |
| `docs/schematics/ios-review-screen.md` | documents | section 7 appended |
| `docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md` | documents | section 12 appended (section 10's first block) |
| `docs/decisions/ADR-359-the-review-screens-media-face-voice-choice-deck-tree-rating-and-test-seam.md` | documents | an amendment appended (section 10's second block) |
| `docs/decisions/ADR-407-the-core-carries-the-ordinal-the-fonts-and-the-voices-and-the-card-view-plays-video-inline.md` | documents | an amendment appended (section 10's third block) |
| `docs/red-first/SPEC-410.md` | documents | new: the red-first record |
| `changelog.d/sound-tag-video-410.md` | documents | new: the changelog fragment |

Not changed: `ios/CardIsolation/Sources/CardIsolation/RuleList.swift`,
`ios/CardIsolation/Sources/CardIsolation/DocumentPolicy.swift`,
`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift`,
`ios/CardIsolation/Sources/CardIsolation/CardScripts.swift`, `ios/App/Sources/CardFaceView.swift`,
`ios/App/Sources/ClipPlayer.swift`, `ios/App/Sources/ReviewModel.swift`, `ios/App/Sources/EngineSession.swift`,
`ios/swift-roles.json`, `scripts/tests/test_ios_thin_swift.py`, the workflows, `crates/web-engine/src/wasm.rs`,
`crates/ffi/src/engine.rs`, `crates/ffi/Cargo.toml`, `crates/engine-core/src/lib.rs`, and `TYPES`.

## 5. What this does NOT cover

- Captions for a sound tag's video: the tag carries no caption source (#632).
- Autoplay of a sound tag's video, and Replay or Stop acting on it: it starts from a tap on its own controls
  (#632).
- `webm` as video: a sound tag naming a `.webm` stays a sound, as `TYPES` reads it (#632).
- A template's own `<video src>` naming an `.mp4`: it keeps the type `TYPES` gives it, `audio/mp4` (#632).
- An audio-only `.mp4` named by a sound tag: it now shows an element whose picture is empty (#632).
- A video over the file cap or the face cap: it is left out and named, never played (#632).
- The web client's parity with the desktop reviewer (#630).
- Any route into the card view other than a `data:` URL inside the document (#619).
- Card scripts, which stay off (#651).
- A repository test that decodes a picture: the fixtures build their files from bytes, so the decode is the
  device session's (section 7, #629, #637).

## 6. Risks

- A sound tag's video no longer starts by itself, where as a sound it followed the face's autoplay. A1 and
  A6 pin the new face, and the pull request names the change.
- A large video makes a large document, a third larger than the file. The caps bound it (4 MiB a file,
  16 MiB a face), and the device session watches the card's first paint.
- Another delivery that changes `complete()`, `clips()`, `MediaFolder`, the fixture example or the
  schematic's section 7 lands first. The build re-measures each shared path at its cut and stops on one that
  changed.
- A red step in the native job skips the steps after it. A6 is read per destination from the step's own log
  lines, and a destination the log does not name is unproven, never red or green.
- A builder who places the element or the guard in another file than section 8 names moves the rows'
  targets. The rows are written after the green, each find counted once in its file.

## 7. Proved on a device

A card whose sound tags name an `.mp4`, an `.m4v` and a `.mov` video shows each picture and plays its sound
from a tap on its controls, on an iPhone and an iPad: a device session (#629, #637), after the green push's
CI, recorded on #808. No repository test decides it.

## 8. Mutation rows

The band file `scripts/mutation-rows.d/S41000-S41099.json` holds these rows in the band's existing shape:
`MUTATIONS` rows of seven cells (id, crate, file relative to the crate, find, replace, killer, why). Each find
is written after formatting and occurs exactly once in its file. A killer named `face::<test>` is in its
row's crate. `VIDEO_TYPES` is a constant, which a generated mutant never changes, so each member owes its own
row.

| id | table | target | the mutant | killer |
|---|---|---|---|---|
| `S41000-THE-VIDEO-TABLE-READS-MP4` | `MUTATIONS` | engine-core `src/media.rs` | the `mp4` row's extension becomes `mpx` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41001-THE-VIDEO-TABLE-TYPES-MP4-AS-VIDEO-MP4` | `MUTATIONS` | engine-core `src/media.rs` | the `mp4` row's type becomes `video/webm` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41002-THE-VIDEO-TABLE-READS-M4V` | `MUTATIONS` | engine-core `src/media.rs` | the `m4v` row's extension becomes `m4x` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41003-THE-VIDEO-TABLE-TYPES-M4V-AS-VIDEO-MP4` | `MUTATIONS` | engine-core `src/media.rs` | the `m4v` row's type becomes `video/webm` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41004-THE-VIDEO-TABLE-READS-MOV` | `MUTATIONS` | engine-core `src/media.rs` | the `mov` row's extension becomes `mox` | `face::the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips` |
| `S41005-THE-VIDEO-TABLE-TYPES-MOV-AS-QUICKTIME` | `MUTATIONS` | engine-core `src/media.rs` | the `mov` row's type becomes `video/mp4` | `face::the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips` |
| `S41006-A-READER-SHOWS-NO-VIDEO-BY-DEFAULT` | `MUTATIONS` | engine-core `src/media.rs` | the default `shows_video` answers `true` | `face::a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound` |
| `S41007-THE-MEDIA-FOLDER-SHOWS-VIDEO` | `MUTATIONS` | ffi `src/face.rs` | `MediaFolder`'s `shows_video` answers `false` | `face::a_sound_tags_video_plays_inline_in_the_native_document` |
| `S41008-A-VIDEO-IS-ADMITTED-UNDER-THE-VIDEO-KIND` | `MUTATIONS` | engine-core `src/media.rs` | the video admission's kind `video/` becomes `audio/` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41009-THE-ELEMENT-CARRIES-CONTROLS` | `MUTATIONS` | engine-core `src/media.rs` | the element loses ` controls` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41010-THE-ELEMENT-FITS-THE-CARD-WIDTH` | `MUTATIONS` | engine-core `src/media.rs` | `max-width:100%` becomes `max-width:none` | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41011-THE-VIDEOS-JOIN-THE-FACE-TEXT` | `MUTATIONS` | engine-core `src/face.rs` | the append of the side's videos to the text is dropped | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41012-ONLY-A-READER-THAT-ASKS-ROUTES-A-VIDEO` | `MUTATIONS` | engine-core `src/media.rs` | the routing guard takes the reader's answer as yes | `face::a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound` |
| `S41013-ONLY-A-VIDEO-NAME-IS-ROUTED` | `MUTATIONS` | engine-core `src/media.rs` | the routing guard takes the table lookup as yes, so `beep.mp3` is routed and refused | `face::a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks` |
| `S41014-THE-ANSWER-SKIPS-THE-QUESTION-VIDEOS-WITH-THE-PRESET` | `MUTATIONS` | engine-core `src/face.rs` | the answer side keeps the question's videos when the preset skips the question | `face::the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips` |
| `S41015-THE-QUESTION-VIDEOS-COME-FIRST-ON-THE-ANSWER` | `MUTATIONS` | engine-core `src/face.rs` | the answer side writes its own videos before the question's | `face::the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips` |

The routing guard is a conjunction of two operands, so S41012 and S41013 each make one operand alone decide
it. Every existing row on `crates/engine-core/src/face.rs`, `crates/engine-core/src/media.rs` and
`crates/ffi/src/face.rs` (40 at DEV) is re-counted and re-killed after the change.

## 9. The fixture

`crates/ffi/tests/support/sound_video.rs` follows `crates/ffi/tests/support/parity.rs`: it builds every file
from fixed bytes, commits no binary, and fixes its card's id.

| item | value |
|---|---|
| deck | `Sound video` |
| note type | the stock `Basic` |
| front | `[sound:sign.mp4][sound:beep.mp3][sound:wave.m4v]` |
| back | `[sound:turn.MOV]` |
| `sign.mp4` | the bytes `sign-video`, base64 `c2lnbi12aWRlbw==` |
| `beep.mp3` | the bytes `beep-sound`, base64 `YmVlcC1zb3VuZA==` |
| `wave.m4v` | the bytes `wave-video`, base64 `d2F2ZS12aWRlbw==` |
| `turn.MOV` | the bytes `turn-video`, base64 `dHVybi12aWRlbw==` |
| card id | `3_000_001`, fixed as `parity.rs` fixes its own, one row checked |
| written to | `<output>/sound-video` by `crates/ffi/examples/review-fixture.rs` |

The four base names differ, because the simulator's file system ignores case.

## 10. Amendments this delivery appends

Each block below is appended, every line strictly between its BEGIN and END lines, at the end of the file
its BEGIN line names, after one blank line.

BEGIN docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md
## 12. Amendments: a sound tag that names a video plays as video (SPEC-410, ADR-424)

SPEC-410 builds the gap section 11's last line leaves open. This section is the pointer, so no rule above is
read without it:

- R4 holds for every reader that does not ask. For a reader that asks (`Reader::shows_video`, which the
  native client's reader answers `true`), a `[sound:name]` tag whose extension is in `VIDEO_TYPES` (`mp4`,
  `m4v`, `mov`, in any ASCII case) becomes no clip: the core appends a `data:` video element with native
  controls to the face's text, in the side's replay order, under R3's rules and caps (ADR-424 D1, D2).
- Such a video never autoplays and is not replayed: the learner starts it from its controls (ADR-424 D2).
- Section 11's last line, that a sound tag naming a video still plays as a sound, holds now only for a
  reader that does not ask.
END

BEGIN docs/decisions/ADR-359-the-review-screens-media-face-voice-choice-deck-tree-rating-and-test-seam.md
## Amendment: a sound tag that names a video (SPEC-410, ADR-424)

- D1 gains a third closed table, `VIDEO_TYPES`, read only for a reader that asks (ADR-424 D1, which names
  the options it was chosen against). A sound tag naming a video crosses as a `data:` video element inside
  the document, not as clip bytes, under D1's caps, unchanged (ADR-424 D2).
- D7 stands: the element carries no `autoplay`, so nothing starts it but the learner.
END

BEGIN docs/decisions/ADR-407-the-core-carries-the-ordinal-the-fonts-and-the-voices-and-the-card-view-plays-video-inline.md
## Amendment: the cut gap is built (SPEC-410, ADR-424)

- D5's cut gap, a sound tag that names a video, is built by SPEC-410 as ADR-424 D1 decides: an element the
  core writes for a reader that asks, played inline by D4's factory setting, contained by D6's `data:` route.
- SPEC-393 section 9's first follow-up is built except its replay: Replay and Stop do not act on such a video
  (#632).
- The last bullet of the Consequences, that such a tag keeps playing as a sound, no longer holds for the
  native client.
END

## 11. Formal

NOT APPLICABLE by surface (ADR-424 D4). No actor, timer, lock, static, task or write path is added:
`VIDEO_TYPES` is a constant, `Reader::shows_video` a pure answer, and the routing and the element are total
functions inside one face's completion, whose budget lives and dies inside one `complete` call. Of the 246
cover lines under `formal/` at DEV, three name `crates/web-engine/src/wasm.rs` and one names
`crates/ffi/src/engine.rs`; this delivery edits neither, and no cover line names a file it edits.

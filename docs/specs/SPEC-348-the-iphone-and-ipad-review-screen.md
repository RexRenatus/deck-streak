# SPEC-348: the iPhone and iPad review screen shows a card in the isolated web view, answers it from the bottom of the screen, and speaks it

- **Wave:** the app campaign, Phase 1 (SPEC-334 row 1.5; R4, R6). **Issue:** #632. **Context(s):**
  `engine-core` (`crates/engine-core`), `ffi` (`crates/ffi`), the iPhone and iPad app (`ios/`).
- **Decided by:** ADR-359 (this SPEC's own: D1 media reaches the frame inside the document, D2 a
  face crosses as one call, D3 the voice choice is kept by the engine adapter on the device, D4 the
  deck tree is decoded by the hand codec, D5 the rating picks its state in the codec, D6 the UI
  tests reach a seeded collection through one launch argument), ADR-335 (a native client per
  platform, card faces in an isolated web view, macOS CI), ADR-342 (bottom answer buttons on both
  devices, the remote's actions), ADR-352 (card scripts off on both platforms; the iPhone and iPad
  layers), and ADR-358 (the app shell, #625: the hand codec, the lexical census, one collection).
- **Schematic:** `docs/schematics/ios-review-screen.md` (this delivery adds it).
- **Status:** two pull requests (section 7). Part 1 is Rust and runs on Linux CI; Part 2 is Swift
  and runs on the Apple job. **Mutation band:** `S34800-S34899`. **Changelog:** `changelog.d/ios-review-348.md`.

Refs read: `$R` is the DeckStreak repository, `DEV` is `ac4fbdaeb02f682991a9b26db539dbc20c8ff559`,
`ENG` is #662's head `af6da34688467f8866167c4affbb53a45cca9d8b` (SPEC-345, the engine core), `$E`
is the engine fork's object store and `$PIN` is `c538de55a23e695234e794029fce0dafff2d36a9`, the
engine commit the workspace pins (root `Cargo.toml`, the `[patch]` table).

Part 1 was built at a later dev, after #662 and #669 landed. It re-read at its cut every figure it
relies on, and the figures below stay as read at DEV.

## 1. The problem, measured

### 1.1 What `ios/` and the card frame hold

| # | measured | figure | command |
|---|---|---|---|
| M1 | the `ios/` tree | 21 files: the harness (`ios/Harness/…`, seven sources, `ios/project.yml`), its tests, the `EnginePackage` and `HarnessWire` packages. No app target, no review screen | `git -C $R ls-tree -r --name-only DEV -- ios` |
| M2 | the harness's card view | `ios/Harness/Sources/CardWebView.swift` (84 lines) sets four layers: a non-persistent store, page JavaScript off, a compiled rule list blocking `.*` (`card-blocks-every-load`), no message handler; the HTML loads through `loadHTMLString(_, baseURL: nil)` after the list is installed. It sets no navigation delegate and no UI delegate | `git -C $R show DEV:ios/Harness/Sources/CardWebView.swift \| grep -n -E 'nonPersistent\|allowsContentJavaScript\|url-filter\|loadHTMLString\|navigationDelegate\|uiDelegate'` |
| M3 | what tests hold it | `ios/HarnessTests/CardWebViewTests.swift` (27 lines) asserts two of the four: the store is not persistent and page JavaScript is off | `git -C $R show DEV:ios/HarnessTests/CardWebViewTests.swift \| cat -n` |
| M4 | what SPEC-341 proved | the web frame alone. Its section 5 says it does not render the iPhone and iPad card view, and its findings table leaves the iPhone and iPad half of SEC01-F14 and SEC01-F15 to the iPhone and iPad delivery (lines 132 and 133). Its frame admits `data:` for images, media and fonts only (R2) | `git -C $R grep -n -E 'iPhone and iPad\|img-src data' DEV -- docs/specs/SPEC-341-*.md` |
| M5 | what ADR-352 settles for the phone | D1 (lines 32 to 58): page JavaScript off on both platforms. D7 (lines 121 to 138): the iPhone and iPad layers L1 to L7 are the iPhone and iPad delivery's. Consequences: a card's media reaches the frame as `data:` until the study screens decide a media path, and any other media path re-runs the planted suite | `git -C $R show DEV:docs/decisions/ADR-352-*.md \| sed -n '32,58p;121,138p'` |
| M6 | the iPhone and iPad layers | `docs/schematics/card-frame-channels.md` section 4 (lines 141 to 201): L1 to L7 (non-persistent store, page JavaScript off, the rule list, no message handler, the navigation gate, the window refusal, a string with no base URL; lines 147 to 155) and one constructor, `CardWebViewFactory.makeCardWebView(html)` (the diagram, lines 57 to 74) | `git -C $R show DEV:docs/schematics/card-frame-channels.md \| sed -n '57,74p;141,201p'` |
| M7 | the iPhone and iPad half of the sandbox | designed, not built: no `CardIsolation` path under `ios/` at DEV, and #619 is open | `git -C $R ls-tree -r --name-only DEV -- ios \| grep -c CardIsolation` (0); `gh issue view 619 --json state` |
| M8 | a `data:` load under the rule list | WebKit's content-rule backend returns before matching any rule for a `data:` URL (`ContentExtensionsBackend::actionsForResourceLoad`, the `protocolIsData()` check), so a document's own `data:` media is not a load the `.*` rule sees; the iPhone and iPad half of #619 renders a `data:` image under all seven layers as its render proof | WebKit source, `Source/WebCore/contentextensions/ContentExtensionsBackend.cpp`; the #619 iPhone and iPad design's R8 |

### 1.2 The engine surface a review needs

| # | measured | figure | command |
|---|---|---|---|
| M9 | the native allow-list | `[Call; 6]` (lines 23 to 56): (3,0) open, (7,13) deck names, (13,3) queued cards, (13,4) answer, (3,8) undo, (27,6) render | `git -C $R show DEV:crates/ffi/src/allow_list.rs \| grep -n -E 'ALLOW_LIST\|Call \{'` |
| M10 | the core's table | `ORDINARY: [Ordinary; 10]` (line 113), whose native column is M9's six; `EXEMPT: [Exempt; 6]` (line 190); `decide` (line 239). A parity test holds the adapter's list equal to the native column | `git -C $R show ENG:crates/engine-core/src/table.rs \| grep -n -E 'ORDINARY\|EXEMPT\|fn decide'` |
| M11 | who adds pairs before this | the app shell's first part adds (1,3) alone (SPEC-347 R1); #624 adds none ("Nothing else changes", SPEC-346 R7) | the #625 and #624 designs, requirement lines |
| M12 | the queue and its states | `QueuedCards {cards 1, new_count 2, learning_count 3, review_count 4}`; `QueuedCard {card 1, queue 2, states 3, context 4}`; `SchedulingStates {current 1, again 2, hard 3, good 4, easy 5}` | `git -C $E show $PIN:proto/anki/scheduler.proto \| sed -n '134,150p;264,272p'` |
| M13 | the interval labels | `DescribeNextStates(SchedulingStates) returns (generic.StringList)` (line 43): pair (13,24). The backend service's three methods come first (lines 71 to 77), then the scheduler service's in order | `git -C $E show $PIN:proto/anki/scheduler.proto \| grep -n -E 'service \|rpc '` |
| M14 | the deck calls | `DeckTree(DeckTreeRequest {now 1}) returns (DeckTreeNode)` (line 18): pair (7,4); `SetCurrentDeck(DeckId)` (line 38): pair (7,22); the backend decks service is empty (line 44). `DeckTreeNode` is recursive: `deck_id 1, name 2, level 4, collapsed 5, review_count 6, learn_count 7, new_count 8`, …, `children` | `git -C $E show $PIN:proto/anki/decks.proto \| grep -n -E 'service \|rpc '`; `… \| sed -n '152,180p'` |
| M15 | the face's parts | `RenderExistingCardRequest {card_id 1, browser 2, partial_render 3}` (lines 94 to 101); `RenderCardResponse {question_nodes, answer_nodes, css, latex_svg, is_empty}`; `AVTag {sound_or_video \| tts}`, `TTSTag {field_text, lang, voices, speed, other_args}` (lines 43 to 66) | `git -C $E show $PIN:proto/anki/card_rendering.proto \| sed -n '43,66p;94,101p;125,144p'` |
| M16 | how Anki joins a face | it renders partially, completes the question, extracts the question's AV tags, puts the question's EXTRACTED text in `{{FrontSide}}`, then extracts the answer's, so a front sound is not an answer clip (`pylib/anki/template.py` lines 236 to 241, 274, 323 to 325). Replay on the answer side plays the question's clips first unless the preset skips them (`qt/aqt/reviewer.py` lines 72 to 79); autoplay plays the shown side's clips (lines 378 to 380, 468 to 469) | `git -C $E show $PIN:pylib/anki/template.py \| sed -n '224,241p;311,335p'`; `git -C $E show $PIN:qt/aqt/reviewer.py \| sed -n '72,81p'` |
| M17 | the engine's helpers | `card_rendering::extract_av_tags(text, question_side, tr)` and `strip_av_tags` are public (lines 13 to 27); `text::replace_media_refs(text, replacer)` is public (line 315) and rewrites the `src`/`data` of `img`, `audio`, `video`, `object`, `source` and the AV tags; `text::html_to_text_line` is public (line 213) | `git -C $E show $PIN:rslib/src/card_rendering/mod.rs \| sed -n '13,27p'`; `git -C $E show $PIN:rslib/src/text.rs \| grep -n -E '^pub (fn\|static)'` |
| M18 | the preset's audio settings | `DeckConfig.Config`: `disable_autoplay = 23` (line 172), `skip_question_when_replaying_answer = 26` (line 181) | `git -C $E show $PIN:proto/anki/deck_config.proto \| grep -n -E 'autoplay\|skip_question'` |
| M19 | the media folder | `OpenCollectionRequest {collection_path 1, media_folder_path 2, media_db_path 3}` (lines 43 to 47); the harness's session opens with a `collection.media` folder beside the collection | `git -C $E show $PIN:proto/anki/collection.proto \| sed -n '43,47p'`; `git -C $R show DEV:ios/Harness/Sources/EngineSession.swift \| grep -n media` |
| M20 | the Swift codec | `Messages.swift` (194 lines): `QueuedCard` (line 102) keeps the current and good states only; `Responses` (line 140) decodes deck names, the queue, a queued card and the question's nodes; `swift-mutants.json` rows are `{id, file, find, replace, killer, why}` | `git -C $R show DEV:ios/HarnessWire/Sources/HarnessWire/Messages.swift \| grep -n -E 'struct\|enum\|static func'` |
| M21 | the core's dependencies | `anki`, `anki_proto`, `prost`, `serde_json`; no base64 encoder is a workspace dependency, and the lockfile resolves `base64` already through the engine's HTTP client | `git -C $R show ENG:crates/engine-core/Cargo.toml`; `git -C $R show DEV:Cargo.lock \| grep -n -A1 '^name = "base64"$'` |

### 1.3 What the platform requires

| # | rule | source |
|---|---|---|
| P1 | `AVSpeechSynthesisVoice.speechVoices()` lists the voices installed on the device, each with an identifier, a name, a BCP 47 language and a quality (default, enhanced, premium); a simulator may list none | Apple, AVSpeechSynthesisVoice |
| P2 | `AVSpeechSynthesisVoice(identifier:)` returns nil for a voice that is not installed; `AVSpeechSynthesisVoice(language:)` picks the system's voice for a language | Apple, AVSpeechSynthesisVoice |
| P3 | `AVSpeechUtterance.rate` runs from `AVSpeechUtteranceMinimumSpeechRate` (0) to `AVSpeechUtteranceMaximumSpeechRate` (1), default `AVSpeechUtteranceDefaultSpeechRate` (0.5) | Apple, AVSpeechUtterance |
| P4 | spoken study audio uses the `.playback` category with the `.spokenAudio` mode; `.duckOthers` and `.interruptSpokenAudioAndMixWithOthers` lower and pause other spoken audio; the synthesizer uses the app's session by default | Apple, AVAudioSession |
| P5 | `AVAudioPlayer(data:)` plays audio held in memory | Apple, AVAudioPlayer |
| P6 | SwiftUI's `.sensoryFeedback(_:trigger:)` plays a haptic when a value changes, from the harness's deployment target (`ios/project.yml` lines 6, 7); the HIG asks for haptics used sparingly and always beside a visible change | Apple, sensoryFeedback; HIG, Playing haptics |
| P7 | `WKWebView.pageZoom` scales a page; `UIFontMetrics(forTextStyle:).scaledValue(for:)` gives the Dynamic Type scale | Apple, WKWebView; UIFontMetrics |
| P8 | `mediaTypesRequiringUserActionForPlayback` defaults to every type and `allowsInlineMediaPlayback` to false on iPhone; both are configuration, set before the view exists | Apple, WKWebViewConfiguration |
| P9 | WCAG 2.2: 2.5.3 the accessible name contains the visible label; 2.5.8 a target of 24 by 24 CSS px at least (Apple asks 44 pt); 1.4.2 audio that plays by itself can be stopped; 4.1.3 a status is announced; 1.4.4 text scales | W3C WCAG 2.2; Apple HIG, Layout |

### 1.4 What each machine can prove

| reading | Linux (no compiler) | the Apple job |
|---|---|---|
| the pairs, the face, the media, the voice store, the launch argument | `cargo test` over synthetic collections | — |
| the thin-Swift census over the new sources and roles | `scripts/tests/test_ios_thin_swift.py` | — |
| no message handler, scheme handler or file load anywhere under `ios/`; one web-view constructor | `scripts/tests/test_ios_review_screen.py` | — |
| the answer buttons' names and order; the bottom inset; the haptic triggers | `test_ios_review_screen.py` (a source reading) | the UI tests (geometry, the accessibility tree) |
| show, reveal, rate, next, finished; intervals; a `data:` image renders; one answer per tap | — | `harness` job, step "the review screen's tests, Debug, on the iPhone and then the iPad" |
| the codec's new lines | — | `harness-wire` job, steps "the codec's tests, on the host" and "the codec's mutants, each against its one killer" |

## 2. Requirements

Part 1, delivered by the first pull request:

R1. **Three pairs.** (7,4) `DecksService.DeckTree`, (7,22) `DecksService.SetCurrentDeck` and
    (13,24) `SchedulerService.DescribeNextStates` join the core's native column and the native
    adapter's allow-list (`crates/ffi/src/allow_list.rs`) in this delivery, so #623's parity test
    holds. They are `Ordinary` rows in the native column. The web column's set is unchanged by this
    delivery: whether the web admits them is the web study screens' decision (#630). No other pair
    joins the native column.
R2. **The face.** The core gains `Dispatcher::face(card, side, autoplay, media)`, where `side` is
    the question or the answer, `autoplay` is the client's wish, and `media` is a reader from a
    media file's name to its bytes. It renders the card partially through the engine, completes
    each side by joining its nodes (a replacement node by its current text), extracts the
    question's AV tags with the question side set, puts the question's extracted text in
    `{{FrontSide}}`, and extracts the answer's with the question side clear (M16). It returns a
    `Face`: the side's display text with every AV tag stripped and its media rewritten (R3), the
    note type's CSS, `autoplay`, `replay` and `omitted`. A card the engine cannot render returns
    the engine's refusal, as `run` does. The face is the same for both transports.
R3. **The media.** Every media reference `replace_media_refs` finds in the display text is
    rewritten to a `data:` URL of the file's bytes when the name, once its entities and percent
    escapes are decoded, is a plain file name (not empty, no `/`, `\` or NUL, not `.` or `..`),
    its extension is in the core's closed table of image and audio types (an mp4 is read as
    audio), the reader returns its bytes, the file is at most 4 MiB, and the face's total stays at
    most 16 MiB. Otherwise the attribute's value is emptied and the name joins `omitted`, once. The
    rewrite never produces a URL of another scheme. The two caps and the type table are the core's
    public constants, one copy both clients read.
R4. **The clips.** A `[sound:name]` tag whose type is audio becomes `Clip::Sound {name, bytes}`
    under R3's rules, and any other sound tag joins `omitted`. A TTS tag becomes
    `Clip::Speech {text, language, rate}`: its text as one plain line (the engine's
    `html_to_text_line`), its language with `_` written `-`, and the engine's speed times the
    platform's default rate, held between its minimum and maximum (P3). `autoplay` is the shown
    side's clips when the client wishes it and the card's preset does not disable autoplay (the
    card's original deck when it sits in a filtered deck); else empty. `replay` on the question is
    the question's clips; on the answer, the question's clips then the answer's, unless the preset
    skips the question.
R5. **The native face.** The native `Engine` exports `face(card_id, answer, night, autoplay)`,
    which reads media from the media folder of the collection the engine opened (the open
    request's `media_folder_path`, kept when (3,0) succeeds), and returns a `CardFace`: the
    `Face`'s clips and `omitted`, and `document`, one HTML document holding a viewport meta element,
    one `style` element with the CSS, and a `body` whose classes are `card`, plus `nightMode` and
    `night_mode` when `night` is set. It names no script, no `base` element and no URL that is not
    `data:`. Its refusal is `EngineRefusal`, text unchanged.
R6. **The voice choice.** The native adapter exports `VoiceChoices`, opened on one file path:
    `chosen(language, installed)` returns the identifier chosen for that language when it is among
    the installed voices, else nothing; `options(language, installed)` returns the installed voices
    whose language equals it, or, when none does, those sharing its primary subtag, ordered by
    quality (premium first) then name; `choose(language, identifier)` records or, given nothing,
    clears the choice. The file holds one `language<TAB>identifier` line per language, is written
    whole to a temporary file and renamed, and a line that does not parse is skipped. The file is
    never inside the collection or its media folder, and nothing of it is synced.
R7. **The test seam.** The native adapter exports `collection_directory(fallback, arguments)`: it
    returns `fallback` when no `-DSCollectionDirectory` argument is given, and the value after that
    argument when it is an absolute path to an existing directory. Every other value is refused by
    name, as a `CollectionDirectoryRefusal`: no value after the argument (`NoValue`), a path that is
    not absolute (`NotAbsolute`), a path that does not exist (`Missing`), or a path that is not a
    directory (`NotADirectory`).
R8. **A review fixture.** `crates/ffi/examples/review-fixture.rs` writes, with this commit's
    engine, a collection and its media folder: a deck `Review` holding a text card, a card whose
    front shows a 64 by 64 pixel PNG with `alt` text, a card whose front plays a short WAV, and a
    card whose front speaks through a TTS tag, all new, on the default preset, every answer
    template opening with `{{FrontSide}}`. The Apple job's `xcframework` job writes and uploads it beside the
    harness's.

Part 2, delivered by the next pull request (section 7):

R9. **The review screen.** Choosing a deck in the sidebar calls (7,22) for it and shows the review
    screen in the detail pane in place of "Choose a deck". The screen shows, top to bottom: the
    deck's name and its new, learning and review counts from the queue (M12), each a number with
    its word, never a colour alone; the card; and a bottom bar inside the safe area. The card is
    the `CardFace.document` in the view `CardWebViewFactory.makeCardWebView(html:)` returns; a new
    view per face, never a reload, so the navigation gate stays one-shot; its `pageZoom` is the
    body text style's Dynamic Type scale. A face with omitted media shows one line naming how many
    files could not be shown.
R10. **The loop.** Next: (13,3) with a fetch limit of one; no card shows the designed end,
     "Congratulations! You have finished this deck for now.", with a button back to the deck list.
     Show: the question face. Reveal: "Show Answer", one button the width of the bar, where Good
     sits, shows the answer face and the four ratings. Rate: Again, Hard, Good and Easy in that
     order, Good the visually primary one; a tap sends (13,4) with the rating's own state from the
     queued card's states and the time the card was shown; then next. While a call runs the bar's
     buttons are disabled, and the session answers a shown card at most once.
R11. **The intervals.** Each rating button shows, under its title, the interval (13,24) gives for
     its state (STAT-01 is signed). The title is the button's accessible name and the interval its
     accessible value; every target is at least 44 by 44 pt.
R12. **Haptics.** One impact haptic when a rating is sent and one success haptic when the designed
     end shows, through `.sensoryFeedback`; none on Show Answer, none on a refusal.
R13. **Speech and sound.** The session's audio category is `.playback` with the `.spokenAudio`
     mode and `.duckOthers` and `.interruptSpokenAudioAndMixWithOthers`. A face's `autoplay` clips
     play in order when it shows: a sound through `AVAudioPlayer(data:)`, speech through one
     `AVSpeechSynthesizer` with the voice `VoiceChoices.chosen` names, or the system's voice for
     the language. `autoplay` is asked for only while VoiceOver is not running. A replay button
     plays `replay`, a stop button stops whatever plays, and showing another face stops it too.
R14. **The voice picker.** A sheet from the review screen lists, for each language the current
     face speaks, "System default" and then `VoiceChoices.options`, each voice by its name and its
     quality in words; choosing one calls `choose` and is marked as chosen. With no installed voice
     for a language, the sheet says so in a sentence.
R15. **The deck counts.** The sidebar's rows come from (7,4) with `now` set: each deck's name,
     indented by its level, with its new, learning and review counts as numbers with words; the
     codec decodes the tree depth first, refuses a tree deeper than 32 levels, and skips the
     root. (7,13) leaves the app's session.
R16. **The seams #633 needs.** Every gesture on the review screen goes through one entry,
     `ReviewModel.perform(_ action: ReviewAction)`, whose cases are show answer, rate with a
     rating, replay and stop; the review's state lives in the model, never a view's `@State`, so it
     survives a size-class change; the bar fills the detail pane's width.
R17. **Swift stays thin.** The thin-Swift census (SPEC-347 R11) gains two roles: `card` (the one
     file that holds the factory's view; it alone of the app's roles imports `WebKit`) and `speech`
     (the clip player and the voice picker's reader; they alone import `AVFoundation`), with
     ceilings `card` 2 and `speech` 5. `CardIsolation`'s files are registered as `isolation`, bound
     like the harness's by the closed register and the forbidden names. Every new file is
     registered with its exact decision count. No ceiling rises.
R18. **The frame stays as #619 proved it.** Under `ios/`, no file outside `CardIsolation`
     constructs a `WKWebView` or a `WKWebViewConfiguration`; no file names
     `WKScriptMessageHandler`, `addScriptMessageHandler`, `userContentController`,
     `WKURLSchemeHandler`, `setURLSchemeHandler`, `loadFileURL` or `allowFileAccess`; and the card
     role calls `makeCardWebView(html:` and sets nothing on a configuration.
R19. **The codec grows.** `HarnessWire` gains: the queued card's five states as opaque bytes and
     `state(for: Rating)`; `Requests.describeNextStates(_:)`, `Requests.deckTree(now:)` and
     `Requests.setCurrentDeck(_:)`; `Responses.stringList(_:)` and `Responses.deckTree(_:)`
     (depth first, R15's bound). Each is pinned by literal bytes, a two-level tree included, and
     carries mutant rows.
R20. **CI.** The Apple job's `harness` job gains one step, "the review screen's tests, Debug, on
     the iPhone and then the iPad", that runs `DeckStreakTests/Review*` and
     `DeckStreakUITests/ReviewFlowTests` with the review fixture placed in the runner's temporary
     directory and named to the test runner; and its report gains that step's row. No job,
     workflow, runner or required context is added.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | The three pairs run on a native dispatcher, every other unlisted pair stays refused on it, and (13,24) describes a new card on the default preset as four literal intervals the test pins | `cargo test -p deck-streak-engine-core --test review_pairs -- --exact the_review_pairs_run_natively_and_no_other_pair_joins` |
| A2 | The adapter's allow-list equals the core's native column: the six at DEV, the shell's login (1,3) and these three | `cargo test -p deck-streak-ffi --test review_pairs -- --exact the_allow_list_carries_the_review_pairs` |
| A3 | On a fixture card whose front plays a sound and whose answer template holds `{{FrontSide}}`, the answer's autoplay is the answer's clip alone and its replay is the question's then the answer's; with the preset's skip set, the answer's only | `cargo test -p deck-streak-engine-core --test face -- --exact the_answer_replays_the_question_unless_the_preset_skips_it` |
| A4 | A face's display text holds no AV tag and no `[anki:play`, and its image's `src` is the `data:` URL of the file's bytes | `cargo test -p deck-streak-engine-core --test face -- --exact a_face_inlines_its_image_and_strips_its_av_tags` |
| A5 | A name that is a path, `..`, empty, of an unknown type, missing, over the file cap or past the face cap is omitted once and its attribute emptied; no output URL is other than `data:` | `cargo test -p deck-streak-engine-core --test face -- --exact a_reference_the_rules_refuse_is_omitted_and_emptied` |
| A6 | A TTS tag becomes speech with its plain text, its language with a hyphen and its rate scaled and held | `cargo test -p deck-streak-engine-core --test face -- --exact a_tts_tag_becomes_speech_in_the_platforms_terms` |
| A7 | A preset with autoplay disabled, or a client that does not wish it, gets an empty autoplay and a full replay | `cargo test -p deck-streak-engine-core --test face -- --exact autoplay_follows_the_preset_and_the_client` |
| A8 | The native face's document inlines the image from the opened collection's media folder, names no script, no base and no non-`data:` URL, and carries the night classes only when asked | `cargo test -p deck-streak-ffi --test face -- --exact the_native_document_is_one_closed_page` |
| A9 | Voice choices persist across reopening, `chosen` forgets a voice no longer installed, `options` falls back to the primary subtag and orders by quality then name, and a damaged line is skipped | `cargo test -p deck-streak-ffi --test voices -- --exact a_voice_choice_survives_and_follows_the_installed_set` |
| A10 | The collection directory is the default with no argument; the argument's when an absolute path to an existing directory; every other value refused by name, each by its own test | `cargo test -p deck-streak-ffi --test seam` |
| A11 | The review fixture writes a collection whose deck `Review` holds the four cards and whose media folder holds the PNG and the WAV | `cargo test -p deck-streak-ffi --test fixture -- --exact the_review_fixture_holds_four_cards_and_two_files` |
| A12 | Every Swift file under `ios/` is registered, the `card` and `speech` doors hold, and each new file's decisions equal the register and sit under its ceiling; planted breaks are refused by name | `python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py` |
| A13 | No message handler, scheme handler, file load or second web-view constructor exists under `ios/`; the card role calls the factory and configures nothing; planted breaks are refused by name | `python3 -m unittest discover -s scripts/tests -p test_ios_review_screen.py -k the_card_frame_is_the_factorys_alone` |
| A14 | The bar names Again, Hard, Good and Easy in order with no `.accessibilityLabel` override, sits in a bottom safe-area inset, and declares one impact and one success `.sensoryFeedback` | `python3 -m unittest discover -s scripts/tests -p test_ios_review_screen.py -k the_answer_bar_is_named_placed_and_felt` |
| A15 | On the fixture, the model goes question, answer, rating, next, through four cards to the designed end | `harness` job, step "the review screen's tests, Debug, on the iPhone and then the iPad": `DeckStreakTests/ReviewModelTests/test_a15_the_loop_reaches_the_designed_end` |
| A16 | Two taps on one rating send one answer: the session's count of sent answers is one, and the next shown card is the fixture's second | the same step: `DeckStreakTests/ReviewSessionTests/test_a16_a_double_tap_answers_once` |
| A17 | The four rating buttons carry, in order, the literal intervals A1 pins for a new card | the same step: `DeckStreakTests/ReviewSessionTests/test_a17_each_rating_shows_its_interval` |
| A18 | The sound card's autoplay plan is its clip, empty while VoiceOver runs, and the speech card's voice falls back to the language's when the chosen one is absent | the same step: `DeckStreakTests/ReviewModelTests/test_a18_the_clip_plan_follows_the_face_and_voiceover` |
| A19 | On both simulators the four rating buttons sit in the bottom fifth of the window, each at least 44 by 44 pt, their names the titles and their values the intervals | the same step: `DeckStreakUITests/ReviewFlowTests/test_a19_the_ratings_sit_at_the_bottom_named_by_their_titles` |
| A20 | Choosing the fixture's deck shows its counts and the first card; Show Answer reveals the four ratings; Good shows the next card | the same step: `DeckStreakUITests/ReviewFlowTests/test_a20_show_reveal_rate_next` |
| A21 | The image card renders its `data:` image in the factory's view: the image element named by its `alt` text is the fixture PNG's size, not a broken image's | the same step: `DeckStreakUITests/ReviewFlowTests/test_a21_the_image_card_renders_its_image` |
| A22 | The voice picker lists "System default" and says when no voice is installed | the same step: `DeckStreakUITests/ReviewFlowTests/test_a22_the_voice_picker_names_the_default` |
| A23 | The codec's new requests and decoders equal their literal bytes: the current deck, the states sent to be described, each rating's own state, the intervals and the queued card's five states | `harness-wire` job, step "the codec's tests, on the host": `HarnessWireTests.RequestBytesTests/test_a23_review_requests_encode_to_their_literal_bytes`, `HarnessWireTests.ResponseDecodingTests/test_a23_the_intervals_and_the_five_states_decode` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test review_pairs -- --exact the_review_pairs_run_natively_and_no_other_pair_joins
A2: cargo test -p deck-streak-ffi --test review_pairs -- --exact the_allow_list_carries_the_review_pairs
A3: cargo test -p deck-streak-engine-core --test face -- --exact the_answer_replays_the_question_unless_the_preset_skips_it
A4: cargo test -p deck-streak-engine-core --test face -- --exact a_face_inlines_its_image_and_strips_its_av_tags
A5: cargo test -p deck-streak-engine-core --test face -- --exact a_reference_the_rules_refuse_is_omitted_and_emptied
A6: cargo test -p deck-streak-engine-core --test face -- --exact a_tts_tag_becomes_speech_in_the_platforms_terms
A7: cargo test -p deck-streak-engine-core --test face -- --exact autoplay_follows_the_preset_and_the_client
A8: cargo test -p deck-streak-ffi --test face -- --exact the_native_document_is_one_closed_page
A9: cargo test -p deck-streak-ffi --test voices -- --exact a_voice_choice_survives_and_follows_the_installed_set
A10: cargo test -p deck-streak-ffi --test seam
A11: cargo test -p deck-streak-ffi --test fixture -- --exact the_review_fixture_holds_four_cards_and_two_files
A12: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
A13: python3 -m unittest discover -s scripts/tests -p test_ios_review_screen.py -k the_card_frame_is_the_factorys_alone
A14: python3 -m unittest discover -s scripts/tests -p test_ios_review_screen.py -k the_answer_bar_is_named_placed_and_felt
A15: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a15_the_loop_reaches_the_designed_end
A16: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a16_a_double_tap_answers_once
A17: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a17_each_rating_shows_its_interval
A18: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a18_the_clip_plan_follows_the_face_and_voiceover
A19: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_a19_the_ratings_sit_at_the_bottom_named_by_their_titles
A20: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_a20_show_reveal_rate_next
A21: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_a21_the_image_card_renders_its_image
A22: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_a22_the_voice_picker_names_the_default
A23: swift test --package-path ios/HarnessWire --filter HarnessWireTests.RequestBytesTests/test_a23_review_requests_encode_to_their_literal_bytes
A23: swift test --package-path ios/HarnessWire --filter HarnessWireTests.ResponseDecodingTests/test_a23_the_intervals_and_the_five_states_decode
```

**The red each shows first.** A1 and A2 are red at their tests commit on the refusal of (7,4): the
pair is `NotAllowed`. A3 to A7 are red over a `face` that returns the joined text unchanged with
empty clips: A3 on the answer's replay (empty, expected two clips), A4 on the image's `src` (the
file name, expected `data:image/png;base64,…` of the fixture's bytes), A5 on the planted path
name's attribute (unchanged), A6 on the speech clip (absent), A7 on the full replay (empty). A8 is
red over a native face whose reader finds no file: the image's `src` is emptied, not `data:`. A9 is
red over a stub that returns nothing; A10 over a stub that returns the default for every argument,
so its refusal tests and its directory test fail. A11 is red over a fixture builder that writes
nothing. A5's plants and A9's damaged line are positive controls: each
names what must be refused, so a rule that went blind fails.

A12 to A14 are red at their tests commit, before the Swift: A12 on its positive artifacts (the
register's `card` and `speech` files, absent), A13 on the card role's factory call (absent), A14
on the bar's four titles (absent). A15 to A22 are red over a `ReviewModel` whose `perform` does
nothing, each on its first positive assertion. A23 is red over decoders that return empty values.
Every Swift criterion runs on the Apple job, never on the Linux box.

## 4. File manifest

| path | context | for |
|---|---|---|
| `crates/engine-core/src/table.rs` | engine-core | R1: three `Ordinary` rows |
| `crates/engine-core/src/face.rs` | engine-core | R2 to R4: `Face`, `Side`, `Clip`, the join, the clips |
| `crates/engine-core/src/media.rs` | engine-core | R3: the name rule, the type table, the caps, the encoder |
| `crates/engine-core/src/dispatch.rs` | engine-core | R2, R5: `Dispatcher::face`; the media folder kept at (3,0) |
| `crates/engine-core/src/lib.rs` | engine-core | the new modules' exports |
| `crates/engine-core/Cargo.toml` | engine-core | `base64`, as the workspace declares it (ADR-359 D1) |
| `Cargo.toml` | workspace | `base64` in `[workspace.dependencies]`, the version the lockfile already resolves |
| `Cargo.lock` | workspace | the core's dependency list gains base64; no new package |
| `crates/engine-core/tests/review_pairs.rs`, `crates/engine-core/tests/face.rs` | engine-core (test) | A1, A3 to A7 |
| `crates/engine-core/tests/table.rs` | engine-core (test) | R1: the native census gains the three pairs |
| `crates/ffi/src/allow_list.rs` | ffi | R1 |
| `crates/ffi/src/engine.rs` | ffi | R5, R7: `face`, `collection_directory` |
| `crates/ffi/src/face.rs` | ffi | R5: `CardFace`, `Clip`, the document |
| `crates/ffi/src/voices.rs` | ffi | R6: `VoiceChoices`, `Voice` |
| `crates/ffi/src/lib.rs` | ffi | the new modules |
| `crates/ffi/examples/review-fixture.rs` | ffi | R8 |
| `crates/ffi/tests/review_pairs.rs`, `crates/ffi/tests/face.rs`, `crates/ffi/tests/voices.rs`, `crates/ffi/tests/seam.rs`, `crates/ffi/tests/fixture.rs`, `crates/ffi/tests/refusal_text.rs` | ffi (test) | A2, A8 to A11 |
| `crates/ffi/tests/support/review.rs` | ffi (test) | R8: the fixture's builder, shared by the example, A8 and A11 |
| `.github/workflows/xcframework.yml` | CI | R8: the fixture written and uploaded; R20: the step and its report row |
| `scripts/mutation-rows.d/S34800-S34899.json` | mutation | section 8 |
| `docs/schematics/ios-review-screen.md` | docs | the schematic |
| `docs/decisions/ADR-359-*.md`, `docs/specs/SPEC-348-*.md`, `docs/red-first/SPEC-348.md`, `changelog.d/ios-review-348.md` | docs | the delivery's records |
| `ios/App/Sources/ReviewModel.swift` | `ios/` (model) | R9 to R12, R16 |
| `ios/App/Sources/ReviewSession.swift` | `ios/` (session) | R9 to R11, R15: the review's engine calls |
| `ios/App/Sources/ReviewView.swift`, `ios/App/Sources/AnswerBar.swift`, `ios/App/Sources/ReviewChrome.swift`, `ios/App/Sources/VoicePickerView.swift` | `ios/` (view) | R9 to R14 |
| `ios/App/Sources/CardFaceView.swift` | `ios/` (card) | R9, R18 |
| `ios/App/Sources/ClipPlayer.swift`, `ios/App/Sources/InstalledVoices.swift` | `ios/` (speech) | R13, R14 |
| `ios/App/Sources/DeckListView.swift`, `ios/App/Sources/AppModel.swift`, `ios/App/Sources/EngineSession.swift` | `ios/` (#625's, amended) | R9, R15, R7's argument |
| `ios/HarnessWire/Sources/HarnessWire/Messages.swift` | `ios/` (wire) | R19 |
| `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift`, `…/ResponseDecodingTests.swift` | `ios/` (test) | R19 |
| `ios/HarnessWire/swift-mutants.json` | `ios/` | R19's rows |
| `ios/AppTests/ReviewModelTests.swift`, `ios/AppTests/ReviewSessionTests.swift` | `ios/` (test) | A15 to A18 |
| `ios/AppUITests/ReviewFlowTests.swift` | `ios/` (test) | A19 to A22 |
| `ios/swift-roles.json` | `ios/` | R17 |
| `scripts/tests/test_ios_thin_swift.py` | census | R17 |
| `scripts/tests/test_ios_review_screen.py` | census | R11, R12, R18 |

## 5. What this does NOT do

- It does not widen the card's isolation or change any file of `CardIsolation`: the layers, the
  navigation gate, the rule list and their planted suite are #619's, and a media path other than
  `data:` inside the document would re-run that suite there (#619).
- It runs no card script and adds no message handler: whether card scripts ever run is the owner's
  decision (#651).
- It builds no undo, no iPad-specific layout, no remote or keyboard mapping, no idle-timer rule,
  no sync screen and no media sync: those are the iPad, remote, sync and undo delivery's (#633),
  which this delivery hands `ReviewAction` and the model-held state (R16).
- It does not apply a template's `voices=` option or other TTS options, play a video AV tag, inline
  a font a template's CSS names by `url()`, add the `card<n>` body class, or play HTML `<video>`
  inline on iPhone: each is a parity gap the review-parity follow-up owns (#666).
- It adds no TestFlight upload, signing step, team id or app id: the lane is #634's, and the dev
  app id and the staging sync user reach internal builds there (#634).
- It does not build the web study screens, though the core's face is shared: #630 decides whether
  the web frame takes it (#630).
- It does not award XP for an answer (#639).
- It adds no proving of Swift mutation rows beyond the codec's existing sweep (#650).
- It is not the owner's device session or acceptance pass, where speech, haptics and a real voice
  list are heard and felt (#629, #637).
- It does not re-run the security review over the shipped review screen (#638).

## 6. Risks

- **A simulator lists no speech voice.** The picker's tests assert the sentence for no voice and
  the clip plan, never audio; the owner's device session hears the voices (#629).
- **A large media file makes a face heavy.** R3's caps bound it, and `omitted` says what was left
  out; the caps are an ADR-359 D1 number, and a measured need moves them.
- **The simulator refuses the UI test's fixture directory.** A20's first assertion is the seeded
  deck's row, a positive artifact, so a refused read fails loudly; the hosted tests (A15 to A18)
  still prove the loop in-process.
- **The engine's answer-side text carries the question's tags.** M16 says Anki completes the front
  first; A3 holds the core to it on a fixture, not on the reading.
- **The factory's API differs from its design.** Part 2 waits for #619's iPhone and iPad half to
  land and reads the factory there before its first test.

## 7. Delivered by the next pull request

Part 2's criteria, A12 to A23, now sit in section 3 with their lines in its fence (section 10).

## 8. Mutation rows

The band file `scripts/mutation-rows.d/S34800-S34899.json` holds `MUTATIONS` rows of seven cells for
Part 1's Rust (the six pair rows on the allow-list and the table, the name rule's three arms, the
two caps' comparisons and values, the face's running total, the omitted attribute, the type table,
the front-side join, the preset's skip and autoplay flags, the client's wish, the rate's hold, the
language's spelling, the strip, the media folder kept at the open, the document's night classes,
the voice file's choice, fallback, order and parse, and the argument's two checks), each killed by
one Part 1 test. Part 2's codec lines carry rows in
`ios/HarnessWire/swift-mutants.json`, swept by the `harness-wire` job; the census tests carry
`SCRIPT_MUTATIONS` rows in the same band.

## 9. Amendments: what part 1 leaves unchanged

- `ios/App/Sources/ReviewModel.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/ReviewSession.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/ReviewView.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/AnswerBar.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/ReviewChrome.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/VoicePickerView.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/CardFaceView.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/ClipPlayer.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/InstalledVoices.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/DeckListView.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/AppModel.swift`: unchanged in this part; delivered by part 2
- `ios/App/Sources/EngineSession.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/Sources/HarnessWire/Messages.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift`: unchanged in this part; delivered by part 2
- `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift` (section 4's `…/ResponseDecodingTests.swift`): unchanged in this part; delivered by part 2
- `ios/HarnessWire/swift-mutants.json`: unchanged in this part; delivered by part 2
- `ios/AppTests/ReviewModelTests.swift`: unchanged in this part; delivered by part 2
- `ios/AppTests/ReviewSessionTests.swift`: unchanged in this part; delivered by part 2
- `ios/AppUITests/ReviewFlowTests.swift`: unchanged in this part; delivered by part 2
- `ios/swift-roles.json`: unchanged in this part; delivered by part 2
- `scripts/tests/test_ios_thin_swift.py`: unchanged in this part; delivered by part 2
- `scripts/tests/test_ios_review_screen.py`: unchanged in this part; delivered by part 2

## 10. Amendments: what part 2 delivers

Part 2 delivers R9 to R20, and section 7's rows A12 to A23 now sit in section 3, with their lines
in its fence. It was cut at `dev` `f3392ec3`. Twelve points move from the text above; ADR-359 D8
and D9 decide the two that change a requirement, and the rest are named here.

- **R18's population (ADR-359 D9).** A13 judges every Swift file git tracks under `ios/` outside
  `ios/CardIsolation/` and outside a test target, as `scripts/tests/test_card_web_view_layers.py`
  names one (a directory under `ios/` named `Tests` or ending in `Tests`), and prints each path it
  skips with their count. The factory's own guard counts script message handlers, and the card
  probe's tests name R18's words to prove the factory refuses them; neither is a card frame the
  app builds.
- **R20's step and its place (ADR-359 D8).** "The review screen's tests, Debug, on the iPhone and
  then the iPad" sits between "the app's tests, Debug, on the iPhone and then the iPad" and "the
  app, archived unsigned for a device", with the id `review-tests`, and runs as the archive does:
  after a red app step too, and not when the run is cancelled or the app step never ran. It runs
  `-only-testing:` the three classes `DeckStreakTests/ReviewModelTests`,
  `DeckStreakTests/ReviewSessionTests` and `DeckStreakUITests/ReviewFlowTests` over the app step's
  derived data, into its own result bundle, and the app's step carries the three matching
  `-skip-testing:` lines, so each review test runs once. SPEC-347 A14's order rule holds the
  three app steps together between the harness's last step and the report. R20 is decided by the
  class `TheReviewScreenIsTestedOnBothSimulators` in `scripts/tests/test_ci_workflows.py`.
- **The fixture reaches the tests.** The step copies `review-fixture` from the run's artifact into
  the runner's temporary directory and names it on its command line as
  `TEST_RUNNER_DS_REVIEW_FIXTURE`, which the test process reads as `DS_REVIEW_FIXTURE`; each test
  copies it into a fresh directory, and the UI tests launch the app with `-DSCollectionDirectory`
  and that directory. A15 to A22's fence lines take SPEC-347's form; the step adds the fixture.
- **Signing.** The step signs ad hoc on its command line (`CODE_SIGN_IDENTITY=-`), as the app's
  step does (SPEC-347 section 10), in place of turning code signing off. No team and no signing
  setting enters a file.
- **The app's project.** `ios/app.yml` gains the `CardIsolation` package and the DeckStreak
  target's dependency on it, as `ios/project.yml` names them for the harness.
- **R7's parameter.** The function is `collection_directory(fallback, arguments)` (ADR-359 D6),
  `collectionDirectory(fallback:arguments:)` in Swift; the app passes the directory `open()` used
  before as `fallback`, and its own launch arguments.
- **R17's doors.** The census requires exactly one `card` file and at least one `speech` file.
  `AVFAudio` is a door beside `AVFoundation`, admitting `speech` files alone, because the player,
  the audio session and the synthesizer are declared in that module, and a census blind to it
  would be a hole. The planted view that imports `WebKit` is now refused as a file only a `card`
  file may be.
- **The codec grows by addition.** `QueuedCard` keeps `goodState` and its five-argument
  initialiser, which the harness and an existing mutant read; the other states are added beside
  it, and `state(for:)` reads each rating's own.
- **A sound the player refuses.** When `AVAudioPlayer(data:)` refuses a clip's bytes, the review
  screen shows one line naming the file, and the face's other clips play on. Swift holds no media
  cap, no type table and no list of sound types: the core's rules decide which files reach a clip
  (R3).
- **The factory as #682 leaves it.** Its layers are L1 to L8 and L10 to L13, and it prefixes its
  document policy ahead of the face's own doctype. The card view hands `CardFace.document` to it
  unchanged, so a face's page carries both doctypes, the policy's first.
- **The fragment.** This part's entry is `changelog.d/ios-review-screen-348.md`, a new file,
  because part 1's `changelog.d/ios-review-348.md` was folded into `CHANGELOG.md` by the release.
- **The deck counts move to a later part.** This part leaves the sidebar's counts to IOS-2c, as
  the design's own cut allows: R15, ADR-359 D4, A23's tree half (`Requests.deckTree(now:)`,
  `Responses.deckTree(_:)`, the two-level tree and the refusal past 32 levels) and the sidebar's
  change are unchanged in this part; delivered by IOS-2c. The sidebar keeps (7,13), whose
  `DeckName` already carries the id (7,22) needs. The review screen's three counts (R9) come from
  (13,3) and stay in this part. A23's second test is
  `test_a23_the_intervals_and_the_five_states_decode`, over the intervals' `StringList` and the
  queued card's five states, and A23's row names what this part's codec adds.
- **SPEC-339 A3's new card names all five states.** The queue's decoder now reads the again, hard
  and easy states beside the current and good ones, so `test_a3`'s new card, whose bytes carry all
  five, expects them; its learning card carries two and expects two.

Files this part changes that section 4 does not name:

- `ios/app.yml`: the `CardIsolation` package and the DeckStreak target's dependency on it
- `scripts/tests/test_ci_workflows.py`: R20's class, the review step in SPEC-347 A14's order rule, and the app step's skips
- `changelog.d/ios-review-screen-348.md`: part 2's entry

Part 1's rows, which this part leaves as they are:

- `crates/engine-core/src/table.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/face.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/media.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/dispatch.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/src/lib.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/Cargo.toml`: unchanged in this part; delivered by part 1
- `Cargo.toml`: unchanged in this part; delivered by part 1
- `Cargo.lock`: unchanged in this part; delivered by part 1
- `crates/engine-core/tests/review_pairs.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/tests/face.rs`: unchanged in this part; delivered by part 1
- `crates/engine-core/tests/table.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/src/allow_list.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/src/engine.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/src/face.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/src/voices.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/src/lib.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/examples/review-fixture.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/review_pairs.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/face.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/voices.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/seam.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/fixture.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/refusal_text.rs`: unchanged in this part; delivered by part 1
- `crates/ffi/tests/support/review.rs`: unchanged in this part; delivered by part 1
- `changelog.d/ios-review-348.md`: unchanged in this part; delivered by part 1 and folded into the changelog by the release

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

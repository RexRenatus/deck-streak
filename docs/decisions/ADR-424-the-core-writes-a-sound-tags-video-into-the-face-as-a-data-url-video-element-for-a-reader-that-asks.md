# ADR-424: the core writes a sound tag's video into the face as a data URL video element, for a reader that asks

- **Status:** proposed, with SPEC-410 (Closes #808, part of #632).
- **Context(s):** `engine-core` (`crates/engine-core`), `ffi` (`crates/ffi`), the iPhone and iPad app's tests
  (`ios/AppTests`). The `CardIsolation` package and the app's sources are read and not changed.
- **Settles:** #808, the gap ADR-407 D5 cut from #666: a `[sound:...]` tag that names a video file plays as
  video in the card view on iPhone and on iPad, as the desktop reviewer does; how its bytes reach the card
  view without widening the view's isolation (#619) or switching card scripts on (#651); the scope of the
  delivery; whether a formal model is owed; and how the native red is read on CI.
- **Amends:** ADR-359 D1 (a third closed table, `VIDEO_TYPES`, read only for a reader that asks, and a sound
  tag naming a video crosses as a `data:` video element inside the document, not as clip bytes), ADR-407 D5
  (the cut gap is built) and SPEC-348 R4 (for a reader that asks, a sound tag whose extension is in
  `VIDEO_TYPES` becomes a video element, not a `Clip::Sound`). The delivery appends a pointer to each
  (SPEC-410 section 10), so no rule they state is left standing unamended.
- **Read at:** DeckStreak `dev` `ff405424907647ee0f41eb6bbff6276f9dadbda3` (DEV). Every `path:line` below is
  read there.

## Context

The engine names the tag `SoundOrVideo(name)` and gives no media kind. The core maps it to
`budget.sound(name)` (`crates/engine-core/src/face.rs:165`), and no other item in the tree names a tag's
kind: `SoundOrVideo`, `is_video` and `VIDEO_EXT` occur in that file alone.

The shared table `TYPES` (`crates/engine-core/src/media.rs:17-36`) holds images and audio only (`:18`). It
types `mp4` as `audio/mp4` (`:35`) and `webm` as `audio/webm` (`:34`), and holds no `mov` and no `m4v`. So
at DEV a sound tag naming `clip.mp4` crosses as `Clip::Sound` bytes that the native player plays as audio
(`ios/App/Sources/ClipPlayer.swift:65-77`), and one naming `clip.mov` or `clip.m4v` joins `omitted`.

A table read only for a reader that asks already exists: `FONT_TYPES` (`media.rs:41-46`), behind
`Reader::inlines_fonts` (`media.rs:60-62`, default `false`), which the ffi's `MediaFolder` answers `true`
(`crates/ffi/src/face.rs:74-76`) and the web's reader leaves at the default (ADR-407 D2).

The card view already plays a video inline. The ffi writes ` playsinline` after every `<video` start tag's
name (`crates/ffi/src/face.rs:102-121`). The factory sets `allowsInlineMediaPlayback = true` on iOS
(`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:198-204`) and sets no
`mediaTypesRequiringUserActionForPlayback`. It loads the one document with `loadHTMLString(html, baseURL:
nil)` (`:64`). The document's policy admits `media-src data:` and `style-src 'unsafe-inline'`
(`ios/CardIsolation/Sources/CardIsolation/DocumentPolicy.swift:13-15`), the rule list blocks every load
(`RuleList.swift:13`), and card scripts are off (`CardScripts.swift:7`).

The face has one budget (`Budget`, `media.rs:123-127`). `admit` (`media.rs:232-254`) reads at most
`FILE_CAP + 1` bytes, refuses a file over `FILE_CAP` (4 MiB, `:13`) and refuses past `FACE_CAP` (16 MiB a
face, `:15`). `take` (`:217-229`) names a refused file once in `omitted`, and the review screen shows the
count as "Media files not shown: N" (`ios/App/Sources/ReviewChrome.swift:62-65`).

The thin-Swift census caps each Swift role's decisions (`scripts/tests/test_ios_thin_swift.py:38-47`) and
grants each framework door to one role (`:74-89`): `WebKit` to the card role alone, `AVFoundation` and
`AVFAudio` to the speech role alone.

The web plays a sound through the page's audio element from a Blob URL
(`web/app/src/lib/study/audio.ts:1-4`, `:71-83`); nothing reaches a card frame. Its reader does not override
`inlines_fonts`, and `clip_value` binds `Clip::Sound {name, bytes}` with no rest pattern
(`crates/web-engine/src/wasm.rs:877-900`).

The review screen's Swift tests run only in the native job's harness: the job (`.github/workflows/xcframework.yml:356`)
runs the step "the review screen's tests, Debug, on the iPhone and then the iPad" (`:498`) with
`-only-testing:DeckStreakTests/ReviewSessionTests` (`:512`). The fixture's directory is written by the
`review-fixture` example (`:127`) and copied whole (`:175`, `:505`), so a new fixture directory needs no
workflow edit.

## Decisions, and the alternatives each was chosen against

D1 decides the fix, D2 the containment, D3 the scope, D4 the formal surface and D5 the reds on CI. Each
decision's rejected options are the bullets under it, each with its reason.

### D1. The fix: the core classifies a sound tag's file by a closed video table, for a reader that asks, and writes a video element into the face

The engine gives no kind, so the core classifies by the file's extension against a new closed table in
`crates/engine-core/src/media.rs`:

```rust
pub const VIDEO_TYPES: [(&str, &str); 3] = [
    ("mp4", "video/mp4"),
    ("m4v", "video/mp4"),
    ("mov", "video/quicktime"),
];
```

The extension is read in any ASCII case by the private `media_type` the other tables use
(`media.rs:112-118`). The table is read only for a reader that asks. `Reader` gains
`fn shows_video(&self) -> bool { false }` beside `inlines_fonts`; the ffi's `MediaFolder` answers `true`, and
the web's reader keeps the default, as ADR-407 D2 did for fonts.

For such a reader, a `SoundOrVideo(name)` whose extension is in `VIDEO_TYPES` is admitted by the face's one
budget under the kind `video/` (`take(written, &VIDEO_TYPES, "video/")`, SPEC-348 R3's rules and both caps
unchanged) and becomes no clip. The core appends one constant element per admitted video to the face's text,
after the side's rewritten text, in the side's replay order. That is the question's videos on the question
side. On the answer side it is the question's videos then the answer's own, with the question's left out
when the preset skips the question on replay. The element is:

```html
<video controls style="display:block;max-width:100%;height:auto;margin:0 auto" src="data:<type>;base64,<bytes>"></video>
```

`<type>` is the table's type and `<bytes>` the file's bytes in standard base64 with padding. The ffi's
existing pass (`crates/ffi/src/face.rs:102-121`) then writes `playsinline` into it. The element carries
only constants, the closed table's type and base64, and never the written name. A refused video joins
`omitted` once, and the learner reads "Media files not shown: N". No Swift product file changes: inline
playback exists (ADR-407 D4), and the learner starts the video from its native controls.

A shape the build may take: `Budget::shows_video(&self, written) -> bool` reads
`self.reader.shows_video() && media_type(written, &VIDEO_TYPES).is_some()`; `Budget::video(&mut self,
written) -> Option<String>` is `take(written, &VIDEO_TYPES, "video/")` mapped to the element;
`clips(tags, budget, videos: &mut Vec<String>)` routes a video's name there and yields no clip; and
`complete` appends `videos.concat()` to the text.

**Chosen against:**

- a `video/` row in the shared `TYPES`: `TYPES` is both clients' table (ADR-359 D1), so the web's
  handling and a template's own `<video src>` type would change in a delivery for the native screen.
- a native player view: the census grants `AVFoundation` to the speech role and `WebKit` to the card role
  alone (`test_ios_thin_swift.py:74-89`), the picture would play outside the card view's isolation, the
  player takes a URL asset so the bytes would need a temporary file or a resource loader, and where the
  picture sits is a layout decision the issue does not make.
- a sniff of the file's container boxes: it is a parser over untrusted bytes, where a closed table of
  extensions is a lookup.
- classifying in Swift: the census keeps the roles thin (`test_ios_thin_swift.py:38-47`), and by then the
  clip has already crossed the ffi as audio bytes.
- classifying in the ffi: the ffi carries no base64 dependency (`crates/ffi/Cargo.toml:14-25`) and would
  need the core's admission rules and caps a second time.
- the element at the tag's own place in the text: it needs a second parser of the tags or a coupling to the
  engine's play markers, where the side's replay order is already the order the learner hears its sounds in.
- `webm` as video: `TYPES` reads it as audio (`media.rs:34`), so a sound tag naming a `webm` stays a sound
  (#632).

### D2. Containment: the video stays a `data:` URL inside the one document, read from the opened collection, bounded by the existing caps, and started only by the learner

The bytes are read through the `Reader` from the opened collection's media folder (`MediaFolder`,
`crates/ffi/src/face.rs:64-77`); nothing is fetched. The route is a `data:` URL inside the one document
string the factory loads with `loadHTMLString(html, baseURL: nil)` (`CardWebViewFactory.swift:64`). The
document's policy already admits it (`media-src data:`, `DocumentPolicy.swift:13-15`), and the rule list
still blocks every load beside it (`RuleList.swift:13`). It is the route ADR-359 D1 and ADR-407 D6 already
hold for images, sounds, fonts and a template's own video, so no route opens and #619's planted suite has no
new route to re-run on.

The element carries no `autoplay` attribute and card scripts stay off (`CardScripts.swift:7`, #651), so
nothing in the card starts it: the learner's tap on its native controls does. ADR-359 D7's hold while the
screen reader runs has nothing to hold, and stands.

Memory is bounded by the existing caps, 4 MiB a file and 16 MiB a face, counted on the file's bytes before
encoding; base64 adds a third in the document. A video over either cap is left out and named in `omitted`,
and the review screen says how many files it did not show.

`RuleList.swift`, `DocumentPolicy.swift`, `CardWebViewFactory.swift` and SPEC-349's rule list are not
changed. A change to any of them is a new decision with its own question (#619).

**Chosen against:**

- a scheme handler or a file base URL: either is a new route into the card view, which re-runs #619's
  planted suite on it and widens what the rule list and the policy must hold.
- a larger cap for video: it weakens ADR-359 D1's bound on one face's memory, which every media kind shares.
- an empty `mediaTypesRequiringUserActionForPlayback` with an `autoplay` attribute: card-authored media
  would start by itself too, past ADR-359 D7, and the video would overlap the native player's sounds.
- app-evaluated script to start and sequence playback: it opens a channel from the app into the frame, adds
  a decision to a role the census caps, and needs plumbing from the model to the view.

### D3. Scope: one delivery, with no web change and no Swift product change

One pull request builds the gap whole, so it says `Closes #808`. The web needs no change: its reader keeps
`shows_video` at the default, so its faces are byte for byte the same; `Clip` gains no variant; and
`clip_value` (`wasm.rs:877-900`) is untouched. The Swift side changes one test: the review screen shows
`ReviewFace.document` as it stands (`ios/App/Sources/ReviewView.swift:33`,
`ios/App/Sources/CardFaceView.swift:22-32`).

The delivery touches engine-core `src/media.rs`, `src/face.rs` and `tests/face.rs`; ffi `src/face.rs`,
`tests/face.rs`, `tests/support/mod.rs`, `examples/review-fixture.rs` and a new
`tests/support/sound_video.rs`; `ios/AppTests/ReviewSessionTests.swift`; a new band of mutation rows; and the
documents. SPEC-410 section 4 names each file, and the files it leaves unchanged.

**Chosen against:**

- two deliveries, the core and then the Swift: the Swift side has no product change, so a second delivery
  would hold one test.
- a web change for parity: the web plays a sound through the page, not a card frame, and its parity with the
  desktop reviewer is #630's.

### D4. FORMAL: not applicable, by surface

The change adds no actor and no shared state. `VIDEO_TYPES` is a constant. `Reader::shows_video` is a pure
answer of the reader. The budget is made inside one `complete` call (`face.rs:200`) and dropped at its end.
`clips` and the element are total functions of their arguments, and the reader reads files and writes none.
Nothing adds a timer, a lock, a static, a task, a write path, or a check followed by an act on something
another actor can change.

Of the 246 cover lines under `formal/` at DEV, none names `crates/engine-core/src/media.rs`,
`crates/engine-core/src/face.rs`, `crates/ffi/src/face.rs` or `crates/ffi/examples/review-fixture.rs`.
Three name `crates/web-engine/src/wasm.rs` and one names `crates/ffi/src/engine.rs`; the delivery edits
neither.

**Chosen against:**

- a TLA+ model of playback: no two actors share state in the change, and the player that runs the video is
  the web view's, outside the code.
- a Lean proof of the table: the table is a closed constant, pinned by tests and by a hand mutation row per
  member, which kills each wrong entry a proof would exclude.

### D5. CI cost and reds: two pushes, the Rust reds before push 1, and the Swift red read on CI by name

The native job is the slowest check, its paths filter judges the pull request's whole diff, and its
concurrency group cancels a run in progress when the branch moves. So the delivery pushes exactly twice, and
never again before the last run has concluded and been read.

Push 1 carries the documents, the seams and the fixture, every red, and the red-first record of the Rust
reds. CI reads the Swift criterion red by name, in the harness's review-screen step, per destination.

Push 2 carries the green, the rows, the changelog and the record's remaining lines: the Swift criterion's red
line from push 1's run, and its green line naming the green commit, which push 2's run confirms by name per
destination. A red there is a fix round, never a third planned push.

The Rust criteria, the whole suites of both crates, the lints, the thin-Swift census, the rows' hand proofs
and the diff's own mutants run before each push. No Swift runs anywhere but CI.

**Chosen against:**

- three pushes, the third carrying only the native green line: every push re-runs the native job, so a
  docs-only third push costs a full native run and records nothing push 2's run cannot confirm.
- recording the Swift criterion as not red: it is red at the seams, so calling it not red would weaken the
  record.
- a fixture holding a real, decodable video: no decodable video can be built from bytes in a test, and the
  fixtures commit no binary (ADR-407 D5); the picture's decode is the device session's (SPEC-410 section 7).

## Consequences

- A `[sound:...]` tag whose file ends in `.mp4`, `.m4v` or `.mov`, in any ASCII case, shows its picture and
  plays its sound in the card view on iPhone and on iPad, under its own controls, within the caps.
- It no longer plays by itself. As a sound it followed the face's autoplay; as a video it waits for the
  learner's tap, and Replay and Stop do not act on it (#632).
- An audio-only `.mp4` named by a sound tag now shows a video element whose picture is empty (#632).
- The web's faces are unchanged, and a template's own `<video src>` keeps the type `TYPES` gives it (#632).
- A sound tag naming a `.webm` file stays a sound (#632).
- A video's bytes sit in the document as base64, a third larger than the file, inside the existing caps.

## What would make this wrong

- A device session in which the card view does not play a `data:video/mp4` or `data:video/quicktime` source
  from a tap on its controls (SPEC-410 section 7): the route would need a decision #619 re-opens.
- Learners missing a sound tag video's autoplay more than they value a tap to start it: the face's autoplay
  would need a media-policy decision of its own (ADR-359 D7).
- Collections whose sound tag videos are mostly over 4 MiB: the bound would need its own decision, never a
  silent raise.
- A second client that renders the face's document: it answers `shows_video` with `true`.

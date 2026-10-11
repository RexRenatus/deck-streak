# ADR-407: the core carries the ordinal, the fonts and the voices, and the card view plays video inline

- **Status:** proposed, with SPEC-393 (Refs #666, part of #632).
- **Context(s):** `engine-core`, `ffi`, the iPhone and iPad app (`ios/`), the `CardIsolation` package.
- **Settles:** the five parity gaps #666 lists, which SPEC-348's exclusions left open (SPEC-348 section 5,
  the bullets citing #666): which are built now and which are cut, how a font and a video reach the
  card view without widening its isolation (#619), the order of the work, whether a formal model is owed,
  and how the native reds are read on CI.
- **Amends:** ADR-359 D1 (a second closed table, for fonts the note type's CSS names, read only for a
  reader that asks) and D3 (the voice that speaks a tag), and SPEC-348 R5 (the body's classes). The
  delivery appends a pointer to each, so no rule they state is left standing unamended.
- **Read at:** DeckStreak `dev` `164ac20690e0d6d0385474d04663d14df5781e19` (DEV). Every `path:line` below is
  read there.

## Context

The first review screen (SPEC-348) shows a card's face in the isolated card view, plays its sounds and
speaks its TTS tags. It left five behaviours of the desktop reviewer out, each named in its exclusions
and gathered in #666:

1. a TTS tag's `voices=` list is not applied. Its speed already is: `crates/engine-core/src/face.rs:145`
   scales the tag's speed to the platform's rate, and `crates/engine-core/tests/face.rs:448-468` pins it.
   The tag's voices are parsed by the engine (SPEC-348 M15, `docs/specs/SPEC-348-the-iphone-and-ipad-review-screen.md:49`)
   and dropped by `speech()` (`face.rs:141-151`), which builds `Clip::Speech {text, language, rate}`;
2. a sound tag that names a video file plays no picture: the type table reads `mp4` as audio
   (`crates/engine-core/src/media.rs:35`), and the clip is handed to the speech role's player
   (`ios/App/Sources/ClipPlayer.swift`);
3. the note type's CSS reaches the view as written (`face.rs:217-223`, `css: rendered.css`), so a font it
   names by `url()` names a file the view can never load: the document has no base URL (L7) and the
   rule list blocks every load (L3);
4. the body carries `card` alone (`crates/ffi/src/face.rs:17` and `:19`, written at `:69-81`), not the
   desktop's `card card<n>`, so a template's `.card2 {}` rule never applies. The web client already
   writes it (`web/app/src/lib/study/ReviewScreen.svelte:200`, from `crates/web-engine/src/wasm.rs:692`);
5. an HTML `<video>` takes the whole screen on an iPhone: the factory sets no inline playback
   (`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:194-240`), and the platform's
   default is off on an iPhone (SPEC-348 P8).

Two facts bound every choice. The card view's containment is #619's and is not widened here: a load
is blocked by one rule (`ios/CardIsolation/Sources/CardIsolation/RuleList.swift:13`), and the
document's policy admits fonts and media only as `data:` URLs
(`ios/CardIsolation/Sources/CardIsolation/DocumentPolicy.swift:13-15`). And Swift stays thin: every
app file sits at its decision ceiling (`ios/swift-roles.json`;
`scripts/tests/test_ios_thin_swift.py:38-47`), and only the `speech` role may import the media
framework (`:74-89`).

## Decisions, and the alternatives each was chosen against

Decisions D1 to D5 decide each gap (one decision each), D6 the containment, D7 the order, D8 the formal
surface and D9 the reds on CI.

### D1. The `card<n>` class: built. The core carries the card's ordinal and the document writes it

The face gains `ordinal`, the card's template index, read from the card that `preset()` already fetches
(`face.rs:126-127`), so no second engine call. The native document writes `card card<ordinal + 1>`,
then the night classes when it is asked for night, the same string the web writes. The engine already
gives the ordinal (the card record's template index); the native view adds nothing, because the class
travels inside the document.

**Chosen against** (each reason on its row's first line):

| alternative | why it lost |
|---|---|
| Swift adding the class to the document after the engine writes it | it rewrites HTML in Swift, which the census keeps in the engine, and the card role is at its ceiling |
| a second engine call for the card in the ffi | the core already holds the card at `preset()`, so a second call costs a round trip for a value in hand |
| the class from the web view's view model (`wasm.rs:692`) | that path is wasm-only, and the native face never passes through it |

### D2. Fonts named by `url()`: built. The core inlines them into the face's CSS for a reader that asks

`Face.css` passes a font pass when the face's reader asks for it: each `url(...)` in the CSS whose
argument, quotes stripped and `%XX` escapes decoded, is a plain file name with a type in a new closed
table, `FONT_TYPES` (`ttf`, `otf`, `woff`, `woff2`, each with its `font/` media type), is judged by the
rules SPEC-348 R3 already applies to a media attribute, in their order: admitted, it becomes
`url("data:font/<type>;base64,<bytes>")`; refused (no such file, over the file cap, past the face's
cap), it becomes `url("")` and its written name joins `omitted` once. Any other argument (a scheme, a
`data:` URL, a path, a name of another type) is left as written, for L3 and the policy to hold. The
font pass runs last, after the text's media and the clips, so a font never takes the budget a card's
own image or sound needed. The `Reader` trait gains `inlines_fonts`, `false` by default; the ffi's
media folder answers `true`. The engine already gives the client the note type's CSS (`face.rs:219`);
the native view adds nothing, because the font arrives inside the document.

**Chosen against:**

| alternative | why it lost |
|---|---|
| inlining for every reader | the web's reader turns every name it is asked for into a fetch (`wasm.rs:814-820`) while its frame renders the view's own CSS (`ReviewScreen.svelte:198`), so it would fetch fonts it never draws (#630 owns the web's half) |
| a parameter on `Dispatcher::face` | it changes the web engine's two calls and the census text that pins them (`crates/web-engine/tests/boundary.rs:216-217`), on files #748 also changes |
| the fonts added to the shared `TYPES` table | `TYPES` decides what the HTML rewrite admits for both clients (ADR-359 D1), so a font named in an `<img>` would become admitted media |
| the fonts first, before the text's media | a large font would take the face's budget and leave out the card's own image or sound |
| a `WKURLSchemeHandler` or a base URL with file access for the media folder | each widens L3 or L7, which ADR-359 D1 already refused for media (#619) |
| leaving a refused font's `url()` as written | the view would ask WebKit to load a file name it cannot reach, and the name would never reach `omitted` |

### D3. The template's voices: built. The core carries them; the ffi picks the voice; Swift calls one method

`Clip::Speech` gains `voices`, the tag's list in its order, on both sides of the FFI. `VoiceChoices`
gains `voice_for(language, requested, installed)`, which answers, in order: the voice the learner
chose for the language when it is installed (what `chosen` answers today,
`crates/ffi/src/voices.rs:112-121`); else the first requested entry that names a voice the picker would
offer for the language (`options`, `voices.rs:129-144`), by its identifier, or by its name with spaces
written `_`, with or without one leading `<platform>_` prefix; else nothing, and the language's own
voice speaks (`ios/App/Sources/InstalledVoices.swift`). `EngineSession.clip`
(`ios/App/Sources/EngineSession.swift:228-240`) binds the new field in its existing `case` and calls
`voice_for` where it calls `chosen` (`:236`): no new decision, so the session role stays at its ceiling.
The engine already gives the tag's language and rate (`face.rs:141-151`); the native view adds the one
call.

**Chosen against:**

| alternative | why it lost |
|---|---|
| the template's list first, over the learner's choice | a shared deck's voice would become one the learner can never change on the device; with no choice kept, the order is the desktop's |
| matching the desktop's exact voice string only | the platform prefix differs per system, so a deck written elsewhere would never match, and a bare name would never match either |
| matching by identifier only | template authors write voice names, never a platform's identifiers |
| resolving the voice in Swift | it is a rule over a list, which the census keeps in Rust, and `EngineSession` is at its ceiling |
| applying the tag's other options | no voice on an iPhone or iPad reads them; the desktop passes them to its own players (#632) |

### D4. Inline video: built. The factory plays media inline on an iPhone, and the document marks each video

The factory sets `allowsInlineMediaPlayback = true` on the configuration it builds, inside the
iPhone-and-iPad compile guard (`#if os(iOS)`), the guard `WindowRefusal.swift` already uses (lines 2 and
62): the property exists only there, and the package also builds for the host its own tests run on. The
native document adds `playsinline` to each `<video` start tag of the face's text, in any case, when the
name ends at a space, a `/` or a `>`. The engine already gives the video's bytes as a `data:` URL
(SPEC-348 M17, `replace_media_refs` covers `video` and `source`); the native view adds the configuration
property, and the ffi the attribute.

**Chosen against:**

| alternative | why it lost |
|---|---|
| the configuration property alone | the platform plays a video inline on an iPhone only when the element also carries `playsinline` |
| the attribute alone | the configuration's default on an iPhone refuses inline playback whatever the element says |
| the attribute added in the core | the web's frame does not need it, and the core's text is one copy both clients read |
| a separate native player for every HTML video | a new surface and a new door for what the card view already plays |

### D5. A sound tag that names a video: cut, with a follow-up issue

The engine already gives the client the file's bytes as a `Sound` clip (`media.rs:35` reads `mp4` as
audio). Showing its picture needs a player surface the review screen does not have, and a decision
about where the picture shows; SPEC-393 section 9 states the behaviour for the follow-up issue, and the
pull request says `Refs #666`, not `Closes`.

**Chosen against:**

| alternative | why it lost |
|---|---|
| building it now with a native player view | it needs a framework door the thin-Swift census grants no role (`test_ios_thin_swift.py:74-89`) and a layout decision the issue does not make |
| writing the tag into the face as an HTML `<video>` | the desktop plays such a tag outside the card, and starting it with sound needs autoplay for every card's media, past ADR-359 D7's VoiceOver hold |
| a video row in the shared type table | `TYPES` is both clients' (ADR-359 D1), so the web's handling would change in a delivery for the native screen |
| testing it with a recorded video | the fixtures build every file from bytes in the test and commit no binary (`crates/ffi/tests/support/review.rs`), and no decodable video can be written that way |

### D6. Containment: every font and every video stays a `data:` URL inside the document

A font is read through the face's `Reader` from the collection's media folder
(`crates/ffi/src/face.rs:57-66`), under the same two caps, and written into the CSS the document already
carries in its one `<style>`. A video was already a `data:` URL in the text. Nothing is fetched: the
document is still handed to `loadHTMLString(_, baseURL: nil)` (L7), WebKit's content-rule backend does not
see `data:` URLs, and the policy already admits `font-src data:` and `media-src data:`
(`DocumentPolicy.swift:13-15`). The planted suite already renders a `data:` font, sound and image through
the whole frame (`ios/CardProbeTests/Planted.swift:547-586`) and holds a remote `@font-face` and remote
media at L3 (`Planted.swift:157-164`). So the rule list, the policy, the navigation gate and the planted
suite are unchanged, and inline playback is not a layer: it decides where a playing video is drawn and
opens no load. A change to `RuleList.swift` or `DocumentPolicy.swift` in this delivery is a STOP.

**Chosen against:**

| alternative | why it lost |
|---|---|
| a scheme handler serving fonts and video from the media folder | a new channel the planted suite must re-prove, and an exception in the rule list (#619) |
| a file base URL with read access | it breaks L7 and reaches every file under the granted directory |
| a rule-list exception for the media folder | the rule list is #619's, and widening it is outside this delivery |

### D7. Order and split: one delivery, built gap 4, then 3, then 1, then 5

One delivery on one branch, in red-first order, with three pushes: the reds, the greens, then the
record of the native greens. The two Rust-only gaps go first because their reds and greens are measured
on the box and their files come first in the flow (the face, then the document). The voice comes third
because its Rust half is box-measured and its Swift half is one call. Inline video comes last because
its proof is a simulator readback and a census. Every native test is a method in a class the CI step
already selects, so no workflow changes. The Rust gaps share the face and the document, so a split
would edit `crates/engine-core/src/face.rs` and `crates/ffi/src/face.rs` in several pull requests.

**Chosen against:**

| alternative | why it lost |
|---|---|
| one pull request per gap | four pull requests over the same two files, each paying the native job's runs |
| a native-first order | its reds wait on a native run, while the Rust reds can be read on the box first |
| a new workflow step for the new tests | the existing steps select their classes whole, and the workflow file is changing in another pull request (#752) |
| a new Python test module for the factory's census | a new module must be placed in the CI shards, which no box run can check; the layer census module holds the factory's other rules |

### D8. FORMAL: not applicable, by surface

No actor, timer, shared state or write path is added. The ordinal, the CSS pass and the voice order are
total functions of their inputs, called inside one face's completion. `voice_for` reads the choices
behind the same lock `chosen` reads, from the same caller (`EngineSession`, an actor), in place of
`chosen`; the write path (`choose`) is unchanged. Of the 219 `@phx covers` lines under `formal/` at DEV,
three name `crates/web-engine/src/wasm.rs` (`undo`, `undo_offer`, `rate`) and one names
`crates/ffi/src/engine.rs` (`run`); this delivery edits `clip_value` alone in the first and does not edit
the second. No recorded failure motivates a proof of the pure functions; their tests and rows hold them.

**Chosen against:**

| alternative | why it lost |
|---|---|
| a model of the voice choice | no second actor reads or writes the choices in a new order, so a model would check one sequential path |
| a proof of the CSS pass | it is a total function with planted refusals and value pins, and no recorded failure asks for a proof |

### D9. CI cost and reds: three runs of the native job, and the reds read by CI name

The native workflow runs on every update of a pull request into dev whose whole diff touches `crates/ffi`,
`crates/engine-core` or `ios`, so each push runs the `harness` job once; push 3 touches the red-first
record alone, and its run reads push 2's code again. The box runs every Rust test, the Python censuses
and the hand mutation proofs; Swift runs only on CI. The two native criteria are read by name in the
`harness` job's steps: the voice in the review screen's step, the factory's inline playback in the card
view's planted-suite step. Push 1 carries their reds, push 2 their greens.

**Chosen against:**

| alternative | why it lost |
|---|---|
| one push with every red and green | CI reads only the head, so no native red would ever be observed |
| recording the native criteria as not red | they are new behaviour, so a test green at the base would prove nothing |
| re-running a failed job to read a green | a re-run reads the same tree, and a fix needs its own push |

## Consequences

- A template's `.card<n>` rule, a font its CSS names, a voice its tags ask for, and an inline video each
  behave on an iPhone and an iPad as on the desktop, within the caps.
- A font over the file cap is left out and named, so a whole CJK font usually falls back to the
  system's; a font sliced by `unicode-range` inlines each slice it names, against the face's cap.
- The web's face is unchanged: its reader does not ask for fonts, its speech clip ignores the new field,
  and its view keeps the class it already writes.
- A sound tag naming a video keeps playing as a sound, until the follow-up issue is built.

## What would make this wrong

- A measurement that the card view refuses a `data:` font or a `data:` video on a current WebKit (the
  planted suite's `permitted` card fails): fonts and video would need a path #619 re-decides.
- A video written as `data:audio/mp4` (the shared table's type for `mp4`) that plays its sound and shows no
  picture: an HTML video source would need its own type, which re-opens ADR-359 D1.
- Learners asking for the template's voices over their own choice: D3's order becomes a setting.
- A second client that renders the face's own CSS: it answers `inlines_fonts` with `true`.

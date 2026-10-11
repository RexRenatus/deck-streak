# ADR-359: the review screen's media, face, voice choice, deck tree, rating and test seam

- **Status:** proposed, with SPEC-348 (#632).
- **Context(s):** `engine-core`, `ffi`, the iPhone and iPad app (`ios/`).
- **Settles:** seven decisions ADR-335, ADR-342, ADR-352 and the app shell's ADR-358 (#625) leave open. ADR-335
  puts card faces in an isolated web view and the review screen, haptics and speech on the
  platform; ADR-342 puts the answer buttons at the bottom; ADR-352 turns card scripts off and gives
  the iPhone and iPad layers to the iPhone and iPad delivery; ADR-358 keeps a hand-written codec, a
  lexical census of thin Swift, and one collection. None of them says how a card's media reaches a
  frame that blocks every load, how a face crosses the FFI, where a voice choice lives, how a
  recursive message is decoded, which side turns a rating into a state, or how a UI test reaches a
  collection with cards.

## Context

The review screen is the first app screen that shows a card. Measured (SPEC-348 section 1):

- The frame blocks every load with a `.*` rule and loads its HTML as a string with no base URL
  (M2, M6), so a card's `<img src="cat.png">` reaches nothing. ADR-352's consequences say media
  reach the frame as `data:` until a study screen decides a media path, and any other path re-runs
  the planted suite (M5). WebKit's content-rule backend does not match `data:` URLs (M8).
- A face is not one engine call. Anki completes it in four steps: a partial render, the question's
  AV extraction, the question's extracted text placed in `{{FrontSide}}`, the answer's extraction
  (M16). Autoplay and replay read the card's preset (M18).
- The census forbids `UserDefaults`, `@AppStorage` and `NSUbiquitousKeyValueStore` in every file,
  and budgets each role's decisions (SPEC-347 R11).
- The deck tree is the first recursive message the codec meets (M14); ADR-358 named it as the
  case that could make generated types win.
- The app shell's UI tests (#625) run on a fresh install, whose collection holds one empty deck.

## Decisions, and the alternatives each was chosen against

### D1. Media reach the frame inside the document, as `data:` URLs the core writes

The core rewrites every media reference in a face's display text, through the engine's own
`replace_media_refs`, to a `data:` URL of the file's bytes. A name is decoded, then admitted only
as a plain file name; its type comes from a closed extension table (images and audio, an mp4 read
as audio); a file over 4 MiB, or past 16 MiB for the face, is left out and named in `omitted`, with
its attribute emptied. `[sound:]` clips cross as bytes and play natively. The base64 encoder is the `base64`
crate, made a workspace dependency at the version the lockfile already resolves for the engine's
HTTP client. The two caps and the table are the core's public constants, one copy both clients
read; the web's study screens take the same ones (#630).

| alternative | why it lost |
|---|---|
| a `WKURLSchemeHandler` serving the media folder, with one rule-list exception | it widens L3 and adds a channel the planted suite must re-prove; the review screen may not widen the frame |
| `loadFileURL` or a file base URL with read access to the media folder | it breaks L7 (a string with no base URL), and file access reaches every file under the granted directory |
| a loopback HTTP server for media | a listener on the device and an exception in L3, for the same bytes |
| Swift rewriting the HTML after the engine renders it | parsing and rewriting HTML is logic, and the census keeps logic in the engine |
| a hand-written base64 encoder in the core | a second implementation of RFC 4648 to test and mutate, where the lockfile already holds a maintained one |
| no caps | one large file would make a face a string of tens of MiB on every show |
| a cap pair and a type table in each client | two copies drift, and a face one client shows could be one the other omits |

### D2. A face crosses the FFI as one call that returns a record

`Dispatcher::face(card, side, autoplay, media)` in the core completes a face as Anki does and
returns its parts; the native `Engine::face(card_id, answer, night, autoplay)` reads the media
folder and returns a UniFFI record, `CardFace {document, autoplay, replay, omitted}`, with
`Clip` an enum of `Sound {name, bytes}` and `Speech {text, language, rate}`. The web may take the
same core call (#630).

| alternative | why it lost |
|---|---|
| (27,3) ExtractAvTags and (27,9) StripAvTags as adapter pairs, Swift composing the face | the four-step join, the FrontSide rule, the preset's flags and the media rules would all be Swift logic |
| a DeckStreak protobuf message for the face, through the hand codec | a new `.proto`, a new decoder and its mutants, for a value that never crosses a transport other than UniFFI |
| JSON through the wire role | a second encoding beside protobuf; the record UniFFI generates is typed on both sides already |
| the face in the FFI crate, not the core | the core holds the engine privately (SPEC-345 R1), and the preset read and the render need it; the web could not share it |

### D3. The voice choice lives in a small file the engine adapter keeps on the device

`VoiceChoices` in the native adapter keeps one `language<TAB>identifier` line per language in a
file under Application Support, outside the collection, written whole and renamed. It answers
`chosen` only for a voice still installed, so a voice the system removed falls back to the
language's own.

| alternative | why it lost |
|---|---|
| `UserDefaults` or `@AppStorage` | the census forbids both in every file; allowing them weakens a census |
| the collection's config, through a config pair | it syncs a device's voice identifier to devices that lack the voice, and it is a collection write outside answering, which widens the native write surface |
| the Keychain | the credential door is for the sync credential (SPEC-347 R9); a voice is not a secret |
| the note type's `voices=` option | a note-type write, synced, and shared by every device |
| no persistence | the learner chooses again on every launch |

### D4. The deck tree is decoded by the hand codec, depth first, with a depth bound

`HarnessWire` gains a recursive decoder for `DeckTreeNode` that flattens the tree depth first,
skips the root, and refuses a tree deeper than 32 levels; literal bytes of a two-level tree pin it.

| alternative | why it lost |
|---|---|
| generated protobuf types with a vendored runtime | ADR-358 D5's reasons stand: a vendored runtime and generated sources the census cannot read, for one recursive message |
| (13,10) CountsForDeckToday per deck | one call per deck, and its response has no learning count |
| a core call returning flattened rows as a UniFFI record | it adds a bespoke export for a value one allow-listed pair already returns; the allow-list exists to bound that surface, and D2's export is justified only because no pair can express a face |
| no recursion bound | a hostile or damaged collection could exhaust the stack |

### D5. The rating picks its state in the codec, from the states the card was shown with

The codec keeps all five of a queued card's states as opaque bytes, and `state(for: Rating)`
selects the one the rating names; `CardAnswer` carries it with the current state. The intervals
on the buttons come from the same states (13,24).

| alternative | why it lost |
|---|---|
| a core `answer(card, rating)` that re-reads the states through (13,23) | the states depend on the time they are computed, so the interval the learner read and the one applied could differ; and it adds a pair |
| the model choosing the state with a `switch` | a decision in a budgeted file, for a mapping the codec's literal-bytes tests and mutants already hold |

### D6. UI tests reach a seeded collection through one launch argument the engine adapter reads

`collection_directory(fallback, arguments)` returns the default when no `-DSCollectionDirectory`
argument is given, and the value after it when that is an absolute path to an existing directory;
every other value is refused by name (`CollectionDirectoryRefusal`: no value, not absolute,
missing, not a directory). The UI tests copy the review fixture to
a fresh directory and launch the app with the argument; a launch without it is the app shell's
fresh install.

The parameter is named `fallback` because `default` is a keyword in the generated C header, which the Swift
module cannot import; chosen against are a header post-processing step in the workflow (it edits generated code,
behind the workflow wall) and a binding-generator upgrade (a dependency change outside this delivery).

| alternative | why it lost |
|---|---|
| a seed collection bundled in Debug builds | the app shell's fresh-install criterion runs on the same Debug build and would see the seed |
| Swift reading the environment or the arguments | an optional to unwrap, a decision in a budgeted file |
| hosted unit tests alone | they cannot read the bar's geometry or the accessibility tree a learner's VoiceOver reads |
| a bundled seed copied by a test-only scheme | a second scheme and a second app configuration to keep equal to the first |
| the default for a value the argument names but cannot use | a UI test that names a wrong directory would run on the fresh install, and fail or pass for the wrong reason |

### D7. Autoplay waits while VoiceOver runs

The app asks the face for autoplay only while VoiceOver is not running; the replay and stop
buttons work either way.

| alternative | why it lost |
|---|---|
| autoplay regardless | the card's speech and VoiceOver's reading of the card talk over each other on every show |
| an in-app autoplay setting | a setting needs storage and a screen; the preset's own flag already turns autoplay off per deck |

### D8. The review screen's tests run in their own step after the app's tests, and the app's step skips them

The `harness` job runs A15 to A22 in a step of their own, "the review screen's tests, Debug, on
the iPhone and then the iPad", between the app's tests and the app's archive, over the app step's
derived data, with the review fixture copied into the runner's temporary directory and named on
the step's command line. The app's step skips the three review classes, so each test runs once,
and the report carries the step's cases and time beside the app's. The step runs after a red app
step, as the archive does, so a red shell test does not hide the review screen's reading.

| alternative | why it lost |
|---|---|
| the review tests inside the app's step | the fixture would be named to every app test, the shell's fresh-install criterion included, and the review screen's reading and time could not be told from the shell's |
| the step after the archive or after the report | SPEC-347 A14 holds the app's steps together between the harness's last step and the report; after the report the review's rows would be missing from it |
| a job of its own for the review screen | R20 adds no job: a job is another macOS runner, another download of the engine and another context to require |
| the step without the app step's skips | each review test would run twice, once with no fixture named, and be red there |

### D9. A13 judges the app's Swift, not the isolation package or the test targets

A13 reads every Swift file git tracks under `ios/` outside `ios/CardIsolation/` and outside a test
target, by the test-target rule `scripts/tests/test_card_web_view_layers.py` already holds,
imported and not copied, and prints each path it skips with their count. R18's words occur in two
places that are no card frame the app builds: the factory's guard, which counts the script message
handlers a configuration is given, and the card probe's tests, which name them to prove the
factory refuses them.

| alternative | why it lost |
|---|---|
| every Swift file under `ios/`, as R18 words it | the factory and the card probe's tests hold R18's names by design, so the rule would refuse dev's own tree or need exceptions by file |
| a list of admitted files, each by its path | each new file in a test target or in the isolation package would need a census edit, and a list of paths drifts where a rule does not |
| a copy of the test-target rule in the new module | two spellings of one rule, free to drift apart; the import keeps one |

## Consequences

- The card's isolation is unchanged: the review screen hands the factory one string, and a media
  file reaches WebKit only inside it (#619 keeps its proof).
- The core's face is shared by both transports; the web decides whether to take it (#630).
- The native allow-list grows by three ordinary pairs, two reads and one write, (7,22), the write
  Anki itself makes when a deck is chosen.
- Parity gaps are named, not hidden: `voices=`, video clips, CSS fonts and inline iPhone video are
  the review-parity follow-up's (#666).
- The census gains two roles and no ceiling rises.
- The `harness` job gains one step and its report one row; no job, runner or required context
  is added (D8).

## What would make this wrong

- A measurement that a `data:` URL is matched by the frame's rule list on a current WebKit (the
  #619 render proof fails): media would need a path ADR-352 re-decides.
- A measured face that a learner's ordinary deck pushes past the caps: the caps move by an
  amendment, with the measurement.
- A device's VoiceOver users asking for autoplay: D7 becomes a setting.
- A second recursive message, or a codec the census can read generated: D4 reopens ADR-358 D5.

## Amendment: the adapter picks a native answer's next state (SPEC-365)

ADR-376 amends D5. A native press answers through the adapter's `Engine::answer`, which takes the
card, its grade, Again or Good, and the states the card was shown with, as the codec keeps them,
and picks the grade's own next state beside the token that records the grade. The codec's
`Rating` names two grades, Again and Good, and the native client sends no next state of its own
choosing. D5's ground stands: the states are those the card was shown with, never read again at
answer time. The rest of the record stands.

- The pick in the adapter, from the states the card was shown with: chosen because the next state is then chosen beside the token, as the web's `rate` chooses it, and the native code chooses none.
- The pick kept in the codec, with the codec's `CardAnswer` sent through the adapter: rejected because the native code would still choose a next state the token does not check.
- A core `answer(card, rating)` that reads the states again at answer time: rejected for D5's own reason, that the interval the learner read and the one applied could differ.

## Amendment: fonts, voices and the body class (SPEC-393, ADR-407)

- D1 gains a second closed table, `FONT_TYPES`, read only by the face's CSS pass and only for a reader that
  asks (ADR-407 D2 and D6). `TYPES` and the HTML rewrite are unchanged, and every font reaches the frame as
  a `data:` URL inside the document.
- D3's choice is the first of three: the kept choice, then the tag's requested voices the picker would
  offer, then the language's own (ADR-407 D3).

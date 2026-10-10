# Schematic: the iPhone and iPad review screen

- **For:** SPEC-348 (#632), decided by ADR-359.
- **Kinds:** component, state machine, data flow.
- **Read at:** DeckStreak `dev` `ac4fbdaeb02f682991a9b26db539dbc20c8ff559` (DEV), #662's head
  `af6da34688467f8866167c4affbb53a45cca9d8b` (ENG), and the engine commit the workspace pins,
  `c538de55a23e695234e794029fce0dafff2d36a9` (PIN). Every `path:line` below names one of the three.
- **Stands on:** the app shell (#625, SPEC-347: the split view, the session, the census), the iPhone
  and iPad half of #619 (`CardWebViewFactory`, layers L1 to L7, `docs/schematics/card-frame-channels.md`
  section 4), and #623's core (`Dispatcher`, the per-transport table).

## 1. Components

```mermaid
flowchart TB
  subgraph App["DeckStreak app (Swift: renders, forwards gestures)"]
    DL["DeckListView (view)<br/>rows from (7,4), counts in words"]
    RV["ReviewView (view)<br/>ReviewChrome + card + AnswerBar"]
    AB["AnswerBar (view)<br/>Show Answer, then Again Hard Good Easy<br/>safeAreaInset bottom, 44 pt targets"]
    VP["VoicePickerView (view)<br/>System default + options"]
    RM["ReviewModel (model)<br/>phase, counts, face, ratings<br/>perform(ReviewAction)"]
    RS["ReviewSession (session)<br/>the review's engine calls<br/>head card, shown-at clock"]
    CF["CardFaceView (card)<br/>one factory view per face<br/>pageZoom from Dynamic Type"]
    CP["ClipPlayer (speech)<br/>AVAudioPlayer(data:)<br/>AVSpeechSynthesizer"]
    IV["InstalledVoices (speech)<br/>speechVoices() as records"]
    HW["HarnessWire (wire)<br/>hand codec: states, intervals,<br/>deck tree, set current deck"]
    HP["sensoryFeedback<br/>impact on a rating, success at the end"]
  end
  subgraph Iso["CardIsolation (#619, unchanged)"]
    FAC["CardWebViewFactory.makeCardWebView(html:)<br/>L1 store, L2 JS off, L3 rule list,<br/>L4 no handler, L5 gate, L6 no windows,<br/>L7 string with no base URL"]
  end
  subgraph FFI["deck-streak-ffi (one static library)"]
    ENG["Engine<br/>run(service, method, bytes)<br/>face(card, answer, night, autoplay)<br/>collection_directory(fallback, arguments)"]
    VC["VoiceChoices(path)<br/>chosen / options / choose"]
    AL["allow_list.rs<br/>+ (7,4) (7,22) (13,24)"]
  end
  subgraph Core["deck-streak-engine-core"]
    DSP["Dispatcher<br/>run (table) / read / face"]
    FACE["face.rs<br/>join, FrontSide, AV extract and strip,<br/>autoplay and replay from the preset"]
    MED["media.rs<br/>name rule, type table, caps,<br/>data: URLs"]
    TBL["table.rs<br/>native column + three rows"]
  end
  RUST["Anki engine (rslib, PIN)<br/>RenderExistingCard, extract_av_tags,<br/>strip_av_tags, replace_media_refs"]
  MF[("collection.media<br/>beside the collection")]
  VF[("voice choices file<br/>Application Support, outside the collection")]
  subgraph CI["CI"]
    LNX["Linux CI<br/>cargo test (Part 1)<br/>census modules (Part 2)"]
    MAC["Apple job: xcframework writes the review fixture<br/>harness: the review screen's tests on iPhone and iPad<br/>harness-wire: the codec's tests and mutants"]
  end

  DL -->|choose deck| RM
  RV --> AB
  RV --> CF
  RV --> VP
  AB -->|"perform(.showAnswer / .rate(r))"| RM
  RM --> RS
  RM --> CP
  RM --> HP
  VP --> VC
  VP --> IV
  CP --> VC
  CF -->|"html: CardFace.document"| FAC
  RS --> HW
  RS -->|bytes| ENG
  RS -->|face| ENG
  ENG --> AL
  ENG --> DSP
  DSP --> TBL
  DSP --> FACE
  FACE --> MED
  FACE --> RUST
  MED -->|reader| MF
  DSP --> RUST
  VC --> VF
  LNX -.-> Core
  LNX -.-> FFI
  MAC -.-> App
```

What each edge may carry, and what it may not:

| edge | carries | never |
|---|---|---|
| card to factory | one HTML string, the face's `document`; a new view per face | a configuration change, a handler, a base URL, a file URL, a reload into a sealed view |
| session to engine (`run`) | allow-listed pairs' protobuf bytes, built and read by `HarnessWire` | a pair outside the native column; the deck tree's or the states' meaning (the codec's) |
| session to engine (`face`) | a card id and three booleans; back a `CardFace` record | a media path; Swift never resolves or reads a media file |
| core to media folder | a file's bytes for a name that passed the name rule | a name with a separator, `.`, `..` or NUL |
| clip player to voice choices | a language and the installed voices as records | a write to the collection, a sync |

The doors the census holds (SPEC-347 R11, extended): only `session` imports `DeckStreakFFI` or
`HarnessWire` or names `FileManager`; only `card` (and `CardIsolation`'s `isolation` files)
imports `WebKit`; only `speech` imports `AVFoundation`; `view` and `model` import neither.

## 2. The review loop, a state machine

```mermaid
stateDiagram-v2
  [*] --> Choosing
  Choosing --> Loading: choose deck / (7,22) then (13,3)
  Loading --> Question: a card / face(question)#59; autoplay
  Loading --> Finished: no card / success haptic
  Loading --> Refused: an engine refusal
  Question --> Revealing: Show Answer
  Revealing --> Answer: face(answer) and (13,24)#59; autoplay
  Revealing --> Refused: an engine refusal
  Answer --> Answering: a rating / impact haptic
  Answering --> Loading: (13,4) with the rating's state, the head cleared
  Answering --> Refused: an engine refusal
  Finished --> Choosing: back to the decks
  Refused --> Choosing: back to the decks
```

- `Loading`, `Revealing` and `Answering` disable the bar: a tap there is not a transition.
- The session clears its head card when it sends (13,4), so a second rating for one shown card
  finds no head and sends nothing; the model's phase guard is the first fence, the head the
  second.
- Showing a face stops the clip player; `replay` and `stop` are self-loops on `Question` and
  `Answer`.
- `perform(ReviewAction)` is the one entry every gesture uses; the remote and keys of #633
  call the same entry.

## 3. The data of one card

```mermaid
sequenceDiagram
  participant V as ReviewView
  participant M as ReviewModel
  participant S as ReviewSession
  participant W as HarnessWire
  participant E as Engine (ffi)
  participant C as Dispatcher (core)
  participant A as Anki engine
  participant F as CardFaceView
  V->>M: perform(choose deck)
  M->>S: setCurrentDeck(id)
  S->>W: Requests.setCurrentDeck
  S->>E: run(7, 22, bytes)
  M->>S: next()
  S->>E: run(13, 3, fetch_limit 1)
  E-->>S: QueuedCards bytes
  S->>W: Responses.queue: head card, five states, counts
  M->>S: face(head, question)
  S->>E: face(card, false, night, !VoiceOver)
  E->>C: face(card, Question, autoplay, reader)
  C->>A: RenderExistingCard partial#59; extract and strip AV
  C->>C: rewrite media as data: (name rule, types, caps)
  C-->>E: Face
  E-->>S: CardFace (document, autoplay, replay, omitted)
  M->>F: document
  F->>F: makeCardWebView(html: document)
  M->>M: play autoplay clips
  V->>M: perform(.showAnswer)
  M->>S: face(head, answer) and intervals
  S->>E: run(13, 24, the head's states bytes)
  E-->>S: StringList: four intervals
  V->>M: perform(.rate(.good))
  M->>S: answer(.good)
  S->>W: CardAnswer with state(for: .good), shown-at, elapsed
  S->>E: run(13, 4, bytes)
  M->>S: next()
```

Where a value crosses a boundary:

- **Swift to Rust:** a card id, three booleans, the process arguments (once, at launch), a
  language, and the installed voices as `{identifier, name, language, quality}` records.
- **Rust to Swift:** the document string, clips (`Sound {name, bytes}` or
  `Speech {text, language, rate}`), the omitted names, the voice identifiers.
- **Rust to WebKit:** nothing directly; the document reaches WebKit only through the factory's
  `loadHTMLString(_, baseURL: nil)` (L7), and its media are `data:` URLs inside it, which the rule
  list's `.*` does not see (WebKit's content-rule backend skips `data:`) and which the frame's
  render proof covers (#619).

## 4. Where the numbers come from

| figure | source |
|---|---|
| (7,4), (7,22) | `proto/anki/decks.proto` lines 18 and 38 at PIN; the backend decks service is empty (line 44), so the collection service's order is the method number |
| (13,24) | `proto/anki/scheduler.proto` line 43 at PIN, after the backend scheduler service's three methods (lines 71 to 77) |
| the FrontSide order | `pylib/anki/template.py` lines 236 to 241 and 323 to 325 at PIN |
| the replay order | `qt/aqt/reviewer.py` lines 72 to 79 at PIN |
| the preset's flags | `proto/anki/deck_config.proto` lines 172 and 181 at PIN |
| the native column today | `crates/ffi/src/allow_list.rs` lines 23 to 56 at DEV; `crates/engine-core/src/table.rs` line 113 at ENG |

## 5. Part 2: the review step in the `harness` job, and the factory as #682 leaves it

Read at DeckStreak `dev` `f3392ec3dde43e80f47138f8ec5da8419a9e4b5c` (CUT). Kind: data flow.

The review fixture travels from the engine's job to the review screen's tests, and the `harness`
job's app steps run in this order (`.github/workflows/xcframework.yml` at CUT: the fixture written
at lines 126 to 127 and uploaded at 175; "the app's tests" at 479; "the app, archived" at 494;
"the report" at 532). The step between the app's tests and the archive is part 2's (ADR-359 D8).

```mermaid
flowchart TD
  FX["xcframework job<br/>review-fixture written and uploaded"] --> DL["harness job<br/>engine-artifact/review-fixture"]
  LAST["the required-reason symbols<br/>the harness's last step"] --> APP["the app's tests<br/>the whole scheme, the three review<br/>classes skipped"]
  APP --> REV["the review screen's tests (review-tests)<br/>ReviewModelTests, ReviewSessionTests,<br/>ReviewFlowTests, on both simulators"]
  DL -->|"copied into the runner's temporary directory"| REV
  REV --> ARC["the app, archived<br/>after a red test step too"]
  ARC --> REP["the report<br/>the app's and the review's cases and times"]
  REV -->|"TEST_RUNNER_DS_REVIEW_FIXTURE on the command line"| TP["the test process<br/>reads DS_REVIEW_FIXTURE"]
  TP -->|"copied into a fresh directory per test"| DIR["the test's collection directory"]
  DIR -->|"-DSCollectionDirectory"| SEAM["collectionDirectory(fallback:arguments:)<br/>the engine adapter decides"]
  SEAM --> OPEN["EngineSession.open<br/>the seeded collection"]
```

The factory the card view awaits is the one #682 leaves
(`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` at CUT): its layers are L1 to
L8 and L10 to L13 (lines 4 to 10; L9 is retired and its number not reused), against section 1's
L1 to L7, and `load(_:into:)` (line 55) prefixes the document policy ahead of the HTML it is
given. The card view hands `CardFace.document` to it unchanged, so the page carries the policy's
doctype and then the face's own; nothing in the app sets anything on a configuration.

## 6. The withheld phase (SPEC-380)

Read at the base `e7ecf10d6b796eb1f86fe6544e04a96a0583c791` (`dev`). Decided by ADR-391. This
section is appended to `docs/schematics/ios-review-screen.md`; no line above it changes. The rule
and both reviews' data flow are drawn in `docs/schematics/web-study-screens.md` section 7.

```mermaid
stateDiagram-v2
  Loading --> Withheld: a card whose face is withheld / face(question)
  Loading --> Question: a card whose face is not withheld / face(question)
  Withheld --> Marking: Bury, or Flag
  Marking --> Loading: after a bury / (13,3)
  Marking --> Withheld: after a flag, back to its side
  Marking --> Refused: an engine refusal
```

- `ReviewSession.next` answers `withheld` when the ffi's face is withheld and `question`
  otherwise (`ios/App/Sources/ReviewSession.swift:63`): the session's one new decision, 3 to 4 of
  its ceiling of 4.
- In `Withheld` the bar disables Show Answer and every rating, as it does outside their own phases
  (`ios/App/Sources/AnswerBar.swift:33`, `:58`), and `perform` has no arm for them there: a tap is
  not a transition, and no rating is sent.
- Bury and flag join `perform`'s existing patterns (`ios/App/Sources/ReviewModel.swift:86`, `:92`),
  and the chrome's idle names `withheld` by one decision in place of one
  (`ios/App/Sources/ReviewChrome.swift:76-78`).
- The card's place shows the ffi's withheld document, which holds the English line
  (`ios/App/Sources/ReviewView.swift:33`): the view gains no branch, and `withheld` joins the
  phase's one case list (`:7`).
- The head card stays the withheld one until a bury, so a flag marks the card the learner was told
  about.

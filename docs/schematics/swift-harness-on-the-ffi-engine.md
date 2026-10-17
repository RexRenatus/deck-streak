# Schematic: the Swift harness, one call from a view to the engine and back, and its CI jobs

Kind: component, then a data flow (one call), then a sequence (one review), then the CI job graph.
Decided by ADR-350, built by SPEC-339. Read at 7507d8a2 against SPEC-336's adapter
(`crates/ffi/src/lib.rs`, `crates/ffi/src/allow_list.rs`, `crates/ffi/src/engine.rs`), its workflow
(`.github/workflows/xcframework.yml`) and its schematic
(`docs/schematics/ffi-adapter-xcframework-and-swift-package.md`), whose dashed Swift package this
harness makes solid.

## The components

The harness is a client at the edge. It depends on no context: it links the adapter's XCFramework,
which depends on the engine alone (ADR-345 D1).

```mermaid
flowchart LR
  subgraph app["ios-harness: the Harness app target"]
    views["DeckListView, ReviewView (SwiftUI, in a NavigationSplitView)"]
    model["HarnessModel (main actor): the review session's state"]
    card["CardWebView: non-persistent store, JavaScript off, a rule list blocking every load, no message handler"]
    session["EngineSession (an actor): holds the one Engine, makes the six calls"]
    fixture["the bundled synthetic collection, copied to a fresh directory at launch"]
    views --> model
    views --> card
    model --> session
    session --> fixture
  end
  subgraph wire["HarnessWire (a package that depends on nothing)"]
    codec["the six calls' requests and responses, as protobuf wire bytes"]
  end
  subgraph pkg["EnginePackage (a local package, filled by CI)"]
    bindings["DeckStreakFFI: the generated Swift bindings, a module of their own"]
    binary["deck_streak_ffiFFI: the binary target, the run's XCFramework"]
    bindings --> binary
  end
  subgraph ffi["deck-streak-ffi, inside the XCFramework"]
    run["Engine::run(service, method, input)"]
    allow["ALLOW_LIST: six pairs"]
    run --> allow
  end
  subgraph engine["Anki's engine (the pinned fork)"]
    backend["Backend::run_service_method"]
  end
  session --> codec
  session --> bindings
  binary --> run
  run --> backend
```

No arrow reaches a DeckStreak context, the coordination crate or the daemon. The context map's
line for the harness reads `depends on: nothing internal`, as the Mini App's does.

## One call, from a view to the engine and back

The deck list's load, as one example; the other five calls take the same path with their own
request and response.

```mermaid
flowchart TD
  tap["DeckListView appears"] --> ask["HarnessModel.loadDecks(), on the main actor"]
  ask --> hop["await EngineSession.deckNames(): leaves the main actor"]
  hop --> encode["HarnessWire encodes GetDeckNamesRequest { include_filtered: true }"]
  encode --> ffi_call["Engine.run(service, method, bytes), through the generated bindings"]
  ffi_call --> lookup{"allowed(service, method)?"}
  lookup -- "no" --> refused(["EngineRefusal.NotAllowed, thrown in Swift"])
  lookup -- "yes" --> backend["Backend::run_service_method"]
  backend -- "Ok(bytes)" --> decode["HarnessWire decodes DeckNames: each entry's name"]
  backend -- "Err(encoded BackendError)" --> engine_refused(["EngineRefusal.Engine, thrown in Swift"])
  decode --> back["the names return to the main actor"]
  refused --> sentence["HarnessModel holds the refusal's sentence"]
  engine_refused --> sentence
  back --> state["HarnessModel.decks = names"]
  state --> render(["DeckListView shows Default, then Synthetic"])
  sentence --> render2(["the view shows the sentence, and the engine keeps serving"])
```

## One review, launch to answer

```mermaid
sequenceDiagram
  participant V as the views
  participant M as HarnessModel
  participant S as EngineSession
  participant E as Engine (FFI)
  V->>M: launch: the signpost interval begins
  M->>S: open()
  S->>S: copy the bundled collection to a fresh directory, with an empty media folder
  S->>E: Engine(message: empty)
  S->>E: run(OpenCollection: path, media folder, media database)
  E-->>S: empty bytes
  M->>S: deckNames()
  S->>E: run(GetDeckNames)
  E-->>S: DeckNames
  S-->>M: ["Default", "Synthetic"]
  M-->>V: the deck list: the signpost interval ends
  V->>M: Study
  M->>S: next()
  S->>E: run(GetQueuedCards: fetch_limit 1)
  E-->>S: QueuedCards: the head card, its states, the counts
  S->>E: run(RenderExistingCard: card id, partial_render false)
  E-->>S: RenderCardResponse: question text nodes
  S-->>M: the card, its states, its question HTML, new count 1
  M-->>V: CardWebView loads the HTML as a string, no base URL
  V->>M: Good
  M->>S: answer(card, states, Good, answered at, time taken)
  S->>E: run(AnswerCard)
  E-->>S: OpChanges
  S->>E: run(GetQueuedCards: fetch_limit 1)
  E-->>S: QueuedCards: new count 0
  S-->>M: new count 0
  M-->>V: the review shows new count 0
```

`GetQueuedCards` reads the collection's current deck, which is `Default` in a new collection; no
allowed call changes it, so `Study` studies `Default` whatever row is highlighted (SPEC-339
section 5). On an iPhone the split view collapses to a stack that starts on the deck list; on an
iPad both columns show. The session's state is the model's, so the collapse keeps it.

## The CI job graph

`xcframework.yml` runs on a pull request into `dev` that changes `crates/ffi/**`, `ios/**`,
`Cargo.lock` or the workflow itself, and on a dispatch. Every job runs on the one macOS runner the
hardening test admits to this file by name, with a read-only token and no secret.

```mermaid
flowchart TD
  trigger["pull_request into dev on the four paths, or workflow_dispatch"] --> xcf
  trigger --> hw
  subgraph xcf["job xcframework (SPEC-336, plus one step)"]
    libs["two static libraries, release, clocked"] --> bind["Swift bindings, library mode"]
    bind --> mm{"modulemap names a builtin module?"}
    mm -- "no" --> assemble["xcodebuild -create-xcframework"]
    assemble --> consumer["the consumer check: the bindings typecheck against each slice"]
    consumer --> fix["cargo run --example harness-fixture: the synthetic collection, by this commit's engine"]
    fix --> up1(["upload xcframework: the framework, the bindings, the fixture, the report"])
  end
  subgraph hw["job harness-wire (needs nothing, builds no Rust)"]
    wt["swift test --package-path ios/HarnessWire: A2, A3"] --> sweep["the Swift mutant sweep: each killer green alone, then red on its mutant, the file restored and checked"]
    sweep --> up2(["upload harness-wire: the test log and the sweep's report"])
  end
  up1 --> h
  subgraph h["job harness (needs xcframework)"]
    dl["download this run's xcframework artifact"] --> place["place the framework and the bindings in EnginePackage, the fixture in the app's resources"]
    place --> gen["XcodeGen, pinned and digest-checked: generate ios/Harness.xcodeproj"]
    gen --> dbg["xcodebuild test, Debug, the iPhone then the iPad simulator: A4 to A8"]
    dbg --> rel["xcodebuild test, Release, the measurement tests only, both simulators"]
    rel --> size["the Release simulator bundle's and executable's bytes"]
    size --> syms["the executable's imported required-reason symbols against the privacy manifest"]
    syms --> rep["report, always: each test's status and first failure, each metric, each size"]
    rep --> up3(["upload harness, always: the report and both result bundles"])
  end
```

The Linux CI runs A1 (the render round trip) with the adapter's other tests, and A9 and A10 with
the hardening tests, on every pull request; those three never need the macOS runner. A failed
`xcframework` job skips `harness`, because its `needs` failed, and its report still uploads.
`harness-wire` runs beside both and fails on its own.

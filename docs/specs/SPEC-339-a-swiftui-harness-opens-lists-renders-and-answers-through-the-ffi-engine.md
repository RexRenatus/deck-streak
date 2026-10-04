# SPEC-339: a SwiftUI harness opens, lists, renders and answers through the FFI engine on the iPhone and iPad simulators

- **Issue:** #616, the app campaign's Swift harness spike. **Context(s):** `ios-harness`, a client
  at the edge under `ios/` that depends on no context and links the adapter's XCFramework; and
  `deck-streak-ffi`, whose allow-list gains one read call.
- **Decided by:** ADR-350 (how the project is produced, how the harness links the framework its
  own run built, the render call, the synthetic collection, the wire codec, how a test is seen red,
  how each figure is measured, the Swift mutant sweep, and the seam the internal build lane
  inherits), resting on ADR-335 (the client), ADR-342 (one universal app), ADR-344 (internal
  builds) and ADR-345 (the adapter, its generator and the macOS job).
- **Status:** a spike, delivered by the draft pull request that adds this file, with its tests and
  `docs/red-first/SPEC-339.md`. **Mutation band:** `S33900-S33999`.

## 1. The problem, measured

ADR-335's decision outcome waits on the iOS spike's two halves. The Rust half (SPEC-336) builds the
adapter as an XCFramework with a device slice and a simulator slice, generates the Swift bindings
and typechecks them against each slice's headers. It links nothing and runs nothing on a simulator
(SPEC-336 section 5). Its measured slices are static libraries before linking: 81.0 MiB each
(SPEC-336 section 7). What an app that links one weighs, how fast it starts and how much memory it
holds is not yet known.

What was read before this SPEC was written:

- The allow-list holds five calls: `OpenCollection`, `GetDeckNames`, `GetQueuedCards`,
  `AnswerCard` and `Undo` (`crates/ffi/src/allow_list.rs`). None renders a card. A queued card
  carries the card, its queue and its scheduling states and no rendered face (the round-trip test's
  `read_queue` reads fields 1 to 3 of the queue's head, `crates/ffi/tests/round_trip.rs`).
- The engine renders an existing card with `CardRenderingService.RenderExistingCard`. Its request is
  `card_id` (1), `browser` (2) and `partial_render` (3); its response is `question_nodes` (1),
  `answer_nodes` (2) and `css` (3), and each node is either `text` (1) or `replacement` (2)
  (`proto/anki/card_rendering.proto` at the engine's pinned rev).
- No allowed call writes a note or a deck, so a client cannot build its own collection through the
  allow-list. The round-trip tests build theirs with the engine's own Rust API before the adapter
  runs (`synthetic()` in `round_trip.rs`).
- `GetQueuedCards` reads the collection's current deck, and no allowed call changes it. A new
  collection's current deck is `Default`, where the synthetic note's card sits.
- `xcframework.yml` runs on the one macOS runner the workflow hardening test admits, by this file's
  name alone (`ADMITTED_RUNNERS` in `scripts/tests/test_ci_workflows.py`), and uploads the
  framework and the bindings as its `xcframework` artifact.
- Nothing in Swift compiles anywhere but on that runner, so every Swift criterion below is decided
  there.

## 2. Requirements

R1. The harness is one universal SwiftUI application target for iPhone and iPad under `ios/`. Its
    Xcode project is generated from a committed XcodeGen spec, `ios/project.yml`, by a pinned and
    digest-checked XcodeGen, and the generated project is never committed. Build settings live in
    committed `.xcconfig` files; the Info.plist and the privacy manifest are committed files the
    spec points at, never generated.
R2. The harness links the XCFramework and the Swift bindings that the same workflow run built from
    the same commit, through a local Swift package, `ios/EnginePackage`: a binary target for the C
    module `deck_streak_ffiFFI` and a Swift target, `DeckStreakFFI`, that holds the generated
    bindings in a module of their own.
R3. At launch the harness copies its bundled synthetic collection into a fresh directory in its own
    container, starts one engine from an empty init message, and opens the copy with
    `OpenCollection`, passing the collection's path, its media folder and its media database. It
    shows the open's outcome.
R4. It lists the collection's decks by name with `GetDeckNames` (`include_filtered` true), in the
    order the engine returns them.
R5. It gets the current deck's next card with `GetQueuedCards` (`fetch_limit` 1), renders its
    question with `RenderExistingCard` (`partial_render` false), and shows the question's text
    nodes, joined, as HTML in a WKWebView. A response holding a `replacement` node is shown as a
    refusal sentence, never as raw text.
R6. The card's web view uses a non-persistent website data store, page JavaScript off, a compiled
    content rule list that blocks every load, and no script message handler, and it is handed the
    HTML as a string with no base URL.
R7. The harness answers the shown card Good with `AnswerCard`: the card's id, the queued card's
    current state and its Good state, the rating Good, the answer time in milliseconds and the time
    taken. It then reads the queue again and shows the deck's new count.
R8. Every engine call runs off the main actor, through one actor that holds the engine. The review
    session's state lives in a model, never in a view's state, so a size-class change or a scene
    reconnect keeps it. An engine refusal is shown as its own sentence.
R9. Requests and responses are encoded and decoded by a hand-written protobuf wire codec in its own
    Swift package, `ios/HarnessWire`, which depends on nothing and covers exactly the fields the
    six calls send and read. Its tests run on the macOS host with `swift test`, no simulator.
R10. The adapter's allow-list gains exactly one pair, `CardRenderingService.RenderExistingCard`, at
    the service and method indices the engine's generated dispatch gives at the pinned rev, read
    there the way SPEC-336 section 1 read the five. The table then holds six pairs, and every
    other call is refused `NotAllowed` as before.
R11. The synthetic collection is written in CI by the engine of the same commit, through an example
    of the adapter crate, `harness-fixture`, which takes an output directory and writes the
    collection the round-trip tests build: one Basic note, front `synthetic front` and back
    `synthetic back`, in `Default`, and a second, empty deck, `Synthetic`. The example and the
    tests share one builder, `crates/ffi/tests/support/synthetic.rs`.
R12. `xcframework.yml` gains two jobs on its admitted runner. `harness-wire` runs the codec's tests
    and its Swift mutant sweep. `harness` needs `xcframework`, downloads that run's `xcframework`
    artifact (no other run's), generates the project, runs the tests on one iPhone and one iPad
    simulator, measures, writes a report, and uploads the report and the result bundles as its
    `harness` artifact whatever the outcome. The pull-request paths gain `ios/**`. No job reads a
    secret.
R13. The tree carries the seam the internal build lane inherits, and no signing material: a shared
    scheme, `Harness`; the bundle id read from one build setting, `DS_APP_ID`, whose committed
    value is a reserved placeholder no upload can carry; no development team anywhere; the
    Info.plist's `CFBundleVersion` and `CFBundleShortVersionString` read from
    `CURRENT_PROJECT_VERSION` and `MARKETING_VERSION`; the Info.plist's `CFBundlePackageType` the
    literal `APPL` and `ITSAppUsesNonExemptEncryption` false; a `PrivacyInfo.xcprivacy` in the app
    target that declares each required-reason API category the linked binary calls; and a CI build
    that turns code signing off on its command line, never in the tree.
R14. The harness measures, on each simulator, the app's size, its cold start and its memory, as
    section 7 states, and reports each figure in the run's summary and in the `harness` artifact.

## 3. Acceptance criteria

The Swift criteria run in the `harness-wire` and `harness` jobs, after their preparation steps:
`$IPHONE_SIM` and `$IPAD_SIM` are the job's two pinned simulator names, and `ios/Harness.xcodeproj`
is the project the job generated. A simulator criterion holds only when its test passes on both.

| id | criterion | decided by |
|---|---|---|
| A1 | an allowed `RenderExistingCard` renders the synthetic note's queued card, and its question's text nodes hold `synthetic front` | `cargo test -p deck-streak-ffi --test render -- --exact a1_renders_the_queued_cards_question` |
| A2 | the codec encodes each of the six requests as the literal bytes the wire format gives for fixed inputs | `swift test --package-path ios/HarnessWire --filter HarnessWireTests.RequestBytesTests/test_a2_each_request_encodes_to_its_literal_bytes` |
| A3 | the codec decodes deck names, the queue's head and counts, and a render's text and replacement nodes from literal response bytes | `swift test --package-path ios/HarnessWire --filter HarnessWireTests.ResponseDecodingTests/test_a3_each_response_decodes_from_its_literal_bytes` |
| A4 | the harness opens its bundled synthetic collection at launch, and shows that it did | `xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessUITests/HarnessFlowTests/test_a4_opens_the_synthetic_collection` |
| A5 | the deck list names `Default` then `Synthetic`, and nothing else | the same command, `-only-testing:HarnessUITests/HarnessFlowTests/test_a5_lists_the_collections_decks` |
| A6 | studying shows the queued card, and its web view shows `synthetic front` | the same command, `-only-testing:HarnessUITests/HarnessFlowTests/test_a6_renders_the_queued_cards_question` |
| A7 | answering Good leaves the deck's new count at 0, where it read 1 before the answer | the same command, `-only-testing:HarnessUITests/HarnessFlowTests/test_a7_answers_the_card_good` |
| A8 | the card's web view uses a non-persistent data store and has page JavaScript off | the same command, `-only-testing:HarnessTests/CardWebViewTests/test_a8_the_card_web_view_is_isolated` |
| A9 | the `harness` job needs `xcframework` and downloads the `xcframework` artifact of its own run, naming no other run | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_links_the_framework_its_own_run_built` |
| A10 | the harness tree carries the seam (package type, version settings, export answer, privacy manifest, the placeholder app id) and no team, profile, key or signing switch; a planted tree with each is refused by name | `python3 -m unittest discover -s scripts/tests -p test_ios_harness_tree.py -k test_the_harness_tree_carries_the_seam_and_no_signing_material` |

```acceptance
A1: cargo test -p deck-streak-ffi --test render -- --exact a1_renders_the_queued_cards_question
A2: swift test --package-path ios/HarnessWire --filter HarnessWireTests.RequestBytesTests/test_a2_each_request_encodes_to_its_literal_bytes
A3: swift test --package-path ios/HarnessWire --filter HarnessWireTests.ResponseDecodingTests/test_a3_each_response_decodes_from_its_literal_bytes
A4: xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessUITests/HarnessFlowTests/test_a4_opens_the_synthetic_collection
A5: xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessUITests/HarnessFlowTests/test_a5_lists_the_collections_decks
A6: xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessUITests/HarnessFlowTests/test_a6_renders_the_queued_cards_question
A7: xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessUITests/HarnessFlowTests/test_a7_answers_the_card_good
A8: xcodebuild test -project ios/Harness.xcodeproj -scheme Harness -destination "platform=iOS Simulator,name=$IPHONE_SIM" -destination "platform=iOS Simulator,name=$IPAD_SIM" -disable-concurrent-destination-testing -only-testing:HarnessTests/CardWebViewTests/test_a8_the_card_web_view_is_isolated
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_links_the_framework_its_own_run_built
A10: python3 -m unittest discover -s scripts/tests -p test_ios_harness_tree.py -k test_the_harness_tree_carries_the_seam_and_no_signing_material
```

The tdd probe resolves A1, A9 and A10; it resolves no `swift test` or `xcodebuild test` form, so
A2 to A8 name their XCTest ids in the table and are decided by the macOS jobs' result bundles,
which the `harness` report quotes test by test. The literal bytes in A2 and A3 are spelled in the
tests, computed by hand from the wire format (a varint 300 is `ac 02`), never produced by the
codec under test. A4 to A7 run against the collection R11 writes; no real collection data is read.
The UI tests launch the app fresh for each test, so each starts from the one new card. A8 asserts
the two layers the web view's configuration exposes; the content rule list and the absence of a
message handler are not observable through that API, and the planted-card proof of all four is
#619's. The measurement tests of section 7 are not criteria: each measures and asserts nothing, so
the red-first record discloses them as not red.

## 4. File manifest

| file | context | change |
|---|---|---|
| `ios/project.yml` | `ios-harness` | added: the XcodeGen spec, three targets and the shared scheme |
| `ios/Config/Harness.xcconfig` | `ios-harness` | added: build settings, the `DS_APP_ID` placeholder, an optional include of an ignored local file |
| `ios/Harness/Info.plist` | `ios-harness` | added |
| `ios/Harness/PrivacyInfo.xcprivacy` | `ios-harness` | added |
| `ios/Harness/Sources/HarnessApp.swift` | `ios-harness` | added: the app, the split view |
| `ios/Harness/Sources/EngineSession.swift` | `ios-harness` | added: the actor that holds the engine and makes the six calls |
| `ios/Harness/Sources/HarnessModel.swift` | `ios-harness` | added: the review session's state, on the main actor |
| `ios/Harness/Sources/DeckListView.swift` | `ios-harness` | added |
| `ios/Harness/Sources/ReviewView.swift` | `ios-harness` | added |
| `ios/Harness/Sources/CardWebView.swift` | `ios-harness` | added: the isolated web view and its configuration |
| `ios/Harness/Sources/Signposts.swift` | `ios-harness` | added: the launch-to-deck-list interval |
| `ios/HarnessTests/CardWebViewTests.swift` | `ios-harness` | added: A8 |
| `ios/HarnessUITests/HarnessFlowTests.swift` | `ios-harness` | added: A4 to A7 |
| `ios/HarnessUITests/HarnessMeasureTests.swift` | `ios-harness` | added: section 7's cold start and memory |
| `ios/EnginePackage/Package.swift` | `ios-harness` | added: the binary target and the bindings target |
| `ios/HarnessWire/Package.swift` | `ios-harness` | added |
| `ios/HarnessWire/Sources/HarnessWire/Wire.swift` | `ios-harness` | added: varints, length-delimited fields, the reader |
| `ios/HarnessWire/Sources/HarnessWire/Messages.swift` | `ios-harness` | added: the six calls' requests and responses |
| `ios/HarnessWire/Tests/HarnessWireTests/RequestBytesTests.swift` | `ios-harness` | added: A2 |
| `ios/HarnessWire/Tests/HarnessWireTests/ResponseDecodingTests.swift` | `ios-harness` | added: A3 |
| `ios/HarnessWire/swift-mutants.json` | `ios-harness` | added: the codec's hand-written mutants and their killers |
| `crates/ffi/src/allow_list.rs` | `deck-streak-ffi` | the sixth pair: R10 |
| `crates/ffi/tests/support/mod.rs` | `deck-streak-ffi` | added: the test support module: a scratch directory per test, the open request, and the two files below |
| `crates/ffi/tests/support/synthetic.rs` | `deck-streak-ffi` | added: the synthetic builder, which the example includes alone |
| `crates/ffi/tests/support/wire.rs` | `deck-streak-ffi` | added: the wire helpers, moved from `round_trip.rs` |
| `crates/ffi/tests/round_trip.rs` | `deck-streak-ffi` | reads the moved helpers; its tests unchanged |
| `crates/ffi/tests/render.rs` | `deck-streak-ffi` | added: A1 |
| `crates/ffi/examples/harness-fixture.rs` | `deck-streak-ffi` | added: R11 |
| `.github/workflows/xcframework.yml` | none (CI) | the fixture step, the two jobs, the `ios/**` path |
| `scripts/tests/test_ci_workflows.py` | none (the gate) | A9 and its planted jobs; `plistlib` joins the read census's vetted modules, since A10 parses its plists with it |
| `scripts/tests/test_ios_harness_tree.py` | none (the gate) | added: A10 and its planted trees |
| `scripts/mutation-rows.d/S33900-S33999.json` | none (the gate) | added: the sixth pair's literal |
| `.gitignore` | the workspace | the generated project, the placed framework, bindings and fixture, result bundles, the local signing include |
| `docs/CONTEXT-MAP.md` | the map | the harness's line |
| `docs/schematics/swift-harness-on-the-ffi-engine.md` | the record | added |
| `docs/red-first/SPEC-339.md` | the record | added |
| `docs/specs/SPEC-339-a-swiftui-harness-opens-lists-renders-and-answers-through-the-ffi-engine.md` | the record | added |
| `docs/decisions/ADR-350-the-swift-harness-is-a-generated-project-over-its-own-runs-framework.md` | the record | added |
| `docs/specs/SPEC-336-a-native-client-reaches-the-engine-through-one-allow-listed-ffi-crate.md` | the record | two amendment paragraphs, inserted: its manifest's two missing rows, and the run its section 7 figures came from |
| `docs/decisions/ADR-335-the-iphone-and-ipad-client-is-swiftui-over-the-engine-through-ffi.md` | the record | the decision outcome, with section 7's figures, after the measured run |
| `changelog.d/swift-harness-339.md` | the record | added |

## 5. What this does NOT cover

- It signs, archives and uploads nothing, and builds no device slice of the app: the internal
  build lane does, and inherits R13's seam (#634).
- It takes no figure on a device; simulator figures are the host's (#629).
- It does not prove the card isolation against planted cards that try the network or the app; it
  configures the four layers and asserts the two its API exposes (#619).
- It selects no deck, syncs nothing and signs nobody in: it studies the collection's current deck,
  and no allowed call changes it (#625).
- It shows no answer side, no other rating, no intervals, no undo, no remote and no keyboard
  command (#632, #633).
- It ships no Swift package over the engine core, the XP crate or the FSRS-7 crate; its package
  wraps this spike's adapter only (#624).
- It allows no call beyond the six; the engine core's dispatcher takes the table over (#623).
- It runs on no tag and on no Apple path outside `ios/` and the adapter's (#622).
- It generates no message types: the codec covers six calls, and the app shell chooses its protobuf
  library (#625).
- Its Swift mutants are proved by each run of the codec job and join no shared mutation population
  (#650).
- It sets no budget for size, cold start or memory; it measures them (#629).

## 6. Risks

- The engine numbers the render call differently from the indices read. Detected by A1, which calls
  the pair by literal indices and decodes its answer, and by the band's row on the pair.
- One of SPEC-336's four unlisted pairs is the render pair, so its A6 starts failing. Detected by
  SPEC-336 A6 itself; the build replaces that pair with another unlisted one and says so.
- The runner image drops a pinned simulator name. Detected by the `harness` job, whose destination
  step fails naming it.
- The engine reads or writes outside the copied directory, which the app's container refuses on
  the simulator. Detected by A4, which shows the open's refusal sentence.
- The privacy manifest misses a category the linked binary calls. Detected by the job's symbol
  step, which lists the binary's imported required-reason symbols beside the manifest's
  categories and fails on a category used and undeclared; an upload would be refused for the same.
- A push while the red run is in flight cancels it, and the red is never read. Detected by the
  red-first record, which needs the run's quoted failures; the build brief forbids that push.
- A pull request that touches only `ios/**` rebuilds both static libraries, because the harness
  links only what its own run built. Detected by the run's clock in section 7.
- XCTest's launch metric on a simulator measures the host's launch, not a device's. Disclosed in
  section 7; the device figures are #629's.

## 7. Measured by the harness job

The `harness` job measures each figure on the commit's Release build, unsigned, on each simulator,
and writes them to the run's summary and to `harness-report/report.md` in the `harness` artifact.
The figures below are run 37233368979's, at commit 33fb20d9.

| measure | how it is measured | iPhone simulator | iPad simulator |
|---|---|---|---|
| app size, bundle | the bytes of every file in the Release simulator `Harness.app`, built with no test action, summed | 35286275 bytes | the same bundle |
| app size, executable | the bytes of the Release simulator executable, the engine linked and dead-stripped | 35145000 bytes | the same bundle |
| cold start, first frame | `XCTApplicationLaunchMetric(waitUntilResponsive: true)`, five measured launches after one discarded, the median | 3.47 s (passes 3.03 to 3.65 s) | 10.57 s (passes 5.26 to 12.99 s) |
| cold start, open to deck list | `XCTOSSignpostMetric` over the harness's own interval, from its first line to the deck names shown, five launches after one discarded, the median | 1.96 s (passes 1.69 to 2.18 s) | 3.03 s (passes 2.41 to 9.41 s) |
| memory | `XCTMemoryMetric(application:)` peak physical memory over one launch, open, list, render and answer, five passes after one discarded, the largest | 58251.9 kB (passes 57727.6 to 58251.9 kB) | 70244.9 kB (the other four passes 59103.7 to 60087.0 kB) |
| two static libraries and the bindings | the `xcframework` job's clock, as SPEC-336 section 7 reports it | 7.6 minutes for the two libraries, 1.6 minutes for the bindings; the job, 10.4 minutes | |
| the harness job | its clock, from the artifact download to the report | 28.1 minutes, of which the Debug tests took 13.7 and the Release measurements 13.7 | |

The iPad simulator's passes spread widely in this run. The run before it, at b9dbe8ca (run
37230471346), read medians of 4.00 s to the first frame and 1.99 s to the deck list on the iPad
simulator and 4.06 s and 2.26 s on the iPhone simulator, and largest memory peaks of 63068.9 kB on
the iPad simulator and 61987.4 kB on the iPhone simulator. So one run's figures are a reading, not
a baseline. That run's bundle size is not quoted, because its Release app was the one the test
action built, with the test bundle and XCTest's frameworks copied in.

Every launch and memory figure is read from the result bundle with `xcrun xcresulttool get
test-results metrics`, and a figure the bundle does not hold reads `not measured`, never zero. A
simulator runs on the runner's own processor and memory, so these figures compare one commit with
another on one runner image; they are not a device's.

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

## 8. Amendments for #1060: each Harness test step is built once and each simulator warmed before it

On #1060's branch the `harness` job failed in 11 of its 14 attempts; 2 were cancelled with their
run and 1 passed, each attempt read from its run's own attempt job list. Each failure sits in one
simulator session of one test step. This section amends R12 and adds R15 and R16, which ADR-350
D10 to D12 decide and A11 (section 10) holds. R1 to R11, R13, R14, section 7 and A1 to A10 stand
as written.

What failed in the two Harness test steps, read from each job's steps and from its log cut to the
failing step's window:

- **A launch timed out** at the first UI test of the Debug step (A4, `HarnessFlowTests.swift:66`),
  in 4 sessions: jobs 124741023204 and 124680855585, and both simulators of job 124751411198.
  The failing launch tests ran from 152.681 seconds (job 124680855585)
  to 327.577 seconds (job 124751411198).
- **The UI test runner exited before its first test** ("operation never finished bootstrapping"),
  in the Debug step, in 2 sessions: jobs 124741023204 and 124868223996. In job 124868223996 the
  runner exited 227 seconds (job 124868223996) after its "Running tests..." line, on the iPhone simulator.
- **An end timed out** ("Failed to terminate") in the Release step, in 4 sessions: M1 at
  `HarnessMeasureTests.swift:33` in jobs 124596873613, 124566920296 and 124540842089, and M2 at
  `:54` in job 124840259780, the one on #1060's head. XCTest waited 61.8 seconds on the end before
  it failed (job 124540842089). Each of the four began within 6 minutes of its session's start, on
  the fourth to eighth launch of the build the step installs (3.1 minutes in job 124540842089,
  5.7 minutes in job 124840259780).
- **Every session starts with a wait.** Before its first test process wrote a line, each session
  of each test step waited 79 to 149 seconds in the one passing attempt (job 124780704116), and
  the Debug step waited 211 and 360 seconds in job 124868223996.

No assertion of A4 to A7, M1 or M2 failed in these sessions: each failure is a launch, a runner
start-up or an end that ran out of time near a session's start. The app's review steps failed too,
in 3 attempts at earlier heads; section 9 leaves them.

- **R12, as #1060 amends it: each Harness test step is built once, warmed, then run without
  building.** The Debug tests and the Release measurements each run as three steps. The first
  builds the scheme for testing (`xcodebuild build-for-testing`) for both simulators, into the
  step's derived data. The second warms each simulator in turn, `$IPHONE_SIM` and then
  `$IPAD_SIM`: it waits for the simulator's boot to finish (`xcrun simctl bootstatus <udid> -b`),
  installs the app the build made, launches it and ends it (`xcrun simctl install`, `launch` and
  `terminate`), with the bundle id read from the built app's `Info.plist`. The third runs the
  tests without building (`xcodebuild test-without-building`), with the destinations, the
  selection, the derived data and the result bundle the step had before. The Debug warm writes
  the warm's script to the runner's temporary directory once, and the Release warm runs that copy.
- **R15. The warm records, and the tests decide.** Each phase's outcome and seconds go to
  `harness-report/warm-debug.json` or `harness-report/warm-release.json`. The warm fails its step
  only when a simulator name is not exactly one available simulator on the job's runtime, which
  the destination would refuse too; a phase that fails or runs out of time is recorded, and the
  tests run. One budget of 480 seconds bounds each warm step, both simulators together, and a
  phase past it is recorded as skipped. No test, wait, retry, attempt count, destination or bound
  changes: the job's `timeout-minutes` stays 150 (SPEC-361 A16).
- **R16. The report records each simulator's session start.** For the Debug and the Release step
  and each simulator, the report's measure table gains five rows: the warm's boot wait, with the
  simulator's state before it, its launch and its end, from the warm's record; and the UI test
  runner's bootstrap, from its "Running tests..." line to its first suite's start, and the
  first-launch gap, from the first UI test's start to xcodebuild's launch parameters line, from
  the step's console. Each test step writes its console to `harness-report/debug-console.log` or
  `harness-report/release-console.log` through `tee`, under `set -o pipefail`, so the step's exit
  stays xcodebuild's. A figure the report cannot read reads `not measured`, never zero.
- **A11.** Section 10's test holds R12 as amended, R15's record-only warm and R16's rows; a
  planted job missing each part is refused by its rule's name.
- **A4 to A8, as #1060 runs them.** Their tests, destinations and selections are unchanged; CI runs
  them without building, over the build the step before made. The commands section 3 names stay
  the form a person runs, which builds and tests in one.
- **Risks of the #1060 amendment.** The warm adds time to the job: its budget caps each warm step
  at 480 seconds, so both add at most 16 minutes, and the slowest of the three passing attempts
  took 127.7 minutes (job 123875777769), so a run that passes still ends inside its 150 minutes.
  The build for testing spares the second simulator's rebuild (15 seconds in job 124780704116).
  A launch or an end that fails in the warm is recorded and fails nothing, so the warm cannot turn
  a passing run red; a name that matches two simulators fails the warm step by name. Without
  `set -o pipefail`, `tee` would hide xcodebuild's exit; A11 refuses a teed step without it. If the
  classes recur at the same rate after the warm, a session's start is not their cause, and R16's
  rows show where each session spent its start (#1060).
- **The #1060 amendment touches these paths:** `.github/workflows/xcframework.yml`,
  `scripts/tests/test_ci_workflows.py`,
  `scripts/mutation-rows.d/S33900-S33999.json` (rows S33901 to S33906, in this SPEC's band),
  `docs/specs/SPEC-339-a-swiftui-harness-opens-lists-renders-and-answers-through-the-ffi-engine.md`,
  `docs/decisions/ADR-350-the-swift-harness-is-a-generated-project-over-its-own-runs-framework.md`,
  `docs/red-first/SPEC-339.md` and `changelog.d/ci-harness-warm.md`.

## 9. What this does NOT do, in the #1060 amendment

- It retries nothing, and removes, skips or loosens no test, wait or bound: a launch, an end or a
  runner start-up that runs out of time still fails its test (#1060).
- It warms neither the card view's planted suite, which failed in no attempt, nor the app's three
  test steps, whose failures came at earlier heads (an end that timed out, a launch refused its
  background assertion, and a UI query that timed out); SPEC-347 A14's test holds the app's step
  order, and a later delivery decides whether those steps take a warm (#1060).
- It does not decide which part of a session's start is slow: R16's rows measure it on every
  run (#1060).
- It takes no figure on a device (#629).

## 10. Acceptance criteria of the #1060 amendment

| id | criterion | decided by |
|---|---|---|
| A11 | the `harness` job builds each Harness test step for testing, warms each simulator (boot wait, install, launch, end) before it and runs it without building, in that order; its warm records each phase and fails only on a name that is not one simulator; its teed test steps keep xcodebuild's exit; and its report reads the warm's and each step's console rows; a planted job missing each part is refused by name | `scripts/tests/test_ci_workflows.py` `TheHarnessWarmsEachSimulatorBeforeItsTests/test_the_harness_boots_and_launches_each_simulator_once_before_its_tests` |

```acceptance
A11: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_boots_and_launches_each_simulator_once_before_its_tests
```

A11's test is committed alone and read red in the pull request's own run, by name, before the
workflow changes; `docs/red-first/SPEC-339.md` quotes that red and names the green.

## 11. Amendments for #1074: a warm budget for each simulator, and each session's longest end

The warm's first run in the `harness` job (job 124927199265) stopped at the Debug tests. This
section amends R15 and R16 as section 8 wrote them and adds A12, which ADR-350 D13 and D14 decide
and section 13 holds. Every other requirement and criterion, those of section 8 included, stands.

What the run measured, read from the job's steps and from its log cut to each step's window:

- **The first simulator spent the warm's one budget.** Both simulators were shut down when the
  Debug warm began. On the iPhone simulator the boot wait took 135.7 seconds (job 124927199265),
  the install 215.0 seconds (job 124927199265), the launch 80.8 seconds (job 124927199265) and
  the end 5.3 seconds (job 124927199265). The iPad simulator's boot wait had what was left of the
  480 and ran out at 41.2 seconds (job 124927199265); its install, launch and end were skipped.
- **An end timed out after the warm.** On the warmed iPhone simulator, A5's launch ended A4's
  instance of the app, and XCTest waited 60.7 seconds (job 124927199265) before "Failed to
  terminate"; the warm's own end there had taken 5.3 seconds (job 124927199265), and the step's
  four other ends took 2.1 to 2.9 seconds (job 124927199265). On #1060's head the Release step
  ended an earlier instance 18 times: 16 ends took 1.0 to 4.6 seconds (job 124840259780), one took
  37.5 seconds and its test passed (job 124840259780), and one failed at 60.35 seconds (job
  124840259780). An end that times out is the tail of XCTest's own ends, at a session's second UI
  launch (job 124927199265) or its eighth (job 124840259780), warmed or not; no warm phase runs it.
- **The iPad simulator's session passed every test.** Its boot, begun by the warm and cut by the
  budget, had finished by its turn.
  Its first test process wrote a line 35 seconds (job 124927199265) after the iPhone simulator's
  session ended, against 360 seconds in job 124868223996.
- **The Debug tests' session start grew.** From the step's start to the iPhone simulator's first
  test process line took 772 seconds (job 124927199265), against 79 to 149 seconds in job
  124780704116 and 211 seconds in job 124868223996. The iPad simulator's boot, which the warm had
  begun and not finished, ran on through that start; whether it slowed the start is not measured.

- **R15, as #1074 amends it: each simulator's warm has a budget of its own.** The warm sets each
  simulator's deadline when it reaches that simulator, after it finds the simulator's device and
  before its first phase: 480 seconds for each simulator in the Debug warm, which boots them, and
  120 seconds for each in the Release warm, which finds them booted. A phase past its simulator's
  deadline is recorded as skipped. The rest of R15 stands.
- **R16, as #1074 amends it: the report records each session's longest end.** For the Debug and
  the Release step and each simulator, the measure table gains a sixth row: the longest time
  XCTest took to end an earlier instance of the app at a launch, with the count of ends it read,
  from the step's console by XCTest's own seconds (a `Terminate` activity line to the next
  activity line). A session with no end reads `no end in the session`, and a count of runner
  sessions other than the simulators' reads `not measured`. The rest of R16 stands.
- **A12.** Section 13's test holds R15 and R16 as this section amends them; a planted job with one
  deadline for both simulators, a deadline per phase, two deadlines, the one budget's figure, one
  budget for both steps, or a missing longest end is refused by its rule's name.
- **A11's test, as this section changes it.** Its census reads the phases' bound, `left =
  deadline - time.monotonic()`, where it read the one budget's line, and its "no budget" plant
  removes that bound; A12's test reads the budget for each simulator. A11 stands as section 10
  states it.
- **Risks of the #1074 amendment.** The warm's budgets now add at most 20 minutes, not 16: two of
  480 seconds before the Debug tests and two of 120 before the Release measurements. This run's
  Debug build and tests took 1263 seconds (job 124927199265), 83 seconds more than the Debug step
  of the slowest passing job (job 123875777769), which took 127.7 minutes in all (job
  123875777769); with the 20 minutes, a run that passes ends near 149.1 minutes (job 123875777769),
  before any change in the Release steps, which this run did not reach. A run as slow as this one
  to the end of its Debug tests (job 124927199265) and as slow after them as job 124483818683
  would pass 150 minutes, under one budget or four. A run past 150 minutes is cancelled red and
  names no test; the bound does not move (SPEC-361 A16). A Release warm phase past its 120 seconds
  is recorded, and the tests decide. The longest end rows measure the tail of the ends on every
  run, red or green; they do not shorten it.
- **The #1074 amendment touches these paths:** `.github/workflows/xcframework.yml`,
  `scripts/tests/test_ci_workflows.py`,
  `scripts/mutation-rows.d/S33900-S33999.json` (rows S33907 and S33908, in this SPEC's band),
  `docs/specs/SPEC-339-a-swiftui-harness-opens-lists-renders-and-answers-through-the-ffi-engine.md`,
  `docs/decisions/ADR-350-the-swift-harness-is-a-generated-project-over-its-own-runs-framework.md`,
  `docs/red-first/SPEC-339.md` and `changelog.d/ci-harness-warm.md`.

## 12. What this does NOT do, in the #1074 amendment

- It does not cure an end that times out: no warm phase runs XCTest's own end, and a retry or a
  longer wait stays refused; the longest end rows measure the ends on every run (#1074).
- It does not decide why the Debug session's start grew after the warm (#1074).
- It retries nothing, and removes, skips or loosens no test, wait or bound (#1074).

## 13. Acceptance criteria of the #1074 amendment

| id | criterion | decided by |
|---|---|---|
| A12 | the `harness` job's warm sets each step's budget once (480 seconds a simulator before the Debug tests, 120 before the Release measurements) and each simulator's deadline inside its simulator loop, before its phases; and its report reads each step's longest end per simulator; a planted job missing each part is refused by name | `scripts/tests/test_ci_workflows.py` `TheHarnessWarmsEachSimulatorBeforeItsTests/test_the_warm_budgets_each_simulator_and_the_report_reads_its_longest_end` |

```acceptance
A12: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_warm_budgets_each_simulator_and_the_report_reads_its_longest_end
```

A12's test is committed with the docs, apart from the workflow, and read red in the pull
request's own run, by name, before the workflow changes; `docs/red-first/SPEC-339.md` quotes that
red and names the green.

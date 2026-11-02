# SPEC-349: a card face renders on iPhone and iPad in a web view that reaches neither the app nor the network

- **Wave:** the app campaign, Phase 0 (SPEC-334 row 1.2, "the card HTML sandbox"; R6). **Issue:**
  #619, its iPhone and iPad half. **Context(s):** `ios-harness` (`ios/`).
- **Decided by:** ADR-360 (this SPEC's own: the iOS layer mechanics and the probe) and
  ADR-352 (card scripts off on both platforms, the channel inventory), which the web delivery
  carries; ADR-335 (card faces in an isolated web view). It closes SEC01-F14 (peer connections from
  card script) and SEC01-F15 (the card frame's other channels) for iPhone and iPad.
- **Schematic:** `docs/schematics/card-frame-channels.md` section 4, which this delivery amends with what its
  suite measured, in an appended section 6.
- **Status:** this pull request delivers R1 to R10, with its tests and
  `docs/red-first/SPEC-349.md`.

## 1. The problem, measured

- ADR-335 names three layers for a card face: a non-persistent store, no message handler on the
  card's frame, and a content rule list that blocks the network. #616's harness (SPEC-339) builds
  a card view with all three and page JavaScript off, and proves two of them through the
  configuration API; it states that the rule list and the absence of a handler cannot be observed
  that way and leaves their proof against planted cards to #619 (its SPEC R6 and its exclusion
  naming #619).
- None of the three layers stops a peer connection from card script: a content rule list governs
  loads, and a peer connection is not a load (SEC01-F14).
- Nothing names what a navigation the card starts itself, a window it opens, or a file URL it
  names reaches (SEC01-F15).
- On the current OS releases, App Transport Security refuses a web view's plain-HTTP load of the
  loopback address unless the app declares local networking, so a probe on the loopback address
  needs a host app that declares it; the shipped app must not.

Count the files that construct a card web view at the cut sha with
`git grep -n -E 'WKWebView\(|WKWebViewConfiguration\(' 611d7427a5ad1f7d090c03cb7329ee8b5f438f28 -- ios`
(one, `ios/Harness/Sources/CardWebView.swift`, at lines 16 and 34). The same file sets the store,
page JavaScript and the rule list, adds no handler and loads the card with no base URL; it sets no
navigation delegate and no UI delegate
(`git grep -n -E 'navigationDelegate|uiDelegate' 611d7427a5ad1f7d090c03cb7329ee8b5f438f28 -- ios`
reads nothing).

## 2. Requirements

R1. `CardWebViewFactory.makeCardWebView(html:)` in the `CardIsolation` package is the only code
    under `ios/`, test targets aside, that constructs a `WKWebView` or a
    `WKWebViewConfiguration`. The harness's `CardWebView` calls it, and nothing under `ios/` adds a
    script message handler or calls `loadFileURL`.
R2. The view the factory returns carries L1 to L7 as the schematic's section 4 states: a
    non-persistent store, page JavaScript off, the compiled rule list, no message handler, the
    navigation gate, the window refusal, and the card handed over as a string with no base URL.
R3. `NavigationGate` allows exactly one navigation action, the first main-frame action of the load
    the factory starts, and cancels every later action of every type, in the main frame or a
    subframe. It has two states, `awaitingFirstLoad` and `sealed`, and no transition back.
R4. The rule list is one rule: `url-filter` `.*`, no `resource-type` condition (so every type), no
    domain condition, action `block`. The factory compiles it before it builds the view; a failed
    compilation returns a refusal and no view (fail closed).
R5. The planted suite plants one card for every channel in the schematic's iOS table, on one
    iPhone and one iPad simulator. Each reaches its probe from a reference view with every layer
    off, unless the suite's declared UNOBSERVABLE table names it, and that table equals the measured
    set; and each reaches nothing from the factory's view.
R6. Removing one layer, every other layer on, opens exactly the channels the schematic gives that
    layer alone; the declared set of layers with no channel of their own equals the measured set.
R7. With page JavaScript on (the measurement variant, every other layer on), the `webrtc` card
    reaches the UDP listener, and the value of `typeof window.webkit` that card script sees is
    recorded. These are measurements ADR-360 records, not behaviour that ships.
R8. The card renders: the factory view's body text equals the reference view's, its `data:` image
    has a natural width above zero, and its snapshot equals the reference view's and differs from a
    blank view's.
R9. Only the test host `CardProbeHost` declares local networking; the harness app's Info.plist
    gains no App Transport Security key.
R10. CI runs the `CardProbe` scheme's tests on both simulators, and the `CardIsolation` package's
     `swift test` and its Swift mutant sweep on the macOS host.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the navigation gate allows only the first main-frame load and seals | `swift test` `NavigationGateTests/test_only_the_first_main_frame_load_is_allowed` |
| A2 | the rule list blocks every load of every type and has no exception | `swift test` `RuleListTests/test_the_rule_list_blocks_every_load_of_every_type` |
| A3 | the factory's view carries every layer the configuration API can show | `xcodebuild test` `CardProbeTests/FactoryTests/test_the_factory_sets_every_observable_layer` |
| A4 | one file builds the card web view, it carries every layer, and only the probe host declares local networking | `scripts/tests/test_card_web_view_layers.py` |
| A5 | a planted card reaches its probe from the reference view and nothing from the card view | `xcodebuild test` `CardProbeTests/PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view` |
| A6 | removing one layer opens exactly the channels the schematic gives that layer alone | `.../test_removing_one_layer_opens_exactly_its_own_channels` |
| A7 | with page JavaScript on, a peer connection reaches the UDP listener | `.../test_with_page_javascript_on_a_peer_connection_reaches_the_udp_listener` |
| A8 | the card renders in the card view as in the reference view | `.../test_the_card_renders_in_the_card_view_as_in_the_reference_view` |
| A9 | CI runs the probe suite on both simulators and the package's tests and sweep on the host | `scripts/tests/test_ci_workflows.py` `test_the_card_probe_suite_runs_on_both_simulators` |

```acceptance
A1: swift test --package-path ios/CardIsolation --filter CardIsolationTests.NavigationGateTests/test_only_the_first_main_frame_load_is_allowed
A2: swift test --package-path ios/CardIsolation --filter CardIsolationTests.RuleListTests/test_the_rule_list_blocks_every_load_of_every_type
A3: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/FactoryTests/test_the_factory_sets_every_observable_layer
A4: python3 -m unittest discover -s scripts/tests -p test_card_web_view_layers.py -k test_one_file_builds_the_card_web_view_and_it_carries_every_layer
A5: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view
A6: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_removing_one_layer_opens_exactly_its_own_channels
A7: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_with_page_javascript_on_a_peer_connection_reaches_the_udp_listener
A8: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_the_card_renders_in_the_card_view_as_in_the_reference_view
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_card_probe_suite_runs_on_both_simulators
```

**The findings, closed (iPhone and iPad):**

| finding | closed by |
|---|---|
| SEC01-F14 (peer connections from card script) | R2's L2; A5's `webrtc` pair (no peer connection from the card view), A6's L2 variant (page JavaScript off alone holds it) and A7 (the measured reach) |
| SEC01-F15 (the card frame's other channels) | R1 to R4; A1, A2, A4, and A5's every other pair, each channel named in the schematic with the layer that closes it; A6 names which of ADR-335's three layers holds a channel alone and which are depth |
| SEC01-F13 (the card frame's navigation) | a web finding; its iOS counterpart, the card navigating itself, is A5's `nav-self` and `nav-data` pairs and A6's L5 variant |

## 4. File manifest

| file | context | change |
|---|---|---|
| `ios/CardIsolation/Package.swift` | `ios-harness` | added: iOS and macOS platforms, so `swift test` runs on the host |
| `ios/CardIsolation/Sources/CardIsolation/NavigationGate.swift` | `ios-harness` | added |
| `ios/CardIsolation/Sources/CardIsolation/RuleList.swift` | `ios-harness` | added: the rule-list source and its compile step |
| `ios/CardIsolation/Sources/CardIsolation/WindowRefusal.swift` | `ios-harness` | added |
| `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | `ios-harness` | added: `makeCardWebView(html:)`, and an internal `make(layers:)` the probe reaches with `@testable import` |
| `ios/CardIsolation/Tests/CardIsolationTests/NavigationGateTests.swift` | `ios-harness` | added (A1) |
| `ios/CardIsolation/Tests/CardIsolationTests/RuleListTests.swift` | `ios-harness` | added (A2) |
| `ios/CardIsolation/swift-mutants.json` | mutation | added: `SW34900` to `SW34999` entries |
| `ios/Harness/Sources/CardWebView.swift` | `ios-harness` | changed: builds through the factory |
| `ios/CardProbeHost/CardProbeHostApp.swift` | `ios-harness` | added: an empty test host |
| `ios/CardProbeHost/Info.plist` | `ios-harness` | added: declares local networking, for the probe only |
| `ios/CardProbeTests/FactoryTests.swift` | `ios-harness` | added (A3) |
| `ios/CardProbeTests/PlantedCardTests.swift` | `ios-harness` | added (A5 to A8) |
| `ios/CardProbeTests/Planted.swift` | `ios-harness` | added: the planted-card table, the layer map, UNOBSERVABLE |
| `ios/CardProbeTests/Listeners.swift` | `ios-harness` | added: the TCP and UDP listeners |
| `ios/project.yml` | `ios-harness` | changed: the `CardIsolation` package, `CardProbeHost`, `CardProbeTests`, the `CardProbe` scheme |
| `scripts/tests/test_card_web_view_layers.py` | CI | added (A4) |
| `.github/workflows/xcframework.yml` | CI | changed: the harness job runs `CardProbe`; the host job runs the package's tests and sweep |
| `scripts/tests/test_ci_workflows.py` | CI | changed: A9 |
| `scripts/mutation-rows.d/S34900-S34999.json` | mutation | added: rows on `CardWebViewFactory.swift` killed by A4 |
| `docs/schematics/card-frame-channels.md` | docs | changed: section 6 appended, section 4's measured values |
| `docs/decisions/ADR-360-card-view-on-iphone-and-ipad.md` | docs | added |
| `docs/specs/SPEC-349-card-view-on-iphone-and-ipad.md` | docs | added: this SPEC |
| `docs/red-first/SPEC-349.md` | docs | added |
| `changelog.d/card-view-ios-349.md` | docs | added |

## 5. What this does NOT do

- It does not put the card view on the native review screen or lay it out on iPad: #632 owns the
  review screen.
- It does not serve card media. Media arrive as `data:` URLs inside the document, so the rule list
  keeps no exception and no scheme handler is registered (#632).
- It does not run card scripts (ADR-352 D1); a scripted card renders without its script until an
  owner decision on scripts is recorded (#651).
- It does not prove the card view on a device: the suite runs on simulators, and the owner's
  device session is the campaign's (#616).
- It does not write the campaign's threat model (SEC01-F16): #653 owns it.

## 6. Risks

- **The rule list does not cover preconnect.** A content rule list governs loads; a preconnect is a
  connection. The `preconnect` pair measures it; if the card view connects, the iOS ADR records the
  residual, and a follow-up strips `link` elements from the card before it reaches the view
  (#664).
- **A followed link opens one connection that no layer holds.** When the card follows a link
  (`nav-self`, `nav-blank`), the card view opens one connection to the link's host with no request
  read, and no layer of this delivery holds it. A5 counts the connections from the card view for
  every planted card: those two may read at most one each, and every other card none. #677 tracks
  the cure.
- **`WKWebView` in a unit-test bundle loads nothing until its host app's run loop turns.** The probe
  tests run hosted by `CardProbeHost` and await each load's navigation delegate callback with a
  bounded expectation, never a sleep.
- **A simulator's ICE agent skips the loopback interface,** so the `webrtc` reference reads blind.
  The reference pair detects it; A7 refuses an UNOBSERVABLE `webrtc` on both simulators.
- **The `@testable import` path that builds reference views ships.** `make(layers:)` is
  `internal`; A4 refuses any caller outside test targets, and the Release build of the harness does
  not enable testability.
- **The harness's files move under this delivery.** Every path above was re-read at the cut sha
  611d7427a5ad1f7d090c03cb7329ee8b5f438f28: the destinations pin the simulator runtime as the
  `harness` job does (`SIM_OS`), `xcframework.yml` is a called workflow whose every job runs on the
  one admitted macOS runner, and the caller's `ios/**` path filter reaches `ios/CardIsolation/`.
  A move after the cut reads red in A4 or A9 by name.

## 7. Amendment for #841 and #796: each card load wait starts when the view's page first answers

Three `apple / harness` attempts, on two pull requests that change no card view path, failed in
one simulator session and passed the same tests in the other. This section adds R11, which ADR-360
D9 decides and A10 and A11 (section 9) hold. Sections 1 to 6, R1 to R10 and A1 to A9 stand as
written.

What failed, read from each job's steps and from its log cut to the failing step's window:

- **A5's test read one card not loaded in the card view** (`PlantedCardTests.swift:639`),
  `object`, in the first session, `iPhone 17`, and none in the second, `iPad (A16)` (job
  125417948615).
- **A5's test read two cards not loaded, and the scripted view's test one** (`:639`, `:862`):
  `stylesheet`, `prefetch` and `form`, in the first session only (job 125379986494).
- **SPEC-339's A6 read the card's web view empty when its wait ended**
  (`HarnessFlowTests.swift:38`), in the first session only (job 125392932893).

Why the planted suite read these cards not loaded:

- **The load wait counts the start of the view's own page process.** `Probe.loadTime(_:)` polls
  from the moment the card is handed to the view, and every card view, with its non-persistent
  store, gets its own WebContent process, started at its first load (ADR-372 D11). These tests
  hold one card view at a time.
- **Each miss followed a process start longer than the whole wait.** The log names a WebContent
  start of 21.8 seconds ending 3 seconds before the `object` miss (job 125417948615), and starts
  of 11.8, 25.1 and 19.6 seconds, each ending 3 to 4 seconds before its miss (job 125379986494),
  for `stylesheet`, `prefetch` and `form`.
- **Starts that long came in the first session only.** Job 125417948615 had 11 starts over 10
  seconds in its first session and none in its second; job 125379986494 had 8 and none.
  The first sessions' starts averaged 2.06 and 2.66 seconds (jobs 125417948615, 125379986494),
  the second sessions' 1.29 and 1.33 seconds (jobs 125417948615, 125379986494).
- **No warm-up before the first test removes them.** At the first session's start the warm-up
  loaded in 3.37 and 5.58 seconds (job 125417948615), and the long starts came all through that
  session (job 125417948615).
- **A held page does not remove them.** Of the first session's starts that followed a GPU process
  exit, 8 of 350 ran over 10 seconds, and of the others 3 of 97 (job 125417948615).

What this changes:

- **R11. Each card load wait starts when the view's page first answers (#841, #796).**
  `Probe.loadTime(_:)` first asks the view's page a script of the app's own,
  `Probe.answerScript`, asking again whenever an ask comes back without its answer, one ask at a
  time, under `Probe.warmUpSeconds`, the 60 seconds the suite already gives a start of WebKit
  (ADR-372 D11). Only once the page has answered does it poll whether the card has loaded, under
  `Probe.loadSeconds`, 10 seconds, unchanged. A page that never answers reads not loaded after the
  whole answer bound; a card that never loads reads not loaded after its whole 10-second wait,
  counted from the answer. Each load prints `card probe answer: took=` with its page's answer
  time. `Probe.warmUp()`, `Probe.keeper(ruleList:)`, every planted card, the layer map,
  `UNOBSERVABLE`, `DEPTH`, every window and every assertion are unchanged; no test is skipped,
  retried or marked, and the harness job's bound does not move.
- **The wait's seam.** `Probe.loadTime(asking:answerSeconds:loadSeconds:)` takes the page as a
  function that asks it a script and hands back its string answer, so a test can hand it a
  stand-in page; `Probe.loadTime(_:)` hands it the view's own `evaluateJavaScript`. A10 and A11
  run the wait over `LatePage`, a stand-in page whose start and load each take a set time.
- **Risks of the #841 amendment.** A page whose process takes longer than 10 seconds to start and
  then loads within its wait now reads loaded where it read not loaded before: the wait judges the
  card's load, and the start is printed and bounded by its own 60 seconds. A process that never
  starts now holds its test for 60 seconds, not 10, before it fails. The first session's long
  starts are measured, not cured, and their cause is not in the job log (#841). SPEC-339's A6 is a
  UI test whose app console the job does not keep, so this amendment takes no measured cause for
  its red and changes neither its wait nor its test (#796).
- **The #841 amendment touches these paths:** `ios/CardProbeTests/PlantedCardTests.swift`,
  `docs/specs/SPEC-349-card-view-on-iphone-and-ipad.md`,
  `docs/decisions/ADR-360-card-view-on-iphone-and-ipad.md`, `docs/red-first/SPEC-349.md` and
  `changelog.d/card-probe-load-wait.md`.

## 8. What this does NOT do, in the #841 amendment

- It raises no wait, bound or timeout: `Probe.loadSeconds` stays 10 seconds, `Probe.warmUpSeconds`
  60, and the harness job's bound stays as it is (#841).
- It retries, skips or removes no test, planted card or assertion, and adds no filter (#841).
- It does not cure the first session's long process starts; it stops counting them as a card's
  load (#841).
- It changes neither SPEC-339's flow tests nor their wait: A6's red has no measured cause in
  the job log (#796).
- It does not name the simulator in a planted-suite failure, which #841 also asks for (#841).
- It changes no workflow step (#841).

## 9. Acceptance criteria of the #841 amendment

| id | criterion | decided by |
|---|---|---|
| A10 | a page whose process first answers after more than the load wait, then finishes loading within it, reads loaded within that wait, counted from its first answer | `xcodebuild test` `CardProbeTests/PlantedCardTests/test_the_load_wait_starts_when_the_page_first_answers` |
| A11 | a page whose process never answers within the answer bound reads not loaded, and only after that whole bound | `.../test_a_page_that_never_answers_reads_not_loaded_after_the_answer_bound` |

```acceptance
A10: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_the_load_wait_starts_when_the_page_first_answers
A11: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_page_that_never_answers_reads_not_loaded_after_the_answer_bound
```

A10's and A11's tests are committed with the wait's seam, the wait itself unchanged, and read
red in the pull request's own run, by name, on both simulators, before the wait changes;
`docs/red-first/SPEC-349.md` quotes each red and names the green.

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

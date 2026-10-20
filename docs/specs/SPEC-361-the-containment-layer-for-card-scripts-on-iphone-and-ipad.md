# SPEC-361: card scripts on iPhone and iPad are held by refused navigations and content-blocking rules, and a planted card proves every channel closed before the switch turns on

- **Wave:** the app campaign, Phase 1 parity for scripted cards. **Issue:** #651, its iPhone and
  iPad half; it holds #677's channel in both card views. **Context(s):** `ios-harness` (`ios/`).
- **Decided by:** ADR-372 (this SPEC's own: the containment layer, its parts, where each
  lives, the proof and the order of the build). It builds on
  ADR-360 (L1 to L7) and ADR-366 (the switch, its read-back, L8), and amends neither in place.
- **Schematic:** `docs/schematics/card-frame-channels.md`, amended INSERT-ONLY by an appended
  section that names every channel, the part of the layer that closes it, and the planted card
  that proves it.
- **Status:** the card-scripts switch defaults OFF on iPhone and iPad, pending a containment layer
  measured to hold. This SPEC is that layer. `CardScripts.switchedOn` reads `false` at every
  commit of this delivery; the switch turns on only in a separate commit, ruled after a
  verification by hand measures the layer, so it turns on only once that measurement holds.

## 1. The problem, measured

Card scripts on iPhone and iPad run only in the card view `CardWebViewFactory` builds, and only
when `CardScripts.switchedOn` is on and every required control is read back from the built view
(SPEC-355 R1, ADR-366 D1). The switch defaults off on iOS pending a measured containment layer
(ADR-372 D1): it stays off until a layer built from refused navigations and content-blocking
rules is measured to hold every channel below.

Each row is a channel a card can open, the platform mechanism that closes it, and the primary
source for that mechanism. The measurement this SPEC takes is its acceptance (section 3); no
row below is a measurement.

| channel | closed by | why that mechanism, and its source |
|---|---|---|
| subresource loads: image, style sheet, font, media, script, `fetch`, XHR, `EventSource`, beacon, ping, prefetch, preload, a speculation-rules prefetch (fired on a link's `mousedown`, `pointerdown` or `keydown`, before any click), a worker's fetch | L3, the compiled rule list (one rule blocking every URL), and L12, the document's own policy | Apple's content-blocker format names each as a resource type (`image`, `style-sheet`, `font`, `media`, `script`, `raw`, `fetch`, `ping`, `other`); WebKit enforces the document's Content Security Policy in a separate check of every fetch, and sends a speculation-rules prefetch through the same resource loader with that check on (`DocumentPrefetcher`) |
| WebSocket | L3 and L12 | the content-blocker format names `websocket`; the policy's `default-src 'none'` governs `connect-src` |
| the hints `link rel=preconnect` and `rel=dns-prefetch`, static or added by script | L3 | WebKit asks the rule lists about each hint as a `ping` (`LinkLoader::preconnectIfNeeded`, `FrameLoader::prefetchDNSIfNeeded`); the policy is not consulted for either |
| a main-frame navigation: `location`, `meta` refresh, a form, a script's `form.submit()` | L5, the navigation gate; L3 refuses the document load; L12's `form-action 'none'` refuses a form | `WKNavigationDelegate.decidePolicyFor` is asked before the engine loads any content |
| a new window: `target=_blank`, `window.open` | L5 and L6 (`createWebViewWith` answers nil) | `WKUIDelegate` creates a window only when the delegate returns a view |
| a nested frame, `object`, `embed` | L5 and L3; L12's `default-src 'none'` | a subframe's load is a navigation action and a `child-document` load |
| **a followed link's early connection** (#677): a tap, a script's `click()`, a dispatched click or an Enter key on a link with an `http` or `https` target in the main frame, or a `_blank` target in any frame | **L10 and L11** (no rule list and no delegate reaches it) | WebKit's `HTMLAnchorElement::handleClick` asks the navigation delegate and then opens a connection to the link's host (`preconnectTo`), gated by no rule list, no policy and no delegate; the only per-view engine switch for it is a private interface. The connection is never opened when the click's default action is cancelled before `handleClick` runs |
| the same, from a link outside the document: a detached `a` clicked or sent a click | L11 | a detached element's click reaches no window, so no listener in any frame sees it; `handleClick` still runs for an `a` that is not connected |
| the same, after `document.open()` erased every listener in a document | L11 | `Document::open` removes every event listener of the document, its window and its nodes, in every content world |
| a peer connection or a datagram transport, in any frame | L8 (unchanged) | not a load: the content-blocker format has no type for it |
| a long-press preview or a context-menu "Open" of a link | L13 and L6's context-menu arm | `allowsLinkPreview` defaults to true; a preview or an app-initiated open is outside the card's document |
| the app's state: stores, files, the message bridge | L1, L4, L7 (unchanged) | ADR-360 |

**The card's own permitted loads are exactly two:** the card's document, handed to the view as a
string with no base URL (L7), which is no load; and `data:` URLs, which WebKit's rule-list
backend answers with no action and the policy admits only as `img-src`, `media-src` and
`font-src`. Media reach the card as `data:` URLs (ADR-359 D1); there is no URL scheme handler
(SPEC-348 R18). Every other load is refused by L3 and, except the two hints above, by L12 too.

## 2. Requirements

R1. **One switch, off at every commit of this delivery.** `CardScripts.switchedOn` stays the only
    switch and stays defined in `ios/CardIsolation/Sources/CardIsolation/CardScripts.swift` alone
    (SPEC-355 A5's rule). It reads `false` at every commit of this delivery. It turns on only in a
    separate commit that changes that one line to `true`, ruled after a verification by hand
    measures the layer: the switch turns on only once that measurement holds.

R2. **The required controls.** `CardScripts.required` is exactly L1, L3, L4, L5, L6, L7, L8, L10,
    L11, L12 and L13. L9 is retired: the app builds no proxy configuration, owns no loopback
    listener for the card view, and `CardLayer` has no `L9` case. Its number is not reused.

R3. **L10, the link-activation refusal.** One `WKUserScript`, at document start, in every frame,
    in a content world the app owns (`WKContentWorld.world(name:)`, never `.page`), registers one
    listener on the frame's window for `click` and `auxclick` in the capture phase. It cancels
    the event's default action when the event's composed path holds a link (an `a` or `area`
    element with an `href` attribute, or an SVG `a` element), or when the event's target is an
    element that can host a shadow root (a custom element, or `article`, `aside`, `blockquote`,
    `body`, `div`, `footer`, `h1` to `h6`, `header`, `main`, `nav`, `p`, `section`, `span`), so a
    link inside a closed shadow root is refused through its host. It never stops propagation, so
    every listener a card registers still runs. Its source lives in
    `LinkActivationRefusal.swift`, which also builds the user script.

R4. **L11, the page guard.** One `WKUserScript`, at document start, in every frame, in the page's
    own content world, installed beside L8. It replaces `HTMLElement.prototype.click` and
    `EventTarget.prototype.dispatchEvent` with wrappers that throw a `NotAllowedError` when the
    target is a node that is not connected, and replaces `Document.prototype.open`, `write` and
    `writeln` with wrappers that always throw a `NotAllowedError`. Each wrapper is installed
    non-writable and non-configurable, and calls only intrinsics captured when it was installed.
    A connected element's `click()` and `dispatchEvent` pass through unchanged, so L10 is what
    refuses them. Its source lives in `PageGuard.swift`, which also builds the user script.

R5. **L12, the document policy.** The factory hands the view `DocumentPolicy.prefix` followed by
    the card's markup, where the prefix is exactly:

```html
<!doctype html><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; media-src data:; font-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; object-src 'none'; form-action 'none'; base-uri 'none'">
```

    A policy a card adds can only narrow it. `DocumentPolicy.weaker(_:)` names every way a
    candidate policy is weaker than this one, as `RuleList.weaker(_:)` does for the rule list.
    The prefix lives in `DocumentPolicy.swift`.

R6. **L13, no link preview, and no menu that opens a link.** The factory sets the view's
    `allowsLinkPreview` to `false`. `WindowRefusal` (L6) gains a context-menu arm: it answers the
    context-menu request for an element with no configuration that previews or opens the link.

R7. **The rule list, unchanged and first.** L3's source stays one rule blocking every URL. It
    compiles in `makeCardWebView(html:)` before any view exists; a list that does not compile
    builds no view at all (SPEC-349 R4, unchanged), so no card script can run without it. No
    second rule list is added.

R8. **Read back, never assumed.** `present(in:built:requestURL:)` reads L10 and L11 from the
    controller's user scripts (the source equal to the layer's own, injected at document start,
    not main-frame-only), L12 from the string the factory handed the view (it begins with
    `DocumentPolicy.prefix`), and L13 from the view (`allowsLinkPreview` is false and the UI
    delegate is the `WindowRefusal` the factory set). A layer the read-back cannot find is
    missing, and the verdict is `.off` naming it.

R9. **Every view gets every layer.** The scripts-off view and the scripted view are one
    configuration; the verdict sets L2 alone. So L10 to L13 hold #677's channel in the scripts-off
    view as well.

R10. **The harness proves it, and is weakened nowhere.** `ios/CardProbeTests/Planted.swift` and
    `PlantedCardTests.swift` gain the cards and the variant below. No listener, variant, card,
    observable, control, declared set or count is removed or loosened, and no tolerance is
    added. The one exception is L9's own: the per-view hold and its count leave with L9 (R2).
    - New scripted cards, each holding its marker: `script-click-self` (a connected link's
      `click()`), `script-click-blank` (the same with `target=_blank`), `script-click-detached`
      (a detached `_blank` link's `click()`), `script-dispatch-detached` (a detached link sent a
      `MouseEvent` click), `script-enter-key` (an Enter `keydown` dispatched on a focused
      connected link), `script-closed-shadow` (a `_blank` link in a closed shadow root, clicked
      from inside it), `script-written-link` (a frame's document opened, a `_blank` link written
      into it and clicked), and `script-frame-link` (a `_blank` link in an appended blank frame,
      clicked). Each is observed by `connection`.
    - A new variant, `shippedWithout(CardLayer)`: the scripts-off view with one layer removed.
    - `CONTROLS` becomes L1, L3, L4, L5, L6, L7, L8, L10, L11, L12, L13, and `DEPTH_SCRIPTED`
      and every card's held set are declared by prediction in the schematic's appended section.
      A measurement that contradicts a declaration is a STOP for the seat, never a silent edit.
    - A new card, `permitted`, carries a `data:` image, a `data:` font and a `data:` audio clip.

R11. **Every enumerating test** prints `examined N <what>` and fails on 0.

R12. **The tree guard follows the layer.** `scripts/tests/test_card_web_view_layers.py` drops L9's
    tokens and names L10 to L13's, each read from the file that holds it (R3 to R6). Its switch
    rule (R1's one file) is unchanged.

R13. **Mutation rows** hold each part: `SW36100` onward in `ios/CardIsolation/swift-mutants.json`,
    killed by the `card-isolation` job's `swift test`, and `S36100` onward in
    `scripts/mutation-rows.d/S36100-S36199.json`, killed by R12's test. The finds of
    SPEC-355's rows `S35500` to `S35502` (`forMainFrameOnly: false`, `.atDocumentStart`,
    `in: .page`) each still occur exactly once in `CardWebViewFactory.swift`: L10 and L11 build
    their user scripts in their own files for that reason.

R14. No property list, project manifest or entitlement changes, and no workflow change but R15's.
    The `harness` job runs the whole `CardProbe` scheme, so new tests run where SPEC-355's do.

R15. **The `harness` job's bound holds the planted suite.** The card view's planted suite ran
    about fifty minutes on a hosted macOS runner, and the whole job projects to 85 to 98 minutes,
    so the job's 90-minute bound is cancelled by this delivery's own cards. The `harness` job's
    `timeout-minutes` is 150, inside a band of 150 to 180 (at most half the hosted job's limit),
    held by `HARNESS_TIMEOUT_MINUTES` in `scripts/tests/test_ci_workflows.py` beside the engine
    jobs' bands. No other job, step or line of any workflow changes.

R16. **The load wait holds for a cold first test (#681).** The planted suite's measured load wait,
    `Probe.loadSeconds`, stays 10 seconds, and its assertion stays `loaded` true, with no
    tolerance, skip or retry. `Probe.warmUp()` loads both paths once per test process, before
    any measured wait: one planted card in a scripts-off view the factory builds, then one card
    whose script runs in a scripts-on view, each mounted, under its own bound,
    `Probe.warmUpSeconds`, 60 seconds. Every test class in `ios/CardProbeTests` that loads a card
    asserts it from `setUp`. After that warm-up, A5's test still read its first row at 4.23
    seconds on the iPhone simulator and 10.15 seconds on the iPad simulator, while its other 12
    rows read 0.01 seconds or less, because it built and started all 13 views, each with its own
    WebContent process, before it mounted the first. So A5's test builds each row's view and
    starts its load just before its mount, and tears it down before the next row's view is
    built, so every row's wait measures its own load. A card that never loads still reads red
    within the wait: `loaded` reads false, and only after the whole bound. A load that ends
    during the poll's final check, after the bound, reads loaded with a `took` past
    `Probe.loadSeconds`, and the land reading names any such reading. Each load's time is recorded:
    `Probe.loadTime(_:)` returns the seconds a load took, or nil when the wait expired, `loaded`
    is read through it, and each warm-up load and each row of A5's test print it into the
    `harness` job's result bundles. A5's test also holds one keeper card across its rows:
    `Probe.keeper(ruleList:)` loads and mounts one planted card in a scripts-off view the factory
    builds, under `Probe.warmUpSeconds`, and the test calls it before the first row's view is
    built, holds it across every row and removes it after the last, so no row's wait pays the
    relaunch of WebKit's GPU and networking processes that the previous row's teardown caused
    (ADR-372 D11). The keeper is not a row and is not timed as one, it changes no row's wait,
    reading or assertion, and its own load is asserted; it prints
    `card probe keeper: loaded= took=`. The owner's signed ruling,
    `docs/rulings/OWNER-RULING-2026-10-07-factory-row-load-wait.md`, is the authority for the
    factory rows' land gate, that every factory row loads within its own `Probe.loadSeconds` wait
    by that wait, on both destinations, on two `apple / harness` runs of the same head, and for
    the keeper.

## 3. Acceptance criteria

`$IPHONE_SIM`, `$IPAD_SIM` and `$SIM_OS` are the `harness` job's own. A simulator criterion is
green only when its test passed on both destinations in one CI run.

| id | criterion | decided by | red first by |
|---|---|---|---|
| A1 | scripts run only when the switch is on and every required control (R2) is present, over every subset of them | `swift test` `CardScriptsTests/test_scripts_run_only_when_switched_on_with_every_control_present` | a subset lacking L10 reads `.run` against the old `required` |
| A2 | L10 cancels the default of every click and auxclick whose path holds a link or whose target can host a shadow root, registers in the capture phase, never stops propagation, and leaves every other click alone | `swift test` `LinkActivationRefusalTests/test_the_refusal_cancels_every_link_activation_and_nothing_else` | the stub's source is empty: no listener is registered |
| A3 | L11 refuses a detached node's `click()` and `dispatchEvent`, refuses every `open`, `write` and `writeln`, passes a connected node's calls through, and a card cannot redefine any wrapper | `swift test` `PageGuardTests/test_the_guard_refuses_detached_activation_and_every_document_rewrite` | the stub's source is empty: a detached click reaches the planted handler |
| A4 | the document policy is exactly R5's, `weaker` names each weakening of it, and the prefix puts the policy first in `head` | `swift test` `DocumentPolicyTests/test_the_policy_admits_only_data_media_and_inline_style_and_script` | the stub's prefix is empty and `weaker` names nothing |
| A5 | the card view carries every required control, and scripts run only with all of them: each removed in turn, the switch on, reads `.off` naming it and the card's script does not run | `xcodebuild test` `FactoryTests/test_the_card_view_runs_scripts_only_with_every_control` | the rows for L10 to L13 read `.run` before the factory builds them |
| A6 | one file holds the switch, and every control's token is set in the file that holds it | `scripts/tests/test_card_web_view_layers.py` `test_the_script_switch_lives_in_one_file_and_every_control_is_set` | L10 to L13's tokens are absent |
| A7 | every planted card opens zero connections, paths and datagrams from the scripts-off view; the planted control: `nav-self` and `nav-blank` each open a connection from `shippedWithout(.L10)` | `xcodebuild test` `PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view` | `nav-self` and `nav-blank` open a connection from the shipped view before L10 exists |
| A8 | every planted card, its planted click included, opens zero connections, paths and datagrams from the scripted view; the planted control: `nav-self` and `nav-blank` each open a connection from `scriptedWithout(.L10)` | `xcodebuild test` `PlantedCardTests/test_a_planted_card_reaches_nothing_from_the_scripted_view` | the same two cards open a connection from the scripted view before L10 exists |
| A9 | every scripted card reaches from its reference and nothing from the scripted view, its marker set; its blind set equals `BLIND_SCRIPTED` | `xcodebuild test` `PlantedCardTests/test_a_scripted_card_reaches_nothing_from_the_scripted_view` | the eight new cards reach from the scripted view before L10 and L11 exist |
| A10 | no peer connection leaves the scripted view from any frame, and the reference is not blind | `xcodebuild test` `PlantedCardTests/test_no_peer_connection_leaves_the_scripted_view_from_any_frame` | not red: SPEC-355's criterion, kept; it guards L8 through the change |
| A11 | a scripted card reads no state the app holds; the reference reads the planted state | `xcodebuild test` `PlantedCardTests/test_a_scripted_card_reads_no_state_the_app_holds` | not red: SPEC-355's criterion, kept |
| A12 | removing one control from the scripted view opens exactly its own channels, as declared; every control in `DEPTH_SCRIPTED` opens nothing alone | `xcodebuild test` `PlantedCardTests/test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels` | `CONTROLS` names L10 to L13, which the factory does not build |
| A13 | a lookup a card asks for reaches the witness from the reference and never from either shipped view, L9 retired | `xcodebuild test` `PlantedCardTests/test_a_lookup_a_card_asks_for_reaches_the_witness_only_from_the_reference` | not red: SPEC-355's criterion, kept; with L9 gone it is L3's proof |
| A14 | a scripted card renders as in the reference and its script runs, under L12 | `xcodebuild test` `PlantedCardTests/test_a_scripted_card_renders_and_its_script_runs` | not red: SPEC-355's criterion, kept |
| A15 | the card's permitted loads load: the `permitted` card's `data:` image decodes, its `data:` font loads and its `data:` audio reads its metadata, in the shipped and the scripted view | `xcodebuild test` `PlantedCardTests/test_the_permitted_loads_load_in_both_card_views` | the `permitted` card is absent from the stub's `Planted.swift` |
| A16 | the `harness` job's `timeout-minutes` is a digit string inside 150 to 180 (R15) | `scripts/tests/test_ci_workflows.py` `TheHarnessLinksItsOwnRunsFramework/test_the_harness_job_timeout_holds_the_planted_suite` | the job's bound is 90 |
| A17 | the load wait holds for a cold first test: the once-per-process warm-up loads its two cards, one on each path, each within its own bound, asserted from `setUp` before every test of `FactoryTests` and `PlantedCardTests`, the measured waits unchanged; and a card that never loads reads not loaded only after the whole wait (R16) | `xcodebuild test` `FactoryTests/test_the_card_view_runs_scripts_only_with_every_control` (the suite's first test, which runs the warm-up's `setUp` assertion first) and `FactoryTests/test_the_load_wait_reads_a_card_that_never_loads_as_not_loaded` | before any warm-up, the suite's first test read its first row not loaded at the 10-second wait on the first simulator |

```acceptance
A1: swift test --package-path ios/CardIsolation --filter CardIsolationTests.CardScriptsTests/test_scripts_run_only_when_switched_on_with_every_control_present
A2: swift test --package-path ios/CardIsolation --filter CardIsolationTests.LinkActivationRefusalTests/test_the_refusal_cancels_every_link_activation_and_nothing_else
A3: swift test --package-path ios/CardIsolation --filter CardIsolationTests.PageGuardTests/test_the_guard_refuses_detached_activation_and_every_document_rewrite
A4: swift test --package-path ios/CardIsolation --filter CardIsolationTests.DocumentPolicyTests/test_the_policy_admits_only_data_media_and_inline_style_and_script
A5: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/FactoryTests/test_the_card_view_runs_scripts_only_with_every_control
A6: python3 -m unittest discover -s scripts/tests -p test_card_web_view_layers.py -k test_the_script_switch_lives_in_one_file_and_every_control_is_set
A7: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view
A8: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_planted_card_reaches_nothing_from_the_scripted_view
A9: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_card_reaches_nothing_from_the_scripted_view
A10: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_no_peer_connection_leaves_the_scripted_view_from_any_frame
A11: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_card_reads_no_state_the_app_holds
A12: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels
A13: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_lookup_a_card_asks_for_reaches_the_witness_only_from_the_reference
A14: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_card_renders_and_its_script_runs
A15: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_the_permitted_loads_load_in_both_card_views
A16: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_harness_job_timeout_holds_the_planted_suite
A17: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/FactoryTests/test_the_card_view_runs_scripts_only_with_every_control -only-testing:CardProbeTests/FactoryTests/test_the_load_wait_reads_a_card_that_never_loads_as_not_loaded
```

Every enumerating test prints `examined N <what>` and fails on 0 (A1 every case of the switch
and every subset of the required controls; A2 and A3 every planted event and element; A4 every
directive; A5 every required control removed in turn; A7 to A15 every planted card and
simulator).

**The planted click is the app's own script activating the element, the stand-in for a tap
(ADR-360 D5).** The harness sees every channel a card's own script can open, on both
simulators. A person's tap and long press enter the engine through the UI process instead, so
section 3a names their proof on a device, held.

**The findings, carried:**

| finding | held by |
|---|---|
| SEC01-F14 (peer connections from card script) | L8, unchanged; A10, A12's L8 variant |
| SEC01-F15 (the card frame's other channels) | L1, L3 to L7 unchanged, L10 to L13; A7, A8, A9, A12, A15 |
| #677 (a followed link opens one connection) | L10 and L11, in both views; A7, A8, A9, A12's L10 and L11 variants |

## 3a. Proved on a device

| id | criterion | who proves it, and when |
|---|---|---|
| D1 | on one iPhone and one iPad, a person's tap on a planted self link and on a `_blank` link, and a long press on each, open no connection to a listener on the device's network, and no menu item opens either link | held: not measured by this delivery, and not a condition of its merge |

## 4. File manifest

| file | context | change |
|---|---|---|
| `ios/CardIsolation/Sources/CardIsolation/CardScripts.swift` | `ios-harness` | changed: `required` per R2; the switch per R1 (`false` at every commit of this delivery, on only in a separate commit once that measurement holds) |
| `ios/CardIsolation/Sources/CardIsolation/LinkActivationRefusal.swift` | `ios-harness` | added: L10's source, its content world and its user script |
| `ios/CardIsolation/Sources/CardIsolation/PageGuard.swift` | `ios-harness` | added: L11's source and its user script |
| `ios/CardIsolation/Sources/CardIsolation/DocumentPolicy.swift` | `ios-harness` | added: L12's prefix and `weaker(_:)` |
| `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | `ios-harness` | changed: `CardLayer` loses L9 and gains L10 to L13; L10 to L13 installed; L12's prefix handed; `present` reads them back; the proxy configuration removed |
| `ios/CardIsolation/Sources/CardIsolation/WindowRefusal.swift` | `ios-harness` | changed: the context-menu arm (R6) |
| `ios/CardIsolation/Sources/CardIsolation/ConnectionHold.swift` | `ios-harness` | removed with L9 (R2), when the base still holds it |
| `ios/CardIsolation/Tests/CardIsolationTests/CardScriptsTests.swift` | `ios-harness` | changed: A1's required set |
| `ios/CardIsolation/Tests/CardIsolationTests/LinkActivationRefusalTests.swift` | `ios-harness` | added (A2) |
| `ios/CardIsolation/Tests/CardIsolationTests/PageGuardTests.swift` | `ios-harness` | added (A3) |
| `ios/CardIsolation/Tests/CardIsolationTests/DocumentPolicyTests.swift` | `ios-harness` | added (A4) |
| `ios/CardIsolation/Tests/CardIsolationTests/ConnectionHoldTests.swift` | `ios-harness` | removed with L9, when the base still holds it |
| `ios/CardIsolation/swift-mutants.json` | mutation | changed: `SW36100` onward appended; L9's entries leave with L9 |
| `ios/CardProbeTests/FactoryTests.swift` | `ios-harness` | changed: A5 over the new `CONTROLS`; the hold removed; R16's warm-up asserted from `setUp`, each row's load time printed, and A17's test of a card that never loads; the keeper held across the rows (D11) |
| `ios/CardProbeTests/PlantedCardTests.swift` | `ios-harness` | changed: A8 and A15 added; A7's control moves to `shippedWithout(.L10)`; the variant; the hold removed; R16's `Probe.loadSeconds`, `Probe.loadTime(_:)` and `Probe.warmUp()`, the warm-up asserted from `setUp`; the keeper added (D11) |
| `ios/CardProbeTests/Planted.swift` | `ios-harness` | changed: R10's cards, `CONTROLS`, `DEPTH_SCRIPTED` and held sets |
| `scripts/tests/test_card_web_view_layers.py` | CI | changed: R12's tokens |
| `.github/workflows/xcframework.yml` | CI | changed: the `harness` job's bound, 90 -> 150 minutes (R15) |
| `scripts/tests/test_ci_workflows.py` | CI | changed: the band and its test (A16) |
| `scripts/mutation-rows.d/S36100-S36199.json` | mutation | added: rows killed by A6 |
| `docs/schematics/card-frame-channels.md` | docs | changed: one section appended, insert-only |
| `docs/decisions/ADR-372-the-containment-layer-for-card-scripts-on-iphone-and-ipad.md` | docs | added |
| `docs/specs/SPEC-361-the-containment-layer-for-card-scripts-on-iphone-and-ipad.md` | docs | added: this SPEC |
| `docs/red-first/SPEC-361.md` | docs | added |
| `changelog.d/card-scripts-ios-361.md` | docs | added |
| `ios/swift-roles.json` | `ios-harness` | changed: the three new sources and three new tests registered; L9's two files leave with L9 |
| `scripts/mutation-rows.d/S35500-S35599.json` | mutation | changed: L9's rows S35503 and S35504 leave with L9 |
| `docs/specs/SPEC-355-card-scripts-on-iphone-and-ipad.md` and `docs/red-first/SPEC-355.md` | docs | changed: A3 retired insert-only (SPEC-056 R14); SPEC-355 section 8 appended |
| `docs/specs/SPEC-056-every-pack-is-judged-on-the-box-and-nothing-of-the-hub-is-published.md` | docs | changed: section 7's row for SPEC-355's A3 and section 11, insert-only |
| `changelog.d/card-scripts-ios-355.md` | docs | changed: L9's clause and the #677 sentence leave |

One workflow change is ruled, the `harness` job's bound in `xcframework.yml` (R15); no other
workflow file, no property list, no project manifest and no other file changes. If the cut
shows another is needed after all, it is a STOP for the seat, never a silent addition (it raises
#602's wall).

## 5. What this does NOT do

- It does not run card scripts on the web: Part W, under its own SPEC, carries #651's web half.
- It does not let a card script reach the network or rewrite its document: a card that needs a
  host library, `eval`, a peer connection or `document.write` still does not work (#651).
- It does not strip `link`, `meta` or `base` from a card on iOS; the layers above hold what they
  would open (#664).
- It does not put the card view on the review screen or lay it out on iPad (#632).
- It does not run the card view's proof on a device in CI; the suite runs on simulators, and D1
  is a person's reading (#616).
- It does not write the campaign's threat model (SEC01-F16, #653).

## 6. Risks

- **L10 does not run in the scripts-off view.** If the engine runs no app-world user script while
  page JavaScript is off, A7 reads `nav-self` and `nav-blank` open from the shipped view: red,
  the switch stays off, and the seat rules.
- **L10's host rule blocks a card's own control.** A tap on a `span` inside a `summary` or a
  `label` no longer toggles it. A14 proves the render card's own button still works; a card that
  relies on such a default is a product question for the seat, never a narrowed rule.
- **A link activation the stand-in does not reach.** A person's tap travels through the UI
  process. A8 cannot see that path; D1, held, is the reading that does.
- **A declared set is wrong.** Every held set and `DEPTH_SCRIPTED` are predictions. A
  contradiction is a STOP with both readings, never an edit that makes the suite pass.
- **L12 changes a card's rendering.** The prefix's own doctype puts every card in standards mode
  and the face document already begins with one. A14 and SPEC-349's render criterion compare
  against the reference; a difference is a STOP.
- **The scripts-off view regresses.** SPEC-349's single-layer variants keep running on its seven-
  layer base, so a change to a layer reads red there first.

## 7. Amendment: SPEC-382 admits its timing lines to the workflow

Made by SPEC-382's delivery, insert-only: every earlier byte is kept in order. It inserts this
section.

SPEC-382 R9 amends the freeze words of two requirements of section 2, and keeps every other word:

- R14's "no workflow change but R15's" admits SPEC-382 R1 to R6's lines too: the timer lines
  around the planted suite's unchanged command, the build timing summary on every other
  `xcodebuild` build, test or archive, the per-target static-library seconds, the framework job's
  developer directory, the `harness` step that boots the iPhone simulator, and each job's timing
  record and its upload.
- R15's "No other job, step or line of any workflow changes" admits exactly those lines, and no
  other.
- R15's bound is kept: `harness` keeps its `timeout-minutes`, inside the band
  `HARNESS_TIMEOUT_MINUTES` holds.
- R16's load wait is kept as written: no wait, tolerance, skip or retry changes, and the planted
  suite's command stays byte-identical, held by SPEC-382 A1.

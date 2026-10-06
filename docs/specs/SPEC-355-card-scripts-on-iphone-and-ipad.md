# SPEC-355: card scripts run on iPhone and iPad only behind one switch and every control, and a planted card proves each control holds

- **Wave:** the app campaign, Phase 1 parity for scripted cards. **Issue:** #651, its iPhone and
  iPad half; it closes #677. **Context(s):** `ios-harness` (`ios/`).
- **Decided by:** ADR-366 (this SPEC's own: the switch, the peer-connection removal, the
  connection hold, the proof). It amends ADR-352 D1 for iPhone and iPad only, as ADR-352's own
  "What would make this wrong" foresaw, and extends ADR-360's seven layers to nine.
- **Schematic:** `docs/schematics/card-frame-channels.md`, amended INSERT-ONLY by an appended
  section that names every script-driven channel and the control that holds it.
- **Status:** this pull request delivers R1 to R13, with its tests and
  `docs/red-first/SPEC-355.md`.

## 1. The problem, measured

- Card scripts are off: `CardWebViewFactory.swift` line 83 sets
  `configuration.defaultWebpagePreferences.allowsContentJavaScript = false` (L2), read with
  `git grep -n -i 'javaScript' 05aef78615980c9fddcf1e0d7e3b353de1942ba4 -- ios/CardIsolation/Sources`.
  A scripted card (a hint toggle, a typed-answer helper) renders as markup only (ADR-352 D1).
- The schematic's measured iOS section (`docs/schematics/card-frame-channels.md` lines 225-264 at
  that sha) records that L2 ALONE holds four channels: `script-inline`, `event-handler`,
  `javascript-url` and `webrtc`. With page JavaScript on and every other layer on, the `webrtc`
  card reached the UDP listener with one datagram on both simulators, and `typeof window.webkit`
  read `undefined`. Turning L2 off as it stands therefore opens a peer connection from every
  shared deck to any host (SEC01-F14).
- No public web view setting refuses a peer connection. The CSP `webrtc` directive is not
  implemented by the engine. Lockdown Mode turns peer connections off, but an app that is not a
  browser cannot set it per web view. A content rule list governs loads, and a peer connection is
  not a load.
- #677: a followed link (`nav-self`, `nav-blank`) opens one connection to the link's host before
  the gate refuses it. SPEC-349's A5 tolerates at most one connection for each of those two cards
  (`ios/CardProbeTests/PlantedCardTests.swift` lines 396-407 at that sha). With scripts off only a
  learner's tap starts it, once per tap, to a host the deck's static markup names. A card script
  can start it itself, in a loop, to hosts it names at run time, so the connection and the name
  it looks up become a channel for whatever the script can read. #677 is closed by this delivery,
  before the switch turns on.
- The suite declares `dns-prefetch` UNOBSERVABLE (`ios/CardProbeTests/Planted.swift` lines 233-234
  at that sha): its card names an address literal, so no lookup reaches a listener the test owns.
  A card script can add a `link rel=dns-prefetch` at run time, naming a host whose labels carry
  data. An unobserved channel cannot stay unobserved once script writes into it.
- The package's deployment target already includes the data store's proxy configuration
  (`ios/CardIsolation/Package.swift` line 9 at that sha), so the connection hold needs no
  availability branch.

## 2. Requirements

R1. One switch. `CardScripts.switchedOn`, in `ios/CardIsolation/Sources/CardIsolation/
    CardScripts.swift`, is the only definition of the switch under `ios/`. A pure
    `CardScripts.decide(switchedOn:present:)` returns `.run` exactly when the switch is on and
    `present` holds every control in `CardScripts.required`, which is L1, L3, L4, L5, L6, L7, L8
    and L9; otherwise `.off` with the missing controls named.
R2. The factory decides from what it BUILT. It reads `present` back from the configuration and
    view it made (the store is not persistent, the rule list is installed, no handler is named,
    each delegate is set, the user script is installed at document start in every frame in the
    page world, the proxy configuration names the ready hold with failover off and no exclusion),
    then sets L2 from the verdict. Any control missing yields today's scripts-off view, still
    carrying every control it has. A rule list that does not compile still refuses the view.
R3. L8, the peer-connection removal: one `WKUserScript`, injected at document start, in every
    frame, in the page's content world, deletes every global whose name matches `^(webkit)?RTC`
    and `WebTransport` before any card script runs. It adds no handler and names no host.
R4. L9, the connection hold (#677): the card view's data store carries exactly one proxy
    configuration, an HTTP CONNECT proxy at a loopback listener the app owns (`ConnectionHold`),
    with failover off and no excluded domain. The hold answers no byte and closes every
    connection it receives, and counts it. A hold that is not ready leaves L9 absent (R1).
R5. L6 gains three refusal arms: a script dialog is answered with no UI (an alert completes, a
    confirm answers false, a prompt answers nothing); a media capture request is denied; a device
    orientation and motion request is denied.
R6. The second origin. A scripted card's document has an opaque origin (L7: a string with no base
    URL), in a non-persistent store distinct from the default store (L1), with no bridge (L4). A
    planted card that tries to read the default store's cookie and storage for a planted origin, a
    file the app wrote in its container, `window.webkit`, and a stored credential reads none,
    while a reference view built on the default store at that origin reads the cookie and the
    storage.
R7. The planted suite gains one scripted card for every script-driven channel the schematic's
    amendment names. Each reaches its observable from its reference (scripts on, its control off)
    and nothing from the scripted view, on one iPhone and one iPad simulator. A card's own marker
    proves its script ran in the scripted view, so an absence is never read off a script that did
    not run.
R8. Removing one control from the scripted view, every other control on, opens exactly the
    channels the amendment gives that control alone; the declared set of controls with no channel
    of their own equals the measured set.
R9. No reference is blind. Every reference reading a planted card relies on is greater than zero
    on both simulators, or the test fails naming the card. The `webrtc` reference (STUN to the
    UDP listener) and its TCP twin (TURN over TCP to the TCP listener) both carry this rule, so the
    simulator's ICE agent skipping the loopback interface reads red, never green.
R10. A lookup witness. The `dns-prefetch` card (static and scripted) names a `.local` host whose
     first label is the card's id; a multicast DNS listener the test owns counts queries naming it.
     The reference reaches it on both simulators, and the scripted and scripts-off views do not.
R11. The scripted card renders and runs: a benign scripted card (a hint toggle) sets its marker in
     the scripted view and its body text equals the reference view's.
R12. SPEC-349's A5 to A8 still hold against the scripts-off view, unchanged except that A5's
     `#677` allowance is removed: every card opens zero connections, and the hold counts the
     refused attempt of `nav-self` and `nav-blank`. SPEC-349's A6 keeps its seven-layer base by
     name (L1 to L7, with L8 and L9 off in every one of its variants), so its declarations stand
     as measured; A10 measures the scripted set, where L9 is expected to share every load L3
     held alone.
R13. CI needs no new job: the `harness` job's planted step already runs the `CardProbe` scheme
     whole, and the `card-isolation` job runs the package's `swift test` and its mutant sweep.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | scripts run only when the switch is on and every control is present, over all 512 cases | `swift test` `CardScriptsTests/test_scripts_run_only_when_switched_on_with_every_control_present` |
| A2 | the removal deletes every peer-connection name and nothing else | `swift test` `PeerConnectionRemovalTests/test_the_removal_deletes_every_peer_connection_name_and_nothing_else` |
| A3 | the hold answers no byte, closes and counts every connection | `swift test` `ConnectionHoldTests/test_the_hold_answers_no_byte_and_counts_every_connection` |
| A4 | the card view carries every control, and scripts run only with all of them | `xcodebuild test` `CardProbeTests/FactoryTests/test_the_card_view_runs_scripts_only_with_every_control` |
| A5 | one file holds the switch, and every control's token is set in the factory | `scripts/tests/test_card_web_view_layers.py` `test_the_script_switch_lives_in_one_file_and_every_control_is_set` |
| A6 | every card opens zero connections from the scripts-off view; the hold counts the refused link (#677) | `xcodebuild test` `.../PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view` |
| A7 | every scripted card reaches from its reference and nothing from the scripted view, its marker set | `.../test_a_scripted_card_reaches_nothing_from_the_scripted_view` |
| A8 | no peer connection leaves the scripted view from any frame, and the reference is not blind | `.../test_no_peer_connection_leaves_the_scripted_view_from_any_frame` |
| A9 | a scripted card reads no state the app holds; the reference reads the planted state | `.../test_a_scripted_card_reads_no_state_the_app_holds` |
| A10 | removing one control opens exactly its own channels | `.../test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels` |
| A11 | a lookup a card asks for reaches the witness from the reference and never from either shipped view | `.../test_a_lookup_a_card_asks_for_reaches_the_witness_only_from_the_reference` |
| A12 | a scripted card renders as in the reference and its script runs | `.../test_a_scripted_card_renders_and_its_script_runs` |

```acceptance
A1: swift test --package-path ios/CardIsolation --filter CardIsolationTests.CardScriptsTests/test_scripts_run_only_when_switched_on_with_every_control_present
A2: swift test --package-path ios/CardIsolation --filter CardIsolationTests.PeerConnectionRemovalTests/test_the_removal_deletes_every_peer_connection_name_and_nothing_else
A3: swift test --package-path ios/CardIsolation --filter CardIsolationTests.ConnectionHoldTests/test_the_hold_answers_no_byte_and_counts_every_connection
A4: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/FactoryTests/test_the_card_view_runs_scripts_only_with_every_control
A5: python3 -m unittest discover -s scripts/tests -p test_card_web_view_layers.py -k test_the_script_switch_lives_in_one_file_and_every_control_is_set
A6: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view
A7: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_card_reaches_nothing_from_the_scripted_view
A8: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_no_peer_connection_leaves_the_scripted_view_from_any_frame
A9: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_card_reads_no_state_the_app_holds
A10: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels
A11: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_lookup_a_card_asks_for_reaches_the_witness_only_from_the_reference
A12: xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:CardProbeTests/PlantedCardTests/test_a_scripted_card_renders_and_its_script_runs
```

Every enumerating test prints `examined N <what>` and fails on 0 (A1 examines 512 cases: the
switch on or off times every subset of the eight required controls; A2 every planted global
name; A4 every required control removed in turn; A6 to A12 every planted card and simulator).

**The findings, carried:**

| finding | held by |
|---|---|
| SEC01-F14 (peer connections from card script) | L8 (R3); A2, A8 (every frame, both simulators, a reference that is not blind), A10's L8 variant |
| SEC01-F15 (the card frame's other channels) | L1, L3 to L7 unchanged, L9 (R4), L6's arms (R5); A6, A7, A10 |
| #677 (a followed link opens one connection) | L9 (R4); A3, A6 (zero for every card), A10's L9 variant |

## 4. File manifest

| file | context | change |
|---|---|---|
| `ios/CardIsolation/Sources/CardIsolation/CardScripts.swift` | `ios-harness` | added: the switch, `required`, `decide(switchedOn:present:)`, the verdict |
| `ios/CardIsolation/Sources/CardIsolation/PeerConnectionRemoval.swift` | `ios-harness` | added: L8's source and its user script |
| `ios/CardIsolation/Sources/CardIsolation/ConnectionHold.swift` | `ios-harness` | added: L9's loopback listener and its proxy configuration |
| `ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift` | `ios-harness` | changed: `CardLayer` gains L8 and L9; L8 and L9 installed; `present` read back; L2 from the verdict |
| `ios/CardIsolation/Sources/CardIsolation/WindowRefusal.swift` | `ios-harness` | changed: R5's three arms |
| `ios/CardIsolation/Tests/CardIsolationTests/CardScriptsTests.swift` | `ios-harness` | added (A1) |
| `ios/CardIsolation/Tests/CardIsolationTests/PeerConnectionRemovalTests.swift` | `ios-harness` | added (A2) |
| `ios/CardIsolation/Tests/CardIsolationTests/ConnectionHoldTests.swift` | `ios-harness` | added (A3) |
| `ios/CardIsolation/swift-mutants.json` | mutation | changed: `SW35500` to `SW35599` entries appended |
| `ios/CardProbeTests/FactoryTests.swift` | `ios-harness` | changed: A4 added |
| `ios/CardProbeTests/PlantedCardTests.swift` | `ios-harness` | changed: A5's residual removed (A6); A7 to A12 added; a scripted variant |
| `ios/CardProbeTests/Planted.swift` | `ios-harness` | changed: the scripted cards, the `.local` names, the scripted declarations |
| `ios/CardProbeTests/Listeners.swift` | `ios-harness` | changed: the multicast DNS witness |
| `scripts/tests/test_card_web_view_layers.py` | CI | changed: A5 added; L2's token follows the verdict; L8 and L9 tokens |
| `scripts/mutation-rows.d/S35500-S35599.json` | mutation | added: rows on `CardWebViewFactory.swift` killed by A5 |
| `docs/schematics/card-frame-channels.md` | docs | changed: one section appended, insert-only |
| `docs/decisions/ADR-366-card-scripts-on-iphone-and-ipad.md` | docs | added |
| `docs/specs/SPEC-355-card-scripts-on-iphone-and-ipad.md` | docs | added: this SPEC |
| `docs/red-first/SPEC-355.md` | docs | added |
| `changelog.d/card-scripts-ios-355.md` | docs | added |

No workflow file, no property list and no other file changes. If the cut shows a workflow edit is
needed after all, it is a STOP for the seat, never a silent addition (it raises #602's wall).

## 5. What this does NOT do

- It does not run card scripts on the web: Part W, under its own SPEC, carries #651's web half.
- It does not let a card script reach the network: a script that needs a library from a host
  still loads nothing (L3), so such a card renders without it (#651).
- It does not strip `link`, `meta` or `base` from a card on iOS; the controls above hold what they
  would open (#664).
- It does not put the card view on the review screen or lay it out on iPad (#632).
- It does not prove the card view on a device; the suite runs on simulators (#616).
- It does not write the campaign's threat model (SEC01-F16, #653).

## 6. Risks

- **L8 misses a nested realm.** A frame the card makes has its own globals. If the engine does not
  inject a document-start user script into a frame's first, empty document, a card reaches a
  peer connection through it. A8's nested-frame cards (an appended blank frame, a frame given
  `srcdoc`, a frame written by `document.write`) measure it against a reference that is not blind;
  a datagram from the scripted view is red, the switch does not ship on, and the seat rules.
- **The engine bypasses the proxy for loopback.** The probe's listeners sit on the loopback
  address. If WebKit connects to them directly with the hold configured, A6 reads a connection and
  the hold's count reads zero, naming the bypass. The builder re-measures once with the listeners
  on the simulator host's non-loopback address; a second bypass is a STOP.
- **The proxy hop needs a transport exception in the shipped app.** If App Transport Security
  refuses the web view's hop to the loopback hold, A4's read-back shows L9 absent and scripts stay
  off. Adding any key to the harness app's property list is a QUESTION, never a fix (A4 of
  SPEC-349 refuses it).
- **The witness is blind on the runner.** If no multicast DNS query reaches the witness from the
  reference on either simulator, A11 fails and `dns-prefetch` cannot be declared UNOBSERVABLE
  again with scripts on: the switch does not ship on, and the seat rules.
- **Scripted cards break.** A card whose script needs a host library or a peer connection still
  does not work; the render card (A12) proves an ordinary script does.
- **The scripts-off view regresses.** R12 keeps SPEC-349's suite running against it, so a change
  to a layer reads red there first.

## 7. Amendment: the switch defaults off on iOS

The switch defaults off on iOS pending a measured containment layer. Every line above in which the
switch turns on, or card scripts run, on iPhone and iPad reads with this default.

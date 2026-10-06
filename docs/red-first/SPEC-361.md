# Red-first record: SPEC-361

Every criterion's test is committed before the code it judges, against stubs that compile. A6 runs
locally and in CI's `hygiene` job; A1 to A4 run on the macOS host, in the `card-isolation` job's
`swift test`; A5 and A7 to A15 run only on the simulators, in the `harness` job's `CardProbe` step,
which tests the iPhone simulator and then the iPad simulator in one invocation. A simulator
criterion is red or green only when it is so on both destinations of one run.

```red-first
A1: red at 7238808e: red not measured (one push); predicted CardScriptsTests.swift:35: XCTAssertEqual failed - a switch-on subset lacking any of L10 to L13 makes `decide` read `.run` against the stub's `required` (L1, L3 to L8), where `.off` naming the missing controls is wanted; and CardScriptsTests.swift:40: the required controls are not L1, L3 to L8 and L10 to L13
A2: red at 7238808e: red not measured (one push); predicted LinkActivationRefusalTests.swift:126 and :127: XCTAssertEqual failed - the stub's source is empty, so no listener is registered and `cancelled` is empty for click and auxclick against the eight links and hosts; and :137: the registrations are [], not one capture-phase listener for click and one for auxclick
A3: red at 7238808e: red not measured (one push); predicted PageGuardTests.swift:185: XCTAssertEqual failed - the stub's source is empty, so the guard installs nothing: `a detached click: threw nothing, reached ["click detached"]; wanted NotAllowedError, []` is the first of the planted calls it lets through
A4: red at 7238808e: red not measured (one push); predicted DocumentPolicyTests.swift:60: XCTAssertEqual failed - the stub's prefix is empty, so each of the nine directives reads nil; :62 to :64: the prefix carries no policy and does not open with one; and :104: `weaker` names nothing for each of the nine planted weaker candidates
A5: red at fc0a34dd: red not measured (one push); predicted FactoryTests.swift:110 and :111: XCTAssertEqual failed - with the stub's `required` (L1, L3 to L8) and none of L10 to L13 built, the rows without L10, L11, L12 and L13 read `.run` with the switch on and the card's script runs, where `.off` naming the control is wanted; the L6 and L7 rows read `.off` without the control each also takes (L13 and L12, named in `alsoTakes` with their reasons)
A6: red at c222171c: AssertionError: Lists differ: 7 problems != [] at test_card_web_view_layers.py:413 `self.assertEqual(problems, [])` (local, scripts/tests): LinkActivationRefusal.swift and PageGuard.swift missing; CardWebViewFactory.swift lacks L10 (addUserScript(LinkActivationRefusal.userScript)), L11 (addUserScript(PageGuard.userScript)), L12 (let html = DocumentPolicy.prefixed(html)) and L13 (allowsLinkPreview = false); WindowRefusal.swift lacks L13 (contextMenuConfigurationForElement)
A7: red at fc0a34dd: red not measured (one push); predicted PlantedCardTests.swift:494: XCTAssertEqual failed - `nav-self` and `nav-blank` open a connection from the scripts-off card view with every built layer, before L10 exists
A8: red at fc0a34dd: red not measured (one push); predicted PlantedCardTests.swift:663: XCTAssertEqual failed - the same two cards open a connection from the scripted view with every built layer, before L10 exists
A9: red at fc0a34dd: red not measured (one push); predicted PlantedCardTests.swift:710 and :711: XCTAssertEqual failed - the eight new link-activation cards reach their probe or a listener from the scripted view, before L10 and L11 exist
A10: not red: SPEC-355's criterion, kept; it guards L8 through the change
A11: not red: SPEC-355's criterion, kept
A12: red at fc0a34dd: red not measured (one push); predicted PlantedCardTests.swift:854: XCTAssertEqual failed - `CONTROLS` names L10 to L13, which the factory does not build, so the channels each control opens when removed alone are not the declared ones
A13: not red: SPEC-355's criterion, kept; with L9 gone it is L3's proof
A14: not red: SPEC-355's criterion, kept
A15: not red: over the factory's stubs (fc0a34dd) no L12 is built until the layer's commit (a550da38), so nothing blocks the `permitted` card's data: image, font and audio, and the criterion, which reads those loads accepted in both card views, holds over the stub
```

A9 and A12 at 8e4a808b read one probe-validity red, the same on both simulators of one `harness`
job (112308297225), with nothing reached from the scripted view: `script-written-link`'s reference
without L11 opened no connection with its marker set, so A9's blind set held it beside the pinned
`BLIND_SCRIPTED` (`PlantedCardTests.swift:715`) and L11's single-control map lost it
(`PlantedCardTests.swift:854`). WebKit's parser names a second control. With L11 removed the
frame's document is opened and written, and the rewrite erases L10's listener, but WebKit runs
every document-start user script again when `write` parses the opened document's new root element,
so L10's refusal (`LinkActivationRefusal.swift`) holds the written link's click. Two controls, L10
and L11, hold `script-written-link`: its held set is L10 and L11, its reference removes both, and
no control alone opens it (`Planted.swift`). `BLIND_SCRIPTED`, the other cards' held sets and the
other channels of each control's map are unchanged.

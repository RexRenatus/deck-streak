# Red-first record: SPEC-349

Every criterion's test was committed before the code it judges, against stubs that compile. A4
and A9 run locally and in CI's `hygiene` job; A1 and A2 run on the macOS host, in the
`card-isolation` job's `swift test`; A3 and A5 to A8 run only on the simulators, so each of their
reds and greens is read from the pull request's own `apple-on-change` run, from the `harness` job's
`CardProbe` step, which tests `iPhone 17` and then `iPad (A16)` in one invocation. A simulator
criterion is recorded green only when its test passed on both.

A4 and A9 were committed alone (ba29482e), red in run 37259607130's `hygiene` job: exactly 2 of
952 python tests failed, these two. A9's second half, the host job, came with the package
(4cd5dbef), so run 37263022889 read A9 red on the CardProbe step alone; the probe suite and its
step came next (8bb083db), and run 37266682781 read A9 green and A4 the one failure of 952.

A1 and A2 were committed over the package's stubs (4cd5dbef): the gate allowed every action and
the rule list was `[]`. Run 37263023091's `card-isolation` job read both red by assertion, A1 with
44 failures over its 25 gate rows.

A3 and A5 to A8 were committed over the factory stub (8bb083db), a default view with no layer.
Run 37266682885's `harness` job read A3, A5 and A6 red on both simulators, each by the same
assertion on both. A7 and A8 passed against the stub, and are recorded `not red`:

- A7: not red, a measurement of the engine. With page JavaScript on, the `webrtc` card sent 1
  datagram to the UDP listener on both simulators, and `typeof window.webkit` read `undefined`
  with no handler; the stub's view had JavaScript on, so the reading did not depend on the code
  under test.
- A8: not red. The stub's view rendered the card as the reference view did, which is what A8
  asks of the factory's view; it guards the layers against breaking the render.

The same run measured the declared UNOBSERVABLE set wrong: the reference view reached nothing
for `nav-data` as well as `prefetch` and `dns-prefetch`, on both simulators. Its reading was
`allowed=1` with no marker and no change of text: the reference delegate allowed the action and
WebKit refused the page's own main-frame navigation to a `data:` URL. The implementation commit
(b797da55) added `nav-data` to UNOBSERVABLE from that measurement, and disclosed it.

The factory, the gate, the rule list, the window refusal and the harness view came next
(b797da55), with the mutants and rows (92877de4), pushed together. The criteria's tests and their
assertions were unchanged apart from that UNOBSERVABLE entry; `RuleListTests.swift` gained the
host test of the factory's refusal, `test_a_rule_list_that_did_not_compile_builds_no_view`, and
imports the package `@testable` to reach it. That test is mutation coverage for `SW34905`, not a
criterion, so it stands outside the fence.

Run 37270629304 read the implementation. The `card-isolation` job read A1 and A2 green, and its
sweep killed all six Swift mutants. The `harness` job read A3, A5, A7 and A8 green on both
simulators, and A6 red on both by its channel map alone (`PlantedCardTests.swift:363`). DEPTH
(line 365) read as declared, {L1, L4, L6, L7}. The one entry that differed was `object`. It was
declared as held by L3 alone, but no single-layer variant opened it. The reference view's
delegate was asked about two navigations for that card (`allowed=2`): an `<object>` and an
`<embed>` of type `text/html` each load as a subframe, which the gate (L5) cancels as it does
`nested-frame`'s. So L3 and L5 both hold it, and neither holds it alone. Every card's outcome
(from the reference view, the card view and each of the seven single-layer variants) was
identical on the two simulators, and the card view reached nothing from any card on either.

That second red of A6 was not planned. It is disclosed here as the `nav-data` correction was, and
corrected from the measurement alone: f764bc05 changes the `object` card's declared layer from L3
to none (`nil`, the spelling `nested-frame` uses) and nothing else. No assertion, tolerance or
other declaration changed, and A6 still requires each layer's measured channels to equal its
declared ones, so the corrected row goes red again if L5 stops holding `object`; A5 is unchanged.
The schematic records the measured iOS table in its own appended section 6 (f7810e0e), beside
section 4's prediction. The commits after f764bc05 change only documents, so A6's green line
names f764bc05, and the `apple-on-change` run at this delivery's head is the run that reads it, on
both simulators.

```red-first
A1: red at 4cd5dbef: NavigationGateTests.swift:67: XCTAssertEqual failed: ("awaitingFirstLoad") is not equal to ("sealed") - the first main-frame load (run 37263023091, job card-isolation, host; Executed 1 test, with 44 failures)
A1: green at 92877de4
A2: red at 4cd5dbef: RuleListTests.swift:21: XCTAssertEqual failed: ("0") is not equal to ("1") - the rule list holds 0 rules, not one (run 37263023091, job card-isolation, host; Executed 1 test, with 2 failures)
A2: green at 92877de4
A3: red at 8bb083db: FactoryTests.swift:33: XCTAssertEqual failed: ("ObservableLayers(persistentStore: true, pageJavaScript: true, navigationDelegate: "none", uiDelegate: "none", url: "about:blank")") is not equal to ("ObservableLayers(persistentStore: false, pageJavaScript: false, navigationDelegate: "GateAdapter", uiDelegate: "WindowRefusal", url: "about:blank")") - A3: the factory's view, layer by layer (run 37266682885, job harness, iPhone 17 and iPad (A16))
A3: green at 92877de4
A4: red at ba29482e: AssertionError: Lists differ: ['ios/Harness/Sources/CardWebView.swift'] != ['ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift'] (run 37259607130, job hygiene)
A4: green at 92877de4
A5: red at 8bb083db: PlantedCardTests.swift:331: XCTAssertEqual failed: (20 cards) is not equal to ("[]") - cards that reached their probe from the card view; PlantedCardTests.swift:333: ("["prefetch", "dns-prefetch", "nav-data"]") is not equal to ("["prefetch", "dns-prefetch"]") - the cards whose reference view reached nothing, against the declared UNOBSERVABLE (run 37266682885, job harness, iPhone 17 and iPad (A16))
A5: green at 92877de4
A6: red at 8bb083db: PlantedCardTests.swift:365: XCTAssertEqual failed: ("[]") is not equal to ("[L1, L4, L6, L7]") - the layers whose removal alone opened nothing, against DEPTH (run 37266682885, job harness, iPhone 17 and iPad (A16); line 363's channel map the same); and red again, not planned, at 92877de4: PlantedCardTests.swift:363: XCTAssertEqual failed: (measured) is not equal to (declared) - the channels each layer opened when removed alone, the measured L3 set lacking `object` and every other layer's set equal (run 37270629304, job harness, iPhone 17 and iPad (A16); reference object allowed=2, every without-Lx object read reached=false), corrected from that measurement at f764bc05
A6: green at f764bc05
A7: not red: passed on both simulators at 8bb083db, a measurement of the engine (datagrams=1, typeof window.webkit=undefined)
A8: not red: passed on both simulators at 8bb083db; the stub's view renders the card
A9: red at ba29482e: AssertionError: Lists differ: ['harness: 0 CardProbe test steps, not one', 'card-isolation: no such job'] != [] (run 37259607130, job hygiene)
A9: green at 8bb083db
```

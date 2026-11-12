# Red-first record: SPEC-392

Every criterion's test is committed before the code it judges, against an identity stub of
`LinkStrip.stripped(_:)` wired into the factory's build at `CardWebViewFactory.swift:47`, which is
the "no strip" mutant (ADR-406 D5). The acceptance fence, line by line:

- A1, A2 and A3 run on the macOS host, in the `card-isolation` job's `swift test` of
  `ios/CardIsolation`.
- A4 runs locally and in CI's `hygiene` job, from `scripts/tests`.
- A5, A6, A7 and A8 run only on the simulators, in the `harness` job's `CardProbe` step, which
  tests the iPhone simulator (`$IPHONE_SIM`) and then the iPad simulator (`$IPAD_SIM`) in one
  invocation. A simulator criterion is red or green only when it is so on both destinations of one
  run.

```red-first
A1: red at aea97145: LinkStripTests.swift:82: XCTAssertEqual failed - A1's goldens the link strip got wrong: each of the 13 came back as it went in, keeping its `<link` opener where `<wbr` was wanted (run 37992282561, job apple / card-isolation, step the card view's gate and rule list, on the host)
A1: green at 50ed2922
A2: red at aea97145: LinkStripTests.swift:88: XCTAssertEqual failed - A2's goldens the link strip got wrong: `<link`, `<p>a</p><link` and `<LINK` each came back unchanged where the `<wbr` opener was wanted (run 37992282561, job apple / card-isolation, step the card view's gate and rule list, on the host)
A2: green at 50ed2922
A3: not red: the identity stub passes it; it pins the strip and kills SW39204 (mutation coverage)
A4: red at 0758f585: AssertionError: Lists differ at test_card_web_view_layers.py:577 `self.assertEqual(self.problems(REPO), [], ...)` (local, scripts/tests): the factory hands a card past the link strip; CardLayer does not name L14; the link strip's file is missing; the link strip defines no stripped(_:)
A4: green at aea97145
A5: red at aea97145: PlantedCardTests.swift:691: XCTAssertEqual failed - the cards whose card view holds a link element: dns-prefetch, prefetch, preconnect, shadow-link and stylesheet at 1 and preload at 2, where none was wanted, on the iPhone simulator and on the iPad simulator alike (run 37992282561, job apple / harness, step the card view's planted suite, Debug, on the iPhone and then the iPad)
A5: green at 50ed2922
A6: red at aea97145: PlantedCardTests.swift:723: XCTAssertEqual failed - the linked cards that reached or opened a connection with only L14 on: preconnect, preload, shadow-link and stylesheet, where none was wanted, on the iPhone simulator and on the iPad simulator alike (run 37992282561, job apple / harness, step the card view's planted suite, Debug, on the iPhone and then the iPad)
A6: green at 50ed2922
A7: not red: SPEC-361's criterion, kept; its zero-connection reading on both simulators is #677's closing reading
A8: not red: SPEC-349's criterion, kept; L14 is in no base variant
```

Each destination's reading of A5 to A8 is read from the step's own result bundle, which the job
uploads as `harness-results/card-probe.xcresult`; the job's `report.md` reads no row from it. The
bundle's two actions name their run destinations, the iPhone simulator and then the iPad
simulator, and each holds 18 tests with 2 failed: A5 at `PlantedCardTests.swift:691` and A6 at
`:723`. A7 and A8 read success on both. Each control held on both: the cards whose reference view
holds a link element were exactly `LINKED` (A5), and every reference reached (A6).

A4's green at aea97145 is also CI's: the `hygiene` job's python stage (run 37992282312) ran it
green, with `examined 6 planted trees`.

Between A4's red and its green, commit aea97145 also replaced the words of the doc comment at
`CardScriptsTests.swift:17` in place (R3), so that they name L14; no assertion changed. Between
the red of A1, A2, A5 and A6 and their green, commit 50ed2922 changed `LinkStrip.swift` alone, and
no test.

The script rows S39200 and S39201 were each proved by hand against A4's test at de8d9041, and the
Swift mutants SW39200 to SW39204 are proved by the `card-isolation` job's mutant sweep.

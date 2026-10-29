# Red-first record: SPEC-339

Every criterion's test was committed before the code it judges. A1, A9 and A10 run on the box and
in CI; A2 to A8 run only on the macOS runner, so each of their reds and greens is read from the
pull request's own `xcframework` run, test by test, from the `harness-wire` job's `swift test` log
(A2, A3) and from the `harness` job's Debug result bundle on both simulators, `iPhone 17` and
`iPad (A16)` (A4 to A8). A simulator criterion is recorded green only when its test passed on both.

A1's test was committed alone (991fdf03): the render call is refused by the allow-list, so the
assertion reads who refused it. The sixth pair came next (3950c8ca): the adapter crate's whole
population ran 9 tests and 9 passed.

A9 and A10 were committed alone (f8db32b7), red in the `ci` run's `hygiene` job; the harness tree
and its two jobs came next (0a9ec800), green in the next run's `hygiene` job.

A2 to A8 were committed alone (f1b1c300), over the stubs the harness tree carried: the codec
returned empty bytes and empty decodes, and the app's model and session did nothing. Run
37224198598 read each red by an `XCTAssertEqual` quoting both sides. On the iPhone, which starts
on the deck list, the stub's empty list left the study pane unreached, so A6 and A7 read the web
view and the new count as `(not shown)`; on the iPad, which shows both columns, the same
assertions read empty values. The codec (9bb1d31c) and the harness (97fc16e5) came next, with the
tests unchanged, and run 37227365194, at the head that also carries the Release step's
architecture fix (6e6d3392), passed all seven.

The fixture writer (`crates/ffi/examples/harness-fixture.rs`) is not a criterion: it is test
support, and A4 to A7 open what it writes. The two measurement tests of section 7 are not
criteria either, so they stand outside the fence:

- M1, `test_m1_cold_start`: not red, a measurement. It measures the launch and the harness's own
  interval and asserts nothing.
- M2, `test_m2_memory`: not red, a measurement. It measures peak physical memory and asserts
  nothing.

```red-first
A1: red at 991fdf03: assertion `left == right` failed: left: (Ok([]), "the allow-list", false) right: (Ok([]), "nobody", true)
A1: green at 3950c8ca
A2: red at f1b1c300: RequestBytesTests.swift:20: XCTAssertEqual failed: ("[]") is not equal to ("[10, 200, 1, 120, 120, 120, ...") - OpenCollectionRequest (the path's bytes elided here; lines 31, 34, 38 and 49 the same for the other four requests; run 37224198598, Executed 1 test, with 5 failures)
A2: green at 6e6d3392
A3: red at f1b1c300: ResponseDecodingTests.swift:17: XCTAssertEqual failed: ("[]") is not equal to ("[HarnessWire.DeckName(id: 1, name: "Default"), HarnessWire.DeckName(id: 300, name: "Synthetic")]") - DeckNames (lines 40, 59 and 81 the same for the two queues and the question's nodes; run 37224198598, Executed 1 test, with 4 failures)
A3: green at 6e6d3392
A4: red at f1b1c300: HarnessFlowTests.swift:18: XCTAssertEqual failed: ("") is not equal to ("opened") (run 37224198598, on both simulators)
A4: green at 6e6d3392
A5: red at f1b1c300: HarnessFlowTests.swift:27: XCTAssertEqual failed: ("[]") is not equal to ("["Default", "Synthetic"]") (run 37224198598, on both simulators)
A5: green at 6e6d3392
A6: red at f1b1c300: HarnessFlowTests.swift:38: XCTAssertEqual failed: ("") is not equal to ("synthetic front") on the iPad, ("(not shown)") on the iPhone (run 37224198598)
A6: green at 6e6d3392
A7: red at f1b1c300: HarnessFlowTests.swift:56: XCTAssertEqual failed: ("["", ""]") is not equal to ("["1", "0"]") on the iPad, ("["(not shown)", "(not shown)"]") on the iPhone (run 37224198598)
A7: green at 6e6d3392
A8: red at f1b1c300: CardWebViewTests.swift:19: XCTAssertEqual failed: ("Isolation(persistentDataStore: true, pageJavaScript: true)") is not equal to ("Isolation(persistentDataStore: false, pageJavaScript: false)") (run 37224198598, on both simulators)
A8: green at 6e6d3392
A9: red at f8db32b7: AssertionError: 'harness' not found in ['xcframework']
A9: green at 0a9ec800
A10: red at f8db32b7: AssertionError: False is not true : ios/Harness/Info.plist is not in the tree
A10: green at 0a9ec800
```

At 8bc14293, under the newer simulator runtime, M1 failed on the iPhone simulator in the Release
step, at `HarnessMeasureTests.swift:33: Failed to terminate invalid.deckstreak.harness`: on the
block's fourth run, `launch()` could not end the instance the third run had left running, inside
that run's measured window. 62395e0b ends each pass's app after `stopMeasuring()` instead, so each
measured window holds one launch and nothing to end. Both metrics, the five measured passes and
each pass's wait for the deck list are unchanged, and M1 is still not red, a measurement.

At e6dc596d, under the newer simulator runtime, SPEC-358's A12
(`test_a12_the_review_survives_the_sidebar`) failed on the iPad simulator in run 40439982510, job
121980478793 (`apple / harness`), in the step that runs the bury and flag tests on the iPhone and
then the iPad, at `ReviewActionsFlowTests.swift:82`. The answer side, read after Show Answer and
before the sidebar moved, read `[false, false, true]` against `[true, true, true]`: one check of
that reading took 41 seconds in job 121980478793 and ended past its 30-second bound, while the two
measurements across the sidebar matched; the iPhone passed. The test and the app code it drives are
those of dev. 3ed6ef3f ends each test's app after it asserts, e4a0c4bc keeps the screen and prints
the seconds when a reading runs out, bd04da9b notes in the device's log when a card's page finishes
loading, and f2bca63c has the step let both simulators settle before its tests, sample the CPU
through them and keep each simulator's log of page loads. A12's rows, its assertion, its bound and
both destinations are unchanged, and it stays `not red` in SPEC-358's record. CI reads it by name at
this push, on the iPhone and on the iPad.

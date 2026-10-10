# Red-first record: SPEC-393

The SPEC, its schematic and ADR-407 were committed first (c4371200). The census of the card view
factory's inline playback was committed alone next (401182c0), and every other criterion's test was
then committed before the code that turns it green (d4298b50), over stubs that keep every input but
the missing behaviour: a face carries its template ordinal as 0, a speech clip carries no voices,
the CSS reaches the page as written, and `voice_for` answers the kept choice alone. Each red below
is quoted from the run at its red commit. A14 and A17 run only on the simulators, so their results are
read from the pull request's checks, by run, job, step and destination.

```red-first
A1: red at d4298b50: assertion `left == right` failed: the forward card's question and answer carry 0, the reverse card's carry 1; left: [0, 0, 0, 0] right: [0, 0, 1, 1]
A2: red at d4298b50: assertion `left == right` failed: by day: the body's classes; left: ["card"] right: ["card card2"]
A3: red at d4298b50: assertion `left == right` failed: by day: the body's classes; left: ["card"] right: ["card card1"]
A4: red at d4298b50: assertion `left == right` failed: each url( naming the font holds its data: URL, and the rest is as written; left keeps url("_parity.ttf") and URL( '_parity.ttf' )
A5: red at d4298b50: assertion `left == right` failed: each refused font is emptied, and every other url( stays as written; left keeps url("big.ttf")
A6: not red: pins what the base already does on the touched path: for a reader that does not ask, the CSS is the note type's byte for byte and the reader is asked for the image alone (SPEC-393 section 3)
A7: red at d4298b50: assertion `left == right` failed: the font, one byte past the face's cap, is emptied; left: url("_one.ttf") right: url("")
A8: red at d4298b50: the font is the data: URL of its bytes: the document carries url("_parity.ttf")
A9: red at d4298b50: assertion `left == right` failed: the first tag's clip carries its two voices in order, the second's none; left: voices: [] right: voices: ["Absent_Voice", "Desk_Parity_Voice"]
A10: red at d4298b50: assertion `left == right` failed: the second entry names the installed voice Parity Voice; left: None right: Some("test.voice.parity")
A11: not red: pins the kept choice the base already speaks; it guards the new order, it proves no new behaviour (SPEC-393 section 3)
A12: red at d4298b50: assertion `left == right` failed: a French voice and an absent one are passed over, and the identifier speaks first; left: None right: Some("voice.ava")
A13: red at d4298b50: assertion `left == right` failed: the clip carries the tag's two voices in order; left: voices: [] right: voices: ["Absent_Voice", "Desk_Parity_Voice"]
A15: red at d4298b50: assertion `left == right` failed: both video start tags carry playsinline after the element's name; left: ["<video src=\"data:audio/mp4;base64,cGFyaXR5LXZpZGVv\" controls>", "<VIDEO controls src=...>"]
A16: red at 401182c0: AssertionError: Lists differ: ['ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift: configuration(layers:) does not set allowsInlineMediaPlayback'] != []
A17: red at d4298b50: run 38054884550, job apple / harness, step "the card view's planted suite, Debug, on the iPhone and then the iPad": on the iPhone, FactoryTests.test_the_factory_plays_media_inline failed: XCTAssertTrue failed - A17: the factory's view plays media inline; on the iPad it passed
A18: not red: guards that the Swift wiring adds no decision; it passed at the base (1 test, examined 61 Swift files, 19 decision counts)
A19: red at d4298b50: assertion `left == right` failed: each font is inlined under its own type, and the rest is as written; left keeps url("_a.ttf") ...
A1: green at 530f6478
A2: green at 530f6478
A3: green at 530f6478
A4: green at 530f6478
A5: green at 530f6478
A7: green at 530f6478
A8: green at 530f6478
A9: green at 530f6478
A10: green at 530f6478
A12: green at 530f6478
A13: green at 530f6478
A15: green at 530f6478
A16: green at 530f6478
A19: green at 530f6478
A14: not read red at push 1 (the harness skipped step 15 behind A17's planned red; run 38054884550)
A14: green at cf1acd50: run 38064479715, job apple / harness, step "the review screen's tests, Debug, on the iPhone and then the iPad": ReviewSessionTests.test_the_parity_card_speaks_the_voice_its_template_asks passed on the iPhone and on the iPad
A17: green at cf1acd50: run 38064479715, job apple / harness, step "the card view's planted suite, Debug, on the iPhone and then the iPad": FactoryTests.test_the_factory_plays_media_inline passed on the iPhone and on the iPad
```

A3's test was on the base before this delivery. Its two literals now read `"card card1"` by day and
`"card card1 nightMode night_mode"` by night, where they read `"card"` and
`"card nightMode night_mode"`; both are still asserted by equality, and nothing in it was deleted,
skipped, filtered or narrowed. Its red above is that test reading the base's class.

The whole suites at d4298b50, every binary run: the core's 117 tests, 111 passed and 6 failed (A1,
A4, A5, A7, A9 and A19); the native adapter's 43 tests, 36 passed and 7 failed (A2, A3, A8, A10,
A12, A13 and A15); the web engine's 37 passed. Every other check was green there.

At 530f6478 every local line above passes alone, and the whole suites pass with every binary run:
the core's 117 tests, the native adapter's 43 and the web engine's 37, with both censuses, the
formatter, the lints and the web engine's release build green.

A17's two destinations are read from the step's result bundle, whose first test action names the iPhone
and whose second names the iPad: the factory's test failed in the first and passed in the second. The
step's log runs the same two sessions in that order and names no device beside a test.

A14 has no line here, because its step did not run at d4298b50. In the same job, the planted suite's
red skipped every later step, and the review screen's step runs only when the app's step ran, so it
was skipped too. A14's red at this commit is unproven: it is not recorded as red, and it is not
recorded as not red.

A14 and A17 were read green at cf1acd50, the second head the pull request's checks ran. Both
destinations are read from each step's result bundle, whose first test action names the iPhone and
whose second names the iPad: the review screen's test and the factory's test passed in both. A14's
red stays unproven, as the paragraph above says; its line records that it was not read, not that it
was not red.

Between the greens at 530f6478 and cf1acd50, the branch took the base branch's card view layers in
one merge (f4125e1a), re-derived the three factory lines a schematic cites (7cdcaa50), and added two
unit tests to the native adapter's face module for a mutant the in-diff run missed (cf1acd50). None
of the three changes an assertion of any criterion's test.

# Red-first record: SPEC-348

The SPEC, its schematic, ADR-359 and the changelog fragment were committed first. Each criterion's
test was then committed before the code that turns it green, over a stub that compiles and does
nothing, and each red below is quoted from the run at the red commit. This record covers part a,
A1 to A11; A12 to A23 are the next part's.

```red-first
A1: red at 47ab0eda: the native dispatcher answers DecksService.DeckTree (7,4): NotAllowed { service: 7, method: 4 }
A2: red at 47ab0eda: the adapter answers DecksService.DeckTree (7,4): NotAllowed { service: 7, method: 4 }
A1: green at 46938f8e
A2: green at 46938f8e
A3: red at eb063628: assertion `left == right` failed: the answer's replay: left: [], right: the question's two Sound clips
A4: red at eb063628: assertion `left == right` failed: left: ["dot.png"], right: ["data:image/png;base64,iVBORw0KGgo="]
A5: red at eb063628: assertion `left == right` failed: left: ["sub/dot.png", "..", ...] (the planted path's attribute unchanged)
A6: red at eb063628: assertion `left == right` failed: the speech clips: left: []
A7: red at eb063628: assertion `left == right` failed: the default preset, wished: the replay is full: left: []
A3: green at 68d81d2e
A4: green at 68d81d2e
A5: green at 68d81d2e
A6: green at 68d81d2e
A7: green at 68d81d2e
A11: red at 7b7cd737: the review fixture wrote its collection at <target tmp>/ffi-review-fixture/<run>/collection.anki2
A11: green at 6aa81b35
A8: red at 32de32ac: by day: the image's src is the data: URL of the file, not ""
A8: green at 0f2ad8d4
A9: red at dde76e0e: assertion `left == right` failed: the line before the damaged one is read
A9: green at 7da31e92
A10: red at 50f4a4b5: 1 passed; 5 failed: a_flag_without_a_value_is_refused, a_relative_directory_is_refused, a_missing_directory_is_refused and a_file_is_refused_as_not_a_directory read left: Ok("/default/collection") against their refusal, and an_absolute_existing_directory_is_returned read the default against the directory
A10: green at 4bce975f
A12: red at 2f286125: run 37660220852, job `hygiene`: AssertionError: Lists differ: ["ios/swift-roles.json: the roles: 0 'card' files, not one", "ios/swift-roles.json: the roles: no 'speech' file"] != []: examined 47 files, 99 doors, 9 decisions, 3 admissions
A13: red at 2f286125: run 37660220852, job `hygiene`: AssertionError: Lists differ: ['ios/swift-roles.json: the card role: no file holds it, so none calls makeCardWebView(html:'] != []: examined 19 Swift files
A14: red at 2f286125: run 37660220852, job `hygiene`: AssertionError: Lists differ: ['ios/App/Sources/AnswerBar.swift: the answer bar: absent', 'ios/App/Sources/ReviewView.swift: the answer bar: absent', "ios/App/Sources/AnswerBar.swift: the titles: 'Again' is absent", ...] != []: examined 0 files
A15: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: test_a15_the_loop_reaches_the_designed_end: ReviewSessionTests.swift:27: failed: caught error: "Refusal(sentence: "Anki already open, or media currently syncing.")"
A16: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: test_a16_a_double_tap_answers_once: ReviewSessionTests.swift:27: failed: caught error: "Refusal(sentence: "Anki already open, or media currently syncing.")"
A17: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: test_a17_each_rating_shows_its_interval: ReviewSessionTests.swift:27: failed: caught error: "Refusal(sentence: "Anki already open, or media currently syncing.")"
A18: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: test_a18_the_clip_plan_follows_the_face_and_voiceover: ReviewSessionTests.swift:27: failed: caught error: "Refusal(sentence: "Anki already open, or media currently syncing.")"
A19: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: ReviewFlowTests.swift:37: Failed to get matching snapshot: No matches found for first query match sequence: `Descendants matching type Button` -> `Elements matching predicate '"Again" IN identifiers'`
A20: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: ReviewFlowTests.swift:72: XCTAssertEqual failed: ("[["(not shown)", "(not shown)", "(not shown)", "(not shown)"], ["(not shown)", "(not shown)", "(not shown)", "(not shown)"], ["(not shown)"]]") is not equal to ("[["4 new", "0 learning", "0 to review", "shown"], ["shown", "shown", "shown", "shown"], ["shown"]]") - A20: the deck shows its counts and the text card; Show Answer reveals the four ratings; Good shows the image card
A21: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: ReviewFlowTests.swift:92: XCTAssertEqual failed: ("[0.0, 0.0]") is not equal to ("[64.0, 64.0]") - A21: the image named by its alt text is the fixture PNG's 64 by 64, not a broken image's size
A22: red at 15d28e1a: run 37667605394, job `harness`, on the iPhone and on the iPad: ReviewFlowTests.swift:112: XCTAssertEqual failed: ("[false, false]") is not equal to ("[true, false]") - A22: the picker lists System default, and says so when no voice is installed (installed: true)
A23: red at 15d28e1a: run 37667605394, job `harness-wire`: RequestBytesTests.swift:88: XCTAssertEqual failed: ("[]") is not equal to ("[8, 1]") - DeckId, the default deck; ResponseDecodingTests.swift:124: XCTAssertEqual failed: ("[]") is not equal to ("["<1m", "<6m", "<10m", "4d"]") - StringList; ResponseDecodingTests.swift:151: XCTAssertEqual failed: ("[[], [], [], []]") is not equal to ("[[34], [35], [36], [37]]") - each rating's own state
A12: green at edce06bf
A13: green at edce06bf
A14: green at edce06bf
A15: green at edce06bf
A16: green at edce06bf
A17: green at 4339c673
A18: green at edce06bf
A19: green at 4339c673
A20: green at edce06bf
A21: green at edce06bf
A22: green at edce06bf
A23: green at edce06bf
```

46938f8e, A1's and A2's green, also grew `crates/engine-core/tests/table.rs`'s `NATIVE` set from
seven entries to ten: the three pairs SPEC-348 R1 adds to the native column. That test pins the
column, so admitting a pair meant editing it; the edit is recorded here as the one weakening a
reader of that table should know about, and the three rows S34803 to S34805 hold each entry.

A11 precedes A8 in the commit order because A8 reads the fixture A11 proves. A8, A9 and A10 each
committed the module or type their test names with a stub body: the native face's reader finds no
file, the voice choice holds nothing, and the collection directory is the default for every
argument. `no_argument_gives_the_default` was green at A10's red commit, because the stub returns
the default; the other five of A10's six tests were red there.

Two green commits also touched files their red commit wrote, and neither changes an assertion. 68d81d2e,
A3 to A7's green, adds a lint allow for `print_stdout` to `crates/engine-core/tests/face.rs`, two
semicolons there, and renames one binding in `crates/engine-core/tests/review_pairs.rs`. 6aa81b35,
A11's green, removes from the builder under `crates/ffi/tests/support/review.rs` the lint
expectation that the stub body needed, since the real body returns each step's error.

Part 2 (R9 to R20, A12 to A23). Its documents were committed first (3319d9f5), and the deck counts
(R15) moved to a later part at 5dd4003e, before any test of the app's code.

- A12, A13 and A14 were committed alone at 71f14966, before the register entries and the app
  files they read. Each was red at 2f286125, the pull request's first run (run 37660220852, job
  `hygiene`), and each was still red at 15d28e1a, where the register did not yet hold the review
  screen's files.
- R20's census, `TheReviewScreenIsTestedOnBothSimulators`, and SPEC-347 A14's order rule, which
  gains the review step, were red at 2f286125 in the same run and green at 15d28e1a, which added
  the step they read.
- A15 to A23 ran over stubs at 15d28e1a: the codec's new encoders and decoders wrote and read
  nothing, the review model's one entry did nothing, and `EngineSession.open(arguments:)` opened
  the app's own collection whatever directory its arguments named.
- So A15 to A18 were red at the fixture's open, not at their first assertion: the stub opened the
  collection the hosting app already held, and the engine refused it as already open
  (`ReviewSessionTests.swift:27`, the helper all four call first). Opening the directory the
  launch argument names is the code's, and these four criteria read through it before anything
  else. A19 was red at its reading of the absent Again button, and A20 to A22 at their first
  assertion.
- A23's two tests were red with SPEC-339 A3's new card, which now expects all five states:
  `test_a3_each_response_decodes_from_its_literal_bytes` failed at `ResponseDecodingTests.swift:40`
  in the same run, since the queue's decoder read the again, hard and easy states as empty.
- The player's refusal test, `test_a_sound_the_player_refuses_is_named_in_one_line` (section 10),
  was red over the stub player on the iPhone and on the iPad: `ReviewModelTests.swift:79:
  XCTAssertEqual failed: ("[]") is not equal to ("["The sound noise.wav could not be played."]")`.
- The code (5e678f82, 73864e2f and f3559685) was committed before these red lines, which could be
  written only once CI had read the reds. No test file changed between 15d28e1a and f3559685.
- A12 to A23 were read at edce06bf, the pull request's third head: A12 to A14 in run 37684801761,
  job `hygiene`; A15 to A22 in run 37684802369, job `harness`, on the iPhone and on the iPad; A23
  in the same run, job `harness-wire`. All but A17 and A19 were green there, and their green lines
  name it.
- A17 and A19 were red at edce06bf for a reason their red lines do not name. On both simulators
  each read the first card's Easy interval as `5d` against A1's `4d`
  (`ReviewSessionTests.swift:66`, and `ReviewFlowTests.swift:41` on the Easy rating's value). The
  cause was the review fixture, not the review screen: the engine seeds a review interval's fuzz
  from the card's id, and the part-1 builder in `crates/ffi/tests/support/review.rs` kept the ids
  the engine gave from its clock, so the word could change from run to run. The builder now fixes
  its four ids (4339c673), and the text card, which the review screen shows first, carries one
  measured to read A1's four words in this fixture. The core's own fixed id reads `5d` here: the
  engine's load balancer counts each new card's position as a day's load, and this deck's sound
  and speech cards sit inside the Easy range. The builder's comment records the measurement.
- The fix's test, `review_pairs::the_review_fixture_shows_first_the_card_whose_intervals_a1_pins`
  in `crates/ffi/tests/review_pairs.rs`, makes the app's calls over the fixture: it asserts the
  four fixed ids, then that the first queued card is the text card and that its four words are
  A1's. It was committed alone at 384e12d9 and was red there, at the ids:
  ``review_pairs.rs:239:5: assertion `left == right` failed: the review fixture's text, image,
  sound and speech cards carry their fixed ids left: [1791415240048, 1791415240052, 1791415240054,
  1791415240055] right: [1000007, 1000008, 1000009, 1000010]``. It was green at 4339c673, with the
  crate's other tests. After A17's and A19's red lines, three commits change a test file: these
  two and edce06bf, which adds the player's test below. None changes an assertion of A17 or A19.
  A17's and A19's green lines name 4339c673, which the pull request's fourth head carries, and
  that head's run reads them.
- The player's test from section 10,
  `test_a_replaced_sounds_end_leaves_the_new_list_where_it_is`, was red at edce06bf on both
  simulators (`ReviewModelTests.swift:99` and `:102`), and its green is 1381a8c6, the guard that
  lets a clip's end start the next clip only while that clip is still the current one.

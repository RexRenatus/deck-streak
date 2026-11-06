# Red-first record: SPEC-600

The SPEC, ADR-620, the schematic and the changelog fragment were installed first. The tests came
next, whole, over an inert seam: the shell's model takes its engine through `ShellEngine`, and each
moment's entry exists and does nothing new, so every red is an assertion on the missing behaviour,
never a missing symbol. The XCTest lines are read from the macOS lane's runs on one iPhone and one
iPad simulator, and A10's from the census run on the same tree; each red and green is appended
below once it is seen.

```red-first
A11: not red: the thin-Swift, app-tree and undo-reach censuses hold unchanged constraints over the changed files
A12: not red: SPEC-503's sync model tests are unchanged and hold unchanged behaviour
A1: red at 42f28480: SyncMomentsTests.swift:140 XCTAssertEqual failed: ("["open", "decks"]") is not equal to ("["open", "decks", "sync"]") - A1: the launch lists the decks, then syncs, run 41830915870
A1: green at ff1eb277
A2: red at 42f28480: SyncMomentsTests.swift:162 XCTAssertEqual failed: ("0") is not equal to ("1") - A2: the launch syncs, and an inactive turn is no moment, run 41830915870
A2: green at ff1eb277
A3: red at 42f28480: SyncMomentsTests.swift:192 XCTAssertEqual failed: ("0") is not equal to ("1") - A3: the launch syncs, and a return under a review does not, run 41830915870
A3: green at ff1eb277
A4: red at 42f28480: SyncMomentsTests.swift:213 XCTAssertEqual failed: ("1") is not equal to ("2") - A4: a return asked while a sync runs is dropped, run 41830915870
A4: green at ff1eb277
A5: red at 42f28480: SyncMomentsTests.swift:236 XCTAssertEqual failed: ("["open", "decks"]") is not equal to ("["open", "decks", "sync", "unsynced", "count 3"]") - A5: the launch's sync stops at the unsynced read and the counts, run 41830915870
A5: green at ff1eb277
A6: red at 42f28480: SyncMomentsTests.swift:256 XCTAssertEqual failed: ("["login"]") is not equal to ("["login", "sync"]") - A6: one sync after the sign-in, run 41830915870
A6: green at ff1eb277
A7: red at 42f28480: SyncMomentsTests.swift:272 XCTAssertEqual failed: ("0") is not equal to ("2") - A7: the launch's sync, then one at the review's close, run 41830915870
A7: green at ff1eb277
A8: red at 42f28480: SyncMomentsTests.swift:294 XCTAssertEqual failed: ("["open", "decks", "download", "sync", "unsynced", "count 3"]") is not equal to ("["open", "decks", "sync", "unsynced", "count 3", "download", "sync", "decks"]") - A8: after the written download and its normal sync, the decks are read again, run 41830915870
A8: green at ff1eb277
A9: red at 42f28480: SyncMomentsTests.swift:312 XCTAssertEqual failed: ("[DeckStreak.Deck(id: 1, name: "Default"), DeckStreak.Deck(id: 2, name: "Gone")]") is not equal to ("[DeckStreak.Deck(id: 1, name: "Default")]") - A9: the decks are read again, run 41830915870
A9: green at ff1eb277
A10: red at 42f28480: test_ios_sync_moments.py: FAIL test_the_shell_hands_each_scene_phase_and_the_review_close_to_the_model and FAIL test_each_planted_moment_site_is_refused_by_name, the first problem 'ios/App/Sources/DeckListView.swift: `@Environment(\\.scenePhase)` found 0 time(s), held 1', run 41830915819
A10: green at ff1eb277
```

The commit that appends these lines changes only this record: the app's sources, its tests and the rows are byte-equal to the green commit.

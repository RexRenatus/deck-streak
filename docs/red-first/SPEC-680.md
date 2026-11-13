# SPEC-680: the red-first record

```red-first
A1: red at 26c806a: XCTAssertTrue failed - a question asked in dark mode carries the night classes
A1: green at 1eaa86e
A2: red at 26c806a: XCTAssertTrue failed - a fresh install offers to sync the collection once its decks are listed
A2: green at 8b32f29
A3: red at 26c806a: XCTAssertEqual failed: ("["0: reads another finish\'s words", "1: names no count, reads another finish\'s words", "2: names no count, reads another ...
A3: green at f29ea0b
A4: red at 26c806a: XCTAssertEqual failed: ("1") is not equal to ("0") - a grade pressed as the answer shows is not sent
A4: green at f9a4735
A5: red at 26c806a: XCTAssertEqual failed: ("["Error Domain=NSCocoaErrorDomain Code=4 \"The file doesn’t exist.\": reads other words, prints its description", "Error ...
A5: green at 49f6c3e
A6: red at 26c806a: XCTAssertEqual failed: ("0") is not equal to ("1") - the first card is announced: []
A6: green at 5340b5c
A7: red at 26c806a: XCTAssertTrue failed - the password field's return key starts the sign-in, and the refused login's message shows; the sheet read:
A7: green at 5137eca
A8: red at 26c806a: XCTAssertEqual failed: ("(not shown)") is not equal to ("4 new, 0 learning, 0 to review") - the queue's three counts read as one element, labelled in ...
A8: green at b6dc70b
```

Each requirement's test entered in its own test commit, with only the stub its design names: R1 in
691f114, R2 in 27e6979, R3 in 619fba5, R4 in 9bcfe61, R5 in 5fc5713, R6 in aeb6bd8, R7 in 79f7245,
R8 in 26c806a. The first push's head, 44a40cd, adds only the SPEC's criterion table after them, so
`git diff --quiet 26c806a 44a40cd -- ios` reads rc 0, and each red line names 26c806a, the test
commit whose `ios` tree that push carried.

The pull request's `apple-on-change` run read all eight red by an assertion failure on the iPhone 17
and then the iPad (A16) destination, in xcodebuild's order, in its `release-and-app` job
(131536560120): the tests of R2, R3, R5 and R7 in the app's tests, and those of R1, R4, R6 and R8 in
the review screen's tests. Each line quotes the iPhone's first failing assertion, and the iPad's
reads the same.

From 26c806a to each green, the one commit that changes a file under `ios/AppTests` or
`ios/AppUITests` is b6dc70b, the green of R8, which moves the review flow's existing counts reading
to the one `counts` element and keeps its expected three counts, one per count. It changes no
requirement's own test, and no commit after the reds changes any of them.

The green commit `b6dc70b` carries the rows, and `git diff --quiet b6dc70b HEAD -- ios/App/Sources/AccountView.swift ios/App/Sources/AnswerBar.swift ios/App/Sources/AnswerGate.swift ios/App/Sources/AppModel.swift ios/App/Sources/DeckListView.swift ios/App/Sources/EngineSession.swift ios/App/Sources/ErrorWords.swift ios/App/Sources/FinishWords.swift ios/App/Sources/FirstRunOffer.swift ios/App/Sources/ReviewAnnouncer.swift ios/App/Sources/ReviewChrome.swift ios/App/Sources/ReviewModel.swift ios/App/Sources/ReviewSession.swift ios/App/Sources/ReviewView.swift ios/AppTests/ErrorWordsTests.swift ios/AppTests/FinishWordsTests.swift ios/AppTests/ReviewModelTests.swift ios/AppTests/ReviewSessionTests.swift ios/AppUITests/AccountFormFlowTests.swift ios/AppUITests/FirstRunFlowTests.swift ios/AppUITests/ReviewFlowTests.swift ios/swift-roles.json` reads rc 0.

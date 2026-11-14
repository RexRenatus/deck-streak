# SPEC-687 red-first record

The red commit adds the two tests with an inert `InstalledVoiceList`, whose initializer reads the
list on its caller's actor as the old default provider did; the green commit moves the read off the
main actor and makes `InstalledVoiceList.listed` the review's default. A Swift test cannot run where
this record is written, so each line is read in CI by its test's name, on the iPhone and on the
iPad, in the `release-and-app` job's step "the review screen's tests, Debug, on the iPhone and then the iPad", at the commit the line names.

```text
A1: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a1_the_device_voice_list_is_read_off_the_main_thread
A2: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a2_the_first_card_shows_while_the_voice_list_is_read
```

```red-first
A1: red at 13c176089e7970b7e4f3398da94ad05dbc6e9c65: iPhone and iPad, release-and-app, the review screen's tests, test_a1_the_device_voice_list_is_read_off_the_main_thread: XCTAssertEqual failed: ("[true]") is not equal to ("[false]") - the device's voice list is read once, off the main thread (SPEC-687)
A1: green at adbc717d9c570bdc844fca8807cf187bd82e1edf: iPhone and iPad, release-and-app, the review screen's tests, test_a1_the_device_voice_list_is_read_off_the_main_thread
A2: red at 13c176089e7970b7e4f3398da94ad05dbc6e9c65: iPhone and iPad, release-and-app, the review screen's tests, test_a2_the_first_card_shows_while_the_voice_list_is_read: XCTAssertFalse failed - the first card shows while the voice list's read is held (SPEC-687)
A2: green at adbc717d9c570bdc844fca8807cf187bd82e1edf: iPhone and iPad, release-and-app, the review screen's tests, test_a2_the_first_card_shows_while_the_voice_list_is_read
```

Between the red and the green the cure changes only `ios/App/Sources/InstalledVoices.swift` and
`ios/App/Sources/ReviewModel.swift`; `ios/AppTests/ReviewModelTests.swift` is byte-unchanged from
the red commit.

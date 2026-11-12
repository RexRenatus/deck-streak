# SPEC-615 red-first record

A1 and A3 are red at the red commit, which adds the tests with an inert day pin that reads the
collection's day and writes nothing, and green at the commit that delivers the pin and the writer's
call. A1 reads red on its lead window, because the inert pin leaves the hour A1's setup sets, whose
day ends within the hour; A3 reads red on its census of the writer. A2 is a guard on the text
card's four intervals, unchanged by the pin, so it is `not red`. A4 is SPEC-348's kept Swift tests,
read in CI. Every line below was measured locally by the cargo line its criterion names, run on a
tree byte-identical to the commit it names.

```text
A1: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact the_review_fixture_day_ends_twelve_hours_after_its_writer
A2: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact the_pinned_review_fixture_keeps_the_text_cards_four_intervals
A3: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact the_fixture_writer_pins_the_review_collection_once
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewModelTests/test_a15_the_loop_reaches_the_designed_end
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a17_each_rating_shows_its_interval
A4: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakUITests/ReviewFlowTests/test_a19_the_ratings_sit_at_the_bottom_named_by_their_titles
A5: cargo test -p deck-streak-ffi --test fixture_day_cutoff -- --exact an_earlier_runs_scratch_directory_is_cleared_before_a_test_writes
```

```red-first
A1: red at c4854d64c53197583c700f1e73ed85a6f46fdb1f: the fixture's day ends more than eleven hours ahead
A1: green at df125de39fceb99374d1907e568d9148f7c0ccbb
A3: red at c4854d64c53197583c700f1e73ed85a6f46fdb1f: assertion `left == right` failed: the writer calls the day pin once: []
A3: green at df125de39fceb99374d1907e568d9148f7c0ccbb
A2: not red: a guard on the text card's four intervals, unchanged by the pin
A4: not red: existing Swift tests, read green by name on both destinations in apple / release-and-app job 126669104649, over the pinned fixture
A5: red at f1d6ac2d3342600d786dff56e7e890225dc396aa: is absent before the test writes it
A5: green at af0b0edccb44d5e9e87e5453ae72f6182c728ae6
```

The pin replaced the inert seam of crates/ffi/tests/support/day_cutoff.rs at df125de3, between the red and the green; crates/ffi/tests/fixture_day_cutoff.rs is byte-unchanged from c4854d64.

A5 came with a later pull request, which removes a scratch directory an earlier run left under the same process id: A5 is red at its red commit, whose helper still refuses that directory, and green at the commit that removes it first, measured locally by its cargo line as the lines above were. A4's reading is the review screen's step of the job its line names, at the last head of the pull request that delivered this SPEC.
The cure changed only the scratch helper of crates/ffi/tests/fixture_day_cutoff.rs at af0b0edc, between A5's red and its green; every test function in that file is byte-unchanged from f1d6ac2d.

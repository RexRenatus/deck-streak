# Red-first record: SPEC-410

SPEC-410 (R1 to R10, A1 to A7). The SPEC, ADR-424, the schematic's section 7 and the three
amendments were committed first (`d714a7d43926f1e73814e49e20ac2847fddab9c1`). The seam and the
fixture were committed next (`503916e46897b3f4ef387ab7bdb24cb2cc9aeb0e`): `Reader::shows_video`
answers `false` by default and the native media folder answers `true`, but nothing routes a sound
tag's video yet, so every face is the base's; and the review fixture's example writes the sound
video collection beside the parity one. The tests of A1 to A6 were then written over that commit,
and A1, A2, A3 and A5 each read red for its own criterion, on an assertion about the missing
element. Every red below is quoted from the run over that commit, locally. A6 runs only on the
simulators, so its lines are read from the pull request's checks, by run, job, step and
destination, and join this record with the green.

```text
A1: cargo test -p deck-streak-engine-core --test face -- --exact a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks
A2: cargo test -p deck-streak-engine-core --test face -- --exact the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips
A3: cargo test -p deck-streak-engine-core --test face -- --exact a_video_over_the_file_cap_is_omitted_and_named_once
A4: cargo test -p deck-streak-engine-core --test face -- --exact a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound
A5: cargo test -p deck-streak-ffi --test face -- --exact a_sound_tags_video_plays_inline_in_the_native_document
A6: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/ReviewSessionTests/test_a_sound_tags_video_shows_in_the_review_screens_face
A7: python3 -m unittest discover -s scripts/tests -p test_ios_thin_swift.py
```

```red-first
A1: red at 503916e46897b3f4ef387ab7bdb24cb2cc9aeb0e: assertion `left == right` failed: the question's text ends with sign's element, then wave's; left: "" right: "<video controls style=\"display:block;max-width:100%;height:auto;margin:0 auto\" src=\"data:video/mp4;base64,c2lnbi12aWRlbw==\"></video><video controls style=... src=\"data:video/mp4;base64,d2F2ZS12aWRlbw==\"></video>"
A2: red at 503916e46897b3f4ef387ab7bdb24cb2cc9aeb0e: assertion `left == right` failed: the answer ends with the question's videos, then its own; left: "" right: sign's, wave's, then turn's element, turn's src="data:video/quicktime;base64,dHVybi12aWRlbw=="
A3: red at 503916e46897b3f4ef387ab7bdb24cb2cc9aeb0e: assertion `left == right` failed: the question's text ends with small's element alone; left: "" right: "<video controls style=\"display:block;max-width:100%;height:auto;margin:0 auto\" src=\"data:video/mp4;base64,c21hbGwtdmlkZW8=\"></video>"
A4: not red: pins what the base already does for a reader that does not ask, on the path the change touches: sign.mp4 and beep.mp3 replay as sounds, and wave.m4v and turn.MOV are named in omitted (SPEC-410 section 3); it passed over the seam commit (1 test)
A5: red at 503916e46897b3f4ef387ab7bdb24cb2cc9aeb0e: the body ends with sign's video, then wave's, each playing inline: the document's body is empty, <body class="card card1"></body></html>
A7: not red: guards that no Swift product decision is added; it passed over the seam commit with the new test in the tree (1 test, examined 63 Swift files, 209 doors, 19 decision counts) (SPEC-410 section 3)
```

The whole suites over the seam commit with these tests in the tree, every binary run: the core's
121 tests, 118 passed and 3 failed (A1, A2 and A3); the native adapter's 46 tests, 45 passed and 1
failed (A5); the web engine's 37 passed. The formatter and the lints of all three crates were green,
and the review fixture's example wrote `sound-video/` with its four files.

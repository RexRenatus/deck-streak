# Red-first record: SPEC-388

The SPEC, ADR-402 and the schematic's section were committed first (7aed4334), before any test or
code. The engine core's tests, with the stubs that keep every input, were committed next
(a8e170ae), each before the code that turns it green. Each red below is quoted from the run at the
red commit.

## The fence, line by line

Each of the 16 lines of SPEC-388 section 3's fence resolves to a test this delivery adds.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/ai_assisted.rs` `the_marker_is_the_tag_ai_assisted` | added (step 2) |
| 2 | A2 | `crates/engine-core/tests/ai_assisted.rs` `a_note_among_other_tags_is_marked` | added (step 2) |
| 3 | A3 | `crates/engine-core/tests/ai_assisted.rs` `the_marker_is_read_in_any_case` | added (step 2) |
| 4 | A4 | `crates/engine-core/tests/ai_assisted.rs` `only_the_whole_tag_marks_a_note` | added (step 2) |
| 5 | A5 | `crates/engine-core/tests/ai_assisted.rs` `the_core_reads_the_engines_own_tags` | added (step 2) |
| 6 | A6 | `crates/engine-core/tests/ai_assisted.rs` `a_note_the_collection_lacks_reads_unmarked` | added (step 2) |
| 7 | A7 | `crates/engine-core/tests/ai_assisted.rs` `the_marker_survives_the_engines_package_export_and_import` | added (step 2) |
| 8 | A8 | `crates/engine-core/tests/ai_assisted.rs` `a_newer_package_moves_the_marker_both_ways` | added (step 2) |
| 9 | A9 | `crates/engine-core/tests/ai_assisted.rs` `the_marker_survives_full_and_normal_sync` | added (step 2) |
| 10 | A10 | `crates/web-engine/tests/ai_assisted_view.rs` `the_shown_card_carries_the_cores_marker_answer` | added (step 3) |
| 11 | A11 | `web/app/src/lib/study/ai-assisted-line.test.ts` "a marked card shows the line above the card" | added (step 4) |
| 12 | A12 | `web/app/src/lib/study/ai-assisted-line.test.ts` "an unmarked card shows no line" | added (step 4) |
| 13 | A13 | `web/app/src/lib/study/ai-assisted-line.test.ts` "every locale holds both AI-assisted texts" | added (step 4) |
| 14 | A14 | `web/app/src/lib/study/ai-assisted-line.test.ts` "the AI-assisted texts name no maker" | added (step 4) |
| 15 | A15 | `web/app/src/routes/progress.test.ts` "states once how AI-assisted cards are marked, under the courses" | added (step 4) |
| 16 | A16 | `web/app/src/routes/progress.test.ts` "says nothing about AI-assisted cards when there is no course" | added (step 4) |

## The reds and greens

Each line's command is the criterion's line in SPEC-388 section 3's fence, run at the commit named.

```red-first
A1: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:162:5: assertion `left == right` failed: a note whose only tag is ai-assisted is marked, bare or as the engine stores its tags; left: [false, false], right: [true, true]
A2: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:171:5: assertion `left == right` failed: ai-assisted marks a note at the start, in the middle and at the end of its tags; left: [false, false, false], right: [true, true, true]
A3: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:184:5: assertion `left == right` failed: the marker is read without regard to ASCII case, as the engine compares tags; left: [false, false, false], right: [true, true, true]
A4: not red: the stub carries answers false; it pins the unmarked side, a note with no tags, a longer tag, a child tag or a part of the tag
A5: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:230:5: assertion `left == right` failed: the core's answer for the marked note and for the note tagged vocab is the engine's reading of each note's tags; left: [Ok(false), Ok(false)], right: [Ok(true), Ok(false)]
A6: not red: the stub read runs the fixed statement and answers false; it pins that a note the collection lacks reads unmarked, with no refusal
A7: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:356:5: assertion `left == right` failed: the core reads the imported marked note marked and the other unmarked; left: [Ok(false), Ok(false)], right: [Ok(true), Ok(false)]
A8: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:397:5: assertion `left == right` failed: the core reads the note unmarked, then marked after the marking package, then unmarked after the newer package without the marker; left: [Ok(false), Ok(false), Ok(false)], right: [Ok(false), Ok(true), Ok(false)]
A9: red at a8e170ae: panicked at crates/engine-core/tests/ai_assisted.rs:491:5: assertion `left == right` failed: the core reads the marked note marked on the second device, and the note marked later unmarked before the normal syncs and marked after them; left: [Ok(false), Ok(false), Ok(false)], right: [Ok(true), Ok(false), Ok(true)]
```

## What the record discloses

- **The stubs keep every input.** At a8e170ae `carries` takes the note's tags and answers false,
  and the dispatcher's `ai_assisted` runs the core's one fixed read of the note's tags, by the
  statement the green step keeps, and answers false. So each red is the missing behaviour, never
  a missing item, and A6's read already runs the statement it pins.
- **Each read's oracle held at the red commit.** A5, A7, A8 and A9 assert the engine's own read of
  the note's tags before the core's answer, and each failed on the core's line: the engine's
  package import took the marker away when a newer package without it arrived (A8), and the
  engine's full and normal sync carried it to the second device (A9).

# Red-first record: SPEC-376

The SPEC, ADR-387 and the schematic's section were committed first (8cf11690), before any test or
code. The two censuses, A8 and A11, were then committed alone (dab01780), and the criteria's tests
with the stubs that keep every input after them (90361ac2), each before the code that turns it
green. Each red below is quoted from the run at the red commit.

## The fence, line by line

Each of the 11 lines of SPEC-376 section 3's fence resolves to a test this delivery adds.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/late.rs` `a_review_card_due_yesterday_is_past_its_due_day` | added (step 3) |
| 2 | A2 | `crates/engine-core/tests/late.rs` `a_review_card_due_today_is_not_past_its_due_day` | added (step 3) |
| 3 | A3 | `crates/engine-core/tests/late.rs` `a_day_learning_card_is_judged_by_its_due_day` | added (step 3) |
| 4 | A4 | `crates/engine-core/tests/late.rs` `a_filtered_card_is_judged_by_its_home_due` | added (step 3) |
| 5 | A5 | `crates/engine-core/tests/late.rs` `an_intraday_learning_card_is_judged_by_the_days_start` | added (step 3) |
| 6 | A6 | `crates/engine-core/tests/late.rs` `a_new_or_preview_card_is_never_past_its_due_day` | added (step 3) |
| 7 | A7 | `crates/engine-core/tests/late.rs` `the_core_reads_the_engines_own_day` | added (step 3) |
| 8 | A8 | `crates/web-engine/tests/late_view.rs` `the_shown_card_carries_the_cores_late_answer` | added (step 2) |
| 9 | A9 | `web/app/src/lib/study/late-line.test.ts` "a late card shows the line above the card" | added (step 3) |
| 10 | A10 | `web/app/src/lib/study/late-line.test.ts` "a card on time shows no line" | added (step 3) |
| 11 | A11 | `web/app/src/lib/study/late-line.test.ts` "every locale holds the late line" | added (step 2) |

## The reds and greens

Each line's command is the criterion's line in SPEC-376 section 3's fence, run at the commit named.

```red-first
A1: red at 90361ac2: panicked at crates/engine-core/tests/late.rs:81:5: assertion `left == right` failed: a review card due yesterday, or a week ago, is past its due day; left: [false, false], right: [true, true]
A2: not red: the stub past_due_day answers false; it pins the on-time side
A3: red at 90361ac2: panicked at crates/engine-core/tests/late.rs:99:5: assertion `left == right` failed: a day-learning card due yesterday is past its due day, and one due today is not; left: [false, false], right: [true, false]
A4: red at 90361ac2: panicked at crates/engine-core/tests/late.rs:108:5: assertion `left == right` failed: a filtered card is judged by its home deck due, never by its filtered position; left: [false, false, false, false, false], right: [true, false, true, true, false]
A5: red at 90361ac2: panicked at crates/engine-core/tests/late.rs:137:5: assertion `left == right` failed: a learning card due before the engine's day began is past its due day, and one due at its start or later is not; left: [false, false, false], right: [true, false, false]
A6: not red: the stub past_due_day answers false; it guards the other queues
A7: red at 90361ac2: panicked at crates/engine-core/tests/late.rs:212:5: the core's engine day Ok(EngineDay { days_elapsed: 0, next_day_at: 0 }) is not the engine's own timing of today
A8: red at dab01780: panicked at crates/web-engine/tests/late_view.rs:174:5: assertion `left == right` failed: the shown card's view does not carry the core's late answer, in one place; left: ["current_card's view holds 0 `late` key(s), not one", "src/wasm.rs calls the core's rule 0 time(s), not once"], right: []
A9: red at 90361ac2: AssertionError: a late card shows the line on its question side: expected null not to be null (late-line.test.ts:155)
A10: not red: the base review screen draws no line; it guards the absence
A11: red at dab01780: AssertionError: expected { …(7) } to deeply equal { Object (en, es, ...) }: en, es, fr, ja, ko, zh-Hans and zh-Hant each read "lacks study_late_review" (late-line.test.ts:66)
```

## What the record discloses

- **A7's red is quoted in short.** The run printed the engine's own day after the quoted text, its
  day count and its next rollover read before and after the core's read; the rollover is an instant
  of the run's clock, so the record stops at the core's answer.
- **The stubs keep every input.** At 90361ac2 `past_due_day` takes the card and the engine's day
  and answers false, and the core's day read is the dispatcher's own method answering a day of 0
  that ends at instant 0, so each red is the missing behaviour, never a missing item.
- **Six fixture files, not eight, gained `late: false`.** SPEC-376 section 4 names eight; the type
  check at 90361ac2, with `late` required and no fixture changed, refused exactly six:
  `audio.test.ts`, `review-screen.test.ts`, `review-templates.test.ts`, `review.test.ts`,
  `voice.test.ts` and `routes/study.test.ts`. `web/app/src/lib/engine/session.test.ts` builds the
  module's JSON text, which the type check does not read as a card view, and
  `web/app/src/lib/study/answer-buttons.test.ts` builds no card view; both are left unchanged, as
  section 4 says.

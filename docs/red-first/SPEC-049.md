# Red-first record: SPEC-049

## The lapse slice (ADR-088)

The predecessor's lapse episode was registered and generated first (72e5f92), an inert `open_lapse`
that compiled and always answered no lapse was committed in streaks and in coordination
(474da3f), then the tests were committed (5fb50bd). Each criterion was run there with the SPEC's own
fenced command and failed by assertion, not by a compile error, a missing fixture or an empty
selection. The implementation followed in two commits, streaks first (fed4e0a) and coordination
second (d392ab7). Between the red and the green no test changed what it asserts, and one test that was never red,
`an_empty_window_holds_no_lapse`, gained a positive assertion after the tdd probe refused it for
asserting only an absence. A15 stays red
until the coordination commit because it counts through streaks' rule, which the first green
supplies.

```red-first
A11: red at 5fb50bd4f939f0cdd1a4c7d99050c9f83ea5e77f: assertion `left == right` failed: class Some("three-silent"), the predecessor's lapse differs; left: None, right: Some(19998)
A11: green at fed4e0a6a0f8bfdd40fa916bc780e0bdb407dd10
A12: red at 5fb50bd4f939f0cdd1a4c7d99050c9f83ea5e77f: assertion `left == right` failed: three silent days; left: None, right: Some(19998)
A12: green at fed4e0a6a0f8bfdd40fa916bc780e0bdb407dd10
A13: red at 5fb50bd4f939f0cdd1a4c7d99050c9f83ea5e77f: assertion `left == right` failed: the third silent day; left: None, right: Some(19996)
A13: green at fed4e0a6a0f8bfdd40fa916bc780e0bdb407dd10
A15: red at 5fb50bd4f939f0cdd1a4c7d99050c9f83ea5e77f: assertion `left == right` failed; left: None, right: Some(StudyDay(19997))
A15: green at d392ab75dbaec5b44332ca1362dd09e6dc67249e
```

## The lapse walk's first day and the rollover boundary (issue 422; the 2026-09-29 amendment)

The code already satisfies R13 and R15, so no new test is red at the base: each criterion below
passes on the unchanged `open_lapse` in both crates, and each is recorded `not red` with the reason.
What each test is worth is shown by the plants: the tests were run beside a planted copy of the
production line, and each plant below turns them red by assertion. Each class has one rule. Class W:
the walk reads every day of its window and no other. Class R: a review counts on the study day the
configured rule gives its instant. Each test generates its population (the walk's 2,252 members and
the mapping's 448) and judges every member against an oracle written in the test from the rule's
words, never by calling the function under test.

```red-first
A16: not red: the walk already reads its whole window; the test's population is generated and its oracle is R13's words, so it pins the bounds and cannot turn red until a plant moves them (see the plants below)
A17: not red: the mapping already uses the configured rule; the test's population and its begins(day) oracle pin every boundary and cannot turn red until a plant changes the day (see the plants below)
A18: not red: the window's first day is already read; the two cases from the issue are members of A16's population, named on their own
```

A15's red panic, read as measured at the red commit (5fb50bd4f939f0cdd1a4c7d99050c9f83ea5e77f): the
test compares the ids as plain day numbers, so the first assertion (the test's line 56) reads
`assertion left == right failed`, `left: None`, `right: Some(19997)`. The A15 line above shows the
right side as a `StudyDay`; the day number is what the test compares.

The plants, each applied to a clean copy of the committed tests and quoted with its first red line
by assertion (test file line numbers are the committed file's). Plants on `crates/streaks/src/lapse.rs`,
against `the_walk_reads_every_day_of_its_window_and_none_outside_it` (W) and
`a_silent_run_on_the_windows_first_day_is_read` (F):

```text
(a) -    while number >= window_start {  ->  +    while number > window_start {
    F: panicked at crates/streaks/tests/lapse.rs:237:5: left: Some(19998) right: Some(19997)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: None right: Some(20001)
(b) the window's start one day earlier: let window_start = ....epoch_day() - 1;
    F: panicked at crates/streaks/tests/lapse.rs:237:5: left: Some(19996) right: Some(19997)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: Some(20000) right: None
(e1) the window's start is its LAST key (next_back)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: None
(e2) the walk starts one day before today (today.epoch_day() - 1)
    F: panicked at crates/streaks/tests/lapse.rs:240:5: left: None right: Some(19998)
(e3) the walk steps two days (checked_sub(2))
    W and F fail by assertion (the existing A12, A13 and A11 tests fail too)
(e4) the window's first day is never silent (&& number != window_start)
    F: panicked at crates/streaks/tests/lapse.rs:237:5: left: Some(19998) right: Some(19997)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: None right: Some(20001)
(e5) a review closes the run only from two reviews (> 1)
    W and F fail by assertion (the existing tests fail too)
```

Plants on `crates/coordination/src/lapse.rs`, each against
`a_review_counts_on_the_study_day_the_rule_gives_at_every_boundary`, which panics at
`crates/coordination/tests/lapse.rs:179:21`:

```text
(c) the study day is the UTC date: StudyDay::from_epoch_day(review.id.div_euclid(86_400_000))
    left: None right: Some(20000)
(d) the rollover taken as the default: StudyDayRule::default().study_day(...)
    left: None right: Some(20000)
(e1) the review's instant minus one millisecond      left: Some(20000) right: Some(20001)
(e2) the review's instant plus one millisecond       left: None right: Some(20000)
(e3) the review's instant plus one hour              left: None right: Some(20000)
(e4) the review's instant minus the offset           left: None right: Some(20000)
```

No plant stayed green, so none is recorded as equivalent. Rows S04907 to S04912 carry (a) and (b) of the walk
and (c), (d), (e1) and (e2) of the mapping; the remaining plants are killed by the existing tests as well as
by these, and each is a variant of a row above.

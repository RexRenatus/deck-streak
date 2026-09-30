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
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: None right: Some(20001)
(e2) the walk starts one day before today (today.epoch_day() - 1)
    F: panicked at crates/streaks/tests/lapse.rs:240:5: left: None right: Some(19998)
(e3) the walk steps two days (checked_sub(2))
    F: panicked at crates/streaks/tests/lapse.rs:237:5: left: None right: Some(19997)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: None right: Some(20001)
    (the existing A12, A13 and A11 tests fail too)
(e4) the window's first day is never silent (&& number != window_start)
    F: panicked at crates/streaks/tests/lapse.rs:237:5: left: Some(19998) right: Some(19997)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: None right: Some(20001)
(e5) a review closes the run only from two reviews (> 1)
    F: panicked at crates/streaks/tests/lapse.rs:240:5: left: Some(19997) right: Some(19998)
    W: panicked at crates/streaks/tests/lapse.rs:227:5: left: Some(20000) right: None
    (the existing tests fail too)
```

Plants on `crates/coordination/src/lapse.rs`, each against
`a_review_counts_on_the_study_day_the_rule_gives_at_every_boundary`, which panics at
`crates/coordination/tests/lapse.rs:181:21`:

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

No plant stayed green, so none is recorded as equivalent. Rows S04907 to S04919 carry every plant a row admits:
(a), (b) and (e1) to (e5) of the walk (S04907, S04908, S04913 to S04917), and (c), (d) and (e1) to (e4) of the
mapping (S04909 to S04912, S04918, S04919). Over dev's tests, (a), (b) and (e4) of the walk and every plant of
the mapping pass; (e1), (e2), (e3) and (e5) of the walk are killed there by the existing tests as well.

## The lapse populations pin their spread (issue 453; the 2026-09-30 amendment)

The counts already hold at the base, so the new distinct assertions cannot be red on their own. They
were committed beside two planted folds of the generators (9c46b04a5d561f716150f0236a9ce94664077b92):
in the walk's generator, place 0 of the run never enters the skip mask; in the mapping's, the offset
345 is written 330. Both plants keep the examined count and drop the distinct count, so the examined
assertions stay green and the distinct ones are red by assertion. The plants were removed in
2ad7e316058f2b958f4985a857c063d67e3b6feb.

```red-first
A19: red at 9c46b04a5d561f716150f0236a9ce94664077b92: assertion `left == right` failed: the population's spread: 964 distinct of 2252; left: 964, right: 1916
A19: green at 2ad7e316058f2b958f4985a857c063d67e3b6feb
A20: red at 9c46b04a5d561f716150f0236a9ce94664077b92: assertion `left == right` failed: the population's spread: 392 distinct of 448; left: 392, right: 448
A20: green at 2ad7e316058f2b958f4985a857c063d67e3b6feb
```

Printed lines on the green tree: `examined 2252 window member(s), 1916 distinct` and
`examined 448 rollover member(s), 448 distinct`. At the base the walk's population also holds 1916
distinct members of 2252: 336 repeat, 224 because for one or two earlier days the second and third
fills are the same map, and 112 because with no earlier day the pass that skips the closing day
skips nothing more and repeats the pass before it. Each repeat follows its first occurrence, so none
changes a verdict.

Further plants, each applied to a clean copy of the tests at 2ad7e316058f2b958f4985a857c063d67e3b6feb
(the line numbers are that file's; a later commit only fits the walk's test to clippy's line limit),
with the first red line by assertion. Each keeps the examined count and drops the distinct count:

```text
W fold, and the walk's `|| number == window_start` on the skip test, together
    panicked at crates/streaks/tests/lapse.rs:359:5: 964 distinct of 2252 (left: 964, right: 1916)
    (the walk plant alone is red by the oracle: the walk differs from R13; the fold alone hid it at the base)
W a fill replaced by a copy of another        1468 distinct of 2252
W the future-study axis is [false, false]     964 distinct of 2252
W the closing-day axis is [false, false]      1020 distinct of 2252
R two offsets made equal                      392 distinct of 448
R the hour axis is [4, 4]                     224 distinct of 448
R an instant replaced by a copy of another    384 distinct of 448
R a now replaced by a copy of another         336 distinct of 448
```

A plant that changes the examined count (a run length listed twice: 2572 members) is red by the
examined assertion, as before. No plant stayed green, so none is recorded as equivalent. Rows S04921
to S04926 carry one plant of each class (the walk's mask fold, fill copy and axis collapse; the
mapping's offset fold, hour collapse and now copy). Each is KILLED by full id on the committed tree,
and each survives over the base tests, which assert no distinct count.

### Fix round 1: each judge records its own member

The rollover test recorded a member from the generator's loop variables, so a fold of one judge's
input kept every count. Each judge now records its member from the arguments it is handed: the lapse
member is the rule, the instant of now and each review's instant, kind and ease (448 distinct), and
the day member is the rule and the instant (112 distinct, the new `DISTINCT_DAY_MEMBERS`). The
criterion is A20, whose red and green lines above stay as they are; this replay is prose.

The per-judge counts were committed beside two planted folds (e38cc56247459e21e96badb210a5341630c94626): the review the lapse judge
is handed folded to its day's first instant, and the instant the day judge is handed folded the same
way. Each keeps the examined count and drops one judge's distinct count. The plants were removed in
76d4aac219157962e3b58c03c34f3dc9949a8d76.

```text
A20 replay: red at e38cc56247459e21e96badb210a5341630c94626: the population's spread: 128 distinct of 448 (left: 128, right: 448); the two plants together, the lapse count is the first assertion to fail
A20 replay: red at e38cc56247459e21e96badb210a5341630c94626: the day judge's spread: 32 distinct (left: 32, right: 112), read with the lapse plant removed, because the lapse count fails first when both stand
A20 replay: green at 76d4aac219157962e3b58c03c34f3dc9949a8d76: examined 448 rollover member(s), 448 distinct, over 112 distinct day member(s)
```

Rows S04927 and S04928 carry the two folds; each is KILLED by full id on the committed tree. The
window test is unchanged and still prints `examined 2252 window member(s), 1916 distinct`.

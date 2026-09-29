# Red-first record: SPEC-049

## The lapse slice (ADR-088)

The predecessor's lapse episode was registered and generated first (72e5f92), an inert `open_lapse`
that compiled and always answered no lapse was committed in streaks and in coordination
(474da3f), then the tests were committed (5fb50bd). Each criterion was run there with the SPEC's own
fenced command and failed by assertion, not by a compile error, a missing fixture or an empty
selection. The implementation followed in two commits, streaks first (fed4e0a) and coordination
second (d392ab7). Between the red and the green no test changed what it asserts, and one test that was never red,
\`an_empty_window_holds_no_lapse\`, gained a positive assertion after the tdd probe refused it for
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

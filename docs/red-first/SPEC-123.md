# Red-first record: SPEC-123

The tests of A1 to A8 were committed first (ef6f220) against the base's runner, with no stub, since
every case drives `scripts/box-packs.sh` itself and its planted scan rows sit in
`test_box_packs.py`. Each was red by assertion: the base takes only `note` and `pending` on the
scan, so a wiring with `expected_red` reads VOID and the scan's line is absent. The change (a1f8c2d)
was read green on the tests one commit later (e644e16), after two amendments to the tests
themselves: the helper case reset its box to a wired copy for each sub-case, where the red test
reused a box that still carried the scan's key, and the closed test's asked-issue list gained `59`,
a name it had omitted (both stricter or equal). A5 then gained a sub-case, a closed issue on a green
row, read as one stale expectation, after row S12304 survived without it (90cd8bf). Each red was
read on a `git worktree` of ef6f220's tree; every criterion's green is read on e644e16's tree, A5's
on 90cd8bf's and A1's on d2cc071's.

After those greens, d2cc071 tightened two assertions: A1 now pins `0 void (1 expected)`, and A5
gains `unexpected 0, expected 1, stale 1`. A1's recorded green is the one read on d2cc071's tree,
after both tightenings. A fix round then added one test to A4's cell,
`an_expectation_with_no_settings_document_names_its_issues`, red by assertion at ba05130 (the detail
still said "the wiring names no issue" beside a named expectation) and green at c3fb53a; it adds no
mutation row, because the detail is a string and no gate.

```red-first
A1: red at ef6f220: AssertionError: False is not true : <0 lines for proxy-client-scan> (the wiring is refused as VOID)
A2: red at ef6f220: AssertionError: False is not true : <0 lines for proxy-client-scan>
A3: red at ef6f220: AssertionError: False is not true : <0 lines for proxy-client-scan>
A4: red at ef6f220: AssertionError: False is not true : <0 lines for proxy-client-scan> (three sub-cases: green, void and absent)
A5: red at ef6f220: AssertionError: False is not true : <0 lines for proxy-client-scan>
A6: red at ef6f220: AssertionError: 'box.proxy-client-scan is pending, so it expects no red row' not found in "box-packs: VOID: box.proxy-client-scan takes only ['note', 'pending']"
A7: red at ef6f220: AssertionError: "box.proxy-client-scan's expected_red maps a row to an issue" not found in "box-packs: VOID: box.proxy-client-scan takes only ['note', 'pending']" (five sub-cases)
A8: red at ef6f220: AssertionError: 2 != 0 : box-packs: VOID: box.proxy-client-scan takes only ['note', 'pending']
A1: green at d2cc071f6db48bef9ba3293f2a8604bfa3846177
A2: green at e644e16af8eae8cbe809c821aad52cfc21374095
A3: green at e644e16af8eae8cbe809c821aad52cfc21374095
A4: green at e644e16af8eae8cbe809c821aad52cfc21374095
A5: green at 90cd8bf0d7b86e45773644faa6427830db86a728
A6: green at e644e16af8eae8cbe809c821aad52cfc21374095
A7: green at e644e16af8eae8cbe809c821aad52cfc21374095
A8: green at e644e16af8eae8cbe809c821aad52cfc21374095
```

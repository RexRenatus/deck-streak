# Red-first record: SPEC-033

The tests were committed before the scrub, the tag ruleset and Dependabot changed (e59a935). Each
criterion was run there for its own reason. A9 and A10 pin two rulesets that already had their
shape, so they are disclosed as not red.

```red-first
A1: red at e59a935: AssertionError: 0 != 1 : examined 1 file(s) against public shapes only; 0 finding(s)
A1: green at 03273f6
A2: red at e59a935: AssertionError: 2 != 1 : public-scrub.py: error: unrecognized arguments: --history
A2: green at 03273f6
A3: red at e59a935: AssertionError: 2 != 1 : public-scrub.py: error: unrecognized arguments: --history
A3: green at 03273f6
A4: red at e59a935: AssertionError: 2 != 1 : public-scrub.py: error: unrecognized arguments: --history
A4: green at 03273f6
A5: red at e59a935: AssertionError: 2 != 0 : public-scrub.py: error: unrecognized arguments: --history
A5: green at 03273f6
A6: red at e59a935: AssertionError: 2 != 3 : public-scrub.py: error: unrecognized arguments: --history
A6: green at 03273f6
A7: red at e59a935: AssertionError: 3 != 1 : examined 0 file(s) against public shapes only; 0 finding(s)
A7: green at 03273f6
A8: red at e59a935: AssertionError: Regex didn't match: 'python3 scripts/public-scrub\.py --root \. --history\b'
A8: green at 03273f6
A9: not red: the committed main ruleset already merged only by merge commit after ci and fragment; the test pins it against drift
A10: not red: the committed dev ruleset already took pull requests only after ci and fragment; the test pins it against drift
A11: red at e59a935: AssertionError: Items in the first set but not the second: 'tag_name_pattern'
A11: green at e5ed34f
A12: red at e59a935: AssertionError: {'update', 'non_fast_forward', 'tag_name_pattern', 'deletion'} not less than or equal to the enforceable rule types : release-tags.json
A12: green at e5ed34f
A13: red at e59a935: AssertionError: Regex didn't match: the semver-major ignore : cargo
A13: green at e5ed34f
```

In A1 the binary was skipped as undecodable, so the tree read as clean. In A2 to A6 the history scan
did not exist. In A7 the oversize file was skipped, so nothing was examined. In A8 the gate's scrub
stage read only the tree.

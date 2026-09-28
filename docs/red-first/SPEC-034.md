# Red-first record: SPEC-034

The tests were committed (c79f14e) before the rulesets, the `ci` workflow and the documents changed.
Each criterion was run there for its own reason. A2, A6 and A7 pin behaviour that was already right,
so they are disclosed as not red. The same commit removed SPEC-033 A9's assertion that `main`
requires an up-to-date head, because ADR-034 reverses it; A9's criterion does not name strictness,
and A9 stays green.

```red-first
A1: red at c79f14e: AssertionError: True is not False
A1: green at d1f503f
A2: not red: dev already required an up-to-date head; the test pins it beside A1's reversal for main
A3: red at c79f14e: AssertionError: None != 15368 : main: ci
A3: green at d1f503f
A4: red at c79f14e: AssertionError: 'Nothing is merged back into dev' not found in the release runbook, which still merged main back into dev
A4: green at b848062
A5: red at c79f14e: AssertionError: 0 != 1 : base-is-dev: ok (pull_request into 'main')
A5: green at d1f503f
A6: not red: base-is-dev already admitted this repository's dev into main; the test is A5's positive control
A7: not red: base-is-dev already refused this repository's other branches; the test pins it through the new condition
A8: red at c79f14e: AssertionError: 'never approve a workflow run from a fork' not found in AGENTS.md
A8: green at b848062
A9: not red: the workflows already comply (they read no secret, pass none on, and check out only this repository), so the planted workflows prove the checker red instead (A10, A12)
A10: red at eeaa4e7: AssertionError: Lists differ: [] != ['checkout-of-another-repository.yml:jobs.[1729 chars]lls']; the stub admitted all eight planted workflows
A10: green at 705d506
A11: not red: the stub admitted everything, so the admitted workflows passed at eeaa4e7; the test is A10's positive control
A12: red at eeaa4e7: AssertionError: AssertionError not raised; the stub judged a directory that holds no workflow file
A12: green at 705d506
```

At c79f14e, A5 showed a fork's branch named `dev` passing `base-is-dev` into `main` (exit 0).

The amendment of 2026-09-28 (the SPEC's section 7, #216) committed the planted workflows and A9 to
A12 at eeaa4e7, against a checker stub that read every workflow and refused none, and the checker at
705d506. A hand sweep then deleted or inverted each branch of the checker in a scratch copy, 28
mutants. 26 were killed at once. The two that survived, the two boundaries of the word `secrets`,
are killed by the admitted workflow that 09eca13 and af4c5b7 add to A11, and the checker itself did
not change after 705d506 until the fix round below.

## Fix round, 2026-09-28

The review found that the checker's own workflow reader did not fail closed. The fix round made it
fail closed on quoting, escapes, anchors, aliases and tags, flow forms and keys, read a checkout
named in any case and git's scp-like form, held a `.yaml` workflow to the hardening tests (A13),
and pinned each rule with a planted workflow. `dev` was merged in first, at 9c84e61, with no
conflict. Each step was committed red, then green.

```red-first
A13: red at 403d9ce: AssertionError: AssertionError not raised, in each of three subtests; the hardening tests took .yml files only, so none saw the planted .yaml workflow
A13: green at 1b5f253
```

A10 and A11 are recorded above, so their fix-round runs are listed here, outside the fence:

- A10 red at 6de90e8: `AssertionError: Lists differ`; four planted workflows that hold a secret or a
  run step in quoted, escaped, aliased, tagged or flow forms, or under a key that is not a plain
  name, passed the reader. Green at 2d6731c, where the reader fails closed.
- A10 red at 8041679: `AssertionError: Lists differ`; flow-list items that hold a key, and a matrix
  item keyed by more than a plain name, were read as text. A11 red at 8041679:
  `AssertionError: line 27 was not read`; a key that begins with a dash was taken for a list item.
  Both green at a3f3ae6.
- A10 red at db630d1: `AssertionError: Lists differ`; a checkout named in another case and a fetch
  in git's scp-like form were admitted. Green at ac103f6.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7. Its expected list grew from 16 to 42
findings in four commits: 6de90e8 (13), 8041679 (3), db630d1 (7) and b0e917c (3). The first three
were committed red, as listed above. b0e917c's three pin reader rules that a hand mutant showed no
test held; the checker already met them, so they were green at once, and each is red against its
mutant. No finding that was in the list at eeaa4e7 was changed or removed. A9, A11, A12 and the
three hardening tests keep their bodies; their setUp takes its files from `workflow_files`.

At 7f9e423 a hand sweep ran 62 mutants over the checker and its reader, one at a time, on a scratch
copy of the committed tree, restoring the file byte for byte after each: the 28 of the first sweep,
re-expressed against the fix round's code, 11 from the review (V1, V3, V6 to V9, X06, and X12 in
four forms) and 23 of the reader's branches. 62 were killed and none survived. The first run, at
ac103f6, left five survivors: b0e917c's planted workflows kill four, and 7f9e423 removed the fifth's
branch, which read a bare dash as an empty item and which no test could tell apart.

## Fix round 2, 2026-09-28

The second review found forms the checker still admitted: white space and controls that YAML, or
GitHub's parser, reads as text, a checkout's input named in another case, a checkout from another
server, and actions/checkout written as the runner reads it apart. The checker failed on an empty
block and on a step that is not a mapping, and three hand mutants survived. `dev` had not moved
since 07322ae merged it. SPEC-034's inserted text was corrected first, at 0bec186. No criterion was
added, so the fences above are unchanged, and this round's runs are listed here. Each fix was
committed red, then green.

- A10 red at 34e669a: 48 of its subtests failed. 36 read `AssertionError: AssertionError not
  raised`: a planted line the reader cannot place was read. Four read `AssertionError: Lists
  differ` and eight `line 15 is not a mapping entry`: no line holding one of twelve characters
  outside printable ASCII was refused by its line, and for eight of them the reader split the line
  at the character. Green at 6ddbfb9.
- A10 red at d068a61: `AttributeError: 'str' object has no attribute 'get'`: the checker judged a
  step the reader had refused. A11 red at d068a61: `ValueError: min() iterable argument is empty`:
  the reader failed on an empty block. Both green at 36f9402.
- A10 red at e3b6f7a: `AssertionError: Lists differ`: two checkouts of another repository, their
  input named in another case, were admitted. Green at ce3cf5f.
- A10 red at 9e0828f: `AssertionError: Lists differ`: two checkouts from another server were
  admitted. Green at 18e3eac.
- A10 red at 9ee09d1: `AssertionError: Lists differ`: four checkouts of another repository, their
  action written as the runner reads actions/checkout, were admitted. A13 red at 9ee09d1:
  `AssertionError: AssertionError not raised`, in two of its three new subtests: the SHA-pin test
  admitted an action written with an empty part or a trailing slash. Both green at 5c12427.
- 94edf64 kills three hand mutants and was green at once, because the checker already met each.
  A13 runs each hardening test through its own setUp: Y0, whose setUp read `.yml` files only,
  survived until then. A10 plants a block the reader does not read before a clone, and a plain value
  over two lines (Y1, the final unplaced-line check deleted), and a secret after a `#` that no space
  precedes (Y5, a bare `#` read as a comment). 34e669a's planted characters already killed Y1 and
  Y5; with those subtests switched off, 94edf64's plants kill each.

DISCLOSURE: A10's body changed after its red commit, eeaa4e7, again in this round. Its committed
list grew from 42 to 53 findings: d068a61 (2), e3b6f7a (2), 9e0828f (2), 94edf64 (1) and 9ee09d1
(4). 34e669a added its planted characters and lines (48 subtests), and 94edf64 its two unplaced
forms. No finding that was in the list before this round changed or left. A11's body did not
change; its admitted workflows gained an empty block (d068a61) and two checkouts from this server
(9e0828f). A13's body changed after its red commit, 403d9ce: at 94edf64 its three subtests run
through their own setUp rather than being handed their files, and at 9ee09d1 it gained three
subtests for the SHA-pin test's forms. Its criterion now also says that an action is pinned only in
its plain form.

At 5c12427 a hand sweep ran 102 mutants over the checker, its reader and the SHA-pin pattern, one at
a time, on a scratch copy of the committed tree, restoring the file byte for byte after each: the 62
of the first fix round, re-expressed against this round's code, the second review's six (Y0 to Y3,
Y5 and Y6), and 34 of this round's: the character check and the line ends, each white-space site,
the empty block, the job, step and input guards, the reading of `uses`, inputs in any case, the
server, the SHA-pin pattern, and the quoted and flow-list dispatches. 100 were killed. Two are
equivalent: W12 and W13 read a block's trailing lines and its indent with white space other than a
space or a tab, which only a line the character check refuses can hold, and no finding reads white
space. Y2, which the second review recorded as equivalent, is killed: each planted line asserts the
reason its refusal gives.

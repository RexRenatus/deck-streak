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
not change after 705d506.

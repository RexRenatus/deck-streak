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
```

At c79f14e, A5 showed a fork's branch named `dev` passing `base-is-dev` into `main` (exit 0).

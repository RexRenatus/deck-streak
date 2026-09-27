# Red-first record: SPEC-002

The skeleton's structure was scaffolded before some of the tests that pin it; those criteria are
disclosed below as not red, with the reason. The guards written ahead of their subject were run
red first.

```red-first
A1: not red: the workspace was scaffolded before its shape test; the test pins every crate's name for every later delivery
A2: not red: the workspace was scaffolded before its lint test; the test pins the lint inheritance for every later delivery
A3: not red: the vendored copy preceded the test that pins its digests; the test guards against later drift
A4: red at ad8822f: AssertionError: unexpectedly None : ddd:lexicon-locks waits on '{{issue:DS-W0-01}}', which is not an issue number
A4: green at d54db93
A5: not red: the oracle's harness and its test were written together; the test pins round-half-to-even and strict JSON
A6: red at 769ee62: AssertionError: examined 0 numbered SPECs: the population is empty, so nothing was judged
A6: green at e05dfa5
A7: not red: the scrub preceded its test; the planted fixtures pin its refusals and its passes
A8: not red: the scaffold's smoke test pins the shell's heading and proves no behaviour
A9: not red: the workflow preceded its test; the test pins every gate stage against drift
A10: not red: the runner's refusal preceded its test; the test pins it with a planted wiring
```

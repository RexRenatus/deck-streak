# Red-first record: SPEC-612

Every criterion is decided by `scripts/tests/test_dynamic_imports_check.py`, which runs before
the push, so each red was read where it was written. The first commit carries SPEC-612, ADR-634
and the changelog fragment. The second (37c932d9) adds the six tests and leaves
`scripts/dynamic-imports-check.py` as dev holds it, so each red is read against the check the
issue describes, by an assertion and not by an error. The third (da3bf087) carries the fix, and no
test file changes in it. Each criterion was run alone by its unittest name filter, at the commit
its line names.

MUTATION COVERAGE: the python mutation run over the script at da3bf087 read 143 killed, 1 survived, 0 uncovered. The survivor replaced 0 with 1 in the else arm of the line in `register_modules` that reads the plain assignment's target, an arm that only a register written as an annotated assignment reaches. The cure a06bd1c7 adds one row to A3's table, an annotated register with one addition, which kills it; it is the one commit after 37c932d9 and up to the green commit that changes a test file. The second run, at a06bd1c7, read 144 killed, 0 survived, 0 uncovered.

```red-first
A1: red at 37c932d9: AssertionError: Lists differ: [(4, 'exec')] != []
A1: green at a06bd1c7
A2: red at 37c932d9: AssertionError: 1 != 0
A2: green at a06bd1c7
A3: red at 37c932d9: AssertionError: Lists differ: ['dyn[19 chars]ter: 1 module(s) in DYNAMIC_IMPORTS at HEAD', [234 chars]SED'] != ['dyn[19 chars]ter: 2 module(s) in DYNAMIC_IMPORTS at HEAD', [96 chars] OK']
A3: green at a06bd1c7
A4: red at 37c932d9: AssertionError: 0 != 2
A4: green at a06bd1c7
A5: red at 37c932d9: AssertionError: 1 != 2
A5: green at a06bd1c7
A6: red at 37c932d9: AssertionError: 1 != 0
A6: green at a06bd1c7
```

The green lines name a06bd1c7, the last commit that changes `scripts/`. The commits after it change
only documents, and the pull request's run at this delivery's head reads every criterion green.

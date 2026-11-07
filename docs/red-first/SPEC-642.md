# Red-first record: SPEC-642

SPEC-642, ADR-665, the passkey schematic's insert-only section 13 and the changelog fragment
were committed first and alone (784aa14e). The commit that adds this record carries the two
criteria's tests with the component as it was: there A1 is read red by name, by assertion on
the focus, and A2 is not red, as ADR-665 D3 says. That commit also removes SPEC-463's
mutation-coverage test that read the focus on the body after this failure, "a removal whose
list cannot be read again moves no focus and raises no error"; A1 takes its place, keeping its
fixture and its no-rejection check.

```text
A1: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "a removal whose list cannot be read again moves the focus to the alert that says why"
A2: pnpm exec vitest run web/app/src/routes/sign-in-methods.test.ts -t "a list the screen cannot read when it opens moves no focus"
```

```red-first
A2: not red: the base moves no focus at the list's first read; this test guards that read against the new focus move
```

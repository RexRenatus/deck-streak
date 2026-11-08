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
A1: red at 9be31ef4: AssertionError: expected <body><div>…(1)</div></body> to be <p class="mt-6" role="alert"></p> // Object.is equality
A1: green at 3115c2ae
A2: not red: the base moves no focus at the list's first read; this test guards that read against the new focus move
```

## What changed between the red and its green

3115c2ae, the greening commit, changed no test file: it changed only
`web/app/src/lib/settings/SignInMethods.svelte`. 9be31ef4, the red commit, removed SPEC-463's
mutation-coverage test that read the focus on the body after this failure, and A1 took its
place, keeping its fixture and its no-rejection check. No commit after the green changed a line
of A1 or A2.

## MUTATION COVERAGE

StrykerJS over `web/app/src/lib/settings/SignInMethods.svelte` at 3115c2ae, with the repository's
own configuration at a break of 100, read 126 mutants: 126 killed, 0
survived, 0 timed out and 0 with no coverage. No equivalent mutant is recorded.

## The green at the head

The component at the pushed head equals 3115c2ae's:
`git diff --quiet 3115c2ae HEAD -- web/app/src/lib/settings/SignInMethods.svelte` exits 0, and the
lines of A1 and A2 are unchanged since 9be31ef4.

# Red-first record: SPEC-463

The SPEC, ADR-480, the amendments to SPEC-385 and the passkey schematic, and the eight new tests
were committed first (88e0015b), with the component as it was. Each red below is the first failing
line of that criterion's test, run by itself with its `-t` filter at 88e0015b, where exactly these
eight tests failed and the other 683 passed. The eight tests find every button by its place in its
row (A1 to A6) so each is red for the focus and not for the names A7 decides.

```red-first
A1: red at 88e0015b: AssertionError: expected <body><div>…(1)</div></body> to be <button type="button" …(1)></button> // Object.is equality
A2: red at 88e0015b: AssertionError: expected null not to be null
A3: red at 88e0015b: AssertionError: expected <body><div>…(1)</div></body> to be <button type="button" …(1)></button> // Object.is equality
A4: red at 88e0015b: AssertionError: expected <p class="mt-3"></p> to be null
A5: red at 88e0015b: AssertionError: expected <body><div>…(1)</div></body> to be <button type="button" …(1)></button> // Object.is equality
A6: red at 88e0015b: AssertionError: expected <body><div>…(1)</div></body> to be <button type="button" …(1)></button> // Object.is equality
A7: red at 88e0015b: AssertionError: expected [ 'Remove', 'Remove', 'Remove' ] to deeply equal [ …(3) ]
A8: red at 88e0015b: AssertionError: expected [ 'en', false, false, false, false ] to deeply equal [ 'en', true, true, true, true ]
A1: green at 32d0e560
A2: green at 32d0e560
A3: green at 32d0e560
A4: green at 32d0e560
A5: green at 32d0e560
A6: green at 32d0e560
A7: green at 32d0e560
A8: green at 32d0e560
```

## What changed between the reds and their greens

32d0e560 is the commit that changed test files between the reds and their greens, and no assertion
of the eight new tests changed in it. It restated the eight `name: 'Remove'` queries of
`web/app/src/routes/sign-in-methods.test.ts` to the new names (each query matches one exact, longer
name and selects the same button; no assertion was removed) and grew the table of SPEC-385's keys in
`web/app/src/lib/passkey-messages.test.ts` by `methods_remove_passkey`, its count pin moving from 25
to 26. They are held back from the red commit so that it fails exactly the eight new tests.

## MUTATION COVERAGE

StrykerJS over `web/app/src/lib/settings/SignInMethods.svelte` at 32d0e560 left five survivors
(120 killed, 5 survived). Each got a test in its own commit after the greens, none a criterion of
SPEC-463 and none recorded as red:

- ae5c2a25: a key other than Escape leaves an open step as it is.
- 04e12e88: Escape with no step open changes nothing and raises no error.
- 7c6dc536: a removal whose list cannot be read again moves no focus and raises no error.
- 905922c9: removing the first of three passkeys puts the focus on its successor.
- 88922a77: a passkey row's Remove still shows the word Remove.

With them the same run kills all 125 mutants of the file, and no equivalent mutant is recorded.

# Red-first record: SPEC-499

The test was committed first, over #1036's head, with the lock still resolving the registry's
`deepmerge` 4.3.1 for the Vite plugin. At that commit T1, T2 and T6 fail by assertion, for the
advisory's reason, and the controls T3 to T5 pass (`Tests  3 failed | 3 passed (6)`). The copy,
then the override and the lock, followed; at the lock's commit the file passes whole
(`Tests  6 passed (6)`).

Three cases followed the copy, at 70830452: T7, the null-prototype-target case of upstream pull
request 277; T8, the kept key's flags; and T9, an own `__proto__` key on both target and source.
Each was run red first over the registry's 4.3.1: the test file of 70830452, placed in a worktree
of #1036's head, reads `Tests  6 failed | 3 passed (9)`, with T1, T2 and T6 failing as
before and T7, T8 and T9 failing by assertion for the advisory's reason (the result's prototype
set from the key, or the key not kept). At 70830452, on the copy, the file passes whole
(`Tests  9 passed (9)`).

```red-first
A1: red at 8b114f4e: AssertionError: expected 'yes' to be undefined
A1: green at ca11b7fc
A2: red at 8b114f4e: AssertionError: expected 'yes' to be undefined
A2: green at ca11b7fc
A3: not red: a control that the registry's 4.3.1 and the copy both pass
A4: not red: a control that the registry's 4.3.1 and the copy both pass
A5: not red: a control that the registry's 4.3.1 and the copy both pass
A6: red at 8b114f4e: AssertionError: expected undefined to be defined
A6: green at ca11b7fc
A6b: red at 70830452: AssertionError: expected { admin: true } to be { …(12) } // Object.is equality
A6b: green at 70830452
A6c: red at 70830452: AssertionError: expected undefined to deeply equal { value: { admin: true }, …(3) }
A6c: green at 70830452
A6d: red at 70830452: AssertionError: expected { a: 1, b: 2 } to be { …(12) } // Object.is equality
A6d: green at 70830452
```

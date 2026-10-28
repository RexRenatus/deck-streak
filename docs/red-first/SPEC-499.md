# Red-first record: SPEC-499

The test was committed first, over #1036's head, with the lock still resolving the registry's
`deepmerge` 4.3.1 for the Vite plugin. At that commit T1, T2 and T6 fail by assertion, for the
advisory's reason, and the controls T3 to T5 pass (`Tests  3 failed | 3 passed (6)`). The copy,
then the override and the lock, followed; at the lock's commit the file passes whole
(`Tests  6 passed (6)`).

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
```

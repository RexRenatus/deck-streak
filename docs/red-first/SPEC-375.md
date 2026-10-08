# Red-first record: SPEC-375

The ten criteria of SPEC-375 were written against a stub reader whose `judge` returns an empty
report and whose `main` prints an examined count of zero and exits 3. Each red was read in CI's
`hygiene` job on the first push, and each green on the second.

```red-first
A1: red at <C1>: <failure>
A2: red at <C1>: <failure>
A3: red at <C1>: <failure>
A4: red at <C1>: <failure>
A5: red at <C1>: <failure>
A6: red at <C1>: <failure>
A7: red at <C1>: <failure>
A8: red at <C1>: <failure>
A9: red at <C1>: <failure>
A10: red at <C1>: <failure>
```

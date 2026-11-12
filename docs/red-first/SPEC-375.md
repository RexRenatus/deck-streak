# Red-first record: SPEC-375

The ten criteria of SPEC-375 were written against a stub reader whose `judge` returns an empty
report and whose `main` prints an examined count of zero and exits 3. Each red was read in CI's
`hygiene` job on the first push, and each green on the second.

```red-first
A1: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/the-app-campaigns-surfac[72 chars].md']
A2: red at 58a80184: AssertionError: Lists differ: [] != ['S1', 'T1', 'R1', 'I1', 'I2', 'D1', 'E1',[157 chars]'E5']
A3: red at 58a80184: AssertionError: Lists differ: [] != ['the iPhone and iPad client', 'the web cl[85 chars]nes']
A4: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/model.md:17: S1: quote n[47 chars]ere']
A5: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/model.md:17: S1: names n[424 chars]ere']
A6: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/model.md:18: T1: the con[269 chars]:1:']
A7: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/model.md:12: surfaces: s[211 chars]ace']
A8: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/model.md:23: X1: not a S[55 chars] id']
A9: red at 58a80184: AssertionError: Lists differ: [] != ['docs/schematics/notes.md:3: trace: a con[72 chars]ace']
A10: red at 58a80184: AssertionError: Tuples differ: (3, 'examined 0 model(s), 0 surface(s), 0 row(s), 0 citation(s)\n') != (0, 'examined 1 model(s), 1 surface(s), 6 row(s), 13 citation(s)\n')
A1: green at 2ea522eb
A2: green at 2ea522eb
A3: green at 2ea522eb
A4: green at 2ea522eb
A5: green at 2ea522eb
A6: green at 2ea522eb
A7: green at 2ea522eb
A8: green at 2ea522eb
A9: green at 2ea522eb
A10: green at 2ea522eb
```
A2: citations re-derived after dev moved twelve cited lines; read green at the merge ref in CI
A2: citations re-derived again after the release build moved 5 cited lines; read green at the merge ref in CI

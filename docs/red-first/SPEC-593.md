# SPEC-593 red-first record

The web criteria are red at the red commit, which adds their tests with an inert colour-mode seam and the token
file unchanged, and green at the colour-mode commit, after which only documents change. A8 and A9 are `not red`.
Each red line quotes the first assertion line of its criterion's first test, captured when that test ran alone
before the red commit was made. The pull request's run 41802910753 at the red commit failed every test of each
criterion with a red line below by name, each on an assertion, and the reds of the other tests are read there:
`a planted low-contrast Tide value is refused`, `test_a_planted_off_role_class_or_literal_is_refused`,
`a planted system rule that misses a colour role is refused`,
`the stored choice is applied to the root element after mount`,
`a planted apply that ignores the stored choice is refused` and
`test_a_planted_card_without_the_border_is_refused`.

```text
A1: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "the Tide palette compiles to twelve custom properties in both modes"
A2: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "a role outside the six or missing from either Tide mode fails the compile by name"
A3: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every declared Tide pair meets its contrast bar in both modes"
A3: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "a planted low-contrast Tide value is refused"
A4: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every colour role falls back to its Tide role in each mode"
A5: python3 -m unittest discover -s scripts/tests -p test_tide_tokens.py -k test_each_colour_set_holds_its_tokens_values
A6: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_every_screen_colour_class_is_a_declared_role
A6: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_a_planted_off_role_class_or_literal_is_refused
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "the stylesheet paints the system mode first, dark unless the browser reports light"
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "a planted system rule that misses a colour role is refused"
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "the stored choice is applied to the root element after mount"
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "a planted apply that ignores the stored choice is refused"
A8: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page shell carries no script, inline or by URL"
A8: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits WebAssembly compilation and nothing else new"
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every colour token resolves to a Telegram theme variable with a fallback"
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every text and background pair meets AA contrast in both Telegram palettes"
A13: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_every_card_carries_the_border_role
A13: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_a_planted_card_without_the_border_is_refused
```

```red-first
A1: red at e939d81b: AssertionError: --tide-dark-card's token: expected 'undefined' to be '#21282f' // Object.is equality
A1: green at 14ae9e5f
A2: red at e939d81b: AssertionError: dark card: expected [Function] to throw an error
A2: green at 14ae9e5f
A3: red at e939d81b: AssertionError: expected [ …(14) ] to deeply equal []
A3: green at 14ae9e5f
A4: red at e939d81b: AssertionError: color.background's Tide role: expected undefined to be 'background' // Object.is equality
A4: green at 14ae9e5f
A5: red at e939d81b: AssertionError: 6 != 12 : the token file holds six role values in each mode
A5: green at 14ae9e5f
A6: red at e939d81b: AssertionError: Lists differ: ['off-role: web/app/src/lib/ladder/TierAni[293 chars]ole'] != []
A6: green at 14ae9e5f
A7: red at e939d81b: AssertionError: examined 0 colour roles the compile emits under .dark: the population is empty, so nothing was judged: expected 0 to be greater than 0
A7: green at 14ae9e5f
A8: not red: its SPEC-400 tests in csp.test.ts pass at the cut dev and the file is unchanged
A9: not red: it guards the Telegram mapping dev already holds
A13: red at e939d81b: AssertionError: Lists differ: ['card-without-border: web/app/src/lib/lad[1457 chars]und'] != []
A13: green at 14ae9e5f
```

The web criteria were committed with an inert colour-mode seam and the token file unchanged
(e939d81b), and read red by name in the pull request own run. In that commit two SPEC-578 tests
began to plant a role named glow where they planted muted, now a role, and a third began to
write out the twelve properties of the six roles as a literal list. The roles and screens
changed in c85d2df8 and the colour mode in 14ae9e5f, with the tests unchanged, and the run at this
delivery head reads them green. The commits after 14ae9e5f change only documents.

The tests were unchanged after the red apart from three edits in 14ae9e5f: the module specifier the
colour-mode test's after-mount assertion looks for in the root layout, the one line naming the settings screen
in the card health test's list of route files, and the settings screen's entry in the startapp test's list of
screens opened by their paths. Each edited test was replayed red in its final form on a copy of c85d2df8.
The colour-mode replay carried the colour-mode module of 14ae9e5f in place of the inert seam, so that the
test reaches its changed assertion, and it read red there because that copy's root layout does not import
the module.

Mutation coverage after the green commit. The diff-scoped mutation job 126573943000 at 6dd440ed
found 38 mutants of the new web code that no test killed. The tests added in e1799f5d kill each
of them that a test can observe, and 781e2421 records as equivalent the six in colour-mode.ts
that no test can observe, each with the measurement that shows it. These cases answer that job,
not a criterion: none is red first, and none takes a red or green line. So the commits after
14ae9e5f change tests and the miniapp equivalence register as well as documents, and 303fcefe
refilled the miniapp campaign row of SPEC-057 from its battery pair, runs 41851868090 and
41857399766.

```red-first
A10: red at dae3deed: AssertionError: 14 problem(s): ios/App/Sources/AccountView.swift: names none of the six Tide members; ios/App/Sources/AnswerBar.swift: passes .bar, a system style or colour; ios/App/Sources/AnswerBar.
A10: green at 07277f4e
A11: not red: the base holds no colour scheme override, and the census guards the system appearance rule of ADR-616 D2
A12: red at dae3deed: AssertionError: 1 problem(s): ios/App/Sources/TideColours.swift: names no Color( for the catalog set(s) TideAccent, TideBackground, TideBorder, TideCard, TideInk, TideMuted
A12: green at 07277f4e
```

The native criteria were committed with the views unchanged (dae3deed), and read red by name in
the pull request own run. The colour extension and the views changed in 07277f4e, with the tests
unchanged, and the run at this delivery head reads them green. The commit after 07277f4e changes
only documents.

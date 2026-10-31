# SPEC-578 red-first record

A1 to A4 are red at the red commit, which adds the tests alone, and green at the commit that delivers each one
(A1 to A3 at the token file and the compile, A4 at the colour sets). A5 to A7 plant their refusal over a fixture
the test spells, beside a control that reads green, so each is `not red`. A8 is that control, a complete planted set
that reads green. A9 is SPEC-028's pair of kept checks, unchanged. Every line below was measured locally by running the
web suite (`vitest run` over `src/lib/design`) or the census module by its path
(`python3 -B scripts/tests/test_ember_tokens.py -v`) on a tree byte-identical to the commit it names.

```text
A1: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "the Ember palette compiles to six custom properties in both modes"
A2: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "a role missing from either Ember mode fails the compile by name"
A3: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "ink and accent on the background meet AA in both Ember modes"
A4: python3 -m unittest discover -s scripts/tests -p test_ember_tokens.py -k test_each_colour_set_holds_its_tokens_values
```

```red-first
A1: red at 699d8db11c5dd6d71d5f5ceef486567a831dfab5: AssertionError: --ember-dark-background is not declared: expected false to be true
A1: green at 032f48004ca219d0f053fa26648579dc5b18f15f
A2: red at 699d8db11c5dd6d71d5f5ceef486567a831dfab5: AssertionError: expected [Function] to throw an error
A2: green at 032f48004ca219d0f053fa26648579dc5b18f15f
A3: red at 699d8db11c5dd6d71d5f5ceef486567a831dfab5: AssertionError: examined 0 Ember modes
A3: green at 032f48004ca219d0f053fa26648579dc5b18f15f
A4: red at 699d8db11c5dd6d71d5f5ceef486567a831dfab5: AssertionError: examined 0 Ember roles
A4: green at 93dc1ec6aa29f822f4364a06d60d9be0cf1111eb
A5: not red: a planted refusal over a fixture inside the test, written with the census; its red is the planted fixture, refused on every run
A6: not red: a planted refusal over a fixture inside the test, written with the census; its red is the planted fixture, refused on every run
A7: not red: a planted refusal over a fixture inside the test, written with the census; its red is the planted fixture, refused on every run
A8: not red: the planted complete set, the control
A9: not red: a kept check, unchanged
```

## Counts per step

At the red commit the design suite reads 2 passed before the new tests are added, and A1 to A4 each fail for the
reason quoted above while A5 to A8 pass at once. At the token file and the compile, `vitest run src/lib/design` reads
5 passed, and the built stylesheet holds `--ember-dark-background` once. At the colour sets, the design suite reads
5 passed and `test_ember_tokens.py` runs 5 tests, OK; its repository scan examined 19 files and its fixture scan
examined 3. After the last test commit the design suite reads 6 passed and the whole web suite reads 113 files, 700
tests passed.

## The rows

Rows S57801 to S57803 of `scripts/mutation-rows.d/S57800-S57899.json` were each proved by hand: the target's sha256
was recorded, the replacement installed once by script, the killer run by its explicit id, and the target restored by
writing the saved bytes back, with the sha256 equal. S57801 fails with `EmberBackground light entry holds #faf6f2, its
token #faf6f3`. S57802 fails with `EmberInk dark entry holds #fbf6f2, its token #faf6f2`. S57803 fails with
`EmberAccent light entry holds #a54e06, its token #a54d06`.

## StrykerJS over the changed compile

The whole-file run over `web/app/scripts/build-tokens.ts` is the file's baseline: 276 mutants, 145 killed, 112
survived, 19 with no coverage, 0 timeout. Most of those survivors sit in code that existed before this delivery, and
the pull request workflow does not mutate this file. The changed ranges were then measured alone, over the four ranges
this delivery adds. The first ranged run read 35 mutants: 27 killed, 7 survived, 1 with no coverage, 0 timeout. After
one new test in `tokens.test.ts` that asserts the compile's exact refusal messages, the second ranged run read 35
mutants: 34 killed, 1 survived, 0 with no coverage, 0 timeout. The one survivor replaces the `where` template passed
to `node()` at line 160 of the file and is equivalent: `members(roles)` at line 153 runs `node()` on every member
whose name does not start with `$` and throws there first, the three role names hold no `$`, and line 159 refuses an
undefined role, so the call at line 160 never throws and its `where` argument is never read. No exclusion, filter,
register or configuration was changed for it.

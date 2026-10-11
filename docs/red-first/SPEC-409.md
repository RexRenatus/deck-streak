# Red-first record: SPEC-409

The SPEC, ADR-423 and the schematic's section were committed first (3c8a74e6), before any test
or code. The two censuses, A13 and A14, were then committed alone, and the criteria's tests after
them, each before the code that turns it green. Each red below is quoted from the run at the red
commit.

## The fence, line by line

Each line of SPEC-409 section 3's fence runs the test it names, in `web/app` by the pinned tool route.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A13 | `web/app/src/lib/study/update-now.test.ts` "every locale holds the offer words" | added (step 2) |
| 2 | A14 | `web/app/src/lib/study/update-now.test.ts` "the tap is the one new sync call in the study code" | added (step 2) |

## The reds and greens

Each line's command is the criterion's line in SPEC-409 section 3's fence, run at the commit named.

```red-first
A13: red at 3c8a74e6: AssertionError: expected { en: [ …(4) ], es: [ …(4) ], …(5) } to deeply equal { Object (en, es, ...) }: en, es, fr, ja, ko, zh-Hans and zh-Hant each read "lacks study_update_prompt" (update-now.test.ts:76)
A14: red at 3c8a74e6: AssertionError: expected { 'src/lib/study/engine.ts': 1 } to deeply equal { 'src/lib/study/engine.ts': 1, …(1) }: the walk finds engine.ts alone (update-now.test.ts:163)
```

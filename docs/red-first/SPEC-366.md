# Red-first record: SPEC-366

The SPEC and ADR-377 were committed first (5036f988), then the census alone (dada2b70), then every
criterion's test (c874856c), all before the code that turns them green (74f5c99f). Each red below
is quoted from the run of that criterion's fence line at its red commit. The fence runs from the
worktree root, as SPEC-366 section 3 writes it.

## The fence, line by line

Each of the 12 lines of SPEC-366 section 3's fence resolves to one test, named by the `-t` filter.

| # | criterion | test file | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/src/lib/study/answer-buttons.test.ts` | named: body changed |
| 2 | A2 | `web/app/src/lib/study/review.test.ts` | named: body changed |
| 3 | A3 | `web/app/src/lib/study/review.test.ts` | named: body changed |
| 4 | A4 | `web/app/src/lib/remote/keys.test.ts` | named: body changed |
| 5 | A5 | `web/app/src/lib/remote/gamepad.test.ts` | named: body changed |
| 6 | A6 | `web/app/src/lib/remote/gamepad.test.ts` | named: body changed |
| 7 | A7 | `web/app/src/lib/study/input.test.ts` | named: body changed |
| 8 | A8 | `web/app/src/lib/study/mapping-store.test.ts` | added |
| 9 | A9 | `web/app/src/lib/study/mapping-store.test.ts` | named: body changed |
| 10 | A10 | `web/app/src/lib/engine/protocol.test.ts` | named: body changed |
| 11 | A11 | `web/app/src/lib/study/two-grades.test.ts` | added |
| 12 | A12 | `web/app/src/lib/study/two-grades.test.ts` | added |

## The census first

`two-grades.test.ts` was committed alone at dada2b70, with the shipped sources unchanged. A11 reads
`RATING` as four entries and A12 finds Hard and Easy in the locales, so each is red for its own
reason, by assertion, with the behaviour's assertion ahead of its `examined()` call.

```red-first
A1: red at c874856c: AssertionError: expected [ 'Again <1m', 'Hard <6m', …(2) ] to deeply equal [ 'Again <1m', 'Good <10m' ]
A1: green at 74f5c99f
A2: red at c874856c: AssertionError: hard: expected { Object (state, effect) } to strictly equal { …(2) }
A2: green at 74f5c99f
A3: red at c874856c: AssertionError: expected [ 'again', 'hard', 'good', …(3) ] to deeply equal [ 'again', 'good', 'bury', 'flag' ]
A3: green at 74f5c99f
A4: red at c874856c: AssertionError: 2 on the answer side: expected 'hard' to be null
A4: green at 74f5c99f
A5: red at c874856c: AssertionError: expected [ 'easy', 'hard' ] to deeply equal []
A5: green at 74f5c99f
A6: red at c874856c: AssertionError: axis 1 at 1: expected [ 'hard' ] to deeply equal []
A6: green at 74f5c99f
A7: red at c874856c: AssertionError: expected [ 'show-answer', 'good', 'good', …(9) ] to deeply equal [ 'show-answer', 'good', 'good', …(3) ]
A7: green at 74f5c99f
A8: red at c874856c: AssertionError: expected { …(3) } to deeply equal { …(3) }
A8: green at 74f5c99f
A9: red at c874856c: AssertionError: expected [ 'Action', …(9) ] to deeply equal [ 'Action', …(7) ]
A9: green at 74f5c99f
A10: red at c874856c: AssertionError: rate's rating is malformed: expected { Object (request) } to deeply equal { id: 1, …(1) }
A10: green at 74f5c99f
A11: red at dada2b70: AssertionError: expected '{again:1,hard:2,good:3,easy:4}' to be '{again:1,good:3}' // Object.is equality
A11: green at 74f5c99f
A12: red at dada2b70: AssertionError: messages/en.json names a grade the review no longer offers: expected [ 'study_hard', 'study_easy' ] to deeply equal []
A12: green at 74f5c99f
```

Correction (verify round 1): A8's red at c874856c is the deep-equality failure of the loaded mapping against one that holds no Hard or Easy pair: INTENTS at c874856c still named hard and easy, so the stored pairs survived the read (mapping-store.test.ts:152).

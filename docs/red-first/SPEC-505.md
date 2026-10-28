# Red-first record: SPEC-505

The SPEC, ADR-523 and the schematic were installed first (f2ca6134). The tests came next, whole, over stubs that keep
every input and lack the behaviour (9890088f): `REFUSAL_REASONS` and `RefusalReason` exist but every check still
answers `escaped`; `frameCard` composes the document and its host as the frame did; the locale files hold
`study_card_escaped` alone; the review reads the document with no classes and no host; and the iOS words file prints
the error's case and announces nothing. f1743b4d turns them green: each check answers its own reason in a fixed
order, the host answers `host-readback`, the frame and the review's status read the one composition, seven messages
per locale replace the one sentence, and the iOS card face shows and announces DeckStreak's words. Each red below is
quoted from a run of the row's seven unit test files on the tree of 9890088f, each green from a run on the tree of
f1743b4d (both with `pnpm install --offline --frozen-lockfile`, `pnpm --dir web/app run check` and
`pnpm --dir web/app exec vitest run` over the seven files).

```red-first
A1: red at 9890088f: src/lib/card/frame-document.test.ts:256: expected [ 'escaped', 'escaped', …(5) ] to deeply equal [ 'class-list', 'style-closed', …(5) ]; frame-host.test.ts:49: expected { refused: 'escaped' } to deeply equal { refused: 'host-readback' }; card-frame.test.ts:87: class-list: expected 'escaped' to be 'class-list'
A1: green at f1743b4d
A2: red at 9890088f: src/lib/study/review-screen.test.ts:555: en class-list: expected 'escaped' to be 'class-list'; refusal.test.ts:50: expected { …(10) } to deeply equal { …(16) }
A2: green at f1743b4d
A3: red at 9890088f: src/lib/study/review-screen.test.ts:590: class-list: expected '' to be undefined (the status was quiet while the frame was marked, and no reason's message existed)
A3: green at f1743b4d
A4: red at 9890088f: src/lib/study/refusal.test.ts:76: expected { printsTheCase: true, …(2) } to deeply equal { printsTheCase: false, …(2) }; the XCTest half runs only on a Mac's simulators and was not run here
A4: green at f1743b4d (the census); the XCTest half not run
A5: red at 9890088f: src/lib/study/review-screen.test.ts:515: expected 'This card cannot be shown safely here…' to be 'This card cannot be shown safely here…' (the one sentence against the reason's message); review-templates.test.ts:331: the control is refused: expected 'escaped' to be 'style-closed'; review.test.ts:277 and :462: expected [ 'question', true, 'escaped' ] to deeply equal [ 'question', true, 'style-closed' ]
A5: green at f1743b4d
A6: not run: the rendered audit needs browsers whose system libraries this delivery's host lacks; it is owed to a host that has them, red at 9890088f and green at f1743b4d
```

## The runs

- Red at 9890088f: 7 test files failed, 18 tests failed and 56 passed (74). Every failure is an `AssertionError` for
  its criterion's reason; none is a missing import or a type error. `check` read 1 error, in `refusal.test.ts:59`:
  the test's new status names were not yet in `Status`, which the implementation adds.
- Green at f1743b4d: 7 test files passed, 74 tests passed (74); `check` read 0 errors and 0 warnings over 1623 files.
- The whole unit suite at f1743b4d: 112 test files passed, 691 tests passed (691).

## What each pair disclosed

- **The test commit was amended once, before the red recorded here.** A first red run of it (c6726a0a) found
  `review-screen.test.ts` resolving the message files from `import.meta.url`, which is not a file URL under jsdom, so
  that file failed to load rather than failing its assertions; the commit was amended to resolve them from
  `import.meta.dirname`, as `review-templates.test.ts` does, and the red above is the amended commit's.
- **f1743b4d edits one test.** `card-frame.test.ts` imports the reason list inside its new test rather than at the
  top, so the line the threat model cites (`card-frame.test.ts:19`) keeps its number; at 9890088f that citation read
  one line off.
- **The existing refusal tests changed only in the word they expect.** `frame-document.test.ts`, `frame-host.test.ts`,
  `card-frame.test.ts`, `review.test.ts`, `review-screen.test.ts` and `review-templates.test.ts` expect each plant's
  reason where they expected `escaped`, and the review's status words where they expected the one sentence.
- **The host's check is not reachable through the screen.** The screen's titles and the composed document always read
  back, so A2 and A3 plant the six checks the screen reaches, and A1 plants the host's.

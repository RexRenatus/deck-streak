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
A6: red at 9890088f: tests/a11y.spec.ts:354, the review route in both palettes: expect(locator).toHaveAttribute(expected) failed, unexpected value "escaped" (the refused frame's mark, read before the status)
A6: green at 3dcd8322: the review route's two tests pass with axe in its legacy mode, which audits the screen around the refused card's frame and does not enter it; at f1743b4d they were not green: each passed its mark and status checks and then timed out after 30000 ms in `AxeBuilder.analyze` (a11y.spec.ts:357), whose default mode waits on the refused card's sandboxed frame, which runs no script, and never finished
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

## Fix 1: the check order, the closed types and the host's reason

A hand-written population of 87 mutants over the five changed modules (`frame-document.ts`, `frame-host.ts`,
`CardFrame.svelte`, `refusal.ts` and `review.ts`), each run through `pnpm --dir web/app run check` and the twenty
Vitest files that cover them on the tree of 3dcd8322, left 11 alive. One was declared equivalent before the run: the
status composed with the other side's frame title, while no locale's two frame titles hold a character the host's
read-back could change. aa571d9a adds tests that kill nine of the other ten. Each red below is a run of the check and
the twenty files on the tree of aa571d9a with one mutant applied; each test is green on that tree unmutated (20
files, 150 tests, the check 0 errors and 0 warnings).

- **The order of the checks (SPEC-505 R2; ADR-523 D2).** Four mutants swapped two adjacent re-parse checks. One test
  per adjacent pair of the seven checks, "a card that breaks both `<earlier>` and `<later>` is refused by the earlier
  check" in `frame-document.test.ts`, first shows each fault alone refused by its own check, then the card with both
  refused by the earlier. Red under each swap, for example: `both faults: expected 'reparse-element' to be
  'style-closed'`.
- **The closed types (SPEC-505 R1, R5; ADR-523 D4).** Three mutants opened a type: the reason list no longer `as
  const`, the reason type any string, the status type any string; a fourth let a refusal carry a document. "no other
  word is a refusal reason, and a refusal carries no document" in `frame-document.test.ts` and "no other word is a
  status" in `refusal.test.ts` hold `@ts-expect-error` assertions the check reads. Red under each: `Error: Unused
  '@ts-expect-error' directive.`
- **The host's reason in the status (SPEC-505 R3).** A mutant had the status read the frame document alone, skipping
  the host. "a face its host does not read back is refused in the status by the host's reason" in `review.test.ts`
  shows a face holding `&#13;`, whose document is kept and whose host reads the carriage return back as a line feed.
  Red: `expected [ 'question', false, null ] to deeply equal [ 'question', true, 'host-readback' ]`. So the host's
  check is reachable through the review after all, and the note above that the screen never reaches it no longer
  holds.
- **A notice and a refused face at once (SPEC-505 R5; SPEC-371 R9).** The tenth mutant announced a refused face's
  reason over a notice from the learner's last gesture. R5 now states that the notice comes first and that the reason
  stays on the frame's mark and is the status once the next gesture clears the notice; "a notice from the last gesture
  is announced before a refused card's reason, which the mark keeps" in `review-screen.test.ts`, added by d14a1a9a,
  pins it. Red on the tree of d14a1a9a with that mutant applied, after an undo press that found the answer synced:
  `expected [ [ 'card', 'undo-offer' ], …(2) ] to deeply equal [ [ 'card', 'undo-offer' ], …(2) ]`, whose diff reads
  the status as "This card cannot be shown safely here…" where "Your last answer has synced, so it can no longer be
  undone." was expected, and the mark as `style-closed` in both. Green unmutated (20 files, 151 tests).
- **A6's lines above** are quoted from runs of the rendered audit's review route in Chromium, in the pinned
  Playwright image with no network: red at 9890088f (2 failed), not green at f1743b4d (2 failed, each timed out in
  axe's analyze) and green at 3dcd8322 (2 passed).

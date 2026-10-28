# Red-first record: SPEC-506

The SPEC, ADR-524, ADR-361's amendment and the schematic's section 9 were committed first (3f5a7785), before any test
or code. The tests came next, whole, over the shipped speaker (a364f804), and 536e55f1 turns them green. Each red below
is quoted from a run of the fence's test files at a364f804, each green from a run at 536e55f1, both from the
repository root with `pnpm exec vitest run --reporter=verbose` over `voice.test.ts`, `refusal.test.ts` and
`review-screen.test.ts`.

## The fence, line by line

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `web/app/src/lib/study/voice.test.ts` "a voice is local only when the device marks it local" | added |
| 2 | A1 | `web/app/src/lib/study/voice.test.ts` "the picker lists only the local voices of the language" | added |
| 3 | A2 | `web/app/src/lib/study/voice.test.ts` "a clip voice is the stored local choice else the first local voice else none" | added |
| 4 | A2 | `web/app/src/lib/study/voice.test.ts` "a stored remote choice is ignored and kept" | added |
| 5 | A3 | `web/app/src/lib/study/voice.test.ts` "no_local_voice_says_so" | added |
| 6 | A3 | `web/app/src/lib/study/review-screen.test.ts` "the review screen says when the device has no voice for a language" | added |
| 7 | A3 | `web/app/src/lib/study/refusal.test.ts` "the no voice line is in every locale" | added |
| 8 | A4 | `web/app/src/lib/study/voice.test.ts` "remote_voice_never_speaks" | added |
| 9 | A5 | `web/app/src/lib/study/voice.test.ts` "speak_sites_census" | added |
| 10 | A6 | `web/app/src/lib/study/voice.test.ts` "the picker accessibility audit runs axe in both colour schemes" | added; holds `web/app/tests/voice-picker-a11y.spec.ts` |

## What each pair disclosed

- **The test commit was amended twice before the implementation commit, and every red below is from its final
  form.** First, the census and the audit's coverage test found their folders from `import.meta.url`, which is not a
  file URL in the jsdom environment, so `voice.test.ts` did not load; they now use `import.meta.dirname`, as
  `late-line.test.ts` does. Second, the planted speaker overrode `chosen` and `voiceFor` with the shipped rule (the
  stored choice, else none). Under the new speaker a clip with none is not spoken, so that plant passed
  `staysSilent` with no choice. The plant now takes out only the one read of `localService`, so its fallback is the
  language's first voice, the remote one included, and it fails all four checks it is run through.
- **Three shipped tests change their expectations, never their titles** (SPEC-350's A27 names them): "a speech
  clip's voice is the stored choice, else the language's default", "a speech clip is spoken in its language at the
  web's rate" and "the picker stores the choice per language". Where they expected `null`, the language's default,
  they now expect the language's first local voice. Each is red at a364f804 for that reason.
- **Two fence lines are not red at a364f804.** A5's census holds what the shipped tree already holds: two speak calls,
  both through the speaker, and no direct call; its four planted trees are each reported by name. A6's Vitest line
  holds the shape of the Playwright spec, which the test commit adds.
- **A6's browser leg did not run here.** The build host lacks the browsers' system libraries, so no browser starts
  there. `playwright test --list` loads the spec and lists its 2 tests (one per palette) at a364f804, and at
  536e55f1 lists 44 tests in 2 files with the app's audit; its red and green are read by name in CI's end-to-end job.
- **536e55f1 changes no test file.** The fence's test files and the spec are byte-identical at a364f804 and 536e55f1.

## The reds and greens

```red-first
A1: red at a364f804: voice.test.ts "a voice is local only when the device marks it local": AssertionError: expected [ …(4) ] to deeply equal [ { voiceURI: 'local-ja', …(3) } ]
A1: red at a364f804: voice.test.ts "the picker lists only the local voices of the language": AssertionError: expected [ …(2) ] to deeply equal [ { voiceURI: 'local-ja', …(3) } ]
A1: green at 536e55f1
A2: red at a364f804: voice.test.ts "a clip voice is the stored local choice else the first local voice else none": AssertionError: expected null to be { voiceURI: 'local-ja', …(3) } // Object.is equality
A2: red at a364f804: voice.test.ts "a stored remote choice is ignored and kept": AssertionError: expected { voiceURI: 'remote-ja', …(4) } to be { voiceURI: 'local-ja', …(3) } // Object.is equality
A2: red at a364f804: voice.test.ts "a speech clip's voice is the stored choice, else the language's default": AssertionError: expected null to be { voiceURI: 'kyoko', …(3) } // Object.is equality
A2: red at a364f804: voice.test.ts "a speech clip is spoken in its language at the web's rate": AssertionError: expected [ …(2) ] to deeply equal [ …(2) ]
A2: red at a364f804: voice.test.ts "the picker stores the choice per language": AssertionError: expected [ { voiceURI: 'markus', …(3) }, null ] to deeply equal [ { voiceURI: 'markus', …(3) }, …(1) ]
A2: green at 536e55f1
A3: red at a364f804: voice.test.ts "no_local_voice_says_so": AssertionError: expected [ [ '犬', 'ja-JP', 1, null ] ] to deeply equal []
A3: red at a364f804: review-screen.test.ts "the review screen says when the device has no voice for a language": AssertionError: expected [ 'the dog', …(2) ] to deeply equal [ 'the dog', [], …(1) ]
A3: red at a364f804: refusal.test.ts "the no voice line is in every locale": AssertionError: expected undefined to be 'No voice on this device for {language…' // Object.is equality
A3: green at 536e55f1
A4: red at a364f804: voice.test.ts "remote_voice_never_speaks": AssertionError: expected [ [ '犬', 'ja-JP', 1, null ] ] to deeply equal [ [ '犬', 'ja-JP', 1, { …(4) } ] ]
A4: green at 536e55f1
A5: not red at a364f804: the shipped tree holds the census, and each planted tree is reported by name
A5: green at 536e55f1
A6: not red at a364f804: its Vitest line holds the spec the test commit adds; the browser leg is read in CI
A6: green at 536e55f1
```

## The counts

- At a364f804: the three files ran 37 tests, 11 failed and 26 passed; the 11 are the eleven named above.
- At 536e55f1: the three files ran 37 tests, 37 passed. The whole `web/app` suite ran 112 files and 696 tests, all
  passed; svelte-check read 0 errors and 0 warnings over 1619 files; the production build passed.

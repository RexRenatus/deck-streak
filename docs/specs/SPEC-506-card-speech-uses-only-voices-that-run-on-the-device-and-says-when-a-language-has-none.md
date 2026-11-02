# SPEC-506: Card speech uses only voices that run on the device, and says when a language has none

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-524, which amends ADR-361 D8 and D13 (the voice choice) |
| measured at | `d3b09d5eb13862b005b5faca9513b0fd148ed0dc` (every `path:line` below is at it) |
| schematic | `docs/schematics/web-study-screens.md`, section 9 appended (the clip, the choice, a local voice or none); no line above it changes |
| context | `miniapp` (`web/app/src/lib/study`, `web/app/messages`, `web/app/tests`) |
| mutation band | S50600-S50699 (no hand row: section 9) |

## 1. The problem, measured

A speech clip on the web and in the Mini App is spoken by the browser's speech synthesis. The Web Speech API lists
two kinds of voice: `SpeechSynthesisVoice.localService` is true for a voice the device runs itself and false for a
voice "supplied by a remote speech synthesizer service" (the Web Speech API, `SpeechSynthesisVoice`). A remote voice
sends the card's text off the device. DeckStreak's speech is meant to cost nothing and to keep the learner's cards on
the device, so a remote voice must never speak a card.

At the base:

- The `Voice` type is `voiceURI`, `name` and `lang` (`web/app/src/lib/study/voice.ts:11-15`). `git grep -c
  localService` over `web/app/src` finds 0 files.
- The picker lists every device voice whose language subtag matches the clip's, local or remote
  (`voice.ts:43-45`, drawn by `VoicePicker.svelte:42-44`).
- The speaker sets `utterance.voice` to the stored choice, else `null` (`web/app/src/lib/study/speech.ts:45`,
  `voice.ts:47-50`). With `null` the browser picks the language's default voice (`voice.ts:3-4`), which may be a
  remote one.
- So a remote voice can be listed, chosen, stored, or picked as the default, and the card's text then leaves the
  device. Nothing tells the learner when a language has no voice the device runs.
- The page's one speaker is made in `deviceSpeaker` (`speech.ts:65-69`). Under `web/app/src`, outside tests, the
  `.speak(` calls are two: `speech.ts:48` (the synthesis, inside `WebSpeaker`) and `audio.ts:60` (the player asking
  its speaker). No file calls `speechSynthesis.speak` directly.
- The review's status region is the `role="status"` line above the card (`ReviewScreen.svelte:220-222`); its words
  come from `Review.statusLine` (`web/app/src/lib/study/review.ts:263-268`), each refusal's message keyed in
  `refusal.ts` and held in every locale by `refusal.test.ts`.
- The app has 7 locales under `web/app/messages` (`git ls-files web/app/messages | wc -l` prints 7).
- The native app is not affected: it speaks with the system's installed voices, which run on the device.

## 2. Requirements

R1. `Voice` carries `localService: boolean`, as `SpeechSynthesisVoice` does. A voice counts as local only when its
`localService` is exactly `true`; a voice whose field is `false`, missing or any other value is remote.

R2. `VoiceChoices.voices(all, tag)` lists only the local voices of `tag`'s language subtag, in the device's order.
The picker draws "Default voice" first, then exactly that list, so a remote voice is never offered.

R3. `VoiceChoices.voiceFor(all, tag)` is the stored choice for `tag` while the device lists it as a local voice of
that language; else the language's first local voice; else `null`. `VoiceChoices.chosen(all, tag)` is the first part
alone (the stored local choice, or `null`), and the picker shows it chosen, else "Default voice".

R4. A clip whose voice is `null` is not spoken: `WebSpeaker.speak` hands nothing to the synthesis and settles at once,
so the clips after it still play. The speaker records each clip's outcome through the record it was given:
`'local-voice'` for a clip it handed to the synthesis, `'no-local-voice'` for one it did not speak. A browser with no
speech synthesis speaks nothing and records `'no-local-voice'` for each clip.

R5. The review screen gives the device speaker a record that, for `'no-local-voice'`, tells the review machine the
clip's language (`Review.unvoiced`). While no refusal, frame refusal or done deck is announced, the status region then
reads the line `study_no_local_voice` with that language, until the learner's next gesture. Every locale under
`web/app/messages` holds the key with the `{language}` placeholder; English reads exactly "No voice on this device
for {language}." and each other locale's value is not the English one.

R6. A stored choice of a voice the device lists as remote is ignored and kept: it never speaks and the picker does
not show it, and nothing deletes or rewrites it, so it applies again if the device later lists that voice as local.

R7. `remote_voice_never_speaks`: with the device's voices `[remote ja (the browser's default), local ja]` and no
choice, a Japanese clip speaks with the local one. Its planted control, a speaker build that ignores `localService`,
turns it and R4's `no_local_voice_says_so` red.

R8. `speak_sites_census`: under `web/app/src`, outside tests, every `.speak(` call is one of the two the base holds
(the synthesis inside `WebSpeaker`, the player asking its speaker), no file calls `speechSynthesis.speak` itself,
and only `speech.ts` makes a `SpeechSynthesisUtterance`. A planted direct call is reported by name.

R9. The review screen, with a card whose clips speak a language the device has a local voice for and one it has only
a remote voice for, so that the picker and the line both show, has no WCAG 2.2 A or AA violation axe-core can find,
in Telegram's light and dark palettes (`web/app/tests/voice-picker-a11y.spec.ts`).

## 3. Acceptance criteria

| # | contract items | criterion | red at the base, for this reason | test |
|---|---|---|---|---|
| A1 | 1, 2 | Only a voice the device marks local counts, and the picker lists the language's local voices alone | the base lists every voice of the language, the remote one and one with no field included | `voice.test.ts` "a voice is local only when the device marks it local", "the picker lists only the local voices of the language" (added) |
| A2 | 3, 6 | A clip's voice is the stored local choice, else the first local voice, else none; a stored remote choice is ignored and kept | the base returns the stored remote voice, and `null` where a local voice exists | `voice.test.ts` "a clip voice is the stored local choice else the first local voice else none", "a stored remote choice is ignored and kept" (added); the three shipped voice tests, their titles kept and their expectations moved to R3 |
| A3 | 4, 5 | A clip with no local voice is not spoken, its outcome is recorded, the clips after it play, and the status region says so in the app's locale; every locale holds the line | the base speaks the clip with a `null` voice and has no line | `voice.test.ts` "no_local_voice_says_so"; `review-screen.test.ts` "the review screen says when the device has no voice for a language"; `refusal.test.ts` "the no voice line is in every locale" (added) |
| A4 | 7 | `remote_voice_never_speaks`, and its planted control turns it and `no_local_voice_says_so` red | the base speaks with the browser's default, which is the remote voice | `voice.test.ts` "remote_voice_never_speaks" (added) |
| A5 | 8 | Every speak call under `web/app/src` goes through `WebSpeaker`, and a planted direct call is reported | not red: the base holds it (section 1); its planted calls are red by name | `voice.test.ts` "speak_sites_census" (added) |
| A6 | 9 | axe-core finds no violation on the review screen with the picker and the line, in both palettes | the base has no such audit, and its screen would show no line | `web/app/tests/voice-picker-a11y.spec.ts` (Playwright, added), whose coverage `voice.test.ts` "the picker accessibility audit runs axe in both colour schemes" holds (added) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "a voice is local only when the device marks it local"
A1: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "the picker lists only the local voices of the language"
A2: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "a clip voice is the stored local choice else the first local voice else none"
A2: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "a stored remote choice is ignored and kept"
A3: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "no_local_voice_says_so"
A3: pnpm exec vitest run web/app/src/lib/study/review-screen.test.ts -t "the review screen says when the device has no voice for a language"
A3: pnpm exec vitest run web/app/src/lib/study/refusal.test.ts -t "the no voice line is in every locale"
A4: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "remote_voice_never_speaks"
A5: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "speak_sites_census"
A6: pnpm exec vitest run web/app/src/lib/study/voice.test.ts -t "the picker accessibility audit runs axe in both colour schemes"
```

Every expected string is spelt as a literal in the test, in English. The fake devices list voices written in the
test, each with its `localService` as a field. The census of A5 and the locale test of A3 print what they examined
and refuse zero. A4's planted speaker is a `VoiceChoices` that ignores `localService`, run through the same checks as
the real one, each of which must fail on it. A6's browser leg runs the Playwright spec over the production build with
the engine's Worker and the device's speech answered in the page, as the rendered audit (`tests/a11y.spec.ts`) answers
the API; its Vitest line holds the spec's shape, as `a11y-coverage.test.ts` holds that audit's.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-506-card-speech-uses-only-voices-that-run-on-the-device-and-says-when-a-language-has-none.md` | docs | added |
| `docs/decisions/ADR-524-a-speech-clip-speaks-only-with-a-voice-the-device-runs-and-a-language-with-none-says-so.md` | docs | added |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md` | docs | an amendment section appended (D8, D13); no existing line changes |
| `docs/schematics/web-study-screens.md` | docs | section 9 appended; no existing line changes |
| `docs/red-first/SPEC-506.md` | docs | added: the red-first record |
| `changelog.d/card-speech-local-voices-506.md` | docs | added: the changelog fragment |
| `web/app/src/lib/study/voice.ts` | miniapp | changed: `localService`, `voices`, `chosen`, `voiceFor` (R1 to R3, R6) |
| `web/app/src/lib/study/speech.ts` | miniapp | changed: a null voice is not spoken, the outcome record (R4) |
| `web/app/src/lib/study/review.ts` | miniapp | changed: `unvoiced` and the line in `statusLine` (R5) |
| `web/app/src/lib/study/refusal.ts` | miniapp | changed: `noVoiceText` (R5) |
| `web/app/src/lib/study/ReviewScreen.svelte` | miniapp | changed: the speaker's record, and the header comment's SPEC-506 sentence (R5) |
| `web/app/src/lib/study/VoicePicker.svelte` | miniapp | changed: shows the stored local choice (R2, R3) |
| `web/app/messages/en.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/messages/es.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/messages/fr.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/messages/ja.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/messages/ko.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/messages/zh-Hans.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/messages/zh-Hant.json` | miniapp | changed: the key `study_no_local_voice` added (R5) |
| `web/app/src/lib/study/voice.test.ts` | miniapp | changed: A1, A2, A3, A4, A5, A6's coverage; the shipped voice tests' expectations moved to R3 |
| `web/app/src/lib/study/review-screen.test.ts` | miniapp | changed: A3's screen test added |
| `web/app/src/lib/study/refusal.test.ts` | miniapp | changed: A3's locale test added |
| `web/app/tests/voice-picker-a11y.spec.ts` | miniapp | added: A6's browser leg |

No other file changes.

## 5. What this does NOT cover

- The native app: its voices are the system's installed ones, which run on the device.
- The voice a speech tag names: the core's clip does not carry it, and that parity work is `#666`'s (ADR-361 D13).
- A voice download prompt or a link to the device's voice settings: the line names the language, and the device's
  own settings add a voice.

## 6. Risks

- **A device that marks no voice local.** Such a browser now speaks nothing, and says so per language. That is the
  rule: a voice the device does not run sends the card off it. The owner's device check (V1) reads each browser.
- **A learner's stored remote choice.** It is ignored and kept (R6), so nothing the learner wrote is lost.
- **The first local voice is not the browser's default.** When the default is remote, the first local voice of the
  language speaks instead; the picker still lets the learner choose another local one.
- **Drift with open work on the same files.** The build re-measures `review.ts`, `ReviewScreen.svelte`, the seven
  locale files and the schematic at its cut, and takes the schematic's next free section number.

## 7. What only CI or a device proves

| # | what | where |
|---|---|---|
| C1 | The type check passes with the new field, method and record | the pull request's `web` job (svelte-check) |
| C2 | A6's browser leg: axe-core over the review screen with the picker and the line, in both palettes | the end-to-end suite (`test:e2e`), in CI |
| C3 | Every changed production file under `web/app` has its mutants killed or recorded equivalent | `mutation-web`, on the pull request |
| V1 | On a device, a card speaks with a local voice, a language with only a remote voice is silent and the line names it, and the picker lists local voices only | the owner, in an acceptance session |

## 8. Formal model

None. The rule is one filter over the device's voice list and one branch in the speaker, held by A1 to A4 with fake
voice lists; no state crosses a process, and no file a model covers is edited.

## 9. Mutation rows

None. The band `S50600-S50699` is claimed and holds no rows file: the rows table holds Rust, and web code is proved by
StrykerJS on the pull request (C3), never by hand rows.

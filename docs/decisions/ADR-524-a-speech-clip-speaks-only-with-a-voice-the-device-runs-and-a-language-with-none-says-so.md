---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-524: a speech clip speaks only with a voice the device runs, and a language with none says so

Decides SPEC-506. Amends ADR-361 D8 and D13, the voice choice. A speech clip on the web and in the Mini App is spoken
by the browser's speech synthesis, whose voice list mixes voices the device runs with voices a remote service
supplies (`SpeechSynthesisVoice.localService`). A remote voice sends the card's text off the device, so no clip may
speak with one, and the learner is told when a language has no voice that the device runs.

## Context and Problem Statement

ADR-361 D8 and D13 chose the device's stored voice for the clip's language, else the language's default, which the
page asks for with a `null` voice. At the base the picker lists every voice of the language, and the `null` voice
lets the browser choose its default (SPEC-506 section 1), so a remote voice could be listed, chosen, stored or
picked as the default. How does the page keep every clip on a voice the device runs, without deleting what the
learner chose, and say when there is none?

## Decision Drivers

- The card's text stays on the device, and speech costs nothing.
- Nothing the learner stored is deleted or rewritten.
- A silent clip is explained, in the app's language, without blocking the review.
- One place speaks, so a later call cannot go around the rule.

## Considered Options (the alternatives it was chosen against)

- **D1. Filter the voice list by `localService` in `VoiceChoices`.** Chosen: the picker and the speaker read the one
  list, so neither can offer or use a remote voice.
- **D1, rejected: ask for the browser's default with a `null` voice, as before.** The default may be remote, and
  the page cannot see which voice a `null` asks for.
- **D1, rejected: filter by a list of known remote voice names.** Names differ by browser and change; the API's own
  field is the device's statement.
- **D2. A language with no local voice is not spoken, and the clip settles at once with outcome `'no-local-voice'`.**
  Chosen: the clips after it still play, as a failed utterance already settles.
- **D2, rejected: speak with a local voice of another language.** It would read the text in the wrong language.
- **D3. The fallback is the language's first local voice in the device's order.** Chosen: it is a voice the device
  runs, and the picker's "Default voice" now names it. The browser's `default` flag is not used, because the default
  may be the remote voice.
- **D4. A stored remote choice is ignored and kept.** Chosen: nothing the learner wrote is lost, and the choice
  applies again if the device later lists that voice as local. Rejected: deleting it on read, which writes storage
  the learner did not ask to change.
- **D5. The line lives in the review's status region, one key in the 7 locales, until the next gesture.** Chosen: the
  region already announces each refusal through `refusal.ts`, and a refusal, a frame refusal or a done deck outranks
  it. Rejected: a line inside the picker, which a screen reader does not announce when a clip is skipped.
- **D6. A browser with no speech synthesis records `'no-local-voice'` too.** Chosen: it has no voice at all, so the
  same line explains the silence.
- **D7. A census holds every speak call to the two the base has.** Chosen: a direct `speechSynthesis.speak` call
  would skip the rule, and a planted one is reported by name.

## Decision Outcome

Chosen: D1 to D7, as SPEC-506 R1 to R9 state them.

### Consequences

- Good, because no clip can speak with a remote voice, and the learner hears why a clip is silent.
- Good, because the stored choices stay as the learner wrote them.
- Bad, because a browser that marks every voice remote speaks nothing; the line says so per language.

### Confirmation

SPEC-506 A1 to A5 in Vitest, with fake voice lists and a planted speaker that ignores `localService`; A6's axe-core
audit of the review screen with the picker and the line in both palettes; StrykerJS on the pull request.

## More Information

- ADR-361 D8 and D13 are amended by an appended section of ADR-361 that cites this record; the rest of both stands.
- SPEC-350 R16's "else the language's default" now reads, for the web, "else the language's first local voice, else
  none, which is not spoken".

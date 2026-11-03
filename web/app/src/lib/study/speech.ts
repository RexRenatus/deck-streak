// SPEC-350 R16, A27; ADR-361 D8, D13; SPEC-506; ADR-524: the page's speaker. A speech clip is
// spoken by the browser's speech synthesis in the language the core gives it, with a voice the
// device runs itself for that language, at the clip's rate as the web reads rates. A clip with no
// such voice is not spoken: a null voice would let the browser choose its default, which may be a
// remote service's. Each clip's outcome is recorded, so the review can say when a language has no
// voice on this device. A browser with no speech synthesis speaks nothing, and records the same. Every
// clip settles, so the clips after it still play.
import type { Speaker, SpeechClip } from './audio';
import type { Voice, VoiceChoices } from './voice';

/** The native platform's default rate, which the core's clip rate carries (SPEC-348 P3): the
 * engine's speed times 0.5. The web's normal rate is 1, so the web speaks at the clip's rate
 * divided by it, and a card speaks at the speed it speaks on the other clients. */
export const NATIVE_DEFAULT_RATE = 0.5;

/** What became of a speech clip: handed to the synthesis with a local voice, or not spoken because
 * the device has no local voice for its language. */
export type SpeechOutcome = 'local-voice' | 'no-local-voice';

/** Where a speaker records each clip's outcome. */
export type OutcomeRecord = (clip: SpeechClip, outcome: SpeechOutcome) => void;

/** One utterance, as the speaker sets it: `SpeechSynthesisUtterance` is one. */
export interface Utterance {
  lang: string;
  rate: number;
  voice: Voice | null;
  onend: ((event: SpeechSynthesisEvent) => void) | null;
  onerror: ((event: SpeechSynthesisErrorEvent) => void) | null;
}

/** The browser's speech synthesis, as the speaker needs it: `speechSynthesis` is one. */
export interface Synthesis {
  getVoices(): Voice[];
  speak(utterance: Utterance): void;
  cancel(): void;
}

/** A record that keeps nothing. */
const UNRECORDED: OutcomeRecord = () => undefined;

export class WebSpeaker implements Speaker {
  readonly #synthesis: Synthesis;
  readonly #choices: VoiceChoices;
  readonly #utter: (text: string) => Utterance;
  readonly #record: OutcomeRecord;

  constructor(
    synthesis: Synthesis,
    choices: VoiceChoices,
    utter: (text: string) => Utterance,
    record: OutcomeRecord = UNRECORDED
  ) {
    this.#synthesis = synthesis;
    this.#choices = choices;
    this.#utter = utter;
    this.#record = record;
  }

  /** Speaks `clip` with a local voice, and settles once it is spoken or the synthesis gave it up; a
   * clip with no local voice is not spoken, and settles at once. */
  speak(clip: SpeechClip): Promise<void> {
    const voice = this.#choices.voiceFor(this.#synthesis.getVoices(), clip.language);
    if (voice === null) {
      this.#record(clip, 'no-local-voice');
      return Promise.resolve();
    }
    const utterance = this.#utter(clip.text);
    utterance.lang = clip.language;
    utterance.rate = clip.rate / NATIVE_DEFAULT_RATE;
    utterance.voice = voice;
    this.#record(clip, 'local-voice');
    return new Promise((resolve) => {
      utterance.onend = utterance.onerror = () => resolve();
      this.#synthesis.speak(utterance);
    });
  }

  cancel(): void {
    this.#synthesis.cancel();
  }
}

/** The speaker of a browser with no speech synthesis: it has no voice, so no clip is spoken. */
class Silent implements Speaker {
  readonly #record: OutcomeRecord;

  constructor(record: OutcomeRecord) {
    this.#record = record;
  }

  async speak(clip: SpeechClip): Promise<void> {
    this.#record(clip, 'no-local-voice');
  }

  cancel(): void {}
}

/** This device's speaker: its speech synthesis, or a silent one where the browser has none. `record`
 * hears each clip's outcome. */
export function deviceSpeaker(choices: VoiceChoices, record: OutcomeRecord = UNRECORDED): Speaker {
  return 'speechSynthesis' in globalThis
    ? new WebSpeaker(globalThis.speechSynthesis, choices, (text) => new SpeechSynthesisUtterance(text), record)
    : new Silent(record);
}

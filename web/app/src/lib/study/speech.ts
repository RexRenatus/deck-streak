// SPEC-350 R16, A27; ADR-361 D8, D13: the page's speaker. A speech clip is spoken by the browser's
// speech synthesis in the language the core gives it, with the device's voice for that language,
// at the clip's rate as the web reads rates. A browser with no speech synthesis speaks nothing, so
// the clips after a speech clip still play.
import type { Speaker, SpeechClip } from './audio';
import type { Voice, VoiceChoices } from './voice';

/** The native platform's default rate, which the core's clip rate carries (SPEC-348 P3): the
 * engine's speed times 0.5. The web's normal rate is 1, so the web speaks at the clip's rate
 * divided by it, and a card speaks at the speed it speaks on the other clients. */
export const NATIVE_DEFAULT_RATE = 0.5;

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

export class WebSpeaker implements Speaker {
  readonly #synthesis: Synthesis;
  readonly #choices: VoiceChoices;
  readonly #utter: (text: string) => Utterance;

  constructor(synthesis: Synthesis, choices: VoiceChoices, utter: (text: string) => Utterance) {
    this.#synthesis = synthesis;
    this.#choices = choices;
    this.#utter = utter;
  }

  /** Speaks `clip`, and settles once it is spoken or the synthesis gave it up. */
  speak(clip: SpeechClip): Promise<void> {
    const utterance = this.#utter(clip.text);
    utterance.lang = clip.language;
    utterance.rate = clip.rate / NATIVE_DEFAULT_RATE;
    utterance.voice = this.#choices.voiceFor(this.#synthesis.getVoices(), clip.language);
    return new Promise((resolve) => {
      utterance.onend = utterance.onerror = () => resolve();
      this.#synthesis.speak(utterance);
    });
  }

  cancel(): void {
    this.#synthesis.cancel();
  }
}

/** The speaker of a browser with no speech synthesis. */
class Silent implements Speaker {
  async speak(): Promise<void> {}

  cancel(): void {}
}

/** This device's speaker: its speech synthesis, or a silent one where the browser has none. */
export function deviceSpeaker(choices: VoiceChoices): Speaker {
  return 'speechSynthesis' in globalThis
    ? new WebSpeaker(globalThis.speechSynthesis, choices, (text) => new SpeechSynthesisUtterance(text))
    : new Silent();
}

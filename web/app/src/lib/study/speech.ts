// SPEC-350 R16, A27; ADR-361 D8, D13: the page's speaker.
import type { Speaker, SpeechClip } from './audio';
import type { Voice, VoiceChoices } from './voice';

/** The native platform's default rate (SPEC-348 P3). */
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
  constructor(_synthesis: Synthesis, _choices: VoiceChoices, _utter: (text: string) => Utterance) {}

  async speak(_clip: SpeechClip): Promise<void> {}

  cancel(): void {}
}

export function deviceSpeaker(_choices: VoiceChoices): Speaker {
  return new WebSpeaker({ getVoices: () => [], speak: () => undefined, cancel: () => undefined }, _choices, () => ({
    lang: '',
    rate: 1,
    voice: null,
    onend: null,
    onerror: null
  }));
}

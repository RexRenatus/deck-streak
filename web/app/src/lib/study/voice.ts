// SPEC-350 R16, A27; ADR-361 D8, D13: the device's voice choice, per language.
import type { SwitchStorage } from './input';

/** The one local-storage key every language's voice choice is kept under, on this device. */
export const VOICE_CHOICES = 'deck-streak.study.voices';

/** A voice as the device lists it: `SpeechSynthesisVoice` is one. */
export interface Voice {
  voiceURI: string;
  name: string;
  lang: string;
}

export class VoiceChoices {
  constructor(_storage: SwitchStorage | undefined) {}

  voices<V extends Voice>(_all: readonly V[], _tag: string): V[] {
    return [];
  }

  voiceFor<V extends Voice>(_all: readonly V[], _tag: string): V | null {
    return null;
  }

  choose(_tag: string, _voiceURI: string): void {}
}

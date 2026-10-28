// SPEC-350 R16, A27; ADR-361 D8, D13; SPEC-506; ADR-524: the device's voice choice, per language. A
// voice is a fact about one device, so the choice is kept in the browser's local storage under one
// key, one entry per language the core gives a speech clip. Only a voice the device runs itself
// counts: a voice whose `localService` is not true is a remote service's, which would send the card's
// text off the device, so it is never listed, chosen or spoken. A language with no choice, or a
// choice the device no longer lists as a local voice, speaks with the language's first local voice,
// and a language with none speaks with no voice, which the speaker does not speak.
import { Unstored, type SwitchStorage } from './input';

/** The one local-storage key every language's voice choice is kept under, on this device. */
export const VOICE_CHOICES = 'deck-streak.study.voices';

/** A voice as the device lists it: `SpeechSynthesisVoice` is one. */
export interface Voice {
  voiceURI: string;
  name: string;
  lang: string;
  /** Whether the device runs the voice itself; a remote service's voice is false. */
  localService: boolean;
}

/** A tag's language subtag: its first part, whether its parts are joined by a hyphen or, as some
 * devices list their voices, an underscore. */
function language(tag: string): string {
  return tag.split(/[-_]/)[0];
}

/** The stored choices, by language; a value the device cannot read, or none at all, is none. */
function read(text: string): Record<string, unknown> {
  try {
    return Object(JSON.parse(text));
  } catch {
    // a value this page did not write is no choice
    return {};
  }
}

export class VoiceChoices {
  readonly #store: Pick<SwitchStorage, 'setItem'>;
  #chosen: Record<string, unknown>;

  constructor(storage: SwitchStorage | undefined) {
    this.#store = storage ?? new Unstored();
    this.#chosen = read(String(storage?.getItem(VOICE_CHOICES)));
  }

  /** The device's local voices for `tag`'s language, in the device's order: what the picker lists.
   * A voice counts as local only when the device marks it so, exactly. */
  voices<V extends Voice>(all: readonly V[], tag: string): V[] {
    return all.filter((voice) => voice.localService === true && language(voice.lang) === language(tag));
  }

  /** The stored choice for `tag` while the device lists it as a local voice of the language, else
   * none: what the picker shows chosen. A stored choice the device lists as remote is kept, and
   * ignored. */
  chosen<V extends Voice>(all: readonly V[], tag: string): V | null {
    return this.voices(all, tag).find((voice) => voice.voiceURI === this.#chosen[tag]) ?? null;
  }

  /** The voice a clip in `tag` speaks with: the stored local choice, else the language's first local
   * voice, else none. */
  voiceFor<V extends Voice>(all: readonly V[], tag: string): V | null {
    return this.chosen(all, tag) ?? this.voices(all, tag)[0] ?? null;
  }

  /** Stores `voiceURI` as `tag`'s choice; an empty one chooses the language's default. */
  choose(tag: string, voiceURI: string): void {
    this.#chosen = { ...this.#chosen, [tag]: voiceURI };
    try {
      this.#store.setItem(VOICE_CHOICES, JSON.stringify(this.#chosen));
    } catch {
      // storage the browser refuses keeps the choice for this page only
    }
  }
}

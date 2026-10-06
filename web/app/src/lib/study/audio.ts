// SPEC-350 R15, A26; ADR-361 D12: the page's player of a face's clips. It plays them in the order
// the core gave, a sound from a page URL made from its bytes and the type the core named, and a
// speech clip through the speaker. Nothing it plays reaches the card frame, and it decides no type,
// cap or order of its own.
import type { Clip } from '$lib/engine/protocol';

/** A speech clip, as the core writes it. */
export type SpeechClip = Extract<Clip, { kind: 'speech' }>;

/** A sound clip, as the core writes it. */
type SoundClip = Extract<Clip, { kind: 'sound' }>;

/** The page's one audio element, as the player needs it. */
export interface AudioOut {
  src: string;
  play(): Promise<void>;
  pause(): void;
  onended: ((event: Event) => void) | null;
  onerror: ((event: Event) => void) | null;
  onpause: ((event: Event) => void) | null;
}

/** Where a sound's page URL is made and revoked: the browser's `URL`. */
export interface ObjectUrls {
  createObjectURL(blob: Blob): string;
  revokeObjectURL(url: string): void;
}

/** What speaks a speech clip and settles once it is spoken, and stops whatever it is speaking. */
export interface Speaker {
  speak(clip: SpeechClip): Promise<void>;
  cancel(): void;
}

export class Player {
  readonly #audio: AudioOut;
  readonly #urls: ObjectUrls;
  readonly #speaker: Speaker;
  /** The sequence playing now: a sequence that is no longer the last asked stops at its next clip. */
  #turn = 0;

  constructor(audio: AudioOut, urls: ObjectUrls, speaker: Speaker) {
    this.#audio = audio;
    this.#urls = urls;
    this.#speaker = speaker;
  }

  /** Stops whatever plays, then plays `clips` in order. Resolves true once they are played or a
   * later sequence stopped them, and false when the browser blocked a sound: nothing after it plays,
   * and the side's Replay control asks again once the page may play. */
  async play(clips: readonly Clip[]): Promise<boolean> {
    const turn = ++this.#turn;
    this.#audio.pause();
    this.#speaker.cancel();
    try {
      for (let at = 0; at < clips.length && turn === this.#turn; at++) {
        const clip = clips[at];
        if (clip.kind === 'speech') await this.#speaker.speak(clip);
        else await this.#sound(clip);
      }
    } catch {
      return false;
    }
    return true;
  }

  /** Plays one sound from a page URL made from its bytes, until it ends, fails or is paused, and
   * revokes the URL whatever happened. */
  async #sound(clip: SoundClip): Promise<void> {
    const url = this.#urls.createObjectURL(new Blob([clip.bytes as Uint8Array<ArrayBuffer>], { type: clip.type ?? '' }));
    try {
      this.#audio.src = url;
      const ended = new Promise<void>((resolve) => {
        this.#audio.onended = this.#audio.onerror = this.#audio.onpause = () => resolve();
      });
      await this.#audio.play();
      await ended;
    } finally {
      this.#urls.revokeObjectURL(url);
    }
  }
}

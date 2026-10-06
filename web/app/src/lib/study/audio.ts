// SPEC-350 R15, A26; ADR-361 D12: the page's player of a face's clips.
import type { Clip } from '$lib/engine/protocol';

/** A speech clip, as the core writes it. */
export type SpeechClip = Extract<Clip, { kind: 'speech' }>;

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
  constructor(_audio: AudioOut, _urls: ObjectUrls, _speaker: Speaker) {}

  async play(_clips: readonly Clip[]): Promise<boolean> {
    return true;
  }
}

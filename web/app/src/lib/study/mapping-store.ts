// SPEC-350 R18, A30; ADR-361 D15. The remote's mapping on this device: a stub until the store is
// written, which keeps nothing and reads every mode at its default.
import type { Intent } from '$lib/remote/actions';
import type { PadSnapshot } from '$lib/remote/gamepad';
import type { KeyInput } from '$lib/remote/keys';
import { BUTTONS, KEYS, STICK } from '$lib/remote/mapping';
import type { SwitchStorage } from './input';

/** The one local-storage key the mapping is kept under, on this device. */
export const MAPPING = 'deck-streak.study.mapping';

/** The two ways a remote sends: a gamepad's buttons, or a keyboard's keys. */
export type Mode = 'gamepad' | 'keyboard';

/** Every intent a learner maps, in the screen's order. */
export const INTENTS: readonly Intent[] = [];

/** What #663's readers take: each mode's map, as this device keeps it. */
export interface Mapping {
  keys: ReadonlyMap<string, Intent>;
  buttons: ReadonlyMap<number, Intent>;
  stick: typeof STICK;
}

export class MappingStore {
  constructor(storage: SwitchStorage | undefined) {
    void storage;
  }

  get mapping(): Mapping {
    return { keys: KEYS, buttons: BUTTONS, stick: STICK };
  }

  bindKey(intent: Intent, key: string): void {
    void [intent, key];
  }

  bindButton(intent: Intent, button: number): void {
    void [intent, button];
  }

  setStick(on: boolean): void {
    void on;
  }

  restore(mode: Mode): void {
    void mode;
  }
}

export function keyOf(event: KeyInput): string | null {
  void event;
  return null;
}

export function buttonOf(before: PadSnapshot | undefined, now: PadSnapshot): number | null {
  void [before, now];
  return null;
}

// SPEC-350 R18, A30; ADR-361 D15. The remote's mapping on this device. A learner's remote may not
// send what #663's default map reads, so the map of each mode, a gamepad's buttons and a keyboard's
// keys, is kept per device under one local-storage key, and the review's input passes it to #663's
// readers. A mode the learner never changed is not stored, so it keeps its default; a value this
// page cannot read is no mapping; and a storage the browser refuses keeps a change for the page.
import type { Intent } from '$lib/remote/actions';
import type { PadSnapshot } from '$lib/remote/gamepad';
import type { KeyInput } from '$lib/remote/keys';
import { BUTTONS, KEYS, STICK } from '$lib/remote/mapping';
import { Unstored, type SwitchStorage } from './input';

/** The one local-storage key the mapping is kept under, on this device. */
export const MAPPING = 'deck-streak.study.mapping';

/** The two ways a remote sends: a gamepad's buttons, or a keyboard's keys. */
export type Mode = 'gamepad' | 'keyboard';

/** Every intent a learner maps, in the screen's order. */
export const INTENTS: readonly Intent[] = [
  'confirm',
  'again',
  'hard',
  'good',
  'easy',
  'undo',
  'bury',
  'flag',
  'replay'
];

/** What #663's readers take: each mode's map, as this device keeps it. */
export interface Mapping {
  keys: ReadonlyMap<string, Intent>;
  buttons: ReadonlyMap<number, Intent>;
  stick: typeof STICK;
}

/** What the store keeps: the pairs of each mode the learner changed, and the stick turned off. */
interface Stored {
  keyboard?: [string, Intent][];
  gamepad?: [number, Intent][];
  stick?: false;
}

/** The keys a capture never takes: Tab and Escape move focus and cancel, and a modifier alone
 * names no key. */
const UNMAPPABLE: ReadonlySet<string> = new Set(['Tab', 'Escape', 'Shift', 'Control', 'Alt', 'Meta']);

/** What each mode's restore clears: the keys, or the gamepad's buttons and its stick. */
const DEFAULTED: Record<Mode, Stored> = {
  keyboard: { keyboard: undefined },
  gamepad: { gamepad: undefined, stick: undefined }
};

/** `value`'s pairs whose intent the review knows, or none for a mode the learner never changed. */
function pairs<K>(value: Iterable<[K, Intent]> | undefined): [K, Intent][] | undefined {
  return value && [...value].filter(([, intent]) => INTENTS.includes(intent));
}

/** The stored mapping; a value this page cannot read, or none at all, is no mapping. */
function read(text: string): Stored {
  try {
    const stored = Object(JSON.parse(text));
    return {
      keyboard: pairs(stored.keyboard),
      gamepad: pairs(stored.gamepad),
      stick: stored.stick === false ? false : undefined
    };
  } catch {
    // a value this page did not write is no mapping
    return {};
  }
}

/** `map` with `input` firing `intent` alone: the intent's other inputs, and what `input` fired
 * before, are dropped. */
function rebound<K>(map: ReadonlyMap<K, Intent>, intent: Intent, input: K): [K, Intent][] {
  const kept = [...map].filter(([at, fired]) => at !== input && fired !== intent);
  return [...kept, [input, intent]];
}

export class MappingStore {
  readonly #store: Pick<SwitchStorage, 'setItem'>;
  #stored: Stored;

  constructor(storage: SwitchStorage | undefined) {
    this.#store = storage ?? new Unstored();
    this.#stored = read(String(storage?.getItem(MAPPING)));
  }

  /** The maps the readers take: each mode's stored pairs, else its default. */
  get mapping(): Mapping {
    return {
      keys: new Map(this.#stored.keyboard ?? KEYS),
      buttons: new Map(this.#stored.gamepad ?? BUTTONS),
      stick: this.#stored.stick === false ? [] : STICK
    };
  }

  /** Maps `key` to `intent` alone. */
  bindKey(intent: Intent, key: string): void {
    this.#write({ ...this.#stored, keyboard: rebound(this.mapping.keys, intent, key) });
  }

  /** Maps the gamepad's `button` to `intent` alone. */
  bindButton(intent: Intent, button: number): void {
    this.#write({ ...this.#stored, gamepad: rebound(this.mapping.buttons, intent, button) });
  }

  /** Turns the left stick's four grades on or off; on is the default, so it is not stored. */
  setStick(on: boolean): void {
    this.#write({ ...this.#stored, stick: on ? undefined : false });
  }

  /** Returns `mode` to its default: the keys, or the gamepad's buttons and its stick. */
  restore(mode: Mode): void {
    this.#write({ ...this.#stored, ...DEFAULTED[mode] });
  }

  #write(stored: Stored): void {
    this.#stored = stored;
    try {
      this.#store.setItem(MAPPING, JSON.stringify(stored));
    } catch {
      // storage the browser refuses keeps the mapping for this page only
    }
  }
}

/** The key `event` maps, or `null`: a key the review's reader would never read (a repeat, a key
 * typed while composing, a key with a modifier), Tab, Escape or a modifier alone maps nothing. */
export function keyOf(event: KeyInput): string | null {
  const plain = !(event.repeat || event.isComposing || event.altKey || event.ctrlKey || event.metaKey);
  return plain && !UNMAPPABLE.has(event.key) ? event.key : null;
}

/** The first button `now` holds that `before` did not, on a gamepad of the standard mapping, the
 * only one the review's reader fires for; a gamepad's first frame is its baseline. */
export function buttonOf(before: PadSnapshot | undefined, now: PadSnapshot): number | null {
  if (before === undefined || now.mapping !== 'standard') return null;
  const index = now.buttons.findIndex((down, at) => down && !before.buttons[at]);
  return index < 0 ? null : index;
}

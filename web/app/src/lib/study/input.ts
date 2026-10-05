// SPEC-350 R8, A14, A15; ADR-361. The review's input: a key, a gamepad button, the stick and a click
// reach one handler, the review's `act`, through #663's modules unchanged: `readKey` reads a key,
// `GamepadReader` reads each frame's snapshots, and `resolve` reads an intent against the side. A
// switch kept per device turns the single-character keys off (WCAG 2.1.4), and focus returns to the
// review after every action and when a pointer moves it into the card frame.
import { resolve, type Action, type Intent, type Side } from '$lib/remote/actions';
import { GamepadReader, type PadSnapshot } from '$lib/remote/gamepad';
import { readKey, type KeyInput } from '$lib/remote/keys';

/** The one local-storage key the key switch is kept under, on this device. */
export const KEY_SWITCH = 'deck-streak.study.character-keys';

/** The stored value of a switch turned off. Anything else, no value included, reads on. */
const OFF = 'off';

/** Where the switch is kept: `localStorage` is one. */
export interface SwitchStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** This device's storage, or none where the browser refuses to give it. */
export function deviceStorage(): SwitchStorage | undefined {
  try {
    return localStorage;
  } catch {
    // a frame refused its storage reads as a device with none: the switch is the page's alone
  }
  return undefined;
}

/** What the input moves: the review's one handler, the side it shows, and its focus. */
export interface InputTarget {
  act(action: Action): void;
  side(): Side;
  focus(): void;
}

/** A key the switch silences: one character, with no Control or Command (WCAG 2.1.4). */
function silenced(event: KeyInput): boolean {
  return event.key.length === 1 && !event.ctrlKey && !event.metaKey;
}

export class StudyInput {
  readonly #target: InputTarget;
  readonly #store: Pick<SwitchStorage, 'setItem'>;
  readonly #reader = new GamepadReader();
  #characterKeys: boolean;
  /** Whether the page's last key was Tab, which moves focus on purpose. */
  #tabbed = false;

  constructor(target: InputTarget, storage: SwitchStorage | undefined) {
    this.#target = target;
    this.#store = storage ?? { setItem: () => {} };
    this.#characterKeys = storage?.getItem(KEY_SWITCH) !== OFF;
  }

  /** Whether single-character keys fire on this device. */
  get characterKeys(): boolean {
    return this.#characterKeys;
  }

  set characterKeys(on: boolean) {
    this.#characterKeys = on;
    try {
      this.#store.setItem(KEY_SWITCH, on ? 'on' : OFF);
    } catch {
      // storage the browser refuses keeps the switch for this page only
    }
  }

  /** A `keydown` on the page. */
  key(event: KeyInput): void {
    this.#tabbed = event.key === 'Tab';
    if (!this.#characterKeys && silenced(event)) return;
    this.#intent(readKey(event));
  }

  /** One animation frame's gamepad snapshots. */
  pads(snapshots: readonly PadSnapshot[]): void {
    for (const reading of this.#reader.read(snapshots)) {
      for (const intent of reading.fired) this.#intent(intent);
    }
  }

  /** A gamepad was disconnected. */
  forget(index: number): void {
    this.#reader.forget(index);
  }

  /** A click on one of the review's controls, which names its action on either side. */
  click(action: Action): void {
    this.#target.act(action);
    this.#target.focus();
  }

  /** A pointer went down on the page: the focus it moves is not a keyboard's. */
  pointer(): void {
    this.#tabbed = false;
  }

  /** The window lost focus. When the card frame holds it and no Tab moved it, the review takes it
   * back, so the next key still reaches the review (P1). */
  blur(active: unknown, frame: unknown): void {
    if (frame !== null && active === frame && !this.#tabbed) this.#target.focus();
  }

  #intent(intent: Intent | null): void {
    const action = intent && resolve(intent, this.#target.side());
    if (action === null) return;
    this.#target.act(action);
    this.#target.focus();
  }
}

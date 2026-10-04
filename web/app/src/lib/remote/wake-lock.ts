/**
 * The wake lock holder (SPEC-343 R15; the schematic's table): it wants the screen lock exactly
 * while a review is open, a gamepad is connected and the page is visible.
 */

/** The holder's states. */
export type LockState =
  | 'off'
  | 'requesting'
  | 'cancelling'
  | 'held'
  | 'released'
  | 'refused'
  | 'unsupported';

/** What reaches the holder: the condition's rise and fall, and the browser's answers. */
export type LockEvent = 'want' | 'unwant' | 'granted' | 'denied' | 'released';

/** What a transition asks of the browser. */
export type Effect = 'request' | 'release' | 'none';

/** The state `event` moves `state` to, and the effect it asks for. */
export function step(state: LockState, event: LockEvent): { state: LockState; effect: Effect } {
  void event;
  return { state, effect: 'none' };
}

/** A lock the browser granted: `WakeLockSentinel` is one. */
export interface SentinelLike {
  release(): Promise<void>;
  addEventListener(type: 'release', listener: () => void): void;
}

/** The browser's wake lock: `navigator.wakeLock` is one. */
export interface WakeLockLike {
  request(type: 'screen'): Promise<SentinelLike>;
}

/** The three parts of the condition. */
export interface Conditions {
  review: boolean;
  gamepad: boolean;
  visible: boolean;
}

/** Holds the screen lock while the condition holds. */
export class WakeLockHolder {
  #state: LockState;
  #refusal: string | null = null;
  #inFlight: Promise<void> = Promise.resolve();

  constructor(api: WakeLockLike | undefined, onChange: () => void = () => undefined) {
    void onChange;
    this.#state = api === undefined ? 'unsupported' : 'off';
  }

  /** The holder's state. */
  get state(): LockState {
    return this.#state;
  }

  /** The name of the last refusal's error, or `null` before any. */
  get refusal(): string | null {
    return this.#refusal;
  }

  /** Reads the condition's parts. */
  set(conditions: Conditions): void {
    void conditions;
  }

  /** Settles when the request in flight, if any, has been answered and its answer applied. */
  settled(): Promise<void> {
    return this.#inFlight;
  }
}

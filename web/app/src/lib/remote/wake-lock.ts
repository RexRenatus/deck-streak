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

/** One cell of the table: the next state and the effect it asks for. */
interface Cell {
  state: LockState;
  effect: Effect;
}

/** The schematic's table, section 4: every state's answer to every event, thirty-five cells. */
const TABLE: Record<LockState, Record<LockEvent, Cell>> = {
  off: {
    want: { state: 'requesting', effect: 'request' },
    unwant: { state: 'off', effect: 'none' },
    granted: { state: 'off', effect: 'none' },
    denied: { state: 'off', effect: 'none' },
    released: { state: 'off', effect: 'none' }
  },
  requesting: {
    want: { state: 'requesting', effect: 'none' },
    unwant: { state: 'cancelling', effect: 'none' },
    granted: { state: 'held', effect: 'none' },
    denied: { state: 'refused', effect: 'none' },
    released: { state: 'requesting', effect: 'none' }
  },
  cancelling: {
    want: { state: 'requesting', effect: 'none' },
    unwant: { state: 'cancelling', effect: 'none' },
    granted: { state: 'off', effect: 'release' },
    denied: { state: 'off', effect: 'none' },
    released: { state: 'cancelling', effect: 'none' }
  },
  held: {
    want: { state: 'held', effect: 'none' },
    unwant: { state: 'off', effect: 'release' },
    granted: { state: 'held', effect: 'none' },
    denied: { state: 'held', effect: 'none' },
    released: { state: 'released', effect: 'none' }
  },
  released: {
    want: { state: 'requesting', effect: 'request' },
    unwant: { state: 'off', effect: 'none' },
    granted: { state: 'released', effect: 'none' },
    denied: { state: 'released', effect: 'none' },
    released: { state: 'released', effect: 'none' }
  },
  refused: {
    want: { state: 'refused', effect: 'none' },
    unwant: { state: 'off', effect: 'none' },
    granted: { state: 'refused', effect: 'none' },
    denied: { state: 'refused', effect: 'none' },
    released: { state: 'refused', effect: 'none' }
  },
  unsupported: {
    want: { state: 'unsupported', effect: 'none' },
    unwant: { state: 'unsupported', effect: 'none' },
    granted: { state: 'unsupported', effect: 'none' },
    denied: { state: 'unsupported', effect: 'none' },
    released: { state: 'unsupported', effect: 'none' }
  }
};

/** The state `event` moves `state` to, and the effect it asks for. */
export function step(state: LockState, event: LockEvent): { state: LockState; effect: Effect } {
  const cell = TABLE[state][event];
  return { state: cell.state, effect: cell.effect };
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

/** The name a refusal carries: a `DOMException`'s, or the error's own string. */
function nameOf(error: unknown): string {
  return error instanceof Error ? error.name : String(error);
}

/**
 * Holds the screen lock while the condition holds. Every event passes through `step`, so the
 * holder's behaviour is the table's: at most one request is in flight (the state is `requesting`
 * or `cancelling` exactly while it is), a lock granted after the condition fell is released at
 * once, and a release from a lock the holder no longer holds changes nothing. `onChange` is
 * called once per applied event, after its effect.
 */
export class WakeLockHolder {
  readonly #api: WakeLockLike | undefined;
  readonly #onChange: () => void;
  #state: LockState;
  #wanted = false;
  #sentinel: SentinelLike | null = null;
  #refusal: string | null = null;
  #inFlight: Promise<void> = Promise.resolve();

  constructor(api: WakeLockLike | undefined, onChange: () => void = () => undefined) {
    this.#api = api;
    this.#onChange = onChange;
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

  /** Reads the condition's parts; its rise is `want` and its fall `unwant`. */
  set(conditions: Conditions): void {
    const wanted = conditions.review && conditions.gamepad && conditions.visible;
    if (wanted === this.#wanted) {
      return;
    }
    this.#wanted = wanted;
    const effect = this.#move(wanted ? 'want' : 'unwant');
    if (effect === 'request') {
      // The table asks for a request only outside `unsupported`, so the browser has a wake lock.
      this.#inFlight = (this.#api as WakeLockLike).request('screen').then(
        (sentinel) => this.#granted(sentinel),
        (error: unknown) => this.#denied(error)
      );
    } else if (effect === 'release') {
      // `unwant` releases only from `held`, which always has its lock.
      void (this.#sentinel as SentinelLike).release();
      this.#sentinel = null;
    }
    this.#onChange();
  }

  /** Settles when the request in flight, if any, has been answered and its answer applied. */
  settled(): Promise<void> {
    return this.#inFlight;
  }

  /** Moves the holder by `event` through the table, and returns the effect it asks for. */
  #move(event: LockEvent): Effect {
    const next = step(this.#state, event);
    this.#state = next.state;
    return next.effect;
  }

  /** The browser granted the request in flight: hold the lock, or release it if it came late. */
  #granted(sentinel: SentinelLike): void {
    if (this.#move('granted') === 'release') {
      void sentinel.release();
    } else {
      this.#sentinel = sentinel;
      sentinel.addEventListener('release', () => this.#released(sentinel));
    }
    this.#onChange();
  }

  /** The browser refused the request in flight; a refusal the holder still wanted is recorded. */
  #denied(error: unknown): void {
    this.#move('denied');
    if (this.#state === 'refused') {
      this.#refusal = nameOf(error);
    }
    this.#onChange();
  }

  /** The browser released `sentinel`; only the lock the holder holds moves it. */
  #released(sentinel: SentinelLike): void {
    if (sentinel !== this.#sentinel) {
      return;
    }
    this.#sentinel = null;
    this.#move('released');
    this.#onChange();
  }
}

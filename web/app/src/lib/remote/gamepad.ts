/**
 * The gamepad reader (SPEC-343 R12): a snapshot of each connected gamepad every animation frame,
 * read against the last one.
 */
import type { Intent } from './actions';
import { BUTTONS, STICK, STICK_FIRE, STICK_REARM } from './mapping';

/** One gamepad as one frame saw it. */
export interface PadSnapshot {
  index: number;
  id: string;
  mapping: string;
  buttons: readonly boolean[];
  axes: readonly number[];
}

/** What one frame read from one gamepad: its raw state, and the intents it fired. */
export interface PadReading {
  index: number;
  id: string;
  mapping: string;
  pressed: number[];
  axes: number[];
  fired: Intent[];
}

/** What the reader keeps of a gamepad between frames. */
interface Kept {
  buttons: readonly boolean[];
  /** Whether each of the stick's directions, in `STICK`'s order, may fire. */
  armed: boolean[];
}

/** How far `axes` lean in `direction`'s way: positive toward it, negative away. */
function leaning(axes: readonly number[], direction: (typeof STICK)[number]): number {
  return (axes[direction.axis] ?? 0) * direction.sign;
}

/**
 * Reads each frame's snapshots against the last frame's. A gamepad's first snapshot is its
 * baseline and fires nothing; a later one fires each mapped button that rose, and each stick
 * direction that leaned past `STICK_FIRE` while armed. A direction re-arms once it returns below
 * `STICK_REARM`. Only the standard mapping fires; any other is reported raw.
 */
export class GamepadReader {
  #kept = new Map<number, Kept>();

  /** The readings of `snapshots`, one per gamepad. A gamepad they no longer name is forgotten. */
  read(snapshots: readonly PadSnapshot[]): PadReading[] {
    const named = new Set(snapshots.map((snapshot) => snapshot.index));
    for (const index of [...this.#kept.keys()]) {
      if (!named.has(index)) {
        this.#kept.delete(index);
      }
    }
    return snapshots.map((snapshot) => this.#readOne(snapshot));
  }

  /** Forgets the gamepad at `index`, so its next snapshot is a baseline again. */
  forget(index: number): void {
    this.#kept.delete(index);
  }

  #readOne(snapshot: PadSnapshot): PadReading {
    const before = this.#kept.get(snapshot.index);
    const pressed = snapshot.buttons.flatMap((down, index) => (down ? [index] : []));
    const armed = STICK.map((direction, at) => {
      const lean = leaning(snapshot.axes, direction);
      const wasArmed = before === undefined ? lean < STICK_REARM : before.armed[at];
      return wasArmed ? lean <= STICK_FIRE : lean < STICK_REARM;
    });
    const fired: Intent[] = [];
    if (before !== undefined && snapshot.mapping === 'standard') {
      for (const index of pressed) {
        const intent = BUTTONS.get(index);
        if (intent !== undefined && !before.buttons[index]) {
          fired.push(intent);
        }
      }
      STICK.forEach((direction, at) => {
        if (before.armed[at] && !armed[at]) {
          fired.push(direction.intent);
        }
      });
    }
    this.#kept.set(snapshot.index, { buttons: [...snapshot.buttons], armed });
    return {
      index: snapshot.index,
      id: snapshot.id,
      mapping: snapshot.mapping,
      pressed,
      axes: [...snapshot.axes],
      fired
    };
  }
}

/** A browser gamepad's snapshot. */
export function snapshotOf(pad: Gamepad): PadSnapshot {
  return {
    index: pad.index,
    id: pad.id,
    mapping: pad.mapping,
    buttons: pad.buttons.map((button) => button.pressed),
    axes: [...pad.axes]
  };
}

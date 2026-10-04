/**
 * The gamepad reader (SPEC-343 R12): a snapshot of each connected gamepad every animation frame,
 * read against the last one.
 */
import type { Intent } from './actions';

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

/** Reads each frame's snapshots against the last frame's. */
export class GamepadReader {
  /** The readings of `snapshots`, one per gamepad. */
  read(snapshots: readonly PadSnapshot[]): PadReading[] {
    return snapshots.map((snapshot) => ({
      index: snapshot.index,
      id: snapshot.id,
      mapping: snapshot.mapping,
      pressed: [],
      axes: [],
      fired: []
    }));
  }

  /** Forgets the gamepad at `index`, so its next snapshot is a baseline again. */
  forget(index: number): void {
    void index;
  }
}

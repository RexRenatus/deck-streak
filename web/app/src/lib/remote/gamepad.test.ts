import { describe, expect, it } from 'vitest';
import type { Intent } from './actions';
import { GamepadReader, type PadSnapshot } from './gamepad';

// SPEC-343 R12, A23 to A25: the reader keeps each gamepad's last snapshot, fires a mapped button
// on its rising edge only, fires nothing on a gamepad's first snapshot or a mapping that is not
// the standard one, and fires the left stick's directions with hysteresis.

const ID = 'Synthetic remote (STANDARD GAMEPAD)';

function pad(pressed: number[], axes: number[] = [0, 0, 0, 0], mapping = 'standard'): PadSnapshot {
  const buttons = Array.from({ length: 17 }, (_, index) => pressed.includes(index));
  return { index: 0, id: ID, mapping, buttons, axes };
}

function fired(reader: GamepadReader, snapshot: PadSnapshot): Intent[] {
  const readings = reader.read([snapshot]);
  expect(readings).toHaveLength(1);
  return readings[0].fired;
}

/** Section 7's buttons, written from the table rather than read from the code. */
const MAPPED: [number, Intent][] = [
  [0, 'confirm'],
  [1, 'confirm'],
  [14, 'again'],
  [13, 'hard'],
  [15, 'good'],
  [12, 'easy'],
  [4, 'undo'],
  [5, 'bury'],
  [3, 'flag'],
  [2, 'replay']
];

describe('the gamepad reader', () => {
  it('each mapped button fires its action on its rising edge only', () => {
    for (const [button, intent] of MAPPED) {
      const reader = new GamepadReader();
      reader.read([pad([])]);
      expect(fired(reader, pad([button])), `button ${button} pressed`).toEqual([intent]);
      expect(fired(reader, pad([button])), `button ${button} held`).toEqual([]);
      expect(fired(reader, pad([])), `button ${button} released`).toEqual([]);
      expect(fired(reader, pad([button])), `button ${button} pressed again`).toEqual([intent]);
    }
    console.log(`examined ${MAPPED.length} mapped buttons`);

    const first = new GamepadReader();
    expect(fired(first, pad([0, 14])), 'the first snapshot').toEqual([]);
    expect(fired(first, pad([0, 14])), 'still held').toEqual([]);
    expect(fired(first, pad([])), 'released').toEqual([]);
    expect(fired(first, pad([0, 14])), 'pressed after the baseline').toEqual(['confirm', 'again']);

    const unmapped = new GamepadReader();
    unmapped.read([pad([])]);
    expect(fired(unmapped, pad([6, 7, 8, 9, 10, 11, 16]))).toEqual([]);
  });

  it('a non-standard mapping fires nothing and reports its raw indices', () => {
    const standard = new GamepadReader();
    standard.read([pad([])]);
    expect(standard.read([pad([0, 14], [0.9, -0.3, 0, 0])])[0]).toEqual({
      index: 0,
      id: ID,
      mapping: 'standard',
      pressed: [0, 14],
      axes: [0.9, -0.3, 0, 0],
      fired: ['confirm', 'again', 'good']
    });

    const raw = new GamepadReader();
    raw.read([pad([], [0, 0], '')]);
    expect(raw.read([pad([0, 14, 16], [0.9, -0.3], '')])[0]).toEqual({
      index: 0,
      id: ID,
      mapping: '',
      pressed: [0, 14, 16],
      axes: [0.9, -0.3],
      fired: []
    });
  });

  it('the left stick fires past its threshold and re-arms below the lower one', () => {
    const directions: [number, number, Intent][] = [
      [0, -1, 'again'],
      [1, 1, 'hard'],
      [0, 1, 'good'],
      [1, -1, 'easy']
    ];
    for (const [axis, sign, intent] of directions) {
      const lean = (value: number) => {
        const axes = [0, 0, 0, 0];
        axes[axis] = sign * value;
        return pad([], axes);
      };
      const reader = new GamepadReader();
      reader.read([lean(0)]);
      expect(fired(reader, lean(0.5)), `${intent} at the threshold`).toEqual([]);
      expect(fired(reader, lean(0.6)), `${intent} past it`).toEqual([intent]);
      expect(fired(reader, lean(0.9)), `${intent} still leaning`).toEqual([]);
      expect(fired(reader, lean(0.3)), `${intent} between the two`).toEqual([]);
      expect(fired(reader, lean(0.6)), `${intent} not re-armed`).toEqual([]);
      expect(fired(reader, lean(0.25)), `${intent} at the lower one`).toEqual([]);
      expect(fired(reader, lean(0.6)), `${intent} still not re-armed`).toEqual([]);
      expect(fired(reader, lean(0.2)), `${intent} re-armed`).toEqual([]);
      expect(fired(reader, lean(0.6)), `${intent} past it again`).toEqual([intent]);

      const leaning = new GamepadReader();
      expect(fired(leaning, lean(0.9)), `${intent} on the first snapshot`).toEqual([]);
      expect(fired(leaning, lean(0.1)), `${intent} back to the centre`).toEqual([]);
      expect(fired(leaning, lean(0.9)), `${intent} after the baseline`).toEqual([intent]);
    }
    console.log(`examined ${directions.length} stick directions`);
  });

  it('a gamepad that leaves is read again from a new baseline', () => {
    const reader = new GamepadReader();
    reader.read([pad([])]);
    expect(reader.read([])).toEqual([]);
    expect(fired(reader, pad([0])), 'back, pressed: a new baseline').toEqual([]);

    reader.forget(0);
    expect(fired(reader, pad([0, 2])), 'forgotten, then read: a new baseline').toEqual([]);
    expect(fired(reader, pad([0, 2, 3])), 'the next snapshot').toEqual(['flag']);
  });

  // MUTATION COVERAGE: green when written. A stick already off the centre on the first snapshot,
  // at the lower threshold or between the two, is armed only once it returns below the lower one.
  it('a stick off the centre on the first snapshot fires only after it returns', () => {
    const lean = (value: number) => pad([], [value, 0, 0, 0]);
    const STARTS: [number, Intent[]][] = [
      [0.2, ['good']],
      [0.25, []],
      [0.4, []]
    ];
    for (const [start, then] of STARTS) {
      const reader = new GamepadReader();
      expect(fired(reader, lean(start)), `${start} on the first snapshot`).toEqual([]);
      expect(fired(reader, lean(0.6)), `${start}, then past the threshold`).toEqual(then);
    }
    console.log(`examined ${STARTS.length} first snapshots`);
  });
});

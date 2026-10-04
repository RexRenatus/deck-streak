import { describe, expect, it, vi } from 'vitest';
import { resolve, sideAfter, type Action, type Side } from './actions';
import { readKey, type KeyInput } from './keys';

// SPEC-343 R13, R14, A26, A27: Anki's desktop keys fire their actions, a mapped key's default
// action is prevented, and a repeat, a composing key, a key in a control, Alt, an unmapped
// Control or Command, and a grade on the question side fire nothing.

type Init = Partial<Omit<KeyInput, 'preventDefault'>>;

function keydown(key: string, init: Init = {}): KeyInput & { prevented: () => boolean } {
  const preventDefault = vi.fn();
  return {
    key,
    code: `Synthetic${key}`,
    repeat: false,
    isComposing: false,
    altKey: false,
    ctrlKey: false,
    metaKey: false,
    target: null,
    ...init,
    preventDefault,
    prevented: () => preventDefault.mock.calls.length > 0
  };
}

function fire(event: KeyInput, side: Side): Action | null {
  const intent = readKey(event);
  return intent === null ? null : resolve(intent, side);
}

/** Section 7's keys on the answer side, written from the table rather than read from the code. */
const ANSWER_SIDE: [string, Init, Action][] = [
  [' ', {}, 'good'],
  ['Enter', {}, 'good'],
  ['1', {}, 'again'],
  ['2', {}, 'hard'],
  ['3', {}, 'good'],
  ['4', {}, 'easy'],
  ['u', {}, 'undo'],
  ['-', {}, 'bury'],
  ['r', {}, 'replay'],
  ['1', { ctrlKey: true }, 'flag'],
  ['1', { metaKey: true }, 'flag']
];

describe('the key reader', () => {
  it("Anki's desktop keys fire their actions", () => {
    for (const [key, init, action] of ANSWER_SIDE) {
      const event = keydown(key, init);
      expect(fire(event, 'answer'), `${JSON.stringify(key)} ${JSON.stringify(init)}`).toBe(action);
      expect(event.prevented(), `${JSON.stringify(key)} is prevented`).toBe(true);
    }
    console.log(`examined ${ANSWER_SIDE.length} keys`);
    expect(fire(keydown(' '), 'question')).toBe('show-answer');
    expect(fire(keydown('Enter'), 'question')).toBe('show-answer');
  });

  it('a repeat, a control, a modifier or the question side fires nothing', () => {
    const control = (tagName: string, isContentEditable = false) =>
      ({ tagName, isContentEditable }) as unknown as EventTarget;
    const plain = keydown('3', { target: control('DIV') });
    expect(fire(plain, 'answer'), 'the control: a plain key fires').toBe('good');
    expect(plain.prevented()).toBe(true);

    const silenced: [string, Init][] = [
      ['3', { repeat: true }],
      ['3', { isComposing: true }],
      ['3', { target: control('INPUT') }],
      ['3', { target: control('TEXTAREA') }],
      ['3', { target: control('SELECT') }],
      ['3', { target: control('A') }],
      ['3', { target: control('BUTTON') }],
      ['3', { target: control('DIV', true) }],
      ['3', { altKey: true }],
      ['1', { ctrlKey: true, altKey: true }],
      ['u', { ctrlKey: true }],
      ['3', { metaKey: true }],
      ['F5', {}],
      ['x', {}]
    ];
    for (const [key, init] of silenced) {
      const event = keydown(key, init);
      expect(fire(event, 'answer'), `${JSON.stringify(key)} ${JSON.stringify(init)}`).toBeNull();
      expect(event.prevented(), `${JSON.stringify(key)} is not prevented`).toBe(false);
    }
    console.log(`examined ${silenced.length} silenced keys`);

    for (const key of ['1', '2', '3', '4', 'u', '-', 'r']) {
      expect(fire(keydown(key), 'question'), `${key} on the question side`).toBeNull();
    }
    expect(fire(keydown('1', { ctrlKey: true }), 'question')).toBeNull();
  });

  // MUTATION COVERAGE (R14): green when written. Show answer moves the review to the answer side,
  // each grade back to the question side, and every other action, or nothing, leaves the side.
  it('the side moves by the action that fired', () => {
    const MOVES: [Action | null, Side, Side][] = [
      ['show-answer', 'question', 'answer'],
      ['again', 'answer', 'question'],
      ['hard', 'answer', 'question'],
      ['good', 'answer', 'question'],
      ['easy', 'answer', 'question'],
      ['undo', 'answer', 'answer'],
      ['bury', 'answer', 'answer'],
      ['flag', 'answer', 'answer'],
      ['replay', 'answer', 'answer'],
      [null, 'answer', 'answer'],
      [null, 'question', 'question']
    ];
    for (const [action, side, after] of MOVES) {
      expect(sideAfter(action, side), `${String(action)} on the ${side} side`).toBe(after);
    }
    console.log(`examined ${MOVES.length} moves`);
  });
});

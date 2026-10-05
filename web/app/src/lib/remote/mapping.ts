/**
 * The remote's default mapping, SPEC-343 section 7: the W3C Gamepad standard mapping's buttons
 * and Anki's desktop reviewer keys. Every other button, axis and key fires nothing.
 */
import type { Intent } from './actions';

/** The standard mapping's buttons that fire, by index. */
export const BUTTONS: ReadonlyMap<number, Intent> = new Map<number, Intent>([
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
]);

/** How far the left stick must lean to fire, and how near the centre it must return to re-arm. */
export const STICK_FIRE = 0.5;
export const STICK_REARM = 0.25;

/** The left stick's four directions: the axis, the sign that leans that way, and the intent. */
export const STICK: readonly { axis: number; sign: 1 | -1; intent: Intent }[] = [
  { axis: 0, sign: -1, intent: 'again' },
  { axis: 1, sign: 1, intent: 'hard' },
  { axis: 0, sign: 1, intent: 'good' },
  { axis: 1, sign: -1, intent: 'easy' }
];

/** Anki's keys, by `KeyboardEvent.key`, with no modifier. */
export const KEYS: ReadonlyMap<string, Intent> = new Map<string, Intent>([
  [' ', 'confirm'],
  ['Enter', 'confirm'],
  ['1', 'again'],
  ['2', 'hard'],
  ['3', 'good'],
  ['4', 'easy'],
  ['u', 'undo'],
  ['-', 'bury'],
  ['r', 'replay']
]);

/** The flag's key, with Control or Command: the one key a modifier does not silence. */
export const FLAG_KEY = '1';

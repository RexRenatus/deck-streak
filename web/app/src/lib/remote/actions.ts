/**
 * The remote's actions and the review side (SPEC-343 R14, section 7).
 *
 * A remote or a key names an intent; the side decides the action. `confirm` shows the answer on
 * the question side and answers Good on the answer side, so the two face buttons, Space and Enter
 * do what a reviewer's thumb expects on either side. Only show answer fires on the question side,
 * as in Anki.
 */

/** What a button, a stick direction or a key names before the side is applied. */
export type Intent =
  | 'confirm'
  | 'again'
  | 'good'
  | 'undo'
  | 'bury'
  | 'flag'
  | 'replay';

/** What fires: an intent read on a side. */
export type Action = 'show-answer' | Exclude<Intent, 'confirm'>;

/** The review's side: the card's question, or its answer. */
export type Side = 'question' | 'answer';

/** The grades, each of which ends the card and returns the review to the question side. */
const GRADES: readonly Action[] = ['again', 'good'];

/** The action `intent` fires on `side`, or `null` when it fires nothing there. */
export function resolve(intent: Intent, side: Side): Action | null {
  if (side === 'question') {
    return intent === 'confirm' ? 'show-answer' : null;
  }
  return intent === 'confirm' ? 'good' : intent;
}

/** The side after `action` fires on `side`; when nothing fired (`null`), the side stays. */
export function sideAfter(action: Action | null, side: Side): Side {
  if (action === 'show-answer') {
    return 'answer';
  }
  return GRADES.some((grade) => grade === action) ? 'question' : side;
}

/**
 * The key reader (SPEC-343 R13): Anki's desktop reviewer keys, section 7.
 */
import type { Intent } from './actions';
import { FLAG_KEY, KEYS } from './mapping';

/** What the reader reads of a `keydown`: a `KeyboardEvent` is one. */
export interface KeyInput {
  key: string;
  code: string;
  repeat: boolean;
  isComposing: boolean;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  target: EventTarget | null;
  preventDefault(): void;
}

/** The elements whose keys belong to them: a form control, a link or a button. */
const CONTROLS: ReadonlySet<unknown> = new Set(['INPUT', 'TEXTAREA', 'SELECT', 'A', 'BUTTON']);

/** Whether `target` is a control or editable content, whose keys are typed into it. */
function inControl(target: EventTarget | null): boolean {
  const element = target as { tagName?: unknown; isContentEditable?: unknown } | null;
  if (element === null) {
    return false;
  }
  return CONTROLS.has(element.tagName) || element.isContentEditable === true;
}

/** The intent `event`'s key names, before the event's filters. */
function mapped(event: KeyInput): Intent | null {
  if (event.ctrlKey || event.metaKey) {
    return event.key === FLAG_KEY ? 'flag' : null;
  }
  return KEYS.get(event.key) ?? null;
}

/**
 * The intent `event` names, or `null` when it fires nothing: a repeat, a key typed while
 * composing, a key in a control, a key with Alt, and a key with Control or Command other than the
 * flag's. A mapped key's default action is prevented, so Space does not scroll the page.
 */
export function readKey(event: KeyInput): Intent | null {
  if (event.repeat || event.isComposing || event.altKey || inControl(event.target)) {
    return null;
  }
  const intent = mapped(event);
  if (intent !== null) {
    event.preventDefault();
  }
  return intent;
}

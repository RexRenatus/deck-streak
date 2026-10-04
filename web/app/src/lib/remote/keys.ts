/**
 * The key reader (SPEC-343 R13): Anki's desktop reviewer keys, section 7.
 */
import type { Intent } from './actions';

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

/** The intent `event` names, or `null` when it fires nothing. */
export function readKey(event: KeyInput): Intent | null {
  void event;
  return null;
}

/**
 * The harness's event log (SPEC-343 R16): the last 200 events, the newest first.
 */
import type { Action } from './actions';
import type { LockState } from './wake-lock';

/** How many events the log keeps. */
export const LOG_LIMIT = 200;

/** What an event came from. */
export type Source = 'gamepad' | 'key' | 'connection' | 'visibility' | 'review' | 'lock';

/** One event: when, from what, its raw input, the action it fired, and the page's state then. */
export interface LogEntry {
  /** Milliseconds since the page loaded. */
  time: number;
  source: Source;
  raw: string;
  action: Action | null;
  visibility: DocumentVisibilityState;
  lock: LockState;
}

/** `log` with `entry` added as its newest, keeping the last `LOG_LIMIT`. */
export function append(log: readonly LogEntry[], entry: LogEntry): LogEntry[] {
  return [entry, ...log].slice(0, LOG_LIMIT);
}

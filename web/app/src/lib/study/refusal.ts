// SPEC-350 R5, R10, R11; ADR-361. Each status the review announces, by its message. Stub: one
// message for every status.
import { m } from '$lib/paraglide/messages.js';
import type { Status } from './review';

type StatusKey = 'study_refused_engine';

export const STATUS_KEYS: Record<Status, StatusKey> = {
  'bad-request': 'study_refused_engine',
  'collection-busy': 'study_refused_engine',
  'storage-refused': 'study_refused_engine',
  'engine-failed': 'study_refused_engine',
  'not-open': 'study_refused_engine',
  'not-shown': 'study_refused_engine',
  escaped: 'study_refused_engine',
  done: 'study_refused_engine'
};

/** The status's message, in the app's locale. */
export function statusText(status: Status): string {
  return m[STATUS_KEYS[status]]();
}

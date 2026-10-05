// SPEC-350 R5, R10, R11; ADR-361. Each status the review announces, by its message: every refusal
// the engine answers with, the card the frame refused, and the done deck.
import { m } from '$lib/paraglide/messages.js';
import type { Status } from './review';

type StatusKey =
  | 'study_refused_bad_request'
  | 'study_refused_collection_busy'
  | 'study_refused_storage'
  | 'study_refused_engine'
  | 'study_refused_not_open'
  | 'study_refused_not_shown'
  | 'study_card_escaped'
  | 'study_done';

/** Each status's message, by its key in `messages/<locale>.json`. */
export const STATUS_KEYS: Record<Status, StatusKey> = {
  'bad-request': 'study_refused_bad_request',
  'collection-busy': 'study_refused_collection_busy',
  'storage-refused': 'study_refused_storage',
  'engine-failed': 'study_refused_engine',
  'not-open': 'study_refused_not_open',
  'not-shown': 'study_refused_not_shown',
  escaped: 'study_card_escaped',
  done: 'study_done'
};

/** The status's message, in the app's locale. */
export function statusText(status: Status): string {
  return m[STATUS_KEYS[status]]();
}

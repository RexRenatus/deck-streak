// SPEC-350 R5, R10, R11; ADR-361. Each status the review announces, by its message: every refusal
// the engine answers with, the card the frame refused, and the done deck. An undo the engine
// refuses, or does not offer, says why: the answer has synced, or something changed after it
// (SPEC-371 R10). SPEC-505 R5, R6; ADR-523 D3: a card the frame refused is announced by its reason,
// each reason with its own message in every locale.
import { m } from '$lib/paraglide/messages.js';
import type { Status } from './review';

type StatusKey =
  | 'study_refused_bad_request'
  | 'study_refused_collection_busy'
  | 'study_refused_storage'
  | 'study_refused_engine'
  | 'study_refused_not_open'
  | 'study_refused_not_shown'
  | 'study_card_refused_class_list'
  | 'study_card_refused_style_closed'
  | 'study_card_refused_reparse_element'
  | 'study_card_refused_reparse_srcset'
  | 'study_card_refused_reparse_target'
  | 'study_card_refused_body_attributes'
  | 'study_card_refused_host_readback'
  | 'study_done'
  | 'undo_synced'
  | 'undo_gone';

/** Each status's message, by its key in `messages/<locale>.json`. */
export const STATUS_KEYS: Record<Status, StatusKey> = {
  'bad-request': 'study_refused_bad_request',
  'collection-busy': 'study_refused_collection_busy',
  'storage-refused': 'study_refused_storage',
  'engine-failed': 'study_refused_engine',
  'not-open': 'study_refused_not_open',
  'not-shown': 'study_refused_not_shown',
  'undo-synced': 'undo_synced',
  'not-undoable': 'undo_gone',
  'class-list': 'study_card_refused_class_list',
  'style-closed': 'study_card_refused_style_closed',
  'reparse-element': 'study_card_refused_reparse_element',
  'reparse-srcset': 'study_card_refused_reparse_srcset',
  'reparse-target': 'study_card_refused_reparse_target',
  'body-attributes': 'study_card_refused_body_attributes',
  'host-readback': 'study_card_refused_host_readback',
  done: 'study_done'
};

/** The status's message, in the app's locale. */
export function statusText(status: Status): string {
  return m[STATUS_KEYS[status]]();
}

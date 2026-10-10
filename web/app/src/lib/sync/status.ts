// SPEC-377 R8 to R10; ADR-388 D5, D11, D13. The page's sync state: the owner's words for each
// status word, whether a normal sync's answer asks for the choice, and a snapshot's age in the
// page's language. Every value it reads is a reply of the Worker's; the page derives nothing (D5).
import type {
  ChoiceConfirmed,
  ChoiceCounted,
  Direction,
  Required,
  StatusWord,
  Synced,
  Unsynced
} from '$lib/engine/protocol';
import { m } from '$lib/paraglide/messages.js';

/** What the choice screen asks of the engine's client. */
export interface ChoiceClient {
  choiceCount(): Promise<ChoiceCounted>;
  choiceConfirm(direction: Direction): Promise<ChoiceConfirmed>;
  choiceCancel(): Promise<null>;
}

/** What the sync screen asks of the engine's client. */
export interface SyncClient extends ChoiceClient {
  credentialStatus(): Promise<StatusWord>;
  syncLogin(user: string, password: string): Promise<StatusWord>;
  sync(): Promise<Synced>;
  unsynced(): Promise<Unsynced>;
  forgetSync(): Promise<StatusWord>;
}

/** The owner's words for `word` (SPEC-377 R8). */
export function statusText(word: StatusWord): string {
  switch (word) {
    case 'absent':
      return m.sync_status_absent();
    case 'sealed':
      return m.sync_status_sealed();
    case 'held':
      return m.sync_status_held();
    case 'needs-sign-in':
      return m.sync_status_needs_sign_in();
    case 'offline':
      return m.sync_status_offline();
  }
}

/** Whether the store holds no key a sync could send, so the screen offers the sign-in. */
export function signsIn(word: StatusWord): boolean {
  return word === 'absent' || word === 'needs-sign-in';
}

/** Whether a normal sync answered that only a full sync can go on: the choice is offered then. */
export function choosing(required: Required | null): boolean {
  return required === 'full-sync' || required === 'full-upload' || required === 'full-download';
}

/** A snapshot's age, `seconds` old, as the page's language says a past time: in minutes under an
 * hour, in hours under two days, and in days after. */
export function snapshotAge(seconds: number, locale: string): string {
  const format = new Intl.RelativeTimeFormat(locale);
  if (seconds < 3600) return format.format(-Math.max(1, Math.round(seconds / 60)), 'minute');
  if (seconds < 172800) return format.format(-Math.round(seconds / 3600), 'hour');
  return format.format(-Math.round(seconds / 86400), 'day');
}

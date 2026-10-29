// SPEC-516 R2, R6, R7; ADR-534 D1, D5, D6. The data screen's reads: the export's route and file name,
// what the screen asks of the Worker and of the API, the snapshot route's answer as the screen reads
// it, whether Telegram runs on a phone, and the anchor that saves a file.
import type { Answer } from '$lib/api';
import type { BackupsClient } from '$lib/engine/backups';
import type { SnapshotRead } from '$lib/engine/protocol';

/** The owner's export: one route, posted, so no link, preview or prefetch starts a download. */
export const EXPORT_PATH = '/api/data/export';
/** The file the JSON copy is saved as: the name the route's attachment carries. */
export const EXPORT_FILE = 'deck-streak-export.json';
/** The server's backup age: the snapshot route the full sync's choice reads too (SPEC-377 R12). */
export const SNAPSHOT_PATH = '/api/sync/snapshot';

/** What the data screen asks of the engine's client: the backup list and its exports, and back up
 * now, which answers the new backup's id. */
export interface DataClient extends BackupsClient {
  backUpNow(): Promise<string>;
}

/** What the data screen asks of the API: the owner's export, and the server's backup age. */
export interface DataApi {
  dataExport(): Promise<Answer<Blob>>;
  serverBackup(): Promise<Answer<SnapshotRead>>;
}

/** The Telegram clients on a phone, by the platform each reports: the official iOS and Android
 * apps and Telegram X for Android. Each may refuse to save a page's download. */
export const PHONES = ['ios', 'android', 'android_x'] as const;

/** Whether `platform` names a Telegram client on a phone, where the screen names `/export`. */
export function onPhone(platform: string): boolean {
  return (PHONES as readonly string[]).includes(platform);
}

/** The snapshot route's body as the screen reads it: found with a whole, non-negative age in
 * seconds, not found, or unknown; anything else is none, which the screen shows as unknown. */
export function parseServerBackup(body: unknown): SnapshotRead | null {
  if (body === null || typeof body !== 'object' || Array.isArray(body)) return null;
  const { found, age_seconds: age } = body as Record<string, unknown>;
  if (found === null) return { found: null };
  if (found === false) return { found: false };
  if (found === true && Number.isSafeInteger(age) && (age as number) >= 0) {
    return { found: true, age: age as number };
  }
  return null;
}

/** What the screen says of the server's backup: its age when found, none when the server holds
 * none, and unknown for an answer of unknown, a refusal or no answer. */
export type ServerAge = { kind: 'found'; age: number } | { kind: 'none' } | { kind: 'unknown' };

/** The server's backup as the screen says it, from the API's answer. */
export function serverAge(answer: Answer<SnapshotRead>): ServerAge {
  if (answer.kind !== 'ok' || answer.value.found === null) return { kind: 'unknown' };
  return answer.value.found ? { kind: 'found', age: answer.value.age } : { kind: 'none' };
}

/** Saves `blob` as the file `name`. The anchor is only the browser's means of saving: it never joins
 * the page, and its address is revoked once the save has begun. */
export function saveFile(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

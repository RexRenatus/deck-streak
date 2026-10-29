// SPEC-516 R2, R6, R7; ADR-534 D1, D5, D6. The data screen's reads: the export's file name, what
// the screen asks of the Worker and of the API, whether Telegram runs on a phone, and the server's
// backup age as the screen says it.
import type { Answer } from '$lib/api';
import type { BackupsClient } from '$lib/engine/backups';
import type { SnapshotRead } from '$lib/engine/protocol';

/** The owner's export: one route, posted. */
export const EXPORT_PATH = '/api/data/export';
/** The file the JSON copy is saved as. */
export const EXPORT_FILE = '';

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

/** Whether `platform` names a Telegram client on a phone. */
export function onPhone(platform: string): boolean {
  void platform;
  return false;
}

/** Saves `blob` as the file `name`. */
export function saveFile(blob: Blob, name: string): void {
  void blob;
  void name;
}

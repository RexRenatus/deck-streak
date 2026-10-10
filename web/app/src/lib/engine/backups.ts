// The Worker's backups (SPEC-377 R15 to R17; ADR-388 D14, D15, D17, D18): the browser's backups
// and server copies as the web engine lists them after its retention, and one backup's bytes, which
// the Worker posts to the page by transfer. The rule of what is kept is the core's; this module
// only carries the engine's answers.
import type { BackupKind, BackupsListed, Reply } from './protocol';

/** A backup or a removal as the web engine answers it: its kind, and its age in whole seconds. */
interface Listed {
  id: string;
  kind: BackupKind;
  age_seconds: number;
}

/** The web engine's three backup exports, as the module wasm-bindgen writes them. */
export interface BackupsEngine {
  /** Applies the core's retention and answers what it removed, as JSON; refused while a choice's
   * stage is held. */
  retain(): Promise<string>;
  /** The pool's backups and server copies, newest first, as JSON. */
  backups(): Promise<string>;
  /** The bytes of the listed backup `id` names; refused for any id the pool does not list. */
  export_backup(id: string): Promise<Uint8Array>;
}

/** The client's two backup calls, as the sync screen needs them. */
export interface BackupsClient {
  backups(): Promise<BackupsListed>;
  backupExport(backup: string): Promise<Uint8Array<ArrayBuffer>>;
}

/** The Worker's backups over the web engine `engine` answers. */
export class Backups {
  readonly #engine: () => Promise<BackupsEngine>;

  constructor(engine: () => Promise<BackupsEngine>) {
    this.#engine = engine;
  }

  /** Retention first, then the list, so the list never names a backup retention removed. */
  async list(): Promise<BackupsListed> {
    const engine = await this.#engine();
    const removed = JSON.parse(await engine.retain()) as Omit<Listed, 'id'>[];
    const listed = JSON.parse(await engine.backups()) as Listed[];
    return {
      backups: listed.map(({ id, kind, age_seconds }) => ({ id, kind, age: age_seconds })),
      removed: removed.map(({ kind, age_seconds }) => ({ kind, age: age_seconds }))
    };
  }

  /** The bytes of the backup the page named by an id the Worker listed. */
  async export(backup: string): Promise<Uint8Array> {
    return (await this.#engine()).export_backup(backup);
  }
}

/** The objects a reply moves to the page rather than copies: an export's bytes. */
export function transferred(reply: Reply): Transferable[] {
  if (!reply.ok) return [];
  const { value } = reply;
  // a shared buffer cannot be transferred, and is copied like any other value
  return value instanceof Uint8Array && value.buffer instanceof ArrayBuffer ? [value.buffer] : [];
}

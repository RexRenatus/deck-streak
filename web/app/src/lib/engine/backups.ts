// The Worker's backups (SPEC-377 R15 to R17; ADR-388 D14, D15, D17, D18): the browser's backups
// and server copies as the web engine lists them after its retention, and one backup's bytes, which
// the Worker posts to the page by transfer; and back up now's new backup, by its id (SPEC-516 R4).
// The rule of what is kept is the core's; this module only carries the engine's answers.
import type { BackupKind, BackupsListed, Reply } from './protocol';

/** A backup or a removal as the web engine answers it: its kind, and its age in whole seconds. */
interface Listed {
  id: string;
  kind: BackupKind;
  age_seconds: number;
}

/** The web engine's four backup exports, as the module wasm-bindgen writes them. */
export interface BackupsEngine {
  /** Applies the core's retention and answers what it removed, as JSON; refused while a choice's
   * stage is held. */
  retain(): Promise<string>;
  /** The pool's backups and server copies, newest first, as JSON. */
  backups(): Promise<string>;
  /** The bytes of the listed backup `id` names; refused for any id the pool does not list. */
  export_backup(id: string): Promise<Uint8Array>;
  /** Back up now: a new Backup-kind pool file the core fills with the open collection, answered
   * as JSON naming its id; refused while a choice's stage is held (SPEC-516 R4). */
  back_up_now(): Promise<string>;
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

  /** Back up now: the id of the new backup the web engine made through the core (SPEC-516 R4). It
   * neither lists nor runs retention: the page lists again, which runs retention first. */
  async backUpNow(): Promise<string> {
    const made = JSON.parse(await (await this.#engine()).back_up_now()) as { id: string };
    return made.id;
  }
}

/** The objects a reply moves to the page rather than copies: an export's bytes. */
export function transferred(reply: Reply): Transferable[] {
  // a refusal carries no value, and a shared buffer cannot be transferred: both are copied
  return reply.ok && reply.value instanceof Uint8Array && reply.value.buffer instanceof ArrayBuffer
    ? [reply.value.buffer]
    : [];
}

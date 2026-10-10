import { describe, expect, it } from 'vitest';
import { Backups, transferred, type BackupsEngine } from './backups';
import { EngineClient, EngineError } from './client';
import type { BackupsListed, Reply } from './protocol';
import type { EngineModule, SessionDeps } from './session';

/** The Worker's module, loaded inside each test: it starts itself when it loads, so a fault there
 * must fail a test rather than the file's load (as `worker.test.ts` loads it). */
const worker = () => import('./worker');

// SPEC-377 R15 to R17, B5; ADR-388 D14, D17, D18. The Worker lists the browser's backups after the
// core's retention, by kind and age as the web engine answers them, exports one backup's bytes by
// the id the page names, and posts those bytes to the page by transfer, never by copy.

/** The web engine's backup exports, each call recorded, each answer set. */
class FakeEngine implements BackupsEngine {
  calls: string[] = [];
  removed = '[]';
  listed = '[]';
  bytes = new Uint8Array([83, 81, 76, 105, 116, 101]);

  async retain() {
    this.calls.push('retain');
    return this.removed;
  }

  async backups() {
    this.calls.push('backups');
    return this.listed;
  }

  async export_backup(id: string) {
    this.calls.push(`export_backup ${id}`);
    return this.bytes;
  }
}

/** Three backups as the web engine answers them, newest first, and one retention removed. */
function listing(engine: FakeEngine): BackupsListed {
  engine.removed = JSON.stringify([{ kind: 'backup', age_seconds: 400000 }]);
  engine.listed = JSON.stringify([
    { id: 'backup-2', kind: 'backup', age_seconds: 5 },
    { id: 'server-1', kind: 'server', age_seconds: 7200 },
    { id: 'backup-4', kind: 'backup', age_seconds: 90000 }
  ]);
  return {
    backups: [
      { id: 'backup-2', kind: 'backup', age: 5 },
      { id: 'server-1', kind: 'server', age: 7200 },
      { id: 'backup-4', kind: 'backup', age: 90000 }
    ],
    removed: [{ kind: 'backup', age: 400000 }]
  };
}

/** The Worker's scope and the page's port, joined: each post is recorded with its transfer list
 * and delivered to the other side on a later turn. */
class Channel {
  posted: [unknown, Transferable[] | undefined][] = [];
  #worker: ((event: MessageEvent) => void)[] = [];
  #page: ((event: MessageEvent) => void)[] = [];

  /** The Worker's side, as `serve` takes it. */
  readonly scope = {
    postMessage: (message: unknown, transfer?: Transferable[]) => {
      this.posted.push([message, transfer]);
      for (const listener of this.#page) listener(new MessageEvent('message', { data: message, origin: '' }));
    },
    addEventListener: (_type: 'message', listener: (event: MessageEvent) => void) => {
      this.#worker.push(listener);
    }
  };

  /** The page's side, as `EngineClient` takes it. */
  readonly port = {
    postMessage: (message: unknown) => {
      for (const listener of this.#worker) listener(new MessageEvent('message', { data: message, origin: '' }));
    },
    addEventListener: (_type: 'message', listener: (event: MessageEvent) => void) => {
      this.#page.push(listener);
    }
  };
}

/** A session's deps over a module that opens an empty collection, with `backups` as given. */
function deps(backups: SessionDeps['backups']): SessionDeps {
  const module = {
    install_storage: async () => undefined,
    init: () => undefined,
    open: () => JSON.stringify({ existed: true, notes: 0 })
  } as unknown as EngineModule;
  return { lock: async () => 'held', storage: async () => null, load: async () => module, backups };
}

/** A page client over a Worker serving `backups`, its collection opened. */
async function opened(backups: SessionDeps['backups']): Promise<{ client: EngineClient; channel: Channel }> {
  const { serve } = await worker();
  const channel = new Channel();
  serve(channel.scope, deps(backups), 'https://app.example');
  const client = new EngineClient(channel.port, 'https://app.example');
  await client.open();
  return { client, channel };
}

describe("the Worker's backups", () => {
  it('the list is the backups after retention, newest first, by kind and age', async () => {
    const engine = new FakeEngine();
    const expected = listing(engine);
    expect(await new Backups(async () => engine).list()).toEqual(expected);
    // retention runs first, so the list never names a backup retention removed
    expect(engine.calls).toEqual(['retain', 'backups']);
  });

  it('an empty pool lists no backup and removes none', async () => {
    const engine = new FakeEngine();
    expect(await new Backups(async () => engine).list()).toEqual({ backups: [], removed: [] });
    expect(engine.calls).toEqual(['retain', 'backups']);
  });

  it("an export answers the engine's bytes for the id the page named", async () => {
    const engine = new FakeEngine();
    expect(await new Backups(async () => engine).export('server-1')).toEqual(engine.bytes);
    expect(engine.calls).toEqual(['export_backup server-1']);
  });

  it("a reply carrying an export's bytes names their buffer for transfer, and no other reply does", () => {
    const bytes = new Uint8Array([1, 2, 3]);
    expect(transferred({ id: 1, ok: true, value: bytes })).toEqual([bytes.buffer]);
    const others: Reply[] = [
      { id: 2, ok: true, value: { backups: [], removed: [] } },
      { id: 3, ok: true, value: [1, 2, 3] },
      { id: 4, ok: true, value: null },
      { id: 5, ok: true, value: new Uint8Array(new SharedArrayBuffer(3)) },
      { id: 6, ok: false, code: 'engine-failed', message: 'refused' }
    ];
    expect(others.map(transferred)).toEqual([[], [], [], [], []]);
  });

  it('the Worker lists through the session and posts an export by transfer', async () => {
    const engine = new FakeEngine();
    const expected = listing(engine);
    const { client, channel } = await opened(new Backups(async () => engine));
    expect(await client.backups()).toEqual(expected);
    const bytes = await client.backupExport('backup-2');
    expect([...bytes]).toEqual([...engine.bytes]);
    expect(engine.calls).toEqual(['retain', 'backups', 'export_backup backup-2']);
    // the open and the list are copied; the export's bytes move
    expect(channel.posted.map(([, transfer]) => transfer)).toEqual([[], [], [engine.bytes.buffer]]);
  });

  it('a Worker that keeps no backups refuses each operation by name', async () => {
    const { client } = await opened(undefined);
    const refusals = await Promise.all(
      [client.backups(), client.backupExport('backup-1')].map((sent) =>
        sent.then(
          () => null,
          (error: unknown) => (error instanceof EngineError ? [error.code, error.message] : String(error))
        )
      )
    );
    expect(refusals).toEqual([
      ['engine-failed', 'this Worker keeps no backups for backups'],
      ['engine-failed', 'this Worker keeps no backups for backup-export']
    ]);
  });

  it("an engine's refusal of an export answers engine-failed with its reason", async () => {
    const engine = new FakeEngine();
    engine.export_backup = async () => {
      throw new Error('no listed backup is named backup-9');
    };
    const { client } = await opened(new Backups(async () => engine));
    const refused = await client.backupExport('backup-9').then(
      () => null,
      (error: unknown) => (error instanceof EngineError ? [error.code, error.message] : String(error))
    );
    expect(refused).toEqual(['engine-failed', 'no listed backup is named backup-9']);
  });
});

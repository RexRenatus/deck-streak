import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  ABSENT,
  COMPANION_DIRECTORY,
  COMPANION_FILE_CAP,
  COMPANION_ROUTE,
  CheckpointStore,
  IMPORTS,
  STATE_FILE,
  loadCheckpoint,
  loadCompanion,
  purgeCompanion
} from './companion';
import type { Companion, CompanionEngine, CompanionFile, CompanionFolder, LoaderDeps } from './companion';
import { companionModule } from './companion-modules.test.support';
import type { Plant } from './companion-modules.test.support';

// SPEC-511 C8 to C13; ADR-529 D2, D3. The Worker's loader of the owner's scheduling companion: it
// fetches the three names from the owner route, checks each digest, instantiates the module with
// the frozen empty IMPORTS and nothing else, and reads Absent on any failure; Absent answers the
// committed refusal frame. The checkpoint is stored verbatim, and kept only when the web engine's
// load check keeps it. Every expected byte, digest and name is written out or computed here, never
// read from the module under test.

/**
 * The repository's root, found by walking up to its pnpm workspace. A fixed count of `..` would miss
 * it when StrykerJS runs this file from its sandbox, which sits below `web/app/.stryker-tmp`.
 */
function repositoryRoot(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return dir;
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

/** The repository's committed frames (SPEC-501's codec vectors), from the repository's root. */
const FRAMES = join(repositoryRoot(import.meta.dirname), 'formal', 'vectors', 'companion-frames.jsonl');
/** The loader's own source, which the script-import census reads. */
const SOURCE = resolve(import.meta.dirname, 'companion.ts');
/** 16 MiB, written out apart from the module's constant. */
const CAP = 16_777_216;
/** The route's prefix and the three names, written out here. */
const ROUTE = '/api/companion/v1/';
const NAMES = ['manifest.json', 'module.wasm', 'weights.bin'];

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString('hex');
const unhex = (text: string) => new Uint8Array(Buffer.from(text, 'hex'));
/** SHA-256, by node's own hash, apart from the Web Crypto the loader uses. */
const sha256 = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');

/** Each committed frame, by its name. */
function committed(): Map<string, Uint8Array> {
  const lines = readFileSync(FRAMES, 'utf8').split('\n').filter((line) => line.trim() !== '');
  return new Map(
    lines.map((line) => {
      const { name, hex: frame } = JSON.parse(line) as { name: string; hex: string };
      return [name, unhex(frame)];
    })
  );
}

/** Prints how many items a check examined and refuses zero (the tdd pack's examined contract). */
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty`).toBeGreaterThan(0);
  return items;
}

/** The weights the tests serve: distinct bytes, a zero and a byte above 0x7f among them. */
const WEIGHTS = new Uint8Array([0x00, 0x01, 0x7f, 0x80, 0xc3, 0x28, 0xff, 0x10, 0x20]);

/** A manifest naming `module` and `weights` by their digests, as the owner's lane writes it. */
function manifest(module: Uint8Array, weights: Uint8Array): Uint8Array {
  return new TextEncoder().encode(JSON.stringify({ module: sha256(module), weights: sha256(weights) }));
}

/** What the route answers for a name: the file's bytes, or a status with no file. */
type Served = Uint8Array | number;

/** A fake owner route: each call recorded, each name answered as `files` says, any other 404. */
function route(files: Record<string, Served>) {
  const asked: [string, RequestInit][] = [];
  const fetch = async (input: string, init: RequestInit) => {
    asked.push([input, init]);
    const served = files[input.startsWith(ROUTE) ? input.slice(ROUTE.length) : ''];
    if (served === undefined) return new Response(null, { status: 404 });
    if (typeof served === 'number') return new Response(null, { status: served });
    return new Response(served.slice(), { status: 200 });
  };
  return { asked, fetch };
}

/** The loader's dependencies over a route serving `files`, with the scope's own Web Crypto. */
function deps(files: Record<string, Served>, crypto: LoaderDeps['crypto'] = globalThis.crypto) {
  const served = route(files);
  return { served, deps: { fetch: served.fetch, crypto } satisfies LoaderDeps };
}

/** The three files of a companion built from `plant`, served whole under their own manifest. */
function whole(plant: Plant = {}, weights: Uint8Array = WEIGHTS): Record<string, Served> {
  const module = companionModule(plant);
  return { 'manifest.json': manifest(module, weights), 'module.wasm': module, 'weights.bin': weights };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('the companion loader', () => {
  it('a manifest that answers 404 reads Absent, and Absent answers the committed refusal frame to all 9 committed requests', async () => {
    const module = companionModule();
    const good = manifest(module, WEIGHTS);
    const text = (value: unknown) => new TextEncoder().encode(typeof value === 'string' ? value : JSON.stringify(value));
    const digest = sha256(module);
    const cases = examined<[string, Served]>('manifests that read Absent', [
      ['a manifest that answers 404', 404],
      ['a manifest that answers 401', 401],
      ['a manifest that answers 500', 500],
      ['a manifest that is not JSON', text('{"module":')],
      ['a manifest that is an array', text([digest, sha256(WEIGHTS)])],
      ['a manifest with no weights', text({ module: digest })],
      ['a manifest with a third key', text({ module: digest, weights: sha256(WEIGHTS), state: digest })],
      ['a manifest whose digest is upper case', text({ module: digest.toUpperCase(), weights: sha256(WEIGHTS) })],
      ['a manifest whose digest is 63 digits', text({ module: digest.slice(1), weights: sha256(WEIGHTS) })],
      ['a manifest whose digest is a number', text({ module: 7, weights: sha256(WEIGHTS) })]
    ]);
    for (const [what, served] of cases) {
      const { served: log, deps: given } = deps({ 'manifest.json': served, 'module.wasm': module, 'weights.bin': WEIGHTS });
      const companion = await loadCompanion(given);
      expect(companion.present, what).toBe(false);
      expect(log.asked.map(([url]) => url), `${what}: nothing is fetched past the manifest`).toEqual([
        `${ROUTE}manifest.json`
      ]);
    }
    // The control: the same module and weights under the good manifest load.
    const loaded = await loadCompanion(deps({ 'manifest.json': good, 'module.wasm': module, 'weights.bin': WEIGHTS }).deps);
    expect(loaded.present, 'the good manifest loads').toBe(true);

    // Absent's hello is the refusal at epoch 0, and its answer to each committed request is the
    // committed refusal, byte for byte, at the request's epoch, 7.
    const frames = committed();
    expect(hex(ABSENT.hello)).toBe('0e00000001000000860000000001');
    const requests = examined(
      'committed requests',
      [...frames.keys()].filter((name) => name.startsWith('request-'))
    );
    expect(requests.length).toBe(9);
    const refused = hex(frames.get('reply-refused') ?? new Uint8Array());
    expect(refused).toBe('0e00000001000000860700000001');
    for (const name of requests) {
      expect(hex(ABSENT.call(frames.get(name) ?? new Uint8Array())), name).toBe(refused);
    }
  });

  it('a module or weights changed by one byte read Absent; a loader that skips the digest admits it, and the census refuses that loader', async () => {
    const module = companionModule({ marker: 1 });
    const changedModule = companionModule({ marker: 2 });
    const changedWeights = WEIGHTS.slice();
    changedWeights[4] ^= 0x01;
    expect(changedModule.length).toBe(module.length);
    expect([...changedModule].filter((byte, at) => byte !== module[at]).length).toBe(1);
    const good = manifest(module, WEIGHTS);
    const planted = examined<[string, Record<string, Served>]>('one-byte changes', [
      ['the module changed by one byte', { 'manifest.json': good, 'module.wasm': changedModule, 'weights.bin': WEIGHTS }],
      ['the weights changed by one byte', { 'manifest.json': good, 'module.wasm': module, 'weights.bin': changedWeights }]
    ]);
    /** The census: each planted change a loader over `crypto` admits, named. */
    const census = async (crypto: LoaderDeps['crypto']) => {
      const found: string[] = [];
      for (const [what, files] of planted) {
        if ((await loadCompanion(deps(files, crypto).deps)).present) found.push(`${what} is admitted`);
      }
      return found;
    };
    // The intact files load, so the census judges a loader that can admit.
    expect((await loadCompanion(deps({ 'manifest.json': good, 'module.wasm': module, 'weights.bin': WEIGHTS }).deps)).present).toBe(
      true
    );
    expect(await census(globalThis.crypto)).toEqual([]);
    // The control: a loader whose digest answers the manifest's own, whatever it reads, skips the
    // digest; the census refuses it by both changes.
    const original = new Map([
      [module.length, createHash('sha256').update(module).digest()],
      [WEIGHTS.length, createHash('sha256').update(WEIGHTS).digest()]
    ]);
    const skipping = {
      subtle: {
        digest: async (_algorithm: AlgorithmIdentifier, data: BufferSource) => {
          const length = data.byteLength;
          return new Uint8Array(original.get(length) ?? []).buffer;
        }
      }
    } as unknown as LoaderDeps['crypto'];
    expect(await census(skipping)).toEqual(['the module changed by one byte is admitted', 'the weights changed by one byte is admitted']);
  });

  it('a module that imports env.x reads Absent under IMPORTS; a loader that passes a non-empty import object links it, and the census refuses that loader', async () => {
    const planted = examined<[string, Record<string, Served>]>('modules that ask for an import', [
      ['a module that imports env.x', whole({ imports: ['env', 'x'] })],
      ['a module that imports wasi_snapshot_preview1.fd_write', whole({ imports: ['wasi_snapshot_preview1', 'fd_write'] })]
    ]);
    const census = async () => {
      const found: string[] = [];
      for (const [what, files] of planted) {
        if ((await loadCompanion(deps(files).deps)).present) found.push(`${what} links`);
      }
      return found;
    };
    expect(await census()).toEqual([]);
    // The control: the loader, with every instantiation handed a non-empty import object.
    const instantiate = WebAssembly.instantiate.bind(WebAssembly) as (bytes: BufferSource, imports?: WebAssembly.Imports) => Promise<WebAssembly.WebAssemblyInstantiatedSource>;
    const given = { env: { x: () => undefined }, wasi_snapshot_preview1: { fd_write: () => 0 } };
    vi.spyOn(WebAssembly, 'instantiate').mockImplementation(((bytes: BufferSource) => instantiate(bytes, given)) as typeof WebAssembly.instantiate);
    expect(await census()).toEqual(['a module that imports env.x links', 'a module that imports wasi_snapshot_preview1.fd_write links']);
  });

  it('a present module is opened with the weights through alloc, answers through call, is given IMPORTS and nothing else, and only the three names are fetched; the source holds no script import', async () => {
    const instantiate = WebAssembly.instantiate.bind(WebAssembly) as (bytes: BufferSource, imports?: WebAssembly.Imports) => Promise<WebAssembly.WebAssemblyInstantiatedSource>;
    const made: WebAssembly.Instance[] = [];
    const spy = vi.spyOn(WebAssembly, 'instantiate').mockImplementation((async (bytes: BufferSource, imports?: WebAssembly.Imports) => {
      const source = await instantiate(bytes, imports);
      made.push(source.instance);
      return source;
    }) as typeof WebAssembly.instantiate);
    const files = whole();
    const { served, deps: given } = deps(files);
    const companion = await loadCompanion(given);
    expect(companion.present).toBe(true);
    // Only the three names, in order, each with the session's cookie and no cache.
    expect(served.asked).toEqual(NAMES.map((name) => [`${ROUTE}${name}`, { credentials: 'same-origin', cache: 'no-store' }]));
    // IMPORTS and nothing else: one instantiation, given the frozen empty object itself.
    expect(spy).toHaveBeenCalledTimes(1);
    const [bytes, imports] = spy.mock.calls[0] as [Uint8Array, unknown];
    expect(hex(new Uint8Array(bytes))).toBe(hex(files['module.wasm'] as Uint8Array));
    expect(imports).toBe(IMPORTS);
    expect(Object.isFrozen(IMPORTS)).toBe(true);
    expect(Reflect.ownKeys(IMPORTS)).toEqual([]);
    // The weights went in through `alloc` (the module's first buffer, at 16) and `open` answered
    // them back as the hello; both buffers were freed.
    const memory = made[0].exports.memory as WebAssembly.Memory;
    const words = () => new DataView(memory.buffer);
    expect(hex(new Uint8Array(memory.buffer, 16, WEIGHTS.length))).toBe(hex(WEIGHTS));
    expect(hex(companion.hello)).toBe(hex(WEIGHTS));
    expect(words().getUint32(0, true), 'buffers freed after open').toBe(2);
    // Every later frame goes through `call` the same way.
    const frame = committed().get('request-preview') ?? new Uint8Array();
    expect(hex(companion.call(frame))).toBe(hex(frame));
    expect(words().getUint32(0, true), 'buffers freed after call').toBe(4);
    // The census of script imports: the loader's source holds none, and a planted one is refused.
    const imported = (source: string) => ['import(', 'importScripts('].filter((call) => source.includes(call));
    const source = readFileSync(SOURCE, 'utf8');
    expect(imported(source)).toEqual([]);
    expect(imported(`${source}\nconst bindings = import('${ROUTE}bindings.js');\n`)).toEqual(['import(']);
    expect(imported(`${source}\nimportScripts('${ROUTE}bindings.js');\n`)).toEqual(['importScripts(']);
  });

  it('a missing export, a trapping or out-of-bounds open and a file over the cap read Absent, and a trapping call answers the refusal', async () => {
    expect(COMPANION_FILE_CAP).toBe(CAP);
    expect(COMPANION_ROUTE).toBe(ROUTE);
    const cases = examined<[string, Record<string, Served>]>('companions that read Absent', [
      ['no memory export', whole({ drop: 'memory' })],
      ['no alloc export', whole({ drop: 'alloc' })],
      ['no free export', whole({ drop: 'free' })],
      ['no open export', whole({ drop: 'open' })],
      ['no call export', whole({ drop: 'call' })],
      ['an open that traps', whole({ open: 'trap' })],
      ['an open that answers past its memory', whole({ open: 'out-of-bounds' })],
      ['a module one byte over the cap', whole({ padTo: CAP + 1 })],
      ['weights one byte over the cap', whole({}, new Uint8Array(CAP + 1).fill(3))],
      ['a module that is no module', { ...whole(), 'module.wasm': new Uint8Array([1, 2, 3]), 'manifest.json': manifest(new Uint8Array([1, 2, 3]), WEIGHTS) }]
    ]);
    for (const [what, files] of cases) {
      expect((await loadCompanion(deps(files).deps)).present, what).toBe(false);
    }
    // A module of exactly the cap loads.
    expect((await loadCompanion(deps(whole({ padTo: CAP })).deps)).present, 'a module of the cap').toBe(true);
    // A present companion whose call traps answers the committed refusal at the request's epoch.
    const trapping = await loadCompanion(deps(whole({ call: 'trap' })).deps);
    expect(trapping.present).toBe(true);
    const frames = committed();
    expect(hex(trapping.call(frames.get('request-preview') ?? new Uint8Array()))).toBe(
      hex(frames.get('reply-refused') ?? new Uint8Array())
    );
  });
});

/** An in-memory origin private file system: folders of folders and files, as the store reads it. */
class FakeFolder implements CompanionFolder {
  folders = new Map<string, FakeFolder>();
  files = new Map<string, Uint8Array>();

  async getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<FakeFolder> {
    let folder = this.folders.get(name);
    if (folder === undefined) {
      if (options?.create !== true) throw new DOMException(`no folder ${name}`, 'NotFoundError');
      folder = new FakeFolder();
      this.folders.set(name, folder);
    }
    return folder;
  }

  async getFileHandle(name: string, options?: { create?: boolean }): Promise<CompanionFile> {
    if (!this.files.has(name)) {
      if (options?.create !== true) throw new DOMException(`no file ${name}`, 'NotFoundError');
      this.files.set(name, new Uint8Array());
    }
    return {
      getFile: async () => new Blob([(this.files.get(name) ?? new Uint8Array()).slice()]),
      createWritable: async () => {
        let written = new Uint8Array();
        return {
          write: async (data: Uint8Array) => {
            written = new Uint8Array([...written, ...data]);
          },
          close: async () => {
            this.files.set(name, written);
          }
        };
      }
    };
  }

  async removeEntry(name: string, options?: { recursive?: boolean }): Promise<void> {
    const folder = this.folders.get(name);
    if (folder !== undefined) {
      if (options?.recursive !== true && (folder.files.size > 0 || folder.folders.size > 0)) {
        throw new DOMException(`${name} is not empty`, 'InvalidModificationError');
      }
      this.folders.delete(name);
      return;
    }
    if (!this.files.delete(name)) throw new DOMException(`no entry ${name}`, 'NotFoundError');
  }
}

/** The web engine's companion exports, each call recorded, the load check answering `verdict`. */
class FakeEngine implements CompanionEngine {
  calls: string[] = [];
  verdict = 0;
  restoreAnswer: Uint8Array | null = null;
  checked: unknown[] = [];

  companion_decide(): Uint8Array {
    this.calls.push('decide');
    return new Uint8Array([1, 3, 0xff]);
  }

  companion_restore(epoch: number, checkpoint: Uint8Array): Uint8Array {
    this.calls.push(`restore ${epoch} ${hex(checkpoint)}`);
    return this.restoreAnswer ?? new Uint8Array([0xaa, ...checkpoint]);
  }

  companion_load_check(
    hello: Uint8Array,
    checkpoint: Uint8Array,
    ack: Uint8Array,
    headNamed: boolean,
    indexCount: bigint | undefined,
    indexHead: bigint | undefined
  ): number {
    this.calls.push('load_check');
    this.checked = [hex(hello), hex(checkpoint), hex(ack), headNamed, indexCount, indexHead];
    return this.verdict;
  }

  companion_purge(epoch: number): Uint8Array {
    this.calls.push(`purge ${epoch}`);
    return new Uint8Array([0xbb, epoch]);
  }
}

/** A companion that records each frame it is sent and answers each with one ack frame. */
function recording(): Companion & { sent: string[] } {
  const sent: string[] = [];
  return {
    present: true,
    hello: new Uint8Array([0x48, 0x49]),
    sent,
    call(frame: Uint8Array) {
      sent.push(hex(frame));
      return new Uint8Array([0xcc]);
    }
  };
}

/** A checkpoint frame as the companion answers it: opaque here, a zero and a high byte among it. */
const CHECKPOINT = new Uint8Array([0x37, 0x00, 0x01, 0x84, 0x00, 0xfe, 0xff, 0x7f]);

describe('the checkpoint store', () => {
  it('a checkpoint is stored and read back verbatim, and purge sends Purge and removes the directory', async () => {
    const root = new FakeFolder();
    const store = new CheckpointStore({ getDirectory: async () => root });
    expect(COMPANION_DIRECTORY).toBe('deck-streak-companion');
    expect(STATE_FILE).toBe('state.bin');
    expect(await store.load(), 'nothing stored yet').toBeNull();
    expect(await store.save(CHECKPOINT)).toBe(true);
    const kept = root.folders.get('deck-streak-companion')?.files.get('state.bin');
    expect(hex(kept ?? new Uint8Array()), 'state.bin holds the frame verbatim').toBe(hex(CHECKPOINT));
    expect(hex((await store.load()) ?? new Uint8Array()), 'read back byte-equal').toBe(hex(CHECKPOINT));
    // A second checkpoint replaces the first whole.
    const next = new Uint8Array([0x01, 0x02]);
    expect(await store.save(next)).toBe(true);
    expect(hex((await store.load()) ?? new Uint8Array())).toBe(hex(next));
    // Purge sends the companion the Purge frame the web engine builds and removes the directory.
    const engine = new FakeEngine();
    const companion = recording();
    await purgeCompanion(companion, engine, store, 7);
    expect(engine.calls).toEqual(['purge 7']);
    expect(companion.sent).toEqual(['bb07']);
    expect(root.folders.has('deck-streak-companion'), 'the directory is removed').toBe(false);
    expect(await store.load()).toBeNull();
    // A purge with nothing stored still sends Purge, and refuses nothing.
    await purgeCompanion(companion, engine, store, 8);
    expect(companion.sent).toEqual(['bb07', 'bb08']);
    // No storage: nothing is stored, and nothing is read.
    const none = new CheckpointStore(undefined);
    expect(await none.save(CHECKPOINT)).toBe(false);
    expect(await none.load()).toBeNull();
  });
});

describe('the load sequence', () => {
  it('the worker keeps a checkpoint only when the load check keeps it, and otherwise purges and folds from genesis', async () => {
    const verdicts = examined('load check verdicts', [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    for (const verdict of verdicts) {
      const root = new FakeFolder();
      const store = new CheckpointStore({ getDirectory: async () => root });
      await store.save(CHECKPOINT);
      const engine = new FakeEngine();
      engine.verdict = verdict;
      const companion = recording();
      const asked: string[] = [];
      const answer = await loadCheckpoint({
        companion,
        engine,
        store,
        epoch: 5,
        headNamed: async (ack) => {
          asked.push(hex(ack));
          return verdict !== 6;
        },
        index: { count: 12n, head: 1736911810000n }
      });
      // The engine builds the Restore frame from the stored checkpoint, verbatim; the companion is
      // sent that frame, never the raw checkpoint; the load check judges the companion's reply.
      expect(engine.calls.slice(0, 2), `verdict ${verdict}`).toEqual([`restore 5 ${hex(CHECKPOINT)}`, 'load_check']);
      expect(companion.sent[0]).toBe(`aa${hex(CHECKPOINT)}`);
      expect(asked).toEqual(['cc']);
      expect(engine.checked).toEqual(['4849', hex(CHECKPOINT), 'cc', verdict !== 6, 12n, 1736911810000n]);
      if (verdict === 0) {
        expect(answer).toBe('kept');
        expect(engine.calls).toEqual([`restore 5 ${hex(CHECKPOINT)}`, 'load_check']);
        expect(companion.sent).toEqual([`aa${hex(CHECKPOINT)}`]);
        expect(hex((await store.load()) ?? new Uint8Array())).toBe(hex(CHECKPOINT));
      } else {
        expect(answer, `verdict ${verdict}`).toBe('genesis');
        expect(engine.calls[2]).toBe('purge 5');
        expect(companion.sent).toEqual([`aa${hex(CHECKPOINT)}`, 'bb05']);
        expect(await store.load(), `verdict ${verdict} removes the checkpoint`).toBeNull();
      }
    }
    // Nothing stored: genesis, and nothing is asked.
    const empty = new CheckpointStore({ getDirectory: async () => new FakeFolder() });
    const engine = new FakeEngine();
    const companion = recording();
    const answer = await loadCheckpoint({
      companion,
      engine,
      store: empty,
      epoch: 5,
      headNamed: async () => true,
      index: null
    });
    expect([answer, engine.calls, companion.sent]).toEqual(['genesis', [], []]);
    // A checkpoint the engine builds no Restore frame for is never sent; the check judges an empty
    // reply, and its discard purges.
    const root = new FakeFolder();
    const store = new CheckpointStore({ getDirectory: async () => root });
    await store.save(CHECKPOINT);
    const unreadable = new FakeEngine();
    unreadable.restoreAnswer = new Uint8Array();
    unreadable.verdict = 2;
    const told = recording();
    const discarded = await loadCheckpoint({
      companion: told,
      engine: unreadable,
      store,
      epoch: 6,
      headNamed: async () => true,
      index: null
    });
    expect(discarded).toBe('genesis');
    expect(told.sent).toEqual(['bb06']);
    expect(unreadable.checked).toEqual(['4849', hex(CHECKPOINT), '', true, undefined, undefined]);
  });
});

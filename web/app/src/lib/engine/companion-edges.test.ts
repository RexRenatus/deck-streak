import { createHash } from 'node:crypto';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ABSENT, CheckpointStore, loadCompanion } from './companion';
import type { CompanionFile, CompanionFolder, LoaderDeps } from './companion';
import { companionModule } from './companion-modules.test.support';

// SPEC-511 R5 to R9; ADR-529 D2, D3. The loader's and the checkpoint store's edges, each judged by
// what a caller sees: the route names fetched, the digests asked of Web Crypto, the companion
// answered, and the checkpoint stored, read back or removed. Every expected byte, digest, cap and
// name is written out or computed here, never read from the module under test.

/** The route's prefix, 16 MiB and 1 MiB, written out apart from the module's constants. */
const ROUTE = '/api/companion/v1/';
const CAP = 16_777_216;
const FRAME_MAX = 1_048_576;

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString('hex');
/** SHA-256, by node's own hash, apart from the Web Crypto the loader uses. */
const sha256 = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');
const json = (value: unknown) => new TextEncoder().encode(JSON.stringify(value));

/** The echo module and distinct weights, a zero and a byte above 0x7f among them. */
const MODULE = companionModule();
const WEIGHTS = new Uint8Array([0x00, 0x01, 0x7f, 0x80, 0xc3, 0x28, 0xff, 0x10, 0x20]);
const M = sha256(MODULE);
const W = sha256(WEIGHTS);

/** What the route answers for a name: its bytes, its status (200 unless named) and a declared
 * content-length, when one is named. */
interface Served {
  bytes: Uint8Array;
  status?: number;
  length?: string;
}

/** The three files of the echo companion under a manifest of `shape`, with `changed` in place. */
function files(shape: unknown = { module: M, weights: W }, changed: Record<string, Served> = {}): Record<string, Served> {
  return {
    'manifest.json': { bytes: json(shape) },
    'module.wasm': { bytes: MODULE },
    'weights.bin': { bytes: WEIGHTS },
    ...changed
  };
}

/** A fake owner route serving `served`, each name asked recorded; a name it lacks answers 404. */
function route(served: Record<string, Served>, crypto: LoaderDeps['crypto'] = globalThis.crypto) {
  const asked: string[] = [];
  const fetch = async (input: string) => {
    const name = input.startsWith(ROUTE) ? input.slice(ROUTE.length) : input;
    asked.push(name);
    const file = served[name];
    if (file === undefined) return new Response(null, { status: 404 });
    const init: ResponseInit = { status: file.status ?? 200 };
    if (file.length !== undefined) init.headers = { 'content-length': file.length };
    return new Response(file.bytes.slice(), init);
  };
  return { asked, deps: { fetch, crypto } satisfies LoaderDeps };
}

/** The scope's Web Crypto, each input it is asked to digest recorded. */
function recordingCrypto() {
  const digested: unknown[] = [];
  const crypto = {
    subtle: {
      digest: async (algorithm: AlgorithmIdentifier, data: BufferSource) => {
        digested.push(data);
        return globalThis.crypto.subtle.digest(algorithm, data);
      }
    }
  } as unknown as LoaderDeps['crypto'];
  return { digested, crypto };
}

/** A module's raw byte-frame ABI written in JavaScript over `memory`: a bump allocator from 16, and
 * `open` and `call` answering the input's own pointer and length unless `open` is given. */
function jsAbi(memory: { buffer: ArrayBuffer }, open?: (pointer: number, length: number) => bigint) {
  let next = 16;
  const echo = (pointer: number, length: number) => (BigInt(pointer) << 32n) | BigInt(length);
  return {
    memory,
    alloc: (length: number) => {
      const at = next;
      next += length;
      return at;
    },
    free: () => undefined,
    open: open ?? echo,
    call: echo
  };
}

/** WebAssembly's instantiation answering an instance with `exports`, whatever bytes it is given. */
function instantiating(exports: object) {
  const scope = WebAssembly as unknown as { instantiate: (...args: unknown[]) => Promise<unknown> };
  vi.spyOn(scope, 'instantiate').mockResolvedValue({ instance: { exports } });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the companion loader's edges", () => {
  it('a request shorter than its header is answered the refusal at epoch 0, and one of 13 bytes the refusal at its own epoch', () => {
    // len 14 | schema 1.0 | kind 0x86 | epoch | status Absent, little-endian
    const refusal = (epoch: string) => `0e000000${'0100'}${'0000'}86${epoch}01`;
    expect(hex(ABSENT.call(new Uint8Array(0)))).toBe(refusal('00000000'));
    expect(hex(ABSENT.call(new Uint8Array(12)))).toBe(refusal('00000000'));
    const whole = new Uint8Array(13);
    whole.set([0x2a, 0x01, 0x00, 0x00], 9);
    expect(hex(ABSENT.call(whole))).toBe(refusal('2a010000'));
  });

  it('a name answered with a status other than 200 reads Absent though its bytes are right, and a refused manifest asks for nothing more', async () => {
    const control = route(files());
    expect((await loadCompanion(control.deps)).present, 'the same files at 200 are present').toBe(true);
    for (const status of [201, 203, 206]) {
      const manifest = route(files(undefined, { 'manifest.json': { bytes: json({ module: M, weights: W }), status } }));
      expect((await loadCompanion(manifest.deps)).present, `a manifest at ${status}`).toBe(false);
      expect(manifest.asked).toEqual(['manifest.json']);
      const module = route(files(undefined, { 'module.wasm': { bytes: MODULE, status } }));
      expect((await loadCompanion(module.deps)).present, `a module at ${status}`).toBe(false);
      const weights = route(files(undefined, { 'weights.bin': { bytes: WEIGHTS, status } }));
      expect((await loadCompanion(weights.deps)).present, `weights at ${status}`).toBe(false);
    }
  });

  it('a declared length past the cap reads Absent though the body is small, and a declared length of exactly the cap is taken', async () => {
    const over = String(CAP + 1);
    const manifest = route(files(undefined, { 'manifest.json': { bytes: json({ module: M, weights: W }), length: over } }));
    expect((await loadCompanion(manifest.deps)).present).toBe(false);
    expect(manifest.asked).toEqual(['manifest.json']);
    const module = route(files(undefined, { 'module.wasm': { bytes: MODULE, length: over } }));
    expect((await loadCompanion(module.deps)).present).toBe(false);
    const exact = String(CAP);
    const taken = route(
      files(undefined, {
        'manifest.json': { bytes: json({ module: M, weights: W }), length: exact },
        'module.wasm': { bytes: MODULE, length: exact },
        'weights.bin': { bytes: WEIGHTS, length: exact }
      })
    );
    const companion = await loadCompanion(taken.deps);
    expect(companion.present, 'a declared length equal to the cap is within it').toBe(true);
    expect(hex(companion.hello)).toBe(hex(WEIGHTS));
  });

  it('a manifest that names the weights first is the same manifest', async () => {
    const served = route(files({ weights: W, module: M }));
    const companion = await loadCompanion(served.deps);
    expect(companion.present).toBe(true);
    expect(hex(companion.hello)).toBe(hex(WEIGHTS));
  });

  it('a manifest with a third key, a digest that is no string, or a digest with a byte before or after its 64 reads Absent and asks for nothing more', async () => {
    const shapes: [string, unknown][] = [
      ['a third key after both', { module: M, weights: W, zzz: M }],
      ['a third key before both', { aaa: M, module: M, weights: W }],
      ['the module digest in an array', { module: [M], weights: W }],
      ['the weights digest in an array', { module: M, weights: [W] }],
      ['both digests in arrays', { module: [M], weights: [W] }],
      ['a byte before the module digest', { module: `x${M}`, weights: W }],
      ['a digit after the weights digest', { module: M, weights: `${W}0` }],
      ['a digit before the weights digest', { module: M, weights: `0${W}` }],
      ['a byte after the module digest', { module: `${M}x`, weights: W }]
    ];
    for (const [what, shape] of shapes) {
      const served = route(files(shape));
      expect((await loadCompanion(served.deps)).present, what).toBe(false);
      expect(served.asked, what).toEqual(['manifest.json']);
    }
  });

  it("a manifest's own two keys name its digests: a digest an object inherits never stands in for a key the manifest lacks", async () => {
    const inherited: [string, string, unknown][] = [
      ['module', M, { aaa: M, weights: W }],
      ['weights', W, { module: M, zzz: W }]
    ];
    for (const [key, digest, shape] of inherited) {
      Object.defineProperty(Object.prototype, key, { value: digest, configurable: true, writable: true });
      try {
        const served = route(files(shape));
        expect((await loadCompanion(served.deps)).present, `${key} inherited`).toBe(false);
        expect(served.asked, `${key} inherited`).toEqual(['manifest.json']);
      } finally {
        delete (Object.prototype as Record<string, unknown>)[key];
      }
      expect(Object.prototype.hasOwnProperty.call(Object.prototype, key)).toBe(false);
    }
  });

  it('a manifest whose one invalid UTF-8 byte lies in the overwritten value of a duplicated key reads Absent and asks for nothing more', async () => {
    const encode = (text: string) => new TextEncoder().encode(text);
    /** `{"module":"<middle>","module":"<M>","weights":"<W>"}` as bytes, `middle` verbatim. */
    const manifest = (middle: Uint8Array) => {
      const head = encode('{"module":"');
      const tail = encode(`","module":"${M}","weights":"${W}"}`);
      const bytes = new Uint8Array(head.length + middle.length + tail.length);
      bytes.set(head, 0);
      bytes.set(middle, head.length);
      bytes.set(tail, head.length + middle.length);
      return bytes;
    };
    // JSON keeps the last value of a duplicated key, so the same manifest with a valid byte in
    // place of 0xff is taken: only the byte that is no UTF-8 refuses it.
    const valid = route(files(undefined, { 'manifest.json': { bytes: manifest(encode('x')) } }));
    expect((await loadCompanion(valid.deps)).present).toBe(true);
    expect(valid.asked).toEqual(['manifest.json', 'module.wasm', 'weights.bin']);
    const invalid = route(files(undefined, { 'manifest.json': { bytes: manifest(new Uint8Array([0xff])) } }));
    expect((await loadCompanion(invalid.deps)).present).toBe(false);
    expect(invalid.asked).toEqual(['manifest.json']);
  });

  it('a module or weights the route lacks reads Absent before Web Crypto is asked for any digest', async () => {
    const control = recordingCrypto();
    expect((await loadCompanion(route(files(), control.crypto).deps)).present).toBe(true);
    expect(control.digested.length, 'a whole companion asks for both digests').toBe(2);
    const files404: [string, Record<string, Served>][] = [
      ['module.wasm', { 'manifest.json': { bytes: json({ module: M, weights: W }) }, 'weights.bin': { bytes: WEIGHTS } }],
      ['weights.bin', { 'manifest.json': { bytes: json({ module: M, weights: W }) }, 'module.wasm': { bytes: MODULE } }]
    ];
    for (const [missing, served] of files404) {
      const recorded = recordingCrypto();
      const answered = route(served, recorded.crypto);
      expect((await loadCompanion(answered.deps)).present, `${missing} answers 404`).toBe(false);
      expect(answered.asked).toEqual(['manifest.json', 'module.wasm', 'weights.bin']);
      expect(recorded.digested, `${missing} answers 404`).toEqual([]);
    }
  });

  it('exports whose memory is no WebAssembly.Memory read Absent, though the rest of the ABI answers', async () => {
    instantiating(jsAbi(new WebAssembly.Memory({ initial: 1 })));
    const control = await loadCompanion(route(files()).deps);
    expect(control.present, 'the same ABI over a real memory is present').toBe(true);
    expect(hex(control.hello)).toBe(hex(WEIGHTS));
    instantiating(jsAbi({ buffer: new ArrayBuffer(65_536) }));
    expect((await loadCompanion(route(files()).deps)).present).toBe(false);
  });

  it('a reply of exactly 1 MiB is taken, and one byte more reads Absent', async () => {
    const memory = new WebAssembly.Memory({ initial: 32 });
    instantiating(jsAbi(memory, () => BigInt(FRAME_MAX)));
    const taken = await loadCompanion(route(files()).deps);
    expect(taken.present).toBe(true);
    expect(taken.hello.length).toBe(FRAME_MAX);
    instantiating(jsAbi(memory, () => BigInt(FRAME_MAX + 1)));
    expect((await loadCompanion(route(files()).deps)).present).toBe(false);
  });

  it('the hello and each reply keep the bytes the module answered, though the module grows its memory after', async () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const abi = jsAbi(memory);
    const echo = abi.call;
    abi.call = (pointer: number, length: number) => {
      memory.grow(1);
      return echo(pointer, length);
    };
    instantiating(abi);
    const companion = await loadCompanion(route(files()).deps);
    expect(companion.present).toBe(true);
    expect(hex(companion.hello)).toBe(hex(WEIGHTS));
    const reply = companion.call(new Uint8Array([0x01, 0x02, 0x03]));
    expect(hex(reply)).toBe('010203');
    expect(hex(companion.hello), 'the hello outlives the growth').toBe(hex(WEIGHTS));
    expect(hex(companion.call(new Uint8Array([0x04])))).toBe('04');
    expect(hex(reply), 'a reply outlives the next growth').toBe('010203');
  });
});

/** An origin root of one folder and one file whose writable keeps the very array it is given, as a
 * file system may hold a caller's bytes until it flushes them. */
function keeping(): CompanionFolder {
  let kept: Uint8Array<ArrayBuffer> = new Uint8Array();
  const file: CompanionFile = {
    getFile: async () => new Blob([kept]),
    createWritable: async () => ({
      write: async (data: Uint8Array<ArrayBuffer>) => {
        kept = data;
      },
      close: async () => undefined
    })
  };
  const folder: CompanionFolder = {
    getDirectoryHandle: async () => folder,
    getFileHandle: async () => file,
    removeEntry: async () => undefined
  };
  return folder;
}

/** An origin root whose removal fails with `error`. */
function failing(error: unknown): CompanionFolder {
  return {
    getDirectoryHandle: async () => {
      throw new Error('not asked');
    },
    getFileHandle: async () => {
      throw new Error('not asked');
    },
    removeEntry: async () => {
      throw error;
    }
  };
}

describe("the checkpoint store's edges", () => {
  it('a root the browser refuses reads as no storage: nothing is stored, nothing is read, and the removal refuses nothing', async () => {
    const refused = new CheckpointStore({ getDirectory: () => Promise.reject(new DOMException('refused', 'SecurityError')) });
    expect(await refused.save(new Uint8Array([0x01]))).toBe(false);
    expect(await refused.load()).toBeNull();
    await expect(refused.remove()).resolves.toBeUndefined();
    await expect(new CheckpointStore(undefined).remove(), 'no storage').resolves.toBeUndefined();
    await expect(new CheckpointStore({}).remove(), 'a storage with no root').resolves.toBeUndefined();
  });

  it('the stored checkpoint is the bytes given to save, though the caller reuses its buffer after', async () => {
    const root = keeping();
    const store = new CheckpointStore({ getDirectory: async () => root });
    const buffer = new Uint8Array([0x37, 0x00, 0x84, 0xff]);
    expect(await store.save(buffer)).toBe(true);
    buffer.fill(0xee);
    expect(hex((await store.load()) ?? new Uint8Array())).toBe('370084ff');
  });

  it('a removal that finds nothing is done, and every other failure of the removal reaches the caller', async () => {
    const over = (root: CompanionFolder) => new CheckpointStore({ getDirectory: async () => root });
    await expect(over(failing(new DOMException('gone', 'NotFoundError'))).remove()).resolves.toBeUndefined();
    const failures: [string, unknown][] = [
      ['a DOMException of another name', new DOMException('busy', 'NoModificationAllowedError')],
      ['an Error named NotFoundError that is no DOMException', Object.assign(new Error('gone'), { name: 'NotFoundError' })],
      ['a TypeError', new TypeError('broken')]
    ];
    for (const [what, error] of failures) {
      await expect(over(failing(error)).remove(), what).rejects.toBe(error);
    }
  });
});

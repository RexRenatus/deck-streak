// SPEC-511 R5 to R9, ADR-529 D2, D3: the Worker's loader of the owner's scheduling companion, its
// checkpoint store and its load sequence.
//
// The loader fetches the owner route's three names and nothing else: the manifest, then the module
// and the weights, each checked against the manifest's SHA-256 with Web Crypto. It instantiates the
// module with IMPORTS, a frozen empty object, so the module reaches no storage, network or clock;
// it imports no script. Every failure reads ABSENT, whose hello and answers are the
// `Refused { Absent }` frame, which the web engine's decision refuses. The module crosses a raw
// byte-frame ABI, and the Worker moves bytes only: the web engine builds every frame it sends and
// judges every reply.

/** The owner route's prefix, under the API's path. */
export const COMPANION_ROUTE = '/api/companion/v1/';
/** The largest file the loader takes, in bytes: 2^24, 16 MiB, the owner route's own cap. */
export const COMPANION_FILE_CAP = 2 ** 24;
/** The origin private file system's directory of the companion's checkpoint. */
export const COMPANION_DIRECTORY = 'deck-streak-companion';
/** The checkpoint's file in that directory. */
export const STATE_FILE = 'state.bin';
/** The import object every companion module is instantiated with, and the only one: frozen, empty
 * and public, so a module that asks for any import fails to link. */
export const IMPORTS: WebAssembly.Imports = Object.freeze({});

/** The largest frame either side sends or reads: 1 MiB (SPEC-501's `FRAME_MAX`). */
const FRAME_MAX = 1_048_576;
/** A frame's header: `len u32 | major u16 | minor u16 | kind u8 | epoch u32`, little-endian. */
const HEADER = 13;
/** The epoch's offset in the header. */
const EPOCH_AT = 9;
/** The `Refused` reply's kind byte, and the `Absent` status. */
const REFUSED_KIND = 0x86;
const STATUS_ABSENT = 1;
/** The route's fetch: the owner's session cookie, and no cache. */
const FETCHED: RequestInit = Object.freeze({ credentials: 'same-origin', cache: 'no-store' });
/** A manifest's digest: SHA-256 as 64 lower-case hex digits. */
const DIGEST = /^[0-9a-f]{64}$/;

/** A loaded companion: whether it is present, its hello frame, and one reply frame for each
 * request frame. */
export interface Companion {
  readonly present: boolean;
  readonly hello: Uint8Array;
  call(frame: Uint8Array): Uint8Array;
}

/** The scope the loader reads: its fetch and its Web Crypto. */
export interface LoaderDeps {
  fetch(input: string, init: RequestInit): Promise<Response>;
  crypto: Pick<Crypto, 'subtle'>;
}

/** The `Refused { Absent }` reply frame at `epoch`: 14 bytes, schema 1.0. */
export function refusedAbsent(epoch: number): Uint8Array {
  const frame = new Uint8Array(HEADER + 1);
  const view = new DataView(frame.buffer);
  view.setUint32(0, frame.length, true);
  view.setUint16(4, 1, true);
  view.setUint16(6, 0, true);
  view.setUint8(8, REFUSED_KIND);
  view.setUint32(EPOCH_AT, epoch >>> 0, true);
  view.setUint8(HEADER, STATUS_ABSENT);
  return frame;
}

/** A request frame's epoch, or 0 when the frame is shorter than its header. */
export function epochOf(frame: Uint8Array): number {
  if (frame.length < HEADER) return 0;
  return new DataView(frame.buffer, frame.byteOffset, frame.byteLength).getUint32(EPOCH_AT, true);
}

/** No companion: its hello is the refusal at epoch 0, and it answers every request with the
 * refusal at the request's epoch. */
export const ABSENT: Companion = Object.freeze({
  present: false,
  hello: refusedAbsent(0),
  call: (frame: Uint8Array) => refusedAbsent(epochOf(frame))
});

/** The raw byte-frame ABI a companion module exports. */
interface Abi {
  memory: WebAssembly.Memory;
  alloc(length: number): number;
  free(pointer: number, length: number): void;
  open(pointer: number, length: number): bigint;
  call(pointer: number, length: number): bigint;
}

/** The module's exports as the ABI, or null when one is missing or of another kind. */
function abiOf(exports: WebAssembly.Exports): Abi | null {
  const { memory, alloc, free, open, call } = exports;
  if (!(memory instanceof WebAssembly.Memory)) return null;
  if (![alloc, free, open, call].every((each) => typeof each === 'function')) return null;
  return exports as unknown as Abi;
}

/** The body of the route's `name` when it answers 200 with at most the cap, else null. */
async function fetched(deps: LoaderDeps, name: string): Promise<Uint8Array<ArrayBuffer> | null> {
  const answer = await deps.fetch(COMPANION_ROUTE + name, { ...FETCHED });
  if (answer.status !== 200) return null;
  const declared = Number(answer.headers.get('content-length') ?? '0');
  if (declared > COMPANION_FILE_CAP) return null;
  const body = new Uint8Array(await answer.arrayBuffer());
  return body.length > COMPANION_FILE_CAP ? null : body;
}

/** The manifest's two digests when it is exactly `{"module": <64 hex>, "weights": <64 hex>}`. */
function digests(bytes: Uint8Array): { module: string; weights: string } | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    return null;
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return null;
  const keys = Object.keys(parsed).sort();
  if (keys.length !== 2 || keys[0] !== 'module' || keys[1] !== 'weights') return null;
  const { module, weights } = parsed as Record<string, unknown>;
  if (typeof module !== 'string' || typeof weights !== 'string') return null;
  return DIGEST.test(module) && DIGEST.test(weights) ? { module, weights } : null;
}

/** SHA-256 of `bytes` as lower-case hex, by the scope's Web Crypto. */
async function sha256(crypto: LoaderDeps['crypto'], bytes: Uint8Array<ArrayBuffer>): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** `input` copied into the module through `alloc`, handed to `run`, and the reply it packs (its
 * pointer high, its length low) copied out; both buffers are then freed. Null when the module
 * traps, answers past its memory, or answers more than a frame. */
function exchange(abi: Abi, input: Uint8Array, run: (pointer: number, length: number) => bigint): Uint8Array | null {
  try {
    const pointer = abi.alloc(input.length) >>> 0;
    new Uint8Array(abi.memory.buffer, pointer, input.length).set(input);
    const packed = BigInt.asUintN(64, BigInt(run(pointer, input.length)));
    const at = Number(packed >> 32n);
    const length = Number(packed & 0xffffffffn);
    if (length > FRAME_MAX) return null;
    const reply = new Uint8Array(abi.memory.buffer, at, length).slice();
    abi.free(pointer, input.length);
    abi.free(at, length);
    return reply;
  } catch {
    return null;
  }
}

/** A present companion over `abi`, opened with its `hello`. A call that traps answers the refusal
 * at the request's epoch. */
function present(abi: Abi, hello: Uint8Array): Companion {
  return Object.freeze({
    present: true,
    hello,
    call: (frame: Uint8Array) => exchange(abi, frame, abi.call) ?? refusedAbsent(epochOf(frame))
  });
}

/** The owner's companion, loaded from the owner route, or ABSENT. */
export async function loadCompanion(deps: LoaderDeps): Promise<Companion> {
  try {
    const manifest = await fetched(deps, 'manifest.json');
    const wanted = manifest === null ? null : digests(manifest);
    if (wanted === null) return ABSENT;
    const module = await fetched(deps, 'module.wasm');
    const weights = await fetched(deps, 'weights.bin');
    if (module === null || weights === null) return ABSENT;
    if ((await sha256(deps.crypto, module)) !== wanted.module) return ABSENT;
    if ((await sha256(deps.crypto, weights)) !== wanted.weights) return ABSENT;
    const { instance } = await WebAssembly.instantiate(module, IMPORTS);
    const abi = abiOf(instance.exports);
    if (abi === null) return ABSENT;
    const hello = exchange(abi, weights, abi.open);
    return hello === null ? ABSENT : present(abi, hello);
  } catch {
    return ABSENT;
  }
}

/** A file as the store needs it. */
export interface CompanionFile {
  getFile(): Promise<Blob>;
  createWritable(): Promise<{ write(data: Uint8Array<ArrayBuffer>): Promise<void>; close(): Promise<void> }>;
}

/** A directory as the store needs it. */
export interface CompanionFolder {
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<CompanionFolder>;
  getFileHandle(name: string, options?: { create?: boolean }): Promise<CompanionFile>;
  removeEntry(name: string, options?: { recursive?: boolean }): Promise<void>;
}

/** The storage as the store needs it: the origin's root directory, when the browser has one. */
export interface CompanionStorage {
  getDirectory?(): Promise<CompanionFolder>;
}

/** The companion's checkpoint frame, kept verbatim as `state.bin` in the origin private file
 * system's `deck-streak-companion` directory. */
export class CheckpointStore {
  constructor(readonly storage: CompanionStorage | undefined) {}

  /** The origin's root directory, or null when the browser has none or refuses it. */
  async #root(): Promise<CompanionFolder | null> {
    try {
      return await this.storage!.getDirectory!();
    } catch {
      return null;
    }
  }

  /** Stores `checkpoint` verbatim in place of any before it; false when there is no storage. */
  async save(checkpoint: Uint8Array): Promise<boolean> {
    const root = await this.#root();
    if (root === null) return false;
    const folder = await root.getDirectoryHandle(COMPANION_DIRECTORY, { create: true });
    const file = await folder.getFileHandle(STATE_FILE, { create: true });
    const writable = await file.createWritable();
    await writable.write(checkpoint.slice());
    await writable.close();
    return true;
  }

  /** The stored checkpoint, byte for byte, or null when none is stored. */
  async load(): Promise<Uint8Array | null> {
    const root = await this.#root();
    if (root === null) return null;
    try {
      const folder = await root.getDirectoryHandle(COMPANION_DIRECTORY);
      const file = await (await folder.getFileHandle(STATE_FILE)).getFile();
      return new Uint8Array(await file.arrayBuffer());
    } catch {
      return null;
    }
  }

  /** Removes the whole directory; a directory that is not there is already removed. */
  async remove(): Promise<void> {
    const root = await this.#root();
    if (root === null) return;
    try {
      await root.removeEntry(COMPANION_DIRECTORY, { recursive: true });
    } catch (error) {
      if (!(error instanceof DOMException && error.name === 'NotFoundError')) throw error;
    }
  }
}

/** The web engine's companion exports (SPEC-511 R10), as wasm-bindgen writes them. */
export interface CompanionEngine {
  companion_decide(hello: Uint8Array, epoch: number, preview: Uint8Array, reply: Uint8Array): Uint8Array;
  companion_restore(epoch: number, checkpoint: Uint8Array): Uint8Array;
  companion_load_check(
    hello: Uint8Array,
    checkpoint: Uint8Array,
    ack: Uint8Array,
    headNamed: boolean,
    indexCount: bigint | undefined,
    indexHead: bigint | undefined
  ): number;
  companion_purge(epoch: number): Uint8Array;
}

/** The fold index held beside a checkpoint: how many reviews were folded, and the newest's id. */
export interface FoldIndex {
  count: bigint;
  head: bigint;
}

/** What the load sequence reads: the companion, the web engine, the store, the session's epoch,
 * whether the companion's reply names a review in the store (the engine's answer), and the fold
 * index held, if any. */
export interface LoadDeps {
  companion: Companion;
  engine: CompanionEngine;
  store: CheckpointStore;
  epoch: number;
  headNamed(ack: Uint8Array): Promise<boolean>;
  index: FoldIndex | null;
}

/** Sends the companion the `Purge` frame the web engine builds at `epoch`, and removes the stored
 * checkpoint's directory. */
export async function purgeCompanion(
  companion: Companion,
  engine: CompanionEngine,
  store: CheckpointStore,
  epoch: number
): Promise<void> {
  companion.call(engine.companion_purge(epoch));
  await store.remove();
}

/** The load sequence: a stored checkpoint is handed back only as the `Restore` frame the web
 * engine builds from it, and kept only when the web load check keeps it. Any discard purges the
 * companion and the store, and the caller folds from genesis; with nothing stored there is nothing
 * to keep. */
export async function loadCheckpoint(deps: LoadDeps): Promise<'kept' | 'genesis'> {
  const { companion, engine, store, epoch, index } = deps;
  const checkpoint = await store.load();
  if (checkpoint === null) return 'genesis';
  const restore = engine.companion_restore(epoch, checkpoint);
  const ack = restore.length > 0 ? companion.call(restore) : new Uint8Array();
  const named = await deps.headNamed(ack);
  const verdict = engine.companion_load_check(companion.hello, checkpoint, ack, named, index?.count, index?.head);
  if (verdict === 0) return 'kept';
  await purgeCompanion(companion, engine, store, epoch);
  return 'genesis';
}

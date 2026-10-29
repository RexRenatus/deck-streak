// SPEC-511 R5 to R9, ADR-529 D2, D3: the Worker's loader of the owner's scheduling companion.
//
// STUB: the shapes only. The loader fetches no manifest, checks no digest and passes an import
// object of its own; Absent answers nothing; the store keeps nothing; the load sequence keeps
// every checkpoint.

/** The owner route's prefix, under the API's path. */
export const COMPANION_ROUTE = '/api/companion/v1/';
/** The largest file the loader takes, in bytes: 16 MiB. */
export const COMPANION_FILE_CAP = 16 * 1024 * 1024;
/** The origin private file system's directory of the companion's checkpoint. */
export const COMPANION_DIRECTORY = 'deck-streak-companion';
/** The checkpoint's file in that directory. */
export const STATE_FILE = 'state.bin';
/** The import object every companion module is instantiated with. */
export const IMPORTS: WebAssembly.Imports = Object.freeze({});

/** A loaded companion: its hello frame, and one reply frame for each request frame. */
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

/** No companion. */
export const ABSENT: Companion = {
  present: false,
  hello: new Uint8Array(),
  call: () => new Uint8Array()
};

/** The stub's own import object. */
const STUB_IMPORTS = { env: { x: () => 0 } };

/** The raw byte-frame ABI a companion module exports. */
interface Abi {
  memory: WebAssembly.Memory;
  alloc(length: number): number;
  free(pointer: number, length: number): void;
  open(pointer: number, length: number): bigint;
  call(pointer: number, length: number): bigint;
}

/** The body of the route's `name`. */
async function body(deps: LoaderDeps, name: string): Promise<Uint8Array<ArrayBuffer>> {
  const answer = await deps.fetch(COMPANION_ROUTE + name, { credentials: 'same-origin', cache: 'no-store' });
  return new Uint8Array(await answer.arrayBuffer());
}

/** `input` copied into the module and handed to `run`, its reply copied out. */
function exchange(abi: Abi, input: Uint8Array, run: (pointer: number, length: number) => bigint): Uint8Array {
  const pointer = abi.alloc(input.length);
  new Uint8Array(abi.memory.buffer, pointer, input.length).set(input);
  const packed = run(pointer, input.length);
  const at = Number(packed >> 32n);
  const length = Number(packed & 0xffffffffn);
  return new Uint8Array(abi.memory.buffer, at, length).slice();
}

/** The companion module the route serves. */
export async function loadCompanion(deps: LoaderDeps): Promise<Companion> {
  try {
    const module = await body(deps, 'module.wasm');
    const weights = await body(deps, 'weights.bin');
    const { instance } = await WebAssembly.instantiate(module, STUB_IMPORTS);
    const abi = instance.exports as unknown as Abi;
    const hello = exchange(abi, weights, abi.open);
    return { present: true, hello, call: (frame) => exchange(abi, frame, abi.call) };
  } catch {
    return { present: true, hello: new Uint8Array(), call: () => new Uint8Array() };
  }
}

/** A file as the store needs it. */
export interface CompanionFile {
  getFile(): Promise<Blob>;
  createWritable(): Promise<{ write(data: Uint8Array): Promise<void>; close(): Promise<void> }>;
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

/** The companion's checkpoint in the origin private file system. */
export class CheckpointStore {
  constructor(readonly storage: CompanionStorage | undefined) {}

  /** Stores `checkpoint`. */
  async save(checkpoint: Uint8Array): Promise<boolean> {
    return checkpoint.length < 0;
  }

  /** The stored checkpoint. */
  async load(): Promise<Uint8Array | null> {
    return null;
  }

  /** Removes the directory. */
  async remove(): Promise<void> {}
}

/** The web engine's companion exports (SPEC-511 R10). */
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

/** What the load sequence reads. */
export interface LoadDeps {
  companion: Companion;
  engine: CompanionEngine;
  store: CheckpointStore;
  epoch: number;
  headNamed(ack: Uint8Array): Promise<boolean>;
  index: FoldIndex | null;
}

/** Sends the companion `Purge` and removes the stored checkpoint. */
export async function purgeCompanion(
  companion: Companion,
  engine: CompanionEngine,
  store: CheckpointStore,
  epoch: number
): Promise<void> {
  void [companion, engine, store, epoch];
}

/** The load sequence. */
export async function loadCheckpoint(deps: LoadDeps): Promise<'kept' | 'genesis'> {
  return (await deps.store.load()) === null ? 'genesis' : 'kept';
}

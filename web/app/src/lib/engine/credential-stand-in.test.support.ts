// Test support for SPEC-363 part 2 (R9 to R16): the fakes the credential store's tests run it over.
// Vitest runs no wasm, so `standIn` is a STAND-IN for PR 1's five credential exports
// (`crates/web-engine/src/wasm.rs`): it answers as the core's rule answers
// (`crates/engine-core/src/credential.rs`, SPEC-363 A2 to A6) for the inputs these tests use. The
// real module's answers are CI's `test:engine`. This file is no test and no production code: its
// name keeps it out of the test runner's files and out of the mutation run's population.
import type { CredentialChannel, CredentialDeps, CredentialEngine } from './credential';
import type { EngineModule } from './session';

/** The origin every test's Worker runs on, and the one route the sync key may go to. */
export const ORIGIN = 'https://app.example';
export const ENDPOINT = `${ORIGIN}/anki-sync/`;

/** The largest generation: a u64, as wasm-bindgen carries it. */
export const MAX = 2n ** 64n - 1n;
const next = (generation: bigint) => (generation < MAX ? generation + 1n : undefined);

/** The engine error kinds the core reads (Anki's `BackendError.Kind`). */
export const NETWORK_ERROR = 6;
export const SYNC_AUTH_ERROR = 7;

/** The engine's encoded error of `kind`, as prost writes a `BackendError`: its message (field 1)
 * and its kind (field 2). */
export function engineError(kind: number): Uint8Array {
  const message = new TextEncoder().encode('a synthetic answer');
  return Uint8Array.of(0x0a, message.length, ...message, 0x10, kind);
}

/** The kind an encoded error names, or null when the bytes do not decode. */
function kindOf(bytes: Uint8Array): number | null {
  let at = 0;
  const varint = () => {
    let value = 0;
    for (let shift = 0; ; shift += 7) {
      if (at >= bytes.length) throw new RangeError('truncated');
      const byte = bytes[at++];
      value += (byte & 0x7f) * 2 ** shift;
      if (byte < 0x80) return value;
    }
  };
  try {
    let kind = 0;
    while (at < bytes.length) {
      const tag = varint();
      if ((tag & 7) === 0) {
        const value = varint();
        if (tag >> 3 === 2) kind = value;
      } else if ((tag & 7) === 2) {
        // the length is read first: `at += varint()` would add it to the offset before the read
        const length = varint();
        at += length;
        if (at > bytes.length) throw new RangeError('truncated');
      } else {
        throw new RangeError('wire type');
      }
    }
    return kind;
  } catch {
    return null;
  }
}

/** The STAND-IN for the core's five exports (SPEC-363 A2 to A6), recording each call. */
export function standIn(): CredentialEngine & { calls: string[] } {
  const calls: string[] = [];
  return {
    calls,
    credential_on_obtained(started, current) {
      calls.push('on_obtained');
      return started === current ? next(current) : undefined;
    },
    credential_may_send(held, current, sealed) {
      calls.push('may_send');
      return sealed && held === current;
    },
    credential_classify(error) {
      calls.push('classify');
      if (error === undefined) return 0;
      return kindOf(error) === SYNC_AUTH_ERROR ? 1 : 2;
    },
    credential_on_outcome(sent, current, outcome) {
      calls.push('on_outcome');
      return outcome === 1 && sent === current;
    },
    credential_on_removed(current) {
      calls.push('on_removed');
      return next(current);
    }
  };
}

const base64url = (bytes: Uint8Array) =>
  btoa(String.fromCharCode(...bytes))
    .replaceAll('+', '-')
    .replaceAll('/', '_')
    .replaceAll('=', '');

/** The service's release, as `crates/api/src/sync_seal_routes.rs` answers it: the owner's session
 * gets a key for each seal id, derived from the secret's version, so rotating the secret changes
 * every key. Each key starts with bytes whose encoding holds both `-` and `_`. */
export class ReleaseService {
  session = true;
  reachable = true;
  version = 1;
  /** A refusal the service answers whatever the session: 404 when it holds no secret, 429 past its
   * bound. Null answers as above. */
  refusing: 404 | 429 | null = null;
  /** Each release asked for, by its seal id, and every URL any request named. */
  released: string[] = [];
  urls: string[] = [];
  /** Each key the service released, in its encoding. */
  keys: string[] = [];

  readonly fetch = async (input: string, init: RequestInit): Promise<Response> => {
    this.urls.push(input);
    if (!this.reachable) throw new TypeError('the network is down');
    const json = new Headers(init.headers).get('content-type') === 'application/json';
    if (input !== `${ORIGIN}/api/sync/seal-key` || init.method !== 'POST' || !json) {
      return Response.json({ reason: 'refused' }, { status: 403 });
    }
    if (this.refusing !== null) return Response.json({ reason: 'refused' }, { status: this.refusing });
    if (!this.session) return Response.json({ reason: 'no_session' }, { status: 401 });
    const seal = (JSON.parse(String(init.body)) as { seal_id: unknown }).seal_id;
    if (typeof seal !== 'string' || !/^[A-Za-z0-9_-]{22}$/.test(seal)) {
      return Response.json({ reason: 'seal_id_invalid' }, { status: 400 });
    }
    this.released.push(seal);
    const key = await this.keyFor(seal);
    this.keys.push(key);
    return Response.json({ key });
  };

  /** The key the service releases for `seal` under its current version. */
  async keyFor(seal: string): Promise<string> {
    const digest = new Uint8Array(
      await crypto.subtle.digest('SHA-256', new TextEncoder().encode(`${this.version}:${seal}`))
    );
    return base64url(Uint8Array.of(0xfb, 0xff, 0xbf, ...digest.subarray(3)));
  }
}

/** A broadcast channel's bus: each channel joined to it hears every other's message at once. */
export class Bus {
  readonly #channels = new Set<FakeChannel>();
  names: string[] = [];

  join(name: string): FakeChannel {
    this.names.push(name);
    const channel = new FakeChannel(this);
    this.#channels.add(channel);
    return channel;
  }

  deliver(from: FakeChannel | null, data: unknown, origin: string) {
    for (const channel of this.#channels) {
      if (channel !== from) channel.hear(new MessageEvent('message', { data, origin }));
    }
  }
}

export class FakeChannel implements CredentialChannel {
  readonly #bus: Bus;
  readonly #listeners: ((event: MessageEvent) => void)[] = [];
  posted: unknown[] = [];

  constructor(bus: Bus) {
    this.#bus = bus;
  }

  postMessage(message: null) {
    this.posted.push(message);
    this.#bus.deliver(this, message, ORIGIN);
  }

  addEventListener(type: 'message', listener: (event: MessageEvent) => void) {
    if (type === 'message') this.#listeners.push(listener);
  }

  hear(event: MessageEvent) {
    for (const listener of this.#listeners) listener(event);
  }
}

/** A Web Crypto whose random bytes are fixed, so every seal id's encoding holds `-` and `_`. */
export const fixedCrypto: Pick<Crypto, 'subtle' | 'getRandomValues'> = {
  subtle: crypto.subtle,
  getRandomValues<T extends ArrayBufferView | null>(array: T): T {
    const view = array as ArrayBufferView;
    const bytes = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
    bytes.forEach((_, at) => (bytes[at] = [0xfb, 0xff, 0xbf, 0x3e][at % 4]));
    return array;
  }
};

/** A sentinel built from its parts at run time, never one literal. */
export const sentinel = (...parts: string[]) => parts.join('-');

/** A login that answers `key` for every call, recording each. */
export function loginAnswering(key: string) {
  const calls: [string, string, string][] = [];
  const login = async (endpoint: string, user: string, password: string) => {
    calls.push([endpoint, user, password]);
    return key;
  };
  return { calls, login };
}

/** A login that lands only when the test lets it. */
export function heldLogin(key: string) {
  let land = () => {};
  const started = new Promise<void>((begun) => {
    land = begun;
  });
  let resolve: (value: string) => void = () => {};
  const login = (_endpoint: string, _user: string, _password: string) => {
    land();
    return new Promise<string>((done) => (resolve = done));
  };
  return { started, login, land: () => resolve(key) };
}

/** The deps of one Worker's store over `database`, `release` and `bus`. */
export function workerDeps(
  database: IDBFactory,
  release: ReleaseService,
  bus: Bus,
  engine: CredentialEngine = standIn()
): CredentialDeps {
  return {
    origin: ORIGIN,
    indexedDB: () => database,
    fetch: release.fetch,
    crypto: fixedCrypto,
    channel: (name) => bus.join(name),
    load: async () => engine
  };
}

/** Reads the store's two keys straight from the database, as no Worker module would. */
export function readStored(database: IDBFactory): Promise<{ generation: unknown; sealed: unknown }> {
  return new Promise((resolve, reject) => {
    const opened = database.open('deck-streak-credential');
    opened.onsuccess = () => {
      const transaction = opened.result.transaction('credential', 'readonly');
      const store = transaction.objectStore('credential');
      const generation = store.get('generation');
      const sealed = store.get('sealed');
      transaction.oncomplete = () => {
        opened.result.close();
        resolve({ generation: generation.result, sealed: sealed.result });
      };
    };
    opened.onerror = () => reject(opened.error);
  });
}

/** Writes `value` under `key` straight into the store, as a tamperer with the origin's storage could. */
export function writeStored(database: IDBFactory, key: string, value: unknown): Promise<void> {
  return new Promise((resolve, reject) => {
    const opened = database.open('deck-streak-credential');
    opened.onsuccess = () => {
      const transaction = opened.result.transaction('credential', 'readwrite');
      transaction.objectStore('credential').put(value, key);
      transaction.oncomplete = () => {
        opened.result.close();
        resolve();
      };
    };
    opened.onerror = () => reject(opened.error);
  });
}

/** A module that studies: each answer a fixed function of its arguments, so two sessions given the
 * same requests answer the same. */
export function studyEngine(): EngineModule {
  return {
    install_storage: async () => 0,
    init: () => undefined,
    open: () => JSON.stringify({ existed: true, notes: 3 }),
    close: () => undefined,
    seed: (count) => count,
    next_card: () => 1001n,
    undo: () => undefined,
    undo_offer: () => JSON.stringify({ offer: null, why: 'none' }),
    snapshot: (card) => JSON.stringify([Number(card), 2, 2, 5, 3, 1, 0]),
    last_panic: () => undefined,
    memory_pages: () => 17,
    deck_tree: () => JSON.stringify([{ id: '1', name: 'Default', level: 1, new: 1, learning: 0, review: 0, children: [] }]),
    set_current_deck: () => undefined,
    current_card: () => JSON.stringify({ counts: { new: 1, learning: 0, review: 0 }, card: null }),
    rate: () => undefined,
    bury: () => undefined,
    flag: () => 1,
    faces: () => {
      const face = { text: 'q', css: '', autoplay: [], replay: [], omitted: [] };
      return { question: face, answer: face, wanted: [] };
    }
  };
}

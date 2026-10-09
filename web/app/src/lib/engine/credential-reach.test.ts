import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { IDBFactory } from 'fake-indexeddb';
import { describe, expect, it } from 'vitest';
import { CredentialStore } from './credential';
import type { Sendable } from './credential';
import {
  Bus,
  ENDPOINT,
  ORIGIN,
  ReleaseService,
  loginAnswering,
  sentinel,
  standIn,
  studyEngine,
  workerDeps
} from './credential-stand-in.test.support';

// SPEC-363 section 7, B7 and B8: where the sync key can reach. No Worker reply and no answer of the
// store carries a secret, and no module but the Worker's imports the credential module.

/** An answer as the scan reads it: JSON, with each bigint as its decimal string. */
const text = (value: unknown) =>
  JSON.stringify(value, (_, field: unknown) => (typeof field === 'bigint' ? field.toString() : field)) ?? 'undefined';

/** Each answer that carries a secret, named by where it came from and which secret it carries. */
function leaks(answers: [string, unknown][], secrets: [string, string][]): string[] {
  return answers.flatMap(([where, answer]) =>
    secrets.filter(([, secret]) => text(answer).includes(secret)).map(([name]) => `${where} carries ${name}`)
  );
}

/** web/app/src, from this file's directory. */
const SOURCE = resolve(import.meta.dirname, '..', '..');
const CREDENTIAL = join(SOURCE, 'lib', 'engine', 'credential');
/** The Worker's modules, the only ones that may import the credential module (R15). */
const WORKER_MODULES = ['lib/engine/session.ts', 'lib/engine/sync.ts', 'lib/engine/worker.ts'];

interface Source {
  path: string;
  text: string;
}

/** Every production module under `dir`: TypeScript, JavaScript and Svelte, never a test, a
 * declaration file or the generated messages. */
function production(dir: string): Source[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === 'paraglide' ? [] : production(path);
    if (!/\.(ts|js|svelte)$/.test(entry.name)) return [];
    if (/\.(test|spec)\./.test(entry.name) || entry.name.endsWith('.d.ts')) return [];
    return [{ path, text: readFileSync(path, 'utf8') }];
  });
}

/** Each module specifier a source names: static imports and re-exports, side-effect imports and
 * dynamic imports. */
const SPECIFIER = /\bfrom\s*['"]([^'"]+)['"]|\bimport\s*\(?\s*['"]([^'"]+)['"]/g;

/** The file a specifier names, without its extension, or null for a package. */
function target(from: string, specifier: string): string | null {
  if (specifier === '$lib' || specifier.startsWith('$lib/')) return join(SOURCE, 'lib', specifier.slice(5));
  if (!specifier.startsWith('.')) return null;
  return resolve(dirname(from), specifier).replace(/\.(ts|js)$/, '');
}

/** The modules that import the credential module, each by its path under web/app/src. */
function importers(sources: Source[]): string[] {
  return sources
    .filter(({ path, text }) =>
      [...text.matchAll(SPECIFIER)].some((match) => target(path, match[1] ?? match[2]) === CREDENTIAL)
    )
    .map(({ path }) => relative(SOURCE, path))
    .sort();
}

/** The sync module, which only the Worker's entry imports (SPEC-364 R17, B5). */
const SYNC = join(SOURCE, 'lib', 'engine', 'sync');

/** The modules that import `module`, a file under web/app/src named without its extension, each by
 * its path under web/app/src. */
function importersOf(sources: Source[], module: string): string[] {
  return sources
    .filter(({ path, text }) => [...text.matchAll(SPECIFIER)].some((match) => target(path, match[1] ?? match[2]) === module))
    .map(({ path }) => relative(SOURCE, path))
    .sort();
}

describe('where the sync key reaches', () => {
  it('no worker reply carries a secret, and a planted one is caught', async () => {
    // B7: a sentinel host key, password and user, and every key the service released
    const hostKey = sentinel('host', 'key', 'reach');
    const password = sentinel('pass', 'word', 'reach');
    const user = sentinel('sync', 'user', 'reach');
    const release = new ReleaseService();
    const store = new CredentialStore(workerDeps(new IDBFactory(), release, new Bus()));
    const answers: [string, unknown][] = [];
    answers.push(['obtain', await store.obtain(loginAnswering(hostKey).login, user, password)]);
    answers.push(['status', await store.status()]);
    // forSend's answer is the key a sync request carries (R12), so it is the one answer not scanned
    const sent = (await store.forSend(ENDPOINT)) as Sendable;
    answers.push(['settle', await store.settle(sent.generation)]);

    const { serve } = await import('./worker');
    const replies: unknown[] = [];
    let hear: (event: MessageEvent) => void = () => {};
    const session = serve(
      { postMessage: (reply) => replies.push(reply), addEventListener: (_, listener) => (hear = listener) },
      { lock: async () => 'held', storage: async () => null, load: async () => studyEngine(), credential: store },
      ORIGIN
    );
    const requests = [
      { op: 'credential-status' },
      { op: 'open' },
      { op: 'card' },
      { op: 'credential-forget' },
      { op: 'credential-status' }
    ];
    for (const [id, body] of requests.entries()) hear(new MessageEvent('message', { data: { id, ...body } }));
    // the session answers in order, so its next answer comes after every reply above is posted
    answers.push(['a last status', await session.handle({ id: requests.length, op: 'credential-status' })]);
    answers.push(...replies.map((reply, at): [string, unknown] => [`reply ${at}`, reply]));
    answers.push(['forget', await store.forget()]);

    const secrets: [string, string][] = [
      ['the host key', hostKey],
      ['the password', password],
      ['the user', user],
      ...release.keys.map((key, at): [string, string] => [`released key ${at}`, key])
    ];
    expect(leaks(answers, secrets)).toEqual([]);
    expect(answers.map(([where]) => where)).toEqual([
      'obtain',
      'status',
      'settle',
      'a last status',
      'reply 0',
      'reply 1',
      'reply 2',
      'reply 3',
      'reply 4',
      'forget'
    ]);
    expect(release.keys).toHaveLength(1);
    // the positive control: a planted reply that carries the host key is caught by name
    expect(leaks([['a planted reply', { id: 9, ok: true, value: { key: hostKey } }]], secrets)).toEqual([
      'a planted reply carries the host key'
    ]);
  });

  it('only the worker imports the credential module', () => {
    // B8: the census of every production module under web/app/src
    const sources = production(SOURCE);
    console.log(`examined ${sources.length} production modules under web/app/src`);
    const found = importers(sources);
    expect(found.filter((path) => !WORKER_MODULES.includes(path))).toEqual([]);
    expect(found).toContain('lib/engine/worker.ts');
    expect(sources.length).toBeGreaterThan(0);
    // the positive control: a planted page that imports the credential module is refused by name
    const planted = {
      path: join(SOURCE, 'routes', 'planted', '+page.svelte'),
      text: `<script lang="ts">\n  import { CredentialStore } from '$lib/engine/credential';\n</script>\n`
    };
    expect(importers([...sources, planted]).filter((path) => !WORKER_MODULES.includes(path))).toEqual([
      'routes/planted/+page.svelte'
    ]);
  });

  it('no reply to sync-login or sync carries a secret, and each answered ok', async () => {
    // SPEC-364 B5: the host key the engine's login answers, the password and the user
    const hostKey = sentinel('host', 'key', 'sync', 'reach');
    const password = sentinel('pass', 'word', 'sync', 'reach');
    const user = sentinel('sync', 'user', 'sync', 'reach');
    const release = new ReleaseService();
    const engine = { ...standIn(), sync_login: () => hostKey, sync_collection: () => 1, handshake: () => undefined };
    const store = new CredentialStore(workerDeps(new IDBFactory(), release, new Bus(), engine));
    const { serve } = await import('./worker');
    const { Sync } = await import('./sync');
    const replies: unknown[] = [];
    let hear: (event: MessageEvent) => void = () => {};
    const session = serve(
      { postMessage: (reply) => replies.push(reply), addEventListener: (_, listener) => (hear = listener) },
      {
        lock: async () => 'held',
        storage: async () => null,
        load: async () => studyEngine(),
        credential: store,
        sync: new Sync(store, async () => engine, ENDPOINT, async () => new Response('{"minimum_client_level":1}', { status: 200 }))
      },
      ORIGIN
    );
    const requests = [{ op: 'open' }, { op: 'sync-login', user, password }, { op: 'sync' }, { op: 'credential-status' }];
    for (const [id, body] of requests.entries()) hear(new MessageEvent('message', { data: { id, ...body } }));
    // the session answers in order, so its next answer comes after every reply above is posted
    const last = await session.handle({ id: requests.length, op: 'sync' });
    const answers: [string, unknown][] = [
      ...replies.map((reply, at): [string, unknown] => [`reply ${at}`, reply]),
      ['a last sync', last]
    ];
    const secrets: [string, string][] = [
      ['the host key', hostKey],
      ['the password', password],
      ['the user', user],
      ...release.keys.map((key, at): [string, string] => [`released key ${at}`, key])
    ];
    expect(leaks(answers, secrets)).toEqual([]);
    // each new operation answered ok, so the scan judged answers and not refusals
    expect(answers.map(([, answer]) => answer)).toEqual([
      { id: 0, ok: true, value: { existed: true, notes: 3 } },
      { id: 1, ok: true, value: 'held' },
      { id: 2, ok: true, value: { status: 'held', required: 'normal-sync' } },
      { id: 3, ok: true, value: 'held' },
      { id: 4, ok: true, value: { status: 'held', required: 'normal-sync' } }
    ]);
    expect(release.keys).toHaveLength(1);
    // the positive control: a planted sync reply that carries the host key is caught by name
    expect(leaks([['a planted sync reply', { id: 9, ok: true, value: { status: 'held', key: hostKey } }]], secrets)).toEqual([
      'a planted sync reply carries the host key'
    ]);
  });

  it('only the worker imports the sync module', () => {
    // SPEC-364 B5: the census of every production module under web/app/src
    const sources = production(SOURCE);
    console.log(`examined ${sources.length} production modules under web/app/src`);
    expect(sources.length).toBeGreaterThan(0);
    expect(importersOf(sources, SYNC)).toEqual(['lib/engine/worker.ts']);
    // the positive control: a planted page that imports the sync module is refused by name
    const planted = {
      path: join(SOURCE, 'routes', 'planted', '+page.svelte'),
      text: `<script lang="ts">\n  import { Sync } from '$lib/engine/sync';\n</script>\n`
    };
    expect(importersOf([...sources, planted], SYNC).filter((path) => path !== 'lib/engine/worker.ts')).toEqual([
      'routes/planted/+page.svelte'
    ]);
  });
});

/** Logs how many `what` a census examined, and fails on an empty population, so a census that
 * judged nothing never reads as a pass. */
function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

// Written after green: the census's own population, so B8's answer is over the modules it must judge.
describe("the credential census's population", () => {
  it('the census examines the credential module and every Worker module', () => {
    const paths = examined('production modules under web/app/src', production(SOURCE)).map(({ path }) => path);
    const judged = ['lib/engine/credential.ts', ...WORKER_MODULES, 'lib/sync/sign-out.ts'];
    for (const module of judged) expect(paths).toContain(join(SOURCE, module));
    // no test file is in the population, so a test's import of the credential module is never judged
    expect(paths.filter((path) => /\.(test|spec)\./.test(path))).toEqual([]);
  });
});

// SPEC-377 R6, A16: the choice's four operations join the census. An absence census: no reply to
// them carries the host key, the password, the user or a released key, and no page module imports
// the choice module; each is held by a planted control caught by name.
const CHOICE = join(SOURCE, 'lib', 'engine', 'choice');

describe('where the sync key reaches the choice', () => {
  it('no reply to the choice operations carries a secret', async () => {
    const hostKey = sentinel('host', 'key', 'choice', 'reach');
    const password = sentinel('pass', 'word', 'choice', 'reach');
    const user = sentinel('choice', 'user', 'choice', 'reach');
    const release = new ReleaseService();
    const engine = {
      ...standIn(),
      sync_login: () => hostKey,
      sync_collection: () => 4,
      handshake: () => undefined,
      full_sync_count: async () => JSON.stringify({ upload: { reviews: 1, cards: 1, notes: 1 }, download: null }),
      full_sync_confirm: async () => JSON.stringify({ outcome: 'written' }),
      full_sync_cancel: () => undefined,
      unsynced: () => JSON.stringify({ reviews: 0, changed: false, schema: false })
    };
    const store = new CredentialStore(workerDeps(new IDBFactory(), release, new Bus(), engine));
    const { serve } = await import('./worker');
    const { Sync } = await import('./sync');
    const { Choice } = await import('./choice');
    const fetch = async (input: string) =>
      new Response(input.endsWith('/api/sync/snapshot') ? '{"found":true,"age_seconds":60}' : '{"minimum_client_level":1}', {
        status: 200
      });
    const replies: unknown[] = [];
    let hear: (event: MessageEvent) => void = () => {};
    const deps = {
      lock: async () => 'held' as const,
      storage: async () => null,
      load: async () => studyEngine(),
      credential: store,
      sync: new Sync(store, async () => engine, ENDPOINT, fetch),
      choice: new Choice(store, async () => engine, ENDPOINT, fetch)
    };
    const session = serve(
      { postMessage: (reply) => replies.push(reply), addEventListener: (_, listener) => (hear = listener) },
      deps,
      ORIGIN
    );
    const requests = [
      { op: 'open' },
      { op: 'sync-login', user, password },
      { op: 'sync' },
      { op: 'choice-count' },
      { op: 'choice-confirm', direction: 'upload' },
      { op: 'choice-count' },
      { op: 'choice-cancel' },
      { op: 'unsynced' }
    ];
    for (const [id, body] of requests.entries()) hear(new MessageEvent('message', { data: { id, ...body } }));
    const last = await session.handle({ id: requests.length, op: 'credential-status' });
    const answers: [string, unknown][] = [
      ...replies.map((reply, at): [string, unknown] => [`reply ${at}`, reply]),
      ['a last status', last]
    ];
    const secrets: [string, string][] = [
      ['the host key', hostKey],
      ['the password', password],
      ['the user', user],
      ...release.keys.map((key, at): [string, string] => [`released key ${at}`, key])
    ];
    examined('replies to the choice operations and the sync before them', answers);
    expect(leaks(answers, secrets)).toEqual([]);
    // the positive control: a planted choice reply that carries the host key is caught by name
    expect(
      leaks([['a planted choice reply', { id: 9, ok: true, value: { status: 'held', outcome: 'written', key: hostKey } }]], secrets)
    ).toEqual(['a planted choice reply carries the host key']);
  });

  it('no page module imports the choice module', () => {
    const sources = examined('production modules under web/app/src', production(SOURCE));
    expect(importersOf(sources, CHOICE).filter((path) => path !== 'lib/engine/worker.ts')).toEqual([]);
    // the positive control: a planted page that imports the choice module is refused by name
    const planted = {
      path: join(SOURCE, 'routes', 'planted', '+page.svelte'),
      text: `<script lang="ts">\n  import { Choice } from '$lib/engine/choice';\n</script>\n`
    };
    expect(importersOf([...sources, planted], CHOICE).filter((path) => path !== 'lib/engine/worker.ts')).toEqual([
      'routes/planted/+page.svelte'
    ]);
  });
});
